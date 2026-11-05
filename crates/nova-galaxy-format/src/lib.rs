//! The game bible's data format: what the files under `data/` hold and how
//! to read them. See `data/README.md` for the format itself.
//!
//! This crate holds what the engine and the checking tool both need: the
//! ids and value types ([`id`]), one type per kind of content file
//! ([`model`], [`fleet`], [`asset`]), front-matter parsing
//! ([`frontmatter`]), the loader ([`load`]) and the [`source::Source`]
//! port it reads through, the JSON Schemas generated from the types
//! ([`schema`]), the rules for derived data, such as how a ship
//! resolves its base chain ([`fleet::resolve`]), and which media are Git
//! LFS pointers rather than the files ([`media`]).
//!
//! It doesn't hold media decoding (glTF, PNG), Ink parsing, the checks,
//! the map, the contact sheet or the command line. Those belong to the
//! `galaxy` tool, which depends on this crate.
//!
//! # Where it lives
//!
//! The format moved here from `game-bible`'s `tools/galaxy-format`, so
//! the engine (through `nova-data`) and the bible read one definition.
//! `game-bible`'s `galaxy` tool depends on this crate by a git dependency
//! pinned to a `rev`, renamed to `galaxy-format` so its paths stay
//! `galaxy_format::`. A change to the format starts here, and reaches the
//! tool when it moves its pin.
//!
//! That makes the public module paths, the item names and the `fixture`
//! feature (`fixture::galaxy`, `fixture::CC0`, `source::TREE`,
//! `source::check_contract`) this crate's contract with the tool:
//! renaming or removing one breaks its build. A type's doc comment is
//! part of it too, as its JSON Schema's description: the tool fails when
//! the schemas it commits differ from [`schema::schemas`].
//!
//! The crate depends on no other workspace crate, only on crates.io
//! crates, as a git dependency builds it alone, without the workspace's
//! `[patch.crates-io]`.

pub mod asset;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod fleet;
pub mod frontmatter;
pub mod id;
pub mod load;
pub mod media;
pub mod model;
pub mod schema;
pub mod source;
