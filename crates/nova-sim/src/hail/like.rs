//! Whether a hailed ship likes the player, and its [`Attitude`].
//!
//! The rules are the original's (`_AIDoesShipLikePlayer` @0x82177 in the
//! `EV Nova` executable). G is the ship's government, S the system's, L
//! the player's legal record in the system (its record with S; none in an
//! independent system), and tol a government's `CrimeTol`. In order:
//!
//! 1. A ship that threatens the player ([`Npc::threatens_player`]) does
//!    not like it.
//! 2. The player's escorts do. (So do the governments that bless the
//!    player, which does not happen yet.)
//! 3. An independent ship does.
//! 4. A xenophobic G in another government's system does not when L is
//!    above that government's tol. At home, and otherwise, the rules
//!    below apply. (The original compares an independent system against
//!    government index 0's tol, a slip that cannot matter, as L is none
//!    there.)
//! 5. In an independent system it does. When S is G, or allied with G,
//!    it does while L is at least -tol(S); when S is G's enemy, only
//!    while L is below -tol(S). When S is neutral to G, a nosy G
//!    (`Flags` 0x0002) does while L is at least -tol(S), and any other G
//!    does.
//!
//! The government byte +0x83 the original falls back to (an IFF or a
//! blessing) is never set here.
//!
//! The [`Attitude`] is hostile while the ship threatens the player,
//! friendly while it likes it, and unfriendly otherwise.

use crate::ai::Surroundings;
use crate::govt::NOSY;
use crate::traffic::npc::Npc;

/// How a hailed ship feels about the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attitude {
    /// It likes the player.
    Friendly,
    /// It neither likes nor threatens the player.
    Unfriendly,
    /// It threatens the player.
    Hostile,
}

/// Whether `npc` likes the player among `around` (see the module docs).
#[must_use]
pub fn likes_player(npc: &Npc, around: &Surroundings) -> bool {
    if npc.threatens_player() {
        return false;
    }
    if npc.escort.is_some() {
        return true;
    }
    let Some(govt) = npc.govt else {
        return true;
    };
    let govts = around.govts;
    let record = i32::from(around.record);
    let Some(system) = around.system_govt else {
        return true;
    };
    let tolerance = i32::from(govts.crime_tol(Some(system)));
    if govts.xenophobic(Some(govt)) && system != govt && record > tolerance {
        return false;
    }
    if govts.allies(Some(system), Some(govt)) {
        record >= -tolerance
    } else if govts.enemies(Some(system), Some(govt)) {
        record < -tolerance
    } else if govts.flag(Some(govt), NOSY) {
        record >= -tolerance
    } else {
        true
    }
}

