// try to keep all of the code needed to manage the entire ketos runtime contained to this file, if
// possible
//
// TODO this file is a disaster

use components::Component;
use components::ComponentConfig;
use components::ComponentConfigMaker;
use components::SineWaveOscillatorConfig;
use components::SquareWaveOscillatorConfig;
use ketos;
use ketos::ForeignValue;
use ketos::FromValue;
use ketos::FromValueRef;
use ketos::ModuleLoader;

use serde;
use std::cell::RefCell;
use std::marker::PhantomData;

use std::path::PathBuf;
use std::rc::Rc;

use voice::Voice;

// TODO don't use this this sucks
#[derive(Debug)]
struct CachePair {
    first: (String, String),
    second: (String, String),
}

struct KetosConfigInput<'a> {
    value: &'a ketos::Value,
}

impl<'a> ComponentConfigMaker for KetosConfigInput<'a> {
    fn make_decoder<T: ComponentConfig + ketos::FromValue + 'static>(&self)
        -> Box<Fn(&Self) -> Result<Box<ComponentConfig>, String>>
    {
        Box::new(|this: &KetosConfigInput| {
            T::from_value(this.value.clone())
                .map(|v| Box::new(v) as Box<ComponentConfig>)
                .map_err(|_| "Component not a valid type".to_owned())
        })
    }
}

#[derive(Debug, ForeignValue, FromValueRef)]
struct Config {
    pub connections: RefCell<Vec<CachePair>>,
    pub components: RefCell<Vec<Box<ComponentConfig>>>,
}

// all the methods need to be available at global scope so might as well not put them on the struct
fn connect(
    config: &Config,
    first: (&str, &str),
    second: (&str, &str),
) -> Result<(), ketos::Error>
{
    let p = CachePair {
        first: (first.0.to_owned(), first.1.to_owned()),
        second: (second.0.to_owned(), second.1.to_owned()),
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
    pub fn from_file<'a>() -> Voice<'a>
    {
        let cache = Rc::new(Config {
                                 connections: RefCell::new(Vec::new()),
                                 components: RefCell::new(Vec::new()),
                             });

        let loader = ketos::FileModuleLoader::with_search_paths(vec![
            PathBuf::from("/home/dpzmick/.cargo/registry/src/github.com-1ecc6299db9ec823/ketos-0.9.0/lib"),
        ]);

        // TODO read a real file
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

        // fucking hell
        interp.scope().register_struct_value::<SquareWaveOscillatorConfig>();

        interp
            .run_code(r#"
                (define (create config)
                    ; add components
                    (add-component config
                       (new SquareWaveOscillatorConfig :name "square"))

                    ; connect components
                    (connect config '("square" "samples_out") '("voice" "samples_in")))
                "#, None)
            .unwrap();

        let result = interp.call("create", vec![ketos::Value::Foreign(cache.clone())]).unwrap();

        // connect everything using the contents of the script
        let mut voice = Voice::new();

        for config in cache.components.borrow().iter() {
            voice.add_component(config.build_component());
        }

        {
            let ports = voice.get_port_manager_mut();

            for connection in cache.connections.borrow().iter() {
                println!("connection: {:?}", connection);

                if let Err(err) = ports.connect_by_name((&connection.first.0,
                                                         &connection.first.1),
                                                        (&connection.second.0,
                                                         &connection.second.1)) {
                    println!("error occurred: {:?}", err);
                }
            }
        }

        println!("{:#?}", voice);
        voice
    }
}
