//! The contract of [`PilotCatalog::rle_sheet`], which the fake
//! ([`FakePilotCatalog`](super::FakePilotCatalog)) and the `GameData`
//! adapter both meet: the session's mask cache and the walk
//! (`nova_sim_base::sprite_mask`) read every sheet it answers on these
//! terms alone.

use crate::catalog::{MaskSheet, PilotCatalog};

/// The most a sheet's frames shift by: 2, a 32-bit sheet's depth >> 4.
pub const MOST_SHIFT: u8 = 2;

/// Checks `catalog` against the port: each of `held` answers a sheet with
/// a frame, a width and a height above 0, a shift of at most
/// [`MOST_SHIFT`] and every frame start inside its words; each of
/// `missing` answers none.
///
/// # Errors
///
/// The first sheet that breaks the contract, and how.
pub fn check_contract(
    catalog: &impl PilotCatalog,
    held: &[MaskSheet],
    missing: &[MaskSheet],
) -> Result<(), String> {
    for &sheet in held {
        let Some(rle) = catalog.rle_sheet(sheet) else {
            return Err(format!("{sheet:?}: no sheet"));
        };
        if !rle.starts.iter().any(Option::is_some) {
            return Err(format!("{sheet:?}: no frame"));
        }
        if rle.width == 0 || rle.height == 0 {
            return Err(format!("{sheet:?}: {} by {}", rle.width, rle.height));
        }
        if rle.shift > MOST_SHIFT {
            return Err(format!("{sheet:?}: a shift of {}", rle.shift));
        }
        if let Some(start) = rle
            .starts
            .iter()
            .flatten()
            .find(|&&at| at >= rle.words.len())
        {
            return Err(format!(
                "{sheet:?}: a frame at {start} of {} words",
                rle.words.len()
            ));
        }
    }
    if let Some(sheet) = missing
        .iter()
        .find(|&&sheet| catalog.rle_sheet(sheet).is_some())
    {
        return Err(format!("{sheet:?}: a sheet, though missing"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use nova_sim_base::fixture::rle::sheet;
    use nova_sim_base::sprite_mask::RleSheet;

    use super::*;
    use crate::catalog::ShipId;
    use crate::fixture::catalog;

    const SHIP: MaskSheet = MaskSheet::Ship(ShipId(128));
    const STELLAR: MaskSheet = MaskSheet::Stellar(3);

    /// The fake holding `rle` as graphic type 3's sheet.
    fn holding(rle: RleSheet) -> crate::fixture::FakePilotCatalog {
        let mut fake = catalog();
        fake.rle_sheets = vec![(STELLAR, rle)];
        fake
    }

    #[test]
    fn the_fake_meets_the_contract_and_records_each_sheet_asked() {
        let fake = holding(sheet(&[&["#."], &[".#"]]));
        assert_eq!(check_contract(&fake, &[STELLAR], &[SHIP]), Ok(()));
        assert_eq!(*fake.rle_sheets_asked.borrow(), [STELLAR, SHIP]);
        assert_eq!(check_contract(&catalog(), &[], &[SHIP, STELLAR]), Ok(()));
    }

    #[test]
    fn the_fake_holds_and_misses_each_kind_of_sheet_by_the_contract() {
        let kinds = [
            MaskSheet::Shot(4),
            MaskSheet::Asteroid(2),
            MaskSheet::CargoBox(1),
        ];
        let mut fake = catalog();
        fake.rle_sheets = kinds.iter().map(|&kind| (kind, sheet(&[&["#"]]))).collect();
        let missing = [
            MaskSheet::Shot(5),
            MaskSheet::Asteroid(3),
            MaskSheet::CargoBox(2),
        ];
        assert_eq!(check_contract(&fake, &kinds, &missing), Ok(()));
    }

    #[test]
    fn a_sheet_out_of_its_terms_breaks_it() {
        let good = sheet(&[&["#"]]);
        assert!(
            check_contract(&catalog(), &[STELLAR], &[]).is_err(),
            "none held"
        );
        assert!(check_contract(&holding(good.clone()), &[], &[STELLAR]).is_err());
        let broken = [
            RleSheet {
                starts: vec![None],
                ..good.clone()
            },
            RleSheet {
                width: 0,
                ..good.clone()
            },
            RleSheet {
                height: 0,
                ..good.clone()
            },
            RleSheet {
                shift: 3,
                ..good.clone()
            },
            RleSheet {
                starts: vec![Some(0), Some(good.words.len())],
                ..good.clone()
            },
        ];
        for rle in broken {
            let fake = holding(rle.clone());
            assert!(check_contract(&fake, &[STELLAR], &[]).is_err(), "{rle:?}");
        }
        let edge = RleSheet {
            shift: MOST_SHIFT,
            starts: vec![None, Some(good.words.len() - 1)],
            ..good
        };
        assert_eq!(check_contract(&holding(edge), &[STELLAR], &[]), Ok(()));
    }
}
