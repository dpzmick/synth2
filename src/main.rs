extern crate easyjack as jack;
#[macro_use]
extern crate generic_array;
#[macro_use]
extern crate enum_primitive;
extern crate time;

use std::thread;
use std::time::Duration;
use std::f32;
use std::f64;
use std::slice::Iter;
use std::slice::IterMut;
use std::marker::PhantomData;
use std::cell::{RefMut, RefCell};
use std::process;
use std::mem;

use generic_array::ArrayLength;
use generic_array::GenericArray;
use generic_array::typenum::{U2, U3, U4096};
use enum_primitive::FromPrimitive;

static SRATE: f32 = 44100.0;

struct MidiMessage<'a> {
    pub data: &'a [u8]
}

enum_from_primitive! {
#[derive(Debug,PartialEq,Clone,Copy)]
pub enum MidiStatus {
    // voice
    NoteOff = 0x80,
    NoteOn = 0x90,
    PolyphonicAftertouch = 0xA0,
    ControlChange = 0xB0,
    ProgramChange = 0xC0,
    ChannelAftertouch = 0xD0,
    PitchBend = 0xE0,

    // sysex
    SysExStart = 0xF0,
    MIDITimeCodeQtrFrame = 0xF1,
    SongPositionPointer = 0xF2,
    SongSelect = 0xF3,
    TuneRequest = 0xF6, // F4 anf 5 are reserved and unused
    SysExEnd = 0xF7,
    TimingClock = 0xF8,
    Start = 0xFA,
    Continue = 0xFB,
    Stop = 0xFC,
    ActiveSensing = 0xFE, // FD also res/unused
    SystemReset = 0xFF,
}
}

impl<'a> MidiMessage<'a> {
    pub fn status(&self) -> MidiStatus {
        MidiStatus::from_u8(self.data[0]).unwrap()
    }
}

/// Signal generators generate a predefined signal
trait SignalGenerator {
    /// Get the value of the signal at time t, where t: [0,1]
    /// Moving from [0, 1] one time should represent one full cycle of the signal
    fn generate(&self, t: f32) -> f32;
}

// POD for a sound generator note
#[derive(Copy, Clone)]
struct Note {
    velocity: f32,
    frequency: f32,
    on: bool, // is the note currently being played on an instrument
    phase: f32,
}

struct SoundGenerator<'a> {
    notes: [Option<Note>; 64],
    osc: &'a SignalGenerator,
}

impl<'a> SoundGenerator<'a> {
    fn next_free(&mut self) -> &mut Option<Note> {
        for note in self.notes.iter_mut() {
            if note.is_none() { return note; }
        }

        panic!("out of space");
    }
}

impl<'a> SoundGenerator<'a> {
    fn new(osc: &'a SignalGenerator) -> Self {
        Self {
            notes: [None; 64],
            osc
        }
    }

    /// A MIDI note on event occurred
    pub fn note_on(&mut self, frequency: f32, velocity: f32) {
        println!("note on");
        let note = Note {
            frequency,
            velocity,
            on: true,
            phase: 0.0
        };

        let free = self.next_free();
        *free = Some(note)
    }

    /// A MIDI note off event occurred
    pub fn note_off(&mut self, frequency: f32) {
        println!("note off");
        // TODO document that only one note may be "on" for each frequency
        // TODO envelope
        for optnote in self.notes.iter_mut() {
            if optnote.is_none() { continue; }

            let note = optnote.unwrap();
            if note.on && note.frequency == frequency {
                *optnote = None;
                return;
            }
        }

        panic!("note not found");
    }

    //pub fn set_oscillator(&mut self, osc: &SignalGenerator) { }
    //pub fn set_envelope(&mut self, env: &SignalGenerator) { }

    /// Generates the next sample
    pub fn generate(&mut self) -> f32 {
        let mut frame = 0.0;
        for note in self.notes.iter_mut() {
            match note.as_mut() {
                Some(ref mut note) => {
                    let subframe = self.osc.generate(note.phase);
                    frame += self.osc.generate(note.phase);
                    note.phase += 2.0 * (note.frequency / SRATE);

                    while note.phase > 1.0 {
                        note.phase -= 1.0;
                    }
                },

                None => ()
            }
        }

        frame.max(0.0).min(1.0)
    }
}

