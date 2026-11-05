//! Which latch rule a dialog over flight closes on, and what flight
//! takes back as the last one closes ([`FlightView::take_back_live_input`]).
//! All addresses are in `EV Nova.app`'s `EV Nova` (i386).
//!
//! `_HandlePlayer` reads every key live each frame (`_QuickKeyCheckHandler`
//! @0x43e01, `_QuickKeyCheck` @0xa0f5) and fires a latched command when its
//! key reads down with its latch clear. `_SetAllKeyCmdFlags` (@0x61e6d)
//! sets every latch, and touches nothing else. It has 16 callers, all in
//! flight's own routines, each after the dialog, map or movie it opened
//! (rdm:phase/play-feel-8/phase-3-keys-live-across-a-dialog). So a close
//! has one of two rules:
//!
//! - **A site followed it.** Every latched key down at the close waits
//!   for its release ([`FlightView::latched_down`]), every latch ours
//!   keeps set ([`FlightView::set_every_latch`]). The flight methods
//!   that port the 16 sites mark it ([`FlightView::latch_at_close`]).
//! - **None did.** The mission handler's texts (`_DoPlayGameWork`
//!   @0x44214), the AI plunder's failure (`_DoMissionFailure` @0x84a0f),
//!   the pod rescue's text (`_HandlePlayer` @0x68eac), an escort's
//!   defection (`_DoEscortPayment` @0x6c620), Preferences
//!   (`_AppEventHandler` @0xcccf1) and About (@0xcccb0) come from outside
//!   the sites' routines, which only `_HandlePlayer` calls, or leave
//!   `_HandlePlayer` with no site after them. No latch is touched, and
//!   `_HandlePlayer` doesn't run under the modal dialog, so each latched
//!   key's latch is what `_HandlePlayer`'s last poll before it left: one
//!   down then waits for its release, its latch set
//!   ([`FlightView::latch_command`]), even if it was let go and pressed
//!   again under the dialog, which no poll saw. One that went down
//!   since and is still down acts on the next pass, and one let go does
//!   nothing. These
//!   open through [`Screen::release_keys`](crate::Screen::release_keys),
//!   which never marks.
//!
//! The mark lasts until the hand-back takes it, so a site anywhere under
//! the dialogs that close together latches the close: Preferences opened
//! over an offer latches, as no `_HandlePlayer` pass runs between the
//! offer's `_SetAllKeyCmdFlags` and the close. A site that opened no
//! dialog (a pickup that fits, an offer accepted at once, a dialog not
//! loaded) still latched what was down, so the next pass's key poll
//! latches the keys held then and drops the mark
//! ([`FlightView::settle_dialog_latch`]), which also keeps it from
//! latching a later dialog no site follows. The original latches in the
//! same pass, so a latched key polled later in that pass doesn't act;
//! ours latches one pass later
//! (rdm:phase/play-feel-8-fidelity/phase-11-latched-keys-under-unlatching-dialogs-left-outs).
//!
//! Under either rule B and L ([`OWN_LATCH`]) are read as the
//! unlatched keys are: held whenever down, their own latches deciding at
//! the next pass, as no site sets them. A latched key waiting for its
//! release still reads down past its latch (the `latched_reads` module).

use super::map_wait::OWN_LATCH;
#[cfg(doc)]
use super::map_wait::UNLATCHED;
use super::{FlightView, slot_block};
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::escorts::EscortMenuLooks;
use crate::flight::sim_port::FlightSim;
use crate::galaxy::GalaxyCatalog;
use crate::keymap::{
    Command, EscortMenuCommand, FlightCommand as F, KeysDown, PlayCommand, is_modifier_input,
};
use crate::system::catalog::SystemCatalog;
use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};
use crate::{Input, Screen as _};

/// What is physically down as the last dialog over flight closes, as the
/// router saw it: the keys down now, those down before flight's input
/// went away, and whether the primary button is down.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LiveKeys {
    /// The keys down now.
    pub down: KeysDown,
    /// The keys down at flight's last poll (a tick it had its input) or
    /// as its input went away, whatever happened to them after: a latched
    /// one read its latch set, as the original's poll before the dialog
    /// would, and no poll saw its release since. A latched key polled
    /// held (J, the slot keys) pressed between flight's last poll
    /// and the dialog's opening, in the same frame, counts too, where the
    /// original's latch is clear: ours acts on the other latched keys'
    /// presses rather than at the poll, so the two can't be told apart
    /// (rdm:phase/play-feel-8-fidelity/phase-11-latched-keys-under-unlatching-dialogs-left-outs).
    pub down_before: KeysDown,
    /// Whether the primary button is down now.
    pub button: bool,
}

