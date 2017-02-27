extern crate easyjack as jack;
extern crate rimd;

use std::thread;
use std::time::Duration;
use std::f32;

#[derive(Copy, Clone)]
struct SineNote {
    frequency: f32,
    velocity: f32, /* 0 -> 1 */
    phase: usize,
}

impl SineNote {
    fn new(frequency: f32, velocity: f32) -> Self {
        SineNote {
            frequency,
            velocity,
            phase: 0,
        }
    }
}

struct SineWaveGenerator {
    srate_constant: f32,
}

impl SineWaveGenerator {
    fn new(srate: f32) -> Self {
        SineWaveGenerator { srate_constant: srate }
    }

    fn generate(&mut self, note: &mut SineNote) -> f32 {
        let c = self.srate_constant / note.frequency;

        if note.phase >= c as usize {
            note.phase = 1;
        } else {
            note.phase += 1;
        }

        let x = note.phase as f32;
        note.velocity * (2.0 * std::f32::consts::PI * (note.frequency/self.srate_constant) * x).sin()
    }
}

// wave generators know the state of the system, but they don't know what notes are being played

type OPort = jack::OutputPortHandle<jack::DefaultAudioSample>;
type IPort = jack::InputPortHandle<jack::MidiEvent>;

struct AudioHandler {
    input: IPort,
    output: OPort,
    generator: SineWaveGenerator,
    notes: [Option<SineNote>; 64], // polyphony of 64
}

impl AudioHandler {
    pub fn new(input: IPort, output: OPort, generator: SineWaveGenerator) -> Self {
        AudioHandler {
            input,
            output,
            generator,
            notes: [None; 64]
        }
    }

    fn midi_note_to_frequency(note: u8) -> f32 {
        let a = 440.0;
        (a / 32.0) * (2.0_f32.powf( (note as f32 - 9.0) / 12.0 ))
    }

    fn midi_velocity_to_velocity(vel: u8) -> f32 {
        vel as f32 / (std::u8::MAX as f32)
    }
}

impl jack::ProcessHandler for AudioHandler {
    fn process(&mut self, ctx: &jack::CallbackContext, nframes: jack::NumFrames) -> i32 {
        let output_buffer = self.output.get_write_buffer(nframes, &ctx);
        let input_buffer  = self.input.get_read_buffer(nframes, &ctx);

        let mut event_index = 0;
        let event_count = input_buffer.len();
        if event_count > 0 {
            println!("event_count = {}", event_count);
        }

        for i in 0..event_count {
            let event = input_buffer.get(i);
            let buf = event.raw_midi_bytes();

            let m = rimd::MidiMessage { data: buf.to_vec() };
            println!("at time = {}, message {:?} = {:?}", event.get_jack_time(), m.status(), m);

        }

        for i in 0..(nframes as usize) {
            if event_index < event_count {
                loop {
                    if event_index >= event_count { break; }

                    let event = input_buffer.get(event_index);
                    if event.get_jack_time() > i as jack::NumFrames { break; }

                    let buf = event.raw_midi_bytes();

                    let m = rimd::MidiMessage { data: buf.to_vec() };
                    println!("message {:?} = {:?}", m.status(), m);

                    match m.status() {
                        rimd::Status::NoteOn => {
                            // find the first open spot and use it
                            for i in 0..64 {
                                if self.notes[i].is_none() {
                                    println!("picked i == {}", i);
                                    self.notes[i] = Some(SineNote::new(
                                            AudioHandler::midi_note_to_frequency(m.data[1]),
                                            AudioHandler::midi_velocity_to_velocity(m.data[2])));
                                    break;
                                }
                            }
                        },

                        rimd::Status::NoteOff => {
                            for i in 0..64 {
                                if self.notes[i].is_some() {
                                    if self.notes[i].unwrap().frequency == AudioHandler::midi_note_to_frequency(m.data[1]) {
                                        println!("cleared i == {}", i);
                                        self.notes[i] = None
                                    }
                                }
                            }
                            self.notes[0] = None;
                        },

                        _ => ()
                    }

                    event_index += 1;
                }
            }

            let mut frame = 0.0;

            for note in self.notes.iter_mut() {
                match note.as_mut() {
                    Some(note) => frame += self.generator.generate(note),
                    None       => ()
                }
            }

            output_buffer[i] = frame;
        }

        0
    }
}

fn main() {
    let gen = SineWaveGenerator::new(44100.0);

    let mut c = jack::Client::open("sine", jack::options::NO_START_SERVER).unwrap().0;
    let i = c.register_input_midi_port("midi_in").unwrap();
    let o = c.register_output_audio_port("audio_out").unwrap();

    let handler = AudioHandler::new(i, o, gen);

    c.set_process_handler(handler).unwrap();

    c.activate().unwrap();
    c.connect_ports("sine:audio_out", "system:playback_1").unwrap();
    //c.connect_ports("jack-keyboard:midi_out", "sine:midi_in").unwrap();

    loop {
        thread::sleep(Duration::from_millis(100000));
    }

    c.close().unwrap();
}
