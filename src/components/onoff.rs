use components::Component;
use ports::{InputPortHandle, OutputPortHandle, PortManager};

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
    {
        self.samples_in = Some(ports
                                   .register_input_port(self.name.clone(),
                                                        "samples_in".to_string())
                                   .unwrap());

        self.gate_in = Some(ports
                                .register_input_port(self.name.clone(), "gate_in".to_string())
                                .unwrap());

        self.samples_out = Some(ports
                                    .register_output_port(self.name.clone(),
                                                          "samples_out".to_string())
                                    .unwrap());
    }

    fn generate(&mut self, ports: &mut PortManager)
    {
        if self.samples_in.is_none() || self.gate_in.is_none() || self.samples_out.is_none() {
            return;
        }

        let samples = ports.get_port_value(&self.samples_in.unwrap());
        let gate = if ports.get_port_value(&self.gate_in.unwrap()) != 0.0 { 1.0 } else { 0.0 };

        let out = samples * gate;
        if ports.get_port_value(&self.samples_out.unwrap()) != out {
            ports.set_port_value(&self.samples_out.unwrap(), out);
        }
    }
}
