//! The scroll offset the particles are drawn by, as `_PostAnimateHandler`
//! keeps it, and when they move.
//!
//! **The original.** `_PlayGame` draws a frame (`_SWAnimateSpriteWorld`
//! @0x45e21, whose `_PostDrawCallback` plots the particles
//! @0x38c8d-0x38c9a), then calls `_PostAnimateHandler` once (@0x45e3e).
//! That routine moves the particles (`_UpdateParticlesInWindow` @0x38f07,
//! `_MoveParticles`), then sets the offset from where the player is,
//! `trunc(player - centre)` (`_SetParticleScrollOffset` @0x38f1a-0x38f53).
//! Double time's second pass, `_DoPlayGameWork(0)` (@0x45f20-0x45f31), comes
//! after both. So:
//!
//! - A frame's particles are drawn by the offset the frame before left: in
//!   single time, the player's place a pass old; in double time, its place
//!   after the frame before's first pass.
//! - They move once per drawn frame, after the draw: a pass's new pieces
//!   are drawn at their spawn on its frame. In double time they move once
//!   for two passes, and a second pass's pieces are drawn unmoved on the
//!   next frame.
//! - Nothing else writes the offset (`_InitParticles` @0xc4feb-0xc4ff5
//!   zeroes it once at setup), so it carries across a wrap
//!   (`_WrapPlayerPosition` @0x68097-0x68119), an arrival and a landing.
//!   On a wrap's frame the particles, which are not carried, are drawn in
//!   their place by the offset from before it, and leave the screen on the
//!   next.
//!
//! **The engine.** The flight view replays each step's passes, and a frame
//! stands after each step's first pass ([`ScrollOffset::drawn`]). The
//! post-animate it leaves runs as the next pass is replayed
//! ([`ScrollOffset::begin_pass`]): in single time the next step's first
//! pass, in double time the step's own second pass. That is where the
//! particles move and the offset is held. Both run before the next draw,
//! so no boundary the player sees tells the two orders apart.
//!
//! **Between steps.** The original has no frame between its steps. Flight
//! draws its camera on the player as drawn, interpolated between steps. At
//! the step's start (`shown == at_rest`) the particles are drawn by the
//! held player exactly, the original's plot for the last step:
//! `floor(q) - trunc(P(n-1) - centre)`. Between steps the held player moves
//! with the camera as drawn, so the particles keep their place against the
//! drawn world instead of freezing on screen while it glides. Each
//! particle itself is not interpolated: it jumps by its own velocity at a
//! step. The lag crosses a drawn frame, so it is built, not deferred.

use crate::Point;

/// The player position the particles are drawn by, and the one the last
/// frame left for the post-animate to take (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollOffset {
    /// The player after the last drawn pass, until the post-animate takes
    /// it.
    left: Option<Point>,
    /// The player the offset was last taken from.
    held: Option<Point>,
}

impl ScrollOffset {
    /// A frame stood after a pass that left the player at `player`. Its
    /// post-animate comes at the next pass ([`ScrollOffset::begin_pass`]).
    pub fn drawn(&mut self, player: Point) {
        self.left = Some(player);
    }

    /// Begins a pass. If a frame stood since the last pass, runs its
    /// post-animate: it holds the player that frame left, and says whether
    /// the particles move now, as `_UpdateParticlesInWindow` moves them
    /// just before (`_PostAnimateHandler` @0x38f07-0x38f53).
    pub fn begin_pass(&mut self) -> bool {
        let Some(player) = self.left.take() else {
            return false;
        };
        self.held = Some(player);
        true
    }

    /// The player this frame's particles are drawn by, the camera on
    /// `shown` now and on `at_rest` at the step's start. This is the held
    /// player, moved by the camera's motion since the step began, or
    /// `shown` itself when none is held yet.
    #[must_use]
    pub fn drawn_from(&self, shown: Point, at_rest: Point) -> Point {
        self.held.map_or(shown, |held| {
            Point::new(
                held.x + (shown.x - at_rest.x),
                held.y + (shown.y - at_rest.y),
            )
        })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const REST: Point = Point::new(100.0, 50.0);

    #[test]
    fn with_none_held_the_particles_draw_by_the_camera() {
        let mut offset = ScrollOffset::default();
        assert_eq!(
            offset.drawn_from(Point::new(3.0, 4.0), REST),
            Point::new(3.0, 4.0)
        );
        assert!(!offset.begin_pass(), "no frame stood: nothing moves");
        offset.drawn(Point::new(7.0, 8.0));
        assert_eq!(
            offset.drawn_from(Point::new(3.0, 4.0), REST),
            Point::new(3.0, 4.0),
            "one pass: still the camera"
        );
    }

    #[test]
    fn after_the_next_pass_begins_the_particles_draw_by_the_player_the_last_frame_left() {
        let mut offset = ScrollOffset::default();
        offset.drawn(Point::new(7.0, 8.0));
        assert!(offset.begin_pass(), "the post-animate: they move");
        offset.drawn(Point::new(9.0, 11.0));
        assert_eq!(offset.drawn_from(REST, REST), Point::new(7.0, 8.0));
    }

    #[test]
    fn the_post_animate_runs_once_a_frame() {
        let mut offset = ScrollOffset::default();
        offset.drawn(Point::new(7.0, 8.0));
        assert!(offset.begin_pass());
        assert!(!offset.begin_pass(), "a second pass with no frame between");
        assert_eq!(offset.drawn_from(REST, REST), Point::new(7.0, 8.0));
    }

    #[test]
    fn between_steps_the_held_player_moves_with_the_camera() {
        let mut offset = ScrollOffset::default();
        offset.drawn(Point::new(7.0, 8.0));
        offset.begin_pass();
        let shown = Point::new(REST.x + 1.5, REST.y - 2.25);
        assert_eq!(offset.drawn_from(shown, REST), Point::new(8.5, 5.75));
    }
}
