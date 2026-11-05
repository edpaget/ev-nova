//! What [`FlightView`] does with M, L, P, Y and I around a jump's end:
//! the presses kept through a jump for the pass that reads them, the pass
//! a jump collapses, with its `_screenFaded` guards, and L and I held past
//! a jump. All addresses are in `EV Nova.app`'s `EV Nova` (i386).
//!
//! **Why the presses wait.** `_HandlePlayer` polls each key on its pass,
//! but ours reads a press before the tick's steps, so a press on the pass
//! a jump ends would see the jump still under way. So a press of one of
//! these keys during a jump, the map shut, is kept
//! ([`FlightView::take_input`]'s only stop for them) and settled right
//! after the steps that follow ([`FlightView::settle_faded_keys`]), in the
//! pass's order: M @0x6a76c, L @0x6a8ac, P @0x702cb, Y @0x703d8, I
//! @0x70411. A press kept for a tick counts as read on that tick's end.
//!
//! **Which keys a pass reads.** M, P and Y are read only if kept: one
//! down earlier latched on its first pass. L and I also read if down with
//! their latch clear, as their failed gate clears it rather than setting
//! it (@0x6a95d, @0x705af), so one held through a jump waits.
//!
//! **The settle's four branches**, by what the steps left:
//!
//! - The jump still under way, its warp not ended: every gate read +0x50
//!   above 0. P and Y refuse with their beep (@0x7039a-0x703bd, @0x61f87),
//!   and M, L and I say nothing. All are dropped.
//! - The warp's end due ([`FlightFrame::warp_end_due`], the arrival about
//!   to run): M's and L's blocks ran with +0x50 above 0, so M is latched
//!   and lost (@0x6a89f) and L waits (@0x6a95d). The pass ends @0x6c625
//!   before P's, Y's and I's blocks. So P and Y stay kept for the first
//!   read after the arrival, and I, like L, waits on its latch.
//! - A jump collapsed under the white ([`StepRun::lit_collapse`]): the
//!   one pass the dialogs' guards decide anything, as +0x50 fell ahead of
//!   every key block (@0x689ea) and `_screenFaded` falls on the next pass
//!   (@0x6841f). M beeps both its beeps around `_DoSystemMap`'s bare
//!   return (@0x6a809, @0x121b7 to @0x13c58, @0x6a85a), so no map, no wait
//!   for its release and nothing queued, and `_SetAllKeyCmdFlags` runs
//!   (@0x6a898). L presses the land key. P, then Y, then I go to the
//!   flight as [`FlightCommand::PressFaded`], and a refusal by the guard
//!   does the same `_SetAllKeyCmdFlags` (@0x70393, @0x70524). That covers
//!   the latches of M, P, I and Y but not L's (@0x61e6d), so the later of
//!   P, Y and I do nothing that pass. A press the guard doesn't refuse
//!   acts as ever: a person's offer, #354.
//! - The jump gone any other way (an unlit collapse, a jump given up):
//!   +0x50 fell before the key blocks and the flag is down (@0x689ea,
//!   @0x683f1), so each key read is a plain press, one that opens something
//!   ending the pass as the same latches do. L's block still runs after
//!   M's map.
//!
//! **After the settle.** Once no jump is under way, a key still kept acts
//! as a plain press if it is down and is dropped if not; L and I act on
//! their latches ([`FlightView::faded_keys_frame`]). Double time's held
//! arrival leaves the jump under way for a tick, and the keys wait through
//! it. A key that opens something from there ends the tick: flight stops
//! under what it opened, as the original's modal call stops the pass.
//!
//! **The latches.** With its key down, a failed gate (a jump, or the
//! flight not taking the command) clears the latch; a passing gate with
//! the latch clear acts once and sets it; the key up clears it. A press
//! sets it. L's latch, `_dockTryFlag`, is set only by a dock request
//! (@0x6a953): a press that found nothing to land on, or that the flight
//! refused before its pick, leaves it clear, so L held reads again on
//! every pass ([`FlightView::press_land`]). It is no `_SetAllKeyCmdFlags`
//! latch, so L is held across every close (the `map_wait` module's
//! `OWN_LATCH`), and the step carries it as the dock approach's
//! hold ([`StepInput::dock_try`](nova_sim::flight_frame::StepInput::dock_try)).
//! I's is, and a site sets it ([`FlightView::set_every_latch`]). A pass in
//! the pod or dead reaches neither block, so neither latch clears there. Dialog keys stay press-only otherwise, by the user's decision:
//! M, P and Y don't act on being held. A key let go of as a dialog or a
//! refusal takes the latched keys is held no more until it goes down
//! afresh: ours has no live read to see it still down, so an OS repeat
//! doesn't count ([`FlightView::track_held`]), and the latch it would have
//! met has nothing to do.
//!
//! **No note in the core.** The session reports the collapse in the step
//! and keeps nothing after it. The view carries the fact only in the
//! settle's commands, so a press on any later frame takes the ordinary
//! path and acts. This is why [`JumpEffect::faded`](crate::flight::jump::JumpEffect::faded)
//! isn't the test here: it is true on the arrival's frame too, where a
//! press must act.
//!
//! Left out:
//!
//! - `_DoSystemMap` from any other caller, the hypergate @0x6385b among
//!   them: the flag is clear whenever those run, so the guard can't fire.
//! - `_DoPortDialog` @0x5f926: unreachable, as @0x66af7 refuses a
//!   collapsed player's dock.
//! - `_ShowTextDialog` and `_OfferOneMission`: their reads of the flag
//!   aren't refusals. `nova_view::ui::faded_texts` has them.
//! - `_IterateInGameEventLoop` @0x56a0c: a close box needs Run In A
//!   Window, which skips the white (@0x683a9). `_HandleQuitEvent`
//!   @0xcc9c8: ours already quits without asking.
//! - The refused opens' `_ShowTheCursor` (@0x6a830 and kin): `_HandleMouse`
//!   hides it again in the same frame (@0x38010), so nothing shows.
//! - `_SetRetainedWindow`, `_HideCursorSprite`, `_RefreshSAT`,
//!   `_RedrawStatus` and `_RedrawComm` around the refusals: repaints, and
//!   ours draws every frame whole.
//! - Which pass of a tick a press went down on: the view reads presses
//!   per frame and has no finer time, so a press kept for a tick counts
//!   for that tick's collapse. The original could read it a pass earlier
//!   and latch it.
//! - Run In A Window: the engine fades its own frame in a window too (the
//!   `warp_stage` docs in `nova_sim`), so the refusal runs there as well,
//!   where the original has no flag: the same accepted reading.

