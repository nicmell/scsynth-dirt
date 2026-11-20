//! `play` and `reply`, beyond the golden events: each drop gives its
//! reason, and changes nothing in the instance. A `vowel` that is a number
//! plays no vowel. Without `when`, a sound plays at once, and with a Unix
//! epoch, its time tag counts from 1900. A live sound leaves at the `/n_end`
//! of its group, or once a later sound starts after its end. The groups take
//! the node ids of `sound_nodes` in turn, and the cut gates come in the order
//! of the starts. `start` and `stop` forget the state of the sounds.

use std::error::Error;
use std::ops::Range;

use scsynth_dirt::{Config, Delay, Dirt, Dropped, Library, Register, Sound};
use scsynth_osc::reply::buffer::{BInfo, BufferInfo};
use scsynth_osc::reply::command_replies::Fail;
use scsynth_osc::reply::node_notification::{NodeKind, NodeState};
use scsynth_osc::{BufferId, Bundle, Command, Control, NodeId, Packet, Reply, Timetag};

/// The configuration of the tests: 2 channels and 2 orbits, with the node
/// ids `sound_nodes` for the sounds, and the epoch 0.
fn config(sound_nodes: Range<i32>) -> Config {
    Config {
        channels: 2,
        orbits: 2,
        root: NodeId(1),
        fixed_nodes: 100,
        sound_nodes,
        first_bus: 16,
        out_bus: 0,
        sample_rate: 48_000.0,
        delay: Delay::Comb,
        register: Register::default(),
        epoch: 0.0,
    }
}

/// An instance of `config`.
fn dirt(config: Config) -> Dirt {
    Dirt::new(config).expect("a valid configuration")
}

/// An instance of the configuration of the tests, with the node ids from
/// 2000 for the sounds.
fn fresh() -> Dirt {
    dirt(config(2000..3000))
}

/// `/b_info` of one buffer.
fn b_info(buffer: i32, frames: i32, channels: i32, sample_rate: f32) -> Reply {
    Reply::BInfo(BInfo {
        buffers: vec![BufferInfo {
            buffer: BufferId::new(buffer).expect("a buffer number"),
            frames,
            channels,
            sample_rate,
        }],
    })
}

/// The library of the tests: `bd` with 2 mono samples, `sn` with a stereo
/// one, `wide` with one of 3 channels, `waiting` with one that has no
/// `/b_info`, `failed` with one that scsynth could not read, and `empty`
/// with none.
fn library() -> Library {
    let mut library = Library::new(0..100).expect("a range of buffers");
    for (name, paths) in [
        ("bd", vec!["bd/0.wav", "bd/1.wav"]),
        ("sn", vec!["sn/0.wav"]),
        ("wide", vec!["wide/0.wav"]),
        ("waiting", vec!["waiting/0.wav"]),
        ("failed", vec!["failed/0.wav"]),
        ("empty", vec![]),
    ] {
        library.add(name, paths).expect("free buffers");
    }
    for reply in [
        b_info(0, 22_050, 1, 44_100.0),
        b_info(1, 11_025, 1, 44_100.0),
        b_info(2, 48_000, 2, 48_000.0),
        b_info(3, 48_000, 3, 48_000.0),
        Reply::Fail(Fail {
            command: "/b_allocRead".to_owned(),
            message: "File 'failed/0.wav' could not be opened: System error\n".to_owned(),
            value: Some(5),
        }),
    ] {
        library.reply(&reply);
    }
    library
}

/// The bundles of `sound` on `dirt`, which plays it.
fn play(dirt: &mut Dirt, sound: &Sound) -> Vec<Bundle<Command>> {
    dirt.play(&library(), sound)
        .unwrap_or_else(|dropped| panic!("{sound:?} drops: {dropped}"))
}

/// The messages of `bundle`, in order.
fn messages(bundle: &Bundle<Command>) -> Vec<Command> {
    Packet::Bundle(bundle.clone()).into_messages()
}

/// The group of the sound of `bundles`: the id of the `/g_new` of the last
/// bundle.
fn group(bundles: &[Bundle<Command>]) -> i32 {
    let last = bundles.last().expect("a bundle");
    messages(last)
        .iter()
        .find_map(|command| match command {
            Command::GNew(new) => Some(new.groups[0].id.get()),
            _ => None,
        })
        .expect("a /g_new")
}

