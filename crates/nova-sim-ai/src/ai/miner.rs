//! [`MinerAi`]: how a leaderless miner decides, as `_MinerAI`
//! (@0x8b202-0x8b492 in the `EV Nova` executable) does. `_AIDispatch`
//! sends it every ship whose `shïp` `Flags3` has [`MINES`] or [`SCOOPS`]
//! and whose leader +0x9a is -1, in place of its AI type's think
//! (@0x8ff64-0x8ff96; see [`NovaAi`](crate::ai::NovaAi)).
//!
//! Its decision ([`decide`]), in the original's order:
//!
//! - **Disabled** (`_IsDisabled`): goal and sub-goal 0, travel stellar and
//!   target -1 (@0x8b217-0x8b23e): [`Goal::Idle`]. The traffic never
//!   decides for a disabled ship, so only this core's own test reaches it.
//! - **Waiting** in goal 0x16 ([`Goal::CountWait`]): it keeps waiting
//!   (@0x8b24a). Only a pirate takes goal 0x16, and `_AIDispatch` sends a
//!   miner to this AI before a pirate's (@0x8ff64), so no ship reaches it.
//! - **Provoked** (+0x96 > 0 with a target): state 3, [`Goal::Flee`] from
//!   its attacker ([`provoked_by`]), then a tail call to `_AICallForHelp`
//!   (@0x8b46c-0x8b483): its [`Decision::comm_call`] (see
//!   [`call_for_help`](crate::ai::call_for_help)).
//! - **[`MINES`]**, unless already leaving (state 2, [`Goal::JumpOut`]):
//!   with asteroid slot 0 live ([`Surroundings::mining_rock`]) state 16,
//!   [`Goal::Mine`]; without, state 6, [`Goal::Idle`], its target and
//!   travel stellar cleared (@0x8b2a1-0x8b2d1).
//! - **[`SCOOPS`]** with an asteroid scoop ([`Npc::can_scoop`], +0xc3):
//!   a hold as full as its `Holds` ([`NpcHold::total`](crate::traffic::npc::NpcHold::total)
//!   ≥ [`Npc::holds`], `_TotalCargo` against `shïp` +0x0) leaves (state 2,
//!   @0x8b2e7-0x8b32f); otherwise, with [`MINES`] too, it mines
//!   (state 16, @0x8b346-0x8b34c), asteroids or none; otherwise it
//!   shuttles. Without a scoop it shuttles, so an Asteroid Miner (0x0003)
//!   without one never mines or stops.
//! - **[`MINES`] alone** keeps the goal the first test gave.
//! - **The shuttle** (@0x8b35f-0x8b463): while its AI wait (+0x4c) runs
//!   ([`Npc::wait_running`]), state 0 with no travel stellar
//!   ([`Goal::Idle`]). Otherwise it keeps the stellar it is bound for
//!   (+0x6c), if still among the system's, or picks the nearest
//!   ([`select_nearest_any_stellar`]), leaving out +0x92, which in the
//!   engine is only ever the gate it came out of ([`Npc::emerges_from`]).
//!   With a stellar, and not leaving or fleeing (states 2 and 3), it lands
//!   there (state 1). Otherwise it leaves with a jump's fuel (state 2),
//!   or stops (state 6). The first test's state 6 comes before this one,
//!   so an Asteroid Miner that can't scoop lands even after fleeing.
//!
//! No `PlaySound` or `_beepSnd` call is anywhere in it; the distress
//! call's beep is `_AICallForHelp`'s.
//!
//! **Left out** (see rdm:phase/combat-ai-fidelity/phase-2-miners-without-asteroids):
//!
//! - State 17, scooping boxes while `_miningBoxesActive` is set
//!   (@0x8b331-0x8b344): rdm:phase/asteroids/phase-5-ai-mining. The core
//!   treats boxes as inactive.
//! - The exact `_AIShipHasFuelForJump` (@0x8b3ba-0x8b41f) and state 2's
//!   +0x50 clamp and +0xa4 stamp: rdm:phase/combat-ai-fidelity/phase-11-leave-by-hyperspace.
//!   The idle block's jump-fuel reading stands in.
//! - +0x92's "last landed" write (@0x8b3ae): a ship that lands leaves the
//!   system here.

