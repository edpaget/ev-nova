//! Each ship's turret facing (rdm:phase/engine-3d/phase-10-sample-in-3d):
//! a readout for the view, which draws a ship's turret outfits facing it
//! (`nova_view::flight::mounted_outfits`), carried on the frame as
//! [`FrameShip::turret_facing`](crate::flight_frame::FrameShip::turret_facing).
//!
//! - The fight records the aim of each turret shot and turreted beam
//!   (`Combat::take_turret_aims`, the `turret_aim` module of
//!   `nova_sim_combat`). After the pass's fight ([`Session::fight_tail`],
//!   before the frame is drawn) the session drains them here, once a
//!   pass: every ship's facing steps its hold, then each aim faces its
//!   ship's turrets that way, the player's on [`PlayerShip`] and an NPC's
//!   on its own record, so a removed NPC's facing goes with it.
//! - A ship that fires no turret shot for [`HOLD_PASSES`] passes faces its
//!   turrets to the nose again. The hold is stepped here alone, once a
//!   pass, never per ship, so it lasts as long whatever the traffic.
//! - The player's facing clears as it enters a system, lands or takes a
//!   new ship; an NPC starts with none at every spawn
//!   (`Traffic::add_npc_at`).
//!
//! The original keeps no turret facing, so this has no counterpart there
//! and no rulebook key. Nothing in the simulation reads it, so stock play
//! is unchanged, and no event carries it. One facing per ship serves all
//! its turning mounts. The view, not the simulation, swings a drawn
//! turret toward it at a rate (`nova_view::flight::turret_swing`). It
//! plays no sound.

use super::{PlayerShip, Session, Space};
use crate::combat::ShipRef;
use crate::combat::turret_aim::TurretAim;

/// How many passes a ship's turrets keep facing their last aim with no
/// turret shot after it: three seconds at a pass a tick, so a turret
/// between shots never flicks to the nose (the new game's sample mass
/// driver reloads in half a second, 15 passes) and settles back soon
/// after the fight.
pub(super) const HOLD_PASSES: u32 = 90;

impl Session {
    /// Drains the pass's turret aims into each ship's facing (see the
    /// module docs).
    pub(super) fn keep_turret_facings(&mut self) {
        let aims = self.space.combat.take_turret_aims();
        self.ship.keep_turret_facing(&aims);
        self.space.keep_turret_facings(&aims);
    }
}

impl PlayerShip {
    /// Steps the player's facing's hold, then faces its last aim among
    /// `aims`, if any.
    fn keep_turret_facing(&mut self, aims: &[TurretAim]) {
        self.turret_facing.step();
        if let Some(aim) = aims.iter().rev().find(|aim| aim.ship == ShipRef::Player) {
            self.turret_facing.aim(aim.heading, HOLD_PASSES);
        }
    }

    /// Faces the player's turrets to the nose: on entering a system,
    /// landing or taking a new ship.
    pub(super) fn clear_turret_facing(&mut self) {
        self.turret_facing = crate::combat::turret_aim::TurretFacing::default();
    }
}

