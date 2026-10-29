//! What a session leaves the run of its galaxy refreshes: the systems the
//! last one hid ([`LastRefresh`]), the original's visible flags (`sÿst`
//! +0x1eb) as the last `_PropagateMissionBitEffects` left them. A new
//! pilot's neighbour pass reads them by the engine's reading of
//! [`RuleKey::NewPilotVisibility`] (see the `explored` module's "A new pilot").
//!
//! **Why it is carried.** `_DoNewPilot` (@0x18b0a) tests +0x1eb of each
//! of its start system's links (@0x18fc5) before its own refresh
//! (@0x1925e); nothing it calls before the loop refreshes. So the loop
//! reads what the run's last refresh wrote:
//!
//! - at launch with a last pilot, that pilot's (`_OpenLastPilotFile`
//!   @0x473ef → `_LoadPilotData` @0x751ca/@0x7531f, then @0x4740c and
//!   `_DoMainScreen` @0x1afcf);
//! - at launch with none, clear bits, the player at system index 0 (sÿst
//!   128; `_ResetPlayer` @0x1d485, @0x4739a) forced visible (@0x472ba,
//!   @0x4740c);
//! - after Open Pilot, that pilot's (`_MainScreenChoice` @0x1a8aa);
//! - after play, the last in-game refresh (a landing @0xa175f/@0xa1af2,
//!   an arrival @0x6bf3f, game entry `_EnterGameFromMainScreen` @0x19f81,
//!   `_PlayGame` @0x457e9);
//! - after another new pilot not yet played, that pilot's @0x1925e,
//!   before its `OnStart`.
//!
//! **What a session leaves** ([`Session::last_refresh`]): the overlay's
//! hidden set as every path through `Session::refresh_galaxy` and
//! `Session::refresh_galaxy_quietly` leaves it, whether the refresh
//! changed anything or not, the player's move out of a hidden system
//! included. Before the first refresh it holds nothing hidden; no reader
//! sees that, as [`Session::fly`] refreshes before it returns.
//!
//! **Ordering against the original.** [`Session::begin`] runs
//! `_PlayGame`'s refresh (@0x457e9) early, after the `OnStart`, so the
//! flight shows the new pilot's bits from its first frame; the original
//! runs it only on entering play. The early refresh crosses a boundary
//! only through this carry, so `begin` keeps the snapshot it had before
//! the `OnStart` (`_DoNewPilot`'s own @0x1925e, which [`Session::fly`]
//! stands for), and [`Session::enter_game`], the entry into play from
//! the main screen, takes it afresh.
//!
//! **What a new pilot reads** ([`Session::with_last_refresh`]): the
//! carried snapshot, or, with none, the launch's (`Chart::launch_refresh`).

use std::collections::BTreeSet;

use super::{Chart, Session, Setup, Statics};
use crate::catalog::SystemId;
use crate::control::{FreshPilot, Gate};
use crate::rulebook::{RuleKey, RuleSource};
use crate::story::GalaxyOverlay;
use crate::story::galaxy::hidden_by;

/// The systems a galaxy refresh left hidden: the original's visible flags
/// (`sÿst` +0x1eb) as the last `_PropagateMissionBitEffects` left them
/// (see the module docs). A system not in the catalog is governed by
/// whether it exists, never by this.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LastRefresh {
    /// The systems hidden.
    hidden: BTreeSet<SystemId>,
}

impl LastRefresh {
    /// A refresh that left exactly `hidden` hidden.
    #[must_use]
    pub fn hiding(hidden: impl IntoIterator<Item = SystemId>) -> Self {
        Self {
            hidden: hidden.into_iter().collect(),
        }
    }

    /// Whether it left `system` hidden.
    #[must_use]
    pub fn hides(&self, system: SystemId) -> bool {
        self.hidden.contains(&system)
    }
}

/// The system the player is in at launch with no pilot: index 0 of the
/// system table, `sÿst` 128 (`_ResetPlayer` @0x1d485, @0x4739a).
const LAUNCH_SYSTEM: SystemId = SystemId(128);

impl Session {
    /// What its last galaxy refresh left hidden, to hand on to a new
    /// pilot made later in the same run ([`Session::with_last_refresh`];
    /// see the module docs).
    #[must_use]
    pub(crate) fn last_refresh(&self) -> LastRefresh {
        self.chart.last_refresh.clone()
    }

