//! The session's side of a ship's refuel of the player
//! (`_HighLevelAIHandler`'s goal 9, @0x8f3e5-0x8f6d4 in the `EV Nova`
//! executable). The traffic flies the refueller, and tallies the fuel it
//! gives the player each think (see
//! [`refuel_dock`](crate::traffic::refuel_dock)), by the rulebook's
//! [`RuleKey::RefuelCap`](crate::rulebook::RuleKey::RefuelCap) reading,
//! which the session hands the traffic in its [`World`](crate::traffic::World).
//!
//! - **The fuel** lands in the pilot's tank after the traffic's tick
//!   ([`Space::take_refuel`]), before the frame is drawn, where the
//!   original adds it inside the refueller's think: no boundary is
//!   crossed. By the original's reading it has no cap, so a frame may show
//!   a tank past its most; the next tick's flight step clamps it back
//!   before the traffic reads it again, as `_HandlePlayer` clamps the fuel
//!   each frame before the AI loop (@0x6d9c7-0x6d9f2), the warp stage
//!   included. A tank of 100 or less thus never passes 100, and its refuel
//!   goes on for ever, until something else ends it.
//! - **The note**, "`CommName`:  Energy transfer complete, `ship name`.",
//!   with `snd ` 151, is queued on the line at the refueller's berth on the
//!   think after the fuel passed 100, and said as the line's work settles
//!   (see the `comm_calls` module).

use super::{Session, Space};
use crate::pilot::Pilot;

impl Session {}

impl Space {
    /// The fuel the ships refuelling the player gave it this tick lands in
    /// its tank (see the module docs).
    pub(super) fn take_refuel(&mut self, pilot: &mut Pilot) {
        pilot.reserves.fuel.now += self.traffic.take_fuel_given();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::tests::{armed, facing_an_npc};
    use super::super::*;
    use crate::ai::wimpy::WimpyTrader;
    use crate::catalog::{StellarId, SystemId};

    use crate::flight::Controls;
    use crate::hail::{CommNote, Help};
    use crate::reserves::Gauge;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::sound::SimSound;
    use crate::testkit::{Draws, FakePilotCatalog};
    use crate::testkit::{work, work_with};
    use crate::traffic::npc::{Mode, NpcId};

    #[test]
    fn the_traffic_steps_in_a_world_holding_the_rulebooks_refuel_cap() {
        let catalog = armed();
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let rules = Rulebook::default().with_override(RuleKey::RefuelCap, source);
            let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
            assert_eq!(session.world().refuel_cap, source);
            let mut flown = None;
            session.step_traffic(|_, world| flown = Some(world.refuel_cap));
            assert_eq!(flown, Some(source));
        }
    }

    /// [`armed`] with its NPC's ship's `CommName` "Shuttle".
    fn catalog() -> FakePilotCatalog {
        let mut catalog = armed();
        for record in &mut catalog.ship_records {
            record.comm_name = "Shuttle".to_owned();
        }
        catalog
    }

    /// `catalog`'s session, the player's ship the "Kestrel" holding `fuel`
    /// of `max`, its sounds taken, with its NPC 100 above it, at rest,
    /// refuelling it, its class turn 3 (a box of 105).
    fn refuelled(catalog: &FakePilotCatalog, fuel: f32, max: f32) -> (Session, NpcId) {
        let mut session = facing_an_npc(catalog, 180);
        session.pilot.ship_name = Some("Kestrel".to_owned());
        session.pilot.reserves.fuel = Gauge { now: fuel, max };
        let _ = session.take_sounds();
        let npc = &mut session.space.traffic.npcs_mut()[0];
        npc.goal = Goal::Assist(Help::Refuel);
        npc.target = Some(ShipRef::Player);
        npc.hull.class_turn = 3.0;
        npc.travel_set = true;
        let helper = npc.id;
        (session, helper)
    }

    /// A pass of `session`, its traffic deciding as Nova's traders do,
    /// its line's work settled in it: the notes said.
    fn traffic_tick(session: &mut Session, catalog: &FakePilotCatalog) -> Vec<CommNote> {
        work_with(
            session,
            catalog,
            Controls::default(),
            &WimpyTrader::default(),
            crate::Rules::default(),
            &mut Draws::of(&[0; 64]),
        );
        session.take_comm()
    }

