//! The Red Alert icon (`_HandleAlertSprite` @0x2e61d-0x2e6f0): `cïcn`
//! 18000 or 18001, as the session's [`AlertIcon`] says, at the playfield's
//! top-right corner, flush with the status bar.
//!
//! `_InitSprites` places it once (@0x20360-0x2038b) at `x = 2 ·
//! _halfPlayWidth − width`, `y = 0`, the width that of the sprite's
//! frame 0, `cïcn` 18000, so both frames draw from the same point. It sits
//! on the interface layer (@0x20304-0x20312) with the target brackets,
//! over space and every ship.
//!
//! The blink follows the session's frame counter (see
//! [`nova_sim::red_alert`]), not the HUD's `TickCount` cadence.
//!
//! The double-time icon (`_HandleDoubleTimeSprite` @0x2e5a7-0x2e61c):
//! `cïcn` [`DOUBLE_TIME_CICN`], one frame (`_LoadSprites`
//! @0x21a0e-0x21a3d), shown at (0, 0) while double time is on
//! (@0x2e5ae-0x2e5df). It is on the interface layer too, added after the
//! Red Alert's sprite (@0x20390-0x203a4 against @0x20304-0x20312), so it
//! is drawn over it.

use nova_sim::AlertIcon;

use crate::geometry::Point;
use crate::system::camera::Anchor;
use crate::{DrawList, ImageKey};

/// Where the icon's top-left corner is, for `cïcn` 18000 `width` pixels
/// wide, in a view `view_width` wide: x = 2 × trunc((`view_width` − 194) /
/// 2) − width, the original's doubled `_screenCenter` (set by
/// `_SetupWind` @0x1b720, [`Anchor::PlayAreaCentre`]). That is the bar's
/// left edge less the width, and a pixel further left when the play
/// area's width is odd. It is placed so by both readings of
/// [`RuleKey::FlightAnchor`](nova_sim::RuleKey::FlightAnchor), as
/// `_InitSprites` places it.
#[must_use]
pub fn alert_origin(width: u32, view_width: f32) -> Point {
    let half_play = Anchor::PlayAreaCentre.point((view_width, 0.0)).x;
    Point::new(2.0 * half_play - width as f32, 0.0)
}

/// Draws `icon`, for `cïcn` 18000 `width` pixels wide, in a view
/// `view_width` wide.
pub fn draw_red_alert(list: &mut DrawList, icon: AlertIcon, width: u32, view_width: f32) {
    list.picture(ImageKey::icon(icon.cicn()), alert_origin(width, view_width));
}

/// The `cïcn` of the double-time icon (0x4e20, @0x21a0e).
pub const DOUBLE_TIME_CICN: i16 = 20000;

/// Draws the double-time icon at the top left.
pub fn draw_double_time(list: &mut DrawList) {
    list.picture(ImageKey::icon(DOUBLE_TIME_CICN), Point::new(0.0, 0.0));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DrawCommand;

    #[test]
    fn the_icon_sits_at_the_playfields_top_right() {
        assert_eq!(alert_origin(24, 1024.0), Point::new(806.0, 0.0));
        assert_eq!(alert_origin(32, 1024.0), Point::new(798.0, 0.0));
    }

    #[test]
    fn in_a_widened_view_the_icon_sits_left_of_the_bar_by_the_doubled_half_play_width() {
        // 1365 − 194 = 1171, odd: trunc(585.5) × 2 = 1170.
        assert_eq!(alert_origin(24, 1365.0), Point::new(1146.0, 0.0));
        // 1366 − 194 = 1172, even: flush with the bar.
        assert_eq!(alert_origin(24, 1366.0), Point::new(1148.0, 0.0));
    }

    #[test]
    fn the_double_time_icon_is_cicn_20000_at_the_top_left() {
        let mut list = DrawList::new();
        draw_double_time(&mut list);
        let commands: Vec<_> = list.iter().cloned().collect();
        assert_eq!(
            commands,
            [DrawCommand::Picture {
                image: ImageKey::icon(20000),
                top_left: Point::new(0.0, 0.0),
            }]
        );
    }

    #[test]
    fn each_frame_draws_its_cicn_from_the_same_point() {
        let mut list = DrawList::new();
        draw_red_alert(&mut list, AlertIcon::Red, 24, 1024.0);
        draw_red_alert(&mut list, AlertIcon::Dark, 24, 1024.0);
        let commands: Vec<_> = list.iter().cloned().collect();
        assert_eq!(
            commands,
            [
                DrawCommand::Picture {
                    image: ImageKey::icon(18000),
                    top_left: Point::new(806.0, 0.0),
                },
                DrawCommand::Picture {
                    image: ImageKey::icon(18001),
                    top_left: Point::new(806.0, 0.0),
                },
            ]
        );
    }
}
