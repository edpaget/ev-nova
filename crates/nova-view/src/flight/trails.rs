//! A shot's particle trail, as `_HandleShot` leaves it (@0x35d49-0x35eab):
//! every live tick of a shot whose weapon has `Particles` above none, one
//! burst at its tail.
//!
//! - The burst goes off at the shot's position moved
//!   `_Accel(trunc(180.0 + heading) mod 360, trunc(width / 2))` (180.0
//!   @0xdd688; the sprite's width, `_GetSpriteXSize` @0x35d92), truncated
//!   to whole pixels ([`trail_start`]), through the table trig
//!   (`nova_sim::trig::accel`).
//! - Its speed is one of the weapon's eight trail speeds and its colour
//!   one of its eight trail colours, each picked by its own `Rand(8)`; it
//!   has no variation and no spread, lives `PartLifeMin` to `PartLifeMax`
//!   ticks and throws `Particles` pieces (@0x35dff-0x35ea6).
//! - The tables ([`TrailTable`]) are drawn once a weapon
//!   (`_LoadObjectData` @0x79a7d-0x79b82): for each of eight, a factor
//!   `(Rand(81) + 60) × 0.01`, the speed `factor × PartVel × 0.01` and the
//!   colour each channel of `PartColor` × the factor. The original's
//!   16-bit channels wrap above 1.0 for a bright channel; the view clamps
//!   each at 255. The view draws the tables the first time it sees the
//!   weapon's shot, on its own chance.
//! - The trail's alpha argument (32 less the sprite's translucency) is
//!   never read by the drawing (see [`effects`](super::effects)): every
//!   particle is opaque, then fades over its last 32 ticks.
//!
//! - The Smoke Trails preference off (the original's inverted
//!   `SmokeTrails` byte set) skips the trail (@0x35d64-0x35d6d): the
//!   flight view's `smoke_trails` flag, which its shot looks' step reads.
//!
//! Left out: the frozen frame (@0x35d73), which the engine never has.

use std::collections::BTreeMap;

use nova_sim::combat::ParticleBurst;
use nova_sim::{Chance, Vec2};

use super::catalog::{WeaponId, WeaponLook};

/// How many speeds and colours a weapon's trail picks from.
pub const TRAIL_PICKS: usize = 8;

/// The turn from a shot's heading to its tail, in degrees (@0xdd688).
pub const TAIL_TURN: f32 = 180.0;

/// A trail table's factor: `(Rand(TRAIL_FACTOR_SPREAD) + TRAIL_FACTOR_BASE)
/// × 0.01` (@0x79a7d-0x79b82).
pub const TRAIL_FACTOR_SPREAD: u32 = 81;
/// See [`TRAIL_FACTOR_SPREAD`].
pub const TRAIL_FACTOR_BASE: u32 = 60;

/// What a factor, and `PartVel`, are multiplied by (the double 0.01).
pub const HUNDREDTH: f64 = 0.01;

/// A weapon's trail speeds and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrailTable {
    /// The speeds, in pixels a tick.
    pub speeds: [f32; TRAIL_PICKS],
    /// The colours, `00RRGGBB`.
    pub colours: [u32; TRAIL_PICKS],
}

impl TrailTable {
    /// The table of a weapon of `PartVel` `part_vel` and `PartColor`
    /// `part_color`, its factors drawn on `chance`.
    pub fn draw(part_vel: i16, part_color: u32, chance: &mut dyn Chance) -> Self {
        let mut speeds = [0.0; TRAIL_PICKS];
        let mut colours = [0; TRAIL_PICKS];
        for (speed, colour) in speeds.iter_mut().zip(&mut colours) {
            let factor =
                f64::from(chance.below(TRAIL_FACTOR_SPREAD) + TRAIL_FACTOR_BASE) * HUNDREDTH;
            // A hundredth of an `i16`, scaled under 1.41: within an `f32`.
            #[allow(clippy::cast_possible_truncation)]
            {
                *speed = (factor * f64::from(part_vel) * HUNDREDTH) as f32;
            }
            *colour = scaled_colour(part_color, factor);
        }
        Self { speeds, colours }
    }
}

/// `00RRGGBB` `raw` with each channel times `factor`, truncated and
/// clamped at 255.
#[must_use]
pub fn scaled_colour(raw: u32, factor: f64) -> u32 {
    [16, 8, 0].into_iter().fold(0, |colour, shift| {
        let channel = f64::from((raw >> shift) & 0xFF);
        // Clamped to 0..=255 before the cast.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let scaled = (channel * factor).trunc().clamp(0.0, 255.0) as u32;
        colour | scaled << shift
    })
}

