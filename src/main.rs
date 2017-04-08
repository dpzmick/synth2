extern crate easyjack as jack;
#[macro_use]
extern crate time;
#[macro_use]
extern crate enum_primitive;

mod ports;

use ports::PortManager;
use ports::{InputPortHandle, OutputPortHandle};

use std::cell::{RefMut, RefCell};
use std::f32;
use std::f64;
use std::marker::PhantomData;
use std::mem;
use std::process;
use std::slice::Iter;
use std::slice::IterMut;
use std::thread;
use std::time::Duration;

use enum_primitive::FromPrimitive;

static SRATE: f32 = 44100.0;

type PortId = usize;

trait Component<'a> {
    fn generate(&mut self, ports: &mut PortManager);
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>);
}

struct SineWaveOscillator<'a> {
    name: String,
    phase: f32,
    frequency_port: Option<InputPortHandle<'a>>,
    output_port: Option<OutputPortHandle<'a>>,
}

impl<'a> SineWaveOscillator<'a> {
    fn new(name: String) -> Self {
        Self {
            name,
            phase: 0.0,
            frequency_port: None,
            output_port: None,
        }
    }

    fn sine(&self, t: f32) -> f32 {
        assert!(t >= 0.0 && t <= 1.0);
        (2.0 * t * f32::consts::PI).sin()
    }
}

impl<'a> Component<'a> for SineWaveOscillator<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>) {
        // TODO error handling?

        self.frequency_port =
            Some(ports.register_input_port(self.name.clone(), "frequency_in".to_string()).unwrap());

        self.output_port =
            Some(ports.register_output_port(self.name.clone(), "samples_out".to_string()).unwrap());
    }

    fn generate(&mut self, ports: &mut PortManager) {
        if self.frequency_port.is_none() || self.output_port.is_none() { return; }

        let freq = ports.get_port_value(&self.frequency_port.unwrap());
        self.phase += (freq / SRATE);

        while self.phase > 1.0 {
            self.phase -= 1.0;
        }

        let v = self.sine(self.phase);
        ports.set_port_value(&self.output_port.unwrap(), v);
    }
}

struct SquareWaveOscillator<'a> {
    name: String,
    sine: SineWaveOscillator<'a>,
}

impl<'a> SquareWaveOscillator<'a> {
    fn new(name: String) -> Self {
        Self {
            name: name.clone(),
            sine: SineWaveOscillator::new(name),
        }
    }
}

impl<'a> Component<'a> for SquareWaveOscillator<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>)  {
        self.sine.initialize_ports(ports);
    }

    fn generate(&mut self, ports: &mut PortManager) {
        // find the sine out port
        let mut out = None;
        match ports.find_ports(&self.name) {
            Some(ports) => {
                for port in ports {
                    match port.promote_to_output() {
                        Ok(port) => out = Some(port),
                        Err(_)   => (),
                    }
                }
            },
            None => (),
        };

        // we can write to the output port, then overwrite the value
        // nothing else can be generating while we are generating so there is no chance of this
        // value leaking into some other component

        match out {
            Some(out) => {
                self.sine.generate(ports);
                let v = ports.get_port_value(&out);

                if v < 0.0 {
                    ports.set_port_value(&out, -1.0);
                } else if v > 0.0 {
                    ports.set_port_value(&out, 1.0);
                }
            },

            None => ()
        }

    }
}

struct OnOff<'a> {
    name:        String,
    samples_in:  Option<InputPortHandle<'a>>,
    gate_in:     Option<InputPortHandle<'a>>,
    samples_out: Option<OutputPortHandle<'a>>,
}

impl<'a> OnOff<'a> {
    fn new(name: String) -> Self {
        Self {
            name,
            samples_in:  None,
            gate_in:     None,
            samples_out: None
        }
    }
}

impl<'a> Component<'a> for OnOff<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>) {
        self.samples_in =
            Some(ports.register_input_port(self.name.clone(), "samples_in".to_string()).unwrap());

