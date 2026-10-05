//! [`EscortAi`]: how the player's escorts fight, by the standing order
//! each follows (`_EscortAI` @0x838d2 in the `EV Nova` executable). The
//! values are in [`escort`](crate::escort).
//!
//! - **Hold Position**: it brakes to a stop where it is ([`Goal::Idle`]),
//!   targets nothing and fires nothing; its point defence still works.
//! - **Defend**: it keeps its target while the target is intact and
//!   within [`DEFEND_KEEP_SQ`] (distance squared) of the player;
//!   otherwise it takes the best threat to the player within
//!   [`DEFEND_RADIUS`] of the player ([`best_threat`]), and attacks it
//!   with every weapon. With none, it keeps formation.
//! - **Attack**: it keeps its target while it is in the system, even
//!   disabled; with none, it takes the best threat to the player at any
//!   distance, and attacks it with every weapon. With none, it keeps
//!   formation.
//! - **No standing command** ("Formation"), as the rulebook's
//!   [`RuleKey::EscortAi`](crate::RuleKey::EscortAi) entry says:
//!   - by the engine, it keeps formation ([`Goal::Formation`]) and fires
//!     its turrets alone at a guard: the one it guards still while that
//!     ship is intact and some weapon it carries reaches it, else one
//!     drawn at random among the ships threatening the player
//!     (`_AIFightThreatToParent` @0x81de6), at any distance;
//!   - by the Bible (`shïp` `InherentAI`: "What AI the ship uses when
//!     it's escorting the player"), it flies as its AI type would: a
//!     warship or interceptor attacks the best threat to the player at
//!     any distance with every weapon, keeping formation while there is
//!     none, and a trader keeps formation and fires nothing.
//!
//! An escort answers threats to the player, not the hits it takes: it has
//! no reaction.

use crate::ai::{Behaviour, Goal, Surroundings, fire};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::escort::{DEFEND_KEEP_SQ, DEFEND_RADIUS, EscortOrder, best_threat, threats};
use crate::flight::ShipState;
use crate::rulebook::RuleSource;
use crate::traffic::npc::{Mode, Npc, NpcId};

/// The player's escorts' AI (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EscortAi {
    /// What an escort with no standing command does.
    pub escort_ai: RuleSource,
}

impl EscortAi {
    /// What `npc`, with no standing command, does among `around`, the
    /// player at `player`, drawing any guard on `chance`.
    fn no_command(
        self,
        npc: &Npc,
        around: &Surroundings,
        player: &ShipState,
        chance: &mut dyn Chance,
    ) -> Goal {
        match self.escort_ai {
            RuleSource::Engine => Goal::Formation {
                guard: guard(npc, around, chance),
            },
            RuleSource::Bible if npc.ai_type.trades() => Goal::Formation { guard: None },
            RuleSource::Bible => attack_or_form(best_threat(npc, around.npcs, player, None)),
        }
    }
}

impl Behaviour for EscortAi {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        let order = npc.escort.and_then(|duty| duty.order);
        if order == Some(EscortOrder::Hold) {
            return Goal::Idle;
        }
        let Some(player) = around.player.map(|player| player.state) else {
            return Goal::Formation { guard: None };
        };
        let kept = npc.target.filter(|&target| target != ShipRef::Player);
        let npc_of = |ship: ShipRef| match ship {
            ShipRef::Npc(id) => around.npc(id),
            ShipRef::Player => None,
        };
        match order {
            Some(EscortOrder::Attack) => {
                let kept = kept.filter(|&target| around.live(target));
                kept.map_or_else(
                    || attack_or_form(best_threat(npc, around.npcs, &player, None)),
                    Goal::Attack,
                )
            }
            Some(EscortOrder::Defend) => {
                let near = |target: &Npc| {
                    let off = target.state.position - player.position;
                    off.x * off.x + off.y * off.y <= DEFEND_KEEP_SQ
                };
                let kept = kept
                    .and_then(npc_of)
                    .filter(|target| intact(target) && near(target));
                kept.map_or_else(
                    || attack_or_form(best_threat(npc, around.npcs, &player, Some(DEFEND_RADIUS))),
                    |target| Goal::Attack(ShipRef::Npc(target.id)),
                )
            }
            _ => self.no_command(npc, around, &player, chance),
        }
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        match npc.goal {
            Goal::Attack(_) => fire::trigger(npc, around),
            Goal::Formation { guard: Some(_) } => Trigger {
                primary: true,
                turrets_only: true,
                ..Trigger::default()
            },
            _ => Trigger::default(),
        }
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        match npc.goal {
            Goal::Attack(target) => Some(target),
            Goal::Formation { guard } => guard,
            _ => None,
        }
    }
}

