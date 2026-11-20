//! The global synths of SuperDirt: each runs once per orbit, on the buses of
//! the orbit, and pauses itself in silence.

use std::f64::consts::PI;

use scsynth_synthdef::ArgRate::{Ir, Kr};
use scsynth_synthdef::{Arg, BuildError, NewUGen, Rate, SynthDef, SynthDefBuilder, UGenInput};

use super::ugens::dirt_pause;
use super::{arguments, as_array, in_ar, number, sum};
use crate::sclang;

/// The delay line of [`dirt_delay`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum Delay {
    /// `CombL`: the delay line of SuperDirt when sclang lacks sc3-plugins.
    #[default]
    Comb,
    /// `SwitchDelay` of sc3-plugins: the delay line of SuperDirt when sclang
    /// has sc3-plugins. Its definition needs the feature `sc3-plugins`.
    Switch,
}

/// The definition `dirt_monitor<channels>`: the output of an orbit. It adds
/// `channels` channels of the bus `effectBus` to those of the bus `dryBus`,
/// and puts 0 in place of each sample that is a NaN, an infinity or a
/// denormal, which it does not post. `limitertype` picks the sum (0), its
/// `Limiter` (1), or twice the soft clip of its half (2). The result goes to
/// the bus `outBus` under an envelope that rises in 0.01 seconds, and falls
/// in 1 second once `gate` closes. Then the synth frees itself. It pauses
/// itself as `DirtPause` does, with a grace time of 4 seconds.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_monitor(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_monitor{channels}"),
        vec![
            Arg::new("dryBus", 0.0).rate(Ir),
            Arg::new("effectBus", 0.0).rate(Ir),
            Arg::new("outBus", 0.0).rate(Kr),
            Arg::new("gate", 1.0).rate(Kr),
            Arg::new("limitertype", 1.0).rate(Kr),
        ],
    )?;
    let [dry_bus, effect_bus, out_bus, gate, limitertype] = arguments(&builder);
    let dry_signal = in_ar(&mut builder, &dry_bus, channels)?;
    let wet_signal = in_ar(&mut builder, &effect_bus, channels)?;
    let signal = builder.binary_op("+", wet_signal, dry_signal)?;
    let post = 0.0;
    let bad = builder.ugen(
        "CheckBadValues",
        "ar",
        &[("in", signal.clone().into()), ("post", post.into())],
    )?;
    let bad = builder.binary_op(">", bad, 0.0)?;
    let silence = builder.ugen("DC", "ar", &[("in", 0.0.into())])?;
    let signal = builder.ugen(
        "Select",
        "ar",
        &[
            ("which", bad.into()),
            ("array", vec![signal, silence].into()),
        ],
    )?;
    let limited = builder.ugen("Limiter", "ar", &[("in", signal.clone().into())])?;
    let clipped = builder.binary_op("*", signal.clone(), 0.5)?;
    let clipped = builder.unary_op("softclip", clipped)?;
    let clipped = builder.binary_op("*", clipped, 2.0)?;
    let signal = builder.ugen(
        "Select",
        "ar",
        &[
            ("which", limitertype.into()),
            ("array", vec![signal, limited, clipped].into()),
        ],
    )?;
    dirt_pause(&mut builder, signal.clone(), 4.0, 0.0)?;
    let env = asr(&mut builder, gate)?;
    let signal = builder.binary_op("*", signal, env)?;
    out(&mut builder, out_bus, signal)?;
    builder.finish()
}

