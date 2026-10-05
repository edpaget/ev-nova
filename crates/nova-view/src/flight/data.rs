//! The flight screen's ports over the game data: thin mappings from
//! `GameData`'s ship sprite lookup to [`ShipSheet`], from its `gövt`,
//! `ïntf` and `PICT` resources to the status bars, and from its `wëap`,
//! `bööm`, `spïn`, `shïp` and `gövt` resources to the combat looks.

use std::num::NonZeroU16;

use nova_data::GameData;
use nova_data::graphics::{PICT, decode_pict};
use nova_data::records::boom::Boom;
use nova_data::records::govt::Govt;
use nova_data::records::interface::Interface;
use nova_data::records::ship::Ship;
use nova_data::records::weapon::Weapon;
use nova_data::sound::decode_snd;
use nova_rsrc::ResType;
use nova_sim::TICKS_PER_SECOND;

use super::catalog::{
    BoomId, BoomLook, CombatLooks, EffectSheet, GovtId, ShipId, ShipSheet, ShipSprites, SoundId,
    StatusBarLayout, StatusBars, TargetCard, WeaponId, WeaponLook,
};
use crate::color::Color;
use crate::font::Font;
use crate::geometry::{Bounds, Point};

/// Decodes the sheet on every call; the flight screen asks once, when it
/// opens.
impl ShipSprites for GameData {
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
        let sprite = self.ship_sprite(id).map_err(|err| err.to_string())?;
        let sheet = &sprite.sheet;
        // The sheet's layout is the `shän`'s frames per rotation as columns.
        let rotations = NonZeroU16::new(sheet.layout().columns())
            .expect("a sheet layout has at least one column");
        Ok(ShipSheet {
            image_id: sprite.image_id,
            rotations,
            frame_width: sheet.frame_width(),
            frame_height: sheet.frame_height(),
        })
    }
}

/// Decodes on every call; the flight screen asks once, when it opens.
impl StatusBars for GameData {
    fn government_interface(&self, id: GovtId) -> Result<i16, String> {
        match self.get::<Govt>(id.0) {
            Some(Ok(govt)) => Ok(govt.record.interface),
            Some(Err(err)) => Err(err.to_string()),
            None => Err(format!("no gövt {}", id.0)),
        }
    }

    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
        let intf = match self.get::<Interface>(id) {
            Some(Ok(intf)) => intf.record,
            Some(Err(err)) => return Err(err.to_string()),
            None => return Err(format!("no ïntf {id}")),
        };
        Ok(StatusBarLayout {
            radar: bounds(intf.radar_area),
            shield: bounds(intf.shield_area),
            armor: bounds(intf.armor_area),
            fuel: bounds(intf.fuel_area),
            nav: bounds(intf.nav_area),
            weap: bounds(intf.weap_area),
            targ: bounds(intf.targ_area),
            bright_text: Color::from_rgb24(intf.bright_text),
            dim_text: Color::from_rgb24(intf.dim_text),
            bright_radar: Color::from_rgb24(intf.bright_radar),
            dim_radar: Color::from_rgb24(intf.dim_radar),
            shield_color: Color::from_rgb24(intf.shield_color),
            armor_color: Color::from_rgb24(intf.armor_color),
            fuel_full: Color::from_rgb24(intf.fuel_full),
            fuel_partial: Color::from_rgb24(intf.fuel_partial),
            font: Font::named(intf.status_font.as_str()),
            font_size: f32::from(intf.stat_font_size),
            subtitle_size: f32::from(intf.subtitle_size),
            status_bkgnd: intf.status_bkgnd,
        })
    }

    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        let found = self.resource(PICT, id)?;
        let picture = decode_pict(found.resource.data()).ok()?;
        Some((picture.width(), picture.height()))
    }
}

/// The first `spïn` of the shots' graphics (the Bible: 3000-3255).
pub const FIRST_SHOT_SPIN: i16 = 3000;
/// The first `snd ` of the weapons' sounds (the loader's 200-455).
pub const FIRST_WEAPON_SOUND: i16 = 200;
/// The first `spïn` of the explosions' graphics (`_LoadObjectData`).
pub const FIRST_BOOM_SPIN: i16 = 400;
/// The first `snd ` of the explosions' sounds (`_LoadSounds`: 300-363).
pub const FIRST_BOOM_SOUND: i16 = 300;
/// The first target picture, `PICT` 3000 for `shïp` 128
/// (`_LoadSprites`).
pub const FIRST_TARGET_PICTURE: i16 = 3000;
/// The first `shïp`.
const FIRST_SHIP: i16 = 128;
/// `FrameAdvance` per frame a tick (the double @0xdd098).
pub const FRAME_ADVANCE_UNIT: f32 = 0.01;

