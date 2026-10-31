//! The session's tests of the voice variant a ship-table slot keeps
//! (see [`berth_voices`](crate::traffic::berth_voices)): which ships hold
//! their berth's word, which write their own, and how
//! [`RuleKey::ReusedSlotVoice`] and the kept berths reach the traffic.

use super::fighters::tests::{fleet_npcs, flying_by, pilot, sitting_ai};
use super::tests::{armed, boardable, captured, facing_an_npc};
use super::voices::tests::{MEDIUM, MEDIUM_EVEN, VIPER, escort, escort_npc, fleet, heard, voiced};
use super::*;
use crate::ai::Peaceful;
use crate::catalog::SystemId;
use crate::chance::NeverFires;
use crate::combat::Rules;
use crate::flight::Controls;
use crate::rulebook::{RuleSource, Rulebook};
use crate::session::capture_naming::tests::use_as_my_ship;
use crate::stats::ShipStats;
use crate::testkit::{FAST, FakePilotCatalog, Ranged, jump};
use crate::traffic::npc::NpcId;
use crate::voice::VoiceVariant;

/// Every berth a test's ships may take.
const BERTHS: std::ops::Range<u32> = 0..16;

/// The rulebook with `reading` of [`RuleKey::ReusedSlotVoice`].
fn reading(reading: RuleSource) -> Rulebook {
    Rulebook::default().with_override(RuleKey::ReusedSlotVoice, reading)
}

/// The session's traffic with every berth holding `variant`.
fn every_berth(session: &mut Session, variant: VoiceVariant) {
    for berth in BERTHS {
        session.space.traffic.write_berth_voice(berth, variant);
    }
}

/// A plain ship brought into `session` at `berth`: its voice.
fn next_ship_at(session: &mut Session, berth: u32) -> Option<VoiceVariant> {
    let ship = crate::testkit::npc(0, ShipStats::new(FAST, &[]));
    let id = session.space.traffic.add_npc_at(ship, berth);
    session.space.npc(id).expect("in the system").voice
}

/// Takes NPC `id` out of `session`: the berth it leaves.
fn vacate(session: &mut Session, id: NpcId) -> u32 {
    let berth = session.space.npc(id).expect("in the system").berth;
    session.space.traffic.remove(id);
    berth
}

/// `catalog`'s session with no fleet, `hired` then placed as the bar
/// places one, with every berth holding `word`: its id.
fn hired(
    catalog: &FakePilotCatalog,
    rules: Rulebook,
    ship: crate::catalog::ShipId,
    word: VoiceVariant,
) -> (Session, NpcId) {
    let mut session = flying_by(catalog, pilot(catalog, 0, Vec::new()), rules);
    every_berth(&mut session, word);
    session.pilot.escorts.push(escort(ship));
    let id = session.place_escort(0, &mut NeverFires).expect("placed");
    session.space.escorts.fleet.push(Some(id));
    (session, id)
}

#[test]
fn a_hired_escort_speaks_its_berths_word_and_is_not_rolled_at_its_first_think() {
    let catalog = voiced();
    // A class forced even holds its odd word: the roll that forces is dead.
    for ship in [MEDIUM, MEDIUM_EVEN] {
        let (mut session, id) = hired(&catalog, Rulebook::default(), ship, VoiceVariant::Odd);
        let voice_of = |session: &Session| session.space.npc(id).expect("there").voice;
        assert_eq!(voice_of(&session), Some(VoiceVariant::Odd), "{ship:?}");
        let mut chance = Ranged::of(&[(2, 0)]);
        heard(&mut session, &catalog, &sitting_ai(), &mut chance);
        assert_eq!(voice_of(&session), Some(VoiceVariant::Odd), "{ship:?}");
        assert!(!chance.was_asked(2), "{ship:?}: not rolled");
    }
}

#[test]
fn a_hired_escort_in_a_never_used_berth_speaks_even() {
    let catalog = voiced();
    let session = fleet(&catalog, &[MEDIUM]);
    assert_eq!(fleet_npcs(&session)[0].voice, Some(VoiceVariant::Even));
}

#[test]
fn by_the_other_reading_a_hired_escort_rolls_at_its_first_think_forced_included() {
    let catalog = voiced();
    for (ship, rolled) in [
        (MEDIUM, VoiceVariant::Odd),
        (MEDIUM_EVEN, VoiceVariant::Even),
    ] {
        let (mut session, _) = hired(
            &catalog,
            reading(RuleSource::Bible),
            ship,
            VoiceVariant::Even,
        );
        assert_eq!(escort_npc(&mut session, 0).voice, None, "{ship:?}");
        heard(
            &mut session,
            &catalog,
            &sitting_ai(),
            &mut Ranged::of(&[(2, 1)]),
        );
        assert_eq!(escort_npc(&mut session, 0).voice, Some(rolled), "{ship:?}");
    }
}

