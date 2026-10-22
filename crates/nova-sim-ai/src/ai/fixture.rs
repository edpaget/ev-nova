//! Ships and governments the AI types' tests share.

use crate::ai::fire::AmmoLeft;
use crate::ai::{Blessed, Blessing, PlayerSide, Surroundings};
use crate::catalog::{EscortClass, GovtId, GovtRecord, LandingSite, WeaponId, WeaponRecord};
use crate::combat::ShipRef;
use crate::combat::armament::{Armament, Arsenal, LauncherRounds};
use crate::combat::hull::{Condition, HullSpec};
use crate::combat::weapon::WeaponSpec;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::reserves::Reserves;
use crate::stats::ShipStats;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::FAST;
use nova_sim_catalog::fixture::{govt, planet, weapon};

/// Traders: class 1, `CrimeTol` 6.
pub(crate) const TRADERS: GovtId = GovtId(140);
/// Police allied with the traders.
pub(crate) const POLICE: GovtId = GovtId(141);
/// Pirates at war with the traders.
pub(crate) const PIRATES: GovtId = GovtId(142);
/// Nobody's friend or foe.
pub(crate) const NEUTRAL: GovtId = GovtId(143);

/// The four governments, the traders' with `flags` and the police's with
/// `police_flags`.
pub(crate) fn govts(flags: u16, police_flags: u16) -> Governments {
    Governments::new([
        GovtRecord {
            flags,
            crime_tol: 6,
            classes: [1, -1, -1, -1],
            ..govt(140)
        },
        GovtRecord {
            flags: police_flags,
            crime_tol: 6,
            allies: [1, -1, -1, -1],
            classes: [2, -1, -1, -1],
            ..govt(141)
        },
        GovtRecord {
            enemies: [1, -1, -1, -1],
            classes: [3, -1, -1, -1],
            ..govt(142)
        },
        govt(143),
    ])
}

/// NPC `id` of `govt` and `ai_type` at (`x`, `y`), at rest facing up, of
/// strength 100 and aggression 2, its reserves full (30 shield), armed
/// with a harmless gun of unlimited ammunition (so it is not out of
/// ammunition, and holds no destroying weapon).
pub(crate) fn ship(id: u32, govt: GovtId, ai_type: AiType, x: f32, y: f32) -> Npc {
    let mut npc = crate::traffic::fixture::npc(id, ShipStats::new(FAST, &[]));
    npc.armament = Armament::new([(WeaponSpec::new(&weapon(127)), 1)]);
    npc.govt = Some(govt);
    npc.ai_type = ai_type;
    npc.state.position = Vec2::new(x, y);
    npc.hull.strength = 100.0;
    npc.aggression = 2;
    npc
}

/// `npc` armed so it has `left` of its ammunition, its hull retreating
/// when out of it (`shïp` `Flags2` 0x0080) as `retreats` says: a
/// launcher (128, of `AmmoType` 10) holding a round ([`AmmoLeft::Armed`]),
/// or none; beside the harmless gun of unlimited ammunition (127) for
/// [`AmmoLeft::OnlyUnlimited`], alone for [`AmmoLeft::Dry`].
pub(crate) fn short_of_ammo(mut npc: Npc, left: AmmoLeft, retreats: bool) -> Npc {
    let launcher = WeaponSpec::new(&WeaponRecord {
        ammo_type: 10,
        ..weapon(128)
    });
    let gun = WeaponSpec::new(&weapon(127));
    npc.armament = match left {
        AmmoLeft::Dry => Armament::new([(launcher, 1)]),
        AmmoLeft::Armed | AmmoLeft::OnlyUnlimited => Armament::new([(gun, 1), (launcher, 1)]),
    };
    npc.rounds = match left {
        AmmoLeft::Armed => LauncherRounds::from([(WeaponId(128), 1)]),
        AmmoLeft::OnlyUnlimited | AmmoLeft::Dry => LauncherRounds::default(),
    };
    npc.hull.retreats_dry = retreats;
    npc
}

/// The player at (`x`, `y`), at rest, of strength 100.
pub(crate) fn player(x: f32, y: f32) -> PlayerSide {
    PlayerSide {
        state: ShipState {
            position: Vec2::new(x, y),
            ..ShipState::default()
        },
        condition: Condition::Intact,
        reserves: Reserves::full(100.0, 100.0, 100.0),
        hull: HullSpec {
            strength: 100.0,
            ..HullSpec::default()
        },
        handling: ShipStats::new(FAST, &[]).handling,
        rating: 0,
        yardstick: 0,
        untargetable: false,
        blessing: Blessing::NONE,
        ship: crate::catalog::ShipId(128),
        combat_govt: None,
        target: None,
        class: EscortClass::Freighter,
        boarded: false,
        speed: 0.0,
    }
}

/// A blessing where `govt` alone blesses the player with peace.
pub(crate) fn blessing_of(govt: GovtId) -> Blessing {
    let mut blessing = Blessing::NONE;
    blessing.bless(govt, Blessed::Peace);
    blessing
}

/// A landable planet, 128, far off: below and left of the centre, as
/// the 999 bound on a destination (see
/// [`destination`](crate::traffic::destination)) is an upper one only.
pub(crate) fn sites() -> Vec<LandingSite> {
    vec![planet(128, -3000.0, -3000.0)]
}

/// `npcs` among `sites`, with the player at (`x`, `y`), in a traders'
/// system where the player's record is `record`.
pub(crate) fn around<'a>(
    sites: &'a [LandingSite],
    npcs: &'a [Npc],
    govts: &'a Governments,
    (x, y): (f32, f32),
    record: i16,
) -> Surroundings<'a> {
    Surroundings {
        player: Some(player(x, y)),
        govts,
        system_govt: Some(TRADERS),
        record,
        ..Surroundings::new(sites, npcs)
    }
}

/// The global weapon records of the weapons `npcs` carry, as their mounts
/// hold them: what a lead reads ([`Surroundings::weapons`]).
pub(crate) fn records_of<'a>(npcs: impl IntoIterator<Item = &'a Npc>) -> Arsenal {
    Arsenal::of_specs(
        npcs.into_iter()
            .flat_map(|npc| npc.armament.mounts().iter().map(|mount| mount.spec)),
    )
}

/// NPC `id` as a ship.
pub(crate) fn n(id: u32) -> ShipRef {
    ShipRef::Npc(NpcId(id))
}

/// `npc` fully cloaked.
pub(crate) fn cloaked(mut npc: Npc) -> Npc {
    npc.cloak = crate::cloak::Cloak {
        fade: crate::cloak::FADE_FULL,
        direction: 0,
    };
    npc
}

/// `npc` with a cloak scanner on screen, seeing a cloaked ship within 200
/// of it on both axes.
pub(crate) fn scanning(mut npc: Npc) -> Npc {
    npc.cloak_gear.scanner.on_screen = true;
    npc
}
