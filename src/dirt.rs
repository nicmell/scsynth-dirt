//! An instance of SuperDirt on scsynth: its configuration, its synth
//! definitions and its orbits, and the commands that start and stop it.

use std::iter;

use scsynth_osc::command::group::{GNew, NewGroup};
use scsynth_osc::command::node::{NFree, NRun, NodeRun};
use scsynth_osc::command::synth::SNew;
use scsynth_osc::command::synth_def::DRecv;
use scsynth_osc::command::top_level::ErrorMode;
use scsynth_osc::{
    AddAction, Bundle, Command, Completion, ControlSetting, NewNodeId, NodeId, Packet, Reply,
    Timetag,
};
use scsynth_synthdef::{BuildError, SynthDef};

use crate::config::{Config, ConfigError, int};
use crate::event::{Dropped, Event, Live};
use crate::library::Library;
use crate::orbit::Orbit;
use crate::sound::Sound;
use crate::synthdefs;

/// The global definitions at the head of [`synthdefs::core`]: the monitor,
/// the rms, the delay, the reverb and the leslie.
const GLOBAL_DEFINITIONS: usize = 5;

/// An instance of SuperDirt on scsynth: the orbits of a [`Config`], and the
/// 25 synth definitions of its channel count.
///
/// The instance does no I/O. [`Dirt::start`], [`Dirt::stop`] and
/// [`Dirt::play`] return commands for scsynth, and the host sends them in
/// order. The host passes the replies of scsynth to [`Dirt::reply`].
#[derive(Clone, Debug)]
pub struct Dirt {
    /// The configuration.
    config: Config,
    /// The 25 definitions, in the order of [`synthdefs::core`].
    definitions: Vec<SynthDef>,
    /// The orbits, in order.
    orbits: Vec<Orbit>,
    /// The commands of [`Dirt::start`].
    start: Vec<Command>,
    /// The sounds that may still play, and the next node id for a sound.
    live: Live,
}

impl Dirt {
    /// The instance of `config`, with the 25 definitions of
    /// [`synthdefs::core`] for its channel count and its delay line, built
    /// once. Each orbit starts with the defaults of SuperDirt.
    ///
    /// # Errors
    ///
    /// The [`ConfigError`] of the first fault of `config`.
    pub fn new(config: Config) -> Result<Self, ConfigError> {
        config.check()?;
        let definitions =
            synthdefs::core(config.channels, config.delay).map_err(ConfigError::Definition)?;
        let orbits = Orbit::all(&config);
        let start = start_commands(&config, &definitions, &orbits)?;
        let live = Live::new(config.sound_nodes.clone());
        Ok(Self {
            config,
            definitions,
            orbits,
            start,
            live,
        })
    }

    /// The configuration of the instance.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The orbits, in order.
    #[must_use]
    pub fn orbits(&self) -> &[Orbit] {
        &self.orbits
    }

    /// The orbit at `index`, or none past the count.
    #[must_use]
    pub fn orbit(&self, index: usize) -> Option<&Orbit> {
        self.orbits.get(index)
    }

    /// The orbit at `index`, to change its defaults, or none past the count.
    pub fn orbit_mut(&mut self, index: usize) -> Option<&mut Orbit> {
        self.orbits.get_mut(index)
    }

    /// The 25 definitions of the instance, in the order of
    /// [`synthdefs::core`], for a host that loads them in another way.
    #[must_use]
    pub fn definitions(&self) -> &[SynthDef] {
        &self.definitions
    }

    /// The commands that load the definitions and make the nodes of the
    /// instance: one `/d_recv` per definition, in the order of
    /// [`Dirt::definitions`], safe to send in order with no wait.
    ///
    /// The first 5, the global definitions, have no completion. The
    /// definition `5 + j` carries, as its completion, one bundle with the
    /// immediate time tag: the node messages of each orbit `k` with
    /// `k mod 20 == j`, in the order of the orbits, or no completion without
    /// such an orbit. The node messages of an orbit make its group at the
    /// head of the root group, then each of its global synths after the
    /// group, paused, in the order monitor, rms, leslie, reverb, delay. So the
    /// synths sit after the group in the order delay, reverb, leslie, rms,
    /// monitor.
    ///
    /// The instance also forgets what [`Dirt::play`] keeps: the last
    /// parameters of the global synths and the live sounds. The node ids of
    /// the sounds start over from the first of `sound_nodes`.
    #[must_use]
    pub fn start(&mut self) -> Vec<Command> {
        self.clear();
        self.start.clone()
    }

    /// The commands that free the nodes of the instance: per orbit, in order,
    /// one bundle with the immediate time tag that holds `/error -1`, one
    /// `/n_free` of the group and of the five global synths, and `/error -2`.
    ///
    /// The instance also forgets what [`Dirt::play`] keeps, as
    /// [`Dirt::start`] does.
    #[must_use]
    pub fn stop(&mut self) -> Vec<Bundle<Command>> {
        self.clear();
        self.orbits
            .iter()
            .map(|orbit| {
                let synths = global_synths(orbit).map(|(_, synth)| synth);
                let nodes = iter::once(orbit.group()).chain(synths).collect();
                Bundle {
                    time: Timetag::IMMEDIATELY,
                    packets: vec![
                        Packet::Message(Command::Error(ErrorMode::BundleOff)),
                        Packet::Message(Command::NFree(NFree { nodes })),
                        Packet::Message(Command::Error(ErrorMode::BundleOn)),
                    ],
                }
            })
            .collect()
    }

