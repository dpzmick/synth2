use components::{Component, SineWaveOscillator};
use ports::PortManager;

pub struct SquareWaveOscillator<'a> {
    name: String,
    sine: SineWaveOscillator<'a>,
}

impl<'a> SquareWaveOscillator<'a> {
    pub fn new(name: String) -> Self
    {
        Self {
            name: name.clone(),
            sine: SineWaveOscillator::new(name),
        }
    }
}

impl<'a> Component<'a> for SquareWaveOscillator<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>)
    {
        self.sine.initialize_ports(ports);
    }

    fn generate(&mut self, ports: &mut PortManager)
    {
        // find the sine out port
        let mut out = None;
        match ports.find_ports(&self.name) {
            Some(ports) => {
                for port in ports {
                    match port.promote_to_output() {
                        Ok(port) => out = Some(port),
                        Err(_) => (),
                    }
                }
            },
            None => (),
        };

        // we can write to the output port, then overwrite the value
        // nothing else can be generating while we are generating so there is no chance of this
        // value leaking into some other component

        match out {
            Some(out) => {
                self.sine.generate(ports);
                let v = ports.get_port_value(&out);

                if v < 0.0 {
                    ports.set_port_value(&out, -1.0);
                } else if v > 0.0 {
                    ports.set_port_value(&out, 1.0);
                }
            },

            None => (),
        }

    }
}