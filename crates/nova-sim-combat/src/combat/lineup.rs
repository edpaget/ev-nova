//! The fight's ships as a loop of landings reads them, each struck ship
//! read again after its landing (rdm weapons-fidelity phase 47).
//!
//! # The original
//!
//! The collision passes (`_HandleShipHit` in the contact walk and in
//! `_CheckShotProximities`) and `_HandleShot`'s visits (a blast @0x36106,
//! a lock @0x35855) each land `_DamageShip` on the live ship record, and
//! every later test of the same loop reads that record: its armour
//! (`_IsDisabled`), its leader and fleet as `escort_loss` leaves them
//! (@0x3afdd), its velocity as the push leaves it, and the rest. A ship's
//! place does not change within a loop: a push writes only its velocity,
//! which moves it the next frame.
//!
//! Here each loop reads the ships through one [`Lineup`], whose
//! [`Lineup::refresh`] reads each struck ship again whole ([`reread`]),
//! keeping it at the place each view meets it. A beam's loop
//! (`_HandleBeams` @0x316a1, `Combat::close_tick`) reads its one view the
//! same way.

use super::hull::DisableRule;
use super::planet_type::BeamPlanetFlag;
use super::projectile::{Sight, Target};
use super::sides::{PassThrough, Sides};
use super::{Fighter, ShipRef};
use crate::geometry::Vec2;
use crate::govt::Governments;

/// The fight's ships as a loop of landings reads them, one entry for each
/// fighter, in the fighters' order, in two views: where the blasts, the
/// fuses and the sub-munitions meet them (`world`: every ship where it
/// is), and where the loop's other tests meet them (`seen`: contact meets
/// the player where it stood as the last tick closed, `Combat::hit_pass`;
/// `_HandleShot`'s moves and locks meet every ship where it is,
/// `Combat::handle_shots`). Each ship a landing strikes is read again
/// after it ([`Lineup::refresh`]).
pub(super) struct Lineup<'a> {
    /// Each ship where the blasts and fuses meet it: where it is.
    pub(super) world: Vec<Target>,
    /// Each ship where the loop's other tests meet it.
    pub(super) seen: Vec<Target>,
    /// Each ship's jamming and sight, which a homing shot reads.
    pub(super) sights: Vec<Sight>,
    /// The governments' relations.
    pub(super) govts: &'a Governments,
    /// Who counts as the player for the pass-through flags.
    pub(super) pass_through: PassThrough,
    /// Which of its weapon's bits makes a beam planet-type.
    pub(super) beam_planet_flag: BeamPlanetFlag,
}

impl<'a> Lineup<'a> {
    /// The ships of `fighters`, each where it is for the blasts, and seen
    /// at its place among `seen_at`, by `govts`' relations,
    /// `pass_through`'s player and `beam_planet_flag`'s beam bit.
    pub(super) fn new(
        fighters: &[Fighter],
        seen_at: &[Vec2],
        govts: &'a Governments,
        (pass_through, beam_planet_flag): (PassThrough, BeamPlanetFlag),
    ) -> Self {
        let world: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        let seen = world
            .iter()
            .zip(seen_at)
            .map(|(target, &position)| Target {
                position,
                ..*target
            })
            .collect();
        Self {
            world,
            seen,
            sights: fighters.iter().map(Fighter::sight).collect(),
            govts,
            pass_through,
            beam_planet_flag,
        }
    }

    /// The hit rules among the ships where they are.
    pub(super) fn sides(&self) -> Sides<'_> {
        Sides {
            targets: &self.world,
            govts: self.govts,
            pass_through: self.pass_through,
            beam_planet_flag: self.beam_planet_flag,
        }
    }

    /// The hit rules among the ships where they are seen.
    pub(super) fn seen_sides(&self) -> Sides<'_> {
        Sides {
            targets: &self.seen,
            ..self.sides()
        }
    }

    /// Reads each ship among `struck` again from its fighter among
    /// `fighters`, as `rule` says, in each view at its place there
    /// ([`reread`]), and its sight. No other ship's record changes in a
    /// loop but by a landing on it.
    pub(super) fn refresh(
        &mut self,
        fighters: &[Fighter],
        struck: &[ShipRef],
        rule: &dyn DisableRule,
    ) {
        for (index, fighter) in fighters.iter().enumerate() {
            if struck.contains(&fighter.ship) {
                reread(fighter, &mut self.world[index], rule);
                reread(fighter, &mut self.seen[index], rule);
                self.sights[index] = fighter.sight();
            }
        }
    }
}

/// Reads `target` again whole from `fighter`, as `rule` says, keeping it
/// at its place: everything a landing may have changed (its velocity,
/// its leader and fleet, `escort_loss`), and whether it is disabled now
/// ([`Fighter::disabled_now`]), which reads the armour the landings left
/// before its condition settles.
pub(super) fn reread(fighter: &Fighter, target: &mut Target, rule: &dyn DisableRule) {
    *target = Target {
        position: target.position,
        disabled: fighter.disabled_now(rule),
        ..fighter.as_target()
    };
}
