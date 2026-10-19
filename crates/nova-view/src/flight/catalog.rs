//! The flight screen's ports, in the view's own terms: the ship sprite
//! port, the player ship's sprite sheet, the status bar port, the `ïntf`
//! layouts the HUD is drawn from, and the combat looks port, how weapons,
//! explosions and targets are shown.

use std::collections::BTreeMap;
use std::num::NonZeroU16;
use std::rc::Rc;

use nova_sim::asteroid::KINDS;
pub use nova_sim::{Blink, BoomId, GovtId, ShipId, SoundId, WeaponId};

use crate::color::Color;
use crate::font::Font;
use crate::geometry::Bounds;

/// One of a ship's `shän` layers drawn over its base sprite: the layer's
/// `rlëD` and how many frames it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerSheet {
    /// The `rlëD`'s ID.
    pub image_id: i16,
    /// The layer's frames. The base's rotation frame `f` is drawn with the
    /// layer's frame `f % frames`.
    pub frames: NonZeroU16,
}

/// A ship's resolved sprite sheet: its `rlëD`, how many frames make one
/// turn, each frame's size, and the engine glow and running lights drawn
/// over it.
///
/// A layer the `shän` names but whose image cannot be resolved is `None`,
/// like a layer it does not name: flight draws the ship without it and
/// says nothing. The ship browser is where layer errors are reported.
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
    /// The engine glow, drawn at the level [`nova_sim::glow_level`] gives
    /// for the ship's glow base, which thrust ramps up.
    pub glow: Option<LayerSheet>,
    /// The running lights, drawn at the level the session's display drew
    /// them (`nova_sim::combat::ship_lights`).
    pub lights: Option<LayerSheet>,
    /// The weapon glow (`WeapImageID`), drawn while a `Flags2` 0x0200
    /// weapon's glow lasts (see the `weapon_glow` module).
    pub weapon: Option<LayerSheet>,
    /// The `shän`'s `WeapDecay`, raw: how fast the weapon glow fades
    /// ([`ShipSheet::weapon_decay`]).
    pub weap_decay: i16,
    /// The alt layer (`AltImageID`, loaded only with an `AltSetCount` above
    /// 0), drawn at its own set with the hull's tint.
    pub alt: Option<LayerSheet>,
    /// The shield layer (`ShieldImageID`), flashed on a hit (see
    /// [`ShieldFlash`](super::flash::ShieldFlash)).
    pub shield: Option<LayerSheet>,
    /// The `shän`'s `BaseTransp`, raw: the hull's translucency in 32nds,
    /// drawn at (32 − it)/32 (see the flight view's ship drawing).
    pub base_transp: i16,
}

/// What `WeapDecay` is multiplied by for the glow's fall a tick
/// (`_LoadExtendedShipSprites` @0x2279f-0x227bb, the double 0.333).
pub const WEAP_DECAY_SCALE: f64 = 0.333;

impl ShipSheet {
    /// How far the weapon glow falls a tick, out of 32: `WeapDecay` ×
    /// [`WEAP_DECAY_SCALE`], a double narrowed to a float as
    /// `_LoadExtendedShipSprites` stores it in the class (+0x4c).
    #[must_use]
    pub fn weapon_decay(&self) -> f32 {
        // A third of an `i16`: well within an `f32`.
        #[allow(clippy::cast_possible_truncation)]
        let decay = (f64::from(self.weap_decay) * WEAP_DECAY_SCALE) as f32;
        decay
    }
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
    /// `CargoArea`: the hold, free space, mission cargo and credits
    /// (`_DrawStatusCargo`).
    pub cargo: Bounds,
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
    /// Govt `id`'s raw `Interface` field, `None` when there is no such
    /// `gövt`, or why it does not decode.
    fn government_interface(&self, id: GovtId) -> Result<Option<i16>, String>;
    /// `ïntf` `id`, or why it cannot be read.
    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String>;
    /// `PICT` `id`'s width and height in pixels, or `None` when it is
    /// missing or does not decode.
    fn picture_size(&self, id: i16) -> Option<(u32, u32)>;
    /// `cicn` `id`'s width and height in pixels, or `None` when it is
    /// missing or does not decode.
    fn icon_size(&self, id: i16) -> Option<(u32, u32)>;
}

