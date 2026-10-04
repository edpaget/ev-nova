//! What the simulation does with each documented weapon flag: the Bible's
//! `wëap` `Flags`, `Seeker`, `Flags2` and `Flags3` bits, each with the
//! guidance it applies to and its [`Status`].
//!
//! A bit applies to every weapon unless the Bible limits it: to guided
//! weapons (Guidance 1: `Flags` 0x0008 and 0x0080, `Flags2` 0x0008 and
//! every `Seeker` bit), to turrets (3, 4 and 7-10: the blind spots,
//! `Flags` 0x1000-0x4000) or to beams (0, 3 and 10: `Flags2` 0x2000). A
//! weapon a bit does not apply to is [`Status::NotApplicable`] for it.
//!
//! [`unimplemented`] lists the bits a weapon sets that apply to it and
//! that the simulation does not do yet, so a session can report each once
//! while the weapon fires with them ignored. Undocumented bits are never
//! reported.

use std::fmt;

use super::weapon::{Guidance, WeaponSpec};

/// Which of a `wëap`'s flag words a bit is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FlagField {
    /// `Flags`.
    Flags,
    /// `Seeker`.
    Seeker,
    /// `Flags2`.
    Flags2,
    /// `Flags3`.
    Flags3,
}

impl fmt::Display for FlagField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Flags => "Flags",
            Self::Seeker => "Seeker",
            Self::Flags2 => "Flags2",
            Self::Flags3 => "Flags3",
        })
    }
}

/// What the simulation does with a flag bit, for a weapon it applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// The simulation does what it says.
    Implemented,
    /// Only the view uses it (graphics and sound), from phase 3 on.
    Presentation,
    /// It does not apply to the weapon's guidance.
    NotApplicable,
    /// Not done yet: reported, and ignored.
    Unimplemented,
}

/// Which weapons a bit applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    /// Every weapon.
    Any,
    /// Guided weapons (Guidance 1).
    Guided,
    /// Turrets (Guidance 3, 4 and 7-10).
    Turret,
    /// Beams (Guidance 0, 3 and 10).
    Beam,
}

impl Scope {
    fn covers(self, guidance: Guidance) -> bool {
        match self {
            Self::Any => true,
            Self::Guided => guidance == Guidance::Other(1),
            Self::Turret => matches!(guidance, Guidance::Other(3 | 4 | 7..=10)),
            Self::Beam => matches!(guidance, Guidance::Beam | Guidance::Other(3 | 10)),
        }
    }
}

use FlagField::{Flags, Flags2, Flags3, Seeker};
use Scope::{Any, Beam, Guided, Turret};
use Status::{Implemented, Presentation, Unimplemented};

