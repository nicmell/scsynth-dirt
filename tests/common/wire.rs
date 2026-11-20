//! The packets of the crate as scsynth reads them, and their comparison with
//! the golden bundles of `events.json`.

use std::collections::BTreeMap;

use scsynth_osc::{Command, Packet, RawArg, RawMessage};

use super::events::{Bundle, Message, Value};

/// The packet in `bytes`, as raw messages: each an address and its typed
/// arguments.
pub fn decode(bytes: &[u8]) -> Packet<RawMessage> {
    Packet::decode(bytes).unwrap_or_else(|error| panic!("{bytes:?}: {error}"))
}

/// The raw messages of `packet`, as its bytes decode: its bundles flattened.
pub fn raw_messages(packet: &Packet<Command>) -> Vec<RawMessage> {
    decode(&packet.encode()).into_messages()
}

/// The comparison of the golden bundles of one scenario with the bundles of
/// the crate. A golden node id pairs with the node id of the crate at its
/// first appearance, and with the same id from then on, in both directions.
pub struct Comparison {
    /// The name of the scenario.
    scenario: String,
    /// The node id of the crate for each golden node id.
    ours: BTreeMap<i64, i32>,
    /// The golden node id for each node id of the crate.
    golden: BTreeMap<i32, i64>,
}

/// What an argument of a message holds.
enum Kind {
    /// A node id.
    Node,
    /// The id of a new synth: -1, or a node id.
    NewSynth,
    /// Any other value.
    Value,
}

impl Comparison {
    /// A comparison for the scenario `scenario`, with no node ids paired.
    pub fn new(scenario: &str) -> Self {
        Self {
            scenario: scenario.to_owned(),
            ours: BTreeMap::new(),
            golden: BTreeMap::new(),
        }
    }

    /// Asserts that the messages of the golden bundle `golden` equal `ours`,
    /// the raw messages of one bundle of the crate: the same addresses and the
    /// same counts of arguments, each argument equal. `bundle` names the
    /// bundle in a failure. The time of the golden bundle is not compared.
    pub fn assert_bundle(&mut self, bundle: &str, golden: &Bundle, ours: &[RawMessage]) {
        assert_eq!(
            golden.messages.len(),
            ours.len(),
            "{}: {bundle}: the counts of the messages differ\n  golden: {:?}\n  crate:  {ours:?}",
            self.scenario,
            golden.messages,
        );
        for (index, (golden, ours)) in golden.messages.iter().zip(ours).enumerate() {
            if let Err(difference) = self.message(golden, ours) {
                panic!(
                    "{}: {bundle}: message {index}: {difference}\n  golden: {golden:?}\n  crate:  {ours:?}",
                    self.scenario
                );
            }
        }
    }

    /// Asserts that the messages of the golden bundle `golden` equal `ours`,
    /// as [`Comparison::assert_bundle`] does, but the messages from index
    /// `first` on as sets. Each of them starts with a node id that is paired
    /// already, and they compare in the order of the node ids of the crate.
    pub fn assert_bundle_unordered(
        &mut self,
        bundle: &str,
        golden: &Bundle,
        ours: &[RawMessage],
        first: usize,
    ) {
        let paired = |message: &Message| match message.args.first() {
            Some(&Value::Int(node)) => *self.ours.get(&node).unwrap_or_else(|| {
                panic!(
                    "{}: {bundle}: the golden node {node} is not paired: {message:?}",
                    self.scenario
                )
            }),
            _ => panic!("{}: {bundle}: {message:?} names no node", self.scenario),
        };
        let mut golden = golden.clone();
        if let Some(rest) = golden.messages.get_mut(first..) {
            rest.sort_by_key(paired);
        }
        let mut ours = ours.to_vec();
        if let Some(rest) = ours.get_mut(first..) {
            rest.sort_by_key(|message| match message.args.first() {
                Some(&RawArg::Int(node)) => node,
                _ => panic!("{}: {bundle}: {message:?} names no node", self.scenario),
            });
        }
        self.assert_bundle(bundle, &golden, &ours);
    }

