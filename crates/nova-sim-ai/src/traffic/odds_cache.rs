//! The odds against each ship as its ship-table slot caches them (the
//! original's +0xc914), refreshed one slot a frame.
//!
//! **The original.** Every spawn marks its slot stale, -1
//! (`_RandomShipSpawn` @0x3c657, `_GenericRandomShipSpawn` @0x3cb6a,
//! `_RespawnEscort` @0x3d4fb). After the frame's thinks,
//! `_DoPlayGameWork`'s odds pass (@0x442b0-0x44336, @0x44542-0x44553)
//! walks slots 1..=63 ([`LAST_SLOT`]) for the first that reads below 0
//! (@0x442d4-0x442db). It marks the next slot stale (@0x442e3; slot 1
//! after slot 63, @0x442ef), then works out the odds of the ship there
//! (`_AICalculateOddsAgainst` @0x4454e, which writes +0xc914 @0x84c27)
//! and stops, when the slot is active, of AI type above 0 and in the
//! player's system (@0x442fe-0x44319); otherwise it zeroes the slot
//! (@0x4431f) and walks on, through the slot it just marked. So each
//! ship's odds refresh once a round of the table, reading -1 from its
//! spawn until the walk reaches it, and 0 in a slot no live ship holds.
//! The walk never wraps within a frame, and a second stale slot waits for
//! it. Slot 0, the player's, is never walked. The pass draws nothing and
//! plays no sound.
//!
//! **Here.** [`OddsCache`] keeps each berth's value, so a berth whose
//! ship has gone keeps it and the marker is not lost with the ship. Each
//! [`Npc::odds`](crate::traffic::npc::Npc::odds) is its berth's value:
//! only the spawns ([`Traffic::add_npc_at`] and the table's placing) and
//! the pass ([`Traffic::refresh_odds`]) write either, and each writes
//! both. A ship coming back from being left behind
//! ([`Leave::Held`](crate::traffic::Leave::Held)) keeps its value, as it
//! was never respawned, and so do the ships an in-flight `M` or `N` move
//! takes along, as `_EvalSetExp` respawns none.
//!
//! The pass runs once each traffic tick, after the thinks, the pursuit,
//! the commands, the gate waits and the pods freed, and before the flight
//! (`_HandleShip`'s work, after the pass in the original), so no lag
//! arises against the departures. Double time's second pass and the
//! warp's standstill run no thinks and so no pass, as the original's
//! advance-pass gate (@0x4402a) has it.
//!
//! A berth past [`LAST_SLOT`], which the unbounded first-free numbering
//! can give, is never walked: the original's table has no such slot. Its
//! ship stays at -1.
//!
//! The cache lives as long as the [`Traffic`] holding it, as the berths'
//! picks do: across systems, but not across a new traffic. The original
//! keeps it through a pilot load or a new game until the app quits, and
//! gives the player's eject hulk (AI type -1, @0x6f31f) a 0 where the
//! engine works its odds out (@0x44307), which nothing reads; both are
//! recorded in rdm phase combat-ai-fidelity/phase-23-left-out-of-phase-16.
//!
//! Which odds the readers read, the cache's or fresh ones, is
//! [`RuleKey::OddsCache`](crate::RuleKey::OddsCache)'s
//! ([`cached_odds`](crate::ai::odds::cached_odds)); the cache and its pass
//! run by either reading.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use super::{Traffic, World};
use crate::ai::odds::odds_against;

/// The value a stale slot reads: the marker the pass walks to (-1.0,
/// 0xbf800000).
pub const STALE: f32 = -1.0;

/// The last slot the pass walks (63: the table's 64 slots, the player's
/// slot 0 aside).
pub const LAST_SLOT: u32 = 63;

/// The odds each berth caches (see the module docs): a berth never
/// written reads 0.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OddsCache(BTreeMap<u32, f32>);

impl OddsCache {
    /// The odds `berth` caches.
    #[must_use]
    pub fn get(&self, berth: u32) -> f32 {
        self.0.get(&berth).copied().unwrap_or(0.0)
    }

    /// Caches `odds` at `berth`.
    pub fn set(&mut self, berth: u32, odds: f32) {
        self.0.insert(berth, odds);
    }

    /// The frame's odds pass (see the module docs): `refresh` gives the
    /// odds of the live ship at a berth, or `None` when no live ship in
    /// the system holds it. Gives the writes made, in order.
    pub fn pass(&mut self, mut refresh: impl FnMut(u32) -> Option<f32>) -> Vec<(u32, f32)> {
        let mut writes = Vec::new();
        let mut write = |cache: &mut Self, berth: u32, odds: f32| {
            cache.set(berth, odds);
            writes.push((berth, odds));
        };
        for berth in 1..=LAST_SLOT {
            // A NaN is skipped too (`jbe` @0x442db).
            if self.get(berth).partial_cmp(&0.0) != Some(Ordering::Less) {
                continue;
            }
            let next = if berth < LAST_SLOT { berth + 1 } else { 1 };
            write(self, next, STALE);
            if let Some(odds) = refresh(berth) {
                write(self, berth, odds);
                break;
            }
            write(self, berth, 0.0);
        }
        writes
    }
}

impl Traffic {
    /// Marks the new ship at `berth`, and its berth, stale, as every spawn
    /// does (see the module docs).
    pub(super) fn mark_spawned(&mut self, berth: u32) {
        self.odds.set(berth, STALE);
    }

