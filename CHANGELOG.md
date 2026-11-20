# Changelog

## 0.1.0 (unreleased)

The first release. It plays the sample sounds of SuperDirt on scsynth, with
no I/O: the 25 core definitions of SuperDirt, the samples, the modules of
its core and its global effects. It takes scsynth-osc 0.2.0 and
scsynth-synthdef 0.15.0 by git tag.

### Added

- `Dirt`: an instance of SuperDirt on scsynth. `start` loads the 25
  definitions and makes the nodes of the orbits, and `stop` frees them.
  `play` returns the bundles of a sound, and `reply` takes the `/n_end` of
  its group. See [The instance](README.md#the-instance) and
  [Play](README.md#play).
- `Config` and `ConfigError`: the structure of an instance, on the node ids
  and the buses that the host gives, and the refusals of `Dirt::new`.
- `Orbit`: the nodes, the buses and the defaults of an orbit, as `DirtOrbit`
  of SuperDirt sets them.
- `Sound`, `Value` and `SoundError`: a sound of open parameters, with a
  setter for each of the 32 fields of libdirt. `Sound::from_args` reads the
  arguments of `/dirt/play`. See [The sounds](README.md#the-sounds).
- `Dropped`: the reasons of a sound that does not play.
- `Library`, `Sample`, `SampleInfo` and `LibraryError`: the samples on the
  buffers of scsynth, with their loads and their replies. See
  [The library](README.md#the-library).
- The module `synthdefs`: the 25 core definitions of SuperDirt at commit
  `a3046d1`, built with scsynth-synthdef, and the players for any count of
  sample channels. `Delay` picks the delay line of `CombL` or of
  `SwitchDelay`. See [The definitions](README.md#the-definitions).
- `Register`: the voice register of the vowels of the Vowel quark.
- The feature `fs`, with `fs::list`: a folder of samples as SuperDirt lists
  it.
- The feature `serde`, for `Config`, `Sound`, `Value`, `Delay` and
  `Register`.
- The feature `sc3-plugins`, for the delay of `SwitchDelay`.
- The golden files of sclang 3.14.1 and SuperDirt, with their generators:
  the definitions, the vowels, and the bundles of 132 sounds. The
  acceptance tests render an instance in scsynth 3.14.1. See
  [Tests](README.md#tests).

### Known issues

- See [Differences from SuperDirt](README.md#differences-from-superdirt)
  for what the crate does in another way, and [Left out](README.md#left-out)
  for what version 0.1 leaves out. See [Traps](README.md#traps) for what it
  does not rule out.
