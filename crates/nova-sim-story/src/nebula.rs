//! Nebulae: when one is active, which systems it holds, and when an
//! exploration explores it, as the original's `nëbu` table keeps them.
//! The session wires these into the galaxy overlay and the story
//! dispatcher (its `nebulae` module).
//!
//! **The table.** The runtime nebula is a slot of 0x208 bytes. There are
//! 32 slots, for `nëbu` 128-159 ([`FIRST_NEBULA`], [`LAST_NEBULA`]), and
//! every loop over them stops at 0x4100 bytes. `_LoadObjectData` fills
//! them (@0x7cf44-0x7d025): it first zeroes each slot's sizes (+0x4,
//! +0x6) and its explored byte (+0x9), then copies `XPos` to +0x0, `YPos`
//! to +0x2, `XSize` to +0x4, `YSize` to +0x6, `ActiveOn` to +0xa and
//! `OnExplore` to +0x109. A missing record therefore keeps a zero size,
//! so it is never active. The Bible agrees: "Max Nebulae 32".
//!
//! **Activity** ([`active`]). `_PropagateMissionBitEffects` writes +0x8
//! for every slot (@0x998b4-0x998ef): `ActiveOn`, evaluated, when `XSize`
//! and `YSize` are both above 0, compared signed; 0 otherwise. A blank
//! test holds (@0x995ad-0x995b4). Activity is read only by
//! `_ExploreNebulaeFromSystem` (@0x50bc), the map (`_MapDialogUpdate`
//! @0xf951-0xf95f) and `_DoSystemMap` (@0x123ee-0x123f8). Both map
//! readers draw a nebula only when it is active and explored, and loop
//! over just the 32 slots (@0xfc7d). Activity has no gameplay effect:
//! interference and murk come from the `sÿst`.
//!
//! **`_ExploreNebulaeFromSystem`** (0x508f-0x51bd, [`explores`]). It
//! takes a system and visits every slot by ascending ID (@0x51a3-0x51b0).
//! A slot is skipped unless it is active, +0x8 (@0x50bc); not yet
//! explored, +0x9 (@0x50c6); and its rectangle, inset by 8, holds the
//! system ([`holds`]). The function builds the rectangle with `SetRect`:
//! left x, top y, right x+`XSize`, bottom y+`YSize` (@0x50d0-0x50fb). It
//! then calls `InsetRect(8, 8)` (@0x5116) and tests the system's map
//! position (x +0x0, y +0x2) with `PtInRect` (@0x5149), which is
//! half-open: the left and top edges are inside, the right and bottom
//! edges are not. A rectangle 16 or less across holds nothing ([`INSET`]).
//! For a slot that passes, the function marks the slot explored first
//! (@0x515a), then runs its `OnExplore` (`_EvalMissionBitSetString`
//! @0x519e), so an `OnExplore` that explores again never re-runs itself.
//! It plays no sound. Every branch is ported.
//!
//! **Who explores.** `_RecursiveAutoExplore` @0xd98a explores a nebula
//! for each system it reaches, not that system's place's head: for the
//! map grants (`_GrantOutfitItem` @0x44dcb, @0x44e12, @0x44e86), at the
//! take-off (@0x634c8), at a jump's arrival after its days (@0x6c1db) and
//! for the escape pod (@0x690cb). A jump explores each hop's system and
//! then the arrival system before its refresh and its days (@0x6bddb,
//! @0x6be12). The take-off explores twice (@0x634c8, @0x634da). A gate's
//! or wormhole's exit explores no nebula (`_PlayerEnterHypergate`
//! 0x637bf, `_PlayerEnterWormhole` 0x64005). A new pilot explores none:
//! `_ResetGameData` clears every slot's +0x8 and +0x9 (@0x18de5,
//! @0x1df44-0x1df4b), and `_DoNewPilot` explores its start system
//! (@0x19118) before its first refresh (@0x1925e), with every nebula
//! inactive ([`RuleKey::NebulaNewPilot`](crate::RuleKey::NebulaNewPilot)).
//!
//! **Never saved.** `_ResetPlayer` clears +0x9 (@0x1da86) before every
//! load, and after the load's refresh `_OpenLastPilotFile`
//! (@0x765a2-0x765ce) and `_MainScreenChoice` (@0x1a8d4-0x1a906) explore
//! every visible explored system by ascending index, so `OnExplore` runs
//! again after every load: Open Pilot's sweep, and the launch's
//! (main-screen phase 4), are the session's load sweep, and the launch
//! then explores the player's system once, as `_DoMainScreen` does
//! (@0x1affe, `nova_sim::Session::open_saved` at launch). The Bible agrees: "the explored state of the
//! nebulae isn't saved from game to game". The once-a-pilot reading
//! keeps a nebula's [`nebula_hook`] in the once-fired record
//! ([`RuleKey::NebulaExploreOnce`](crate::RuleKey::NebulaExploreOnce),
//! [`kept`]).
//!
//! Left to others: the escape pod's exploration (@0x690cb), roadmap
//! player-death; the developer key that sets or clears every explored
//! flag (`_HandlePlayer` @0x6da73-0x6dacf), not ported.