        self.gate_in =
            Some(ports.register_input_port(self.name.clone(), "gate_in".to_string()).unwrap());

        self.samples_out =
            Some(ports.register_output_port(self.name.clone(), "samples_out".to_string()).unwrap());
    }

    fn generate(&mut self, ports: &mut PortManager) {
        if self.samples_in.is_none() || self.gate_in.is_none() || self.samples_out.is_none() { return; }

        let samples = ports.get_port_value(&self.samples_in.unwrap());
        let gate    = if ports.get_port_value(&self.gate_in.unwrap()) != 0.0 { 1.0 } else { 0.0 };

        let out = samples * gate;
        if ports.get_port_value(&self.samples_out.unwrap()) != out {
            ports.set_port_value(&self.samples_out.unwrap(), out);
        }
    }
}

struct Math<'a> {
    name: String,
    math_function: Box<Fn(f32) -> f32>,
    input:  Option<InputPortHandle<'a>>,
    output: Option<OutputPortHandle<'a>>,
}

impl<'a> Math<'a> {
    fn new<M: Fn(f32) -> f32 + 'static>(name: String, math: M) -> Self {
        Self {
            name,
            math_function: Box::new(math),
            input:  None,
            output: None,
        }
    }

    // fn parse_math(expr: String) -> Box<Fn(f32) -> f32> {
    //     // TODO
    // }
}

impl<'a> Component<'a> for Math<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>) {
        self.input = Some(ports.register_input_port(self.name.clone(), "input".to_string()).unwrap());
        self.output = Some(ports.register_output_port(self.name.clone(), "output".to_string()).unwrap());
    }

    fn generate(&mut self, ports: &mut PortManager) {
        let i = ports.get_port_value(&self.input.unwrap());
        ports.set_port_value(&self.output.unwrap(), (self.math_function)(i))
    }
}

// has many inputs and combines them proportionally to how many are emitting a signal
struct CombineInputs<'a> {
    name: String,
    num_inputs: usize, // needed when initializing ports
    inputs: Vec<InputPortHandle<'a>>,
    output: Option<OutputPortHandle<'a>>,
}

impl<'a> CombineInputs<'a> {
    fn new(name: String, num_inputs: usize) -> Self {
        Self {
            name,
            num_inputs,
            inputs: Vec::new(),
            output: None
        }
    }
}

impl<'a> Component<'a> for CombineInputs<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>) {
        for i in 0..self.num_inputs {
            let iname = format!("{}_input{}", self.name, i);
            let i = ports.register_input_port(self.name.clone(), iname);
            self.inputs.push(i.unwrap());
        }

        self.output = Some(ports.register_output_port(self.name.clone(), "out".to_string()).unwrap());
    }

    fn generate(&mut self, ports: &mut PortManager) {
        let mut count: usize = 0;
        let mut input_sum = 0.0;
        for input in self.inputs.iter() {
            let i = ports.get_port_value(input);
            if i != 0.0 {
                input_sum += i;
                count     += 1;
            }
        }

        if count > 0 {
            let v = input_sum * (1.0 / count as f32);
            ports.set_port_value(&self.output.unwrap(), v);
        }
    }
}

// One voice holds a complete representation of the graph
struct Voice<'a> {
    components: Vec<Box<Component<'a> + 'a>>,
    edges: Vec<(usize, usize)>,
    ports: PortManager<'a>,
    // we have a few default ports to contend with
    // these are populated and read from by the audio library
    midi_frequency_in: OutputPortHandle<'a>,
    midi_gate_in:      OutputPortHandle<'a>,
    samples_out:       InputPortHandle<'a>,
}

