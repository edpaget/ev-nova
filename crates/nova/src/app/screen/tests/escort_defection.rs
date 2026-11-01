//! The router over an escort's defection
//! (rdm:phase/play-feel-8/phase-7-escort-defection-dialog): a hired
//! escort the cash can't pay defects, and `_DoEscortPayment` says so in a
//! text dialog, `_ShowTextDialog(0, 0, 0)` (@0x40301), not on the
//! message line. At a jump's arrival (@0x6c620) the dialog takes the
//! faded branch and fades the new system in from black
//! (@0x5af96-0x5b038), and the defection has put the white out
//! (@0x40298-0x402aa), so no white fades back. At take-off
//! (`_DoEscortLand` @0x4085a) it shows unfaded over the spaceport, after
//! the escorts' report, and holds the leave. Either way the last `dësc`
//! loaded's after-movie plays once it closes (@0x5b40b-0x5b44f).

use super::jump_white::{black, jump_ready, lit_from, plot_beta, tick_past_the_arrival, white};
use super::shop_text_movies::{
    empty_shops_built, empty_shops_with, port_fork, started_since_landing,
};
use super::*;
use nova_sim::hire::{DEFECTED_ONE, DEFECTED_SOME};
use nova_sim::{RuleKey, RuleSource, Rulebook};
use nova_view::ui::faded_texts::TEXT_FADE_IN_FOR;

/// The wage of [`hired`]'s escorts, kept as they were hired at it.
const WAGE: i64 = 1_000;

/// The rule set the escorts are paid by: the wage they were hired at
/// ([`RuleKey::EscortWage`]'s other reading), as the fixture's ship has
/// no `Cost` to work one out from.
fn kept_wage() -> Rulebook {
    Rulebook::default().with_override(RuleKey::EscortWage, RuleSource::Bible)
}

/// A First hired at [`WAGE`] a day.
fn hired() -> nova_sim::Escort {
    nova_sim::Escort {
        ship: nova_sim::ShipId(128),
        reserves: nova_sim::Reserves::full(10.0, 10.0, 100.0),
        order: None,
        carried: false,
        wage: Some(WAGE),
        person: None,
        mark: None,
    }
}

/// Ada with `escorts` hired escorts holding `cash` credits.
fn paying(escorts: usize, cash: i64) -> impl FnOnce(&mut nova_sim::Pilot) {
    move |ada| {
        ada.escorts = vec![hired(); escorts];
        ada.cash = cash;
    }
}

/// The text shown over the spaceport, if any.
fn port_text(screen: &AppScreen) -> Option<String> {
    spaceport(screen)
        .mission_text()
        .map(|text| text.lines().join(" "))
}

/// The text shown over flight, if any.
fn flight_text(screen: &AppScreen) -> Option<String> {
    screen
        .flight_mission_text()
        .map(|text| text.lines().join(" "))
}

/// Ticks `screen` until it shows flight, bounded.
fn until_flight(screen: &mut AppScreen) {
    for _ in 0..600 {
        if screen.showing() == Showing::Flight {
            return;
        }
        screen.tick(Duration::from_millis(16));
    }
    panic!("never back to flight: {:?}", screen.showing());
}

// At take-off.

#[test]
fn a_take_offs_defection_shows_over_the_spaceport_and_holds_the_leave_through_its_movie() {
    let (mut screen, reel) = empty_shops_with(kept_wage(), |fork| fork, paying(1, WAGE - 1));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Spaceport, "held");
    assert_eq!(port_text(&screen).as_deref(), Some(DEFECTED_ONE));
    assert!(
        flight(&screen)
            .session()
            .expect("flying")
            .landed()
            .is_some(),
        "the ship still landed under it (@0x601a0, before @0x6024c)"
    );
    assert!(!flight(&screen).screen_faded(), "unfaded");
    assert!(pilot(&screen).escorts().is_empty(), "defected");
    assert_eq!(started_since_landing(&reel), [], "not before it closes");
    screen.input(&key(Key::Enter, true));
    assert_eq!(
        started_since_landing(&reel),
        [Call::Start("Port.mov".into())],
        "the stellar's dësc was the last loaded (@0x5f96b)"
    );
    assert_eq!(screen.showing(), Showing::Spaceport, "held through it");
    until_flight(&mut screen);
    assert!(
        flight(&screen)
            .message()
            .is_none_or(|said| !said.contains(DEFECTED_ONE)),
        "not said on the line: {:?}",
        flight(&screen).message()
    );
    assert_eq!(pilot(&screen).cash, WAGE - 1, "no second day paid");
}

