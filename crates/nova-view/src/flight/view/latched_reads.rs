//! What [`FlightView`] reads down past a latch. All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386).
//!
//! `_HandlePlayer` polls every key from `GetKeys` (`_QuickKeyCheck`
//! @0xa0f5), so a key held past a close that `_SetAllKeyCmdFlags`
//! (@0x61e6d) latched still reads down: only the command behind its latch
//! waits for its release. Ours keeps such a key out of
//! [`FlightView::held`] ([`FlightView::latched_down`]), so its OS repeats
//! are no presses, and sets ours' copy of its latch as the original's site
//! does ([`FlightView::set_every_latch`], [`FlightView::latch_key`]). The
//! frames that poll a latched key read it here
//! ([`FlightView::reads_down`], [`FlightView::keys_down`]): it reads down
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

use crate::Key;
use crate::flight::sim_port::FlightSim;
use crate::ui::mission_info::MISSION_INFO_KEY;

use super::{
    FlightView, HYPER_SELECT_KEY, HYPERSPACE_MODE_KEY, JUMP_KEY, NAV_OFF_KEY, TARGET_KEY,
    slot_block,
};

impl<C, S: FlightSim> FlightView<C, S> {
    /// Whether `_QuickKeyCheck` (@0xa0f5) reads `key` down: held, or held
    /// past a close that latched it ([`FlightView::latched_down`]).
    pub(super) fn reads_down(&self, key: Key) -> bool {
        self.held.contains(&key) || self.latched_down.contains(&key)
    }

    /// Every key `_QuickKeyCheck` reads down ([`FlightView::reads_down`]):
    /// the held ones and the latched ones, which never share a key.
    pub(super) fn keys_down(&self) -> impl Iterator<Item = Key> + '_ {
        self.held.iter().chain(&self.latched_down).copied()
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
    /// `OWN_LATCH_KEYS`). A latch whose keys are up clears at the next
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

    /// Sets `key`'s latch, as [`FlightView::set_every_latch`] sets it:
    /// for a key down at the poll before a dialog no site follows, whose
    /// latch that poll set.
    pub(super) fn latch_key(&mut self, key: Key) {
        match key {
            NAV_OFF_KEY | HYPERSPACE_MODE_KEY => self.nav_latch = true,
            HYPER_SELECT_KEY => self.select_latch = true,
            JUMP_KEY => self.jump_latch = true,
            MISSION_INFO_KEY => self.faded_keys.set_mission_latch(),
            TARGET_KEY => self.target_latch = true,
            _ if slot_block::is_block_key(key) => self.slot_block.set_latch(),
            _ => {}
        }
    }
}
