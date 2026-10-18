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
//! - **The candidates** are `_SelectRandomStellarDest`'s
//!   ([`destination::candidates`] with [`Admit::All`]): the ok, inhabited
//!   stellars that are no enemy's, the government's `Flags2` choosing
//!   among the gates. An independent ship has no switch, so its
//!   candidates are the ok, inhabited planets and stations and the ok
//!   hypergates; never a wormhole. The pick among them is one
//!   `below(candidates)` draw, in the system's stellar order.
//! - **The `Rand(3) == 0` argument changes nothing** and is not drawn
//!   (see [`destination`]). The original consumes the draw; this does
//!   not.
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
//!   `_ShipMaxSpeed` (see `nova_sim::top_speed`); a fleet
//!   escort's is its fleet lead. A ship the
//!   player leads, made to come out of the exit gate after the player
//!   (see the session's `gate_escorts` module), starts at
//!   [`LED_EXIT_SPEED`] instead: `_AIMakeShipEmergeFromHyperGate` sets
//!   -15.0 for leader 0 and -30.0 otherwise (@0x895aa-0x895bd), and the
//!   burst slows from there at the same rate.
//!
//!   The slowing is sub-goal 0xa's -1.165 (`_LowLevelAIHandler`
//!   @0x88114-0x8813d), the hyperspace glide's, not the emerge's own -3.0
//!   (@0x895c4). `_DoPlayGameWork` runs the AI pass (`_AIDispatch`
//!   @0x4429d) after the frame's spawns and before its `_HandleShip` pass
//!   (@0x44483), and `_LowLevelAIHandler` zeroes +0x30 on every call
//!   (@0x851f1), so the -3.0 is gone before the ship can move. Once the
//!   wait is out, goal 0x15 becomes goal 8 (@0x8dd61-0x8dd93) and sub-goal
//!   0xa (@0x8f3c8-0x8f3d9), whose block keeps the emerge's negative start
//!   and sets +0x30 to -1.165 (@0x88136) before every tick of the burst.
//!   Only a ship with AI type 0 or below, which `_AIDispatch` never runs
//!   (@0x4427e-0x44286), would keep the -3.0; the engine has none.
//! - **Drawn** (`_HandleShipDisplay` @0x2c090-0x2c18e): hidden while its
//!   timer is above [`FADE_TICKS`], then fading in ([`emerge_level`]).
//! - **Holding fire.** A ship coming out of a gate fires nothing while it
//!   waits or bursts. The original holds no trigger from one frame to the
//!   next: `_LowLevelAIHandler` clears the fire flag +0xba each frame
//!   (@0x851f6) for an `_AIFire*` call to set again. While the wait runs
//!   `_HandleShip` skips the fire step (@0x34a54-0x34a72, the step at
//!   @0x34fcb), and `_AIEscortFireUnprovoked` (@0x85236) fires only at a
//!   target, which the emerge has cleared (@0x89544). Once the wait is
//!   out, `_HighLevelAIHandler` turns goal 0x15 into the jump-in goal 8
//!   (@0x8dd61-0x8dd93) with sub-goal 0xa (@0x8f3c8-0x8f3d9), whose
//!   `_LowLevelAIHandler` block calls no `_AIFire*` (@0x88114-0x88140).
//!   Here no decision is due while it waits or bursts, and an arrival out
//!   of a gate starts with no trigger; a ship sent to a gate from flight
//!   has its trigger released and its target cleared there
//!   ([`Npc::emerge_from_gate`](crate::traffic::npc::Npc::emerge_from_gate)).
//!   One exception: a hit gives the waiting ship a target, and an escort
//!   or fighter with an intact target then runs its point defence every
//!   tick of the wait, as `_AIEscortFireUnprovoked` reaches
//!   `_AIFireTurret` (@0x80e98, @0x80b0d); its trigger is released as it
//!   bursts out. A hit by the player, or a Good Samaritan's answer, holds
//!   the ship at its gate instead ([`Mode::HeldAtGate`](crate::traffic::npc::Mode::HeldAtGate)):
//!   seen and deciding, but still and firing only that point defence
//!   until its wait is out.
//!
//! The radar and the shots do not read the emerging state: an emerging
//! ship shows on the radar at its gate and can be hit.
//!
//! - **Holding its course.** A ship with no lead then holds its course
//!   for `Rand(30) + 30` ticks (@0x34d5c-0x34d8b), as after a glide in
//!   from hyperspace: [`Mode::Coasting`](crate::traffic::npc::Mode::Coasting).
//!
//! Not modelled: the stellar the ship remembers (+0x92).

use crate::catalog::{GovtId, LandingSite};
use crate::chance::Chance;
use crate::gate::GateKind;
use crate::govt::Governments;
use crate::traffic::destination::{self, Admit};

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
/// How much slower each tick of leaving a gate is than the last: sub-goal
/// 0xa's -1.165 (`_LowLevelAIHandler` @0x88136), the hyperspace glide's
/// [`JUMP_IN_SLOWING`](crate::traffic::spawn::JUMP_IN_SLOWING). The
/// emerge's own -3.0 (@0x895c4) is dead: `_LowLevelAIHandler` zeroes
/// +0x30 (@0x851f1) in the AI pass that follows the emerge in the same
/// frame, and sets it to -1.165 before every tick of the burst.
pub const EXIT_SLOWING: f32 = crate::traffic::spawn::JUMP_IN_SLOWING;
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
    let candidates = destination::candidates(govt, sites, govts, Admit::All);
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

/// How fast a ship leaving a gate moves on tick `tick` (from 0) of its
/// burst: from [`LED_EXIT_SPEED`] for a ship `led` out of the gate by the
/// player, from [`EXIT_SPEED`] otherwise, [`EXIT_SLOWING`] slower each
/// tick.
///
/// The original adds 1.165 to an f32 each frame (@0x34c2f-0x34c63), so
/// its speed drifts from this closed form by about 1e-5 over a burst, a
/// difference no player can see.
#[must_use]
pub fn exit_speed(led: bool, tick: u32) -> f32 {
    let start = if led { LED_EXIT_SPEED } else { EXIT_SPEED };
    EXIT_SLOWING.mul_add(-(tick as f32), start)
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
    use crate::govt::{AVOIDS_HYPERGATES, PREFERS_HYPERGATES, PREFERS_WORMHOLES};
    use crate::landing::StellarFlags;
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::{govt, planet};

    #[test]
    fn a_led_burst_starts_at_15_and_slows_by_the_same_rate() {
        assert_eq!(exit_speed(true, 0), 15.0);
        assert!((exit_speed(true, 1) - 13.835).abs() < 1e-4);
    }

    #[test]
    fn a_gate_burst_slows_as_the_hyperspace_glide_does() {
        assert_eq!(EXIT_SLOWING, crate::traffic::spawn::JUMP_IN_SLOWING);
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
    fn leaving_a_gate_starts_at_30_and_slows_by_1_165_a_tick() {
        assert_eq!(exit_speed(false, 0), 30.0);
        assert!((exit_speed(false, 1) - 28.835).abs() < 1e-4);
        assert!((exit_speed(false, 10) - 18.35).abs() < 1e-4);
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
