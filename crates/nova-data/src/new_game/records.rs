//! The records the new game's data maps to: one neutral record per type,
//! and the small pure conversions the mapping applies over it.

use nova_galaxy_format::id::Color;
use nova_galaxy_format::model::{Service, StellarKind};

use crate::records::character::Character;
use crate::records::desc::Desc;
use crate::records::dude::Dude;
use crate::records::govt::Govt;
use crate::records::ship::Ship;
use crate::records::ship_anim::ShipAnim;
use crate::records::stellar::{Stellar, flags};
use crate::records::system::System;
use crate::wire::flags::{Flags16, Flags32, Flags64};
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::{Record, decode_bytes};

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
    fn the_neutral_dude_is_zero_but_its_unused_references() {
        let expected = zeroed::<Dude>(|dude| {
            dude.govt = None;
            dude.ship_type = [None; 16];
        });
        assert_eq!(neutral_dude(), expected);
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
