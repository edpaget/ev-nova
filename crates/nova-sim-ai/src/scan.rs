//! An interceptor's or a warship's scan of the player (`_ScanPlayer`
//! @0x7e46e in the `EV Nova` executable): whether it goes on
//! ([`goes_on`]), and the contraband it finds after the mission cargo
//! ([`contraband`]). The mission cargo's own part is
//! `nova_sim::mission::cargo::scan`; the session
//! runs the three in order (its `mission_cargo` module).
//!
//! # The gates
//!
//! A ship scans the player once it has looked the player over (see
//! [`ai::interceptor`](crate::ai::interceptor)), and only when, in order:
//! its AI type is 3 or 4, a warship or an interceptor (+0x88,
//! @0x7e483-0x7e491); it has a government (@0x7e497); that government's
//! `SmugPenalty` is not 0 (gövt +0x48, @0x7e4a8-0x7e4bf); it is within
//! 100.0 of the player on both axes (@0x7e4c5-0x7e52d,
//! [`within_reach`]); and `Rand(100)` ≤ 75 (@0x7e54c-0x7e55c). The
//! visibility test between the reach and the draw (`_ShipVisibleToShip`
//! @0x7e53f) always passes: the player can cloak, but this site reads it
//! as seen until
//! rdm:phase/cloaking-fidelity/phase-9-npc-sight-of-cloaked-player. The reach
//! is tested again before each contraband loop (@0x7eacb-0x7eb3f,
//! @0x7ef0d-0x7ef7b); nothing moves in between, so once is enough.
//!
//! The jump-timing check after the draw (@0x7e562-0x7e633) holds every
//! scan back once the player's warp is past its lead-in: while
//! t·w/(d·0.01) − 35/w ≤ 0 (t the ticks since the warp stage began, w
//! the ship class's warp factor, d the warp sound's duration) the scan
//! runs, and so it does through the pre-jump braking, and always for a
//! disabled player (see the [`warp`](crate::warp) module, which works
//! it). The session takes the scans in `Session::tick_combat` through
//! the lead-in, the traffic flying on to queue them, and holds them in
//! `Session::scanned_by` once it is past.
//!
//! # Contraband
//!
//! After the mission cargo, found or not, two branches scan the player's
//! `jünk` and outfits, each behind a latch that
//! `_ResetPlayerPrecalcedValues` sets ([`Latches`]):
//!
//! - **Jünk** (@0x7eabd-0x7eefa), while the jünk latch is set: the first
//!   `jünk` by ID that the player holds and whose `ScanMask` shares a bit
//!   with the government's (@0x7eb7d-0x7eba3) is found, and fined
//!   [`scan_fine`] of its government's `ScanFine`, as the mission branch
//!   fines it (@0x7ed60-0x7ee65). With the latch set and nothing found the
//!   scan ends, outfits unscanned (@0x7eefa).
//! - **Outfits** (@0x7eeff-0x7f3d7), only when the jünk latch is clear
//!   and the outfit latch set: the first outfit by ID that the player owns
//!   and whose `ScanMask` shares a bit with the government's
//!   (@0x7efd1-0x7eff6) is found, and fined its `ScanFine` only when that
//!   is above 0 (`jle` @0x7f222): a negative one, which the other
//!   branches take as a percent of the cash, fines nothing.
//!
//! A find convicts the player of [`Crime::Smuggle`](crate::legal::Crime),
//! the original's type 0 (`_SlapWithPenalty` @0x7ebcc, @0x7f01f, slot
//! -1), sets the scanner attacking the player, sounds `_beepSnd[4]`, tells
//! the player for 400 ticks and clears its own latch (@0x7eed3,
//! @0x7f394-0x7f399).
//!
//! Two readings are the [`Rulebook`](crate::Rulebook)'s, the engine's by
//! default ([`ScanRules`]): whether a set jünk latch with nothing found
//! shadows the outfits
//! ([`ScanOutfitsPastJunk`](crate::RuleKey::ScanOutfitsPastJunk)), and
//! whether a negative `ScanFine` fines an outfit
//! ([`OutfitScanFine`](crate::RuleKey::OutfitScanFine)).

use std::collections::BTreeMap;

use crate::ai::interceptor::within_reach;
use crate::catalog::{GovtRecord, JunkId, JunkRecord, OutfitId, OutfitRecord};
use crate::chance::Chance;
use crate::geometry::Vec2;
use crate::market::Good;
use crate::rulebook::RuleSource;
use crate::traffic::npc::AiType;

