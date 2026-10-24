//! Persons' mission offers in flight: the session's side of the
//! [`mission`](crate::mission) module's "From a ship".
//!
//! **At spawn** (`_SpawnPerson` @0x4104b-0x41072). Every person NPC the
//! traffic brings in, populating the system or arriving mid-stay, has its
//! `LinkMission` resolved ([`resolve`](crate::mission::resolve)), with no
//! availability check, once the traffic's step is done, on the traffic's
//! chance, in the order the persons came. The ship offers are kept by
//! mission, as the original's `_briefMissionInfo` is, so a later person
//! linked to the same mission overwrites the values. Nothing else
//! resolves a mission offered from a ship: the landing roll never lists
//! one. They are never saved, and a landing leaves them alone. The
//! stellar the offer is resolved from, the player ship's stellar target
//! (+0x6c, @0xa151f-0xa1526), is read as
//! [`RuleKey::PersonMissionStellar`] says: by the engine the navigation
//! target, or no stellar with none; by the other reading the first
//! stellar the player's system lists, or spöb 128 when it lists none
//! (the `_offeringFromShip`-set branch, @0xa14e8-0xa1518). `_SpawnPerson`
//! never runs with `_offeringFromShip` set, so the original always takes
//! the navigation target.
//!
//! **The answer** (`_HandlePlayerCommunication` @0x622b6-0x62547,
//! `_HandlePlayerBoardAttempt` @0x65982-0x65f6d; see
//! [`Session::end_ship_offer`]): an accept on the hail makes a person of
//! `Flags` 0x0100 gone for good and one of 0x0800 leave; then, for a
//! person of 0x0040 whose mission the first slot holding it has one ship
//! left of, after `OnAccept`, whatever the goal, the person is swapped
//! for that ship as the settling begins (see the `mission_ships`
//! module). Only the hail swaps: a boarding never does.
//!
//! **`<OSN>`** reads the original's `_offeringShipIndex` by the engine
//! ([`RuleKey::OfferingShipName`]): a hail of a person with a
//! `LinkMission` and without `Flags` 0x0200 names its ship's slot
//! whether or not the mission is offered, and keeps it after the dialog
//! (@0x621de, @0x625a4); a board offer names the boarded ship only while
//! it is open (@0x65882, @0x65a11); a hail quote, and the 0x0400 quote
//! filter's check of a linked person, name nobody after (see the
//! `persons` module); a new pilot or a load names nobody. The name is the
//! person of the ship last brought in at that slot ([`Npc::berth`],
//! reused as the original's slots are), whether or not it is still there
//! ([`Traffic::berth_person`](crate::Traffic::berth_person)): none for a
//! ship with no person, or once a capture cleared it. After an accept
//! whose person is swapped for its mission ship, the slot named is the
//! swapped-in ship's, the player's new target (@0x6252f, @0x625a4),
//! which has no person (`_GenericRandomShipSpawn` @0x3cb48), so `<OSN>`
//! reads `"[Error]"`.
//! **The hail** ([`Session::hail_key`]) offers only when it hails a ship:
//! with no ship target, or with Option held, the comm key hails the
//! navigation stellar instead ([`Hailed::Stellar`]), and no person is
//! asked.

use std::collections::BTreeMap;

use super::hail::{HailTarget, make_leave};
use super::{Chart, MissionRun, Session, Setup, Space};
use crate::catalog::{CommCatalog, MissionId, PersonId, StellarId};
use crate::chance::{Chance, NeverFires};
use crate::escort_dialog::EscortDialogView;
use crate::hail::{Conversation, HailOptions, HailRefusal, HailView};
use crate::mission::VenueOffer;
use crate::mission::{self, Offer, Venue};
use crate::person::{GONE_ON_ACCEPT, LEAVES_ON_ACCEPT, MISSION_ON_BOARD, swaps_for_mission_ship};
use crate::rulebook::{RuleKey, RuleSource};
use crate::sound::SimSound;
use crate::traffic::npc::{Npc, NpcId};

/// The offers from a ship, by mission: each resolved as a person linked
/// to it spawned (`_briefMissionInfo`, 0x35d800).
pub(super) type ShipOffers = BTreeMap<MissionId, Offer>;

/// What brought a person's offer: its hail, or its boarding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OfferedBy {
    /// The player hailed it (`_HandlePlayerCommunication` @0x6219f).
    Hail,
    /// The player boarded it (`_HandlePlayerBoardAttempt` @0x6585c).
    Boarding,
}

/// A person's offer the offer dialog shows, until the player answers it
/// (`_offeringFromShip` and `_offeringShipIndex`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Offering {
    /// The NPC it flies.
    pub(super) npc: NpcId,
    /// The person.
    pub(super) person: PersonId,
    /// Its `LinkMission`.
    pub(super) mission: MissionId,
    /// What brought it.
    pub(super) by: OfferedBy,
}

/// The ship the original's `_offeringShipIndex` names when a hail or a
/// boarding leaves it set, for `<OSN>` by the engine
/// ([`RuleKey::OfferingShipName`]; see the module docs): its slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct OfferingShip {
    /// Its ship-table slot ([`Npc::berth`]).
    pub(super) berth: u32,
}

/// What hailing the player's target opened ([`Session::open_hail`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Hailed {
    /// The comm dialog, as [`Session::hail`] opens it.
    Comm(HailView),
    /// The escort dialog, opened in place of the comm dialog for one of
    /// the fleet's escorts (see the `escort_dialog` module).
    Escort(EscortDialogView),
    /// The person's mission, offered in place of the comm dialog
    /// (`_OfferOneMission` @0x62293): the offer dialog, or the mission
    /// accepted at once.
    Offer(VenueOffer),
    /// The navigation stellar, which answers: the original opens its comm
    /// dialog here (`_DoPlanetCommDialog` @0x6204b). The engine has no
    /// planet comm dialog yet (rdm:task/planet-domination), so this is
    /// the seam where it opens. No sound plays.
    Stellar(StellarId),
}

impl Session {
    /// The stellar whose being a mission's destination counts it reached
    /// as an offer from a ship is accepted in flight (`_DoMissionAccept`
    /// @0xa1fc5-0xa1ff8): none with no such offer open; else, as
    /// [`RuleKey::PersonMissionStellar`] reads the player ship's stellar
    /// target, by the engine the navigation target, and by the other
    /// reading none.
    pub(super) fn ship_offer_reaches(&self) -> Option<StellarId> {
        self.run.offering?;
        match self.setup.rules.source_for(RuleKey::PersonMissionStellar) {
            RuleSource::Engine => self.nav_target(),
            RuleSource::Bible => None,
        }
    }

    /// Resolves the `LinkMission` of each person NPC numbered above
    /// `last` (every one with none), in the order they came, drawing on
    /// `chance` (see the module docs): one whose mission the session has
    /// no record of resolves nothing.
    pub(super) fn resolve_person_offers(&mut self, last: Option<NpcId>, chance: &mut dyn Chance) {
        let linked: Vec<MissionId> = self
            .npcs()
            .iter()
            .filter(|npc| last.is_none_or(|last| npc.id > last))
            .filter_map(|npc| {
                self.space
                    .traffic
                    .person(npc.person?.id)?
                    .record
                    .link_mission
            })
            .map(MissionId)
            .collect();
        for id in linked {
            let Some(record) = self.statics.mission_record(id).cloned() else {
                continue;
            };
            let origin = self.chart.person_mission_stellar(&self.setup);
            let today = self.pilot.date;
            let offer = self
                .chart
                .in_world(&self.pilot, &self.statics, &self.setup, |world| {
                    mission::resolve(&record, Venue::Ship, origin, world, today, chance)
                });
            self.run.ship_offers.insert(id, offer);
        }
    }

