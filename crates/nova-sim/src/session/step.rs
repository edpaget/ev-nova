//! [`Session::step`]: a frame's fixed steps, each in `_DoPlayGameWork`'s
//! order, as the flight view ran them itself before (its `run_steps`).
//!
//! **The frame** (`_PlayGame` @0x45b6a-0x45f31). Once a frame, before
//! its steps, steps or none:
//!
//! 1. the field of a session started or loaded in flight is settled
//!    ([`Session::settle_asteroid_field`]), so the first frame drawn shows
//!    it;
//! 2. the input is held ([`StepInput`]): the play area, the window's
//!    width, the fire keys, the eject and self-destruct chords, the cloak
//!    and weapon safety keys, the double-time key, Hyperspace Effects and
//!    the land key, each a store but the double-time key, which latches
//!    the switch at once (see the `double_time` module).
//!
//! The view settled the field at the top of its tick, before it caught
//! up with the session's events and ran its held keys; nothing there
//! reads the field or its due flag, which is due only as a session
//! starts or loads in flight and is taken even while docked, and the
//! first frame always reaches the steps (the map is shut and no hold has
//! run out). The setters store what they are given, in the view's order;
//! the double-time key's latch reads the heading and the warp-ups, which
//! nothing between the two sites moves.
//!
//! **Each step**, stopped before it once the warp has ended
//! ([`Session::warp_end_due`]: the arrival comes first) or the hold after
//! the player's explosion has run out ([`Session::hold_expired`]):
//!
//! 1. each ship's state is recorded, which the view draws from;
//! 2. the advance pass (`_DoPlayGameWork(1)` @0x43de5-0x4453d,
//!    [`Session::game_work`]), then its comm notes are taken;
//! 3. the NPCs answer what the session left them
//!    ([`Session::answer_strikes`]): the pass answers every hit it lands
//!    as its phase settles, the beams' and shockwaves' included, as
//!    `_DamageShip` does (@0x44385, @0x4453d), so only the session's own
//!    calls are left here, before the frame is drawn. Then the attack
//!    quotes the answers raise are said
//!    (`_ShowPersonHailQuote` @0x89ce1) and `_HandleShip`'s quote roll runs
//!    (@0x33f0f-0x33fba; [`Session::tick_quotes`]), and the pass's quotes,
//!    fight events and ships are reported ([`PassReport`]);
//! 4. in double time, the frame as it stands is reported
//!    ([`StepRun::between_passes`]), as that is what the original draws
//!    (`_PlayGame` @0x45d96-0x45e82), then the second pass
//!    (`_DoPlayGameWork(0)` @0x45f31, [`Session::tick_again`]), the rescue
//!    asked after it
//!    ([`Session::rescue_if_due`]), the strikes answered, the quote roll
//!    on the clock unmoved ([`Session::tick_quotes_again`]), and its own
//!    report. Its comm notes are taken with the step's, so the frame that
//!    raised them relays them.
//! 5. the line the step raised last, whatever its kind
//!    ([`StepRun::last_line`]): `_DisplayComm` replaces its one slot with
//!    each line (@0x90251-0x90265), so the frame ends on the last raised,
//!    and the view shows it last. The frame clears the record before its
//!    first step: a line raised before it (a press the view says at once,
//!    a note or quote the inbox held) is older than any the steps raise.
//!    With it, whether the step's passes cleared an escort class's kept
//!    order ([`StepRun::class_orders_cleared`], see the `class_orders`
//!    module), which the frame clears before its first step too: the
//!    view shows the escort menu in full again after that step's fade.
//!
//! **The rescue.** The escape pod's rescue settles on the pass the pod's
//! countdown runs out (`_HandlePlayer` runs it in place of the rest of
//! that pass's player handling, @0x68ddd) and ends the stepping, so its
//! `dësc` opens before the next step; the view lays itself out where it
//! put the ship (@0x68fab). The pass is not reported, and its quotes and
//! fight events wait for the next frame's take; its comm and its last
//! line are taken all the same, on either pass, so a note shows before
//! the `dësc` opens. A rescue on the second pass reports no frame between
//! the passes, as the view shows the new layout at once.

use crate::ai::Behaviour;
use crate::catalog::{CommCatalog, PilotCatalog, TrafficCatalog};
use crate::chance::Chance;
use crate::combat::{Rules, ShipRef};
#[cfg(doc)]
use crate::flight_frame::RaisedLine;
use crate::flight_frame::{
    FlightFrame, FrameShot, PassReport, ShipPlace, StepInput, StepReport, StepRun,
};

use super::Session;

/// What a frame's steps read of the catalog: the traffic's, the pilot's
/// and the comm's ([`Session::step`]).
pub trait SimCatalog: PilotCatalog + TrafficCatalog + CommCatalog {}

impl<T: PilotCatalog + TrafficCatalog + CommCatalog + ?Sized> SimCatalog for T {}

/// What a frame's steps are handed ([`Session::step`]).
pub struct StepCollaborators<'a> {
    /// The catalog the traffic, the rescue and the quotes read.
    pub catalog: &'a dyn SimCatalog,
    /// How the NPCs decide and answer.
    pub behaviour: &'a dyn Behaviour,
    /// The rules the fight is fought by.
    pub rules: Rules<'a>,
    /// What the steps draw on.
    pub chance: &'a mut dyn Chance,
}

