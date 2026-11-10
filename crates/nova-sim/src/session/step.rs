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
//!    and weapon safety keys, the double-time key, Hyperspace Effects,
//!    the land key and a stick's amounts (see the `analog` module), each
//!    a store but the double-time key, which latches the switch at once
//!    (see the `double_time` module).
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
//! **Each step**, stopped before it once the hold after the player's
//! explosion has run out ([`Session::hold_expired`]), and after one an
//! exit settled on (below):
//!
//! 1. each ship's state is recorded, which the view draws from; the
//!    frame's gain is taken; and while the gate autopilot is engaged its
//!    controls replace the input's, at the keys' full amounts
//!    (`Session::step_controls`, see the `gate_pilot` module), once a
//!    step, so both passes below fly them, as they fly a held key;
//! 2. the advance pass (`_DoPlayGameWork(1)` @0x43de5-0x4453d,
//!    [`Session::game_work`]), or, on the first step after a comm, escort
//!    or info dialog's zero, the rest of the frame it opened on, at gain 0
//!    ([`Session::rest_of_opening_pass`], see the `opening_rest` module),
//!    then the exit it settled, if any, and its comm notes are taken;
//! 3. the NPCs answer what the session left them
//!    ([`Session::answer_strikes`]): the pass answers every hit it lands
//!    as its phase settles, the beams' and shockwaves' included, as
//!    `_DamageShip` does (@0x44385, @0x4453d), so only the session's own
//!    calls are left here, before the frame is drawn. The pass said each
//!    hit's attack quote as its hit settled, counted the line down at
//!    `_HandleComm`'s place (@0x4421e) and ran `_HandleShip`'s quote roll
//!    (@0x33f0f-0x33fba; [`Session::roll_quote_at_turn`]) at each NPC's
//!    turn of its ship loop (see [`Session::ship_loop`]), so here only
//!    the attack quotes these answers queue are said
//!    (`_ShowPersonHailQuote` @0x89ce1;
//!    [`Session::say_attack_quotes_left`]), and the pass's quotes,
//!    fight events and ships are reported ([`PassReport`]), with its
//!    escort-menu block ([`PassReport::escort_menu`], see the
//!    `class_orders` module): whether an escort flew as its fade counted
//!    them and whether its bookkeeping cleared a class's kept order, so
//!    the view fades the escort menu once a pass and shows it in full
//!    again between that pass's drop and the next;
//! 4. in double time, the frame as it stands is reported
//!    ([`StepRun::between_passes`]), as that is what the original draws
//!    (`_PlayGame` @0x45d96-0x45e82), then the second pass
//!    (`_DoPlayGameWork(0)` @0x45f31, [`Session::tick_again`]), the exit
//!    it settled taken, the strikes answered, the attack quotes left
//!    said, and its own report. Its ship loop ran the quote
//!    roll on the clock unmoved, with no countdown, as `_HandleComm` runs
//!    on the advance pass alone. The lines the second pass raises, the
//!    arrival's among them, are held apart while it runs ([`StepRun::second_pass_lines`]), off
//!    the step's comm, its quotes, the messages and its last line. The
//!    original's `_DisplayComm` draws its line into the background and
//!    work frames at once (@0x90294-0x90390), and the line reaches the
//!    screen only at the next frame's draw (`_FloaterDrawCallback`
//!    @0x38b52-0x38b90), so the view shows them on the frame after. Only
//!    the frame's last step holds them: as the next step begins, an
//!    earlier step's are folded back into its own streams, its messages
//!    onto those not yet drained, so the view replays that step in full.
//!    A step an exit or the hold's expiry stops after keeps them.
//! 5. the line the step raised last, whatever its kind
//!    ([`StepRun::last_line`]): `_DisplayComm` replaces its one slot with
//!    each line (@0x90251-0x90265), so the frame ends on the last raised,
//!    and the view shows it last. On a step holding second-pass lines it
//!    is the slot as the first pass left it. The frame clears the record before its
//!    first step: a line raised before it (a press the view says at once,
//!    a note or quote the inbox held) is older than any the steps raise.
//!    The frame drops a pass's escort-menu block left from before its
//!    first step too. And whether a jump collapsed under the white on the step
//!    ([`StepRun::lit_collapse`], see the `jump_collapse` module), read
//!    per step, so neither double time's second pass nor a catch-up
//!    tick's later steps lose it.
//!
//! **The exits.** The jump's arrival and the escape pod's rescue settle
//! inside the pass that reaches them, on either pass, and the rest of
//! that pass runs among the new system (see the `settled_in_pass`
//! module). Every pass is answered and reported whatever exit it had: the
//! step reports where the exit came ([`StepRun::arrival`],
//! [`StepRun::rescued`]: which pass, the player's place, the fight's
//! events before it and how many comm notes came before it), and the pass
//! report at that index is the rest of the pass. The exit's step ends the
//! stepping, so the texts the arrival or the rescue queued, the
//! defections and the rescue's `dësc` open before the next step.
//!
//! In double time, an exit on the advance pass leaves the frame between
//! the passes in the new system, and the second pass runs there; one on
//! the second pass leaves it in the old, the pod where the first pass left
//! it, as the original draws it before that pass (a rescue's
//! `_FadeScreenOut` flushes and fades frame n's draw, @0x5358, @0x53a4).
//! The arrival's line comes with the second pass's held lines.
//!
//! A rescue's lines split at it: those raised before it come with the
//! step as any line does (on the second pass, among its held lines),
//! and those raised after it, in the rest of its pass and an advance-pass
//! rescue's second pass, are held to the step's end
//! ([`StepRun::rescue_lines`]), as `_ResetPlayer` empties the message line
//! (`_commCount` @0x1d8ff-0x1d904) and the view shows them after its
//! rescue screen.

