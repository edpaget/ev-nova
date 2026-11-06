//! What flight takes back of the pad as the last dialog over it closes
//! ([`FlightView::take_back_pad`], part of
//! [`FlightView::take_back_live_input`]). The pad has no original: these
//! are its own rules, the keys' (the `dialog_latch` module) read through
//! the pad's layers.
//!
//! The router notes the pad as live input tells it (`nova`'s
//! `LiveInput`): each button down with the layer it went down under and
//! whether that layer is still held, the pad at flight's last poll, the
//! stick axes, and the press the closing dialog took. As the last dialog
//! closes:
//!
//! 1. **The axes** are taken as they stand. The pad reports an axis only
//!    when it moves (`nova`'s `PadInputs::route`), and flight's let-go
//!    centred them as the dialog opened, so a throttle or turn stick held
//!    still across it would otherwise read centred until it moved. An
//!    axis has no latch.
//! 2. **The taken button**, the pad press the closing dialog took (East
//!    closes the comm dialogs, the escort dialog, player info and the
//!    plain dialogs on its press), is let go: neither taken back nor
//!    latched nor replayed, so East's Land never follows the close it
//!    made. Its release reaches flight live and means nothing, as no
//!    press of it was noted. A South that pressed a dialog's button did
//!    so on its release, so it is up by the close. Keys keep the
//!    original's live read of the key that closed a dialog; only the
//!    pad's default table puts a flight command on a dialog's Back and
//!    Activate.
//! 3. **Every other button down** is adopted into flight's reader in the
//!    order pressed, with its recorded layer and liveness
//!    ([`CommandReader::adopt_pad`](crate::keymap::CommandReader::adopt_pad)),
//!    as flight holds every pad button while it has its input: a layer
//!    button must be down for a press after the close to take its layer,
//!    and a replayed press means its recorded layer. A plain West pressed
//!    before Select comes back as Closest Target, not Communicate.
//! 4. **What each one means** is read under its recorded layer only
//!    ([`CommandReader::pad_meanings`](crate::keymap::CommandReader::pad_meanings)),
//!    never through any layer's binding, which would class Select and
//!    South as read live for plain South's Fire Primary:
//!    - nothing (a layer alone, or a button whose layer went up first):
//!      held, meaning nothing;
//!    - read live ([`UNLATCHED`], [`OWN_LATCH`]): held, and an own-latch
//!      one joins the poll order for its place there, as an own-latch
//!      key does;
//!    - latched: with a site marked, it waits for its release, its latch
//!      set; with none, it waits if it was down at flight's last poll or
//!      flight read its press since, and otherwise, unless flight still
//!      held it, comes back as a fresh press at its first meaning's place
//!      in the poll order, among the keys';
//!    - anything else: held, meaning nothing to flight.
//! 5. Pause (Select and Start) is the router's: held, never replayed.
//!
//! A pad button flight still holds that is not latched is let go first,
//! as a key read live is: the pad says again what is down.
//!
//! A held d-pad or stick direction goes on repeating across the close
//! (`nova`'s repeater doesn't watch dialogs); adopted, its repeats are
//! followed as a held key's, and one a site's close latched drops them
//! until its release ([`FlightView::track_held`]). Touch controls are let
//! go of as a dialog opens and never handed back. Flight's own let-go
//! ([`FlightView::let_go_of_input`]) lets go of an adopted pad and its
//! axes, as of its keys.
//!
//! The taken press is a guess at which press closed the dialogs: the last
//! fresh press under them, if it was the pad's and is still down. A
//! dialog that closed by itself after a pad press under it would cost
//! that button a press; none over flight does.

use super::FlightView;
use super::dialog_latch::poll_index;
#[cfg(doc)]
use super::map_wait::UNLATCHED;
use super::map_wait::{OWN_LATCH, latches, reads_live};
use crate::Input;
use crate::flight::sim_port::FlightSim;
use crate::input::PadButton;
use crate::keymap::{Command, Gamepad, PadDown};

/// A fresh press of `button`.
const fn press_of(button: PadButton) -> Input {
    Input::PadButton {
        button,
        pressed: true,
        repeat: false,
    }
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// Takes back the pad as the last dialog over flight closes (see the
    /// module docs): `pad` what is down now, `pad_before` what was down at
    /// flight's last poll, `taken` the press the closing dialog took,
    /// `latch_all` whether the close latches every latched command, and
    /// `read` the presses flight read since that poll
    /// ([`FlightView::note_read_on_press`]). Gives the entries that join
    /// the keys' poll order, sorted already: the own-latch commands held,
    /// for their places, and the fresh presses to replay, each with its
    /// place and command.
    pub(super) fn take_back_pad(
        &mut self,
        pad: &Gamepad,
        pad_before: &Gamepad,
        taken: Option<PadButton>,
        latch_all: bool,
        read: &[Input],
    ) -> Vec<(usize, Command, Input)> {
        self.keys.adopt_axes(pad);
        let held: Vec<PadDown> = self.keys.gamepad().held_buttons().collect();
        for down in held {
            if !self.keys.pad_meanings(down).into_iter().any(latches) {
                self.keys.follow(&Input::PadButton {
                    button: down.button,
                    pressed: false,
                    repeat: false,
                });
            }
        }
        let mut entries = Vec::new();
        for down in pad.held_buttons() {
            if Some(down.button) == taken {
                continue;
            }
            let press = press_of(down.button);
            // One flight still holds is no fresh press.
            let still_held = self.keys.is_down(&press);
            let meanings = self.keys.pad_meanings(down);
            self.keys.adopt_pad(down);
            if meanings.iter().copied().any(reads_live) {
                let own = meanings
                    .iter()
                    .find_map(|&command| own_latch_entry(command, press));
                entries.extend(own);
                continue;
            }
            if !meanings.iter().copied().any(latches) {
                continue;
            }
            let waits = latch_all
                || pad_before.is_down(down.button)
                || read.iter().any(|taken| taken.same_key(&press));
            if waits {
                self.latch_pad(&meanings);
            } else if !still_held {
                let fresh = meanings
                    .iter()
                    .find_map(|&command| Some((poll_index(command)?, command, press)));
                entries.extend(fresh);
            }
        }
        entries
    }

    /// Notes an adopted pad button's `meanings` as latched: each latched
    /// one waits for its release ([`FlightView::latched_down`]), its latch
    /// set ([`FlightView::latch_command`]), as
    /// [`FlightView::latch_keys`] does for a key, but by the button's
    /// exact layer.
    fn latch_pad(&mut self, meanings: &[Command]) {
        for &command in meanings {
            if latches(command) {
                self.latch_command(command);
                self.latched_down.insert(command);
            }
        }
    }
}

/// `command`'s entry in the poll order if it has a latch of its own
/// ([`OWN_LATCH`]).
fn own_latch_entry(command: Command, press: Input) -> Option<(usize, Command, Input)> {
    if !OWN_LATCH.contains(&command) {
        return None;
    }
    Some((poll_index(command)?, command, press))
}