use std::collections::BTreeSet;

pub use crate::catalog::{FIRST_NEBULA, LAST_NEBULA};
use crate::catalog::{NebulaId, NebulaRecord};
use crate::control::Gate;
use crate::geometry::Vec2;
use crate::story::{Consumer, SubscriptionId};

/// How far `_ExploreNebulaeFromSystem` insets a nebula's rectangle on
/// each side (`InsetRect(8, 8)` @0x5116).
pub const INSET: i32 = 8;

/// Whether `record` is active for `gate`: both its sizes above 0 and its
/// `ActiveOn` holding (`_PropagateMissionBitEffects` @0x998b4-0x998ef).
#[must_use]
pub fn active(record: &NebulaRecord, gate: &Gate) -> bool {
    record.width > 0 && record.height > 0 && gate.allows(&record.active_on)
}

/// Whether `record`'s rectangle, inset by [`INSET`] on each side, holds
/// `position`: its left and top edges included, its right and bottom ones
/// not (`SetRect`, `InsetRect`, `PtInRect` @0x50d0-0x5149). Computed in
/// `i32`, so a rectangle reaching past the 16-bit range does not wrap.
#[must_use]
pub fn holds(record: &NebulaRecord, position: Vec2) -> bool {
    let left = i32::from(record.left) + INSET;
    let top = i32::from(record.top) + INSET;
    let right = i32::from(record.left) + i32::from(record.width) - INSET;
    let bottom = i32::from(record.top) + i32::from(record.height) - INSET;
    #[allow(clippy::cast_precision_loss)] // map coordinates are small
    let (left, top, right, bottom) = (left as f32, top as f32, right as f32, bottom as f32);
    left <= position.x && position.x < right && top <= position.y && position.y < bottom
}

/// Whether exploring a system at `position` (`None` for a system not in
/// the catalog) explores `record`, `active` and `explored` or not:
/// `_ExploreNebulaeFromSystem`'s test (@0x50bc, @0x50c6, @0x5149).
#[must_use]
pub fn explores(
    record: &NebulaRecord,
    active: bool,
    explored: bool,
    position: Option<Vec2>,
) -> bool {
    active && !explored && position.is_some_and(|position| holds(record, position))
}

/// The name of nebula `nebula`'s `OnExplore` subscription, which keys its
/// once-fired record under the once-a-pilot reading.
#[must_use]
pub const fn nebula_hook(nebula: NebulaId) -> SubscriptionId {
    SubscriptionId {
        consumer: Consumer::Nebulae,
        record: nebula.0,
        hook: 0,
    }
}

