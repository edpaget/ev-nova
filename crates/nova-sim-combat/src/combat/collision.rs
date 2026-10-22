//! The collision passes, as `_DoPlayGameWork` walks them, each hit landed
//! as the walk finds it (rdm weapons-fidelity phase 33).
//!
//! # The original
//!
//! **The walk.** `_SWCollideSpriteLayer(world, outer, inner)` (@0xacb4b)
//! walks its outer layer from the head (+0x8, @0xacb84) by each sprite's
//! next link (+0x4, @0xacba7), and for each outer sprite the inner layer
//! the same way (@0xacba0-0xacbbc). It skips a sprite that is not visible
//! (+0x14, @0xacba3, @0xacbc6) or is flagged +0xd8 (@0xacbaf, @0xacbcc),
//! and an inner sprite that is the outer one (@0xacbbf). Where the two
//! bounding rects overlap (@0xacc30-0xacc81) it calls the outer sprite's
//! proc (+0x94, @0xacd55), and goes on with the inner walk while the outer
//! sprite is still in a layer (@0xacd69), so one ship meets every shot
//! that touches it in one walk. `_SWAddSprite` appends a sprite at a
//! layer's tail (+0xc, @0xac620-0xac639), so both walks go in entry order,
//! the order a layer is drawn in, bottom first ([`SpriteRank`]). Its +0xa0
//! restart (@0xacbea-0xacc18) is `SpriteWorld` bookkeeping and is not
//! modelled.
//!
//! **The passes** (`_DoPlayGameWork` @0x43e46-0x44015), each a
//! `_SWCollideSpriteLayer(world, outer, inner)`:
//!
//! 1. ships × boxes while `*0x3b554c` is set: scooping
//!    (`nova_sim_ai`'s `scoop`), not here;
//! 2. the ship layers 0x3b541c, 0x3b52a8, 0x3b53a0, then the asteroid
//!    layer 0x3b53b4, each × the shot layer 0x3b52f0 (@0x43e75-0x43eec);
//! 3. the same four × the homing layer 0x3b531c (@0x43ef1-0x43f62);
//! 4. the same four × the turret-over-ships layer 0x3b5308
//!    (@0x43f67-0x43fd8);
//! 5. the asteroids × point defence's 0x3b53bc (@0x43fdd-0x43ffa);
//! 6. point defence's layer × the homing layer (@0x43fff-0x44010,
//!    [`Combat::intercept`]);
//! 7. `_CheckShotProximities` (@0x44015).
//!
//! So within each pair the walk is ship-major: each ship, in its layer's
//! entry order, meets every shot of the shot layer, in that layer's entry
//! order ([`SHOT_LAYER_WALK`], [`SHIP_LAYER_WALK`]). The ships' proc is
//! `_ShipCollisionHandler` (set @0x1f203, @0x1f320); the asteroid
//! layer's is `_AsteroidCollisionHandler` (@0x20063).
//!
//! **A ship's layer.** Only `_HandleShipDisplay` puts a ship's sprite in
//! a layer (`_PutSpriteInLayer` @0x2bd58-0x2bdad): the player's runs just
//! before the passes (@0x43e29), each NPC's after its own `_HandleShip`
//! the frame before (@0x444ad). So a ship an earlier hit disables
//! mid-walk keeps its layer for the rest of the passes: its rank is read
//! once, as the pass begins ([`Fighter::sprite`]). A hidden ship's base
//! sprite is hidden (@0x2b7f3 → @0x2cc6b), so the walk skips it, but
//! `_CheckShotProximities` tests only the slot (+0xb8, @0x37585), so it
//! still sets off a fuse. A cloaked ship's base sprite stays visible
//! (@0x2d1be-0x2d1cd) and is walked.
//!
//! **A contact** (`_ShipCollisionHandler` @0x36f87-0x371b1) acts on a shot
//! of the three shot layers (@0x36fb8-0x36fd6): `_ShotCanHitShip` first
//! (@0x37002), then the shape test (@0x3700f-0x3704a), for which the
//! engine keeps its swept hit radius ([`Shot::touches`]), then
//! `ProxSafety` (@0x37052-0x37086), then `_HandleShipHit(shot, ship, 0)`
//! (@0x3708c-0x370a6). No sound.
//!
//! **A hit lands in place.** `_HandleShipHit` (@0x36a45-0x36f86) sets off
//! the explosion (@0x36aa9), takes the decay off (@0x36add-0x36afd),
//! ionizes (@0x36b27), sprays the hit particles (@0x36bae), damages the
//! ship (@0x36bc5-0x36c7a), and blasts the slots around (@0x36d4b-0x36f05)
//! before the walk tests its next pair; then it spends the shot (life -1
//! @0x36f4f) and hides its sprite (@0x36f71). So the next pair's
//! `_ShotCanHitShip` reads the armour every hit before it left: a turret
//! shot's miss rule asks `_IsDisabled` of the ship as it is now
//! (@0x448ce-0x448d5,
//! [`Target::disabled`](super::projectile::Target::disabled)). Here each
//! hit lands through [`Combat::land`] as the walk finds it
//! ([`Combat::land_hit`]), and each ship struck is read again after it
//! ([`Lineup::refresh`]).
//!
//! **The proximity pass** (`_CheckShotProximities` @0x371bc-0x37775)
//! walks the shots by slot (@0x371e0-0x37765); each takes the first ship
//! in slot order that it can hit within reach and lands at once
//! (`_HandleShipHit(shot, ship, 1)` @0x3768a).
//!
//! **The shots' order** (rdm weapons-fidelity phase 34). Each shot layer's
//! entry order is its slots' ([`shot_slots`](super::shot_slots)): a shot
//! slot reused while its sprite is still in the layer keeps its old place
//! (`_PutSpriteInLayer` @0x768c), so the walk can meet a later shot first
//! ([`Combat::entry_order`], by
//! [`RuleKey::ShotOverlap`](crate::RuleKey::ShotOverlap)'s engine
//! reading; firing order by the other). The proximity pass goes by slot,
//! the order the shots are kept in, by either reading.
//!
//! # Left out, each with its owner
//!
//! - **The asteroid layer's order.** The asteroids are the outer walk, so
//!   a shot touching two meets the first in their entry order; the engine
//!   takes the first along the move:
//!   rdm:phase/asteroids/phase-9-asteroid-contact-before-point-defence.
//!
//! **Built in phase 42.** An escort's leader is cleared as the hit that
//! leaves it disabled lands (`_DamageShip` @0x3ae58-0x3afdd, the
//! `escort_loss` module), and [`Lineup::refresh`] reads it again, so the
//! rest of the walk reads it unled. A Derelict
//! government's ships are held disabled at any armour (`_IsDisabled`
//! @0x2cf1-0x2d14): the session sets [`Fighter::held_disabled`]. A slot
//! made active since its last display is hidden, so the walk skips it
//! (@0xacba3): by `RuleKey::UndisplayedShips`' engine reading the session
//! gives such a ship no rank ([`Fighter::sprite`] none).
//!
//! **Built in phase 47.** The sub-munitions of a proximity hit are
//! released in place, right after its hit and blast land, as
//! `_HandleShipHit(shot, ship, 1)` releases them (@0x36f12-0x36f47) before
//! the pass moves on ([`Combat::proximity_pass`]). Each seeks once seated,
//! with its slot's leftover miss roll and its interference jam (see
//! [`submunition`](super::submunition)), and reads the ships as the pass's
//! earlier hits and its own left them, an escort's cleared leader
//! included, never a later one. Every shot of the pass still holds its
//! slot then, so the sub-munitions take the slots the original gives
//! them. A sub-munition released in the pass is not fused in it, as its
//! life is still its `Count` (@0x3720e-0x3721e), nor tested in the frame
//! it spawns (its sprite is not shown).
//!
//! *Ordering against the original.* The struck ships' conditions settle
//! after the pass (`Combat::settle_struck`), as nothing in the pass reads
//! them but whether a ship is destroyed, which no hit there changes.

