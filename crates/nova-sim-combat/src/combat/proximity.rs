//! The proximity pass, a port of `_CheckShotProximities`
//! (@0x371bc-0x37775), the hit pass's last test (rdm asteroids phase 9).
//!
//! # The original
//!
//! One loop over the 128 shot slots of 0x48 bytes (@0x371e0-0x37765, to
//! 0x2400 @0x3775a-0x37765). Each slot, in turn:
//!
//! 1. is skipped at a life below 0 (@0x371ea-0x371f6), then while its
//!    fuse is not armed, `Count` − `ProxSafety` ≤ its life
//!    (@0x3720e-0x3721e, [`Shot::fuse_armed`]);
//! 2. if its weapon is planet-type (`Flags2` 0x0400, @0x37224), walks the
//!    16 stellars of the system's list (+0x2a + 2i of the system's
//!    record, @0x37230-0x37527), skipping an empty entry (@0x37250) and
//!    one `_StellarIsDestroyed` reports (@0x3727b); the first its sprite
//!    overlaps (`_SWRLECollision` @0x372a8) takes `MassDmg` + `EnergyDmg`
//!    (@0x372dc-0x372eb), the shot exploding (@0x3732a) and spraying its
//!    hit particles (@0x373b4) at life -1, hidden (@0x373c4-0x373fe), and
//!    the loop ends there (@0x37521) ([`stellar::met`]);
//! 3. reads its life again and goes no further below 0 (@0x3752d-0x37543),
//!    nor without a `ProxRadius` above none (@0x3755b-0x37560);
//! 4. walks the 64 ship slots (@0x3757b-0x376a5): the first in reach that
//!    `_ShotCanHitShip` lets it hit takes `_HandleShipHit(shot, ship, 1)`
//!    (@0x3768a), and the slot ends (@0x3768f) ([`Shot::fused`]);
//! 5. goes no further under `Seeker` 0x0001 (@0x37566-0x37575,
//!    @0x376ab-0x376b0);
//! 6. walks the 16 asteroid slots (@0x376dd-0x37758): the first active
//!    one in reach takes `_HandleAsteroidHit` (@0x37744)
//!    ([`asteroid_hit::fused`]).
//!
//! A shot point defence ended (`_PointDefenseCollisionHandler` @0x377c5,
//! @0x377fa; `_HandleBeams` @0x30ab1) is at life 0, hidden: it passes the
//! first test, so it takes its turn in its slot among the live shots, and
//! `_ShotCanHitShip` refuses it every ship (@0x447c2-0x447ce). Its slot is
//! held until `_HandleShot` visits it (@0x36195 → @0x3617c), so no shot
//! released in the pass takes it ([`hidden`](super::hidden)). No sound is
//! played in the pass.
//!
//! The stellar loop goes by the system's list (`nav_def`), the order the
//! session gives the stellars (`nova_sim::Session`'s chart keeps
//! `PilotCatalog::landing_sites`' order), so the first in that order that
//! a shot's move meets is struck, not the nearest along it.
//!
//! *The engine.* A stellar is a disc and a shot's move is swept to it, as
//! the [`stellar`] module records, where the original overlaps the
//! sprites where the shot is.
//!
//! *Ordering against the original.* Each hit lands as the pass finds it,
//! a hidden shot's asteroid blast included, before the next slot is
//! tested.

use super::collision::{Among, WalkTerms};
use super::projectile::Shot;
use super::stellar::{self, StellarStrike, StellarTarget};
use super::{Combat, CombatEvent, asteroid_hit};
use crate::chance::Chance;
use crate::combat::asteroid_hit::AsteroidTarget;
use crate::geometry::Vec2;

/// Whose turn a slot is in the pass: the shot in flight at an index of
/// the shots, or the hidden one at an index of the hidden shots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Turn {
    /// A shot in flight.
    Live(usize),
    /// A shot point defence ended, hidden at life 0.
    Hidden(usize),
}

impl Combat {
    /// The hit pass's proximity pass (see the module docs), among
    /// `fighters` as `lineup` reads them: every slot held by a shot in
    /// flight or a hidden one, in ascending order, takes its turn
    /// ([`Combat::live_turn`], [`Combat::hidden_turn`]) among `stellars`
    /// and `asteroids`. Then every hidden shot is let go, as `_HandleShot`
    /// frees it before any later pass; its slot stays held until then.
    pub(super) fn proximity_pass(
        &mut self,
        (fighters, lineup): Among,
        terms: &WalkTerms,
        chance: &mut dyn Chance,
        (stellars, asteroids): (&mut [StellarTarget], &mut [AsteroidTarget]),
    ) {
        let mut next = 0;
        while let Some((slot, turn)) = self.next_turn(next) {
            next = slot + 1;
            match turn {
                Turn::Live(index) => self.live_turn(
                    index,
                    (fighters, lineup),
                    (terms, chance),
                    (stellars, asteroids),
                ),
                Turn::Hidden(index) => {
                    let shot = self.hidden[index];
                    self.hidden_turn(&shot, (fighters, lineup), terms, (stellars, asteroids));
                }
            }
        }
        self.hidden.clear();
    }

