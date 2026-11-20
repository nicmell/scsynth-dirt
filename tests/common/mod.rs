//! The golden files of `tests/fixtures`, as the generators of `generator/`
//! write them with sclang 3.14.1, the packets of the crate on the wire, and
//! their render in scsynth.
#![allow(dead_code, reason = "each test file uses a part of this module")]

pub mod events;
pub mod nrt;
pub mod wire;

use std::fs;
use std::path::{Path, PathBuf};

use scsynth_synthdef::{DefFile, InputSpec, Rate, SynthDef, UGen, operator, read_def_file};

/// The channel counts of the golden definition files.
pub const CHANNELS: [u32; 3] = [1, 2, 4];

/// The largest datagram that sc-app3 sends to scsynth: `MAX_DATAGRAM` of its
/// core.
pub const MAX_DATAGRAM: usize = 8_192;

/// The path of a file of `tests/fixtures`.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// The bytes of the golden definition file `definitions/<name>.scsyndef`.
pub fn golden_bytes(name: &str) -> Vec<u8> {
    fs::read(fixture(&format!("definitions/{name}.scsyndef"))).expect("the golden file reads")
}

/// The golden definition file `definitions/<name>.scsyndef`, read.
pub fn golden(name: &str) -> DefFile {
    read_def_file(&golden_bytes(name)).unwrap_or_else(|error| panic!("{name}: {error}"))
}

/// Asserts that `built` has the bytes of the definition of its name in the
/// golden file `definitions/<file>.scsyndef`, as `SynthDef::as_bytes` writes
/// both. On a difference, it panics with the dumps of both definitions, line
/// by line.
pub fn assert_golden_bytes(file: &str, built: &SynthDef) {
    let golden = golden(file)
        .synth_defs
        .into_iter()
        .find(|def| def.name == built.name)
        .unwrap_or_else(|| panic!("{file} holds no definition {}", built.name));
    let golden_bytes = golden
        .as_bytes()
        .expect("a golden definition meets the limits");
    let built_bytes = built
        .as_bytes()
        .unwrap_or_else(|error| panic!("{}: {error}", built.name));
    assert!(
        built_bytes == golden_bytes,
        "{} differs from its golden definition in {file}: {} bytes against {}, first at byte {}\n{}",
        built.name,
        built_bytes.len(),
        golden_bytes.len(),
        first_difference(&golden_bytes, &built_bytes),
        difference(&golden, built),
    );
}

/// The index of the first byte where `a` and `b` differ, or the length of
/// the shorter.
fn first_difference(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .zip(b)
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| a.len().min(b.len()))
}

/// The dumps of two definitions, line by line. A line of both shows once.
/// Where they differ, `-` marks the line of `golden`, and `+` the line of
/// `built`.
fn difference(golden: &SynthDef, built: &SynthDef) -> String {
    let (golden, built) = (dump(golden), dump(built));
    let mut report = vec!["- golden".to_owned(), "+ built".to_owned()];
    for index in 0..golden.len().max(built.len()) {
        match (golden.get(index), built.get(index)) {
            (Some(golden), Some(built)) if golden == built => report.push(format!("  {golden}")),
            (golden, built) => {
                report.extend(golden.map(|line| format!("- {line}")));
                report.extend(built.map(|line| format!("+ {line}")));
            }
        }
    }
    report.join("\n")
}

/// The dump of a definition, one line per item: its constants, its
/// parameters, its names, its UGens in order and its variants. A UGen shows
/// its rate, its special index, its inputs and the rates of its outputs. An
/// input is a constant, by its value, or `u<ugen>.<output>`.
fn dump(def: &SynthDef) -> Vec<String> {
    let names: Vec<String> = def
        .control_names
        .iter()
        .map(|name| format!("{}={}", name.name, name.index))
        .collect();
    let mut lines = vec![
        format!("constants [{}]", numbers(&def.constants)),
        format!("controls [{}]", numbers(&def.controls)),
        format!("names [{}]", names.join(" ")),
    ];
    for (index, ugen) in def.children.iter().enumerate() {
        lines.push(format!("u{index} {}", ugen_line(def, ugen)));
    }
    for variant in &def.variants {
        lines.push(format!("{} [{}]", variant.name, numbers(&variant.values)));
    }
    lines
}

/// A UGen of `def`, as a line of its [`dump`].
fn ugen_line(def: &SynthDef, ugen: &UGen) -> String {
    let symbol = match ugen.name.as_str() {
        "BinaryOpUGen" => operator::binary_name(ugen.special_index),
        "UnaryOpUGen" => operator::unary_name(ugen.special_index),
        _ => None,
    };
    let special = match symbol {
        Some(symbol) => format!("{} ({symbol})", ugen.special_index),
        None => ugen.special_index.to_string(),
    };
    let inputs: Vec<String> = ugen
        .inputs
        .iter()
        .map(|input| match *input {
            InputSpec::Constant(index) => def.constants[index as usize].to_string(),
            InputSpec::UGen { index, output } => format!("u{index}.{output}"),
        })
        .collect();
    let outputs: Vec<&str> = ugen.outputs.iter().map(|&rate| rate_name(rate)).collect();
    format!(
        "{}.{} special {special} [{}] -> [{}]",
        ugen.name,
        rate_name(ugen.rate),
        inputs.join(" "),
        outputs.join(" "),
    )
}

/// Numbers, apart by spaces.
fn numbers(values: &[f32]) -> String {
    let numbers: Vec<String> = values.iter().map(ToString::to_string).collect();
    numbers.join(" ")
}

/// The selector of a rate in sclang, such as `ar`.
fn rate_name(rate: Rate) -> &'static str {
    match rate {
        Rate::Scalar => "ir",
        Rate::Control => "kr",
        Rate::Audio => "ar",
        Rate::Demand => "dr",
    }
}