use super::asteroid_hit::{self, AsteroidTarget};
use super::damage::Hit;
use super::hull::DisableRule;
use super::ion::Dose;
use super::lineup::Lineup;
use super::planet_type::BlastKind;
use super::projectile::{Shot, ShotLayer};
use super::{Arsenal, Combat, Fighter, Landed, Push, ShipRef, blasted};
use crate::chance::Chance;
use crate::geometry::Vec2;
use crate::ship_layer::ShipLayer;

/// Where a ship's slot sprite stands as the collision passes find it: its
/// layer, and its place in that layer's order, which is the order the
/// layer is drawn in, bottom first (`_SWAddSprite` @0xac620). The session
/// gives each ship its rank as the last frame drew it
/// (`nova_sim::Session::draw_layers`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpriteRank {
    /// The layer it stands in.
    pub layer: ShipLayer,
    /// Its place in the layer: a ship of a lower entry is walked before
    /// one of a higher. Only the order counts, not the number itself.
    pub entry: i64,
}

/// The shot layers, in the order `_DoPlayGameWork` collides them with the
/// ships: 0x3b52f0 (@0x43e75), 0x3b531c (@0x43ef1), then 0x3b5308
/// (@0x43f67). Point defence's own layer meets no ship's proc (see the
/// module docs).
pub(super) const SHOT_LAYER_WALK: [ShotLayer; 3] =
    [ShotLayer::Shots, ShotLayer::Homing, ShotLayer::OverShips];