    /// The lowest slot from `next` on that a shot in flight or a hidden
    /// one holds, and whose turn it is.
    fn next_turn(&self, next: usize) -> Option<(usize, Turn)> {
        let live = self
            .shots
            .iter()
            .enumerate()
            .map(|(index, shot)| (shot.slot, Turn::Live(index)));
        let hidden = self
            .hidden
            .iter()
            .enumerate()
            .map(|(index, shot)| (shot.slot, Turn::Hidden(index)));
        live.chain(hidden)
            .filter(|&(slot, _)| slot >= next)
            .min_by_key(|&(slot, _)| slot)
    }

    /// The turn of the shot in flight at `index`. One whose move spent it
    /// is not tested (@0x371ea), and waits for its end of life in
    /// `_HandleShot` ([`Combat::handle_shots`]); nor is one that has not
    /// moved, a sub-munition released in the pass among them, whose life
    /// is still its `Count` (@0x3720e-0x3721e). Otherwise a planet-type
    /// shot meeting a stellar on its move strikes it
    /// ([`Combat::strike_stellar`]), and goes no further; or else the
    /// first ship in slot order whose reach its fuse is in
    /// ([`Shot::fused`]) takes its hit at once ([`Combat::land_hit`]), and
    /// the shot releases its sub-munitions at that ship
    /// (`_HandleShipHit` @0x36f12-0x36f47), their weapons read from
    /// `terms`' arsenal and their spread drawn on `chance`, seeking among
    /// the ships as the pass's earlier hits and its own left them; or else
    /// the first asteroid in slot order its fuse reaches takes its hit
    /// ([`Combat::land_on_asteroid`]). The shot is spent on whatever it
    /// meets, holding its slot.
    fn live_turn(
        &mut self,
        index: usize,
        (fighters, lineup): Among,
        (terms, chance): (&WalkTerms, &mut dyn Chance),
        (stellars, asteroids): (&mut [StellarTarget], &mut [AsteroidTarget]),
    ) {
        let shot = self.shots[index];
        if shot.expired() {
            return;
        }
        let Some(from) = shot.moved_from else {
            return;
        };
        if let Some((met, at)) = stellar::met(&shot, from, stellars) {
            self.spend(index);
            self.strike_stellar(&shot, &mut stellars[met], at);
        } else if let Some(met) = shot.fused(from, &lineup.sides(), self.proximity_fuse) {
            self.spend(index);
            let mut parent = shot;
            self.land_hit(
                &mut parent,
                (met.ship, met.at),
                (fighters, lineup),
                terms.rule,
            );
            let sides = lineup.sides();
            let target = (Some(met.ship), &*fighters);
            self.release(&parent, target, &sides, terms.arsenal, chance);
        } else if let Some(rock) = asteroid_hit::fused(&shot, asteroids, self.asteroid_fuse) {
            self.spend(index);
            let at = shot.position;
            self.land_on_asteroid(
                (&shot, &mut asteroids[rock], at),
                (fighters, lineup),
                terms.rule,
            );
        }
    }

    /// The turn of `shot`, hidden at life 0 (see the
    /// [`hidden`](super::hidden) module): a planet-type one meeting a
    /// stellar on its move, or where it is if it never moved, strikes it
    /// ([`Combat::strike_stellar`]), and goes no further; it meets no
    /// ship; or else the first asteroid in slot order its armed fuse
    /// reaches takes its hit where it is, the player's blast landing at
    /// once among the ships, as `terms` say ([`Combat::land_on_asteroid`]).
    fn hidden_turn(
        &mut self,
        shot: &Shot,
        among: Among,
        terms: &WalkTerms,
        (stellars, asteroids): (&mut [StellarTarget], &mut [AsteroidTarget]),
    ) {
        let was = shot.moved_from.unwrap_or(shot.position);
        if let Some((met, at)) = stellar::met(shot, was, stellars) {
            self.strike_stellar(shot, &mut stellars[met], at);
        } else if let Some(rock) = asteroid_hit::fused(shot, asteroids, self.asteroid_fuse) {
            let at = shot.position;
            self.land_on_asteroid((shot, &mut asteroids[rock], at), among, terms.rule);
        }
    }

    /// `shot`'s hit on `target` at `at`, which spends it (@0x37230-0x37521):
    /// it explodes there with its hit particles, with no blast and no
    /// sub-munitions, and takes its damage off the stellar's strength,
    /// raising a [`StellarStrike`].
    fn strike_stellar(&mut self, shot: &Shot, target: &mut StellarTarget, at: Vec2) {
        if let Some(explosion) = shot.weapon.explosion {
            self.events.push(CombatEvent::Exploded {
                at,
                explosion,
                size: shot.weapon.blast_radius,
            });
        }
        self.spray(&shot.weapon, at, asteroid_hit::HIT_PART_VARIATION);
        let damage = stellar::damage(&shot.weapon);
        target.strength -= damage;
        self.stellar_strikes.push(StellarStrike {
            stellar: target.stellar,
            by: shot.attacker(),
            damage,
            destroyed: stellar::shot_down(target.strength),
        });
    }
}
