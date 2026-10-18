//! Stand-off ships: a ship whose hull stands off (`shïp` `Flags2` 0x0002,
//! [`HullSpec::stands_off`](crate::combat::hull::HullSpec::stands_off))
//! fights from its weapons' reach rather than closing to dogfight.
//!
//! The rules are the original's (`_ShipMaxWeaponRange` @0xbafd-0xbbe2 and
//! the attack block of `_HighLevelAIHandler` state 4 @0x8e3d0-0x8e712 in
//! the `EV Nova` executable).
//!
//! - **Its longest reach** ([`max_weapon_range`], `_ShipMaxWeaponRange`):
//!   the largest reach of the weapons it carries (@0xbb12; an
//!   [`Armament`](crate::combat::armament::Armament) holds none it
//!   carries none of) that have their ammunition
//!   ([`has_ammo`](crate::ai::fire::has_ammo), @0xbb2a), by
//!   guidance: a beam or turreted beam its length (@0xbb58); an unguided
//!   gun, a turret or a front-quadrant turret its range, truncated
//!   (@0xbb86); a homing weapon [`STAND_OFF_SHARE`] of it, truncated
//!   (@0xbb8d); a rocket [`ROCKET_SHARE`] of it, truncated (@0xbb9c); any
//!   other nothing (@0xbb7c-0xbb82). The range read is
//!   [`WeaponSpec::range`], the reach down a weapon's sub-munition chain
//!   (the original's +0x5c). None with nothing counted.
//! - **Its band** ([`band`]): [`STAND_OFF_SHARE`] of its longest reach,
//!   truncated (@0x8e4a2-0x8e4bb), and half that, truncated, against a
//!   disabled target (`_IsDisabled` @0x8e4ca, @0x8e4d3-0x8e4e2).
//! - **Its manoeuvre** ([`keep`]), re-decided on every think while it
//!   attacks (state 4): inside [`DOGFIGHT_BOX`] on both axes it backs away
//!   (sub 5, @0x8e6b7-0x8e6c4, flown as [`Manoeuvre::Flee`]); beyond its
//!   band on either axis it approaches (sub 7, @0x8e527, @0x8e561); within
//!   it, it snipes (sub 0xe, @0x8e5a7-0x8e5ee). Both comparisons are
//!   strict, as the original's (`ja`).
//! - It never asks `_AIEvalHopelessChase`: the `Flags2` test (@0x8e48e)
//!   jumps past it (see [`warship`](crate::ai::warship)).
//! - Neither routine plays a sound.
//!
//! # Not modelled
//!
//! - `_ShipMaxWeaponRange`'s clamp to 0x7fff and the attack block's 16-bit
//!   band (@0xbbca-0xbbe0, @0x8e4d3): integer widths, not replicated.
//! - The band's back-away inside its own 165 re-test (@0x8e5f3), which is
//!   unreachable: the block is entered only from outside the box.

#[cfg(doc)]
use crate::ai::fire::DOGFIGHT_BOX;
use crate::ai::fire::{self, Manoeuvre};
use crate::combat::weapon::{Guidance, WeaponSpec};
use crate::geometry::Vec2;
use crate::traffic::npc::Npc;

/// The share of a homing weapon's range it reaches, and of its longest
/// reach a stand-off ship keeps within (0.85 @0xdd150).
pub const STAND_OFF_SHARE: f64 = 0.85;
/// The share of a rocket's range it reaches, and of its band a stand-off
/// ship keeps within against a disabled target (0.5 @0xdd128).
pub const ROCKET_SHARE: f64 = 0.5;

/// How far a weapon of `spec` reaches for [`max_weapon_range`], by its
/// guidance (see the module docs).
fn reach(spec: &WeaponSpec) -> f32 {
    let range = spec.range();
    match spec.guidance {
        Guidance::Beam | Guidance::TurretBeam => range,
        Guidance::Unguided | Guidance::Turret | Guidance::FrontTurret => range.trunc(),
        Guidance::Homing => share(range, STAND_OFF_SHARE),
        Guidance::Rocket => share(range, ROCKET_SHARE),
        _ => 0.0,
    }
}

