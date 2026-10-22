//! Whom a stock warship targets (`_SelectWarshipTarget` @0x89d5e-0x8b01e
//! and `_AIShipFriendStrength` @0x825a8; see `nova_sim::ai::target`,
//! `nova_sim::ai::candidates` and `nova_sim::ai::odds`). Skips, passing,
//! when `NOVA_DATA` is unset.
//!
//! - `oütf` 442, the Federation IFF Projector, is `ModType` 48 of
//!   `ModVal` 1 and marks the governments of class 1 (128, 136, 146, 151,
//!   154, 186); 443, the Moash IFF Projector, of 3 marks 131 and 188. None
//!   of them is xenophobic, so no stock xenophobe ever keeps a player its
//!   IFF cleared.
//! - The Fed Destroyer (`shïp` 141) fights as the Federation, so a pilot
//!   flying it counts the Federation's ships as its friends, and a
//!   Rebellion warship (at war with the Federation) turns on it at 1 in 50.
//! - The Bureau (164) has `Flags` 0x0004: its warship at home leaves a
//!   pilot in good standing out of its target step and turns on it by its
//!   government's word alone.
//! - Only the escape pod (`shïp` 895) holds no shield, and no xenophobe,
//!   a Pirate (137) for one, takes a player in it.

use crate::common;

use std::collections::BTreeSet;

use nova_data::GameData;
use nova_sim::ai::odds::friend_strength;
use nova_sim::ai::target::{TargetRules, select_target};
use nova_sim::ai::{PlayerSide, Surroundings, Warship};
use nova_sim::combat::armament::Arsenal;
use nova_sim::flight::ShipState;
use nova_sim::geometry::Vec2;
use nova_sim::govt::Governments;
use nova_sim::rank::Blessing;
use nova_sim::stats::ShipStats;
use nova_sim::traffic::npc::{AiType, Npc, NpcId};
use nova_sim::{
    Behaviour, Condition, Goal, GovtId, NovaAi, Pilot, PilotCatalog, Session, ShipId, ShipRef,
};
use nova_sim_base::fixture::Draws;

const P: ShipRef = ShipRef::Player;
const FEDERATION: GovtId = GovtId(128);
const REBELLION: GovtId = GovtId(141);
const BUREAU: GovtId = GovtId(164);
const PIRATES: GovtId = GovtId(137);
const SHUTTLE: ShipId = ShipId(128);
const FED_DESTROYER: ShipId = ShipId(141);
const ESCAPE_POD: ShipId = ShipId(895);

fn opened() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// NPC `id`, a warship of stock ship type `ship` and `govt` at (`x`, `y`),
/// as the traffic arms it, of aggression 2.
fn stock_warship(
    arsenal: &Arsenal,
    id: u32,
    ship: ShipId,
    govt: GovtId,
    (x, y): (f32, f32),
) -> Npc {
    let mut npc = nova_sim::traffic::fixture::npc(id, ShipStats::default());
    let (armament, rounds) = arsenal.npc(ship);
    npc.ship = ship;
    npc.armament = armament;
    npc.rounds = rounds;
    npc.hull = arsenal.hull(ship);
    npc.govt = Some(govt);
    npc.ai_type = AiType::Warship;
    npc.aggression = 2;
    npc.reserves = nova_sim::Reserves::full(100.0, 100.0, 100.0);
    npc.state.position = Vec2::new(x, y);
    npc
}

/// The player flying stock ship type `ship` at (`x`, `y`), its shield at a
/// quarter of the class's or less (a shield factor of 0.25), as its
/// record says it fights.
fn stock_player(
    data: &GameData,
    arsenal: &Arsenal,
    ship: ShipId,
    (x, y): (f32, f32),
) -> PlayerSide {
    let record = data
        .ships()
        .into_iter()
        .find(|record| record.id == ship)
        .expect("the ship type reads");
    PlayerSide {
        state: ShipState {
            position: Vec2::new(x, y),
            ..ShipState::default()
        },
        condition: Condition::Intact,
        reserves: nova_sim::Reserves {
            shield: nova_sim::Gauge {
                now: 1.0,
                max: 1000.0,
            },
            ..nova_sim::Reserves::full(1000.0, 1000.0, 100.0)
        },
        hull: arsenal.hull(ship),
        handling: ShipStats::default().handling,
        rating: 0,
        yardstick: 0,
        untargetable: false,
        blessing: Blessing::NONE,
        ship,
        combat_govt: record.combat_govt,
        target: None,
        class: nova_sim::escort::EscortClass::of(
            record.escort_type,
            record.inherent_ai,
            record.fields.mass,
        ),
        boarded: false,
        speed: 0.0,
        cloak: nova_sim::cloak::Cloak::default(),
    }
}

