//! A ship docking with the disabled player to repair it, or with the
//! disabled NPC a hit turned it on: goal 0xf's block
//! of `_HighLevelAIHandler` (@0x8f6d4-0x8f82f in the `EV Nova` executable,
//! [`high_level`]) and the repair half of `_LowLevelAIHandler`'s sub 0xf
//! (@0x86d4f-0x86f37, [`complete`]). The docking itself, its substates
//! 0xb and 0xf, is the plunder's ([`plunder_dock`]), which this shares.
//!
//! - **The high level** ([`high_level`]), on each think while its AI wait
//!   is out (@0x8d4b9-0x8d4d2 returns for every goal but 10 and 0xe while
//!   it runs). With no target nothing changes (@0x8f6e9), nor with one
//!   whose slot went, which the high level's slot test has made -1
//!   (@0x8d4e4-0x8d50d). Its target is
//!   any ship: the player it was asked to repair, or the NPC an NPC's hit
//!   turned it on (`_DamageShip` @0x3bcf6). With that ship not
//!   disabled (@0x8f702-0x8f715, `_IsDisabled`: disabled or breaking up)
//!   it gives up: goal 0, sub 0, target and nav -1 (@0x8f717-0x8f732), so
//!   a hit by an intact ship ends the help. The +0xb8 test at @0x8f702
//!   never fails: the slot test came first.
//! - **A patient gone.** The type AIs return at once for goal 0xf (see
//!   [`assist_think`](crate::traffic::assist_think)), so a repairer whose
//!   patient's slot went keeps its goal, its sub and its nav (+0x6c,
//!   [`Npc::docking`]), which no slot test clears: closing (sub 0xb), it
//!   closes on the freed slot's record and holds there, no high level
//!   writing sub 0xf; docking (sub 0xf), it matches the record's velocity,
//!   starts its dock and completes on it, then drifts. The armour and the
//!   boarded flag land on the record, which nothing reads before a spawn
//!   rewrites it (+0xb9 @0x3c981). A spawn into the slot hands the
//!   docking to the newcomer, as the nav names the slot
//!   ([`Traffic::slot_taken`](crate::traffic::Traffic::slot_taken)). The
//!   dock's completion, creep and velocity match are read where the fly
//!   half decides them today;
//!   rdm:phase/combat-ai-fidelity-cross-roadmap/phase-12-left-out-of-phase-9
//!   moves them to the slot. Otherwise nav = the target
//!   (@0x8f74c) and, within [`repair_box`] of it on both axes,
//!   inclusive, it docks (sub 0xf, @0x8f815); otherwise it closes (sub 0xb,
//!   @0x8f823). There is no approach (sub 9): the box is 2-way, not the
//!   plunder's 3-way one. The box is
//!   [`INERTIALESS_BOX`](plunder_dock::INERTIALESS_BOX) times as wide for
//!   an inertialess repairer (`_ShipIsInertialess` @0x8f781-0x8f797), the
//!   integer box multiplied, so a fractional one truncates first. The
//!   original takes the result as a 16-bit word (`movswl %bx` @0x8f79d);
//!   this keeps an `i32`.
//! - **The dock** is the plunder's sub 0xf: inside 3 pixels with no wait
//!   left, the ship (the player or an NPC) is boarded and the wait is
//!   `Rand(80)` + 100 (@0x86fa4-0x8700a).
//! - **The completion** ([`complete`]): docked, with its wait above none and
//!   at most [`DOCK_WAIT`](plunder_dock::DOCK_WAIT) (@0x86d54-0x86d6d), the
//!   ship is unboarded (@0x86d92), its armour raised [`REPAIR_STEP`] at a
//!   time while it is disabled ([`raise_armor`], @0x86d9c-0x86dcf), and, for the player
//!   alone (@0x86dd4), "`CommName`:  Repairs complete, `ship name`." said
//!   for 240 ticks with `snd ` 151 (@0x86ddf-0x86ef4): the session's, from
//!   [`PirateEvent::PlayerRepaired`](crate::traffic::plunder::PirateEvent::PlayerRepaired),
//!   or silently for an NPC, from
//!   [`PirateEvent::NpcRepaired`](crate::traffic::plunder::PirateEvent::NpcRepaired).
//!   The repairer then drifts (goal 0xe) with a wait of [`REPAIR_HOLD`],
//!   its target, nav and substate cleared (@0x86f09-0x86f31).
//!
//! With the wait counted down after each tick's flight (`_HandleShip`
//! @0x35037-0x35048, after every ship's `_AIDispatch`), a dock started on
//! tick k with wait W repairs on tick k + max(1, W - 100), and the drift
//! that follows steps 100 times and sets goal 0 on its 100th.
//!
//! The repair's high level runs on each of its thinks though an assister
//! decides nothing, provoked or not: the type AIs return at once for
//! goals 9 and 0xf (`_WimpyTraderAI` @0x8b045-0x8b05c, see
//! [`assist_think`](crate::traffic::assist_think)), but `_AIDispatch`
//! runs the high and low levels whatever they did (@0x901e5-0x901f4).
//!
//! Only `_AIMakeShipRepairPlayer` (@0x82df2) sets goal 0xf, always at the
//! player; the NPC target comes from a hit's retarget, which names the
//! strike's maker whatever its state, so a shot from a ship since disabled
//! turns the repairer on a disabled NPC. The `_statusRedraw` writes and
//! +0x8c -1 are recorded only. The original's armour loop has
//! no cap; [`raise_armor`] stops at the armour's most, so a disable rule
//! that never lets the ship fly still ends.

