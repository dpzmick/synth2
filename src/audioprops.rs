/// A simple audio property change
/// Expected to be representable with a Copy type
#[derive(Copy, Clone, Debug)]
pub enum AudioProperties {
    SampleRate(f32)
}