#[test]
fn a_take_offs_defection_follows_the_escorts_report() {
    let kit = |ada: &mut nova_sim::Pilot| {
        paying(2, 0)(ada);
        ada.escorts[0].wage = None;
        ada.escorts[0].mark = Some(nova_sim::fleet::EscortMark::Sell);
    };
    let (mut screen, _) = empty_shops_with(kept_wage(), |fork| fork, kit);
    screen.input(&key(Key::Escape, true));
    let report = port_text(&screen).expect("the report");
    assert_ne!(report, DEFECTED_ONE, "the report first (@0x407e8)");
    screen.input(&key(Key::Enter, true));
    for _ in 0..60 {
        if port_text(&screen).as_deref() == Some(DEFECTED_ONE) {
            break;
        }
        screen.tick(Duration::from_millis(16));
    }
    assert_eq!(port_text(&screen).as_deref(), Some(DEFECTED_ONE), "then it");
    assert_eq!(screen.showing(), Showing::Spaceport, "still held");
    screen.input(&key(Key::Enter, true));
    until_flight(&mut screen);
    assert!(pilot(&screen).escorts().is_empty());
}

#[test]
fn a_q_leaves_defection_holds_its_leave_too() {
    let leave =
        |fork: ForkBuilder| fork.resource(StrList::TYPE, 25048, None, &str_list(&["Off you go."]));
    let (mut screen, _) = empty_shops_with(kept_wage(), leave, paying(3, WAGE - 1));
    flight_mut(&mut screen).change_sim(|sim| {
        sim.session_mut().expect("landed").run_set(
            &nova_sim::control::SetExpr::parse("Q25048").expect("parses"),
            &mut nova_sim::chance::NeverFires,
        );
    });
    screen.tick(Duration::from_millis(16));
    assert_eq!(screen.showing(), Showing::Spaceport, "held");
    assert_eq!(port_text(&screen).as_deref(), Some(DEFECTED_SOME), "#303");
    screen.input(&key(Key::Enter, true));
    until_flight(&mut screen);
    assert!(pilot(&screen).escorts().is_empty());
}

// At an arrival.

/// [`empty_shops_with`] over [`jump_ready`]'s fork, Ada with one escort
/// hired at [`WAGE`] and just enough cash for the take-off's day, so the
/// jump's day can't pay it; left, flown out of the take-off's grace, a
/// course plotted to Beta and lit white by the warp.
fn lit_with_a_defector() -> (AppScreen, Rc<MockMoviePlayer>) {
    let (screen, reel) = plotted_with_a_defector(Prefs::default());
    (lit_from(screen), reel)
}

/// [`lit_with_a_defector`] under `prefs`, before J is pressed.
fn plotted_with_a_defector(prefs: Prefs) -> (AppScreen, Rc<MockMoviePlayer>) {
    let built = jump_ready(&port_fork());
    let (mut screen, reel) = empty_shops_built(kept_wage(), prefs, built, paying(1, WAGE));
    screen.input(&key(Key::Escape, true));
    until_flight(&mut screen);
    assert_eq!(pilot(&screen).escorts().len(), 1, "paid at take-off");
    assert_eq!(pilot(&screen).cash, 0);
    for _ in 0..120 {
        screen.tick(TICK);
    }
    plot_beta(&mut screen);
    (screen, reel)
}

#[test]
fn an_arrivals_defection_fades_the_new_system_in_from_black_and_no_white_comes_back() {
    let (mut screen, reel) = lit_with_a_defector();
    let before = started_since_landing(&reel).len();
    tick_past_the_arrival(&mut screen);
    assert_eq!(screen.showing(), Showing::MissionText);
    assert_eq!(flight_text(&screen).as_deref(), Some(DEFECTED_ONE));
    assert!(pilot(&screen).escorts().is_empty(), "defected");
    assert_ne!(
        flight(&screen).message(),
        Some(DEFECTED_ONE),
        "not on the line"
    );
    assert_eq!(white(&screen), None, "the white is gone");
    assert_eq!(black(&screen), Some(255), "from black");
    let frame = Duration::from_nanos(1_000_000_000 / 60);
    let mut faded_for = Duration::ZERO;
    while faded_for < TEXT_FADE_IN_FOR + frame {
        screen.tick(frame);
        faded_for += frame;
    }
    assert_eq!(black(&screen), None, "faded in");
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    assert_eq!(
        started_since_landing(&reel)[before..],
        [Call::Start("Port.mov".into())],
        "the last dësc loaded's after-movie"
    );
    until_flight(&mut screen);
    for _ in 0..3 {
        screen.tick(frame);
        assert!(flight(&screen).jump_effect().is_none(), "no white fade-in");
        assert_eq!(white(&screen), None);
    }
}

