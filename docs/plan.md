# The plan of scsynth-dirt

Version 0.1 is in work. This file is its plan: the decisions, the sources,
the facts, the design and the steps. When 0.1 is done, its decisions and
its facts move to the README, the tests and the CHANGELOG. This file then
keeps what is still open.

## The goal

scsynth-dirt plays Dirt events on scsynth. Its interface follows libdirt,
the library of the classic Dirt sampler (codeberg.org/uzu/dirt). Its sound
is the sound of SuperDirt (codeberg.org/musikinformatik/SuperDirt): the
crate builds the synth definitions of SuperDirt with scsynth-synthdef, and
sends for each sound the messages that SuperDirt sends.

The crate does no I/O. A call returns packets of scsynth-osc, and the host
sends them. The host passes the replies of scsynth back. One instance lives
inside a group that the host made, on node ids and buses that the host
gives. Many instances share one library of samples.

## Decisions

The owner accepted these on 2026-11-19.

| ID | Decision |
| --- | --- |
| D1 | The license is GPL-3.0-or-later, because the definitions and the event logic translate SuperDirt, which is GPL-2.0-or-later. |
| D2 | The name is `scsynth-dirt`. |
| D3 | The crate is a state machine without I/O: calls and replies go in, packets come out. |
| D4 | The interface is an instance, and a `Sound` of open parameters with the 32 fields of libdirt as setters. No C ABI. |
| D5 | The reference of the sound is SuperDirt at `a3046d1`. StrudelDirt stays out. |
| D6 | Version 0.1 is the core: 25 definitions, the samples, 13 modules, the global effects. |
| D7 | The crate builds both delays, and the configuration picks one. The default is `CombL`. |
| D8 | The siblings come by git tag: scsynth-osc `v0.2.0`, scsynth-synthdef `v0.15.0`. CI has no `cargo package` step. |
| D9 | The samples are file paths that scsynth reads. |
| D10 | The crate builds the definitions with scsynth-synthdef. It does not embed the bytes of sclang. |

## The sources

| Source | Where | At |
| --- | --- | --- |
| SuperDirt | `~/Projects/SuperDirt` | `a3046d1` |
| The Vowel quark | `~/Projects/Vowel` | `ab59caa` |
| uzu/dirt | `~/Projects/dirt` | `123b399` |
| SuperCollider | `~/Projects/supercollider` | `Version-3.14.1` |
| sclang 3.14.1 | `~/.bin/sclang` | |
| scsynth 3.14.1 | `/Applications/SuperCollider.app/Contents/Resources/scsynth` | |
| sc3-plugins | `~/Library/Application Support/SuperCollider/Extensions/SC3plugins` | 3.14.0 |

`~/.bin/scsynth` needs a running jackd, also for `-N`. The tests use the
scsynth of the bundle.

## Facts

Each fact was measured on 2026-11-19, with sclang 3.14.1 on macOS, or read
in the sources named above.

1. sclang builds the definitions of SuperDirt with no server:
   `SuperDirt.newCopyArgs(channels, server)`, then `modules = []`, a
   `DirtSoundLibrary`, and the load of the synth files in an environment
   whose `dirt` is that object.
2. For 1 channel, the extras of SuperDirt throw (`compressor` sends `mean`
   to one channel). The three core files load for every channel count:
   `core-modules.scd`, `core-synths-global.scd`, `core-synths.scd`.
3. `core-synths-global.scd` builds `dirt_delay` with `SwitchDelay` if
   sclang has that class, and with `CombL` otherwise.
4. When the calls follow the order in which sclang evaluates the source, a
   definition built with scsynth-synthdef 0.15 equals the bytes of sclang.
   `dirt_gate2`, `dirt_sample_1_2` and `dirt_sample_2_2` did so at the
   first try.
5. `binary_op` makes a UGen of two numbers, where sclang computes a
   number. So the arithmetic of two numbers happens in Rust: `60.midicps`,
   `2 ** 24`, `log2(-60.dbamp)`.
6. `60.midicps` is 261.62556530059862: `440 * 2 ** ((60 - 69) / 12)` in
   f64.