/// `share` of `value`, truncated, reckoned in doubles as the original
/// does (`cvtss2sd`, `mulsd`, `cvttsd2si`).
#[allow(clippy::cast_possible_truncation)] // A truncated reach fits an f32.
fn share(value: f32, share: f64) -> f32 {
    (f64::from(value) * share).trunc() as f32
}

/// The longest reach of `npc`'s weapons, in pixels, as
/// `_ShipMaxWeaponRange` gives it (see the module docs).
#[must_use]
pub fn max_weapon_range(npc: &Npc) -> f32 {
    npc.armament
        .mounts()
        .iter()
        .filter(|mount| fire::has_ammo(npc, mount))
        .map(|mount| reach(&mount.spec))
        .fold(0.0, f32::max)
}

/// The distance, in pixels on each axis, `npc` keeps within while it
/// attacks a target, halved when `target_disabled`: none for a ship whose
/// hull doesn't stand off (see the module docs).
#[must_use]
pub fn band(npc: &Npc, target_disabled: bool) -> Option<f32> {
    if !npc.hull.stands_off {
        return None;
    }
    let band = share(max_weapon_range(npc), STAND_OFF_SHARE);
    Some(if target_disabled {
        share(band, ROCKET_SHARE)
    } else {
        band
    })
}

/// How a stand-off ship with `band` flies at a target `off` from it (see
/// the module docs).
#[must_use]
pub fn keep(off: Vec2, band: f32) -> Manoeuvre {
    let beyond = |reach: f32| off.x.abs() > reach || off.y.abs() > reach;
    if !beyond(fire::DOGFIGHT_BOX) {
        Manoeuvre::Flee
    } else if beyond(band) {
        Manoeuvre::Approach
    } else {
        Manoeuvre::Snipe
    }
}

