//! A ship leaving its fleet: the parent -1 and the class's `InherentAI`
//! that `_AICheckParentExists` (@0x8935e-0x89383, @0x894b1-0x894f3),
//! `_EscortAI`'s lost parent (@0x83917-0x8397b, the AI type @0x83c4a) and
//! `_AIDispatch`'s disabled reset (@0x8fece-0x8ff13, its parent alone)
//! write ([`Npc::leave_fleet`]).

use super::npc::{AiType, Npc};

impl Npc {
    /// It leaves its fleet: no leader (+0x9a = -1), no carrier, holding its
    /// class's capacities again ([`Npc::leave_carrier`]), and its class's
    /// `InherentAI` (+0x12) as its AI type. The one writer of both: the
    /// succession, the disabled reset, `_EscortAI`'s lost parent
    /// ([`Decision::leaves_fleet`](crate::ai::Decision::leaves_fleet)) and
    /// the session's releases of the player's ships.
    pub fn leave_fleet(&mut self) {
        self.leave_carrier();
        self.leader = None;
        self.ai_type = AiType::from_raw(self.hull.inherent_ai);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::bay::Carrier;
    use crate::combat::ShipRef;
    use crate::rulebook::RuleSource;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::NpcId;
    use nova_sim_base::fixture::FAST;

    #[test]
    fn a_ship_leaving_its_fleet_drops_its_leader_and_carrier_and_takes_its_classs_ai() {
        let mut fighter = npc(1, ShipStats::new(FAST, &[]));
        fighter.leader = Some(NpcId(4));
        fighter.hull.inherent_ai = 3;
        fighter.take_carrier(
            Carrier {
                ship: ShipRef::Npc(NpcId(4)),
                window: 100.0,
                reach: 40.0,
            },
            RuleSource::Engine,
        );
        fighter.reserves.shield.now = 35.0;
        assert_eq!(fighter.reserves.shield.max, 39.99);
        fighter.leave_fleet();
        assert_eq!(fighter.leader, None);
        assert_eq!(fighter.carrier, None);
        assert_eq!(fighter.ai_type, AiType::Warship);
        assert_eq!(
            (fighter.reserves.shield.now, fighter.reserves.shield.max),
            (35.0, 30.0),
            "its class's capacity, what it holds kept"
        );
    }
}
