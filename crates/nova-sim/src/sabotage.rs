//! Sabotage outfits: the flag a sabotaged loadout sets, the clock that
//! sets them off in flight, and the `dësc`s a sabotaged player sees as it
//! dies.
//!
//! # The flag
//!
//! [`sabotage_flag`] is `_playerSabotaged` as `_ResetPlayerPrecalcedValues`
//! sets it whenever the loadout changes: cleared (@0xc3ff), then for each
//! owned outfit (count > 0, @0xc5b2) and each of its four mod slots, a
//! [`SABOTAGE_KILL`] slot sets it to 1 (@0xc5d4-0xc5e8), and a
//! [`SABOTAGE_BOMB`] slot sets it to 2 unless it is already set
//! (@0xc5f6-0xc60a). So a kill slot anywhere wins, and no slot's `ModVal`
//! is read. `_InitObjects` clears it too (@0x1d11f). The flag is a global
//! only that reset writes, so the engine caches it at its stand-in for the
//! reset ([`SabotageClock::refresh`]), and the clock's tick and the
//! death-time dialogs read the cache, not the loadout as it is now.
//!
//! # The clock
//!
//! [`SabotageClock`] is `_sabotageCounter` (a float; nothing else reads or
//! writes it, so it is never saved) with the cached flag.
//!
//! - **The refresh** (`_ResetPlayerPrecalcedValues`): each owned
//!   [`SABOTAGE_KILL`] or [`SABOTAGE_BOMB`] slot zeroes the count
//!   (@0xc5ed, @0xc614), and after the loop a set flag with a count at or
//!   below none seeds it with `Rand(100)` (@0xc73e-0xc769). So a refresh
//!   with the flag set always restarts the count at 0..99, and one with it
//!   clear leaves the count alone.
//! - **The tick** (`_HandlePlayer` @0x692e5-0x6934d): with the flag clear
//!   the count is zeroed (@0x69a0c-0x69a11); a count below none is seeded
//!   with `Rand(100)` and the tick ends (@0x692ff-0x6931f), a branch
//!   nothing in play reaches, ported anyway; a dying player's tick ends
//!   there (@0x69324-0x6932b); else the count grows by `_gSpeedMult`, 1.0
//!   a tick (@0x69331-0x6933e), and the flag goes off once it is above
//!   [`FUSE`] (@0x69342-0x6934d): 202 to 301 ticks after the refresh.
//!
//! *Ordering against the original.* Most of the engine's refresh points
//! have no [`Chance`] to draw on, so the refresh only marks the seed due,
//! and the next tick draws it before anything reads the count. The count
//! is never shown or saved, so drawing it a little later crosses no
//! boundary the player can see.
//!
//! What goes off: [`kill_notice`] names the outfit a [`Sabotage::Kill`]
//! blames, and [`bombs`] lists what a [`Sabotage::Bomb`] sets off, each
//! bomb's damage drawn by [`bomb_damage`]. The session does the rest (see
//! its `sabotage` module).
//!
//! ## Left out
//!
//! - `_warpSoundDone` (cleared @0x695ef, @0x69612 after each warp-up
//!   halt): it holds the original's warp-out until the warp-up sound ends
//!   (@0x6bbc5); the engine's jump is not timed by the sound, and the kill
//!   outfit cancels the jump anyway. Only the halts are ported.
//! - `_statusRedraw[1]` (@0x699fc-0x69a01): the engine's status display
//!   reads the session every frame, so there is nothing to mark.
//! - The jump's hyperspace stage: the original's clock runs on through
//!   it, and a kill outfit's +0x50 = -1.0 cancels the warp too. The engine
//!   steps nothing of the session's fight while `Jump::Hyperspace` is
//!   under way, so the clock stands still through it and only a pre-jump
//!   is cancelled. This waits on the engine ticking the warp stage
//!   (`ship-handling-fidelity/phase-8-warp-out`).
//!
//! # The death-time dialogs
//!
//! [`death_descs`] lists the `dësc`s `_HandlePlayer` shows once the
//! player's death hold expires (@0x68739-0x68832), each in its own text
//! dialog over the stopped flight scene.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use crate::catalog::{OutfitId, OutfitRecord};
use crate::chance::Chance;
use crate::combat::weapon::Explosion;

