//! Nova's pirate warship: a leaderless warship of a government with
//! `Flags` 0x1000 ([`PLUNDERS`]), which `_AIDispatch` sends to
//! `_PirateWarshipAI` (@0x8ffc8-0x8fff0, @0x8c2d2-0x8c894 in the `EV Nova`
//! executable) in place of the warship's think ([`PirateWarship`],
//! [`think`]). It disables a trader, a former escort of the player's or the
//! player to board it (goal 0xd, [`Goal::Plunder`]), attacks anything else
//! it takes on (goal 4), and waits while another ship boards the ship it
//! would plunder (goal 0x16, [`Goal::CountWait`]).
//!
//! Each think, in order:
//!
//! - **Disabled, or waiting** (@0x8c2e1, @0x8c2ee): a pirate in its count
//!   wait keeps it, whatever became of the ship it waited on; the traffic
//!   counts it down and ends it ([`board_wait`](crate::traffic::board_wait)).
//! - **The head drop** (@0x8c2fc-0x8c330): a target that has left the
//!   system or is breaking up is dropped, with the goal (target -1, goal
//!   0), and the search below runs in the same think, so a breaking-up
//!   ship that is still the nearest to plunder is taken again.
//! - **The search** (@0x8c339-0x8c385), with no goal ([`Goal::Idle`]) or
//!   no target: its target cleared (@0x8c35d), first the ship to plunder
//!   ([`plunder_target::find`]), then the warship's target
//!   ([`pick_target`] with none kept, `_SelectWarshipTarget`), then its
//!   business ([`idle`]), its target left cleared
//!   ([`Decision::clears_target`]). A pirate holding ([`Goal::Hold`])
//!   with no target so leaves the hold. A pirate with any other goal and a
//!   target set keeps both and thinks no more (@0x8c34d → @0x8c5e8): one
//!   bound to land that an NPC's hit gave a target lands on, holding the
//!   target, and one holding holds. Its target is the reaction's
//!   (`_DamageShip` @0x3be5d), which [`PirateWarship::target`] keeps.
//! - **The classification** (@0x8c5e1-0x8c743), of the ship it found or
//!   the target an attack or plunder holds (the reaction's write included):
//!   the player, a former escort (+0xc8de, [`Npc::former`]) or a ship whose
//!   class's `InherentAI` is 2 or below, whose class's `Crew` is above none
//!   ([`HullSpec::crew`](crate::combat::hull::HullSpec::crew)), is plundered,
//!   unless it is boarded ([`Npc::boarded`]): then it waits if another ship
//!   boards it ([`Decision::board_wait`], the wait loop, which the traffic
//!   runs in slot order) and attacks it otherwise. Anything else is
//!   attacked. The first classification of a ship the search found
//!   (@0x8c38b-0x8c40b) is always overridden by this one, so it is not
//!   modelled apart: with a `Crew` of -1 the first gives 0xd and this one
//!   goal 4.
//! - **A plunder** of an intact ship snipes when the chase is hopeless,
//!   outside [`DOGFIGHT_BOX`] on either axis alone (`_HighLevelAIHandler`
//!   @0x8ea49-0x8eacb, [`hopeless_chase`]); it has no ally drop, no
//!   stand-off band, no fighter launch but the snipe's and no call for
//!   reinforcements (see [`pursuit`](super::pursuit)). A plunder of a
//!   disabled ship docks with it (@0x8e873-0x8ea44):
//!   rdm:phase/combat-ai-fidelity/phase-10-pirates-plunder.
//! - **The disabled-warship drop** (@0x8c7b4-0x8c842): attacking a disabled
//!   NPC of a class whose `InherentAI` is above 2, not a former escort,
//!   with no destroying weapon ([`fire::destroys`]) or fully dry, it drops
//!   it: goal 0 and target -1 alone, so a kept rush or break-off carries
//!   over to its next fight ([`Decision::keeps_pursuit`]).
//! - **The dry drop** (@0x8c848-0x8c887): attacking or plundering fully dry
//!   ([`AmmoLeft::Dry`]), it drops the fight, its sub-goal too. It comes
//!   after the wait loop, so a dry pirate waits all the same when another
//!   ship boards its victim.
//! - **An attack** it keeps is goal 4's, as `_HighLevelAIHandler` flies it
//!   in the same frame: it drops a target it spares (the ally drop
//!   @0x8e277-0x8e2e4, [`spares`]: goal and sub 0, the target kept), then
//!   one breaking up or gone (@0x8e3ef → @0x8e714: target -1, goal 0, the
//!   sub kept), and snipes a hopeless chase unless it stands off (as
//!   [`warship::hunt`](super::warship::hunt)'s hunted ship does).
//!
//! It takes no government's word, has no odds, shield or out-of-ammunition
//! retreat, reads no provocation, and plays no sound. A pirate that picks
//! the player through [`pick_target`]'s xenophobe, hunt or grudge turns on
//! it through `_AIMakeShipAttackPlayer` as a warship does
//! ([`Decision::turns_on_player`]); a plunder pick of the player makes no
//! such call (`_AIFindShipToPlunder` @0x7fd10-0x7feb5).
//!
//! **Left out**, each owned elsewhere (see the phase body of
//! rdm:phase/combat-ai-fidelity/phase-9-pirates-disable-and-board): the
//! idle block (@0x8c410-0x8c5d2) is
//! rdm:phase/combat-ai-fidelity/phase-12-idle-block-and-patrols's, and
//! [`idle`] stands in; the plunder and state 14 (@0x8c74c-0x8c7ae) and the
//! goal-0xe search skip (@0x8c353) are phase 10's.

