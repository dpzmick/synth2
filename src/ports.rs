use util;

use std::collections::HashMap;
use std::marker::PhantomData;

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

impl<'a, 'b> PartialEq<&'a PortHandle> for &'b PortHandle {
    fn eq(&self, other: &&PortHandle) -> bool
    {
        self.id() == other.id() && self.direction() == other.direction()
    }
}

/// A PortHandle whose direction is not yet known.
/// Can be promoted to input or output ports types
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
    pub fn promote_to_output(self)
        -> Result<OutputPortHandle<'a>, PortManagerError>
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

    pub fn promote_to_input(self)
        -> Result<InputPortHandle<'a>, PortManagerError>
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
        other as &PortHandle == self as &PortHandle
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
        other as &PortHandle == self as &PortHandle
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
        other as &PortHandle == self as &PortHandle
    }
}

/// Each port is associated with a component, and must be given a name
/// For a single component, each port name must be unique
#[derive(Debug, PartialEq, Clone)]
pub struct PortName {
    component: String,
    port: String,
}

impl PortName {
    pub fn new<T1: ToString, T2: ToString>(component: T1, port: T2) -> Self
    {
        Self {
            component: component.to_string(),
            port: port.to_string(),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum PortManagerError {
    PortsNotUnique,
    NotOutputPort,
    NotInputPort,
    NoSuchPort(PortName),
}

/// A RealtimePortManager can only access the portions of a PortManager that are
/// safe to use in a realtime context
pub trait RealtimePortManager<'a> {
    /// Get the current value of the port. If the port handle was registered
    /// with this PortManager, this will never fail because ports cannot be
    /// destroyed. Calling this function with a handle to a port from a
    /// different PortManager is undefined behavior.
    fn get_port_value(&self, p: &PortHandle) -> f32;

    /// Set the current value of the port. Calling this function with a handle
    /// to a port from a different PortManager is undefined behavior. This may
    /// only be called on an Output port
    fn set_port_value(&mut self, p: &OutputPortHandle, val: f32);
}

/// A port manager manages the connections between different components. Every
/// component can register a variety of input and output ports with the port
/// manager. When two ports are connected, any values written to the "Input" end
/// of the port will also be written to the "Output" end.
pub trait PortManager<'a>: RealtimePortManager<'a> {
    fn register_input_port(&mut self, name: &PortName)
        -> Result<InputPortHandle<'a>, PortManagerError>;

    fn register_output_port(&mut self, name: &PortName)
        -> Result<OutputPortHandle<'a>, PortManagerError>;

    /// Connect ports
    /// The value on the output port will always be available on the input port.
    /// Note that this will always succeed, as long as both of the port handles
    /// are owned by this PortManager.
    /// It is impossible to request an invalid connection due to type safety.
    fn connect(&mut self, p1: &OutputPortHandle, p2: &InputPortHandle);

    /// Disconnect ports
    /// If two ports are already connected, this will remove the connection
    /// between them. If they are not connected, this will be a potentially
    /// expensive noop.
    /// It is impossible to request an invalid disconnection due to type safety
    fn disconnect(&mut self, p1: &OutputPortHandle, p2: &InputPortHandle);

    fn connect_by_name(&mut self, p1: &PortName, p2: &PortName)
        -> Result<(), PortManagerError>;

    fn find_port(&self, name: &PortName) -> Option<UnknownPortHandle<'a>>;

    /// Returns a copy of the component adjacency matrix for this port manager
    /// and map from Component Name -> index in the matrix that was used when
    /// creating the matrix
    fn get_component_adjacency_matrix(&self)
        -> (HashMap<usize, String>, util::nmat::Matrix<bool, util::nmat::RowMajor>);
}

#[derive(Debug)]
pub struct PortManagerImpl<'a> {
    // graph implementation
    ports: Vec<f32>,
    // not using an adjacency matrix here to keep the cost of resizing lower
    // and because the matrix will likely be sparse
    connections: Vec<(usize, usize)>,

    // metadata information
    // component_name -> (port_name -> handle)
    ports_meta: HashMap<String, HashMap<String, UnknownPortHandle<'a>>>,

    // used to enforce the lifetime constraints of the port handles
    phantom: PhantomData<&'a usize>,
}

// private impl
impl<'a> PortManagerImpl<'a> {
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

    fn save_port_meta(
        &mut self,
        component: &str,
        port_name: &str,
        id: PortId,
        direction: PortDirection,
    )
    {
        let e = self.ports_meta
            .entry(component.to_owned())
            .or_insert_with(|| HashMap::new());

        e.entry(port_name.to_owned()).or_insert(
            UnknownPortHandle {
                id: id,
                dir: direction,
                phantom: PhantomData,
            });
    }