#[cfg(test)]
// The reaches compared are whole numbers, exact in an f32.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{WeaponId, WeaponRecord};
    use crate::combat::armament::{Armament, LauncherRounds};
    use crate::stats::ShipStats;
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::weapon;

    /// A weapon of `id` and `guidance` whose range is `range`: 10 pixels a
    /// tick, for `range` / 10 ticks, its beam `range` long.
    fn of(id: i16, guidance: i16, range: f32) -> WeaponSpec {
        let mut spec = WeaponSpec::new(&WeaponRecord {
            guidance,
            speed: 1000,
            ..weapon(id)
        });
        spec.chain_range = range;
        spec
    }

    /// A ship carrying one of each of `weapons`, its hull standing off.
    fn carrying(weapons: impl IntoIterator<Item = WeaponSpec>) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
        npc.armament = Armament::new(weapons.into_iter().map(|spec| (spec, 1)));
        npc.hull.stands_off = true;
        npc
    }

    fn longest(spec: WeaponSpec) -> f32 {
        max_weapon_range(&carrying([spec]))
    }

    #[test]
    fn the_shares_are_the_originals() {
        assert_eq!([STAND_OFF_SHARE, ROCKET_SHARE], [0.85, 0.5]);
    }

    #[test]
    fn a_beam_reaches_its_length() {
        assert_eq!(longest(of(128, 0, 300.0)), 300.0);
        assert_eq!(longest(of(128, 3, 250.0)), 250.0, "a turreted beam");
    }

    #[test]
    fn a_gun_or_turret_reaches_its_range_truncated() {
        assert_eq!(longest(of(128, -1, 400.7)), 400.0, "unguided");
        assert_eq!(longest(of(128, 4, 350.9)), 350.0, "a turret");
        assert_eq!(longest(of(128, 7, 320.5)), 320.0, "a front-quadrant turret");
    }

    #[test]
    fn a_homing_weapon_reaches_085_of_its_range_truncated() {
        // 1000 x 0.85 = 850; 999 x 0.85 = 849.15.
        assert_eq!(longest(of(128, 1, 1000.0)), 850.0);
        assert_eq!(longest(of(128, 1, 999.0)), 849.0);
    }

    #[test]
    fn a_rocket_reaches_half_its_range_truncated() {
        assert_eq!(longest(of(128, 6, 801.0)), 400.0);
    }

    #[test]
    fn any_other_guidance_reaches_nothing() {
        for guidance in [5, 8, 9, 10, 99, 42] {
            assert_eq!(longest(of(128, guidance, 500.0)), 0.0, "{guidance}");
        }
        let mut bay = of(128, -1, 500.0);
        bay.guidance = Guidance::FighterBay;
        assert_eq!(longest(bay), 0.0, "a bay");
    }

    #[test]
    fn the_longest_reach_is_the_largest_of_its_weapons() {
        let npc = carrying([of(128, -1, 200.0), of(129, 0, 500.0), of(130, 1, 400.0)]);
        assert_eq!(max_weapon_range(&npc), 500.0);
        let npc = carrying([of(128, 6, 2000.0), of(129, -1, 300.0)]);
        assert_eq!(max_weapon_range(&npc), 1000.0);
    }

    #[test]
    fn no_weapons_reach_nothing() {
        assert_eq!(max_weapon_range(&carrying([])), 0.0);
    }

    #[test]
    fn a_weapon_without_its_ammunition_does_not_count() {
        let launcher = {
            let mut spec = WeaponSpec::new(&WeaponRecord {
                ammo_type: 10,
                guidance: 1,
                ..weapon(129)
            });
            spec.chain_range = 1000.0;
            spec
        };
        let mut npc = carrying([of(128, -1, 200.0), launcher]);
        assert_eq!(max_weapon_range(&npc), 200.0, "no round aboard");
        npc.rounds = LauncherRounds::from([(WeaponId(129), 1)]);
        assert_eq!(max_weapon_range(&npc), 850.0, "a round aboard");
    }

    #[test]
    fn the_reach_is_the_sub_munition_chains() {
        // Its own flight is 10 x 20 = 200 pixels; its chain reaches 700.
        let mut spec = WeaponSpec::new(&WeaponRecord {
            speed: 1000,
            count: 20,
            ..weapon(128)
        });
        assert_eq!(spec.flight(), 200.0);
        spec.chain_range = 700.0;
        assert_eq!(longest(spec), 700.0);
    }

    #[test]
    fn the_band_is_085_of_the_longest_reach_truncated() {
        let npc = carrying([of(128, 0, 999.0)]);
        // 999 x 0.85 = 849.15.
        assert_eq!(band(&npc, false), Some(849.0));
    }

    #[test]
    fn the_band_halves_truncated_against_a_disabled_target() {
        let npc = carrying([of(128, 0, 999.0)]);
        // 849 x 0.5 = 424.5.
        assert_eq!(band(&npc, true), Some(424.0));
    }

    #[test]
    fn a_hull_that_does_not_stand_off_has_no_band() {
        let mut npc = carrying([of(128, 0, 999.0)]);
        npc.hull.stands_off = false;
        assert_eq!(band(&npc, false), None);
        assert_eq!(band(&npc, true), None);
    }

    #[test]
    fn it_backs_away_inside_the_box() {
        assert_eq!(keep(Vec2::new(165.0, -165.0), 500.0), Manoeuvre::Flee);
        assert_eq!(keep(Vec2::new(0.0, 0.0), 500.0), Manoeuvre::Flee);
    }

    #[test]
    fn it_snipes_outside_the_box_within_its_band() {
        assert_eq!(keep(Vec2::new(165.1, 0.0), 500.0), Manoeuvre::Snipe);
        assert_eq!(keep(Vec2::new(0.0, -165.1), 500.0), Manoeuvre::Snipe);
        assert_eq!(
            keep(Vec2::new(-500.0, 500.0), 500.0),
            Manoeuvre::Snipe,
            "exactly its band is within it"
        );
    }

    #[test]
    fn it_approaches_beyond_its_band_on_either_axis() {
        assert_eq!(keep(Vec2::new(500.1, 0.0), 500.0), Manoeuvre::Approach);
        assert_eq!(keep(Vec2::new(0.0, -500.1), 500.0), Manoeuvre::Approach);
        assert_eq!(keep(Vec2::new(-500.1, 400.0), 500.0), Manoeuvre::Approach);
    }

    #[test]
    fn a_band_inside_the_box_approaches_to_the_box_and_backs_away_inside() {
        assert_eq!(keep(Vec2::new(166.0, 0.0), 100.0), Manoeuvre::Approach);
        assert_eq!(keep(Vec2::new(150.0, 0.0), 100.0), Manoeuvre::Flee);
    }
}