#[test]
fn an_escort_with_no_npc_keeps_its_seed_through_an_entry() {
    // As a pilot's first entry places its fleet: nothing kept from an NPC.
    let catalog = voiced();
    let mut session =
        Session::fly(&catalog, pilot(&catalog, 0, vec![escort(MEDIUM)])).expect("flies");
    every_berth(&mut session, VoiceVariant::Odd);
    session.populate(&catalog, &mut NeverFires);
    assert_eq!(escort_npc(&mut session, 0).voice, Some(VoiceVariant::Odd));
    assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
    assert_eq!(escort_npc(&mut session, 0).voice, Some(VoiceVariant::Odd));
}

#[test]
fn an_upgraded_escorts_voice_stays_in_its_berth() {
    // By the other reading the berth seeds none, so only the write shows.
    let catalog = voiced();
    let (mut session, id) = hired(
        &catalog,
        reading(RuleSource::Bible),
        MEDIUM,
        VoiceVariant::Even,
    );
    session.space.traffic.set_voice(id, VoiceVariant::Odd);
    assert_eq!(escort_npc(&mut session, 0).voice, Some(VoiceVariant::Odd));
    session.upgrade_escort(0, MEDIUM_EVEN, &mut NeverFires);
    assert_eq!(escort_npc(&mut session, 0).ship, MEDIUM_EVEN);
    assert_eq!(escort_npc(&mut session, 0).voice, Some(VoiceVariant::Odd));
    let id = session.space.escorts.fleet[0].expect("placed");
    let berth = vacate(&mut session, id);
    session
        .space
        .traffic
        .set_reused_slot_voice(RuleSource::Engine);
    assert_eq!(next_ship_at(&mut session, berth), Some(VoiceVariant::Odd));
}

#[test]
fn a_captures_roll_leaves_its_word_for_the_berths_next_ship() {
    use crate::session::Assignment;
    let catalog = boardable();
    let mut session = captured(&catalog);
    let id = session.space.escorts.aboard.expect("aboard").npc;
    every_berth(&mut session, VoiceVariant::Even);
    session.assign(Assignment::Escort, &mut Ranged::of(&[(2, 1)]));
    assert_eq!(
        session.space.npc(id).expect("joined").voice,
        Some(VoiceVariant::Odd)
    );
    let berth = vacate(&mut session, id);
    assert_eq!(next_ship_at(&mut session, berth), Some(VoiceVariant::Odd));
}

#[test]
fn the_old_ships_roll_leaves_its_word_for_the_berths_next_ship() {
    let catalog = boardable();
    let mut session = captured(&catalog);
    every_berth(&mut session, VoiceVariant::Even);
    use_as_my_ship(&mut session, "Prize", &mut Ranged::of(&[(2, 1)]));
    let old = fleet_npcs(&session).pop().expect("the old ship kept");
    assert_eq!(old.voice, Some(VoiceVariant::Odd));
    let berth = vacate(&mut session, old.id);
    assert_eq!(next_ship_at(&mut session, berth), Some(VoiceVariant::Odd));
}

#[test]
fn a_launched_fighters_roll_leaves_its_word_for_the_berths_next_ship() {
    let mut catalog = voiced();
    for record in &mut catalog.ship_records {
        if record.id == VIPER {
            record.inherent_govt = Some(crate::catalog::GovtId(140));
        }
    }
    let mut session = flying_by(
        &catalog,
        pilot(&catalog, 1, Vec::new()),
        Rulebook::default(),
    );
    session.hold_fire(false, true);
    session.game_work(
        Controls::default(),
        &catalog,
        &Peaceful,
        Rules::default(),
        &mut Ranged::of(&[(2, 1)]),
    );
    let fighter = fleet_npcs(&session).pop().expect("launched");
    assert_eq!(fighter.voice, Some(VoiceVariant::Odd));
    let berth = vacate(&mut session, fighter.id);
    assert_eq!(next_ship_at(&mut session, berth), Some(VoiceVariant::Odd));
}

#[test]
fn a_ship_the_traffic_draws_may_be_odd_though_its_class_forces_even() {
    let mut catalog = crate::session::tests::trafficked(130, 1, 3);
    catalog.voices = voiced().voices;
    for record in &mut catalog.ship_records {
        record.inherent_govt = Some(crate::catalog::GovtId(141));
    }
    let mut session = Session::start(&catalog).expect("starts");
    // `NeverFires` draws the top of every range: a Rand(2) of 1.
    session.populate(&catalog, &mut NeverFires);
    let [ship] = session.npcs() else {
        panic!("one ship: {:?}", session.npcs());
    };
    assert_eq!(ship.voice, Some(VoiceVariant::Odd));
}