    /// The frame's odds pass in `world` (see the module docs): a berth is
    /// live when an NPC in the system holds it, and its odds are
    /// [`odds_against`] it as the NPCs stand now, after the frame's goals.
    /// A berth held only by a ship left behind, or by none, reads 0. Each
    /// write lands on the ships at its berth too.
    pub fn refresh_odds(&mut self, world: &World) {
        let around = world.around(&self.npcs);
        let npcs = &self.npcs;
        let writes = self.odds.pass(|berth| {
            npcs.iter()
                .find(|npc| npc.berth == berth)
                .map(|npc| odds_against(npc, &around))
        });
        for (berth, odds) in writes {
            let left = self
                .left_behind
                .values_mut()
                .flatten()
                .map(|left| &mut left.npc);
            for npc in self.npcs.iter_mut().chain(left) {
                if npc.berth == berth {
                    npc.odds = odds;
                }
            }
        }
    }

    /// The odds `berth` caches.
    #[must_use]
    pub fn berth_odds(&self, berth: u32) -> f32 {
        self.odds.get(berth)
    }

    /// Caches `odds` for NPC `id` and its berth, for tests that set a
    /// ship's odds up.
    #[cfg(any(test, feature = "fixture"))]
    pub fn cache_odds(&mut self, id: crate::traffic::npc::NpcId, odds: f32) {
        if let Some(npc) = self.npcs.iter_mut().find(|npc| npc.id == id) {
            npc.odds = odds;
            self.odds.set(npc.berth, odds);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// A cache whose berths read as `values` give them.
    fn cache(values: &[(u32, f32)]) -> OddsCache {
        let mut cache = OddsCache::default();
        for &(berth, odds) in values {
            cache.set(berth, odds);
        }
        cache
    }

    #[test]
    fn a_berth_never_written_reads_zero() {
        assert_eq!(OddsCache::default().get(5), 0.0);
        assert_eq!(cache(&[(5, 1.5)]).get(5), 1.5);
    }

    #[test]
    fn with_no_stale_berth_nothing_changes() {
        let mut odds = cache(&[(1, 0.5), (2, 0.0), (63, 2.0)]);
        let before = odds.clone();
        let writes = odds.pass(|_| panic!("no berth is refreshed"));
        assert_eq!(writes, []);
        assert_eq!(odds, before);
    }

    #[test]
    fn the_first_stale_live_berth_is_refreshed_and_the_next_marked_and_the_walk_stops() {
        let mut odds = cache(&[(3, STALE), (9, STALE)]);
        let mut asked = Vec::new();
        let writes = odds.pass(|berth| {
            asked.push(berth);
            Some(1.25)
        });
        assert_eq!(asked, [3]);
        assert_eq!(writes, [(4, STALE), (3, 1.25)]);
        assert_eq!((odds.get(3), odds.get(4)), (1.25, STALE));
        assert_eq!(odds.get(9), STALE, "a later stale berth waits");
    }

    #[test]
    fn the_marker_wraps_from_the_last_slot_to_slot_one_and_slot_zero_is_never_walked() {
        let mut odds = cache(&[(0, STALE), (63, STALE)]);
        let mut asked = Vec::new();
        let writes = odds.pass(|berth| {
            asked.push(berth);
            Some(0.5)
        });
        assert_eq!(asked, [63]);
        assert_eq!(writes, [(1, STALE), (63, 0.5)]);
        assert_eq!(odds.get(0), STALE, "the player's slot is never walked");
    }

    #[test]
    fn an_empty_stale_berth_reads_zero_and_the_walk_goes_on_to_the_first_live_ship() {
        let mut odds = cache(&[(2, STALE)]);
        let mut asked = Vec::new();
        let writes = odds.pass(|berth| {
            asked.push(berth);
            (berth == 4).then_some(3.0)
        });
        assert_eq!(asked, [2, 3, 4]);
        assert_eq!(
            writes,
            [
                (3, STALE),
                (2, 0.0),
                (4, STALE),
                (3, 0.0),
                (5, STALE),
                (4, 3.0)
            ]
        );
        assert_eq!(
            (odds.get(2), odds.get(3), odds.get(4), odds.get(5)),
            (0.0, 0.0, 3.0, STALE)
        );
    }

    #[test]
    fn with_no_live_ship_from_the_marker_up_every_berth_to_the_last_reads_zero_and_slot_one_is_marked()
     {
        let mut odds = cache(&[(60, STALE), (2, 4.0)]);
        let mut asked = Vec::new();
        odds.pass(|berth| {
            asked.push(berth);
            None
        });
        assert_eq!(asked, [60, 61, 62, 63], "no wrap within the pass");
        assert!((60..=63).all(|berth| odds.get(berth) == 0.0));
        assert_eq!(odds.get(1), STALE);
        assert_eq!(odds.get(2), 4.0, "not reached");
    }

    #[test]
    fn the_marker_overwrites_the_next_berths_odds() {
        let mut odds = cache(&[(7, STALE), (8, 2.5)]);
        odds.pass(|_| Some(1.0));
        assert_eq!(odds.get(8), STALE);
    }

    #[test]
    fn a_nan_is_not_stale() {
        let mut odds = cache(&[(1, f32::NAN), (2, STALE)]);
        let mut asked = Vec::new();
        odds.pass(|berth| {
            asked.push(berth);
            Some(1.0)
        });
        assert_eq!(asked, [2]);
    }

    #[test]
    fn a_berth_past_the_last_slot_is_never_walked() {
        let mut odds = cache(&[(64, STALE), (70, STALE)]);
        assert_eq!(odds.pass(|_| Some(1.0)), []);
        assert_eq!(odds.get(64), STALE);
    }
}