/// The count past which the sabotage clock goes off (the float
/// @0xdd95c, `_HandlePlayer` @0x69342-0x6934d).
pub const FUSE: f32 = 300.0;
/// The sides of the clock's seed, `Rand(100)` (`_ResetPlayerPrecalcedValues`
/// @0xc758, `_HandlePlayer` @0x69308).
pub const SEED_SIDES: u32 = 100;
/// The share of the armour capacity a bomb's damage roll takes (the
/// double @0xdd128, `_HandlePlayer` @0x69912).
pub const DAMAGE_SHARE: f64 = 0.5;
/// What a bomb's damage roll adds to its share (the double @0xdd0c0,
/// `_HandlePlayer` @0x6991a).
pub const DAMAGE_PLUS: f64 = 1.0;
/// The bomb slot `ModVal`s that set off a `bööm`, `ModVal` - 128
/// (`_HandlePlayer` @0x698bc-0x698c8: `addl $-0x80`, then an unsigned
/// `cmpw $0x3f`).
pub const BOOM_MOD_VALS: RangeInclusive<i16> = 128..=191;

/// The `ModType` of a sabotage slot that kills the player
/// (`_ResetPlayerPrecalcedValues` @0xc5d4), and whose `ModVal` names the
/// `dësc` shown as the player dies (`_HandlePlayer` @0x68772).
pub const SABOTAGE_KILL: i16 = 47;

/// The `ModType` of a sabotage slot that sets off bombs
/// (`_ResetPlayerPrecalcedValues` @0xc5f6-0xc60a).
pub const SABOTAGE_BOMB: i16 = 50;

/// `_playerSabotaged`: what a loadout's sabotage outfits do (the
/// original's 0, 1 and 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sabotage {
    /// No owned outfit has a sabotage slot (0).
    None,
    /// An owned outfit has a [`SABOTAGE_KILL`] slot (1).
    Kill,
    /// Owned outfits have [`SABOTAGE_BOMB`] slots but none a
    /// [`SABOTAGE_KILL`] one (2).
    Bomb,
}

/// The sabotage flag of the outfits `owned`, read from `records` (see the
/// module docs).
#[must_use]
pub fn sabotage_flag(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> Sabotage {
    let slots: Vec<i16> = held(owned, records)
        .flat_map(|record| record.mods.map(|(mod_type, _)| mod_type))
        .collect();
    if slots.contains(&SABOTAGE_KILL) {
        Sabotage::Kill
    } else if slots.contains(&SABOTAGE_BOMB) {
        Sabotage::Bomb
    } else {
        Sabotage::None
    }
}

/// The `dësc` IDs a dying sabotaged player sees, in order
/// (`_HandlePlayer` @0x68739-0x68832).
///
/// Nothing unless `flag`, the flag the last refresh cached, is set (the
/// gate @0x6873e reads `_playerSabotaged`). So an outfit gained since
/// the last refresh adds nothing. Then, for each outfit in
/// index order (@0x68828, whatever `records`' order) that `owned` holds
/// at least once (@0x6875a), the `ModVal` of its first
/// [`SABOTAGE_KILL`] slot with a `ModVal` above 0 (@0x68772, @0x6877d):
/// that slot's `dësc`, not the outfit's own (@0x687b7). An outfit adds
/// one at most, whatever its count (@0x68807). The loop plays no sound.
#[must_use]
pub fn death_descs(
    flag: Sabotage,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
) -> Vec<i16> {
    if flag == Sabotage::None {
        return Vec::new();
    }
    in_index_order(owned, records)
        .into_iter()
        .filter_map(|record| {
            record
                .mods
                .iter()
                .find(|&&(mod_type, mod_val)| mod_type == SABOTAGE_KILL && mod_val > 0)
                .map(|&(_, mod_val)| mod_val)
        })
        .collect()
}

/// `_playerSabotaged` and `_sabotageCounter` (see the module docs'
/// **The clock**): the flag as the last refresh cached it, the count, and
/// whether the refresh's seed is still to be drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SabotageClock {
    flag: Sabotage,
    counter: f32,
    seed_due: bool,
}

impl Default for SabotageClock {
    /// No flag, counting nothing, as `_InitObjects` leaves them.
    fn default() -> Self {
        Self {
            flag: Sabotage::None,
            counter: 0.0,
            seed_due: false,
        }
    }
}

impl SabotageClock {
    /// The flag the last refresh cached.
    #[must_use]
    pub fn flag(&self) -> Sabotage {
        self.flag
    }

