//! The definitions of `synthdefs`, against the golden definitions that
//! sclang 3.14.1 wrote for SuperDirt: each definition has the bytes of the
//! golden definition of its name, for 1, 2 and 4 channels. Each function
//! refuses a count of channels that the builder refuses.

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
];

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
