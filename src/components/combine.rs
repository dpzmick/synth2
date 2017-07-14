use components::Component;
use ports::{InputPortHandle, OutputPortHandle, PortManager, PortName};

// has many inputs and combines them proportionally to how many are emitting a
// signal
#[derive(Debug)]
pub struct CombineInputs<'a> {
    name: String,
    num_inputs: usize, // needed when initializing ports
    inputs: Vec<InputPortHandle<'a>>,
    output: Option<OutputPortHandle<'a>>,
}

impl<'a> CombineInputs<'a> {
    pub fn new(name: String, num_inputs: usize) -> Self
    {
        Self {
            name,
            num_inputs,
            inputs: Vec::new(),
            output: None,
        }
    }
}

impl<'a> Component<'a> for CombineInputs<'a> {
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>)
    {
        for i in 0..self.num_inputs {
            let iname = format!("{}_input{}", self.name, i);
            let i = ports.register_input_port(&PortName::new(&self.name, iname));
            self.inputs.push(i.unwrap());
        }

        self.output = Some(ports
                               .register_output_port(&PortName::new(&self.name, "out"))
                               .unwrap());
    }

    fn generate(&mut self, ports: &mut PortManager)
    {
        let mut count: usize = 0;
        let mut input_sum = 0.0;
        for input in self.inputs.iter() {
            let i = ports.get_port_value(input);
            if i != 0.0 {
                input_sum += i;
                count += 1;
            }
        }

        if count > 0 {
            let v = input_sum * (1.0 / count as f32);
            ports.set_port_value(&self.output.unwrap(), v);
        }
    }

    fn get_name(&self) -> String
    {
        self.name.clone()
    }
}
