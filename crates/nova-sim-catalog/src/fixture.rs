//! Test support for the catalog layer's tests and the nova-sim crates'
//! above, behind the `fixture` feature: builders for the catalog's
//! records, each a plain record that tests change with struct-update
//! syntax; mocks of the [`ControlBits`] port; and [`bits_allowing`], the
//! control bits that make a test hold, for setting a pilot up directly.

mod bits;
mod pilot_catalog;

pub use bits::bits_allowing;
pub use pilot_catalog::{FakePilotCatalog, START, catalog, starting};

use std::collections::BTreeMap;

use crate::catalog::{
    AuxShip, CronId, CronRecord, EscortClass, GovtId, GovtRecord, HullRecord, LandingSite,
    ListedStellar, MissionId, MissionRecord, MissionShip, NebulaId, NebulaRecord, OutfitId,
    OutfitRecord, Penalties, PersonId, PersonRecord, RankId, RankRecord, ShipId, ShipRecord,
    StarSystem, StellarId, SystemId, WeaponId, WeaponRecord,
};
use crate::control::{ControlBits, PilotFacts, Script, Test, TestExpr};
use crate::geometry::Vec2;
use crate::handling::ShipFields;

/// Cron `id`, open on every date, starting on any roll and asking
/// nothing, running 0 days with no holdoffs, and with no scripts and no
/// `Contribute`.
#[must_use]
pub fn cron(id: i16) -> CronRecord {
    CronRecord {
        id: CronId(id),
        first_day: 0,
        first_month: 0,
        first_year: 0,
        last_day: 0,
        last_month: 0,
        last_year: 0,
        random: 100,
        duration: 0,
        pre_holdoff: 0,
        post_holdoff: 0,
        flags: 0,
        enable_on: Test::default(),
        on_start: Script::default(),
        on_end: Script::default(),
        contribute: 0,
        require: 0,
        news_govt: [-1; 4],
        govt_news: [-1; 4],
        ind_news: -1,
    }
}

/// Nebula `id` over `rect` (left, top, width, height), active by
/// `active_on` and running `on_explore` as it is explored.
#[must_use]
pub fn nebula(
    id: i16,
    rect: (i16, i16, i16, i16),
    active_on: &str,
    on_explore: &str,
) -> NebulaRecord {
    NebulaRecord {
        id: NebulaId(id),
        left: rect.0,
        top: rect.1,
        width: rect.2,
        height: rect.3,
        active_on: Test::parse(active_on),
        on_explore: Script::parse(on_explore),
    }
}

/// Rank `id`: unaffiliated, of `Weight` 0, paying nothing, with no cap,
/// no `Contribute`, no flags, every name empty and a `PriceMod` of 100.
#[must_use]
pub fn rank(id: i16) -> RankRecord {
    RankRecord {
        id: RankId(id),
        name: String::new(),
        weight: 0,
        affil_govt: None,
        salary: 0,
        salary_cap: 0,
        contribute: 0,
        flags: 0,
        conv_name: String::new(),
        short_name: String::new(),
        price_mod: 100,
    }
}

/// Mission `id`, offered at the mission computer of any stellar on every
/// visit, to anyone: going nowhere, carrying nothing, paying nothing and
/// with no deadline.
#[must_use]
pub fn mission(id: i16) -> MissionRecord {
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
        on_accept: Script::default(),
        on_refuse: Script::default(),
        on_success: Script::default(),
        on_failure: Script::default(),
        on_abort: Script::default(),
        on_ship_done: Script::default(),
        can_abort: 1,
        comp_govt: None,
        comp_reward: 0,
        date_post_inc: 0,
        ships: MissionShip::NONE,
        aux: AuxShip::NONE,
    }
}

/// A landable planet at (`x`, `y`), 100 x 100 (radius 50).
#[must_use]
pub fn planet(id: i16, x: f32, y: f32) -> LandingSite {
    LandingSite {
        id: StellarId(id),
        position: Vec2::new(x, y),
        frame_size: Some((100, 100)),
        // `nova_sim::landing::StellarFlags::CAN_LAND`: it can land.
        flags: 0x01,
        min_status: 0,
        tech_level: 0,
        special_tech: [0; 8],
        govt: None,
        flags2: 0,
        destroyed: false,
        graphic: 0,
        dead_graphic: None,
        dead_frame_size: None,
        type_counts_frames: true,
        exit_angle: -1,
        tribute: 0,
        gravity: 0,
        animation: crate::catalog::SiteAnimation::default(),
    }
}

