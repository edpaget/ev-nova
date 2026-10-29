//! The reinforcement fleets over the stock data
//! (`_AICallForReinforcements` @0x7f5a1-0x7f71e, `_HandleReinforcements`
//! @0x39048-0x392c3; see `nova_sim::ai::reinforcements` and the session's
//! `reinforcements` module). Skips, passing, when `NOVA_DATA` is unset.
//!
//! - `STR#` 2002 #306 and #307 are the line's words, "Sensors detect" and
//!   "reinforcement fleet approaching." (@0x3916d, @0x39205).
//! - 344 stock systems name one of 28 reinforcement fleets, every one of a
//!   government with a `MediumName`; Kania (128) names fleet 129 for 900
//!   ticks every 2 days.
//! - `oütf` 441, the Transmission Jammer, is `ModType` 44 of `ModVal` -1:
//!   the global inhibitor.
//! - In Kania, a Federation ship outnumbered by a Raven calls the
//!   Federation's fleet: "Sensors detect Federation reinforcement fleet
//!   approaching." shows with `snd ` 154, and 224 ticks later the fleet
//!   comes, its lead, the carrier (`shïp` 143), an interceptor.

use crate::common;

use std::collections::BTreeSet;
use std::rc::Rc;

use nova_data::GameData;
use nova_sim::ai::{Decision, Surroundings};
use nova_sim::catalog::SystemReinforcements;
use nova_sim::traffic::npc::{AiType, Npc};
use nova_sim::{
    Behaviour, Chance, CombatCatalog, CommCatalog, FleetId, Goal, GovtId, OutfitId, Peaceful,
    Pilot, PilotCatalog, Session, ShipId, ShipRef, SimMessage, SimSound, SystemId, TrafficCatalog,
};

const KANIA: SystemId = SystemId(128);
const FEDERATION: GovtId = GovtId(128);
/// The Raven, the stock ship of the greatest `Strength` (1450).
const RAVEN: ShipId = ShipId(310);
/// "Federation Carrier + Escorts", led by `shïp` 143.
const KANIA_FLEET: FleetId = FleetId(129);

fn opened() -> Option<Rc<GameData>> {
    let dir = common::nova_data()?;
    Some(Rc::new(
        GameData::open(&dir, None).expect("the stock data opens"),
    ))
}

#[test]
fn str_2002_306_and_307_are_the_reinforcement_lines_words() {
    let Some(data) = opened() else {
        return;
    };
    let strings = CommCatalog::string_list(data.as_ref(), 2002);
    assert_eq!(
        strings.get(305..307),
        Some(
            &[
                "Sensors detect".to_owned(),
                "reinforcement fleet approaching.".to_owned()
            ][..]
        )
    );
}

#[test]
fn every_stock_reinforcement_fleet_is_of_a_government_with_a_medium_name() {
    let Some(data) = opened() else {
        return;
    };
    let kept: Vec<(SystemId, SystemReinforcements)> = data
        .star_map()
        .iter()
        .filter_map(|system| Some((system.id, data.system_reinforcements(system.id)?)))
        .collect();
    let fleets: BTreeSet<FleetId> = kept.iter().map(|(_, kept)| kept.fleet).collect();
    assert_eq!((kept.len(), fleets.len()), (344, 28));
    let records = data.fleets();
    let govts = data.governments();
    for fleet in &fleets {
        let record = records
            .iter()
            .find(|record| record.id == *fleet)
            .unwrap_or_else(|| panic!("flët {fleet:?} has a record"));
        let govt = record.govt.expect("of a government");
        let named = govts
            .iter()
            .find(|record| record.id == govt)
            .is_some_and(|record| !record.medium_name.is_empty());
        assert!(named, "{fleet:?}'s government {govt:?} has a MediumName");
    }
    assert!(
        kept.iter().all(|(_, kept)| kept.interval >= 1),
        "an interval of 0 reads as 1"
    );
    assert_eq!(
        data.system_reinforcements(KANIA),
        Some(SystemReinforcements {
            fleet: KANIA_FLEET,
            time: 900,
            interval: 2,
        })
    );
}