// eventually will be polyphonic
impl<'a> Voice<'a> {
    fn new() -> Self {
        let mut ports = PortManager::new();
        let midi_frequency_in =
            ports.register_output_port("voice".to_string(), "midi_frequency_out".to_string()).unwrap();

        let midi_gate_in =
            ports.register_output_port("voice".to_string(), "midi_gate_out".to_string()).unwrap();

        let samples_out =
            ports.register_input_port("voice".to_string(), "samples_in".to_string()).unwrap();

        Self {
            components: Vec::new(),
            edges:      Vec::new(),
            ports:      ports,
            midi_frequency_in,
            midi_gate_in,
            samples_out
        }
    }

    fn note_on(&mut self, freq: f32, vel: f32) {
        // TODO velocity?
        self.ports.set_port_value(&self.midi_frequency_in, freq);
        self.ports.set_port_value(&self.midi_gate_in, 1.0);
    }

    // TODO will frequency ever be used? Probably not
    fn note_off(&mut self, freq: f32) {
        self.ports.set_port_value(&self.midi_gate_in, 0.0);
    }

    fn current_frequency(&self) -> Option<f32> {
        if self.ports.get_port_value(&self.midi_gate_in) != 0.0 {
            Some(self.ports.get_port_value(&self.midi_frequency_in))
        } else {
            None
        }
    }

    fn add_component<T: Component<'a> + 'a>(&mut self, comp: T) {
        // TODO fix this up, do a sort
        self.components.push(Box::new(comp));
        let s = self.components.len();
        let ref mut comp = self.components[s - 1];
        comp.initialize_ports(&mut self.ports);
    }

    /// Generate a single sample
    fn generate(&mut self) -> f32 {
        // TODO topo sort the components as they get added
        // update the world, in order
        for component in self.components.iter_mut() {
            component.generate(&mut self.ports);
        }

        // get the value on the output wire
        self.ports.get_port_value(&self.samples_out)
    }

    fn example_connections(&mut self) {
        // creates two harmonics
        // midi input goes through here to get second harmonic
        self.add_component(Math::new("math".to_string(), |x| x*2.0));
        self.add_component(SquareWaveOscillator::new("harmonic_osc".to_string()));

        // midi input also sent through here
        self.add_component(SineWaveOscillator::new("base_osc".to_string()));

        // create an input combiner with 2 inputs
        self.add_component(CombineInputs::new("combine".to_string(), 2));

        // finally, gate is sent through the OnOff
        self.add_component(OnOff::new("envelope".to_string()));

        // connect things
        let pairs = [
            // push midi frequency the right places
            ( ("voice", "midi_frequency_out"), ("base_osc", "frequency_in") ),
            ( ("voice", "midi_frequency_out"), ("math", "input") ),

            // finish up the connections for math
            ( ("math", "output"), ("harmonic_osc", "frequency_in") ),

            // connect the oscillators to the combiner
            ( ("base_osc", "samples_out"), ("combine", "combine_input0") ),
            ( ("harmonic_osc", "samples_out"), ("combine", "combine_input1") ),

            // set up the envelope
            ( ("voice", "midi_gate_out"), ("envelope", "gate_in") ),
            ( ("combine", "out"), ("envelope", "samples_in") ),

            // send audio back to the card
            ( ("envelope", "samples_out"), ("voice", "samples_in") )
        ];

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
    fn new() -> Self {
        let mut voices = Vec::new();
        for _ in 0..16 {
            voices.push(Voice::new());
        }

        Self { voices }
    }

    fn example_connections(&mut self) {
        for voice in self.voices.iter_mut() {
            voice.example_connections()
        }
    }

    fn note_on(&mut self, freq: f32, vel: f32) {
        for voice in self.voices.iter_mut() {
            match voice.current_frequency() {
                Some(_f) => (),
                None => {
                    voice.note_on(freq, vel);
                    return
                }
            }
        }

        // TODO replacement policy
    }

    fn note_off(&mut self, freq: f32) {
        for voice in self.voices.iter_mut() {
            match voice.current_frequency() {
                Some(f) => {
                    if freq == f {
                        voice.note_off(f)
                    }
                },
                None => ()
            }
        }
    }

    fn generate(&mut self) -> f32 {
        let mut sample = 0.0;
        for voice in self.voices.iter_mut() {
            let subsample = voice.generate();
            sample += subsample;
        }

        sample * (1.0 / self.voices.len() as f32)
    }
}

struct MidiMessage<'a> {
    pub data: &'a [u8]
}

