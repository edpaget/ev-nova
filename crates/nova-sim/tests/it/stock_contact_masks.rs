//! The masks the contact passes and the scoop touch by, over the stock
//! data (see `nova_sim::combat::contact` and `nova_sim::scoop`): every
//! stock sheet a contact reads is read, so stock never falls back to the
//! swept discs; every stock ship's `shän` sizes are its sheet's; a stock
//! shot over a stock ship's middle touches it by the masks; and the
//! Asteroid Miner on a box's point takes it. Skips, passing, when
//! `NOVA_DATA` is unset.

use std::collections::BTreeSet;
use std::rc::Rc;

use crate::common;

use nova_sim::cargo_box::CargoBoxes;
use nova_sim::catalog::MaskSheet;
use nova_sim::combat::armament::Arsenal;
use nova_sim::geometry::Vec2;
use nova_sim::scoop::{BoxFrames, BoxSheets, Scooping};
use nova_sim::sprite_mask::{MaskWalk, PlacedMask, RleSheet};
use nova_sim::sprite_place::Placement;
use nova_sim::{AsteroidCatalog, CombatCatalog, NeverFires, PilotCatalog, ShipId};

/// `sheet` read from `data`.
fn read(data: &impl PilotCatalog, sheet: MaskSheet) -> Option<Rc<RleSheet>> {
    data.rle_sheet(sheet).map(Rc::new)
}

#[test]
fn every_stock_sheet_a_contact_reads_is_read_at_its_shans_size() {
    let Some(data) = common::opened() else {
        return;
    };
    let hulls = data.hulls();
    let bases: BTreeSet<ShipId> = hulls.iter().map(|hull| hull.sprite_base).collect();
    let mut unread = Vec::new();
    for hull in &hulls {
        match read(&data, MaskSheet::Ship(hull.sprite_base)) {
            Some(rle) => {
                let shan = (hull.size.map(i32::from), hull.height.map(i32::from));
                let sheet = (
                    i32::try_from(rle.width).ok(),
                    i32::try_from(rle.height).ok(),
                );
                assert_eq!(shan, sheet, "shïp {}: its shän's sizes", hull.id.0);
            }
            None => unread.push(MaskSheet::Ship(hull.sprite_base)),
        }
    }
    let arsenal = Arsenal::read(&data);
    let graphics: BTreeSet<i16> = arsenal
        .weapons()
        .filter(|weapon| weapon.spawns_shot() && !weapon.is_beam() && weapon.graphic >= 0)
        .map(|weapon| weapon.graphic)
        .collect();
    let shots = graphics.iter().map(|&graphic| MaskSheet::Shot(graphic));
    let kinds = data.asteroid_types();
    let asteroids = kinds.iter().map(|kind| MaskSheet::Asteroid(kind.kind));
    let boxes = (0..5).map(MaskSheet::CargoBox);
    unread.extend(
        shots
            .chain(asteroids)
            .chain(boxes)
            .filter(|&sheet| read(&data, sheet).is_none()),
    );
    assert_eq!(unread, [], "every one is read");
    assert_eq!(
        (bases.len(), graphics.len(), kinds.len()),
        (65, 24, 16),
        "sprite bases, shot graphics and asteroid kinds"
    );
}

#[test]
fn a_stock_shot_over_each_wide_stock_ships_middle_touches_it_by_the_masks() {
    let Some(data) = common::opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    // The first stock weapon's shot, `wëap` 128.
    let blaster = arsenal
        .weapons()
        .find(|weapon| weapon.id.0 == 128)
        .expect("wëap 128");
    let shot_sheet = read(&data, MaskSheet::Shot(blaster.graphic)).expect("its sheet");
    let shot = PlacedMask {
        rect: Placement::Shot.rect(Vec2::ZERO, (shot_sheet.width, shot_sheet.height)),
        sheet: shot_sheet,
        frame: 0,
    };
    let mut missed = Vec::new();
    let mut wide = 0;
    for base in data
        .hulls()
        .iter()
        .map(|hull| hull.sprite_base)
        .collect::<BTreeSet<_>>()
    {
        let sheet = read(&data, MaskSheet::Ship(base)).expect("read");
        if sheet.width <= 32 {
            continue;
        }
        wide += 1;
        let ship = PlacedMask {
            rect: Placement::Standard.rect(Vec2::ZERO, (sheet.width, sheet.height)),
            sheet,
            frame: 0,
        };
        if ship.touches(&shot, MaskWalk::Original) != Some(true) {
            missed.push(base.0);
        }
    }
    assert!(wide > 20, "{wide}");
    assert_eq!(missed, Vec::<i16>::new(), "a clear middle");
}

#[test]
fn the_stock_asteroid_miner_on_a_boxs_point_takes_it_at_every_rotation() {
    let Some(data) = common::opened() else {
        return;
    };
    // The Asteroid Miner, the one stock miner that scoops (`stock_miners`).
    let miner = data
        .hulls()
        .into_iter()
        .find(|hull| hull.id == ShipId(379))
        .expect("shïp 379");
    let sheet = read(&data, MaskSheet::Ship(miner.sprite_base)).expect("its sheet");
    let sheets = BoxSheets(std::array::from_fn(|graphic| {
        read(
            &data,
            MaskSheet::CargoBox(u16::try_from(graphic).expect("a graphic")),
        )
    }));
    let frames = BoxFrames::new(data.box_frame_sizes());
    let mut missed = Vec::new();
    for frame in 0..36 {
        let mask = PlacedMask {
            sheet: Rc::clone(&sheet),
            frame,
            rect: Placement::Standard.rect(Vec2::ZERO, (sheet.width, sheet.height)),
        };
        for graphic in 1..=4 {
            let mut boxes = CargoBoxes::default();
            boxes.spawn_scoopable(Vec2::ZERO, Some(1), graphic, &mut NeverFires);
            let scooping = Scooping {
                mask: Some(&mask),
                sheets: Some(&sheets),
                ..Scooping::disc(Vec2::ZERO, 0.0)
            };
            if boxes.take_touching(&scooping, &frames).is_none() {
                missed.push((frame, graphic));
            }
        }
    }
    assert_eq!(missed, [], "(rotation, box graphic)");
}
