//! `wëap`: weapons.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The wëap
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 522), which sums to the 134 bytes of every stock record. Some
//! fields change meaning for beams; the bytes are the same either way.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::Flags16;
use crate::wire::id::{WeaponId, id};
use crate::wire::raw::RawArray;

/// A weapon. The resource name is the weapon name.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Weapon {
    /// Bible `Reload` (offset 0x00, i16): frames between shots.
    pub reload: i16,
    /// Bible `Count` (offset 0x02, i16): shot lifetime in frames.
    pub count: i16,
    /// Bible `MassDmg` (offset 0x04, i16).
    pub mass_dmg: i16,
    /// Bible `EnergyDmg` (offset 0x06, i16).
    pub energy_dmg: i16,
    /// Bible `Guidance` (offset 0x08, i16): -1 unguided, 0 beam, 1 homing,
    /// ... 99 carried ship.
    pub guidance: i16,
    /// Bible `Speed` (offset 0x0A, i16): pixels per frame x100.
    pub speed: i16,
    /// Bible `AmmoType` (offset 0x0C, i16): encoded: -1 unlimited, 0-255
    /// weapon index, -999 self-destruct, -1000 and below fuel; for carried
    /// ships the `shïp` ID.
    pub ammo_type: i16,
    /// Bible `Graphic` (offset 0x0E, i16): graphic index (`spïn` 3000+).
    pub graphic: i16,
    /// Bible `Inaccuracy` (offset 0x10, i16): degrees.
    pub inaccuracy: i16,
    /// Bible `Sound` (offset 0x12, i16): sound index 0-63 (`snd ` 200+), or
    /// -1 for silent.
    pub sound: i16,
    /// Bible `Impact` (offset 0x14, i16): negative makes a beam a tractor.
    pub impact: i16,
    /// Bible `ExplodType` (offset 0x16, i16): encoded explosion type: 0-63,
    /// +1000 for extra small explosions, -1 for none.
    pub explod_type: i16,
    /// Bible `ProxRadius` (offset 0x18, i16).
    pub prox_radius: i16,
    /// Bible `BlastRadius` (offset 0x1A, i16).
    pub blast_radius: i16,
    /// Bible `Flags` (offset 0x1C, u16).
    pub flags: Flags16,
    /// Bible `Seeker` (offset 0x1E, u16): guidance flags.
    pub seeker: Flags16,
    /// Bible `SmokeSet` (offset 0x20, i16): `cicn` smoke set index.
    pub smoke_set: i16,
    /// Bible `Decay` (offset 0x22, i16).
    pub decay: i16,
    /// Bible `Particles` (offset 0x24, i16): particles per frame.
    pub particles: i16,
    /// Bible `PartVel` (offset 0x26, i16).
    pub part_vel: i16,
    /// Bible `PartLifeMin` (offset 0x28, i16).
    pub part_life_min: i16,
    /// Bible `PartLifeMax` (offset 0x2A, i16).
    pub part_life_max: i16,
    /// Bible `PartColor` (offset 0x2C, u32): `00RRGGBB`.
    pub part_color: u32,
    /// Bible `BeamLength` (offset 0x30, i16).
    pub beam_length: i16,
    /// Bible `BeamWidth` (offset 0x32, i16): beam radius, or frame delay for
    /// spinning sprites.
    pub beam_width: i16,
    /// Bible `Falloff` (offset 0x34, i16): corona falloff, 2-16.
    pub falloff: i16,
    /// Bible `BeamColor` (offset 0x36, u32): `00RRGGBB`.
    pub beam_color: u32,
    /// Bible `CoronaColor` (offset 0x3A, u32): `00RRGGBB`.
    pub corona_color: u32,
    /// Bible `SubCount` (offset 0x3E, i16): submunitions; 0 or -1 if unused.
    pub sub_count: i16,
    /// Bible `SubType` (offset 0x40, i16): submunition `wëap` ID.
    #[br(map = id::<WeaponId>)]
    pub sub_type: Option<WeaponId>,
    /// Bible `SubTheta` (offset 0x42, i16): degrees; negative for a
    /// starburst.
    pub sub_theta: i16,
    /// Bible `SubLimit` (offset 0x44, i16): recursion limit.
    pub sub_limit: i16,
    /// Bible `ProxSafety` (offset 0x46, i16): fuse delay in 30ths of a
    /// second.
    pub prox_safety: i16,
    /// Bible `Flags2` (offset 0x48, u16).
    pub flags2: Flags16,
    /// Bible `Ionization` (offset 0x4A, i16).
    pub ionization: i16,
    /// Bible `HitParticles` (offset 0x4C, i16).
    pub hit_particles: i16,
    /// Bible `HitPartLife` (offset 0x4E, i16).
    pub hit_part_life: i16,
    /// Bible `HitPartVel` (offset 0x50, i16).
    pub hit_part_vel: i16,
    /// Bible `HitPartColor` (offset 0x52, u32): `00RRGGBB`.
    pub hit_part_color: u32,
    /// Bible `Recoil` (offset 0x56, i16).
    pub recoil: i16,
    /// Bible `ExitType` (offset 0x58, i16): -1 centre, 0 gun, 1 turret, 2
    /// guided, 3 beam exit points.
    pub exit_type: i16,
    /// Bible `BurstCount` (offset 0x5A, i16).
    pub burst_count: i16,
    /// Bible `BurstReload` (offset 0x5C, i16).
    pub burst_reload: i16,
    /// Bible `JamVuln1-4` (offset 0x5E, 4 x i16): percent per jamming type.
    pub jam_vuln: [i16; 4],
    /// Bible `Flags3` (offset 0x66, u16).
    pub flags3: Flags16,
    /// Bible `Durability` (offset 0x68, i16): point-defence hits survived.
    pub durability: i16,
    /// Bible `GuidedTurn` (offset 0x6A, i16).
    pub guided_turn: i16,
    /// Bible `MaxAmmo` (offset 0x6C, i16).
    pub max_ammo: i16,
    /// Bible `LiDensity` (offset 0x6E, i16): lightning zig-zags per 100
    /// pixels; 0 for a straight beam.
    pub li_density: i16,
    /// Bible `LiAmplitude` (offset 0x70, i16).
    pub li_amplitude: i16,
    /// Bible `IonizeColor` (offset 0x72, u32): `00RRGGBB`.
    pub ionize_color: u32,
    /// Offset 0x76, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x76: RawArray<16>,
}