enum_from_primitive! {
#[derive(Debug,PartialEq,Clone,Copy)]
pub enum MidiStatus {
    // voice
    NoteOff = 0x80,
    NoteOn = 0x90,
    PolyphonicAftertouch = 0xA0,
    ControlChange = 0xB0,
    ProgramChange = 0xC0,
    ChannelAftertouch = 0xD0,
    PitchBend = 0xE0,

    // sysex
    SysExStart = 0xF0,
    MIDITimeCodeQtrFrame = 0xF1,
    SongPositionPointer = 0xF2,
    SongSelect = 0xF3,
    TuneRequest = 0xF6, // F4 anf 5 are reserved and unused
    SysExEnd = 0xF7,
    TimingClock = 0xF8,
    Start = 0xFA,
    Continue = 0xFB,
    Stop = 0xFC,
    ActiveSensing = 0xFE, // FD also res/unused
    SystemReset = 0xFF,
}
}

impl<'a> MidiMessage<'a> {
    pub fn status(&self) -> MidiStatus {
        MidiStatus::from_u8(self.data[0]).unwrap()
    }
}

type OPort = jack::OutputPortHandle<jack::DefaultAudioSample>;
type IPort = jack::InputPortHandle<jack::MidiEvent>;

fn midi_note_to_frequency(note: u8) -> f32 {
    let a = 440.0;
    // this is a magic formula from the internet
    (a / 32.0) * (2.0_f32.powf( (note as f32 - 9.0) / 12.0 ))
}

fn midi_velocity_to_velocity(vel: u8) -> f32 {
    vel as f32 / (std::u8::MAX as f32)
}

struct AudioHandler<'a> {
    input: IPort,
    output: OPort,
    soundscape: Soundscape<'a>,
}


impl<'a> AudioHandler<'a> {
    pub fn new(input: IPort, output: OPort) -> Self {
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
    fn process(&mut self, ctx: &jack::CallbackContext, nframes: jack::NumFrames) -> i32 {
        let start = time::precise_time_ns();
        let output_buffer = self.output.get_write_buffer(nframes, &ctx);
        let input_buffer  = self.input.get_read_buffer(nframes, &ctx);
        let end = time::precise_time_ns();
        //println!("buffer setup: {}", end - start);

        let mut current_event = unsafe { mem::uninitialized() };
        let mut current_event_index = 0;
        let event_count = input_buffer.len();

        let start2 = time::precise_time_ns();
        for i in 0..(nframes as usize) {
            while current_event_index < event_count {
                current_event = input_buffer.get(current_event_index);
                if current_event.get_jack_time() as usize != i { break; }
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

                    _ => ()
                }

            }

            output_buffer[i] = self.soundscape.generate();
        }

        let end = time::precise_time_ns();
        //println!("frames filling: {}", end - start2);
        let end = time::precise_time_ns();
        //println!("elapsed: {}", end - start);
        0
    }
}

fn dump_samples() {
    let mut gen = Soundscape::new();
    gen.note_on(440.0, 1.0);

    for i in 0..1024  {
        println!("{}", gen.generate());
    }
}

fn main() {
    // dump_samples();
    let mut c = jack::Client::open("sine", jack::options::NO_START_SERVER).unwrap().0;
    let i = c.register_input_midi_port("midi_in").unwrap();
    let o = c.register_output_audio_port("audio_out").unwrap();

    let handler = AudioHandler::new(i, o);

    c.set_process_handler(handler).unwrap();

    c.activate().unwrap();
    c.connect_ports("sine:audio_out", "system:playback_1").unwrap();

    loop {
        thread::sleep(Duration::from_millis(100000));
    }

    c.close().unwrap();
}
