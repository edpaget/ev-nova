//! The stellar slot keys' block of `_HandlePlayer` (@0x69d2b-0x69f17),
//! polled once a frame ([`SlotBlock::frame`]), whole:
//!
//! - While a jump is under way (ship +0x50 > 0) the block is skipped,
//!   its latch's clearing included (@0x69d2b-0x69d32).
//! - 1-4 (keycodes 0x12-0x15, @0x69d38-0x69d8d), then F1-F4 (key table
//!   +0x34..+0x3a, @0x69d90-0x69daa), each found down overriding the one
//!   before; then F5 (0x60, @0x69dac-0x69dba) or, with F5 up, Command and
//!   the top-row 5 (0x37, 0x17, @0x69dbc-0x69dda) pick the nearest. So of
//!   keys down together F5 and Command-5 win, then the F keys, then the
//!   highest. No modifier is read for the others: Command-1 is 1.
//! - With none down the latch (0xdede0) is cleared (@0x69ddc-0x69de7);
//!   with one down, the latch set and, if it was clear, the pick made
//!   (@0x69e15-0x69e23), so one pick a frame, and none while any of the
//!   block's keys stays down.
//! - While the escort menu shows, `_QuickKeyCheck` reports 1-5 up
//!   (@0xa106-0xa124), so the block never finds 1-4 or the 5 down.
//!
//! The pick itself, a slot of the nav defaults or the nearest stellar,
//! its owner check and its sound, is `nova_sim::Session::select_stellar`'s.
//!
//! A press is the OS's ([`SlotBlock::press`]): it follows a release, so
//! it clears the latch unless another of the block's keys is still down,
//! and it counts as down at the next frame even if let go before it, as
//! the frame's poll would see a tap. The frame runs at the top of the
//! next tick ([`FlightView::held_keys_frame`](super::FlightView)), before
//! any frame is drawn, so all the presses of one frame make one pick.

use std::collections::HashSet;

use nova_sim::StellarPick;

use super::{NEAREST_STELLAR_COMMAND_KEY, NEAREST_STELLAR_KEY, STELLAR_SLOT_KEYS};
use crate::Key;
use crate::flight::escorts::GROUP_KEYS;

/// The block's latch (0xdede0) and the presses since its last frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct SlotBlock {
    latch: bool,
    presses: Vec<Key>,
}

impl SlotBlock {
    /// An OS press of `key`, one of the block's, with the keys `held`
    /// before it and the escort menu open or not: the latch cleared unless
    /// another of the block's keys is down, and `key` down at the next
    /// frame, the 5 with the Command it was pressed with.
    pub(super) fn press(&mut self, key: Key, held: &HashSet<Key>, menu_open: bool) {
        if pick(|other| other != key && held.contains(&other), menu_open).is_none() {
            self.latch = false;
        }
        self.presses.push(key);
        if key == NEAREST_STELLAR_COMMAND_KEY && held.contains(&Key::Command) {
            self.presses.push(Key::Command);
        }
    }

    /// One frame of the block, with the keys `held` and the presses since
    /// the last, the escort menu open or not, `polled` unless a jump is
    /// under way or the session refuses the nav keys: the stellar to pick,
    /// if any (see the module docs).
    pub(super) fn frame(
        &mut self,
        held: &HashSet<Key>,
        menu_open: bool,
        polled: bool,
    ) -> Option<StellarPick> {
        let presses = std::mem::take(&mut self.presses);
        if !polled {
            return None;
        }
        let Some(picked) = pick(
            |key| held.contains(&key) || presses.contains(&key),
            menu_open,
        ) else {
            self.latch = false;
            return None;
        };
        if std::mem::replace(&mut self.latch, true) {
            None
        } else {
            Some(picked)
        }
    }

    /// Whether the block holds input a later frame will act on, with the
    /// keys `held`, the escort menu open or not and `polled` as
    /// [`SlotBlock::frame`] takes them: whether one more frame would
    /// change it. So a press is queued until the frame that picks or drops
    /// it, and a pick's latch until the frame that clears it; a key held
    /// down after its pick is not. A pick always sets the latch, so a frame
    /// that would pick changes the block too.
    ///
    /// It asks the block's fixed point, not which field holds the presses,
    /// so it stays true for as many frames as the block takes to act.
    #[cfg(any(test, feature = "fixture"))]
    pub(super) fn queued(&self, held: &HashSet<Key>, menu_open: bool, polled: bool) -> bool {
        let mut next = self.clone();
        next.frame(held, menu_open, polled);
        next != *self
    }
}