/// The definition `dirt_rms<channels>`: the meter of an orbit. For each of
/// `channels` channels of the sum of the buses `effectBus` and `dryBus`, it
/// sends the peak and the power to the clients of the server,
/// `rmsReplyRate` times per second, as `/rms` replies with the ID
/// `orbitIndex`. The peaks pass through a lag of `rmsPeakLag` seconds. The
/// synth frees itself 1 second after `gate` closes. It pauses itself as
/// `DirtPause` does, with a grace time of 4 seconds.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_rms(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_rms{channels}"),
        vec![
            Arg::new("gate", 1.0).rate(Kr),
            Arg::new("dryBus", 0.0).rate(Ir),
            Arg::new("effectBus", 0.0).rate(Ir),
            Arg::new("rmsReplyRate", 8.0).rate(Kr),
            Arg::new("rmsPeakLag", 3.0).rate(Kr),
            Arg::new("orbitIndex", 0.0).rate(Kr),
        ],
    )?;
    let [
        gate,
        dry_bus,
        effect_bus,
        rms_reply_rate,
        rms_peak_lag,
        orbit_index,
    ] = arguments(&builder);
    let dry_signal = in_ar(&mut builder, &dry_bus, channels)?;
    let wet_signal = in_ar(&mut builder, &effect_bus, channels)?;
    let signal = builder.binary_op("+", wet_signal, dry_signal)?;
    dirt_pause(&mut builder, signal.clone(), 4.0, 0.0)?;
    asr(&mut builder, gate)?;
    send_peak_rms(
        &mut builder,
        signal,
        rms_reply_rate,
        rms_peak_lag,
        "/rms",
        orbit_index,
    )?;
    builder.finish()
}

/// The definition `dirt_delay<channels>` with the delay line `delay`: the
/// delay of an orbit. It echoes `channels` channels of the bus `dryBus`
/// every `delaytime`, up to 4 seconds, which counts in cycles of `cps` when
/// `lock` is 1, and in seconds when it is 0. Each echo has the level of the
/// one before times `delayfeedback`. The echoes go to the bus `effectBus` at
/// the level `delayAmp`, under an envelope that rises in 0.01 seconds, and
/// falls in 1 second once `gate` closes. Then the synth frees itself. It
/// pauses itself as `DirtPause` does, with a grace time of 4 seconds.
///
/// - [`Delay::Comb`] echoes through a `CombL`, with `delayfeedback` from 0
///   to 0.99 and echoes that fall by 60 dB in at most 20 seconds. A
///   `LeakDC` takes the offset out of the echoes.
/// - [`Delay::Switch`] echoes through a `SwitchDelay`, with `delayfeedback`
///   from 0. It scales its input by `delaySend`, and passes the input on
///   with the echoes.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
/// [`BuildError::UnknownClass`] for [`Delay::Switch`] without the feature
/// `sc3-plugins`.
pub fn dirt_delay(channels: u32, delay: Delay) -> Result<SynthDef, BuildError> {
    match delay {
        Delay::Comb => comb_delay(channels),
        Delay::Switch => switch_delay(channels),
    }
}

/// `dirt_delay<channels>` with the delay line of `CombL`.
fn comb_delay(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_delay{channels}"),
        vec![
            Arg::new("dryBus", 0.0).rate(Ir),
            Arg::new("effectBus", 0.0).rate(Ir),
            Arg::new("gate", 1.0),
            Arg::new("delaytime", 0.0),
            Arg::new("delayfeedback", 0.0),
            Arg::new("delayAmp", 1.0),
            Arg::new("lock", 0.0),
            Arg::new("cps", 1.0),
        ],
    )?;
    let [
        dry_bus,
        effect_bus,
        gate,
        delaytime,
        delayfeedback,
        delay_amp,
        lock,
        cps,
    ] = arguments(&builder);
    let signal = in_ar(&mut builder, &dry_bus, channels)?;
    let max_delay_time = 4.0;
    let delayfeedback = clip(&mut builder, delayfeedback, 0.0, 0.99)?;
    let delaytime = locked(&mut builder, delaytime, lock, cps)?;
    let delaytime = clip(&mut builder, delaytime, 0.0, max_delay_time)?;
    let log_feedback = builder.unary_op("log2", delayfeedback)?;
    let decay_time = builder.binary_op("/", sclang::log2(sclang::dbamp(-60.0)), log_feedback)?;
    let decay_time = builder.binary_op("*", decay_time, delaytime.clone())?;
    let decay_time = clip(&mut builder, decay_time, 0.0, 20.0)?;
    let delaytime = lag(&mut builder, delaytime, 1.0.into())?;
    let signal = builder.ugen(
        "CombL",
        "ar",
        &[
            ("in", signal.into()),
            ("maxdelaytime", max_delay_time.into()),
            ("delaytime", delaytime.into()),
            ("decaytime", decay_time.into()),
        ],
    )?;
    let signal = builder.ugen("LeakDC", "ar", &[("in", signal.into())])?;
    let delay_amp = lag(&mut builder, delay_amp, 0.01.into())?;
    let signal = builder.binary_op("*", signal, delay_amp)?;
    let env = asr(&mut builder, gate)?;
    let signal = builder.binary_op("*", signal, env)?;
    dirt_pause(&mut builder, signal.clone(), 4.0, 0.0)?;
    out(&mut builder, effect_bus, signal)?;
    builder.finish()
}

