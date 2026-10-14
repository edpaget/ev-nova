//! NPCs coming out of hypergates and wormholes: which gate an arrival
//! emerges from, and how long it takes.
//!
//! Evidence: disassembly of the Mac OS X `EV Nova` (i386, with symbols).
//!
//! - **The pick** ([`emerge_stellar`]). `_HyperShipSpawn` (@0x42ac0) and
//!   `_HyperSpawnFleet`, for a fleet's lead (@0x41f0b), ask
//!   `_PickEmergeStellar` (@0x7f9a). That calls
//!   `_SelectRandomStellarDest(ship, Rand(3) == 0, 0)` and keeps the pick
//!   only when it is a hypergate or wormhole (`Flags2` 0x1000 or 0x2000,
//!   @0x7fe9); otherwise the ship jumps in from hyperspace.
//! - **The candidates.** `_SelectRandomStellarDest` (@0x805de) walks the
//!   system's stellars. One is *ok* when it is landable
//!   (`_StellarIsLandable` @0xa989, [`is_landable`]: a destroyed one only
//!   when only landable destroyed, `Flags` 0x80, an intact one only
//!   without it), its `xPos`
//!   and `yPos` are each at most 999 with no lower bound (@0x80781), and
//!   it is not hostile: both the ship and the stellar have a government,
//!   and they are enemies (@0x807e2-0x80816). One with `Flags` 0x20
//!   (uninhabited) that is no gate is *uninhabited* (@0x807c8). Then,
//!   by the ship's government's `Flags2` (@0x80625-0x8065d):
//!   - with [`PREFERS_WORMHOLES`] and an ok wormhole, among the ok
//!     wormholes (@0x80947);
//!   - else with [`PREFERS_HYPERGATES`] and an ok hypergate, among the
//!     ok hypergates (@0x80989);
//!   - else among the ok stellars that are not uninhabited, leaving out
//!     wormholes without [`PREFERS_WORMHOLES`] and hypergates with
//!     [`AVOIDS_HYPERGATES`] (@0x809cb, @0x80a30).
//!
//!   Each is a `Rand(16)` rejection loop over the slots: a uniform pick
//!   among the candidates, here one `below(candidates)` draw, in the
//!   system's stellar order. An independent ship has no switch, so its
//!   candidates are the ok, inhabited planets and stations and the ok
//!   hypergates; never a wormhole.
//! - **The `Rand(3) == 0` argument changes nothing** and is not drawn.
//!   It only chooses whether step 4 (@0x809cb) runs before step 5
//!   (@0x80a30), and whenever step 4 runs its test and its candidates are
//!   step 5's. The original consumes the draw; this does not.
//! - **No draw without a gate.** When no candidate is a gate, the pick can
//!   only end in a jump-in, so no draw is asked: a system with no ok gate
//!   draws exactly as before gates were modelled.
//! - **Emerging** (`_AIMakeShipEmergeFromHyperGate` @0x89518). The ship is
//!   placed at the gate's centre, at rest (the callers, @0x42ae4-0x42b0a,
//!   @0x41f34-0x41f64), heading the gate's `CustSndID` when 0-359 and
//!   `Rand(360)` otherwise ([`exit_heading`](crate::gate::exit_heading)),
//!   with its wait timer at [`EMERGE_TICKS`]. A fleet's escort comes out
//!   of its lead's gate with its lead's heading, waiting
//!   [`EMERGE_TICKS`] + [`ESCORT_EMERGE_EXTRA`] + `Rand(`
//!   [`ESCORT_EMERGE_SPREAD`] `)` ticks (@0x42534-0x4255e). While the
//!   timer runs the ship neither moves (`_HandleShip` @0x34a54) nor
//!   decides (`_HighLevelAIHandler` @0x8d4b9), and no ship sees it
//!   (`_ShipVisibleToShip` @0x972d): not the AI, not Target Select or
//!   Closest Target (@0xa801, @0x9330), and not a mission's
//!   `_MissionObjectivesCheck` (@0x9ea5c).
//! - **Leaving the gate.** Once the timer is out the ship bursts out along
//!   its heading at [`EXIT_SPEED`] pixels a tick, [`EXIT_SLOWING`] slower
//!   each tick ([`exit_speed`], @0x34bc0-0x34c63), until it would be at or
//!   below its cap (`_AIMakeShipStopJumping` @0x34cee): its top speed, or,
//!   while it has a lead in the system, the lower of its own and its
//!   lead's (`_HandleShip` @0x34c68-0x34ce6;
//!   [`TopSpeeds::burst_cap`](crate::traffic::autopilot::TopSpeeds::burst_cap)).
//!   The lead of a ship the player leads is the player, at its
//!   `_ShipMaxSpeed` (see [`top_speed`](crate::top_speed)); a fleet
//!   escort's is its fleet lead. A ship the
//!   player leads, made to come out of the exit gate after the player
//!   (see [`session::gate_escorts`](crate::session)), starts at
//!   [`LED_EXIT_SPEED`] instead: `_AIMakeShipEmergeFromHyperGate` sets
//!   -15.0 for leader 0 and -30.0 otherwise (@0x895aa-0x895bd), so its
//!   burst is the generic one from [`LED_BURST_TICK`].
//! - **Drawn** (`_HandleShipDisplay` @0x2c090-0x2c18e): hidden while its
//!   timer is above [`FADE_TICKS`], then fading in ([`emerge_level`]).
//!
//! The radar and the shots do not read the emerging state: an emerging
//! ship shows on the radar at its gate and can be hit.
//!
//! - **Holding its course.** A ship with no lead then holds its course
//!   for `Rand(30) + 30` ticks (@0x34d5c-0x34d8b), as after a glide in
//!   from hyperspace: [`Mode::Coasting`](crate::traffic::npc::Mode::Coasting).
//!
//! Not modelled: the
//! stellar the ship remembers (+0x92); a hypergate animating while a ship
//! comes out of it (`_HandleStellarSprites` @0x2e8bd).

