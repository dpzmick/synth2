// clippy
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#![feature(conservative_impl_trait)]

extern crate serde;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate enum_primitive;
extern crate easyjack as jack;

#[macro_use]
extern crate ketos;

#[macro_use]
extern crate ketos_derive;

mod components;
mod jack_engine;
mod midi;
mod patch;
mod ports;
mod soundscape;
mod voice;

use jack_engine::run_audio_thread;
use patch::Patch;
use soundscape::Soundscape;

use std::path::Path;

static SRATE: f32 = 44100.0;

fn main()
{
    // TODO return result from patch
    let voice = Patch::from_file(Path::new("patches/sine.patch"));
    let mut soundscape = Soundscape::new(16, voice);
    run_audio_thread(soundscape);

    loop {
        use std::thread;
        use std::time::Duration;
        thread::sleep(Duration::from_millis(100000));
    }
}