/// The stellar the block picks with the keys `down` and the escort menu
/// open or not, by its order (see the module docs); `None` when it finds
/// none of its keys down.
pub(super) fn pick(down: impl Fn(Key) -> bool, menu_open: bool) -> Option<StellarPick> {
    if down(NEAREST_STELLAR_KEY)
        || (block_polls(NEAREST_STELLAR_COMMAND_KEY, &down, menu_open)
            && down(NEAREST_STELLAR_COMMAND_KEY))
    {
        return Some(StellarPick::Nearest);
    }
    let found = |column: usize| {
        STELLAR_SLOT_KEYS
            .iter()
            .rposition(|keys| block_polls(keys[column], &down, menu_open) && down(keys[column]))
    };
    found(1).or_else(|| found(0)).map(StellarPick::Slot)
}

/// Whether `key` is one of the block's: 1-4, F1-F5 or the top-row 5.
pub(super) fn is_block_key(key: Key) -> bool {
    key == NEAREST_STELLAR_KEY
        || key == NEAREST_STELLAR_COMMAND_KEY
        || STELLAR_SLOT_KEYS.iter().any(|keys| keys.contains(&key))
}

/// Whether the block, polling, would find `key` down were it down, with
/// the other keys `down` and the escort menu open or not: F1-F5 always;
/// 1-4 while the menu is shut; the 5 while the menu is shut, Command is
/// down and F5 up. Its `_QuickKeyCheck` then sets `_quickKeyCheckResult`
/// (@0xa145-0xa14c), which the floating map's zoom keys read.
pub(super) fn block_polls(key: Key, down: impl Fn(Key) -> bool, menu_open: bool) -> bool {
    if key == NEAREST_STELLAR_KEY || STELLAR_SLOT_KEYS.iter().any(|keys| keys[1] == key) {
        return true;
    }
    if menu_open {
        return false;
    }
    if key == NEAREST_STELLAR_COMMAND_KEY {
        return down(Key::Command) && !down(NEAREST_STELLAR_KEY);
    }
    STELLAR_SLOT_KEYS.iter().any(|keys| keys[0] == key)
}

