//! The effects of SuperDirt on one sound: each definition changes `channels`
//! channels of the bus `out`, in their place.

use std::f64::consts::PI;

use scsynth_synthdef::ArgRate::{Ir, Kr};
use scsynth_synthdef::{Arg, BuildError, SynthDef, SynthDefBuilder, UGenInput};

use super::{arguments, as_array, sum};

/// The definition `dirt_vowel<channels>`: a vowel. Five band-pass filters
/// at the formants of `vowelFreqs`, `vowelRqs` and `vowelAmps` shape
/// `channels` channels of the bus `out`, in their place. `cutoff`, from 0 to
/// 1, raises the frequencies by up to an octave, to at most half the sample
/// rate. `resonance`, from 0 to 1, raises the levels, narrows the bands, and
/// crossfades from mostly the dry sound to the filtered one. The crossfade
/// has the level 0.25.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_vowel(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_vowel{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("resonance", 0.5),
            Arg::new("cutoff", 0.0),
        ],
    )?;
    let [out, resonance, cutoff] = arguments(&builder);
    let signal = in_ar(&mut builder, &out, channels)?;
    let vowel_freqs = formant_control(&mut builder, "vowelFreqs", 1000.0)?;
    let cutoff = map(&mut builder, cutoff, "linexp", [0.0, 1.0, 1.0, 2.0])?;
    let vowel_freqs = builder.binary_op("*", vowel_freqs, cutoff)?;
    let vowel_amps = formant_control(&mut builder, "vowelAmps", 0.0)?;
    let gain = map(
        &mut builder,
        resonance.clone(),
        "linlin",
        [0.0, 1.0, 50.0, 200.0],
    )?;
    let vowel_amps = builder.binary_op("*", vowel_amps, gain)?;
    let vowel_rqs = formant_control(&mut builder, "vowelRqs", 0.0)?;
    let width = map(
        &mut builder,
        resonance.clone(),
        "linlin",
        [0.0, 1.0, 1.0 / 10.0, 1.0 / 2.0],
    )?;
    let vowel_rqs = builder.binary_op("/", vowel_rqs, width)?;
    let vowel_freqs = builder.send(vowel_freqs, "minNyquist", &[])?;
    let voweled = builder.ugen(
        "BPF",
        "ar",
        &[
            ("in", vec![signal.clone()].into()),
            ("freq", vowel_freqs.into()),
            ("rq", vowel_rqs.into()),
            ("mul", vowel_amps.into()),
        ],
    )?;
    let voweled = sum(&mut builder, as_array(voweled))?;
    let pan = map(&mut builder, resonance, "linlin", [0.0, 1.0, -0.5, 1.0])?;
    let signal = builder.ugen(
        "XFade2",
        "ar",
        &[
            ("inA", signal.into()),
            ("inB", voweled.into()),
            ("pan", pan.into()),
            ("level", 0.25.into()),
        ],
    )?;
    replace_out(&mut builder, out, signal)?;
    builder.finish()
}

/// The definition `dirt_crush<channels>`: a bit crusher. It rounds
/// `channels` channels of the bus `out` to multiples of
/// `0.5 ** (crush - 1)`, in their place.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_crush(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_crush{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("crush", 4.0).rate(Kr),
        ],
    )?;
    let [out, crush] = arguments(&builder);
    let signal = in_ar(&mut builder, &out, channels)?;
    let crush = builder.binary_op("-", crush, 1.0)?;
    let step = builder.binary_op("pow", 0.5, crush)?;
    let signal = builder.binary_op("round", signal, step)?;
    replace_out(&mut builder, out, signal)?;
    builder.finish()
}

