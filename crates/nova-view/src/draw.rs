//! Draw commands and draw lists.

use nova_sim::blink::FULL;

use crate::color::Color;
use crate::colorize::Colorize;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;

/// How a sprite combines with what is beneath it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Blend {
    /// Painted over what is beneath, by its alpha.
    #[default]
    Normal,
    /// Adds its colour, scaled by its alpha, to what is beneath: a true
    /// saturating add. The main screen's pilot panel draws the ship's
    /// target picture with it, for QuickDraw's addOver (`CopyBits` mode
    /// 0x22), which wraps instead; the two agree over black, as every
    /// stock picture lies (see
    /// [`pilot_panel`](crate::menu::pilot_panel)).
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
    /// Takes, in each colour channel, the larger of its colour (scaled by
    /// its alpha) and what is beneath, and keeps the destination's alpha. A
    /// transparent texel scales to black, so it leaves what is beneath as it
    /// is. This is QuickDraw's `addMax` transfer mode (`CopyBits` mode
    /// 0x25), which `_MapDialogUpdate` copies each map nebula with (0xfc42),
    /// so overlapping nebulae, and the borders under them, take the
    /// brighter of each channel.
    Max,
    /// Crossfades it with what is beneath by OR, in 5 bits: the original's
    /// translucent copy, `_BlitPixieTranslucentCopy` (0xc1110), with its
    /// channel factors at 0, which computes
    /// `((src_c × k) >> 5) | ((dst_c × (32 − k)) >> 5)` per 5-bit channel,
    /// k the source's opacity in 32nds (32 − the sprite's translucency
    /// +0xa8). A shot's plain fade zeroes the channel factors
    /// (`_HandleShot` @0x359ac-0x359d8) and an escape pod's are never set
    /// (`_SWCreateSprite` @0xa8d56-0xa8d71; `_HandleEscapePods`
    /// @0x3a00a-0x3a024), so the general form, `min(c − t + 32, 32)` for
    /// a channel factor c, is never needed.
    ///
    /// # The tint
    ///
    /// The tint's colour multiplies the source (white, or a murk's
    /// darkening), and its alpha is the opacity: `k = round(a × 32 / 255)`,
    /// exact for [`lights_tint`]`(k)` and every alpha a fade gives, as the
    /// renderer stores it in 32nds.
    ///
    /// # The bit depth
    ///
    /// Per channel, with the texel `t` (straight RGBA) and the destination
    /// `d8`:
    ///
    /// ```text
    /// s5  = round(t.rgb × tint.rgb × 255) >> 3
    /// d5  = d8 >> 3
    /// c5  = ((s5 × k) >> 5) | ((d5 × (32 − k)) >> 5)
    /// c8  = (c5 << 3) | (c5 >> 2)
    /// out = mix(d8, c8, t.a) ;  out.a = d.a
    /// ```
    ///
    /// Unlike [`Blend::Or`], the destination is quantised to 5 bits, as the
    /// original scales it: every bit of it moves, so none can be kept at
    /// 8. A transparent texel (the original's skipped RLE pixel) and k = 0
    /// leave the destination exactly; an opaque black texel darkens it to
    /// `d5 × (32 − k) >> 5`.
    OrCrossfade,
    /// The original's translucent copy with its source and destination
    /// factors set apart: `_BlitPixieRLETranslucent` (0xc1568) with
    /// `_BlitPixieTranslucentCopy` (0xc1110), which compute
    /// `((src_c × n_c) >> 5) | ((dst_c × m) >> 5)` per 5-bit channel. A
    /// fully cloaked ship's hull at translucency 0x1e (+0xa8) with its
    /// channel factors at 2 draws so, at n = 2 − 30 + 32 = 4 and m = 30
    /// (`_HandleShipDisplay` @0x2d1d6-0x2d22a; see
    /// [`cloak_fade`](crate::flight::cloak_fade)). [`Blend::OrCrossfade`]
    /// ties m to 32 − n, so it cannot.
    ///
    /// # The tint
    ///
    /// Its colour channels are the source factors and its alpha the
    /// destination factor, each in 32nds: `n_c = round(c × 32 / 255)`,
    /// `m = round(a × 32 / 255)`, exact for [`translucent_tint`].
    ///
    /// # The bit depth
    ///
    /// As [`Blend::OrCrossfade`]'s, with n per channel in place of k and m
    /// in place of 32 − k: the source is the texel's straight colour in 5
    /// bits, the destination is quantised to 5 bits, and a transparent
    /// texel (a skipped RLE pixel) leaves the destination exactly.
    OrTranslucent,
}

