//! The acceptance test: scsynth 3.14.1 renders the packets of an instance in
//! NRT mode, as `common/nrt.rs` runs it, and the test reads the sound that
//! comes out. The tests are ignored by default:
//!
//! ```sh
//! cargo test --all-features -- --ignored
//! ```
//!
//! Each test acts as a host. It makes the root group of the instance at time
//! 0, then sends the commands of `start` and loads the samples that it
//! writes after the first block. It plays its sounds with the epoch 0, and
//! ends with the bundles of `stop`. NRT mode drops every reply, so the host
//! gives the library a `/b_info` of its own. Each orbit has `limitertype` 0,
//! and a silent sound opens the monitor of orbit 0 before the sounds that
//! the test measures (see Traps in the README). scsynth exits with success,
//! so `stop` freed each synth of the instance, and prints no failure. A
//! sound of a constant sample comes out at the level of the chain, and its
//! first sample sits [`ONSET`] frames after the frame of its time. A cut
//! silences the sound that it cuts. The delay echoes, with `CombL`, and with
//! `SwitchDelay` and the feature `sc3-plugins`. On 1 channel, a sound is the
//! sum of the channels of its sample, and the reverb leaves the next orbit
//! silent. On 4 channels, a sound comes out on the channel of its pan.

mod common;

use std::f32::consts::FRAC_1_SQRT_2;
use std::f64::consts::TAU;
use std::ffi::OsStr;
use std::iter;
use std::ops::Range;

use scsynth_dirt::{Config, Delay, Dirt, Library, Register, Sound};
use scsynth_osc::command::buffer::BAllocRead;
use scsynth_osc::command::group::{GNew, NewGroup};
use scsynth_osc::command::node::NSet;
use scsynth_osc::command::synth::SNew;
use scsynth_osc::command::synth_def::DRecv;
use scsynth_osc::reply::buffer::{BInfo, BufferInfo};
use scsynth_osc::{AddAction, BufferId, Command, ControlSetting, NewNodeId, NodeId, Reply};
use scsynth_synthdef::{NewUGen, Rate, SynthDefBuilder};

use common::nrt::{self, Audio, Folder, Format, SAMPLE_RATE, Score};

/// The frames from the frame of the time `t` of a sound, `round(t * 48000)`,
/// to its first sample that is not 0, for a time in the middle of a block:
/// measured with scsynth 3.14.1. The gate starts on the frame of the time,
/// and the first sample of its envelope is 0.
const ONSET: usize = 1;

/// The most that a level of the chain differs from its prediction: the
/// rounding of its products of f32.
const LEVEL_ERROR: f32 = 1e-6;

/// The time of `start` and of the loads of the samples, in seconds: after
/// the first block. In the first block of NRT mode, the constructor of
/// `LocalIn` in scsynth 3.14.1 takes the memory of its bus as written by
/// `LocalOut` (`IOUGens.cpp`). So a reverb made there starts with garbage
/// in its loop: measured up to 1e14 on 4 channels, and different at each
/// run.
const START: f64 = 0.01;

/// The time of the silent sound that opens the monitor of orbit 0, in
/// seconds.
const OPEN: f64 = 0.1;

