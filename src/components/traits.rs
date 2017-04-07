use ports::PortManager;

pub trait Component<'a> {
    fn generate(&mut self, ports: &mut PortManager);
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>);
}