7. `ugen` refuses `SendPeakRMS.kr`. The UGen goes through `add`, at
   control rate, with no output. Its inputs are `replyRate`, `peakLag`,
   `replyID`, the count n, the n signals, the size of the command name,
   and its characters.
8. With the `CombL` delay, the 25 core definitions of 2 channels take
   31,265 bytes as files of their own. `dirt_leslie2` is the largest, with
   3,461 bytes and 10 wire buffers. For 4 channels, `dirt_reverb4` is the
   largest, with 5,080 bytes, and `dirt_leslie4` needs the most wire
   buffers, 11.
9. `DirtEvent` sends its messages inside `server.makeBundle(~latency, …)`.
   A sound with a `cut` that is not 0 sends two bundles: `/error -1` and
   the `cut_gate` sets, then the sound.
10. sclang writes nil as the int 0 in a message. So a module argument that
    the sound lacks is 0, and a parameter of a global synth that the next
    sound lacks goes to 0.
11. sclang writes some addresses as numbers or bare symbols: `9` is
    `/s_new`, `12` is `/n_run`, `15` is `/n_set`, `21` is `/g_new`,
    `'s_new'` and `'g_new'` have no slash.
12. sclang keeps Integer and Float apart in a message (`cutoff 800`,
    `speed -2`). A `Sound` holds f64, so the tests compare numbers.
13. `Synth.newPaused` takes its server from `Integer:asTarget`, which is
    `Server.default`. A generator that does not set `Server.default` sends
    to UDP 57110.
14. Every sound resumes the delay, the reverb and the monitor of its
    orbit, because `delayAmp`, `lock`, `cps` and `dry` always hold a value.
15. An unknown sound still sends its group, its effects and its gate in
    SuperDirt, with no source.
16. When its definitions are in place, `/d_recv` runs its completion
    message. scsynth runs asynchronous commands in order.
17. The `Limiter` of `dirt_monitor` delays the sound by its lookahead, and
    the monitor starts paused, so the 10 ms attack of its envelope shapes
    the first sound after a start.
18. In NRT mode, scsynth drops every reply, a completion bundle needs the
    immediate time tag, and time tags out of order end the score.
19. sclang sorts strings by their bytes: `10.wav`, `9.wav`, `A.wav`,
    `_x.wav`, `a.aiff`.
20. The `amps` of a vowel are the `dbamp` of its levels, and its `rqs` the
    reciprocals of its widths (`Vowel.sc`).
21. cargo-deny counts a git dependency without `version` as a wildcard.
    `cargo package` fails on a git dependency that crates.io lacks.

## The design of 0.1

### The modules

| Module | Holds |
| --- | --- |
| `config` | `Config`, and the errors of a configuration |
| `sound` | `Sound`, `Value`, the setters of the 32 fields of libdirt, the read of the arguments of `/dirt/play` |
| `library` | `Library` and `Sample`: the names, the buffers, their loads and replies |
| `dirt` | `Dirt`: `new`, `start`, `stop`, `play`, `reply`, the orbits |
| `event` | Private: the port of `DirtEvent`, from a sound to its bundles |
| `orbit` | The defaults, the nodes, the buses and the state of the global synths of one orbit |
| `vowel` | `Register`, and the table of the Vowel quark |
| `pitch` | Private: `midicps` |
| `synthdefs` | `Delay`, and the definitions: the pseudo-UGens of SuperDirt, the sample players, the routing, the effects, the global synths |
| `fs` | With the feature `fs`: a folder as SuperDirt lists it |

`synthdefs` and `fs` are public modules. The root of the crate re-exports
the public types of the other modules. The crate also re-exports
`scsynth_osc` and `scsynth_synthdef`.

### Config

`Config` holds the structure of an instance, and nothing that a sound can
change.

| Field | Meaning |
| --- | --- |
| `channels` | The channel count of the orbits, 1 or more |
| `orbits` | The number of orbits, 1 or more. SuperDirt starts 12. |
| `root` | The group of the host. The instance makes its nodes inside it, and never frees it. |
| `fixed_nodes` | The first of `6 * orbits` node ids: for each orbit its group, then its monitor, rms, leslie, reverb and delay |
| `sound_nodes` | A range of node ids for the groups of the sounds. It starts over at its end. |
| `first_bus` | The first of `3 * channels * orbits` private audio buses: for each orbit its synth bus, its dry bus, its effect bus |
| `out_bus` | The output bus of the monitors |
| `sample_rate` | The sample rate of the server |
| `delay` | `Delay::Comb` or `Delay::Switch`. `Switch` needs the feature `sc3-plugins`. |
| `register` | The voice register of the vowels. The default is the counter tenor, as in SuperDirt. |
| `epoch` | The seconds between the OSC epoch and time 0 of `when`: 2,208,988,800 for Unix time, 0 for a score of `scsynth -N` |

