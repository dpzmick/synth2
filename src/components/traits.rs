use ketos;

use ports::PortManager;

use std::collections::HashMap;
use std::fmt;

pub trait Component<'a>: fmt::Debug {
    // audio generation
    fn generate(&mut self, ports: &mut PortManager);
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>);

    // ui convenience
    fn get_name(&self) -> String;
}

/// To be constructable from a config file, a component must implement this trait
pub trait ComponentConfig: fmt::Debug {
    /// Builds a component from a component config
    fn build_component<'a, 'b>(&'b self) -> Box<Component<'a> + 'a>;
}
