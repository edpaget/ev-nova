//! The story the bar hosts under
//! [`RuleKey::BarStory`](nova_sim::RuleKey::BarStory)'s other reading,
//! the new game's: the scenes it lists, each opened from a button, and the
//! news heard in the system, which its Holovid lists in place of the drawn
//! news. The listing is data the bar is given
//! ([`BarScreen::set_story`], a [`BarStory`] the session answers), never a
//! question the bar asks.
//!
//! - **The slots.** Items [`STORY_ITEMS`] (9 to 12) of the bar's template
//!   show one button each for the first four scenes: an offer's labelled
//!   with its title and its giver's name ([`offer_label`]), the stellar's
//!   own with [`OWN_CONVERSATION_LABEL`]. A
//!   slot with no scene is hidden: undrawn and greyed, so it draws
//!   nothing, takes no click and no focus, and the pad passes over it. The
//!   bar doesn't scroll: past four, the rest wait until fewer hold.
//! - **Choosing.** A slot clicked, or pressed with the keyboard's focus or
//!   the pad, asks for its scene by its index into the listing
//!   ([`BarScreen::take_scene_request`]); whoever holds the session opens
//!   it over the spaceport.
//! - **Holovid** with a story asks for the story's news
//!   ([`BarScreen::take_story_news_request`]), not the drawn news, and
//!   whoever holds the session shows it in the story log. Without a story
//!   it asks for the drawn news, as the original's does.
//!
//! Without a story (the engine's reading), and over a template whose items
//! 9 to 12 aren't shown user items inside it (the stock bar parks its 9
//! and 10 outside and has no 11 or 12), the slots show nothing; a missing
//! item changes nothing and never panics.

use nova_data::MissionGiver;
use nova_sim::flight_frame::{BarScene, BarStory};

use super::BarScreen;
use crate::ui::dialog::{Dialog, DialogTemplate, ItemSpec, Role};

/// The bar's story slots, items 9 to 12, top to bottom: one for each of
/// the first four scenes listed.
pub const STORY_ITEMS: [usize; 4] = [9, 10, 11, 12];

/// The label of the stellar's own conversation's slot.
pub const OWN_CONVERSATION_LABEL: &str = "Look around";

/// An offer's slot label: its `title`, then the name of its `giver` in
/// brackets when it has one ("Hull plate for Verge Camp (Ode)"), else the
/// title alone. The giver's role is left to the story log's entry, which
/// wraps: the slot is one centred line of fixed width.
#[must_use]
pub fn offer_label(title: &str, giver: Option<&MissionGiver>) -> String {
    match giver {
        Some(giver) => format!("{title} ({})", giver.name),
        None => title.to_owned(),
    }
}

/// `scene`'s slot label.
fn label(scene: &BarScene) -> String {
    match scene {
        BarScene::Offer { title, giver, .. } => offer_label(title, giver.as_ref()),
        BarScene::Stellar => OWN_CONVERSATION_LABEL.to_owned(),
    }
}

/// Enables `template`'s story slots that are user items, and gives them
/// button roles, appended to `roles`, for [`show_slots`] to label or hide.
pub(super) fn slot_roles(template: &mut DialogTemplate, roles: &mut Vec<(usize, Role)>) {
    for item in STORY_ITEMS {
        if let Some(found) = template.items.get_mut(item - 1)
            && found.kind == ItemSpec::User
        {
            found.enabled = true;
            roles.push((item, Role::Button(String::new())));
        }
    }
}

/// Shows `story`'s first four scenes in `dialog`'s slots and hides the
/// rest (see the module docs).
pub(super) fn show_slots(dialog: &mut Dialog, story: Option<&BarStory>) {
    let scenes = story.map_or(&[][..], |story| &story.scenes[..]);
    for (index, item) in STORY_ITEMS.into_iter().enumerate() {
        let scene = scenes.get(index);
        if let Some(scene) = scene {
            dialog.set_label(item, &label(scene));
        }
        dialog.set_drawn(item, scene.is_some());
        dialog.set_greyed(item, scene.is_none());
    }
}

impl BarScreen {
    /// Gives the bar the story it hosts, or none (see the module docs):
    /// its slots show `story`'s scenes, and Holovid asks for its news.
    pub fn set_story(&mut self, story: Option<BarStory>) {
        self.story = story;
        if let Ok(laid) = &mut self.laid {
            show_slots(&mut laid.dialog, self.story.as_ref());
        }
    }

    /// The story the bar hosts, if it was given one.
    #[must_use]
    pub fn story(&self) -> Option<&BarStory> {
        self.story.as_ref()
    }

    /// The scene a slot has asked for since this was last asked, by its
    /// index into the story's scenes, once.
    pub fn take_scene_request(&mut self) -> Option<usize> {
        self.asks.scene.take()
    }

    /// Whether Holovid has asked for the story's news since this was last
    /// asked: only while the bar hosts a story.
    pub fn take_story_news_request(&mut self) -> bool {
        std::mem::take(&mut self.asks.story_news)
    }