use crate::ai::Behaviour;
use crate::catalog::{CommCatalog, PilotCatalog, TrafficCatalog};
use crate::chance::Chance;
use crate::combat::{Rules, ShipRef};
use crate::flight_frame::{
    FlightFrame, PassReport, RaisedLine, ShipPlace, StepInput, StepReport, StepRun,
};

use super::Session;
use super::settled_in_pass::{Exit, SettledExit};

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
        self.out.escort_menu = None;
        self.out.in_step = true;
        for _ in 0..steps {
            if self.hold_expired() {
                into.stopped = true;
                break;
            }
            if let Some(earlier) = into.steps.last_mut() {
                self.fold_back_held_lines(earlier);
            }
            let run = self.run_step(input, collab);
            let exited = run.arrival.is_some() || run.rescued.is_some();
            into.steps.push(run);
            if exited {
                into.stopped = true;
                break;
            }
        }
        self.out.in_step = false;
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
        self.set_dock_try(input.dock_try);
        self.hold_jettison(input.jettison);
        self.ship.hold_amounts(input.amounts);
        if input.held {
            self.hold_time();
        }
    }

    /// One step under `input`, handed `collab` (see the module docs).
    fn run_step(&mut self, input: &StepInput, collab: &mut StepCollaborators<'_>) -> StepRun {
        let catalog = collab.catalog;
        // The quote roll's view of the catalog, which both passes hand down
        // to their ship loops.
        let quotes: &dyn CommCatalog = catalog;
        let mut run = StepRun {
            before: self.ships_before(),
            ..StepRun::default()
        };
        let controls = self.step_controls(input);
        if self.take_opening_rest() {
            // The rest of the frame a dialog opened on, at the gain its
            // zero left (see the `opening_rest` module).
            self.rest_of_opening_pass(
                &catalog,
                Some(quotes),
                collab.behaviour,
                collab.rules,
                &mut *collab.chance,
            );
        } else {
            self.adjust_time();
            self.game_work_quoting(
                controls,
                &catalog,
                Some(quotes),
                collab.behaviour,
                collab.rules,
                &mut *collab.chance,
            );
        }
        self.take_exit(&mut run, 0);
        run.comm = self.take_comm();
        self.answer_strikes(collab.behaviour);
        self.say_attack_quotes_left(catalog, &mut *collab.chance);
        run.passes.push(self.pass_report());
        if !self.double_time() {
            self.take_step_ends(&mut run);
            return run;
        }
        run.between_passes = Some(self.frame_between_passes());
        #[cfg(test)]
        self.give_test_command_between_passes(&mut *collab.chance);
        // An advance-pass rescue's hold stays open through the second
        // pass (see the `settled_in_pass` module).
        if run.rescued.is_none() {
            self.out.second_pass = Some(Vec::new());
        }
        self.tick_again_quoting(
            controls,
            &catalog,
            Some(quotes),
            collab.behaviour,
            collab.rules,
            &mut *collab.chance,
        );
        #[cfg(test)]
        self.raise_test_second_pass_lines();
        self.take_exit(&mut run, 1);
        self.answer_strikes(collab.behaviour);
        self.say_attack_quotes_left(catalog, &mut *collab.chance);
        run.passes.push(self.pass_report());
        run.comm.extend(self.take_comm());
        if run.rescued.is_none() {
            run.second_pass_lines = self.close_second_pass();
        }
        self.take_step_ends(&mut run);
        run
    }

    /// Takes the exit a step's `pass` settled, if any, into `run`: the
    /// arrival or the rescue, its pass and its count of comm notes before
    /// it among the step's, and a rescue's texts; a rescue on the second
    /// pass gives the lines it held before it as the step's second-pass
    /// lines (see the module docs).
    fn take_exit(&mut self, run: &mut StepRun, pass: usize) {
        let Some(SettledExit {
            exit,
            mut report,
            held_before,
        }) = self.out.exit.take()
        else {
            return;
        };
        report.pass = pass;
        // The notes the step took before this pass, then this pass's
        // before the exit.
        report.comm += run.comm.len();
        match exit {
            Exit::Arrival => run.arrival = Some(report),
            Exit::Rescue => {
                run.rescued = Some(report);
                run.rescue_texts = self.ship.fate.rescue_texts.take();
                if let Some(lines) = held_before {
                    run.second_pass_lines = lines;
                }
            }
        }
    }

    /// Ends double time's second pass's hold on its lines: the lines it
    /// raised, in order (see the module docs).
    fn close_second_pass(&mut self) -> Vec<RaisedLine> {
        self.out.second_pass.take().unwrap_or_default()
    }

    /// Folds the lines `run`'s second pass held back into its streams,
    /// as a step after it begins: its messages onto the messages not yet
    /// drained, its comm notes onto the step's comm, its quotes onto that
    /// pass's, and the last of them as the line the step raised last (see
    /// the module docs).
    fn fold_back_held_lines(&mut self, run: &mut StepRun) {
        let held = std::mem::take(&mut run.second_pass_lines);
        if let Some(last) = held.last() {
            run.last_line = Some(last.clone());
        }
        for line in held {
            match line {
                RaisedLine::Message(message) => self.out.messages.push(message),
                RaisedLine::Comm(note) => run.comm.push(note),
                RaisedLine::Quote(quote) => {
                    if let Some(pass) = run.passes.last_mut() {
                        pass.quotes.push(quote);
                    }
                }
            }
        }
    }

    /// Raises the next batch of the lines a test set for a second pass
    /// to raise (`Outbox::second_pass_lines`), each as the session raises
    /// a line.
    #[cfg(test)]
    fn raise_test_second_pass_lines(&mut self) {
        if self.out.second_pass_lines.is_empty() {
            return;
        }
        for line in self.out.second_pass_lines.remove(0) {
            self.space.raise_line(&mut self.out, line);
        }
    }

    /// Gives the escort command a test set for between double time's
    /// passes (`Outbox::command_between_passes`), drawing on `chance`.
    #[cfg(test)]
    fn give_test_command_between_passes(&mut self, chance: &mut dyn crate::chance::Chance) {
        if let Some((group, command)) = self.out.command_between_passes.take() {
            self.command_escorts(group, command, chance);
        }
    }

    /// The flight as it stands, between double time's two passes.
    fn frame_between_passes(&self) -> Box<FlightFrame> {
        let mut frame = Box::<FlightFrame>::default();
        self.fill_frame(&mut frame);
        frame
    }

    /// What `run`'s step leaves at its end, taken into it: the lines a
    /// rescue on it held after it, the line it raised last, and whether a
    /// jump collapsed under the white on it (see the module docs).
    fn take_step_ends(&mut self, run: &mut StepRun) {
        if run.rescued.is_some() {
            run.rescue_lines = self.close_second_pass();
        }
        run.last_line = self.out.last_line.take();
        run.lit_collapse = std::mem::take(&mut self.out.lit_collapse);
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

    /// The pass just run, as it stands, its quotes, fight events and
    /// escort-menu block taken: each ship's record as the frame builds it
    /// (`Session::fill_ships`), and each shot's in the fight's order.
    fn pass_report(&mut self) -> PassReport {
        let mut ships = Vec::new();
        self.fill_ships(&mut ships);
        let shots = self
            .shots()
            .iter()
            .map(|shot| self.space.frame_shot(shot))
            .collect();
        PassReport {
            ships,
            shots,
            beam_weapons: self.beams().iter().map(|beam| beam.weapon.id).collect(),
            combat: self.take_combat_events(),
            smoke_frame: self.smoke_frame(),
            system_murk: self.system_murk(),
            player_position: self.player().position,
            double_time: self.double_time(),
            speed_mult: self.speed_mult(),
            quotes: self.take_quotes(),
            wrap: self.out.player_wrap.take(),
            escort_menu: self.out.escort_menu.take(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::MaskSheet;
    use crate::chance::NeverFires;
    use crate::combat::CombatEvent;
    use crate::flight::Controls;
    use crate::flight_frame::{EscortMenuPass, FrameShot, RaisedLine};
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

    /// What a pass took by hand: its quotes and fight events, and its
    /// escort-menu block.
    type Took = (Vec<PersonQuote>, Vec<CombatEvent>, Option<EscortMenuPass>);

    /// What a step took by hand: its comm, each pass's takes, the line it
    /// raised last, and where a rescue's texts fell.
    type StepTook = (
        Vec<CommNote>,
        Vec<Took>,
        Option<RaisedLine>,
        Option<crate::flight_frame::RescueTexts>,
    );

    /// What the pass just run took by hand.
    fn pass_took(session: &mut Session) -> Took {
        (
            session.take_quotes(),
            session.take_combat_events(),
            session.out.escort_menu.take(),
        )
    }

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
        session.set_dock_try(input.dock_try);
        session.hold_jettison(input.jettison);
        session.out.last_line = None;
        session.out.escort_menu = None;
        let mut took = Vec::new();
        session.out.in_step = true;
        for _ in 0..count {
            if session.hold_expired() {
                break;
            }
            session.game_work_quoting(
                input.controls,
                catalog,
                Some(catalog),
                &behaviour,
                Rules::default(),
                chance,
            );
            let mut exits = vec![session.out.exit.take()];
            let mut comm = session.take_comm();
            let mut passes = Vec::new();
            session.answer_strikes(&behaviour);
            session.say_attack_quotes_left(catalog, chance);
            passes.push(pass_took(session));
            let rescued = |exits: &[Option<SettledExit>]| {
                exits.iter().flatten().any(|exit| exit.exit == Exit::Rescue)
            };
            if session.double_time() {
                if let Some((group, command)) = session.out.command_between_passes.take() {
                    session.command_escorts(group, command, chance);
                }
                session.tick_again_quoting(
                    input.controls,
                    catalog,
                    Some(catalog),
                    &behaviour,
                    Rules::default(),
                    chance,
                );
                session.raise_test_second_pass_lines();
                exits.push(session.out.exit.take());
                session.answer_strikes(&behaviour);
                session.say_attack_quotes_left(catalog, chance);
                passes.push(pass_took(session));
                comm.extend(session.take_comm());
            }
            // The rescue's hold on the lines after it.
            if rescued(&exits) {
                session.out.second_pass = None;
            }
            took.push(step_took(session, comm, passes));
            if exits.iter().any(Option::is_some) {
                break;
            }
        }
        session.out.in_step = false;
        took
    }

    /// What a step took by hand, its `comm` and its `passes`' takes: then
    /// the line it raised last and where a rescue's texts fell.
    fn step_took(session: &mut Session, comm: Vec<CommNote>, passes: Vec<Took>) -> StepTook {
        let line = session.out.last_line.take();
        let rescue_texts = session.ship.fate.rescue_texts.take();
        (comm, passes, line, rescue_texts)
    }

    /// What `report` took: each step's comm, then each pass's quotes,
    /// fight events and escort-menu block, the line it raised last and
    /// where a rescue's texts fell.
    fn took(report: &StepReport) -> Vec<StepTook> {
        report
            .steps
            .iter()
            .map(|run| {
                let passes = run
                    .passes
                    .iter()
                    .map(|pass| (pass.quotes.clone(), pass.combat.clone(), pass.escort_menu))
                    .collect();
                (
                    run.comm.clone(),
                    passes,
                    run.last_line.clone(),
                    run.rescue_texts,
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
            dock_try: true,
            jettison: Some(false),
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
        let one: &[&str] = &["#"];
        let catalog = FakePilotCatalog {
            rle_sheets: vec![(
                MaskSheet::Shot(0),
                nova_sim_base::fixture::rle::sheet(&[one; 4]),
            )],
            ..armed()
        };
        for double in [false, true] {
            let mut session = facing_an_npc(&catalog, 180);
            let input = firing(double);
            // Each pass's records, as the session stands after it.
            let mut expected = Vec::new();
            let mut hand = session.clone();
            let behaviour = crate::ai::NovaAi::default();
            let chance = &mut NeverFires;
            by_hand(&mut hand, &catalog, &input, 0);
            for _ in 0..30 {
                hand.game_work_quoting(
                    input.controls,
                    &catalog,
                    Some(&catalog),
                    &behaviour,
                    Rules::default(),
                    chance,
                );
                hand.take_comm();
                hand.answer_strikes(&behaviour);
                hand.say_attack_quotes_left(&catalog, chance);
                expected.push(records(&mut hand));
                if hand.double_time() {
                    hand.tick_again_quoting(
                        input.controls,
                        &catalog,
                        Some(&catalog),
                        &behaviour,
                        Rules::default(),
                        chance,
                    );
                    hand.rescue_if_due(&catalog, chance);
                    hand.answer_strikes(&behaviour);
                    hand.say_attack_quotes_left(&catalog, chance);
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
            let mut shots = passes.iter().flat_map(|pass| &pass.shots).peekable();
            assert!(shots.peek().is_some(), "fired");
            assert!(shots.all(|shot| shot.frame.is_some()), "each at its frame");
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
        let lines = crate::testkit::QuoteLines(catalog.strings[0].1.clone());
        let mut session = facing_an_npc(&catalog, 180).with_strings(std::rc::Rc::new(lines));
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
        assert_eq!(run.passes.len(), 1, "the rest of the pass reported");
        assert_eq!(run.passes[0].quotes, [quote], "its quotes taken with it");
        assert_eq!(run.comm, [note], "its comm taken");
        let rescued = run.rescued.as_ref().expect("rescued");
        assert_eq!(rescued.comm, 1, "the note raised before the rescue");
        assert_eq!(rescued.place, ShipPlace::from(*session.player()));
        assert_eq!(
            session.take_rescue_desc().map(|desc| desc.id),
            Some(13999),
            "rescued"
        );
    }

    /// [`catalog`] with the shuttle the rescue boards (`shïp` 128), so
    /// its `_ResetPlayer` takes the pilot out of the pod.
    fn rescuing() -> FakePilotCatalog {
        FakePilotCatalog {
            ship_records: vec![crate::testkit::ship(128, crate::testkit::FAST)],
            ..catalog()
        }
    }

    #[test]
    fn an_advance_pass_rescue_reports_the_rest_of_its_pass_among_the_new_system() {
        let catalog = rescuing();
        let mut session = in_the_pod(&catalog, 1);
        // A ship of the system left, gone with it.
        let left = session.space.traffic.add_npc(crate::testkit::npc(
            1,
            crate::stats::ShipStats::new(crate::testkit::FAST, &[]),
        ));
        let report = steps(&mut session, &catalog, &StepInput::default(), 3);
        let run = &report.steps[0];
        let rescued = run.rescued.as_ref().expect("rescued");
        assert_eq!(rescued.pass, 0);
        assert_eq!(rescued.place.position, crate::session::rescue::RESCUE_AT);
        let pass = &run.passes[0];
        assert_eq!(pass.player_position, crate::session::rescue::RESCUE_AT);
        assert!(
            pass.ships
                .iter()
                .all(|ship| ship.ship != ShipRef::Npc(left)),
            "the new system's ships: {:?}",
            pass.ships
        );
        assert_eq!(run.rescue_lines, [], "none raised after it");
    }

    #[test]
    fn an_advance_pass_rescue_in_double_time_reports_the_frame_between_in_the_new_system_and_its_second_pass()
     {
        let catalog = rescuing();
        let mut session = in_the_pod(&catalog, 1);
        let report = steps(&mut session, &catalog, &doubled(), 3);
        assert_eq!(report.steps.len(), 1, "no later step");
        let run = &report.steps[0];
        assert_eq!(run.rescued.as_ref().map(|exit| exit.pass), Some(0));
        assert_eq!(run.passes.len(), 2, "both passes");
        let between = run.between_passes.as_ref().expect("the frame between");
        assert_eq!(between.system, session.pilot.system, "the new system");
        assert!(
            !matches!(between.fate, crate::PlayerFate::Ejected { .. }),
            "out of the pod"
        );
        assert_eq!(between.player().position, crate::session::rescue::RESCUE_AT);
    }

    #[test]
    fn a_line_raised_after_the_rescue_is_in_its_rescue_lines() {
        let catalog = rescuing();
        let mut session = in_the_pod(&catalog, 1);
        messages_drained(&mut session);
        session.out.second_pass_lines.push(three_lines());
        let report = steps(&mut session, &catalog, &doubled(), 3);
        let run = &report.steps[0];
        assert_eq!(run.rescued.as_ref().map(|exit| exit.pass), Some(0));
        assert_eq!(run.rescue_lines, three_lines(), "in the order raised");
        assert_eq!(
            run.second_pass_lines,
            [],
            "the rescue's hold, not the pass's"
        );
        assert_eq!(run.comm, [], "not on the step's comm");
        assert_eq!(run.passes[1].quotes, [], "nor on its pass's quotes");
        assert!(
            !messages_drained(&mut session).contains(&crate::SimMessage::NoActiveMissions),
            "nor drained"
        );
        assert_eq!(session.out.second_pass, None, "the hold closed");
    }

    #[test]
    fn a_lone_passs_rescue_holds_no_lines() {
        let catalog = rescuing();
        let mut session = in_the_pod(&catalog, 1);
        let rescued = session.game_work(
            Controls::default(),
            &catalog,
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        assert!(rescued);
        assert_eq!(session.out.second_pass, None, "no hold");
        assert_eq!(session.out.exit, None, "no exit recorded");
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
    fn a_rescue_on_the_second_pass_reports_the_frame_between_and_holds_its_lines() {
        let catalog = catalog();
        let mut session = in_the_pod(&catalog, 2);
        let input = doubled();
        let mut hand = session.clone();
        let report = steps(&mut session, &catalog, &input, 3);
        let expected = by_hand(&mut hand, &catalog, &input, 3);
        assert_eq!(session, hand);
        assert_eq!(took(&report), expected);
        assert_eq!(report.steps.len(), 1, "no later step");
        assert!(report.stopped);
        let run = &report.steps[0];
        assert_eq!(run.passes.len(), 2, "the second pass's rest reported too");
        let between = run.between_passes.as_ref().expect("the pod drawn");
        assert_eq!(
            between.player().position,
            run.passes[0].player_position,
            "where the first pass left the pod"
        );
        assert!(
            matches!(between.fate, crate::PlayerFate::Ejected { .. }),
            "not yet rescued"
        );
        let rescued = run.rescued.as_ref().expect("rescued");
        assert_eq!(rescued.pass, 1);
        assert_eq!(rescued.place, ShipPlace::from(*session.player()));
        assert_eq!(
            run.passes[1].player_position,
            crate::session::rescue::RESCUE_AT,
            "its rest among the new system"
        );
        let mut session = in_the_pod(&catalog, 2);
        steps(&mut session, &catalog, &input, 0);
        session.ship.fate.rescue_due = false;
        let mut copy = session.clone();
        copy.out.before_exit_lines = vec![RaisedLine::Comm(repaired())];
        let report = steps(&mut copy, &catalog, &input, 1);
        assert_eq!(
            copy.take_rescue_desc().map(|desc| desc.id),
            Some(13999),
            "rescued on the second pass"
        );
        assert_eq!(report.steps[0].comm, [], "held");
        assert_eq!(
            report.steps[0].second_pass_lines,
            [RaisedLine::Comm(repaired())],
            "for the frame after"
        );
        assert_eq!(report.steps[0].last_line, None, "the first pass's");
        assert_eq!(copy.take_comm(), [], "none left for the next take");
    }

    #[test]
    fn a_second_pass_rescue_reports_its_rest_and_splits_its_lines() {
        let catalog = rescuing();
        let mut session = in_the_pod(&catalog, 2);
        session.out.before_exit_lines = vec![RaisedLine::Comm(repaired())];
        session.out.second_pass_lines.push(three_lines());
        let report = steps(&mut session, &catalog, &doubled(), 1);
        let run = &report.steps[0];
        assert_eq!(run.rescued.as_ref().map(|exit| exit.pass), Some(1));
        assert_eq!(run.passes.len(), 2, "its rest reported");
        assert_eq!(
            run.second_pass_lines,
            [RaisedLine::Comm(repaired())],
            "the lines before the rescue"
        );
        assert_eq!(run.rescue_lines, three_lines(), "the lines after it");
        assert_eq!(run.comm, []);
        assert_eq!(session.out.second_pass, None, "the hold closed");
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

    /// The input of a frame in double time.
    fn doubled() -> StepInput {
        StepInput {
            double_time_key: true,
            ..StepInput::default()
        }
    }

    /// A message, a comm note and a quote, as a second pass raises them.
    fn three_lines() -> Vec<RaisedLine> {
        vec![
            RaisedLine::Comm(repaired()),
            RaisedLine::Message(crate::SimMessage::NoActiveMissions),
            RaisedLine::Quote(halt()),
        ]
    }

    /// A person's "Halt!".
    fn halt() -> PersonQuote {
        PersonQuote {
            npc: crate::traffic::npc::NpcId(0),
            text: "Halt!".to_owned(),
        }
    }

    /// The messages `session` has raised since they were last drained.
    fn messages_drained(session: &mut Session) -> Vec<crate::SimMessage> {
        let mut events = Vec::new();
        session.drain_events(&mut events);
        events
            .into_iter()
            .filter_map(|event| match event {
                crate::flight_frame::FlightEvent::Message(message) => Some(message),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn double_times_second_pass_lines_are_held_off_the_steps_streams() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        steps(&mut session, &catalog, &doubled(), 1);
        messages_drained(&mut session);
        session.out.second_pass_lines.push(three_lines());
        let report = steps(&mut session, &catalog, &doubled(), 1);
        let run = &report.steps[0];
        assert_eq!(run.passes.len(), 2, "both passes");
        assert_eq!(run.second_pass_lines, three_lines(), "in the order raised");
        assert_eq!(run.comm, [], "not on the step's comm");
        assert_eq!(run.passes[1].quotes, [], "nor on its pass's quotes");
        assert_eq!(messages_drained(&mut session), [], "nor drained");
        assert_eq!(run.last_line, None, "the slot as the first pass left it");
        assert!(
            !session.space.comm_line.free_for_calls(),
            "the count set at once"
        );
    }

    #[test]
    fn only_the_frames_last_step_holds_its_second_passs_lines() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        steps(&mut session, &catalog, &doubled(), 1);
        messages_drained(&mut session);
        let own = vec![RaisedLine::Comm(CommNote {
            ship_name: "Lightning".to_owned(),
            ..repaired()
        })];
        session.out.second_pass_lines = vec![three_lines(), own.clone()];
        let report = steps(&mut session, &catalog, &doubled(), 2);
        let [earlier, last] = &report.steps[..] else {
            panic!("two steps: {report:?}");
        };
        assert_eq!(earlier.second_pass_lines, [], "folded back");
        assert_eq!(earlier.comm, [repaired()]);
        assert_eq!(earlier.passes[1].quotes, [halt()]);
        assert_eq!(earlier.last_line, Some(three_lines()[2].clone()));
        assert_eq!(
            messages_drained(&mut session),
            [crate::SimMessage::NoActiveMissions]
        );
        assert_eq!(last.second_pass_lines, own, "its own held");
        assert_eq!(last.comm, []);
        assert_eq!(last.last_line, None);
    }

    #[test]
    fn a_step_the_warps_end_stops_after_keeps_its_lines_held() {
        use crate::session::warp_stage::tests::{frames, jumping_in_double_time, with_a_warp_out};
        let catalog = with_a_warp_out();
        let mut session = jumping_in_double_time(&catalog);
        frames(&mut session, 96);
        session.out.second_pass_lines.push(three_lines());
        let report = steps(&mut session, &catalog, &doubled(), 3);
        assert_eq!(report.steps.len(), 1, "stopped after the arrival's step");
        assert!(report.stopped);
        let run = &report.steps[0];
        assert_eq!(run.arrival.as_ref().map(|exit| exit.pass), Some(1));
        assert!(
            run.second_pass_lines.ends_with(&three_lines()),
            "{:?}",
            run.second_pass_lines
        );
    }

    #[test]
    fn a_single_time_step_holds_no_lines() {
        let (catalog, mut session) = fight();
        let report = steps(&mut session, &catalog, &firing(false), 30);
        assert!(
            report
                .steps
                .iter()
                .all(|run| run.second_pass_lines.is_empty())
        );
    }

    #[test]
    fn comm_raised_on_the_second_pass_is_held_with_its_step() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        steps(&mut session, &catalog, &doubled(), 1);
        session
            .out
            .second_pass_lines
            .push(vec![RaisedLine::Comm(repaired())]);
        let report = steps(&mut session, &catalog, &doubled(), 1);
        assert_eq!(report.steps[0].passes.len(), 2, "both passes");
        assert_eq!(report.steps[0].comm, [], "held");
        assert_eq!(
            report.steps[0].second_pass_lines,
            [RaisedLine::Comm(repaired())]
        );
        let report = steps(&mut session, &catalog, &doubled(), 1);
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
        steps(&mut session, &catalog, &doubled(), 1);
        session
            .out
            .second_pass_lines
            .push(vec![RaisedLine::Comm(repaired())]);
        let report = steps(&mut session, &catalog, &doubled(), 2);
        assert_eq!(report.steps[0].comm, [repaired()]);
        assert_eq!(
            report.steps[0].last_line,
            Some(RaisedLine::Comm(repaired())),
            "raised on the second pass, folded back as the next step began"
        );
        assert_eq!(report.steps[1].last_line, None, "taken with its step");
    }

    /// A session of [`carrying`](crate::session::fighters::tests::carrying)'s
    /// with a warship escort flying.
    fn escorted_warship() -> (FakePilotCatalog, Session) {
        use crate::session::fighters::tests::{carrying, fleet, warship};
        let catalog = carrying();
        let session = fleet(&catalog, 0, vec![warship(None)]);
        (catalog, session)
    }

    /// The warships' group, and Return to Hangar, which the next pass's
    /// bookkeeping clears for them (see the `class_orders` module).
    const WARSHIPS_RETURN: (crate::escort::EscortGroup, crate::escort::EscortCommand) = (
        crate::escort::EscortGroup::Class(crate::escort::EscortClass::Warship),
        crate::escort::EscortCommand::Dock,
    );

    /// [`escorted_warship`], its warships given Return to Hangar.
    fn returning_warship() -> (FakePilotCatalog, Session) {
        let (catalog, mut session) = escorted_warship();
        let (group, command) = WARSHIPS_RETURN;
        session.command_escorts(group, command, &mut NeverFires);
        (catalog, session)
    }

    /// Whether each pass of each step of `report` cleared a class's kept
    /// order; `None` for a pass that reported no escort-menu block.
    fn cleared_by_pass(report: &StepReport) -> Vec<Vec<Option<bool>>> {
        report
            .steps
            .iter()
            .map(|run| {
                run.passes
                    .iter()
                    .map(|pass| pass.escort_menu.map(|menu| menu.class_orders_cleared))
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_class_cleared_is_reported_with_the_pass_that_cleared_it() {
        for (double, cleared) in [
            (false, vec![vec![Some(true)], vec![Some(false)]]),
            (
                true,
                vec![
                    vec![Some(true), Some(false)],
                    vec![Some(false), Some(false)],
                ],
            ),
        ] {
            let (catalog, mut session) = returning_warship();
            let mut hand = session.clone();
            let input = StepInput {
                double_time_key: double,
                ..StepInput::default()
            };
            let report = steps(&mut session, &catalog, &input, 2);
            assert_eq!(cleared_by_pass(&report), cleared, "double time {double}");
            assert_eq!(took(&report), by_hand(&mut hand, &catalog, &input, 2));
            assert_eq!(session, hand);
            assert_eq!(session.out.escort_menu, None, "taken");
        }
    }

    #[test]
    fn a_class_cleared_on_double_times_second_pass_is_reported_with_that_pass() {
        let (catalog, mut session) = escorted_warship();
        session.out.command_between_passes = Some(WARSHIPS_RETURN);
        let mut hand = session.clone();
        let input = StepInput {
            double_time_key: true,
            ..StepInput::default()
        };
        let report = steps(&mut session, &catalog, &input, 1);
        assert_eq!(cleared_by_pass(&report), [[Some(false), Some(true)]]);
        assert_eq!(took(&report), by_hand(&mut hand, &catalog, &input, 1));
        assert_eq!(session, hand);
    }

    #[test]
    fn each_pass_reports_the_escorts_flying_as_its_menu_block_counted_them() {
        let (catalog, mut session) = escorted_warship();
        let input = StepInput {
            double_time_key: true,
            ..StepInput::default()
        };
        let flying = |report: &StepReport| -> Vec<Option<u32>> {
            report.steps[0]
                .passes
                .iter()
                .map(|pass| pass.escort_menu.map(|menu| menu.escorts_flying))
                .collect()
        };
        let report = steps(&mut session, &catalog, &input, 1);
        assert_eq!(flying(&report), [Some(1), Some(1)]);
        session.drop_fleet_escort(0);
        let report = steps(&mut session, &catalog, &input, 1);
        assert_eq!(flying(&report), [Some(0), Some(0)]);
    }

    #[test]
    fn a_menu_block_before_the_frame_is_not_reported_with_its_steps() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.out.escort_menu = Some(EscortMenuPass {
            escorts_flying: 1,
            class_orders_cleared: true,
            ..EscortMenuPass::default()
        });
        let report = steps(&mut session, &catalog, &StepInput::default(), 1);
        assert_eq!(
            report.steps[0].passes[0].escort_menu,
            Some(EscortMenuPass::default())
        );
    }

    #[test]
    fn a_pod_pass_before_a_rescue_on_the_second_pass_runs_no_menu_block_and_comes_with_its_pass() {
        // The pod's frames return (@0x68ddd) before the escort menu's
        // block (@0x6df59) and the class-order bookkeeping (@0x6e2e8), so
        // the pod's pass before the rescue runs and reports no block, nor
        // does the rest of the rescue's (@0x692ce).
        let catalog = catalog();
        let mut session = in_the_pod(&catalog, 2);
        session.space.escorts.class_orders[2] = Some(crate::escort::EscortOrder::Hold);
        // The system set up, as the bookkeeping waits for it.
        session.space.traffic_due = false;
        let input = StepInput {
            double_time_key: true,
            ..StepInput::default()
        };
        let report = steps(&mut session, &catalog, &input, 3);
        assert_eq!(report.steps.len(), 1, "rescued");
        assert!(report.steps[0].rescued.is_some());
        assert_eq!(
            cleared_by_pass(&report),
            [[None, None]],
            "the pod's pass, then the rescue's rest"
        );
        assert_eq!(session.out.escort_menu, None, "none left for the next step");
    }

    #[test]
    fn a_clear_on_a_flying_pass_before_a_rescue_on_the_second_pass_comes_with_its_pass() {
        // The first pass begins out of the pod, so it runs the escort
        // menu's block and its bookkeeping clears the kept order; its
        // stray beam of `ExitType` 5 steps the pod word to 1 (see the
        // `stray_words` module), and the second pass's pod flight runs it
        // out and rescues the pilot (@0x68ddd). The clear comes with the
        // first pass, and none is left for the next step.
        let catalog = crate::session::stray_words::tests::beamed();
        let mut session = facing_an_npc(&catalog, 180);
        session.space.escorts.class_orders[2] = Some(crate::escort::EscortOrder::Hold);
        session.space.traffic_due = false;
        let report = steps(&mut session, &catalog, &firing(true), 3);
        assert_eq!(report.steps.len(), 1, "rescued");
        assert!(report.steps[0].rescued.is_some(), "on the second pass");
        assert_eq!(
            cleared_by_pass(&report),
            [[Some(true), None]],
            "the rescue's rest runs no block"
        );
        assert_eq!(session.space.escorts.class_orders[2], None, "cleared");
        assert_eq!(session.out.escort_menu, None, "none left for the next step");
    }

    #[test]
    fn a_class_order_kept_into_a_rescue_is_gone_before_the_next_pass_and_no_pass_reports_it() {
        // The rescue's pass returns (`jmp 0x71113` @0x692ce) before the
        // escort menu's block (@0x6df59) and the bookkeeping (@0x6e2e8),
        // and `_ResetPlayer(0)` keeps the class table (@0x1d97e). The
        // original's next pass clears it (@0x6e3bb-0x6e452), as the pod
        // released every escort and `_SetupShipsInSystem` (@0x69114)
        // places none marked. The engine resets the orders at the
        // rescue's entry instead (`Session::enter_escorts`), which nothing
        // reads before that pass: the dësc shows none, and the escort
        // menu's rows show no order for a class with no ship out. So the
        // rescue's pass reports its rest with no block, and the next pass
        // clears nothing.
        // The shuttle the rescue boards (`shïp` 128).
        let catalog = FakePilotCatalog {
            ship_records: vec![crate::testkit::ship(128, crate::testkit::FAST)],
            ..catalog()
        };
        let mut session = in_the_pod(&catalog, 1);
        session.space.escorts.class_orders[2] = Some(crate::escort::EscortOrder::Hold);
        session.space.traffic_due = false;
        let report = steps(&mut session, &catalog, &StepInput::default(), 3);
        assert!(report.steps[0].rescued.is_some());
        assert_eq!(
            cleared_by_pass(&report),
            [[None]],
            "the rescue's rest runs no block"
        );
        assert_eq!(session.space.escorts.class_orders, [None; 4], "reset");
        assert_eq!(session.out.escort_menu, None, "none left for the next step");
        assert_eq!(session.pod, None, "out of the pod");
        session.take_rescue_desc();
        let report = steps(&mut session, &catalog, &StepInput::default(), 1);
        assert_eq!(cleared_by_pass(&report), [[Some(false)]]);
    }

    #[test]
    fn a_warps_end_arrives_in_its_pass_and_ends_the_stepping_after_its_step() {
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
        while steps(&mut probe, &catalog, &StepInput::default(), 1).steps[0]
            .arrival
            .is_none()
        {
            ran += 1;
            assert!(ran < 2000, "the warp never ends");
        }
        for _ in 0..ran {
            steps(&mut session, &catalog, &StepInput::default(), 1);
        }
        let report = steps(&mut session, &catalog, &StepInput::default(), 3);
        assert_eq!(report.steps.len(), 1, "no later step");
        assert!(report.stopped);
        let arrival = report.steps[0].arrival.as_ref().expect("arrived");
        assert_eq!(arrival.pass, 0);
        assert_eq!(arrival.place, ShipPlace::from(*session.player()));
        assert_eq!(session.jumping(), None, "arrived in the pass");
        assert_eq!(session.pilot.system, crate::catalog::SystemId(131));
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
                life: shot.life,
                fade: shot.fade_shown,
                frame: session
                    .space
                    .sprite_masks
                    .sheet(MaskSheet::Shot(shot.weapon.graphic))
                    .and_then(|sheet| shot.frame_on(sheet)),
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
            speed_mult: session.speed_mult(),
            quotes: session.take_quotes(),
            combat: session.take_combat_events(),
            wrap: session.out.player_wrap.take(),
            escort_menu: session.out.escort_menu.take(),
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
    fn a_warp_end_on_the_second_pass_reports_the_frame_between_in_the_old_system() {
        use crate::session::warp_stage::tests::{frames, jumping_in_double_time, with_a_warp_out};
        use crate::sound::{SimSound, WARP_UP_HALTS};
        let catalog = with_a_warp_out();
        let mut session = jumping_in_double_time(&catalog);
        frames(&mut session, 96);
        session.take_sounds();
        let from = session.pilot.system;
        // A ship of the system left, gone with it.
        let left = session.space.traffic.add_npc(crate::testkit::npc(
            1,
            crate::stats::ShipStats::new(crate::testkit::FAST, &[]),
        ));
        session.out.second_pass_lines.push(three_lines());
        let report = steps(&mut session, &catalog, &doubled(), 1);
        let sounds = session.take_sounds();
        assert!(
            sounds.contains(&SimSound::Halt(&WARP_UP_HALTS)) && sounds.contains(&SimSound::Arrived),
            "the halt and the warp-out raised with that step: {sounds:?}"
        );
        let run = &report.steps[0];
        assert_eq!(run.arrival.as_ref().map(|exit| exit.pass), Some(1));
        let frame = run.between_passes.as_ref().expect("reported");
        assert_eq!(frame.system, from, "the frame between its passes");
        assert!(frame.warping);
        assert_eq!(run.passes.len(), 2);
        assert!(
            run.passes[0]
                .ships
                .iter()
                .any(|ship| ship.ship == ShipRef::Npc(left)),
            "the first pass in the old system"
        );
        assert!(
            run.passes[1]
                .ships
                .iter()
                .all(|ship| ship.ship != ShipRef::Npc(left)),
            "the second among the new system's ships"
        );
        assert_eq!(
            run.second_pass_lines.first(),
            Some(&RaisedLine::Message(crate::SimMessage::Arrived(
                crate::catalog::SystemId(131)
            ))),
            "the arrival's line first"
        );
        assert!(run.second_pass_lines.ends_with(&three_lines()));
        assert_eq!(session.jumping(), None, "the second pass arrived");
        let next = steps(&mut session, &catalog, &doubled(), 1);
        assert_eq!(next.steps.len(), 1, "the new system steps on");
        assert!(!next.stopped);
    }

    #[test]
    fn a_second_pass_arrival_splits_the_steps_comm_at_it() {
        use crate::session::warp_stage::tests::{frames, jumping_in_double_time, with_a_warp_out};
        let catalog = with_a_warp_out();
        let mut session = jumping_in_double_time(&catalog);
        frames(&mut session, 96);
        session.out.comm.push(repaired());
        let report = steps(&mut session, &catalog, &doubled(), 1);
        let run = &report.steps[0];
        let arrival = run.arrival.as_ref().expect("arrived");
        assert_eq!(arrival.pass, 1);
        assert_eq!(run.comm, [repaired()]);
        assert_eq!(arrival.comm, 1, "the advance pass's note before it");
    }

    #[test]
    fn a_natural_warp_end_on_the_advance_pass_is_in_the_frame_between_its_passes() {
        // w 0.91: over on frame 115's advance pass, its warp-up stepped
        // out at the pass's head.
        use crate::session::warp_stage::tests::{flagged, frames, jumping_in_double_time};
        let catalog = flagged(0x0001);
        let mut session = jumping_in_double_time(&catalog);
        frames(&mut session, 114);
        assert!(session.jumping().is_some(), "frame 114");
        let report = steps(&mut session, &catalog, &doubled(), 1);
        let run = &report.steps[0];
        assert_eq!(run.arrival.as_ref().map(|exit| exit.pass), Some(0));
        let frame = run.between_passes.as_ref().expect("reported");
        assert_eq!(
            frame.system,
            crate::catalog::SystemId(131),
            "the new system"
        );
        assert!(!frame.warping);
        assert_eq!(run.passes.len(), 2, "the second pass runs among it");
        assert_eq!(session.pilot.system, crate::catalog::SystemId(131));
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
        assert_eq!(report.steps[0].passes[0].wrap, None, "on no report");
        assert_eq!(session.out.player_wrap, None, "dropped");
    }
}