    /// Hails as the original's comm key does, Option not held: see
    /// [`Session::hail_key`].
    ///
    /// # Errors
    ///
    /// As [`Session::hail_key`].
    pub fn open_hail(
        &mut self,
        catalog: &(impl CommCatalog + ?Sized),
        options: &HailOptions,
        chance: &mut dyn Chance,
    ) -> Result<Hailed, HailRefusal> {
        self.hail_key(catalog, options, false, chance)
    }

    /// Hails as the original's comm key does, with Option held as
    /// `option` says (`_HandlePlayerCommunication` @0x61f48; see the
    /// module docs). With no ship target, or with Option held, it hails
    /// the navigation stellar ([`Hailed::Stellar`]) when that answers
    /// ([`landing::answers_hail`](crate::landing::answers_hail)),
    /// drawing nothing and making no sound. Otherwise it hails the ship
    /// target: a person whose mission is offered on its hail offers it,
    /// drawing on `chance`, in place of the comm dialog, and no
    /// conversation opens; one of the fleet's escorts opens the escort
    /// dialog in its place (see the `escort_dialog` module), rolling
    /// nothing; any other target opens the comm dialog
    /// ([`Session::hail`]), with `options` listed and the replies read
    /// from `catalog`. A person who may join opens the comm dialog when
    /// `options` list a way to join for it ([`HailOptions::lists_join`]).
    /// Any hail under way ends. An offer beeps
    /// [`SimSound::HailOffer`](crate::SimSound::HailOffer)
    /// (@0x62232-0x6223d).
    ///
    /// # Errors
    ///
    /// The [`HailRefusal`] that applies, first, drawing nothing and
    /// beeping [`SimSound::Refused`](crate::SimSound::Refused)
    /// once: [`HailRefusal::NoTarget`] while landed, jumping or breaking
    /// up, or with no navigation stellar on the stellar path;
    /// [`HailRefusal::NoResponse`] for a navigation stellar that does not
    /// answer, or one not shown; and [`Session::hail`]'s refusals of the
    /// ship target.
    pub fn hail_key(
        &mut self,
        catalog: &(impl CommCatalog + ?Sized),
        options: &HailOptions,
        option: bool,
        chance: &mut dyn Chance,
    ) -> Result<Hailed, HailRefusal> {
        self.space.escorts.talk = None;
        self.space.escorts.dialog = None;
        // Beeps here once: once answering, it answers in `hail` too.
        if let HailTarget::Stellar(stellar) = self.out.beep_unanswered(self.hail_target(option))? {
            return Ok(Hailed::Stellar(stellar));
        }
        let linked = self.space.linked_ship(self.answering()?, OfferedBy::Hail);
        // The hail names a linked person's ship before it checks the
        // mission, and keeps it after the dialog (@0x621de, @0x625a4).
        self.reach_linked(linked);
        let npc = self.answering()?;
        if super::escort_dialog::gets_escort_dialog(npc) {
            let id = npc.id;
            if let Some(view) = self.open_escort_dialog(id) {
                return Ok(Hailed::Escort(view));
            }
        }
        let npc = self.answering()?;
        let offered = self
            .person_offering(npc, OfferedBy::Hail)
            .filter(|_| !self.joins_instead(npc, options));
        match offered {
            Some(offering) => Ok(Hailed::Offer(self.offer_from_ship(offering, chance))),
            None => self.hail(catalog, options, chance).map(Hailed::Comm),
        }
    }

    /// The offer `npc`'s person makes on its hail or boarding, as `by`
    /// says: its `LinkMission`, resolved at its spawn and available from
    /// a ship now, offered on its boarding with `Flags` 0x0200 and on its
    /// hail without; none from the player's escort.
    pub(super) fn person_offering(&self, npc: &Npc, by: OfferedBy) -> Option<Offering> {
        if npc.escort.is_some() {
            return None;
        }
        let person = self.space.traffic.person(npc.person?.id)?;
        let on_board = person.record.flags & MISSION_ON_BOARD != 0;
        if on_board != (by == OfferedBy::Boarding) {
            return None;
        }
        let mission = MissionId(person.record.link_mission?);
        (self.run.ship_offer(mission).is_some() && self.ship_mission_available(mission)).then_some(
            Offering {
                npc: npc.id,
                person: person.record.id,
                mission,
                by,
            },
        )
    }

    /// Names `linked`'s ship for `<OSN>` and checks its mission's
    /// `AvailBits`, writing the cache, as a hail or board of a linked
    /// person does before it offers (@0x621de-0x6220d, @0x65882-0x658b1).
    pub(super) fn reach_linked(&mut self, linked: Option<(OfferingShip, MissionId)>) {
        if let Some((ship, mission)) = linked {
            self.run.offering_ship = Some(ship);
            self.check_avail_bits(mission);
        }
    }

    /// The name `<OSN>` reads by the engine (`_MungeBriefing`
    /// @0x9cebe-0x9cf28): the person of the ship last brought in at the
    /// slot `_offeringShipIndex` names, whether or not it is still there
    /// ([`Traffic::berth_person`](crate::Traffic::berth_person)), none
    /// when it had none or a capture cleared it. None with no slot named.
    pub(super) fn offering_ship_name(&self) -> Option<&str> {
        self.space
            .traffic
            .berth_person(self.run.offering_ship?.berth)
    }

    /// Whether `options` list a way for `npc` to join the fleet, its hail
    /// seen before any conversation is rolled.
    fn joins_instead(&self, npc: &Npc, options: &HailOptions) -> bool {
        let unrolled = Conversation::open(0, false, 0, &mut NeverFires);
        options.lists_join(&self.hail_of(npc, &unrolled))
    }

    /// Opens the offer dialog for `offering`, drawing on `chance`: it
    /// stays pending until the player answers it. The hail or the board
    /// that brought it sounds first, as the original plays it before
    /// `_OfferOneMission`, so an offer accepted at once sounds too:
    /// [`SimSound::HailOffer`] (@0x6223d) or
    /// [`SimSound::PersonOfferBoarded`] (@0x65907).
    pub(super) fn offer_from_ship(
        &mut self,
        offering: Offering,
        chance: &mut dyn Chance,
    ) -> VenueOffer {
        self.out.sounds.push(match offering.by {
            OfferedBy::Hail => SimSound::HailOffer,
            OfferedBy::Boarding => SimSound::PersonOfferBoarded,
        });
        self.run.offering = Some(offering);
        self.offer_dialog(offering.mission, chance)
    }

    /// The person whose offer the offer dialog shows, while it does.
    pub(super) fn offering_person(&self) -> Option<&str> {
        let offering = self.run.offering?;
        let person = self.space.traffic.person(offering.person)?;
        Some(&person.record.name)
    }

