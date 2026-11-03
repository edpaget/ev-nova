//! The frame's gain in the AI, where the session hands it to the traffic
//! (`_LowLevelAIHandler`'s, `_HighLevelAIHandler`'s, `_AIDispatch`'s and
//! `_AIMaintainFormation`'s `_gSpeedMult`, see `nova_sim_base::speed_mult`).
//!
//! `Session::step_traffic` fills each think run's [`World`] with
//! `Session::speed_mult`, the same gain for every slot's run of one
//! frame's `_AIDispatch` loop, as nothing between the runs writes it. The
//! traffic hands it on (the `nova_sim_ai` traffic's `World::speed_mult`):
//! the dispatch reads it for an own jump's stage, a pod's wait, the
//! regroup release and a drift; the low level, through
//! `Readings::speed_mult`, for the formation nudge, the dock pull, the
//! dock's match and creep and the mining steps; the think, through
//! `Surroundings::speed_mult`, for the lost-sight search and a miner's
//! fire. Double time's second pass runs no AI, so none of these reads
//! the gain twice. No sub-state method reads it.
//!
//! These tests pin the wiring only: each expected value is the upstream
//! function at the same gain, on a copy of the state before the pass, and
//! differs from it at a full frame's.
//!
//! [`World`]: crate::traffic::World

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::SpeedMult;
    use crate::ai::Goal;
    use crate::ai::regroup::Kept;
    use crate::flight::{Controls, FlightRules};
    use crate::geometry::Vec2;
    use crate::session::Session;
    use crate::stats::ShipStats;
    use crate::testkit::{FakePilotCatalog, Passive, catalog, work, work_with};
    use crate::traffic::npc::{Npc, NpcId};
    use crate::traffic::{formation_flight, plunder, regroup};
    use nova_sim_base::fixture::{Draws, FAST};

    const HALF: SpeedMult = SpeedMult::new(0.5);

    /// A session in flight with `npc` added: the session, its catalog and
    /// the NPC's id.
    fn with(npc: Npc) -> (FakePilotCatalog, Session, NpcId) {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        work(&mut session, &catalog, Controls::default());
        let id = session.space.traffic.add_npc(npc);
        (catalog, session, id)
    }

    fn ship_at(x: f32, y: f32) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
        npc.state.position = Vec2::new(x, y);
        npc
    }

    fn npc(session: &Session, id: NpcId) -> &Npc {
        (session.npcs().iter())
            .find(|npc| npc.id == id)
            .expect("it stays")
    }

    /// `session` one pass on at `gain`, from a copy.
    fn passed(session: &Session, catalog: &FakePilotCatalog, gain: SpeedMult) -> Session {
        let mut session = session.clone();
        session.at_gain(gain);
        work(&mut session, catalog, Controls::default());
        session
    }

    #[test]
    fn at_half_gain_a_drifting_npc_drifts_by_the_traffics_drift_at_half_gain() {
        let mut pirate = ship_at(600.0, 600.0);
        pirate.goal = Goal::Drift;
        pirate.ai_wait = 5.0;
        let (catalog, session, id) = with(pirate);
        let drifted = |gain| {
            let mut npc = npc(&session, id).clone();
            plunder::drift(std::slice::from_mut(&mut npc), &[true], gain);
            npc.state.position
        };
        let after = npc(&passed(&session, &catalog, HALF), id).state.position;
        assert_eq!(after, drifted(HALF));
        assert_ne!(after, drifted(SpeedMult::FULL));
    }

    #[test]
    fn at_half_gain_a_paused_npc_draws_its_release_below_the_half_gain_range() {
        let mut paused = ship_at(600.0, 600.0);
        paused.goal = Goal::Regroup { kept: Kept::Coast };
        let (catalog, mut session, _) = with(paused);
        let range = regroup::release_draw(HALF);
        assert_ne!(range, regroup::release_draw(SpeedMult::FULL));
        session.at_gain(HALF);
        let mut chance = Draws::of(&[0]).keyed(range, 1);
        let rules = crate::combat::Rules::default();
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &Passive,
            rules,
            &mut chance,
        );
        assert_eq!(chance.times_asked(range), 1);
    }

    #[test]
    fn at_half_gain_an_escort_keeping_formation_is_nudged_by_maintain_at_half_gain() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        work(&mut session, &catalog, Controls::default());
        // The player at rest, so neither it nor its escort moves.
        session.ship.player.velocity = Vec2::ZERO;
        let player = session.ship.player;
        let mut escort = ship_at(0.0, 0.0);
        escort.goal = Goal::Formation { guard: None };
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot {
                slot: 2,
                ships: 3,
                spacing: 30.0,
            },
            order: None,
        });
        let slot = escort.slot_at(&player).expect("a slot");
        escort.state.position = slot + Vec2::new(20.0, 0.0);
        escort.state.velocity = player.velocity;
        escort.state.heading = player.heading;
        let id = session.space.traffic.add_npc(escort);
        let later = passed(&session, &catalog, HALF);
        // The pass places the player's escorts before the dispatch
        // (`_AICalcFormationPositions`), its spacing theirs.
        let placed = npc(&later, id).slot_at(&player).expect("a slot");
        let maintained = |gain| {
            let mut npc = npc(&session, id).clone();
            let handling = npc.flying_handling(npc.top_speed(1.0), FlightRules::default());
            formation_flight::maintain(&mut npc, placed, &handling, false, gain);
            npc.state.position
        };
        let after = npc(&later, id).state.position;
        assert_eq!(after, maintained(HALF));
        assert_ne!(after, maintained(SpeedMult::FULL));
    }
}
