//! The session's star map over the stock data: by the engine's
//! `HyperlinkRule`, a link listed by one system alone is one-way unless the
//! other system lists a system at the first one's position (a replacement
//! system and its original), and only the genuine one-way links are left.
//! Skips, passing, when `NOVA_DATA` is unset.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use nova_data::GameData;
use nova_data::records::system::System;
use nova_sim::{HyperlinkRule, PilotCatalog, StarMap, SystemId};

/// The stock links that stay one-way after allowing for replacement
/// systems, as `nova-data/tests/stock.rs` lists them.
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
