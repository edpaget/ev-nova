//! `_TrimResourceName` (@0x717ac-0x717fd), ported whole: how the original
//! cuts the designer's note off a resource name, and the New Pilot dialog's
//! name and nickname (`_DoNewPilot` @0x18bf2-0x18c03).

/// `name` as `_TrimResourceName` (@0x717ac-0x717fd) keeps it.
///
/// The original walks a Pascal string from its last character to its first:
///
/// - Until it meets a `;` it changes nothing, so a name with no `;` is kept
///   whole, leading and trailing spaces included.
/// - At the first `;` from the end the kept length becomes the characters
///   before it (@0x717ca-0x717db).
/// - After that a space moves the kept length to the characters before it
///   (@0x717e9-0x717ef), a `;` is passed over without moving it
///   (@0x717f1-0x717f3), and the first other character keeps the name up to
///   and including itself and ends the walk (@0x717f5).
///
/// So `";;"` keeps `";"`: a leading `;` that no space moved past stays.
#[must_use]
pub fn trim_resource_name(name: &str) -> &str {
    let mut kept = name.len();
    let mut found = false;
    for (at, c) in name.char_indices().rev() {
        if !found {
            if c == ';' {
                kept = at;
                found = true;
            }
        } else if c == ' ' {
            kept = at;
        } else if c != ';' {
            kept = at + c.len_utf8();
            break;
        }
    }
    &name[..kept]
}

#[cfg(test)]
mod tests {
    use super::trim_resource_name;

    #[test]
    fn a_name_without_a_semicolon_is_kept_whole_spaces_and_all() {
        assert_eq!(trim_resource_name(" Ada "), " Ada ");
        assert_eq!(trim_resource_name("Ada"), "Ada");
    }

    #[test]
    fn the_last_semicolon_and_what_follows_it_are_cut() {
        assert_eq!(trim_resource_name("Ace;note"), "Ace");
        assert_eq!(trim_resource_name("Ace;a;b"), "Ace;a");
    }

    #[test]
    fn the_walk_stops_at_the_first_other_character_before_the_last_semicolon() {
        assert_eq!(trim_resource_name("Ace ;; x;"), "Ace ;; x");
    }

    #[test]
    fn spaces_and_semicolons_before_the_last_semicolon_are_passed_over() {
        assert_eq!(trim_resource_name("Ace ;; ;"), "Ace");
        assert_eq!(trim_resource_name("Ace  ;"), "Ace");
    }

    #[test]
    fn a_leading_semicolon_no_space_moved_past_is_kept() {
        assert_eq!(trim_resource_name(";"), "");
        assert_eq!(trim_resource_name(" ;x"), "");
        assert_eq!(trim_resource_name(";;"), ";");
        assert_eq!(trim_resource_name(";; ;"), ";;");
    }

    #[test]
    fn an_empty_name_stays_empty() {
        assert_eq!(trim_resource_name(""), "");
    }

    #[test]
    fn a_character_wider_than_a_byte_is_kept_whole() {
        assert_eq!(trim_resource_name("Zoë;x"), "Zoë");
    }
}
