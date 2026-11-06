//! The session's tests of the escort-command mark (+0xc4) a ship-table
//! slot keeps (see [`berth_marks`](crate::traffic::berth_marks)): how
//! [`RuleKey::ReusedSlotMark`] reaches the traffic, which of the session's
//! placements keep the berth's mark or clear it, and the class-order
//! bookkeeping reading a kept mark (see the `class_orders` module).

use super::fighters::tests::{carrying, fleet_npcs, flying_by, launch, pilot, warship};
use super::*;
use crate::chance::NeverFires;
use crate::escort::{EscortClass, EscortCommand, EscortGroup, EscortOrder, FormationSlot};
use crate::govt::Governments;
use crate::mission::ships::ShipBehav;
use crate::rulebook::{RuleSource, Rulebook};
use crate::testkit::FakePilotCatalog;
use crate::traffic::SPAWN_RESERVE;
use crate::traffic::npc::{AiType, MissionMark, NpcHold, NpcId};
use crate::traffic::spawn::{Entry, NewShip};

/// The rulebook with `reading` of [`RuleKey::ReusedSlotMark`].
fn reading(reading: RuleSource) -> Rulebook {
    Rulebook::default().with_override(RuleKey::ReusedSlotMark, reading)
}

const WARSHIPS: EscortGroup = EscortGroup::Class(EscortClass::Warship);

/// The order kept for `class`.
fn kept(session: &Session, class: EscortClass) -> Option<EscortOrder> {
    session.space.kept_for(class)
}

/// The bookkeeping run once on `session`: whether it cleared a class.
fn settle(session: &mut Session) -> bool {
    session
        .space
        .settle_class_orders(&mut session.pilot, &session.statics)
}

/// NPC `id` of `session`.
fn npc_of(session: &Session, id: NpcId) -> &Npc {
    session.space.npc(id).expect("in the system")
}

/// A warship escort flying by `reading` of the rule, commanded Hold, so
/// its berth is marked, and then gone: the session, still keeping the
/// warships' Hold, and the berth it left.
fn commanded_and_gone(catalog: &FakePilotCatalog, source: RuleSource) -> (Session, u32) {
    let mut session = flying_by(
        catalog,
        pilot(catalog, 2, vec![warship(None)]),
        reading(source),
    );
    assert_eq!(session.space.traffic.reused_slot_mark(), source);
    session.command_escorts(WARSHIPS, EscortCommand::Hold, &mut NeverFires);
    let berth = fleet_npcs(&session)[0].berth;
    assert_eq!(
        session.space.traffic.berth_mark(berth),
        Some(true),
        "the command marks its berth (@0x66190)"
    );
    session.drop_fleet_escort(0);
    assert_eq!(
        kept(&session, EscortClass::Warship),
        Some(EscortOrder::Hold)
    );
    (session, berth)
}

/// A ship of the warship type (150) as a spawn of the traffic brings it
/// in, `clears` its mark or not, of `mission`.
fn new_warship(clears: bool, mission: Option<MissionMark>) -> NewShip {
    NewShip {
        ship: ShipId(150),
        govt: None,
        ai_type: AiType::Warship.into(),
        lead: None,
        state: ShipState::default(),
        entry: Entry::Placed,
        aggression: 0,
        booty: None,
        info_types: 0,
        person: None,
        mission,
        aux: None,
        derelict: false,
        held: false,
        boarded: false,
        cloaked: false,
        quote: None,
        cargo: None,
        rolls_voice: clears,
        asks_scoop: clears,
        clears_mark: clears,
    }
}

/// A mission mark of slot 0, by `behav`, in formation when `escorts`.
fn mission_mark(behav: ShipBehav, escorts: bool) -> MissionMark {
    MissionMark {
        slot: 0,
        behav,
        formation: escorts.then(FormationSlot::default),
        order: None,
    }
}

/// Brings `ship` into `session`'s traffic, its type's kind added to the
/// spawn table: its NPC.
fn bring(session: &mut Session, ship: NewShip) -> NpcId {
    let record = session.statics.ship_record(ship.ship).expect("a record");
    let kind = session.setup.ship_kind(&session.statics, record);
    session
        .space
        .traffic
        .table_mut()
        .ships
        .insert(ship.ship, kind);
    let came = session.space.traffic.bring_in_within(
        vec![ship],
        SPAWN_RESERVE,
        &Governments::empty(),
        &mut NeverFires,
    );
    assert_eq!(came, 1);
    session.npcs().last().expect("brought in").id
}

/// Whether the warships' Hold stands through the bookkeeping, as the
/// engine reading keeps a kept-placed ship's berth mark and the other
/// reading starts it unmarked.
fn holds(source: RuleSource) -> bool {
    source == RuleSource::Engine
}

