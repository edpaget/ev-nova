//! The radar's outfits and colour codes: what the original's IFF
//! (`_ColorCodeShip`, `_ColorCodeStellar`), density scanner and
//! interference decide, in the core's terms. The view draws them (see
//! `nova-view`'s `flight::radar`).
//!
//! Addresses are in `EV Nova.app/Contents/MacOS/EV Nova` (i386).
//!
//! # The outfits ([`RadarGear`])
//!
//! `_HasIFF` (@0x4348), `_HasDensityScanner` (@0x42c0) and
//! `_ShipAntiInterference` (@0x5f2c) each walk the player's outfit counts
//! (`_shipSystems`) against the `oütf` records, look only at outfits with a
//! count above 0, and check all four `ModType` slots.
//!
//! - IFF is [`MOD_IFF`], `ModType` 14 (`cmpw $0xe`, @0x43a7); the density
//!   scanner [`MOD_DENSITY_SCANNER`], `ModType` 13 (`cmpw $0xd`,
//!   @0x431f). Both answer only for the player (slot +0x86 == 0,
//!   @0x42d4, @0x435c).
//! - Anti-interference is [`MOD_ANTI_INTERFERENCE`], `ModType` 24 (`cmpw
//!   $0x18`, @0x5f76): the sum of count × `ModVal` over every owned outfit
//!   and slot (@0x5f67-0x5f9b), clamped to −100..=100 (@0x5f9d-0x5fbd).
//!   Ours sums in `i32`: the original's 16-bit `imulw`/`addw` wraps are
//!   not replicated.
//! - The original caches all three (`_playerHasIFF`,
//!   `_playerHasDensityScanner`, `_playerAntiInterference`) lazily:
//!   `_ResetPlayerPrecalcedValues` stores a sentinel (@0xc3b6-0xc3d9), and
//!   the getters recount the outfits on their next call (@0x436c,
//!   @0x42e4, @0x5f3e). Every change to the player's outfits resets them
//!   before the next read. The outfitter does so through
//!   `_SystemInfoToShipStats` (buy @0x5c305, sell @0x5cffb, which
//!   tail-jumps to the reset @0xcae9) and then redraws the bar
//!   (`_DrawStatus` @0x5c5a1, @0x5d328); set expressions do so through
//!   the wrappers @0x99a85 and @0x99dea. So reading the outfits afresh
//!   each time, as ours does, matches.
//!
//! # Ships ([`ship_iff`], `_ColorCodeShip` @0x4ae8)
//!
//! Tested in order: the player's own slot (+0x86 == 0) is cyan
//! (@0x4afa-0x4b0f), which the view draws for the player pixel and no NPC
//! ever is; disabled (`_IsDisabled` @0x2ce6, whose armour test also holds
//! for a ship breaking up) dark grey (@0x4b14-0x4b25); a threat
//! (`_IsThreatToPlayer` @0x7f501, ours [`Npc::threatens_player_among`],
//! which counts a ship attacking one the player leads, @0x7f572-0x7f590)
//! red (@0x4b27-0x4b44); an escort green (@0x4b46-0x4b8f); anything else
//! blue (@0x4b91-0x4b9c). An escort is a ship whose parent (+0x9a) is the
//! player and that defends no stellar, or whose parent's parent is the
//! player (@0x4b80-0x4b8f): ours, a ship that
//! [escorts the player](Npc::escorts_player) (a hired or captured escort,
//! the player's own fighter, a mission escort), or one whose leader does
//! (an escort's fighter, which `Session::join_carrier` leads by the
//! escort and gives no escort duty).
//!
//! Left out: the escort test's defender case (@0x4b52-0x4b7c), a ship
//! led by the player that defends a stellar. Ours has no such ship.
//!
//! # Stellars ([`stellar_iff`], `_ColorCodeStellar` @0xaa26)
//!
//! 1. Not landable (`_StellarIsLandable` @0xa989, ours
//!    [`landing::is_landable`]) or uninhabited (`Flags` 0x20): medium
//!    grey (@0xaa3b-0xaa63).
//! 2. A hypergate (`Flags2` 0x1000) goes straight to the legal colours,
//!    skipping steps 3 and 4 (@0xaa68-0xaa6f).
//! 3. A wormhole (`Flags2` 0x2000): teal (@0xaa71-0xaa87).
//! 4. Dominated (the stellar's +0x46 byte): green (@0xaa8c-0xaa90,
//!    @0xab19-0xab24). The byte is set from the always-dominated bit
//!    (`Flags2` 0x0020, [`market::DOMINATED`]) and by demanding tribute.
//! 5. The legal colours (@0xaa96-0xab24): yellow when the stellar's
//!    government has the rank blessing that lets the player land
//!    (`gövt` +0x85); otherwise by the player's record in the system
//!    against `MinStatus`: with `MinStatus` 0x7FFF red below 0 and orange
//!    from 0, never yellow; else yellow when `MinStatus` ≤ the record or
//!    `MinStatus` ≤ −32767, red below 0 and orange from 0.
//!
//! The [`Standing`] the caller passes carries both. The session fills
//! the blessing from its rank blessing table (`Session::blessing`);
//! domination by tribute has no state yet (rdm:task/planet-domination).
//!
//! # Systems on the galaxy map ([`system_color_code`], `_ColorCodeSystem` @0xd672-0xd941)
//!
//! The colour the galaxy map frames a system's oval in (`_DrawMap`
//! @0xef8a-0xefad), worked out from the system's head (+0xba, or itself
//! when that is −1: an inlined `_FindFirstCoLocatedSystem`,
//! @0xd680-0xd6a9) and the system standing for it (an inlined
//! `_FindActiveCoLocatedSystem`, @0xd6ac-0xd70d):
//!
//! 1. `_rgbDarkGrey` ([`SystemColorCode::Unexplored`]) when no system
//!    stands for it, its head is invalid (@0xd712-0xd742) or its head is
//!    unexplored (+0x90 ≤ 0, @0xd75b-0xd764).
//! 2. `_rgbLightGrey` ([`SystemColorCode::Uninhabited`]) when the system
//!    standing for it isn't inhabited: an inlined `_SystemIsInhabited`,
//!    which reads only its first four `NavDefs` slots
//!    (@0xd776-0xd7b7, → @0xd915).
//! 3. Otherwise each of its 16 `NavDefs` stellars that is active (+0x44),
//!    not a hypergate or wormhole (`Flags2` & 0x3000), inhabited (no
//!    `Flags` 0x20) and landable (`_StellarIsLandable`)
//!    (@0xd7e6-0xd82c) is counted into one bucket: dominated (+0x46,
//!    @0xd82e); else clear when its government (0-0xff) blesses landing
//!    (`gövt` +0x85, @0xd83a-0xd862); else by `MinStatus` against the
//!    player's record in the head (`_playerRecord[head]`, @0xd864-0xd8a1),
//!    as `_ColorCodeStellar` reads it ([`stellar_iff`]'s step 5): with
//!    `MinStatus` 0x7FFF hostile below 0 and denied from 0; else clear
//!    when `MinStatus` ≤ the record or ≤ −32767; else hostile below 0 and
//!    denied from 0.
//! 4. The first bucket with a stellar wins, in this order
//!    (@0xd8b0-0xd915): dominated `_brightGreen`
//!    ([`SystemColorCode::Dominated`]), clear blue (0, 0, 0xffff,
//!    [`SystemColorCode::Cleared`]), denied orange (0xffff, 0x6666, 0,
//!    [`SystemColorCode::Denied`]), hostile red (0xffff, 0, 0,
//!    [`SystemColorCode::Hostile`]); with none, `_rgbLightGrey`.
//!
//! The caller works the inputs out ([`MapStellar`]): the session reads the
//! inhabited test and the record as its rules say (`Session`'s `map`
//! module). IDs above 0x7ff (@0xd6ac, @0xd718) don't arise in ours.
//!
//! # Interference ([`interference`], `_HandleStatus` @0x494d7)
//!
//! The system's `sÿst` `Interference` less the anti-interference, clamped
//! to 0..=100 (@0x494de-0x4951c). The view rolls the static against it.
//!
//! None of these functions plays a sound.