/// The highest draw of `Rand(100)` with which a scan goes on
/// (`_ScanPlayer` @0x7e54c-0x7e55c).
pub const SCAN_ROLL: u32 = 75;
/// What a `ScanFine` below none takes of the cash, times its negation
/// (0.0001 @0xdda58), before it is counted in hundreds.
const SCAN_FINE_SHARE: f64 = 0.0001;

/// Whether a ship of AI type `ai_type` and government `govt`, `off` from
/// the player, scans the player, drawing on `chance` (see the module
/// docs' gates): only a warship or an interceptor, of a government with
/// a `SmugPenalty`, within reach, then on a draw of `Rand(100)` of 75 or
/// less. Nothing is drawn when an earlier gate fails.
pub fn goes_on(ai_type: AiType, govt: &GovtRecord, off: Vec2, chance: &mut dyn Chance) -> bool {
    matches!(ai_type, AiType::Warship | AiType::Interceptor)
        && govt.penalties.smuggle != 0
        && within_reach(off)
        && chance.below(100) <= SCAN_ROLL
}

/// The credits a `ScanFine` of `fine` takes of `cash` (`_ScanPlayer`
/// @0x7e982-0x7e9c1, @0x7edba-0x7edf9): above none as it is; below none
/// max(1, trunc(−`fine` × 0.0001 × cash) × 100), worked in doubles.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn scan_fine(fine: i16, cash: i64) -> i64 {
    if fine >= 0 {
        return i64::from(fine);
    }
    let share = (-f64::from(fine) * SCAN_FINE_SHARE * cash as f64).trunc() as i64;
    (share * 100).max(1)
}

/// `_ScanPlayer`'s two latches, as the session keeps them: each is armed
/// wherever `_ResetPlayerPrecalcedValues` runs, and cleared by its own
/// branch's conviction. The original's latch is set at the reset only
/// when the player carries some `jünk` (owns some outfit) with a
/// `ScanMask` (@0xc524-0xc53c, @0xc572-0xc5a3); [`contraband`] works that
/// out live, which is the same, since every cargo or outfit change in
/// flight runs a reset. Neither is saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Latches {
    /// The jünk latch (0x3b5108).
    pub junk: bool,
    /// The outfit latch (0x3b50c8).
    pub outfits: bool,
}

impl Latches {
    /// Both armed, as a reset leaves them.
    pub const ARMED: Self = Self {
        junk: true,
        outfits: true,
    };

    /// Clears the latch of the branch that found `found`.
    pub fn clear(&mut self, found: &Contraband) {
        match found {
            Contraband::Junk { .. } => self.junk = false,
            Contraband::Outfit { .. } => self.outfits = false,
        }
    }
}

/// The disputed readings of the contraband scan (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScanRules {
    /// [`ScanOutfitsPastJunk`](crate::RuleKey::ScanOutfitsPastJunk): by
    /// the engine, a set jünk latch ends the scan when no `jünk` is found;
    /// by the other reading, the outfits are scanned then.
    pub outfits_past_junk: RuleSource,
    /// [`OutfitScanFine`](crate::RuleKey::OutfitScanFine): by the engine,
    /// only a `ScanFine` above 0 fines an outfit; by the other reading, a
    /// negative one takes its percent of the cash, as for `jünk`.
    pub outfit_fine: RuleSource,
}

/// What the player carries that a scan can find.
#[derive(Clone, Copy, Debug)]
pub struct Carried<'a> {
    /// The player's cargo: its `jünk` is scanned.
    pub cargo: &'a BTreeMap<Good, u32>,
    /// The `jünk` records.
    pub junk: &'a [JunkRecord],
    /// The outfits the player owns, and how many of each.
    pub outfits: &'a BTreeMap<OutfitId, u16>,
    /// The outfit records.
    pub outfit_records: &'a [OutfitRecord],
}

impl Carried<'_> {
    /// The `jünk` held, in ID order, with its record and how many tons.
    fn junk_held(&self) -> impl Iterator<Item = (&JunkRecord, u32)> {
        self.cargo.iter().filter_map(|(&good, &tons)| match good {
            Good::Junk(id) if tons > 0 => self
                .junk
                .iter()
                .find(|record| record.id == id)
                .map(|record| (record, tons)),
            _ => None,
        })
    }

    /// The outfits owned, in ID order, with their records and counts.
    fn outfits_owned(&self) -> impl Iterator<Item = (&OutfitRecord, u16)> {
        self.outfits.iter().filter_map(|(&id, &count)| {
            let record = self.outfit_records.iter().find(|record| record.id == id);
            record.filter(|_| count > 0).map(|record| (record, count))
        })
    }
}

