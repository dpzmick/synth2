use audioprops::AudioProperties;
use components::{Component, ComponentConfig};
use ports::{InputPortHandle, OutputPortHandle, PortName};
use ports::{PortManager, RealtimePortManager, PortManagerError};

use std::f32;

#[derive(Debug, Clone, StructValue, ForeignValue, FromValueClone)]
pub struct SineWaveOscillatorConfig {
    pub name: String,
    pub frequency_input_name: String,
    pub samples_output_name: String,
}

impl ComponentConfig for SineWaveOscillatorConfig {
    fn build_component<'a, 'b>(&'b self) -> Box<Component<'a> + 'a>
    {
        Box::new(SineWaveOscillator::new(self.clone()))
    }

    fn box_clone(&self) -> Box<ComponentConfig> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct SineWaveOscillator<'a> {
    config: SineWaveOscillatorConfig,
    phase: f32,
    sample_rate: Option<f32>,
    frequency_port: Option<InputPortHandle<'a>>,
    output_port: Option<OutputPortHandle<'a>>,
}

impl<'a> SineWaveOscillator<'a> {
    pub fn new(config: SineWaveOscillatorConfig) -> Self
    {
        Self {
            config,
            phase: 0.0,
            sample_rate: None,
            frequency_port: None,
            output_port: None,
        }
    }

    fn sine(t: f32) -> f32
    {
        debug_assert!(t >= 0.0 && t <= 1.0);
        (2.0 * t * f32::consts::PI).sin()
    }

    fn fully_initialized(&self) -> bool
    {
        !self.frequency_port.is_none()
            && !self.output_port.is_none()
            && !self.sample_rate.is_none()
    }
}

impl<'a> Component<'a> for SineWaveOscillator<'a> {
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
        if !self.fully_initialized() {
            return;
        }

        let freq = ports.get_port_value(&self.frequency_port.unwrap());
        self.phase += freq / self.sample_rate.unwrap();;

        while self.phase > 1.0 {
            self.phase -= 1.0;
        }

        let v = SineWaveOscillator::sine(self.phase);
        ports.set_port_value(&self.output_port.unwrap(), v);
    }

    fn handle_audio_property_change(&mut self, prop: AudioProperties)
    {
        match prop {
            AudioProperties::SampleRate(r) => self.sample_rate = Some(r),
            _ => (),
        }
    }

    fn get_name(&self) -> String
    {
        self.config.name.clone()
    }
}
