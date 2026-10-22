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
//! and leader 0 on every think), with no standing order. A ship with an
//! NPC parent, whatever its AI type, goes to [`FleetEscortAi`], which
//! flies `_EscortAI` under its lead: a fleet's escort or a ship a pirate
//! took into its fleet (a [`leader`](Npc::leader), the original's AI type
//! 5), until it leaves its fleet ([`Decision::leaves_fleet`]) or its
//! lead's loss gives it another lead (`traffic::succession`). By
//! the other reading of the rulebook's [`RuleKey::NpcEscortAi`], the
//! port's earlier routing, a fighter goes to [`CarriedAi`] and a led ship
//! to its AI type's behaviour, following its lead.
//! A leaderless miner (`shïp` `Flags3` & 3 with leader +0x9a -1,
//! `_AIDispatch` @0x8ff64-0x8ff96), whatever its AI type, decides and
//! targets as [`MinerAi`] instead of its type's think, but fires, waits in
//! its gate and answers hits as its type does, as `_AIDispatch` replaces
//! only the type's call. So a warship-typed miner a Good Samaritan's call
//! turns on the player (`react`'s reaction goal) attacks until its next
//! think, which the miner's AI then decides, as the original's next
//! `_MinerAI` does; the type's fire reads the goal, and a miner's own
//! goals fire at nothing but an attacker it flees. A mission ship of
//! `ShipBehav` 0 still hunts the player, miner or not; with the player
//! disabled and no destroying weapon it gives up and leaves by hyperspace
//! after its type's think, whatever that decided and whatever its fuel
//! (`_HighLevelAIHandler` @0x8d6be-0x8d6e9; see [`leave`](crate::ai::leave)).
//! A leaderless warship of a government with `Flags` 0x1000 goes to the
//! pirate's AI ([`PirateWarship`], `_PirateWarshipAI`, `_AIDispatch`
//! @0x8ffc8-0x8fff0), which disables, docks with and plunders its victim
//! (see [`plunder`](crate::traffic::plunder)); a pirate with a leader is
//! one of its fleet's escorts.
//!
//! Each decision ([`Behaviour::decision`]) carries whether the think
//! turned the ship on the player through `_AIMakeShipAttackPlayer`: the
//! type's own, or a `ShipBehav` 0 mission ship's hunt (`_HighLevelAIHandler`
//! @0x8d6b7), whose quote gate its mark always fails (+0xc8d2, @0x89c71);
//! and whether it calls for reinforcements, the type's own and
//! `_HighLevelAIHandler`'s after it (see
//! [`reinforcements`](crate::ai::reinforcements)).

use std::rc::Rc;

use crate::ai::fire::FireRules;
use crate::ai::flee_drop;
use crate::ai::lost_sight;
use crate::ai::miner::{self, MinerAi};
use crate::ai::pirate::PirateWarship;
use crate::ai::target::TargetRules;
use crate::ai::{
    Behaviour, BraveTrader, CarriedAi, Decision, EscortAi, FleetEscortAi, GateWait, Goal,
    Interceptor, Reaction, Surroundings, Warship, WimpyTrader, reinforcements,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::govt::PLUNDERS;
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
    fleet_escorts: Rc<dyn Behaviour>,
    miner: Rc<dyn Behaviour>,
    pirate: Rc<dyn Behaviour>,
    /// The source of [`RuleKey::HunterBribe`]: whether a paid-off
    /// `ShipBehav` 0 hunter attacks the player again.
    hunter_bribe: RuleSource,
    /// The source of [`RuleKey::LedAllyAttack`]: which allies a fleeing
    /// ship gives up fleeing ([`flee_drop`]).
    led_ally: RuleSource,
    /// The source of [`RuleKey::NpcEscortAi`]: whether a ship with an NPC
    /// parent flies [`FleetEscortAi`], or as the port flew it before.
    npc_escort_ai: RuleSource,
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
    /// [`RuleKey::PatrolUninhabited`] entry says, the ships with an NPC
    /// parent fly as its [`RuleKey::NpcEscortAi`] entry says,
    /// and every one of them, the escorts, the fleet escorts and the
    /// carried fighters aim their quadrant turrets as its
    /// [`RuleKey::QuadrantTurretArc`] entry says.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        let piracy_police = rulebook.source_for(RuleKey::PiracyPolice);
        let person_coward = rulebook.source_for(RuleKey::PersonCoward);
        let always_attacks = rulebook.source_for(RuleKey::BlessingAlwaysAttacks);
        let fire = FireRules {
            quadrant_arc: rulebook.source_for(RuleKey::QuadrantTurretArc),
        };
        let led_ally = rulebook.source_for(RuleKey::LedAllyAttack);
        let target = TargetRules {
            led_ally,
            friend_carry: rulebook.source_for(RuleKey::FriendShieldCarry),
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
                target,
            }),
            interceptor: Rc::new(Interceptor {
                piracy_police,
                person_coward,
                always_attacks,
                fire,
                patrol_uninhabited: rulebook.source_for(RuleKey::PatrolUninhabited),
                target,
            }),
            escorts: Rc::new(EscortAi {
                escort_ai: rulebook.source_for(RuleKey::EscortAi),
                fire,
            }),
            carried: Rc::new(CarriedAi { fire }),
            fleet_escorts: Rc::new(FleetEscortAi { fire }),
            miner: Rc::new(MinerAi),
            pirate: Rc::new(PirateWarship {
                piracy_police,
                fire,
                target,
            }),
            hunter_bribe: rulebook.source_for(RuleKey::HunterBribe),
            led_ally,
            npc_escort_ai: rulebook.source_for(RuleKey::NpcEscortAi),
        }
    }

    /// This AI with `behaviour` for the player's escorts.
    #[must_use]
    pub fn with_escorts(mut self, behaviour: Rc<dyn Behaviour>) -> Self {
        self.escorts = behaviour;
        self
    }

    /// This AI with `behaviour` for an NPC carrier's fighters under the
    /// other reading of [`RuleKey::NpcEscortAi`].
    #[must_use]
    pub fn with_carried(mut self, behaviour: Rc<dyn Behaviour>) -> Self {
        self.carried = behaviour;
        self
    }

    /// This AI with `behaviour` for the ships with an NPC parent, a
    /// fleet's escorts and an NPC carrier's fighters, under the engine's
    /// reading of [`RuleKey::NpcEscortAi`].
    #[must_use]
    pub fn with_fleet_escorts(mut self, behaviour: Rc<dyn Behaviour>) -> Self {
        self.fleet_escorts = behaviour;
        self
    }

    /// This AI with `behaviour` for the leaderless miners' thinks.
    #[must_use]
    pub fn with_miner(mut self, behaviour: Rc<dyn Behaviour>) -> Self {
        self.miner = behaviour;
        self
    }

    /// This AI with `behaviour` for the pirate warships: the leaderless
    /// warships of a government with `Flags` 0x1000 ([`PLUNDERS`]).
    #[must_use]
    pub fn with_pirate(mut self, behaviour: Rc<dyn Behaviour>) -> Self {
        self.pirate = behaviour;
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

    /// The behaviour for `npc` among `around`: the escorts' for the
    /// player's escort (its fighters too) and for a mission escort
    /// ([`Npc::mission_escort`]); for a ship with an NPC parent, a leader
    /// or a carrier (the original's AI types 6 and 5, `_AIDispatch`
    /// @0x90011-0x90016), the fleet escorts' ([`FleetEscortAi`]) by the
    /// engine's reading of [`RuleKey::NpcEscortAi`], and by its other
    /// reading the carried fighters' for one an NPC launched and its own
    /// AI type's for a led ship; the pirate's for a leaderless warship of
    /// a government with `Flags` 0x1000 ([`PLUNDERS`], `_AIDispatch`
    /// @0x8ffc8-0x8fff0); otherwise the one for its AI type.
    fn of(&self, npc: &Npc, around: &Surroundings) -> &dyn Behaviour {
        if npc.escorts_player() {
            return &*self.escorts;
        }
        if (npc.leader.is_some() || npc.carrier.is_some())
            && self.npc_escort_ai == RuleSource::Engine
        {
            return &*self.fleet_escorts;
        }
        if npc.carrier.is_some() {
            return &*self.carried;
        }
        match npc.ai_type {
            AiType::WimpyTrader => &*self.wimpy,
            AiType::BraveTrader => &*self.brave,
            AiType::Warship if npc.leader.is_none() && around.govts.flag(npc.govt, PLUNDERS) => {
                &*self.pirate
            }
            AiType::Warship => &*self.warship,
            AiType::Interceptor => &*self.interceptor,
        }
    }

    /// The behaviour that decides for `npc` and picks its target: the
    /// miners' for a leaderless miner ([`miner::thinks_for`]), otherwise
    /// its own ([`NovaAi::of`]). A miner's fire, gate wait and answer to
    /// hits stay its own: `_AIDispatch` replaces only the type's think.
    fn thinker(&self, npc: &Npc, around: &Surroundings) -> &dyn Behaviour {
        if miner::thinks_for(npc) {
            &*self.miner
        } else {
            self.of(npc, around)
        }
    }
}

