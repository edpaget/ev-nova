//! An NPC's skill and its government's `SkillMult`: the two factors on
//! its top speed and thrust beside its class's own (`_ShipMaxSpeed`
//! @0x37c5-0x37ee, `_ShipAccelRate` @0x361e-0x363a), and the `shïp`
//! `Flags` bit that holds a class still.
//!
//! - **The skill** ([`random_skill_level`], `_RandomSkillLevel`
//!   @0x6922-0x6984) is rolled as the ship spawns and kept on it (ship
//!   +0x40): `(Rand(2·SkillVar + 1) − SkillVar + 100) × 0.01`, worked in
//!   doubles and kept as a float, `SkillVar` the class's (+0x9f8). The
//!   original cuts `2·SkillVar + 1` to 16 bits (@0x6948-0x694c), so a
//!   `SkillVar` of 16384 or more wraps; the engine counts the sides wide.
//!   `_Rand` (@0xa4c76) takes a signed count, and a negative one gives
//!   `n..=0`, so a negative `SkillVar` v gives a skill in
//!   `[(101 − |v|)/100, (100 + |v|)/100]`. A `SkillVar` over 100 can roll a
//!   negative skill, which every reader floors at none.
//! - **The government's factor** ([`skill_mult_factor`], runtime gövt
//!   +0x64, `_LoadObjectData` @0x7bb69-0x7bb9b): `SkillMult` × 0.01, or
//!   1.0 when `SkillMult` is 0 or less.
//! - **A still hull** ([`STILL_HULL`], class +0x9e9 bit 0x04): an NPC of
//!   such a class has no top speed and no thrust, returned before every
//!   other factor (@0x37b5-0x37bf, @0x3601-0x360b). Its turn is
//!   untouched (`_ShipTurnRate` @0x32f7 reads no such bit).

use crate::chance::Chance;

/// `shïp` `Flags` 0x0400: an NPC of the class neither moves nor thrusts
/// (`testb $4, class+0x9e9`, `_ShipMaxSpeed` @0x37b5, `_ShipAccelRate`
/// @0x3601). Unlike an immobile class (a raw `Speed` and `Accel` of 0),
/// it keeps its `Speed` on record, which the setup speed reads.
pub const STILL_HULL: u16 = 0x0400;

/// The double `_RandomSkillLevel` and `_LoadObjectData` scale by (0.01
/// @0xdd098).
const PERCENT: f64 = 0.01;

/// A skill rolled for a class of `SkillVar` `skill_var` on `chance`, as
/// `_RandomSkillLevel` rolls it (see the module docs): `SkillVar` 0 rolls
/// nothing, its one outcome 1.0.
#[must_use]
pub fn random_skill_level(skill_var: i16, chance: &mut (impl Chance + ?Sized)) -> f32 {
    if skill_var == 0 {
        return 1.0;
    }
    let var = i32::from(skill_var);
    let sides = 2 * var + 1;
    // `_Rand(n)` for `n > 0` is `0..n`; for `n < 0` it is `n..=0`, |n| + 1
    // outcomes. The widest count, a `SkillVar` of -32768's 65536, loses
    // its top outcome to the roll's 16 bits.
    let drawn = if sides > 0 {
        i32::from(chance.roll(u16::try_from(sides).unwrap_or(u16::MAX)))
    } else {
        let outcomes = u16::try_from(1 - sides).unwrap_or(u16::MAX);
        sides + i32::from(chance.roll(outcomes))
    };
    (f64::from(drawn - var + 100) * PERCENT) as f32
}

/// The factor a government of `SkillMult` `raw` puts on its ships' top
/// speed and thrust (see the module docs).
#[must_use]
pub fn skill_mult_factor(raw: i16) -> f32 {
    if raw > 0 {
        (f64::from(raw) * PERCENT) as f32
    } else {
        1.0
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::fixture::Draws;

    /// The skill `SkillVar` `var` rolls on `draw`, and the sides asked.
    fn rolled(var: i16, draw: u32) -> (f32, Vec<u32>) {
        let mut chance = Draws::of(&[draw]);
        let skill = random_skill_level(var, &mut chance);
        (skill, chance.asked)
    }

    /// `percent` as the original keeps it: × 0.01 in doubles, as a float.
    fn kept(percent: i32) -> f32 {
        (f64::from(percent) * 0.01) as f32
    }

    #[test]
    fn skill_var_10_rolls_21_sides_from_0_9_to_1_1() {
        assert_eq!(rolled(10, 0), (kept(90), vec![21]));
        assert_eq!(rolled(10, 20), (kept(110), vec![21]));
        assert_eq!(rolled(10, 10), (1.0, vec![21]));
    }

    #[test]
    fn skill_var_0_is_1_and_rolls_nothing() {
        assert_eq!(rolled(0, 0), (1.0, vec![]));
    }

    #[test]
    fn skill_var_100_rolls_from_none_to_double() {
        assert_eq!(rolled(100, 0), (0.0, vec![201]));
        assert_eq!(rolled(100, 200), (2.0, vec![201]));
    }

    #[test]
    fn skill_var_over_100_can_roll_a_negative_skill() {
        assert_eq!(rolled(150, 0).0, kept(-50));
    }

    #[test]
    fn a_negative_skill_var_rolls_from_101_less_its_size_to_100_more() {
        // `_Rand(-9)` gives -9..=0: ten outcomes.
        assert_eq!(rolled(-5, 0), (kept(96), vec![10]));
        assert_eq!(rolled(-5, 9), (kept(105), vec![10]));
        assert_eq!(rolled(-1, 0), (1.0, vec![2]));
        assert_eq!(rolled(-1, 1), (kept(101), vec![2]));
    }

    #[test]
    fn a_wide_skill_var_counts_its_sides_without_wrapping() {
        assert_eq!(rolled(16384, 0), (kept(100 - 16384), vec![32769]));
        assert_eq!(rolled(i16::MAX, 65534), (kept(100 + 32767), vec![65535]));
        assert_eq!(rolled(i16::MIN, 0).1, vec![65535]);
        assert_eq!(rolled(i16::MIN, 0).0, kept(101 - 32768));
    }

    #[test]
    fn skill_mult_is_a_hundredth_and_1_at_0_or_less() {
        assert_eq!(skill_mult_factor(150), 1.5);
        assert_eq!(skill_mult_factor(1), (0.01_f64) as f32);
        assert_eq!(skill_mult_factor(250), 2.5);
        assert_eq!(skill_mult_factor(0), 1.0);
        assert_eq!(skill_mult_factor(-5), 1.0);
    }

    #[test]
    fn a_still_hull_is_flags_0x0400() {
        assert_eq!(STILL_HULL, 0x0400);
    }
}
