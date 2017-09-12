extern crate synth;

use synth::audioprops::AudioProperties;
use synth::components::Component;
use synth::components::{SquareWaveOscillator, SquareWaveOscillatorConfig};
use synth::ports::*;
use synth::util::ft;
use synth::util::nmat::*;

const NAME: &'static str = "square";
const FIN: &'static str = "fin";
const OUT: &'static str = "sout";

struct Square<'a> {
    square:  SquareWaveOscillator<'a>,
    ports:   PortManagerImpl<'a>,
    samples: InputPortHandle<'a>,
}

impl<'a> Square<'a> {
    pub fn new(freq: f32, srate: f32) -> Self {
        let mut ports = PortManagerImpl::new();
        let mut square = SquareWaveOscillator::new(Square::make_config());
        square.initialize_ports(&mut ports).unwrap();

        let f = PortName::new("dump", "f");
        let s = PortName::new("dump", "s");

        let frequency = ports.register_output_port(&f).unwrap();
        let samples = ports.register_input_port(&s).unwrap();

        ports.connect_by_name(&f, &PortName::new(NAME, FIN)).unwrap();
        ports.connect_by_name(&PortName::new(NAME, OUT), &s).unwrap();

        ports.set_port_value(&frequency, freq);
        square.handle_audio_property_change(AudioProperties::SampleRate(srate));

        Self {
            square,
            ports,
            // frequency,
            samples,
        }
    }

    pub fn go(&mut self) -> f32 {
        self.square.generate(&mut self.ports);
        self.ports.get_port_value(&self.samples)
    }

    fn make_config() -> SquareWaveOscillatorConfig
    {
        SquareWaveOscillatorConfig {
            name: NAME.to_owned(),
            frequency_input_name: FIN.to_owned(),
            samples_output_name: OUT.to_owned(),
        }
    }
}

fn main()
{
    let srate = 44100.0;
    let freq  = 440.0;
    let mut s = Square::new(freq, srate);

    let mut samples: Matrix<_, RowMajor> = Matrix::new((srate as usize, 1));
    //println!("samples.dim = {:?}", samples.dim());

    for i in 0..(srate as usize) {
        let sample = s.go();
        println!("{},{}", i, f32::to_string(&sample));
        samples[(i, 0)] = sample;
    }

    // let f = ft::reference_fourier(&samples);
    // for i in 0..(f.dim().0 / 2) {
    //     //println!("{},{}", i, f[(i, 0)].norm())
    // }
}
