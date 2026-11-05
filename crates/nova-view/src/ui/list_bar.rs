//! The vertical scroll bar the List Manager gives a list whose rows don't
//! fit: the mission computer's and the mission info screen's.
//!
//! The game builds its lists with `CreateCustomList` and a vertical bar
//! (`_SetupMissionInfoList` @0x9e013, `_SetupMissionList` @0xa086c). The
//! bar is the system's, run by Carbon's List and Control Managers, which
//! can't be read. The best evidence is their classic source, Apple's ROM
//! (`Toolbox/ListMgr/ListMgrPACK.a` and `Toolbox/ControlMgr/` in
//! <https://git.applefritter.com/Macintosh-SW/mac-rom>), and this module
//! follows it:
//!
//! - **Its rect** lies outside the list's view, on its right, 16 wide and
//!   one pixel past the view at the top and bottom (`GetScSize`,
//!   `ListMgrPACK.a` line 1828).
//! - **Its parts** (`HitSBar`, `ScrollBarCDEF.a` line 965): the up arrow
//!   where the point is no more than 16 below the bar's top, the down
//!   arrow where it is no more than 16 above its bottom, the thumb, and
//!   otherwise the page above or below the thumb's midpoint. Each is
//!   tested with `PtInRect`, so the bar's right and bottom edges are out.
//! - **The thumb** is the classic `CDEF` 1's, 16 high and inset a pixel
//!   at each side, placed by scaling the value over the track between the
//!   arrows less its own height, rounded to the nearest (`CalcIBox`,
//!   line 883; `RoundOff`, line 1211). A drag moves its outline, and the
//!   value is set from where the outline is dropped (`MoveSBar`,
//!   line 1141), unless the pointer is released out of the slop rect,
//!   which leaves it (`ThumbSBar`, line 1241). The Appearance Manager's
//!   proportional thumb and live drag are the OS theme's, not replicable.
//! - **A held arrow or page** steps at once and then after each wait
//!   `ScrollDelay` gives (`ControlMgrExtensions.a` line 73): 48 ticks over
//!   the ticks held plus 4, so 12, 3, 2, 2, 2, 1 and so on, and none once
//!   held 45 ([`scroll_delay`]).
//!
//! It is drawn in plain greys, the engine's look for an OS-themed control.
//! It plays no sound.

use crate::color::Color;
use crate::draw::{DrawList, fill_rect, frame_rect};
use crate::geometry::{Bounds, Point};

/// The bar's width, and the height of each arrow box and of the thumb.
pub const BAR_WIDTH: f32 = 16.0;

/// How far past the bar the thumb's outline may be dragged up or down
/// (`ThumbSBar`'s inset, `$FFE8`).
const DRAG_REACH: f32 = 24.0;

/// How far left and right of the drag's limits the pointer may go before
/// a release leaves the thumb (`ThumbSBar`'s slop inset, `$FF80`, after
/// the limit's 16).
const SLOP_REACH: f32 = 128.0 - BAR_WIDTH;

/// The frame's, the arrows' and the thumb's edge.
pub const EDGE: Color = Color::rgba(0, 0, 0, 255);
/// The track's grey.
pub const TRACK: Color = Color::rgba(170, 170, 170, 255);
/// The arrow boxes' and the thumb's grey.
pub const KNOB: Color = Color::rgba(221, 221, 221, 255);
/// A held arrow's or page's grey.
pub const HELD: Color = Color::rgba(102, 102, 102, 255);

/// A part of the bar (the Control Manager's `inUpButton`, `inDownButton`,
/// `inPageUp`, `inPageDown` and `inThumb`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarPart {
    /// The up arrow: back a row.
    Up,
    /// The down arrow: on a row.
    Down,
    /// The track above the thumb: back a page.
    PageUp,
    /// The track below the thumb: on a page.
    PageDown,
    /// The thumb, dragged.
    Thumb,
}

/// How many ticks `ScrollDelay` waits after a step made `held` ticks into
/// a hold: `DecayValue` (48) over `held` plus `DecayValue / FirstValue`
/// (4), the end value 0 (`ControlMgrExtensions.a` lines 83-123).
#[must_use]
pub fn scroll_delay(held: u64) -> u64 {
    48 / (held + 4)
}

