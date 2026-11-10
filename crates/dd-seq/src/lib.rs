//! DDream's sequencer: the project (tracks, notes, sections, mix), its timeline, the
//! engine that plays and mixes it live or offline, SoundFont instruments, recording,
//! and MIDI and WAV files.

pub mod city;
pub mod compose;
pub mod demo;
pub mod engine;
pub mod fonts;
pub mod midi_file;
pub mod palette;
pub mod project;
pub mod record;
pub mod render;
pub mod rng;
pub mod timeline;

#[cfg(test)]
mod tests;
