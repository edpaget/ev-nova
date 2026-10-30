//! The plunder dialog's loop, as `_DoPlunderDialog` (@0x9302b-0x94a1a)
//! runs it: what each press beeps, and the pass that ends every press
//! that leaves the dialog open. All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! **The press.** `_PlunderFilter` (@0x92f20) gives Return, Enter and
//! Escape as item 1, Abort, and a mouse-down to `_TrackPlunderButtons`
//! (@0x2732a), which hit-tests only the enabled buttons
//! (@0x273d3-0x27426): anything but a button released under the mouse is
//! item -1 ([`Take::Miss`](crate::board::Take::Miss)), which still ends
//! the modal wait (@0x92fdf). Every item but Abort first rolls the
//! self-destruct while it is armed (@0x935f7-0x93613), and every press
//! that doesn't trip it disarms it (@0x935e5); a take that does something
//! arms it again. So a miss, or a take with nothing on board, rolls and
//! disarms.
//!
//! **The beeps** (`PlaySound`, priority 1). [`SimSound::PlunderBeep`]
//! (`_beepSnd[2]`, `snd ` 152): Abort (@0x935cb), a take's self-destruct
//! (@0x93681), each take that does something (cargo @0x939b9, or none
//! stored @0x937a0; credits @0x93b3b; ammunition @0x93f10, or none
//! fitting @0x93f77; energy @0x94095), the capture's successful roll
//! (@0x94368), and a person's grant line as the dialog opens (@0x933fe).
//! [`SimSound::Refused`] (`_beepSnd[3]`, 153): a take with nothing on
//! board (@0x93ad2, @0x93cb8, @0x93d2f, @0x94256), or an Ammo press whose
//! rounds no outfit's first mod names (@0x93d2f; see the `plunder_ammo`
//! module), a failed capture
//! (@0x94936) and a full fleet (@0x9487f). Silent: a miss, the capture's
//! own self-destruct (@0x942de-0x94336) and the escort path
//! (@0x9459b-0x947cf).
//!
//! **The loop's pass.** Each press that leaves the dialog open (a take, a
//! miss, a full fleet) ends at @0x9499c: `_PlunderDialogUpdate`, then
//! `_HandleComm` (@0x949a1), which counts the message line down one tick
//! (`Session::comm_line`) and hides the floating map once it has been
//! up 500 ticks (the flight view's). So the line loses a tick a press,
//! not a frame. Abort, a self-destruct, a failed capture and a capture
//! close the dialog first and count nothing.

use super::{Aboard, Outbox, Session, Space};
use crate::board::Taken;
use crate::sound::SimSound;

impl Session {
    /// Lets the boarding under way go, as the plunder dialog's close does
    /// (the session resets, @0x94a0b), but silently and taking nothing:
    /// the flight's way out when it has no dialog to show it in. A capture
    /// awaiting its assignment is let go too.
    pub(crate) fn let_go_of_boarding(&mut self) {
        if self.space.escorts.aboard.is_some() {
            self.close_plunder();
        }
    }
}

impl Space {
    /// A press that leaves the plunder dialog open, the boarding going
    /// on as `aboard`: the loop's pass ends with `_HandleComm`
    /// (@0x949a1), counting the message line down one tick (see the
    /// module docs). Gives `taken`.
    pub(super) fn keep_plunder_open(&mut self, aboard: Aboard, taken: Taken) -> Taken {
        self.escorts.aboard = Some(aboard);
        self.count_down_comm_line();
        taken
    }

