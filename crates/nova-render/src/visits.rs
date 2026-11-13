//! The renderer's visits to flight systems: which of what it uploaded it
//! keeps, and which it lets go, as the pilot flies from system to system.
//!
//! A frame names the system it flies in, if any
//! ([`nova_view::DrawList::system`]: only flight names one). Each frame
//! that names a system other than the last one named starts a new visit
//! ([`Visits::arrive`]). A frame that names none (the spaceport, a comm
//! dialog, the galaxy map and its system viewer, the main menu) starts
//! none, and what it draws counts in the current visit: landing and taking
//! off let nothing go, and browsing the map doesn't let flight's planets
//! go.
//!
//! What was last drawn in this visit or the one before is kept
//! ([`Visits::kept`], [`KEPT_VISITS`]); anything older is let go once the
//! frame that began the visit is submitted. Two visits, not one: a trade
//! run back and forth between two systems makes nothing again, and only a
//! third system lets the first go. Going back makes it ready again in the
//! first frame there, as a first visit does.
//!
//! The renderer keeps one `Visits` for every kind it uploads. Planets are
//! the first (`crate::planets`); sky images could join it with a release
//! call of their own, as planets have [`Gpu::release_planet`](crate::Gpu::release_planet).

use nova_data::SystemId;

/// How many visits back something last drawn is kept: drawn in this
/// visit or the one before.
pub(crate) const KEPT_VISITS: u64 = 2;

/// The last system a frame named, and the visit it began (see the
/// [module docs](self)). Before any frame names a system, the visit is
/// 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Visits {
    last: Option<SystemId>,
    visit: u64,
}

impl Visits {
    /// Notes a frame that names `system`: whether it begins a new visit,
    /// which it does when it names a system other than the last one
    /// named. A frame that names none begins none.
    pub(crate) fn arrive(&mut self, system: Option<SystemId>) -> bool {
        match system {
            Some(system) if self.last != Some(system) => {
                self.last = Some(system);
                self.visit += 1;
                true
            }
            _ => false,
        }
    }

    /// The current visit's number.
    pub(crate) fn current(&self) -> u64 {
        self.visit
    }

    /// Whether something last drawn in visit `drawn_in` is kept: drawn in
    /// this visit or the one before ([`KEPT_VISITS`]).
    pub(crate) fn kept(&self, drawn_in: u64) -> bool {
        drawn_in + KEPT_VISITS > self.visit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOL: SystemId = SystemId(130);
    const ALPHA: SystemId = SystemId(131);
    const BARNARD: SystemId = SystemId(132);

    #[test]
    fn a_frame_in_another_system_starts_a_visit_and_one_naming_none_or_the_same_does_not() {
        let mut visits = Visits::default();
        assert_eq!(visits.current(), 0);
        assert!(!visits.arrive(None), "the main menu");
        assert_eq!(visits.current(), 0);
        assert!(visits.arrive(Some(SOL)), "the first flight frame");
        assert_eq!(visits.current(), 1);
        assert!(!visits.arrive(Some(SOL)), "still in Sol");
        assert!(!visits.arrive(None), "landed");
        assert!(!visits.arrive(Some(SOL)), "taken off in Sol again");
        assert_eq!(visits.current(), 1);
        assert!(visits.arrive(Some(ALPHA)));
        assert!(visits.arrive(Some(SOL)), "back is a new visit");
        assert!(visits.arrive(Some(BARNARD)));
        assert_eq!(visits.current(), 4);
    }

    #[test]
    fn what_was_drawn_in_this_visit_or_the_one_before_is_kept() {
        let mut visits = Visits::default();
        assert!(visits.kept(0), "before any system");
        visits.arrive(Some(SOL));
        assert!(visits.kept(0) && visits.kept(1));
        visits.arrive(Some(ALPHA));
        assert!(!visits.kept(0), "two visits back");
        assert!(visits.kept(1), "the visit before");
        assert!(visits.kept(2), "this visit");
        visits.arrive(Some(BARNARD));
        assert!(!visits.kept(1));
        assert!(visits.kept(2) && visits.kept(3));
        assert_eq!(KEPT_VISITS, 2);
    }
}