    /// The difference of the golden message `golden` and the raw message
    /// `ours` of the crate, if any.
    fn message(&mut self, golden: &Message, ours: &RawMessage) -> Result<(), String> {
        if golden.address != ours.address {
            return Err(format!(
                "the address {} against {}",
                golden.address, ours.address
            ));
        }
        if golden.args.len() != ours.args.len() {
            return Err(format!(
                "{} arguments against {}",
                golden.args.len(),
                ours.args.len()
            ));
        }
        for (index, (golden_arg, our_arg)) in golden.args.iter().zip(&ours.args).enumerate() {
            let result = match kind(&ours.address, index) {
                Kind::Node => self.node(golden_arg, our_arg),
                Kind::NewSynth => match (golden_arg, our_arg) {
                    (Value::Int(-1), RawArg::Int(-1)) => Ok(()),
                    (Value::Int(-1), _) | (_, RawArg::Int(-1)) => {
                        Err("-1 on one side alone".to_owned())
                    }
                    _ => self.node(golden_arg, our_arg),
                },
                Kind::Value if same(golden_arg, our_arg) => Ok(()),
                Kind::Value => Err("the values differ".to_owned()),
            };
            result.map_err(|reason| {
                format!("argument {index}, {golden_arg:?} against {our_arg:?}: {reason}")
            })?;
        }
        Ok(())
    }

    /// Pairs the golden node id `golden` with the node id `ours` of the
    /// crate, or checks the pairs that either has.
    fn node(&mut self, golden: &Value, ours: &RawArg) -> Result<(), String> {
        let (&Value::Int(golden), &RawArg::Int(ours)) = (golden, ours) else {
            return Err("a node id that is no int".to_owned());
        };
        match (self.ours.get(&golden), self.golden.get(&ours)) {
            (None, None) => {
                self.ours.insert(golden, ours);
                self.golden.insert(ours, golden);
                Ok(())
            }
            (Some(&paired), _) if paired == ours => Ok(()),
            (Some(&paired), _) => Err(format!(
                "the golden node {golden} pairs with the node {paired} of the crate"
            )),
            (None, Some(&paired)) => Err(format!(
                "the node {ours} of the crate pairs with the golden node {paired}"
            )),
        }
    }
}

/// What the argument at `index` of a message to `address` holds:
/// `/g_new id action target`, `/s_new def id action target …`,
/// `/n_set id …`, `/n_run id flag` and `/n_free id …` hold node ids.
fn kind(address: &str, index: usize) -> Kind {
    match address {
        "/g_new" if index % 3 != 1 => Kind::Node,
        "/s_new" if index == 1 => Kind::NewSynth,
        "/s_new" if index == 3 => Kind::Node,
        "/n_set" if index == 0 => Kind::Node,
        "/n_run" if index % 2 == 0 => Kind::Node,
        "/n_free" => Kind::Node,
        _ => Kind::Value,
    }
}

/// Whether the golden argument `golden` equals the argument `ours` of the
/// crate: a string a string, a number a number as an f32, null the int 0,
/// and a list an array, element by element.
fn same(golden: &Value, ours: &RawArg) -> bool {
    match (golden, ours) {
        (Value::Text(golden), RawArg::String(ours)) => golden == ours,
        (Value::Null, RawArg::Int(0)) => true,
        (Value::List(golden), RawArg::Array(ours)) => {
            golden.len() == ours.len()
                && golden
                    .iter()
                    .zip(ours)
                    .all(|(golden, ours)| same(golden, ours))
        }
        (golden, ours) => match (golden_number(golden), our_number(ours)) {
            (Some(golden), Some(ours)) => golden.to_bits() == ours.to_bits(),
            _ => false,
        },
    }
}

/// The golden Integer or Float `value` as an f32, the type of a control
/// value on the wire.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "the test compares numbers as f32"
)]
fn golden_number(value: &Value) -> Option<f32> {
    match *value {
        Value::Int(number) => Some(number as f32),
        Value::Float(number) => Some(number as f32),
        _ => None,
    }
}

/// The int, float or double `arg` of the crate as an f32, the type of a
/// control value on the wire.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "the test compares numbers as f32"
)]
fn our_number(arg: &RawArg) -> Option<f32> {
    match *arg {
        RawArg::Int(number) => Some(number as f32),
        RawArg::Float(number) => Some(number),
        RawArg::Double(number) => Some(number as f32),
        _ => None,
    }
}
