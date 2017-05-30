use ketos;
use ketos::ForeignValue;
use ketos::FromValueRef;
use ketos::ModuleLoader;
use std::cell::RefCell;

use std::path::PathBuf;
use std::rc::Rc;

use voice::Voice;

#[derive(Debug)]
struct CachePair {
    first: (String, String),
    second: (String, String),
}

// looks like a voice but isn't a voice, can be used in ketos since we can't
// use the real voice in
// ketos (only static lifetime types can be used as ketos vals)
#[derive(Debug, ForeignValue, FromValueRef)]
struct VoiceCache {
    pub connections: RefCell<Vec<CachePair>>,
}

// all the methods need to be available at global scope so might as well not
// put them on the struct
fn cache_connection(
    cache: &VoiceCache,
    first: (&str, &str),
    second: (&str, &str),
) -> Result<(), ketos::Error>
{
    let p = CachePair {
        first: (first.0.to_owned(), first.1.to_owned()),
        second: (second.0.to_owned(), second.1.to_owned()),
    };

    cache.connections.borrow_mut().push(p);
    Ok(())
}

pub struct Patch {}

// public impl
impl Patch {
    pub fn from_file<'a>() -> Voice<'a>
    {
        let vc = Rc::new(VoiceCache { connections: RefCell::new(Vec::new()) });

        let loader = ketos::FileModuleLoader::with_search_paths(vec![
            PathBuf::from("/home/dpzmick/.cargo/registry/src/github.com-1ecc6299db9ec823/ketos-0.9.0/lib"),
        ]);

        // TODO read a real file
        let interp = ketos::Interpreter::with_loader(Box::new(ketos::BuiltinModuleLoader
                                                                  .chain(loader)));

        ketos_fn!{
            interp.scope()
            => "cache-connection"
            => fn cache_connection(cache: &VoiceCache, first: (&str, &str), second: (&str, &str))
                                   -> ()
        }

        interp
            .run_code(r#"
            (use list :all)

            (define connections
                '(
                    ( ("voice" "midi_frequency_in") ("base_osc" "frequency_in"))
                ))

            (define (connect voice)
                (map
                    (lambda (pair)
                        (cache-connection voice (first pair) (first (tail pair))))
                    connections))
        "#,
                      None)
            .unwrap();

        let result = interp
            .call("connect", vec![ketos::Value::Foreign(vc.clone())])
            .unwrap();

        let mut voice = Voice::new();

        use components::{CombineInputs, Math, OnOff, SineWaveOscillator, SquareWaveOscillator};

        // creates two harmonics
        // midi input goes through here to get second harmonic
        voice.add_component(Math::new("math".to_string(), |x| x * 2.0));
        voice.add_component(SquareWaveOscillator::new("harmonic_osc".to_string()));

        // midi input also sent through here
        voice.add_component(SineWaveOscillator::new("base_osc".to_string()));

        // create an input combiner with 2 inputs
        voice.add_component(CombineInputs::new("combine".to_string(), 2));

        // finally, gate is sent through the OnOff
        voice.add_component(OnOff::new("envelope".to_string()));

        // connect everything using the contents of the script
        {
            let ports = voice.get_port_manager_mut();

            for connection in vc.connections.borrow().iter() {
                println!("connection: {:?}", connection);

                if let Err(err) = ports.connect_by_name((&connection.first.0,
                                                         &connection.first.1),
                                                        (&connection.second.0,
                                                         &connection.second.1)) {
                    println!("error occurred: {:?}", err);
                }
            }
        }

        voice
    }
}