use std::collections::BTreeMap;

use crate::catalog::{LandingSite, OutfitId, OutfitRecord};
use crate::combat::hull::Condition;
use crate::exploration::GATES;
use crate::gate::{HYPERGATE, WORMHOLE};
use crate::landing::{StellarFlags, is_landable, landable};
use crate::outfitter::outfit_mods;
use crate::traffic::npc::Npc;

#[cfg(doc)]
use crate::{landing, market};

/// The `ModType` of a density scanner (`_HasDensityScanner` @0x431f).
pub const MOD_DENSITY_SCANNER: i16 = 13;
/// The `ModType` of IFF (`_HasIFF` @0x43a7).
pub const MOD_IFF: i16 = 14;
/// The `ModType` of anti-interference (`_ShipAntiInterference` @0x5f76).
pub const MOD_ANTI_INTERFERENCE: i16 = 24;
/// The most anti-interference counts for, either way (@0x5f9d-0x5fbd).
pub const ANTI_INTERFERENCE_LIMIT: i16 = 100;
/// The most interference there is (@0x494de-0x4951c).
pub const INTERFERENCE_LIMIT: u8 = 100;

/// What the player's outfits give the radar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RadarGear {
    /// Whether an owned outfit gives IFF.
    pub iff: bool,
    /// Whether an owned outfit gives a density scanner.
    pub density_scanner: bool,
    /// The anti-interference the owned outfits sum to, −100..=100.
    pub anti_interference: i16,
}

