//! `Dirt`: `new` refuses each fault of a `Config` with its variant, and
//! takes node ids and buses up to the largest i32. `start` returns one
//! `/d_recv` per definition of `core`, and the node bundle of orbit `k` rides
//! on the event definition `k mod 20`: on the wire, it holds the node
//! messages of the golden start of SuperDirt, but the group of SuperDirt
//! itself. Each message of `start` fits one datagram of sc-app3. `stop` frees
//! the nodes of each orbit in a bundle of its own. Orbit `k` takes the 6 node
//! ids from `fixed_nodes + 6 * k`, and its buses after those of orbit
//! `k - 1`: 3 buses of `channels` channels, and 4 for 1 channel. Each orbit
//! has the defaults of SuperDirt, and `set` replaces one and adds one.

mod common;

use std::error::Error;

use scsynth_dirt::synthdefs::{self, Delay};
use scsynth_dirt::{Config, ConfigError, Dirt, Orbit, Register, Value};
use scsynth_osc::command::group::{GNew, NewGroup};
use scsynth_osc::command::node::{NFree, NRun, NodeRun};
use scsynth_osc::command::synth::SNew;
use scsynth_osc::command::top_level::ErrorMode;
use scsynth_osc::{
    AddAction, Bundle, Command, Completion, ControlSetting, NewNodeId, NodeId, Packet, RawArg,
    RawMessage, Timetag,
};
use scsynth_synthdef::BuildError;

use common::events;
use common::wire::{self, Comparison};
use common::{CHANNELS, MAX_DATAGRAM};

/// A change that makes a valid configuration faulty.
type Fault = fn(&mut Config);

/// A configuration of `channels` channels and `orbits` orbits, which `new`
/// takes for up to 16 channels and 150 orbits.
fn valid(channels: u32, orbits: u32) -> Config {
    Config {
        channels,
        orbits,
        root: NodeId(1),
        fixed_nodes: 100,
        sound_nodes: 1000..2000,
        first_bus: 16,
        out_bus: 0,
        sample_rate: 48_000.0,
        delay: Delay::Comb,
        register: Register::default(),
        epoch: 0.0,
    }
}

/// The instance of `config`.
fn dirt(config: Config) -> Dirt {
    let shown = format!("{config:?}");
    Dirt::new(config).unwrap_or_else(|error| panic!("{shown}: {error}"))
}

/// Asserts that `new` refuses the configuration of 2 channels and 2 orbits
/// after each fault of `cases`, with the error of the fault.
fn assert_refuses(cases: &[(Fault, ConfigError)]) {
    for (fault, error) in cases {
        let mut config = valid(2, 2);
        fault(&mut config);
        let shown = format!("{config:?}");
        assert_eq!(Dirt::new(config).err().as_ref(), Some(error), "{shown}");
    }
}

