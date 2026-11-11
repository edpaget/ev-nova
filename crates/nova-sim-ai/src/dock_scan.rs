//! The new game's law over mod kits: the attention a ship draws, the odds
//! that traffic control scans a ship that asks to land, and the cargo its
//! kits hide from any scan.
//!
//! **Attention and the odds.** A mod kit draws the law's attention (its
//! `attention`, kept on the store's fittings,
//! `nova_sim_catalog::fittings`). A ship's attention is the sum over the
//! kits it carries ([`attention`]), default items included, as its stats
//! count them. When the player asks a law-keeping stellar's traffic
//! control for clearance, traffic control scans the ship before it
//! answers with odds of its attention times its government's rate, a
//! percent a point ([`SCAN_RATE`] when the government names none, its
//! own from the fittings' scan rates when it does), capped at 100
//! ([`scan_odds`]). The draw is one below 100, under the odds to scan
//! ([`scans`]). A ship drawing no attention, or a government whose rate
//! is 0, is never scanned and draws nothing; nor does a government that
//! keeps no law. The scan is the same contraband scan an interceptor
//! makes ([`scan::contraband`](crate::scan::contraband)), a find
//! convicting, fining and sounding as an interceptor's does. The session
//! runs it at the dock request (its `dock_scan` module), drawing on the
//! chance `FlightCommand::Land`'s collaborators hold.
//!
//! **Hidden cargo.** A kit's `hidden_cargo` hides that many tons from a
//! scan. The tons pool over the kits a ship carries ([`hidden_cargo`]), as
//! attention does, and each good is compared alone with the whole pool: a
//! scan sees a good only when the ship holds more tons of it than its
//! kits hide ([`seen_cargo`]). Both the interceptor's scan and the dock
//! scan read the seen cargo, so a good the kits hide is never found and
//! sets no `jünk` latch either: the scan never saw it, so it doesn't stop
//! the outfit branch under the engine's reading of
//! `ScanOutfitsPastJunk`. Mission cargo isn't hidden: it is no good in
//! the hold, and its scan is a part of its own.
//!
//! The original has neither: its traffic control reads only the legal
//! record (`_HandlePlayerDockRequest` @0x67903-0x67a05), its scans are
//! its interceptors' and warships' alone (`_ScanPlayer` @0x7e46e), and
//! nothing hides cargo. Both are the new game's, and stock data never
//! reaches them: no stock outfit has fittings, so its attention is 0,
//! which is never scanned and draws nothing, and it hides no tons, so a
//! scan sees every good it holds.

use std::collections::BTreeMap;

use crate::catalog::{GovtRecord, OutfitId};
use crate::chance::Chance;
use crate::market::Good;
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

/// The percent chance, for each point of a ship's attention, that
/// traffic control scans it, when its government names no rate of its
/// own.
pub const SCAN_RATE: u32 = 25;

/// The percent odds that traffic control scans a ship of `attention`, at
/// `rate` percent a point ([`SCAN_RATE`] when its government names none),
/// capped at 100.
#[must_use]
pub fn scan_odds(attention: u64, rate: Option<u32>) -> u32 {
    let odds = attention.saturating_mul(u64::from(rate.unwrap_or(SCAN_RATE)));
    u32::try_from(odds.min(100)).expect("at most 100")
}

/// Whether traffic control of government `govt` scans a ship of
/// `attention` before it answers, at the government's own `rate` (see the
/// module docs): never for a government that keeps no law, its
/// `SmugPenalty` 0 (the gate an interceptor's scan has, `scan::goes_on`),
/// nor at odds of 0, each without a draw; else when a draw below 100
/// falls under the odds ([`scan_odds`]).
#[must_use]
pub fn scans(
    govt: &GovtRecord,
    attention: u64,
    rate: Option<u32>,
    chance: &mut dyn Chance,
) -> bool {
    if govt.penalties.smuggle == 0 {
        return false;
    }
    let odds = scan_odds(attention, rate);
    odds > 0 && chance.below(100) < odds
}

/// The tons of cargo a ship carrying `carried` hides from a scan, by
/// `kits`' hidden cargo: each kit's times how many are carried, summed.
#[must_use]
pub fn hidden_cargo(carried: &BTreeMap<OutfitId, u16>, kits: &BTreeMap<OutfitId, Kit>) -> u64 {
    carried
        .iter()
        .filter_map(|(outfit, &count)| {
            let kit = kits.get(outfit)?;
            Some(u64::from(kit.hidden_cargo) * u64::from(count))
        })
        .sum()
}

