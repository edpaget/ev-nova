//! How a sprite whose box the world's edge cuts is drawn: whole, or cut
//! to its box, and with which key.
//!
//! The sprite world's draw proc (`_BlitPixieRLETranslucentDrawProc`
//! @0xbb3fc-0xbba11) clips each frame's box to the world rect, the play
//! area. It takes the plain blit only when that leaves the box whole
//! (@0xbb56d-0xbb579, @0xbb770-0xbb789), drawing the whole image, past
//! the box too where the sheet overhangs it (a 32-bit `rlëD`, rendering
//! phase 1). Any cut takes the clipped blit (`_BlitPixieRLEClipped`
//! @0xbde9c-0xbe0b3, its translucent and add-over twins @0xbb6ba and
//! @0xbb703), which draws only the box's rows and columns within the
//! world, so the overhang goes on every side. A box wholly outside the
//! world draws nothing (@0xbb480-0xbb4a9), which the world clip
//! ([`DrawList::world_clip`]) already gives.
//!
//! So a sprite is drawn ([`BoxCut::of`]):
//!
//! - with no world clip, or with its box wholly inside it: as it is,
//!   overhang and all;
//! - with its box cut by the world, from a sheet that overhangs its box:
//!   cut to the box ([`DrawCommand::Clipped`]);
//! - with its box cut by the world, from a sheet whose clipped draw can
//!   differ from a crop ([`SheetEdge::clip_quirks`]): the clipped blit's
//!   frame instead ([`BoxCut::key`], [`ImageKind::RledClipped`] and, for
//!   its solid runs, [`ImageKind::RledClippedSolid`]), keyed by how many
//!   of the box's rows and pixels the world cuts at the top and the left
//!   (none for a box cut only at the right or bottom), cut to the box too;
//! - otherwise as it is: the world clip cuts a box-sized image right.
//!
//! The clipped dispatch mirrors the unclipped one routine for routine
//! (plain, add-over, translucent, nothing above level 0x3f, depth 8), so a
//! cut sprite keeps the tint, blend, colorize and `EightBitBlit` look it
//! has whole: only its image changes. A decision is a value, so a sprite's
//! companion command (a ship's solid-run overlay) takes the same one.

use crate::draw::{DrawCommand, DrawList};
use crate::geometry::{Bounds, Point};
use crate::image::{ImageKey, ImageKind};

/// What deciding a sprite's draw at the world's edge needs of its sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SheetEdge {
    /// Each frame's box, (width, height): what the sprite is placed by.
    pub frame_size: (u32, u32),
    /// Each frame's image, (width, height): the box's, unless the sheet
    /// draws past it, padded to keep the box at its centre.
    pub image_size: (u32, u32),
    /// Whether a frame the world's edge cuts can draw otherwise than its
    /// image cut to its box (`nova_data::graphics::SpriteSheet::clip_quirks`).
    /// Only an `rlëD` sheet has them.
    pub clip_quirks: bool,
}

impl SheetEdge {
    /// A sheet whose image is its box, with no clip quirks: one the world
    /// clip alone cuts right.
    #[must_use]
    pub const fn boxed(frame_size: (u32, u32)) -> Self {
        Self {
            frame_size,
            image_size: frame_size,
            clip_quirks: false,
        }
    }

    /// Whether its image draws past its box.
    fn overhangs(self) -> bool {
        self.image_size != self.frame_size
    }
}

/// How one sprite is drawn at the world's edge (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxCut {
    /// The box it is cut to, if it is.
    clip: Option<Bounds>,
    /// The clipped blit's cut, `(top, left)`, when it draws the clipped
    /// blit's frame.
    cut: Option<(u16, u16)>,
}

impl BoxCut {
    /// Drawn as it is.
    pub const WHOLE: Self = Self {
        clip: None,
        cut: None,
    };

