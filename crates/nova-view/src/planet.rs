//! Planets in the draw list: which planet a [`DrawCommand::Planet`]
//! draws ([`PlanetKey`]) and where, how big and at what moment
//! ([`PlanetPose`]).
//!
//! A planet is a lit globe seen from the 2D view's camera, with whatever
//! clouds, rings and atmosphere it was given. What it looks like is
//! registered with the renderer under its key
//! (`nova_render::Renderer::add_planet`), as a model's mesh is under a
//! [`ModelKey`](crate::ModelKey): the command names the planet and places
//! it, and carries no look of its own.
//!
//! [`DrawCommand::Planet`]: crate::DrawCommand::Planet

use crate::geometry::Point;

/// Which planet a [`DrawCommand::Planet`](crate::DrawCommand::Planet)
/// draws: a key the renderer was given a planet for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanetKey(pub u32);

/// Where a planet is drawn, how big, and when.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetPose {
    /// Where the planet's centre goes, in logical units, as a sprite's
    /// centre is placed.
    pub center: Point,
    /// The globe's radius in logical units (its rings and atmosphere
    /// reach beyond it). One that is not a positive, finite number draws
    /// nothing.
    pub radius: f32,
    /// The game time, in seconds, that turns the globe and its clouds: the
    /// same time always draws the same planet.
    pub time: f64,
}