/// The goods of `cargo` a scan sees past `hidden` tons hidden: each good
/// the ship holds more tons of than its kits hide, each compared alone.
#[must_use]
pub fn seen_cargo(cargo: &BTreeMap<Good, u32>, hidden: u64) -> BTreeMap<Good, u32> {
    cargo
        .iter()
        .filter(|&(_, &tons)| u64::from(tons) > hidden)
        .map(|(&good, &tons)| (good, tons))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{JunkId, JunkRecord, OutfitRecord, Penalties};
    use crate::rulebook::RuleSource;
    use crate::scan::{self, Carried, Contraband, Latches, ScanRules};
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::{govt, outfit};

    fn kits() -> BTreeMap<OutfitId, Kit> {
        BTreeMap::from([
            (
                OutfitId(130),
                Kit {
                    attention: 1,
                    ..Kit::default()
                },
            ),
            (
                OutfitId(131),
                Kit {
                    attention: 2,
                    ..Kit::default()
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

    /// A government that keeps the law, scanning for bit 0x0004 and
    /// fining 500.
    fn law() -> GovtRecord {
        GovtRecord {
            penalties: Penalties {
                smuggle: 10,
                ..Penalties::default()
            },
            scan_mask: 0x0004,
            scan_fine: 500,
            news_pic: -1,
            ..govt(128)
        }
    }

    /// Whether `govt` scans a ship of `attention` at `rate` on a draw of
    /// `roll`, and the draws asked.
    fn rolled(govt: &GovtRecord, attention: u64, rate: Option<u32>, roll: u32) -> (bool, Vec<u32>) {
        let mut chance = Draws::of(&[roll]);
        let scanned = scans(govt, attention, rate, &mut chance);
        (scanned, chance.asked)
    }

    #[test]
    fn a_roll_that_passes_attention_1_scans_attention_2_and_attention_0_is_never_scanned() {
        assert_eq!(rolled(&law(), 1, None, 30), (false, vec![100]));
        assert_eq!(rolled(&law(), 2, None, 30), (true, vec![100]));
        assert_eq!(
            rolled(&law(), 0, None, 0),
            (false, Vec::new()),
            "no attention draws nothing"
        );
    }

    #[test]
    fn the_odds_are_the_rate_a_point_capped_at_100() {
        assert_eq!(SCAN_RATE, 25);
        assert_eq!(scan_odds(0, None), 0);
        assert_eq!(scan_odds(1, None), SCAN_RATE);
        assert_eq!(scan_odds(3, None), 3 * SCAN_RATE);
        assert_eq!(scan_odds(5, None), 100);
        assert_eq!(scan_odds(2, Some(10)), 20);
        assert_eq!(scan_odds(3, Some(40)), 100);
        assert_eq!(scan_odds(u64::MAX, None), 100, "saturates");
        assert_eq!(scan_odds(u64::MAX, Some(u32::MAX)), 100);
        assert_eq!(scan_odds(u64::MAX, Some(0)), 0);
    }

    #[test]
    fn odds_of_100_scan_on_every_roll() {
        assert_eq!(rolled(&law(), 5, None, 99), (true, vec![100]));
        assert_eq!(rolled(&law(), 1, Some(100), 99), (true, vec![100]));
    }

    #[test]
    fn a_government_s_own_rate_sets_the_odds() {
        assert_eq!(rolled(&law(), 2, Some(10), 19), (true, vec![100]));
        assert_eq!(rolled(&law(), 2, Some(10), 20), (false, vec![100]));
        assert_eq!(
            rolled(&law(), 5, Some(0), 0),
            (false, Vec::new()),
            "a rate of 0 never looks and draws nothing"
        );
    }

    #[test]
    fn a_government_with_no_smuggling_penalty_scans_nobody_and_draws_nothing() {
        let lawless = GovtRecord {
            penalties: Penalties::default(),
            ..law()
        };
        assert_eq!(rolled(&lawless, 5, None, 0), (false, Vec::new()));
        assert_eq!(rolled(&lawless, 5, Some(100), 0), (false, Vec::new()));
    }

    fn hold_kits() -> BTreeMap<OutfitId, Kit> {
        BTreeMap::from([
            (
                OutfitId(140),
                Kit {
                    hidden_cargo: 3,
                    ..Kit::default()
                },
            ),
            (
                OutfitId(141),
                Kit {
                    attention: 1,
                    hidden_cargo: 4,
                    ..Kit::default()
                },
            ),
        ])
    }

    #[test]
    fn hidden_tons_pool_across_the_kits_carried() {
        let kits = hold_kits();
        assert_eq!(hidden_cargo(&BTreeMap::new(), &kits), 0);
        let loose = BTreeMap::from([(OutfitId(128), 3)]);
        assert_eq!(hidden_cargo(&loose, &kits), 0, "no kit");
        let both = BTreeMap::from([(OutfitId(140), 1), (OutfitId(141), 1)]);
        assert_eq!(hidden_cargo(&both, &kits), 7);
        let two = BTreeMap::from([(OutfitId(140), 2)]);
        assert_eq!(hidden_cargo(&two, &kits), 6);
        let many = BTreeMap::from([(OutfitId(141), u16::MAX)]);
        assert_eq!(hidden_cargo(&many, &kits), 4 * u64::from(u16::MAX));
    }

    const WINE: Good = Good::Junk(JunkId(128));
    const OPALS: Good = Good::Junk(JunkId(129));
    const GRAIN: Good = Good::Commodity(0);

    #[test]
    fn a_scan_sees_only_a_good_held_past_the_hidden_tons() {
        let cargo = BTreeMap::from([(WINE, 3), (OPALS, 4), (GRAIN, 0)]);
        assert_eq!(
            seen_cargo(&cargo, 3),
            BTreeMap::from([(OPALS, 4)]),
            "3 tons hidden by 3; one more is seen"
        );
        assert_eq!(
            seen_cargo(&BTreeMap::from([(WINE, 4), (OPALS, 4)]), 5),
            BTreeMap::new(),
            "each good is compared alone: neither passes 5"
        );
        assert_eq!(
            seen_cargo(&cargo, 0),
            BTreeMap::from([(WINE, 3), (OPALS, 4)]),
            "with nothing hidden every good held is seen"
        );
    }

    /// `jünk` `id` masked `mask`.
    fn junk(id: i16, mask: u16) -> JunkRecord {
        JunkRecord {
            id: JunkId(id),
            name: format!("Junk {id}"),
            abbrev: String::new(),
            base_price: 100,
            sold_at: Vec::new(),
            bought_at: Vec::new(),
            buy_on: crate::control::Test::default(),
            sell_on: crate::control::Test::default(),
            flags: 0,
            scan_mask: mask,
            lc_name: format!("junk {id}"),
        }
    }

    /// What [`law`] finds, by the engine's readings, in a ship holding
    /// `tons` of the masked `jünk` 128 and owning `outfits`, with
    /// `hidden` tons hidden.
    fn found(tons: u32, hidden: u64, outfits: &[(i16, u16)]) -> Option<Contraband> {
        let cargo = BTreeMap::from([(WINE, tons)]);
        let seen = seen_cargo(&cargo, hidden);
        let owned: BTreeMap<OutfitId, u16> =
            outfits.iter().map(|&(id, n)| (OutfitId(id), n)).collect();
        let junk = [junk(128, 0x0004)];
        let outfit_records = [OutfitRecord {
            scan_mask: 0x0004,
            ..outfit(150, &[])
        }];
        let carried = Carried {
            cargo: &seen,
            junk: &junk,
            outfits: &owned,
            outfit_records: &outfit_records,
        };
        let rules = ScanRules {
            outfits_past_junk: RuleSource::Engine,
            outfit_fine: RuleSource::Engine,
        };
        scan::contraband(&law(), &carried, 2000, Latches::ARMED, rules)
    }

    #[test]
    fn the_contraband_scan_finds_a_good_only_past_the_hidden_tons() {
        assert_eq!(found(2, 2, &[]), None);
        assert_eq!(
            found(3, 2, &[]),
            Some(Contraband::Junk {
                id: JunkId(128),
                fine: 500
            })
        );
    }

    #[test]
    fn a_hidden_good_sets_no_latch_so_an_outfit_is_still_found() {
        assert_eq!(
            found(2, 2, &[(150, 1)]),
            Some(Contraband::Outfit {
                id: OutfitId(150),
                one: true,
                fine: 500
            }),
            "the scan never saw the jünk, so the engine's reading doesn't stop at it"
        );
        assert_eq!(
            found(3, 2, &[(150, 1)]),
            Some(Contraband::Junk {
                id: JunkId(128),
                fine: 500
            }),
            "seen, the jünk is found first"
        );
    }
}
