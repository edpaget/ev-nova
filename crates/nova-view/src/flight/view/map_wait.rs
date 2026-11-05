//! What [`FlightView`] keeps of the keys pressed while the map waits for
//! the map key's release ([`PlayCommand::GalaxyMap`](crate::keymap::PlayCommand::GalaxyMap)) to close (`_DoSystemMap`
//! @0x13a2e-0x13a46). All addresses are in `EV Nova.app`'s `EV Nova`
//! (i386).
//!
//! The wait reads no events (`_BusyIdle` @0x5981 is empty) and plays no
//! sound up to the return (@0x13a2e-0x13c62), and the closing
//! `FlushEvents(7, 0)` (@0x13c44-0x13c53) drops only null, mouse-down and
//! mouse-up events. So a click during the wait is gone, and a key-down,
//! an auto-key and a key-up stay queued for whoever reads events next.
//! What that does depends on who that is:
//!
//! - **Flight reads no queued key.** `_IterateInGameEventLoop` drops them
//!   (@0x56a6b, @0x56cec), and `_HandlePlayer` polls every key from the
//!   `GetKeys` snapshot of `_QuickKeyCheckHandler` (@0x5d5a, read by
//!   `_QuickKeyCheck` @0xa0f5). A key tapped during the wait does nothing
//!   after the close, and one still down reads as down, whenever it went
//!   down, before the wait and before the map opened too. So over flight
//!   the view keeps the keys of [`UNLATCHED`] and [`OWN_LATCH`] held across
//!   the map, presses and releases, and everything else is let go as the
//!   map opens. The latched keys wait for their release, their OS repeats
//!   included, which the poll never sees ([`FlightView::latched_down`]),
//!   reading down meanwhile for every read that skips their latch (the
//!   `latched_reads` module): `_HandlePlayer` sets its 22 latches and the
//!   escort menu's 5 after M's map (`_SetAllKeyCmdFlags` @0x61e6d, called
//!   @0x6a898), but not B's and L's. The same
//!   poll reads them after every other dialog over flight, which takes
//!   flight's input from the router instead: the router keeps what is
//!   physically down under it and hands it back as the last one closes
//!   ([`FlightView::take_back_live_input`]), which latches the rest only
//!   when a site followed the dialog (the `dialog_latch` module). A
//!   gate's movie, which flight plays, keeps them as the map does
//!   (`FlightView::came_through`), and leaves nothing queued to replay.
//! - **A screen reads the queue.** Every screen's loop takes it through
//!   `_MyModalDialog` (@0x5999a), and no caller flushes between
//!   `_DoSystemMap` and that loop. So over a screen the view keeps what the
//!   wait takes (the `ui::key_wait` module's `WaitKeys`, waiting for M)
//!   and hands it to the router at the close
//!   ([`FlightView::take_map_replay`]), which gives it to the screen.
//!
//! Left out:
//!
//! - `MenuKey` for a queued Command key in flight (@0x56a81), whose result
//!   is ignored, as ours' Command-Q acts on its press in the router.
//! - An auto-key's live read of M, P or I (the `what == 5` tests @0x4e0c6,
//!   @0x608cd, @0x9f7da and their kin): those keys stay press-only by the
//!   user's decision.
//! - Key-up events as such: no filter tests `what == 4`, and the view
//!   replays a queued key's release with its press only so a screen isn't
//!   left holding it.
//! - Update and activate events, which also survive the flush: ours draws
//!   every frame whole and has no such input.
//! - A wait ended by losing focus: ours can't read keys unfocused, as
//!   `_KeyCheck` reads them live (@0xa19b-0xa1dc), so
//!   [`Screen::release_keys`](crate::Screen::release_keys) closes the map
//!   and drops what was queued.
//! - [`Input::Text`] on its own: only the character a queued key typed
//!   is kept, behind that key, for the offer's filter, which reads the
//!   character (`_SingleMissionFilter` @0x98e4a); each text dialog opens
//!   through a `0xff7f` flush (`_EVTextInputDialog` @0x55429).

use super::FlightView;
use super::dialog_map::MapOpening;
use crate::flight::sim_port::FlightSim;
use crate::keymap::{Command, FlightCommand, Kind, PlayCommand, SharedBindings};
use crate::{Input, Key};

/// The key the map key's command, [`PlayCommand::GalaxyMap`], is bound to
/// in `bindings`, if any: what the map's close waits on. A key wait
/// compares the key itself (the phase's fixed keys).
pub(super) fn map_key(bindings: &SharedBindings) -> Option<Key> {
    bindings.with(|bindings| {
        bindings
            .keyboard
            .combo(PlayCommand::GalaxyMap.into())
            .map(|combo| combo.key)
    })
}

