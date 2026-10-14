use super::*;
use crate::catalog::{LandingSite, MissionShip};
use crate::control::Test;
use crate::mission::AcceptRefusal;
use crate::testkit::{FakePilotCatalog, catalog, land_now, mission, planet, with_variants};

const A: MissionId = MissionId(200);
const B: MissionId = MissionId(201);

/// The testkit galaxy, stellars 128 and 129 in system 130 and 140 in
/// 131, with `missions`.
fn galaxy(missions: Vec<MissionRecord>) -> FakePilotCatalog {
    FakePilotCatalog {
        missions,
        ..catalog()
    }
}

/// The testkit galaxy with `missions`, and in system 131 a stellar, 135,
/// that cannot be landed on, listed before 140.
fn atlas(missions: Vec<MissionRecord>) -> FakePilotCatalog {
    let mut catalog = galaxy(missions);
    catalog.sites[1].1.insert(
        0,
        LandingSite {
            flags: 0,
            ..planet(135, -300.0, 0.0)
        },
    );
    catalog
}

/// A new pilot of `catalog` landed on 128, its offers rolled with every
/// roll 1.
fn docked(catalog: &FakePilotCatalog) -> Session {
    let mut session = Session::start(catalog).expect("starts");
    assert_eq!(land_now(&mut session), Ok(StellarId(128)));
    let _ = session.settle_script(catalog, &mut NeverFires);
    session
}

fn bit(n: u16) -> Bit {
    Bit::new(n).expect("a bit")
}

fn bits(numbers: &[u16]) -> Vec<Bit> {
    numbers.iter().copied().map(bit).collect()
}

/// Mission `id`, offered at stellar `at`, completed at `to` for 1000
/// credits, running `on_success`.
fn errand(id: i16, at: i16, to: i16, on_success: &str) -> MissionRecord {
    MissionRecord {
        avail_stel: at,
        return_stel: to,
        pay_val: 1000,
        on_success: Script::parse(on_success),
        ..mission(id)
    }
}

/// The date `days` days after `date`.
fn after(date: GameDate, days: usize) -> GameDate {
    std::iter::successors(Some(date), |date| Some(date.next_day()))
        .nth(days)
        .expect("a date")
}

fn held(session: &Session) -> Vec<MissionId> {
    session.pilot().missions().missions()
}

/// A failure of `step` with `fault`, having looked nowhere, with `set`
/// set.
fn failure(step: usize, at: Step, fault: Fault, set: &[u16]) -> WalkFailure {
    WalkFailure {
        step,
        at,
        fault,
        looked: Vec::new(),
        bits: bits(set),
    }
}

// The script's steps.

#[test]
fn each_step_reads_as_its_script_line() {
    assert_eq!(Step::Take(MissionId(504)).to_string(), "take 504");
    assert_eq!(Step::Deliver(MissionId(505)).to_string(), "deliver 505");
    assert_eq!(Step::PassDays(3).to_string(), "pass 3 days");
    assert_eq!(Step::PassDays(1).to_string(), "pass 1 day");
    assert_eq!(Step::MeetGoal(MissionId(506)).to_string(), "meet goal 506");
}

// The plain decisions.

fn listed(id: i16, systems: &[i16]) -> ListedStellar {
    ListedStellar {
        id: StellarId(id),
        systems: systems.iter().copied().map(SystemId).collect(),
        flags: 0,
        flags2: 0,
        govt: None,
    }
}

fn places(pairs: &[(i16, i16)]) -> Vec<(StellarId, SystemId)> {
    pairs
        .iter()
        .map(|&(stellar, system)| (StellarId(stellar), SystemId(system)))
        .collect()
}

#[test]
fn the_candidates_are_every_listed_stellar_by_id_at_its_first_system_the_docked_one_last() {
    let listed = [
        listed(140, &[131]),
        listed(129, &[130, 132]),
        listed(150, &[]),
        listed(128, &[130]),
    ];
    assert_eq!(
        candidates(&listed, Some(StellarId(129))),
        places(&[(128, 130), (140, 131), (129, 130)])
    );
    let in_order = places(&[(128, 130), (129, 130), (140, 131)]);
    assert_eq!(candidates(&listed, None), in_order);
    assert_eq!(candidates(&listed, Some(StellarId(999))), in_order);
}