/// `shïp` `id` with `fields`: 10,000 credits, tech level 1, for sale
/// every day (`BuyRandom` 100), never for hire (`HireRandom` 0),
/// requiring nothing, with no default items, no `Flags` or `Flags3` and no inherent
/// government, hailed as "ship `id`", a wimpy trader whose escort class
/// is worked out (`EscortType` -1).
#[must_use]
pub fn ship(id: i16, fields: ShipFields) -> ShipRecord {
    ShipRecord {
        id: ShipId(id),
        name: format!("Ship {id}"),
        short_name: format!("Ship\\n{id}"),
        long_name: format!("The Ship {id}"),
        fields,
        defaults: Vec::new(),
        cost: 10_000,
        tech_level: 1,
        buy_random: 100,
        hire_random: 0,
        require: 0,
        availability: Test::default(),
        appear_on: Test::default(),
        flags: 0,
        flags3: 0,
        disp_weight: 0,
        max_gun: 2,
        max_tur: 1,
        length: 20,
        crew: 3,
        inherent_ai: 1,
        comm_name: format!("ship {id}"),
        inherent_govt: None,
        combat_govt: None,
        escort_type: -1,
        on_capture: crate::control::Script::default(),
        on_purchase: crate::control::Script::default(),
        on_retire: crate::control::Script::default(),
    }
}

/// `wëap` `id`: an unguided weapon with unlimited ammo that does
/// nothing, with every other field none or unused (-1).
#[must_use]
pub fn weapon(id: i16) -> WeaponRecord {
    WeaponRecord {
        id: WeaponId(id),
        reload: 0,
        count: 0,
        mass_dmg: 0,
        energy_dmg: 0,
        guidance: -1,
        speed: 0,
        ammo_type: -1,
        graphic: 0,
        inaccuracy: 0,
        impact: 0,
        explod_type: -1,
        prox_radius: 0,
        blast_radius: 0,
        flags: 0,
        seeker: 0,
        flags2: 0,
        flags3: 0,
        decay: 0,
        beam_length: 0,
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
        hit_particles: 0,
        hit_part_life: 0,
        hit_part_vel: 0,
        hit_part_color: 0,
        recoil: 0,
        exit_type: -1,
        ionization: 0,
        ionize_color: 0,
        jam_vuln: [0; 4],
        frame: None,
    }
}

/// `shïp` `id`'s combat fields: no flags, no weapons, no `shän`, no
/// explosions, gone at once, massless and of no strength.
#[must_use]
pub fn hull(id: i16) -> HullRecord {
    HullRecord {
        id: ShipId(id),
        flags: 0,
        flags2: 0,
        flags3: 0,
        death_delay: 0,
        explode1: -1,
        explode2: -1,
        mass: 0,
        weapons: Vec::new(),
        size: None,
        height: None,
        strength: 0,
        key_carried: None,
        cost: 0,
        speed: 0,
        exits: None,
        escort_class: EscortClass::Freighter,
    }
}

/// `oütf` `id` with these mods (the unused ones none): tech level 1, a
/// ton, 1000 credits, up to 10 owned, no flags, requiring nothing, for
/// sale every day (`BuyRandom` 100), of
/// `ItemClass` 0, its lower-case names "outfit `id`" and "outfits `id`".
#[must_use]
pub fn outfit(id: i16, mods: &[(i16, i16)]) -> OutfitRecord {
    let mut pairs = [(0, 0); 4];
    pairs[..mods.len()].copy_from_slice(mods);
    OutfitRecord {
        id: OutfitId(id),
        name: format!("Outfit {id}"),
        short_name: format!("Outfit\\n{id}"),
        disp_weight: 0,
        mass: 1,
        tech_level: 1,
        max: 10,
        flags: 0,
        cost: 1000,
        mods: pairs,
        contribute: 0,
        require: 0,
        require_govt: -1,
        buy_random: 100,
        availability: Test::default(),
        item_class: 0,
        lc_name: format!("outfit {id}"),
        lc_plural: format!("outfits {id}"),
        scan_mask: 0,
        on_purchase: Script::default(),
        on_sell: Script::default(),
    }
}

/// `gövt` `id`: no flags, no classes, allies or enemies, no tolerance
/// for crime, no initial record, no penalties, and `MaxOdds` 100 (even odds), hailed as
/// "Govt `id`", scanning for nothing, fining nothing, requiring nothing
/// and jamming nothing.
#[must_use]
pub fn govt(id: i16) -> GovtRecord {
    GovtRecord {
        id: GovtId(id),
        name: String::new(),
        flags: 0,
        flags2: 0,
        crime_tol: 0,
        initial_rec: 0,
        penalties: Penalties::default(),
        max_odds: 100,
        classes: [-1; 4],
        allies: [-1; 4],
        enemies: [-1; 4],
        comm_name: format!("Govt {id}"),
        scan_mask: 0,
        scan_fine: 0,
        news_pic: -1,
        require: 0,
        inh_jam: [0; 4],
    }
}

