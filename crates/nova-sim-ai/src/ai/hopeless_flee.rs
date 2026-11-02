//! The first frame of a trader's hopeless flee, as `_HighLevelAIHandler`
//! writes it in the `EV Nova` executable: goal 4's or goal 0xd's high
//! level turning a chase into a flight ([`HopelessFlee`]).
//!
//! - **Goal 4's hopeless chase** (@0x8e5f8-0x8e637): for an AI type of 2
//!   or below it writes goal 3 with sub 5 (@0x8e622-0x8e62e), then jumps to
//!   the state-4 tail (@0x8e6e5), which tests `_IsDisabled(self)`
//!   (@0x8e6eb), calls `_AILaunchFighter` (@0x8e6fa: its target in the
//!   system and its slot active, @0x8137e-0x813b2, then a bay of guidance
//!   99 with ammunition, @0x813e8, so a trader with no bay launches
//!   nothing) and `_AICallForReinforcements(ship, 0)` (@0x8e70d), with its
//!   own gates ([`reinforcements`](super::reinforcements)).
//! - **Goal 0xd's hopeless chase** writes the same goal 3 and sub 5
//!   (@0x8eaef-0x8eafb), then jumps to @0x8eb6b, past every call.
//!
//! Goal 3's block (@0x8df75-0x8e0dc) comes before both, so on that frame
//! no flee drop runs (@0x8e077-0x8e0c3), no goal-3 call is made
//! (@0x8e0d7) and no sub is chosen by reach: sub 5, the low level's flee
//! close (@0x85c62-0x85e17, [`Manoeuvre::Flee`](super::fire::Manoeuvre::Flee)),
//! flies whatever the reach. It wants the bearing away from the attacker,
//! runs its turret pass (`_AIFireTurret` @0x85d72), and rushes within 165
//! with an afterburner (@0x85d77-0x85e0b). On the next frame goal 3's
//! block sets the sub by reach: sub 5 within 250 (@0x8e017-0x8e06b), subs
//! 1, 3 and 4 beyond (@0x8e0e1-0x8e21a), the flight the engine already
//! flies. The think's calls read the think's goal, goal 4, so a wimpy
//! trader makes no distress call that frame.
//!
//! The decision carries which high level wrote it
//! ([`Decision::hopeless_flee`](super::Decision::hopeless_flee)), and the
//! traffic keeps it on the ship until its next decision
//! ([`Npc::fled_hopeless`]), for the pursuit's start
//! ([`pursuit::high_level`](super::pursuit::high_level)) and the fighter
//! launch ([`fire::trigger`](super::fire::trigger)). `NovaAi` skips goal
//! 3's flee drop for it and calls as goal 4's tail for [`HopelessFlee::Attack`]
//! and not at all for [`HopelessFlee::Plunder`]
//! ([`reinforcements::with_high_level_call`](super::reinforcements::with_high_level_call)).
//!
//! The other ships that reach the state-4 block
//! ([`attack_drop::attack_block`](super::attack_drop::attack_block)) never
//! flee there: the interceptor's goal 4 ([`held_attack`](super::held_attack)),
//! the warship's [`hunt`](super::warship::hunt), the interceptor's
//! re-pick and the pirate's attack fly ships of AI type 3 or 4, which
//! snipe a hopeless chase (@0x8e60a). A bribe writes AI type 1
//! (`_DoCommDialog` @0x961f8), and the bribed ship thinks as the wimpy
//! trader here too ([`Npc::take_bribe`]), whose kept attack carries the
//! marker. Nothing here plays a sound or draws a random number.

use crate::ai::Goal;
use crate::traffic::npc::Npc;

/// Which high level turned a chase into a flight this frame (see the
/// module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HopelessFlee {
    /// Goal 4's (@0x8e622-0x8e62e), which goes on to the state-4 tail.
    Attack,
    /// Goal 0xd's (@0x8eaef-0x8eafb), which makes no call.
    Plunder,
}

/// The ship `npc` flees on the first frame of a flight goal 4's high level
/// wrote ([`HopelessFlee::Attack`]), which the state-4 tail still serves:
/// its fighter launch and its call read it as goal 4's target.
#[must_use]
pub fn attack_fled(npc: &Npc) -> Option<crate::combat::ShipRef> {
    match npc.goal {
        Goal::Flee(ship) if npc.fled_hopeless == Some(HopelessFlee::Attack) => Some(ship),
        _ => None,
    }
}