#[test]
fn the_bits_expected_are_those_a_script_leaves_set() {
    assert_eq!(
        expected_bits(&Script::parse("b1 b2 !b1 ^b3 b4 ^b4 R(b5 b6) b7 b2")),
        bits(&[2, 7])
    );
    assert_eq!(expected_bits(&Script::parse("^b3 b3")), bits(&[3]));
    assert_eq!(expected_bits(&Script::parse("b8 K145")), bits(&[8]));
    assert_eq!(expected_bits(&Script::default()), []);
    let broken = Script::parse("b1 (");
    assert!(broken.tree().is_none(), "does not parse");
    assert_eq!(expected_bits(&broken), []);
}

#[test]
fn the_pay_expected_is_a_positive_pays_cash_and_nothing_for_a_cleaning() {
    assert_eq!(expected_pay(1000), Some(1000));
    assert_eq!(expected_pay(1), Some(1));
    assert_eq!(expected_pay(0), Some(0));
    assert_eq!(expected_pay(-10_128), Some(0), "cleaning a record");
    assert_eq!(expected_pay(-60_000), Some(0), "charged at accept");
    assert_eq!(expected_pay(-40_010), None, "a fine");
}

// take

#[test]
fn a_mission_offered_where_docked_is_accepted_there_without_landing() {
    let catalog = galaxy(vec![mission(200)]);
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A)])
        .expect("walks");
    assert_eq!(
        walked.done,
        [Done::Took {
            mission: A,
            stellar: StellarId(128),
            slot: 0,
            others: Vec::new(),
        }]
    );
    assert_eq!(session.landed(), Some(StellarId(128)));
    assert_eq!(held(&session), [A]);
}

#[test]
fn a_mission_offered_elsewhere_is_accepted_where_the_walker_lands() {
    let catalog = galaxy(vec![MissionRecord {
        avail_stel: 140,
        ..mission(200)
    }]);
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A)])
        .expect("walks");
    assert_eq!(
        walked.done,
        [Done::Took {
            mission: A,
            stellar: StellarId(140),
            slot: 0,
            others: Vec::new(),
        }]
    );
    assert_eq!(session.system(), SystemId(131));
    assert_eq!(session.landed(), Some(StellarId(140)));
    assert_eq!(held(&session), [A]);
}

#[test]
fn a_mission_offered_nowhere_reports_the_stellars_its_avail_stel_takes() {
    let catalog = galaxy(vec![MissionRecord {
        avail_stel: 140,
        avail_bits: Test::parse("b11"),
        ..mission(200)
    }]);
    let mut session = docked(&catalog);
    session.set_control_bit(bit(5), true);
    let failure = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A)])
        .expect_err("offered nowhere");
    assert_eq!(
        failure,
        WalkFailure {
            looked: vec![Looked {
                stellar: StellarId(140),
                system: SystemId(131),
                refusals: vec![("AvailBits", Refusal::AvailBits)],
            }],
            ..super::tests::failure(1, Step::Take(A), Fault::NotOffered, &[5])
        },
        "128 and 129 left out: AvailStel refuses them"
    );
    assert_eq!(held(&session), []);
}

#[test]
fn a_mission_whose_avail_stel_takes_no_stellar_reports_every_stellar() {
    let catalog = galaxy(vec![MissionRecord {
        avail_stel: 141,
        ..mission(200)
    }]);
    let mut session = docked(&catalog);
    let failure = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A)])
        .expect_err("offered nowhere");
    let refused = |stellar, system| Looked {
        stellar: StellarId(stellar),
        system: SystemId(system),
        refusals: vec![("AvailStel", Refusal::AvailStel)],
    };
    assert_eq!(
        failure.looked,
        [refused(129, 130), refused(140, 131), refused(128, 130)],
        "the docked stellar last"
    );
    assert_eq!(failure.fault, Fault::NotOffered);
}

#[test]
fn a_stellar_that_cannot_be_landed_on_is_skipped_and_reported_nowhere() {
    let catalog = atlas(vec![MissionRecord {
        avail_stel: 141,
        ..mission(200)
    }]);
    let mut session = docked(&catalog);
    let failure = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A)])
        .expect_err("offered nowhere");
    let refused = |stellar, system| Looked {
        stellar: StellarId(stellar),
        system: SystemId(system),
        refusals: vec![("AvailStel", Refusal::AvailStel)],
    };
    assert_eq!(
        failure.looked,
        [refused(129, 130), refused(140, 131), refused(128, 130)],
        "135 left out: it cannot be landed on"
    );
    assert_eq!(failure.fault, Fault::NotOffered);
}

