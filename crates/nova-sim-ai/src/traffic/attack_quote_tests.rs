//! Tests of the attack quotes on the line: a think's goes in the line
//! queue at its ship's berth, ahead of that think's calls and scan, and
//! one raised outside the thinks stays on
//! [`Traffic::take_attack_quotes`].

use std::collections::BTreeSet;

use super::{Traffic, World};
use crate::ai::{Behaviour, CommCall, Decision, Goal, Surroundings};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::stats::ShipStats;
use crate::traffic::QUOTE_ON_ATTACK;
use crate::traffic::fixture::{npc, quotes_on_line};
use crate::traffic::npc::{Npc, NpcId, NpcPerson};
use crate::traffic::on_line::OnLine;
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_catalog::catalog::PersonId;

/// Each NPC of the set turns on the player in its think, attacks it and
/// calls for help.
#[derive(Debug)]
struct Turning(BTreeSet<NpcId>);

impl Behaviour for Turning {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }

    fn decision(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Decision {
        Decision {
            goal: Goal::Attack(ShipRef::Player),
            turns_on_player: self.0.contains(&npc.id),
            comm_call: Some(CommCall::Distress),
            ..Decision::default()
        }
    }
}

/// Traffic of `count` persons at rest, of `Flags` 0x0010, each deciding on
/// every tick.
fn persons(count: u32) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = (0..count)
        .map(|id| Npc {
            person: Some(NpcPerson {
                id: PersonId(151),
                flags: QUOTE_ON_ATTACK,
                coward: 0,
                comm_quote: -1,
                hail_quote: 8,
                mission: false,
                portrait: None,
                invincible: false,
                grudge: false,
                quoted: false,
                quoted_at: None,
                zero_fuel: false,
            }),
            ..npc(id, ShipStats::new(FAST, &[]))
        })
        .collect();
    traffic
}

/// `traffic` one tick on as `behaviour` says, the player at the centre,
/// where the NPCs are.
fn tick_by_the_player(traffic: &mut Traffic, behaviour: &Turning) {
    let world = World {
        player: Some(crate::ai::fixture::player(0.0, 0.0)),
        ..World::new(&[])
    };
    traffic.tick_in(behaviour, world, &[], &mut Draws::of(&[]));
}

#[test]
fn a_thinks_quote_comes_ahead_of_its_own_call_and_scan_and_only_on_the_line() {
    let mut traffic = persons(1);
    traffic.npcs[0].goal = Goal::Inspect(ShipRef::Player);
    tick_by_the_player(&mut traffic, &Turning(BTreeSet::from([NpcId(0)])));
    assert_eq!(traffic.take_attack_quotes(), [], "said by the settle");
    assert_eq!(
        traffic.take_on_line(),
        [
            OnLine::AttackQuote(NpcId(0)),
            OnLine::Call(NpcId(0), CommCall::Distress, 0.0),
            OnLine::Scan(NpcId(0)),
        ]
    );
}

#[test]
fn the_lower_berths_quote_comes_first_whatever_the_npc_order() {
    let mut traffic = persons(2);
    traffic.npcs[0].berth = 5;
    traffic.npcs[1].berth = 2;
    let both = Turning(BTreeSet::from([NpcId(0), NpcId(1)]));
    tick_by_the_player(&mut traffic, &both);
    assert_eq!(
        quotes_on_line(&traffic.take_on_line()),
        [NpcId(1), NpcId(0)]
    );
}

#[test]
fn a_quote_raised_outside_the_thinks_stays_off_the_line() {
    let mut traffic = persons(1);
    traffic.turn_with_quote(NpcId(0), crate::traffic::persons::GateView::default());
    assert_eq!(traffic.take_on_line(), []);
    assert_eq!(traffic.take_attack_quotes(), [NpcId(0)]);
}
