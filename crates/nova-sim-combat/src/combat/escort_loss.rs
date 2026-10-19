//! A ship of the player's fleet leaves it as the hit that leaves it
//! disabled lands, as `_DamageShip`'s escort block has it (rdm
//! weapons-fidelity phase 42).
//!
//! # The original
//!
//! Every damage path of `_DamageShip` reaches @0x3ae58, where
//! `_IsDisabled(ship)` is asked (@0x3ae5b); a ship not disabled goes on to
//! @0x3b1d4. The mission block before it (@0x3ae68-0x3af8b, the session's
//! `raise_disabled`) jumps to the escort block whether or not the ship was
//! disabled before the hit (@0x3ae73, @0x3ae90), so the escort block runs on
//! **any** hit that leaves the ship disabled, not only the one that
//! disabled it.
//!
//! The escort block runs for a ship whose leader +0x9a is 0, the player
//! (@0x3af8f), and that has no mission mark (+0xc8d2 == -1, @0x3af9d):
//!
//! - +0xc8de is set to 0 for AI type 5 (a fighter), else 2 when the hired
//!   flag +0xbb is clear and 1 when it is set (@0x3afab-0x3afce): the
//!   record the session remembers;
//! - its leader +0x9a is set to -1 (@0x3afdd);
//! - AI type 6 with a class `InherentAI` of 2 or less takes its cargo share
//!   (`_DestroyPartialFleetCargo` @0x3b013), and the cargo HUD is redrawn
//!   (@0x3b018-0x3b021);
//! - its AI type +0x88 goes back to its class's `InherentAI`
//!   (@0x3b0c0-0x3b0d7).
//!
//! No sound and no message. `_DamageShip`'s reaction block (@0x3b408-0x3bb74)
//! runs after it, so the reactions read the escort unled, and so does every
//! later test that frame: `_ShotCanHitShip`'s rules 6a, 6f-6i, 9 and 10,
//! the beam filter (@0x30ba0-0x30c05, @0x30d10; `_HandleBeams` lands each
//! beam's hit in its loop, @0x316a1), and a seeking sub-munition's
//! `_FindNearestValidShotTarget` (@0x6a3a).
//!
//! Here the fight clears the leader in place ([`leave_fleet_if_disabled`],
//! from [`Combat::land`](super::Combat)); the record, the AI type and the
//! cargo share are the session's, settled as the phase's hits settle
//! (`nova_sim::Session::lose_escorts`).
//!
//! The crime block before it reads the shooter's leader as each hit lands
//! (@0x3ad54-0x3ad7b): a kill an escort scores counts for the player only
//! while the escort is still led by the player. So the fight notes each
//! loss with where it fell among the strikes ([`EscortLoss`],
//! [`Combat::take_landed`](super::Combat::take_landed)), for the session
//! to credit a kill by the fleet as it stood at the kill.

use super::Fighter;
use super::ShipRef;
use super::hull::DisableRule;

/// A ship of the player's fleet that left it as a hit landed: `ship`,
/// lost as the strike at index `strike` among the strikes taken with it
/// ([`Combat::take_landed`](super::Combat::take_landed)) landed. Every
/// strike before that one found it still the player's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscortLoss {
    /// The ship lost.
    pub ship: ShipRef,
    /// The index of the strike that lost it.
    pub strike: usize,
}

/// Clears `fighter`'s leader when it is a ship of the player's fleet
/// ([`Fighter::fleet_escort`]) that `_IsDisabled` holds for now, as `rule`
/// says ([`Fighter::disabled_now`]): its leader is set to none and its
/// fleet to itself (`_DamageShip` @0x3afdd, see the module docs). Gives
/// whether it left the fleet.
pub(super) fn leave_fleet_if_disabled(fighter: &mut Fighter, rule: &dyn DisableRule) -> bool {
    if !fighter.fleet_escort
        || fighter.leader != Some(ShipRef::Player)
        || !fighter.disabled_now(rule)
    {
        return false;
    }
    fighter.leader = None;
    fighter.fleet = fighter.ship;
    true
}