/// How `npc` feels about the player among `around` (see the module
/// docs).
#[must_use]
pub fn attitude(npc: &Npc, around: &Surroundings) -> Attitude {
    if npc.threatens_player() {
        Attitude::Hostile
    } else if likes_player(npc, around) {
        Attitude::Friendly
    } else {
        Attitude::Unfriendly
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::catalog::{GovtId, GovtRecord};
    use crate::combat::ShipRef;
    use crate::govt::{Governments, XENOPHOBIC};
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, govt};

    const ME: GovtId = GovtId(140);
    const ALLY: GovtId = GovtId(141);
    const ENEMY: GovtId = GovtId(142);
    const NEUTRAL: GovtId = GovtId(143);
    const XENO: GovtId = GovtId(144);

    /// The ship's government 140 (class 1, with `flags`), its ally 141,
    /// its enemy 142, a neutral 143, and a xenophobe 144 of class 1; each
    /// of `CrimeTol` 6.
    fn govts(flags: u16) -> Governments {
        Governments::new([
            GovtRecord {
                flags,
                crime_tol: 6,
                classes: [1, -1, -1, -1],
                ..govt(140)
            },
            GovtRecord {
                crime_tol: 6,
                allies: [1, -1, -1, -1],
                classes: [2, -1, -1, -1],
                ..govt(141)
            },
            GovtRecord {
                crime_tol: 6,
                enemies: [1, -1, -1, -1],
                classes: [3, -1, -1, -1],
                ..govt(142)
            },
            GovtRecord {
                crime_tol: 6,
                classes: [4, -1, -1, -1],
                ..govt(143)
            },
            GovtRecord {
                flags: XENOPHOBIC,
                crime_tol: 6,
                classes: [1, -1, -1, -1],
                ..govt(144)
            },
        ])
    }

    /// A ship of `govt`, idle.
    fn ship(govt: Option<GovtId>) -> Npc {
        let mut npc = crate::testkit::npc(1, ShipStats::new(FAST, &[]));
        npc.govt = govt;
        npc
    }

    /// Whether a ship of `govt` (with `flags` for 140) likes the player
    /// of `record` in a system of `system`.
    fn likes(flags: u16, govt: Option<GovtId>, system: Option<GovtId>, record: i16) -> bool {
        let govts = govts(flags);
        let npc = ship(govt);
        let around = Surroundings {
            govts: &govts,
            system_govt: system,
            record,
            ..Surroundings::new(&[], &[])
        };
        likes_player(&npc, &around)
    }

    #[test]
    fn a_ship_threatening_the_player_never_likes_it() {
        let govts = govts(0);
        for govt in [None, Some(ME)] {
            let mut npc = ship(govt);
            npc.goal = Goal::Attack(ShipRef::Player);
            let around = Surroundings {
                govts: &govts,
                system_govt: Some(ME),
                record: 100,
                ..Surroundings::new(&[], &[])
            };
            assert!(!likes_player(&npc, &around), "{govt:?}");
            assert_eq!(attitude(&npc, &around), Attitude::Hostile);
        }
    }

    #[test]
    fn the_players_escort_likes_it_whatever_its_record() {
        let govts = govts(0);
        let mut escort = ship(Some(ALLY));
        escort.escort = Some(crate::escort::EscortDuty {
            slot: 2,
            ships: 2,
            spacing: 30.0,
            order: None,
        });
        let around = Surroundings {
            govts: &govts,
            system_govt: Some(ME),
            record: -100,
            ..Surroundings::new(&[], &[])
        };
        assert!(!likes(0, Some(ALLY), Some(ME), -100), "not as a stranger");
        assert!(likes_player(&escort, &around));
        assert_eq!(attitude(&escort, &around), Attitude::Friendly);
    }

    #[test]
    fn an_independent_ship_likes_the_player_whatever_its_record() {
        assert!(likes(0, None, Some(ME), -100));
        assert!(likes(0, None, None, -100));
    }

    #[test]
    fn in_an_independent_system_every_ship_likes_the_player() {
        for govt in [ME, ENEMY, XENO] {
            assert!(likes(0x0002, Some(govt), None, -100), "{govt:?}");
        }
    }

    #[test]
    fn at_home_or_with_an_ally_it_likes_a_record_down_to_minus_the_systems_tolerance() {
        for system in [ME, ALLY] {
            assert!(likes(0, Some(ME), Some(system), -6), "{system:?}");
            assert!(!likes(0, Some(ME), Some(system), -7), "{system:?}");
            assert!(likes(0, Some(ME), Some(system), 30), "{system:?}");
        }
    }

    #[test]
    fn in_an_enemys_system_it_likes_only_a_record_below_minus_its_tolerance() {
        assert!(likes(0, Some(ME), Some(ENEMY), -7));
        assert!(!likes(0, Some(ME), Some(ENEMY), -6));
        assert!(!likes(0, Some(ME), Some(ENEMY), 30));
    }

    #[test]
    fn in_a_neutral_system_only_a_nosy_government_minds_the_record() {
        assert!(likes(0x0002, Some(ME), Some(NEUTRAL), -6));
        assert!(!likes(0x0002, Some(ME), Some(NEUTRAL), -7));
        assert!(likes(0, Some(ME), Some(NEUTRAL), -100));
    }

    #[test]
    fn a_xenophobe_at_home_likes_a_record_down_to_minus_its_tolerance() {
        assert!(likes(0, Some(XENO), Some(XENO), -6));
        assert!(!likes(0, Some(XENO), Some(XENO), -7));
        assert!(likes(0, Some(XENO), Some(XENO), 7), "at home a good record");
    }

    #[test]
    fn a_xenophobe_elsewhere_dislikes_a_record_above_that_tolerance_then_follows_the_rules() {
        // 144 is of class 1, which 141 is allied with.
        assert!(
            !likes(0, Some(XENO), Some(ALLY), 7),
            "above 141's tolerance"
        );
        assert!(likes(0, Some(XENO), Some(ALLY), 6), "then the ally rule");
        assert!(
            !likes(0, Some(XENO), Some(ALLY), -7),
            "which minds a bad record"
        );
        assert!(
            !likes(0, Some(XENO), Some(NEUTRAL), 0),
            "a xenophobe is everyone's enemy"
        );
        assert!(likes(0, Some(XENO), Some(NEUTRAL), -7));
    }

    #[test]
    fn the_attitude_is_friendly_unfriendly_or_hostile() {
        let govts = govts(0);
        let around = |record| Surroundings {
            govts: &govts,
            system_govt: Some(ME),
            record,
            ..Surroundings::new(&[], &[])
        };
        let npc = ship(Some(ME));
        assert_eq!(attitude(&npc, &around(0)), Attitude::Friendly);
        assert_eq!(attitude(&npc, &around(-7)), Attitude::Unfriendly);
        let mut hunting = npc.clone();
        hunting.goal = Goal::Snipe(ShipRef::Player);
        assert_eq!(attitude(&hunting, &around(0)), Attitude::Hostile);
    }
}
