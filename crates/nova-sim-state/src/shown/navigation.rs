//! The player's nav mode ([`NavMode`]) and a sprite slot's rect
//! ([`SpriteRect`]) with the click margin it is tested by, from
//! `nova_sim_ai::navigation`, which re-exports them and keeps the stellar
//! picking.

use nova_sim_base::geometry::Vec2;
use nova_sim_catalog::catalog::{StellarId, SystemId};

/// The player's nav mode and its selection: the original's player +0x6a
/// and +0x6c. Mode 4, the hypergate map's, is not modelled: the
/// hypergate resets it to −1 (@0x63fa2) before any status draw. The
/// mode is not saved: `_ResetPlayer` sets −1 / −1 (@0x1d4cd-0x1d4d5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavMode {
    /// Mode −1: the nav system off, nothing selected.
    #[default]
    Off,
    /// Mode 2: the stellar selected, `None` for none (+0x6c −1).
    Stellar(Option<StellarId>),
    /// Mode 3: the system selected, `None` for none (+0x6c −1). The
    /// original keeps a slot of the current system's links, resolved by
    /// place whenever it is read (`_FindActiveCoLocatedSystem` @0x4be0);
    /// here it is the system, resolved by place as it is read.
    Hyperspace(Option<SystemId>),
}

impl NavMode {
    /// The mode kept and its selection let go (+0x6c −1), as a take-off,
    /// a gate's exit or an in-flight `M` writes it.
    #[must_use]
    pub fn deselected(self) -> Self {
        match self {
            Self::Off => Self::Off,
            Self::Stellar(_) => Self::Stellar(None),
            Self::Hyperspace(_) => Self::Hyperspace(None),
        }
    }

    /// The stellar selected in mode 2, the one every reader of the
    /// stellar selection asks for: `None` in any other mode.
    #[must_use]
    pub fn stellar(self) -> Option<StellarId> {
        match self {
            Self::Stellar(stellar) => stellar,
            _ => None,
        }
    }

    /// The system selected in mode 3, the one the galaxy map draws its
    /// selected link to (`_DrawMap` @0xebd8-0xed31): `None` in any other
    /// mode.
    #[must_use]
    pub fn hyperspace(self) -> Option<SystemId> {
        match self {
            Self::Hyperspace(system) => system,
            _ => None,
        }
    }
}

/// How far a click may land outside a small sprite and still hit it, in
/// pixels on each side: the original grows the sprite's rect by 16 on
/// every side before testing a click against it (`_InsetRect(r, -16,
/// -16)`, `_HandlePlayer` @0x6a671 for stellars, @0x6a3db and @0x6a4df for
/// ships, `_HandleMouse` @0x381a8 and @0x38264 for the hover).
pub const CLICK_MARGIN: f32 = 16.0;

/// The widest sprite frame, in pixels, that gets the [`CLICK_MARGIN`]:
/// the original compares the rect's width, right less left, with 0x2f
/// (47) and grows the rect only when it is no wider, whatever its height
/// (`_HandlePlayer` @0x6a649-0x6a66c, @0x6a3b3-0x6a3d6, @0x6a4b7-0x6a4da;
/// `_HandleMouse` @0x38183-0x381a3, @0x3823f-0x3825f).
pub const SMALL_SPRITE_WIDTH: u32 = 47;

/// A sprite slot's rect (the original's +0x1a, a QuickDraw `Rect`), in
/// system coordinates: the box a click and the hover test.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteRect {
    /// The left edge.
    pub left: f32,
    /// The top edge.
    pub top: f32,
    /// The width in pixels.
    pub width: u32,
    /// The height in pixels.
    pub height: u32,
}

impl SpriteRect {
    /// A `size` rect placed as the original places a slot around
    /// `centre`: its top left at `centre` less half the size, halved as
    /// integers (`_HandleStellarSprites` @0x2eec3-0x2ef08,
    /// `_HandleShipDisplay` @0x2cd1c-0x2ce31), so an odd size's extra
    /// pixel is on the right or the bottom.
    #[must_use]
    pub fn centred(centre: Vec2, (width, height): (u32, u32)) -> Self {
        Self {
            left: centre.x - (width / 2) as f32,
            top: centre.y - (height / 2) as f32,
            width,
            height,
        }
    }

    /// Whether a click at `at` hits the rect: grown by [`CLICK_MARGIN`] on
    /// every side when it is no wider than [`SMALL_SPRITE_WIDTH`], then
    /// tested as `_PtInRect` does (@0x6a686): left and top in, right and
    /// bottom out.
    #[must_use]
    pub fn holds(self, at: Vec2) -> bool {
        let margin = if self.width <= SMALL_SPRITE_WIDTH {
            CLICK_MARGIN
        } else {
            0.0
        };
        let (left, top) = (self.left - margin, self.top - margin);
        let right = self.left + self.width as f32 + margin;
        let bottom = self.top + self.height as f32 + margin;
        left <= at.x && at.x < right && top <= at.y && at.y < bottom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nav_mode_starts_off() {
        assert_eq!(NavMode::default(), NavMode::Off);
    }

    #[test]
    fn deselecting_keeps_the_mode_and_lets_go_of_the_selection() {
        let cases = [
            (NavMode::Off, NavMode::Off),
            (
                NavMode::Stellar(Some(StellarId(128))),
                NavMode::Stellar(None),
            ),
            (NavMode::Stellar(None), NavMode::Stellar(None)),
            (
                NavMode::Hyperspace(Some(SystemId(131))),
                NavMode::Hyperspace(None),
            ),
            (NavMode::Hyperspace(None), NavMode::Hyperspace(None)),
        ];
        for (mode, deselected) in cases {
            assert_eq!(mode.deselected(), deselected, "{mode:?}");
        }
    }

    #[test]
    fn only_mode_2_with_a_stellar_names_a_stellar() {
        assert_eq!(
            NavMode::Stellar(Some(StellarId(128))).stellar(),
            Some(StellarId(128))
        );
        for mode in [
            NavMode::Off,
            NavMode::Stellar(None),
            NavMode::Hyperspace(Some(SystemId(128))),
            NavMode::Hyperspace(None),
        ] {
            assert_eq!(mode.stellar(), None, "{mode:?}");
        }
    }

    #[test]
    fn only_mode_3_with_a_system_names_a_hyperspace_selection() {
        assert_eq!(
            NavMode::Hyperspace(Some(SystemId(131))).hyperspace(),
            Some(SystemId(131))
        );
        for mode in [
            NavMode::Off,
            NavMode::Stellar(Some(StellarId(128))),
            NavMode::Stellar(None),
            NavMode::Hyperspace(None),
        ] {
            assert_eq!(mode.hyperspace(), None, "{mode:?}");
        }
    }
}
