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
//!   ([`FlightView::latch_key`]), even if it was let go and pressed
//!   again under the dialog, which no poll saw. One that went down
//!   since and is still down acts on the next pass, and one let go does
//!   nothing. These
//!   open through [`Screen::release_keys`](crate::Screen::release_keys),
//!   which never marks.
//!
//! The mark lasts until the hand-back takes it, so a site anywhere under
//! the dialogs that close together latches the close: Preferences opened
//! over an offer latches, as no `_HandlePlayer` pass runs between the
//! offer's `_SetAllKeyCmdFlags` and the close. A site that can open no
//! dialog (a pickup that fits, an offer accepted at once, a refusal whose
//! text the flight queues, a key read on a faded pass) latches the keys
//! held as it runs ([`FlightView::latch_at_site`]), so a latched key
//! polled later in that pass doesn't act, and a latched key pressed
//! before the next pass's poll waits for its release
//! ([`FlightView::wait_after_site`]), as no poll has cleared its latch.
//! That poll drops the mark ([`FlightView::settle_dialog_latch`]), which
//! keeps it from latching a later dialog no site follows. A latched key
//! pressed between a site's dialog's close and the next poll acts on its
//! press, where the original's latch is still set; so do two latched
//! presses of one frame, which ours acts on in their arrival order
//! rather than the poll's
//! (rdm:phase/flight-controls-2/phase-6-what-flight-controls-phase-16-left-out).
//!
//! Under either rule B, L and the escort commands F, D, V and C
//! ([`OWN_LATCH_KEYS`]) are read as the unlatched keys are: held whenever
//! down, their own latches deciding at the next pass, as no site sets
//! them. A latched key waiting for its
//! release still reads down past its latch (the `latched_reads` module).

use super::map_wait::{OWN_LATCH_KEYS, UNLATCHED_KEYS, reads_live};
use super::{
    BOARD_KEY, ESCORT_ATTACK_KEY, ESCORT_DEFEND_KEY, ESCORT_HOLD_KEY, ESCORT_RECALL_KEY,
    FRAME_RATE_KEY, FlightView, HAIL_KEY, HYPER_SELECT_KEY, HYPERSPACE_MODE_KEY, JUMP_KEY,
    LAND_KEY, MAP_KEY, MENU_CLOSE_KEY, NAV_OFF_KEY, NEAREST_KEY, NEAREST_STELLAR_COMMAND_KEY,
    NEAREST_STELLAR_KEY, SELECT_KEY, STELLAR_SLOT_KEYS, TARGET_KEY, ZOOM_IN_KEY,
    ZOOM_IN_KEYPAD_KEY, ZOOM_OUT_KEY,
};
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::escorts::{EscortMenuLooks, GROUP_KEYS, MENU_KEY};
use crate::flight::sim_port::FlightSim;
use crate::galaxy::GalaxyCatalog;
use crate::system::catalog::SystemCatalog;
use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::{Input, Key, Screen as _};

/// What is physically down as the last dialog over flight closes, as the
/// router saw it: the keys down now, those down at flight's last poll,
/// and whether the primary button is down. The router passes the physical
/// state; flight knows which latched keys it read on their press since
/// that poll ([`FlightView::note_read_on_press`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LiveKeys {
    /// The keys down now.
    pub down: Vec<Key>,
    /// The keys down at flight's last poll (a tick it had its input, or
    /// the last hand-back), whatever happened to them after: a latched
    /// one read its latch set, as the original's poll before the dialog
    /// would, and no poll saw its release since. A key pressed after that
    /// poll and before the dialog opened, in the same frame, isn't here:
    /// flight says itself whether it read it on its press.
    pub down_before: Vec<Key>,
    /// Whether the primary button is down now.
    pub button: bool,
}

