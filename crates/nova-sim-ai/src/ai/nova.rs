//! [`NovaAi`]: Nova's combat AI, routing each NPC by its AI type to the
//! behaviour for it (`_AIDispatch` @0x8fb52 in the `EV Nova` executable),
//! by default [`WimpyTrader`], [`BraveTrader`], [`Warship`] and
//! [`Interceptor`], each answering the player's attacks and boardings as
//! the rulebook's [`RuleKey::PiracyPolice`] entry says
//! ([`NovaAi::from_rulebook`]). The player's escorts, whatever their AI
//! type, go to [`EscortAi`] instead (the original's AI type 6, @0x90011),
//! flying as the rulebook's [`RuleKey::EscortAi`] entry says, and so does a
//! mission escort, a `ShipBehav` 1 mission ship that has joined the
//! player's formation (`_AIDispatch` @0x8fc03-0x8fc22 gives it AI type 6
//! and leader 0 on every think), with no standing order; and an NPC
//! carrier's fighters, whatever their AI type, to [`CarriedAi`] (the
//! original's AI type 5 with an NPC parent), until their carrier is gone.
//! `_PirateWarshipAI` (a warship of `Flags` 0x1000, which disables and
//! plunders) waits for NPC boarding, and with it the plunder of the
//! player's cargo and the mission failure it brings
//! (`_AIPlunderShipContents` @0x84a0f, `mïsn` `Flags` 0x8000).
//!
//! Each decision ([`Behaviour::decision`]) carries whether the think
//! turned the ship on the player through `_AIMakeShipAttackPlayer`: the
//! type's own, or a `ShipBehav` 0 mission ship's hunt (`_HighLevelAIHandler`
//! @0x8d6b7), whose quote gate its mark always fails (+0xc8d2, @0x89c71).

use std::rc::Rc;