impl Session {
    /// Runs up to `steps` fixed steps under `input`, handed `collab`, and
    /// reports each into `into`, filled afresh (see the module docs).
    pub fn step(
        &mut self,
        input: &StepInput,
        steps: u32,
        collab: &mut StepCollaborators<'_>,
        into: &mut StepReport,
    ) {
        into.clear();
        self.settle_asteroid_field(&mut *collab.chance);
        self.hold_input(input);
        self.out.last_line = None;
        self.out.class_orders_cleared = false;
        for _ in 0..steps {
            if self.warp_end_due() || self.hold_expired() {
                into.stopped = true;
                return;
            }
            let run = self.run_step(input, collab);
            let rescued = run.rescued.is_some();
            into.steps.push(run);
            if rescued {
                into.stopped = true;
                return;
            }
        }
    }

    /// Holds `input`'s holds and settings, in the order the view set them.
    fn hold_input(&mut self, input: &StepInput) {
        self.set_play_area(input.play_area);
        self.set_window_width(input.window_width);
        self.hold_fire(input.fire.0, input.fire.1);
        self.hold_eject(input.eject);
        self.hold_self_destruct(input.self_destruct);
        self.hold_cloak(input.cloak);
        self.hold_weapon_safety(input.weapon_safety);
        self.set_double_time_key(input.double_time_key);
        self.set_hyperspace_effects(input.hyperspace_effects);
        self.set_land_key(input.land_key);
    }

    /// One step under `input`, handed `collab` (see the module docs).
    fn run_step(&mut self, input: &StepInput, collab: &mut StepCollaborators<'_>) -> StepRun {
        let catalog = &collab.catalog;
        let mut run = StepRun {
            before: self.ships_before(),
            ..StepRun::default()
        };
        let rescued = self.game_work(
            input.controls,
            catalog,
            collab.behaviour,
            collab.rules,
            &mut *collab.chance,
        );
        run.comm = self.take_comm();
        if rescued {
            run.rescued = Some(ShipPlace::from(*self.player()));
            self.take_step_ends(&mut run);
            return run;
        }
        self.answer_strikes(collab.behaviour);
        self.tick_quotes(catalog, &mut *collab.chance);
        run.passes.push(self.pass_report());
        if !self.double_time() {
            self.take_step_ends(&mut run);
            return run;
        }
        run.between_passes = Some(self.frame_between_passes());
        self.tick_again(
            input.controls,
            collab.behaviour,
            collab.rules,
            &mut *collab.chance,
        );
        #[cfg(test)]
        for note in std::mem::take(&mut self.out.second_pass_comm) {
            self.space
                .raise_line(&mut self.out, crate::flight_frame::RaisedLine::Comm(note));
        }
        if self.rescue_if_due(catalog, &mut *collab.chance) {
            run.between_passes = None;
            run.comm.extend(self.take_comm());
            run.rescued = Some(ShipPlace::from(*self.player()));
            self.take_step_ends(&mut run);
            return run;
        }
        self.answer_strikes(collab.behaviour);
        self.tick_quotes_again(catalog, &mut *collab.chance);
        run.passes.push(self.pass_report());
        run.comm.extend(self.take_comm());
        self.take_step_ends(&mut run);
        run
    }

    /// The flight as it stands, between double time's two passes.
    fn frame_between_passes(&self) -> Box<FlightFrame> {
        let mut frame = Box::<FlightFrame>::default();
        self.fill_frame(&mut frame);
        frame
    }

    /// What `run`'s step leaves at its end, taken into it: the line it
    /// raised last, and whether it cleared an escort class's kept order
    /// (see the module docs). A wrap a rescue's unreported pass left is
    /// dropped, as the view lays itself out afresh.
    fn take_step_ends(&mut self, run: &mut StepRun) {
        self.out.player_wrap = None;
        run.last_line = self.out.last_line.take();
        run.class_orders_cleared = std::mem::take(&mut self.out.class_orders_cleared);
    }

    /// Each ship's place: the player's, then each NPC's.
    fn ships_before(&self) -> Vec<(ShipRef, ShipPlace)> {
        let npcs = self
            .npcs()
            .iter()
            .map(|npc| (ShipRef::Npc(npc.id), ShipPlace::from(npc.state)));
        std::iter::once((ShipRef::Player, ShipPlace::from(*self.player())))
            .chain(npcs)
            .collect()
    }