/// A flight command, for the const table.
const fn flight(command: F) -> Command {
    Command::Flight(command)
}

/// A group of the escort menu, for the const table.
const fn group(command: EscortMenuCommand) -> Command {
    Command::EscortMenu(command)
}

/// The latched commands and the press commands, in the order
/// `_HandlePlayer` polls their keys, each with its read and its latch
/// test (rdm:phase/play-feel-8/phase-19-latched-keys-under-unlatching-dialogs
/// records them), each Option or Shift variant beside its base: Nav Off
/// with Option and without (@0x69a23, @0x69aa1), Hyperspace Mode
/// (@0x69ac1), Hyper Select (@0x69bbe), the stellar slot block's slots
/// and nearest stellar (@0x69d3f-0x69dd3) with the escort menu's groups
/// on the same keys (@0x6e08c), Target Select (@0x69df6), the map key
/// (@0x6a778), Land (@0x6a8b8), Board (@0x6a98b), Closest Target
/// (@0x6a9b7), Select Secondary (@0x6ac1d), Hyper Jump (@0x6b0e6),
/// Acknowledge (@0x6dd00), the escort menu key (@0x6dda9), the escort
/// commands (@0x6e1b6), Show Framerate (@0x6e460), Player Info
/// (@0x702cb), Communicate (@0x703d8), Mission Info (@0x70411), Engage
/// Cloak (@0x705c3) and Zoom In (@0x71046-0x710f8, latch @0x70fdb).
/// Exactly the commands of flight's contexts less [`UNLATCHED`], Zoom Out
/// and Pause: Zoom Out's key is Self-Destruct's, held for its chord
/// (rdm:phase/play-feel-8-fidelity/phase-11-latched-keys-under-unlatching-dialogs-left-outs),
/// and Pause is the router's.
pub(super) const POLL_ORDER: [Command; 40] = [
    flight(F::ClearTarget),
    flight(F::NavOff),
    flight(F::HyperspaceMode),
    flight(F::HyperSelect),
    flight(F::StellarSlot1),
    flight(F::StellarSlot2),
    flight(F::StellarSlot3),
    flight(F::StellarSlot4),
    flight(F::NearestStellar),
    group(EscortMenuCommand::Group1),
    group(EscortMenuCommand::Group2),
    group(EscortMenuCommand::Group3),
    group(EscortMenuCommand::Group4),
    group(EscortMenuCommand::Group5),
    flight(F::TargetSelect),
    flight(F::TargetSelectBack),
    flight(F::TargetEscort),
    flight(F::TargetEscortBack),
    Command::Play(PlayCommand::GalaxyMap),
    flight(F::LandDock),
    flight(F::Board),
    flight(F::ClosestTarget),
    flight(F::ClosestShip),
    flight(F::SelectSecondary),
    flight(F::SelectSecondaryBack),
    flight(F::HyperJump),
    flight(F::Acknowledge),
    flight(F::EscortMenu),
    flight(F::AttackTarget),
    flight(F::DefendMe),
    flight(F::HoldPosition),
    flight(F::Recall),
    flight(F::RecallToHangar),
    flight(F::ShowFramerate),
    Command::Play(PlayCommand::PlayerInfo),
    flight(F::Communicate),
    flight(F::HailStellar),
    Command::Play(PlayCommand::MissionInfo),
    flight(F::EngageCloak),
    flight(F::ZoomIn),
];

