#![feature(conservative_impl_trait)]

extern crate serde;

#[macro_use]
extern crate enum_primitive;
extern crate easyjack as jack;

#[macro_use]
extern crate ketos;

#[macro_use]
extern crate ketos_derive;

mod audioprops;
mod components;
mod jack_engine;
mod midi;
mod patch;
mod ports;
mod soundscape;
mod topo;
mod util;
mod voice;

use jack_engine::run_audio_thread;
use patch::Patch;
use soundscape::Soundscape;

use std::path::Path;

fn main()
{
    let patch = Patch::from_file(Path::new("patches/sine.patch")).unwrap();
    let soundscape = Soundscape::new(1, patch);
    run_audio_thread(soundscape);

    loop {
        use std::thread;
        use std::time::Duration;
        thread::sleep(Duration::from_millis(100000));
    }
}
