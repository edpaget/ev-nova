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

use nova_sim::AlertIcon;

use super::hud::STATUS_BAR_WIDTH;
use crate::geometry::Point;
use crate::system::camera::VIEW_SIZE;
use crate::{DrawList, ImageKey};

/// Where the icon's top-left corner is, for `cïcn` 18000 `width` pixels
/// wide.
#[must_use]
pub fn alert_origin(width: u32) -> Point {
    Point::new(VIEW_SIZE.0 - STATUS_BAR_WIDTH - width as f32, 0.0)
}

/// Draws `icon`, for `cïcn` 18000 `width` pixels wide.
pub fn draw_red_alert(list: &mut DrawList, icon: AlertIcon, width: u32) {
    list.picture(ImageKey::icon(icon.cicn()), alert_origin(width));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DrawCommand;

    #[test]
    fn the_icon_sits_at_the_playfields_top_right() {
        assert_eq!(alert_origin(24), Point::new(806.0, 0.0));
        assert_eq!(alert_origin(32), Point::new(798.0, 0.0));
    }

    #[test]
    fn each_frame_draws_its_cicn_from_the_same_point() {
        let mut list = DrawList::new();
        draw_red_alert(&mut list, AlertIcon::Red, 24);
        draw_red_alert(&mut list, AlertIcon::Dark, 24);
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
