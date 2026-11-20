//! The definitions of `synthdefs`, against the golden definitions that
//! sclang 3.14.1 wrote for SuperDirt: each definition has the bytes of the
//! golden definition of its name, for 1, 2 and 4 channels, and each sample
//! player for samples of 1 to 4 channels. With the feature `sc3-plugins`,
//! the delay of `SwitchDelay` has the bytes of its own golden file, and
//! without it, the builder refuses that delay. Each function refuses a
//! count that the builder refuses, but a player refuses 0 channels with
//! `NotANumber`. `core` gives the 25 definitions in the load order of
//! SuperDirt, and sorted by name, they write the bytes of the core file.
//! For 1, 2 and 4 channels, the sum of the bytes of the definitions, the
//! largest one and the one with the most wire buffers stay as measured. Up to
//! 7 channels, each one fits one datagram, and for 8 channels, `dirt_reverb8`
//! alone does not.

mod common;

use scsynth_dirt::synthdefs::{self, Delay};
use scsynth_synthdef::{BuildError, SynthDef, write_def_file};

use common::{CHANNELS, MAX_DATAGRAM};

/// A public function of `synthdefs` that builds a definition for a count of
/// channels.
type Function = fn(u32) -> Result<SynthDef, BuildError>;

/// Each public function of `synthdefs` for a count of channels, with its
/// name. `dirt_delay` builds the delay of `CombL`.
const FUNCTIONS: &[(&str, Function)] = &[
    ("dirt_monitor", synthdefs::dirt_monitor),
    ("dirt_rms", synthdefs::dirt_rms),
    ("dirt_delay", |channels| {
        synthdefs::dirt_delay(channels, Delay::Comb)
    }),
    ("dirt_reverb", synthdefs::dirt_reverb),
    ("dirt_leslie", synthdefs::dirt_leslie),
    ("dirt_gate", synthdefs::dirt_gate),
    ("dirt_out", synthdefs::dirt_out),
    ("dirt_vowel", synthdefs::dirt_vowel),
    ("dirt_crush", synthdefs::dirt_crush),
    ("dirt_coarse", synthdefs::dirt_coarse),
    ("dirt_hpf", synthdefs::dirt_hpf),
    ("dirt_bpf", synthdefs::dirt_bpf),
    ("dirt_shape", synthdefs::dirt_shape),
    ("dirt_lpf", synthdefs::dirt_lpf),
    ("dirt_envelope", synthdefs::dirt_envelope),
    ("dirt_grain_envelope", synthdefs::dirt_grain_envelope),
    ("dirt_pitch_shift", synthdefs::dirt_pitch_shift),
    ("dirt_tremolo", synthdefs::dirt_tremolo),
    ("dirt_phaser", synthdefs::dirt_phaser),
];

/// A sample player of `synthdefs`, which builds a definition for a count of
/// sample channels and a count of channels.
type Player = fn(u32, u32) -> Result<SynthDef, BuildError>;

/// Each sample player of `synthdefs`, with its name.
const PLAYERS: &[(&str, Player)] = &[
    ("dirt_sample", synthdefs::dirt_sample),
    ("dirt_sample_long", synthdefs::dirt_sample_long),
    ("dirt_stretchsample", synthdefs::dirt_stretchsample),
];

/// The sample channel counts of the golden players: `core_<n>` holds those
/// of 1 and 2, and `players_3_4_<n>` those of 3 and 4.
const SAMPLE_CHANNELS: [u32; 4] = [1, 2, 3, 4];

/// The names of the 25 definitions of `core`, without their channel count,
/// in the order in which SuperDirt loads them.
const LOAD_ORDER: [&str; 25] = [
    "monitor",
    "rms",
    "delay",
    "reverb",
    "leslie",
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
];

/// The definitions of `core` for `channels` channels and `delay`.
fn core(channels: u32, delay: Delay) -> Vec<SynthDef> {
    synthdefs::core(channels, delay)
        .unwrap_or_else(|error| panic!("core({channels}, {delay:?}): {error}"))
}

/// The size of `def` as a file of its own, the bytes of its `/d_recv`.
fn size(def: &SynthDef) -> usize {
    def.as_bytes()
        .unwrap_or_else(|error| panic!("{}: {error}", def.name))
        .len()
}

/// The names of the definitions of `defs` with the largest `measure`, and that
/// measure.
fn largest(defs: &[SynthDef], measure: impl Fn(&SynthDef) -> usize) -> (Vec<&str>, usize) {
    let most = defs.iter().map(&measure).max().unwrap_or_default();
    let names = defs
        .iter()
        .filter(|def| measure(def) == most)
        .map(|def| def.name.as_str())
        .collect();
    (names, most)
}

#[test]
fn each_definition_has_the_bytes_of_sclang_for_each_channel_count() {
    for (name, function) in FUNCTIONS {
        for channels in CHANNELS {
            let def =
                function(channels).unwrap_or_else(|error| panic!("{name}{channels}: {error}"));
            common::assert_golden_bytes(&format!("core_{channels}"), &def);
        }
    }
}

#[cfg(feature = "sc3-plugins")]
#[test]
fn the_switch_delay_has_the_bytes_of_sclang_with_sc3_plugins_for_each_channel_count() {
    for channels in CHANNELS {
        let def = synthdefs::dirt_delay(channels, Delay::Switch)
            .unwrap_or_else(|error| panic!("dirt_delay{channels}: {error}"));
        common::assert_golden_bytes(&format!("switch_delay_{channels}"), &def);
    }
}

