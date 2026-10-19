//! `_HandleShot`'s loop over the shot slots (rdm weapons-fidelity phase
//! 34): each slot's visit frees a held slot, drops a shot of no life, and
//! runs a spent shot's end of life (@0x36195 → @0x35f08), which detonates
//! or fizzles where it ended and releases its sub-munitions before its
//! slot is freed (@0x3617c).

use super::{Combat, Fighter, Landed, Plunderer, Sides, Source, blasted};
use crate::chance::Chance;
use crate::combat::CombatEvent;
use crate::combat::armament::Arsenal;
use crate::combat::damage::Hit;
use crate::combat::projectile::Shot;

/// What `_HandleShot`'s loop over the slots leaves ([`Combat::visit_slots`]):
/// the hits of the blasts to land, and the sub-munitions released into a
/// slot the loop had passed, which are not moved this tick.
pub(super) struct Visited {
    /// The blasts' hits, to land.
    pub(super) expiries: Vec<Landed>,
    /// The slots of the sub-munitions released behind the visit.
    pub(super) sitting_out: Vec<usize>,
}

impl Combat {
    /// `_HandleShot`'s loop over the slots, in slot order
    /// (`_DoPlayGameWork` @0x4435e-0x44393), all but the moves, which
    /// [`Combat::fly`] makes after it. At each slot's visit:
    ///
    /// - a slot held by a shot that ended since the last loop is freed
    ///   (@0x356c1, @0x30ab1 → @0x3617c);
    /// - a shot of no life (`Count` 0 or below) is dropped unmoved and
    ///   unseen, as `_HandleShot` sends it straight to its end of life with
    ///   its sprite never shown (@0x355a8-0x355b3, @0x35f0a), freeing its
    ///   slot;
    /// - a shot whose last move spent it ([`Shot::expired`]) comes to its
    ///   end of life (@0x36195 → @0x35f08): it detonates, exploding where
    ///   it is and blasting the ships among `sides` it reaches with its
    ///   weapon's whole damage, weighing its `Reload` (@0x360a2), or
    ///   fizzles; then it releases its sub-munitions at the ship its damage
    ///   is aimed at (@0x35f49), each weapon read from `arsenal`, its spread
    ///   drawn on `chance`, its owner's plunder read among `fighters`, each
    ///   taking the first slot free as of this visit, its own still held;
    ///   then its slot is freed (@0x3617c). A shot that has not moved yet
    ///   is left for its first move.
    ///
    /// A sub-munition released into a slot below the visit is not visited
    /// again this loop, so it sits out the tick's moves; one released
    /// above it is visited later and moves.
    ///
    /// The original moves each shot at its own visit, between the other
    /// slots' ends of life; here every end of life comes before every
    /// move. No end of life reads where another shot is, and both settle
    /// before the frame is drawn, crossing no boundary the player sees.
    pub(super) fn visit_slots(
        &mut self,
        fighters: &[Fighter],
        sides: &Sides,
        arsenal: &Arsenal,
        chance: &mut dyn Chance,
    ) -> Visited {
        let mut visited = Visited {
            expiries: Vec::new(),
            sitting_out: Vec::new(),
        };
        let mut visit = 0;
        while visit < self.slot_bound() {
            if self.slots.is_held(visit) {
                self.slots.free(visit);
            } else if let Ok(index) = self.shots.binary_search_by_key(&visit, |shot| shot.slot) {
                let shot = self.shots[index];
                if shot.weapon.lifetime == 0 {
                    self.shots.remove(index);
                } else if shot.moved_from.is_some() && shot.expired() {
                    let (hits, placed) = self.end_life(&shot, fighters, sides, arsenal, chance);
                    visited.expiries.extend(hits);
                    self.shots.retain(|flying| flying.slot != visit);
                    visited
                        .sitting_out
                        .extend(placed.into_iter().filter(|&sub| sub < visit));
                }
            }
            visit += 1;
        }
        visited
    }

    /// One past the highest slot held or holding a shot in flight.
    fn slot_bound(&self) -> usize {
        let held = self.slots.held().last();
        let flying = self.shots.last().map(|shot| shot.slot);
        held.max(flying).map_or(0, |slot| slot + 1)
    }

    /// `shot`'s end of life (see [`Combat::visit_slots`]): its blast's
    /// hits, to land, and the slots its sub-munitions took.
    fn end_life(
        &mut self,
        shot: &Shot,
        fighters: &[Fighter],
        sides: &Sides,
        arsenal: &Arsenal,
        chance: &mut dyn Chance,
    ) -> (Vec<Landed>, Vec<usize>) {
        let mut expiries = Vec::new();
        let mut placed = Vec::new();
        if shot.weapon.detonates() {
            let hit = Hit {
                cant_kill: shot.disable_only,
                ..Hit::of(&shot.weapon)
            };
            let blast = shot.blast(None);
            self.explode_shot(shot, &blast);
            let weight = shot.weapon.reload;
            expiries.extend(blasted(shot, &blast, hit, weight, sides.targets));
        } else if shot.weapon.fizzles {
            self.events.push(CombatEvent::Fizzled { at: shot.position });
        }
        if shot.weapon.subs_on_expiry() {
            // `_SpawnShotSubmunitions` spawns as the parent's owner (+0x2c):
            // its plunder as it is now, not the parent's flag.
            let owner = fighters
                .iter()
                .find(|fighter| fighter.ship == shot.firer)
                .filter(|_| matches!(shot.source, Source::Ship(_)))
                .map_or_else(Plunderer::default, Plunderer::of);
            placed = self.release(shot, (shot.aimed_at, owner), sides, arsenal, chance);
        }
        (expiries, placed)
    }
}
