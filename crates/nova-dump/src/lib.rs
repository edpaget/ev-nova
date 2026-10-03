//! `nova-dump`: writes EV Nova game data out as JSON, PNG and WAV files.
//!
//! The dump makes the data browsable, and is an end-to-end check that every
//! decoder works on real data. It doubles as a compatibility report for
//! plug-ins.
//!
//! # Usage
//!
//! ```text
//! nova-dump <DATA_DIR> <OUT_DIR> [--plugins <DIR>] [--verbose|-v]
//! nova-dump --help|-h
//! ```
//!
//! `DATA_DIR` is the `Nova Files` directory; `--plugins` adds a plug-ins
//! tree, layered over it as [`nova_data::store`] describes. `OUT_DIR` is
//! created if it is missing and must otherwise be empty: the tool never
//! deletes or replaces a file, so stale files from an earlier run cannot
//! mix with a new one. `--` ends the options.
//!
//! # Output
//!
//! ```text
//! OUT_DIR/json/<type>.json          one per registered record type present
//! OUT_DIR/png/PICT/<id>[ <name>].png
//! OUT_DIR/png/cicn/<id>[ <name>].png
//! OUT_DIR/png/ppat/<id>[ <name>].png
//! OUT_DIR/png/rlëD/<id>[ <name>].png  a composed sprite sheet
//! OUT_DIR/wav/<id>[ <name>].wav     one per snd
//! ```
//!
//! Type directories and JSON files are named by [`names::type_component`]
//! and files by [`names::file_stem`]: the resource ID, then its name made
//! safe for a file name.
//!
//! - **JSON**: a pretty-printed array, by ascending ID, of
//!   `{"id", "name", "source", "warning", "record"}`: `name` is `null` when
//!   the resource has none, `source` is the file that won the resource,
//!   `warning` appears only when decoding raised one, and `record` holds
//!   the typed fields. A record that fails to decode is left out and
//!   reported. Types that are neither registered records nor pictures,
//!   sheets or sounds are not exported; the summary lists them.
//! - **PNG**: 8-bit RGBA with straight alpha, compressed for speed.
//! - **Sprite sheets**: every `rlëD` is composed into one image, laid out
//!   by the `shän` or `spïn` that uses it ([`layouts::sheet_layouts`]), or
//!   with the default layout when none does.
//! - **WAV**: 16-bit linear PCM at the sound's rate rounded to the nearest
//!   hertz ([`wav`]); loop points and the base note are not written.
//!
//! # Summary and failures
//!
//! The summary goes to standard output ([`report`]): counts of each kind of
//! file, then one line per failure, naming the resource's type, ID, name
//! and source file, or the file that could not be loaded. Decode failures
//! and unloadable files never stop the run. `--verbose` also lists the
//! entries the store skipped on purpose and, under each failure, the field
//! path (for records), the failing offset and a hex window around it
//! ([`hex::window`]).
//!
//! # Exit codes
//!
//! - **0**: the dump finished with no failures (the last line of the
//!   summary is `failures: 0`). `--help` also exits 0.
//! - **1**: the dump finished, but failures are listed; everything else
//!   was written.
//! - **2**: nothing useful was done: a usage error, a data or plug-ins
//!   directory that cannot be listed, a data directory with no game data
//!   in it, an output directory that cannot be created or read or is not
//!   empty, or a file that cannot be written. The reason goes to standard
//!   error.
//!
//! A data directory has no game data when not one file directly inside it
//! is a resource-file candidate: typically the folder *containing*
//! `Nova Files` was passed, whose `Nova Files` sub-folder is skipped. The
//! run then stops before the output directory is created, and the message
//! counts the entries the data directory skipped (`--verbose` lists them).
//! Plug-ins alone are not game data. A data file that is found but fails
//! to load is different: it is a listed failure, exit 1.
//!
//! # Structure
//!
//! The core ([`app`], [`export`], [`report`] and the pure helpers) reaches
//! the outside world only through the [`ports`]; [`fs`] holds the std
//! adapters, which the binary wires in.

pub mod app;
pub mod cli;
pub mod export;
pub mod fs;
pub mod hex;
pub mod layouts;
pub mod names;
pub mod png;
pub mod ports;
pub mod report;
pub mod wav;

#[cfg(test)]
mod testutil;