#[test]
fn a_launched_fighters_record_keeps_its_voice() {
    let mut catalog = voiced();
    for record in &mut catalog.ship_records {
        if record.id == VIPER {
            record.inherent_govt = Some(crate::catalog::GovtId(140));
        }
    }
    let mut session = flying_by(
        &catalog,
        pilot(&catalog, 1, Vec::new()),
        Rulebook::default(),
    );
    session.hold_fire(false, true);
    session.game_work(
        Controls::default(),
        &catalog,
        &Peaceful,
        Rules::default(),
        &mut Ranged::of(&[(2, 1)]),
    );
    assert_eq!(session.pilot.escorts[0].voice, Some(VoiceVariant::Odd));
}

/// A session of `escorts` loaded with every berth holding `word`, its fleet
/// placed: escort 0's voice.
fn loaded_voice(escorts: Vec<Escort>, word: VoiceVariant) -> Option<VoiceVariant> {
    let catalog = voiced();
    let mut session = Session::fly(&catalog, pilot(&catalog, 0, escorts)).expect("flies");
    every_berth(&mut session, word);
    session.populate(&catalog, &mut NeverFires);
    escort_npc(&mut session, 0).voice
}

#[test]
fn a_loaded_fighter_speaks_its_saved_variant_over_its_berths_word() {
    let fighter = Escort {
        carried: true,
        voice: Some(VoiceVariant::Odd),
        ..escort(VIPER)
    };
    assert_eq!(
        loaded_voice(vec![fighter], VoiceVariant::Even),
        Some(VoiceVariant::Odd)
    );
}

#[test]
fn a_loaded_fighter_with_no_saved_variant_takes_its_berths_word() {
    let fighter = Escort {
        carried: true,
        ..escort(VIPER)
    };
    assert_eq!(
        loaded_voice(vec![fighter], VoiceVariant::Odd),
        Some(VoiceVariant::Odd)
    );
}

#[test]
fn a_loaded_hired_escort_speaks_its_berths_word_not_its_records() {
    let hired = Escort {
        voice: Some(VoiceVariant::Even),
        ..escort(MEDIUM)
    };
    assert_eq!(
        loaded_voice(vec![hired], VoiceVariant::Odd),
        Some(VoiceVariant::Odd)
    );
}

#[test]
fn a_loaded_fighters_variant_is_left_in_its_berth() {
    let catalog = voiced();
    let fighter = Escort {
        carried: true,
        voice: Some(VoiceVariant::Odd),
        ..escort(VIPER)
    };
    let mut session = Session::fly(&catalog, pilot(&catalog, 0, vec![fighter])).expect("flies");
    session.populate(&catalog, &mut NeverFires);
    let id = session.space.escorts.fleet[0].expect("placed");
    let berth = vacate(&mut session, id);
    assert_eq!(next_ship_at(&mut session, berth), Some(VoiceVariant::Odd));
}

#[test]
fn the_reused_slot_voice_reading_is_set_by_the_rulebook_and_outlives_a_new_traffic() {
    let catalog = armed();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let mut session = facing_an_npc(&catalog, 180).with_rules(reading(source));
        assert_eq!(session.space.traffic.reused_slot_voice(), source);
        let stellar = session.chart.sites[0].id;
        session
            .land_at(&catalog, session.pilot.system, stellar)
            .expect("relocates");
        assert_eq!(session.npcs(), [], "a new traffic");
        assert_eq!(
            session.space.traffic.reused_slot_voice(),
            source,
            "re-applied"
        );
    }
}

#[test]
fn a_word_outlasts_a_traffic_reset() {
    let catalog = armed();
    let mut session = facing_an_npc(&catalog, 180);
    session
        .space
        .traffic
        .write_berth_voice(5, VoiceVariant::Odd);
    let stellar = session.chart.sites[0].id;
    session
        .land_at(&catalog, session.pilot.system, stellar)
        .expect("relocates");
    assert_eq!(next_ship_at(&mut session, 5), Some(VoiceVariant::Odd));
}

#[test]
fn a_word_reaches_a_new_session_through_the_kept_berths() {
    let catalog = armed();
    let mut old = facing_an_npc(&catalog, 180);
    old.space.traffic.write_berth_voice(5, VoiceVariant::Odd);
    let mut new = facing_an_npc(&catalog, 180).with_kept_berths(old.kept_berths());
    assert_eq!(next_ship_at(&mut new, 5), Some(VoiceVariant::Odd));
}
