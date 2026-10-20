//! Tests of the reinforcement fleet in a session.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::*;
use crate::ai::{Behaviour, CommCall, Decision, Goal, Peaceful, Surroundings, Warship};
use crate::catalog::{
    CommCatalog, FleetId, FleetRecord, GovtRecord, MissionRecord, OutfitId, Penalties, ShipId,
    SystemReinforcements, SystemTraffic,
};
use crate::chance::NeverFires;
use crate::combat::{Rules, ShipRef};
use crate::control::Test;
use crate::flight::Controls;
use crate::govt::WARSHIPS_RETREAT;
use crate::pilot::Pilot;
use crate::testkit::{
    Draws, FAST, FakePilotCatalog, carrying, catalog, govt, jump, mission, npc, outfit, ship, star,
};
use crate::traffic::npc::{AiType, Npc};
use crate::traffic::odds_cache::STALE;

/// The Federation (class 1), whose fleet is system 130's reinforcements.
const FED: GovtId = GovtId(128);
/// Pirates (class 2), at war with the Federation.
const PIRATES: GovtId = GovtId(129);
/// The Federation's allies (class 3), of another government.
const ALLIES: GovtId = GovtId(130);
/// System 130's reinforcement fleet.
const FLEET: FleetId = FleetId(140);
/// Its `ReinfTime`: announced on the call's 31st tick (the timer 9 < 10),
/// arriving on its 40th.
const TIME: i16 = 40;
/// The arriving fleet's quote list.
const QUOTE: i16 = 7600;
/// A global inhibitor (`ModVal` -1).
const JAMMER: OutfitId = OutfitId(441);
/// An inhibitor of class 1, the Federation's.
const FED_JAMMER: OutfitId = OutfitId(442);
/// An inhibitor of class 3, the allies'.
const ALLIED_JAMMER: OutfitId = OutfitId(443);

/// The line the Federation's fleet is announced with.
const SENSORS_DETECT_FEDERATION: &str =
    "Sensors detect Federation reinforcement fleet approaching.";

/// `STR#` 2002 #306 "Sensors detect" and #307 "reinforcement fleet
/// approaching.", and the fleet's quote, "Here we come.".
#[derive(Debug)]
struct Lines;

impl CommCatalog for Lines {
    fn string_list(&self, id: i16) -> Vec<String> {
        match id {
            REINFORCEMENT_LINES => (1..=FLEET_APPROACHING)
                .map(|n| match n {
                    SENSORS_DETECT => "Sensors detect".to_owned(),
                    FLEET_APPROACHING => "reinforcement fleet approaching.".to_owned(),
                    _ => String::new(),
                })
                .collect(),
            QUOTE => vec!["Here we come.".to_owned()],
            _ => Vec::new(),
        }
    }
}

/// No strings at all.
#[derive(Debug)]
struct Empty;

impl CommCatalog for Empty {
    fn string_list(&self, _id: i16) -> Vec<String> {
        Vec::new()
    }
}

/// Systems 130 and 131, linked both ways, with no traffic of their own;
/// 130's reinforcements are [`FLEET`], a Federation ship 128 with its
/// quote, for [`TIME`] ticks every 2 days. The Federation's warships
/// retreat outnumbered (`Flags` 0x0010) and its `MediumName` is
/// "Federation"; the jammers inhibit as their names say.
fn reinforced() -> FakePilotCatalog {
    let quiet = SystemTraffic {
        dude_types: [(-1, 0); 8],
        avg_ships: 0,
        persons: Default::default(),
    };
    let mut catalog = FakePilotCatalog {
        ship_records: vec![ship(128, FAST)],
        govts: vec![
            GovtRecord {
                flags: WARSHIPS_RETREAT,
                classes: [1, -1, -1, -1],
                medium_name: "Federation".to_owned(),
                ..govt(128)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                enemies: [1, -1, -1, -1],
                ..govt(129)
            },
            GovtRecord {
                classes: [3, -1, -1, -1],
                allies: [1, -1, -1, -1],
                ..govt(130)
            },
        ],
        traffic: vec![(SystemId(130), quiet), (SystemId(131), quiet)],
        reinforcements: vec![(
            SystemId(130),
            SystemReinforcements {
                fleet: FLEET,
                time: TIME,
                interval: 2,
            },
        )],
        fleets: vec![FleetRecord {
            id: FLEET,
            lead: Some(ShipId(128)),
            escorts: Vec::new(),
            govt: Some(FED),
            link_syst: 133,
            appear_on: Test::default(),
            quote: Some(QUOTE),
            flags: 0,
        }],
        outfits: vec![
            outfit(441, &[(44, -1)]),
            outfit(442, &[(INHIBITOR, 1)]),
            outfit(443, &[(INHIBITOR, 3)]),
        ],
        ..catalog()
    };
    catalog.star_map[1] = star(131, (600.0, 0.0), &[130, 132]);
    catalog
}

