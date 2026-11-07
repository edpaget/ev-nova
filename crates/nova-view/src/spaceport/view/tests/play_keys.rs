//! The play keys on touch over the spaceport and each of its screens
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps): told touch, a tap
//! on Map, Pilot or Missions asks for what the galaxy map, player info or
//! mission info key asks for there, and nothing where the key asks for
//! nothing. Each opening the keys make is listed and pinned: over the
//! spaceport and each venue, the mission screens, an offer over the
//! spaceport or a venue, and the accept's briefing that opens its map.

use super::*;
use crate::input::{Device, Surroundings};
use crate::keymap::PlayCommand;
use crate::keymap::test_keys::{MISSION_INFO_KEY, PLAYER_INFO_KEY};
use crate::touch::testkit::Fingers;
use crate::ui::touch_aids::play_keys::PLAY_KEYS;

/// The map key's default.
const MAP_KEY: Key = Key::KeyM;

fn key_of(command: Command) -> Key {
    match command {
        Command::Play(PlayCommand::GalaxyMap) => MAP_KEY,
        Command::Play(PlayCommand::PlayerInfo) => PLAYER_INFO_KEY,
        Command::Play(PlayCommand::MissionInfo) => MISSION_INFO_KEY,
        other => panic!("{other:?} is no play key"),
    }
}

/// What the spaceport asked its owner for: the map, player info and
/// mission info.
type Asked = (Option<PortMap>, bool, bool);

fn asked(view: &mut SpaceportView) -> Asked {
    (
        view.take_map_request(),
        view.take_player_info_request(),
        view.take_mission_info_request(),
    )
}

/// Told touch, a tap on `command`'s control.
fn tapped(build: fn() -> SpaceportView, command: Command) -> Asked {
    let mut view = build();
    view.input(&Input::Surroundings(Surroundings::of(Device::Touch)));
    let _ = asked(&mut view);
    assert!(view.play_keys().shown(), "shown");
    let at = view.play_keys().rect(command).expect("a play key").center();
    let mut fingers = Fingers::on(&mut view);
    fingers.down(1, at);
    fingers.up(1, at);
    asked(&mut view)
}

/// `command`'s key pressed and let go of.
fn keyed(build: fn() -> SpaceportView, command: Command) -> Asked {
    let mut view = build();
    let _ = asked(&mut view);
    let k = key_of(command);
    for pressed in [true, false] {
        view.input(&Input::Key {
            key: k,
            pressed,
            repeat: false,
        });
    }
    asked(&mut view)
}

fn port() -> SpaceportView {
    informed(missioned())
}

fn trade() -> SpaceportView {
    let mut view = port().with_trade(
        Ok(trade_template()),
        Ok(quantity_template()),
        exchange(0),
        COLORS,
    );
    click_item(&mut view, 7);
    assert!(view.open_trade().is_some());
    view
}

fn outfitter() -> SpaceportView {
    let mut view = outfitting(Ok(outfit_template()));
    click_item(&mut view, 8);
    assert!(view.open_outfitter().is_some());
    view
}

fn shipyard() -> SpaceportView {
    let mut view = shipbuying(Ok(shipyard_template()));
    click_item(&mut view, 9);
    assert!(view.open_shipyard().is_some());
    view
}

fn computer() -> SpaceportView {
    let mut view = port();
    click_item(&mut view, 10);
    view.open_mission_computer(Ok(offers()));
    let _ = view.take_computer_text_request();
    view
}

fn mission_info() -> SpaceportView {
    in_mission_info(2)
}

fn player_info() -> SpaceportView {
    let mut view = port();
    view.open_player_info(pilot_info());
    view
}

/// The marks of [`briefing`]'s map.
fn marks() -> nova_sim::MapMarks {
    nova_sim::MapMarks {
        green: Some(nova_sim::SystemId(131)),
        multi_arrows: true,
    }
}

/// A mission accepted from the computer: its briefing in front, which
/// opens its marked map on the map key.
fn briefing() -> SpaceportView {
    let mut view = computer();
    view.show_mission_texts(vec![MissionText {
        kind: MissionTextKind::Brief,
        mission: nova_sim::MissionId(130),
        text: "Go.".to_owned(),
        graphic: None,
        movie: None,
        map: Some(marks()),
    }]);
    assert!(view.mission_text().is_some(), "the briefing in front");
    view
}