/// The groups that the cut of the sound of `bundles` stops: the node of
/// each `/n_set` of its first bundle, in order. None for a sound with one
/// bundle.
fn cut_gates(bundles: &[Bundle<Command>]) -> Option<Vec<i32>> {
    let [cut, _] = bundles else {
        return None;
    };
    let nodes = messages(cut)
        .iter()
        .filter_map(|command| match command {
            Command::NSet(set) => Some(set.node.0),
            _ => None,
        })
        .collect();
    Some(nodes)
}

/// The definitions of the synths of the sound of `bundles`, in order.
fn definitions(bundles: &[Bundle<Command>]) -> Vec<String> {
    let last = bundles.last().expect("a bundle");
    messages(last)
        .into_iter()
        .filter_map(|command| match command {
            Command::SNew(new) => Some(new.def),
            _ => None,
        })
        .collect()
}

/// The names of the parameters that the last bundle of `bundles` sets on
/// `node`, in order, but those of a resume.
fn set_parameters(bundles: &[Bundle<Command>], node: NodeId) -> Vec<String> {
    let last = bundles.last().expect("a bundle");
    messages(last)
        .into_iter()
        .filter_map(|command| match command {
            Command::NSet(set) if set.node == node => Some(set.controls),
            _ => None,
        })
        .flatten()
        .filter_map(|setting| match setting.control {
            Control::Name(name) if name != "pauseImmediately" && name != "dirt_resumed" => {
                Some(name)
            }
            _ => None,
        })
        .collect()
}

/// The node of the group `group`, as a node notification of scsynth gives
/// it.
fn group_state(group: i32) -> NodeState {
    NodeState {
        node: NodeId(group),
        parent: Some(NodeId(100)),
        prev: None,
        next: None,
        kind: NodeKind::Group {
            head: None,
            tail: None,
        },
    }
}

/// `/n_end` of the group `group`.
fn n_end(group: i32) -> Reply {
    Reply::NEnd(group_state(group))
}

/// A sound at `when` without a name, which cuts the sounds of cut group 1,
/// and sets the parameters of the delay.
fn nameless(when: f64) -> Sound {
    Sound::new().at(when).cut(1).delay(0.5).delaytime(0.25)
}

/// The sound of `bd` that [`nameless`] gives.
fn cutting(when: f64) -> Sound {
    nameless(when).s("bd")
}

/// The sounds that drop, each with its reason, but those of a NaN, whose
/// reason equals no reason.
fn drops() -> Vec<(Sound, Dropped)> {
    let wrong = |name: &str| Dropped::WrongKind(name.to_owned());
    vec![
        (cutting(1.0).s(""), Dropped::Silence),
        (nameless(1.0), Dropped::NoName),
        (cutting(1.0).s("hh"), Dropped::UnknownSound("hh".to_owned())),
        (
            cutting(1.0).s("empty"),
            Dropped::UnknownSound("empty".to_owned()),
        ),
        (
            cutting(1.0).s("waiting"),
            Dropped::NotLoaded("waiting".to_owned()),
        ),
        (
            cutting(1.0).s("failed"),
            Dropped::NotLoaded("failed".to_owned()),
        ),
        (cutting(1.0).s("wide"), Dropped::TooManyChannels(3)),
        (cutting(1.0).unit("x"), Dropped::BadUnit("x".to_owned())),
        (cutting(1.0).unit("C"), Dropped::BadUnit("C".to_owned())),
        (
            cutting(1.0).with("sustain", 0.0001),
            Dropped::TooShort(0.0001 - 0.0001 * 0.19098),
        ),
        (cutting(1.0).with("speed", "fast"), wrong("speed")),
        (cutting(1.0).with("s", 3), wrong("s")),
        (cutting(1.0).with("orbit", "x"), wrong("orbit")),
        (cutting(1.0).with("unit", 1), wrong("unit")),
        (cutting(1.0).with("gain", "x"), wrong("gain")),
        (cutting(1.0).with("shape", "x"), wrong("shape")),
        (cutting(1.0).with("delaytime", "x"), wrong("delaytime")),
        (cutting(1.0).with("cutAll", "x"), wrong("cutAll")),
        (cutting(f64::INFINITY), Dropped::BadTime(f64::INFINITY)),
        (cutting(-1.0), Dropped::BadTime(-1.0)),
        (cutting(1.0).with("lag", -2.0), Dropped::BadTime(-1.0)),
        (
            cutting(f64::MAX).with("lag", f64::MAX),
            Dropped::BadTime(f64::INFINITY),
        ),
    ]
}

