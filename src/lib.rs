#![doc = include_str!("../README.md")]

mod config;
mod dirt;
#[cfg(feature = "fs")]
pub mod fs;
mod library;
mod orbit;
mod sclang;
mod sound;
pub mod synthdefs;
mod vowel;

pub use config::{Config, ConfigError};
pub use dirt::Dirt;
pub use library::{Library, LibraryError, Sample, SampleInfo};
pub use orbit::Orbit;
pub use sound::{Sound, SoundError, Value};
pub use synthdefs::Delay;
pub use vowel::Register;

/// The OSC protocol of scsynth: the types of the packets that this crate returns.
pub use scsynth_osc;
/// The synth definitions of SuperCollider: the compiler of the definitions of this crate.
pub use scsynth_synthdef;
