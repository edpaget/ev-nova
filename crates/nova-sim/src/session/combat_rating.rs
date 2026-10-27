//! The combat rating's words, which the main screen's pilot panel and the
//! player info's Combat Rating row both draw with
//! `_DrawCombatRatingString` (@0x57e5; see [`rating`](crate::rating)).
//! Both screens read them here, so they can't disagree.

use super::Setup;
use crate::rating::{RATING_STRINGS, string_index};

impl Setup {
    /// The words for combat rating `rating`: `STR#` 138 entry
    /// [`string_index`] + 1, empty when the list has no such entry.
    pub(super) fn combat_rating_text(&self, rating: i64) -> String {
        self.nth_string(RATING_STRINGS, string_index(rating) + 1)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::catalog::CommCatalog;
    use crate::rating::RATING_STRINGS;
    use crate::session::Session;
    use crate::testkit::catalog;

    /// `STR#` 138, each "rating n" for its entry n, with #11 missing.
    #[derive(Debug)]
    struct Ratings;

    impl CommCatalog for Ratings {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                RATING_STRINGS => (1..=10).map(|n| format!("rating {n}")).collect(),
                _ => Vec::new(),
            }
        }
    }

    #[test]
    fn the_words_are_str_138_by_the_ratings_index() {
        let session = Session::start(&catalog())
            .expect("starts")
            .with_strings(Rc::new(Ratings));
        let text = |rating| session.setup.combat_rating_text(rating);
        assert_eq!(text(-5), "rating 1");
        assert_eq!(text(0), "rating 1");
        assert_eq!(text(1), "rating 2");
        assert_eq!(text(99), "rating 2");
        assert_eq!(text(100), "rating 3");
        assert_eq!(text(12_799), "rating 9");
        assert_eq!(text(12_800), "rating 10");
        assert_eq!(text(25_600), "", "#11 is missing");
    }
}
