//! What the mission screens decide (see the [module docs](super)'s
//! "Screens"): the order offers show in.

use std::cmp::Reverse;

use crate::rulebook::RuleSource;

/// `items`, each with its `DispWeight`, in list order, put in the order
/// they show, as [`RuleKey::MissionOrder`](crate::RuleKey::MissionOrder)
/// reads it. By the engine (`_RandomizeMissionData` @0xa1842-0xa197d),
/// those weighing 0 or more keep their order and come first, then the
/// rest by descending weight, ties in list order; by the Bible, all by
/// descending weight, ties in list order.
#[must_use]
pub fn display_order<T>(items: Vec<(T, i16)>, reading: RuleSource) -> Vec<T> {
    // The engine's first threshold is index -1's weight, the zero padding
    // before `_missionAvailData` (@0xa18bb): 0.
    let (mut shown, mut rest): (Vec<_>, Vec<_>) = match reading {
        RuleSource::Engine => items.into_iter().partition(|&(_, weight)| weight >= 0),
        RuleSource::Bible => (Vec::new(), items),
    };
    rest.sort_by_key(|&(_, weight)| Reverse(weight));
    shown.extend(rest);
    shown.into_iter().map(|(item, _)| item).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ordered(weights: &[i16], reading: RuleSource) -> Vec<usize> {
        let items = weights.iter().copied().enumerate().collect();
        display_order(items, reading)
    }

    #[test]
    fn by_the_engine_weights_of_0_or_more_keep_their_order_then_the_rest_descend() {
        assert_eq!(
            ordered(&[3, -1, 0, -5, -1, 7], RuleSource::Engine),
            [0, 2, 5, 1, 4, 3]
        );
    }

    #[test]
    fn by_the_bible_every_weight_descends_ties_in_list_order() {
        assert_eq!(
            ordered(&[3, -1, 0, -5, -1, 7], RuleSource::Bible),
            [5, 0, 2, 1, 4, 3]
        );
    }

    #[test]
    fn all_zero_keeps_list_order_under_both_readings() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(ordered(&[0, 0, 0], reading), [0, 1, 2], "{reading:?}");
        }
        assert_eq!(ordered(&[], RuleSource::Engine), Vec::<usize>::new());
    }
}
