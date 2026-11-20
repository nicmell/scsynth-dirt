//! The port of `DirtEvent` of SuperDirt for a sample sound: from a sound to
//! its bundles, with the live sounds that a cut reaches, and the reasons of
//! a drop.

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

use scsynth_osc::command::group::{GNew, NewGroup};
use scsynth_osc::command::node::{NRun, NSet, NodeRun};
use scsynth_osc::command::synth::SNew;
use scsynth_osc::command::top_level::ErrorMode;
use scsynth_osc::{
    AddAction, ArrayElement, BufferId, Bundle, Command, ControlSetting, ControlValue, NewNodeId,
    NodeId, Packet, Timetag,
};

use crate::config::{Config, int};
use crate::library::{Library, SampleInfo};
use crate::orbit::Orbit;
use crate::sclang;
use crate::sound::{Sound, Value};
use crate::vowel;

/// The most frames of a buffer that `dirt_sample` plays. A longer buffer
/// plays with `dirt_sample_long`, as `instrumentForBuffer` of
/// `DirtSoundLibrary` picks it.
const SHORT_FRAMES: u32 = 16_777_216;

/// The most channels of a sample: [`synthdefs::core`](crate::synthdefs::core)
/// holds the players of samples of 1 and 2 channels.
const SAMPLE_CHANNELS: u32 = 2;

/// 2 ** 32: the seconds of an era of OSC time tags, and the units of a
/// second in a time tag.
const TWO_POW_32: f64 = 4_294_967_296.0;

/// A global synth of an orbit that `set` of `GlobalDirtEffect` reaches.
struct GlobalEffect {
    /// The node of the synth on the orbit.
    synth: fn(&Orbit) -> NodeId,
    /// The parameters of the synth, in their order.
    parameters: &'static [&'static str],
    /// Whether the synth resumes for every sound: `alwaysRun` of
    /// `GlobalDirtEffect`.
    always_run: bool,
}

/// The global effects of `DirtOrbit.initDefaultGlobalEffects`, in their
/// order, but `dirt_rms`, which is not active and sends nothing.
const GLOBAL_EFFECTS: [GlobalEffect; 4] = [
    GlobalEffect {
        synth: Orbit::delay,
        parameters: &[
            "delaytime",
            "delayfeedback",
            "delaySend",
            "delayAmp",
            "lock",
            "cps",
        ],
        always_run: false,
    },
    GlobalEffect {
        synth: Orbit::reverb,
        parameters: &["size", "room", "dry"],
        always_run: false,
    },
    GlobalEffect {
        synth: Orbit::leslie,
        parameters: &["leslie", "lrate", "lsize"],
        always_run: false,
    },
    GlobalEffect {
        synth: Orbit::monitor,
        parameters: &["limitertype"],
        always_run: true,
    },
];

/// The test of a module: whether its synth plays for a sound.
type Test = fn(&Parameters<'_>) -> Result<bool, Dropped>;

/// A module of `core-modules.scd` after `sound` and `vowel`.
struct Module {
    /// The name of the definition, without the channel count.
    definition: &'static str,
    /// The test of the module.
    test: Test,
    /// The controls of the synth, in their order.
    controls: &'static [&'static str],
}

/// The modules of `core-modules.scd` after `sound` and `vowel`, in their
/// order.
const MODULES: [Module; 11] = [
    Module {
        definition: "dirt_shape",
        test: |parameters| Ok(parameters.has("shape")),
        controls: &["shape", "out"],
    },
    Module {
        definition: "dirt_hpf",
        test: |parameters| Ok(parameters.has("hcutoff")),
        controls: &["hcutoff", "hresonance", "out"],
    },
    Module {
        definition: "dirt_bpf",
        test: |parameters| Ok(parameters.has("bandf")),
        controls: &["bandqf", "bandq", "out"],
    },
    Module {
        definition: "dirt_crush",
        test: |parameters| Ok(parameters.has("crush")),
        controls: &["crush", "out"],
    },
    Module {
        definition: "dirt_coarse",
        test: |parameters| {
            Ok(parameters
                .optional("coarse")?
                .is_some_and(|coarse| coarse > 1.0))
        },
        controls: &["coarse", "out"],
    },
    Module {
        definition: "dirt_lpf",
        test: |parameters| Ok(parameters.has("cutoff")),
        controls: &["cutoff", "resonance", "out"],
    },
    Module {
        definition: "dirt_pitch_shift",
        test: |parameters| Ok(parameters.has("psrate")),
        controls: &["psrate", "psdisp", "sustain", "out"],
    },
    Module {
        definition: "dirt_envelope",
        test: |parameters| Ok(parameters.has("attack") || parameters.has("release")),
        controls: &["attack", "hold", "release", "curve", "out"],
    },
    Module {
        definition: "dirt_grain_envelope",
        test: |parameters| Ok(parameters.has("tilt")),
        controls: &["sustain", "tilt", "plat", "curve", "out"],
    },
    Module {
        definition: "dirt_tremolo",
        test: |parameters| Ok(parameters.has("tremolorate")),
        controls: &["tremolorate", "tremolodepth", "out"],
    },
    Module {
        definition: "dirt_phaser",
        test: |parameters| Ok(parameters.has("phaserrate")),
        controls: &["phaserrate", "phaserdepth", "out"],
    },
];

