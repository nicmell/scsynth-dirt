//! The definitions of `synthdefs`, against the golden definitions that
//! sclang 3.14.1 wrote for SuperDirt: each definition has the bytes of the
//! golden definition of its name, for 1, 2 and 4 channels, and each sample
//! player for samples of 1 to 4 channels. Each function refuses a count
//! that the builder refuses, but a player refuses 0 channels with
//! `NotANumber`.

mod common;

use scsynth_dirt::synthdefs;
use scsynth_synthdef::{BuildError, SynthDef};

use common::CHANNELS;

/// A public function of `synthdefs` that builds a definition for a count of
/// channels.
type Function = fn(u32) -> Result<SynthDef, BuildError>;

/// Each public function of `synthdefs`, with its name.
const FUNCTIONS: &[(&str, Function)] = &[
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
