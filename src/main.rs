extern crate easyjack as jack;
#[macro_use]
extern crate generic_array;
#[macro_use]
extern crate enum_primitive;
extern crate time;

use std::thread;
use std::time::Duration;
use std::f32;
use std::f64;
use std::slice::Iter;
use std::slice::IterMut;
use std::marker::PhantomData;
use std::cell::{RefMut, RefCell};
use std::process;
use std::mem;

use generic_array::ArrayLength;
use generic_array::GenericArray;
use generic_array::typenum::{U2, U3, U4096};
use enum_primitive::FromPrimitive;

static SRATE: f32 = 44100.0;

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

type PortId = usize;

struct PortManager {
    ports: Vec<f32>
}

impl PortManager {
    fn new() -> Self {
        Self {
            ports: Vec::new()
        }
    }

    pub fn register_port(&mut self) -> PortId {
        self.ports.push(0.0);
        self.ports.len() - 1
    }

    pub fn get_port_val(&mut self, p: PortId) -> Option<f32> {
        Some(self.ports[p])
    }

    pub fn set_port_value(&mut self, p: PortId, val: f32) {
        // assert cannot allocate or resize
        self.ports[p] = val;
    }
}

trait Component {
    fn hack_connect(&mut self, comp_port: PortId, other_port: PortId);
    fn generate(&mut self, ports: &mut PortManager);
}

struct SineWaveOscilator {
    phase: f32,
    frequency_port: Option<PortId>,
    output_port: Option<PortId>,
}

impl SineWaveOscilator {
    fn new() -> Self {
        Self {
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

impl Component for SineWaveOscilator {
    fn hack_connect(&mut self, f: PortId, o: PortId) {
        self.frequency_port = Some(f);
        self.output_port = Some(o);
    }

    fn generate(&mut self, ports: &mut PortManager) {
        if self.frequency_port.is_none() || self.output_port.is_none() { return; }

        let freq = ports.get_port_val(self.frequency_port.unwrap()).unwrap();
        self.phase += (freq / SRATE);

        while self.phase > 1.0 {
            self.phase -= 1.0;
        }

        let v = self.sine(self.phase);
        ports.set_port_value(self.output_port.unwrap(), v);
    }
}

struct Soundscape {
    components: Vec<Box<Component>>,
    edges: Vec<(usize, usize)>,
    ports: PortManager,
    // we have a few default wires to contend with
    // these are populated and read from by the audio library
    midi_frequency_in: PortId,
    midi_gate_in:      PortId,
    samples_out:       PortId,
}

// eventually will be polyphonic
impl Soundscape {
    fn new() -> Self {
        let mut ports = PortManager::new();
        let midi_frequency_in = ports.register_port();
        let samples_out       = ports.register_port();
        let midi_gate_in      = ports.register_port();

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
        self.ports.set_port_value(self.midi_frequency_in, freq);
        self.ports.set_port_value(self.midi_gate_in, 1.0);
    }

    fn note_off(&mut self, freq: f32) {
        self.ports.set_port_value(self.midi_gate_in, 0.0);
    }

    fn add_component<T: Component + 'static>(&mut self, comp: T) {
        // TODO fix this up, do a sort
        self.components.push(Box::new(comp));
        let s = self.components.len();
        let ref mut comp = self.components[s - 1];
        comp.hack_connect(self.midi_frequency_in, self.samples_out);
    }

    /// Generate a single sample
    fn generate(&mut self) -> f32 {
        // TODO topo sort the components as they get added
        // update the world, in order
        for component in self.components.iter_mut() {
            component.generate(&mut self.ports);
        }

        // get the value on the output wire
        self.ports.get_port_val(self.samples_out).unwrap()
    }
}

fn main() {
    let mut sounds = Soundscape::new();
    sounds.add_component(SineWaveOscilator::new());

    // fire up a note
    sounds.note_on(440.0, 1.0);

    for i in 0..256 {
        println!("{}", sounds.generate());
    }
}
