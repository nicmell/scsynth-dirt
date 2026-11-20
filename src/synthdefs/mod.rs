//! The synth definitions of SuperDirt, built with scsynth-synthdef. A
//! function builds the definition of its name for the channel counts that it
//! takes: `dirt_gate(2)` builds `dirt_gate2`. Each definition follows its
//! source in SuperDirt at commit `a3046d1`, in `synths/core-synths.scd` or
//! `synths/core-synths-global.scd`.

mod effects;
mod routing;
mod sample;
mod ugens;

pub use effects::{
    dirt_bpf, dirt_coarse, dirt_crush, dirt_envelope, dirt_grain_envelope, dirt_hpf, dirt_lpf,
    dirt_phaser, dirt_pitch_shift, dirt_shape, dirt_tremolo, dirt_vowel,
};
pub use routing::{dirt_gate, dirt_out};
pub use sample::{dirt_sample, dirt_sample_long, dirt_stretchsample};

use scsynth_synthdef::{BuildError, SynthDefBuilder, UGenInput};

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