/// Contraband a scan found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contraband {
    /// A `jünk`, and the credits fined.
    Junk {
        /// The `jünk`.
        id: JunkId,
        /// The credits fined: none for no fine.
        fine: i64,
    },
    /// An outfit, whether exactly one is owned, and the credits fined.
    Outfit {
        /// The outfit.
        id: OutfitId,
        /// Whether the player owns exactly one: the message names it in
        /// the singular, with an article.
        one: bool,
        /// The credits fined: none for no fine.
        fine: i64,
    },
}

/// The contraband a ship of government `govt` finds scanning what the
/// player has `carried`, with `cash` left after any mission fine, behind
/// the `armed` latches, as `rules` read (see the module docs'
/// "Contraband"): none when nothing is found or scanned.
#[must_use]
pub fn contraband(
    govt: &GovtRecord,
    carried: &Carried,
    cash: i64,
    armed: Latches,
    rules: ScanRules,
) -> Option<Contraband> {
    let found = |mask: u16| mask & govt.scan_mask != 0;
    let junk_latch = armed.junk && carried.junk_held().any(|(record, _)| record.scan_mask != 0);
    if junk_latch {
        if let Some((record, _)) = carried
            .junk_held()
            .find(|(record, _)| found(record.scan_mask))
        {
            return Some(Contraband::Junk {
                id: record.id,
                fine: scan_fine(govt.scan_fine, cash),
            });
        }
        if rules.outfits_past_junk == RuleSource::Engine {
            return None;
        }
    }
    let outfit_latch = armed.outfits
        && carried
            .outfits_owned()
            .any(|(record, _)| record.scan_mask != 0);
    if !outfit_latch {
        return None;
    }
    let (record, count) = carried
        .outfits_owned()
        .find(|(record, _)| found(record.scan_mask))?;
    let fine = match rules.outfit_fine {
        RuleSource::Engine => i64::from(govt.scan_fine.max(0)),
        RuleSource::Bible => scan_fine(govt.scan_fine, cash),
    };
    Some(Contraband::Outfit {
        id: record.id,
        one: count == 1,
        fine,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Penalties;
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::{govt, outfit};

    /// A government with a `SmugPenalty` of 5, scanning for bit 0x0004,
    /// fining `fine`.
    fn scanning(fine: i16) -> GovtRecord {
        GovtRecord {
            penalties: Penalties {
                smuggle: 5,
                ..Penalties::default()
            },
            scan_mask: 0x0004,
            scan_fine: fine,
            news_pic: -1,
            ..govt(128)
        }
    }

    /// Whether a ship of `ai_type`, `off` from the player, scans for
    /// `govt` on a draw of `draw`, and the draws asked.
    fn gate(ai_type: AiType, govt: &GovtRecord, off: Vec2, draw: u32) -> (bool, Vec<u32>) {
        let mut chance = Draws::of(&[draw]);
        let goes = goes_on(ai_type, govt, off, &mut chance);
        (goes, chance.asked)
    }

    #[test]
    fn a_warship_or_an_interceptor_within_reach_scans_on_a_draw_of_75_or_less() {
        for ai_type in [AiType::Warship, AiType::Interceptor] {
            assert_eq!(
                gate(ai_type, &scanning(0), Vec2::new(100.0, -100.0), 75),
                (true, vec![100]),
                "{ai_type:?}"
            );
            assert_eq!(
                gate(ai_type, &scanning(0), Vec2::ZERO, 76),
                (false, vec![100])
            );
        }
    }

    #[test]
    fn a_trader_a_lax_government_or_a_ship_out_of_reach_scans_nothing_and_draws_nothing() {
        for ai_type in [AiType::WimpyTrader, AiType::BraveTrader] {
            assert_eq!(
                gate(ai_type, &scanning(0), Vec2::ZERO, 0),
                (false, vec![]),
                "{ai_type:?}"
            );
        }
        let lax = GovtRecord {
            penalties: Penalties::default(),
            ..scanning(0)
        };
        assert_eq!(
            gate(AiType::Interceptor, &lax, Vec2::ZERO, 0),
            (false, vec![])
        );
        for off in [Vec2::new(100.5, 0.0), Vec2::new(0.0, -100.5)] {
            assert_eq!(
                gate(AiType::Interceptor, &scanning(0), off, 0),
                (false, vec![]),
                "{off:?}"
            );
        }
    }

    #[test]
    fn a_scan_fine_above_none_is_as_it_is_and_below_none_a_percent_in_hundreds_at_least_1() {
        assert_eq!(scan_fine(0, 50_000), 0);
        assert_eq!(scan_fine(500, 50_000), 500);
        assert_eq!(scan_fine(-10, 50_000), 5000, "10% of 50,000");
        assert_eq!(scan_fine(-10, 12_345), 1200, "trunc(12.345) hundreds");
        assert_eq!(scan_fine(-10, 999), 1, "no hundred: 1");
        assert_eq!(scan_fine(-10, 0), 1);
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

    /// Outfit `id` masked `mask`.
    fn masked(id: i16, mask: u16) -> OutfitRecord {
        OutfitRecord {
            scan_mask: mask,
            ..outfit(id, &[])
        }
    }

    /// The player's cargo and outfits, the records of `jünk` 128 (0x0800),
    /// 129 (0x0004), 130 (0x0006) and 131 (none), and of outfits 128
    /// (0x0800), 129 (0x0004), 130 (0x000c) and 131 (none).
    struct Hold {
        cargo: BTreeMap<Good, u32>,
        outfits: BTreeMap<OutfitId, u16>,
        junk_records: Vec<JunkRecord>,
        outfit_records: Vec<OutfitRecord>,
    }

    impl Hold {
        fn of(junk_held: &[(i16, u32)], outfits: &[(i16, u16)]) -> Self {
            let mut cargo: BTreeMap<Good, u32> = junk_held
                .iter()
                .map(|&(id, tons)| (Good::Junk(JunkId(id)), tons))
                .collect();
            cargo.insert(Good::Commodity(0), 5);
            Self {
                cargo,
                outfits: outfits
                    .iter()
                    .map(|&(id, count)| (OutfitId(id), count))
                    .collect(),
                junk_records: vec![
                    junk(128, 0x0800),
                    junk(129, 0x0004),
                    junk(130, 0x0006),
                    junk(131, 0),
                ],
                outfit_records: vec![
                    masked(128, 0x0800),
                    masked(129, 0x0004),
                    masked(130, 0x000c),
                    masked(131, 0),
                ],
            }
        }

        fn carried(&self) -> Carried<'_> {
            Carried {
                cargo: &self.cargo,
                junk: &self.junk_records,
                outfits: &self.outfits,
                outfit_records: &self.outfit_records,
            }
        }

        /// What [`scanning`] `fine` finds with `cash`, both latches armed,
        /// by `rules`.
        fn scanned(&self, fine: i16, cash: i64, rules: ScanRules) -> Option<Contraband> {
            contraband(
                &scanning(fine),
                &self.carried(),
                cash,
                Latches::ARMED,
                rules,
            )
        }
    }

    const ENGINE: ScanRules = ScanRules {
        outfits_past_junk: RuleSource::Engine,
        outfit_fine: RuleSource::Engine,
    };

    #[test]
    fn the_first_held_junk_by_id_sharing_a_mask_bit_is_found() {
        let hold = Hold::of(&[(128, 3), (130, 1), (129, 2)], &[(129, 1)]);
        assert_eq!(
            hold.scanned(500, 1000, ENGINE),
            Some(Contraband::Junk {
                id: JunkId(129),
                fine: 500
            }),
            "128's mask shares no bit; 129 before 130"
        );
    }

    #[test]
    fn junk_not_held_or_sharing_no_bit_is_not_found() {
        let hold = Hold::of(&[(128, 3), (129, 0), (131, 2)], &[]);
        assert_eq!(hold.scanned(500, 1000, ENGINE), None);
    }

    #[test]
    fn the_junk_fine_is_the_scan_fine_or_its_percent() {
        let hold = Hold::of(&[(129, 1)], &[]);
        for (fine, credits) in [(0, 0), (500, 500), (-10, 5000)] {
            assert_eq!(
                hold.scanned(fine, 50_000, ENGINE),
                Some(Contraband::Junk {
                    id: JunkId(129),
                    fine: credits
                }),
                "{fine}"
            );
        }
        assert_eq!(
            hold.scanned(-10, 50, ENGINE),
            Some(Contraband::Junk {
                id: JunkId(129),
                fine: 1
            }),
            "floored at 1"
        );
    }

    #[test]
    fn the_first_owned_outfit_by_id_sharing_a_mask_bit_is_found_one_or_more() {
        let hold = Hold::of(&[], &[(128, 1), (129, 0), (130, 2), (131, 1)]);
        assert_eq!(
            hold.scanned(500, 1000, ENGINE),
            Some(Contraband::Outfit {
                id: OutfitId(130),
                one: false,
                fine: 500
            }),
            "128 shares no bit; 129 is not owned"
        );
        let hold = Hold::of(&[], &[(129, 1), (130, 2)]);
        assert_eq!(
            hold.scanned(500, 1000, ENGINE),
            Some(Contraband::Outfit {
                id: OutfitId(129),
                one: true,
                fine: 500
            })
        );
    }

    #[test]
    fn by_the_engine_only_a_scan_fine_above_none_fines_an_outfit() {
        let hold = Hold::of(&[], &[(129, 1)]);
        let found = |fine, rules| hold.scanned(fine, 50_000, rules);
        let outfit = |fine| {
            Some(Contraband::Outfit {
                id: OutfitId(129),
                one: true,
                fine,
            })
        };
        assert_eq!(found(500, ENGINE), outfit(500));
        assert_eq!(found(0, ENGINE), outfit(0));
        assert_eq!(found(-10, ENGINE), outfit(0), "jle @0x7f222");
        let bible = ScanRules {
            outfit_fine: RuleSource::Bible,
            ..ENGINE
        };
        assert_eq!(found(-10, bible), outfit(5000), "the percent");
        assert_eq!(found(500, bible), outfit(500));
    }

    #[test]
    fn by_the_engine_scannable_junk_shadows_the_outfits_even_when_none_is_found() {
        let hold = Hold::of(&[(128, 1)], &[(129, 1)]);
        assert_eq!(
            hold.scanned(500, 1000, ENGINE),
            None,
            "water's 0x0800 sets the latch (@0xc524-0xc53c)"
        );
        let past = ScanRules {
            outfits_past_junk: RuleSource::Bible,
            ..ENGINE
        };
        assert_eq!(
            hold.scanned(500, 1000, past),
            Some(Contraband::Outfit {
                id: OutfitId(129),
                one: true,
                fine: 500
            })
        );
        let unmasked = Hold::of(&[(131, 1)], &[(129, 1)]);
        assert!(
            matches!(
                unmasked.scanned(500, 1000, ENGINE),
                Some(Contraband::Outfit { .. })
            ),
            "a jünk without a ScanMask sets no latch"
        );
    }

    #[test]
    fn a_cleared_latch_skips_its_branch_and_neither_armed_finds_nothing() {
        let hold = Hold::of(&[(129, 1)], &[(129, 1)]);
        let scan = |armed| contraband(&scanning(500), &hold.carried(), 1000, armed, ENGINE);
        assert_eq!(
            scan(Latches {
                junk: false,
                outfits: true
            }),
            Some(Contraband::Outfit {
                id: OutfitId(129),
                one: true,
                fine: 500
            }),
            "after a jünk conviction the outfits are scanned"
        );
        assert_eq!(
            scan(Latches {
                junk: true,
                outfits: false
            }),
            Some(Contraband::Junk {
                id: JunkId(129),
                fine: 500
            })
        );
        let outfits_only = Hold::of(&[], &[(129, 1)]);
        assert_eq!(
            contraband(
                &scanning(500),
                &outfits_only.carried(),
                1000,
                Latches {
                    junk: true,
                    outfits: false
                },
                ENGINE
            ),
            None
        );
        assert_eq!(
            scan(Latches {
                junk: false,
                outfits: false
            }),
            None
        );
    }

    #[test]
    fn a_latch_clears_for_its_own_branch() {
        let mut latches = Latches::ARMED;
        latches.clear(&Contraband::Junk {
            id: JunkId(129),
            fine: 0,
        });
        assert_eq!(
            latches,
            Latches {
                junk: false,
                outfits: true
            }
        );
        latches = Latches::ARMED;
        latches.clear(&Contraband::Outfit {
            id: OutfitId(129),
            one: true,
            fine: 0,
        });
        assert_eq!(
            latches,
            Latches {
                junk: true,
                outfits: false
            }
        );
    }

    #[test]
    fn a_junk_or_outfit_without_a_record_is_skipped() {
        let mut hold = Hold::of(&[(140, 1), (129, 1)], &[]);
        assert_eq!(
            hold.scanned(500, 1000, ENGINE),
            Some(Contraband::Junk {
                id: JunkId(129),
                fine: 500
            })
        );
        hold = Hold::of(&[], &[(140, 1), (129, 1)]);
        assert!(matches!(
            hold.scanned(500, 1000, ENGINE),
            Some(Contraband::Outfit {
                id: OutfitId(129),
                ..
            })
        ));
    }
}