struct SineOscilator { }

impl SignalGenerator for SineOscilator {
    fn generate(&self, t: f32) -> f32 {
        assert!(t >= 0.0 && t <= 1.0);
        (t * f32::consts::PI).sin()
    }
}

fn sine_singleton() -> &'static SineOscilator {
    static SINGLETON: SineOscilator = SineOscilator { };
    &SINGLETON
}

/// Manages all of the things we currently have running and the connections between them
struct Soundscape<'a> {
    // TODO graph
    root: SoundGenerator<'a>,
}

impl<'a> Soundscape<'a> {
    fn new() -> Self {
        Self {
            root: SoundGenerator::new(sine_singleton())
        }
    }

    fn note_on(&mut self, freq: f32, vel: f32) {
        self.root.note_on(freq, vel);
    }

    fn note_off(&mut self, freq: f32) {
        self.root.note_off(freq);
    }

    fn generate(&mut self) -> f32 {
        self.root.generate()
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

struct AudioHandler<'a> {
    input: IPort,
    output: OPort,
    soundscape: Soundscape<'a>
}

impl<'a> AudioHandler<'a> {
    pub fn new(input: IPort, output: OPort) -> Self {
        Self {
            input,
            output,
            soundscape: Soundscape::new()
        }
    }

}

impl<'a> jack::ProcessHandler for AudioHandler<'a> {
    fn process(&mut self, ctx: &jack::CallbackContext, nframes: jack::NumFrames) -> i32 {
        let start = time::precise_time_ns();
        let output_buffer = self.output.get_write_buffer(nframes, &ctx);
        let input_buffer  = self.input.get_read_buffer(nframes, &ctx);
        let end = time::precise_time_ns();
        //println!("buffer setup: {}", end - start);

        let mut current_event = unsafe { mem::uninitialized() };
        let mut current_event_index = 0;
        let event_count = input_buffer.len();

        let start2 = time::precise_time_ns();
        for i in 0..(nframes as usize) {
            while current_event_index < event_count {
                current_event = input_buffer.get(current_event_index);
                if current_event.get_jack_time() as usize != i { break; }
                current_event_index += 1;

                let buf = current_event.raw_midi_bytes();
                let m = MidiMessage { data: buf };
                match m.status() {
                    MidiStatus::NoteOff => {
                        let f = midi_note_to_frequency(m.data[1]);
                        self.soundscape.note_off(f);
                    },

                    MidiStatus::NoteOn => {
                        let f = midi_note_to_frequency(m.data[1]);
                        let v = midi_velocity_to_velocity(m.data[2]);
                        self.soundscape.note_on(f, v);
                    },

                    _ => ()
                }

            }

            output_buffer[i] = self.soundscape.generate();
        }

        let end = time::precise_time_ns();
        //println!("frames filling: {}", end - start2);
        let end = time::precise_time_ns();
        //println!("elapsed: {}", end - start);
        0
    }
}

struct MDHandler { }

impl MDHandler {
    fn new() -> Self { Self {} }
}

impl jack::MetadataHandler for MDHandler {
    fn on_xrun(&mut self) -> i32 {
        println!("terminating due to xrun");
        1
    }

    fn callbacks_of_interest(&self) -> Vec<jack::MetadataHandlers> {
        vec![jack::MetadataHandlers::Xrun]
    }
}

fn main() {
    let mut c = jack::Client::open("sine", jack::options::NO_START_SERVER).unwrap().0;
    let i = c.register_input_midi_port("midi_in").unwrap();
    let o = c.register_output_audio_port("audio_out").unwrap();

    let handler = AudioHandler::new(i, o);
    let mdhandler = MDHandler::new();

    c.set_metadata_handler(mdhandler).unwrap();
    c.set_process_handler(handler).unwrap();

    c.activate().unwrap();
    c.connect_ports("sine:audio_out", "system:playback_1").unwrap();
    //c.connect_ports("jack-keyboard:midi_out", "sine:midi_in").unwrap();

    loop {
        thread::sleep(Duration::from_millis(100000));
    }

    c.close().unwrap();
}
