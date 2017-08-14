use ports::{PortManager, RealtimePortManager, PortManagerError};
use audioprops::AudioProperties;

use std::fmt;

pub trait Component<'a>: fmt::Debug {
    /// Called when it is time for the component to generate audio on its output
    /// ports
    fn generate(&mut self, ports: &mut RealtimePortManager<'a>);

    /// Called with the audio system property that has changed
    /// A default noop implementation is provided
    fn handle_audio_property_change(&mut self, _props: AudioProperties) { }

    // port management
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>)
        -> Result<(), PortManagerError>;

    fn get_name(&self) -> String;
}

/// To be constructable from a config file, a component must implement this trait
pub trait ComponentConfig: fmt::Debug {
    /// Builds a component from a component config
    fn build_component<'a, 'b>(&'b self) -> Box<Component<'a> + 'a>;

    /// Clones the underlying config and returns it as a trait object
    fn box_clone(&self) -> Box<ComponentConfig>;
}

impl Clone for Box<ComponentConfig> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}