/// A flight of `catalog` with the lines, its system set up and nothing
/// raised yet.
fn flying(catalog: &FakePilotCatalog) -> Session {
    flown(catalog, Pilot::new(catalog, "Ada").expect("a pilot"))
}

/// `pilot`'s flight of `catalog`, as [`flying`].
fn flown(catalog: &FakePilotCatalog, pilot: Pilot) -> Session {
    let mut session = Session::fly(catalog, pilot)
        .expect("flies")
        .with_strings(Rc::new(Lines));
    session.tick_traffic(catalog, &Peaceful, &mut NeverFires);
    let _ = (session.take_messages(), session.take_sounds());
    session
}

/// Adds an NPC of `govt` and `strength` at the player's side, its goal
/// `goal`.
fn add(session: &mut Session, govt: GovtId, strength: f32, goal: Goal) -> NpcId {
    let mut ship = npc(0, session.stats());
    ship.govt = Some(govt);
    ship.hull.strength = strength;
    ship.goal = goal;
    ship.state.position = session.ship.player.position;
    session.space.traffic.add_npc(ship)
}

/// A warship of `govt` and strength 100 fighting a pirate of strength
/// `foe`, which attacks it: the odds against it `foe` / 100, and cached
/// so, as the odds pass would have them.
fn fight(session: &mut Session, govt: GovtId, foe: f32) -> NpcId {
    let warship = add(session, govt, 100.0, Goal::Idle);
    let pirate = add(session, PIRATES, foe, Goal::Attack(ShipRef::Npc(warship)));
    if let Some(npc) = npc_mut(session, warship) {
        npc.goal = Goal::Attack(ShipRef::Npc(pirate));
    }
    session.space.traffic.cache_odds(warship, foe / 100.0);
    warship
}

// The odds a call reads (see `odds_cache`).

#[test]
fn a_call_whose_think_read_stale_odds_brings_nothing_though_the_cache_is_up_by_the_answer() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 150.0);
    session.call_for_reinforcements(warship, false, Some(STALE));
    assert_eq!(timer(&session), 0, "-1 at its think (@0x7f60d)");
    session.call_for_reinforcements(warship, false, Some(1.5));
    assert_eq!(timer(&session), i32::from(TIME), "its think's 1.5");
}

#[test]
fn a_think_at_stale_odds_calls_nothing_and_its_think_after_the_refresh_calls() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 150.0);
    session.space.traffic.cache_odds(warship, STALE);
    let calls = Calling(vec![warship]);
    step(&mut session, &catalog, &calls);
    assert_eq!(timer(&session), 0, "-1 at its think");
    assert_eq!(
        session.space.npc(warship).map(|npc| npc.odds),
        Some(1.5),
        "refreshed after it"
    );
    step(&mut session, &catalog, &calls);
    assert_eq!(
        timer(&session),
        i32::from(TIME) - 1,
        "called, and its first tick"
    );
}

#[test]
fn the_traffic_steps_in_a_world_holding_the_rulebooks_odds_cache() {
    let catalog = reinforced();
    for source in RuleSource::ALL {
        let mut session = flying(&catalog);
        session.setup.rules =
            crate::rulebook::Rulebook::default().with_override(RuleKey::OddsCache, source);
        assert_eq!(session.world().odds_cache, source);
        let mut flown = None;
        session.step_traffic(|_, world| flown = Some(world.odds_cache));
        assert_eq!(flown, Some(source));
    }
}

#[test]
fn a_call_takes_its_thinks_odds_over_the_cache() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 0.0);
    session.call_for_reinforcements(warship, false, Some(1.5));
    assert_eq!(
        timer(&session),
        i32::from(TIME),
        "cached at 0 by the answer"
    );
}

