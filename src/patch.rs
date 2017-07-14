// try to keep all of the code needed to manage the entire ketos runtime contained to this file, if
// possible

use components::Component;
use components::ComponentConfig;
use ports::PortName;

use ketos;
use ketos::ForeignValue;
use ketos::FromValue;
use ketos::FromValueRef;
use ketos::ModuleLoader;

use serde;
use std::cell::RefCell;
use std::marker::PhantomData;

use std::path::{Path, PathBuf};
use std::rc::Rc;

use voice::Voice;

type Decoder<T> = Box<Fn(&T) -> Result<Box<ComponentConfig>, String>>;

struct KetosConfigInput<'a> {
    value: &'a ketos::Value,
}

impl<'a> KetosConfigInput<'a> {
    /// Create a decoder for a specific type
    fn make_decoder<T: ComponentConfig + ketos::FromValue + 'static>(&self) -> Decoder<Self>
    {
        Box::new(|this: &KetosConfigInput| {
            T::from_value(this.value.clone())
                .map(|v| Box::new(v) as Box<ComponentConfig>)
                .map_err(|_| "Component not a valid type".to_owned())
        })
    }

    fn get_all_decoders(&self) -> Vec<Decoder<Self>>
    {
        use components::{SineWaveOscillatorConfig, SquareWaveOscillatorConfig};
        let mut decoders = Vec::new();
        decoders.push(self.make_decoder::<SquareWaveOscillatorConfig>());
        decoders
    }

    pub fn register_all_decoders(scope: &ketos::Scope)
    {
        use components::{SineWaveOscillatorConfig, SquareWaveOscillatorConfig};
        scope.register_struct_value::<SquareWaveOscillatorConfig>();
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

        Err("NONE FOUND".to_owned())
    }
}

#[derive(Debug)]
struct Connection {
    first: PortName,
    second: PortName,
}

#[derive(Debug, ForeignValue, FromValueRef)]
struct Config {
    pub connections: RefCell<Vec<Connection>>,
    pub components: RefCell<Vec<Box<ComponentConfig>>>,
}

// all the methods need to be available at global scope so might as well not put them on the struct
fn connect(config: &Config, first: (&str, &str), second: (&str, &str)) -> Result<(), ketos::Error>
{
    let p = Connection {
        first: PortName::new(first.0, first.1),
        second: PortName::new(second.0, second.1),
    };

    config.connections.borrow_mut().push(p);
    Ok(())
}

fn add_component(config: &Config, comp: Box<ComponentConfig>) -> Result<(), ketos::Error>
{
    config.components.borrow_mut().push(comp);
    Ok(())
}

pub struct Patch {}

// public impl
impl Patch {
    pub fn from_file<'a>(path: &Path) -> Voice<'a>
    {
        let cache = Rc::new(Config {
                                connections: RefCell::new(Vec::new()),
                                components: RefCell::new(Vec::new()),
                            });

        let loader = ketos::FileModuleLoader::with_search_paths(vec![
            PathBuf::from("/home/dpzmick/.cargo/registry/src/github.com-1ecc6299db9ec823/ketos-0.9.0/lib"),
        ]);

        let interp = ketos::Interpreter::with_loader(Box::new(ketos::BuiltinModuleLoader
                                                                  .chain(loader)));
        ketos_fn!{
            interp.scope()
            => "connect"
            => fn connect(cache: &Config, first: (&str, &str), second: (&str, &str)) -> ()
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
                let cache = try!(<&Config as ketos::value::FromValueRef>::from_value_ref(value));

                let value = iter.next().unwrap();
                let kval = KetosConfigInput { value };
                let config = kval.parse().unwrap(); // TODO

                let res = try!(add_component(cache, config));
                Ok(<() as Into<ketos::value::Value>>::into(res))
            })
        });

        KetosConfigInput::register_all_decoders(interp.scope());

        match interp.run_file(path) {
            Ok(()) => (),
            Err(error) => {
                println!("error occured: {:?}", error);
                panic!("gtfo");
            },
        }

        let result = interp.call("create", vec![ketos::Value::Foreign(cache.clone())]);

        let result = match result {
            Ok(result) => result,
            Err(error) => {
                println!("error occured: {:?}", error);
                panic!("gtfo");
            },
        };

        // connect everything using the contents of the script
        let mut voice = Voice::new();

        for config in cache.components.borrow().iter() {
            voice.add_component(config.build_component());
        }

        {
            let ports = voice.get_port_manager_mut();
            for connection in cache.connections.borrow().iter() {
                if let Err(err) = ports.connect_by_name(&connection.first, &connection.second) {
                    println!("error occurred: {:?}", err);
                }
            }
        }

        println!("{:#?}", voice);
        voice
    }
}