#[test]
fn outfit_441_the_transmission_jammer_is_the_global_inhibitor() {
    let Some(data) = opened() else {
        return;
    };
    let jammer = data
        .outfits()
        .into_iter()
        .find(|outfit| outfit.id == OutfitId(441))
        .expect("oütf 441");
    assert!(jammer.mods.contains(&(44, -1)), "{:?}", jammer.mods);
}

/// A seeded source of draws: a 64-bit linear congruential generator.
struct Seeded(u64);

impl Chance for Seeded {
    fn fires(&mut self, percent: u8) -> bool {
        self.below(100) < u32::from(percent)
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        u32::try_from((self.0 >> 33) % u64::from(n)).unwrap_or(0)
    }
}

/// Every Federation ship attacks the player and calls for its
/// reinforcements on every think; every other ship keeps its goal.
#[derive(Debug)]
struct Outnumbered;

impl Behaviour for Outnumbered {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    fn decision(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Decision {
        if npc.govt == Some(FEDERATION) {
            Decision {
                goal: Goal::Attack(ShipRef::Player),
                calls_reinforcements: true,
                ..Decision::default()
            }
        } else {
            Decision {
                goal: npc.goal,
                ..Decision::default()
            }
        }
    }
}

/// A Raven flying in Kania whose traffic, drawn on `chance`, holds a
/// Federation ship.
fn in_kania(data: &Rc<GameData>, chance: &mut Seeded) -> Option<Session> {
    let mut pilot = Pilot::new(data.as_ref(), "").expect("the stock first chär starts");
    pilot.system = KANIA;
    pilot.stellar = None;
    pilot.ship = RAVEN;
    let mut session = Session::fly(data.as_ref(), pilot)
        .expect("flies")
        .with_strings(Rc::clone(data) as Rc<dyn CommCatalog>);
    crate::passes::work_with(
        &mut session,
        data.as_ref(),
        nova_sim::Controls::default(),
        &Peaceful,
        nova_sim::Rules::default(),
        chance,
    );
    let federal = session
        .npcs()
        .iter()
        .any(|npc| npc.govt == Some(FEDERATION));
    federal.then_some(session)
}

#[test]
fn a_federation_ship_outnumbered_in_kania_calls_its_fleet_which_comes_after_its_line() {
    let Some(data) = opened() else {
        return;
    };
    let (mut session, mut chance) = (0..64)
        .find_map(|seed| {
            let mut chance = Seeded(seed);
            in_kania(&data, &mut chance).map(|session| (session, chance))
        })
        .expect("a seed brings a Federation ship to Kania");
    let _ = common::drained(&mut session);
    let carrier = |session: &Session| {
        session
            .npcs()
            .iter()
            .any(|npc| npc.ship == ShipId(143) && npc.ai_type == AiType::Interceptor)
    };
    let line = SimMessage::ReinforcementsApproaching(
        "Sensors detect Federation reinforcement fleet approaching.".to_owned(),
    );
    let mut announced = None;
    let mut arrived = None;
    for tick in 0..2000_u32 {
        // The pass settles its frame's calls (see the session's
        // `comm_calls` module).
        crate::passes::work_with(
            &mut session,
            data.as_ref(),
            nova_sim::Controls::default(),
            &Outnumbered,
            nova_sim::Rules::default(),
            &mut chance,
        );
        let events = common::drained(&mut session);
        let (messages, sounds) = (common::messages(&events), common::sounds(&events));
        if messages.contains(&line) {
            assert_eq!(announced, None, "announced once");
            assert!(sounds.contains(&SimSound::ReinforcementsApproaching));
            announced = Some(tick);
        }
        if arrived.is_none() && carrier(&session) {
            arrived = Some(tick);
            break;
        }
    }
    let announced = announced.expect("the line shows");
    let arrived = arrived.expect("the fleet comes");
    assert_eq!(arrived - announced, 224, "from the timer's 224 to 0");
}
