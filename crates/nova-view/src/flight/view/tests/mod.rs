//! The flight view's tests. Its component tests run on [`FakeFlight`],
//! each module over scripted frames ([`frames`]), answers and command
//! assertions. This module holds what they share: [`FakeCatalog`] and its
//! data builders, the fake-based helpers ([`Faked`], [`faked`], [`sent`],
//! the `faked_map_*` helpers, ...) and the helpers generic over any
//! [`FlightSim`]. A real `nova_sim::Session` is started only by the
//! adapter's checks (`adapter`) and the edge's (`edge`), and, outside
//! this module, by `tests/it/flight_seam.rs` and the stock tests
//! (rdm:phase/view-sim-seam/phase-14-view-tests-on-port).

use std::cell::RefCell;
use std::num::NonZeroU16;

use crate::system::camera::{VIEW_CENTER, VIEW_SIZE};
use nova_sim::Session;
use nova_sim::flight_frame::GateFade;
use nova_sim::landing::StellarFlags;
use nova_sim::{
    Behaviour, CharacterStart, CommodityStrings, DateAffixes, DisableRule, DisasterRecord,
    GateSite, JunkRecord, LandingSite, OutfitId, OutfitRecord, Reserves, ShipFields, ShipId,
    ShipRecord, ShipStats, SimSound, StarSystem, StartDate, StartError, SystemId, TICK, Vec2,
};

use super::*;
use crate::devtools::{PilotDesk, PilotEdit};
use crate::draw::crossed_box;
use crate::draw::fill_command;
use crate::draw::lights_tint;
use crate::flight::cadence::{NAV_PERIOD_TICKS, TARGET_PERIOD_TICKS};
use crate::flight::catalog::{
    Blink, BoomLook, GovtId, LayerSheet, StarSheet, StatusBarLayout, TargetCard, WeaponLook,
};
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::escorts::{EscortMenuLooks, GROUP_KEYS, MENU_KEY, NO_ESCORTS};
use crate::flight::hud::{self, HudState, NavDisplay, StatusBar};
use crate::flight::jump::JumpPhase;
use crate::flight::radar::RadarStellar;
use crate::flight::radar::{self, IRect, RadarColor, RadarMark};
use crate::flight::sim_port::FlightSim;
use crate::flight::sim_port::fixture::{FakeFlight, record};
use crate::flight::sprite::rotation_frame;
use crate::flight::target::Tag;
use crate::flight::weapons;
use crate::flight::{
    Collaborators, FlightStart, SessionFlight, begin_game, enter_game, open_flight, test_flight,
};
use crate::galaxy::GalaxyCatalog;
use crate::galaxy::{Galaxy, MapMode, SystemEntry};
use crate::sound::Sound;
use crate::spaceport::PortDialog;
use crate::system::scene::PLACEHOLDER;
use crate::system::{scene, starfield};
use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::{DrawList, ImageKey, Input, MouseButton, Screen, ScreenAction};
use nova_sim::CommCatalog;
use nova_sim::SessionSetup;
use nova_sim::ShipIff;
use nova_sim::fixture::SessionBuilder;
use nova_sim::flight_frame::{Answer, FlightCommand, FlightFrame, MapClick};
use nova_sim::mission::screens::NO_ACTIVE_MISSIONS;
use nova_sim::{
    ALERT_CICN, AbortError, AcceptError, Assigned, Assignment, AsteroidCatalog, CombatCatalog,
    ComputerRefusal, Controls, FighterNote, MissionId, MissionShown, NotOffered, Pilot,
    PilotCatalog, PlayerCommand, Rulebook, Rules, StellarIff, StellarPick, Take, Taken,
    TrafficCatalog, Turn, VenueOffer,
};

use crate::system::catalog::{
    AnimationData, StellarContents, StellarId, StellarSheet, SystemContents,
};
use crate::{Blend, DrawCommand, Font};
use nova_sim::hyperspace::MIN_JUMP_DISTANCE;
use nova_sim::{BoardRefusal, DockKind, MissionBoardLine};
use nova_sim::{GLOW_CRUISE, glow_level};
use nova_sim::{RuleKey, RuleSource};

mod adapter;
mod anchor;
mod boarding;
mod cloak;
mod combat;
mod comm_line;
mod controls;
mod dialog_map;
mod edge;
mod escorts;
mod faked_board;
mod faked_combat;
mod faked_desk;
mod faked_docked;
mod faked_escorts;
mod faked_flight;
mod faked_hail;
mod faked_hud;
mod faked_keys;
mod faked_landing;
mod faked_missions;
mod faked_open;
mod faked_player;
mod faked_screen;
mod faked_shops;
mod faked_travel;
mod faked_wording;
mod faked_world;
mod flying;
mod frames;
mod hyperspace;
mod kept_frame;
mod landing;
mod layers;
mod map_borders;
mod map_find;
mod maps;
mod missions;
mod nav;
mod outputs;
mod port_screen_map;
mod ship_drawing;
mod shots;
mod stars;
mod stellar_looks;
mod stellars;
mod stepping;
mod widescreen;
mod world_layers;

use frames::*;

/// The first `chär` flies ship 128 (an average ship that turns 3° a
/// tick, with a 36-rotation, 40 x 40 sheet, `rlëD` 2000) from system
/// 130, Sol: Earth at (0, -600) and Moon at (300, -200), which animates
/// a frame a tick. On the map Sol is at (0, 0), linked both ways to Alpha
/// Centauri (131) at (600, 0), which holds Proxima at its centre;
/// Barnard (132) at (0, 600) is linked to nothing, unless `onward`.
/// Records the systems read.
// Each flag adds a system or a link of its own, read on its own.
#[allow(clippy::struct_excessive_bools)]
struct FakeCatalog {
    character: Result<CharacterStart, StartError>,
    fields: ShipFields,
    sheet: Result<ShipSheet, String>,
    /// `ïntf` 128: stock-like, or why it cannot be read.
    bar: Result<StatusBarLayout, String>,
    /// `cicn` 18000's size: 24 x 24, by default.
    alert_icon: Option<(u32, u32)>,
    /// Each `gövt`'s `Interface`: none, by default.
    interfaces: Vec<(i16, i16)>,
    /// The `ïntf`s past 128: none, by default.
    other_bars: Vec<(i16, StatusBarLayout)>,
    /// The `ïntf`s asked for.
    bars_asked: RefCell<Vec<i16>>,
    /// System 130's landing sites: Earth and Moon, by default.
    sites: Vec<LandingSite>,
    /// System 130's `NavDefs` slots on the map: none, by default.
    sol_navs: Vec<Option<(u32, u16)>>,
    /// System 130's `Interference`: none, by default.
    sol_interference: i16,
    /// System 130's `Murk`: none, by default.
    sol_murk: i16,
    /// System 130's `BkgndColor`: black, by default.
    sol_background: u32,
    /// System 131's `Message`: no buoy (−1), by default.
    alpha_buoy: i16,
    /// The escorts' voices: none, by default.
    voices: nova_sim::voice::Voices,
    /// The stellars Sol lists after Earth and the Moon: none, by
    /// default.
    sol_more: Vec<StellarContents>,
    /// How many frames Proxima's (140's) `Type` sheet has in Alpha
    /// Centauri: 1, by default.
    proxima_frames: u16,
    systems_read: RefCell<Vec<SystemId>>,
    /// The `dësc`s: none, by default.
    descriptions: Vec<(i16, String)>,
    /// The movies the `dësc`s name: none, by default.
    desc_movies: Vec<(i16, nova_sim::DescMovie)>,
    /// The commodities: none, by default.
    commodities: CommodityStrings,
    /// The planetary events: none, by default.
    disasters: Vec<DisasterRecord>,
    /// The outfits: none, by default.
    outfits: Vec<OutfitRecord>,
    /// The `jünk`s: none, by default.
    junk: Vec<JunkRecord>,
    /// The ship classes the shipyard reads: none, by default.
    ships: Vec<ShipRecord>,
    /// The ships whose sheets were asked for.
    sheets_asked: RefCell<Vec<ShipId>>,
    /// Each batch of ships whose target cards were asked for.
    cards_asked: RefCell<Vec<Vec<ShipId>>>,
    /// Each system's traffic: none, by default.
    traffic: Vec<(SystemId, nova_sim::SystemTraffic)>,
    /// The düdes: none, by default.
    dudes: Vec<(nova_sim::DudeId, nova_sim::DudeRecord)>,
    /// The weapons: none, by default.
    weapons: Vec<nova_sim::WeaponRecord>,
    /// The ship types' combat fields: none, by default.
    hulls: Vec<nova_sim::HullRecord>,
    /// The governments: none, by default.
    govts: Vec<nova_sim::GovtRecord>,
    /// The ranks: none, by default.
    ranks: Vec<nova_sim::catalog::RankRecord>,
    /// The weapons' looks: none, by default.
    looks: Vec<(i16, Result<WeaponLook, String>)>,
    /// The explosions' looks: none, by default.
    booms: Vec<(i16, Result<BoomLook, String>)>,
    /// The ship types' target cards: none, by default.
    cards: Vec<(i16, TargetCard)>,
    /// The governments' target codes: none, by default.
    codes: Vec<(i16, String)>,
    /// The jettisoned cargo box's sheet: `rlëD` 500 of 36 frames, by
    /// default.
    box_sheet: Result<EffectSheet, String>,
    /// The mouse cursor's sheet: `rlëD` 651 of 8 frames, by default.
    cursor_sheet: Result<EffectSheet, String>,
    /// The stars' sheet: none, by default, so no star is drawn
    /// ([`star_sheet`] draws them).
    star_sheet: Result<StarSheet, String>,
    /// The asteroid types: none, by default.
    asteroid_types: Vec<nova_sim::AsteroidTypeRecord>,
    /// Each system's asteroid field: none, by default.
    asteroid_fields: Vec<nova_sim::AsteroidFieldRecord>,
    /// The string lists: none, by default.
    strings: Vec<(i16, Vec<String>)>,
    /// The persons: none, by default.
    persons: Vec<nova_sim::PersonRecord>,
    /// Ship 128's default items: none, by default.
    defaults: Vec<(OutfitId, u16)>,
    /// Ship 129's sheet in place of the one [`FakeCatalog::ship_sheet`]
    /// gives: none, by default.
    npc_sheet: Option<ShipSheet>,
    /// Whether Alpha Centauri links on to Barnard, which has no
    /// stellars: not by default.
    onward: bool,
    /// Whether Sol lists Barnard too, after Alpha Centauri, so that
    /// Hyper Select has two systems to cycle through: not by default.
    fan: bool,
    /// Whether Proxima Near (134) exists at (40, 30), close enough to
    /// Sol to show on the floating map, listed by Sol after its other
    /// links and linking back, with no stellars: not by default.
    near: bool,
    /// Every stellar a gate may lead to: none, by default. Behind a
    /// `RefCell`, so a test can change a gate's links behind a view
    /// already built, which shares the catalog with its flight.
    gates: RefCell<Vec<GateSite>>,
    /// The gates each system lists beyond its stellars: none, by
    /// default.
    gate_listers: Vec<(SystemId, StellarId)>,
    /// Whether Alpha Reborn (133) exists, at Alpha Centauri's place,
    /// with no links and Proxima Reborn (141) at its centre: not by
    /// default.
    reborn: bool,
    /// Whether system 128 exists, the escape pod's rescue's fallback,
    /// with no links and no stellars: not by default.
    home: bool,
    /// Whether the map's catalog has nebula 128, (−20, −20) 60 × 60 over
    /// Sol, with a 30 × 30 picture in slot 0 (`PICT` 9500) and a
    /// 120 × 120 one in slot 2 (9502): not by default. The session's
    /// catalog has no `nëbu`, so it is active, and unexplored until a
    /// test explores it.
    nebula: bool,
    /// The stellars that can be destroyed and regenerated: none, by
    /// default.
    lives: Vec<nova_sim::StellarLife>,
    /// The Moon's (129) sheets and graphic type 0's, given to the sim
    /// and the view alike: the defaults, unless set.
    moon: Option<MoonSheets>,
    /// Why the Moon's (129) `Type` sheet cannot be read, in the sim and
    /// the view alike: it can, by default.
    moon_unreadable: Option<&'static str>,
    /// The missions: none, by default.
    missions: Vec<nova_sim::catalog::MissionRecord>,
    /// What the mission screens show of each: none, by default.
    mission_texts: Vec<nova_sim::catalog::MissionTexts>,
    /// Each system's `Visibility`: none, by default, so every system
    /// is visible.
    visibility: Vec<nova_sim::catalog::SystemVisibility>,
    /// The session's `spöb` and `sÿst` names: none, by default.
    place_names: nova_sim::catalog::PlaceNames,
    /// The `crön`s: none, by default.
    crons: Vec<nova_sim::catalog::CronRecord>,
    /// Each `bööm`'s sound, as the session reads it: none, by default.
    boom_sounds: Vec<(nova_sim::BoomId, nova_sim::SoundId)>,
    /// Each system's government, in the session's map and the course
    /// map's catalog: none, by default.
    governed: Vec<(i16, i16)>,
    /// The systems besides Sol with a lived-in `NavDefs` slot on the
    /// map: none, by default.
    lived: Vec<i16>,
}

