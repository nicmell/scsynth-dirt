//! The golden events of `events.json`: on an instance and a library of each
//! scenario, `play` sends for each sound the bundles that SuperDirt sends,
//! message by message, with the time tag of the golden time at `epoch` 0. In
//! a cut bundle, the `cut_gate` messages compare as a set. A sound that
//! SuperDirt drops, `play` drops.

mod common;

use scsynth_dirt::{Dirt, Library, Sound, Value};
use scsynth_osc::reply::buffer::{BInfo, BufferInfo};
use scsynth_osc::{BufferId, Command, Completion, Packet, RawMessage, Reply, Timetag};

use common::events::{self, Bundle, Scenario, Step};
use common::wire::{self, Comparison};

/// The count of the sounds of `events.json`, of their bundles, and of the
/// sounds that SuperDirt drops.
const COUNTS: (usize, usize, usize) = (132, 141, 2);

/// The library of `scenario`: its names in the order of their adds, each
/// sample on the buffer of the scenario, and loaded with the numbers of the
/// scenario.
fn library(scenario: &Scenario) -> Library {
    let samples = || scenario.library.iter().flat_map(|entry| &entry.samples);
    let first = samples()
        .map(|sample| sample.buffer)
        .min()
        .expect("a sample");
    let mut library = Library::new(first..first + 1000).expect("a range of buffers");
    for entry in &scenario.library {
        let paths = (0..entry.samples.len()).map(|index| format!("{}/{index}.wav", entry.name));
        library.add(&entry.name, paths).expect("free buffers");
        let buffers: Vec<i32> = library
            .samples(&entry.name)
            .unwrap_or_default()
            .iter()
            .map(|sample| sample.buffer().get())
            .collect();
        let golden: Vec<i32> = entry.samples.iter().map(|sample| sample.buffer).collect();
        assert_eq!(buffers, golden, "{}: {}", scenario.name, entry.name);
    }
    for sample in samples() {
        library.reply(&Reply::BInfo(BInfo {
            buffers: vec![BufferInfo {
                buffer: BufferId::new(sample.buffer).expect("a buffer number"),
                frames: i32::try_from(sample.frames).expect("frames of an int"),
                channels: i32::try_from(sample.channels).expect("channels of an int"),
                sample_rate: f32_of(sample.sample_rate),
            }],
        }));
    }
    library
}

/// `number`, a sample rate of `events.json`, as the float of `/b_info`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the sample rates of events.json are whole numbers that an f32 holds"
)]
fn f32_of(number: f64) -> f32 {
    number as f32
}

/// The value of the crate for the golden value `value` of a parameter: a
/// number for an Integer, a Float or a Boolean, which is 1 for true.
#[allow(
    clippy::cast_precision_loss,
    reason = "the Integers of events.json are far below 2 ** 53"
)]
fn value(value: &events::Value) -> Value {
    match value {
        events::Value::Int(number) => Value::Number(*number as f64),
        events::Value::Float(number) => Value::Number(*number),
        events::Value::Bool(flag) => Value::Number(if *flag { 1.0 } else { 0.0 }),
        events::Value::Text(text) => Value::from(text.as_str()),
        other => panic!("{other:?} is no value of a parameter"),
    }
}

/// The sound of the crate for the golden sound `golden`.
fn sound(golden: &events::Sound) -> Sound {
    golden
        .parameters
        .iter()
        .fold(Sound::new().at(golden.when), |sound, (name, parameter)| {
            sound.with(name.clone(), value(parameter))
        })
}

/// The time tag of the golden time `seconds` at `epoch` 0: the whole
/// seconds, and the fraction in units of 2 ** -32, rounded down.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the golden times lie in [0, 2 ** 32), and the cast of the fraction rounds it down"
)]
fn timetag(seconds: f64) -> Timetag {
    let whole = seconds.floor();
    Timetag {
        seconds: whole as u32,
        fraction: ((seconds - whole) * 4_294_967_296.0) as u32,
    }
}

/// The raw messages of the node bundle of the `/d_recv` `command`.
fn node_messages(command: &Command) -> Vec<RawMessage> {
    let Command::DRecv(recv) = command else {
        panic!("{command:?} is no /d_recv");
    };
    let Some(Completion::Bundle { time, commands }) = &recv.completion else {
        panic!("the /d_recv has no completion bundle");
    };
    let packets = commands.iter().cloned().map(Packet::Message).collect();
    wire::raw_messages(&Packet::Bundle(scsynth_osc::Bundle {
        time: *time,
        packets,
    }))
}

/// Plays the golden sound `golden` on `dirt`, and asserts that it sends the
/// golden bundles `bundles`, or drops without them. Returns the count of the
/// bundles.
fn assert_sound(
    comparison: &mut Comparison,
    dirt: &mut Dirt,
    library: &Library,
    golden: &events::Sound,
    bundles: &[Bundle],
) -> usize {
    let label = format!("the sound at {} {:?}", golden.when, golden.parameters);
    let played = dirt.play(library, &sound(golden));
    if bundles.is_empty() {
        assert!(played.is_err(), "{label} plays: {played:?}");
        return 0;
    }
    let ours = played.unwrap_or_else(|dropped| panic!("{label} drops: {dropped}"));
    assert_eq!(
        ours.len(),
        bundles.len(),
        "{label}: the counts of the bundles"
    );
    for (index, (golden, ours)) in bundles.iter().zip(&ours).enumerate() {
        let label = format!("{label}, bundle {index}");
        let time = golden.time.expect("a bundle of a sound has a time");
        assert_eq!(ours.time, timetag(time), "{label}: the time tag of {time}");
        let messages = wire::raw_messages(&Packet::Bundle(ours.clone()));
        if index + 1 < bundles.len() {
            comparison.assert_bundle_unordered(&label, golden, &messages, 1);
        } else {
            comparison.assert_bundle(&label, golden, &messages);
        }
    }
    bundles.len()
}

#[test]
fn each_sound_sends_the_golden_bundles_of_superdirt_or_drops_as_superdirt_does() {
    let (mut sounds, mut bundles, mut drops) = (0, 0, 0);
    for scenario in events::read().scenarios {
        let mut dirt = scenario.dirt();
        let library = library(&scenario);
        let mut comparison = Comparison::new(&scenario.name);
        let commands = dirt.start();
        for (index, golden) in scenario.start.iter().enumerate() {
            let golden = Bundle {
                time: golden.time,
                messages: golden.messages[1..].to_vec(),
            };
            let ours = node_messages(&commands[5 + index]);
            comparison.assert_bundle(&format!("the start of orbit {index}"), &golden, &ours);
        }
        for step in &scenario.steps {
            match step {
                Step::Set { set } => {
                    let orbit = dirt.orbit_mut(set.orbit).expect("the orbit of a default");
                    orbit.set(set.name.clone(), value(&set.value));
                }
                Step::Sound {
                    sound,
                    bundles: golden,
                } => {
                    let sent = assert_sound(&mut comparison, &mut dirt, &library, sound, golden);
                    sounds += 1;
                    bundles += sent;
                    drops += usize::from(sent == 0);
                }
            }
        }
    }
    assert_eq!((sounds, bundles, drops), COUNTS);
}