#[test]
fn a_stellar_that_cannot_be_landed_on_before_the_one_offering_the_mission_is_passed_over() {
    let catalog = atlas(vec![MissionRecord {
        avail_stel: 140,
        ..mission(200)
    }]);
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A)])
        .expect("walks");
    assert_eq!(
        walked.done,
        [Done::Took {
            mission: A,
            stellar: StellarId(140),
            slot: 0,
            others: Vec::new(),
        }]
    );
    assert_eq!(session.landed(), Some(StellarId(140)));
}

#[test]
fn a_mission_with_no_record_is_offered_nowhere() {
    let catalog = galaxy(Vec::new());
    let mut session = docked(&catalog);
    assert_eq!(
        StoryWalker::new(&catalog).walk(&mut session, &[Step::Take(A)]),
        Err(failure(1, Step::Take(A), Fault::NotOffered, &[]))
    );
}

#[test]
fn an_accept_refused_is_reported() {
    let catalog = galaxy(vec![MissionRecord {
        cargo_type: 0,
        cargo_qty: 50,
        pickup_mode: 0,
        ..mission(200)
    }]);
    let mut session = docked(&catalog);
    assert_eq!(
        StoryWalker::new(&catalog).walk(&mut session, &[Step::Take(A)]),
        Err(failure(
            1,
            Step::Take(A),
            Fault::Refused(AcceptError::Refused(AcceptRefusal::NoHolds)),
            &[]
        ))
    );
}

// deliver

#[test]
fn a_mission_is_delivered_at_its_return_paid_and_its_bits_set() {
    let catalog = galaxy(vec![errand(200, 128, 140, "b10")]);
    let mut session = docked(&catalog);
    let cash = session.pilot().cash();
    let walked = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A), Step::Deliver(A)])
        .expect("walks");
    assert_eq!(
        walked.done[1],
        Done::Delivered {
            mission: A,
            stellar: StellarId(140),
            pay: 1000,
            others: Vec::new(),
        }
    );
    assert_eq!(session.landed(), Some(StellarId(140)));
    assert_eq!(session.pilot().cash(), cash + 1000);
    assert!(session.pilot().control_bit(bit(10)));
    assert_eq!(held(&session), []);
}

#[test]
fn a_mission_with_a_destination_picks_its_cargo_up_there_then_is_delivered_at_its_return() {
    let catalog = galaxy(vec![MissionRecord {
        travel_stel: 129,
        cargo_type: 0,
        cargo_qty: 2,
        pickup_mode: 1,
        drop_off_mode: 1,
        ..errand(200, 128, 140, "")
    }]);
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A), Step::Deliver(A)])
        .expect("walks");
    assert_eq!(
        walked.done[1],
        Done::Delivered {
            mission: A,
            stellar: StellarId(140),
            pay: 1000,
            others: Vec::new(),
        }
    );
    assert_eq!(session.pilot().total_cargo(), 0, "dropped off");
}

#[test]
fn a_mission_whose_ship_goals_are_unmet_is_not_completed() {
    let catalog = galaxy(vec![MissionRecord {
        ships: MissionShip {
            count: 1,
            goal: 0,
            ..MissionShip::NONE
        },
        ..errand(200, 128, 140, "b10")
    }]);
    let mut session = docked(&catalog);
    let failure = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A), Step::Deliver(A)])
        .expect_err("goals unmet");
    let active = session.pilot().missions().get(0).expect("held").clone();
    assert!(active.reached && !active.goals_met, "{active:?}");
    assert_eq!(
        failure,
        super::tests::failure(
            2,
            Step::Deliver(A),
            Fault::NotCompleted(Box::new(active)),
            &[]
        )
    );
}

// Ship goals.

/// [`errand`] 200, offered at 128 and completed at 140, with one ship of
/// `goal` of düde 150 (warships of govt 131 flying ship 200), in system
/// `syst`.
fn hunt(goal: i16, syst: i16) -> FakePilotCatalog {
    let mut catalog = galaxy(vec![MissionRecord {
        ships: MissionShip {
            count: 1,
            goal,
            syst,
            dude: Some(crate::catalog::DudeId(150)),
            ..MissionShip::NONE
        },
        ..errand(200, 128, 140, "b10")
    }]);
    catalog
        .ship_records
        .push(crate::testkit::ship(200, crate::testkit::FAST));
    catalog.dudes = vec![(
        crate::catalog::DudeId(150),
        crate::catalog::DudeRecord {
            ai_type: 3,
            govt: Some(crate::catalog::GovtId(131)),
            ships: vec![(crate::catalog::ShipId(200), 100)],
            booty: 0,
            info_types: 0,
        },
    )];
    catalog
}