use crate::ai::fire::FireRules;
use crate::ai::{
    Behaviour, BraveTrader, CarriedAi, Decision, EscortAi, GateWait, Goal, Interceptor, Reaction,
    Surroundings, Warship, WimpyTrader,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::traffic::npc::{AiType, Npc};

/// Nova's combat AI (see the module docs).
#[derive(Clone, Debug)]
pub struct NovaAi {
    wimpy: Rc<dyn Behaviour>,
    brave: Rc<dyn Behaviour>,
    warship: Rc<dyn Behaviour>,
    interceptor: Rc<dyn Behaviour>,
    escorts: Rc<dyn Behaviour>,
    carried: Rc<dyn Behaviour>,
    /// The source of [`RuleKey::HunterBribe`]: whether a paid-off
    /// `ShipBehav` 0 hunter attacks the player again.
    hunter_bribe: RuleSource,
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
    /// [`RuleKey::PiracyPolice`] entry says, the warship and interceptor
    /// persons run at their `Coward` as its [`RuleKey::PersonCoward`]
    /// entry says, the escorts fly as its [`RuleKey::EscortAi`] entry
    /// says, a paid-off `ShipBehav` 0 hunter attacks the player again
    /// or honours the bribe as its [`RuleKey::HunterBribe`] entry says,
    /// the warships' and interceptors' government's 0x0004 word reaches a
    /// blessed player as its [`RuleKey::BlessingAlwaysAttacks`] entry says,
    /// the interceptors' patrol picks an uninhabited stellar as its
    /// [`RuleKey::PatrolUninhabited`] entry says,
    /// and every one of them, the escorts and the carried fighters aim
    /// their quadrant turrets as its [`RuleKey::QuadrantTurretArc`] entry
    /// says.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        let piracy_police = rulebook.source_for(RuleKey::PiracyPolice);
        let person_coward = rulebook.source_for(RuleKey::PersonCoward);
        let always_attacks = rulebook.source_for(RuleKey::BlessingAlwaysAttacks);
        let fire = FireRules {
            quadrant_arc: rulebook.source_for(RuleKey::QuadrantTurretArc),
        };
        Self {
            wimpy: Rc::new(WimpyTrader {
                piracy_police,
                fire,
            }),
            brave: Rc::new(BraveTrader {
                piracy_police,
                fire,
            }),
            warship: Rc::new(Warship {
                piracy_police,
                person_coward,
                always_attacks,
                fire,
            }),
            interceptor: Rc::new(Interceptor {
                piracy_police,
                person_coward,
                always_attacks,
                fire,
                patrol_uninhabited: rulebook.source_for(RuleKey::PatrolUninhabited),
            }),
            escorts: Rc::new(EscortAi {
                escort_ai: rulebook.source_for(RuleKey::EscortAi),
                fire,
            }),
            carried: Rc::new(CarriedAi { fire }),
            hunter_bribe: rulebook.source_for(RuleKey::HunterBribe),
        }
    }

    /// This AI with `behaviour` for the player's escorts.
    #[must_use]
    pub fn with_escorts(mut self, behaviour: Rc<dyn Behaviour>) -> Self {
        self.escorts = behaviour;
        self
    }

    /// This AI with `behaviour` for an NPC carrier's fighters.
    #[must_use]
    pub fn with_carried(mut self, behaviour: Rc<dyn Behaviour>) -> Self {
        self.carried = behaviour;
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

    /// The behaviour for `npc`: the escorts' for the player's escort (its
    /// fighters too) and for a mission escort ([`Npc::mission_escort`]),
    /// the carried fighters' for one an NPC launched, otherwise the one for
    /// its AI type.
    fn of(&self, npc: &Npc) -> &dyn Behaviour {
        if npc.escorts_player() {
            return &*self.escorts;
        }
        if npc.carrier.is_some() {
            return &*self.carried;
        }
        match npc.ai_type {
            AiType::WimpyTrader => &*self.wimpy,
            AiType::BraveTrader => &*self.brave,
            AiType::Warship => &*self.warship,
            AiType::Interceptor => &*self.interceptor,
        }
    }
}

impl NovaAi {
    /// Whether `npc` is a mission's special ship of `ShipBehav` 0 that
    /// attacks the player now, among `around`, on every think (see
    /// [`mission_hunt`](crate::ai::target::mission_hunt)); one that does
    /// not decides as its type does. A ship the player paid off hunts
    /// again by the engine, and not under [`RuleSource::Bible`] for
    /// [`RuleKey::HunterBribe`] (`_HighLevelAIHandler` @0x8d628 never reads
    /// the bribe). Only this site reads the key: a paid-off ship is a
    /// wimpy trader, which never asks
    /// [`select_target`](crate::ai::target::select_target).
    fn hunts_player(&self, npc: &Npc, around: &Surroundings) -> bool {
        if npc.spared && self.hunter_bribe == RuleSource::Bible {
            return false;
        }
        crate::ai::target::mission_hunt(npc, around) == Some(true)
    }
}

impl Behaviour for NovaAi {
    /// [`NovaAi::decision`]'s goal.
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    /// A mission ship of `ShipBehav` 0 attacks the player
    /// (`hunts_player`), turning on it through `_AIMakeShipAttackPlayer`
    /// (`_HighLevelAIHandler` @0x8d6b7), whose quote gate its mark fails
    /// (+0xc8d2, @0x89c71); every other NPC decides as its behaviour does,
    /// its turn on the player with it.
    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        if self.hunts_player(npc, around) {
            return Decision {
                goal: Goal::Attack(ShipRef::Player),
                turns_on_player: true,
                dropped_target: false,
                provokes: false,
            };
        }
        self.of(npc).decision(npc, around, chance)
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        self.of(npc).trigger(npc, around)
    }

    /// A mission ship of `ShipBehav` 0 targets the player
    /// (`hunts_player`).
    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        if self.hunts_player(npc, around) {
            return Some(ShipRef::Player);
        }
        self.of(npc).target(npc, around)
    }

    fn gate_wait(&self, npc: &Npc, around: &Surroundings) -> GateWait {
        self.of(npc).gate_wait(npc, around)
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
    use crate::ai::fire;
    use crate::catalog::WeaponId;
    use crate::combat::hull::Condition;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::stats::ShipStats;
    use crate::traffic::ShipBehav;
    use crate::traffic::npc::NpcId;
    use nova_sim_base::fixture::{Draws, FAST};

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

        fn gate_wait(&self, npc: &Npc, _around: &Surroundings) -> GateWait {
            self.asked.borrow_mut().push(npc.id);
            GateWait {
                trigger: Trigger {
                    only: Some(WeaponId(i16::try_from(self.mark).unwrap_or_default())),
                    ..Trigger::default()
                },
                drops_target: self.mark.is_multiple_of(2),
            }
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
            ..crate::traffic::fixture::npc(id, ShipStats::new(FAST, &[]))
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
            by: Some(ShipRef::Player),
            damage: 0.0,
            downed: None,
            direct: true,
            intended: true,
            lock: false,
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
            formation: crate::escort::FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            },
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
            by: Some(ShipRef::Player),
            damage: 0.0,
            downed: None,
            direct: true,
            intended: true,
            lock: false,
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

    /// `npc` as a fighter launched by `carrier`.
    fn carried(mut npc: Npc, carrier: ShipRef) -> Npc {
        npc.carrier = Some(crate::bay::Carrier {
            ship: carrier,
            window: 100.0,
            reach: 40.0,
        });
        npc
    }

    #[test]
    fn an_npc_carriers_fighter_goes_to_the_carried_behaviour_whatever_its_ai_type() {
        let marks: Vec<Rc<Marked>> = (1..=4).map(Marked::new).collect();
        let fighters = Marked::new(8);
        let escorts = Marked::new(9);
        let ai = TYPES
            .into_iter()
            .zip(&marks)
            .fold(NovaAi::default(), |ai, (ai_type, mark)| {
                ai.with(ai_type, mark.clone())
            })
            .with_escorts(escorts.clone())
            .with_carried(fighters.clone());
        let carrier = ShipRef::Npc(NpcId(30));
        let npcs: Vec<Npc> = TYPES
            .into_iter()
            .enumerate()
            .map(|(i, ai_type)| carried(of(10 + i as u32, ai_type), carrier))
            .collect();
        let around = Surroundings::new(&[], &npcs);
        for npc in &npcs {
            assert_eq!(
                ai.decide(npc, &around, &mut Draws::of(&[])),
                Goal::Attack(fighters.ship())
            );
            assert_eq!(ai.trigger(npc, &around).only, Some(WeaponId(8)));
            assert_eq!(ai.target(npc, &around), Some(fighters.ship()));
        }
        assert_eq!(fighters.asked.borrow().len(), 12);
        assert!(marks.iter().all(|mark| mark.asked.borrow().is_empty()));
        // The player's fighter is the player's escort.
        let players = escorting(carried(of(20, AiType::Warship), ShipRef::Player));
        let npcs = [players];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Attack(escorts.ship())
        );
        assert_eq!(fighters.asked.borrow().len(), 12, "not the carried slot");
        // Once its carrier is gone, it flies by its AI type.
        let orphan = of(21, AiType::Warship);
        let npcs = [orphan];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Attack(marks[2].ship())
        );
    }

    #[test]
    fn a_ships_gate_wait_goes_to_its_own_behaviour() {
        let marks: Vec<Rc<Marked>> = (1..=4).map(Marked::new).collect();
        let fighters = Marked::new(8);
        let escorts = Marked::new(9);
        let ai = TYPES
            .into_iter()
            .zip(&marks)
            .fold(NovaAi::default(), |ai, (ai_type, mark)| {
                ai.with(ai_type, mark.clone())
            })
            .with_escorts(escorts.clone())
            .with_carried(fighters.clone());
        let mut npcs: Vec<Npc> = TYPES
            .into_iter()
            .enumerate()
            .map(|(i, ai_type)| of(10 + i as u32, ai_type))
            .collect();
        npcs.push(escorting(of(20, AiType::Warship)));
        npcs.push(carried(of(21, AiType::Warship), ShipRef::Npc(NpcId(30))));
        let around = Surroundings::new(&[], &npcs);
        let behaviours = marks.iter().chain([&escorts, &fighters]);
        for (npc, mark) in npcs.iter().zip(behaviours) {
            let wait = ai.gate_wait(npc, &around);
            assert_eq!(
                wait.trigger.only,
                Some(WeaponId(i16::try_from(mark.mark).unwrap_or_default())),
                "{:?}",
                npc.id
            );
            assert_eq!(
                wait.drops_target,
                mark.mark.is_multiple_of(2),
                "{:?}",
                npc.id
            );
            assert_eq!(*mark.asked.borrow(), [npc.id], "only its own");
        }
    }

    #[test]
    fn nova_ai_runs_the_point_defence_of_an_escort_or_fighter_waiting_in_its_gate_alone() {
        // `_AIEscortFireUnprovoked` reaches `_AIFireTurret` for AI type
        // above 4 alone (@0x80e59).
        use crate::ai::fixture::{PIRATES, player, ship};
        let pirate = ship(2, PIRATES, AiType::Warship, 300.0, 0.0);
        let target = |mut npc: Npc| {
            npc.target = Some(ShipRef::Npc(NpcId(2)));
            npc.mode = crate::traffic::npc::Mode::Emerging { ticks_left: 30 };
            npc
        };
        let escort = target(escorting(ship(1, PIRATES, AiType::Warship, 0.0, 0.0)));
        let fighter = target(carried(
            ship(3, PIRATES, AiType::Interceptor, 50.0, 0.0),
            ShipRef::Npc(NpcId(4)),
        ));
        let warship = target(ship(5, PIRATES, AiType::Warship, 0.0, 0.0));
        let npcs = [escort, pirate, fighter, warship];
        let around = Surroundings {
            player: Some(player(0.0, 300.0)),
            ..Surroundings::new(&[], &npcs)
        };
        let ai = NovaAi::default();
        for i in [0, 2] {
            let wait = ai.gate_wait(&npcs[i], &around);
            assert!(wait.trigger.point_defence, "{:?}", npcs[i].id);
            assert!(!wait.drops_target, "{:?}", npcs[i].id);
        }
        for ai_type in TYPES {
            let mut other = npcs[3].clone();
            other.ai_type = ai_type;
            assert_eq!(
                ai.gate_wait(&other, &around),
                GateWait::default(),
                "{ai_type:?}"
            );
        }
    }

    #[test]
    fn nova_ai_flies_npc_fighters_by_the_carried_ai() {
        use crate::ai::fixture::{PIRATES, player, ship};
        let mut carrier = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        carrier.goal = Goal::Idle;
        let fighter = carried(
            ship(2, PIRATES, AiType::Interceptor, 50.0, 0.0),
            ShipRef::Npc(NpcId(1)),
        );
        let npcs = [carrier, fighter];
        let around = Surroundings {
            player: Some(player(0.0, 300.0)),
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            NovaAi::default().decide(&npcs[1], &around, &mut Draws::of(&[])),
            Goal::Dock(ShipRef::Npc(NpcId(1)))
        );
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

    /// What `npc`, carrying a front quadrant turret and heading 350,
    /// fires by `ai` at the player 100 away bearing 10: across the seam.
    fn fired_across_the_seam(ai: &NovaAi, mut npc: Npc) -> Option<WeaponId> {
        npc.armament = crate::combat::armament::Armament::new([(
            crate::combat::weapon::WeaponSpec::new(&crate::catalog::WeaponRecord {
                guidance: 7,
                speed: 1000,
                count: 20,
                ..nova_sim_catalog::fixture::weapon(130)
            }),
            1,
        )]);
        npc.state.heading = 350.0;
        npc.goal = Goal::Attack(ShipRef::Player);
        let (sin, cos) = 10f32.to_radians().sin_cos();
        let at = npc.state.position + crate::geometry::Vec2::new(100.0 * sin, -100.0 * cos);
        let npcs = [npc];
        let around = Surroundings {
            player: Some(crate::ai::fixture::player(at.x, at.y)),
            ..Surroundings::new(&[], &npcs)
        };
        ai.trigger(&npcs[0], &around).only
    }

    #[test]
    fn nova_ai_aims_quadrant_turrets_as_its_rulebook_says() {
        let carried = Npc {
            carrier: Some(crate::bay::Carrier {
                ship: ShipRef::Npc(NpcId(9)),
                window: 100.0,
                reach: 40.0,
            }),
            ..of(1, AiType::Warship)
        };
        let ships: Vec<Npc> = TYPES
            .into_iter()
            .map(|ai_type| of(1, ai_type))
            .chain([escorting(of(1, AiType::Warship)), carried])
            .collect();
        let engine = NovaAi::from_rulebook(&Rulebook::default());
        let bible = NovaAi::from_rulebook(
            &Rulebook::default().with_override(RuleKey::QuadrantTurretArc, RuleSource::Bible),
        );
        for npc in ships {
            let kind = format!("{:?} {:?}", npc.ai_type, npc.escort.is_some());
            assert_eq!(fired_across_the_seam(&engine, npc.clone()), None, "{kind}");
            assert_eq!(
                fired_across_the_seam(&NovaAi::default(), npc.clone()),
                None,
                "{kind}"
            );
            assert_eq!(
                fired_across_the_seam(&bible, npc),
                Some(WeaponId(130)),
                "{kind}"
            );
        }
        for source in RuleSource::ALL {
            let rulebook = Rulebook::default().with_override(RuleKey::QuadrantTurretArc, source);
            let ai = format!("{:?}", NovaAi::from_rulebook(&rulebook));
            let expected = format!("quadrant_arc: {source:?}");
            assert_eq!(ai.matches(&expected).count(), 6, "all six: {ai}");
        }
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
            by: Some(ShipRef::Player),
            damage: 0.0,
            downed: None,
            direct: true,
            intended: true,
            lock: false,
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
    fn nova_ai_runs_its_persons_as_its_rulebook_says() {
        for source in RuleSource::ALL {
            let rulebook = Rulebook::default().with_override(RuleKey::PersonCoward, source);
            let ai = format!("{:?}", NovaAi::from_rulebook(&rulebook));
            let expected = format!("person_coward: {source:?}");
            assert_eq!(
                ai.matches(&expected).count(),
                2,
                "the warship and the interceptor: {ai}"
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

    /// `npc` as slot 1's special ship of `ShipBehav` `behav`.
    fn mission_ship(mut npc: Npc, behav: i16) -> Npc {
        npc.mission = Some(crate::traffic::npc::MissionMark {
            order: None,
            slot: 1,
            behav: ShipBehav::from_code(behav),
            formation: None,
        });
        npc
    }

    /// The ships `npcs` with the player intact or `disabled` at the
    /// centre.
    fn with_player(npcs: &[Npc], condition: Condition) -> Surroundings<'_> {
        let mut player = crate::ai::fixture::player(0.0, 0.0);
        player.condition = condition;
        Surroundings {
            player: Some(player),
            ..Surroundings::new(&[], npcs)
        }
    }

    #[test]
    fn a_mission_ship_of_ship_behav_0_attacks_the_player_on_every_think() {
        let mark = Marked::new(7);
        let ai = NovaAi::default().with(AiType::Warship, mark.clone());
        let mut hunter = mission_ship(of(1, AiType::Warship), 0);
        hunter.goal = Goal::Attack(mark.ship());
        hunter.target = Some(mark.ship());
        let npcs = [hunter];
        let around = with_player(&npcs, Condition::Intact);
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Attack(ShipRef::Player)
        );
        assert_eq!(ai.target(&npcs[0], &around), Some(ShipRef::Player));
        assert_eq!(
            ai.trigger(&npcs[0], &around).only,
            Some(WeaponId(7)),
            "it fires as its type does"
        );
        let strike = Strike {
            ship: ShipRef::Npc(NpcId(1)),
            by: Some(ShipRef::Player),
            damage: 1.0,
            downed: None,
            direct: true,
            intended: true,
            lock: false,
        };
        assert_eq!(ai.react(&npcs[0], &strike, &around).provoked, 7.0);
        assert_eq!(
            *mark.asked.borrow(),
            [NpcId(1); 2],
            "trigger and react only"
        );
    }

    #[test]
    fn a_mission_ship_goes_back_to_its_types_ai_unmarked_or_of_another_behaviour() {
        let mark = Marked::new(7);
        let ai = NovaAi::default().with(AiType::Warship, mark.clone());
        let plain = of(1, AiType::Warship);
        let npcs = [
            plain.clone(),
            mission_ship(plain.clone(), -1),
            mission_ship(plain.clone(), 1),
            mission_ship(plain, 2),
        ];
        let around = with_player(&npcs, Condition::Intact);
        for npc in &npcs {
            assert_eq!(
                ai.decide(npc, &around, &mut Draws::of(&[])),
                Goal::Attack(mark.ship()),
                "{:?}",
                npc.mission
            );
            assert_eq!(ai.target(npc, &around), Some(mark.ship()));
        }
    }

    #[test]
    fn a_mission_escort_goes_to_the_escort_behaviour_whatever_its_ai_type() {
        let mark = Marked::new(7);
        let escorts = Marked::new(9);
        let ai = NovaAi::default()
            .with(AiType::Warship, mark.clone())
            .with_escorts(escorts.clone());
        let mut joined = mission_ship(of(1, AiType::Warship), 1);
        if let Some(mission) = &mut joined.mission {
            mission.formation = Some(crate::escort::FormationSlot::default());
        }
        let npcs = [joined];
        let around = with_player(&npcs, Condition::Intact);
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Attack(escorts.ship())
        );
        assert_eq!(ai.target(&npcs[0], &around), Some(escorts.ship()));
        assert_eq!(ai.trigger(&npcs[0], &around).only, Some(WeaponId(9)));
        let strike = Strike {
            ship: ShipRef::Npc(NpcId(1)),
            by: Some(ShipRef::Player),
            damage: 1.0,
            downed: None,
            direct: true,
            intended: true,
            lock: false,
        };
        assert_eq!(ai.react(&npcs[0], &strike, &around).provoked, 9.0);
        assert!(mark.asked.borrow().is_empty(), "not its type's");
    }

    #[test]
    fn a_ship_behav_0_ship_without_destroying_weapons_gives_up_on_a_disabled_player() {
        let mark = Marked::new(7);
        let ai = NovaAi::default().with(AiType::Warship, mark.clone());
        let npcs = [mission_ship(of(1, AiType::Warship), 0)];
        let disabled = with_player(&npcs, Condition::Disabled);
        assert_eq!(
            ai.decide(&npcs[0], &disabled, &mut Draws::of(&[])),
            Goal::Attack(mark.ship())
        );
        assert_eq!(ai.target(&npcs[0], &disabled), Some(mark.ship()));
        let dying = with_player(
            &npcs,
            Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
        );
        assert_eq!(
            ai.decide(&npcs[0], &dying, &mut Draws::of(&[])),
            Goal::Attack(mark.ship()),
            "a breaking-up player is disabled to _IsDisabled"
        );
        assert_eq!(ai.target(&npcs[0], &dying), Some(mark.ship()));
        let gone = Surroundings::new(&[], &npcs);
        assert_eq!(
            ai.decide(&npcs[0], &gone, &mut Draws::of(&[])),
            Goal::Attack(mark.ship()),
            "no player in the system"
        );
        let armed = [Npc {
            armament: crate::combat::armament::Armament::new([(
                crate::combat::weapon::WeaponSpec::new(&crate::catalog::WeaponRecord {
                    mass_dmg: 5,
                    ..nova_sim_catalog::fixture::weapon(128)
                }),
                1,
            )]),
            ..npcs[0].clone()
        }];
        assert!(fire::destroys(&armed[0]));
        let disabled = with_player(&armed, Condition::Disabled);
        assert_eq!(
            ai.decide(&armed[0], &disabled, &mut Draws::of(&[])),
            Goal::Attack(ShipRef::Player),
            "with destroying weapons it goes on"
        );
        let dying = with_player(
            &armed,
            Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
        );
        assert_eq!(
            ai.decide(&armed[0], &dying, &mut Draws::of(&[])),
            Goal::Attack(ShipRef::Player),
            "with destroying weapons it goes on while the player breaks up"
        );
        assert_eq!(ai.target(&armed[0], &dying), Some(ShipRef::Player));
    }

    #[test]
    fn a_ship_behav_0_warship_that_gives_up_on_a_disabled_player_turns_on_no_other_ship() {
        use crate::ai::fixture::{PIRATES, TRADERS, govts, ship};
        let govts = govts(0, 0);
        let npcs = [
            mission_ship(ship(1, TRADERS, AiType::Warship, 0.0, 0.0), 0),
            ship(2, PIRATES, AiType::Warship, 0.0, 50.0),
        ];
        let mut player = crate::ai::fixture::player(0.0, -100.0);
        player.condition = Condition::Disabled;
        let around = Surroundings {
            player: Some(player),
            govts: &govts,
            system_govt: Some(TRADERS),
            record: 30,
            ..Surroundings::new(&[], &npcs)
        };
        assert!(!fire::destroys(&npcs[0]));
        let ai = NovaAi::default();
        let goal = ai.decide(&npcs[0], &around, &mut Draws::of(&[0; 16]));
        assert_ne!(goal, Goal::Attack(ShipRef::Npc(NpcId(2))));
        assert_eq!(goal.quarry(), None, "{goal:?}");
        let free = [ship(1, TRADERS, AiType::Warship, 0.0, 0.0), npcs[1].clone()];
        let around = Surroundings {
            npcs: &free,
            ..around
        };
        assert_eq!(
            ai.decide(&free[0], &around, &mut Draws::of(&[0; 16])),
            Goal::Attack(ShipRef::Npc(NpcId(2))),
            "without the mark it fights the pirate"
        );
    }

    #[test]
    fn a_paid_off_ship_behav_0_ship_attacks_the_player_again_unless_the_rulebook_honours_the_bribe()
    {
        let mut hunter = mission_ship(of(1, AiType::WimpyTrader), 0);
        hunter.spared = true;
        let npcs = [hunter];
        let around = with_player(&npcs, Condition::Intact);
        let ai = NovaAi::from_rulebook(&Rulebook::default());
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[0; 16])),
            Goal::Attack(ShipRef::Player),
            "the engine: it attacks again"
        );
        assert_eq!(ai.target(&npcs[0], &around), Some(ShipRef::Player));
        let mark = Marked::new(7);
        let honoured = Rulebook::default().with_override(RuleKey::HunterBribe, RuleSource::Bible);
        let ai = NovaAi::from_rulebook(&honoured).with(AiType::WimpyTrader, mark.clone());
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[0; 16])),
            Goal::Attack(mark.ship()),
            "the bribe holds: it decides as its type does"
        );
        assert_eq!(ai.target(&npcs[0], &around), Some(mark.ship()));
        let unspared = [Npc {
            spared: false,
            ..npcs[0].clone()
        }];
        assert_eq!(
            ai.decide(
                &unspared[0],
                &with_player(&unspared, Condition::Intact),
                &mut Draws::of(&[0; 16])
            ),
            Goal::Attack(ShipRef::Player),
            "an unspared hunter still hunts"
        );
        assert_eq!(
            ai.target(&unspared[0], &with_player(&unspared, Condition::Intact)),
            Some(ShipRef::Player)
        );
    }

    #[test]
    fn nova_ai_reads_blessing_always_attacks_from_the_rulebook() {
        use crate::ai::fixture::{POLICE, around, blessing_of, govts, ship, sites};
        let govts = govts(0, crate::govt::ALWAYS_ATTACKS_PLAYER);
        let sites = sites();
        for ai_type in [AiType::Warship, AiType::Interceptor] {
            let npcs = [ship(1, POLICE, ai_type, 0.0, 0.0)];
            let mut seen = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
            if let Some(player) = seen.player.as_mut() {
                player.blessing = blessing_of(POLICE);
            }
            let decide = |rulebook: &Rulebook| {
                NovaAi::from_rulebook(rulebook).decide(&npcs[0], &seen, &mut Draws::of(&[0; 4]))
            };
            assert_eq!(
                decide(&Rulebook::default()),
                Goal::Attack(ShipRef::Player),
                "{ai_type:?}: the engine"
            );
            let held = Rulebook::default()
                .with_override(RuleKey::BlessingAlwaysAttacks, RuleSource::Bible);
            assert_ne!(
                decide(&held),
                Goal::Attack(ShipRef::Player),
                "{ai_type:?}: the blessing holds the word back"
            );
        }
    }

    #[test]
    fn nova_ai_reads_patrol_uninhabited_from_the_rulebook() {
        use crate::ai::fixture::{POLICE, govts, ship};
        use crate::catalog::{LandingSite, StellarId};
        use crate::landing::StellarFlags;
        let govts = govts(0, 0);
        let sites = [
            LandingSite {
                flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
                ..nova_sim_catalog::fixture::planet(129, 0.0, 0.0)
            },
            nova_sim_catalog::fixture::planet(128, 0.0, 0.0),
        ];
        let npcs = [ship(1, POLICE, AiType::Interceptor, 0.0, 0.0)];
        let seen = crate::ai::Surroundings {
            govts: &govts,
            ..crate::ai::Surroundings::new(&sites, &npcs)
        };
        let decide = |rulebook: &Rulebook| {
            NovaAi::from_rulebook(rulebook).decide(&npcs[0], &seen, &mut Draws::of(&[0]))
        };
        assert_eq!(decide(&Rulebook::default()), Goal::Land(StellarId(129)));
        let inhabited =
            Rulebook::default().with_override(RuleKey::PatrolUninhabited, RuleSource::Bible);
        assert_eq!(decide(&inhabited), Goal::Land(StellarId(128)));
    }

    /// Turns on the player in each decision, its goal idle.
    #[derive(Debug)]
    struct Turning;

    impl Behaviour for Turning {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }

        fn decision(
            &self,
            _npc: &Npc,
            _around: &Surroundings,
            _chance: &mut dyn Chance,
        ) -> Decision {
            Decision {
                goal: Goal::Idle,
                turns_on_player: true,
                dropped_target: true,
                provokes: false,
            }
        }
    }

    #[test]
    fn nova_ai_carries_each_types_turn_and_its_mission_hunt_turns_on_the_player() {
        use crate::ai::fixture::{POLICE, around, govts, ship, sites};
        let hunters = [mission_ship(of(1, AiType::Warship), 0)];
        assert_eq!(
            NovaAi::default().decision(
                &hunters[0],
                &with_player(&hunters, Condition::Intact),
                &mut Draws::of(&[])
            ),
            Decision {
                goal: Goal::Attack(ShipRef::Player),
                turns_on_player: true,
                dropped_target: false,
                provokes: false,
            },
            "the mission hunt (@0x8d6b7)"
        );
        let ai = NovaAi::default().with(AiType::BraveTrader, Rc::new(Turning));
        let brave = [of(2, AiType::BraveTrader)];
        let around_brave = with_player(&brave, Condition::Intact);
        assert_eq!(
            ai.decision(&brave[0], &around_brave, &mut Draws::of(&[])),
            Turning.decision(&brave[0], &around_brave, &mut Draws::of(&[])),
            "its type's, whole"
        );
        let govts = govts(0, crate::govt::ALWAYS_ATTACKS_PLAYER);
        let sites = sites();
        let police = [ship(3, POLICE, AiType::Warship, 0.0, 0.0)];
        let seen = around(&sites, &police, &govts, (0.0, -1000.0), 0);
        assert!(
            NovaAi::default()
                .decision(&police[0], &seen, &mut Draws::of(&[0; 4]))
                .turns_on_player,
            "a 0x0004 warship's word"
        );
        let escort = [escorting(ship(4, POLICE, AiType::Warship, 0.0, 0.0))];
        let seen = around(&sites, &escort, &govts, (0.0, -1000.0), 0);
        assert!(
            !NovaAi::default()
                .decision(&escort[0], &seen, &mut Draws::of(&[0; 4]))
                .turns_on_player,
            "an escort"
        );
        let trader = [ship(5, POLICE, AiType::WimpyTrader, 0.0, 0.0)];
        let seen = around(&sites, &trader, &govts, (0.0, -1000.0), 0);
        assert!(
            !NovaAi::default()
                .decision(&trader[0], &seen, &mut Draws::of(&[0; 4]))
                .turns_on_player,
            "a trader"
        );
    }
}
