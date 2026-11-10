//! The new game's media, read as data: a glTF model's triangles,
//! materials and footprint ([`model`](mod@model)), how a model sits on a
//! ship ([`placement`]), and the model seen from above as painted, shaded
//! triangles, or a ship's size box when it has no model ([`outline`]),
//! and which livery slot paints a material ([`paint`](mod@paint)), the rule
//! the silhouettes and the renderer's 3D models share.
//!
//! The engine draws these silhouettes where the new game has no sprite
//! (`nova-data` builds them at load), and game-bible's `galaxy` tool draws
//! the same silhouettes on its contact sheet: the code is a port of its
//! `media.rs` `model`, `placement.rs` and `sheet.rs` (`top_down`,
//! `draw_model`, `draw_box`, `painted`), without the SVG and the contact
//! sheet's hardpoint marks. `galaxy` may switch to this crate once it
//! reads the format from `nova-galaxy-format`.
//!
//! Where a hardpoint's outfit stands on its hull, and how an outfit's
//! model sits on its mount, is [`mount`](mod@mount): a ray cast straight
//! down onto the hull's model, placed as it is drawn, and the outfit's
//! lift, pivot and muzzle. The loader (`nova-data`) keeps the result for
//! the view and the simulation.
//!
//! For drawing models in 3D (the renderer's model adapter,
//! `nova_render::GalaxyModels`), it also reads a model whole
//! ([`scene`]: each primitive each node draws, with normals, texture
//! coordinates and materials, and the images as stored), places it in 3D
//! ship space by its front and length
//! ([`Placement::ship_point_3d`]), and works out which art budget
//! covers it and each limit it breaks ([`budget`], a port of `galaxy`'s
//! `check/art.rs` budget rule, for models and textures).
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

pub mod budget;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod model;
pub mod mount;
pub mod outline;
pub mod paint;
pub mod placement;
pub mod scene;

pub use model::{Bounds, Face, Model, NoModel, model, read_model};
pub use outline::{BARE, Outline, Triangle, painted, rgb, silhouette, size_box, top_down};
pub use paint::{Slot, slot_of};
pub use placement::Placement;
pub use scene::{Scene, read_asset_scene, read_scene};