/// The sizes of the Moon's (129) `Type` sheet (1129) and `DeadType`
/// sheet (2129, when there is one), and of graphic type 0's sheet,
/// each square; the frames of its `Type` and `DeadType` sheets; and
/// its `Flags2` (0x0080 to animate only when destroyed).
#[derive(Clone, Copy, Debug)]
struct MoonSheets {
    width: u32,
    dead_width: Option<u32>,
    zero_width: u32,
    frames: u16,
    dead_frames: u16,
    flags2: u16,
}

/// No strings unless a test sets them.
impl nova_sim::CommCatalog for FakeCatalog {
    fn string_list(&self, id: i16) -> Vec<String> {
        self.strings
            .iter()
            .find(|(list, _)| *list == id)
            .map_or_else(Vec::new, |(_, strings)| strings.clone())
    }
}

impl FakeCatalog {
    /// System `id`'s government, as [`FakeCatalog::governed`] says.
    fn govt_of(&self, id: i16) -> Option<GovtId> {
        self.governed
            .iter()
            .find(|&&(system, _)| system == id)
            .map(|&(_, govt)| GovtId(govt))
    }

    /// Sol's links: Alpha Centauri, then Barnard with `fan`, then
    /// Proxima Near with `near`.
    fn sol_links(&self) -> Vec<i16> {
        let mut links = vec![131];
        if self.fan {
            links.push(132);
        }
        if self.near {
            links.push(134);
        }
        links
    }
}

const FIELDS: ShipFields = ShipFields {
    speed: 300,
    accel: 300,
    maneuver: 30,
    shield: 40,
    armor: 60,
    fuel: 250,
    fuel_regen: 0,
    holds: 0,
    mass: 15,
    free_mass: 8,
    contribute: 1,
    shield_rech: 0,
    armor_rech: 0,
    flags: 0,
    flags2: 0,
    ionize_max: 0,
    deionize: 0,
};

fn sheet() -> ShipSheet {
    ShipSheet {
        weapon: None,
        weap_decay: 0,
        image_id: 2000,
        rotations: NonZeroU16::new(36).expect("non-zero"),
        frame_width: 40,
        frame_height: 40,
        glow: None,
        lights: None,
        alt: None,
        shield: None,
        base_transp: 0,
        solid: false,
    }
}

fn catalog() -> FakeCatalog {
    FakeCatalog {
        character: Ok(CharacterStart {
            ship: Some(ShipId(128)),
            systems: [None, Some(SystemId(130)), None, None],
            start: StartDate {
                day: 23,
                month: 6,
                year: 1177,
            },
            ..CharacterStart::default()
        }),
        fields: FIELDS,
        sheet: Ok(sheet()),
        bar: Ok(layout()),
        alert_icon: Some((24, 24)),
        interfaces: Vec::new(),
        other_bars: Vec::new(),
        bars_asked: RefCell::default(),
        sites: vec![
            site(128, (0.0, -600.0), StellarFlags::CAN_LAND),
            site(129, (300.0, -200.0), StellarFlags::CAN_LAND),
        ],
        systems_read: RefCell::default(),
        descriptions: Vec::new(),
        desc_movies: Vec::new(),
        commodities: CommodityStrings::default(),
        disasters: Vec::new(),
        outfits: Vec::new(),
        junk: Vec::new(),
        ships: Vec::new(),
        sheets_asked: RefCell::default(),
        cards_asked: RefCell::default(),
        traffic: Vec::new(),
        dudes: Vec::new(),
        weapons: Vec::new(),
        hulls: vec![hull_of(128, &[]), hull_of(129, &[]), hull_of(130, &[])],
        govts: Vec::new(),
        ranks: Vec::new(),
        looks: Vec::new(),
        booms: Vec::new(),
        cards: Vec::new(),
        codes: Vec::new(),
        box_sheet: Ok(EffectSheet {
            frame_width: 0,
            image: crate::SheetImage::Rled(500),
            frames: NonZeroU16::new(36).expect("non-zero"),
        }),
        cursor_sheet: Ok(EffectSheet {
            frame_width: 0,
            image: crate::SheetImage::Rled(651),
            frames: NonZeroU16::new(8).expect("non-zero"),
        }),
        star_sheet: Err("no spïn 700".to_owned()),
        asteroid_types: Vec::new(),
        asteroid_fields: Vec::new(),
        strings: Vec::new(),
        persons: Vec::new(),
        defaults: Vec::new(),
        onward: false,
        npc_sheet: None,
        fan: false,
        near: false,
        reborn: false,
        home: false,
        nebula: false,
        sol_navs: Vec::new(),
        sol_interference: 0,
        sol_murk: 0,
        sol_background: 0,
        alpha_buoy: -1,
        voices: nova_sim::voice::Voices::default(),
        sol_more: Vec::new(),
        proxima_frames: 1,
        gates: RefCell::new(Vec::new()),
        gate_listers: Vec::new(),
        lives: Vec::new(),
        moon: None,
        moon_unreadable: None,
        missions: Vec::new(),
        mission_texts: Vec::new(),
        visibility: Vec::new(),
        place_names: nova_sim::catalog::PlaceNames::default(),
        crons: Vec::new(),
        boom_sounds: Vec::new(),
        governed: Vec::new(),
        lived: Vec::new(),
    }
}

impl FakeCatalog {
    /// System `system`'s landing sites, before any `DeadType`.
    fn system_sites(&self, system: SystemId) -> Vec<LandingSite> {
        match system.0 {
            130 => {
                let mut sites = self.sites.clone();
                if let Some(moon) = self.moon {
                    for site in sites.iter_mut().filter(|site| site.id == StellarId(129)) {
                        site.frame_size = Some((moon.width, moon.width));
                        site.dead_graphic = moon.dead_width.map(|_| 9);
                        site.dead_frame_size = moon.dead_width.map(|w| (w, w));
                    }
                }
                if self.moon_unreadable.is_some() {
                    for site in sites.iter_mut().filter(|site| site.id == StellarId(129)) {
                        site.frame_size = None;
                    }
                }
                sites
            }
            131 => vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)],
            132 if self.onward || self.fan => Vec::new(),
            133 if self.reborn => vec![site(141, (0.0, 0.0), StellarFlags::CAN_LAND)],
            128 if self.home => Vec::new(),
            134 if self.near => Vec::new(),
            other => panic!("asked for sÿst {other}'s sites"),
        }
    }
}

/// Stellar `id` at `(x, y)`, 20 x 20 (radius 10), with `flags`.
fn site(id: i16, (x, y): (f32, f32), flags: u32) -> LandingSite {
    LandingSite {
        id: StellarId(id),
        position: Vec2::new(x, y),
        frame_size: Some((20, 20)),
        flags,
        min_status: 0,
        tech_level: 1,
        special_tech: [0; 8],
        govt: None,
        flags2: 0,
        destroyed: false,
        graphic: 0,
        dead_graphic: None,
        dead_frame_size: None,
        type_counts_frames: true,
        dead_counts_frames: true,
        exit_angle: -1,
        tribute: 0,
        gravity: 0,
        fee: 0,
        animation: nova_sim::catalog::SiteAnimation::default(),
    }
}

/// The start's date, with stock's affixes.
const DATE: &str = "June 23rd, 1177 NC";

/// Stock `ïntf` 128's areas, with background `PICT` 700.
fn layout() -> StatusBarLayout {
    let rect = |left, top, right, bottom| crate::geometry::Bounds {
        min: at(left, top),
        max: at(right, bottom),
    };
    StatusBarLayout {
        radar: rect(8.0, 8.0, 184.0, 184.0),
        shield: rect(35.0, 199.0, 184.0, 206.0),
        armor: rect(35.0, 216.0, 184.0, 223.0),
        fuel: rect(35.0, 234.0, 184.0, 241.0),
        nav: rect(8.0, 254.0, 184.0, 286.0),
        weap: rect(8.0, 300.0, 184.0, 315.0),
        targ: rect(8.0, 330.0, 184.0, 442.0),
        cargo: rect(8.0, 458.0, 184.0, 552.0),
        bright_text: Color::WHITE,
        dim_text: Color::DIM,
        bright_radar: Color::rgba(0, 255, 0, 255),
        dim_radar: Color::rgba(0, 128, 0, 255),
        shield_color: Color::rgba(0, 0, 255, 255),
        armor_color: Color::rgba(255, 0, 0, 255),
        fuel_full: Color::rgba(255, 255, 0, 255),
        fuel_partial: Color::rgba(128, 128, 0, 255),
        font: Font::Geneva,
        font_size: 12.0,
        subtitle_size: 10.0,
        status_bkgnd: 700,
    }
}