`Dirt::new` refuses a configuration whose counts are 0, whose ranges
overlap or are too short, or whose delay needs a missing feature.

### Sound

A `Sound` is an optional `when`, in seconds, and an ordered set of
parameters. A `Value` is a number (f64) or a text. A parameter that is not
set stays out. In SuperDirt, a parameter that the sound holds is on.

- `Sound::default()` is empty.
- `set(name, value)` sets any parameter. A second `set` of a name replaces
  its value.
- Each of the 32 fields of libdirt is a setter by its name. The list
  follows.
- `Sound::from_args` reads the arguments of a `/dirt/play` message, which
  are pairs of a name and a value. A name has the type tag `s` or `S`. A
  value has one of the type tags `i`, `f`, `h`, `d`, `s`, `S`, `T` and
  `F`. It refuses an odd count, a name that is no text, and a value of
  another type.

```text
cps s offset begin end speed pan velocity vowel cutoff resonance
accelerate shape kriole gain cut delay delaytime delayfeedback crush
coarse hcutoff hresonance bandf bandq unit loop n attack hold release
orbit
```

### Library

A `Library` maps a name to its samples, in order. It takes its buffer
numbers from a range of the host.

- `add(name, paths)` replaces the samples of a name, and `append` adds to
  them. Each returns one `/b_allocRead buffer path 0 -1` per file, with
  `/b_query buffer` as its completion, and the `/b_free` of each buffer
  that it replaced.
- `reply` takes a reply of scsynth. A `/b_info` gives the frames, the
  channels and the sample rate of a buffer. A `/fail` of `/b_allocRead`
  marks its buffer as failed.
- `remove(name)` returns the `/b_free` of each buffer of the name.
- `sample(name, n)` gives the sample at index `n`, truncated to a whole
  number and wrapped to the count.
- A sample plays only after the library holds its `/b_info`.

With the feature `fs`, a helper lists a folder as SuperDirt does. Each
folder inside it is a name. Its samples are its files with the extensions
`wav`, `aif`, `aiff` and `aifc`, without regard to case. They come sorted
by the bytes of their paths.

### The orbits

Each orbit has these defaults, as `DirtOrbit.makeDefaultParentEvent` sets
them. `Dirt::orbit_mut(index).set(name, value)` changes one, as
`DirtOrbit.set` does.

```text
cps 1        offset 0     begin 0      end 1        speed 1      pan 0.5
gain 1       overgain 0   cut 0        unit r       octave 5     dur 1
lag 0        loop 1       dry 0        lock 0       amp 0.4      fadeTime 0.001
```

`n` has no default. It counts as 0 for the index of a sample.

A sound reads a parameter from its own pairs, then from its sample, then
from the defaults of its orbit. A sample gives `note` 0. So a sample plays
at the frequency of note 60, unless the sound sets `note`, `midinote` or
`freq`.

The getters of an orbit give its group, its five global synths, its three
buses and its output bus.

### start and stop

`start` returns one list of packets, safe to send in order with no wait.

1. One `/d_recv` per global definition: monitor, rms, delay, reverb,
   leslie.
2. One `/d_recv` per event definition. The node bundle of orbit k rides as
   the completion of the event definition `k mod 20`, with the immediate
   time tag.
3. The node bundle of an orbit starts with `/g_new group 0 root`. Then it
   holds two messages for each global synth, in the order monitor, rms,
   leslie, reverb, delay. The first is
   `/s_new name id 3 group outBus o dryBus d effectBus e orbitIndex k`.
   The second is `/n_run id 0`. So the five synths sit after the group, in
   the order delay, reverb, leslie, rms, monitor.

