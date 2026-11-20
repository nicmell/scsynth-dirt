//! The library of samples: names, each with its sound files in order, on
//! buffers of scsynth, and the replies of scsynth to their loads.

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

use scsynth_osc::command::buffer::{BAllocRead, BFree, BQuery};
use scsynth_osc::reply::buffer::BufferInfo;
use scsynth_osc::{BufferId, Command, Completion, Reply};

use crate::sclang;

/// A library of samples: names, each with its samples in order. A sample is
/// a sound file that scsynth reads into a buffer. The buffers come from a
/// range of buffer numbers that the host gives.
///
/// The library does no I/O. [`Library::add`], [`Library::append`] and
/// [`Library::remove`] return commands for scsynth, and the host sends them
/// in order. The host passes the replies of scsynth to [`Library::reply`].
#[derive(Clone, Debug)]
pub struct Library {
    /// The buffer numbers that the samples take.
    buffers: Range<i32>,
    /// The samples of each name, in order.
    names: BTreeMap<String, Vec<Sample>>,
    /// The buffers that the samples hold, each with the name and the index of
    /// its sample.
    held: BTreeMap<BufferId, (String, usize)>,
}

impl Library {
    /// An empty library, whose samples take the buffer numbers of `buffers`.
    ///
    /// # Errors
    ///
    /// [`LibraryError::EmptyRange`] for an empty range, and
    /// [`LibraryError::NegativeStart`] for a range that starts under 0.
    pub fn new(buffers: Range<i32>) -> Result<Self, LibraryError> {
        if buffers.is_empty() {
            return Err(LibraryError::EmptyRange(buffers));
        }
        if buffers.start < 0 {
            return Err(LibraryError::NegativeStart(buffers));
        }
        Ok(Self {
            buffers,
            names: BTreeMap::new(),
            held: BTreeMap::new(),
        })
    }

    /// Replaces the samples of `name` with one sample per path of `paths`, in
    /// their order. A new sample takes the lowest free number of the range,
    /// and a number that the call frees is free for it.
    ///
    /// The commands are the `/b_free` of each buffer that the call replaced,
    /// in the order of the old samples. Then come one
    /// `/b_allocRead buffer path 0 -1` per path, each with `/b_query buffer`
    /// as its completion message.
    ///
    /// # Errors
    ///
    /// [`LibraryError::TooFewBuffers`] if the range has too few free numbers
    /// for the paths. The library then stays as it was.
    pub fn add(
        &mut self,
        name: &str,
        paths: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Vec<Command>, LibraryError> {
        let paths: Vec<String> = paths.into_iter().map(Into::into).collect();
        let replaced = self.names.get(name).map_or(0, Vec::len);
        self.check(paths.len(), replaced)?;
        let mut commands = self.remove(name);
        commands.extend(self.load(name, paths));
        Ok(commands)
    }

    /// Adds one sample per path of `paths` after the samples of `name`, in
    /// their order, with the `/b_allocRead` commands of [`Library::add`]. A
    /// name that the library lacks starts with no samples.
    ///
    /// # Errors
    ///
    /// [`LibraryError::TooFewBuffers`] if the range has too few free numbers
    /// for the paths. The library then stays as it was.
    pub fn append(
        &mut self,
        name: &str,
        paths: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Vec<Command>, LibraryError> {
        let paths: Vec<String> = paths.into_iter().map(Into::into).collect();
        self.check(paths.len(), 0)?;
        Ok(self.load(name, paths))
    }

    /// Takes `name` out of the library, and returns the `/b_free` of the
    /// buffer of each of its samples, in their order. The list is empty for a
    /// name that the library lacks.
    #[must_use]
    pub fn remove(&mut self, name: &str) -> Vec<Command> {
        let Some(samples) = self.names.remove(name) else {
            return Vec::new();
        };
        samples
            .iter()
            .map(|sample| {
                self.held.remove(&sample.buffer);
                Command::BFree(BFree {
                    buffer: sample.buffer,
                    completion: None,
                })
            })
            .collect()
    }

    /// Takes a reply of scsynth.
    ///
    /// - Each entry of a `/b_info` for the buffer of a sample gives the
    ///   sample its frames, its channels and its sample rate, and the sample
    ///   is then loaded. With under 1 frame, under 1 channel, or a sample rate
    ///   that is not a finite number above 0, the sample is failed instead:
    ///   scsynth answers `/b_query` of a buffer without data with zeros.
    /// - A `/fail` of `/b_allocRead` whose value is the buffer of a sample
    ///   marks the sample as failed: scsynth could not read its file.
    /// - Every other reply does nothing.
    pub fn reply(&mut self, reply: &Reply) {
        match reply {
            Reply::BInfo(info) => {
                for &entry in &info.buffers {
                    if let Some(sample) = self.sample_mut(entry.buffer) {
                        sample.state = sample_info(entry).map_or(State::Failed, State::Loaded);
                    }
                }
            }
            Reply::Fail(fail) if fail.command == "/b_allocRead" => {
                let buffer = fail.value.and_then(BufferId::new);
                if let Some(sample) = buffer.and_then(|buffer| self.sample_mut(buffer)) {
                    sample.state = State::Failed;
                }
            }
            _ => {}
        }
    }

    /// The sample at index `n` of `name`, as `DirtSoundLibrary.getEvent` of
    /// SuperDirt picks it with `wrapAt(index.asInteger)`: `n` truncated
    /// toward 0 to an `i32`, then wrapped to the count, so a negative index
    /// counts from the end. A value past the range of `i32` saturates, and a
    /// NaN counts as 0. None for a name that the library lacks or that has no
    /// samples.
    #[must_use]
    pub fn sample(&self, name: &str, n: f64) -> Option<&Sample> {
        let samples = self.names.get(name)?;
        let count = i32::try_from(samples.len()).ok()?;
        let index = sclang::as_integer(n).checked_rem_euclid(count)?;
        samples.get(usize::try_from(index).ok()?)
    }

    /// The samples of `name`, in order, or none for a name that the library
    /// lacks.
    #[must_use]
    pub fn samples(&self, name: &str) -> Option<&[Sample]> {
        self.names.get(name).map(Vec::as_slice)
    }

    /// The names of the library, in the order of their bytes.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names.keys().map(String::as_str)
    }

    /// Refuses `needed` new samples if the range has fewer free numbers, with
    /// the `replaced` numbers that the call frees.
    fn check(&self, needed: usize, replaced: usize) -> Result<(), LibraryError> {
        let free = self.buffers.len() - self.held.len() + replaced;
        if needed > free {
            Err(LibraryError::TooFewBuffers(needed - free))
        } else {
            Ok(())
        }
    }

    /// Adds one sample per path after the samples of `name`, each on the
    /// lowest free number of the range, and returns their `/b_allocRead`
    /// commands. The range must have a free number for each path.
    fn load(&mut self, name: &str, paths: Vec<String>) -> Vec<Command> {
        let free: Vec<BufferId> = self
            .buffers
            .clone()
            .filter_map(BufferId::new)
            .filter(|buffer| !self.held.contains_key(buffer))
            .take(paths.len())
            .collect();
        let samples = self.names.entry(name.to_owned()).or_default();
        let mut commands = Vec::with_capacity(paths.len());
        for (buffer, path) in free.into_iter().zip(paths) {
            self.held.insert(buffer, (name.to_owned(), samples.len()));
            commands.push(alloc_read(buffer, &path));
            samples.push(Sample {
                buffer,
                path,
                state: State::Waiting,
            });
        }
        commands
    }

    /// The sample that holds `buffer`, if any.
    fn sample_mut(&mut self, buffer: BufferId) -> Option<&mut Sample> {
        let (name, index) = self.held.get(&buffer)?;
        self.names.get_mut(name)?.get_mut(*index)
    }
}

/// `/b_allocRead buffer path 0 -1`, with `/b_query buffer` as its completion
/// message.
fn alloc_read(buffer: BufferId, path: &str) -> Command {
    let query = Command::BQuery(BQuery {
        buffers: vec![buffer],
    });
    Command::BAllocRead(BAllocRead {
        buffer,
        path: path.to_owned(),
        file_frame: 0,
        frames: -1,
        completion: Some(Completion::Message(Box::new(query))),
    })
}

/// The frames, the channels and the sample rate of an entry of `/b_info`, or
/// none for a buffer without data: under 1 frame, under 1 channel, or a
/// sample rate that is not a finite number above 0.
fn sample_info(entry: BufferInfo) -> Option<SampleInfo> {
    let frames = u32::try_from(entry.frames)
        .ok()
        .filter(|&frames| frames > 0)?;
    let channels = u32::try_from(entry.channels)
        .ok()
        .filter(|&channels| channels > 0)?;
    let sample_rate = f64::from(entry.sample_rate);
    (sample_rate.is_finite() && sample_rate > 0.0).then_some(SampleInfo {
        frames,
        channels,
        sample_rate,
    })
}

/// A sample of a [`Library`]: a sound file, and the buffer of scsynth that
/// holds it. A new sample waits for the replies of scsynth to its load, and
/// is then loaded or failed.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    /// The buffer that holds the sample.
    buffer: BufferId,
    /// The path of the sound file.
    path: String,
    /// What the replies of scsynth said of the load.
    state: State,
}