/// Why [`Dirt::play`](crate::Dirt::play) drops a sound. A sound that drops
/// changes nothing in the instance.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Dropped {
    /// The name of the sound is empty: the sound is silence.
    Silence,
    /// The sound has neither `s` nor `sound`, so it has no name.
    NoName,
    /// The library has no samples of the sound of this name.
    UnknownSound(String),
    /// The sample of the sound of this name is not loaded: the library lacks
    /// its `/b_info`, or scsynth could not read its file.
    NotLoaded(String),
    /// The sample has this many channels, but a sample player takes 1 or 2.
    TooManyChannels(u32),
    /// The unit, this text, is none of `r`, `rate`, `c` and `s`.
    BadUnit(String),
    /// The sustain, this many seconds, is under 8 samples of the server.
    TooShort(f64),
    /// The parameter of this name holds a value of the wrong kind: `s`,
    /// `sound`, `bank` and `unit` take a text, and every other parameter that
    /// `play` reads takes a number.
    WrongKind(String),
    /// The time of the sound, this many seconds from the OSC epoch, is not
    /// finite, or under 0.
    BadTime(f64),
}

impl fmt::Display for Dropped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Silence => write!(f, "the sound is silence: its name is empty"),
            Self::NoName => write!(f, "the sound has no name: it has neither 's' nor 'sound'"),
            Self::UnknownSound(name) => {
                write!(f, "the library has no samples of the sound '{name}'")
            }
            Self::NotLoaded(name) => write!(f, "the sample of the sound '{name}' is not loaded"),
            Self::TooManyChannels(channels) => write!(
                f,
                "the sample has {channels} channels, but a sample player takes 1 or 2"
            ),
            Self::BadUnit(unit) => write!(f, "the unit '{unit}' is none of r, rate, c and s"),
            Self::TooShort(sustain) => write!(f, "the sustain {sustain} s is under 8 samples"),
            Self::WrongKind(name) => write!(
                f,
                "the value of '{name}' is of the wrong kind: 's', 'sound', 'bank' and 'unit' take a text, and the other parameters a number"
            ),
            Self::BadTime(time) => write!(
                f,
                "the time {time} s from the OSC epoch is not finite, or under 0"
            ),
        }
    }
}

impl std::error::Error for Dropped {}

/// The parameters of a sound on its orbit, as the event of `DirtEvent` reads
/// them: from the sound, then from the defaults of the orbit. The sample of
/// the sound sits between them, and gives `note` 0 alone.
struct Parameters<'a> {
    /// The sound.
    sound: &'a Sound,
    /// The orbit of the sound.
    orbit: &'a Orbit,
}

impl<'a> Parameters<'a> {
    /// The value of `name`, if the sound or the orbit holds it.
    fn get(&self, name: &str) -> Option<&'a Value> {
        self.sound.get(name).or_else(|| self.orbit.get(name))
    }

    /// Whether `name` has a value: `notNil` of sclang.
    fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// The number `name`, if it has a value.
    fn optional(&self, name: &str) -> Result<Option<f64>, Dropped> {
        match self.get(name) {
            None => Ok(None),
            Some(Value::Number(number)) => Ok(Some(*number)),
            Some(Value::Text(_)) => Err(Dropped::WrongKind(name.to_owned())),
        }
    }

    /// The number `name`, which the defaults of an orbit hold. sclang throws
    /// on nil in its place, as on a text.
    fn number(&self, name: &str) -> Result<f64, Dropped> {
        self.optional(name)?
            .ok_or_else(|| Dropped::WrongKind(name.to_owned()))
    }

    /// The text `name`, if it has a value.
    fn text(&self, name: &str) -> Result<Option<&'a str>, Dropped> {
        match self.get(name) {
            None => Ok(None),
            Some(Value::Text(text)) => Ok(Some(text)),
            Some(Value::Number(_)) => Err(Dropped::WrongKind(name.to_owned())),
        }
    }

    /// The note: `note` of the sound, else the 0 of its sample, which hides
    /// the defaults of the orbit.
    fn note(&self) -> Result<f64, Dropped> {
        match self.sound.get("note") {
            None => Ok(0.0),
            Some(Value::Number(note)) => Ok(*note),
            Some(Value::Text(_)) => Err(Dropped::WrongKind("note".to_owned())),
        }
    }
}

/// The span of a sound, as `orderTimeSpan` and `calcTimeSpan` of `DirtEvent`
/// compute it.
struct Span {
    /// Where the sample starts, after the swap of a negative speed.
    begin: f64,
    /// Where the sample ends, after the swap of a negative speed.
    end: f64,
    /// The speed at the start.
    speed: f64,
    /// The speed at the end.
    end_speed: f64,
    /// The frequency, in hertz.
    freq: f64,
    /// The duration of the sound between its fades, in seconds.
    sustain: f64,
    /// The duration of the fade out, in seconds.
    fade_time: f64,
    /// The duration of the fade in, in seconds.
    fade_in_time: f64,
    /// The stretch of a sound that `dirt_stretchsample` plays.
    timescale: Option<f64>,
}