impl Blend {
    /// Whether it reads what is beneath it as it draws ([`Blend::Or`] and
    /// [`Blend::OrCrossfade`]): every quad of such a batch reads the same
    /// copy of the scene, so a later overlapping quad starts a new batch.
    #[must_use]
    pub fn reads_backdrop(self) -> bool {
        matches!(self, Self::Or | Self::OrCrossfade | Self::OrTranslucent)
    }
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
    /// A sprite frame as [`DrawCommand::Sprite`] draws it, each texel
    /// first mixed toward a colour by `colorize` ([`Colorize::mix`]), as
    /// the murk and an ionized ship's tint colorize a ship's layers. A
    /// colorize that does not mix draws as the plain sprite.
    Colorized {
        /// The frame.
        image: ImageKey,
        /// Where the frame's centre goes.
        center: Point,
        /// Colour multiplier and alpha, or the OR level, as for a sprite.
        tint: Color,
        /// How it combines with what is beneath it.
        blend: Blend,
        /// The mix toward a colour.
        colorize: Colorize,
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
    /// A picture stretched (or shrunk) to fill a rectangle, untinted, and
    /// combined with what is beneath by `blend`. It follows
    /// [`DrawCommand::StretchedPicture`]'s rules: a rectangle whose width or
    /// height is not a positive, finite number draws nothing. A
    /// `StretchedPicture` is this with [`Blend::Normal`].
    BlendedPicture {
        /// The picture.
        image: ImageKey,
        /// The rectangle's top-left corner.
        top_left: Point,
        /// The rectangle's width.
        width: f32,
        /// The rectangle's height.
        height: f32,
        /// How it combines with what is beneath it.
        blend: Blend,
    },
    /// A picture repeated edge to edge from `origin`, as QuickDraw's
    /// `FillCRect` tiles a pattern from the port's origin, and drawn only
    /// inside `area`, untinted. An area with no drawable size draws
    /// nothing.
    TiledPicture {
        /// The picture.
        image: ImageKey,
        /// Where it shows.
        area: Bounds,
        /// Where one copy's top-left corner is, which every other copy
        /// lines up with: the screen's origin, (0, 0), unless the command
        /// was moved ([`DrawList::extend_translated`]), so a pattern keeps
        /// its phase where its frame goes.
        origin: Point,
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
    /// A rectangle whose colour is OR-ed into what is beneath it, per 5-bit
    /// channel, keeping the destination's alpha. The arithmetic is
    /// [`Blend::Or`]'s at level 32 (`s5 = c8 >> 3`,
    /// `out = d8 | ((s5 << 3) | (s5 >> 2))`); see it for the bit depth.
    /// The original's `_BlitPixieTranslucentCopy` (0xc1110) with every
    /// factor at 32, called by `_BlitPixieFloatingRect` @0xbe501-0xbe555,
    /// combines a rect so. An area with no drawable size draws nothing.
    OrFill {
        /// Where it ORs.
        area: Bounds,
        /// The colour OR-ed in; its alpha is unused.
        color: Color,
    },
    /// `command` drawn only inside `clip`: a text run cut to it, a
    /// sprite's or picture's quad cropped to it with its texture
    /// coordinates, a tiled picture's or OR fill's area cut to it. A line or a dot is
    /// drawn as it is, uncut. The floating map clips its names and arrows
    /// to its frame so.
    Clipped {
        /// Where it may draw.
        clip: Bounds,
        /// What it draws.
        command: Box<DrawCommand>,
    },
}

impl DrawCommand {
    /// The command moved by `by` ([`DrawList::extend_translated`]).
    #[must_use]
    pub fn translated(&self, by: Point) -> Self {
        let moved = |p: Point| Point::new(p.x + by.x, p.y + by.y);
        let moved_bounds = |b: Bounds| Bounds {
            min: moved(b.min),
            max: moved(b.max),
        };
        let mut command = self.clone();
        match &mut command {
            Self::Sprite { center, .. }
            | Self::Colorized { center, .. }
            | Self::Dot { center, .. } => *center = moved(*center),
            Self::Picture { top_left, .. }
            | Self::StretchedPicture { top_left, .. }
            | Self::BlendedPicture { top_left, .. } => *top_left = moved(*top_left),
            Self::OrFill { area, .. } => *area = moved_bounds(*area),
            Self::TiledPicture { area, origin, .. } => {
                *area = moved_bounds(*area);
                *origin = moved(*origin);
            }
            Self::Text { origin, .. } => *origin = moved(*origin),
            Self::Line { from, to, .. } => {
                *from = moved(*from);
                *to = moved(*to);
            }
            Self::Clipped { clip, command } => {
                *clip = moved_bounds(*clip);
                **command = command.translated(by);
            }
        }
        command
    }
}

impl DrawCommand {
    /// Whether it draws text: a [`DrawCommand::Text`], clipped or not.
    fn is_text(&self) -> bool {
        match self {
            Self::Text { .. } => true,
            Self::Clipped { command, .. } => command.is_text(),
            _ => false,
        }
    }
}

/// Which layer of the frame a command is drawn in.
///
/// Only flight draws a world layer ([`DrawList::world`]): space, the
/// stellars, asteroids, boxes, beams, ships, shots, effects and smoke, the
/// brackets, the cursor and the red-alert and double-time icons, the
/// message line's box, the floating map and the escort menu (but for
/// their text), everything the original draws into its sprite world.
/// Everything else (the HUD, every text, the course map, the dialogs and
/// the fades) is overlay, and so is all of every other screen's drawing.
///
/// The world may be clipped ([`DrawList::world_in`]): flight clips it to
/// the play area, the window less the status bar's column, as the
/// original's sprite world ends there. The overlay is never clipped by it.
///
/// A renderer may draw the world at the logical resolution and enlarge it
/// (a frame upscaler), drawing the overlay over it at the window's own.
/// Drawn at the window's own resolution, the layers change nothing: the
/// commands are drawn in the list's order, whatever their layers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DrawLayer {
    /// The sprite world, under the overlay.
    World,
    /// Drawn at the window's own resolution, over the world.
    #[default]
    Overlay,
}

/// A frame's draw commands, drawn in order (later ones on top), each in
/// its [`DrawLayer`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawList {
    commands: Vec<DrawCommand>,
    /// Each command's layer, index for index.
    layers: Vec<DrawLayer>,
    /// Where the world may draw, if it is clipped ([`DrawList::world_in`]).
    /// One clip serves the whole list: only flight draws a world, and it
    /// draws one, clipped to its play area.
    world_clip: Option<Bounds>,
}