    /// A frame of `session`: its pass ([`traffic_tick`]), the player's
    /// flight step among it.
    fn frame(session: &mut Session, catalog: &FakePilotCatalog) -> Vec<CommNote> {
        traffic_tick(session, catalog)
    }

    #[test]
    fn a_refuelling_ship_at_rest_inside_its_box_gives_a_unit_a_tick() {
        let catalog = catalog();
        let (mut session, _) = refuelled(&catalog, 40.0, 300.0);
        assert!(traffic_tick(&mut session, &catalog).is_empty());
        assert_eq!(session.reserves().fuel.now, 41.0);
        let (mut far, helper) = refuelled(&catalog, 40.0, 300.0);
        far.space.npc_mut(helper).expect("here").state.position = Vec2::new(0.0, -106.0);
        traffic_tick(&mut far, &catalog);
        assert_eq!(far.reserves().fuel.now, 40.0, "beyond its box");
        let (mut moving, helper) = refuelled(&catalog, 40.0, 300.0);
        moving.space.npc_mut(helper).expect("here").state.velocity = Vec2::new(0.0, 0.36);
        traffic_tick(&mut moving, &catalog);
        assert_eq!(moving.reserves().fuel.now, 40.0, "not slow enough");
    }

    #[test]
    fn a_refuel_stops_on_the_think_after_the_fuel_passes_100_and_says_so() {
        let catalog = catalog();
        let (mut session, helper) = refuelled(&catalog, 40.0, 300.0);
        for tick in 1..=61 {
            assert!(frame(&mut session, &catalog).is_empty(), "tick {tick}");
        }
        assert_eq!(session.reserves().fuel.now, 101.0);
        assert_eq!(session.take_sounds(), []);
        assert_eq!(
            frame(&mut session, &catalog),
            [CommNote {
                from: helper,
                comm_name: "Shuttle".to_owned(),
                ship_name: "Kestrel".to_owned(),
                done: Help::Refuel,
            }],
            "the 62nd tick"
        );
        assert_eq!(session.take_sounds(), [SimSound::EnergyTransferComplete]);
        assert_eq!(session.reserves().fuel.now, 101.0);
        let done = session.space.npc(helper).expect("here");
        assert_eq!(done.goal, Goal::Idle, "it decides again");
        assert_eq!(done.target, None);
        assert!(!done.travel_set, "+0x6c -1, @0x8f6ce");
        assert!(frame(&mut session, &catalog).is_empty());
        assert_eq!(session.reserves().fuel.now, 101.0);
    }

    #[test]
    fn under_double_time_the_second_pass_adds_no_help_and_tells_no_comm() {
        // The help and its note are the think's, which runs on the advance
        // pass alone (`_AIDispatch` @0x4402a).
        let catalog = catalog();
        let again = |session: &mut Session| {
            session.set_double_time_key(true);
            session.tick_again(
                Controls::default(),
                &WimpyTrader::default(),
                crate::Rules::default(),
                &mut Draws::of(&[0; 64]),
            );
            session.take_comm()
        };
        let (mut refuelling, _) = refuelled(&catalog, 95.0, 300.0);
        assert_eq!(again(&mut refuelling), [], "no note");
        assert_eq!(refuelling.reserves().fuel.now, 95.0, "no unit");
        let (mut done, helper) = refuelled(&catalog, 100.5, 300.0);
        let mut thought = done.clone();
        assert_eq!(
            again(&mut done),
            [],
            "no note, though the next think's is due"
        );
        assert_eq!(done.reserves().fuel.now, 100.5);
        assert_eq!(
            traffic_tick(&mut thought, &catalog)
                .iter()
                .map(|note| (note.from, note.done))
                .collect::<Vec<_>>(),
            [(helper, Help::Refuel)],
            "the think's"
        );
    }