`definitions` gives the 25 definitions, for a host that loads them in
another way. `stop` returns, per orbit, one bundle with the immediate time
tag: `/error -1`, `/n_free` of the group and of the five synths,
`/error -2`. It clears the state of the global synths and the cut groups.

### play

`play(library, sound)` is the port of `DirtEvent.play` for a sample sound.
It returns the bundles of the sound, or the reason of a drop:

- Silence, or a sound without a name.
- An unknown sound.
- A sample that is not loaded.
- A sample of more than 2 channels.
- A bad unit.
- A sustain under 8 samples.
- A parameter of the wrong kind. SuperDirt throws on a text in the place
  of a number.

1. The name. Without `s`, the text of `sound` splits at `:`. Its first
   part is the name. Its second part gives `n`: the number at its start,
   or 0. With `bank`, the name is `bank_name`. An empty name is silence. A
   sound without `s` and `sound` drops.
2. The orbit is `orbit`, truncated and wrapped to the count.
3. The sample is `library.sample(name, n)`. A negative `n` counts from the
   end.
4. The span, as `orderTimeSpan` and `calcTimeSpan`:
   - A negative `speed` swaps `begin` and `end`. `length` is
     `|end - begin|`.
   - `freq` is the `freq` of the sound, else the `midicps` of `midinote`,
     else that of `note + octave * 12`.
   - `unitDuration` is `frames / sample rate of the buffer * 60.midicps /
     freq`.
   - With `unit` c, `speed` is `speed * cps * unitDuration`.
   - Without `accelerate`, `endSpeed` is `speed`, and the average speed
     `|speed|`. With it, `endSpeed` is `speed * (1 + accelerate)`, and the
     average speed the mean of `|speed|` and `|endSpeed|`.
   - With `unit` r or c, `unitDuration` is `unitDuration * length /
     average speed`. With `unit` s, it is `length`. The text `rate` counts
     as r. Any other unit drops the sound.
   - `delta` is the `delta` of the sound. Without it, `delta` is `dur`,
     which is 1 by default.
   - `sustain` is the `sustain` of the sound. Else, with `legato`, it is
     `delta * legato`. Else `unitDuration` becomes `unitDuration * |loop|`,
     and `sustain` takes that value.
   - `sustain` is at most `unitDuration`.
   - `fadeTime` is at most `sustain * 0.19098`. If `begin` is not 0,
     `fadeInTime` is `fadeTime`. Otherwise it is 0.
   - With `timescale`, `sustain` is `sustain * timescale`.
   - `sustain` loses `fadeTime + fadeInTime`.
5. A sustain under `8 / sample_rate` drops the sound.
6. With `channel`, `pan` gains `channel / channels`. Then `pan` is
   `pan * 2 - 1`. `delayAmp` is `delay`, or 0.
7. The time of the sound is `when + lag + offset * |speed|`, with the
   speed of step 4. Its time tag adds `epoch`. Without `when`, the time
   tag is the immediate one.
8. With a `cut` that is not 0, a first bundle holds `/error -1`. Then it
   holds `/n_set node cut_gate 0` for each live sound that passes these
   tests:
   - Its cut group is `|cut|`.
   - Its orbit is this orbit, and its `when` differs from this `when`. A
     sound without `when` differs from every sound. With `cutAll`, this
     test is off.
   - For a negative `cut`, its sample is this sample.

   If the value of `cutAll` is a number other than 0, `cutAll` is on.
9. The bundle of the sound:
   - The global synths, in the order delay, reverb, leslie, rms, monitor.
     The first table below gives the parameters of each. The rms is off,
     and sends nothing.
   - If a parameter of a global synth has a value, the synth resumes. The
     monitor always resumes. A resume is `/n_run id 1`, then
     `/n_set id pauseImmediately 0 dirt_resumed 1`.
   - Then one `/n_set` names each parameter whose value differs from the
     last sound of the orbit. A parameter that has no value now is 0.
   - `/g_new group 1 orbit_group`, with the next id of `sound_nodes`. With
     a `cut` that is not 0, the group joins the live sounds.
   - The source: `/s_new name -1 1 group`, then the arguments `bufnum`,
     `sustain`, `speed`, `freq`, `endSpeed`, `begin`, `end`, `pan` and
     `out`. The name is `dirt_sample_<sample channels>_<channels>`, with
     `_long` for a buffer over 16,777,216 frames.
   - With `timescale`, the name of the source is `dirt_stretchsample_…`.
     Its arguments gain `timescale` and `timescalewin`, which is 1 by
     default.
   - One `/s_new name -1 1 group …` per module whose test holds, in the
     order of the second table below. A missing argument is the int 0.
   - The gate: `/s_new dirt_gate<channels> -1 1 group`, then the arguments
     `in`, `out`, `amp`, `gain`, `overgain`, `cut`, `sustain`,
     `fadeInTime` and `fadeTime`. `in` is the synth bus, `out` the dry
     bus, and `cut` is `|cut|`.
   - SuperDirt also sends `sample` to the gate, a hash of sclang. The
     crate leaves it out, because the definition has no such control.