    fn new_port(
        &mut self,
        component: &str,
        port_name: &str,
        direction: PortDirection,
    ) -> Result<usize, PortManagerError>
    {
        if !self.check_key_usable(component, port_name) {
            return Err(PortManagerError::PortsNotUnique);
        }

        self.ports.push(0.0);
        let id = self.ports.len() - 1;
        self.save_port_meta(component, port_name, id, direction);
        Ok(id)
    }
}

impl<'a> PortManagerImpl<'a> {
    pub fn new() -> Self
    {
        Self {
            ports:       Vec::new(),
            connections: Vec::new(),
            ports_meta:  HashMap::new(),
            phantom:     PhantomData,
        }
    }
}

impl<'a> RealtimePortManager<'a> for PortManagerImpl<'a> {
    fn get_port_value(&self, p: &PortHandle) -> f32
    {
        self.ports[p.id()]
    }

    fn set_port_value(&mut self, p: &OutputPortHandle, val: f32)
    {
        // assert cannot allocate or resize
        self.ports[p.id] = val;

        for &(p1, p2) in &self.connections {
            if p1 == p.id {
                self.ports[p2] = val;
            }
        }
    }
}

impl<'a> PortManager<'a> for PortManagerImpl<'a> {
    fn register_input_port(&mut self, name: &PortName)
        -> Result<InputPortHandle<'a>, PortManagerError>
    {
        self.new_port(&name.component, &name.port, PortDirection::Input)
            .map(|id| {
                InputPortHandle {
                    id,
                    phantom: PhantomData,
                }
            })
    }

    fn register_output_port(&mut self, name: &PortName)
        -> Result<OutputPortHandle<'a>, PortManagerError>
    {
        self.new_port(&name.component, &name.port, PortDirection::Output)
            .map(|id| {
                OutputPortHandle {
                    id,
                    phantom: PhantomData,
                }
            })
    }

    fn connect(&mut self, p1: &OutputPortHandle, p2: &InputPortHandle)
    {
        self.connections.push((p1.id, p2.id));
    }

    fn disconnect(&mut self, p1: &OutputPortHandle, p2: &InputPortHandle)
    {
        self.connections
            .retain(|&(a, b)| a != p1.id && b != p2.id);
    }

    fn connect_by_name(&mut self, p1: &PortName, p2: &PortName)
        -> Result<(), PortManagerError>
    {
        // lookup both of the ports that are requested
        self.find_port(p1)
            .ok_or(PortManagerError::NoSuchPort(p1.clone()))
            .and_then(|port| port.promote_to_output())
            .and_then(|output| {
                self.find_port(p2)
                    .ok_or(PortManagerError::NoSuchPort(p2.clone()))
                    .and_then(|port| port.promote_to_input())
                    .map(|input| (output, input))
            })
            .and_then(|(output, input)| {
                // if we get this far, we have both ports, so connect them
                self.connect(&output, &input);
                Ok(())
            })
    }

    fn find_port(&self, name: &PortName) -> Option<UnknownPortHandle<'a>>
    {
        self.ports_meta
            .get(&name.component)
            .and_then(|comp| comp.get(&name.port))
            .cloned()
    }

    fn get_component_adjacency_matrix(&self)
        -> (HashMap<usize, String>, util::nmat::Matrix<bool, util::nmat::RowMajor>)
    {

        // map component name to matrix index
        let mut name_map: HashMap<String, usize> = HashMap::new();
        let mut port_map: HashMap<usize, String> = HashMap::new();

        for (comp_name, ports) in &self.ports_meta {
            let next = name_map.len();
            name_map.entry(comp_name.to_owned()).or_insert(next);

            for (_, handle) in ports.iter() {
                port_map.insert(handle.id(), comp_name.to_owned());
            }
        }

        // we now have a single id for each component we know which component
        // each port maps to. Now we can create one node for each component and
        // set up the connections
        let n_components = self.ports_meta.len();
        let mut adj = util::nmat::Matrix::new((n_components, n_components));

        for &(first, second) in self.connections.iter() {
            let first_component_name = port_map.get(&first).unwrap();
            let first_component_idx = name_map[first_component_name];

            let second_component_name = port_map.get(&second).unwrap();
            let second_component_idx  = name_map[second_component_name];

            adj[(first_component_idx, second_component_idx)] = true;
        }

        // reverse the name_map so users can lookup a name by idx instead of
        // looking up an idx by name
        let mut idx_map = HashMap::new();
        for (k, v) in name_map.iter() {
            idx_map.insert(*v, k.to_owned());
        }

        (idx_map, adj)
    }
}

#[test]
fn test_set_get()
{
    let mut manager = PortManagerImpl::new();
    let port = manager
        .register_output_port(&PortName::new("test", "out"))
        .unwrap();

    let expected = 10.0;
    manager.set_port_value(&port, expected);
    let res = manager.get_port_value(&port);

    assert!(expected == res);
}