/// The nebulae whose [`nebula_hook`] is among `fired`, by ascending ID:
/// those the once-a-pilot reading keeps explored.
pub fn kept(fired: &BTreeSet<SubscriptionId>) -> impl Iterator<Item = NebulaId> + '_ {
    fired
        .iter()
        .filter(|id| **id == nebula_hook(NebulaId(id.record)))
        .map(|id| NebulaId(id.record))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{OutfitId, SystemId};
    use crate::control::{Bit, NovaBits, PilotFacts, Script, Test};
    use crate::pilot::Gender;

    /// A pilot with only `bits` set.
    #[derive(Debug, Default)]
    struct Bits(BTreeSet<u16>);

    impl PilotFacts for Bits {
        fn bit(&self, bit: Bit) -> bool {
            self.0.contains(&bit.get())
        }

        fn gender(&self) -> Gender {
            Gender::Male
        }

        fn paid(&self, _days: u16) -> bool {
            true
        }

        fn has_outfit(&self, _outfit: OutfitId) -> bool {
            false
        }

        fn explored(&self, _system: SystemId) -> bool {
            false
        }
    }

    /// Whether `record` is active for a pilot with `bits` set.
    fn active_for(record: &NebulaRecord, bits: &[u16]) -> bool {
        let pilot = Bits(bits.iter().copied().collect());
        active(
            record,
            &Gate {
                control_bits: &NovaBits,
                pilot: &pilot,
            },
        )
    }

    /// Nebula 128 over (left, top, width, height), active by `active_on`.
    fn nebula(rect: (i16, i16, i16, i16), active_on: &str) -> NebulaRecord {
        NebulaRecord {
            id: NebulaId(128),
            left: rect.0,
            top: rect.1,
            width: rect.2,
            height: rect.3,
            active_on: Test::parse(active_on),
            on_explore: Script::default(),
        }
    }

    fn at(x: f32, y: f32) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn a_nebula_of_no_width_or_no_height_is_inactive_whatever_its_test() {
        for rect in [(0, 0, 0, 50), (0, 0, 50, 0), (0, 0, -5, 50), (0, 0, 50, -1)] {
            assert!(!active_for(&nebula(rect, ""), &[]), "{rect:?}");
        }
        assert!(active_for(&nebula((0, 0, 1, 1), ""), &[]));
    }

    #[test]
    fn a_nebula_is_active_by_its_active_on_and_a_blank_test_holds() {
        let gated = nebula((0, 0, 100, 50), "b5");
        assert!(!active_for(&gated, &[]));
        assert!(active_for(&gated, &[5]));
        assert!(active_for(&nebula((0, 0, 100, 50), ""), &[]), "blank");
        assert!(
            !active_for(&nebula((0, 0, 100, 50), "b5 &"), &[5]),
            "unparsed"
        );
    }

    #[test]
    fn the_inset_rectangle_holds_its_left_and_top_edges_and_not_its_right_and_bottom() {
        let record = nebula((0, 0, 100, 50), "");
        assert!(holds(&record, at(8.0, 8.0)));
        assert!(!holds(&record, at(7.0, 8.0)));
        assert!(!holds(&record, at(8.0, 7.0)));
        assert!(holds(&record, at(91.0, 41.0)));
        assert!(!holds(&record, at(92.0, 8.0)));
        assert!(!holds(&record, at(8.0, 42.0)));
        let shifted = nebula((500, -100, 200, 200), "");
        assert!(holds(&shifted, at(600.0, 0.0)));
        assert!(holds(&shifted, at(508.0, -92.0)));
        assert!(!holds(&shifted, at(692.0, 0.0)));
        assert!(!holds(&shifted, at(600.0, 92.0)));
    }

    #[test]
    fn a_rectangle_16_or_narrower_holds_nothing() {
        for (width, height) in [(16, 100), (100, 16), (10, 10)] {
            let record = nebula((0, 0, width, height), "");
            for x in -2..=20 {
                for y in -2..=20 {
                    #[allow(clippy::cast_precision_loss)]
                    let point = at(x as f32, y as f32);
                    assert!(!holds(&record, point), "{width}x{height} at {point:?}");
                }
            }
        }
        assert!(
            holds(&nebula((0, 0, 17, 17), ""), at(8.0, 8.0)),
            "17 holds one"
        );
    }

    #[test]
    fn a_rectangle_past_the_16_bit_range_does_not_wrap() {
        let record = nebula((i16::MAX - 20, 0, i16::MAX, 100), "");
        assert!(holds(&record, at(40_000.0, 50.0)));
    }

    #[test]
    fn a_nebula_explores_a_system_only_active_unexplored_and_holding_it() {
        let record = nebula((0, 0, 100, 50), "");
        let inside = Some(at(50.0, 25.0));
        assert!(explores(&record, true, false, inside));
        assert!(!explores(&record, false, false, inside), "inactive");
        assert!(!explores(&record, true, true, inside), "explored");
        assert!(
            !explores(&record, true, false, Some(at(0.0, 0.0))),
            "outside"
        );
        assert!(!explores(&record, true, false, None), "no such system");
    }

    #[test]
    fn a_nebulas_hook_names_it_under_the_nebulae() {
        assert_eq!(
            nebula_hook(NebulaId(130)),
            SubscriptionId {
                consumer: Consumer::Nebulae,
                record: 130,
                hook: 0,
            }
        );
    }

    #[test]
    fn kept_gives_the_nebulae_whose_hook_has_fired() {
        let other = |consumer, record, hook| SubscriptionId {
            consumer,
            record,
            hook,
        };
        let fired = BTreeSet::from([
            nebula_hook(NebulaId(140)),
            other(Consumer::Crons, 129, 0),
            nebula_hook(NebulaId(128)),
            other(Consumer::Nebulae, 131, 1),
            other(Consumer::Fixture, 132, 0),
        ]);
        assert_eq!(
            kept(&fired).collect::<Vec<_>>(),
            [NebulaId(128), NebulaId(140)]
        );
        assert_eq!(kept(&BTreeSet::new()).count(), 0);
    }
}
