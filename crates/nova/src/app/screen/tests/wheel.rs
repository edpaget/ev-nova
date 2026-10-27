//! The `dialog_text_wheel` rule: by the engine a wheel or trackpad
//! scroll reaches no screen, as the original takes no wheel; by the other
//! reading it scrolls the dialog text under the pointer, here a mission
//! offer's over widened flight, moved back into the dialog's classic frame.

use super::*;

use nova_sim::Rulebook;
use nova_view::ScrollDelta;
use nova_view::spaceport::offer::TEXT_ITEM;
use nova_view::ui::ScrollText;

/// A 16:9 window's widened size, and how far right a 1024-wide frame sits
/// in it.
const WIDE: (f32, f32) = (1365.0, 768.0);
const OFFSET_X: f32 = 170.0;

/// The router under `rules`, in widened flight, offering a mission whose
/// text overflows its box.
fn offering(rules: Rulebook) -> AppScreen {
    let mut screen = AppScreen::new(data())
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_rules(rules);
    screen.set_widened_size(WIDE);
    fly(&mut screen);
    let words: Vec<String> = (0..200).map(|n| format!("word{n}")).collect();
    screen.open_offer(OfferPrompt {
        mission: MissionId(130),
        text: words.join(" "),
        labels: nova_sim::mission::offer_labels("", "", true),
        graphic: None,
        movie: None,
    });
    assert_eq!(screen.showing(), Showing::Offer);
    assert!(text(&screen).overflows());
    screen
}

fn text(screen: &AppScreen) -> &ScrollText {
    screen
        .offer()
        .and_then(|offer| offer.dialog())
        .and_then(|dialog| dialog.text(TEXT_ITEM))
        .expect("the offer's text")
}

/// A notch over the text's right edge, as the widened window gives it:
/// outside the text unless moved back into the classic frame.
fn wheel_over_text(screen: &mut AppScreen) {
    let rect = text(screen).rect();
    screen.input(&Input::Scroll {
        delta: ScrollDelta::Lines(1.0),
        at: Point::new(rect.max.x - 2.0 + OFFSET_X, rect.center().y),
    });
}

#[test]
fn by_the_engine_the_wheel_scrolls_no_dialog_text_and_the_arrows_still_do() {
    let mut screen = offering(Rulebook::default());
    wheel_over_text(&mut screen);
    assert_eq!(text(&screen).first(), 0);
    screen.input(&key(Key::Down, true));
    assert_eq!(text(&screen).first(), 1, "Down still scrolls");
}

#[test]
fn by_the_other_reading_the_wheel_scrolls_the_dialog_text_under_it() {
    let mut screen =
        offering(Rulebook::default().with_override(RuleKey::DialogTextWheel, RuleSource::Bible));
    wheel_over_text(&mut screen);
    assert_eq!(text(&screen).first(), 1);
    screen.input(&key(Key::Down, true));
    assert_eq!(text(&screen).first(), 2, "Down still scrolls");
}
