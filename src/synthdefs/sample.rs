//! The sample players of SuperDirt: each plays a buffer, and pans the sound
//! on the channels of the orbit.

use scsynth_synthdef::ArgRate::{Ir, Kr};
use scsynth_synthdef::{Arg, BuildError, SynthDef, SynthDefBuilder, UGenInput};

use super::ugens::dirt_pan;
use super::{arguments, as_array, sum};
use crate::sclang;

/// The definition `dirt_sample_<sample_channels>_<channels>`: the player of
/// a buffer of `sample_channels` channels. `BufRd` reads the buffer
/// `bufnum` from `begin` to `end`, at a speed that goes from `speed` to
/// `endSpeed` in `sustain`, times `freq` over the frequency of note 60. The
/// sound goes on `channels` channels at `pan` to the bus `out`.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 sample channels, and for more than 65,535
/// sample channels or channels. [`BuildError::NotANumber`] for 0 channels.
pub fn dirt_sample(sample_channels: u32, channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_sample_{sample_channels}_{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("bufnum", 0.0).rate(Ir),
            Arg::new("sustain", 1.0).rate(Ir),
            Arg::new("begin", 0.0).rate(Ir),
            Arg::new("end", 1.0).rate(Ir),
            Arg::new("speed", 1.0).rate(Ir),
            Arg::new("endSpeed", 1.0).rate(Ir),
            Arg::new("freq", 440.0).rate(Ir),
            Arg::new("pan", 0.0).rate(Kr),
        ],
    )?;
    let [
        out,
        bufnum,
        sustain,
        begin,
        end,
        speed,
        end_speed,
        freq,
        pan,
    ] = arguments(&builder);
    let rate = playback_rate(&mut builder, speed.clone(), end_speed, sustain, freq)?;
    let num_frames = builder.ugen("BufFrames", "ir", &[("bufnum", bufnum.clone().into())])?;
    let num_frames = builder.binary_op("max", num_frames, 1.0)?;
    let sample_rate = builder.ugen("BufSampleRate", "ir", &[("bufnum", bufnum.clone().into())])?;
    let scaled = builder.binary_op("*", rate, sample_rate)?;
    let span = builder.binary_op("absdif", begin.clone(), end.clone())?;
    let span = builder.binary_op("*", span, num_frames.clone())?;
    let sawrate = builder.binary_op("/", scaled, span)?;
    let sign = builder.unary_op("sign", speed)?;
    let saw = builder.ugen(
        "LFSaw",
        "ar",
        &[("freq", sawrate.into()), ("iphase", 1.0.into())],
    )?;
    let phase = builder.binary_op("*", sign, saw)?;
    let phase = builder.send(phase, "range", &[("lo", begin.into()), ("hi", end.into())])?;
    let phase = builder.binary_op("*", phase, num_frames)?;
    let sound = buf_rd(&mut builder, sample_channels, bufnum, phase)?;
    let sound = dirt_pan(&mut builder, sound, channels, pan, 1.0.into())?;
    builder.ugen(
        "Out",
        "ar",
        &[("bus", out.into()), ("channelsArray", sound.into())],
    )?;
    builder.finish()
}

