use std::collections::HashMap;
use std::marker::PhantomData;

// I'm not really sure I like the way this port handle thing is working out
// This is quite a bit of complexity just to get a little a bit of extra type safety

pub type PortId = usize;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum PortDirection {
    Input,
    Output,
}

pub trait PortHandle {
    fn id(&self) -> PortId;
    fn direction(&self) -> PortDirection;
}

impl PartialEq<PortHandle> for PortHandle {
    fn eq(&self, other: &Self) -> bool
    {
        self.id() == other.id() && self.direction() == other.direction()
    }
}

/// Can be promoted to an input or output port handle
#[derive(Clone, Copy, Debug)]
pub struct UnknownPortHandle<'a> {
    dir: PortDirection,
    id: PortId,

    // used to enforce the lifetime constraint
    phantom: PhantomData<&'a usize>,
}

impl<'a> PortHandle for UnknownPortHandle<'a> {
    fn id(&self) -> PortId
    {
        self.id
    }

    fn direction(&self) -> PortDirection
    {
        self.dir
    }
}

impl<'a> UnknownPortHandle<'a> {
    pub fn promote_to_output(self) -> Result<OutputPortHandle<'a>, PortManagerError>
    {
        match self.dir {
            PortDirection::Output => {
                Ok(OutputPortHandle {
                       id: self.id,
                       phantom: PhantomData,
                   })
            },
            PortDirection::Input => Err(PortManagerError::NotOutputPort),
        }
    }

    pub fn promote_to_input(self) -> Result<InputPortHandle<'a>, PortManagerError>
    {
        match self.dir {
            PortDirection::Input => {
                Ok(InputPortHandle {
                       id: self.id,
                       phantom: PhantomData,
                   })
            },
            PortDirection::Output => Err(PortManagerError::NotInputPort),
        }
    }
}

impl<'a, T: PortHandle> PartialEq<T> for UnknownPortHandle<'a> {
    fn eq(&self, other: &T) -> bool
    {
        self.id() == other.id() && self.direction() == other.direction()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InputPortHandle<'a> {
    id: PortId,

    // used to enforce the lifetime constraint
    phantom: PhantomData<&'a usize>,
}

impl<'a> PortHandle for InputPortHandle<'a> {
    fn id(&self) -> PortId
    {
        self.id
    }
    fn direction(&self) -> PortDirection
    {
        PortDirection::Input
    }
}

impl<'a, T: PortHandle> PartialEq<T> for InputPortHandle<'a> {
    fn eq(&self, other: &T) -> bool
    {
        self.id() == other.id() && self.direction() == other.direction()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OutputPortHandle<'a> {
    id: PortId,

    // used to enforce the lifetime constraint
    phantom: PhantomData<&'a usize>,
}

impl<'a> PortHandle for OutputPortHandle<'a> {
    fn id(&self) -> PortId
    {
        self.id
    }
    fn direction(&self) -> PortDirection
    {
        PortDirection::Output
    }
}

impl<'a, T: PortHandle> PartialEq<T> for OutputPortHandle<'a> {
    fn eq(&self, other: &T) -> bool
    {
        self.id() == other.id() && self.direction() == other.direction()
    }
}

#[derive(PartialEq, Debug)]
pub enum PortManagerError {
    PortsNotUnique,
    NotOutputPort,
    NotInputPort,
    NoSuchPort,
}

/// A port manager manages the connections between different components
/// Every component can register a variety of input and output ports with the port manager.
/// When two ports are connected, any values written to the "Input" end of the port will also be
/// written to the "Output" end
/// An input may only have a single incoming connection, but, an output port may be connected to
/// many outputs
pub struct PortManager<'a> {
    // (very poor) graph implementation
    ports: Vec<f32>,
    connections: Vec<(usize, usize)>,

    // metadata information
    // component_name -> (port_name -> PortId)
    ports_meta: HashMap<String, HashMap<String, UnknownPortHandle<'a>>>,

    // used to enforce the lifetime constraints
    phantom: PhantomData<&'a usize>,
}

// private impl
impl<'a> PortManager<'a> {
    fn check_key_usable(&self, component: &str, port: &str) -> bool
    {
        match self.ports_meta.get(component) {
            Some(comp_ports) => {
                match comp_ports.get(port) {
                    Some(_) => return false,
                    None => return true,
                }
            },

            None => {
                return true;
            },
        }
    }