#[test]
fn the_stock_iff_projectors_mark_federation_and_moash_governments() {
    let Some(data) = opened() else {
        return;
    };
    let outfits = data.outfits();
    let mod_of = |id: i16| {
        outfits
            .iter()
            .find(|outfit| outfit.id.0 == id)
            .map(|outfit| outfit.mods[0])
    };
    assert_eq!(mod_of(442), Some((48, 1)), "the Federation IFF Projector");
    assert_eq!(mod_of(443), Some((48, 3)), "the Moash IFF Projector");
    let scramblers = outfits
        .iter()
        .filter(|outfit| outfit.mods.iter().any(|&(mod_type, _)| mod_type == 48))
        .count();
    assert_eq!(scramblers, 2);
    let govts = Governments::read(&data);
    let of_class = |class: i16| -> BTreeSet<i16> {
        govts
            .ids()
            .filter(|&govt| {
                govts
                    .get(govt)
                    .is_some_and(|record| record.classes.contains(&class))
            })
            .map(|govt| govt.0)
            .collect()
    };
    assert_eq!(of_class(1), BTreeSet::from([128, 136, 146, 151, 154, 186]));
    assert_eq!(of_class(3), BTreeSet::from([131, 188]));
    for govt in of_class(1).into_iter().chain(of_class(3)) {
        assert!(!govts.xenophobic(Some(GovtId(govt))), "{govt}");
    }
}

/// A new stock pilot flying a Shuttle in flight at the centre of
/// Fomalhaut, wanted by the Federation (-100), owning `outfits`, and the
/// first Federation warship or interceptor some seed brings.
fn wanted_in_fomalhaut(data: &GameData, outfits: serde_json::Value) -> (Session, NpcId) {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(136);
    save["stellar"] = serde_json::Value::Null;
    save["ship"] = serde_json::json!(SHUTTLE.0);
    save["outfits"] = outfits;
    save["stock_weapons_fitted"] = serde_json::json!(false);
    save["legal"] = serde_json::json!([{ "govt": 128, "record": -100 }]);
    save["system_legal"] = serde_json::Value::Null;
    for gauge in ["shield", "armor", "fuel"] {
        save["reserves"][gauge]["now"] = serde_json::json!(1_000_000.0);
        save["reserves"][gauge]["max"] = serde_json::json!(1_000_000.0);
    }
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(data, pilot).expect("flies");
    let fed = (1..=200)
        .find_map(|seed| {
            session.populate(data, &mut Seeded(seed));
            session
                .npcs()
                .iter()
                .find(|npc| {
                    npc.govt == Some(FEDERATION) && npc.aggression > 0 && !npc.ai_type.trades()
                })
                .map(|npc| npc.id)
        })
        .expect("a seed brings the Federation");
    (session, fed)
}

/// Whether any Federation ship attacks the player over 600 ticks of
/// [`wanted_in_fomalhaut`]'s session.
fn hunted(data: &GameData, outfits: serde_json::Value) -> bool {
    let (mut session, _) = wanted_in_fomalhaut(data, outfits);
    let mut chance = Seeded(0x5EED);
    (0..600).any(|_| {
        session.tick_traffic(data, &NovaAi::default(), &mut chance);
        session
            .npcs()
            .iter()
            .any(|npc| npc.govt == Some(FEDERATION) && npc.goal == Goal::Attack(P))
    })
}

#[test]
fn the_federation_iff_projector_keeps_the_federation_off_a_wanted_pilot() {
    let Some(data) = opened() else {
        return;
    };
    assert!(hunted(&data, serde_json::json!([])), "without it");
    assert!(
        !hunted(&data, serde_json::json!([{ "outfit": 442, "count": 1 }])),
        "with it (@0x8aa13)"
    );
}

#[test]
#[allow(clippy::float_cmp)]
fn a_pilot_flying_a_fed_destroyer_counts_the_federations_ships_as_friends() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let govts = Governments::read(&data);
    let npcs = [stock_warship(
        &arsenal,
        1,
        ShipId(144),
        FEDERATION,
        (0.0, 300.0),
    )];
    let strength = |ship| {
        let around = Surroundings {
            player: Some(stock_player(&data, &arsenal, ship, (0.0, 0.0))),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        friend_strength(P, &around, nova_sim::RuleSource::Engine)
    };
    let destroyer = strength(FED_DESTROYER);
    let own = (arsenal.hull(FED_DESTROYER).strength * 0.25).trunc();
    let viper = arsenal.hull(ShipId(144)).strength;
    assert_eq!(destroyer, own + viper, "the Fed Viper counts");
    assert_eq!(
        strength(SHUTTLE),
        (arsenal.hull(SHUTTLE).strength * 0.25).trunc(),
        "a Shuttle fights as no government"
    );
}