#[test]
fn meeting_a_destroy_goal_lets_the_mission_be_delivered() {
    let catalog = hunt(0, -1);
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(
            &mut session,
            &[Step::Take(A), Step::MeetGoal(A), Step::Deliver(A)],
        )
        .expect("walks");
    assert_eq!(
        walked.done[1],
        Done::MetGoal {
            mission: A,
            others: Vec::new()
        }
    );
    assert!(held(&session).is_empty(), "completed");
}

#[test]
fn meeting_a_chase_off_goal_lets_the_mission_be_delivered() {
    let catalog = hunt(6, -1);
    let mut session = docked(&catalog);
    StoryWalker::new(&catalog)
        .walk(
            &mut session,
            &[Step::Take(A), Step::MeetGoal(A), Step::Deliver(A)],
        )
        .expect("walks");
    assert!(held(&session).is_empty(), "completed");
}

#[test]
fn meeting_an_escort_goal_lands_at_the_return_and_brings_the_escorts_in_there() {
    let catalog = hunt(3, 130);
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(
            &mut session,
            &[Step::Take(A), Step::MeetGoal(A), Step::Deliver(A)],
        )
        .expect("walks");
    assert!(held(&session).is_empty(), "completed");
    assert!(matches!(
        walked.done[2],
        Done::Delivered {
            stellar: StellarId(140),
            ..
        }
    ));
}

#[test]
fn meeting_an_escort_goal_lands_at_an_unreached_destination_first() {
    let mut catalog = hunt(3, 130);
    catalog.missions[0].travel_stel = 129;
    let mut session = docked(&catalog);
    StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A), Step::MeetGoal(A)])
        .expect("walks");
    let active = session.pilot().missions().get(0).expect("held").clone();
    assert!(active.reached, "landed at 129 on the way");
    assert!(active.goals_met);
    assert_eq!(session.landed(), Some(StellarId(140)), "then at the return");
}

#[test]
fn meeting_the_goal_of_a_mission_not_held_or_of_no_ship_goal_fails() {
    let catalog = hunt(0, -1);
    let mut session = docked(&catalog);
    let failure = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::MeetGoal(A)])
        .expect_err("not held");
    assert_eq!(failure.fault, Fault::NotActive);
    let catalog = galaxy(vec![errand(200, 128, 140, "b10")]);
    let mut session = docked(&catalog);
    let failure = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A), Step::MeetGoal(A)])
        .expect_err("no goal");
    assert_eq!(
        failure,
        super::tests::failure(2, Step::MeetGoal(A), Fault::NoShipGoal, &[])
    );
}

#[test]
fn a_mission_failed_before_its_return_is_not_completed() {
    let catalog = galaxy(vec![MissionRecord {
        on_accept: Script::parse("F200"),
        ..errand(200, 128, 140, "b10")
    }]);
    let mut session = docked(&catalog);
    let failure = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A), Step::Deliver(A)])
        .expect_err("failed");
    let Fault::NotCompleted(active) = failure.fault else {
        panic!("{failure}");
    };
    assert!(active.failed.is_some(), "{active:?}");
    assert_eq!(session.landed(), Some(StellarId(128)), "never landed");
}

#[test]
fn an_on_success_bit_cleared_again_is_reported_unset() {
    let catalog = galaxy(vec![
        errand(200, 128, 140, "b10 S201"),
        MissionRecord {
            on_accept: Script::parse("!b10"),
            ..mission(201)
        },
    ]);
    let mut session = docked(&catalog);
    assert_eq!(
        StoryWalker::new(&catalog).walk(&mut session, &[Step::Take(A), Step::Deliver(A)]),
        Err(failure(
            2,
            Step::Deliver(A),
            Fault::BitsNotSet(bits(&[10])),
            &[]
        ))
    );
}