impl DrawList {
    /// An empty list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends `command`, in the overlay.
    pub fn push(&mut self, command: DrawCommand) -> &mut Self {
        self.commands.push(command);
        self.layers.push(DrawLayer::Overlay);
        self
    }

    /// Has `draw` append the world's commands: each one it appends is in
    /// [`DrawLayer::World`], except text, which stays in the overlay. Text
    /// is drawn at the window's own resolution, so a world drawn at the
    /// logical resolution never holds it, even text drawn among the world's
    /// sprites (flight's message line, a placeholder's reason).
    pub fn world(&mut self, draw: impl FnOnce(&mut Self)) -> &mut Self {
        let start = self.commands.len();
        draw(self);
        for (command, layer) in self.commands[start..].iter().zip(&mut self.layers[start..]) {
            if !command.is_text() {
                *layer = DrawLayer::World;
            }
        }
        self
    }

    /// Has `draw` append the world's commands as [`DrawList::world`] does,
    /// and clips the world to `clip`: no world command draws outside it.
    /// Flight clips its world to the play area so. Text drawn in `draw`
    /// stays in the overlay and isn't clipped by it.
    pub fn world_in(&mut self, clip: Bounds, draw: impl FnOnce(&mut Self)) -> &mut Self {
        self.world_clip = Some(clip);
        self.world(draw)
    }

    /// Where the world may draw, if it is clipped ([`DrawList::world_in`]).
    #[must_use]
    pub fn world_clip(&self) -> Option<Bounds> {
        self.world_clip
    }

