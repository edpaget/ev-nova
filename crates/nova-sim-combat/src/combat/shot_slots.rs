//! The original's shot slots: which slot a shot takes, when its slot is
//! free again, the place each slot's sprite keeps in its layer, and the
//! miss roll a slot is left holding (rdm weapons-fidelity phase 34).
//!
//! # The original
//!
//! **The table.** 128 records of 0x48 bytes (handle `0x3b5474`), each with
//! its sprite (+0x0) and its life (+0x14, a float). `_SpawnShot` walks
//! slots 0..127 and takes the first whose life is below -1.0
//! (@0x3e5ab-0x3e5ce), returning -1 with none free (@0x3e5d0)
//! ([`ShotSlots::first_free`]).
//!
//! **Held until `_HandleShot` visits.** A shot that ends (life -1.0 on a
//! hit, 0 to point defence) keeps its slot until `_HandleShot`'s next
//! visit to that slot writes -2.0 (@0x36195 → @0x3617a-0x3618e)
//! ([`ShotSlots::hold`], [`ShotSlots::free`]).
//!
//! **Every slot's sprite lives in a layer for the whole run.**
//! `_InitSprites` adds the 128 slot sprites to the shot layer 0x3b52f0 in
//! slot order, hidden (@0x1fa90-0x1fb83), once, at launch. Only
//! `_PutSpriteInLayer` (@0x766c-0x76b1) moves one: a sprite already in the
//! wanted layer stays where it is (@0x768c), and any other leaves its
//! layer and is appended at the wanted one's tail (`_SWAddSprite`
//! @0xac620-0xac639) ([`ShotSlots::put`]). A freed slot's sprite is only
//! hidden, so it keeps its place, across systems and pilots
//! ([`ShotSlots::clear`] keeps it). The layer's order is its entry order,
//! which drawing and the contact passes walk ([`ShotSlots::rank`]).
//!
//! **The leftover miss roll.** `_SpawnShot` writes a shot's miss roll
//! (+0x42) only for a ship-fired shot (@0x3e5e1-0x3e5f7, @0x3e89e); a
//! sub-munition writes none and carries what the slot's last ship-fired
//! shot left there ([`ShotSlots::write_roll`], [`ShotSlots::roll`]).
//!
//! **The leftover spin timer.** `_SpawnShot` never writes a shot's spin
//! timer (+0x1c; no store to it @0x3e550-0x3f143), and only
//! `_HandleShot`'s spin step does (@0x35884-0x358c4), so a new shot
//! starts on the timer the slot's last spinning shot left
//! ([`ShotSlots::spin`], [`ShotSlots::spin_pass`]; see
//! [`spin_timer`](super::spin_timer)), across systems and pilots too. A
//! slot never used holds 0, the zeroed table's.
//!
//! Slots past 127 exist only uncapped
//! ([`RuleKey::CombatCaps`](crate::RuleKey::CombatCaps)'s default), the
//! engine's extension past the table: such a slot is in no layer until
//! its first [`ShotSlots::put`] appends it.

use std::collections::{BTreeMap, BTreeSet};

use super::projectile::{Shot, Source};
use super::shot_layer::ShotLayer;
use super::spin_timer;
use super::{Combat, MAX_SHOTS};
use crate::rulebook::RuleSource;

/// The original's shot slots (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShotSlots {
    /// The slots whose shot has ended but which `_HandleShot` has not
    /// freed yet.
    held: BTreeSet<usize>,
    /// The slots put in a layer since launch, each with its layer and its
    /// entry, counted from 1. A slot not here below [`MAX_SHOTS`] is where
    /// `_InitSprites` left it.
    moved: BTreeMap<usize, (ShotLayer, i64)>,
    /// The last entry given.
    last: i64,
    /// The miss roll each slot's last ship-fired shot left, where it left
    /// one.
    rolls: BTreeMap<usize, u32>,
    /// Each slot's spin timer (+0x1c), in passes, where a spinning shot
    /// has run it.
    spins: BTreeMap<usize, u32>,
}

