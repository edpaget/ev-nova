//! [`NovaAi`]: Nova's combat AI, routing each NPC by its AI type to the
//! behaviour for it (`_AIDispatch` @0x8fb52 in the `EV Nova` executable),
//! by default [`WimpyTrader`], [`BraveTrader`], [`Warship`] and
//! [`Interceptor`]. `_PirateWarshipAI` (a warship of `Flags` 0x1000,
//! which disables and plunders) waits for boarding.

use std::rc::Rc;

use crate::ai::{
    Behaviour, BraveTrader, Goal, Interceptor, Reaction, Surroundings, Warship, WimpyTrader,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::traffic::npc::{AiType, Npc};

/// Nova's combat AI (see the module docs).
#[derive(Clone, Debug)]
pub struct NovaAi {
    wimpy: Rc<dyn Behaviour>,
    brave: Rc<dyn Behaviour>,
    warship: Rc<dyn Behaviour>,
    interceptor: Rc<dyn Behaviour>,
}

impl Default for NovaAi {
    /// Nova's four.
    fn default() -> Self {
        Self {
            wimpy: Rc::new(WimpyTrader),
            brave: Rc::new(BraveTrader),
            warship: Rc::new(Warship),
            interceptor: Rc::new(Interceptor),
        }
    }
}

impl NovaAi {
    /// This AI with `behaviour` for NPCs of `ai_type`.
    #[must_use]
    pub fn with(mut self, ai_type: AiType, behaviour: Rc<dyn Behaviour>) -> Self {
        *self.slot(ai_type) = behaviour;
        self
    }

    /// Where the behaviour for NPCs of `ai_type` is kept.
    fn slot(&mut self, ai_type: AiType) -> &mut Rc<dyn Behaviour> {
        match ai_type {
            AiType::WimpyTrader => &mut self.wimpy,
            AiType::BraveTrader => &mut self.brave,
            AiType::Warship => &mut self.warship,
            AiType::Interceptor => &mut self.interceptor,
        }
    }

    /// The behaviour for NPCs of `ai_type`.
    fn of(&self, ai_type: AiType) -> &dyn Behaviour {
        match ai_type {
            AiType::WimpyTrader => &*self.wimpy,
            AiType::BraveTrader => &*self.brave,
            AiType::Warship => &*self.warship,
            AiType::Interceptor => &*self.interceptor,
        }
    }
}

impl Behaviour for NovaAi {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.of(npc.ai_type).decide(npc, around, chance)
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        self.of(npc.ai_type).trigger(npc, around)
    }

    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        self.of(npc.ai_type).target(npc, around)
    }

    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        self.of(npc.ai_type).react(npc, strike, around)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::catalog::WeaponId;
    use crate::stats::ShipStats;
    use crate::testkit::{Draws, FAST};
    use crate::traffic::npc::NpcId;

    /// Answers with its own `mark`, recording the NPCs it was asked of.
    #[derive(Debug)]
    struct Marked {
        mark: u32,
        asked: RefCell<Vec<NpcId>>,
    }

    impl Marked {
        fn new(mark: u32) -> Rc<Self> {
            Rc::new(Self {
                mark,
                asked: RefCell::default(),
            })
        }

        fn ship(&self) -> ShipRef {
            ShipRef::Npc(NpcId(self.mark))
        }
    }

    impl Behaviour for Marked {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            self.asked.borrow_mut().push(npc.id);
            Goal::Attack(self.ship())
        }

        fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
            self.asked.borrow_mut().push(npc.id);
            Trigger {
                only: Some(WeaponId(i16::try_from(self.mark).unwrap_or_default())),
                ..Trigger::default()
            }
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            self.asked.borrow_mut().push(npc.id);
            Some(self.ship())
        }

        fn react(&self, npc: &Npc, _strike: &Strike, _around: &Surroundings) -> Reaction {
            self.asked.borrow_mut().push(npc.id);
            Reaction {
                provoked: self.mark as f32,
                ..Reaction::default()
            }
        }
    }

    const TYPES: [AiType; 4] = [
        AiType::WimpyTrader,
        AiType::BraveTrader,
        AiType::Warship,
        AiType::Interceptor,
    ];

    fn of(id: u32, ai_type: AiType) -> Npc {
        Npc {
            ai_type,
            ..crate::testkit::npc(id, ShipStats::new(FAST, &[]))
        }
    }

    #[test]
    fn each_ai_type_is_routed_to_its_own_behaviour() {
        let marks: Vec<Rc<Marked>> = (1..=4).map(Marked::new).collect();
        let ai = TYPES
            .into_iter()
            .zip(&marks)
            .fold(NovaAi::default(), |ai, (ai_type, mark)| {
                ai.with(ai_type, mark.clone())
            });
        let npcs: Vec<Npc> = TYPES
            .into_iter()
            .enumerate()
            .map(|(i, ai_type)| of(10 + i as u32, ai_type))
            .collect();
        let around = Surroundings::new(&[], &npcs);
        let strike = Strike {
            ship: ShipRef::Player,
            by: ShipRef::Player,
            damage: 0.0,
            downed: None,
        };
        for (npc, mark) in npcs.iter().zip(&marks) {
            let ship = mark.ship();
            assert_eq!(
                ai.decide(npc, &around, &mut Draws::of(&[])),
                Goal::Attack(ship)
            );
            assert_eq!(ai.target(npc, &around), Some(ship));
            assert_eq!(
                ai.trigger(npc, &around).only,
                Some(WeaponId(i16::try_from(mark.mark).unwrap_or_default()))
            );
            assert_eq!(ai.react(npc, &strike, &around).provoked, mark.mark as f32);
            assert_eq!(*mark.asked.borrow(), [npc.id; 4], "only its own type");
        }
    }

    #[test]
    fn with_swaps_one_type_and_leaves_novas_others() {
        let mark = Marked::new(7);
        let ai = NovaAi::default().with(AiType::Warship, mark.clone());
        let npcs = [of(1, AiType::Warship), of(2, AiType::WimpyTrader)];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Attack(mark.ship())
        );
        assert_eq!(
            ai.decide(&npcs[1], &around, &mut Draws::of(&[])),
            WimpyTrader.decide(&npcs[1], &around, &mut Draws::of(&[]))
        );
        assert_eq!(*mark.asked.borrow(), [NpcId(1)]);
    }

    #[test]
    fn by_default_nova_ai_is_novas_four() {
        let ai = NovaAi::default();
        assert!(format!("{ai:?}").contains("WimpyTrader"));
        assert!(format!("{ai:?}").contains("BraveTrader"));
        assert!(format!("{ai:?}").contains("Warship"));
        assert!(format!("{ai:?}").contains("Interceptor"));
    }
}
