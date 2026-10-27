//! Tests of a landed pass of the game's work (`Session::landed_work`),
//! each through [`Session::game_work`] with the session landed: one for
//! each piece of the work, in the order the pass runs them, so leaving
//! any piece out fails its test.
//!
//! Some state a landed pass meets can't be reached by a landing alone (a
//! gate's pending entry, a jump accepted, an explosion of the last pass);
//! those tests set it by hand on the landed session, as the pass would
//! find it.

use super::Session;
use super::fighters::tests::fleet;
use super::reinforcements::tests::{FED, TIME, fight, reinforced, timer};
use super::wait_for_me::tests::{DART, fighter};
use crate::catalog::{AsteroidFieldRecord, AsteroidTypeRecord, ShipId, StellarId, SystemId};
use crate::chance::NeverFires;
use crate::combat::{Rules, ShipRef};
use crate::flight::Controls;
use crate::message::SimMessage;
use crate::sound::SimSound;
use crate::testkit::{FakePilotCatalog, Passive, catalog, land_now, npc, work};
use crate::traffic::on_line::OnLine;

/// One pass of `catalog`'s session's game work, the traffic keeping its
/// goals ([`Passive`]), the fight by the default rules, no chance firing:
/// whether a rescue ended it.
fn pass(session: &mut Session, catalog: &FakePilotCatalog) -> bool {
    session.game_work(
        Controls::default(),
        catalog,
        &Passive,
        Rules::default(),
        &mut NeverFires,
    )
}

/// `catalog`'s session started in flight, then landed.
fn landed(catalog: &FakePilotCatalog) -> Session {
    let mut session = Session::start(catalog).expect("starts");
    land_now(&mut session).expect("lands");
    session
}

#[test]
fn a_landed_pass_ends_a_gates_pending_entry() {
    // Any pass ends it first (see the `game_work` module).
    let catalog = catalog();
    let mut session = landed(&catalog);
    session.ship.warp.gate = Some(StellarId(128));
    assert!(!pass(&mut session, &catalog));
    assert_eq!(session.ship.warp.gate, None);
}

#[test]
fn a_rescue_due_while_landed_ends_the_pass_before_the_rest_of_its_work() {
    // `rescue_if_due` settles, and the pass says so and stops: the
    // headings drawn, which its later work drops, are still there.
    let catalog = catalog();
    let mut session = landed(&catalog);
    session.ship.fate.rescue_due = true;
    session.space.drawn.insert(ShipRef::Player, 90.0);
    assert!(pass(&mut session, &catalog), "the rescue ended the pass");
    assert!(
        session.space.drawn.contains_key(&ShipRef::Player),
        "the rest of the work never ran"
    );
}

#[test]
fn a_landed_pass_warns_of_fighters_left_behind_by_a_new_jump() {
    // `_HandlePlayer`'s warning (see the `wait_for_me` module): a fighter
    // of no fuel is out as the player lands, and the jump's mark is set.
    let catalog = super::wait_for_me::tests::catalog();
    let mut session = fleet(&catalog, 0, vec![fighter(DART)]);
    land_now(&mut session).expect("lands");
    let _ = (session.take_messages(), session.take_sounds());
    session.ship.warp.jump_accepted = true;
    assert!(!pass(&mut session, &catalog));
    assert_eq!(
        session.take_messages(),
        [SimMessage::WaitForMe("Ship 145:  ".to_owned())]
    );
    assert!(session.take_sounds().contains(&SimSound::WaitForMe));
    assert!(!pass(&mut session, &catalog));
    assert_eq!(session.take_messages(), [], "once a jump");
}

#[test]
fn a_landed_pass_spends_a_held_reinforcement_tick_and_the_clock_stands() {
    // The clock's tick held behind the last pass's work on the line is
    // spent, and stands as the landed clock does (see the
    // `reinforcements` module).
    let catalog = reinforced();
    let mut session = super::reinforcements::tests::flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    land_now(&mut session).expect("lands");
    session.space.reinforcements_held = true;
    assert!(!pass(&mut session, &catalog));
    assert!(!session.space.reinforcements_held, "spent");
    assert_eq!(timer(&session), i32::from(TIME), "landed, the clock stands");
}

