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

pub trait ComponentConfigMaker {
    /// Create a decoder for a specific type
    /// TODO someday make this more generic somehow
    fn make_decoder<T: ComponentConfig + ketos::FromValue + 'static>(&self)
        -> Box<Fn(&Self) -> Result<Box<ComponentConfig>, String>>;

    /// Attempts to build a component config from some underlying config format
    /// Will iterate through every available decoder looking for the first one
    /// that works
    fn parse(&self) -> Result<Box<ComponentConfig>, String>
    {
        for decoder in self.get_all_decoders().into_iter() {
            if let Ok(config) = decoder(self) {
                return Ok(config);
            }
        }

        Err("NONE FOUND".to_owned())
    }

    fn get_all_decoders(&self) -> Vec<Box<Fn(&Self) -> Result<Box<ComponentConfig>, String>>>
    {
        use components::{SineWaveOscillatorConfig, SquareWaveOscillatorConfig};

        let mut decoders = Vec::new();
        // decoders.push(self.make_decoder::<SineWaveOscillatorConfig>());
        decoders.push(self.make_decoder::<SquareWaveOscillatorConfig>());

        decoders
    }
}