/// The global synths of `orbit`, in the order of their node ids, each with
/// the name of its definition without the channel count.
fn synths(orbit: &Orbit) -> [(&'static str, NodeId); 5] {
    [
        ("monitor", orbit.monitor()),
        ("rms", orbit.rms()),
        ("leslie", orbit.leslie()),
        ("reverb", orbit.reverb()),
        ("delay", orbit.delay()),
    ]
}

/// `id` as the id of a new node.
fn new_node(id: NodeId) -> NewNodeId {
    NewNodeId::new(id.0).unwrap_or_else(|| panic!("{id:?} is no new node id"))
}

/// `number` as an int of OSC.
fn int<T: TryInto<i32> + Copy + std::fmt::Debug>(number: T) -> i32 {
    number
        .try_into()
        .unwrap_or_else(|_| panic!("{number:?} is no i32"))
}

/// The node messages of the orbit `index` of `dirt`: `/g_new group 0 root`,
/// then for each global synth
/// `/s_new name id 3 group outBus o dryBus d effectBus e orbitIndex k` and
/// `/n_run id 0`.
fn nodes(dirt: &Dirt, index: usize) -> Vec<Command> {
    let config = dirt.config();
    let orbit = dirt
        .orbit(index)
        .unwrap_or_else(|| panic!("no orbit {index}"));
    let controls = vec![
        ControlSetting::new("outBus", int(orbit.out_bus())),
        ControlSetting::new("dryBus", int(orbit.dry_bus())),
        ControlSetting::new("effectBus", int(orbit.effect_bus())),
        ControlSetting::new("orbitIndex", int(index)),
    ];
    let mut commands = vec![Command::GNew(GNew {
        groups: vec![NewGroup {
            id: new_node(orbit.group()),
            action: AddAction::Head,
            target: config.root,
        }],
    })];
    for (name, synth) in synths(orbit) {
        commands.push(Command::SNew(SNew {
            def: format!("dirt_{name}{}", config.channels),
            id: new_node(synth),
            action: AddAction::After,
            target: orbit.group(),
            controls: controls.clone(),
        }));
        commands.push(Command::NRun(NRun {
            nodes: vec![NodeRun {
                node: synth,
                run: false,
            }],
        }));
    }
    commands
}

/// The completion of the `/d_recv` `command`.
fn completion(command: &Command) -> Option<&Completion> {
    let Command::DRecv(recv) = command else {
        panic!("{command:?} is no /d_recv");
    };
    recv.completion.as_ref()
}

/// The completion bundle of the `/d_recv` `command` as its bytes decode: its
/// time tag and its raw messages.
fn raw_completion(command: &Command) -> (Timetag, Vec<RawMessage>) {
    let messages = wire::raw_messages(&Packet::Message(command.clone()));
    let [recv] = messages.as_slice() else {
        panic!("{messages:?}");
    };
    let [RawArg::Blob(_), RawArg::Blob(bytes)] = recv.args.as_slice() else {
        panic!("{recv:?} is no /d_recv with a completion");
    };
    match wire::decode(bytes) {
        Packet::Bundle(bundle) => (bundle.time, Packet::Bundle(bundle).into_messages()),
        Packet::Message(message) => panic!("the completion {message:?} is no bundle"),
    }
}

/// The defaults of an orbit in SuperDirt, by name.
fn superdirt_defaults() -> [(&'static str, Value); 18] {
    [
        ("cps", Value::Number(1.0)),
        ("offset", Value::Number(0.0)),
        ("begin", Value::Number(0.0)),
        ("end", Value::Number(1.0)),
        ("speed", Value::Number(1.0)),
        ("pan", Value::Number(0.5)),
        ("gain", Value::Number(1.0)),
        ("overgain", Value::Number(0.0)),
        ("cut", Value::Number(0.0)),
        ("unit", Value::from("r")),
        ("octave", Value::Number(5.0)),
        ("dur", Value::Number(1.0)),
        ("lag", Value::Number(0.0)),
        ("loop", Value::Number(1.0)),
        ("dry", Value::Number(0.0)),
        ("lock", Value::Number(0.0)),
        ("amp", Value::Number(0.4)),
        ("fadeTime", Value::Number(0.001)),
    ]
}

#[test]
fn the_node_messages_of_each_orbit_are_its_golden_start_but_the_group_of_superdirt() {
    for scenario in events::read().scenarios {
        let mut dirt = scenario.dirt();
        let commands = dirt.start();
        let mut comparison = Comparison::new(&scenario.name);
        for (index, golden) in scenario.start.iter().enumerate() {
            let (time, ours) = raw_completion(&commands[5 + index]);
            assert_eq!(time, Timetag::IMMEDIATELY, "{}", scenario.name);
            let golden = events::Bundle {
                time: golden.time,
                messages: golden.messages[1..].to_vec(),
            };
            comparison.assert_bundle(&format!("the start of orbit {index}"), &golden, &ours);
        }
    }
}

#[test]
fn start_returns_a_d_recv_of_the_bytes_of_each_definition_of_core_in_its_order() {
    for channels in CHANNELS {
        let mut dirt = dirt(valid(channels, 2));
        let core = synthdefs::core(channels, Delay::Comb)
            .unwrap_or_else(|error| panic!("core({channels}): {error}"));
        assert!(dirt.definitions() == core, "{channels} channels");
        let commands = dirt.start();
        assert_eq!(commands.len(), 25, "{channels} channels");
        for (command, definition) in commands.iter().zip(dirt.definitions()) {
            let Command::DRecv(recv) = command else {
                panic!("{command:?} is no /d_recv");
            };
            let bytes = definition
                .as_bytes()
                .unwrap_or_else(|error| panic!("{}: {error}", definition.name));
            assert!(recv.data == bytes, "{}", definition.name);
        }
    }
}

#[test]
fn with_2_orbits_the_event_definitions_5_and_6_carry_the_node_bundles_of_orbits_0_and_1() {
    let mut dirt = dirt(valid(2, 2));
    let commands = dirt.start();
    for (index, command) in commands.iter().enumerate() {
        let expected = match index {
            5 => Some(nodes(&dirt, 0)),
            6 => Some(nodes(&dirt, 1)),
            _ => None,
        };
        let expected = expected.map(|commands| Completion::Bundle {
            time: Timetag::IMMEDIATELY,
            commands,
        });
        assert_eq!(completion(command), expected.as_ref(), "definition {index}");
    }
}

#[test]
fn with_21_orbits_the_definition_5_carries_the_nodes_of_orbits_0_and_20_in_one_bundle() {
    let mut dirt = dirt(valid(2, 21));
    let commands = dirt.start();
    for (index, command) in commands.iter().enumerate() {
        let expected = match index {
            ..5 => None,
            5 => Some([nodes(&dirt, 0), nodes(&dirt, 20)].concat()),
            _ => Some(nodes(&dirt, index - 5)),
        };
        let expected = expected.map(|commands| Completion::Bundle {
            time: Timetag::IMMEDIATELY,
            commands,
        });
        assert_eq!(completion(command), expected.as_ref(), "definition {index}");
    }
}

#[test]
fn each_message_of_start_fits_one_datagram_for_1_2_and_4_channels_and_12_orbits() {
    for channels in CHANNELS {
        let mut dirt = dirt(valid(channels, 12));
        for (command, definition) in dirt.start().iter().zip(dirt.definitions()) {
            let size = command.encode().len();
            assert!(size <= MAX_DATAGRAM, "{}: {size} bytes", definition.name);
        }
    }
}

#[test]
fn stop_frees_the_group_and_the_global_synths_of_each_orbit_in_a_bundle_of_its_own() {
    let mut dirt = dirt(valid(2, 3));
    let expected: Vec<Bundle<Command>> = dirt
        .orbits()
        .iter()
        .map(|orbit| {
            let nodes = vec![
                orbit.group(),
                orbit.monitor(),
                orbit.rms(),
                orbit.leslie(),
                orbit.reverb(),
                orbit.delay(),
            ];
            Bundle {
                time: Timetag::IMMEDIATELY,
                packets: vec![
                    Packet::Message(Command::Error(ErrorMode::BundleOff)),
                    Packet::Message(Command::NFree(NFree { nodes })),
                    Packet::Message(Command::Error(ErrorMode::BundleOn)),
                ],
            }
        })
        .collect();
    assert_eq!(dirt.stop(), expected);
}

#[test]
fn orbit_k_takes_6_node_ids_from_fixed_nodes_plus_6_k_and_buses_after_orbit_k_minus_1() {
    for (channels, buses) in [
        (1, [[16, 17, 18], [20, 21, 22], [24, 25, 26]]),
        (2, [[16, 18, 20], [22, 24, 26], [28, 30, 32]]),
        (4, [[16, 20, 24], [28, 32, 36], [40, 44, 48]]),
    ] {
        let mut config = valid(channels, 3);
        config.out_bus = 2;
        let dirt = dirt(config);
        assert_eq!(dirt.orbits().len(), 3, "{channels} channels");
        for ((index, orbit), (first, buses)) in dirt
            .orbits()
            .iter()
            .enumerate()
            .zip([100, 106, 112].into_iter().zip(buses))
        {
            let ids = [
                orbit.group(),
                orbit.monitor(),
                orbit.rms(),
                orbit.leslie(),
                orbit.reverb(),
                orbit.delay(),
            ];
            let expected = [0, 1, 2, 3, 4, 5].map(|offset| NodeId(first + offset));
            assert_eq!(orbit.index(), index, "{channels} channels");
            assert_eq!(ids, expected, "{channels} channels: orbit {index}");
            let found = [orbit.synth_bus(), orbit.dry_bus(), orbit.effect_bus()];
            assert_eq!(found, buses, "{channels} channels: orbit {index}");
            assert_eq!(orbit.out_bus(), 2, "{channels} channels: orbit {index}");
        }
    }
}

#[test]
fn each_orbit_has_the_defaults_of_superdirt_and_no_n() {
    let dirt = dirt(valid(2, 2));
    for orbit in dirt.orbits() {
        for (name, value) in superdirt_defaults() {
            assert_eq!(orbit.get(name), Some(&value), "orbit {}", orbit.index());
        }
        assert_eq!(orbit.get("n"), None);
    }
}

#[test]
fn set_replaces_a_default_and_adds_a_name_on_its_orbit_alone() {
    let mut dirt = dirt(valid(2, 2));
    let orbit = dirt.orbit_mut(0).unwrap_or_else(|| panic!("no orbit 0"));
    orbit.set("amp", 0.6).set("unit", "c").set("room", 0.3);
    let changed = dirt.orbit(0).unwrap_or_else(|| panic!("no orbit 0"));
    assert_eq!(changed.get("amp"), Some(&Value::Number(0.6)));
    assert_eq!(changed.get("unit"), Some(&Value::from("c")));
    assert_eq!(changed.get("room"), Some(&Value::Number(0.3)));
    let other = dirt.orbit(1).unwrap_or_else(|| panic!("no orbit 1"));
    assert_eq!(other.get("amp"), Some(&Value::Number(0.4)));
    assert_eq!(other.get("unit"), Some(&Value::from("r")));
    assert_eq!(other.get("room"), None);
}

#[test]
fn orbit_and_orbit_mut_give_nothing_past_the_count() {
    let mut dirt = dirt(valid(2, 2));
    assert_eq!(dirt.orbit(1).map(Orbit::index), Some(1));
    assert_eq!(dirt.orbit(2), None);
    assert_eq!(dirt.orbit_mut(2), None);
}

#[test]
fn new_refuses_0_channels_and_0_orbits() {
    assert_refuses(&[
        (|config| config.channels = 0, ConfigError::NoChannels),
        (|config| config.orbits = 0, ConfigError::NoOrbits),
    ]);
}

#[test]
#[allow(
    clippy::reversed_empty_ranges,
    reason = "new must refuse a reversed range as empty"
)]
fn new_refuses_node_ids_under_1_a_root_under_0_and_an_empty_range_of_sound_nodes() {
    assert_refuses(&[
        (
            |config| config.fixed_nodes = 0,
            ConfigError::FixedNodesUnder1(0),
        ),
        (
            |config| config.fixed_nodes = -7,
            ConfigError::FixedNodesUnder1(-7),
        ),
        (
            |config| config.sound_nodes = 0..10,
            ConfigError::SoundNodesUnder1(0..10),
        ),
        (
            |config| config.sound_nodes = -3..5,
            ConfigError::SoundNodesUnder1(-3..5),
        ),
        (
            |config| config.sound_nodes = 5..5,
            ConfigError::EmptySoundNodes(5..5),
        ),
        (
            |config| config.sound_nodes = 9..3,
            ConfigError::EmptySoundNodes(9..3),
        ),
        (
            |config| config.sound_nodes = 0..0,
            ConfigError::EmptySoundNodes(0..0),
        ),
        (
            |config| config.root = NodeId(-1),
            ConfigError::RootUnder0(NodeId(-1)),
        ),
        (
            |config| config.root = NodeId(i32::MIN),
            ConfigError::RootUnder0(NodeId(i32::MIN)),
        ),
    ]);
}

