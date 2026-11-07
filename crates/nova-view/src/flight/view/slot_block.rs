//! The stellar slot keys' block of `_HandlePlayer` (@0x69d2b-0x69f17),
//! polled once a frame ([`SlotBlock::frame`]), whole:
//!
//! - While a jump is under way (ship +0x50 > 0) the block is skipped,
//!   its latch's clearing included (@0x69d2b-0x69d32).
//! - The four slots (keys 1-4, codes 0x12-0x15, @0x69d38-0x69d8d, then
//!   their twins F1-F4, key table +0x34..+0x3a, @0x69d90-0x69daa), each
//!   found down overriding the one before; then F5 (0x60,
//!   @0x69dac-0x69dba) or, with F5 up, Command and the top-row 5 (0x37,
//!   0x17, @0x69dbc-0x69dda) pick the nearest. So of keys down together
//!   the nearest stellar wins, then the twins, then the highest. No
//!   modifier is read for the slots: Command-1 is 1. A slot's key and its
//!   F-key twin are one command ([`FlightCommand::StellarSlot1`] and on),
//!   so the block reads each command by [`Column`]: down through its
//!   binding, or through its fixed twin.
//! - With none down the latch (0xdede0) is cleared (@0x69ddc-0x69de7);
//!   with one down, the latch set and, if it was clear, the pick made
//!   (@0x69e15-0x69e23), so one pick a frame, and none while any of the
//!   block's commands stays down.
//! - While the escort menu shows, `_QuickKeyCheck` reports 1-5 up
//!   (@0xa106-0xa124), so the block never finds 1-4 or the 5 down: flight's
//!   reader then reads those keys as the menu's groups, and only the
//!   F keys hold the block's commands.
//!
//! The pick itself, a slot of the nav defaults or the nearest stellar,
//! its owner check and its sound, is `nova_sim::Session::select_stellar`'s.
//!
//! A press is the OS's ([`SlotBlock::press`]): it follows a release, so
//! it clears the latch unless another of the block's commands is still
//! down, and it counts as down at the next frame even if let go before
//! it, as the frame's poll would see a tap, Command-5's with its Command.
//! The frame runs at the top of the next tick
//! ([`FlightView::held_keys_frame`](super::FlightView)), before any frame
//! is drawn, so all the presses of one frame make one pick.

use nova_sim::StellarPick;

use crate::keymap::{Command, FlightCommand};

/// The four slots' commands, in order.
const SLOTS: [FlightCommand; 4] = [
    FlightCommand::StellarSlot1,
    FlightCommand::StellarSlot2,
    FlightCommand::StellarSlot3,
    FlightCommand::StellarSlot4,
];

/// Which of a block command's keys holds it: its binding's (the number
/// row's 1-4, Command-5) or its fixed twin's (F1-F5), which the block
/// polls after them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Column {
    /// Its binding's key.
    Bound,
    /// Its fixed twin.
    Twin,
}

/// The block's latch (0xdede0) and the presses since its last frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct SlotBlock {
    latch: bool,
    presses: Vec<(Command, Column)>,
}

impl SlotBlock {
    /// An OS press of `command`, one of the block's, by its `column`'s
    /// key, with the commands read `down` before it, by column: the latch
    /// cleared unless another of the block's keys is down, and `command`
    /// down at the next frame.
    pub(super) fn press(
        &mut self,
        command: Command,
        column: Column,
        down: impl Fn(Command, Column) -> bool,
    ) {
        let other = |each: Command, at: Column| (each, at) != (command, column) && down(each, at);
        if pick(other).is_none() {
            self.latch = false;
        }
        self.presses.push((command, column));
    }

    /// Sets the latch, as `_SetAllKeyCmdFlags` (@0x61e6d) does: a block
    /// key down then picks nothing until a frame finds all of them up.
    pub(super) fn set_latch(&mut self) {
        self.latch = true;
    }

