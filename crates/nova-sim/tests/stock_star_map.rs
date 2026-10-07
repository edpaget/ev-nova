//! The session's star map over the stock data: by the engine's
//! `HyperlinkRule`, a link listed by one system alone is one-way unless the
//! other system lists a system at the first one's position (a replacement
//! system and its original), and only the genuine one-way links are left.
//! Skips, passing, when `NOVA_DATA` is unset.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use nova_data::GameData;
use nova_data::records::system::System;
use nova_sim::{HyperlinkRule, PilotCatalog, StarMap, StellarId, SystemId, Vec2};

/// The stock links that stay one-way after allowing for replacement
/// systems: a copy of `ONE_WAY_LINKS` in `crates/nova-data/tests/stock.rs`,
/// which another crate's test binary cannot import.
const ONE_WAY_LINKS: &[(i16, i16)] = &[(558, 557), (561, 563), (584, 590), (585, 582), (625, 616)];

#[test]
fn stock_links_listed_by_one_system_route_one_way_unless_listed_back_by_position() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let raw: BTreeMap<i16, System> = data
        .records::<System>()
        .map(|(id, record)| (id, record.expect("every sÿst decodes").record.clone()))
        .collect();
    let map = StarMap::new(data.star_map());
    let position = |id: i16| (raw[&id].x_pos, raw[&id].y_pos);
    let directed: BTreeSet<(i16, i16)> = raw
        .iter()
        .flat_map(|(&from, system)| system.con.iter().flatten().map(move |to| (from, to.0)))
        .collect();
    let listed_by_one: Vec<(i16, i16)> = directed
        .iter()
        .copied()
        .filter(|&(from, to)| !directed.contains(&(to, from)))
        .collect();
    assert_eq!(listed_by_one.len(), 49);

    let route = |from: i16, to: i16| map.route(SystemId(from), SystemId(to), HyperlinkRule::Engine);
    let mut one_way = Vec::new();
    for &(a, b) in &listed_by_one {
        assert_eq!(route(a, b), Ok(vec![SystemId(b)]), "{a} -> {b}");
        let listed_back = raw[&b]
            .con
            .iter()
            .flatten()
            .any(|back| position(back.0) == position(a));
        assert_eq!(
            route(b, a) == Ok(vec![SystemId(a)]),
            listed_back,
            "{b} -> {a}"
        );
        if !listed_back {
            one_way.push((a, b));
        }
    }
    assert_eq!(one_way, ONE_WAY_LINKS);
}

/// HG-Kania (`spöb` 1404) sits at (-70, 250) in Kania (128), a hypergate
/// (`Flags2` 0x1200) linked to HG-Tichel, HG-Dani and HG-Koria, heading
/// ships out on 120°; HG-Dani is in Dani (298), the lowest of the systems
/// listing it, and wormhole 465 in Sol (130).
#[test]
fn stock_gate_sites_place_the_hypergates_and_wormholes() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let sites = data.gate_sites();
    let site = |id| {
        sites
            .iter()
            .find(|site| site.id == StellarId(id))
            .unwrap_or_else(|| panic!("spöb {id} is listed"))
    };
    let kania = site(1404);
    assert_eq!(kania.system, SystemId(128));
    assert_eq!(kania.position, Vec2::new(-70.0, 250.0));
    assert_eq!(kania.flags2, 0x1200);
    assert_eq!(kania.exit_angle, 120);
    let links: Vec<StellarId> = kania.links.iter().flatten().copied().collect();
    assert_eq!(links, [StellarId(1405), StellarId(1413), StellarId(1418)]);
    assert_eq!(site(1413).system, SystemId(298));
    assert_eq!(site(465).system, SystemId(130));
    assert_eq!(site(465).flags2 & 0x2000, 0x2000);
    assert!(
        sites.windows(2).all(|pair| pair[0].id < pair[1].id),
        "by ascending ID"
    );
}
