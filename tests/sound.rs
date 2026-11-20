//! `Sound` and `Value`: a new sound is empty, each of the 32 fields of
//! libdirt has a setter that stores its value as it is, under the name of the
//! field, and the parameters come in the order of their names. `from_args`
//! reads the arguments of a `/dirt/play` message, and refuses an odd count, a
//! name that is no string and a value of another type, with the place of the
//! fault.

use std::collections::BTreeSet;
use std::error::Error;

use scsynth_dirt::{Sound, SoundError, Value};
use scsynth_osc::{Color, MidiMessage, RawArg, Timetag};

/// The setter of a field of libdirt, by the letter of its type in
/// `DIRT_FIELDS` of `dirt.h`.
enum Setter {
    /// `F`, a float.
    F(fn(Sound, f64) -> Sound),
    /// `I`, an int.
    I(fn(Sound, i32) -> Sound),
    /// `S`, a string.
    S(fn(Sound, &'static str) -> Sound),
}

/// The 32 fields of `DIRT_FIELDS` in `dirt.h` of libdirt at `123b399`, in
/// their order, each with its setter.
const FIELDS: [(&str, Setter); 32] = [
    ("cps", Setter::F(Sound::cps)),
    ("s", Setter::S(Sound::s)),
    ("offset", Setter::F(Sound::offset)),
    ("begin", Setter::F(Sound::begin)),
    ("end", Setter::F(Sound::end)),
    ("speed", Setter::F(Sound::speed)),
    ("pan", Setter::F(Sound::pan)),
    ("velocity", Setter::F(Sound::velocity)),
    ("vowel", Setter::S(Sound::vowel)),
    ("cutoff", Setter::F(Sound::cutoff)),
    ("resonance", Setter::F(Sound::resonance)),
    ("accelerate", Setter::F(Sound::accelerate)),
    ("shape", Setter::F(Sound::shape)),
    ("kriole", Setter::I(Sound::kriole)),
    ("gain", Setter::F(Sound::gain)),
    ("cut", Setter::I(Sound::cut)),
    ("delay", Setter::F(Sound::delay)),
    ("delaytime", Setter::F(Sound::delaytime)),
    ("delayfeedback", Setter::F(Sound::delayfeedback)),
    ("crush", Setter::F(Sound::crush)),
    ("coarse", Setter::I(Sound::coarse)),
    ("hcutoff", Setter::F(Sound::hcutoff)),
    ("hresonance", Setter::F(Sound::hresonance)),
    ("bandf", Setter::F(Sound::bandf)),
    ("bandq", Setter::F(Sound::bandq)),
    ("unit", Setter::S(Sound::unit)),
    ("loop", Setter::F(Sound::r#loop)),
    ("n", Setter::F(Sound::n)),
    ("attack", Setter::F(Sound::attack)),
    ("hold", Setter::F(Sound::hold)),
    ("release", Setter::F(Sound::release)),
    ("orbit", Setter::I(Sound::orbit)),
];

/// A string argument.
fn text(value: &str) -> RawArg {
    RawArg::String(value.to_owned())
}

/// The arguments of a `/dirt/play` message of `pairs`, each a name and a
/// value.
fn pairs<const N: usize>(pairs: [(&str, RawArg); N]) -> Vec<RawArg> {
    pairs
        .into_iter()
        .flat_map(|(name, value)| [text(name), value])
        .collect()
}

#[test]
fn a_new_sound_is_empty_and_has_no_when() {
    let sound = Sound::new();
    assert_eq!(sound.parameters().count(), 0);
    assert_eq!(sound.when(), None);
    assert_eq!(sound, Sound::default());
}

#[test]
fn each_setter_of_a_field_of_libdirt_stores_its_value_as_it_is_under_the_name_of_the_field() {
    let names: BTreeSet<&str> = FIELDS.iter().map(|&(name, _)| name).collect();
    assert_eq!(names.len(), FIELDS.len(), "a name comes twice in FIELDS");
    for (name, setter) in FIELDS {
        let (sound, value) = match setter {
            Setter::F(setter) => (setter(Sound::new(), -7.5), Value::Number(-7.5)),
            Setter::I(setter) => (setter(Sound::new(), -3), Value::Number(-3.0)),
            Setter::S(setter) => (setter(Sound::new(), "x"), Value::Text("x".to_owned())),
        };
        assert_eq!(sound.parameters().collect::<Vec<_>>(), [(name, &value)]);
        assert_eq!(sound.when(), None, "{name}");
    }
}

#[test]
fn set_and_with_replace_the_value_of_a_name() {
    let mut sound = Sound::new().with("n", 1).with("n", "x");
    assert_eq!(sound.get("n"), Some(&Value::from("x")));
    sound.set("n", 2.5).set("s", "bd").set("s", "sn");
    assert_eq!(
        sound.parameters().collect::<Vec<_>>(),
        [("n", &Value::Number(2.5)), ("s", &Value::from("sn"))]
    );
}

#[test]
fn get_reads_a_parameter_and_remove_takes_it_out() {
    let mut sound = Sound::new().with("s", "bd").with("n", 3);
    assert_eq!(sound.get("n"), Some(&Value::Number(3.0)));
    assert_eq!(sound.get("speed"), None);
    assert_eq!(sound.remove("n"), Some(Value::Number(3.0)));
    assert_eq!(sound.remove("n"), None);
    assert_eq!(sound.get("n"), None);
    assert_eq!(
        sound.parameters().collect::<Vec<_>>(),
        [("s", &Value::from("bd"))]
    );
}

#[test]
fn the_parameters_come_in_the_order_of_the_bytes_of_their_names() {
    let sound = Sound::new()
        .with("speed", 2)
        .with("delaytime", 0.5)
        .with("s", "bd")
        .with("delayfeedback", 0.2)
        .with("delaySend", 1)
        .with("delay", 0.3);
    let names: Vec<&str> = sound.parameters().map(|(name, _)| name).collect();
    assert_eq!(
        names,
        [
            "delay",
            "delaySend",
            "delayfeedback",
            "delaytime",
            "s",
            "speed"
        ]
    );
}

#[test]
fn at_and_set_when_set_the_time_that_when_reads() {
    let mut sound = Sound::new().with("s", "bd").at(1.5);
    assert_eq!(sound.when(), Some(1.5));
    sound.set_when(Some(2.25));
    assert_eq!(sound.when(), Some(2.25));
    sound.set_when(None);
    assert_eq!(sound.when(), None);
    assert_eq!(sound, Sound::new().with("s", "bd"));
}

#[test]
fn a_value_comes_from_an_f64_an_f32_an_i32_a_str_and_a_string() {
    assert_eq!(Value::from(0.25), Value::Number(0.25));
    assert_eq!(Value::from(0.1_f32), Value::Number(f64::from(0.1_f32)));
    assert_eq!(Value::from(-3), Value::Number(-3.0));
    assert_eq!(Value::from("bd"), Value::Text("bd".to_owned()));
    assert_eq!(Value::from("bd".to_owned()), Value::Text("bd".to_owned()));
}

#[test]
fn as_number_reads_a_number_and_as_text_a_text() {
    assert_eq!(Value::Number(0.25).as_number(), Some(0.25));
    assert_eq!(Value::Number(0.25).as_text(), None);
    assert_eq!(Value::from("bd").as_text(), Some("bd"));
    assert_eq!(Value::from("bd").as_number(), None);
}

#[test]
fn from_args_reads_each_type_that_it_takes_and_leaves_when_unset() {
    let args = pairs([
        ("int", RawArg::Int(-3)),
        ("float", RawArg::Float(0.1)),
        ("double", RawArg::Double(0.25)),
        ("long", RawArg::Long(5_000_000_000)),
        ("string", text("bd")),
        ("true", RawArg::True),
        ("false", RawArg::False),
    ]);
    let sound = Sound::from_args(&args).unwrap_or_else(|error| panic!("{error}"));
    let expected = Sound::new()
        .with("int", -3)
        .with("float", 0.1_f32)
        .with("double", 0.25)
        .with("long", 5_000_000_000.0)
        .with("string", "bd")
        .with("true", 1)
        .with("false", 0);
    assert_eq!(sound, expected);
    assert_eq!(sound.when(), None);
    assert_eq!(Sound::from_args(&[]), Ok(Sound::new()));
}

#[test]
fn from_args_lets_a_later_pair_replace_an_earlier_one_of_the_same_name() {
    let args = pairs([("n", RawArg::Int(1)), ("s", text("bd")), ("n", text("3"))]);
    assert_eq!(
        Sound::from_args(&args),
        Ok(Sound::new().with("s", "bd").with("n", "3"))
    );
}

#[test]
fn from_args_refuses_an_odd_count_with_the_count() {
    assert_eq!(Sound::from_args(&[text("s")]), Err(SoundError::OddCount(1)));
    let args = [text("s"), text("bd"), text("n")];
    assert_eq!(Sound::from_args(&args), Err(SoundError::OddCount(3)));
}

#[test]
fn from_args_refuses_a_name_that_is_no_string_with_its_index() {
    for name in [
        RawArg::Int(1),
        RawArg::Double(0.5),
        RawArg::True,
        RawArg::Nil,
        RawArg::Char('n'),
        RawArg::Array(vec![text("n")]),
    ] {
        let args = [text("s"), text("bd"), name.clone(), RawArg::Int(1)];
        assert_eq!(
            Sound::from_args(&args),
            Err(SoundError::NotAName(2)),
            "{name:?}"
        );
    }
}

#[test]
fn from_args_refuses_a_value_of_another_type_with_the_name_of_its_parameter() {
    for value in [
        RawArg::Nil,
        RawArg::Blob(vec![1, 2, 3]),
        RawArg::Array(vec![RawArg::Int(1)]),
        RawArg::Time(Timetag::IMMEDIATELY),
        RawArg::Inf,
        RawArg::Char('a'),
        RawArg::Color(Color {
            red: 1,
            green: 2,
            blue: 3,
            alpha: 4,
        }),
        RawArg::Midi(MidiMessage {
            port: 0,
            status: 0x90,
            data1: 60,
            data2: 100,
        }),
    ] {
        let args = pairs([("s", text("bd")), ("n", value.clone())]);
        assert_eq!(
            Sound::from_args(&args),
            Err(SoundError::WrongType("n".to_owned())),
            "{value:?}"
        );
    }
}

#[test]
fn an_error_names_its_place() {
    for (error, message) in [
        (
            SoundError::OddCount(3),
            "the count 3 of the arguments is odd, but they must be pairs of a name and a value",
        ),
        (
            SoundError::NotAName(2),
            "argument 2 is no string, but it must be the name of a parameter",
        ),
        (
            SoundError::WrongType("n".to_owned()),
            "the value of 'n' must be an int, a float, a double, a long, a string, true or false",
        ),
    ] {
        assert_eq!(error.to_string(), message);
        assert!(error.source().is_none(), "{error}");
    }
}