#[test]
fn another_mission_completed_at_the_return_is_logged_and_its_pay_left_out() {
    let catalog = galaxy(vec![
        errand(200, 128, 140, ""),
        MissionRecord {
            pay_val: 300,
            ..errand(201, 128, 140, "")
        },
    ]);
    let mut session = docked(&catalog);
    let cash = session.pilot().cash();
    let walked = StoryWalker::new(&catalog)
        .walk(
            &mut session,
            &[Step::Take(A), Step::Take(B), Step::Deliver(A)],
        )
        .expect("walks");
    assert_eq!(
        walked.done[2],
        Done::Delivered {
            mission: A,
            stellar: StellarId(140),
            pay: 1000,
            others: vec![(B, Became::Completed)],
        },
        "both completed at 140"
    );
    assert_eq!(session.pilot().cash(), cash + 1300);
    assert_eq!(held(&session), []);
}

#[test]
fn another_mission_fining_at_the_return_leaves_the_pay_unchecked() {
    let catalog = galaxy(vec![
        errand(200, 128, 140, ""),
        MissionRecord {
            pay_val: -40_010,
            ..errand(201, 128, 140, "")
        },
    ]);
    let mut session = docked(&catalog);
    let cash = session.pilot().cash();
    let walked = StoryWalker::new(&catalog)
        .walk(
            &mut session,
            &[Step::Take(A), Step::Take(B), Step::Deliver(A)],
        )
        .expect("walks");
    let rise = session.pilot().cash() - cash;
    assert_ne!(rise, 1000, "fined");
    assert_eq!(
        walked.done[2],
        Done::Delivered {
            mission: A,
            stellar: StellarId(140),
            pay: rise,
            others: vec![(B, Became::Completed)],
        }
    );
}

#[test]
fn a_mission_completed_where_a_take_looks_is_logged() {
    let catalog = galaxy(vec![
        errand(200, 128, 129, ""),
        MissionRecord {
            avail_stel: 140,
            ..mission(201)
        },
    ]);
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A), Step::Take(B)])
        .expect("walks");
    assert_eq!(
        walked.done[1],
        Done::Took {
            mission: B,
            stellar: StellarId(140),
            slot: 0,
            others: vec![(A, Became::Completed)],
        },
        "completed at 129, on the way to 140"
    );
    assert_eq!(
        StoryWalker::new(&catalog).walk(&mut session, &[Step::Deliver(A)]),
        Err(failure(1, Step::Deliver(A), Fault::NotActive, &[]))
    );
}

/// What a walk of `take A, take B, deliver A` over `missions` logged for
/// the missions other than A as A was delivered.
fn others_at_delivery(missions: Vec<MissionRecord>) -> Vec<(MissionId, Became)> {
    let catalog = galaxy(missions);
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(
            &mut session,
            &[Step::Take(A), Step::Take(B), Step::Deliver(A)],
        )
        .expect("walks");
    let Some(Done::Delivered { others, .. }) = walked.done.last() else {
        panic!("{walked:?}");
    };
    others.clone()
}

#[test]
fn a_mission_gone_at_a_variant_of_its_return_reads_completed() {
    let catalog = with_variants(galaxy(vec![
        errand(200, 128, 129, ""),
        errand(201, 128, 151, ""),
    ]));
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(
            &mut session,
            &[Step::Take(A), Step::Take(B), Step::Deliver(A)],
        )
        .expect("walks");
    let Some(Done::Delivered { others, .. }) = walked.done.last() else {
        panic!("{walked:?}");
    };
    assert_eq!(others, &[(B, Became::Completed)], "129 is 151's variant");
}

#[test]
fn another_mission_left_as_it_was_is_not_logged() {
    let unchanged = others_at_delivery(vec![errand(200, 128, 140, ""), errand(201, 128, 129, "")]);
    assert_eq!(unchanged, [], "201 held, reached since its accept");
}

#[test]
fn another_missions_destination_reached_at_the_return_is_logged() {
    let reached = others_at_delivery(vec![
        errand(200, 128, 140, ""),
        MissionRecord {
            travel_stel: 140,
            ..errand(201, 128, 129, "")
        },
    ]);
    assert_eq!(reached, [(B, Became::Reached)]);
}

#[test]
fn another_mission_failed_by_the_missions_on_success_is_logged() {
    let failed = others_at_delivery(vec![
        errand(200, 128, 140, "F201"),
        errand(201, 128, 129, ""),
    ]);
    assert_eq!(failed, [(B, Became::Failed)]);
}