/// The definition `dirt_coarse<channels>`: a reducer of the sample rate. It
/// samples `channels` channels of the bus `out` once every `coarse` samples,
/// and holds each value in their place. Its control `bandq` has no effect.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_coarse(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_coarse{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("coarse", 0.0).rate(Kr),
            Arg::new("bandq", 10.0).rate(Kr),
        ],
    )?;
    let [out, coarse, _bandq] = arguments(&builder);
    let signal = in_ar(&mut builder, &out, channels)?;
    let rate = builder.ugen("SampleRate", "ir", &[])?;
    let rate = builder.binary_op("/", rate, coarse)?;
    let trig = builder.ugen("Impulse", "ar", &[("freq", rate.into())])?;
    let signal = builder.ugen(
        "Latch",
        "ar",
        &[("in", signal.into()), ("trig", trig.into())],
    )?;
    replace_out(&mut builder, out, signal)?;
    builder.finish()
}

/// The definition `dirt_hpf<channels>`: a resonant high-pass filter of
/// `channels` channels of the bus `out`, in their place. Its cutoff is the
/// magnitude of `hcutoff`, from 20 hertz to half the sample rate.
/// `hresonance`, from 0 to 1, takes its reciprocal Q from 1 to 0.001.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_hpf(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_hpf{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("hcutoff", 440.0).rate(Kr),
            Arg::new("hresonance", 0.0).rate(Kr),
        ],
    )?;
    let [out, hcutoff, hresonance] = arguments(&builder);
    let signal = in_ar(&mut builder, &out, channels)?;
    let hcutoff = builder.unary_op("abs", hcutoff)?;
    let nyquist = builder.ugen("SampleRate", "ir", &[])?;
    let nyquist = builder.binary_op("/", nyquist, 2.0)?;
    let hcutoff = builder.send(
        hcutoff,
        "clip",
        &[("lo", 20.0.into()), ("hi", nyquist.into())],
    )?;
    let hresonance = map(&mut builder, hresonance, "linexp", [0.0, 1.0, 1.0, 0.001])?;
    let signal = builder.ugen(
        "RHPF",
        "ar",
        &[
            ("in", signal.into()),
            ("freq", hcutoff.into()),
            ("rq", hresonance.into()),
        ],
    )?;
    replace_out(&mut builder, out, signal)?;
    builder.finish()
}

/// The definition `dirt_bpf<channels>`: a band-pass filter of `channels`
/// channels of the bus `out`, in their place. Its center is `bandqf`, from
/// 20 hertz to half the sample rate, and its Q and its gain are `bandq`, at
/// least 1.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_bpf(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_bpf{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("bandqf", 440.0).rate(Kr),
            Arg::new("bandq", 10.0).rate(Kr),
        ],
    )?;
    let [out, bandqf, bandq] = arguments(&builder);
    let signal = in_ar(&mut builder, &out, channels)?;
    let bandq = builder.binary_op("max", 1.0, bandq)?;
    let nyquist = builder.ugen("SampleRate", "ir", &[])?;
    let nyquist = builder.binary_op("/", nyquist, 2.0)?;
    let bandqf = builder.send(
        bandqf,
        "clip",
        &[("lo", 20.0.into()), ("hi", nyquist.into())],
    )?;
    let rq = builder.binary_op("/", 1.0, bandq.clone())?;
    let signal = builder.ugen(
        "BPF",
        "ar",
        &[
            ("in", signal.into()),
            ("freq", bandqf.into()),
            ("rq", rq.into()),
        ],
    )?;
    let gain = builder.binary_op("max", bandq, 1.0)?;
    let signal = builder.binary_op("*", signal, gain)?;
    replace_out(&mut builder, out, signal)?;
    builder.finish()
}

