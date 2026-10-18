//! The session's tests of the exit cursors (rdm weapons-fidelity phase
//! 18): a ship-table slot keeps its last ship's cursors for the next ship
//! brought into it, and the player's carry through every refit and, by
//! the engine, through a change of ship, as only `_ModifyShotStartPosition`
//! and `_SpawnBeam` write +0xc8fe-+0xc905 (see
//! [`berth_exits`](crate::traffic::berth_exits) and
//! [`RuleKey::ReusedSlotExits`]). The fold set (+0xc8fc), which no refit
//! writes either, carries through an outfit refit too, but a change of
//! ship sets it back to 0, the original's `BaseSets` − 1 for a hull of
//! one set (`_HandlePlayer` @0x6ef3f).

use super::ship_change::ClassChange;
use super::tests::{FIRE, Firing, armed, blaster, facing_an_npc};
use super::*;
use crate::catalog::WeaponRecord;
use crate::chance::NeverFires;
use crate::combat::Rules;
use crate::combat::exits::ExitKind;
use crate::rulebook::{RuleSource, Rulebook};
use crate::testkit::FakePilotCatalog;

/// [`armed`] with a gun of no reload and `ExitType` 0, every hull's four
/// gun exits 10 ahead and 1, 2, 3 and 4 pixels to starboard.
fn four_guns() -> FakePilotCatalog {
    let gun = WeaponRecord {
        reload: 0,
        exit_type: 0,
        ..blaster()
    };
    let mut catalog = FakePilotCatalog {
        weapons: vec![gun],
        ..armed()
    };
    for hull in &mut catalog.hulls {
        hull.exits = Some(crate::catalog::ShipExits {
            points: [
                [(1, 10), (2, 10), (3, 10), (4, 10)],
                [(0, 0); 4],
                [(0, 0); 4],
                [(0, 0); 4],
            ],
            up_compress: (100, 100),
            down_compress: (100, 100),
            frames: 36,
            ..crate::catalog::ShipExits::default()
        });
    }
    catalog
}

/// Which of [`four_guns`]' exits, 0 to 3, `ship`'s newest shot left from,
/// the ship facing up at `centre`.
fn newest_exit(session: &Session, ship: ShipRef, centre: Vec2) -> usize {
    let shot = session
        .shots()
        .iter()
        .rfind(|shot| shot.firer == ship)
        .expect("a shot");
    let start = shot.position - shot.velocity - centre;
    assert!((start.y + 10.0).abs() < 1e-3, "10 ahead: {start:?}");
    (start.x.round() as usize) - 1
}

/// The session facing its NPC, which fires two gun shots and leaves,
/// reading reused slots' cursors by `reading`; then the same class brought
/// into its slot, with fresh cursors of its own.
fn reused_slot(reading: RuleSource) -> (FakePilotCatalog, Session) {
    let catalog = four_guns();
    let rules = Rulebook::default().with_override(RuleKey::ReusedSlotExits, reading);
    let mut session = facing_an_npc(&catalog, 0).with_rules(rules);
    let npc = ShipRef::Npc(NpcId(0));
    for exit in [0, 1] {
        session.tick_traffic(&catalog, &Firing, &mut NeverFires);
        session.tick_combat(Rules::default(), &mut NeverFires);
        let centre = session.npcs()[0].state.position;
        assert_eq!(newest_exit(&session, npc, centre), exit, "the first ship's");
    }
    let mut next = session.npcs()[0].clone();
    *next.armament.exit_cursors_mut() = crate::combat::exits::ExitCursors::default();
    session.traffic.remove(NpcId(0));
    let id = session.traffic.add_npc(next);
    assert_eq!(session.npc(id).map(|npc| npc.berth), Some(1), "its slot");
    (catalog, session)
}

#[test]
fn a_new_ship_in_a_reused_slot_fires_its_first_gun_shot_from_the_exit_after_its_predecessors_last()
{
    for (reading, exit) in [(RuleSource::Engine, 2), (RuleSource::Bible, 0)] {
        let (catalog, mut session) = reused_slot(reading);
        let id = session.npcs()[0].id;
        session.tick_traffic(&catalog, &Firing, &mut NeverFires);
        session.tick_combat(Rules::default(), &mut NeverFires);
        let centre = session.npcs()[0].state.position;
        assert_eq!(
            newest_exit(&session, ShipRef::Npc(id), centre),
            exit,
            "{reading:?}"
        );
    }
}