/// A borrowed catalog is a catalog.
impl<T: StatusBars + ?Sized> StatusBars for &T {
    fn government_interface(&self, id: GovtId) -> Result<Option<i16>, String> {
        (**self).government_interface(id)
    }

    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
        (**self).status_bar(id)
    }

    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        (**self).picture_size(id)
    }

    fn icon_size(&self, id: i16) -> Option<(u32, u32)> {
        (**self).icon_size(id)
    }
}

/// A shared catalog is a catalog.
impl<T: StatusBars + ?Sized> StatusBars for Rc<T> {
    fn government_interface(&self, id: GovtId) -> Result<Option<i16>, String> {
        (**self).government_interface(id)
    }

    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
        (**self).status_bar(id)
    }

    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        (**self).picture_size(id)
    }

    fn icon_size(&self, id: i16) -> Option<(u32, u32)> {
        (**self).icon_size(id)
    }
}

/// An effect's sprite sheet: its `rlëD`, how many frames it holds and
/// how wide each is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectSheet {
    /// The `rlëD`'s ID.
    pub image_id: i16,
    /// How many frames the sheet holds.
    pub frames: NonZeroU16,
    /// Each frame's width in pixels (`_GetSpriteXSize`): a shot's trail
    /// leaves half of it behind the shot.
    pub frame_width: u32,
}

/// A smoke set's sprite: the eight `cicn`s from
/// [`FIRST_SMOKE_ICON`] + 8 × set, which `_LoadSprites` makes its master
/// from (@0x21855-0x21907), for sets 0 and 1 only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SmokeSheet {
    /// The `cicn` of its first frame.
    pub first_icon: i16,
}

/// The `cicn` of smoke set 0's first frame (@0x2184b).
pub const FIRST_SMOKE_ICON: i16 = 1000;
/// The frames of a smoke set, one `cicn` each (@0x2186e).
pub const SMOKE_FRAMES: i16 = 8;
/// The smoke sets `_LoadSprites` makes a master for: 0 and 1 (the loop
/// ends at `cicn` 1016, @0x21901).
pub const SMOKE_SETS: u8 = 2;