/// `dirt_delay<channels>` with the delay line of `SwitchDelay`.
fn switch_delay(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_delay{channels}"),
        vec![
            Arg::new("dryBus", 0.0).rate(Ir),
            Arg::new("effectBus", 0.0).rate(Ir),
            Arg::new("gate", 1.0),
            Arg::new("delaytime", 0.0),
            Arg::new("delayfeedback", 0.0),
            Arg::new("delaySend", 1.0),
            Arg::new("delayAmp", 1.0),
            Arg::new("lock", 0.0),
            Arg::new("cps", 1.0),
        ],
    )?;
    let [
        dry_bus,
        effect_bus,
        gate,
        delaytime,
        delayfeedback,
        delay_send,
        delay_amp,
        lock,
        cps,
    ] = arguments(&builder);
    let input = in_ar(&mut builder, &dry_bus, channels)?;
    let max_delay_time = 4.0;
    let delay_send = noisy_lag(&mut builder, delay_send)?;
    let input = builder.binary_op("*", input, delay_send)?;
    let delayfeedback = builder.binary_op("max", delayfeedback, 0.0)?;
    let delaytime = locked(&mut builder, delaytime, lock, cps)?;
    let delaytime = clip(&mut builder, delaytime, 0.0, max_delay_time)?;
    let signal = builder.ugen(
        "SwitchDelay",
        "ar",
        &[
            ("in", input.into()),
            ("drylevel", 1.0.into()),
            ("wetlevel", 1.0.into()),
            ("delaytime", delaytime.into()),
            ("delayfactor", delayfeedback.into()),
            ("maxdelaytime", max_delay_time.into()),
        ],
    )?;
    let env = asr(&mut builder, gate)?;
    let signal = builder.binary_op("*", signal, env)?;
    let delay_amp = lag(&mut builder, delay_amp, 0.01.into())?;
    let signal = builder.binary_op("*", signal, delay_amp)?;
    dirt_pause(&mut builder, signal.clone(), 4.0, 0.0)?;
    out(&mut builder, effect_bus, signal)?;
    builder.finish()
}