impl PilotCatalog for FakeCatalog {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        self.character.clone()
    }

    fn voices(&self) -> nova_sim::voice::Voices {
        self.voices.clone()
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        assert_eq!(id, ShipId(128));
        Ok(self.fields)
    }

    fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
        assert_eq!(id, ShipId(128));
        self.defaults.clone()
    }

    fn outfits(&self) -> Vec<OutfitRecord> {
        self.outfits.clone()
    }

    fn ships(&self) -> Vec<ShipRecord> {
        self.ships.clone()
    }

    /// Sol and Alpha Centauri; Barnard, which has no stellars, only
    /// when Alpha Centauri links on to it.
    fn system_exists(&self, id: SystemId) -> bool {
        matches!(id.0, 130 | 131)
            || (id.0 == 132 && self.onward)
            || (id.0 == 133 && self.reborn)
            || (id.0 == 128 && self.home)
            || (id.0 == 134 && self.near)
    }

    /// Earth (128) and Proxima (140) have the `DeadType` sheet the
    /// view draws for them ([`dying`]).
    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
        let mut sites = self.system_sites(system);
        for site in sites
            .iter_mut()
            .filter(|site| matches!(site.id.0, 128 | 140))
        {
            site.dead_graphic = Some(site.id.0 - 100);
            site.dead_frame_size = Some((30, 30));
        }
        sites
    }

    fn star_map(&self) -> Vec<StarSystem> {
        let star = |id, (x, y), links: &[i16]| StarSystem {
            id: SystemId(id),
            position: Vec2::new(x, y),
            links: links.iter().copied().map(SystemId).collect(),
            govt: None,
            stellars: Vec::new(),
            interference: 0,
            murk: 0,
            message: -1,
            background: 0,
        };
        let onward: &[i16] = if self.onward { &[130, 132] } else { &[130] };
        let sol = self.sol_links();
        let mut systems = vec![
            StarSystem {
                stellars: self.sol_navs.clone(),
                interference: self.sol_interference,
                murk: self.sol_murk,
                background: self.sol_background,
                ..star(130, (0.0, 0.0), &sol)
            },
            StarSystem {
                message: self.alpha_buoy,
                ..star(131, (600.0, 0.0), onward)
            },
            star(132, (0.0, 600.0), &[]),
        ];
        if self.reborn {
            systems.push(star(133, (600.0, 0.0), &[]));
        }
        if self.home {
            systems.push(star(128, (-600.0, 0.0), &[]));
        }
        if self.near {
            systems.push(star(134, (40.0, 30.0), &[130]));
        }
        for system in &mut systems {
            system.govt = self.govt_of(system.id.0);
            if self.lived.contains(&system.id.0) {
                system.stellars = vec![Some((StellarFlags::CAN_LAND, 0))];
            }
        }
        systems
    }

    fn commodity_strings(&self) -> CommodityStrings {
        self.commodities.clone()
    }

    fn string_patch(&self, _id: i16) -> nova_sim::catalog::StringPatch {
        nova_sim::catalog::StringPatch::Absent
    }

    /// [`FakeCatalog::strings`]' `STR#` 1000.
    fn buoy_strings(&self) -> Vec<String> {
        self.string_list(nova_sim::buoy::BUOY_STRINGS)
    }

    fn junk(&self) -> Vec<JunkRecord> {
        self.junk.clone()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        self.disasters.clone()
    }

    fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
        Vec::new()
    }

    fn stellar_weapons(&self) -> Vec<(StellarId, nova_sim::WeaponId)> {
        Vec::new()
    }

    /// Stock's: no prefix, and " NC".
    fn date_affixes(&self) -> DateAffixes {
        DateAffixes {
            prefix: String::new(),
            suffix: " NC".to_owned(),
        }
    }

    fn gate_sites(&self) -> Vec<GateSite> {
        self.gates.borrow().clone()
    }

    fn stellar_lives(&self) -> Vec<nova_sim::StellarLife> {
        self.lives.clone()
    }

    fn fallback_frame_size(&self) -> Option<(u32, u32)> {
        Some(
            self.moon
                .map_or((100, 100), |moon| (moon.zero_width, moon.zero_width)),
        )
    }

    fn missions(&self) -> Vec<nova_sim::catalog::MissionRecord> {
        self.missions.clone()
    }

    fn mission_texts(&self) -> Vec<nova_sim::catalog::MissionTexts> {
        self.mission_texts.clone()
    }

    /// Every stellar of each system's [`landing_sites`], with every
    /// system listing it, and the gates of `gate_listers`.
    fn listed_stellars(&self) -> Vec<nova_sim::catalog::ListedStellar> {
        let mut systems = vec![SystemId(130), SystemId(131)];
        if self.onward {
            systems.push(SystemId(132));
        }
        if self.reborn {
            systems.push(SystemId(133));
        }
        if self.near {
            systems.push(SystemId(134));
        }
        let sites: Vec<(SystemId, Vec<LandingSite>)> = systems
            .into_iter()
            .map(|system| (system, self.landing_sites(system)))
            .collect();
        let mut listed = nova_sim::fixture::listed_stellars(
            sites
                .iter()
                .map(|(system, sites)| (*system, sites.as_slice())),
        );
        nova_sim::fixture::list_also(&mut listed, self.gate_listers.iter().copied());
        listed
    }

    fn system_visibility(&self) -> Vec<nova_sim::catalog::SystemVisibility> {
        self.visibility.clone()
    }

    fn place_names(&self) -> nova_sim::catalog::PlaceNames {
        self.place_names.clone()
    }

    fn stellar_marks(&self) -> Vec<nova_sim::catalog::StellarMark> {
        Vec::new()
    }

    fn crons(&self) -> Vec<nova_sim::catalog::CronRecord> {
        self.crons.clone()
    }

    fn boom_sounds(&self) -> Vec<(nova_sim::BoomId, nova_sim::SoundId)> {
        self.boom_sounds.clone()
    }

    fn ranks(&self) -> Vec<nova_sim::catalog::RankRecord> {
        self.ranks.clone()
    }

    fn nebulae(&self) -> Vec<nova_sim::catalog::NebulaRecord> {
        Vec::new()
    }
}

/// The weapons, ship types' combat fields and governments given.
impl CombatCatalog for FakeCatalog {
    fn weapons(&self) -> Vec<nova_sim::WeaponRecord> {
        self.weapons.clone()
    }

    fn hulls(&self) -> Vec<nova_sim::HullRecord> {
        self.hulls.clone()
    }

    fn governments(&self) -> Vec<nova_sim::GovtRecord> {
        self.govts.clone()
    }
}

/// The asteroid types and fields given.
impl AsteroidCatalog for FakeCatalog {
    fn asteroid_types(&self) -> Vec<nova_sim::AsteroidTypeRecord> {
        self.asteroid_types.clone()
    }

    fn asteroid_fields(&self) -> Vec<nova_sim::AsteroidFieldRecord> {
        self.asteroid_fields.clone()
    }

    fn box_frame_sizes(&self) -> [Option<(u32, u32)>; nova_sim::BOX_GRAPHICS] {
        [None; nova_sim::BOX_GRAPHICS]
    }
}

/// The traffic, düdes and persons given; no fleets.
impl TrafficCatalog for FakeCatalog {
    fn system_traffic(&self, id: SystemId) -> Option<nova_sim::SystemTraffic> {
        self.traffic
            .iter()
            .find(|(system, _)| *system == id)
            .map(|(_, traffic)| *traffic)
    }

    fn dude(&self, id: nova_sim::DudeId) -> Option<nova_sim::DudeRecord> {
        self.dudes
            .iter()
            .find(|(dude, _)| *dude == id)
            .map(|(_, record)| record.clone())
    }

    fn fleets(&self) -> Vec<nova_sim::FleetRecord> {
        Vec::new()
    }

    fn persons(&self) -> Vec<nova_sim::PersonRecord> {
        self.persons.clone()
    }
}

fn stellar(id: i16, name: &str, (x, y): (i16, i16), frames: u16) -> StellarContents {
    StellarContents {
        id: StellarId(id),
        name: name.to_owned(),
        x,
        y,
        sprite: Ok(StellarSheet {
            image: crate::SheetImage::Rled(1000 + id),
            frames: NonZeroU16::new(frames).expect("non-zero"),
            frame_width: 20,
            frame_height: 20,
        }),
        dead_sprite: None,
        animation: AnimationData {
            delay: 1,
            frame0_bias: 0,
            flags2: 0,
        },
        radar_square: false,
    }
}

/// Graphic type 0's sheet (`spïn` 1000) in every system: one 40 x 40
/// frame.
const TYPE_ZERO: i16 = 3000;

/// `stellar`, drawing `DeadType` sheet 2000 + `id` (one 30 x 30 frame)
/// while destroyed.
fn dying(id: i16, name: &str, at: (i16, i16), frames: u16) -> StellarContents {
    StellarContents {
        dead_sprite: Some(Ok(StellarSheet {
            image: crate::SheetImage::Rled(2000 + id),
            frames: NonZeroU16::MIN,
            frame_width: 30,
            frame_height: 30,
        })),
        ..stellar(id, name, at, frames)
    }
}

impl SystemCatalog for FakeCatalog {
    fn system(&self, id: SystemId) -> SystemContents {
        self.systems_read.borrow_mut().push(id);
        let (name, stellars) = match id.0 {
            130 => (
                "Sol",
                [
                    dying(128, "Earth", (0, -600), 1),
                    stellar(129, "Moon", (300, -200), 4),
                ]
                .into_iter()
                .chain(self.sol_more.iter().cloned())
                .collect(),
            ),
            131 => (
                "Alpha Centauri",
                vec![dying(140, "Proxima", (0, 0), self.proxima_frames)],
            ),
            // Read without `onward` too: a scripted flight's frame lays
            // Barnard out whatever its links.
            132 => ("Barnard", Vec::new()),
            133 if self.reborn => (
                "Alpha Reborn",
                vec![stellar(141, "Proxima Reborn", (0, 0), 1)],
            ),
            128 if self.home => ("Home", Vec::new()),
            134 if self.near => ("Proxima Near", Vec::new()),
            // A scripted flight's default frame's: no system, nothing in it.
            0 => ("", Vec::new()),
            other => panic!("asked for sÿst {other}"),
        };
        let mut stellars: Vec<StellarContents> = stellars;
        let mut zero = 40;
        if let Some(moon) = self.moon {
            zero = moon.zero_width;
            let square = |image_id, width, frames| StellarSheet {
                image: crate::SheetImage::Rled(image_id),
                frames: NonZeroU16::new(frames).expect("non-zero"),
                frame_width: width,
                frame_height: width,
            };
            for stellar in stellars.iter_mut().filter(|s| s.id == StellarId(129)) {
                stellar.sprite = Ok(square(1129, moon.width, moon.frames));
                stellar.dead_sprite = moon
                    .dead_width
                    .map(|w| Ok(square(2129, w, moon.dead_frames)));
                stellar.animation.flags2 = moon.flags2;
            }
        }
        if let Some(reason) = self.moon_unreadable {
            for stellar in stellars.iter_mut().filter(|s| s.id == StellarId(129)) {
                stellar.sprite = Err(reason.to_owned());
            }
        }
        SystemContents {
            id,
            name: name.to_owned(),
            stellars,
            type_zero_sprite: Ok(StellarSheet {
                image: crate::SheetImage::Rled(TYPE_ZERO),
                frames: NonZeroU16::MIN,
                frame_width: zero,
                frame_height: zero,
            }),
            problems: Vec::new(),
        }
    }
}