use crate::ai::fire::{AmmoLeft, DOGFIGHT_BOX, FireRules, ammo_left};
use crate::ai::odds::hopeless_chase;
use crate::ai::target::{TargetRules, pick_target, spares};
use crate::ai::{
    Behaviour, Decision, Goal, Reaction, Surroundings, fire, idle, plunder_target, react,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
#[cfg(doc)]
use crate::govt::PLUNDERS;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;

/// Nova's pirate warship (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PirateWarship {
    /// Who comes to a ship's help ([`react::answer`]).
    pub piracy_police: RuleSource,
    /// The rulebook's entries its weapon pick follows.
    pub fire: FireRules,
    /// The rulebook's entries its warship's target pick follows.
    pub target: TargetRules,
}

impl Behaviour for PirateWarship {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        think(npc, around, self.target, chance)
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        fire::trigger(npc, around, self.fire)
    }

    /// The ship its fight is about; none in its count wait, which
    /// `_HighLevelAIHandler` clears every frame (@0x8d480); and for any
    /// other goal the target it holds, which its think keeps with a goal it
    /// keeps (see the module docs).
    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        match npc.goal {
            Goal::CountWait { .. } => None,
            goal => goal.quarry().or(npc.target),
        }
    }

    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        react::answer(npc, strike, around, self.piracy_police)
    }
}

/// What `npc`, a pirate, decides among `around`, its warship's target pick
/// by `rules`, rolling on `chance` (see the module docs).
#[must_use]
pub fn think(
    npc: &Npc,
    around: &Surroundings,
    rules: TargetRules,
    chance: &mut dyn Chance,
) -> Decision {
    if matches!(npc.goal, Goal::CountWait { .. }) {
        return of(npc.goal);
    }
    let dropped = npc.target.is_some_and(|ship| gone_or_dying(ship, around));
    let goal = if dropped { Goal::Idle } else { npc.goal };
    let kept = npc.target.filter(|_| !dropped && goal != Goal::Idle);
    let (target, turns_on_player) = if let Some(ship) = kept {
        if !matches!(
            goal,
            Goal::Attack(_) | Goal::Snipe(_) | Goal::Plunder { .. }
        ) {
            return of(npc.goal);
        }
        (ship, false)
    } else if let Some(found) = search(npc, around, rules, chance) {
        found
    } else {
        let idler = Npc {
            goal: if goal == Goal::Hold { Goal::Idle } else { goal },
            ..npc.clone()
        };
        return Decision {
            goal: idle(&idler, around, chance),
            clears_target: true,
            ..Decision::default()
        };
    };
    Decision {
        turns_on_player,
        ..fought(npc, target, around, rules)
    }
}

/// The search (@0x8c35d-0x8c385): the ship `npc` plunders first among
/// `around`, else the warship's pick by `rules` on `chance`, with whether
/// the pick turns it on the player; none when there is neither.
fn search(
    npc: &Npc,
    around: &Surroundings,
    rules: TargetRules,
    chance: &mut dyn Chance,
) -> Option<(ShipRef, bool)> {
    if let Some(ship) = plunder_target::find(npc, around) {
        return Some((ship, false));
    }
    pick_target(npc, None, around, rules, chance).map(|pick| (pick.ship, pick.turns_on_player))
}

