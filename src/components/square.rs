use audioprops::AudioProperties;
use components::{Component, ComponentConfig};
use ports::{InputPortHandle, OutputPortHandle, PortName};
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
    phase: f32,
    sample_rate: Option<f32>,
    frequency_port: Option<InputPortHandle<'a>>,
    output_port: Option<OutputPortHandle<'a>>,
}

impl<'a> SquareWaveOscillator<'a> {
    pub fn new(config: SquareWaveOscillatorConfig) -> Self
    {
        Self {
            config: config,
            sample_rate: None,
            frequency_port: None,
            output_port: None,
            phase: 0.0,
        }
    }
}

impl<'a> Component<'a> for SquareWaveOscillator<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>)
        -> Result<(), PortManagerError>
    {
        let input_name = PortName::new(
            &self.config.name, &self.config.frequency_input_name);

        let output_name = PortName::new(
            &self.config.name, &self.config.samples_output_name);

        let res = ports.register_input_port(&input_name)
            .and_then(|port| {
                self.frequency_port = Some(port);
                ports.register_output_port(&output_name)
            })
            .map(|port| {
                self.output_port = Some(port);
            });

        res
    }

    fn generate(&mut self, ports: &mut RealtimePortManager<'a>)
    {
        use std::f32;

        if self.output_port.is_none() {
            return;
        }

        let mut f = ports.get_port_value(&self.frequency_port.unwrap());
        if f - 0.0 < 0.001 {
            ports.set_port_value(&self.output_port.unwrap(), 0.0);
            return;
        }

        // generate odd numbered sine waves up until nyquist frequency
        let mut sample = 0.0;
        let mut factor = 1.0;
        while (f * factor) < self.sample_rate.unwrap() / 2.0 {
            sample += (2.0 * f32::consts::PI * self.phase * factor).sin() / factor;
            factor += 2.0;
        }

        self.phase += f / self.sample_rate.unwrap();
        while self.phase > 1.0 {
            self.phase -= 1.0;
        }

        ports.set_port_value(&self.output_port.unwrap(), sample);
    }

    fn handle_audio_property_change(&mut self, prop: AudioProperties)
    {
        match prop {
            AudioProperties::SampleRate(r) => self.sample_rate = Some(r)
        }
    }

    fn get_name(&self) -> String
    {
        self.config.name.clone()
    }
}


// TODO test that 0 terminates