#[test]
fn a_forced_call_reads_the_cache_and_needs_odds_above_0() {
    let catalog = reinforced();
    for (odds, comes) in [(STALE, false), (0.0, false), (0.25, true), (1.5, true)] {
        let mut session = flying(&catalog);
        let warship = fight(&mut session, FED, 150.0);
        session.space.traffic.cache_odds(warship, odds);
        session.call_for_reinforcements(warship, true, None);
        assert_eq!(timer(&session) > 0, comes, "cached at {odds}");
    }
}

#[test]
fn by_the_other_reading_a_call_works_its_odds_out_at_the_answer() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    session.setup.rules =
        crate::rulebook::Rulebook::default().with_override(RuleKey::OddsCache, RuleSource::Bible);
    let warship = fight(&mut session, FED, 150.0);
    session.space.traffic.cache_odds(warship, STALE);
    session.call_for_reinforcements(warship, false, Some(STALE));
    assert_eq!(timer(&session), i32::from(TIME), "1.5 now, stale or not");
    let mut calm = flying(&catalog);
    calm.setup.rules =
        crate::rulebook::Rulebook::default().with_override(RuleKey::OddsCache, RuleSource::Bible);
    let warship = fight(&mut calm, FED, 50.0);
    calm.space.traffic.cache_odds(warship, 0.0);
    calm.call_for_reinforcements(warship, true, None);
    assert_eq!(
        timer(&calm),
        i32::from(TIME),
        "forced at 0.5 now, cached at 0"
    );
    let mut alone = flying(&catalog);
    alone.setup.rules =
        crate::rulebook::Rulebook::default().with_override(RuleKey::OddsCache, RuleSource::Bible);
    let lone = add(&mut alone, FED, 100.0, Goal::Idle);
    alone.space.traffic.cache_odds(lone, 1.5);
    alone.call_for_reinforcements(lone, true, Some(1.5));
    assert_eq!(timer(&alone), 0, "no odds now, though cached and thought");
}

/// NPC `id` of `session`, to change.
fn npc_mut(session: &mut Session, id: NpcId) -> Option<&mut Npc> {
    session
        .space
        .traffic
        .npcs_mut()
        .iter_mut()
        .find(|npc| npc.id == id)
}

/// A traffic tick of `catalog` in which nobody decides anything new: the
/// messages and sounds it raised.
fn ticked(session: &mut Session, catalog: &FakePilotCatalog) -> (Vec<SimMessage>, Vec<SimSound>) {
    session.tick_traffic(catalog, &Keeping, &mut NeverFires);
    (session.take_messages(), session.take_sounds())
}

/// The messages of `ticks` traffic ticks of `catalog`.
fn heard(session: &mut Session, catalog: &FakePilotCatalog, ticks: i16) -> Vec<SimMessage> {
    (0..ticks)
        .flat_map(|_| ticked(session, catalog).0)
        .collect()
}

/// Every NPC keeps its goal and calls nothing.
#[derive(Debug)]
struct Keeping;

impl Behaviour for Keeping {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }
}

/// Every NPC keeps its goal; each listed calls for reinforcements.
#[derive(Debug)]
struct Calling(Vec<NpcId>);

impl Behaviour for Calling {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn decision(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Decision {
        Decision {
            goal: npc.goal,
            calls_reinforcements: self.0.contains(&npc.id),
            ..Decision::default()
        }
    }
}

/// The player's system's timer.
fn timer(session: &Session) -> i32 {
    session.clock_here().timer
}

/// A traffic tick as `calls` decide, then the combat tick of the same
/// step, which settles the calls (see the `comm_calls` module).
fn step(session: &mut Session, catalog: &FakePilotCatalog, calls: &Calling) {
    session.tick_traffic(catalog, calls, &mut NeverFires);
    session.tick_combat(Rules::default(), &mut NeverFires);
}

/// The player's system's countdown.
fn countdown(session: &Session) -> u16 {
    session.clock_here().countdown
}

/// The announcement of the Federation's fleet.
fn announced() -> Vec<SimMessage> {
    vec![SimMessage::ReinforcementsApproaching(
        SENSORS_DETECT_FEDERATION.to_owned(),
    )]
}

#[test]
fn a_think_that_calls_starts_the_timer_and_the_clock_ticks_on_the_same_frame() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    let calls = Calling(vec![warship]);
    for _ in 0..20 {
        step(&mut session, &catalog, &calls);
        if timer(&session) > 0 {
            break;
        }
    }
    assert_eq!(timer(&session), i32::from(TIME) - 1, "its first tick");
}

