# scsynth-dirt

Dirt events on scsynth, the synthesis server of SuperCollider, for Rust.
The interface of the crate follows libdirt, the library of
[uzu/dirt](https://codeberg.org/uzu/dirt), the classic Dirt sampler. Its
sound is the sound of
[SuperDirt](https://codeberg.org/musikinformatik/SuperDirt), the Dirt
sampler in SuperCollider. The crate builds the synth definitions of
SuperDirt with scsynth-synthdef, and returns for each sound the messages
that SuperDirt sends.

The crate does no I/O. A call returns packets of scsynth-osc, and the host
sends them. The host passes the replies of scsynth back to the crate.

One instance lives inside a group that the host makes, on node ids and
buses that the host gives. Many instances share one library of samples.

```rust
use scsynth_dirt::scsynth_osc::command::top_level::Notify;
use scsynth_dirt::scsynth_osc::reply::buffer::{BInfo, BufferInfo};
use scsynth_dirt::scsynth_osc::{Command, NodeId, Packet, Reply, Timetag};
use scsynth_dirt::{Config, Delay, Dirt, Library, Register, Sound};

// 12 orbits of 2 channels in the group 1 of the host, with `when` in Unix time.
let mut dirt = Dirt::new(Config {
    channels: 2,
    orbits: 12,
    root: NodeId(1),
    fixed_nodes: 100,
    sound_nodes: 1000..2000,
    first_bus: 16,
    out_bus: 0,
    sample_rate: 48_000.0,
    delay: Delay::Comb,
    register: Register::CounterTenor,
    epoch: 2_208_988_800.0,
})?;
let mut library = Library::new(0..1024)?;

// What the host sends to scsynth, one datagram per packet, in order and with no wait:
// its registration for the node notifications, the 25 definitions with the nodes of
// the orbits, and the load of a sample.
let mut sent: Vec<Vec<u8>> = Vec::new();
sent.push(Command::Notify(Notify { on: true, client_id: None }).encode());
for command in dirt.start() {
    sent.push(command.encode());
}
for command in library.add("bd", ["/samples/bd/0.wav"])? {
    sent.push(command.encode());
}

// scsynth answers the load with /b_info. The host passes each reply of scsynth to
// the library and to the instance.
let buffer = library.sample("bd", 0.0).ok_or("no sample")?.buffer();
let info = BufferInfo { buffer, frames: 22_050, channels: 1, sample_rate: 44_100.0 };
let datagram = Reply::BInfo(BInfo { buffers: vec![info] }).encode();
let reply = Reply::decode(&datagram)?;
library.reply(&reply);
dirt.reply(&reply);

// A sound of the sample at twice its speed, at a Unix time.
let sound = Sound::new().at(1_700_000_000.25).s("bd").speed(2.0);
let bundles = dirt.play(&library, &sound)?;
assert_eq!(bundles[0].time, Timetag { seconds: 3_908_988_800, fraction: 1 << 30 });
let synths: Vec<String> = Packet::Bundle(bundles[0].clone())
    .into_messages()
    .into_iter()
    .filter_map(|command| match command {
        Command::SNew(new) => Some(new.def),
        _ => None,
    })
    .collect();
assert_eq!(synths, ["dirt_sample_1_2", "dirt_gate2"]);
for bundle in bundles {
    sent.push(Packet::Bundle(bundle).encode());
}
assert_eq!(sent.len(), 28);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The crate follows SuperDirt at commit `a3046d1`, on SuperCollider 3.14.1.
Its definitions have the bytes that sclang writes for SuperDirt. For each
sound, it returns the messages that SuperDirt sends, except for the
differences that this file lists. It builds for `wasm32-unknown-unknown`.

## The instance

`Dirt` is one instance of SuperDirt: the orbits of a `Config`, and the 25
synth definitions of its channel count. `Dirt::start`, `Dirt::stop` and
`Dirt::play` return commands for scsynth, and the host sends them in order.
The host passes the replies of scsynth to `Dirt::reply`. The crate
re-exports `scsynth_osc`, the types of its packets, and `scsynth_synthdef`,
the compiler of its definitions.

### The configuration

`Config` holds the structure of an instance, and nothing that a sound can
change. It is a struct of public fields, with no default.

| Field | Type | Meaning |
| --- | --- | --- |
| `channels` | `u32` | The channel count of the orbits, 1 or more |
| `orbits` | `u32` | The number of orbits, 1 or more. SuperDirt starts 12. |
| `root` | `NodeId` | The group of the host. The instance makes its nodes inside it, and never frees it. |
| `fixed_nodes` | `i32` | The first of the `6 * orbits` node ids of the orbits |
| `sound_nodes` | `Range<i32>` | The node ids of the groups of the sounds |
| `first_bus` | `u32` | The first of the private audio buses of the orbits |
| `out_bus` | `u32` | The first of the `channels` output buses |
| `sample_rate` | `f64` | The sample rate of the server, in hertz |
| `delay` | `Delay` | The delay line of the orbits (see [The definitions](#the-definitions)) |
| `register` | `Register` | The voice register of the vowels |
| `epoch` | `f64` | The seconds from the OSC epoch, 1 January 1900, to time 0 of `when` |

- `epoch` is 2,208,988,800 for `when` in Unix time. It is 0 for `when` in
  the seconds of a time tag, as in a score of `scsynth -N`.
- `register` is one of the five voices of the Vowel quark:
  `Register::Soprano`, `Alto`, `Tenor`, `Bass` or `CounterTenor`. As in
  SuperDirt, `CounterTenor` is the default.
- The host keeps the node ids and the buses apart from those of other
  clients. It also keeps the private buses past the input and output buses
  of scsynth.

### The nodes and the buses of an orbit

Orbit `k` takes the 6 node ids from `fixed_nodes + 6 * k`: its group, then
its monitor, its rms, its leslie, its reverb and its delay. The group sits
at the head of `root`. The five global synths sit after it, in the order
delay, reverb, leslie, rms and monitor. A sound plays in a group of its
own, at the tail of the group of its orbit, with an id of `sound_nodes`.

An orbit has three private buses, one after the other, each of `channels`
channels:

- The synth bus: the player and the effects of a sound write it, and the
  gate of the sound reads it.
- The dry bus: the gate of a sound writes it, and the global synths read
  it.
- The effect bus: the delay, the reverb and the leslie write it.

The monitor adds the dry bus and the effect bus, and writes the sum to the
output buses from `out_bus`. Every orbit writes the same output buses.

Orbit `k` takes `3 * channels` buses after those of orbit `k - 1`, from
`first_bus`. For 1 channel, it takes 4: the reverb of 1 channel writes 2
channels on its effect bus, and no synth reads the second (see
[Traps](#traps)). With `first_bus` 16 and 2 channels, orbit 0 has the buses
16, 18 and 20, and orbit 1 has the buses 22, 24 and 26.

### What `Dirt::new` refuses

`Dirt::new(config)` checks the configuration. Then it builds the 25
definitions of `synthdefs::core` once, and keeps them, so `start` cannot
fail. It returns the `ConfigError` of the first fault, in this order:

| Error | Fault |
| --- | --- |
| `NoChannels` | `channels` is 0. |
| `NoOrbits` | `orbits` is 0. |
| `FixedNodesUnder1(first)` | `fixed_nodes` is under 1. |
| `EmptySoundNodes(range)` | `sound_nodes` is empty. |
| `SoundNodesUnder1(range)` | `sound_nodes` starts under 1. |
| `RootUnder0(root)` | `root` is under 0. |
| `FixedNodesPastI32` | The node ids of the orbits pass the largest i32. |
| `PrivateBusesPastI32` | The private buses pass the largest i32. |
| `OutBusesPastI32` | The output buses pass the largest i32. |
| `RootInFixedNodes(root)` | `root` is a node id of the orbits. |
| `RootInSoundNodes(root)` | `root` is in `sound_nodes`. |
| `FixedNodeInSoundNodes(id)` | A node id of the orbits is in `sound_nodes`. The error gives the lowest one. |
| `OutBusInPrivateBuses(bus)` | An output bus is a private bus. The error gives the lowest one. |
| `BadSampleRate(rate)` | `sample_rate` is not a finite number above 0. |
| `BadEpoch(epoch)` | `epoch` is not finite. |
| `Definition(error)` | A definition does not build. The error is the `BuildError` of scsynth-synthdef. |

The build fails for more than 65,535 channels, with
`BuildError::BadCount`. It fails for `Delay::Switch` without the feature
`sc3-plugins`, with `BuildError::UnknownClass`. A definition that breaks a
limit of scsynth gives `BuildError::Limit`. Each `ConfigError` says what is
wrong in its `Display`, and `Definition` gives its `BuildError` as its
source.

### Start and stop

`Dirt::start` returns the commands that load the definitions and make the
nodes of the instance: one `/d_recv` per definition, in the order of
`Dirt::definitions`. The host sends them in order, with no wait.

1. The 5 global definitions come first: monitor, rms, delay, reverb and
   leslie. They carry no completion message.
2. Then come the 20 definitions of a sound. As its completion message, the
   definition `5 + j` carries one bundle with the immediate time tag. It
   holds the node messages of each orbit `k` with `k mod 20 = j`, in the
   order of the orbits. A definition without such an orbit carries no
   completion message.
3. The node messages of an orbit are `/g_new group 0 root`, then two
   messages per global synth. The first is
   `/s_new name id 3 group outBus o dryBus d effectBus e orbitIndex k`, and
   the second is `/n_run id 0`. The synths come in the order monitor, rms,
   leslie, reverb and delay, each after the group. So they sit in the order
   delay, reverb, leslie, rms and monitor, and they start paused.

The commands need no wait, because scsynth runs its asynchronous commands
in order. It runs the completion message of a `/d_recv` once the
definitions of that `/d_recv` are in place. So the global definitions are
in place before the first node bundle runs. The node bundles spread over 20
definitions, so that each message stays small (see [Limits](#limits)).

A sound needs the definitions of a sound too. To know that all of them are
in place, the host sends `/sync` after the commands of `start`, and waits
for `/synced`.

`start` also forgets what `play` keeps: the last parameters of the global
synths, and the live sounds. The node ids of the sounds start over from the
first of `sound_nodes`.

`Dirt::stop` returns one bundle per orbit, in order, each with the
immediate time tag. It holds `/error -1`, one `/n_free` of the group of the
orbit and of its five global synths, then `/error -2`. scsynth frees the
sounds of the orbit with its group. The `/error` messages turn off the
posts of failures inside the bundle, such as that of a node that is gone.
`stop` forgets what `start` forgets, and leaves the definitions and `root`
in place.

`Dirt::definitions` gives the 25 definitions, for a host that loads them in
another way. `Dirt::config` gives the configuration.

### The orbits

`Dirt::orbits` gives the orbits in order. `Dirt::orbit(index)` gives one,
and `Dirt::orbit_mut(index)` gives one to change. Past the count, both give
nothing. An `Orbit` gives its `index`, its `group`, and its global synths:
its `monitor`, its `rms`, its `leslie`, its `reverb` and its `delay`. It
also gives its `synth_bus`, its `dry_bus`, its `effect_bus` and its
`out_bus`.

Each orbit starts with the defaults of `DirtOrbit.makeDefaultParentEvent`
of SuperDirt:

| Name | Default | Name | Default | Name | Default |
| --- | --- | --- | --- | --- | --- |
| `cps` | 1 | `offset` | 0 | `begin` | 0 |
| `end` | 1 | `speed` | 1 | `pan` | 0.5 |
| `gain` | 1 | `overgain` | 0 | `cut` | 0 |
| `unit` | `r` | `octave` | 5 | `dur` | 1 |
| `lag` | 0 | `loop` | 1 | `dry` | 0 |
| `lock` | 0 | `amp` | 0.4 | `fadeTime` | 0.001 |

`Orbit::get(name)` reads a default. `Orbit::set(name, value)` replaces a
default or adds one, as `DirtOrbit.set` does, and returns the orbit for the
next call. `n` has no default, and counts as 0 for the index of a sample.

## The sounds

A `Sound` is an optional time, `when`, in seconds, and its parameters by
name. A `Value` is a number, `Value::Number` of an f64, or a text,
`Value::Text`. A parameter that is not set stays out of the sound. As in
SuperDirt, a parameter that the sound holds is on: its effect plays, at any
value.

- `Sound::new()` and `Sound::default()` are empty.
- `Sound::set(name, value)` sets a parameter in place, and
  `Sound::with(name, value)` sets it in a chain. A second value of a name
  replaces the first. A `Value` comes from an `f64`, an `f32`, an `i32`, a
  `&str` or a `String`.
- `Sound::get(name)` reads a parameter, and `Sound::remove(name)` takes it
  out. `Sound::parameters()` gives the pairs in the order of the bytes of
  their names.
- `Sound::at(when)` sets the time in a chain, and `Sound::set_when` sets it
  in place. `Sound::when()` reads it.
- `Value::as_number` and `Value::as_text` read a value.

### The fields of libdirt

Each of the 32 fields of `dirt_sound_t` in libdirt has a setter in a chain,
by its name, with the type that libdirt gives it. `loop` is a keyword of
Rust, so its setter is `r#loop`. A setter stores its value as it is, under
the name of its field. No default of libdirt applies.

| Type in libdirt | The setter takes | Fields |
| --- | --- | --- |
| float | `f64` | `cps`, `offset`, `begin`, `end`, `speed`, `pan`, `velocity`, `cutoff`, `resonance`, `accelerate`, `shape`, `gain`, `delay` |
| float | `f64` | `delaytime`, `delayfeedback`, `crush`, `hcutoff`, `hresonance`, `bandf`, `bandq`, `loop`, `n`, `attack`, `hold`, `release` |
| int | `i32` | `kriole`, `cut`, `coarse`, `orbit` |
| string | `impl Into<String>` | `s`, `vowel`, `unit` |

`play` reads every field but `velocity` and `kriole`, as the core of
SuperDirt does. The crate takes the names and the types of libdirt, and no
C ABI.

### The arguments of `/dirt/play`

Tidal and Strudel send each sound to SuperDirt as a `/dirt/play` message.
Its arguments are pairs of a name and a value, and `Sound::from_args` reads
them.

```rust
use scsynth_dirt::scsynth_osc::{RawArg, RawMessage};
use scsynth_dirt::{Sound, Value};

// A /dirt/play message as the host receives it, inside a bundle.
let text = |text: &str| RawArg::String(text.to_owned());
let message = RawMessage {
    address: "/dirt/play".to_owned(),
    args: vec![text("s"), text("bd"), text("n"), RawArg::Int(3), text("cps"), RawArg::Float(0.5625)],
};
let received = RawMessage::decode(&message.encode())?;
let mut sound = Sound::from_args(&received.args)?;
assert_eq!(sound.get("n"), Some(&Value::Number(3.0)));

// The time tag of the bundle gives the time, here in the seconds of a time tag.
sound.set_when(Some(3_908_988_800.25));
# Ok::<(), Box<dyn std::error::Error>>(())
```

- A name is a string. An int, a float, a double or a long gives a number,
  and a string gives a text. True gives 1, and false gives 0.
- A later pair replaces an earlier one of the same name.
- The sound has no `when`: the time of the message is the time tag of its
  bundle, and the host sets it with `Sound::set_when`.
- `from_args` refuses an odd count of arguments with
  `SoundError::OddCount`. It refuses a name that is no string with
  `SoundError::NotAName` and its index. It refuses a value of another type
  with `SoundError::WrongType` and its name.

### The parameters

`Dirt::play` reads a parameter from the sound first, then from the
defaults of its orbit. The sample sits between them, and gives `note` 0.
`s`, `sound`, `bank` and `unit` are texts, and every other parameter that
`play` reads is a number. A `vowel` that is no text counts as no vowel.

A value of the wrong kind drops the sound, with `Dropped::WrongKind`.
`play` reads a parameter only where a rule needs it. So a text in a
parameter that the sound does not read drops nothing.

#### The name and the sample

| Name | Meaning |
| --- | --- |
| `s` | The name of the sound |
| `sound` | Without `s`, the name, then a colon and the index of the sample, as `bd:3` |
| `n` | The index of the sample, without an index in `sound` |
| `bank` | A prefix of the name: with `bank`, the name is `bank_name` |

- Without `s`, the text of `sound` splits at its colons. Its first part is
  the name. Its second part gives the index, as `asFloat` of sclang reads
  it: the decimal number at its start, or 0.
- An empty name is silence, and drops with `Dropped::Silence`. A sound
  without `s` and `sound` drops with `Dropped::NoName`.
- The sample is `Library::sample(name, n)`, with 0 for no index (see
  [The library](#the-library)).

#### The orbit

`orbit` is the index of the orbit of the sound. It is truncated toward 0,
then wrapped to the count of the orbits. It comes from the sound alone, and
is 0 without it.

#### The span

| Name | Default | Meaning |
| --- | --- | --- |
| `speed` | 1 | The rate of the playback. A negative speed plays backwards. |
| `begin` | 0 | Where the playback starts in the sample, from 0 to 1 |
| `end` | 1 | Where the playback ends in the sample, from 0 to 1 |
| `accelerate` | none | The change of the speed: the speed at the end is `speed * (1 + accelerate)` |
| `unit` | `r` | The unit of `speed`: `r` or `rate` for a rate, `c` for cycles, `s` for seconds |
| `cps` | 1 | The cycles per second of the unit `c` |
| `loop` | 1 | How many times the sample plays in a row |
| `sustain` | none | The duration of the sound, in seconds |
| `legato` | none | The duration of the sound, in units of `delta` |
| `delta` | `dur` | The time from this sound to the next, in seconds |
| `dur` | 1 | The default of `delta` |
| `fadeTime` | 0.001 | The fade out, and the fade in after a `begin` other than 0 |
| `timescale` | none | A stretch of the time of the sound, with no change of its pitch |
| `timescalewin` | 1 | The size of the windows of the stretch |

The span of a sound follows `orderTimeSpan` and `calcTimeSpan` of
`DirtEvent`:

1. A negative `speed` swaps `begin` and `end`. The length is
   `|end - begin|`.
2. The unit duration is the duration of the buffer, its frames over its
   sample rate, times the frequency of note 60 over `freq` (see
   [The pitch](#the-pitch)).
3. With the unit `c`, the speed becomes `speed * cps * unitDuration`. So
   with a `speed` of 1, the whole sample takes one cycle.
4. Without `accelerate`, the end speed is the speed, and the mean speed is
   its magnitude. With it, the end speed is `speed * (1 + accelerate)`, and
   the mean speed is the mean of the magnitudes of both.
5. With the unit `r`, `rate` or `c`, the unit duration becomes
   `unitDuration * length / meanSpeed`. With the unit `s`, it becomes the
   length. Any other unit drops the sound, with `Dropped::BadUnit`.
6. The sustain is `sustain`. Without it, the sustain is `delta * legato`,
   and `delta` is `dur` without `delta`. Without `legato` too, the unit
   duration becomes `unitDuration * |loop|`, and the sustain takes that
   value.
7. The sustain is at most the unit duration.
8. The fade out is `fadeTime`, at most `sustain * 0.19098`. After a `begin`
   other than 0, the fade in is the fade out. Else it is 0.
9. With `timescale`, the sustain becomes `sustain * timescale`.
10. Last, the sustain loses the fade in and the fade out.

A sustain under 8 samples of `sample_rate` drops the sound, with
`Dropped::TooShort`. So does a sustain that is not a number.

#### The pitch

| Name | Default | Meaning |
| --- | --- | --- |
| `freq` | none | The frequency of the sound, in hertz |
| `midinote` | none | The MIDI note of the sound |
| `note` | 0 | The note in the octave of `octave`, from the sound alone |
| `octave` | 5 | The octave of `note` |

The frequency is `freq`. Without it, the frequency is the `midicps` of
`midinote`, or of `note + octave * 12` without `midinote`. The sample gives
`note` 0, which hides a `note` of the orbit. So a sample plays at the
frequency of note 60, unless the sound sets `note`, `midinote` or `freq`.

The player multiplies its speed by the frequency over the frequency of
note 60. So `note` 12 plays a sample an octave higher, and in half the
time. `midicps` is `440 * 2 ** ((note - 69) / 12)` in f64, as sclang
computes it.

#### The level and the place

| Name | Default | Meaning |
| --- | --- | --- |
| `amp` | 0.4 | The level of the gate |
| `gain` | 1 | The gain of the gate, at most 2 |
| `overgain` | 0 | A gain on top of `gain` |
| `pan` | 0.5 | The place of the sound, from 0 for left to 1 for right |
| `channel` | none | A shift of the place, by channels |

- The gate scales the sound by `amp * (min(gain, 2) + overgain) ** 4`.
- With `channel`, the pan gains `channel / channels`. Then `play` sends it
  as `pan * 2 - 1`.
- On 1 channel, the channels of the sample add up, and `pan` has no
  effect. On 2 channels, `Pan2` places a sample of 1 channel, and
  `Balance2` a sample of 2, at the pan folded into -1 to 1.
- On more channels, `pan` goes around the circle of the channels. On 4
  channels, a `pan` of 0.25 is channel 1.

#### The modules

A module plays an effect on the synth bus of a sound, after its player.
`play` takes the modules of `core-modules.scd` in this order, and plays the
definition of each module whose test holds:

| Module | Test | Definition | Arguments |
| --- | --- | --- | --- |
| `vowel` | `vowel` is `a`, `e`, `i`, `o` or `u` | `dirt_vowel` | `out`, `vowelFreqs`, `vowelAmps`, `vowelRqs`, `resonance` |
| `shape` | `shape` | `dirt_shape` | `shape`, `out` |
| `hpf` | `hcutoff` | `dirt_hpf` | `hcutoff`, `hresonance`, `out` |
| `bpf` | `bandf` | `dirt_bpf` | `bandqf`, `bandq`, `out` |
| `crush` | `crush` | `dirt_crush` | `crush`, `out` |
| `coarse` | `coarse` above 1 | `dirt_coarse` | `coarse`, `out` |
| `lpf` | `cutoff` | `dirt_lpf` | `cutoff`, `resonance`, `out` |
| `pshift` | `psrate` | `dirt_pitch_shift` | `psrate`, `psdisp`, `sustain`, `out` |
| `envelope` | `attack` or `release` | `dirt_envelope` | `attack`, `hold`, `release`, `curve`, `out` |
| `grenvelo` | `tilt` | `dirt_grain_envelope` | `sustain`, `tilt`, `plat`, `curve`, `out` |
| `tremolo` | `tremolorate` | `dirt_tremolo` | `tremolorate`, `tremolodepth`, `out` |
| `phaser` | `phaserrate` | `dirt_phaser` | `phaserrate`, `phaserdepth`, `out` |

- The name of a definition ends in the channel count, as `dirt_lpf2`.
- A test of a name holds for any value of that name in the sound or in its
  orbit.
- An argument takes the value of the parameter of its name. `out` is the
  synth bus, `sustain` the sustain of the sound, and `bandqf` the value of
  `bandf`.
- `vowelFreqs`, `vowelAmps` and `vowelRqs` are arrays of the 5 formants of
  the vowel in `register`, from the Vowel quark. They hold the frequencies,
  the `dbamp` of the levels, and the reciprocals of the widths.
- An argument that the sound and its orbit lack goes as the int 0, as
  sclang writes nil. So `attack` without `release` ends the sound after the
  attack.

[The definitions](#the-definitions) says what each effect does.

#### The global synths

Each orbit runs five global synths: the delay, the reverb, the leslie, the
rms and the monitor. A sound sets the parameters of four of them, in this
order:

| Synth | Parameters |
| --- | --- |
| delay | `delaytime`, `delayfeedback`, `delaySend`, `delayAmp`, `lock`, `cps` |
| reverb | `size`, `room`, `dry` |
| leslie | `leslie`, `lrate`, `lsize` |
| monitor | `limitertype` |

- `delayAmp` is the `delay` of the sound, or 0. A sound cannot set it by
  its name.
- A synth resumes for each sound that gives one of its parameters a value.
  The monitor resumes for every sound. A resume is `/n_run id 1`, then
  `/n_set id pauseImmediately 0 dirt_resumed 1`.
- `delayAmp`, `lock`, `cps` and `dry` always hold a value. So every sound
  resumes the delay, the reverb and the monitor of its orbit. The leslie
  resumes for `leslie`, `lrate` or `lsize`.
- A global synth pauses itself after 4 seconds of silence, as `DirtPause`
  of SuperDirt does.
- After a resume, one `/n_set` names each parameter whose value differs
  from the last sound of the orbit. A parameter without a value goes as the
  int 0. So a parameter that the next sound lacks goes to 0, and not to the
  default of its definition.
- The rms is off, as in SuperDirt. It never resumes, and `rmsReplyRate` and
  `rmsPeakLag` have no effect.

What the parameters do:

- `delaytime`: the time between the echoes, in seconds, at most 4. With
  `lock` 1, it counts in cycles of `cps`.
- `delayfeedback`: the level of each echo against the echo before.
  `Delay::Comb` keeps it from 0 to 0.99.
- `delaySend`: the level of the input of `Delay::Switch`. The definition of
  `Delay::Comb` has no such control, and scsynth ignores the `/n_set`.
- `delayAmp`: the level of the echoes.
- `room`: the level of the input of the reverb.
- `size`: the depth of the loop of the reverb, from 0.01 to 0.98 for a
  `size` from 0 to 1.
- `dry`: the reverb writes its sound at the level `1 - dry`.
- `leslie`: the level of the rotary speaker. It leaves `1 - leslie` of the
  dry sound in place.
- `lrate`: the rate of the turn of the speaker, in hertz.
- `lsize`: the size of its cabinet, in meters.
- `limitertype`: 0 for the sum of the buses, 1 for a `Limiter`, and 2 for
  twice the soft clip of half the sum. 1 is the default of `dirt_monitor`.
  After a sound with `limitertype` 2, the next sound without it sets 0, the
  sum.

#### The cut

| Name | Default | Meaning |
| --- | --- | --- |
| `cut` | 0 | The cut group of the sound |
| `cutAll` | none | A number other than 0 widens the cut to every orbit and every time |

With a `cut` that is not 0, a sound cuts the live sounds that pass these
tests:

- The cut group of the live sound is `|cut|`.
- The live sound is on the orbit of the sound, and its `when` differs. A
  sound without `when` differs from every sound. With `cutAll`, this test is
  off.
- For a negative `cut`, the live sound plays the sample of the sound.

The cut sends `/n_set group cut_gate 0` to the group of each such sound.
Its gate then fades it out over its `fadeTime`, and frees its group.

#### The time

| Name | Default | Meaning |
| --- | --- | --- |
| `lag` | 0 | A delay of the sound, in seconds |
| `offset` | 0 | A delay of the sound, in seconds per unit of the speed |

The time of a sound is `when + lag + offset * |speed|`, with the speed
after the unit `c`. The time tag adds `epoch` to it. Without `when`, the
sound plays at once, and `lag` and `offset` have no effect.

#### What `play` does not read

- `velocity` and `kriole`: the core of SuperDirt does not read them.
- `metatune`: the crate reads no `smpl` chunk of a WAV file.
- `rmsReplyRate` and `rmsPeakLag`: the rms is off.
- `out`, `buffer`, `instrument`, `unitDuration`, `latency` and the other
  values that SuperDirt keeps in its event: the instance gives them.
- Any other name.

## Play

`Dirt::play(library, sound)` is the port of `DirtEvent.play` of SuperDirt
for a sample sound. It returns the bundles of the sound, or the reason of a
drop. The host sends each bundle as one packet, in order.

1. With a `cut` that is not 0, a first bundle holds `/error -1`. Then it
   holds `/n_set group cut_gate 0` for each live sound that the cut
   reaches, in the order of their starts.
2. The bundle of the sound holds, in order:
   - For each global synth that resumes, its resume, then its `/n_set` of
     the parameters that changed. The synths come in the order delay,
     reverb, leslie and monitor.
   - `/g_new group 1 orbitGroup`: the group of the sound, at the tail of
     the group of its orbit.
   - The player: `/s_new name -1 1 group`, then `bufnum`, `sustain`,
     `speed`, `freq`, `endSpeed`, `begin`, `end`, `pan` and `out`. The name
     is `dirt_sample_<sample channels>_<channels>`, or
     `dirt_sample_long_…` for a buffer over 16,777,216 frames. With
     `timescale`, it is `dirt_stretchsample_…`, with `timescale` and
     `timescalewin` at the end.
   - One `/s_new name -1 1 group` for each module whose test holds.
   - The gate: `/s_new dirt_gate<channels> -1 1 group`, then `in`, `out`,
     `amp`, `gain`, `overgain`, `cut`, `sustain`, `fadeInTime` and
     `fadeTime`. `in` is the synth bus, `out` the dry bus, and `cut` is
     `|cut|`.

In the messages, a node id, a bus and a buffer number are ints, and a
parameter of a sound is a float. Both bundles have the time tag of the
sound: its time plus `epoch`. The time tag holds the whole seconds modulo
2^32, and the fraction in units of 2^-32, rounded down. Without `when`, it
is the immediate time tag.

```rust
use scsynth_dirt::scsynth_osc::reply::buffer::{BInfo, BufferInfo};
use scsynth_dirt::scsynth_osc::reply::node_notification::{NodeKind, NodeState};
use scsynth_dirt::scsynth_osc::{Bundle, Command, NodeId, Packet, Reply};
use scsynth_dirt::{Config, Delay, Dirt, Library, Register, Sound};

/// The addresses of the messages of `bundle`.
fn addresses(bundle: &Bundle<Command>) -> Vec<String> {
    let messages = Packet::Bundle(bundle.clone()).into_messages();
    messages.iter().map(|command| command.to_raw().address).collect()
}

# let mut dirt = Dirt::new(Config {
#     channels: 2,
#     orbits: 2,
#     root: NodeId(1),
#     fixed_nodes: 100,
#     sound_nodes: 1000..2000,
#     first_bus: 16,
#     out_bus: 0,
#     sample_rate: 48_000.0,
#     delay: Delay::Comb,
#     register: Register::CounterTenor,
#     epoch: 0.0,
# })?;
# let mut library = Library::new(0..1024)?;
# let _ = library.add("bd", ["/samples/bd/0.wav"])?;
# let buffer = library.sample("bd", 0.0).ok_or("no sample")?.buffer();
# let info = BufferInfo { buffer, frames: 22_050, channels: 1, sample_rate: 44_100.0 };
# library.reply(&Reply::BInfo(BInfo { buffers: vec![info] }));
// `bd` lasts half a second. The first sound of an orbit resumes its delay, its
// reverb and its monitor, and makes its group 1000.
let first = dirt.play(&library, &Sound::new().at(1.0).s("bd").cut(1))?;
assert_eq!(addresses(&first[0]), ["/error"]);
assert_eq!(
    addresses(&first[1]),
    [
        "/n_run", "/n_set", "/n_set", "/n_run", "/n_set", "/n_set", "/n_run", "/n_set",
        "/g_new", "/s_new", "/s_new",
    ],
);

// The cut of the second sound reaches the first one, which still plays.
let second = dirt.play(&library, &Sound::new().at(1.1).s("bd").cut(1))?;
assert_eq!(addresses(&second[0]), ["/error", "/n_set"]);

// The first sound ends, and the /n_end of its group takes it out of the live sounds.
// So the cut of the third sound reaches the second one alone.
let group = NodeState {
    node: NodeId(1000),
    parent: Some(NodeId(100)),
    prev: None,
    next: None,
    kind: NodeKind::Group { head: None, tail: None },
};
dirt.reply(&Reply::NEnd(group));
let third = dirt.play(&library, &Sound::new().at(1.2).s("bd").cut(1))?;
assert_eq!(addresses(&third[0]), ["/error", "/n_set"]);
# Ok::<(), Box<dyn std::error::Error>>(())
```

### The live sounds

The instance keeps a list of the live sounds, for the cuts and for the ids
of their groups. An entry holds the group of the sound, its orbit, its cut
group, its buffer, its `when` and its end. The end is the time of the
sound plus its fade in, its sustain and its fade out. A sound without
`when` has no end.

- `Dirt::reply` takes a reply of scsynth. The `/n_end` of the group of a
  live sound takes the sound out of the list, and every other reply does
  nothing.
- A sound also leaves the list once a later sound starts after its end.
- The group of a new sound takes the next id of `sound_nodes` that no live
  sound holds. The ids start over at the end of the range.
- With every id of `sound_nodes` live, the oldest live sound leaves the
  list, and its id serves again. The cut of the new sound runs first, so it
  still reaches that sound.

### Drops

`play` gives the `Dropped` reason of a sound that does not play. A sound
that drops changes nothing in the instance.

| Reason | Cause |
| --- | --- |
| `Silence` | The name is empty. |
| `NoName` | The sound has neither `s` nor `sound`. |
| `UnknownSound(name)` | The library has no samples of the name. |
| `NotLoaded(name)` | The sample has no `/b_info` yet, or scsynth failed to read its file. |
| `TooManyChannels(channels)` | The sample has more than 2 channels. |
| `BadUnit(unit)` | `unit` is none of `r`, `rate`, `c` and `s`. |
| `TooShort(sustain)` | The sustain is under 8 samples of `sample_rate`, or not a number. |
| `WrongKind(name)` | The parameter of the name holds a value of the wrong kind. |
| `BadTime(seconds)` | The time of the sound plus `epoch` is not finite, or under 0. |

Each reason says what is wrong in its `Display`.

## The library

A `Library` maps a name to its samples, in order. A sample is a sound file
at a path that scsynth reads into a buffer. `Library::new(buffers)` takes
the range of buffer numbers that the host gives. It refuses an empty range
with `LibraryError::EmptyRange`, and a range that starts under 0 with
`LibraryError::NegativeStart`. The host keeps the range under the buffer
count of scsynth, its `-b` (see the README of scsynth-osc, Traps).

- `Library::add(name, paths)` replaces the samples of a name with one
  sample per path. It returns the `/b_free` of each buffer that it
  replaced, in order. Then come one `/b_allocRead buffer path 0 -1` per
  path, each with `/b_query buffer` as its completion message.
- `Library::append(name, paths)` adds samples after those of a name, with
  the same `/b_allocRead` messages. A new name starts with no samples.
- A new sample takes the lowest free number of the range. A number that
  the call frees is free for it.
- If the range has too few free numbers, `add` and `append` change
  nothing. They return `LibraryError::TooFewBuffers` with the count that
  lacks.
- `Library::remove(name)` takes the name out, and returns the `/b_free` of
  each of its buffers, in order. For a name that the library lacks, it
  returns no command.
- `Library::samples(name)` gives the samples of a name, and
  `Library::names()` gives the names in the order of their bytes.
- `Library::sample(name, n)` gives the sample at index `n`, as
  `DirtSoundLibrary.getEvent` of SuperDirt picks it. The index is truncated
  toward 0, then wrapped to the count. A negative index counts from the
  end, and a NaN counts as 0. The library gives nothing for a name without
  samples.

`Library::reply` takes a reply of scsynth:

- Each entry of a `/b_info` for the buffer of a sample gives the sample
  its frames, its channels and its sample rate. The sample is then loaded.
- A `/b_info` of a buffer without data fails its sample: under 1 frame,
  under 1 channel, or a sample rate that is not a finite number above 0.
- A `/fail` of `/b_allocRead` that names the buffer of a sample fails the
  sample: scsynth failed to read its file.
- The last reply for a buffer wins, and every other reply does nothing.

A `Sample` gives its `buffer` and its `path`. A new sample waits for its
replies, then it is loaded or failed. `Sample::info` gives the `SampleInfo`
of a loaded sample: its `frames`, its `channels` and its `sample_rate`.
`Sample::failed` tells a failed sample. Only a loaded sample plays.

```rust
use scsynth_dirt::Library;

let mut library = Library::new(0..1024)?;
let commands = library.add("bd", ["bd/0.wav", "bd/1.wav", "bd/2.wav"])?;
assert_eq!(commands.len(), 3);

// An index wraps to the count, and a negative one counts from the end.
assert_eq!(library.sample("bd", 4.0).map(|sample| sample.path()), Some("bd/1.wav"));
assert_eq!(library.sample("bd", -1.0).map(|sample| sample.path()), Some("bd/2.wav"));

// No sample is loaded before its /b_info.
let samples = library.samples("bd").unwrap_or_default();
assert!(samples.iter().all(|sample| sample.info().is_none() && !sample.failed()));
# Ok::<(), Box<dyn std::error::Error>>(())
```

### A folder of samples

With the feature `fs`, `fs::list(root)` lists a folder as
`DirtSoundLibrary.loadSoundFiles` of SuperDirt does. It gives each name
with the paths of its samples, as `Library::add` takes them.

- Each folder inside `root` is a name. Its samples are its files with the
  extension `wav`, `aif`, `aiff` or `aifc`, without regard to case.
- A folder inside a name stays out, and so do a name without samples and a
  file of `root`.
- A name or a file that starts with a dot stays out, as with the glob of
  sclang. A link counts as what it points to, and a link that points
  nowhere counts as a file.
- The names come in the order of their bytes, and the samples of a name in
  the order of the bytes of their paths, as sclang sorts them.
- A path is `root` joined with the name and the file.
- An error of the file system comes back as it is, such as `NotFound` for a
  missing `root`. A name or a path that is not UTF-8 gives `InvalidData`.

```rust
# #[cfg(feature = "fs")]
# {
use scsynth_dirt::{Library, fs};

// A folder of samples: a folder per name, and a sound file per sample.
let root = std::env::temp_dir().join(format!("scsynth-dirt-readme-{}", std::process::id()));
std::fs::create_dir_all(root.join("bd"))?;
for file in ["0.wav", "1.WAV", "notes.txt"] {
    std::fs::write(root.join("bd").join(file), b"")?;
}

let mut library = Library::new(0..1024)?;
let mut commands = Vec::new();
for (name, paths) in fs::list(&root)? {
    commands.extend(library.add(&name, paths)?);
}
assert_eq!(library.names().collect::<Vec<_>>(), ["bd"]);
assert_eq!(commands.len(), 2);
# std::fs::remove_dir_all(&root)?;
# }
# Ok::<(), Box<dyn std::error::Error>>(())
```

## The definitions

`synthdefs::core(channels, delay)` returns the 25 definitions of SuperDirt
for a channel count, in the order in which SuperDirt loads them. The global
synths come first, then the definitions of a sound, in the order of
`core-synths.scd`:

| Family | Definitions, in order |
| --- | --- |
| Global synths | `dirt_monitor`, `dirt_rms`, `dirt_delay`, `dirt_reverb`, `dirt_leslie` |
| Sample players | `dirt_sample`, `dirt_sample_long` and `dirt_stretchsample`, each for 1 then 2 sample channels |
| Routing | `dirt_gate`, `dirt_out` |
| Effects | `dirt_vowel`, `dirt_crush`, `dirt_coarse`, `dirt_hpf`, `dirt_bpf`, `dirt_shape`, `dirt_lpf`, `dirt_envelope`, `dirt_grain_envelope`, `dirt_pitch_shift`, `dirt_tremolo`, `dirt_phaser` |

A name ends in its channel counts, as in SuperDirt: `dirt_gate2`, and
`dirt_sample_1_2` for the player of 1 sample channel on 2 channels. Each
definition has a public function of `synthdefs`, with the name of SuperDirt
less its channel counts. It takes the counts as `u32`, and returns the
`SynthDef` of scsynth-synthdef, or its `BuildError`:

| Function | The definition |
| --- | --- |
| `dirt_monitor(channels)` | The output of an orbit: the dry bus and the effect bus, through `limitertype`, to the output buses |
| `dirt_rms(channels)` | The meter of an orbit: `/rms` replies of the peak and the power of each channel |
| `dirt_delay(channels, delay)` | The delay of an orbit, on the delay line of `delay` |
| `dirt_reverb(channels)` | The reverb of an orbit |
| `dirt_leslie(channels)` | The rotary speaker of an orbit |
| `dirt_sample(sample_channels, channels)` | The player of a buffer, with `BufRd` |
| `dirt_sample_long(sample_channels, channels)` | The player of a long buffer, with `PlayBuf` |
| `dirt_stretchsample(sample_channels, channels)` | The player that stretches the time by `timescale` |
| `dirt_gate(channels)` | The gate of a sound: its envelope, its level and its cut |
| `dirt_out(channels)` | The routing of a bus to another bus, which no module of the core plays |
| `dirt_vowel(channels)` | A vowel: five band-pass filters at the formants |
| `dirt_crush(channels)` | A bit crusher, to multiples of `0.5 ** (crush - 1)` |
| `dirt_coarse(channels)` | A reducer of the sample rate, to one value every `coarse` samples |
| `dirt_hpf(channels)` | A resonant high-pass filter at `hcutoff` |
| `dirt_bpf(channels)` | A band-pass filter at `bandqf`, with the Q and the gain `bandq` |
| `dirt_shape(channels)` | A wave shaper, which distorts more as `shape` goes from 0 to 1 |
| `dirt_lpf(channels)` | A resonant low-pass filter at `cutoff` |
| `dirt_envelope(channels)` | An envelope of `attack`, `hold` and `release`, of the curvature `curve` |
| `dirt_grain_envelope(channels)` | An envelope over the sustain, with the plateau `plat` and the tilt `tilt` |
| `dirt_pitch_shift(channels)` | A pitch shifter by the ratio `psrate`, with the time dispersion `psdisp` |
| `dirt_tremolo(channels)` | A tremolo at `tremolorate` hertz, of the depth `tremolodepth` |
| `dirt_phaser(channels)` | A phaser at `phaserrate` hertz, of the depth `phaserdepth` |

The doc comment of each function gives its controls and their ranges.

```rust
use scsynth_dirt::synthdefs::{self, Delay};

let core = synthdefs::core(2, Delay::Comb)?;
assert_eq!(core.len(), 25);
assert_eq!(core[0].name, "dirt_monitor2");

// A player of samples of 4 channels, which `core` leaves out, as the data of a /d_recv.
let player = synthdefs::dirt_sample(4, 2)?;
assert_eq!(player.name, "dirt_sample_4_2");
assert!(!player.as_bytes()?.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

- `Delay` is the delay line of `dirt_delay`. `Delay::Comb` is the `CombL`
  of SuperDirt without sc3-plugins, and the default. `Delay::Switch` is the
  `SwitchDelay` of sc3-plugins.
- `core-synths-global.scd` picks `SwitchDelay` once sclang has its class,
  and `CombL` otherwise. The crate builds both, and `Config::delay` picks
  one. Without the feature `sc3-plugins`, `Delay::Switch` gives
  `BuildError::UnknownClass`.
- A player builds for any count of sample channels, because `DirtPan` takes
  any count of signals. `core` holds the players of 1 and 2 sample
  channels, as SuperDirt does with its `maxSampleNumChannels` of 2.
- The pseudo-UGens of `SuperDirtUGens.sc` are functions over the builder of
  scsynth-synthdef: `DirtPan` with `DirtPanBalance2` and `DirtSplayAz`,
  `DirtGateCutGroup`, and `DirtPause`.
- `dirt_monitor` reads `SuperDirt.postBadValues`, which is false, so its
  `CheckBadValues` posts nothing.
- The arithmetic of two numbers happens in Rust, as sclang computes it in
  f64: `60.midicps`, `2 ** 24` and `log2(-60.dbamp)`. `binary_op` of
  scsynth-synthdef makes a UGen of two numbers.
- Each function refuses 0 channels and more than 65,535 with
  `BuildError::BadCount`, but a player refuses 0 channels with
  `BuildError::NotANumber` (see [Traps](#traps)). A player also refuses 0
  sample channels and more than 65,535 with `BuildError::BadCount`.

The crate proves its definitions against sclang:

- Each definition follows its source in SuperDirt line by line, in the
  order in which sclang evaluates it. A receiver comes before its
  arguments, and the arguments go from left to right. That order gives the
  UGens their order in the file.
- `tests/synthdefs.rs` compares each definition with the definition of its
  name that sclang 3.14.1 built for SuperDirt, byte for byte. It covers 1,
  2 and 4 channels, and the players of 1 to 4 sample channels. With the
  feature `sc3-plugins`, it covers the delay of `SwitchDelay` too.
- The crate does not embed the bytes of sclang. It builds every definition
  with scsynth-synthdef.

## Strictness

The crate checks its inputs, and gives the reason of each refusal.
`Dirt::new` refuses a configuration whose node ids or buses overlap.
`play` drops a sound with a value of the wrong kind, where SuperDirt sends
the text to scsynth. `Sound::from_args` refuses an argument that it cannot
read, and `Library` refuses a call with too few free buffers. A refusal
changes nothing.

### Limits

Over UDP, each packet travels in one datagram. On macOS, a UDP socket of
the default size sends at most 9,216 bytes (`net.inet.udp.maxdgram`). For
1, 2 and 4 channels and 12 orbits, each message of `start` fits 8,192
bytes, the largest datagram that sc-app3 sends.

The definitions take these bytes as files of their own, with
`Delay::Comb`:

| Channels | The 25 definitions | The largest | The most wire buffers |
| --- | --- | --- | --- |
| 1 | 26,584 bytes | `dirt_stretchsample_2_1`, 2,547 bytes | `dirt_stretchsample_2_1`, 7 |
| 2 | 31,265 bytes | `dirt_leslie2`, 3,461 bytes | `dirt_leslie2`, 10 |
| 4 | 39,779 bytes | `dirt_reverb4`, 5,080 bytes | `dirt_leslie4`, 11 |

- Up to 7 channels, each definition fits 8,192 bytes. For 8 channels,
  `dirt_reverb8` alone does not: it takes 8,923 bytes.
- A scsynth started without `-w` has 64 wire buffers, and refuses a
  definition that needs more (see the README of scsynth-synthdef). The
  definitions of 1, 2 and 4 channels need at most 7, 10 and 11.
- The delay of `SwitchDelay` fits both limits too, for 1, 2 and 4
  channels.

### Traps

The crate does not rule these out. Each one costs a host time.

- The definitions take the names of SuperDirt, which hold the channel count
  and not the delay. So the instances of one channel count on one server
  share them, and the last `/d_recv` of a name wins. A synth keeps the
  definition that it started with (`SC_GraphDef.cpp:519-535`). So each
  instance keeps the delay of its own `start`, as long as the commands of
  two `start`s do not interleave.
- scsynth sends `/n_end` only to a client that registered with `/notify`.
  Without it, a live sound leaves only once a later sound starts after its
  end, or once every id of `sound_nodes` is live. `start` and `stop` also
  forget every live sound. A sound without `when` has no end, so the cuts
  of later sounds keep reaching its group. scsynth fails such an `/n_set`
  without a post, under the `/error -1` of the bundle.
- A sample plays only after its `/b_info`. Until the host passes that reply
  to `Library::reply`, a sound of the sample drops with
  `Dropped::NotLoaded`, as `tests/play.rs` checks.
- `play` does not know whether `start` ran. A sound before `start`, or after
  `stop`, reaches nodes that scsynth lacks.
- A setter turns its parameter on, at any value. libdirt takes a `cutoff`
  of 0 as no filter (`audio.c:652`), but here `cutoff(0.0)` plays
  `dirt_lpf` at 20 hertz, as SuperDirt does. So a host sets only the
  fields that its sound has.
- `dirt_reverb1` writes 2 channels on its effect bus, because its loop
  takes the 2 delay times of its `DelayN`. In SuperDirt with 1 channel, the
  second channel lands on the synth bus of the next orbit. Here, the effect
  bus has 2 channels, and `tests/scsynth.rs` hears the second one.
- `dirt_reverb1` also has no feedback. Its `LocalOut` has 2 channels and
  its `LocalIn` has 1, and with counts that differ, `LocalOut` writes
  nothing (`IOUGens.cpp:1645`). SuperDirt has the same definition.
- On 3 channels or more, `DirtPan` drops its `mul`, because `DirtSplayAz.ar`
  takes it and never reads it. The players pass a `mul` of 1, so their sound
  does not change.
- A player of 0 channels fails with `BuildError::NotANumber`, and not with
  `BadCount`: the place of its first signal in `DirtSplayAz` is 0 / 0.
  `tests/synthdefs.rs` checks it.
- The time of a sound sums from left to right: `when + lag`, then
  `offset * |speed|`. Another order can change the last bit of the sum, and
  with it the time tag. `tests/events.rs` compares each time tag with the
  time of SuperDirt.
- The monitor starts paused, and its envelope rises over 0.01 seconds once
  it runs. So the first sound of an orbit after `start` comes in with that
  rise. With `limitertype` 1, the `Limiter` of the monitor also delays
  every sound by its lookahead. So the acceptance tests set `limitertype` 0
  on each orbit, and play a silent sound first.
- The envelope of a cut runs at control rate. So a cut in the middle of a
  block keeps the level of the sound to the end of that block, and fades
  over the next one. `tests/scsynth.rs` checks the silence after a cut.
- The first sample of a sound sits 1 frame after the frame of its time,
  because the envelope of its gate starts at 0. `tests/scsynth.rs` measures
  it for a time in the middle of a block.

In NRT mode, scsynth 3.14.1 adds its own traps:

- It sends no replies, so the host gives the library a `/b_info` of its
  own. A time tag before the one of the entry before it ends the score, and
  a completion bundle needs the immediate time tag.
- At the end of a score, it unloads its plugins before it frees its nodes.
  A synth that still lives then crashes scsynth, after the output file is
  complete. So a score ends with the bundles of `stop`.
- In the first block, `LocalIn` reads memory that nothing wrote. A reverb
  made at time 0 starts with garbage in its loop, measured up to 1e14 on 4
  channels. So a score sends `start` after the first block.
- scsynth exits with success after a failed command. A line of its output
  with `FAILURE`, `ERROR`, `exception` or `could not be opened` tells of a
  failure, and the acceptance tests check for these lines.

## Differences from SuperDirt

- An unknown sound sends nothing. SuperDirt still sends its group, its
  global synths and its gate, which play no sound without a source.
- A sound without `s` and `sound` drops, because it has no name. SuperDirt
  looks for a sound of the name `nil`, as `nil.asString` of sclang gives
  it.
- A text in the place of a number drops the sound, so that no text reaches
  scsynth as a control value. SuperDirt sends the text to scsynth, or drops
  the sound without a post.
- The index after the colon of `sound` reads a decimal number alone. sclang
  also reads a hexadecimal one there, as `0x2`, which the port of `atof`
  leaves out.
- A sound cannot replace the values that SuperDirt keeps in its event, such
  as `out`, `buffer`, `instrument`, `unitDuration` and `latency`. The
  instance owns its buses, its buffers and its definitions. In SuperDirt,
  the pairs of a sound come first.
- `cutAll` is a number, and any number but 0 turns it on. SuperDirt takes
  the Boolean true, and a `Value` holds no Boolean.
- The `cut_gate` messages of a bundle come in the order of the starts of
  their sounds, so that a bundle stays the same from run to run. SuperDirt
  sends them in the order of a dictionary of sclang.
- The gate gets no `sample` argument. SuperDirt sends a hash of sclang
  there, and `dirt_gate` has no such control.
- A `Value` holds no array, so a sound plays one synth per module. In
  SuperDirt, a parameter that is an array plays one synth per element.
- No `diversion`, `play`, `playInside` or `notFound` function of an event:
  a `Value` holds no function.
- `metatune` has no effect. The crate reads no file, so it cannot read the
  `smpl` chunk of a WAV file.
- A sample that is not loaded drops its sound, because `play` returns its
  bundles at once. SuperDirt can read a sample at its first use.
- A file that scsynth fails to read keeps its index, as a failed sample. The
  crate learns of the failure from scsynth, after `add` gave the index. In
  SuperDirt, sclang reads the header first, and such a file takes no index.
- `Sound::from_args` takes a long as a number, and refuses the type `I`,
  because a long holds a number and `I` carries no data. sclang has no OSC
  type for a long, and reads `I` as the float infinity.
- A sample of more than 2 channels drops its sound, because `start` loads
  the players of 1 and 2 sample channels. SuperDirt sends a synth whose
  definition scsynth lacks.
- For 1 channel, the effect bus of an orbit has 2 channels, so that no orbit
  hears the reverb of another. In SuperDirt, the reverb of 1 channel also
  writes the synth bus of the next orbit.
- The orbits go in the group `root` of the host, on the node ids and the
  buses of the `Config`, because the host owns its tree of nodes. SuperDirt
  makes a group of its own, and takes its ids and buses from sclang.
- A parameter goes to scsynth as a float, because a `Value` holds an f64.
  sclang sends an Integer as an int, and scsynth sets a control to the same
  value from both.
- A live sound leaves once a later sound starts after its end, because the
  instance knows the end of each sound. SuperDirt keeps a sound until its
  `/n_end` arrives, and can send `cut_gate` to a group that is gone.
- `stop` frees the group and the global synths of each orbit at once, so
  that nothing of the instance stays on the server. SuperDirt releases each
  global synth through its `gate`, and leaves the group of the orbit.

## Left out

Version 0.1 is the core of SuperDirt for samples. It leaves out these parts
of SuperDirt:

- The synth sounds: the synths of `default-synths.scd` and of the extras,
  sounds of a synth by name, the pitch of a synth, and MIDI sounds.
- The extra effects and their modules, which need sc3-plugins, and modules
  or global effects of the host's own.
- The routing buses, the modules `out_to` and `map_from`, and the sound
  `from`.
- The RMS replies of `dirt_rms`, and their read.
- The OSC server of SuperDirt, with `/dirt/hello`, `/dirt/handshake`,
  `/dirt/synth-info` and the control buses of Tidal. A host reads
  `/dirt/play` itself, and `Sound::from_args` reads its arguments.
- `superpanic`, an output bus per orbit, and its change.
- Loads at the first use, sample data in `/b_setn`, and the `smpl` chunk
  of a WAV file.
- Samples of more than 2 channels in `play`, and the expansion of array
  arguments.
- The `minSustain` of an orbit: the shortest sound is 8 samples of
  `sample_rate`.
- A `/d_load` of the definitions that pass one datagram, and the count of
  the wire buffers that an instance needs.
- A profile of StrudelDirt.

## Tests

`cargo test --all-features` runs every test but the acceptance tests, and
needs no SuperCollider. The tests compare the crate with golden files that
sclang 3.14.1 wrote for SuperDirt.

| Test | What it checks |
| --- | --- |
| `tests/synthdefs.rs` | Each definition against its golden bytes, and the sizes of [Limits](#limits) |
| `tests/golden.rs` | The golden definition files and `vowels.json` |
| `tests/golden_events.rs` | The shape of `events.json` |
| `tests/events.rs` | Each sound of `events.json` against the bundles of SuperDirt, message by message |
| `tests/dirt.rs` | `Dirt::new`, `start`, `stop` and the orbits |
| `tests/play.rs` | The drops, the time tags, the live sounds and the cuts |
| `tests/sound.rs` | `Sound`, `Value` and `from_args` |
| `tests/library.rs` | `Library` and its replies |
| `tests/fs.rs` | `fs::list`, with the feature `fs` |
| `tests/serde.rs` | The forms of serde, with the feature `serde` |
| `tests/scsynth.rs` | The acceptance tests, in scsynth |

The unit tests of `src` cover the arithmetic of sclang, the split of a
name, the time tags and the table of the vowels. The golden files are in
`tests/fixtures`:

- `definitions/core_1.scsyndef`, `core_2` and `core_4` hold the 25
  definitions of each channel count, with the delay of `CombL`.
- `definitions/players_3_4_1.scsyndef`, `_2` and `_4` hold the 6 players
  of 3 and 4 sample channels. They test `DirtPan` for more than 2 signals.
- `definitions/switch_delay_1.scsyndef`, `_2` and `_4` hold `dirt_delay`
  with `SwitchDelay`.
- `vowels.json` holds the frequencies, the amps and the rqs of the 5
  vowels in the 5 registers of the Vowel quark. The tests compare each
  number as an f32, the type of a control value on the wire.
- `events.json` holds 12 scenarios of sounds, with the bundles that
  SuperDirt sent for each sound.

The events:

- Each scenario runs on a new instance of SuperDirt with 2 orbits. Its
  library holds 4 names: `bd` with 2 mono samples, `sn` with a stereo one,
  `long` with a mono one over 16,777,216 frames, and `x_bd` for `bank`.
- The scenarios cover the start, the names, the time, the span, the pitch,
  the level and the place, the modules, the global synths, the cut and the
  orbits. The last two scenarios play the same three sounds on 1 and on 4
  channels.
- An object of the script takes the place of the address of the server,
  and records each bundle with its time. The script gives each event its
  `when` as `timeStamp` and as `latency`, so the time of a bundle is the
  time of its sound.
- The texts of a sound are Symbols, as the OSC parser of sclang makes them
  for Tidal.
- `tests/events.rs` pairs the node ids of both sides by their order, and
  compares each number as an f32. It compares each time tag with the time
  of SuperDirt at `epoch` 0. In a cut bundle, the `cut_gate` messages
  compare as a set.
- The first message of a golden start makes the group of SuperDirt, which
  the crate leaves to the host.
- The script sees no `/n_end`. So in the scenario of the cut, each sound
  starts before the end of the earlier ones.
- The tests read each number of `events.json` from its text, with
  `str::parse` of Rust. The default parse of `serde_json` reads some of its
  floats 1 unit of the last bit off.

The generators and the tests take these sources:

| Source | At | For |
| --- | --- | --- |
| sclang | 3.14.1 | Both generators |
| SuperDirt | `a3046d1` | Both generators |
| The Vowel quark | `ab59caa` | Both generators |
| sc3-plugins | 3.14.0 | The second configuration of `fixtures.scd`, and the acceptance test of `SwitchDelay` |
| scsynth | 3.14.1 | The acceptance tests |
| uzu/dirt | `123b399` | The 32 fields of `tests/sound.rs` |

Both generators fail unless sclang is 3.14.1, and the clones of SuperDirt
and of the Vowel quark are at their commits. A clone also needs no change
to its tracked files, and each class comes from the include paths of the
configuration. The generators set `Server.default` to a server object of
their own, and boot nothing.
`generator/sclang_conf.yaml` names the class folders of SuperCollider, of
SuperDirt and of the Vowel quark, and `generator/sc3_plugins/` adds the
folder `SC3plugins`. For another machine, change their paths. Run the
generators from the root of the repository, with absolute paths, because
`~/.bin/sclang` changes the directory:

```sh
sclang -l "$PWD/generator/sclang_conf.yaml" "$PWD/generator/fixtures.scd"
sclang -l "$PWD/generator/sc3_plugins/sclang_conf.yaml" "$PWD/generator/fixtures.scd"
sclang -l "$PWD/generator/sclang_conf.yaml" "$PWD/generator/events.scd"
```

- With the first configuration, `fixtures.scd` writes the core files, the
  players of 3 and 4 sample channels, and `vowels.json`. With the second,
  it writes the files of `SwitchDelay`.
- `events.scd` writes `events.json`.

`tests/scsynth.rs` renders the packets of an instance in scsynth 3.14.1, in
NRT mode, and reads the sound that comes out. It is ignored by default:

```sh
cargo test --all-features -- --ignored
```

- scsynth runs with `-N`, so it opens no port and no audio device. When it
  exits, its output is complete.
- `SCSYNTH` names the scsynth, and `SCSYNTH_PLUGINS` its plugins. Both
  default to the bundle of `/Applications/SuperCollider.app`. `-U` keeps
  the plugins of the Extensions folder out.
- With the feature `sc3-plugins`, the test of `SwitchDelay` also loads the
  plugins of sc3-plugins. `SCSYNTH_SC3_PLUGINS` names their folder, by
  default the folder `SC3plugins` of the Extensions.
- Each test acts as a host. It makes `root` at time 0, then sends `start`
  and loads its samples after the first block. It ends with the bundles of
  `stop`.
- A sound of a constant sample comes out at the level that the chain
  predicts, 1 frame after the frame of its time. A cut silences the sound
  that it cuts, and the delay echoes.
- On 1 channel, a sound is the sum of the channels of its sample, and the
  reverb leaves the next orbit silent. On 4 channels, a sound comes out on
  the channel of its pan.
- scsynth prints no failure. A test of its own makes scsynth fail on
  purpose, and checks that each failure line comes out.

## Features

- `sc3-plugins`: `Delay::Switch`, through the feature `sc3-plugins` of
  scsynth-synthdef, whose catalogue holds `SwitchDelay`. To load that
  delay, scsynth needs the plugins of sc3-plugins.
- `fs`: the module `fs`, with `fs::list`.
- `serde`: `Serialize` and `Deserialize` for `Config`, `Sound`, `Value`,
  `Delay` and `Register`. It also turns on the feature `serde` of
  scsynth-osc, for the packets.

```rust
# #[cfg(feature = "serde")]
# {
use scsynth_dirt::scsynth_osc::NodeId;
use scsynth_dirt::{Config, Delay, Register, Sound};

let sound = Sound::new().at(1.5).s("bd").n(3.0);
let json = serde_json::to_string(&sound)?;
assert_eq!(json, r#"{"when":1.5,"parameters":{"n":3.0,"s":"bd"}}"#);
assert_eq!(serde_json::from_str::<Sound>(&json)?, sound);

let config = Config {
    channels: 2,
    orbits: 12,
    root: NodeId(1),
    fixed_nodes: 100,
    sound_nodes: 1000..2000,
    first_bus: 16,
    out_bus: 0,
    sample_rate: 48_000.0,
    delay: Delay::Comb,
    register: Register::CounterTenor,
    epoch: 0.0,
};
assert_eq!(
    serde_json::to_string(&config)?,
    concat!(
        r#"{"channels":2,"orbits":12,"root":1,"fixed_nodes":100,"#,
        r#""sound_nodes":{"start":1000,"end":2000},"first_bus":16,"out_bus":0,"#,
        r#""sample_rate":48000.0,"delay":"comb","register":"counter_tenor","epoch":0.0}"#,
    ),
);
# }
# Ok::<(), Box<dyn std::error::Error>>(())
```

- A `Sound` is an object of `when`, a number or `null`, and `parameters`, an
  object of its parameters by name.
- A `Value` is a bare number or a bare string.
- A `Config` is an object of its fields. `root` is a bare int, and
  `sound_nodes` is an object of its `start` and its `end`.
- `Delay` and `Register` name their variants in snake case, as scsynth-osc
  does, such as `"comb"` and `"counter_tenor"`.

## License

The license of the crate is the GNU General Public License, version 3 or
any later version (GPL-3.0-or-later). The file `LICENSE` holds the text of
version 3.

This license follows from SuperDirt alone. The synth definitions and the
event logic of the crate translate SuperDirt, which is GPL-2.0-or-later.
The interface takes its names from libdirt of uzu/dirt, which is under the
GPL version 3. The crate copies no code of libdirt.
