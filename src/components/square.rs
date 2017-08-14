use audioprops::AudioProperties;
use components::{Component, ComponentConfig};
use components::{SineWaveOscillator, SineWaveOscillatorConfig};
use ports::{OutputPortHandle, PortName};
use ports::{PortManager, RealtimePortManager, PortManagerError};

#[derive(Debug, Clone, StructValue, ForeignValue, FromValueClone)]
pub struct SquareWaveOscillatorConfig {
    pub name: String,
    pub frequency_input_name: String,
    pub samples_output_name: String,
}

impl ComponentConfig for SquareWaveOscillatorConfig {
    fn build_component<'a, 'b>(&'b self) -> Box<Component<'a> + 'a>
    {
        Box::new(SquareWaveOscillator::new(self.clone()))
    }

    fn box_clone(&self) -> Box<ComponentConfig> {
        Box::new(self.clone())
    }
}

#[derive(Debug)]
pub struct SquareWaveOscillator<'a> {
    config: SquareWaveOscillatorConfig,
    sine: SineWaveOscillator<'a>,
    output_port: Option<OutputPortHandle<'a>>,
}

impl<'a> SquareWaveOscillator<'a> {
    pub fn new(config: SquareWaveOscillatorConfig) -> Self
    {
        let sconfig = SineWaveOscillatorConfig {
            name: config.name.clone(),
            frequency_input_name: config.frequency_input_name.clone(),
            samples_output_name: config.samples_output_name.clone(),
        };

        Self {
            config: config,
            sine: SineWaveOscillator::new(sconfig),
            output_port: None,
        }
    }
}

impl<'a> Component<'a> for SquareWaveOscillator<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>)
        -> Result<(), PortManagerError>
    {
        self.sine.initialize_ports(ports)?;

        // we don't actually care about the input port, only need to grab a
        // handle to the output port
        let output = PortName::new(
            &self.config.name, &self.config.samples_output_name);

        // if this fails, the sine wave generator is possibly rewriting port
        // names and we need to be more clever
        ports.find_port(&output)
            .ok_or(PortManagerError::NoSuchPort(output))
            .and_then(|port| {
                port.promote_to_output()
            }).map(|port| {
                self.output_port = Some(port);
            })
    }

    fn generate(&mut self, ports: &mut RealtimePortManager<'a>)
    {
        if self.output_port.is_none() {
            return;
        }

        // Let the sine wave generator generate a sine wave at the output port,
        // then overwrite the value. Nothing else can be running `generate`
        // while we are running `generate` so there is no chance of the sine
        // wave value leaking into some other component

        self.sine.generate(ports);

        let out = self.output_port.unwrap();
        let v = ports.get_port_value(&out);

        if v < 0.0 {
            ports.set_port_value(&out, -1.0);
        } else if v > 0.0 {
            ports.set_port_value(&out, 1.0);
        }
    }

    fn handle_audio_property_change(&mut self, prop: AudioProperties)
    {
        // sine needs some of these updates
        self.sine.handle_audio_property_change(prop);
    }

    fn get_name(&self) -> String
    {
        self.config.name.clone()
    }
}