impl GalaxyCatalog for FakeCatalog {
    fn galaxy(&self) -> Galaxy {
        let entry = |id, name: &str, (x, y), links: &[i16]| SystemEntry {
            id: SystemId(id),
            name: name.to_owned(),
            x,
            y,
            links: links.iter().copied().map(SystemId).collect(),
            govt: None,
            stellars: Vec::new(),
        };
        let onward: &[i16] = if self.onward { &[130, 132] } else { &[130] };
        let sol = self.sol_links();
        let mut systems = vec![
            entry(130, "Sol", (0, 0), &sol),
            entry(131, "Alpha Centauri", (600, 0), onward),
            entry(132, "Barnard", (0, 600), &[]),
        ];
        if self.reborn {
            systems.push(entry(133, "Alpha Reborn", (600, 0), &[]));
        }
        if self.near {
            systems.push(entry(134, "Proxima Near", (40, 30), &[130]));
        }
        if self.home {
            systems.push(entry(128, "Home", (-300, 200), &[]));
        }
        let picture = |slot: u8, side| crate::galaxy::NebulaPicture {
            id: 9500 + i16::from(slot),
            slot,
            width: side,
            height: side,
        };
        let nebulae = if self.nebula {
            vec![crate::galaxy::NebulaEntry {
                id: nova_sim::NebulaId(128),
                name: "Sol Nebula".to_owned(),
                left: -20,
                top: -20,
                width: 60,
                height: 60,
                pictures: vec![picture(0, 30), picture(2, 120)],
            }]
        } else {
            Vec::new()
        };
        for system in &mut systems {
            system.govt = self.govt_of(system.id.0);
        }
        Galaxy {
            systems,
            nebulae,
            ..Galaxy::default()
        }
    }
}

impl ShipSprites for FakeCatalog {
    /// Ship 128's sheet is [`FakeCatalog::sheet`]; ship 129's is
    /// `rlëD` 2001's, with 72 rotations, glow `rlëD` 2101, lights
    /// `rlëD` 2201 and weapon glow `rlëD` 2301 of `WeapDecay` 3; ship 130's cannot be read, nor can the escape
    /// pod's, 895.
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
        self.sheets_asked.borrow_mut().push(id);
        match id.0 {
            128 => self.sheet.clone(),
            895 => Err("no shän 895".to_owned()),
            129 if self.npc_sheet.is_some() => self.npc_sheet.ok_or_else(String::new),
            129 => Ok(ShipSheet {
                image_id: 2001,
                rotations: NonZeroU16::new(72).expect("non-zero"),
                glow: Some(layer(2101, 72)),
                lights: Some(layer(2201, 72)),
                weapon: Some(layer(2301, 72)),
                weap_decay: 3,
                ..sheet()
            }),
            130 => Err("no shän 130".to_owned()),
            131 => Ok(ShipSheet {
                image_id: 2002,
                frame_width: 100,
                frame_height: 100,
                ..sheet()
            }),
            other => panic!("asked for shïp {other}'s sheet"),
        }
    }
}

/// The looks, cards and codes given; any other weapon cannot be read,
/// and there is no other explosion type. Smoke sets 0 and 1 have their
/// sprites.
impl CombatLooks for FakeCatalog {
    fn weapon_look(&self, id: nova_sim::WeaponId) -> Result<WeaponLook, String> {
        self.looks
            .iter()
            .find(|(weapon, _)| *weapon == id.0)
            .map_or_else(
                || Err(format!("no wëap {}", id.0)),
                |(_, look)| look.clone(),
            )
    }

    fn boom_look(&self, id: nova_sim::BoomId) -> Option<Result<BoomLook, String>> {
        self.booms
            .iter()
            .find(|(boom, _)| *boom == id.0)
            .map(|(_, look)| look.clone())
    }

    fn target_card(&self, ship: ShipId) -> TargetCard {
        self.cards
            .iter()
            .find(|(id, _)| *id == ship.0)
            .map(|(_, card)| card.clone())
            .unwrap_or_default()
    }

    /// Each card as [`FakeCatalog::target_card`] gives it, the batch
    /// recorded.
    fn target_cards(&self, ships: &[ShipId]) -> Vec<TargetCard> {
        self.cards_asked.borrow_mut().push(ships.to_vec());
        ships.iter().map(|&ship| self.target_card(ship)).collect()
    }

    fn target_code(&self, govt: GovtId) -> Option<String> {
        self.codes
            .iter()
            .find(|(id, _)| *id == govt.0)
            .map(|(_, code)| code.clone())
    }

    /// Graphic 0's sheet as set; graphics 1-4, an asteroid's boxes',
    /// `rlëD` 501-504 of 36 frames.
    fn box_sheet(&self, graphic: u16) -> Result<EffectSheet, String> {
        match graphic {
            0 => self.box_sheet.clone(),
            1..=4 => Ok(EffectSheet {
                frame_width: 0,
                image: crate::SheetImage::Rled(500 + graphic.cast_signed()),
                frames: NonZeroU16::new(36).expect("non-zero"),
            }),
            other => Err(format!("no spïn {}", 500 + other)),
        }
    }

    /// Type `kind`'s sheet: `rlëD` 1800 + `kind`, of 36 frames.
    fn asteroid_sheet(&self, kind: u8) -> Result<EffectSheet, String> {
        Ok(EffectSheet {
            frame_width: 0,
            image: crate::SheetImage::Rled(1800 + i16::from(kind)),
            frames: NonZeroU16::new(36).expect("non-zero"),
        })
    }

    fn cursor_sheet(&self) -> Result<EffectSheet, String> {
        self.cursor_sheet.clone()
    }

    /// The escape pods' sheet: `rlëD` 1095, of 36 rotations.
    fn pod_sheet(&self) -> Result<EffectSheet, String> {
        Ok(EffectSheet {
            image: crate::SheetImage::Rled(1095),
            frames: NonZeroU16::new(36).expect("non-zero"),
            frame_width: 0,
        })
    }

    fn smoke_sheet(&self, set: u8) -> Option<crate::flight::catalog::SmokeSheet> {
        (set < 2).then(|| crate::flight::catalog::SmokeSheet {
            first_icon: 1000 + 8 * i16::from(set),
        })
    }

    fn star_sheet(&self) -> Result<StarSheet, String> {
        self.star_sheet.clone()
    }
}

/// The escort menu in its own colours, so tests can tell them.
impl EscortMenuLooks for FakeCatalog {
    fn escort_menu_colors(&self) -> EscortMenuColors {
        MENU_COLORS
    }
}

const MENU_COLORS: EscortMenuColors = EscortMenuColors {
    border: Color::from_rgb24(0x0012_3456),
    hilite: Color::from_rgb24(0x0065_4321),
};

/// `ïntf` 128 and the others listed; every government listed has an
/// `Interface`, and the others have no record. `PICT` 700 (128's
/// background), 705 and 128 are 194 x 767, and there are no others.
impl StatusBars for FakeCatalog {
    fn government_interface(&self, id: GovtId) -> Result<Option<i16>, String> {
        Ok(self
            .interfaces
            .iter()
            .find(|(govt, _)| *govt == id.0)
            .map(|&(_, interface)| interface))
    }

    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
        self.bars_asked.borrow_mut().push(id);
        if id == 128 {
            return self.bar.clone();
        }
        self.other_bars
            .iter()
            .find(|(bar, _)| *bar == id)
            .map(|(_, layout)| layout.clone())
            .ok_or_else(|| format!("no ïntf {id}"))
    }

    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        [700, 705, 128].contains(&id).then_some((194, 767))
    }

    fn icon_size(&self, id: i16) -> Option<(u32, u32)> {
        (id == ALERT_CICN).then_some(self.alert_icon).flatten()
    }
}

/// Where flight draws the player's ship in Nova's own 1024 x 768 by the
/// original's reading of [`RuleKey::FlightAnchor`]: the play area's
/// centre, `_screenCenter`, (trunc((1024 − 194) / 2), 384).
const PLAYER_AT: Point = Point::new(415.0, 384.0);