impl ShotSlots {
    /// The slots as `_InitSprites` leaves them: none held, slots 0..127 in
    /// [`ShotLayer::Shots`] in slot order, and no roll in any.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The first slot a shot spawned now takes (`_SpawnShot`
    /// @0x3e5ab-0x3e5ce): the lowest neither among `taken` (the shots in
    /// flight) nor held. Under the caps (`capped`) it is below
    /// [`MAX_SHOTS`] or there is none (@0x3e5d0); uncapped, the table
    /// grows past it.
    #[must_use]
    pub fn first_free(
        &self,
        taken: impl IntoIterator<Item = usize>,
        capped: bool,
    ) -> Option<usize> {
        let mut used: BTreeSet<usize> = taken.into_iter().collect();
        used.extend(&self.held);
        // Sorted and distinct, so the first slot not at its own index is
        // the lowest gap.
        let slot = used
            .iter()
            .enumerate()
            .find(|&(index, &slot)| index != slot)
            .map_or(used.len(), |(index, _)| index);
        (!capped || slot < MAX_SHOTS).then_some(slot)
    }

    /// Puts `slot`'s sprite in `layer`, as `_PutSpriteInLayer` does
    /// (@0x766c-0x76b1): nothing when it is there already (@0x768c);
    /// otherwise it leaves its layer (`_SWRemoveSprite` @0x7693) and is
    /// appended at `layer`'s tail (`_SWAddSprite` @0x76a5). Its null guard
    /// (@0x767e-0x768a) has no counterpart.
    pub fn put(&mut self, slot: usize, layer: ShotLayer) {
        if self.rank(slot).map(|(now, _)| now) != Some(layer) {
            self.last += 1;
            self.moved.insert(slot, (layer, self.last));
        }
    }

    /// `slot`'s layer and its entry there: of two slots in one layer, the
    /// lower entry is drawn and walked first. None for a slot past the
    /// original's table never put in a layer.
    #[must_use]
    pub fn rank(&self, slot: usize) -> Option<(ShotLayer, i64)> {
        self.moved.get(&slot).copied().or_else(|| {
            let initial = i64::try_from(slot).ok().map(|slot| slot - MAX_SHOTS as i64);
            initial
                .filter(|_| slot < MAX_SHOTS)
                .map(|entry| (ShotLayer::Shots, entry))
        })
    }

    /// Holds `slot`, whose shot has ended, until `_HandleShot` frees it.
    pub fn hold(&mut self, slot: usize) {
        self.held.insert(slot);
    }

    /// Frees `slot`, as `_HandleShot`'s visit does (@0x3617c): its sprite
    /// keeps its place and the slot its roll.
    pub fn free(&mut self, slot: usize) {
        self.held.remove(&slot);
    }

    /// Whether `slot` is held.
    #[must_use]
    pub fn is_held(&self, slot: usize) -> bool {
        self.held.contains(&slot)
    }

    /// The held slots, lowest first.
    pub fn held(&self) -> impl Iterator<Item = usize> + '_ {
        self.held.iter().copied()
    }

    /// Writes `roll` as `slot`'s miss roll, as a ship-fired shot does
    /// (`_SpawnShot` @0x3e89e): none for a shot that rolls no miss
    /// (@0x3e95c, the original's -1).
    pub fn write_roll(&mut self, slot: usize, roll: Option<u32>) {
        match roll {
            Some(roll) => self.rolls.insert(slot, roll),
            None => self.rolls.remove(&slot),
        };
    }

    /// The miss roll `slot` holds, which a sub-munition spawned into it
    /// carries: none for a slot never used (the original's 0) or last used
    /// by a shot that rolls no miss (-1).
    #[must_use]
    pub fn roll(&self, slot: usize) -> Option<u32> {
        self.rolls.get(&slot).copied()
    }

    /// The spin timer `slot` holds, in passes, which a shot put in it
    /// starts on: 0 for a slot never used.
    #[must_use]
    pub fn spin(&self, slot: usize) -> u32 {
        self.spins.get(&slot).copied().unwrap_or(0)
    }

    /// Runs `slot`'s spin timer a pass, as a live visit of a spinning shot
    /// of `BeamWidth` `beam_width` does ([`spin_timer::pass`]).
    pub fn spin_pass(&mut self, slot: usize, beam_width: i16) {
        let (timer, _) = spin_timer::pass(self.spin(slot), beam_width);
        self.spins.insert(slot, timer);
    }

    /// Puts `slot`'s spin timer back to 0.
    pub fn reset_spin(&mut self, slot: usize) {
        self.spins.remove(&slot);
    }

    /// Frees every slot, as a system change does, and keeps each slot's
    /// place in its layer, its roll and its spin timer (see the module
    /// docs).
    pub fn clear(&mut self) {
        self.held.clear();
    }
}

