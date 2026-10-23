//! Homing shots that seek asteroids: `_HandleShotGuidance`'s `Seeker`
//! 0x0002 branch and the steering of a shot it locks.
//!
//! # The original
//!
//! In `EV Nova.app/Contents/MacOS/EV Nova` (rdm asteroids phase 7 records
//! the reading):
//!
//! - **Who rolls.** Every homing shot whose guidance began the tick at
//!   jam 0 reaches the branch (@0x3252d) after its tick's velocity, or
//!   straight from the entry when its target's slot is inactive or holds
//!   another ship (@0x3213b, @0x32149), and with no age test. A lost
//!   (998), interfered (999) or locked (1) shot goes to the jam dispatch
//!   instead (@0x3210e) and never rolls. `_HandleShot` makes a shot whose
//!   ship is gone lost before its guidance (@0x355e0-0x35621), so here
//!   ([`Shot::step`]) such a shot never rolls either. The engine's ships
//!   are identities, not slots, so it has no slot another ship took.
//! - **The lock** (@0x3253b-0x3260e, [`lock_on`]): with guidance 1 and
//!   `Seeker` 0x0002, a `_Rand(10)` of none ([`SEEK_ODDS`], once a tick)
//!   walks the 16 asteroid slots in order and takes the first whose
//!   `trunc(asteroid − shot)` is within [`SEEK_BOX`] on each axis
//!   (@0x3257c-0x325aa) and whose truncated bearing less the heading is
//!   within [`SEEK_CONE`] (@0x325ac-0x325fb). The original measures that
//!   angle as the `Seeker` 0x4000 test does, absolute and mod 360 with no
//!   fold, so the rulebook's
//!   [`RuleKey::SeekerLockAngle`](crate::RuleKey::SeekerLockAngle) governs
//!   both ([`LockAngle`]). The shot is then jam 1 and its ship slot (+0x2a)
//!   holds the asteroid's slot (@0x32919-0x3291f): here
//!   [`Jam::Asteroid`], with its target and the ship its damage is aimed
//!   at cleared. Its velocity for the tick is already set, so it steers
//!   from the next.
//! - **The steering** (@0x3277c-0x32917, [`Shot::steer_at_asteroid`]): the
//!   bearing is `_CalcInterceptAngle` of its slot's asteroid, whatever
//!   asteroid that is now, a fragment or slot 0's refill included. Past
//!   [`HOMING_DELAY`] ticks (`Count − life > 15.0`, unscaled, @0x327eb)
//!   it turns its whole `GuidedTurn` when `|_AngularDifference|` is above
//!   the untruncated turn, the way [`turned`] says. Its heading is then
//!   wrapped and its velocity is its speed along it. A slot found empty
//!   is let go (+0x2a −1, @0x327b5), and from then on the shot's own
//!   truncated heading is its bearing (@0x327bd), so it flies straight.
//!   A locked shot skips the jam, cloak and 0x4000 tests and the branch,
//!   so it never locks again and never goes back to jam 0.
//! - Neither plays a sound: the range calls only `_Rand`,
//!   `_CalcInterceptAngle`, `_AngularDifference`, `_Accel` and
//!   `_SpeedLimit`.
//!
//! A locked shot is jammed for everything that tests jam 0: it makes no
//! missile lock (@0x35756), point defence never picks it (@0x393e9), and
//! its damage counts as aimed at no ship (`_CalculateTargetedDamage`
//! @0x3a1bd). It hits any asteroid it meets (the asteroid passes read no
//! jam; see [`asteroid_hit`](super::asteroid_hit)).
//!
//! # Left out
//!
//! - On the tick its slot is found empty, the original turns at the stale
//!   `ecx`, the low word of the weapon table's handle (@0x327b5 →
//!   0x327c8): an address, which can't be replicated. Here it reads its
//!   own heading, as with no slot.
//! - The box test's 16-bit wrap (`addw $0xc7` / `cmpw $0x18e`) is not
//!   replicated.
//! - The original reads the asteroid's slot k in +0x2a as a ship slot
//!   elsewhere: `_ShotCanHitShip` rejects only jam 998 (@0x447f7), so a
//!   locked shot can hit the ship in ship slot k, and any ship under
//!   `Flags2` 0x0008 (@0x44821-0x44836); and its end-of-life sub-munitions
//!   are aimed at that ship (@0x35f3e-0x35f49). Here a locked shot hits no
//!   ship ([`Sides`](super::sides::Sides)) and its sub-munitions are aimed
//!   at none: rdm:phase/asteroids-fidelity/phase-1-locked-shot-reads-asteroid-slot-as-ship-slot.

