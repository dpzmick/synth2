use ports::PortManager;

use std::fmt;

pub trait Component<'a>: fmt::Debug {
    // audio generation
    fn generate(&mut self, ports: &mut PortManager);
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>);

    // ui convenience
    fn get_name(&self) -> String;
}