    /// Ends the offer from a ship pending, `accepted` or not (see the
    /// module docs): on its hail, an accept makes a person of `Flags`
    /// 0x0100 gone for good (@0x622f4-0x62300), and one of 0x0800 leave
    /// (`_AIMakeShipLeave` @0x6234d); on its boarding, an accept marks
    /// the ship boarded (@0x65f43-0x65f6d), and 0x0100 makes it gone for
    /// good whatever the answer (@0x65982-0x65988). Then, on its hail, an accept of
    /// a person of `Flags` 0x0040 whose mission the first slot holding it
    /// has one ship left of (@0x6236d-0x62445,
    /// [`swaps_for_mission_ship`]) makes its swap for that ship due, for
    /// the settling to come ([`Session::settle_script`]; see the
    /// `mission_ships` module).
    pub(super) fn end_ship_offer(&mut self, accepted: bool) {
        let Some(offering) = self.run.offering.take() else {
            return;
        };
        let flags = self
            .space
            .traffic
            .person(offering.person)
            .map_or(0, |person| person.record.flags);
        let gone = flags & GONE_ON_ACCEPT != 0;
        match offering.by {
            OfferedBy::Hail if accepted => {
                if gone {
                    self.go(offering.person);
                }
                if flags & LEAVES_ON_ACCEPT != 0
                    && let Some(npc) = self.space.npc_mut(offering.npc)
                {
                    make_leave(npc);
                }
                let held = self.pilot.missions.holding(offering.mission);
                if let Some(&slot) = held.first()
                    && let Some(active) = self.pilot.missions.get(slot)
                    && swaps_for_mission_ship(flags, active.ships.left)
                {
                    self.run.swap_due = Some((offering.npc, slot));
                }
            }
            OfferedBy::Hail => {}
            OfferedBy::Boarding => {
                // `_HandlePlayerBoardAttempt` names the boarded ship only
                // around its offer (@0x65882 to @0x65a11).
                self.run.offering_ship = None;
                if accepted {
                    self.mark_boarded(offering.npc);
                }
                if gone {
                    self.go(offering.person);
                }
            }
        }
    }
}

impl Space {
    /// The last NPC numbered in the system: every NPC the traffic brings
    /// in after it is numbered above it.
    pub(super) fn last_npc(&self) -> Option<NpcId> {
        self.npcs().iter().map(|npc| npc.id).max()
    }

    /// The ship `npc` is as `_offeringShipIndex` names it, and its
    /// person's `LinkMission`, when its hail or boarding, as `by` says,
    /// reaches its person's offer: one whose person has a `LinkMission` and
    /// offers on its boarding with `Flags` 0x0200 and on its hail without
    /// (`_HandlePlayerCommunication` @0x621a9-0x621de,
    /// `_HandlePlayerBoardAttempt` @0x6584d-0x65882), whether or not the
    /// mission is then available; none from the player's escort.
    pub(super) fn linked_ship(
        &self,
        npc: &Npc,
        by: OfferedBy,
    ) -> Option<(OfferingShip, MissionId)> {
        if npc.escort.is_some() {
            return None;
        }
        let person = self.traffic.person(npc.person?.id)?;
        let on_board = person.record.flags & MISSION_ON_BOARD != 0;
        let mission = MissionId(person.record.link_mission?);
        (on_board == (by == OfferedBy::Boarding)).then(|| {
            let ship = OfferingShip { berth: npc.berth };
            (ship, mission)
        })
    }
}

impl Chart {
    /// The stellar a person's mission is offered from, as
    /// [`RuleKey::PersonMissionStellar`] reads the player ship's stellar
    /// target: by the engine the navigation target; by the other reading
    /// the first stellar the player's system lists, or spöb 128 with none.
    pub(super) fn person_mission_stellar(&self, setup: &Setup) -> Option<StellarId> {
        match setup.rules.source_for(RuleKey::PersonMissionStellar) {
            RuleSource::Engine => self.nav_target(),
            RuleSource::Bible => Some(self.first_listed_stellar()),
        }
    }

    /// The first stellar the player's system lists, or spöb 128 when it
    /// lists none, as `_RandomizeOneMission` reads the origin with
    /// `_offeringFromShip` set (@0xa14e8-0xa1518: the first nav slot not
    /// -1, else index 0 @0xa1514).
    pub(super) fn first_listed_stellar(&self) -> StellarId {
        self.sites.first().map_or(StellarId(128), |site| site.id)
    }
}

impl MissionRun {
    /// The offer from a ship of `mission`, as the last person linked to it
    /// resolved it.
    pub(super) fn ship_offer(&self, mission: MissionId) -> Option<&Offer> {
        self.ship_offers.get(&mission)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::ai::NovaAi;
    use crate::catalog::{MissionRecord, PersonId, PersonRecord, SystemId, SystemTraffic};
    use crate::chance::NeverFires;
    use crate::rulebook::Rulebook;
    use crate::testkit::{Draws, FAST, FakePilotCatalog, catalog, mission, person, ship};

    /// System 130 (stellars 128 and 129) holds `avg_ships` of no traffic
    /// of its own, with one Person slot naming "Ace" (`përs` 600, ship
    /// 129) at 100 %, and "Bee" (601) linked anywhere; each is linked to
    /// mission 130, offered from a ship (`AvailLoc` 2) and going to its
    /// origin (`TravelStel` -4).
    fn linked(avg_ships: i16) -> FakePilotCatalog {
        let mut slots: [(Option<PersonId>, i16); 8] = Default::default();
        slots[0] = (Some(PersonId(600)), 100);
        FakePilotCatalog {
            traffic: vec![(
                SystemId(130),
                SystemTraffic {
                    dude_types: [(-1, 0); 8],
                    avg_ships,
                    persons: slots,
                },
            )],
            persons: vec![
                PersonRecord {
                    name: "Ace".to_owned(),
                    link_syst: 131,
                    link_mission: Some(130),
                    ..person(600, 129)
                },
                PersonRecord {
                    name: "Bee".to_owned(),
                    link_mission: Some(130),
                    ..person(601, 129)
                },
            ],
            ship_records: vec![ship(129, FAST)],
            missions: vec![MissionRecord {
                avail_loc: 2,
                travel_stel: -4,
                ..mission(130)
            }],
            ..catalog()
        }
    }

    /// The stellar mission 130's offer from a ship, which there must be,
    /// goes to.
    fn offered_to(session: &Session) -> Option<StellarId> {
        let offer = session.run.ship_offer(MissionId(130)).expect("an offer");
        offer.destination.map(|stop| stop.stellar)
    }

    fn populated(catalog: &FakePilotCatalog, rules: Rulebook, nav: Option<i16>) -> Session {
        let mut session = Session::start(catalog).expect("starts").with_rules(rules);
        session.chart.nav = crate::navigation::NavMode::Stellar(nav.map(StellarId));
        session.populate(catalog, &mut NeverFires);
        session
    }

    #[test]
    fn a_person_brought_in_has_its_mission_resolved_from_the_nav_target() {
        let catalog = linked(0);
        let session = populated(&catalog, Rulebook::default(), Some(129));
        assert_eq!(offered_to(&session), Some(StellarId(129)));
        assert_eq!(
            session
                .run
                .ship_offer(MissionId(130))
                .map(|offer| offer.venue),
            Some(Venue::Ship)
        );
        let session = populated(&catalog, Rulebook::default(), None);
        assert_eq!(offered_to(&session), None, "no stellar");
    }

    #[test]
    fn by_the_other_reading_it_is_resolved_from_the_systems_first_stellar() {
        let catalog = linked(0);
        let rules =
            Rulebook::default().with_override(RuleKey::PersonMissionStellar, RuleSource::Bible);
        let session = populated(&catalog, rules, Some(129));
        assert_eq!(offered_to(&session), Some(StellarId(128)));
    }

    /// `catalog` with its linked person's traffic in system 133, which
    /// lists no stellar, and the character starting there.
    fn in_an_empty_system(mut catalog: FakePilotCatalog) -> FakePilotCatalog {
        catalog.systems.push(SystemId(133));
        catalog.traffic[0].0 = SystemId(133);
        if let Ok(start) = &mut catalog.character {
            start.systems[0] = Some(SystemId(133));
        }
        catalog
    }

    #[test]
    fn by_the_other_reading_a_system_with_no_stellar_resolves_from_128() {
        let catalog = in_an_empty_system(linked(0));
        let rules =
            Rulebook::default().with_override(RuleKey::PersonMissionStellar, RuleSource::Bible);
        let session = populated(&catalog, rules, None);
        assert_eq!(session.system(), SystemId(133));
        assert!(session.chart.sites.is_empty());
        assert_eq!(offered_to(&session), Some(StellarId(128)));
    }

    #[test]
    fn by_the_engine_a_system_with_no_stellar_resolves_from_no_stellar() {
        let catalog = in_an_empty_system(linked(0));
        let session = populated(&catalog, Rulebook::default(), None);
        assert_eq!(session.system(), SystemId(133));
        assert!(session.chart.sites.is_empty());
        assert_eq!(offered_to(&session), None);
    }

    #[test]
    fn the_resolution_draws_on_the_traffics_chance_after_its_population() {
        let mut catalog = linked(0);
        catalog.missions[0].cargo_type = 1000;
        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = Draws::of(&[]);
        session.populate(&catalog, &mut chance);
        assert_eq!(chance.asked.last(), Some(&6), "{:?}", chance.asked);
        assert_eq!(
            session
                .run
                .ship_offer(MissionId(130))
                .and_then(|offer| offer.cargo_type),
            Some(5)
        );
    }

    /// [`linked`] with `AvgShips` 2 of a düde's ship 129, populated with
    /// no navigation target, and its düde ships gone: Ace alone, whom a
    /// ship may join.
    fn awaiting_arrivals() -> (FakePilotCatalog, Session) {
        let mut catalog = linked(2);
        catalog.traffic[0].1.dude_types[0] = (128, 100);
        catalog.dudes = vec![(
            crate::catalog::DudeId(128),
            crate::catalog::DudeRecord {
                ai_type: 1,
                govt: None,
                ships: vec![(crate::catalog::ShipId(129), 1)],
                booty: 0,
                info_types: 0,
            },
        )];
        let mut session = populated(&catalog, Rulebook::default(), None);
        let ships: Vec<NpcId> = session
            .npcs()
            .iter()
            .filter(|npc| npc.person.is_none())
            .map(|npc| npc.id)
            .collect();
        for id in ships {
            session.space.traffic.remove(id);
        }
        (catalog, session)
    }

    #[test]
    fn a_person_jumping_in_mid_stay_resolves_and_overwrites_the_last_offer() {
        let (catalog, mut session) = awaiting_arrivals();
        assert_eq!(offered_to(&session), None, "Ace's");
        session.chart.nav = crate::navigation::NavMode::Stellar(Some(StellarId(129)));
        session.tick_traffic(&catalog, &NovaAi::default(), &mut Draws::of(&[0, 0, 473]));
        let names: Vec<Option<i16>> = session
            .npcs()
            .iter()
            .map(|npc| npc.person.map(|person| person.id.0))
            .collect();
        assert_eq!(names, [Some(600), Some(601)], "Bee jumps in");
        assert_eq!(offered_to(&session), Some(StellarId(129)), "Bee's");
        session.chart.nav = crate::navigation::NavMode::Off;
        session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
        assert_eq!(
            offered_to(&session),
            Some(StellarId(129)),
            "nobody new: kept"
        );
    }

    /// A [`Chance`] answering 0 to every question but the person draw
    /// after the first arrival roll, which picks Bee (`përs` 601): the
    /// setup's person rolls pick nobody, and the arrival roll brings Bee
    /// in by `_HyperShipSpawn`.
    #[derive(Debug, Default)]
    struct BeeArrives {
        rolled: bool,
    }

    impl Chance for BeeArrives {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            if n == crate::traffic::spawn::ARRIVAL_ODDS {
                self.rolled = true;
            }
            if self.rolled && n == crate::person::PERSON_DRAW {
                601 - u32::from(crate::person::FIRST_PERSON.unsigned_abs())
            } else {
                0
            }
        }
    }

