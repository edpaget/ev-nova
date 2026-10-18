//! The shots point defence ends, kept hidden at life 0 for the next
//! proximity pass.
//!
//! Point defence leaves each shot it ends at a life of exactly 0, its
//! sprite hidden: the point-defence shot spent on a missile and a missile
//! it downs (`_PointDefenseCollisionHandler` @0x377c5, @0x377fa), and a
//! missile a point-defence beam downs (`_HandleBeams` @0x30ab1).
//! `_CheckShotProximities` skips only a life below 0 (@0x371ea-0x371f6),
//! so the next proximity pass still tests them:
//!
//! - its fuse is armed when the weapon's `Count` is above its
//!   `ProxSafety` (@0x371fc-0x3721e), whatever the shot's age;
//! - a planet-type shot meets a stellar as a live one does
//!   (@0x37224-0x37521), and one that hits goes no further
//!   (@0x3752d-0x37543);
//! - the ship branch refuses it (`_ShotCanHitShip` @0x447c2-0x447ce,
//!   life ≤ 0), so it meets no ship;
//! - the asteroid branch tests no life (@0x376ab-0x37758), so an asteroid
//!   in its fuse's reach sets it off (`_HandleAsteroidHit`, see
//!   `Combat::hit_asteroid`): the explosion, the particles, the player's
//!   blast, and the damage, the break or the push.
//!
//! The pass that tests them is the one in the same `test_hits` for a
//! point-defence shot's meeting, whose layer the original collides right
//! before `_CheckShotProximities` (@0x43fff-0x44015), and the next tick's
//! for a beam's kill, as `_HandleBeams` is the frame's last step
//! (@0x4453d). `_HandleShot` then frees each at its visit with no end of
//! life: a hidden sprite skips straight to the free (@0x36195 → @0x35f0a →
//! @0x3617c). Kept out of the shots in flight, a hidden shot is neither
//! drawn, nor moved, nor counted in [`Combat::targeted_damage`], nor
//! chosen by point defence; its slot stays held through
//! `Combat::shot_room`'s count of the shots ended.

use super::{Combat, Fighter, Landed, asteroid_hit, stellar};
use crate::combat::asteroid_hit::AsteroidTarget;
use crate::combat::projectile::Shot;
use crate::combat::stellar::StellarTarget;

impl Combat {
    /// Keeps `shot`, which point defence ended, hidden at life 0 for the
    /// next proximity pass ([`Combat::test_hidden`]). Life 0 is the
    /// original's (age = `Count` − life): a missile whose move had spent it
    /// (life -1) is put back to 0 too, as `_PointDefenseCollisionHandler`
    /// tests no life before it writes 0 (@0x377fa).
    pub(super) fn hide(&mut self, mut shot: Shot) {
        shot.age = shot.weapon.lifetime;
        self.hidden.push(shot);
    }

    /// The proximity pass over the hidden shots (`_CheckShotProximities`
    /// @0x371bc-0x37775, see the [module docs](self)), each tested once, in
    /// the order they are kept, among `stellars` and `asteroids`, the
    /// player's blasts reaching `fighters`: gives the blasts' hits to land.
    /// Every hidden shot is then let go, as `_HandleShot` frees it before
    /// any later pass.
    ///
    /// The original interleaves them with the live shots by slot; here
    /// they come after the live shots of the same pass, and their blasts'
    /// hits land after the live proximities. Everything settles inside
    /// the collision passes, before any frame is drawn.
    pub(super) fn test_hidden(
        &mut self,
        fighters: &[Fighter],
        stellars: &mut [StellarTarget],
        asteroids: &mut [AsteroidTarget],
    ) -> Vec<Landed> {
        let mut hits = Vec::new();
        for shot in std::mem::take(&mut self.hidden) {
            let was = shot.moved_from.unwrap_or(shot.position);
            if let Some((index, at)) = stellar::met(&shot, was, stellars) {
                self.strike_stellar(&shot, &mut stellars[index], at);
                continue;
            }
            if let Some(index) = asteroid_hit::fused(&shot, asteroids, self.asteroid_fuse) {
                let at = shot.position;
                self.hit_asteroid(&shot, &mut asteroids[index], at, fighters, &mut hits);
            }
        }
        hits
    }
}
