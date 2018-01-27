extern crate synth;
extern crate signal;

use synth::jack_engine::run_audio_threads;
use synth::patch::Patch;
use synth::soundscape::Soundscape;

use signal::trap::Trap;

use std::env;
use std::path::Path;
use std::time::Instant;
use std::time::Duration;
use std::thread;

fn usage()
{
    println!("usage: synth2 patch_file");
}

fn main()
{
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
        return;
    }

    let patch = Patch::from_file(Path::new(&args[1])).unwrap();
    let soundscape = Soundscape::new(1, patch);
    let client = run_audio_threads(soundscape); // important to hold a reference to the client

    let t = Trap::trap(&[signal::Signal::SIGINT, signal::Signal::SIGTERM]);
    loop {
        let stime = Duration::from_millis(500);
        if (t.wait(Instant::now() + stime).is_some()) {
            println!("cleaning up");
            client.shutdown();
            return;
        }
    }
}
