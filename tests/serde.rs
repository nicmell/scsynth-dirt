//! The serde names of the enums of the crate: each writes its variants in
//! snake case, as scsynth-osc does, and reads them back.
#![cfg(feature = "serde")]

use std::fmt::Debug;

use scsynth_dirt::{Delay, Register};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Asserts that `value` writes as the JSON string `name`, and reads back from
/// it.
fn assert_name<T>(value: &T, name: &str)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let text = serde_json::to_string(value).unwrap_or_else(|error| panic!("{value:?}: {error}"));
    assert_eq!(text, format!("\"{name}\""), "{value:?}");
    let read: T = serde_json::from_str(&text).unwrap_or_else(|error| panic!("{text}: {error}"));
    assert_eq!(&read, value, "{text}");
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