use crate::ai::{Goal, Surroundings};
use crate::combat::ShipRef;
use crate::combat::hull::{Condition, HullSpec};
use crate::hail::Help;
use crate::inertialess::InertialDampener;
use crate::reserves::Gauge;
use crate::traffic::npc::Npc;
#[cfg(doc)]
use crate::traffic::plunder_dock;
use crate::traffic::plunder_dock::{BOX_PER_TURN, BOX_TURNS, DockStep, DockSub, inertialess_box};

/// What a repair box adds (the float 50.0 @0xdd064).
pub const REPAIR_BOX_BASE: f32 = 50.0;
/// The AI wait a repairer drifts for after its repair (the float 100.0
/// @0x86f09).
pub const REPAIR_HOLD: u32 = 100;
/// The armour a repair raises at a time (the float 1.0 @0xdd068).
pub const REPAIR_STEP: f32 = 1.0;

/// The repair box of a class whose turn (+0x38) is `class_turn`:
/// trunc((10 − turn) × 30 + 50), in floats (@0x8f740-0x8f77d), with no
/// guard.
#[must_use]
pub fn repair_box(class_turn: f32) -> i32 {
    // A multiply then an add, rounded at each, as the original's two
    // instructions are (@0x8f76d-0x8f775); within an i32 for any turn.
    #[allow(clippy::suboptimal_flops, clippy::cast_possible_truncation)]
    let side = ((BOX_TURNS - class_turn) * BOX_PER_TURN + REPAIR_BOX_BASE) as i32;
    side
}

/// The high level of `npc`'s repair of the ship it targets among `around`
/// (see the module docs): none when it is no repair or has no target, nor
/// when its target's slot is inactive, which the high level's slot test
/// makes -1 first (@0x8d4e4-0x8d50d), so the block is skipped
/// (@0x8f6e9) and the goal, sub and nav kept (the traffic's
/// `dispatch_leave` runs the test itself). A dampener counts toward its
/// inertialess box as `rule` says.
#[must_use]
pub fn high_level(npc: &Npc, around: &Surroundings, rule: InertialDampener) -> Option<DockStep> {
    if npc.goal != Goal::Assist(Help::Repair) {
        return None;
    }
    let ship = npc.target?;
    // The player's slot is active while the AI loop runs in its system,
    // so the block reads it whatever the world shows of it.
    let gone = match ship {
        ShipRef::Npc(_) => !around.condition_of(ship).is_some_and(Condition::active),
        ShipRef::Player => false,
    };
    if gone {
        return None;
    }
    let Some(at) = around
        .state_of(ship)
        .filter(|_| around.condition_of(ship).is_some_and(Condition::disabled))
        .map(|state| state.position)
    else {
        return Some(DockStep::GiveUp);
    };
    // f32: the box is converted back to a float for the test.
    #[allow(clippy::cast_precision_loss)]
    let side = inertialess_box(npc, repair_box(npc.hull.class_turn), rule) as f32;
    let off = npc.state.position - at;
    Some(DockStep::Sub(
        if off.x.abs() <= side && off.y.abs() <= side {
            DockSub::Dock
        } else {
            DockSub::Close
        },
    ))
}

/// `npc`'s repair done (@0x86f09-0x86f31): it drifts with a wait of
/// [`REPAIR_HOLD`], its target and docking dropped, its travel stellar
/// with the nav (+0x6c -1, @0x86f31).
pub fn complete(npc: &mut Npc) {
    npc.goal = Goal::Drift;
    npc.ai_wait = REPAIR_HOLD;
    npc.target = None;
    npc.docking = None;
    npc.travel = None;
}