/// What `npc` does about `target` among `around`, by `rules`: the
/// classification, then the plunder's or the attack's high level, the
/// disabled-warship drop and the dry drop (see the module docs).
fn fought(npc: &Npc, target: ShipRef, around: &Surroundings, rules: TargetRules) -> Decision {
    let dry = ammo_left(npc) == AmmoLeft::Dry;
    let dropped = Decision {
        clears_target: true,
        ..of(Goal::Idle)
    };
    match classify(target, around) {
        Class::Plunder if dry => dropped,
        Class::Plunder => of(Goal::Plunder {
            ship: target,
            hopeless: outside_box(npc, target, around) && hopeless_chase(npc, target, around),
        }),
        Class::Boarded => Decision {
            board_wait: Some(target),
            ..if dry {
                dropped
            } else {
                of(attacked(npc, target, around))
            }
        },
        Class::Attack => {
            // The think's drops, then goal 4's high level in the same
            // frame: its ally drop (goal and sub 0, the target kept), then
            // its gone-or-breaking-up drop (the sub kept).
            let keeps = Decision {
                keeps_pursuit: true,
                ..dropped
            };
            if drops_disabled_warship(npc, target, around) {
                keeps
            } else if dry {
                dropped
            } else if spares(npc, target, around, rules) {
                of(Goal::Idle)
            } else if gone_or_dying(target, around) {
                keeps
            } else {
                of(attacked(npc, target, around))
            }
        }
    }
}

/// How `_PirateWarshipAI`'s second classification (@0x8c5e1-0x8c743) takes
/// a ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    /// It plunders it (goal 0xd).
    Plunder,
    /// It would plunder it, but the ship is boarded: it waits if another
    /// ship boards it, and attacks it otherwise.
    Boarded,
    /// It attacks it (goal 4).
    Attack,
}

/// How a pirate takes `target` among `around` (see [`Class`]): the player,
/// a former escort or a class of `InherentAI` 2 or below, with a crew
/// above none, is plundered unless boarded.
fn classify(target: ShipRef, around: &Surroundings) -> Class {
    let (trader, crew, boarded) = match target {
        ShipRef::Player => (
            true,
            around.player.map_or(0, |player| player.hull.crew),
            false,
        ),
        ShipRef::Npc(id) => around.npc(id).map_or((false, 0, false), |other| {
            (
                other.former.is_some() || other.hull.inherent_ai <= 2,
                other.hull.crew,
                other.boarded,
            )
        }),
    };
    match (trader && crew > 0, boarded) {
        (true, false) => Class::Plunder,
        (true, true) => Class::Boarded,
        (false, _) => Class::Attack,
    }
}

/// The disabled-warship drop's test (@0x8c7b4-0x8c837): `target` is a
/// disabled NPC among `around` of a class whose `InherentAI` is above 2,
/// not a former escort, and `npc` holds no destroying weapon or is fully
/// dry.
fn drops_disabled_warship(npc: &Npc, target: ShipRef, around: &Surroundings) -> bool {
    let ShipRef::Npc(id) = target else {
        return false;
    };
    around.npc(id).is_some_and(|other| {
        other.condition.disabled()
            && other.hull.inherent_ai > 2
            && other.former.is_none()
            && (!fire::destroys(npc) || ammo_left(npc) == AmmoLeft::Dry)
    })
}

/// How `npc` attacks `target` among `around`, as goal 4's high level
/// flies it: a snipe for a hopeless chase, unless it stands off.
fn attacked(npc: &Npc, target: ShipRef, around: &Surroundings) -> Goal {
    if !npc.hull.stands_off && hopeless_chase(npc, target, around) {
        Goal::Snipe(target)
    } else {
        Goal::Attack(target)
    }
}

/// Whether `target` is outside [`DOGFIGHT_BOX`] of `npc` on either axis,
/// by float abs (`_HighLevelAIHandler` @0x8ea49-0x8eac6); a ship not in
/// the system is not.
fn outside_box(npc: &Npc, target: ShipRef, around: &Surroundings) -> bool {
    around.state_of(target).is_some_and(|at| {
        let off = at.position - npc.state.position;
        off.x.abs() > DOGFIGHT_BOX || off.y.abs() > DOGFIGHT_BOX
    })
}

/// Whether `ship` has left the system or is breaking up, as the head drop
/// reads it (its slot +0xb8 clear, or `_IsDying`, @0x8c315-0x8c328): a ship
/// still jumping in is neither.
fn gone_or_dying(ship: ShipRef, around: &Surroundings) -> bool {
    !around.condition_of(ship).is_some_and(Condition::standing)
}

