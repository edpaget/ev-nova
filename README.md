# ev-nova

Rust libraries and tools for reading the data files of EV Nova.

[![CI](https://github.com/edpaget/ev-nova/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/edpaget/ev-nova/actions/workflows/ci.yml?query=branch%3Amain)
[![License: GPL v3](https://img.shields.io/badge/license-GPLv3-blue.svg)](LICENSE)

## What it is

EV Nova is a space trading and combat game released by Ambrosia Software in
2002. This project reads the game's resource files, decodes them into typed
records, graphics and sounds, and exports them as JSON, PNG and WAV. An engine
is planned; the `nova` crate is currently a stub.

## Game data

EV Nova's data files are copyrighted and are not included in this repository.
To use the tools or run the stock-data tests, supply your own copy of the
game's `Nova Files` directory.

## Building and testing

The toolchain (Rust 1.98.1) and the cargo tools are pinned in `mise.toml`.
Install [mise](https://mise.jdx.dev), then from the repository root:

```sh
mise install
mise run ci     # rustfmt check, clippy with -D warnings, nextest and doctests
mise run test   # nextest and doctests only
```

Plain `cargo build` and `cargo test` also work in a shell where mise is
activated, or with Rust 1.98.1 installed.

## Using your own data

Dump the game data to JSON, PNG and WAV:

```sh
cargo run -p nova-dump -- "<path>/Nova Files" <out-dir> [--plugins <dir>]
```

Pass the `Nova Files` directory itself, not the folder that contains it. The
output layout, options and exit codes are documented in
[`crates/nova-dump/src/lib.rs`](crates/nova-dump/src/lib.rs), also available
with `cargo doc -p nova-dump --open`.

The stock-data tests in `nova-rsrc`, `nova-data` and `nova-dump` read the
`Nova Files` path from the `NOVA_DATA` environment variable. Without it, they
skip and pass.

```sh
NOVA_DATA="<path>/Nova Files" mise run test
```

## Crate layout

| Crate | Role |
| --- | --- |
| [`nova-rsrc`](crates/nova-rsrc) | Reader for classic Mac OS resource forks (`.ndat` / `.rsrc` files). Format only; no EV Nova knowledge. |
| [`nova-data`](crates/nova-data) | Typed EV Nova records, graphics and sound decoders, and the layered game data store (base data plus plug-ins). |
| [`nova-dump`](crates/nova-dump) | Command-line tool that writes game data out as JSON, PNG and WAV files. |
| [`nova`](crates/nova) | Engine and player. Currently a stub binary. |

API documentation: `cargo doc --workspace --open`.

## Development

See [CLAUDE.md](CLAUDE.md) for the test-driven workflow, the ports-and-adapters
structure, and commit conventions.

## License

Licensed under the GNU General Public License v3.0 or later
(GPL-3.0-or-later). See [LICENSE](LICENSE). The license covers the code in this
repository, not the EV Nova game data.
