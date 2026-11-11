//! The records the new game's data maps to: one neutral record per type,
//! and the small pure conversions the mapping applies over it.

use nova_galaxy_format::id::Color;
use nova_galaxy_format::model::{Service, StellarKind};

use crate::records::character::Character;
use crate::records::desc::Desc;
use crate::records::dude::Dude;
use crate::records::govt::Govt;
use crate::records::interface::Interface;
use crate::records::mission::Mission;
use crate::records::outfit::Outfit;
use crate::records::ship::Ship;
use crate::records::ship_anim::ShipAnim;
use crate::records::stellar::{Stellar, flags};
use crate::records::system::System;
use crate::records::weapon::Weapon;
use crate::wire::flags::{Flags16, Flags32, Flags64};
use crate::wire::geometry::Rect;
use crate::wire::raw::RawArray;
use crate::wire::string::MacString;

/// A `gövt` that sets nothing: no flags, penalties, classes, names or
/// colours. Class, ally and enemy slots are -1 (unused) and the voice is
/// -1 (silent).
#[must_use]
pub fn neutral_govt() -> Govt {
    Govt {
        voice_type: -1,
        flags: Flags16::default(),
        flags2: Flags16::default(),
        scan_fine: 0,
        crime_tol: 0,
        smug_penalty: 0,
        disab_penalty: 0,
        board_penalty: 0,
        kill_penalty: 0,
        shoot_penalty: 0,
        initial_rec: 0,
        max_odds: 0,
        class: [-1; 4],
        ally: [-1; 4],
        enemy: [-1; 4],
        skill_mult: 0,
        scan_mask: Flags16::default(),
        comm_name: MacString::default(),
        target_code: MacString::default(),
        require: Flags64::default(),
        inh_jam: [0; 4],
        medium_name: MacString::default(),
        color: 0,
        ship_color: 0,
        interface: 0,
        news_pic: 0,
        unknown_0xb0: RawArray::default(),
    }
}

/// A `sÿst` at the map's origin with nothing in it: no links, stellars,
/// traffic, persons, asteroids, message or reinforcements, and always
/// visible.
#[must_use]
pub fn neutral_system() -> System {
    System {
        x_pos: 0,
        y_pos: 0,
        con: [None; 16],
        nav_def: [None; 16],
        dude_types: [-1; 8],
        prob: [0; 8],
        avg_ships: 0,
        govt: None,
        message: -1,
        asteroids: 0,
        interference: 0,
        person: [None; 8],
        person_prob: [0; 8],
        bkgnd_color: 0,
        murk: 0,
        ast_types: Flags16::default(),
        visibility: MacString::default(),
        reinf_fleet: None,
        reinf_time: 0,
        reinf_intrval: 0,
        unknown_0x19c: RawArray::default(),
    }
}

/// A `spöb` at its system's centre that does nothing: no flags (no
/// landing), tech, government, picture, sound, defence, links, scripts,
/// fee, gravity or weapon; invincible, never destroyed.
#[must_use]
pub fn neutral_stellar() -> Stellar {
    Stellar {
        x_pos: 0,
        y_pos: 0,
        graphic_type: 0,
        flags: Flags32::default(),
        tribute: 0,
        tech_level: 0,
        special_tech1_3: [-1; 3],
        govt: None,
        min_status: 0,
        cust_pic_id: 0,
        cust_snd_id: -1,
        defense_dude: None,
        def_count: 0,
        flags2: Flags16::default(),
        anim_delay: 0,
        frame0_bias: 0,
        hyper_link: [None; 8],
        on_dominate: MacString::default(),
        on_release: MacString::default(),
        fee: 0,
        gravity: 0,
        weapon: None,
        strength: 0,
        dead_type: -1,
        dead_time: -1,
        explod_type: -1,
        on_destroy: MacString::default(),
        on_regen: MacString::default(),
        special_tech4_8: [-1; 5],
        unknown_0x44e: RawArray::default(),
    }
}

