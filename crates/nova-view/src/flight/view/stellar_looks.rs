//! [`FlightView`]'s scene as the kept frame holds its stellars, over the
//! [`FlightSim`] port: which are shown, how each looks and the frame its
//! session reports it shows
//! ([`FrameStellar`](nova_sim_frame::flight_frame::FrameStellar)), and the
//! starfield scattered about the frame's ship.
//! Any flight drives them, a scripted one in the component tests.

use nova_sim_frame::SystemId;

use crate::flight::catalog::{CombatLooks, ShipSprites};
use crate::flight::sim_port::FlightSim;
use crate::system::catalog::SystemCatalog;

use super::between_passes;
use super::stars;
use super::{FlightView, Redrawn, show_stellars, shown_scene};

impl<C, S: FlightSim> FlightView<C, S> {
    /// Moves the flight's clock on by `dt`, which the escort menu, the
    /// floating map and the drawing read. The stellars take no part: their
    /// session steps their frames.
    pub(super) fn advance_clock(&mut self, dt: std::time::Duration) {
        self.elapsed += dt;
    }

    /// Draws each stellar of the scene shown as the frame drawn holds it
    /// ([`show_stellars`]): its look and the frame its session reports.
    /// The scene is not laid out again.
    pub(super) fn show_stellars(&mut self) {
        let drawn = between_passes::drawn(self.between_passes.as_ref(), &self.frame);
        if let Some(scene) = &mut self.scene {
            show_stellars(scene, drawn);
        }
    }

    /// Scatters the stars afresh about the frame's ship and places them
    /// once, on the effects' chance, when the flight draws the original's
    /// starfield (see the `stars` module): never for a flight that failed,
    /// whose looks hold no stars' sheet, nor under the layered sky.
    pub(super) fn reset_stars(&mut self) {
        let starfield = self.presentation.starfield.stars();
        if starfield.is_some_and(|source| stars::original(source, &self.looks)) {
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
    /// Lets go of the scene shown and shows the frame's in its place, read
    /// from the catalog with the stellars the kept frame shows
    /// ([`shown_scene`]), each stellar looking as the frame holds it and
    /// on the frame its session reports ([`show_stellars`]): the session
    /// keeps each stellar's frames across systems, so one shown again is
    /// where it was left (see `nova_sim::Session`'s `stellar_frames`
    /// module).
    pub(super) fn show_scene(&mut self) {
        let mut scene = shown_scene(&*self.catalog, &self.frame);
        show_stellars(&mut scene, &self.frame);
        self.scene = Some(scene);
    }
}

/// The scene's redraws as the flight's events ask for them, over the port:
/// the player's sheet, the layout or the ship's place, the scene read
/// again, the stellars' looks and the stars, all from the kept frame.
impl<C: SystemCatalog + ShipSprites + CombatLooks, S: FlightSim> FlightView<C, S> {
    /// [`FlightView::catch_up`]'s redraws, given what the events asked for
    /// (`redrawn`), in their order: the ship's sheet, the bar (`bar`), the
    /// layout or the ship's place, the scene read again, the stellars'
    /// looks and frames ([`FlightView::show_stellars`]), the status bar
    /// (`status`, on a system entered) and the stars.
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
        if redrawn.system {
            self.lay_out(system);
        } else if redrawn.stellar {
            self.place_ship();
        }
        if reshown {
            self.show_scene();
        }
        self.show_stellars();
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
    /// filled afresh, has it, the last system's NPCs and effects let go,
    /// the NPCs' flashes and glows held for the escorts the fleet keeps
    /// ([`HeldDisplays`](crate::flight::carried_displays::HeldDisplays)).
    fn lay_out(&mut self, system: SystemId) {
        self.drop_between_passes();
        self.show_scene();
        self.map.show_course(system, &self.frame.course_shown);
        self.previous = self.player_place();
        self.alpha = 0.0;
        self.npc_previous.clear();
        self.effects.clear();
        self.held_displays.hold(&mut self.flashes, &mut self.glows);
        self.held_alts.get_mut().clear();
        self.turret_swings.clear();
        self.shot_looks.clear();
        self.read_npc_sheets();
    }

    /// Draws the ship where the flight has it, not on its way from where
    /// it was, and shows the flight's course again on the course map:
    /// after a move to another stellar of the system shown, a take-off and
    /// a new pilot's beginning. Its place is the kept frame's
    /// ([`FlightView::player_place`]), which the change filled afresh.
    pub(super) fn place_ship(&mut self) {
        self.drop_between_passes();
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
            (self.sheet, self.outline) =
                super::ship_looks::read_ship(&*self.catalog, &self.ship_looks, ship);
            self.sheet_ship = Some(ship);
        }
    }
}