/// A decision of `goal` alone.
fn of(goal: Goal) -> Decision {
    Decision {
        goal,
        ..Decision::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::PlayerSide;
    use crate::ai::fixture::{
        NEUTRAL, PIRATES, TRADERS, around, govts, n, player, ship, short_of_ammo, sites,
    };
    use crate::catalog::{GovtRecord, ShipId, StellarId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::weapon::WeaponSpec;
    use crate::fleet::Escort;
    use crate::geometry::Vec2;
    use crate::govt::{ALWAYS_ATTACKS_PLAYER, Governments, PLUNDERS, WARSHIPS_RETREAT, XENOPHOBIC};
    use crate::reserves::Reserves;
    use crate::self_repair::FormerEscort;
    use crate::traffic::npc::AiType;
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::{govt, weapon};

    const P: ShipRef = ShipRef::Player;
    /// Where an idle pirate heads: the one planet of [`sites`].
    const IDLE: Goal = Goal::Land(StellarId(128));

    /// The pirate, NPC 1 at the centre in berth 1, a warship of the
    /// pirates.
    fn pirate() -> Npc {
        let mut npc = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        npc.berth = 1;
        npc
    }

    /// The pirate with `goal` and `target`.
    fn holding(goal: Goal, target: Option<ShipRef>) -> Npc {
        Npc {
            goal,
            target,
            ..pirate()
        }
    }

    /// A trader, NPC `id` in berth `id`, `y` below the centre, of a class
    /// with a crew of 1 and `InherentAI` 2, intact.
    fn trader(id: u32, y: f32) -> Npc {
        let mut npc = ship(id, TRADERS, AiType::WimpyTrader, 0.0, y);
        npc.berth = id;
        npc.hull.crew = 1;
        npc.hull.inherent_ai = 2;
        npc
    }

    /// The trader disabled.
    fn wreck(id: u32, y: f32) -> Npc {
        Npc {
            condition: Condition::Disabled,
            ..trader(id, y)
        }
    }

    /// The trader of a warship's class (`InherentAI` 3).
    fn warship_class(id: u32, y: f32) -> Npc {
        let mut npc = trader(id, y);
        npc.hull.inherent_ai = 3;
        npc
    }

    /// The four governments, the pirates' with `flags`.
    fn pirate_govts(flags: u16) -> Governments {
        Governments::new([
            GovtRecord {
                crime_tol: 6,
                classes: [1, -1, -1, -1],
                ..govt(140)
            },
            GovtRecord {
                crime_tol: 6,
                allies: [1, -1, -1, -1],
                classes: [2, -1, -1, -1],
                ..govt(141)
            },
            GovtRecord {
                flags,
                enemies: [1, -1, -1, -1],
                classes: [3, -1, -1, -1],
                ..govt(142)
            },
            govt(143),
        ])
    }

    /// The decision of `npcs[0]` by `govts`, the player as `side` (if
    /// any), in the traders' system where its record is 0.
    fn decided_by(govts: &Governments, npcs: &[Npc], side: Option<&PlayerSide>) -> Decision {
        let sites = sites();
        let mut seen = around(&sites, npcs, govts, (0.0, 0.0), 0);
        seen.player = side.copied();
        think(
            &npcs[0],
            &seen,
            TargetRules::default(),
            &mut Draws::of(&[0, 0, 0]),
        )
    }

    /// The decision of `npcs[0]`, the player intact far off with no crew.
    fn decided(npcs: &[Npc]) -> Decision {
        decided_by(&govts(0, 0), npcs, Some(&player(0.0, 3000.0)))
    }

    /// The player at (`x`, `y`) as `condition`, its class's crew `crew`.
    fn side(x: f32, y: f32, condition: Condition, crew: i16) -> PlayerSide {
        let mut side = player(x, y);
        side.condition = condition;
        side.hull.crew = crew;
        side
    }

    /// A plunder of `ship`, not hopeless.
    fn plunder(ship: ShipRef) -> Goal {
        Goal::Plunder {
            ship,
            hopeless: false,
        }
    }

    /// A decision of `goal` alone.
    fn only(goal: Goal) -> Decision {
        of(goal)
    }

    /// A decision dropping its fight: no goal, its target cleared.
    fn dropped() -> Decision {
        Decision {
            clears_target: true,
            ..of(Goal::Idle)
        }
    }

    #[test]
    fn with_no_target_it_plunders_the_nearest_disabled_ship_first() {
        let npcs = [pirate(), trader(2, 100.0), wreck(3, 300.0)];
        assert_eq!(
            decided(&npcs),
            only(plunder(n(3))),
            "before the intact trader"
        );
    }

    #[test]
    fn with_nothing_to_plunder_it_plunders_the_warships_pick_or_attacks_a_warship_class() {
        assert_eq!(decided(&[pirate(), trader(2, 300.0)]), only(plunder(n(2))));
        assert_eq!(
            decided(&[pirate(), warship_class(2, 300.0)]),
            only(Goal::Attack(n(2)))
        );
        assert_eq!(
            decided(&[pirate()]),
            Decision {
                goal: IDLE,
                clears_target: true,
                ..Decision::default()
            },
            "its business, its target cleared"
        );
    }

    #[test]
    fn the_player_is_plundered_with_a_crew_and_attacked_without() {
        let govts = govts(0, 0);
        let downed = |crew| side(0.0, 100.0, Condition::Disabled, crew);
        assert_eq!(
            decided_by(&govts, &[pirate()], Some(&downed(1))),
            only(plunder(P)),
            "its plunder pick, which makes no call"
        );
        let attacking = [holding(Goal::Attack(P), Some(P))];
        let intact = |crew| side(0.0, 100.0, Condition::Intact, crew);
        assert_eq!(
            decided_by(&govts, &attacking, Some(&intact(3))),
            only(plunder(P))
        );
        assert_eq!(
            decided_by(&govts, &attacking, Some(&intact(0))),
            only(Goal::Attack(P))
        );
        let plundering = [holding(plunder(P), Some(P))];
        assert_eq!(
            decided_by(&govts, &plundering, Some(&intact(-1))),
            only(Goal::Attack(P)),
            "a crew of -1 is no crew"
        );
    }

    #[test]
    fn a_crew_of_none_or_below_is_attacked_and_a_former_escort_of_any_class_plundered() {
        for (crew, goal) in [
            (-1, Goal::Attack(n(2))),
            (0, Goal::Attack(n(2))),
            (1, plunder(n(2))),
        ] {
            let mut victim = trader(2, 300.0);
            victim.hull.crew = crew;
            let npcs = [holding(plunder(n(2)), Some(n(2))), victim];
            assert_eq!(decided(&npcs).goal, goal, "{crew}");
        }
        let mut escort = warship_class(2, 300.0);
        escort.former = Some(FormerEscort {
            record: Escort {
                ship: ShipId(128),
                reserves: Reserves::full(1.0, 1.0, 1.0),
                order: None,
                carried: false,
                wage: None,
                person: None,
            },
        });
        let npcs = [holding(Goal::Attack(n(2)), Some(n(2))), escort];
        assert_eq!(decided(&npcs), only(plunder(n(2))));
    }

    #[test]
    fn a_disabled_warship_class_is_dropped_keeping_the_pursuit_without_destroying_weapons() {
        // AI type 1, so a plunder candidate, but of a warship's class.
        let downed = Npc {
            condition: Condition::Disabled,
            ..warship_class(2, 300.0)
        };
        assert_eq!(
            decided(&[pirate(), downed.clone()]),
            Decision {
                keeps_pursuit: true,
                ..dropped()
            },
            "goal 0 and target -1 alone (@0x8c839-0x8c842)"
        );
        let mut armed = pirate();
        armed.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                mass_dmg: 1,
                ..weapon(128)
            }),
            1,
        )]);
        assert_eq!(
            decided(&[armed, downed]),
            only(Goal::Attack(n(2))),
            "it finishes it off"
        );
    }

    #[test]
    fn only_a_pick_that_calls_it_turns_the_pirate_on_the_player() {
        let downed = side(0.0, 100.0, Condition::Disabled, 1);
        let plain = decided_by(&govts(0, 0), &[pirate()], Some(&downed));
        assert!(!plain.turns_on_player);
        let xenophobes = pirate_govts(PLUNDERS | XENOPHOBIC);
        let intact = side(0.0, 100.0, Condition::Intact, 1);
        assert_eq!(
            decided_by(&xenophobes, &[pirate()], Some(&intact)),
            Decision {
                turns_on_player: true,
                ..only(plunder(P))
            },
            "the xenophobe's pick (@0x8a40e)"
        );
    }

    #[test]
    fn a_target_breaking_up_or_gone_is_dropped_and_the_search_runs_at_once() {
        let mut dying = wreck(2, 300.0);
        dying.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let npcs = [holding(Goal::Attack(n(2)), Some(n(2))), dying];
        assert_eq!(decided(&npcs), only(plunder(n(2))), "the nearest still");
        let gone = [holding(plunder(n(9)), Some(n(9)))];
        assert_eq!(decided(&gone), dropped().goal_of(IDLE));
        let mut jumping = trader(2, 300.0);
        jumping.mode = crate::traffic::npc::Mode::JumpingIn { tick: 3 };
        let npcs = [holding(plunder(n(2)), Some(n(2))), jumping];
        assert_eq!(
            decided(&npcs),
            only(plunder(n(2))),
            "a ship jumping in stays"
        );
    }

    #[test]
    fn attacking_or_plundering_it_takes_the_target_a_hit_gave_it() {
        for goal in [plunder(n(2)), Goal::Attack(n(2)), Goal::Snipe(n(2))] {
            let npcs = [
                holding(goal, Some(n(3))),
                wreck(2, 100.0),
                warship_class(3, 300.0),
            ];
            assert_eq!(decided(&npcs), only(Goal::Attack(n(3))), "{goal:?}");
        }
    }

    #[test]
    fn idle_with_a_target_it_searches_and_ignores_the_provoker() {
        let hitter = ship(3, NEUTRAL, AiType::Warship, 0.0, 200.0);
        let idle = holding(Goal::Idle, Some(n(3)));
        assert_eq!(
            decided(&[idle.clone(), hitter.clone(), wreck(2, 300.0)]),
            only(plunder(n(2)))
        );
        assert_eq!(decided(&[idle, hitter]), dropped().goal_of(IDLE));
    }

    #[test]
    fn holding_with_a_target_it_holds_and_with_none_it_searches() {
        let hitter = ship(3, NEUTRAL, AiType::Warship, 0.0, 200.0);
        let npcs = [holding(Goal::Hold, Some(n(3))), hitter, wreck(2, 300.0)];
        assert_eq!(decided(&npcs), only(Goal::Hold));
        assert_eq!(
            decided(&[holding(Goal::Hold, None), wreck(2, 300.0)]),
            only(plunder(n(2)))
        );
        assert_eq!(
            decided(&[holding(Goal::Hold, None)]),
            dropped().goal_of(IDLE),
            "it leaves the hold"
        );
    }

    #[test]
    fn bound_to_land_with_a_target_an_npcs_hit_gave_it_it_keeps_landing() {
        let hitter = ship(3, NEUTRAL, AiType::Warship, 0.0, 200.0);
        let bound = holding(IDLE, Some(n(3)));
        assert_eq!(
            decided(&[bound.clone(), hitter, wreck(2, 300.0)]),
            only(IDLE),
            "no search while it has a target"
        );
        let unbound = holding(IDLE, None);
        assert_eq!(
            decided(&[unbound.clone(), wreck(2, 300.0)]),
            only(plunder(n(2)))
        );
        let sites = sites();
        let govts = govts(0, 0);
        let npcs = [unbound];
        let seen = around(&sites, &npcs, &govts, (0.0, 3000.0), 0);
        let mut chance = Draws::of(&[]);
        assert_eq!(
            think(&npcs[0], &seen, TargetRules::default(), &mut chance),
            Decision {
                clears_target: true,
                ..only(IDLE)
            },
            "with nothing to fight it keeps the stellar it is bound for"
        );
        assert!(chance.asked.is_empty(), "no new pick");
        let gate = holding(Goal::EnterGate(StellarId(130)), Some(n(2)));
        assert_eq!(decided(&[gate.clone(), wreck(2, 300.0)]), only(gate.goal));
        let jumping = holding(Goal::JumpOut, Some(n(2)));
        assert_eq!(decided(&[jumping, wreck(2, 300.0)]), only(Goal::JumpOut));
    }

    #[test]
    fn a_boarded_victim_is_waited_on_or_else_attacked() {
        let mut boarded = wreck(2, 300.0);
        boarded.boarded = true;
        let npcs = [holding(plunder(n(2)), Some(n(2))), boarded.clone()];
        assert_eq!(
            decided(&npcs),
            Decision {
                board_wait: Some(n(2)),
                ..only(Goal::Attack(n(2)))
            }
        );
        boarded.boarded = false;
        let npcs = [holding(plunder(n(2)), Some(n(2))), boarded];
        assert_eq!(decided(&npcs), only(plunder(n(2))));
    }

    #[test]
    fn fully_dry_it_drops_an_attack_or_a_plunder_and_still_waits_on_a_boarded_victim() {
        let dry = |goal, target| short_of_ammo(holding(goal, target), AmmoLeft::Dry, true);
        for goal in [Goal::Attack(n(2)), plunder(n(2))] {
            let npcs = [dry(goal, Some(n(2))), warship_class(2, 300.0)];
            assert_eq!(decided(&npcs), dropped(), "{goal:?}: its sub-goal too");
            let npcs = [dry(goal, Some(n(2))), trader(2, 300.0)];
            assert_eq!(decided(&npcs), dropped(), "{goal:?}");
        }
        let mut boarded = wreck(2, 300.0);
        boarded.boarded = true;
        let npcs = [dry(plunder(n(2)), Some(n(2))), boarded];
        assert_eq!(
            decided(&npcs),
            Decision {
                board_wait: Some(n(2)),
                ..dropped()
            }
        );
        let npcs = [
            short_of_ammo(
                holding(plunder(n(2)), Some(n(2))),
                AmmoLeft::OnlyUnlimited,
                true,
            ),
            trader(2, 300.0),
        ];
        assert_eq!(decided(&npcs), only(plunder(n(2))), "not fully dry");
    }

    #[test]
    fn it_never_takes_its_governments_word() {
        for flags in [
            PLUNDERS | ALWAYS_ATTACKS_PLAYER,
            PLUNDERS | crate::govt::NEVER_ATTACKS_PLAYER,
        ] {
            let govts = pirate_govts(flags);
            let near = side(0.0, 100.0, Condition::Intact, 0);
            let decision = decided_by(&govts, &[pirate()], Some(&near));
            assert_eq!(decision, dropped().goal_of(IDLE), "{flags:#x}");
            let attacking = [holding(Goal::Attack(P), Some(P))];
            let decision = decided_by(&govts, &attacking, Some(&near));
            assert_eq!(decision, only(Goal::Attack(P)), "{flags:#x}");
        }
    }

    #[test]
    fn it_never_retreats_by_odds_shield_or_ammunition() {
        let govts = pirate_govts(PLUNDERS | WARSHIPS_RETREAT);
        let mut worn = short_of_ammo(
            holding(Goal::Attack(n(2)), Some(n(2))),
            AmmoLeft::OnlyUnlimited,
            true,
        );
        worn.reserves.shield.now = 0.5;
        worn.aggression = 1;
        let mut strong = warship_class(2, 300.0);
        strong.hull.strength = 10_000.0;
        strong.goal = Goal::Attack(n(1));
        let decision = decided_by(&govts, &[worn, strong], Some(&player(0.0, 3000.0)));
        assert_eq!(decision, only(Goal::Attack(n(2))));
    }

    #[test]
    fn its_count_wait_is_kept_whatever_became_of_the_victim() {
        for ticks_left in [0, 1, 120] {
            let wait = Goal::CountWait { ticks_left };
            assert_eq!(decided(&[holding(wait, None), wreck(2, 300.0)]), only(wait));
            assert_eq!(decided(&[holding(wait, Some(n(9)))]), only(wait));
        }
    }

    #[test]
    fn a_plunder_is_hopeless_only_outside_the_dogfight_box() {
        // A heavy pirate behind the player, which runs off at 1 a tick.
        let mut heavy = holding(plunder(P), Some(P));
        heavy.hull.mass = 100.0;
        let running = |y: f32| {
            let mut side = side(0.0, y, Condition::Intact, 1);
            side.state.velocity = Vec2::new(0.0, -1.0);
            side
        };
        let govts = govts(0, 0);
        for y in [-165.1, -400.0] {
            assert_eq!(
                decided_by(&govts, &[heavy.clone()], Some(&running(y))),
                only(Goal::Plunder {
                    ship: P,
                    hopeless: true,
                }),
                "{y}"
            );
        }
        for y in [-165.0, -100.0] {
            assert_eq!(
                decided_by(&govts, &[heavy.clone()], Some(&running(y))),
                only(plunder(P)),
                "{y}: inside, never asked"
            );
        }
        let mut attacker = heavy.clone();
        attacker.goal = Goal::Attack(P);
        let mut crewless = running(-100.0);
        crewless.hull.crew = 0;
        assert_eq!(
            decided_by(&govts, &[attacker], Some(&crewless)),
            only(Goal::Snipe(P)),
            "goal 4 asks inside too"
        );
    }

    #[test]
    fn a_plunder_has_no_ally_drop_and_an_attack_of_an_ally_drops_it() {
        let mut kin = trader(2, 300.0);
        kin.govt = Some(PIRATES);
        let npcs = [holding(Goal::Attack(n(2)), Some(n(2))), kin.clone()];
        assert_eq!(decided(&npcs), only(plunder(n(2))), "goal 0xd keeps it");
        kin.hull.inherent_ai = 3;
        let npcs = [holding(Goal::Attack(n(2)), Some(n(2))), kin];
        assert_eq!(
            decided(&npcs),
            only(Goal::Idle),
            "goal 4's ally drop: goal and sub 0, the target kept"
        );
    }

    #[test]
    fn a_warship_class_attacked_is_sniped_in_a_hopeless_chase_unless_it_stands_off() {
        let mut fleeing = warship_class(2, 300.0);
        fleeing.goal = Goal::Flee(n(1));
        fleeing.state.velocity = Vec2::new(0.0, 1.0);
        let mut heavy = holding(Goal::Attack(n(2)), Some(n(2)));
        heavy.hull.mass = 100.0;
        assert_eq!(
            decided(&[heavy.clone(), fleeing.clone()]),
            only(Goal::Snipe(n(2)))
        );
        heavy.hull.stands_off = true;
        assert_eq!(decided(&[heavy, fleeing]), only(Goal::Attack(n(2))));
    }

    #[test]
    fn the_pirate_targets_its_fight_its_kept_target_and_nothing_waiting() {
        let sites = sites();
        let npcs = [pirate()];
        let around = Surroundings::new(&sites, &npcs);
        let ai = PirateWarship::default();
        let target = |goal, target| ai.target(&holding(goal, target), &around);
        assert_eq!(target(plunder(n(2)), Some(n(3))), Some(n(2)));
        assert_eq!(target(Goal::Attack(n(2)), None), Some(n(2)));
        assert_eq!(target(IDLE, Some(n(3))), Some(n(3)), "kept with its goal");
        assert_eq!(target(Goal::Idle, None), None);
        assert_eq!(target(Goal::CountWait { ticks_left: 3 }, Some(n(3))), None);
    }

    #[test]
    fn a_waiting_pirate_fires_nothing_and_a_plundering_one_fires_at_its_victim() {
        let gun = WeaponSpec::new(&WeaponRecord {
            speed: 1000,
            count: 30,
            energy_dmg: 5,
            ..weapon(141)
        });
        let armed = |goal| Npc {
            armament: Armament::new([(gun, 1)]),
            ..holding(goal, None)
        };
        let ai = PirateWarship::default();
        let sites = sites();
        let fired = |npc: Npc| {
            let npcs = [npc, trader(2, -100.0)];
            let around = Surroundings::new(&sites, &npcs);
            ai.trigger(&npcs[0], &around)
        };
        assert_eq!(
            fired(armed(Goal::CountWait { ticks_left: 9 })),
            Trigger::default()
        );
        let plundering = fired(armed(plunder(n(2))));
        assert_eq!(plundering.only, Some(crate::catalog::WeaponId(141)));
    }

    #[test]
    fn the_pirate_decides_by_its_think_and_answers_a_hit_as_a_warship() {
        let npcs = [pirate(), wreck(2, 300.0)];
        let sites = sites();
        let govts = govts(0, 0);
        let around = around(&sites, &npcs, &govts, (0.0, 3000.0), 0);
        let ai = PirateWarship::default();
        assert_eq!(
            ai.decide(&npcs[0], &around, &mut Draws::of(&[])),
            plunder(n(2))
        );
        let hit = Strike {
            damage: 4.0,
            ..crate::combat::fixture::strike(n(1), n(2))
        };
        assert_eq!(
            ai.react(&npcs[0], &hit, &around),
            react::answer(&npcs[0], &hit, &around, RuleSource::Engine)
        );
        assert_eq!(ai.react(&npcs[0], &hit, &around).target, Some(n(2)));
    }

    #[test]
    fn a_disabled_class_of_inherent_ai_2_without_a_crew_is_attacked_not_dropped() {
        // The drop needs `InherentAI` above 2 (`jle` @0x8c80e).
        let mut crewless = wreck(2, 300.0);
        crewless.hull.crew = 0;
        let npcs = [holding(Goal::Attack(n(2)), Some(n(2))), crewless.clone()];
        assert_eq!(decided(&npcs), only(Goal::Attack(n(2))));
        crewless.hull.inherent_ai = 3;
        let npcs = [holding(Goal::Attack(n(2)), Some(n(2))), crewless];
        assert_eq!(
            decided(&npcs),
            Decision {
                keeps_pursuit: true,
                ..dropped()
            }
        );
    }

    #[test]
    fn the_box_is_measured_from_the_pirate_on_either_axis() {
        // A heavy pirate off the centre, the player running off across.
        let mut heavy = holding(plunder(P), Some(P));
        heavy.hull.mass = 100.0;
        heavy.state.position = Vec2::new(500.0, 0.0);
        let govts = govts(0, 0);
        let running = |x: f32| {
            let mut side = side(500.0 + x, 0.0, Condition::Intact, 1);
            side.state.velocity = Vec2::new(1.0, 0.0);
            side
        };
        let hopeless = |x: f32| decided_by(&govts, &[heavy.clone()], Some(&running(x))).goal;
        assert_eq!(
            hopeless(400.0),
            Goal::Plunder {
                ship: P,
                hopeless: true
            }
        );
        assert_eq!(hopeless(100.0), plunder(P), "inside, from the pirate");
        assert_eq!(hopeless(165.0), plunder(P), "165 is inside");
    }

    impl Decision {
        /// This decision with `goal`.
        fn goal_of(self, goal: Goal) -> Self {
            Self { goal, ..self }
        }
    }
}