#[test]
fn new_refuses_node_ids_that_overlap_with_the_root_or_with_the_lowest_shared_node_id() {
    assert_refuses(&[
        (
            |config| config.root = NodeId(100),
            ConfigError::RootInFixedNodes(NodeId(100)),
        ),
        (
            |config| config.root = NodeId(111),
            ConfigError::RootInFixedNodes(NodeId(111)),
        ),
        (
            |config| config.root = NodeId(1000),
            ConfigError::RootInSoundNodes(NodeId(1000)),
        ),
        (
            |config| config.root = NodeId(1999),
            ConfigError::RootInSoundNodes(NodeId(1999)),
        ),
        (
            |config| config.sound_nodes = 111..200,
            ConfigError::FixedNodeInSoundNodes(111),
        ),
        (
            |config| config.sound_nodes = 50..101,
            ConfigError::FixedNodeInSoundNodes(100),
        ),
        (
            |config| config.sound_nodes = 105..107,
            ConfigError::FixedNodeInSoundNodes(105),
        ),
    ]);
}

#[test]
fn new_refuses_output_buses_that_overlap_the_private_buses_with_the_lowest_shared_bus() {
    assert_refuses(&[
        (
            |config| config.out_bus = 27,
            ConfigError::OutBusInPrivateBuses(27),
        ),
        (
            |config| config.out_bus = 15,
            ConfigError::OutBusInPrivateBuses(16),
        ),
        (
            |config| config.first_bus = 1,
            ConfigError::OutBusInPrivateBuses(1),
        ),
    ]);
}

