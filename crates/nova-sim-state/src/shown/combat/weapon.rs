//! How a weapon's shots fly, [`Guidance`], and its `Falloff` as the
//! original loads it ([`loaded_falloff`]), from
//! `nova_sim_combat::combat::weapon`, which re-exports them and keeps the
//! rest of a weapon's decoding (`WeaponSpec`).

/// How a weapon's shots fly, from its `Guidance`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guidance {
    /// -1: an unguided projectile, at constant velocity.
    Unguided,
    /// 0: a beam.
    Beam,
    /// 1: a homing shot, steering at its target.
    Homing,
    /// 3: a turreted beam, held on its target.
    TurretBeam,
    /// 4: a turret, firing at its target at the lead angle.
    Turret,
    /// 5: a freefall bomb, launched at 80% of the firer's velocity, that
    /// turns into the wind.
    FreefallBomb,
    /// 6: a freeflight rocket, launched straight ahead, that accelerates
    /// to its top speed.
    Rocket,
    /// 7: a front-quadrant turret: at a target within 45 degrees of the
    /// nose, otherwise straight ahead.
    FrontTurret,
    /// 8: a rear-quadrant turret: only at a target within 45 degrees of
    /// the tail.
    RearTurret,
    /// 9: a point-defence turret, firing at missiles.
    PointDefence,
    /// 10: a point-defence beam.
    PointDefenceBeam,
    /// 99 with a `shïp` ID for its `AmmoType`: a fighter bay, launching
    /// that ship (see `nova_sim_combat::bay`). [`Guidance::decode`] never
    /// gives it, as it needs the `AmmoType`; `nova_sim_combat::combat::weapon::WeaponSpec::new` does.
    FighterBay,
    /// Any other: 99 without a ship to carry, and anything undocumented.
    Other(i16),
}

impl Guidance {
    /// The guidance `raw` means, 99 as [`Guidance::Other`] (see
    /// [`Guidance::FighterBay`]).
    #[must_use]
    pub fn decode(raw: i16) -> Self {
        match raw {
            -1 => Self::Unguided,
            0 => Self::Beam,
            1 => Self::Homing,
            3 => Self::TurretBeam,
            4 => Self::Turret,
            5 => Self::FreefallBomb,
            6 => Self::Rocket,
            7 => Self::FrontTurret,
            8 => Self::RearTurret,
            9 => Self::PointDefence,
            10 => Self::PointDefenceBeam,
            other => Self::Other(other),
        }
    }
}

/// The corona falloff a beam of none or less loads with (`_LoadObjectData`
/// @0x795e2-0x795f0).
pub const DEFAULT_FALLOFF: i16 = 16;

/// A weapon's `Falloff` as `_LoadObjectData` loads it from the `wëap`'s
/// `falloff`, `beam_length` and `li_density` (@0x795e2-0x79645): a
/// lightning weapon (a `LiDensity` above none) has none at all, whatever
/// it says and whatever its `BeamLength` (@0x79625-0x79645); a beam (a
/// `BeamLength` above none) of a `Falloff` of none or less takes
/// [`DEFAULT_FALLOFF`] (@0x795e2-0x795f0). Any other weapon's is kept.
/// The view draws a beam's corona and a shot's fade from it too.
#[must_use]
pub fn loaded_falloff(beam_length: i16, falloff: i16, li_density: i16) -> i16 {
    if li_density > 0 {
        0
    } else if beam_length <= 0 {
        falloff
    } else if falloff <= 0 {
        DEFAULT_FALLOFF
    } else {
        falloff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guidance_decodes_at_each_value() {
        for (raw, guidance) in [
            (-1, Guidance::Unguided),
            (0, Guidance::Beam),
            (1, Guidance::Homing),
            (2, Guidance::Other(2)),
            (3, Guidance::TurretBeam),
            (4, Guidance::Turret),
            (5, Guidance::FreefallBomb),
            (6, Guidance::Rocket),
            (7, Guidance::FrontTurret),
            (8, Guidance::RearTurret),
            (9, Guidance::PointDefence),
            (10, Guidance::PointDefenceBeam),
            (11, Guidance::Other(11)),
            (99, Guidance::Other(99)),
            (-2, Guidance::Other(-2)),
        ] {
            assert_eq!(Guidance::decode(raw), guidance, "{raw}");
        }
    }
}