#[test]
fn the_reused_slot_mark_reading_is_set_by_the_rulebook_and_outlives_a_new_traffic() {
    let catalog = super::tests::armed();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let mut session = super::tests::facing_an_npc(&catalog, 180).with_rules(reading(source));
        assert_eq!(session.space.traffic.reused_slot_mark(), source);
        let stellar = session.chart.sites[0].id;
        session
            .land_at(&catalog, session.pilot.system, stellar)
            .expect("relocates");
        assert_eq!(session.npcs(), [], "a new traffic");
        assert_eq!(
            session.space.traffic.reused_slot_mark(),
            source,
            "re-applied"
        );
    }
}

#[test]
fn a_mission_ship_on_a_commanded_escorts_berth_captured_as_an_escort_holds_its_classs_order() {
    // (a) `_MissionDudeSpawn` (@0x3cd67) keeps +0xc4, and so does Use As
    // Escort; the bookkeeping's loop 2 (@0x6e3f1) reads it.
    let catalog = carrying();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let (mut session, berth) = commanded_and_gone(&catalog, source);
        let ship = new_warship(false, Some(mission_mark(ShipBehav::Own, false)));
        let id = bring(&mut session, ship);
        assert_eq!(npc_of(&session, id).berth, berth, "the escort's berth");
        session.join_fleet(id, &mut NeverFires);
        assert!(npc_of(&session, id).mission_escort(), "{source:?}");
        assert_eq!(npc_of(&session, id).class, EscortClass::Warship);
        assert_eq!(settle(&mut session), !holds(source), "{source:?}");
        let held = holds(source).then_some(EscortOrder::Hold);
        assert_eq!(kept(&session, EscortClass::Warship), held, "{source:?}");
    }
}

#[test]
fn the_old_ship_of_use_as_my_ship_on_a_commanded_escorts_berth_holds_its_classs_order() {
    // (c) `_DoShipCapture`'s spawn (@0x4114d) keeps +0xc4.
    let catalog = carrying();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let (mut session, berth) = commanded_and_gone(&catalog, source);
        session.pilot.escorts.push(warship(None));
        let id = session
            .place_old_ship(0, ShipState::default(), NpcHold::default(), &mut NeverFires)
            .expect("placed");
        session.space.escorts.fleet = vec![Some(id)];
        assert_eq!(npc_of(&session, id).berth, berth, "the escort's berth");
        assert_eq!(npc_of(&session, id).berth_mark, holds(source), "{source:?}");
        assert_eq!(
            session.space.traffic.berth_mark(berth),
            Some(true),
            "{source:?}: left as it was"
        );
        assert_eq!(settle(&mut session), !holds(source), "{source:?}");
        let held = holds(source).then_some(EscortOrder::Hold);
        assert_eq!(kept(&session, EscortClass::Warship), held, "{source:?}");
    }
}

#[test]
fn a_hire_holds_its_berths_mark_while_landed_and_the_take_off_respawn_clears_it() {
    // (b) `_SpawnEscort` (@0x3fe8f) keeps +0xc4, but the take-off's
    // `_SetupShipsInSystem` respawns the hire (`_RespawnEscort` @0x3d546,
    // from @0x42c79) before any bookkeeping reads it.
    use super::hire::tests::{barred, hireable, landed};
    use super::mission_ships::tests::take_off;
    let catalog = barred(vec![hireable(129, 100)]);
    let mut session = landed(&catalog, 25_000).with_rules(reading(RuleSource::Engine));
    session.space.traffic.write_berth_mark(1, true);
    assert!(session.hire(ShipId(129), &mut NeverFires).is_ok());
    let id = session.space.escorts.fleet[0].expect("placed");
    assert_eq!(npc_of(&session, id).berth, 1);
    assert!(npc_of(&session, id).berth_mark, "the berth's, while landed");
    session.space.escorts.class_orders = [Some(EscortOrder::Hold); 4];
    take_off(&mut session, &catalog);
    let respawned = fleet_npcs(&session);
    assert_eq!(respawned.len(), 1);
    assert!(!respawned[0].berth_mark, "respawned unmarked");
    assert_eq!(
        session.space.traffic.berth_mark(respawned[0].berth),
        Some(false),
        "and its berth cleared"
    );
    assert_eq!(session.space.escorts.class_orders, [None; 4]);
}

