//! The pseudo-UGens of SuperDirt, from `classes/SuperDirtUGens.sc`: each
//! adds the UGens of its class method to a build.

use scsynth_synthdef::{BuildError, SynthDefBuilder, UGenInput};

use super::{as_array, sum};

/// `DirtPan.ar(signal, numChannels, pan, mul)` with the default panning
/// function of `DirtPan`: the signals of `signal` on `num_channels`
/// channels, at `pan` from -1 to 1. One channel takes the sum of the
/// signals. On 1 or 2 channels, `mul` scales the sound. `signal` holds at
/// least one signal.
pub(super) fn dirt_pan(
    builder: &mut SynthDefBuilder,
    signal: UGenInput,
    num_channels: u32,
    pan: UGenInput,
    mul: UGenInput,
) -> Result<UGenInput, BuildError> {
    let signals = as_array(signal);
    match num_channels {
        1 => {
            let signal = sum(builder, signals)?;
            builder.binary_op("*", signal, mul)
        }
        2 => {
            let span = named_ir(builder, "span", 1.0)?;
            dirt_pan_balance2(builder, signals, span, pan, mul)
        }
        _ => {
            let span = named_ir(builder, "span", 1.0)?;
            let splay = named_ir(builder, "splay", 1.0)?;
            let width = named_ir(builder, "panwidth", 2.0)?;
            let orientation = named_ir(builder, "orientation", 0.0)?;
            // DirtSplayAz.ar takes mul as its fifth argument, and reads it nowhere.
            dirt_splay_az(
                builder,
                num_channels,
                signals,
                span,
                pan,
                splay,
                &width,
                &orientation,
            )
        }
    }
}

/// `DirtPanBalance2.ar(signals, span, pan, mul)`: one signal through `Pan2`,
/// and two through `Balance2`, at `pan` folded into -1 to 1. More signals
/// go through `Splay` first, which spreads them over `span`.
fn dirt_pan_balance2(
    builder: &mut SynthDefBuilder,
    signals: Vec<UGenInput>,
    span: UGenInput,
    pan: UGenInput,
    mul: UGenInput,
) -> Result<UGenInput, BuildError> {
    let n = signals.len();
    if n == 1 {
        let pos = fold(builder, pan)?;
        return builder.ugen(
            "Pan2",
            "ar",
            &[
                ("in", signals[0].clone().into()),
                ("pos", pos.into()),
                ("level", mul.into()),
            ],
        );
    }
    let signals = if n > 2 {
        as_array(builder.ugen(
            "Splay",
            "ar",
            &[("inArray", signals.into()), ("spread", span.into())],
        )?)
    } else {
        signals
    };
    let pos = fold(builder, pan)?;
    builder.ugen(
        "Balance2",
        "ar",
        &[
            ("left", signals[0].clone().into()),
            ("right", signals[1].clone().into()),
            ("pos", pos.into()),
            ("level", mul.into()),
        ],
    )
}

/// `DirtSplayAz.ar(numChannels, signals, span, pan, mul, splay, width,
/// orientation)`: the mix of a `PanAz` of `num_channels` channels for each
/// signal. The signals spread from `pan` by `span` and `splay`.
#[allow(clippy::too_many_arguments, reason = "the arguments of DirtSplayAz.ar")]
fn dirt_splay_az(
    builder: &mut SynthDefBuilder,
    num_channels: u32,
    signals: Vec<UGenInput>,
    span: UGenInput,
    pan: UGenInput,
    splay: UGenInput,
    width: &UGenInput,
    orientation: &UGenInput,
) -> Result<UGenInput, BuildError> {
    let num_channels = f64::from(num_channels);
    let n = number(signals.len());
    let scale = builder.send(
        splay,
        "linlin",
        &[
            ("inMin", 0.0.into()),
            ("inMax", 1.0.into()),
            ("outMin", (num_channels / n).into()),
            ("outMax", 1.0.into()),
        ],
    )?;
    let span = builder.binary_op("*", span, scale)?;
    let pan = builder.binary_op("+", pan, 1.0)?;
    let mut channels = Vec::with_capacity(signals.len());
    for (i, x) in signals.into_iter().enumerate() {
        let pan_offset = builder.binary_op("*", number(i) / num_channels * 2.0, span.clone())?;
        let pos = builder.binary_op("+", pan_offset, pan.clone())?;
        channels.push(builder.ugen(
            "PanAz",
            "ar",
            &[
                ("numChans", num_channels.into()),
                ("in", x.into()),
                ("pos", pos.into()),
                ("width", width.clone().into()),
                ("orientation", orientation.clone().into()),
            ],
        )?);
    }
    builder.ugen("Mix", "new", &[("array", channels.into())])
}

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

/// `\name.ir(value)`: the named control `name` at scalar rate, with the
/// default `value`.
fn named_ir(
    builder: &mut SynthDefBuilder,
    name: &str,
    value: f64,
) -> Result<UGenInput, BuildError> {
    builder.ugen(
        "NamedControl",
        "ir",
        &[("name", name.into()), ("values", value.into())],
    )
}

/// `pan.fold(-1, 1)`.
fn fold(builder: &mut SynthDefBuilder, pan: UGenInput) -> Result<UGenInput, BuildError> {
    builder.send(pan, "fold", &[("lo", (-1.0).into()), ("hi", 1.0.into())])
}

/// A count of signals, or an index among them, as a number of sclang.
#[allow(
    clippy::cast_precision_loss,
    reason = "a count of signals stays far below 2 ** 53, up to which an f64 holds every integer"
)]
fn number(value: usize) -> f64 {
    value as f64
}