/// A `shïp` that does nothing: no stats, weapons, outfits, flags, names,
/// scripts or price; no crew, government, explosion, escort type, key or
/// upgrade. Its weapon, outfit, key and upgrade slots are none, and its
/// `InherentGovt`, `Explode1`, `Explode2` and `EscortType` are -1 (unused).
#[must_use]
pub fn neutral_ship() -> Ship {
    Ship {
        holds: 0,
        shield: 0,
        accel: 0,
        speed: 0,
        maneuver: 0,
        fuel: 0,
        free_mass: 0,
        armor: 0,
        shield_rech: 0,
        weap_type1_4: [None; 4],
        weap_count1_4: [0; 4],
        ammo_load1_4: [0; 4],
        max_gun: 0,
        max_tur: 0,
        tech_level: 0,
        cost: 0,
        death_delay: 0,
        armor_rech: 0,
        explode1: -1,
        explode2: -1,
        disp_weight: 0,
        mass: 0,
        length: 0,
        inherent_ai: 0,
        crew: 0,
        strength: 0,
        inherent_govt: -1,
        flags: Flags16::default(),
        pod_count: 0,
        default_items1_4: [None; 4],
        item_count1_4: [0; 4],
        fuel_regen: 0,
        skill_var: 0,
        flags2: Flags16::default(),
        contribute: Flags64::default(),
        availability: MacString::default(),
        appear_on: MacString::default(),
        on_purchase: MacString::default(),
        deionize: 0,
        ionize_max: 0,
        key_carried: None,
        default_items5_8: [None; 4],
        item_count5_8: [0; 4],
        require: Flags64::default(),
        buy_random: 0,
        hire_random: 0,
        unknown_0x38c: RawArray::default(),
        on_capture: MacString::default(),
        on_retire: MacString::default(),
        short_name: MacString::default(),
        comm_name: MacString::default(),
        long_name: MacString::default(),
        movie_file: MacString::default(),
        weap_type5_8: [None; 4],
        weap_count5_8: [0; 4],
        ammo_load5_8: [0; 4],
        subtitle: MacString::default(),
        flags3: Flags16::default(),
        upgrade_to: None,
        esc_upgrd_cost: 0,
        esc_sell_value: 0,
        escort_type: -1,
        unknown_0x734: RawArray::default(),
    }
}

/// A `shän` that draws nothing: no images, sizes, frames, blinking or
/// weapon positions. The view's sheet lookup fails for it, so its ship
/// draws as the view's crossed box.
#[must_use]
pub fn neutral_ship_anim() -> ShipAnim {
    ShipAnim {
        base_image_id: 0,
        base_mask_id: 0,
        base_set_count: 0,
        base_x_size: 0,
        base_y_size: 0,
        base_transp: 0,
        alt_image_id: 0,
        alt_mask_id: 0,
        alt_set_count: 0,
        alt_x_size: 0,
        alt_y_size: 0,
        glow_image_id: 0,
        glow_mask_id: 0,
        glow_x_size: 0,
        glow_y_size: 0,
        light_image_id: 0,
        light_mask_id: 0,
        light_x_size: 0,
        light_y_size: 0,
        weap_image_id: 0,
        weap_mask_id: 0,
        weap_x_size: 0,
        weap_y_size: 0,
        flags: Flags16::default(),
        anim_delay: 0,
        weap_decay: 0,
        frames_per: 0,
        blink_mode: 0,
        blink_val_a: 0,
        blink_val_b: 0,
        blink_val_c: 0,
        blink_val_d: 0,
        shield_image_id: 0,
        shield_mask_id: 0,
        shield_x_size: 0,
        shield_y_size: 0,
        gun_pos_x: [0; 4],
        gun_pos_y: [0; 4],
        turret_pos_x: [0; 4],
        turret_pos_y: [0; 4],
        guided_pos_x: [0; 4],
        guided_pos_y: [0; 4],
        beam_pos_x: [0; 4],
        beam_pos_y: [0; 4],
        up_compress_x: 0,
        up_compress_y: 0,
        dn_compress_x: 0,
        dn_compress_y: 0,
        gun_pos_z: [0; 4],
        turret_pos_z: [0; 4],
        guided_pos_z: [0; 4],
        beam_pos_z: [0; 4],
        unknown_0xb0: RawArray::default(),
    }
}

/// A `düde` that spawns nothing: no ships, AI type, government, booty or
/// info types.
#[must_use]
pub fn neutral_dude() -> Dude {
    Dude {
        ai_type: 0,
        govt: None,
        booty: Flags16::default(),
        info_types: Flags16::default(),
        ship_type: [None; 16],
        probability: [0; 16],
        unknown_0x48: RawArray::default(),
    }
}

