//! Draw commands and draw lists.

use crate::color::Color;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;

/// One thing to draw, in logical coordinates.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCommand {
    /// A sprite frame centred on `center`, as Nova places sprites. The tint
    /// multiplies the frame's colour, and its alpha is the sprite's alpha.
    Sprite {
        /// The frame.
        image: ImageKey,
        /// Where the frame's centre goes.
        center: Point,
        /// Colour multiplier and alpha.
        tint: Color,
    },
    /// A picture, unscaled and untinted, with its top-left corner at
    /// `top_left`.
    Picture {
        /// The picture.
        image: ImageKey,
        /// Where its top-left corner goes.
        top_left: Point,
    },
    /// A picture stretched (or shrunk) to fill a rectangle, untinted. A
    /// rectangle whose width or height is not a positive, finite number
    /// draws nothing.
    StretchedPicture {
        /// The picture.
        image: ImageKey,
        /// The rectangle's top-left corner.
        top_left: Point,
        /// The rectangle's width.
        width: f32,
        /// The rectangle's height.
        height: f32,
    },
    /// A run of text.
    Text {
        /// The text.
        text: String,
        /// The font it is drawn in.
        font: Font,
        /// The top-left corner of its first line.
        origin: Point,
        /// The font size.
        size: f32,
        /// Wraps lines longer than this, if set.
        wrap_width: Option<f32>,
        /// The text colour.
        color: Color,
    },
    /// A straight line.
    Line {
        /// One end.
        from: Point,
        /// The other end.
        to: Point,
        /// Its thickness.
        width: f32,
        /// Its colour.
        color: Color,
    },
    /// A square dot.
    Dot {
        /// Its centre.
        center: Point,
        /// Its side length.
        size: f32,
        /// Its colour.
        color: Color,
    },
}

/// A frame's draw commands, drawn in order (later ones on top).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawList {
    commands: Vec<DrawCommand>,
}

impl DrawList {
    /// An empty list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends `command`.
    pub fn push(&mut self, command: DrawCommand) -> &mut Self {
        self.commands.push(command);
        self
    }

    /// The commands, in draw order.
    pub fn iter(&self) -> std::slice::Iter<'_, DrawCommand> {
        self.commands.iter()
    }

    /// How many commands there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether there are no commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Appends a [`DrawCommand::Sprite`].
    pub fn sprite(&mut self, image: ImageKey, center: Point, tint: Color) -> &mut Self {
        self.push(DrawCommand::Sprite {
            image,
            center,
            tint,
        })
    }

    /// Appends a [`DrawCommand::Picture`].
    pub fn picture(&mut self, image: ImageKey, top_left: Point) -> &mut Self {
        self.push(DrawCommand::Picture { image, top_left })
    }

    /// Appends a [`DrawCommand::StretchedPicture`].
    pub fn stretched_picture(
        &mut self,
        image: ImageKey,
        top_left: Point,
        width: f32,
        height: f32,
    ) -> &mut Self {
        self.push(DrawCommand::StretchedPicture {
            image,
            top_left,
            width,
            height,
        })
    }

    /// Appends a [`DrawCommand::Text`] in [`Font::Geneva`].
    pub fn text(
        &mut self,
        text: impl Into<String>,
        origin: Point,
        size: f32,
        wrap_width: Option<f32>,
        color: Color,
    ) -> &mut Self {
        self.text_in(Font::Geneva, text, origin, size, wrap_width, color)
    }

    /// Appends a [`DrawCommand::Text`] in `font`.
    pub fn text_in(
        &mut self,
        font: Font,
        text: impl Into<String>,
        origin: Point,
        size: f32,
        wrap_width: Option<f32>,
        color: Color,
    ) -> &mut Self {
        self.push(DrawCommand::Text {
            text: text.into(),
            font,
            origin,
            size,
            wrap_width,
            color,
        })
    }

    /// Appends a [`DrawCommand::Line`].
    pub fn line(&mut self, from: Point, to: Point, width: f32, color: Color) -> &mut Self {
        self.push(DrawCommand::Line {
            from,
            to,
            width,
            color,
        })
    }

    /// Appends a [`DrawCommand::Dot`].
    pub fn dot(&mut self, center: Point, size: f32, color: Color) -> &mut Self {
        self.push(DrawCommand::Dot {
            center,
            size,
            color,
        })
    }
}

/// Draws a placeholder where an image cannot be shown: the outline of the
/// `size` x `size` square centred on `center`, with 2-unit edges, and its
/// two diagonals, 1 unit wide, all in `color`.
pub fn crossed_box(list: &mut DrawList, center: Point, size: f32, color: Color) {
    let half = size / 2.0;
    let (left, right) = (center.x - half, center.x + half);
    let (top, bottom) = (center.y - half, center.y + half);
    let corners = [
        Point::new(left, top),
        Point::new(right, top),
        Point::new(right, bottom),
        Point::new(left, bottom),
    ];
    for (at, &from) in corners.iter().enumerate() {
        list.line(from, corners[(at + 1) % 4], 2.0, color);
    }
    list.line(corners[0], corners[2], 1.0, color)
        .line(corners[1], corners[3], 1.0, color);
}

/// Fills `area` with `color`. There is no rectangle command: a horizontal
/// line as thick as the area, along its middle, is one.
pub fn fill_rect(list: &mut DrawList, area: Bounds, color: Color) {
    let middle = area.center().y;
    list.line(
        Point::new(area.min.x, middle),
        Point::new(area.max.x, middle),
        area.height(),
        color,
    );
}

