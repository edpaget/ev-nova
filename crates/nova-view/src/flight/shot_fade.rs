//! How a shot fades, as `_HandleShot` sets its sprite each live tick
//! (@0x359ac-0x35c07): out over its last 32 ticks by its `Falloff`, in
//! over its first by a negative one, and a translucent shot (`Flags3`
//! 0x0002) added to what is under it.
//!
//! Each tick first zeroes the sprite's translucency (+0xa8) and channel
//! levels (+0xaa-+0xae, @0x359ac-0x359d8). F is the weapon's shot fade
//! `Falloff` ([`WeaponLook::shot_falloff`]) and f the shot's fade (+0x24),
//! 0 at its spawn (@0x3e81c). A shot is on its n-th tick when drawn at
//! age n ([`Shot::age`](nova_sim::combat::projectile::Shot::age), the
//! ticks it has flown), and its life is then `Count` − n, counted down
//! before the fade (@0x35696-0x356a3).
//!
//! - **Fade-in, F < 0** (@0x359f0-0x35aa4), while f < 32: a plain shot is
//!   translucent by 32 − f, then f rises by |F|; a translucent one is
//!   additive at level f, and by the engine f never steps (the jump
//!   @0x35a59 skips the step @0x35a72-0x35aa4), so it stays at level 0,
//!   unseen, all its life. Once f is 32 or more the shot is opaque.
//! - **Fade-out, F > 0, life below 32** (@0x35aa9-0x35ba4): a plain shot
//!   is translucent by min(f, 31); a translucent one additive at 32 − f,
//!   at least 4 (@0x35b11-0x35b24); then f rises by F while below 32. On
//!   the k-th tick of the window, from 0, f = F × min(k, ⌈32/F⌉).
//! - **No `Falloff`** (@0x35bad-0x35c07): a translucent shot is additive
//!   at 32, the full `AddOver`; a plain one opaque.
//!
//! So a translucent shot with a `Falloff` is drawn opaque outside its
//! fade, as nothing sets +0xa8 there: the engine's reading of the
//! rulebook's [`RuleKey::TranslucentShotFade`](nova_sim::RuleKey::TranslucentShotFade).
//! By the other reading it is additive at 32 there, and its fade-in steps
//! as a plain shot's does.
//!
//! The blit (`_BlitPixieRLETranslucentDrawProc` @0xbb3fc,
//! `_BlitPixieTranslucentCopy` @0xc1110) draws additive level n as
//! `(src × n) >> 5 | dst`, [`Blend::Or`](crate::draw::Blend::Or) at
//! [`lights_tint`](crate::draw::lights_tint)`(n)`, and translucency t as
//! `(src × (32 − t)) >> 5 | (dst × t) >> 5`, a crossfade by OR. The view
//! draws the crossfade as a normal blend at (32 − t)/32, as the cloak's
//! translucency is drawn: rdm:phase/weapons-fidelity/phase-35-left-out-of-phase-24
//! has the renderer blend. No stock weapon takes the plain fade.
//!
//! Left out: the 8-bit depth, where a translucent shot takes the plain
//! path (@0x35a21, @0x35ae5, @0x35bd3): the engine draws at 16-bit only.

use nova_sim::RuleSource;

use super::catalog::WeaponLook;
use super::weapons::TRANSLUCENT;

/// The steps of a fade: 32nds.
pub const FADE_STEPS: i32 = 32;

/// The least an additive shot fades out to (@0x35b11-0x35b24).
pub const FADE_OUT_FLOOR: i32 = 4;

/// The most a plain shot fading out is translucent (@0x35b3d-0x35b57).
pub const FADE_OUT_MOST: i32 = 31;

/// How a shot is drawn on a tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotShade {
    /// As it is.
    Opaque,
    /// Translucent by `t` 32nds: drawn at alpha (32 − t)/32.
    Translucent(u8),
    /// Added to what is under it at level `n` 32nds: drawn
    /// [`Blend::Or`](crate::draw::Blend::Or) at
    /// [`lights_tint`](crate::draw::lights_tint)`(n)`.
    Additive(u8),
}

