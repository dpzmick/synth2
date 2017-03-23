extern crate easyjack as jack;
extern crate rimd;

use std::thread;
use std::time::Duration;
use std::f32;
use std::slice::IterMut;

trait Note: Clone {
    fn new(frequency: f32, velocity: f32) -> Self;

    fn note_on(&self) -> bool;
    fn set_note_on(&mut self);
    fn set_note_off(&mut self);

    fn frequency(&self) -> f32;

    fn set_phase_offset(&mut self, phase_offest: usize);
}

trait WaveGenerator {
    type NoteType: Note;

    fn new(srate: f32) -> Self;

    fn new_note(&mut self, frequency: f32, velocity: f32) -> Self::NoteType {
        Self::NoteType::new(frequency, velocity)
    }

    fn generate(&mut self, note: &mut Self::NoteType) -> f32;
    fn is_note_dead(&self, note: &Self::NoteType) -> bool;
}

#[derive(Copy, Clone)]
struct SineNote {
    frequency: f32,
    velocity: f32, /* 0 -> 1 */
    phase: usize,
    note_on: bool // note will decay when it is no longer on, when note off, velocity will decay
}

impl Note for SineNote {
    fn new(frequency: f32, velocity: f32) -> Self {
        SineNote {
            frequency,
            velocity,
            phase: 0,
            note_on: true,
        }
    }

    fn note_on(&self) -> bool { self.note_on }
    fn set_note_on(&mut self) { self.note_on = true }
    fn set_note_off(&mut self) { self.note_on = false }

    fn frequency(&self) -> f32 { self.frequency }

    // TODO check for wraparound?
    fn set_phase_offset(&mut self, phase_offest: usize) { self.phase += phase_offest }
}

struct SineWaveGenerator {
    srate_constant: f32,
    decay_constant: f32,
}

impl WaveGenerator for SineWaveGenerator {
    type NoteType = SineNote;

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

        if !note.note_on() {
            note.velocity = note.velocity * self.decay_constant;
        }

        let x = note.phase as f32;
        note.velocity * (2.0 * std::f32::consts::PI * (note.frequency/self.srate_constant) * x).sin()
    }

    fn is_note_dead(&self, note: &SineNote) -> bool {
        note.velocity < 0.01
    }
}

struct SquareWaveGenerator {
    sine_gen: SineWaveGenerator
}

impl WaveGenerator for SquareWaveGenerator {
    type NoteType = SineNote;

    fn new(srate: f32) -> Self {
        Self {
            sine_gen: SineWaveGenerator::new(srate)
        }
    }

    fn generate(&mut self, note: &mut SineNote) -> f32 {
        let s = self.sine_gen.generate(note);
        if s > 0.0 {
            return 1.0 * note.velocity;
        } else {
            return -1.0 * note.velocity;
        }
    }

    fn is_note_dead(&self, note: &SineNote) -> bool {
        self.sine_gen.is_note_dead(note)
    }
}

// simpler types is perhaps a good argument for associated type?
struct WaveFusion<Gen1: WaveGenerator, Gen2: WaveGenerator> {
    gen1: Gen1,
    gen2: Gen2,
    phase_offest: usize,
}

impl<Gen1: WaveGenerator, Gen2: WaveGenerator> WaveFusion<Gen1, Gen2> {
    fn phased_new(srate: f32, phase_offest: usize) -> Self {
        Self {
            gen1: Gen1::new(srate),
            gen2: Gen2::new(srate),
            phase_offest,
        }
    }
}

impl<Gen1: WaveGenerator, Gen2: WaveGenerator> WaveGenerator for WaveFusion<Gen1, Gen2> {
    type NoteType = WaveFusionNote<
        <Gen1 as WaveGenerator>::NoteType,
        <Gen2 as WaveGenerator>::NoteType>;

    fn new(srate: f32) -> Self {
        Self::phased_new(srate, 0)
    }

    fn new_note(&mut self, frequency: f32, velocity: f32) -> Self::NoteType {
        let n1 = <Gen1 as WaveGenerator>::NoteType::new(frequency, velocity);
        let mut n2 = <Gen2 as WaveGenerator>::NoteType::new(frequency, velocity);

        n2.set_phase_offset(self.phase_offest);

        Self::NoteType::phased_new(n1, n2)
    }

