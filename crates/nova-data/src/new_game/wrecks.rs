//! A ship's `wreck` as its `shïp`'s `DeathDelay`, `Explode1`, `Explode2`
//! and `PodCount` ([`apply`]), through the `base` chain
//! ([`Resolved::wreck`]: each word the nearest given).
//!
//! - `break_up` is its `DeathDelay`: its seconds at
//!   [`TICKS_PER_SECOND`], rounded, held within [`MIN_BREAK_UP_TICKS`] and
//!   `i16::MAX`; left out, [`DEFAULT_BREAK_UP`]. The engine counts the
//!   player's break-up three times as long, and a dying NPC's crew pods
//!   leave from half its `DeathDelay` (`nova_sim::npc_escape`).
//! - `explosion` is its `Explode2`, its final explosion: `small`
//!   [`SMALL_EXPLOSION`] and `large` [`LARGE_EXPLOSION`], with none of the
//!   +1000 extras; left out, small.
//! - Its `Explode1`, the explosions it throws off while it breaks up, is
//!   always [`SMALL_EXPLOSION`]: the format has no word for them.
//! - `pods` is its `PodCount`, held within `i16`; left out, 0.
//!
//! A ship that names no `wreck` anywhere in its chain dies by the default:
//! a break-up of [`DEFAULT_BREAK_UP`] (30 ticks, so the player's is 90),
//! the small explosion and no pods.
//!
//! The engine reads an `Explode*` of 0-63 as `bööm` 128 + n, a record the
//! new game doesn't author. When `game.md` names an `explosion` sound,
//! [`super::sounds::booms`] supplies 128 and 129, so the explosions are
//! heard; they aren't seen until the view has a look for them (see the
//! module docs' "Not mapped yet").

use nova_galaxy_format::fleet::{Explosion, Resolved};

use super::outfits::TICKS_PER_SECOND;
use crate::records::ship::Ship;

/// The `break_up` of a ship that names none, in seconds.
pub const DEFAULT_BREAK_UP: f64 = 1.0;

/// The least `DeathDelay` a `break_up` maps to, in ticks. A `DeathDelay`
/// of 0, or 1 for an NPC, never explodes under the engine's reading of
/// the `breakup_stall` rule, which a ruleset or a setting can select: the
/// death timer stalls above the final explosion's 2. Two ticks is the
/// least that explodes under both readings.
pub const MIN_BREAK_UP_TICKS: i16 = 2;

/// The `Explode*` of the small explosion: `bööm` 129, the engine's small
/// one.
pub const SMALL_EXPLOSION: i16 = 1;

/// The `Explode*` of the large explosion: `bööm` 128, the engine's large
/// one.
pub const LARGE_EXPLOSION: i16 = 0;

/// Sets `ship`'s `DeathDelay`, `Explode1`, `Explode2` and `PodCount` from
/// `resolved`'s wreck (see the module docs).
pub(super) fn apply(ship: &mut Ship, resolved: &Resolved) {
    let wreck = resolved.wreck;
    ship.death_delay = break_up_ticks(wreck.break_up.unwrap_or(DEFAULT_BREAK_UP));
    ship.explode1 = SMALL_EXPLOSION;
    ship.explode2 = match wreck.explosion.unwrap_or(Explosion::Small) {
        Explosion::Small => SMALL_EXPLOSION,
        Explosion::Large => LARGE_EXPLOSION,
    };
    ship.pod_count = wreck
        .pods
        .map_or(0, |pods| i16::try_from(pods).unwrap_or(i16::MAX));
}

/// `seconds` as a `DeathDelay`: rounded at [`TICKS_PER_SECOND`], held
/// within [`MIN_BREAK_UP_TICKS`] and `i16::MAX`, a value that isn't a
/// number at the least.
fn break_up_ticks(seconds: f64) -> i16 {
    let ticks = (seconds * TICKS_PER_SECOND).round();
    if ticks >= f64::from(i16::MAX) {
        i16::MAX
    } else if ticks >= f64::from(MIN_BREAK_UP_TICKS) {
        // Within i16 and whole, by the two checks above.
        #[allow(clippy::cast_possible_truncation)]
        let held = ticks as i16;
        held
    } else {
        MIN_BREAK_UP_TICKS
    }
}