/// `value * span / range`, rounded as `RoundOff` rounds a `DIVU`: up when
/// the remainder is more than half the divisor.
fn scaled(value: u64, span: u64, range: u64) -> u64 {
    if range == 0 {
        return 0;
    }
    let product = value * span;
    let (quotient, remainder) = (product / range, product % range);
    quotient + u64::from(remainder > range / 2)
}

/// A list's vertical scroll bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListBar {
    rect: Bounds,
}

impl ListBar {
    /// The bar of a list whose view is `view`: right of it, 16 wide, a
    /// pixel past it above and below.
    #[must_use]
    pub fn beside(view: Bounds) -> Self {
        Self {
            rect: Bounds {
                min: Point::new(view.max.x, view.min.y - 1.0),
                max: Point::new(view.max.x + BAR_WIDTH, view.max.y + 1.0),
            },
        }
    }

    /// Where it is.
    #[must_use]
    pub fn rect(&self) -> Bounds {
        self.rect
    }

    /// Where the arrow `part` ([`BarPart::Up`] or [`BarPart::Down`]) is: a
    /// square of the bar's width at its top or bottom. `None` for another
    /// part.
    #[must_use]
    pub fn arrow(&self, part: BarPart) -> Option<Bounds> {
        let (left, right) = (self.rect.min.x, self.rect.max.x);
        match part {
            BarPart::Up => Some(Bounds {
                min: self.rect.min,
                max: Point::new(right, self.rect.min.y + BAR_WIDTH),
            }),
            BarPart::Down => Some(Bounds {
                min: Point::new(left, self.rect.max.y - BAR_WIDTH),
                max: self.rect.max,
            }),
            BarPart::PageUp | BarPart::PageDown | BarPart::Thumb => None,
        }
    }

    /// The track between the arrows, less the thumb's height: where the
    /// thumb's top moves, in pixels.
    fn span(&self) -> u64 {
        (self.rect.height() - 3.0 * BAR_WIDTH).max(0.0) as u64
    }

    /// The thumb at `value` of `0..=max`.
    #[must_use]
    pub fn thumb(&self, value: usize, max: usize) -> Bounds {
        let offset = scaled(value as u64, self.span(), max as u64) as f32;
        self.thumb_at(self.rect.min.y + BAR_WIDTH + offset)
    }

    /// The thumb with its top at `top`.
    fn thumb_at(&self, top: f32) -> Bounds {
        Bounds {
            min: Point::new(self.rect.min.x + 1.0, top),
            max: Point::new(self.rect.max.x - 1.0, top + BAR_WIDTH),
        }
    }

    /// The part at `at`, the thumb at `value` of `0..=max`; none off the
    /// bar.
    #[must_use]
    pub fn part_at(&self, at: Point, value: usize, max: usize) -> Option<BarPart> {
        if !self.rect.contains_half_open(at) {
            return None;
        }
        if at.y - self.rect.min.y <= BAR_WIDTH {
            return Some(BarPart::Up);
        }
        if self.rect.max.y - at.y <= BAR_WIDTH {
            return Some(BarPart::Down);
        }
        let thumb = self.thumb(value, max);
        if thumb.contains_half_open(at) {
            return Some(BarPart::Thumb);
        }
        let middle = thumb.min.y + (BAR_WIDTH / 2.0).floor();
        Some(if at.y < middle {
            BarPart::PageUp
        } else {
            BarPart::PageDown
        })
    }

    /// Where the thumb's top is dragged, pressed at `press` with the thumb
    /// at `value` of `0..=max` and the pointer now at `at`: the pointer's
    /// travel held to `ThumbSBar`'s limits, 24 past the bar at either end.
    fn dragged_top(&self, value: usize, max: usize, press: Point, at: Point) -> f32 {
        let top = self.thumb(value, max).min.y;
        let grab = press.y - top;
        let low = self.rect.min.y - DRAG_REACH + grab;
        let high = self.rect.max.y + DRAG_REACH - (BAR_WIDTH - 1.0) + grab;
        top + at.y.clamp(low, high.max(low)) - press.y
    }

