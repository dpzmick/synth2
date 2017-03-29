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

// do the values decay?
struct Wire {
    current_value: f32
}

impl Wire {
    fn new() -> Self {
        Self { current_value: 0.0 }
    }

    fn read(&self) -> f32 { self.current_value }
    fn write(&mut self, val: f32) { self.current_value = val; }
}

// What happens if a wire is destroyed?
type WireId = usize;

struct WireManager {
    wires: Vec<Wire>
}

impl WireManager {
    /// Only safe in slow path
    fn new() -> Self {
        Self { wires: Vec::new() }
    }

    fn register_wire(&mut self) -> WireId {
        let wire = Wire::new();
        self.wires.push(wire);
        self.wires.len() - 1
    }

    fn borrow_wire(&self, id: WireId) -> &Wire {
        &self.wires[id as usize]
    }

    fn borrow_wire_mut(&mut self, id: WireId) -> &mut Wire {
        &mut self.wires[id as usize]
    }
}

trait Component {
    fn set_incoming(&mut self, inpt: WireId);
    fn set_outgoing(&mut self, inpt: WireId);
    fn generate(&mut self, wires: &mut WireManager);
}

struct SineWaveOscilator {
    incoming: Option<WireId>,
    outgoing: Option<WireId>,
    phase: f32,
}

impl SineWaveOscilator {
    fn new() -> Self {
        Self {
            incoming: None,
            outgoing: None,
            phase: 0.0,
        }
    }

    fn sine(&self, t: f32) -> f32 {
        assert!(t >= 0.0 && t <= 1.0);
        (2.0 * t * f32::consts::PI).sin()
    }
}

impl Component for SineWaveOscilator {
    fn set_incoming(&mut self, inpt: WireId) {
        self.incoming = Some(inpt)
    }

    fn set_outgoing(&mut self, out: WireId) {
        self.outgoing = Some(out)
    }

    fn generate(&mut self, wires: &mut WireManager) {
        if self.incoming.is_none() || self.outgoing.is_none() { return; }

        let incoming = self.incoming.unwrap();

        // TODO do something about the input and output ranges
        let freq = wires.borrow_wire(incoming).read();
        self.phase += (freq / SRATE);

        while self.phase > 1.0 {
            self.phase -= 1.0;
        }

        let outgoing = self.outgoing.unwrap();
        wires.borrow_wire_mut(outgoing).write(self.sine(self.phase));
    }
}

struct Soundscape {
    components: Vec<Box<Component>>,
    wires: WireManager,
    // we have a few default wires to contend with
    // these are populated and read from by the audio library
    midi_frequency_in: WireId,
    midi_gate_in:      WireId,
    samples_out:       WireId,
}

// eventually will be polyphonic
impl Soundscape {
    fn new() -> Self {
        let mut wires = WireManager::new();
        let midi_frequency_in = wires.register_wire();
        let midi_gate_in      = wires.register_wire();
        let samples_out       = wires.register_wire();

        Self {
            components: Vec::new(),
            wires, midi_frequency_in, midi_gate_in, samples_out
        }
    }

    fn note_on(&mut self, freq: f32, vel: f32) {
        // TODO velocity?
        self.wires.borrow_wire_mut(self.midi_frequency_in).write(freq);
        self.wires.borrow_wire_mut(self.midi_gate_in).write(1.0);
    }

    fn note_off(&mut self, freq: f32) {
        self.wires.borrow_wire_mut(self.midi_gate_in).write(0.0);
    }

    fn add_component<T: Component + 'static>(&mut self, comp: T) {
        // TODO fix this up, do a sort
        self.components.push(Box::new(comp));
    }

    /// no idea what this interface needs to look like so I'm just gonna wing it
    fn do_connecting(&mut self) {
        self.components[0].set_incoming(self.midi_frequency_in);
        self.components[0].set_outgoing(self.samples_out);
    }

    /// Generate a single sample
    fn generate(&mut self) -> f32 {
        // TODO topo sort the components as they get added
        // update the world, in order
        for component in self.components.iter_mut() {
            component.generate(&mut self.wires);
        }

        // get the value on the output wire
        self.wires.borrow_wire(self.samples_out).read()
    }
}

fn main() {
    let mut sounds = Soundscape::new();
    sounds.add_component(SineWaveOscilator::new());
    sounds.do_connecting();

    // fire up a note
    sounds.note_on(440.0, 1.0);

    for i in 0..256 {
        println!("{}", sounds.generate());
    }
}