/// A sample of a host: its name, its audio, and the format of its file.
type SampleFile = (&'static str, Audio, Format);

/// A host of one instance: the folder of its files, the instance, its
/// library, and the score of all that it sends to scsynth.
struct Host {
    /// The folder of the samples, the score and the output.
    folder: Folder,
    /// The instance.
    dirt: Dirt,
    /// The library of the samples.
    library: Library,
    /// What the host sends, in time order.
    score: Score,
}

impl Host {
    /// A host of an instance of `config` with `limitertype` 0 on each orbit,
    /// in a new folder `name`. At time 0, the score makes the root group of
    /// `config` at the head of the group 0 of scsynth. At [`START`], it sends
    /// the commands of `start`, and loads a silent sample `open`, then each
    /// of `samples`. At [`OPEN`], `open` plays on orbit 0.
    fn new(name: &str, config: Config, samples: &[SampleFile]) -> Self {
        let root = config.root;
        let mut dirt = Dirt::new(config).unwrap_or_else(|error| panic!("{name}: {error}"));
        for index in 0..dirt.orbits().len() {
            if let Some(orbit) = dirt.orbit_mut(index) {
                orbit.set("limitertype", 0.0);
            }
        }
        let mut score = Score::default();
        score.message(
            0.0,
            Command::GNew(GNew {
                groups: vec![NewGroup {
                    id: NewNodeId::new(root.0).expect("the root is a group"),
                    action: AddAction::Head,
                    target: NodeId(0),
                }],
            }),
        );
        for command in dirt.start() {
            score.message(START, command);
        }
        let mut host = Self {
            folder: Folder::new(name),
            dirt,
            library: Library::new(0..16).expect("a range of buffers"),
            score,
        };
        let open = ("open", Audio::constant(&[0.0], 2_400), Format::Float);
        for (name, audio, format) in iter::once(&open).chain(samples) {
            host.load(name, audio, *format);
        }
        host.play(&Sound::new().at(OPEN).s("open"));
        host
    }

    /// Writes `audio` to a WAV file of `format`, adds the file to the library
    /// as the sample `name`, and loads it at [`START`]. The library takes a
    /// `/b_info` of the numbers of the file.
    fn load(&mut self, name: &str, audio: &Audio, format: Format) {
        let path = self.folder.write_wav(&format!("{name}.wav"), audio, format);
        let path = path.to_str().expect("a path in UTF-8");
        for command in self.library.add(name, [path]).expect("free buffers") {
            self.score.message(START, command);
        }
        let buffer = self.library.samples(name).expect("the sample")[0].buffer();
        self.library.reply(&Reply::BInfo(BInfo {
            buffers: vec![BufferInfo {
                buffer,
                frames: i32::try_from(audio.frames()).expect("frames of an i32"),
                channels: i32::try_from(audio.channels).expect("channels of an i32"),
                sample_rate: sample_rate(),
            }],
        }));
    }

    /// Plays `sound`: its bundles join the score.
    fn play(&mut self, sound: &Sound) {
        let bundles = self
            .dirt
            .play(&self.library, sound)
            .unwrap_or_else(|dropped| panic!("{sound:?} drops: {dropped}"));
        for bundle in bundles {
            self.score.bundle(bundle);
        }
    }

    /// The output of the render of the score, which ends with the bundles of
    /// `stop` at `end` seconds, on `outputs` output channels, with the
    /// plugins `plugins`. scsynth must print no failure.
    fn render(mut self, end: f64, outputs: usize, plugins: &OsStr) -> Audio {
        // At its exit, scsynth 3.14.1 unloads its plugins before it frees the
        // nodes that remain (World_Cleanup), and crashes on a unit whose
        // destructor is in a plugin, as in each global synth: measured, a
        // SIGSEGV. stop frees every node of the instance, which prevents it,
        // so a synth that stop leaves fails the render.
        for bundle in self.dirt.stop() {
            self.score.bundle_at(end, bundle);
        }
        let render = self.folder.render(&self.score, outputs, plugins);
        let failures = render.failures();
        assert!(failures.is_empty(), "{failures:?}\n{}", render.text);
        render.output
    }
}

/// An instance of `channels` channels and `orbits` orbits at the sample rate
/// of a render, with the root group 1 and the epoch 0 of a score. The
/// private buses start at 16, and the output buses at 0.
fn config(channels: u32, orbits: u32) -> Config {
    Config {
        channels,
        orbits,
        root: NodeId(1),
        fixed_nodes: 100,
        sound_nodes: 1000..2000,
        first_bus: 16,
        out_bus: 0,
        sample_rate: f64::from(SAMPLE_RATE),
        delay: Delay::Comb,
        register: Register::default(),
        epoch: 0.0,
    }
}

/// The sample rate of a render, as a `/b_info` gives it.
#[allow(
    clippy::cast_precision_loss,
    reason = "an f32 holds every integer up to 2 ** 24 exactly"
)]
fn sample_rate() -> f32 {
    SAMPLE_RATE as f32
}

/// The frame of `seconds`: `round(seconds * 48000)`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the times of the tests are a few seconds from 0"
)]
fn frame(seconds: f64) -> usize {
    (seconds * f64::from(SAMPLE_RATE)).round() as usize
}

/// The time of the frame `frame`, in seconds.
#[allow(
    clippy::cast_precision_loss,
    reason = "the frames of the tests stay far below 2 ** 52"
)]
fn time(frame: usize) -> f64 {
    frame as f64 / f64::from(SAMPLE_RATE)
}