#[test]
fn new_refuses_node_ids_and_buses_past_the_largest_i32() {
    assert_refuses(&[
        (
            |config| config.fixed_nodes = 2_147_483_637,
            ConfigError::FixedNodesPastI32,
        ),
        (
            |config| config.orbits = u32::MAX,
            ConfigError::FixedNodesPastI32,
        ),
        (
            |config| config.first_bus = 2_147_483_637,
            ConfigError::PrivateBusesPastI32,
        ),
        (
            |config| config.first_bus = u32::MAX,
            ConfigError::PrivateBusesPastI32,
        ),
        (
            |config| config.channels = u32::MAX,
            ConfigError::PrivateBusesPastI32,
        ),
        (
            |config| {
                config.channels = 1;
                config.first_bus = 2_147_483_641;
            },
            ConfigError::PrivateBusesPastI32,
        ),
        (
            |config| config.out_bus = 2_147_483_647,
            ConfigError::OutBusesPastI32,
        ),
        (
            |config| config.out_bus = u32::MAX,
            ConfigError::OutBusesPastI32,
        ),
    ]);
}

#[test]
fn new_refuses_a_sample_rate_that_is_no_finite_number_above_0_and_an_epoch_that_is_not_finite() {
    assert_refuses(&[
        (
            |config| config.sample_rate = 0.0,
            ConfigError::BadSampleRate(0.0),
        ),
        (
            |config| config.sample_rate = -48_000.0,
            ConfigError::BadSampleRate(-48_000.0),
        ),
        (
            |config| config.sample_rate = f64::INFINITY,
            ConfigError::BadSampleRate(f64::INFINITY),
        ),
        (
            |config| config.epoch = f64::INFINITY,
            ConfigError::BadEpoch(f64::INFINITY),
        ),
        (
            |config| config.epoch = f64::NEG_INFINITY,
            ConfigError::BadEpoch(f64::NEG_INFINITY),
        ),
    ]);
    let mut config = valid(2, 2);
    config.sample_rate = f64::NAN;
    let error = Dirt::new(config).err();
    assert!(
        matches!(error, Some(ConfigError::BadSampleRate(rate)) if rate.is_nan()),
        "{error:?}"
    );
    let mut config = valid(2, 2);
    config.epoch = f64::NAN;
    let error = Dirt::new(config).err();
    assert!(
        matches!(error, Some(ConfigError::BadEpoch(epoch)) if epoch.is_nan()),
        "{error:?}"
    );
}