    /// Whether a drag's pointer at `at` is in the slop rect, where a
    /// release sets the value.
    fn in_slop(&self, at: Point) -> bool {
        Bounds {
            min: Point::new(self.rect.min.x - SLOP_REACH, self.rect.min.y - DRAG_REACH),
            max: Point::new(self.rect.max.x + SLOP_REACH, self.rect.max.y + DRAG_REACH),
        }
        .contains_half_open(at)
    }

    /// The thumb's outline while it is dragged, pressed at `press` with
    /// the thumb at `value` of `0..=max` and the pointer at `at`; none
    /// while the pointer is out of the slop rect.
    #[must_use]
    pub fn drag_outline(
        &self,
        value: usize,
        max: usize,
        press: Point,
        at: Point,
    ) -> Option<Bounds> {
        self.in_slop(at)
            .then(|| self.thumb_at(self.dragged_top(value, max, press, at)))
    }

    /// The value a thumb drag sets, pressed at `press` with the thumb at
    /// `value` of `0..=max` and released at `at`: the outline's top scaled
    /// back over the range and held to it (`MoveSBar`, `SetCtlValue`), or
    /// `value` when released out of the slop rect. An outline above the
    /// track sets 0: the original's unsigned 16-bit arithmetic there isn't
    /// replicated.
    #[must_use]
    pub fn dragged_value(&self, value: usize, max: usize, press: Point, at: Point) -> usize {
        if !self.in_slop(at) {
            return value;
        }
        let top = self.dragged_top(value, max, press, at);
        let offset = (top - self.rect.min.y - BAR_WIDTH).max(0.0) as u64;
        let span = self.span();
        if span == 0 {
            return value;
        }
        (scaled(offset, max as u64, span) as usize).min(max)
    }

