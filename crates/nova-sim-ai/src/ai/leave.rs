//! Leaving by hyperspace: who can jump out, and which leg of the leave a
//! ship flies, as the original's `_AIShipHasFuelForJump`,
//! `_ShipCanExpiditeJumps` and `_HighLevelAIHandler`'s states 2 and 3 have
//! them (addresses in the `EV Nova` executable).
//!
//! - **Who can jump** ([`can_jump`], `_AIShipHasFuelForJump`
//!   @0x7f48f-0x7f500, inlined at every decision site): a ship whose
//!   class's `Fuel` is above 99 ([`Npc::class_fuel`], the raw word, not
//!   the tank its default items give it), that no other ship's tractor
//!   beam holds ([`Npc::tractor`]). Of those, only a ship whose person
//!   starts with zero fuel ([`NpcPerson::zero_fuel`](crate::traffic::npc::NpcPerson::zero_fuel))
//!   needs a jump's fuel ([`JUMP_FUEL`]) in its tank now; any other jumps
//!   with an empty tank. By the other reading of
//!   [`RuleKey::NpcJumpFuel`](crate::RuleKey::NpcJumpFuel), every NPC
//!   needs that fuel now.
//! - **Who expedites** ([`expedites`], `_ShipCanExpiditeJumps`
//!   @0x7872-0x7925): a hull of `shïp` `Flags2` 0x0020 or a default item
//!   of `ModType` 37, the ship's [`fast_jump`](crate::stats::ShipStats::fast_jump).
//! - **The leave** ([`leg`], state 2 @0x8dd99-0x8de57): within
//!   [`LEAVE_REACH_SQ`] of the centre, squared and inclusive, it runs out
//!   ([`Leg::RunOut`], sub 3). Beyond, an expediting ship jumps
//!   ([`Leg::Jump`], sub 4); any other brakes ([`Leg::Brake`], sub 1)
//!   while it is moving 0.35 or more on an axis ([`snipe::moving`], the
//!   same double test), and jumps once it is not. The reach is the same
//!   1000 for every NPC: `_ShipHyperSafeDist` @0x4485 gives its outfits'
//!   `ModType` 23 to the player alone. State 2 tests no fuel: the type
//!   AIs test it before they set it, and `_AIMakeShipLeave` sets it with
//!   no test.
//! - **The flee's far side** ([`flee_leg`], state 3 @0x8e0e1-0x8e21a): a
//!   ship fleeing beyond its attacker's reach leaves as above when it is
//!   beyond 1000 and can jump, and runs out otherwise.
//!
//! The decision and the flight up to sub 4 are here and in
//! [`leave_flight`](crate::traffic::leave_flight); sub 4 itself, the warp
//! out, is [`warp_out`](crate::traffic::warp_out)'s.

use crate::flight::ShipState;
use crate::hyperspace::JUMP_FUEL;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;
use crate::traffic::snipe;

/// How far from the centre a ship must be to leave, squared (1000000.0
/// @0xdda50): the original's dist² test, inclusive (`jbe` @0x8dde5).
pub const LEAVE_REACH_SQ: f32 = 1_000_000.0;

/// The least class `Fuel` that can jump: above 99 (`cmpw $0x63`, `jle`
/// @0x7f4a7-0x7f4ad).
const CLASS_FUEL_FLOOR: i16 = 99;

/// Which leg of the leave a ship flies: the low-level sub-goal the high
/// level sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leg {
    /// Sub 3: it runs out from the centre.
    RunOut,
    /// Sub 1: it brakes.
    Brake,
    /// Sub 4: it jumps.
    Jump,
}

/// Whether `npc` can jump out (`_AIShipHasFuelForJump`, see the module
/// docs), by `reading` of
/// [`RuleKey::NpcJumpFuel`](crate::RuleKey::NpcJumpFuel).
#[must_use]
pub fn can_jump(npc: &Npc, reading: RuleSource) -> bool {
    let untractored = npc
        .tractor
        .is_none_or(|lock| lock.holder == crate::combat::ShipRef::Npc(npc.id));
    let needs_fuel = match reading {
        RuleSource::Engine => npc.person.is_some_and(|person| person.zero_fuel),
        RuleSource::Bible => true,
    };
    npc.class_fuel > CLASS_FUEL_FLOOR
        && untractored
        && (!needs_fuel || npc.reserves.fuel.now >= JUMP_FUEL)
}

