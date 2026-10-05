//! [`NovaAi`]: Nova's combat AI, routing each NPC by its AI type to the
//! behaviour for it (`_AIDispatch` @0x8fb52 in the `EV Nova` executable),
//! by default [`WimpyTrader`], [`BraveTrader`], [`Warship`] and
//! [`Interceptor`], each answering the player's attacks and boardings as
//! the rulebook's [`RuleKey::PiracyPolice`] entry says
//! ([`NovaAi::from_rulebook`]). The player's escorts, whatever their AI
//! type, go to [`EscortAi`] instead (the original's AI type 6, @0x90011),
//! flying as the rulebook's [`RuleKey::EscortAi`] entry says.
//! `_PirateWarshipAI` (a warship of `Flags` 0x1000, which disables and
//! plunders) waits for NPC boarding.

use std::rc::Rc;

use crate::ai::{
    Behaviour, BraveTrader, EscortAi, Goal, Interceptor, Reaction, Surroundings, Warship,
    WimpyTrader,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::rulebook::{RuleKey, Rulebook};
use crate::traffic::npc::{AiType, Npc};

/// Nova's combat AI (see the module docs).
#[derive(Clone, Debug)]
pub struct NovaAi {
    wimpy: Rc<dyn Behaviour>,
    brave: Rc<dyn Behaviour>,
    warship: Rc<dyn Behaviour>,
    interceptor: Rc<dyn Behaviour>,
    escorts: Rc<dyn Behaviour>,
}

impl Default for NovaAi {
    /// Nova's four, by the engine.
    fn default() -> Self {
        Self::from_rulebook(&Rulebook::default())
    }
}

impl NovaAi {
    /// Nova's four and its escorts' AI as `rulebook` chooses: each of
    /// the four answers the player's attack or boarding as its
    /// [`RuleKey::PiracyPolice`] entry says, and the escorts fly as its
    /// [`RuleKey::EscortAi`] entry says.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        let piracy_police = rulebook.source_for(RuleKey::PiracyPolice);
        Self {
            wimpy: Rc::new(WimpyTrader { piracy_police }),
            brave: Rc::new(BraveTrader { piracy_police }),
            warship: Rc::new(Warship { piracy_police }),
            interceptor: Rc::new(Interceptor { piracy_police }),
            escorts: Rc::new(EscortAi {
                escort_ai: rulebook.source_for(RuleKey::EscortAi),
            }),
        }
    }

    /// This AI with `behaviour` for the player's escorts.
    #[must_use]
    pub fn with_escorts(mut self, behaviour: Rc<dyn Behaviour>) -> Self {
        self.escorts = behaviour;
        self
    }

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

    /// The behaviour for `npc`: the escorts' for the player's escort,
    /// otherwise the one for its AI type.
    fn of(&self, npc: &Npc) -> &dyn Behaviour {
        if npc.escort.is_some() {
            return &*self.escorts;
        }
        match npc.ai_type {
            AiType::WimpyTrader => &*self.wimpy,
            AiType::BraveTrader => &*self.brave,
            AiType::Warship => &*self.warship,
            AiType::Interceptor => &*self.interceptor,
        }
    }
}