#[test]
fn a_landed_pass_lets_go_of_a_target_gone_from_the_system() {
    // The traffic's tail (`Session::traffic_tail`) lets go of it.
    let catalog = catalog();
    let mut session = landed(&catalog);
    let id = session.space.traffic.add_npc(npc(0, session.stats()));
    session.chart.target = Some(id);
    session.space.traffic.remove(id);
    assert!(!pass(&mut session, &catalog));
    assert_eq!(session.chart.target, None);
}

#[test]
fn a_landed_pass_forgets_the_last_passes_explosion() {
    let catalog = catalog();
    let mut session = landed(&catalog);
    session.ship.fate.player_exploded_this_tick = true;
    assert!(!pass(&mut session, &catalog));
    assert!(!session.ship.fate.player_exploded_this_tick);
}

#[test]
fn nothing_on_the_line_settles_while_landed_and_the_clocks_tick_stays_held() {
    // Nothing on the line settles landed (`Session::settle_line_queue`
    // returns at once there, so a landed pass doesn't call it): a
    // refuel's note queued waits, unsaid, and the traffic's tail holds
    // the reinforcement clock's tick behind it.
    let catalog = catalog();
    let mut session = landed(&catalog);
    let id = session.space.traffic.add_npc(npc(0, session.stats()));
    session
        .space
        .traffic
        .queue_on_line(id, OnLine::RefuelDone(id, ShipId(128)));
    let _ = (session.take_messages(), session.take_sounds());
    assert!(!pass(&mut session, &catalog));
    assert!(session.space.traffic.holds_line_work(), "still queued");
    assert!(session.space.reinforcements_held, "held behind it");
    assert!(
        !session
            .take_sounds()
            .contains(&SimSound::EnergyTransferComplete)
    );
}

#[test]
fn a_landed_pass_drops_the_headings_the_ships_were_drawn_at() {
    // The last flight pass's headings, which a landing keeps.
    let catalog = catalog();
    let mut session = Session::start(&catalog).expect("starts");
    work(&mut session, &catalog, Controls::default());
    land_now(&mut session).expect("lands");
    assert!(!session.space.drawn.is_empty(), "drawn in flight");
    assert!(!pass(&mut session, &catalog));
    assert!(session.space.drawn.is_empty());
}

/// [`catalog`] with system 130 holding 4 asteroids of type 0.
fn rocky() -> FakePilotCatalog {
    FakePilotCatalog {
        asteroid_types: vec![AsteroidTypeRecord {
            kind: 0,
            spin_rate: 100,
            frames: 36,
            frame_size: Some((40, 30)),
            ..AsteroidTypeRecord::default()
        }],
        asteroid_fields: vec![AsteroidFieldRecord {
            system: SystemId(130),
            count: 4,
            types: 0x0001,
        }],
        ..catalog()
    }
}

#[test]
fn a_landed_pass_spends_the_landings_asteroid_kill() {
    // A landing kills the field through its frame (see the `asteroids`
    // module): here the take-off's first, -14. The landed pass's asteroid
    // work spends the kill, so the next take-off's first frame, -14
    // again, doesn't meet it and spawns.
    let catalog = rocky();
    let mut session = Session::start(&catalog).expect("starts");
    session.set_window_width(640);
    land_now(&mut session).expect("lands");
    session
        .take_off(&catalog, &mut NeverFires)
        .expect("takes off");
    work(&mut session, &catalog, Controls::default());
    land_now(&mut session).expect("lands again");
    assert!(!pass(&mut session, &catalog));
    session
        .take_off(&catalog, &mut NeverFires)
        .expect("takes off");
    work(&mut session, &catalog, Controls::default());
    assert_eq!(session.space.pass_frame, -14, "the landing's frame again");
    assert_eq!(session.asteroids().count(), 1, "spawned, not killed");
}
