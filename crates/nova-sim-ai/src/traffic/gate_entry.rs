//! NPCs entering a hypergate or wormhole and leaving the system through
//! it: `_HighLevelAIHandler` state 20, goal 0x14 (@0x8db5f-0x8dd5e in the
//! `EV Nova` executable), and how the ship then goes.
//!
//! Evidence: disassembly of the Mac OS X `EV Nova` (i386, with symbols).
//!
//! - **The hand-off** (state 1, @0x8d8b8-0x8d8fe). A ship whose goal is 1,
//!   to land, with a travel stellar (+0x6c) whose `Flags2` has 0x3000
//!   takes goal 0x14 and falls straight into the 0x14 block in the same
//!   pass (@0x8d8fe → 0x8db5f): a landing goal at a gate is gate entry
//!   from its first tick
//!   ([`Goal::EnterGate`](crate::ai::Goal::EnterGate)).
//! - **The guard** (@0x8db62-0x8db81): goal 0x14, with +0x6c in
//!   0..=0x7ff. It zeroes its hyperspace state, +0x50 (@0x8db8f).
//! - **The box.** dx = trunc(gate `xPos` - ship x), and dy likewise, each
//!   truncated to whole pixels (`cvttss2si`, @0x8db9e-0x8dbda). Its reach
//!   is `_GetStellarSpriteXSize(gate)` / 4, rounded toward zero
//!   (@0x8dbdf-0x8dbfa, [`reach`]): the `Type` frame's width, 150 with the
//!   master not loaded or the gate's owner not the player's system
//!   (@0xacb9-0xacc1), and 32 with its frame index out of range
//!   ([`stellar_width`](crate::landing::stellar_width)).
//! - **Outside**, |dx| > reach or |dy| > reach (@0x8dc1b-0x8dc55): it
//!   approaches (sub-goal 2 @0x8dc45, `approach`): `_LowLevelAIHandler`
//!   flies it as a landing ship's approach
//!   ([`approach_flight`](crate::traffic::approach_flight)), turning to the
//!   gate's whole-degree bearing, thrusting within its turn and 5 degrees,
//!   capped at a quarter of its top speed within 500 on both axes (so on
//!   the whole run in, the reach being far inside 500), and never braking.
//!   By the other reading of
//!   [`RuleKey::NpcLanding`](crate::RuleKey::NpcLanding) it flies the
//!   port's earlier approach, braking by its stopping distance. Its wait
//!   (+0x4c) is -1.0 (@0x8dc4e): it has no wait at all, not one counting
//!   down.
//! - **Inside**, both within reach, inclusive ([`inside`],
//!   @0x8dc5a-0x8dc94): its velocity is 0 (the speed, +0x48, is left as
//!   it was), its sub-goal 0x17, and its wait [`ENTRY_TICKS`] when below
//!   0 or above 16 (0xdd048).
//! - **The release** (@0x8dc94-0x8dd58): every other ship, active, whose
//!   leader (+0x9a) is this one has its leader cleared, takes this ship's
//!   AI type capped at 3 ([`released_ai_type`]), clears the stellar it
//!   defends (+0x8c) and its target (+0x70), sets its provocation (+0x96)
//!   to -1 ([`RELEASED_PROVOCATION`]),
//!   and heads for this gate with goal 0x14 and sub-goal 0. Disabled
//!   followers are not skipped. While +0x4c is above 0
//!   `_HighLevelAIHandler` returns at once (@0x8d4b9-0x8d4d2), so the
//!   release runs once, on the tick the ship reaches the box.
//! - **Leaving.** `_HandleShip` holds a ship still while +0x4c is above 0
//!   and counts it down by one a tick (@0x34a54, @0x3501f-0x35048).
//!   `_HandleShipDisplay` (@0x2bf94-0x2c088), for an NPC of sub-goal 0x17
//!   (`_AIShipIsEnteringHypergate` @0x89502, which tests the sub-goal
//!   alone), draws it at clamp(trunc(-2 × +0x4c + 32), 0, 32) 32nds
//!   (0xdd650, 0xdd654): a fade-out, the emerge's fade in reverse
//!   ([`enter_level`]). Once +0x4c is below 1.0 (0xdd068) it frees the
//!   slot (+0xb8 = 0, +0x9a = -1, @0x2c066-0x2c07f): the ship is gone,
//!   with no jump flash. On the tick it reaches the box it counts 16 to
//!   15 and is drawn at 30/32; on the 16th tick it counts to 0 and goes
//!   before it is drawn, so it is drawn for 15 frames.
//! - **No count.** The slot is freed with no call: a mission ship's
//!   chase-off count and ships left change only at the end of a warp out
//!   (`_LowLevelAIHandler` @0x85960-0x859aa), so a special ship that
//!   leaves through a gate counts nothing and comes back at the next
//!   setup, as [`RuleKey::GateExitCounts`](crate::RuleKey::GateExitCounts)
//!   says.
//! - **Hit while it fades**, `_DamageShip` can call
//!   `_AIMakeShipAttackPlayer` (@0x3be69): for goal 0x14 it sets goal 4,
//!   +0x6c = -1 and the target, but leaves the sub-goal (@0x89d0b-0x89d4d).
//!   The fade reads the sub-goal, so it runs on and the ship still
//!   leaves. `_ShipVisibleToShip` tests goal 0x15 only, so a ship
//!   entering a gate is seen, targeted and fought, and with +0x50 at 0 it
//!   answers hails.
//!
//! None of this calls `PlaySound` or `_beepSnd`: state 1's hand-off and
//! state 20 call `_GetStellarSpriteXSize` and `_FloatAbs`; sub 2 calls
//! `_XYShortToFloat`, `_CalcInterceptAngle`, `_AngularDifference`,
//! `_ShipTurnRate`, `_FloatAbs`, `_ShipAccelRate` and `_ShipMaxSpeed`;
//! `_HandleShipDisplay`'s entering block calls only the predicate; and
//! `_HandleShip`'s countdown calls nothing. It raises no
//! [`SimSound`](crate::SimSound).
//!
//! Left out, each unreachable here or of no visible effect:
//!
//! - **+0x6c out of 0..=0x7ff** (@0x8db70-0x8db81) does nothing. The
//!   engine's goal always names a stellar; one not shown brakes.
//! - **+0x50 = 0** (@0x8db8f). The engine keeps no hyperspace countdown
//!   for a ship not jumping out.
//! - **+0x4c kept when 0..=16 on reaching the box** (@0x8dc73-0x8dc88).
//!   A ship that comes in from the approach always has -1 (@0x8dc4e), and
//!   so starts at 16. Only a ship whose AI wait has just run out and
//!   which first thinks inside the box keeps 0, and leaves at once,
//!   unfaded. The engine has no AI wait outside
//!   [`Mode::Coasting`](crate::traffic::npc::Mode::Coasting), which
//!   decides nothing, so every entry starts at 16.
//! - **The 16-bit wraps** of dx and dy (`cwtl` @0x8dbb3, @0x8dbd5) and of
//!   the reach (@0x8dbfa): wider numbers are kept.
//! - **The release's +0x8c = -1** (@0x8dd08), never reached: the
//!   ships released are led by this one, and a defence ship has no leader (its spawn writes -1) and a capture or a pirate's take clears its stellar.
//! - **Ships 1-63 only** (@0x8dc97-0x8dd58). The engine takes every NPC.
//! - **The 16-bit blit** (+0xb0/+0xb4, @0x2c010-0x2c05c). It is drawn as
//!   an alpha fade, as the emerge's is.