#[test]
fn an_outnumbered_allied_warships_fleet_is_announced_at_three_quarters_and_arrives_at_its_time() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    assert_eq!(timer(&session), i32::from(TIME));
    for tick in 1..31 {
        assert_eq!(ticked(&mut session, &catalog), (vec![], vec![]), "{tick}");
    }
    let (messages, sounds) = ticked(&mut session, &catalog);
    assert_eq!(messages, announced(), "on the 31st tick, the timer 9");
    assert_eq!(sounds, [SimSound::ReinforcementsApproaching]);
    assert_eq!(countdown(&session), 2, "ReinfIntrval");
    let before = session.npcs().len();
    for tick in 32..40 {
        assert_eq!(ticked(&mut session, &catalog), (vec![], vec![]), "{tick}");
    }
    assert_eq!(session.npcs().len(), before, "not yet");
    let (messages, sounds) = ticked(&mut session, &catalog);
    assert_eq!(
        messages,
        [SimMessage::FleetArrivalQuote("Here we come.".to_owned())],
        "its quote, on the 40th tick"
    );
    assert_eq!(sounds, [SimSound::FleetArrivalQuote]);
    let lead = session.npcs().last().expect("the fleet");
    assert_eq!(
        (lead.ship, lead.govt, lead.ai_type),
        (ShipId(128), Some(FED), AiType::Interceptor)
    );
    assert_eq!(timer(&session), reinforcements::ARRIVED);
    assert_eq!(session.pilot().reinforcement_days[&SystemId(130)], 2);
}

#[test]
fn a_ship_not_outnumbered_calls_nothing_unless_forced_as_battle_help_forces_it() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 50.0);
    session.call_for_reinforcements(warship, false, None);
    assert_eq!(timer(&session), 0, "odds of 0.5, not above half of 1");
    session.call_for_reinforcements(warship, true, None);
    assert_eq!(timer(&session), i32::from(TIME), "forced (@0x9636d)");
    let mut alone = flying(&catalog);
    let lone = add(&mut alone, FED, 100.0, Goal::Idle);
    alone.call_for_reinforcements(lone, true, None);
    assert_eq!(timer(&alone), 0, "no odds at all: not even forced");
    alone.call_for_reinforcements(NpcId(999), true, None);
    assert_eq!(timer(&alone), 0, "a ship not here");
}

#[test]
fn a_ship_of_a_government_not_allied_with_the_fleets_calls_nothing() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let pirate = add(&mut session, PIRATES, 100.0, Goal::Idle);
    add(&mut session, FED, 300.0, Goal::Attack(ShipRef::Npc(pirate)));
    session.call_for_reinforcements(pirate, true, None);
    assert_eq!(timer(&session), 0);
    let allied = fight(&mut session, ALLIES, 300.0);
    session.call_for_reinforcements(allied, false, None);
    assert_eq!(timer(&session), i32::from(TIME), "an ally's");
}

#[test]
fn the_timer_running_blocks_a_second_call() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    ticked(&mut session, &catalog);
    session.call_for_reinforcements(warship, true, None);
    assert_eq!(timer(&session), i32::from(TIME) - 1);
}

/// `session` with its fleet called and arrived.
fn arrived(session: &mut Session, catalog: &FakePilotCatalog, warship: NpcId) {
    session.call_for_reinforcements(warship, false, None);
    heard(session, catalog, TIME);
    assert_eq!(timer(session), reinforcements::ARRIVED);
}

#[test]
fn a_second_call_before_the_interval_brings_none_and_one_after_it_does() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    arrived(&mut session, &catalog, warship);
    session.call_for_reinforcements(warship, false, None);
    assert_eq!(timer(&session), reinforcements::ARRIVED, "2 days to go");
    session.chart.reinforcements_day(&mut session.pilot);
    session.call_for_reinforcements(warship, false, None);
    assert_eq!(timer(&session), reinforcements::ARRIVED, "1 day to go");
    session.chart.reinforcements_day(&mut session.pilot);
    assert_eq!(countdown(&session), 0);
    assert!(session.pilot().reinforcement_days.is_empty(), "none kept");
    session.call_for_reinforcements(warship, false, None);
    assert_eq!(timer(&session), i32::from(TIME), "called again");
}