/// `frames` frames of a sine of 1 kHz at the amplitude 0.5, on one channel.
#[allow(clippy::cast_possible_truncation, reason = "a sample is an f32")]
fn tone(frames: usize) -> Audio {
    Audio {
        channels: 1,
        samples: (0..frames)
            .map(|frame| ((TAU * 1_000.0 * time(frame)).sin() * 0.5) as f32)
            .collect(),
    }
}

/// The first frame from `from` on that holds a sample other than 0.
fn first_sound(output: &Audio, from: usize) -> Option<usize> {
    (from..output.frames())
        .find(|&frame| (0..output.channels).any(|channel| output.at(frame, channel) != 0.0))
}

/// The largest magnitude of the samples of `channel` in `frames`.
fn peak(output: &Audio, channel: usize, frames: Range<usize>) -> f32 {
    frames
        .map(|frame| output.at(frame, channel).abs())
        .fold(0.0, f32::max)
}

/// Asserts that `channel` holds `level` in each frame of `frames`, within
/// [`LEVEL_ERROR`].
fn assert_level(output: &Audio, channel: usize, frames: Range<usize>, level: f32) {
    for frame in frames {
        let sample = output.at(frame, channel);
        assert!(
            (sample - level).abs() <= LEVEL_ERROR,
            "frame {frame}, channel {channel}: {sample} against {level}"
        );
    }
}

/// The first frame of a sound of a sample of 0.5 at `time`, on 2 channels,
/// from the time of the silent sound before it.
fn onset(name: &str, time: f64) -> Option<usize> {
    let mono = ("mono", Audio::constant(&[0.5], 4_800), Format::Int16);
    let mut host = Host::new(name, config(2, 1), &[mono]);
    host.play(&Sound::new().at(time).s("mono"));
    let output = host.render(time + 0.2, 2, &nrt::plugins());
    first_sound(&output, frame(OPEN))
}

/// Asserts that the delay line `delay` echoes, with the plugins `plugins`. A
/// tone of 50 ms plays at 0.4 seconds, then again at 0.8 seconds with
/// `delay` 0.5, `delaytime` 0.1 and `delayfeedback` 0.5. The output is 0
/// after the first tone, and holds echoes after the second.
fn assert_echoes(name: &str, delay: Delay, plugins: &OsStr) {
    let mut config = config(2, 1);
    config.delay = delay;
    let mut host = Host::new(name, config, &[("tone", tone(2_400), Format::Float)]);
    host.play(&Sound::new().at(0.4).s("tone"));
    let delayed = Sound::new()
        .at(0.8)
        .s("tone")
        .delay(0.5)
        .delaytime(0.1)
        .delayfeedback(0.5);
    host.play(&delayed);
    let output = host.render(1.2, 2, plugins);
    for channel in 0..2 {
        let dry = peak(&output, channel, frame(0.45)..frame(0.8));
        assert!(dry == 0.0, "channel {channel}: {dry} after the dry tone");
        let echoes = peak(&output, channel, frame(0.85)..frame(1.2));
        assert!(
            echoes > 0.01,
            "channel {channel}: {echoes} after the echoed tone"
        );
    }
}

#[test]
#[ignore = "needs scsynth 3.14.1: cargo test -- --ignored"]
fn a_sound_of_a_constant_sample_comes_out_at_the_level_that_the_chain_predicts() {
    let mono = ("mono", Audio::constant(&[0.5], 4_800), Format::Int16);
    let stereo = (
        "stereo",
        Audio::constant(&[0.5, -0.25], 4_800),
        Format::Float,
    );
    let mut host = Host::new("level", config(2, 1), &[mono, stereo]);
    host.play(&Sound::new().at(0.4).s("mono"));
    host.play(&Sound::new().at(0.7).s("stereo"));
    let output = host.render(1.0, 2, &nrt::plugins());
    // A channel is the product of the chain:
    // - the player reads the sample as it is: 0.5, or 0.5 and -0.25;
    // - Pan2 and Balance2 take the pan 0.5 of the orbit, which play sends as
    //   0, and scale each side by the sine table of scsynth at pi / 4, the
    //   f32 of 1 / sqrt(2);
    // - the gate scales by the amp 0.4 of the orbit, times
    //   (gain.min(2) + overgain) ** 4, which the gain 1 and the overgain 0
    //   of the orbit make 1;
    // - the monitor adds the effect bus, on which the delay and the reverb
    //   write 0, takes the sum with limitertype 0, and its envelope holds 1
    //   after its attack.
    let side = FRAC_1_SQRT_2 * 0.4;
    for (time, levels) in [(0.4, [0.5, 0.5]), (0.7, [0.5, -0.25])] {
        let frames = frame(time) + ONSET..frame(time) + 4_700;
        for (channel, level) in levels.into_iter().enumerate() {
            assert_level(&output, channel, frames.clone(), level * side);
        }
    }
}

