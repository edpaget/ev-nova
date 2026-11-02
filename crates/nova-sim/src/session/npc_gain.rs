//! The frame's gain in the NPCs' flight, where the session hands it to
//! the traffic (`_HandleShip`'s `_gSpeedMult`, see
//! `nova_sim_base::speed_mult`).
//!
//! `Session::step_traffic` fills the traffic's [`World`] with
//! `Session::speed_mult`, and the traffic hands it to each NPC's move,
//! turn, thrust, ion block, cloak upkeep, bank, AI wait and fuel (the
//! `nova_sim_ai` traffic's `World::speed_mult`). `Session::ship_loop`
//! eases the NPCs' stray provocation at it ([`Traffic::ease_stray`]), and
//! `Session::npc_escape` hands it to `Space::eject_pods`, which spaces a
//! dying NPC's crew pods by it (`nova_sim_port::npc_escape`). The fight
//! reads it for an NPC's reloads and regeneration from the pass's
//! `Field` (`Session::field`). The entry's world (`Session::populate_as`)
//! and `Session::world_in` fly no ship, and read it all the same.
//!
//! These tests pin the wiring only: each expected value is the upstream
//! function at the same gain.
//!
//! [`World`]: crate::traffic::World
//! [`Traffic::ease_stray`]: crate::traffic::Traffic::ease_stray

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::SpeedMult;
    use crate::flight::{Controls, moved};
    use crate::geometry::Vec2;
    use crate::session::Session;
    use crate::stats::ShipStats;
    use crate::testkit::{catalog, work};
    use crate::traffic::npc::{Mode, Npc, NpcId};

    const HALF: SpeedMult = SpeedMult::new(0.5);

    /// A session flying at half gain with an NPC of `FAST` added at
    /// (600, 600), moving (2, 1), holding its course for 50 ticks.
    fn with_coaster() -> (crate::testkit::FakePilotCatalog, Session, NpcId) {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        work(&mut session, &catalog, Controls::default());
        session.at_gain(HALF);
        let mut coaster =
            crate::traffic::fixture::npc(1, ShipStats::new(nova_sim_base::fixture::FAST, &[]));
        coaster.state.position = Vec2::new(600.0, 600.0);
        coaster.state.velocity = Vec2::new(2.0, 1.0);
        coaster.mode = Mode::Coasting { ticks_left: 50.0 };
        let id = session.space.traffic.add_npc(coaster);
        (catalog, session, id)
    }

    fn npc(session: &Session, id: NpcId) -> &Npc {
        (session.npcs().iter())
            .find(|npc| npc.id == id)
            .expect("it stays")
    }

    #[test]
    fn at_half_gain_a_bare_pass_moves_a_coasting_npc_half_its_velocity() {
        let (catalog, mut session, id) = with_coaster();
        work(&mut session, &catalog, Controls::default());
        let flown = npc(&session, id).state;
        let expected = moved(Vec2::new(600.0, 600.0), flown.velocity, HALF);
        assert_eq!(flown.position, expected);
        assert_eq!(flown.velocity, Vec2::new(2.0, 1.0), "nothing pulls it");
    }

    #[test]
    fn at_half_gain_a_pass_eases_an_npcs_stray_provocation_at_half() {
        let (catalog, mut session, id) = with_coaster();
        let at = session
            .npcs()
            .iter()
            .position(|npc| npc.id == id)
            .expect("here");
        session.space.traffic.npcs_mut()[at].stray = 1.0;
        let mut eased = session.space.traffic.clone();
        eased.ease_stray(HALF);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(npc(&session, id).stray, eased.npcs()[at].stray);
    }
}
