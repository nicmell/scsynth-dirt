# scsynth-dirt

A Rust crate that plays Dirt events on scsynth, the synthesis server of
SuperCollider. Its interface follows libdirt of uzu/dirt, and its sound is
the sound of SuperDirt. It does no I/O: a call returns packets of
scsynth-osc. Its synth definitions are the definitions of SuperDirt,
rebuilt with scsynth-synthdef. This file is the operating manual.

The README is the reference of the crate. [docs/plan.md](docs/plan.md)
holds what is still open after version 0.1: a fact without an owner, what
no test measures, the release, the later versions and the note on sc-app3.
Read both before any work.

## Ground rules

The sources are on this machine:

| Source | Where | At |
| --- | --- | --- |
| SuperDirt | `~/Projects/SuperDirt` | `a3046d1` |
| The Vowel quark | `~/Projects/Vowel` | `ab59caa` |
| uzu/dirt | `~/Projects/dirt` | `123b399` |
| SuperCollider | `~/Projects/supercollider` | `Version-3.14.1` |
| sclang 3.14.1 | `~/.bin/sclang` | |
| scsynth 3.14.1 | `/Applications/SuperCollider.app/Contents/Resources/scsynth` | |
| sc3-plugins | `~/Library/Application Support/SuperCollider/Extensions/SC3plugins` | 3.14.0 |

- A fact comes from a source of the table, or from a measurement with
  sclang or scsynth 3.14.1.
- Never touch the scsynth on UDP 57110. A generator in `generator/` sets
  `Server.default` to its own server object, and boots nothing. A test runs
  its own scsynth in NRT mode, with the scsynth of the SuperCollider
  bundle. `~/.bin/scsynth` needs a running jackd, also for `-N`.
- Make no change to the sibling crates scsynth-osc and scsynth-synthdef
  from this repository.
- A step passes on byte equality for a definition, and on message equality
  for a sound. No weaker test takes their place.
- Every dependency from crates.io must be at least 7 days old. The sibling
  crates come by git tag, so the rule does not cover them.

## Commands

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --all-features
RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps --all-features
cargo clippy --all-features --target wasm32-unknown-unknown -- -D warnings
cargo +1.85.0 test --all-features    # the MSRV
cargo deny check
```

CI (`.github/workflows/ci.yml`) runs them with `--locked`. CI has no
`cargo package` step, because the sibling crates are git dependencies.
Inside the sc-app3 workspace, cargo uses the lock of sc-app3, so a
`--locked` run needs a standalone clone.

The acceptance tests of `tests/scsynth.rs` are ignored by default, because
they run the scsynth of the bundle in NRT mode. With the feature
`sc3-plugins`, the test of `SwitchDelay` also needs the plugins of
sc3-plugins. CI has no scsynth, so it does not run them.

```bash
cargo test --all-features -- --ignored
```

`SCSYNTH` names another scsynth, and `SCSYNTH_PLUGINS` another folder of
its plugins. `SCSYNTH_SC3_PLUGINS` names another folder of the plugins of
sc3-plugins.

The generators write the golden files of `tests/fixtures`. The README says
what each run writes, under Tests. Run them from the root of the
repository, with absolute paths, because `~/.bin/sclang` changes the
directory. If no terminal reads the output of sclang, sclang hides it, so a
run under `script` shows it.

```bash
sclang -l "$PWD/generator/sclang_conf.yaml" "$PWD/generator/fixtures.scd"
sclang -l "$PWD/generator/sc3_plugins/sclang_conf.yaml" "$PWD/generator/fixtures.scd"
sclang -l "$PWD/generator/sclang_conf.yaml" "$PWD/generator/events.scd"
```

## Conventions

- Edition 2024, MSRV 1.85, clippy pedantic, no unsafe code, docs on every
  public item. CI runs clippy with `-D warnings`.
- A comment states a fact that the code cannot: a measured number, a trap,
  a constraint. Design rationale goes in the README or in `docs/`.
- One fact, one owner. The owner is the test that breaks with the fact.
  Without a test, the owner is the file where the edit goes.
- Work goes on a branch, with one commit per step of the plan. Every commit
  leaves fmt, clippy, the tests and the docs green. A branch reaches `main`
  through a GitHub PR.