/// A flight command, for the const tables.
const fn flight(command: FlightCommand) -> Command {
    Command::Flight(command)
}

/// The commands `_HandlePlayer` acts on the moment they read down, with
/// no latch to wait on, so the map keeps their keys held: thrust +0x2a
/// (@0x6cb71), turn +0x26 and +0x28 (@0x6c9cc), reverse +0x2c (@0x6c72f),
/// face-target +0xe (@0x6aec6), Option's too, afterburner +0x30
/// (@0x6cadf), primary +0x4 (@0x6ab06), secondary +0x6 (@0x6ab87), weapon
/// safety +0x2 (@0x6adc2-0x6addf), the eject chord +0x22
/// (@0x6f29e-0x6f2c1), the self-destruct chord +0x24 (@0x6fdb4-0x6fdfc)
/// and jettison +0x1e (@0x6fd54-0x6fda1), Option's too. The raw Option,
/// Command and Shift (0x3a, 0x37, 0x38) are read live as well: flight's
/// reader keeps the modifier keys ([`FlightView::key_reads_live`]).
///
/// Every other held flight command has a latch `_SetAllKeyCmdFlags`
/// (@0x61e6d) sets: `_navModeSelectFlag` Nav Off and Hyperspace Mode
/// (0xdee04), `_navTargetSelectFlag` Hyper Select (0xdee48),
/// `_navDefaultSelectFlag` the stellar slots (0xdede0),
/// `_weapTargetSelectFlag` Target Select (0xdede8), `_mapTryFlag` the
/// map key (0xdedf8), `_weapCloseTargFlag` Closest Target (0xdee50),
/// `_weapSelectFlag` Select Secondary (0xdee54), `_jumpTryFlag` Hyper
/// Jump (0xdee44), `_ackCommFlag` Acknowledge (0xdedf4), `_playerInfoFlag`
/// Player Info (0xdedd4), `_commTryFlag` Communicate (0xdee0c),
/// `_missionInfoFlag` Mission Info (0xdee00) and `_cloakFlag` Engage
/// Cloak (0xdedfc), each Option or Shift variant with its base. Board and
/// Land have latches of their own that it doesn't set ([`OWN_LATCH`]).
/// A latched command held past the close still reads down for every read
/// that skips its latch (the `latched_reads` module).
pub(super) const UNLATCHED: [Command; 14] = [
    flight(FlightCommand::Accelerate),
    flight(FlightCommand::RotateLeft),
    flight(FlightCommand::RotateRight),
    flight(FlightCommand::ReverseCourse),
    flight(FlightCommand::Autopilot),
    flight(FlightCommand::FaceStellar),
    flight(FlightCommand::Afterburner),
    flight(FlightCommand::FirePrimary),
    flight(FlightCommand::FireSecondary),
    flight(FlightCommand::WeaponSafety),
    flight(FlightCommand::Eject),
    flight(FlightCommand::SelfDestruct),
    flight(FlightCommand::JettisonCargo),
    flight(FlightCommand::JettisonNonMission),
];

/// The commands whose latches `_SetAllKeyCmdFlags` (@0x61e6d) never sets:
/// Land's `_dockTryFlag` (0xdee34), set only as L makes a dock request
/// (@0x6a953) and cleared on a pass L is up or its gate closed
/// (@0x6a95d), and Board's `_boardTryFlag` (0xdee18), set by a board
/// attempt past its no-target and cloak returns (@0x6504c) and cleared on
/// a pass B is up (@0x6a9a3). So across a close, a map or a gate's movie
/// they are read as the unlatched commands are: held whenever their keys
/// are physically down and read on every pass, their own latches
/// deciding ([`FlightView::board_latch`], the `faded_keys` module's L
/// latch). One first pressed under the dialog acts at the next pass, its
/// latch clear; one that acted before it waits for its release, even let
/// go and pressed again under it, which no pass saw.
pub(super) const OWN_LATCH: [Command; 2] = [
    flight(FlightCommand::LandDock),
    flight(FlightCommand::Board),
];

/// Whether flight reads `command` across a close with no latch of
/// `_SetAllKeyCmdFlags`' in its way: [`UNLATCHED`] and [`OWN_LATCH`].
pub(super) fn reads_live(command: Command) -> bool {
    UNLATCHED.contains(&command) || OWN_LATCH.contains(&command)
}

