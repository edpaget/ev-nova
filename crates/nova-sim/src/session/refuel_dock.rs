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
//! - **Through the warp** the refueller runs as ever: `_DoPlayGameWork`'s
//!   AI loop (@0x44270-0x442ae) tests only that a ship is active, has AI
//!   and is in the player's system, and the session thinks and flies the
//!   traffic on every pass of a jump. The fuel, the note and its sound
//!   come past the warp's lead-in as before it, until the player's
//!   move-off takes it out of the box and it closes after the player.

use super::{Session, Space};
use crate::pilot::Pilot;

impl Session {}

impl Space {
    /// The fuel the ships refuelling the player have given it lands in
    /// its tank, after each slot's think, so the next slot reads the tank
    /// live (see the module docs).
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
    use crate::traffic::plunder_dock::DockSub;
    use crate::traffic::refuel_dock::refuel_box;

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

    /// [`refuelled`], its NPC at berth 1, with a second refueller on the
    /// same terms at berth 5 and a ready-bay carrier launching nothing
    /// between them. The session and the higher refueller.
    fn refuelled_either_side_of_a_bay(
        catalog: &FakePilotCatalog,
        fuel: f32,
        max: f32,
        rules: Rulebook,
    ) -> (Session, NpcId) {
        let (session, low) = refuelled(catalog, fuel, max);
        let mut session = session.with_rules(rules);
        session.space.npc_mut(low).expect("here").berth = 1;
        let helper = session.space.npc(low).expect("here").clone();
        let high = session.space.traffic.add_npc_at(helper, 5);
        let mut carrier = crate::session::fighters::tests::carrier_npc(low, 2);
        carrier.goal = Goal::Idle;
        carrier.target = None;
        session.space.traffic.add_npc_at(carrier, 3);
        (session, high)
    }

    #[test]
    fn refuellers_either_side_of_a_ready_bay_read_the_fuel_the_lower_one_gave() {
        // The tank is live: the higher's goal 9 reads what the lower gave
        // this frame (@0x8f3e5-0x8f6d4), so at 101 it stops and says so.
        let catalog = catalog();
        let (mut session, high) =
            refuelled_either_side_of_a_bay(&catalog, 100.0, 300.0, Rulebook::default());
        let notes = traffic_tick(&mut session, &catalog);
        assert_eq!(session.reserves().fuel.now, 101.0);
        assert_eq!(notes.len(), 1, "the higher's note");
        assert!(
            session
                .take_sounds()
                .contains(&SimSound::EnergyTransferComplete)
        );
        assert_eq!(session.space.npc(high).expect("here").goal, Goal::Idle);
    }

