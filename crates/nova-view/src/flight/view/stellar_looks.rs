//! [`FlightView`]'s scene as the kept frame holds its stellars, over the
//! [`FlightSim`] port: which are shown, how each looks
//! ([`FrameStellar`](nova_sim::flight_frame::FrameStellar)) and where its
//! animation stands, and the starfield scattered about the frame's ship.
//! Any flight drives them, a scripted one in the component tests.

use std::time::Duration;

use nova_sim::SystemId;

use crate::flight::catalog::{CombatLooks, ShipSprites};
use crate::flight::sim_port::FlightSim;
use crate::system::catalog::SystemCatalog;

use super::stars;
use super::{FlightView, Redrawn, look, show_destroyed, show_gate_frames, shown_scene};

impl<C, S: FlightSim> FlightView<C, S> {
    /// Draws each stellar of the scene shown as the kept frame holds it
    /// after a rebuild point
    /// ([`SystemScene::rebuild`](crate::system::scene::SystemScene::rebuild)):
    /// one whose look changed shows its current frame. The scene is not
    /// laid out again.
    pub(super) fn rebuild_stellars(&mut self) {
        if let Some(scene) = &mut self.scene {
            scene.rebuild(|id| look(&self.frame, id));
        }
    }

    /// Draws each stellar of the scene shown as the kept frame holds it
    /// ([`show_destroyed`]). The scene is not laid out again.
    pub(super) fn show_destroyed(&mut self) {
        if let Some(scene) = &mut self.scene {
            show_destroyed(scene, &self.frame);
        }
    }

    /// The take-off's reset of every stellar's frames
    /// (`_HandlePlayerDockRequest` @0x675ce-0x675fa, play-feel-4 phase
    /// 14; `nova_sim::SessionEvent::StellarFramesReset`): those of the
    /// scene shown
    /// ([`SystemScene::take_off_reset`](crate::system::scene::SystemScene::take_off_reset))
    /// and those kept
    /// ([`KeptFrames::reset`](crate::system::kept_frames::KeptFrames::reset)).
    /// [`FlightView::redraw_scene_then`] runs it before the scene is laid
    /// out or drawn again, so a gate's exit keeps the old scene's frames
    /// zeroed and the new scene resumes them zeroed, as the original
    /// zeroes every `spöb` before its rebuild (@0x67653). The session
    /// raises the event at the take-off or the exit, and the view settles
    /// it in the catch-up that follows, before the first frame is drawn.
    pub(super) fn reset_stellar_frames(&mut self) {
        self.kept_frames.reset();
        if let Some(scene) = &mut self.scene {
            scene.take_off_reset();
        }
    }

    /// Moves the clock on by `dt` and steps the stellars' animations by
    /// the whole ticks of 1/30 s it crossed, as the original's sprite tick
    /// runs every play frame, the jump's included. A random next frame
    /// (`Flags2` 0x0002) is drawn on the effects' chance, never the
    /// session's: the frame a stellar shows is drawn state only.
    pub(super) fn step_stellars(&mut self, dt: Duration) {
        let before = crate::time::ticks(self.elapsed);
        self.elapsed += dt;
        if let Some(scene) = &mut self.scene {
            scene.step(
                crate::time::ticks(self.elapsed) - before,
                &mut self.effects_chance,
            );
        }
    }

    /// Scatters the stars afresh about the frame's ship and places them
    /// once, on the effects' chance, when the flight draws the original's
    /// starfield (see the `stars` module): never for a flight that failed,
    /// whose looks hold no stars' sheet.
    pub(super) fn reset_stars(&mut self) {
        if stars::original(self.presentation.starfield, &self.looks) {
            let at = stars::surroundings(&self.frame, self.view, self.anchor(), &self.looks);
            stars::reset(
                &mut self.stars,
                &at,
                self.parallax_starfield,
                &mut self.effects_chance,
            );
        }
    }
}

impl<C: SystemCatalog, S: FlightSim> FlightView<C, S> {
    /// Lets go of the scene shown, keeping each of its stellars'
    /// animation state
    /// ([`KeptFrames::keep`](crate::system::kept_frames::KeptFrames::keep)),
    /// and shows the frame's in its place, read from the catalog with the
    /// stellars the kept frame shows ([`shown_scene`]): each
    /// stellar looking as the frame holds it ([`show_destroyed`]), then each
    /// the flight has shown before resumed where it was left, on the sheet
    /// it now draws
    /// ([`SystemScene::resume`](crate::system::scene::SystemScene::resume)),
    /// and each hypergate on its frame's frame. The look is set before the
    /// frames resume, so the rebuild or redraw that follows finds it
    /// unchanged and leaves them alone.
    pub(super) fn show_scene(&mut self) {
        if let Some(left) = &self.scene {
            self.kept_frames.keep(left);
        }
        let mut scene = shown_scene(&*self.catalog, &self.frame);
        show_destroyed(&mut scene, &self.frame);
        scene.resume(&self.kept_frames);
        show_gate_frames(&mut scene, &self.frame);
        self.scene = Some(scene);
    }
}