#[test]
fn a_traffic_dude_ship_on_a_commanded_escorts_berth_captured_clears_its_classs_order() {
    // (d) `_RandomShipSpawn` clears +0xc4 (@0x3c6ff).
    let catalog = carrying();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let (mut session, berth) = commanded_and_gone(&catalog, source);
        let id = bring(&mut session, new_warship(true, None));
        assert_eq!(npc_of(&session, id).berth, berth, "the escort's berth");
        assert_eq!(session.space.traffic.berth_mark(berth), Some(false));
        session.join_fleet(id, &mut NeverFires);
        assert_eq!(npc_of(&session, id).leader, None, "{source:?}");
        assert!(settle(&mut session), "{source:?}");
        assert_eq!(kept(&session, EscortClass::Warship), None, "{source:?}");
    }
}

#[test]
fn the_fleets_respawn_on_entering_a_system_clears_its_berths_mark() {
    // (e) `_RespawnEscort` clears +0xc4 (@0x3d546).
    let catalog = carrying();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let mut session = flying_by(
            &catalog,
            pilot(&catalog, 2, vec![warship(None)]),
            reading(source),
        );
        session.command_escorts(WARSHIPS, EscortCommand::Hold, &mut NeverFires);
        let berth = fleet_npcs(&session)[0].berth;
        assert_eq!(session.space.traffic.berth_mark(berth), Some(true));
        session.populate(&catalog, &mut NeverFires);
        let respawned = &fleet_npcs(&session)[0];
        assert_eq!(respawned.berth, berth, "its berth again");
        assert!(!respawned.berth_mark, "{source:?}");
        assert_eq!(session.space.traffic.berth_mark(berth), Some(false));
        session.space.escorts.class_orders[2] = Some(EscortOrder::Hold);
        assert!(settle(&mut session), "{source:?}");
        assert_eq!(kept(&session, EscortClass::Warship), None, "{source:?}");
    }
}

#[test]
fn a_fighter_launched_onto_a_marked_berth_starts_unmarked() {
    // (f) `_LaunchFighter` clears +0xc4 (@0x3d9c2).
    let catalog = carrying();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let mut session = flying_by(&catalog, pilot(&catalog, 2, Vec::new()), reading(source));
        for berth in 0..64 {
            session.space.traffic.write_berth_mark(berth, true);
        }
        launch(&mut session, &catalog);
        let fighters = fleet_npcs(&session);
        assert_eq!(fighters.len(), 1, "launched");
        assert!(!fighters[0].berth_mark, "{source:?}");
        assert_eq!(
            session.space.traffic.berth_mark(fighters[0].berth),
            Some(false)
        );
        session.space.escorts.class_orders[0] = Some(EscortOrder::Hold);
        assert_eq!(fighters[0].class, EscortClass::Fighter);
        assert!(settle(&mut session), "{source:?}");
        assert_eq!(kept(&session, EscortClass::Fighter), None, "{source:?}");
    }
}

#[test]
fn a_berth_an_npc_leads_orders_marked_passes_its_mark_to_a_mission_escort_placed_there() {
    // `_AIIssueEscortOrders` sets +0xc4 (@0x89b71) on its follower's slot,
    // set up here as the traffic leaves it; `_MissionDudeSpawn` keeps it.
    let catalog = carrying();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let mut session = flying_by(&catalog, pilot(&catalog, 2, Vec::new()), reading(source));
        let berth = session
            .space
            .traffic
            .free_berth(SPAWN_RESERVE)
            .expect("free");
        session.space.traffic.write_berth_mark(berth, true);
        session.space.escorts.class_orders[2] = Some(EscortOrder::Hold);
        let ship = new_warship(false, Some(mission_mark(ShipBehav::EscortPlayer, true)));
        let id = bring(&mut session, ship);
        assert_eq!(npc_of(&session, id).berth, berth);
        assert!(npc_of(&session, id).mission_escort(), "{source:?}");
        assert_eq!(settle(&mut session), !holds(source), "{source:?}");
        let held = holds(source).then_some(EscortOrder::Hold);
        assert_eq!(kept(&session, EscortClass::Warship), held, "{source:?}");
    }
}

#[test]
fn an_escorts_upgrade_in_the_port_holds_its_berths_mark_by_the_engine() {
    // `_SpawnEscort` (@0x3fe8f) keeps +0xc4; the take-off's respawn then
    // clears it, as the hire's test shows.
    let catalog = carrying();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let mut session = flying_by(
            &catalog,
            pilot(&catalog, 2, vec![warship(None)]),
            reading(source),
        );
        session.command_escorts(WARSHIPS, EscortCommand::Hold, &mut NeverFires);
        let berth = fleet_npcs(&session)[0].berth;
        session.upgrade_escort(0, ShipId(150), &mut NeverFires);
        let upgraded = &fleet_npcs(&session)[0];
        assert_eq!(upgraded.berth, berth, "the same berth");
        assert_eq!(upgraded.berth_mark, holds(source), "{source:?}");
        assert_eq!(session.space.traffic.berth_mark(berth), Some(true));
    }
}
