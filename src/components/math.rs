use components::Component;
use ports::{InputPortHandle, OutputPortHandle, PortManager};

pub struct Math<'a> {
    name: String,
    math_function: Box<Fn(f32) -> f32>,
    input: Option<InputPortHandle<'a>>,
    output: Option<OutputPortHandle<'a>>,
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
    {
        self.input = Some(ports
                              .register_input_port(self.name.clone(), "input".to_string())
                              .unwrap());

        self.output = Some(ports
                               .register_output_port(self.name.clone(), "output".to_string())
                               .unwrap());
    }

    fn generate(&mut self, ports: &mut PortManager)
    {
        let i = ports.get_port_value(&self.input.unwrap());
        ports.set_port_value(&self.output.unwrap(), (self.math_function)(i))
    }
}
