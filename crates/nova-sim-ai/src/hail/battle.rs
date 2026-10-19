//! Request Assistance while an NPC threatens the player (`_DoCommDialog`
//! @0x96310-0x96514 in the `EV Nova` executable; see [`nova`](super::nova)'s
//! module docs, item 4), and the routine that sends a helper at a
//! threat: `_AIFightEnemyThreatToPlayer` ([`enemy_threat_pick`]).

use super::reply::{IN_YOUR_DREAMS, ON_MY_WAY, RATHER_NOT};
use super::{Answer, Deed, Defence, Hail, Reply};
use crate::chance::Chance;
use crate::traffic::npc::NpcId;

/// Request Assistance while the player is threatened (see the module
/// docs): "In your dreams, pal." unless an enemy of the ship's threatens
/// the player (@0x9632f); with battle help (@0x96335), "Okay, I'm on my
/// way." and it fights a threat picked on `chance`, calling for
/// reinforcements; else "I'd rather not.".
pub(super) fn defend(hail: &Hail, chance: &mut dyn Chance) -> Answer {
    if !hail.threatened_by_enemy {
        return Answer::say(Reply::Comm(IN_YOUR_DREAMS));
    }
    if !hail.dispositions.battle_help {
        return Answer::say(Reply::Comm(RATHER_NOT));
    }
    let threat = enemy_threat_pick(hail, chance);
    Answer {
        deed: threat.map(|threat| {
            Deed::Defend(Defence {
                threat: Some(threat),
                reinforce: true,
                drop_travel: false,
            })
        }),
        ..Answer::say(Reply::Comm(ON_MY_WAY))
    }
}

/// The threat `_AIFightEnemyThreatToPlayer` (@0x82c80-0x82dba) sends the
/// ship hailed at, picked on `chance`. It draws `Rand(63)+1` until a slot
/// holds a threat (@0x82d24-0x82d99): a uniform pick among every ship
/// that threatens the player ([`Hail::threats_to_player`]), here one draw
/// below their count. With none, nothing is drawn and there is no threat
/// (@0x82daa).
pub(crate) fn enemy_threat_pick(hail: &Hail, chance: &mut dyn Chance) -> Option<NpcId> {
    let threats: Vec<NpcId> = hail.threats_to_player().map(|npc| npc.id).collect();
    pick(&threats, chance)
}

/// One of `ships`, picked uniformly on `chance`; none, drawing nothing,
/// from none.
fn pick(ships: &[NpcId], chance: &mut dyn Chance) -> Option<NpcId> {
    let count = u32::try_from(ships.len()).unwrap_or(u32::MAX);
    if count == 0 {
        return None;
    }
    ships.get(chance.below(count) as usize).copied()
}