    fn save_port_meta(&mut self,
                      component: String,
                      port_name: String,
                      id: PortId,
                      direction: PortDirection)
    {
        // todo could probably inline this and check_key_usable for more faster
        let e = self.ports_meta
            .entry(component)
            .or_insert_with(|| HashMap::new());

        e.entry(port_name)
            .or_insert(UnknownPortHandle {
                           id: id,
                           dir: direction,
                           phantom: PhantomData,
                       });
    }

    fn new_port(&mut self,
                component: String,
                port_name: String,
                direction: PortDirection)
        -> Result<usize, PortManagerError>
    {
        // TODO not realtime safe
        if !self.check_key_usable(&component, &port_name) {
            return Err(PortManagerError::PortsNotUnique);
        }

        self.ports.push(0.0);
        let id = self.ports.len() - 1;
        self.save_port_meta(component, port_name, id, direction);
        Ok(id)
    }
}

// public impl
impl<'a> PortManager<'a> {
    pub fn new() -> Self
    {
        Self {
            ports: Vec::new(),
            connections: Vec::new(),
            ports_meta: HashMap::new(),
            phantom: PhantomData,
        }
    }

    /// Register a new port
    /// Each port is associated with a component, and must be given a name
    /// For a single component, each port name must be unique
    pub fn register_input_port(&mut self,
                               component: String,
                               port_name: String)
        -> Result<InputPortHandle<'a>, PortManagerError>
    {
        self.new_port(component, port_name, PortDirection::Input)
            .map(|id| {
                InputPortHandle {
                    id,
                    phantom: PhantomData,
                }
            })
    }

    /// Register a new port
    /// Each port is associated with a component, and must be given a name
    /// For a single component, each port name must be unique
    pub fn register_output_port(&mut self,
                                component: String,
                                port_name: String)
        -> Result<OutputPortHandle<'a>, PortManagerError>
    {
        self.new_port(component, port_name, PortDirection::Output)
            .map(|id| {
                OutputPortHandle {
                    id,
                    phantom: PhantomData,
                }
            })
    }

    /// Get the current value of the port.
    /// If the port handle was registered with this PortManager, this will never fail because ports
    /// cannot be destroyed.
    /// Calling this function with a handle to a port from a different PortManager is undefined
    /// behavior.
    pub fn get_port_value(&self, p: &PortHandle) -> f32
    {
        self.ports[p.id()]
    }

    /// Set the current value of the port.
    /// Calling this function with a handle to a port from a different PortManager is undefined
    /// behavior.
    /// This may only be called on an Output port
    pub fn set_port_value(&mut self, p: &OutputPortHandle, val: f32)
    {
        // assert cannot allocate or resize
        self.ports[p.id] = val;

        for &(p1, p2) in self.connections.iter() {
            if p1 == p.id {
                self.ports[p2] = val;
            }
        }
    }

    /// Connect ports, the value on the output port will always be available on the input port
    /// Note that this will always succeed, as long as both of the port handles are owned by this
    /// PortManager
    /// It is impossible to request an invalid connection due to type safety.
    pub fn connect(&mut self, p1: &OutputPortHandle, p2: &InputPortHandle)
    {
        // TODO not realtime safe
        self.connections.push((p1.id, p2.id));
    }

    /// Disconnect ports
    /// If two ports are already connected, this will remove the connection between them
    /// If they are not connected, this will be a potentially expensive noop
    /// It is impossible to request an invalid disconnection due to type safety
    pub fn disconnect(&mut self, p1: &OutputPortHandle, p2: &InputPortHandle)
    {
        // TODO not relatime safe
        // this will probably technically be realtime safe, but that isn't the interface we want to
        // expose
        self.connections
            .retain(|&(a, b)| a != p1.id && b != p2.id);
    }

