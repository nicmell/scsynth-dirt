//! The pseudo-UGens of SuperDirt, from `classes/SuperDirtUGens.sc`: each
//! adds the UGens of its class method to a build.

use scsynth_synthdef::{BuildError, SynthDefBuilder, UGenInput};

/// `DirtGateCutGroup.ar(releaseTime, doneAction)`: an envelope that holds 1
/// until the control `cut_gate` closes, then falls to 0 in `release_time`,
/// and does `done_action`.
pub(super) fn dirt_gate_cut_group(
    builder: &mut SynthDefBuilder,
    release_time: UGenInput,
    done_action: f64,
) -> Result<UGenInput, BuildError> {
    let envelope = builder.env("cutoff", &[("releaseTime", release_time.into())])?;
    let cut_gate = builder.ugen(
        "NamedControl",
        "kr",
        &[("name", "cut_gate".into()), ("values", 1.0.into())],
    )?;
    builder.ugen(
        "EnvGen",
        "kr",
        &[
            ("envelope", envelope.into()),
            ("gate", cut_gate.into()),
            ("doneAction", done_action.into()),
        ],
    )
}
