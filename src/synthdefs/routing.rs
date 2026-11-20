//! The routing of SuperDirt: the gate of each sound, and the routing of a
//! bus to another.

use scsynth_synthdef::ArgRate::{Ir, Kr};
use scsynth_synthdef::{Arg, BuildError, SynthDef, SynthDefBuilder};

use super::arguments;
use super::ugens::dirt_gate_cut_group;

/// The definition `dirt_gate<channels>`: the gate of a sound. It moves
/// `channels` channels from the bus `in` to the bus `out`, under an envelope
/// of `fadeInTime`, `sustain` and `fadeTime`, and a level of `amp`, `gain`
/// and `overgain`. When its envelope ends, or `fadeTime` after its control
/// `cut_gate` goes to 0, it frees the group that holds it.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_gate(channels: u32) -> Result<SynthDef, BuildError> {
    let num_channels = f64::from(channels);
    let mut builder = SynthDefBuilder::new(
        format!("dirt_gate{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("in", 0.0).rate(Ir),
            Arg::new("sustain", 1.0).rate(Ir),
            Arg::new("fadeInTime", 0.001).rate(Ir),
            Arg::new("fadeTime", 0.001).rate(Ir),
            Arg::new("amp", 1.0).rate(Kr),
            Arg::new("gain", 1.0).rate(Ir),
            Arg::new("overgain", 0.0).rate(Ir),
        ],
    )?;
    let [
        out,
        r#in,
        sustain,
        fade_in_time,
        fade_time,
        amp,
        gain,
        overgain,
    ] = arguments(&builder);
    let signal = builder.ugen(
        "In",
        "ar",
        &[
            ("bus", r#in.clone().into()),
            ("numChannels", num_channels.into()),
        ],
    )?;
    let envelope = builder.env(
        "new",
        &[
            (
                "levels",
                vec![0.0.into(), 1.0.into(), 1.0.into(), 0.0.into()].into(),
            ),
            (
                "times",
                vec![fade_in_time, sustain, fade_time.clone()].into(),
            ),
            ("curve", "sin".into()),
        ],
    )?;
    let env = builder.ugen(
        "EnvGen",
        "ar",
        &[("envelope", envelope.into()), ("doneAction", 14.0.into())],
    )?;
    let gain = builder.binary_op("min", gain, 2.0)?;
    let gain = builder.binary_op("+", gain, overgain)?;
    let gain = builder.binary_op("pow", gain, 4.0)?;
    let amp = builder.binary_op("*", amp, gain)?;
    let signal = builder.binary_op("*", signal, env)?;
    let signal = builder.binary_op("*", signal, amp)?;
    let cut_group = dirt_gate_cut_group(&mut builder, fade_time, 14.0)?;
    let signal = builder.binary_op("*", signal, cut_group)?;
    builder.ugen(
        "OffsetOut",
        "ar",
        &[("bus", out.into()), ("channelsArray", signal.into())],
    )?;
    let silence = builder.ugen("Silent", "ar", &[("numChannels", num_channels.into())])?;
    builder.ugen(
        "ReplaceOut",
        "ar",
        &[("bus", r#in.into()), ("channelsArray", silence.into())],
    )?;
    builder.finish()
}

/// The definition `dirt_out<channels>`: it adds `channels` channels of the
/// bus `out` to the bus `to`, under an envelope of `fadeInTime`, `sustain`
/// and `fadeTime` and a level of `amp`, and leaves them on `out` scaled by
/// `through`.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_out(channels: u32) -> Result<SynthDef, BuildError> {
    let num_channels = f64::from(channels);
    let mut builder = SynthDefBuilder::new(
        format!("dirt_out{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("to", 0.0).rate(Ir),
            Arg::new("sustain", 1.0).rate(Ir),
            Arg::new("fadeInTime", 0.001).rate(Ir),
            Arg::new("fadeTime", 0.001).rate(Ir),
            Arg::new("through", 0.0).rate(Ir),
            Arg::new("amp", 1.0).rate(Kr),
        ],
    )?;
    let [out, to, sustain, fade_in_time, fade_time, through, amp] = arguments(&builder);
    let signal = builder.ugen(
        "In",
        "ar",
        &[
            ("bus", out.clone().into()),
            ("numChannels", num_channels.into()),
        ],
    )?;
    let envelope = builder.env(
        "new",
        &[
            (
                "levels",
                vec![0.0.into(), 1.0.into(), 1.0.into(), 0.0.into()].into(),
            ),
            ("times", vec![fade_in_time, sustain, fade_time].into()),
            ("curve", "sin".into()),
        ],
    )?;
    let env = builder.ugen("EnvGen", "ar", &[("envelope", envelope.into())])?;
    let sent = builder.binary_op("*", signal.clone(), env)?;
    let sent = builder.binary_op("*", sent, amp)?;
    builder.ugen(
        "Out",
        "ar",
        &[("bus", to.into()), ("channelsArray", sent.into())],
    )?;
    let kept = builder.binary_op("*", signal, through)?;
    builder.ugen(
        "ReplaceOut",
        "ar",
        &[("bus", out.into()), ("channelsArray", kept.into())],
    )?;
    builder.finish()
}
