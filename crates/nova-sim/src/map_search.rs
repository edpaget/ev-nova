//! The galaxy map's Find, as `_DoSystemSearch` (@0x11c86-0x12102) matches
//! a typed name against the systems' names. Which systems are searched is
//! the caller's: an in-flight map asks the session
//! (`nova_sim::Session::find_map_system`), and the viewer searches every
//! system in the data.
//!
//! - **Normalising** ([`normalise`]): the query (@0x11e63-0x11eba) and
//!   each name (@0x11f4b-0x11fb4) go through `__tolower` a character at a
//!   time, and only a-z and 0-9 are kept. `__tolower` leaves Mac Roman's
//!   high bytes alone, so no letter outside ASCII is kept.
//! - **The match** (@0x11fb6-0x11feb): m is the length of the common
//!   prefix of the normalised query and name.
//! - **The best** (@0x11ff2-0x12035): each candidate with m > 0 is
//!   counted. The best has the largest m; a tie goes to the shorter
//!   normalised name, then to the earlier candidate, so the caller gives
//!   them in ascending ID, as the original visits its system slots.
//! - **Found** (@0x1205c-0x120ce): the best is found when its m > 1, or
//!   when it is the only candidate counted. Otherwise, a single letter
//!   several names share, or no match at all, nothing is found
//!   (@0x120d0-0x120f3).
//!
//! **Not replicated.** The original's prefix compare stops at the query's
//! end, and tests the name's end only past it (`k > length`,
//! @0x11fca-0x11fde): at k = the name's length it compares the query's
//! next character with the byte after the name, left over from an
//! earlier, longer name or uninitialised, and a match there makes m one
//! longer. That read is out of bounds: here m stops at the shorter of the
//! two. The original reads a name's length as a signed byte (`movsbw`
//! @0x11fa9) and keeps at most 255 characters of the query; here both are
//! any length.
//!
//! An empty query is the map's to refuse (an empty text plays nothing),
//! not this module's: a query with nothing left once normalised finds
//! nothing here.

use crate::catalog::SystemId;

/// What a search found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchOutcome {
    /// This system: the map selects it and centres on it.
    Found(SystemId),
    /// No system: the map stays as it was.
    NotFound,
}

/// `text` lower-cased with only a-z and 0-9 kept (see the module docs).
#[must_use]
pub fn normalise(text: &str) -> String {
    text.chars()
        .map(|c| c.to_ascii_lowercase())
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .collect()
}

/// The system among `candidates`, each an ID and its name in ascending
/// ID, that `query` finds (see the module docs).
pub fn search<'a>(
    query: &str,
    candidates: impl IntoIterator<Item = (SystemId, &'a str)>,
) -> SearchOutcome {
    let query = normalise(query);
    let mut counted = 0_usize;
    // The best so far: its ID, m and normalised length.
    let mut best: Option<(SystemId, usize, usize)> = None;
    for (id, name) in candidates {
        let name = normalise(name);
        let m = query
            .bytes()
            .zip(name.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        if m == 0 {
            continue;
        }
        counted += 1;
        let better = best.is_none_or(|(_, best_m, best_len)| {
            m > best_m || (m == best_m && name.len() < best_len)
        });
        if better {
            best = Some((id, m, name.len()));
        }
    }
    match best {
        Some((id, m, _)) if m > 1 || counted == 1 => SearchOutcome::Found(id),
        _ => SearchOutcome::NotFound,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(id: i16) -> SearchOutcome {
        SearchOutcome::Found(SystemId(id))
    }

    fn among(query: &str, systems: &[(i16, &str)]) -> SearchOutcome {
        search(
            query,
            systems.iter().map(|&(id, name)| (SystemId(id), name)),
        )
    }

    #[test]
    fn normalising_lower_cases_and_keeps_only_letters_and_digits() {
        assert_eq!(normalise("Ne'mor"), "nemor");
        assert_eq!(normalise("HJG-1034"), "hjg1034");
        assert_eq!(normalise("Sol!"), "sol");
        assert_eq!(normalise("a b\tc"), "abc");
        assert_eq!(normalise("Égée"), "ge", "no letter outside ASCII");
        assert_eq!(normalise("!!"), "");
    }

    #[test]
    fn an_exact_name_is_found_whatever_its_case_and_punctuation() {
        let systems = [(128, "Sol"), (205, "Ne'mor"), (300, "Nemesis")];
        assert_eq!(among("NEMOR", &systems), found(205));
        assert_eq!(among("ne'mor", &systems), found(205));
    }

    #[test]
    fn a_prefix_is_enough_and_a_longer_query_stops_at_the_name() {
        let systems = [(128, "Sol"), (129, "Earth")];
        assert_eq!(among("sox", &systems), found(128), "m = 2");
        assert_eq!(among("solar", &systems), found(128), "m = 3");
    }

    #[test]
    fn the_longest_prefix_wins() {
        let systems = [(128, "Nesre Primus"), (129, "Nesre Secundus")];
        assert_eq!(among("nesres", &systems), found(129));
        assert_eq!(among("nesrep", &systems), found(128));
    }

    #[test]
    fn a_tie_goes_to_the_shorter_name_then_the_lower_id() {
        let systems = [(128, "Kania"), (389, "Kanto"), (461, "Kanius")];
        assert_eq!(among("kan", &systems), found(128));
        let systems = [(128, "Kanius"), (389, "Kanto"), (461, "Kania")];
        assert_eq!(among("kan", &systems), found(389), "shorter, then earlier");
        let systems = [(128, "Kanius"), (461, "Kania")];
        assert_eq!(among("kan", &systems), found(461), "shorter beats earlier");
    }

    #[test]
    fn one_letter_several_names_share_finds_nothing() {
        let systems = [(128, "Kania"), (129, "Kanto"), (130, "Sol")];
        assert_eq!(among("k", &systems), SearchOutcome::NotFound);
        assert_eq!(among("kz", &systems), SearchOutcome::NotFound, "m = 1");
    }

    #[test]
    fn one_letter_only_one_name_shares_finds_it() {
        let systems = [(128, "Kania"), (130, "Sol")];
        assert_eq!(among("k", &systems), found(128));
        assert_eq!(among("kz", &systems), found(128));
    }

    #[test]
    fn no_match_and_nothing_left_once_normalised_find_nothing() {
        let systems = [(128, "Kania"), (130, "Sol")];
        assert_eq!(among("zzz", &systems), SearchOutcome::NotFound);
        assert_eq!(among("!!", &systems), SearchOutcome::NotFound);
        assert_eq!(among("", &systems), SearchOutcome::NotFound);
        assert_eq!(among("sol", &[]), SearchOutcome::NotFound);
    }
}