impl Span {
    /// The span of a sound of the sample `info`.
    fn new(parameters: &Parameters<'_>, info: SampleInfo) -> Result<Self, Dropped> {
        let speed = parameters.number("speed")?;
        let (begin, end) = if speed < 0.0 {
            (parameters.number("end")?, parameters.number("begin")?)
        } else {
            (parameters.number("begin")?, parameters.number("end")?)
        };
        let length = (end - begin).abs();
        let accelerate = parameters.optional("accelerate")?;
        let freq = freq(parameters)?;
        let unit_duration =
            f64::from(info.frames) / info.sample_rate * sclang::midicps(60.0) / freq;
        let unit = parameters
            .text("unit")?
            .ok_or_else(|| Dropped::WrongKind("unit".to_owned()))?;
        let speed = if unit == "c" {
            speed * parameters.number("cps")? * unit_duration
        } else {
            speed
        };
        let (end_speed, avg_speed) = match accelerate {
            None => (speed, speed.abs()),
            Some(accelerate) => {
                let end_speed = speed * (1.0 + accelerate);
                #[allow(
                    clippy::manual_midpoint,
                    reason = "sclang adds before it halves, and overflows where midpoint does not"
                )]
                let avg_speed = (speed.abs() + end_speed.abs()) * 0.5;
                (end_speed, avg_speed)
            }
        };
        let mut unit_duration = match unit {
            "r" | "rate" | "c" => unit_duration * length / avg_speed,
            "s" => length,
            _ => return Err(Dropped::BadUnit(unit.to_owned())),
        };
        let sustain = if let Some(sustain) = parameters.optional("sustain")? {
            sustain
        } else {
            let delta = delta(parameters)?;
            if let Some(legato) = parameters.optional("legato")? {
                delta * legato
            } else {
                unit_duration *= parameters.number("loop")?.abs();
                unit_duration
            }
        };
        let sustain = sclang::min(unit_duration, sustain);
        let fade_time = sclang::min(parameters.number("fadeTime")?, sustain * 0.19098);
        let fade_in_time = if begin == 0.0 { 0.0 } else { fade_time };
        let timescale = parameters.optional("timescale")?;
        let sustain = timescale.map_or(sustain, |timescale| sustain * timescale);
        Ok(Self {
            begin,
            end,
            speed,
            end_speed,
            freq,
            sustain: sustain - (fade_time + fade_in_time),
            fade_time,
            fade_in_time,
            timescale,
        })
    }
}

/// `~freq.value` of the event: `freq`, else the `midicps` of `midinote`,
/// else of `note + octave * 12`, as the defaults of
/// `DirtOrbit.makeDefaultParentEvent` compute them.
fn freq(parameters: &Parameters<'_>) -> Result<f64, Dropped> {
    if let Some(freq) = parameters.optional("freq")? {
        return Ok(freq);
    }
    let midinote = match parameters.optional("midinote")? {
        Some(midinote) => midinote,
        None => parameters.note()? + parameters.number("octave")? * 12.0,
    };
    Ok(sclang::midicps(midinote))
}

/// `~delta.value` of the event: `delta`, else `dur`, as the default of
/// `DirtOrbit.makeDefaultParentEvent` computes it.
fn delta(parameters: &Parameters<'_>) -> Result<f64, Dropped> {
    match parameters.optional("delta")? {
        Some(delta) => Ok(delta),
        None => parameters.number("dur"),
    }
}

/// A synth of a sound: the name of its definition, and its controls.
struct Synth {
    /// The name of the definition.
    definition: String,
    /// The controls, in their order.
    controls: Vec<ControlSetting>,
}

/// What the synths of a sound read.
struct Values<'a> {
    /// The parameters of the sound.
    parameters: &'a Parameters<'a>,
    /// The configuration of the instance.
    config: &'a Config,
    /// The synth bus of the orbit.
    synth_bus: i32,
    /// The dry bus of the orbit.
    dry_bus: i32,
    /// The buffer of the sample.
    buffer: BufferId,
    /// The frames, the channels and the sample rate of the sample.
    info: SampleInfo,
    /// The span of the sound.
    span: &'a Span,
    /// The pan, from -1 to 1.
    pan: f64,
    /// The value of `cut`.
    cut: f64,
}