/// The definition `dirt_shape<channels>`: a wave shaper. It distorts
/// `channels` channels of the bus `out`, in their place, more as `shape`
/// goes from 0 to 1.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_shape(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_shape{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("shape", 0.0).rate(Kr),
        ],
    )?;
    let [out, shape] = arguments(&builder);
    let signal = in_ar(&mut builder, &out, channels)?;
    let amp = 1.0;
    let shape = builder.binary_op("min", shape, 1.0 - 4e-10)?;
    let numerator = builder.binary_op("*", 2.0, shape.clone())?;
    let denominator = builder.binary_op("-", 1.0, shape)?;
    let shape = builder.binary_op("/", numerator, denominator)?;
    let numerator = builder.binary_op("+", 1.0, shape.clone())?;
    let numerator = builder.binary_op("*", numerator, signal.clone())?;
    let denominator = builder.unary_op("abs", signal)?;
    let denominator = builder.binary_op("*", shape, denominator)?;
    let denominator = builder.binary_op("+", 1.0, denominator)?;
    let signal = builder.binary_op("/", numerator, denominator)?;
    let signal = builder.binary_op("*", signal, amp)?;
    replace_out(&mut builder, out, signal)?;
    builder.finish()
}

/// The definition `dirt_lpf<channels>`: a resonant low-pass filter of
/// `channels` channels of the bus `out`, in their place. Its cutoff is the
/// magnitude of `cutoff`, from 20 hertz to half the sample rate.
/// `resonance`, from 0 to 1, takes its reciprocal Q from 1 to 0.001.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_lpf(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_lpf{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("cutoff", 440.0).rate(Kr),
            Arg::new("resonance", 0.0).rate(Kr),
        ],
    )?;
    let [out, cutoff, resonance] = arguments(&builder);
    let signal = in_ar(&mut builder, &out, channels)?;
    let cutoff = builder.unary_op("abs", cutoff)?;
    let nyquist = builder.ugen("SampleRate", "ir", &[])?;
    let nyquist = builder.binary_op("/", nyquist, 2.0)?;
    let cutoff = builder.send(
        cutoff,
        "clip",
        &[("lo", 20.0.into()), ("hi", nyquist.into())],
    )?;
    let resonance = map(&mut builder, resonance, "linexp", [0.0, 1.0, 1.0, 0.001])?;
    let signal = builder.ugen(
        "RLPF",
        "ar",
        &[
            ("in", signal.into()),
            ("freq", cutoff.into()),
            ("rq", resonance.into()),
        ],
    )?;
    replace_out(&mut builder, out, signal)?;
    builder.finish()
}

/// The definition `dirt_envelope<channels>`: an envelope on `channels`
/// channels of the bus `out`, in their place. It rises to 1 in `attack`,
/// holds for `hold` and falls to 0 in `release`, on segments of the
/// curvature `curve`. The default `release` is infinite.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_envelope(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_envelope{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("attack", 0.0).rate(Ir),
            Arg::new("hold", 0.0).rate(Ir),
            Arg::new("release", f64::INFINITY).rate(Ir),
            Arg::new("curve", -3.0).rate(Ir),
        ],
    )?;
    let [out, attack, hold, release, curve] = arguments(&builder);
    let signal = in_ar(&mut builder, &out, channels)?;
    let envelope = builder.env(
        "linen",
        &[
            ("attackTime", attack.into()),
            ("sustainTime", hold.into()),
            ("releaseTime", release.into()),
            ("level", 1.0.into()),
            ("curve", curve.into()),
        ],
    )?;
    let env = builder.ugen("EnvGen", "ar", &[("envelope", envelope.into())])?;
    let signal = builder.binary_op("*", signal, env)?;
    replace_out(&mut builder, out, signal)?;
    builder.finish()
}

