extern crate synth;

use synth::audioprops::AudioProperties;
use synth::components::Component;
use synth::components::{SineWaveOscillator, SineWaveOscillatorConfig};
use synth::ports::*;

const SINE_NAME: &'static str = "sine";
const SINE_FIN: &'static str = "fin";
const SINE_OUT: &'static str = "sout";

struct Sine<'a> {
    sine:      SineWaveOscillator<'a>,
    ports:     PortManagerImpl<'a>,
    // frequency: OutputPortHandle<'a>,
    samples:   InputPortHandle<'a>,
}

impl<'a> Sine<'a> {
    pub fn new(freq: f32, srate: f32) -> Self {
        let mut ports = PortManagerImpl::new();
        let mut sine = SineWaveOscillator::new(Sine::make_config());
        sine.initialize_ports(&mut ports).unwrap();

        let f = PortName::new("dump", "f");
        let s = PortName::new("dump", "s");

        let frequency = ports.register_output_port(&f).unwrap();
        let samples = ports.register_input_port(&s).unwrap();

        ports.connect_by_name(&f, &PortName::new(SINE_NAME, SINE_FIN)).unwrap();
        ports.connect_by_name(&PortName::new(SINE_NAME, SINE_OUT), &s).unwrap();

        ports.set_port_value(&frequency, freq);
        sine.handle_audio_property_change(AudioProperties::SampleRate(srate));

        Self {
            sine,
            ports,
            // frequency,
            samples,
        }
    }

    pub fn go(&mut self) -> f32 {
        self.sine.generate(&mut self.ports);
        self.ports.get_port_value(&self.samples)
    }

    fn make_config() -> SineWaveOscillatorConfig
    {
        SineWaveOscillatorConfig {
            name: SINE_NAME.to_owned(),
            frequency_input_name: SINE_FIN.to_owned(),
            samples_output_name: SINE_OUT.to_owned(),
        }
    }
}

fn main()
{
    let srate = 100.0;
    let freq  = 1.0;
    let mut s = Sine::new(freq, srate);

    for i in 0..(srate as usize * 2) {
        println!("{},{}", i, f32::to_string(&s.go()))
    }
}