#[test]
fn with_hyperspace_effects_off_an_arrivals_defection_shows_unfaded() {
    let prefs = Prefs {
        hyperspace_effects: false,
        ..Prefs::default()
    };
    let (mut screen, _) = plotted_with_a_defector(prefs);
    screen.input(&key(Key::Up, true));
    for _ in 0..1800 {
        if ship(&screen).position.length() >= nova_sim::hyperspace::MIN_JUMP_DISTANCE {
            break;
        }
        screen.tick(TICK);
    }
    screen.input(&key(Key::Up, false));
    screen.input(&key(Key::Char('j'), true));
    screen.input(&key(Key::Char('j'), false));
    for _ in 0..600 {
        if flight(&screen)
            .session()
            .expect("flying")
            .jumping()
            .is_some()
        {
            break;
        }
        screen.tick(TICK);
    }
    tick_past_the_arrival(&mut screen);
    assert_eq!(flight_text(&screen).as_deref(), Some(DEFECTED_ONE));
    assert_eq!(black(&screen), None, "no fade-in from black");
}

/// [`port_fork`] with mission 130, whose `OnFailure` starts mission 131,
/// briefed "Briefed." (`dësc` 5000).
fn briefing_fork() -> ForkBuilder {
    let mission = |on_failure: &[u8], brief: i16| {
        let mut bytes = venue_mission(0);
        bytes[0x34..0x36].copy_from_slice(&brief.to_be_bytes());
        bytes[0x458..0x458 + on_failure.len()].copy_from_slice(on_failure);
        bytes
    };
    port_fork()
        .resource(Mission::TYPE, 130, Some(b"Timed"), &mission(b"S131", -1))
        .resource(Mission::TYPE, 131, Some(b"Next"), &mission(b"", 5000))
        .resource(Desc::TYPE, 5000, None, &desc_of("Briefed."))
}

/// `ada` with mission 130 active in slot 0, two days left: the take-off's
/// day leaves one, and the jump's day misses it, failing it at the
/// arrival.
fn timed(ada: &mut nova_sim::Pilot) {
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(ada)).expect("JSON");
    save["missions"] = serde_json::json!([{
        "slot": 0,
        "mission": 130,
        "destination": null,
        "return": null,
        "cargo_type": null,
        "cargo_tons": 0,
        "pay": 0,
        "days_left": 2,
        "deadline": null,
        "reached": false,
        "goals_met": false,
        "aboard": false,
        "failed": null,
        "ships": {"system": null, "left": 0, "arrived": 0, "appear": -1, "name": "", "subtitle": "", "destroyed": 0, "boarded": 0, "disabled": 0, "chased_off": 0, "uniform_type": null, "ship_done": false},
        "aux": {"countdown": 0, "arrived": 0, "left": 0},
    }]);
    *ada = nova_sim::save::decode(&save.to_string()).expect("a pilot");
}

#[test]
fn an_arrivals_own_text_fades_in_first_and_the_defection_follows_unfaded() {
    let built = jump_ready(&briefing_fork());
    let kit = |ada: &mut nova_sim::Pilot| {
        paying(1, WAGE)(ada);
        timed(ada);
    };
    let (mut screen, _) = empty_shops_built(kept_wage(), Prefs::default(), built, kit);
    screen.input(&key(Key::Escape, true));
    until_flight(&mut screen);
    for _ in 0..120 {
        screen.tick(TICK);
    }
    plot_beta(&mut screen);
    let mut screen = lit_from(screen);
    tick_past_the_arrival(&mut screen);
    assert_eq!(
        flight_text(&screen).as_deref(),
        Some("Briefed."),
        "the arrival's text first"
    );
    assert_eq!(black(&screen), Some(255), "faded in from black");
    let frame = Duration::from_nanos(1_000_000_000 / 60);
    let mut waited = Duration::ZERO;
    while waited < TEXT_FADE_IN_FOR + frame {
        screen.tick(frame);
        waited += frame;
    }
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    assert_eq!(
        flight_text(&screen).as_deref(),
        Some(DEFECTED_ONE),
        "then it"
    );
    assert_eq!(black(&screen), None, "unfaded");
    assert_eq!(white(&screen), None, "no white");
}
