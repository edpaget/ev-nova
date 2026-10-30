//! Test support for the ai and traffic layer's tests and the tests above
//! it. Behind the `fixture` feature, for other crates' tests: a default
//! NPC ([`npc`]). For this crate's tests alone: one put in a pirate's
//! count wait (`count_waiting`), the calls among the line's work
//! (`calls_on_line`), and a behaviour logging the ships it decides for
//! and the player in a condition in a world of its own (`assisting`).

use super::npc::{AiType, Mode, Npc, NpcHold, NpcId};
use crate::ai::Goal;
use crate::ai::plunder_contents::ClassMarines;
use crate::ai::pursuit::Pursuit;
use crate::catalog::{EscortClass, ShipId};
use crate::cloak::{Cloak, CloakGear};
use crate::combat::armament::{Armament, LauncherRounds, Trigger};
use crate::combat::hull::{Condition, HullSpec};
use crate::combat::ion::Ion;
use crate::flight::ShipState;
use crate::stats::ShipStats;

/// NPC `id`, a wimpy trader of ship type 128 performing as `stats`, its
/// reserves full, at rest at the centre facing up, flying and idle,
/// intact and unarmed, of skill 1.0 and no government's factor, its
/// class's `Fuel` its `stats`' tank.
///
/// Kept for the facade's, story's and port's tests: a default NPC built
/// from public fields, pinning no rule.
#[must_use]
pub fn npc(id: u32, stats: ShipStats) -> Npc {
    Npc {
        id: NpcId(id),
        berth: 0,
        ship: ShipId(128),
        govt: None,
        ai_type: AiType::WimpyTrader,
        leader: None,
        class: EscortClass::Freighter,
        escort: None,
        stats,
        reserves: stats.full(),
        state: ShipState::default(),
        speed: 0.0,
        mode: Mode::Flying,
        goal: Goal::Idle,
        condition: Condition::Intact,
        hull: HullSpec::default(),
        armament: Armament::default(),
        rounds: LauncherRounds::default(),
        trigger: Trigger::default(),
        pick: None,
        target: None,
        provoked: 0.0,
        stray: 0.0,
        aggression: 0,
        inspected: None,
        booty: None,
        boarded: false,
        info_types: 0,
        cargo: NpcHold::default(),
        spared: false,
        carrier: None,
        person: None,
        mission: None,
        aux: None,
        cloak: Cloak::default(),
        cloak_gear: CloakGear::default(),
        led_out: false,
        emerges_from: None,
        visited: None,
        travel_set: false,
        self_repair: false,
        immobile: false,
        skill: 1.0,
        skill_mult: 1.0,
        still: false,
        miner: 0,
        can_scoop: false,
        holds: 0,
        class_fuel: class_fuel(&stats),
        former: None,
        tractor: None,
        ion: Ion::default(),
        afterburner: false,
        voice: None,
        pursuit: Pursuit::default(),
        ai_wait: 0,
        docking: None,
        lost_sight: crate::ai::lost_sight::SearchTimer::Unset,
        swarm_leader: None,
        owed_burst: None,
        close_form: false,
        fleet_slot: None,
        order_word: crate::fleet::OrderWord::NONE,
        marines: ClassMarines::default(),
        pods_left: 0,
        warp: crate::traffic::warp_out::Warp::default(),
        display: crate::traffic::ship_display::NpcDisplay::default(),
        odds: 0.0,
        no_ai: false,
        kept_escort_ai: false,
    }
}

/// `npc` put in goal 0x16, a pirate's count wait ([`Goal::CountWait`]),
/// with `ticks` of its wait left.
#[cfg(test)]
pub fn count_waiting(npc: &mut Npc, ticks: u32) {
    npc.goal = Goal::CountWait;
    npc.ai_wait = ticks;
}

/// A class `Fuel` holding the tank of `stats`, saturating at the word.
#[allow(clippy::cast_possible_truncation)]
fn class_fuel(stats: &ShipStats) -> i16 {
    stats.fuel as i16
}

/// The calls among the line's `work`, in order, each with the odds its
/// think read.
#[cfg(test)]
#[must_use]
pub fn calls_on_line(work: &[super::on_line::OnLine]) -> Vec<(NpcId, crate::ai::CommCall, f32)> {
    work.iter()
        .filter_map(|work| match *work {
            super::on_line::OnLine::Call(id, call, odds) => Some((id, call, odds)),
            _ => None,
        })
        .collect()
}

/// A behaviour logging the ships it decides for ([`assisting::Recording`]),
/// and the player in a condition ([`assisting::player_in`]) in a world of
/// its own ([`assisting::world_with`]), for the tests of the ships
/// assisting the player.
#[cfg(test)]
pub mod assisting {
    use std::cell::RefCell;

    use crate::ai::{Behaviour, Goal, PlayerSide, Reaction, Surroundings, react};
    use crate::chance::Chance;
    use crate::combat::Strike;
    use crate::combat::hull::Condition;
    use crate::traffic::World;
    use crate::traffic::npc::{Npc, NpcId};

    /// Logs each ship it decides for, keeping its goal, or waiting (state 6)
    /// when it has none; answers a hit as every AI type's victim does
    /// ([`react::victim`]).
    #[derive(Debug, Default)]
    pub struct Recording {
        /// The ships decided for, in order.
        pub decided: RefCell<Vec<NpcId>>,
    }

    impl Behaviour for Recording {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            self.decided.borrow_mut().push(npc.id);
            if npc.goal == Goal::Idle {
                Goal::Wait
            } else {
                npc.goal
            }
        }

        fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
            react::victim(npc, strike, around)
        }
    }

    /// The player at the centre at rest facing up, in `condition`.
    #[must_use]
    pub fn player_in(condition: Condition) -> PlayerSide {
        PlayerSide {
            condition,
            ..crate::ai::fixture::player(0.0, 0.0)
        }
    }

    /// A world holding the player in `condition`, and nothing else.
    #[must_use]
    pub fn world_with(condition: Condition) -> World<'static> {
        World {
            player: Some(player_in(condition)),
            ..World::new(&[])
        }
    }
}