impl Combat {
    /// The slot a shot spawned now takes ([`ShotSlots::first_free`]), the
    /// shots in flight taking theirs.
    pub(super) fn first_free_slot(&self) -> Option<usize> {
        let taken = self.shots.iter().map(|shot| shot.slot);
        self.slots.first_free(taken, self.capped)
    }

    /// Puts `shot` in flight at the first free slot, as `_SpawnShot` does
    /// (@0x3e5ab-0x3e5ce), keeping the shots in slot order: its slot's
    /// sprite is put in its layer ([`ShotSlots::put`]). A ship's own shot
    /// (a ship source, not a sub-munition) writes its miss roll into the
    /// slot (@0x3e5e1-0x3e5f7, @0x3e89e); any other writes none, and by
    /// [`RuleKey::ReusedShotSlotRoll`](crate::RuleKey::ReusedShotSlotRoll)'s
    /// engine reading carries the roll the slot's last ship-fired shot
    /// left ([`ShotSlots::roll`]). It starts on the spin timer the slot
    /// holds ([`ShotSlots::spin`]), as `_SpawnShot` never writes +0x1c, by
    /// [`RuleKey::ReusedShotSlotSpin`](crate::RuleKey::ReusedShotSlotSpin)'s
    /// engine reading; by the other, on 0, the slot's put back to 0. The
    /// slot it took, if it found one.
    pub(super) fn put_in_flight(&mut self, mut shot: Shot) -> Option<usize> {
        let free = self.seat(&mut shot)?;
        self.insert_seated(&shot);
        Some(free)
    }

    /// Seats `shot` at the first free slot, as [`Combat::put_in_flight`]
    /// does, without putting it in flight: its slot, its layer, its spin
    /// timer and its miss roll, written or read. A sub-munition is aimed
    /// between the two ([`submunition::aim`](super::submunition::aim)),
    /// as `_SpawnShotSubmunitions` seeks only once `_SpawnShot` has
    /// seated it (@0x3f328, @0x3f3cf). The slot it took, if it found one.
    pub(super) fn seat(&mut self, shot: &mut Shot) -> Option<usize> {
        let free = self.first_free_slot()?;
        shot.slot = free;
        self.slots.put(free, shot.layer);
        if self.reused_spin != RuleSource::Engine {
            self.slots.reset_spin(free);
        }
        shot.spin_timer = self.slots.spin(free);
        if shot.generation == 0 && matches!(shot.source, Source::Ship(_)) {
            self.slots.write_roll(free, shot.miss_roll);
        } else if self.reused_roll == RuleSource::Engine {
            shot.miss_roll = self.slots.roll(free);
        }
        Some(free)
    }

    /// Puts `shot`, seated ([`Combat::seat`]), in flight, keeping the
    /// shots in slot order.
    pub(super) fn insert_seated(&mut self, shot: &Shot) {
        let at = self.shots.partition_point(|flying| flying.slot < shot.slot);
        self.shots.insert(at, *shot);
    }

    /// Moves the shot in flight in `slot` to `layer`, as
    /// `_PutSpriteInLayer` does ([`ShotSlots::put`]).
    pub(super) fn move_to_layer(&mut self, slot: usize, layer: ShotLayer) {
        self.slots.put(slot, layer);
        for shot in self.shots.iter_mut().filter(|shot| shot.slot == slot) {
            shot.layer = layer;
        }
    }