impl Behaviour for NovaAi {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.of(npc).decide(npc, around, chance)
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        self.of(npc).trigger(npc, around)
    }

    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        self.of(npc).target(npc, around)
    }

    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        self.of(npc).react(npc, strike, around)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::catalog::WeaponId;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
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
            WimpyTrader::default().decide(&npcs[1], &around, &mut Draws::of(&[]))
        );
        assert_eq!(*mark.asked.borrow(), [NpcId(1)]);
    }

    /// `npc` as the player's escort.
    fn escorting(mut npc: Npc) -> Npc {
        npc.escort = Some(crate::escort::EscortDuty {
            slot: 2,
            ships: 2,
            spacing: 30.0,
            order: None,
        });
        npc
    }

    #[test]
    fn an_escort_goes_to_the_escort_behaviour_whatever_its_ai_type() {
        let marks: Vec<Rc<Marked>> = (1..=4).map(Marked::new).collect();
        let escorts = Marked::new(9);
        let ai = TYPES
            .into_iter()
            .zip(&marks)
            .fold(NovaAi::default(), |ai, (ai_type, mark)| {
                ai.with(ai_type, mark.clone())
            })
            .with_escorts(escorts.clone());
        let npcs: Vec<Npc> = TYPES
            .into_iter()
            .enumerate()
            .map(|(i, ai_type)| escorting(of(10 + i as u32, ai_type)))
            .collect();
        let around = Surroundings::new(&[], &npcs);
        let strike = Strike {
            ship: ShipRef::Player,
            by: ShipRef::Player,
            damage: 0.0,
            downed: None,
        };
        for npc in &npcs {
            assert_eq!(
                ai.decide(npc, &around, &mut Draws::of(&[])),
                Goal::Attack(escorts.ship())
            );
            assert_eq!(ai.target(npc, &around), Some(escorts.ship()));
            assert_eq!(ai.trigger(npc, &around).only, Some(WeaponId(9)));
            assert_eq!(ai.react(npc, &strike, &around).provoked, 9.0);
        }
        assert_eq!(escorts.asked.borrow().len(), 16);
        assert!(marks.iter().all(|mark| mark.asked.borrow().is_empty()));
    }

    #[test]
    fn nova_ai_flies_escorts_as_its_rulebook_says() {
        use crate::ai::fixture::{PIRATES, player, ship};
        let mut warship = escorting(ship(1, PIRATES, AiType::Warship, 0.0, 0.0));
        warship.govt = None;
        let mut pirate = ship(2, PIRATES, AiType::Warship, 300.0, 0.0);
        pirate.goal = Goal::Attack(ShipRef::Player);
        let npcs = [warship, pirate];
        let around = Surroundings {
            player: Some(player(0.0, 0.0)),
            ..Surroundings::new(&[], &npcs)
        };
        let decide = |ai: &NovaAi| ai.decide(&npcs[0], &around, &mut Draws::of(&[0]));
        let guarding = Goal::Formation {
            guard: Some(ShipRef::Npc(NpcId(2))),
        };
        assert_eq!(decide(&NovaAi::default()), guarding);
        assert_eq!(
            decide(&NovaAi::from_rulebook(&Rulebook::default())),
            guarding
        );
        let bible = Rulebook::default().with_override(RuleKey::EscortAi, RuleSource::Bible);
        assert_eq!(
            decide(&NovaAi::from_rulebook(&bible)),
            Goal::Attack(ShipRef::Npc(NpcId(2)))
        );
    }

    /// How each of a police warship and a police interceptor answers the
    /// player's hit on a trader allied with the police, by `ai`.
    fn police_answers(ai: &NovaAi) -> [Option<Goal>; 2] {
        use crate::ai::fixture::{POLICE, TRADERS, govts, ship};
        let govts = govts(0, 0);
        let npcs = [
            ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0),
            ship(2, POLICE, AiType::Warship, 500.0, 0.0),
            ship(3, POLICE, AiType::Interceptor, 900.0, 0.0),
        ];
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let hit = Strike {
            ship: ShipRef::Npc(NpcId(1)),
            by: ShipRef::Player,
            damage: 0.0,
            downed: None,
        };
        [&npcs[1], &npcs[2]].map(|npc| ai.react(npc, &hit, &around).goal)
    }

    #[test]
    fn nova_ai_polices_piracy_as_its_rulebook_says() {
        let both = [Some(Goal::Attack(ShipRef::Player)); 2];
        assert_eq!(police_answers(&NovaAi::default()), both);
        assert_eq!(
            police_answers(&NovaAi::from_rulebook(&Rulebook::default())),
            both
        );
        let bible = Rulebook::default().with_override(RuleKey::PiracyPolice, RuleSource::Bible);
        assert_eq!(
            police_answers(&NovaAi::from_rulebook(&bible)),
            [None, Some(Goal::Attack(ShipRef::Player))],
            "the interceptor only"
        );
        for source in RuleSource::ALL {
            let rulebook = Rulebook::default().with_override(RuleKey::PiracyPolice, source);
            let ai = NovaAi::from_rulebook(&rulebook);
            let expected = format!("piracy_police: {source:?}");
            assert_eq!(
                format!("{ai:?}").matches(&expected).count(),
                4,
                "all four types: {ai:?}"
            );
        }
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