#[test]
fn test_connect()
{
    let mut manager = PortManagerImpl::new();
    let port1 = manager
        .register_output_port(&PortName::new("test", "out"))
        .unwrap();
    let port2 = manager
        .register_input_port(&PortName::new("test", "in"))
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
    let mut manager = PortManagerImpl::new();
    let port1 = manager.register_output_port(&PortName::new("test", "name"));
    let port2 = manager.register_output_port(&PortName::new("test", "name"));

    assert!(port1.is_ok());
    assert!(port2.is_err());
}

#[test]
fn test_duplicate_port2()
{
    let mut manager = PortManagerImpl::new();
    let port1 = manager.register_input_port(&PortName::new("test", "name"));
    let port2 = manager.register_input_port(&PortName::new("test", "name"));

    assert!(port1.is_ok());
    assert!(port2.is_err());
}

#[test]
fn test_duplicate_port3()
{
    let mut manager = PortManagerImpl::new();
    let port1 = manager.register_input_port(&PortName::new("test", "name"));
    let port2 = manager.register_output_port(&PortName::new("test", "name"));

    assert!(port1.is_ok());
    assert!(port2.is_err());
}

#[test]
fn test_disconnect()
{
    let mut manager = PortManagerImpl::new();
    let port1 = manager
        .register_output_port(&PortName::new("test", "out"))
        .unwrap();
    let port2 = manager
        .register_input_port(&PortName::new("test", "in"))
        .unwrap();

    manager.connect(&port1, &port2);
    manager.disconnect(&port1, &port2);

    manager.set_port_value(&port1, 10.0);
    assert!(manager.get_port_value(&port2) != 10.0);
}

#[test]
fn test_find1()
{
    let p = PortName::new("test", "out");
    let mut manager = PortManagerImpl::new();
    let port1 = manager.register_output_port(&p).unwrap();

    let also_port1 = manager.find_port(&p);
    assert!(also_port1.is_some());

    let also_port1 = also_port1.unwrap().promote_to_output();
    assert!(also_port1.is_ok());

    let also_port1 = also_port1.unwrap();
    assert!(port1 == also_port1);
}

#[test]
fn test_find2()
{
    let p = PortName::new("test", "out");

    let mut manager = PortManagerImpl::new();
    let port1 = manager.register_input_port(&p).unwrap();

    let also_port1 = manager.find_port(&p);
    assert!(also_port1.is_some());

    let also_port1 = also_port1.unwrap().promote_to_input();
    assert!(also_port1.is_ok());

    let also_port1 = also_port1.unwrap();
    assert!(port1 == also_port1);
}

#[test]
fn test_bad_promote()
{
    let p = PortName::new("test", "out");
    let mut manager = PortManagerImpl::new();
    manager.register_output_port(&p).unwrap();

    let port1 = manager.find_port(&p);
    assert!(port1.is_some());

    let port1 = port1.unwrap().promote_to_input();
    assert!(port1.is_err());
}

#[test]
fn test_connect_by_name()
{
    let i = PortName::new("test", "out");
    let o = PortName::new("test", "in");

    let mut manager = PortManagerImpl::new();
    let port1 = manager.register_output_port(&i).unwrap();
    let port2 = manager.register_input_port(&o).unwrap();

    let connected = manager.connect_by_name(&i, &o);
    assert!(connected.is_ok());

    let expected = 10.0;
    manager.set_port_value(&port1, expected);

    assert!(manager.get_port_value(&port1) == expected);
    assert!(manager.get_port_value(&port2) == expected);
}

#[test]
fn test_connect_by_name_fail1()
{
    let out = PortName::new("test", "out");

    let mut manager = PortManagerImpl::new();
    manager.register_output_port(&out).unwrap();
    manager.register_input_port(&PortName::new("test", "in")).unwrap();

    let bad = PortName::new("test", "dne");
    let connected = manager.connect_by_name(&out, &bad);
    assert!(connected.unwrap_err() == PortManagerError::NoSuchPort(bad));
}

#[test]
fn test_connect_by_name_fail2()
{
    let n1 = PortName::new("test", "in1");
    let n2 = PortName::new("test", "in2");

    let mut manager = PortManagerImpl::new();
    manager.register_input_port(&n1).unwrap();
    manager.register_input_port(&n2).unwrap();

    let connected = manager.connect_by_name(&n1, &n2);
    assert!(connected.unwrap_err() == PortManagerError::NotOutputPort);
}

#[test]
fn test_connect_by_name_fail3()
{
    let n1 = PortName::new("test", "p1");
    let n2 = PortName::new("test", "p2".to_string());

    let mut manager = PortManagerImpl::new();
    manager.register_output_port(&n1).unwrap();
    manager.register_output_port(&n2).unwrap();

    let connected = manager.connect_by_name(&n1, &n2);
    assert!(connected.unwrap_err() == PortManagerError::NotInputPort);
}
