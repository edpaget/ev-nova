//! The player's combat rating: what a kill adds to it and which kills
//! count.
//!
//! The rules are the original's (`_AddToPlayerKills` @0x82dc and its
//! caller `_DamageShip` @0x3a807 in the `EV Nova` executable), which keep
//! the rating in the 32-bit global `_playerKills` (0x230530). Here it is
//! an `i64`, so it never wraps.
//!
//! - **What a kill adds** ([`credit`]): a rating over 9,999,999 is set to
//!   [`RATING_CAP`] and nothing is added (@0x82e7, @0x8314). Otherwise a
//!   victim of `shïp` `Strength` 4 or less adds 1 (@0x82ee-0x82f4), and a
//!   stronger one `Strength` / 5 rounded down (`Strength` × 0.2, the
//!   double at 0xdd108, truncated with the rating, @0x82f7-0x830c). The
//!   cap is checked before the add, so one kill can take the rating just
//!   past the cap and the next sets it back.
//! - **When**: on the hit that starts a ship breaking up (its armour at or
//!   below 0, and not dying before the hit: @0x3ade4-0x3adfa), not at its
//!   final destruction. Disabling, boarding and capturing credit nothing.
//! - **Which kills count** ([`counts`], @0x3ad4f-0x3ae53): the shooter is
//!   the player or a ship of its fleet, its escorts and fighters (parent
//!   the player, @0x3ad54-0x3ad7b), and the victim's government is not
//!   derelict (`_ShipIsDerelict` @0x8225). The original also skips a
//!   stellar's defence ships and the shareware nag ship, which the engine
//!   does not have.
//! - **The start**: a new pilot's rating is its `chär`'s `Kills`, sign
//!   extended (`_ActivateCharResource` @0x76d57-0x76d60), or 0 with no
//!   `chär` (@0x76f70).

/// The most a rating is set back to once it is over it.
pub const RATING_CAP: i64 = 10_000_000;

/// The rating after a kill of a ship of `strength` (its `shïp`'s raw
/// `Strength`), from `rating` (see the module docs).
#[must_use]
pub fn credit(rating: i64, strength: i64) -> i64 {
    if rating > RATING_CAP - 1 {
        RATING_CAP
    } else if strength <= 4 {
        rating + 1
    } else {
        rating + strength / 5
    }
}

/// Whether a kill counts toward the rating: the shooter is
/// `by_players_fleet` (the player, an escort or a fighter of its own) and
/// the victim's government is not `victim_derelict`.
#[must_use]
pub fn counts(by_players_fleet: bool, victim_derelict: bool) -> bool {
    by_players_fleet && !victim_derelict
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_victim_of_strength_4_or_less_adds_1() {
        assert_eq!(credit(10, 4), 11);
        assert_eq!(credit(10, 0), 11);
        assert_eq!(credit(10, -2), 11);
    }

    #[test]
    fn a_stronger_victim_adds_a_fifth_of_its_strength_rounded_down() {
        assert_eq!(credit(10, 5), 11);
        assert_eq!(credit(10, 9), 11);
        assert_eq!(credit(10, 10), 12);
        assert_eq!(credit(10, 325), 75);
    }

    #[test]
    fn the_cap_is_checked_before_the_add() {
        assert_eq!(credit(9_999_999, 50), 10_000_009, "just past the cap");
        assert_eq!(credit(10_000_000, 50), RATING_CAP, "over 9,999,999");
        assert_eq!(credit(10_000_001, 50), RATING_CAP);
        assert_eq!(credit(10_000_009, 0), RATING_CAP);
        assert_eq!(RATING_CAP, 10_000_000);
    }

    #[test]
    fn only_the_players_fleets_kills_of_ships_not_derelict_count() {
        assert!(counts(true, false));
        assert!(!counts(true, true));
        assert!(!counts(false, false));
        assert!(!counts(false, true));
    }
}