impl RadarGear {
    /// What the outfits `owned` (of `records`) give the radar: only counts
    /// above 0 count, and every one of an outfit's four mods.
    #[must_use]
    pub fn of(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> Self {
        let mods = outfit_mods(owned, records);
        let mods = mods.iter().filter(|m| m.count > 0);
        let mut gear = Self::default();
        let mut anti = 0_i32;
        for m in mods {
            match m.mod_type {
                MOD_IFF => gear.iff = true,
                MOD_DENSITY_SCANNER => gear.density_scanner = true,
                MOD_ANTI_INTERFERENCE => anti += i32::from(m.count) * i32::from(m.mod_val),
                _ => {}
            }
        }
        let limit = i32::from(ANTI_INTERFERENCE_LIMIT);
        gear.anti_interference = i16::try_from(anti.clamp(-limit, limit)).unwrap_or_default();
        gear
    }
}

/// What IFF shows a ship as (`_ColorCodeShip`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipIff {
    /// Disabled or breaking up: dark grey.
    Disabled,
    /// Threatening the player: red.
    Threat,
    /// Escorting the player: green.
    Escort,
    /// Any other ship: blue.
    Other,
}

/// What IFF shows `npc` as, led by `leader` (the ship its
/// [`leader`](Npc::leader) names, if it is in the system), among the
/// system's `npcs` (to tell which ships the player leads), tested in
/// `_ColorCodeShip`'s order (see the module docs).
#[must_use]
pub fn ship_iff(npc: &Npc, leader: Option<&Npc>, npcs: &[Npc]) -> ShipIff {
    if matches!(npc.condition, Condition::Disabled | Condition::Dying { .. }) {
        ShipIff::Disabled
    } else if npc.threatens_player_among(npcs) {
        ShipIff::Threat
    } else if npc.escorts_player() || leader.is_some_and(Npc::escorts_player) {
        ShipIff::Escort
    } else {
        ShipIff::Other
    }
}

/// What IFF shows a stellar as (`_ColorCodeStellar`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StellarIff {
    /// Not landable, or uninhabited: medium grey.
    Unlandable,
    /// A wormhole: teal.
    Wormhole,
    /// Dominated: green.
    Dominated,
    /// The player may land: yellow.
    Cleared,
    /// The player may not land, with a record of 0 or more: orange.
    Denied,
    /// The player may not land, with a record below 0: red.
    Hostile,
}

/// What the player stands with a stellar beyond its record: the inputs to
/// `_ColorCodeStellar` the caller works out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    /// Whether it is dominated (its +0x46 byte).
    pub dominated: bool,
    /// Whether its government's rank blessing lets the player land
    /// (`gövt` +0x85).
    pub blessed: bool,
}

/// The `MinStatus` that refuses every record (@0xaa96-0xab24).
const NEVER: i16 = 0x7FFF;
/// The `MinStatus` at or below which every record lands.
const ALWAYS: i16 = -32767;

/// What IFF shows `site` as, with the player's legal `record` in its
/// system and `standing` (see the module docs).
#[must_use]
pub fn stellar_iff(site: &LandingSite, record: i16, standing: Standing) -> StellarIff {
    if !is_landable(site) || site.flags & StellarFlags::UNINHABITED != 0 {
        return StellarIff::Unlandable;
    }
    if site.flags2 & HYPERGATE == 0 {
        if site.flags2 & WORMHOLE != 0 {
            return StellarIff::Wormhole;
        }
        if standing.dominated {
            return StellarIff::Dominated;
        }
    }
    by_record(site.min_status, record, standing.blessed)
}

