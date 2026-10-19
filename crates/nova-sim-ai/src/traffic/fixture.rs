//! Test support for the ai and traffic layer's tests and the tests above
//! it, behind the `fixture` feature: a default NPC ([`npc`]).

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
/// intact and unarmed, its class's `Fuel` its `stats`' tank.
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
        assisting: 0,
        carrier: None,
        person: None,
        mission: None,
        aux: None,
        cloak: Cloak::default(),
        cloak_gear: CloakGear::default(),
        led_out: false,
        emerges_from: None,
        visited: None,
        self_repair: false,
        immobile: false,
        miner: 0,
        can_scoop: false,
        holds: 0,
        class_fuel: class_fuel(&stats),
        former: None,
        tractor: None,
        ion: Ion::default(),
        afterburner: false,
        pursuit: Pursuit::default(),
        ai_wait: 0,
        docking: None,
        lost_sight: None,
        swarm_leader: None,
        marines: ClassMarines::default(),
        pods_left: 0,
        odds: 0.0,
    }
}

/// A class `Fuel` holding the tank of `stats`, saturating at the word.
#[allow(clippy::cast_possible_truncation)]
fn class_fuel(stats: &ShipStats) -> i16 {
    stats.fuel as i16
}
