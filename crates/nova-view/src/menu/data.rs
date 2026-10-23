//! The main screen's port over the game data ([`GameData`]): `cölr` 128,
//! `spïn` 600 to 610 and what they name, decoded only as far as the core
//! needs (the sizes, and the buttons' frame-0 masks), and the pilot
//! panel's ([`PilotPanelSource`]): `cölr` 128's menu font and colours,
//! `STR#` 2002's labels, each `shïp`'s Subtitle, and the `shän` base
//! images and `PICT` sizes [`target_picture`] picks its picture from,
//! and the intro screen's ([`IntroSource`]): the first `chär`'s intro
//! fields, `PICT` sizes, and whether a `snd ` exists. Thin lookups; the
//! decisions are the core's.

use nova_data::GameData;
use nova_data::graphics::{Image, PICT, RLED, SpriteSheet, decode_pict, decode_rled};
use nova_data::records::character::Character;
use nova_data::records::colors::Colors;
use nova_data::records::ship::Ship;
use nova_data::records::spin::Spin;
use nova_sim::catalog::{CommCatalog, ShipId};
use nova_sim::news::MISC_STRINGS;

use super::art::{
    BACKGROUND_PICT, BUTTON_SPINS, ButtonArt, COLORS_ID, LOGO_SPIN, MainScreenArt,
    MainScreenSource, PictFrames, ROLLOVER_SPIN, RolloverArt, SLIDE_SPINS, SpriteMask,
};
use super::intro::{CharacterIntro, IntroPicture, IntroSource};
use super::pilot_panel::{PanelLabels, PanelLooks, PanelShip, PilotPanelSource, target_picture};
use crate::color::Color;
use crate::font::Font;
use crate::geometry::Point;
use crate::sound::SoundId;
use crate::spaceport::ShipBaseImages;

/// The `snd ` resource type.
const SND: nova_rsrc::ResType = nova_rsrc::ResType::new(*b"snd ");

/// Reads afresh on every call; the router reads the first `chär`'s intro
/// as New Pilot makes a pilot and as play is entered.
impl IntroSource for GameData {
    fn first_character_intro(&self) -> Option<CharacterIntro> {
        let (_, first) = self.records::<Character>().next()?;
        let character = &first.ok()?.record;
        Some(CharacterIntro {
            pictures: std::array::from_fn(|slot| IntroPicture {
                picture: character.intro_pict[slot].map_or(-1, |id| id.0),
                delay: character.pict_delay[slot],
            }),
            text: character.intro_text_id.map(|id| id.0),
        })
    }

    fn intro_picture_size(&self, picture: i16) -> Option<(u32, u32)> {
        picture_size(self, picture)
    }

    fn has_sound(&self, sound: SoundId) -> bool {
        self.resource(SND, sound.0).is_some()
    }
}

/// Reads afresh on every call; the router reads the looks once and a
/// ship's picture when the pilot's ship type changes.
impl PilotPanelSource for GameData {
    fn pilot_panel_looks(&self) -> PanelLooks {
        let labels = PanelLabels::from_list(&self.string_list(MISC_STRINGS));
        let Some(Ok(colors)) = self.get::<Colors>(COLORS_ID) else {
            return PanelLooks::stock(labels);
        };
        let colors = &colors.record;
        PanelLooks {
            font: Font::named(colors.menu_font.as_str()),
            size: f32::from(colors.menu_font_size),
            value: Color::from_rgb24(colors.menu_color1),
            label: Color::from_rgb24(colors.menu_color2),
            labels,
        }
    }

    fn panel_ship(&self, ship: ShipId) -> PanelShip {
        let Some(Ok(entry)) = self.get::<Ship>(ship.0) else {
            return PanelShip::default();
        };
        PanelShip {
            subtitle: entry.record.subtitle.as_str().to_owned(),
            picture: target_picture(ship, &ship_bases(self), |id| picture_size(self, id)),
        }
    }
}