    /// One frame of the block, with the commands read `down` and the
    /// presses since the last, `polled` unless a jump is under way or the
    /// session refuses the nav keys: the stellar to pick, if any (see the
    /// module docs).
    pub(super) fn frame(
        &mut self,
        down: impl Fn(Command, Column) -> bool,
        polled: bool,
    ) -> Option<StellarPick> {
        let presses = std::mem::take(&mut self.presses);
        if !polled {
            return None;
        }
        let Some(picked) =
            pick(|command, column| down(command, column) || presses.contains(&(command, column)))
        else {
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
    /// commands read `down` and `polled` as [`SlotBlock::frame`] takes
    /// them: whether one more frame would change it. So a press is queued
    /// until the frame that picks or drops it, and a pick's latch until
    /// the frame that clears it; a key held down after its pick is not. A
    /// pick always sets the latch, so a frame that would pick changes the
    /// block too.
    ///
    /// It asks the block's fixed point, not which field holds the presses,
    /// so it stays true for as many frames as the block takes to act.
    #[cfg(any(test, feature = "fixture"))]
    pub(super) fn queued(&self, down: impl Fn(Command, Column) -> bool, polled: bool) -> bool {
        let mut next = self.clone();
        next.frame(down, polled);
        next != *self
    }
}

/// The stellar the block picks with the commands `down`, by column, by
/// its order (see the module docs); `None` when it finds none of its
/// commands down.
pub(super) fn pick(down: impl Fn(Command, Column) -> bool) -> Option<StellarPick> {
    let nearest = Command::Flight(FlightCommand::NearestStellar);
    if down(nearest, Column::Twin) || down(nearest, Column::Bound) {
        return Some(StellarPick::Nearest);
    }
    let found = |column: Column| {
        SLOTS
            .iter()
            .rposition(|&slot| down(Command::Flight(slot), column))
    };
    found(Column::Twin)
        .or_else(|| found(Column::Bound))
        .map(StellarPick::Slot)
}

/// Whether `command` is one of the block's: the four slots and the
/// nearest stellar.
pub(super) fn is_block_command(command: Command) -> bool {
    command == Command::Flight(FlightCommand::NearestStellar)
        || SLOTS.iter().any(|&slot| command == Command::Flight(slot))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: Command = Command::Flight(FlightCommand::StellarSlot1);
    const TWO: Command = Command::Flight(FlightCommand::StellarSlot2);
    const THREE: Command = Command::Flight(FlightCommand::StellarSlot3);
    const FOUR: Command = Command::Flight(FlightCommand::StellarSlot4);
    const NEAREST: Command = Command::Flight(FlightCommand::NearestStellar);
    const B: Column = Column::Bound;
    const T: Column = Column::Twin;

    /// `held` as the block's read of what is down.
    fn down(held: &[(Command, Column)]) -> impl Fn(Command, Column) -> bool + use<> {
        let held = held.to_vec();
        move |command, column| held.contains(&(command, column))
    }

    fn pick_of(held: &[(Command, Column)]) -> Option<StellarPick> {
        pick(down(held))
    }

    #[test]
    fn each_command_picks_its_slot_or_the_nearest_by_either_column() {
        for (slot, command) in [ONE, TWO, THREE, FOUR].into_iter().enumerate() {
            for column in [B, T] {
                assert_eq!(pick_of(&[(command, column)]), Some(StellarPick::Slot(slot)));
            }
        }
        assert_eq!(pick_of(&[(NEAREST, T)]), Some(StellarPick::Nearest));
        assert_eq!(pick_of(&[(NEAREST, B)]), Some(StellarPick::Nearest));
        assert_eq!(
            pick_of(&[(Command::Flight(FlightCommand::Accelerate), B)]),
            None
        );
        assert_eq!(pick_of(&[]), None);
    }

    #[test]
    fn of_keys_down_together_the_nearest_wins_then_the_twins_then_the_highest() {
        // @0x69d38-0x69dda: each key found down overrides the last.
        assert_eq!(pick_of(&[(ONE, B), (THREE, B)]), Some(StellarPick::Slot(2)));
        assert_eq!(pick_of(&[(THREE, B), (ONE, T)]), Some(StellarPick::Slot(0)));
        assert_eq!(pick_of(&[(ONE, T), (FOUR, T)]), Some(StellarPick::Slot(3)));
        assert_eq!(
            pick_of(&[(FOUR, T), (NEAREST, T)]),
            Some(StellarPick::Nearest)
        );
        assert_eq!(
            pick_of(&[(FOUR, T), (NEAREST, B)]),
            Some(StellarPick::Nearest)
        );
    }

    #[test]
    fn a_frame_picks_once_until_a_frame_with_no_key_down() {
        let mut block = SlotBlock::default();
        block.press(TWO, B, down(&[]));
        assert_eq!(
            block.frame(down(&[]), true),
            Some(StellarPick::Slot(1)),
            "a tap"
        );
        block.press(TWO, B, down(&[]));
        assert_eq!(
            block.frame(down(&[(TWO, B)]), true),
            Some(StellarPick::Slot(1)),
            "again"
        );
        assert_eq!(block.frame(down(&[(TWO, B)]), true), None, "held: latched");
        assert_eq!(block.frame(down(&[]), true), None, "up: the latch clears");
        assert_eq!(
            block.frame(down(&[(TWO, B)]), true),
            Some(StellarPick::Slot(1))
        );
    }

    #[test]
    fn a_press_while_another_block_key_is_held_does_nothing() {
        let mut block = SlotBlock::default();
        block.press(ONE, T, down(&[]));
        assert_eq!(
            block.frame(down(&[(ONE, T)]), true),
            Some(StellarPick::Slot(0))
        );
        block.press(TWO, B, down(&[(ONE, T)]));
        assert_eq!(
            block.frame(down(&[(ONE, T)]), true),
            None,
            "F1 holds the latch"
        );
        block.press(ONE, B, down(&[(ONE, T)]));
        assert_eq!(
            block.frame(down(&[(ONE, T)]), true),
            None,
            "F1 holds it for 1 too"
        );
        block.frame(down(&[]), true);
        block.press(THREE, T, down(&[]));
        assert_eq!(
            block.frame(down(&[]), true),
            Some(StellarPick::Slot(2)),
            "fresh once all are up"
        );
    }

    #[test]
    fn presses_in_one_frame_make_one_pick_by_the_blocks_order() {
        let mut block = SlotBlock::default();
        block.press(NEAREST, T, down(&[]));
        block.press(TWO, B, down(&[]));
        assert_eq!(block.frame(down(&[]), true), Some(StellarPick::Nearest));
        block.frame(down(&[]), true);
        block.press(THREE, T, down(&[]));
        block.press(ONE, B, down(&[]));
        assert_eq!(block.frame(down(&[]), true), Some(StellarPick::Slot(2)));
        assert_eq!(block.frame(down(&[]), true), None, "taken");
    }

    #[test]
    fn command_5_tapped_counts_at_the_next_frame() {
        let mut block = SlotBlock::default();
        block.press(NEAREST, B, down(&[]));
        assert_eq!(block.frame(down(&[]), true), Some(StellarPick::Nearest));
    }

    #[test]
    fn an_unpolled_frame_leaves_the_latch_and_drops_the_presses() {
        // @0x69d2b-0x69d32: during a jump the block, its latch's clearing
        // included, is skipped.
        let mut block = SlotBlock::default();
        let two = [(TWO, B)];
        block.press(TWO, B, down(&[]));
        assert_eq!(block.frame(down(&[]), false), None);
        assert_eq!(block.frame(down(&[]), true), None, "the press dropped");
        assert_eq!(block.frame(down(&two), false), None, "pressed in the jump");
        assert_eq!(
            block.frame(down(&two), true),
            Some(StellarPick::Slot(1)),
            "acts after it"
        );
        assert_eq!(block.frame(down(&[]), false), None, "let go in a jump");
        assert_eq!(block.frame(down(&two), true), None, "still latched");
    }

    #[test]
    fn a_press_is_queued_until_its_pick_then_while_the_latch_clears() {
        let mut block = SlotBlock::default();
        assert!(!block.queued(down(&[]), true), "nothing pressed");
        block.press(TWO, B, down(&[]));
        assert!(block.queued(down(&[]), true), "pressed");
        assert_eq!(block.frame(down(&[]), true), Some(StellarPick::Slot(1)));
        assert!(block.queued(down(&[]), true), "the latch to clear");
        assert_eq!(block.frame(down(&[]), true), None);
        assert!(!block.queued(down(&[]), true), "settled");
    }

    #[test]
    fn a_held_key_is_not_queued() {
        let mut block = SlotBlock::default();
        let two = [(TWO, B)];
        block.press(TWO, B, down(&[]));
        assert_eq!(block.frame(down(&two), true), Some(StellarPick::Slot(1)));
        assert!(!block.queued(down(&two), true));
    }

    #[test]
    fn a_press_in_a_jump_is_queued_until_the_frame_that_drops_it() {
        let mut block = SlotBlock::default();
        block.press(TWO, B, down(&[]));
        assert!(block.queued(down(&[]), false));
        assert_eq!(block.frame(down(&[]), false), None);
        assert!(!block.queued(down(&[]), false), "dropped");
    }

    #[test]
    fn the_blocks_commands_are_the_four_slots_and_the_nearest() {
        for command in [ONE, TWO, THREE, FOUR, NEAREST] {
            assert!(is_block_command(command), "{command:?}");
        }
        for command in [
            Command::Flight(FlightCommand::Accelerate),
            Command::EscortMenu(crate::keymap::EscortMenuCommand::Group1),
        ] {
            assert!(!is_block_command(command), "{command:?}");
        }
    }

    #[test]
    fn a_latched_block_key_keeps_a_fresh_one_from_picking() {
        // F1 latched across a close reads down with the latch set
        // (@0x69e15), so 2 pressed picks nothing until both are up.
        let mut block = SlotBlock::default();
        block.set_latch();
        let f1 = [(ONE, T)];
        block.press(TWO, B, down(&f1));
        assert_eq!(block.frame(down(&f1), true), None, "F1 holds it");
        assert_eq!(block.frame(down(&[]), true), None);
        block.press(TWO, B, down(&[]));
        assert_eq!(block.frame(down(&[]), true), Some(StellarPick::Slot(1)));
    }
}
