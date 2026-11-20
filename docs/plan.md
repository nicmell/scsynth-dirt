# The plan of scsynth-dirt

Version 0.1 is done. As its plan said, its decisions and its facts moved to
the README, the tests, the doc comments, the CHANGELOG and `CLAUDE.md`.
This file keeps only what is still open. The history of git holds the plan
of version 0.1.

## Open from version 0.1

### A fact without an owner

- Without the feature `float_roundtrip` of serde_json, 6 of the 2,511
  floats of `events.json` read 1 unit of the last bit off. The tests read
  the file with the feature, and `tests/golden_events.rs` checks one of
  these floats. No test holds the two counts, because the feature is on for
  every test.

### Not measured

- No test runs scsynth in real time. The acceptance tests measure `start`
  without a wait in NRT mode, and the sources of scsynth give the same
  order in real time. The replies that the crate takes come from the tests
  themselves, because NRT mode sends none.

### Left to the owner

- The release of 0.1.0: the date of its section in the CHANGELOG, and the
  tag `v0.1.0`.

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