/// Each `shän`'s `BaseImage` whose `shïp` decodes.
fn ship_bases(data: &GameData) -> Vec<(ShipId, i16)> {
    let mut bases = data.ship_base_images();
    bases.retain(|(id, _)| matches!(data.get::<Ship>(id.0), Some(Ok(_))));
    bases
}

/// `PICT` `id`'s width and height, if it exists and decodes.
fn picture_size(data: &GameData, id: i16) -> Option<(u32, u32)> {
    crate::ui::PictureSizes::picture_size(data, id)
}

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
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::records::string_list::StrList;
    use nova_rsrc::ResType;

    use super::*;
    use crate::menu::fixture::{
        BACKGROUND_SIZE, BUTTON_POSITIONS, BUTTON_SIZE, BUTTON_SPRITES, CLEAR_CORNER, LOGO_FRAME,
        LOGO_FRAMES, LOGO_PICT, LOGO_POSITION, ROLLOVER_FRAMES, ROLLOVER_POSITION, ROLLOVER_SIZE,
        ROLLOVER_SPRITE, SLIDE_FRAME, SLIDE_FRAMES, SLIDE_PICTS, SLIDE_POSITIONS,
        game_data as game, main_screen_resources,
    };
    use crate::menu::pilot_panel::ShipPicture;

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
    fn the_fixtures_panel_looks_are_the_stock_ones() {
        let looks = crate::menu::fixture::main_screen_data().pilot_panel_looks();
        assert_eq!(looks, PanelLooks::stock(PanelLabels::default()));
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

    // The pilot panel's port.

    /// A `cölr` whose menu font is `font` at `size`, its colours
    /// `values` and `labels`.
    fn menu_colors(font: &[u8], size: i16, values: u32, labels: u32) -> Vec<u8> {
        let mut bytes = vec![0; Colors::SIZE.expect("fixed")];
        bytes[0x0C..0x0C + font.len()].copy_from_slice(font);
        bytes[0x4C..0x4E].copy_from_slice(&size.to_be_bytes());
        bytes[0x4E..0x52].copy_from_slice(&values.to_be_bytes());
        bytes[0x52..0x56].copy_from_slice(&labels.to_be_bytes());
        bytes
    }

    /// A `STR#` of `strings`.
    fn str_list(strings: &[String]) -> Vec<u8> {
        let mut bytes = u16::try_from(strings.len())
            .expect("few")
            .to_be_bytes()
            .to_vec();
        for string in strings {
            bytes.push(u8::try_from(string.len()).expect("short"));
            bytes.extend(string.as_bytes());
        }
        bytes
    }

    /// `STR#` 2002 up to #279, each label "#n" and the rest empty.
    fn labels_list() -> Vec<u8> {
        let mut strings = vec![String::new(); 279];
        for n in [251, 252, 254, 255, 256, 276, 277, 278, 279] {
            strings[n - 1] = format!("#{n}");
        }
        str_list(&strings)
    }

    #[test]
    fn the_panels_looks_are_colr_128s_menu_font_and_colours_and_str_2002s_labels() {
        let data = game(&[
            (
                Colors::TYPE,
                128,
                menu_colors(b"Charcoal", 12, 0x0011_2233, 0x0044_5566),
            ),
            (StrList::TYPE, 2002, labels_list()),
        ]);
        let looks = data.pilot_panel_looks();
        assert_eq!((looks.font, looks.size), (Font::Charcoal, 12.0));
        assert_eq!(looks.value, Color::from_rgb24(0x0011_2233));
        assert_eq!(looks.label, Color::from_rgb24(0x0044_5566));
        assert_eq!(looks.labels.pilot_name, "#251");
        assert_eq!(looks.labels.date, "#252");
        assert_eq!(looks.labels.combat_rating, "#254");
        assert_eq!(looks.labels.ship_name, "#255");
        assert_eq!(looks.labels.ship_class, "#256");
        assert_eq!(looks.labels.no_pilot, "#276");
        assert_eq!(looks.labels.killed, "#277");
        assert_eq!(looks.labels.legal_status, "#278");
        assert_eq!(looks.labels.current_system, "#279");
        let geneva = game(&[(Colors::TYPE, 128, menu_colors(b"Geneva", 9, 0, 0))]);
        assert_eq!(geneva.pilot_panel_looks().font, Font::Geneva);
    }

    #[test]
    fn without_colr_128_the_panel_takes_the_stock_looks_and_its_labels_still_read() {
        let data = game(&[(StrList::TYPE, 2002, labels_list())]);
        let looks = data.pilot_panel_looks();
        assert_eq!(looks, PanelLooks::stock(looks.labels.clone()));
        assert_eq!(looks.labels.no_pilot, "#276");
        let broken = game(&[(Colors::TYPE, 128, vec![0; 3])]);
        assert_eq!(
            broken.pilot_panel_looks(),
            PanelLooks::stock(PanelLabels::default()),
            "undecodable, and no labels"
        );
    }

    /// A `shïp` subtitled `subtitle`.
    fn ship_record(subtitle: &str) -> Vec<u8> {
        let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
        bytes[0x6E6..0x6E6 + subtitle.len()].copy_from_slice(subtitle.as_bytes());
        bytes
    }

    /// A `shän` whose `BaseImage` is `base`.
    fn anim(base: i16) -> Vec<u8> {
        let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
        bytes[0..2].copy_from_slice(&base.to_be_bytes());
        bytes
    }

    /// A blank `PICT` `width` x `height`.
    fn blank(width: i16, height: i16) -> Vec<u8> {
        nova_data::graphics::fixture::PictBuilder::new([0, 0, height, width])
            .end()
            .build()
    }

    fn shown(picture: i16, width: u32, height: u32) -> ShipPicture {
        ShipPicture {
            picture,
            width,
            height,
        }
    }

    #[test]
    fn a_ships_subtitle_and_its_own_target_picture() {
        let data = game(&[
            (Ship::TYPE, 130, ship_record("Light Transport")),
            (ShipAnim::TYPE, 130, anim(1000)),
            (PICT, 3002, blank(41, 20)),
        ]);
        assert_eq!(
            data.panel_ship(ShipId(130)),
            PanelShip {
                subtitle: "Light Transport".to_owned(),
                picture: Some(shown(3002, 41, 20)),
            }
        );
        assert_eq!(
            data.panel_ship(ShipId(131)),
            PanelShip::default(),
            "no shïp"
        );
    }

    #[test]
    fn a_ship_without_a_picture_takes_its_base_ships() {
        let data = game(&[
            (Ship::TYPE, 129, ship_record("")),
            (ShipAnim::TYPE, 129, anim(1000)),
            (PICT, 3001, blank(6, 3)),
            (Ship::TYPE, 131, ship_record("Variant")),
            (ShipAnim::TYPE, 131, anim(1000)),
        ]);
        assert_eq!(
            data.panel_ship(ShipId(131)),
            PanelShip {
                subtitle: "Variant".to_owned(),
                picture: Some(shown(3001, 6, 3)),
            },
            "129's picture, its own subtitle"
        );
    }

    #[test]
    fn a_ship_with_no_shan_or_no_picture_anywhere_has_none() {
        let data = game(&[
            (Ship::TYPE, 128, ship_record("")),
            (ShipAnim::TYPE, 128, anim(1000)),
            (Ship::TYPE, 129, ship_record("")),
            (ShipAnim::TYPE, 129, anim(1000)),
            (Ship::TYPE, 130, ship_record("Bare")),
            (PICT, 3002, blank(4, 4)),
            // 131's picture does not decode.
            (Ship::TYPE, 131, ship_record("")),
            (ShipAnim::TYPE, 131, anim(1300)),
            (PICT, 3003, vec![0, 4]),
            // 134 shares 133's base image, but 133's shïp does not decode.
            (Ship::TYPE, 133, vec![0; 3]),
            (ShipAnim::TYPE, 133, anim(1400)),
            (PICT, 3005, blank(4, 4)),
            (Ship::TYPE, 134, ship_record("")),
            (ShipAnim::TYPE, 134, anim(1400)),
        ]);
        assert_eq!(
            data.panel_ship(ShipId(129)).picture,
            None,
            "no base picture"
        );
        assert_eq!(
            data.panel_ship(ShipId(130)),
            PanelShip {
                subtitle: "Bare".to_owned(),
                picture: None
            },
            "no shän, though PICT 3002 exists"
        );
        assert_eq!(data.panel_ship(ShipId(131)).picture, None, "undecodable");
        assert_eq!(
            data.panel_ship(ShipId(134)).picture,
            None,
            "133's shïp does not decode, so it is no base"
        );
        assert_eq!(
            data.panel_ship(ShipId(127)),
            PanelShip::default(),
            "below the first shïp"
        );
    }

    /// A `chär` whose intro is `pictures` (each its `PICT` and delay) and
    /// `text`.
    fn character(pictures: [(i16, i16); 4], text: i16) -> Vec<u8> {
        let mut bytes = vec![0; Character::SIZE.expect("fixed")];
        for (slot, (picture, delay)) in pictures.into_iter().enumerate() {
            let at = 0x20 + 2 * slot;
            bytes[at..at + 2].copy_from_slice(&picture.to_be_bytes());
            let at = 0x28 + 2 * slot;
            bytes[at..at + 2].copy_from_slice(&delay.to_be_bytes());
        }
        bytes[0x30..0x32].copy_from_slice(&text.to_be_bytes());
        bytes
    }

    #[test]
    fn the_intro_is_the_first_chars_by_ascending_id() {
        let data = game(&[
            (
                Character::TYPE,
                129,
                character([(9000, 1), (-1, 0), (-1, 0), (-1, 0)], 300),
            ),
            (
                Character::TYPE,
                128,
                character([(8200, 45), (-1, 0), (8201, -1), (0, 5)], 128),
            ),
        ]);
        assert_eq!(
            data.first_character_intro(),
            Some(CharacterIntro {
                pictures: [
                    IntroPicture {
                        picture: 8200,
                        delay: 45
                    },
                    IntroPicture {
                        picture: -1,
                        delay: 0
                    },
                    IntroPicture {
                        picture: 8201,
                        delay: -1
                    },
                    IntroPicture {
                        picture: 0,
                        delay: 5
                    },
                ],
                text: Some(128),
            })
        );
    }

    #[test]
    fn a_text_of_minus_1_is_none_and_no_char_no_intro() {
        let data = game(&[(
            Character::TYPE,
            128,
            character([(-1, 0), (0, 0), (-1, 0), (-1, 0)], -1),
        )]);
        let intro = data.first_character_intro().expect("a chär");
        assert_eq!(intro.text, None);
        assert_eq!(
            intro.pictures.map(|picture| picture.picture),
            [-1, 0, -1, -1]
        );
        let zero = game(&[(Character::TYPE, 128, character([(-1, 0); 4], 0))]);
        assert_eq!(
            zero.first_character_intro().map(|intro| intro.text),
            Some(Some(0)),
            "0 is a text, as only -1 is none"
        );
        assert_eq!(game(&[]).first_character_intro(), None);
        let broken = game(&[(Character::TYPE, 128, vec![0; 3])]);
        assert_eq!(broken.first_character_intro(), None, "undecodable");
    }

    #[test]
    fn the_intro_reads_picture_sizes_and_whether_a_sound_exists() {
        let data = game(&[
            (PICT, 8200, blank(64, 48)),
            (PICT, 8201, vec![0, 4]),
            (SND, 30_003, vec![0; 8]),
        ]);
        assert_eq!(data.intro_picture_size(8200), Some((64, 48)));
        assert_eq!(data.intro_picture_size(8201), None, "undecodable");
        assert_eq!(data.intro_picture_size(8202), None, "missing");
        assert!(data.has_sound(SoundId(30_003)));
        assert!(!data.has_sound(SoundId(30_004)));
        assert!(!game(&[]).has_sound(SoundId(30_003)));
    }
}