    #[test]
    fn a_tank_of_100_or_less_is_refuelled_for_ever_under_the_originals_reading() {
        let catalog = catalog();
        let (mut session, helper) = refuelled(&catalog, 99.5, 100.0);
        assert!(frame(&mut session, &catalog).is_empty());
        assert_eq!(session.reserves().fuel.now, 100.5, "past its tank");
        for tick in 1..300 {
            assert!(frame(&mut session, &catalog).is_empty(), "tick {tick}");
            assert_eq!(session.reserves().fuel.now, 101.0, "tick {tick}");
        }
        assert_eq!(
            session.space.npc(helper).map(|npc| npc.goal),
            Some(Goal::Assist(Help::Refuel))
        );
        // With the refueller gone, the pass's flight step clamps it and
        // no think tops it up again (`_HandlePlayer` @0x6d9c7-0x6d9f2,
        // before the AI loop).
        session.space.traffic.remove(helper);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.reserves().fuel.now, 100.0, "clamped each frame");
    }

    #[test]
    fn under_the_cap_reading_a_full_tank_ends_the_refuel() {
        let catalog = catalog();
        let (session, helper) = refuelled(&catalog, 99.5, 100.0);
        let rules = Rulebook::default().with_override(RuleKey::RefuelCap, RuleSource::Bible);
        let mut session = session.with_rules(rules);
        assert!(frame(&mut session, &catalog).is_empty());
        assert_eq!(session.reserves().fuel.now, 100.0, "to the tank");
        assert_eq!(frame(&mut session, &catalog).len(), 1, "full: done");
        assert_eq!(session.reserves().fuel.now, 100.0);
        assert_eq!(
            session.space.npc(helper).map(|npc| npc.goal),
            Some(Goal::Idle)
        );
    }

    #[test]
    fn a_finished_help_holds_the_message_line_for_its_note() {
        let catalog = catalog();
        let (mut session, _) = refuelled(&catalog, 150.0, 300.0);
        assert!(session.comm_line().free_for_calls(), "free before");
        let [note] = traffic_tick(&mut session, &catalog)
            .try_into()
            .expect("one note");
        for tick in 1..note.ticks() {
            session.space.count_down_comm_line();
            assert!(!session.comm_line().free_for_calls(), "{tick} counted");
        }
        session.space.count_down_comm_line();
        assert!(session.comm_line().free_for_calls(), "the note counted");
    }

    #[test]
    fn a_player_already_above_a_jumps_fuel_gains_nothing_and_the_refuel_is_done() {
        let catalog = catalog();
        let (mut session, _) = refuelled(&catalog, 150.0, 300.0);
        assert_eq!(traffic_tick(&mut session, &catalog).len(), 1);
        assert_eq!(session.reserves().fuel.now, 150.0);
    }

    #[test]
    fn nothing_is_given_while_landed_or_by_a_ship_not_intact_or_coasting() {
        let catalog = catalog();
        let (mut landed, _) = refuelled(&catalog, 40.0, 300.0);
        landed.mode = crate::session::Mode::Landed(StellarId(128));
        let (mut disabled, helper) = refuelled(&catalog, 40.0, 300.0);
        disabled.space.npc_mut(helper).expect("here").condition = Condition::Disabled;
        let (mut coasting, helper) = refuelled(&catalog, 40.0, 300.0);
        coasting.space.npc_mut(helper).expect("here").mode = Mode::Coasting { ticks_left: 3 };
        // One at a time: three sessions make too large an array.
        let given_nothing = |name: &str, mut session: Session| {
            traffic_tick(&mut session, &catalog);
            assert_eq!(session.reserves().fuel.now, 40.0, "{name}");
        };
        given_nothing("landed", landed);
        given_nothing("disabled", disabled);
        given_nothing("coasting", coasting);
    }

    #[test]
    fn a_refuel_goes_on_through_the_warp_lead_in() {
        let catalog = catalog();
        let (mut session, _) = refuelled(&catalog, 40.0, 300.0);
        session.mode = crate::session::Mode::Jumping(Jump::Hyperspace {
            to: SystemId(131),
            warp: 0,
        });
        assert!(session.in_warp_lead_in());
        traffic_tick(&mut session, &catalog);
        assert_eq!(session.reserves().fuel.now, 41.0);
    }
}