#[test]
#[ignore = "needs scsynth 3.14.1: cargo test -- --ignored"]
fn the_first_sample_of_a_sound_sits_onset_frames_after_the_frame_of_its_time() {
    // Frame 21 of block 300: in the middle of a block, the time tag of a
    // bundle takes the frame of its time (World_NonRealTimeSynthesis).
    let first = 300 * 64 + 21;
    assert_eq!(onset("onset", time(first)), Some(first + ONSET));
    assert_eq!(
        onset("onset_later", time(first + 1)),
        Some(first + 1 + ONSET)
    );
}

#[test]
#[ignore = "needs scsynth 3.14.1: cargo test -- --ignored"]
fn a_cut_silences_the_sound_that_it_cuts() {
    let long = ("long", Audio::constant(&[0.5], 48_000), Format::Int16);
    let silent = ("silent", Audio::constant(&[0.0], 4_800), Format::Int16);
    let mut host = Host::new("cut", config(2, 1), &[long, silent]);
    host.play(&Sound::new().at(0.4).s("long").cut(1));
    host.play(&Sound::new().at(0.65).s("silent").cut(1));
    let output = host.render(1.5, 2, &nrt::plugins());
    let cut = frame(0.65);
    for channel in 0..2 {
        assert_level(&output, channel, cut - 96..cut, 0.5 * FRAC_1_SQRT_2 * 0.4);
        // The envelope of cut_gate in the gate runs at control rate, a block
        // of 64 frames at a time, and fades in fadeTime, 0.001 seconds.
        let after = peak(&output, channel, cut + frame(0.005)..output.frames());
        assert!(after == 0.0, "channel {channel}: {after} after the cut");
    }
}

#[test]
#[ignore = "needs scsynth 3.14.1: cargo test -- --ignored"]
fn the_comb_delay_echoes_after_the_dry_sound_ends() {
    assert_echoes("comb", Delay::Comb, &nrt::plugins());
}

#[cfg(feature = "sc3-plugins")]
#[test]
#[ignore = "needs scsynth 3.14.1 and sc3-plugins: cargo test --all-features -- --ignored"]
fn the_switch_delay_loads_with_sc3_plugins_and_echoes_after_the_dry_sound_ends() {
    assert_echoes("switch", Delay::Switch, &nrt::sc3_plugins());
}

#[test]
#[ignore = "needs scsynth 3.14.1: cargo test -- --ignored"]
fn on_1_channel_a_sound_sums_its_sample_and_the_reverb_leaves_the_next_orbit_silent() {
    // scsynth writes each output bus as a block leaves it, so the private
    // buses of both orbits, from bus 1, sit among its 9 output channels.
    // Channel 2 is the dry bus of orbit 0, 3 and 4 its effect bus of 2
    // channels, and 5 to 8 the buses of orbit 1.
    let mut config = config(1, 2);
    config.first_bus = 1;
    let stereo = (
        "stereo",
        Audio::constant(&[0.5, -0.25], 4_800),
        Format::Float,
    );
    let mut host = Host::new("one", config, &[stereo]);
    host.play(&Sound::new().at(0.4).s("stereo"));
    host.play(&Sound::new().at(0.7).s("stereo").with("room", 0.3));
    let output = host.render(1.5, 9, &nrt::plugins());
    // DirtPan sums the channels of the sample on 1 channel, and the gate
    // scales the sum by the amp 0.4 of the orbit.
    let frames = frame(0.4) + ONSET..frame(0.4) + 4_700;
    assert_level(&output, 0, frames, (0.5 - 0.25) * 0.4);
    let second = peak(&output, 4, frame(0.7)..output.frames());
    assert!(
        second > 0.001,
        "{second} on the second channel of the reverb"
    );
    for channel in 5..9 {
        let next = peak(&output, channel, 0..output.frames());
        assert!(next == 0.0, "channel {channel}: {next} on a bus of orbit 1");
    }
    // The output is the sum of the buses that the monitor of orbit 0 reads,
    // so the monitor of orbit 1 adds nothing.
    for frame in frame(0.2)..output.frames() {
        let orbit = output.at(frame, 2) + output.at(frame, 3);
        assert!(
            output.at(frame, 0).to_bits() == orbit.to_bits(),
            "frame {frame}: the output is not the sum of the buses of orbit 0"
        );
    }
}