#[cfg(not(feature = "sc3-plugins"))]
#[test]
fn the_switch_delay_without_the_feature_sc3_plugins_is_an_unknown_class() {
    for channels in CHANNELS {
        let result = synthdefs::dirt_delay(channels, Delay::Switch);
        assert!(
            matches!(&result, Err(BuildError::UnknownClass(class)) if class == "SwitchDelay"),
            "dirt_delay({channels}, Switch): {:?}",
            result.map(|def| def.name)
        );
    }
}

#[test]
fn core_gives_the_25_definitions_in_the_load_order_of_superdirt() {
    for channels in CHANNELS {
        let names: Vec<String> = core(channels, Delay::Comb)
            .into_iter()
            .map(|def| def.name)
            .collect();
        let expected: Vec<String> = LOAD_ORDER
            .iter()
            .map(|stem| format!("dirt_{stem}{channels}"))
            .collect();
        assert_eq!(names, expected);
    }
}

#[test]
fn core_sorted_by_name_writes_the_bytes_of_the_core_file() {
    for channels in CHANNELS {
        let mut defs = core(channels, Delay::Comb);
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        let written = write_def_file(&defs).unwrap_or_else(|error| panic!("{error}"));
        assert!(
            written == common::golden_bytes(&format!("core_{channels}")),
            "core({channels}, Comb) differs from core_{channels}.scsyndef"
        );
    }
}

#[cfg(feature = "sc3-plugins")]
#[test]
fn core_with_the_switch_delay_differs_in_dirt_delay_alone() {
    for channels in CHANNELS {
        let delay = format!("dirt_delay{channels}");
        let comb = core(channels, Delay::Comb);
        let switch = core(channels, Delay::Switch);
        assert_eq!(comb.len(), switch.len());
        for (comb, switch) in comb.iter().zip(&switch) {
            assert_eq!(comb.name, switch.name);
            if switch.name == delay {
                assert_ne!(comb, switch, "{delay}");
                common::assert_golden_bytes(&format!("switch_delay_{channels}"), switch);
            } else {
                assert_eq!(comb, switch, "{}", comb.name);
            }
        }
    }
}

#[test]
fn each_player_has_the_bytes_of_sclang_for_each_sample_channel_count_and_channel_count() {
    for (name, player) in PLAYERS {
        for sample_channels in SAMPLE_CHANNELS {
            for channels in CHANNELS {
                let def = player(sample_channels, channels)
                    .unwrap_or_else(|error| panic!("{name}_{sample_channels}_{channels}: {error}"));
                let file = if sample_channels <= 2 {
                    format!("core_{channels}")
                } else {
                    format!("players_3_4_{channels}")
                };
                common::assert_golden_bytes(&file, &def);
            }
        }
    }
}

#[test]
fn each_function_refuses_0_channels_and_more_than_65535_with_bad_count() {
    for (name, function) in FUNCTIONS {
        for channels in [0, 65_536] {
            let result = function(channels);
            assert!(
                matches!(result, Err(BuildError::BadCount { .. })),
                "{name}({channels}): {:?}",
                result.map(|def| def.name)
            );
        }
    }
}

#[test]
fn each_player_refuses_0_sample_channels_and_a_count_over_65535_with_bad_count() {
    let counts = [0, 65_536]
        .into_iter()
        .flat_map(|sample_channels| CHANNELS.map(|channels| (sample_channels, channels)))
        .chain(SAMPLE_CHANNELS.map(|sample_channels| (sample_channels, 65_536)));
    for (sample_channels, channels) in counts {
        for (name, player) in PLAYERS {
            let result = player(sample_channels, channels);
            assert!(
                matches!(result, Err(BuildError::BadCount { .. })),
                "{name}({sample_channels}, {channels}): {:?}",
                result.map(|def| def.name)
            );
        }
    }
}

#[test]
fn the_definitions_of_1_2_and_4_channels_take_their_measured_bytes_and_wire_buffers() {
    for (channels, total, (biggest, bytes), (busiest, wires)) in [
        (
            1,
            26_584,
            ("dirt_stretchsample_2_1", 2_547),
            ("dirt_stretchsample_2_1", 7),
        ),
        (2, 31_265, ("dirt_leslie2", 3_461), ("dirt_leslie2", 10)),
        (4, 39_779, ("dirt_reverb4", 5_080), ("dirt_leslie4", 11)),
    ] {
        let defs = core(channels, Delay::Comb);
        assert_eq!(defs.iter().map(size).sum::<usize>(), total, "{channels}");
        assert_eq!(largest(&defs, size), (vec![biggest], bytes), "{channels}");
        let wire_buffers = |def: &SynthDef| {
            def.wire_buffers()
                .unwrap_or_else(|error| panic!("{}: {error}", def.name))
        };
        assert_eq!(
            largest(&defs, wire_buffers),
            (vec![busiest], wires),
            "{channels}"
        );
    }
}

#[test]
fn each_definition_fits_one_datagram_up_to_7_channels_and_dirt_reverb8_alone_does_not() {
    for channels in 1..=7 {
        for def in core(channels, Delay::Comb) {
            assert!(size(&def) <= MAX_DATAGRAM, "{}: {}", def.name, size(&def));
        }
    }
    let defs = core(8, Delay::Comb);
    let over: Vec<(&str, usize)> = defs
        .iter()
        .filter(|def| size(def) > MAX_DATAGRAM)
        .map(|def| (def.name.as_str(), size(def)))
        .collect();
    assert_eq!(over, [("dirt_reverb8", 8_923)]);
}

#[test]
fn each_player_refuses_0_channels_with_not_a_number() {
    for (name, player) in PLAYERS {
        for sample_channels in SAMPLE_CHANNELS {
            let result = player(sample_channels, 0);
            assert!(
                matches!(result, Err(BuildError::NotANumber { .. })),
                "{name}({sample_channels}, 0): {:?}",
                result.map(|def| def.name)
            );
        }
    }
}