/// `përs` `id`, "Person `id`", flying ship `ship`: a warship of no
/// government and `Aggress` 2 that may appear anywhere (`LinkSyst` -1),
/// with no `Coward`, extra weapons, credits, `ShieldMod`, quotes,
/// picture, mission, flags, `ActiveOn` or grant.
#[must_use]
pub fn person(id: i16, ship: i16) -> PersonRecord {
    PersonRecord {
        id: PersonId(id),
        name: format!("Person {id}"),
        link_syst: -1,
        govt: None,
        ai_type: 3,
        aggress: 2,
        coward: 0,
        ship: Some(ShipId(ship)),
        weapons: Vec::new(),
        credits: 0,
        shield_mod: 0,
        hail_pict: None,
        comm_quote: -1,
        hail_quote: -1,
        link_mission: None,
        flags: 0,
        active_on: Test::default(),
        subtitle: String::new(),
        flags2: 0,
        grant_class: 0,
        grant_count: 0,
        grant_prob: 0,
        mission_ship: None,
    }
}

/// System `id` at (`x`, `y`) on the map, linked to `links`: of no
/// government, holding no stellars, with no interference and no buoy
/// message.
#[must_use]
pub fn star(id: i16, (x, y): (f32, f32), links: &[i16]) -> StarSystem {
    StarSystem {
        id: SystemId(id),
        position: Vec2::new(x, y),
        links: links.iter().copied().map(SystemId).collect(),
        govt: None,
        stellars: Vec::new(),
        interference: 0,
        murk: 0,
        message: -1,
    }
}