impl<'a> IntoIterator for &'a DrawList {
    type Item = &'a DrawCommand;
    type IntoIter = std::slice::Iter<'a, DrawCommand>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn a_new_list_is_empty() {
        let list = DrawList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert_eq!(list.iter().count(), 0);
    }

    #[test]
    fn builders_append_commands_in_call_order() {
        let red = Color::rgba(255, 0, 0, 255);
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(0.0, 0.0))
            .sprite(ImageKey::sprite(200, 3), at(10.0, 20.0), Color::WHITE)
            .text("Hi", at(1.0, 2.0), 12.0, Some(100.0), red)
            .line(at(0.0, 0.0), at(4.0, 0.0), 2.0, red)
            .dot(at(5.0, 6.0), 3.0, Color::BLACK);
        list.push(DrawCommand::Dot {
            center: at(7.0, 8.0),
            size: 1.0,
            color: red,
        });
        assert!(!list.is_empty());
        assert_eq!(list.len(), 6);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        assert_eq!(
            *commands[0],
            DrawCommand::Picture {
                image: ImageKey::picture(128),
                top_left: at(0.0, 0.0)
            }
        );
        assert_eq!(
            *commands[1],
            DrawCommand::Sprite {
                image: ImageKey::sprite(200, 3),
                center: at(10.0, 20.0),
                tint: Color::WHITE
            }
        );
        assert_eq!(
            *commands[2],
            DrawCommand::Text {
                text: "Hi".to_owned(),
                font: Font::Geneva,
                origin: at(1.0, 2.0),
                size: 12.0,
                wrap_width: Some(100.0),
                color: red
            }
        );
        assert_eq!(
            *commands[3],
            DrawCommand::Line {
                from: at(0.0, 0.0),
                to: at(4.0, 0.0),
                width: 2.0,
                color: red
            }
        );
        assert_eq!(
            *commands[4],
            DrawCommand::Dot {
                center: at(5.0, 6.0),
                size: 3.0,
                color: Color::BLACK
            }
        );
        assert_eq!(
            *commands[5],
            DrawCommand::Dot {
                center: at(7.0, 8.0),
                size: 1.0,
                color: red
            }
        );
    }

    #[test]
    fn text_in_records_its_font() {
        let red = Color::rgba(255, 0, 0, 255);
        let mut list = DrawList::new();
        list.text_in(Font::Charcoal, "Kestrel", at(3.0, 4.0), 18.0, None, red)
            .text_in(
                Font::Geneva,
                "Cost",
                at(5.0, 6.0),
                9.0,
                Some(50.0),
                Color::WHITE,
            );
        let commands: Vec<&DrawCommand> = list.iter().collect();
        assert_eq!(
            commands,
            [
                &DrawCommand::Text {
                    text: "Kestrel".to_owned(),
                    font: Font::Charcoal,
                    origin: at(3.0, 4.0),
                    size: 18.0,
                    wrap_width: None,
                    color: red
                },
                &DrawCommand::Text {
                    text: "Cost".to_owned(),
                    font: Font::Geneva,
                    origin: at(5.0, 6.0),
                    size: 9.0,
                    wrap_width: Some(50.0),
                    color: Color::WHITE
                },
            ]
        );
    }

    #[test]
    fn a_stretched_picture_keeps_its_rectangle() {
        let mut list = DrawList::new();
        list.stretched_picture(ImageKey::picture(9502), at(-3.5, 20.0), 188.25, 223.5)
            .picture(ImageKey::picture(1), at(0.0, 0.0));
        let commands: Vec<&DrawCommand> = list.iter().collect();
        assert_eq!(
            *commands[0],
            DrawCommand::StretchedPicture {
                image: ImageKey::picture(9502),
                top_left: at(-3.5, 20.0),
                width: 188.25,
                height: 223.5
            }
        );
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn a_borrowed_list_iterates_in_draw_order() {
        let mut list = DrawList::new();
        list.dot(at(1.0, 0.0), 1.0, Color::WHITE)
            .dot(at(2.0, 0.0), 1.0, Color::WHITE);
        let mut xs = Vec::new();
        for command in &list {
            if let DrawCommand::Dot { center, .. } = command {
                xs.push(center.x);
            }
        }
        assert_eq!(xs, [1.0, 2.0]);
    }

    #[test]
    fn default_is_empty() {
        assert!(DrawList::default().is_empty());
    }

    #[test]
    fn a_filled_rectangle_is_a_line_through_its_middle_as_thick_as_it_is() {
        let grey = Color::rgba(16, 16, 28, 255);
        let mut list = DrawList::new();
        let area = Bounds {
            min: at(528.0, 712.0),
            max: at(708.0, 744.0),
        };
        fill_rect(&mut list, area, grey);
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [DrawCommand::Line {
                from: at(528.0, 728.0),
                to: at(708.0, 728.0),
                width: 32.0,
                color: grey,
            }]
        );
    }

    #[test]
    fn a_crossed_box_is_its_four_edges_then_its_diagonals() {
        let grey = Color::rgba(128, 128, 128, 255);
        let mut list = DrawList::new();
        crossed_box(&mut list, at(240.0, 320.0), 128.0, grey);
        let [top_left, top_right, bottom_right, bottom_left] = [
            at(176.0, 256.0),
            at(304.0, 256.0),
            at(304.0, 384.0),
            at(176.0, 384.0),
        ];
        let line = |from, to, width| DrawCommand::Line {
            from,
            to,
            width,
            color: grey,
        };
        let commands: Vec<DrawCommand> = list.iter().cloned().collect();
        assert_eq!(
            commands,
            [
                line(top_left, top_right, 2.0),
                line(top_right, bottom_right, 2.0),
                line(bottom_right, bottom_left, 2.0),
                line(bottom_left, top_left, 2.0),
                line(top_left, bottom_right, 1.0),
                line(top_right, bottom_left, 1.0),
            ]
        );
    }
}
