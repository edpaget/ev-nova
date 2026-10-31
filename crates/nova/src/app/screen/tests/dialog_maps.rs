use super::*;
use nova_view::DrawLayer;

// A mission dialog's map over flight, through the router: the offer from
// a ship, here opened as its hail would.

/// [`with_dialogs`] in flight, a person's offer of mission 130 open
/// over it.
fn offered_in_flight() -> AppScreen {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    offer_job(&mut screen);
    screen
}

/// Opens a person's offer of mission 130 on `screen`.
fn offer_job(screen: &mut AppScreen) {
    screen.open_offer(nova_sim::OfferPrompt {
        mission: MissionId(130),
        text: "Job.".to_owned(),
        labels: nova_sim::mission::offer_labels("", "", true),
        graphic: None,
        movie: None,
    });
    assert_eq!(screen.showing(), Showing::Offer);
}

fn map_sounds(screen: &mut AppScreen) -> Vec<Sound> {
    screen
        .take_sounds()
        .into_iter()
        .filter(|sound| matches!(sound, Sound::Ui(UiSound::MapOpened | UiSound::MapClosed)))
        .collect()
}

#[test]
fn m_on_an_offer_over_flight_opens_its_map_and_m_closes_back_to_the_offer() {
    let mut screen = offered_in_flight();
    screen.take_sounds();
    screen.input(&key(MAP, true));
    assert_eq!(screen.showing(), Showing::FlightMap);
    assert!(flight(&screen).dialog_map_open());
    screen.input(&key(MAP, false));
    assert_eq!(map_sounds(&mut screen), [], "opened in silence");
    screen.input(&key(MAP, true));
    assert!(flight(&screen).map_open(), "waits for M's release");
    screen.input(&key(MAP, false));
    assert!(!flight(&screen).map_open(), "M closes it");
    assert_eq!(screen.showing(), Showing::Offer, "back to the offer");
    assert_eq!(map_sounds(&mut screen), [], "closed in silence");
}

#[test]
fn escape_closes_a_dialogs_map_over_flight_back_to_the_dialog() {
    let mut screen = offered_in_flight();
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    screen.input(&key(Key::Escape, true));
    assert!(!flight(&screen).map_open());
    assert_eq!(screen.showing(), Showing::Offer, "flight not left");
}

#[test]
fn a_dialogs_map_over_flight_takes_the_input_before_the_dialog() {
    // Return on the map closes it, and doesn't accept the offer.
    let mut screen = offered_in_flight();
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    assert!(!flight(&screen).map_open());
    assert_eq!(screen.showing(), Showing::Offer, "the offer unanswered");
}

/// Where `commands` hold the offer's text. The test's dialogs have no
/// `DLOG` 1016: the offer says so.
fn offer_text<'a>(mut commands: impl Iterator<Item = &'a DrawCommand>) -> Option<usize> {
    commands.position(|command| {
        matches!(command, DrawCommand::Text { text, .. } if text.starts_with("Cannot show the mission offer"))
    })
}

#[test]
fn a_dialogs_map_over_flight_draws_the_scene_then_the_dialog_then_the_map() {
    let mut screen = offered_in_flight();
    screen.input(&key(MAP, true));
    let mut under = DrawList::new();
    flight(&screen).draw_under_dialog_map(&mut under);
    let mut over = DrawList::new();
    flight(&screen).draw_dialog_map(&mut over);
    let scene: Vec<&DrawCommand> = under.iter().collect();
    let map: Vec<&DrawCommand> = over.iter().collect();
    assert!(!scene.is_empty());
    assert!(!map.is_empty());

    let mut drawn = DrawList::new();
    screen.draw(&mut drawn);
    let drawn: Vec<&DrawCommand> = drawn.iter().collect();
    assert!(drawn.starts_with(&scene), "the paused scene first");
    assert!(drawn.ends_with(&map), "the map last");
    let offer = offer_text(drawn.iter().copied()).expect("the offer's drawn");
    assert!(
        (scene.len()..drawn.len() - map.len()).contains(&offer),
        "the offer between them"
    );
}

#[test]
fn a_dialogs_map_over_flight_leaves_no_world_over_the_dialog() {
    let mut screen = offered_in_flight();
    screen.input(&key(MAP, true));
    let mut drawn = DrawList::new();
    screen.draw(&mut drawn);
    let offer = offer_text(drawn.iter()).expect("the offer's drawn");
    let world = drawn
        .layered()
        .enumerate()
        .filter(|(_, (layer, _))| *layer == DrawLayer::World)
        .last()
        .map(|(at, _)| at);
    assert!(
        world.is_some_and(|last| last < offer),
        "the last world command, {world:?}, under the offer at {offer}"
    );
}

#[test]
fn an_offer_over_flight_hides_flights_cursor_and_keeps_it_hidden_after_its_map() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.tick(TICK);
    screen.input(&Input::PointerMoved(Point::new(300.0, 300.0)));
    assert!(flight(&screen).cursor_at().is_some(), "over flight");
    offer_job(&mut screen);
    assert_eq!(flight(&screen).cursor_at(), None, "under the offer");
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert!(flight(&screen).map_open());
    screen.input(&key(Key::Escape, true));
    assert!(!flight(&screen).map_open());
    assert_eq!(screen.showing(), Showing::Offer);
    assert_eq!(flight(&screen).cursor_at(), None, "after its map");
}

#[test]
fn command_comma_on_a_dialogs_map_over_flight_opens_the_preferences_unless_find_is_open() {
    let mut screen = offered_in_flight();
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    command_comma(&mut screen);
    assert!(screen.preferences().is_some(), "as on flight's own map");
    let mut screen = offered_in_flight();
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    screen.input(&key(Key::Char('f'), true));
    screen.input(&key(Key::Char('f'), false));
    assert!(flight(&screen).map_finding());
    command_comma(&mut screen);
    assert!(screen.preferences().is_none(), "Find takes every key");
    assert!(flight(&screen).map_finding());
}

#[test]
fn losing_the_keys_completes_a_dialogs_map_close_over_flight() {
    // The map waits for M's release (`_DoSystemMap` @0x13a2e-0x13a46);
    // focus lost counts as the release, though the offer is in front of
    // flight among the overlays.
    let mut screen = offered_in_flight();
    screen.input(&key(MAP, true));
    screen.input(&key(Key::Escape, true));
    assert!(flight(&screen).map_open(), "waits for M's release");
    screen.release_keys();
    assert!(!flight(&screen).map_open(), "closed");
    assert_eq!(screen.showing(), Showing::Offer, "back to the offer");
}
