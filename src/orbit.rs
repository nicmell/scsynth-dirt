//! An orbit of an instance: its node ids, its buses, and the defaults of its
//! sounds.

use std::collections::BTreeMap;

use scsynth_osc::NodeId;

use crate::config::{Config, int, orbit_buses};
use crate::sound::Value;

/// One orbit of a [`Dirt`](crate::Dirt): a group for its sounds, five global
/// synths after the group, three private buses and the output buses of its
/// monitor, and the defaults of its sounds.
///
/// Orbit `k` takes the 6 node ids from `fixed_nodes + 6 * k` of its
/// [`Config`]: its group, then its monitor, rms, leslie, reverb and delay. Its
/// synth bus, its dry bus and its effect bus follow each other, `channels`
/// apart, after the private buses of orbit `k - 1`.
#[derive(Clone, Debug, PartialEq)]
pub struct Orbit {
    /// The index of the orbit among the orbits of its instance.
    index: usize,
    /// The group of the sounds.
    group: NodeId,
    /// The global synths: the monitor, the rms, the leslie, the reverb and
    /// the delay.
    synths: [NodeId; 5],
    /// The synth bus.
    synth_bus: u32,
    /// The dry bus.
    dry_bus: u32,
    /// The effect bus.
    effect_bus: u32,
    /// The first output bus.
    out_bus: u32,
    /// The defaults of the parameters of a sound, by name.
    defaults: BTreeMap<String, Value>,
}

impl Orbit {
    /// The orbits of `config`, a configuration that [`Config::check`]
    /// accepts, in order.
    pub(crate) fn all(config: &Config) -> Vec<Self> {
        let buses = orbit_buses(config.channels);
        (0..config.orbits)
            .enumerate()
            .map(|(index, number)| {
                let group = config.fixed_nodes + int(6 * number);
                let synth_bus = config.first_bus + number * buses;
                Self {
                    index,
                    group: NodeId(group),
                    synths: [1, 2, 3, 4, 5].map(|offset| NodeId(group + offset)),
                    synth_bus,
                    dry_bus: synth_bus + config.channels,
                    effect_bus: synth_bus + 2 * config.channels,
                    out_bus: config.out_bus,
                    defaults: defaults(),
                }
            })
            .collect()
    }

    /// The index of the orbit among the orbits of its instance, from 0.
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    /// The group of the sounds of the orbit, at the head of the root group.
    #[must_use]
    pub fn group(&self) -> NodeId {
        self.group
    }

    /// The monitor: the synth of `dirt_monitor`, which sends the dry bus and
    /// the effect bus to the output buses.
    #[must_use]
    pub fn monitor(&self) -> NodeId {
        self.synths[0]
    }

    /// The synth of `dirt_rms`, the meter of the orbit.
    #[must_use]
    pub fn rms(&self) -> NodeId {
        self.synths[1]
    }

    /// The synth of `dirt_leslie`, the rotary speaker of the orbit.
    #[must_use]
    pub fn leslie(&self) -> NodeId {
        self.synths[2]
    }

    /// The synth of `dirt_reverb`, the reverb of the orbit.
    #[must_use]
    pub fn reverb(&self) -> NodeId {
        self.synths[3]
    }

    /// The synth of `dirt_delay`, the delay of the orbit.
    #[must_use]
    pub fn delay(&self) -> NodeId {
        self.synths[4]
    }

    /// The synth bus: the first of the `channels` buses that the sources and
    /// the effects of a sound write, and that its gate reads.
    #[must_use]
    pub fn synth_bus(&self) -> u32 {
        self.synth_bus
    }

    /// The dry bus: the first of the `channels` buses that the gate of a sound
    /// writes, and that the global synths read.
    #[must_use]
    pub fn dry_bus(&self) -> u32 {
        self.dry_bus
    }

    /// The effect bus: the first of the buses that the global effects write,
    /// and that the monitor and the rms read.
    #[must_use]
    pub fn effect_bus(&self) -> u32 {
        self.effect_bus
    }

    /// The first of the `channels` output buses of the monitor.
    #[must_use]
    pub fn out_bus(&self) -> u32 {
        self.out_bus
    }

    /// The value of the default `name`, if the orbit has one.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.defaults.get(name)
    }

    /// Sets the default `name` to `value`, in place of any value that it had,
    /// as `DirtOrbit.set` of SuperDirt does.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<Value>) -> &mut Self {
        self.defaults.insert(name.into(), value.into());
        self
    }
}

/// The defaults of a new orbit, as `DirtOrbit.makeDefaultParentEvent` of
/// SuperDirt sets them.
fn defaults() -> BTreeMap<String, Value> {
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
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value))
    .collect()
}