/// The definition `dirt_sample_long_<sample_channels>_<channels>`: the
/// player of a long buffer of `sample_channels` channels. `PlayBuf` plays
/// the buffer `bufnum` from `begin`, or from its end when it plays
/// backward from 0, at a speed that goes from `speed` to `endSpeed` in
/// `sustain`, times `freq` over the frequency of note 60. Unless
/// `roughOffset` is set, the start is at most frame 2 ** 24. The sound
/// lasts `sustain`, and goes on `channels` channels at `pan` to the bus
/// `out`.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 sample channels, and for more than 65,535
/// sample channels or channels. [`BuildError::NotANumber`] for 0 channels.
pub fn dirt_sample_long(sample_channels: u32, channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_sample_long_{sample_channels}_{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("bufnum", 0.0).rate(Ir),
            Arg::new("sustain", 1.0).rate(Ir),
            Arg::new("begin", 0.0).rate(Ir),
            Arg::new("end", 1.0).rate(Ir),
            Arg::new("speed", 1.0).rate(Ir),
            Arg::new("endSpeed", 1.0).rate(Ir),
            Arg::new("freq", 440.0).rate(Ir),
            Arg::new("pan", 0.0).rate(Kr),
            Arg::new("roughOffset", 0.0),
        ],
    )?;
    let [
        out,
        bufnum,
        sustain,
        begin,
        _end,
        speed,
        end_speed,
        freq,
        pan,
        rough_offset,
    ] = arguments(&builder);
    let rate = playback_rate(&mut builder, speed, end_speed, sustain.clone(), freq)?;
    let rate_scale = builder.ugen("BufRateScale", "ir", &[("bufnum", bufnum.clone().into())])?;
    let rate = builder.binary_op("*", rate, rate_scale)?;
    let num_frames = builder.ugen("BufFrames", "ir", &[("bufnum", bufnum.clone().into())])?;
    let num_frames = builder.binary_op("max", num_frames, 1.0)?;
    let backward = builder.binary_op("<", rate.clone(), 0.0)?;
    let from_zero = builder.binary_op("<=", begin.clone(), 0.0)?;
    let from_end = builder.binary_op("*", backward, from_zero)?;
    let from_begin = builder.binary_op("*", num_frames.clone(), begin)?;
    let start_pos = builder.send(
        from_end,
        "if",
        &[
            ("trueUGen", num_frames.into()),
            ("falseUGen", from_begin.into()),
        ],
    )?;
    let limited = builder.binary_op("min", start_pos.clone(), 2.0_f64.powf(24.0))?;
    let start_pos = builder.send(
        rough_offset,
        "if",
        &[
            ("trueUGen", start_pos.into()),
            ("falseUGen", limited.into()),
        ],
    )?;
    let sound = builder.ugen(
        "PlayBuf",
        "ar",
        &[
            ("numChannels", f64::from(sample_channels).into()),
            ("bufnum", bufnum.into()),
            ("rate", rate.into()),
            ("startPos", start_pos.into()),
            ("loop", 0.0.into()),
        ],
    )?;
    let envelope = builder.env(
        "linen",
        &[
            ("attackTime", 0.0.into()),
            ("sustainTime", sustain.into()),
            ("releaseTime", 0.0.into()),
        ],
    )?;
    let env = builder.ugen("EnvGen", "kr", &[("envelope", envelope.into())])?;
    let sound = builder.binary_op("*", sound, env)?;
    let sound = dirt_pan(&mut builder, sound, channels, pan, 1.0.into())?;
    builder.ugen(
        "Out",
        "ar",
        &[("bus", out.into()), ("channelsArray", sound.into())],
    )?;
    builder.finish()
}

