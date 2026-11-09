//! The original's fixed tables whose entries take the first free slot:
//! `_boxData`, the cargo boxes ([`cargo_box`](crate::cargo_box)),
//! `_podData`, the escape pods ([`escape_pod`](crate::escape_pod)), and
//! `_explodData` and `_smokeData`, the explosions and the smoke, which
//! only the view simulates (`nova_view::flight::effects` and
//! `nova_view::flight::smoke`, through `nova_sim::slot_pool`).
//!
//! # The original
//!
//! A new box takes the first slot whose life is below 0 (`_SpawnBox`
//! @0x3dd94, `_SpawnScoopableBox` @0x3e3f8), and so does a new pod
//! (`_SpawnEscapePod` @0x45564), a new explosion (`_SpawnExplod`
//! @0x3f5ef) and a new puff of smoke (`_SpawnSmoke` @0x3f957).
//! `_InitSprites` adds each table's sprites to its layer once, in slot
//! order (the boxes @0x1fb89-0x1fc6b, then the pods @0x1fc71-0x1fd57, to
//! `_boxLayer`; `_explodData` @0x1f962; `_smokeData` @0x1fe8f), and
//! nothing moves them after (no `_PutSpriteInLayer` on them, no
//! `_SWSortSpriteLayer`), so each is drawn, and a box or pod collided, in
//! slot order: a new one in a freed low slot comes before an older one in
//! a higher slot, and draws under it.
//!
//! # The engine
//!
//! A [`SlotPool`] keeps its items in slot order, each with its slot
//! ([`SlotPool::insert`] takes the lowest free one), and lends them as a
//! slice in that order ([`SlotPool::as_slice`]). Under a cap
//! (`RuleKey::CombatCaps`' engine reading) an item with no free slot below
//! the cap is dropped. Without one the pool grows past the table's size,
//! still first-free: slot n is first used only while slots 0..n are all
//! taken, which is the order the original's layer would show for a larger
//! table, as `ShotSlots::first_free` extends the shot table.

/// Items kept in slot order, a new one taking the first free slot (see
/// the module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct SlotPool<T> {
    /// The items kept, in slot order.
    items: Vec<T>,
    /// Each item's slot, ascending, one for each of `items`.
    slots: Vec<usize>,
}

impl<T> Default for SlotPool<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            slots: Vec::new(),
        }
    }
}

impl<T> SlotPool<T> {
    /// Puts `item` in the first free slot, or a new one at the end, and
    /// returns that slot; with `cap` and no free slot below it, drops
    /// `item` and returns `None`.
    pub fn insert(&mut self, item: T, cap: Option<usize>) -> Option<usize> {
        // The slots ascend, so the first index whose slot is past it is
        // the first free slot.
        let slot = self
            .slots
            .iter()
            .enumerate()
            .position(|(index, &slot)| slot != index)
            .unwrap_or(self.slots.len());
        if cap.is_some_and(|cap| slot >= cap) {
            return None;
        }
        self.items.insert(slot, item);
        self.slots.insert(slot, slot);
        Some(slot)
    }

    /// Keeps each item for which `keep` is true and frees the slot of each
    /// other, in place, walking in slot order.
    pub fn retain_mut(&mut self, mut keep: impl FnMut(&mut T) -> bool) {
        let mut index = 0;
        while index < self.items.len() {
            if keep(&mut self.items[index]) {
                index += 1;
            } else {
                self.remove(index);
            }
        }
    }

    /// Takes out the item at `index` of [`SlotPool::as_slice`], freeing
    /// its slot.
    ///
    /// # Panics
    ///
    /// When `index` is past the items kept.
    pub fn remove(&mut self, index: usize) -> T {
        self.slots.remove(index);
        self.items.remove(index)
    }

    /// The slot of the item at `index` of [`SlotPool::as_slice`], if there
    /// is one.
    #[must_use]
    pub fn slot_of(&self, index: usize) -> Option<usize> {
        self.slots.get(index).copied()
    }

    /// The items kept, in slot order.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    /// The items kept, in slot order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    /// The items kept, in slot order, to change in place.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.items.iter_mut()
    }

    /// How many items are kept.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether no item is kept.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Frees every slot.
    pub fn clear(&mut self) {
        self.items.clear();
        self.slots.clear();
    }
}

impl<T: Clone> SlotPool<T> {
    /// The items kept, in slot order.
    #[must_use]
    pub fn to_vec(&self) -> Vec<T> {
        self.items.clone()
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
        assert_eq!(pool.as_slice(), [4, 2, 3]);
    }

    #[test]
    fn a_hole_between_kept_slots_is_filled_first() {
        let mut pool = pool(&[1, 2, 3, 4]);
        pool.retain_mut(|item| *item != 2 && *item != 3);
        assert_eq!(pool.insert(5, None), Some(1));
        assert_eq!(pool.insert(6, None), Some(2));
        assert_eq!(pool.to_vec(), [1, 5, 6, 4]);
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
    fn removing_an_item_frees_its_slot_and_says_whose_it_was() {
        let mut pool = pool(&[1, 2, 3]);
        assert_eq!(pool.remove(1), 2);
        assert_eq!(pool.to_vec(), [1, 3]);
        assert_eq!((pool.slot_of(0), pool.slot_of(1)), (Some(0), Some(2)));
        assert_eq!(pool.slot_of(2), None, "past the items");
        assert_eq!(pool.insert(4, None), Some(1));
    }

    #[test]
    fn len_counts_the_items_kept_and_clear_frees_every_slot() {
        let mut pool = pool(&[1, 2, 3]);
        pool.retain_mut(|item| *item != 1);
        assert_eq!(pool.len(), 2);
        assert!(!pool.is_empty());
        pool.clear();
        assert_eq!(pool.len(), 0);
        assert!(pool.is_empty());
        assert_eq!(pool, SlotPool::default());
    }
}
