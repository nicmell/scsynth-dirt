//! The golden events of `tests/fixtures/events.json`, as
//! `generator/events.scd` writes them with sclang 3.14.1 and SuperDirt: for
//! each scenario, the instance and its library, the start of its orbits, and
//! the bundles that SuperDirt sends for each sound. The reader takes each
//! number from its text, through `str::parse` of Rust.

use std::collections::BTreeMap;
use std::fs;

use scsynth_dirt::{Delay, Dirt, Register};
use scsynth_osc::NodeId;
use serde::Deserialize;
use serde::de::{DeserializeOwned, Deserializer, Error as _};
use serde_json::value::RawValue;

/// The golden events.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Events {
    /// The scenarios, in the order of `generator/events.scd`.
    pub scenarios: Vec<Scenario>,
}

/// A scenario: a new instance of SuperDirt, the start of its orbits, and its
/// steps.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Scenario {
    /// The name of the scenario, such as `cut`.
    pub name: String,
    /// The structure of the instance.
    pub config: Config,
    /// The names of the library, in the order of their adds.
    pub library: Vec<Entry>,
    /// The bundles that `initNodeTree` sends for each orbit, in the order of
    /// the orbits.
    pub start: Vec<Bundle>,
    /// The steps, in their order.
    pub steps: Vec<Step>,
}

impl Scenario {
    /// A new instance of the crate for the scenario: the numbers of its
    /// configuration, with the delay of `CombL`, the voice register of
    /// SuperDirt and the epoch 0. Its node ids differ from those of the
    /// golden bundles.
    pub fn dirt(&self) -> Dirt {
        let config = scsynth_dirt::Config {
            channels: self.config.channels,
            orbits: self.config.orbits,
            root: NodeId(1),
            fixed_nodes: 100,
            sound_nodes: 2000..3000,
            first_bus: self.config.first_bus,
            out_bus: self.config.out_bus,
            sample_rate: self.config.sample_rate,
            delay: Delay::Comb,
            register: Register::CounterTenor,
            epoch: 0.0,
        };
        Dirt::new(config).unwrap_or_else(|error| panic!("{}: {error}", self.name))
    }
}

/// The structure of an instance, with the fields of the same names in the
/// `Config` of the crate.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Config {
    /// The channel count of the orbits.
    pub channels: u32,
    /// The number of orbits.
    pub orbits: u32,
    /// The first of the private audio buses of the orbits.
    pub first_bus: u32,
    /// The first of the output buses of the monitors.
    pub out_bus: u32,
    /// The sample rate of the server, in hertz.
    #[serde(deserialize_with = "exact")]
    pub sample_rate: f64,
}

/// A name of the library, with its samples.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Entry {
    /// The name, such as `bd`.
    pub name: String,
    /// The samples, in their order.
    pub samples: Vec<Sample>,
}

/// A sample: its buffer, and what `/b_info` gives of the buffer.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[allow(
    clippy::struct_field_names,
    reason = "the fields take the keys of events.json, which are the names of SampleInfo"
)]
pub struct Sample {
    /// The buffer number.
    pub buffer: i32,
    /// The number of frames.
    pub frames: u32,
    /// The number of channels.
    pub channels: u32,
    /// The sample rate, in hertz.
    #[serde(deserialize_with = "exact")]
    pub sample_rate: f64,
}

/// A step of a scenario.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(try_from = "StepFields")]
pub enum Step {
    /// An orbit default, which SuperDirt keeps and does not send.
    Set {
        /// The orbit default.
        set: OrbitDefault,
    },
    /// A sound, and the bundles that SuperDirt sends for it. A sound that
    /// SuperDirt drops has no bundle.
    Sound {
        /// The sound.
        sound: Sound,
        /// The bundles, in the order of their sends.
        bundles: Vec<Bundle>,
    },
}

/// The fields of a step in `events.json`. serde reads an `untagged` enum
/// through a buffer of its own, which holds no `RawValue` (measured).
#[derive(Deserialize)]
struct StepFields {
    /// The orbit default of a step that sets one.
    set: Option<OrbitDefault>,
    /// The sound of a step that plays one.
    sound: Option<Sound>,
    /// The bundles of the sound.
    bundles: Option<Vec<Bundle>>,
}

