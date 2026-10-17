use super::*;

// Find and Space on the maps, through the router.

/// Types `text` as the platform sends it: each key, its character, its
/// release.
fn type_in(screen: &mut AppScreen, text: &str) {
    for c in text.chars() {
        let k = Key::Char(c.to_ascii_lowercase());
        screen.input(&key(k, true));
        screen.input(&Input::Text(c));
        screen.input(&key(k, false));
    }
}

/// [`with_dialogs`] on the galaxy viewer.
fn on_the_viewer() -> AppScreen {
    let mut screen = with_dialogs(data());
    screen.input(&key(Key::Tab, true));
    screen.input(&key(Key::Tab, false));
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    screen
}

fn finding_text(screen: &AppScreen) -> String {
    screen
        .galaxy_map()
        .find_dialog()
        .expect("Find open")
        .field()
        .text()
        .to_owned()
}

fn ui(sounds: Vec<Sound>) -> Vec<Sound> {
    sounds
        .into_iter()
        .filter(|sound| matches!(sound, Sound::Ui(_)))
        .collect()
}

#[test]
fn s_opens_find_on_the_viewer_and_it_takes_every_key() {
    let mut screen = on_the_viewer();
    type_in(&mut screen, "s");
    assert!(screen.galaxy_map().finding());
    type_in(&mut screen, "fpi");
    for k in [Key::Tab, Key::Escape] {
        assert_eq!(screen.input(&key(k, true)), ScreenAction::None, "{k:?}");
        screen.input(&key(k, false));
    }
    command_comma(&mut screen);
    assert_eq!(screen.showing(), Showing::GalaxyMap, "no side switched");
    assert!(screen.flight_view().is_none(), "F didn't fly");
    assert!(screen.about().is_none(), "I opened no About box");
    assert!(
        screen.preferences().is_none(),
        "nor P or Command+comma Prefs"
    );
    assert_eq!(finding_text(&screen), "fpi");
    for _ in 0..3 {
        screen.input(&key(Key::Backspace, true));
    }
    type_in(&mut screen, "beta");
    screen.input(&key(Key::Enter, true));
    assert!(!screen.galaxy_map().finding());
    assert_eq!(screen.galaxy_map().selected(), Some(SystemId(129)));
    assert_eq!(
        ui(screen.take_sounds()),
        [Sound::Ui(UiSound::MapSystemFound)]
    );
    type_in(&mut screen, "f");
    assert_eq!(screen.showing(), Showing::Flight, "F flies again");
}

#[test]
fn in_flight_find_keeps_escape_and_command_comma_from_the_map() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    type_in(&mut screen, "m");
    assert!(flight(&screen).map_open());
    type_in(&mut screen, "f");
    assert!(flight(&screen).map_finding());
    screen.take_sounds();
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert!(flight(&screen).map_open(), "Escape doesn't close the map");
    command_comma(&mut screen);
    assert!(screen.preferences().is_none(), "nor open the Preferences");
    assert!(flight(&screen).map_finding());
    type_in(&mut screen, "alpha");
    screen.input(&key(Key::Enter, true));
    assert_eq!(
        ui(screen.take_sounds()),
        [Sound::Ui(UiSound::MapSystemFound)]
    );
    assert!(!flight(&screen).map_finding());
    screen.input(&key(Key::Escape, true));
    assert!(!flight(&screen).map_open(), "Escape closes it again");
    assert_eq!(ui(screen.take_sounds()), [Sound::Ui(UiSound::MapClosed)]);
}

#[test]
fn space_on_the_flight_map_beeps_through_the_router() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    type_in(&mut screen, "m");
    screen.take_sounds();
    screen.input(&key(Key::Space, true));
    assert_eq!(ui(screen.take_sounds()), [Sound::Ui(UiSound::MapCentred)]);
    assert_eq!(flight(&screen).course_map().selected(), Some(SystemId(128)));
}

#[test]
fn the_flights_find_is_laid_out_in_the_datas_button_style() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    type_in(&mut screen, "m");
    type_in(&mut screen, "f");
    let dialog = flight(&screen).course_map().find_dialog().expect("open");
    let bounds = dialog.dialog().bounds();
    assert_eq!(
        (bounds.min, bounds.max),
        (Point::new(370.0, 331.0), Point::new(654.0, 436.0)),
        "the fallback, centred, as the dialogs have no DLOG 3005"
    );
    let mut list = DrawList::new();
    flight(&screen).course_map().draw(&mut list);
    let style = screen.data.button_style();
    assert!(list.iter().any(|command| matches!(
        command,
        DrawCommand::Text { text, font, .. } if text == "Find" && *font == style.font
    )));
}
