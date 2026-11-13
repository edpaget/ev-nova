//! The bar's story through the spaceport: given to the bar open, and its
//! asks taken through the spaceport. What the bar does with it is the bar's
//! (`spaceport/bar/story.rs`).

use super::*;
use nova_sim_frame::flight_frame::{BarScene, BarStory};

/// Earth's bar laid out with its four story slots, under the text.
fn storied() -> SpaceportView {
    let mut template = bar_template();
    template.items.truncate(8);
    for n in 0..4 {
        template.items.push(ItemTemplate {
            bounds: rect(16.0, 117.0 + 2.0 * n as f32, 230.0, 2.0),
            enabled: false,
            kind: ItemSpec::User,
        });
    }
    let art: Rc<dyn ShipyardCatalog> = Rc::new(catalog());
    earth().with_bar(
        plain_bar(Ok(template)),
        Ok(news_template()),
        Ok(shipyard_template()),
        no_info(),
        art,
        true,
    )
}

fn story() -> BarStory {
    BarStory {
        scenes: vec![BarScene::Stellar],
        news: Vec::new(),
    }
}

#[test]
fn the_story_goes_to_the_bar_open_and_nowhere_else() {
    let mut view = storied();
    view.set_bar_story(Some(story()));
    assert_eq!(view.bar_story(), None, "nothing open");
    click_item(&mut view, 11);
    assert_eq!(view.bar_story(), None, "not kept for a bar not open");
    view.set_bar_story(Some(story()));
    assert_eq!(view.bar_story(), Some(&story()));
    assert_eq!(view.open_bar().and_then(BarScreen::story), Some(&story()));
    view.set_bar_story(None);
    assert_eq!(view.bar_story(), None);
}

#[test]
fn a_slot_s_scene_is_asked_through_the_spaceport_once() {
    let mut view = storied();
    assert_eq!(view.take_bar_scene_request(), None, "nothing open");
    click_item(&mut view, 11);
    view.set_bar_story(Some(story()));
    let at = bar_item(&view, 9);
    click(&mut view, at);
    assert_eq!(view.take_bar_scene_request(), Some(0));
    assert_eq!(view.take_bar_scene_request(), None, "once");
}

#[test]
fn holovid_over_a_story_asks_for_its_news_through_the_spaceport_once() {
    let mut view = storied();
    assert!(!view.take_bar_story_news_request(), "nothing open");
    click_item(&mut view, 11);
    view.set_bar_story(Some(story()));
    let at = bar_item(&view, 3);
    click(&mut view, at);
    assert!(view.take_bar_story_news_request());
    assert!(!view.take_bar_story_news_request(), "once");
    assert!(!view.take_news_request(), "not the drawn news");
}
