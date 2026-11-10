//! Kibble-481's sound engine, free of any interface: the CS-style polysynth, the city kit,
//! and the effects (reverb, echo, ensemble) shared by every track.

pub mod effects;
pub mod fx;
pub mod kit;
pub mod patch;
pub mod poly;
pub mod rain;
pub mod reverb;
pub mod sampler;

/// Something that plays one track: a synth patch, a SoundFont preset or a kit. It hears
/// one MIDI channel's worth of messages and renders dry stereo; reverb and echo are
/// added by the mixer.
pub trait Instrument: Send {
    fn note_on(&mut self, key: u8, vel: u8);
    fn note_off(&mut self, key: u8);
    fn control(&mut self, _cc: u8, _value: u8) {}
    /// Pitch bend, -8192..=8191.
    fn bend(&mut self, _value: i16) {}
    /// Channel pressure (aftertouch).
    fn pressure(&mut self, _value: u8) {}
    /// Release every note.
    fn all_notes_off(&mut self);
    /// Silence at once and forget controllers.
    fn reset(&mut self);
    /// Overwrite both buffers with the next block.
    fn render(&mut self, left: &mut [f32], right: &mut [f32]);
    /// Change the synth patch without stopping the notes that are sounding.
    fn set_patch(&mut self, _patch: &patch::Patch) {}
    /// Offset from A = 440 Hz, in cents.
    fn set_tuning(&mut self, _cents: f32) {}
}