/// Every documented bit: its field, the bit, which weapons it applies to,
/// and what the simulation does with it.
const TABLE: [(FlagField, u16, Scope, Status); 44] = [
    // Spin the graphic.
    (Flags, 0x0001, Any, Presentation),
    // Fired by the second trigger.
    (Flags, 0x0002, Any, Implemented),
    // Cycling graphic starts on its first frame.
    (Flags, 0x0004, Any, Presentation),
    // Guided: won't fire at fast ships.
    (Flags, 0x0008, Guided, Unimplemented),
    // Looped sound.
    (Flags, 0x0010, Any, Presentation),
    // Passes through shields.
    (Flags, 0x0020, Any, Implemented),
    // Copies fire simultaneously.
    (Flags, 0x0040, Any, Unimplemented),
    // Homing: point defence can't target it.
    (Flags, 0x0080, Guided, Unimplemented),
    // Blast doesn't hurt the player.
    (Flags, 0x0100, Any, Implemented),
    // Small smoke, big smoke, persistent smoke.
    (Flags, 0x0200, Any, Presentation),
    (Flags, 0x0400, Any, Presentation),
    (Flags, 0x0800, Any, Presentation),
    // Turret blind spots: front, sides, rear.
    (Flags, 0x1000, Turret, Unimplemented),
    (Flags, 0x2000, Turret, Unimplemented),
    (Flags, 0x4000, Turret, Unimplemented),
    // Detonates at the end of its life.
    (Flags, 0x8000, Any, Implemented),
    // Guided weapons' behaviour.
    (Seeker, 0x0001, Guided, Unimplemented),
    (Seeker, 0x0002, Guided, Unimplemented),
    (Seeker, 0x0008, Guided, Unimplemented),
    (Seeker, 0x0010, Guided, Unimplemented),
    (Seeker, 0x0020, Guided, Unimplemented),
    (Seeker, 0x4000, Guided, Unimplemented),
    (Seeker, 0x8000, Guided, Unimplemented),
    // Cycling graphic: held on the first frame, stopped on the last.
    (Flags2, 0x0001, Any, Presentation),
    (Flags2, 0x0002, Any, Presentation),
    // Proximity detonator ignores asteroids.
    (Flags2, 0x0004, Any, Unimplemented),
    // Guided: proximity detonator set off by other ships.
    (Flags2, 0x0008, Guided, Unimplemented),
    // Submunitions aimed at the nearest target; none on expiry.
    (Flags2, 0x0010, Any, Unimplemented),
    (Flags2, 0x0020, Any, Unimplemented),
    // Ammo hidden on the status display.
    (Flags2, 0x0040, Any, Presentation),
    // Needs a KeyCarried ship aboard.
    (Flags2, 0x0080, Any, Unimplemented),
    // AI ships won't use it.
    (Flags2, 0x0100, Any, Unimplemented),
    // Uses the ship's weapon sprite.
    (Flags2, 0x0200, Any, Presentation),
    // Planet-type weapon.
    (Flags2, 0x0400, Any, Unimplemented),
    // Not selectable or shown out of ammo.
    (Flags2, 0x0800, Any, Presentation),
    // Disables but never destroys.
    (Flags2, 0x1000, Any, Unimplemented),
    // Beam drawn under ships.
    (Flags2, 0x2000, Beam, Presentation),
    // Fires while cloaked.
    (Flags2, 0x4000, Any, Unimplemented),
    // x10 mass damage to asteroids.
    (Flags2, 0x8000, Any, Unimplemented),
    // Ammo only at the end of a burst.
    (Flags3, 0x0001, Any, Implemented),
    // Translucent shots.
    (Flags3, 0x0002, Any, Presentation),
    // One shot at a time.
    (Flags3, 0x0004, Any, Unimplemented),
    // Fires from the exit point nearest the target.
    (Flags3, 0x0010, Any, Unimplemented),
    // Exclusive.
    (Flags3, 0x0020, Any, Unimplemented),
];

/// What the simulation does with `field`'s `bit` on a weapon of
/// `guidance`; `None` for a bit the Bible does not document.
#[must_use]
pub fn status(field: FlagField, bit: u16, guidance: Guidance) -> Option<Status> {
    TABLE
        .iter()
        .find(|&&(f, b, _, _)| f == field && b == bit)
        .map(|&(_, _, scope, status)| {
            if scope.covers(guidance) {
                status
            } else {
                Status::NotApplicable
            }
        })
}

/// The bits `spec` sets that apply to it and are not done yet, in the
/// table's order.
#[must_use]
pub fn unimplemented(spec: &WeaponSpec) -> Vec<(FlagField, u16)> {
    TABLE
        .iter()
        .filter(|&&(field, bit, scope, status)| {
            status == Unimplemented && scope.covers(spec.guidance) && word(spec, field) & bit != 0
        })
        .map(|&(field, bit, _, _)| (field, bit))
        .collect()
}

