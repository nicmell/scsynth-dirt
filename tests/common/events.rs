//! The golden events of `tests/fixtures/events.json`, as
//! `generator/events.scd` writes them with sclang 3.14.1 and SuperDirt: for
//! each scenario, the instance and its library, the start of its orbits, and
//! the bundles that SuperDirt sends for each sound.

use std::collections::BTreeMap;
use std::fs;

use serde::Deserialize;

/// The golden events.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Events {
    /// The scenarios, in the order of the plan.
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
    pub sample_rate: f64,
}

/// A step of a scenario.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
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
    pub when: f64,
    /// The parameters, by name.
    pub parameters: BTreeMap<String, Value>,
}

/// A bundle that reached the address of the server.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Bundle {
    /// The time of the bundle, in seconds, or none for a bundle with no time.
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
#[serde(untagged)]
pub enum Value {
    /// nil.
    Null,
    /// A Boolean, such as the value of `cutAll`.
    Bool(bool),
    /// An Integer.
    Int(i64),
    /// A Float.
    Float(f64),
    /// A Symbol or a String.
    Text(String),
    /// An array argument of a message.
    List(Vec<Value>),
}

/// The golden events of `events.json`.
pub fn read() -> Events {
    let text = fs::read_to_string(super::fixture("events.json")).expect("events.json reads");
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("events.json: {error}"))
}
