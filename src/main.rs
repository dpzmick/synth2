extern crate easyjack as jack;
#[macro_use]
extern crate enum_primitive;

mod components;
mod midi;
mod ports;

use components::{CombineInputs, Math, OnOff, SineWaveOscillator, SquareWaveOscillator};
use components::Component;
use midi::{MidiMessage, MidiStatus};
use ports::{InputPortHandle, OutputPortHandle};
use ports::PortManager;

use std::mem;
use std::thread;
use std::time::Duration;

static SRATE: f32 = 44100.0;

// One voice holds a complete representation of the graph
struct Voice<'a> {
    components: Vec<Box<Component<'a> + 'a>>,
    //edges: Vec<(usize, usize)>,
    ports: PortManager<'a>,
    // we have a few default ports to contend with
    // these are populated and read from by the audio library
    midi_frequency_in: OutputPortHandle<'a>,
    midi_gate_in: OutputPortHandle<'a>,
    samples_out: InputPortHandle<'a>,
}

// eventually will be polyphonic
impl<'a> Voice<'a> {
    fn new() -> Self
    {
        let mut ports = PortManager::new();
        let midi_frequency_in = ports
            .register_output_port("voice".to_string(), "midi_frequency_out".to_string())
            .unwrap();

        let midi_gate_in = ports
            .register_output_port("voice".to_string(), "midi_gate_out".to_string())
            .unwrap();

        let samples_out = ports
            .register_input_port("voice".to_string(), "samples_in".to_string())
            .unwrap();

        Self {
            components: Vec::new(),
            //edges:      Vec::new(),
            ports: ports,
            midi_frequency_in,
            midi_gate_in,
            samples_out,
        }
    }

    fn note_on(&mut self, freq: f32, vel: f32)
    {
        // TODO velocity?
        self.ports.set_port_value(&self.midi_frequency_in, freq);
        self.ports.set_port_value(&self.midi_gate_in, 1.0);
    }

    // TODO will frequency ever be used? Probably not
    fn note_off(&mut self, freq: f32)
    {
        self.ports.set_port_value(&self.midi_gate_in, 0.0);
    }

    fn current_frequency(&self) -> Option<f32>
    {
        if self.ports.get_port_value(&self.midi_gate_in) != 0.0 {
            Some(self.ports.get_port_value(&self.midi_frequency_in))
        } else {
            None
        }
    }

    fn add_component<T: Component<'a> + 'a>(&mut self, comp: T)
    {
        // TODO fix this up, do a sort
        self.components.push(Box::new(comp));
        let s = self.components.len();
        let ref mut comp = self.components[s - 1];
        comp.initialize_ports(&mut self.ports);
    }

    /// Generate a single sample
    fn generate(&mut self) -> f32
    {
        // TODO topo sort the components as they get added
        // update the world, in order
        for component in self.components.iter_mut() {
            component.generate(&mut self.ports);
        }

        // get the value on the output wire
        self.ports.get_port_value(&self.samples_out)
    }

    fn example_connections(&mut self)
    {
        // creates two harmonics
        // midi input goes through here to get second harmonic
        self.add_component(Math::new("math".to_string(), |x| x * 2.0));
        self.add_component(SquareWaveOscillator::new("harmonic_osc".to_string()));

        // midi input also sent through here
        self.add_component(SineWaveOscillator::new("base_osc".to_string()));

        // create an input combiner with 2 inputs
        self.add_component(CombineInputs::new("combine".to_string(), 2));

        // finally, gate is sent through the OnOff
        self.add_component(OnOff::new("envelope".to_string()));

        // connect things
        let pairs = [// push midi frequency the right places
                     (("voice", "midi_frequency_out"), ("base_osc", "frequency_in")),
                     (("voice", "midi_frequency_out"), ("math", "input")),

                     // finish up the connections for math
                     (("math", "output"), ("harmonic_osc", "frequency_in")),

                     // connect the oscillators to the combiner
                     (("base_osc", "samples_out"), ("combine", "combine_input0")),
                     (("harmonic_osc", "samples_out"), ("combine", "combine_input1")),

                     // set up the envelope
                     (("voice", "midi_gate_out"), ("envelope", "gate_in")),
                     (("combine", "out"), ("envelope", "samples_in")),

                     // send audio back to the card
                     (("envelope", "samples_out"), ("voice", "samples_in"))];

        for &(p1, p2) in pairs.iter() {
            println!("connecting {:?} to {:?}", p1, p2);
            self.ports.connect_by_name(p1, p2).unwrap();
        }
    }
}

/// A soundscape contains many voices, manages NoteOn/NoteOff for each voice
/// For the moment, this will just make lots of copies. There's lots of room for optimization
/// though
struct Soundscape<'a> {
    // this would be an array, but arrays are so severely limited in rust that I'm using a vector.
    // Don't ever resize it!
    voices: Vec<Voice<'a>>,
}

impl<'a> Soundscape<'a> {
    fn new() -> Self
    {
        let mut voices = Vec::new();
        for _ in 0..16 {
            voices.push(Voice::new());
        }

        Self { voices }
    }

    fn example_connections(&mut self)
    {
        for voice in self.voices.iter_mut() {
            voice.example_connections()
        }
    }

    fn note_on(&mut self, freq: f32, vel: f32)
    {
        for voice in self.voices.iter_mut() {
            match voice.current_frequency() {
                Some(_f) => (),
                None => {
                    voice.note_on(freq, vel);
                    return;
                },
            }
        }

        // TODO replacement policy
    }

    fn note_off(&mut self, freq: f32)
    {
        for voice in self.voices.iter_mut() {
            match voice.current_frequency() {
                Some(f) => {
                    if freq == f {
                        voice.note_off(f)
                    }
                },
                None => (),
            }
        }
    }

    fn generate(&mut self) -> f32
    {
        let mut count = 0;
        let mut sample = 0.0;
        for voice in self.voices.iter_mut() {
            let subsample = voice.generate();
            sample += subsample;
        }

        sample * (1.0 / self.voices.len() as f32)
    }
}

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
    pub fn new(input: IPort, output: OPort) -> Self
    {
        let mut soundscape = Soundscape::new();
        soundscape.example_connections();

        Self {
            input,
            output,
            soundscape,
        }
    }
}

impl<'a> jack::ProcessHandler for AudioHandler<'a> {
    fn process(&mut self, ctx: &jack::CallbackContext, nframes: jack::NumFrames) -> i32
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

                    _ => (),
                }

            }

            output_buffer[i] = self.soundscape.generate();
        }

        0
    }
}

fn main()
{
    // start an audio handler
    // start a ui

    let mut c = jack::Client::open("sine", jack::options::NO_START_SERVER)
        .unwrap()
        .0;
    let i = c.register_input_midi_port("midi_in").unwrap();
    let o = c.register_output_audio_port("audio_out").unwrap();

    let handler = AudioHandler::new(i, o);

    c.set_process_handler(handler).unwrap();

    c.activate().unwrap();
    c.connect_ports("sine:audio_out", "system:playback_1")
        .unwrap();

    loop {
        thread::sleep(Duration::from_millis(100000));
    }

    c.close().unwrap();
}