#[cfg(doc)]
use nova_sim::flight_frame::{FlightCommand, FlightFrame, StepRun};

use nova_sim::Takes;

use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::escorts::EscortMenuLooks;
use crate::flight::sim_port::FlightSim;
use crate::galaxy::GalaxyCatalog;
use crate::keymap::{Command, FlightCommand as F, PlayCommand};
use crate::system::catalog::SystemCatalog;
use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};
use crate::{Input, UiSound};

use super::FlightView;

/// The map key, the land key's and the rest kept through a jump, in the
/// pass's order: M, L, P, Y (with Option too) and I.
const KEPT: [Command; 6] = [
    Command::Play(PlayCommand::GalaxyMap),
    Command::Flight(F::LandDock),
    Command::Play(PlayCommand::PlayerInfo),
    Command::Flight(F::Communicate),
    Command::Flight(F::HailStellar),
    Command::Play(PlayCommand::MissionInfo),
];

/// Whether `command` is Communicate, with Option or not.
const fn is_hail(command: Command) -> bool {
    matches!(command, Command::Flight(F::Communicate | F::HailStellar))
}

/// The presses kept through a jump, and L's and I's latches (see the
/// module docs).
#[derive(Clone, Debug, Default)]
pub(super) struct FadedKeys {
    /// The commands pressed during the jump, not yet read.
    kept: Vec<Command>,
    /// L's latch (0xdee34).
    land_latch: bool,
    /// I's latch (0xdee00).
    mission_latch: bool,
}

impl FadedKeys {
    /// Keeps `press` if it is one of [`KEPT`]: whether it was.
    pub(super) fn keep(&mut self, press: Option<Command>) -> bool {
        let kept = press.filter(|command| KEPT.contains(command));
        self.kept.extend(kept);
        kept.is_some()
    }

    /// Drops the presses kept.
    pub(super) fn forget(&mut self) {
        self.kept.clear();
    }

    /// Sets I's latch, as `_SetAllKeyCmdFlags` does (0xdee00); L's is
    /// not among its latches.
    pub(super) fn set_mission_latch(&mut self) {
        self.mission_latch = true;
    }

