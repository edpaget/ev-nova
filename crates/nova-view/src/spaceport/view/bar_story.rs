//! The spaceport's side of the story the bar hosts under
//! [`RuleKey::BarStory`](nova_sim_frame::RuleKey::BarStory)'s other reading:
//! the story given to the bar open ([`SpaceportView::set_bar_story`]), and
//! the bar's asks taken through the spaceport, a slot's scene
//! ([`SpaceportView::take_bar_scene_request`]) and Holovid's story news
//! ([`SpaceportView::take_bar_story_news_request`]). What the bar does with
//! them is the bar's (the [`story`](crate::spaceport::bar::story) module);
//! whoever holds the session opens the scene or the news over the
//! spaceport.

use nova_sim_frame::flight_frame::BarStory;

use super::{Open, SpaceportView};

impl SpaceportView {
    /// Gives the bar open the story it hosts, or none
    /// ([`BarScreen::set_story`](crate::spaceport::BarScreen::set_story)).
    /// Nothing when the bar isn't open: it is given as the bar opens.
    pub fn set_bar_story(&mut self, story: Option<BarStory>) {
        if let Some(Open::Bar(screen)) = &mut self.open {
            screen.set_story(story);
        }
    }

    /// The story the bar open hosts, if it was given one.
    #[must_use]
    pub fn bar_story(&self) -> Option<&BarStory> {
        match &self.open {
            Some(Open::Bar(screen)) => screen.story(),
            _ => None,
        }
    }

    /// The scene a slot of the bar open asked for since this was last
    /// asked, by its index into the story's scenes, once.
    pub fn take_bar_scene_request(&mut self) -> Option<usize> {
        match &mut self.open {
            Some(Open::Bar(screen)) => screen.take_scene_request(),
            _ => None,
        }
    }

    /// Whether the bar open's Holovid asked for the story's news since
    /// this was last asked, once.
    pub fn take_bar_story_news_request(&mut self) -> bool {
        match &mut self.open {
            Some(Open::Bar(screen)) => screen.take_story_news_request(),
            _ => false,
        }
    }
}
