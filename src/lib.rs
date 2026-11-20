#![doc = include_str!("../README.md")]

mod pitch;
pub mod synthdefs;

/// The OSC protocol of scsynth: the types of the packets that this crate returns.
pub use scsynth_osc;
/// The synth definitions of SuperCollider: the compiler of the definitions of this crate.
pub use scsynth_synthdef;