/// A `mïsn` that asks nothing and is never offered: no availability,
/// pay, ships, texts, scripts, buttons, flags or time limit; no cargo
/// (`CargoType` -1) and no destination or return (`TravelStel` and
/// `ReturnStel` -1); every `dësc`, `STR#`, `düde` and `gövt` reference
/// none.
#[must_use]
pub fn neutral_mission() -> Mission {
    Mission {
        avail_stel: 0,
        unknown_0x02: 0,
        avail_loc: 0,
        avail_record: 0,
        avail_rating: 0,
        avail_random: 0,
        travel_stel: -1,
        return_stel: -1,
        cargo_type: -1,
        cargo_qty: 0,
        pickup_mode: 0,
        drop_off_mode: 0,
        scan_mask: Flags16::default(),
        unknown_0x1a: 0,
        pay_val: 0,
        ship_count: 0,
        ship_syst: 0,
        ship_dude: None,
        ship_goal: 0,
        ship_behav: 0,
        ship_name_id: None,
        ship_start: 0,
        comp_govt: None,
        comp_reward: 0,
        ship_subtitle: None,
        brief_text: None,
        quick_brief: None,
        load_carg_text: None,
        dump_cargo_text: None,
        comp_text: None,
        fail_text: None,
        time_limit: 0,
        can_abort: 0,
        ship_done_text: None,
        unknown_0x46: 0,
        aux_ship_count: 0,
        aux_ship_dude: None,
        aux_ship_syst: 0,
        unknown_0x4e: 0,
        flags: Flags16::default(),
        flags2: Flags16::default(),
        unknown_0x54: RawArray::default(),
        refuse_text: None,
        avail_ship_type: 0,
        avail_bits: MacString::default(),
        on_accept: MacString::default(),
        on_refuse: MacString::default(),
        on_success: MacString::default(),
        on_failure: MacString::default(),
        on_abort: MacString::default(),
        require: Flags64::default(),
        date_post_inc: 0,
        on_ship_done: MacString::default(),
        accept_button: MacString::default(),
        refuse_button: MacString::default(),
        disp_weight: 0,
        unknown_0x7a2: RawArray::default(),
    }
}

/// A `chär` that starts nowhere in nothing: no cash, ship, systems, legal
/// records, kills, intro, script, flags, date or date affixes.
#[must_use]
pub fn neutral_character() -> Character {
    Character {
        cash: 0,
        ship_type: None,
        system: [None; 4],
        govt: [None; 4],
        status: [0; 4],
        kills: 0,
        intro_pict: [None; 4],
        pict_delay: [0; 4],
        intro_text_id: None,
        on_start: MacString::default(),
        flags: Flags16::default(),
        unknown_0x134: 0,
        unknown_0x136: 0,
        start_year: 0,
        date_prefix: MacString::default(),
        date_suffix: MacString::default(),
        unknown_0x15a: RawArray::default(),
    }
}

/// The `oütf` `TechLevel` no outfitter lists, under either reading of the
/// tech level rule: 0x7fff, which the original's
/// `_SetupPortAvailableItems` skips (@0xbf93).
pub const OUTFIT_TECH_NEVER: i16 = 0x7fff;

/// An `oütf` that does nothing and is never sold: no mass, price, flags,
/// names, scripts, requirements or contribution; every `ModType` slot
/// unused (-1, with a `ModVal` of 0, as the stock data leaves an empty
/// slot); no limit on how many are owned (`Max` `i16::MAX`); never listed
/// ([`OUTFIT_TECH_NEVER`]) and never offered (`BuyRandom` 0); for every
/// government (`RequireGovt` -1).
#[must_use]
pub fn neutral_outfit() -> Outfit {
    Outfit {
        disp_weight: 0,
        mass: 0,
        tech_level: OUTFIT_TECH_NEVER,
        mod_type: -1,
        mod_val: 0,
        max: i16::MAX,
        flags: Flags16::default(),
        cost: 0,
        mod_type2: -1,
        mod_val2: 0,
        mod_type3: -1,
        mod_val3: 0,
        mod_type4: -1,
        mod_val4: 0,
        contribute: Flags64::default(),
        require: Flags64::default(),
        availability: MacString::default(),
        on_purchase: MacString::default(),
        on_sell: MacString::default(),
        short_name: MacString::default(),
        lc_name: MacString::default(),
        lc_plural: MacString::default(),
        item_class: 0,
        scan_mask: Flags16::default(),
        buy_random: 0,
        require_govt: -1,
        unknown_0x3f4: RawArray::default(),
    }
}