    #[test]
    fn refuellers_either_side_of_a_ready_bay_fill_no_more_than_the_tank_by_the_cap() {
        // One unit of room: the lower fills it, and the higher reads the
        // tank full and is done.
        let catalog = catalog();
        let rules = Rulebook::default().with_override(RuleKey::RefuelCap, RuleSource::Bible);
        let (mut session, high) = refuelled_either_side_of_a_bay(&catalog, 99.0, 100.0, rules);
        traffic_tick(&mut session, &catalog);
        assert_eq!(session.reserves().fuel.now, 100.0);
        assert_eq!(session.space.npc(high).expect("here").goal, Goal::Idle);
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

    /// Where [`refuelled_in_a_jump`] puts the player, far enough out to
    /// jump.
    const OUT: Vec2 = Vec2 { x: 0.0, y: -5000.0 };

    /// The box of `helper` of `session`, as the traffic reads it: the
    /// traffic's rule, followed here rather than repeated.
    fn box_of(session: &Session, helper: NpcId) -> f32 {
        refuel_box(session.space.npc(helper).expect("here").hull.class_turn) as f32
    }

    /// [`refuelled`], with the player at rest at [`OUT`] facing east, the
    /// bearing to 131, its NPC still 100 above it, and its jump begun: its
    /// warp under way, with the stock warp sounds.
    fn refuelled_in_a_jump(catalog: &FakePilotCatalog, fuel: f32, max: f32) -> (Session, NpcId) {
        let (mut session, helper) = refuelled(catalog, fuel, max);
        session.statics_mut().warp_sounds = [364, 252];
        session.plot_course(SystemId(131)).expect("a route");
        session.ship.player = ShipState {
            position: OUT,
            velocity: Vec2::ZERO,
            heading: 90.0,
        };
        session.space.npc_mut(helper).expect("here").state.position = OUT + Vec2::new(0.0, -100.0);
        session.begin_jump().expect("jumps");
        assert_eq!(session.jumping(), Some(SystemId(131)), "the warp, at once");
        assert!(session.in_warp_lead_in());
        (session, helper)
    }

    /// Runs `session`'s passes until its warp is past its lead-in, `each`
    /// looking on after every pass, and the warp not yet at its end.
    fn past_the_lead_in(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        mut each: impl FnMut(&Session),
    ) {
        while session.in_warp_lead_in() {
            assert!(traffic_tick(session, catalog).is_empty());
            each(session);
        }
        assert_eq!(session.jumping(), Some(SystemId(131)));
        assert!(!session.warp_end_due(), "past the lead-in, not at the end");
    }

    /// Whether `session`'s sounds taken now hold the refuel's, the warp
    /// raising its own among them.
    fn took_the_refuels_sound(session: &mut Session) -> bool {
        session
            .take_sounds()
            .contains(&SimSound::EnergyTransferComplete)
    }

    /// `helper` of `session` put in the player's box at rest, refuelling
    /// it, as a hail would have it past the warp's lead-in.
    fn refuelling_from_here(session: &mut Session, helper: NpcId) {
        let above = session.ship.player.position + Vec2::new(0.0, -100.0);
        let npc = session.space.npc_mut(helper).expect("here");
        npc.goal = Goal::Assist(Help::Refuel);
        npc.target = Some(ShipRef::Player);
        npc.travel_set = true;
        npc.state.position = above;
        npc.state.velocity = Vec2::ZERO;
    }

    #[test]
    fn a_refueller_past_the_warps_lead_in_closes_after_the_player() {
        let catalog = catalog();
        let (mut session, helper) = refuelled_in_a_jump(&catalog, 200.0, 300.0);
        session.space.npc_mut(helper).expect("here").state.position = Vec2::new(-400.0, OUT.y);
        past_the_lead_in(&mut session, &catalog, |_| {});
        let mut passes = 0;
        while !session.warp_end_due() {
            let was = session.space.npc(helper).expect("here").state.position.x;
            let player_was = session.ship.player.position.x;
            assert!(traffic_tick(&mut session, &catalog).is_empty());
            let npc = session.space.npc(helper).expect("here");
            assert_eq!(npc.goal, Goal::Assist(Help::Refuel), "pass {passes}");
            assert_eq!(npc.target, Some(ShipRef::Player), "pass {passes}");
            assert_eq!(
                npc.docking.map(|docking| docking.sub),
                Some(DockSub::Close),
                "pass {passes}"
            );
            assert!(npc.state.position.x > was, "it closes, pass {passes}");
            assert!(
                session.ship.player.position.x >= player_was,
                "the player moves off, pass {passes}"
            );
            passes += 1;
            assert!(passes < 1000, "the warp never ended");
        }
        assert!(session.ship.player.position.x > 1000.0, "well off");
    }

    #[test]
    fn a_refueller_beside_the_player_leaves_its_box_as_the_player_moves_off() {
        let catalog = catalog();
        // 100 of 100: a refuel that goes on for ever (see the module docs).
        let (mut session, helper) = refuelled_in_a_jump(&catalog, 100.0, 100.0);
        past_the_lead_in(&mut session, &catalog, |session| {
            assert_eq!(session.reserves().fuel.now, 101.0, "it refuels");
        });
        let gap = |session: &Session| {
            let npc = session.space.npc(helper).expect("here");
            session.ship.player.position.x - npc.state.position.x
        };
        let side = box_of(&session, helper);
        assert!(gap(&session) <= side, "inside its box");
        let mut passes = 0;
        while gap(&session) <= side {
            traffic_tick(&mut session, &catalog);
            passes += 1;
            assert!(passes < 1000, "the player never moved off");
        }
        let at_rest = session.space.npc(helper).expect("here").state;
        while !session.warp_end_due() {
            assert_eq!(session.reserves().fuel.now, 100.0, "the clamp's, no more");
            assert!(traffic_tick(&mut session, &catalog).is_empty());
            let npc = session.space.npc(helper).expect("here");
            assert_eq!(npc.goal, Goal::Assist(Help::Refuel));
            assert_eq!(npc.docking.map(|docking| docking.sub), Some(DockSub::Close));
            passes += 1;
            assert!(passes < 1000, "the warp never ended");
        }
        // It turns onto the player from where it stopped, then flies.
        let npc = session.space.npc(helper).expect("here");
        assert!(npc.state.heading < at_rest.heading, "it turns to follow");
        assert!(npc.state.velocity.x > 0.0, "no longer zeroed");
        assert!(
            npc.state.position.x > at_rest.position.x + side,
            "it moves after the player"
        );
    }

    #[test]
    fn a_refuel_due_past_the_warps_lead_in_completes_with_its_note_and_sound() {
        let catalog = catalog();
        let (mut session, helper) = refuelled_in_a_jump(&catalog, 200.0, 300.0);
        let npc = session.space.npc_mut(helper).expect("here");
        npc.goal = Goal::Idle;
        npc.target = None;
        past_the_lead_in(&mut session, &catalog, |_| {});
        let _ = session.take_sounds();
        refuelling_from_here(&mut session, helper);
        assert_eq!(
            traffic_tick(&mut session, &catalog),
            [CommNote {
                from: helper,
                comm_name: "Shuttle".to_owned(),
                ship_name: "Kestrel".to_owned(),
                done: Help::Refuel,
            }]
        );
        assert!(took_the_refuels_sound(&mut session));
        let done = session.space.npc(helper).expect("here");
        assert_eq!(done.goal, Goal::Idle, "it decides again");
        assert_eq!(done.target, None);
        assert_eq!(session.jumping(), Some(SystemId(131)), "still in the warp");
    }

    #[test]
    fn a_refuel_passing_100_past_the_warps_lead_in_completes_on_the_next_think() {
        let catalog = catalog();
        let (mut session, helper) = refuelled_in_a_jump(&catalog, 100.0, 300.0);
        let npc = session.space.npc_mut(helper).expect("here");
        npc.goal = Goal::Idle;
        npc.target = None;
        past_the_lead_in(&mut session, &catalog, |_| {});
        let _ = session.take_sounds();
        refuelling_from_here(&mut session, helper);
        assert!(traffic_tick(&mut session, &catalog).is_empty());
        assert_eq!(session.reserves().fuel.now, 101.0);
        assert!(!took_the_refuels_sound(&mut session));
        assert_eq!(traffic_tick(&mut session, &catalog).len(), 1);
        assert_eq!(session.reserves().fuel.now, 101.0);
        assert!(took_the_refuels_sound(&mut session));
        assert_eq!(
            session.space.npc(helper).map(|npc| npc.goal),
            Some(Goal::Idle)
        );
    }
}