/// A test of the reason of a drop.
type Reason = fn(&Dropped) -> bool;

/// The sounds that drop for a NaN, each with a test of its reason.
fn nan_drops() -> Vec<(Sound, Reason)> {
    vec![
        (
            cutting(f64::NAN),
            |dropped| matches!(dropped, Dropped::BadTime(time) if time.is_nan()),
        ),
        (
            cutting(1.0).with("sustain", f64::NAN),
            |dropped| matches!(dropped, Dropped::TooShort(sustain) if sustain.is_nan()),
        ),
    ]
}

#[test]
fn each_drop_gives_its_reason() {
    for (sound, dropped) in drops() {
        assert_eq!(fresh().play(&library(), &sound), Err(dropped), "{sound:?}");
    }
    for (sound, test) in nan_drops() {
        let played = fresh().play(&library(), &sound);
        assert!(played.as_ref().is_err_and(test), "{sound:?}: {played:?}");
    }
}

#[test]
fn a_drop_changes_nothing_so_the_next_sound_plays_as_on_a_fresh_instance() {
    // A sound of `bd` at 1.0 ends at 1.5, after the next one starts.
    let (before, next) = (cutting(1.0), cutting(1.2));
    let alone = play(&mut fresh(), &next);
    assert_eq!(cut_gates(&alone), Some(vec![]));
    let mut dirt = fresh();
    let live = group(&play(&mut dirt, &before));
    let after_live = play(&mut dirt, &next);
    assert_eq!(cut_gates(&after_live), Some(vec![live]));
    let sounds = drops().into_iter().map(|(sound, _)| sound);
    for sound in sounds.chain(nan_drops().into_iter().map(|(sound, _)| sound)) {
        let mut dirt = fresh();
        assert!(dirt.play(&library(), &sound).is_err(), "{sound:?}");
        assert_eq!(play(&mut dirt, &next), alone, "after {sound:?}");
        let mut dirt = fresh();
        let _ = play(&mut dirt, &before);
        assert!(dirt.play(&library(), &sound).is_err(), "{sound:?}");
        assert_eq!(play(&mut dirt, &next), after_live, "after {sound:?}");
    }
}

#[test]
fn a_vowel_that_is_a_number_plays_without_the_vowel_synth() {
    let sound = Sound::new().at(1.0).s("bd");
    let numbered = play(&mut fresh(), &sound.clone().with("vowel", 1.0));
    assert_eq!(definitions(&numbered), ["dirt_sample_1_2", "dirt_gate2"]);
    assert_eq!(numbered, play(&mut fresh(), &sound));
    let voiced = play(&mut fresh(), &sound.vowel("a"));
    let expected = ["dirt_sample_1_2", "dirt_vowel2", "dirt_gate2"];
    assert_eq!(definitions(&voiced), expected);
}

#[test]
fn without_when_a_sound_plays_at_once_and_lag_and_offset_change_nothing() {
    let sound = Sound::new().s("bd").cut(1);
    let bundles = play(&mut fresh(), &sound);
    for bundle in &bundles {
        assert_eq!(bundle.time, Timetag::IMMEDIATELY);
    }
    let shifted = sound.with("lag", 0.5).offset(0.25).speed(2.0);
    let expected = play(&mut fresh(), &Sound::new().s("bd").cut(1).speed(2.0));
    assert_eq!(play(&mut fresh(), &shifted), expected);
}

