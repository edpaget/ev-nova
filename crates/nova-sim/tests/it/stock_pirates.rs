//! Stock pirates (`_PirateWarshipAI` @0x8c2d2-0x8c894 and
//! `_AIFindShipToPlunder` @0x7fd10-0x7feb5; see `nova_sim::ai::pirate`
//! and `nova_sim::ai::plunder_target`). Skips, passing, when `NOVA_DATA`
//! is unset.
//!
//! - Sixteen governments have `Flags` 0x1000, so their leaderless
//!   warships think as pirates: the Pirates (137 and 192) and the others
//!   the phase names.
//! - Every stock class's hull carries its `Crew` and `InherentAI`. A
//!   class with no crew, the Cargo Drone (130) or a Wraith (169), is never
//!   plundered; the escape pod (895) has a crew of 1.
//! - The Pirates are xenophobic (`Flags` 0x0001), so their warship takes
//!   an intact Federation Shuttle through the warship's pick, and plunders
//!   it (`InherentAI` 1, a crew of 1), as it plunders one disabled first.

use crate::common;

use nova_data::GameData;
use nova_sim::ai::Surroundings;
use nova_sim::combat::armament::Arsenal;
use nova_sim::geometry::Vec2;
use nova_sim::govt::{Governments, PLUNDERS};
use nova_sim::traffic::npc::{AiType, Npc, NpcId};
use nova_sim::traffic::table::kind;
use nova_sim::{
    Behaviour, Condition, Goal, GovtId, NovaAi, PilotCatalog, Rulebook, ShipId, ShipRef,
};
use nova_sim_base::fixture::Draws;

const FEDERATION: GovtId = GovtId(128);
const PIRATES: GovtId = GovtId(137);
const SHUTTLE: ShipId = ShipId(128);
const PIRATE_STARBRIDGE: ShipId = ShipId(148);

fn opened() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// NPC `id` in berth `id`, of stock ship type `ship` and `govt`, `y`
/// below the centre, as the traffic builds its kind, of its class's AI
/// type.
fn stock_npc(
    data: &GameData,
    arsenal: &Arsenal,
    id: u32,
    (ship, govt): (ShipId, GovtId),
    y: f32,
) -> Npc {
    let record = data
        .ships()
        .into_iter()
        .find(|record| record.id == ship)
        .expect("the ship type reads");
    let kind = kind(&record, &data.outfits(), arsenal);
    let mut npc = nova_sim::traffic::fixture::npc(id, kind.stats);
    npc.berth = id;
    npc.ship = ship;
    npc.hull = kind.hull;
    npc.armament = kind.armament;
    npc.rounds = kind.rounds;
    npc.govt = Some(govt);
    npc.ai_type = AiType::from_raw(record.inherent_ai);
    npc.aggression = 2;
    npc.state.position = Vec2::new(0.0, y);
    npc
}

#[test]
fn sixteen_governments_plunder() {
    let Some(data) = opened() else {
        return;
    };
    let govts = Governments::read(&data);
    let plunderers: Vec<i16> = govts
        .ids()
        .filter(|&id| govts.flag(Some(id), PLUNDERS))
        .map(|id| id.0)
        .collect();
    assert_eq!(
        plunderers,
        [
            131, 133, 134, 137, 149, 167, 169, 170, 171, 176, 177, 178, 181, 188, 192, 194
        ]
    );
}

#[test]
fn every_stock_class_carries_its_crew_and_inherent_ai_on_its_hull() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let outfits = data.outfits();
    let ships = data.ships();
    for record in &ships {
        let hull = kind(record, &outfits, &arsenal).hull;
        assert_eq!(
            (hull.crew, hull.inherent_ai),
            (record.crew, record.inherent_ai),
            "{:?}",
            record.id
        );
    }
    let crew = |id: i16| {
        ships
            .iter()
            .find(|record| record.id == ShipId(id))
            .map(|record| record.crew)
    };
    assert_eq!(crew(130), Some(0), "the Cargo Drone, never plundered");
    assert_eq!(crew(169), Some(0), "a Wraith");
    assert_eq!(crew(895), Some(1), "the escape pod has a crew");
    assert_eq!(crew(128), Some(1), "the Shuttle");
}

#[test]
fn a_stock_pirate_warship_plunders_a_federation_shuttle_disabled_or_intact() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let govts = Governments::read(&data);
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    let pirate = stock_npc(&data, &arsenal, 1, (PIRATE_STARBRIDGE, PIRATES), 0.0);
    assert_eq!(pirate.ai_type, AiType::Warship);
    let shuttle = ShipRef::Npc(NpcId(2));
    let plundered = Goal::Plunder {
        ship: shuttle,
        hopeless: false,
    };
    for condition in [Condition::Disabled, Condition::Intact] {
        let mut trader = stock_npc(&data, &arsenal, 2, (SHUTTLE, FEDERATION), 400.0);
        trader.condition = condition;
        let npcs = [pirate.clone(), trader];
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let decision = ai.decision(&npcs[0], &around, &mut Draws::of(&[0; 4]));
        assert_eq!(decision.goal, plundered, "{condition:?}");
        assert!(!decision.calls_reinforcements, "goal 0xd calls none");
        let plundering = Npc {
            goal: decision.goal,
            ..npcs[0].clone()
        };
        assert_eq!(ai.target(&plundering, &around), Some(shuttle));
    }
}