fn at(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

fn key(key: Key, pressed: bool) -> Input {
    Input::Key {
        key,
        pressed,
        repeat: false,
    }
}

fn held(key: Key) -> Input {
    Input::Key {
        key,
        pressed: true,
        repeat: true,
    }
}

/// The second half of a tick: `TICK / 2` rounds down, so this is a
/// nanosecond longer.
const REST_OF_TICK: Duration = Duration::from_nanos(16_666_667);

const THRUST: Controls = Controls {
    thrust: true,
    turn: Turn::None,
    reverse: false,
    face: nova_sim::flight::Face::Off,
    afterburner: false,
};

impl DescriptionSource for FakeCatalog {
    fn description(&self, id: i16) -> Result<String, String> {
        self.descriptions
            .iter()
            .find(|(found, _)| *found == id)
            .map(|(_, text)| text.clone())
            .ok_or_else(|| format!("no dësc {id}"))
    }

    fn button_style(&self) -> crate::ui::ButtonStyle {
        crate::ui::ButtonStyle::STOCK
    }
}

impl DescriptionGraphics for FakeCatalog {
    fn description_graphic(&self, _: i16) -> Option<i16> {
        None
    }

    fn description_movie(&self, id: i16) -> Option<nova_sim::DescMovie> {
        self.desc_movies
            .iter()
            .find(|(found, _)| *found == id)
            .map(|(_, movie)| movie.clone())
    }
}
fn drawn(view: &FlightView<FakeCatalog, impl FlightSim>) -> DrawList {
    let mut list = DrawList::new();
    view.draw(&mut list);
    list
}

fn overlay(text: &str, origin: Point, size: f32, color: Color) -> DrawCommand {
    DrawCommand::Text {
        text: text.to_owned(),
        font: Font::Geneva,
        origin,
        size,
        wrap_width: None,
        color,
    }
}

/// The stars' sheet, `rlëD` 700 of 16 frames of 5 × 5, as stock's.
fn star_sheet() -> StarSheet {
    StarSheet {
        image_id: 700,
        frames: NonZeroU16::new(16).expect("non-zero"),
        size: (5, 5),
    }
}

/// [`catalog`] with the stars' sheet, so flight draws its stars.
fn starry() -> FakeCatalog {
    FakeCatalog {
        star_sheet: Ok(star_sheet()),
        ..catalog()
    }
}

/// Whether `command` is one of the stars [`star_sheet`] draws.
fn is_star(command: &DrawCommand) -> bool {
    matches!(
        command,
        DrawCommand::Sprite {
            image,
            blend: Blend::Or,
            ..
        } if image.id == 700
    )
}

/// The sprites drawn, the stars left out.
fn sprites(list: &DrawList) -> Vec<(ImageKey, Point)> {
    list.iter()
        .filter(|command| !is_star(command))
        .filter_map(|command| match unclipped(command) {
            DrawCommand::Sprite { image, center, .. } => Some((*image, *center)),
            _ => None,
        })
        .collect()
}

fn lines(list: &DrawList) -> Vec<DrawCommand> {
    list.iter()
        .filter(|c| matches!(c, DrawCommand::Line { .. }))
        .cloned()
        .collect()
}

/// The stock radar, with the bar at (830, 0): centred on (926, 96).
const STOCK_RADAR: IRect = IRect::new(838, 8, 1014, 184);

/// What the radar shows this frame, as marks on the stock radar.
fn radar_marks(view: &FlightView<FakeCatalog, impl FlightSim>) -> Vec<RadarMark> {
    radar::marks(
        STOCK_RADAR,
        &view.radar_shown(&view.frame_now()),
        view.hud_cadence.blink_on(),
    )
}

/// The ships' blips on the radar this frame: its pixels but the
/// player's, the last.
fn blips(view: &FlightView<FakeCatalog, impl FlightSim>) -> Vec<RadarMark> {
    let mut marks: Vec<RadarMark> = radar_marks(view)
        .into_iter()
        .filter(|mark| matches!(mark, RadarMark::Pixel { .. }))
        .collect();
    marks.pop();
    marks
}

const PLAYER_PIXEL: RadarMark = RadarMark::Pixel {
    x: 926,
    y: 96,
    color: RadarColor::Bright,
};

fn texts(list: &DrawList) -> Vec<String> {
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The port's title, and its help line, on.
const TITLED: FlightOverlays = FlightOverlays {
    title: true,
    help: true,
    cheats: false,
};
fn layer(image_id: i16, frames: u16) -> LayerSheet {
    LayerSheet {
        image_id,
        frames: NonZeroU16::new(frames).expect("non-zero"),
        solid: false,
    }
}

/// [`layered`], with the lights blinking by `blink`.
fn blinking(blink: Blink) -> FakeCatalog {
    FakeCatalog {
        sheet: Ok(ShipSheet {
            glow: Some(layer(2100, 36)),
            lights: Some(layer(2200, 12)),
            ..sheet()
        }),
        hulls: vec![lit_hull(hull_of(128, &[]), blink)],
        ..catalog()
    }
}

/// `hull` with a lights sprite blinking `blink`, as its `shän` names one,
/// which the session's display steps (`nova_sim::combat::ship_lights`).
fn lit_hull(hull: nova_sim::HullRecord, blink: Blink) -> nova_sim::HullRecord {
    let display = hull.display.unwrap_or(nova_sim::catalog::ShipDisplay {
        base_sets: 1,
        ..nova_sim::catalog::ShipDisplay::default()
    });
    nova_sim::HullRecord {
        display: Some(nova_sim::catalog::ShipDisplay {
            blink,
            lights_image: true,
            ..display
        }),
        ..hull
    }
}

/// `ïntf` 129: [`layout`] in another shield colour, over `PICT` 705.
fn bar_129() -> StatusBarLayout {
    StatusBarLayout {
        shield_color: Color::rgba(0, 255, 255, 255),
        status_bkgnd: 705,
        ..layout()
    }
}

/// `catalog` where ship type `ship` (given a record if it has none)
/// belongs to gövt 150, whose `Interface` is `interface`, and with
/// `ïntf` 129 ([`bar_129`]).
fn governed(mut catalog: FakeCatalog, ship: i16, interface: i16) -> FakeCatalog {
    if !catalog.ships.iter().any(|record| record.id == ShipId(ship)) {
        let mut record = shipbuying().ships[0].clone();
        record.id = ShipId(ship);
        catalog.ships.push(record);
    }
    for record in &mut catalog.ships {
        if record.id == ShipId(ship) {
            record.inherent_govt = Some(GovtId(150));
        }
    }
    catalog.interfaces = vec![(150, interface)];
    catalog.other_bars = vec![(129, bar_129())];
    catalog
}
const LAND: Key = Key::Char('l');

/// Where the message's first line is drawn: the message box's
/// top-left.
fn message_at() -> Point {
    hud::message_box(VIEW_SIZE).min
}

/// The message `text` as a flight without metrics draws it.
fn message_drawn(text: &str) -> DrawCommand {
    DrawCommand::Text {
        text: text.to_owned(),
        font: hud::MESSAGE_FONT,
        origin: at(25.0, 724.0),
        size: 12.0,
        wrap_width: Some(755.0),
        color: Color::WHITE,
    }
}

const MAP: Key = Key::Char('m');
const JUMP: Key = Key::Char('j');

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// Presses and releases `k`.
fn tap(view: &mut FlightView<FakeCatalog, impl FlightSim>, k: Key) {
    view.input(&key(k, true));
    view.input(&key(k, false));
}

/// Nova's collaborators, rolling on `chance`, the NPCs deciding as
/// `behaviour` says.
fn on(chance: SharedChance, behaviour: Rc<dyn Behaviour>) -> Collaborators {
    Collaborators {
        chance,
        behaviour,
        ..Collaborators::default()
    }
}

/// The fade quad's place in the list and its colour, if drawn.
fn fade(list: &DrawList) -> Option<(usize, Color)> {
    list.iter().enumerate().find_map(|(at, c)| match *c {
        DrawCommand::Line {
            from, width, color, ..
        } if from == Point::new(0.0, 384.0) && width == 768.0 => Some((at, color)),
        _ => None,
    })
}

/// Where the HUD starts: its status bar picture.
fn hud_at(list: &DrawList) -> usize {
    list.iter()
        .position(
            |c| matches!(c, DrawCommand::Picture { image, .. } if *image == ImageKey::picture(700)),
        )
        .expect("the HUD")
}

/// Ship 128 carrying the stock Multi-Jumping Organ (`oütf` 275,
/// multi-jump 10), with Alpha Centauri linked on to Barnard.
fn multi_jumping() -> FakeCatalog {
    FakeCatalog {
        outfits: vec![OutfitRecord {
            id: OutfitId(275),
            name: "Multi-Jumping Organ".to_owned(),
            short_name: "Multi-Jumping Organ".to_owned(),
            disp_weight: 0,
            mass: 0,
            tech_level: 1,
            max: 1,
            flags: 0,
            cost: 0,
            mods: [(32, 10), (0, 0), (0, 0), (0, 0)],
            contribute: 0,
            require: 0,
            require_govt: -1,
            buy_random: 100,
            availability: nova_sim::Test::default(),
            item_class: 0,
            lc_name: "multi-jumping organ".to_owned(),
            lc_plural: "multi-jumping organs".to_owned(),
            scan_mask: 0,
            on_purchase: nova_sim::Script::default(),
            on_sell: nova_sim::Script::default(),
        }],
        defaults: vec![(OutfitId(275), 1)],
        onward: true,
        ..catalog()
    }
}

use nova_sim::{Chance, Direction, DisasterId, Good, Lot, Order};

/// Earth at the centre, a trade center trading food at 75, the first
/// `chär` holding 1000 credits, and its ship 10 tons.
fn trading() -> FakeCatalog {
    let flags = StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | 2 << 28;
    FakeCatalog {
        character: Ok(CharacterStart {
            cash: 1000,
            ..catalog().character.expect("a chär")
        }),
        fields: ShipFields {
            holds: 10,
            ..FIELDS
        },
        sites: vec![site(128, (0.0, 0.0), flags)],
        commodities: CommodityStrings {
            names: vec!["Food".to_owned()],
            name_patches: Default::default(),
            base_prices: vec!["75".to_owned()],
            price_patches: Default::default(),
        },
        ..catalog()
    }
}

const BUY_FOOD: Order = Order {
    row: 0,
    good: Good::Commodity(0),
    direction: Direction::Buy,
    lot: Lot::Click,
};

use nova_sim::OutfitOrder;

/// Earth at the centre, an outfitter selling a fuel tank (+100 fuel,
/// a ton, 1000 credits), the first `chär` holding 1000 credits.
fn outfitting() -> FakeCatalog {
    FakeCatalog {
        character: Ok(CharacterStart {
            cash: 1000,
            ..catalog().character.expect("a chär")
        }),
        sites: vec![site(
            128,
            (0.0, 0.0),
            StellarFlags::CAN_LAND | StellarFlags::OUTFITTER,
        )],
        outfits: vec![OutfitRecord {
            id: OutfitId(200),
            name: "Fuel Tank".to_owned(),
            short_name: "Fuel Tank".to_owned(),
            disp_weight: 0,
            mass: 1,
            tech_level: 1,
            max: 5,
            flags: 0,
            cost: 1000,
            mods: [(12, 100), (0, 0), (0, 0), (0, 0)],
            contribute: 0,
            require: 0,
            require_govt: -1,
            buy_random: 100,
            availability: nova_sim::Test::default(),
            item_class: 0,
            lc_name: "fuel tank".to_owned(),
            lc_plural: "fuel tanks".to_owned(),
            scan_mask: 0,
            on_purchase: nova_sim::Script::default(),
            on_sell: nova_sim::Script::default(),
        }],
        ..catalog()
    }
}

const BUY_TANK: OutfitOrder = OutfitOrder {
    outfit: OutfitId(200),
    direction: Direction::Buy,
};

/// Fires every time, and records each percent it is asked; each draw
/// is 0, and records its `n`.
#[derive(Default)]
struct Always {
    asked: Vec<u8>,
    drawn: Vec<u32>,
}

impl Chance for Always {
    fn fires(&mut self, percent: u8) -> bool {
        self.asked.push(percent);
        true
    }

    fn below(&mut self, n: u32) -> u32 {
        self.drawn.push(n);
        0
    }

    /// The last of the outcomes.
    fn roll(&mut self, sides: u16) -> u16 {
        sides.saturating_sub(1)
    }
}

/// [`outfitting`], where Earth is a shipyard too, selling ship 129 for
/// 900 credits: twice as fast, with a fuel tank.
fn shipbuying() -> FakeCatalog {
    let mut earth = site(
        128,
        (0.0, 0.0),
        StellarFlags::CAN_LAND | StellarFlags::OUTFITTER | StellarFlags::SHIPYARD,
    );
    earth.tech_level = 1;
    FakeCatalog {
        sites: vec![earth],
        ships: vec![ShipRecord {
            id: ShipId(129),
            name: "Fast".to_owned(),
            short_name: "Fast".to_owned(),
            long_name: String::new(),
            fields: ShipFields {
                speed: 600,
                ..FIELDS
            },
            defaults: vec![(OutfitId(200), 1)],
            default_slots: [None; 8],
            strength: 0,
            cost: 900,
            tech_level: 1,
            buy_random: 100,
            hire_random: 0,
            require: 0,
            availability: nova_sim::Test::default(),
            appear_on: nova_sim::Test::default(),
            flags: 0,
            flags3: 0,
            disp_weight: 0,
            max_gun: 0,
            max_tur: 0,
            length: 0,
            crew: 0,
            inherent_ai: 1,
            comm_name: String::new(),
            inherent_govt: None,
            combat_govt: None,
            escort_type: -1,
            skill_var: 0,
            subtitle: String::new(),
            upgrade_to: None,
            esc_upgrd_cost: 0,
            esc_sell_value: 0,
            on_capture: nova_sim::Script::default(),
            on_purchase: nova_sim::Script::default(),
            on_retire: nova_sim::Script::default(),
        }],
        ..outfitting()
    }
}
/// Rolls the second outcome of every roll, so `R(a b)` takes `a`.
#[derive(Debug)]
struct RollsOne;

impl Chance for RollsOne {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n.min(1)
    }
}

fn rolling_one() -> SharedChance {
    SharedChance::new(Rc::new(RefCell::new(RollsOne)))
}

/// The texts drawn in the nav area of the port's status bar
/// ([`FlightView::draw_status_bar`]), as [`nav`] reads flight's.
fn port_nav(view: &FlightView<FakeCatalog, impl FlightSim>) -> Vec<String> {
    let area = layout().nav.offset(at(830.0, 0.0));
    let mut bar = DrawList::new();
    view.draw_status_bar(&mut bar);
    bar.iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, origin, .. } if area.contains(*origin) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The nav area naming `name` in mode 2.
fn naming(name: &str) -> Vec<String> {
    vec![hud::NAV_STELLAR.to_owned(), name.to_owned()]
}

/// A view over a scripted flight ([`FakeFlight`]), for the component
/// tests of the code on the port.
type Faked = FlightView<FakeCatalog, FakeFlight>;

/// A view over a flight scripted to show `frame`, read from [`catalog`].
fn faked(frame: FlightFrame) -> Faked {
    faked_in(catalog(), frame)
}

/// A view over a flight scripted to show `frame`, read from `catalog`,
/// the flight saying the date as [`DATE`] and naming nothing else.
fn faked_in(catalog: FakeCatalog, frame: FlightFrame) -> Faked {
    let mut fake = FakeFlight::new();
    fake.set_frame(frame);
    fake.names_mut().long_date = DATE.to_owned();
    FlightView::scripted(catalog, fake, &Rulebook::default())
}

/// The commands `view` sent its flight, in order.
fn sent(view: &Faked) -> &[FlightCommand] {
    view.sim.applied()
}

/// Gives `view`'s course map `input` as flight's map input does, and
/// says whether it closed the map: the close is the session's, so a
/// scripted flight only notes it.
fn faked_map_input(view: &mut Faked, input: &Input) -> bool {
    let closed = std::cell::Cell::new(false);
    view.map_input_then(input, |_| closed.set(true));
    closed.get()
}

/// Presses and releases `k` on `view`'s open course map, as flight's map
/// input gives them, and says whether the press closed it.
fn faked_map_tap(view: &mut Faked, k: Key) -> bool {
    let closed = faked_map_input(view, &key(k, true));
    faked_map_input(view, &key(k, false));
    closed
}

/// Clicks the left button at `at` on `view`'s open course map, and says
/// whether the click closed it.
fn faked_map_click(view: &mut Faked, at: Point) -> bool {
    let button = |pressed| Input::PointerButton {
        button: crate::MouseButton::Left,
        pressed,
        at,
    };
    let by_press = faked_map_input(view, &button(true));
    let by_release = faked_map_input(view, &button(false));
    by_press || by_release
}

/// Shift-clicks at `at` on `view`'s open course map.
fn faked_map_shift_click(view: &mut Faked, at: Point) {
    faked_map_input(view, &key(Key::Shift, true));
    faked_map_click(view, at);
    faked_map_input(view, &key(Key::Shift, false));
}

/// Where system `id` is on `view`'s course map, brought into item 3
/// first, as [`on_map`] finds it on a session's.
fn faked_on_map(view: &mut Faked, id: i16) -> Point {
    view.map.centre_on_system(SystemId(id));
    let system = view.map.model().system(SystemId(id)).expect("on the map");
    view.map.transform().world_to_screen(system.position())
}

/// System `id` as a flight's drawing frames it in `code`, with no
/// mission arrows.
fn drawn_system(
    id: i16,
    code: nova_sim::iff::SystemColorCode,
) -> nova_sim::galaxy_map::DrawnSystem {
    nova_sim::galaxy_map::DrawnSystem {
        id: SystemId(id),
        code,
        red_arrow: false,
        green_arrow: false,
    }
}

/// The course map's drawing in Sol (130) with the nav off, as a session
/// over [`catalog`] draws it: Sol, uninhabited and the one system named,
/// and its link to Alpha Centauri (131), unexplored.
fn sol_drawing() -> nova_sim::galaxy_map::MapDrawing {
    use nova_sim::iff::SystemColorCode;
    let (sol, alpha) = (SystemId(130), SystemId(131));
    nova_sim::galaxy_map::MapDrawing {
        hypergate: false,
        course: Vec::new(),
        current_links: vec![(sol, alpha)],
        links: vec![nova_sim::galaxy_map::MapLink {
            from: sol,
            to: alpha,
            selected: false,
        }],
        mode3_link: None,
        systems: vec![
            drawn_system(130, SystemColorCode::Uninhabited),
            drawn_system(131, SystemColorCode::Unexplored),
        ],
        current: sol,
        gate_arrows: Vec::new(),
        named: vec![sol],
    }
}

/// [`faked`] in `frame`, its flight drawing the course map as
/// [`sol_drawing`] and preselecting Sol on it.
fn faked_sol(frame: FlightFrame) -> Faked {
    let mut view = faked(frame);
    let map = view.sim.map_mut();
    map.drawing = Some(sol_drawing());
    map.map_preselection = Some(SystemId(130));
    view
}

#[test]
fn a_view_over_a_scripted_flight_shows_its_frame() {
    let mut view = faked(FlightFrame {
        nav_mode: NavMode::Hyperspace(Some(SystemId(131))),
        jump_readiness: nova_sim::JumpReadiness::Clear,
        ..in_sol()
    });
    view.sim.map_mut().explored = vec![SystemId(131)];
    let scene = view.scene.as_ref().expect("the frame's scene");
    assert_eq!(scene.name(), "Sol", "the frame's system");
    assert_eq!(
        view.nav_now(),
        NavDisplay::Hyperspace {
            destination: hud::NavDestination::Named("Alpha Centauri".to_owned()),
            readiness: nova_sim::JumpReadiness::Clear,
        }
    );
}

#[test]
fn a_scripted_flights_map_sends_its_cycle_and_reports_its_close() {
    let mut view = faked(in_sol());
    view.open_map();
    assert!(!faked_map_input(&mut view, &key(Key::Tab, true)));
    assert_eq!(sent(&view), [FlightCommand::CycleMapSelection]);
    assert!(faked_map_input(&mut view, &key(Key::Enter, true)));
}

/// The nav area showing Alpha Centauri (131), unexplored, in mode 3.
fn cueing_alpha() -> Vec<String> {
    vec![
        hud::NAV_HYPERSPACE.to_owned(),
        hud::NAV_UNEXPLORED.to_owned(),
    ]
}
use nova_sim::mission::screens::SPACEPORT;

/// Which of the bar's parts show as they are now: the gauges, the
/// nav area, the target panel and the radar.
fn retaken(view: &FlightView<FakeCatalog, impl FlightSim>) -> [bool; 4] {
    [
        view.reserves_shown(&view.frame_now()) == view.reserves(),
        view.nav_shown(&view.frame_now()) == view.nav_now(),
        *view.target_shown(&view.frame_now()) == view.target_now(&view.frame_now()),
        view.radar_scene == Some(view.radar_now(&view.frame_now())),
    ]
}
use std::collections::VecDeque;

use nova_sim::{Goal, NpcId};

/// Draws its script, then the last outcome, so no roll fires;
/// records each `n` asked.
#[derive(Default)]
struct Script {
    draws: VecDeque<u32>,
    asked: Vec<u32>,
}

impl Chance for Script {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.asked.push(n);
        self.draws.pop_front().unwrap_or(n - 1)
    }
}