/// The legal colours of a stellar of `MinStatus` `min_status` with the
/// player's `record`, `blessed` or not: [`StellarIff::Cleared`],
/// [`StellarIff::Denied`] or [`StellarIff::Hostile`], as
/// `_ColorCodeStellar` (@0xaa96-0xab24) and `_ColorCodeSystem`
/// (@0xd83a-0xd8a1) both read them (see the module docs).
fn by_record(min_status: i16, record: i16, blessed: bool) -> StellarIff {
    let refused = |record: i16| {
        if record < 0 {
            StellarIff::Hostile
        } else {
            StellarIff::Denied
        }
    };
    if blessed {
        StellarIff::Cleared
    } else if min_status == NEVER {
        refused(record)
    } else if min_status <= record || min_status <= ALWAYS {
        StellarIff::Cleared
    } else {
        refused(record)
    }
}

/// What the galaxy map frames a system in (`_ColorCodeSystem`; see the
/// module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemColorCode {
    /// No system stands for it, or its head is unexplored:
    /// `_rgbDarkGrey`.
    Unexplored,
    /// The system standing for it isn't inhabited, or none of its
    /// stellars counts: `_rgbLightGrey`.
    Uninhabited,
    /// A stellar is dominated: `_brightGreen`.
    Dominated,
    /// The player may land on a stellar: blue.
    Cleared,
    /// The player may not land, with a record of 0 or more: orange.
    Denied,
    /// The player may not land, with a record below 0: red.
    Hostile,
}

/// A stellar of the system standing for the one coloured, as
/// `_ColorCodeSystem` reads it (@0xd7e6-0xd8a1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapStellar {
    /// Whether it is active (+0x44): held by a visible system.
    pub active: bool,
    /// Its `Flags`.
    pub flags: u32,
    /// Its `Flags2`.
    pub flags2: u16,
    /// Whether it is destroyed.
    pub destroyed: bool,
    /// Its `MinStatus`.
    pub min_status: i16,
    /// Whether it is dominated, and whether its government blesses
    /// landing.
    pub standing: Standing,
}

impl MapStellar {
    /// Whether it counts at all: active, not a hypergate or wormhole,
    /// inhabited and landable (@0xd801-0xd82c).
    fn counts(&self) -> bool {
        self.active
            && self.flags2 & GATES == 0
            && self.flags & StellarFlags::UNINHABITED == 0
            && landable(self.flags, self.destroyed)
    }
}

/// What the galaxy map frames a system in (see the module docs): `stood`,
/// whether a system stands for it; `explored`, whether its head is valid
/// and explored; `inhabited`, whether the system standing for it is; and,
/// with the player's legal `record` in its head, the `stellars` of the
/// system standing for it.
#[must_use]
pub fn system_color_code(
    stood: bool,
    explored: bool,
    inhabited: bool,
    record: i16,
    stellars: &[MapStellar],
) -> SystemColorCode {
    if !stood || !explored {
        return SystemColorCode::Unexplored;
    }
    if !inhabited {
        return SystemColorCode::Uninhabited;
    }
    let (mut dominated, mut cleared, mut denied, mut hostile) = (false, false, false, false);
    for stellar in stellars.iter().filter(|stellar| stellar.counts()) {
        if stellar.standing.dominated {
            dominated = true;
            continue;
        }
        match by_record(stellar.min_status, record, stellar.standing.blessed) {
            StellarIff::Cleared => cleared = true,
            StellarIff::Denied => denied = true,
            _ => hostile = true,
        }
    }
    if dominated {
        SystemColorCode::Dominated
    } else if cleared {
        SystemColorCode::Cleared
    } else if denied {
        SystemColorCode::Denied
    } else if hostile {
        SystemColorCode::Hostile
    } else {
        SystemColorCode::Uninhabited
    }
}