/// Whether `npc` is intact and flying in the system: one an escort may
/// keep fighting.
fn intact(npc: &Npc) -> bool {
    npc.condition == Condition::Intact && npc.mode == Mode::Flying
}

/// An attack on `threat`, or formation with none.
fn attack_or_form(threat: Option<NpcId>) -> Goal {
    threat.map_or(Goal::Formation { guard: None }, |id| {
        Goal::Attack(ShipRef::Npc(id))
    })
}

/// The ship `npc`, keeping formation, guards against among `around`:
/// its target still, while that ship is intact and some weapon `npc`
/// carries reaches it; otherwise one drawn on `chance` among the ships
/// threatening the player ([`threats`]); none when there is none.
fn guard(npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Option<ShipRef> {
    let kept = match npc.target {
        Some(ShipRef::Npc(id)) => around
            .npc(id)
            .filter(|target| intact(target) && fire::in_reach(npc, &target.state)),
        _ => None,
    };
    if let Some(target) = kept {
        return Some(ShipRef::Npc(target.id));
    }
    let threats: Vec<NpcId> = threats(npc, around.npcs).map(|threat| threat.id).collect();
    if threats.is_empty() {
        return None;
    }
    let count = u32::try_from(threats.len()).unwrap_or(u32::MAX);
    let pick = chance.below(count) as usize;
    threats.get(pick).copied().map(ShipRef::Npc)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, govts, n, player, ship};
    use crate::catalog::WeaponId;
    use crate::combat::armament::Armament;
    use crate::combat::weapon::WeaponSpec;
    use crate::escort::{EscortClass, EscortDuty};
    use crate::testkit::{Draws, weapon};
    use crate::traffic::npc::AiType;

    /// A turret reaching 300 pixels (332 with the margin), and a gun.
    fn armed() -> Armament {
        Armament::new([
            (
                WeaponSpec::new(&crate::catalog::WeaponRecord {
                    guidance: 4,
                    speed: 1000,
                    count: 30,
                    energy_dmg: 5,
                    ..weapon(140)
                }),
                1,
            ),
            (
                WeaponSpec::new(&crate::catalog::WeaponRecord {
                    speed: 1000,
                    count: 30,
                    energy_dmg: 5,
                    ..weapon(141)
                }),
                1,
            ),
        ])
    }

    /// The escort, NPC 1, a medium warship at the centre following
    /// `order`, armed with [`armed`].
    fn escort(order: Option<EscortOrder>) -> Npc {
        let mut npc = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        npc.govt = None;
        npc.class = EscortClass::Medium;
        npc.armament = armed();
        npc.escort = Some(EscortDuty {
            slot: 2,
            ships: 2,
            spacing: 30.0,
            order,
        });
        npc
    }

    /// NPC `id` at (`x`, `y`) attacking the player.
    fn pirate(id: u32, x: f32, y: f32) -> Npc {
        let mut npc = ship(id, PIRATES, AiType::Warship, x, y);
        npc.class = EscortClass::Medium;
        npc.goal = Goal::Attack(ShipRef::Player);
        npc
    }

    /// NPC `id` at (`x`, `y`), idle.
    fn bystander(id: u32, x: f32, y: f32) -> Npc {
        let mut npc = ship(id, PIRATES, AiType::Warship, x, y);
        npc.class = EscortClass::Medium;
        npc
    }

    /// What `ai` decides for `npcs[0]` among `npcs`, the player at
    /// (`x`, `y`), drawing on `draws`; and the draws asked for.
    fn decide_with(
        ai: EscortAi,
        npcs: &[Npc],
        (x, y): (f32, f32),
        draws: &[u32],
    ) -> (Goal, Vec<u32>) {
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(x, y)),
            govts: &govts,
            ..Surroundings::new(&[], npcs)
        };
        let mut chance = Draws::of(draws);
        let goal = ai.decide(&npcs[0], &around, &mut chance);
        (goal, chance.asked)
    }

    const ENGINE: EscortAi = EscortAi {
        escort_ai: RuleSource::Engine,
    };
    const BIBLE: EscortAi = EscortAi {
        escort_ai: RuleSource::Bible,
    };

    fn decide(npcs: &[Npc], player_at: (f32, f32), draws: &[u32]) -> (Goal, Vec<u32>) {
        decide_with(ENGINE, npcs, player_at, draws)
    }

    /// `npc` having decided `goal`, and targeting what it decided.
    fn having(mut npc: Npc, goal: Goal, target: Option<ShipRef>) -> Npc {
        npc.goal = goal;
        npc.target = target;
        npc
    }

    /// What `ai` holds for `npcs[0]` once it decided: its trigger and
    /// target.
    fn command(ai: EscortAi, npcs: &[Npc]) -> (Trigger, Option<ShipRef>) {
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(0.0, 0.0)),
            govts: &govts,
            ..Surroundings::new(&[], npcs)
        };
        (ai.trigger(&npcs[0], &around), ai.target(&npcs[0], &around))
    }

    const TURRETS: Trigger = Trigger {
        primary: true,
        secondary: None,
        only: None,
        turrets_only: true,
    };

    #[test]
    fn with_no_command_and_no_threat_it_keeps_formation_and_fires_nothing() {
        let npcs = [escort(None), bystander(2, 100.0, 0.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]),
            (Goal::Formation { guard: None }, vec![])
        );
        let formed = having(escort(None), Goal::Formation { guard: None }, None);
        assert_eq!(command(ENGINE, &[formed]), (Trigger::default(), None));
    }

    #[test]
    fn with_no_command_it_guards_against_a_threat_drawn_at_random_at_any_distance() {
        let npcs = [
            escort(None),
            pirate(2, 100.0, 0.0),
            bystander(3, 50.0, 0.0),
            pirate(4, 5000.0, 0.0),
        ];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[0]),
            (Goal::Formation { guard: Some(n(2)) }, vec![2])
        );
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[1]),
            (Goal::Formation { guard: Some(n(4)) }, vec![2])
        );
    }

    #[test]
    fn a_guard_within_some_weapons_reach_is_kept_and_one_beyond_it_replaced() {
        // The turret and gun reach 332.
        let kept = having(
            escort(None),
            Goal::Formation { guard: Some(n(2)) },
            Some(n(2)),
        );
        let npcs = [kept.clone(), pirate(2, 332.0, 0.0), pirate(3, 10.0, 0.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]),
            (Goal::Formation { guard: Some(n(2)) }, vec![]),
            "no draw"
        );
        let npcs = [kept.clone(), pirate(2, 0.0, 333.0), pirate(3, 10.0, 0.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[1]),
            (Goal::Formation { guard: Some(n(3)) }, vec![2]),
            "drawn among both"
        );
        let mut unarmed = kept;
        unarmed.armament = Armament::default();
        let npcs = [unarmed, pirate(2, 10.0, 0.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[0]).1,
            vec![1],
            "nothing reaches"
        );
    }

    #[test]
    fn with_no_command_it_fires_its_turrets_alone_at_its_guard_and_never_leaves_formation() {
        let guarding = having(
            escort(None),
            Goal::Formation { guard: Some(n(2)) },
            Some(n(2)),
        );
        assert_eq!(
            command(ENGINE, &[guarding, pirate(2, 100.0, 0.0)]),
            (TURRETS, Some(n(2)))
        );
        for at in [(10.0, 0.0), (300.0, 300.0), (4000.0, 0.0)] {
            let npcs = [escort(None), pirate(2, at.0, at.1)];
            assert!(
                matches!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Formation { .. }),
                "{at:?}"
            );
        }
    }

    #[test]
    fn a_disabled_target_is_dropped_under_defend_and_with_no_command() {
        for order in [None, Some(EscortOrder::Defend)] {
            let mut down = pirate(2, 100.0, 0.0);
            down.condition = Condition::Disabled;
            let npcs = [having(escort(order), Goal::Attack(n(2)), Some(n(2))), down];
            assert_eq!(
                decide(&npcs, (0.0, 0.0), &[]).0,
                Goal::Formation { guard: None },
                "{order:?}"
            );
        }
    }

    #[test]
    fn defending_it_keeps_its_target_within_about_639_pixels_of_the_player() {
        let defend = Some(EscortOrder::Defend);
        let fighting = having(escort(defend), Goal::Attack(n(2)), Some(n(2)));
        // 408375 is between 639² (408321) and 640² (409600).
        let npcs = [fighting.clone(), bystander(2, 0.0, 639.0)];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Attack(n(2)), "kept");
        let npcs = [fighting.clone(), bystander(2, 0.0, 640.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]).0,
            Goal::Formation { guard: None },
            "dropped"
        );
        let npcs = [fighting.clone(), bystander(2, 1000.0, 639.0)];
        assert_eq!(
            decide(&npcs, (1000.0, 0.0), &[]).0,
            Goal::Attack(n(2)),
            "measured from the player"
        );
        // 451² x 2 = 406802 is within, and 452² x 2 = 408608 is not.
        let npcs = [fighting.clone(), bystander(2, 451.0, 451.0)];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Attack(n(2)));
        let npcs = [fighting, bystander(2, 452.0, -452.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]).0,
            Goal::Formation { guard: None }
        );
    }

    #[test]
    fn defending_it_takes_a_threat_within_550_pixels_of_the_player() {
        let defend = Some(EscortOrder::Defend);
        let npcs = [escort(defend), pirate(2, 0.0, 550.0)];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Attack(n(2)));
        let npcs = [escort(defend), pirate(2, 0.0, 551.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]).0,
            Goal::Formation { guard: None }
        );
        let npcs = [escort(defend), pirate(2, 0.0, 551.0), pirate(3, 400.0, 0.0)];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Attack(n(3)));
        let npcs = [escort(defend)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]).0,
            Goal::Formation { guard: None }
        );
    }

    #[test]
    fn attacking_it_keeps_its_target_disabled_or_not_else_the_best_threat_anywhere() {
        let attack = Some(EscortOrder::Attack);
        let mut down = bystander(2, 5000.0, 0.0);
        down.condition = Condition::Disabled;
        let npcs = [
            having(escort(attack), Goal::Formation { guard: None }, Some(n(2))),
            down,
            pirate(3, 10.0, 0.0),
        ];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Attack(n(2)), "kept");
        let npcs = [
            escort(attack),
            pirate(2, 5000.0, 0.0),
            pirate(3, 3000.0, 0.0),
        ];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]).0,
            Goal::Attack(n(3)),
            "the nearer"
        );
        let gone = having(escort(attack), Goal::Attack(n(9)), Some(n(9)));
        assert_eq!(
            decide(&[gone, bystander(2, 10.0, 0.0)], (0.0, 0.0), &[]).0,
            Goal::Formation { guard: None },
            "its target gone, and no threat"
        );
    }

    #[test]
    fn attacking_or_defending_it_fires_as_any_fighter_does() {
        for order in [EscortOrder::Attack, EscortOrder::Defend] {
            let fighting = having(escort(Some(order)), Goal::Attack(n(2)), Some(n(2)));
            let npcs = [fighting, pirate(2, 0.0, -100.0)];
            let govts = govts(0, 0);
            let around = Surroundings {
                player: Some(player(0.0, 0.0)),
                govts: &govts,
                ..Surroundings::new(&[], &npcs)
            };
            assert_eq!(
                command(ENGINE, &npcs),
                (fire::trigger(&npcs[0], &around), Some(n(2))),
                "{order:?}"
            );
            assert_eq!(
                fire::trigger(&npcs[0], &around).only,
                Some(WeaponId(141)),
                "the gun ahead"
            );
        }
    }

    #[test]
    fn holding_it_idles_targets_nothing_and_fires_nothing() {
        let hold = Some(EscortOrder::Hold);
        let npcs = [
            having(escort(hold), Goal::Attack(n(2)), Some(n(2))),
            pirate(2, 10.0, 0.0),
        ];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]), (Goal::Idle, vec![]));
        let holding = having(escort(hold), Goal::Idle, Some(n(2)));
        assert_eq!(
            command(ENGINE, &[holding, pirate(2, 10.0, 0.0)]),
            (Trigger::default(), None)
        );
    }

    #[test]
    fn by_the_bible_a_warship_with_no_command_attacks_the_best_threat() {
        for ai_type in [AiType::Warship, AiType::Interceptor] {
            let mut warship = escort(None);
            warship.ai_type = ai_type;
            let npcs = [
                warship.clone(),
                pirate(2, 5000.0, 0.0),
                pirate(3, 3000.0, 0.0),
            ];
            assert_eq!(
                decide_with(BIBLE, &npcs, (0.0, 0.0), &[]),
                (Goal::Attack(n(3)), vec![]),
                "{ai_type:?}"
            );
            assert_eq!(
                decide_with(BIBLE, &[warship.clone()], (0.0, 0.0), &[]).0,
                Goal::Formation { guard: None }
            );
            let fighting = having(warship, Goal::Attack(n(3)), Some(n(3)));
            let npcs = [fighting, pirate(3, 0.0, -100.0)];
            assert_eq!(command(BIBLE, &npcs).1, Some(n(3)));
            assert_ne!(command(BIBLE, &npcs).0, TURRETS, "every weapon");
        }
    }

    #[test]
    fn by_the_bible_a_trader_with_no_command_keeps_formation_and_fires_nothing() {
        for ai_type in [AiType::WimpyTrader, AiType::BraveTrader] {
            let mut trader = escort(None);
            trader.ai_type = ai_type;
            let npcs = [trader, pirate(2, 100.0, 0.0)];
            assert_eq!(
                decide_with(BIBLE, &npcs, (0.0, 0.0), &[]),
                (Goal::Formation { guard: None }, vec![]),
                "{ai_type:?}"
            );
        }
    }

    #[test]
    fn by_the_bible_explicit_orders_are_unchanged() {
        for order in [EscortOrder::Attack, EscortOrder::Defend, EscortOrder::Hold] {
            for ai_type in [AiType::WimpyTrader, AiType::Warship] {
                let mut npc = escort(Some(order));
                npc.ai_type = ai_type;
                let npcs = [npc, pirate(2, 100.0, 0.0), pirate(3, 3000.0, 0.0)];
                assert_eq!(
                    decide_with(BIBLE, &npcs, (0.0, 0.0), &[]),
                    decide_with(ENGINE, &npcs, (0.0, 0.0), &[]),
                    "{order:?} {ai_type:?}"
                );
            }
        }
    }

    #[test]
    fn an_escort_answers_no_hit() {
        let npcs = [escort(None), pirate(2, 100.0, 0.0)];
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(0.0, 0.0)),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let strike = crate::combat::Strike {
            ship: n(1),
            by: n(2),
            damage: 10.0,
            downed: None,
        };
        assert_eq!(
            ENGINE.react(&npcs[0], &strike, &around),
            crate::ai::Reaction::default()
        );
    }
}