#[test]
fn another_mission_failed_before_and_ended_at_the_return_is_logged_failed() {
    let failed = others_at_delivery(vec![
        errand(200, 128, 140, ""),
        MissionRecord {
            on_accept: Script::parse("F201"),
            ..errand(201, 128, 140, "")
        },
    ]);
    assert_eq!(failed, [(B, Became::Failed)]);
}

#[test]
fn another_mission_aborted_by_the_missions_on_success_is_logged() {
    let aborted = others_at_delivery(vec![
        errand(200, 128, 140, "A201"),
        errand(201, 128, 129, ""),
    ]);
    assert_eq!(aborted, [(B, Became::Aborted)]);
}

#[test]
fn a_mission_started_by_the_missions_on_success_is_logged() {
    let catalog = galaxy(vec![
        errand(200, 128, 140, "S201"),
        errand(201, 129, 128, ""),
    ]);
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A), Step::Deliver(A)])
        .expect("walks");
    assert_eq!(
        walked.done[1],
        Done::Delivered {
            mission: A,
            stellar: StellarId(140),
            pay: 1000,
            others: vec![(B, Became::Started)],
        }
    );
    assert_eq!(held(&session), [B]);
}

#[test]
fn another_mission_failed_by_the_accept_is_logged() {
    let catalog = galaxy(vec![
        errand(200, 128, 129, ""),
        MissionRecord {
            on_accept: Script::parse("F200"),
            ..errand(201, 128, 129, "")
        },
    ]);
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A), Step::Take(B)])
        .expect("walks");
    assert_eq!(
        walked.done[1],
        Done::Took {
            mission: B,
            stellar: StellarId(128),
            slot: 1,
            others: vec![(A, Became::Failed)],
        }
    );
}

#[test]
fn a_pay_other_than_the_missions_is_reported() {
    let catalog = galaxy(vec![
        errand(200, 128, 140, "S202"),
        MissionRecord {
            flags: 0x0001,
            flags2: 0x0002,
            pay_val: 300,
            ..mission(202)
        },
    ]);
    let mut session = docked(&catalog);
    assert_eq!(
        StoryWalker::new(&catalog).walk(&mut session, &[Step::Take(A), Step::Deliver(A)]),
        Err(failure(
            2,
            Step::Deliver(A),
            Fault::WrongPay {
                expected: 1000,
                got: 1300
            },
            &[]
        )),
        "202 started and aborted at once, paying 300"
    );
}

#[test]
fn a_mission_not_held_is_not_active() {
    let catalog = galaxy(vec![errand(200, 128, 140, "")]);
    let mut session = docked(&catalog);
    assert_eq!(
        StoryWalker::new(&catalog).walk(&mut session, &[Step::Deliver(A)]),
        Err(failure(1, Step::Deliver(A), Fault::NotActive, &[]))
    );
}

#[test]
fn a_mission_with_no_return_is_undeliverable() {
    let catalog = galaxy(vec![mission(200)]);
    let mut session = docked(&catalog);
    assert_eq!(
        StoryWalker::new(&catalog).walk(&mut session, &[Step::Take(A), Step::Deliver(A)]),
        Err(failure(2, Step::Deliver(A), Fault::Undeliverable, &[]))
    );
}

#[test]
fn a_return_that_cannot_be_landed_on_is_reported() {
    let catalog = atlas(vec![errand(200, 128, 135, "b10")]);
    let mut session = docked(&catalog);
    assert_eq!(
        StoryWalker::new(&catalog).walk(&mut session, &[Step::Take(A), Step::Deliver(A)]),
        Err(failure(
            2,
            Step::Deliver(A),
            Fault::CannotLand(RelocateRefusal::NotLandable),
            &[]
        ))
    );
    assert_eq!(session.landed(), Some(StellarId(128)), "still docked");
    assert_eq!(held(&session), [A]);
}

#[test]
fn a_return_in_a_system_hidden_when_it_was_offered_is_landed_at_in_the_first_listing_it() {
    // 131 shows only once b50 is set, which the mission's accept sets: its
    // return, 140, had no system when it was offered.
    let mut catalog = galaxy(vec![MissionRecord {
        on_accept: Script::parse("b50"),
        ..errand(200, 128, 140, "b60")
    }]);
    catalog.visibility = vec![crate::catalog::SystemVisibility {
        id: SystemId(131),
        visibility: crate::control::Test::parse("b50"),
    }];
    let mut session = docked(&catalog);
    assert!(!session.galaxy().visible(SystemId(131)));
    let walked = StoryWalker::new(&catalog).walk(&mut session, &[Step::Take(A), Step::Deliver(A)]);
    assert!(walked.is_ok(), "{walked:?}");
    assert_eq!(session.landed(), Some(StellarId(140)));
    assert_eq!(session.system(), SystemId(131));
    assert!(session.control_bit(bit(60)), "completed");
}

