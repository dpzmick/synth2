// try to keep all of the code needed to manage the entire ketos runtime
// contained to this file, if possible

use components::Component;
use components::ComponentConfig;

use ketos;
use ketos::ForeignValue;
use ketos::FromValue;
use ketos::ModuleLoader;
use ports::PortName;

use serde;
use std::cell::RefCell;
use std::marker::PhantomData;

use std::path::{Path, PathBuf};
use std::rc::Rc;

use voice::Voice;

type Decoder<T> = Box<Fn(&T) -> Result<Box<ComponentConfig>, String>>;

#[derive(Debug)]
struct KetosConfigInput<'a> {
    value: &'a ketos::Value,
}

impl<'a> KetosConfigInput<'a> {
    /// Create a decoder for a specific type
    fn make_decoder<T: ComponentConfig + ketos::FromValue + 'static>(&self)
        -> Decoder<Self>
    {
        Box::new(|this: &KetosConfigInput| {
            println!("attemting to decode for {:?}", this);
            T::from_value(this.value.clone())
                .map(|v| Box::new(v) as Box<ComponentConfig>)
                .map_err(|err| {
                    println!("{:?}", err);
                    "Component not a valid type".to_owned()
                })
        })
    }

    fn get_all_decoders(&self) -> Vec<Decoder<Self>>
    {
        use components::{SineWaveOscillatorConfig, SquareWaveOscillatorConfig};
        use components::{OnOffConfig};

        let mut decoders = Vec::new();
        decoders.push(self.make_decoder::<SineWaveOscillatorConfig>());
        decoders.push(self.make_decoder::<SquareWaveOscillatorConfig>());
        decoders.push(self.make_decoder::<OnOffConfig>());
        decoders
    }

    pub fn register_all_decoders(scope: &ketos::Scope)
    {
        use components::{SineWaveOscillatorConfig, SquareWaveOscillatorConfig};
        use components::{OnOffConfig};

        scope.register_struct_value::<SineWaveOscillatorConfig>();
        scope.register_struct_value::<SquareWaveOscillatorConfig>();
        scope.register_struct_value::<OnOffConfig>();
    }

    /// Attempts to build a component config from some underlying config format
    /// Will iterate through every available decoder looking for the first one
    /// that works
    pub fn parse(&self) -> Result<Box<ComponentConfig>, String>
    {
        for decoder in self.get_all_decoders().into_iter() {
            if let Ok(config) = decoder(self) {
                return Ok(config);
            }
        }

        Err(format!("Could not decode the rust struct for {:?}", self.value))
    }
}

/// Exists only to allow us to read values from ketos
/// Values are copied from here into the actual patch after we are done with the
/// ketos magic
#[derive(Debug, ForeignValue, FromValueRef)]
struct Config {
    pub connections: RefCell<Vec<Connection>>,
    pub components: RefCell<Vec<Box<ComponentConfig>>>,
}

// all the methods need to be available at global scope so might as well not put
// them on the struct
fn connect(config: &Config, first: (&str, &str), second: (&str, &str))
    -> Result<(), ketos::Error>
{
    let p = Connection {
        first: PortName::new(first.0, first.1),
        second: PortName::new(second.0, second.1),
    };

    config.connections.borrow_mut().push(p);
    Ok(())
}

fn add_component(config: &Config, comp: Box<ComponentConfig>)
    -> Result<(), ketos::Error>
{
    config.components.borrow_mut().push(comp);
    Ok(())
}

#[derive(Debug, Clone)]
struct Connection {
    first: PortName,
    second: PortName,
}

/// A can be used to create an instance of a Voice with a certain configuration.
pub struct Patch {
    connections: Vec<Connection>,
    components: Vec<Box<ComponentConfig>>,
}

// public impl
impl Patch {
    pub fn from_file(path: &Path) -> Result<Self, String>
    {
        let config = Rc::new(Config {
            connections: RefCell::new(Vec::new()),
            components: RefCell::new(Vec::new()),
        });

        let loader = ketos::FileModuleLoader::with_search_paths(vec![
                PathBuf::from("/home/dpzmick/.cargo/registry/src/github.com-1ecc6299db9ec823/ketos-0.9.0/lib"),
        ]);

        let interp = ketos::Interpreter::with_loader(
            Box::new(ketos::BuiltinModuleLoader.chain(loader)));

        ketos_fn!{
            interp.scope()
            => "connect"
            => fn connect(
                config: &Config,
                first: (&str, &str),
                second: (&str, &str))
            -> ()
        }

        interp.scope().add_value_with_name("add-component", |name| {
            ketos::value::Value::new_foreign_fn(name, move |scope, args| {
                let expected = 2;
                if args.len() != expected {
                    return Err(From::from(ketos::exec::ExecError::ArityError {
                        name: Some(name),
                        expected: ketos::function::Arity::Exact(expected as u32),
                        found: args.len() as u32
                    }));
                }

                let mut iter = (&*args).iter();

                let value = iter.next().unwrap();
                let config = try!(
                    <&Config as ketos::value::FromValueRef>::from_value_ref(value));

                let value = iter.next().unwrap();
                let kval = KetosConfigInput { value };
                let compconfig = kval.parse().unwrap(); // TODO

                let res = try!(add_component(config, compconfig));
                Ok(<() as Into<ketos::value::Value>>::into(res))
            })
        });

        KetosConfigInput::register_all_decoders(interp.scope());

        interp.run_file(path)
            .and_then(|_| {
                interp.call("create", vec![ketos::Value::Foreign(config.clone())])
            })
            .map(|_value| {
                // ignore the return, reuse the original config, then create the
                // actual patch from the config
                let mut p = Patch {
                    connections: Vec::new(),
                    components: Vec::new(),
                };

                p.connections.clone_from(&*config.connections.borrow());
                p.components.clone_from(&*config.components.borrow());

                p
            })
            .map_err(|error| {
                use ketos::name::display_names;
                use std::ops::Deref;

                format!("{}", display_names(
                        interp.scope().borrow_names().deref(),
                        &error))
            })
    }

    pub fn create_voice<'a>(&self) -> Voice<'a> {
        // connect everything using the contents of the script
        let mut voice = Voice::new();

        for config in self.components.iter() {
            voice.add_component(config.build_component());
        }

        {
            let ports = voice.get_port_manager_mut();
            for connection in self.connections.iter() {
                let res = ports.connect_by_name(
                    &connection.first, &connection.second);
                if let Err(err) = res {
                    println!("error occurred: {:?}", err);
                }
            }
        }

        println!("{:#?}", voice);
        voice
    }
}
