//! Kibble-481's sequencer: the project (tracks, notes, sections, mix), its timeline, the
//! engine that plays and mixes it live or offline, SoundFont instruments, recording,
//! and MIDI and WAV files.

mod chamber;
mod cinder;
pub mod city;
pub mod compose;
pub mod demo;
pub mod engine;
mod fill;
mod filmstrip;
pub mod fonts;
mod homeward;
mod lumen;
mod mantra;
pub mod midi_file;
mod nebula;
mod orbit;
pub mod pachinko;
pub mod palette;
pub mod project;
pub mod record;
pub mod render;
pub mod rng;
pub mod shape;
mod strata;
pub mod timeline;
mod vigil;

#[cfg(test)]
mod tests;
