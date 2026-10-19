//! The escape pod's rescue (`_HandlePlayer` 0x68de3-0x6923c), on the
//! tick the pod's countdown reaches 0 (see the `eject` module's
//! `pod_tick`). [`Session::rescue_if_due`] settles all of it within that
//! tick, before the next frame is drawn and before its dësc shows, in the
//! original's order:
//!
//! 1. **Missions.** Every active mission ends as `_AbortMission(slot, 1)`
//!    ends it (@0x68efa-0x68f1f): its `OnAbort` runs and its special
//!    ships are released, with no `CanAbort` refusal and no record
//!    penalty, which only the Mission Info dialog's abort has.
//! 2. **`_ResetPlayer(0)`** (@0x68f2f; its findings are in the phase
//!    body). The ship becomes `shïp` 128, keeping only its persistent
//!    outfits (`Flags` 0x0004, @0x1d6dd-0x1d703), with the class's stock
//!    weapons and ammunition (@0x68fe0-0x690ab) and none of its default
//!    items, which the rescue never adds. The condition is intact and the
//!    pod gone; the targets, the secondary weapon (+0x72), the system
//!    jumped from (+0x94) and the course go; the cargo is emptied, the
//!    commodities (+0x7a) and every `jünk` (`_junkData` +0x22); the paint
//!    is cleared (`_playerPaint`); the Red Alert forgets its threat and
//!    its icon (`_playerThreatened`, `_oldPlayerThreatened`,
//!    `_alertSpritePending`); the rank last activated is forgotten
//!    (`_recentActivatedRank`, @0x1db44); the self-destruct countdown
//!    idles (`_destructCount` = -1.0, @0x1d764; see the `self_destruct`
//!    module); the system died in has its
//!    reinforcement timer cancelled and its countdown cleared
//!    (@0x1db17-0x1db30; see the `reinforcements` module); and its tail
//!    call into `_ResetPlayerPrecalcedValues` runs
//!    (`Session::reset_precalced`: the latches, the sabotage clock, the
//!    reinforcement inhibitors and the blessing table; the cargo is
//!    already empty, so its trim never cuts).
//!    The credits, the records, the exploration, the persons and the
//!    escort orders are kept: argument 0 skips their block (@0x1d97e).
//! 3. **`OnPurchase`** of `shïp` 128 (@0x68f34-0x68f55).
//! 4. **Relocation.** The system the pod is taken to (below), entered as
//!    the rescue's way in: at (50, 50) at rest, heading 0, not landed,
//!    the system explored and set up afresh, the scene and the offers'
//!    stellar let go. It raises no enter-system event: the rescue calls
//!    no `_MissionHandlePlayerEnteredNewSystem`.
//! 5. **Reserves.** Fuel and shield full, and the armour as
//!    [`RuleKey::RescueArmor`] reads (@0x690db-0x69100). Nothing takes an
//!    armour above its most away.
//! 6. **Days.** `Rand(30) + 15` days pass through the daily step
//!    (@0x69119-0x69133). No wage is paid: `_DoEscortPayment` is called
//!    only at a jump's arrival and from the bar (@0x6c620, @0x4085a), and
//!    the pod released every escort anyway.
//! 7. **The stellars** shown are rebuilt (`_HandleStellarSystemVisibility`
//!    @0x69135).
//! 8. **The name** becomes `shïp` 128's name, a space and four digits each
//!    1 to 9 (@0x6913a-0x691c6): four, where the shipyard's default has
//!    three.
//! 9. **Legal records.** Every system's record, hidden ones included, is
//!    its government's `InitialRec`, and none when independent
//!    (@0x69257-0x69290); so is the record with each government, as every
//!    writer of the one changes the other.
//! 10. **The Strict Play save** (@0x69292-0x692e3), only with Strict Play
//!     set: the pilot resumes at the first stellar of the system it ends
//!     up in (+0x74), walking its nav slots in order (@0x692b5-0x692da),
//!     or at stellar 128 when it has none (the original's 0, @0x692dc),
//!     and a save is due, as `_SavePilotFile` (@0x692c4) saves before
//!     `_FadeScreenIn` (@0x692c9). The session stays unlanded. Without
//!     Strict Play the rescue makes no save due: the missions ended, the
//!     story's events and the days inside it make none. On load the
//!     original also moves the pilot to its saved stellar's own system
//!     (@0x7525b-0x75291), which
//!     rdm:phase/player-death/phase-13-resume-at-saved-stellar-system
//!     owns.
//! 11. **The dësc**, 13999 ([`RESCUE_DESC`]), is due once
//!     ([`Session::take_rescue_desc`]); the original shows it before the
//!     missions end (@0x68e80-0x68eac), over a black screen, which comes to
//!     the same frame here.
//!
//! Left out, with no counterpart in the engine: the fades, paints and
//! cursor calls (@0x68de3-0x68e7b, @0x68eb1-0x68eef, @0x691cb-0x69253),
//! `_RandomizeStars` (@0x690d0; the starfield is drawn the same each
//! time), and of `_ResetPlayer`'s writes the price flux
//! (`_globalPriceFlux`, `_localPriceFlux`), the bribe roll
//! (`_bribeRandom`), the advice roll (`_adviceRandom`), the docking globals
//! (`_dockTarget`, `_dockPermission`: the engine's clearance is its nav
//! target, which the entry lets go), `_antiEscortDelay`,
//! `_greenMissionSyst`, `_newbieFlag`, `_commCount`, `_fadePending`,
//! the scan flags (`_weapLawFlag`, `_smuggleFlag`), the explosion and
//! smoke tables (the view's effects, which a system change clears) and
//! the ship and outfit shop rolls (+0xa2a/+0xa2c, +0x1e), which the days
//! that follow draw afresh ([`RuleKey::DayRolls`]).
//!
//! **Where the pod is taken** ([`Session::escape_destination`], the
//! session's side of [`escape_stellar`]).
//!
//! **The search's galaxy** ([`SearchMap`], the [`EscapeMap`] port). A
//! system's links are its raw `Con1-Con16` from the catalog's star map,
//! not the resolved map's. A system is open while it is in the galaxy,
//! visible ([`OverlaidGalaxy::visible`](crate::story::OverlaidGalaxy::visible)),
//! known (+0x1ec, @0xd166; [`Session::is_known`]) and explored
//! ([`Pilot::has_explored`](crate::Pilot::has_explored)). A stellar is active (+0x44)
//! while a visible system owns it
//! ([`OverlaidGalaxy::owner`](crate::story::OverlaidGalaxy::owner)), as
//! `_HandleStellarSystemVisibility` sets it from its owner's visibility
//! (@0x32c42-0x32cdf), and is read with the overlay's destruction; the
//! rest of the test is [`qualifies`], against the player's record in the
//! system searched. The system the pod goes to is the found stellar's
//! owner (+0x14, @0x68f87-0x68f9e), or the system it was found in when no
//! system owns it.

