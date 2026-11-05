//! Tests of a spawn's raw AI type word as the traffic reads it in (see
//! [`spawn_ai`](super::spawn_ai)): 0 or below a ship with no AI, above 4
//! one read as an escort until its first think whole.

#![allow(clippy::float_cmp)]

use std::cell::RefCell;
use std::collections::BTreeMap;

use super::{Traffic, World};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::{GovtId, ShipId};
use crate::chance::Chance;
use crate::flight::ShipState;
use crate::stats::ShipStats;
use crate::traffic::npc::{AiType, Npc, NpcId};
use crate::traffic::spawn::{Entry, NewShip};
use crate::traffic::spawn_ai::SpawnAi;
use crate::traffic::table::ShipKind;
use nova_sim_base::fixture::{Draws, FAST};

/// Records whom it decides for; every NPC keeps its goal.
#[derive(Debug, Default)]
struct Counting(RefCell<Vec<NpcId>>);

impl Behaviour for Counting {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        self.0.borrow_mut().push(npc.id);
        npc.goal
    }
}

/// Traffic deciding every tick, holding a ship of class 200, whose
/// `InherentAI` is `inherent_ai`, brought in with each word of `ai`.
fn spawned(ai: &[i16], inherent_ai: i16) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    let mut kind = ShipKind {
        stats: ShipStats::new(FAST, &[]),
        inherent_ai,
        ..ShipKind::default()
    };
    kind.hull.inherent_ai = inherent_ai;
    traffic.table.ships = BTreeMap::from([(ShipId(200), kind)]);
    let ship = |ai| NewShip {
        ship: ShipId(200),
        govt: Some(GovtId(130)),
        ai_type: SpawnAi(ai),
        lead: None,
        state: ShipState::default(),
        entry: Entry::Placed,
        aggression: 0,
        booty: None,
        info_types: 0,
        person: None,
        mission: None,
        aux: None,
        derelict: false,
        held: false,
        cloaked: false,
        quote: None,
        cargo: None,
        boarded: false,
        rolls_voice: false,
    };
    let came = traffic.bring_in_within(
        ai.iter().copied().map(ship).collect(),
        0,
        &crate::govt::Governments::empty(),
        &mut crate::chance::NeverFires,
    );
    assert_eq!(came, ai.len());
    traffic
}

fn tick(traffic: &mut Traffic, behaviour: &Counting) {
    traffic.tick_in(behaviour, World::new(&[]), &[], &mut Draws::of(&[]));
}

#[test]
fn a_ship_spawned_with_word_0_or_below_has_no_ai_never_thinks_and_reads_odds_0() {
    for word in [0, -1] {
        // A ship with an AI after it, where the walk stops.
        let mut traffic = spawned(&[word, 3], word);
        assert!(traffic.npcs()[0].no_ai, "{word}");
        assert_eq!(traffic.npcs()[0].original_ai_type(), -1);
        let behaviour = Counting::default();
        for _ in 0..4 {
            tick(&mut traffic, &behaviour);
        }
        let hulk = traffic.npcs()[0].id;
        assert!(
            !behaviour.0.borrow().contains(&hulk),
            "never dispatched (@0x4427e)"
        );
        traffic.refresh_odds(&World::new(&[]));
        assert_eq!(traffic.npcs()[0].odds, 0.0, "the odds pass @0x44307");
    }
}

#[test]
fn a_ship_spawned_with_word_5_reads_as_an_escort_until_its_first_think_whole() {
    let mut traffic = spawned(&[5], 3);
    let ship = &traffic.npcs()[0];
    assert!(!ship.no_ai);
    assert_eq!(ship.ai_type, AiType::Warship, "its class's InherentAI");
    assert_eq!(ship.original_ai_type(), 6);
    let behaviour = Counting::default();
    tick(&mut traffic, &behaviour);
    assert!(!behaviour.0.borrow().is_empty(), "it thinks");
    assert_eq!(traffic.npcs()[0].original_ai_type(), 3, "@0x83c4a");
}

#[test]
fn a_ship_spawned_with_word_1_to_4_reads_it() {
    let traffic = spawned(&[4], 3);
    let ship = &traffic.npcs()[0];
    assert_eq!(ship.ai_type, AiType::Interceptor);
    assert!(!ship.no_ai && !ship.kept_escort_ai);
}

#[test]
fn a_ship_spawned_with_word_0_or_below_answers_no_hit() {
    use crate::ai::answer_flag::{FlagRules, Verdict, verdict};
    use crate::ai::fixture::{PIRATES, TRADERS, govts};
    use crate::combat::fixture::strike;
    use crate::combat::{ShipRef, Strike};
    let govts = govts(0, 0);
    for word in [0, -1] {
        // A pirate with an AI after it, to hit it.
        let traffic = spawned(&[word, 3], word);
        let mut npcs = traffic.npcs().to_vec();
        npcs[0].govt = Some(TRADERS);
        npcs[1].govt = Some(PIRATES);
        let hit = strike(ShipRef::Npc(npcs[0].id), ShipRef::Npc(npcs[1].id));
        let judge = |npcs: &[Npc], hit: &Strike| {
            let around = Surroundings {
                govts: &govts,
                ..Surroundings::new(&[], npcs)
            };
            let mut chance = Draws::of(&[]);
            verdict(hit, &around, FlagRules::default(), &mut chance)
        };
        let astray = Strike {
            intended: false,
            weight: 10.0,
            ..hit
        };
        for hit in [hit, astray] {
            assert_eq!(
                judge(&npcs, &hit),
                Verdict::default(),
                "{word}: no answer and no stray provocation (@0x3b459)"
            );
        }
        npcs[0].no_ai = false;
        assert!(judge(&npcs, &hit).answers, "a ship with an AI answers it");
    }
}
