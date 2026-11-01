//! A sprite's solid runs, drawn raw over its layer, as the `EV Nova`
//! executable's 16-bit RLE blits fill them (rdm ship-handling-fidelity
//! phase 23, findings 7-11).
//!
//! - **The runs.** An `rlëD` frame's opcode-4 tokens are runs of one
//!   colour. The original loads a ship's `rlëD` straight from its tokens
//!   (`_LoadSprites` @0x205bf, `_LoadMasterStellarSprite` @0x1ebcc, through
//!   `_SWCreateSpriteFromRLEResource` @0xa99a9-0xa9bdc), so its runs reach
//!   the blits as stored.
//! - **The fill.** Every 16-bit RLE blit fills a run with
//!   `_BlitPixieMemSet`, its 32-bit pattern as stored: `_BlitPixieRLE`
//!   @0xbde38, `_BlitPixieRLEClipped` @0xbdf5a, `_BlitPixieRLETranslucent`
//!   @0xc170b (@0xc16f5), `_BlitPixieRLETranslucentClipped` @0xc1afc,
//!   `_BlitPixieRLEAddOver` @0xc251b and `_BlitPixieRLEAddOverClipped`
//!   @0xc26e5. Whichever path `_BlitPixieRLETranslucentDrawProc` picks
//!   (@0xbb5b6-0xbb9e7), a run is drawn opaque in its own colour: no
//!   colorize, no channel factors, no translucency, and in an add-over blit no OR.
//! - **What it changes.** The engine decodes every texel and draws each
//!   layer whole, colorized, translucent or OR-ed. Over that, each layer
//!   whose sheet holds runs ([`LayerSheet::solid`], [`ShipSheet::solid`])
//!   draws its runs alone again ([`ImageKey::solid_runs`]), plainly and
//!   opaque ([`solid_overlay`]), right after itself on the same frame and
//!   before the next layer, so later layers still cover them. A layer
//!   held to level none draws nothing, its runs neither (finding 14).
//! - **Stock.** One stock `rlëD` holds runs: 1930, the Aurora
//!   Thunderforge's weapon glow (`shän` 380's `WeapImageID`), 16 white runs
//!   in frames 13-15, 17-18, 45-47 and 49-51. So its weapon glow's runs stay
//!   full white while the rest of the glow fades, and while an ion tint,
//!   murk or gate colorizes it. At level 32 with no colorize the OR with
//!   white is white already, and the two agree.
//!
//! The 8-bit blits (`_BlitPixieRLEAddOver8`, `_BlitPixieRLETranslucent8`,
//! @0xbb85b-0xbb9e7) aren't modelled, as the engine draws at 16-bit only.
//! A `PICT`-built sprite's runs (`_BlitPixieRLEParsePixels` @0xc2875,
//! opcode 4 @0xc29c7, for `_SWCreateRLEData` @0xaeeb6) aren't a ship
//! layer's, and no stock `spïn` `rlëD` holds a run.
//!
//! [`LayerSheet::solid`]: super::catalog::LayerSheet::solid
//! [`ShipSheet::solid`]: super::catalog::ShipSheet::solid

use crate::{Color, DrawList, ImageKey, Point};

