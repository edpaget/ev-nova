//! The fight's tests of what a ship's shots leave from as its hull turns
//! (rdm engine-3d phase 3): by the engine from the exit turned to the
//! frame it was drawn on, by
//! [`RuleKey::ContinuousHeadings`](crate::RuleKey::ContinuousHeadings)'
//! other reading from the exit turned to the drawn heading itself.

use super::*;
use crate::flight::facing;

/// Where a gun 20 pixels ahead of the nose leaves from, the ship last
/// drawn facing `heading`, its hull turning as `source` says.
fn start_at(heading: f32, source: RuleSource) -> Vec2 {
    let mut combat = Combat::default();
    combat.set_continuous_headings(source);
    let ship = Ship::at(1, 0.0, 0.0)
        .armed(gun())
        .facing(heading)
        .drawn_at(heading)
        .of(exits_at(0, [(0, 20); 4]));
    fired_start(&mut combat, &mut [ship])
}

/// The distinct points among `points`.
fn distinct(points: &[Vec2]) -> usize {
    let mut seen: Vec<Vec2> = Vec::new();
    for &point in points {
        if !seen.iter().any(|&other| close_to(other, point)) {
            seen.push(point);
        }
    }
    seen.len()
}

#[test]
fn under_continuous_headings_a_shot_leaves_from_the_exit_turned_to_the_drawn_heading() {
    // 1, 3 and 7 are all on a 36-frame hull's frame 0.
    let headings = [1.0, 3.0, 7.0];
    let by = |source| headings.map(|h| start_at(h, source));
    let continuous = by(RuleSource::Bible);
    assert_eq!(distinct(&continuous), 3, "{continuous:?}");
    for (heading, at) in headings.iter().zip(continuous) {
        let want = facing(*heading) * 20.0;
        assert!(close_to(at, want), "{heading}: {at:?} not {want:?}");
    }
    let engine = by(RuleSource::Engine);
    assert_eq!(distinct(&engine), 1, "one frame, one point: {engine:?}");
    assert!(close_to(engine[0], Vec2::new(0.0, -20.0)), "{engine:?}");
}