use std::collections::BTreeMap;

use super::Session;
use super::entry::{Arrival, Entry, Placement, ShipPlacement};
use super::hooks::ShipHook;
use crate::catalog::{PilotCatalog, ShipId, StellarId, SystemId, TrafficCatalog};
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::escape::{EscapeMap, RESCUE_DESC, escape_stellar, qualifies};
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::red_alert::RedAlert;
use crate::rulebook::{RuleKey, RuleSource};

/// The system the rescue takes the pod to when the search finds no
/// stellar: `sÿst` 128, the original's system 0 (@0x68f7c-0x68f7f).
pub const RESCUE_FALLBACK: SystemId = SystemId(128);

/// The stellar a Strict Play rescue saves when the rescue system has no
/// stellars: the original's 0 (@0x692dc), a 0-based stellar index, so
/// `spöb` 128.
pub const STRICT_SAVE_FALLBACK: StellarId = StellarId(128);

/// The ship the rescued player flies: `shïp` 128, class index 0, which
/// `_ResetPlayer` gives (@0x1d43d).
pub const RESCUE_SHIP: ShipId = ShipId(128);

/// Where the rescued ship is placed, at rest (`_ResetPlayer`
/// @0x1d427-0x1d42f).
pub const RESCUE_AT: Vec2 = Vec2::new(50.0, 50.0);

/// The fewest days the rescue passes; `Rand(30)` more pass on top
/// (@0x69119-0x69133).
pub const RESCUE_DAYS: u32 = 15;

/// How many more days the rescue may pass: `Rand(30)`.
pub const RESCUE_EXTRA_DAYS: u16 = 30;

/// The galaxy as the escape search reads it (see the module docs).
pub(super) struct SearchMap<'a, C: ?Sized> {
    /// The session searched for.
    session: &'a Session,
    /// The catalog the stellars are read from.
    catalog: &'a C,
    /// Each system's raw links, from the catalog's star map.
    links: BTreeMap<SystemId, Vec<SystemId>>,
}

impl<C: PilotCatalog + ?Sized> EscapeMap for SearchMap<'_, C> {
    fn raw_links(&self, system: SystemId) -> Vec<SystemId> {
        self.links.get(&system).cloned().unwrap_or_default()
    }

    fn open(&self, system: SystemId) -> bool {
        let galaxy = &self.session.galaxy;
        galaxy.exists(system)
            && galaxy.visible(system)
            && self.session.is_known(system)
            && self.session.pilot.has_explored(system)
    }

    fn stellars(&self, system: SystemId) -> Vec<(StellarId, bool)> {
        let record = self.session.pilot.system_record(system);
        self.catalog
            .landing_sites(system)
            .into_iter()
            .map(|site| {
                let site = self.session.galaxy.site(site);
                let active = self.session.stellar_owner(site.id).is_some();
                (site.id, qualifies(&site, active, record))
            })
            .collect()
    }
}

impl Session {
    /// Rescues the player from the escape pod once its countdown has run
    /// out, reading `catalog` and drawing on `chance` (see the module
    /// docs); whether it did. The flight view asks after each tick and
    /// stops stepping when it did, so the dësc opens before the next.
    pub fn rescue_if_due(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        chance: &mut (impl Chance + ?Sized),
    ) -> bool {
        if !std::mem::take(&mut self.rescue_due) {
            return false;
        }
        let save_due = self.save_due;
        let from = self.pilot.system;
        self.end_every_mission_for_rescue(&mut &mut *chance);
        self.reset_player_for_rescue();
        self.ship_hook(RESCUE_SHIP, ShipHook::Purchase, &mut &mut *chance);
        let system = self.escape_destination(from, catalog);
        let arrival = Arrival {
            system,
            sites: self.read_sites(catalog, system),
            placement: Placement {
                ship: ShipPlacement::Arrive(ShipState {
                    position: RESCUE_AT,
                    velocity: Vec2::ZERO,
                    heading: 0.0,
                }),
                stellar: None,
                hold: false,
            },
        };
        self.enter_system(Entry::Rescue, arrival, catalog, catalog, chance);
        self.fill_reserves_for_rescue();
        let days = RESCUE_DAYS + u32::from(chance.roll(RESCUE_EXTRA_DAYS));
        self.pass_days(days, chance);
        self.rebuild_stellars();
        self.rename_for_rescue(&mut &mut *chance);
        self.reset_records_for_rescue();
        self.save_due = save_due;
        self.strict_play_save(catalog);
        self.rescue_desc_due = true;
        true
    }