/// The [`neutral_outfit`] with `name` as written as its `ShortName`,
/// which the outfitter's grid cell draws, and lower-cased as its
/// `LCName`, as a good's `LCName` is ([`super::goods`]). Its `LCPlural`
/// stays blank: an English plural doesn't derive from a name.
#[must_use]
pub fn named_outfit(name: &str) -> Outfit {
    Outfit {
        short_name: name.into(),
        lc_name: name.to_lowercase().as_str().into(),
        ..neutral_outfit()
    }
}

/// A `wëap` that does nothing: no reload, life, damage, speed, flags,
/// particles, beam, submunitions or ammunition limit; unguided
/// (`Guidance` -1), using no ammunition (`AmmoType` -1), drawn with
/// `Graphic` 0, whose `spïn` (3000) the new game's data never holds, so
/// the view draws its shot as the placeholder box (a `Graphic` read above
/// 255 unsigned, a negative one among them, spawns no shot at all:
/// `nova_sim::combat::weapon`), silent (`Sound` -1), exploding into
/// nothing (`ExplodType` -1) and leaving from the ship's centre
/// (`ExitType` -1).
#[must_use]
pub fn neutral_weapon() -> Weapon {
    Weapon {
        reload: 0,
        count: 0,
        mass_dmg: 0,
        energy_dmg: 0,
        guidance: -1,
        speed: 0,
        ammo_type: -1,
        graphic: 0,
        inaccuracy: 0,
        sound: -1,
        impact: 0,
        explod_type: -1,
        prox_radius: 0,
        blast_radius: 0,
        flags: Flags16::default(),
        seeker: Flags16::default(),
        smoke_set: 0,
        decay: 0,
        particles: 0,
        part_vel: 0,
        part_life_min: 0,
        part_life_max: 0,
        part_color: 0,
        beam_length: 0,
        beam_width: 0,
        falloff: 0,
        beam_color: 0,
        corona_color: 0,
        sub_count: 0,
        sub_type: None,
        sub_theta: 0,
        sub_limit: 0,
        prox_safety: 0,
        flags2: Flags16::default(),
        ionization: 0,
        hit_particles: 0,
        hit_part_life: 0,
        hit_part_vel: 0,
        hit_part_color: 0,
        recoil: 0,
        exit_type: -1,
        burst_count: 0,
        burst_reload: 0,
        jam_vuln: [0; 4],
        flags3: Flags16::default(),
        durability: 0,
        guided_turn: 0,
        max_ammo: 0,
        li_density: 0,
        li_amplitude: 0,
        ionize_color: 0,
        unknown_0x76: RawArray::default(),
    }
}

/// The `spöb` `MinStatus` that clears a pilot of any legal record:
/// -32767, which the clearance reads as "any record"
/// (`nova_sim::landing::ANY_RECORD`).
pub const ANY_RECORD: i16 = -32767;

/// A `dësc` holding `text` alone: no picture, movie or flags.
#[must_use]
pub fn text_desc(text: &str) -> Desc {
    Desc {
        text: MacString::from(text),
        graphic: None,
        movie_file: MacString::default(),
        flags: Flags16::default(),
    }
}

/// `color` (`#rrggbb`) as Nova writes colours: `00RRGGBB`.
#[must_use]
pub fn rgb(color: &Color) -> u32 {
    // The format checked the spelling, so the digits always parse.
    u32::from_str_radix(&color.as_str()[1..], 16).unwrap_or_default()
}

/// The `spöb` `Flags` of a stellar of `kind` offering `services`: landing
/// when it offers any, a bit for each of the bar, the commodity exchange,
/// the outfitter and the shipyard, the station bit for a station or an
/// outpost, and uninhabited (no refuelling) without `refuel`.
#[must_use]
pub fn stellar_flags(kind: StellarKind, services: &[Service]) -> u32 {
    let mut bits = if services.is_empty() {
        0
    } else {
        flags::CAN_LAND
    };
    for service in services {
        bits |= match service {
            Service::Bar => flags::BAR,
            Service::Trade => flags::TRADE_CENTER,
            Service::Outfitter => flags::OUTFITTER,
            Service::Shipyard => flags::SHIPYARD,
            Service::Refuel => 0,
        };
    }
    if matches!(kind, StellarKind::Station | StellarKind::Outpost) {
        bits |= flags::STATION;
    }
    if !services.contains(&Service::Refuel) {
        bits |= flags::UNINHABITED;
    }
    bits
}

/// `value` as an `i16`, or the nearest one with `false` when it is
/// outside.
#[must_use]
pub fn clamp_i16(value: i32) -> (i16, bool) {
    match i16::try_from(value) {
        Ok(value) => (value, true),
        Err(_) => (value.clamp(i16::MIN.into(), i16::MAX.into()) as i16, false),
    }
}

