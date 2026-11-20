//! The golden events of `tests/fixtures/events.json`, as
//! `generator/events.scd` wrote them with sclang 3.14.1: the scenarios of the
//! plan in order, each on 2 orbits with the library of the plan. A start
//! holds one bundle per orbit with no time, which makes two groups first.
//! Each bundle of a sound has a time, a sound with a cut sends a first bundle
//! of its own, and each sound that SuperDirt plays ends with its gate. The
//! reader reads a float that the default parse of `serde_json` gets wrong to
//! the bits of its decimal text.

mod common;

use std::fs;

use common::events::{self, Bundle, Scenario, Sound, Step, Value};

/// The decimal text of a float of `events.json` that the default parse of
/// `serde_json` reads 1 ULP off (measured): the `fadeTime` of the gate of the
/// sound with `sustain` 0.004 in the scenario `span`.
const INEXACT: &str = "0.00076392000000000007";

/// The scenarios of the plan, in order. The scenario of the channels runs on
/// an instance of 1 channel, then on one of 4.
const SCENARIOS: [&str; 12] = [
    "start",
    "names",
    "time",
    "span",
    "pitch",
    "level_and_place",
    "modules",
    "global_synths",
    "cut",
    "orbits",
    "channels_1",
    "channels_4",
];

/// The sounds of `scenario` that SuperDirt plays, each with its bundles.
fn played(scenario: &Scenario) -> impl Iterator<Item = (&Sound, &[Bundle])> {
    scenario.steps.iter().filter_map(|step| match step {
        Step::Sound { sound, bundles } if !bundles.is_empty() => Some((sound, bundles.as_slice())),
        _ => None,
    })
}

/// Whether the parameters of `sound` hold a `cut` that is not 0.
fn cuts(sound: &Sound) -> bool {
    match sound.parameters.get("cut") {
        Some(&Value::Int(cut)) => cut != 0,
        Some(&Value::Float(cut)) => cut != 0.0,
        _ => false,
    }
}

#[test]
fn the_file_holds_the_scenarios_of_the_plan_in_order() {
    let names: Vec<String> = events::read()
        .scenarios
        .into_iter()
        .map(|scenario| scenario.name)
        .collect();
    assert_eq!(names, SCENARIOS);
}

#[test]
fn each_scenario_runs_on_2_orbits_with_the_library_of_the_plan() {
    for scenario in events::read().scenarios {
        let channels = match scenario.name.as_str() {
            "channels_1" => 1,
            "channels_4" => 4,
            _ => 2,
        };
        assert_eq!(scenario.config.channels, channels, "{}", scenario.name);
        assert_eq!(scenario.config.orbits, 2, "{}", scenario.name);
        let library: Vec<(&str, Vec<u32>)> = scenario
            .library
            .iter()
            .map(|entry| {
                let channels = entry.samples.iter().map(|sample| sample.channels);
                (entry.name.as_str(), channels.collect())
            })
            .collect();
        assert_eq!(
            library,
            [
                ("bd", vec![1, 1]),
                ("sn", vec![2]),
                ("long", vec![1]),
                ("x_bd", vec![1]),
            ],
            "{}",
            scenario.name
        );
        let long = &scenario.library[2].samples[0];
        assert!(long.frames > 16_777_216, "{}: {long:?}", scenario.name);
    }
}

#[test]
fn each_start_holds_one_bundle_per_orbit_with_no_time_that_makes_two_groups_first() {
    for scenario in events::read().scenarios {
        assert_eq!(
            u32::try_from(scenario.start.len()),
            Ok(scenario.config.orbits),
            "{}",
            scenario.name
        );
        for bundle in &scenario.start {
            assert_eq!(bundle.time, None, "{}", scenario.name);
            let first: Vec<&str> = bundle
                .messages
                .iter()
                .take(2)
                .map(|message| message.address.as_str())
                .collect();
            assert_eq!(first, ["/g_new", "/g_new"], "{}", scenario.name);
        }
    }
}

#[test]
fn each_address_starts_with_a_slash() {
    for scenario in events::read().scenarios {
        let sounds = scenario.steps.iter().filter_map(|step| match step {
            Step::Sound { bundles, .. } => Some(bundles),
            Step::Set { .. } => None,
        });
        for bundle in scenario.start.iter().chain(sounds.flatten()) {
            for message in &bundle.messages {
                assert!(
                    message.address.starts_with('/'),
                    "{}: {message:?}",
                    scenario.name
                );
            }
        }
    }
}

#[test]
fn each_bundle_of_a_sound_has_a_time_and_a_message() {
    for scenario in events::read().scenarios {
        for (sound, bundles) in played(&scenario) {
            for bundle in bundles {
                assert!(
                    bundle.time.is_some() && !bundle.messages.is_empty(),
                    "{}: {sound:?}: {bundle:?}",
                    scenario.name
                );
            }
        }
    }
}

#[test]
fn a_played_sound_with_a_cut_sends_a_first_bundle_that_starts_with_error_minus_1() {
    for scenario in events::read().scenarios {
        for (sound, bundles) in played(&scenario) {
            if cuts(sound) {
                assert_eq!(bundles.len(), 2, "{}: {sound:?}", scenario.name);
                let first = &bundles[0].messages[0];
                assert_eq!(
                    (first.address.as_str(), first.args.as_slice()),
                    ("/error", [Value::Int(-1)].as_slice()),
                    "{}: {sound:?}",
                    scenario.name
                );
            } else {
                assert_eq!(bundles.len(), 1, "{}: {sound:?}", scenario.name);
            }
        }
    }
}

#[test]
fn the_reader_reads_a_float_to_the_bits_of_its_decimal_text() {
    let text = fs::read_to_string(common::fixture("events.json")).expect("events.json reads");
    assert!(text.contains(&format!("\"fadeTime\", {INEXACT}]")));
    let span = events::read()
        .scenarios
        .into_iter()
        .find(|scenario| scenario.name == "span")
        .expect("the scenario span");
    let sustain = Value::Float(0.004);
    let gate = span
        .steps
        .iter()
        .find_map(|step| match step {
            Step::Sound { sound, bundles } if sound.parameters.get("sustain") == Some(&sustain) => {
                bundles.last()?.messages.last()
            }
            _ => None,
        })
        .expect("the gate of the sound with sustain 0.004");
    let name = Value::Text("fadeTime".to_owned());
    let index = gate.args.iter().position(|arg| *arg == name);
    let Some(&Value::Float(fade_time)) = index.and_then(|index| gate.args.get(index + 1)) else {
        panic!("{gate:?}");
    };
    let exact: f64 = INEXACT.parse().expect("a float");
    assert_eq!(
        fade_time.to_bits(),
        exact.to_bits(),
        "{fade_time} against {exact}"
    );
}

#[test]
fn each_played_sound_ends_with_its_gate_without_the_argument_sample() {
    for scenario in events::read().scenarios {
        let gate = Value::Text(format!("dirt_gate{}", scenario.config.channels));
        for (sound, bundles) in played(&scenario) {
            let last = bundles.last().and_then(|bundle| bundle.messages.last());
            let last = last.unwrap_or_else(|| panic!("{}: {sound:?}", scenario.name));
            assert_eq!(last.address, "/s_new", "{}: {sound:?}", scenario.name);
            assert_eq!(
                last.args.first(),
                Some(&gate),
                "{}: {sound:?}",
                scenario.name
            );
            assert!(
                !last.args.contains(&Value::Text("sample".to_owned())),
                "{}: {last:?}",
                scenario.name
            );
        }
    }
}
