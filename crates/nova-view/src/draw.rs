//! Draw commands and draw lists.

use nova_sim::blink::FULL;

use crate::color::Color;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;

/// How a sprite combines with what is beneath it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Blend {
    /// Painted over what is beneath, by its alpha.
    #[default]
    Normal,
    /// Adds its colour, scaled by its alpha, to what is beneath. It stands
    /// in for the original's OR-based `AddOver` and translucent light and
    /// glow blits (see [`lights_tint`]).
    Additive,
    /// ORs its colour, scaled by a level from the tint, into what is
    /// beneath, bit for bit: the original's ship glow and lights blits,
    /// `_BlitPixieRLEAddOver` (0xc24bf; the copy loop at 0xc2560) at full
    /// level and `_BlitPixieRLETranslucent` (0xc1568) with
    /// `_BlitPixieTranslucentCopy` (0xc1110; per pixel at 0xc11cc–0xc1275)
    /// below it, which compute `((src_c × n) >> 5) | dst_c` per 5-bit
    /// channel. `_HandleShipDisplay` (0x2b514) sets n for the lights at
    /// 0x2c672–0x2c692 and for the glow at 0x2c2b6–0x2c2d6. See
    /// [`lights_tint`] for the full trace.
    ///
    /// # The level
    ///
    /// The tint is the per-channel level in 32nds:
    /// `n_c = round(tint_c × tint_a × 32 / 255²)`, so [`Color::WHITE`] is
    /// 32 (the layer as it is) and [`lights_tint`]`(n)` is exactly n in
    /// every channel, for every n from 0 to 32.
    ///
    /// # The bit depth
    ///
    /// Per channel, with the layer's texel `t` (straight RGBA, 0 to 1) and
    /// the destination `d8` (0 to 255):
    ///
    /// ```text
    /// s8  = round(t.rgb × t.a × 255)   // premultiplied: transparent adds 0
    /// s5  = s8 >> 3                    // the 5-bit source
    /// t5  = (s5 × n) >> 5              // the original's floor, exactly
    /// t8  = (t5 << 3) | (t5 >> 2)      // widened by bit replication
    /// out = d8 | t8 ;  out.a = d.a
    /// ```
    ///
    /// The source is scaled in 5 bits and the OR is taken at 8 bits
    /// against the destination as it is:
    ///
    /// - Stock layer texels are 5-bit values widened by replication, so
    ///   `s8 >> 3` recovers the original's 5-bit value and `(s5 × n) >> 5`
    ///   is its own truncation. Scaling in 8 bits would miss that floor by
    ///   up to one 5-bit step (blue 5 at level 10 is 1 in the original, 2
    ///   if rounded).
    /// - Replication distributes over OR: `w(a) | w(b) = w(a | b)`. Where
    ///   the destination is itself a widened 5-bit colour (every stock hull
    ///   pixel), the 8-bit OR is exactly the original's 5-bit OR, widened.
    ///   Where it is not (antialiased text, blended or tinted draws), its
    ///   low bits survive, and a black or transparent layer pixel leaves
    ///   any destination unchanged bit for bit. Quantising the destination
    ///   to 5 bits as well would posterise every such pixel under the
    ///   layer's black area, which is the whole ship frame.
    Or,
}

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
        /// How it combines with what is beneath it.
        blend: Blend,
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

    /// Appends a [`DrawCommand::Sprite`] drawn with [`Blend::Normal`].
    pub fn sprite(&mut self, image: ImageKey, center: Point, tint: Color) -> &mut Self {
        self.push(DrawCommand::Sprite {
            image,
            center,
            tint,
            blend: Blend::Normal,
        })
    }

    /// Appends a [`DrawCommand::Sprite`] drawn with [`Blend::Additive`].
    pub fn additive_sprite(&mut self, image: ImageKey, center: Point, tint: Color) -> &mut Self {
        self.push(DrawCommand::Sprite {
            image,
            center,
            tint,
            blend: Blend::Additive,
        })
    }

    /// Appends a [`DrawCommand::Sprite`] drawn with [`Blend::Or`].
    pub fn or_sprite(&mut self, image: ImageKey, center: Point, tint: Color) -> &mut Self {
        self.push(DrawCommand::Sprite {
            image,
            center,
            tint,
            blend: Blend::Or,
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

/// The tint the running lights are added with at `level` out of
/// [`nova_sim::blink::FULL`] (32): white, with that many 32nds of full
/// alpha, rounded to nearest, so level 32 adds the lights as they are.
/// Levels past 32 are 32.
///
/// # The original's light blit
///
/// Traced in `EV Nova.app/Contents/MacOS/EV Nova` (i386, `otool -tV`), in
/// the 16-bit ("thousands of colours", RGB555) depth this reproduces. Each
/// routine is cited at its entry point; instruction sites inside one are
/// given after "at".
///
/// - `_HandleShipDisplay` (0x2b514) shows the lights sprite while the
///   intensity is above 1, at level n = `trunc(intensity)`. At
///   0x2c672–0x2c692 it writes the sprite's destination factor (+0xa8) as
///   32 and its red, green and blue factors (+0xaa, +0xac, +0xae) as n;
///   an uncloaked ship's lights get no bias (+0xb0 = 0).
/// - `_BlitPixieRLETranslucentDrawProc` (0xbb3fc), the draw proc of every
///   ship's glow and lights sprites, picks `_BlitPixieRLEAddOver`
///   (0xc24bf) when all four factors are 32 and there is no bias, which
///   is level 32, and `_BlitPixieRLETranslucent` (0xc1568) otherwise,
///   which is levels 1 to 31.
/// - `_BlitPixieRLEAddOver` ORs the sprite's packed pixels into the
///   screen: `dst | src`.
/// - `_BlitPixieRLETranslucent` takes +0xa8 (32) as the destination factor
///   and each channel's factor - +0xa8 + 32, capped at 32, as that
///   channel's source factor (n). `_BlitPixieTranslucentCopy` (0xc1110)
///   then combines each 5-bit channel as `((src_c * source factor) >> 5)
///   | ((dst_c * destination factor) >> 5)`.
///
/// So at every level n from 1 to 32, per 5-bit channel:
///
/// ```text
/// out_c = ((src_c * n) >> 5) | dst_c
/// ```
///
/// A black lights pixel adds nothing, and the destination is never
/// dimmed: the lights draw no silhouette at any level. At 32 the formula
/// is `src | dst`, the `AddOver` blit, so one operator covers every level:
/// the lights scaled by n/32 and added into the screen. That is why both
/// screens draw the lights [`Blend::Additive`] with this tint at every
/// level rather than choosing the blend by level. The engine glow's sprite
/// has the same draw proc and field writes (`_HandleShipDisplay` at
/// 0x2c2b6-0x2c2d6), so flight adds the glow with this tint at its level
/// too.
///
/// Where this deviates from the original:
///
/// - **Saturating add, not bitwise OR.** `a | b` is `a + b - (a & b)`, so
///   ours matches exactly where the scaled light and the screen share no
///   set bits in a channel (always where either is black), and elsewhere
///   is brighter by `a & b` before saturating: 5-bit 16 over 16 is 16 in
///   the original and 31 here. WebGPU has no logic-op blending.
/// - **No 5-bit truncation.** The original floors `src_c * n / 32` to 5
///   bits; we scale 8-bit colour by this alpha, which differs by less than
///   one 5-bit step per channel.
/// - **Cloaking is not modelled.** A cloaked ship's bias (+0xb0 ≠ 0)
///   mixes the lights toward a colour and keeps them off `AddOver` even at
///   level 32. Nor is the 8-bit depth, whose alpha-table blits were not
///   traced.
#[must_use]
pub fn lights_tint(level: u8) -> Color {
    let full = u16::from(FULL);
    let level = u16::from(level).min(full);
    Color::rgba(255, 255, 255, ((level * 255 + full / 2) / full) as u8)
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
    fn a_lights_level_is_white_at_that_many_32nds_of_full_alpha() {
        let alphas = [32, 31, 16, 10, 1, 0].map(|level| lights_tint(level).a);
        assert_eq!(alphas, [255, 247, 128, 80, 8, 0]);
        assert_eq!(lights_tint(32), Color::WHITE);
        assert_eq!(lights_tint(16), Color::rgba(255, 255, 255, 128));
        assert_eq!(lights_tint(33), Color::WHITE, "past full is full");
        assert_eq!(lights_tint(u8::MAX), Color::WHITE);
    }

    #[test]
    fn a_sprite_draws_normally_unless_asked_to_add() {
        let tint = Color::rgba(255, 0, 0, 128);
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(200, 3), at(10.0, 20.0), tint)
            .additive_sprite(ImageKey::sprite(201, 4), at(30.0, 40.0), tint);
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [
                DrawCommand::Sprite {
                    image: ImageKey::sprite(200, 3),
                    center: at(10.0, 20.0),
                    tint,
                    blend: Blend::Normal,
                },
                DrawCommand::Sprite {
                    image: ImageKey::sprite(201, 4),
                    center: at(30.0, 40.0),
                    tint,
                    blend: Blend::Additive,
                },
            ]
        );
        assert_eq!(Blend::default(), Blend::Normal);
    }

    #[test]
    fn an_or_sprite_asks_for_the_or_composite() {
        let tint = lights_tint(10);
        let mut list = DrawList::new();
        list.or_sprite(ImageKey::sprite(202, 5), at(50.0, 60.0), tint);
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [DrawCommand::Sprite {
                image: ImageKey::sprite(202, 5),
                center: at(50.0, 60.0),
                tint,
                blend: Blend::Or,
            }]
        );
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
                tint: Color::WHITE,
                blend: Blend::Normal,
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
