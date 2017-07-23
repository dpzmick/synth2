use patch::Patch;
use voice::Voice;

/// A soundscape contains many voices, manages NoteOn/NoteOff for each voice
/// For the moment, this will just make lots of copies. There's lots of room
/// for optimization
/// though
pub struct Soundscape<'a> {
    // this would be an array, but arrays are so severely limited in rust that I'm using a vector.
    // Don't ever resize it!
    // TODO make this not resizable
    voices: Vec<Voice<'a>>,
}

impl<'a> Soundscape<'a> {
    pub fn new(polyphony: usize, p: Patch) -> Self
    {
        let mut voices = Vec::new();
        for _ in 0..polyphony {
            voices.push(Voice::new(&p).unwrap());
        }

        Self { voices }
    }

    pub fn note_on(&mut self, freq: f32, vel: f32)
    {
        for voice in &mut self.voices {
            match voice.current_frequency() {
                Some(_f) => (),
                None => {
                    voice.note_on(freq, vel);
                    return;
                },
            }
        }

        // TODO replacement policy
    }

    pub fn note_off(&mut self, freq: f32)
    {
        for voice in &mut self.voices {
            if let Some(f) = voice.current_frequency() {
                if f == freq {
                    voice.note_off(f)
                }
            }
        }
    }

    pub fn control_value_change(&mut self, cc: u8, new_val: u8)
    {
        // all voices get the change
        for voice in &mut self.voices {
            voice.control_value_change(cc, new_val)
        }
    }

    pub fn generate(&mut self) -> f32
    {
        let mut sample = 0.0;
        for voice in &mut self.voices {
            let subsample = voice.generate();
            sample += subsample;
        }

        sample * (1.0 / self.voices.len() as f32)
    }
}
