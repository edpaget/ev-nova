//! The stellar hazards inside a fight's tick: gravity's pull
//! (`_HandleGravity` @0x39bc3-0x39ccc) and the deadly stellars' kills
//! (`_HandleDeadlyStellars` @0x39ccd-0x39e47), which `_DoPlayGameWork`
//! calls after the shots and the ships (@0x44509, @0x4450e) and before it
//! jumps to `_HandleBeams` (@0x4453d). So both land between step 6's shots
//! and the beams ([`Combat::tick_on`]), on either pass:
//!
//! - **The pull** ([`pull`]) is added to the velocity of each fighter not
//!   destroyed that does not resist gravity ([`HazardResistance::gravity`]),
//!   source by source in the system's slot order, and reported as a
//!   [`Shove`]. A later beam tick's knockback starts from the pulled
//!   velocity, so `_SpeedLimit`'s clamp covers the pull, as the original's
//!   does.
//! - **A kill**, disc by disc in slot order, fighters in their order (the
//!   player first): each fighter neither destroyed nor breaking up, that
//!   does not resist ([`HazardResistance::deadly`]) and touches the disc
//!   ([`touches`]), goes destroyed with its armour at [`KILLED_ARMOR`],
//!   and its `Explode2` goes off at size 0 ([`Combat::explode`]; none when
//!   it has none). It is kept as a [`StellarKill`]
//!   ([`Combat::take_stellar_kills`]) for the session to settle. A ship
//!   killed by one disc is out for the later ones. The kill makes no
//!   breakup, death timer or shockwave, and the beams that follow neither
//!   follow nor hit the ship.
//!
//! Which stellars pull and which discs kill, and what a kill does to the
//! ship beyond this, are the session's (`nova_sim::Session`'s gravity and
//! deadly-stellar steps). The discs stand in for the original's sprite
//! masks (`_SWRLECollision` @0x39d97; see [`deadly_stellar`]).

use super::{Combat, Fighter, ShipRef, Shove};
use crate::combat::hull::Condition;
#[cfg(doc)]
use crate::deadly_stellar;
use crate::deadly_stellar::{KILLED_ARMOR, touches};
use crate::geometry::Vec2;
use crate::gravity::{SPEED_MULT, pull};

/// A stellar that pulls: where it is and its `spöb` `Gravity`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GravitySource {
    /// Where it is.
    pub at: Vec2,
    /// Its `Gravity`: below none, it pushes.
    pub gravity: i16,
}

/// The disc a deadly stellar's sprite kills within: its centre and
/// radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeadlyDisc {
    /// Its centre.
    pub at: Vec2,
    /// Its radius.
    pub radius: f32,
}

/// The stellar hazards of the player's system for a tick, each in the
/// system's slot order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StellarHazards {
    /// The stellars that pull.
    pub gravity: Vec<GravitySource>,
    /// The deadly stellars' discs.
    pub deadly: Vec<DeadlyDisc>,
}

/// Which stellar hazards a fighter resists (`_ShipResistsGravity`,
/// `_ShipResistsDeadlyStellars`), as its session works it out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HazardResistance {
    /// Whether gravity leaves it alone.
    pub gravity: bool,
    /// Whether a deadly stellar leaves it alone.
    pub deadly: bool,
}

/// A ship a deadly stellar killed this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StellarKill {
    /// The ship killed.
    pub ship: ShipRef,
    /// Its condition just before the kill: what the tick's hits left it,
    /// which the session's lines for the player's hits read.
    pub was: Condition,
}

impl Combat {
    /// The pull, then the kills, of `hazards` on `fighters` (see the module
    /// docs).
    pub(super) fn stellar_hazards(&mut self, fighters: &mut [Fighter], hazards: &StellarHazards) {
        for source in &hazards.gravity {
            for fighter in fighters.iter_mut() {
                if *fighter.condition == Condition::Destroyed || fighter.resists.gravity {
                    continue;
                }
                let pulled = pull(
                    fighter.state.position,
                    source.at,
                    source.gravity,
                    SPEED_MULT,
                );
                let velocity = fighter.state.velocity + pulled;
                fighter.state.velocity = velocity;
                self.shoves.push(Shove {
                    ship: fighter.ship,
                    velocity,
                });
            }
        }
        for disc in &hazards.deadly {
            for fighter in fighters.iter_mut() {
                let standing = !matches!(
                    *fighter.condition,
                    Condition::Destroyed | Condition::Dying { .. }
                );
                let touching = touches(
                    fighter.state.position,
                    fighter.hull.hit_radius,
                    disc.at,
                    Some(disc.radius),
                );
                if standing && !fighter.resists.deadly && touching {
                    self.kill_by_stellar(fighter);
                }
            }
        }
    }

    /// `fighter`, touching a deadly stellar, is destroyed with its
    /// `Explode2` (see the module docs).
    fn kill_by_stellar(&mut self, fighter: &mut Fighter) {
        let was = *fighter.condition;
        *fighter.condition = Condition::Destroyed;
        fighter.reserves.armor.now = KILLED_ARMOR;
        if let Some(explosion) = fighter.hull.explosion {
            self.explode(fighter.state.position, explosion);
        }
        self.stellar_kills.push(StellarKill {
            ship: fighter.ship,
            was,
        });
    }

    /// The ships deadly stellars killed since they were last taken, in
    /// order; taking them empties the list.
    pub fn take_stellar_kills(&mut self) -> Vec<StellarKill> {
        std::mem::take(&mut self.stellar_kills)
    }
}