fn scripted(draws: &[u32]) -> (Rc<RefCell<Script>>, SharedChance) {
    let script = Rc::new(RefCell::new(Script {
        draws: draws.iter().copied().collect(),
        asked: Vec::new(),
    }));
    let shared: Rc<RefCell<dyn Chance>> = script.clone();
    (script, SharedChance::new(shared))
}

/// One setup pass placing the düde's ship at (`x` - 750, `y` - 750)
/// facing `heading`, of aggression 0.
fn placed(x: u32, y: u32, heading: u32) -> [u32; 8] {
    [6, 6, 0, 0, x, y, heading, 2]
}

/// Decides nothing: every NPC idles.
#[derive(Debug)]
struct Still;

impl Behaviour for Still {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::Idle
    }
}

/// [`catalog`] with `avg` ships on average in each of `systems`, all of
/// düde 128's ship `ship` with AI `ai_type`; ships 129 and 130 are
/// records the traffic can fly.
fn trafficked(systems: &[i16], avg: i16, ship: i16, ai_type: i16) -> FakeCatalog {
    let mut dude_types = [(-1, 0); 8];
    dude_types[0] = (128, 100);
    let record = |id| ShipRecord {
        id: ShipId(id),
        name: format!("Ship {id}"),
        short_name: String::new(),
        long_name: String::new(),
        fields: ShipFields {
            speed: 600,
            ..FIELDS
        },
        defaults: Vec::new(),
        default_slots: [None; 8],
        strength: 0,
        cost: 1,
        tech_level: 1,
        buy_random: 100,
        hire_random: 0,
        require: 0,
        availability: nova_sim::Test::default(),
        appear_on: nova_sim::Test::default(),
        flags: 0,
        flags3: 0,
        disp_weight: 0,
        max_gun: 0,
        max_tur: 0,
        length: 0,
        crew: 0,
        inherent_ai: 1,
        comm_name: String::new(),
        inherent_govt: None,
        combat_govt: None,
        escort_type: -1,
        skill_var: 0,
        subtitle: String::new(),
        upgrade_to: None,
        esc_upgrd_cost: 0,
        esc_sell_value: 0,
        on_capture: nova_sim::Script::default(),
        on_purchase: nova_sim::Script::default(),
        on_retire: nova_sim::Script::default(),
    };
    FakeCatalog {
        traffic: systems
            .iter()
            .map(|&id| {
                (
                    SystemId(id),
                    nova_sim::SystemTraffic {
                        dude_types,
                        avg_ships: avg,
                        persons: Default::default(),
                    },
                )
            })
            .collect(),
        dudes: vec![(
            nova_sim::DudeId(128),
            nova_sim::DudeRecord {
                ai_type,
                govt: None,
                ships: vec![(ShipId(ship), 1)],
                booty: 0,
                info_types: 0,
            },
        )],
        ships: vec![record(129), record(130)],
        ..catalog()
    }
}

