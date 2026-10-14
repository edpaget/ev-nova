//! The galaxy overlay: what play changes over the static catalog's
//! galaxy ([`GalaxyOverlay`], saved on the pilot), and the one galaxy
//! every reader in the session sees ([`OverlaidGalaxy`]): the static
//! catalog with the overlay applied. See the `story` module's "Galaxy
//! overlay" for the design and the original's evidence.
//!
//! - A system is visible unless the overlay hides it, and its government
//!   is the overlay's when it has one, else the catalog's. The overlaid
//!   [`StarMap`] has the visible systems alone, with those governments
//!   ([`StarMap::with_hidden`]).
//! - A stellar's government is the overlay's when it has one, else its
//!   record's ([`OverlaidGalaxy::site`]). A destroyed stellar holds its
//!   regeneration [`Countdown`], saved.
//! - A nebula is active unless the overlay says otherwise.
//! - A system's visibility is written from its `Visibility` at each
//!   galaxy refresh ([`GalaxyOverlay::refresh_visibility`]), overwriting
//!   every system's; `Some(true)` is the player's move's reveal alone.
//! - Each stellar's owner is derived from the overlay, never stored
//!   ([`OverlaidGalaxy::owner`]): the chain-shadow rule of
//!   `_PropagateMissionBitEffects` (@0x996ed-0x997bd), as
//!   [`RuleKey::ColocatedResolution`](crate::RuleKey::ColocatedResolution)
//!   reads.
//! - A system at the same place as a hidden one stands in for it
//!   ([`OverlaidGalaxy::same_place`]): the lowest-ID visible system at its
//!   exact map position, as the original's `_FindActiveCoLocatedSystem`
//!   (@0x4be0) gives it, for a visible system too unless
//!   [`RuleKey::ColocatedResolution`](crate::RuleKey::ColocatedResolution)
//!   reads the other way ([`CoLocated`]). A player in, or arriving in, a
//!   hidden system goes there or, with none, to the chain head made
//!   visible ([`OverlaidGalaxy::arrival`], @0x998f1-0x99a36). A player in
//!   a visible system shadowed at its place stays: the original's move
//!   tests only the player's own system's visible flag (@0x99918).
//! - A stellar with no owner that the player's system lists is held by
//!   the player's system ([`OverlaidGalaxy::holder`]), as
//!   `_HandleStellarSystemVisibility`'s rescue writes it (@0x32c58-0x32cb3)
//!   until the next refresh clears every owner. The player's system shows
//!   exactly the stellars it holds, while it is visible
//!   ([`OverlaidGalaxy::shows`], [`OverlaidGalaxy::shown`]): the
//!   original's active stellars owned by the player's system
//!   (@0x32cbe-0x32d4c).
//! - `_FindSystemFromStellar` (0x7ec1-0x7f64) gives the lowest visible
//!   lister, shadowed or not, else the lowest lister
//!   ([`OverlaidGalaxy::find_system_from_stellar`]); the owner-first
//!   readers take the holder, else that ([`OverlaidGalaxy::system_of`]).
//!   A reader finds a stellar's listers with [`listers_of`].
//!
//! Of `_HandleStellarSystemVisibility` (0x32aa5-0x33331), this module
//! models the rescue, the active test and the shown set. The rest is left
//! to others:
//!
//! - the course rewrite and cut (@0x32ab3-0x32b0f): crons-news-ranks
//!   phase 15;
//! - the explored level shared to the chain head (@0x32b0f-0x32b8f):
//!   crons-news-ranks phase 14 models it, as one explored set, in the
//!   session's rebuild points (the session's `explored` module);
//! - the known flag +0x1ec (@0x32b91-0x32c10): deferred to
//!   crons-news-ranks-fidelity phase 20, as no reader in the engine uses
//!   it yet (the galaxy map draws every visible system);
//! - the dominated byte +0x46, set for an active stellar with `Flags2`
//!   0x0020 (@0x32ce5-0x32ceb): the engine reads the bit directly
//!   (`landing::cleared`, `mission::dominated`, `Session::markup`), and
//!   the stored byte is task planet-domination's;
//! - +0x47, the defence fleet launched, cleared for every stellar
//!   (@0x32d5d): task planet-domination;
//! - unloading the master sprites no longer needed (@0x32d74-0x32e4e):
//!   nothing observable, not modelled;
//! - the sprite slots and their fallback frames (@0x32e54-0x33331): the
//!   story module's `rebuild`. The slots' ascending-ID order is left to
//!   the fidelity work.
//!
//! The function plays no sound.

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{
    GovtId, LandingSite, ListedStellar, NebulaId, StarSystem, StellarId, SystemId, SystemVisibility,
};
use crate::control::Gate;
use crate::geometry::Vec2;
use crate::hyperspace::{CoLocated, StarMap};

/// What the overlay changes of one system. The default changes nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SystemOverride {
    /// Whether it is visible: `Some(false)` hidden by the last refresh of
    /// its `Visibility`, `Some(true)` a chain head the player's move made
    /// visible ([`HiddenMove::make_visible`]), which owns no stellar
    /// ([`OverlaidGalaxy::owner`]), and `None` visible, as the catalog has
    /// it. The refresh never writes `Some(true)`, so it marks the move's
    /// reveal exactly.
    pub visible: Option<bool>,
    /// Its government: `Some(None)` makes it independent, and `None` leaves
    /// the catalog's.
    pub govt: Option<Option<GovtId>>,
}

/// A destroyed stellar's regeneration countdown: the days left until the
/// daily step restores it (see the `story` module's "Galaxy overlay").
/// The original keeps it at the stellar's +0x47c (@0xb68a).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Countdown {
    /// The days left: at 1 or 0 the next day restores it.
    pub days_left: u16,
}

/// What the overlay changes of one stellar. The default changes nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StellarOverride {
    /// Its government: `Some(None)` makes it independent, and `None` leaves
    /// its record's.
    pub govt: Option<Option<GovtId>>,
    /// Its regeneration countdown while it is destroyed; `None` while it is
    /// not.
    pub destroyed: Option<Countdown>,
}

/// What play has changed over the static galaxy: per system, per stellar
/// and per nebula, each only where it changes something. The default is
/// no change at all. Each setter says whether it changed anything, and an
/// entry left changing nothing is dropped, so two overlays are equal
/// exactly when they change the same things.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GalaxyOverlay {
    systems: BTreeMap<SystemId, SystemOverride>,
    stellars: BTreeMap<StellarId, StellarOverride>,
    nebulae: BTreeMap<NebulaId, bool>,
}

/// Changes `map`'s entry for `key` as `change` says, dropping it when it
/// is left at its default, and says whether it changed.
fn edit<K: Ord, V: Copy + Default + PartialEq>(
    map: &mut BTreeMap<K, V>,
    key: K,
    change: impl FnOnce(&mut V),
) -> bool {
    let was = map.get(&key).copied().unwrap_or_default();
    let mut now = was;
    change(&mut now);
    if now == V::default() {
        map.remove(&key);
    } else {
        map.insert(key, now);
    }
    now != was
}