impl TryFrom<StepFields> for Step {
    type Error = &'static str;

    fn try_from(fields: StepFields) -> Result<Self, Self::Error> {
        match fields {
            StepFields {
                set: Some(set),
                sound: None,
                bundles: None,
            } => Ok(Self::Set { set }),
            StepFields {
                set: None,
                sound: Some(sound),
                bundles: Some(bundles),
            } => Ok(Self::Sound { sound, bundles }),
            _ => Err("a step holds set alone, or sound and bundles"),
        }
    }
}

/// An orbit default: `orbit.set(name, value)` of SuperDirt.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct OrbitDefault {
    /// The index of the orbit.
    pub orbit: usize,
    /// The name of the parameter.
    pub name: String,
    /// The value of the parameter.
    pub value: Value,
}

/// A sound: its time, and its parameters.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Sound {
    /// The time of the sound, in seconds: the `latency` and the `timeStamp`
    /// of its event in SuperDirt.
    #[serde(deserialize_with = "exact")]
    pub when: f64,
    /// The parameters, by name.
    pub parameters: BTreeMap<String, Value>,
}

/// A bundle that reached the address of the server.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Bundle {
    /// The time of the bundle, in seconds, or none for a bundle with no time.
    #[serde(deserialize_with = "exact_or_null")]
    pub time: Option<f64>,
    /// The messages, in their order.
    pub messages: Vec<Message>,
}

/// A message: its address and its arguments.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(try_from = "Vec<Value>")]
pub struct Message {
    /// The address, such as `/s_new`.
    pub address: String,
    /// The arguments, in their order.
    pub args: Vec<Value>,
}

impl TryFrom<Vec<Value>> for Message {
    type Error = String;

    fn try_from(values: Vec<Value>) -> Result<Self, Self::Error> {
        let mut values = values.into_iter();
        match values.next() {
            Some(Value::Text(address)) => Ok(Self {
                address,
                args: values.collect(),
            }),
            first => Err(format!("a message starts with its address, not {first:?}")),
        }
    }
}

/// A value of sclang: an argument of a message, a parameter of a sound, or
/// the value of an orbit default.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(try_from = "Box<RawValue>")]
pub enum Value {
    /// nil.
    Null,
    /// A Boolean, such as the value of `cutAll`.
    Bool(bool),
    /// An Integer: a number with no point and no exponent.
    Int(i64),
    /// A Float: a number with a point or an exponent.
    Float(f64),
    /// A Symbol or a String.
    Text(String),
    /// An array argument of a message.
    List(Vec<Value>),
}

impl TryFrom<Box<RawValue>> for Value {
    type Error = String;

    fn try_from(raw: Box<RawValue>) -> Result<Self, Self::Error> {
        let text = raw.get();
        let value = match text {
            "null" => Self::Null,
            "true" => Self::Bool(true),
            "false" => Self::Bool(false),
            _ if text.starts_with('"') => Self::Text(json(text)?),
            _ if text.starts_with('[') => Self::List(json(text)?),
            _ if text.contains(['.', 'e', 'E']) => Self::Float(number(text)?),
            _ => Self::Int(text.parse().map_err(|error| format!("{text}: {error}"))?),
        };
        Ok(value)
    }
}

/// The string or the list of values of the JSON text `text`.
fn json<T: DeserializeOwned>(text: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|error| format!("{text}: {error}"))
}

/// The f64 nearest to the JSON number `text`, by `str::parse` of Rust.
fn number(text: &str) -> Result<f64, String> {
    text.parse().map_err(|error| format!("{text}: {error}"))
}

/// An f64 field, from the text of its number.
fn exact<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let raw = Box::<RawValue>::deserialize(deserializer)?;
    number(raw.get()).map_err(D::Error::custom)
}

/// An f64 field that can be null, from the text of its number.
fn exact_or_null<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<f64>, D::Error> {
    let raw = Box::<RawValue>::deserialize(deserializer)?;
    match raw.get() {
        "null" => Ok(None),
        text => number(text).map(Some).map_err(D::Error::custom),
    }
}

/// The golden events of `events.json`.
pub fn read() -> Events {
    let text = fs::read_to_string(super::fixture("events.json")).expect("events.json reads");
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("events.json: {error}"))
}
