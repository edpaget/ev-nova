//! The flight of a landing ship, [`Goal::Land`]: the original's goal 1 as
//! `_HighLevelAIHandler` state 1 (@0x8d8b8-0x8db5a in the `EV Nova`
//! executable) picks its substate and `_LowLevelAIHandler` flies it, by
//! the engine's reading of
//! [`RuleKey::NpcLanding`](crate::RuleKey::NpcLanding). The autopilot
//! flies the other reading, the port's earlier circle at
//! [`LANDING_SPEED`](crate::landing::LANDING_SPEED)
//! ([`autopilot`](crate::traffic::autopilot)).
//!
//! State 1 runs only once the AI wait is out: the wait gate
//! (@0x8d4b9-0x8d4d2) returns from the handler while +0x4c > 0, for every
//! goal but 0xa and 0xe, so the autopilot hands a gate off and flies this
//! only below its AI-wait gate. Each pass, in this order:
//!
//! 1. **The guard** (@0x8d8b8-0x8d8d1): the block runs for goal +0xc8c8 ==
//!    1 with a travel stellar +0x6c ≠ −1; with −1 it does nothing and
//!    keeps its sub-goal. A [`Goal::Land`] always names its stellar, so
//!    the engine can't reach that branch; one whose stellar is not among
//!    the system's brakes (the autopilot's).
//! 2. **The gate hand-off** (@0x8d8d7-0x8d8fe): a stellar whose `Flags2`
//!    (record +0x34, 0x498-byte records at *0x3b5010) has 0x3000, a
//!    hypergate or wormhole, turns the goal to 0x14 and runs state 20 in
//!    the same pass (@0x8db5f, `cmp $0x14` @0x8db62): the autopilot's
//!    hand-off to [`Goal::EnterGate`], see
//!    [`gate_entry`](crate::traffic::gate_entry).
//! 3. **The jump stage** +0x50 = 0 (@0x8d903-0x8d906), every pass.
//!    `warp_out::hold_stage` already zeroes the stage of any ship whose
//!    goal is not a jump, a flee or a leader's jump, so nothing here
//!    writes it.
//! 4. **The box** ([`landing::npc_over`]): the whole-pixel offset from the
//!    stellar's centre, at most [`landing::npc_box`] of its
//!    `_ShipTurnRate` on each axis (@0x8d90d-0x8d9a8; the 16-bit `cwde`
//!    @0x8d929, @0x8d94b and `movsx` @0x8d9b7 not replicated). The turn
//!    is the one its flight flies by, [`Npc::flying_handling`]'s: the
//!    class's turn floored as `_ShipTurnRate` floors it and slowed by its
//!    ion charge, as `_ShipTurnRate` reads it.
//! 5. **Outside** (@0x8d9b2-0x8da07): sub-goal +0xc8ca = 2, the block
//!    ends, and the low level flies sub 2 (@0x8563b-0x857bd,
//!    [`approach_flight::toward`]), whose thrust writes +0x34.
//! 6. **Inside, the unfold** (@0x8da0c-0x8da38): a folding ship (`shän`
//!    flags 0x02, class +0xa24) above fold set 0 unfolds
//!    ([`ship_display::landing_unfold`]), before the speed test, so on the
//!    landing pass too.
//! 7. **The speed gate** (@0x8da41-0x8da8b): moving 0.35 (the double
//!    @0xdda68) or more on either axis ([`inertialess::moving`]) it brakes;
//!    otherwise it lands. A velocity that is not a number on x falls
//!    through to the y test, and one on y lands (`jb`), as `moving` reads
//!    it; `0.35_f32` is below the double, so it lands.
//! 8. **The brake** (@0x8da8d-0x8dad7): sub-goal 1, and its velocity and
//!    speed (+0x48) each × [`LANDING_DRAG`] in doubles
//!    ([`Npc::drag_in_doubles`]), then the low level flies sub 1
//!    ([`leave_flight`]'s stop, not refuelling: goal 1 is not goal 9),
//!    reading the cut velocity. The high level runs every pass, so the cut
//!    applies on every flight tick.
//! 9. **The landing** (@0x8dadc-0x8db5a): it stops and waits at the
//!    stellar ([`stay`]), and the flight gives [`Outcome::Landed`]. The
//!    traffic keeps it in its slot (+0xb8 stays set), so it still counts
//!    toward the system's ships; `_HandleShip` counts its wait down a
//!    tick at a time, `_gSpeedMult` 1.0 (@0x35025-0x35048), as the
//!    traffic counts every AI wait down
//!    ([`plunder::count_down`](crate::traffic::plunder::count_down)),
//!    skipping its turn, thrust and fire meanwhile (@0x34a54-0x34a72;
//!    the autopilot's wait gate and
//!    [`fire::gated`](crate::ai::fire::gated)). `_AIDispatch` runs its
//!    type's think from the next frame whatever the wait (@0x8ff26-0x90016),
//!    in goal 0, so a trader or warship finds [`Npc::visited`] and leaves
//!    by hyperspace or stops, and its high level flies that once the wait
//!    is out. Only a mission ship by the other reading of
//!    [`RuleKey::LandedLeaves`](crate::RuleKey::LandedLeaves) leaves the
//!    system as it lands ([`leaves`]).
//!
//! It plays no sound: state 1 calls no `PlaySound` or `_beepSnd`.

