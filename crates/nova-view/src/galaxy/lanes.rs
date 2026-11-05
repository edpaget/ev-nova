//! How the in-flight maps draw a link in its gate lane's look, by
//! `RuleKey::MapLaneStyle`'s other reading (`nova_sim`'s
//! [`map_lanes`](nova_sim::map_lanes) works the looks out): each way in
//! its stability's colour and dash, in two halves where the ways differ,
//! and an arrowhead on a one-way link ([`strokes`]).
//!
//! The original has none of this: `_DrawMap` draws every link in one grey
//! (@0xe7a8-0xed31). The looks are game-bible's galaxy map's
//! (`tools/galaxy/src/map.rs`): stable [`STABLE`], solid; unstable
//! [`UNSTABLE`], dashed [`UNSTABLE_DASH`]; dead [`DEAD`], dotted
//! [`DEAD_DASH`]; a one-way link ending in an arrow. Its colours are for a
//! light page, drawn here on Nova's black map, one constant each. A way no
//! gate serves is drawn in the link's own grey.
//!
//! The halves meet at the link's midpoint, truncated to a whole pixel:
//! the half nearer each system is drawn in its way out, so each player sees
//! the way they can leave by. The arrowhead is the hypergate arrow's
//! ([`arrow_head`]), two strokes back from the far end, in the way out's
//! colour.

use nova_sim::Vec2;
use nova_sim::catalog::GateStability;
use nova_sim::galaxy_map::arrow_head;
use nova_sim::map_lanes::{Back, LaneLook};

use crate::Color;

/// A stable way: game-bible's `#4a7a96`, solid.
pub const STABLE: Color = Color::rgba(0x4a, 0x7a, 0x96, 255);
/// An unstable way: game-bible's `#d9822b`, dashed.
pub const UNSTABLE: Color = Color::rgba(0xd9, 0x82, 0x2b, 255);
/// A dead way: game-bible's `#9a9a9a`, dotted.
pub const DEAD: Color = Color::rgba(0x9a, 0x9a, 0x9a, 255);
/// An unstable way's dash, in port pixels: 8 on, 5 off.
pub const UNSTABLE_DASH: (f32, f32) = (8.0, 5.0);
/// A dead way's dots, in port pixels: 2 on, 5 off.
pub const DEAD_DASH: (f32, f32) = (2.0, 5.0);

/// One pen-1 line of a link's look, in whole port pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaneStroke {
    /// Where it starts.
    pub from: (i32, i32),
    /// Where it ends.
    pub to: (i32, i32),
    /// Its colour.
    pub color: Color,
}

/// The colour a way of `stability` is drawn in, `grey` for a way no gate
/// serves.
#[must_use]
pub fn color_of(stability: Option<GateStability>, grey: Color) -> Color {
    match stability {
        Some(GateStability::Stable) => STABLE,
        Some(GateStability::Unstable) => UNSTABLE,
        Some(GateStability::Dead) => DEAD,
        None => grey,
    }
}

/// The dash a way of `stability` is drawn with, `None` for a solid line.
#[must_use]
pub fn dash_of(stability: Option<GateStability>) -> Option<(f32, f32)> {
    match stability {
        Some(GateStability::Unstable) => Some(UNSTABLE_DASH),
        Some(GateStability::Dead) => Some(DEAD_DASH),
        Some(GateStability::Stable) | None => None,
    }
}

/// The strokes that draw `look`'s link from the pixel `from` to the pixel
/// `to`, `grey` the link's own colour (see the module docs).
#[must_use]
pub fn strokes(look: &LaneLook, from: (i32, i32), to: (i32, i32), grey: Color) -> Vec<LaneStroke> {
    let mut strokes = Vec::new();
    match look.back {
        Back::Way(back) if back != look.out => {
            let mid = (i32::midpoint(from.0, to.0), i32::midpoint(from.1, to.1));
            way(&mut strokes, from, mid, look.out, grey);
            way(&mut strokes, mid, to, back, grey);
        }
        Back::Way(_) => way(&mut strokes, from, to, look.out, grey),
        Back::OneWay => {
            way(&mut strokes, from, to, look.out, grey);
            let color = color_of(look.out, grey);
            let point = |(x, y): (i32, i32)| {
                #[allow(clippy::cast_precision_loss)] // Screen pixels.
                Vec2::new(x as f32, y as f32)
            };
            for end in arrow_head(point(to), point(from)) {
                strokes.push(LaneStroke {
                    from: to,
                    to: truncated(end),
                    color,
                });
            }
        }
    }
    strokes
}

/// Pushes the strokes of one way of `stability` from `from` to `to`: one
/// line, or its dash's pieces along it, each truncated to whole pixels.
fn way(
    strokes: &mut Vec<LaneStroke>,
    from: (i32, i32),
    to: (i32, i32),
    stability: Option<GateStability>,
    grey: Color,
) {
    let color = color_of(stability, grey);
    #[allow(clippy::cast_precision_loss)] // Screen pixels.
    let (start, end) = (
        Vec2::new(from.0 as f32, from.1 as f32),
        Vec2::new(to.0 as f32, to.1 as f32),
    );
    let length = (end - start).length();
    let Some((on, off)) = dash_of(stability).filter(|_| length > 0.0) else {
        strokes.push(LaneStroke { from, to, color });
        return;
    };
    let along = (end - start) * length.recip();
    let mut at = 0.0;
    while at < length {
        let stop = (at + on).min(length);
        strokes.push(LaneStroke {
            from: truncated(start + along * at),
            to: truncated(start + along * stop),
            color,
        });
        at += on + off;
    }
}

