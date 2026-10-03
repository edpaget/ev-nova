//! Which layout each `rlëD` sprite sheet is composed with.
//!
//! An `rlëD` header gives frame size and count but not how many frames go
//! in a row: that comes from the record pointing at the sheet. This is an
//! export policy, not game data, so it lives here rather than in the store.

use std::collections::BTreeMap;

use nova_data::graphics::{RLED, SheetLayout};
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::store::GameData;

/// The layout of every `rlëD` some record claims, by `rlëD` ID.
///
/// Claims are made in this order, and the first claim on a sheet wins:
///
/// 1. every `shän`'s base image, by ascending `shän` ID, with the `shän`'s
///    frames per rotation as columns ([`ShipAnim::sheet_layout`]);
/// 2. every `shän`'s other layers (alt, glow, light, weapon and shield
///    images), the same way;
/// 3. every `spïn`'s sprites, by ascending `spïn` ID, with its horizontal
///    tile count as columns ([`Spin::sheet_layout`]).
///
/// Only IDs that exist as `rlëD` are claimed. A record that failed to
/// decode, or whose columns are zero or negative, claims nothing. An
/// unclaimed sheet is composed with [`SheetLayout::DEFAULT`].
#[must_use]
pub fn sheet_layouts(data: &GameData) -> BTreeMap<i16, SheetLayout> {
    let anims: Vec<&ShipAnim> = data
        .records::<ShipAnim>()
        .filter_map(|(_, result)| Some(result.ok()?.record))
        .collect();
    let base = anims
        .iter()
        .map(|a| (vec![a.base_image_id], a.sheet_layout()));
    let layers = anims.iter().map(|a| {
        let ids = vec![
            a.alt_image_id,
            a.glow_image_id,
            a.light_image_id,
            a.weap_image_id,
            a.shield_image_id,
        ];
        (ids, a.sheet_layout())
    });
    let spins = data
        .records::<Spin>()
        .filter_map(|(_, result)| Some(result.ok()?.record))
        .map(|s| (vec![s.sprites_id], s.sheet_layout()));

    let mut layouts = BTreeMap::new();
    for (ids, layout) in base.chain(layers).chain(spins) {
        let Some(layout) = layout else { continue };
        for id in ids {
            if data.resource(RLED, id).is_some() {
                layouts.entry(id).or_insert(layout);
            }
        }
    }
    layouts
}

#[cfg(test)]
mod tests {
    use nova_data::Record;
    use nova_rsrc::ResType;

    use super::*;
    use crate::testutil::{record, store};

    type Res = (ResType, i16, Option<&'static str>, Vec<u8>);

    /// A `shän` whose base image is `base`, with `frames_per` columns.
    fn shan(id: i16, base: i16, frames_per: i16) -> Res {
        shan_layers(id, &[(0x00, base)], frames_per)
    }

    /// A `shän` with image IDs at the given offsets.
    fn shan_layers(id: i16, layers: &[(usize, i16)], frames_per: i16) -> Res {
        let mut fields = layers.to_vec();
        fields.push((0x34, frames_per));
        (ShipAnim::TYPE, id, None, record(192, &fields))
    }

    fn spin(id: i16, sprites: i16, x_tiles: i16) -> Res {
        (
            Spin::TYPE,
            id,
            None,
            record(12, &[(0, sprites), (8, x_tiles)]),
        )
    }

    fn sheet(id: i16) -> Res {
        (RLED, id, None, b"sheet".to_vec())
    }

    fn columns(resources: &[Res]) -> Vec<(i16, u16)> {
        sheet_layouts(&store(resources))
            .into_iter()
            .map(|(id, layout)| (id, layout.columns()))
            .collect()
    }

    #[test]
    fn a_shan_beats_a_spin_on_the_same_sheet() {
        let resources = [spin(200, 1000, 6), shan(128, 1000, 36), sheet(1000)];
        assert_eq!(columns(&resources), [(1000, 36)]);
    }

    #[test]
    fn a_spin_lays_out_a_sheet_no_shan_uses() {
        let resources = [spin(200, 1000, 6), sheet(1000)];
        assert_eq!(columns(&resources), [(1000, 6)]);
        assert_eq!(
            sheet_layouts(&store(&resources)).get(&1000),
            SheetLayout::new(6).as_ref()
        );
    }

    #[test]
    fn every_shan_layer_claims_its_sheet() {
        let layers = [
            (0x00, 1),
            (0x0C, 2),
            (0x16, 3),
            (0x1E, 4),
            (0x26, 5),
            (0x40, 6),
        ];
        let mut resources: Vec<Res> = (1..=6).map(sheet).collect();
        resources.push(shan_layers(128, &layers, 9));
        assert_eq!(
            columns(&resources),
            [(1, 9), (2, 9), (3, 9), (4, 9), (5, 9), (6, 9)]
        );
    }

    #[test]
    fn a_base_image_beats_another_shans_other_layer() {
        let resources = [
            shan_layers(128, &[(0x00, 7), (0x0C, 1000)], 10),
            shan(129, 1000, 20),
            sheet(1000),
        ];
        assert_eq!(columns(&resources), [(1000, 20)]);
    }

    #[test]
    fn the_lowest_id_wins_among_equals() {
        let resources = [
            shan(129, 1000, 20),
            shan(128, 1000, 10),
            spin(301, 1001, 3),
            spin(300, 1001, 2),
            sheet(1000),
            sheet(1001),
        ];
        assert_eq!(columns(&resources), [(1000, 10), (1001, 2)]);
    }

    #[test]
    fn a_pointer_to_a_missing_sheet_is_ignored() {
        let resources = [shan(128, 5000, 10), spin(200, 5001, 4), sheet(1000)];
        assert_eq!(columns(&resources), []);
    }

    #[test]
    fn a_record_without_a_layout_does_not_claim_its_sheet() {
        let resources = [
            shan(128, 1000, 0),
            shan(129, 1001, -3),
            spin(200, 1000, 4),
            spin(201, 1001, 0),
            spin(202, 1001, 5),
            sheet(1000),
            sheet(1001),
        ];
        assert_eq!(columns(&resources), [(1000, 4), (1001, 5)]);
    }

    #[test]
    fn an_undecodable_record_is_skipped() {
        let short_shan = (ShipAnim::TYPE, 128, None, record(10, &[(0, 1000)]));
        let short_spin = (Spin::TYPE, 200, None, record(10, &[(0, 1001)]));
        let resources = [
            short_shan,
            short_spin,
            spin(201, 1000, 3),
            sheet(1000),
            sheet(1001),
        ];
        assert_eq!(columns(&resources), [(1000, 3)]);
    }
}
