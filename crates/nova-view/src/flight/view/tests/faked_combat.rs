//! The combat keys and the fight's outputs on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-14-view-tests-on-port): combat.rs's
//! tests that ran on a real session, converted under their own names.
//! What the session does with each command and held key is nova-sim's,
//! named in the commit that moved them.

use super::faked_screen::{in_flight, sent_count, sent_since};
use super::faked_world::raise;
use super::*;
use nova_sim::FireSound;
use nova_sim::flight_frame::FlightEvent;
use nova_sim::{TargetPick, combat::PLAYER_FIRE_PRIORITY};

/// The player's blaster fired at the centre, heard at the player's
/// priority.
fn blaster_fired() -> CombatEvent {
    CombatEvent::Fired {
        ship: ShipRef::Player,
        weapon: BLASTER,
        at: Vec2::ZERO,
        sound: Some(FireSound::at(PLAYER_FIRE_PRIORITY)),
    }
}

/// The combat sounds `view` gives.
fn combat_heard(view: &mut Faked) -> Vec<nova_sim::SoundId> {
    view.drained_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Combat(sound) => Some(sound.sound),
            _ => None,
        })
        .collect()
}

/// The fire and safety keys the step after now holds.
fn held_in_the_next_step(view: &mut Faked) -> ((bool, bool), bool) {
    let from = view.sim.inputs().len();
    view.tick(TICK);
    let input = view.sim.inputs()[from];
    (input.fire, input.weapon_safety)
}

#[test]
fn a_look_whose_sheet_cannot_be_read_still_sounds_and_is_reported_once() {
    // The weapons whose looks are read are the flight's list.
    let mut catalog = armed(0, &[BLASTER]);
    catalog.looks[0].1 = Ok(WeaponLook {
        sheet: Some(Err("no spïn 3005".to_owned())),
        ..look("Blaster", 208, None)
    });
    catalog.booms.push((
        129,
        Ok(BoomLook {
            sheet: Err("no spïn 401".to_owned()),
            advance: 1.0,
            sound: None,
        }),
    ));
    catalog.star_sheet = Ok(star_sheet());
    let mut fake = FakeFlight::new();
    fake.set_frame(in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]));
    fake.set_weapons(vec![BLASTER, UNSEEN]);
    let opening = fake.opening();
    let mut view = FlightView::over(
        Rc::new(catalog),
        fake,
        opening,
        &Rulebook::default(),
        SharedChance::default(),
    );
    assert_eq!(
        view.take_diagnostics(),
        [
            Diagnostic::Unreadable("wëap 128: no spïn 3005".to_owned()),
            Diagnostic::Unreadable("no wëap 150".to_owned()),
            Diagnostic::Unreadable("bööm 129: no spïn 401".to_owned()),
        ],
        "and the unseen weapon, which has no look at all"
    );
    raise(&mut view, &[blaster_fired()]);
    assert_eq!(
        combat_heard(&mut view),
        [nova_sim::SoundId(208)],
        "the blaster is heard"
    );
    for _ in 0..3 {
        view.tick(TICK);
    }
    assert_eq!(view.take_diagnostics(), [], "once");
}

#[test]
fn tab_targets_the_next_npc_and_alt_r_the_nearest() {
    let mut view = in_flight();
    tap(&mut view, Key::Tab);
    view.input(&held(Key::Tab));
    view.input(&key(Key::Tab, false));
    view.input(&key(Key::Alt, true));
    tap(&mut view, NEAREST_KEY);
    view.input(&held(NEAREST_KEY));
    assert_eq!(
        sent_since(&view, 0),
        [
            FlightCommand::SelectTarget(TargetPick::Next),
            FlightCommand::SelectTarget(TargetPick::Nearest),
        ],
        "a repeat of either sends nothing"
    );
    assert_eq!((TARGET_KEY, NEAREST_KEY), (Key::Tab, Key::Char('r')));
}

#[test]
fn a_target_key_press_beeps_once_and_its_repeat_is_silent() {
    // The beep is the flight's, raised by the pick it is sent.
    let mut view = in_flight();
    let frame = view.frame.clone();
    view.sim.queue_answer_telling(
        Answer::Done,
        frame,
        vec![FlightEvent::Sound(SimSound::TargetSelected)],
    );
    view.input(&key(Key::Tab, true));
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TargetSelected)]);
    let from = sent_count(&view);
    view.input(&held(Key::Tab));
    view.input(&key(Key::Tab, false));
    view.input(&key(NEAREST_KEY, true));
    view.input(&held(NEAREST_KEY));
    assert_eq!(
        sent_since(&view, from),
        [FlightCommand::SelectTarget(TargetPick::NearestThreat)],
        "a repeat of Tab or R asks nothing"
    );
    assert_eq!(view.take_sounds(), [], "and raises nothing");
}

