//! A dying NPC's escape pods and its last fighter over the stock data:
//! 83 `shïp` carry crew pods and 97 have an odd `DeathDelay`, 416 of the
//! 516 `përs` launch a pod of their own, every stock fighter bay but 175
//! and 176 launches a fighter of `Flags` 0x8000, which 51 `shïp` carry as
//! a stock weapon, and the Star Liner (`shïp` 134, `DeathDelay` 100,
//! `PodCount` 10) ejects a pod every 10 frames. Through stock sessions, a
//! Star Liner's dying hulk ejects its pods on that interval, and a Fed
//! Carrier's rolls once for its escape ship. Skips, passing, when
//! `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::combat::Rules;
use nova_sim::combat::armament::Arsenal;
use nova_sim::combat::hull::Condition;
use nova_sim::npc_escape::{ESCAPE_ROLL_ODDS, pod_interval};
use nova_sim::person::ESCAPE_POD;
use nova_sim::{
    Chance, CombatCatalog, CombatEvent, Controls, NpcId, Pilot, PilotCatalog, Session, ShipId,
    ShipRef, TrafficCatalog,
};

/// The Star Liner.
const STAR_LINER: ShipId = ShipId(134);
/// The Fed Carrier, carrying a Viper Bay and an Anaconda Bay.
const FED_CARRIER: ShipId = ShipId(143);
/// The Viper, the Fed Carrier's first bay's fighter.
const VIPER: ShipId = ShipId(144);
/// The escape pod and auto-eject outfits.
const ESCAPE_POD_OUTFIT: i16 = 188;
const AUTO_EJECT_OUTFIT: i16 = 187;

fn data() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

#[test]
fn the_stock_pod_counts_odd_delays_and_escape_pod_persons() {
    let Some(data) = data() else {
        return;
    };
    let hulls = data.hulls();
    let podded = hulls.iter().filter(|hull| hull.pod_count > 0).count();
    let odd = hulls
        .iter()
        .filter(|hull| hull.death_delay % 2 != 0)
        .count();
    assert_eq!((podded, odd), (83, 97));
    assert!(hulls.iter().all(|hull| hull.death_delay >= 0));
    let persons = data.persons();
    let pods = persons
        .iter()
        .filter(|person| person.flags & ESCAPE_POD != 0)
        .count();
    assert_eq!((pods, persons.len()), (416, 516));
}

#[test]
fn the_stock_star_liner_ejects_10_pods_every_10_frames_from_half_its_delay() {
    let Some(data) = data() else {
        return;
    };
    let liner = data
        .hulls()
        .into_iter()
        .find(|hull| hull.id == STAR_LINER)
        .expect("shïp 134");
    assert_eq!((liner.death_delay, liner.pod_count), (100, 10));
    assert_eq!(pod_interval(100, 10), 10);
    let name = data
        .ships()
        .into_iter()
        .find(|record| record.id == STAR_LINER)
        .expect("a record")
        .name;
    assert_eq!(name, "Star Liner");
}

#[test]
fn every_stock_bay_but_175_and_176_launches_an_escape_ship_and_51_classes_carry_one() {
    let Some(data) = data() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let ships = data.ships();
    let escape = |fighter: ShipId| {
        ships
            .iter()
            .find(|record| record.id == fighter)
            .is_some_and(|record| record.flags & nova_sim::escape::ESCAPE_SHIP_FLAG != 0)
    };
    let bays: Vec<(i16, bool)> = arsenal
        .weapons()
        .filter_map(|weapon| Some((weapon.id.0, escape(weapon.carried?))))
        .collect();
    assert_eq!(bays.len(), 23);
    let others: Vec<i16> = bays
        .iter()
        .filter(|(_, escapes)| !escapes)
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(others, [175, 176]);
    let escaping: Vec<i16> = bays
        .iter()
        .filter(|(_, escapes)| *escapes)
        .map(|(id, _)| *id)
        .collect();
    let classes = data
        .hulls()
        .iter()
        .filter(|hull| {
            hull.weapons
                .iter()
                .any(|stock| escaping.contains(&stock.weapon.0))
        })
        .count();
    assert_eq!(classes, 51);
}

