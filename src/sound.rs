//! A Dirt sound: its time and its parameters by name, a setter for each field
//! of libdirt, and the read of the arguments of a `/dirt/play` message.

use std::collections::BTreeMap;
use std::fmt;

use scsynth_osc::RawArg;

/// A Dirt sound: an optional time, in seconds, and its parameters by name.
/// A parameter that is not set stays out of the sound.
///
/// Each of the 32 fields of `dirt_sound_t` in libdirt has a setter in a
/// chain, by its name and with the type that libdirt gives it: an f64 for a
/// float, an i32 for an int, a text for a string. `loop` is a keyword of
/// Rust, so its setter is `r#loop`. A setter stores its value as it is,
/// under the name of its field, and no default of libdirt applies.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sound {
    /// The time, in seconds.
    when: Option<f64>,
    /// The parameters, by name.
    parameters: BTreeMap<String, Value>,
}

impl Sound {
    /// A sound with no time and no parameters, as [`Sound::default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The sound at the time `when`, in seconds.
    #[must_use]
    pub fn at(mut self, when: f64) -> Self {
        self.when = Some(when);
        self
    }

    /// The time of the sound, in seconds, if it has one.
    #[must_use]
    pub fn when(&self) -> Option<f64> {
        self.when
    }

    /// Sets the time of the sound, in seconds, or takes it out with `None`.
    pub fn set_when(&mut self, when: Option<f64>) {
        self.when = when;
    }

    /// The sound with the parameter `name` set to `value`, in place of any
    /// value that it had.
    #[must_use]
    pub fn with(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.set(name, value);
        self
    }

    /// Sets the parameter `name` to `value`, in place of any value that it
    /// had.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<Value>) -> &mut Self {
        self.parameters.insert(name.into(), value.into());
        self
    }

    /// The value of the parameter `name`, if the sound holds it.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.parameters.get(name)
    }

    /// Takes the parameter `name` out of the sound, and returns its value.
    pub fn remove(&mut self, name: &str) -> Option<Value> {
        self.parameters.remove(name)
    }

    /// The parameters of the sound, as pairs of a name and a value, in the
    /// order of the bytes of their names.
    pub fn parameters(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.parameters
            .iter()
            .map(|(name, value)| (name.as_str(), value))
    }

    /// The sound of the arguments of a `/dirt/play` message, which are pairs
    /// of a name and a value. A name is a string. An int, a float, a double
    /// or a long gives a number, a string gives a text, true gives 1 and
    /// false gives 0. A later pair replaces an earlier one of the same name.
    /// The sound has no time: the time of the message is the time tag of its
    /// bundle.
    ///
    /// # Errors
    ///
    /// [`SoundError::OddCount`] for an odd count of arguments,
    /// [`SoundError::NotAName`] for a name that is no string, and
    /// [`SoundError::WrongType`] for a value of another type.
    pub fn from_args(args: &[RawArg]) -> Result<Self, SoundError> {
        if args.len() % 2 == 1 {
            return Err(SoundError::OddCount(args.len()));
        }
        let mut sound = Self::new();
        for (index, pair) in args.chunks_exact(2).enumerate() {
            let RawArg::String(name) = &pair[0] else {
                return Err(SoundError::NotAName(2 * index));
            };
            let value = value_of(&pair[1]).ok_or_else(|| SoundError::WrongType(name.clone()))?;
            sound.set(name.clone(), value);
        }
        Ok(sound)
    }
}