impl Sample {
    /// The buffer that holds the sample.
    #[must_use]
    pub fn buffer(&self) -> BufferId {
        self.buffer
    }

    /// The path of the sound file.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The frames, the channels and the sample rate of the buffer, once the
    /// sample is loaded.
    #[must_use]
    pub fn info(&self) -> Option<SampleInfo> {
        match self.state {
            State::Loaded(info) => Some(info),
            State::Waiting | State::Failed => None,
        }
    }

    /// Whether scsynth could not load the sample.
    #[must_use]
    pub fn failed(&self) -> bool {
        self.state == State::Failed
    }
}

/// What the replies of scsynth said of the load of a [`Sample`].
#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    /// No reply yet.
    Waiting,
    /// A `/b_info` of the buffer with data.
    Loaded(SampleInfo),
    /// A `/fail` of the read, or a `/b_info` of the buffer without data.
    Failed,
}

/// The frames, the channels and the sample rate of the buffer of a loaded
/// [`Sample`], from its `/b_info`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampleInfo {
    /// The number of frames. A frame holds one value of each channel.
    pub frames: u32,
    /// The number of channels.
    pub channels: u32,
    /// The sample rate, in hertz.
    pub sample_rate: f64,
}

/// Why a [`Library`] refuses its range of buffer numbers, or the paths of a
/// call.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LibraryError {
    /// The range of buffer numbers is empty.
    EmptyRange(Range<i32>),
    /// The range of buffer numbers starts under 0, but a buffer number is 0
    /// or more.
    NegativeStart(Range<i32>),
    /// The range has too few free numbers for the paths of a call: this many
    /// lack.
    TooFewBuffers(usize),
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRange(range) => {
                write!(f, "the range {range:?} of buffer numbers is empty")
            }
            Self::NegativeStart(range) => write!(
                f,
                "the range {range:?} of buffer numbers starts under 0, but a buffer number is 0 or more"
            ),
            Self::TooFewBuffers(lacking) => write!(
                f,
                "the range of buffer numbers is {lacking} short of free numbers for the new samples"
            ),
        }
    }
}

impl std::error::Error for LibraryError {}