    /// How a sprite of `sheet` centred on `centre` is drawn in a world
    /// clipped to `world`, if it is.
    #[must_use]
    pub fn of(world: Option<Bounds>, centre: Point, sheet: SheetEdge) -> Self {
        // A frame's sides are a header's u16s, far within an f32's whole
        // numbers.
        #[allow(clippy::cast_precision_loss)]
        let size = (sheet.frame_size.0 as f32, sheet.frame_size.1 as f32);
        let frame_box = Bounds::at(
            Point::new(centre.x - size.0 / 2.0, centre.y - size.1 / 2.0),
            size.0,
            size.1,
        );
        let Some(world) =
            world.filter(|world| !(world.contains(frame_box.min) && world.contains(frame_box.max)))
        else {
            return Self::WHOLE;
        };
        if sheet.clip_quirks {
            Self {
                clip: Some(frame_box),
                cut: Some(cut_by(world, frame_box, size)),
            }
        } else if sheet.overhangs() {
            Self {
                clip: Some(frame_box),
                cut: None,
            }
        } else {
            Self::WHOLE
        }
    }

    /// The key it draws for `key`: the clipped blit's frame of an `rlëD`
    /// frame or of its solid runs when it draws the clipped blit's, else
    /// `key`.
    #[must_use]
    pub fn key(self, key: ImageKey) -> ImageKey {
        match (self.cut, key.kind) {
            (Some(cut), ImageKind::Rled) => ImageKey::clipped_sprite(key.id, key.frame, cut),
            (Some(cut), ImageKind::RledSolid) => {
                ImageKey::clipped_solid_runs(key.id, key.frame, cut)
            }
            _ => key,
        }
    }

    /// Appends `command` to `list`, cut to the box when it is.
    pub fn push(self, list: &mut DrawList, command: DrawCommand) {
        match self.clip {
            Some(clip) => list.clipped(clip, command),
            None => list.push(command),
        };
    }
}

