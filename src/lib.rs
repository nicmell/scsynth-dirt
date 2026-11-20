#![doc = include_str!("../README.md")]

mod sclang;
mod sound;
pub mod synthdefs;
mod vowel;

pub use sound::{Sound, SoundError, Value};
pub use synthdefs::Delay;
pub use vowel::Register;

/// The OSC protocol of scsynth: the types of the packets that this crate returns.
pub use scsynth_osc;
/// The synth definitions of SuperCollider: the compiler of the definitions of this crate.
pub use scsynth_synthdef;