    /// The pass just run, as it stands, its quotes and fight events
    /// taken: each ship's record as the frame builds it
    /// (`Session::fill_ships`), and each shot's in the fight's order.
    fn pass_report(&mut self) -> PassReport {
        let mut ships = Vec::new();
        self.fill_ships(&mut ships);
        let shots = self.shots().iter().map(FrameShot::of).collect();
        PassReport {
            ships,
            shots,
            beam_weapons: self.beams().iter().map(|beam| beam.weapon.id).collect(),
            combat: self.take_combat_events(),
            smoke_frame: self.smoke_frame(),
            system_murk: self.system_murk(),
            player_position: self.player().position,
            double_time: self.double_time(),
            quotes: self.take_quotes(),
            wrap: self.out.player_wrap.take(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::combat::CombatEvent;
    use crate::flight::Controls;
    use crate::flight_frame::RaisedLine;
    use crate::geometry::Vec2;
    use crate::hail::CommNote;
    use crate::player_wrap::{PlayerWrap, WRAP_EDGE};
    use crate::session::PersonQuote;
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::{FakePilotCatalog, catalog};

    /// The input of a frame with the primary fire key held, and double
    /// time's key as `double`.
    fn firing(double: bool) -> StepInput {
        StepInput {
            fire: (true, false),
            double_time_key: double,
            window_width: 1024,
            ..StepInput::default()
        }
    }

    /// `catalog`'s collaborators: Nova's AI and rules, the chance that
    /// never fires.
    fn steps(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        input: &StepInput,
        count: u32,
    ) -> StepReport {
        let behaviour = crate::ai::NovaAi::default();
        let mut chance = NeverFires;
        let mut collab = StepCollaborators {
            catalog,
            behaviour: &behaviour,
            rules: Rules::default(),
            chance: &mut chance,
        };
        let mut report = StepReport::default();
        session.step(input, count, &mut collab, &mut report);
        report
    }

    /// What a pass took by hand: its quotes and fight events.
    type Took = (Vec<PersonQuote>, Vec<CombatEvent>);

    /// What a step took by hand: its comm, each pass's takes, the line it
    /// raised last, and whether it cleared a class's kept order.
    type StepTook = (Vec<CommNote>, Vec<Took>, Option<RaisedLine>, bool);

    /// The frame's calls made by hand, in the order the view made them,
    /// on the same collaborators: each step's comm, then each pass's
    /// takes, the comm again after a second pass, and the line raised
    /// last.
    fn by_hand(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        input: &StepInput,
        count: u32,
    ) -> Vec<StepTook> {
        let behaviour = crate::ai::NovaAi::default();
        let chance = &mut NeverFires;
        session.settle_asteroid_field(chance);
        session.set_play_area(input.play_area);
        session.set_window_width(input.window_width);
        session.hold_fire(input.fire.0, input.fire.1);
        session.hold_eject(input.eject);
        session.hold_self_destruct(input.self_destruct);
        session.hold_cloak(input.cloak);
        session.hold_weapon_safety(input.weapon_safety);
        session.set_double_time_key(input.double_time_key);
        session.set_hyperspace_effects(input.hyperspace_effects);
        session.set_land_key(input.land_key);
        session.out.last_line = None;
        session.out.class_orders_cleared = false;
        let mut took = Vec::new();
        for _ in 0..count {
            if session.warp_end_due() || session.hold_expired() {
                break;
            }
            let rescued = session.game_work(
                input.controls,
                catalog,
                &behaviour,
                Rules::default(),
                chance,
            );
            let mut comm = session.take_comm();
            let mut passes = Vec::new();
            if rescued {
                took.push(step_took(session, comm, passes));
                break;
            }
            session.answer_strikes(&behaviour);
            session.tick_quotes(catalog, chance);
            passes.push((session.take_quotes(), session.take_combat_events()));
            if session.double_time() {
                session.tick_again(input.controls, &behaviour, Rules::default(), chance);
                for note in std::mem::take(&mut session.out.second_pass_comm) {
                    session
                        .space
                        .raise_line(&mut session.out, RaisedLine::Comm(note));
                }
                if session.rescue_if_due(catalog, chance) {
                    comm.extend(session.take_comm());
                    took.push(step_took(session, comm, passes));
                    break;
                }
                session.answer_strikes(&behaviour);
                session.tick_quotes_again(catalog, chance);
                passes.push((session.take_quotes(), session.take_combat_events()));
                comm.extend(session.take_comm());
            }
            took.push(step_took(session, comm, passes));
        }
        took
    }

    /// What a step took by hand, its `comm` and its `passes`' takes: then
    /// the line it raised last and whether it cleared a class.
    fn step_took(session: &mut Session, comm: Vec<CommNote>, passes: Vec<Took>) -> StepTook {
        let line = session.out.last_line.take();
        let cleared = std::mem::take(&mut session.out.class_orders_cleared);
        (comm, passes, line, cleared)
    }

    /// What `report` took: each step's comm, then each pass's quotes and
    /// fight events, the line it raised last, and whether it cleared a
    /// class.
    fn took(report: &StepReport) -> Vec<StepTook> {
        report
            .steps
            .iter()
            .map(|run| {
                let passes = run
                    .passes
                    .iter()
                    .map(|pass| (pass.quotes.clone(), pass.combat.clone()))
                    .collect();
                (
                    run.comm.clone(),
                    passes,
                    run.last_line.clone(),
                    run.class_orders_cleared,
                )
            })
            .collect()
    }

    /// [`armed`]'s fight: the NPC 100 pixels ahead, facing the player.
    fn fight() -> (FakePilotCatalog, Session) {
        let catalog = armed();
        let session = facing_an_npc(&catalog, 180);
        (catalog, session)
    }

    #[test]
    fn a_frames_steps_make_the_calls_the_view_made_in_their_order() {
        for double in [false, true] {
            let (catalog, mut session) = fight();
            let mut hand = session.clone();
            let input = firing(double);
            let report = steps(&mut session, &catalog, &input, 30);
            let expected = by_hand(&mut hand, &catalog, &input, 30);
            assert_eq!(session, hand, "double time {double}");
            assert_eq!(took(&report), expected, "double time {double}");
            assert_eq!(report.steps.len(), 30);
            assert!(!report.stopped);
            let hits = report
                .steps
                .iter()
                .flat_map(|run| &run.passes)
                .flat_map(|pass| &pass.combat)
                .filter(|event| matches!(event, CombatEvent::Hit { .. }))
                .count();
            assert!(hits > 0, "a fight, double time {double}");
        }
    }

    #[test]
    fn a_step_runs_a_second_pass_only_in_double_time() {
        for (double, passes) in [(false, 1), (true, 2)] {
            let (catalog, mut session) = fight();
            let report = steps(&mut session, &catalog, &firing(double), 3);
            let counts: Vec<_> = report.steps.iter().map(|run| run.passes.len()).collect();
            assert_eq!(counts, [passes; 3], "double time {double}");
            let doubled = report.steps[0].passes.iter().all(|pass| pass.double_time);
            assert_eq!(doubled, double);
        }
    }

    #[test]
    fn with_no_step_the_input_is_held_and_the_field_settled() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let mut hand = session.clone();
        let input = StepInput {
            fire: (true, true),
            eject: true,
            self_destruct: true,
            cloak: true,
            weapon_safety: true,
            double_time_key: true,
            hyperspace_effects: true,
            land_key: true,
            window_width: 640,
            play_area: crate::cloak::Area {
                left: -1.0,
                top: -2.0,
                right: 3.0,
                bottom: 4.0,
            },
            ..StepInput::default()
        };
        let report = steps(&mut session, &catalog, &input, 0);
        assert_eq!(report, StepReport::default());
        by_hand(&mut hand, &catalog, &input, 0);
        assert_eq!(session, hand);
        assert_ne!(session, Session::start(&catalog).expect("starts"));
        assert!(!session.space.asteroid_field_due, "settled");
    }

    #[test]
    fn each_step_records_every_ship_as_it_stood_before_it() {
        let (catalog, mut session) = fight();
        let mut hand = session.clone();
        let input = StepInput {
            controls: Controls {
                thrust: true,
                ..Controls::default()
            },
            ..firing(true)
        };
        let report = steps(&mut session, &catalog, &input, 3);
        for run in &report.steps {
            let place = |state: &crate::flight::ShipState| ShipPlace {
                position: state.position,
                heading: state.heading,
            };
            let mut before = vec![(ShipRef::Player, place(hand.player()))];
            before.extend(
                hand.npcs()
                    .iter()
                    .map(|npc| (ShipRef::Npc(npc.id), place(&npc.state))),
            );
            assert_eq!(run.before, before);
            by_hand(&mut hand, &catalog, &input, 1);
        }
        assert_ne!(report.steps[0].before, report.steps[2].before, "moved");
    }

    #[test]
    fn each_pass_reports_the_session_as_it_stood_after_it() {
        for double in [false, true] {
            let (catalog, mut session) = fight();
            let input = firing(double);
            // Each pass's records, as the session stands after it.
            let mut expected = Vec::new();
            let mut hand = session.clone();
            let behaviour = crate::ai::NovaAi::default();
            let chance = &mut NeverFires;
            by_hand(&mut hand, &catalog, &input, 0);
            for _ in 0..30 {
                hand.game_work(
                    input.controls,
                    &catalog,
                    &behaviour,
                    Rules::default(),
                    chance,
                );
                hand.take_comm();
                hand.answer_strikes(&behaviour);
                hand.tick_quotes(&catalog, chance);
                expected.push(records(&mut hand));
                if hand.double_time() {
                    hand.tick_again(input.controls, &behaviour, Rules::default(), chance);
                    hand.rescue_if_due(&catalog, chance);
                    hand.answer_strikes(&behaviour);
                    hand.tick_quotes_again(&catalog, chance);
                    expected.push(records(&mut hand));
                }
            }
            let report = steps(&mut session, &catalog, &input, 30);
            let passes: Vec<_> = report
                .steps
                .iter()
                .flat_map(|run| run.passes.iter().cloned())
                .collect();
            assert_eq!(passes, expected, "double time {double}");
            assert!(passes.iter().any(|pass| !pass.shots.is_empty()), "fired");
        }
    }

    #[test]
    fn a_beams_late_hit_is_reported_in_the_pass_it_lands_in() {
        // Guidance 0, a beam.
        let mut catalog = armed();
        catalog.weapons[0].guidance = 0;
        catalog.weapons[0].beam_length = 400;
        let mut session = facing_an_npc(&catalog, 180);
        let mut hand = session.clone();
        let input = firing(true);
        let report = steps(&mut session, &catalog, &input, 30);
        assert_eq!(took(&report), by_hand(&mut hand, &catalog, &input, 30));
        let passes: Vec<_> = report.steps.iter().flat_map(|run| &run.passes).collect();
        let late = passes.iter().any(|pass| {
            pass.combat
                .iter()
                .any(|event| matches!(event, CombatEvent::Hit { late: true, .. }))
        });
        assert!(late, "a beam's hit shows late");
        assert!(
            passes
                .iter()
                .any(|pass| pass.beam_weapons == [catalog.weapons[0].id]),
            "its beam reported"
        );
    }

    #[test]
    fn a_shot_that_lives_one_pass_is_reported_in_that_pass_alone() {
        // 90 pixels a tick: a shot reaches the NPC on the pass after its
        // firing.
        let mut catalog = armed();
        catalog.weapons[0].speed = 9000;
        let mut session = facing_an_npc(&catalog, 180);
        // By the engine's landing box the NPC, idling near planet 128,
        // would land on the first pass, before any shot reached it.
        crate::session::tests::landing_by_circle(&mut session);
        let report = steps(&mut session, &catalog, &firing(true), 60);
        let shots =
            |pass: &PassReport| -> Vec<_> { pass.shots.iter().map(|shot| shot.id).collect() };
        let passes: Vec<_> = report.steps.iter().flat_map(|run| &run.passes).collect();
        let once = passes.windows(3).any(|three| {
            shots(three[1])
                .iter()
                .any(|id| !shots(three[0]).contains(id) && !shots(three[2]).contains(id))
        });
        assert!(once, "a shot reported in one pass alone");
    }

    #[test]
    fn an_attack_quote_is_said_on_the_pass_of_its_hit() {
        let catalog = crate::session::tests::quoting_armed();
        let mut session = facing_an_npc(&catalog, 180);
        crate::session::tests::make_quoting_warship(&mut session);
        for _ in 0..60 {
            session.chart.target = Some(crate::traffic::npc::NpcId(0));
            let report = steps(&mut session, &catalog, &firing(false), 1);
            let pass = &report.steps[0].passes[0];
            let hit = pass.combat.iter().any(|event| {
                matches!(event, CombatEvent::Hit { ship, .. } if *ship == ShipRef::Npc(crate::traffic::npc::NpcId(0)))
            });
            if hit {
                let said: Vec<_> = pass
                    .quotes
                    .iter()
                    .map(|quote| quote.text.as_str())
                    .collect();
                assert_eq!(said, ["Prepare to die!"]);
                assert_eq!(
                    report.steps[0].last_line,
                    Some(RaisedLine::Quote(pass.quotes[0].clone())),
                    "the quote raised last"
                );
                return;
            }
            assert_eq!(pass.quotes, [], "nothing said before the hit");
        }
        panic!("never hit");
    }

    #[test]
    fn the_comm_raised_after_a_steps_take_comes_with_the_next_steps() {
        let (catalog, mut session) = fight();
        steps(&mut session, &catalog, &firing(true), 1);
        let note = CommNote {
            from: crate::traffic::npc::NpcId(0),
            comm_name: "Ally".to_owned(),
            ship_name: "Shuttle".to_owned(),
            done: crate::hail::Help::Repair,
        };
        session.out.comm.push(note.clone());
        let report = steps(&mut session, &catalog, &firing(true), 2);
        assert_eq!(report.steps[0].comm, [note]);
        assert_eq!(report.steps[1].comm, []);
        assert_eq!(session.take_comm(), [], "all taken");
    }

    #[test]
    fn the_quotes_said_before_a_pass_come_with_that_pass() {
        let (catalog, mut session) = fight();
        let quote = PersonQuote {
            npc: crate::traffic::npc::NpcId(0),
            text: "Halt!".to_owned(),
        };
        session.out.quotes.push(quote.clone());
        let report = steps(&mut session, &catalog, &firing(true), 1);
        let quotes: Vec<_> = report.steps[0]
            .passes
            .iter()
            .map(|pass| pass.quotes.clone())
            .collect();
        assert_eq!(quotes, [vec![quote], vec![]]);
    }

    /// `catalog`'s pilot in the escape pod, its countdown running out on
    /// the `passes`th pass from now.
    fn in_the_pod(catalog: &FakePilotCatalog, passes: u32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.pilot.ship = crate::combat::hull::ESCAPE_POD_SHIP;
        session.pod = Some(passes);
        session
    }

    #[test]
    fn a_rescue_on_the_advance_pass_ends_the_stepping_with_its_comm_taken() {
        let catalog = catalog();
        let mut session = in_the_pod(&catalog, 1);
        let quote = PersonQuote {
            npc: crate::traffic::npc::NpcId(0),
            text: "Halt!".to_owned(),
        };
        session.out.quotes.push(quote.clone());
        let note = CommNote {
            from: crate::traffic::npc::NpcId(0),
            comm_name: "Ally".to_owned(),
            ship_name: "Shuttle".to_owned(),
            done: crate::hail::Help::Repair,
        };
        session.out.comm.push(note.clone());
        let report = steps(&mut session, &catalog, &StepInput::default(), 3);
        assert_eq!(report.steps.len(), 1, "no later step");
        assert!(report.stopped);
        let run = &report.steps[0];
        assert_eq!(run.passes, [], "the pass is not reported");
        assert_eq!(run.comm, [note], "its comm taken");
        let player = session.player();
        assert_eq!(
            run.rescued,
            Some(ShipPlace {
                position: player.position,
                heading: player.heading,
            })
        );
        assert_eq!(session.take_quotes(), [quote], "its quotes not taken");
        assert_eq!(session.take_rescue_desc(), Some(13999), "rescued");
    }

    #[test]
    fn a_line_raised_on_the_advance_pass_before_its_rescue_comes_with_its_step() {
        // A fighter of no fuel out of its bay as a new jump is accepted:
        // the pass's player part warns before the rescue settles.
        use crate::session::wait_for_me::tests::{DART, catalog, fighter};
        let catalog = catalog();
        let mut session = crate::session::fighters::tests::fleet(&catalog, 0, vec![fighter(DART)]);
        session.pilot.ship = crate::combat::hull::ESCAPE_POD_SHIP;
        session.pod = Some(1);
        session.ship.warp.jump_accepted = true;
        let mut hand = session.clone();
        let input = StepInput::default();
        let report = steps(&mut session, &catalog, &input, 3);
        assert_eq!(took(&report), by_hand(&mut hand, &catalog, &input, 3));
        assert_eq!(session, hand);
        assert!(report.stopped);
        let run = &report.steps[0];
        assert!(run.rescued.is_some(), "rescued on the advance pass");
        assert!(
            matches!(
                run.last_line,
                Some(RaisedLine::Message(crate::SimMessage::WaitForMe(_)))
            ),
            "the warning raised last: {:?}",
            run.last_line
        );
        assert_eq!(session.out.last_line, None, "taken with its step");
    }

    #[test]
    fn a_rescue_on_the_second_pass_ends_the_stepping_with_its_comm_taken() {
        let catalog = catalog();
        let mut session = in_the_pod(&catalog, 2);
        let input = StepInput {
            double_time_key: true,
            ..StepInput::default()
        };
        let mut hand = session.clone();
        let report = steps(&mut session, &catalog, &input, 3);
        let expected = by_hand(&mut hand, &catalog, &input, 3);
        assert_eq!(session, hand);
        assert_eq!(took(&report), expected);
        assert_eq!(report.steps.len(), 1, "no later step");
        assert!(report.stopped);
        let run = &report.steps[0];
        assert_eq!(run.passes.len(), 1, "the advance pass alone");
        assert_eq!(run.between_passes, None, "the view lays out afresh");
        let player = session.player();
        assert_eq!(
            run.rescued,
            Some(ShipPlace {
                position: player.position,
                heading: player.heading,
            })
        );
        let mut session = in_the_pod(&catalog, 2);
        steps(&mut session, &catalog, &input, 0);
        session.ship.fate.rescue_due = false;
        let mut copy = session.clone();
        copy.out.second_pass_comm.push(repaired());
        let report = steps(&mut copy, &catalog, &input, 1);
        assert_eq!(
            copy.take_rescue_desc(),
            Some(13999),
            "rescued on the second pass"
        );
        assert_eq!(report.steps[0].comm, [repaired()], "taken with its step");
        assert_eq!(
            report.steps[0].last_line,
            Some(RaisedLine::Comm(repaired()))
        );
        assert_eq!(copy.take_comm(), [], "none held for the next take");
    }

    /// An ally's note that it has repaired the player.
    fn repaired() -> CommNote {
        CommNote {
            from: crate::traffic::npc::NpcId(0),
            comm_name: "Ally".to_owned(),
            ship_name: "Shuttle".to_owned(),
            done: crate::hail::Help::Repair,
        }
    }

    #[test]
    fn comm_raised_on_the_second_pass_comes_with_its_step() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let input = StepInput {
            double_time_key: true,
            ..StepInput::default()
        };
        steps(&mut session, &catalog, &input, 1);
        session.out.second_pass_comm.push(repaired());
        let report = steps(&mut session, &catalog, &input, 1);
        assert_eq!(report.steps[0].passes.len(), 2, "both passes");
        assert_eq!(report.steps[0].comm, [repaired()], "taken with its step");
        let report = steps(&mut session, &catalog, &input, 1);
        assert_eq!(report.steps[0].comm, [], "none left for the next");
    }

    #[test]
    fn the_line_a_step_raised_last_comes_with_that_step() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session
            .space
            .say(&mut session.out, crate::SimMessage::NoActiveMissions);
        let report = steps(&mut session, &catalog, &StepInput::default(), 2);
        assert_eq!(report.steps[0].last_line, None, "said before the frame");
        assert_eq!(report.steps[1].last_line, None);
        let input = StepInput {
            double_time_key: true,
            ..StepInput::default()
        };
        steps(&mut session, &catalog, &input, 1);
        session.out.second_pass_comm.push(repaired());
        let report = steps(&mut session, &catalog, &input, 2);
        assert_eq!(report.steps[0].comm, [repaired()]);
        assert_eq!(
            report.steps[0].last_line,
            Some(RaisedLine::Comm(repaired())),
            "raised on the second pass"
        );
        assert_eq!(report.steps[1].last_line, None, "taken with its step");
    }