use std::collections::BTreeMap;

use crate::catalog::{LandingSite, StellarId};
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::landing::{TypeWidth, stellar_width};
use crate::traffic::approach_flight;
use crate::traffic::emerge::FULL_LEVEL;
use crate::traffic::held_burst::{Flight, Word};
use crate::traffic::npc::AiType;

/// The wait a ship entering a gate starts at, as it reaches the box: 16.0
/// (0xdd048).
pub const ENTRY_TICKS: u32 = 16;

/// How near a gate `width` pixels wide a ship must be, on each axis, to
/// enter it: a quarter of the width, rounded toward zero (@0x8dbdf-0x8dbfa).
#[must_use]
pub fn reach(width: u32) -> i32 {
    i32::try_from(width / 4).unwrap_or(i32::MAX)
}

/// The reach of `site`, its `Type` master giving the width `type_widths`
/// holds for it, or, with none held, the one a loaded master gives: the
/// frame's, or the out-of-range width for a `spïn` of no frame count.
#[must_use]
pub fn gate_reach(site: &LandingSite, type_widths: &BTreeMap<StellarId, TypeWidth>) -> i32 {
    let loaded = if site.type_counts_frames {
        TypeWidth::Frame
    } else {
        TypeWidth::OutOfRange
    };
    let width = type_widths.get(&site.id).copied().unwrap_or(loaded);
    reach(stellar_width(site, width))
}

/// State 20's flight outside the gate's box (@0x8dc42-0x8dc55): sub 2,
/// [`approach_flight::toward`] the gate at `gate` for a ship at `state` on
/// `handling`, whose thrust writes +0x34 ([`Word::Writes`]) and which
/// flies by the handling it hands back, capped near the gate as it
/// thrusts. It never brakes: inside the box the velocity is zeroed.
pub(crate) fn approach(state: &mut ShipState, gate: Vec2, handling: &Handling) -> Flight {
    let (controls, flown) = approach_flight::toward(state, gate, handling);
    Flight {
        controls,
        word: Word::Writes,
        flown: Some(flown),
    }
}