    /// The bundles that play `sound` with the samples of `library`, as
    /// `DirtEvent.play` of SuperDirt sends them for a sample sound. Each has
    /// the time tag of the sound: `when + lag + offset * |speed|` plus the
    /// `epoch` of the configuration, or the immediate one without `when`.
    ///
    /// - With a `cut` that is not 0, a first bundle holds `/error -1`, then
    ///   `/n_set group cut_gate 0` for each live sound that the cut reaches,
    ///   in the order of their starts.
    /// - The bundle of the sound resumes the global synths of its orbit, and
    ///   sets their parameters that changed since the last sound of the
    ///   orbit. It then makes the group of the sound at the tail of the group
    ///   of the orbit, and the synths of the sound in it: the player of the
    ///   sample, the effects of its modules, and its gate.
    ///
    /// The group takes the next node id of `sound_nodes` that no live sound
    /// holds, and the sound joins the live sounds. With every id live, the
    /// oldest live sound leaves first, and its id serves again. A sound also
    /// leaves at the `/n_end` of its group, which [`Dirt::reply`] takes, or
    /// once a later sound starts after its end.
    ///
    /// # Errors
    ///
    /// The [`Dropped`] reason of a sound that does not play. The instance then
    /// stays as it was.
    pub fn play(
        &mut self,
        library: &Library,
        sound: &Sound,
    ) -> Result<Vec<Bundle<Command>>, Dropped> {
        let event = Event::new(&self.config, &self.orbits, library, sound)?;
        Ok(event.send(&mut self.orbits, &mut self.live))
    }

    /// Takes a reply of scsynth: the `/n_end` of the group of a live sound
    /// takes the sound out of the live sounds, so that no later cut reaches
    /// it. Every other reply does nothing.
    pub fn reply(&mut self, reply: &Reply) {
        if let Reply::NEnd(state) = reply {
            self.live.end(state.node);
        }
    }

    /// Forgets the last parameters of the global synths and the live sounds,
    /// and starts the node ids of the sounds over.
    fn clear(&mut self) {
        self.live.clear();
        for orbit in &mut self.orbits {
            orbit.clear_effects();
        }
    }
}

/// The commands of [`Dirt::start`] for `definitions` and `orbits` of
/// `config`.
fn start_commands(
    config: &Config,
    definitions: &[SynthDef],
    orbits: &[Orbit],
) -> Result<Vec<Command>, ConfigError> {
    let events = definitions.len() - GLOBAL_DEFINITIONS;
    let mut nodes = vec![Vec::new(); events];
    for (number, orbit) in (0..config.orbits).zip(orbits) {
        nodes[orbit.index() % events].extend(node_messages(config, orbit, number)?);
    }
    let completions = iter::repeat_n(None, GLOBAL_DEFINITIONS).chain(nodes.into_iter().map(
        |commands: Vec<Command>| {
            (!commands.is_empty()).then_some(Completion::Bundle {
                time: Timetag::IMMEDIATELY,
                commands,
            })
        },
    ));
    definitions
        .iter()
        .zip(completions)
        .map(|(definition, completion)| {
            let data = definition
                .as_bytes()
                .map_err(|error| ConfigError::Definition(BuildError::Limit(error)))?;
            Ok(Command::DRecv(DRecv { data, completion }))
        })
        .collect()
}

/// The node messages of `orbit`, the orbit `number` of `config`:
/// `/g_new group 0 root`, then for each global synth
/// `/s_new name id 3 group outBus o dryBus d effectBus e orbitIndex k` and
/// `/n_run id 0`.
fn node_messages(config: &Config, orbit: &Orbit, number: u32) -> Result<Vec<Command>, ConfigError> {
    let new_node =
        |id: NodeId| NewNodeId::new(id.0).ok_or(ConfigError::FixedNodesUnder1(config.fixed_nodes));
    let controls = [
        ("outBus", orbit.out_bus()),
        ("dryBus", orbit.dry_bus()),
        ("effectBus", orbit.effect_bus()),
        ("orbitIndex", number),
    ];
    let mut commands = vec![Command::GNew(GNew {
        groups: vec![NewGroup {
            id: new_node(orbit.group())?,
            action: AddAction::Head,
            target: config.root,
        }],
    })];
    for (name, synth) in global_synths(orbit) {
        commands.push(Command::SNew(SNew {
            def: format!("dirt_{name}{}", config.channels),
            id: new_node(synth)?,
            action: AddAction::After,
            target: orbit.group(),
            controls: controls
                .iter()
                .map(|&(control, value)| ControlSetting::new(control, int(value)))
                .collect(),
        }));
        commands.push(Command::NRun(NRun {
            nodes: vec![NodeRun {
                node: synth,
                run: false,
            }],
        }));
    }
    Ok(commands)
}

/// The global synths of `orbit`, in the order of their node ids: the name of
/// the definition of each, without its channel count, and its node id.
fn global_synths(orbit: &Orbit) -> [(&'static str, NodeId); 5] {
    [
        ("monitor", orbit.monitor()),
        ("rms", orbit.rms()),
        ("leslie", orbit.leslie()),
        ("reverb", orbit.reverb()),
        ("delay", orbit.delay()),
    ]
}