/// The setters in a chain of the fields of libdirt, as `DIRT_FIELDS` of
/// `dirt.h` lists them: `F` for a float, which a setter takes as an f64, `I`
/// for an int, as an i32, and `S` for a string, as a text.
macro_rules! fields {
    ($($(#[$doc:meta])* $kind:ident($setter:ident),)*) => {
        impl Sound {
            $(
                $(#[$doc])*
                #[must_use]
                pub fn $setter(self, value: fields!(@type $kind)) -> Self {
                    let name = stringify!($setter).trim_start_matches("r#");
                    self.with(name, fields!(@value $kind value))
                }
            )*
        }
    };
    (@type F) => { f64 };
    (@type I) => { i32 };
    (@type S) => { impl Into<String> };
    (@value S $value:ident) => { Value::Text($value.into()) };
    (@value $kind:ident $value:ident) => { $value };
}

fields! {
    /// The tempo, in cycles per second, for the timings of the unit c and for
    /// a delay with `lock` 1.
    F(cps),
    /// The sound: the name of the folder of its samples.
    S(s),
    /// A delay of `offset * |speed|` seconds in SuperDirt, unused in libdirt.
    F(offset),
    /// Where in the sample to start playing, from 0 to 1.
    F(begin),
    /// Where in the sample to stop playing, from 0 to 1.
    F(end),
    /// How fast to play the sample: a negative speed plays it backwards.
    F(speed),
    /// The panning, from 0 for left to 1 for right. On more than 2 channels,
    /// it goes around the circle of the channels.
    F(pan),
    /// The velocity, which neither libdirt nor [`Dirt::play`](crate::Dirt::play)
    /// reads.
    F(velocity),
    /// The vowel of the formant filter: a, e, i, o or u.
    S(vowel),
    /// The frequency of the low pass filter, in hertz.
    F(cutoff),
    /// The resonance of the low pass filter and of the vowel, from 0 to 1.
    F(resonance),
    /// The change of speed: the end speed is `speed * (1 + accelerate)`.
    F(accelerate),
    /// The wave-shaping distortion.
    F(shape),
    /// The kriole, which neither libdirt nor [`Dirt::play`](crate::Dirt::play)
    /// reads.
    I(kriole),
    /// The volume, at the end of the chain of effects.
    F(gain),
    /// The cut group: a sound stops the sounds of its group, if negative those of its sample.
    I(cut),
    /// The level of the echoes of the global delay effect: its `delayAmp`.
    F(delay),
    /// The delay time of the global delay effect.
    F(delaytime),
    /// The feedback amount of the global delay effect.
    F(delayfeedback),
    /// The bit depth reduction.
    F(crush),
    /// The sample rate reduction.
    I(coarse),
    /// The frequency of the high pass filter, in hertz.
    F(hcutoff),
    /// The resonance of the high pass filter.
    F(hresonance),
    /// The frequency of the band pass filter.
    F(bandf),
    /// The Q factor of the band pass filter.
    F(bandq),
    /// The unit of the timings: r or rate, c for cycles, s for seconds.
    S(unit),
    /// How many times to repeat the playback.
    F(r#loop),
    /// The index of the sample in the folder of the sound.
    F(n),
    /// The duration of the attack of the volume envelope, in seconds.
    F(attack),
    /// The duration of the hold of the volume envelope, in seconds.
    F(hold),
    /// The duration of the release of the volume envelope, in seconds.
    F(release),
    /// The index of the orbit of the sound.
    I(orbit),
}

/// The value of the argument `arg` of a `/dirt/play` message, or none for an
/// argument of another type.
#[allow(
    clippy::cast_precision_loss,
    reason = "a sound holds its numbers as f64, so a long past 2 ** 53 in magnitude rounds to the nearest f64"
)]
fn value_of(arg: &RawArg) -> Option<Value> {
    match arg {
        RawArg::Int(value) => Some(Value::from(*value)),
        RawArg::Float(value) => Some(Value::from(*value)),
        RawArg::Double(value) => Some(Value::Number(*value)),
        RawArg::Long(value) => Some(Value::Number(*value as f64)),
        RawArg::String(value) => Some(Value::Text(value.clone())),
        RawArg::True => Some(Value::Number(1.0)),
        RawArg::False => Some(Value::Number(0.0)),
        _ => None,
    }
}

/// The value of a parameter of a [`Sound`]: a number or a text. With the
/// feature `serde`, it is a bare number or a bare string.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(untagged)
)]
pub enum Value {
    /// A number.
    Number(f64),
    /// A text.
    Text(String),
}

impl Value {
    /// The number, or none for a text.
    #[must_use]
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(*number),
            Self::Text(_) => None,
        }
    }

    /// The text, or none for a number.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Number(_) => None,
            Self::Text(text) => Some(text),
        }
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::Number(f64::from(value))
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Number(f64::from(value))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

/// Why the arguments of a `/dirt/play` message are not a [`Sound`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SoundError {
    /// The count of the arguments is odd, so they are not pairs of a name
    /// and a value.
    OddCount(usize),
    /// The argument at this index is no string, where the name of a pair
    /// must be.
    NotAName(usize),
    /// The value of the parameter of this name is none of an int, a float, a
    /// double, a long, a string, true and false.
    WrongType(String),
}

impl fmt::Display for SoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OddCount(count) => write!(
                f,
                "the count {count} of the arguments is odd, but they must be pairs of a name and a value"
            ),
            Self::NotAName(index) => write!(
                f,
                "argument {index} is no string, but it must be the name of a parameter"
            ),
            Self::WrongType(name) => write!(
                f,
                "the value of '{name}' must be an int, a float, a double, a long, a string, true or false"
            ),
        }
    }
}

impl std::error::Error for SoundError {}