impl GalaxyOverlay {
    /// Whether it changes nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty() && self.stellars.is_empty() && self.nebulae.is_empty()
    }

    /// What it changes of `system`.
    #[must_use]
    pub fn system(&self, system: SystemId) -> SystemOverride {
        self.systems.get(&system).copied().unwrap_or_default()
    }

    /// What it changes of `stellar`.
    #[must_use]
    pub fn stellar(&self, stellar: StellarId) -> StellarOverride {
        self.stellars.get(&stellar).copied().unwrap_or_default()
    }

    /// Whether `nebula` is active, if it says.
    #[must_use]
    pub fn nebula(&self, nebula: NebulaId) -> Option<bool> {
        self.nebulae.get(&nebula).copied()
    }

    /// Every system it changes, by ascending ID.
    pub fn systems(&self) -> impl Iterator<Item = (SystemId, SystemOverride)> + '_ {
        self.systems.iter().map(|(&id, &change)| (id, change))
    }

    /// Every stellar it changes, by ascending ID.
    pub fn stellars(&self) -> impl Iterator<Item = (StellarId, StellarOverride)> + '_ {
        self.stellars.iter().map(|(&id, &change)| (id, change))
    }

    /// Every nebula it sets active or not, by ascending ID.
    pub fn nebulae(&self) -> impl Iterator<Item = (NebulaId, bool)> + '_ {
        self.nebulae.iter().map(|(&id, &active)| (id, active))
    }

    /// Sets whether `system` is visible (`None` leaves it as the catalog
    /// has it), and says whether that changed anything. A visibility
    /// written here lasts only until the next refresh of the systems'
    /// `Visibility` ([`GalaxyOverlay::refresh_visibility`]), which
    /// overwrites every system's; `Some(true)` is the player's move's
    /// alone (see [`SystemOverride::visible`]).
    pub fn set_visible(&mut self, system: SystemId, visible: Option<bool>) -> bool {
        edit(&mut self.systems, system, |change| change.visible = visible)
    }

    /// Hides exactly the systems of `hidden` and leaves every other one
    /// visible as the catalog has it (`None`), clearing any `Some(true)`,
    /// and says whether that changed anything.
    pub fn set_hidden(&mut self, hidden: &BTreeSet<SystemId>) -> bool {
        let held: Vec<SystemId> = self.systems.keys().copied().collect();
        let mut changed = false;
        for system in held.into_iter().filter(|system| !hidden.contains(system)) {
            changed |= self.set_visible(system, None);
        }
        for &system in hidden {
            changed |= self.set_visible(system, Some(false));
        }
        changed
    }

    /// Writes every system's visibility from its `Visibility`, as the
    /// original's galaxy refresh does (`_PropagateMissionBitEffects`
    /// @0x996a1-0x996eb): each of `systems` (the catalog's) hidden when its
    /// test in `tests` fails for `gate` ([`hidden_by`]), and left visible
    /// (`None`) otherwise, a `Some(true)` the player's move wrote included;
    /// and any visibility held for a system outside them cleared. Says
    /// whether that changed anything. The original hides an unused slot
    /// too; the engine has only catalog systems.
    pub fn refresh_visibility(
        &mut self,
        systems: impl IntoIterator<Item = SystemId>,
        tests: &[SystemVisibility],
        gate: &Gate,
    ) -> bool {
        self.set_hidden(&hidden_by(systems, tests, gate))
    }

    /// Sets `system`'s government (`Some(None)` independent, `None` the
    /// catalog's), and says whether that changed anything.
    pub fn set_govt(&mut self, system: SystemId, govt: Option<Option<GovtId>>) -> bool {
        edit(&mut self.systems, system, |change| change.govt = govt)
    }

    /// Sets `stellar`'s government (`Some(None)` independent, `None` its
    /// record's), and says whether that changed anything.
    pub fn set_stellar_govt(&mut self, stellar: StellarId, govt: Option<Option<GovtId>>) -> bool {
        edit(&mut self.stellars, stellar, |change| change.govt = govt)
    }

    /// Sets `stellar` destroyed with the countdown `destroyed`, or not
    /// destroyed with `None`, and says whether that changed anything: a
    /// new countdown does.
    pub fn set_destroyed(&mut self, stellar: StellarId, destroyed: Option<Countdown>) -> bool {
        edit(&mut self.stellars, stellar, |change| {
            change.destroyed = destroyed;
        })
    }

    /// Sets whether `nebula` is active (`None` leaves it active, as it is
    /// with no entry), and says whether that changed anything.
    pub fn set_nebula_active(&mut self, nebula: NebulaId, active: Option<bool>) -> bool {
        let was = self.nebula(nebula);
        match active {
            Some(active) => self.nebulae.insert(nebula, active),
            None => self.nebulae.remove(&nebula),
        };
        was != active
    }

    /// The systems it hides.
    pub(crate) fn hidden(&self) -> BTreeSet<SystemId> {
        self.systems()
            .filter(|(_, change)| change.visible == Some(false))
            .map(|(id, _)| id)
            .collect()
    }
}

/// The systems of `systems` whose `Visibility` in `tests` fails for
/// `gate`: hidden by the galaxy refresh. A system with no entry in `tests`
/// counts as a blank test, which holds (@0x995ad-0x995b4), and a test that
/// did not parse never holds ([`Gate::allows`]). An entry for a system
/// outside `systems` is never read.
#[must_use]
pub fn hidden_by(
    systems: impl IntoIterator<Item = SystemId>,
    tests: &[SystemVisibility],
    gate: &Gate,
) -> BTreeSet<SystemId> {
    let tests: BTreeMap<SystemId, &SystemVisibility> =
        tests.iter().map(|test| (test.id, test)).collect();
    systems
        .into_iter()
        .filter(|system| {
            tests
                .get(system)
                .is_some_and(|test| !gate.allows(&test.visibility))
        })
        .collect()
}

/// Where a player in, or arriving in, a system goes (see
/// [`OverlaidGalaxy::arrival`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HiddenMove {
    /// The system it goes to.
    pub to: SystemId,
    /// Whether that system must be made visible first: it is the chain
    /// head of a place with no visible system.
    pub make_visible: bool,
}

/// The galaxy every reader sees: the static catalog's systems, read once,
/// with a [`GalaxyOverlay`] applied, and the [`StarMap`] built from both,
/// rebuilt only when the overlay changes. With an empty overlay its map is
/// exactly [`StarMap::new`] of the catalog.
#[derive(Clone, Debug, PartialEq)]
pub struct OverlaidGalaxy {
    /// The catalog's systems, as read.
    systems: Vec<StarSystem>,
    /// Each system's government in the catalog, the last of a repeated ID
    /// winning, as the star map takes it.
    govts: BTreeMap<SystemId, Option<GovtId>>,
    /// What play has changed.
    overlay: GalaxyOverlay,
    /// Which visible system stands for a place two share.
    colocated: CoLocated,
    /// The overlaid map.
    map: StarMap,
}

impl OverlaidGalaxy {
    /// The catalog's `systems` with `overlay` applied, a place two visible
    /// systems share resolved by the original's reading,
    /// [`CoLocated::Lowest`] ([`OverlaidGalaxy::set_colocated`]).
    #[must_use]
    pub fn new(systems: Vec<StarSystem>, overlay: GalaxyOverlay) -> Self {
        let govts = systems
            .iter()
            .map(|system| (system.id, system.govt))
            .collect();
        let colocated = CoLocated::default();
        let map = Self::build(&systems, &overlay, colocated);
        Self {
            systems,
            govts,
            overlay,
            colocated,
            map,
        }
    }

    /// The map of `systems` under `overlay`, resolved as `colocated` says.
    fn build(systems: &[StarSystem], overlay: &GalaxyOverlay, colocated: CoLocated) -> StarMap {
        let overlaid = systems
            .iter()
            .cloned()
            .map(|mut system| {
                if let Some(govt) = overlay.system(system.id).govt {
                    system.govt = govt;
                }
                system
            })
            .collect();
        StarMap::with_hidden(overlaid, &overlay.hidden(), colocated)
    }

    /// Which visible system stands for a place two share
    /// ([`RuleKey::ColocatedResolution`](crate::RuleKey::ColocatedResolution)).
    #[must_use]
    pub fn colocated(&self) -> CoLocated {
        self.colocated
    }

