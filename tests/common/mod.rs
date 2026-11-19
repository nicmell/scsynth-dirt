//! The golden files of `tests/fixtures`, as `generator/fixtures.scd` writes
//! them with sclang 3.14.1.

use std::fs;
use std::path::{Path, PathBuf};

use scsynth_synthdef::{DefFile, read_def_file};

/// The channel counts of the golden definition files.
pub const CHANNELS: [usize; 3] = [1, 2, 4];

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
