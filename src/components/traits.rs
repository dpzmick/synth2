use ports::PortManager;

pub trait Component<'a> {
    // audio generation
    fn generate(&mut self, ports: &mut PortManager);
    fn initialize_ports(&mut self, ports: &mut PortManager<'a>);

    // ui convenience
    fn get_name(&self) -> String;
}