use super::projectile::{Guide, HOMING_DELAY, Jam, LockAngle, Shot, wrap_turned};
use super::weapon::{Guidance, SEEKS_ASTEROIDS};
use crate::chance::Chance;
use crate::geometry::Vec2;
use crate::trig;

/// How near, on each axis, an asteroid must be for a seeking shot to lock
/// onto it, truncated (±199: `addw $0xc7`, `cmpw $0x18e` unsigned,
/// @0x3258a-0x325aa).
pub const SEEK_BOX: f32 = 199.0;
/// How far off its heading, in degrees, an asteroid may lie for a seeking
/// shot to lock onto it (`cmpw $0xf` @0x325f6).
pub const SEEK_CONE: f32 = 15.0;
/// The odds, one in this, that a seeking shot looks for an asteroid in a
/// tick (`_Rand(10)` @0x32550).
pub const SEEK_ODDS: u32 = 10;

/// The slot of the asteroid a shot at `position` heading `heading` locks
/// onto among `asteroids`, by slot (@0x32565-0x3260e): the first holding
/// one within [`SEEK_BOX`] on each axis, truncated, whose table bearing
/// less the heading, truncated, is within [`SEEK_CONE`] as `measure`
/// says.
#[must_use]
pub fn lock_on(
    position: Vec2,
    heading: f32,
    asteroids: &[Option<Vec2>],
    measure: LockAngle,
) -> Option<usize> {
    asteroids.iter().enumerate().find_map(|(slot, asteroid)| {
        let at = (*asteroid)?;
        let off = at - position;
        if off.x.trunc().abs() > SEEK_BOX || off.y.trunc().abs() > SEEK_BOX {
            return None;
        }
        let bearing = trig::intercept_angle(position, at);
        let angle = (f32::from(bearing) - heading).trunc();
        (measure.off_ahead(angle) <= SEEK_CONE).then_some(slot)
    })
}

/// A locked shot's heading after its turn of `turn` at the whole-degree
/// `bearing` (@0x327f9-0x3287d): it turns only while
/// `|_AngularDifference(bearing, trunc(heading))|` is above `turn`, a
/// float compare against the untruncated turn, where a clear shot's
/// ([`homing_turn`](super::projectile::homing_turn)) truncates it; back
/// when `trunc(bearing − heading)`, the float difference truncated and
/// taken into 0..359, is above 180, and on otherwise. A negative turn
/// turns even at its bearing.
#[must_use]
pub fn turned(heading: f32, bearing: i16, turn: f32) -> f32 {
    let off = f32::from(trig::angular_difference(bearing, trig::whole(heading)).abs());
    if off <= turn {
        return heading;
    }
    let mut ahead = i32::from(trig::whole(f32::from(bearing) - heading));
    if ahead >= 360 {
        ahead -= 360;
    }
    if ahead <= -1 {
        ahead += 360;
    }
    if ahead > 180 {
        heading - turn
    } else {
        heading + turn
    }
}

impl Shot {
    /// The `Seeker` 0x0002 branch, for a homing shot whose guidance began
    /// the tick not jammed, after its velocity is set (see the module
    /// docs): with the bit, a [`SEEK_ODDS`] draw of none on `chance` looks
    /// for an asteroid among `guide`'s ([`lock_on`], by its
    /// [`LockAngle`]); one found locks the shot onto its slot, and it is
    /// aimed at no ship from then on.
    pub(super) fn seek_asteroid(&mut self, guide: &Guide, chance: &mut dyn Chance) {
        if self.weapon.guidance != Guidance::Homing
            || !self.weapon.seeks(SEEKS_ASTEROIDS)
            || chance.below(SEEK_ODDS) != 0
        {
            return;
        }
        let found = lock_on(
            self.position,
            self.heading,
            guide.asteroids,
            guide.lock_angle,
        );
        if let Some(slot) = found {
            self.jam = Jam::Asteroid(Some(slot));
            self.target = None;
            self.aimed_at = None;
        }
    }