/// The ship layers, in the order each shot layer's pass walks them, top
/// layer first: 0x3b541c (@0x43e7b), 0x3b52a8 (@0x43e9f), then 0x3b53a0
/// (@0x43ebb).
pub(super) const SHIP_LAYER_WALK: [ShipLayer; 3] =
    [ShipLayer::Ships, ShipLayer::Led, ShipLayer::Disabled];

/// The indices among `ranks` of the ships the contact passes walk, in
/// the order they walk them: by layer in [`SHIP_LAYER_WALK`]'s order, and
/// within a layer by entry, bottom first. A ship of no rank, a hidden
/// one, is not walked.
pub(super) fn walk_order(ranks: &[Option<SpriteRank>]) -> Vec<usize> {
    let mut walked: Vec<(usize, SpriteRank)> = ranks
        .iter()
        .enumerate()
        .filter_map(|(index, rank)| rank.map(|rank| (index, rank)))
        .collect();
    walked.sort_by_key(|&(_, rank)| {
        let layer = SHIP_LAYER_WALK
            .iter()
            .position(|&layer| layer == rank.layer);
        (layer, rank.entry)
    });
    walked.into_iter().map(|(index, _)| index).collect()
}

/// What the walk reads beside the ships: each sub-munition's weapon, and
/// when a ship is disabled.
pub(super) struct WalkTerms<'a> {
    /// Each sub-munition's weapon.
    pub(super) arsenal: &'a Arsenal,
    /// When a ship is disabled.
    pub(super) rule: &'a dyn DisableRule,
}

