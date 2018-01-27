use midi::{MidiMessage, MidiStatus};
use soundscape::Soundscape;
use audioprops::AudioProperties;

use jack;

use std;
use std::mem;
use std::sync::mpsc;

type OPort = jack::OutputPortHandle<jack::DefaultAudioSample>;
type IPort = jack::InputPortHandle<jack::MidiEvent>;

fn midi_note_to_frequency(note: u8) -> f32
{
    let a = 440.0;
    // this is a magic formula from the internet
    (a / 32.0) * (2.0_f32.powf((note as f32 - 9.0) / 12.0))
}

fn midi_velocity_to_velocity(vel: u8) -> f32
{
    vel as f32 / (std::u8::MAX as f32)
}

#[derive(Debug)]
enum Message {
    AudioProperties(AudioProperties),
}

struct AudioHandler<'a> {
    input: IPort,
    output: OPort,
    // I own the soundscape
    soundscape: Soundscape<'a>,
    // Any additional soundscape operations will be sent over a queue to this
    // thread, instead of requiring the soundscape to manage any synchronization
    incoming: mpsc::Receiver<Message>
}

impl<'a> AudioHandler<'a> {
    pub fn new(
        input: IPort,
        output: OPort,
        soundscape: Soundscape<'a>,
        incoming: mpsc::Receiver<Message>)
    -> Self
    {
        Self {
            input,
            output,
            soundscape,
            incoming,
        }
    }

    fn handle_incoming(&mut self)
    {
        while let Ok(m) = self.incoming.try_recv() {
            match m {
                Message::AudioProperties(p) =>
                    self.soundscape.handle_audio_property_change(p),
            }
        }
    }
}

impl<'a> jack::ProcessHandler for AudioHandler<'a> {
    fn process(&mut self, ctx: &jack::CallbackContext, nframes: jack::NumFrames)
        -> i32
    {
        self.handle_incoming();

        let output_buffer = self.output.get_write_buffer(nframes, &ctx);
        let input_buffer = self.input.get_read_buffer(nframes, &ctx);

        let mut current_event = unsafe { mem::uninitialized() };
        let mut current_event_index = 0;
        let event_count = input_buffer.len();

        for i in 0..(nframes as usize) {
            while current_event_index < event_count {
                current_event = input_buffer.get(current_event_index);
                if current_event.get_jack_time() as usize != i {
                    break;
                }
                current_event_index += 1;

                let buf = current_event.raw_midi_bytes();
                let m = MidiMessage { data: buf };
                match m.status() {
                    MidiStatus::NoteOff => {
                        let f = midi_note_to_frequency(m.data[1]);
                        self.soundscape.note_off(f);
                    },

                    MidiStatus::NoteOn => {
                        let f = midi_note_to_frequency(m.data[1]);
                        let v = midi_velocity_to_velocity(m.data[2]);
                        self.soundscape.note_on(f, v);
                    },

                    MidiStatus::ControlChange => {
                        let cc = m.data[1];
                        let val = m.data[2];
                        self.soundscape.control_value_change(cc, val);
                    },

                    _ => (),
                }

            }

            let s = self.soundscape.generate();
            output_buffer[i] = s;
        }

        0
    }
}

struct MetadataHandler {
    sender: mpsc::SyncSender<Message>,
}

impl MetadataHandler {
    fn new(sender: mpsc::SyncSender<Message>) -> Self
    {
        Self { sender }
    }
}

impl jack::MetadataHandler for MetadataHandler {
    fn sample_rate_changed(&mut self, srate: jack::NumFrames) -> i32
    {
        let prop = AudioProperties::SampleRate(srate as f32);
        let message = Message::AudioProperties(prop);
        self.sender.send(message).unwrap(); // TODO bad!

        0 // success
    }

    fn callbacks_of_interest(&self) -> Vec<jack::MetadataHandlers>
    {
        vec![jack::MetadataHandlers::SampleRate]
    }
}

/// The threads will continue running until this struct is dropped out of scope or until
pub struct JackAudioThreads<'a> {
    client: jack::Client<'a>,
}

impl<'a> JackAudioThreads<'a> {
    pub fn shutdown(mut self) {
        self.client.close().unwrap();
    }
}

// TODO find a way to panic if the threads are dropped before jack is shutdown

pub fn run_audio_threads(soundscape: Soundscape) -> JackAudioThreads
{
    let mut c = jack::Client::open("sine", jack::options::NO_START_SERVER)
        .unwrap()
        .0;

    let i = c.register_input_midi_port("midi_in").unwrap();
    let o = c.register_output_audio_port("audio_out").unwrap();

    let (sender, receiver) = mpsc::sync_channel(1024);

    // TODO make sure easy jack has appropriate thread safety stuff added
    // The audio handler thread takes ownership of the soundscape.
    // Any external messages to the soundscape must be sent over a channel
    let ahandler = AudioHandler::new(i, o, soundscape, receiver);
    let mhandler = MetadataHandler::new(sender.clone());

    c.set_process_handler(ahandler).unwrap();
    c.set_metadata_handler(mhandler).unwrap();

    c.activate().unwrap();
    c.connect_ports("sine:audio_out", "system:playback_1")
        .unwrap();

    JackAudioThreads {
        client: c,
    }
}