impl Record for Weapon {
    const TYPE: ResType = ResType::new([b'w', 0x91, b'a', b'p']);
    const SIZE: Option<usize> = Some(134);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn flight_and_particle_fields_sit_at_their_bible_offsets() {
        let weap: Weapon = buf::<Weapon>()
            .i16(0x00, 30)
            .i16(0x02, 90)
            .i16(0x04, 10)
            .i16(0x06, 20)
            .i16(0x08, 1)
            .i16(0x0A, 1500)
            .i16(0x0C, -1005)
            .i16(0x0E, 7)
            .i16(0x10, -2)
            .i16(0x12, 11)
            .i16(0x14, 30)
            .i16(0x16, 1003)
            .i16(0x18, 15)
            .i16(0x1A, 25)
            .u16(0x1C, 0x8020)
            .u16(0x1E, 0x4001)
            .i16(0x20, 2)
            .i16(0x22, 6)
            .i16(0x24, 3)
            .i16(0x26, 120)
            .i16(0x28, 5)
            .i16(0x2A, 25)
            .u32(0x2C, 0x00FF_8000)
            .decode();
        let ints = [weap.reload, weap.count, weap.mass_dmg, weap.energy_dmg];
        assert_eq!(ints, [30, 90, 10, 20]);
        let ints = [weap.guidance, weap.speed, weap.ammo_type, weap.graphic];
        assert_eq!(ints, [1, 1500, -1005, 7]);
        let ints = [weap.inaccuracy, weap.sound, weap.impact, weap.explod_type];
        assert_eq!(ints, [-2, 11, 30, 1003]);
        assert_eq!((weap.prox_radius, weap.blast_radius), (15, 25));
        assert_eq!(
            (weap.flags, weap.seeker),
            (Flags16(0x8020), Flags16(0x4001))
        );
        let ints = [weap.smoke_set, weap.decay, weap.particles, weap.part_vel];
        assert_eq!(ints, [2, 6, 3, 120]);
        assert_eq!((weap.part_life_min, weap.part_life_max), (5, 25));
        assert_eq!(weap.part_color, 0x00FF_8000);
    }