impl SmokeSheet {
    /// The `cicn` of its frame `frame`.
    #[must_use]
    pub fn icon(self, frame: u8) -> i16 {
        self.first_icon + i16::from(frame)
    }
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
    /// `Falloff` as the original loads it
    /// ([`loaded_falloff`](nova_sim::combat::weapon::loaded_falloff)): how
    /// fast a beam's corona fades, 16 for a beam that says none, and none
    /// for a lightning weapon.
    pub falloff: i16,
    /// The shot fade's `Falloff` (weapon +0xb8): the loaded `Falloff` for
    /// a guidance other than 0, 3 and 10, none for a beam's
    /// (`_LoadObjectData` @0x796fa-0x79722; see
    /// the `shot_fade` module).
    pub shot_falloff: i16,
    /// `LiDensity` as loaded: 1 is 2 (`_LoadObjectData` @0x79625-0x7967c);
    /// above none for a lightning beam.
    pub li_density: i16,
    /// `LiAmplitude` as loaded: none or below is 1 (@0x79625-0x7967c).
    pub li_amplitude: i16,
    /// `Count`: a shot's life in ticks.
    pub count: i16,
    /// `Particles`: the trail's particles a tick, none for no trail.
    pub particles: i16,
    /// `PartVel`: the trail's speed, in hundredths of a pixel a tick.
    pub part_vel: i16,
    /// `PartLifeMin` and `PartLifeMax`: a trail particle's life, in ticks.
    pub part_life: (i16, i16),
    /// `PartColor`, `00RRGGBB`: the trail's colour.
    pub part_color: u32,
    /// `SmokeSet` as loaded: above 7, read unsigned, is 0
    /// (`_LoadObjectData` @0x79b8d-0x79b94).
    pub smoke_set: i16,
    /// `BeamColor`, `00RRGGBB`.
    pub beam_color: u32,
    /// `CoronaColor`, `00RRGGBB`.
    pub corona_color: u32,
    /// `ProxSafety`: the ticks a shot's frame waits before it spins, with
    /// `Flags2` 0x0001.
    pub prox_safety: i16,
    /// How long its sound lasts, in ticks rounded up; none when it has no
    /// sound or the sound cannot be decoded.
    pub sound_ticks: Option<u32>,
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
    /// The sheet of cargo box graphic `graphic`, `spïn` 500 + `graphic`,
    /// or why it cannot be read.
    fn box_sheet(&self, graphic: u16) -> Result<EffectSheet, String>;
    /// The master sheet of asteroid type `kind`, `spïn` 800 + `kind`, or
    /// why it cannot be read.
    fn asteroid_sheet(&self, kind: u8) -> Result<EffectSheet, String>;
    /// The mouse cursor's sheet, `spïn` 650 (`_InitSprites`
    /// @0x2147c-0x215eb), or why it cannot be read.
    fn cursor_sheet(&self) -> Result<EffectSheet, String>;
    /// The escape pods' sheet, the escape pod's ship sprite (shän 895,
    /// `nova_sim::ESCAPE_POD_SHIP`), its frames the rotations of one set,
    /// as `_InitSprites` clones the pods from class 0x2ff's master
    /// (@0x1fc90-0x1fc9e; see [`pods`](super::pods)); or why it cannot be
    /// read.
    fn pod_sheet(&self) -> Result<EffectSheet, String>;
    /// Smoke set `set`'s sprite ([`SmokeSheet`]): none for a set past 1,
    /// or when any of its eight `cicn`s is missing or does not decode.
    fn smoke_sheet(&self, set: u8) -> Option<SmokeSheet>;
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

    fn box_sheet(&self, graphic: u16) -> Result<EffectSheet, String> {
        (**self).box_sheet(graphic)
    }

    fn asteroid_sheet(&self, kind: u8) -> Result<EffectSheet, String> {
        (**self).asteroid_sheet(kind)
    }

    fn cursor_sheet(&self) -> Result<EffectSheet, String> {
        (**self).cursor_sheet()
    }

    fn pod_sheet(&self) -> Result<EffectSheet, String> {
        (**self).pod_sheet()
    }

    fn smoke_sheet(&self, set: u8) -> Option<SmokeSheet> {
        (**self).smoke_sheet(set)
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

    fn box_sheet(&self, graphic: u16) -> Result<EffectSheet, String> {
        (**self).box_sheet(graphic)
    }

    fn asteroid_sheet(&self, kind: u8) -> Result<EffectSheet, String> {
        (**self).asteroid_sheet(kind)
    }

    fn cursor_sheet(&self) -> Result<EffectSheet, String> {
        (**self).cursor_sheet()
    }

    fn pod_sheet(&self) -> Result<EffectSheet, String> {
        (**self).pod_sheet()
    }

    fn smoke_sheet(&self, set: u8) -> Option<SmokeSheet> {
        (**self).smoke_sheet(set)
    }
}

/// The first explosion type, `bööm` 128.
pub const FIRST_BOOM: i16 = 128;
/// The last explosion type, `bööm` 191: 64 in all.
pub const LAST_BOOM: i16 = 191;

/// The cargo box graphics read: a jettisoned box's, 0, and graphics 1-4,
/// the scoopable boxes a destroyed asteroid drops
/// (rdm:phase/asteroids/phase-2-shoot-and-break-asteroids), scooped up in
/// rdm:phase/asteroids/phase-4-scoopable-boxes.
pub const BOX_GRAPHICS: [u16; 5] = [0, 1, 2, 3, 4];

/// The weapons', explosions', cargo boxes' and asteroids' looks, read
/// once.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Looks {
    /// Each weapon's look, or why it cannot be read.
    pub weapons: BTreeMap<WeaponId, Result<WeaponLook, String>>,
    /// Each explosion type there is: its look, or why it cannot be read.
    pub booms: BTreeMap<BoomId, Result<BoomLook, String>>,
    /// Each cargo box graphic a box can have: its sheet, or why it cannot
    /// be read.
    pub boxes: BTreeMap<u16, Result<EffectSheet, String>>,
    /// Each asteroid type's master sheet, or why it cannot be read.
    pub asteroids: BTreeMap<u8, Result<EffectSheet, String>>,
    /// The mouse cursor's sheet, or why it cannot be read; `None` when it
    /// was not read.
    pub cursor: Option<Result<EffectSheet, String>>,
    /// The escape pods' sheet ([`CombatLooks::pod_sheet`]), or why it
    /// cannot be read; `None` when it was not read. A pod is not drawn
    /// without it.
    pub pod: Option<Result<EffectSheet, String>>,
    /// Each smoke set's sprite there is ([`SMOKE_SETS`]).
    pub smokes: BTreeMap<u8, SmokeSheet>,
}

