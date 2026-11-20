//! scsynth 3.14.1 in NRT mode, for the acceptance tests: a score of the
//! packets of the crate, the WAV files of the samples, and a render. NRT mode
//! opens no port and no audio device, and the output is complete when
//! scsynth exits.
//!
//! `SCSYNTH` names the scsynth to run, and `SCSYNTH_PLUGINS` its folder of
//! plugins. Both default to the bundle of `/Applications/SuperCollider.app`.
//! `SCSYNTH_SC3_PLUGINS` names the folder of the plugins of sc3-plugins, by
//! default the folder `SC3plugins` of the Extensions.

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

use scsynth_osc::{Bundle, Command, Packet, Timetag};

/// The sample rate of a render and of the samples of the tests, in hertz.
pub const SAMPLE_RATE: u32 = 48_000;

/// The folder of the bundle of SuperCollider that holds scsynth and its
/// plugins.
const BUNDLE: &str = "/Applications/SuperCollider.app/Contents/Resources";

/// The marks of a line that tells of a failure, among the lines that scsynth
/// 3.14.1 prints in NRT mode.
const FAILURES: [&str; 4] = ["FAILURE", "ERROR", "exception", "could not be opened"];

/// The count of the folders that the tests made, which keeps their names
/// apart.
static FOLDERS: AtomicUsize = AtomicUsize::new(0);

/// The path in the variable `variable`, or `name` in the bundle.
fn bundle_path(variable: &str, name: &str) -> PathBuf {
    env::var_os(variable).map_or_else(|| PathBuf::from(BUNDLE).join(name), PathBuf::from)
}

/// The plugins of the bundle, for `-U`. With `-U`, scsynth loads no other
/// plugin, such as those of the Extensions folder.
pub fn plugins() -> OsString {
    bundle_path("SCSYNTH_PLUGINS", "plugins").into_os_string()
}

/// The plugins of the bundle, then those of sc3-plugins, for `-U`.
pub fn sc3_plugins() -> OsString {
    let folder = env::var_os("SCSYNTH_SC3_PLUGINS").unwrap_or_else(|| {
        let home = PathBuf::from(env::var_os("HOME").expect("a home folder"));
        home.join("Library/Application Support/SuperCollider/Extensions/SC3plugins")
            .into_os_string()
    });
    let mut plugins = plugins();
    plugins.push(":");
    plugins.push(folder);
    plugins
}

/// The time tag of `seconds` from the OSC epoch: the whole seconds, and the
/// fraction in units of 2 ** -32, rounded down.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the time of a score is a few seconds from 0, and the cast of the fraction rounds it down"
)]
fn timetag(seconds: f64) -> Timetag {
    let whole = seconds.floor();
    Timetag {
        seconds: whole as u32,
        fraction: ((seconds - whole) * 4_294_967_296.0) as u32,
    }
}

/// A score of NRT mode: bundles in time order, each written as its size, a
/// big-endian i32, then its bytes. scsynth ends a score at a time tag that
/// comes before the one of the entry before it, so the score refuses one.
#[derive(Debug, Default)]
pub struct Score {
    /// The entries, as scsynth reads them.
    bytes: Vec<u8>,
    /// The time tag of the last entry, its seconds in the high 32 bits.
    last: u64,
}

impl Score {
    /// Adds `bundle` as the next entry.
    ///
    /// # Panics
    ///
    /// If the time tag of `bundle` comes before the one of the last entry.
    pub fn bundle(&mut self, bundle: Bundle<Command>) {
        let time = (u64::from(bundle.time.seconds) << 32) | u64::from(bundle.time.fraction);
        assert!(
            time >= self.last,
            "the score is in time order, but {:?} comes before the last entry",
            bundle.time
        );
        self.last = time;
        let bytes = Packet::Bundle(bundle).encode();
        let size = i32::try_from(bytes.len()).expect("a bundle fits the size of an entry");
        self.bytes.extend(size.to_be_bytes());
        self.bytes.extend(bytes);
    }

