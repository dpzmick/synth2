use enum_primitive::FromPrimitive;

// mostly ripped off from rimd crate
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

/// A struct holding a MIDI message and some extra data
pub struct MidiMessage<'a> {
    pub data: &'a [u8],
}

impl<'a> MidiMessage<'a> {
    pub fn status(&self) -> MidiStatus
    {
        MidiStatus::from_u8(self.data[0]).unwrap()
    }
}
