//! A ship's alt layer as it was last drawn, which a disabled ship of
//! `shän` `Flags` 0x0020 keeps showing.
//!
//! `_HandleShipDisplay` skips a disabled 0x0020 ship's whole alt block
//! without hiding the sprite (@0x2b6b7-0x2b705), so the sprite keeps the
//! frame and tint it last had (@0x2c97d-0x2cb11, the tint copied from the
//! hull @0x2caa1-0x2cb0a) and its screen place, which is set relative to
//! the player's ship (@0x2cef5-0x2cf5a) and so stays put on screen as the
//! view moves. The session says when ([`Alt::Held`], by the engine's
//! reading of `nova_sim::RuleKey::DisabledAltHeld`; see
//! `nova_sim::combat::ship_frames`); [`HeldAlts`] keeps, for each ship,
//! the alt layer as last drawn live and gives it back while held.
//!
//! The original hides the sprite only by the paths that hide it whatever
//! its flag: a slot gone inactive, the ship leaving the system
//! (@0x2b525-0x2b589), and a fully faded ship, a cloak's or a gate
//! fade's (-0x151, @0x2d735-0x2d77b). So a ship whose hull is not drawn
//! forgets its record, as does one whose alt layer is not drawn at all
//! ([`Alt::None`]). A ship that has left is not drawn at all, and an NPC's
//! number is never reused in a system; every record is forgotten as the
//! player enters another.

use std::collections::BTreeMap;

use nova_sim::ShipRef;
use nova_sim::combat::ship_frames::Alt;

use crate::{Blend, Color, Colorize, ImageKey, Point};

/// An alt layer as drawn: its frame, its screen place, its tint and its
/// colorize.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AltLayer {
    /// Its sprite and frame.
    pub image: ImageKey,
    /// Its centre on screen.
    pub at: Point,
    /// Its tint, the hull's.
    pub tint: Color,
    /// Its colorize, the hull's, if any.
    pub colorize: Option<Colorize>,
    /// How it combines with what is beneath: the hull's, plain unless a
    /// cloak fades it (see [`cloak_fade`](super::cloak_fade)).
    pub blend: Blend,
}

/// Each ship's alt layer as last drawn live (see the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeldAlts {
    last: BTreeMap<ShipRef, AltLayer>,
}

impl HeldAlts {
    /// What `ship`'s alt layer draws this frame, its session saying `alt`,
    /// `live` its layer drawn as it is now, if drawn on a set, its hull
    /// drawn when `hull`: the live layer, recorded; the record while held;
    /// nothing, forgotten, when its hull is faded out or no alt shows.
    pub fn shown(
        &mut self,
        ship: ShipRef,
        alt: Alt,
        live: Option<AltLayer>,
        hull: bool,
    ) -> Option<AltLayer> {
        if !hull {
            self.forget(ship);
            return None;
        }
        match (alt, live) {
            (Alt::Drawn(_), Some(live)) => {
                self.last.insert(ship, live);
                Some(live)
            }
            (Alt::Held, _) => self.last.get(&ship).copied(),
            (Alt::Drawn(_) | Alt::None, _) => {
                self.forget(ship);
                None
            }
        }
    }

    /// Forgets `ship`'s record: it is faded out or gone.
    pub fn forget(&mut self, ship: ShipRef) {
        self.last.remove(&ship);
    }

    /// Forgets every record: a new system.
    pub fn clear(&mut self) {
        self.last.clear();
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::traffic::npc::NpcId;

    use super::*;

    const SHIP: ShipRef = ShipRef::Npc(NpcId(3));

    fn layer(frame: u16, x: f32) -> AltLayer {
        AltLayer {
            image: ImageKey::sprite(2400, frame),
            at: Point::new(x, 200.0),
            tint: Color::WHITE,
            colorize: None,
            blend: Blend::Normal,
        }
    }

    #[test]
    fn a_live_alt_is_recorded_each_frame() {
        let mut held = HeldAlts::default();
        assert_eq!(
            held.shown(SHIP, Alt::Drawn(0), Some(layer(0, 100.0)), true),
            Some(layer(0, 100.0))
        );
        assert_eq!(
            held.shown(SHIP, Alt::Drawn(1), Some(layer(37, 110.0)), true),
            Some(layer(37, 110.0))
        );
        assert_eq!(
            held.shown(SHIP, Alt::Held, None, true),
            Some(layer(37, 110.0)),
            "the latest"
        );
    }

    #[test]
    fn a_held_alt_stays_at_its_last_screen_point_and_frame_as_the_view_moves() {
        let mut held = HeldAlts::default();
        held.shown(SHIP, Alt::Drawn(2), Some(layer(74, 100.0)), true);
        for _ in 0..3 {
            assert_eq!(
                held.shown(SHIP, Alt::Held, None, true),
                Some(layer(74, 100.0))
            );
        }
        // Repaired, it is drawn live again where the ship now is.
        assert_eq!(
            held.shown(SHIP, Alt::Drawn(2), Some(layer(75, 300.0)), true),
            Some(layer(75, 300.0))
        );
        // Another ship's record is its own.
        let other = ShipRef::Player;
        assert_eq!(held.shown(other, Alt::Held, None, true), None, "none kept");
    }

    #[test]
    fn a_held_alt_is_forgotten_when_its_ship_fades_out_or_shows_none() {
        let mut held = HeldAlts::default();
        held.shown(SHIP, Alt::Drawn(0), Some(layer(0, 100.0)), true);
        assert_eq!(held.shown(SHIP, Alt::Held, None, false), None, "faded out");
        assert_eq!(held.shown(SHIP, Alt::Held, None, true), None, "forgotten");
        held.shown(SHIP, Alt::Drawn(0), Some(layer(0, 100.0)), true);
        assert_eq!(held.shown(SHIP, Alt::None, None, true), None, "hidden");
        assert_eq!(held.shown(SHIP, Alt::Held, None, true), None);
        held.shown(SHIP, Alt::Drawn(0), Some(layer(0, 100.0)), true);
        held.forget(SHIP);
        assert_eq!(held.shown(SHIP, Alt::Held, None, true), None, "forgotten");
        held.shown(SHIP, Alt::Drawn(0), Some(layer(0, 100.0)), true);
        assert_eq!(
            held.shown(SHIP, Alt::Drawn(0), None, true),
            None,
            "no sheet"
        );
        assert_eq!(held.shown(SHIP, Alt::Held, None, true), None);
        held.shown(SHIP, Alt::Drawn(0), Some(layer(0, 100.0)), true);
        held.clear();
        assert_eq!(held.shown(SHIP, Alt::Held, None, true), None, "new system");
    }
}