#[test]
fn the_reused_slot_exits_reading_is_set_by_the_rulebook_and_outlives_a_new_traffic() {
    let catalog = armed();
    for reading in [RuleSource::Engine, RuleSource::Bible] {
        let rules = Rulebook::default().with_override(RuleKey::ReusedSlotExits, reading);
        let mut session = facing_an_npc(&catalog, 180).with_rules(rules);
        assert_eq!(session.traffic.reused_slot_exits(), reading);
        let stellar = session.sites[0].id;
        session
            .land_at(&catalog, session.pilot.system, stellar)
            .expect("relocates");
        assert_eq!(session.npcs(), [], "a new traffic");
        assert_eq!(session.traffic.reused_slot_exits(), reading, "re-applied");
    }
}

/// [`four_guns`]' session, reading the player's cursors by `reading`,
/// after the player has fired one gun shot.
fn fired_once(catalog: &FakePilotCatalog, reading: RuleSource) -> Session {
    let rules = Rulebook::default().with_override(RuleKey::ReusedSlotExits, reading);
    let mut session = Session::start(catalog).expect("starts").with_rules(rules);
    session.player.heading = 0.0;
    fire(&mut session);
    assert_eq!(session.armament.exit_cursors().next(ExitKind::Gun), 1);
    session
}

/// The player fires one tick of gun shots and lets go, giving the exit it
/// fired from.
fn fire(session: &mut Session) -> usize {
    session.hold_trigger(FIRE);
    session.tick_combat(Rules::default(), &mut NeverFires);
    session.hold_trigger(Trigger::default());
    newest_exit(session, ShipRef::Player, session.player.position)
}

#[test]
fn a_refit_keeps_the_players_exit_cursors_by_either_reading() {
    let catalog = four_guns();
    for reading in [RuleSource::Engine, RuleSource::Bible] {
        let mut session = fired_once(&catalog, reading);
        session.refit(true, ReservePolicy::Hold);
        assert_eq!(
            session.armament.exit_cursors().next(ExitKind::Gun),
            1,
            "{reading:?}"
        );
        assert_eq!(fire(&mut session), 1, "{reading:?}");
    }
}

#[test]
fn a_refit_keeps_the_players_fold_set() {
    let catalog = four_guns();
    let mut session = Session::start(&catalog).expect("starts");
    session.armament = session.armament.clone().with_fold_set(2);
    session.refit(true, ReservePolicy::Hold);
    assert_eq!(session.armament.fold_set(), 2);
}

#[test]
fn a_change_of_ship_keeps_the_players_exit_cursors_by_the_engine_and_resets_them_by_the_other_reading()
 {
    let catalog = four_guns();
    for (reading, exit) in [(RuleSource::Engine, 1), (RuleSource::Bible, 0)] {
        let mut session = fired_once(&catalog, reading);
        let record = session.ship_record(ShipId(129)).cloned().expect("ship 129");
        let change = ClassChange {
            outfits: session.pilot.outfits.clone(),
            reserves: ReservePolicy::Hold,
        };
        session.become_class(&record, change, |_| {});
        assert_eq!(session.ship(), ShipId(129));
        assert_eq!(
            session.armament.exit_cursors().next(ExitKind::Gun),
            exit,
            "{reading:?}"
        );
        assert_eq!(fire(&mut session), exit, "{reading:?}: the new hull's");
    }
}

#[test]
fn a_change_of_ship_puts_the_players_fold_set_back_to_0_by_either_reading() {
    let catalog = four_guns();
    for reading in [RuleSource::Engine, RuleSource::Bible] {
        let rules = Rulebook::default().with_override(RuleKey::ReusedSlotExits, reading);
        let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
        session.armament = session.armament.clone().with_fold_set(2);
        let record = session.ship_record(ShipId(129)).cloned().expect("ship 129");
        let change = ClassChange {
            outfits: session.pilot.outfits.clone(),
            reserves: ReservePolicy::Hold,
        };
        session.become_class(&record, change, |_| {});
        assert_eq!(session.ship(), ShipId(129));
        assert_eq!(session.armament.fold_set(), 0, "{reading:?}");
    }
}

#[test]
fn the_rulebook_says_whether_a_stray_beam_steps_the_fold_set() {
    let catalog = armed();
    let session = |rules: Rulebook| Session::start(&catalog).expect("starts").with_rules(rules);
    assert_eq!(session(Rulebook::default()).combat, Combat::default());
    let mut still = Combat::default();
    still.set_stray_beam_exit(RuleSource::Bible);
    assert_ne!(still, Combat::default());
    let other = Rulebook::default().with_override(RuleKey::StrayBeamExit, RuleSource::Bible);
    assert_eq!(session(other).combat, still);
}