#[test]
fn a_day_counts_down_only_the_visible_systems() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let mut overlay = session.chart.galaxy.overlay().clone();
    assert!(overlay.set_visible(SystemId(131), Some(false)));
    session.chart.galaxy.set_overlay(overlay);
    session.pilot.reinforcement_days =
        BTreeMap::from([(SystemId(130), 3), (SystemId(131), 2), (SystemId(132), 1)]);
    session.chart.reinforcements_day(&mut session.pilot);
    assert_eq!(
        session.pilot().reinforcement_days,
        BTreeMap::from([(SystemId(130), 2), (SystemId(131), 2)]),
        "131 is hidden (+0x1eb clear, @0xb730)"
    );
}

#[test]
fn the_daily_step_counts_the_countdowns_down() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    session.pilot.reinforcement_days = BTreeMap::from([(SystemId(130), 30)]);
    session.ship.fit.stats.jump_days = 3;
    jump(&mut session, &catalog, 131);
    let left = session
        .pilot()
        .reinforcement_days
        .get(&SystemId(130))
        .copied();
    assert_eq!(left, Some(27), "a jump's three days counted it down");
}

#[test]
fn a_full_ship_table_brings_no_fleet_and_no_quote_but_sets_the_countdown() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    for berth in 3..=55 {
        let filler = npc(0, session.stats());
        session.space.traffic.add_npc_at(filler, berth);
    }
    let crowd = session.npcs().len();
    session.call_for_reinforcements(warship, false, None);
    assert_eq!(heard(&mut session, &catalog, TIME), announced(), "no quote");
    assert_eq!(session.npcs().len(), crowd, "no fleet");
    assert_eq!(countdown(&session), 2);
    assert_eq!(timer(&session), reinforcements::ARRIVED);
}

#[test]
fn a_fleet_government_without_a_medium_name_comes_unannounced() {
    let mut catalog = reinforced();
    catalog.govts[0].medium_name = String::new();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    assert_eq!(heard(&mut session, &catalog, 31), [], "no line (@0x3914d)");
    assert_eq!(countdown(&session), 2, "but the countdown set");
}

#[test]
fn a_missing_entry_of_the_line_reads_as_empty() {
    let catalog = reinforced();
    let mut session = flying(&catalog).with_strings(Rc::new(Empty));
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    assert_eq!(
        heard(&mut session, &catalog, 31),
        [SimMessage::ReinforcementsApproaching(
            " Federation ".to_owned()
        )]
    );
}

/// What a call does with `owned` aboard, the ship of `govt`: the clock it
/// leaves, and the messages of the ticks to the fleet's time.
fn inhibited_by(owned: OutfitId, govt: GovtId) -> (Clock, Vec<SimMessage>) {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    session.pilot.outfits.insert(owned, 1);
    session.reset_precalced();
    let warship = fight(&mut session, govt, 100.0);
    session.call_for_reinforcements(warship, false, None);
    let clock = session.clock_here();
    (clock, heard(&mut session, &catalog, TIME))
}

#[test]
fn each_inhibitor_marks_the_call_with_no_fleet_no_timer_and_no_line() {
    let marked = Clock {
        timer: 0,
        countdown: 1,
    };
    for (owned, govt, why) in [
        (JAMMER, FED, "the global one"),
        (FED_JAMMER, FED, "the ship's and the fleet's government's"),
        (ALLIED_JAMMER, ALLIES, "the ship's government's"),
        (FED_JAMMER, ALLIES, "the fleet's government's"),
    ] {
        assert_eq!(inhibited_by(owned, govt), (marked, vec![]), "{why}");
    }
    let (clock, _) = inhibited_by(ALLIED_JAMMER, FED);
    assert_eq!(clock.timer, i32::from(TIME), "another class's");
}

/// Whether a call for the fleet comes, a day after `JAMMER` was bought,
/// a call marked by it, and the jammer sold, by `source`'s reading of the
/// inhibitor.
fn comes_after_the_sale(source: RuleSource) -> bool {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    session.setup.rules =
        crate::rulebook::Rulebook::default().with_override(RuleKey::ReinforcementInhibitor, source);
    session.pilot.outfits.insert(JAMMER, 1);
    session.reset_precalced();
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    assert_eq!(countdown(&session), 1, "marked");
    session.pilot.outfits.remove(&JAMMER);
    session.reset_precalced();
    session.chart.reinforcements_day(&mut session.pilot);
    session.call_for_reinforcements(warship, false, None);
    timer(&session) > 0
}

