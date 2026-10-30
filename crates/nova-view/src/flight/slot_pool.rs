//! The original's fixed sprite tables, `_explodData`, `_smokeData` and
//! `_podData`, with their first-free pick.
//!
//! # The original
//!
//! `_InitSprites` adds each table's sprites to its layer once, in slot
//! order (`_explodData` @0x1f962, `_podData` @0x1fc71, `_smokeData`
//! @0x1fe8f), and nothing moves them after (no `_PutSpriteInLayer` on
//! them, no `_SWSortSpriteLayer`), so each layer draws in slot order. A
//! new one takes the first free slot (`_SpawnExplod` @0x3f5ef,
//! `_SpawnSmoke` @0x3f957, `_SpawnEscapePod` @0x45564), so a new one in a
//! freed low slot draws under an older one in a higher slot.
//!
//! # The engine
//!
//! A [`SlotPool`] keeps its items by slot ([`SlotPool::insert`] takes the
//! lowest free one) and walks them in slot order ([`SlotPool::iter`]).
//! Under a cap (`RuleKey::CombatCaps`' engine reading) an item with no
//! free slot below the cap is dropped. Without one the pool grows past the
//! table's size, still first-free: slot n is first used only while slots
//! 0..n are all taken, which is the order the original's layer would show
//! for a larger table, as `ShotSlots::first_free` extends the shot table.

/// Items kept by slot, a new one taking the first free slot (see the
/// module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct SlotPool<T> {
    /// Each slot's item, `None` when free. Never ends in a free slot.
    slots: Vec<Option<T>>,
}

impl<T> Default for SlotPool<T> {
    fn default() -> Self {
        Self { slots: Vec::new() }
    }
}

impl<T> SlotPool<T> {
    /// Puts `item` in the first free slot, or a new one at the end, and
    /// returns that slot; with `cap` and no free slot below it, drops
    /// `item` and returns `None`.
    pub fn insert(&mut self, item: T, cap: Option<usize>) -> Option<usize> {
        let slot = self
            .slots
            .iter()
            .position(Option::is_none)
            .unwrap_or(self.slots.len());
        if cap.is_some_and(|cap| slot >= cap) {
            return None;
        }
        if slot == self.slots.len() {
            self.slots.push(Some(item));
        } else {
            self.slots[slot] = Some(item);
        }
        Some(slot)
    }

    /// Keeps each item for which `keep` is true and frees the slot of each
    /// other, in place, walking in slot order.
    pub fn retain_mut(&mut self, mut keep: impl FnMut(&mut T) -> bool) {
        for slot in &mut self.slots {
            if slot.as_mut().is_some_and(|item| !keep(item)) {
                *slot = None;
            }
        }
        self.trim();
    }

    /// The items kept, in slot order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.slots.iter().flatten()
    }

    /// The items kept, in slot order, to change in place.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.slots.iter_mut().flatten()
    }

    /// How many items are kept.
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Frees every slot.
    pub fn clear(&mut self) {
        self.slots.clear();
    }

    /// Drops the free slots at the end, so two pools holding the same
    /// items in the same slots compare equal.
    fn trim(&mut self) {
        while self.slots.last().is_some_and(Option::is_none) {
            self.slots.pop();
        }
    }
}

impl<T: Clone> SlotPool<T> {
    /// The items kept, in slot order.
    pub fn to_vec(&self) -> Vec<T> {
        self.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(items: &[u32]) -> SlotPool<u32> {
        let mut pool = SlotPool::default();
        for &item in items {
            pool.insert(item, None);
        }
        pool
    }

    #[test]
    fn an_item_takes_the_first_free_slot() {
        let mut pool = pool(&[1, 2, 3]);
        assert_eq!(pool.insert(4, None), Some(3), "none free: the end");
        pool.retain_mut(|item| *item != 1 && *item != 2);
        assert_eq!(pool.insert(5, None), Some(0));
        assert_eq!(pool.insert(6, None), Some(1));
        assert_eq!(pool.insert(7, None), Some(4));
    }

    #[test]
    fn a_freed_lower_slot_is_walked_before_an_older_higher_one() {
        let mut pool = pool(&[1, 2, 3]);
        pool.retain_mut(|item| *item != 1);
        assert_eq!(pool.to_vec(), [2, 3]);
        pool.insert(4, None);
        assert_eq!(pool.to_vec(), [4, 2, 3]);
        assert_eq!(pool.iter().copied().collect::<Vec<_>>(), [4, 2, 3]);
    }

    #[test]
    fn retain_mut_changes_the_items_kept_in_place() {
        let mut pool = pool(&[1, 2, 3]);
        pool.retain_mut(|item| {
            *item *= 10;
            *item != 20
        });
        assert_eq!(pool.to_vec(), [10, 30]);
        assert_eq!(pool.insert(4, None), Some(1), "2's slot freed");
        assert_eq!(pool.to_vec(), [10, 4, 30]);
    }

    #[test]
    fn under_a_cap_an_item_with_no_free_slot_below_it_is_dropped() {
        let mut pool = SlotPool::default();
        assert_eq!(pool.insert(1, Some(2)), Some(0));
        assert_eq!(pool.insert(2, Some(2)), Some(1));
        assert_eq!(pool.insert(3, Some(2)), None);
        assert_eq!(pool.to_vec(), [1, 2]);
        pool.retain_mut(|item| *item != 1);
        assert_eq!(pool.insert(4, Some(2)), Some(0), "the freed slot");
        assert_eq!(pool.to_vec(), [4, 2]);
    }

    #[test]
    fn without_a_cap_the_pool_grows_past_it() {
        let mut pool = pool(&[1, 2]);
        assert_eq!(pool.insert(3, None), Some(2));
        assert_eq!(pool.len(), 3);
    }

    #[test]
    fn freed_slots_at_the_end_leave_no_trace() {
        let mut freed = pool(&[1, 2, 3]);
        freed.retain_mut(|item| *item == 1);
        assert_eq!(freed, pool(&[1]));
        let mut holed = pool(&[1, 2, 3]);
        holed.retain_mut(|item| *item != 2);
        assert_ne!(holed, pool(&[1, 3]), "3 keeps its slot");
    }

    #[test]
    fn len_counts_the_items_kept_and_clear_frees_every_slot() {
        let mut pool = pool(&[1, 2, 3]);
        pool.retain_mut(|item| *item != 1);
        assert_eq!(pool.len(), 2);
        pool.clear();
        assert_eq!(pool.len(), 0);
        assert_eq!(pool, SlotPool::default());
    }
}