impl NovaAi {
    /// Whether `npc`, a mission's special ship of `ShipBehav` 0, attacks
    /// the player now among `around` (`Some(true)`) or gives up on it
    /// (`Some(false)`), on every think (see
    /// [`mission_hunt`](crate::ai::target::mission_hunt)); `None` for any
    /// other ship, which decides as its type does. A ship the player paid
    /// off hunts again by the engine, and decides as its type does under
    /// [`RuleSource::Bible`] for [`RuleKey::HunterBribe`]
    /// (`_HighLevelAIHandler` @0x8d628 never reads the bribe). Only this
    /// site reads the key: a paid-off ship is a wimpy trader, which never
    /// asks [`select_target`](crate::ai::target::select_target).
    fn mission_hunt(&self, npc: &Npc, around: &Surroundings) -> Option<bool> {
        let attacks = crate::ai::target::mission_hunt(npc, around)?;
        if attacks && npc.spared && self.hunter_bribe == RuleSource::Bible {
            return None;
        }
        Some(attacks)
    }
}

impl Behaviour for NovaAi {
    /// [`NovaAi::decision`]'s goal.
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    /// A mission ship of `ShipBehav` 0 attacks the player
    /// ([`NovaAi::mission_hunt`]), turning on it through
    /// `_AIMakeShipAttackPlayer` (`_HighLevelAIHandler` @0x8d6b7), whose
    /// quote gate its mark fails (+0xc8d2, @0x89c71). One that gives up on
    /// the player leaves by hyperspace (@0x8d6be-0x8d6e9: state 2, its
    /// target -1), after its type's think, which runs first (`_AIDispatch`
    /// @0x8ff43-0x90016, then @0x901e5) and so is overridden, with no fuel
    /// test: even a warship that cannot jump leaves, as its own fallback
    /// ran before it. The give-up's +0x50 clamp and +0xa4 stamp are the
    /// warp stage's (rdm:phase/ship-handling-fidelity/phase-8-warp-out).
    /// Every other NPC decides as its behaviour does, its turn on the
    /// player with it. Either way `_HighLevelAIHandler`'s lost-sight block
    /// follows ([`lost_sight::after_think`], @0x8d6ef-0x8d8ae): a ship
    /// that can't see its target searches, gives up, leaves or waits.
    /// Unless that search or give-up ended the handler
    /// ([`lost_sight::ends_high_level`], the jump to @0x8fb47), a flight
    /// from a ship gone from its slot ends with its provocation
    /// ([`flight_ended`](crate::traffic::provocation::flight_ended),
    /// @0x8d4e4-0x8d50d → @0x8e228-0x8e23d), and its flee
    /// drop ([`flee_drop::with_flee_drop`]) and its call for
    /// reinforcements end the think
    /// ([`reinforcements::with_high_level_call`]), whatever type decided,
    /// as the handler runs after every type's think (`_AIDispatch`
    /// @0x901e5).
    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        let decision = match self.mission_hunt(npc, around) {
            Some(true) => Decision {
                goal: Goal::Attack(ShipRef::Player),
                turns_on_player: true,
                dropped_target: false,
                provokes: false,
                forgets_provocation: false,
                comm_call: None,
                calls_reinforcements: false,
                board_wait: None,
                clears_target: false,
                keeps_pursuit: false,
                plunders: None,
                lost_sight: None,
                travel: crate::ai::travel::TravelWrite::Keep,
                visits: None,
                re_picked: false,
                ends_wait: false,
                order_word: None,
                leaves_fleet: false,
                follows_jump: false,
                voice: None,
            },
            Some(false) => Decision {
                goal: Goal::JumpOut,
                turns_on_player: false,
                clears_target: true,
                ..self.thinker(npc, around).decision(npc, around, chance)
            },
            None => self.thinker(npc, around).decision(npc, around, chance),
        };
        let decision = lost_sight::after_think(npc, decision, around, chance);
        if lost_sight::ends_high_level(&decision) {
            return decision;
        }
        // A think that clears its target has ended its fight itself, so
        // the high level finds no flight of the old goal's to end.
        let decision = Decision {
            forgets_provocation: decision.forgets_provocation
                || (!decision.clears_target
                    && crate::traffic::provocation::flight_ended(npc, decision.goal, around)),
            ..decision
        };
        let decision = flee_drop::with_flee_drop(npc, decision, around, self.led_ally);
        reinforcements::with_high_level_call(npc, decision, around)
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        self.of(npc, around).trigger(npc, around)
    }

    /// A mission ship of `ShipBehav` 0 targets the player, and none once
    /// it gives up on it ([`NovaAi::mission_hunt`]).
    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        match self.mission_hunt(npc, around) {
            Some(true) => Some(ShipRef::Player),
            Some(false) => None,
            None => self.thinker(npc, around).target(npc, around),
        }
    }

    fn gate_wait(&self, npc: &Npc, around: &Surroundings) -> GateWait {
        self.of(npc, around).gate_wait(npc, around)
    }

    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        self.of(npc, around).react(npc, strike, around)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::ai::fire;
    use crate::ai::miner::{MINER_BITS, MINES, SCOOPS};
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
            damage: 0.0,
            ..crate::combat::fixture::strike(ShipRef::Player, ShipRef::Player)
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
            damage: 0.0,
            ..crate::combat::fixture::strike(ShipRef::Player, ShipRef::Player)
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
    fn by_the_other_reading_an_npc_carriers_fighter_goes_to_the_carried_behaviour() {
        let marks: Vec<Rc<Marked>> = (1..=4).map(Marked::new).collect();
        let fighters = Marked::new(8);
        let escorts = Marked::new(9);
        let bible = Rulebook::default().with_override(RuleKey::NpcEscortAi, RuleSource::Bible);
        let ai = TYPES
            .into_iter()
            .zip(&marks)
            .fold(NovaAi::from_rulebook(&bible), |ai, (ai_type, mark)| {
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
        let strike = crate::combat::fixture::strike(ShipRef::Player, ShipRef::Player);
        for npc in &npcs {
            assert_eq!(
                ai.decide(npc, &around, &mut Draws::of(&[])),
                Goal::Attack(fighters.ship())
            );
            assert_eq!(ai.trigger(npc, &around).only, Some(WeaponId(8)));
            assert_eq!(ai.target(npc, &around), Some(fighters.ship()));
            assert_eq!(ai.react(npc, &strike, &around).provoked, 8.0);
        }
        assert_eq!(fighters.asked.borrow().len(), 16);
        assert!(marks.iter().all(|mark| mark.asked.borrow().is_empty()));
        // The player's fighter is the player's escort.
        let players = escorting(carried(of(20, AiType::Warship), ShipRef::Player));
        let npcs = [players];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Attack(escorts.ship())
        );
        assert_eq!(fighters.asked.borrow().len(), 16, "not the carried slot");
        // Once its carrier is gone, it flies by its AI type.
        let orphan = of(21, AiType::Warship);
        let npcs = [orphan];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Attack(marks[2].ship())
        );
    }

    /// `npc` led by NPC 30.
    fn led_by_30(mut npc: Npc) -> Npc {
        npc.leader = Some(NpcId(30));
        npc
    }

    /// Nova's AI as `rulebook` says, with marks 1-4 for the four types, 8
    /// for the carried fighters and 7 for the NPC-led ships.
    fn fleet_marked(rulebook: &Rulebook) -> (NovaAi, Vec<Rc<Marked>>, [Rc<Marked>; 2]) {
        let marks: Vec<Rc<Marked>> = (1..=4).map(Marked::new).collect();
        let others = [Marked::new(8), Marked::new(7)];
        let ai = TYPES
            .into_iter()
            .zip(&marks)
            .fold(NovaAi::from_rulebook(rulebook), |ai, (ai_type, mark)| {
                ai.with(ai_type, mark.clone())
            })
            .with_carried(others[0].clone())
            .with_fleet_escorts(others[1].clone());
        (ai, marks, others)
    }

    /// Every one of `mark`'s answers, as [`routed`] reads them.
    fn all_of(mark: &Marked) -> (Goal, Option<ShipRef>, [f32; 3]) {
        (
            Goal::Attack(mark.ship()),
            Some(mark.ship()),
            [mark.mark as f32; 3],
        )
    }

    #[test]
    fn by_default_a_led_ship_and_an_npc_carriers_fighter_go_to_the_fleet_escort_behaviour() {
        // The original's AI types 6 and 5 under an NPC parent: `_AIDispatch`
        // calls `_EscortAI` for AI type above 4 (@0x90011-0x90016).
        let (ai, marks, [fighters, fleet]) = fleet_marked(&Rulebook::default());
        let npcs: Vec<Npc> = TYPES
            .into_iter()
            .enumerate()
            .flat_map(|(i, ai_type)| {
                let id = 10 + 2 * i as u32;
                [
                    led_by_30(of(id, ai_type)),
                    carried(of(id + 1, ai_type), ShipRef::Npc(NpcId(30))),
                ]
            })
            .collect();
        let around = Surroundings::new(&[], &npcs);
        for npc in &npcs {
            assert_eq!(routed(&ai, npc, &around), all_of(&fleet), "{:?}", npc.id);
        }
        assert_eq!(fleet.asked.borrow().len(), 5 * npcs.len());
        assert!(fighters.asked.borrow().is_empty());
        assert!(marks.iter().all(|mark| mark.asked.borrow().is_empty()));
        // The player's escorts and their fighters stay the escorts'.
        let players = escorting(carried(of(20, AiType::Warship), ShipRef::Player));
        let around = Surroundings::new(&[], std::slice::from_ref(&players));
        assert_ne!(routed(&ai, &players, &around), all_of(&fleet));
    }

    #[test]
    fn by_the_other_reading_a_led_ship_flies_by_its_type_and_a_fighter_by_the_carried_ai() {
        let bible = Rulebook::default().with_override(RuleKey::NpcEscortAi, RuleSource::Bible);
        let (ai, marks, [fighters, fleet]) = fleet_marked(&bible);
        for (ai_type, mark) in TYPES.into_iter().zip(&marks) {
            let npcs = [
                led_by_30(of(10, ai_type)),
                carried(of(11, ai_type), ShipRef::Npc(NpcId(30))),
            ];
            let around = Surroundings::new(&[], &npcs);
            assert_eq!(routed(&ai, &npcs[0], &around), all_of(mark), "{ai_type:?}");
            assert_eq!(
                routed(&ai, &npcs[1], &around),
                all_of(&fighters),
                "{ai_type:?}"
            );
        }
        assert!(fleet.asked.borrow().is_empty());
    }

    #[test]
    fn nova_ai_flies_a_fleet_escort_under_its_lead_as_its_rulebook_says() {
        use crate::ai::fixture::{PIRATES, TRADERS, ship};
        let mut lead = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        lead.goal = Goal::Attack(ShipRef::Npc(NpcId(4)));
        let mut escort = ship(2, PIRATES, AiType::WimpyTrader, 0.0, 50.0);
        escort.leader = Some(NpcId(1));
        escort.order_word = crate::fleet::OrderWord(2);
        let mut raider = ship(3, TRADERS, AiType::Warship, 0.0, 300.0);
        raider.goal = Goal::Attack(ShipRef::Npc(NpcId(1)));
        let victim = ship(4, TRADERS, AiType::WimpyTrader, 0.0, -300.0);
        let npcs = [lead, escort, raider, victim];
        let govts = crate::ai::fixture::govts(0, 0);
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let decide = |ai: &NovaAi| ai.decision(&npcs[1], &around, &mut Draws::of(&[0]));
        let decision = decide(&NovaAi::default());
        assert_eq!(
            decision.goal,
            Goal::Attack(ShipRef::Npc(NpcId(3))),
            "its stored Attack: the nearest threat to its lead"
        );
        assert!(decision.ends_wait);
        let bible = Rulebook::default().with_override(RuleKey::NpcEscortAi, RuleSource::Bible);
        let decision = decide(&NovaAi::from_rulebook(&bible));
        assert_eq!(
            decision.goal,
            Goal::Follow {
                lead: NpcId(1),
                guard: None
            },
            "its trader's way: following its lead"
        );
        assert!(!decision.ends_wait);
    }

    #[test]
    fn novas_escorts_and_carried_fighters_answer_as_victims_and_never_as_samaritans() {
        // `_DamageShip`'s reaction block reads no AI type (@0x3bb7a-0x3bed2);
        // `_DoGoodSamaritan` takes AI 3 and 4 alone (@0x8289f-0x828bb).
        let ai = NovaAi::default();
        let mut trader = of(1, AiType::WimpyTrader);
        trader.govt = Some(crate::catalog::GovtId(140));
        let mut warship = of(2, AiType::Warship);
        warship.state.position = crate::geometry::Vec2::new(3000.0, 0.0);
        let escort = escorting(Npc {
            state: warship.state,
            ..of(3, AiType::Warship)
        });
        let fighter = carried(
            Npc {
                state: warship.state,
                ..of(4, AiType::Warship)
            },
            ShipRef::Npc(NpcId(9)),
        );
        let npcs = [trader, warship, escort, fighter];
        let around = Surroundings::new(&[], &npcs);
        let on_trader = crate::combat::fixture::strike(ShipRef::Npc(NpcId(1)), ShipRef::Player);
        assert_eq!(
            ai.react(&npcs[1], &on_trader, &around).goal,
            Some(Goal::Attack(ShipRef::Player)),
            "a warship comes to its help"
        );
        for npc in &npcs[2..] {
            assert_eq!(
                ai.react(npc, &on_trader, &around),
                Reaction::default(),
                "{:?}",
                npc.id
            );
            let on_it =
                crate::combat::fixture::strike(ShipRef::Npc(npc.id), ShipRef::Npc(NpcId(1)));
            assert_eq!(
                ai.react(npc, &on_it, &around),
                Reaction {
                    target: Some(ShipRef::Npc(NpcId(1))),
                    provoked: 24.0,
                    goal: None,
                },
                "{:?}",
                npc.id
            );
        }
    }

    /// `npc` as a miner of `Flags3` `bits`.
    fn mining(mut npc: Npc, bits: u16) -> Npc {
        npc.miner = bits;
        npc
    }

    /// Nova's AI with marks 1-4 for the four types, 9 for the escorts, 8
    /// for the NPC-led ships and carried fighters (the fleet escorts' and,
    /// unused by default, the carried fighters') and 5 for the miners.
    fn marked() -> (NovaAi, Vec<Rc<Marked>>, [Rc<Marked>; 3]) {
        let marks: Vec<Rc<Marked>> = (1..=4).map(Marked::new).collect();
        let others = [Marked::new(9), Marked::new(8), Marked::new(5)];
        let ai = TYPES
            .into_iter()
            .zip(&marks)
            .fold(NovaAi::default(), |ai, (ai_type, mark)| {
                ai.with(ai_type, mark.clone())
            })
            .with_escorts(others[0].clone())
            .with_carried(others[1].clone())
            .with_fleet_escorts(others[1].clone())
            .with_miner(others[2].clone());
        (ai, marks, others)
    }

    /// The traders' government plundering: its warships are pirates.
    fn plundering() -> crate::govt::Governments {
        crate::ai::fixture::govts(crate::govt::PLUNDERS, 0)
    }

    /// NPC `id` of `ai_type`, of the plundering traders' government.
    fn traders(id: u32, ai_type: AiType) -> Npc {
        Npc {
            govt: Some(crate::ai::fixture::TRADERS),
            ..of(id, ai_type)
        }
    }

    /// What each of `ai`'s routes answers for `npc` among `around`:
    /// its decision's goal, its target, its trigger, its gate wait's
    /// trigger and its answer's provocation, as the marks carry them.
    fn routed(ai: &NovaAi, npc: &Npc, around: &Surroundings) -> (Goal, Option<ShipRef>, [f32; 3]) {
        let strike = Strike {
            damage: 0.0,
            ..crate::combat::fixture::strike(ShipRef::Player, ShipRef::Player)
        };
        let weapon = |trigger: Trigger| trigger.only.map_or(0.0, |weapon| f32::from(weapon.0));
        (
            ai.decide(npc, around, &mut Draws::of(&[0])),
            ai.target(npc, around),
            [
                weapon(ai.trigger(npc, around)),
                weapon(ai.gate_wait(npc, around).trigger),
                ai.react(npc, &strike, around).provoked,
            ],
        )
    }

    #[test]
    fn a_leaderless_warship_of_a_plundering_government_goes_to_the_pirate() {
        let (ai, marks, _) = marked();
        let pirates = Marked::new(6);
        let ai = ai.with_pirate(pirates.clone());
        let govts = plundering();
        let npcs = [traders(1, AiType::Warship)];
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            routed(&ai, &npcs[0], &around),
            (Goal::Attack(pirates.ship()), Some(pirates.ship()), [6.0; 3])
        );
        assert_eq!(pirates.asked.borrow().len(), 5);
        assert!(marks.iter().all(|mark| mark.asked.borrow().is_empty()));
    }

    #[test]
    fn no_other_ship_of_a_plundering_government_goes_to_the_pirate() {
        let (ai, marks, [escorts, fighters, miners]) = marked();
        let pirates = Marked::new(6);
        let ai = ai.with_pirate(pirates.clone());
        let govts = plundering();
        let mut led = traders(5, AiType::Warship);
        led.leader = Some(NpcId(9));
        let mut police = of(6, AiType::Warship);
        police.govt = Some(crate::ai::fixture::POLICE);
        let npcs = [
            traders(1, AiType::Interceptor),
            of(2, AiType::Warship),
            escorting(traders(3, AiType::Warship)),
            carried(traders(4, AiType::Warship), ShipRef::Npc(NpcId(30))),
            led,
            police,
            traders(7, AiType::BraveTrader),
        ];
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let all = |mark: &Marked| {
            (
                Goal::Attack(mark.ship()),
                Some(mark.ship()),
                [mark.mark as f32; 3],
            )
        };
        let wanted = [
            all(&marks[3]),
            all(&marks[2]),
            all(&escorts),
            all(&fighters),
            all(&fighters),
            all(&marks[2]),
            all(&marks[1]),
        ];
        for (npc, wanted) in npcs.iter().zip(wanted) {
            assert_eq!(routed(&ai, npc, &around), wanted, "{:?}", npc.id);
        }
        assert!(pirates.asked.borrow().is_empty());
        // A leaderless miner thinks as the miners' AI, but fires, waits and
        // answers as its type, the pirate (`_AIDispatch` @0x8ff64).
        let miner = [mining(traders(8, AiType::Warship), MINER_BITS)];
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &miner)
        };
        assert_eq!(
            routed(&ai, &miner[0], &around),
            (Goal::Attack(miners.ship()), Some(miners.ship()), [6.0; 3])
        );
        // A `ShipBehav` 0 mission ship hunts the player, firing as the pirate.
        let hunter = [mission_ship(traders(9, AiType::Warship), 0)];
        let mut around = with_player(&hunter, Condition::Intact);
        around.govts = &govts;
        assert_eq!(
            routed(&ai, &hunter[0], &around),
            (
                Goal::Attack(ShipRef::Player),
                Some(ShipRef::Player),
                [6.0; 3]
            )
        );
    }

    #[test]
    fn by_default_a_pirates_plunder_calls_no_reinforcements_and_its_attack_does() {
        use crate::ai::fixture::{NEUTRAL, ship};
        let govts = plundering();
        let mut pirate = ship(1, crate::ai::fixture::TRADERS, AiType::Warship, 0.0, 0.0);
        pirate.berth = 1;
        let mut wreck = ship(2, NEUTRAL, AiType::WimpyTrader, 0.0, 300.0);
        wreck.condition = Condition::Disabled;
        wreck.hull.crew = 1;
        wreck.hull.inherent_ai = 2;
        let npcs = [pirate.clone(), wreck.clone()];
        let around = Surroundings {
            govts: &govts,
            ..with_player(&npcs, Condition::Intact)
        };
        let decision = NovaAi::default().decision(&npcs[0], &around, &mut Draws::of(&[0]));
        assert_eq!(
            decision.goal,
            Goal::Plunder {
                ship: ShipRef::Npc(NpcId(2)),
                hopeless: false
            }
        );
        assert!(!decision.calls_reinforcements, "goal 0xd has no call");
        let mut warship = wreck;
        warship.condition = Condition::Intact;
        warship.hull.inherent_ai = 3;
        pirate.goal = Goal::Attack(ShipRef::Npc(NpcId(2)));
        pirate.target = Some(ShipRef::Npc(NpcId(2)));
        let npcs = [pirate, warship];
        let around = Surroundings {
            govts: &govts,
            ..with_player(&npcs, Condition::Intact)
        };
        let decision = NovaAi::default().decision(&npcs[0], &around, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, Goal::Attack(ShipRef::Npc(NpcId(2))));
        assert!(decision.calls_reinforcements, "goal 4's (@0x8e70d)");
    }

    #[test]
    fn a_leaderless_miner_of_any_ai_type_decides_and_targets_as_the_miners_ai() {
        let (ai, marks, [_, _, miners]) = marked();
        let strike = Strike {
            ship: ShipRef::Player,
            by: Some(ShipRef::Player),
            damage: 0.0,
            downed: None,
            direct: true,
            intended: true,
            lock: false,
            nominal: crate::combat::Nominal::default(),
            weight: 0.0,
            answered: true,
        };
        for bits in [MINES, SCOOPS, MINER_BITS] {
            let npcs: Vec<Npc> = TYPES
                .into_iter()
                .enumerate()
                .map(|(i, ai_type)| mining(of(10 + i as u32, ai_type), bits))
                .collect();
            let around = Surroundings::new(&[], &npcs);
            for (npc, mark) in npcs.iter().zip(&marks) {
                assert_eq!(
                    ai.decide(npc, &around, &mut Draws::of(&[])),
                    Goal::Attack(miners.ship())
                );
                assert_eq!(ai.target(npc, &around), Some(miners.ship()));
                let wanted = Some(WeaponId(i16::try_from(mark.mark).unwrap_or_default()));
                assert_eq!(ai.trigger(npc, &around).only, wanted, "its type's fire");
                assert_eq!(ai.gate_wait(npc, &around).trigger.only, wanted);
                assert_eq!(ai.react(npc, &strike, &around).provoked, mark.mark as f32);
                assert_eq!(mark.asked.borrow_mut().drain(..).count(), 3);
            }
        }
        assert_eq!(miners.asked.borrow().len(), 24);
    }

    #[test]
    fn a_led_miner_flies_under_its_lead_not_as_a_miner() {
        let (ai, _, [_, fleet, miners]) = marked();
        let mut led = mining(of(2, AiType::Warship), MINER_BITS);
        led.leader = Some(NpcId(1));
        let npcs = [of(1, AiType::Warship), led];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(
            ai.decide(&npcs[1], &around, &mut Draws::of(&[])),
            Goal::Attack(fleet.ship())
        );
        assert!(miners.asked.borrow().is_empty());
        let nova = NovaAi::default().with_miner(miners.clone());
        let mut trader = mining(of(2, AiType::WimpyTrader), SCOOPS);
        trader.leader = Some(NpcId(1));
        let npcs = [of(1, AiType::WimpyTrader), trader];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(
            nova.decide(&npcs[1], &around, &mut Draws::of(&[])),
            Goal::Follow {
                lead: NpcId(1),
                guard: None
            }
        );
        assert!(miners.asked.borrow().is_empty());
    }

    #[test]
    fn a_miner_escorting_the_player_or_carried_goes_to_the_escorts_or_fighters_ai() {
        let (ai, _, [escorts, fighters, miners]) = marked();
        let npcs = [
            escorting(mining(of(1, AiType::Warship), MINER_BITS)),
            carried(
                mining(of(2, AiType::Warship), MINER_BITS),
                ShipRef::Npc(NpcId(30)),
            ),
        ];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Attack(escorts.ship())
        );
        assert_eq!(
            ai.decide(&npcs[1], &around, &mut Draws::of(&[])),
            Goal::Attack(fighters.ship())
        );
        assert!(miners.asked.borrow().is_empty());
    }

    #[test]
    fn a_ship_behav_0_miner_still_hunts_the_player() {
        let (ai, _, [_, _, miners]) = marked();
        let npcs = [mission_ship(mining(of(1, AiType::Warship), MINER_BITS), 0)];
        let around = with_player(&npcs, Condition::Intact);
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Attack(ShipRef::Player)
        );
        assert_eq!(ai.target(&npcs[0], &around), Some(ShipRef::Player));
        assert!(miners.asked.borrow().is_empty());
    }

    #[test]
    fn by_default_nova_ai_sends_a_leaderless_miner_to_the_miners_ai() {
        let npcs = [mining(of(1, AiType::Warship), SCOOPS)];
        let sites = [nova_sim_catalog::fixture::planet(128, 100.0, 0.0)];
        let around = Surroundings::new(&sites, &npcs);
        assert_eq!(
            NovaAi::default().decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Land(crate::catalog::StellarId(128)),
            "drawing nothing"
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
            .with_fleet_escorts(fighters.clone());
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
    fn nova_ai_docks_an_npc_fighter_with_its_idle_carrier_by_either_reading() {
        use crate::ai::fixture::{PIRATES, player, ship};
        let mut carrier = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        carrier.goal = Goal::Idle;
        let mut fighter = carried(
            ship(2, PIRATES, AiType::Interceptor, 50.0, 0.0),
            ShipRef::Npc(NpcId(1)),
        );
        fighter.leader = Some(NpcId(1));
        // The Return (3) its idle carrier issues it.
        fighter.order_word = crate::fleet::OrderWord(3);
        let npcs = [carrier, fighter];
        let around = Surroundings {
            player: Some(player(0.0, 300.0)),
            ..Surroundings::new(&[], &npcs)
        };
        let bible = Rulebook::default().with_override(RuleKey::NpcEscortAi, RuleSource::Bible);
        for ai in [NovaAi::default(), NovaAi::from_rulebook(&bible)] {
            assert_eq!(
                ai.decide(&npcs[1], &around, &mut Draws::of(&[])),
                Goal::Dock(ShipRef::Npc(NpcId(1)))
            );
        }
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
            assert_eq!(ai.matches(&expected).count(), 8, "all eight: {ai}");
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
            damage: 0.0,
            ..crate::combat::fixture::strike(ShipRef::Npc(NpcId(1)), ShipRef::Player)
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
                5,
                "all four types and the pirate: {ai:?}"
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
    fn every_think_ends_with_the_high_level_handlers_call_for_reinforcements() {
        let mark = Marked::new(2);
        let ai = NovaAi::default().with(AiType::Warship, mark.clone());
        let npcs = [of(1, AiType::Warship), of(2, AiType::Warship)];
        let around = with_player(&npcs, Condition::Intact);
        assert!(
            ai.decision(&npcs[0], &around, &mut Draws::of(&[]))
                .calls_reinforcements,
            "attacking (@0x8e70d)"
        );
        let mut disabled = npcs.clone();
        disabled[0].condition = Condition::Disabled;
        let around = with_player(&disabled, Condition::Intact);
        assert!(
            !ai.decision(&disabled[0], &around, &mut Draws::of(&[]))
                .calls_reinforcements,
            "disabled (@0x8e6eb)"
        );
        let escorts = NovaAi::default().with_escorts(mark.clone());
        let mut escort = of(1, AiType::WimpyTrader);
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        let led = [escort, of(2, AiType::Warship)];
        let around = with_player(&led, Condition::Intact);
        assert!(
            escorts
                .decision(&led[0], &around, &mut Draws::of(&[]))
                .calls_reinforcements,
            "an escort's think too (@0x901e5)"
        );
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
            damage: 1.0,
            ..crate::combat::fixture::strike(ShipRef::Npc(NpcId(1)), ShipRef::Player)
        };
        assert_eq!(ai.react(&npcs[0], &strike, &around).provoked, 7.0);
        assert_eq!(
            *mark.asked.borrow(),
            [NpcId(1); 2],
            "trigger and react only"
        );
    }

    #[test]
    fn a_captured_ship_behav_0_mission_escort_still_hunts_the_player() {
        // A captured mission ship keeps its mark and flies as a mission
        // escort; `_HighLevelAIHandler` runs after `_EscortAI` (@0x901e5)
        // and attacks the player for a `ShipBehav` 0 slot in formation
        // (@0x8d628-0x8d6ea), with no test on the leader (@0x89c3e).
        let mark = Marked::new(7);
        let ai = NovaAi::default().with_escorts(mark.clone());
        let mut captured = mission_ship(of(1, AiType::Warship), 0);
        if let Some(mission) = &mut captured.mission {
            mission.formation = Some(crate::escort::FormationSlot::default());
        }
        captured.goal = Goal::Formation { guard: None };
        assert!(captured.mission_escort());
        let npcs = [captured];
        let around = with_player(&npcs, Condition::Intact);
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Attack(ShipRef::Player)
        );
        assert_eq!(ai.target(&npcs[0], &around), Some(ShipRef::Player));
        let mut unmarked = npcs[0].clone();
        unmarked.mission = None;
        unmarked.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        let npcs = [unmarked];
        let around = with_player(&npcs, Condition::Intact);
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Attack(mark.ship()),
            "a hired escort thinks as the escorts do"
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
            damage: 1.0,
            ..crate::combat::fixture::strike(ShipRef::Npc(NpcId(1)), ShipRef::Player)
        };
        assert_eq!(ai.react(&npcs[0], &strike, &around).provoked, 9.0);
        assert!(mark.asked.borrow().is_empty(), "not its type's");
    }

    #[test]
    fn a_ship_behav_0_ship_without_destroying_weapons_gives_up_on_a_disabled_player_and_leaves() {
        let mark = Marked::new(7);
        let ai = NovaAi::default().with(AiType::Warship, mark.clone());
        let npcs = [mission_ship(of(1, AiType::Warship), 0)];
        let disabled = with_player(&npcs, Condition::Disabled);
        let gave_up = ai.decision(&npcs[0], &disabled, &mut Draws::of(&[]));
        assert_eq!(gave_up.goal, Goal::JumpOut, "state 2 (@0x8d6be)");
        assert!(gave_up.clears_target, "+0x70 = -1 (@0x8d6db)");
        assert!(!gave_up.turns_on_player);
        assert_eq!(ai.target(&npcs[0], &disabled), None);
        assert_eq!(
            *mark.asked.borrow(),
            [NpcId(1)],
            "its type thought first (`_AIDispatch` @0x8fffa, then @0x901e5)"
        );
        let dying = with_player(
            &npcs,
            Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
        );
        assert_eq!(
            ai.decide(&npcs[0], &dying, &mut Draws::of(&[])),
            Goal::JumpOut,
            "a breaking-up player is disabled to _IsDisabled"
        );
        assert_eq!(ai.target(&npcs[0], &dying), None);
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
    fn a_giving_up_hunter_leaves_with_no_fuel_test_and_somewhere_to_land() {
        let mut hunter = mission_ship(of(1, AiType::Warship), 0);
        hunter.person = Some(crate::traffic::npc::NpcPerson {
            zero_fuel: true,
            ..crate::traffic::npc::NpcPerson::of(
                &nova_sim_catalog::fixture::person(151, 128),
                false,
            )
        });
        hunter.reserves.fuel.now = 0.0;
        hunter.goal = Goal::JumpOut;
        assert!(!crate::ai::leave::can_jump(&hunter, RuleSource::Engine));
        let sites = [nova_sim_catalog::fixture::planet(128, 300.0, 0.0)];
        let npcs = [hunter];
        let mut player = crate::ai::fixture::player(0.0, 0.0);
        player.condition = Condition::Disabled;
        let around = Surroundings {
            player: Some(player),
            ..Surroundings::new(&sites, &npcs)
        };
        let ai = NovaAi::default();
        assert_eq!(
            Warship::default().decide(&npcs[0], &around, &mut Draws::of(&[0])),
            Goal::Land(crate::catalog::StellarId(128)),
            "its type's think alone would land"
        );
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[0])),
            Goal::JumpOut,
            "the high level's give-up comes after it"
        );
        let intact = with_player(&npcs, Condition::Intact);
        assert_eq!(
            ai.decide(&npcs[0], &intact, &mut Draws::of(&[0])),
            Goal::Attack(ShipRef::Player),
            "the player intact again, it attacks again"
        );
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
    fn nova_ai_reads_led_ally_attack_from_the_rulebook() {
        use crate::ai::fixture::{POLICE, TRADERS, around, govts, ship, sites};
        use crate::escort::{EscortDuty, FormationSlot};
        let govts = govts(0, 0);
        let sites = sites();
        let mut escort = ship(2, POLICE, AiType::Warship, 0.0, 100.0);
        escort.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        let spare = Rulebook::default().with_override(RuleKey::LedAllyAttack, RuleSource::Bible);
        for ai_type in [AiType::Warship, AiType::Interceptor] {
            let mut hunter = ship(1, TRADERS, ai_type, 0.0, 0.0);
            hunter.goal = Goal::Attack(ShipRef::Npc(NpcId(2)));
            let npcs = [hunter, escort.clone()];
            let seen = around(&sites, &npcs, &govts, (0.0, -3000.0), 0);
            let decide = |rulebook: &Rulebook| {
                NovaAi::from_rulebook(rulebook).decide(&npcs[0], &seen, &mut Draws::of(&[0; 4]))
            };
            assert_eq!(
                decide(&Rulebook::default()),
                Goal::Attack(ShipRef::Npc(NpcId(2))),
                "{ai_type:?}: the engine attacks on an ally the player leads"
            );
            assert_ne!(
                decide(&spare),
                Goal::Attack(ShipRef::Npc(NpcId(2))),
                "{ai_type:?}: the other reading spares it"
            );
        }
        // The flee drop reads it too.
        let mut fleeing = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        fleeing.goal = Goal::Flee(ShipRef::Npc(NpcId(2)));
        let npcs = [fleeing, escort];
        let seen = around(&sites, &npcs, &govts, (0.0, -3000.0), 0);
        let flee = |rulebook: &Rulebook| {
            let ai = NovaAi::from_rulebook(rulebook).with(AiType::WimpyTrader, Rc::new(Fleeing));
            ai.decision(&npcs[0], &seen, &mut Draws::of(&[0; 4]))
        };
        assert_eq!(
            flee(&Rulebook::default()).goal,
            Goal::Flee(ShipRef::Npc(NpcId(2)))
        );
        let given_up = flee(&spare);
        assert_eq!(given_up.goal, Goal::Idle);
        assert!(!given_up.calls_reinforcements, "and calls no fleet");
    }

    /// Flees NPC 2 in each decision.
    #[derive(Debug)]
    struct Fleeing;

    impl Behaviour for Fleeing {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Flee(ShipRef::Npc(NpcId(2)))
        }
    }

    #[test]
    fn nova_ai_gives_up_fleeing_a_stranger_ally_within_250() {
        use crate::ai::fixture::{POLICE, TRADERS, around, govts, ship, sites};
        let govts = govts(0, 0);
        let sites = sites();
        let npcs = [
            ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0),
            ship(2, POLICE, AiType::Warship, 0.0, 200.0),
        ];
        let seen = around(&sites, &npcs, &govts, (0.0, -3000.0), 0);
        let ai = NovaAi::default().with(AiType::WimpyTrader, Rc::new(Fleeing));
        let decision = ai.decision(&npcs[0], &seen, &mut Draws::of(&[0; 4]));
        assert_eq!(decision.goal, Goal::Idle);
        assert!(!decision.calls_reinforcements);
    }

    #[test]
    fn nova_ai_reads_friend_shield_carry_from_the_rulebook() {
        use crate::ai::fixture::{PIRATES, POLICE, TRADERS, around, govts, ship, sites};
        use crate::reserves::Gauge;
        // The hunter's own 0.5 of 100, a full-shielded enemy before its
        // shieldless ally of 40, and a pirate of 75 to take on: by the
        // engine the ally carries the enemy's factor 1 (50 + 40), by the
        // other reading its raw 0.25 (50 + 10).
        let govts = govts(0, 0);
        let sites = sites();
        let mut hunter = ship(1, TRADERS, AiType::Warship, 0.0, 0.0);
        hunter.reserves.shield.now = 15.0;
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.hull.strength = 75.0;
        let mut ally = ship(3, POLICE, AiType::WimpyTrader, 0.0, 600.0);
        ally.hull.strength = 40.0;
        ally.reserves.shield = Gauge { now: 0.0, max: 0.0 };
        let npcs = [hunter, pirate, ally];
        let seen = around(&sites, &npcs, &govts, (0.0, -9000.0), 0);
        let decide = |rulebook: &Rulebook| {
            NovaAi::from_rulebook(rulebook).decide(&npcs[0], &seen, &mut Draws::of(&[0; 4]))
        };
        assert_eq!(
            decide(&Rulebook::default()),
            Goal::Attack(ShipRef::Npc(NpcId(2)))
        );
        let raw = Rulebook::default().with_override(RuleKey::FriendShieldCarry, RuleSource::Bible);
        assert_ne!(decide(&raw), Goal::Attack(ShipRef::Npc(NpcId(2))));
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
                forgets_provocation: false,
                comm_call: None,
                calls_reinforcements: false,
                board_wait: None,
                clears_target: false,
                keeps_pursuit: false,
                plunders: None,
                lost_sight: None,
                travel: crate::ai::travel::TravelWrite::Keep,
                visits: None,
                re_picked: false,
                ends_wait: false,
                order_word: None,
                leaves_fleet: false,
                follows_jump: false,
                voice: None,
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
                forgets_provocation: false,
                comm_call: None,
                calls_reinforcements: true,
                board_wait: None,
                clears_target: false,
                keeps_pursuit: false,
                plunders: None,
                lost_sight: Some(crate::ai::lost_sight::LostSight::Seen),
                travel: crate::ai::travel::TravelWrite::Keep,
                visits: None,
                re_picked: false,
                ends_wait: false,
                order_word: None,
                leaves_fleet: false,
                follows_jump: false,
                voice: None,
            },
            "the mission hunt (@0x8d6b7), the player seen (@0x8d8ae)"
        );
        let gives_up = NovaAi::default().with(AiType::Warship, Rc::new(Turning));
        let disabled = with_player(&hunters, Condition::Disabled);
        let gave_up = gives_up.decision(&hunters[0], &disabled, &mut Draws::of(&[]));
        assert_eq!(
            (
                gave_up.goal,
                gave_up.turns_on_player,
                gave_up.dropped_target
            ),
            (Goal::JumpOut, false, true),
            "giving up, it turns on nobody; the rest is its type's"
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

    /// `npc` fully cloaked.
    fn cloaked(mut npc: Npc) -> Npc {
        npc.cloak = crate::cloak::Cloak {
            fade: crate::cloak::FADE_FULL,
            direction: 0,
        };
        npc
    }

    #[test]
    fn a_searching_ship_makes_no_high_level_reinforcement_call() {
        use crate::ai::lost_sight::LostSight;
        let mark = Marked::new(2);
        let ai = NovaAi::default().with(AiType::Warship, mark.clone());
        let npcs = [of(1, AiType::Warship), cloaked(of(2, AiType::Warship))];
        let around = with_player(&npcs, Condition::Intact);
        let searching = ai.decision(&npcs[0], &around, &mut Draws::of(&[0]));
        assert_eq!(searching.goal, Goal::Attack(mark.ship()));
        assert_eq!(searching.lost_sight, Some(LostSight::Searching(100)));
        assert!(!searching.calls_reinforcements, "the jump to @0x8fb47");
        let mut gave_up = npcs[0].clone();
        gave_up.lost_sight = crate::ai::lost_sight::SearchTimer::Searching(1);
        let dropped = ai.decision(&gave_up, &around, &mut Draws::of(&[]));
        assert_eq!(dropped.goal, Goal::Idle);
        assert_eq!(dropped.lost_sight, Some(LostSight::GaveUp));
        assert!(!dropped.calls_reinforcements);
        let seen = [of(1, AiType::Warship), of(2, AiType::Warship)];
        let around = with_player(&seen, Condition::Intact);
        let fight = ai.decision(&seen[0], &around, &mut Draws::of(&[]));
        assert_eq!(fight.lost_sight, Some(LostSight::Seen));
        assert!(fight.calls_reinforcements, "seen, @0x8e6eb");
    }

    #[test]
    fn the_lost_sight_block_runs_for_every_type_after_its_think() {
        use crate::ai::lost_sight::LostSight;
        let mark = Marked::new(3);
        let escorts = NovaAi::default().with_escorts(mark.clone());
        let carriers = NovaAi::default().with_fleet_escorts(mark.clone());
        let bible = Rulebook::default().with_override(RuleKey::NpcEscortAi, RuleSource::Bible);
        let earlier = NovaAi::from_rulebook(&bible).with_carried(mark.clone());
        let mut escort = of(1, AiType::WimpyTrader);
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        let fighter = carried(of(1, AiType::WimpyTrader), ShipRef::Npc(NpcId(2)));
        for (ai, npc) in [
            (&escorts, escort),
            (&carriers, fighter.clone()),
            (&earlier, fighter),
        ] {
            let npcs = [npc, of(2, AiType::Warship), cloaked(of(3, AiType::Warship))];
            let around = with_player(&npcs, Condition::Intact);
            let decision = ai.decision(&npcs[0], &around, &mut Draws::of(&[0]));
            assert_eq!(decision.goal, Goal::Attack(mark.ship()));
            assert_eq!(decision.lost_sight, Some(LostSight::Searching(100)));
        }
    }
}
