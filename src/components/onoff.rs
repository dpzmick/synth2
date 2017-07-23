use components::{Component, ComponentConfig};
use ports::{InputPortHandle, OutputPortHandle, PortName};
use ports::{PortManager, RealtimePortManager, PortManagerError};

#[derive(Debug, Clone, StructValue, ForeignValue, FromValueClone)]
pub struct OnOffConfig {
    pub name: String,
}

impl ComponentConfig for OnOffConfig {
    fn build_component<'a, 'b>(&'b self) -> Box<Component<'a> + 'a>
    {
        Box::new(OnOff::new(self.name.clone()))
    }

    fn box_clone(&self) -> Box<ComponentConfig> {
        Box::new(self.clone())
    }
}

#[derive(Debug)]
pub struct OnOff<'a> {
    name: String,
    samples_in: Option<InputPortHandle<'a>>,
    gate_in: Option<InputPortHandle<'a>>,
    samples_out: Option<OutputPortHandle<'a>>,
}

impl<'a> OnOff<'a> {
    pub fn new(name: String) -> Self
    {
        Self {
            name,
            samples_in: None,
            gate_in: None,
            samples_out: None,
        }
    }
}

impl<'a> Component<'a> for OnOff<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>)
        -> Result<(), PortManagerError>
    {
        self.samples_in = Some(ports.register_input_port(
                &PortName::new(&self.name, "samples_in")) .unwrap());

        self.gate_in = Some(ports.register_input_port(
                &PortName::new(&self.name, "gate_in")).unwrap());

        self.samples_out = Some(ports.register_output_port(
                &PortName::new(&self.name, "samples_out")).unwrap());

        Ok( () )
    }

    fn generate(&mut self, ports: &mut RealtimePortManager)
    {
        if self.samples_in.is_none() || self.gate_in.is_none() ||
           self.samples_out.is_none() {
            return;
        }

        let samples = ports.get_port_value(&self.samples_in.unwrap());
        let gate = if ports.get_port_value(&self.gate_in.unwrap()) != 0.0 {
            1.0
        } else {
            0.0
        };

        let out = samples * gate;
        if ports.get_port_value(&self.samples_out.unwrap()) != out {
            ports.set_port_value(&self.samples_out.unwrap(), out);
        }
    }

    fn get_name(&self) -> String
    {
        self.name.clone()
    }
}