    /// The count, the seed not yet drawn while it is due.
    #[must_use]
    pub fn counter(&self) -> f32 {
        self.counter
    }

    /// `_ResetPlayerPrecalcedValues`' part: caches the flag of the outfits
    /// `owned`, read from `records`, and with it set zeroes the count and
    /// makes the seed due; with it clear, leaves the count and drops any
    /// due seed (see the module docs).
    pub fn refresh(&mut self, owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) {
        self.flag = sabotage_flag(owned, records);
        self.seed_due = self.flag != Sabotage::None;
        if self.seed_due {
            self.counter = 0.0;
        }
    }

    /// `_HandlePlayer`'s tick of the clock (@0x692e5-0x6934d) for a player
    /// `dying` or not, drawing on `chance`: the flag that goes off this
    /// tick, if any (see the module docs).
    pub fn tick(&mut self, dying: bool, chance: &mut dyn Chance) -> Option<Sabotage> {
        if std::mem::take(&mut self.seed_due) {
            self.counter = seed(chance);
        }
        if self.flag == Sabotage::None {
            self.counter = 0.0;
            return None;
        }
        if self.counter < 0.0 {
            self.counter = seed(chance);
            return None;
        }
        if dying {
            return None;
        }
        self.counter += 1.0;
        (self.counter > FUSE).then_some(self.flag)
    }

    /// Sets the count, as nothing in play does: for the branch that
    /// seeds a negative one.
    #[cfg(test)]
    pub(crate) fn set_counter(&mut self, counter: f32) {
        self.counter = counter;
    }
}

/// The clock's seed, `Rand(100)` (0..99).
fn seed(chance: &mut dyn Chance) -> f32 {
    // Below 100, so exact in an `f32`.
    #[allow(clippy::cast_precision_loss)]
    let seed = chance.below(SEED_SIDES) as f32;
    seed
}

/// The outfit a [`Sabotage::Kill`] names as it goes off
/// (`_HandlePlayer` @0x693c8-0x69578): the first, in index order, that
/// `owned` holds at least once (@0x693e3) and that has a
/// [`SABOTAGE_KILL`] slot, whatever its `ModVal` (@0x693f8), skipping one
/// whose `LCName` (owned once) or `LCPlural` (more) is empty (@0x6940b,
/// @0x694cb); with whether it is owned once. None when no such outfit is
/// named.
#[must_use]
pub fn kill_notice(
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
) -> Option<(OutfitId, bool)> {
    in_index_order(owned, records)
        .into_iter()
        .filter(|record| has_slot(record, SABOTAGE_KILL))
        .find_map(|record| {
            let one = owned.get(&record.id) == Some(&1);
            let name = if one {
                &record.lc_name
            } else {
                &record.lc_plural
            };
            (!name.is_empty()).then_some((record.id, one))
        })
}

/// One bomb outfit as a [`Sabotage::Bomb`] sets it off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bomb {
    /// The outfit, named on the message line and removed.
    pub outfit: OutfitId,
    /// Whether the player owns exactly one.
    pub one: bool,
    /// The `bööm` its first [`SABOTAGE_BOMB`] slot sets off, `ModVal` -
    /// 128, when that `ModVal` is in [`BOOM_MOD_VALS`].
    pub explosion: Option<Explosion>,
}