impl<C, S: FlightSim> FlightView<C, S> {
    /// Marks that a `_SetAllKeyCmdFlags` site ran on this pass, so the
    /// dialogs it opened latch every key down at their close (see the
    /// module docs). The 16 sites (@0x61e6d's callers) and what ports
    /// each:
    ///
    /// - `_HandlePlayerCommunication`'s planet comm (@0x62062), offer
    ///   (@0x622ac) and comm dialogs (@0x6258e, @0x625ee): `hail`'s
    ///   answers and [`FlightView::hold_offer`], through
    ///   [`FlightView::let_go_into_dialog`];
    /// - `_PlayerLandOnStellar`'s take-off (@0x63583): `departed`;
    /// - `_PlayerEnterHypergate`'s map (@0x6387a) and both gates' movies
    ///   (@0x639f2, @0x6429c), and `_HandlePlayer`'s M map (@0x6a898):
    ///   flight's own, which keeps the input and latches at their close
    ///   (`FlightView::keep_unlatched_keys`);
    /// - `_HandlePlayerBoardAttempt`'s pickup (@0x6535a), its offer
    ///   (@0x65956) and its plunder (@0x659f1, @0x65f3e): `board` and
    ///   `boarded`;
    /// - the sabotage death's texts (@0x68802), unmarked: the dead player
    ///   is steered by nothing and play ends after the fade;
    /// - `_HandlePlayer`'s player info (@0x70393) and mission info
    ///   (@0x70524): `ask_player_info` and `ask_mission_info`.
    pub(super) fn latch_at_close(&mut self) {
        self.latch_at_close = true;
    }

    /// Lets go of flight's input for a dialog a `_SetAllKeyCmdFlags` site
    /// follows ([`FlightView::let_go_of_input`], then
    /// [`FlightView::latch_at_close`]).
    pub(super) fn let_go_into_dialog(&mut self) {
        self.let_go_of_input();
        self.latch_at_close();
    }

    /// Settles a site's mark at the start of a pass's key poll: still set,
    /// the site opened no dialog that took flight's input, and
    /// `_SetAllKeyCmdFlags` still latched what was down, so the keys held
    /// are latched ([`FlightView::keep_unlatched_keys`]) and the mark is
    /// dropped.
    pub(super) fn settle_dialog_latch(&mut self) {
        if std::mem::take(&mut self.latch_at_close) {
            self.keep_unlatched_keys();
        }
    }

    /// Takes back flight's input as the last dialog over it closes, `live`
    /// what is physically down: `_HandlePlayer` polls the keys
    /// (`_QuickKeyCheck` @0xa0f5) and `_HandleMouse` reads `_Button`
    /// (@0x3801c-0x3803b) each frame, so what is down reads as down
    /// whenever it went down. The unlatched keys held, and B and L, become
    /// exactly those of `live.down` ([`UNLATCHED`], [`OWN_LATCH`], and the
    /// modifier keys; [`FlightView::key_reads_live`]),
    /// none flight holds of the rest is let go, and a button still down
    /// keeps the cursor whole. B and L are never handed back as presses:
    /// held, the next pass reads them by their own latches. The latched
    /// keys follow the close's rule (see the module docs), the mark taken:
    ///
    /// - With a site marked, every latched key of `live.down` waits for
    ///   its release ([`FlightView::latched_down`]), every latch set
    ///   ([`FlightView::set_every_latch`]), and nothing comes back. So with flight's own map open (a dialog no site follows,
    ///   Preferences, closing over it): `_HandlePlayer` doesn't poll
    ///   under the map, whose close is a site (M's @0x6a898, the
    ///   hypergate's @0x6387a). No dialog opens over a flight movie, as
    ///   the movie takes every key.
    /// - With none, those also down before the input went away
    ///   (`live.down_before`) wait, and every other latched key of
    ///   `live.down` is read as a fresh press, in `_HandlePlayer`'s poll
    ///   order (the module's `POLL_ORDER`). One flight still holds is no
    ///   fresh press. Most act on their press in ours, so they come back
    ///   for the router to give flight as it gives any key. The slot
    ///   block's keys and the cloak act only at the next poll, as do B
    ///   and L with their latches clear, so a press key after the first
    ///   of them in the order (Tab, M, J, Y, P and the rest after the
    ///   block @0x69d3f; R, W, J and on after L @0x6a8b8) is kept for that
    ///   poll instead
    ///   (`FlightView::read_after_poll`): the block picks first, and a
    ///   dialog the later key opens latches the keys after it, as P's
    ///   `_SetAllKeyCmdFlags` (@0x70393) does.
    ///
    /// A key's place in that order is its command's, as a press of it
    /// would mean now, its key read through flight's reader; keys of one
    /// command (a slot's number and its F-key twin) go in key order.
    #[must_use]
    pub fn take_back_live_input(&mut self, live: LiveKeys) -> Vec<Input> {
        let LiveKeys {
            down,
            down_before,
            button,
        } = live;
        let latch_all = std::mem::take(&mut self.latch_at_close) || self.map_open;
        self.sync_escort_keys();
        for press in self.keys.down() {
            if self.key_reads_live(&press) {
                self.keys.follow(&press.released());
            }
        }
        self.latched_down.clear();
        // The modifiers first, so each other key reads as it would with
        // them down.
        let (modifiers, keys): (Vec<Input>, Vec<Input>) =
            down.presses().partition(is_modifier_input);
        for press in modifiers.iter().chain(&keys) {
            self.hold_live_key(press);
        }
        self.cursor.button(button);
        if latch_all {
            self.latch_keys(keys);
            self.set_every_latch();
            return Vec::new();
        }
        let waiting = keys
            .iter()
            .copied()
            .filter(|press| down_before.holds(press));
        self.latch_keys(waiting);
        let mut ordered: Vec<(usize, Command, Input)> = keys
            .iter()
            .filter_map(|&press| {
                let command = self
                    .keys
                    .peek(&press)
                    .or_else(|| self.keys.bound_to(&press).first().copied())?;
                let at = POLL_ORDER.iter().position(|&polled| polled == command)?;
                Some((at, command, press))
            })
            .collect();
        ordered.sort_by_key(|&(at, ..)| at);
        let mut presses = Vec::new();
        let mut polled_before = false;
        self.after_poll.clear();
        for (_, command, press) in ordered {
            if OWN_LATCH.contains(&command) {
                // Held, and read by its own latch at the next poll.
                polled_before |= !self.own_latch(command);
                continue;
            }
            if down_before.holds(&press) || self.keys.is_down(&press) {
                continue;
            }
            let at_poll = Self::acts_at_the_poll(command);
            if polled_before && !at_poll {
                self.after_poll.push(press);
            } else {
                presses.push(press);
            }
            polled_before |= at_poll;
        }
        presses
    }

