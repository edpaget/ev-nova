//! What the flight view keeps of the shots between steps, for drawing
//! them as the original does: each spinning shot's start
//! ([`SpinStarts`]), the weapons' trail tables ([`TrailTables`]) and the
//! puffs of smoke ([`SmokePuffs`]). The simulation keeps none of it: it
//! is the view's, drawn on the effects' chance, never the simulation's.

use nova_sim::{Chance, Session, Vec2};

use crate::DrawList;
use crate::flight::catalog::Looks;
use crate::flight::effects::Effects;
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
}

impl ShotLooks {
    /// Steps it after a pass of `session`'s fight, the shots' looks read
    /// from `looks`, drawing on `chance`, as `_HandleShot` then
    /// `_HandleSmoke` do each frame: each shot new in flight draws its
    /// spin start, and each gone is forgotten; each shot in flight
    /// leaves its trail's burst in `effects`, under the `caps` or not,
    /// and its puff of smoke; then the smoke ages.
    pub(super) fn step(
        &mut self,
        session: &Session,
        looks: &Looks,
        (effects, caps): (&mut Effects, bool),
        chance: &mut dyn Chance,
    ) {
        let flags = |id| looks.weapon(id).map_or(0, |look| look.flags);
        let shots = session
            .shots()
            .iter()
            .map(|shot| (shot.id, flags(shot.weapon.id)));
        self.spin_starts.step(shots, chance);
        for shot in session.shots() {
            let Some(look) = looks.weapon(shot.weapon.id) else {
                continue;
            };
            let at = (shot.position, shot.heading);
            if let Some(burst) = self.trails.burst(shot.weapon.id, look, at, chance) {
                effects.spray(&burst, caps, chance);
            }
            if let Some(kind) = smoke_kind(look.flags) {
                let sheet = looks.smoke_set(look.smoke_set);
                self.smoke.spawn(shot.position, kind, sheet, caps);
            }
        }
        self.smoke.step();
    }

    /// Draws the smoke where `camera` shows it, murked as seen from the
    /// player's ship at `viewer` in a system of effective murk `murk`.
    pub(super) fn draw_smoke(&self, list: &mut DrawList, camera: &Camera, viewer: Vec2, murk: u8) {
        self.smoke.draw(list, camera, viewer, murk);
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