/// Whether a ship at `ship` is inside the box of the gate at `gate`, of
/// `reach`: its offset truncated to whole pixels on each axis, within
/// `reach` inclusive (@0x8db9e-0x8dc5a).
#[must_use]
pub fn inside(gate: Vec2, ship: Vec2, reach: i32) -> bool {
    let within = |off: f32| f64::from(off).trunc().abs() <= f64::from(reach);
    within(gate.x - ship.x) && within(gate.y - ship.y)
}

/// How opaque a ship entering a gate with `ticks_left` of its wait is
/// drawn, in 32nds ([`FULL_LEVEL`]): 2 × `ticks_left`, at most full
/// (`_HandleShipDisplay` @0x2bf94-0x2c088).
#[must_use]
pub fn enter_level(ticks_left: u32) -> u8 {
    let level = ticks_left.saturating_mul(2).min(u32::from(FULL_LEVEL));
    u8::try_from(level).unwrap_or(FULL_LEVEL)
}

/// The provocation a ship takes as its lead enters a gate and releases
/// it: +0x96 = -1 (@0x8dd14), so the first point of damage that hits it
/// leaves it unprovoked (the readers' `cmpw $0` and `jle`).
pub const RELEASED_PROVOCATION: f32 = -1.0;

/// The AI type a ship led by a ship of `lead`'s type takes as its lead
/// enters a gate: the lead's, capped at 3, so an interceptor's followers
/// become warships (@0x8dc94-0x8dd58).
#[must_use]
pub fn released_ai_type(lead: AiType) -> AiType {
    match lead {
        AiType::Interceptor => AiType::Warship,
        kept => kept,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reach_is_a_quarter_of_the_width_rounded_down() {
        assert_eq!(reach(150), 37);
        assert_eq!(reach(32), 8);
        assert_eq!(reach(3), 0);
    }

    #[test]
    fn the_box_holds_within_the_reach_inclusive_on_each_axis_in_whole_pixels() {
        let gate = Vec2::new(100.0, -50.0);
        let off = |x: f32, y: f32| Vec2::new(gate.x + x, gate.y + y);
        assert!(inside(gate, off(37.9, 0.0), 37));
        assert!(inside(gate, off(-37.9, 0.0), 37));
        assert!(inside(gate, off(37.0, -37.0), 37));
        assert!(!inside(gate, off(38.0, 0.0), 37));
        assert!(!inside(gate, off(-38.0, 0.0), 37));
        assert!(!inside(gate, off(0.0, 38.0), 37));
        assert!(!inside(gate, off(0.0, -38.0), 37));
        assert!(inside(gate, gate, 0));
    }

    #[test]
    fn a_ship_entering_fades_out_by_two_32nds_a_tick() {
        assert_eq!(enter_level(16), 32);
        assert_eq!(enter_level(17), 32);
        assert_eq!(enter_level(15), 30);
        assert_eq!(enter_level(1), 2);
        assert_eq!(enter_level(0), 0);
        assert_eq!(enter_level(u32::MAX), FULL_LEVEL);
        assert_eq!(ENTRY_TICKS, 16);
    }

    #[test]
    fn a_follower_released_at_a_gate_takes_provocation_minus_1_so_a_1_point_hit_leaves_it_unprovoked()
     {
        use crate::ai::fixture::{PIRATES, TRADERS, around, govts, n, ship, sites};
        let (sites, govts) = (sites(), govts(0, 0));
        let mut follower = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        follower.target = Some(n(2));
        let npcs = [
            follower.clone(),
            ship(2, PIRATES, AiType::Warship, 50.0, 0.0),
        ];
        let around = around(&sites, &npcs, &govts, (0.0, -500.0), 0);
        let after = |hit: f32| {
            let mut hit_follower = follower.clone();
            hit_follower.provoked = RELEASED_PROVOCATION + hit;
            crate::ai::provoked_by(&hit_follower, &around)
        };
        assert_eq!(after(1.0), None, "-1 + 1 is 0: `jle` reads it unprovoked");
        assert_eq!(after(2.0), Some(n(2)));
    }

    #[test]
    fn an_interceptors_followers_become_warships_and_the_rest_keep_its_type() {
        assert_eq!(released_ai_type(AiType::Interceptor), AiType::Warship);
        for kept in [AiType::WimpyTrader, AiType::BraveTrader, AiType::Warship] {
            assert_eq!(released_ai_type(kept), kept);
        }
    }
}