impl Values<'_> {
    /// The synths of the sound, in their order: the source of the module
    /// `sound`, the synth of each module of `core-modules.scd` whose test
    /// holds, then the gate of `sendGateSynth`.
    fn synths(&self) -> Result<Vec<Synth>, Dropped> {
        let mut synths = vec![self.source()?];
        synths.extend(self.vowel()?);
        for module in &MODULES {
            if (module.test)(self.parameters)? {
                let controls = module
                    .controls
                    .iter()
                    .map(|&name| Ok(ControlSetting::new(name, self.control(name)?)))
                    .collect::<Result<_, Dropped>>()?;
                synths.push(Synth {
                    definition: format!("{}{}", module.definition, self.config.channels),
                    controls,
                });
            }
        }
        synths.push(self.gate()?);
        Ok(synths)
    }

    /// The source: the player of the sample, as the module `sound` of
    /// `core-modules.scd` sends it.
    fn source(&self) -> Result<Synth, Dropped> {
        let span = self.span;
        let player = if span.timescale.is_some() {
            "dirt_stretchsample"
        } else if self.info.frames > SHORT_FRAMES {
            "dirt_sample_long"
        } else {
            "dirt_sample"
        };
        let mut controls = vec![
            ControlSetting::new("bufnum", ControlValue::Int(self.buffer.get())),
            ControlSetting::new("sustain", float(span.sustain)),
            ControlSetting::new("speed", float(span.speed)),
            ControlSetting::new("freq", float(span.freq)),
            ControlSetting::new("endSpeed", float(span.end_speed)),
            ControlSetting::new("begin", float(span.begin)),
            ControlSetting::new("end", float(span.end)),
            ControlSetting::new("pan", float(self.pan)),
            ControlSetting::new("out", ControlValue::Int(self.synth_bus)),
        ];
        if let Some(timescale) = span.timescale {
            let timescalewin = self.parameters.optional("timescalewin")?.unwrap_or(1.0);
            controls.push(ControlSetting::new("timescale", float(timescale)));
            controls.push(ControlSetting::new("timescalewin", float(timescalewin)));
        }
        Ok(Synth {
            definition: format!("{player}_{}_{}", self.info.channels, self.config.channels),
            controls,
        })
    }

    /// The synth of the module `vowel`, for a `vowel` that is the text of a
    /// vowel of the Vowel quark.
    fn vowel(&self) -> Result<Option<Synth>, Dropped> {
        let Some(Value::Text(name)) = self.parameters.get("vowel") else {
            return Ok(None);
        };
        let Some(formants) = vowel::formants(self.config.register, name) else {
            return Ok(None);
        };
        let array = |numbers: [f64; 5]| {
            ControlValue::Array(
                numbers
                    .iter()
                    .map(|&number| ArrayElement::Float(f32_of(number)))
                    .collect(),
            )
        };
        Ok(Some(Synth {
            definition: format!("dirt_vowel{}", self.config.channels),
            controls: vec![
                ControlSetting::new("out", ControlValue::Int(self.synth_bus)),
                ControlSetting::new("vowelFreqs", array(formants.freqs)),
                ControlSetting::new("vowelAmps", array(formants.amps)),
                ControlSetting::new("vowelRqs", array(formants.rqs)),
                ControlSetting::new("resonance", self.parameter("resonance")?),
            ],
        }))
    }

    /// The value of the control `name` of a module, as `core-modules.scd`
    /// gives it: the synth bus for `out`, the sustain of the sound for
    /// `sustain`, `bandf` for `bandqf`, and else the parameter of its name.
    fn control(&self, name: &str) -> Result<ControlValue, Dropped> {
        match name {
            "out" => Ok(ControlValue::Int(self.synth_bus)),
            "sustain" => Ok(float(self.span.sustain)),
            "bandqf" => self.parameter("bandf"),
            _ => self.parameter(name),
        }
    }

    /// The number `name` as the control value of [`nil_or`].
    fn parameter(&self, name: &str) -> Result<ControlValue, Dropped> {
        self.parameters.optional(name).map(nil_or)
    }

    /// The gate, as `sendGateSynth` of `DirtEvent` sends it, without its
    /// argument `sample`, which the definition lacks.
    fn gate(&self) -> Result<Synth, Dropped> {
        let span = self.span;
        Ok(Synth {
            definition: format!("dirt_gate{}", self.config.channels),
            controls: vec![
                ControlSetting::new("in", ControlValue::Int(self.synth_bus)),
                ControlSetting::new("out", ControlValue::Int(self.dry_bus)),
                ControlSetting::new("amp", self.parameter("amp")?),
                ControlSetting::new("gain", self.parameter("gain")?),
                ControlSetting::new("overgain", self.parameter("overgain")?),
                ControlSetting::new("cut", float(self.cut.abs())),
                ControlSetting::new("sustain", float(span.sustain)),
                ControlSetting::new("fadeInTime", float(span.fade_in_time)),
                ControlSetting::new("fadeTime", float(span.fade_time)),
            ],
        })
    }
}

/// A sound that plays: every value of its bundles that does not depend on
/// the state of the instance.
pub(crate) struct Event {
    /// The index of the orbit.
    orbit: usize,
    /// The `when` of the sound: the `timeStamp` of SuperDirt.
    when: Option<f64>,
    /// The time of the sound, `when + lag + offset * |speed|`, in seconds from
    /// time 0 of `when`.
    time: Option<f64>,
    /// The end of the sound: its time plus its fade in, its sustain and its
    /// fade out.
    end: Option<f64>,
    /// The time tag of the bundles.
    timetag: Timetag,
    /// The buffer of the sample.
    buffer: BufferId,
    /// The value of `cut`.
    cut: f64,
    /// Whether the cut reaches the sounds of every orbit and of every time.
    cut_all: bool,
    /// The value of each parameter of each global effect, in the order of
    /// [`GLOBAL_EFFECTS`].
    effects: Vec<Vec<Option<f64>>>,
    /// The synths, in their order.
    synths: Vec<Synth>,
}