    /// The session, a new pilot's neighbour pass reading `carried`, what
    /// a session earlier in the same run left ([`Session::last_refresh`]),
    /// in place of the launch's refresh, by the engine's reading of
    /// [`RuleKey::NewPilotVisibility`].
    #[must_use]
    pub(crate) fn with_last_refresh(mut self, carried: LastRefresh) -> Self {
        self.chart.carried_refresh = Some(carried);
        self
    }

    /// Enters play from the main screen (`_EnterGameFromMainScreen`
    /// @0x19f81): its refresh leaves the galaxy as the session already
    /// has it, so only the snapshot a later new pilot reads is taken
    /// afresh (see the module docs' "Ordering against the original").
    pub fn enter_game(&mut self) {
        self.chart.snapshot_refresh();
    }
}

impl Chart {
    /// Takes the snapshot a refresh leaves: the overlay's hidden set as
    /// it now stands.
    pub(super) fn snapshot_refresh(&mut self) {
        self.last_refresh = LastRefresh {
            hidden: self.galaxy.overlay().hidden(),
        };
    }

    /// The visibility a new pilot's neighbour pass reads: by the engine,
    /// the carried snapshot, else the launch's; by the other reading, the
    /// galaxy as the new pilot's own refresh left it.
    pub(super) fn new_pilot_refresh(&self, statics: &Statics, setup: &Setup) -> LastRefresh {
        match setup.rules.source_for(RuleKey::NewPilotVisibility) {
            RuleSource::Engine => self
                .carried_refresh
                .clone()
                .unwrap_or_else(|| self.launch_refresh(statics, setup)),
            RuleSource::Bible => LastRefresh {
                hidden: self.galaxy.overlay().hidden(),
            },
        }
    }

    /// The launch's refresh with no pilot (@0x472ba, @0x4740c): every
    /// catalog system's `Visibility` tested with clear bits
    /// ([`FreshPilot`]), then the player's move out of a hidden system
    /// from [`LAUNCH_SYSTEM`], which shows its place's head when no system
    /// there is visible (@0x9993f-0x999a4). Worked out on a scratch copy
    /// of the galaxy; the session's own is untouched.
    fn launch_refresh(&self, statics: &Statics, setup: &Setup) -> LastRefresh {
        let gate = Gate {
            control_bits: &*setup.control_bits.0,
            pilot: &FreshPilot,
        };
        let mut hidden = hidden_by(self.galaxy.catalog_systems(), &statics.visibility, &gate);
        let mut overlay = GalaxyOverlay::default();
        overlay.set_hidden(&hidden);
        let mut scratch = self.galaxy.clone();
        scratch.set_overlay(overlay);
        if let Some(to) = scratch.hidden_move(LAUNCH_SYSTEM)
            && to.make_visible
        {
            hidden.remove(&to.to);
        }
        LastRefresh { hidden }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SystemVisibility;
    use crate::chance::NeverFires;
    use crate::control::{Script, Test};
    use crate::pilot::Pilot;
    use crate::rulebook::Rulebook;
    use crate::testkit::{FakePilotCatalog, catalog, hideable, hiding_bit};

    /// The fixture catalog, each system hideable by its own bit.
    fn hideable_catalog() -> FakePilotCatalog {
        let mut catalog = catalog();
        hideable(&mut catalog);
        catalog
    }

    fn hiding_132() -> Pilot {
        let catalog = hideable_catalog();
        let mut pilot = Pilot::new(&catalog, "").expect("starts");
        pilot.bits_mut().set(hiding_bit(132));
        pilot
    }

    #[test]
    fn a_refresh_leaves_what_it_hid() {
        assert_eq!(LastRefresh::default(), LastRefresh::hiding([]));
        let refresh = LastRefresh::hiding([SystemId(132)]);
        assert!(refresh.hides(SystemId(132)));
        assert!(!refresh.hides(SystemId(131)));
    }

    #[test]
    fn a_session_leaves_its_last_refresh() {
        let catalog = hideable_catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.last_refresh(), LastRefresh::default());
        session.pilot.bits_mut().set(hiding_bit(132));
        assert!(session.refresh_galaxy(&catalog), "the refresh hides 132");
        assert_eq!(session.last_refresh(), LastRefresh::hiding([SystemId(132)]));
        // A refresh that changes nothing still takes it.
        session.chart.last_refresh = LastRefresh::default();
        assert!(!session.refresh_galaxy(&catalog));
        assert_eq!(session.last_refresh(), LastRefresh::hiding([SystemId(132)]));
    }