    /// The indices of the shots in flight in the order each layer keeps
    /// them, layer by layer, bottom first: by
    /// [`RuleKey::ShotOverlap`](crate::RuleKey::ShotOverlap)'s engine
    /// reading, the order their slots entered the layer
    /// ([`ShotSlots::rank`]); by the other, firing order.
    pub(super) fn entry_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.shots.len()).collect();
        let by_entry = self.shot_overlap == RuleSource::Engine;
        order.sort_by_key(|&index| {
            let shot = &self.shots[index];
            let entry = if by_entry {
                self.slots
                    .rank(shot.slot)
                    .map_or(i64::MAX, |(_, entry)| entry)
            } else {
                i64::from(shot.id.0)
            };
            (shot.layer, entry)
        });
        order
    }

    /// The shots in flight in the order they are drawn, bottom first: by
    /// layer ([`SpriteLayer`](crate::sprite_layer::SpriteLayer)), each
    /// layer in the order it keeps them (see the module docs).
    #[must_use]
    pub fn shots_drawn(&self) -> Vec<&Shot> {
        self.entry_order()
            .into_iter()
            .map(|index| &self.shots[index])
            .collect()
    }

    /// Has each shot layer drawn and walked in the order `source` reads
    /// ([`RuleKey::ShotOverlap`](crate::RuleKey::ShotOverlap)): by default,
    /// the engine's, the order its slots entered it.
    pub fn set_shot_overlap(&mut self, source: RuleSource) {
        self.shot_overlap = source;
    }

    /// The shot slots' history, each slot's place in its layer and its
    /// leftover roll, none held: what outlives the fight
    /// ([`SlotSprites::shots`](crate::ship_layer::SlotSprites::shots)).
    #[must_use]
    pub fn shot_slots(&self) -> ShotSlots {
        let mut history = self.slots.clone();
        history.clear();
        history
    }

    /// Starts the shot slots from `history`, the places and rolls a fight
    /// earlier in the run left ([`Combat::shot_slots`]), none held.
    pub fn set_shot_slots(&mut self, history: ShotSlots) {
        self.slots = history;
        self.slots.clear();
    }

    /// Has a sub-munition carry its slot's leftover miss roll as `source`
    /// reads
    /// ([`RuleKey::ReusedShotSlotRoll`](crate::RuleKey::ReusedShotSlotRoll)):
    /// by default, the engine's, it does; by the other reading, none.
    pub fn set_reused_shot_slot_roll(&mut self, source: RuleSource) {
        self.reused_roll = source;
    }

    /// Has a new shot start on its slot's leftover spin timer as `source`
    /// reads
    /// ([`RuleKey::ReusedShotSlotSpin`](crate::RuleKey::ReusedShotSlotSpin)):
    /// by default, the engine's, it does; by the other reading, on 0.
    pub fn set_reused_shot_slot_spin(&mut self, source: RuleSource) {
        self.reused_spin = source;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_free_slot_is_the_lowest_neither_taken_nor_held() {
        let mut slots = ShotSlots::new();
        assert_eq!(slots.first_free([], true), Some(0));
        assert_eq!(slots.first_free([0, 1, 3], true), Some(2));
        slots.hold(2);
        assert_eq!(slots.first_free([0, 1, 3], true), Some(4));
        assert_eq!(slots.first_free([1, 3], false), Some(0));
        slots.free(2);
        assert_eq!(slots.first_free([0, 1, 3], true), Some(2));
    }

    #[test]
    fn a_full_table_has_none_under_the_caps_and_grows_past_it_uncapped() {
        let mut slots = ShotSlots::new();
        let full: Vec<usize> = (0..MAX_SHOTS).collect();
        assert_eq!(slots.first_free(full.clone(), true), None);
        assert_eq!(slots.first_free(full.clone(), false), Some(MAX_SHOTS));
        slots.hold(MAX_SHOTS - 1);
        let held_out: Vec<usize> = (0..MAX_SHOTS - 1).collect();
        assert_eq!(slots.first_free(held_out, true), None, "held");
        let mut more = full;
        more.push(MAX_SHOTS);
        assert_eq!(slots.first_free(more, false), Some(MAX_SHOTS + 1));
    }

    #[test]
    fn the_slots_start_in_the_shot_layer_in_slot_order() {
        let slots = ShotSlots::new();
        let ranks: Vec<_> = (0..MAX_SHOTS).map(|slot| slots.rank(slot)).collect();
        assert!(ranks.iter().all(|rank| rank.unwrap().0 == ShotLayer::Shots));
        assert!(ranks.windows(2).all(|pair| pair[0] < pair[1]), "slot order");
    }

    #[test]
    fn a_put_into_the_layer_a_slot_is_in_keeps_its_place() {
        let mut slots = ShotSlots::new();
        slots.put(5, ShotLayer::Shots);
        assert_eq!(slots.rank(5), ShotSlots::new().rank(5));
        assert!(slots.rank(5) < slots.rank(6), "still under slot 6");
    }

    #[test]
    fn a_put_into_another_layer_appends_the_slot_at_its_tail() {
        let mut slots = ShotSlots::new();
        slots.put(3, ShotLayer::Homing);
        slots.put(1, ShotLayer::Homing);
        let (layer, three) = slots.rank(3).unwrap();
        assert_eq!(layer, ShotLayer::Homing);
        assert!(three < slots.rank(1).unwrap().1, "3 entered first");
        slots.put(3, ShotLayer::Homing);
        assert!(slots.rank(3) < slots.rank(1), "kept its place");
        slots.put(3, ShotLayer::Shots);
        slots.put(3, ShotLayer::Homing);
        assert!(slots.rank(1) < slots.rank(3), "back again, at the tail");
        slots.put(0, ShotLayer::Shots);
        assert!(slots.rank(0) < slots.rank(2), "0 never left");
    }

    #[test]
    fn a_shot_slot_moved_out_and_back_is_drawn_over_the_slots_that_stayed() {
        let mut slots = ShotSlots::new();
        slots.put(0, ShotLayer::PointDefence);
        slots.put(0, ShotLayer::Shots);
        assert!(slots.rank(127) < slots.rank(0));
    }

    #[test]
    fn freeing_a_slot_keeps_its_place_and_roll() {
        let mut slots = ShotSlots::new();
        slots.put(2, ShotLayer::Homing);
        slots.write_roll(2, Some(90));
        let rank = slots.rank(2);
        slots.hold(2);
        assert!(slots.is_held(2));
        slots.free(2);
        assert!(!slots.is_held(2));
        assert_eq!((slots.rank(2), slots.roll(2)), (rank, Some(90)));
    }

    #[test]
    fn clearing_frees_every_slot_and_keeps_the_places_and_rolls() {
        let mut slots = ShotSlots::new();
        slots.put(2, ShotLayer::Homing);
        slots.write_roll(4, Some(7));
        slots.hold(2);
        slots.hold(9);
        let mut kept = slots.clone();
        slots.clear();
        assert_eq!(slots.held().count(), 0);
        assert_eq!(slots.first_free([0, 1], true), Some(2));
        assert_eq!((slots.rank(2), slots.roll(4)), (kept.rank(2), Some(7)));
        kept.free(2);
        kept.free(9);
        assert_eq!(slots, kept, "only the holds go");
    }

    #[test]
    fn a_slot_past_the_table_enters_its_layer_at_the_tail_on_first_use() {
        let mut slots = ShotSlots::new();
        assert_eq!(slots.rank(MAX_SHOTS), None);
        slots.put(MAX_SHOTS, ShotLayer::Shots);
        assert!(slots.rank(MAX_SHOTS - 1) < slots.rank(MAX_SHOTS));
        slots.put(5, ShotLayer::Homing);
        slots.put(MAX_SHOTS + 1, ShotLayer::Homing);
        assert!(slots.rank(5) < slots.rank(MAX_SHOTS + 1));
    }

    #[test]
    fn a_slots_spin_timer_runs_on_its_spinning_shots_passes() {
        let mut slots = ShotSlots::new();
        assert_eq!(slots.spin(0), 0, "never used");
        slots.spin_pass(0, 3);
        slots.spin_pass(0, 3);
        assert_eq!(slots.spin(0), 2);
        assert_eq!(slots.spin(1), 0, "its own");
        slots.spin_pass(0, 3);
        assert_eq!(slots.spin(0), 0, "stepped at 3");
        slots.spin_pass(0, 3);
        slots.reset_spin(0);
        assert_eq!(slots.spin(0), 0, "reset");
    }

    #[test]
    fn freeing_and_clearing_keep_the_spin_timers() {
        let mut slots = ShotSlots::new();
        slots.spin_pass(4, 9);
        slots.hold(4);
        slots.free(4);
        assert_eq!(slots.spin(4), 1);
        slots.hold(4);
        slots.clear();
        assert_eq!(slots.spin(4), 1);
    }

    #[test]
    fn a_slot_holds_the_roll_its_last_ship_fired_shot_left() {
        let mut slots = ShotSlots::new();
        assert_eq!(slots.roll(0), None, "never used");
        slots.write_roll(0, Some(90));
        assert_eq!(slots.roll(0), Some(90));
        assert_eq!(slots.roll(1), None);
        slots.write_roll(0, None);
        assert_eq!(slots.roll(0), None, "a shot of no roll");
    }
}