#[test]
fn new_refuses_more_channels_than_the_builder_takes_with_the_error_of_the_build() {
    let mut config = valid(65_536, 1);
    config.first_bus = 100_000;
    let error = Dirt::new(config).err();
    assert!(
        matches!(
            &error,
            Some(ConfigError::Definition(BuildError::BadCount { .. }))
        ),
        "{error:?}"
    );
}

#[cfg(not(feature = "sc3-plugins"))]
#[test]
fn new_refuses_the_switch_delay_without_the_feature_sc3_plugins_with_the_error_of_the_build() {
    let mut config = valid(2, 2);
    config.delay = Delay::Switch;
    let error = Dirt::new(config).err();
    assert!(
        matches!(
            &error,
            Some(ConfigError::Definition(BuildError::UnknownClass(class))) if class == "SwitchDelay"
        ),
        "{error:?}"
    );
}

#[cfg(feature = "sc3-plugins")]
#[test]
fn new_takes_the_switch_delay_with_the_feature_sc3_plugins() {
    let mut config = valid(2, 2);
    config.delay = Delay::Switch;
    let dirt = dirt(config);
    let delay = synthdefs::dirt_delay(2, Delay::Switch)
        .unwrap_or_else(|error| panic!("dirt_delay(2, Switch): {error}"));
    let built = dirt.definitions().iter().find(|def| def.name == delay.name);
    assert!(built == Some(&delay), "{:?}", built.map(|def| &def.name));
}