/// What a Rebellion warship (a Rebel Destroyer, `shïp` 181) far from the
/// player flying `ship` in an independent system takes, drawing `draws`:
/// its pick, and the ranges asked.
fn rebel_pick(data: &GameData, ship: ShipId, draws: &[u32]) -> (Option<ShipRef>, Vec<u32>) {
    let arsenal = Arsenal::read(data);
    let govts = Governments::read(data);
    let npcs = [stock_warship(
        &arsenal,
        1,
        ShipId(181),
        REBELLION,
        (0.0, 0.0),
    )];
    let around = Surroundings {
        player: Some(stock_player(data, &arsenal, ship, (0.0, -9000.0))),
        govts: &govts,
        ..Surroundings::new(&[], &npcs)
    };
    let mut chance = Draws::of(draws);
    let pick = select_target(&npcs[0], None, &around, TargetRules::default(), &mut chance);
    (pick, chance.asked)
}

#[test]
fn a_rebellion_warship_turns_on_a_fed_destroyer_at_1_in_50() {
    let Some(data) = opened() else {
        return;
    };
    let govts = Governments::read(&data);
    assert!(govts.enemies(Some(REBELLION), Some(FEDERATION)));
    assert!(!govts.xenophobic(Some(REBELLION)));
    assert_eq!(rebel_pick(&data, FED_DESTROYER, &[0]), (Some(P), vec![50]));
    assert_eq!(rebel_pick(&data, FED_DESTROYER, &[1]), (None, vec![50]));
    assert_eq!(
        rebel_pick(&data, SHUTTLE, &[0]),
        (None, vec![]),
        "a Shuttle fights as no government: no roll"
    );
}

#[test]
fn a_bureau_warship_at_home_takes_a_pilot_in_good_standing_only_by_its_word() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let govts = Governments::read(&data);
    assert!(govts.flag(Some(BUREAU), 0x0004), "Flags 0x0004");
    assert!(!govts.xenophobic(Some(BUREAU)));
    let npcs = [stock_warship(&arsenal, 1, ShipId(141), BUREAU, (0.0, 0.0))];
    let around = Surroundings {
        player: Some(stock_player(&data, &arsenal, SHUTTLE, (0.0, -9000.0))),
        govts: &govts,
        system_govt: Some(BUREAU),
        record: 1,
        ..Surroundings::new(&[], &npcs)
    };
    assert_eq!(
        select_target(
            &npcs[0],
            None,
            &around,
            TargetRules::default(),
            &mut Draws::of(&[])
        ),
        None,
        "not through the target step (crons-news-ranks-fidelity 32)"
    );
    let decision = Warship::default().decision(&npcs[0], &around, &mut Draws::of(&[]));
    assert_eq!(decision.goal, Goal::Attack(P), "the government's word");
    assert!(decision.turns_on_player);
}

#[test]
fn only_the_escape_pod_holds_no_shield_and_no_pirate_takes_a_player_in_it() {
    let Some(data) = opened() else {
        return;
    };
    let shieldless: Vec<ShipId> = data
        .ships()
        .into_iter()
        .filter(|record| record.fields.shield == 0)
        .map(|record| record.id)
        .collect();
    assert_eq!(shieldless, vec![ESCAPE_POD]);
    let arsenal = Arsenal::read(&data);
    let govts = Governments::read(&data);
    assert!(govts.xenophobic(Some(PIRATES)));
    let npcs = [stock_warship(&arsenal, 1, ShipId(166), PIRATES, (0.0, 0.0))];
    let pick = |ship| {
        let around = Surroundings {
            player: Some(stock_player(&data, &arsenal, ship, (0.0, -100.0))),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        select_target(
            &npcs[0],
            None,
            &around,
            TargetRules::default(),
            &mut Draws::of(&[]),
        )
    };
    assert_eq!(pick(SHUTTLE), Some(P));
    assert_eq!(pick(ESCAPE_POD), None, "class 0x2ff (@0x8a0f1)");
}

/// A small seeded generator (xorshift64*), so the stock traffic rolls the
/// same dice every run; it never fires a percentage.
struct Seeded(u64);

impl nova_sim::Chance for Seeded {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let draw = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        u32::try_from(draw % u64::from(n.max(1))).expect("below n")
    }
}