| Global synth | Parameters |
| --- | --- |
| delay | `delaytime`, `delayfeedback`, `delaySend`, `delayAmp`, `lock`, `cps` |
| reverb | `size`, `room`, `dry` |
| leslie | `leslie`, `lrate`, `lsize` |
| monitor | `limitertype` |

| Module | Test | Definition | Arguments |
| --- | --- | --- | --- |
| vowel | `vowel` is a, e, i, o or u | `dirt_vowel` | `out`, `vowelFreqs`, `vowelAmps`, `vowelRqs` (arrays of 5), `resonance` |
| shape | `shape` | `dirt_shape` | `shape`, `out` |
| hpf | `hcutoff` | `dirt_hpf` | `hcutoff`, `hresonance`, `out` |
| bpf | `bandf` | `dirt_bpf` | `bandqf` (the value of `bandf`), `bandq`, `out` |
| crush | `crush` | `dirt_crush` | `crush`, `out` |
| coarse | `coarse` above 1 | `dirt_coarse` | `coarse`, `out` |
| lpf | `cutoff` | `dirt_lpf` | `cutoff`, `resonance`, `out` |
| pshift | `psrate` | `dirt_pitch_shift` | `psrate`, `psdisp`, `sustain`, `out` |
| envelope | `attack` or `release` | `dirt_envelope` | `attack`, `hold`, `release`, `curve`, `out` |
| grenvelo | `tilt` | `dirt_grain_envelope` | `sustain`, `tilt`, `plat`, `curve`, `out` |
| tremolo | `tremolorate` | `dirt_tremolo` | `tremolorate`, `tremolodepth`, `out` |
| phaser | `phaserrate` | `dirt_phaser` | `phaserrate`, `phaserdepth`, `out` |

A live sound leaves the cut groups at its `/n_end`, which `reply` takes.
It also leaves them once a later sound starts after its end.

Differences from SuperDirt, for the README:

- An unknown sound sends nothing (fact 15).
- A sound without `s` and `sound` drops. SuperDirt plays the sound `nil`.
- `cutAll` is a number. SuperDirt takes the boolean true.
- The gate gets no `sample` argument.
- An argument that is an array plays one synth, not one per element.
- No `diversion`, `play` or `playInside` function of an event.
- No `smpl` chunk of a WAV file: `metatune` has no effect.
- A sample that is not loaded drops its sound. SuperDirt can read it at
  the first use.
- A sample of more than 2 channels drops its sound, because `start` loads
  the players of 1 and 2 channels. SuperDirt sends a synth whose
  definition scsynth lacks.

### The definitions

`synthdefs::core(channels, delay)` returns the 25 definitions of a channel
count. Their names keep the form of SuperDirt: `dirt_gate2`,
`dirt_sample_1_2`.

| Family | Definitions |
| --- | --- |
| Sample players | `dirt_sample`, `dirt_sample_long`, `dirt_stretchsample`, each for samples of 1 and 2 channels |
| Routing | `dirt_gate`, `dirt_out` |
| Event effects | `dirt_vowel`, `dirt_crush`, `dirt_coarse`, `dirt_hpf`, `dirt_bpf`, `dirt_shape`, `dirt_lpf`, `dirt_envelope`, `dirt_grain_envelope`, `dirt_pitch_shift`, `dirt_tremolo`, `dirt_phaser` |
| Global synths | `dirt_monitor`, `dirt_rms`, `dirt_delay`, `dirt_reverb`, `dirt_leslie` |