    /// Whether `command`'s own latch is set, one of [`OWN_LATCH`]: L's
    /// (the `faded_keys` module's) or B's ([`FlightView::board_latch`]).
    fn own_latch(&self, command: Command) -> bool {
        if command == Command::Flight(F::LandDock) {
            self.faded_keys.land_latched()
        } else {
            self.board_latch
        }
    }

    /// Whether flight acts on `command` only at its next key poll or step,
    /// not on its press: the slot block's commands
    /// ([`FlightView::slot_keys_frame`]), but not the escort menu's groups
    /// on their keys, which act on the press, and Engage Cloak, whose edge
    /// the step reads. B and L, which also act at the poll while their
    /// latches are clear, are the hand-back's own test
    /// ([`FlightView::own_latch`]).
    fn acts_at_the_poll(command: Command) -> bool {
        slot_block::is_block_command(command) || command == Command::Flight(F::EngageCloak)
    }
}

impl<
    C: CombatLooks
        + SystemCatalog
        + ShipSprites
        + StatusBars
        + GalaxyCatalog
        + EscortMenuLooks
        + DescriptionSource
        + DescriptionGraphics,
    S: FlightSim,
> FlightView<C, S>
{
    /// Right after a tick's key poll ([`FlightView::held_keys_frame`]),
    /// reads the press keys a hand-back kept for it, in order, each still
    /// down ([`FlightView::take_back_live_input`]): whether one opened
    /// something, which ends the tick, as `_HandlePlayer`'s modal call
    /// stops the pass. The ones after it are dropped, as the site that
    /// follows its dialog latches them; so are all of them when the poll
    /// itself opened something, B's board read held.
    pub(super) fn read_after_poll(&mut self) -> bool {
        let kept = std::mem::take(&mut self.after_poll);
        if !kept.is_empty() && self.opened_something() {
            // The poll's own read opened it: B's board, its key held.
            return true;
        }
        for press in kept {
            let _ = self.input(&press);
            if self.opened_something() {
                return true;
            }
        }
        false
    }
}

#[cfg(any(test, feature = "fixture"))]
impl<C, S: FlightSim> FlightView<C, S> {
    /// Marks a `_SetAllKeyCmdFlags` site as run
    /// ([`FlightView::latch_at_close`]), for a test in a crate above that
    /// opens a site's dialog through the router itself, past the flight
    /// method that ports the site: `nova`'s fixtures open a person's offer
    /// so.
    pub fn mark_site(&mut self) {
        self.latch_at_close();
    }
}