#[test]
fn only_a_mod_val_of_minus_1_sets_the_global_inhibitor_and_others_inhibit_their_class() {
    let catalog = reinforced();
    let reset_with = |owned: OutfitId| {
        let mut session = flying(&catalog);
        session.pilot.outfits.insert(owned, 1);
        session.reset_precalced();
        session.space.reinforcements
    };
    let global = reset_with(JAMMER);
    assert!(global.every_government);
    assert!(global.inhibited.is_empty(), "no class of -1 (@0xc66c)");
    let fed = reset_with(FED_JAMMER);
    assert!(!fed.every_government);
    assert_eq!(fed.inhibited, BTreeSet::from([FED]), "class 1");
    assert_eq!(
        reset_with(OutfitId(999)),
        Reinforcements::default(),
        "no record, no mods"
    );
}

#[test]
fn by_the_engine_an_inhibitor_outlives_its_sale_and_by_the_bible_it_does_not() {
    assert!(
        !comes_after_the_sale(RuleSource::Engine),
        "only _InitObjects clears it"
    );
    assert!(comes_after_the_sale(RuleSource::Bible), "worked out afresh");
}

#[test]
fn leaving_the_system_freezes_its_timer_and_returning_resumes_it() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    heard(&mut session, &catalog, 5);
    assert_eq!(timer(&session), i32::from(TIME) - 5);
    jump(&mut session, &catalog, 131);
    heard(&mut session, &catalog, 50);
    assert_eq!(timer(&session), 0, "no fleet called here");
    assert_eq!(
        session.space.reinforcements.timers.get(&SystemId(130)),
        Some(&(i32::from(TIME) - 5)),
        "frozen"
    );
    jump(&mut session, &catalog, 130);
    ticked(&mut session, &catalog);
    assert_eq!(timer(&session), i32::from(TIME) - 6, "resumed");
}

#[test]
fn the_clock_counts_down_through_a_jumps_frames_too() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    session.plot_course(SystemId(131)).expect("a route");
    crate::testkit::fly_out(&mut session);
    crate::testkit::begin_jump_now(&mut session).expect("jumps");
    assert!(session.jumping().is_some());
    ticked(&mut session, &catalog);
    assert_eq!(timer(&session), i32::from(TIME) - 1);
}

#[test]
fn double_times_second_pass_counts_the_clock_down_again() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    session.tick_again(
        Controls::default(),
        &crate::ai::NovaAi::default(),
        Rules::default(),
        &mut NeverFires,
    );
    assert_eq!(timer(&session), i32::from(TIME) - 1);
}

#[test]
fn an_advance_pass_counts_the_clock_down_once() {
    // `_HandleReinforcements` (@0x44504) runs once in each pass of
    // `_DoPlayGameWork`: on the advance pass in the traffic's motion, and
    // not again after its asteroids, as the second pass ticks it.
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    let rescued = session.game_work(
        Controls::default(),
        &catalog,
        &crate::ai::NovaAi::default(),
        Rules::default(),
        &mut NeverFires,
    );
    assert!(!rescued);
    assert_eq!(timer(&session), i32::from(TIME) - 1);
}

#[test]
fn landed_the_clock_stands() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    crate::testkit::land_now(&mut session).expect("lands");
    session.tick_reinforcements(&mut NeverFires);
    assert_eq!(timer(&session), i32::from(TIME));
}

#[test]
fn a_save_keeps_the_countdowns_and_no_timer() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    arrived(&mut session, &catalog, warship);
    let saved = crate::save::encode(session.pilot());
    let pilot = crate::save::decode(&saved).expect("loads");
    let loaded = flown(&catalog, pilot);
    assert_eq!(countdown(&loaded), 2);
    assert_eq!(timer(&loaded), 0, "the timer isn't saved");
}

#[test]
fn the_escape_pods_reset_cancels_the_timer_and_clears_the_countdown_here() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    session.pilot.reinforcement_days = BTreeMap::from([(SystemId(130), 2), (SystemId(131), 3)]);
    session
        .space
        .reinforcements
        .timers
        .insert(SystemId(130), 12);
    session.space.reinforcements.timers.insert(SystemId(131), 7);
    session.space.reset_reinforcements_here(&mut session.pilot);
    assert_eq!(session.clock_here(), Clock::default());
    assert_eq!(
        session.pilot().reinforcement_days,
        BTreeMap::from([(SystemId(131), 3)])
    );
    assert_eq!(
        session.space.reinforcements.timers.get(&SystemId(131)),
        Some(&7)
    );
}

