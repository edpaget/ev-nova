//! The stellars shown that have been dead since the last rebuild point:
//! the original's sprite slots holding the frames its rebuild
//! (`_HandleStellarSystemVisibility` @0x32aa5) gave a destroyed stellar.
//! See the module docs' "Built by phase 9" and "Checked for phase 9".
//!
//! At a rebuild point the original frees each stellar master no stellar of
//! the player's system uses, empties every cloned slot, and clones into
//! each slot the master of the graphic its stellar now shows: `DeadType`'s
//! when destroyed (@0x32f0f-0x32f2f). A slot left empty, as one of a
//! destroyed stellar whose `DeadType` has no sheet is, gets graphic type
//! 0's frames (`spïn` 1000, @0x33244-0x332d0). From then on, the per-frame
//! sprite tick (`_HandleStellarSprites` @0x2e77b-0x2e7e5) re-clones the
//! `Type` master into the slot of a stellar it finds whole, reloading the
//! master; one it finds destroyed with no `DeadType` sheet keeps whatever
//! frames it has. So a stellar is in [`Rebuilt`] from a rebuild point that
//! found it destroyed until a flight frame finds it whole, whatever `Y`
//! and `U` do between frames.
//!
//! The state is shared by the readings that depend on it:
//!
//! - story-runtime phase 9: the flight view draws such a stellar with no
//!   `DeadType` sheet with graphic type 0
//!   ([`RuleKey::DeadTypeFallback`](crate::RuleKey::DeadTypeFallback));
//! - story-runtime phase 10: its `Type` master was freed, so its landing
//!   radius falls back to 150 until a frame reloads it (plus that phase's
//!   own check for a master another stellar of the system shares);
//! - story-runtime phase 12: the frame count and frame index kept from the
//!   last rebuild, re-read on `SessionEvent::StellarsRebuilt` and
//!   `SessionEvent::SystemChanged`.

use std::collections::BTreeSet;

use crate::catalog::StellarId;

/// The stellars shown that were destroyed at the last rebuild point and
/// have not been seen whole at a flight frame since (see the module
/// docs). Never saved: a load is itself a rebuild point.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Rebuilt {
    dead: BTreeSet<StellarId>,
}

impl Rebuilt {
    /// A rebuild point: the stellars of `shown` that `destroyed` holds
    /// destroyed become the set, and every other leaves it. Whether the
    /// set changed.
    pub(crate) fn rebuild(
        &mut self,
        shown: impl IntoIterator<Item = StellarId>,
        destroyed: impl Fn(StellarId) -> bool,
    ) -> bool {
        let dead: BTreeSet<StellarId> = shown.into_iter().filter(|&id| destroyed(id)).collect();
        let changed = dead != self.dead;
        self.dead = dead;
        changed
    }

    /// A flight frame: every stellar that `destroyed` holds whole leaves
    /// the set.
    pub(crate) fn frame(&mut self, destroyed: impl Fn(StellarId) -> bool) {
        self.dead.retain(|&id| destroyed(id));
    }

    /// Whether `stellar` has been dead since the last rebuild point.
    pub(crate) fn dead(&self, stellar: StellarId) -> bool {
        self.dead.contains(&stellar)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn ids(ids: &[i16]) -> Vec<StellarId> {
        ids.iter().copied().map(StellarId).collect()
    }

    /// A destroyed state: these IDs are destroyed.
    fn destroyed(ids: &[i16]) -> impl Fn(StellarId) -> bool {
        let set: BTreeSet<i16> = ids.iter().copied().collect();
        move |id: StellarId| set.contains(&id.0)
    }

    fn dead(rebuilt: &Rebuilt, ids: &[i16]) -> Vec<bool> {
        ids.iter().map(|&id| rebuilt.dead(StellarId(id))).collect()
    }

    #[test]
    fn a_rebuild_records_exactly_the_shown_stellars_destroyed() {
        let mut rebuilt = Rebuilt::default();
        assert_eq!(dead(&rebuilt, &[128, 129]), [false, false], "none at first");
        assert!(rebuilt.rebuild(ids(&[128, 129, 130]), destroyed(&[129, 130, 140])));
        assert_eq!(
            dead(&rebuilt, &[128, 129, 130, 140]),
            [false, true, true, false],
            "140 is not shown"
        );
    }

    #[test]
    fn a_rebuild_reports_a_change_only_when_the_set_changed() {
        let mut rebuilt = Rebuilt::default();
        assert!(!rebuilt.rebuild(ids(&[128]), destroyed(&[])), "none still");
        assert!(rebuilt.rebuild(ids(&[128]), destroyed(&[128])));
        assert!(!rebuilt.rebuild(ids(&[128]), destroyed(&[128])), "the same");
        assert!(rebuilt.rebuild(ids(&[128, 129]), destroyed(&[128, 129])));
        assert!(
            rebuilt.rebuild(ids(&[129]), destroyed(&[128, 129])),
            "one less"
        );
        assert_eq!(dead(&rebuilt, &[128, 129]), [false, true]);
        assert!(rebuilt.rebuild(ids(&[]), destroyed(&[129])), "emptied");
        assert_eq!(dead(&rebuilt, &[129]), [false]);
    }

    #[test]
    fn a_frame_drops_a_stellar_seen_whole_and_keeps_one_still_destroyed() {
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(ids(&[128, 129]), destroyed(&[128, 129]));
        rebuilt.frame(destroyed(&[129]));
        assert_eq!(dead(&rebuilt, &[128, 129]), [false, true]);
        rebuilt.frame(destroyed(&[128, 129]));
        assert_eq!(dead(&rebuilt, &[128, 129]), [false, true], "not back");
    }

    #[test]
    fn y_u_y_with_no_frame_between_keeps_it_and_a_frame_while_whole_drops_it() {
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(ids(&[128]), destroyed(&[128]));
        // `U` then `Y`: nothing is told until a frame, which finds it
        // destroyed again.
        rebuilt.frame(destroyed(&[128]));
        assert!(rebuilt.dead(StellarId(128)));
        // `U`, a frame while whole, then `Y`.
        rebuilt.frame(destroyed(&[]));
        rebuilt.frame(destroyed(&[128]));
        assert!(!rebuilt.dead(StellarId(128)));
    }
}
