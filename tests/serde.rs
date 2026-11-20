//! The serde forms of the crate: each enum writes its variants in snake
//! case, as scsynth-osc does, and a `Value` writes as a bare number or a
//! bare string. Each reads back, and so do a `Sound` and a `Config`.
#![cfg(feature = "serde")]

use std::fmt::Debug;

use scsynth_dirt::{Config, Delay, Register, Sound, Value};
use scsynth_osc::NodeId;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Asserts that `value` writes as the JSON text `json`, and reads back from
/// it.
fn assert_json<T>(value: &T, json: &str)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let text = serde_json::to_string(value).unwrap_or_else(|error| panic!("{value:?}: {error}"));
    assert_eq!(text, json, "{value:?}");
    let read: T = serde_json::from_str(&text).unwrap_or_else(|error| panic!("{text}: {error}"));
    assert_eq!(&read, value, "{text}");
}

/// Asserts that `value` writes as the JSON string `name`, and reads back from
/// it.
fn assert_name<T>(value: &T, name: &str)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    assert_json(value, &format!("\"{name}\""));
}

#[test]
fn delay_writes_comb_and_switch() {
    assert_name(&Delay::Comb, "comb");
    assert_name(&Delay::Switch, "switch");
}

#[test]
fn register_writes_its_voices_in_snake_case() {
    assert_name(&Register::Soprano, "soprano");
    assert_name(&Register::Alto, "alto");
    assert_name(&Register::Tenor, "tenor");
    assert_name(&Register::Bass, "bass");
    assert_name(&Register::CounterTenor, "counter_tenor");
}

#[test]
fn a_value_writes_as_a_bare_number_or_a_bare_string() {
    assert_json(&Value::Number(0.25), "0.25");
    assert_json(&Value::Number(-3.0), "-3.0");
    assert_json(&Value::from("bd"), "\"bd\"");
    let read: Value = serde_json::from_str("3").unwrap_or_else(|error| panic!("3: {error}"));
    assert_eq!(read, Value::Number(3.0));
}

#[test]
fn a_sound_with_a_when_and_parameters_reads_back_equal() {
    let sound = Sound::new().at(1.5).s("bd").n(3.0).vowel("a").cut(-1);
    let text = serde_json::to_string(&sound).unwrap_or_else(|error| panic!("{sound:?}: {error}"));
    let read: Sound = serde_json::from_str(&text).unwrap_or_else(|error| panic!("{text}: {error}"));
    assert_eq!(read, sound, "{text}");
}

#[test]
fn a_config_reads_back_equal() {
    let config = Config {
        channels: 2,
        orbits: 12,
        root: NodeId(1),
        fixed_nodes: 100,
        sound_nodes: 1000..2000,
        first_bus: 16,
        out_bus: 0,
        sample_rate: 48_000.0,
        delay: Delay::Switch,
        register: Register::Bass,
        epoch: 2_208_988_800.0,
    };
    let text = serde_json::to_string(&config).unwrap_or_else(|error| panic!("{config:?}: {error}"));
    let read: Config =
        serde_json::from_str(&text).unwrap_or_else(|error| panic!("{text}: {error}"));
    assert_eq!(read, config, "{text}");
}
