//! `_HandleShot` once per shot slot, in slot order (`_DoPlayGameWork`
//! @0x4435e-0x44393; rdm weapons-fidelity phases 34 and 47).
//!
//! # The original
//!
//! `_HandleShot` (@0x35586-0x361ab) does all of a slot's work at its
//! visit:
//!
//! - a shot of another system is killed (@0x3559a-0x355a2, @0x3617a);
//! - a shot of no life (@0x355a8-0x355b3 → @0x36195) comes to its end of
//!   life (@0x35f08): its sub-munitions first (`_SpawnShotSubmunitions`
//!   @0x35f49, with `SubCount` above 0 and not class +0x22 & 0x20), then
//!   its detonation (`_CreateExplosion` @0x35f97, then, with a
//!   `BlastRadius`, the blast over slots 0-63, `_DamageShip` @0x36106 and
//!   `_IonizeShip` @0x3612d on each ship it reaches) or a fizzle
//!   (`_SpawnExplod` @0x36175); then its slot is freed (@0x3617c);
//! - any other shot loses a tick of life (@0x35686-0x356a3), steers and
//!   moves (`_HandleShotGuidance` @0x356d0, @0x356d5-0x3570f), then makes
//!   its missile lock (@0x35717-0x35861, `_DamageShip` @0x35855), its spin
//!   (@0x35879-0x358c4), its frames and its trail.
//!
//! So a shot's own sub-munitions seek before its own blast lands, and read
//! a ship that blast disables as it was; but every later slot's visit (its
//! sub-munitions' seek, `_ShotCanHitShip` @0x4477e; its move; its lock's
//! leader test @0x35773) reads what the earlier visits landed, an escort's
//! cleared leader included (`_DamageShip` @0x3afdd, the `escort_loss`
//! module). The strikes land in slot order. The release's sound (@0x3f2ad)
//! comes before the explosion's. Neither the lock nor the blast makes a
//! sound of its own.
//!
//! Here [`Combat::handle_shots`] walks the slots the same way, landing each
//! blast and each lock at its visit.

use super::lineup::Lineup;
use super::projectile::{Guide, Target};
use super::{Combat, Fighter, Landed, ShipRef, Sides, blasted, lock};
use crate::chance::Chance;
use crate::combat::CombatEvent;
use crate::combat::armament::Arsenal;
use crate::combat::damage::Hit;
use crate::combat::hull::DisableRule;
use crate::combat::projectile::Shot;
use crate::geometry::Vec2;
use crate::govt::Governments;

/// Puts each of `fighters` at its place among `at`, a view of the
/// [`Lineup`].
fn place(fighters: &mut [Fighter], at: &[Target]) {
    for (fighter, target) in fighters.iter_mut().zip(at) {
        fighter.state.position = target.position;
    }
}

impl Combat {
    /// `_HandleShot` for each shot slot in turn (see the module docs),
    /// among `fighters`, by `govts`' relations, each sub-munition's weapon
    /// read from `arsenal`, each ship struck settling as `rule` says, the
    /// draws made on `chance`. At each slot's visit:
    ///
    /// - a slot held by a shot that ended since the last loop is freed
    ///   (@0x356c1, @0x30ab1 → @0x3617c);
    /// - a shot of no life (`Count` 0 or below) is dropped unmoved and
    ///   unseen, as `_HandleShot` sends it straight to its end of life with
    ///   its sprite never shown (@0x355a8-0x355b3, @0x35f0a), freeing its
    ///   slot;
    /// - a shot whose last move spent it ([`Shot::expired`]) comes to its
    ///   end of life (@0x36195 → @0x35f08): it releases its sub-munitions
    ///   at the ship its damage is aimed at (@0x35f49), each weapon read
    ///   from `arsenal`, its spread drawn on `chance`, its owner's plunder
    ///   read among `fighters`, each taking the first slot free as of this
    ///   visit, its own still held, and seeking among the ships as the
    ///   earlier visits left them; then it detonates, exploding where it
    ///   is and blasting the ships it reaches with its weapon's whole
    ///   damage, weighing its `Reload` (@0x360a2), each NPC met where it
    ///   stood as the last tick ran and the player where it is, as the
    ///   collision passes meet them (`Combat::hit_pass`), or it fizzles;
    ///   its blast's hits land at once ([`Combat::land`]), and its slot is
    ///   freed (@0x3617c);
    /// - any other shot flies a tick ([`Shot::step`]), homing among the
    ///   ships where they are now, with each ship's jamming and sight as
    ///   the hits so far left them
    ///   ([`Sight`](super::projectile::Sight)), and runs a spinning
    ///   shot's spin timer in its slot (`ShotSlots::spin_pass`,
    ///   @0x35879-0x358c4); then its missile lock, if it makes one
    ///   ([`lock`]), lands at once (@0x35855), reading the target where it
    ///   is now. A homing shot reads the `asteroids`, by slot, where they
    ///   are as `_HandleShot` meets them, which `_HandleAsteroids` moves
    ///   only after the ship loop (see
    ///   [`asteroid_seek`](super::asteroid_seek)).
    ///
    /// Each ship a visit's landing strikes settles its condition at once
    /// (`Combat::settle_struck`), and is read again whole for the visits
    /// after it ([`Lineup::refresh`]). A sub-munition released into a slot
    /// below the visit is not visited again this loop, so it sits out the
    /// tick's moves; one released above it is visited later and moves.
    pub(super) fn handle_shots(
        &mut self,
        fighters: &mut [Fighter],
        (govts, arsenal): (&Governments, &Arsenal),
        rule: &dyn DisableRule,
        asteroids: &[Option<Vec2>],
        chance: &mut dyn Chance,
    ) {
        // Each NPC where it stood as the last tick ran, which the ends of
        // life meet as the collision passes do, and each ship where it is
        // now, which the moves and the locks meet.
        let now_at = self.stand_where_they_stood(fighters);
        let mut lineup = Lineup::new(fighters, &now_at, govts, self.pass_through);
        place(fighters, &lineup.seen);
        let mut visit = 0;
        while visit < self.slot_bound() {
            if self.slots.is_held(visit) {
                self.slots.free(visit);
            } else if let Ok(index) = self.shots.binary_search_by_key(&visit, |shot| shot.slot) {
                let shot = self.shots[index];
                if shot.weapon.lifetime == 0 {
                    self.shots.remove(index);
                } else if shot.moved_from.is_some() && shot.expired() {
                    let sides = lineup.sides();
                    let hits = self.end_life(&shot, fighters, &sides, arsenal, chance);
                    self.shots.retain(|flying| flying.slot != visit);
                    self.land_at_visit(fighters, hits, (&mut lineup, true), rule);
                } else {
                    self.move_shot(index, &lineup, asteroids, chance);
                    let sides = lineup.seen_sides();
                    let locked: Vec<Landed> =
                        lock(&self.shots[index], &sides).into_iter().collect();
                    self.land_at_visit(fighters, locked, (&mut lineup, false), rule);
                }
            }
            visit += 1;
        }
    }

