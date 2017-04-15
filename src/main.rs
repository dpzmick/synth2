extern crate easyjack as jack;
#[macro_use]
extern crate enum_primitive;

mod components;
mod midi;
mod ports;
mod soundscape;
mod voice;
mod jack_engine;

use soundscape::Soundscape;
use jack_engine::run_audio_thread;

static SRATE: f32 = 44100.0;

fn main()
{
    // start the realtime soundscape
    let mut soundscape = Soundscape::new();
    soundscape.example_connections();
    run_audio_thread(soundscape);
    loop {
        use std::thread;
        use std::time::Duration;
        thread::sleep(Duration::from_millis(100000));
    }
}
