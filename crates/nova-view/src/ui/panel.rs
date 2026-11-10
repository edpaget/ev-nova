//! The new game's panel style ([`PanelBacking`]): a fill with a one-pixel
//! frame just inside it, drawn behind a panel whose source has no art of
//! its own.
//!
//! The story log, the conversation panel and every dialog the new game
//! lays out ([`DialogTemplate::backing`](super::dialog::DialogTemplate::backing))
//! draw it, so the new game's panels read as one family.
//!
//! A backed dialog's backing replaces whatever its host would draw under
//! an unbacked one: [`backdrop`] for the hosts drawing a picture or the
//! dark outlined backdrop, [`filled_backdrop`] for those filling a colour
//! of their own.

use crate::color::Color;
use crate::draw::{DrawList, fill_rect, frame_rect};
use crate::geometry::Bounds;
use crate::image::ImageKey;

use super::dialog::{Dialog, outline};
use super::prefs::{BACKDROP, BORDER};

/// An opaque backing: `fill` over the whole area, then a one-pixel `frame`
/// just inside its edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelBacking {
    /// The colour filling the area.
    pub fill: Color,
    /// The colour of the one-pixel frame just inside it.
    pub frame: Color,
}

impl PanelBacking {
    /// The new game's panels: black, framed dim.
    pub const NEW_GAME: Self = Self {
        fill: Color::BLACK,
        frame: Color::DIM,
    };

    /// Fills `bounds` with the fill colour, then frames it.
    pub fn draw(self, list: &mut DrawList, bounds: Bounds) {
        fill_rect(list, bounds, self.fill);
        frame_rect(list, bounds, self.frame);
    }
}

/// Draws what goes under `dialog`'s items: its backing, standing in for
/// the art a backed template's source lacks; else `picture` stretched
/// over its window; else a dark backdrop with an outline. A backed
/// dialog's backing replaces the host's own backdrop. The caller draws
/// the items after it with [`Dialog::draw_items`].
pub(crate) fn backdrop(list: &mut DrawList, dialog: &Dialog, picture: Option<ImageKey>) {
    let bounds = dialog.bounds();
    match (dialog.backing(), picture) {
        (Some(_), _) => dialog.draw_backing(list),
        (None, Some(image)) => {
            list.stretched_picture(image, bounds.min, bounds.width(), bounds.height());
        }
        (None, None) => {
            fill_rect(list, bounds, BACKDROP);
            outline(list, bounds, BORDER);
        }
    }
}

/// Draws `backing` over `bounds`, else, for a host whose own backdrop is
/// a plain fill, `fill` over them: the rule of [`backdrop`] for a host
/// that lays out its window's `backing` and `bounds` itself, or fills
/// under an unbacked dialog in a colour of its own.
pub(crate) fn filled_backdrop(
    list: &mut DrawList,
    backing: Option<PanelBacking>,
    bounds: Bounds,
    fill: Color,
) {
    match backing {
        Some(backing) => backing.draw(list, bounds),
        None => fill_rect(list, bounds, fill),
    }
}

/// The check every host of a backed dialog runs on what it draws.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use crate::draw::{DrawCommand, fill_command};

    /// Whether `command` is a fill of any colour covering `bounds`.
    fn fills(command: &DrawCommand, bounds: Bounds) -> bool {
        match command {
            DrawCommand::Line { color, .. } => *command == fill_command(bounds, *color),
            _ => false,
        }
    }

    /// Asserts that `drawn` begins with [`PanelBacking::NEW_GAME`] over
    /// `bounds`, before anything the host draws, and holds no other fill
    /// covering `bounds`, such as a host's own backdrop.
    #[track_caller]
    pub(crate) fn assert_backed_first(drawn: &[DrawCommand], bounds: Bounds) {
        assert_eq!(backed_at(drawn, bounds), 0, "the backing comes first");
    }

    /// Where in `drawn` [`PanelBacking::NEW_GAME`] over `bounds` begins,
    /// asserting that it is there and that no other fill covers `bounds`.
    /// What follows it is its host's, drawn over the backing.
    #[track_caller]
    pub(crate) fn backed_at(drawn: &[DrawCommand], bounds: Bounds) -> usize {
        let mut backing = DrawList::new();
        PanelBacking::NEW_GAME.draw(&mut backing, bounds);
        let backing: Vec<_> = backing.iter().cloned().collect();
        let at = drawn
            .windows(backing.len())
            .position(|window| window == backing.as_slice());
        let Some(at) = at else {
            panic!("no backing over {bounds:?} in {drawn:?}");
        };
        assert_eq!(
            drawn
                .iter()
                .filter(|command| fills(command, bounds))
                .count(),
            1,
            "one fill covers the window"
        );
        at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn the_new_games_backing_fills_black_then_frames_dim() {
        let bounds = Bounds::at(Point::new(12.0, 30.0), 200.0, 90.0);
        let mut list = DrawList::new();
        PanelBacking::NEW_GAME.draw(&mut list, bounds);
        let mut expected = DrawList::new();
        fill_rect(&mut expected, bounds, Color::BLACK);
        frame_rect(&mut expected, bounds, Color::DIM);
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            expected.iter().cloned().collect::<Vec<_>>()
        );
    }
}
