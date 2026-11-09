//! A resource's name as the game shows it.

/// A resource's name as the game shows it: up to its first ';', after
/// which the designers kept their notes (stock `shïp` 361 is
/// "Shuttle;Second-Hand - poor"). Nothing is trimmed: spaces either side
/// of the cut stay, and a name with no ';' comes back whole.
///
/// This is not `nova_sim_catalog::trim_resource_name`, the port of the
/// original's `_TrimResourceName`, which cuts at the *last* ';' and drops
/// the spaces and ';'s before it.
#[must_use]
pub fn shown_name(name: &str) -> &str {
    name.split(';').next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::shown_name;

    #[test]
    fn a_name_with_a_note_stops_at_its_first_semicolon() {
        assert_eq!(shown_name("Shuttle;Second-Hand - poor"), "Shuttle");
        assert_eq!(shown_name("a;b;c"), "a");
    }

    #[test]
    fn a_name_without_a_note_is_kept_whole_spaces_and_all() {
        assert_eq!(shown_name("Lightning"), "Lightning");
        assert_eq!(shown_name(" Ace "), " Ace ");
        assert_eq!(shown_name(""), "");
    }

    #[test]
    fn an_empty_name_before_the_semicolon_is_empty() {
        assert_eq!(shown_name(";"), "");
        assert_eq!(shown_name(";note"), "");
    }

    #[test]
    fn spaces_around_the_semicolon_are_kept() {
        assert_eq!(shown_name("Lightning; Wild Geese"), "Lightning");
        assert_eq!(shown_name(" Koria ;x"), " Koria ");
    }
}
