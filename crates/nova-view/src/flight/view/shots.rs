//! What the flight view keeps of the shots between steps, for drawing
//! them as the original does: each spinning shot's start
//! ([`SpinStarts`]), the weapons' trail tables ([`TrailTables`]), the
//! puffs of smoke ([`SmokePuffs`]) and the step's lightning rolls
//! ([`StepRolls`]). The simulation keeps none of it: it
//! is the view's, drawn on the effects' chance, never the simulation's.

use nova_sim::flight_frame::PassReport;
use nova_sim::player_wrap::PlayerWrap;
use nova_sim::{Chance, Vec2};

use crate::DrawList;
use crate::flight::catalog::Looks;
use crate::flight::effects::Effects;
use crate::flight::lightning::StepRolls;
use crate::flight::smoke::{SmokePuffs, smoke_kind};
use crate::flight::spin::SpinStarts;
use crate::flight::trails::TrailTables;
use crate::system::camera::Camera;

/// The view's state of the shots in flight.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct ShotLooks {
    /// Each spinning shot's start.
    pub(super) spin_starts: SpinStarts,
    /// Each trailing weapon's trail table, drawn once.
    trails: TrailTables,
    /// The smoke the shots have left.
    smoke: SmokePuffs,
    /// The rolls the step's lightning zig-zags are drawn on.
    lightning: StepRolls,
}

impl ShotLooks {
    /// Steps it after a `pass` of the fight, the shots' looks read
    /// from `looks`, drawing on `chance`, as `_HandleShot` then
    /// `_HandleSmoke` do each frame: each shot new in flight draws its
    /// spin start, and each gone is forgotten; with `smoke_trails` on,
    /// each shot in flight leaves its trail's burst in `effects`, under
    /// the `caps` or not, and, on a smoke frame
    /// ([`PassReport::smoke_frame`]), its puff of smoke; then the smoke ages,
    /// whether or not the preference is on.
    ///
    /// The Smoke Trails preference is the original's inverted
    /// `SmokeTrails` byte, which skips the particle trail
    /// (@0x35d64-0x35d6d) and `_SpawnSmoke` (@0x3f93d-0x3f951), but never
    /// `_HandleSmoke`.
    pub(super) fn step(
        &mut self,
        pass: &PassReport,
        looks: &Looks,
        (effects, caps): (&mut Effects, bool),
        smoke_trails: bool,
        chance: &mut dyn Chance,
    ) {
        let flags = |id| looks.weapon(id).map_or(0, |look| look.flags);
        let shots = pass.shots.iter().map(|shot| (shot.id, flags(shot.weapon)));
        self.spin_starts.step(shots, chance);
        let smoke_frame = pass.smoke_frame;
        for shot in pass.shots.iter().filter(|_| smoke_trails) {
            let Some(look) = looks.weapon(shot.weapon) else {
                continue;
            };
            let at = (shot.position, shot.heading);
            if let Some(burst) = self.trails.burst(shot.weapon, look, at, chance) {
                effects.spray(&burst, caps, chance);
            }
            if let Some(kind) = smoke_kind(look.flags).filter(|_| smoke_frame) {
                let sheet = looks.smoke_set(look.smoke_set);
                self.smoke.spawn(shot.position, kind, sheet, caps);
            }
        }
        self.smoke.step();
        let lightning = |id| looks.weapon(id).is_some_and(|look| look.li_density > 0);
        if pass.beam_weapons.iter().any(|&weapon| lightning(weapon)) {
            self.lightning = StepRolls::draw(chance);
        }
    }

    /// The rolls this step's lightning zig-zags are drawn on, the same
    /// for every frame drawn in the step ([`StepRolls`]).
    pub(super) fn lightning(&self) -> StepRolls {
        self.lightning
    }

    /// Draws the smoke where `camera` shows it, murked as seen from the
    /// player's ship at `viewer` in a system of effective murk `murk`.
    pub(super) fn draw_smoke(&self, list: &mut DrawList, camera: &Camera, viewer: Vec2, murk: u8) {
        self.smoke.draw(list, camera, viewer, murk);
    }

    /// Carries the smoke by `wrap`.
    pub(super) fn carry(&mut self, wrap: &PlayerWrap) {
        self.smoke.carry(wrap);
    }

    /// The smoke the shots have left.
    #[cfg(test)]
    pub(super) fn smoke(&self) -> &SmokePuffs {
        &self.smoke
    }

    /// Forgets every shot and frees the smoke, as the effects are cleared
    /// on landing and arriving.
    pub(super) fn clear(&mut self) {
        self.spin_starts.clear();
        self.smoke.clear();
    }
}
