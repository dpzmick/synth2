use components::Component;
use ports::{InputPortHandle, OutputPortHandle, PortManager};

/// A single instance of the graph
#[derive(Debug)]
pub struct Voice<'a> {
    components: Vec<Box<Component<'a> + 'a>>,
    ports: PortManager<'a>,
    // we have a few default ports to contend with
    // these are populated and read from by the audio library
    midi_frequency_in: OutputPortHandle<'a>,
    midi_gate_in: OutputPortHandle<'a>,
    samples_out: InputPortHandle<'a>,
}

impl<'a> Voice<'a> {
    pub fn new() -> Self
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
            ports: ports,
            midi_frequency_in,
            midi_gate_in,
            samples_out,
        }
    }

    pub fn note_on(&mut self, freq: f32, vel: f32)
    {
        // TODO realtime safe
        // TODO velocity?
        self.ports.set_port_value(&self.midi_frequency_in, freq);
        self.ports.set_port_value(&self.midi_gate_in, 1.0);
    }

    pub fn note_off(&mut self, freq: f32)
    {
        // TODO realtime safe
        self.ports.set_port_value(&self.midi_gate_in, 0.0);
    }

    pub fn current_frequency(&self) -> Option<f32>
    {
        // TODO realtime safe
        if self.ports.get_port_value(&self.midi_gate_in) != 0.0 {
            Some(self.ports.get_port_value(&self.midi_frequency_in))
        } else {
            None
        }
    }

    pub fn add_component<T: Component<'a> + 'a>(&mut self, comp: T)
    {
        // TODO ensure name unique
        // TODO NOT realtime safe
        // TODO fix this up, do a sort
        self.components.push(Box::new(comp));
        let s = self.components.len();
        let comp = &mut self.components[s - 1];
        comp.initialize_ports(&mut self.ports);
    }

    /// Generate a single sample
    pub fn generate(&mut self) -> f32
    {
        // TODO realtime safe
        for comp in &mut self.components {
            comp.generate(&mut self.ports);
        }

        // get the value on the output wire
        self.ports.get_port_value(&self.samples_out)
    }

    pub fn get_components(&self) -> Vec<String>
    {
        // TODO NOT realtime safe
        // TODO return an iterator
        let mut ret = Vec::new();

        for comp in &self.components {
            ret.push(comp.get_name())
        }

        // add the voice in so we can connect to the voice ports
        ret.push("voice".to_string());

        ret
    }

    pub fn get_port_manager(&self) -> &PortManager<'a>
    {
        return &self.ports;
    }

    pub fn get_port_manager_mut(&mut self) -> &mut PortManager<'a>
    {
        return &mut self.ports;
    }

    pub fn example_connections(&mut self)
    {
        use components::{CombineInputs, Math, OnOff, SineWaveOscillator, SquareWaveOscillator};

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
        let pairs = [
            // push midi frequency the right places
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
            (("envelope", "samples_out"), ("voice", "samples_in")),
        ];

        for &(p1, p2) in &pairs {
            println!("connecting {:?} to {:?}", p1, p2);
            self.ports.connect_by_name(p1, p2).unwrap();
        }
    }
}