    /// Whether L's latch is set.
    pub(super) fn land_latched(&self) -> bool {
        self.land_latch
    }

    /// `command`'s latch, L's or else I's.
    fn latch_mut(&mut self, command: Command) -> &mut bool {
        if command == Command::Flight(F::LandDock) {
            &mut self.land_latch
        } else {
            &mut self.mission_latch
        }
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
    /// A key's `input`, its press `press`, during a jump: whether that's
    /// all of it. One of [`KEPT`] is kept for the pass that reads it,
    /// the map shut, and tracked as down, whatever comes of it (@0x702d8
    /// has no gate); with the map open P goes nowhere, and the rest are
    /// the map's.
    pub(super) fn stop_during_jump(&mut self, press: Option<Command>, input: &Input) -> bool {
        if self.map_open {
            return press == Some(Command::Play(PlayCommand::PlayerInfo));
        }
        let kept = self.faded_keys.keep(press);
        if kept {
            self.track_held(input);
        }
        kept
    }

    /// Each of L and I down with its latch clear, which a pass reads
    /// besides the presses kept; when `clearing`, each latch cleared with
    /// its key up or its gate shut (@0x6a95d, @0x705af). A pass in the pod
    /// or dead returns before either block (@0x68ddd, @0x68853), so it
    /// clears neither: L held from before an eject still waits on its
    /// latch after the rescue.
    fn waiting_keys(&mut self, clearing: bool) -> Vec<Command> {
        let reached = self.frame.fate.row().input != Takes::Nothing;
        [
            Command::Flight(F::LandDock),
            Command::Play(PlayCommand::MissionInfo),
        ]
        .into_iter()
        .filter(|&command| {
            let down = self.reads_down(command);
            let taken = self.press_taken(Some(command), false).is_some();
            let latch = self.faded_keys.latch_mut(command);
            let waiting = down && !*latch;
            if clearing && reached && !(down && taken) {
                *latch = false;
            }
            waiting
        })
        .collect()
    }

    /// A press of L read: the land key pressed, and its latch set only if
    /// the press made a dock request (@0x6a938-0x6a953): not when the
    /// flight refused it before its pick or the pick found nothing
    /// ([`FlightView::land`]'s answer), so L held with nothing to land
    /// reads again on every pass, silently, as the original's does. It is
    /// the keyboard's and the polls' alike.
    pub(super) fn press_land(&mut self) {
        if self.land() {
            self.faded_keys.land_latch = true;
        }
    }

    /// A press of I read: its latch set (@0x7047a), and the mission info
    /// asked for. It is the keyboard's and the polls' alike.
    pub(super) fn press_mission_info(&mut self) {
        self.faded_keys.mission_latch = true;
        self.ask_mission_info();
    }

    /// The top of a tick, before the held keys' polls: during a jump, L's
    /// and I's latches cleared, as their failed gates clear them; past
    /// one, each key still kept is read if down and dropped if not, and L
    /// and I read on their latches, each as a plain press (see the module
    /// docs). Whether one opened something, which ends the tick.
    pub(super) fn faded_keys_frame(&mut self) -> bool {
        if self.jump_under_way() {
            self.faded_keys.land_latch = false;
            self.faded_keys.mission_latch = false;
            return false;
        }
        let kept = std::mem::take(&mut self.faded_keys.kept);
        let mut reads: Vec<Command> = kept
            .into_iter()
            .filter(|&command| self.kept_still_held(command))
            .collect();
        reads.extend(self.waiting_keys(true));
        if reads.is_empty() {
            return false;
        }
        self.read_keys(&reads, false);
        self.opened_something()
    }

    /// Whether a key read opened something that takes flight's input: its
    /// map, a dialog for the router, a landing or a movie.
    pub(super) fn opened_something(&self) -> bool {
        self.map_open
            || self.pending_player_info
            || self.pending_mission_info.is_some()
            || self.hail_opened()
            || self.pending_boarding
            || self.pending_landing.is_some()
            || self.playing()
    }

    /// Right after a tick's steps, which began with a jump under way as
    /// `jumping` says, once one has run: settles the keys kept by what the
    /// steps left (see the module docs).
    pub(super) fn settle_faded_keys(&mut self, jumping: bool) {
        if !jumping || self.step_report.steps.is_empty() {
            return;
        }
        if self.frame.warp_end_due {
            self.faded_keys.kept.retain(|&command| {
                command == Command::Play(PlayCommand::PlayerInfo) || is_hail(command)
            });
        } else if self.jump_under_way() {
            let kept = std::mem::take(&mut self.faded_keys.kept);
            if kept.contains(&Command::Play(PlayCommand::PlayerInfo)) {
                self.refuse_player_info();
            }
            if let Some(&hail) = kept.iter().find(|&&command| is_hail(command)) {
                let stellar = self.hail_at_stellar(hail);
                self.hail_at(stellar);
            }
        } else {
            let faded = self.step_report.steps.iter().any(|run| run.lit_collapse);
            let mut reads = std::mem::take(&mut self.faded_keys.kept);
            reads.extend(self.waiting_keys(false));
            self.read_keys(&reads, faded);
        }
    }

    /// Whether the key of `command`, a press kept, is still held: for
    /// Communicate, with Option or not, as Option may have changed since
    /// the press.
    fn kept_still_held(&self, command: Command) -> bool {
        if is_hail(command) {
            self.held(F::Communicate) || self.held(F::HailStellar)
        } else {
            self.held(command)
        }
    }

    /// Whether Communicate, pressed as `pressed` and read now, hails the
    /// navigation stellar: as Option is now while its key is still held
    /// ([`F::HailStellar`]), as the original reads Option at the pass, and
    /// otherwise as its press was.
    fn hail_at_stellar(&self, pressed: Command) -> bool {
        if self.held(F::Communicate) || self.held(F::HailStellar) {
            self.held(F::HailStellar)
        } else {
            pressed == Command::Flight(F::HailStellar)
        }
    }

    /// One pass's reads of the commands `reads`, in its order, on a
    /// `faded` screen or not (see the module docs): M, then L, then P, Y
    /// and I until one of them latches the rest.
    fn read_keys(&mut self, reads: &[Command], faded: bool) {
        let read = |command: Command| {
            reads.contains(&command) && self.press_taken(Some(command), false).is_some()
        };
        let (map, land, player) = (
            read(Command::Play(PlayCommand::GalaxyMap)),
            read(Command::Flight(F::LandDock)),
            read(Command::Play(PlayCommand::PlayerInfo)),
        );
        let hail = reads
            .iter()
            .copied()
            .find(|&command| is_hail(command) && read(command));
        let mission = read(Command::Play(PlayCommand::MissionInfo));
        if map {
            self.read_map(faded);
        }
        if land {
            self.press_land();
        }
        if map
            || (player && self.read_player_info(faded))
            || hail.is_some_and(|hail| self.read_hail(hail, faded))
        {
            return;
        }
        if mission {
            self.read_mission_info(faded);
        }
    }

    /// M, read on a `faded` screen: `_DoSystemMap` refuses between its two
    /// beeps, and the latched keys go. Otherwise it opens the map, which
    /// waits on M's release only if M is still down: a press kept for the
    /// tick may have gone up already.
    fn read_map(&mut self, faded: bool) {
        if faded {
            self.ui_sounds.push(UiSound::MapOpened);
            self.ui_sounds.push(UiSound::MapClosed);
            self.keep_unlatched_keys();
        } else {
            let down = self.held(PlayCommand::GalaxyMap);
            self.open_map_by_key();
            self.map_key_held = down;
        }
    }

    /// P, read on a `faded` screen or not: whether it latched the keys
    /// after it, refused by the guard or opening its dialog.
    fn read_player_info(&mut self, faded: bool) -> bool {
        if faded {
            return self.press_faded_player_info();
        }
        self.ask_player_info();
        self.pending_player_info
    }

    /// Y, pressed as `pressed`, read on a `faded` screen or not: whether
    /// it latched the keys after it, refused by the guard or opening
    /// something.
    fn read_hail(&mut self, pressed: Command, faded: bool) -> bool {
        let stellar = self.hail_at_stellar(pressed);
        if faded {
            return self.hail_faded(stellar);
        }
        self.hail_at(stellar);
        self.hail_opened()
    }

    /// I, read on a `faded` screen or not.
    fn read_mission_info(&mut self, faded: bool) {
        if faded {
            self.faded_keys.mission_latch = true;
            self.press_faded_mission_info();
        } else {
            self.press_mission_info();
        }
    }
}