/// The definition `dirt_grain_envelope<channels>`: an envelope of the length
/// `sustain` on `channels` channels of the bus `out`, in their place.
/// `plat`, from 0 to 1, is the part of its plateau, and `tilt`, from 0 to 1,
/// the part of the rest that rises. Its segments have the curvature `curve`.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_grain_envelope(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_grain_envelope{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("sustain", 1.0).rate(Ir),
            Arg::new("tilt", 0.5).rate(Ir),
            Arg::new("plat", 0.0).rate(Ir),
            Arg::new("curve", -3.0).rate(Ir),
        ],
    )?;
    let [out, sustain, tilt, plat, curve] = arguments(&builder);
    let signal = in_ar(&mut builder, &out, channels)?;
    let p = builder.send(plat, "clip", &[("lo", 0.0.into()), ("hi", 1.0.into())])?;
    let c = builder.send(tilt, "clip", &[("lo", 0.0.into()), ("hi", 1.0.into())])?;
    let rise_and_fall = builder.binary_op("-", 1.0, p.clone())?;
    let attack = builder.binary_op("*", c.clone(), rise_and_fall.clone())?;
    let release = builder.binary_op("-", 1.0, c)?;
    let release = builder.binary_op("*", release, rise_and_fall)?;
    let hold = p;
    let envelope = builder.env(
        "linen",
        &[
            ("attackTime", attack.into()),
            ("sustainTime", hold.into()),
            ("releaseTime", release.into()),
            ("level", 1.0.into()),
            ("curve", curve.into()),
        ],
    )?;
    let env = builder.ugen(
        "EnvGen",
        "ar",
        &[("envelope", envelope.into()), ("timeScale", sustain.into())],
    )?;
    let signal = builder.binary_op("*", signal, env)?;
    replace_out(&mut builder, out, signal)?;
    builder.finish()
}

/// The definition `dirt_pitch_shift<channels>`: a pitch shifter of
/// `channels` channels of the bus `out`, in their place, by the ratio
/// `psrate` and with the time dispersion `psdisp`. Its window grows linearly
/// with `sustain`, from 0.01 seconds at a `sustain` of 0.01 seconds or less
/// to 0.18 seconds at 0.3 seconds or more.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_pitch_shift(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_pitch_shift{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("psrate", 1.0).rate(Ir),
            Arg::new("sustain", 1.0).rate(Ir),
            Arg::new("psdisp", 0.0),
        ],
    )?;
    let [out, psrate, sustain, psdisp] = arguments(&builder);
    let signal = in_ar(&mut builder, &out, channels)?;
    let window_size = map(&mut builder, sustain, "linlin", [0.01, 0.3, 0.01, 0.18])?;
    let signal = builder.ugen(
        "PitchShift",
        "ar",
        &[
            ("in", signal.into()),
            ("windowSize", window_size.into()),
            ("pitchRatio", psrate.into()),
            ("pitchDispersion", 0.0.into()),
            ("timeDispersion", psdisp.into()),
        ],
    )?;
    replace_out(&mut builder, out, signal)?;
    builder.finish()
}

/// The definition `dirt_tremolo<channels>`: a tremolo on `channels` channels
/// of the bus `out`, in their place. A sine at `tremolorate` hertz swings
/// their level between 1 and 1 minus `tremolodepth`, from 0 to 1.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_tremolo(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_tremolo{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("tremolorate", 1.0).rate(Kr),
            Arg::new("tremolodepth", 0.5).rate(Kr),
        ],
    )?;
    let [out, tremolorate, tremolodepth] = arguments(&builder);
    let tremolodepth = builder.send(
        tremolodepth,
        "clip",
        &[("lo", 0.0.into()), ("hi", 1.0.into())],
    )?;
    let tremolodepth = builder.binary_op("*", tremolodepth, 0.5)?;
    let tremolowave = builder.binary_op("-", 1.0, tremolodepth.clone())?;
    let sine = builder.ugen(
        "SinOsc",
        "ar",
        &[
            ("freq", tremolorate.into()),
            ("phase", (0.5 * PI).into()),
            ("mul", tremolodepth.into()),
        ],
    )?;
    let tremolowave = builder.binary_op("+", tremolowave, sine)?;
    let signal = in_ar(&mut builder, &out, channels)?;
    let signal = builder.binary_op("*", signal, tremolowave)?;
    replace_out(&mut builder, out, signal)?;
    builder.finish()
}