/// How a shot of a weapon that looks as `look` says is drawn at `age`
/// (see the module docs), by `rule`'s reading of
/// [`RuleKey::TranslucentShotFade`](nova_sim::RuleKey::TranslucentShotFade).
#[must_use]
pub fn shot_shade(look: &WeaponLook, age: u32, rule: RuleSource) -> ShotShade {
    let additive = look.flags3 & TRANSLUCENT != 0;
    let mended = rule == RuleSource::Bible;
    // Outside its fade, a translucent shot with a `Falloff` is left
    // opaque by the engine.
    let rest = if additive && (mended || look.shot_falloff == 0) {
        ShotShade::Additive(level(FADE_STEPS))
    } else {
        ShotShade::Opaque
    };
    let falloff = i64::from(look.shot_falloff);
    let tick = i64::from(age.max(1));
    if falloff < 0 {
        let fade = if additive && !mended {
            0
        } else {
            -falloff * (tick - 1)
        };
        if fade >= i64::from(FADE_STEPS) {
            return rest;
        }
        // Below 32: within an `i32`.
        let fade = fade as i32;
        return if additive {
            ShotShade::Additive(level(fade))
        } else {
            ShotShade::Translucent(level(FADE_STEPS - fade))
        };
    }
    let life = i64::from(look.count) - tick;
    if falloff == 0 || life >= i64::from(FADE_STEPS) {
        return rest;
    }
    // The window's first tick, from 1: life first below 32.
    let first = (i64::from(look.count) - i64::from(FADE_STEPS - 1)).max(1);
    // The original stops f at the first step at or past 32, F × ⌈32/F⌉;
    // past 32 every level is the same (4 added, 31 translucent), so f
    // runs on here. The window is under 32 ticks long, so f stays below
    // 32 × 32767: within an `i32`.
    let into = tick - first;
    let fade = i32::try_from(falloff * into).unwrap_or(i32::MAX);
    if additive {
        ShotShade::Additive(level((FADE_STEPS - fade).max(FADE_OUT_FLOOR)))
    } else {
        ShotShade::Translucent(level(fade.min(FADE_OUT_MOST)))
    }
}