    #[test]
    fn a_load_leaves_the_loaded_pilots_refresh() {
        let catalog = hideable_catalog();
        let loaded = Session::load(&catalog, hiding_132(), &Rulebook::default()).expect("loads");
        assert_eq!(loaded.last_refresh(), LastRefresh::hiding([SystemId(132)]));
    }

    #[test]
    fn a_load_whose_refresh_changes_nothing_still_leaves_it() {
        let catalog = hideable_catalog();
        let mut pilot = hiding_132();
        // Saved hidden already: the load's refresh writes nothing.
        pilot.galaxy.set_visible(SystemId(132), Some(false));
        let loaded = Session::load(&catalog, pilot, &Rulebook::default()).expect("loads");
        assert!(!loaded.galaxy().visible(SystemId(132)));
        assert_eq!(loaded.last_refresh(), LastRefresh::hiding([SystemId(132)]));
    }

    #[test]
    fn a_begun_pilot_leaves_its_visibility_before_on_start() {
        let mut catalog = catalog();
        catalog.visibility.push(SystemVisibility {
            id: SystemId(132),
            visibility: Test::parse("b50"),
        });
        if let Ok(start) = &mut catalog.character {
            start.on_start = Script::parse("b50");
        }
        let mut session = Session::start(&catalog).expect("starts");
        session.begin(&catalog, &mut NeverFires);
        assert!(session.galaxy().visible(SystemId(132)), "shown by OnStart");
        assert_eq!(
            session.last_refresh(),
            LastRefresh::hiding([SystemId(132)]),
            "@0x1925e, before the OnStart"
        );
        session.enter_game();
        assert_eq!(session.last_refresh(), LastRefresh::default(), "@0x19f81");
    }

    #[test]
    fn a_carried_refresh_is_what_a_new_pilot_reads_by_the_engine() {
        let catalog = hideable_catalog();
        let carried = LastRefresh::hiding([SystemId(131)]);
        let session = Session::fly(&catalog, hiding_132())
            .expect("flies")
            .with_last_refresh(carried.clone());
        let reads = |session: &Session| {
            session
                .chart
                .new_pilot_refresh(&session.statics, &session.setup)
        };
        assert_eq!(reads(&session), carried);
        let session = session.with_rules(
            Rulebook::default().with_override(RuleKey::NewPilotVisibility, RuleSource::Bible),
        );
        assert_eq!(
            reads(&session),
            LastRefresh::hiding([SystemId(132)]),
            "its own"
        );
    }

    #[test]
    fn with_none_carried_a_new_pilot_reads_the_launchs_clear_bits() {
        let catalog = hideable_catalog();
        let session = Session::fly(&catalog, hiding_132()).expect("flies");
        assert_eq!(
            session
                .chart
                .new_pilot_refresh(&session.statics, &session.setup),
            LastRefresh::default()
        );
    }

    #[test]
    fn the_launchs_refresh_shows_the_head_of_system_128s_place_when_none_there_is() {
        // 127, 128 and 131 share 131's place, each hidden at launch.
        let mut catalog = catalog();
        catalog
            .star_map
            .push(crate::testkit::star(127, (600.0, 0.0), &[]));
        catalog
            .star_map
            .push(crate::testkit::star(128, (600.0, 0.0), &[]));
        for id in [127, 128, 131] {
            catalog.visibility.push(SystemVisibility {
                id: SystemId(id),
                visibility: Test::parse("b1"),
            });
        }
        let session =
            Session::fly(&catalog, Pilot::new(&catalog, "").expect("starts")).expect("flies");
        assert_eq!(
            session
                .chart
                .new_pilot_refresh(&session.statics, &session.setup),
            LastRefresh::hiding([SystemId(128), SystemId(131)]),
            "127, the head, shown"
        );
        // With 131 visible at launch, nothing is shown.
        catalog.visibility.retain(|test| test.id != SystemId(131));
        let session =
            Session::fly(&catalog, Pilot::new(&catalog, "").expect("starts")).expect("flies");
        assert_eq!(
            session
                .chart
                .new_pilot_refresh(&session.statics, &session.setup),
            LastRefresh::hiding([SystemId(127), SystemId(128)]),
        );
    }
}