#[test]
fn a_warship_holds_at_odds_below_twice_max_odds_while_its_fleet_comes() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 150.0);
    if let Some(npc) = npc_mut(&mut session, warship) {
        npc.reserves.shield.now = npc.reserves.shield.max * 0.4;
        npc.aggression = 3;
    }
    let decide = |session: &Session| {
        let world = session.world();
        let around = world.around(session.npcs());
        let npc = around.npc(warship).expect("in");
        Warship::default().decide(npc, &around, &mut NeverFires)
    };
    assert!(matches!(decide(&session), Goal::Flee(_)), "odds 1.5 over 1");
    assert_eq!(session.reinforcements_coming(), None);
    session.call_for_reinforcements(warship, false, None);
    assert_eq!(
        session.reinforcements_coming(),
        Some(Coming { govt: Some(FED) })
    );
    let goal = decide(&session);
    assert!(
        !matches!(goal, Goal::Flee(_)),
        "no retreat below twice 1 while the fleet comes: {goal:?}"
    );
}

// The clock held behind the frame's calls held behind a scan (see the
// `comm_calls` module).

/// Every NPC keeps its goal but the scanner, which gives its inspection
/// of the player up; each of `calls` calls for reinforcements and each of
/// `flees` flees the pirate, calling for help.
#[derive(Debug)]
struct ScanFirst {
    scanner: NpcId,
    calls: Vec<NpcId>,
    flees: Vec<(NpcId, NpcId)>,
}

impl Behaviour for ScanFirst {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    fn decision(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Decision {
        let flees = self.flees.iter().find(|(id, _)| *id == npc.id);
        let goal = if npc.id == self.scanner {
            Goal::Idle
        } else if let Some(&(_, pirate)) = flees {
            Goal::Flee(ShipRef::Npc(pirate))
        } else {
            npc.goal
        };
        Decision {
            goal,
            calls_reinforcements: self.calls.contains(&npc.id),
            comm_call: flees.map(|_| CommCall::Distress),
            ..Decision::default()
        }
    }
}

/// Adds an interceptor at the player's side inspecting it, of a
/// government the catalog has no record of, so it is nobody's ally and
/// its scan finds nothing.
fn scanner(session: &mut Session) -> NpcId {
    let scanner = add(session, GovtId(131), 100.0, Goal::Inspect(ShipRef::Player));
    if let Some(npc) = npc_mut(session, scanner) {
        npc.ai_type = AiType::Interceptor;
    }
    scanner
}

/// The scanner inspects the player again, at its side.
fn inspect_again(session: &mut Session, scanner: NpcId) {
    let at = session.ship.player.position;
    if let Some(npc) = npc_mut(session, scanner) {
        npc.goal = Goal::Inspect(ShipRef::Player);
        npc.state.position = at;
    }
}

#[test]
fn a_call_held_behind_a_scan_still_starts_the_timer_the_clock_ticks_on_that_frame() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let scanner = scanner(&mut session);
    let warship = fight(&mut session, FED, 100.0);
    let behaviour = ScanFirst {
        scanner,
        calls: vec![warship],
        flees: Vec::new(),
    };
    for _ in 0..20 {
        inspect_again(&mut session, scanner);
        session.tick_traffic(&catalog, &behaviour, &mut NeverFires);
        assert_eq!(timer(&session), 0, "held behind the scan, no tick yet");
        session.tick_combat(Rules::default(), &mut NeverFires);
        if timer(&session) > 0 {
            break;
        }
    }
    assert_eq!(timer(&session), i32::from(TIME) - 1, "its first tick");
}

/// The police (class 4), scanning for bit 0x0004 and punishing
/// smuggling, its fine 500.
const POLICE: GovtId = GovtId(131);

/// [`reinforced`], with [`POLICE`] and mission 140, its cargo masked
/// 0x0006.
fn policed() -> FakePilotCatalog {
    let mut catalog = reinforced();
    catalog.govts.push(GovtRecord {
        classes: [4, -1, -1, -1],
        scan_mask: 0x0004,
        scan_fine: 500,
        penalties: Penalties {
            smuggle: 5,
            ..Penalties::default()
        },
        ..govt(131)
    });
    catalog.missions.push(MissionRecord {
        scan_mask: 0x0006,
        ..mission(140)
    });
    catalog
}