/// The definition `dirt_reverb<channels>`: the reverb of an orbit. It sums
/// `channels` channels of the bus `dryBus` at the level `room`, and spreads
/// the sum over `channels` channels through four allpass filters. A loop of
/// `LocalIn` and `LocalOut` adds them to its own output, which it filters,
/// delays and feeds back at a depth from 0.01 to 0.98 that follows `size`.
/// The delay times are random, drawn when the synth starts. The output of
/// the loop goes to the bus `effectBus` at the level `1 - dry`, under an
/// envelope that rises in 0.01 seconds, and falls in 1 second once `gate`
/// closes. Then the synth frees itself. It pauses itself as `DirtPause`
/// does, with a grace time of 4 seconds. The loop has at least the 2
/// channels of its two `DelayN`, so `dirt_reverb1` writes 2 channels. Its
/// `LocalOut` of 2 channels then writes nothing into its `LocalIn` of 1, so
/// the loop of `dirt_reverb1` has no feedback.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_reverb(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_reverb{channels}"),
        vec![
            Arg::new("dryBus", 0.0).rate(Ir),
            Arg::new("effectBus", 0.0).rate(Ir),
            Arg::new("gate", 1.0),
            Arg::new("room", 0.0),
            Arg::new("size", 0.1),
            Arg::new("dry", 0.0),
        ],
    )?;
    let [dry_bus, effect_bus, gate, room, size, dry] = arguments(&builder);
    let r#in = in_ar(&mut builder, &dry_bus, channels)?;
    let r#in = sum(&mut builder, as_array(r#in))?;
    let room = noisy_lag(&mut builder, room)?;
    let mut r#in = builder.binary_op("*", r#in, room)?;
    for _ in 0..4 {
        let delaytime = rands(&mut builder, 0.005, 0.02, channels)?;
        r#in = allpass_n(&mut builder, r#in, 0.03, delaytime, 1.0)?;
    }
    let depth = lag(&mut builder, size, 0.02.into())?;
    let depth = builder.send(
        depth,
        "linexp",
        &[
            ("inMin", 0.0.into()),
            ("inMax", 1.0.into()),
            ("outMin", 0.01.into()),
            ("outMax", 0.98.into()),
        ],
    )?;
    let r#loop = builder.ugen(
        "LocalIn",
        "ar",
        &[("numChannels", f64::from(channels).into())],
    )?;
    let mut depths = Vec::new();
    for _ in 0..channels {
        let jitter = rand(&mut builder, 0.0, 0.05)?;
        depths.push(builder.binary_op("+", depth.clone(), jitter)?);
    }
    let r#loop = builder.binary_op("*", r#loop, depths)?;
    let r#loop = builder.ugen(
        "OnePole",
        "ar",
        &[("in", r#loop.into()), ("coef", 0.5.into())],
    )?;
    let delaytime = rands(&mut builder, 0.01, 0.05, channels)?;
    let r#loop = allpass_n(&mut builder, r#loop, 0.05, delaytime, 2.0)?;
    let jitter = rands(&mut builder, -0.003, 0.003, 2)?;
    let delaytime = builder.binary_op("+", vec![0.19.into(), 0.26.into()], jitter)?;
    let r#loop = builder.ugen(
        "DelayN",
        "ar",
        &[
            ("in", r#loop.into()),
            ("maxdelaytime", 0.3.into()),
            ("delaytime", delaytime.into()),
        ],
    )?;
    let delaytime = rands(&mut builder, 0.03, 0.15, channels)?;
    let r#loop = allpass_n(&mut builder, r#loop, 0.05, delaytime, 2.0)?;
    let r#loop = builder.binary_op("+", r#loop, r#in)?;
    let r#loop = builder.ugen("LeakDC", "ar", &[("in", r#loop.into())])?;
    builder.ugen(
        "LocalOut",
        "ar",
        &[("channelsArray", r#loop.clone().into())],
    )?;
    let snd = r#loop;
    let wet = builder.binary_op("-", 1.0, dry)?;
    let wet = noisy_lag(&mut builder, wet)?;
    let snd = builder.binary_op("*", snd, wet)?;
    dirt_pause(&mut builder, snd.clone(), 4.0, 0.0)?;
    let env = asr(&mut builder, gate)?;
    let snd = builder.binary_op("*", snd, env)?;
    out(&mut builder, effect_bus, snd)?;
    builder.finish()
}

/// The definition `dirt_leslie<channels>`: the rotary speaker of an orbit.
/// It leaves `channels` channels of the bus `dryBus` in their place at the
/// level `1 - leslie`. The part of the sound above 800 hertz goes through a
/// delay of the distance of a horn that turns at `lrate` hertz in a cabinet
/// of `lsize` meters, and the rest throbs at 0.8 times that rate. Both parts
/// swing between two sides, and go to the bus `effectBus` at the level
/// `leslie`, times 1.2, under an envelope that rises in 0.01 seconds, and
/// falls in 1 second once `gate` closes. Then the synth frees itself. It
/// pauses itself as `DirtPause` does, with a grace time of 4 seconds. On 1
/// or 2 channels, each channel turns on its own and gets the sum of its two
/// sides. On more channels, the pairs of channels add up to one pair, which
/// turns, and each of its sides circles the channels at `lrate` hertz.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_leslie(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_leslie{channels}"),
        vec![
            Arg::new("dryBus", 0.0).rate(Ir),
            Arg::new("effectBus", 0.0).rate(Ir),
            Arg::new("gate", 1.0),
            Arg::new("leslie", 0.5),
            Arg::new("lrate", 6.7),
            Arg::new("lsize", 0.3),
        ],
    )?;
    let [dry_bus, effect_bus, gate, leslie, lrate, lsize] = arguments(&builder);
    let r#in = in_ar(&mut builder, &dry_bus, channels)?;
    let level = builder.binary_op("-", 1.0, leslie.clone())?;
    let dry = builder.binary_op("*", r#in.clone(), level)?;
    builder.ugen(
        "ReplaceOut",
        "ar",
        &[("bus", dry_bus.into()), ("channelsArray", dry.into())],
    )?;
    let r#in = if channels > 2 {
        let pairs = as_array(r#in)
            .chunks(2)
            .map(|pair| pair.to_vec().into())
            .collect();
        sum(&mut builder, pairs)?
    } else {
        vec![r#in.clone(), r#in].into()
    };
    let sine = lagged_sine(&mut builder, "ar", lrate.clone(), 10.0, 0.0)?;
    let distance = range(&mut builder, sine, 0.0.into(), lsize.clone())?;
    let rate = builder.binary_op("*", lrate.clone(), 0.8)?;
    let sine = lagged_sine(&mut builder, "ar", rate, 20.0, 0.0)?;
    let depth = builder.binary_op("*", lsize, 1.3)?;
    let throb = range(&mut builder, sine, 0.0.into(), depth)?;
    let rate = builder.binary_op("*", lrate.clone(), 0.8)?;
    let sine = lagged_sine(&mut builder, "kr", rate, 4.0, PI / 2.0)?;
    let bal1 = range(&mut builder, sine, (-0.2).into(), 0.2.into())?;
    let sine = lagged_sine(&mut builder, "kr", lrate.clone(), 2.0, PI / 2.0)?;
    let bal2 = range(&mut builder, sine, (-0.4).into(), 0.4.into())?;
    let sound2 = builder.ugen(
        "HPF",
        "ar",
        &[("in", r#in.clone().into()), ("freq", 800.0.into())],
    )?;
    let sound1 = builder.binary_op("-", r#in, sound2.clone())?;
    let level = builder.binary_op("-", 1.0, throb)?;
    let level = builder.binary_op("*", leslie.clone(), level)?;
    let sound1 = builder.binary_op("*", level, sound1)?;
    let sound1 = balance2(&mut builder, sound1, bal1)?;
    let delaytime = builder.binary_op("/", distance.clone(), 343.0)?;
    let sound2 = builder.ugen(
        "DelayC",
        "ar",
        &[
            ("in", sound2.into()),
            ("maxdelaytime", 1.0.into()),
            ("delaytime", delaytime.into()),
        ],
    )?;
    let level = builder.binary_op("-", 1.0, distance)?;
    let level = builder.binary_op("*", leslie, level)?;
    let sound2 = builder.binary_op("*", level, sound2)?;
    let sound2 = balance2(&mut builder, sound2, bal2)?;
    let snd = builder.binary_op("+", sound1, sound2)?;
    let snd = builder.binary_op("*", 1.2, snd)?;
    dirt_pause(&mut builder, snd.clone(), 4.0, 0.0)?;
    let env = asr(&mut builder, gate)?;
    let snd = builder.binary_op("*", snd, env)?;
    let snd = if channels == 1 {
        sum(&mut builder, as_array(snd))?
    } else if channels > 2 {
        let snd = as_array(snd);
        let pos = builder.ugen("LFSaw", "kr", &[("freq", lrate.clone().into())])?;
        let first = pan_az(&mut builder, channels, snd[0].clone(), pos)?;
        let pos = builder.ugen("LFSaw", "kr", &[("freq", lrate.into())])?;
        let pos = builder.binary_op("+", pos, 1.0)?;
        let second = pan_az(&mut builder, channels, snd[1].clone(), pos)?;
        sum(&mut builder, vec![first, second])?
    } else {
        snd
    };
    out(&mut builder, effect_bus, snd)?;
    builder.finish()
}

/// `Out.ar(bus, signal)`: `signal` added to the channels of the bus `bus`.
fn out(
    builder: &mut SynthDefBuilder,
    bus: UGenInput,
    signal: UGenInput,
) -> Result<UGenInput, BuildError> {
    builder.ugen(
        "Out",
        "ar",
        &[("bus", bus.into()), ("channelsArray", signal.into())],
    )
}

/// `EnvGen.kr(Env.asr, gate, doneAction: 2)`: an envelope that rises to 1 in
/// 0.01 seconds, and falls to 0 in 1 second once `gate` closes. Then it
/// frees the synth.
fn asr(builder: &mut SynthDefBuilder, gate: UGenInput) -> Result<UGenInput, BuildError> {
    let envelope = builder.env("asr", &[])?;
    builder.ugen(
        "EnvGen",
        "kr",
        &[
            ("envelope", envelope.into()),
            ("gate", gate.into()),
            ("doneAction", 2.0.into()),
        ],
    )
}

/// `SendPeakRMS.kr(sig, replyRate, peakLag, cmdName, replyID)`: a UGen
/// without outputs that sends the peak and the power of each channel of
/// `sig` to the clients of the server, as `cmd_name` replies with the ID
/// `reply_id`.
fn send_peak_rms(
    builder: &mut SynthDefBuilder,
    sig: UGenInput,
    reply_rate: UGenInput,
    peak_lag: UGenInput,
    cmd_name: &str,
    reply_id: UGenInput,
) -> Result<UGenInput, BuildError> {
    let sig = as_array(sig);
    // Char:ascii gives a byte from -128 to 127.
    let ascii: Vec<UGenInput> = cmd_name
        .bytes()
        .map(|byte| f64::from(i8::from_ne_bytes([byte])).into())
        .collect();
    let mut inputs = vec![reply_rate, peak_lag, reply_id, number(sig.len()).into()];
    inputs.extend(sig);
    inputs.push(number(ascii.len()).into());
    inputs.extend(ascii);
    // ugen refuses SendPeakRMS.kr, whose inputs follow a count: the inputs
    // are those of SendPeakRMS.new1.
    builder.add(
        NewUGen::new("SendPeakRMS", Rate::Control)
            .inputs(inputs)
            .outputs([]),
    )
}

/// `delaytime * if(lock, reciprocal(cps), 1)`: `delaytime` in cycles of
/// `cps` when `lock` is 1, and in seconds when it is 0.
fn locked(
    builder: &mut SynthDefBuilder,
    delaytime: UGenInput,
    lock: UGenInput,
    cps: UGenInput,
) -> Result<UGenInput, BuildError> {
    let cycle = builder.unary_op("reciprocal", cps)?;
    let scale = builder.send(
        lock,
        "if",
        &[("trueUGen", cycle.into()), ("falseUGen", 1.0.into())],
    )?;
    builder.binary_op("*", delaytime, scale)
}

/// `input.lag(LFNoise1.kr(1).range(0.01, 0.02))`: `input` through a `Lag`
/// whose time wanders between 0.01 and 0.02 seconds.
fn noisy_lag(builder: &mut SynthDefBuilder, input: UGenInput) -> Result<UGenInput, BuildError> {
    let noise = builder.ugen("LFNoise1", "kr", &[("freq", 1.0.into())])?;
    let time = range(builder, noise, 0.01.into(), 0.02.into())?;
    lag(builder, input, time)
}

/// `input.lag(t1)`: `input` through a `Lag` of `t1` seconds.
fn lag(
    builder: &mut SynthDefBuilder,
    input: UGenInput,
    t1: UGenInput,
) -> Result<UGenInput, BuildError> {
    builder.send(input, "lag", &[("t1", t1.into())])
}

/// `input.clip(lo, hi)`.
fn clip(
    builder: &mut SynthDefBuilder,
    input: UGenInput,
    lo: f64,
    hi: f64,
) -> Result<UGenInput, BuildError> {
    builder.send(input, "clip", &[("lo", lo.into()), ("hi", hi.into())])
}

/// `input.range(lo, hi)`: the map of `input` from its signal range to the
/// range from `lo` to `hi`.
fn range(
    builder: &mut SynthDefBuilder,
    input: UGenInput,
    lo: UGenInput,
    hi: UGenInput,
) -> Result<UGenInput, BuildError> {
    builder.send(input, "range", &[("lo", lo.into()), ("hi", hi.into())])
}

/// `Rand(lo, hi)`: a number from `lo` to `hi`, drawn when the synth starts.
fn rand(builder: &mut SynthDefBuilder, lo: f64, hi: f64) -> Result<UGenInput, BuildError> {
    builder.ugen("Rand", "new", &[("lo", lo.into()), ("hi", hi.into())])
}

/// `{ Rand(lo, hi) }.dup(n)`: `n` numbers from `lo` to `hi`, each drawn by a
/// `Rand` of its own when the synth starts.
fn rands(builder: &mut SynthDefBuilder, lo: f64, hi: f64, n: u32) -> Result<UGenInput, BuildError> {
    let rands = (0..n)
        .map(|_| rand(builder, lo, hi))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rands.into())
}

/// `AllpassN.ar(in, maxdelaytime, delaytime, decaytime)`: an allpass filter
/// of `in` with a delay of `delaytime` seconds, at most `maxdelaytime`, and
/// a decay of `decaytime` seconds.
#[allow(clippy::similar_names, reason = "the arguments of AllpassN.ar")]
fn allpass_n(
    builder: &mut SynthDefBuilder,
    r#in: UGenInput,
    maxdelaytime: f64,
    delaytime: UGenInput,
    decaytime: f64,
) -> Result<UGenInput, BuildError> {
    builder.ugen(
        "AllpassN",
        "ar",
        &[
            ("in", r#in.into()),
            ("maxdelaytime", maxdelaytime.into()),
            ("delaytime", delaytime.into()),
            ("decaytime", decaytime.into()),
        ],
    )
}

/// `SinOsc.<selector>(Lag.kr(freq, lagTime), phase)`: a sine at the rate of
/// `selector`, whose frequency follows `freq` through a `Lag` of `lag_time`
/// seconds.
fn lagged_sine(
    builder: &mut SynthDefBuilder,
    selector: &str,
    freq: UGenInput,
    lag_time: f64,
    phase: f64,
) -> Result<UGenInput, BuildError> {
    let freq = builder.ugen(
        "Lag",
        "kr",
        &[("in", freq.into()), ("lagTime", lag_time.into())],
    )?;
    builder.ugen(
        "SinOsc",
        selector,
        &[("freq", freq.into()), ("phase", phase.into())],
    )
}

/// `Balance2.ar(sound[0], sound[1], pos)`: the first two elements of `sound`
/// as the left and the right side, balanced by `pos`.
fn balance2(
    builder: &mut SynthDefBuilder,
    sound: UGenInput,
    pos: UGenInput,
) -> Result<UGenInput, BuildError> {
    let sound = as_array(sound);
    builder.ugen(
        "Balance2",
        "ar",
        &[
            ("left", sound[0].clone().into()),
            ("right", sound[1].clone().into()),
            ("pos", pos.into()),
        ],
    )
}

/// `PanAz.ar(numChannels, in, pos)`: `in` panned at `pos` over `channels`
/// channels in a circle.
fn pan_az(
    builder: &mut SynthDefBuilder,
    channels: u32,
    r#in: UGenInput,
    pos: UGenInput,
) -> Result<UGenInput, BuildError> {
    builder.ugen(
        "PanAz",
        "ar",
        &[
            ("numChans", f64::from(channels).into()),
            ("in", r#in.into()),
            ("pos", pos.into()),
        ],
    )
}