/// The definition `dirt_phaser<channels>`: a phaser on `channels` channels
/// of the bus `out`, in their place. It adds to them their `AllpassL`, at the
/// level `phaserdepth` from 0 to 1, with a delay that swings from 8.8 to 10
/// milliseconds at `phaserrate` hertz. `sustain` is the longest delay.
///
/// # Errors
///
/// [`BuildError::BadCount`] for 0 channels, or for more than 65,535.
pub fn dirt_phaser(channels: u32) -> Result<SynthDef, BuildError> {
    let mut builder = SynthDefBuilder::new(
        format!("dirt_phaser{channels}"),
        vec![
            Arg::new("out", 0.0).rate(Ir),
            Arg::new("sustain", 4.0).rate(Ir),
            Arg::new("phaserrate", 1.0).rate(Kr),
            Arg::new("phaserdepth", 0.5).rate(Kr),
        ],
    )?;
    let [out, sustain, phaserrate, phaserdepth] = arguments(&builder);
    let phase = builder.ugen("LFPar", "kr", &[("freq", phaserrate.into())])?;
    let phase = builder.send(
        phase,
        "range",
        &[("lo", 0.0088.into()), ("hi", 0.01.into())],
    )?;
    let phaserdepth = builder.send(
        phaserdepth,
        "clip",
        &[("lo", 0.0.into()), ("hi", 1.0.into())],
    )?;
    let signal = in_ar(&mut builder, &out, channels)?;
    let phaserwave = builder.ugen(
        "AllpassL",
        "ar",
        &[
            ("in", signal.clone().into()),
            ("maxdelaytime", sustain.into()),
            ("delaytime", phase.into()),
            ("decaytime", 0.0.into()),
            ("mul", phaserdepth.into()),
        ],
    )?;
    let signal = builder.binary_op("+", signal, phaserwave)?;
    replace_out(&mut builder, out, signal)?;
    builder.finish()
}

/// `In.ar(out, numChannels)`: `channels` channels of the bus `out`.
fn in_ar(
    builder: &mut SynthDefBuilder,
    out: &UGenInput,
    channels: u32,
) -> Result<UGenInput, BuildError> {
    builder.ugen(
        "In",
        "ar",
        &[
            ("bus", out.clone().into()),
            ("numChannels", f64::from(channels).into()),
        ],
    )
}

/// `ReplaceOut.ar(out, signal)`: `signal` in the place of the channels of
/// the bus `out`.
fn replace_out(
    builder: &mut SynthDefBuilder,
    out: UGenInput,
    signal: UGenInput,
) -> Result<UGenInput, BuildError> {
    builder.ugen(
        "ReplaceOut",
        "ar",
        &[("bus", out.into()), ("channelsArray", signal.into())],
    )
}

/// `\name.ir(value ! 5)`: the named control `name` of five values at scalar
/// rate, each with the default `value`.
fn formant_control(
    builder: &mut SynthDefBuilder,
    name: &str,
    value: f64,
) -> Result<UGenInput, BuildError> {
    builder.ugen(
        "NamedControl",
        "ir",
        &[
            ("name", name.into()),
            ("values", vec![value.into(); 5].into()),
        ],
    )
}

/// `input.linlin(inMin, inMax, outMin, outMax)` or `input.linexp(…)`, by
/// `selector`: the map of `input` from the range of `inMin` and `inMax` to
/// the range of `outMin` and `outMax`, which clips `input` first.
fn map(
    builder: &mut SynthDefBuilder,
    input: UGenInput,
    selector: &str,
    [in_min, in_max, out_min, out_max]: [f64; 4],
) -> Result<UGenInput, BuildError> {
    builder.send(
        input,
        selector,
        &[
            ("inMin", in_min.into()),
            ("inMax", in_max.into()),
            ("outMin", out_min.into()),
            ("outMax", out_max.into()),
        ],
    )
}