/// Whether `npc` jumps without braking first (`_ShipCanExpiditeJumps`).
#[must_use]
pub fn expedites(npc: &Npc) -> bool {
    npc.stats.fast_jump
}

/// The leg a leaving ship (state 2) at `state` flies, expediting as
/// `expedites` says (see the module docs).
#[must_use]
pub fn leg(state: &ShipState, expedites: bool) -> Leg {
    if !beyond_reach(state) {
        Leg::RunOut
    } else if expedites || !snipe::moving(state) {
        Leg::Jump
    } else {
        Leg::Brake
    }
}

/// The leg a ship fleeing beyond its attacker's reach (state 3's far
/// branch) at `state` flies: as [`leg`] beyond 1000 when it `can_jump`,
/// else it runs out.
#[must_use]
pub fn flee_leg(state: &ShipState, expedites: bool, can_jump: bool) -> Leg {
    if can_jump && beyond_reach(state) {
        leg(state, expedites)
    } else {
        Leg::RunOut
    }
}

/// Whether `state` is beyond [`LEAVE_REACH_SQ`]: `_FloatAbs` of the `f32`
/// dist² from the centre above it, a NaN counting as beyond (`jp`
/// @0x8dde3).
fn beyond_reach(state: &ShipState) -> bool {
    let at = state.position;
    let squared = (at.x * at.x + at.y * at.y).abs();
    squared.is_nan() || squared > LEAVE_REACH_SQ
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::combat::ShipRef;
    use crate::combat::impact::TractorLock;
    use crate::fuel::OutfitMod;
    use crate::geometry::Vec2;
    use crate::stats::{FAST_JUMP, ShipStats};
    use crate::traffic::npc::{Npc, NpcId, NpcPerson};
    use nova_sim_base::fixture::FAST;

    /// An NPC of class `Fuel` `class_fuel` holding `now` fuel.
    fn tank(class_fuel: i16, now: f32) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
        npc.class_fuel = class_fuel;
        npc.reserves.fuel.now = now;
        npc
    }

    fn flown_by(mut npc: Npc, zero_fuel: bool) -> Npc {
        npc.person = Some(NpcPerson {
            zero_fuel,
            ..NpcPerson::of(&nova_sim_catalog::fixture::person(151, 128), false)
        });
        npc
    }

    fn at(x: f32, y: f32, vx: f32, vy: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            ..ShipState::default()
        }
    }

    /// The float just above the float 0.35, which widens to at least the
    /// double 0.35.
    fn above_035() -> f32 {
        f32::from_bits(0.35_f32.to_bits() + 1)
    }

    #[test]
    fn the_named_values_are_pinned() {
        assert_eq!(LEAVE_REACH_SQ, 1_000_000.0);
    }

    #[test]
    fn a_class_fuel_above_99_jumps_with_an_empty_tank_and_99_does_not() {
        assert!(can_jump(&tank(100, 0.0), RuleSource::Engine));
        assert!(can_jump(&tank(i16::MAX, 0.0), RuleSource::Engine));
        assert!(!can_jump(&tank(99, 300.0), RuleSource::Engine));
        assert!(!can_jump(&tank(0, 300.0), RuleSource::Engine));
        assert!(!can_jump(&tank(-1, 300.0), RuleSource::Engine));
    }

    #[test]
    fn a_ship_another_holds_in_its_tractor_beam_cannot_jump_and_one_holding_itself_can() {
        let mut held = tank(300, 300.0);
        held.tractor = Some(TractorLock::by(ShipRef::Player));
        assert!(!can_jump(&held, RuleSource::Engine));
        held.tractor = Some(TractorLock::by(ShipRef::Npc(NpcId(2))));
        assert!(!can_jump(&held, RuleSource::Engine));
        held.tractor = Some(TractorLock::by(ShipRef::Npc(held.id)));
        assert!(can_jump(&held, RuleSource::Engine));
        held.tractor = None;
        assert!(can_jump(&held, RuleSource::Engine));
    }

    #[test]
    fn only_a_person_that_starts_with_zero_fuel_needs_a_jumps_fuel_now() {
        assert!(!can_jump(
            &flown_by(tank(300, 99.9), true),
            RuleSource::Engine
        ));
        assert!(can_jump(
            &flown_by(tank(300, 100.0), true),
            RuleSource::Engine
        ));
        assert!(can_jump(
            &flown_by(tank(300, 0.0), false),
            RuleSource::Engine
        ));
        assert!(
            !can_jump(&flown_by(tank(99, 300.0), true), RuleSource::Engine),
            "the class's Fuel comes first"
        );
        let mut held = flown_by(tank(300, 300.0), false);
        held.tractor = Some(TractorLock::by(ShipRef::Player));
        assert!(!can_jump(&held, RuleSource::Engine), "so does the tractor");
    }

    #[test]
    fn by_the_other_reading_every_npc_needs_a_jumps_fuel_now() {
        assert!(!can_jump(&tank(300, 99.0), RuleSource::Bible));
        assert!(can_jump(&tank(300, 100.0), RuleSource::Bible));
        assert!(!can_jump(
            &flown_by(tank(300, 0.0), false),
            RuleSource::Bible
        ));
        assert!(!can_jump(&tank(99, 300.0), RuleSource::Bible));
        let mut held = tank(300, 300.0);
        held.tractor = Some(TractorLock::by(ShipRef::Player));
        assert!(!can_jump(&held, RuleSource::Bible));
    }

    #[test]
    fn a_ship_expedites_by_its_hulls_flags2_0x0020_or_a_fast_jump_default_item() {
        let of = |stats| crate::traffic::fixture::npc(1, stats);
        assert!(!expedites(&of(ShipStats::new(FAST, &[]))));
        let hull = crate::handling::ShipFields {
            flags2: 0x0020,
            ..FAST
        };
        assert!(expedites(&of(ShipStats::new(hull, &[]))));
        let item = |count| OutfitMod {
            mod_type: FAST_JUMP,
            mod_val: 0,
            count,
        };
        assert!(expedites(&of(ShipStats::new(FAST, &[item(1)]))));
        assert!(!expedites(&of(ShipStats::new(FAST, &[item(0)]))));
    }

    #[test]
    fn within_1000_of_the_centre_it_runs_out_and_exactly_1000_is_within() {
        assert_eq!(leg(&at(0.0, -1000.0, 0.0, 0.0), true), Leg::RunOut);
        assert_eq!(leg(&at(600.0, 800.0, 9.0, 9.0), false), Leg::RunOut);
        assert_eq!(
            leg(&at(800.0, -800.0, 0.0, 0.0), false),
            Leg::Jump,
            "1131 out"
        );
        assert_eq!(
            leg(&at(-700.0, 700.0, 0.0, 0.0), false),
            Leg::RunOut,
            "990 out"
        );
        assert_eq!(leg(&at(0.0, 0.0, 0.0, 0.0), false), Leg::RunOut);
        assert_eq!(
            leg(&at(f32::NAN, 0.0, 0.0, 0.0), false),
            Leg::Jump,
            "NaN is beyond"
        );
    }

    #[test]
    fn beyond_1000_an_expediting_ship_jumps_at_full_speed() {
        assert_eq!(leg(&at(0.0, -1000.5, 0.0, -9.0), true), Leg::Jump);
        assert_eq!(leg(&at(1000.5, 0.0, 9.0, 9.0), true), Leg::Jump);
    }

    #[test]
    fn beyond_1000_a_ship_that_does_not_expedite_brakes_while_moving_035_on_an_axis() {
        let past = 1000.5;
        assert_eq!(leg(&at(0.0, past, above_035(), 0.0), false), Leg::Brake);
        assert_eq!(leg(&at(0.0, past, 0.0, -above_035()), false), Leg::Brake);
        assert_eq!(leg(&at(0.0, past, 0.0, -9.0), false), Leg::Brake);
        assert_eq!(leg(&at(0.0, past, 0.35, -0.35), false), Leg::Jump);
        assert_eq!(leg(&at(0.0, past, 0.0, 0.0), false), Leg::Jump);
    }

    #[test]
    fn a_fleeing_ship_beyond_1000_leaves_as_a_leaving_one_when_it_can_jump_and_runs_out_otherwise()
    {
        for state in [
            at(0.0, -1000.5, 0.0, -9.0),
            at(0.0, -1000.5, 0.0, 0.0),
            at(5000.0, 0.0, above_035(), 0.0),
        ] {
            for expediting in [true, false] {
                assert_eq!(
                    flee_leg(&state, expediting, true),
                    leg(&state, expediting),
                    "{state:?} {expediting}"
                );
                assert_eq!(flee_leg(&state, expediting, false), Leg::RunOut);
            }
        }
        let inside = at(0.0, -1000.0, 0.0, 0.0);
        assert_eq!(flee_leg(&inside, true, true), Leg::RunOut);
        assert_eq!(flee_leg(&inside, false, true), Leg::RunOut);
    }
}
