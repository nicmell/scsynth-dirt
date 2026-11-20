//! The configuration of an instance: the structure of its orbits, the node
//! ids and the buses that the host gives it, and its server, with the
//! refusals of a configuration.

use std::fmt;
use std::ops::Range;

use scsynth_osc::NodeId;
use scsynth_synthdef::BuildError;

use crate::synthdefs::Delay;
use crate::vowel::Register;

/// The structure of an instance of [`Dirt`](crate::Dirt): its orbits, the
/// node ids and the buses that the host gives it, and its server. It holds
/// nothing that a sound can change.
///
/// The private buses of an orbit follow each other: its synth bus, its dry
/// bus, then its effect bus. For 1 channel, the effect bus has a second
/// channel that no synth reads. The buses of the next orbit come after them.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Config {
    /// The channel count of the orbits, 1 or more.
    pub channels: u32,
    /// The number of orbits, 1 or more. SuperDirt starts 12.
    pub orbits: u32,
    /// The group of the host. The instance makes its nodes inside it, and
    /// never frees it.
    pub root: NodeId,
    /// The first of `6 * orbits` node ids: for each orbit its group, then its
    /// monitor, rms, leslie, reverb and delay.
    pub fixed_nodes: i32,
    /// The node ids for the groups of the sounds. They start over at the end
    /// of the range.
    pub sound_nodes: Range<i32>,
    /// The first of the private audio buses: for each orbit its synth bus, its
    /// dry bus, its effect bus. An orbit takes `3 * channels` of them, and 4
    /// for 1 channel.
    pub first_bus: u32,
    /// The first of the `channels` output buses of the monitors.
    pub out_bus: u32,
    /// The sample rate of the server, in hertz.
    pub sample_rate: f64,
    /// The delay line of the orbits. [`Delay::Switch`] needs the feature
    /// `sc3-plugins`.
    pub delay: Delay,
    /// The voice register of the vowels.
    pub register: Register,
    /// The seconds between the OSC epoch and time 0 of `when`: 2,208,988,800
    /// for Unix time, 0 for a score of `scsynth -N`.
    pub epoch: f64,
}

impl Config {
    /// The refusals of [`Dirt::new`](crate::Dirt::new), but those of the
    /// build of the definitions.
    pub(crate) fn check(&self) -> Result<(), ConfigError> {
        if self.channels == 0 {
            return Err(ConfigError::NoChannels);
        }
        if self.orbits == 0 {
            return Err(ConfigError::NoOrbits);
        }
        if self.fixed_nodes < 1 {
            return Err(ConfigError::FixedNodesUnder1(self.fixed_nodes));
        }
        if self.sound_nodes.is_empty() {
            return Err(ConfigError::EmptySoundNodes(self.sound_nodes.clone()));
        }
        if self.sound_nodes.start < 1 {
            return Err(ConfigError::SoundNodesUnder1(self.sound_nodes.clone()));
        }
        if self.root.0 < 0 {
            return Err(ConfigError::RootUnder0(self.root));
        }
        let fixed = span(self.fixed_nodes.into(), 6 * u64::from(self.orbits))
            .ok_or(ConfigError::FixedNodesPastI32)?;
        let buses = u64::from(self.orbits) * u64::from(orbit_buses(self.channels));
        let private = span(self.first_bus.into(), buses).ok_or(ConfigError::PrivateBusesPastI32)?;
        let out =
            span(self.out_bus.into(), self.channels.into()).ok_or(ConfigError::OutBusesPastI32)?;
        if fixed.contains(&self.root.0.into()) {
            return Err(ConfigError::RootInFixedNodes(self.root));
        }
        if self.sound_nodes.contains(&self.root.0) {
            return Err(ConfigError::RootInSoundNodes(self.root));
        }
        let sounds = i64::from(self.sound_nodes.start)..i64::from(self.sound_nodes.end);
        if overlap(&fixed, &sounds) {
            let shared = self.fixed_nodes.max(self.sound_nodes.start);
            return Err(ConfigError::FixedNodeInSoundNodes(shared));
        }
        if overlap(&out, &private) {
            let shared = self.out_bus.max(self.first_bus);
            return Err(ConfigError::OutBusInPrivateBuses(shared));
        }
        if !(self.sample_rate.is_finite() && self.sample_rate > 0.0) {
            return Err(ConfigError::BadSampleRate(self.sample_rate));
        }
        if !self.epoch.is_finite() {
            return Err(ConfigError::BadEpoch(self.epoch));
        }
        Ok(())
    }
}

/// The private buses of one orbit of `channels` channels: 3 buses of
/// `channels` channels each, and 4 buses for 1 channel.
pub(crate) fn orbit_buses(channels: u32) -> u32 {
    if channels == 1 {
        4
    } else {
        // A count that saturates passes the largest i32 in Config::check.
        channels.saturating_mul(3)
    }
}

