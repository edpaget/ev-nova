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
//! 3. the NPCs answer what the pass's close struck
//!    ([`Session::answer_strikes`]): the pass answers its collision
//!    passes' and shots' hits as they land, as `_HandleShot` does
//!    (@0x44385), and the beams' and shockwaves' here, before the frame is
//!    drawn. Then the attack quotes the answers raise are said
//!    (`_ShowPersonHailQuote` @0x89ce1) and `_HandleShip`'s quote roll runs
//!    (@0x33f0f-0x33fba; [`Session::tick_quotes`]), and the pass's quotes,
//!    fight events and ships are reported ([`PassReport`]);
//! 4. in double time, the second pass (`_DoPlayGameWork(0)` @0x45f31,
//!    [`Session::tick_again`]), the rescue asked after it
//!    ([`Session::rescue_if_due`]), the strikes answered, the quote roll
//!    on the clock unmoved ([`Session::tick_quotes_again`]), and its own
//!    report. Its comm notes wait for the next step's take.
//!
//! **The rescue.** The escape pod's rescue settles on the pass the pod's
//! countdown runs out (`_HandlePlayer` runs it in place of the rest of
//! that pass's player handling, @0x68ddd) and ends the stepping, so its
//! `dësc` opens before the next step; the view lays itself out where it
//! put the ship (@0x68fab). The pass is not reported, and its quotes and
//! fight events wait for the next frame's take; an advance pass's comm
//! is taken all the same.

use crate::ai::Behaviour;
use crate::catalog::{CommCatalog, PilotCatalog, TrafficCatalog};
use crate::chance::Chance;
use crate::combat::{Rules, ShipRef};
use crate::flight_frame::{FrameShot, PassReport, ShipPlace, StepInput, StepReport, StepRun};

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
            return run;
        }
        self.answer_strikes(collab.behaviour);
        self.tick_quotes(catalog, &mut *collab.chance);
        run.passes.push(self.pass_report());
        if !self.double_time() {
            return run;
        }
        self.tick_again(
            input.controls,
            collab.behaviour,
            collab.rules,
            &mut *collab.chance,
        );
        #[cfg(test)]
        {
            let raised = std::mem::take(&mut self.out.second_pass_comm);
            self.out.comm.extend(raised);
        }
        if self.rescue_if_due(catalog, &mut *collab.chance) {
            run.rescued = Some(ShipPlace::from(*self.player()));
            return run;
        }
        self.answer_strikes(collab.behaviour);
        self.tick_quotes_again(catalog, &mut *collab.chance);
        run.passes.push(self.pass_report());
        run
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::combat::CombatEvent;
    use crate::flight::Controls;
    use crate::hail::CommNote;
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

    /// The frame's calls made by hand, in the order the view made them,
    /// on the same collaborators: each step's comm, then each pass's
    /// takes.
    fn by_hand(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        input: &StepInput,
        count: u32,
    ) -> Vec<(Vec<CommNote>, Vec<Took>)> {
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
            let comm = session.take_comm();
            let mut passes = Vec::new();
            if rescued {
                took.push((comm, passes));
                break;
            }
            session.answer_strikes(&behaviour);
            session.tick_quotes(catalog, chance);
            passes.push((session.take_quotes(), session.take_combat_events()));
            if session.double_time() {
                session.tick_again(input.controls, &behaviour, Rules::default(), chance);
                if session.rescue_if_due(catalog, chance) {
                    took.push((comm, passes));
                    break;
                }
                session.answer_strikes(&behaviour);
                session.tick_quotes_again(catalog, chance);
                passes.push((session.take_quotes(), session.take_combat_events()));
            }
            took.push((comm, passes));
        }
        took
    }

    /// What `report` took: each step's comm, then each pass's quotes and
    /// fight events.
    fn took(report: &StepReport) -> Vec<(Vec<CommNote>, Vec<Took>)> {
        report
            .steps
            .iter()
            .map(|run| {
                let passes = run
                    .passes
                    .iter()
                    .map(|pass| (pass.quotes.clone(), pass.combat.clone()))
                    .collect();
                (run.comm.clone(), passes)
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
        assert_eq!(
            session.take_rescue_desc().map(|desc| desc.id),
            Some(13999),
            "rescued"
        );
    }

    #[test]
    fn a_rescue_on_the_second_pass_ends_the_stepping_with_no_comm_taken() {
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
            copy.take_rescue_desc().map(|desc| desc.id),
            Some(13999),
            "rescued on the second pass"
        );
        assert!(report.steps[0].comm.is_empty(), "none taken");
        assert_eq!(copy.take_comm(), [repaired()], "held for the next take");
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
    fn comm_raised_on_the_second_pass_waits_for_the_next_steps_take() {
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
        assert!(
            report.steps[0].comm.is_empty(),
            "taken after the advance pass"
        );
        let report = steps(&mut session, &catalog, &input, 1);
        assert_eq!(report.steps[0].comm, [repaired()], "the next step's take");
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
        }
    }
}
