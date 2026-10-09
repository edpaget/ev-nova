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
//! - A system at the same place as a hidden one stands in for it
//!   ([`OverlaidGalaxy::same_place`]): the lowest-ID visible system at its
//!   exact map position, the engine's reading of the original's
//!   `_FindActiveCoLocatedSystem` (@0x4be0). A player in, or arriving in,
//!   a hidden system goes there or, with none, to the chain head made
//!   visible ([`OverlaidGalaxy::arrival`], @0x998f1-0x99a36).

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{GovtId, LandingSite, NebulaId, StarSystem, StellarId, SystemId};
use crate::geometry::Vec2;
use crate::hyperspace::StarMap;

/// What the overlay changes of one system. The default changes nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SystemOverride {
    /// Whether it is visible; `None` leaves it visible, as the catalog has
    /// it.
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
    /// has it), and says whether that changed anything.
    pub fn set_visible(&mut self, system: SystemId, visible: Option<bool>) -> bool {
        edit(&mut self.systems, system, |change| change.visible = visible)
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
    fn hidden(&self) -> BTreeSet<SystemId> {
        self.systems()
            .filter(|(_, change)| change.visible == Some(false))
            .map(|(id, _)| id)
            .collect()
    }
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
    /// The overlaid map.
    map: StarMap,
}

impl OverlaidGalaxy {
    /// The catalog's `systems` with `overlay` applied.
    #[must_use]
    pub fn new(systems: Vec<StarSystem>, overlay: GalaxyOverlay) -> Self {
        let govts = systems
            .iter()
            .map(|system| (system.id, system.govt))
            .collect();
        let map = Self::build(&systems, &overlay);
        Self {
            systems,
            govts,
            overlay,
            map,
        }
    }

    /// The map of `systems` under `overlay`.
    fn build(systems: &[StarSystem], overlay: &GalaxyOverlay) -> StarMap {
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
        StarMap::with_hidden(overlaid, &overlay.hidden())
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
        self.map = Self::build(&self.systems, &overlay);
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

    /// `site` with the overlay's government for its stellar, if any, and
    /// destroyed exactly when the overlay holds it destroyed.
    #[must_use]
    pub fn site(&self, site: LandingSite) -> LandingSite {
        let change = self.overlay.stellar(site.id);
        LandingSite {
            govt: change.govt.unwrap_or(site.govt),
            destroyed: change.destroyed.is_some(),
            ..site
        }
    }

    /// Whether `nebula` is active: unless the overlay says otherwise.
    #[must_use]
    pub fn nebula_active(&self, nebula: NebulaId) -> bool {
        self.overlay.nebula(nebula).unwrap_or(true)
    }

    /// The visible system that stands for `system`: itself when it is
    /// visible, else the lowest-ID visible system at its exact map
    /// position, if any
    /// ([`Places::stand_in`](crate::hyperspace::Places::stand_in), the one
    /// resolver).
    #[must_use]
    pub fn same_place(&self, system: SystemId) -> Option<SystemId> {
        self.map.places().stand_in(system)
    }

    /// Where a player arriving in `system` goes: itself when it is
    /// visible, else the visible system at the same place
    /// ([`OverlaidGalaxy::same_place`]), else the chain head, the lowest-ID
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

    /// Where a player in `system` goes when it is hidden
    /// ([`OverlaidGalaxy::arrival`]); `None` while it is visible.
    #[must_use]
    pub fn hidden_move(&self, system: SystemId) -> Option<HiddenMove> {
        (!self.visible(system)).then(|| self.arrival(system))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{planet, star};

    fn ids(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
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
        for id in 1..=5 {
            assert!(galaxy.visible(SystemId(id)));
            assert!(galaxy.exists(SystemId(id)));
            assert_eq!(galaxy.same_place(SystemId(id)), Some(SystemId(id)));
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
        let galaxy = hiding(&[2]);
        assert_eq!(galaxy.same_place(SystemId(2)), Some(SystemId(3)));
        assert_eq!(galaxy.same_place(SystemId(5)), Some(SystemId(5)), "visible");
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
        assert_eq!(hiding(&[]).arrival(SystemId(3)), stay(3));
        assert_eq!(hiding(&[3]).arrival(SystemId(3)), stay(2));
        assert_eq!(hiding(&[2, 3]).arrival(SystemId(3)), stay(5));
        assert_eq!(
            hiding(&[2, 3, 5]).arrival(SystemId(3)),
            shown(2),
            "the head"
        );
        assert_eq!(hiding(&[2, 3, 5]).arrival(SystemId(2)), shown(2), "itself");
        assert_eq!(hiding(&[4]).arrival(SystemId(4)), shown(4), "alone");
        assert_eq!(hiding(&[99]).arrival(SystemId(99)), shown(99), "nowhere");
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

    /// [`systems`] with 2 hidden and 1 governed by 7.
    fn hiding_and_governing() -> OverlaidGalaxy {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(2), Some(false));
        overlay.set_govt(SystemId(1), Some(Some(GovtId(7))));
        OverlaidGalaxy::new(systems(), overlay)
    }
}