- Each definition follows its SuperDirt source line by line, in the order
  in which sclang evaluates it: a receiver before its arguments, the
  arguments from left to right.
- `DirtPan`, `DirtGateCutGroup` and `DirtPause` of `SuperDirtUGens.sc` are
  functions over the builder.
- `dirt_monitor` reads `SuperDirt.postBadValues`, which is false: the
  `post` of `CheckBadValues` is 0.
- Each definition has a public function of `synthdefs`, with the name of
  SuperDirt less its channel counts: `dirt_gate(channels)`,
  `dirt_sample(sample_channels, channels)`. A channel count is a `u32`.
  The function returns the `SynthDef` of scsynth-synthdef, or its
  `BuildError`.
- The module has one file per family: `ugens.rs` for the three
  pseudo-UGens, then `sample.rs`, `routing.rs`, `effects.rs` and
  `global.rs`.
- `tests/synthdefs.rs` compares each definition with the golden one of its
  name, as bytes, for 1, 2 and 4 channels.
- A sample player builds for any channel count of the sample, because
  `DirtPan` takes any count of signals. `core` holds the players of 1 and
  2 channels. SuperDirt builds the same ones, because its
  `maxSampleNumChannels` is 2.
- `midicps` is a private function of the crate, as `sc_midicps` of
  SuperCollider computes it for an f64. The players and the events share
  it.

### The golden files

- `generator/fixtures.scd` writes `tests/fixtures/definitions/`. The files
  `core_1.scsyndef`, `core_2.scsyndef` and `core_4.scsyndef` each hold the
  25 definitions, with the `CombL` delay, from the three core files.
- The files `players_3_4_1.scsyndef`, `_2` and `_4` each hold the 6
  players for samples of 3 and 4 channels. The script builds them with
  `SuperDirt.maxSampleNumChannels` at 4. They are the test of `DirtPan`
  for more than 2 signals.
- With the configuration of `generator/sc3_plugins/`, the script writes
  `switch_delay_1.scsyndef`, `_2` and `_4`. Each holds the one definition
  `dirt_delay`.
- The script also writes `tests/fixtures/vowels.json`. For each of the 5
  registers and the 5 vowels, it holds the frequencies, the amps and the
  rqs. The test of the vowel table compares each number as an f32, the
  type of a control value on the wire.
- `generator/events.scd` writes `tests/fixtures/events.json`. For each
  scenario, it holds the sounds, and the bundles that SuperDirt sent for
  each sound. Every bundle stays apart, with its time. A class of
  `generator/classes/` takes the place of the address of the server, and
  records.
- Both generators fail unless sclang is 3.14.1 and the clones of SuperDirt
  and Vowel are at their commits. Both set `Server.default` to their own
  server object, and boot nothing.

Each scenario of the events runs on a fresh instance of 2 orbits. The
library holds four names: `bd` with 2 mono samples, `sn` with 1 stereo
sample, `long` with 1 mono sample over 16,777,216 frames, and `x_bd` for
`bank`.

1. The start: the nodes of both orbits.
2. Names: `s`, `sound` with `:n`, `sound` with a text after the colon, `n`
   past the count, a negative `n`, a fraction as `n`. Also the stereo
   player, the long player and `bank`.
3. Time: `lag`, `offset` with a speed, and both.
4. Span: negative `speed`, `accelerate`, `begin` and `end`, a `begin` that
   is not 0, `unit` c and s, `loop`, `legato`, `sustain`. Also `dur`
   without `delta`, a sustain under 8 samples, and `timescale` with and
   without `timescalewin`.
5. Pitch: `note`, `octave`, `freq`, `midinote`.
6. Level and place: `gain`, `overgain`, `amp`, an orbit default of `amp`,
   `pan` 0 and 1, `channel`.
7. Each module alone, with and without its optional arguments. The
   scenario covers the 5 vowels and an unknown one, `shape`, `hcutoff`,
   `bandf`, `crush`, and `coarse` 1 and 4. It also covers `cutoff`,
   `psrate`, `attack`, `release`, `tilt`, `tremolorate` and `phaserrate`.
   Then all modules play in one sound.
