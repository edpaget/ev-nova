//! What the player sees of the galaxy and has selected in it: the
//! stellars of the system shown and the listing they were built from, the
//! galaxy overlay and the systems known, the nav selection and its saved
//! copies, the NPC targeted, and the brackets' fly-ins. The coordinators
//! that read other state too (`set_player_system`, `refresh_galaxy`,
//! `rebuild_stellars`, `let_go_of_gone_target` and `enter_system`) stay
//! on `Session`.

use std::collections::BTreeSet;

use crate::brackets::FlyIn;
use crate::catalog::{LandingSite, SystemId};
use crate::navigation::NavMode;
use crate::story::OverlaidGalaxy;
use crate::traffic::npc::NpcId;

use super::nav::NavSlot;

/// What the player sees of the galaxy and has selected in it, in
/// `Session`'s former field order.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Chart {
    /// The stellars shown: those of `listed` that `shown` holds, and the
    /// one docked at, derived whenever [`Session::set_player_system`](super::Session::set_player_system) says
    /// and at the flight rebuild points (see the `entry` module).
    pub(super) sites: Vec<LandingSite>,
    /// Every stellar `shown` lists, in `NavDefs` order, through the
    /// galaxy overlay: what `sites` are derived from.
    pub(super) listed: Vec<LandingSite>,
    /// The system whose stellars `sites` are: the player's, except while
    /// the ship is docked at a stellar of another system.
    pub(super) shown: SystemId,
    /// The player's nav mode and its selection (see the `nav` module): a
    /// stellar selected is one of [`Chart::pick_candidates`], a stellar
    /// the system lists that is active, not only one of `sites`: the land
    /// key selects one another visible system owns (see the
    /// `stellar_picks` module).
    pub(super) nav: NavMode,
    /// The Con slot the mode-3 selection was written from, while it still
    /// stands (see the `nav` module's `NavSlot`).
    pub(super) nav_slot: Option<NavSlot>,
    /// The nav saved as a port dialog opened, until its close restores it
    /// ([`Session::save_port_nav`](super::Session::save_port_nav), [`Session::restore_port_nav`](super::Session::restore_port_nav)).
    pub(super) port_nav: Option<NavMode>,
    /// The nav saved as the bar opened, put back after each map opened
    /// from it ([`Session::open_bar`](super::Session::open_bar), [`Session::restore_bar_nav`](super::Session::restore_bar_nav)),
    /// forgotten with the news at landing and take-off, so no later
    /// stay can put back a nav saved in an earlier one.
    pub(super) bar_nav: Option<NavMode>,
    /// The galaxy every reader sees: the catalog's systems, read when the
    /// session starts, with the pilot's overlay (see the
    /// [`story`](crate::story) module's "Galaxy overlay").
    pub(super) galaxy: OverlaidGalaxy,
    /// The NPC the player targets, if any.
    pub(super) target: Option<NpcId>,
    /// How far out the nav target's brackets are (see the `brackets`
    /// module); not saved, as the original's pilot file does not keep it.
    pub(super) nav_fly_in: FlyIn,
    /// How far out the ship target's brackets are; not saved either.
    pub(super) target_fly_in: FlyIn,
    /// Every stellar the player's system lists, waiting for the take-off
    /// while the ship is docked at a stellar of another system, `shown`
    /// (see [`Session::set_player_system`](super::Session::set_player_system)).
    pub(super) next_sites: Option<Vec<LandingSite>>,
    /// The systems known (+0x1ec): what the galaxy map draws besides the
    /// explored ones, and what the escape search needs (see the
    /// [`exploration`](crate::exploration) module's "Known" and the
    /// `explored` module). Never saved, as the original saves no flag:
    /// each rebuild point works it out afresh.
    pub(super) known: BTreeSet<SystemId>,
    /// Whether a set expression has changed the pilot since the last
    /// settling: by the other reading of [`RuleKey::VisibilityRefresh`](crate::rulebook::RuleKey::VisibilityRefresh)
    /// the settling refreshes the galaxy then.
    pub(super) galaxy_stale: bool,
}

impl Chart {
    /// The chart as a session starts or loads: the galaxy `load` built
    /// and resumed in, showing `shown`, the player's system, with no
    /// stellar listed yet, the nav off and nothing targeted or known.
    pub(super) fn new(galaxy: OverlaidGalaxy, shown: SystemId) -> Self {
        Self {
            sites: Vec::new(),
            listed: Vec::new(),
            shown,
            nav: NavMode::Off,
            nav_slot: None,
            port_nav: None,
            bar_nav: None,
            galaxy,
            target: None,
            nav_fly_in: FlyIn::default(),
            target_fly_in: FlyIn::default(),
            next_sites: None,
            known: BTreeSet::new(),
            galaxy_stale: false,
        }
    }
}