use crate::ai::Goal;
use crate::ai::leave::Leg;
use crate::ai::miner::SCOOPS;
use crate::catalog::{LandingSite, StellarId};
use crate::chance::Chance;
use crate::flight::FlightRules;
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::landing;
use crate::rulebook::RuleSource;
use crate::traffic::autopilot::Outcome;
use crate::traffic::held_burst::{self, Word};
use crate::traffic::inertialess::{self, Inertia};
use crate::traffic::npc::Npc;
use crate::traffic::{approach_flight, leave_flight, ship_display};

/// What a landing ship inside its box but still moving keeps of its
/// velocity and its speed each pass (the double 0.98 @0xdd988).
pub const LANDING_DRAG: f64 = 0.98;

/// Flies `npc`, landing on `site` (not a gate), one tick on `handling`
/// under `flight`, as `inertia` says, by `_HighLevelAIHandler` state 1
/// (see the module docs): outside its box it approaches, inside it
/// unfolds and brakes while it moves 0.35 or more on an axis, and
/// otherwise lands.
pub fn fly(
    npc: &mut Npc,
    site: &LandingSite,
    handling: &Handling,
    flight: FlightRules,
    inertia: Inertia,
    chance: &mut (impl Chance + ?Sized),
) -> Outcome {
    let turn = handling.turn_rate;
    if !landing::npc_over(site, npc.state.position, turn, RuleSource::Engine) {
        let (controls, flown) = approach_flight::toward(&npc.state, site.position, handling);
        held_burst::step(npc, &flown, controls, flight, inertia, Word::Writes);
        return Outcome::Flying;
    }
    ship_display::landing_unfold(npc);
    if inertialess::moving(npc.state.velocity) {
        npc.drag_in_doubles(LANDING_DRAG);
        return leave_flight::fly(npc, Leg::Brake, handling, flight, inertia);
    }
    stay(npc, site.id, chance);
    Outcome::Landed(site.id)
}

/// Whether `npc`, landed, leaves the system as it lands, under
/// `landed_leaves`
/// ([`RuleKey::LandedLeaves`](crate::RuleKey::LandedLeaves)): only a
/// mission's special ship ([`Npc::mission`]) by the other reading, which
/// counts it as one that jumped out. By the engine's reading, as the
/// original, every landed ship stays in its slot (+0xb8 is not cleared,
/// @0x8dadc-0x8db5a), an auxiliary ship and ordinary traffic always.
#[must_use]
pub fn leaves(npc: &Npc, landed_leaves: RuleSource) -> bool {
    npc.mission.is_some() && landed_leaves == RuleSource::Bible
}

/// The least a ship whose class scoops (`shïp` `Flags3` 0x0002) waits at
/// the stellar it landed on, in ticks (@0x8db19-0x8db3a).
pub const SHORT_STAY_BASE: u32 = 100;
/// `Rand(75)`: the spread of that wait.
pub const SHORT_STAY_SPAN: u32 = 75;
/// The least any other ship waits at the stellar it landed on, in ticks
/// (@0x8db41-0x8db5a).
pub const STAY_BASE: u32 = 300;
/// `Rand(200)`: the spread of that wait.
pub const STAY_SPAN: u32 = 200;