#[test]
fn a_later_slots_call_is_still_made_after_an_earlier_slots_scan_puts_its_find_on_the_line() {
    let catalog = policed();
    let mut session = flying(&catalog);
    session.pilot.missions.put(0, carrying(140, 4));
    let scanner = add(&mut session, POLICE, 100.0, Goal::Inspect(ShipRef::Player));
    if let Some(npc) = npc_mut(&mut session, scanner) {
        npc.ai_type = AiType::Interceptor;
    }
    let warship = fight(&mut session, FED, 100.0);
    let behaviour = ScanFirst {
        scanner,
        calls: vec![warship],
        flees: Vec::new(),
    };
    for _ in 0..20 {
        inspect_again(&mut session, scanner);
        let mut chance = Draws::of(&[0]);
        session.tick_traffic(&catalog, &behaviour, &mut chance);
        assert_eq!(timer(&session), 0, "held behind the scan, no tick yet");
        session.tick_combat(Rules::default(), &mut chance);
        let (messages, sounds) = (session.take_messages(), session.take_sounds());
        if timer(&session) > 0 {
            assert!(
                matches!(
                    messages.as_slice(),
                    [SimMessage::Smuggling { fine: 500, .. }]
                ),
                "the scan's find on the line: {messages:?}"
            );
            assert_eq!(sounds, [SimSound::Smuggling]);
            assert!(!session.comm_line().free_for_calls(), "the line shows it");
            break;
        }
    }
    assert_eq!(
        timer(&session),
        i32::from(TIME) - 1,
        "the call made behind the find, and the clock's first tick"
    );
}

#[test]
fn a_frames_announcement_comes_after_its_held_distress_call() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    for _ in 1..31 {
        ticked(&mut session, &catalog);
    }
    assert_eq!(timer(&session), 10, "one tick from announcing");
    let scanner = scanner(&mut session);
    let pirate = add(&mut session, PIRATES, 100.0, Goal::Idle);
    let trader = add(&mut session, FED, 100.0, Goal::Idle);
    let behaviour = ScanFirst {
        scanner,
        calls: Vec::new(),
        flees: vec![(trader, pirate)],
    };
    session.tick_traffic(&catalog, &behaviour, &mut NeverFires);
    assert_eq!(session.take_messages(), [], "the call and the clock held");
    assert_eq!(timer(&session), 10);
    session.tick_combat(Rules::default(), &mut NeverFires);
    let messages = session.take_messages();
    assert!(
        matches!(messages.as_slice(), [SimMessage::CallForHelp(_), announcement] if *announcement == announced()[0]),
        "help first: {messages:?}"
    );
    assert_eq!(
        session.take_sounds(),
        [SimSound::CallForHelp, SimSound::ReinforcementsApproaching]
    );
    assert_eq!(timer(&session), 9);
}

#[test]
fn a_held_clock_tick_is_not_lost_when_no_combat_tick_follows() {
    let catalog = reinforced();
    let mut session = flying(&catalog);
    let warship = fight(&mut session, FED, 100.0);
    session.call_for_reinforcements(warship, false, None);
    let scanner = scanner(&mut session);
    let behaviour = ScanFirst {
        scanner,
        calls: Vec::new(),
        flees: Vec::new(),
    };
    session.tick_traffic(&catalog, &behaviour, &mut NeverFires);
    assert_eq!(timer(&session), i32::from(TIME), "held");
    session.tick_traffic(&catalog, &Keeping, &mut NeverFires);
    assert_eq!(timer(&session), i32::from(TIME) - 1, "the held tick ran");
    session.tick_combat(Rules::default(), &mut NeverFires);
    assert_eq!(
        timer(&session),
        i32::from(TIME) - 2,
        "held again behind the scan still queued, then settled"
    );
    session.tick_combat(Rules::default(), &mut NeverFires);
    assert_eq!(timer(&session), i32::from(TIME) - 2, "once only");
    session.tick_traffic(&catalog, &Keeping, &mut NeverFires);
    assert_eq!(timer(&session), i32::from(TIME) - 3, "back in place");
}
