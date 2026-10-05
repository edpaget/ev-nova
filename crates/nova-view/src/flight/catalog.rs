//! The flight screen's ports, in the view's own terms: the ship sprite
//! port, the player ship's sprite sheet, the status bar port, the `ïntf`
//! layouts the HUD is drawn from, and the combat looks port, how weapons,
//! explosions and targets are shown.

use std::collections::BTreeMap;
use std::num::NonZeroU16;
use std::rc::Rc;

pub use nova_sim::{BoomId, GovtId, ShipId, SoundId, WeaponId};

use crate::color::Color;
use crate::font::Font;
use crate::geometry::Bounds;

/// A ship's resolved sprite sheet: its `rlëD`, how many frames make one
/// turn, and each frame's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipSheet {
    /// The `rlëD`'s ID.
    pub image_id: i16,
    /// The `shän`'s frames per rotation: the frames of one set, which turn
    /// the ship once round.
    pub rotations: NonZeroU16,
    /// Each frame's width in pixels.
    pub frame_width: u32,
    /// Each frame's height in pixels.
    pub frame_height: u32,
}

/// The ships' sprite sheets.
pub trait ShipSprites {
    /// Ship `id`'s sheet, or why it cannot be shown.
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String>;
}

/// A borrowed catalog is a catalog.
impl<T: ShipSprites + ?Sized> ShipSprites for &T {
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
        (**self).ship_sheet(id)
    }
}

/// A shared catalog is a catalog, so a view and the renderer can read the
/// same game data.
impl<T: ShipSprites + ?Sized> ShipSprites for Rc<T> {
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
        (**self).ship_sheet(id)
    }
}

/// An `ïntf` status bar layout. Every area is relative to the status
/// bar's top-left corner.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusBarLayout {
    /// `RadarArea`.
    pub radar: Bounds,
    /// `ShieldArea`.
    pub shield: Bounds,
    /// `ArmorArea`.
    pub armor: Bounds,
    /// `FuelArea`.
    pub fuel: Bounds,
    /// `NavArea`.
    pub nav: Bounds,
    /// `WeapArea`: the secondary weapon's line.
    pub weap: Bounds,
    /// `TargArea`: the target panel.
    pub targ: Bounds,
    /// `BrightText`.
    pub bright_text: Color,
    /// `DimText`.
    pub dim_text: Color,
    /// `BrightRadar`.
    pub bright_radar: Color,
    /// `DimRadar`.
    pub dim_radar: Color,
    /// `ShieldColor`.
    pub shield_color: Color,
    /// `ArmorColor`.
    pub armor_color: Color,
    /// `FuelFull`: whole jumps' worth of fuel.
    pub fuel_full: Color,
    /// `FuelPartial`: the fuel left over a whole jump.
    pub fuel_partial: Color,
    /// `StatusFont`.
    pub font: Font,
    /// `StatFontSize`.
    pub font_size: f32,
    /// `SubtitleSize`: the target's subtitle.
    pub subtitle_size: f32,
    /// `StatusBkgnd`, raw: the background `PICT`'s ID, where values below
    /// 128 mean 128.
    pub status_bkgnd: i16,
}

/// The status bars: which one each government shows and how each is laid
/// out.
pub trait StatusBars {
    /// Govt `id`'s raw `Interface` field, or why it cannot be read.
    fn government_interface(&self, id: GovtId) -> Result<i16, String>;
    /// `ïntf` `id`, or why it cannot be read.
    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String>;
    /// `PICT` `id`'s width and height in pixels, or `None` when it is
    /// missing or does not decode.
    fn picture_size(&self, id: i16) -> Option<(u32, u32)>;
}

/// A borrowed catalog is a catalog.
impl<T: StatusBars + ?Sized> StatusBars for &T {
    fn government_interface(&self, id: GovtId) -> Result<i16, String> {
        (**self).government_interface(id)
    }

    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
        (**self).status_bar(id)
    }

    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        (**self).picture_size(id)
    }
}

/// A shared catalog is a catalog.
impl<T: StatusBars + ?Sized> StatusBars for Rc<T> {
    fn government_interface(&self, id: GovtId) -> Result<i16, String> {
        (**self).government_interface(id)
    }

    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
        (**self).status_bar(id)
    }

    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        (**self).picture_size(id)
    }
}

/// An effect's sprite sheet: its `rlëD`, and how many frames it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectSheet {
    /// The `rlëD`'s ID.
    pub image_id: i16,
    /// How many frames the sheet holds.
    pub frames: NonZeroU16,
}