    /// A take whose value is not on board: the refusal's beep
    /// ([`SimSound::Refused`], @0x93ad2, @0x93cb8, @0x93d2f, @0x94256),
    /// nothing taken, and the dialog open.
    pub(super) fn refuse_take(&mut self, out: &mut Outbox, aboard: Aboard) -> Taken {
        out.sounds.push(SimSound::Refused);
        self.keep_plunder_open(aboard, Taken::Missed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Assigned, Assignment, Boarding, MAX_ESCORTS, NovaBoarding, Take};
    use crate::catalog::{OutfitId, ShipId};
    use crate::chance::NeverFires;

    use crate::comm_line::CommLine;
    use crate::flight::Controls;
    use crate::pilot::Escort;
    use crate::reserves::Reserves;
    use crate::session::tests::{
        ACE_DRAWS, aboard, alongside, alongside_ace, ammo_aboard, ammo_session, board_drawing,
        boardable, captured, granting, take,
    };
    use crate::testkit::Draws;
    use crate::testkit::FakePilotCatalog;
    use crate::testkit::{Passive, work_with};

    /// The sounds raised since they were last taken.
    fn sounds(session: &mut Session) -> Vec<SimSound> {
        session.take_sounds()
    }

    /// [`aboard`], its boarding's sounds taken.
    fn quiet(catalog: &FakePilotCatalog) -> Session {
        let mut session = aboard(catalog);
        session.take_sounds();
        session
    }

    const BEEP: SimSound = SimSound::PlunderBeep;

    /// Presses in the plunder dialog, each with its draws.
    type Presses<'a> = &'a [(Take, &'a [u32])];

    // The beeps.

    #[test]
    fn abort_beeps() {
        let mut session = quiet(&boardable());
        assert_eq!(take(&mut session, Take::Abort, &[]).0, Taken::Aborted);
        assert_eq!(sounds(&mut session), [BEEP]);
    }

    #[test]
    fn each_take_beeps() {
        let catalog = boardable();
        for press in [Take::Cargo, Take::Credits, Take::Energy] {
            let mut session = quiet(&catalog);
            take(&mut session, press, &[]);
            assert_eq!(sounds(&mut session), [BEEP], "{press:?}");
        }
        let mut full = quiet(&catalog);
        full.pilot
            .missions
            .put(0, crate::testkit::carrying(140, 25));
        assert!(matches!(
            take(&mut full, Take::Cargo, &[]).0,
            Taken::Cargo { stored: 0, .. }
        ));
        assert_eq!(sounds(&mut full), [BEEP], "no room");
        let mut ammo = ammo_session(&ammo_aboard(1, 20));
        ammo.take_sounds();
        assert!(matches!(
            take(&mut ammo, Take::Ammo, &[]).0,
            Taken::Ammo { count: 9, .. }
        ));
        assert_eq!(sounds(&mut ammo), [BEEP], "some taken");
        let mut none_fit = ammo_session(&ammo_aboard(1, 12));
        none_fit.pilot.outfits.insert(OutfitId(310), 12);
        none_fit.take_sounds();
        assert!(matches!(
            take(&mut none_fit, Take::Ammo, &[]).0,
            Taken::Ammo { count: 0, .. }
        ));
        assert_eq!(sounds(&mut none_fit), [BEEP], "none fitting");
    }

    #[test]
    fn a_takes_self_destruct_beeps_and_the_captures_does_not() {
        let catalog = boardable();
        let mut session = quiet(&catalog);
        take(&mut session, Take::Cargo, &[]);
        assert_eq!(take(&mut session, Take::Credits, &[0]).0, Taken::Tripped);
        assert_eq!(
            sounds(&mut session),
            [BEEP, BEEP],
            "the take, then the trip"
        );
        let mut session = quiet(&catalog);
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 0]).0,
            Taken::Tripped
        );
        assert_eq!(sounds(&mut session), [], "@0x942de-0x94336");
    }

    #[test]
    fn a_failed_capture_beeps_its_refusal_and_closes() {
        let mut session = quiet(&boardable());
        assert_eq!(
            take(&mut session, Take::Capture, &[44]).0,
            Taken::CaptureFailed
        );
        assert_eq!(sounds(&mut session), [SimSound::Refused]);
        assert_eq!(session.boarding(), None);
    }