/// `spec`'s `field`.
fn word(spec: &WeaponSpec, field: FlagField) -> u16 {
    match field {
        Flags => spec.flags,
        Seeker => spec.seeker,
        Flags2 => spec.flags2,
        Flags3 => spec.flags3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::testkit::weapon;

    const NA: Status = Status::NotApplicable;

    /// Every documented bit's status on an unguided weapon, a beam, a
    /// guided weapon and a turret, in that order.
    const EXPECTED: [(FlagField, u16, [Status; 4]); 44] = [
        (Flags, 0x0001, [Presentation; 4]),
        (Flags, 0x0002, [Implemented; 4]),
        (Flags, 0x0004, [Presentation; 4]),
        (Flags, 0x0008, [NA, NA, Unimplemented, NA]),
        (Flags, 0x0010, [Presentation; 4]),
        (Flags, 0x0020, [Implemented; 4]),
        (Flags, 0x0040, [Unimplemented; 4]),
        (Flags, 0x0080, [NA, NA, Unimplemented, NA]),
        (Flags, 0x0100, [Implemented; 4]),
        (Flags, 0x0200, [Presentation; 4]),
        (Flags, 0x0400, [Presentation; 4]),
        (Flags, 0x0800, [Presentation; 4]),
        (Flags, 0x1000, [NA, NA, NA, Unimplemented]),
        (Flags, 0x2000, [NA, NA, NA, Unimplemented]),
        (Flags, 0x4000, [NA, NA, NA, Unimplemented]),
        (Flags, 0x8000, [Implemented; 4]),
        (Seeker, 0x0001, [NA, NA, Unimplemented, NA]),
        (Seeker, 0x0002, [NA, NA, Unimplemented, NA]),
        (Seeker, 0x0008, [NA, NA, Unimplemented, NA]),
        (Seeker, 0x0010, [NA, NA, Unimplemented, NA]),
        (Seeker, 0x0020, [NA, NA, Unimplemented, NA]),
        (Seeker, 0x4000, [NA, NA, Unimplemented, NA]),
        (Seeker, 0x8000, [NA, NA, Unimplemented, NA]),
        (Flags2, 0x0001, [Presentation; 4]),
        (Flags2, 0x0002, [Presentation; 4]),
        (Flags2, 0x0004, [Unimplemented; 4]),
        (Flags2, 0x0008, [NA, NA, Unimplemented, NA]),
        (Flags2, 0x0010, [Unimplemented; 4]),
        (Flags2, 0x0020, [Unimplemented; 4]),
        (Flags2, 0x0040, [Presentation; 4]),
        (Flags2, 0x0080, [Unimplemented; 4]),
        (Flags2, 0x0100, [Unimplemented; 4]),
        (Flags2, 0x0200, [Presentation; 4]),
        (Flags2, 0x0400, [Unimplemented; 4]),
        (Flags2, 0x0800, [Presentation; 4]),
        (Flags2, 0x1000, [Unimplemented; 4]),
        (Flags2, 0x2000, [NA, Presentation, NA, NA]),
        (Flags2, 0x4000, [Unimplemented; 4]),
        (Flags2, 0x8000, [Unimplemented; 4]),
        (Flags3, 0x0001, [Implemented; 4]),
        (Flags3, 0x0002, [Presentation; 4]),
        (Flags3, 0x0004, [Unimplemented; 4]),
        (Flags3, 0x0010, [Unimplemented; 4]),
        (Flags3, 0x0020, [Unimplemented; 4]),
    ];

    const KINDS: [Guidance; 4] = [
        Guidance::Unguided,
        Guidance::Beam,
        Guidance::Other(1),
        Guidance::Other(4),
    ];

    #[test]
    fn every_documented_bit_has_its_status() {
        for (field, bit, statuses) in EXPECTED {
            for (guidance, expected) in KINDS.into_iter().zip(statuses) {
                assert_eq!(
                    status(field, bit, guidance),
                    Some(expected),
                    "{field} {bit:#06x} on {guidance:?}"
                );
            }
        }
    }

    #[test]
    fn every_guidance_in_scope_treats_a_bit_alike() {
        for (field, bit, statuses) in EXPECTED {
            for guidance in [Guidance::FreefallBomb, Guidance::Rocket] {
                assert_eq!(
                    status(field, bit, guidance),
                    Some(statuses[0]),
                    "{field} {bit:#x}"
                );
            }
        }
    }

    #[test]
    fn each_turret_and_beam_guidance_is_one() {
        for raw in [3, 4, 7, 8, 9, 10] {
            let turret = Guidance::Other(raw);
            assert_eq!(status(Flags, 0x1000, turret), Some(Unimplemented), "{raw}");
        }
        for raw in [2, 5, 6, 11, 99] {
            let other = Guidance::decode(raw);
            assert_eq!(status(Flags, 0x1000, other), Some(NA), "{raw}");
        }
        for beam in [Guidance::Beam, Guidance::Other(3), Guidance::Other(10)] {
            assert_eq!(status(Flags2, 0x2000, beam), Some(Presentation), "{beam:?}");
        }
        for not_beam in [Guidance::Other(4), Guidance::Other(9), Guidance::Rocket] {
            assert_eq!(status(Flags2, 0x2000, not_beam), Some(NA), "{not_beam:?}");
        }
        assert_eq!(status(Seeker, 0x0001, Guidance::Other(2)), Some(NA));
    }

    #[test]
    fn an_undocumented_bit_has_no_status() {
        for (field, bit) in [
            (Seeker, 0x0004),
            (Seeker, 0x0100),
            (Flags3, 0x0008),
            (Flags3, 0x8000),
            (Flags, 0x0000),
        ] {
            assert_eq!(
                status(field, bit, Guidance::Unguided),
                None,
                "{field} {bit:#x}"
            );
        }
    }

    fn spec(guidance: i16, flags: u16, flags2: u16, flags3: u16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance,
            flags,
            flags2,
            flags3,
            ..weapon(128)
        })
    }

    #[test]
    fn stock_shaped_weapons_report_only_the_asteroid_damage_bit() {
        // The Light Blaster, Medium Blaster and Light Cannon: blast spares
        // the player, blind spots that do not apply, translucent shots.
        assert_eq!(unimplemented(&spec(-1, 0x6100, 0, 0x0002)), []);
        assert_eq!(unimplemented(&spec(-1, 0x0100, 0, 0x0002)), []);
        // The Mining Blaster: secondary, and x10 against asteroids.
        assert_eq!(
            unimplemented(&spec(-1, 0x0102, 0x8000, 0x0002)),
            [(Flags2, 0x8000)]
        );
        // The Raven Rocket and the Stellar Grenade.
        assert_eq!(unimplemented(&spec(6, 0x8002, 0, 0)), []);
        assert_eq!(unimplemented(&spec(6, 0x8003, 0, 0)), []);
        // The Polaron Cannon, the Pulse Laser and the BioRelay Laser.
        assert_eq!(unimplemented(&spec(0, 0x0012, 0, 0x0001)), []);
        assert_eq!(unimplemented(&spec(0, 0x4010, 0x2200, 0)), []);
        assert_eq!(unimplemented(&spec(0, 0x6010, 0, 0)), []);
        // A Wraith Graviton Beam, and the Winter Tempest.
        assert_eq!(
            unimplemented(&spec(0, 0x0010, 0x8200, 0)),
            [(Flags2, 0x8000)]
        );
        assert_eq!(unimplemented(&spec(0, 0x0032, 0x0200, 0)), []);
    }

    #[test]
    fn every_unimplemented_bit_set_is_listed_in_the_tables_order() {
        assert_eq!(
            unimplemented(&spec(-1, 0x0048, 0x1080, 0x0024)),
            [
                (Flags, 0x0040),
                (Flags2, 0x0080),
                (Flags2, 0x1000),
                (Flags3, 0x0004),
                (Flags3, 0x0020),
            ],
            "the guided-only 0x0008 left out"
        );
        assert_eq!(unimplemented(&spec(-1, 0, 0, 0)), []);
        let seeking = WeaponSpec::new(&WeaponRecord {
            guidance: 1,
            seeker: 0x8001,
            ..weapon(131)
        });
        assert_eq!(
            unimplemented(&seeking),
            [(Seeker, 0x0001), (Seeker, 0x8000)]
        );
    }

    #[test]
    fn a_turrets_blind_spots_are_phase_4s() {
        assert_eq!(unimplemented(&spec(-1, 0x7000, 0, 0)), []);
        assert_eq!(
            unimplemented(&spec(4, 0x7000, 0, 0)),
            [(Flags, 0x1000), (Flags, 0x2000), (Flags, 0x4000)]
        );
    }

    #[test]
    fn each_field_is_named_as_the_bible_names_it() {
        assert_eq!(
            [Flags, Seeker, Flags2, Flags3].map(|field| field.to_string()),
            ["Flags", "Seeker", "Flags2", "Flags3"]
        );
    }
}