/// A repaired ship's armour raised (`_LowLevelAIHandler`'s loop
/// @0x86d9c-0x86dcf): while `disabled` has a ship of `hull` disabled at
/// `armor`, it rises [`REPAIR_STEP`], the rule asked again after each
/// step. The session calls it for the player and for an NPC, with its own
/// rule (`nova_sim::Session`'s `repair_dock` module).
///
/// The original's loop has no cap. This one stops at the armour's most,
/// after as many steps as the points it is short (a step is a point), so a
/// rule that never lets the ship fly, such as `_IsDisabled` holding a
/// mission's rescue ship disabled at any armour, still ends.
pub fn raise_armor(
    armor: &mut Gauge,
    hull: &HullSpec,
    disabled: impl Fn(Gauge, &HullSpec) -> bool,
) {
    // At most the steps up to its most, so a rule that never lets it fly
    // still ends.
    let steps = (armor.max - armor.now).ceil().max(0.0) as u32;
    for _ in 0..steps {
        if !disabled(*armor, hull) {
            return;
        }
        armor.now = (armor.now + REPAIR_STEP).min(armor.max);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::PlayerSide;
    use crate::combat::ShipRef;
    use crate::combat::hull::class_turn;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::World;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::NpcId;
    use crate::traffic::plunder_dock::Docking;
    use nova_sim_base::fixture::FAST;

    /// A ship repairing the player from (`x`, `y`), its class turn 3 (a
    /// box of 260).
    fn repairer(x: f32, y: f32) -> Npc {
        let mut ship = npc(1, ShipStats::new(FAST, &[]));
        ship.goal = Goal::Assist(Help::Repair);
        ship.target = Some(ShipRef::Player);
        ship.hull.class_turn = 3.0;
        ship.state.position = Vec2::new(x, y);
        ship
    }

    /// The player at (`x`, `y`) in `condition`.
    fn player(x: f32, y: f32, condition: Condition) -> PlayerSide {
        PlayerSide {
            condition,
            ..crate::ai::fixture::player(x, y)
        }
    }

    /// The high level of `ship` with `player` in the system, if any, a
    /// dampener counting for the player alone.
    fn step(ship: &Npc, player: Option<&PlayerSide>) -> Option<DockStep> {
        step_under(ship, player, InertialDampener::PlayerOnly)
    }

    /// [`step`], a dampener counting as `rule` says.
    fn step_under(
        ship: &Npc,
        player: Option<&PlayerSide>,
        rule: InertialDampener,
    ) -> Option<DockStep> {
        let npcs = [ship.clone()];
        let world = World {
            player: player.copied(),
            ..World::new(&[])
        };
        high_level(&npcs[0], &world.around(&npcs), rule)
    }

    /// `ship` on an inertialess hull ([`FAST`]'s, `Flags2` 0x0040).
    fn inertialess(mut ship: Npc) -> Npc {
        ship.stats = ShipStats::new(
            crate::handling::ShipFields {
                flags2: crate::stats::INERTIALESS_HULL,
                ..FAST
            },
            &[],
        );
        ship
    }

    #[test]
    fn the_repair_box_is_ten_less_the_class_turn_times_thirty_plus_fifty_in_floats() {
        assert_eq!(repair_box(3.0), 260);
        assert_eq!(repair_box(8.0), 110);
        // In floats (10 - 3.3) × 30 is 201.0, and 251 with the 50.
        assert_eq!(repair_box(class_turn(33)), 251);
        assert_eq!(repair_box(10.5), 35);
        assert_eq!(repair_box(12.0), -10, "no guard");
    }

    #[test]
    fn the_repair_high_level_docks_inside_the_box_on_both_axes_inclusive_else_closes() {
        let downed = player(0.0, 0.0, Condition::Disabled);
        let at = |x: f32, y: f32| step(&repairer(x, y), Some(&downed));
        assert_eq!(at(260.0, -260.0), Some(DockStep::Sub(DockSub::Dock)));
        assert_eq!(at(-260.0, 260.0), Some(DockStep::Sub(DockSub::Dock)));
        assert_eq!(at(260.5, 0.0), Some(DockStep::Sub(DockSub::Close)));
        assert_eq!(at(0.0, -260.5), Some(DockStep::Sub(DockSub::Close)));
        assert_eq!(
            at(5000.0, 5000.0),
            Some(DockStep::Sub(DockSub::Close)),
            "never an approach"
        );
    }

    #[test]
    fn the_repair_high_level_measures_the_box_from_the_player_where_it_is() {
        let downed = player(1000.0, -1000.0, Condition::Disabled);
        assert_eq!(
            step(&repairer(1260.0, -740.0), Some(&downed)),
            Some(DockStep::Sub(DockSub::Dock))
        );
        assert_eq!(
            step(&repairer(0.0, 0.0), Some(&downed)),
            Some(DockStep::Sub(DockSub::Close))
        );
    }

    #[test]
    fn the_repair_high_level_gives_up_on_a_player_gone_or_no_longer_disabled() {
        let ship = repairer(0.0, 0.0);
        assert_eq!(step(&ship, None), Some(DockStep::GiveUp));
        assert_eq!(
            step(&ship, Some(&player(0.0, 0.0, Condition::Intact))),
            Some(DockStep::GiveUp)
        );
        assert_eq!(
            step(&ship, Some(&player(0.0, 0.0, Condition::Disabled))),
            Some(DockStep::Sub(DockSub::Dock))
        );
    }

    /// The high level of `ship`, its target NPC 2 at (`x`, `y`) in
    /// `condition`, with the player disabled at the centre.
    fn step_at_npc(ship: &Npc, x: f32, y: f32, condition: Condition) -> Option<DockStep> {
        let mut ship = ship.clone();
        ship.target = Some(ShipRef::Npc(NpcId(2)));
        let mut other = npc(2, ShipStats::new(FAST, &[]));
        other.state.position = Vec2::new(x, y);
        other.condition = condition;
        let npcs = [ship, other];
        let world = World {
            player: Some(player(0.0, 0.0, Condition::Disabled)),
            ..World::new(&[])
        };
        high_level(&npcs[0], &world.around(&npcs), InertialDampener::PlayerOnly)
    }

    #[test]
    fn a_repair_whose_target_is_an_intact_npc_gives_up() {
        let ship = repairer(0.0, 0.0);
        assert_eq!(
            step_at_npc(&ship, 0.0, 0.0, Condition::Intact),
            Some(DockStep::GiveUp)
        );
    }

    #[test]
    fn a_repair_whose_target_slot_went_skips_its_block() {
        // The slot test makes +0x70 -1 first (@0x8d4e4-0x8d50d), so the
        // block is skipped (@0x8f6e9): +0xb8's test at @0x8f702 is never
        // reached by a gone slot.
        let ship = repairer(0.0, 0.0);
        assert_eq!(step_at_npc(&ship, 0.0, 0.0, Condition::Destroyed), None);
        let mut gone = ship;
        gone.target = Some(ShipRef::Npc(NpcId(7)));
        let world = World {
            player: Some(player(0.0, 0.0, Condition::Disabled)),
            ..World::new(&[])
        };
        let npcs = [gone];
        assert_eq!(
            high_level(&npcs[0], &world.around(&npcs), InertialDampener::PlayerOnly),
            None,
            "a ship gone"
        );
    }

    #[test]
    fn a_repair_whose_target_is_a_disabled_npc_docks_or_closes_on_it() {
        let ship = repairer(1000.0, 0.0);
        assert_eq!(
            step_at_npc(&ship, 1260.0, -260.0, Condition::Disabled),
            Some(DockStep::Sub(DockSub::Dock)),
            "the box measured from the NPC, not the player"
        );
        assert_eq!(
            step_at_npc(&ship, 1260.5, 0.0, Condition::Disabled),
            Some(DockStep::Sub(DockSub::Close))
        );
        assert_eq!(
            step_at_npc(
                &ship,
                1000.0,
                0.0,
                Condition::Dying {
                    ticks_left: 5,
                    starting: false
                }
            ),
            Some(DockStep::Sub(DockSub::Dock)),
            "breaking up reads as `_IsDisabled`"
        );
    }

    #[test]
    fn an_inertialess_repairer_docks_within_four_times_the_box_and_closes_outside_it() {
        let downed = player(0.0, 0.0, Condition::Disabled);
        let at = |x: f32, y: f32| step(&inertialess(repairer(x, y)), Some(&downed));
        let (dock, close) = (
            Some(DockStep::Sub(DockSub::Dock)),
            Some(DockStep::Sub(DockSub::Close)),
        );
        assert_eq!(at(1040.0, -1040.0), dock, "inclusive");
        assert_eq!(at(1041.0, 0.0), close);
        assert_eq!(at(0.0, 1041.0), close);
        assert_eq!(at(1041.0, 1041.0), close);
        assert_eq!(at(5000.0, 0.0), close, "never an approach");
        let inertial = |x: f32| step(&repairer(x, 0.0), Some(&downed));
        assert_eq!(inertial(1040.0), close);
        assert_eq!(inertial(260.0), dock);
    }

    #[test]
    fn the_inertialess_box_multiplies_the_truncated_box() {
        let downed = player(0.0, 0.0, Condition::Disabled);
        let at = |x: f32| {
            let mut ship = inertialess(repairer(x, 0.0));
            // 259.7, so 259, so 1036, where ×4 of the float is 1038.8.
            ship.hull.class_turn = 3.01;
            step(&ship, Some(&downed))
        };
        assert_eq!(at(1036.0), Some(DockStep::Sub(DockSub::Dock)));
        assert_eq!(at(1037.0), Some(DockStep::Sub(DockSub::Close)));
    }

    #[test]
    fn a_dampener_widens_a_repair_box_only_by_the_bible() {
        let downed = player(0.0, 0.0, Condition::Disabled);
        let mut ship = repairer(1040.0, 0.0);
        ship.stats.dampener = true;
        let under = |rule| step_under(&ship, Some(&downed), rule);
        assert_eq!(
            under(InertialDampener::PlayerOnly),
            Some(DockStep::Sub(DockSub::Close))
        );
        assert_eq!(
            under(InertialDampener::AnyShip),
            Some(DockStep::Sub(DockSub::Dock))
        );
    }

    #[test]
    fn the_repair_high_level_changes_nothing_with_no_target() {
        let mut ship = repairer(0.0, 0.0);
        ship.target = None;
        assert_eq!(
            step(&ship, Some(&player(0.0, 0.0, Condition::Intact))),
            None
        );
    }

    #[test]
    fn the_repair_high_level_is_for_a_repair_only() {
        let mut ship = repairer(0.0, 0.0);
        ship.goal = Goal::Assist(Help::Refuel);
        assert_eq!(
            step(&ship, Some(&player(0.0, 0.0, Condition::Disabled))),
            None
        );
    }

    #[test]
    fn a_completed_repair_drifts_with_a_hold_of_100_and_drops_its_target_and_docking() {
        let mut ship = repairer(0.0, 0.0);
        ship.ai_wait = 40;
        ship.docking = Some(Docking {
            ship: ShipRef::Player,
            sub: DockSub::Dock,
        });
        ship.travel = Some(crate::catalog::StellarId(150));
        complete(&mut ship);
        assert_eq!(ship.goal, Goal::Drift);
        assert_eq!(ship.ai_wait, REPAIR_HOLD);
        assert_eq!((ship.target, ship.docking), (None, None));
        assert!(!ship.travel_set(), "nav -1, @0x86f31");
    }

    /// The armour `raise_armor` leaves a gauge of `now` of `max` with,
    /// and the armour it asked the rule about, in order, the rule calling
    /// a ship disabled while `disabled` says so.
    fn raised(now: f32, max: f32, disabled: impl Fn(f32) -> bool) -> (f32, Vec<f32>) {
        let asked = std::cell::RefCell::new(Vec::new());
        let mut armor = Gauge { now, max };
        raise_armor(&mut armor, &HullSpec::default(), |armor, _hull| {
            asked.borrow_mut().push(armor.now);
            disabled(armor.now)
        });
        (armor.now, asked.into_inner())
    }

    #[test]
    fn a_repair_raises_the_armour_a_step_at_a_time_asking_after_each_until_it_flies() {
        let (now, asked) = raised(10.0, 45.0, |now| now < 20.0);
        assert_eq!(now, 20.0);
        let steps: Vec<f32> = (10..=20u8).map(f32::from).collect();
        assert_eq!(asked, steps, "a point at a time (@0x86d9c-0x86dcf)");
    }

    #[test]
    fn a_repair_raises_the_armour_no_further_than_its_most_under_a_rule_that_never_lets_it_fly() {
        let (now, asked) = raised(10.0, 45.0, |_| true);
        assert_eq!(now, 45.0);
        assert_eq!(asked.last(), Some(&44.0), "no step past the most");
    }

    #[test]
    fn a_repair_of_a_ship_the_rule_lets_fly_raises_nothing() {
        assert_eq!(raised(10.0, 45.0, |_| false), (10.0, vec![10.0]));
    }

    #[test]
    fn a_full_armour_takes_no_step() {
        let (now, asked) = raised(45.0, 45.0, |_| true);
        assert_eq!(now, 45.0);
        assert!(asked.is_empty(), "{asked:?}");
    }

    #[test]
    fn a_part_point_short_of_the_most_takes_one_step_to_it() {
        assert_eq!(raised(44.5, 45.0, |_| true), (45.0, vec![44.5]));
    }

    #[test]
    fn the_values_are_the_originals() {
        assert_eq!([REPAIR_BOX_BASE, REPAIR_STEP], [50.0, 1.0]);
        assert_eq!(REPAIR_HOLD, 100);
    }
}
