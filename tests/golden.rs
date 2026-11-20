//! The golden files of `tests/fixtures`, as `generator/fixtures.scd` wrote
//! them with sclang 3.14.1: each definition file reads and writes back to
//! its own bytes, holds the definitions of its channel count, and fits the
//! limits of the hosts. `vowels.json` holds the formants of the Vowel quark.

mod common;

use std::collections::BTreeMap;
use std::fs;

use scsynth_synthdef::{DefFile, read_def_file, write_def_file};

use common::CHANNELS;

/// The largest datagram that sc-app3 sends to scsynth: `MAX_DATAGRAM` of its
/// core.
const MAX_DATAGRAM: usize = 8_192;

/// The wire buffers of scsynth 3.14.1 without `-w`: `mMaxWireBufs` of
/// `SC_WorldOptions.h`.
const DEFAULT_WIRE_BUFFERS: usize = 64;

/// The names of the 25 definitions of the core files of SuperDirt, without
/// their channel count.
const CORE_STEMS: [&str; 25] = [
    "sample_1_",
    "sample_2_",
    "sample_long_1_",
    "sample_long_2_",
    "stretchsample_1_",
    "stretchsample_2_",
    "gate",
    "out",
    "vowel",
    "crush",
    "coarse",
    "hpf",
    "bpf",
    "shape",
    "lpf",
    "envelope",
    "grain_envelope",
    "pitch_shift",
    "tremolo",
    "phaser",
    "monitor",
    "rms",
    "delay",
    "reverb",
    "leslie",
];

/// The formants of one vowel: its `freqs`, its `amps` and its `rqs`.
type Formants = BTreeMap<String, Vec<f64>>;

/// The names of the core definitions of `channels` channels, sorted.
fn core_names(channels: u32) -> Vec<String> {
    let mut names: Vec<String> = CORE_STEMS
        .iter()
        .map(|stem| format!("dirt_{stem}{channels}"))
        .collect();
    names.sort();
    names
}

/// The names of the definitions of a file, in file order.
fn names(file: &DefFile) -> Vec<&str> {
    file.synth_defs
        .iter()
        .map(|def| def.name.as_str())
        .collect()
}

/// Asserts that each definition of the golden file `name` fits one datagram
/// as a file of its own.
fn assert_fits_one_datagram(name: &str) {
    for def in common::golden(name).synth_defs {
        let bytes = def
            .as_bytes()
            .expect("a golden definition meets the limits");
        let size = bytes.len();
        assert!(size <= MAX_DATAGRAM, "{name} {}: {size} bytes", def.name);
    }
}

/// Asserts that each definition of the golden file `name` fits the default
/// wire buffers of scsynth.
fn assert_fits_the_default_wire_buffers(name: &str) {
    for def in common::golden(name).synth_defs {
        let wires = def
            .wire_buffers()
            .unwrap_or_else(|error| panic!("{name} {}: {error}", def.name));
        assert!(
            wires <= DEFAULT_WIRE_BUFFERS,
            "{name} {}: {wires} wire buffers",
            def.name
        );
    }
}

/// The formants of `vowels.json`, by register and by vowel.
fn vowels() -> BTreeMap<String, BTreeMap<String, Formants>> {
    let text = fs::read_to_string(common::fixture("vowels.json")).expect("vowels.json reads");
    serde_json::from_str(&text).expect("vowels.json is JSON")
}

#[test]
fn each_definition_file_writes_back_to_its_bytes() {
    for channels in CHANNELS {
        for name in [
            format!("core_{channels}"),
            format!("switch_delay_{channels}"),
        ] {
            let bytes = common::golden_bytes(&name);
            let file = read_def_file(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
            let written = write_def_file(&file.synth_defs);
            assert_eq!(written.as_deref(), Ok(&bytes[..]), "{name}");
        }
    }
}

#[test]
fn each_core_file_holds_the_25_definitions_of_its_channel_count_sorted_by_name() {
    for channels in CHANNELS {
        let file = common::golden(&format!("core_{channels}"));
        assert_eq!(names(&file), core_names(channels), "core_{channels}");
    }
}

#[test]
fn each_switch_delay_file_holds_dirt_delay_alone() {
    for channels in CHANNELS {
        let file = common::golden(&format!("switch_delay_{channels}"));
        assert_eq!(names(&file), [format!("dirt_delay{channels}")]);
    }
}

#[test]
fn each_core_definition_fits_one_datagram_and_the_default_wire_buffers() {
    for channels in CHANNELS {
        assert_fits_one_datagram(&format!("core_{channels}"));
        assert_fits_the_default_wire_buffers(&format!("core_{channels}"));
    }
}

#[test]
fn each_switch_delay_definition_fits_one_datagram() {
    for channels in CHANNELS {
        assert_fits_one_datagram(&format!("switch_delay_{channels}"));
    }
}

#[cfg(feature = "sc3-plugins")]
#[test]
fn each_switch_delay_definition_fits_the_default_wire_buffers() {
    for channels in CHANNELS {
        assert_fits_the_default_wire_buffers(&format!("switch_delay_{channels}"));
    }
}

#[test]
fn vowels_json_holds_5_registers_of_5_vowels_with_5_finite_numbers_per_array() {
    let vowels = vowels();
    let registers: Vec<&str> = vowels.keys().map(String::as_str).collect();
    assert_eq!(
        registers,
        ["alto", "bass", "counterTenor", "soprano", "tenor"]
    );
    for (register, table) in &vowels {
        let names: Vec<&str> = table.keys().map(String::as_str).collect();
        assert_eq!(names, ["a", "e", "i", "o", "u"], "{register}");
        for (vowel, formants) in table {
            let fields: Vec<&str> = formants.keys().map(String::as_str).collect();
            assert_eq!(fields, ["amps", "freqs", "rqs"], "{register} {vowel}");
            for (field, numbers) in formants {
                assert_eq!(numbers.len(), 5, "{register} {vowel} {field}");
                assert!(
                    numbers.iter().all(|number| number.is_finite()),
                    "{register} {vowel} {field}: {numbers:?}"
                );
            }
        }
    }
}

#[test]
fn the_vowel_a_of_the_counter_tenor_holds_the_values_of_vowel_sc() {
    let vowels = vowels();
    let formants = &vowels["counterTenor"]["a"];
    assert_eq!(formants["freqs"], [660.0, 1120.0, 2750.0, 3000.0, 3350.0]);
    assert_eq!(formants["amps"][0].to_bits(), 1.0_f64.to_bits());
    assert_eq!(formants["rqs"][0].to_bits(), (1.0_f64 / 80.0).to_bits());
}
