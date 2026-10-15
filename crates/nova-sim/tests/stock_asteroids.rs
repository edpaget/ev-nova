//! The asteroid fields over the stock data. Skips, passing, when
//! `NOVA_DATA` is unset.
//!
//! - Every stock `röid`, 128-143, reads, its master sprite (`spïn` 800 +
//!   type) counting frames and sizing them.
//! - Every stock system's `AstTypes` names only types that read, and
//!   some system holds asteroids.
//! - Every stock `röid`'s fragment types name a type that reads, and some
//!   `röid` yields boxes of a kept `YieldType`.

mod common;

use nova_data::GameData;
use nova_sim::AsteroidCatalog;
use nova_sim::asteroid::AsteroidType;

#[test]
fn every_stock_asteroid_type_reads_with_frames_and_a_size() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let types = data.asteroid_types();
    let kinds: Vec<u8> = types.iter().map(|record| record.kind).collect();
    assert_eq!(kinds, (0..16).collect::<Vec<u8>>());
    for record in &types {
        assert!(record.frames > 0, "{record:?}");
        assert!(
            record
                .frame_size
                .is_some_and(|(width, height)| width > 0 && height > 0),
            "{record:?}"
        );
    }
}

#[test]
fn every_stock_field_names_types_that_read() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let read: u16 = data
        .asteroid_types()
        .iter()
        .fold(0, |bits, record| bits | 1 << record.kind);
    let fields = data.asteroid_fields();
    assert!(
        fields
            .iter()
            .any(|field| field.count > 0 && field.types != 0),
        "some system holds asteroids"
    );
    for field in fields {
        assert_eq!(field.types & !read, 0, "{field:?}");
        assert!((0..=16).contains(&field.count), "{field:?}");
    }
}

#[test]
fn every_stock_fragment_type_reads_and_some_asteroid_yields_boxes() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let records = data.asteroid_types();
    let read: Vec<u8> = records.iter().map(|record| record.kind).collect();
    for record in &records {
        let kind = AsteroidType::from_record(record);
        for (raw, fragment) in record.frag_types.into_iter().zip(kind.fragments) {
            if raw == -1 {
                continue;
            }
            let fragment = fragment.unwrap_or_else(|| panic!("{raw} names no type: {record:?}"));
            assert!(read.contains(&fragment), "{record:?}");
        }
    }
    assert!(
        records.iter().any(|record| {
            let kind = AsteroidType::from_record(record);
            kind.yield_qty > 0 && kind.yield_type.is_some()
        }),
        "some asteroid yields boxes"
    );
}