    /// Connects a pair of ports by name. Each pair is given as (component, port)
    pub fn connect_by_name(&mut self,
                           p1: (&str, &str),
                           p2: (&str, &str))
        -> Result<(), PortManagerError>
    {
        // lookup both of the ports that are requested
        self.find_port(p1.0, p1.1)
            .ok_or(PortManagerError::NoSuchPort)
            .and_then(|port| port.promote_to_output())
            .and_then(|output| {
                self.find_port(p2.0, p2.1)
                    .ok_or(PortManagerError::NoSuchPort)
                    .and_then(|port| port.promote_to_input())
                    .map(|input| (output, input))
            })
            .and_then(|(output, input)| {
                // if we get this far, we have both ports, so connect them
                self.connect(&output, &input);
                Ok(())
            })
    }

    pub fn find_port(&self, component: &str, port_name: &str) -> Option<UnknownPortHandle<'a>>
    {
        // must be realtime safe
        self.ports_meta
            .get(component)
            .and_then(|comp| comp.get(port_name))
            .map(|port_name| port_name.clone())
    }

    pub fn find_ports(&self, component: &str) -> Option<Vec<UnknownPortHandle<'a>>>
    {
        // TODO must be realtime safe
        self.ports_meta
            .get(component)
            .map(|comp| {
                let mut res = Vec::new(); // TODO I can't be doing this!
                for handle in comp.values() {
                    res.push(handle.clone())
                }

                res
            })
    }

    /// Get all of the port connections known by the system
    pub fn get_connections(&self) -> Vec<(OutputPortHandle, InputPortHandle)>
    {
        // not realtime safe
        let mut v = Vec::new();
        for &(o, i) in self.connections.iter() {
            let e = (OutputPortHandle {
                         id: o,
                         phantom: PhantomData,
                     },
                     InputPortHandle {
                         id: i,
                         phantom: PhantomData,
                     });
            v.push(e);
        }

        v
    }