/// The latched keys in the order `_HandlePlayer` polls them, each with its
/// read and its latch test (rdm:phase/play-feel-8/phase-19-latched-keys-under-unlatching-dialogs
/// records them): Option-\` and \` (@0x69a23, @0x69aa1), H (@0x69ac1),
/// `\` (@0x69bbe), the stellar slot block's 1-4, F1-F4, F5 and Command-5
/// (@0x69d3f-0x69dd3, the escort menu's 1-5 too, @0x6e08c), Tab (@0x69df6),
/// M (@0x6a778), L (@0x6a8b8), B (@0x6a98b), R (@0x6a9b7), W (@0x6ac1d),
/// J (@0x6b0e6), Return (@0x6dd00), E (@0x6dda9), the escort commands F,
/// D, V and C (@0x6e1b6), F12 (@0x6e460), P (@0x702cb), Y (@0x703d8),
/// I (@0x70411), U (@0x705c3), then the zoom keys against their shared
/// latch `_mapZoomFlag` (@0x70fdb): `-` (@0x70fc2) before `=` and keypad
/// `+` (@0x71046-0x710f8). Exactly [`every_key`](super::every_key) less
/// [`UNLATCHED_KEYS`], plus `-`, which is both: the self-destruct key,
/// held for its chord with no latch, and the zoom out.
pub(super) const POLL_ORDER: [Key; 34] = [
    NAV_OFF_KEY,
    HYPERSPACE_MODE_KEY,
    HYPER_SELECT_KEY,
    STELLAR_SLOT_KEYS[0][0],
    STELLAR_SLOT_KEYS[1][0],
    STELLAR_SLOT_KEYS[2][0],
    STELLAR_SLOT_KEYS[3][0],
    STELLAR_SLOT_KEYS[0][1],
    STELLAR_SLOT_KEYS[1][1],
    STELLAR_SLOT_KEYS[2][1],
    STELLAR_SLOT_KEYS[3][1],
    NEAREST_STELLAR_KEY,
    NEAREST_STELLAR_COMMAND_KEY,
    TARGET_KEY,
    MAP_KEY,
    LAND_KEY,
    BOARD_KEY,
    NEAREST_KEY,
    SELECT_KEY,
    JUMP_KEY,
    MENU_CLOSE_KEY,
    MENU_KEY,
    ESCORT_ATTACK_KEY,
    ESCORT_DEFEND_KEY,
    ESCORT_HOLD_KEY,
    ESCORT_RECALL_KEY,
    FRAME_RATE_KEY,
    PLAYER_INFO_KEY,
    HAIL_KEY,
    MISSION_INFO_KEY,
    super::CLOAK_KEY,
    ZOOM_OUT_KEY,
    ZOOM_IN_KEY,
    ZOOM_IN_KEYPAD_KEY,
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

    /// Runs a `_SetAllKeyCmdFlags` site that may open no dialog (a pickup
    /// that fits, an offer accepted at once, a refusal whose text the
    /// flight queues, a key read on the pass a jump collapsed under the
    /// white): the keys held are latched now, in the pass, as the site
    /// latches them before any later read of it
    /// ([`FlightView::keep_unlatched_keys`]), and the site is marked
    /// ([`FlightView::latch_at_close`]), so a dialog it queued, or one
    /// opened before the next poll, latches at its close, and a latched
    /// key pressed before that poll waits ([`FlightView::wait_after_site`]).
    pub(super) fn latch_at_site(&mut self) {
        self.keep_unlatched_keys();
        self.latch_at_close();
        self.latched_at_site = true;
    }

    /// A key `input` presses or repeats after a site that opened no dialog
    /// ([`FlightView::latch_at_site`]), up to the next poll, flight
    /// keeping its input meanwhile: whether it was taken here. A latched
    /// key's latch is set (`_SetAllKeyCmdFlags` @0x61e6d), and only a poll
    /// that reads it up
    /// clears it, so one pressed now reads down with its latch set at that
    /// poll and waits for its release: it is noted latched
    /// ([`FlightView::latch_held_keys`]) and not acted on. `-` is held
    /// for the self-destruct chord, which reads it with no latch
    /// (@0x6fdb4-0x6fdfc), but doesn't zoom, as `_mapZoomFlag` is set
    /// (@0x61ef8); a zoom in key is dropped. The keys flight reads live
    /// ([`UNLATCHED_KEYS`], [`OWN_LATCH_KEYS`]) pass. A dialog over
    /// flight ends it ([`FlightView::let_go_of_input`]): what is pressed
    /// between its close and the next poll, the router's replays of what
    /// was queued under it among them, acts on its press
    /// (rdm:phase/flight-controls-2/phase-6-what-flight-controls-phase-16-left-out).
    pub(super) fn wait_after_site(&mut self, input: &Input) -> bool {
        let Input::Key {
            key, pressed: true, ..
        } = *input
        else {
            return false;
        };
        if !self.latched_at_site {
            return false;
        }
        if key == ZOOM_OUT_KEY {
            self.track_held(input);
            return true;
        }
        if !POLL_ORDER.contains(&key) || OWN_LATCH_KEYS.contains(&key) {
            return false;
        }
        // The zoom in keys are no flight keys: dropped.
        self.latch_held_keys([key]);
        true
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
    /// are latched ([`FlightView::keep_unlatched_keys`]), for a site whose
    /// dialog never opened, and the mark is dropped: from this poll on a
    /// latched key's press acts again. A site run on a press stands for
    /// the pass whose poll that press is, as ours' press keys do
    /// throughout, so the keys pressed after it and before this poll were
    /// latched at their press ([`FlightView::wait_after_site`]).
    pub(super) fn settle_dialog_latch(&mut self) {
        self.latched_at_site = false;
        self.read_on_press.clear();
        if std::mem::take(&mut self.latch_at_close) {
            self.keep_unlatched_keys();
        }
    }

    /// Takes back flight's input as the last dialog over it closes, `live`
    /// what is physically down: `_HandlePlayer` polls the keys
    /// (`_QuickKeyCheck` @0xa0f5) and `_HandleMouse` reads `_Button`
    /// (@0x3801c-0x3803b) each frame, so what is down reads as down
    /// whenever it went down. The unlatched keys held, and those with
    /// latches of their own (B, L and the escort commands), become exactly
    /// those of `live.down` ([`UNLATCHED_KEYS`], [`OWN_LATCH_KEYS`]), none
    /// flight holds of the rest is let go, and a button still down keeps
    /// the cursor whole. The own-latch keys are never handed back as
    /// presses: held, the next pass reads them by their own latches. The
    /// latched keys follow the close's rule (see the module docs), the mark taken:
    ///
    /// - With a site marked, every latched key of `live.down` waits for
    ///   its release ([`FlightView::latched_down`]), every latch set
    ///   ([`FlightView::set_every_latch`]), and nothing comes back. So
    ///   with flight's own map open (a dialog no site follows,
    ///   Preferences, closing over it): `_HandlePlayer` doesn't poll
    ///   under the map, whose close is a site (M's @0x6a898, the
    ///   hypergate's @0x6387a). No dialog opens over a flight movie, as
    ///   the movie takes every key.
    /// - With none, those also down at flight's last poll
    ///   (`live.down_before`), or whose press flight read since
    ///   ([`FlightView::note_read_on_press`]), wait, and every other
    ///   latched key of
    ///   `live.down` is read as a fresh press, in `_HandlePlayer`'s poll
    ///   order (the module's `POLL_ORDER`). One flight still holds is no
    ///   fresh press, but `-`, which it holds for the self-destruct chord
    ///   and reads as the zoom out by `_mapZoomFlag` (@0x70fdb): it comes
    ///   back as a press, its zoom held back by Command or any other key
    ///   down ([`FlightView::zoom_floating_map`]). Most act on their
    ///   press in ours, so they come back for the router to give flight
    ///   as it gives any key. The slot
    ///   block's keys and the cloak act only at the next poll, as do B, L
    ///   and the escort commands (@0x6e1b6, before F12, P, Y, I and the
    ///   zooms) with their latches clear, so a press key after the first
    ///   of them in the order (Tab, M, J, Y, P and the rest after the
    ///   block @0x69d3f; R, W, J and on after L @0x6a8b8) is kept for that
    ///   poll instead
    ///   (`FlightView::read_after_poll`): the block picks first, and a
    ///   dialog the later key opens latches the keys after it, as P's
    ///   `_SetAllKeyCmdFlags` (@0x70393) does.
    #[must_use]
    pub fn take_back_live_input(&mut self, live: LiveKeys) -> Vec<Input> {
        let latch_all = std::mem::take(&mut self.latch_at_close) || self.map_open;
        self.held.retain(|&key| !reads_live(key));
        self.latched_down.clear();
        for &key in &live.down {
            self.hold_live_key(key);
        }
        self.cursor.button(live.button);
        if latch_all {
            self.latch_held_keys(live.down);
            self.set_every_latch();
            return Vec::new();
        }
        let read = std::mem::take(&mut self.read_on_press);
        let waits = |key: &Key| live.down_before.contains(key) || read.contains(key);
        let waiting: Vec<Key> = live.down.iter().copied().filter(waits).collect();
        self.latch_held_keys(waiting);
        let mut presses = Vec::new();
        let mut polled_before = false;
        self.after_poll.clear();
        for key in POLL_ORDER.into_iter().filter(|key| live.down.contains(key)) {
            if OWN_LATCH_KEYS.contains(&key) {
                // Held, and read by its own latch at the next poll.
                polled_before |= !self.own_latch(key);
                continue;
            }
            // `-` is held for its chord whatever its zoom's latch says.
            let still_held = self.held.contains(&key) && !UNLATCHED_KEYS.contains(&key);
            if waits(&key) || still_held {
                continue;
            }
            let at_poll = self.acts_at_the_poll(key);
            if polled_before && !at_poll {
                self.after_poll.push(key);
            } else {
                presses.push(press(key));
            }
            polled_before |= at_poll;
        }
        presses
    }

    /// Notes that flight took a press of `key` since its last key poll: a
    /// latched key ours acts on at its press, as the original's poll reads
    /// it, its latch set, so one still down at the close of a dialog no
    /// site follows waits for its release (`live.down_before`'s keys
    /// aside, [`FlightView::take_back_live_input`]). Not the keys read only
    /// at the poll ([`FlightView::acts_at_the_poll`]) nor those with
    /// latches of their own ([`OWN_LATCH_KEYS`]): a press of one after the
    /// poll and before an input's dialog in the same frame (Preferences
    /// by Command-`,`, About) leaves its latch clear. The next key poll
    /// forgets them ([`FlightView::settle_dialog_latch`]), as does the
    /// hand-back that reads them.
    pub(super) fn note_read_on_press(&mut self, key: Key) {
        if POLL_ORDER.contains(&key)
            && !OWN_LATCH_KEYS.contains(&key)
            && !self.acts_at_the_poll(key)
        {
            self.read_on_press.insert(key);
        }
    }

    /// Whether `key`'s own latch is set, one of [`OWN_LATCH_KEYS`]: L's
    /// (the `faded_keys` module's), B's ([`FlightView::board_latch`]) or an
    /// escort command's (`FlightView::escort_commands`).
    fn own_latch(&self, key: Key) -> bool {
        match key {
            LAND_KEY => self.faded_keys.land_latched(),
            BOARD_KEY => self.board_latch,
            _ => super::cockpit::ESCORT_COMMAND_KEYS
                .iter()
                .position(|&command| command == key)
                .is_some_and(|index| self.escort_commands.is_set(index)),
        }
    }

    /// Whether flight acts on `key` only at its next key poll or step,
    /// not on its press: the slot block's keys
    /// ([`FlightView::slot_keys_frame`]), but 1-5 while the escort menu
    /// takes them as its groups, which act on the press, and the cloak,
    /// whose edge the step reads. B, L and the escort commands, which also
    /// act at the poll while their latches are clear, are the hand-back's
    /// own test
    /// ([`FlightView::own_latch`]).
    fn acts_at_the_poll(&self, key: Key) -> bool {
        let group = self.escort_menu.is_open() && GROUP_KEYS.contains(&key);
        (super::slot_block::is_block_key(key) && !group) || key == super::CLOAK_KEY
    }
}

/// A fresh press of `key`.
fn press(key: Key) -> Input {
    Input::Key {
        key,
        pressed: true,
        repeat: false,
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
        for key in kept {
            let _ = self.input(&press(key));
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