/// Draws frame `frame` of `rlëD` `sheet_id`'s solid runs at `at`: plainly,
/// opaque and uncolorized, over the layer just drawn.
pub fn solid_overlay(list: &mut DrawList, sheet_id: i16, frame: u16, at: Point) {
    list.sprite(ImageKey::solid_runs(sheet_id, frame), at, Color::WHITE);
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use nova_sim::combat::ship_frames::{Alt, Drawn, FrameSpec};

    use super::*;
    use crate::flight::catalog::{LayerSheet, ShipSheet};
    use crate::flight::held_alt::AltLayer;
    use crate::flight::ship_layers::ShipDraw;
    use crate::{Blend, Colorize, DrawCommand, ImageKind};

    const AT: Point = Point::new(40.0, 30.0);

    #[test]
    fn the_overlay_is_the_runs_frame_drawn_plainly_in_white() {
        let mut list = DrawList::new();
        solid_overlay(&mut list, 1930, 13, AT);
        assert_eq!(
            list.iter().collect::<Vec<_>>(),
            [&DrawCommand::Sprite {
                image: ImageKey::solid_runs(1930, 13),
                center: AT,
                tint: Color::WHITE,
                blend: Blend::Normal,
            }]
        );
    }

    fn layer(image_id: i16, solid: bool) -> LayerSheet {
        LayerSheet {
            image_id,
            frames: NonZeroU16::new(36).expect("non-zero"),
            frame_width: crate::flight::fake_catalog::LAYER_SIDE,
            frame_height: crate::flight::fake_catalog::LAYER_SIDE,
            solid,
        }
    }

    /// A hull `rlëD` 1000 of 36 rotations, with a glow 1100, lights 1200,
    /// weapon glow 1930, alt 1400 and shield 1500; `solid` says which hold
    /// runs, by ID.
    fn sheet(solid: &[i16]) -> ShipSheet {
        let has = |id| solid.contains(&id);
        ShipSheet {
            image_id: 1000,
            rotations: NonZeroU16::new(36).expect("non-zero"),
            frame_width: 10,
            frame_height: 10,
            glow: Some(layer(1100, has(1100))),
            lights: Some(layer(1200, has(1200))),
            weapon: Some(layer(1930, has(1930))),
            weap_decay: 0,
            alt: Some(layer(1400, has(1400))),
            shield: Some(layer(1500, has(1500))),
            base_transp: 0,
            solid: has(1000),
        }
    }

    /// Every image drawn by `ship` with an alt layer of `rlëD` 1400 frame
    /// 0, in order: its kind and ID.
    fn drawn(ship: &ShipDraw) -> Vec<(ImageKind, i16)> {
        let mut list = DrawList::new();
        let alt = AltLayer {
            image: ImageKey::sprite(1400, 0),
            at: AT,
            tint: Color::WHITE,
            colorize: None,
            blend: crate::Blend::Normal,
        };
        ship.draw(&mut list, &|_| AT, Some(alt));
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Sprite { image, .. } | DrawCommand::Colorized { image, .. } => {
                    Some((image.kind, image.id))
                }
                _ => None,
            })
            .collect()
    }

    fn ship<'a>(sheet: &'a ShipSheet, spec: &'a FrameSpec) -> ShipDraw<'a> {
        ShipDraw {
            sheet,
            spec,
            drawn: Drawn {
                alt: Alt::Drawn(0),
                ..Drawn::default()
            },
            heading: 0.0,
            tint: Color::WHITE,
            paint: None,
            cloak: None,
            hull: true,
            held: true,
            glow: Some(20),
            lights: Some(20),
            weapon: Some(20),
            shield: Some(20),
            colorize: Some(Colorize::toward(16, 0x0034_c2ff)),
        }
    }

    const RUNS: ImageKind = ImageKind::RledSolid;
    const SHEET: ImageKind = ImageKind::Rled;

    #[test]
    fn each_layer_whose_sheet_holds_runs_draws_them_right_after_itself() {
        let sheet = sheet(&[1000, 1100, 1200, 1930, 1400, 1500]);
        let spec = FrameSpec::NONE;
        assert_eq!(
            drawn(&ship(&sheet, &spec)),
            [
                (SHEET, 1000),
                (RUNS, 1000),
                (SHEET, 1100),
                (RUNS, 1100),
                (SHEET, 1200),
                (RUNS, 1200),
                (SHEET, 1930),
                (RUNS, 1930),
                (SHEET, 1400),
                (RUNS, 1400),
                (SHEET, 1500),
                (RUNS, 1500),
            ]
        );
    }

    #[test]
    fn a_sheet_without_runs_draws_no_overlay() {
        let sheet = sheet(&[1930]);
        let spec = FrameSpec::NONE;
        assert_eq!(
            drawn(&ship(&sheet, &spec)),
            [
                (SHEET, 1000),
                (SHEET, 1100),
                (SHEET, 1200),
                (SHEET, 1930),
                (RUNS, 1930),
                (SHEET, 1400),
                (SHEET, 1500),
            ]
        );
    }

    #[test]
    fn a_layer_not_drawn_or_held_to_none_draws_no_overlay() {
        let sheet = sheet(&[1000, 1930, 1400]);
        let spec = FrameSpec::NONE;
        let unlit = ShipDraw {
            hull: false,
            weapon: None,
            ..ship(&sheet, &spec)
        };
        let images = drawn(&unlit);
        assert!(!images.contains(&(RUNS, 1000)), "{images:?}");
        assert!(!images.contains(&(RUNS, 1930)), "{images:?}");
        // A murk of 22 holds the layers to none.
        let held_out = ShipDraw {
            colorize: Some(Colorize::toward_pixel(22, 0)),
            ..ship(&sheet, &spec)
        };
        let images = drawn(&held_out);
        assert!(images.contains(&(SHEET, 1930)), "{images:?}");
        assert!(!images.contains(&(RUNS, 1930)), "{images:?}");
        // A shield flash at level none.
        let shielded = self::sheet(&[1500]);
        let faded = ShipDraw {
            shield: Some(0),
            ..ship(&shielded, &spec)
        };
        let images = drawn(&faded);
        assert!(images.contains(&(SHEET, 1500)), "{images:?}");
        assert!(!images.contains(&(RUNS, 1500)), "{images:?}");
    }

    #[test]
    fn the_overlay_is_on_the_layers_own_frame() {
        let sheet = sheet(&[1930]);
        let spec = FrameSpec::NONE;
        let turned = ShipDraw {
            heading: 130.0,
            ..ship(&sheet, &spec)
        };
        let mut list = DrawList::new();
        turned.draw(&mut list, &|_| AT, None);
        let frames: Vec<(ImageKind, u16)> = list
            .iter()
            .filter_map(|command| match *command {
                DrawCommand::Sprite { image, .. } | DrawCommand::Colorized { image, .. }
                    if image.id == 1930 =>
                {
                    Some((image.kind, image.frame))
                }
                _ => None,
            })
            .collect();
        let frame = turned.frame();
        assert!(frame > 0);
        assert_eq!(frames, [(SHEET, frame), (RUNS, frame)]);
    }

    /// The solid-run commands `ship` draws with alt layer `alt`, the hull
    /// turned so its frame isn't 0: each one's image and centre.
    fn alt_runs(alt: AltLayer) -> Vec<(ImageKey, Point)> {
        let sheet = sheet(&[1400]);
        let spec = FrameSpec::NONE;
        let turned = ShipDraw {
            heading: 130.0,
            ..ship(&sheet, &spec)
        };
        assert!(turned.frame() > 0);
        let mut list = DrawList::new();
        turned.draw(&mut list, &|_| AT, Some(alt));
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Sprite { image, center, .. } if image.kind == RUNS => {
                    Some((image, center))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn an_alt_layers_runs_are_drawn_at_its_own_place_and_frame() {
        let held = AltLayer {
            image: ImageKey::sprite(1400, 7),
            at: Point::new(55.0, 12.0),
            tint: Color::WHITE,
            colorize: None,
            blend: crate::Blend::Normal,
        };
        assert_eq!(alt_runs(held), [(ImageKey::solid_runs(1400, 7), held.at)]);
    }

    #[test]
    fn an_alt_layer_of_another_sheet_draws_no_runs() {
        let other = AltLayer {
            image: ImageKey::sprite(1401, 7),
            at: Point::new(55.0, 12.0),
            tint: Color::WHITE,
            colorize: None,
            blend: crate::Blend::Normal,
        };
        assert_eq!(alt_runs(other), []);
    }
}
