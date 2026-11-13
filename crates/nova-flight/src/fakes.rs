//! The fakes of the session's collaborators that the adapter's and the
//! edge's checks share: NPC behaviours ([`Still`], [`Firing`]), a
//! disable rule that counts its asks ([`Counting`]), and Nova's
//! collaborators with a chance and a behaviour given ([`on`]). They moved
//! here from nova-view's `flight::fake_catalog` with the checks, as they
//! name nova-sim's AI and combat traits and the adapter's
//! [`Collaborators`].

use std::rc::Rc;

use nova_sim::{Behaviour, Chance, DisableRule, Goal};
use nova_view::flight::SharedChance;

use crate::sim_adapter::Collaborators;

/// Nova's collaborators, rolling on `chance`, the NPCs deciding as
/// `behaviour` says.
pub(crate) fn on(chance: SharedChance, behaviour: Rc<dyn Behaviour>) -> Collaborators {
    Collaborators {
        chance,
        behaviour,
        ..Collaborators::default()
    }
}

/// Decides nothing: every NPC idles.
#[derive(Debug)]
pub(crate) struct Still;

impl Behaviour for Still {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::Idle
    }
}

/// Says no ship is disabled, counting the times it is asked.
#[derive(Debug, Default)]
pub(crate) struct Counting {
    pub(crate) asked: std::cell::Cell<usize>,
}

impl DisableRule for Counting {
    fn disabled(&self, _armor: nova_sim::Gauge, _hull: &nova_sim::HullSpec) -> bool {
        self.asked.set(self.asked.get() + 1);
        false
    }
}

/// Every NPC idles and picks its first weapon each time it thinks.
#[derive(Debug)]
pub(crate) struct Firing;

impl Behaviour for Firing {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::Idle
    }

    fn trigger(&self, npc: &nova_sim::Npc, _around: &nova_sim::Surroundings) -> nova_sim::Trigger {
        // Its pick, its first weapon.
        nova_sim::Trigger {
            only: npc.armament.mounts().first().map(|mount| mount.spec.id),
            ..nova_sim::Trigger::default()
        }
    }
}