    /// Resolves a place two visible systems share as `colocated` says
    /// instead, rebuilding the map when it differs, and says whether it
    /// did.
    pub fn set_colocated(&mut self, colocated: CoLocated) -> bool {
        if colocated == self.colocated {
            return false;
        }
        self.map = Self::build(&self.systems, &self.overlay, colocated);
        self.colocated = colocated;
        true
    }

    /// The overlay applied.
    #[must_use]
    pub fn overlay(&self) -> &GalaxyOverlay {
        &self.overlay
    }

    /// Applies `overlay` instead, rebuilding the map when it differs, and
    /// says whether it did.
    pub fn set_overlay(&mut self, overlay: GalaxyOverlay) -> bool {
        if overlay == self.overlay {
            return false;
        }
        self.map = Self::build(&self.systems, &overlay, self.colocated);
        self.overlay = overlay;
        true
    }

    /// The overlaid map: the visible systems, their links and governments.
    #[must_use]
    pub fn map(&self) -> &StarMap {
        &self.map
    }

    /// Whether `system` is in the catalog, visible or not.
    #[must_use]
    pub fn exists(&self, system: SystemId) -> bool {
        self.map.places().contains(system)
    }

    /// `system`'s map position in the catalog, visible or not: where a
    /// ship jumping there arrives from, as the system standing for it is
    /// at the same place.
    #[must_use]
    pub fn position(&self, system: SystemId) -> Option<Vec2> {
        self.map.places().position(system)
    }

    /// Whether `system` is visible: unless the overlay hides it.
    #[must_use]
    pub fn visible(&self, system: SystemId) -> bool {
        self.map.places().visible(system)
    }

    /// `system`'s government, visible or not: the overlay's, else the
    /// catalog's; `None` when it is independent or not in the catalog.
    #[must_use]
    pub fn govt(&self, system: SystemId) -> Option<GovtId> {
        let catalog = self.govts.get(&system).copied().flatten();
        self.overlay.system(system).govt.unwrap_or(catalog)
    }

    /// Every system in the catalog, visible or not, by ascending ID: the
    /// systems the galaxy refresh evaluates.
    pub fn catalog_systems(&self) -> impl Iterator<Item = SystemId> + '_ {
        self.govts.keys().copied()
    }

    /// `system`'s Con slots as the catalog reads them, unresolved, visible
    /// or not: they may repeat, name itself or a system that does not
    /// exist, as `_DoNewPilot` reads them (@0x18f97). Empty for a system
    /// not in the catalog; the last of a repeated ID wins.
    #[must_use]
    pub fn con_slots(&self, system: SystemId) -> &[SystemId] {
        self.systems
            .iter()
            .rev()
            .find(|record| record.id == system)
            .map_or(&[], |record| &record.links)
    }

    /// Every system in the catalog, visible or not, by ascending ID, with
    /// its government ([`OverlaidGalaxy::govt`]): what the original's
    /// loops over every `_systemData` slot read (@0xa05a3-0xa0672).
    pub fn governed(&self) -> impl Iterator<Item = (SystemId, Option<GovtId>)> + '_ {
        self.govts.keys().map(|&system| (system, self.govt(system)))
    }

    /// Whether `stellar` is destroyed: whether the overlay holds its
    /// regeneration countdown. The flight view draws a stellar by this
    /// (story-runtime phase 7), and [`site`](Self::site) stamps it.
    #[must_use]
    pub fn destroyed(&self, stellar: StellarId) -> bool {
        self.overlay.stellar(stellar).destroyed.is_some()
    }

    /// `site` with the overlay's government for its stellar, if any, and
    /// destroyed exactly when the overlay holds it destroyed.
    #[must_use]
    pub fn site(&self, site: LandingSite) -> LandingSite {
        LandingSite {
            govt: self.overlay.stellar(site.id).govt.unwrap_or(site.govt),
            destroyed: self.destroyed(site.id),
            ..site
        }
    }

    /// Whether `nebula` is active: unless the overlay says otherwise.
    #[must_use]
    pub fn nebula_active(&self, nebula: NebulaId) -> bool {
        self.overlay.nebula(nebula).unwrap_or(true)
    }

    /// The visible system that stands for `system`: the lowest-ID visible
    /// system at its exact map position, if any, or by
    /// [`CoLocated::Itself`] itself when it is visible
    /// ([`Places::stand_in`](crate::hyperspace::Places::stand_in), the one
    /// resolver).
    #[must_use]
    pub fn same_place(&self, system: SystemId) -> Option<SystemId> {
        self.map.places().stand_in(system)
    }

    /// Where a player arriving in `system` goes: the visible system that
    /// stands for it ([`OverlaidGalaxy::same_place`]), else the chain head, the lowest-ID
    /// system at that place (itself when it is not in the catalog), made
    /// visible, as the original moves a player whose system is hidden
    /// (@0x998f1-0x99a36).
    #[must_use]
    pub fn arrival(&self, system: SystemId) -> HiddenMove {
        match self.same_place(system) {
            Some(to) => HiddenMove {
                to,
                make_visible: false,
            },
            None => HiddenMove {
                to: self.map.places().head(system).unwrap_or(system),
                make_visible: true,
            },
        }
    }

    /// The system that owns a stellar `listers` list (its
    /// [`ListedStellar::systems`](crate::catalog::ListedStellar::systems),
    /// by ascending ID): the first that claims it, none when none does. A
    /// system claims when it is in the catalog and visible, was not made
    /// visible by the move (its overlay entry is not `Some(true)`), and
    /// stands for itself at its place ([`OverlaidGalaxy::same_place`]).
    ///
    /// This is `_PropagateMissionBitEffects`' ownership (@0x996ed-0x997bd),
    /// derived rather than stored: it visits the systems by ascending ID,
    /// and a visible system not yet done claims each of its stellars still
    /// unowned, then marks every higher system at its place done. So by
    /// [`CoLocated::Lowest`] a visible system shadowed by a lower visible
    /// one at its place claims nothing, and by [`CoLocated::Itself`] the
    /// owner is the lowest-ID visible lister. The ownership runs before
    /// the player's move out of a hidden system (@0x998f1-0x99a36), so a
    /// chain head the move shows claims nothing; it never shadows anything
    /// either, as the move shows it only at a place with no visible
    /// system. A stellar listed only by hidden or shadowed systems has no
    /// owner.
    #[must_use]
    pub fn owner(&self, listers: &[SystemId]) -> Option<SystemId> {
        listers.iter().copied().find(|&system| {
            self.exists(system)
                && self.visible(system)
                && self.overlay.system(system).visible != Some(true)
                && self.same_place(system) == Some(system)
        })
    }

    /// The system the original's `_FindSystemFromStellar` (0x7ec1-0x7f64)
    /// gives a stellar `listers` list (by ascending ID): the first that is
    /// in the catalog and visible, else the first of them, else none.
    ///
    /// Its first loop visits the slots whose visible byte (+0x1eb) is set,
    /// a chain head the player's move showed among them, as the move sets
    /// that byte; unlike [`owner`](Self::owner), it never asks whether a
    /// system is shadowed at its place. Its second loop visits every slot,
    /// with no visible or in-use test. It is the fallback of the
    /// owner-first readers ([`system_of`](Self::system_of)).
    #[must_use]
    pub fn find_system_from_stellar(&self, listers: &[SystemId]) -> Option<SystemId> {
        listers
            .iter()
            .copied()
            .find(|&system| self.exists(system) && self.visible(system))
            .or_else(|| listers.first().copied())
    }

    /// The system that holds a stellar `listers` list, with the player in
    /// `player`: its [`owner`](Self::owner), else `player` when it lists
    /// the stellar, else none.
    ///
    /// This is `_HandleStellarSystemVisibility`'s rescue
    /// (@0x32c58-0x32cb3): a stellar with no owner (+0x14 above 0x7ff) is
    /// given to the player's system (player +0x74) when that system's
    /// `NavDefs` list it. The rescue writes the owner, so it lasts until
    /// the next galaxy refresh clears every owner
    /// (`_PropagateMissionBitEffects` @0x9968c); this gives it as the
    /// original holds it after a rebuild point.
    #[must_use]
    pub fn holder(&self, listers: &[SystemId], player: SystemId) -> Option<SystemId> {
        self.owner(listers)
            .or_else(|| listers.contains(&player).then_some(player))
    }

    /// The system a stellar `listers` list is in, with the player in
    /// `player`, as the original's owner-first readers resolve it: its
    /// [`holder`](Self::holder), else
    /// [`find_system_from_stellar`](Self::find_system_from_stellar). The
    /// hypergate's readers resolve a link so (`_DrawMap` @0xf1f8-0xf21a,
    /// `_DoSystemMap` @0x12ff2-0x13011, `_PlayerEnterHypergate`
    /// @0x638cd-0x638ff), each then by place.
    #[must_use]
    pub fn system_of(&self, listers: &[SystemId], player: SystemId) -> Option<SystemId> {
        self.holder(listers, player)
            .or_else(|| self.find_system_from_stellar(listers))
    }

    /// Whether `system`, the player's, shows a stellar `listers` list: it
    /// is visible and holds the stellar ([`holder`](Self::holder)).
    ///
    /// `_HandleStellarSystemVisibility` sets a stellar active (+0x44) when
    /// its owner's visible byte is set (@0x32cbe-0x32cdf), and its sprite
    /// pass shows each active stellar whose owner is the player's system
    /// (@0x32cef-0x32d4c, @0x32ec1-0x33229). So a stellar another visible
    /// system owns is never shown, though `system` lists it.
    #[must_use]
    pub fn shows(&self, listers: &[SystemId], system: SystemId) -> bool {
        self.visible(system) && self.holder(listers, system) == Some(system)
    }

    /// The stellars of `listed` that `system`, the player's, shows
    /// ([`shows`](Self::shows)). The original takes them in ascending ID
    /// into at most 16 sprite slots; the cap never bites, as a system lists
    /// at most 16 stellars.
    #[must_use]
    pub fn shown(&self, listed: &[ListedStellar], system: SystemId) -> BTreeSet<StellarId> {
        listed
            .iter()
            .filter(|stellar| self.shows(&stellar.systems, system))
            .map(|stellar| stellar.id)
            .collect()
    }

    /// Where a player in `system` goes when it is hidden
    /// ([`OverlaidGalaxy::arrival`]); `None` while it is visible.
    #[must_use]
    pub fn hidden_move(&self, system: SystemId) -> Option<HiddenMove> {
        (!self.visible(system)).then(|| self.arrival(system))
    }
}