#[test]
fn a_return_in_no_system_is_reported() {
    let catalog = galaxy(vec![errand(200, 128, 150, "")]);
    let mut session = docked(&catalog);
    assert_eq!(
        StoryWalker::new(&catalog).walk(&mut session, &[Step::Take(A), Step::Deliver(A)]),
        Err(failure(
            2,
            Step::Deliver(A),
            Fault::CannotLand(RelocateRefusal::NoSystem),
            &[]
        ))
    );
}

// pass

/// Mission `id`, offered at 128 and completed at 140, timed two days,
/// its `OnFailure` `on_failure`.
fn timed(id: i16, on_failure: &str) -> MissionRecord {
    MissionRecord {
        time_limit: 2,
        on_failure: Script::parse(on_failure),
        ..errand(id, 128, 140, "")
    }
}

#[test]
fn passing_days_takes_off_and_moves_the_date_on() {
    let catalog = galaxy(Vec::new());
    let mut session = docked(&catalog);
    let start = session.date();
    let walked = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::PassDays(3), Step::PassDays(2)])
        .expect("walks");
    assert_eq!(
        walked.done,
        [
            Done::Passed {
                days: 3,
                took_off: true,
                date: after(start, 4),
                failed: Vec::new(),
            },
            Done::Passed {
                days: 2,
                took_off: false,
                date: after(start, 6),
                failed: Vec::new(),
            },
        ],
        "the take-off's own day first"
    );
    assert_eq!(session.landed(), None, "in flight");
}

#[test]
fn a_mission_whose_deadline_lapses_while_days_pass_is_logged_failed() {
    let catalog = galaxy(vec![timed(200, "b40"), errand(201, 128, 140, "")]);
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(
            &mut session,
            &[Step::Take(A), Step::Take(B), Step::PassDays(3)],
        )
        .expect("walks");
    let Some(Done::Passed { failed, .. }) = walked.done.last() else {
        panic!("{walked:?}");
    };
    assert_eq!(failed, &[A], "the untimed one held");
    assert!(session.pilot().control_bit(bit(40)));
    assert_eq!(held(&session), [B]);
}

#[test]
fn an_unabortable_mission_failed_while_days_pass_is_logged_failed_while_held() {
    let catalog = galaxy(vec![MissionRecord {
        can_abort: 0,
        ..timed(200, "")
    }]);
    let mut session = docked(&catalog);
    let walked = StoryWalker::new(&catalog)
        .walk(&mut session, &[Step::Take(A), Step::PassDays(3)])
        .expect("walks");
    let Some(Done::Passed { failed, .. }) = walked.done.last() else {
        panic!("{walked:?}");
    };
    assert_eq!(failed, &[A]);
    assert_eq!(held(&session), [A], "held, failed");
}

/// A chance whose every roll is the last outcome.
struct LastRoll;

impl Chance for LastRoll {
    fn fires(&mut self, _percent: u8) -> bool {
        true
    }

    fn below(&mut self, n: u32) -> u32 {
        n - 1
    }

    fn roll(&mut self, sides: u16) -> u16 {
        sides.saturating_sub(1)
    }
}

#[test]
fn the_days_draw_on_the_day_chance() {
    let catalog = galaxy(vec![timed(200, "R(b20 b21)")]);
    let script = [Step::Take(A), Step::PassDays(3)];
    let mut session = docked(&catalog);
    StoryWalker::new(&catalog)
        .walk(&mut session, &script)
        .expect("walks");
    assert_eq!(
        session.pilot().control_bits().iter().collect::<Vec<_>>(),
        bits(&[21])
    );
    let mut session = docked(&catalog);
    StoryWalker::new(&catalog)
        .with_day_chance(LastRoll)
        .walk(&mut session, &script)
        .expect("walks");
    assert_eq!(
        session.pilot().control_bits().iter().collect::<Vec<_>>(),
        bits(&[20])
    );
}

// A chain.

