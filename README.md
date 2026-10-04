# ev-nova

A Rust reimplementation of EV Nova, with libraries and tools for its data files.

[![CI](https://github.com/edpaget/ev-nova/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/edpaget/ev-nova/actions/workflows/ci.yml?query=branch%3Amain)
[![License: GPL v3](https://img.shields.io/badge/license-GPLv3-blue.svg)](LICENSE)

## What it is

EV Nova is a space trading and combat game released by Ambrosia Software in
2002. This project reimplements it in Rust: libraries for reading the game's
data files, and an engine and player built on them. It is early work in
progress.

## Game data

EV Nova's data files are copyrighted and are not included in this repository.
To use the tools or run the stock-data tests, supply your own copy of the
game's `Nova Files` directory.

## Building and testing

The Rust toolchain and the cargo tools are pinned in `mise.toml`.
Install [mise](https://mise.jdx.dev), then from the repository root:

```sh
mise install
mise run ci     # rustfmt check, clippy with -D warnings, nextest and doctests
mise run test   # nextest and doctests only
```

Plain `cargo build` and `cargo test` also work in a shell where mise is
activated, or with the toolchain pinned in `mise.toml` installed.

## Nightly builds

The [`nightly` prerelease](https://github.com/edpaget/ev-nova/releases/tag/nightly)
holds `nova` and `nova-dump` built from the latest commit on `main`:

- `ev-nova-x86_64-unknown-linux-gnu.tar.gz`
- `ev-nova-universal-apple-darwin.tar.gz` (Apple silicon and Intel)
- `ev-nova-x86_64-pc-windows-msvc.zip`

These are unsigned tester builds, and they contain no game data. Each archive
has a `README.txt` on running them. On macOS, Gatekeeper blocks unsigned
binaries; in the extracted folder, run
`xattr -d com.apple.quarantine nova nova-dump`.

## Using your own data

Dump the game data to JSON, PNG and WAV:

```sh
cargo run -p nova-dump -- "<path>/Nova Files" <out-dir> [--plugins <dir>]
```

With a prebuilt binary from the [nightly builds](#nightly-builds):

```sh
nova-dump "<path>/Nova Files" <out-dir> [--plugins <dir>]
```

Pass the `Nova Files` directory itself, not the folder that contains it. The
output layout, options and exit codes are documented in
[`crates/nova-dump/src/lib.rs`](crates/nova-dump/src/lib.rs), also available
with `cargo doc -p nova-dump --open`.

Stock-data tests read the `Nova Files` path from the `NOVA_DATA` environment
variable. Without it, they skip and pass.

```sh
NOVA_DATA="<path>/Nova Files" mise run test
```

## Finding your way around

Each crate lives in [`crates/<name>`](crates). Its `Cargo.toml` `description`
and its crate-level docs say what it is for; `cargo doc --workspace --open`
renders them all.

## Development

See [CLAUDE.md](CLAUDE.md) for the test-driven workflow, the ports-and-adapters
structure, and commit conventions.

## License

Licensed under the GNU General Public License v3.0 or later
(GPL-3.0-or-later). See [LICENSE](LICENSE). The license covers the code in this
repository, not the EV Nova game data.

### Fonts

The font in [`crates/nova-render/fonts/`](crates/nova-render/fonts), Noto
Sans Regular, is licensed separately under the SIL Open Font License 1.1
(OFL-1.1); see [its README](crates/nova-render/fonts/README.md) and
[`OFL.txt`](crates/nova-render/fonts/OFL.txt). The game's own Charcoal font
is read from your copy of the game data at run time and is never included.