impl Looks {
    /// The looks of `weapons`, of every explosion type, of the cargo
    /// boxes, of every asteroid type, of the cursor and of the escape
    /// pods, read from `catalog`.
    pub fn read(catalog: &impl CombatLooks, weapons: impl IntoIterator<Item = WeaponId>) -> Self {
        Self {
            weapons: weapons
                .into_iter()
                .map(|id| (id, catalog.weapon_look(id)))
                .collect(),
            booms: (FIRST_BOOM..=LAST_BOOM)
                .filter_map(|id| Some((BoomId(id), catalog.boom_look(BoomId(id))?)))
                .collect(),
            boxes: BOX_GRAPHICS
                .into_iter()
                .map(|graphic| (graphic, catalog.box_sheet(graphic)))
                .collect(),
            asteroids: (0..KINDS as u8)
                .map(|kind| (kind, catalog.asteroid_sheet(kind)))
                .collect(),
            cursor: Some(catalog.cursor_sheet()),
            pod: Some(catalog.pod_sheet()),
            smokes: (0..SMOKE_SETS)
                .filter_map(|set| Some((set, catalog.smoke_sheet(set)?)))
                .collect(),
        }
    }

    /// Smoke set `set`'s sprite, if there is one: only sets 0 and 1 have
    /// one, and only when their `cicn`s were read. A set is `SmokeSet` as
    /// loaded ([`WeaponLook::smoke_set`]).
    #[must_use]
    pub fn smoke_set(&self, set: i16) -> Option<SmokeSheet> {
        let set = u8::try_from(set).ok()?;
        self.smokes.get(&set).copied()
    }

    /// The escape pods' sheet, if it was read.
    #[must_use]
    pub fn pod_sheet(&self) -> Option<&EffectSheet> {
        self.pod.as_ref()?.as_ref().ok()
    }

    /// Cargo box graphic `graphic`'s sheet, if it was read.
    #[must_use]
    pub fn box_sheet(&self, graphic: u16) -> Option<&EffectSheet> {
        self.boxes.get(&graphic)?.as_ref().ok()
    }

    /// Asteroid type `kind`'s master sheet, if it was read.
    #[must_use]
    pub fn asteroid_sheet(&self, kind: u8) -> Option<&EffectSheet> {
        self.asteroids.get(&kind)?.as_ref().ok()
    }