    fn generate(&mut self, note: &mut Self::NoteType) -> f32 {
        let g1 = self.gen1.generate(&mut note.n1);
        let g2 = self.gen2.generate(&mut note.n2);

        (0.9 * g1) + (0.1 * g2)
    }

    fn is_note_dead(&self, note: &Self::NoteType) -> bool {
        self.gen1.is_note_dead(&note.n1) && self.gen2.is_note_dead(&note.n2)
    }
}

#[derive(Copy, Clone)]
struct WaveFusionNote<N1: Note, N2: Note> {
    n1: N1,
    n2: N2,
}

impl <N1: Note, N2: Note> WaveFusionNote<N1, N2> {
    fn phased_new(n1: N1, n2: N2) -> Self {
        Self { n1, n2 }
    }
}

impl<N1: Note, N2: Note> Note for WaveFusionNote<N1, N2> {
    fn new(frequency: f32, velocity: f32) -> Self {
        Self {
            n1: N1::new(frequency, velocity),
            n2: N2::new(frequency, velocity),
        }
    }

    fn note_on(&self) -> bool {
        // both notes will be kept in sync, if one is on, so is the other
        self.n1.note_on()
    }

    fn set_note_on(&mut self) {
        self.n1.set_note_on();
        self.n2.set_note_on();
    }

    fn set_note_off(&mut self) {
        self.n1.set_note_off();
        self.n2.set_note_off();
    }

    fn frequency(&self) -> f32 {
        // both notes constructed with the same frequency, they shouldn't be changing their
        // frequencies, but this might not be a valid assertion to make
        self.n1.frequency()
    }

    fn set_phase_offset(&mut self, phase_offest: usize) {
        self.n1.set_phase_offset(phase_offest);
        self.n2.set_phase_offset(phase_offest);
    }
}

struct EventManager<T: Note> {
    events: Vec<Option<T>>
}

impl<T: Note> EventManager<T> {
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

    pub fn note_on(&mut self, note: T) {
        match self.next_free_idx() {
            Some(idx) => {
                println!("picked {} for {}", idx, note.frequency());
                self.events[idx] = Some(note)
            },
            None => ()
        }
    }

    pub fn note_off(&mut self, frequency: f32) {
        for note in self.events.iter_mut() {
            match note.as_mut() {
                Some(ref mut note) => {
                    if note.frequency() == frequency && note.note_on() {
                        note.set_note_off();
                    }
                },
                None => ()
            }
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<Option<T>> {
        self.events.iter_mut()
    }
}

type OPort = jack::OutputPortHandle<jack::DefaultAudioSample>;
type IPort = jack::InputPortHandle<jack::MidiEvent>;

fn midi_note_to_frequency(note: u8) -> f32 {
    let a = 440.0;
    // this is a magic formula from the internet
    (a / 32.0) * (2.0_f32.powf( (note as f32 - 9.0) / 12.0 ))
}

fn midi_velocity_to_velocity(vel: u8) -> f32 {
    vel as f32 / (std::u8::MAX as f32)
}

struct AudioHandler<Gen: WaveGenerator> {
    input: IPort,
    output: OPort,
    generator: Gen,
    ev: EventManager< <Gen as WaveGenerator>::NoteType >,
}

impl<Gen: WaveGenerator> AudioHandler<Gen> {
    pub fn new(input: IPort, output: OPort, generator: Gen) -> Self {
        Self {
            input,
            output,
            generator,
            ev: EventManager::new(64)
        }
    }

}

impl<Gen: WaveGenerator> jack::ProcessHandler for AudioHandler<Gen> {
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
                            let f = midi_note_to_frequency(m.data[1]);
                            self.ev.note_off(f);
                        },

                        rimd::Status::NoteOn => {
                            let f = midi_note_to_frequency(m.data[1]);
                            let v = midi_velocity_to_velocity(m.data[2]);
                            let note = self.generator.new_note(f, v);
                            self.ev.note_on(note);
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
                        *el = None
                    }
                }
            }

            output_buffer[i] = frame.min(0.9).max(-0.9);
        }

        0
    }
}

fn main() {
    let gen = WaveFusion::<SineWaveGenerator, SquareWaveGenerator>::phased_new(44100.0, 2048);

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
