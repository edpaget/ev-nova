//! The hover test: whether the mouse pointer is over something the player
//! can target, which the view animates its cursor sprite over.
//!
//! The original (`_HandleMouse` @0x37fba-0x38466 in `EV Nova.app`, read
//! with `otool -tV -arch i386`) walks the ship slots 0-63 (@0x380e3-0x381ec)
//! with the player's two scanner answers read once (@0x380ce, @0x380db),
//! and skips a slot that is:
//!
//! - inactive (+0xb8, @0x380e7), or dying (`_IsDying`, @0x380f7): only an
//!   intact or disabled ship counts ([`hover_gate`]);
//! - cloaked (`_ShipIsCloaked`), unless the player can target cloaked
//!   ships, its leader +0x9a is the player, or it is the player
//!   (@0x3810b-0x38130);
//! - of an untargetable type (`shïp` `Flags2` 0x4, +0x9ea) unless the
//!   player can target those (@0x3814f-0x3816f);
//! - without a sprite (+0x00 null, @0x38171), which the engine never has:
//!   [`DrawnShips`] giving `None`. It tests the sprite pointer, not
//!   whether the sprite is shown, so a hidden ship
//!   ([`Session::is_hidden`](crate::Session::is_hidden)), whose sprite
//!   `_HandleShipDisplay` still positions (@0x2cd17), is hovered where it
//!   would be drawn.
//!
//! Unlike [`targetable`](crate::targeting::targetable) it has no check for
//! a ship emerging from a gate. Its hit box is the sprite's rect (+0x1a),
//! grown by [`CLICK_MARGIN`](crate::navigation::CLICK_MARGIN) on every
//! side when it is no taller than
//! [`SMALL_SPRITE_HEIGHT`](crate::navigation::SMALL_SPRITE_HEIGHT)
//! (@0x38183-0x381ba), the rule a click on a stellar uses. The player's own
//! ship counts only while it has a target (+0x70 not -1,
//! @0x381c3-0x381d5). The other-system check (@0x38145) has no engine
//! counterpart: every NPC is in the player's system.
//!
//! With no ship under the pointer the stellars are tried, unless a jump is
//! under way (+0x50 > 0, @0x381f2-0x3828d), with the click's box
//! ([`Session::hover`](crate::Session::hover)). The test makes no sound and
//! targets nothing.

use crate::catalog::StellarId;
use crate::cloak::Scanner;
use crate::combat::hull::Condition;
use crate::geometry::Vec2;
use crate::navigation::half_box;
use crate::targeting::type_targetable;
use crate::traffic::npc::{Npc, NpcId};

/// A ship's sprite as it is drawn: its centre, in system coordinates, and
/// its frame's size in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawnBox {
    /// Where the sprite is centred.
    pub centre: Vec2,
    /// The frame's width.
    pub width: u32,
    /// The frame's height.
    pub height: u32,
}

/// Where the view draws the ships: the port the hover test reads.
pub trait DrawnShips {
    /// The box NPC `id` is drawn in, or would be when it is hidden, or
    /// `None` when it has no sprite slot (the original's null sprite,
    /// @0x38171), which the engine never has.
    fn npc(&self, id: NpcId) -> Option<DrawnBox>;
    /// The box the player's ship is drawn in, or `None` when it has none.
    fn player(&self) -> Option<DrawnBox>;
}

/// What the pointer is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hover {
    /// The player's own ship, which counts only while it has a target.
    Player,
    /// An NPC.
    Ship(NpcId),
    /// A stellar.
    Stellar(StellarId),
}

/// Whether `npc` can be hovered by a player of `scanner` (see the module
/// docs): intact or disabled, not cloaked unless `scanner` targets cloaked
/// ships or it escorts the player, and of a type `scanner` may pick.
#[must_use]
pub fn hover_gate(npc: &Npc, scanner: Scanner) -> bool {
    matches!(npc.condition, Condition::Intact | Condition::Disabled)
        && (scanner.targets || npc.escorts_player() || !npc.cloak.is_cloaked())
        && type_targetable(npc, scanner)
}