/// A number of a configuration that [`Config::check`] accepts, as an int of
/// OSC: an offset among its node ids, a bus or an orbit index.
#[allow(
    clippy::cast_possible_wrap,
    reason = "Config::check refuses the node ids and the buses past i32::MAX, and an orbit index is under the count of the node ids"
)]
pub(crate) fn int(number: u32) -> i32 {
    number as i32
}

/// The `count` numbers from `first`, or none if one of them passes
/// `i32::MAX`.
fn span(first: i64, count: u64) -> Option<Range<i64>> {
    let end = first.checked_add(i64::try_from(count).ok()?)?;
    (end <= i64::from(i32::MAX) + 1).then_some(first..end)
}

/// Whether the ranges `a` and `b` share a number.
fn overlap(a: &Range<i64>, b: &Range<i64>) -> bool {
    a.start < b.end && b.start < a.end
}

/// Why [`Dirt::new`](crate::Dirt::new) refuses a [`Config`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ConfigError {
    /// `channels` is 0.
    NoChannels,
    /// `orbits` is 0.
    NoOrbits,
    /// A definition of [`synthdefs::core`](crate::synthdefs::core) does not
    /// build for `channels` and `delay`, or breaks a limit of scsynth: the
    /// error of the build.
    Definition(BuildError),
    /// `fixed_nodes`, this first node id of the orbits, is under 1.
    FixedNodesUnder1(i32),
    /// `sound_nodes`, this range of node ids, starts under 1.
    SoundNodesUnder1(Range<i32>),
    /// `sound_nodes`, this range of node ids, is empty.
    EmptySoundNodes(Range<i32>),
    /// `root`, this node id, is under 0.
    RootUnder0(NodeId),
    /// `root`, this group, is one of the `6 * orbits` node ids from
    /// `fixed_nodes`.
    RootInFixedNodes(NodeId),
    /// `root`, this group, is in `sound_nodes`.
    RootInSoundNodes(NodeId),
    /// This node id is one of the `6 * orbits` from `fixed_nodes`, and in
    /// `sound_nodes`: the lowest such node id.
    FixedNodeInSoundNodes(i32),
    /// This bus is one of the `channels` output buses from `out_bus`, and one
    /// of the private buses of the orbits from `first_bus`: the lowest such
    /// bus.
    OutBusInPrivateBuses(u32),
    /// The `6 * orbits` node ids from `fixed_nodes` pass `i32::MAX`.
    FixedNodesPastI32,
    /// The private buses of the orbits from `first_bus` pass `i32::MAX`.
    PrivateBusesPastI32,
    /// The `channels` output buses from `out_bus` pass `i32::MAX`.
    OutBusesPastI32,
    /// `sample_rate`, this number, is not a finite number above 0.
    BadSampleRate(f64),
    /// `epoch`, this number, is not finite.
    BadEpoch(f64),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoChannels => write!(
                f,
                "the channel count is 0, but an orbit has 1 channel or more"
            ),
            Self::NoOrbits => write!(
                f,
                "the orbit count is 0, but an instance has 1 orbit or more"
            ),
            Self::Definition(_) => write!(
                f,
                "a synth definition does not build for the channel count and the delay"
            ),
            Self::FixedNodesUnder1(first) => write!(
                f,
                "the node ids of the orbits start at {first}, but a node id of the instance is 1 or more"
            ),
            Self::SoundNodesUnder1(range) => write!(
                f,
                "the range {range:?} of node ids of the sounds starts under 1, but a node id of the instance is 1 or more"
            ),
            Self::EmptySoundNodes(range) => {
                write!(f, "the range {range:?} of node ids of the sounds is empty")
            }
            Self::RootUnder0(root) => write!(
                f,
                "the root group {} is under 0, but the id of a group is 0 or more",
                root.0
            ),
            Self::RootInFixedNodes(root) => write!(
                f,
                "the root group {} is one of the node ids of the orbits",
                root.0
            ),
            Self::RootInSoundNodes(root) => write!(
                f,
                "the root group {} is in the range of node ids of the sounds",
                root.0
            ),
            Self::FixedNodeInSoundNodes(id) => write!(
                f,
                "the node id {id} is one of the orbits, and in the range of node ids of the sounds"
            ),
            Self::OutBusInPrivateBuses(bus) => write!(
                f,
                "the bus {bus} is an output bus, and a private bus of an orbit"
            ),
            Self::FixedNodesPastI32 => {
                write!(f, "the node ids of the orbits pass the largest i32")
            }
            Self::PrivateBusesPastI32 => {
                write!(f, "the private buses of the orbits pass the largest i32")
            }
            Self::OutBusesPastI32 => write!(f, "the output buses pass the largest i32"),
            Self::BadSampleRate(rate) => {
                write!(f, "the sample rate {rate} is not a finite number above 0")
            }
            Self::BadEpoch(epoch) => write!(f, "the epoch {epoch} is not finite"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Definition(error) => Some(error),
            _ => None,
        }
    }
}
