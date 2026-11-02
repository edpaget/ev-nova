//! The walks of the fight's phases that stop after each direct hit, kept
//! on the [`Combat`] between their stretches (rdm play-feel-9 phase 10,
//! see [`hit_turns`](super::hit_turns)): `_HandleBeams`' beam loop
//! ([`BeamWalk`]) and the collision passes ([`HitWalk`]).
//!
//! A walk opens at its phase's first stretch and is taken up again at
//! each later one, among the fighters lent afresh. What it keeps across a
//! stop is what the original's loop reads once as it begins, or carries
//! in its own locals:
//!
//! - **The beams**: the next beam to test, how many beams the tick held as
//!   it opened (for [`Combat::beam_room`]'s count of ended beams). The
//!   opening (the beams' moves and stamps, and point defence's beams on
//!   their missiles) runs once.
//! - **The collision passes**: the pass's shots, taken out of flight for
//!   the contact passes, which of them are spent and the order each layer
//!   keeps them in; the asteroids shown as the passes began ([`Shown`]);
//!   the ships' walk order, by [`ShipRef`], as their sprites stood when
//!   the passes began ([`walk_order`], the original's layers, which no
//!   answer reorders); where contact meets each ship; where it stopped;
//!   and what of the hit it stopped after is still to come: its blast
//!   and, in the proximity pass, its sub-munitions ([`Pending`]).
//!
//! On resume a walk reads its ships afresh from the fighters
//! ([`Lineup::new`]) and finds each by [`ShipRef`], never by its place
//! among the fighters. Each stop settles the ships its stretch struck
//! before it stops (`Combat::settle_struck`), so a ship read afresh reads
//! as the loop's own refresh reads a struck one
//! ([`reread`](super::lineup::reread)): what `_IsDisabled` makes of its
//! armour is settled into its condition, and the rest of its record is
//! read whole either way. So the walk keeps no list of the ships struck.
//! No index into the fight's
//! strikes is kept across a stop: the keeper's answer takes them
//! ([`Combat::take_landed`]), so each stretch settles from its own first
//! strike.
//!
//! *Ordering against the original.* Between two stretches of the
//! collision passes the shots the contact passes still walk are held here,
//! out of [`Combat::shots`], so an answer's
//! [`Combat::targeted_damage`] misses them. Only the AI's fire reads it,
//! after the passes (`_CalculateTargetedDamage` @0x44223), so nothing
//! differs.

use super::asteroid_walk::{self, AsteroidField, Shown};
use super::collision::{WalkTerms, walk_order};
use super::hit_turns::Stretch;
use super::hull::DisableRule;
use super::lineup::Lineup;
use super::projectile::Shot;
use super::{Arsenal, Combat, Fighter, Landed, Scenery, ShipRef};
use crate::asteroid::Asteroids;
use crate::chance::Chance;
use crate::geometry::Vec2;
use crate::govt::Governments;

/// A phase's walk stopped after a direct hit.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Walk {
    /// `_HandleBeams`' beam loop.
    Beams(BeamWalk),
    /// The collision passes.
    Hits(Box<HitWalk>),
}

/// `_HandleBeams`' beam loop under way (`Combat::close_tick_in_turn`).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct BeamWalk {
    /// The next beam to test.
    pub(super) next: usize,
    /// How many beams the tick held as the walk opened.
    pub(super) beams_held: usize,
}

impl BeamWalk {
    /// A walk opening on `beams_held` beams.
    pub(super) fn open(beams_held: usize) -> Self {
        Self {
            next: 0,
            beams_held,
        }
    }
}

/// The collision passes under way (`Combat::test_hits_in_turn`).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct HitWalk {
    /// The pass's shots, out of flight for the contact passes: empty once
    /// they are kept for the proximity pass.
    pub(super) shots: Vec<Shot>,
    /// Which of `shots` are spent.
    pub(super) spent: Vec<bool>,
    /// The indices of `shots` in the order each layer keeps them
    /// ([`Combat::entry_order`]).
    pub(super) entry: Vec<usize>,
    /// The asteroids shown as the passes began, less those destroyed.
    pub(super) shown: Shown,
    /// The ships the contact passes walk, in their walk order.
    pub(super) order: Vec<ShipRef>,
    /// Where contact meets each ship, as the passes began.
    pub(super) places: Vec<(ShipRef, Vec2)>,
    /// Where the walk is.
    pub(super) stage: Stage,
    /// What is still to come of the hit it stopped after.
    pub(super) pending: Option<Pending>,
}