#[test]
fn new_takes_node_ids_and_buses_up_to_the_largest_i32() {
    let mut config = valid(2, 2);
    config.fixed_nodes = 2_147_483_636;
    let orbits = dirt(config).orbits().to_vec();
    assert_eq!(orbits[1].delay(), NodeId(i32::MAX));
    for (channels, first_bus, effect_bus) in [
        (2, 2_147_483_636, 2_147_483_646),
        (1, 2_147_483_640, 2_147_483_646),
    ] {
        let mut config = valid(channels, 2);
        config.first_bus = first_bus;
        let orbits = dirt(config).orbits().to_vec();
        assert_eq!(orbits[1].effect_bus(), effect_bus, "{channels} channels");
    }
    let mut config = valid(2, 2);
    config.out_bus = 2_147_483_646;
    let orbits = dirt(config).orbits().to_vec();
    assert_eq!(orbits[0].out_bus(), 2_147_483_646);
}

#[test]
fn an_error_says_what_is_wrong() {
    for (error, message) in [
        (
            ConfigError::NoChannels,
            "the channel count is 0, but an orbit has 1 channel or more",
        ),
        (
            ConfigError::NoOrbits,
            "the orbit count is 0, but an instance has 1 orbit or more",
        ),
        (
            ConfigError::FixedNodesUnder1(0),
            "the node ids of the orbits start at 0, but a node id of the instance is 1 or more",
        ),
        (
            ConfigError::SoundNodesUnder1(-3..5),
            "the range -3..5 of node ids of the sounds starts under 1, but a node id of the instance is 1 or more",
        ),
        (
            ConfigError::EmptySoundNodes(5..5),
            "the range 5..5 of node ids of the sounds is empty",
        ),
        (
            ConfigError::RootUnder0(NodeId(-1)),
            "the root group -1 is under 0, but the id of a group is 0 or more",
        ),
        (
            ConfigError::RootInFixedNodes(NodeId(100)),
            "the root group 100 is one of the node ids of the orbits",
        ),
        (
            ConfigError::RootInSoundNodes(NodeId(1000)),
            "the root group 1000 is in the range of node ids of the sounds",
        ),
        (
            ConfigError::FixedNodeInSoundNodes(105),
            "the node id 105 is one of the orbits, and in the range of node ids of the sounds",
        ),
        (
            ConfigError::OutBusInPrivateBuses(16),
            "the bus 16 is an output bus, and a private bus of an orbit",
        ),
        (
            ConfigError::FixedNodesPastI32,
            "the node ids of the orbits pass the largest i32",
        ),
        (
            ConfigError::PrivateBusesPastI32,
            "the private buses of the orbits pass the largest i32",
        ),
        (
            ConfigError::OutBusesPastI32,
            "the output buses pass the largest i32",
        ),
        (
            ConfigError::BadSampleRate(-48_000.0),
            "the sample rate -48000 is not a finite number above 0",
        ),
        (
            ConfigError::BadEpoch(f64::NAN),
            "the epoch NaN is not finite",
        ),
    ] {
        assert_eq!(error.to_string(), message);
        assert!(error.source().is_none(), "{error}");
    }
    let build = BuildError::UnknownClass("SwitchDelay".to_owned());
    let error = ConfigError::Definition(build.clone());
    assert_eq!(
        error.to_string(),
        "a synth definition does not build for the channel count and the delay"
    );
    assert_eq!(
        error.source().map(ToString::to_string),
        Some(build.to_string())
    );
}