/// How a weapon's shots, beams and firing are shown: its `wëap`'s
/// presentation fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeaponLook {
    /// Its name: the `wëap` resource's, up to any ';'.
    pub name: String,
    /// Its shots' sheet, `spïn` 3000 + `Graphic`, or why it cannot be read;
    /// none for a weapon whose shots have no graphic (a beam's).
    pub sheet: Option<Result<EffectSheet, String>>,
    /// The sound it fires with, `snd ` 200 + `Sound`, if any.
    pub sound: Option<SoundId>,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `Flags2`.
    pub flags2: u16,
    /// Its `Flags3`.
    pub flags3: u16,
    /// `BeamWidth`: a beam's width, and a spinning shot's ticks a frame.
    pub beam_width: i16,
    /// `Falloff`: how fast a beam's corona fades; none for no corona.
    pub falloff: i16,
    /// `BeamColor`, `00RRGGBB`.
    pub beam_color: u32,
    /// `CoronaColor`, `00RRGGBB`.
    pub corona_color: u32,
    /// `ProxSafety`: the ticks a shot's frame waits before it spins, with
    /// `Flags2` 0x0001.
    pub prox_safety: i16,
}

/// How an explosion type is shown and heard: its `bööm`.
#[derive(Clone, Debug, PartialEq)]
pub struct BoomLook {
    /// Its sheet, `spïn` 400 + `GraphicIndex`, or why it cannot be read.
    pub sheet: Result<EffectSheet, String>,
    /// The frames it advances a tick: `FrameAdvance` / 100.
    pub advance: f32,
    /// Its sound, `snd ` 300 + `SoundIndex`, if any.
    pub sound: Option<SoundId>,
}

/// What the target panel shows of a ship type besides its name, which is
/// the simulation's (`ShipRecord::name`, through
/// `Session::ship_name`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TargetCard {
    /// Its `Subtitle`.
    pub subtitle: String,
    /// Its picture, `PICT` 3000 + (ID - 128), when there is one.
    pub picture: Option<i16>,
}

/// How weapons, explosions and targets look.
pub trait CombatLooks {
    /// Weapon `id`'s look, or why it cannot be read.
    fn weapon_look(&self, id: WeaponId) -> Result<WeaponLook, String>;
    /// Explosion type `id`'s look, or why it cannot be read; none when
    /// there is no such `bööm`.
    fn boom_look(&self, id: BoomId) -> Option<Result<BoomLook, String>>;
    /// What the target panel shows of ship type `ship`; empty when it
    /// cannot be read.
    fn target_card(&self, ship: ShipId) -> TargetCard;
    /// Govt `govt`'s `TargetCode`, if it has one.
    fn target_code(&self, govt: GovtId) -> Option<String>;
}

/// A borrowed catalog is a catalog.
impl<T: CombatLooks + ?Sized> CombatLooks for &T {
    fn weapon_look(&self, id: WeaponId) -> Result<WeaponLook, String> {
        (**self).weapon_look(id)
    }

    fn boom_look(&self, id: BoomId) -> Option<Result<BoomLook, String>> {
        (**self).boom_look(id)
    }

    fn target_card(&self, ship: ShipId) -> TargetCard {
        (**self).target_card(ship)
    }

    fn target_code(&self, govt: GovtId) -> Option<String> {
        (**self).target_code(govt)
    }
}

/// A shared catalog is a catalog.
impl<T: CombatLooks + ?Sized> CombatLooks for Rc<T> {
    fn weapon_look(&self, id: WeaponId) -> Result<WeaponLook, String> {
        (**self).weapon_look(id)
    }

    fn boom_look(&self, id: BoomId) -> Option<Result<BoomLook, String>> {
        (**self).boom_look(id)
    }

    fn target_card(&self, ship: ShipId) -> TargetCard {
        (**self).target_card(ship)
    }

    fn target_code(&self, govt: GovtId) -> Option<String> {
        (**self).target_code(govt)
    }
}

/// The first explosion type, `bööm` 128.
pub const FIRST_BOOM: i16 = 128;
/// The last explosion type, `bööm` 191: 64 in all.
pub const LAST_BOOM: i16 = 191;

/// The weapons' and explosions' looks, read once.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Looks {
    /// Each weapon's look, or why it cannot be read.
    pub weapons: BTreeMap<WeaponId, Result<WeaponLook, String>>,
    /// Each explosion type there is: its look, or why it cannot be read.
    pub booms: BTreeMap<BoomId, Result<BoomLook, String>>,
}

impl Looks {
    /// The looks of `weapons` and of every explosion type, read from
    /// `catalog`.
    pub fn read(catalog: &impl CombatLooks, weapons: impl IntoIterator<Item = WeaponId>) -> Self {
        Self {
            weapons: weapons
                .into_iter()
                .map(|id| (id, catalog.weapon_look(id)))
                .collect(),
            booms: (FIRST_BOOM..=LAST_BOOM)
                .filter_map(|id| Some((BoomId(id), catalog.boom_look(BoomId(id))?)))
                .collect(),
        }
    }