/// Where a shot at `position` heading `heading` leaves its trail: half
/// its sprite's `width` behind it, whole pixels (see the module docs).
#[must_use]
pub fn trail_start(position: Vec2, heading: f32, width: u32) -> Vec2 {
    // `cvttss2si`, then `% 360` on the whole degree (C's sign-keeping
    // remainder).
    #[allow(clippy::cast_possible_truncation)]
    let tail = ((TAIL_TURN + heading) as i32 % 360) as i16;
    // Half an `rlëD`'s width: well within an `f32`.
    #[allow(clippy::cast_precision_loss)]
    let back = nova_sim::trig::accel(tail, (width / 2) as f32);
    Vec2::new((position.x + back.x).trunc(), (position.y + back.y).trunc())
}

/// The weapons' trail tables, each drawn the first time it is needed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrailTables {
    tables: BTreeMap<WeaponId, TrailTable>,
}

impl TrailTables {
    /// The burst a shot of `weapon`, which looks as `look` says, at
    /// `position` heading `heading` leaves this tick, its table drawn on
    /// `chance` if it is new, and its picks drawn on `chance`; none for a
    /// weapon of no `Particles`.
    pub fn burst(
        &mut self,
        weapon: WeaponId,
        look: &WeaponLook,
        (position, heading): (Vec2, f32),
        chance: &mut dyn Chance,
    ) -> Option<ParticleBurst> {
        let count = u32::try_from(look.particles).ok().filter(|&n| n > 0)?;
        let table = *self
            .tables
            .entry(weapon)
            .or_insert_with(|| TrailTable::draw(look.part_vel, look.part_color, chance));
        let width = match &look.sheet {
            Some(Ok(sheet)) => sheet.frame_width,
            _ => 0,
        };
        let pick = |chance: &mut dyn Chance| {
            usize::try_from(chance.below(TRAIL_PICKS as u32)).unwrap_or(0) % TRAIL_PICKS
        };
        let speed = table.speeds[pick(chance)];
        let color = table.colours[pick(chance)];
        let life = |ticks: i16| u32::try_from(ticks).unwrap_or(0);
        Some(ParticleBurst {
            at: trail_start(position, heading, width),
            speed,
            variation: 0,
            life: (life(look.part_life.0), life(look.part_life.1)),
            color,
            count,
            spread: 0.0,
        })
    }

    /// Weapon `weapon`'s table, if it has been drawn.
    #[must_use]
    pub fn table(&self, weapon: WeaponId) -> Option<&TrailTable> {
        self.tables.get(&weapon)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::VecDeque;
    use std::num::NonZeroU16;

    use super::*;
    use crate::flight::catalog::EffectSheet;

    /// Gives its draws in turn, recording what it was asked.
    struct Draws {
        draws: VecDeque<u32>,
        asked: Vec<u32>,
    }

    impl Draws {
        fn of(draws: &[u32]) -> Self {
            Self {
                draws: draws.iter().copied().collect(),
                asked: Vec::new(),
            }
        }
    }

    impl Chance for Draws {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked.push(n);
            self.draws.pop_front().unwrap_or(0)
        }
    }

    /// A trailing weapon on a sheet `width` wide: 2 particles of 10-24
    /// ticks at `PartVel` 300 in 0x1040ff.
    fn trailing(width: u32) -> WeaponLook {
        WeaponLook {
            sheet: Some(Ok(EffectSheet {
                image: crate::SheetImage::Rled(3500),
                frames: NonZeroU16::MIN,
                frame_width: width,
            })),
            particles: 2,
            part_vel: 300,
            part_life: (10, 24),
            part_color: 0x0010_40FF,
            ..WeaponLook::default()
        }
    }

    #[test]
    fn a_table_draws_eight_factors_of_60_to_140_hundredths() {
        let mut chance = Draws::of(&[0, 80, 40, 0, 0, 0, 0, 0]);
        let table = TrailTable::draw(300, 0x0010_40FF, &mut chance);
        assert_eq!(chance.asked, [81; 8]);
        assert_eq!(table.speeds[0], (0.6_f64 * 300.0 * 0.01) as f32, "0.6 × 3");
        assert_eq!(table.speeds[1], (1.4_f64 * 300.0 * 0.01) as f32);
        assert_eq!(table.speeds[2], (1.0_f64 * 300.0 * 0.01) as f32);
        assert_eq!(
            table.colours[0], 0x0009_2699,
            "each channel × 0.6, truncated"
        );
        assert_eq!(table.colours[2], 0x0010_40FF, "× 1.0");
        assert_eq!(table.colours[1], 0x0016_59FF, "× 1.4, blue clamped at 255");
    }