    /// Holovid pressed: asks for the story's news while the bar hosts a
    /// story, else for the visit's drawn news.
    pub(super) fn ask_news(&mut self) {
        if self.story.is_some() {
            self.asks.story_news = true;
        } else {
            self.asks.news = true;
        }
    }

    /// Activates `item` if it is a story slot: asks for its scene. Only a
    /// slot showing a scene can be activated, as [`show_slots`] greys and
    /// undraws the rest. Anything else does nothing.
    pub(super) fn activate_slot(&mut self, item: usize) {
        if let Some(index) = STORY_ITEMS.iter().position(|&slot| slot == item) {
            self.asks.scene = Some(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{bar_over, click_item, template, texts};
    use super::*;
    use crate::geometry::{Bounds, Point};
    use crate::input::{ControllerFamily, Device, Input, Key, PadButton, Surroundings};
    use crate::screen::Screen;
    use crate::spaceport::bar::{HOLOVID_ITEM, LEAVE_ITEM};
    use crate::ui::dialog::{DialogTemplate, ItemTemplate, Placement};

    fn user(x: f32, y: f32, w: f32, h: f32) -> ItemTemplate {
        ItemTemplate {
            bounds: Bounds::at(Point::new(x, y), w, h),
            enabled: false,
            kind: ItemSpec::User,
        }
    }

    /// The new game's placeholder bar's shape: 263 x 289, centred, the
    /// text (7) at the top, the four story slots (9-12) under it, the
    /// Gamble and Hire Escort row and the Holovid and Leave row below
    /// them, and 4, 6 and 8 parked outside.
    fn slotted() -> DialogTemplate {
        let parked = || user(0.0, 400.0, 10.0, 10.0);
        DialogTemplate {
            bounds: Bounds::at(Point::new(40.0, 40.0), 263.0, 289.0),
            placement: Placement::Center,
            items: vec![
                user(156.0, 258.0, 99.0, 26.0),
                user(156.0, 229.0, 99.0, 26.0),
                user(6.0, 258.0, 146.0, 26.0),
                parked(),
                user(6.0, 229.0, 146.0, 26.0),
                parked(),
                user(16.0, 10.0, 230.0, 106.0),
                parked(),
                user(16.0, 122.0, 230.0, 22.0),
                user(16.0, 148.0, 230.0, 22.0),
                user(16.0, 174.0, 230.0, 22.0),
                user(16.0, 200.0, 230.0, 22.0),
            ],
            backing: None,
        }
    }

    fn offer(title: &str) -> BarScene {
        BarScene::Offer {
            mission: title.to_lowercase(),
            title: title.to_owned(),
            giver: None,
        }
    }

    fn story(scenes: Vec<BarScene>) -> BarStory {
        BarStory {
            scenes,
            news: Vec::new(),
        }
    }

    fn words(screen: &BarScreen) -> Vec<String> {
        texts(screen).into_iter().map(|(text, _)| text).collect()
    }

    /// Each slot inside the dialog, by item, and whether it is a live
    /// button: drawn, enabled, and so a target of the pad's focus.
    fn slots(screen: &BarScreen) -> Vec<(usize, bool)> {
        let dialog = screen.dialog().expect("laid out");
        let targets = dialog.pad_targets();
        STORY_ITEMS
            .into_iter()
            .filter(|&item| dialog.item_shown(item))
            .map(|item| (item, targets.iter().any(|target| target.id == item)))
            .collect()
    }

    #[test]
    fn the_named_values() {
        assert_eq!(STORY_ITEMS, [9, 10, 11, 12]);
        assert_eq!(OWN_CONVERSATION_LABEL, "Look around");
    }

    #[test]
    fn without_a_story_the_slots_are_hidden() {
        let screen = bar_over(slotted());
        assert_eq!(
            slots(&screen),
            [(9, false), (10, false), (11, false), (12, false)]
        );
        assert_eq!(screen.story(), None);
    }

    #[test]
    fn the_slots_are_labelled_by_the_story_and_the_unused_hidden() {
        let mut screen = bar_over(slotted());
        screen.set_story(Some(story(vec![offer("Hull plate"), BarScene::Stellar])));
        let words = words(&screen);
        assert!(words.contains(&"Hull plate".to_owned()), "{words:?}");
        assert!(words.contains(&"Look around".to_owned()), "{words:?}");
        assert_eq!(
            slots(&screen),
            [(9, true), (10, true), (11, false), (12, false)]
        );
    }

    #[test]
    fn an_offer_with_a_giver_is_labelled_its_title_then_the_giver_s_name() {
        let ode = MissionGiver {
            name: "Ode".to_owned(),
            role: "dockmaster".to_owned(),
        };
        assert_eq!(offer_label("Hull plate", Some(&ode)), "Hull plate (Ode)");
        assert_eq!(offer_label("Hull plate", None), "Hull plate");
        let mut screen = bar_over(slotted());
        let given = BarScene::Offer {
            mission: "hull plate".to_owned(),
            title: "Hull plate".to_owned(),
            giver: Some(ode),
        };
        screen.set_story(Some(story(vec![given, offer("Toll")])));
        let words = words(&screen);
        assert!(words.contains(&"Hull plate (Ode)".to_owned()), "{words:?}");
        assert!(words.contains(&"Toll".to_owned()), "no giver: {words:?}");
        assert!(!words.contains(&"Hull plate".to_owned()), "{words:?}");
    }

    #[test]
    fn past_four_scenes_the_rest_wait() {
        let mut screen = bar_over(slotted());
        let scenes: Vec<BarScene> = ["A", "B", "C", "D", "E"].map(offer).into();
        screen.set_story(Some(story(scenes)));
        let words = words(&screen);
        assert!(words.contains(&"D".to_owned()), "{words:?}");
        assert!(!words.contains(&"E".to_owned()), "{words:?}");
    }

    #[test]
    fn a_story_taken_away_hides_the_slots_again() {
        let mut screen = bar_over(slotted());
        screen.set_story(Some(story(vec![offer("Hull plate")])));
        screen.set_story(Some(story(Vec::new())));
        assert!(!words(&screen).contains(&"Hull plate".to_owned()));
        assert_eq!(
            slots(&screen),
            [(9, false), (10, false), (11, false), (12, false)]
        );
        screen.set_story(Some(story(vec![offer("Hull plate")])));
        screen.set_story(None);
        assert!(!words(&screen).contains(&"Hull plate".to_owned()));
        assert_eq!(screen.story(), None);
    }

    #[test]
    fn a_slot_clicked_asks_for_its_scene_once() {
        let mut screen = bar_over(slotted());
        screen.set_story(Some(story(vec![offer("Hull plate"), BarScene::Stellar])));
        click_item(&mut screen, 10);
        assert_eq!(screen.take_scene_request(), Some(1));
        assert_eq!(screen.take_scene_request(), None, "once");
        click_item(&mut screen, 9);
        assert_eq!(screen.take_scene_request(), Some(0));
        assert!(!screen.closed());
    }

    #[test]
    fn a_hidden_slot_clicked_asks_for_nothing() {
        let mut screen = bar_over(slotted());
        screen.set_story(Some(story(vec![offer("Hull plate")])));
        click_item(&mut screen, 11);
        assert_eq!(screen.take_scene_request(), None);
        let mut screen = bar_over(slotted());
        click_item(&mut screen, 9);
        assert_eq!(screen.take_scene_request(), None, "no story");
    }

    #[test]
    fn holovid_with_a_story_asks_for_the_story_s_news_not_the_drawn_news() {
        let mut screen = bar_over(slotted());
        screen.set_story(Some(story(Vec::new())));
        click_item(&mut screen, HOLOVID_ITEM);
        assert!(screen.take_story_news_request());
        assert!(!screen.take_story_news_request(), "once");
        assert!(!screen.take_news_request());
        for pressed in [true, false] {
            screen.input(&Input::Key {
                key: Key::KeyN,
                pressed,
                repeat: false,
            });
        }
        assert!(screen.take_story_news_request(), "its key too");
        assert!(!screen.take_news_request());
    }

    #[test]
    fn holovid_without_a_story_asks_for_the_drawn_news_as_before() {
        let mut screen = bar_over(slotted());
        click_item(&mut screen, HOLOVID_ITEM);
        assert!(screen.take_news_request());
        assert!(!screen.take_story_news_request());
    }

    #[test]
    fn over_the_stock_template_a_story_shows_no_slot_and_nothing_breaks() {
        let mut screen = bar_over(template());
        let before = texts(&screen);
        screen.set_story(Some(story(vec![offer("Hull plate"), BarScene::Stellar])));
        assert_eq!(texts(&screen), before, "nothing new drawn");
        assert_eq!(slots(&screen), [], "9 and 10 parked outside");
        let mut bare = template();
        bare.items.truncate(7);
        let mut screen = bar_over(bare);
        screen.set_story(Some(story(vec![offer("Hull plate")])));
        click_item(&mut screen, HOLOVID_ITEM);
        assert!(screen.take_story_news_request(), "Holovid still asks");
    }

    #[test]
    fn the_pad_reaches_a_shown_slot_and_south_asks_for_its_scene() {
        let mut screen = bar_over(slotted());
        screen.set_story(Some(story(vec![offer("Hull plate")])));
        screen.input(&Input::Surroundings(Surroundings::of(Device::Gamepad(
            ControllerFamily::Generic,
        ))));
        let tap = |screen: &mut BarScreen, button| {
            for pressed in [true, false] {
                screen.input(&Input::PadButton {
                    button,
                    pressed,
                    repeat: false,
                });
            }
        };
        let at = |screen: &BarScreen| screen.dialog().and_then(Dialog::pad_highlight);
        assert_eq!(at(&screen), Some(LEAVE_ITEM));
        for _ in 0..8 {
            if at(&screen) == Some(9) {
                break;
            }
            tap(&mut screen, PadButton::RightShoulder);
        }
        assert_eq!(at(&screen), Some(9));
        tap(&mut screen, PadButton::South);
        assert_eq!(screen.take_scene_request(), Some(0));
    }
}