/// The scene's redraws as the flight's events ask for them, over the port:
/// the player's sheet, the layout or the ship's place, the scene read
/// again, the stellars' looks and the stars, all from the kept frame.
impl<C: SystemCatalog + ShipSprites + CombatLooks, S: FlightSim> FlightView<C, S> {
    /// [`FlightView::catch_up`]'s redraws, given what the events asked for
    /// (`redrawn`), in their order: the ship's sheet, the bar (`bar`), the
    /// stellars' frames reset ([`FlightView::reset_stellar_frames`]), the
    /// layout or the ship's place, the scene read again, the stellars'
    /// looks, the status bar (`status`, on a system entered) and the stars.
    /// The kept frame was filled afresh as the events were drained
    /// ([`FlightView::catch_up_then`]), so each reads the flight as the
    /// events left it. `bar` and `status` are the session's still
    /// (rdm:phase/view-sim-seam/phase-8-hud-and-target-frame), split at the
    /// concrete callee.
    pub(super) fn redraw_scene_then(
        &mut self,
        redrawn: Redrawn,
        bar: impl FnOnce(&mut Self),
        status: impl FnOnce(&mut Self),
    ) {
        let system = self.frame.system;
        let reshown = redrawn.rebuilt
            && !redrawn.system
            && self.scene.as_ref().is_some_and(|scene| {
                !scene
                    .stellars()
                    .iter()
                    .map(|stellar| stellar.id)
                    .eq(self.frame.stellars_shown())
            });
        if redrawn.ship {
            self.follow_ship();
        }
        bar(self);
        if redrawn.frames_reset {
            self.reset_stellar_frames();
        }
        if redrawn.system {
            self.lay_out(system);
        } else if redrawn.stellar {
            self.place_ship();
        }
        if reshown {
            self.show_scene();
        }
        if redrawn.rebuilt {
            self.rebuild_stellars();
        } else if redrawn.galaxy || redrawn.system {
            self.show_destroyed();
        }
        if redrawn.system {
            status(self);
        }
        if std::mem::take(&mut self.stars_due) || redrawn.stars {
            self.reset_stars();
        }
    }

    /// Lays out `system`, the frame's: its scene ([`FlightView::show_scene`]),
    /// each stellar shown before resuming its animation where it was left,
    /// the course map's course, and the ship drawn from where the frame,
    /// filled afresh, has it, the last system's NPCs and effects let go.
    fn lay_out(&mut self, system: SystemId) {
        self.show_scene();
        self.map.show_course(system, &self.frame.course_shown);
        self.previous = self.player_place();
        self.alpha = 0.0;
        self.npc_previous.clear();
        self.effects.clear();
        self.flashes.clear();
        self.held_alts.get_mut().clear();
        self.glows.clear();
        self.shot_looks.clear();
        self.read_npc_sheets();
    }

    /// Draws the ship where the flight has it, not on its way from where
    /// it was, and shows the flight's course again on the course map:
    /// after a move to another stellar of the system shown, a take-off and
    /// a new pilot's beginning. Its place is the kept frame's
    /// ([`FlightView::player_place`]), which the change filled afresh.
    pub(super) fn place_ship(&mut self) {
        self.previous = self.player_place();
        self.show_course();
        self.alpha = 0.0;
    }

    /// Reads the player's sprite sheet afresh when the frame's ship is of
    /// a class other than the one it was read for, so a ship bought,
    /// captured or changed by a set expression is drawn in its own
    /// sprites; called only when the flight tells the ship changed
    /// ([`FlightView::catch_up`]), which it may tell of the class it was.
    /// A sheet that cannot be read is kept against its class too, so it is
    /// not tried again. The status bar follows on its own
    /// (`FlightView::follow_bar`).
    fn follow_ship(&mut self) {
        let ship = self.frame.player().class;
        if self.sheet_ship != Some(ship) {
            self.sheet = self.catalog.ship_sheet(ship);
            self.sheet_ship = Some(ship);
        }
    }
}