    /// A session of [`carrying`](crate::session::fighters::tests::carrying)'s
    /// with a warship escort flying, given Return to Hangar, which the
    /// next pass clears (see the `class_orders` module).
    fn returning_warship() -> (FakePilotCatalog, Session) {
        use crate::escort::{EscortClass, EscortCommand, EscortGroup};
        use crate::session::fighters::tests::{carrying, fleet, warship};
        let catalog = carrying();
        let mut session = fleet(&catalog, 0, vec![warship(None)]);
        session.command_escorts(
            EscortGroup::Class(EscortClass::Warship),
            EscortCommand::Dock,
            &mut NeverFires,
        );
        (catalog, session)
    }

    #[test]
    fn a_class_cleared_is_reported_with_the_step_whose_pass_cleared_it() {
        for double in [false, true] {
            let (catalog, mut session) = returning_warship();
            let mut hand = session.clone();
            let input = StepInput {
                double_time_key: double,
                ..StepInput::default()
            };
            let report = steps(&mut session, &catalog, &input, 2);
            let cleared: Vec<bool> = report
                .steps
                .iter()
                .map(|run| run.class_orders_cleared)
                .collect();
            assert_eq!(cleared, [true, false], "double time {double}");
            assert_eq!(took(&report), by_hand(&mut hand, &catalog, &input, 2));
            assert_eq!(session, hand);
            assert!(!session.out.class_orders_cleared, "taken");
        }
    }

