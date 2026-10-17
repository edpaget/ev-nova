use super::*;

// A mission dialog's map over flight, through the router: the offer from
// a ship, here opened as its hail would.

/// [`with_dialogs`] in flight, a person's offer of mission 130 open
/// over it.
fn offered_in_flight() -> AppScreen {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.open_offer(nova_sim::OfferPrompt {
        mission: MissionId(130),
        text: "Job.".to_owned(),
        labels: nova_sim::mission::offer_labels("", "", true),
        graphic: None,
        movie: None,
    });
    assert_eq!(screen.showing(), Showing::Offer);
    screen
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

#[test]
fn a_dialogs_map_over_flight_is_drawn_over_the_dialog() {
    let mut screen = offered_in_flight();
    screen.input(&key(MAP, true));
    let mut drawn = DrawList::new();
    screen.draw(&mut drawn);
    let mut map = DrawList::new();
    flight(&screen).draw(&mut map);
    let drawn: Vec<&DrawCommand> = drawn.iter().collect();
    let map: Vec<&DrawCommand> = map.iter().collect();
    assert!(!map.is_empty());
    assert!(drawn.ends_with(&map), "the map last");
    // The test's dialogs have no `DLOG` 1016: the offer says so.
    let offer_text = drawn.iter().position(|command| {
        matches!(command, DrawCommand::Text { text, .. } if text.starts_with("Cannot show the mission offer"))
    });
    assert!(
        offer_text.is_some_and(|at| at < drawn.len() - map.len()),
        "the offer under it"
    );
}