/// Says no ship is disabled, counting the times it is asked.
#[derive(Debug, Default)]
struct Counting {
    asked: std::cell::Cell<usize>,
}

impl DisableRule for Counting {
    fn disabled(&self, _armor: nova_sim::Gauge, _hull: &nova_sim::HullSpec) -> bool {
        self.asked.set(self.asked.get() + 1);
        false
    }
}

/// Every NPC idles and holds its trigger.
#[derive(Debug)]
struct Firing;

impl Behaviour for Firing {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::Idle
    }

    fn trigger(&self, _npc: &nova_sim::Npc, _around: &nova_sim::Surroundings) -> nova_sim::Trigger {
        nova_sim::Trigger {
            primary: true,
            secondary: None,
            only: None,
            bays: false,
            bays_after: false,
            point_defence: false,
            keep: None,
        }
    }
}
use std::num::NonZeroU16 as Frames;

use nova_sim::{BoomId, Condition, PlayerFate, WeaponId};

use crate::flight::brackets::{self, Standing};
use crate::flight::catalog::EffectSheet;
use crate::flight::effects::{DEBRIS_COLOR, Effects};
use crate::flight::target::{self, NO_SECONDARY, NO_TARGET};
use crate::flight::weapons::BEAM_UNDER_SHIPS;
use crate::sound::CombatSound;

const BLASTER: WeaponId = WeaponId(128);
const ROCKET: WeaponId = WeaponId(140);
const MISSILE: WeaponId = WeaponId(141);
const TORCH: WeaponId = WeaponId(142);
const UNDER: WeaponId = WeaponId(146);
const OVER: WeaponId = WeaponId(147);
const UNSEEN: WeaponId = WeaponId(150);

/// `wëap` `id` firing every tick, 20 pixels a tick for 30 ticks, doing
/// no damage, with `flags`, `guidance` and `explod_type`.
fn gun(id: WeaponId, flags: u16, guidance: i16, explod_type: i16) -> nova_sim::WeaponRecord {
    nova_sim::WeaponRecord {
        id,
        reload: 0,
        count: 30,
        mass_dmg: 0,
        energy_dmg: 0,
        guidance,
        speed: 2000,
        ammo_type: -1,
        graphic: 0,
        inaccuracy: 0,
        impact: 0,
        explod_type,
        prox_radius: 0,
        blast_radius: 0,
        flags,
        seeker: 0,
        flags2: 0,
        flags3: 0,
        decay: 0,
        beam_length: 50,
        beam_width: 0,
        falloff: 0,
        li_density: 0,
        burst_count: 0,
        burst_reload: 0,
        guided_turn: 0,
        durability: 0,
        sub_count: 0,
        sub_type: None,
        sub_theta: 0,
        sub_limit: 0,
        max_ammo: 0,
        prox_safety: 0,
        recoil: 0,
        exit_type: -1,
        ionization: 0,
        ionize_color: 0,
        jam_vuln: [0; 4],
        frame: None,
        hit_particles: 0,
        hit_part_life: 0,
        hit_part_vel: 0,
        hit_part_color: 0,
    }
}

/// Ship `id` carrying one of each of `weapons`, gone at once in
/// `bööm` 128.
fn hull_of(id: i16, weapons: &[WeaponId]) -> nova_sim::HullRecord {
    nova_sim::HullRecord {
        id: ShipId(id),
        flags: 0,
        flags2: 0,
        flags3: 0,
        death_delay: 0,
        pod_count: 0,
        explode1: -1,
        explode2: 0,
        mass: 0,
        weapons: weapons
            .iter()
            .map(|&weapon| nova_sim::StockWeapon {
                weapon,
                count: 1,
                ammo: 0,
            })
            .collect(),
        size: None,
        height: None,
        strength: 0,
        key_carried: None,
        cost: 0,
        speed: 0,
        exits: None,
        // Steady lights, drawn where its sheet has a lights layer.
        display: Some(nova_sim::catalog::ShipDisplay {
            base_sets: 1,
            lights_image: true,
            ..nova_sim::catalog::ShipDisplay::default()
        }),
        escort_class: nova_sim::EscortClass::Freighter,
    }
}

/// `oütf` `id`, "Gun `id`", holding a `weapon`: a ton, 1000 credits.
fn holding(id: i16, weapon: WeaponId) -> OutfitRecord {
    OutfitRecord {
        id: OutfitId(id),
        name: format!("Gun {id}"),
        short_name: format!("Gun {id}"),
        disp_weight: 0,
        mass: 1,
        tech_level: 1,
        max: 10,
        flags: 0,
        cost: 1000,
        mods: [
            (nova_sim::combat::armament::MOD_WEAPON, weapon.0),
            (0, 0),
            (0, 0),
            (0, 0),
        ],
        contribute: 0,
        require: 0,
        require_govt: -1,
        buy_random: 100,
        availability: nova_sim::Test::default(),
        item_class: 0,
        lc_name: format!("gun {id}"),
        lc_plural: format!("guns {id}"),
        scan_mask: 0,
        on_purchase: nova_sim::Script::default(),
        on_sell: nova_sim::Script::default(),
    }
}

fn effect_sheet(image_id: i16, frames: u16) -> EffectSheet {
    EffectSheet {
        frame_width: 0,
        image: crate::SheetImage::Rled(image_id),
        frames: Frames::new(frames).expect("non-zero"),
    }
}

/// A look named `name` sounding `snd ` `sound`, its shots on `rlëD`
/// `image` of 36 frames, if any.
fn look(name: &str, sound: i16, image: Option<i16>) -> WeaponLook {
    WeaponLook {
        name: name.to_owned(),
        sheet: image.map(|image| Ok(effect_sheet(image, 36))),
        sound: Some(nova_sim::SoundId(sound)),
        ..WeaponLook::default()
    }
}

/// [`trafficked`] (ships of type 129, warships) where the player's
/// ship carries `weapons`, held by outfits 250 on, in that order; the
/// blaster, rockets, missiles and torch have looks, and `bööm` 128 shows `rlëD` 400's 3 frames at a frame
/// a step, sounding `snd ` 302. Ship 129 is a "Light Transport" with
/// picture 3001, and govt 140's code is "Fed.".
fn armed(avg: i16, weapons: &[WeaponId]) -> FakeCatalog {
    let mut catalog = trafficked(&[130], avg, 129, 3);
    catalog.dudes[0].1.govt = Some(GovtId(140));
    FakeCatalog {
        weapons: vec![
            gun(BLASTER, 0, -1, 0),
            gun(ROCKET, 0x0002, -1, -1),
            gun(MISSILE, 0x0002, -1, -1),
            gun(TORCH, 0x0002, -1, -1),
            gun(UNDER, 0, 0, -1),
            gun(OVER, 0, 0, -1),
            gun(UNSEEN, 0, -1, -1),
        ],
        hulls: vec![hull_of(128, weapons), hull_of(129, &[])],
        outfits: (250..)
            .zip(weapons)
            .map(|(id, &weapon)| holding(id, weapon))
            .collect(),
        looks: vec![
            (128, Ok(look("Blaster", 208, Some(3500)))),
            (140, Ok(look("Rocket", 209, Some(3501)))),
            (141, Ok(look("Missile", 210, None))),
            (142, Ok(look("Torch", 211, None))),
            (
                146,
                Ok(WeaponLook {
                    flags2: BEAM_UNDER_SHIPS,
                    beam_width: 1,
                    beam_color: 0x0000_00FF,
                    ..WeaponLook::default()
                }),
            ),
            (
                147,
                Ok(WeaponLook {
                    beam_width: 1,
                    beam_color: 0x0000_FF00,
                    ..WeaponLook::default()
                }),
            ),
        ],
        booms: vec![(
            128,
            Ok(BoomLook {
                sheet: Ok(effect_sheet(400, 3)),
                advance: 1.0,
                sound: Some(nova_sim::SoundId(302)),
            }),
        )],
        cards: vec![(
            129,
            TargetCard {
                subtitle: "Light Transport".to_owned(),
                picture: Some(3001),
            },
        )],
        codes: vec![(140, "Fed.".to_owned())],
        ..catalog
    }
}

/// The first command matching `wanted`'s position in `list`.
fn first(list: &DrawList, wanted: impl Fn(&DrawCommand) -> bool) -> usize {
    list.iter()
        .position(wanted)
        .unwrap_or_else(|| panic!("not drawn: {list:?}"))
}

fn sprite_of(id: i16) -> impl Fn(&DrawCommand) -> bool {
    move |command| matches!(unclipped(command), DrawCommand::Sprite { image, .. } if image.id == id)
}

/// `command`, or what it draws when it is [`DrawCommand::Clipped`]: the
/// cursor is clipped to the play area.
fn unclipped(command: &DrawCommand) -> &DrawCommand {
    match command {
        DrawCommand::Clipped { command, .. } => unclipped(command),
        command => command,
    }
}

/// Whether a command draws a `cïcn` in `ids`.
fn icon_in(ids: std::ops::RangeInclusive<i16>) -> impl Fn(&DrawCommand) -> bool {
    move |command| {
        matches!(
            command,
            DrawCommand::Picture { image, .. }
                if image.kind == crate::ImageKind::Icon && ids.contains(&image.id)
        )
    }
}

