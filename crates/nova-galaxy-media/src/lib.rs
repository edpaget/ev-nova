//! The new game's media, read as data: a glTF model's triangles,
//! materials and footprint ([`model`](mod@model)), how a model sits on a
//! ship (`placement`), and the model seen from above as painted, shaded
//! triangles, or a ship's size box when it has no model (`outline`).
//!
//! The engine draws these silhouettes where the new game has no sprite
//! (`nova-data` builds them at load), and game-bible's `galaxy` tool draws
//! the same silhouettes on its contact sheet: the code is a port of its
//! `media.rs` `model`, `placement.rs` and `sheet.rs` (`top_down`,
//! `draw_model`, `draw_box`, `painted`), without the SVG and the
//! hardpoints. `galaxy` may switch to this crate once it reads the format
//! from `nova-galaxy-format`.
//!
//! # What it holds, and why here
//!
//! The format crate keeps media decoding out, so the format's
//! schema-only users don't link `gltf`; and `nova-data` would pull the
//! whole stock data crate into `galaxy`. So the decoding sits here, on
//! the format alone: the crate depends on no other workspace crate, only
//! on `nova-galaxy-format` and crates.io crates, so a git dependency
//! builds it alone, as it builds the format. Only this crate decodes
//! models (`deny.toml` bans `gltf` everywhere else).
//!
//! Nothing here touches the filesystem: media are read through the
//! format's [`Source`](nova_galaxy_format::source::Source) port, and the
//! `fixture` feature's builders make test models in code, so no art
//! enters the repo.

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod model;
pub mod placement;

pub use model::{Bounds, Face, Model, NoModel, model, read_model};
pub use placement::Placement;