    /// The Strict Play save that ends the rescue (@0x69292-0x692e3): with
    /// Strict Play set, the pilot resumes at the first stellar of the
    /// system it was taken to, in nav order, or [`STRICT_SAVE_FALLBACK`]
    /// when it has none, and a save is due; without it, nothing. It comes
    /// after the rescue's own `save_due` is put back, so this is the one
    /// save the rescue makes.
    ///
    /// The session stays unlanded. A later save (the menu, quitting)
    /// still carries this stellar until a jump or a script's move resets
    /// it, as the original's file does after the rescue.
    fn strict_play_save(&mut self, catalog: &(impl PilotCatalog + ?Sized)) {
        if !self.pilot.strict_play() {
            return;
        }
        let first = self
            .read_sites(catalog, self.pilot.system)
            .first()
            .map_or(STRICT_SAVE_FALLBACK, |site| site.id);
        self.pilot.stellar = Some(first);
        self.save_due = true;
    }

    /// The rescue's dësc, once: [`RESCUE_DESC`] after a rescue, then none.
    pub fn take_rescue_desc(&mut self) -> Option<i16> {
        std::mem::take(&mut self.rescue_desc_due).then_some(RESCUE_DESC)
    }

    /// Ends every active mission as `_AbortMission(slot, 1)` does, in slot
    /// order, drawing on `chance` (see the module docs).
    pub(super) fn end_every_mission_for_rescue(&mut self, chance: &mut dyn Chance) {
        let active: Vec<_> = self
            .pilot
            .missions
            .iter()
            .map(|(slot, active)| (slot, active.mission))
            .collect();
        for (slot, mission) in active {
            let record = self.lifecycle_record(mission);
            self.end_mission(slot, &record, true, chance);
        }
    }

    /// `_ResetPlayer(0)`'s part of the rescue that the engine models (see
    /// the module docs), the placement and the scene left to the entry.
    fn reset_player_for_rescue(&mut self) {
        let Some(record) = self.ship_record(RESCUE_SHIP).cloned() else {
            return;
        };
        self.board_class(&record, &BTreeMap::new());
        self.condition = Condition::Intact;
        self.pod = None;
        self.target = None;
        self.self_destruct.reset();
        // The nav mode and the dock request go to -1, and the pilot is past
        // the tutorial, as at every flight start (`_ResetPlayer`
        // @0x1d4cd-0x1d4d5, @0x1d7cf; see the `nav` and `clearance`
        // modules).
        self.set_nav(crate::navigation::NavMode::Off);
        self.dock = None;
        self.tutorial = crate::tutorial::Tutorial::SEASONED;
        self.secondary = None;
        self.pilot.jumped_from = None;
        self.pilot.course.clear();
        self.pilot.cargo.clear();
        self.pilot.paint = None;
        self.red_alert = RedAlert::default();
        self.recent_rank = None;
        // The system died in: its timer cancelled and its countdown
        // cleared (@0x1db17-0x1db30; see the `reinforcements` module).
        self.reset_reinforcements_here();
        self.reset_precalced();
    }

    /// The rescue's reserves: fuel and shield full, the armour as
    /// [`RuleKey::RescueArmor`] reads (see the module docs).
    fn fill_reserves_for_rescue(&mut self) {
        let stats = self.stats;
        let armor = match self.rules.source_for(RuleKey::RescueArmor) {
            RuleSource::Engine => stats.shield,
            RuleSource::Bible => stats.armor,
        };
        let reserves = &mut self.pilot.reserves;
        reserves.fuel.now = stats.fuel;
        reserves.shield.now = stats.shield;
        reserves.armor.now = armor;
    }

    /// Names the rescued ship `shïp` 128's name, a space and four digits
    /// each 1 to 9, drawn on `chance`.
    fn rename_for_rescue(&mut self, chance: &mut dyn Chance) {
        let Some(name) = self.ship_name(RESCUE_SHIP) else {
            return;
        };
        self.pilot.ship_name = Some(crate::shipyard::numbered_name(name, 4, chance));
    }

    /// Every system's record, hidden ones included, and the record with
    /// each government, its government's `InitialRec` (see the module
    /// docs).
    fn reset_records_for_rescue(&mut self) {
        let govts: Vec<_> = self.govts.ids().collect();
        for govt in govts {
            let record = self.govts.initial_record(Some(govt));
            self.pilot.set_legal_record(govt, record);
        }
        self.pilot.system_legal = Some(crate::legal::systems_from_govts(
            self.galaxy.governed(),
            &BTreeMap::new(),
            &self.govts,
        ));
    }