    /// Adds the packets of `bundle` as the next entry, in a bundle at
    /// `seconds` in place of its own time tag.
    ///
    /// # Panics
    ///
    /// If `seconds` comes before the time of the last entry.
    pub fn bundle_at(&mut self, seconds: f64, bundle: Bundle<Command>) {
        self.bundle(Bundle {
            time: timetag(seconds),
            packets: bundle.packets,
        });
    }

    /// Adds `command` as the next entry, in a bundle of its own at `seconds`.
    ///
    /// # Panics
    ///
    /// If `seconds` comes before the time of the last entry.
    pub fn message(&mut self, seconds: f64, command: Command) {
        self.bundle(Bundle {
            time: timetag(seconds),
            packets: vec![Packet::Message(command)],
        });
    }
}

/// The samples of a sound, frame after frame. A frame holds one sample of
/// each channel.
#[derive(Clone, Debug, PartialEq)]
pub struct Audio {
    /// The number of channels.
    pub channels: usize,
    /// The samples, frame after frame.
    pub samples: Vec<f32>,
}

impl Audio {
    /// `frames` frames, each a copy of `frame`, with one channel per value of
    /// `frame`.
    pub fn constant(frame: &[f32], frames: usize) -> Self {
        Self {
            channels: frame.len(),
            samples: frame.repeat(frames),
        }
    }

    /// The number of frames.
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels
    }

    /// The sample of `channel` in the frame `frame`.
    ///
    /// # Panics
    ///
    /// For a frame past the end, or a channel past the count.
    pub fn at(&self, frame: usize, channel: usize) -> f32 {
        assert!(
            channel < self.channels,
            "channel {channel} of {}",
            self.channels
        );
        self.samples[frame * self.channels + channel]
    }
}

/// The sample format of a WAV file of the tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Integers of 16 bits.
    Int16,
    /// Floats of 32 bits.
    Float,
}

/// The bytes of a WAV file of `audio` at [`SAMPLE_RATE`], in `format`. A
/// sample of [`Format::Int16`] is the value times 32,768, rounded and held
/// to the range of an i16.
///
/// # Panics
///
/// For audio too large for the fields of the header of a WAV file.
fn wav(audio: &Audio, format: Format) -> Vec<u8> {
    let (tag, bits): (u16, u16) = match format {
        Format::Int16 => (1, 16),
        Format::Float => (3, 32),
    };
    let mut data = Vec::new();
    for &sample in &audio.samples {
        match format {
            Format::Int16 => data.extend(int16(sample).to_le_bytes()),
            Format::Float => data.extend(sample.to_le_bytes()),
        }
    }
    let too_large = "audio that fits the header of a WAV file";
    let channels = u16::try_from(audio.channels).expect(too_large);
    let block_align = channels.checked_mul(bits / 8).expect(too_large);
    let data_size = u32::try_from(data.len()).expect(too_large);
    let mut bytes = Vec::with_capacity(data.len() + 44);
    bytes.extend(b"RIFF");
    bytes.extend(data_size.checked_add(36).expect(too_large).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(tag.to_le_bytes());
    bytes.extend(channels.to_le_bytes());
    bytes.extend(SAMPLE_RATE.to_le_bytes());
    bytes.extend((SAMPLE_RATE * u32::from(block_align)).to_le_bytes());
    bytes.extend(block_align.to_le_bytes());
    bytes.extend(bits.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(data_size.to_le_bytes());
    bytes.extend(data);
    bytes
}

/// `sample` as an integer of 16 bits: times 32,768, rounded, and held to the
/// range of an i16.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the value is rounded and held to the range of an i16"
)]
fn int16(sample: f32) -> i16 {
    (sample * 32_768.0).round().clamp(-32_768.0, 32_767.0) as i16
}