/// The definition `dirt_stretchsample_<sample_channels>_<channels>`: the
/// player of a buffer of `sample_channels` channels that stretches its
/// time by `timescale`, without a change of pitch. Two readers of `BufRd`
/// overlap under Gaussian windows of `timescalewin` times a size from
/// `timescale`, from `begin` to `end` of the buffer `bufnum`. Their speed
/// goes from `speed` to `endSpeed` in `sustain`, times `freq` over the
/// frequency of note 60. The sound goes on `channels` channels at `pan` to
/// the bus `out`.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 sample channels, and for more than 65,535
/// sample channels or channels. [`BuildError::NotANumber`] for 0 channels.
pub fn dirt_stretchsample(sample_channels: u32, channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_stretchsample_{sample_channels}_{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("bufnum", 0.0).rate(Ir),
            Arg::new("sustain", 1.0).rate(Ir),
            Arg::new("begin", 0.0).rate(Ir),
            Arg::new("end", 1.0).rate(Ir),
            Arg::new("speed", 1.0).rate(Ir),
            Arg::new("endSpeed", 1.0).rate(Ir),
            Arg::new("freq", 440.0).rate(Ir),
            Arg::new("pan", 0.0).rate(Kr),
            Arg::new("timescale", 1.0).rate(Ir),
            Arg::new("timescalewin", 1.0).rate(Ir),
        ],
    )?;
    let [
        out,
        bufnum,
        sustain,
        begin,
        end,
        speed,
        end_speed,
        freq,
        pan,
        timescale,
        timescalewin,
    ] = arguments(&builder);
    let rate = playback_rate(&mut builder, speed.clone(), end_speed, sustain, freq)?;
    let num_frames = builder.ugen("BufFrames", "ir", &[("bufnum", bufnum.clone().into())])?;
    let window_size = builder.send(
        timescale.clone(),
        "clip",
        &[("lo", 0.1.into()), ("hi", 2.0.into())],
    )?;
    let window_size = builder.binary_op("*", window_size, 0.05)?;
    let sample_rate = builder.ugen("BufSampleRate", "ir", &[("bufnum", bufnum.clone().into())])?;
    let window_size = builder.binary_op("*", window_size, sample_rate)?;
    let window_size = builder.binary_op("*", window_size, timescalewin)?;
    let timescale_step = builder.binary_op("/", window_size.clone(), timescale.clone())?;
    let sample_rate = builder.ugen("BufSampleRate", "ir", &[("bufnum", bufnum.clone().into())])?;
    let scaled = builder.binary_op("*", rate, sample_rate)?;
    let span = builder.binary_op("absdif", begin.clone(), end.clone())?;
    let span = builder.binary_op("*", span, timescale.clone())?;
    let span = builder.binary_op("*", span, num_frames.clone())?;
    let sawrate = builder.binary_op("/", scaled, span)?;
    let sign = builder.unary_op("sign", speed)?;
    let saw = builder.ugen(
        "LFSaw",
        "ar",
        &[("freq", sawrate.into()), ("iphase", 1.0.into())],
    )?;
    let phase = builder.binary_op("*", sign, saw)?;
    let phase = builder.send(phase, "range", &[("lo", begin.into()), ("hi", end.into())])?;
    let phase = builder.binary_op("*", phase, num_frames)?;
    let phase = builder.binary_op("*", phase, timescale.clone())?;
    let n_steps = builder.binary_op("div", phase.clone(), timescale_step.clone())?;
    let n_steps = builder.binary_op("-", n_steps, vec![1.0.into(), 0.0.into()])?;
    let step = builder.binary_op("/", timescale_step.clone(), timescale)?;
    let step = builder.binary_op("-", step, timescale_step.clone())?;
    let index = builder.binary_op("*", n_steps.clone(), step)?;
    let index = as_array(builder.binary_op("+", index, phase.clone())?);
    let steps = builder.binary_op("*", n_steps, timescale_step)?;
    let window_index = builder.binary_op("-", phase, steps)?;
    let window = builder.binary_op("/", window_index, window_size)?;
    let window = builder.binary_op("-", window, 0.5)?;
    let window = builder.unary_op("squared", window)?;
    let window = builder.binary_op("*", -50.0, window)?;
    let window = as_array(builder.unary_op("exp", window)?);
    let sound0 = buf_rd(
        &mut builder,
        sample_channels,
        bufnum.clone(),
        index[0].clone(),
    )?;
    let sound0 = builder.binary_op("*", window[0].clone(), sound0)?;
    let sound1 = buf_rd(&mut builder, sample_channels, bufnum, index[1].clone())?;
    let sound1 = builder.binary_op("*", window[1].clone(), sound1)?;
    let sound = builder.binary_op("+", sound0, sound1)?;
    let total = sum(&mut builder, window)?;
    let sound = builder.binary_op("/", sound, total)?;
    let sound = dirt_pan(&mut builder, sound, channels, pan, 1.0.into())?;
    builder.ugen(
        "Out",
        "ar",
        &[("bus", out.into()), ("channelsArray", sound.into())],
    )?;
    builder.finish()
}

/// `Line.kr(speed, endSpeed, sustain) * (freq / 60.midicps)`: the playback
/// rate of a player.
fn playback_rate(
    builder: &mut SynthDefBuilder,
    speed: UGenInput,
    end_speed: UGenInput,
    sustain: UGenInput,
    freq: UGenInput,
) -> Result<UGenInput, BuildError> {
    let line = builder.ugen(
        "Line",
        "kr",
        &[
            ("start", speed.into()),
            ("end", end_speed.into()),
            ("dur", sustain.into()),
        ],
    )?;
    let ratio = builder.binary_op("/", freq, sclang::midicps(60.0))?;
    builder.binary_op("*", line, ratio)
}

/// `BufRd.ar(sampleNumChannels, bufnum, phase, 0, 4)`: `sample_channels`
/// channels of the buffer `bufnum` at `phase`, with no loop and with cubic
/// interpolation.
fn buf_rd(
    builder: &mut SynthDefBuilder,
    sample_channels: u32,
    bufnum: UGenInput,
    phase: UGenInput,
) -> Result<UGenInput, BuildError> {
    builder.ugen(
        "BufRd",
        "ar",
        &[
            ("numChannels", f64::from(sample_channels).into()),
            ("bufnum", bufnum.into()),
            ("phase", phase.into()),
            ("loop", 0.0.into()),
            ("interpolation", 4.0.into()),
        ],
    )
}