/// The bombs a [`Sabotage::Bomb`] sets off, in order (`_HandlePlayer`
/// @0x696a7-0x699f6): each outfit, in index order, that `owned` holds at
/// least once (@0x696df) and that has a [`SABOTAGE_BOMB`] slot, read from
/// its first one (@0x696f6), whatever its names.
#[must_use]
pub fn bombs(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> Vec<Bomb> {
    in_index_order(owned, records)
        .into_iter()
        .filter_map(|record| {
            let &(_, mod_val) = record
                .mods
                .iter()
                .find(|&&(mod_type, _)| mod_type == SABOTAGE_BOMB)?;
            Some(Bomb {
                outfit: record.id,
                one: owned.get(&record.id) == Some(&1),
                explosion: BOOM_MOD_VALS
                    .contains(&mod_val)
                    .then(|| Explosion::decode(mod_val - BOOM_MOD_VALS.start()))
                    .flatten(),
            })
        })
        .collect()
}

/// A bomb's damage to a ship of armour capacity `capacity`, drawn on
/// `chance` (`_HandlePlayer` @0x698ff-0x69988): `Rand` of `capacity` x
/// [`DAMAGE_SHARE`] + [`DAMAGE_PLUS`], worked in doubles and truncated
/// (`cvttsd2si` @0x69922), plus `capacity`, worked in floats and
/// truncated (`cvttss2si` @0x69984). So `capacity` to `capacity` +
/// floor(`capacity` / 2). The original's `cwtl`s keep 16 bits of each;
/// the engine keeps the whole number.
#[must_use]
pub fn bomb_damage(capacity: f32, chance: &mut dyn Chance) -> f32 {
    // Armour capacities are far below `u32::MAX`, and never below none.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let sides = f64::from(capacity)
        .mul_add(DAMAGE_SHARE, DAMAGE_PLUS)
        .trunc()
        .max(1.0) as u32;
    // A draw below an armour capacity is exact in an `f32`.
    #[allow(clippy::cast_precision_loss)]
    let draw = chance.below(sides) as f32;
    (draw + capacity).trunc()
}

/// Whether `record` has a slot of `mod_type`.
fn has_slot(record: &OutfitRecord, mod_type: i16) -> bool {
    record.mods.iter().any(|&(kind, _)| kind == mod_type)
}

/// The records of the outfits `owned` holds at least once, in index order.
fn in_index_order<'a>(
    owned: &'a BTreeMap<OutfitId, u16>,
    records: &'a [OutfitRecord],
) -> Vec<&'a OutfitRecord> {
    let mut held: Vec<&OutfitRecord> = held(owned, records).collect();
    held.sort_by_key(|record| record.id);
    held
}