use crate::ai::call_for_help::when_fleeing;
use crate::ai::{Behaviour, Decision, Goal, Surroundings, provoked_by};
use crate::catalog::StellarId;
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::hyperspace::JUMP_FUEL;
use crate::traffic::destination::select_nearest_any_stellar;
use crate::traffic::npc::Npc;

/// `shïp` `Flags3` 0x0001: the ship mines asteroid slot 0 (state 16), or
/// stops and waits without asteroids (state 6).
pub const MINES: u16 = 0x0001;

/// `shïp` `Flags3` 0x0002: with an asteroid scoop the ship scoops and
/// leaves on a full hold; without one it shuttles to its nearest stellar.
pub const SCOOPS: u16 = 0x0002;

/// The `Flags3` bits that make a ship a miner (`testb $0x3` @0x8ff76).
pub const MINER_BITS: u16 = 0x0003;

/// How a leaderless miner decides (see the module docs): it fires and
/// answers hits as its AI type does, which [`NovaAi`](crate::ai::NovaAi)
/// keeps, as `_AIDispatch` replaces only the type's think.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MinerAi;

impl Behaviour for MinerAi {
    fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        decide(npc, around)
    }

    /// Its [`decide`] goal, with a distress call fleeing, as `_MinerAI`
    /// tail-calls `_AICallForHelp` in state 3 (@0x8b46c-0x8b483,
    /// [`when_fleeing`]).
    fn decision(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Decision {
        let goal = decide(npc, around);
        Decision {
            goal,
            comm_call: when_fleeing(goal),
            ..Decision::default()
        }
    }

    /// Its attacker while it flees; no ship otherwise, as the stop and the
    /// leave clear its target (@0x8b2c5, @0x8b31e, @0x8b43f).
    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        npc.goal.quarry()
    }
}

/// `npc`'s goal among `around`, as `_MinerAI` decides it (see the module
/// docs).
#[must_use]
pub fn decide(npc: &Npc, around: &Surroundings) -> Goal {
    if npc.condition != Condition::Intact {
        return Goal::Idle;
    }
    if let Goal::CountWait { .. } = npc.goal {
        return npc.goal;
    }
    if let Some(attacker) = provoked_by(npc, around) {
        // `_MinerAI` tail-calls `_AICallForHelp` in state 3 (@0x8b46c-0x8b483):
        // its decision carries the call (`MinerAi::decision`).
        return Goal::Flee(attacker);
    }
    let mines = npc.miner & MINES != 0;
    let scoops = npc.miner & SCOOPS != 0;
    let mut state = npc.goal;
    let mut nav = bound_for(npc.goal, around);
    if mines && state != Goal::JumpOut {
        if around.mining_rock.is_some() {
            state = Goal::Mine;
        } else {
            state = Goal::Idle;
            nav = None;
        }
    }
    if !scoops {
        return if mines {
            state
        } else {
            shuttle(npc, around, state, nav)
        };
    }
    if npc.can_scoop {
        if npc.cargo.total() >= u64::from(npc.holds) {
            return Goal::JumpOut;
        }
        if mines {
            return Goal::Mine;
        }
    }
    shuttle(npc, around, state, nav)
}

/// The stellar a ship flying `goal` is bound for (+0x6c), while it is
/// still among `around`'s.
fn bound_for(goal: Goal, around: &Surroundings) -> Option<StellarId> {
    match goal {
        Goal::Land(stellar) if around.sites.iter().any(|site| site.id == stellar) => Some(stellar),
        _ => None,
    }
}

