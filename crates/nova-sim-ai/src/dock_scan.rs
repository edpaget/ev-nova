//! The new game's attention from the law: traffic control's scan of a
//! ship that asks to land.
//!
//! A mod kit draws the law's attention (its `attention`, kept on the
//! store's fittings, `nova_sim_catalog::fittings`). A ship's attention is
//! the sum over the kits it carries ([`attention`]), default items
//! included, as its stats count them. When the player asks a law-keeping
//! stellar's traffic control for clearance with any attention at all,
//! traffic control scans the ship before it answers ([`scans`]): the same
//! contraband scan an interceptor makes ([`scan::contraband`](crate::scan::contraband)), a find
//! convicting, fining and sounding as an interceptor's does. The session
//! runs it at the dock request (its `dock_scan` module).
//!
//! The original has no such scan: its traffic control reads only the
//! legal record (`_HandlePlayerDockRequest` @0x67903-0x67a05), and its
//! scans are its interceptors' and warships' alone (`_ScanPlayer`
//! @0x7e46e). This one is the new game's, and stock data never reaches
//! it: no stock outfit has attention.
//!
//! It draws nothing. The land press takes no `Chance`, so attention
//! decides only whether traffic control looks (any at all), and what the
//! scan finds is decided by what the ship carries: a lawless kit's
//! `ScanMask` is what a law-keeping government's scan looks for. How much
//! more 2 attention is noticed than 1 waits for the bible's numbers for
//! the law.

use std::collections::BTreeMap;

use crate::catalog::{GovtRecord, OutfitId};
use nova_sim_catalog::fittings::Kit;

/// The attention a ship carrying `carried` (how many of each outfit)
/// draws, by `kits`' attention: each kit's times how many are carried,
/// summed.
#[must_use]
pub fn attention(carried: &BTreeMap<OutfitId, u16>, kits: &BTreeMap<OutfitId, Kit>) -> u64 {
    carried
        .iter()
        .filter_map(|(outfit, &count)| {
            let kit = kits.get(outfit)?;
            Some(u64::from(kit.attention) * u64::from(count))
        })
        .sum()
}

/// Whether traffic control of government `govt` scans a ship of
/// `attention` before it answers (see the module docs): only a
/// government that keeps the law, its `SmugPenalty` not 0 (the gate an
/// interceptor's scan has, `scan::goes_on`), and only a ship with any
/// attention.
#[must_use]
pub fn scans(govt: &GovtRecord, attention: u64) -> bool {
    govt.penalties.smuggle != 0 && attention > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Penalties;
    use nova_sim_catalog::fixture::govt;

    fn kits() -> BTreeMap<OutfitId, Kit> {
        BTreeMap::from([
            (
                OutfitId(130),
                Kit {
                    attention: 1,
                    spots: Vec::new(),
                },
            ),
            (
                OutfitId(131),
                Kit {
                    attention: 2,
                    spots: Vec::new(),
                },
            ),
        ])
    }

    #[test]
    fn no_kits_draw_no_attention() {
        let carried = BTreeMap::from([(OutfitId(128), 3)]);
        assert_eq!(attention(&carried, &kits()), 0);
        assert_eq!(attention(&BTreeMap::new(), &kits()), 0);
    }

    #[test]
    fn two_kits_sum_and_a_count_of_two_doubles() {
        let carried = BTreeMap::from([(OutfitId(130), 1), (OutfitId(131), 1)]);
        assert_eq!(attention(&carried, &kits()), 3);
        let carried = BTreeMap::from([(OutfitId(131), 2)]);
        assert_eq!(attention(&carried, &kits()), 4);
        let carried = BTreeMap::from([(OutfitId(131), u16::MAX)]);
        assert_eq!(attention(&carried, &kits()), 2 * u64::from(u16::MAX));
    }

    #[test]
    fn a_law_keeping_government_scans_any_attention_and_not_none() {
        let law = GovtRecord {
            penalties: Penalties {
                smuggle: 10,
                ..Penalties::default()
            },
            ..govt(128)
        };
        assert!(scans(&law, 1));
        assert!(scans(&law, 2));
        assert!(!scans(&law, 0));
    }

    #[test]
    fn a_government_with_no_smuggling_penalty_scans_nobody() {
        let lawless = GovtRecord {
            penalties: Penalties::default(),
            ..govt(128)
        };
        assert!(!scans(&lawless, 5));
    }
}