/// The records of the outfits `owned` holds at least once.
fn held<'a>(
    owned: &'a BTreeMap<OutfitId, u16>,
    records: &'a [OutfitRecord],
) -> impl Iterator<Item = &'a OutfitRecord> + 'a {
    records
        .iter()
        .filter(|record| owned.get(&record.id).is_some_and(|&count| count > 0))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::BoomId;
    use crate::testkit::{Draws, outfit};

    fn owning(counts: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        counts.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    #[test]
    fn the_flag_is_none_when_the_only_sabotage_outfits_are_owned_zero_times() {
        let records = [
            outfit(128, &[(SABOTAGE_KILL, 5000)]),
            outfit(129, &[(SABOTAGE_BOMB, 1)]),
        ];
        let owned = owning(&[(128, 0), (129, 0)]);
        assert_eq!(sabotage_flag(&owned, &records), Sabotage::None);
        assert_eq!(sabotage_flag(&owning(&[]), &records), Sabotage::None);
    }

    #[test]
    fn a_kill_outfit_wins_over_a_bomb_outfit_before_it() {
        let records = [
            outfit(128, &[(SABOTAGE_BOMB, 1)]),
            outfit(129, &[(1, 3), (SABOTAGE_KILL, 5000)]),
        ];
        let owned = owning(&[(128, 1), (129, 1)]);
        assert_eq!(sabotage_flag(&owned, &records), Sabotage::Kill);
    }

    #[test]
    fn bomb_outfits_alone_give_the_bomb_flag() {
        let records = [
            outfit(128, &[(SABOTAGE_BOMB, 1)]),
            outfit(129, &[(SABOTAGE_KILL, 5000)]),
        ];
        let owned = owning(&[(128, 2)]);
        assert_eq!(sabotage_flag(&owned, &records), Sabotage::Bomb);
    }

    #[test]
    fn the_flag_ignores_mod_val() {
        let records = [outfit(128, &[(SABOTAGE_KILL, 0)])];
        assert_eq!(
            sabotage_flag(&owning(&[(128, 1)]), &records),
            Sabotage::Kill
        );
    }

    /// The `dësc`s of `owned`, the flag cached from it.
    fn descs(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> Vec<i16> {
        death_descs(sabotage_flag(owned, records), owned, records)
    }

    #[test]
    fn death_descs_are_empty_while_the_cached_flag_is_clear() {
        let records = [outfit(128, &[(SABOTAGE_KILL, 5000)])];
        let owned = owning(&[(128, 1)]);
        assert_eq!(
            death_descs(Sabotage::None, &owned, &records),
            Vec::<i16>::new()
        );
        assert_eq!(death_descs(Sabotage::Bomb, &owned, &records), [5000]);
    }

    #[test]
    fn death_descs_are_the_slots_mod_val_not_the_outfits_own_desc() {
        // An outfit's own `dësc` is 3000 + its index (128 here).
        let records = [outfit(128, &[(SABOTAGE_KILL, 5000)])];
        assert_eq!(descs(&owning(&[(128, 1)]), &records), [5000]);
    }

    #[test]
    fn death_descs_list_an_outfit_once_whatever_its_count() {
        let records = [outfit(128, &[(SABOTAGE_KILL, 5000)])];
        assert_eq!(descs(&owning(&[(128, 3)]), &records), [5000]);
    }

    #[test]
    fn death_descs_skip_a_non_positive_mod_val_for_a_later_slot_on_the_same_outfit() {
        let records = [outfit(
            128,
            &[
                (SABOTAGE_KILL, 0),
                (SABOTAGE_KILL, -5),
                (SABOTAGE_KILL, 700),
            ],
        )];
        assert_eq!(descs(&owning(&[(128, 1)]), &records), [700]);
    }

    #[test]
    fn death_descs_take_only_the_first_qualifying_slot() {
        let records = [outfit(
            128,
            &[
                (SABOTAGE_BOMB, 9),
                (SABOTAGE_KILL, 700),
                (SABOTAGE_KILL, 701),
            ],
        )];
        assert_eq!(descs(&owning(&[(128, 1)]), &records), [700]);
    }

    #[test]
    fn death_descs_list_outfits_in_index_order() {
        let records = [
            outfit(130, &[(SABOTAGE_KILL, 730)]),
            outfit(128, &[(SABOTAGE_KILL, 728)]),
            outfit(129, &[(SABOTAGE_KILL, 729)]),
        ];
        let owned = owning(&[(130, 1), (128, 1)]);
        assert_eq!(descs(&owned, &records), [728, 730]);
    }

    #[test]
    fn death_descs_are_empty_for_bombs_only_and_without_sabotage() {
        let records = [
            outfit(128, &[(SABOTAGE_BOMB, 728)]),
            outfit(129, &[(2, 729)]),
        ];
        assert_eq!(descs(&owning(&[(128, 1)]), &records), Vec::<i16>::new());
        assert_eq!(descs(&owning(&[(129, 1)]), &records), Vec::<i16>::new());
    }

    /// A clock refreshed with one of `records`' outfits owned once.
    fn refreshed(records: &[OutfitRecord], id: i16) -> SabotageClock {
        let mut clock = SabotageClock::default();
        clock.refresh(&owning(&[(id, 1)]), records);
        clock
    }

    #[test]
    fn a_refresh_with_a_sabotage_outfit_seeds_below_100_and_the_tick_counts_on_from_the_draw() {
        for mod_type in [SABOTAGE_KILL, SABOTAGE_BOMB] {
            let records = [outfit(128, &[(1, 3), (mod_type, 0)])];
            let mut clock = refreshed(&records, 128);
            clock.set_counter(250.0);
            clock.refresh(&owning(&[(128, 1)]), &records);
            assert_eq!(clock.counter(), 0.0, "zeroed");
            let mut draws = Draws::of(&[37]);
            assert_eq!(clock.tick(false, &mut draws), None);
            assert_eq!(draws.asked, [100]);
            assert_eq!(clock.counter(), 38.0, "seeded 37, and counted");
            assert_eq!(clock.tick(false, &mut draws), None);
            assert_eq!(draws.asked, [100], "seeded once");
            assert_eq!(clock.counter(), 39.0);
        }
        assert_eq!(SEED_SIDES, 100);
    }

    #[test]
    fn a_refresh_without_a_sabotage_outfit_leaves_the_counter_alone() {
        let records = [outfit(128, &[(1, 3)])];
        let mut clock = refreshed(&records, 128);
        clock.set_counter(250.0);
        clock.refresh(&owning(&[(128, 1)]), &records);
        assert_eq!(clock.flag(), Sabotage::None);
        assert_eq!(clock.counter(), 250.0);
    }

    #[test]
    fn a_refresh_that_clears_the_flag_drops_a_due_seed() {
        let records = [outfit(128, &[(SABOTAGE_KILL, 0)])];
        let mut clock = refreshed(&records, 128);
        assert_eq!(clock.flag(), Sabotage::Kill);
        clock.refresh(&owning(&[]), &records);
        assert_eq!(clock.flag(), Sabotage::None);
        let mut draws = Draws::of(&[]);
        assert_eq!(clock.tick(false, &mut draws), None);
        assert_eq!(draws.asked, Vec::<u32>::new(), "no seed drawn");
    }

    #[test]
    fn the_tick_zeroes_the_counter_when_the_flag_is_clear() {
        let mut clock = SabotageClock::default();
        clock.set_counter(400.0);
        let mut draws = Draws::of(&[]);
        assert_eq!(clock.tick(false, &mut draws), None);
        assert_eq!(clock.counter(), 0.0);
        assert_eq!(draws.asked, Vec::<u32>::new());
    }

    #[test]
    fn a_negative_counter_is_seeded_and_the_tick_ends_there() {
        let records = [outfit(128, &[(SABOTAGE_KILL, 0)])];
        let mut clock = refreshed(&records, 128);
        clock.tick(false, &mut Draws::of(&[0]));
        clock.set_counter(-0.5);
        let mut draws = Draws::of(&[42]);
        assert_eq!(clock.tick(false, &mut draws), None);
        assert_eq!(draws.asked, [100]);
        assert_eq!(clock.counter(), 42.0, "seeded, not counted");
        clock.set_counter(0.0);
        let mut draws = Draws::of(&[]);
        clock.tick(false, &mut draws);
        assert_eq!(draws.asked, Vec::<u32>::new(), "none is no negative");
        assert_eq!(clock.counter(), 1.0);
    }

    #[test]
    fn a_dying_tick_draws_a_due_seed_but_does_not_count() {
        let records = [outfit(128, &[(SABOTAGE_BOMB, 0)])];
        let mut clock = refreshed(&records, 128);
        let mut draws = Draws::of(&[20]);
        assert_eq!(clock.tick(true, &mut draws), None);
        assert_eq!(draws.asked, [100]);
        assert_eq!(clock.counter(), 20.0);
        clock.set_counter(300.5);
        assert_eq!(clock.tick(true, &mut draws), None, "no count, no firing");
        assert_eq!(clock.counter(), 300.5);
    }

    #[test]
    fn the_clock_fires_on_the_first_tick_above_300_and_not_at_300() {
        for (mod_type, flag) in [
            (SABOTAGE_KILL, Sabotage::Kill),
            (SABOTAGE_BOMB, Sabotage::Bomb),
        ] {
            let records = [outfit(128, &[(mod_type, 0)])];
            let mut clock = refreshed(&records, 128);
            let mut draws = Draws::of(&[98]);
            assert_eq!(clock.tick(false, &mut draws), None);
            let mut fired = None;
            let mut ticks = 1;
            while fired.is_none() {
                assert!(clock.counter() <= FUSE, "{}", clock.counter());
                fired = clock.tick(false, &mut draws);
                ticks += 1;
            }
            assert_eq!(fired, Some(flag));
            assert_eq!(clock.counter(), 301.0, "300 is not past the fuse");
            assert_eq!(ticks, 203, "seeded 98: 99, then 202 more to 301");
            assert_eq!(clock.tick(false, &mut draws), Some(flag), "it stays lit");
        }
        assert_eq!(FUSE, 300.0);
    }

    #[test]
    fn the_kill_notice_names_the_first_owned_kill_outfit_in_index_order() {
        let records = [
            outfit(131, &[(SABOTAGE_KILL, 0)]),
            outfit(129, &[(1, 1), (SABOTAGE_KILL, 0)]),
            outfit(128, &[(SABOTAGE_KILL, 0)]),
            outfit(130, &[(SABOTAGE_BOMB, 0)]),
        ];
        let owned = owning(&[(131, 1), (129, 2), (130, 1), (128, 0)]);
        assert_eq!(kill_notice(&owned, &records), Some((OutfitId(129), false)));
        let owned = owning(&[(131, 1), (130, 1)]);
        assert_eq!(kill_notice(&owned, &records), Some((OutfitId(131), true)));
    }

    #[test]
    fn the_kill_notice_skips_an_outfit_whose_name_for_its_count_is_empty() {
        let unnamed = |id, singular: &str, plural: &str| OutfitRecord {
            lc_name: singular.to_owned(),
            lc_plural: plural.to_owned(),
            ..outfit(id, &[(SABOTAGE_KILL, 0)])
        };
        let records = [
            unnamed(128, "", "plurals"),
            unnamed(129, "one", ""),
            outfit(130, &[(SABOTAGE_KILL, 0)]),
        ];
        let owned = owning(&[(128, 1), (129, 2), (130, 1)]);
        assert_eq!(kill_notice(&owned, &records), Some((OutfitId(130), true)));
        let owned = owning(&[(128, 2), (129, 2), (130, 1)]);
        assert_eq!(
            kill_notice(&owned, &records),
            Some((OutfitId(128), false)),
            "its plural is named"
        );
        let owned = owning(&[(129, 1), (130, 1)]);
        assert_eq!(
            kill_notice(&owned, &records),
            Some((OutfitId(129), true)),
            "its singular is named"
        );
    }

    #[test]
    fn the_kill_notice_ignores_mod_val_and_is_none_without_a_named_outfit() {
        let records = [
            outfit(128, &[(SABOTAGE_KILL, -7)]),
            OutfitRecord {
                lc_name: String::new(),
                ..outfit(129, &[(SABOTAGE_KILL, 5000)])
            },
            outfit(130, &[(SABOTAGE_BOMB, 0)]),
        ];
        assert_eq!(
            kill_notice(&owning(&[(128, 1)]), &records),
            Some((OutfitId(128), true))
        );
        assert_eq!(kill_notice(&owning(&[(129, 1), (130, 1)]), &records), None);
        assert_eq!(kill_notice(&owning(&[]), &records), None);
    }

    #[test]
    fn bombs_go_off_in_index_order_from_each_outfits_first_bomb_slot() {
        let records = [
            outfit(131, &[(SABOTAGE_BOMB, 130)]),
            outfit(129, &[(1, 140), (SABOTAGE_BOMB, 128), (SABOTAGE_BOMB, 129)]),
            outfit(128, &[(SABOTAGE_KILL, 0)]),
            OutfitRecord {
                lc_name: String::new(),
                lc_plural: String::new(),
                ..outfit(130, &[(SABOTAGE_BOMB, 0)])
            },
            outfit(132, &[(SABOTAGE_BOMB, 128)]),
        ];
        let owned = owning(&[(131, 1), (129, 3), (128, 1), (130, 1), (132, 0)]);
        let boom = |id| {
            Some(Explosion {
                boom: BoomId(id),
                extra: false,
            })
        };
        assert_eq!(
            bombs(&owned, &records),
            [
                Bomb {
                    outfit: OutfitId(129),
                    one: false,
                    explosion: boom(128),
                },
                Bomb {
                    outfit: OutfitId(130),
                    one: true,
                    explosion: None,
                },
                Bomb {
                    outfit: OutfitId(131),
                    one: true,
                    explosion: boom(130),
                },
            ]
        );
    }

    #[test]
    fn a_bomb_explodes_only_for_mod_val_128_to_191() {
        let explosion = |mod_val| {
            let records = [outfit(128, &[(SABOTAGE_BOMB, mod_val)])];
            bombs(&owning(&[(128, 1)]), &records)[0]
                .explosion
                .map(|explosion| explosion.boom)
        };
        assert_eq!(explosion(127), None);
        assert_eq!(explosion(128), Some(BoomId(128)));
        assert_eq!(explosion(191), Some(BoomId(191)));
        assert_eq!(explosion(192), None);
        assert_eq!(explosion(-1), None);
        assert_eq!(explosion(1128), None, "no extra explosions");
    }

    #[test]
    fn bomb_damage_draws_below_half_the_capacity_plus_one_and_adds_the_capacity() {
        let mut draws = Draws::of(&[0, 150]);
        assert_eq!(bomb_damage(300.0, &mut draws), 300.0);
        assert_eq!(bomb_damage(300.0, &mut draws), 450.0);
        assert_eq!(draws.asked, [151, 151]);
        let mut draws = Draws::of(&[2]);
        assert_eq!(
            bomb_damage(5.75, &mut draws),
            7.0,
            "Rand(trunc(3.875)), and trunc(7.75)"
        );
        assert_eq!(draws.asked, [3]);
        let mut draws = Draws::of(&[0]);
        assert_eq!(bomb_damage(0.0, &mut draws), 0.0);
        assert_eq!(draws.asked, [1]);
        assert_eq!((DAMAGE_SHARE, DAMAGE_PLUS), (0.5, 1.0));
    }
}
