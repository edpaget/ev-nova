//! The game bible's data format: what the files under `data/` hold and how
//! to read them. See `data/README.md` for the format itself.
//!
//! This crate holds what the engine and the checking tool both need: the
//! ids and value types ([`id`]), one type per kind of content file
//! ([`model`], [`fleet`], [`asset`]), front-matter parsing
//! ([`frontmatter`]), the loader ([`load`]) and the [`source::Source`]
//! port it reads through, the JSON Schemas generated from the types
//! ([`schema`]), and the rules for derived data, such as how a ship
//! resolves its base chain ([`fleet::resolve`]).
//!
//! It doesn't hold media decoding (glTF, PNG), Ink parsing, the checks,
//! the map, the contact sheet or the command line. Those belong to the
//! `galaxy` tool, which depends on this crate.

pub mod asset;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod fleet;
pub mod frontmatter;
pub mod id;
pub mod load;
pub mod model;
pub mod schema;
pub mod source;
