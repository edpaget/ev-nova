//! The passes the integration tests work a session through: one advance
//! pass of `Session::game_work` at a time.

use nova_sim::ai::{Behaviour, Goal, NovaAi, Surroundings};
use nova_sim::chance::Chance;
use nova_sim::combat::ShipRef;
use nova_sim::combat::armament::Trigger;
use nova_sim::traffic::npc::Npc;
use nova_sim::{Controls, NeverFires, PilotCatalog, Rules, Session, TrafficCatalog};

/// Every NPC keeps its goal, its target and its trigger: the traffic of a
/// pass whose test is about something else.
#[derive(Debug)]
pub struct Passive;

impl Behaviour for Passive {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
        npc.trigger
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        npc.target
    }
}

/// One advance pass under `controls`, the traffic drawn from `catalog`
/// keeping its goals ([`Passive`]), the fight by the default rules, no
/// chance firing. Asserts no rescue ended it.
pub fn work(
    session: &mut Session,
    catalog: &(impl PilotCatalog + TrafficCatalog),
    controls: Controls,
) {
    work_with(
        session,
        catalog,
        controls,
        &Passive,
        Rules::default(),
        &mut NeverFires,
    );
}

/// One advance pass under `controls`, the traffic drawn from `catalog`
/// deciding as Nova's does, the fight by the default rules, drawing on
/// `chance`. Asserts no rescue ended it.
pub fn nova(
    session: &mut Session,
    catalog: &(impl PilotCatalog + TrafficCatalog),
    controls: Controls,
    chance: &mut (impl Chance + ?Sized),
) {
    work_with(
        session,
        catalog,
        controls,
        &NovaAi::default(),
        Rules::default(),
        chance,
    );
}

/// One advance pass under `controls`, the traffic drawn from `catalog`
/// deciding as `behaviour` says, the fight by `rules`, drawing on
/// `chance`. Asserts no rescue ended it.
pub fn work_with(
    session: &mut Session,
    catalog: &(impl PilotCatalog + TrafficCatalog),
    controls: Controls,
    behaviour: &(impl Behaviour + ?Sized),
    rules: Rules<'_>,
    chance: &mut (impl Chance + ?Sized),
) {
    let rescued = session.game_work(controls, catalog, behaviour, rules, chance);
    assert!(!rescued, "no rescue ends the pass");
}