8. Global synths, as a sequence. Delay parameters, the same again, then
   none. `room` and `size`, then none. `dry`. `lock` with `cps`. The
   leslie parameters, then none.
9. Cut, as a sequence with times: the same group twice, twice at one
   time, a negative group with the same and with another sample, another
   group, another orbit, and `cutAll` from another orbit.
10. Orbits: orbit 1, and an orbit past the count.
11. Channels: the same three sounds on instances of 1 and 4 channels.

The test of the events maps the node ids of both sides by their order of
appearance. It compares each number as an f32, and leaves `sample` out.

### The acceptance test

`tests/scsynth.rs` is ignored by default. It renders a score in
`scsynth -N` and reads the WAV that scsynth writes.

- The score is in time order. It holds the packets of `start`, a
  `/b_allocRead` of a WAV file that the test writes, and the bundles of
  `play`, with `epoch` 0. The test hands the library a `/b_info` of its
  own, because NRT mode drops the replies.
- The test sets `limitertype` to 0, and plays one sound first (fact 17).
- scsynth must exit clean, with no failure line in its output.
- The onset of a sound sits at `round(t * sample rate) + k`, for a time
  tag in the middle of a block, with k measured once. A time tag one
  sample later moves the onset by one sample.
- The output is silent after a cut.
- A second test loads the `SwitchDelay` delay with the plugins of
  sc3-plugins.

## The steps

One commit per step. Step 1 is the first commit of `main`, and the other
steps are on `feat/core`.

| Step | Content | State |
| --- | --- | --- |
| 1 | The scaffold | done |
| 2 | The generator of the definitions, and its golden files | done |
| 3 | `DirtGateCutGroup`, `dirt_gate`, `dirt_out`, and the test of byte equality | done |
| 4 | `DirtPan`, the sample players, and the golden players of 3 and 4 sample channels | done |
| 5 | The 12 event effects, and the vowel table | |
| 6 | `DirtPause`, the 5 global definitions, and `synthdefs::core` | |
| 7 | `Sound` | |
| 8 | `Library`, and the `fs` helper | |
| 9 | The generator of the events, and its golden file | |
| 10 | `Config`, `Dirt::new`, `start`, `stop`, the orbits | |
| 11 | `play` and `reply` | |
| 12 | The acceptance test | |
| 13 | The README, the CHANGELOG, this file | |

A step passes on byte equality for a definition, and on message equality
for a sound. No weaker test takes their place. A step that cannot pass
gets no commit.

## Later versions

Version 0.2, the synths and the routing:

1. The 11 default synths (`imp`, `psin`, `pmsin`, `in`, `in1`,
   `dirt_from`, `inr`, `gabor`, `dirac`, `cyclo`, `debug`), with golden
   files.
2. `Library::add_synth`, and the synth branch of the `sound` module: the
   arguments come from the control names of the definition, as
   `msgFunc.valueEnvir` gives them.
3. The pitch of a synth: `n` as the note, `note`, `octave`, `midinote`,
   `freq`.
4. The routing buses (16 by default), the modules `out_to` and `map_from`,
   and the sound `from`.
5. `Dirt::panic`, as `superpanic`. The change of an output bus.
6. The RMS replies: the switch of `dirt_rms`, and the read of `/rms`.

Version 0.3, the extras behind `sc3-plugins`:

1. `DirtFreqScale`, and the 32 extra synths, with golden files.
2. The 20 extra effects and their modules, in the load order of SuperDirt.
3. The definitions over one datagram: the bytes for a file, and `/d_load`.
4. The wire buffers that an instance needs, as a getter.

Later candidates, each with a decision of its own:

- The control buses of Tidal.
- `/dirt/handshake`.
- Sample data in `/b_setn`.
- Loads at the first use.
- The `smpl` chunk of a WAV file.
- More than 2 sample channels.
- A StrudelDirt profile.
- The expansion of array arguments.

## sc-app3

sc-app3 takes the crate as the submodule `crates/scsynth-dirt`. A `[patch]`
of its workspace points both git dependencies at its own submodules. A
sibling that leaves the version range of the manifest ends the patch, and
cargo-deny of sc-app3 then refuses the git source. The crate then needs a
release with the new range. The pump of sc-app3 does not use the crate
yet. That use is a plan of sc-app3.