/// Whether the escort menu's loop polls `key`: 1-5 while the menu shows,
/// read unmasked through `_QuickKeyCheckPlusEscorts`, which sets
/// `_quickKeyCheckResult` for a key down (@0x6df4a-0x6e186,
/// @0xa18a-0xa191), during a jump too.
pub(super) fn escort_loop_polls(key: Key, menu_open: bool) -> bool {
    menu_open && GROUP_KEYS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: Key = Key::Char('1');
    const TWO: Key = Key::Char('2');
    const THREE: Key = Key::Char('3');
    const FIVE: Key = Key::Char('5');

    fn set(keys: &[Key]) -> HashSet<Key> {
        keys.iter().copied().collect()
    }

    fn pick_of(keys: &[Key], menu_open: bool) -> Option<StellarPick> {
        let keys = set(keys);
        pick(|key| keys.contains(&key), menu_open)
    }

    #[test]
    fn each_key_picks_its_slot_or_the_nearest() {
        for (slot, keys) in STELLAR_SLOT_KEYS.iter().enumerate() {
            for &key in keys {
                assert_eq!(pick_of(&[key], false), Some(StellarPick::Slot(slot)));
            }
        }
        assert_eq!(
            pick_of(&[NEAREST_STELLAR_KEY], false),
            Some(StellarPick::Nearest)
        );
        assert_eq!(
            pick_of(&[Key::Command, FIVE], false),
            Some(StellarPick::Nearest)
        );
        assert_eq!(pick_of(&[FIVE], false), None, "5 needs Command");
        assert_eq!(pick_of(&[Key::Command], false), None);
        assert_eq!(pick_of(&[Key::Char('6'), Key::Alt], false), None);
        assert_eq!(pick_of(&[], false), None);
    }

    #[test]
    fn the_open_menu_hides_1_to_5_from_the_block() {
        assert_eq!(pick_of(&[TWO], true), None);
        assert_eq!(pick_of(&[Key::Command, FIVE], true), None);
        assert_eq!(
            pick_of(&[Key::Function(2)], true),
            Some(StellarPick::Slot(1))
        );
        assert_eq!(
            pick_of(&[NEAREST_STELLAR_KEY], true),
            Some(StellarPick::Nearest)
        );
    }

    #[test]
    fn of_keys_down_together_f5_wins_then_the_f_keys_then_the_highest() {
        // @0x69d38-0x69dda: each key found down overrides the last.
        assert_eq!(pick_of(&[ONE, THREE], false), Some(StellarPick::Slot(2)));
        assert_eq!(
            pick_of(&[THREE, Key::Function(1)], false),
            Some(StellarPick::Slot(0))
        );
        assert_eq!(
            pick_of(&[Key::Function(1), Key::Function(4)], false),
            Some(StellarPick::Slot(3))
        );
        assert_eq!(
            pick_of(&[Key::Function(4), NEAREST_STELLAR_KEY], false),
            Some(StellarPick::Nearest)
        );
        assert_eq!(
            pick_of(&[Key::Function(4), Key::Command, FIVE], false),
            Some(StellarPick::Nearest)
        );
    }

    #[test]
    fn a_frame_picks_once_until_a_frame_with_no_key_down() {
        let mut block = SlotBlock::default();
        let none = set(&[]);
        block.press(TWO, &none, false);
        assert_eq!(
            block.frame(&none, false, true),
            Some(StellarPick::Slot(1)),
            "a tap"
        );
        block.press(TWO, &none, false);
        let two = set(&[TWO]);
        assert_eq!(
            block.frame(&two, false, true),
            Some(StellarPick::Slot(1)),
            "again"
        );
        assert_eq!(block.frame(&two, false, true), None, "held: latched");
        assert_eq!(
            block.frame(&none, false, true),
            None,
            "up: the latch clears"
        );
        assert_eq!(block.frame(&two, false, true), Some(StellarPick::Slot(1)));
    }

    #[test]
    fn a_press_while_another_block_key_is_held_does_nothing() {
        let mut block = SlotBlock::default();
        let none = set(&[]);
        let f1 = set(&[Key::Function(1)]);
        block.press(Key::Function(1), &none, false);
        assert_eq!(block.frame(&f1, false, true), Some(StellarPick::Slot(0)));
        block.press(TWO, &f1, false);
        assert_eq!(block.frame(&f1, false, true), None, "F1 holds the latch");
        // 5 held without Command holds nothing, so F3 is fresh.
        let five = set(&[FIVE]);
        block.frame(&none, false, true);
        block.press(Key::Function(3), &five, false);
        assert_eq!(block.frame(&five, false, true), Some(StellarPick::Slot(2)));
        // A key the menu hides holds nothing either.
        let mut block = SlotBlock::default();
        let two = set(&[TWO]);
        block.press(Key::Function(1), &none, false);
        block.frame(&set(&[Key::Function(1)]), false, true);
        block.press(Key::Function(1), &two, true);
        assert_eq!(block.frame(&two, true, true), Some(StellarPick::Slot(0)));
    }

    #[test]
    fn presses_in_one_frame_make_one_pick_by_the_blocks_order() {
        let mut block = SlotBlock::default();
        let none = set(&[]);
        block.press(NEAREST_STELLAR_KEY, &none, false);
        block.press(TWO, &none, false);
        assert_eq!(block.frame(&none, false, true), Some(StellarPick::Nearest));
        block.frame(&none, false, true);
        block.press(Key::Function(3), &none, false);
        block.press(ONE, &none, false);
        assert_eq!(block.frame(&none, false, true), Some(StellarPick::Slot(2)));
        assert_eq!(block.frame(&none, false, true), None, "taken");
    }

    #[test]
    fn command_5_tapped_counts_with_its_command_at_the_next_frame() {
        let mut block = SlotBlock::default();
        block.press(FIVE, &set(&[Key::Command]), false);
        assert_eq!(
            block.frame(&set(&[]), false, true),
            Some(StellarPick::Nearest)
        );
        block.frame(&set(&[]), false, true);
        block.press(FIVE, &set(&[]), false);
        assert_eq!(block.frame(&set(&[]), false, true), None, "5 alone");
    }

    #[test]
    fn an_unpolled_frame_leaves_the_latch_and_drops_the_presses() {
        // @0x69d2b-0x69d32: during a jump the block, its latch's clearing
        // included, is skipped.
        let mut block = SlotBlock::default();
        let none = set(&[]);
        let two = set(&[TWO]);
        block.press(TWO, &none, false);
        assert_eq!(block.frame(&none, false, false), None);
        assert_eq!(block.frame(&none, false, true), None, "the press dropped");
        assert_eq!(block.frame(&two, false, false), None, "pressed in the jump");
        assert_eq!(
            block.frame(&two, false, true),
            Some(StellarPick::Slot(1)),
            "acts after it"
        );
        assert_eq!(block.frame(&none, false, false), None, "let go in a jump");
        assert_eq!(block.frame(&two, false, true), None, "still latched");
    }

    #[test]
    fn a_press_is_queued_until_its_pick_then_while_the_latch_clears() {
        let mut block = SlotBlock::default();
        let none = set(&[]);
        assert!(!block.queued(&none, false, true), "nothing pressed");
        block.press(TWO, &none, false);
        assert!(block.queued(&none, false, true), "pressed");
        assert_eq!(block.frame(&none, false, true), Some(StellarPick::Slot(1)));
        assert!(block.queued(&none, false, true), "the latch to clear");
        assert_eq!(block.frame(&none, false, true), None);
        assert!(!block.queued(&none, false, true), "settled");
    }

    #[test]
    fn a_held_key_is_not_queued() {
        let mut block = SlotBlock::default();
        let two = set(&[TWO]);
        block.press(TWO, &set(&[]), false);
        assert_eq!(block.frame(&two, false, true), Some(StellarPick::Slot(1)));
        assert!(!block.queued(&two, false, true));
    }

    #[test]
    fn a_press_in_a_jump_is_queued_until_the_frame_that_drops_it() {
        let mut block = SlotBlock::default();
        let none = set(&[]);
        block.press(TWO, &none, false);
        assert!(block.queued(&none, false, false));
        assert_eq!(block.frame(&none, false, false), None);
        assert!(!block.queued(&none, false, false), "dropped");
    }

    #[test]
    fn the_block_polls_its_keys_as_the_menu_and_command_let_it() {
        let none = set(&[]);
        let down = |keys: HashSet<Key>| move |key: Key| keys.contains(&key);
        for n in 1..=5 {
            assert!(block_polls(Key::Function(n), down(none.clone()), false));
            assert!(block_polls(Key::Function(n), down(none.clone()), true));
        }
        for key in [ONE, TWO, THREE, Key::Char('4')] {
            assert!(block_polls(key, down(none.clone()), false), "{key:?}");
            assert!(
                !block_polls(key, down(none.clone()), true),
                "{key:?} masked"
            );
        }
        assert!(!block_polls(FIVE, down(none.clone()), false), "no Command");
        assert!(block_polls(FIVE, down(set(&[Key::Command])), false));
        assert!(
            !block_polls(FIVE, down(set(&[Key::Command])), true),
            "masked"
        );
        assert!(
            !block_polls(FIVE, down(set(&[Key::Command, NEAREST_STELLAR_KEY])), false),
            "F5 down skips Command-5 (@0x69dba)"
        );
        assert!(!block_polls(Key::Char('6'), down(none.clone()), false));
        assert!(
            !block_polls(Key::Command, down(none), false),
            "not the block's"
        );
    }

    #[test]
    fn the_blocks_keys_are_1_to_5_and_f1_to_f5() {
        for key in [ONE, TWO, THREE, Key::Char('4'), FIVE] {
            assert!(is_block_key(key), "{key:?}");
        }
        for n in 1..=5 {
            assert!(is_block_key(Key::Function(n)));
        }
        for key in [
            Key::Char('6'),
            Key::Function(6),
            Key::Command,
            Key::Char('a'),
        ] {
            assert!(!is_block_key(key), "{key:?}");
        }
    }

    #[test]
    fn the_escort_loop_polls_1_to_5_while_the_menu_shows() {
        for key in GROUP_KEYS {
            assert!(escort_loop_polls(key, true), "{key:?}");
            assert!(!escort_loop_polls(key, false), "{key:?}");
        }
        assert!(!escort_loop_polls(Key::Function(1), true));
        assert_eq!(NEAREST_STELLAR_COMMAND_KEY, FIVE);
    }
}
