//! The synth definitions of SuperDirt, built with scsynth-synthdef. A
//! function builds the definition of its name for the channel counts that it
//! takes: `dirt_gate(2)` builds `dirt_gate2`. Each definition follows its
//! source in SuperDirt at commit `a3046d1`, in `synths/core-synths.scd` or
//! `synths/core-synths-global.scd`.

mod routing;
mod ugens;

pub use routing::{dirt_gate, dirt_out};

use scsynth_synthdef::{SynthDefBuilder, UGenInput};

/// The input of each of the first `N` arguments of `builder`, in their order.
fn arguments<const N: usize>(builder: &SynthDefBuilder) -> [UGenInput; N] {
    std::array::from_fn(|index| builder.arguments()[index].clone())
}