/// How much of `frame_box`, a frame of `size`, the `world` cuts at its
/// top and its left, as `(top, left)`: each rounded to whole pixels, as
/// the renderer rounds a clip's edge, and kept within the box.
fn cut_by(world: Bounds, frame_box: Bounds, (width, height): (f32, f32)) -> (u16, u16) {
    // Within a box whose sides are a header's u16s.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let side = |cut: f32, side: f32| cut.round().clamp(0.0, side) as u16;
    (
        side(world.min.y - frame_box.min.y, height),
        side(world.min.x - frame_box.min.x, width),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    /// The world: 100 square from the origin.
    const WORLD: Option<Bounds> = Some(Bounds {
        min: Point { x: 0.0, y: 0.0 },
        max: Point { x: 100.0, y: 100.0 },
    });

    /// A 10x8 box whose image is 14x8.
    fn overhanging() -> SheetEdge {
        SheetEdge {
            image_size: (14, 8),
            ..SheetEdge::boxed((10, 8))
        }
    }

    /// A 10x8 box-sized sheet with clip quirks.
    fn quirky() -> SheetEdge {
        SheetEdge {
            clip_quirks: true,
            ..SheetEdge::boxed((10, 8))
        }
    }

    /// The box of a 10x8 frame centred on `centre`.
    fn box_at(centre: Point) -> Bounds {
        Bounds::at(Point::new(centre.x - 5.0, centre.y - 4.0), 10.0, 8.0)
    }

    fn sprite(image: ImageKey) -> DrawCommand {
        DrawCommand::mixed_sprite(
            image,
            Point::new(1.0, 2.0),
            Color::WHITE,
            crate::Blend::Normal,
            None,
        )
    }

    /// What `cut` appends for frame 3 of `rlëD` 1001.
    fn pushed(cut: BoxCut) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        cut.push(&mut list, sprite(cut.key(ImageKey::sprite(1001, 3))));
        list.iter().cloned().collect()
    }

    fn whole() -> Vec<DrawCommand> {
        vec![sprite(ImageKey::sprite(1001, 3))]
    }

    fn cut_to(clip: Bounds, image: ImageKey) -> Vec<DrawCommand> {
        vec![DrawCommand::Clipped {
            clip,
            command: Box::new(sprite(image)),
        }]
    }

    /// Centres whose 10x8 box straddles the world's left, top, right and
    /// bottom edges.
    const STRADDLING: [(&str, Point); 4] = [
        ("left", Point { x: 2.0, y: 50.0 }),
        ("top", Point { x: 50.0, y: 1.0 }),
        ("right", Point { x: 98.0, y: 50.0 }),
        ("bottom", Point { x: 50.0, y: 99.0 }),
    ];

    #[test]
    fn with_no_world_clip_a_sprite_draws_whole() {
        for sheet in [overhanging(), quirky()] {
            let cut = BoxCut::of(None, Point::new(2.0, 1.0), sheet);
            assert_eq!(cut, BoxCut::WHOLE);
            assert_eq!(pushed(cut), whole());
        }
    }

    #[test]
    fn a_box_wholly_inside_the_world_draws_whole_overhang_and_all() {
        // Touching the world's edges from inside counts as inside.
        for centre in [
            Point::new(50.0, 50.0),
            Point::new(5.0, 4.0),
            Point::new(95.0, 96.0),
        ] {
            for sheet in [overhanging(), quirky()] {
                assert_eq!(
                    pushed(BoxCut::of(WORLD, centre, sheet)),
                    whole(),
                    "{centre:?}"
                );
            }
        }
    }

    #[test]
    fn an_overhanging_box_straddling_any_edge_is_cut_to_its_box() {
        for (edge, centre) in STRADDLING {
            assert_eq!(
                pushed(BoxCut::of(WORLD, centre, overhanging())),
                cut_to(box_at(centre), ImageKey::sprite(1001, 3)),
                "{edge}"
            );
        }
    }

    #[test]
    fn a_box_sized_sheet_without_quirks_straddling_an_edge_draws_whole() {
        for (edge, centre) in STRADDLING {
            let cut = BoxCut::of(WORLD, centre, SheetEdge::boxed((10, 8)));
            assert_eq!(pushed(cut), whole(), "{edge}");
        }
    }

    #[test]
    fn a_quirky_sheet_cut_at_the_top_and_left_draws_the_clipped_frame_by_its_cut() {
        let centre = Point::new(2.0, 1.0);
        assert_eq!(
            pushed(BoxCut::of(WORLD, centre, quirky())),
            cut_to(box_at(centre), ImageKey::clipped_sprite(1001, 3, (3, 3)))
        );
        // Rounded to whole pixels, and kept within the box.
        let centre = Point::new(-20.0, 3.4);
        assert_eq!(
            pushed(BoxCut::of(WORLD, centre, quirky())),
            cut_to(box_at(centre), ImageKey::clipped_sprite(1001, 3, (1, 10)))
        );
    }

    #[test]
    fn a_quirky_sheet_cut_only_at_the_right_or_bottom_draws_the_clipped_frame_uncut() {
        for (edge, centre) in &STRADDLING[2..] {
            assert_eq!(
                pushed(BoxCut::of(WORLD, *centre, quirky())),
                cut_to(box_at(*centre), ImageKey::clipped_sprite(1001, 3, (0, 0))),
                "{edge}"
            );
        }
    }

    #[test]
    fn the_clipped_key_of_solid_runs_is_their_clipped_draw_and_other_keys_stay() {
        let cut = BoxCut::of(WORLD, Point::new(2.0, 1.0), quirky());
        assert_eq!(
            cut.key(ImageKey::solid_runs(1001, 3)),
            ImageKey::clipped_solid_runs(1001, 3, (3, 3))
        );
        for other in [ImageKey::spin_sheet(1001, 3), ImageKey::picture(1001)] {
            assert_eq!(cut.key(other), other);
        }
        let cropped = BoxCut::of(WORLD, Point::new(2.0, 1.0), overhanging());
        assert_eq!(
            cropped.key(ImageKey::solid_runs(1001, 3)),
            ImageKey::solid_runs(1001, 3)
        );
    }
}
