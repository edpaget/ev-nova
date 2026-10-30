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
//!   derelict (`_ShipIsDerelict` @0x8225). The test reads the shooter's
//!   own parent (+0x9a) and goes no further: a fighter launched by the
//!   player's escort is led by the escort (`_LaunchFighter` @0x3d866), so
//!   its kills do not count, though its fleet's root is the player. The
//!   same test convicts the player of the hit's crimes (see the session's
//!   `players_hits` module). The original also skips a
//!   stellar's defence ships and the shareware nag ship, which the engine
//!   does not have.
//! - **The start**: a new pilot's rating is its `chär`'s `Kills`, sign
//!   extended (`_ActivateCharResource` @0x76d57-0x76d60), or 0 with no
//!   `chär` (@0x76f70).
//! - **Its words** ([`string_index`]): `_DrawCombatRatingString` @0x57e5
//!   reads `_playerKills` and draws `_combatRatingString[index]`, which
//!   `_LoadStrings` fills from [`RATING_STRINGS`] #1 to #11
//!   (@0x71c5d-0x71c8f): 0 or less is the first, 1 to 99 the second, and
//!   each doubling from 100 the next, up to 25,600 and over, the
//!   eleventh. The main screen's pilot panel shows it, and so does the
//!   player info's Combat Rating row (`_PlayerInfoDialogUpdate` calls the
//!   same routine @0x52ade), both through one session lookup, so the two
//!   can't disagree.
//! - **Not read: `_EvalPlayerStudliness`** (@0x92190, its rating read
//!   @0x921f3). It scores `_playerStudliness` (0x31b246) from the comm
//!   ship's government record and the rating (1 when over 0, 2 more over
//!   499), but nothing in 1.1 calls it: no `call` targets 0x92190, which
//!   appears only in the symbol table (file offsets 0xed018, 0xed024,
//!   0xf8e5c), and `_playerStudliness`'s only pointer slot (0xdef04) is
//!   loaded only inside it (@0x9219f). The engine builds nothing for it.
//! - **Demand Tribute** (`_DoPlanetCommDialog`, rating read
//!   @0x975d5-0x975ea): below 12,800, cheats off, the planet laughs off
//!   the demand (`STR#` 3002 #`_responseRandom` + 6; see
//!   [`planet_comm::demand`](crate::planet_comm::demand)). The player's
//!   hits on a defence ship it sends up earn no kill (`_DamageShip`
//!   @0x3ad81-0x3ad89, see the session's `players_hits` module).

/// The most a rating is set back to once it is over it.
pub const RATING_CAP: i64 = 10_000_000;

/// The `STR#` of the combat rating's words, #1 to #11.
pub const RATING_STRINGS: i16 = 138;

/// The ratings at which each of the words after the first starts, the
/// second's first (`_DrawCombatRatingString` @0x57e5).
const RATING_STEPS: [i64; 10] = [1, 100, 200, 400, 800, 1600, 3200, 6400, 12_800, 25_600];

/// Which of [`RATING_STRINGS`]'s words, 0-based, `rating` reads as (see
/// the module docs).
#[must_use]
pub fn string_index(rating: i64) -> usize {
    RATING_STEPS.iter().filter(|&&step| rating >= step).count()
}

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
    fn the_strings_index_steps_at_each_doubling_from_100() {
        let cases = [
            (i64::MIN, 0),
            (-1, 0),
            (0, 0),
            (1, 1),
            (99, 1),
            (100, 2),
            (199, 2),
            (200, 3),
            (399, 3),
            (400, 4),
            (799, 4),
            (800, 5),
            (1599, 5),
            (1600, 6),
            (3199, 6),
            (3200, 7),
            (6399, 7),
            (6400, 8),
            (12_799, 8),
            (12_800, 9),
            (25_599, 9),
            (25_600, 10),
            (i64::MAX, 10),
        ];
        for (rating, index) in cases {
            assert_eq!(string_index(rating), index, "{rating}");
        }
        assert_eq!(RATING_STRINGS, 138);
    }

    #[test]
    fn only_the_players_fleets_kills_of_ships_not_derelict_count() {
        assert!(counts(true, false));
        assert!(!counts(true, true));
        assert!(!counts(false, false));
        assert!(!counts(false, true));
    }
}