    /// One past the highest slot held or holding a shot in flight.
    fn slot_bound(&self) -> usize {
        let held = self.slots.held().last();
        let flying = self.shots.last().map(|shot| shot.slot);
        held.max(flying).map_or(0, |slot| slot + 1)
    }

    /// The shot in flight at `index` flies a tick among the ships of
    /// `lineup` where they are now and the `asteroids`, by slot, drawing on
    /// `chance`, and runs its slot's spin timer if it spins (see
    /// [`Combat::handle_shots`]).
    fn move_shot(
        &mut self,
        index: usize,
        lineup: &Lineup,
        asteroids: &[Option<Vec2>],
        chance: &mut dyn Chance,
    ) {
        let guide = Guide {
            asteroids,
            sights: &lineup.sights,
            frame: self.field.frame,
            lock_angle: self.lock_angle,
            bomb_heading_wrap: self.bomb_heading_wrap,
            ..Guide::among(&lineup.seen)
        };
        let shot = &mut self.shots[index];
        shot.step(&guide, chance);
        if shot.weapon.spins() {
            self.slots.spin_pass(shot.slot, shot.weapon.beam_width);
        }
    }

    /// Lands `hits` among `fighters` at a visit ([`Combat::land`]), as
    /// `rule` says, the ships standing where they stood when `standing`
    /// (a blast's) and where they are now otherwise (a lock's); then each
    /// ship struck settles its condition where it is now, and `lineup`
    /// reads it again.
    fn land_at_visit(
        &mut self,
        fighters: &mut [Fighter],
        hits: Vec<Landed>,
        (lineup, standing): (&mut Lineup, bool),
        rule: &dyn DisableRule,
    ) {
        if hits.is_empty() {
            return;
        }
        let first_strike = self.strikes.len();
        let struck: Vec<ShipRef> = hits.iter().map(|landed| landed.ship).collect();
        if standing {
            place(fighters, &lineup.stood);
        }
        self.land(fighters, hits, rule);
        place(fighters, &lineup.seen);
        self.settle_struck(fighters, first_strike, rule);
        lineup.refresh(fighters, &struck, rule);
    }

    /// `shot`'s end of life among `sides` (see [`Combat::handle_shots`]):
    /// its sub-munitions released, then its explosion or fizzle. Gives
    /// its blast's hits, to land.
    fn end_life(
        &mut self,
        shot: &Shot,
        fighters: &[Fighter],
        sides: &Sides,
        arsenal: &Arsenal,
        chance: &mut dyn Chance,
    ) -> Vec<Landed> {
        if shot.weapon.subs_on_expiry() {
            self.release(shot, (shot.aimed_at, fighters), sides, arsenal, chance);
        }
        if shot.weapon.detonates() {
            let hit = Hit {
                cant_kill: shot.disable_only,
                ..Hit::of(&shot.weapon)
            };
            let blast = shot.blast(None);
            self.explode_shot(shot, &blast);
            let weight = shot.weapon.reload;
            return blasted(shot, &blast, hit, weight, sides.targets);
        }
        if shot.weapon.fizzles {
            self.events.push(CombatEvent::Fizzled { at: shot.position });
        }
        Vec::new()
    }
}
