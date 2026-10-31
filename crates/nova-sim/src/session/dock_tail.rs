//! The tail of `_HandlePlayerDockRequest` (@0x66f40-0x67682 in
//! `EV Nova.app`'s `EV Nova`, i386), shared by its three ways in: the
//! take-off (`Session::lift_off`, under [`Session::take_off`]), a gate's exit (`Session::come_out`,
//! from [`Session::enter_hypergate`] and [`Session::enter_wormhole`]) and
//! a wormhole that leads nowhere (`Session::refuse_no_exit`; see the
//! `wormhole_no_exit` module).
//!
//! The tail runs in two halves around the system's setup, which each
//! caller makes its own way:
//!
//! - [`Session::begin_dock_tail`], before the setup: the ship's fold and
//!   player block (@0x66f72-0x67020), the escort-strength timer
//!   (@0x6706f-0x67084) and `_ResetPlayerPrecalcedValues` (@0x6709b; see
//!   the `precalced` module), whose `_ResetPlayer` writes -1 to +0xc908
//!   (see the `stray_words` module). All three callers run it.
//! - [`Session::end_dock_tail`], after the setup and the frames' reset:
//!   the rebuild (@0x674c7, @0x67653), the dock request dropped
//!   (@0x6705f, @0x6374a-0x63759; see the `clearance` module), both
//!   `_PrepareAllSprites`' stellar passes (@0x6764e, @0x67658; see the
//!   `stellar_passes` module) and the second's ship part, `populate`'s
//!   the first, the nav
//!   selection let go with the mode kept, and the course cued
//!   (@0x674d2-0x674d8; see the `nav` module). The take-off and the
//!   wormhole with no exit run it; a gate's exit runs its own loops and
//!   sprites instead.
//!
//! What the callers keep apart: the placement, the exploration, the
//! days, the save, the arrival sounds and messages, and a gate exit's
//! loops. Each caller populates the system (`_DeleteAllComputerShips`
//! @0x67096, `_SetupShipsInSystem` @0x674c2) and resets the stellars'
//! frames (@0x675ce-0x675fa; see the `takeoff_frames` module) between
//! the halves, the population first.
//!
//! The order within each half is the take-off's. Only the deselect
//! before the cue, and the population before the frames' reset, the
//! rebuild and the sprites' preparation, matter: everything settles
//! before the next frame is drawn.

use super::Session;
use super::stellar_passes::GateHolds;
use crate::chance::Chance;
use crate::gate_sprite::StellarPass;

impl Session {
    /// The dock request's tail before the system is set up (see the
    /// module docs): the ship's fold and player block, the
    /// escort-strength timer armed on `chance`, the precalced values
    /// reset and the pod word read as flying again.
    pub(super) fn begin_dock_tail(&mut self, chance: &mut (impl Chance + ?Sized)) {
        // The fold and the player block (@0x66f72-0x67020).
        self.ship.dock_tail_reset();
        // `_antiEscortDelay` (@0x6706f-0x67084).
        self.space.arm_strength_timer(chance);
        // `_ResetPlayerPrecalcedValues` (@0x6709b), whose `_ResetPlayer`
        // writes -1 to +0xc908.
        self.reset_precalced();
        self.ship.fate.pod_zeroed = false;
    }

    /// The dock request's tail once the system is set up and its frames
    /// reset (see the module docs): the stellars rebuilt, the dock request
    /// dropped, both `_PrepareAllSprites`' stellar passes run as `pass`,
    /// their random picks drawn on `chance`, every sprite prepared again,
    /// the nav selection let go and the course cued.
    pub(super) fn end_dock_tail(&mut self, pass: StellarPass, chance: &mut (impl Chance + ?Sized)) {
        // The rebuild point (@0x674c7, @0x67653).
        self.rebuild_stellars();
        // `_dockPermission` = 0 (@0x67064), before either pass reads it.
        self.port.dock = None;
        // @0x6764e's stellar pass: after the frames' reset, and after a
        // rebuild, so here, though its ship part was `populate`'s. It
        // follows @0x674c7's rebuild in the original and the only one
        // after the reset in the engine: the session's sites are as the
        // rebuild left them either way.
        self.stellar_pass(pass, GateHolds::AsTheyStand, chance);
        // The second `_PrepareAllSprites` (@0x67658), `populate`'s being
        // the first (@0x6764e): every NPC's fade steps again.
        self.prepare_all_sprites();
        self.stellar_pass(pass, GateHolds::AsTheyStand, chance);
        // The selection let go, the mode kept, then the course cued
        // (@0x674d2-0x674d8).
        self.chart.deselect();
        self.cue_course();
    }
}
