//! The player's wrap in the fight: `_WrapPlayerPosition`'s loops of
//! `_shotData` (@0x68010-0x68091) and `_beamData` (@0x682b7-0x6833c), and
//! the slots' kept positions, and how the contact walks see its pass.
//!
//! - **Shots** ([`Combat::carry`]). Each within reach of the player
//!   ([`PlayerWrap::carries`]) moves, its last move with it
//!   ([`Shot::moved_from`]), velocity kept; the shots point defence ended,
//!   still in their slots ([`hidden`](super::hidden)), likewise.
//! - **Beams**. All 40 slots move, with no test (@0x682b7-0x6833c): each
//!   beam's start and end, whatever it is near, floats where the original
//!   truncates them into shorts.
//! - **The slots' kept positions** ([`source_slot`](super::source_slot)).
//!   Each moves by the test, an empty slot's as a live one's, as the
//!   beams' asteroid pass reads it ([`asteroid_beam`](super::asteroid_beam)).
//! - **Where the player stood** (`Combat::player_stood`), always: it stands
//!   for the player's own sprite, at the screen's centre.
//!
//! # The wrap's pass
//!
//! `_SWCollideSpriteLayer` (@0x43e70-0x44015) reads sprite rects in screen
//! terms as the last pass placed them, so on the wrap's pass nothing has
//! moved relative to anything, though the routine moved some records and
//! left others. So for that pass's contact walks alone ([`WrapPass`]) an
//! unmoved asteroid or ship is met at its position plus the delta, where
//! its sprite stood, and a shot left behind, which stayed in the world but
//! not on screen, meets everything a delta back, so a hit it lands stays
//! where the shot is. The walks' places are the only ones that shift: a
//! blast, a fuse and a stellar read world positions in the original too
//! (`_CheckShotProximities`, `stellar::met`), so they read them here. The
//! pass ends the shift (`Combat::hit_pass`).

use super::Combat;
use super::ShipRef;
use super::projectile::{Shot, ShotId};
use crate::geometry::Vec2;
use crate::player_wrap::PlayerWrap;

/// What the contact walks of a wrap's pass read of it (see the module
/// docs).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct WrapPass {
    /// What the wrap added.
    delta: Vec2,
    /// The ships it left behind.
    ships: Vec<ShipRef>,
    /// The shots it left behind.
    shots: Vec<ShotId>,
}

impl Combat {
    /// Carries the shots, the beams, the slots' kept positions and where
    /// the player stood by `wrap`, whose ships left behind are
    /// `ships_left` (see the module docs).
    pub fn carry(&mut self, wrap: &PlayerWrap, ships_left: Vec<ShipRef>) {
        let mut shots_left = Vec::new();
        for shot in &mut self.shots {
            if !carry_shot(wrap, shot) {
                shots_left.push(shot.id);
            }
        }
        for shot in &mut self.hidden {
            carry_shot(wrap, shot);
        }
        for beam in &mut self.beams {
            beam.start = wrap.carried(beam.start);
            beam.end = wrap.carried(beam.end);
        }
        self.player_stood = self.player_stood.map(|at| wrap.carried(at));
        for ship in self.slot_ships.values_mut() {
            wrap.carry(&mut ship.position);
        }
        self.wrap_pass = Some(WrapPass {
            delta: wrap.delta,
            ships: ships_left,
            shots: shots_left,
        });
    }

    /// Where the contact walks meet `ship`, standing at `at`.
    pub(super) fn ship_sprite(&self, ship: ShipRef, at: Vec2) -> Vec2 {
        match &self.wrap_pass {
            Some(pass) if pass.ships.contains(&ship) => at + pass.delta,
            _ => at,
        }
    }

    /// Where the contact walks meet the asteroid standing at `at`.
    pub(super) fn rock_sprite(&self, at: Vec2) -> Vec2 {
        self.wrap_pass.as_ref().map_or(at, |pass| at + pass.delta)
    }

    /// How far back `shot` meets what the walks see (see the module docs):
    /// nothing but on a wrap's pass, for a shot it left behind.
    pub(super) fn left_back(&self, shot: &Shot) -> Vec2 {
        match &self.wrap_pass {
            Some(pass) if pass.shots.contains(&shot.id) => pass.delta,
            _ => Vec2::ZERO,
        }
    }

    /// `shot` as point defence's walk meets it: at its sprite.
    pub(super) fn shot_sprite(&self, shot: &Shot) -> Shot {
        Shot {
            position: shot.position + self.left_back(shot),
            ..*shot
        }
    }
}

/// Carries `shot` and its last move if `wrap` reaches it. Whether it did.
fn carry_shot(wrap: &PlayerWrap, shot: &mut Shot) -> bool {
    let moves = wrap.carry(&mut shot.position);
    if moves {
        shot.moved_from = shot.moved_from.map(|at| wrap.carried(at));
    }
    moves
}
