use voice::Voice;

/// A soundscape contains many voices, manages NoteOn/NoteOff for each voice
/// For the moment, this will just make lots of copies. There's lots of room
/// for optimization
/// though
pub struct Soundscape<'a> {
    // this would be an array, but arrays are so severely limited in rust that I'm using a vector.
    // Don't ever resize it!
    voices: Vec<Voice<'a>>,
}

impl<'a> Soundscape<'a> {
    pub fn new() -> Self
    {
        let mut voices = Vec::new();
        for _ in 0..16 {
            voices.push(Voice::new());
        }

        Self { voices }
    }

    pub fn example_connections(&mut self)
    {
        for voice in self.voices.iter_mut() {
            voice.example_connections()
        }
    }

    pub fn note_on(&mut self, freq: f32, vel: f32)
    {
        for voice in self.voices.iter_mut() {
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
        for voice in self.voices.iter_mut() {
            match voice.current_frequency() {
                Some(f) => {
                    if freq == f {
                        voice.note_off(f)
                    }
                },
                None => (),
            }
        }
    }

    pub fn generate(&mut self) -> f32
    {
        let mut sample = 0.0;
        for voice in self.voices.iter_mut() {
            let subsample = voice.generate();
            sample += subsample;
        }

        sample * (1.0 / self.voices.len() as f32)
    }
}