/// Whether `at` is in `drawn`'s hit box, edges included (see the module
/// docs).
#[must_use]
pub fn in_box(at: Vec2, drawn: DrawnBox) -> bool {
    let (half_width, half_height) = half_box(Some((drawn.width, drawn.height)));
    let offset = at - drawn.centre;
    offset.x.abs() <= half_width && offset.y.abs() <= half_height
}

/// The ship under `at` among the player's (only when `targeting`, the
/// player having a target) and `npcs`, for a player of `scanner`, boxed as
/// `drawn` draws them (see the module docs). Which of two overlapping ships
/// is given does not matter to the cursor.
#[must_use]
pub fn ship_under(
    npcs: &[Npc],
    scanner: Scanner,
    targeting: bool,
    at: Vec2,
    drawn: &impl DrawnShips,
) -> Option<Hover> {
    if targeting && drawn.player().is_some_and(|ship| in_box(at, ship)) {
        return Some(Hover::Player);
    }
    npcs.iter()
        .filter(|npc| hover_gate(npc, scanner))
        .find(|npc| drawn.npc(npc.id).is_some_and(|ship| in_box(at, ship)))
        .map(|npc| Hover::Ship(npc.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloak::{Cloak, CloakGear, FADE_FULL, UNTARGETABLE};
    use crate::stats::ShipStats;
    use crate::testkit::npc;
    use crate::traffic::npc::Mode;
    use std::collections::BTreeMap;

    /// The view's boxes, canned.
    #[derive(Default)]
    struct FakeDrawn {
        npcs: BTreeMap<NpcId, DrawnBox>,
        player: Option<DrawnBox>,
    }

    impl DrawnShips for FakeDrawn {
        fn npc(&self, id: NpcId) -> Option<DrawnBox> {
            self.npcs.get(&id).copied()
        }

        fn player(&self) -> Option<DrawnBox> {
            self.player
        }
    }

    const BLIND: Scanner = Scanner::NONE;

    /// A 60-pixel-square box (too tall for the margin) at (`x`, `y`).
    fn square(x: f32, y: f32) -> DrawnBox {
        DrawnBox {
            centre: Vec2::new(x, y),
            width: 60,
            height: 60,
        }
    }

    /// An intact NPC `id`.
    fn ship(id: u32) -> Npc {
        npc(id, ShipStats::default())
    }

    fn cloaked(mut npc: Npc) -> Npc {
        npc.cloak = Cloak {
            fade: FADE_FULL,
            direction: 0,
        };
        npc
    }

    /// `npcs` each drawn in a 60-pixel square at the origin.
    fn all_at_origin(npcs: &[Npc]) -> FakeDrawn {
        FakeDrawn {
            npcs: npcs.iter().map(|npc| (npc.id, square(0.0, 0.0))).collect(),
            player: None,
        }
    }

    /// What the pointer at the origin is over among `npcs`, drawn at it.
    fn over(npcs: &[Npc], scanner: Scanner) -> Option<Hover> {
        ship_under(npcs, scanner, false, Vec2::ZERO, &all_at_origin(npcs))
    }

    #[test]
    fn hover_skips_an_untargetable_ship_without_the_scanner_and_takes_it_with_it() {
        let mut hidden = ship(1);
        hidden.cloak_gear = CloakGear {
            flags2: UNTARGETABLE,
            ..CloakGear::default()
        };
        let npcs = [hidden];
        assert_eq!(over(&npcs, BLIND), None);
        let cloak_bit = Scanner {
            targets: true,
            ..BLIND
        };
        assert_eq!(over(&npcs, cloak_bit), None, "the other bit");
        let scanner = Scanner {
            untargetable: true,
            ..BLIND
        };
        assert_eq!(over(&npcs, scanner), Some(Hover::Ship(NpcId(1))));
    }

    #[test]
    fn hover_skips_a_cloaked_ship_without_the_cloak_bit_and_takes_it_with_it() {
        let npcs = [cloaked(ship(1))];
        assert_eq!(over(&npcs, BLIND), None);
        let screen_only = Scanner {
            on_screen: true,
            on_radar: true,
            untargetable: true,
            ..BLIND
        };
        assert_eq!(over(&npcs, screen_only), None, "seeing is not targeting");
        let scanner = Scanner {
            targets: true,
            ..BLIND
        };
        assert_eq!(over(&npcs, scanner), Some(Hover::Ship(NpcId(1))));
    }

    #[test]
    fn hover_takes_a_cloaked_escort_of_the_player_without_the_bit() {
        let mut ours = cloaked(ship(1));
        ours.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot {
                slot: 0,
                ships: 1,
                spacing: 30.0,
            },
            order: None,
        });
        assert_eq!(over(&[ours], BLIND), Some(Hover::Ship(NpcId(1))));
    }

    #[test]
    fn hover_takes_intact_and_disabled_ships_and_skips_dying_and_destroyed_ones() {
        let of = |condition| {
            let mut npc = ship(1);
            npc.condition = condition;
            over(&[npc], BLIND)
        };
        assert_eq!(of(Condition::Intact), Some(Hover::Ship(NpcId(1))));
        assert_eq!(of(Condition::Disabled), Some(Hover::Ship(NpcId(1))));
        assert_eq!(of(Condition::Dying { ticks_left: 5 }), None);
        assert_eq!(of(Condition::Destroyed), None);
    }

    #[test]
    fn hover_takes_a_ship_emerging_from_a_gate() {
        let mut emerging = ship(1);
        emerging.mode = Mode::Emerging { ticks_left: 3 };
        assert!(emerging.emerging());
        assert_eq!(over(&[emerging], BLIND), Some(Hover::Ship(NpcId(1))));
    }

    #[test]
    fn a_ship_that_is_not_drawn_is_not_hovered() {
        let npcs = [ship(1), ship(2)];
        let mut drawn = all_at_origin(&npcs);
        drawn.npcs.remove(&NpcId(1));
        assert_eq!(
            ship_under(&npcs, BLIND, false, Vec2::ZERO, &drawn),
            Some(Hover::Ship(NpcId(2)))
        );
        drawn.npcs.clear();
        assert_eq!(ship_under(&npcs, BLIND, false, Vec2::ZERO, &drawn), None);
    }

    #[test]
    fn a_box_no_taller_than_47_grows_by_16_and_a_48_high_one_does_not() {
        let sized = |height| DrawnBox {
            centre: Vec2::new(100.0, 100.0),
            width: 20,
            height,
        };
        // Half of 20 wide is 10, plus the 16-pixel margin.
        assert!(in_box(Vec2::new(126.0, 100.0), sized(47)));
        assert!(!in_box(Vec2::new(126.5, 100.0), sized(47)));
        assert!(in_box(Vec2::new(100.0, 100.0 - 23.5 - 16.0), sized(47)));
        assert!(in_box(Vec2::new(110.0, 124.0), sized(48)), "edges count");
        assert!(!in_box(Vec2::new(110.5, 100.0), sized(48)));
        assert!(!in_box(Vec2::new(100.0, 124.5), sized(48)));
        assert!(in_box(Vec2::new(90.0, 76.0), sized(48)));
    }

    #[test]
    fn the_players_own_ship_is_hovered_only_while_it_holds_a_target() {
        let drawn = FakeDrawn {
            npcs: BTreeMap::new(),
            player: Some(square(0.0, 0.0)),
        };
        assert_eq!(ship_under(&[], BLIND, false, Vec2::ZERO, &drawn), None);
        assert_eq!(
            ship_under(&[], BLIND, true, Vec2::ZERO, &drawn),
            Some(Hover::Player)
        );
        assert_eq!(
            ship_under(&[], BLIND, true, Vec2::new(31.0, 0.0), &drawn),
            None,
            "off it"
        );
        assert_eq!(
            ship_under(&[], BLIND, true, Vec2::ZERO, &FakeDrawn::default()),
            None,
            "no sprite"
        );
    }

    #[test]
    fn a_ship_off_the_pointer_is_not_hovered() {
        let npcs = [ship(1)];
        let drawn = FakeDrawn {
            npcs: [(NpcId(1), square(200.0, -50.0))].into(),
            player: None,
        };
        assert_eq!(ship_under(&npcs, BLIND, false, Vec2::ZERO, &drawn), None);
        assert_eq!(
            ship_under(&npcs, BLIND, false, Vec2::new(225.0, -75.0), &drawn),
            Some(Hover::Ship(NpcId(1)))
        );
    }
}
