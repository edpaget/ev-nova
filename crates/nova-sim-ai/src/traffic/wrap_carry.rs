//! The player's wrap over the ship table: `_WrapPlayerPosition`'s loop of
//! the NPC slots 1-63 (@0x67f8e-0x6800e, stride 0xc940), which moves each
//! slot within reach of the player ([`PlayerWrap::carries`]) and tests
//! neither its active flag nor its system, so a freed slot's record moves
//! as a live ship does.
//!
//! [`Traffic::carry`] moves the ships in the system, the gone ships a think
//! may still read ([`gone`](super::gone)) and those an in-flight move left
//! behind, which hold their slots until the next tick ends. Velocities stay:
//! the routine writes none. A vacated berth's record
//! ([`vacated`](super::vacated)) stays too, as nothing reads its place.

use super::Traffic;
use super::npc::NpcId;
use crate::player_wrap::PlayerWrap;

impl Traffic {
    /// Carries the ships within `wrap`'s reach (see the module docs). The
    /// ships in the system it left behind, which the fight's contact walks
    /// still meet where their sprites stood.
    pub fn carry(&mut self, wrap: &PlayerWrap) -> Vec<NpcId> {
        let left = self
            .npcs
            .iter_mut()
            .filter_map(|npc| (!wrap.carry(&mut npc.state.position)).then_some(npc.id))
            .collect();
        self.gone.carry(wrap);
        for ship in self.left_behind.values_mut().flatten() {
            wrap.carry(&mut ship.npc.state.position);
        }
        left
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::SystemId;
    use crate::geometry::Vec2;
    use crate::player_wrap::{WRAP_EDGE, WRAP_SHIFT};
    use crate::stats::ShipStats;
    use crate::traffic::Leave;
    use crate::traffic::fixture::npc;
    use crate::traffic::gone::Freed;
    use crate::traffic::npc::Npc;
    use crate::traffic::table::SpawnTable;

    const PLAYER: Vec2 = Vec2::new(WRAP_EDGE + 10.0, 0.0);

    /// Well beyond the wrap's reach of [`PLAYER`].
    const FAR: Vec2 = Vec2::new(PLAYER.x - WRAP_EDGE * 2.0, 0.0);

    fn wrap() -> PlayerWrap {
        PlayerWrap::of(PLAYER).expect("past the edge")
    }

    fn ship_at(at: Vec2) -> Npc {
        let mut ship = npc(0, ShipStats::default());
        ship.state.position = at;
        ship.state.velocity = Vec2::new(3.0, -4.0);
        ship
    }

    fn traffic_of(ships: &[Vec2]) -> Traffic {
        let mut traffic = Traffic::new();
        for &at in ships {
            traffic.add_npc(ship_at(at));
        }
        traffic
    }

    fn position(traffic: &Traffic, index: usize) -> Vec2 {
        traffic.npcs()[index].state.position
    }

    #[test]
    fn a_ship_within_reach_moves_with_the_player_and_keeps_its_velocity() {
        let near = PLAYER + Vec2::new(-100.0, 40.0);
        let mut traffic = traffic_of(&[near]);
        traffic.carry(&wrap());
        assert_eq!(position(&traffic, 0), near + Vec2::new(-WRAP_SHIFT, 0.0));
        assert_eq!(traffic.npcs()[0].state.velocity, Vec2::new(3.0, -4.0));
    }

    #[test]
    fn a_ship_beyond_reach_stays_and_is_named_as_left_behind() {
        let near = PLAYER;
        let mut traffic = traffic_of(&[near, FAR, near]);
        let left = traffic.carry(&wrap());
        assert_eq!(position(&traffic, 1), FAR);
        assert_eq!(left, [traffic.npcs()[1].id]);
    }

    #[test]
    fn a_gone_ships_kept_place_moves_with_it() {
        let near = PLAYER + Vec2::new(-100.0, 40.0);
        let mut traffic = Traffic::new();
        let gone = traffic.add_npc(ship_at(near));
        traffic.remove_destroyed(gone);
        traffic.carry(&wrap());
        let kept = traffic.gone.take();
        assert_eq!(kept[0].freed, Freed::InShipPass);
        assert_eq!(
            kept[0].ship.state.position,
            near + Vec2::new(-WRAP_SHIFT, 0.0)
        );
    }

    #[test]
    fn a_ship_an_in_flight_move_left_behind_comes_back_moved() {
        let (old, new) = (SystemId(130), SystemId(131));
        let near = PLAYER + Vec2::new(-100.0, 40.0);
        let table = SpawnTable::default();
        let mut traffic = traffic_of(&[near, FAR]);
        let away = Leave::Held { from: old, to: new };
        traffic.retarget(table.clone(), &BTreeSet::new(), away);
        assert_eq!(traffic.carry(&wrap()), [], "not in the system");
        let back = Leave::Held { from: new, to: old };
        traffic.retarget(table, &BTreeSet::new(), back);
        assert_eq!(position(&traffic, 0), near + Vec2::new(-WRAP_SHIFT, 0.0));
        assert_eq!(position(&traffic, 1), FAR);
    }
}
