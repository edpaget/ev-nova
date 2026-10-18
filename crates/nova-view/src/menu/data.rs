//! The main screen's port over the game data ([`GameData`]): `cölr` 128,
//! `spïn` 600 to 610 and what they name, decoded only as far as the core
//! needs (the sizes, and the buttons' frame-0 masks). Thin lookups; the
//! decisions are the core's.

use nova_data::GameData;
use nova_data::graphics::{Image, PICT, RLED, SpriteSheet, decode_pict, decode_rled};
use nova_data::records::colors::Colors;
use nova_data::records::spin::Spin;

use super::art::{
    BACKGROUND_PICT, BUTTON_SPINS, ButtonArt, COLORS_ID, LOGO_SPIN, MainScreenArt,
    MainScreenSource, PictFrames, ROLLOVER_SPIN, RolloverArt, SLIDE_SPINS, SpriteMask,
};
use crate::geometry::Point;

impl MainScreenSource for GameData {
    fn main_screen_art(&self) -> Result<MainScreenArt, String> {
        let colors = match self.get::<Colors>(COLORS_ID) {
            None => return Err(format!("no cölr {COLORS_ID}")),
            Some(Err(err)) => return Err(format!("cölr {COLORS_ID}: {err}")),
            Some(Ok(colors)) => colors.record.clone(),
        };
        let point = |p: nova_data::Point| Point::new(f32::from(p.x), f32::from(p.y));
        let background = picture(self, BACKGROUND_PICT)?;
        let mut buttons = Vec::with_capacity(BUTTON_SPINS.len());
        for (spin_id, position) in BUTTON_SPINS.into_iter().zip(colors.button) {
            let spin = spin(self, spin_id)?;
            let sheet = sheet(self, spin_id, &spin)?;
            // A decoded sheet always has a frame.
            let [frame, ..] = sheet.frames() else {
                unreachable!("rlëD {} decoded with no frames", spin.sprites_id)
            };
            buttons.push(ButtonArt {
                sprite: spin.sprites_id,
                position: point(position),
                mask: SpriteMask::from_alpha(frame),
            });
        }
        let buttons: [ButtonArt; 6] = buttons.try_into().map_err(|_| "six buttons".to_owned())?;
        let logo = pict_frames(self, LOGO_SPIN, point(colors.logo))?;
        let mut slides = Vec::with_capacity(SLIDE_SPINS.len());
        for (spin_id, position) in SLIDE_SPINS.into_iter().zip(colors.slide) {
            slides.push(pict_frames(self, spin_id, point(position))?);
        }
        let slides: [PictFrames; 3] = slides.try_into().map_err(|_| "three slides".to_owned())?;
        let rollover_spin = spin(self, ROLLOVER_SPIN)?;
        let rollover = sheet(self, ROLLOVER_SPIN, &rollover_spin)?;
        Ok(MainScreenArt {
            background: BACKGROUND_PICT,
            background_size: (background.width(), background.height()),
            buttons,
            logo,
            slides,
            rollover: RolloverArt {
                sprite: rollover_spin.sprites_id,
                frame_size: (
                    rollover.frame_width() as f32,
                    rollover.frame_height() as f32,
                ),
                frames: u16::try_from(rollover.frames().len()).unwrap_or(u16::MAX),
                position: point(colors.rollover),
            },
        })
    }
}

/// The `PICT` of frames `spïn` `id` names, decoded to check it, its
/// frames placed at `position`: the frame size from the `spïn`, and the
/// frame count from its tiles, as `_GetSpriteInfo` reads them.
fn pict_frames(data: &GameData, id: i16, position: Point) -> Result<PictFrames, String> {
    let spin = spin(data, id)?;
    let found = data
        .resource(PICT, spin.sprites_id)
        .ok_or_else(|| format!("spïn {id} names PICT {}, which is missing", spin.sprites_id))?;
    decode_pict(found.resource.data()).map_err(|err| format!("PICT {}: {err}", spin.sprites_id))?;
    let tiles = |n: i16| u16::try_from(n).unwrap_or(0).max(1);
    let (columns, rows) = (tiles(spin.x_tiles), tiles(spin.y_tiles));
    Ok(PictFrames {
        picture: spin.sprites_id,
        frame_size: (f32::from(spin.x_size), f32::from(spin.y_size)),
        columns,
        frames: columns.saturating_mul(rows),
        position,
    })
}

/// `spïn` `id`.
fn spin(data: &GameData, id: i16) -> Result<Spin, String> {
    match data.get::<Spin>(id) {
        None => Err(format!("no spïn {id}")),
        Some(Err(err)) => Err(format!("spïn {id}: {err}")),
        Some(Ok(spin)) => Ok(spin.record.clone()),
    }
}

/// The `rlëD` `spin` (`spïn` `id`) names, decoded.
fn sheet(data: &GameData, id: i16, spin: &Spin) -> Result<SpriteSheet, String> {
    let sprite = spin.sprites_id;
    let found = data
        .resource(RLED, sprite)
        .ok_or_else(|| format!("spïn {id} names rlëD {sprite}, which is missing"))?;
    decode_rled(found.resource.data(), spin.sheet_layout())
        .map_err(|err| format!("rlëD {sprite}: {err}"))
}