/// Decodes on every call; the flight screen asks once for each weapon and
/// explosion when it opens, and once for each ship type and government
/// its traffic brings.
impl CombatLooks for GameData {
    fn weapon_look(&self, id: WeaponId) -> Result<WeaponLook, String> {
        let entry = match self.get::<Weapon>(id.0) {
            Some(Ok(entry)) => entry,
            Some(Err(err)) => return Err(err.to_string()),
            None => return Err(format!("no wëap {}", id.0)),
        };
        let weapon = entry.record;
        let sheet = (weapon.graphic >= 0)
            .then(|| effect_sheet(self, FIRST_SHOT_SPIN.saturating_add(weapon.graphic)));
        let sound = offset_sound(FIRST_WEAPON_SOUND, weapon.sound);
        Ok(WeaponLook {
            name: resource_name(entry.name.unwrap_or_default()),
            sheet,
            sound,
            sound_ticks: sound.and_then(|sound| sound_ticks(self, sound)),
            flags: weapon.flags.bits(),
            flags2: weapon.flags2.bits(),
            flags3: weapon.flags3.bits(),
            beam_width: weapon.beam_width,
            falloff: weapon.falloff,
            beam_color: weapon.beam_color,
            corona_color: weapon.corona_color,
            prox_safety: weapon.prox_safety,
        })
    }

    fn boom_look(&self, id: BoomId) -> Option<Result<BoomLook, String>> {
        let boom = match self.get::<Boom>(id.0)? {
            Ok(entry) => entry.record,
            Err(err) => return Some(Err(err.to_string())),
        };
        Some(Ok(BoomLook {
            sheet: effect_sheet(self, FIRST_BOOM_SPIN.saturating_add(boom.graphic_index)),
            advance: f32::from(boom.frame_advance) * FRAME_ADVANCE_UNIT,
            sound: offset_sound(FIRST_BOOM_SOUND, boom.sound_index),
        }))
    }

    fn target_card(&self, ship: ShipId) -> TargetCard {
        let Some(Ok(entry)) = self.get::<Ship>(ship.0) else {
            return TargetCard::default();
        };
        let picture = FIRST_TARGET_PICTURE.saturating_add(ship.0 - FIRST_SHIP);
        TargetCard {
            subtitle: entry.record.subtitle.as_str().to_owned(),
            picture: self.resource(PICT, picture).map(|_| picture),
        }
    }

    fn target_code(&self, govt: GovtId) -> Option<String> {
        let Some(Ok(entry)) = self.get::<Govt>(govt.0) else {
            return None;
        };
        let code = entry.record.target_code.as_str();
        (!code.is_empty()).then(|| code.to_owned())
    }
}

/// `spïn` `id`'s sheet in `data`, as an effect's, or why it cannot be
/// read.
fn effect_sheet(data: &GameData, id: i16) -> Result<EffectSheet, String> {
    let spin = data.spin_sheet(id).map_err(|err| err.to_string())?;
    // The decoder refuses a sheet without frames.
    let count = u16::try_from(spin.frame_count()).unwrap_or(u16::MAX);
    let frames = NonZeroU16::new(count).unwrap_or(NonZeroU16::MIN);
    Ok(EffectSheet {
        image_id: spin.image_id,
        frames,
    })
}

/// A resource's name as the game shows it: up to any ';'.
fn resource_name(name: &str) -> String {
    name.split(';').next().unwrap_or_default().to_owned()
}

/// How long `snd ` `sound` lasts in `data`, in ticks rounded up: its
/// frames over its rate; none when it is missing or cannot be decoded
/// (which a rate of none cannot).
fn sound_ticks(data: &GameData, sound: SoundId) -> Option<u32> {
    let snd = data.resource(SND, sound.0)?;
    let pcm = decode_snd(snd.resource.data()).ok()?;
    let frames_x_ticks = pcm.frames() as f64 * f64::from(TICKS_PER_SECOND);
    Some((frames_x_ticks / pcm.sample_rate().hz()).ceil() as u32)
}

