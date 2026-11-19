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

## State

Version 0.1 is in work. `docs/plan.md` holds its plan: the decisions, the
sources, the facts, the design and the steps.

## License

The license of the crate is the GNU General Public License, version 3 or
any later version (GPL-3.0-or-later). The file `LICENSE` holds the text of
version 3.

This license follows from SuperDirt alone. The synth definitions and the
event logic of the crate translate SuperDirt, which is GPL-2.0-or-later.
The interface takes its names from libdirt of uzu/dirt, which is under the
GPL version 3. The crate copies no code of libdirt.