    /// Draws it, the thumb at `value` of `0..=max`, the part `held` darker,
    /// and the thumb's `outline` while it is dragged.
    pub fn draw(
        &self,
        list: &mut DrawList,
        value: usize,
        max: usize,
        held: Option<BarPart>,
        outline: Option<Bounds>,
    ) {
        let rect = self.rect;
        fill_rect(list, rect, TRACK);
        let up = Bounds {
            min: rect.min,
            max: Point::new(rect.max.x, rect.min.y + BAR_WIDTH),
        };
        let down = Bounds {
            min: Point::new(rect.min.x, rect.max.y - BAR_WIDTH),
            max: rect.max,
        };
        let thumb = self.thumb(value, max);
        for (part, area) in [(BarPart::Up, up), (BarPart::Down, down)] {
            fill_rect(list, area, if held == Some(part) { HELD } else { KNOB });
            frame_rect(list, area, EDGE);
        }
        let page = match held {
            Some(BarPart::PageUp) => Some(Bounds {
                min: Point::new(rect.min.x, up.max.y),
                max: Point::new(rect.max.x, thumb.min.y),
            }),
            Some(BarPart::PageDown) => Some(Bounds {
                min: Point::new(rect.min.x, thumb.max.y),
                max: Point::new(rect.max.x, down.min.y),
            }),
            _ => None,
        };
        if let Some(page) = page {
            fill_rect(list, page, HELD);
        }
        fill_rect(list, thumb, KNOB);
        frame_rect(list, thumb, EDGE);
        frame_rect(list, rect, EDGE);
        if let Some(outline) = outline {
            frame_rect(list, outline, EDGE);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::{DrawCommand, fill_command};

    /// The mission info screen's view on stock `DITL` 1012, item 2 less
    /// the bar's 15: (9, 24) to (189, 108).
    const VIEW: Bounds = Bounds {
        min: Point::new(9.0, 24.0),
        max: Point::new(189.0, 108.0),
    };

    fn bar() -> ListBar {
        ListBar::beside(VIEW)
    }

    #[test]
    fn the_bar_lies_right_of_the_view_16_wide_and_a_pixel_past_it() {
        assert_eq!(
            bar().rect(),
            Bounds {
                min: Point::new(189.0, 23.0),
                max: Point::new(205.0, 109.0),
            },
            "GetScSize, ListMgrPACK.a line 1828"
        );
    }

    #[test]
    fn the_thumb_moves_over_the_track_between_the_arrows() {
        // 86 high: the track less the thumb is 86 - 48 = 38.
        let bar = bar();
        assert_eq!(
            bar.thumb(0, 4),
            Bounds {
                min: Point::new(190.0, 39.0),
                max: Point::new(204.0, 55.0),
            },
            "below the up arrow, inset a pixel"
        );
        assert_eq!(bar.thumb(4, 4).min.y, 39.0 + 38.0, "at the end");
        assert_eq!(bar.thumb(2, 4).min.y, 39.0 + 19.0, "half way");
        // 1 x 38 / 4 = 9 r 2: 2 is not more than half of 4, so down.
        assert_eq!(bar.thumb(1, 4).min.y, 39.0 + 9.0);
        // 3 x 38 / 4 = 28 r 2: down again; 1 x 38 / 3 = 12 r 2: up.
        assert_eq!(bar.thumb(3, 4).min.y, 39.0 + 28.0);
        assert_eq!(bar.thumb(1, 3).min.y, 39.0 + 13.0, "RoundOff rounds up");
        assert_eq!(bar.thumb(0, 0).min.y, 39.0, "no range: at the top");
    }

    #[test]
    fn each_part_is_hit_to_its_edges() {
        let bar = bar();
        let at = |y: f32| bar.part_at(Point::new(195.0, y), 0, 4);
        assert_eq!(at(22.0), None, "above the bar");
        assert_eq!(at(23.0), Some(BarPart::Up), "the bar's top");
        assert_eq!(at(39.0), Some(BarPart::Up), "16 below its top");
        assert_eq!(at(40.0), Some(BarPart::Thumb), "the thumb at 0");
        assert_eq!(at(54.0), Some(BarPart::Thumb));
        assert_eq!(at(55.0), Some(BarPart::PageDown), "below the thumb");
        assert_eq!(at(92.0), Some(BarPart::PageDown));
        assert_eq!(at(93.0), Some(BarPart::Down), "16 above its bottom");
        assert_eq!(at(108.0), Some(BarPart::Down), "its last row");
        assert_eq!(at(109.0), None, "its bottom edge is out");
        assert_eq!(bar.part_at(Point::new(188.0, 60.0), 0, 4), None, "left");
        assert_eq!(
            bar.part_at(Point::new(189.0, 60.0), 0, 4),
            Some(BarPart::PageDown),
            "its first column"
        );
        assert_eq!(bar.part_at(Point::new(205.0, 60.0), 0, 4), None, "right");
        assert_eq!(
            bar.part_at(Point::new(189.0, 50.0), 0, 4),
            Some(BarPart::PageDown),
            "the column the thumb's inset leaves is the page's"
        );
        assert_eq!(
            bar.part_at(Point::new(189.0, 45.0), 0, 4),
            Some(BarPart::PageUp),
            "above the thumb's middle"
        );
    }

    #[test]
    fn the_page_parts_split_at_the_thumbs_middle() {
        // At 4 of 4 the thumb is 77 to 93, its middle 85.
        let bar = bar();
        let at = |y: f32| bar.part_at(Point::new(195.0, y), 4, 4);
        assert_eq!(at(76.0), Some(BarPart::PageUp));
        assert_eq!(at(77.0), Some(BarPart::Thumb));
        assert_eq!(at(92.0), Some(BarPart::Thumb));
        assert_eq!(at(93.0), Some(BarPart::Down));
        // With the thumb's rows but in the inset column, by the middle.
        let side = |y: f32| bar.part_at(Point::new(189.0, y), 4, 4);
        assert_eq!(side(84.0), Some(BarPart::PageUp));
        assert_eq!(side(85.0), Some(BarPart::PageDown));
    }

    #[test]
    fn a_drag_sets_the_value_where_the_outline_is_dropped() {
        let bar = bar();
        let press = Point::new(195.0, 45.0);
        assert_eq!(
            bar.dragged_value(0, 4, press, Point::new(195.0, 64.0)),
            2,
            "19 of 38 is half way"
        );
        assert_eq!(bar.dragged_value(0, 4, press, press), 0, "not moved");
        assert_eq!(
            bar.dragged_value(0, 4, press, Point::new(195.0, 130.0)),
            4,
            "held to the end"
        );
        assert_eq!(
            bar.dragged_value(4, 4, Point::new(195.0, 80.0), Point::new(195.0, 0.0)),
            0,
            "held to the start"
        );
        // 5 of 38 x 4 = 20 / 38 = 0 r 20: more than half of 38, so 1.
        assert_eq!(bar.dragged_value(0, 4, press, Point::new(195.0, 50.0)), 1);
        assert_eq!(
            bar.dragged_value(1, 4, press, Point::new(205.0 + 112.0, 64.0)),
            1,
            "released right of the slop rect: unchanged"
        );
        assert_eq!(
            bar.dragged_value(0, 4, press, Point::new(205.0 + 111.0, 64.0)),
            2,
            "just inside it"
        );
        assert_eq!(
            bar.dragged_value(0, 4, press, Point::new(195.0, 109.0 + 24.0)),
            0,
            "below the slop rect: unchanged"
        );
        assert_eq!(
            bar.dragged_value(1, 4, press, Point::new(189.0 - 113.0, 64.0)),
            1,
            "left of the slop rect: unchanged"
        );
        assert_eq!(
            bar.dragged_value(0, 4, press, Point::new(189.0 - 112.0, 64.0)),
            2,
            "its left edge is in"
        );
    }

    #[test]
    fn the_outline_follows_the_pointer_to_its_limits() {
        let bar = bar();
        let press = Point::new(195.0, 45.0);
        assert_eq!(
            bar.drag_outline(0, 4, press, Point::new(195.0, 64.0))
                .map(|outline| outline.min.y),
            Some(58.0)
        );
        assert_eq!(
            bar.drag_outline(0, 4, press, Point::new(195.0, 0.0))
                .map(|outline| outline.min.y),
            Some(23.0 - 24.0),
            "24 above the bar at most"
        );
        assert_eq!(
            bar.drag_outline(0, 4, press, Point::new(195.0, 132.0))
                .map(|outline| outline.min.y),
            Some(109.0 + 24.0 - 15.0),
            "its top 24 less 15 below the bar at most"
        );
        assert_eq!(
            bar.drag_outline(0, 4, press, Point::new(400.0, 64.0)),
            None,
            "hidden out of the slop rect"
        );
    }

    #[test]
    fn scroll_delay_decays_from_12_ticks_to_none() {
        let waits: Vec<u64> = [0, 12, 15, 17, 19, 21, 22, 44, 45]
            .into_iter()
            .map(scroll_delay)
            .collect();
        assert_eq!(waits, [12, 3, 2, 2, 2, 1, 1, 1, 0]);
    }

    fn drawn(list: &DrawList) -> Vec<DrawCommand> {
        list.iter().cloned().collect()
    }

    #[test]
    fn it_draws_the_track_arrows_and_thumb_the_held_part_darker() {
        let bar = bar();
        let mut list = DrawList::new();
        bar.draw(&mut list, 0, 4, None, None);
        let commands = drawn(&list);
        assert_eq!(commands[0], fill_command(bar.rect(), TRACK));
        assert!(commands.contains(&fill_command(bar.thumb(0, 4), KNOB)));
        let up = Bounds::at(Point::new(189.0, 23.0), 16.0, 16.0);
        assert!(commands.contains(&fill_command(up, KNOB)));
        let mut held = DrawList::new();
        bar.draw(&mut held, 0, 4, Some(BarPart::Up), None);
        assert!(drawn(&held).contains(&fill_command(up, HELD)));
        let mut paged = DrawList::new();
        bar.draw(&mut paged, 0, 4, Some(BarPart::PageDown), None);
        let below = Bounds {
            min: Point::new(189.0, 55.0),
            max: Point::new(205.0, 93.0),
        };
        assert!(drawn(&paged).contains(&fill_command(below, HELD)));
        let mut back = DrawList::new();
        bar.draw(&mut back, 4, 4, Some(BarPart::PageUp), None);
        let above = Bounds {
            min: Point::new(189.0, 39.0),
            max: Point::new(205.0, 77.0),
        };
        assert!(drawn(&back).contains(&fill_command(above, HELD)));
        let mut dragged = DrawList::new();
        let outline = bar.thumb(2, 4);
        bar.draw(&mut dragged, 0, 4, Some(BarPart::Thumb), Some(outline));
        assert!(dragged.iter().count() > list.iter().count(), "the outline");
    }
}
