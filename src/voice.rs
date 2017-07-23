use components::Component;
use patch::Patch;
use ports::{InputPortHandle, OutputPortHandle, PortManagerImpl, PortName};
use ports::{PortManager, RealtimePortManager, PortManagerError};
use topo;

use std::collections::HashMap;

/// A single instance of the graph
#[derive(Debug)]
pub struct Voice<'a> {
    components: Vec<Box<Component<'a> + 'a>>,
    ports: PortManagerImpl<'a>,
    // we have a few default ports to contend with
    // these are populated and read from by the audio library
    midi_frequency_in: OutputPortHandle<'a>,
    midi_gate_in: OutputPortHandle<'a>,
    midi_control_ports: Vec<OutputPortHandle<'a>>,
    samples_out: InputPortHandle<'a>,
}

impl<'a> Voice<'a> {
    // TODO actually leverage the realtime port manager trait?
    pub fn new(patch: &Patch) -> Result<Self, PortManagerError>
    {
        let mut ports = PortManagerImpl::new();
        let midi_frequency_in = ports.register_output_port(
            &PortName::new("voice", "midi_frequency_out")).unwrap();

        let midi_gate_in = ports.register_output_port(
            &PortName::new("voice", "midi_gate_out")).unwrap();

        let samples_out = ports.register_input_port(
            &PortName::new("voice", "samples_in")).unwrap();

        let mut midi_control_ports = Vec::new();
        // register all of the midi control signals
        // there are 128 total available in the midi spec
        for i in 0..128 {
            let n = format!("midi_control_{}", i);
            let pn = PortName::new("voice", n);
            midi_control_ports.push(ports.register_output_port(&pn).unwrap());
        }

        // First, register all of the components with the port manager
        let mut components = Vec::new();

        for config in patch.components.iter() {
            let mut comp = config.build_component();
            if let Err(e) = comp.initialize_ports(&mut ports) {
                return Err(e);
            }

            components.push(comp);
        }

        // TODO this would be a lot less of a disaster if I stopped using
        // strings for everything
        // I've sort of done this because the port manager is the only bit of
        // the application that actually cares about ports once everything is
        // inited

        // now connect everything according to the patch
        for connection in patch.connections.iter() {
            let res = ports.connect_by_name(
                &connection.first, &connection.second);

            if let Err(err) = res {
                return Err(err);
            }
        }

        // now the port manager knows all of the connections, lets do the sort!
        let (names, mut adj) = ports.get_component_adjacency_matrix();
        println!("names: {:?}", names);
        println!("adj: {:?}", adj);

        // now, remove any back edges
        topo::remove_back_edges(&mut adj);
        println!("adj: {:?}", adj);

        // topsort it all
        let ordering = topo::topological_sort(&mut adj);
        println!("ordering: {:?}", ordering);

        // figure out what that ordering means, and reorder the vector of
        // components
        let mut order_by_component_name = HashMap::new();
        for (index, element) in ordering.iter().enumerate() {
            let comp_name = names.get(element).unwrap();
            order_by_component_name.insert(comp_name, index);
        }

        println!("order_by_name: {:?}", order_by_component_name);

        components.sort_by(|ref e1, ref e2| {
            let o1 = order_by_component_name[&e1.get_name()];
            let o2 = order_by_component_name[&e2.get_name()];

            o1.cmp(&o2)
        });

        Ok(Self {
            components,
            ports: ports,
            midi_frequency_in,
            midi_gate_in,
            midi_control_ports,
            samples_out,
        })
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

    pub fn control_value_change(&mut self, cc: u8, new_val: u8)
    {
        let handle = &self.midi_control_ports[cc as usize];
        self.ports.set_port_value(handle, new_val as f32);
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

    pub fn get_port_manager(&self) -> &PortManager<'a>
    {
        return &self.ports;
    }

    pub fn get_port_manager_mut(&mut self) -> &mut PortManager<'a>
    {
        return &mut self.ports;
    }
}