    /// Weapon `id`'s look, if it was read.
    #[must_use]
    pub fn weapon(&self, id: WeaponId) -> Option<&WeaponLook> {
        self.weapons.get(&id)?.as_ref().ok()
    }

    /// Explosion type `id`'s look, if it was read.
    #[must_use]
    pub fn boom(&self, id: BoomId) -> Option<&BoomLook> {
        self.booms.get(&id)?.as_ref().ok()
    }

    /// What could not be read, a line each: a look that could not be read
    /// at all, as the catalog says, and a sheet that could not, after its
    /// `wëap` or `bööm` ("wëap 140: no spïn 3005"); the weapons' by ID,
    /// then the explosions'.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let weapons = self.weapons.iter().filter_map(|(id, look)| match look {
            Err(reason) => Some(reason.clone()),
            Ok(look) => match &look.sheet {
                Some(Err(reason)) => Some(format!("wëap {}: {reason}", id.0)),
                _ => None,
            },
        });
        let booms = self.booms.iter().filter_map(|(id, look)| match look {
            Err(reason) => Some(reason.clone()),
            Ok(look) => match &look.sheet {
                Err(reason) => Some(format!("bööm {}: {reason}", id.0)),
                Ok(_) => None,
            },
        });
        weapons.chain(booms).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    /// Every ship has a 36-rotation sheet with its own ID.
    struct Sheets;

    impl ShipSprites for Sheets {
        fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
            Ok(ShipSheet {
                image_id: id.0,
                rotations: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 48,
                frame_height: 48,
            })
        }
    }

    fn image(catalog: impl ShipSprites, id: i16) -> Result<i16, String> {
        catalog.ship_sheet(ShipId(id)).map(|sheet| sheet.image_id)
    }

    #[test]
    fn borrowed_and_shared_catalogs_are_catalogs() {
        assert_eq!(image(&Sheets, 130), Ok(130));
        assert_eq!(image(Rc::new(Sheets), 131), Ok(131));
    }

    /// Every govt shows its own ID's bar; every bar is empty but for its
    /// background, its own ID; every picture is as wide as its ID.
    struct Bars;

    impl StatusBars for Bars {
        fn government_interface(&self, id: GovtId) -> Result<i16, String> {
            Ok(id.0)
        }

        fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
            let none = Bounds::at(Point::default(), 0.0, 0.0);
            Ok(StatusBarLayout {
                radar: none,
                shield: none,
                armor: none,
                fuel: none,
                nav: none,
                weap: none,
                targ: none,
                bright_text: Color::WHITE,
                dim_text: Color::WHITE,
                bright_radar: Color::WHITE,
                dim_radar: Color::WHITE,
                shield_color: Color::WHITE,
                armor_color: Color::WHITE,
                fuel_full: Color::WHITE,
                fuel_partial: Color::WHITE,
                font: Font::Geneva,
                font_size: 12.0,
                subtitle_size: 10.0,
                status_bkgnd: id,
            })
        }

        fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
            Some((u32::try_from(id).ok()?, 1))
        }
    }

    /// Everything `catalog` says about govt 130, `ïntf` 131 and `PICT` 194.
    fn bar(catalog: impl StatusBars) -> String {
        format!(
            "{:?} {:?} {:?}",
            catalog.government_interface(GovtId(130)),
            catalog.status_bar(131).map(|bar| bar.status_bkgnd),
            catalog.picture_size(194),
        )
    }

    #[test]
    fn borrowed_and_shared_status_bars_are_status_bars() {
        let direct = "Ok(130) Ok(131) Some((194, 1))";
        assert_eq!(bar(Bars), direct);
        assert_eq!(bar(&Bars), direct);
        assert_eq!(bar(Rc::new(Bars)), direct);
    }

    /// Every weapon is named for its ID; every explosion type up to 150 is
    /// a sheet of its ID, and there are none past it; every ship's
    /// subtitle is its ID; every govt's code is its ID.
    struct Named;

    impl CombatLooks for Named {
        fn weapon_look(&self, id: WeaponId) -> Result<WeaponLook, String> {
            Ok(WeaponLook {
                name: format!("w{}", id.0),
                ..WeaponLook::default()
            })
        }

        fn boom_look(&self, id: BoomId) -> Option<Result<BoomLook, String>> {
            (id.0 <= 150).then_some(Ok(BoomLook {
                sheet: Ok(EffectSheet {
                    image_id: id.0,
                    frames: NonZeroU16::MIN,
                }),
                advance: 1.0,
                sound: None,
            }))
        }

        fn target_card(&self, ship: ShipId) -> TargetCard {
            TargetCard {
                subtitle: format!("s{}", ship.0),
                ..TargetCard::default()
            }
        }

        fn target_code(&self, govt: GovtId) -> Option<String> {
            Some(format!("g{}", govt.0))
        }
    }

    /// Everything `catalog` says about weapon 128, `bööm` 129, ship 130
    /// and govt 131.
    fn looks(catalog: impl CombatLooks) -> String {
        format!(
            "{:?} {:?} {:?} {:?}",
            catalog.weapon_look(WeaponId(128)).map(|look| look.name),
            catalog
                .boom_look(BoomId(129))
                .map(|look| look.map(|look| look.sheet.map(|sheet| sheet.image_id))),
            catalog.target_card(ShipId(130)).subtitle,
            catalog.target_code(GovtId(131)),
        )
    }

    #[test]
    fn the_looks_read_are_the_weapons_given_and_every_explosion_type_there_is() {
        let looks = Looks::read(&Named, [WeaponId(140), WeaponId(128)]);
        let weapons: Vec<_> = looks.weapons.keys().copied().collect();
        assert_eq!(weapons, [WeaponId(128), WeaponId(140)]);
        assert_eq!(
            looks.weapon(WeaponId(140)).map(|look| look.name.as_str()),
            Some("w140")
        );
        assert_eq!(looks.weapon(WeaponId(129)), None);
        let booms: Vec<_> = looks.booms.keys().map(|id| id.0).collect();
        assert_eq!(booms, (128..=150).collect::<Vec<_>>());
        assert_eq!(
            looks.boom(BoomId(150)).map(|look| look.sheet.clone()),
            Some(Ok(EffectSheet {
                image_id: 150,
                frames: NonZeroU16::MIN
            }))
        );
        assert_eq!(looks.boom(BoomId(151)), None);
        assert_eq!(looks.problems(), Vec::<String>::new());
        let unread = Looks {
            weapons: BTreeMap::from([(WeaponId(1), Err("no".to_owned()))]),
            booms: BTreeMap::from([(BoomId(1), Err("no".to_owned()))]),
        };
        assert_eq!(unread.weapon(WeaponId(1)), None);
        assert_eq!(unread.boom(BoomId(1)), None);
    }

    #[test]
    fn borrowed_and_shared_combat_looks_are_combat_looks() {
        let direct = r#"Ok("w128") Some(Ok(Ok(129))) "s130" Some("g131")"#;
        assert_eq!(looks(Named), direct);
        assert_eq!(looks(&Named), direct);
        assert_eq!(looks(Rc::new(Named)), direct);
    }

    /// Weapon 140's shots and `bööm` 128 have no sheet, and weapon 141
    /// and `bööm` 129 cannot be read at all; there are no other
    /// explosion types.
    struct Broken;

    impl CombatLooks for Broken {
        fn weapon_look(&self, id: WeaponId) -> Result<WeaponLook, String> {
            match id.0 {
                140 => Ok(WeaponLook {
                    name: "Blaster".to_owned(),
                    sheet: Some(Err("no spïn 3005".to_owned())),
                    sound: Some(SoundId(208)),
                    flags2: 0x0040,
                    ..WeaponLook::default()
                }),
                other => Err(format!("no wëap {other}")),
            }
        }

        fn boom_look(&self, id: BoomId) -> Option<Result<BoomLook, String>> {
            match id.0 {
                128 => Some(Ok(BoomLook {
                    sheet: Err("no spïn 400".to_owned()),
                    advance: 1.0,
                    sound: Some(SoundId(302)),
                })),
                129 => Some(Err("bööm 129: too short".to_owned())),
                _ => None,
            }
        }

        fn target_card(&self, _ship: ShipId) -> TargetCard {
            TargetCard::default()
        }

        fn target_code(&self, _govt: GovtId) -> Option<String> {
            None
        }
    }

    #[test]
    fn a_look_whose_sheet_cannot_be_read_keeps_the_rest_and_says_why() {
        let looks = Looks::read(&Broken, [WeaponId(141), WeaponId(140)]);
        let blaster = looks.weapon(WeaponId(140)).expect("read");
        assert_eq!(
            (blaster.name.as_str(), blaster.sound, blaster.flags2),
            ("Blaster", Some(SoundId(208)), 0x0040)
        );
        let boom = looks.boom(BoomId(128)).expect("read");
        assert_eq!((boom.advance, boom.sound), (1.0, Some(SoundId(302))));
        assert_eq!(looks.weapon(WeaponId(141)), None);
        assert_eq!(looks.boom(BoomId(129)), None);
        assert_eq!(
            looks.problems(),
            [
                "wëap 140: no spïn 3005",
                "no wëap 141",
                "bööm 128: no spïn 400",
                "bööm 129: too short",
            ]
        );
    }
}