/// The bracket `cïcn`s drawn (10000-10023), in order.
fn bracket_icons(list: &DrawList) -> Vec<i16> {
    list.iter()
        .filter(|command| icon_in(10000..=10023)(command))
        .map(|command| match command {
            DrawCommand::Picture { image, .. } => image.id,
            _ => unreachable!("a picture"),
        })
        .collect()
}

/// [`armed`] where the NPC, straight above the player and facing it,
/// fires a blaster that kills at a hit, and the player's ship breaks
/// up in `death_delay` ticks, then explodes as `bööm` 128 (`rlëD` 400).
fn killed_in(death_delay: i16) -> FakeCatalog {
    let mut catalog = armed(1, &[]);
    catalog.weapons[0].mass_dmg = 1000;
    catalog.weapons[0].flags = 0x0020;
    catalog.weapons[0].explod_type = -1;
    catalog.hulls[0].death_delay = death_delay;
    catalog.hulls[1] = hull_of(129, &[BLASTER]);
    catalog
}

/// What the target panel's status line shows this frame, if there is
/// a target.
fn status_shown(view: &FlightView<FakeCatalog, impl FlightSim>) -> Option<nova_sim::TargetStatus> {
    view.target_shown(&view.frame_now())
        .as_ref()
        .as_ref()
        .map(|panel| panel.status)
}

/// What the target panel's status line would show of the target now.
fn status_now(view: &FlightView<FakeCatalog, impl FlightSim>) -> Option<nova_sim::TargetStatus> {
    view.target_now(&view.frame_now()).map(|panel| panel.status)
}

const QUAD: WeaponId = WeaponId(133);
const IR: WeaponId = WeaponId(134);

/// [`armed`] with the player, over a planet at the centre, carrying a
/// point-defence turret (held by outfit 250), and its traffic a
/// missile it fires once, 5 pixels a tick.
fn defending() -> FakeCatalog {
    let mut catalog = armed(1, &[QUAD]);
    catalog.sites = vec![site(
        140,
        (0.0, 0.0),
        StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
    )];
    let sprite = |side| {
        Some(nova_sim::FrameSize {
            width: side,
            height: side,
        })
    };
    catalog.weapons.push(nova_sim::WeaponRecord {
        reload: 5,
        count: 12,
        mass_dmg: 1,
        energy_dmg: 4,
        frame: sprite(35),
        ..gun(QUAD, 0, 9, -1)
    });
    catalog.weapons.push(nova_sim::WeaponRecord {
        reload: 1000,
        count: 200,
        speed: 500,
        frame: sprite(18),
        ..gun(IR, 0, 1, -1)
    });
    catalog.hulls = vec![hull_of(128, &[QUAD]), hull_of(129, &[IR])];
    catalog
}

/// [`trafficked`] with its ship 129 (of strength 5, a crew of 3 and a
/// `Cost` of 150,000, and no speed, so it starts at rest) carrying
/// money (`Booty` 0x0040), and the player's ship 128 a crew of 10.
fn boardable() -> FakeCatalog {
    let mut catalog = trafficked(&[130], 1, 129, 1);
    catalog.dudes[0].1.booty = 0x0040;
    for record in &mut catalog.ships {
        if record.id == ShipId(129) {
            record.crew = 3;
            record.cost = 150_000;
        }
    }
    let mut player = catalog.ships[0].clone();
    player.id = ShipId(128);
    player.crew = 10;
    catalog.ships[0].fields.speed = 0;
    catalog.ships.push(player);
    catalog.hulls = vec![
        hull_of(128, &[]),
        nova_sim::HullRecord {
            strength: 5,
            ..hull_of(129, &[])
        },
    ];
    catalog
}

use nova_sim::HailRefusal;

/// [`boardable`]'s ship comm strings: "c<n>" for each, with "Channel
/// open." in every variant of the first group.
fn comm_strings() -> Vec<String> {
    (1..=200)
        .map(|n| {
            if n <= 5 {
                "Channel open.".to_owned()
            } else {
                format!("c{n}")
            }
        })
        .collect()
}

use nova_sim::EscortClass;

const HELLO: &str = "Message Buoy: hello.";

/// The interface sounds `view` holds, taken.
fn ui_sounds(view: &mut FlightView<FakeCatalog, impl FlightSim>) -> Vec<UiSound> {
    view.take_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Ui(sound) => Some(sound),
            _ => None,
        })
        .collect()
}
use nova_sim::hire::DEFECTED_ONE;
use nova_sim::{ControlBits, HireRefusal};

/// [`shipbuying`] with its stellar a bar too, and ship 129 (`Cost`
/// 900) for hire half the days.
fn hiring() -> FakeCatalog {
    let mut catalog = shipbuying();
    catalog.sites[0].flags |= StellarFlags::BAR;
    for record in &mut catalog.ships {
        if record.id == ShipId(129) {
            record.hire_random = 50;
        }
    }
    catalog
}
use nova_sim::MissionTextKind;
use nova_sim::catalog::{MissionRecord, MissionShip};
use nova_sim::control::{Script as SetScript, Test};

/// Mission `id`, offered at the mission computer of any stellar on
/// every visit, to anyone, going nowhere.
fn errand(id: i16) -> MissionRecord {
    MissionRecord {
        id: MissionId(id),
        avail_stel: -1,
        avail_loc: 0,
        avail_record: 0,
        avail_rating: 0,
        avail_random: 100,
        avail_ship_type: 0,
        avail_bits: Test::default(),
        require: 0,
        flags: 0,
        flags2: 0,
        travel_stel: -1,
        return_stel: -1,
        cargo_type: -1,
        cargo_qty: 0,
        pickup_mode: -1,
        drop_off_mode: -1,
        scan_mask: 0,
        pay_val: 0,
        time_limit: 0,
        disp_weight: 0,
        on_accept: SetScript::default(),
        on_refuse: SetScript::default(),
        on_success: SetScript::default(),
        on_failure: SetScript::default(),
        on_abort: SetScript::default(),
        on_ship_done: SetScript::default(),
        can_abort: 1,
        comp_govt: None,
        comp_reward: 0,
        date_post_inc: 0,
        ships: MissionShip::NONE,
        aux: nova_sim::catalog::AuxShip::NONE,
    }
}

/// The background of the status bar `view` shows.
fn bar_shown(view: &FlightView<FakeCatalog, impl FlightSim>) -> Option<i16> {
    view.status_bar
        .as_ref()
        .ok()
        .map(|bar| bar.layout.status_bkgnd)
}
/// Hides system `id`.
fn hiding(id: i16) -> impl FnOnce(&mut nova_sim::story::GalaxyOverlay) {
    move |overlay| {
        overlay.set_visible(SystemId(id), Some(false));
    }
}
/// Stellar `id` with `Strength` 100 and `DeadTime` 5, so that `Y`
/// destroys it, and `Flags2` `flags2`.
fn life(id: i16, flags2: u16) -> nova_sim::StellarLife {
    nova_sim::StellarLife {
        id: StellarId(id),
        strength: 100,
        dead_time: 5,
        flags2,
        explosion: None,
        on_destroy: nova_sim::Script::default(),
        on_regen: nova_sim::Script::default(),
    }
}

/// The four corners from `cïcn` `first` round `centre`, half-size
/// `half`: top-left, top-right, bottom-right, bottom-left.
fn corners_at(first: i16, centre: Point, half: f32) -> Vec<(i16, Point)> {
    let (near, far) = (-half - 16.0, half);
    vec![
        (first, at(centre.x + near, centre.y + near)),
        (first + 1, at(centre.x + far, centre.y + near)),
        (first + 2, at(centre.x + far, centre.y + far)),
        (first + 3, at(centre.x + near, centre.y + far)),
    ]
}

/// The engine tick `view` has reached.
fn now_tick(view: &FlightView<FakeCatalog, impl FlightSim>) -> u64 {
    u64::try_from(crate::time::ticks(view.elapsed)).expect("in range")
}
const HYPER_SELECT: Key = Key::Char('\\');

use nova_sim::GateKind;

/// The course map's sounds `view` holds, taken with the rest.
fn map_sounds(view: &mut FlightView<FakeCatalog, impl FlightSim>) -> Vec<UiSound> {
    ui_sounds(view)
        .into_iter()
        .filter(|sound| matches!(sound, UiSound::MapOpened | UiSound::MapClosed))
        .collect()
}

/// Where on screen `view` draws NPC `id`.
fn npc_on_screen<C, S: FlightSim>(view: &FlightView<C, S>, id: NpcId) -> Point {
    view.camera()
        .world_to_screen(view.shown_npc(&npc_record(view, id)).0)
}

/// NPC `id`'s record in `view`'s kept frame.
fn npc_record<C, S: crate::flight::sim_port::FlightSim>(
    view: &FlightView<C, S>,
    id: NpcId,
) -> nova_sim::flight_frame::FrameShip {
    *view.frame.ship(ShipRef::Npc(id)).expect("in the frame")
}

/// The index in `list` of the first sprite centred on `at`, or of a
/// crossed box's diagonal centred on it.
fn drawn_at(list: &DrawList, at: Point) -> usize {
    let near = |p: Point| (p.x - at.x).abs() < 0.01 && (p.y - at.y).abs() < 0.01;
    list.iter()
        .position(|command| match *command {
            DrawCommand::Sprite { center, .. } => near(center),
            DrawCommand::Line { from, to, .. } => near(Point::new(
                f32::midpoint(from.x, to.x),
                f32::midpoint(from.y, to.y),
            )),
            _ => false,
        })
        .unwrap_or_else(|| panic!("nothing drawn at {at:?}"))
}
/// Quiet sound preferences: effects at level 1.
const QUIET: SoundPrefs = SoundPrefs {
    sound: true,
    music: true,
    effects_level: 1,
    music_level: 7,
};

use crate::flight::floating;
use crate::galaxy::map::{
    BRIGHT_GREEN, CLEARED, CYAN, DARK_GREY, DENIED, DIM_GREEN, HOSTILE, LESS_DARK_GREY, LIGHT_GREY,
};
use crate::geometry::Bounds;
use nova_sim::mission::screens::mac_ticks;

/// The floating map's border lines in `list`: those in the
/// `FloatingMap` colour on the frame's edges, whatever their alpha.
fn floating_border(list: &DrawList) -> Vec<DrawCommand> {
    let border = MENU_COLORS.border;
    list.iter()
        .filter(|command| {
            matches!(command, DrawCommand::Line { from, color, .. }
                if (color.r, color.g, color.b) == (border.r, border.g, border.b)
                    && from.x >= 558.0)
        })
        .cloned()
        .collect()
}

/// The alpha the floating map's border is drawn at, if it is drawn.
fn floating_alpha(view: &FlightView<FakeCatalog, impl FlightSim>) -> Option<u8> {
    floating_border(&drawn(view))
        .first()
        .map(|command| match command {
            DrawCommand::Line { color, .. } => color.a,
            _ => unreachable!(),
        })
}