    /// Lookup the component associated with a port handle
    pub fn get_component<P: PortHandle>(&self, p: P) -> Option<String>
    {
        // probably realtime safe but super slow
        for (component, ports) in self.ports_meta.iter() {
            for (port, handle) in ports.iter() {
                if handle.id() == p.id() {
                    return Some(component.clone());
                }
            }
        }

        return None;
    }
}

#[test]
fn test_set_get()
{
    let mut manager = PortManager::new();
    let port = manager
        .register_output_port("test".to_string(), "out".to_string())
        .unwrap();

    let expected = 10.0;
    manager.set_port_value(&port, expected);
    let res = manager.get_port_value(&port);

    assert!(expected == res);
}

#[test]
fn test_connect()
{
    let mut manager = PortManager::new();
    let port1 = manager
        .register_output_port("test".to_string(), "out".to_string())
        .unwrap();
    let port2 = manager
        .register_input_port("test".to_string(), "in".to_string())
        .unwrap();

    manager.connect(&port1, &port2);

    let expected = 10.0;
    manager.set_port_value(&port1, expected);

    assert!(manager.get_port_value(&port1) == expected);
    assert!(manager.get_port_value(&port2) == expected);
}

#[test]
fn test_duplicate_port1()
{
    let mut manager = PortManager::new();
    let port1 = manager.register_output_port("test".to_string(), "name".to_string());
    let port2 = manager.register_output_port("test".to_string(), "name".to_string());

    assert!(port1.is_ok());
    assert!(port2.is_err());
}

#[test]
fn test_duplicate_port2()
{
    let mut manager = PortManager::new();
    let port1 = manager.register_input_port("test".to_string(), "name".to_string());
    let port2 = manager.register_input_port("test".to_string(), "name".to_string());

    assert!(port1.is_ok());
    assert!(port2.is_err());
}

#[test]
fn test_duplicate_port3()
{
    let mut manager = PortManager::new();
    let port1 = manager.register_input_port("test".to_string(), "name".to_string());
    let port2 = manager.register_output_port("test".to_string(), "name".to_string());

    assert!(port1.is_ok());
    assert!(port2.is_err());
}

#[test]
fn test_disconnect()
{
    let mut manager = PortManager::new();
    let port1 = manager
        .register_output_port("test".to_string(), "out".to_string())
        .unwrap();
    let port2 = manager
        .register_input_port("test".to_string(), "in".to_string())
        .unwrap();

    manager.connect(&port1, &port2);
    manager.disconnect(&port1, &port2);

    manager.set_port_value(&port1, 10.0);
    assert!(manager.get_port_value(&port2) != 10.0);
}

#[test]
fn test_find1()
{
    let mut manager = PortManager::new();
    let port1 = manager
        .register_output_port("test".to_string(), "out".to_string())
        .unwrap();

    let also_port1 = manager.find_port("test", "out");
    assert!(also_port1.is_some());

    let also_port1 = also_port1.unwrap().promote_to_output();

    assert!(also_port1.is_ok());
    let also_port1 = also_port1.unwrap();

    assert!(port1 == also_port1);
}

#[test]
fn test_find2()
{
    let mut manager = PortManager::new();
    let port1 = manager
        .register_input_port("test".to_string(), "out".to_string())
        .unwrap();

    let also_port1 = manager.find_port("test", "out");
    assert!(also_port1.is_some());

    let also_port1 = also_port1.unwrap().promote_to_input();

    assert!(also_port1.is_ok());
    let also_port1 = also_port1.unwrap();

    assert!(port1 == also_port1);
}

#[test]
fn test_find3()
{
    let mut manager = PortManager::new();
    let port1 = manager
        .register_input_port("test".to_string(), "out1".to_string())
        .unwrap();
    let port2 = manager
        .register_input_port("test".to_string(), "out2".to_string())
        .unwrap();

    let ports = manager.find_ports("test");
    assert!(ports.is_some());

    let mut p1_found = false;
    let mut p2_found = false;
    for port in ports.unwrap().into_iter() {
        if port == port1 {
            p1_found = true;
        }

        if port == port2 {
            p2_found = true;
        }
    }

    assert!(p1_found);
    assert!(p2_found);
}

#[test]
fn test_bad_promote()
{
    let mut manager = PortManager::new();
    let port1 = manager.register_output_port("test".to_string(), "out".to_string());

    let also_port1 = manager.find_port("test", "out");
    assert!(also_port1.is_some());

    let also_port1 = also_port1.unwrap().promote_to_input();

    assert!(also_port1.is_err());
}

#[test]
fn test_connect_by_name()
{
    let mut manager = PortManager::new();
    let port1 = manager
        .register_output_port("test".to_string(), "out".to_string())
        .unwrap();
    let port2 = manager
        .register_input_port("test".to_string(), "in".to_string())
        .unwrap();

    let connected = manager.connect_by_name(("test", "out"), ("test", "in"));
    assert!(connected.is_ok());

    let expected = 10.0;
    manager.set_port_value(&port1, expected);

    assert!(manager.get_port_value(&port1) == expected);
    assert!(manager.get_port_value(&port2) == expected);
}

#[test]
fn test_connect_by_name_fail1()
{
    let mut manager = PortManager::new();
    let port1 = manager
        .register_output_port("test".to_string(), "out".to_string())
        .unwrap();
    let port2 = manager
        .register_input_port("test".to_string(), "in".to_string())
        .unwrap();

    let connected = manager.connect_by_name(("test", "out"), ("test", "dne"));
    assert!(connected.unwrap_err() == PortManagerError::NoSuchPort);
}

#[test]
fn test_connect_by_name_fail2()
{
    let mut manager = PortManager::new();
    let port2 = manager
        .register_input_port("test".to_string(), "in1".to_string())
        .unwrap();
    let port2 = manager
        .register_input_port("test".to_string(), "in2".to_string())
        .unwrap();

    let connected = manager.connect_by_name(("test", "in1"), ("test", "in1"));
    assert!(connected.unwrap_err() == PortManagerError::NotOutputPort);
}

#[test]
fn test_connect_by_name_fail3()
{
    let mut manager = PortManager::new();
    let port2 = manager
        .register_output_port("test".to_string(), "p1".to_string())
        .unwrap();
    let port2 = manager
        .register_output_port("test".to_string(), "p2".to_string())
        .unwrap();

    let connected = manager.connect_by_name(("test", "p1"), ("test", "p2"));
    assert!(connected.unwrap_err() == PortManagerError::NotInputPort);
}