use crate::catalog::{GovtId, LandingSite};
use crate::chance::Chance;
use crate::gate::GateKind;
use crate::govt::{AVOIDS_HYPERGATES, Governments, PREFERS_HYPERGATES, PREFERS_WORMHOLES};
use crate::landing::{StellarFlags, is_landable};

/// How many ticks a ship coming out of a gate waits there, hidden and
/// still (the timer's 60.0, @0x89518).
pub const EMERGE_TICKS: u32 = 60;
/// How many ticks longer a fleet's escort waits than its lead (60 +
/// `Rand(15)` + 5, @0x42534-0x4255e).
pub const ESCORT_EMERGE_EXTRA: u32 = 5;
/// `Rand(15)`: the spread of a fleet escort's wait.
pub const ESCORT_EMERGE_SPREAD: u32 = 15;
/// For how many of its last ticks a ship coming out of a gate fades in
/// (the 16.0 at 0xdd048).
pub const FADE_TICKS: u32 = 16;
/// A ship's speed, in pixels a tick, on the first tick it leaves a gate
/// (-30.0, @0x895aa).
pub const EXIT_SPEED: f32 = 30.0;
/// A ship the player leads starts its burst at 15.0 rather than 30.0
/// (`_AIMakeShipEmergeFromHyperGate` @0x895aa-0x895bd, leader 0).
pub const LED_EXIT_SPEED: f32 = 15.0;
/// The tick of the generic burst at which a led ship's burst starts:
/// [`exit_speed`]`(5)` is [`LED_EXIT_SPEED`] (@0x895aa-0x895bd).
pub const LED_BURST_TICK: u32 = 5;
/// How much slower each tick of leaving a gate is than the last (-3.0,
/// @0x895c4).
pub const EXIT_SLOWING: f32 = 3.0;
/// How far a stellar may be from the centre, on each axis, to come out of
/// (999, @0x80781-0x8078f): an upper bound only.
pub const MAX_COORDINATE: f32 = 999.0;
/// The opacity [`emerge_level`] counts in: 32nds.
pub const FULL_LEVEL: u8 = 32;