/// `steps` 32nds as a level, 0 to 32.
fn level(steps: i32) -> u8 {
    u8::try_from(steps.clamp(0, FADE_STEPS)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENGINE: RuleSource = RuleSource::Engine;
    const MENDED: RuleSource = RuleSource::Bible;

    fn look(count: i16, falloff: i16, flags3: u16) -> WeaponLook {
        WeaponLook {
            count,
            shot_falloff: falloff,
            flags3,
            ..WeaponLook::default()
        }
    }

    fn shades(
        look: &WeaponLook,
        ages: std::ops::RangeInclusive<u32>,
        rule: RuleSource,
    ) -> Vec<ShotShade> {
        ages.map(|age| shot_shade(look, age, rule)).collect()
    }

    use ShotShade::{Additive, Opaque, Translucent};

    #[test]
    fn a_plain_shot_of_no_falloff_is_opaque() {
        for rule in [ENGINE, MENDED] {
            assert_eq!(shades(&look(50, 0, 0), 1..=50, rule), [Opaque; 50]);
        }
    }

    #[test]
    fn a_translucent_shot_of_no_falloff_is_additive_at_32() {
        for rule in [ENGINE, MENDED] {
            assert_eq!(
                shades(&look(50, 0, TRANSLUCENT), 1..=50, rule),
                [Additive(32); 50]
            );
        }
    }

    #[test]
    fn an_additive_shot_fades_out_by_its_falloff_to_a_floor_of_4() {
        // Count 40, F 2: the window opens on tick 9, life 31.
        let fading = look(40, 2, TRANSLUCENT);
        let window: Vec<ShotShade> = (9..=40)
            .map(|age| shot_shade(&fading, age, ENGINE))
            .collect();
        let mut expected: Vec<ShotShade> = (0..14).map(|k| Additive(32 - 2 * k)).collect();
        expected.extend([Additive(4); 18]);
        assert_eq!(window, expected, "32, 30, 28 … 6, then 4");
        assert_eq!(
            shades(&look(40, 13, TRANSLUCENT), 9..=12, ENGINE),
            [Additive(32), Additive(19), Additive(6), Additive(4)],
            "f stops at 39, the first step at or past 32"
        );
    }

    #[test]
    fn a_plain_shot_fades_out_by_its_falloff_capped_at_31() {
        let fading = look(40, 3, 0);
        assert_eq!(shades(&fading, 1..=8, ENGINE), [Opaque; 8]);
        let window = shades(&fading, 9..=40, ENGINE);
        let mut expected: Vec<ShotShade> = (0..11).map(|k| Translucent(3 * k)).collect();
        expected.extend([Translucent(31); 21]);
        assert_eq!(window, expected, "0, 3, 6 … 30, then 33 capped at 31");
        assert_eq!(
            shades(&fading, 9..=12, MENDED),
            [
                Translucent(0),
                Translucent(3),
                Translucent(6),
                Translucent(9)
            ],
            "the same by either reading"
        );
    }

    #[test]
    fn a_shot_of_less_than_32_ticks_fades_from_its_first() {
        assert_eq!(
            shades(&look(5, 4, 0), 1..=5, ENGINE),
            [
                Translucent(0),
                Translucent(4),
                Translucent(8),
                Translucent(12),
                Translucent(16)
            ]
        );
        assert_eq!(
            shot_shade(&look(32, 4, 0), 1, ENGINE),
            Translucent(0),
            "life 31 on its first tick"
        );
        assert_eq!(shot_shade(&look(33, 4, 0), 1, ENGINE), Opaque, "life 32");
        assert_eq!(shot_shade(&look(33, 4, 0), 2, ENGINE), Translucent(0));
    }

    #[test]
    fn a_plain_shot_fades_in_by_its_falloff_then_is_opaque() {
        let rising = look(100, -5, 0);
        assert_eq!(
            shades(&rising, 1..=9, ENGINE),
            [
                Translucent(32),
                Translucent(27),
                Translucent(22),
                Translucent(17),
                Translucent(12),
                Translucent(7),
                Translucent(2),
                Opaque,
                Opaque
            ]
        );
        assert_eq!(
            shades(&rising, 1..=9, MENDED),
            shades(&rising, 1..=9, ENGINE)
        );
    }

    #[test]
    fn an_additive_shots_fade_in_stays_at_0_by_the_engine_and_steps_by_the_other_reading() {
        let rising = look(100, -10, TRANSLUCENT);
        assert_eq!(shades(&rising, 1..=100, ENGINE), [Additive(0); 100]);
        assert_eq!(
            shades(&rising, 1..=6, MENDED),
            [
                Additive(0),
                Additive(10),
                Additive(20),
                Additive(30),
                Additive(32),
                Additive(32)
            ]
        );
    }

    #[test]
    fn an_additive_shot_with_a_falloff_is_opaque_before_its_window_by_the_engine_only() {
        let fading = look(110, 2, TRANSLUCENT);
        assert_eq!(shades(&fading, 1..=78, ENGINE), [Opaque; 78]);
        assert_eq!(shades(&fading, 1..=78, MENDED), [Additive(32); 78]);
        assert_eq!(shot_shade(&fading, 79, ENGINE), Additive(32), "the window");
    }

    #[test]
    fn a_stock_shaped_156_fades_on_its_last_32_ticks() {
        // `wëap` 156: Count 110, Falloff 2, `Flags3` 0x0002.
        let shot = look(110, 2, TRANSLUCENT);
        let at = |age| shot_shade(&shot, age, ENGINE);
        assert_eq!(at(70), Opaque);
        assert_eq!(at(78), Opaque, "life 32");
        assert_eq!(at(79), Additive(32), "life 31: the window's first tick");
        assert_eq!(at(80), Additive(30));
        assert_eq!(at(95), Additive(4), "32 − 32, at the floor");
        assert_eq!(at(110), Additive(4));
    }

    #[test]
    fn an_age_of_0_reads_as_the_first_tick() {
        assert_eq!(
            shot_shade(&look(100, -5, 0), 0, ENGINE),
            shot_shade(&look(100, -5, 0), 1, ENGINE)
        );
    }
}