impl Event {
    /// The sound `sound` on the orbits `orbits` of `config`, with the samples
    /// of `library`, as `DirtEvent.play` reads it up to its first message, or
    /// the reason of its drop.
    pub(crate) fn new(
        config: &Config,
        orbits: &[Orbit],
        library: &Library,
        sound: &Sound,
    ) -> Result<Self, Dropped> {
        let orbit = orbit_index(sound, config)?;
        let parameters = Parameters {
            sound,
            orbit: &orbits[orbit],
        };
        let (name, n) = split_name(&parameters)?;
        let (buffer, info) = sample(library, &name, n)?;
        let span = Span::new(&parameters, info)?;
        if span.sustain.is_nan() || span.sustain < 8.0 / config.sample_rate {
            return Err(Dropped::TooShort(span.sustain));
        }
        let (pan, delay_amp, time) = finalise(&parameters, config.channels, span.speed)?;
        let timetag = match time {
            None => Timetag::IMMEDIATELY,
            Some(time) => {
                let seconds = time + config.epoch;
                timetag(seconds).ok_or(Dropped::BadTime(seconds))?
            }
        };
        let cut = parameters.number("cut")?;
        let cut_all = cut != 0.0
            && parameters
                .optional("cutAll")?
                .is_some_and(|cut_all| cut_all != 0.0);
        let effects = GLOBAL_EFFECTS
            .iter()
            .map(|effect| {
                effect
                    .parameters
                    .iter()
                    .map(|&name| match name {
                        "delayAmp" => Ok(Some(delay_amp)),
                        _ => parameters.optional(name),
                    })
                    .collect()
            })
            .collect::<Result<_, _>>()?;
        let synths = Values {
            parameters: &parameters,
            config,
            synth_bus: int(parameters.orbit.synth_bus()),
            dry_bus: int(parameters.orbit.dry_bus()),
            buffer,
            info,
            span: &span,
            pan,
            cut,
        }
        .synths()?;
        let end = time.map(|time| time + span.fade_in_time + span.sustain + span.fade_time);
        Ok(Self {
            orbit,
            when: sound.when(),
            time,
            end,
            timetag,
            buffer,
            cut,
            cut_all,
            effects,
            synths,
        })
    }

    /// The bundles of the sound, as `playSynths` of `DirtEvent` sends them,
    /// on `orbits`: with a `cut` that is not 0, `/error -1` and the cut gates
    /// of the live sounds that the cut reaches, then the global effects, the
    /// group and the synths of the sound. The sounds of `live` that end
    /// before this one starts leave it, and this one joins it.
    pub(crate) fn send(self, orbits: &mut [Orbit], live: &mut Live) -> Vec<Bundle<Command>> {
        if let Some(time) = self.time {
            live.end_before(time);
        }
        let mut bundles = Vec::with_capacity(2);
        if self.cut != 0.0 {
            let mut commands = vec![Command::Error(ErrorMode::BundleOff)];
            commands.extend(live.cut(&self).into_iter().map(cut_gate));
            bundles.push(bundle(self.timetag, commands));
        }
        let orbit = &mut orbits[self.orbit];
        let mut commands = set_global_effects(orbit, &self.effects);
        let group = live.next_group();
        commands.push(Command::GNew(GNew {
            groups: vec![NewGroup {
                id: new_group(group),
                action: AddAction::Tail,
                target: orbit.group(),
            }],
        }));
        live.join(group, &self);
        commands.extend(self.synths.into_iter().map(|synth| {
            Command::SNew(SNew {
                def: synth.definition,
                id: NewNodeId::AUTO,
                action: AddAction::Tail,
                target: NodeId(group),
                controls: synth.controls,
            })
        }));
        bundles.push(bundle(self.timetag, commands));
        bundles
    }
}

/// The index of the orbit of `sound` among the orbits of `config`, as
/// `orbits @@ (event[\orbit] ? 0)` of SuperDirt picks it: `orbit` of the
/// sound alone, truncated toward 0 and wrapped to the count.
fn orbit_index(sound: &Sound, config: &Config) -> Result<usize, Dropped> {
    let number = match sound.get("orbit") {
        None => 0.0,
        Some(Value::Number(number)) => *number,
        Some(Value::Text(_)) => return Err(Dropped::WrongKind("orbit".to_owned())),
    };
    let index = sclang::as_integer(number).rem_euclid(int(config.orbits));
    Ok(index.unsigned_abs() as usize)
}