    /// The escape search's galaxy, read from `catalog` (see the module
    /// docs).
    pub(super) fn search_map<'a, C: PilotCatalog + ?Sized>(
        &'a self,
        catalog: &'a C,
    ) -> SearchMap<'a, C> {
        let links = catalog
            .star_map()
            .into_iter()
            .map(|system| (system.id, system.links))
            .collect();
        SearchMap {
            session: self,
            catalog,
            links,
        }
    }

    /// The system a visible system owns `stellar` in, if any.
    fn stellar_owner(&self, stellar: StellarId) -> Option<SystemId> {
        let listed = self
            .listed_stellars
            .iter()
            .find(|listed| listed.id == stellar)?;
        self.galaxy.owner(&listed.systems)
    }

    /// Where the rescue takes the pod from `from`, the system the player
    /// died in (see the module docs): the system of the stellar the escape
    /// search finds, or [`RESCUE_FALLBACK`].
    pub(super) fn escape_destination(
        &self,
        from: SystemId,
        catalog: &(impl PilotCatalog + ?Sized),
    ) -> SystemId {
        escape_stellar(from, &self.search_map(catalog))
            .map_or(RESCUE_FALLBACK, |(stellar, found_in)| {
                self.stellar_owner(stellar).unwrap_or(found_in)
            })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::LandingSite;
    use crate::landing::StellarFlags;
    use crate::testkit::{FakePilotCatalog, catalog, planet, star};

    /// A shipyard `id` at the system's centre.
    fn yard(id: i16) -> LandingSite {
        LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::SHIPYARD,
            ..planet(id, 0.0, 0.0)
        }
    }

    /// The test galaxy: 130 (the start) links to 131 twice, then to 999
    /// (no system) and 133; 131 to 132. 131 has a plain planet and a
    /// shipyard 141, 132 a shipyard 150, 133 a shipyard 160.
    fn galaxy() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.star_map = vec![
            star(130, (0.0, 0.0), &[131, 131, 999, 133]),
            star(131, (600.0, 0.0), &[132]),
            star(132, (600.0, 600.0), &[]),
            star(133, (-600.0, 0.0), &[]),
        ];
        catalog.systems = vec![SystemId(130), SystemId(131), SystemId(132), SystemId(133)];
        catalog.sites = vec![
            (SystemId(130), vec![yard(128)]),
            (SystemId(131), vec![planet(140, 0.0, 0.0), yard(141)]),
            (SystemId(132), vec![yard(150)]),
            (SystemId(133), vec![yard(160)]),
        ];
        catalog
    }

    /// A session of `catalog` with `systems` explored too, known as the
    /// next rebuild point works them out.
    fn explored(catalog: &FakePilotCatalog, systems: &[i16]) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        for &system in systems {
            session.pilot.explore(SystemId(system));
        }
        session.rework_known();
        session
    }

    #[test]
    fn the_search_map_closes_an_unknown_system() {
        let catalog = galaxy();
        let mut session = explored(&catalog, &[133]);
        assert!(session.search_map(&catalog).open(SystemId(133)));
        session.known.remove(&SystemId(133));
        assert!(!session.search_map(&catalog).open(SystemId(133)), "unknown");
        assert_eq!(
            session.escape_destination(SystemId(130), &catalog),
            RESCUE_FALLBACK,
            "133's shipyard is passed over"
        );
    }

    #[test]
    fn the_search_map_reads_raw_links_and_open_systems() {
        let catalog = galaxy();
        let session = explored(&catalog, &[131]);
        let map = session.search_map(&catalog);
        assert_eq!(
            map.raw_links(SystemId(130)),
            [131, 131, 999, 133].map(SystemId),
            "raw, repeats and all"
        );
        assert_eq!(map.raw_links(SystemId(999)), []);
        assert!(map.open(SystemId(131)));
        assert!(!map.open(SystemId(132)), "not explored");
        assert!(!map.open(SystemId(999)), "not in the galaxy");
    }

    #[test]
    fn the_search_map_qualifies_each_stellar_in_nav_order() {
        let catalog = galaxy();
        let mut session = explored(&catalog, &[131]);
        let map = session.search_map(&catalog);
        assert_eq!(
            map.stellars(SystemId(131)),
            [(StellarId(140), false), (StellarId(141), true)]
        );
        session.pilot.set_system_record(SystemId(131), -1);
        let mut barred = galaxy();
        barred.sites[1].1[1].min_status = 0;
        let map = session.search_map(&barred);
        assert_eq!(
            map.stellars(SystemId(131))[1],
            (StellarId(141), false),
            "MinStatus"
        );
    }

    #[test]
    fn the_search_map_holds_a_stellar_whose_owner_is_hidden_inactive() {
        let mut catalog = galaxy();
        hideable(&mut catalog);
        let mut session = explored(&catalog, &[131, 133]);
        session.pilot.bits_mut().set(hiding_bit(131));
        session.refresh_galaxy(&catalog);
        assert!(!session.galaxy().visible(SystemId(131)), "hidden");
        let map = session.search_map(&catalog);
        assert_eq!(
            map.stellars(SystemId(131)),
            [(StellarId(140), false), (StellarId(141), false)],
            "the shipyard's owner is hidden"
        );
        assert_eq!(map.stellars(SystemId(133)), [(StellarId(160), true)]);
    }

    #[test]
    fn the_search_map_passes_over_a_shipyard_the_story_destroyed() {
        let catalog = galaxy().living([crate::testkit::life(141, 5)]);
        let mut session = explored(&catalog, &[131, 132, 133]);
        session.run_set(
            &crate::control::SetExpr::parse("Y141").expect("parses"),
            &mut Scripted::default(),
        );
        assert!(session.galaxy().destroyed(StellarId(141)), "destroyed");
        let map = session.search_map(&catalog);
        assert_eq!(
            map.stellars(SystemId(131)),
            [(StellarId(140), false), (StellarId(141), false)],
            "the overlay's destruction"
        );
        assert_eq!(
            session.escape_destination(SystemId(130), &catalog),
            SystemId(133),
            "on to the next linked system's shipyard"
        );
    }

    #[test]
    fn the_search_map_closes_a_hidden_system() {
        let mut catalog = galaxy();
        hideable(&mut catalog);
        let mut session = explored(&catalog, &[131, 133]);
        session.pilot.bits_mut().set(hiding_bit(133));
        session.refresh_galaxy(&catalog);
        assert!(
            session.galaxy().exists(SystemId(133)),
            "still in the galaxy"
        );
        assert!(!session.search_map(&catalog).open(SystemId(133)), "hidden");
        assert!(session.search_map(&catalog).open(SystemId(131)));
    }

    #[test]
    fn the_pod_passes_over_a_stellar_no_system_owns() {
        let mut catalog = galaxy();
        catalog.star_map[0].links = [134, 133].map(SystemId).to_vec();
        catalog.star_map.push(star(134, (-600.0, 0.0), &[]));
        catalog.systems.push(SystemId(134));
        catalog.sites.push((SystemId(134), vec![yard(170)]));
        let session = explored(&catalog, &[133, 134]);
        assert!(session.search_map(&catalog).open(SystemId(134)));
        assert_eq!(
            session.stellar_owner(StellarId(170)),
            None,
            "133 stands for 134's place"
        );
        assert_eq!(
            session.search_map(&catalog).stellars(SystemId(134)),
            [(StellarId(170), false)]
        );
        assert_eq!(
            session.escape_destination(SystemId(130), &catalog),
            SystemId(133)
        );
    }

    #[test]
    fn the_pod_goes_to_the_found_stellars_system_or_128() {
        let catalog = galaxy();
        let session = explored(&catalog, &[131, 132, 133]);
        assert_eq!(
            session.escape_destination(SystemId(130), &catalog),
            SystemId(131)
        );
        let session = explored(&catalog, &[132, 133]);
        assert_eq!(
            session.escape_destination(SystemId(130), &catalog),
            SystemId(133)
        );
        let session = explored(&catalog, &[]);
        assert_eq!(
            session.escape_destination(SystemId(130), &catalog),
            RESCUE_FALLBACK
        );
    }

    use crate::catalog::{
        GovtId, GovtRecord, HullRecord, MissionRecord, OutfitId, StockWeapon, WeaponId,
    };
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::combat::armament::MOD_WEAPON;
    use crate::combat::hull::ESCAPE_POD_SHIP;
    use crate::control::{Bit, Script};
    use crate::handling::ShipFields;
    use crate::market::Good;
    use crate::outfitter::OutfitFlags;
    use crate::rulebook::Rulebook;
    use crate::story::StoryEvent;
    use crate::testkit::{
        FAST, Scripted, govt, hideable, hiding_bit, hull, mission, outfit, ship, weapon,
    };

    /// [`galaxy`] with system 128 too, governed: 130 by 140 (`InitialRec`
    /// 7), 131 and 133 by 141 (3); the player's ship 128, "Shuttle", of
    /// 60 shield and 45 armour, a blaster mounted, `OnPurchase` setting
    /// bit 21 and one of outfit 302 aboard by default; the pod; a
    /// blaster's outfit 250, a persistent outfit 300 and a plain 301; and
    /// mission 130, which may not be aborted and is penalised for it
    /// (`Flags` 0x0040), whose `OnAbort` sets bit 22.
    fn rescuing() -> FakePilotCatalog {
        let mut catalog = galaxy();
        catalog.star_map.push(star(128, (-600.0, -600.0), &[]));
        catalog.systems.push(SystemId(128));
        for system in &mut catalog.star_map {
            system.govt = match system.id.0 {
                130 => Some(GovtId(140)),
                131 | 133 => Some(GovtId(141)),
                _ => None,
            };
        }
        hideable(&mut catalog);
        catalog.govts = vec![
            GovtRecord {
                initial_rec: 7,
                ..govt(140)
            },
            GovtRecord {
                initial_rec: 3,
                ..govt(141)
            },
        ];
        let mut shuttle = ship(
            128,
            ShipFields {
                shield: 60,
                armor: 45,
                ..FAST
            },
        );
        shuttle.name = "Shuttle".to_owned();
        shuttle.on_purchase = Script::parse("b21");
        shuttle.defaults = vec![(OutfitId(302), 1)];
        catalog.ship_records = vec![shuttle, ship(ESCAPE_POD_SHIP.0, FAST)];
        catalog.weapons = vec![weapon(128)];
        catalog.hulls = vec![
            HullRecord {
                weapons: vec![StockWeapon {
                    weapon: WeaponId(128),
                    count: 1,
                    ammo: 0,
                }],
                ..hull(128)
            },
            hull(ESCAPE_POD_SHIP.0),
        ];
        catalog.outfits = vec![
            outfit(250, &[(MOD_WEAPON, 128)]),
            crate::catalog::OutfitRecord {
                flags: OutfitFlags::PERSISTENT,
                ..outfit(300, &[])
            },
            outfit(301, &[]),
            outfit(302, &[]),
        ];
        catalog.missions = vec![MissionRecord {
            on_abort: Script::parse("b22"),
            can_abort: 0,
            flags: 0x0040,
            ..mission(130)
        }];
        catalog
    }

    /// `catalog`'s pilot in the pod, the rescue due, having explored 131.
    fn due(catalog: &FakePilotCatalog) -> Session {
        let mut session = explored(catalog, &[131]);
        session.pilot.ship = ESCAPE_POD_SHIP;
        session.pod = Some(0);
        session.rescue_due = true;
        session
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("a bit")
    }

    /// A chance whose every roll is its highest outcome and that never
    /// fires.
    struct Highest;

    impl Chance for Highest {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            n - 1
        }

        fn roll(&mut self, sides: u16) -> u16 {
            sides.saturating_sub(1)
        }
    }

    #[test]
    fn the_rescue_starts_no_takeoff_grace() {
        use crate::player_fate::PlayerCommand;
        let catalog = rescuing();
        let mut session = due(&catalog);
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        let _ = session.take_sounds();
        assert!(session.takes(PlayerCommand::Map));
        assert!(session.takes(PlayerCommand::MissionInfo));
        assert!(session.press_player_info());
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn the_rescue_idles_the_self_destruct() {
        let catalog = rescuing();
        let mut session = due(&catalog);
        session.self_destruct.step(true, false);
        assert_eq!(session.self_destruct.count(), Some(149));
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert_eq!(session.self_destruct.count(), None, "_ResetPlayer @0x1d764");
    }

    #[test]
    fn the_rescue_shows_desc_13999_once() {
        let catalog = rescuing();
        let mut session = due(&catalog);
        assert_eq!(session.take_rescue_desc(), None, "not before");
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert_eq!(session.take_rescue_desc(), Some(13999));
        assert_eq!(session.take_rescue_desc(), None, "once");
        assert!(!session.rescue_if_due(&catalog, &mut NeverFires), "once");
    }

    #[test]
    fn the_rescue_ends_every_mission_with_its_on_abort_and_releases_its_ships() {
        let catalog = rescuing();
        let mut session = due(&catalog);
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(130, 0));
        session
            .pilot
            .missions
            .put(3, crate::testkit::carrying(130, 0));
        let marked = session.traffic.add_npc(crate::traffic::npc::Npc {
            mission: Some(crate::traffic::npc::MissionMark {
                slot: 0,
                behav: crate::mission::ships::ShipBehav::Own,
                formation: None,
                order: None,
            }),
            ..crate::testkit::npc(0, crate::stats::ShipStats::default())
        });
        session.end_every_mission_for_rescue(&mut NeverFires);
        assert_eq!(session.pilot.missions.iter().count(), 0);
        assert!(session.control_bit(bit(22)), "OnAbort");
        assert_eq!(session.npc(marked).expect("here").mission, None, "released");
        let mut session = due(&catalog);
        session
            .pilot
            .missions
            .put(5, crate::testkit::carrying(130, 0));
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert_eq!(session.pilot.missions.iter().count(), 0);
        assert!(session.control_bit(bit(22)));
    }

    #[test]
    fn ending_missions_for_the_rescue_ignores_can_abort_and_changes_no_record() {
        let catalog = rescuing();
        let mut session = due(&catalog);
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(130, 0));
        session.pilot.set_legal_record(GovtId(140), 5);
        session.pilot.set_system_record(SystemId(130), 5);
        session.end_every_mission_for_rescue(&mut NeverFires);
        assert_eq!(session.pilot.missions.iter().count(), 0, "aborted");
        assert_eq!(session.pilot.legal_record(GovtId(140)), 5);
        assert_eq!(session.pilot.system_record(SystemId(130)), 5);
    }

    #[test]
    fn the_rescued_player_flies_ship_128_renamed_with_four_digits_after_on_purchase() {
        let catalog = rescuing();
        let lowest: Box<dyn Chance> = Box::new(Scripted::default());
        for (mut chance, name) in [
            (lowest, "Shuttle 1111"),
            (Box::new(Highest), "Shuttle 9999"),
        ] {
            let mut session = due(&catalog);
            for id in [300, 301] {
                session.pilot.outfits.insert(OutfitId(id), 1);
            }
            assert!(session.rescue_if_due(&catalog, &mut *chance));
            assert_eq!(session.pilot.ship, RESCUE_SHIP);
            assert_eq!(session.pilot.ship_name(), Some(name));
            assert!(session.control_bit(bit(21)), "OnPurchase");
            let owned: Vec<(i16, u16)> = session
                .pilot
                .outfits
                .iter()
                .map(|(id, &n)| (id.0, n))
                .collect();
            assert_eq!(
                owned,
                [(250, 1), (300, 1)],
                "persistent and stock, no defaults"
            );
            assert_eq!(session.pod, None);
            assert_eq!(session.player_condition(), Condition::Intact);
            assert_eq!(session.player_fate(), crate::PlayerFate::Flying);
        }
    }

    #[test]
    fn the_rescue_fills_fuel_and_shield_and_sets_armour_by_the_rule() {
        let catalog = rescuing();
        for (source, armor) in [(RuleSource::Engine, 60.0), (RuleSource::Bible, 45.0)] {
            let rules = Rulebook::default().with_override(RuleKey::RescueArmor, source);
            let mut session = due(&catalog).with_rules(rules);
            session.pilot.reserves.fuel.now = 1.0;
            session.pilot.reserves.shield.now = 1.0;
            assert!(session.rescue_if_due(&catalog, &mut NeverFires));
            let reserves = session.pilot.reserves;
            assert_eq!(
                (reserves.fuel.now, reserves.shield.now, reserves.armor.now),
                (300.0, 60.0, armor),
                "{source:?}"
            );
            assert_eq!(reserves.armor.max, 45.0);
            for _ in 0..30 {
                session.tick_combat(Rules::default(), &mut NeverFires);
            }
            assert_eq!(session.pilot.reserves.armor.now, armor, "{source:?}: kept");
        }
    }

    #[test]
    fn the_rescue_lands_in_the_searched_stellars_system_or_128() {
        let catalog = rescuing();
        let mut session = due(&catalog);
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert_eq!(session.system(), SystemId(131));
        let mut session = explored(&catalog, &[]);
        session.pod = Some(0);
        session.rescue_due = true;
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert_eq!(session.system(), RESCUE_FALLBACK);
    }

    #[test]
    fn the_rescued_player_is_not_landed_and_the_system_is_populated_afresh() {
        let mut catalog = rescuing();
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        catalog.traffic = vec![(
            SystemId(131),
            crate::catalog::SystemTraffic {
                dude_types,
                avg_ships: 2,
                persons: Default::default(),
            },
        )];
        catalog.dudes = vec![(
            crate::catalog::DudeId(128),
            crate::catalog::DudeRecord {
                ai_type: 1,
                govt: None,
                ships: vec![(ShipId(129), 1)],
                booty: 0,
                info_types: 0,
            },
        )];
        catalog.ship_records.push(ship(129, FAST));
        let mut session = due(&catalog);
        let hulk = session
            .traffic
            .add_npc(crate::testkit::npc(0, crate::stats::ShipStats::default()));
        session.player.velocity = Vec2::new(3.0, 4.0);
        session.player.heading = 90.0;
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert_eq!(session.landed(), None);
        assert_eq!(session.pilot().stellar, None);
        assert_eq!(
            *session.player(),
            ShipState {
                position: RESCUE_AT,
                velocity: Vec2::ZERO,
                heading: 0.0,
            }
        );
        assert!(session.npc(hulk).is_none(), "the old ships gone");
        assert_eq!(session.npcs().len(), 2, "the new system's set up");
        assert!(
            session
                .take_events()
                .contains(&super::super::SessionEvent::SystemChanged)
        );
    }

    #[test]
    fn the_rescue_passes_15_to_44_days() {
        let catalog = rescuing();
        let lowest: Box<dyn Chance> = Box::new(Scripted::default());
        for (mut chance, days) in [(lowest, 15), (Box::new(Highest), 44)] {
            let mut session = due(&catalog);
            let mut expected = session.date();
            for _ in 0..days {
                expected = expected.next_day();
            }
            assert!(session.rescue_if_due(&catalog, &mut *chance));
            assert_eq!(session.date(), expected, "{days}");
        }
    }

    #[test]
    fn the_rescue_resets_each_systems_record_to_its_govts_initial_rec() {
        let catalog = rescuing();
        let mut session = due(&catalog);
        session.pilot.bits_mut().set(hiding_bit(133));
        session.refresh_galaxy(&catalog);
        assert!(!session.galaxy().visible(SystemId(133)), "hidden");
        for system in [130, 131, 133] {
            session.pilot.set_system_record(SystemId(system), 99);
        }
        session.pilot.set_legal_record(GovtId(140), 99);
        session.pilot.set_legal_record(GovtId(141), 99);
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        let records: Vec<i16> = [128, 130, 131, 132, 133]
            .map(|system| session.pilot.system_record(SystemId(system)))
            .to_vec();
        assert_eq!(records, [0, 7, 3, 0, 3], "the hidden 133 too");
        assert_eq!(session.pilot.legal_record(GovtId(140)), 7);
        assert_eq!(session.pilot.legal_record(GovtId(141)), 3);
    }

    #[test]
    fn the_rescue_empties_the_cargo_and_keeps_the_credits_and_exploration() {
        let catalog = rescuing();
        let mut session = due(&catalog);
        session.pilot.cargo.insert(Good::Commodity(0), 5);
        session
            .pilot
            .cargo
            .insert(Good::Junk(crate::catalog::JunkId(128)), 2);
        session.pilot.cash = 1234;
        session.pilot.explore(SystemId(133));
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert_eq!(session.pilot.cargo, BTreeMap::new());
        assert_eq!(session.pilot.cash, 1234);
        for system in [130, 131, 133] {
            assert!(session.pilot.has_explored(SystemId(system)), "{system}");
        }
    }

    #[test]
    fn the_rescue_clears_what_reset_player_clears_and_explores_the_fallback() {
        use crate::catalog::{RankId, SoundId};
        use crate::scan::Latches;
        use crate::traffic::npc::NpcId;
        let catalog = rescuing();
        let mut session = explored(&catalog, &[]);
        session.pod = Some(0);
        session.rescue_due = true;
        session.pilot.course = vec![SystemId(131), SystemId(132)];
        session.pilot.paint = Some(crate::Rgb15 { r: 1, g: 2, b: 3 });
        session.secondary = Some(WeaponId(128));
        session.pilot.jumped_from = Some(SystemId(133));
        session.red_alert.check(true, false, true);
        session.red_alert.step(0);
        assert!(session.red_alert_icon().is_some(), "the alert is up");
        session.recent_rank = Some(RankId(140));
        session.queued.sound = Some(SoundId(200));
        session.target = Some(NpcId(5));
        session.nav = crate::navigation::NavMode::Stellar(Some(StellarId(128)));
        session.pilot.ranks.insert(RankId(141));
        session.blessing_ranks.clear();
        session.scan_latches = Latches {
            junk: false,
            outfits: false,
        };
        assert!(!session.pilot.has_explored(RESCUE_FALLBACK), "unexplored");
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert_eq!(session.system(), RESCUE_FALLBACK);
        assert!(session.pilot.has_explored(RESCUE_FALLBACK), "explored");
        assert_eq!(session.pilot.course(), []);
        assert_eq!(session.pilot.paint, None);
        assert_eq!(session.secondary, None);
        assert_eq!(session.pilot.jumped_from, None);
        assert_eq!(session.red_alert, RedAlert::default(), "threat and icon");
        assert_eq!(session.recent_rank, None);
        assert_eq!(session.queued.sound, None);
        assert_eq!(session.target, None);
        assert_eq!(session.nav, crate::navigation::NavMode::Off, "_ResetPlayer");
        assert_eq!(session.blessing_ranks, session.pilot.ranks, "taken again");
        assert_eq!(session.scan_latches, Latches::ARMED);
    }

    #[test]
    fn the_rescue_cancels_the_reinforcements_of_the_system_died_in_alone() {
        let catalog = rescuing();
        let mut session = explored(&catalog, &[]);
        let died_in = session.system();
        session.pod = Some(0);
        session.rescue_due = true;
        session.pilot.reinforcement_days =
            std::collections::BTreeMap::from([(died_in, 2), (SystemId(999), 3)]);
        session.reinforcements.timers.insert(died_in, 12);
        session.reinforcements.timers.insert(SystemId(999), 7);
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert_eq!(
            session.pilot.reinforcement_days.get(&died_in),
            None,
            "@0x1db30"
        );
        assert_eq!(
            session.reinforcements.timers.get(&died_in),
            None,
            "@0x1db17"
        );
        assert_eq!(session.reinforcements.timers.get(&SystemId(999)), Some(&7));
    }

    #[test]
    fn the_rescue_raises_no_entered_system_hook() {
        let catalog = rescuing();
        let mut session = due(&catalog);
        session.drop_raised();
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        let raised = session.story_raised();
        assert!(
            !raised
                .iter()
                .any(|event| matches!(event, StoryEvent::EnteredSystem { .. })),
            "{raised:?}"
        );
        assert!(
            raised
                .iter()
                .any(|event| matches!(event, StoryEvent::DayPassed { .. })),
            "the days pass: {raised:?}"
        );
    }

    #[test]
    fn a_rescue_without_strict_play_makes_no_save_due() {
        let catalog = rescuing();
        let mut session = due(&catalog);
        session.take_save_due();
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert!(!session.take_save_due());
        assert_eq!(session.pilot().stellar, None, "no resume stellar");
    }

    /// `session` with its pilot playing strictly.
    fn strict(mut session: Session) -> Session {
        session.pilot = session.pilot.clone().with_strict_play(true);
        session
    }

    #[test]
    fn a_strict_rescue_makes_a_save_due_at_the_systems_first_stellar() {
        let catalog = rescuing();
        let mut session = strict(due(&catalog));
        for system in [130, 131, 133] {
            session.pilot.set_system_record(SystemId(system), 99);
        }
        session.pilot.set_legal_record(GovtId(140), 99);
        session.pilot.set_legal_record(GovtId(141), 99);
        let mut expected = session.date();
        for _ in 0..RESCUE_DAYS {
            expected = expected.next_day();
        }
        session.take_save_due();
        assert!(session.rescue_if_due(&catalog, &mut Scripted::default()));
        assert!(session.take_save_due(), "a save is due");
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("decodes");
        assert_eq!(saved.system, SystemId(131));
        assert_eq!(
            saved.stellar(),
            Some(StellarId(140)),
            "the first in nav order, not the shipyard found"
        );
        assert_eq!(saved.date, expected);
        let records: Vec<i16> = [130, 131, 133]
            .map(|system| saved.system_record(SystemId(system)))
            .to_vec();
        assert_eq!(records, [7, 3, 3]);
        assert_eq!(saved.legal_record(GovtId(140)), 7);
        assert_eq!(saved.legal_record(GovtId(141)), 3);
    }

    #[test]
    fn a_strict_rescue_leaves_the_session_unlanded() {
        let catalog = rescuing();
        let mut session = strict(due(&catalog));
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert_eq!(session.landed(), None);
        assert_eq!(
            *session.player(),
            ShipState {
                position: RESCUE_AT,
                velocity: Vec2::ZERO,
                heading: 0.0,
            }
        );
    }

    #[test]
    fn a_strict_rescue_into_a_system_without_stellars_saves_stellar_128() {
        let catalog = rescuing();
        let mut session = strict(explored(&catalog, &[]));
        session.pod = Some(0);
        session.rescue_due = true;
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert_eq!(session.system(), RESCUE_FALLBACK);
        assert!(session.take_save_due());
        assert_eq!(session.pilot().stellar, Some(StellarId(128)));
        assert_eq!(STRICT_SAVE_FALLBACK, StellarId(128));
    }

    #[test]
    fn a_strict_rescued_pilot_flown_again_resumes_on_the_first_stellar() {
        let catalog = rescuing();
        let mut session = strict(due(&catalog));
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("decodes");
        let resumed = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(resumed.system(), SystemId(131));
        assert_eq!(crate::testkit::resumed_on(&resumed), Some(StellarId(140)));
    }
}
