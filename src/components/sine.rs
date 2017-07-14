// TODO FIX THIS!
use SRATE;

use components::{Component, ComponentConfig};
use ports::{InputPortHandle, OutputPortHandle, PortManager, PortName};

use std::collections::HashMap;
use std::f32;

#[derive(Debug, Serialize, Deserialize)]
pub struct SineWaveOscillatorConfig {
    pub name: String,
}

impl ComponentConfig for SineWaveOscillatorConfig {
    fn build_component<'a, 'b>(&'b self) -> Box<Component<'a> + 'a>
    {
        Box::new(SineWaveOscillator::new(self.name.clone()))
    }
}

#[derive(Debug, Clone)]
pub struct SineWaveOscillator<'a> {
    name: String,
    phase: f32,
    frequency_port: Option<InputPortHandle<'a>>,
    output_port: Option<OutputPortHandle<'a>>,
}

impl<'a> SineWaveOscillator<'a> {
    pub fn new(name: String) -> Self
    {
        Self {
            name,
            phase: 0.0,
            frequency_port: None,
            output_port: None,
        }
    }

    fn sine(&self, t: f32) -> f32
    {
        debug_assert!(t >= 0.0 && t <= 1.0);
        (2.0 * t * f32::consts::PI).sin()
    }
}

impl<'a> Component<'a> for SineWaveOscillator<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>)
    {
        // TODO error handling?

        self.frequency_port = Some(ports
                                       .register_input_port(&PortName::new(self.get_name(),
                                                                           "frequency_in"))
                                       .unwrap());
        self.output_port = Some(ports
                                    .register_output_port(&PortName::new(self.get_name(),
                                                                         "samples_out"))
                                    .unwrap());
    }

    fn generate(&mut self, ports: &mut PortManager)
    {
        if self.frequency_port.is_none() || self.output_port.is_none() {
            return;
        }

        let freq = ports.get_port_value(&self.frequency_port.unwrap());
        self.phase += freq / SRATE;

        while self.phase > 1.0 {
            self.phase -= 1.0;
        }

        let v = self.sine(self.phase);
        ports.set_port_value(&self.output_port.unwrap(), v);
    }

    fn get_name(&self) -> String
    {
        self.name.clone()
    }
}
