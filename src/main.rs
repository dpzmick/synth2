extern crate synth;

use synth::jack_engine::run_audio_thread;
use synth::patch::Patch;
use synth::soundscape::Soundscape;

use std::path::Path;

fn main()
{
    let patch = Patch::from_file(Path::new("patches/square.patch")).unwrap();
    let soundscape = Soundscape::new(16, patch);
    let client = run_audio_thread(soundscape); // important to hold a reference to the client

    loop {
        use std::thread;
        use std::time::Duration;
        thread::sleep(Duration::from_millis(100000));
    }
}