/// `PICT` `id`, decoded.
fn picture(data: &GameData, id: i16) -> Result<Image, String> {
    let found = data
        .resource(PICT, id)
        .ok_or_else(|| format!("no PICT {id}"))?;
    decode_pict(found.resource.data()).map_err(|err| format!("PICT {id}: {err}"))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_data::Record;
    use nova_rsrc::ResType;

    use super::*;
    use crate::menu::fixture::{
        BACKGROUND_SIZE, BUTTON_POSITIONS, BUTTON_SIZE, BUTTON_SPRITES, CLEAR_CORNER, LOGO_FRAME,
        LOGO_FRAMES, LOGO_PICT, LOGO_POSITION, ROLLOVER_FRAMES, ROLLOVER_POSITION, ROLLOVER_SIZE,
        ROLLOVER_SPRITE, SLIDE_FRAME, SLIDE_FRAMES, SLIDE_PICTS, SLIDE_POSITIONS,
        game_data as game, main_screen_resources,
    };

    /// The fixture's resources without (`ty`, `id`).
    fn without(ty: ResType, id: i16) -> GameData {
        let resources: Vec<_> = main_screen_resources()
            .into_iter()
            .filter(|(t, i, _)| (*t, *i) != (ty, id))
            .collect();
        game(&resources)
    }

    /// The fixture's resources with (`ty`, `id`) replaced by `data`.
    fn replacing(ty: ResType, id: i16, data: &[u8]) -> GameData {
        let resources: Vec<_> = main_screen_resources()
            .into_iter()
            .map(|(t, i, bytes)| {
                if (t, i) == (ty, id) {
                    (t, i, data.to_vec())
                } else {
                    (t, i, bytes)
                }
            })
            .collect();
        game(&resources)
    }

    fn point((x, y): (i16, i16)) -> Point {
        Point::new(f32::from(x), f32::from(y))
    }

    #[test]
    fn the_art_resolves_from_colr_128_and_the_spins() {
        let art = game(&main_screen_resources())
            .main_screen_art()
            .expect("resolves");
        assert_eq!(art.background, 8000);
        assert_eq!(
            art.background_size,
            (u32::from(BACKGROUND_SIZE.0), u32::from(BACKGROUND_SIZE.1))
        );
        let sprites: Vec<i16> = art.buttons.iter().map(|button| button.sprite).collect();
        assert_eq!(sprites, BUTTON_SPRITES);
        let positions: Vec<Point> = art.buttons.iter().map(|button| button.position).collect();
        assert_eq!(positions, BUTTON_POSITIONS.map(point));
        assert_eq!(
            art.logo,
            PictFrames {
                picture: LOGO_PICT,
                frame_size: (f32::from(LOGO_FRAME.0), f32::from(LOGO_FRAME.1)),
                columns: 1,
                frames: LOGO_FRAMES,
                position: point(LOGO_POSITION),
            }
        );
        assert_eq!(
            art.rollover,
            RolloverArt {
                sprite: ROLLOVER_SPRITE,
                frame_size: (f32::from(ROLLOVER_SIZE.0), f32::from(ROLLOVER_SIZE.1)),
                frames: ROLLOVER_FRAMES,
                position: point(ROLLOVER_POSITION),
            }
        );
    }

    #[test]
    fn the_slides_resolve_from_colr_128_and_spins_608_to_610() {
        let art = game(&main_screen_resources())
            .main_screen_art()
            .expect("resolves");
        let expected = [0, 1, 2].map(|slide| PictFrames {
            picture: SLIDE_PICTS[slide],
            frame_size: (f32::from(SLIDE_FRAME.0), f32::from(SLIDE_FRAME.1)),
            columns: 1,
            frames: SLIDE_FRAMES[slide],
            position: point(SLIDE_POSITIONS[slide]),
        });
        assert_eq!(art.slides, expected);
    }

    #[test]
    fn the_fixtures_slides_are_strips_of_their_frames() {
        let data = crate::menu::fixture::main_screen_data();
        for (id, frames) in SLIDE_PICTS.into_iter().zip(SLIDE_FRAMES) {
            let strip = picture(&data, id).expect("decodes");
            assert_eq!(
                (strip.width(), strip.height()),
                (u32::from(SLIDE_FRAME.0), u32::from(SLIDE_FRAME.1 * frames)),
                "{id}"
            );
        }
    }

    #[test]
    fn the_fixtures_logo_is_a_strip_of_its_frames() {
        let data = crate::menu::fixture::main_screen_data();
        let logo = picture(&data, LOGO_PICT).expect("decodes");
        assert_eq!(
            (logo.width(), logo.height()),
            (
                u32::from(LOGO_FRAME.0),
                u32::from(LOGO_FRAME.1 * LOGO_FRAMES)
            )
        );
    }

    #[test]
    fn the_fixtures_button_style_is_the_stock_one() {
        use crate::ui::{ButtonStyle, DescriptionSource};
        assert_eq!(
            crate::menu::fixture::main_screen_data().button_style(),
            ButtonStyle::STOCK
        );
    }

    #[test]
    fn the_masks_follow_frame_0s_alpha() {
        let art = game(&main_screen_resources())
            .main_screen_art()
            .expect("resolves");
        for button in &art.buttons {
            let mask = &button.mask;
            assert_eq!(
                (mask.width(), mask.height()),
                (u32::from(BUTTON_SIZE.0), u32::from(BUTTON_SIZE.1))
            );
            let corner = (i64::from(CLEAR_CORNER.0), i64::from(CLEAR_CORNER.1));
            assert!(!mask.opaque_at(corner.0, corner.1), "a clear corner");
            assert!(!mask.opaque_at(3, 2), "the opposite corner");
            assert!(mask.opaque_at(2, 1), "the middle");
            assert!(mask.opaque_at(1, 0), "beside the corner");
        }
    }

    #[test]
    fn what_is_missing_is_named() {
        let cases = [
            (without(Colors::TYPE, 128), "no cölr 128"),
            (without(PICT, 8000), "no PICT 8000"),
            (without(Spin::TYPE, 603), "no spïn 603"),
            (
                without(RLED, 8053),
                "spïn 603 names rlëD 8053, which is missing",
            ),
            (without(Spin::TYPE, 606), "no spïn 606"),
            (
                without(PICT, LOGO_PICT),
                "spïn 606 names PICT 8010, which is missing",
            ),
            (without(Spin::TYPE, 607), "no spïn 607"),
            (
                without(RLED, ROLLOVER_SPRITE),
                "spïn 607 names rlëD 8020, which is missing",
            ),
            (without(Spin::TYPE, 608), "no spïn 608"),
            (without(Spin::TYPE, 610), "no spïn 610"),
            (
                without(PICT, SLIDE_PICTS[1]),
                "spïn 609 names PICT 8031, which is missing",
            ),
        ];
        for (data, expected) in cases {
            assert_eq!(data.main_screen_art(), Err(expected.to_owned()));
        }
    }

    #[test]
    fn what_does_not_decode_says_why() {
        let broken = replacing(RLED, 8053, &[0, 4]);
        let error = broken.main_screen_art().expect_err("undecodable");
        assert!(error.starts_with("rlëD 8053: "), "{error}");
        let broken = replacing(PICT, 8000, &[0, 4]);
        let error = broken.main_screen_art().expect_err("undecodable");
        assert!(error.starts_with("PICT 8000: "), "{error}");
        let broken = replacing(PICT, LOGO_PICT, &[0, 4]);
        let error = broken.main_screen_art().expect_err("undecodable");
        assert!(error.starts_with("PICT 8010: "), "{error}");
        let broken = replacing(PICT, SLIDE_PICTS[2], &[0, 4]);
        let error = broken.main_screen_art().expect_err("undecodable");
        assert!(error.starts_with("PICT 8032: "), "{error}");
        let broken = replacing(Spin::TYPE, 609, &[0, 4]);
        let error = broken.main_screen_art().expect_err("undecodable");
        assert!(error.starts_with("spïn 609: "), "{error}");
        let broken = replacing(Spin::TYPE, 600, &[0, 4]);
        let error = broken.main_screen_art().expect_err("undecodable");
        assert!(error.starts_with("spïn 600: "), "{error}");
        let broken = replacing(Colors::TYPE, 128, &[0, 4]);
        let error = broken.main_screen_art().expect_err("undecodable");
        assert!(error.starts_with("cölr 128: "), "{error}");
    }

    #[test]
    fn a_sheet_with_no_frames_is_refused() {
        let empty: Vec<u8> = [4_u16, 3, 16, 0, 0, 0, 0, 0]
            .iter()
            .flat_map(|field| field.to_be_bytes())
            .collect();
        let broken = replacing(RLED, 8050, &empty);
        assert_eq!(
            broken.main_screen_art(),
            Err("rlëD 8050: the rlëD sheet has no frames".to_owned())
        );
    }

    #[test]
    fn a_logo_spin_with_no_tiles_is_one_frame_across() {
        let mut spin = Vec::new();
        for field in [LOGO_PICT, -1, 2, 1, 0, -3] {
            spin.extend(field.to_be_bytes());
        }
        let art = replacing(Spin::TYPE, 606, &spin)
            .main_screen_art()
            .expect("resolves");
        assert_eq!((art.logo.columns, art.logo.frames), (1, 1));
        let mut spin = Vec::new();
        for field in [LOGO_PICT, -1, 2, 1, 2, 3] {
            spin.extend(field.to_be_bytes());
        }
        let art = replacing(Spin::TYPE, 606, &spin)
            .main_screen_art()
            .expect("resolves");
        assert_eq!((art.logo.columns, art.logo.frames), (2, 6));
    }
}