/// The name of the sound and its `n`, as `splitName` of `DirtEvent` gives
/// them. Without `s`, the text of `sound` splits at `:`, its first part is
/// the name, and its second part gives `n` as `asFloat` of sclang reads it.
/// With `bank`, the name is `bank_name`.
fn split_name(parameters: &Parameters<'_>) -> Result<(String, Option<f64>), Dropped> {
    let (name, note) = if let Some(name) = parameters.text("s")? {
        (name, None)
    } else {
        let sound = parameters.text("sound")?.ok_or(Dropped::NoName)?;
        let mut parts = sound.split(':');
        (parts.next().unwrap_or_default(), parts.next())
    };
    let name = match parameters.text("bank")? {
        Some(bank) => format!("{bank}_{name}"),
        None => name.to_owned(),
    };
    if name.is_empty() {
        return Err(Dropped::Silence);
    }
    let n = match note {
        Some(note) => Some(sclang::as_float(note)),
        None => parameters.optional("n")?,
    };
    Ok((name, n))
}

/// The buffer and the info of the sample `n` of `name`, as `getEvent` of
/// `DirtSoundLibrary` picks it: `n` truncated toward 0 and wrapped to the
/// count, and 0 without `n`.
fn sample(
    library: &Library,
    name: &str,
    n: Option<f64>,
) -> Result<(BufferId, SampleInfo), Dropped> {
    let sample = library
        .sample(name, n.unwrap_or(0.0))
        .ok_or_else(|| Dropped::UnknownSound(name.to_owned()))?;
    let info = sample
        .info()
        .ok_or_else(|| Dropped::NotLoaded(name.to_owned()))?;
    if info.channels > SAMPLE_CHANNELS {
        return Err(Dropped::TooManyChannels(info.channels));
    }
    Ok((sample.buffer(), info))
}

/// `finaliseParameters` of `DirtEvent` for a sound of the speed `speed` on
/// `channels` channels: the pan from -1 to 1, `delayAmp`, and the time of
/// the sound in seconds from time 0 of `when`, which a sound without `when`
/// lacks.
fn finalise(
    parameters: &Parameters<'_>,
    channels: u32,
    speed: f64,
) -> Result<(f64, f64, Option<f64>), Dropped> {
    let mut pan = parameters.number("pan")?;
    if let Some(channel) = parameters.optional("channel")? {
        pan += channel / f64::from(channels);
    }
    let delay_amp = parameters.optional("delay")?.unwrap_or(0.0);
    let time = match parameters.sound.when() {
        None => None,
        Some(when) => {
            Some(when + parameters.number("lag")? + parameters.number("offset")? * speed.abs())
        }
    };
    Ok((pan * 2.0 - 1.0, delay_amp, time))
}

/// The time tag of `seconds` from the OSC epoch: the whole seconds modulo
/// 2 ** 32, and the fraction in units of 2 ** -32, rounded down. None for
/// seconds that are not finite, or under 0.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "both parts lie in [0, 2 ** 32), and the cast of the fraction rounds it down"
)]
fn timetag(seconds: f64) -> Option<Timetag> {
    if !(seconds.is_finite() && seconds >= 0.0) {
        return None;
    }
    let whole = seconds.floor();
    Some(Timetag {
        seconds: (whole % TWO_POW_32) as u32,
        fraction: ((seconds - whole) * TWO_POW_32) as u32,
    })
}

/// `set` of each global effect of `orbit` for a sound, with the values
/// `effects` of their parameters: a synth resumes if one of its parameters
/// has a value, and the monitor always does. Then one `/n_set` names each
/// parameter whose value differs from the last one, with the int 0 for no
/// value.
fn set_global_effects(orbit: &mut Orbit, effects: &[Vec<Option<f64>>]) -> Vec<Command> {
    let mut commands = Vec::new();
    for (effect, values) in GLOBAL_EFFECTS.iter().zip(effects) {
        let node = (effect.synth)(orbit);
        let mut changed = Vec::new();
        for (&name, &value) in effect.parameters.iter().zip(values) {
            if orbit.change_effect(name, value) {
                changed.push(ControlSetting::new(name, nil_or(value)));
            }
        }
        if effect.always_run || values.iter().any(Option::is_some) {
            commands.push(Command::NRun(NRun {
                nodes: vec![NodeRun { node, run: true }],
            }));
            commands.push(Command::NSet(NSet {
                node,
                controls: vec![
                    ControlSetting::new("pauseImmediately", 0),
                    ControlSetting::new("dirt_resumed", 1),
                ],
            }));
        }
        if !changed.is_empty() {
            commands.push(Command::NSet(NSet {
                node,
                controls: changed,
            }));
        }
    }
    commands
}

/// `/n_set node cut_gate 0`, which ends the sound of the group `node`.
fn cut_gate(node: NodeId) -> Command {
    Command::NSet(NSet {
        node,
        controls: vec![ControlSetting::new("cut_gate", 0)],
    })
}

/// A bundle of `commands` at `time`.
fn bundle(time: Timetag, commands: Vec<Command>) -> Bundle<Command> {
    Bundle {
        time,
        packets: commands.into_iter().map(Packet::Message).collect(),
    }
}

/// `group`, a node id of `sound_nodes`, as the id of a new node.
/// [`Config::check`] refuses node ids under 1, so the server never picks it.
fn new_group(group: i32) -> NewNodeId {
    NewNodeId::new(group).unwrap_or(NewNodeId::AUTO)
}

