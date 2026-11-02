//! The stellars a slot names: +0x6c holds a stellar's table index or a
//! ship's or a Con's slot, and a reader that takes it for a stellar
//! (`_MinerAI` @0x8b36c-0x8b3ae) has no bounds test. Goal 1 tests -1 alone
//! (@0x8d8c9-0x8d8d1), then reads the record at that index in the stellar
//! table (@0x8d8d7-0x8d8e8), which holds 0x800 records (`NewPtr(0x24c000)`
//! @0x1b7cd-0x1b7d4, 0x498 each). So a slot, at most 63, names the stellar
//! of ID 128 + slot ([`travel::of_slot`](crate::ai::travel::of_slot)), a
//! real record that usually belongs to another system:
//!
//! - goal 1 flies to that record's `xPos` and `yPos` as they stand
//!   (@0x8d90d-0x8d950), and lands there, wherever the system's stellars
//!   are;
//! - a record with `Flags2` 0x3000 hands off to goal 0x14 (@0x8d8ed-0x8d8f5),
//!   which is in range (@0x8db7d) and is entered as any gate is. Stock has
//!   three in reach: 130, 131 and 178, all hypergates.
//!
//! Every stellar value the engine writes comes from the system's own, so
//! only a slot reads outside the system, and the range is fixed at
//! [`ids`]. The traffic flies a goal naming a stellar the system doesn't
//! list among [`World::slot_stellars`] ([`World::flown_among`]): the
//! autopilot's hand-off and landing then work unchanged. A gate there is
//! sized as a gate another system owns, the unloaded master's width
//! ([`TypeWidth::Unloaded`](crate::landing::TypeWidth::Unloaded)), as the
//! owner test comes first (@0xacb9-0xacc1); the session gives it so.
//!
//! Left out: a slot naming a `spöb` that never loaded (a plug-in's gap in
//! IDs 128 to 191), whose record the original reads as `NewPtr` left it,
//! which can't be replicated. The autopilot brakes such a ship. All of
//! stock's 128 to 191 exist.
//!
//! Nothing here plays a sound or draws a random number.

use super::World;
use crate::ai::{Goal, travel};
use crate::catalog::{LandingSite, StellarId};

/// The ship slots a slot stellar can stand for: 0 to 63.
pub const SLOTS: u32 = 64;

/// The stellars a slot can name, IDs 128 to 191, in order.
pub fn ids() -> impl Iterator<Item = StellarId> {
    (0..SLOTS).map(travel::of_slot)
}

impl<'a> World<'a> {
    /// The stellars `goal` is flown among: [`World::slot_stellars`] when
    /// it names a stellar the system doesn't list ([`World::listed`]),
    /// else the system's listing.
    #[must_use]
    pub fn flown_among(&self, goal: Goal) -> &'a [LandingSite] {
        let named = match goal {
            Goal::Land(stellar) | Goal::EnterGate(stellar) => Some(stellar),
            _ => None,
        };
        match named {
            Some(stellar) if !self.listed.iter().any(|site| site.id == stellar) => {
                self.slot_stellars
            }
            _ => self.listed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_names_the_stellars_of_ids_128_to_191() {
        let all: Vec<StellarId> = ids().collect();
        assert_eq!(all.len(), 64);
        assert_eq!(all.first(), Some(&StellarId(128)));
        assert_eq!(all.last(), Some(&StellarId(191)));
    }
}
