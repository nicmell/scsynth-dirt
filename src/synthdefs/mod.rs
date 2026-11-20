//! The 25 core synth definitions of SuperDirt, built with scsynth-synthdef.
//! [`core()`] gives them for a channel count, in the order in which SuperDirt
//! loads them. A function builds the definition of its name for the channel
//! counts that it takes: `dirt_gate(2)` builds `dirt_gate2`, and
//! `dirt_sample(1, 2)` builds `dirt_sample_1_2`. Each definition follows its
//! source in SuperDirt at commit `a3046d1`, in `synths/core-synths.scd` or
//! `synths/core-synths-global.scd`.
//!
//! - The global synths, which run once per orbit: [`dirt_monitor`],
//!   [`dirt_rms`], [`dirt_delay`] with the delay line of a [`Delay`],
//!   [`dirt_reverb`] and [`dirt_leslie`].
//! - The sample players: [`dirt_sample`], [`dirt_sample_long`] and
//!   [`dirt_stretchsample`].
//! - The routing of a sound: [`dirt_gate`] and [`dirt_out`].
//! - The effects of a sound: [`dirt_vowel`], [`dirt_crush`], [`dirt_coarse`],
//!   [`dirt_hpf`], [`dirt_bpf`], [`dirt_shape`], [`dirt_lpf`],
//!   [`dirt_envelope`], [`dirt_grain_envelope`], [`dirt_pitch_shift`],
//!   [`dirt_tremolo`] and [`dirt_phaser`].

mod effects;
mod global;
mod routing;
mod sample;
mod ugens;

pub use effects::{
    dirt_bpf, dirt_coarse, dirt_crush, dirt_envelope, dirt_grain_envelope, dirt_hpf, dirt_lpf,
    dirt_phaser, dirt_pitch_shift, dirt_shape, dirt_tremolo, dirt_vowel,
};
pub use global::{Delay, dirt_delay, dirt_leslie, dirt_monitor, dirt_reverb, dirt_rms};
pub use routing::{dirt_gate, dirt_out};
pub use sample::{dirt_sample, dirt_sample_long, dirt_stretchsample};

use scsynth_synthdef::{BuildError, SynthDef, SynthDefBuilder, UGenInput};

/// The 25 definitions of SuperDirt for `channels` channels, in the order in
/// which SuperDirt loads them: the global synths, with the delay line
/// `delay`, then the definitions of a sound. The sample players are those
/// for samples of 1 and 2 channels.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
/// [`BuildError::UnknownClass`] for [`Delay::Switch`] without the feature
/// `sc3-plugins`.
pub fn core(channels: u32, delay: Delay) -> Result<Vec<SynthDef>, BuildError> {
    Ok(vec![
        dirt_monitor(channels)?,
        dirt_rms(channels)?,
        dirt_delay(channels, delay)?,
        dirt_reverb(channels)?,
        dirt_leslie(channels)?,
        dirt_sample(1, channels)?,
        dirt_sample(2, channels)?,
        dirt_sample_long(1, channels)?,
        dirt_sample_long(2, channels)?,
        dirt_stretchsample(1, channels)?,
        dirt_stretchsample(2, channels)?,
        dirt_gate(channels)?,
        dirt_out(channels)?,
        dirt_vowel(channels)?,
        dirt_crush(channels)?,
        dirt_coarse(channels)?,
        dirt_hpf(channels)?,
        dirt_bpf(channels)?,
        dirt_shape(channels)?,
        dirt_lpf(channels)?,
        dirt_envelope(channels)?,
        dirt_grain_envelope(channels)?,
        dirt_pitch_shift(channels)?,
        dirt_tremolo(channels)?,
        dirt_phaser(channels)?,
    ])
}

/// The input of each of the first `N` arguments of `builder`, in their order.
fn arguments<const N: usize>(builder: &SynthDefBuilder) -> [UGenInput; N] {
    std::array::from_fn(|index| builder.arguments()[index].clone())
}

/// `input.asArray` in sclang: the elements of an array, or the input alone.
fn as_array(input: UGenInput) -> Vec<UGenInput> {
    match input {
        UGenInput::Array(items) => items,
        other => vec![other],
    }
}

/// `items.sum` in sclang: 0, plus each item in order.
fn sum(builder: &mut SynthDefBuilder, items: Vec<UGenInput>) -> Result<UGenInput, BuildError> {
    items
        .into_iter()
        .try_fold(UGenInput::Number(0.0), |total, item| {
            builder.binary_op("+", total, item)
        })
}

/// `In.ar(bus, numChannels)`: `channels` channels of the bus `bus`.
fn in_ar(
    builder: &mut SynthDefBuilder,
    bus: &UGenInput,
    channels: u32,
) -> Result<UGenInput, BuildError> {
    builder.ugen(
        "In",
        "ar",
        &[
            ("bus", bus.clone().into()),
            ("numChannels", f64::from(channels).into()),
        ],
    )
}

/// A count, or an index, as a number of sclang.
#[allow(
    clippy::cast_precision_loss,
    reason = "a count of signals or of characters, and an index among them, stay far below 2 ** 53, up to which an f64 holds every integer"
)]
fn number(value: usize) -> f64 {
    value as f64
}
