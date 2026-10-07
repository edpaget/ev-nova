//! Navigation: the stellar the player has selected as the navigation
//! target.
//!
//! The stellars that can be selected are the system's navigation
//! defaults, its `sÿst`'s `NavDef`s, in their order (the Nova Bible: "if
//! you don't set a planet as a nav default ... you can't select it").
//! Tab selects the next one, [`next_stellar`]: the first when none is
//! selected (or the one selected is no longer in the system), and after
//! the last the first again. There is no "no target" stop in the cycle,
//! and a system with no stellars has nothing to select. [`next_after`]
//! is that cycle over any list, which Hyper Select shares.

use crate::catalog::StellarId;

/// The stellar Tab selects next among `stellars`, in navigation order,
/// after `current`: the one after it, wrapping from the last to the
/// first, or the first when `current` is `None` or not among them.
/// `None` when there are no stellars.
#[must_use]
pub fn next_stellar(stellars: &[StellarId], current: Option<StellarId>) -> Option<StellarId> {
    next_after(stellars, current)
}

/// The item after `current` in the cycle `items`: the one after it,
/// wrapping from the last to the first, or the first when `current` is
/// `None` or not among them. `None` when there are no items. Tab's
/// stellars and Hyper Select's systems both cycle this way.
#[must_use]
pub fn next_after<T: Copy + PartialEq>(items: &[T], current: Option<T>) -> Option<T> {
    let after = current
        .and_then(|current| items.iter().position(|&item| item == current))
        .map_or(0, |at| at + 1);
    items.get(after).or_else(|| items.first()).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SystemId;

    fn ids(ids: &[i16]) -> Vec<StellarId> {
        ids.iter().copied().map(StellarId).collect()
    }

    #[test]
    fn from_no_target_tab_selects_the_first_stellar() {
        assert_eq!(
            next_stellar(&ids(&[131, 128, 129]), None),
            Some(StellarId(131))
        );
    }

    #[test]
    fn tab_selects_the_stellar_after_the_current_one() {
        let stellars = ids(&[131, 128, 129]);
        assert_eq!(
            next_stellar(&stellars, Some(StellarId(131))),
            Some(StellarId(128))
        );
        assert_eq!(
            next_stellar(&stellars, Some(StellarId(128))),
            Some(StellarId(129))
        );
    }

    #[test]
    fn after_the_last_stellar_tab_wraps_to_the_first() {
        let stellars = ids(&[131, 128, 129]);
        assert_eq!(
            next_stellar(&stellars, Some(StellarId(129))),
            Some(StellarId(131))
        );
        assert_eq!(
            next_stellar(&ids(&[140]), Some(StellarId(140))),
            Some(StellarId(140))
        );
    }

    #[test]
    fn a_target_no_longer_in_the_system_gives_way_to_the_first() {
        assert_eq!(
            next_stellar(&ids(&[131, 128]), Some(StellarId(150))),
            Some(StellarId(131))
        );
    }

    #[test]
    fn next_after_cycles_any_list_and_restarts_from_a_missing_current() {
        let systems = [SystemId(134), SystemId(131), SystemId(135)];
        assert_eq!(next_after(&systems, None), Some(SystemId(134)));
        assert_eq!(
            next_after(&systems, Some(SystemId(134))),
            Some(SystemId(131))
        );
        assert_eq!(
            next_after(&systems, Some(SystemId(135))),
            Some(SystemId(134)),
            "wraps"
        );
        assert_eq!(
            next_after(&systems, Some(SystemId(7))),
            Some(SystemId(134)),
            "missing"
        );
        assert_eq!(next_after::<SystemId>(&[], Some(SystemId(7))), None);
    }

    #[test]
    fn a_system_with_no_stellars_has_nothing_to_select() {
        assert_eq!(next_stellar(&[], None), None);
        assert_eq!(next_stellar(&[], Some(StellarId(128))), None);
    }
}