    #[test]
    fn a_colour_channel_clamps_at_255() {
        assert_eq!(scaled_colour(0x00FF_FFFF, 1.4), 0x00FF_FFFF);
        assert_eq!(scaled_colour(0x00B6_0000, 1.4), 0x00FE_0000);
        assert_eq!(scaled_colour(0x00B7_0000, 1.4), 0x00FF_0000);
        assert_eq!(scaled_colour(0x0000_0000, 1.4), 0);
    }

    #[test]
    fn a_trail_starts_half_its_sprite_behind_the_shot_in_whole_pixels() {
        let at = Vec2::new(100.5, -40.25);
        assert_eq!(trail_start(at, 0.0, 10), Vec2::new(100.0, -35.0), "below");
        assert_eq!(trail_start(at, 90.0, 10), Vec2::new(95.0, -40.0), "left");
        assert_eq!(
            trail_start(at, 90.0, 11),
            Vec2::new(95.0, -40.0),
            "5 of 5.5"
        );
        assert_eq!(trail_start(at, 270.0, 0), Vec2::new(100.0, -40.0));
        assert_eq!(
            trail_start(Vec2::ZERO, 359.9, 20),
            trail_start(Vec2::ZERO, 359.0, 20),
            "the whole degree"
        );
    }

    #[test]
    fn a_shot_leaves_a_burst_at_its_tail_from_its_weapons_tables() {
        let mut tables = TrailTables::default();
        // The table's eight factors, then the speed's pick and the
        // colour's.
        let mut chance = Draws::of(&[0, 80, 40, 0, 0, 0, 0, 0, 1, 2]);
        let burst = tables
            .burst(
                WeaponId(156),
                &trailing(10),
                (Vec2::new(0.0, 0.0), 90.0),
                &mut chance,
            )
            .expect("a trail");
        assert_eq!(chance.asked, [[81; 8].as_slice(), &[8, 8]].concat());
        let table = *tables.table(WeaponId(156)).expect("drawn");
        assert_eq!(
            burst,
            ParticleBurst {
                at: Vec2::new(-5.0, 0.0),
                speed: table.speeds[1],
                variation: 0,
                life: (10, 24),
                color: table.colours[2],
                count: 2,
                spread: 0.0,
            }
        );
        let mut again = Draws::of(&[7, 0]);
        let next = tables
            .burst(
                WeaponId(156),
                &trailing(10),
                (Vec2::new(0.0, 0.0), 0.0),
                &mut again,
            )
            .expect("a trail");
        assert_eq!(again.asked, [8, 8], "the table drawn once");
        assert_eq!(
            (next.speed, next.color),
            (table.speeds[7], table.colours[0])
        );
        assert_eq!(next.at, Vec2::new(0.0, 5.0));
    }

    #[test]
    fn a_weapon_of_no_particles_leaves_no_trail() {
        let mut tables = TrailTables::default();
        let mut chance = Draws::of(&[]);
        let none = WeaponLook {
            particles: 0,
            ..trailing(10)
        };
        let shot = (Vec2::ZERO, 0.0);
        assert_eq!(tables.burst(WeaponId(128), &none, shot, &mut chance), None);
        let negative = WeaponLook {
            particles: -1,
            ..trailing(10)
        };
        assert_eq!(
            tables.burst(WeaponId(128), &negative, shot, &mut chance),
            None
        );
        assert!(chance.asked.is_empty());
        assert_eq!(tables.table(WeaponId(128)), None);
    }

    #[test]
    fn a_negative_life_is_none() {
        let mut tables = TrailTables::default();
        let mut chance = Draws::of(&[]);
        let look = WeaponLook {
            part_life: (-5, 3),
            sheet: None,
            ..trailing(10)
        };
        let burst = tables
            .burst(
                WeaponId(128),
                &look,
                (Vec2::new(10.0, 10.0), 0.0),
                &mut chance,
            )
            .expect("a trail");
        assert_eq!(burst.life, (0, 3));
        assert_eq!(burst.at, Vec2::new(10.0, 10.0), "no sheet: no width");
    }
}