/// The placeholder status bar's radar: 170 px square, as stock `ïntf` 128's
/// is, at the bar's top. It stands until the bible authors an interface.
pub const PLACEHOLDER_RADAR: Rect = area(4, 12, 174, 182);
/// The placeholder shield gauge, under the radar. A placeholder until the
/// bible authors an interface.
pub const PLACEHOLDER_SHIELD: Rect = area(182, 12, 190, 182);
/// The placeholder armour gauge, under the shield gauge. A placeholder
/// until the bible authors an interface.
pub const PLACEHOLDER_ARMOR: Rect = area(194, 12, 202, 182);
/// The placeholder fuel gauge, under the armour gauge. A placeholder until
/// the bible authors an interface.
pub const PLACEHOLDER_FUEL: Rect = area(206, 12, 214, 182);
/// The placeholder navigation readout. A placeholder until the bible
/// authors an interface.
pub const PLACEHOLDER_NAV: Rect = area(218, 12, 250, 182);
/// The placeholder weapon readout. A placeholder until the bible authors an
/// interface.
pub const PLACEHOLDER_WEAPON: Rect = area(252, 12, 268, 182);
/// The placeholder target readout, 112 px tall as stock `ïntf` 128's is.
/// A placeholder until the bible authors an interface.
pub const PLACEHOLDER_TARGET: Rect = area(270, 12, 382, 182);
/// The placeholder cargo readout, the bar's lowest area: 86 px, tall enough
/// for its credits value's baseline 82 px down, ending inside a 480-tall view. A placeholder until the bible authors an interface.
pub const PLACEHOLDER_CARGO: Rect = area(384, 12, 470, 182);
/// The placeholder bright text colour, white. A placeholder until the bible
/// authors an interface.
pub const PLACEHOLDER_BRIGHT_TEXT: u32 = 0x00FF_FFFF;
/// The placeholder dim text colour, grey. A placeholder until the bible
/// authors an interface.
pub const PLACEHOLDER_DIM_TEXT: u32 = 0x0080_8080;
/// The placeholder bright radar colour (the player), light green. A
/// placeholder until the bible authors an interface.
pub const PLACEHOLDER_BRIGHT_RADAR: u32 = 0x0080_FF80;
/// The placeholder dim radar colour (everything else), dark green. A
/// placeholder until the bible authors an interface.
pub const PLACEHOLDER_DIM_RADAR: u32 = 0x0000_8000;
/// The placeholder shield gauge colour, blue. A placeholder until the bible
/// authors an interface.
pub const PLACEHOLDER_SHIELD_COLOR: u32 = 0x0040_80FF;
/// The placeholder armour gauge colour, red. A placeholder until the bible
/// authors an interface.
pub const PLACEHOLDER_ARMOR_COLOR: u32 = 0x00FF_4040;
/// The placeholder full fuel colour, yellow. A placeholder until the bible
/// authors an interface.
pub const PLACEHOLDER_FUEL_FULL: u32 = 0x00FF_D000;
/// The placeholder partial fuel colour, dark yellow. A placeholder until
/// the bible authors an interface.
pub const PLACEHOLDER_FUEL_PARTIAL: u32 = 0x0080_6800;
/// The placeholder status font. A placeholder until the bible authors an
/// interface.
pub const PLACEHOLDER_FONT: &str = "Geneva";
/// The placeholder status and subtitle font size. A placeholder until the
/// bible authors an interface.
pub const PLACEHOLDER_FONT_SIZE: i16 = 9;

const fn area(top: i16, left: i16, bottom: i16, right: i16) -> Rect {
    Rect {
        top,
        left,
        bottom,
        right,
    }
}

