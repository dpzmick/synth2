use components::Component;
use ports::{InputPortHandle, OutputPortHandle, PortName};
use ports::{PortManager, RealtimePortManager, PortManagerError};

use std::fmt;

pub struct Math<'a> {
    name: String,
    math_function: Box<Fn(f32) -> f32>,
    input: Option<InputPortHandle<'a>>,
    output: Option<OutputPortHandle<'a>>,
}

impl<'a> fmt::Debug for Math<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        write!(
            f, "Math {{ name: {:?}, function: [[opaque]], input: {:?}, output: {:?} }}",
            self.name, self.input, self.output)?;

        Ok(())
    }
}

impl<'a> Math<'a> {
    pub fn new<M: Fn(f32) -> f32 + 'static>(name: String, math: M) -> Self
    {
        Self {
            name,
            math_function: Box::new(math),
            input: None,
            output: None,
        }
    }

    // fn parse_math(expr: String) -> Box<Fn(f32) -> f32> {
    //     // TODO
    // }
}

impl<'a> Component<'a> for Math<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>)
        -> Result<(), PortManagerError>
    {
        self.input = Some(ports.register_input_port(
                &PortName::new(&self.name, "input"))?);

        self.output = Some(ports.register_output_port(
                &PortName::new(&self.name, "output"))?);

        Ok(())
    }

    fn generate(&mut self, ports: &mut RealtimePortManager)
    {
        let i = ports.get_port_value(&self.input.unwrap());
        ports.set_port_value(&self.output.unwrap(), (self.math_function)(i))
    }

    fn get_name(&self) -> String
    {
        self.name.clone()
    }
}