    #[test]
    fn a_full_fleet_beeps_the_roll_then_its_refusal_and_stays_open() {
        let mut session = quiet(&boardable());
        session.pilot.escorts = vec![
            Escort {
                ship: ShipId(130),
                reserves: Reserves::default(),
                order: None,
                carried: false,
                wage: None,
                person: None,
                mark: None,
            };
            MAX_ESCORTS
        ];
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 1]).0,
            Taken::FleetFull
        );
        assert_eq!(sounds(&mut session), [BEEP, SimSound::Refused]);
        assert!(session.boarding().is_some());
    }

    #[test]
    fn a_successful_roll_beeps_whatever_comes_of_it() {
        let catalog = boardable();
        let mut session = quiet(&catalog);
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 1]).0,
            Taken::Captured
        );
        assert_eq!(sounds(&mut session), [BEEP]);
        let mut crewless = boardable();
        for record in &mut crewless.ship_records {
            if record.id == ShipId(128) {
                record.crew = 0;
            }
        }
        let mut session = alongside(&crewless);
        session
            .board(
                &crate::legal::NovaLaw::default(),
                &NovaBoarding::default(),
                &mut Draws::of(&crate::session::tests::BOARD_DRAWS),
            )
            .expect("boards");
        session.take_sounds();
        assert_eq!(
            take(&mut session, Take::Capture, &[10, 1]).0,
            Taken::Escorted
        );
        assert_eq!(sounds(&mut session), [BEEP], "the escort path is silent");
    }

    #[test]
    fn a_grant_beeps_only_when_it_grants_some() {
        let catalog = granting();
        let mut session = alongside_ace(&catalog);
        let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
        assert!(matches!(boarded, Ok(Boarding::Opened(_))));
        assert_eq!(sounds(&mut session), [SimSound::Boarded, BEEP]);
        let mut session = alongside_ace(&catalog);
        session.pilot.outfits.insert(OutfitId(200), 10);
        let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
        assert!(matches!(boarded, Ok(Boarding::Opened(_))));
        assert_eq!(session.take_grant(), None, "none fits under its Max");
        assert_eq!(sounds(&mut session), [SimSound::Boarded]);
    }

    #[test]
    fn either_assignment_beeps_first() {
        let catalog = boardable();
        for choice in [Assignment::Escort, Assignment::MyShip] {
            let mut session = captured(&catalog);
            session.take_sounds();
            assert!(session.assign(choice, &mut Draws::of(&[])).is_some());
            assert_eq!(
                sounds(&mut session).first(),
                Some(&SimSound::CaptureAssigned),
                "{choice:?}"
            );
        }
        let mut session = captured(&catalog);
        session.take_sounds();
        while session
            .space
            .traffic
            .free_berth(session.setup.spawn_reserve(crate::traffic::CAPTURE_RESERVE))
            .is_some()
        {
            let filler = session.space.traffic.npcs()[1].clone();
            session.space.traffic.add_npc(filler);
        }
        assert_eq!(
            crate::session::capture_naming::tests::use_as_my_ship(
                &mut session,
                "Prize",
                &mut Draws::of(&[])
            ),
            Some(Assigned::Abandoned)
        );
        assert_eq!(sounds(&mut session), [SimSound::CaptureAssigned]);
    }

    #[test]
    fn a_boarding_let_go_closes_silently_and_resets() {
        use crate::scan::Latches;
        let catalog = boardable();
        let mut session = quiet(&catalog);
        crate::session::precalced::tests::convicted(&mut session);
        session.let_go_of_boarding();
        assert_eq!(session.boarding(), None);
        assert_eq!(session.space.escorts.aboard, None);
        assert_eq!(sounds(&mut session), []);
        assert_eq!(
            session.ship.latches.scan_latches,
            Latches::ARMED,
            "the close's reset"
        );
        let mut captured = captured(&catalog);
        captured.let_go_of_boarding();
        assert_eq!(
            captured.assign(Assignment::Escort, &mut Draws::of(&[])),
            None,
            "a capture awaiting its assignment let go too"
        );
    }

    #[test]
    fn the_flight_counts_its_own_line_down_a_tick() {
        let mut session = quiet(&boardable());
        session.comm_shown(10);
        session.comm_ticked();
        assert_eq!(session.comm_line(), line(1));
    }

    // The loop.

    #[test]
    fn a_miss_rolls_the_armed_self_destruct_and_disarms_it() {
        let catalog = boardable();
        let mut session = quiet(&catalog);
        take(&mut session, Take::Cargo, &[]);
        assert_eq!(
            take(&mut session, Take::Miss, &[0]),
            (Taken::Tripped, vec![100])
        );
        let mut session = quiet(&catalog);
        take(&mut session, Take::Cargo, &[]);
        session.take_sounds();
        assert_eq!(
            take(&mut session, Take::Miss, &[99]),
            (Taken::Missed, vec![100])
        );
        assert_eq!(sounds(&mut session), [], "a miss is silent");
        assert_eq!(
            take(&mut session, Take::Credits, &[]),
            (Taken::Credits(5750), vec![]),
            "disarmed: no roll"
        );
    }

    #[test]
    fn a_take_of_nothing_beeps_its_refusal_rolls_and_disarms() {
        let catalog = boardable();
        let mut session = quiet(&catalog);
        take(&mut session, Take::Cargo, &[]);
        session.take_sounds();
        assert_eq!(
            take(&mut session, Take::Cargo, &[99]),
            (Taken::Missed, vec![100])
        );
        assert_eq!(sounds(&mut session), [SimSound::Refused]);
        assert_eq!(
            take(&mut session, Take::Credits, &[]),
            (Taken::Credits(5750), vec![]),
            "disarmed"
        );
        for press in [Take::Credits, Take::Energy, Take::Ammo] {
            let mut session = quiet(&catalog);
            take(&mut session, press, &[]);
            take(&mut session, Take::Abort, &[]);
            let mut session = quiet(&catalog);
            if press != Take::Ammo {
                take(&mut session, press, &[]);
                take(&mut session, Take::Miss, &[99]);
            }
            session.take_sounds();
            assert_eq!(take(&mut session, press, &[]).0, Taken::Missed, "{press:?}");
            assert_eq!(sounds(&mut session), [SimSound::Refused], "{press:?}");
            assert!(session.boarding().is_some());
        }
        let mut session = quiet(&catalog);
        take(&mut session, Take::Cargo, &[]);
        assert_eq!(
            take(&mut session, Take::Cargo, &[0]).0,
            Taken::Tripped,
            "a take of nothing still rolls"
        );
    }

    #[test]
    fn a_capture_at_no_odds_fails_and_closes() {
        let mut session = quiet(&boardable());
        if let Some(aboard) = &mut session.space.escorts.aboard {
            aboard.plunder.odds = 0;
        }
        assert_eq!(
            take(&mut session, Take::Capture, &[0]),
            (Taken::CaptureFailed, vec![100])
        );
        assert_eq!(sounds(&mut session), [SimSound::Refused]);
        assert_eq!(session.boarding(), None);
    }

    /// The comm line shown for 10 ticks, counted down `n`.
    fn line(n: usize) -> CommLine {
        let mut line = CommLine::default();
        line.show(10);
        for _ in 0..n {
            line.tick();
        }
        line
    }

    #[test]
    fn each_press_that_leaves_the_dialog_open_counts_the_line_down_a_tick() {
        let catalog = boardable();
        let mut session = quiet(&catalog);
        session.comm_shown(10);
        take(&mut session, Take::Cargo, &[]);
        assert_eq!(session.comm_line(), line(1), "a take");
        take(&mut session, Take::Miss, &[99]);
        assert_eq!(session.comm_line(), line(2), "a miss");
        take(&mut session, Take::Cargo, &[]);
        assert_eq!(session.comm_line(), line(3), "a take of nothing");
        session.pilot.escorts = vec![
            Escort {
                ship: ShipId(130),
                reserves: Reserves::default(),
                order: None,
                carried: false,
                wage: None,
                person: None,
                mark: None,
            };
            MAX_ESCORTS
        ];
        take(&mut session, Take::Capture, &[43, 1]);
        assert_eq!(session.comm_line(), line(4), "a full fleet");
    }

    #[test]
    fn a_press_that_closes_the_dialog_counts_nothing() {
        let catalog = boardable();
        let cases: [(&str, Take, &[u32]); 4] = [
            ("abort", Take::Abort, &[]),
            ("a failed capture", Take::Capture, &[44]),
            ("a capture's trip", Take::Capture, &[43, 0]),
            ("a capture", Take::Capture, &[43, 1]),
        ];
        for (why, press, draws) in cases {
            let mut session = quiet(&catalog);
            session.comm_shown(10);
            take(&mut session, press, draws);
            assert_eq!(session.comm_line(), line(0), "{why}");
        }
        let mut session = quiet(&catalog);
        take(&mut session, Take::Cargo, &[]);
        session.comm_shown(10);
        assert_eq!(take(&mut session, Take::Credits, &[0]).0, Taken::Tripped);
        assert_eq!(session.comm_line(), line(0), "a take's trip");
    }

    #[test]
    fn a_tripped_ship_starts_its_breakup_with_a_fresh_timer_on_the_next_tick() {
        use crate::combat::hull::Condition;
        let mut catalog = boardable();
        for hull in &mut catalog.hulls {
            if hull.id == ShipId(129) {
                hull.death_delay = 30;
            }
        }
        let presses: [(&str, Presses); 2] = [
            (
                "a take's trip",
                &[(Take::Cargo, &[]), (Take::Credits, &[0])],
            ),
            ("a capture's trip", &[(Take::Capture, &[43, 0])]),
        ];
        for (why, presses) in presses {
            let mut session = quiet(&catalog);
            for &(press, draws) in presses {
                take(&mut session, press, draws);
            }
            assert_eq!(session.npcs()[0].condition, Condition::Disabled, "{why}");
            work_with(
                &mut session,
                &catalog,
                Controls::default(),
                &Passive,
                crate::combat::Rules::default(),
                &mut NeverFires,
            );
            assert_eq!(
                session.npcs()[0].condition,
                Condition::Dying {
                    ticks_left: 30,
                    starting: true
                },
                "{why}: its timer set afresh from its DeathDelay"
            );
        }
    }
}