/// The audio of the bytes of a WAV file of 32-bit floats, as scsynth 3.14.1
/// writes it with the format `WAV float`: the chunks `fmt ` and `data`, and
/// others that it skips, such as `fact` and `PEAK`.
///
/// # Panics
///
/// For bytes that are no such WAV file.
fn read_wav(bytes: &[u8]) -> Audio {
    assert!(
        bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
        "a RIFF file of WAVE"
    );
    let u16_at = |body: &[u8], at: usize| u16::from_le_bytes([body[at], body[at + 1]]);
    let mut channels = None;
    let mut samples = None;
    let mut rest = &bytes[12..];
    while rest.len() >= 8 {
        let size = u32::from_le_bytes([rest[4], rest[5], rest[6], rest[7]]);
        let size = usize::try_from(size).expect("a chunk size");
        let body = &rest[8..8 + size];
        match &rest[..4] {
            b"fmt " => {
                let format = (u16_at(body, 0), u16_at(body, 14));
                assert_eq!(format, (3, 32), "floats of 32 bits");
                channels = Some(usize::from(u16_at(body, 2)));
            }
            b"data" => {
                let floats = body.chunks_exact(4);
                samples = Some(
                    floats
                        .map(|float| f32::from_le_bytes([float[0], float[1], float[2], float[3]]))
                        .collect::<Vec<_>>(),
                );
            }
            _ => {}
        }
        rest = &rest[(8 + size + size % 2).min(rest.len())..];
    }
    Audio {
        channels: channels.expect("a fmt chunk"),
        samples: samples.expect("a data chunk"),
    }
}

/// What a render gives back: the output of scsynth, and the text that it
/// printed.
#[derive(Debug)]
pub struct Render {
    /// The output, at [`SAMPLE_RATE`].
    pub output: Audio,
    /// What scsynth printed on its standard output, then on its standard
    /// error.
    pub text: String,
}

impl Render {
    /// The lines of the text that tell of a failure: each line that holds a
    /// mark of [`FAILURES`].
    pub fn failures(&self) -> Vec<&str> {
        self.text
            .lines()
            .filter(|line| FAILURES.iter().any(|mark| line.contains(mark)))
            .collect()
    }
}

/// A folder of a test, under the temporary folder of the system: the
/// samples, the score and the output of a render. It goes when it drops.
#[derive(Debug)]
pub struct Folder(PathBuf);

impl Folder {
    /// A new folder for the test `name`.
    ///
    /// # Panics
    ///
    /// If the folder cannot be made.
    pub fn new(name: &str) -> Self {
        let path = env::temp_dir().join(format!(
            "scsynth-dirt-{name}-{}-{}",
            process::id(),
            FOLDERS.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("the folder of the test is made");
        Self(path)
    }

    /// Writes `audio` to the WAV file `name` of the folder in `format`, and
    /// returns its path.
    ///
    /// # Panics
    ///
    /// If the file cannot be written.
    pub fn write_wav(&self, name: &str, audio: &Audio, format: Format) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, wav(audio, format)).expect("the WAV file is written");
        path
    }

    /// Renders `score` in NRT mode: its last entry ends the output. scsynth
    /// runs at [`SAMPLE_RATE`], with no input channel, `outputs` output
    /// channels, no definition of its own folder, and the plugins `plugins`.
    ///
    /// # Panics
    ///
    /// If scsynth does not run, or does not exit with success, or if its
    /// output does not read.
    pub fn render(&self, score: &Score, outputs: usize, plugins: &OsStr) -> Render {
        let score_path = self.0.join("score.osc");
        fs::write(&score_path, &score.bytes).expect("the score is written");
        let output_path = self.0.join("output.wav");
        let run = process::Command::new(bundle_path("SCSYNTH", "scsynth"))
            .arg("-N")
            .arg(&score_path)
            .arg("_")
            .arg(&output_path)
            .arg(SAMPLE_RATE.to_string())
            .args(["WAV", "float", "-i", "0", "-o"])
            .arg(outputs.to_string())
            .args(["-D", "0", "-U"])
            .arg(plugins)
            .output()
            .expect("scsynth runs");
        let text = String::from_utf8_lossy(&run.stdout).into_owned()
            + &String::from_utf8_lossy(&run.stderr);
        assert!(
            run.status.success(),
            "scsynth fails: {}\n{text}",
            run.status
        );
        let output = read_wav(&fs::read(&output_path).expect("the output reads"));
        assert_eq!(output.channels, outputs, "{text}");
        Render { output, text }
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        // A panic here, in the unwinding of a failed test, would abort the run.
        let _ = fs::remove_dir_all(&self.0);
    }
}