    /// The commands, in draw order.
    pub fn iter(&self) -> std::slice::Iter<'_, DrawCommand> {
        self.commands.iter()
    }

    /// The commands with their layers, in draw order.
    pub fn layered(&self) -> impl Iterator<Item = (DrawLayer, &DrawCommand)> {
        self.layers.iter().copied().zip(&self.commands)
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

    /// Appends a [`DrawCommand::Sprite`] drawn with [`Blend::OrCrossfade`].
    pub fn crossfade_sprite(&mut self, image: ImageKey, center: Point, tint: Color) -> &mut Self {
        self.push(DrawCommand::Sprite {
            image,
            center,
            tint,
            blend: Blend::OrCrossfade,
        })
    }

    /// Appends a [`DrawCommand::Sprite`] drawn with [`Blend::OrTranslucent`].
    pub fn translucent_sprite(&mut self, image: ImageKey, center: Point, tint: Color) -> &mut Self {
        self.push(DrawCommand::Sprite {
            image,
            center,
            tint,
            blend: Blend::OrTranslucent,
        })
    }

    /// Appends a [`DrawCommand::Colorized`] combined by `blend`.
    pub fn colorized_sprite(
        &mut self,
        image: ImageKey,
        center: Point,
        tint: Color,
        blend: Blend,
        colorize: Colorize,
    ) -> &mut Self {
        self.push(DrawCommand::Colorized {
            image,
            center,
            tint,
            blend,
            colorize,
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

    /// Appends a [`DrawCommand::BlendedPicture`].
    pub fn blended_picture(
        &mut self,
        image: ImageKey,
        top_left: Point,
        width: f32,
        height: f32,
        blend: Blend,
    ) -> &mut Self {
        self.push(DrawCommand::BlendedPicture {
            image,
            top_left,
            width,
            height,
            blend,
        })
    }

    /// Appends a [`DrawCommand::TiledPicture`] tiled from the screen's
    /// origin, (0, 0).
    pub fn tiled_picture(&mut self, image: ImageKey, area: Bounds) -> &mut Self {
        self.push(DrawCommand::TiledPicture {
            image,
            area,
            origin: Point::new(0.0, 0.0),
        })
    }

    /// Appends every command of `other`, in order, moved by `by`: each
    /// point, each rectangle and each clip, and a tiled picture's tiling
    /// origin with its area, so what `other` draws looks the same, `by`
    /// further on. A frame drawn for Nova's 1024x768 is drawn centred in a
    /// widened view so. Each keeps its layer. `other`'s world clip, moved
    /// by `by`, replaces this list's if `other` has one.
    pub fn extend_translated(&mut self, other: &Self, by: Point) -> &mut Self {
        self.commands
            .extend(other.iter().map(|command| command.translated(by)));
        self.layers.extend(&other.layers);
        if let Some(clip) = other.world_clip {
            self.world_clip = Some(clip.offset(by));
        }
        self
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

    /// Appends a [`DrawCommand::OrFill`].
    pub fn or_fill(&mut self, area: Bounds, color: Color) -> &mut Self {
        self.push(DrawCommand::OrFill { area, color })
    }

    /// Appends `command` drawn only inside `clip` ([`DrawCommand::Clipped`]).
    pub fn clipped(&mut self, clip: Bounds, command: DrawCommand) -> &mut Self {
        self.push(DrawCommand::Clipped {
            clip,
            command: Box::new(command),
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

/// The tint the running lights (and flight's engine glow) are drawn with
/// at `level` out of [`nova_sim::blink::FULL`] (32): white, with that many
/// 32nds of full alpha, rounded to nearest, so level 32 draws the layer as
/// it is. Levels past 32 are 32. Under [`Blend::Or`] the alpha encodes n
/// exactly: `round(a × 32 / 255) = n` for every n from 0 to 32.
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
/// the lights scaled by n/32 and combined into the screen by OR. That is
/// why both screens draw the lights with [`Blend::Or`] at this tint at
/// every level rather than choosing the blend by level; [`Blend::Or`]
/// records how the renderer reproduces the formula, 5-bit floor included.
/// The engine glow's sprite has the same draw proc and field writes
/// (`_HandleShipDisplay` at 0x2c2b6-0x2c2d6), so flight ORs the glow in
/// at this tint at its level too.
///
/// Where this deviates from the original:
///
/// - **The 8-bit depth is not modelled**: its alpha-table blits were not
///   traced.
///
/// A cloaking ship's layers take no bias either: `_HandleShipDisplay`
/// leaves +0xb0 alone below a full fade and caps each channel factor at
/// trunc(32 − fade) (@0x2d4ed-0x2d641), which takes a level of 32 off
/// `AddOver` onto the translucent blit, the same OR formula at the capped
/// level (see [`cloak_fade`](crate::flight::cloak_fade)).
#[must_use]
pub fn lights_tint(level: u8) -> Color {
    let full = u16::from(FULL);
    let level = u16::from(level).min(full);
    Color::rgba(255, 255, 255, ((level * 255 + full / 2) / full) as u8)
}

/// The tint of a [`Blend::OrTranslucent`] sprite whose source factor is
/// `source` and destination factor `destination`, each in 32nds and held
/// to 32: white at `source` 32nds, at `destination` 32nds of full alpha.
#[must_use]
pub fn translucent_tint(source: u8, destination: u8) -> Color {
    let full = u16::from(FULL);
    let scale = |n: u8| ((u16::from(n).min(full) * 255 + full / 2) / full) as u8;
    let n = scale(source);
    Color::rgba(n, n, n, scale(destination))
}

/// Fills `area` with `color`. There is no painted rectangle command (an OR
/// is [`DrawCommand::OrFill`]): a horizontal line as thick as the area,
/// along its middle, is one.
pub fn fill_rect(list: &mut DrawList, area: Bounds, color: Color) {
    let middle = area.center().y;
    list.line(
        Point::new(area.min.x, middle),
        Point::new(area.max.x, middle),
        area.height(),
        color,
    );
}

/// A one-pixel rule from `from` to `to` in `color`, as a pen-1 `LineTo`
/// covers it: both ends included.
pub fn rule(list: &mut DrawList, from: Point, to: Point, color: Color) {
    let area = Bounds {
        min: from,
        max: Point::new(to.x + 1.0, to.y + 1.0),
    };
    fill_rect(list, area, color);
}

/// QuickDraw's `FrameRect(rect)` in `color`: a one-pixel border just
/// inside it, ruled top, bottom, left, then right.
pub fn frame_rect(list: &mut DrawList, rect: Bounds, color: Color) {
    let (left, top) = (rect.min.x, rect.min.y);
    let (right, bottom) = (rect.max.x - 1.0, rect.max.y - 1.0);
    rule(list, Point::new(left, top), Point::new(right, top), color);
    rule(
        list,
        Point::new(left, bottom),
        Point::new(right, bottom),
        color,
    );
    rule(list, Point::new(left, top), Point::new(left, bottom), color);
    rule(
        list,
        Point::new(right, top),
        Point::new(right, bottom),
        color,
    );
}

/// The one command [`fill_rect`] draws for `area` in `color`, for tests
/// to compare drawn commands against.
#[cfg(test)]
pub(crate) fn fill_command(area: Bounds, color: Color) -> DrawCommand {
    let mut list = DrawList::new();
    fill_rect(&mut list, area, color);
    list.iter().next().cloned().expect("a fill is one command")
}

/// The 8-bit value of `n` 32nds of full, rounded half up, for `n` in
/// 0..=32, written out: what [`lights_tint`], [`translucent_tint`] and
/// `see_through` must give, and what nova-render's tests write as
/// literals.
#[cfg(test)]
pub(crate) const THIRTY_SECONDS: [u8; 33] = [
    0, 8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120, 128, 135, 143, 151, 159, 167,
    175, 183, 191, 199, 207, 215, 223, 231, 239, 247, 255,
];

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
    fn a_rule_covers_both_ends_as_a_pen_1_line_to_does() {
        let mut list = DrawList::new();
        rule(&mut list, at(2.0, 5.0), at(9.0, 5.0), Color::WHITE);
        let area = Bounds {
            min: at(2.0, 5.0),
            max: at(10.0, 6.0),
        };
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [fill_command(area, Color::WHITE)]
        );
    }

    #[test]
    fn frame_rect_rules_a_one_pixel_border_just_inside_the_rect() {
        let red = Color::rgba(255, 0, 0, 255);
        let mut list = DrawList::new();
        frame_rect(&mut list, Bounds::at(at(10.0, 20.0), 84.0, 55.0), red);
        let edge = |min: Point, max: Point| fill_command(Bounds { min, max }, red);
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [
                edge(at(10.0, 20.0), at(94.0, 21.0)),
                edge(at(10.0, 74.0), at(94.0, 75.0)),
                edge(at(10.0, 20.0), at(11.0, 75.0)),
                edge(at(93.0, 20.0), at(94.0, 75.0)),
            ]
        );
    }

    #[test]
    fn a_translucent_tint_carries_its_source_factor_in_colour_and_its_destination_in_alpha() {
        assert_eq!(translucent_tint(4, 30), Color::rgba(32, 32, 32, 239));
        assert_eq!(translucent_tint(32, 32), Color::WHITE);
        assert_eq!(translucent_tint(40, 0), Color::rgba(255, 255, 255, 0));
        assert!(Blend::OrTranslucent.reads_backdrop());
        let mut list = DrawList::new();
        let tint = translucent_tint(4, 30);
        list.translucent_sprite(ImageKey::sprite(1, 0), at(1.0, 2.0), tint)
            .translucent_sprite(ImageKey::sprite(2, 0), at(3.0, 4.0), tint);
        assert_eq!(list.iter().count(), 2, "chained onto the list");
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
    fn every_lights_level_is_white_at_exactly_its_32nds_alpha() {
        for (level, &alpha) in (0..=32).zip(&THIRTY_SECONDS) {
            assert_eq!(
                lights_tint(level),
                Color::rgba(255, 255, 255, alpha),
                "level {level}"
            );
            let back = (u16::from(alpha) * 32 + 127) / 255;
            assert_eq!(back, u16::from(level), "level {level} reads back in 32nds");
        }
        assert_eq!(lights_tint(10).a, 80);
        assert_eq!(lights_tint(20).a, 159);
        assert_eq!(lights_tint(24).a, 191);
    }

    #[test]
    fn every_translucent_factor_pair_is_exactly_its_32nds() {
        for (source, &n) in (0..=32).zip(&THIRTY_SECONDS) {
            for (destination, &m) in (0..=32).zip(&THIRTY_SECONDS) {
                assert_eq!(
                    translucent_tint(source, destination),
                    Color::rgba(n, n, n, m),
                    "({source}, {destination})"
                );
            }
        }
        assert_eq!(translucent_tint(16, 8), Color::rgba(128, 128, 128, 64));
        assert_eq!(translucent_tint(17, 9), Color::rgba(135, 135, 135, 72));
        assert_eq!(translucent_tint(0, 32), Color::rgba(0, 0, 0, 255));
        assert_eq!(translucent_tint(32, 0), Color::rgba(255, 255, 255, 0));
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
    fn a_crossfade_sprite_asks_for_the_or_crossfade() {
        let tint = Color::rgba(200, 200, 200, 191);
        let mut list = DrawList::new();
        list.crossfade_sprite(ImageKey::sprite(202, 5), at(50.0, 60.0), tint);
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [DrawCommand::Sprite {
                image: ImageKey::sprite(202, 5),
                center: at(50.0, 60.0),
                tint,
                blend: Blend::OrCrossfade,
            }]
        );
    }

    #[test]
    fn only_the_or_blends_read_the_backdrop() {
        let reads: Vec<(Blend, bool)> = [
            Blend::Normal,
            Blend::Additive,
            Blend::Or,
            Blend::Max,
            Blend::OrCrossfade,
        ]
        .map(|blend| (blend, blend.reads_backdrop()))
        .into();
        assert_eq!(
            reads,
            [
                (Blend::Normal, false),
                (Blend::Additive, false),
                (Blend::Or, true),
                (Blend::Max, false),
                (Blend::OrCrossfade, true),
            ]
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
    fn a_blended_picture_keeps_its_rectangle_and_its_blend() {
        let mut list = DrawList::new();
        list.blended_picture(
            ImageKey::picture(9502),
            at(-3.5, 20.0),
            188.25,
            223.5,
            Blend::Max,
        )
        .picture(ImageKey::picture(1), at(0.0, 0.0));
        let commands: Vec<&DrawCommand> = list.iter().collect();
        assert_eq!(
            *commands[0],
            DrawCommand::BlendedPicture {
                image: ImageKey::picture(9502),
                top_left: at(-3.5, 20.0),
                width: 188.25,
                height: 223.5,
                blend: Blend::Max,
            }
        );
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn a_tiled_picture_keeps_its_area() {
        let area = Bounds {
            min: at(838.0, 8.0),
            max: at(1014.0, 184.0),
        };
        let mut list = DrawList::new();
        list.tiled_picture(ImageKey::pattern(130), area)
            .picture(ImageKey::picture(1), at(0.0, 0.0));
        let commands: Vec<&DrawCommand> = list.iter().collect();
        assert_eq!(
            *commands[0],
            DrawCommand::TiledPicture {
                image: ImageKey::pattern(130),
                area,
                origin: at(0.0, 0.0),
            }
        );
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn an_or_fill_keeps_its_area_and_colour() {
        let area = Bounds {
            min: at(25.0, 700.0),
            max: at(805.0, 716.0),
        };
        let mut list = DrawList::new();
        list.or_fill(area, Color::rgba(0, 8, 16, 255));
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            vec![DrawCommand::OrFill {
                area,
                color: Color::rgba(0, 8, 16, 255),
            }]
        );
    }

    /// One of each command at `(x, y)`-based points, the clipped one
    /// clipping a tiled picture.
    fn one_of_each(x: f32, y: f32, origin: Point) -> Vec<DrawCommand> {
        let p = |dx: f32, dy: f32| at(x + dx, y + dy);
        let area = |dx: f32| Bounds {
            min: p(dx, 0.0),
            max: p(dx + 10.0, 10.0),
        };
        vec![
            DrawCommand::Sprite {
                image: ImageKey::sprite(1, 0),
                center: p(1.0, 2.0),
                tint: Color::WHITE,
                blend: Blend::Additive,
            },
            DrawCommand::Picture {
                image: ImageKey::picture(2),
                top_left: p(3.0, 4.0),
            },
            DrawCommand::StretchedPicture {
                image: ImageKey::picture(3),
                top_left: p(5.0, 6.0),
                width: 7.0,
                height: 8.0,
            },
            DrawCommand::BlendedPicture {
                image: ImageKey::picture(4),
                top_left: p(9.0, 10.0),
                width: 11.0,
                height: 12.0,
                blend: Blend::Max,
            },
            DrawCommand::TiledPicture {
                image: ImageKey::pattern(5),
                area: area(13.0),
                origin,
            },
            DrawCommand::Text {
                text: "Hi".to_owned(),
                font: Font::Charcoal,
                origin: p(14.0, 15.0),
                size: 12.0,
                wrap_width: Some(100.0),
                color: Color::WHITE,
            },
            DrawCommand::Line {
                from: p(16.0, 17.0),
                to: p(18.0, 19.0),
                width: 2.0,
                color: Color::BLACK,
            },
            DrawCommand::Dot {
                center: p(20.0, 21.0),
                size: 3.0,
                color: Color::WHITE,
            },
            DrawCommand::OrFill {
                area: area(24.0),
                color: Color::rgba(0, 8, 16, 255),
            },
            DrawCommand::Clipped {
                clip: area(22.0),
                command: Box::new(DrawCommand::TiledPicture {
                    image: ImageKey::pattern(6),
                    area: area(23.0),
                    origin,
                }),
            },
        ]
    }

    #[test]
    fn a_list_extended_translated_moves_every_command_and_its_tiling_origin() {
        let mut other = DrawList::new();
        for command in one_of_each(0.0, 0.0, at(0.0, 0.0)) {
            other.push(command);
        }
        let mut list = DrawList::new();
        list.dot(at(1.0, 1.0), 1.0, Color::WHITE);
        list.extend_translated(&other, at(170.0, -5.0));
        let mut expected = vec![DrawCommand::Dot {
            center: at(1.0, 1.0),
            size: 1.0,
            color: Color::WHITE,
        }];
        expected.extend(one_of_each(170.0, -5.0, at(170.0, -5.0)));
        assert_eq!(list.iter().cloned().collect::<Vec<_>>(), expected);
    }

    #[test]
    fn a_list_extended_untranslated_is_appended_as_it_is() {
        let mut other = DrawList::new();
        for command in one_of_each(4.0, 2.0, at(1.0, 1.0)) {
            other.push(command);
        }
        let mut list = DrawList::new();
        list.extend_translated(&other, at(0.0, 0.0));
        assert_eq!(list, other);
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

    /// Each command's layer, in draw order.
    fn layers(list: &DrawList) -> Vec<DrawLayer> {
        list.layered().map(|(layer, _)| layer).collect()
    }

    #[test]
    fn commands_drawn_in_the_world_are_world_and_the_rest_overlay() {
        let mut list = DrawList::new();
        list.dot(at(1.0, 0.0), 1.0, Color::WHITE);
        list.world(|list| {
            list.dot(at(2.0, 0.0), 1.0, Color::WHITE).sprite(
                ImageKey::sprite(200, 0),
                at(3.0, 0.0),
                Color::WHITE,
            );
        });
        list.dot(at(4.0, 0.0), 1.0, Color::WHITE);
        assert_eq!(
            layers(&list),
            [
                DrawLayer::Overlay,
                DrawLayer::World,
                DrawLayer::World,
                DrawLayer::Overlay
            ]
        );
        let layered: Vec<&DrawCommand> = list.layered().map(|(_, command)| command).collect();
        assert_eq!(layered, list.iter().collect::<Vec<_>>());
        assert_eq!(list.len(), 4);
        assert_eq!(DrawLayer::default(), DrawLayer::Overlay);
    }

    #[test]
    fn text_drawn_in_the_world_stays_in_the_overlay() {
        let mut list = DrawList::new();
        list.world(|list| {
            list.dot(at(1.0, 0.0), 1.0, Color::WHITE).text(
                "message",
                at(0.0, 0.0),
                10.0,
                None,
                Color::WHITE,
            );
            let clip = Bounds {
                min: at(0.0, 0.0),
                max: at(9.0, 9.0),
            };
            let text = DrawCommand::Text {
                text: "clipped".into(),
                font: Font::Geneva,
                origin: at(0.0, 0.0),
                size: 10.0,
                wrap_width: None,
                color: Color::WHITE,
            };
            list.clipped(
                clip,
                DrawCommand::Clipped {
                    clip,
                    command: Box::new(text),
                },
            )
            .clipped(
                clip,
                DrawCommand::Picture {
                    image: ImageKey::picture(128),
                    top_left: at(0.0, 0.0),
                },
            );
        });
        assert_eq!(
            layers(&list),
            [
                DrawLayer::World,
                DrawLayer::Overlay,
                DrawLayer::Overlay,
                DrawLayer::World
            ]
        );
    }

    #[test]
    fn a_world_inside_the_world_is_still_world() {
        let mut list = DrawList::new();
        list.world(|list| {
            list.dot(at(1.0, 0.0), 1.0, Color::WHITE);
            list.world(|list| {
                list.dot(at(2.0, 0.0), 1.0, Color::WHITE);
            });
            list.dot(at(3.0, 0.0), 1.0, Color::WHITE);
        });
        assert_eq!(layers(&list), [DrawLayer::World; 3]);
    }

    #[test]
    fn a_list_extended_translated_keeps_its_layers() {
        let mut other = DrawList::new();
        other.dot(at(1.0, 0.0), 1.0, Color::WHITE);
        other.world(|list| {
            list.dot(at(2.0, 0.0), 1.0, Color::WHITE);
        });
        let mut list = DrawList::new();
        list.world(|list| {
            list.dot(at(0.0, 0.0), 1.0, Color::WHITE);
        });
        list.extend_translated(&other, at(5.0, 5.0));
        assert_eq!(
            layers(&list),
            [DrawLayer::World, DrawLayer::Overlay, DrawLayer::World]
        );
        // Moved into a world, an overlay command is world too.
        let mut world = DrawList::new();
        world.world(|list| {
            list.extend_translated(&other, at(0.0, 0.0));
        });
        assert_eq!(layers(&world), [DrawLayer::World; 2]);
    }

    /// A clip for the world tests.
    fn play_area() -> Bounds {
        Bounds {
            min: at(0.0, 0.0),
            max: at(830.0, 768.0),
        }
    }

    #[test]
    fn a_world_drawn_in_a_clip_is_world_but_for_its_text_and_records_the_clip() {
        let mut list = DrawList::new();
        list.dot(at(0.0, 0.0), 1.0, Color::WHITE);
        list.world_in(play_area(), |list| {
            list.dot(at(1.0, 0.0), 1.0, Color::WHITE).text(
                "message",
                at(0.0, 0.0),
                10.0,
                None,
                Color::WHITE,
            );
        });
        assert_eq!(
            layers(&list),
            [DrawLayer::Overlay, DrawLayer::World, DrawLayer::Overlay]
        );
        assert_eq!(list.world_clip(), Some(play_area()));
    }

    #[test]
    fn a_fresh_list_and_a_world_drawn_unclipped_have_no_clip() {
        assert_eq!(DrawList::new().world_clip(), None);
        let mut list = DrawList::new();
        list.world(|list| {
            list.dot(at(1.0, 0.0), 1.0, Color::WHITE);
        });
        assert_eq!(list.world_clip(), None);
    }

    #[test]
    fn a_list_extended_translated_takes_the_others_clip_moved() {
        let mut other = DrawList::new();
        other.world_in(play_area(), |list| {
            list.dot(at(1.0, 0.0), 1.0, Color::WHITE);
        });
        let mut list = DrawList::new();
        list.world_in(Bounds::at(at(0.0, 0.0), 5.0, 5.0), |_| {});
        list.extend_translated(&other, at(170.0, -5.0));
        assert_eq!(list.world_clip(), Some(play_area().offset(at(170.0, -5.0))));
    }

    #[test]
    fn a_list_extended_by_an_unclipped_one_keeps_its_own_clip() {
        let mut other = DrawList::new();
        other.dot(at(1.0, 0.0), 1.0, Color::WHITE);
        let mut list = DrawList::new();
        list.world_in(play_area(), |_| {});
        list.extend_translated(&other, at(170.0, -5.0));
        assert_eq!(list.world_clip(), Some(play_area()));
    }

    #[test]
    fn lists_drawing_the_same_commands_in_different_layers_differ() {
        let mut overlay = DrawList::new();
        overlay.dot(at(1.0, 0.0), 1.0, Color::WHITE);
        let mut world = DrawList::new();
        world.world(|list| {
            list.dot(at(1.0, 0.0), 1.0, Color::WHITE);
        });
        assert_eq!(
            overlay.iter().collect::<Vec<_>>(),
            world.iter().collect::<Vec<_>>()
        );
        assert_ne!(overlay, world);
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