/// The gate among `sites` a ship of `govt` arriving comes out of, rolled
/// on `chance`; `None` when it jumps in from hyperspace (see the module
/// docs). No draw is asked unless a gate is among the candidates.
pub fn emerge_stellar<'s>(
    govt: Option<GovtId>,
    sites: &'s [LandingSite],
    govts: &Governments,
    chance: &mut (impl Chance + ?Sized),
) -> Option<&'s LandingSite> {
    let candidates = candidates(govt, sites, govts);
    if !candidates
        .iter()
        .any(|site| GateKind::of(site.flags2).is_some())
    {
        return None;
    }
    let draw = chance.below(u32::try_from(candidates.len()).unwrap_or(u32::MAX));
    candidates
        .get(usize::try_from(draw).unwrap_or(usize::MAX))
        .copied()
        .filter(|site| GateKind::of(site.flags2).is_some())
}

/// The stellars among `sites` that `_SelectRandomStellarDest` picks among
/// for a ship of `govt`, in order (see the module docs).
fn candidates<'s>(
    govt: Option<GovtId>,
    sites: &'s [LandingSite],
    govts: &Governments,
) -> Vec<&'s LandingSite> {
    let ok: Vec<&LandingSite> = sites
        .iter()
        .filter(|site| {
            is_landable(site)
                && site.position.x <= MAX_COORDINATE
                && site.position.y <= MAX_COORDINATE
                && !govts.npc_enemies(govt, site.govt)
        })
        .collect();
    let of_kind = |kind| -> Vec<&'s LandingSite> {
        ok.iter()
            .copied()
            .filter(|site| GateKind::of(site.flags2) == Some(kind))
            .collect()
    };
    let wormholes = govts.flag2(govt, PREFERS_WORMHOLES);
    if wormholes {
        let found = of_kind(GateKind::Wormhole);
        if !found.is_empty() {
            return found;
        }
    }
    if govts.flag2(govt, PREFERS_HYPERGATES) {
        let found = of_kind(GateKind::Hypergate);
        if !found.is_empty() {
            return found;
        }
    }
    let no_hypergates = govts.flag2(govt, AVOIDS_HYPERGATES);
    ok.into_iter()
        .filter(|site| match GateKind::of(site.flags2) {
            None => site.flags & StellarFlags::UNINHABITED == 0,
            Some(GateKind::Wormhole) => wormholes,
            Some(GateKind::Hypergate) => !no_hypergates,
        })
        .collect()
}

/// How fast a ship leaving a gate moves on tick `tick` (from 0) of its
/// burst.
#[must_use]
pub fn exit_speed(tick: u32) -> f32 {
    EXIT_SLOWING.mul_add(-(tick as f32), EXIT_SPEED)
}

