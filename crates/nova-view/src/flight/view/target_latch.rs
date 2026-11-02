//! Tab and the click in space sharing `_weapTargetSelectFlag` (0xdede8,
//! [`FlightView::target_latch`]), as `_HandlePlayer` polls them
//! (@0x69dea-0x6a76c). All addresses are in `EV Nova.app`'s `EV Nova`
//! (i386).
//!
//! - With Tab (+0x14) down the pass goes to Tab's branch (@0x69dfd) and
//!   `_Button` isn't read at all: no click reaches the escort menu
//!   (@0x6a060-0x6a12b), the floating map (@0x6a130) or space. Tab's
//!   branch sets `_keyPressedThisFrame` (@0x69f1c) and, with the latch
//!   clear, sets it (@0x69f32) and picks the next or previous ship; with
//!   it set, does nothing (@0x69f24). Nothing there clears it.
//! - With Tab up and the button down, the latch set does nothing
//!   (@0x6a015); clear, it is set (@0x6a01b) before the status panel's
//!   test (@0x6a065) and the menu's, the map's and space's clicks.
//! - With both up the latch clears (@0x6a764).
//!
//! Ours acts on Tab's and the button's presses, which follow a release,
//! so each press clears the latch first unless the other still holds it
//! down; a pass's poll clears it with both up
//! ([`FlightView::target_poll`]). Tab reads down past a close's latch
//! (the `latched_reads` module), and `_SetAllKeyCmdFlags` sets this latch
//! too, so a button held across a site's close keeps Tab from picking.
//!
//! No gate stands before the block but the pass's own early returns
//! (death @0x68853, the pod @0x68ddd, the rescue @0x692ce): ours' gates on
//! Tab (`PlayerCommand::Target`) and on the click (`PlayerCommand::Nav`)
//! refuse only where the fate takes nothing.

use crate::Point;
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::escorts::EscortMenuLooks;
use crate::flight::sim_port::FlightSim;
use crate::galaxy::GalaxyCatalog;
use crate::system::catalog::SystemCatalog;
use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};

use super::{FlightView, TARGET_KEY};

impl<C, S: FlightSim> FlightView<C, S> {
    /// The pass's poll of Tab and the button, after the slot block and
    /// before B, as @0x69dea sits between them: with Tab not read down
    /// and the button up, the latch cleared (@0x6a764).
    pub(super) fn target_poll(&mut self) {
        if !self.reads_down(TARGET_KEY) && !self.cursor.button_down() {
            self.target_latch = false;
        }
    }

    /// A press of Tab: the latch cleared as the release before it clears
    /// it, unless the button still holds it; then with it clear, Tab's
    /// pick ([`FlightView::press_target_key`]) and the latch set
    /// (@0x69f24-0x69f32).
    pub(super) fn press_target_latched(&mut self) {
        if !self.cursor.button_down() {
            self.target_latch = false;
        }
        if !std::mem::replace(&mut self.target_latch, true) {
            self.press_target_key();
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
    /// A left press at `at` in flight: with Tab read down, nothing at all
    /// (@0x69dfd); otherwise the latch cleared, as Tab is up and the press
    /// follows a release, then set (@0x6a01b) before the escort menu, the
    /// floating map and space are tried, in that order.
    pub(super) fn press_button_latched(&mut self, at: Point) {
        if self.reads_down(TARGET_KEY) {
            return;
        }
        self.target_latch = true;
        if !self.click_escort_menu(at) && !self.click_floating_map(at) {
            self.click_in_space(at);
        }
    }
}
