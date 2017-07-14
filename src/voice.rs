use components::Component;
use ports::{InputPortHandle, OutputPortHandle, PortManagerImpl, PortName};
use ports::{PortManager, RealtimePortManager};

/// A single instance of the graph
#[derive(Debug)]
pub struct Voice<'a> {
    components: Vec<Box<Component<'a> + 'a>>,
    ports: PortManagerImpl<'a>,
    // we have a few default ports to contend with
    // these are populated and read from by the audio library
    midi_frequency_in: OutputPortHandle<'a>,
    midi_gate_in: OutputPortHandle<'a>,
    samples_out: InputPortHandle<'a>,
}

impl<'a> Voice<'a> {
    pub fn new() -> Self
    {
        let mut ports = PortManagerImpl::new();
        let midi_frequency_in = ports.register_output_port(
                &PortName::new("voice", "midi_frequency_out")) .unwrap();

        let midi_gate_in = ports.register_output_port(
            &PortName::new("voice", "midi_gate_out")) .unwrap();

        let samples_out = ports.register_input_port(
            &PortName::new("voice", "samples_in")) .unwrap();

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

    pub fn add_component(&mut self, comp: Box<Component<'a> + 'a>)
    {
        // TODO ensure name unique
        // TODO NOT realtime safe
        // TODO fix this up, do a sort
        self.components.push(comp);
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
}