impl Space {
    /// Steps each NPC's facing's hold, then faces each NPC's last aim
    /// among `aims`; an aim of an NPC gone is dropped.
    fn keep_turret_facings(&mut self, aims: &[TurretAim]) {
        for npc in self.traffic.npcs_mut() {
            npc.turret_facing.step();
            let ship = ShipRef::Npc(npc.id);
            if let Some(aim) = aims.iter().rev().find(|aim| aim.ship == ship) {
                npc.turret_facing.aim(aim.heading, HOLD_PASSES);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::combat::armament::Trigger;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::{FakePilotCatalog, work};

    /// [`armed`]'s catalog, its blaster a turret (guidance 4).
    fn turreted() -> FakePilotCatalog {
        let mut catalog = armed();
        let blaster = &mut catalog.weapons[0];
        *blaster = WeaponRecord {
            guidance: 4,
            ..*blaster
        };
        catalog
    }

    /// `catalog`'s session with the player at rest at the centre, facing
    /// up, its NPC 100 above and 60 to the right, off its nose, targeted.
    fn targeting(catalog: &FakePilotCatalog) -> Session {
        let mut session = facing_an_npc(catalog, 180);
        session.ship.player.position = Vec2::ZERO;
        session.ship.player.velocity = Vec2::ZERO;
        session.space.traffic.npcs_mut()[0].state.position = Vec2::new(60.0, -100.0);
        session.select_target(crate::TargetPick::Nearest);
        session
    }

    fn hold_fire(session: &mut Session) {
        session.hold_trigger(Trigger {
            primary: true,
            ..Trigger::default()
        });
    }

    /// The player's facing, and its NPC's, as the frame carries them.
    fn facings(session: &Session) -> (Option<f32>, Option<f32>) {
        let npc = &session.npcs()[0];
        (
            session.frame_ship_player().turret_facing,
            session.frame_ship_npc(npc).turret_facing,
        )
    }

    /// `aim` for `ship`, raised as the fight records one.
    fn raise(session: &mut Session, ship: ShipRef, heading: f32) {
        session
            .space
            .combat
            .raise_turret_aim(TurretAim { ship, heading });
    }

    #[test]
    fn the_players_facing_is_its_first_turret_shots_aim() {
        let catalog = turreted();
        let mut session = targeting(&catalog);
        assert_eq!(facings(&session).0, None, "none before it fires");
        hold_fire(&mut session);
        work(&mut session, &catalog, Controls::default());
        let shot = session
            .shots()
            .iter()
            .find(|shot| shot.firer == ShipRef::Player)
            .copied()
            .expect("the player fired");
        // The turret is true, so its shot flies along its aim.
        assert_eq!(facings(&session).0, Some(shot.heading));
        assert!(shot.heading.abs() > 1.0, "off the nose: {}", shot.heading);
    }

    #[test]
    fn a_fixed_gun_leaves_the_facing_at_the_nose() {
        let catalog = armed();
        let mut session = targeting(&catalog);
        hold_fire(&mut session);
        work(&mut session, &catalog, Controls::default());
        assert!(
            session
                .shots()
                .iter()
                .any(|shot| shot.firer == ShipRef::Player),
            "the player fired"
        );
        assert_eq!(facings(&session).0, None);
    }

    #[test]
    fn an_npcs_facing_is_its_aim() {
        let catalog = armed();
        let mut session = targeting(&catalog);
        let npc = ShipRef::Npc(session.npcs()[0].id);
        raise(&mut session, npc, 200.0);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(facings(&session), (None, Some(200.0)));
    }

    /// How many passes `session`'s player and first NPC keep an aim raised
    /// now, with no turret fire after it.
    fn held_passes(session: &mut Session, catalog: &FakePilotCatalog) -> (u32, u32) {
        let npc = ShipRef::Npc(session.npcs()[0].id);
        raise(session, ShipRef::Player, 30.0);
        raise(session, npc, 40.0);
        let mut held = (0, 0);
        for _ in 0..HOLD_PASSES * 2 {
            work(session, catalog, Controls::default());
            let (player, npc) = facings(session);
            held.0 += u32::from(player.is_some());
            held.1 += u32::from(npc.is_some());
        }
        held
    }

    #[test]
    fn a_facing_holds_for_the_hold_and_then_faces_the_nose() {
        let catalog = armed();
        let mut session = targeting(&catalog);
        // The aim's own pass counts as the hold's first.
        assert_eq!(
            held_passes(&mut session, &catalog),
            (HOLD_PASSES, HOLD_PASSES)
        );
        assert_eq!(facings(&session), (None, None));
    }

    #[test]
    fn the_hold_lasts_as_long_with_more_ships_in_the_system() {
        let catalog = armed();
        let mut session = targeting(&catalog);
        let ship = session.npcs()[0].clone();
        for _ in 0..4 {
            session.space.traffic.add_npc(ship.clone());
        }
        assert_eq!(session.npcs().len(), 5);
        assert_eq!(
            held_passes(&mut session, &catalog),
            (HOLD_PASSES, HOLD_PASSES)
        );
    }

    #[test]
    fn a_facing_held_while_firing_keeps_its_latest_aim() {
        let catalog = armed();
        let mut session = targeting(&catalog);
        raise(&mut session, ShipRef::Player, 30.0);
        for _ in 0..HOLD_PASSES - 1 {
            work(&mut session, &catalog, Controls::default());
        }
        raise(&mut session, ShipRef::Player, 50.0);
        for _ in 0..HOLD_PASSES - 1 {
            work(&mut session, &catalog, Controls::default());
        }
        assert_eq!(facings(&session).0, Some(50.0), "held past the first hold");
    }

    #[test]
    fn the_players_facing_clears_on_landing_on_entering_a_system_and_on_a_new_ship() {
        let catalog = armed();
        let faced = || {
            let mut session = targeting(&catalog);
            raise(&mut session, ShipRef::Player, 30.0);
            work(&mut session, &catalog, Controls::default());
            assert_eq!(facings(&session).0, Some(30.0));
            session
        };
        let mut landed = faced();
        let stellar = landed.chart.sites[0].id;
        landed
            .land_at(&catalog, landed.pilot.system, stellar)
            .expect("lands");
        assert_eq!(landed.frame_ship_player().turret_facing, None, "landed");

        let mut jumped = faced();
        // Holding the facing through the flight out, as if it fired.
        jumped.ship.turret_facing.aim(30.0, u32::MAX);
        crate::testkit::jump(&mut jumped, &catalog, 132).expect("arrives");
        assert_eq!(jumped.frame_ship_player().turret_facing, None, "entered");

        let mut changed = faced();
        let record = catalog.ship_records[0].clone();
        let change = super::super::ship_change::ClassChange {
            outfits: changed.pilot.outfits.clone(),
            reserves: super::super::ReservePolicy::Hold,
            held: std::collections::BTreeMap::new(),
        };
        changed.become_class(&record, change, |_| {});
        assert_eq!(
            changed.frame_ship_player().turret_facing,
            None,
            "a new ship"
        );
    }

    #[test]
    fn an_npc_placed_anew_from_a_faced_record_faces_the_nose() {
        let catalog = armed();
        let mut session = targeting(&catalog);
        let npc = ShipRef::Npc(session.npcs()[0].id);
        raise(&mut session, npc, 40.0);
        work(&mut session, &catalog, Controls::default());
        let faced = session.npcs()[0].clone();
        assert_eq!(faced.turret_facing.heading(), Some(40.0));
        session.space.traffic.remove(faced.id);
        let id = session.space.traffic.add_npc_at(faced.clone(), faced.berth);
        let placed = session
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("placed");
        assert_eq!(placed.berth, faced.berth, "the same berth");
        assert_eq!(placed.turret_facing.heading(), None);
    }

    #[test]
    fn nothing_the_simulation_steps_reads_a_facing() {
        let catalog = turreted();
        let run = |seeded: bool| {
            let mut session = targeting(&catalog);
            hold_fire(&mut session);
            if seeded {
                let npc = session.npcs()[0].id;
                session.ship.turret_facing.aim(123.0, HOLD_PASSES * 10);
                session.space.traffic.npcs_mut()[0]
                    .turret_facing
                    .aim(321.0, HOLD_PASSES * 10);
                assert_eq!(session.npcs()[0].id, npc);
            }
            for _ in 0..40 {
                work(&mut session, &catalog, Controls::default());
            }
            // Every facing back to none, so the rest compares whole.
            session.ship.clear_turret_facing();
            for npc in session.space.traffic.npcs_mut() {
                npc.turret_facing = crate::combat::turret_aim::TurretFacing::default();
            }
            session
        };
        let (plain, seeded) = (run(false), run(true));
        assert!(
            plain
                .shots()
                .iter()
                .any(|shot| shot.firer == ShipRef::Player),
            "the turret fired"
        );
        assert_eq!(plain.ship, seeded.ship);
        assert_eq!(plain.npcs(), seeded.npcs());
        assert_eq!(plain.shots(), seeded.shots());
    }
}