/// [`venues`] with an offer open over `venue`, as it asks for one.
fn offer_at(venue: i16) -> SpaceportView {
    let mut view = venues();
    assert_eq!(view.take_offer_request(), Some(SPACEPORT));
    if venue != SPACEPORT {
        let item = match venue {
            BAR => 11,
            TRADE_CENTER => 7,
            OUTFITTER => 8,
            _ => 9,
        };
        click_item(&mut view, item);
        serve_openings(&mut view);
        view.tick(FIRST_OFFER_DELAY);
        assert_eq!(view.take_offer_request(), Some(venue));
    }
    view.open_offer(offered());
    assert!(view.offer_dialog().is_some(), "{venue}: an offer open");
    view
}

fn spaceport_offer() -> SpaceportView {
    offer_at(SPACEPORT)
}

fn bar_offer() -> SpaceportView {
    offer_at(BAR)
}

fn trade_offer() -> SpaceportView {
    offer_at(TRADE_CENTER)
}

fn outfitter_offer() -> SpaceportView {
    offer_at(OUTFITTER)
}

fn shipyard_offer() -> SpaceportView {
    offer_at(SHIPYARD)
}

/// A screen's name and how to build it.
type Shown = (&'static str, fn() -> SpaceportView);

/// Each screen the play keys show over.
fn screens() -> Vec<Shown> {
    vec![
        ("spaceport", port),
        ("trade center", trade),
        ("outfitter", outfitter),
        ("shipyard", shipyard),
        ("shipyard's info", in_the_shipyards_panel),
        ("bar", in_the_bar),
        ("news", in_the_news),
        ("hire's info", in_the_hire_panel),
        ("mission computer", computer),
        ("mission info", mission_info),
        ("player info", player_info),
        ("briefing", briefing),
        ("spaceport's offer", spaceport_offer),
        ("bar's offer", bar_offer),
        ("trade center's offer", trade_offer),
        ("outfitter's offer", outfitter_offer),
        ("shipyard's offer", shipyard_offer),
    ]
}

const MAP: Command = Command::Play(PlayCommand::GalaxyMap);
const PILOT: Command = Command::Play(PlayCommand::PlayerInfo);
const MISSIONS: Command = Command::Play(PlayCommand::MissionInfo);

/// Asked for the map `map` opens over.
const fn map_of(map: PortMap) -> Asked {
    (Some(map), false, false)
}

/// The map [`offered`]'s offer opens, over whatever it is open over.
const fn offer() -> PortMap {
    PortMap::Dialog(MapOpener::Offer(nova_sim::MissionId(141)))
}

const PLAYER_INFO: Asked = (None, true, false);
const MISSION_INFO: Asked = (None, false, true);

/// One opening: the screen's name and build, the key's command, and what
/// it asks for.
type Opening = (&'static str, fn() -> SpaceportView, Command, Asked);

/// Every opening the play keys make from the spaceport and its screens,
/// one row each.
fn openings() -> Vec<Opening> {
    use crate::flight::PortScreen as S;
    use crate::spaceport::port_map::PortMap::{Bar, Dialog, Screen};
    use nova_sim::MissionId;
    vec![
        ("spaceport", port, MAP, map_of(Screen(S::Spaceport))),
        ("spaceport", port, PILOT, PLAYER_INFO),
        ("spaceport", port, MISSIONS, MISSION_INFO),
        ("trade center", trade, MAP, map_of(Screen(S::Exchange))),
        ("trade center", trade, PILOT, PLAYER_INFO),
        ("trade center", trade, MISSIONS, MISSION_INFO),
        ("outfitter", outfitter, MAP, map_of(Screen(S::Outfitter))),
        ("outfitter", outfitter, MISSIONS, MISSION_INFO),
        ("shipyard", shipyard, MAP, map_of(Screen(S::Shipyard))),
        ("shipyard", shipyard, PILOT, PLAYER_INFO),
        ("shipyard", shipyard, MISSIONS, MISSION_INFO),
        (
            "shipyard's info",
            in_the_shipyards_panel,
            MAP,
            map_of(Screen(S::ShipDescription)),
        ),
        (
            "shipyard's info",
            in_the_shipyards_panel,
            PILOT,
            PLAYER_INFO,
        ),
        (
            "shipyard's info",
            in_the_shipyards_panel,
            MISSIONS,
            MISSION_INFO,
        ),
        ("bar", in_the_bar, MAP, map_of(Bar)),
        ("bar", in_the_bar, PILOT, PLAYER_INFO),
        ("bar", in_the_bar, MISSIONS, MISSION_INFO),
        (
            "hire's info",
            in_the_hire_panel,
            MAP,
            map_of(Screen(S::ShipDescription)),
        ),
        ("hire's info", in_the_hire_panel, PILOT, PLAYER_INFO),
        ("hire's info", in_the_hire_panel, MISSIONS, MISSION_INFO),
        (
            "mission computer",
            computer,
            MAP,
            map_of(Dialog(MapOpener::Computer(Some(MissionId(130))))),
        ),
        ("mission computer", computer, PILOT, PLAYER_INFO),
        ("mission computer", computer, MISSIONS, MISSION_INFO),
        (
            "mission info",
            mission_info,
            MAP,
            map_of(Dialog(MapOpener::MissionInfo(Some(0)))),
        ),
        (
            "briefing",
            briefing,
            MAP,
            map_of(Dialog(MapOpener::Briefing(marks()))),
        ),
        ("spaceport's offer", spaceport_offer, MAP, map_of(offer())),
        ("spaceport's offer", spaceport_offer, PILOT, PLAYER_INFO),
        ("spaceport's offer", spaceport_offer, MISSIONS, MISSION_INFO),
        ("bar's offer", bar_offer, MAP, map_of(offer())),
        ("bar's offer", bar_offer, PILOT, PLAYER_INFO),
        ("bar's offer", bar_offer, MISSIONS, MISSION_INFO),
        ("trade center's offer", trade_offer, MAP, map_of(offer())),
        ("trade center's offer", trade_offer, PILOT, PLAYER_INFO),
        ("trade center's offer", trade_offer, MISSIONS, MISSION_INFO),
        ("outfitter's offer", outfitter_offer, MAP, map_of(offer())),
        ("outfitter's offer", outfitter_offer, PILOT, PLAYER_INFO),
        ("outfitter's offer", outfitter_offer, MISSIONS, MISSION_INFO),
        ("shipyard's offer", shipyard_offer, MAP, map_of(offer())),
        ("shipyard's offer", shipyard_offer, PILOT, PLAYER_INFO),
        ("shipyard's offer", shipyard_offer, MISSIONS, MISSION_INFO),
    ]
}

#[test]
fn by_touch_alone_each_play_key_opens_what_its_key_opens_on_each_screen() {
    for (name, build, command, opens) in openings() {
        assert_eq!(keyed(build, command), opens, "{name} {command:?}: the key");
        assert_eq!(tapped(build, command), opens, "{name} {command:?}: a tap");
    }
}

#[test]
fn a_play_key_asks_for_nothing_where_its_key_asks_for_nothing() {
    let nothing: Asked = (None, false, false);
    let mut quiet = 0;
    for (name, build) in screens() {
        for command in PLAY_KEYS {
            let by_key = keyed(build, command);
            assert_eq!(tapped(build, command), by_key, "{name} {command:?}");
            if by_key == nothing {
                quiet += 1;
            }
        }
    }
    let listed = openings().len();
    assert_eq!(
        quiet + listed,
        screens().len() * PLAY_KEYS.len(),
        "every opening listed"
    );
}

#[test]
fn the_play_keys_hide_while_a_mission_text_is_in_front() {
    let mut view = port();
    view.input(&Input::Surroundings(Surroundings::of(Device::Touch)));
    let mut strip = DrawList::new();
    crate::ui::touch_aids::host::draw(&view, &mut strip);
    assert!(!strip.is_empty(), "shown");
    view.show_mission_texts(vec![MissionText {
        kind: MissionTextKind::Brief,
        mission: nova_sim::MissionId(130),
        text: "Hello.".to_owned(),
        graphic: None,
        movie: None,
        map: None,
    }]);
    assert!(view.mission_text().is_some(), "a text in front");
    let drawn = drawn(&view);
    assert!(
        strip.iter().all(|command| !drawn.contains(command)),
        "not drawn over the text"
    );
}

/// The labels of the play keys' strip as `view` draws it now.
fn strip_labels(view: &SpaceportView) -> Vec<String> {
    let mut strip = DrawList::new();
    crate::ui::touch_aids::host::draw(view, &mut strip);
    strip
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn over_a_briefing_with_a_map_only_the_map_control_shows_at_once() {
    let mut view = computer();
    view.input(&Input::Surroundings(Surroundings::of(Device::Touch)));
    assert_eq!(strip_labels(&view), ["Map", "Pilot", "Missions"]);
    view.show_mission_texts(vec![MissionText {
        kind: MissionTextKind::Brief,
        mission: nova_sim::MissionId(130),
        text: "Go.".to_owned(),
        graphic: None,
        movie: None,
        map: Some(marks()),
    }]);
    assert_eq!(strip_labels(&view), ["Map"], "drawn before any input");
    assert!(view.play_keys().offers(MAP));
    assert!(!view.play_keys().offers(PILOT));
    assert!(!view.play_keys().offers(MISSIONS));
    view.input(&key(Key::Enter));
    assert!(view.mission_text().is_none(), "the briefing closed");
    assert_eq!(
        strip_labels(&view),
        ["Map", "Pilot", "Missions"],
        "all back"
    );
}

#[test]
fn over_a_briefing_a_tap_where_pilot_or_missions_would_be_reaches_the_text() {
    for command in [PILOT, MISSIONS] {
        let mut view = briefing();
        view.input(&Input::Surroundings(Surroundings::of(Device::Touch)));
        let at = view.play_keys().rect(command).expect("its place").center();
        let mut fingers = Fingers::on(&mut view);
        fingers.down(1, at);
        fingers.up(1, at);
        assert_eq!(asked(&mut view), (None, false, false), "{command:?}");
        assert!(view.mission_text().is_some(), "{command:?}: still in front");
    }
}

#[test]
fn the_play_keys_hide_while_a_mission_movie_plays() {
    use crate::movie::fixture::MockMoviePlayer;
    use nova_sim::{DescMovie, MissionShown};
    let player = MockMoviePlayer::playing((160, 120), 1);
    let mut view = port().with_movies(movies_of(&player));
    view.input(&Input::Surroundings(Surroundings::of(Device::Touch)));
    let mut strip = DrawList::new();
    crate::ui::touch_aids::host::draw(&view, &mut strip);
    assert!(!strip.is_empty(), "shown");
    let movie = DescMovie::from_desc("Accepted.mov", 0).expect("named");
    view.show_mission_texts([MissionShown::Movie(movie)]);
    assert!(view.playing(), "a movie in front");
    assert!(view.mission_text().is_none(), "and no text");
    let drawn = drawn(&view);
    assert!(
        strip.iter().all(|command| !drawn.contains(command)),
        "not drawn over the movie"
    );
}

#[test]
fn the_play_keys_are_drawn_only_while_touch_was_used_last() {
    for device in crate::ui::touch_aids::testkit::OTHER_DEVICES {
        let mut view = port();
        view.input(&Input::Surroundings(Surroundings::of(Device::Touch)));
        view.input(&Input::Surroundings(Surroundings::of(device)));
        let mut bare = port();
        bare.input(&Input::Surroundings(Surroundings::of(device)));
        assert_eq!(drawn(&view), drawn(&bare), "{device:?}");
    }
}

#[test]
fn a_click_where_a_play_key_would_be_does_what_it_did() {
    let mut touched = port();
    touched.input(&Input::Surroundings(Surroundings::of(Device::Touch)));
    let at = touched.play_keys().rects()[0].center();
    let mut never = port();
    for view in [&mut touched, &mut never] {
        view.input(&Input::Surroundings(Surroundings::of(Device::Mouse)));
        click(view, at);
    }
    assert_eq!(asked(&mut touched), asked(&mut never));
    assert_eq!(drawn(&touched), drawn(&never));
}
