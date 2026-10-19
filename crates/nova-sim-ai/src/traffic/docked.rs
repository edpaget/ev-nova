//! Whether an NPC is docked with the player, `_AIShipIsDockedWithShip`
//! (@0x8960a-0x89692) with the player as the other ship. Its only caller
//! is the player's self-destruct (@0x70293), which destroys every ship
//! docked with it (see `nova_sim::self_destruct`).
//!
//! A ship is docked with another when all of these hold:
//!
//! - its sub-goal +0xc8ca is 0xf, whatever its goal (@0x89614);
//! - its target +0x70 **or** its nav +0x6c is the other ship's slot +0x86,
//!   0 for the player (@0x89621-0x89632);
//! - it is no further than [`DOCKED_WITH`] on each axis, inclusive, a NaN
//!   off counting as too far (@0x89634-0x89689).
//!
//! On the engine, sub 0xf is:
//!
//! - a pirate's [`Npc::docking`] in [`DockSub::Dock`], whose
//!   [`ship`](crate::traffic::plunder_dock::Docking::ship) is its nav (see
//!   [`plunder_dock`](crate::traffic::plunder_dock));
//! - a repairer's stand-in: [`Goal::Assist`] with [`Help::Repair`] within
//!   its [`assist::reach`] of the player. `_AIMakeShipRepairPlayer`
//!   (@0x82e08) targets the player, so its target test always passes.
//!
//! *Reconcile:* rdm:phase/combat-ai-fidelity/phase-19-left-out-of-phase-10
//! gives the repairer an [`Npc::docking`] in [`DockSub::Dock`] with the
//! player; drop the repairer's stand-in here when it lands, as that phase
//! drops it from the answer flag's `docking`.

use crate::ai::{Goal, Help};
use crate::combat::ShipRef;
use crate::flight::ShipState;
use crate::hail::assist;
use crate::traffic::npc::Npc;
use crate::traffic::plunder_dock::DockSub;

/// How far, in pixels on each axis, a docked ship may be from the ship it
/// docks with (16.0, 0xdd048).
pub const DOCKED_WITH: f32 = 16.0;

/// Whether `npc` is docked with the player at `player` (see the module
/// docs).
#[must_use]
pub fn docked_with_player(npc: &Npc, player: &ShipState) -> bool {
    let nav = npc
        .docking
        .filter(|docking| docking.sub == DockSub::Dock)
        .map(|docking| docking.ship);
    let repairing = npc.goal == Goal::Assist(Help::Repair)
        && assist::within(
            &npc.state,
            player,
            assist::reach(Help::Repair, npc.turn_rate()),
        );
    let on_player =
        repairing || nav == Some(ShipRef::Player) || npc.target == Some(ShipRef::Player);
    let off = npc.state.position - player.position;
    (nav.is_some() || repairing)
        && on_player
        && off.x.abs() <= DOCKED_WITH
        && off.y.abs() <= DOCKED_WITH
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::NpcId;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::plunder_dock::Docking;
    use nova_sim_base::fixture::FAST;

    /// The player, off the centre and moving, so the test reads the
    /// offset, not the position.
    fn player() -> ShipState {
        ShipState {
            position: Vec2::new(100.0, -50.0),
            velocity: Vec2::new(2.0, 1.0),
            heading: 90.0,
        }
    }

    /// An NPC `off` the player.
    fn near(off: Vec2) -> Npc {
        let mut ship = npc(1, ShipStats::new(FAST, &[]));
        ship.state.position = player().position + off;
        ship
    }

    /// A pirate `off` the player, docking with `ship` by `sub`.
    fn pirate(off: Vec2, ship: ShipRef, sub: DockSub) -> Npc {
        let mut pirate = near(off);
        pirate.goal = Goal::Plunder {
            ship,
            hopeless: false,
        };
        pirate.docking = Some(Docking { ship, sub });
        pirate
    }

    /// A ship repairing the player, `off` it.
    fn repairer(off: Vec2) -> Npc {
        let mut ship = near(off);
        ship.goal = Goal::Assist(Help::Repair);
        ship
    }

    #[test]
    fn a_docked_pirate_on_the_player_counts_out_to_16_on_both_axes() {
        for off in [
            Vec2::new(16.0, 16.0),
            Vec2::new(-16.0, -16.0),
            Vec2::new(0.0, 0.0),
        ] {
            let ship = pirate(off, ShipRef::Player, DockSub::Dock);
            assert!(docked_with_player(&ship, &player()), "{off:?}");
        }
    }

    #[test]
    fn a_docked_pirate_past_16_on_either_axis_does_not() {
        for off in [
            Vec2::new(16.01, 0.0),
            Vec2::new(0.0, -16.01),
            Vec2::new(f32::NAN, 0.0),
            Vec2::new(0.0, f32::NAN),
        ] {
            let ship = pirate(off, ShipRef::Player, DockSub::Dock);
            assert!(!docked_with_player(&ship, &player()), "{off:?}");
        }
    }

    #[test]
    fn a_pirate_closing_or_approaching_is_not_docked() {
        for sub in [DockSub::Approach, DockSub::Close] {
            let ship = pirate(Vec2::ZERO, ShipRef::Player, sub);
            assert!(!docked_with_player(&ship, &player()), "{sub:?}");
        }
    }

    #[test]
    fn docking_with_another_ship_counts_only_if_it_targets_the_player() {
        let other = ShipRef::Npc(NpcId(3));
        let mut ship = pirate(Vec2::ZERO, other, DockSub::Dock);
        assert!(!docked_with_player(&ship, &player()));
        ship.target = Some(ShipRef::Player);
        assert!(docked_with_player(&ship, &player()));
        ship.target = Some(other);
        assert!(!docked_with_player(&ship, &player()));
    }

    #[test]
    fn a_ship_not_docking_is_not_docked_whatever_it_targets() {
        let mut ship = near(Vec2::ZERO);
        ship.target = Some(ShipRef::Player);
        assert!(!docked_with_player(&ship, &player()));
    }

    #[test]
    fn a_repairer_within_its_reach_and_16_counts() {
        let ship = repairer(Vec2::new(16.0, -16.0));
        assert!(docked_with_player(&ship, &player()));
    }

    #[test]
    fn a_repairer_within_its_reach_but_17_off_does_not() {
        let ship = repairer(Vec2::new(17.0, 0.0));
        let reach = assist::reach(Help::Repair, ship.turn_rate());
        assert!(reach > 17.0, "within its reach");
        assert!(!docked_with_player(&ship, &player()));
    }

    #[test]
    fn docked_with_is_16() {
        assert!((DOCKED_WITH - 16.0).abs() < f32::EPSILON);
    }
}
