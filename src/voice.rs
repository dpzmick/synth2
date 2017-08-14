use audioprops::AudioProperties;
use components::Component;
use patch::Patch;
use ports::{InputPortHandle, OutputPortHandle, PortManagerImpl, PortName};
use ports::{PortManager, RealtimePortManager, PortManagerError};
use topo;

use std::collections::HashMap;

/// Monophonic set of components.
#[derive(Debug)]
pub struct Voice<'a> {
    components: Vec<Box<Component<'a> + 'a>>,
    ports: PortManagerImpl<'a>,
    // we have a few default ports to contend with
    // these are populated and read from by the audio library
    midi_frequency_in: OutputPortHandle<'a>,
    midi_gate_in: OutputPortHandle<'a>,
    midi_vel_in: OutputPortHandle<'a>,
    midi_control_ports: Vec<OutputPortHandle<'a>>,
    samples_out: InputPortHandle<'a>,
}

impl<'a> Voice<'a> {
    // TODO actually leverage the realtime port manager trait?
    pub fn new(patch: &Patch) -> Result<Self, PortManagerError>
    {
        let mut ports = PortManagerImpl::new();
        let midi_frequency_in = ports.register_output_port(
            &PortName::new("voice", "midi_frequency_out"))?;

        let midi_gate_in = ports.register_output_port(
            &PortName::new("voice", "midi_gate_out"))?;

        let midi_vel_in = ports.register_output_port(
            &PortName::new("voice", "midi_velocity_out"))?;

        let samples_out = ports.register_input_port(
            &PortName::new("voice", "samples_in"))?;

        // register all of the midi control signals
        // there are 128 total available in the midi spec
        let mut midi_control_ports = Vec::new();
        for i in 0..128 {
            let n = format!("midi_control_{}", i);
            let pn = PortName::new("voice", n);
            midi_control_ports.push(ports.register_output_port(&pn)?);
        }

        // Register all of the components with the port manager
        let mut components = Vec::new();

        for config in patch.components.iter() {
            let mut comp = config.build_component();
            comp.initialize_ports(&mut ports)?;
            components.push(comp);
        }

        // TODO consider moving the String -> id mappings somewhere else
        // - need to think through exactly how ports will be looked up
        // - don't store the ports more than once (easy to get out of sync)

        // now connect everything according to the patch
        for connection in patch.connections.iter() {
            ports.connect_by_name(&connection.first, &connection.second)?;
        }

        // now the port manager knows all of the connections, we topologically
        // sort the component connection graph. Some components will use values
        // produced by other components. We need to do this sort to make sure
        // that the all of the port values get updated in the right order.
        let (names, mut adj) = ports.get_component_adjacency_matrix();

        // TODO make sure that the (0, 0) element is always a sane one to start
        // from.

        // To allow cycles in the component graph, remove any edges which would
        // form a cycle by pointing back to a previously visited component
        topo::remove_back_edges(&mut adj);

        let ordering = topo::topological_sort(&mut adj);

        // figure out what that ordering means, and reorder the vector of
        // components
        let mut order_by_component_name = HashMap::new();
        for (index, element) in ordering.iter().enumerate() {
            let comp_name = names.get(element).unwrap();
            order_by_component_name.insert(comp_name, index);
        }

        components.sort_by(|ref e1, ref e2| {
            let o1 = order_by_component_name[&e1.get_name()];
            let o2 = order_by_component_name[&e2.get_name()];

            o1.cmp(&o2)
        });

        // phew, we made it out alive
        Ok(Self {
            components,
            ports,
            midi_frequency_in,
            midi_vel_in,
            midi_gate_in,
            midi_control_ports,
            samples_out,
        })
    }

    pub fn note_on(&mut self, freq: f32, vel: f32)
    {
        // TODO realtime safe
        self.ports.set_port_value(&self.midi_frequency_in, freq);
        self.ports.set_port_value(&self.midi_gate_in, 1.0);
        self.ports.set_port_value(&self.midi_vel_in, vel);
    }

    pub fn note_off(&mut self, _freq: f32)
    {
        // TODO realtime safe
        self.ports.set_port_value(&self.midi_gate_in, 0.0);
    }

    pub fn control_value_change(&mut self, cc: u8, new_val: u8)
    {
        let handle = &self.midi_control_ports[cc as usize];
        self.ports.set_port_value(handle, new_val as f32);
    }

    pub fn handle_audio_property_change(&mut self, prop: AudioProperties)
    {
        for comp in &mut self.components {
            comp.handle_audio_property_change(prop);
        }
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
}