/// The `snd ` resource type.
const SND: ResType = ResType::new(*b"snd ");

/// The `snd ` `index` past `first`, or none for a negative index.
fn offset_sound(first: i16, index: i16) -> Option<SoundId> {
    (index >= 0).then(|| SoundId(first.saturating_add(index)))
}

fn bounds(rect: nova_data::Rect) -> Bounds {
    Bounds {
        min: Point::new(f32::from(rect.left), f32::from(rect.top)),
        max: Point::new(f32::from(rect.right), f32::from(rect.bottom)),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::io;
    use std::path::Path;

    use nova_data::Record;
    use nova_data::graphics::RLED;
    use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
    use nova_data::records::ship::Ship;
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};

    use super::*;

    /// One data file, `/data/Nova Data`, holding a fork.
    struct OneFile(Vec<u8>);

    impl DirLister for OneFile {
        fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
            Ok(vec![Listing {
                name: "Nova Data".into(),
                kind: EntryKind::File,
            }])
        }
    }

    impl ForkReader for OneFile {
        fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            Ok((fork == Fork::Data).then(|| self.0.clone()))
        }
    }

    fn store(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
        let fork = resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
                fork.resource(*ty, *id, None, data)
            })
            .build()
            .bytes;
        let file = OneFile(fork);
        GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
    }

    fn put_i16(bytes: &mut [u8], at: usize, value: i16) {
        bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
    }

    /// A `shän` with base image `base`, `sets` sets of `frames_per`
    /// frames.
    fn anim(base: i16, sets: i16, frames_per: i16) -> Vec<u8> {
        let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
        put_i16(&mut bytes, 0x00, base);
        put_i16(&mut bytes, 0x04, sets);
        put_i16(&mut bytes, 0x34, frames_per);
        bytes
    }

    /// An `rlëD` of `frames` 3 x 2 frames.
    fn sheet(frames: u16) -> Vec<u8> {
        (0..frames)
            .fold(RledBuilder::new(3, 2), |b, _| {
                b.frame(|f| f.line().pixels(&[0x7C00; 3]).line().pixels(&[0x7C00; 3]))
            })
            .build()
    }

    fn ship() -> Vec<u8> {
        vec![0; Ship::SIZE.expect("fixed")]
    }

    #[test]
    fn a_ships_sheet_has_its_image_rotations_and_frame_size() {
        // Two sets of 8 rotations: 16 frames, a turn every 8.
        let data = store(&[
            (Ship::TYPE, 128, ship()),
            (ShipAnim::TYPE, 128, anim(1000, 2, 8)),
            (RLED, 1000, sheet(16)),
        ]);
        assert_eq!(
            data.ship_sheet(ShipId(128)),
            Ok(ShipSheet {
                image_id: 1000,
                rotations: NonZeroU16::new(8).expect("non-zero"),
                frame_width: 3,
                frame_height: 2,
            })
        );
    }

    #[test]
    fn a_ship_whose_sprite_cannot_be_resolved_says_why() {
        let data = store(&[
            (Ship::TYPE, 128, ship()),
            (Ship::TYPE, 129, ship()),
            (ShipAnim::TYPE, 129, anim(1001, 1, 36)),
        ]);
        assert_eq!(
            data.ship_sheet(ShipId(128)),
            Err("no shän 128 for shïp 128".to_owned())
        );
        assert_eq!(
            data.ship_sheet(ShipId(129)),
            Err("shïp 129: no rlëD 1001 for its base image".to_owned())
        );
        assert_eq!(data.ship_sheet(ShipId(140)), Err("no shïp 140".to_owned()));
    }

    // Status bars.

    fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_be_bytes());
    }

    /// Writes `(top, left, bottom, right)` at `at`.
    fn put_rect(bytes: &mut [u8], at: usize, (top, left, bottom, right): (i16, i16, i16, i16)) {
        for (i, edge) in [top, left, bottom, right].into_iter().enumerate() {
            put_i16(bytes, at + 2 * i, edge);
        }
    }

    /// Stock `ïntf` 128's areas, distinct colours 1 to 8, `font` at
    /// `size`, and background `bkgnd`.
    fn interface(font: &str, size: i16, bkgnd: i16) -> Vec<u8> {
        let mut bytes = vec![0; Interface::SIZE.expect("fixed")];
        put_u32(&mut bytes, 0x00, 0x00FF_FFFF);
        put_u32(&mut bytes, 0x04, 0x0080_8080);
        put_rect(&mut bytes, 0x08, (8, 8, 184, 184));
        put_u32(&mut bytes, 0x10, 0x0000_FF00);
        put_u32(&mut bytes, 0x14, 0x0000_8000);
        put_rect(&mut bytes, 0x18, (199, 35, 206, 184));
        put_u32(&mut bytes, 0x20, 0x0000_00FF);
        put_rect(&mut bytes, 0x24, (216, 35, 223, 184));
        put_u32(&mut bytes, 0x2C, 0x00FF_0000);
        put_rect(&mut bytes, 0x30, (234, 35, 241, 184));
        put_u32(&mut bytes, 0x38, 0x00FF_FF00);
        put_u32(&mut bytes, 0x3C, 0x0080_8000);
        put_rect(&mut bytes, 0x40, (254, 8, 286, 184));
        put_rect(&mut bytes, 0x48, (300, 8, 315, 184));
        put_rect(&mut bytes, 0x50, (330, 8, 442, 184));
        bytes[0x60..0x60 + font.len()].copy_from_slice(font.as_bytes());
        put_i16(&mut bytes, 0xA0, size);
        put_i16(&mut bytes, 0xA2, size - 2);
        put_i16(&mut bytes, 0xA4, bkgnd);
        bytes
    }

    /// A `gövt` whose `Interface` is `interface`.
    fn govt(interface: i16) -> Vec<u8> {
        let mut bytes = vec![0; Govt::SIZE.expect("fixed")];
        put_i16(&mut bytes, 0xAC, interface);
        bytes
    }

    /// A grey `width` x `height` `PICT`.
    fn picture(width: i16, height: i16) -> Vec<u8> {
        let bounds = [0, 0, height, width];
        let pixels = vec![0x4210; (width * height) as usize];
        PictBuilder::new(bounds)
            .direct_bits(&DirectBits::rgb555(bounds, &pixels))
            .end()
            .build()
    }

    fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
        Bounds {
            min: Point::new(left, top),
            max: Point::new(right, bottom),
        }
    }

    #[test]
    fn a_status_bar_is_its_areas_colours_font_and_raw_background() {
        let data = store(&[(Interface::TYPE, 128, interface("Charcoal", 12, 700))]);
        assert_eq!(
            data.status_bar(128),
            Ok(StatusBarLayout {
                radar: rect(8.0, 8.0, 184.0, 184.0),
                shield: rect(35.0, 199.0, 184.0, 206.0),
                armor: rect(35.0, 216.0, 184.0, 223.0),
                fuel: rect(35.0, 234.0, 184.0, 241.0),
                nav: rect(8.0, 254.0, 184.0, 286.0),
                weap: rect(8.0, 300.0, 184.0, 315.0),
                targ: rect(8.0, 330.0, 184.0, 442.0),
                bright_text: Color::rgba(255, 255, 255, 255),
                dim_text: Color::rgba(128, 128, 128, 255),
                bright_radar: Color::rgba(0, 255, 0, 255),
                dim_radar: Color::rgba(0, 128, 0, 255),
                shield_color: Color::rgba(0, 0, 255, 255),
                armor_color: Color::rgba(255, 0, 0, 255),
                fuel_full: Color::rgba(255, 255, 0, 255),
                fuel_partial: Color::rgba(128, 128, 0, 255),
                font: Font::Charcoal,
                font_size: 12.0,
                subtitle_size: 10.0,
                status_bkgnd: 700,
            })
        );
        let geneva = store(&[(Interface::TYPE, 128, interface("Geneva", 9, 0))]);
        let bar = geneva.status_bar(128).expect("decodes");
        assert_eq!((bar.font, bar.font_size), (Font::Geneva, 9.0));
        assert_eq!(bar.subtitle_size, 7.0);
        assert_eq!(bar.status_bkgnd, 0, "raw, below 128");
        let below = store(&[(Interface::TYPE, 128, interface("Geneva", 9, -1))]);
        assert_eq!(below.status_bar(128).map(|bar| bar.status_bkgnd), Ok(-1));
    }

    #[test]
    fn a_missing_or_short_status_bar_says_why() {
        let mut short = interface("Geneva", 12, 700);
        short.pop();
        let data = store(&[(Interface::TYPE, 129, short)]);
        assert_eq!(data.status_bar(128), Err("no ïntf 128".to_owned()));
        let Err(message) = data.status_bar(129) else {
            panic!("an error")
        };
        assert!(message.contains("129"), "{message}");
    }

    #[test]
    fn a_governments_interface_is_its_raw_field() {
        let mut short = govt(130);
        short.pop();
        let data = store(&[
            (Govt::TYPE, 128, govt(130)),
            (Govt::TYPE, 129, govt(-1)),
            (Govt::TYPE, 131, short),
        ]);
        assert_eq!(data.government_interface(GovtId(128)), Ok(130));
        assert_eq!(data.government_interface(GovtId(129)), Ok(-1));
        assert_eq!(
            data.government_interface(GovtId(130)),
            Err("no gövt 130".to_owned())
        );
        let Err(message) = data.government_interface(GovtId(131)) else {
            panic!("an error")
        };
        assert!(message.contains("131"), "{message}");
    }

    #[test]
    fn a_pictures_size_is_its_decoded_width_and_height() {
        let data = store(&[(PICT, 700, picture(194, 16)), (PICT, 701, vec![0; 4])]);
        assert_eq!(data.picture_size(700), Some((194, 16)));
        assert_eq!(data.picture_size(701), None, "undecodable");
        assert_eq!(data.picture_size(702), None, "missing");
    }

    // Combat looks.

    use nova_data::records::boom::Boom;
    use nova_data::records::spin::Spin;
    use nova_data::records::weapon::Weapon;
    use nova_data::sound::fixture::{Header, SndBuilder, SndFormat};

    use super::super::catalog::{BoomId, CombatLooks, EffectSheet, SoundId, TargetCard, WeaponId};

    /// The store of these resources, each with its name, if any.
    fn named_store(resources: &[(ResType, i16, Option<&str>, Vec<u8>)]) -> GameData {
        let fork = resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, name, data)| {
                fork.resource(*ty, *id, name.map(str::as_bytes), data)
            })
            .build()
            .bytes;
        let file = OneFile(fork);
        GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
    }

    fn put_u16(bytes: &mut [u8], at: usize, value: u16) {
        bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
    }

    /// A `wëap` of `graphic` and `sound`, with distinct presentation
    /// fields.
    fn weapon(graphic: i16, sound: i16) -> Vec<u8> {
        let mut bytes = vec![0; Weapon::SIZE.expect("fixed")];
        put_i16(&mut bytes, 0x0E, graphic);
        put_i16(&mut bytes, 0x12, sound);
        put_u16(&mut bytes, 0x1C, 0x0011);
        put_i16(&mut bytes, 0x32, 3);
        put_i16(&mut bytes, 0x34, 8);
        put_u32(&mut bytes, 0x36, 0x00FF_8000);
        put_u32(&mut bytes, 0x3A, 0x0000_80FF);
        put_i16(&mut bytes, 0x46, 6);
        put_u16(&mut bytes, 0x48, 0x2002);
        put_u16(&mut bytes, 0x66, 0x0002);
        bytes
    }

    /// A `spïn` of sprites `image` in a 6 x 6 grid.
    fn spin(image: i16) -> Vec<u8> {
        let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
        put_i16(&mut bytes, 0x00, image);
        put_i16(&mut bytes, 0x02, -1);
        put_i16(&mut bytes, 0x08, 6);
        put_i16(&mut bytes, 0x0A, 6);
        bytes
    }

    fn boom(advance: i16, sound: i16, graphic: i16) -> Vec<u8> {
        let mut bytes = vec![0; Boom::SIZE.expect("fixed")];
        put_i16(&mut bytes, 0, advance);
        put_i16(&mut bytes, 2, sound);
        put_i16(&mut bytes, 4, graphic);
        bytes
    }

    fn sheet_of(image_id: i16, frames: u16) -> EffectSheet {
        EffectSheet {
            image_id,
            frames: NonZeroU16::new(frames).expect("non-zero"),
        }
    }

    #[test]
    fn a_weapons_look_is_its_name_shot_sheet_sound_and_presentation_fields() {
        let data = named_store(&[
            (
                Weapon::TYPE,
                128,
                Some("Light Blaster;mounted"),
                weapon(0, 8),
            ),
            (Spin::TYPE, 3000, None, spin(3500)),
            (RLED, 3500, None, sheet(36)),
        ]);
        assert_eq!(
            data.weapon_look(WeaponId(128)),
            Ok(WeaponLook {
                name: "Light Blaster".to_owned(),
                sheet: Some(Ok(sheet_of(3500, 36))),
                sound: Some(SoundId(208)),
                flags: 0x0011,
                flags2: 0x2002,
                flags3: 0x0002,
                beam_width: 3,
                falloff: 8,
                beam_color: 0x00FF_8000,
                corona_color: 0x0000_80FF,
                prox_safety: 6,
                sound_ticks: None,
            })
        );
    }

    /// A `snd ` of `frames` 8-bit mono frames at 22,050 Hz.
    fn snd(frames: usize) -> Vec<u8> {
        SndBuilder::new(
            SndFormat::Two,
            Header::Standard {
                rate: 22_050 << 16,
                loop_points: (0, 0),
                base_note: 60,
                samples: vec![0x80; frames],
            },
        )
        .bytes()
    }

    #[test]
    fn a_weapons_sound_lasts_its_frames_over_its_rate_in_ticks_rounded_up() {
        for (frames, ticks) in [(11_025, 15), (11_026, 16), (11_024, 15), (735, 1)] {
            let data = named_store(&[
                (Weapon::TYPE, 155, Some("Hail Chaingun"), weapon(-1, 5)),
                (SND, 205, None, snd(frames)),
            ]);
            let look = data.weapon_look(WeaponId(155)).expect("reads");
            assert_eq!(look.sound_ticks, Some(ticks), "{frames}");
        }
    }

    #[test]
    fn a_weapon_whose_sound_is_missing_or_cannot_be_decoded_has_no_length() {
        let data = named_store(&[
            (Weapon::TYPE, 155, Some("Hail Chaingun"), weapon(-1, 5)),
            (Weapon::TYPE, 156, Some("Railgun"), weapon(-1, 6)),
            (Weapon::TYPE, 157, Some("Silent"), weapon(-1, -1)),
            (Weapon::TYPE, 158, Some("Rateless"), weapon(-1, 8)),
            (SND, 205, None, vec![0, 1, 2]),
            (SND, 199, None, snd(100)),
            (
                SND,
                208,
                None,
                SndBuilder::new(
                    SndFormat::Two,
                    Header::Standard {
                        rate: 0,
                        loop_points: (0, 0),
                        base_note: 60,
                        samples: vec![0x80; 100],
                    },
                )
                .bytes(),
            ),
        ]);
        for id in [155, 156, 157, 158] {
            let look = data.weapon_look(WeaponId(id)).expect("reads");
            assert_eq!(look.sound_ticks, None, "{id}");
        }
    }

    #[test]
    fn a_weapon_without_a_graphic_or_a_sound_has_neither() {
        let data = named_store(&[(Weapon::TYPE, 146, Some("Laser"), weapon(-1, -1))]);
        let look = data.weapon_look(WeaponId(146)).expect("reads");
        assert_eq!(
            (look.name.as_str(), look.sheet, look.sound),
            ("Laser", None, None)
        );
        let unnamed = named_store(&[(Weapon::TYPE, 147, None, weapon(-1, 0))]);
        let look = unnamed.weapon_look(WeaponId(147)).expect("reads");
        assert_eq!((look.name.as_str(), look.sound), ("", Some(SoundId(200))));
    }

    #[test]
    fn a_weapon_whose_shots_cannot_be_shown_keeps_the_rest_of_its_look() {
        let data = named_store(&[(Weapon::TYPE, 128, Some("Blaster"), weapon(1, 8))]);
        let look = data.weapon_look(WeaponId(128)).expect("reads");
        assert_eq!(look.sheet, Some(Err("no spïn 3001".to_owned())));
        assert_eq!(
            (look.name.as_str(), look.sound, look.flags2, look.beam_color),
            ("Blaster", Some(SoundId(208)), 0x2002, 0x00FF_8000)
        );
    }

    #[test]
    fn a_weapon_that_cannot_be_read_says_why() {
        let mut short = weapon(-1, -1);
        short.pop();
        let data = named_store(&[(Weapon::TYPE, 129, None, short)]);
        let Err(message) = data.weapon_look(WeaponId(129)) else {
            panic!("an error")
        };
        assert!(message.contains("129"), "{message}");
        assert_eq!(
            data.weapon_look(WeaponId(130)),
            Err("no wëap 130".to_owned())
        );
    }

    #[test]
    fn an_explosions_look_is_its_sheet_advance_and_sound() {
        let data = named_store(&[
            (Boom::TYPE, 130, None, boom(100, 2, 0)),
            (Boom::TYPE, 135, None, boom(30, -1, 5)),
            (Spin::TYPE, 400, None, spin(400)),
            (RLED, 400, None, sheet(12)),
            (Spin::TYPE, 405, None, spin(405)),
            (RLED, 405, None, sheet(20)),
        ]);
        let look = data.boom_look(BoomId(130)).expect("there").expect("reads");
        assert_eq!(look.sheet, Ok(sheet_of(400, 12)));
        assert_eq!((look.advance, look.sound), (1.0, Some(SoundId(302))));
        let slow = data.boom_look(BoomId(135)).expect("there").expect("reads");
        assert_eq!(slow.sheet, Ok(sheet_of(405, 20)));
        assert!((slow.advance - 0.3).abs() < 1e-6, "{slow:?}");
        assert_eq!(slow.sound, None);
    }

    #[test]
    fn an_explosion_that_cannot_be_shown_keeps_its_sound_and_advance() {
        let data = named_store(&[(Boom::TYPE, 128, None, boom(100, 0, 3))]);
        assert_eq!(
            data.boom_look(BoomId(128)),
            Some(Ok(BoomLook {
                sheet: Err("no spïn 403".to_owned()),
                advance: 1.0,
                sound: Some(SoundId(300)),
            }))
        );
        let empty = named_store(&[
            (Boom::TYPE, 128, None, boom(100, 0, 0)),
            (Spin::TYPE, 400, None, spin(400)),
            (RLED, 400, None, sheet(0)),
        ]);
        let look = empty.boom_look(BoomId(128)).expect("there").expect("reads");
        assert_eq!(
            look.sheet,
            Err("rlëD 400: the rlëD sheet has no frames".to_owned())
        );
    }

    #[test]
    fn an_explosion_that_cannot_be_read_says_why_and_one_not_there_is_none() {
        let mut short = boom(100, 0, 0);
        short.pop();
        let data = named_store(&[(Boom::TYPE, 128, None, short)]);
        let Some(Err(message)) = data.boom_look(BoomId(128)) else {
            panic!("an error")
        };
        assert!(message.contains("128"), "{message}");
        assert_eq!(data.boom_look(BoomId(129)), None);
    }

    /// A `shïp` with `subtitle`.
    fn subtitled(subtitle: &str) -> Vec<u8> {
        let mut bytes = ship();
        bytes[0x6E6..0x6E6 + subtitle.len()].copy_from_slice(subtitle.as_bytes());
        bytes
    }

    #[test]
    fn a_target_card_is_the_ships_subtitle_and_picture() {
        let data = named_store(&[
            (Ship::TYPE, 128, None, subtitled("Light Transport")),
            (Ship::TYPE, 129, None, subtitled("Freighter")),
            (PICT, 3000, None, picture(2, 2)),
        ]);
        assert_eq!(
            data.target_card(ShipId(128)),
            TargetCard {
                subtitle: "Light Transport".to_owned(),
                picture: Some(3000),
            }
        );
        assert_eq!(
            data.target_card(ShipId(129)),
            TargetCard {
                subtitle: "Freighter".to_owned(),
                picture: None,
            },
            "without a PICT 3001"
        );
        assert_eq!(data.target_card(ShipId(130)), TargetCard::default());
    }

    /// A `gövt` whose `TargetCode` is `code`.
    fn coded(code: &str) -> Vec<u8> {
        let mut bytes = govt(-1);
        bytes[0x44..0x44 + code.len()].copy_from_slice(code.as_bytes());
        bytes
    }

    #[test]
    fn a_target_code_is_the_governments() {
        let data = named_store(&[
            (Govt::TYPE, 128, None, coded("Fed.")),
            (Govt::TYPE, 129, None, coded("")),
        ]);
        assert_eq!(data.target_code(GovtId(128)), Some("Fed.".to_owned()));
        assert_eq!(data.target_code(GovtId(129)), None, "none set");
        assert_eq!(data.target_code(GovtId(130)), None, "no gövt");
    }
}
