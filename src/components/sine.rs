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

    #[cfg(test)]
    fn freq_port(&mut self) -> Option<InputPortHandle<'a>>
    {
        self.frequency_port
    }

    #[cfg(test)]
    fn output_port(&mut self) -> Option<OutputPortHandle<'a>>
    {
        self.output_port
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

        // TODO log if sample rate is less than 2x frequency (nyquist)

        // generate the current state first, then increment phase
        let v = SineWaveOscillator::sine(self.phase);
        let freq = ports.get_port_value(&self.frequency_port.unwrap());

        ports.set_port_value(&self.output_port.unwrap(), v);

        self.phase += freq / self.sample_rate.unwrap();
        while self.phase > 1.0 {
            self.phase -= 1.0;
        }
    }

    fn handle_audio_property_change(&mut self, prop: AudioProperties)
    {
        match prop {
            AudioProperties::SampleRate(r) => self.sample_rate = Some(r),
        }
    }

    fn get_name(&self) -> String
    {
        self.config.name.clone()
    }
}

// TODO this testing thing is a disaster, too much code

#[cfg(test)]
mod tests {
    use super::*;
    use ports::PortManagerImpl;

    const NAME: &'static str = "sine";
    const FIN: &'static str = "FIN";
    const SOUT: &'static str = "SOUT";

    fn get_config() -> SineWaveOscillatorConfig
    {
        SineWaveOscillatorConfig {
            name: NAME.to_owned(),
            frequency_input_name: FIN.to_owned(),
            samples_output_name: SOUT.to_owned(),
        }
    }

    struct Tester<'a> {
        pub sample_rate: f32,
        pub sine:        SineWaveOscillator<'a>,
        pub ports:       PortManagerImpl<'a>,
        pub freq_out:    OutputPortHandle<'a>,
        pub sample_in:   InputPortHandle<'a>
    }

    impl<'a> Tester<'a> {
        fn make_sine() -> (PortManagerImpl<'a>, SineWaveOscillator<'a>)
        {
            let mut ports = PortManagerImpl::new();
            let mut sine = SineWaveOscillator::new(get_config());
            sine.initialize_ports(&mut ports).unwrap();

            (ports, sine)
        }

        pub fn new() -> Self
        {
            let (mut ports, mut sine) = Tester::make_sine();

            let freq_out =
                ports.register_output_port(&PortName::new("test", "f")).unwrap();

            let sample_in =
                ports.register_input_port(&PortName::new("test", "s")).unwrap();

            ports.connect(&freq_out, &sine.freq_port().unwrap());
            ports.connect(&sine.output_port().unwrap(), &sample_in);

            let sample_rate = 1024.0;
            let mut s = Self {
                sample_rate,
                sine,
                ports,
                freq_out,
                sample_in,
            };

            s.set_sample_rate(sample_rate);

            s
        }

        pub fn set_sample_rate(&mut self, rate: f32)
        {
            self.sample_rate = rate;
            self.sine.handle_audio_property_change(
                AudioProperties::SampleRate(self.sample_rate));
        }
    }

    #[test]
    fn test_dft()
    {
        use util::nmat::{Matrix, RowMajor};
        use util::ft;

        // run a sine wave generator for an entire cycle (with sufficiently high
        // sample rate), the run a DFT over the output and ensure that only one
        // component (below Nyquist) is present

        let mut t = Tester::new();
        let freq = 1.0;
        t.ports.set_port_value(&t.freq_out, freq);
        t.set_sample_rate(4.0 * freq);

        // sampling 4x per one wave, so I need 4 samples
        let samples_needed = 4;

        let mut samples: Matrix<_, RowMajor> = Matrix::new((samples_needed, 1));

        // TODO some fancy generator?
        let mut i = 0;
        while i < samples_needed {
            t.sine.generate(&mut t.ports);

            let curr = t.ports.get_port_value(&t.sample_in);
            samples[(i, 0)] = curr;

            i += 1;
        }

        let out = ft::reference_fourier(&samples);

        assert_eq!(out.dim(), (4, 1));

        let mut significant_factors = 0;
        for r in 0..(out.dim().0 /2) {
            if out[(r, 0)].norm() > 0.01 {
                significant_factors += 1;
            }
        }

        assert_eq!(1, significant_factors);
    }
}