#[test]
fn with_a_unix_epoch_the_time_tag_counts_from_1900_modulo_2_pow_32() {
    let mut config = config(2000..3000);
    config.epoch = 2_208_988_800.0;
    for (sound, seconds, fraction) in [
        // 1_700_000_000 + 2_208_988_800, and 0.25 of 2 ** 32.
        (Sound::new().at(1_700_000_000.25), 3_908_988_800, 1 << 30),
        // 0.25 + 0.5 of lag, and 0.75 of 2 ** 32.
        (
            Sound::new().at(1_700_000_000.25).with("lag", 0.5),
            3_908_988_800,
            3 << 30,
        ),
        // 2_100_000_000 + 2_208_988_800 - 2 ** 32, and 0.5 of 2 ** 32.
        (Sound::new().at(2_100_000_000.5), 14_021_504, 1 << 31),
        // -1000 + 2_208_988_800, which is 0 or more.
        (Sound::new().at(-1000.0), 2_208_987_800, 0),
    ] {
        let sound = sound.s("bd");
        let bundles = play(&mut dirt(config.clone()), &sound);
        let expected = Timetag { seconds, fraction };
        assert_eq!(bundles[0].time, expected, "{sound:?}");
    }
    let early = Sound::new().at(-2_208_988_801.0).s("bd");
    let played = dirt(config).play(&library(), &early);
    assert_eq!(played, Err(Dropped::BadTime(-1.0)));
}

#[test]
fn the_n_end_of_its_group_takes_a_sound_out_of_the_later_cuts() {
    let mut dirt = fresh();
    let first = group(&play(&mut dirt, &cutting(1.0)));
    let second = play(&mut dirt, &cutting(1.1));
    assert_eq!(cut_gates(&second), Some(vec![first]));
    let second = group(&second);
    dirt.reply(&n_end(first));
    dirt.reply(&Reply::NGo(group_state(second)));
    dirt.reply(&Reply::NOff(group_state(second)));
    let third = play(&mut dirt, &cutting(1.2));
    assert_eq!(cut_gates(&third), Some(vec![second]));
}

#[test]
fn a_sound_that_ends_before_a_later_sound_starts_gets_no_cut_gate() {
    let mut dirt = fresh();
    let first = group(&play(&mut dirt, &cutting(1.0)));
    let second = play(&mut dirt, &cutting(1.4));
    assert_eq!(cut_gates(&second), Some(vec![first]));
    let second = group(&second);
    // The sounds of `bd` at 1.0 and 1.4 end at 1.5 and 1.9.
    let third = play(&mut dirt, &cutting(1.6));
    assert_eq!(cut_gates(&third), Some(vec![second]));
    let fourth = play(&mut dirt, &cutting(2.0));
    assert_eq!(cut_gates(&fourth), Some(vec![group(&third)]));
}

#[test]
fn a_sound_without_when_has_no_end_and_a_later_one_without_when_ends_none() {
    let mut dirt = fresh();
    let at_once = group(&play(&mut dirt, &Sound::new().s("bd").cut(1)));
    let late = play(&mut dirt, &cutting(100.0));
    assert_eq!(cut_gates(&late), Some(vec![at_once]));
    let late = group(&late);
    let next = play(&mut dirt, &Sound::new().s("bd").cut(1));
    assert_eq!(cut_gates(&next), Some(vec![at_once, late]));
}

#[test]
fn the_groups_take_the_ids_of_sound_nodes_in_turn_and_start_over_at_the_end() {
    let mut dirt = dirt(config(5000..5002));
    let at_once = Sound::new().s("bd");
    assert_eq!(group(&play(&mut dirt, &at_once)), 5000);
    dirt.reply(&n_end(5000));
    assert_eq!(group(&play(&mut dirt, &at_once)), 5001);
    assert_eq!(group(&play(&mut dirt, &at_once)), 5000);
}

#[test]
fn the_next_group_skips_the_id_of_a_live_sound() {
    let mut dirt = dirt(config(5000..5003));
    let at_once = Sound::new().s("bd");
    for id in [5000, 5001, 5002] {
        assert_eq!(group(&play(&mut dirt, &at_once)), id);
    }
    dirt.reply(&n_end(5000));
    dirt.reply(&n_end(5002));
    assert_eq!(group(&play(&mut dirt, &at_once)), 5000);
    assert_eq!(group(&play(&mut dirt, &at_once)), 5002);
}