/// `at` truncated toward zero to a whole pixel.
#[allow(clippy::cast_possible_truncation)] // Screen pixels.
fn truncated(at: Vec2) -> (i32, i32) {
    (at.x.trunc() as i32, at.y.trunc() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use GateStability::{Dead, Stable, Unstable};
    use nova_sim::SystemId;

    const GREY: Color = Color::rgba(1, 2, 3, 255);

    fn look(out: Option<GateStability>, back: Back) -> LaneLook {
        LaneLook {
            from: SystemId(128),
            to: SystemId(129),
            out,
            back,
        }
    }

    fn colors(strokes: &[LaneStroke]) -> Vec<Color> {
        let mut seen: Vec<Color> = Vec::new();
        for stroke in strokes {
            if seen.last() != Some(&stroke.color) {
                seen.push(stroke.color);
            }
        }
        seen
    }

    #[test]
    fn each_stability_has_its_colour_and_a_way_no_gate_serves_the_grey() {
        assert_eq!(color_of(Some(Stable), GREY), STABLE);
        assert_eq!(color_of(Some(Unstable), GREY), UNSTABLE);
        assert_eq!(color_of(Some(Dead), GREY), DEAD);
        assert_eq!(color_of(None, GREY), GREY);
    }

    #[test]
    fn a_stable_or_grey_way_is_one_solid_line() {
        for out in [Some(Stable), None] {
            let drawn = strokes(&look(out, Back::Way(out)), (0, 0), (100, 0), GREY);
            assert_eq!(
                drawn,
                [LaneStroke {
                    from: (0, 0),
                    to: (100, 0),
                    color: color_of(out, GREY),
                }]
            );
        }
    }

    #[test]
    fn an_unstable_way_is_dashed_8_on_5_off() {
        let drawn = strokes(
            &look(Some(Unstable), Back::Way(Some(Unstable))),
            (0, 0),
            (30, 0),
            GREY,
        );
        let pieces: Vec<_> = drawn
            .iter()
            .map(|stroke| (stroke.from, stroke.to))
            .collect();
        assert_eq!(
            pieces,
            [((0, 0), (8, 0)), ((13, 0), (21, 0)), ((26, 0), (30, 0))]
        );
        assert!(drawn.iter().all(|stroke| stroke.color == UNSTABLE));
    }

    #[test]
    fn a_dead_way_is_dotted_2_on_5_off() {
        let drawn = strokes(
            &look(Some(Dead), Back::Way(Some(Dead))),
            (0, 0),
            (0, 15),
            GREY,
        );
        let pieces: Vec<_> = drawn
            .iter()
            .map(|stroke| (stroke.from, stroke.to))
            .collect();
        assert_eq!(
            pieces,
            [((0, 0), (0, 2)), ((0, 7), (0, 9)), ((0, 14), (0, 15))]
        );
        assert!(drawn.iter().all(|stroke| stroke.color == DEAD));
    }

    #[test]
    fn a_link_whose_ways_differ_is_drawn_in_two_halves_each_its_ways_look() {
        let drawn = strokes(
            &look(Some(Stable), Back::Way(Some(Dead))),
            (0, 0),
            (100, 0),
            GREY,
        );
        assert_eq!(colors(&drawn), [STABLE, DEAD]);
        assert_eq!(
            drawn[0],
            LaneStroke {
                from: (0, 0),
                to: (50, 0),
                color: STABLE,
            }
        );
        assert_eq!(drawn[1].from, (50, 0), "the second half from the midpoint");
        assert_eq!(drawn.last().map(|stroke| stroke.to), Some((100, 0)));
        let way_back_grey = strokes(&look(Some(Stable), Back::Way(None)), (0, 0), (100, 0), GREY);
        assert_eq!(colors(&way_back_grey), [STABLE, GREY]);
    }

    #[test]
    fn a_one_way_link_has_two_head_strokes_at_its_far_end_and_none_at_its_near_end() {
        let drawn = strokes(&look(Some(Unstable), Back::OneWay), (0, 0), (100, 0), GREY);
        let heads: Vec<_> = drawn
            .iter()
            .filter(|stroke| stroke.from == (100, 0))
            .collect();
        assert_eq!(heads.len(), 2);
        let ends = arrow_head(Vec2::new(100.0, 0.0), Vec2::new(0.0, 0.0)).map(truncated);
        assert_eq!(
            heads.iter().map(|stroke| stroke.to).collect::<Vec<_>>(),
            ends
        );
        assert!(heads.iter().all(|stroke| stroke.color == UNSTABLE));
        assert!(
            drawn.iter().all(|stroke| stroke.to != (0, 0)),
            "nothing ends at the near end"
        );
        assert!(drawn.iter().all(|stroke| stroke.color == UNSTABLE));
    }
}