/// Where the collision passes' walk is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stage {
    /// The contact passes: the shot layer, the ship of the walk order and
    /// the shot of the entry order to test next. Past the last layer, the
    /// asteroids against point defence, point defence's layer and the
    /// proximity pass are to come.
    Contact {
        /// The index into the shot layers' walk.
        layer: usize,
        /// The index into the walk order.
        ship: usize,
        /// The index into the entry order.
        entry: usize,
    },
    /// The proximity pass, at the lowest slot from `next` on.
    Proximity {
        /// The slot to take next.
        next: usize,
    },
}

/// What is still to come of the direct hit a walk stopped after, as
/// `_HandleShipHit` runs it after the direct hit's `_DamageShip`
/// (@0x36c7a): its blast's hits (@0x36ec0), then a proximity hit's
/// sub-munitions (@0x36f47).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Pending {
    /// The blast's hits, worked out at the direct hit.
    pub(super) blast: Vec<Landed>,
    /// The shot that hit, where it hit, and the ship it hit, whose
    /// sub-munitions it releases at that ship: a proximity hit's.
    pub(super) release: Option<(Shot, ShipRef)>,
}

impl Combat {
    /// Steps 2 and 3, the hit pass, as far as its next stop: each shot's
    /// last move, made the tick before (its [`Shot::moved_from`]), is
    /// tested as the original's next frame tests it, before `_HandleShot`
    /// moves it again (`_DoPlayGameWork` @0x43e70-0x44015, @0x44385). A
    /// shot that has not moved yet is not tested, as its sprite is not
    /// shown. The tests go in the original's order
    /// ([`collision`](super::collision)):
    ///
    /// 1. the contact passes, each shot layer against the ships, walked by
    ///    layer and entry, then the asteroids, walked by slot, against the
    ///    layer; then the asteroids against point defence's layer;
    /// 2. point defence's layer against the missiles
    ///    ([`Combat::intercept`]): a shot spent in 1 is hidden by then and
    ///    never met, and each it ends is kept hidden at life 0
    ///    ([`hidden`](super::hidden));
    /// 3. the proximity pass ([`Combat::proximity_pass`]), slot by slot,
    ///    the shots point defence ended, this pass's and the last tick's
    ///    beam kills, among the live ones: each slot's stellar, then, live,
    ///    its fuse's ship, then its fuse's asteroid.
    ///
    /// All three run in [`Combat::resolve`]. A shot whose move spent it
    /// meets only the asteroids it touches and point defence, and is kept
    /// for its end of life in `_HandleShot` ([`Combat::handle_shots`]).
    /// Point defence's [`CombatEvent::ShotDown`](super::CombatEvent::ShotDown)
    /// follows the contact passes' events.
    ///
    /// The pass meets each NPC where it is, as its last `_HandleShip` left
    /// it (@0x3428f-0x342aa), and the player where it is now, as
    /// `_HandlePlayer` has moved it. Contact and the point-defence layer
    /// meet the player where it stood as the last tick closed, or where it
    /// is if it was not in that tick's fight: their sprites are placed
    /// relative to the player as it stood when the shots moved
    /// (`_HandleShot` @0x35c48-0x35cc0), and the player's own is always
    /// at the screen's centre (@0x2cd32-0x2cd9d). The fuse, the blasts
    /// and the hits' pushes and ionization read world positions, the
    /// player's where it is now. Each sub-munition's weapon is read from
    /// `arsenal`, its spread drawn on `chance`; the relations between
    /// governments are `govts`'. On the pass of a wrap of the player, the
    /// contact walks meet what it left where its sprite stood, and the
    /// pass ends that ([`wrap_carry`](super::wrap_carry)).
    ///
    /// Each hit lands as the walk finds it ([`Combat::land`]), as
    /// `_HandleShipHit` lands it before the walk's next pair, so a later
    /// test reads the armour it left
    /// ([`Target::disabled`](super::Target::disabled)). **It stops** just
    /// after each direct hit of the contact walk and of the proximity
    /// pass ([`Stretch::Answer`]), before its blast and its sub-munitions,
    /// its walk kept to resume ([`HitWalk`]); the next stretch, among the
    /// ships lent afresh, lands what is still to come of that hit
    /// ([`Pending`]) and walks on. So each later hit reads the ships as the
    /// answers of the hits before it left them (see the
    /// [`hit_turns`](super::hit_turns) module).
    ///
    /// At each stop, and as the walk ends, each ship struck in the
    /// stretch settles its condition as step 9 does but for a breakup's
    /// countdown, its last strike in the stretch marked as downing it, as
    /// `rule` says, as the original settles each in `_DamageShip`: so an
    /// NPC downed by a shot fires nothing more that tick, as
    /// `_HandleShip`'s fire reads `_IsDisabled` after the passes, and the
    /// answers read its condition settled. It keeps the ships whose
    /// breakup it began, whose first tick takes no tick off (step 9, see
    /// `Combat::settle_struck`). Nothing in the pass reads the condition
    /// but whether a ship is destroyed, which no hit there changes, so a
    /// blast's hit settling at the next stop crosses no boundary.
    pub(super) fn hit_stretch(
        &mut self,
        fighters: &mut [Fighter],
        (govts, arsenal): (&Governments, &Arsenal),
        rule: &dyn DisableRule,
        scenery: Scenery<'_>,
        chance: &mut dyn Chance,
    ) -> Stretch {
        let Scenery {
            stellars,
            asteroids,
            hazards: _,
        } = scenery;
        let mut none = Asteroids::default();
        let mut field = asteroids.unwrap_or(AsteroidField {
            asteroids: &mut none,
            types: &asteroid_walk::NO_TYPES,
        });
        let mut walk = match self.walk.take() {
            Some(Walk::Hits(walk)) => walk,
            other => {
                debug_assert!(other.is_none(), "a phase began amid {other:?}");
                Box::new(self.open_hits(fighters, field.asteroids))
            }
        };
        let first_strike = self.strikes.len();
        let places: Vec<Vec2> = fighters
            .iter()
            .map(|fighter| {
                walk.places
                    .iter()
                    .find(|&&(ship, _)| ship == fighter.ship)
                    .map_or_else(|| self.touchable_at(fighter), |&(_, at)| at)
            })
            .collect();
        let mut lineup = Lineup::new(fighters, &places, govts, self.hit_readings());
        let terms = WalkTerms { arsenal, rule };
        if let Some(pending) = walk.pending.take() {
            self.land_pending(pending, (fighters, &mut lineup), &terms, chance);
        }
        let stopped = self.resolve(
            &mut walk,
            (fighters, &mut lineup),
            &terms,
            chance,
            (stellars, &mut field),
        );
        self.settle_struck(fighters, first_strike, rule);
        if stopped {
            self.walk = Some(Walk::Hits(walk));
            return Stretch::Answer;
        }
        self.wrap_pass = None;
        Stretch::Done
    }

