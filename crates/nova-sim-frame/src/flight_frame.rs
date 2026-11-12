//! The values the flight view's `FlightSim` port carries across the seam
//! between the view and the session, as far as they have moved down from
//! `nova_sim::flight_frame`, which re-exports each by name and holds the
//! rest: the new game's storyline screens ([`StoryRow`], [`BarStory`],
//! [`RoomRefusal`], the `story_screens` module).

mod story_screens;

pub use story_screens::{BarScene, BarStory, RoomRefusal, StoryObjective, StoryRow};
