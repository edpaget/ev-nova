//! What [`FlightView`] reads down past a latch. All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386).
//!
//! `_HandlePlayer` polls every key from `GetKeys` (`_QuickKeyCheck`
//! @0xa0f5), so a key held past a close that `_SetAllKeyCmdFlags`
//! (@0x61e6d) latched still reads down: only the command behind its latch
//! waits for its release. Ours keeps such a key's commands out of
//! [`FlightView::held`] ([`FlightView::latched_down`]), so its OS repeats
//! are no presses, and sets ours' copy of its latch as the original's site
//! does ([`FlightView::set_every_latch`], [`FlightView::latch_command`]). The
//! frames that poll a latched key read it here
//! ([`FlightView::reads_down`], [`FlightView::commands_down`]): it reads down
//! with its latch set, so its command waits, and every read that skips
//! the latch sees it down. That is the cursor's fade (each latched block
//! sets `_keyPressedThisFrame` *0x3b5550 before its latch test), the
//! zoom's hold (`_quickKeyCheckResult` *0x3b504c, set by every read down,
//! @0xa145-0xa14c), the floating map's shows (\`/H @0x69b8e-0x69ba3, `\`
//! @0x69cdd-0x69ce7) and the latches keys share (H's for \`, a slot key's
//! for the rest of its block, Tab's for the click).
//!
//! The press handlers read [`FlightView::held`] alone: a latched key
//! reaches its press only after its release. So do the step's controls
//! (the cloak's edge among them) and the escort menu.

use crate::flight::sim_port::FlightSim;
use crate::keymap::{Command, Context, EscortMenuCommand, FlightCommand, Kind, PlayCommand};

use super::FlightView;

impl<C, S: FlightSim> FlightView<C, S> {
    /// Whether flight holds `command`: its key is down, as flight's reader
    /// follows it, and no close latched it ([`FlightView::latched_down`]).
    pub(super) fn held(&self, command: impl Into<Command>) -> bool {
        let command = command.into();
        self.keys.held(command) && !self.latched_down.contains(&command)
    }

    /// Whether `_QuickKeyCheck` (@0xa0f5) reads `command` down: held, or
    /// held past a close that latched it ([`FlightView::latched_down`]).
    pub(super) fn reads_down(&self, command: impl Into<Command>) -> bool {
        self.keys.held(command)
    }

    /// Every held command `_QuickKeyCheck` reads down
    /// ([`FlightView::reads_down`]), the latched ones among them: the
    /// commands of [`Kind::Held`], which the original polls every frame.
    pub(super) fn commands_down(&self) -> impl Iterator<Item = Command> + '_ {
        self.keys
            .held_commands()
            .filter(|command| command.kind() == Kind::Held)
    }

    /// Switches the escort menu's group keys on while the menu shows,
    /// fading too, and off otherwise, so 1-5 mean its groups only then
    /// (`_QuickKeyCheck` @0xa106-0xa124).
    pub(super) fn sync_escort_keys(&mut self) {
        let shown = self.escort_menu.is_open();
        if self.keys.active().contains(&Context::EscortMenu) != shown {
            self.keys.set_active(Context::EscortMenu, shown);
            self.latched_down.retain(|&command| self.keys.held(command));
        }
    }

    /// Sets every latch ours keeps of those `_SetAllKeyCmdFlags`
    /// (@0x61e6d) sets: `_navModeSelectFlag` (0xdee04,
    /// [`FlightView::nav_latch`]), `_navTargetSelectFlag` (0xdee48,
    /// [`FlightView::select_latch`]), `_navDefaultSelectFlag` (0xdede0, the
    /// slot block's), `_jumpTryFlag` (0xdee44, [`FlightView::jump_latch`]),
    /// `_missionInfoFlag` (0xdee00, I's) and `_weapTargetSelectFlag`
    /// (0xdede8, [`FlightView::target_latch`]). The other pointers it
    /// sets latch keys ours acts on at their press (M, R, W, Return, E,
    /// F12, P, Y, the escort commands, the zooms), which a latched key
    /// reaches only after its release, or U, whose edge the step reads.
    /// B's and L's latches (`_boardTryFlag` 0xdee18, `_dockTryFlag`
    /// 0xdee34) are not among them (the `map_wait` module's
    /// `OWN_LATCH`). A latch whose keys are up clears at the next
    /// poll, and a fresh press clears its own, so setting them all is
    /// safe.
    pub(super) fn set_every_latch(&mut self) {
        self.nav_latch = true;
        self.select_latch = true;
        self.slot_block.set_latch();
        self.jump_latch = true;
        self.faded_keys.set_mission_latch();
        self.target_latch = true;
    }

    /// Sets `command`'s latch, as [`FlightView::set_every_latch`] sets
    /// it: for a key down at the poll before a dialog no site follows,
    /// whose latch that poll set.
    pub(super) fn latch_command(&mut self, command: Command) {
        use FlightCommand as F;
        match command {
            Command::Flight(F::NavOff | F::ClearTarget | F::HyperspaceMode) => {
                self.nav_latch = true;
            }
            Command::Flight(F::HyperSelect) => self.select_latch = true,
            Command::Flight(F::HyperJump) => self.jump_latch = true,
            Command::Play(PlayCommand::MissionInfo) => self.faded_keys.set_mission_latch(),
            Command::Flight(
                F::TargetSelect | F::TargetSelectBack | F::TargetEscort | F::TargetEscortBack,
            ) => self.target_latch = true,
            Command::Flight(
                F::StellarSlot1
                | F::StellarSlot2
                | F::StellarSlot3
                | F::StellarSlot4
                | F::NearestStellar,
            )
            | Command::EscortMenu(
                EscortMenuCommand::Group1
                | EscortMenuCommand::Group2
                | EscortMenuCommand::Group3
                | EscortMenuCommand::Group4
                | EscortMenuCommand::Group5,
            ) => self.slot_block.set_latch(),
            _ => {}
        }
    }
}