    /// The jam-1 steering of a shot locked onto `slot` (see the module
    /// docs): at the bearing of the asteroid `guide` has in it, past
    /// [`HOMING_DELAY`] ticks ([`turned`]); a slot found empty is let go,
    /// and the shot keeps its heading that tick and every tick after, as
    /// with no slot. Then its velocity is its speed along its heading.
    pub(super) fn steer_at_asteroid(&mut self, slot: Option<usize>, guide: &Guide) {
        let at = slot.map(|slot| guide.asteroids.get(slot).copied().flatten());
        let bearing = match at {
            Some(Some(at)) => trig::intercept_angle(self.position, at),
            Some(None) => {
                self.jam = Jam::Asteroid(None);
                trig::whole(self.heading)
            }
            None => trig::whole(self.heading),
        };
        if self.age >= HOMING_DELAY {
            self.heading = turned(self.heading, bearing, self.weapon.turn);
        }
        self.heading = wrap_turned(self.heading);
        self.velocity = trig::accel(trig::whole(self.heading), self.weapon.speed);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::ShipId;
    use crate::catalog::WeaponRecord;
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::combat::projectile::{HOMING_DELAY, Jam, Target};
    use crate::combat::sides::Ties;
    use crate::combat::weapon::{Guidance, SEEKS_ASTEROIDS, WeaponSpec};
    use crate::combat::{NpcId, ShipRef};
    use crate::flight::ShipState;
    use crate::ship_layer::Slot;
    use nova_sim_base::fixture::Ranged;
    use nova_sim_catalog::fixture::weapon;

    const SHIP: ShipRef = ShipRef::Npc(NpcId(2));

    /// A homing missile of `seeker`, 10 pixels a tick, turning 7 degrees,
    /// living 100 ticks.
    fn missile(seeker: u16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance: 1,
            speed: 1000,
            count: 100,
            guided_turn: 70,
            seeker,
            ..weapon(134)
        })
    }

    /// `weapon` launched from the centre heading up, aimed at `SHIP`.
    fn fired(weapon: WeaponSpec) -> Shot {
        let from = ShipState {
            position: Vec2::ZERO,
            ..ShipState::default()
        };
        Shot {
            target: Some(SHIP),
            aimed_at: Some(SHIP),
            ..Shot::launch(weapon, ShipRef::Player, ShipRef::Player, &from, 0.0)
        }
    }

    /// `SHIP` at (`x`, `y`), intact.
    fn ship_at(x: f32, y: f32) -> Target {
        Target {
            ship: SHIP,
            ship_type: ShipId(128),
            fleet: SHIP,
            ties: Ties::default(),
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            radius: 5.0,
            sprite_x: 15.0,
            sheet: true,
            condition: Condition::Intact,
            disabled: false,
            miss_class: 100,
            slot: Slot::Berth(2),
        }
    }

    /// `shot` flown a tick among `ships` and `asteroids`, its draws on
    /// `chance`.
    fn fly(shot: &mut Shot, ships: &[Target], asteroids: &[Option<Vec2>], chance: &mut dyn Chance) {
        let guide = Guide {
            asteroids,
            ..Guide::among(ships)
        };
        shot.step(&guide, chance);
    }

    /// A draw of none for the seek, every other draw its highest.
    fn seeks() -> Ranged {
        Ranged::of(&[(SEEK_ODDS, 0)])
    }

    /// How many seeks `chance` was asked to draw.
    fn rolls(chance: &Ranged) -> usize {
        chance.asked.iter().filter(|&&n| n == SEEK_ODDS).count()
    }

    #[test]
    fn a_seeking_missile_locks_onto_an_asteroid_ahead_on_a_roll_of_none_from_its_first_tick() {
        let ships = [ship_at(300.0, 0.0)];
        let rocks = [None, at(0.0, -150.0)];
        let mut shot = fired(missile(SEEKS_ASTEROIDS));
        let mut chance = seeks();
        fly(&mut shot, &ships, &rocks, &mut chance);
        assert_eq!(shot.jam, Jam::Asteroid(Some(1)));
        assert_eq!(
            (shot.target, shot.aimed_at),
            (None, None),
            "its asteroid's slot overwrites its ship"
        );
        assert_eq!(rolls(&chance), 1, "one roll a tick, not one an asteroid");
        assert_eq!(
            shot.velocity,
            trig::accel(0, 10.0),
            "the tick's velocity was set before the lock"
        );
        let mut missed = fired(missile(SEEKS_ASTEROIDS));
        fly(
            &mut missed,
            &ships,
            &rocks,
            &mut Ranged::of(&[(SEEK_ODDS, 1)]),
        );
        assert_eq!(missed.jam, Jam::Clear, "a roll of 1");
        assert_eq!(missed.target, Some(SHIP));
        let mut beyond = fired(missile(SEEKS_ASTEROIDS));
        fly(&mut beyond, &ships, &[at(0.0, -300.0)], &mut seeks());
        assert_eq!(beyond.jam, Jam::Clear, "nothing in reach");
    }

    #[test]
    fn a_missile_without_the_bit_or_a_shot_not_homing_never_rolls() {
        let rocks = [at(0.0, -150.0)];
        let ships = [ship_at(300.0, 0.0)];
        let mut plain = fired(missile(0));
        let mut chance = seeks();
        fly(&mut plain, &ships, &rocks, &mut chance);
        assert_eq!(plain.jam, Jam::Clear);
        assert_eq!(rolls(&chance), 0);
        let rocket = WeaponSpec {
            guidance: Guidance::Rocket,
            ..missile(SEEKS_ASTEROIDS)
        };
        let mut rocket = fired(rocket);
        let mut chance = seeks();
        fly(&mut rocket, &ships, &rocks, &mut chance);
        assert_eq!(rocket.jam, Jam::Clear);
        assert_eq!(rolls(&chance), 0);
    }

    #[test]
    fn a_jammed_or_locked_shot_never_rolls() {
        let rocks = [at(0.0, -150.0), at(0.0, -100.0)];
        let ships = [ship_at(300.0, 0.0)];
        for jam in [Jam::Interfered, Jam::Asteroid(Some(1)), Jam::Asteroid(None)] {
            let mut shot = Shot {
                jam,
                ..fired(missile(SEEKS_ASTEROIDS))
            };
            let mut chance = seeks();
            fly(&mut shot, &ships, &rocks, &mut chance);
            assert_eq!(shot.jam, jam);
            assert_eq!(rolls(&chance), 0, "{jam:?}");
        }
    }

    #[test]
    fn a_missile_whose_ship_went_this_tick_is_lost_and_never_locks() {
        let mut shot = fired(missile(SEEKS_ASTEROIDS));
        let mut chance = seeks();
        fly(&mut shot, &[], &[at(0.0, -150.0)], &mut chance);
        assert_eq!(shot.jam, Jam::Lost);
        assert_eq!(rolls(&chance), 0);
    }

    #[test]
    fn a_clear_missile_with_no_ship_target_still_rolls() {
        let mut shot = Shot {
            target: None,
            aimed_at: None,
            ..fired(missile(SEEKS_ASTEROIDS))
        };
        fly(&mut shot, &[], &[at(0.0, -150.0)], &mut seeks());
        assert_eq!(shot.jam, Jam::Asteroid(Some(0)));
    }

    /// `missile` locked onto `slot`, `age` ticks old, at the centre
    /// heading up.
    fn locked(slot: Option<usize>, age: u32) -> Shot {
        Shot {
            jam: Jam::Asteroid(slot),
            target: None,
            aimed_at: None,
            age,
            ..fired(missile(SEEKS_ASTEROIDS))
        }
    }

    #[test]
    fn past_its_delay_a_locked_shot_turns_its_turn_at_its_asteroid() {
        let rocks = [at(150.0, -150.0)];
        let mut shot = locked(Some(0), HOMING_DELAY);
        fly(&mut shot, &[], &rocks, &mut NeverFires);
        assert_eq!(shot.heading, 7.0);
        assert_eq!(shot.velocity, trig::accel(7, 10.0));
        let mut young = locked(Some(0), HOMING_DELAY - 1);
        fly(&mut young, &[], &rocks, &mut NeverFires);
        assert_eq!(young.heading, 0.0, "within its delay");
        assert_eq!(young.jam, Jam::Asteroid(Some(0)));
        // An asteroid 3 or 4 degrees off, within the turn: it holds.
        let mut near = locked(Some(0), HOMING_DELAY);
        fly(&mut near, &[], &[at(10.0, -150.0)], &mut NeverFires);
        assert_eq!(near.heading, 0.0);
    }

    #[test]
    fn a_locked_shot_steers_from_the_tick_after_its_lock() {
        // About 11 degrees right: in the cone, and further off than the
        // turn.
        let rocks = [at(30.0, -150.0)];
        let mut shot = Shot {
            age: HOMING_DELAY,
            target: None,
            aimed_at: None,
            ..fired(missile(SEEKS_ASTEROIDS))
        };
        fly(&mut shot, &[], &rocks, &mut seeks());
        assert_eq!(shot.jam, Jam::Asteroid(Some(0)));
        assert_eq!(shot.heading, 0.0, "the lock's tick");
        fly(&mut shot, &[], &rocks, &mut seeks());
        assert_eq!(shot.heading, 7.0);
    }

    #[test]
    fn a_locked_shot_whose_asteroid_is_gone_flies_straight_on_and_never_locks_again() {
        let mut shot = locked(Some(1), HOMING_DELAY);
        let ahead = [at(150.0, -150.0), None];
        fly(&mut shot, &[], &ahead, &mut NeverFires);
        assert_eq!(shot.jam, Jam::Asteroid(None));
        assert_eq!(shot.heading, 0.0, "the gone tick keeps its heading");
        let field = [at(150.0, -150.0), at(0.0, -100.0)];
        for _ in 0..5 {
            let mut chance = seeks();
            fly(&mut shot, &[], &field, &mut chance);
            assert_eq!(shot.jam, Jam::Asteroid(None));
            assert_eq!(rolls(&chance), 0);
            assert_eq!(shot.heading, 0.0);
            assert_eq!(shot.velocity, trig::accel(0, 10.0));
        }
    }

    #[test]
    fn a_locked_shot_chases_whatever_asteroid_its_slot_holds() {
        let mut shot = locked(Some(0), HOMING_DELAY);
        fly(&mut shot, &[], &[at(150.0, 0.0)], &mut NeverFires);
        assert_eq!(shot.heading, 7.0, "on, at its asteroid to the right");
        fly(&mut shot, &[], &[at(-150.0, 0.0)], &mut NeverFires);
        assert_eq!(shot.heading, 0.0, "back, at a new one to the left");
    }

    const PLAIN: LockAngle = LockAngle::Plain;
    const SHORTEST: LockAngle = LockAngle::Shortest;

    /// A slot holding an asteroid at (`x`, `y`).
    #[allow(clippy::unnecessary_wraps)]
    fn at(x: f32, y: f32) -> Option<Vec2> {
        Some(Vec2::new(x, y))
    }

    #[test]
    fn a_shot_locks_onto_the_first_slot_ahead_within_199_pixels() {
        // Heading up (0), so straight up is dead ahead.
        let field = [None, at(0.0, -150.0), at(0.0, -50.0)];
        for measure in [PLAIN, SHORTEST] {
            assert_eq!(lock_on(Vec2::ZERO, 0.0, &field, measure), Some(1));
            assert_eq!(lock_on(Vec2::ZERO, 0.0, &[None, None], measure), None);
        }
    }

    #[test]
    fn the_box_takes_199_pixels_on_each_axis_and_not_200() {
        for (field, locks) in [
            (at(0.0, -199.0), true),
            (at(0.0, -199.9), true),
            (at(0.0, -200.0), false),
            (at(30.0, -199.0), true),
        ] {
            assert_eq!(
                lock_on(Vec2::ZERO, 0.0, &[field], PLAIN),
                locks.then_some(0),
                "{field:?}"
            );
        }
        // Sideways, heading right (90).
        for (x, locks) in [(199.0, true), (200.0, false)] {
            assert_eq!(
                lock_on(Vec2::ZERO, 90.0, &[at(x, 0.0)], PLAIN),
                locks.then_some(0),
                "{x}"
            );
        }
        // A shot heading down (180) at an asteroid 199 below.
        assert_eq!(
            lock_on(Vec2::ZERO, 180.0, &[at(0.0, 199.0)], PLAIN),
            Some(0)
        );
    }

    #[test]
    fn the_cone_takes_15_degrees_either_side_and_not_16() {
        // The asteroid's bearing from the shot is 0; the shot's heading
        // runs either side of it.
        let field = [at(0.0, -150.0)];
        for (heading, locks) in [
            (15.0, true),
            (15.9, true),
            (16.0, false),
            (345.0, false),
            (344.0, false),
        ] {
            assert_eq!(
                lock_on(Vec2::ZERO, heading, &field, PLAIN),
                locks.then_some(0),
                "plain {heading}"
            );
        }
        for (heading, locks) in [(345.0, true), (344.0, false)] {
            assert_eq!(
                lock_on(Vec2::ZERO, heading, &field, SHORTEST),
                locks.then_some(0),
                "shortest {heading}"
            );
        }
    }

    #[test]
    fn across_north_the_plain_measure_never_locks_and_the_short_way_does() {
        // A bearing just right of north, the heading 10 left of it across
        // north: 350 off plainly, 10 the short way.
        let field = [at(20.0, -150.0)];
        let bearing = trig::intercept_angle(Vec2::ZERO, Vec2::new(20.0, -150.0));
        assert!((1..10).contains(&bearing), "{bearing}");
        let heading = f32::from(bearing) + 350.0;
        assert_eq!(lock_on(Vec2::ZERO, heading, &field, PLAIN), None);
        assert_eq!(lock_on(Vec2::ZERO, heading, &field, SHORTEST), Some(0));
    }

    #[test]
    fn the_box_is_measured_from_the_shot_wherever_it_is() {
        let shot = Vec2::new(1000.0, 1000.0);
        for (y, locks) in [(850.0, true), (800.0, false)] {
            assert_eq!(
                lock_on(shot, 0.0, &[at(1000.0, y)], PLAIN),
                locks.then_some(0),
                "{y}"
            );
        }
    }

    #[test]
    fn an_asteroid_outside_the_cone_is_passed_over_for_a_later_one_inside() {
        let field = [at(150.0, 0.0), at(0.0, -150.0)];
        assert_eq!(lock_on(Vec2::ZERO, 0.0, &field, PLAIN), Some(1));
    }

    #[test]
    fn a_locked_shot_turns_its_whole_turn_while_the_bearing_is_further_off_than_it() {
        // 10 off, turn 5: on by 5.
        assert_eq!(turned(0.0, 10, 5.0), 5.0);
        // 10 off the other way: back by 5.
        assert_eq!(turned(20.0, 10, 5.0), 15.0);
        // 3 off, turn 2.5: 3 > 2.5, so it turns, where jam 0 compares 3
        // against the truncated 2 and turns too; 2 off against 2.5 holds.
        assert_eq!(turned(0.0, 3, 2.5), 2.5);
        assert_eq!(turned(0.0, 2, 2.5), 0.0);
        // Exactly the turn holds.
        assert_eq!(turned(0.0, 5, 5.0), 0.0);
    }

    #[test]
    fn its_direction_is_the_truncated_float_difference_taken_into_0_to_359() {
        // Bearing 181 from a heading of 0.5: trunc(180.5) is 180, half
        // round, so on; jam 0's 181 − 0 would turn back.
        assert_eq!(turned(0.5, 181, 10.0), 10.5);
        // Bearing 182: trunc(181.5) is 181, back.
        assert_eq!(turned(0.5, 182, 10.0), -9.5);
        // Bearing 0 from 350: trunc(−350) + 360 is 10, on.
        assert_eq!(turned(350.0, 0, 5.0), 355.0);
        // Bearing 350 from 0: 350, back.
        assert_eq!(turned(0.0, 350, 5.0), -5.0);
        // A difference of 360 or more is taken round once.
        assert_eq!(turned(-10.0, 355, 2.0), -8.0);
        // 549 less 360 is 189, back.
        assert_eq!(turned(-190.0, 359, 2.0), -192.0);
    }

    #[test]
    fn a_negative_turn_turns_even_onto_its_bearing() {
        // 0 off is above a turn of −2, so it turns on by −2.
        assert_eq!(turned(10.0, 10, -2.0), 8.0);
    }
}
