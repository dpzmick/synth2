use audioprops::AudioProperties;
use components::{Component, ComponentConfig};
use ports::{InputPortHandle, OutputPortHandle, PortName};
use ports::{PortManager, RealtimePortManager, PortManagerError};

#[derive(Debug, Clone, StructValue, ForeignValue, FromValueClone)]
pub struct SimpleLowPassConfig {
    pub name: String,
    pub input_name: String,
    pub output_name: String,
}

impl ComponentConfig for SimpleLowPassConfig {
    fn build_component<'a, 'b>(&'b self) -> Box<Component<'a> +'a>
    {
        Box::new(SimpleLowPass::new(self.clone()))
    }

    fn box_clone(&self) -> Box<ComponentConfig>
    {
        Box::new(self.clone())
    }
}

#[derive(Debug)]
pub struct SimpleLowPass<'a> {
    config: SimpleLowPassConfig,
    last: Option<f32>,
    input_port: Option<InputPortHandle<'a>>,
    output_port: Option<OutputPortHandle<'a>>,
}

impl<'a> SimpleLowPass<'a> {
    pub fn new(config: SimpleLowPassConfig) -> Self
    {
        Self {
            config,
            last: None,
            input_port: None,
            output_port: None,
        }
    }

    fn fully_initialized(&self) -> bool
    {
        !self.input_port.is_none() && !self.output_port.is_none()
    }
}

impl<'a> Component<'a> for SimpleLowPass<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>)
        -> Result<(), PortManagerError>
    {
        let input_name = PortName::new(&self.config.name, &self.config.input_name);
        let output_name = PortName::new(&self.config.name, &self.config.output_name);

        ports.register_input_port(&input_name)
            .and_then(|port| {
                self.input_port = Some(port);
                ports.register_output_port(&output_name)
            })
            .map(|port| {
                self.output_port = Some(port);
            })
    }

    fn generate(&mut self, ports: &mut RealtimePortManager)
    {
        if !self.fully_initialized() {
            return;
        }

        let x = ports.get_port_value(&self.input_port.unwrap());

        if self.last.is_none() {
            self.last = Some(x);
        }
        else {
            let last = self.last.unwrap();
            let v = x + last;
            self.last = Some(v);
        }

        ports.set_port_value(&self.output_port.unwrap(), self.last.unwrap());
    }

    fn get_name(&self) -> String
    {
        self.config.name.clone()
    }
}