/// Mission A, offered at 128, completed at 129 for 1000 credits and b10;
/// and B, offered at 140 once b10 is set (or `b_tests`), completed at
/// 128 for 500.
fn chain(b_tests: &str) -> FakePilotCatalog {
    galaxy(vec![
        errand(200, 128, 129, "b10"),
        MissionRecord {
            avail_stel: 140,
            avail_bits: Test::parse(b_tests),
            return_stel: 128,
            pay_val: 500,
            ..mission(201)
        },
    ])
}

const CHAIN: [Step; 4] = [
    Step::Take(A),
    Step::Deliver(A),
    Step::Take(B),
    Step::Deliver(B),
];

#[test]
fn a_chain_is_walked_from_its_first_mission_to_its_last() {
    let catalog = chain("b10");
    let mut session = docked(&catalog);
    let cash = session.pilot().cash();
    let walked = StoryWalker::new(&catalog)
        .walk(&mut session, &CHAIN)
        .expect("walks");
    assert_eq!(
        walked.done,
        [
            Done::Took {
                mission: A,
                stellar: StellarId(128),
                slot: 0,
                others: Vec::new(),
            },
            Done::Delivered {
                mission: A,
                stellar: StellarId(129),
                pay: 1000,
                others: Vec::new(),
            },
            Done::Took {
                mission: B,
                stellar: StellarId(140),
                slot: 0,
                others: Vec::new(),
            },
            Done::Delivered {
                mission: B,
                stellar: StellarId(128),
                pay: 500,
                others: Vec::new(),
            },
        ]
    );
    assert_eq!(session.pilot().cash(), cash + 1500);
    assert_eq!(held(&session), []);
}

#[test]
fn a_chain_broken_on_purpose_names_the_mission_and_the_field_that_failed() {
    let catalog = chain("b11");
    let mut session = docked(&catalog);
    let failure = StoryWalker::new(&catalog)
        .walk(&mut session, &CHAIN)
        .expect_err("broken");
    assert_eq!(
        failure,
        WalkFailure {
            looked: vec![Looked {
                stellar: StellarId(140),
                system: SystemId(131),
                refusals: vec![("AvailBits", Refusal::AvailBits)],
            }],
            ..super::tests::failure(3, Step::Take(B), Fault::NotOffered, &[10])
        }
    );
    assert_eq!(
        failure.to_string(),
        "step 3 (take 201) failed: mission 201 is offered nowhere\n  \
         stellar 140 in system 131: AvailBits (AvailBits)\n  \
         bits set: b10"
    );
}

// The report.

#[test]
fn each_fault_is_reported_with_its_mission() {
    let report = |at, fault| failure(2, at, fault, &[3, 9]).to_string();
    let active = ActiveMission {
        mission: A,
        destination: None,
        return_to: None,
        cargo_type: None,
        cargo_tons: 0,
        pay: 0,
        days_left: None,
        deadline: None,
        reached: true,
        goals_met: false,
        aboard: false,
        failed: None,
        ships: crate::mission::SpecialShips::default(),
        aux: crate::mission::aux::AuxShips::default(),
    };
    for (at, fault, line) in [
        (
            Step::Take(A),
            Fault::Refused(AcceptError::NotOffered),
            "mission 200 was refused: the mission is not offered here",
        ),
        (
            Step::Deliver(A),
            Fault::NotActive,
            "mission 200 is not active",
        ),
        (
            Step::Deliver(A),
            Fault::NotCompleted(Box::new(active.clone())),
            &format!("mission 200 was not completed: {active:?}"),
        ),
        (
            Step::Deliver(A),
            Fault::WrongPay {
                expected: 5,
                got: 7,
            },
            "mission 200 paid 7, not 5",
        ),
        (
            Step::Deliver(A),
            Fault::BitsNotSet(bits(&[4, 6])),
            "mission 200 left unset: b4 b6",
        ),
        (
            Step::Deliver(A),
            Fault::CannotLand(RelocateRefusal::NoSystem),
            "mission 200: cannot land: no such system",
        ),
        (
            Step::Deliver(A),
            Fault::Undeliverable,
            "mission 200 has no return stellar",
        ),
        (
            Step::MeetGoal(A),
            Fault::NoShipGoal,
            "mission 200 has no ship goal to meet",
        ),
        (
            Step::PassDays(3),
            Fault::NotActive,
            "the step is not active",
        ),
    ] {
        assert_eq!(
            report(at, fault),
            format!("step 2 ({at}) failed: {line}\n  bits set: b3 b9")
        );
    }
}