    /// The mouse cursor's sheet, if it was read.
    #[must_use]
    pub fn cursor_sheet(&self) -> Option<&EffectSheet> {
        self.cursor.as_ref()?.as_ref().ok()
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
    /// `wëap`, `bööm` or cargo box graphic ("wëap 140: no spïn 3005");
    /// the weapons' by ID, then the explosions', then the boxes', then the
    /// cursor's ("cursor: no spïn 650"). An asteroid type's sheet is left
    /// out: a scenario may lack the types no system holds, and an asteroid
    /// whose sheet was not read draws as a crossed box.
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
        let boxes = self.boxes.iter().filter_map(|(graphic, sheet)| {
            sheet
                .as_ref()
                .err()
                .map(|reason| format!("cargo box {graphic}: {reason}"))
        });
        let cursor = match &self.cursor {
            Some(Err(reason)) => Some(format!("cursor: {reason}")),
            _ => None,
        };
        weapons.chain(booms).chain(boxes).chain(cursor).collect()
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
                weapon: None,
                weap_decay: 0,
                image_id: id.0,
                rotations: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 48,
                frame_height: 48,
                glow: None,
                lights: None,
                alt: None,
                shield: None,
                base_transp: 0,
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
        fn government_interface(&self, id: GovtId) -> Result<Option<i16>, String> {
            Ok(Some(id.0))
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
                cargo: none,
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

        fn icon_size(&self, id: i16) -> Option<(u32, u32)> {
            Some((1, u32::try_from(id).ok()?))
        }
    }

    /// Everything `catalog` says about govt 130, `ïntf` 131, `PICT` 194
    /// and `cicn` 18.
    fn bar(catalog: impl StatusBars) -> String {
        format!(
            "{:?} {:?} {:?} {:?}",
            catalog.government_interface(GovtId(130)),
            catalog.status_bar(131).map(|bar| bar.status_bkgnd),
            catalog.picture_size(194),
            catalog.icon_size(18),
        )
    }

    #[test]
    fn borrowed_and_shared_status_bars_are_status_bars() {
        let direct = "Ok(Some(130)) Ok(131) Some((194, 1)) Some((1, 18))";
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
                    frame_width: 0,
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

        fn box_sheet(&self, graphic: u16) -> Result<EffectSheet, String> {
            Ok(EffectSheet {
                frame_width: 0,
                image_id: 500 + graphic.cast_signed(),
                frames: NonZeroU16::MIN,
            })
        }

        fn asteroid_sheet(&self, kind: u8) -> Result<EffectSheet, String> {
            Ok(EffectSheet {
                frame_width: 0,
                image_id: 800 + i16::from(kind),
                frames: NonZeroU16::MIN,
            })
        }

        fn cursor_sheet(&self) -> Result<EffectSheet, String> {
            Ok(EffectSheet {
                frame_width: 0,
                image_id: 650,
                frames: NonZeroU16::MIN,
            })
        }

        fn pod_sheet(&self) -> Result<EffectSheet, String> {
            Ok(EffectSheet {
                image_id: 895,
                frames: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 0,
            })
        }

        fn smoke_sheet(&self, set: u8) -> Option<SmokeSheet> {
            Some(SmokeSheet {
                first_icon: FIRST_SMOKE_ICON + SMOKE_FRAMES * i16::from(set),
            })
        }
    }

    /// Everything `catalog` says about weapon 128, `bööm` 129, ship 130,
    /// govt 131, cargo box graphic 2, asteroid type 3 and smoke set 1.
    fn looks(catalog: impl CombatLooks) -> String {
        format!(
            "{:?} {:?} {:?} {:?} {:?} {:?} {:?}",
            catalog.weapon_look(WeaponId(128)).map(|look| look.name),
            catalog
                .boom_look(BoomId(129))
                .map(|look| look.map(|look| look.sheet.map(|sheet| sheet.image_id))),
            catalog.target_card(ShipId(130)).subtitle,
            catalog.target_code(GovtId(131)),
            catalog.box_sheet(2).map(|sheet| sheet.image_id),
            catalog.asteroid_sheet(3).map(|sheet| sheet.image_id),
            catalog.smoke_sheet(1).map(|sheet| sheet.first_icon),
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
                frame_width: 0,
                image_id: 150,
                frames: NonZeroU16::MIN
            }))
        );
        assert_eq!(looks.boom(BoomId(151)), None);
        assert_eq!(
            looks.box_sheet(0),
            Some(&EffectSheet {
                frame_width: 0,
                image_id: 500,
                frames: NonZeroU16::MIN
            }),
            "a jettisoned box's"
        );
        let boxes: Vec<_> = looks.boxes.keys().copied().collect();
        assert_eq!(boxes, [0, 1, 2, 3, 4], "and the asteroids' 1-4");
        assert_eq!(
            looks.box_sheet(4),
            Some(&EffectSheet {
                frame_width: 0,
                image_id: 504,
                frames: NonZeroU16::MIN
            })
        );
        assert_eq!(looks.box_sheet(5), None);
        assert_eq!(
            looks.cursor_sheet(),
            Some(&EffectSheet {
                frame_width: 0,
                image_id: 650,
                frames: NonZeroU16::MIN
            }),
            "the cursor's"
        );
        let asteroids: Vec<_> = looks.asteroids.keys().copied().collect();
        assert_eq!(asteroids, (0..16).collect::<Vec<u8>>(), "every type's");
        assert_eq!(
            looks.asteroid_sheet(15),
            Some(&EffectSheet {
                frame_width: 0,
                image_id: 815,
                frames: NonZeroU16::MIN
            })
        );
        assert_eq!(looks.asteroid_sheet(16), None);
        assert_eq!(
            looks.pod_sheet(),
            Some(&EffectSheet {
                image_id: 895,
                frames: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 0,
            }),
            "the escape pods'"
        );
        let smokes: Vec<_> = looks.smokes.keys().copied().collect();
        assert_eq!(smokes, [0, 1], "only sets 0 and 1 are asked for");
        assert_eq!(looks.smoke_set(1), Some(SmokeSheet { first_icon: 1008 }));
        assert_eq!(looks.smoke_set(2), None);
        assert_eq!(looks.smoke_set(-1), None);
        assert_eq!(SmokeSheet { first_icon: 1008 }.icon(7), 1015);
        assert_eq!(looks.problems(), Vec::<String>::new());
        let unread = Looks {
            smokes: BTreeMap::default(),
            weapons: BTreeMap::from([(WeaponId(1), Err("no".to_owned()))]),
            booms: BTreeMap::from([(BoomId(1), Err("no".to_owned()))]),
            boxes: BTreeMap::from([(0, Err("no".to_owned()))]),
            asteroids: BTreeMap::from([(0, Err("no".to_owned()))]),
            cursor: Some(Err("no".to_owned())),
            pod: Some(Err("no".to_owned())),
        };
        assert_eq!(unread.asteroid_sheet(0), None);
        assert_eq!(unread.weapon(WeaponId(1)), None);
        assert_eq!(unread.boom(BoomId(1)), None);
        assert_eq!(unread.box_sheet(0), None);
        assert_eq!(unread.cursor_sheet(), None);
        assert_eq!(Looks::default().cursor_sheet(), None, "not read");
        assert_eq!(unread.pod_sheet(), None);
        assert_eq!(Looks::default().pod_sheet(), None, "not read");
    }

    #[test]
    fn borrowed_and_shared_combat_looks_are_combat_looks() {
        let direct =
            r#"Ok("w128") Some(Ok(Ok(129))) "s130" Some("g131") Ok(502) Ok(803) Some(1008)"#;
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

        fn box_sheet(&self, graphic: u16) -> Result<EffectSheet, String> {
            Err(format!("no spïn {}", 500 + graphic))
        }

        fn asteroid_sheet(&self, kind: u8) -> Result<EffectSheet, String> {
            Err(format!("no spïn {}", 800 + u16::from(kind)))
        }

        fn cursor_sheet(&self) -> Result<EffectSheet, String> {
            Err("no spïn 650".to_owned())
        }

        fn pod_sheet(&self) -> Result<EffectSheet, String> {
            Err("no shän 895".to_owned())
        }

        fn smoke_sheet(&self, _set: u8) -> Option<SmokeSheet> {
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
        assert_eq!(looks.smoke_set(0), None, "no smoke sprite");
        assert_eq!(
            looks.problems(),
            [
                "wëap 140: no spïn 3005",
                "no wëap 141",
                "bööm 128: no spïn 400",
                "bööm 129: too short",
                "cargo box 0: no spïn 500",
                "cargo box 1: no spïn 501",
                "cargo box 2: no spïn 502",
                "cargo box 3: no spïn 503",
                "cargo box 4: no spïn 504",
                "cursor: no spïn 650",
            ]
        );
    }
}