#[test]
#[ignore = "needs scsynth 3.14.1: cargo test -- --ignored"]
fn on_4_channels_a_sound_comes_out_on_the_channel_of_its_pan() {
    let mono = ("mono", Audio::constant(&[0.5], 4_800), Format::Int16);
    let mut host = Host::new("four", config(4, 1), &[mono]);
    let pans = [0.0, 0.25, 0.5, 0.75];
    let times = [0.4, 0.7, 1.0, 1.3];
    for (pan, time) in pans.into_iter().zip(times) {
        host.play(&Sound::new().at(time).s("mono").pan(pan));
    }
    let output = host.render(1.6, 4, &nrt::plugins());
    // DirtSplayAz turns the pan p, which play sends as 2p - 1, into the
    // position 2p of PanAz, whose 4 channels sit 0.5 apart from channel 0 at
    // the orientation 0. Channel 4p takes the sound alone, at the 0.5 of the
    // sample times the amp 0.4 of the gate.
    for (sounding, time) in times.into_iter().enumerate() {
        let frames = frame(time) + ONSET..frame(time) + 4_700;
        for channel in 0..4 {
            let level = if channel == sounding { 0.5 * 0.4 } else { 0.0 };
            assert_level(&output, channel, frames.clone(), level);
        }
    }
}

#[test]
#[ignore = "needs scsynth 3.14.1: cargo test -- --ignored"]
fn the_check_of_failures_finds_each_line_that_scsynth_prints_for_a_failure() {
    let folder = Folder::new("failures");
    let mut score = Score::default();
    score.message(
        0.0,
        Command::SNew(SNew {
            def: "missing".to_owned(),
            id: NewNodeId::AUTO,
            action: AddAction::Head,
            target: NodeId(0),
            controls: Vec::new(),
        }),
    );
    score.message(
        0.0,
        Command::NSet(NSet {
            node: NodeId(9_999),
            controls: vec![ControlSetting::new("cut_gate", 0)],
        }),
    );
    score.message(
        0.0,
        Command::GNew(GNew {
            groups: vec![NewGroup {
                id: NewNodeId::new(5).expect("a group id"),
                action: AddAction::Tail,
                target: NodeId(777),
            }],
        }),
    );
    score.message(
        0.0,
        Command::BAllocRead(BAllocRead {
            buffer: BufferId::new(0).expect("a buffer"),
            path: "/nonexistent/sample.wav".to_owned(),
            file_frame: 0,
            frames: -1,
            completion: None,
        }),
    );
    let mut builder = SynthDefBuilder::new("broken", Vec::new()).expect("a definition");
    builder
        .add(NewUGen::new("NoSuchUGen", Rate::Audio))
        .expect("a UGen");
    let broken = builder.finish().expect("a definition");
    score.message(
        0.0,
        Command::DRecv(DRecv {
            data: broken.as_bytes().expect("the bytes of a definition"),
            completion: None,
        }),
    );
    // Each command fails, so no node is left for scsynth to free at its exit.
    let render = folder.render(&score, 2, &nrt::plugins());
    assert_eq!(
        render.failures(),
        [
            "*** ERROR: SynthDef missing not found",
            "FAILURE IN SERVER /s_new SynthDef not found",
            "FAILURE IN SERVER /n_set Node 9999 not found",
            "FAILURE IN SERVER /g_new Group 777 not found",
            "File '/nonexistent/sample.wav' could not be opened: System error : No such file or directory.",
            "exception in GraphDef_Recv: UGen 'NoSuchUGen' not installed.",
        ],
        "{}",
        render.text
    );
}