/// The new game's status bar, every value a placeholder until the bible
/// authors an interface: a radar at the top, then the shield, armour and
/// fuel gauges and the nav, weapon, target and cargo readouts stacked
/// below it, with no background picture (`StatusBkgnd` 0).
#[must_use]
pub fn default_interface() -> Interface {
    Interface {
        bright_text: PLACEHOLDER_BRIGHT_TEXT,
        dim_text: PLACEHOLDER_DIM_TEXT,
        radar_area: PLACEHOLDER_RADAR,
        bright_radar: PLACEHOLDER_BRIGHT_RADAR,
        dim_radar: PLACEHOLDER_DIM_RADAR,
        shield_area: PLACEHOLDER_SHIELD,
        shield_color: PLACEHOLDER_SHIELD_COLOR,
        armor_area: PLACEHOLDER_ARMOR,
        armor_color: PLACEHOLDER_ARMOR_COLOR,
        fuel_area: PLACEHOLDER_FUEL,
        fuel_full: PLACEHOLDER_FUEL_FULL,
        fuel_partial: PLACEHOLDER_FUEL_PARTIAL,
        nav_area: PLACEHOLDER_NAV,
        weap_area: PLACEHOLDER_WEAPON,
        targ_area: PLACEHOLDER_TARGET,
        cargo_area: PLACEHOLDER_CARGO,
        status_font: MacString::from(PLACEHOLDER_FONT),
        stat_font_size: PLACEHOLDER_FONT_SIZE,
        subtitle_size: PLACEHOLDER_FONT_SIZE,
        status_bkgnd: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::{Record, decode_bytes};

    /// The bar's width: nova-view's `STATUS_BAR_WIDTH`, whose own test pins
    /// the 194.
    const BAR_WIDTH: i16 = 194;
    /// The smallest view height the game draws.
    const VIEW_HEIGHT: i16 = 480;

    fn intf_areas(intf: &Interface) -> [Rect; 8] {
        [
            intf.radar_area,
            intf.shield_area,
            intf.armor_area,
            intf.fuel_area,
            intf.nav_area,
            intf.weap_area,
            intf.targ_area,
            intf.cargo_area,
        ]
    }

    #[test]
    fn every_default_area_is_non_empty_and_inside_the_bar() {
        for r in intf_areas(&default_interface()) {
            assert!(r.top >= 0 && r.left >= 0, "{r:?}");
            assert!(r.top < r.bottom && r.left < r.right, "{r:?}");
            assert!(r.right <= BAR_WIDTH && r.bottom <= VIEW_HEIGHT, "{r:?}");
        }
    }

    /// The descent below a baseline of the bar's 9-point font.
    const DESCENT: i16 = 3;
    /// The nav readout's lowest baseline:
    /// `nova_view::flight::hud::NAV_VALUE_BASELINE`.
    const NAV_DEEPEST: i16 = 29;
    /// The weapon readout's only baseline:
    /// `nova_view::flight::target::WEAPON_BASELINE`.
    const WEAPON_DEEPEST: i16 = 12;
    /// The target readout's lowest top-anchored baseline:
    /// `nova_view::flight::target::NO_TARGET_BASELINE`.
    const TARGET_DEEPEST: i16 = 47;
    /// The target's picture's height:
    /// `nova_view::flight::target::PICTURE_HEIGHT`.
    const TARGET_PICTURE: i16 = 64;
    /// The target's subtitle baseline,
    /// `nova_view::flight::target::SUBTITLE_BASELINE`, and its status line
    /// above the bottom, `nova_view::flight::target::BOTTOM_BASELINE`.
    const TARGET_SUBTITLE: i16 = 29;
    const TARGET_BOTTOM: i16 = 6;
    /// The cargo readout's lowest baseline:
    /// `nova_view::flight::hud::CARGO_CREDITS_VALUE_BASELINE`.
    const CARGO_DEEPEST: i16 = 82;

    /// A cheap sanity check against hand-copied nova-view baselines, which
    /// go stale if nova-view moves one. The authoritative check is the
    /// wiring test over nova-view's real drawing,
    /// `nova`'s `sample_status_bar::the_sample_draws_every_readout_inside_its_area`.
    #[test]
    fn every_default_readout_area_holds_its_readout() {
        let intf = default_interface();
        let h = |r: Rect| r.bottom - r.top;
        assert!(
            h(intf.nav_area) >= NAV_DEEPEST + DESCENT,
            "{:?}",
            intf.nav_area
        );
        assert!(
            h(intf.weap_area) >= WEAPON_DEEPEST + DESCENT,
            "{:?}",
            intf.weap_area
        );
        let targ = h(intf.targ_area);
        assert!(targ >= TARGET_DEEPEST + DESCENT, "{:?}", intf.targ_area);
        assert!(targ >= TARGET_PICTURE, "{:?}", intf.targ_area);
        assert!(
            targ - TARGET_BOTTOM - DESCENT > TARGET_SUBTITLE + DESCENT,
            "{:?}",
            intf.targ_area
        );
        assert!(
            h(intf.cargo_area) >= CARGO_DEEPEST + DESCENT,
            "{:?}",
            intf.cargo_area
        );
        let lowest = intf_areas(&intf).iter().map(|r| r.bottom).max();
        assert!(lowest <= Some(VIEW_HEIGHT), "{lowest:?}");
    }

    #[test]
    fn the_default_radar_is_topmost_and_no_areas_overlap() {
        let areas = intf_areas(&default_interface());
        let radar = areas[0];
        assert!(areas[1..].iter().all(|r| r.top >= radar.bottom));
        for (i, a) in areas.iter().enumerate() {
            for b in &areas[i + 1..] {
                let apart = a.bottom <= b.top
                    || b.bottom <= a.top
                    || a.right <= b.left
                    || b.right <= a.left;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
    }

    #[test]
    fn the_default_marks_have_distinct_colours() {
        let intf = default_interface();
        let colours = [
            intf.bright_radar,
            intf.dim_radar,
            intf.shield_color,
            intf.armor_color,
            intf.fuel_full,
            intf.fuel_partial,
        ];
        for (i, a) in colours.iter().enumerate() {
            assert!(!colours[i + 1..].contains(a), "{a:#08x} repeats");
        }
        assert_ne!(intf.bright_text, intf.dim_text);
    }

    #[test]
    fn the_default_bar_has_no_picture_and_positive_font_sizes() {
        let intf = default_interface();
        assert!(intf.status_bkgnd < 128);
        assert!(intf.stat_font_size > 0 && intf.subtitle_size > 0);
        assert_eq!(&*intf.status_font, "Geneva");
    }

    fn color(text: &str) -> Color {
        Color::try_from(text.to_owned()).expect("a colour")
    }

    #[test]
    fn a_colour_is_00rrggbb() {
        assert_eq!(rgb(&color("#aa8800")), 0x00AA_8800);
        assert_eq!(rgb(&color("#FFfF01")), 0x00FF_FF01);
        assert_eq!(rgb(&color("#000000")), 0);
    }

    #[test]
    fn a_number_inside_i16_is_kept_and_one_outside_is_clamped() {
        assert_eq!(clamp_i16(-300), (-300, true));
        assert_eq!(clamp_i16(32_767), (32_767, true));
        assert_eq!(clamp_i16(-32_768), (-32_768, true));
        assert_eq!(clamp_i16(32_768), (32_767, false));
        assert_eq!(clamp_i16(-40_000), (-32_768, false));
    }

    #[test]
    fn no_services_is_no_landing_and_uninhabited() {
        assert_eq!(stellar_flags(StellarKind::Planet, &[]), flags::UNINHABITED);
    }

    #[test]
    fn each_service_sets_its_bit_and_landing() {
        let inhabited = |service| stellar_flags(StellarKind::Planet, &[service, Service::Refuel]);
        let land = flags::CAN_LAND;
        assert_eq!(inhabited(Service::Bar), land | flags::BAR);
        assert_eq!(inhabited(Service::Trade), land | flags::TRADE_CENTER);
        assert_eq!(inhabited(Service::Outfitter), land | flags::OUTFITTER);
        assert_eq!(inhabited(Service::Shipyard), land | flags::SHIPYARD);
        assert_eq!(inhabited(Service::Refuel), land);
    }

    #[test]
    fn a_service_listed_twice_is_listed_once() {
        assert_eq!(
            stellar_flags(
                StellarKind::Planet,
                &[Service::Bar, Service::Bar, Service::Refuel]
            ),
            flags::CAN_LAND | flags::BAR
        );
    }

    #[test]
    fn without_refuel_a_stellar_is_uninhabited() {
        assert_eq!(
            stellar_flags(StellarKind::Planet, &[Service::Bar]),
            flags::CAN_LAND | flags::BAR | flags::UNINHABITED
        );
    }

    #[test]
    fn a_station_or_an_outpost_sets_the_station_bit_and_a_planet_or_moon_does_not() {
        let refuel = [Service::Refuel];
        for kind in [StellarKind::Station, StellarKind::Outpost] {
            assert_eq!(
                stellar_flags(kind, &refuel),
                flags::CAN_LAND | flags::STATION,
                "{kind:?}"
            );
        }
        for kind in [StellarKind::Planet, StellarKind::Moon] {
            assert_eq!(stellar_flags(kind, &refuel), flags::CAN_LAND, "{kind:?}");
        }
    }

    /// The all-zero record with `edit` applied.
    fn zeroed<T: Record>(edit: impl FnOnce(&mut T)) -> T {
        let size = T::SIZE.expect("a fixed size");
        let mut record = decode_bytes::<T>(&vec![0; size]).expect("decodes").record;
        edit(&mut record);
        record
    }

    #[test]
    fn the_neutral_govt_is_zero_but_its_unused_slots_and_silent_voice() {
        let expected = zeroed::<Govt>(|govt| {
            govt.voice_type = -1;
            govt.class = [-1; 4];
            govt.ally = [-1; 4];
            govt.enemy = [-1; 4];
        });
        assert_eq!(neutral_govt(), expected);
    }

    #[test]
    fn the_neutral_system_is_zero_but_its_unused_references() {
        let expected = zeroed::<System>(|system| {
            system.con = [None; 16];
            system.nav_def = [None; 16];
            system.dude_types = [-1; 8];
            system.govt = None;
            system.message = -1;
            system.person = [None; 8];
            system.reinf_fleet = None;
        });
        assert_eq!(neutral_system(), expected);
    }

    #[test]
    fn the_neutral_stellar_is_zero_but_its_unused_and_never_fields() {
        let expected = zeroed::<Stellar>(|stellar| {
            stellar.govt = None;
            stellar.defense_dude = None;
            stellar.hyper_link = [None; 8];
            stellar.weapon = None;
            stellar.special_tech1_3 = [-1; 3];
            stellar.special_tech4_8 = [-1; 5];
            stellar.cust_snd_id = -1;
            stellar.dead_type = -1;
            stellar.dead_time = -1;
            stellar.explod_type = -1;
        });
        assert_eq!(neutral_stellar(), expected);
    }

    #[test]
    fn the_neutral_ship_is_zero_but_its_unused_references() {
        let expected = zeroed::<Ship>(|ship| {
            ship.weap_type1_4 = [None; 4];
            ship.weap_type5_8 = [None; 4];
            ship.default_items1_4 = [None; 4];
            ship.default_items5_8 = [None; 4];
            ship.key_carried = None;
            ship.upgrade_to = None;
            ship.inherent_govt = -1;
            ship.explode1 = -1;
            ship.explode2 = -1;
            ship.escort_type = -1;
        });
        assert_eq!(neutral_ship(), expected);
    }

    #[test]
    fn the_neutral_ship_anim_is_zero() {
        assert_eq!(neutral_ship_anim(), zeroed::<ShipAnim>(|_| {}));
    }

    #[test]
    fn the_neutral_weapon_is_zero_but_its_unused_and_none_fields() {
        let expected = zeroed::<Weapon>(|weapon| {
            weapon.guidance = -1;
            weapon.ammo_type = -1;
            weapon.sound = -1;
            weapon.explod_type = -1;
            weapon.exit_type = -1;
            weapon.sub_type = None;
        });
        assert_eq!(neutral_weapon(), expected);
    }

    #[test]
    fn the_neutral_dude_is_zero_but_its_unused_references() {
        let expected = zeroed::<Dude>(|dude| {
            dude.govt = None;
            dude.ship_type = [None; 16];
        });
        assert_eq!(neutral_dude(), expected);
    }

    #[test]
    fn the_neutral_mission_is_zero_but_its_unused_references_cargo_and_places() {
        let expected = zeroed::<Mission>(|mission| {
            mission.travel_stel = -1;
            mission.return_stel = -1;
            mission.cargo_type = -1;
            mission.ship_dude = None;
            mission.ship_name_id = None;
            mission.comp_govt = None;
            mission.ship_subtitle = None;
            mission.brief_text = None;
            mission.quick_brief = None;
            mission.load_carg_text = None;
            mission.dump_cargo_text = None;
            mission.comp_text = None;
            mission.fail_text = None;
            mission.ship_done_text = None;
            mission.aux_ship_dude = None;
            mission.refuse_text = None;
        });
        assert_eq!(neutral_mission(), expected);
    }

    #[test]
    fn the_neutral_character_is_zero_but_its_unused_references() {
        let expected = zeroed::<Character>(|character| {
            character.ship_type = None;
            character.system = [None; 4];
            character.govt = [None; 4];
            character.intro_pict = [None; 4];
            character.intro_text_id = None;
        });
        assert_eq!(neutral_character(), expected);
    }

    #[test]
    fn a_text_desc_holds_only_its_text() {
        let desc = text_desc("A quiet place.");
        assert_eq!(desc.text.as_str(), "A quiet place.");
        assert_eq!(desc.graphic, None);
        assert_eq!(desc.movie_file.as_str(), "");
        assert_eq!(desc.flags, Flags16(0));
    }
}