/// Control bits that refuse every test reading one of these bits, and
/// hold every other.
#[derive(Clone, Copy, Debug)]
pub struct RefuseBits(pub &'static [u16]);

impl ControlBits for RefuseBits {
    fn allows(&self, test: &TestExpr, _pilot: &dyn PilotFacts) -> bool {
        !test.reads().iter().any(|bit| self.0.contains(&bit.get()))
    }
}

/// Control bits that hold every test asked.
#[derive(Clone, Copy, Debug)]
pub struct AllowAll;

impl ControlBits for AllowAll {
    fn allows(&self, _test: &TestExpr, _pilot: &dyn PilotFacts) -> bool {
        true
    }
}

/// Every stellar of `sites` (each system's landing sites, as a fake
/// `PilotCatalog::landing_sites` gives them), by ascending ID, with every
/// system listing it, by ascending ID, each once, and the flags and
/// government of its first site: the listed stellars
/// (`PilotCatalog::listed_stellars`) a fake gives for those sites, as the
/// data's `NavDefs` give them. A session shows only the stellars its
/// system holds by these, so a fake that lists none shows none.
#[must_use]
pub fn listed_stellars<'a>(
    sites: impl IntoIterator<Item = (SystemId, &'a [LandingSite])>,
) -> Vec<ListedStellar> {
    let mut listed: BTreeMap<StellarId, ListedStellar> = BTreeMap::new();
    for (system, sites) in sites {
        for site in sites {
            let stellar = listed.entry(site.id).or_insert_with(|| ListedStellar {
                id: site.id,
                systems: Vec::new(),
                flags: site.flags,
                flags2: site.flags2,
                govt: site.govt,
                min_status: site.min_status,
            });
            if !stellar.systems.contains(&system) {
                stellar.systems.push(system);
                stellar.systems.sort_unstable();
            }
        }
    }
    listed.into_values().collect()
}

/// Adds to `listed` (as [`listed_stellars`] gives it) each `(system,
/// stellar)` of `listers`: a `NavDefs` entry with no landing site, such as
/// a gate in a system the ship never flies in. A stellar not yet listed is
/// added with no flags and no government.
pub fn list_also(
    listed: &mut Vec<ListedStellar>,
    listers: impl IntoIterator<Item = (SystemId, StellarId)>,
) {
    for (system, stellar) in listers {
        let at = listed
            .binary_search_by_key(&stellar, |listed| listed.id)
            .unwrap_or_else(|at| {
                listed.insert(
                    at,
                    ListedStellar {
                        id: stellar,
                        systems: Vec::new(),
                        flags: 0,
                        flags2: 0,
                        govt: None,
                        min_status: 0,
                    },
                );
                at
            });
        let systems = &mut listed[at].systems;
        if !systems.contains(&system) {
            systems.push(system);
            systems.sort_unstable();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cron_carries_no_news() {
        let cron = cron(128);
        assert_eq!((cron.news_govt, cron.govt_news), ([-1; 4], [-1; 4]));
        assert_eq!(cron.ind_news, -1);
    }

    #[test]
    fn a_mission_goes_nowhere_and_carries_nothing() {
        let mission = mission(128);
        assert_eq!(mission.avail_stel, -1, "offered at any stellar");
        assert_eq!((mission.travel_stel, mission.return_stel), (-1, -1));
        assert_eq!(mission.cargo_type, -1);
        assert_eq!((mission.pickup_mode, mission.drop_off_mode), (-1, -1));
    }

    #[test]
    fn a_planet_can_be_landed_on_and_has_no_exit_angle() {
        let planet = planet(128, 1.0, 2.0);
        assert_eq!(planet.flags, 0x01);
        assert_eq!(planet.exit_angle, -1);
        assert_eq!(planet.position, Vec2::new(1.0, 2.0));
    }

    #[test]
    fn a_ships_escort_class_is_worked_out() {
        assert_eq!(ship(128, ShipFields::default()).escort_type, -1);
    }

    #[test]
    fn a_weapon_is_unguided_with_unlimited_ammo_and_no_explosion_or_exit() {
        let weapon = weapon(128);
        assert_eq!((weapon.guidance, weapon.ammo_type), (-1, -1));
        assert_eq!((weapon.explod_type, weapon.exit_type), (-1, -1));
    }

    #[test]
    fn an_outfit_requires_no_government_and_pads_its_mods_with_none() {
        let outfit = outfit(300, &[(2, 5)]);
        assert_eq!(outfit.require_govt, -1, "no government required");
        assert_eq!(outfit.mods, [(2, 5), (0, 0), (0, 0), (0, 0)]);
    }

    #[test]
    fn a_hull_has_no_explosions() {
        let hull = hull(128);
        assert_eq!((hull.explode1, hull.explode2), (-1, -1));
    }

    #[test]
    fn a_government_has_no_classes_allies_enemies_or_news_picture() {
        let govt = govt(128);
        assert_eq!(govt.classes, [-1; 4]);
        assert_eq!((govt.allies, govt.enemies), ([-1; 4], [-1; 4]));
        assert_eq!(govt.news_pic, -1);
    }

    #[test]
    fn a_person_may_appear_anywhere_and_has_no_quotes() {
        let person = person(510, 128);
        assert_eq!(person.link_syst, -1);
        assert_eq!((person.comm_quote, person.hail_quote), (-1, -1));
        assert_eq!(person.ship, Some(ShipId(128)));
    }

    #[test]
    fn a_star_has_no_buoy_message() {
        let star = star(130, (1.0, 2.0), &[131]);
        assert_eq!(star.message, -1);
        assert_eq!(star.links, [SystemId(131)]);
    }

    #[test]
    fn the_listed_stellars_are_every_site_by_id_with_every_system_listing_it() {
        let flagged = LandingSite {
            flags: 4,
            flags2: 0x2000,
            ..planet(129, 0.0, 0.0)
        };
        let sites = [
            (SystemId(131), vec![flagged, planet(128, 0.0, 0.0)]),
            (
                SystemId(130),
                vec![planet(129, 1.0, 1.0), planet(129, 1.0, 1.0)],
            ),
        ];
        let listed = listed_stellars(sites.iter().map(|(system, sites)| (*system, &sites[..])));
        assert_eq!(
            listed,
            [
                ListedStellar {
                    id: StellarId(128),
                    systems: vec![SystemId(131)],
                    flags: planet(128, 0.0, 0.0).flags,
                    flags2: 0,
                    govt: None,
                    min_status: 0,
                },
                ListedStellar {
                    id: StellarId(129),
                    systems: vec![SystemId(130), SystemId(131)],
                    flags: 4,
                    flags2: 0x2000,
                    govt: None,
                    min_status: 0,
                },
            ],
            "by ascending ID, each system once and in order, the first site's flags"
        );
        assert_eq!(listed_stellars([]), []);
    }

    #[test]
    fn listing_also_adds_each_lister_in_order_and_each_stellar_by_id() {
        let sites = [(SystemId(131), vec![planet(129, 0.0, 0.0)])];
        let mut listed = listed_stellars(sites.iter().map(|(system, sites)| (*system, &sites[..])));
        list_also(
            &mut listed,
            [
                (SystemId(130), StellarId(129)),
                (SystemId(131), StellarId(129)),
                (SystemId(132), StellarId(140)),
                (SystemId(133), StellarId(128)),
            ],
        );
        let gate = |id, systems: &[i16]| ListedStellar {
            id: StellarId(id),
            systems: systems.iter().copied().map(SystemId).collect(),
            flags: 0,
            flags2: 0,
            govt: None,
            min_status: 0,
        };
        assert_eq!(
            listed,
            [
                gate(128, &[133]),
                ListedStellar {
                    systems: vec![SystemId(130), SystemId(131)],
                    ..listed_stellars([(SystemId(131), &[planet(129, 0.0, 0.0)][..])])[0].clone()
                },
                gate(140, &[132]),
            ]
        );
    }
}