/// `npc` landing on `stellar` (@0x8dadc-0x8db5a): it stops, its velocity
/// and speed (+0x48) none, takes goal 0 ([`Goal::Idle`], sub-goal 0), marks
/// the stellar visited (+0x92 = +0x6c @0x8db00, [`Npc::visited`]) and
/// keeps it as its travel stellar (+0x6c is not written, so
/// [`Npc::travel_set`] holds), and starts its AI wait (+0x4c): a class
/// that scoops (`shïp` `Flags3` 0x0002, class +0x9ec, copied from the
/// record's +0x726 @0x7a204-0x7a20b; [`SCOOPS`])
/// [`SHORT_STAY_BASE`] + `Rand(`[`SHORT_STAY_SPAN`]`)`, any other
/// [`STAY_BASE`] + `Rand(`[`STAY_SPAN`]`)`, rolled on `chance`. Its
/// target, provocation, trigger and pick are left as they are, and its
/// slot stays active (+0xb8 is not cleared). The wait is the original's
/// under either reading of `npc_landing`.
pub fn stay(npc: &mut Npc, stellar: StellarId, chance: &mut (impl Chance + ?Sized)) {
    npc.state.velocity = Vec2::ZERO;
    npc.speed = 0.0;
    npc.goal = Goal::Idle;
    npc.visited = Some(stellar);
    npc.travel_set = true;
    npc.ai_wait = if npc.miner & SCOOPS != 0 {
        SHORT_STAY_BASE + chance.below(SHORT_STAY_SPAN)
    } else {
        STAY_BASE + chance.below(STAY_SPAN)
    };
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::flight::ShipState;
    use crate::stats::ShipStats;
    use nova_sim_base::fixture::{Draws, FAST};
    use nova_sim_catalog::fixture::planet;

    /// Turning 8 a tick: the box's 40.
    const TURN_8: Handling = Handling {
        max_speed: 8.0,
        accel: 0.2,
        turn_rate: 8.0,
    };

    /// Turning 7 a tick: a box of 48.
    const TURN_7: Handling = Handling {
        turn_rate: 7.0,
        ..TURN_8
    };

    /// Stellar 140 at (1000, -500), 100 x 100.
    fn site() -> LandingSite {
        planet(140, 1000.0, -500.0)
    }

    /// A ship landing on 140, `(dx, dy)` off its centre, moving `(vx,
    /// vy)`, heading 90.
    fn lander((dx, dy): (f32, f32), (vx, vy): (f32, f32)) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
        npc.goal = Goal::Land(StellarId(140));
        npc.state = ShipState {
            position: site().position + Vec2::new(dx, dy),
            velocity: Vec2::new(vx, vy),
            heading: 90.0,
        };
        npc
    }

    /// [`lander`], folded, of a folding class.
    fn folded(off: (f32, f32), v: (f32, f32)) -> Npc {
        let mut npc = lander(off, v);
        npc.hull.frames.flags = crate::combat::ship_frames::FOLDS;
        npc.hull.frames.base_sets = 6;
        npc.armament.fold_mut().set = 5;
        npc
    }

    /// Flies `npc` a tick on `handling`, as `inertia` says.
    fn flown_as(npc: &mut Npc, handling: &Handling, inertia: Inertia) -> Outcome {
        fly(
            npc,
            &site(),
            handling,
            FlightRules::default(),
            inertia,
            &mut Draws::of(&[]),
        )
    }

    /// Flies `npc` a tick on `handling`, inertial.
    fn flown(npc: &mut Npc, handling: &Handling) -> Outcome {
        flown_as(npc, handling, Inertia::Inertial)
    }

    /// `npc`'s twin flown a tick of sub 2's approach on `handling`.
    fn approached(npc: &Npc, handling: &Handling) -> Npc {
        let mut twin = npc.clone();
        let (controls, capped) = approach_flight::toward(&twin.state, site().position, handling);
        held_burst::step(
            &mut twin,
            &capped,
            controls,
            FlightRules::default(),
            Inertia::Inertial,
            Word::Writes,
        );
        twin
    }

    /// `npc`'s twin cut × 0.98 in doubles, then flown a tick of sub 1 on
    /// `handling`, as `inertia` says.
    fn braked(npc: &Npc, handling: &Handling, inertia: Inertia) -> Npc {
        let mut twin = npc.clone();
        let cut = |axis: f32| (f64::from(axis) * 0.98) as f32;
        twin.state.velocity = Vec2::new(cut(twin.state.velocity.x), cut(twin.state.velocity.y));
        twin.speed = cut(twin.speed);
        leave_flight::fly(
            &mut twin,
            Leg::Brake,
            handling,
            FlightRules::default(),
            inertia,
        );
        twin
    }

    #[test]
    fn the_drag_is_pinned() {
        assert_eq!(LANDING_DRAG, 0.98);
    }

    #[test]
    fn at_rest_in_the_box_of_a_turn_of_8_or_more_it_lands() {
        for handling in [
            TURN_8,
            Handling {
                turn_rate: 12.0,
                ..TURN_8
            },
        ] {
            for off in [(40.9, 40.9), (-40.9, -40.9), (40.9, -40.9), (0.0, 0.0)] {
                let mut npc = lander(off, (0.0, 0.0));
                let before = npc.state;
                assert_eq!(
                    flown(&mut npc, &handling),
                    Outcome::Landed(StellarId(140)),
                    "{off:?}"
                );
                assert_eq!(npc.state, before, "it lands where it is");
            }
        }
    }

    #[test]
    fn just_outside_the_box_on_either_axis_it_flies_sub_2s_approach() {
        for off in [(41.0, 0.0), (-41.0, 0.0), (0.0, 41.0), (0.0, -41.0)] {
            let mut npc = lander(off, (0.0, 0.0));
            let twin = approached(&npc, &TURN_8);
            assert_eq!(flown(&mut npc, &TURN_8), Outcome::Flying, "{off:?}");
            assert_eq!(npc, twin, "{off:?}");
        }
        // Far out, where sub 2 flies at its top speed.
        let mut far = lander((-3000.0, 0.0), (0.0, 0.0));
        let twin = approached(&far, &TURN_8);
        assert_eq!(flown(&mut far, &TURN_8), Outcome::Flying);
        assert_eq!(far, twin);
        assert!(far.state.velocity.x > 0.0, "thrusting at it");
    }

    #[test]
    fn sub_2s_approach_drops_an_owed_gate_burst() {
        let mut npc = lander((-3000.0, 0.0), (0.0, 0.0));
        npc.owed_burst = Some(-30.0);
        flown(&mut npc, &TURN_8);
        assert_eq!(npc.owed_burst, None, "sub 2 writes +0x34");
    }

    #[test]
    fn turning_7_it_lands_48_off_and_not_49() {
        let mut inside = lander((48.0, -48.0), (0.0, 0.0));
        assert_eq!(flown(&mut inside, &TURN_7), Outcome::Landed(StellarId(140)));
        for off in [(49.0, 0.0), (0.0, 49.0)] {
            let mut outside = lander(off, (0.0, 0.0));
            let twin = approached(&outside, &TURN_7);
            assert_eq!(flown(&mut outside, &TURN_7), Outcome::Flying);
            assert_eq!(outside, twin, "{off:?}");
        }
        // The same 48 off is outside a turn of 8's box.
        let mut turn_8 = lander((48.0, 0.0), (0.0, 0.0));
        assert_eq!(flown(&mut turn_8, &TURN_8), Outcome::Flying);
    }

    #[test]
    fn inside_below_0_35_a_tick_on_each_axis_it_lands() {
        for v in [(0.34, -0.34), (0.35, 0.0), (0.0, -0.35), (-0.2, 0.3)] {
            let mut npc = lander((10.0, -10.0), v);
            assert_eq!(
                flown(&mut npc, &TURN_8),
                Outcome::Landed(StellarId(140)),
                "{v:?}: 0.35_f32 is below the double 0.35"
            );
        }
        let mut no_speed = lander((0.0, 0.0), (0.0, f32::NAN));
        assert_eq!(
            flown(&mut no_speed, &TURN_8),
            Outcome::Landed(StellarId(140)),
            "a y that is not a number lands"
        );
    }

    #[test]
    fn inside_at_0_35_or_more_on_an_axis_it_cuts_its_motion_by_0_98_and_brakes_by_sub_1() {
        for v in [(0.5, 0.0), (0.0, 0.5), (-0.36, 0.1), (2.0, -3.0)] {
            let mut npc = lander((10.0, 20.0), v);
            npc.speed = 2.5;
            let twin = braked(&npc, &TURN_8, Inertia::Inertial);
            assert_eq!(flown(&mut npc, &TURN_8), Outcome::Flying, "{v:?}");
            assert_eq!(npc.state, twin.state, "{v:?}");
            assert_eq!(npc.speed, (f64::from(2.5_f32) * 0.98) as f32, "{v:?}");
            assert_eq!(npc, twin, "{v:?}");
        }
    }

    #[test]
    fn a_brake_that_cuts_it_below_0_35_settles_by_sub_1_that_tick() {
        // 0.355 × 0.98 is below 0.35, so sub 1 reads it at rest and settles
        // it by 0.95 with no thrust.
        let mut npc = lander((0.0, 0.0), (0.355, 0.0));
        let twin = braked(&npc, &TURN_8, Inertia::Inertial);
        assert_eq!(flown(&mut npc, &TURN_8), Outcome::Flying);
        assert_eq!(npc, twin);
        let settled = (f64::from((f64::from(0.355_f32) * 0.98) as f32) * 0.95) as f32;
        assert_eq!(npc.state.velocity.x, settled);
    }

    #[test]
    fn an_inertialess_ship_inside_and_moving_cuts_its_speed_then_brakes_inertialess() {
        let mut npc = lander((0.0, 0.0), (1.0, 0.0));
        npc.speed = 1.0;
        let inertia = Inertia::Inertialess { speed: 1.0 };
        let twin = braked(&npc, &TURN_8, inertia);
        assert_eq!(flown_as(&mut npc, &TURN_8, inertia), Outcome::Flying);
        assert_eq!(npc, twin);
        assert!(npc.speed < 0.98, "{}", npc.speed);
    }

    #[test]
    fn a_folding_ship_unfolds_just_inside_the_box_on_both_axes_and_not_outside_either() {
        for (handling, edge) in [(TURN_8, 40.9), (TURN_7, 48.9)] {
            for (off, v) in [((edge, edge), (0.0, 0.0)), ((-edge, edge), (1.0, 0.0))] {
                let mut inside = folded(off, v);
                flown(&mut inside, &handling);
                assert_eq!(inside.armament.fold().dir, -1, "{edge} {off:?} {v:?}");
            }
            let out = edge.ceil();
            for off in [(out, 0.0), (0.0, -out)] {
                let mut outside = folded(off, (0.0, 0.0));
                flown(&mut outside, &handling);
                assert_eq!(outside.armament.fold().dir, 0, "{edge} {off:?}");
            }
        }
    }

    #[test]
    fn a_ship_of_no_fold_or_folded_open_does_not_unfold() {
        let mut plain = lander((0.0, 0.0), (0.0, 0.0));
        plain.armament.fold_mut().set = 5;
        flown(&mut plain, &TURN_8);
        assert_eq!(plain.armament.fold().dir, 0, "not of a folding class");
        let mut open = folded((0.0, 0.0), (0.0, 0.0));
        open.armament.fold_mut().set = 0;
        flown(&mut open, &TURN_8);
        assert_eq!(open.armament.fold().dir, 0, "set 0");
    }

    #[test]
    fn only_a_mission_ship_by_the_other_reading_leaves_as_it_lands() {
        let plain = lander((0.0, 0.0), (0.0, 0.0));
        let marked = Npc {
            mission: Some(crate::traffic::npc::MissionMark {
                slot: 0,
                behav: crate::traffic::ShipBehav::AttackPlayer,
                formation: None,
                order: None,
            }),
            ..plain.clone()
        };
        let aux = Npc {
            aux: Some(2),
            ..plain.clone()
        };
        assert!(leaves(&marked, RuleSource::Bible));
        assert!(!leaves(&marked, RuleSource::Engine));
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            assert!(!leaves(&plain, reading), "{reading:?}");
            assert!(!leaves(&aux, reading), "{reading:?}: an aux ship");
        }
    }

    #[test]
    fn the_stays_are_pinned() {
        assert_eq!((SHORT_STAY_BASE, SHORT_STAY_SPAN), (100, 75));
        assert_eq!((STAY_BASE, STAY_SPAN), (300, 200));
    }

    /// A ship of miner bits `miner`, moving, its goal, target, provocation,
    /// trigger and pick set, staying on 140 as its draws below 75 and 200
    /// give `drawn`: the ship, and what was asked of the dice.
    fn stayed(miner: u16, drawn: u32) -> (Npc, Vec<u32>) {
        let mut npc = lander((10.0, 0.0), (0.2, -0.1));
        npc.miner = miner;
        npc.speed = 0.3;
        npc.target = Some(crate::combat::ShipRef::Player);
        npc.provoked = 7.0;
        npc.trigger.primary = true;
        npc.pick = Some(crate::catalog::WeaponId(130));
        let mut chance = Draws::of(&[]).keyed(75, drawn.min(74)).keyed(200, drawn);
        stay(&mut npc, StellarId(140), &mut chance);
        (npc, chance.asked)
    }

    #[test]
    fn a_landed_ship_stops_takes_goal_0_and_marks_the_stellar_visited() {
        let (npc, _) = stayed(0, 0);
        assert_eq!(npc.state.velocity, Vec2::ZERO);
        assert_eq!(npc.speed, 0.0);
        assert_eq!(npc.goal, Goal::Idle);
        assert_eq!(npc.visited, Some(StellarId(140)), "+0x92 @0x8db00");
        assert!(npc.travel_set, "+0x6c keeps the stellar");
        assert_eq!(npc.state.position, site().position + Vec2::new(10.0, 0.0));
        assert_eq!(npc.target, Some(crate::combat::ShipRef::Player), "kept");
        assert_eq!(npc.provoked, 7.0, "kept");
        assert!(npc.trigger.primary, "kept");
        assert_eq!(npc.pick, Some(crate::catalog::WeaponId(130)), "kept");
    }

    #[test]
    fn a_ship_that_scoops_stays_100_to_174_ticks_and_any_other_300_to_499() {
        for (drawn, short, long) in [(0, 100, 300), (74, 174, 374), (199, 174, 499)] {
            let (scooper, asked) = stayed(crate::ai::miner::SCOOPS, drawn);
            assert_eq!(scooper.ai_wait, short, "{drawn}");
            assert_eq!(asked, [75], "Rand(75)");
            let (other, asked) = stayed(0, drawn);
            assert_eq!(other.ai_wait, long, "{drawn}");
            assert_eq!(asked, [200], "Rand(200)");
            let both = crate::ai::miner::SCOOPS | crate::ai::miner::MINES;
            assert_eq!(stayed(both, drawn).0.ai_wait, short, "{drawn}");
        }
        let (miner, asked) = stayed(crate::ai::miner::MINES, 0);
        assert_eq!(miner.ai_wait, 300, "Flags3 0x0001 alone stays long");
        assert_eq!(asked, [200]);
    }

    #[test]
    fn a_landed_ship_waits_holding_its_guns() {
        let (npc, _) = stayed(0, 0);
        assert!(npc.wait_running());
        let gated = crate::ai::fire::gated(&npc);
        assert!(!gated.primary, "its guns held");
        assert_eq!(gated.secondary, None);
    }

    #[test]
    fn landing_in_its_box_it_stays() {
        let mut npc = lander((0.0, 0.0), (0.1, 0.0));
        let mut chance = Draws::of(&[]).keyed(200, 17);
        let outcome = fly(
            &mut npc,
            &site(),
            &TURN_8,
            FlightRules::default(),
            Inertia::Inertial,
            &mut chance,
        );
        assert_eq!(outcome, Outcome::Landed(StellarId(140)));
        assert_eq!(npc.ai_wait, 317);
        assert_eq!(npc.goal, Goal::Idle);
        assert_eq!(npc.visited, Some(StellarId(140)));
        assert_eq!(npc.state.velocity, Vec2::ZERO);
        let mut flying = lander((0.0, 0.0), (1.0, 0.0));
        let mut chance = Draws::of(&[]);
        flown(&mut flying, &TURN_8);
        fly(
            &mut flying,
            &site(),
            &TURN_8,
            FlightRules::default(),
            Inertia::Inertial,
            &mut chance,
        );
        assert!(chance.asked.is_empty(), "no stay while it brakes");
        assert_eq!(flying.ai_wait, 0);
    }
}