/// The ships a hit lands among, as fighters and as the walk reads them.
type Among<'f, 'a, 'l> = (&'f mut [Fighter<'a>], &'f mut Lineup<'l>);

impl Combat {
    /// The hit pass's ship and asteroid tests (see [`Combat::hit_pass`]
    /// and the module docs), among `fighters` as `lineup` reads them: the
    /// shots `gone` to point defence are taken out first and kept hidden
    /// at life 0 ([`Combat::hide`]); then the contact passes, a shot
    /// layer at a time in [`SHOT_LAYER_WALK`]'s order, each walking the
    /// ships in [`walk_order`] and, for each, the layer's shots in
    /// `entry`'s order ([`Combat::entry_order`]), then the layer's shots
    /// against the asteroids in that order; then point defence's layer
    /// against the asteroids; then the proximity pass, shot by shot in
    /// slot order, a ship then an asteroid. Each hit
    /// and its blast land as they are found ([`Combat::land_hit`]), and
    /// an asteroid's blast likewise. Each proximity hit releases its
    /// sub-munitions in place, their weapons read from `terms`' arsenal and
    /// their spread drawn on `chance` ([`Combat::proximity_pass`]).
    ///
    /// The move that spends a shot is never tested against a ship
    /// (`_ShotCanHitShip` @0x447c2, `_CheckShotProximities` @0x371ea), nor
    /// an asteroid's fuse, but meets an asteroid it touches, as
    /// `_AsteroidCollisionHandler` tests no life. Such a shot meeting none
    /// is kept for its end of life in `_HandleShot` ([`Combat::handle_shots`]),
    /// as is every shot that met nothing.
    pub(super) fn resolve(
        &mut self,
        (fighters, lineup): Among,
        (gone, entry): (&[bool], &[usize]),
        terms: &WalkTerms,
        chance: &mut dyn Chance,
        asteroids: &mut [AsteroidTarget],
    ) {
        let mut shots = std::mem::take(&mut self.shots);
        // A shot spent this pass, or hidden for point defence.
        let mut spent = vec![false; shots.len()];
        for (index, shot) in shots.iter().enumerate() {
            if gone[index] && shot.moved_from.is_some() {
                spent[index] = true;
                self.hide(*shot);
            }
        }
        let ranks: Vec<_> = fighters.iter().map(|fighter| fighter.sprite).collect();
        let order = walk_order(&ranks);
        for layer in SHOT_LAYER_WALK {
            for &ship in &order {
                for &index in entry {
                    let shot = &shots[index];
                    if spent[index] || shot.layer != layer || shot.expired() {
                        continue;
                    }
                    let Some(from) = shot.moved_from else {
                        continue;
                    };
                    let target = &lineup.seen[ship];
                    let Some(at) = shot.touches(from, target) else {
                        continue;
                    };
                    if !lineup.seen_sides().shot_can_hit(shot, target) {
                        continue;
                    }
                    spent[index] = true;
                    let met = (target.ship, at);
                    self.land_hit(&mut shots[index], met, (fighters, lineup), terms.rule);
                }
            }
            let in_layer = |shot: &Shot| shot.layer == layer;
            self.meet_asteroids(
                (&shots, &mut spent, entry),
                in_layer,
                (fighters, lineup),
                terms.rule,
                asteroids,
            );
        }
        let point_defence = |shot: &Shot| shot.layer == ShotLayer::PointDefence;
        self.meet_asteroids(
            (&shots, &mut spent, entry),
            point_defence,
            (fighters, lineup),
            terms.rule,
            asteroids,
        );
        // Every shot of the pass holds its slot through the proximity pass,
        // as only `_HandleShot` frees a spent one (@0x3617c), so a
        // sub-munition released mid-pass is seated as the original seats it.
        self.keep_unspent(shots, &spent);
        self.proximity_pass((fighters, lineup), terms, chance, asteroids);
    }

    /// The hit pass's proximity pass (`_CheckShotProximities`
    /// @0x371bc-0x37775), among `fighters` as `lineup` reads them: each
    /// shot in flight, by slot, that its last move did not spend is set
    /// off by the first ship in slot order it can hit within reach
    /// ([`Shot::fused`]), or else by an asteroid among `asteroids`, and is
    /// spent there, holding its slot. A ship's hit lands at once
    /// ([`Combat::land_hit`]), and then the shot releases its
    /// sub-munitions at that ship (`_HandleShipHit` @0x36f12-0x36f47),
    /// their weapons read from `terms`' arsenal and their spread drawn on
    /// `chance`, seeking among the ships as the pass's earlier hits and its
    /// own left them. A shot that has not moved, a sub-munition released
    /// in the pass among them, sets off no fuse (@0x3720e-0x3721e).
    fn proximity_pass(
        &mut self,
        (fighters, lineup): Among,
        terms: &WalkTerms,
        chance: &mut dyn Chance,
        asteroids: &mut [AsteroidTarget],
    ) {
        let mut next = 0;
        while let Some(index) = self.shots.iter().position(|shot| shot.slot >= next) {
            let shot = self.shots[index];
            next = shot.slot + 1;
            if shot.expired() {
                continue;
            }
            let Some(from) = shot.moved_from else {
                continue;
            };
            if let Some(met) = shot.fused(from, &lineup.sides(), self.proximity_fuse) {
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
    }

    /// Takes the shot in flight at `index` out, its slot held until
    /// `_HandleShot` frees it (@0x3617c).
    fn spend(&mut self, index: usize) {
        let shot = self.shots.remove(index);
        self.slots.hold(shot.slot);
    }

    /// Keeps the shots among `shots` not `spent` in flight; each spent one
    /// holds its slot until `_HandleShot` frees it (@0x3617c).
    fn keep_unspent(&mut self, shots: Vec<Shot>, spent: &[bool]) {
        for (shot, spent) in shots.into_iter().zip(spent) {
            if *spent {
                self.slots.hold(shot.slot);
            } else {
                self.shots.push(shot);
            }
        }
    }

    /// Each shot among `shots` that `takes` in, not yet `spent` and that
    /// has moved, meets the first asteroid along its move among
    /// `asteroids`, if it touches one (`_AsteroidCollisionHandler`, the
    /// asteroid layer's pass for each shot layer): spent there, its hit's
    /// blast landing at once among the ships, as `rule` says
    /// ([`Combat::land_on_asteroid`]).
    fn meet_asteroids(
        &mut self,
        (shots, spent, entry): (&[Shot], &mut [bool], &[usize]),
        takes: impl Fn(&Shot) -> bool,
        (fighters, lineup): Among,
        rule: &dyn DisableRule,
        asteroids: &mut [AsteroidTarget],
    ) {
        for &index in entry {
            let shot = &shots[index];
            if spent[index] || !takes(shot) {
                continue;
            }
            let Some(from) = shot.moved_from else {
                continue;
            };
            if let Some((rock, at)) = asteroid_hit::touched(shot, from, asteroids) {
                spent[index] = true;
                self.land_on_asteroid((shot, &mut asteroids[rock], at), (fighters, lineup), rule);
            }
        }
    }

    /// A shot's hit on an asteroid at a point ([`Combat::hit_asteroid`]),
    /// its blast's hits landed at once among the ships, as `rule` says,
    /// each ship struck read again after them ([`Lineup::refresh`]).
    fn land_on_asteroid(
        &mut self,
        (shot, target, at): (&Shot, &mut AsteroidTarget, Vec2),
        (fighters, lineup): Among,
        rule: &dyn DisableRule,
    ) {
        let mut hits = Vec::new();
        self.hit_asteroid(shot, target, at, fighters, &mut hits);
        let hit: Vec<ShipRef> = hits.iter().map(|landed| landed.ship).collect();
        self.land(fighters, hits, rule);
        lineup.refresh(fighters, &hit, rule);
    }

    /// `shot`'s hit on a ship at a point, landed at once, as
    /// `_HandleShipHit` lands it (see the module docs): it explodes and
    /// sprays its hit particles there, and its direct hit, its decay
    /// taken off, and its blast's hits on the ships around land
    /// ([`Combat::land`]), as `rule` says; each ship struck is then read
    /// again ([`Lineup::refresh`]). The shot is left where it hit, which
    /// its sub-munitions leave from.
    fn land_hit(
        &mut self,
        shot: &mut Shot,
        (ship, at): (ShipRef, Vec2),
        (fighters, lineup): Among,
        rule: &dyn DisableRule,
    ) {
        let hit = Hit {
            cant_kill: shot.disable_only,
            ..Hit::of(&shot.weapon)
        };
        let direct = Landed {
            ship,
            by: shot.attacker(),
            weapon: Some(shot.weapon.id),
            at,
            hit: hit.decayed(shot.decay_steps(self.decay)),
            direct: true,
            intended: shot.aimed_at == Some(ship),
            lock: false,
            weight: shot.weapon.reload,
            push: Push {
                from: at,
                impact: shot.weapon.impact,
                beam: false,
            },
            ion: Some(Dose::of(&shot.weapon, None)),
        };
        shot.position = at;
        let blast = shot.blast(Some(ship));
        self.explode_shot(shot, &blast);
        // Once a hit, where the shot is (`_HandleShipHit`
        // @0x36b44-0x36bae); the ships its blast reaches spray none.
        self.spray(&shot.weapon, at, asteroid_hit::HIT_PART_VARIATION);
        let mut hits = vec![direct];
        // A blast on a hit weighs nothing (`_HandleShipHit` @0x36e80).
        let planet = (BlastKind::OnHit, self.planet_blasts);
        hits.extend(blasted(shot, &blast, (hit, 0.0), &lineup.stood, planet));
        let hit: Vec<ShipRef> = hits.iter().map(|landed| landed.ship).collect();
        self.land(fighters, hits, rule);
        lineup.refresh(fighters, &hit, rule);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rank(layer: ShipLayer, entry: i64) -> SpriteRank {
        SpriteRank { layer, entry }
    }

    #[test]
    fn the_walk_takes_the_ship_layers_top_first_and_each_bottom_first() {
        let ranks = [
            Some(rank(ShipLayer::Disabled, 3)),
            Some(rank(ShipLayer::Ships, 5)),
            Some(rank(ShipLayer::Led, -2)),
            Some(rank(ShipLayer::Ships, -4)),
            Some(rank(ShipLayer::Disabled, 1)),
            Some(rank(ShipLayer::Led, 7)),
        ];
        assert_eq!(walk_order(&ranks), [3, 1, 2, 5, 4, 0]);
    }

    #[test]
    fn a_hidden_ship_is_not_walked() {
        let ranks = [
            Some(rank(ShipLayer::Ships, 0)),
            None,
            Some(rank(ShipLayer::Led, 1)),
        ];
        assert_eq!(walk_order(&ranks), [0, 2]);
    }

    #[test]
    fn the_shot_layers_go_shots_then_homing_then_over_the_ships() {
        assert_eq!(
            SHOT_LAYER_WALK,
            [ShotLayer::Shots, ShotLayer::Homing, ShotLayer::OverShips]
        );
        assert_eq!(
            SHIP_LAYER_WALK,
            [ShipLayer::Ships, ShipLayer::Led, ShipLayer::Disabled]
        );
    }
}