/// The nearest-stellar shuttle (@0x8b35f-0x8b463) of `npc` among
/// `around`, its state so far `state` and the stellar it is bound for
/// `nav` (see the module docs).
fn shuttle(npc: &Npc, around: &Surroundings, state: Goal, nav: Option<StellarId>) -> Goal {
    if npc.wait_running() {
        return Goal::Idle;
    }
    let nav = nav.or_else(|| {
        select_nearest_any_stellar(
            npc.state.position,
            npc.govt,
            around.sites,
            around.govts,
            npc.emerges_from,
        )
    });
    match nav {
        Some(stellar) if !matches!(state, Goal::JumpOut | Goal::Flee(_)) => Goal::Land(stellar),
        _ if npc.reserves.fuel.now >= JUMP_FUEL => Goal::JumpOut,
        _ => Goal::Idle,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, TRADERS, n, ship};
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::catalog::{LandingSite, StellarId};
    use crate::combat::hull::Condition;
    use crate::gate::{HYPERGATE, WORMHOLE};
    use crate::geometry::Vec2;
    use crate::hyperspace::JUMP_FUEL;
    use crate::traffic::npc::{AiType, Mode, Npc};
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::planet;
    use nova_sim_combat::asteroid::Asteroid;

    /// A leaderless miner of `bits`, scooping as `can_scoop` says, at the
    /// centre with a 20-ton hold, empty.
    fn miner(bits: u16, can_scoop: bool) -> Npc {
        let mut npc = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        npc.miner = bits;
        npc.can_scoop = can_scoop;
        npc.holds = 20;
        npc
    }

    /// Three stellars at 300, 100 and 200 from the centre.
    fn stellars() -> Vec<LandingSite> {
        vec![
            planet(128, 300.0, 0.0),
            planet(129, 0.0, -100.0),
            planet(130, -200.0, 0.0),
        ]
    }

    /// A live asteroid in slot 0.
    fn rock() -> Asteroid {
        Asteroid {
            position: Vec2::new(200.0, 200.0),
            velocity: Vec2::ZERO,
            frame: 0.0,
            spin: 0.0,
            strength: 30,
            kind: 0,
        }
    }

    fn deciding(npc: &Npc, sites: &[LandingSite], rock: Option<Asteroid>) -> Goal {
        let npcs = [npc.clone()];
        let around = Surroundings {
            mining_rock: rock,
            ..Surroundings::new(sites, &npcs)
        };
        decide(npc, &around)
    }

    #[test]
    fn the_miner_bits_are_the_two_flags() {
        assert_eq!(MINES | SCOOPS, MINER_BITS);
        assert_eq!(MINES & SCOOPS, 0);
    }

    #[test]
    fn a_mining_miner_stops_and_waits_without_asteroids_and_mines_slot_0_when_live() {
        let mut zephyr = miner(MINES, false);
        zephyr.goal = Goal::Land(StellarId(129));
        assert_eq!(deciding(&zephyr, &stellars(), None), Goal::Idle, "state 6");
        assert_eq!(deciding(&zephyr, &stellars(), Some(rock())), Goal::Mine);
        let npcs = [zephyr.clone()];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(MinerAi.target(&zephyr, &around), None);
    }

    #[test]
    fn a_miner_in_its_count_wait_keeps_it_whatever_it_is() {
        // `_MinerAI` returns on goal 0x16 (@0x8b24a), before its
        // provocation and its bits.
        for bits in [MINES, SCOOPS, MINER_BITS] {
            for can_scoop in [false, true] {
                let mut waiting = miner(bits, can_scoop);
                waiting.goal = Goal::CountWait { ticks_left: 7 };
                waiting.provoked = 5.0;
                waiting.target = Some(n(2));
                let attacker = ship(2, PIRATES, AiType::Warship, 0.0, 100.0);
                let npcs = [waiting.clone(), attacker];
                let around = Surroundings::new(&[], &npcs);
                assert_eq!(
                    decide(&waiting, &around),
                    waiting.goal,
                    "{bits} {can_scoop}"
                );
                assert_eq!(deciding(&waiting, &stellars(), Some(rock())), waiting.goal);
            }
        }
    }

    #[test]
    fn a_mining_miner_already_leaving_stays_leaving() {
        let mut leaving = miner(MINES, false);
        leaving.goal = Goal::JumpOut;
        assert_eq!(deciding(&leaving, &stellars(), None), Goal::JumpOut);
        assert_eq!(deciding(&leaving, &stellars(), Some(rock())), Goal::JumpOut);
    }

    #[test]
    fn a_shuttle_that_cant_scoop_lands_on_its_nearest_stellar_and_keeps_it() {
        let shuttle = miner(SCOOPS, false);
        assert_eq!(
            deciding(&shuttle, &stellars(), None),
            Goal::Land(StellarId(129))
        );
        let mut bound = shuttle.clone();
        bound.goal = Goal::Land(StellarId(128));
        assert_eq!(
            deciding(&bound, &stellars(), None),
            Goal::Land(StellarId(128)),
            "nav +0x6c kept, though 129 is nearer"
        );
        let gone = [planet(129, 0.0, -100.0), planet(130, -200.0, 0.0)];
        assert_eq!(
            deciding(&bound, &gone, None),
            Goal::Land(StellarId(129)),
            "its stellar gone, it picks again"
        );
    }

    #[test]
    fn a_shuttle_skips_gates_and_takes_an_unlandable_or_destroyed_stellar() {
        let shuttle = miner(SCOOPS, false);
        let sites = [
            LandingSite {
                flags2: HYPERGATE,
                ..planet(128, 10.0, 0.0)
            },
            LandingSite {
                flags2: WORMHOLE,
                ..planet(129, 20.0, 0.0)
            },
            LandingSite {
                flags: 0,
                destroyed: true,
                ..planet(130, 900.0, 0.0)
            },
        ];
        assert_eq!(deciding(&shuttle, &sites, None), Goal::Land(StellarId(130)));
    }

    #[test]
    fn a_shuttle_leaves_out_the_stellar_it_came_out_of() {
        let mut out = miner(SCOOPS, false);
        out.emerges_from = Some(StellarId(129));
        assert_eq!(
            deciding(&out, &stellars(), None),
            Goal::Land(StellarId(130)),
            "+0x92"
        );
    }

    #[test]
    fn a_shuttle_with_no_stellar_leaves_with_a_jumps_fuel_and_stops_without() {
        let mut shuttle = miner(SCOOPS, false);
        assert_eq!(deciding(&shuttle, &[], None), Goal::JumpOut);
        shuttle.reserves.fuel.now = JUMP_FUEL - 1.0;
        assert_eq!(deciding(&shuttle, &[], None), Goal::Idle, "state 6");
    }

    #[test]
    fn a_shuttle_leaving_or_no_longer_fleeing_doesnt_land() {
        for goal in [Goal::JumpOut, Goal::Flee(n(2))] {
            let mut shuttle = miner(SCOOPS, false);
            shuttle.goal = goal;
            assert_eq!(
                deciding(&shuttle, &stellars(), None),
                Goal::JumpOut,
                "{goal:?}"
            );
            shuttle.reserves.fuel.now = 0.0;
            assert_eq!(
                deciding(&shuttle, &stellars(), None),
                Goal::Idle,
                "{goal:?}"
            );
        }
    }

    #[test]
    fn a_shuttle_whose_wait_runs_drops_its_stellar_and_idles() {
        let mut held = miner(SCOOPS, false);
        held.goal = Goal::Land(StellarId(128));
        held.mode = Mode::HeldAtGate { ticks_left: 5 };
        assert_eq!(deciding(&held, &stellars(), None), Goal::Idle, "state 0");
    }

    #[test]
    fn a_scooping_miner_leaves_on_a_full_hold_mines_with_0x0001_and_else_shuttles() {
        let mut full = miner(SCOOPS, true);
        full.cargo.add(0, 12);
        full.cargo.add(4, 8);
        assert_eq!(deciding(&full, &stellars(), None), Goal::JumpOut);
        let mut both = full.clone();
        both.miner = MINER_BITS;
        assert_eq!(deciding(&both, &stellars(), Some(rock())), Goal::JumpOut);
        let mut room = miner(MINER_BITS, true);
        room.cargo.add(0, 19);
        assert_eq!(
            deciding(&room, &stellars(), None),
            Goal::Mine,
            "no asteroids too"
        );
        assert_eq!(deciding(&room, &stellars(), Some(rock())), Goal::Mine);
        let scooper = miner(SCOOPS, true);
        assert_eq!(
            deciding(&scooper, &stellars(), None),
            Goal::Land(StellarId(129)),
            "no boxes: the shuttle"
        );
    }

    #[test]
    fn a_miner_with_no_hold_is_full() {
        let mut empty = miner(SCOOPS, true);
        empty.holds = 0;
        assert_eq!(deciding(&empty, &stellars(), None), Goal::JumpOut);
    }

    #[test]
    fn an_asteroid_miner_that_cant_scoop_shuttles_rather_than_stops() {
        let mut asteroid_miner = miner(MINER_BITS, false);
        assert_eq!(
            deciding(&asteroid_miner, &stellars(), None),
            Goal::Land(StellarId(129))
        );
        asteroid_miner.goal = Goal::Land(StellarId(128));
        assert_eq!(
            deciding(&asteroid_miner, &stellars(), None),
            Goal::Land(StellarId(129)),
            "state 6 clears its nav first"
        );
        assert_eq!(
            deciding(&asteroid_miner, &stellars(), Some(rock())),
            Goal::Land(StellarId(128)),
            "state 16 keeps it"
        );
        asteroid_miner.goal = Goal::Flee(n(2));
        assert_eq!(
            deciding(&asteroid_miner, &stellars(), None),
            Goal::Land(StellarId(129)),
            "state 6 is no longer fleeing"
        );
    }

    #[test]
    fn a_provoked_miner_flees_its_attacker_and_targets_it() {
        let mut provoked = miner(MINER_BITS, true);
        provoked.provoked = 5.0;
        provoked.target = Some(n(2));
        let npcs = [
            provoked.clone(),
            ship(2, PIRATES, AiType::Warship, 50.0, 0.0),
        ];
        let around = Surroundings::new(&[], &npcs);
        let mut chance = Draws::of(&[]);
        assert_eq!(
            MinerAi.decide(&provoked, &around, &mut chance),
            Goal::Flee(n(2))
        );
        let fleeing = Npc {
            goal: Goal::Flee(n(2)),
            ..provoked.clone()
        };
        assert_eq!(MinerAi.target(&fleeing, &around), Some(n(2)));
        let alone = [provoked.clone()];
        let around = Surroundings::new(&[], &alone);
        assert_eq!(
            decide(&provoked, &around),
            Goal::Mine,
            "its attacker gone, it decides as unprovoked"
        );
    }

    #[test]
    fn a_provoked_miner_calls_for_help_and_a_working_one_does_not() {
        let mut provoked = miner(MINER_BITS, true);
        provoked.provoked = 5.0;
        provoked.target = Some(n(2));
        let npcs = [
            provoked.clone(),
            ship(2, PIRATES, AiType::Warship, 50.0, 0.0),
        ];
        let around = Surroundings::new(&[], &npcs);
        let decision = MinerAi.decision(&provoked, &around, &mut Draws::of(&[]));
        assert_eq!(decision.goal, Goal::Flee(n(2)));
        assert_eq!(decision.comm_call, Some(crate::ai::CommCall::Distress));
        let alone = [provoked.clone()];
        let around = Surroundings::new(&[], &alone);
        let decision = MinerAi.decision(&provoked, &around, &mut Draws::of(&[]));
        assert_eq!(decision.goal, Goal::Mine);
        assert_eq!(decision.comm_call, None);
    }

    #[test]
    fn a_disabled_miner_idles_with_no_target() {
        let mut disabled = miner(SCOOPS, false);
        disabled.condition = Condition::Disabled;
        disabled.goal = Goal::Land(StellarId(128));
        disabled.provoked = 5.0;
        disabled.target = Some(n(2));
        let npcs = [
            disabled.clone(),
            ship(2, PIRATES, AiType::Warship, 50.0, 0.0),
        ];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(decide(&disabled, &around), Goal::Idle);
        let idle = Npc {
            goal: Goal::Idle,
            ..disabled
        };
        assert_eq!(MinerAi.target(&idle, &around), None);
    }

    #[test]
    fn in_traffic_a_shuttle_lands_on_its_nearest_stellar_and_a_mining_miner_comes_to_rest() {
        use crate::ai::NovaAi;
        use crate::traffic::Traffic;
        use crate::traffic::autopilot::Outcome;
        let sites = stellars();
        let mut traffic = Traffic::new();
        let mut shuttle = miner(SCOOPS, false);
        shuttle.state.position = Vec2::new(0.0, 100.0);
        let mut zephyr = miner(MINES, false);
        zephyr.state.position = Vec2::new(500.0, 500.0);
        zephyr.state.velocity = Vec2::new(3.0, -2.0);
        let shuttle = traffic.add_npc(shuttle);
        let zephyr = traffic.add_npc(zephyr);
        let ai = NovaAi::default();
        let mut landed = None;
        for _ in 0..600 {
            traffic.tick(&ai, &sites, &mut Draws::of(&[]));
            if let Some(&(_, outcome)) = traffic.departed().iter().find(|(id, _)| *id == shuttle) {
                landed = Some(outcome);
            }
        }
        assert_eq!(landed, Some(Outcome::Landed(StellarId(129))));
        let at_rest = traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == zephyr)
            .expect("it stays");
        assert_eq!(at_rest.goal, Goal::Idle);
        assert!(at_rest.state.velocity.length() < 0.5, "{:?}", at_rest.state);
    }
}