#[test]
fn with_every_id_live_the_oldest_sound_leaves_and_gives_its_id() {
    let mut dirt = dirt(config(5000..5002));
    let at_once = Sound::new().s("bd").cut(1);
    for id in [5000, 5001] {
        assert_eq!(group(&play(&mut dirt, &at_once)), id);
    }
    let third = play(&mut dirt, &at_once);
    assert_eq!(cut_gates(&third), Some(vec![5000, 5001]));
    assert_eq!(group(&third), 5000);
    let fourth = play(&mut dirt, &at_once);
    assert_eq!(cut_gates(&fourth), Some(vec![5001, 5000]));
    assert_eq!(group(&fourth), 5001);
}

#[test]
fn the_cut_gates_come_in_the_order_of_the_starts_of_their_sounds() {
    let mut dirt = dirt(config(5000..5003));
    let at_once = Sound::new().s("bd").cut(1);
    for id in [5000, 5001, 5002] {
        assert_eq!(group(&play(&mut dirt, &at_once)), id);
    }
    dirt.reply(&n_end(5000));
    assert_eq!(group(&play(&mut dirt, &at_once)), 5000);
    dirt.reply(&n_end(5001));
    let last = play(&mut dirt, &at_once);
    assert_eq!(cut_gates(&last), Some(vec![5002, 5000]));
}

#[test]
fn a_cut_all_of_0_is_off_and_one_of_another_number_is_on() {
    let mut dirt = fresh();
    let first = group(&play(&mut dirt, &cutting(1.0)));
    let other = cutting(1.1).orbit(1);
    let off = play(&mut dirt, &other.clone().with("cutAll", 0.0));
    assert_eq!(cut_gates(&off), Some(vec![]));
    let off = group(&off);
    let on = play(&mut dirt, &other.with("cutAll", -2.0));
    assert_eq!(cut_gates(&on), Some(vec![first, off]));
}

#[test]
fn start_and_stop_forget_the_parameters_of_the_global_synths_the_live_sounds_and_the_ids() {
    let mut dirt = fresh();
    let delay = dirt.orbit(0).expect("orbit 0").delay();
    let first = play(&mut dirt, &cutting(1.0));
    assert_eq!(group(&first), 2000);
    assert_eq!(cut_gates(&first), Some(vec![]));
    let changed = ["delaytime", "delayAmp", "lock", "cps"];
    assert_eq!(set_parameters(&first, delay), changed);
    let again = play(&mut dirt, &cutting(1.1));
    assert_eq!(group(&again), 2001);
    assert_eq!(cut_gates(&again), Some(vec![2000]));
    assert_eq!(set_parameters(&again, delay), Vec::<String>::new());
    let _ = dirt.start();
    assert_eq!(play(&mut dirt, &cutting(1.0)), first);
    let _ = play(&mut dirt, &cutting(1.1));
    let _ = dirt.stop();
    assert_eq!(play(&mut dirt, &cutting(1.0)), first);
}

#[test]
fn a_drop_says_why() {
    for (dropped, message) in [
        (Dropped::Silence, "the sound is silence: its name is empty"),
        (
            Dropped::NoName,
            "the sound has no name: it has neither 's' nor 'sound'",
        ),
        (
            Dropped::UnknownSound("hh".to_owned()),
            "the library has no samples of the sound 'hh'",
        ),
        (
            Dropped::NotLoaded("bd".to_owned()),
            "the sample of the sound 'bd' is not loaded",
        ),
        (
            Dropped::TooManyChannels(3),
            "the sample has 3 channels, but a sample player takes 1 or 2",
        ),
        (
            Dropped::BadUnit("x".to_owned()),
            "the unit 'x' is none of r, rate, c and s",
        ),
        (
            Dropped::TooShort(0.0001),
            "the sustain 0.0001 s is under 8 samples",
        ),
        (
            Dropped::WrongKind("speed".to_owned()),
            "the value of 'speed' is of the wrong kind: 's', 'sound', 'bank' and 'unit' take a text, and the other parameters a number",
        ),
        (
            Dropped::BadTime(-1.0),
            "the time -1 s from the OSC epoch is not finite, or under 0",
        ),
    ] {
        assert_eq!(dropped.to_string(), message);
        assert!(dropped.source().is_none(), "{dropped}");
    }
}