/// Every system listing `stellar`, by ascending ID, from `listed` (by
/// ascending stellar ID, as [`PilotCatalog::listed_stellars`] gives it):
/// none when no system lists it.
///
/// [`PilotCatalog::listed_stellars`]: crate::catalog::PilotCatalog::listed_stellars
#[must_use]
pub fn listers_of(listed: &[ListedStellar], stellar: StellarId) -> &[SystemId] {
    listed
        .binary_search_by_key(&stellar, |listed| listed.id)
        .map_or(&[], |at| &listed[at].systems)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::OutfitId;
    use crate::control::{Bit, NovaBits, PilotFacts, Test};
    use crate::hyperspace::CoLocated;
    use crate::pilot::Gender;
    use crate::testkit::{planet, star};

    fn ids(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    #[test]
    fn con_slots_are_the_catalogs_links_unresolved_hidden_or_not() {
        let mut systems = systems();
        systems.push(star(6, (9.0, 9.0), &[6, 2, 2, 999]));
        systems.push(star(6, (9.0, 9.0), &[3, 999]));
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(3), Some(false));
        let galaxy = OverlaidGalaxy::new(systems, overlay);
        assert_eq!(galaxy.con_slots(SystemId(1)), ids(&[2]));
        assert_eq!(galaxy.con_slots(SystemId(3)), ids(&[1]), "hidden");
        assert_eq!(galaxy.con_slots(SystemId(6)), ids(&[3, 999]), "the last");
        assert_eq!(galaxy.con_slots(SystemId(7)), ids(&[]));
    }

    /// 1 at (0, 0) lists 2; 2, 3 and 5 share (50, 50), 3 governed by 140;
    /// 4 at (50, 51) is governed by 141.
    fn systems() -> Vec<StarSystem> {
        let governed = |mut system: StarSystem, govt| {
            system.govt = Some(GovtId(govt));
            system
        };
        vec![
            star(1, (0.0, 0.0), &[2]),
            star(2, (50.0, 50.0), &[]),
            governed(star(3, (50.0, 50.0), &[1]), 140),
            governed(star(4, (50.0, 51.0), &[]), 141),
            star(5, (50.0, 50.0), &[]),
        ]
    }

    /// [`systems`] with each of `hidden` hidden.
    fn hiding(hidden: &[i16]) -> OverlaidGalaxy {
        let mut overlay = GalaxyOverlay::default();
        for &id in hidden {
            overlay.set_visible(SystemId(id), Some(false));
        }
        OverlaidGalaxy::new(systems(), overlay)
    }

    /// [`hiding`], resolved as `colocated` says.
    fn resolving(hidden: &[i16], colocated: CoLocated) -> OverlaidGalaxy {
        let mut galaxy = hiding(hidden);
        galaxy.set_colocated(colocated);
        galaxy
    }

    #[test]
    fn the_setters_say_whether_they_changed_anything_and_drop_what_changes_nothing() {
        let mut overlay = GalaxyOverlay::default();
        assert!(overlay.is_empty());
        let s = SystemId(1);
        assert!(!overlay.set_visible(s, None), "already so");
        assert!(overlay.set_visible(s, Some(false)));
        assert!(!overlay.set_visible(s, Some(false)), "again");
        assert!(overlay.set_govt(s, Some(None)));
        assert!(!overlay.set_govt(s, Some(None)));
        assert_eq!(
            overlay.system(s),
            SystemOverride {
                visible: Some(false),
                govt: Some(None),
            }
        );
        assert!(overlay.set_visible(s, None));
        assert!(!overlay.is_empty(), "the government is left");
        assert!(overlay.set_govt(s, None));
        assert!(overlay.is_empty(), "dropped");
        assert_eq!(overlay, GalaxyOverlay::default());

        let p = StellarId(128);
        assert!(!overlay.set_destroyed(p, None));
        assert!(overlay.set_destroyed(p, Some(Countdown { days_left: 3 })));
        assert!(
            !overlay.set_destroyed(p, Some(Countdown { days_left: 3 })),
            "the same countdown"
        );
        assert!(
            overlay.set_destroyed(p, Some(Countdown { days_left: 0 })),
            "another countdown"
        );
        assert!(overlay.set_stellar_govt(p, Some(Some(GovtId(140)))));
        assert!(!overlay.set_stellar_govt(p, Some(Some(GovtId(140)))));
        assert!(overlay.set_stellar_govt(p, Some(Some(GovtId(141)))));
        assert_eq!(
            overlay.stellar(p),
            StellarOverride {
                govt: Some(Some(GovtId(141))),
                destroyed: Some(Countdown { days_left: 0 }),
            }
        );
        assert!(overlay.set_destroyed(p, None));
        assert!(!overlay.is_empty(), "the government is left");
        assert!(overlay.set_stellar_govt(p, None));
        assert!(overlay.is_empty());
        assert!(overlay.set_destroyed(p, Some(Countdown { days_left: 0 })));
        assert!(overlay.set_destroyed(p, None));
        assert!(overlay.is_empty(), "a destroyed state alone, dropped");

        let n = NebulaId(128);
        assert!(!overlay.set_nebula_active(n, None));
        assert!(overlay.set_nebula_active(n, Some(true)), "said explicitly");
        assert!(overlay.set_nebula_active(n, Some(false)));
        assert!(!overlay.set_nebula_active(n, Some(false)));
        assert_eq!(overlay.nebula(n), Some(false));
        assert!(overlay.set_nebula_active(n, None));
        assert!(overlay.is_empty());
    }

    #[test]
    fn the_overlay_lists_what_it_changes_by_ascending_id() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_govt(SystemId(9), Some(None));
        overlay.set_visible(SystemId(2), Some(true));
        overlay.set_destroyed(StellarId(300), Some(Countdown { days_left: 2 }));
        overlay.set_stellar_govt(StellarId(129), Some(Some(GovtId(5))));
        overlay.set_nebula_active(NebulaId(7), Some(false));
        overlay.set_nebula_active(NebulaId(3), Some(true));
        assert_eq!(
            overlay.systems().map(|(id, _)| id).collect::<Vec<_>>(),
            ids(&[2, 9])
        );
        assert_eq!(
            overlay.stellars().map(|(id, _)| id.0).collect::<Vec<_>>(),
            [129, 300]
        );
        assert_eq!(
            overlay.nebulae().collect::<Vec<_>>(),
            [(NebulaId(3), true), (NebulaId(7), false)]
        );
    }

    #[test]
    fn with_no_overlay_the_map_is_the_catalogs() {
        let galaxy = OverlaidGalaxy::new(systems(), GalaxyOverlay::default());
        assert_eq!(*galaxy.map(), StarMap::new(systems()));
        assert!(galaxy.overlay().is_empty());
        for (id, place) in [(1, 1), (2, 2), (3, 2), (4, 4), (5, 2)] {
            assert!(galaxy.visible(SystemId(id)));
            assert!(galaxy.exists(SystemId(id)));
            assert_eq!(galaxy.same_place(SystemId(id)), Some(SystemId(place)));
            assert_eq!(galaxy.hidden_move(SystemId(id)), None);
        }
        assert_eq!(galaxy.govt(SystemId(3)), Some(GovtId(140)));
        assert_eq!(galaxy.govt(SystemId(1)), None);
        assert!(!galaxy.exists(SystemId(99)));
        assert_eq!(galaxy.govt(SystemId(99)), None);
    }

    #[test]
    fn a_government_is_the_overlays_else_the_catalogs() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_govt(SystemId(1), Some(Some(GovtId(150))));
        overlay.set_govt(SystemId(3), Some(None));
        let galaxy = OverlaidGalaxy::new(systems(), overlay);
        assert_eq!(galaxy.govt(SystemId(1)), Some(GovtId(150)));
        assert_eq!(galaxy.govt(SystemId(3)), None, "made independent");
        assert_eq!(galaxy.govt(SystemId(4)), Some(GovtId(141)), "untouched");
        assert_eq!(galaxy.map().govt(SystemId(1)), Some(GovtId(150)));
        assert_eq!(galaxy.map().govt(SystemId(3)), None);
        assert_eq!(galaxy.map().govt(SystemId(4)), Some(GovtId(141)));
    }

    #[test]
    fn a_hidden_system_is_not_visible_and_off_the_map_but_still_in_the_catalog() {
        let galaxy = hiding(&[3]);
        assert!(!galaxy.visible(SystemId(3)));
        assert!(galaxy.exists(SystemId(3)));
        assert_eq!(galaxy.map().position(SystemId(3)), None);
        assert_eq!(galaxy.position(SystemId(3)), Some(Vec2::new(50.0, 50.0)));
        assert_eq!(galaxy.position(SystemId(99)), None);
        assert_eq!(galaxy.govt(SystemId(3)), Some(GovtId(140)));
        assert!(galaxy.visible(SystemId(99)), "nothing hides it");
    }

    #[test]
    fn a_stellars_government_is_the_overlays_for_that_stellar_alone() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_stellar_govt(StellarId(128), Some(Some(GovtId(140))));
        overlay.set_stellar_govt(StellarId(130), Some(None));
        let galaxy = OverlaidGalaxy::new(systems(), overlay);
        let governed = |id, govt| LandingSite {
            govt,
            ..planet(id, 0.0, 0.0)
        };
        assert_eq!(
            galaxy.site(governed(128, None)),
            governed(128, Some(GovtId(140)))
        );
        assert_eq!(
            galaxy.site(governed(129, Some(GovtId(5)))),
            governed(129, Some(GovtId(5)))
        );
        assert_eq!(
            galaxy.site(governed(130, Some(GovtId(5)))),
            governed(130, None)
        );
    }

    #[test]
    fn a_stellar_is_destroyed_as_the_overlay_says_whatever_its_site_said() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_destroyed(StellarId(128), Some(Countdown { days_left: 0 }));
        overlay.set_stellar_govt(StellarId(129), Some(None));
        let galaxy = OverlaidGalaxy::new(systems(), overlay);
        let site = |id, destroyed| LandingSite {
            destroyed,
            govt: Some(GovtId(5)),
            ..planet(id, 0.0, 0.0)
        };
        assert_eq!(galaxy.site(site(128, false)), site(128, true));
        assert_eq!(galaxy.site(site(128, true)), site(128, true));
        assert_eq!(
            galaxy.site(site(129, true)),
            LandingSite {
                govt: None,
                ..site(129, false)
            },
            "not destroyed, and independent"
        );
        assert_eq!(galaxy.site(site(130, false)), site(130, false));
    }

    #[test]
    fn a_stellar_is_destroyed_while_the_overlay_holds_its_countdown() {
        let mut galaxy = OverlaidGalaxy::new(systems(), GalaxyOverlay::default());
        assert!(!galaxy.destroyed(StellarId(128)), "no override");
        let mut overlay = GalaxyOverlay::default();
        overlay.set_destroyed(StellarId(128), Some(Countdown { days_left: 0 }));
        overlay.set_stellar_govt(StellarId(129), Some(None));
        galaxy.set_overlay(overlay.clone());
        assert!(galaxy.destroyed(StellarId(128)));
        assert!(
            !galaxy.destroyed(StellarId(129)),
            "an override with no countdown"
        );
        overlay.set_destroyed(StellarId(128), None);
        galaxy.set_overlay(overlay);
        assert!(!galaxy.destroyed(StellarId(128)), "regenerated");
    }

    #[test]
    fn a_nebula_is_active_unless_the_overlay_says_otherwise() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_nebula_active(NebulaId(129), Some(false));
        overlay.set_nebula_active(NebulaId(130), Some(true));
        let galaxy = OverlaidGalaxy::new(systems(), overlay);
        assert!(galaxy.nebula_active(NebulaId(128)));
        assert!(!galaxy.nebula_active(NebulaId(129)));
        assert!(galaxy.nebula_active(NebulaId(130)));
    }

    #[test]
    fn the_same_place_is_itself_when_visible_else_the_lowest_visible_one_there() {
        let itself = |hidden: &[i16]| resolving(hidden, CoLocated::Itself);
        let galaxy = itself(&[2]);
        assert_eq!(galaxy.same_place(SystemId(2)), Some(SystemId(3)));
        assert_eq!(galaxy.same_place(SystemId(5)), Some(SystemId(5)), "visible");
        let galaxy = itself(&[2, 3]);
        assert_eq!(galaxy.same_place(SystemId(2)), Some(SystemId(5)));
        assert_eq!(galaxy.same_place(SystemId(3)), Some(SystemId(5)));
        let galaxy = itself(&[2, 3, 5]);
        assert_eq!(galaxy.same_place(SystemId(2)), None, "none there");
        let galaxy = itself(&[4]);
        assert_eq!(galaxy.same_place(SystemId(4)), None, "alone at its place");
        let galaxy = itself(&[99]);
        assert_eq!(galaxy.same_place(SystemId(99)), None, "nowhere");
    }

    #[test]
    fn by_the_engine_the_same_place_is_the_lowest_visible_one_there_visible_or_not() {
        let galaxy = hiding(&[2]);
        assert_eq!(galaxy.same_place(SystemId(2)), Some(SystemId(3)));
        assert_eq!(
            galaxy.same_place(SystemId(5)),
            Some(SystemId(3)),
            "shadowed"
        );
        assert_eq!(galaxy.same_place(SystemId(3)), Some(SystemId(3)));
        let galaxy = hiding(&[2, 3]);
        assert_eq!(galaxy.same_place(SystemId(2)), Some(SystemId(5)));
        assert_eq!(galaxy.same_place(SystemId(3)), Some(SystemId(5)));
        let galaxy = hiding(&[2, 3, 5]);
        assert_eq!(galaxy.same_place(SystemId(2)), None, "none there");
        let galaxy = hiding(&[4]);
        assert_eq!(galaxy.same_place(SystemId(4)), None, "alone at its place");
        let galaxy = hiding(&[99]);
        assert_eq!(galaxy.same_place(SystemId(99)), None, "nowhere");
    }

    #[test]
    fn an_arrival_is_itself_the_visible_one_there_or_the_chain_head_made_visible() {
        let stay = |to| HiddenMove {
            to: SystemId(to),
            make_visible: false,
        };
        let shown = |to| HiddenMove {
            to: SystemId(to),
            make_visible: true,
        };
        let itself = |hidden: &[i16]| resolving(hidden, CoLocated::Itself);
        assert_eq!(itself(&[]).arrival(SystemId(3)), stay(3));
        assert_eq!(itself(&[3]).arrival(SystemId(3)), stay(2));
        assert_eq!(itself(&[2, 3]).arrival(SystemId(3)), stay(5));
        assert_eq!(
            itself(&[2, 3, 5]).arrival(SystemId(3)),
            shown(2),
            "the head"
        );
        assert_eq!(itself(&[2, 3, 5]).arrival(SystemId(2)), shown(2), "itself");
        assert_eq!(itself(&[4]).arrival(SystemId(4)), shown(4), "alone");
        assert_eq!(itself(&[99]).arrival(SystemId(99)), shown(99), "nowhere");
    }

    #[test]
    fn by_the_engine_an_arrival_is_the_lowest_visible_one_there_or_the_chain_head_made_visible() {
        let stay = |to| HiddenMove {
            to: SystemId(to),
            make_visible: false,
        };
        let shown = |to| HiddenMove {
            to: SystemId(to),
            make_visible: true,
        };
        assert_eq!(hiding(&[]).arrival(SystemId(3)), stay(2), "shadowed");
        assert_eq!(hiding(&[]).arrival(SystemId(2)), stay(2));
        assert_eq!(hiding(&[2]).arrival(SystemId(5)), stay(3));
        assert_eq!(hiding(&[3]).arrival(SystemId(3)), stay(2));
        assert_eq!(hiding(&[2, 3]).arrival(SystemId(3)), stay(5));
        assert_eq!(hiding(&[2, 3, 5]).arrival(SystemId(3)), shown(2));
        assert_eq!(hiding(&[4]).arrival(SystemId(4)), shown(4), "alone");
        assert_eq!(hiding(&[99]).arrival(SystemId(99)), shown(99), "nowhere");
    }

    #[test]
    fn a_visible_system_shadowed_at_its_place_never_moves_its_player() {
        for colocated in [CoLocated::Lowest, CoLocated::Itself] {
            let galaxy = resolving(&[], colocated);
            assert_eq!(galaxy.hidden_move(SystemId(5)), None, "{colocated:?}");
            assert_eq!(galaxy.hidden_move(SystemId(3)), None, "{colocated:?}");
        }
    }

    #[test]
    fn a_new_resolution_rebuilds_the_map_only_when_it_differs() {
        let mut galaxy = hiding(&[]);
        assert_eq!(galaxy.colocated(), CoLocated::Lowest, "the default");
        assert!(!galaxy.set_colocated(CoLocated::Lowest));
        assert_eq!(*galaxy.map(), StarMap::new(systems()));
        assert!(galaxy.set_colocated(CoLocated::Itself));
        assert_eq!(galaxy.colocated(), CoLocated::Itself);
        assert_eq!(
            *galaxy.map(),
            StarMap::with_hidden(systems(), &BTreeSet::new(), CoLocated::Itself)
        );
        assert_eq!(galaxy.same_place(SystemId(5)), Some(SystemId(5)));
        assert!(!galaxy.set_colocated(CoLocated::Itself));
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(2), Some(false));
        galaxy.set_overlay(overlay);
        assert_eq!(
            galaxy.same_place(SystemId(5)),
            Some(SystemId(5)),
            "a new overlay keeps the resolution"
        );
        assert!(galaxy.set_colocated(CoLocated::Lowest));
        assert_eq!(galaxy.same_place(SystemId(5)), Some(SystemId(3)));
    }

    #[test]
    fn only_a_hidden_system_moves_its_player() {
        assert_eq!(hiding(&[3]).hidden_move(SystemId(2)), None, "visible");
        assert_eq!(
            hiding(&[3]).hidden_move(SystemId(3)),
            Some(HiddenMove {
                to: SystemId(2),
                make_visible: false,
            }),
            "to a visible alternate"
        );
        assert_eq!(
            hiding(&[2, 3, 5]).hidden_move(SystemId(2)),
            Some(HiddenMove {
                to: SystemId(2),
                make_visible: true,
            }),
            "none visible, and its own system is the head"
        );
        assert_eq!(
            hiding(&[2, 3, 5]).hidden_move(SystemId(5)),
            Some(HiddenMove {
                to: SystemId(2),
                make_visible: true,
            }),
            "none visible, to the lower head"
        );
    }

    #[test]
    fn a_new_overlay_rebuilds_the_map_only_when_it_differs() {
        let mut galaxy = hiding(&[]);
        assert!(!galaxy.set_overlay(GalaxyOverlay::default()));
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(2), Some(false));
        overlay.set_govt(SystemId(1), Some(Some(GovtId(7))));
        assert!(galaxy.set_overlay(overlay.clone()));
        assert_eq!(*galaxy.overlay(), overlay);
        assert_eq!(galaxy.map(), hiding_and_governing().map());
        assert_eq!(galaxy.map().listed_links(SystemId(1)), ids(&[3]));
        assert!(!galaxy.set_overlay(overlay));
        assert!(galaxy.set_overlay(GalaxyOverlay::default()));
        assert_eq!(*galaxy.map(), StarMap::new(systems()), "back");
    }

    #[test]
    fn the_catalog_systems_are_every_one_hidden_or_not_by_ascending_id() {
        let galaxy = hiding(&[3]);
        assert_eq!(
            galaxy.catalog_systems().collect::<Vec<_>>(),
            ids(&[1, 2, 3, 4, 5])
        );
    }

    #[test]
    fn every_system_is_governed_as_overlaid_hidden_or_not_by_ascending_id() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(3), Some(false));
        overlay.set_govt(SystemId(1), Some(Some(GovtId(7))));
        overlay.set_govt(SystemId(4), Some(None));
        let galaxy = OverlaidGalaxy::new(systems(), overlay);
        assert_eq!(
            galaxy.governed().collect::<Vec<_>>(),
            [
                (SystemId(1), Some(GovtId(7))),
                (SystemId(2), None),
                (SystemId(3), Some(GovtId(140))),
                (SystemId(4), None),
                (SystemId(5), None),
            ],
            "1 by its override, 3 though hidden, 4 made independent"
        );
    }

    /// The owner `galaxy` gives a stellar listed by `listers`.
    fn owner(galaxy: &OverlaidGalaxy, listers: &[i16]) -> Option<i16> {
        galaxy.owner(&ids(listers)).map(|id| id.0)
    }

    #[test]
    fn a_stellar_is_owned_by_the_lowest_visible_lister_standing_for_its_place() {
        let galaxy = hiding(&[]);
        assert_eq!(owner(&galaxy, &[1, 2]), Some(1));
        assert_eq!(owner(&galaxy, &[2, 3]), Some(2), "the lowest");
        assert_eq!(owner(&galaxy, &[3]), None, "3 is shadowed by 2");
        assert_eq!(owner(&galaxy, &[3, 5]), None, "both shadowed");
        assert_eq!(
            owner(&galaxy, &[3, 4]),
            Some(4),
            "a later visible system elsewhere still claims it"
        );
        assert_eq!(owner(&galaxy, &[]), None, "no lister");
        let galaxy = hiding(&[2]);
        assert_eq!(owner(&galaxy, &[2]), None, "only a hidden lister");
        assert_eq!(owner(&galaxy, &[2, 3]), Some(3));
        assert_eq!(owner(&galaxy, &[5]), None, "3 shadows 5");
        let galaxy = hiding(&[2, 3]);
        assert_eq!(owner(&galaxy, &[5]), Some(5), "the lowest visible now");
        assert_eq!(owner(&galaxy, &[99]), None, "not in the catalog");
    }

    #[test]
    fn by_the_other_reading_each_visible_lister_claims_its_own() {
        let galaxy = resolving(&[], CoLocated::Itself);
        assert_eq!(owner(&galaxy, &[3]), Some(3));
        assert_eq!(owner(&galaxy, &[3, 5]), Some(3), "the lowest visible");
        assert_eq!(owner(&galaxy, &[2, 3]), Some(2));
        assert_eq!(owner(&galaxy, &[5]), Some(5));
        let galaxy = resolving(&[3], CoLocated::Itself);
        assert_eq!(owner(&galaxy, &[3]), None, "only a hidden lister");
        assert_eq!(owner(&galaxy, &[3, 5]), Some(5));
    }

    #[test]
    fn a_chain_head_made_visible_by_the_move_claims_nothing() {
        for colocated in [CoLocated::Lowest, CoLocated::Itself] {
            let mut overlay = GalaxyOverlay::default();
            overlay.set_visible(SystemId(4), Some(true));
            let mut galaxy = OverlaidGalaxy::new(systems(), overlay);
            galaxy.set_colocated(colocated);
            assert!(galaxy.visible(SystemId(4)));
            assert_eq!(owner(&galaxy, &[4]), None, "{colocated:?}");
            assert_eq!(owner(&galaxy, &[1, 4]), Some(1), "{colocated:?}");
        }
    }

    /// The system `galaxy`'s `_FindSystemFromStellar` gives a stellar
    /// listed by `listers`.
    fn found(galaxy: &OverlaidGalaxy, listers: &[i16]) -> Option<i16> {
        galaxy
            .find_system_from_stellar(&ids(listers))
            .map(|id| id.0)
    }

    #[test]
    fn find_system_from_stellar_takes_the_lowest_visible_lister() {
        let galaxy = hiding(&[2]);
        assert_eq!(found(&galaxy, &[2, 4]), Some(4), "2 is hidden");
        assert_eq!(found(&galaxy, &[1, 4]), Some(1), "the lowest");
        assert_eq!(
            found(&galaxy, &[99, 4]),
            Some(4),
            "a system outside the catalog is never visible"
        );
    }

    #[test]
    fn find_system_from_stellar_counts_a_visible_lister_shadowed_at_its_place() {
        let galaxy = hiding(&[]);
        assert_eq!(owner(&galaxy, &[3]), None, "2 shadows 3");
        assert_eq!(found(&galaxy, &[3]), Some(3));
        assert_eq!(found(&galaxy, &[3, 4]), Some(3), "the lowest visible");
    }

    #[test]
    fn find_system_from_stellar_falls_back_to_the_lowest_lister() {
        let galaxy = hiding(&[2, 3]);
        assert_eq!(found(&galaxy, &[2, 3]), Some(2), "both hidden");
        assert_eq!(found(&galaxy, &[99]), Some(99), "with no in-use test");
        assert_eq!(found(&galaxy, &[]), None);
    }

    #[test]
    fn find_system_from_stellar_counts_a_chain_head_the_move_showed() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(4), Some(true));
        overlay.set_visible(SystemId(2), Some(false));
        let galaxy = OverlaidGalaxy::new(systems(), overlay);
        assert_eq!(owner(&galaxy, &[2, 4]), None);
        assert_eq!(found(&galaxy, &[2, 4]), Some(4));
    }

    /// The system `galaxy` holds a stellar listed by `listers` for, with
    /// the player in `player`.
    fn holder(galaxy: &OverlaidGalaxy, listers: &[i16], player: i16) -> Option<i16> {
        galaxy
            .holder(&ids(listers), SystemId(player))
            .map(|id| id.0)
    }

    #[test]
    fn holder_is_the_owner_else_the_players_system_when_it_lists_the_stellar() {
        let galaxy = hiding(&[2]);
        assert_eq!(holder(&galaxy, &[1, 4], 4), Some(1), "the owner wins");
        assert_eq!(holder(&galaxy, &[2, 5], 5), Some(5), "3 shadows 5: rescued");
        assert_eq!(holder(&galaxy, &[2, 5], 4), None, "4 does not list it");
        assert_eq!(holder(&galaxy, &[], 4), None);
    }

    #[test]
    fn system_of_is_the_holder_else_find_system_from_stellar() {
        let galaxy = hiding(&[2]);
        let system_of = |listers: &[i16], player| {
            galaxy
                .system_of(&ids(listers), SystemId(player))
                .map(|id| id.0)
        };
        assert_eq!(system_of(&[1, 4], 4), Some(1), "the owner");
        assert_eq!(system_of(&[2, 5], 5), Some(5), "the rescue");
        assert_eq!(system_of(&[2, 5], 4), Some(5), "the lowest visible lister");
        assert_eq!(system_of(&[2], 4), Some(2), "the lowest lister");
        assert_eq!(system_of(&[], 4), None);

        let galaxy = hiding(&[]);
        let system_of = |listers: &[i16], player| {
            galaxy
                .system_of(&ids(listers), SystemId(player))
                .map(|id| id.0)
        };
        assert_eq!(
            system_of(&[3, 4], 1),
            Some(4),
            "the owner, not 3, visible but shadowed by 2, as _FindSystemFromStellar takes"
        );
        assert_eq!(
            system_of(&[3, 5], 5),
            Some(5),
            "both shadowed by 2: rescued into 5, not the lowest visible lister 3"
        );
    }

    #[test]
    fn shows_owned_and_rescued_stellars_and_nothing_owned_elsewhere() {
        let galaxy = hiding(&[]);
        let shows = |listers: &[i16], system| galaxy.shows(&ids(listers), SystemId(system));
        assert!(shows(&[1], 1));
        assert!(!shows(&[2, 3], 3), "2 owns it");
        assert!(shows(&[2, 3], 2));
        assert!(
            shows(&[3], 3),
            "shadowed by 2, and no one owns it: the rescue"
        );
        assert!(!shows(&[3], 2), "2 does not list it");
        assert!(!shows(&[], 1));
        let galaxy = hiding(&[1]);
        assert!(!galaxy.shows(&ids(&[1]), SystemId(1)), "1 is hidden");
    }

    #[test]
    fn shown_collects_the_stellars_a_system_shows() {
        let galaxy = hiding(&[]);
        let listed = |id, systems: &[i16]| ListedStellar {
            id: StellarId(id),
            systems: ids(systems),
            flags: 0,
            flags2: 0,
            govt: None,
        };
        let listed = [
            listed(128, &[1]),
            listed(129, &[2, 3]),
            listed(130, &[3]),
            listed(131, &[3, 4]),
        ];
        let shown = |system| {
            galaxy
                .shown(&listed, SystemId(system))
                .into_iter()
                .map(|id| id.0)
                .collect::<Vec<_>>()
        };
        assert_eq!(shown(3), [130], "129 is 2's and 131 is 4's");
        assert_eq!(shown(2), [129]);
        assert_eq!(shown(4), [131]);
        assert_eq!(shown(1), [128]);
        assert_eq!(shown(5), [] as [i16; 0]);
    }

    #[test]
    fn the_listers_of_a_stellar_are_its_listings_and_none_when_unlisted() {
        let listed = |id, systems: &[i16]| ListedStellar {
            id: StellarId(id),
            systems: ids(systems),
            flags: 0,
            flags2: 0,
            govt: None,
        };
        let listed = [listed(128, &[1]), listed(130, &[2, 3]), listed(140, &[4])];
        assert_eq!(listers_of(&listed, StellarId(130)), ids(&[2, 3]));
        assert_eq!(listers_of(&listed, StellarId(128)), ids(&[1]));
        assert_eq!(listers_of(&listed, StellarId(140)), ids(&[4]));
        assert_eq!(listers_of(&listed, StellarId(129)), [] as [SystemId; 0]);
        assert_eq!(listers_of(&[], StellarId(129)), [] as [SystemId; 0]);
    }

    /// A pilot with only `bits` set.
    #[derive(Debug, Default)]
    struct Bits(BTreeSet<u16>);

    impl PilotFacts for Bits {
        fn bit(&self, bit: Bit) -> bool {
            self.0.contains(&bit.get())
        }

        fn gender(&self) -> Gender {
            Gender::Male
        }

        fn paid(&self, _days: u16) -> bool {
            true
        }

        fn has_outfit(&self, _outfit: OutfitId) -> bool {
            false
        }

        fn explored(&self, _system: SystemId) -> bool {
            false
        }
    }

    /// `system`'s `Visibility`, `test`.
    fn visibility(system: i16, test: &str) -> SystemVisibility {
        SystemVisibility {
            id: SystemId(system),
            visibility: Test::parse(test),
        }
    }

    /// Refreshes `overlay` over systems 1 to 5 by `tests` for a pilot with
    /// `bits` set, tested by Nova's control bits: whether it changed.
    fn refresh(overlay: &mut GalaxyOverlay, tests: &[SystemVisibility], bits: &[u16]) -> bool {
        let pilot = Bits(bits.iter().copied().collect());
        let gate = Gate {
            control_bits: &NovaBits,
            pilot: &pilot,
        };
        overlay.refresh_visibility(ids(&[1, 2, 3, 4, 5]), tests, &gate)
    }

    #[test]
    fn a_refresh_hides_a_system_whose_visibility_fails_and_shows_it_once_it_holds() {
        let tests = [visibility(2, "b10"), visibility(3, "!b10")];
        let mut overlay = GalaxyOverlay::default();
        assert!(refresh(&mut overlay, &tests, &[]));
        assert_eq!(overlay.system(SystemId(2)).visible, Some(false));
        assert_eq!(overlay.system(SystemId(3)).visible, None);
        assert!(!refresh(&mut overlay, &tests, &[]), "nothing changed");
        assert!(refresh(&mut overlay, &tests, &[10]));
        assert_eq!(overlay.system(SystemId(2)).visible, None, "shown again");
        assert_eq!(overlay.system(SystemId(3)).visible, Some(false));
    }

    #[test]
    fn a_blank_test_or_no_test_leaves_a_system_visible_and_an_unparsed_one_hides_it() {
        let tests = [visibility(1, ""), visibility(4, "b10 &")];
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(1), Some(false));
        overlay.set_visible(SystemId(5), Some(false));
        assert!(refresh(&mut overlay, &tests, &[10]));
        assert_eq!(overlay.system(SystemId(1)).visible, None, "blank");
        assert_eq!(overlay.system(SystemId(5)).visible, None, "no test");
        assert_eq!(overlay.system(SystemId(4)).visible, Some(false), "unparsed");
    }

    #[test]
    fn a_refresh_never_writes_a_system_shown_and_overwrites_the_moves_reveal() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(2), Some(true));
        overlay.set_visible(SystemId(3), Some(true));
        overlay.set_govt(SystemId(3), Some(None));
        assert!(refresh(&mut overlay, &[visibility(3, "b10")], &[]));
        assert_eq!(overlay.system(SystemId(2)).visible, None, "visible");
        assert_eq!(overlay.system(SystemId(3)).visible, Some(false), "hidden");
        assert_eq!(overlay.system(SystemId(3)).govt, Some(None), "kept");
        assert!(
            overlay
                .systems()
                .all(|(_, change)| change.visible != Some(true))
        );
    }

    #[test]
    fn a_refresh_clears_what_it_held_of_a_system_outside_the_catalog() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(99), Some(false));
        overlay.set_visible(SystemId(98), Some(true));
        overlay.set_nebula_active(NebulaId(128), Some(false));
        assert!(refresh(&mut overlay, &[visibility(99, "b10")], &[]));
        assert_eq!(overlay.system(SystemId(99)), SystemOverride::default());
        assert_eq!(overlay.system(SystemId(98)), SystemOverride::default());
        assert_eq!(overlay.nebula(NebulaId(128)), Some(false), "untouched");
    }

    #[test]
    fn the_systems_hidden_are_those_whose_test_fails_for_the_pilot() {
        let pilot = Bits([10].into_iter().collect());
        let gate = Gate {
            control_bits: &NovaBits,
            pilot: &pilot,
        };
        let tests = [
            visibility(1, "!b10"),
            visibility(2, "b10"),
            visibility(4, "b11"),
            visibility(99, "b11"),
        ];
        assert_eq!(
            hidden_by(ids(&[1, 2, 3, 4, 5]), &tests, &gate),
            ids(&[1, 4]).into_iter().collect::<BTreeSet<_>>(),
            "99 is outside the catalog"
        );
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(2), Some(false));
        assert!(overlay.set_hidden(&ids(&[1, 4]).into_iter().collect()));
        assert_eq!(overlay.hidden(), ids(&[1, 4]).into_iter().collect());
        assert!(!overlay.set_hidden(&ids(&[1, 4]).into_iter().collect()));
    }

    /// [`systems`] with 2 hidden and 1 governed by 7.
    fn hiding_and_governing() -> OverlaidGalaxy {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(2), Some(false));
        overlay.set_govt(SystemId(1), Some(Some(GovtId(7))));
        OverlaidGalaxy::new(systems(), overlay)
    }
}