/// Rolls 0 for each escape roll, counting them, and the last outcome
/// for every other draw.
#[derive(Debug, Default)]
struct Rolls {
    escape_rolls: usize,
}

impl Chance for Rolls {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        if n == ESCAPE_ROLL_ODDS {
            self.escape_rolls += 1;
            0
        } else {
            n.saturating_sub(1)
        }
    }
}

/// The stock first pilot flying `ship` in space with the escape pod and
/// auto-eject, its ship breaking up with `ticks_left` of its death timer
/// left.
fn dying_in(data: &GameData, ship: ShipId, ticks_left: u32) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["ship"] = serde_json::json!(ship.0);
    save["stellar"] = serde_json::Value::Null;
    save["outfits"] = serde_json::json!([
        {"outfit": ESCAPE_POD_OUTFIT, "count": 1},
        {"outfit": AUTO_EJECT_OUTFIT, "count": 1}
    ]);
    let flown = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(data, flown).expect("flies");
    assert_eq!(session.ship(), ship);
    nova_sim::fixture::set_player_condition(
        &mut session,
        Condition::Dying {
            ticks_left,
            starting: false,
        },
    );
    session
}

/// The hulk of class `ship` the player left, if it is in the system.
fn hulk(session: &Session, ship: ShipId) -> Option<NpcId> {
    session
        .npcs()
        .iter()
        .find(|npc| npc.ship == ship)
        .map(|npc| npc.id)
}

/// Ticks `session` on `chance` until its hulk of class `ship` has come
/// and gone: the tick of each of the hulk's pods, counted from the first
/// tick.
fn hulk_pods(session: &mut Session, ship: ShipId, chance: &mut Rolls) -> Vec<u32> {
    let mut left = None;
    let mut pods = Vec::new();
    for tick in 0..600 {
        session.tick(Controls::default());
        session.tick_combat(Rules::default(), &mut *chance);
        let events = session.take_combat_events();
        left = left.or_else(|| hulk(session, ship));
        if let Some(id) = left {
            pods.extend(events.iter().filter_map(|event| match event {
                CombatEvent::PodEjected { ship, .. } if *ship == ShipRef::Npc(id) => Some(tick),
                _ => None,
            }));
            if hulk(session, ship).is_none() {
                return pods;
            }
        }
    }
    panic!("the hulk never came and went");
}

#[test]
fn a_stock_star_liners_dying_hulk_ejects_its_pods_on_the_interval() {
    let Some(data) = data() else {
        return;
    };
    // Auto-eject arms at half the DeathDelay, 50: the hulk is left at 50,
    // and ejects its first pod at once.
    let mut session = dying_in(&data, STAR_LINER, 60);
    let pods = hulk_pods(&mut session, STAR_LINER, &mut Rolls::default());
    assert!((4..=10).contains(&pods.len()), "{pods:?}");
    let gaps: Vec<u32> = pods.windows(2).map(|pair| pair[1] - pair[0]).collect();
    assert!((1..=10).contains(&gaps[0]), "the first at once: {gaps:?}");
    assert!(
        gaps[1..].iter().all(|&gap| gap == 10),
        "then every 10 frames: {gaps:?}"
    );
}

#[test]
fn a_stock_fed_carriers_dying_hulk_rolls_once_and_launches_a_viper() {
    let Some(data) = data() else {
        return;
    };
    let mut session = dying_in(&data, FED_CARRIER, 90);
    let mut chance = Rolls::default();
    let pods = hulk_pods(&mut session, FED_CARRIER, &mut chance);
    assert_eq!(chance.escape_rolls, 1, "one roll for the dying hulk");
    let vipers = session
        .npcs()
        .iter()
        .filter(|npc| npc.ship == VIPER)
        .count();
    assert_eq!(vipers, 1, "its escape ship launched");
    assert_eq!(pods.len(), 1, "and its one crew pod");
}
