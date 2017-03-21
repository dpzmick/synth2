extern crate easyjack as jack;
extern crate rimd;

use std::thread;
use std::time::Duration;
use std::f32;
use std::slice::IterMut;

#[derive(Copy, Clone)]
struct SineNote {
    frequency: f32,
    velocity: f32, /* 0 -> 1 */
    phase: usize,
    note_on: bool // note will decay when it is no longer on, when note off, velocity will decay
}

impl SineNote {
    fn new(frequency: f32, velocity: f32) -> Self {
        SineNote {
            frequency,
            velocity,
            phase: 0,
            note_on: true,
        }
    }
}

struct SineWaveGenerator {
    srate_constant: f32,
    decay_constant: f32,
}

impl SineWaveGenerator {
    fn new(srate: f32) -> Self {
        SineWaveGenerator {
            srate_constant: srate,
            decay_constant: 0.9999,
        }
    }

    fn generate(&mut self, note: &mut SineNote) -> f32 {
        let c = self.srate_constant / note.frequency;

        if note.phase >= c as usize {
            note.phase = 1;
        } else {
            note.phase += 1;
        }

        if !note.note_on {
            note.velocity = note.velocity * self.decay_constant;
        }

        let x = note.phase as f32;
        note.velocity * (2.0 * std::f32::consts::PI * (note.frequency/self.srate_constant) * x).sin()
    }

    fn is_note_dead(&self, note: &SineNote) -> bool {
        note.velocity < 0.01
    }
}

struct EventManager {
    events: Vec<Option<SineNote>> // TODO better datatype
}

impl EventManager {
    pub fn new(events: usize) -> Self {
        let events = vec![None; events];
        EventManager {
            events
        }
    }

    /// Find the next available location for a note
    fn next_free_idx(&self) -> Option<usize> {
        for i in 0..self.events.len() {
            if self.events[i].is_none() {
                return Some(i)
            }
        }

        return None
    }

    pub fn note_on(&mut self, frequency: f32, velocity: f32) {
        let note = SineNote::new(frequency, velocity);

        match self.next_free_idx() {
            Some(idx) => {
                println!("picked {} for {}", idx, frequency);
                self.events[idx] = Some(note)
            },
            None => ()
        }
    }

    pub fn note_off(&mut self, frequency: f32) {
        for note in self.events.iter_mut() {
            match note.as_mut() {
                Some(ref mut note) => {
                    if note.frequency == frequency && note.note_on {
                        (*note).note_on = false;
                    }
                },
                None => ()
            }
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<Option<SineNote>> {
        self.events.iter_mut()
    }
}

type OPort = jack::OutputPortHandle<jack::DefaultAudioSample>;
type IPort = jack::InputPortHandle<jack::MidiEvent>;

struct AudioHandler {
    input: IPort,
    output: OPort,
    generator: SineWaveGenerator,
    ev: EventManager
}

impl AudioHandler {
    pub fn new(input: IPort, output: OPort, generator: SineWaveGenerator) -> Self {
        AudioHandler {
            input,
            output,
            generator,
            ev: EventManager::new(64)
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

        for i in 0..(nframes as usize) {
            if event_index < event_count {
                while event_index < event_count
                    && input_buffer.get(event_index).get_jack_time() == (i as u32)
                {
                    let event = input_buffer.get(event_index);
                    let buf = event.raw_midi_bytes();

                    let m = rimd::MidiMessage { data: buf.to_vec() };
                    match m.status() {
                        rimd::Status::NoteOff => {
                            let f = AudioHandler::midi_note_to_frequency(m.data[1]);
                            self.ev.note_off(f);
                        },

                        rimd::Status::NoteOn => {
                            let f = AudioHandler::midi_note_to_frequency(m.data[1]);
                            let v = AudioHandler::midi_velocity_to_velocity(m.data[2]);
                            self.ev.note_on(f, v);
                        },

                        _ => ()
                    }

                    event_index += 1;
                }

            }

            let mut frame = 0.0;
            for el in self.ev.iter_mut() {
                if el.is_some() {
                    let dead = {
                        let note = el.as_mut().unwrap();
                        frame += self.generator.generate(note).min(1.0);
                        self.generator.is_note_dead(note)
                    };

                    if dead {
                        println!("killed {}", el.unwrap().frequency);
                        *el = None
                    }
                }
            }

            output_buffer[i] = frame.min(1.0).max(-1.0);
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