/// The interference in a system of `Interference` `system` for a player
/// with `anti` anti-interference: the difference, clamped to 0..=100.
#[must_use]
pub fn interference(system: i16, anti: i16) -> u8 {
    let level = (i32::from(system) - i32::from(anti)).clamp(0, i32::from(INTERFERENCE_LIMIT));
    u8::try_from(level).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::combat::ShipRef;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::market::DOMINATED;
    use crate::stats::ShipStats;
    use crate::testkit::{npc, outfit, planet};
    use crate::traffic::npc::{MissionMark, NpcId};

    // The outfits.

    fn owned(counts: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        counts.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    #[test]
    fn the_mod_types_are_the_originals() {
        assert_eq!(
            [MOD_DENSITY_SCANNER, MOD_IFF, MOD_ANTI_INTERFERENCE],
            [13, 14, 24]
        );
    }

    #[test]
    fn iff_and_the_scanner_are_found_in_any_of_the_four_slots() {
        for slot in 0..4 {
            let mut mods = [(0, 0); 4];
            mods[slot] = (MOD_IFF, 0);
            let iff = outfit(128, &mods);
            mods[slot] = (MOD_DENSITY_SCANNER, 0);
            let scanner = outfit(129, &mods);
            let records = [iff, scanner];
            assert_eq!(
                RadarGear::of(&owned(&[(128, 1)]), &records),
                RadarGear {
                    iff: true,
                    ..RadarGear::default()
                },
                "slot {slot}"
            );
            assert_eq!(
                RadarGear::of(&owned(&[(129, 1)]), &records),
                RadarGear {
                    density_scanner: true,
                    ..RadarGear::default()
                },
                "slot {slot}"
            );
        }
    }

    #[test]
    fn an_outfit_owned_none_of_or_not_owned_gives_nothing() {
        let records = [
            outfit(128, &[(MOD_IFF, 0)]),
            outfit(129, &[(MOD_DENSITY_SCANNER, 0)]),
            outfit(130, &[(MOD_ANTI_INTERFERENCE, 30)]),
        ];
        let none = RadarGear::default();
        assert_eq!(
            RadarGear::of(&owned(&[(128, 0), (129, 0), (130, 0)]), &records),
            none
        );
        assert_eq!(RadarGear::of(&owned(&[]), &records), none);
        assert_eq!(RadarGear::of(&owned(&[(131, 5)]), &records), none);
    }

    #[test]
    fn other_mod_types_give_nothing() {
        let records = [outfit(128, &[(12, 30), (15, 30), (23, 30), (25, 30)])];
        assert_eq!(
            RadarGear::of(&owned(&[(128, 2)]), &records),
            RadarGear::default()
        );
    }

    #[test]
    fn anti_interference_sums_count_times_mod_val() {
        let records = [
            outfit(128, &[(MOD_ANTI_INTERFERENCE, 30)]),
            outfit(129, &[(MOD_ANTI_INTERFERENCE, 15), (MOD_IFF, 0)]),
        ];
        let gear = RadarGear::of(&owned(&[(128, 2), (129, 1)]), &records);
        assert_eq!(gear.anti_interference, 75);
        assert!(gear.iff);
    }

    #[test]
    fn anti_interference_clamps_at_100_either_way() {
        let records = [
            outfit(128, &[(MOD_ANTI_INTERFERENCE, 30)]),
            outfit(129, &[(MOD_ANTI_INTERFERENCE, -30)]),
        ];
        let anti =
            |counts: &[(i16, u16)]| RadarGear::of(&owned(counts), &records).anti_interference;
        assert_eq!(anti(&[(128, 3)]), 90);
        assert_eq!(anti(&[(128, 4)]), 100);
        assert_eq!(anti(&[(128, 2000)]), 100, "no 16-bit wrap");
        assert_eq!(anti(&[(129, 3)]), -90);
        assert_eq!(anti(&[(129, 4)]), -100);
        assert_eq!(anti(&[(128, 4), (129, 1)]), 90, "clamped once, summed");
    }

    // Ships.

    fn ship() -> Npc {
        npc(1, ShipStats::default())
    }

    fn hired() -> Npc {
        Npc {
            escort: Some(EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            }),
            ..ship()
        }
    }

    fn attacking(npc: Npc) -> Npc {
        Npc {
            goal: Goal::Attack(ShipRef::Player),
            ..npc
        }
    }

    #[test]
    fn a_disabled_or_breaking_up_ship_is_disabled_even_attacking() {
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
        ] {
            let npc = Npc {
                condition,
                ..attacking(hired())
            };
            assert_eq!(
                ship_iff(&npc, None, &[]),
                ShipIff::Disabled,
                "{condition:?}"
            );
        }
    }

    #[test]
    fn a_ship_attacking_the_player_is_a_threat_even_an_escort() {
        assert_eq!(ship_iff(&attacking(ship()), None, &[]), ShipIff::Threat);
        assert_eq!(ship_iff(&attacking(hired()), None, &[]), ShipIff::Threat);
        let elsewhere = Npc {
            goal: Goal::Attack(ShipRef::Npc(NpcId(9))),
            ..ship()
        };
        assert_eq!(ship_iff(&elsewhere, None, &[]), ShipIff::Other);
    }

    #[test]
    fn a_ship_attacking_one_the_player_leads_is_a_threat() {
        // `_IsThreatToPlayer`'s led clause (@0x7f572-0x7f590).
        let escort = Npc {
            id: NpcId(5),
            ..hired()
        };
        let stranger = Npc {
            id: NpcId(6),
            ..ship()
        };
        let hunting = |id| Npc {
            id: NpcId(2),
            goal: Goal::Attack(ShipRef::Npc(NpcId(id))),
            ..ship()
        };
        let npcs = [escort, stranger];
        assert_eq!(ship_iff(&hunting(5), None, &npcs), ShipIff::Threat);
        assert_eq!(ship_iff(&hunting(6), None, &npcs), ShipIff::Other);
    }

    #[test]
    fn a_hired_or_mission_escort_is_an_escort() {
        assert_eq!(ship_iff(&hired(), None, &[]), ShipIff::Escort);
        let mission = Npc {
            mission: Some(MissionMark {
                slot: 0,
                behav: crate::mission::ships::ShipBehav::EscortPlayer,
                formation: Some(FormationSlot::default()),
                order: None,
            }),
            ..ship()
        };
        assert_eq!(ship_iff(&mission, None, &[]), ShipIff::Escort);
    }

    #[test]
    fn any_other_ship_is_other_led_by_an_npc_too() {
        assert_eq!(ship_iff(&ship(), None, &[]), ShipIff::Other);
        let led = Npc {
            leader: Some(NpcId(7)),
            ..ship()
        };
        assert_eq!(ship_iff(&led, Some(&ship()), &[]), ShipIff::Other);
    }

    #[test]
    fn a_ship_led_by_the_players_escort_is_an_escort() {
        // Its parent's parent is the player (@0x4b80-0x4b8f): an
        // escort's fighter, which has no escort duty of its own.
        let fighter = Npc {
            leader: Some(NpcId(1)),
            ..npc(2, ShipStats::default())
        };
        assert_eq!(ship_iff(&fighter, Some(&hired()), &[]), ShipIff::Escort);
        assert_eq!(
            ship_iff(&attacking(fighter.clone()), Some(&hired()), &[]),
            ShipIff::Threat,
            "a threat first"
        );
        let disabled = Npc {
            condition: Condition::Disabled,
            ..fighter
        };
        assert_eq!(
            ship_iff(&disabled, Some(&hired()), &[]),
            ShipIff::Disabled,
            "disabled first"
        );
    }

    // Stellars.

    const NONE: Standing = Standing {
        dominated: false,
        blessed: false,
    };
    const DOMINATED_ONLY: Standing = Standing {
        dominated: true,
        blessed: false,
    };
    const BLESSED: Standing = Standing {
        dominated: false,
        blessed: true,
    };

    fn site(flags: u32, flags2: u16, min_status: i16) -> LandingSite {
        LandingSite {
            flags,
            min_status,
            flags2,
            ..planet(128, 0.0, 0.0)
        }
    }

    const LAND: u32 = StellarFlags::CAN_LAND;

    #[test]
    fn an_unlandable_or_uninhabited_stellar_is_unlandable() {
        let destroyed = LandingSite {
            destroyed: true,
            ..site(LAND, 0, 0)
        };
        let gates = WORMHOLE | HYPERGATE;
        for (shown, why) in [
            (site(0, 0, 0), "no can-land"),
            (destroyed, "destroyed"),
            (
                site(LAND | StellarFlags::ONLY_WHEN_DESTROYED, 0, 0),
                "only once destroyed",
            ),
            (site(LAND | StellarFlags::UNINHABITED, 0, 0), "uninhabited"),
            (
                site(LAND | StellarFlags::UNINHABITED, WORMHOLE, 0),
                "an uninhabited wormhole",
            ),
            (
                site(LAND | StellarFlags::UNINHABITED, gates | DOMINATED, 0),
                "uninhabited, whatever else",
            ),
        ] {
            assert_eq!(
                stellar_iff(&shown, 0, DOMINATED_ONLY),
                StellarIff::Unlandable,
                "{why}"
            );
        }
        let landable_destroyed = LandingSite {
            destroyed: true,
            ..site(LAND | StellarFlags::ONLY_WHEN_DESTROYED, 0, 0)
        };
        assert_eq!(
            stellar_iff(&landable_destroyed, 0, NONE),
            StellarIff::Cleared
        );
    }

    #[test]
    fn a_wormhole_is_teal_before_domination_or_the_record() {
        let wormhole = site(LAND, WORMHOLE, NEVER);
        assert_eq!(stellar_iff(&wormhole, -5, NONE), StellarIff::Wormhole);
        assert_eq!(
            stellar_iff(&wormhole, -5, DOMINATED_ONLY),
            StellarIff::Wormhole
        );
    }

    #[test]
    fn a_dominated_stellar_is_dominated_before_its_record() {
        let shown = site(LAND, 0, NEVER);
        assert_eq!(
            stellar_iff(&shown, -5, DOMINATED_ONLY),
            StellarIff::Dominated
        );
        assert_eq!(
            stellar_iff(
                &shown,
                -5,
                Standing {
                    dominated: true,
                    blessed: true
                }
            ),
            StellarIff::Dominated
        );
    }

    #[test]
    fn a_hypergate_skips_the_wormhole_and_dominated_tests() {
        for flags2 in [HYPERGATE, HYPERGATE | WORMHOLE] {
            let gate = site(LAND, flags2, 10);
            assert_eq!(stellar_iff(&gate, 20, DOMINATED_ONLY), StellarIff::Cleared);
            assert_eq!(stellar_iff(&gate, 5, DOMINATED_ONLY), StellarIff::Denied);
            assert_eq!(stellar_iff(&gate, -1, NONE), StellarIff::Hostile);
        }
    }

    #[test]
    fn a_blessing_clears_whatever_the_record() {
        assert_eq!(
            stellar_iff(&site(LAND, 0, 10), -50, BLESSED),
            StellarIff::Cleared
        );
        assert_eq!(
            stellar_iff(&site(LAND, 0, NEVER), -50, BLESSED),
            StellarIff::Cleared
        );
    }

    #[test]
    fn the_record_against_min_status_picks_yellow_orange_or_red() {
        let shown = site(LAND, 0, 10);
        assert_eq!(stellar_iff(&shown, 10, NONE), StellarIff::Cleared);
        assert_eq!(stellar_iff(&shown, 11, NONE), StellarIff::Cleared);
        assert_eq!(stellar_iff(&shown, 9, NONE), StellarIff::Denied);
        assert_eq!(stellar_iff(&shown, 0, NONE), StellarIff::Denied);
        assert_eq!(stellar_iff(&shown, -1, NONE), StellarIff::Hostile);
    }

    #[test]
    fn min_status_0x7fff_is_never_yellow() {
        let never = site(LAND, 0, 0x7FFF);
        assert_eq!(stellar_iff(&never, i16::MAX, NONE), StellarIff::Denied);
        assert_eq!(stellar_iff(&never, 0, NONE), StellarIff::Denied);
        assert_eq!(stellar_iff(&never, -1, NONE), StellarIff::Hostile);
    }

    #[test]
    fn min_status_at_or_below_minus_32767_is_always_yellow() {
        for min_status in [-32767, i16::MIN] {
            let always = site(LAND, 0, min_status);
            assert_eq!(
                stellar_iff(&always, i16::MIN, NONE),
                StellarIff::Cleared,
                "{min_status}"
            );
        }
        assert_eq!(
            stellar_iff(&site(LAND, 0, -32766), i16::MIN, NONE),
            StellarIff::Hostile
        );
    }

    // Systems on the galaxy map.

    /// A stellar that counts, of `MinStatus` `min_status`.
    fn counted(min_status: i16) -> MapStellar {
        MapStellar {
            active: true,
            flags: LAND,
            min_status,
            ..MapStellar::default()
        }
    }

    fn code(record: i16, stellars: &[MapStellar]) -> SystemColorCode {
        system_color_code(true, true, true, record, stellars)
    }

    #[test]
    fn a_system_with_no_stand_in_or_an_unexplored_head_is_dark_grey() {
        let stellars = [counted(0)];
        for (stood, explored) in [(false, true), (true, false), (false, false)] {
            assert_eq!(
                system_color_code(stood, explored, true, 0, &stellars),
                SystemColorCode::Unexplored
            );
        }
    }

    #[test]
    fn an_uninhabited_system_is_light_grey() {
        assert_eq!(
            system_color_code(true, true, false, 0, &[counted(0)]),
            SystemColorCode::Uninhabited
        );
    }

    #[test]
    fn each_bucket_gives_its_colour() {
        let dominated = MapStellar {
            standing: Standing {
                dominated: true,
                blessed: false,
            },
            ..counted(NEVER)
        };
        assert_eq!(code(-5, &[dominated]), SystemColorCode::Dominated);
        assert_eq!(code(10, &[counted(10)]), SystemColorCode::Cleared);
        assert_eq!(code(5, &[counted(10)]), SystemColorCode::Denied);
        assert_eq!(code(0, &[counted(10)]), SystemColorCode::Denied);
        assert_eq!(code(-1, &[counted(10)]), SystemColorCode::Hostile);
    }

    #[test]
    fn the_first_bucket_with_a_stellar_wins() {
        let dominated = MapStellar {
            standing: Standing {
                dominated: true,
                blessed: false,
            },
            ..counted(0)
        };
        let (clear, refused) = (counted(-5), counted(5));
        assert_eq!(
            code(0, &[refused, clear, dominated]),
            SystemColorCode::Dominated
        );
        assert_eq!(code(0, &[refused, clear]), SystemColorCode::Cleared);
        assert_eq!(code(-1, &[refused]), SystemColorCode::Hostile);
        assert_eq!(code(-1, &[counted(-5), refused]), SystemColorCode::Cleared);
    }

    #[test]
    fn a_blessing_clears_a_refused_stellar() {
        for min_status in [NEVER, 10] {
            let blessed = MapStellar {
                standing: Standing {
                    dominated: false,
                    blessed: true,
                },
                ..counted(min_status)
            };
            assert_eq!(code(-100, &[blessed]), SystemColorCode::Cleared);
        }
    }

    #[test]
    fn min_status_0x7fff_refuses_and_minus_32767_clears_every_record() {
        assert_eq!(code(i16::MAX, &[counted(NEVER)]), SystemColorCode::Denied);
        assert_eq!(code(-1, &[counted(NEVER)]), SystemColorCode::Hostile);
        assert_eq!(code(i16::MIN, &[counted(ALWAYS)]), SystemColorCode::Cleared);
        assert_eq!(
            code(i16::MIN, &[counted(i16::MIN)]),
            SystemColorCode::Cleared
        );
    }

    #[test]
    fn gates_uninhabited_inactive_and_unlandable_stellars_are_passed_over() {
        let skipped = [
            MapStellar {
                flags2: HYPERGATE,
                ..counted(0)
            },
            MapStellar {
                flags2: WORMHOLE,
                ..counted(0)
            },
            MapStellar {
                flags: LAND | StellarFlags::UNINHABITED,
                ..counted(0)
            },
            MapStellar {
                active: false,
                ..counted(0)
            },
            MapStellar {
                flags: 0,
                ..counted(0)
            },
            MapStellar {
                destroyed: true,
                ..counted(0)
            },
        ];
        for stellar in skipped {
            assert_eq!(
                code(0, &[stellar]),
                SystemColorCode::Uninhabited,
                "{stellar:?}"
            );
        }
        let rebuilt = MapStellar {
            flags: LAND | StellarFlags::ONLY_WHEN_DESTROYED,
            destroyed: true,
            ..counted(0)
        };
        assert_eq!(
            code(0, &[rebuilt]),
            SystemColorCode::Cleared,
            "landable destroyed"
        );
        assert_eq!(
            code(0, &[]),
            SystemColorCode::Uninhabited,
            "nothing counted"
        );
    }

    // Interference.

    #[test]
    fn interference_is_the_difference_clamped_to_0_to_100() {
        assert_eq!(interference(50, 60), 0);
        assert_eq!(interference(150, 0), 100);
        assert_eq!(interference(100, -100), 100);
        assert_eq!(interference(30, 10), 20);
        assert_eq!(interference(100, 0), 100);
        assert_eq!(interference(10, 10), 0);
        assert_eq!(interference(i16::MIN, i16::MAX), 0);
        assert_eq!(interference(i16::MAX, i16::MIN), 100);
    }
}