/// The value of a parameter as a control value: its float, or the int 0 for
/// no value, as sclang writes nil.
fn nil_or(value: Option<f64>) -> ControlValue {
    value.map_or(ControlValue::Int(0), float)
}

/// A number as a control value: its float.
fn float(value: f64) -> ControlValue {
    ControlValue::Float(f32_of(value))
}

/// The f32 nearest to `value`: a control value on the wire.
#[allow(
    clippy::cast_possible_truncation,
    reason = "a control value on the wire is an f32, and the cast rounds to the nearest, as the C cast of sclang does"
)]
fn f32_of(value: f64) -> f32 {
    value as f32
}

/// Whether two `when`s are one time. A sound without `when` differs from
/// every sound.
#[allow(
    clippy::float_cmp,
    reason = "cutAllCuts of DirtEvent compares the time stamps exactly"
)]
fn same_time(a: Option<f64>, b: Option<f64>) -> bool {
    matches!((a, b), (Some(a), Some(b)) if a == b)
}

/// The sounds that may still play, and the next node id for the group of a
/// sound.
#[derive(Clone, Debug)]
pub(crate) struct Live {
    /// The node ids for the groups of the sounds: `sound_nodes`.
    nodes: Range<i32>,
    /// The next node id to try.
    next: i32,
    /// The live sounds, by their groups.
    sounds: BTreeMap<i32, LiveSound>,
    /// The count of the sounds that joined, which orders their starts.
    joined: u64,
}

/// A sound that may still play: what a cut tests.
#[derive(Clone, Debug)]
struct LiveSound {
    /// The place of the sound in the order of the starts.
    serial: u64,
    /// The index of its orbit.
    orbit: usize,
    /// Its cut group: the magnitude of its `cut`.
    cut_group: f64,
    /// The buffer of its sample.
    buffer: BufferId,
    /// Its `when`.
    when: Option<f64>,
    /// Its end, or none without a `when`.
    end: Option<f64>,
}

impl Live {
    /// No live sounds, and the first node id of `nodes` next.
    pub(crate) fn new(nodes: Range<i32>) -> Self {
        Self {
            next: nodes.start,
            nodes,
            sounds: BTreeMap::new(),
            joined: 0,
        }
    }

    /// Forgets the live sounds, and starts the node ids over.
    pub(crate) fn clear(&mut self) {
        self.next = self.nodes.start;
        self.sounds.clear();
    }

    /// Takes the sound of the group `node` out, at its `/n_end`.
    pub(crate) fn end(&mut self, node: NodeId) {
        self.sounds.remove(&node.0);
    }

    /// Takes out the sounds that end before `time`, when a later sound
    /// starts.
    fn end_before(&mut self, time: f64) {
        self.sounds
            .retain(|_, sound| !sound.end.is_some_and(|end| end < time));
    }

    /// The groups of the live sounds that `event` cuts, in the order of their
    /// starts, as `cutAllCuts` of `DirtEvent` finds them: those of its cut
    /// group, on its orbit at another `when` unless `cutAll` is on, and for a
    /// negative `cut` of its sample alone.
    #[allow(
        clippy::float_cmp,
        reason = "cutAllCuts of DirtEvent compares the cut groups exactly"
    )]
    fn cut(&self, event: &Event) -> Vec<NodeId> {
        let cut_group = event.cut.abs();
        let mut cut: Vec<(u64, i32)> = self
            .sounds
            .iter()
            .filter(|(_, sound)| {
                sound.cut_group == cut_group
                    && (event.cut_all
                        || (sound.orbit == event.orbit && !same_time(sound.when, event.when)))
                    && (event.cut > 0.0 || sound.buffer == event.buffer)
            })
            .map(|(&group, sound)| (sound.serial, group))
            .collect();
        cut.sort_unstable();
        cut.into_iter().map(|(_, group)| NodeId(group)).collect()
    }

    /// The group of a new sound: the first node id from the next one on that
    /// no live sound holds. The ids start over at the end of the range. With
    /// every id live, the oldest live sound leaves first, and its id serves
    /// again.
    fn next_group(&mut self) -> i32 {
        if self.sounds.len() >= self.nodes.len() {
            let oldest = self
                .sounds
                .iter()
                .min_by_key(|(_, sound)| sound.serial)
                .map(|(&group, _)| group);
            if let Some(group) = oldest {
                self.sounds.remove(&group);
            }
        }
        // Fewer sounds than ids are live, so an id is free.
        loop {
            let group = self.next;
            self.next = if group + 1 == self.nodes.end {
                self.nodes.start
            } else {
                group + 1
            };
            if !self.sounds.contains_key(&group) {
                return group;
            }
        }
    }

    /// Adds the sound of `event` in the group `group`.
    fn join(&mut self, group: i32, event: &Event) {
        self.sounds.insert(
            group,
            LiveSound {
                serial: self.joined,
                orbit: event.orbit,
                cut_group: event.cut.abs(),
                buffer: event.buffer,
                when: event.when,
                end: event.end,
            },
        );
        self.joined += 1;
    }
}