    /// The collision passes' walk as it opens among `fighters`, the
    /// asteroids of `asteroids` shown: the shots taken out of flight in
    /// the order each layer keeps them, the ships in their walk order, and
    /// where contact meets each.
    fn open_hits(&mut self, fighters: &[Fighter], asteroids: &Asteroids) -> HitWalk {
        let places = fighters
            .iter()
            .map(|fighter| (fighter.ship, self.touchable_at(fighter)))
            .collect();
        let entry = self.entry_order();
        let shots = std::mem::take(&mut self.shots);
        let ranks: Vec<_> = fighters.iter().map(|fighter| fighter.sprite).collect();
        let order = walk_order(&ranks)
            .into_iter()
            .map(|index| fighters[index].ship)
            .collect();
        HitWalk {
            spent: vec![false; shots.len()],
            shots,
            entry,
            shown: Shown::of(asteroids),
            order,
            places,
            stage: Stage::Contact {
                layer: 0,
                ship: 0,
                entry: 0,
            },
            pending: None,
        }
    }

    /// Where contact and point defence's layer meet `fighter`: the player
    /// where it stood as the last tick closed, if it was in that tick's
    /// fight, and every ship at its sprite on a wrap's pass
    /// ([`wrap_carry`](super::wrap_carry)).
    fn touchable_at(&self, fighter: &Fighter) -> Vec2 {
        match self.player_stood {
            Some(position) if fighter.ship == ShipRef::Player => position,
            _ => self.ship_sprite(fighter.ship, fighter.state.position),
        }
    }

    /// Lands what is still to come of the direct hit a walk stopped after,
    /// among `fighters` as `lineup` reads them: its blast's hits, then a
    /// proximity hit's sub-munitions, released at the ship it hit as the
    /// answer left its owner's plunder (`_SpawnShotSubmunitions` @0x36f47,
    /// after the direct hit's `_DamageShip` @0x36c7a), their weapons read
    /// from `terms`' arsenal and their spread drawn on `chance`.
    fn land_pending(
        &mut self,
        pending: Pending,
        (fighters, lineup): (&mut [Fighter], &mut Lineup),
        terms: &WalkTerms,
        chance: &mut dyn Chance,
    ) {
        self.land_blast(pending.blast, (fighters, lineup), terms.rule);
        if let Some((parent, ship)) = pending.release {
            let sides = lineup.sides();
            self.release(
                &parent,
                (Some(ship), &*fighters),
                &sides,
                terms.arsenal,
                chance,
            );
        }
    }
}
