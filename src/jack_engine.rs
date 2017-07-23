use jack;

use midi::{MidiMessage, MidiStatus};
use soundscape::Soundscape;
use std;
use std::mem;

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

struct AudioHandler<'a> {
    input: IPort,
    output: OPort,
    soundscape: Soundscape<'a>,
}

impl<'a> AudioHandler<'a> {
    pub fn new(input: IPort, output: OPort, soundscape: Soundscape<'a>)
        -> Self
    {
        Self {
            input,
            output,
            soundscape,
        }
    }
}

impl<'a> jack::ProcessHandler for AudioHandler<'a> {
    fn process(&mut self, ctx: &jack::CallbackContext, nframes: jack::NumFrames)
        -> i32
    {
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

            output_buffer[i] = self.soundscape.generate();
        }

        0
    }
}

pub fn run_audio_thread(soundscape: Soundscape)
{
    let mut c = jack::Client::open("sine", jack::options::NO_START_SERVER)
        .unwrap()
        .0;

    let i = c.register_input_midi_port("midi_in").unwrap();
    let o = c.register_output_audio_port("audio_out").unwrap();

    let handler = AudioHandler::new(i, o, soundscape);

    c.set_process_handler(handler).unwrap();

    c.activate().unwrap();
    c.connect_ports("sine:audio_out", "system:playback_1")
        .unwrap();

    // TODO make this some struct so I can shut down gracefully
}
