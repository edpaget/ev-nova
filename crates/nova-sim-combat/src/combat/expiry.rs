//! A spent shot's end of life, as `_HandleShot` runs it (@0x36195 →
//! @0x35f08): at the shot's first visit after the move that spent it, it
//! detonates or fizzles where it ended and releases its sub-munitions, and
//! its slot is freed (@0x3617c).

use super::{Combat, Fighter, Landed, Plunderer, Sides, Source, blasted};
use crate::chance::Chance;
use crate::combat::CombatEvent;
use crate::combat::armament::Arsenal;
use crate::combat::damage::Hit;
use crate::combat::projectile::Shot;

impl Combat {
    /// The end of life of each shot whose last move spent it
    /// ([`Shot::expired`]), in the order they are kept (`_HandleShot`
    /// @0x36195 → @0x35f08): it detonates, exploding where it is and
    /// blasting the ships among `sides` it reaches with its weapon's
    /// whole damage, weighing its `Reload` (@0x360a2), or fizzles; then it
    /// releases its sub-munitions at the ship its damage is aimed at
    /// (@0x35f49), each weapon read from `arsenal`, its spread drawn on
    /// `chance`, its owner's plunder read among `fighters`; and it leaves
    /// the table, still holding its slot through the releases
    /// ([`Combat::shot_room`]), which `_HandleShot` frees after them
    /// (@0x3617c, see [`Combat::fly_shots`]). A shot that
    /// has not moved yet is left for its first move, as a shot of no life
    /// is dropped unmoved ([`Combat::fly`]). Gives the blasts' hits to land.
    ///
    /// The original runs each end of life at that shot's slot in its
    /// `_HandleShot` loop, between the other shots' moves; here every end
    /// of life comes before every move. Both settle before the frame is
    /// drawn, crossing no boundary the player sees.
    pub(super) fn expire(
        &mut self,
        fighters: &[Fighter],
        sides: &Sides,
        arsenal: &Arsenal,
        chance: &mut dyn Chance,
    ) -> Vec<Landed> {
        let mut expiries: Vec<Landed> = Vec::new();
        let mut released: Vec<Shot> = Vec::new();
        let mut kept = Vec::with_capacity(self.shots.len());
        let mut ended = Vec::new();
        for shot in std::mem::take(&mut self.shots) {
            if shot.moved_from.is_none() || !shot.expired() {
                kept.push(shot);
                continue;
            }
            if shot.weapon.detonates() {
                let hit = Hit {
                    cant_kill: shot.disable_only,
                    ..Hit::of(&shot.weapon)
                };
                let blast = shot.blast(None);
                self.explode_shot(&shot, &blast);
                let weight = shot.weapon.reload;
                expiries.extend(blasted(&shot, &blast, hit, weight, sides.targets));
            } else if shot.weapon.fizzles {
                self.events.push(CombatEvent::Fizzled { at: shot.position });
            }
            if shot.weapon.subs_on_expiry() {
                released.push(shot);
            }
            ended.push(shot.slot);
        }
        for slot in ended {
            self.slots.hold(slot);
        }
        self.shots = kept;
        for parent in released {
            // `_SpawnShotSubmunitions` spawns as the parent's owner (+0x2c):
            // its plunder as it is now, not the parent's flag.
            let owner = fighters
                .iter()
                .find(|fighter| fighter.ship == parent.firer)
                .filter(|_| matches!(parent.source, Source::Ship(_)))
                .map_or_else(Plunderer::default, Plunderer::of);
            self.release(&parent, (parent.aimed_at, owner), sides, arsenal, chance);
        }
        expiries
    }
}