#[test]
fn the_flight_keys_are_held_until_let_go_or_released() {
    let mut view = in_flight();
    assert!(!view.holds_keys());
    view.input(&key(Key::Up, true));
    assert!(view.holds_keys());
    view.input(&key(Key::Up, false));
    assert!(!view.holds_keys(), "let go");
    view.input(&key(Key::Up, true));
    view.release_keys();
    assert!(!view.holds_keys(), "released");
}

#[test]
fn letting_go_of_the_keys_lets_go_of_the_fire_keys() {
    let mut view = in_flight();
    view.input(&key(FIRE_KEY, true));
    view.input(&key(SECONDARY_KEY, true));
    assert_eq!(held_in_the_next_step(&mut view).0, (true, true));
    view.release_keys();
    assert_eq!(held_in_the_next_step(&mut view).0, (false, false));
}

#[test]
fn w_selects_the_next_secondary_and_alt_w_the_one_before() {
    let mut view = in_flight();
    tap(&mut view, SELECT_KEY);
    view.input(&held(SELECT_KEY));
    view.input(&key(Key::Alt, true));
    tap(&mut view, SELECT_KEY);
    view.release_keys();
    tap(&mut view, SELECT_KEY);
    assert_eq!(
        sent_since(&view, 0),
        [
            FlightCommand::SelectWeapon { backwards: false },
            FlightCommand::SelectWeapon { backwards: true },
            FlightCommand::SelectWeapon { backwards: false },
        ],
        "a repeat sends nothing, and the release lets go of Option"
    );
    assert_eq!(SELECT_KEY, Key::Char('w'));
}

#[test]
fn s_pressed_while_the_map_is_open_does_nothing_once_it_closes() {
    let mut view = in_flight();
    tap(&mut view, MAP);
    assert!(view.map_open());
    let from = sent_count(&view);
    view.input(&key(DESELECT_KEY, true));
    tap(&mut view, MAP);
    assert!(!view.map_open());
    assert!(!held_in_the_next_step(&mut view).1, "no safety held");
    assert_eq!(
        sent_since(&view, from),
        [FlightCommand::CueCourse],
        "the map's close alone"
    );
}

#[test]
fn the_combat_keys_do_nothing_while_the_map_is_open() {
    let mut view = in_flight();
    tap(&mut view, MAP);
    let from = sent_count(&view);
    for k in [
        Key::Tab,
        NEAREST_KEY,
        SELECT_KEY,
        DESELECT_KEY,
        FIRE_KEY,
        SECONDARY_KEY,
    ] {
        view.input(&key(k, true));
    }
    tap(&mut view, MAP);
    assert_eq!(
        sent_since(&view, from),
        [FlightCommand::CycleMapSelection, FlightCommand::CueCourse],
        "Tab is the map's, and the map's close; no combat command"
    );
    assert_eq!(
        held_in_the_next_step(&mut view),
        ((false, false), false),
        "no fire or safety held"
    );
}

#[test]
fn the_sounds_are_the_sessions_then_the_fights() {
    let mut view = super::faked_world::world(armed(0, &[BLASTER]), sol());
    view.sim
        .queue_event(FlightEvent::Sound(SimSound::ThrustStarted));
    raise(&mut view, &[blaster_fired()]);
    assert_eq!(
        view.drained_sounds(),
        [
            Sound::Sim(SimSound::ThrustStarted),
            Sound::Combat(CombatSound {
                sound: nova_sim::SoundId(208),
                offset: (0, 0),
                priority: u16::from(PLAYER_FIRE_PRIORITY),
                until_stopped: false,
            })
        ]
    );
}

#[test]
fn a_first_tick_eject_rolls_no_puff_for_the_player() {
    // The pod's player intact, its old ship's hulk breaking up from the
    // start of its `DeathDelay`, as the session leaves them.
    let pod = record::player(895, Vec2::ZERO, 0.0);
    let hulk = FrameShip {
        condition: Condition::Dying {
            ticks_left: 100,
            starting: true,
        },
        ..record::npc(1, 128, Vec2::new(5.0, 0.0), 0.0)
    };
    let breaking = super::super::dying(
        &[pod, hulk],
        &Ok(sheet()),
        &std::collections::BTreeMap::new(),
    );
    assert_eq!(breaking.len(), 1, "the hulk alone");
    assert_eq!((breaking[0].ticks_left, breaking[0].starting), (100, true));
}

#[test]
fn only_a_living_pilot_may_be_saved() {
    // Whether the pilot may be saved is the adapter's and the session's
    // (`SessionFlight::may_save`); a flight that never started holds,
    // fades and ends nothing.
    let mut fake = FakeFlight::new();
    fake.fail("no chär to start from");
    let mut failed = scripted_over(catalog(), fake, &Rulebook::default());
    assert!(!failed.take_hold_expired());
    assert!(!failed.game_over());
    failed.begin_fade();
    failed.tick(Duration::from_secs(2));
    assert!(!failed.game_over());
}