/// Whether `command` is one a `_SetAllKeyCmdFlags` site latches: a held
/// flight command not read live.
pub(super) fn latches(command: Command) -> bool {
    command.kind() == Kind::Held && !reads_live(command)
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// Whether the map open is flight's own: M's or the hypergate's, which
    /// takes its input from flight ([`MapOpening::Plain`]).
    pub(super) fn plain_map_open(&self) -> bool {
        self.map_opened
            .is_some_and(|opened| opened.by == MapOpening::Plain)
    }

    /// Whether the key of `input` is one flight holds: a modifier key
    /// flight reads (the left ones), or a key bound to a held command of flight's contexts, whatever
    /// modifiers it needs ([`Kind::Held`]; X is the eject key alone too).
    pub(super) fn is_flight_key(&self, input: &Input) -> bool {
        self.keys.reads_modifier(input)
            || self
                .keys
                .bound_to(input)
                .iter()
                .any(|command| command.kind() == Kind::Held)
    }

    /// Whether flight reads the key of `input` down across a close with no
    /// latch in its way: a modifier, or a key bound to one of
    /// [`UNLATCHED`] or [`OWN_LATCH`].
    pub(super) fn key_reads_live(&self, input: &Input) -> bool {
        self.keys.reads_modifier(input) || self.keys.bound_to(input).into_iter().any(reads_live)
    }

    /// Whether the key of `input` is a latched one: a flight key not read
    /// live ([`FlightView::key_reads_live`]).
    pub(super) fn key_latches(&self, input: &Input) -> bool {
        !self.key_reads_live(input) && self.keys.bound_to(input).into_iter().any(latches)
    }

    /// Keeps held of the commands held only those flight goes on reading
    /// live across the map ([`UNLATCHED`], [`OWN_LATCH`]), as every path
    /// that calls it runs `_SetAllKeyCmdFlags`, and sets every latch it
    /// sets ([`FlightView::set_every_latch`]): the latched commands it
    /// lets go of wait for their keys' release
    /// ([`FlightView::latched_down`]), repeats and all, still reading down
    /// past their latches.
    pub(super) fn keep_unlatched_keys(&mut self) {
        let latched: Vec<Command> = self.commands_down().filter(|&c| latches(c)).collect();
        for command in latched {
            self.latched_down.insert(command);
            self.latch_command(command);
        }
        self.set_every_latch();
    }

    /// Notes the keys `presses` press as down and latched, those flight
    /// reads with a latch `_SetAllKeyCmdFlags` sets
    /// ([`FlightView::key_latches`]): each followed, its latched commands
    /// waiting for its release, and each latch it could meet set
    /// ([`FlightView::latch_command`]), as the poll that saw it down or
    /// the site set it.
    pub(super) fn latch_keys(&mut self, presses: impl IntoIterator<Item = Input>) {
        for press in presses {
            if !self.key_latches(&press) {
                continue;
            }
            for command in self.keys.bound_to(&press) {
                if latches(command) {
                    self.latch_command(command);
                }
            }
            self.keys.follow(&press);
            let latched: Vec<Command> = self
                .keys
                .meanings(&press)
                .into_iter()
                .filter(|&command| latches(command))
                .collect();
            self.latched_down.extend(latched);
        }
    }

    /// Tracks `input`'s key as held if it is one flight goes on reading
    /// live across the map ([`UNLATCHED`], [`OWN_LATCH`]). A latched
    /// flight key isn't held, but its press or repeat is noted as latched
    /// and its release lets go of it, so the close leaves latched exactly
    /// those still down ([`FlightView::latched_down`]), however they went
    /// down: `_SetAllKeyCmdFlags` (@0x6a898) latches every key down at the
    /// close, so one first seen here by a repeat, its press gone
    /// elsewhere, is latched as a pressed one is. A gate's movie starts
    /// with the keys flight held, which its come-out sorts
    /// (`FlightView::came_through`), and its queue is no replay to flight
    /// ([`FlightView::take_movie_replay`]), so a latched key pressed under
    /// it acts only once let go and pressed again.
    pub(super) fn track_unlatched(&mut self, input: &Input) {
        let Input::Key { pressed, .. } = *input else {
            return;
        };
        if pressed && self.key_latches(input) {
            self.latch_keys([*input]);
        } else {
            self.track_held(input);
        }
    }

    /// Holds the key `press` presses if flight would read it down at once
    /// ([`FlightView::key_reads_live`]), for one still down as the last
    /// dialog over flight closes. Its release comes to flight live.
    pub fn hold_live_key(&mut self, press: &Input) {
        if self.key_reads_live(press) {
            self.keys.follow(press);
        }
    }

    /// What the last map over a screen left for the screen under it, in
    /// order, once: the keys typed during its wait (see the module docs),
    /// for the router to replay. Nothing from a map over flight, and
    /// nothing from a wait that focus ended.
    pub fn take_map_replay(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.map_replay)
    }
}