    #[test]
    fn a_clear_before_the_frame_is_not_reported_with_its_steps() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.out.class_orders_cleared = true;
        let report = steps(&mut session, &catalog, &StepInput::default(), 1);
        assert!(!report.steps[0].class_orders_cleared);
    }

    #[test]
    fn a_clear_before_a_rescue_on_the_second_pass_comes_with_its_step() {
        let catalog = catalog();
        let mut session = in_the_pod(&catalog, 2);
        session.space.escorts.class_orders[2] = Some(crate::escort::EscortOrder::Hold);
        let input = StepInput {
            double_time_key: true,
            ..StepInput::default()
        };
        let report = steps(&mut session, &catalog, &input, 3);
        assert_eq!(report.steps.len(), 1, "rescued");
        assert!(report.steps[0].rescued.is_some());
        assert!(report.steps[0].class_orders_cleared);
        assert!(!session.out.class_orders_cleared, "taken");
    }

    #[test]
    fn the_warps_end_stops_the_stepping_before_the_next_step() {
        let catalog = catalog();
        let mut session = crate::session::tests::bound_for_131(
            &catalog,
            crate::session::tests::OUT,
            crate::geometry::Vec2::ZERO,
            90.0,
        );
        session.begin_jump().expect("jumps");
        let mut ran = 0;
        let mut probe = session.clone();
        while !probe.warp_end_due() {
            steps(&mut probe, &catalog, &StepInput::default(), 1);
            ran += 1;
            assert!(ran < 2000, "the warp never ends");
        }
        let report = steps(&mut session, &catalog, &StepInput::default(), ran + 5);
        assert_eq!(report.steps.len(), ran as usize);
        assert!(report.stopped);
        assert!(session.warp_end_due());
    }

    #[test]
    fn the_holds_expiry_stops_the_stepping_before_the_next_step() {
        let (catalog, mut session) = fight();
        let report = steps(&mut session, &catalog, &StepInput::default(), 1);
        assert!(!report.stopped);
        session.ship.condition = crate::combat::hull::Condition::Destroyed;
        session.player_end = Some(crate::player_fate::Ending::Expired);
        let mut held = session.clone();
        let report = steps(&mut session, &catalog, &StepInput::default(), 2);
        assert_eq!(report.steps, []);
        assert!(report.stopped);
        by_hand(&mut held, &catalog, &StepInput::default(), 0);
        assert_eq!(session, held, "no step ran");
    }

    /// `session`'s records after a pass, its quotes and fight events
    /// taken, as [`PassReport`] has them.
    fn records(session: &mut Session) -> PassReport {
        let mut ships = vec![super::super::frame::tests::player_record(session)];
        ships.extend(
            session
                .npcs()
                .iter()
                .map(|npc| super::super::frame::tests::npc_record(session, npc)),
        );
        let shots = session
            .shots()
            .iter()
            .map(|shot| FrameShot {
                id: shot.id,
                weapon: shot.weapon.id,
                position: shot.position,
                velocity: shot.velocity,
                heading: shot.heading,
                age: shot.age,
                spin_timer: shot.spin_timer,
                layer: shot.layer,
            })
            .collect();
        PassReport {
            ships,
            shots,
            beam_weapons: session.beams().iter().map(|beam| beam.weapon.id).collect(),
            smoke_frame: session.smoke_frame(),
            system_murk: session.system_murk(),
            player_position: session.player().position,
            double_time: session.double_time(),
            quotes: session.take_quotes(),
            combat: session.take_combat_events(),
            wrap: session.out.player_wrap.take(),
        }
    }

    // The wrap (see the `player_wrap` module).

    /// [`catalog`]'s session flown a pass, the ship at `at` at `velocity`.
    fn flying_at(catalog: &FakePilotCatalog, at: Vec2, velocity: Vec2) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        steps(&mut session, catalog, &StepInput::default(), 1);
        session.ship.player.position = at;
        session.ship.player.velocity = velocity;
        session
    }

    #[test]
    fn a_double_time_step_reports_the_frame_between_its_passes() {
        let catalog = catalog();
        let at = Vec2::new(100.0, 0.0);
        let mut session = flying_at(&catalog, at, Vec2::new(2.0, 0.0));
        let input = StepInput {
            double_time_key: true,
            ..StepInput::default()
        };
        session.set_double_time_key(true);
        let report = steps(&mut session, &catalog, &input, 1);
        let run = &report.steps[0];
        let first = &run.passes[0];
        let frame = run.between_passes.as_ref().expect("reported");
        assert_eq!(frame.ships, first.ships);
        assert_eq!(frame.player().position, first.player_position);
        assert_ne!(frame.player().position, session.player().position);

        let mut session = flying_at(&catalog, at, Vec2::new(2.0, 0.0));
        let report = steps(&mut session, &catalog, &StepInput::default(), 1);
        assert_eq!(report.steps[0].between_passes, None, "single time");
    }

    #[test]
    fn a_warp_end_on_the_second_pass_reports_a_frame_not_yet_arrived() {
        use crate::session::warp_stage::tests::{frames, jumping_in_double_time, with_a_warp_out};
        use crate::sound::{SimSound, WARP_UP_HALTS};
        let catalog = with_a_warp_out();
        let mut session = jumping_in_double_time(&catalog);
        frames(&mut session, 96);
        session.take_sounds();
        let input = StepInput {
            double_time_key: true,
            ..StepInput::default()
        };
        let report = steps(&mut session, &catalog, &input, 1);
        let sounds = session.take_sounds();
        assert!(
            sounds.contains(&SimSound::Halt(&WARP_UP_HALTS)) && sounds.contains(&SimSound::Arrived),
            "the halt and the warp-out raised with that step: {sounds:?}"
        );
        let frame = report.steps[0].between_passes.as_ref().expect("reported");
        assert!(!frame.warp_end_due, "the frame between its passes");
        assert!(session.warp_end_due(), "the second pass ended the warp");
        let next = steps(&mut session, &catalog, &input, 1);
        assert_eq!(next.steps, []);
        assert!(next.stopped);
    }

    #[test]
    fn a_natural_warp_end_on_the_advance_pass_is_in_the_frame_between_its_passes() {
        // w 0.91: over on frame 115's advance pass, its warp-up stepped
        // out at the pass's head.
        use crate::session::warp_stage::tests::{flagged, frames, jumping_in_double_time};
        let catalog = flagged(0x0001);
        let mut session = jumping_in_double_time(&catalog);
        frames(&mut session, 114);
        assert!(!session.warp_end_due(), "frame 114");
        let input = StepInput {
            double_time_key: true,
            ..StepInput::default()
        };
        let report = steps(&mut session, &catalog, &input, 1);
        let frame = report.steps[0].between_passes.as_ref().expect("reported");
        assert!(frame.warp_end_due, "the view arrives on that tick");
    }

    #[test]
    fn a_wrap_is_on_the_report_of_the_advance_pass_it_ran_on_alone() {
        let catalog = catalog();
        let past = Vec2::new(WRAP_EDGE + 1.0, 0.0);
        let mut session = flying_at(&catalog, past, Vec2::ZERO);
        let report = steps(&mut session, &catalog, &StepInput::default(), 2);
        let wraps: Vec<_> = report.steps.iter().map(|run| run.passes[0].wrap).collect();
        assert_eq!(wraps, [PlayerWrap::of(past), None]);
    }

    #[test]
    fn a_wrap_is_on_the_report_of_double_times_second_pass_when_it_ran_there() {
        let catalog = catalog();
        let near = Vec2::new(WRAP_EDGE - 1.0, 0.0);
        let mut session = flying_at(&catalog, near, Vec2::new(2.0, 0.0));
        let input = StepInput {
            double_time_key: true,
            ..StepInput::default()
        };
        session.set_double_time_key(true);
        let report = steps(&mut session, &catalog, &input, 2);
        let wraps: Vec<_> = report.steps[0].passes.iter().map(|p| p.wrap).collect();
        assert_eq!(wraps, [None, PlayerWrap::of(near + Vec2::new(2.0, 0.0))]);
        assert!(report.steps[1].passes.iter().all(|p| p.wrap.is_none()));
    }

    #[test]
    fn a_rescues_dropped_wrap_reaches_no_report() {
        let catalog = catalog();
        let mut session = in_the_pod(&catalog, 1);
        session.ship.player.position = Vec2::new(WRAP_EDGE + 1.0, 0.0);
        let report = steps(&mut session, &catalog, &StepInput::default(), 1);
        assert!(report.steps[0].rescued.is_some());
        assert_eq!(session.out.player_wrap, None, "dropped");
    }
}
