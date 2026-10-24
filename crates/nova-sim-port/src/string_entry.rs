//! One entry of a `STR#` list, the lookup the port's prompts and default
//! names share. A missing list or entry reads as empty.

use crate::catalog::CommCatalog;

/// Entry `index` (1-based) of `STR#` `list`; empty when either is missing.
pub(crate) fn entry(strings: &(impl CommCatalog + ?Sized), list: i16, index: usize) -> String {
    strings
        .string_list(list)
        .into_iter()
        .nth(index - 1)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One canned `STR#` list, 128.
    struct OneList;

    impl CommCatalog for OneList {
        fn string_list(&self, id: i16) -> Vec<String> {
            if id == 128 {
                vec!["first".to_owned(), "second".to_owned()]
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn a_present_entry_is_read_one_based() {
        assert_eq!(entry(&OneList, 128, 2), "second");
    }

    #[test]
    fn an_index_past_the_end_is_empty() {
        assert_eq!(entry(&OneList, 128, 3), "");
    }

    #[test]
    fn a_missing_list_is_empty() {
        assert_eq!(entry(&OneList, 129, 1), "");
    }
}