/// How opaque a ship coming out of a gate with `ticks_left` of its wait
/// is drawn, in 32nds: `None` while it is hidden, above [`FADE_TICKS`],
/// then `32 - 2 * ticks_left`, rising to full.
#[must_use]
pub fn emerge_level(ticks_left: u32) -> Option<u8> {
    if ticks_left > FADE_TICKS {
        return None;
    }
    u8::try_from(2 * ticks_left)
        .ok()
        .map(|faded| FULL_LEVEL - faded)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::GovtRecord;
    use crate::gate::{HYPERGATE, WORMHOLE};
    use crate::testkit::{Draws, govt, planet};

    #[test]
    fn a_led_burst_is_the_generic_burst_from_its_fifth_tick() {
        assert_eq!(exit_speed(LED_BURST_TICK), LED_EXIT_SPEED);
        assert_eq!(
            exit_speed(LED_BURST_TICK - 1),
            LED_EXIT_SPEED + EXIT_SLOWING
        );
    }

    const SHIP: Option<GovtId> = Some(GovtId(128));
    const FOE: Option<GovtId> = Some(GovtId(129));
    const FRIEND: Option<GovtId> = Some(GovtId(130));

    /// Government 128 (class 1) at war with 129 (class 2); 130 is at war
    /// with nobody. 140 prefers hypergates, 141 wormholes, 142 avoids
    /// hypergates, 143 prefers both.
    fn govts() -> Governments {
        let with = |id, flags2| GovtRecord { flags2, ..govt(id) };
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                enemies: [2, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..govt(129)
            },
            govt(130),
            with(140, PREFERS_HYPERGATES),
            with(141, PREFERS_WORMHOLES),
            with(142, AVOIDS_HYPERGATES),
            with(143, PREFERS_HYPERGATES | PREFERS_WORMHOLES),
        ])
    }

    fn hypergate(id: i16) -> LandingSite {
        LandingSite {
            flags2: HYPERGATE,
            exit_angle: 90,
            ..planet(id, 300.0, 0.0)
        }
    }

    fn wormhole(id: i16) -> LandingSite {
        LandingSite {
            flags2: WORMHOLE,
            ..planet(id, -300.0, 0.0)
        }
    }

    /// Which of `sites` a ship of `govt` comes out of on `draws`, by ID,
    /// and the draws asked.
    fn picked(
        govt: Option<GovtId>,
        sites: &[LandingSite],
        draws: &[u32],
    ) -> (Option<i16>, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let pick = emerge_stellar(govt, sites, &govts(), &mut chance);
        (pick.map(|site| site.id.0), chance.asked)
    }

    #[test]
    fn a_draw_among_a_planet_and_a_hypergate_picks_the_gate_or_a_jump_in() {
        let sites = [planet(128, 0.0, 0.0), hypergate(129)];
        assert_eq!(picked(None, &sites, &[0]), (None, vec![2]), "the planet");
        assert_eq!(picked(None, &sites, &[1]), (Some(129), vec![2]));
        assert_eq!(picked(SHIP, &sites, &[1]), (Some(129), vec![2]));
    }

    #[test]
    fn only_a_government_preferring_wormholes_comes_out_of_one() {
        let sites = [planet(128, 0.0, 0.0), wormhole(129)];
        assert_eq!(
            picked(None, &sites, &[]),
            (None, vec![]),
            "no candidate gate"
        );
        assert_eq!(picked(SHIP, &sites, &[]), (None, vec![]));
        assert_eq!(
            picked(Some(GovtId(141)), &sites, &[0]),
            (Some(129), vec![1])
        );
        let lone = [wormhole(129)];
        assert_eq!(picked(None, &lone, &[]), (None, vec![]));
    }

    #[test]
    fn a_destroyed_gate_is_skipped_unless_it_is_only_landable_destroyed() {
        let destroyed = LandingSite {
            destroyed: true,
            ..hypergate(129)
        };
        let sites = [planet(128, 0.0, 0.0), destroyed];
        assert_eq!(picked(None, &sites, &[]), (None, vec![]));
        let only_destroyed = |destroyed| LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::ONLY_WHEN_DESTROYED,
            destroyed,
            ..hypergate(129)
        };
        let sites = [planet(128, 0.0, 0.0), only_destroyed(true)];
        assert_eq!(picked(None, &sites, &[1]), (Some(129), vec![2]));
        let sites = [planet(128, 0.0, 0.0), only_destroyed(false)];
        assert_eq!(picked(None, &sites, &[]), (None, vec![]));
        let unlandable = LandingSite {
            flags: 0,
            ..hypergate(129)
        };
        assert_eq!(picked(None, &[unlandable], &[]), (None, vec![]));
    }

    #[test]
    fn a_gate_of_an_enemy_government_is_skipped() {
        let gate_of = |govt| LandingSite {
            govt,
            ..hypergate(129)
        };
        let sites = [planet(128, 0.0, 0.0), gate_of(FOE)];
        assert_eq!(picked(SHIP, &sites, &[]), (None, vec![]), "at war");
        assert_eq!(picked(FRIEND, &sites, &[1]), (Some(129), vec![2]));
        assert_eq!(
            picked(None, &sites, &[1]),
            (Some(129), vec![2]),
            "independent ship"
        );
        let sites = [planet(128, 0.0, 0.0), gate_of(None)];
        assert_eq!(
            picked(SHIP, &sites, &[1]),
            (Some(129), vec![2]),
            "independent gate"
        );
        let planet_of_foe = LandingSite {
            govt: FOE,
            ..planet(128, 0.0, 0.0)
        };
        let sites = [planet_of_foe, hypergate(129)];
        assert_eq!(
            picked(SHIP, &sites, &[0]),
            (Some(129), vec![1]),
            "the planet is out"
        );
    }

    #[test]
    fn an_uninhabited_planet_is_no_candidate_but_an_uninhabited_gate_is() {
        let empty = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            ..planet(128, 0.0, 0.0)
        };
        assert_eq!(
            picked(None, &[empty, hypergate(129)], &[0]),
            (Some(129), vec![1])
        );
        let empty_gate = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            ..hypergate(129)
        };
        let sites = [planet(128, 0.0, 0.0), empty_gate];
        assert_eq!(picked(None, &sites, &[1]), (Some(129), vec![2]));
    }

    #[test]
    fn a_stellar_beyond_999_on_either_axis_is_out_but_none_is_too_low() {
        for (x, y) in [(1000.0, 0.0), (0.0, 1000.0)] {
            let far = LandingSite {
                position: crate::geometry::Vec2::new(x, y),
                ..hypergate(129)
            };
            assert_eq!(
                picked(None, &[planet(128, 0.0, 0.0), far], &[]),
                (None, vec![])
            );
        }
        let edge = LandingSite {
            position: crate::geometry::Vec2::new(999.0, 999.0),
            ..hypergate(129)
        };
        assert_eq!(picked(None, &[edge], &[0]), (Some(129), vec![1]));
        let low = LandingSite {
            position: crate::geometry::Vec2::new(-5000.0, -5000.0),
            ..hypergate(129)
        };
        assert_eq!(picked(None, &[low], &[0]), (Some(129), vec![1]));
    }

    #[test]
    fn a_governments_switches_choose_among_the_gates() {
        let sites = [planet(128, 0.0, 0.0), hypergate(129), wormhole(130)];
        // Prefers hypergates: among the ok hypergates alone.
        assert_eq!(
            picked(Some(GovtId(140)), &sites, &[0]),
            (Some(129), vec![1])
        );
        // Prefers wormholes: among the ok wormholes alone, ahead of
        // hypergates.
        assert_eq!(
            picked(Some(GovtId(141)), &sites, &[0]),
            (Some(130), vec![1])
        );
        assert_eq!(
            picked(Some(GovtId(143)), &sites, &[0]),
            (Some(130), vec![1])
        );
        // Avoids hypergates: never one.
        assert_eq!(picked(Some(GovtId(142)), &sites, &[]), (None, vec![]));
        // Prefers wormholes with none ok: the planet, the hypergate and
        // wormholes all count.
        let no_wormhole = [planet(128, 0.0, 0.0), hypergate(129)];
        assert_eq!(
            picked(Some(GovtId(141)), &no_wormhole, &[1]),
            (Some(129), vec![2])
        );
        assert_eq!(
            picked(Some(GovtId(140)), &no_wormhole, &[0]),
            (Some(129), vec![1])
        );
        // Prefers hypergates with none ok: as an independent.
        let only_planet = [planet(128, 0.0, 0.0), wormhole(130)];
        assert_eq!(picked(Some(GovtId(140)), &only_planet, &[]), (None, vec![]));
    }

    #[test]
    fn with_no_gate_among_the_candidates_nothing_is_drawn() {
        let sites = [planet(128, 0.0, 0.0), planet(129, 100.0, 0.0)];
        assert_eq!(picked(None, &sites, &[]), (None, vec![]));
        assert_eq!(picked(None, &[], &[]), (None, vec![]));
    }

    #[test]
    fn a_pick_is_uniform_over_the_candidates_in_stellar_order() {
        let sites = [
            planet(128, 0.0, 0.0),
            hypergate(129),
            planet(130, 0.0, 0.0),
            hypergate(131),
        ];
        for (draw, id) in [(0, None), (1, Some(129)), (2, None), (3, Some(131))] {
            assert_eq!(picked(None, &sites, &[draw]), (id, vec![4]), "{draw}");
        }
    }

    #[test]
    fn leaving_a_gate_starts_at_30_and_slows_by_3_a_tick() {
        assert_eq!(exit_speed(0), 30.0);
        assert_eq!(exit_speed(1), 27.0);
        assert_eq!(exit_speed(5), 15.0);
    }

    #[test]
    fn an_emerging_ship_is_hidden_above_16_ticks_then_fades_in() {
        assert_eq!(emerge_level(60), None);
        assert_eq!(emerge_level(17), None);
        assert_eq!(emerge_level(16), Some(0));
        assert_eq!(emerge_level(8), Some(16));
        assert_eq!(emerge_level(1), Some(30));
        assert_eq!(emerge_level(0), Some(FULL_LEVEL));
    }
}