#[cfg(test)]
mod tests {
    use scsynth_osc::{NodeId, Timetag};

    use super::{Dropped, Parameters, TWO_POW_32, split_name, timetag};
    use crate::config::Config;
    use crate::orbit::Orbit;
    use crate::sound::Sound;
    use crate::synthdefs::Delay;
    use crate::vowel::Register;

    /// The name and the `n` of `sound`, on an orbit with the defaults of
    /// SuperDirt.
    fn split(sound: &Sound) -> Result<(String, Option<f64>), Dropped> {
        let config = Config {
            channels: 2,
            orbits: 1,
            root: NodeId(1),
            fixed_nodes: 100,
            sound_nodes: 1000..2000,
            first_bus: 16,
            out_bus: 0,
            sample_rate: 48_000.0,
            delay: Delay::Comb,
            register: Register::default(),
            epoch: 0.0,
        };
        let orbits = Orbit::all(&config);
        split_name(&Parameters {
            sound,
            orbit: &orbits[0],
        })
    }

    #[test]
    fn without_s_sound_splits_at_its_colons_into_the_name_and_the_as_float_of_n() {
        for (text, name, n) in [
            ("bd", "bd", None),
            ("bd:3", "bd", Some(3.0)),
            ("bd:", "bd", Some(0.0)),
            ("bd:x", "bd", Some(0.0)),
            ("bd:1x", "bd", Some(1.0)),
            ("bd:-1", "bd", Some(-1.0)),
            ("bd:2.5", "bd", Some(2.5)),
            ("bd:1:2", "bd", Some(1.0)),
        ] {
            let sound = Sound::new().with("sound", text);
            assert_eq!(split(&sound), Ok((name.to_owned(), n)), "{text:?}");
        }
    }

    #[test]
    fn s_comes_before_sound_and_the_n_of_sound_before_the_parameter_n() {
        for (sound, name, n) in [
            (Sound::new().s("sn").with("sound", "bd:3"), "sn", None),
            (Sound::new().s("sn").n(2.0), "sn", Some(2.0)),
            (Sound::new().with("sound", "bd:3").n(2.0), "bd", Some(3.0)),
            (Sound::new().with("sound", "bd").n(2.0), "bd", Some(2.0)),
        ] {
            assert_eq!(split(&sound), Ok((name.to_owned(), n)), "{sound:?}");
        }
    }

    #[test]
    fn bank_joins_the_name_after_an_underscore() {
        for (sound, name, n) in [
            (Sound::new().s("bd").with("bank", "x"), "x_bd", None),
            (
                Sound::new().with("sound", "bd:1").with("bank", "x"),
                "x_bd",
                Some(1.0),
            ),
            (Sound::new().s("").with("bank", "x"), "x_", None),
        ] {
            assert_eq!(split(&sound), Ok((name.to_owned(), n)), "{sound:?}");
        }
    }

    #[test]
    fn an_empty_name_is_silence_before_a_wrong_n_and_no_s_and_no_sound_is_no_name() {
        for (sound, dropped) in [
            (Sound::new().s(""), Dropped::Silence),
            (Sound::new().with("sound", ""), Dropped::Silence),
            (Sound::new().with("sound", ":3"), Dropped::Silence),
            (Sound::new().s("").with("n", "x"), Dropped::Silence),
            (Sound::new(), Dropped::NoName),
            (Sound::new().n(1.0), Dropped::NoName),
            (
                Sound::new().with("s", 3),
                Dropped::WrongKind("s".to_owned()),
            ),
            (
                Sound::new().with("sound", 3),
                Dropped::WrongKind("sound".to_owned()),
            ),
            (
                Sound::new().s("bd").with("bank", 1),
                Dropped::WrongKind("bank".to_owned()),
            ),
            (
                Sound::new().s("bd").with("n", "x"),
                Dropped::WrongKind("n".to_owned()),
            ),
        ] {
            assert_eq!(split(&sound), Err(dropped), "{sound:?}");
        }
    }

    #[test]
    fn a_time_tag_holds_the_whole_seconds_modulo_2_pow_32_and_the_fraction_rounded_down() {
        for (seconds, whole, fraction) in [
            (0.0, 0, 0),
            (1.5, 1, 2_147_483_648),
            (3.499_999_999_999_999_6, 3, 2_147_483_647),
            (3.5, 3, 2_147_483_648),
            (1.0 / TWO_POW_32, 0, 1),
            (0.9 / TWO_POW_32, 0, 0),
            (TWO_POW_32 - 1.0, u32::MAX, 0),
            (TWO_POW_32, 0, 0),
            (TWO_POW_32 + 2.25, 2, 1_073_741_824),
        ] {
            assert_eq!(
                timetag(seconds),
                Some(Timetag {
                    seconds: whole,
                    fraction
                }),
                "{seconds}"
            );
        }
    }

    #[test]
    fn a_time_that_is_not_finite_or_under_0_has_no_time_tag() {
        for seconds in [-1e-300, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(timetag(seconds), None, "{seconds}");
        }
    }
}
