extern crate easyjack as jack;
#[macro_use]
extern crate time;

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

static SRATE: f32 = 44100.0;

type PortId = usize;

#[derive(PartialEq, Eq)]
enum PortDirection {
    In,
    Out,
}

struct PortInfo {
    pub comp: &'static str,
    pub port: &'static str,
    pub dir: PortDirection,
    pub id: PortId,
}

struct PortManager {
    ports: Vec<f32>,
    connections: Vec<(usize, usize)>,
}

impl PortManager {
    fn new() -> Self {
        Self {
            ports: Vec::new(),
            connections: Vec::new(),
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

        for &(p1, p2) in self.connections.iter() {
            if p1 == p {
                self.ports[p2] = val;
            }
        }
        println!("set value on port {} to {}", p, val);
    }

    pub fn connect_ports(&mut self, p1: PortId, p2: PortId) {
        // we can do better
        self.connections.push( (p1, p2) );
        self.connections.push( (p2, p1) );
        println!("connected {} and {}", p1, p2);
    }
}

trait Component {
    fn generate(&mut self, ports: &mut PortManager);

    // slow path
    fn initialize_ports(&mut self, ports: &mut PortManager);
    fn get_ports(&self) -> &[PortInfo];
}

struct SineWaveOscilator {
    phase: f32,
    my_ports: Vec<PortInfo>,
    frequency_port: Option<PortId>,
    output_port: Option<PortId>,
}

impl SineWaveOscilator {
    fn new() -> Self {
        Self {
            phase: 0.0,
            my_ports: Vec::new(),
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
    fn initialize_ports(&mut self, ports: &mut PortManager) {
        let i = PortInfo {
            comp: "Sine",
            port: "frequency_in",
            dir: PortDirection::In,
            id: ports.register_port(),
        };

        println!("output; {}", i.id);
        self.frequency_port = Some(i.id);
        self.my_ports.push(i);

        let o = PortInfo {
            comp: "Sine",
            port: "samples_out",
            dir: PortDirection::Out,
            id: ports.register_port(),
        };

        println!("output; {}", o.id);
        self.output_port = Some(o.id);
        self.my_ports.push(o);
    }

    fn get_ports(&self) -> &[PortInfo] {
        self.my_ports.as_slice()
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
    // we have a few default ports to contend with
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
        self.ports.get_port_val(self.samples_out).unwrap()
    }

    fn example_connections(&mut self) {
        for port in self.components[0].get_ports() {
            if port.dir == PortDirection::In {
                self.ports.connect_ports(port.id, self.midi_frequency_in);
            }

            if port.dir == PortDirection::Out {
                self.ports.connect_ports(port.id, self.samples_out);
            }
        }

    }
}

fn main() {
    let mut sounds = Soundscape::new();
    sounds.add_component(SineWaveOscilator::new());
    sounds.example_connections();

    // fire up a note
    sounds.note_on(440.0, 1.0);

    for i in 0..256 {
        println!("{}", sounds.generate());
    }
}
