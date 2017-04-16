#[macro_use] extern crate conrod;
#[macro_use] extern crate enum_primitive;
extern crate easyjack as jack;

mod components;
mod jack_engine;
mod midi;
mod ports;
mod soundscape;
mod ui;
mod voice;

use jack_engine::run_audio_thread;
use soundscape::Soundscape;
use ui::SynthUi;

static SRATE: f32 = 44100.0;

fn main()
{
    // // start the realtime soundscape
    // let mut soundscape = Soundscape::new();
    // soundscape.example_connections();
    // run_audio_thread(soundscape);
    // loop {
    //     use std::thread;
    //     use std::time::Duration;
    //     thread::sleep(Duration::from_millis(100000));
    // }

    let mut last_update = std::time::Instant::now();
    let mut gui = SynthUi::new();

    'main: loop {
        let sixteen_ms = std::time::Duration::from_millis(16);
        let duration_since_last_update = std::time::Instant::now().duration_since(last_update);
        if duration_since_last_update < sixteen_ms {
            std::thread::sleep(sixteen_ms - duration_since_last_update);
        }

        for event in gui.event_loop() {
            match event {
                ui::UiEvent::Exit => break 'main,
                _ => (),
            }
        }
    }
}