    #[test]
    fn beam_submunition_and_later_fields_sit_at_their_bible_offsets() {
        let weap: Weapon = buf::<Weapon>()
            .i16(0x30, 300)
            .i16(0x32, 2)
            .i16(0x34, 8)
            .u32(0x36, 0x0000_FF00)
            .u32(0x3A, 0x0000_00FF)
            .i16(0x3E, 6)
            .i16(0x40, 131)
            .i16(0x42, -30)
            .i16(0x44, 2)
            .i16(0x46, 10)
            .u16(0x48, 0x1001)
            .i16(0x4A, 100)
            .i16(0x4C, 12)
            .i16(0x4E, 20)
            .i16(0x50, 200)
            .u32(0x52, 0x0012_3456)
            .i16(0x56, -50)
            .i16(0x58, 2)
            .i16(0x5A, 4)
            .i16(0x5C, 90)
            .i16(0x5E, 40)
            .u16(0x66, 0x0021)
            .i16(0x68, 3)
            .i16(0x6A, 60)
            .i16(0x6C, 40)
            .i16(0x6E, 8)
            .i16(0x70, 15)
            .u32(0x72, 0x0000_80FF)
            .bytes(0x76, &[0x11; 16])
            .decode();
        assert_eq!(
            [weap.beam_length, weap.beam_width, weap.falloff],
            [300, 2, 8]
        );
        assert_eq!(
            (weap.beam_color, weap.corona_color),
            (0x0000_FF00, 0x0000_00FF)
        );
        assert_eq!(weap.sub_count, 6);
        assert_eq!(weap.sub_type, Some(WeaponId(131)));
        assert_eq!(
            [weap.sub_theta, weap.sub_limit, weap.prox_safety],
            [-30, 2, 10]
        );
        assert_eq!(weap.flags2, Flags16(0x1001));
        let hits = [
            weap.ionization,
            weap.hit_particles,
            weap.hit_part_life,
            weap.hit_part_vel,
        ];
        assert_eq!(hits, [100, 12, 20, 200]);
        assert_eq!(weap.hit_part_color, 0x0012_3456);
        let ints = [
            weap.recoil,
            weap.exit_type,
            weap.burst_count,
            weap.burst_reload,
        ];
        assert_eq!(ints, [-50, 2, 4, 90]);
        assert_eq!(weap.jam_vuln, [40, 0, 0, 0]);
        assert_eq!(weap.flags3, Flags16(0x0021));
        assert_eq!(
            [weap.durability, weap.guided_turn, weap.max_ammo],
            [3, 60, 40]
        );
        assert_eq!((weap.li_density, weap.li_amplitude), (8, 15));
        assert_eq!(weap.ionize_color, 0x0000_80FF);
        assert_eq!(weap.unknown_0x76, RawArray([0x11; 16]));
    }

    #[test]
    fn no_submunition_is_none() {
        let weap: Weapon = buf::<Weapon>().i16(0x40, -1).decode();
        assert_eq!(weap.sub_type, None);
    }
}