    #[test]
    fn a_person_arriving_on_the_gate_exits_pass_has_its_offer_resolved() {
        // 131, the hypergate's exit, holds one ship on average of düde
        // 128, whose setup brings nobody; the exit's pass (@0x672f2)
        // brings Bee.
        let base = linked(0);
        let mut catalog = crate::session::tests::gated();
        catalog.traffic = vec![(
            SystemId(131),
            SystemTraffic {
                dude_types: [
                    (128, 100),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                ],
                avg_ships: 1,
                persons: Default::default(),
            },
        )];
        catalog.dudes = vec![(
            crate::catalog::DudeId(128),
            crate::catalog::DudeRecord {
                ai_type: 1,
                govt: None,
                ships: vec![(crate::catalog::ShipId(129), 1)],
                booty: 0,
                info_types: 0,
            },
        )];
        catalog.persons = base.persons;
        catalog.ship_records.extend(base.ship_records);
        catalog.missions = base.missions;
        let mut session = crate::session::tests::at_gate(&catalog);
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut BeeArrives::default())
            .expect("enters");
        let names: Vec<Option<i16>> = session
            .npcs()
            .iter()
            .map(|npc| npc.person.map(|person| person.id.0))
            .collect();
        assert_eq!(names, [Some(601)], "Bee comes on the pass");
        assert!(
            session.run.ship_offer(MissionId(130)).is_some(),
            "resolved before the first frame"
        );
    }

    #[test]
    fn a_person_whose_mission_has_no_record_resolves_nothing() {
        let mut catalog = linked(0);
        catalog.persons[0].link_mission = Some(131);
        let session = populated(&catalog, Rulebook::default(), None);
        assert_eq!(session.npcs().len(), 1, "Ace is there");
        assert!(session.run.ship_offers.is_empty());
        let mut catalog = linked(0);
        catalog.persons[0].link_mission = None;
        let session = populated(&catalog, Rulebook::default(), None);
        assert!(session.run.ship_offers.is_empty(), "no LinkMission");
    }

    #[test]
    fn a_landing_leaves_the_offers_from_ships_alone() {
        let catalog = linked(0);
        let mut session = populated(&catalog, Rulebook::default(), None);
        assert_eq!(crate::testkit::land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(offered_to(&session), None);
    }

    // Offers on the hail.

    use crate::catalog::MissionTexts;
    use crate::combat::hull::Condition;
    use crate::hail::{HailOptions, HailRefusal};
    use crate::mission::OfferLabels;
    use crate::mission::{OfferPrompt, VenueOffer};
    use crate::person::MISSION_ON_BOARD;

    /// [`linked`] with mission 130's offer text "<OSN> needs a hand,
    /// <PN>." and its `RefuseText` "<OSN> sighs.".
    fn talking() -> FakePilotCatalog {
        FakePilotCatalog {
            mission_texts: vec![MissionTexts {
                offer: Some("<OSN> needs a hand, <PN>.".to_owned()),
                refuse: Some("<OSN> sighs.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Help Ace")
            }],
            ..linked(0)
        }
    }

    /// A pilot named "Stock" flying among [`talking`]'s Ace (NPC 0),
    /// targeted.
    fn targeting_ace(catalog: &FakePilotCatalog) -> Session {
        let pilot = crate::pilot::Pilot::new(catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.populate(catalog, &mut NeverFires);
        session.chart.target = Some(NpcId(0));
        session
    }

    /// Hails the target, drawing on a fresh [`Draws`]: what opened, and
    /// the bounds asked.
    fn hail(
        session: &mut Session,
        catalog: &FakePilotCatalog,
    ) -> (Result<Hailed, HailRefusal>, Vec<u32>) {
        let mut chance = Draws::of(&[]);
        let hailed = session.open_hail(catalog, &HailOptions::default(), &mut chance);
        (hailed, chance.asked)
    }

    fn opened_comm(hailed: &Result<Hailed, HailRefusal>) -> bool {
        matches!(hailed, Ok(Hailed::Comm(_)))
    }

    #[test]
    fn hailing_a_person_whose_mission_is_available_offers_it_in_place_of_the_comm_dialog() {
        let catalog = talking();
        let mut session = targeting_ace(&catalog);
        let (hailed, asked) = hail(&mut session, &catalog);
        assert_eq!(
            hailed,
            Ok(Hailed::Offer(VenueOffer::Prompt(OfferPrompt {
                mission: MissionId(130),
                text: "Ace needs a hand, Stock.".to_owned(),
                labels: OfferLabels {
                    accept: "Yes".to_owned(),
                    refuse: Some("No".to_owned()),
                },
                graphic: None,
                movie: None,
            })))
        );
        assert!(asked.is_empty(), "no conversation rolled: {asked:?}");
        assert!(session.hailing(&catalog, &HailOptions::default()).is_none());
        assert_eq!(
            session.offer_text(MissionId(130), "<OSN>"),
            Some("Ace".to_owned()),
            "the offer stays open"
        );
    }

    #[test]
    fn hailing_opens_the_comm_dialog_when_the_mission_is_not_available() {
        let mut unavailable = talking();
        unavailable.missions[0].avail_random = 50;
        let mut session = targeting_ace(&unavailable);
        session.run.offers.set_roll(MissionId(130), 51);
        let (hailed, asked) = hail(&mut session, &unavailable);
        assert!(opened_comm(&hailed), "{hailed:?}");
        assert!(!asked.is_empty(), "the conversation rolled");
        assert!(
            session
                .hailing(&unavailable, &HailOptions::default())
                .is_some()
        );
        assert_eq!(
            session.offer_text(MissionId(130), "<OSN>"),
            None,
            "no offer"
        );
    }

    #[test]
    fn hailing_opens_the_comm_dialog_for_a_person_offering_on_boarding_an_escort_or_a_ship() {
        let mut boarded = talking();
        boarded.persons[0].flags = MISSION_ON_BOARD;
        let mut session = targeting_ace(&boarded);
        assert!(opened_comm(&hail(&mut session, &boarded).0), "0x0200");
        let catalog = talking();
        let mut session = targeting_ace(&catalog);
        session.space.traffic.npcs_mut()[0].escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        assert!(opened_comm(&hail(&mut session, &catalog).0), "an escort");
        let mut session = targeting_ace(&catalog);
        session.space.traffic.npcs_mut()[0].person = None;
        assert!(opened_comm(&hail(&mut session, &catalog).0), "no person");
        let mut unresolved = talking();
        unresolved.missions.clear();
        let mut session = targeting_ace(&unresolved);
        assert!(opened_comm(&hail(&mut session, &unresolved).0), "no record");
        let mut session = targeting_ace(&catalog);
        session.run.ship_offers.clear();
        assert!(session.ship_mission_available(MissionId(130)));
        assert!(
            opened_comm(&hail(&mut session, &catalog).0),
            "available, but never resolved at a spawn"
        );
    }

    #[test]
    fn the_hails_refusals_come_first_drawing_nothing() {
        let catalog = talking();
        let mut session = targeting_ace(&catalog);
        session.space.traffic.npcs_mut()[0].condition = Condition::Disabled;
        assert_eq!(
            hail(&mut session, &catalog),
            (Err(HailRefusal::NoResponse), vec![])
        );
        let mut session = targeting_ace(&catalog);
        session.chart.target = None;
        assert_eq!(
            hail(&mut session, &catalog),
            (Err(HailRefusal::NoTarget), vec![])
        );
        let mut session = targeting_ace(&catalog);
        session.space.traffic.npcs_mut()[0].mode = crate::traffic::npc::Mode::JumpingIn { tick: 0 };
        assert_eq!(
            hail(&mut session, &catalog),
            (Err(HailRefusal::InHyperspace), vec![])
        );
        let mut session = targeting_ace(&catalog);
        session.space.traffic.npcs_mut()[0].mode = crate::traffic::npc::Mode::JumpingIn { tick: 0 };
        session.space.traffic.npcs_mut()[0].condition = Condition::Disabled;
        assert_eq!(
            hail(&mut session, &catalog),
            (Err(HailRefusal::NoResponse), vec![]),
            "disabled, still jumping in (@0x62610-0x62644)"
        );
    }

    #[test]
    fn an_offer_on_the_hail_beeps() {
        let catalog = talking();
        let mut session = targeting_ace(&catalog);
        session.take_sounds();
        let (hailed, _) = hail(&mut session, &catalog);
        assert!(matches!(hailed, Ok(Hailed::Offer(_))), "{hailed:?}");
        assert_eq!(
            session.take_sounds(),
            [crate::SimSound::HailOffer],
            "_beepSnd[4] (@0x62232-0x6223d)"
        );
    }

    #[test]
    fn an_unrefusable_mission_with_no_offer_text_is_accepted_at_once_on_the_hail() {
        let mut catalog = linked(0);
        catalog.missions[0].flags = 0x0004;
        let mut session = targeting_ace(&catalog);
        assert_eq!(
            hail(&mut session, &catalog).0,
            Ok(Hailed::Offer(VenueOffer::Accepted {
                mission: MissionId(130),
                accepted: Ok(0),
            }))
        );
        assert_eq!(session.pilot().missions().missions(), [MissionId(130)]);
        assert_eq!(session.offer_text(MissionId(130), ""), None, "answered");
    }

    // Answering an offer from a ship.

    use crate::ai::Goal;
    use crate::control::Bit;
    use crate::mission::AcceptRefusal;
    use crate::person::{GONE_ON_ACCEPT, LEAVES_ON_ACCEPT};
    use crate::session::{AcceptError, MissionTextKind, NotOffered};

    /// [`targeting_ace`] of [`talking`] with Ace of `flags`, hailed and
    /// offering mission 130.
    fn offered(flags: u16) -> (FakePilotCatalog, Session) {
        let mut catalog = talking();
        catalog.persons[0].flags = flags;
        let mut session = targeting_ace(&catalog);
        let (hailed, _) = hail(&mut session, &catalog);
        assert!(matches!(hailed, Ok(Hailed::Offer(_))), "{hailed:?}");
        (catalog, session)
    }

    fn accept(session: &mut Session) -> Result<u8, AcceptError> {
        session.accept_mission(MissionId(130), &mut NeverFires)
    }

    #[test]
    fn accepted_on_its_hail_a_person_of_0x0100_is_gone_for_good_and_quiet() {
        let (catalog, mut session) = offered(GONE_ON_ACCEPT);
        assert_eq!(accept(&mut session), Ok(0));
        assert!(session.pilot().gone(PersonId(600)));
        assert_eq!(
            session.npcs()[0].person.map(|person| person.id),
            Some(PersonId(600)),
            "it flies on"
        );
        assert_eq!(session.npcs()[0].goal, Goal::Idle, "and stays");
        let mut quoting = catalog;
        quoting.strings = vec![(7101, vec!["Hi.".to_owned()])];
        session.space.traffic.npcs_mut()[0]
            .person
            .as_mut()
            .expect("Ace")
            .hail_quote = 1;
        let mut chance = Draws::of(&[0]);
        session.tick_quotes(&quoting, &mut chance);
        assert_eq!(session.take_quotes(), [], "gone: no hail quote");
        assert!(chance.asked.is_empty(), "nothing drawn");
    }

    #[test]
    fn accepted_on_its_hail_a_person_of_0x0800_leaves() {
        let (_, mut session) = offered(LEAVES_ON_ACCEPT);
        session.space.traffic.npcs_mut()[0].target = Some(crate::combat::ShipRef::Player);
        assert_eq!(accept(&mut session), Ok(0));
        let ace = &session.npcs()[0];
        assert_eq!((ace.goal, ace.target), (Goal::JumpOut, None));
        assert!(!session.pilot().gone(PersonId(600)));
    }

    #[test]
    fn accepted_without_those_flags_a_person_stays_as_it_was() {
        let (_, mut session) = offered(0);
        let before = session.npcs()[0].clone();
        assert_eq!(accept(&mut session), Ok(0));
        assert_eq!(session.npcs()[0], before);
        assert!(!session.pilot().gone(PersonId(600)));
        assert_eq!(session.run.offer_rolls().roll(MissionId(130)), Some(0));
        assert_eq!(session.offer_text(MissionId(130), ""), None, "answered");
        assert_eq!(
            session.out.take_texts_only().len(),
            0,
            "130 has no BriefText"
        );
    }

    #[test]
    fn in_flight_only_the_mission_the_ship_offers_can_be_answered() {
        let (_, mut session) = offered(0);
        assert_eq!(
            session.accept_mission(MissionId(131), &mut NeverFires),
            Err(AcceptError::NotOffered)
        );
        assert_eq!(
            session.refuse_mission(MissionId(131), &mut NeverFires),
            Err(NotOffered)
        );
        assert_eq!(session.offer_text(MissionId(131), "<OSN>"), None);
        assert_eq!(
            session.offer_text(MissionId(130), "<OSN>"),
            Some("Ace".to_owned()),
            "130 is still offered"
        );
        assert_eq!(accept(&mut session), Ok(0));
    }

    /// Whether mission 130, going to its origin, counts its destination
    /// reached as it is accepted in flight, its offer resolved with the
    /// navigation target `resolved` and accepted with `accepted`, by
    /// `reading`.
    fn reached(reading: RuleSource, resolved: i16, accepted: i16) -> bool {
        let catalog = talking();
        let rules = Rulebook::default().with_override(RuleKey::PersonMissionStellar, reading);
        let mut session = populated(&catalog, rules, Some(resolved));
        session.chart.target = Some(NpcId(0));
        let (hailed, _) = hail(&mut session, &catalog);
        assert!(matches!(hailed, Ok(Hailed::Offer(_))), "{hailed:?}");
        session.chart.nav = crate::navigation::NavMode::Stellar(Some(StellarId(accepted)));
        assert_eq!(accept(&mut session), Ok(0));
        session.pilot().missions().get(0).expect("held").reached
    }

    #[test]
    fn by_the_engine_an_accept_in_flight_reaches_a_destination_that_is_the_nav_target() {
        assert!(reached(RuleSource::Engine, 129, 129));
        assert!(!reached(RuleSource::Engine, 129, 128));
    }

    #[test]
    fn the_accept_at_once_on_the_hail_reaches_no_destination_that_is_the_nav_target() {
        let mut catalog = linked(0);
        catalog.missions[0].flags = 0x0004;
        let mut session = populated(&catalog, Rulebook::default(), Some(129));
        session.chart.target = Some(NpcId(0));
        let (hailed, _) = hail(&mut session, &catalog);
        assert!(
            matches!(
                hailed,
                Ok(Hailed::Offer(VenueOffer::Accepted {
                    accepted: Ok(0),
                    ..
                }))
            ),
            "{hailed:?}"
        );
        let active = session.pilot().missions().get(0).expect("held");
        assert_eq!(
            active.destination.map(|stop| stop.stellar),
            Some(StellarId(129))
        );
        assert!(!active.reached, "the dialog never opened (@0xa230a)");
    }

    #[test]
    fn by_the_other_reading_an_accept_in_flight_reaches_no_destination() {
        assert!(!reached(RuleSource::Bible, 129, 128), "resolved to 128");
    }

    #[test]
    fn refusing_runs_on_refuse_alone_and_a_hail_offers_the_same_values_again() {
        let mut catalog = talking();
        catalog.persons[0].flags = GONE_ON_ACCEPT | LEAVES_ON_ACCEPT;
        catalog.missions[0].on_refuse = crate::control::Script::parse("b5");
        let mut session = targeting_ace(&catalog);
        let (first, _) = hail(&mut session, &catalog);
        let before = session.npcs()[0].clone();
        let offer = session.run.ship_offer(MissionId(130)).cloned();
        assert_eq!(
            session
                .refuse_mission(MissionId(130), &mut NeverFires)
                .map(|_| ()),
            Ok(())
        );
        assert!(session.control_bit(Bit::new(5).expect("a bit")));
        let texts: Vec<String> = session
            .out
            .take_texts_only()
            .into_iter()
            .map(|text| text.text)
            .collect();
        assert_eq!(texts, ["Ace sighs."], "<OSN> during the refusal");
        assert_eq!(session.npcs()[0], before);
        assert!(!session.pilot().gone(PersonId(600)));
        assert!(session.pilot().missions().missions().is_empty());
        assert_eq!(session.offer_text(MissionId(130), ""), None, "answered");
        assert_eq!(hail(&mut session, &catalog).0, first, "offered again");
        assert_eq!(session.run.ship_offer(MissionId(130)).cloned(), offer);
    }

    #[test]
    fn an_accept_refused_for_want_of_room_changes_nothing_and_is_offered_again() {
        let mut catalog = talking();
        catalog.persons[0].flags = GONE_ON_ACCEPT;
        catalog.missions[0].cargo_qty = -60;
        let mut session = targeting_ace(&catalog);
        let (first, _) = hail(&mut session, &catalog);
        assert_eq!(
            accept(&mut session),
            Err(AcceptError::Refused(AcceptRefusal::NoHolds)),
            "89 tons in 20"
        );
        assert!(!session.pilot().gone(PersonId(600)));
        assert!(session.pilot().missions().missions().is_empty());
        assert_eq!(
            session
                .out
                .take_texts_only()
                .into_iter()
                .map(|text| text.kind)
                .collect::<Vec<_>>(),
            [MissionTextKind::NoRoom]
        );
        assert_eq!(session.offer_text(MissionId(130), ""), None, "answered");
        assert_eq!(hail(&mut session, &catalog).0, first, "not suppressed");
    }

    // Offers on boarding.

    use crate::board::{BoardRefusal, Boarding, NovaBoarding};
    use crate::catalog::{GovtId, GovtRecord};
    use crate::combat::ShipRef;
    use crate::legal::NovaLaw;
    use crate::story::{Fate, StoryEvent};

    /// [`talking`] with Ace of the Federation (`gövt` 128, whose boarding
    /// penalty is 5) and of `flags`, 0x0200 among them: it offers mission
    /// 130 on boarding.
    pub(in crate::session) fn boardable(flags: u16) -> FakePilotCatalog {
        let mut catalog = talking();
        catalog.persons[0].flags = MISSION_ON_BOARD | flags;
        catalog.persons[0].govt = Some(GovtId(128));
        catalog.govts = vec![GovtRecord {
            penalties: crate::catalog::Penalties {
                board: 5,
                ..crate::catalog::Penalties::default()
            },
            ..crate::testkit::govt(128)
        }];
        catalog
    }

    /// [`targeting_ace`] with Ace disabled, drifting with the player at
    /// its place, and a second ship (NPC 1) attacking it.
    pub(in crate::session) fn alongside_ace(catalog: &FakePilotCatalog) -> Session {
        let mut session = targeting_ace(catalog);
        let mut state = *session.player();
        state.velocity = crate::geometry::Vec2::new(0.25, 0.0);
        let ace = &mut session.space.traffic.npcs_mut()[0];
        ace.state = state;
        ace.condition = Condition::Disabled;
        let mut other = crate::testkit::npc(9, ace.stats);
        other.target = Some(ShipRef::Npc(NpcId(0)));
        other.goal = Goal::Attack(ShipRef::Npc(NpcId(0)));
        session.space.traffic.add_npc(other);
        session
    }

    /// Boards by Nova's law and boarding, drawing on a fresh [`Draws`]:
    /// what it did, and the bounds asked.
    pub(in crate::session) fn board(
        session: &mut Session,
    ) -> (Result<Boarding, BoardRefusal>, Vec<u32>) {
        let mut chance = Draws::of(&[]);
        let boarded = session.board(&NovaLaw::default(), &NovaBoarding::default(), &mut chance);
        (boarded, chance.asked)
    }

    fn boarded_fates(session: &Session) -> usize {
        session
            .story_raised()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    StoryEvent::ShipFate {
                        fate: Fate::Boarded,
                        ..
                    }
                )
            })
            .count()
    }

    #[test]
    fn boarding_a_person_of_0x0200_offers_its_mission_in_place_of_the_plunder() {
        let catalog = boardable(0);
        let mut session = alongside_ace(&catalog);
        let (boarded, asked) = board(&mut session);
        let Ok(Boarding::Offered(VenueOffer::Prompt(prompt))) = boarded else {
            panic!("an offer: {boarded:?}");
        };
        assert_eq!(prompt.text, "Ace needs a hand, Stock.");
        assert!(asked.is_empty(), "no plunder, odds or grant: {asked:?}");
        assert_eq!(session.pilot().legal_record(GovtId(128)), -5, "the crime");
        assert_eq!(
            session.player().velocity,
            crate::geometry::Vec2::new(0.25, 0.0)
        );
        assert!(session.boarding().is_none(), "no plunder dialog");
        assert_eq!(session.take_grant(), None);
        assert!(!session.npcs()[0].boarded, "not marked");
        assert_eq!(boarded_fates(&session), 0);
        assert_eq!(
            session.npcs()[1].target,
            Some(ShipRef::Npc(NpcId(0))),
            "still hunted"
        );
    }

    #[test]
    fn accepting_on_boarding_marks_the_ship_boarded_and_lets_the_others_go() {
        let catalog = boardable(LEAVES_ON_ACCEPT);
        let mut session = alongside_ace(&catalog);
        let _ = board(&mut session);
        let before = session.npcs()[0].goal;
        assert_eq!(accept(&mut session), Ok(0));
        assert!(session.npcs()[0].boarded);
        assert_eq!(boarded_fates(&session), 1);
        let other = &session.npcs()[1];
        assert_eq!((other.target, other.goal), (None, Goal::Idle), "let go");
        assert_eq!(session.npcs()[0].goal, before, "0x0800 is not read");
        assert!(!session.pilot().gone(PersonId(600)));
        assert_eq!(
            board(&mut session).0,
            Err(BoardRefusal::CantBoard),
            "boarded before"
        );
    }

    #[test]
    fn refusing_on_boarding_leaves_it_boardable_and_offered_again() {
        let catalog = boardable(0);
        let mut session = alongside_ace(&catalog);
        let (first, _) = board(&mut session);
        assert_eq!(
            session
                .refuse_mission(MissionId(130), &mut NeverFires)
                .map(|_| ()),
            Ok(())
        );
        assert!(!session.npcs()[0].boarded);
        assert_eq!(boarded_fates(&session), 0);
        assert_eq!(board(&mut session).0, first);
    }

    #[test]
    fn on_boarding_0x0100_makes_the_person_gone_whatever_the_answer() {
        let catalog = boardable(GONE_ON_ACCEPT);
        let mut session = alongside_ace(&catalog);
        let _ = board(&mut session);
        assert_eq!(
            session
                .refuse_mission(MissionId(130), &mut NeverFires)
                .map(|_| ()),
            Ok(())
        );
        assert!(session.pilot().gone(PersonId(600)), "refused");
        let mut session = alongside_ace(&catalog);
        let _ = board(&mut session);
        assert_eq!(accept(&mut session), Ok(0));
        assert!(session.pilot().gone(PersonId(600)), "accepted");
    }

    // `<OSN>`: `_offeringShipIndex`.

    /// `session` reading `<OSN>` as `rule` says.
    fn osn(session: &Session, rule: RuleSource) -> String {
        session
            .clone()
            .with_rules(Rulebook::default().with_override(RuleKey::OfferingShipName, rule))
            .plain_text("<OSN>")
    }

    #[test]
    fn a_new_session_names_no_offering_ship() {
        let catalog = talking();
        let session = targeting_ace(&catalog);
        assert_eq!(
            osn(&session, RuleSource::Engine),
            "[Error]",
            "_InitMissions @0x97bbf"
        );
        assert_eq!(osn(&session, RuleSource::Bible), "[Error]");
    }

    #[test]
    fn by_the_engine_osn_names_the_last_hailed_person_after_the_dialog() {
        let (_, mut session) = offered(0);
        assert_eq!(osn(&session, RuleSource::Engine), "Ace", "while offering");
        assert_eq!(osn(&session, RuleSource::Bible), "Ace", "while offering");
        assert_eq!(
            session
                .refuse_mission(MissionId(130), &mut NeverFires)
                .map(|_| ()),
            Ok(())
        );
        assert_eq!(
            osn(&session, RuleSource::Engine),
            "Ace",
            "kept after the dialog (@0x625a4)"
        );
        assert_eq!(
            osn(&session, RuleSource::Bible),
            "[Error]",
            "only when offering a mission from a ship"
        );
    }

    #[test]
    fn by_the_engine_hailing_a_linked_person_names_it_whether_or_not_it_offers() {
        let mut unavailable = talking();
        unavailable.missions[0].avail_random = 50;
        let mut session = targeting_ace(&unavailable);
        session.run.offers.set_roll(MissionId(130), 51);
        assert!(opened_comm(&hail(&mut session, &unavailable).0));
        assert_eq!(osn(&session, RuleSource::Engine), "Ace", "@0x621de");
        assert_eq!(osn(&session, RuleSource::Bible), "[Error]");
    }

    #[test]
    fn hailing_a_person_offering_on_boarding_or_with_no_mission_names_nobody() {
        let mut boarded = talking();
        boarded.persons[0].flags = MISSION_ON_BOARD;
        let mut session = targeting_ace(&boarded);
        assert!(opened_comm(&hail(&mut session, &boarded).0));
        assert_eq!(osn(&session, RuleSource::Engine), "[Error]", "0x0200");
        let mut unlinked = talking();
        unlinked.persons[0].link_mission = None;
        let mut session = targeting_ace(&unlinked);
        assert!(opened_comm(&hail(&mut session, &unlinked).0));
        assert_eq!(
            osn(&session, RuleSource::Engine),
            "[Error]",
            "no LinkMission"
        );
    }

    #[test]
    fn by_the_engine_osn_reads_whoever_holds_the_hailed_ships_slot_now() {
        let (_, mut session) = offered(0);
        assert_eq!(
            session
                .refuse_mission(MissionId(130), &mut NeverFires)
                .map(|_| ()),
            Ok(())
        );
        session.space.traffic.remove(NpcId(0));
        assert_eq!(
            osn(&session, RuleSource::Engine),
            "Ace",
            "a slot left empty still holds what it last held"
        );
        let newcomer = session
            .space
            .traffic
            .add_npc(crate::testkit::npc(0, crate::stats::ShipStats::default()));
        assert_eq!(
            osn(&session, RuleSource::Engine),
            "[Error]",
            "reused by a ship with no person (-1 @0x3cb48)"
        );
        session.space.traffic.remove(newcomer);
        assert_eq!(
            osn(&session, RuleSource::Engine),
            "[Error]",
            "left empty again: the last ship it held, not Ace"
        );
    }

    #[test]
    fn by_the_engine_a_reused_slot_names_its_newcomers_person() {
        let (_, mut session) = offered(0);
        let ace = session.npcs()[0].clone();
        session.space.traffic.remove(ace.id);
        let _ = session
            .space
            .traffic
            .add_npc(crate::testkit::npc(0, crate::stats::ShipStats::default()));
        session.space.traffic.remove(NpcId(1));
        session.space.traffic.add_npc(ace);
        assert_eq!(osn(&session, RuleSource::Engine), "Ace", "@0x40cef");
    }

    #[test]
    fn by_the_engine_a_captured_ships_slot_names_nobody() {
        let (_, mut session) = offered(0);
        session
            .space
            .lose_captured(&mut session.pilot, NpcId(0), true);
        assert_eq!(
            osn(&session, RuleSource::Engine),
            "[Error]",
            "_DoShipCapture @0x411b3"
        );
        session.space.traffic.remove(NpcId(0));
        assert_eq!(
            osn(&session, RuleSource::Engine),
            "[Error]",
            "nor once gone"
        );
    }

    #[test]
    fn by_the_engine_a_board_offer_names_the_boarded_person_then_nobody() {
        let catalog = boardable(0);
        let mut session = alongside_ace(&catalog);
        let _ = board(&mut session);
        assert_eq!(osn(&session, RuleSource::Engine), "Ace", "@0x65882");
        assert_eq!(
            session
                .refuse_mission(MissionId(130), &mut NeverFires)
                .map(|_| ()),
            Ok(())
        );
        assert_eq!(osn(&session, RuleSource::Engine), "[Error]", "-1 @0x65a11");
    }

    #[test]
    fn by_the_engine_boarding_a_linked_person_with_no_offer_forgets_the_hailed_one() {
        let mut unavailable = boardable(0);
        unavailable.missions[0].avail_random = 50;
        let mut session = alongside_ace(&unavailable);
        session.space.traffic.set_berth_person(99, "Earlier");
        session.run.offering_ship = Some(OfferingShip { berth: 99 });
        session.run.offers.set_roll(MissionId(130), 51);
        let (boarded, _) = board(&mut session);
        assert!(matches!(boarded, Ok(Boarding::Opened(_))), "{boarded:?}");
        assert_eq!(osn(&session, RuleSource::Engine), "[Error]");
    }

    #[test]
    fn boarding_a_linked_person_writes_its_missions_avail_bits_verdict() {
        let catalog = boardable(0);
        let mut session = alongside_ace(&catalog);
        session.run.avail_cache.insert(MissionId(130), false);
        let _ = board(&mut session);
        assert_eq!(
            session.run.avail_cache.get(&MissionId(130)),
            Some(&true),
            "_IsMissionAvailable(m, 1) @0x658b1"
        );
    }

    #[test]
    fn with_the_mission_unavailable_or_without_0x0200_the_plunder_dialog_opens() {
        let mut unavailable = boardable(0);
        unavailable.missions[0].avail_random = 50;
        let mut session = alongside_ace(&unavailable);
        session.run.offers.set_roll(MissionId(130), 51);
        let (boarded, _) = board(&mut session);
        assert!(matches!(boarded, Ok(Boarding::Opened(_))), "{boarded:?}");
        assert!(session.npcs()[0].boarded);
        let mut hailed = boardable(0);
        hailed.persons[0].flags = 0;
        let mut session = alongside_ace(&hailed);
        let (boarded, _) = board(&mut session);
        assert!(matches!(boarded, Ok(Boarding::Opened(_))), "{boarded:?}");
    }

    // The sounds the offers from ships make.

    /// The offer sounds among the sounds raised since the last taken.
    fn offer_sounds(session: &mut Session) -> Vec<SimSound> {
        session
            .take_sounds()
            .into_iter()
            .filter(|sound| matches!(sound, SimSound::HailOffer | SimSound::PersonOfferBoarded))
            .collect()
    }

    #[test]
    fn a_hailed_offer_beeps_and_a_boarded_one_plays_the_airlock() {
        let catalog = talking();
        let mut session = targeting_ace(&catalog);
        let _ = session.take_sounds();
        let _ = hail(&mut session, &catalog);
        assert_eq!(
            offer_sounds(&mut session),
            [SimSound::HailOffer],
            "_beepSnd[4] (@0x6223d)"
        );
        let catalog = boardable(0);
        let mut session = alongside_ace(&catalog);
        let _ = session.take_sounds();
        let _ = board(&mut session);
        assert_eq!(
            offer_sounds(&mut session),
            [SimSound::PersonOfferBoarded],
            "snd 390 (@0x65907)"
        );
    }

    #[test]
    fn an_offer_accepted_at_once_still_sounds() {
        let mut catalog = linked(0);
        catalog.missions[0].flags = 0x0004;
        let mut session = targeting_ace(&catalog);
        let _ = session.take_sounds();
        assert!(matches!(
            hail(&mut session, &catalog).0,
            Ok(Hailed::Offer(VenueOffer::Accepted { .. }))
        ));
        assert_eq!(
            offer_sounds(&mut session),
            [SimSound::HailOffer],
            "played before _OfferOneMission"
        );
    }

    /// How many airlocks for a plunder are among the sounds raised since
    /// the last taken.
    fn plunder_airlocks(session: &mut Session) -> usize {
        session
            .take_sounds()
            .into_iter()
            .filter(|sound| *sound == SimSound::Boarded)
            .count()
    }

    #[test]
    fn a_boarded_offer_plays_the_offers_airlock_alone() {
        let catalog = boardable(0);
        let mut session = alongside_ace(&catalog);
        let _ = session.take_sounds();
        assert!(matches!(board(&mut session).0, Ok(Boarding::Offered(_))));
        assert_eq!(
            session.take_sounds(),
            [SimSound::PersonOfferBoarded],
            "the offer jumps past @0x65991"
        );
    }

    #[test]
    fn a_named_person_plundered_plays_the_airlock() {
        let mut unavailable = boardable(0);
        unavailable.missions[0].avail_random = 50;
        let mut session = alongside_ace(&unavailable);
        session.run.offers.set_roll(MissionId(130), 51);
        let _ = session.take_sounds();
        assert!(matches!(board(&mut session).0, Ok(Boarding::Opened(_))));
        assert_eq!(plunder_airlocks(&mut session), 1, "@0x659b3");
        let mut linked_only = boardable(0);
        linked_only.persons[0].flags = 0;
        let mut session = alongside_ace(&linked_only);
        let _ = session.take_sounds();
        assert!(matches!(board(&mut session).0, Ok(Boarding::Opened(_))));
        assert_eq!(plunder_airlocks(&mut session), 1, "@0x65a3f → @0x65f00");
    }

    #[test]
    fn a_hail_or_board_with_no_offer_makes_neither_sound() {
        let mut unavailable = talking();
        unavailable.missions[0].avail_random = 50;
        let mut session = targeting_ace(&unavailable);
        session.run.offers.set_roll(MissionId(130), 51);
        let _ = session.take_sounds();
        assert!(opened_comm(&hail(&mut session, &unavailable).0));
        assert_eq!(offer_sounds(&mut session), [], "the comm dialog");
        let mut hailed = boardable(0);
        hailed.persons[0].flags = 0;
        let mut session = alongside_ace(&hailed);
        let _ = session.take_sounds();
        assert!(matches!(board(&mut session).0, Ok(Boarding::Opened(_))));
        assert_eq!(offer_sounds(&mut session), [], "the plunder dialog");
    }
}
