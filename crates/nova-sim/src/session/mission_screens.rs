//! The session's side of the mission screens (see the
//! [`mission`](crate::mission) module's "Screens"), each a plain value the
//! view draws, as [`HireList`](crate::HireList) is the bar's hire screen:
//!
//! - what each mission's screens show of it, read as the session starts
//!   ([`Session::mission_texts`]);
//! - the mission computer's view of the visit's offers
//!   ([`Session::mission_computer`], `_DoMissionDialog` @0xa37cf), or why
//!   it does not open;
//! - a venue's next offer outside the computer, at the bar, spaceport,
//!   trade center, shipyard or outfitter ([`Session::next_offer`],
//!   `_OfferMissionFromPort` @0xa369e), which enters the venue, with the
//!   offers suppressed after a refused accept until another venue is
//!   entered ([`Session::enter_offer_venue`]);
//! - the mission info screen's view of the active missions
//!   ([`Session::mission_info`], `_DoMissionInfoDialog` @0x9e19b);
//! - the queue of mission texts to show ([`Session::take_mission_texts`]):
//!   a mission's `BriefText`, `CompText`, `FailText` and `RefuseText`,
//!   queued at the lifecycle's moments (see the `missions` module), its
//!   `ShipDoneText` as its ship goals are met (see the `mission_ships`
//!   module), and an accept refused for want of room for its cargo.
//!
//! None of it is saved. The next outside offer the original drops after
//! an accept is rdm:phase/missions/phase-15-accept-drops-next-offer.

use super::Session;
use super::missions::TextSource;
use crate::catalog::{MissionId, MissionTexts};
use crate::chance::{Chance, NeverFires};
use crate::mission::screens::{HIDDEN, NO_MISSIONS_HERE, SPACEPORT, UNREFUSABLE};
use crate::mission::{
    AcceptRefusal, AvailFacts, Offer, OfferPrompt, Venue, VenueOffer, availability, offer_labels,
};

/// One offer the mission computer lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComputerRow {
    /// The mission.
    pub mission: MissionId,
    /// Its name, its tags filled for the offer.
    pub name: String,
    /// Its offer text, its tags filled for the offer: empty with none.
    pub text: String,
}

/// The mission computer, as the player sees it (`DLOG` 1006).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionComputer {
    /// The offers, in the order they show.
    pub rows: Vec<ComputerRow>,
    /// Today, as the medium date.
    pub today: String,
}

/// Why the mission computer does not open (`_DoMissionDialog`
/// @0xa37e9-0xa3956).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComputerRefusal {
    /// Every mission slot is taken.
    SlotsFull,
    /// No offer it lists is available now, or the ship is not docked.
    NoneAvailable,
}

impl ComputerRefusal {
    /// What the original says: `STR#` 2002 #351/#352 with every slot
    /// taken, #353 with nothing available.
    #[must_use]
    pub fn message(self) -> String {
        match self {
            Self::SlotsFull => AcceptRefusal::SlotsFull.message(),
            Self::NoneAvailable => NO_MISSIONS_HERE.to_owned(),
        }
    }
}

/// Which of a mission's texts is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionTextKind {
    /// Its `BriefText`, as it is accepted.
    Brief,
    /// Its `CompText`, as it is completed.
    Comp,
    /// Its `FailText`, as it fails at its return.
    Fail,
    /// Its `RefuseText`, as its offer is refused.
    Refuse,
    /// Why it has no room for its cargo: `STR#` 2002 #355 or #356 as its
    /// accept is refused.
    NoRoom,
    /// Its `LoadCargText`, as its cargo is loaded.
    LoadCargo,
    /// Its `DumpCargoText`, as its cargo is unloaded.
    DumpCargo,
    /// Its `ShipDoneText`, as its ship goals are met.
    ShipDone,
}

impl MissionTextKind {
    /// This text of `texts`, as written, if it has one.
    fn of(self, texts: &MissionTexts) -> Option<&str> {
        match self {
            Self::Brief => texts.brief.as_deref(),
            Self::Comp => texts.comp.as_deref(),
            Self::Fail => texts.fail.as_deref(),
            Self::Refuse => texts.refuse.as_deref(),
            Self::LoadCargo => texts.load_cargo.as_deref(),
            Self::DumpCargo => texts.dump_cargo.as_deref(),
            Self::ShipDone => texts.ship_done.as_deref(),
            Self::NoRoom => None,
        }
    }
}

/// A mission text to show in the "Desc Dialog" (`_ShowTextDialog`
/// @0x5a752), its tags filled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionText {
    /// Which text it is.
    pub kind: MissionTextKind,
    /// The mission.
    pub mission: MissionId,
    /// The text.
    pub text: String,
}

/// One active mission the mission info screen lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfoRow {
    /// Its slot.
    pub slot: u8,
    /// Its name, its tags filled for the slot.
    pub name: String,
    /// Whether it has failed: the screen marks it.
    pub failed: bool,
    /// Its `QuickBrief`, its tags filled for the slot: empty with none.
    pub text: String,
    /// Whether the player can abort it: its `CanAbort` is not 0.
    pub abortable: bool,
}

/// The mission info screen, as the player sees it (`DLOG` 1012).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionInfo {
    /// The active missions shown, in slot order.
    pub rows: Vec<InfoRow>,
    /// Today, as the medium date.
    pub today: String,
}

impl Session {
    /// What the screens show of mission `id`, as the session read it as
    /// it started: `None` for a `mïsn` it has no texts of.
    #[must_use]
    pub fn mission_texts(&self, id: MissionId) -> Option<&MissionTexts> {
        self.mission_texts.get(&id)
    }

    /// The mission texts to show since they were last taken, in the order
    /// they came.
    pub fn take_mission_texts(&mut self) -> Vec<MissionText> {
        std::mem::take(&mut self.shown_texts)
    }

    /// Queues `mission`'s `kind` text to show, when it has one, taken in
    /// as a `dësc` ([`TextSource::Desc`], drawing on `chance`) and filled for `slot`
    /// ([`Session::mission_text`]), or with no mission
    /// ([`Session::plain_text`]): whether it had one.
    pub(super) fn show_mission_text(
        &mut self,
        kind: MissionTextKind,
        mission: MissionId,
        slot: Option<u8>,
        chance: &mut dyn Chance,
    ) -> bool {
        let Some(raw) = self
            .mission_texts
            .get(&mission)
            .and_then(|texts| kind.of(texts))
            .map(str::to_owned)
        else {
            return false;
        };
        let raw = self.take_in(TextSource::Desc, &raw, chance);
        let text = match slot {
            Some(slot) => self.mission_text(slot, &raw),
            None => self.plain_text(&raw),
        };
        self.show_text(kind, mission, text);
        true
    }

    /// Queues `text`, `mission`'s `kind` text, to show as it is.
    pub(super) fn show_text(&mut self, kind: MissionTextKind, mission: MissionId, text: String) {
        self.shown_texts.push(MissionText {
            kind,
            mission,
            text,
        });
    }

    /// The mission computer of the stellar docked at (`_DoMissionDialog`
    /// @0xa37cf): every offer at the computer in the order it shows, with
    /// no availability check (`_SetupMissionList` @0xa08c4-0xa09d6), each
    /// name and text filled for its offer, and today's medium date. The
    /// names read the remembered rank governments, as the original's list
    /// munges them before any text is loaded (@0xa0947); each text reads
    /// its own, as if loaded, changing nothing: the screen loads the
    /// selected one ([`Session::show_computer_row`]).
    ///
    /// # Errors
    ///
    /// [`ComputerRefusal::SlotsFull`] with every slot taken
    /// (@0xa37e9-0xa38b8); then [`ComputerRefusal::NoneAvailable`] in
    /// flight, or when none of the offers is available now
    /// (`_IsMissionAvailable(i, 1)` @0xa38bd-0xa3956), checked as an
    /// accept leaves them, drawing nothing.
    pub fn mission_computer(&self) -> Result<MissionComputer, ComputerRefusal> {
        if self.accept_refusal(0, &mut NeverFires) == Err(AcceptRefusal::SlotsFull) {
            return Err(ComputerRefusal::SlotsFull);
        }
        let offers = self.mission_offers(Venue::Computer);
        if !offers.iter().any(|offer| self.still_available(offer)) {
            return Err(ComputerRefusal::NoneAvailable);
        }
        let rows = offers
            .iter()
            .map(|offer| {
                let texts = self.mission_texts(offer.mission);
                let name = texts.map_or("", |texts| texts.name.as_str());
                let text = texts
                    .and_then(|texts| texts.offer.as_deref())
                    .unwrap_or_default();
                let loaded = TextSource::Desc.govts(self.rank_govts, text);
                ComputerRow {
                    mission: offer.mission,
                    name: self.offer_text(offer.mission, name).unwrap_or_default(),
                    text: self
                        .offer_text_reading(offer.mission, text, loaded)
                        .unwrap_or_default(),
                }
            })
            .collect();
        Ok(MissionComputer {
            rows,
            today: self.med_date_text(),
        })
    }

    /// The mission info screen (`_DoMissionInfoDialog` @0x9e19b): each
    /// active slot whose mission lacks `Flags` 0x0400, in slot order
    /// (`_SetupMissionInfoList` @0x9de7a), its name and `QuickBrief`
    /// filled for the slot (@0x9e2e1-0x9e309), and today's medium date. A
    /// slot whose `mïsn` is gone reads with no text, and abortable. The
    /// names read the remembered rank governments, as the original's list
    /// munges them before any text is loaded (@0x9e108); each text reads
    /// its own, as if loaded, changing nothing: the screen loads the
    /// selected one ([`Session::show_info_row`]).
    #[must_use]
    pub fn mission_info(&self) -> MissionInfo {
        let rows = self
            .pilot
            .missions
            .iter()
            .filter_map(|(slot, active)| {
                let record = self.lifecycle_record(active.mission);
                if record.flags & HIDDEN != 0 {
                    return None;
                }
                let texts = self.mission_texts(active.mission);
                let name = texts.map_or("", |texts| texts.name.as_str());
                let text = texts
                    .and_then(|texts| texts.quick_brief.as_deref())
                    .unwrap_or_default();
                let loaded = TextSource::Desc.govts(self.rank_govts, text);
                Some(InfoRow {
                    slot,
                    name: self.mission_text(slot, name),
                    failed: active.failed.is_some(),
                    text: self.mission_text_reading(slot, text, loaded),
                    abortable: record.can_abort != 0,
                })
            })
            .collect();
        MissionInfo {
            rows,
            today: self.med_date_text(),
        }
    }

    /// Shows the mission computer's row of `mission`, selected: its offer
    /// text loaded as a `dësc` and filled for its offer, as
    /// `_MissionFilter` loads a selected row's (@0x9f9bd-0x9f9eb), and
    /// then its name filled, as `_MissionDialogUpdate` munges the
    /// selected name after (@0x9f40c), drawing on `chance`. `None` when
    /// it is not offered.
    pub fn show_computer_row(
        &mut self,
        mission: MissionId,
        chance: &mut dyn Chance,
    ) -> Option<ComputerRow> {
        self.offer_of(mission)?;
        let texts = self.mission_texts(mission);
        let name = texts.map_or("", |texts| texts.name.as_str()).to_owned();
        let raw = texts
            .and_then(|texts| texts.offer.clone())
            .unwrap_or_default();
        let raw = self.take_in(TextSource::Desc, &raw, chance);
        Some(ComputerRow {
            mission,
            text: self.offer_text(mission, &raw)?,
            name: self.offer_text(mission, &name)?,
        })
    }

    /// Shows the mission info screen's row of `slot`, selected: its
    /// `QuickBrief` loaded as a `dësc` and filled for the slot, as
    /// `_MissionInfoFilter` loads a selected row's (@0x9d89f-0x9d8bc),
    /// drawing on `chance`. `None` for a free slot.
    pub fn show_info_row(&mut self, slot: u8, chance: &mut dyn Chance) -> Option<String> {
        let active = self.pilot.missions.get(slot)?;
        let raw = self
            .mission_texts(active.mission)
            .and_then(|texts| texts.quick_brief.clone())
            .unwrap_or_default();
        let raw = self.take_in(TextSource::Desc, &raw, chance);
        Some(self.mission_text(slot, &raw))
    }

    /// The next offer at venue `loc` (an `AvailLoc` other than the mission
    /// computer's: [`BAR`](crate::mission::screens::BAR),
    /// [`SPACEPORT`], [`TRADE_CENTER`](crate::mission::screens::TRADE_CENTER),
    /// [`SHIPYARD`](crate::mission::screens::SHIPYARD) or
    /// [`OUTFITTER`](crate::mission::screens::OUTFITTER)), drawing on
    /// `chance`, as `_OfferMissionFromPort` (@0xa369e) offers one. It
    /// enters the venue first ([`Session::enter_offer_venue`]), as that
    /// routine does (@0xa36a7-0xa36d7), then offers the first outside
    /// offer, in the order they show, whose `AvailLoc` is `loc`, that is
    /// not suppressed, and that is still available, checked as an accept
    /// leaves them, drawing nothing (`_IsMissionAvailable(i, 1)`,
    /// @0xa36da-0xa3742). One that cannot be refused, with no offer text,
    /// is accepted at once. `None` with nothing to offer, or in flight.
    pub fn next_offer(&mut self, loc: i16, chance: &mut dyn Chance) -> Option<VenueOffer> {
        self.enter_offer_venue(loc);
        let offer = self
            .mission_offers(Venue::Outside)
            .iter()
            .find(|offer| {
                self.mission_record(offer.mission)
                    .is_some_and(|record| record.avail_loc == loc)
                    && !self.suppressed.contains(&offer.mission)
                    && self.still_available(offer)
            })?
            .mission;
        Some(self.offer_dialog(offer, chance))
    }

    /// The offer dialog for `mission`, offered now (`_OfferOneMission`
    /// @0xa21ac), drawing on `chance`: its offer text filled for the
    /// offer, and its labels; or, for one that cannot be refused with no
    /// offer text, its accept at once (@0xa22ec-0xa231c), made before the
    /// dialog is, so with no offer dialog open
    /// ([`Session::accept_offered`]), and its offer then dropped when it
    /// succeeds, as `_OfferMissionFromPort` drops it (@0xa3765-0xa37a1).
    pub(super) fn offer_dialog(
        &mut self,
        mission: MissionId,
        chance: &mut dyn Chance,
    ) -> VenueOffer {
        let can_refuse = self
            .mission_record(mission)
            .is_some_and(|record| record.flags & UNREFUSABLE == 0);
        let texts = self.mission_texts(mission);
        let raw = texts
            .and_then(|texts| texts.offer.as_deref())
            .unwrap_or_default();
        if !can_refuse && raw.is_empty() {
            let accepted = self.accept_offered(mission, chance);
            if accepted.is_ok() {
                // `_OfferMissionFromPort` drops the offer whenever
                // `_OfferOneMission` returns other than -1 (@0xa3765-0xa37a1),
                // and the accept at once returns 1 once it succeeds
                // (@0xa2311, @0xa368e), though `_DoMissionAccept` refilters
                // nothing with the dialog closed (@0xa200b).
                self.offers.drop_offer(mission);
            }
            return VenueOffer::Accepted { mission, accepted };
        }
        let labels = offer_labels(
            texts.map_or("", |texts| texts.accept_button.as_str()),
            texts.map_or("", |texts| texts.refuse_button.as_str()),
            can_refuse,
        );
        let raw = raw.to_owned();
        let raw = self.take_in(TextSource::Desc, &raw, chance);
        VenueOffer::Prompt(OfferPrompt {
            mission,
            text: self.offer_text(mission, &raw).unwrap_or_default(),
            labels,
        })
    }

    /// Enters venue `loc` for its offers (`_OfferMissionFromPort`
    /// @0xa36a7-0xa36d7): entering one other than the last forgets the
    /// suppressed offers, unless it is the spaceport ([`SPACEPORT`]).
    pub fn enter_offer_venue(&mut self, loc: i16) {
        if self.offer_venue != Some(loc) {
            if loc != SPACEPORT {
                self.suppressed.clear();
            }
            self.offer_venue = Some(loc);
        }
    }

    /// Suppresses the offer of `mission`, whose accept was refused, until
    /// another venue is entered (`_OfferMissionFromPort` @0xa3765).
    pub(super) fn suppress_offer(&mut self, mission: MissionId) {
        self.suppressed.insert(mission);
    }

    /// Whether `offer`, on the visit under way, is still available at its
    /// venue, the visit's rolls read and nothing drawn, as
    /// `_IsMissionAvailable(i, 1)` checks it: never in flight, nor for a
    /// mission the session has no record of.
    fn still_available(&self, offer: &Offer) -> bool {
        let (Some(stellar), Some(record)) = (self.landed, self.mission_record(offer.mission))
        else {
            return false;
        };
        self.at_visit(stellar, |visit| {
            let facts = AvailFacts {
                visit,
                mission: record,
                venue: offer.venue,
                rolls: self.offers.table(),
            };
            availability()
                .first_refusal(&facts, &mut NeverFires)
                .is_ok()
        })
        .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{DateAffixes, MissionRecord, PlaceNames, StellarId, SystemId};
    use crate::control::{Bit, Script, SetExpr, Test};
    use crate::geometry::Vec2;
    use crate::mission::AbortRefusal;
    use crate::mission::screens::{BAR, NO_LABEL, OKAY_LABEL, OUTFITTER, SHIPYARD, TRADE_CENTER};
    use crate::mission::{AcceptError, ActiveMission, OfferLabels, SLOTS};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::AbortError;
    use crate::testkit::{Draws, FakePilotCatalog, catalog, land_now, mission};

    /// Stellars 128 Earth, 129 Luna and 140 Mars; systems 130 Sol and 131
    /// Alpha.
    fn names() -> PlaceNames {
        PlaceNames {
            stellars: [(128, "Earth"), (129, "Luna"), (140, "Mars")]
                .into_iter()
                .map(|(id, name)| (StellarId(id), name.to_owned()))
                .collect(),
            systems: [(130, "Sol"), (131, "Alpha")]
                .into_iter()
                .map(|(id, name)| (SystemId(id), name.to_owned()))
                .collect(),
        }
    }

    /// [`catalog`], named, with 131 linked back to 130, `missions` and
    /// `texts`.
    fn screens(missions: Vec<MissionRecord>, texts: Vec<MissionTexts>) -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.star_map[1].links.push(SystemId(130));
        catalog.place_names = names();
        catalog.missions = missions;
        catalog.mission_texts = texts;
        catalog
    }

    fn settle(session: &mut Session, catalog: &FakePilotCatalog) {
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    /// A session of `catalog` landed on 128 and settled, every roll 1.
    fn docked(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, catalog);
        session
    }

    /// Mission `id`, offered at the mission computer, to Mars for 900.
    fn to_mars(id: i16) -> MissionRecord {
        MissionRecord {
            travel_stel: 140,
            pay_val: 900,
            ..mission(id)
        }
    }

    /// Mission `id`'s texts: named `name`, offering `offer`.
    fn texts(id: i16, name: &str, offer: Option<&str>) -> MissionTexts {
        MissionTexts {
            offer: offer.map(str::to_owned),
            ..MissionTexts::named(MissionId(id), name)
        }
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    /// [`screens`] of `missions` and `texts`, with ranks 200 (government
    /// 128, "Cdr") and 201 (129, "Ad"), docked and both held.
    fn ranked(missions: Vec<MissionRecord>, texts: Vec<MissionTexts>, rules: Rulebook) -> Session {
        let mut catalog = screens(missions, texts);
        let rank = |id, govt, short: &str| crate::catalog::RankRecord {
            short_name: short.to_owned(),
            affil_govt: Some(crate::catalog::GovtId(govt)),
            ..crate::testkit::rank(id)
        };
        catalog.ranks = vec![rank(200, 128, "Cdr"), rank(201, 129, "Ad")];
        let mut session = docked(&catalog).with_rules(rules);
        session.run_set(
            &SetExpr::parse("K200 K201").expect("parses"),
            &mut NeverFires,
        );
        session
    }

    #[test]
    fn by_the_engine_the_computers_names_read_the_governments_left_and_each_text_its_own() {
        let mut session = ranked(
            vec![to_mars(130)],
            vec![texts(130, "Run <PRK128>", Some("Go, <PRK128>."))],
            Rulebook::default(),
        );
        let _ = session.take_in(
            TextSource::Desc,
            "Earlier: <PRK128> <SRK129>.",
            &mut NeverFires,
        );
        let computer = session.mission_computer().expect("opens");
        assert_eq!(
            (
                computer.rows[0].name.as_str(),
                computer.rows[0].text.as_str()
            ),
            ("Run Ad", "Go, captain."),
            "the list's names are munged before any text loads (@0xa0947); the \
             text as if loaded"
        );
        assert_eq!(
            session.plain_text("<PRK128>"),
            "Ad",
            "the computer's view changes nothing"
        );
        let shown = session
            .show_computer_row(MissionId(130), &mut NeverFires)
            .expect("offered");
        assert_eq!(
            (shown.name.as_str(), shown.text.as_str()),
            ("Run captain", "Go, captain."),
            "selected: its text loaded (@0x9f9bd), then its name munged again (@0x9f40c)"
        );
        assert_eq!(
            session.plain_text("<PRK128>"),
            "captain",
            "left by the load"
        );
        assert_eq!(
            session.mission_computer().expect("opens").rows[0].name,
            "Run captain",
            "opened again, the names read what the last load left"
        );
        assert_eq!(
            session.show_computer_row(MissionId(131), &mut NeverFires),
            None,
            "not offered"
        );
    }

    #[test]
    fn by_the_bible_the_computers_rank_nnn_tags_read_their_own_government() {
        let rules = Rulebook::default().with_override(RuleKey::CrossedRankTags, RuleSource::Bible);
        let mut session = ranked(
            vec![to_mars(130)],
            vec![texts(130, "Run <PRK128>", Some("Go, <PRK128>."))],
            rules,
        );
        let _ = session.take_in(
            TextSource::Desc,
            "Earlier: <PRK128> <SRK129>.",
            &mut NeverFires,
        );
        let computer = session.mission_computer().expect("opens");
        assert_eq!(
            (
                computer.rows[0].name.as_str(),
                computer.rows[0].text.as_str()
            ),
            ("Run Cdr", "Go, Cdr.")
        );
    }

    #[test]
    fn by_the_engine_a_shown_text_resets_the_governments_before_its_own() {
        let mut session = ranked(
            vec![to_mars(130)],
            vec![MissionTexts {
                brief: Some("Off you go, <PRK128>.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
            Rulebook::default(),
        );
        let _ = session.take_in(
            TextSource::Quote,
            "Earlier: <SRK129> said.",
            &mut NeverFires,
        );
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        assert_eq!(
            session.take_mission_texts(),
            [text(MissionTextKind::Brief, 130, "Off you go, captain.")],
            "the earlier <SRK129> is reset (@0x71926-0x71930)"
        );
    }

    #[test]
    fn the_mission_infos_names_read_the_governments_left_and_a_selected_row_loads_its_text() {
        let mut session = ranked(
            vec![to_mars(130)],
            vec![MissionTexts {
                quick_brief: Some("To <PRK128>.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run <PRK128>")
            }],
            Rulebook::default(),
        );
        let slot = session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        let _ = session.take_in(
            TextSource::Desc,
            "Earlier: <PRK128> <SRK129>.",
            &mut NeverFires,
        );
        let info = session.mission_info();
        assert_eq!(
            (info.rows[0].name.as_str(), info.rows[0].text.as_str()),
            ("Run Ad", "To captain.")
        );
        assert_eq!(
            session.show_info_row(slot, &mut NeverFires).as_deref(),
            Some("To captain.")
        );
        assert_eq!(
            session.plain_text("<PRK128>"),
            "captain",
            "left by the load"
        );
        assert_eq!(
            session.show_info_row(15, &mut NeverFires),
            None,
            "a free slot"
        );
    }

    /// String lists of none, from data holding a `l33t` resource.
    struct Leet;

    impl crate::catalog::CommCatalog for Leet {
        fn string_list(&self, _id: i16) -> Vec<String> {
            Vec::new()
        }

        fn leet_mode(&self) -> bool {
            true
        }
    }

    /// [`docked`] in `catalog`, its strings [`Leet`]'s.
    fn leet(catalog: &FakePilotCatalog) -> Session {
        docked(catalog).with_strings(std::rc::Rc::new(Leet))
    }

    /// Mission 130 to Mars, "Ride" offering "You go to <DST> alone."
    fn leet_catalog() -> FakePilotCatalog {
        screens(
            vec![to_mars(130)],
            vec![texts(130, "Ride", Some("You go to <DST> alone."))],
        )
    }

    #[test]
    fn without_leet_mode_texts_are_unchanged() {
        let catalog = leet_catalog();
        let mut session = docked(&catalog);
        let mut chance = Draws::of(&[]);
        let shown = session
            .show_computer_row(MissionId(130), &mut chance)
            .expect("offered");
        assert_eq!(shown.text, "You go to Mars alone.");
        assert_eq!(chance.asked, Vec::<u32>::new(), "nothing drawn");
    }

    #[test]
    fn with_leet_mode_a_shown_text_draws_its_letters_then_is_munged() {
        let catalog = leet_catalog();
        let mut session = leet(&catalog);
        // "You go to <DST> alone.": o (go) kept, o (to) replaced, a and o
        // (alone) replaced, e kept; Y's "ou" turns to j00 with no draw.
        // The munge then turns the e to 3 and the l to 7.
        let mut chance = Draws::of(&[2, 0, 1, 0, 2]);
        let shown = session
            .show_computer_row(MissionId(130), &mut chance)
            .expect("offered");
        assert_eq!(chance.asked, [3; 5]);
        assert_eq!(
            shown.text, "j00 go t0 Mars 470n3.",
            "letters drawn on the dësc (@0x7194f), then the filled text \
             munged (@0x9d5aa): the stellar's name too"
        );
        assert_eq!(shown.name, "R1d3", "a name is munged, never drawn on");
    }

    #[test]
    fn with_leet_mode_a_query_gives_the_same_text_each_time() {
        let catalog = leet_catalog();
        let session = leet(&catalog);
        let first = session.mission_computer().expect("opens");
        assert_eq!(first, session.mission_computer().expect("opens"));
        assert_eq!(
            first.rows[0].text, "j00 go to Mars a7on3.",
            "munged only, nothing drawn"
        );
        assert_eq!(
            session.offer_text(MissionId(130), "Hello"),
            session.offer_text(MissionId(130), "Hello")
        );
        assert_eq!(session.plain_text("Hello").as_str(), "H377o");
    }

    fn ids(computer: &MissionComputer) -> Vec<i16> {
        computer.rows.iter().map(|row| row.mission.0).collect()
    }

    #[test]
    fn today_reads_as_the_medium_date_with_the_chärs_affixes() {
        let mut catalog = catalog();
        catalog.date_affixes = DateAffixes {
            prefix: String::new(),
            suffix: " NC".to_owned(),
        };
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session.med_date_text(),
            crate::date::med_date_text(session.date(), &catalog.date_affixes)
        );
        assert!(
            session.med_date_text().ends_with(" NC"),
            "{}",
            session.med_date_text()
        );
    }

    #[test]
    fn the_session_reads_each_missions_texts_as_it_starts() {
        let mut catalog = catalog();
        let run = MissionTexts {
            offer: Some("Go.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        };
        catalog.mission_texts = vec![run.clone(), MissionTexts::named(MissionId(131), "Walk")];
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.mission_texts(MissionId(130)), Some(&run));
        assert_eq!(
            session
                .mission_texts(MissionId(131))
                .map(|texts| texts.name.as_str()),
            Some("Walk")
        );
        assert_eq!(session.mission_texts(MissionId(132)), None);
    }

    #[test]
    fn the_computer_lists_its_offers_in_display_order_filled_for_each_offer() {
        let catalog = screens(
            vec![
                MissionRecord {
                    disp_weight: -1,
                    ..to_mars(130)
                },
                to_mars(131),
                MissionRecord {
                    avail_loc: 1,
                    ..to_mars(132)
                },
            ],
            vec![
                texts(130, "Run to <DST>", Some("Take this to <DST> for <PAY>.")),
                texts(131, "Walk", None),
            ],
        );
        let session = docked(&catalog);
        let computer = session.mission_computer().expect("opens");
        assert_eq!(
            computer,
            MissionComputer {
                rows: vec![
                    ComputerRow {
                        mission: MissionId(131),
                        name: "Walk".to_owned(),
                        text: String::new(),
                    },
                    ComputerRow {
                        mission: MissionId(130),
                        name: "Run to Mars".to_owned(),
                        text: "Take this to Mars for 900.".to_owned(),
                    },
                ],
                today: session.med_date_text(),
            },
            "131 weighs 0, 130 below it; 132 is the bar's"
        );
    }

    #[test]
    fn with_every_slot_taken_the_computer_does_not_open() {
        let catalog = screens(vec![to_mars(130)], Vec::new());
        let mut session = docked(&catalog);
        let offer = session.mission_offers(Venue::Computer)[0].clone();
        for _ in 0..SLOTS {
            let active = ActiveMission::accepted(&mission(141), &offer, session.date());
            session.pilot.missions.fill(active);
        }
        assert_eq!(session.mission_computer(), Err(ComputerRefusal::SlotsFull));
    }

    #[test]
    fn with_no_offer_available_again_the_computer_does_not_open() {
        let catalog = screens(
            vec![MissionRecord {
                avail_bits: Test::parse("!b5"),
                ..to_mars(130)
            }],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        assert!(session.mission_computer().is_ok());
        session.set_control_bit(bit(5), true);
        assert_eq!(
            session.mission_offers(Venue::Computer).len(),
            1,
            "still offered"
        );
        assert_eq!(
            session.mission_computer(),
            Err(ComputerRefusal::NoneAvailable)
        );
    }

    #[test]
    fn the_computer_lists_an_offer_no_longer_available_while_another_is() {
        let catalog = screens(
            vec![
                MissionRecord {
                    avail_bits: Test::parse("!b5"),
                    ..to_mars(130)
                },
                to_mars(131),
            ],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        session.set_control_bit(bit(5), true);
        let computer = session.mission_computer().expect("opens");
        assert_eq!(ids(&computer), [130, 131]);
    }

    #[test]
    fn in_flight_the_computer_has_nothing() {
        let catalog = screens(vec![to_mars(130)], Vec::new());
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session.mission_computer(),
            Err(ComputerRefusal::NoneAvailable)
        );
    }

    /// Takes `session` off, settles, puts it at rest over `stellar` (one
    /// of 130's), lands there and settles.
    fn land_on(session: &mut Session, catalog: &FakePilotCatalog, stellar: i16) {
        session
            .take_off(catalog, &mut NeverFires)
            .expect("takes off");
        settle(session, catalog);
        let site = *session
            .sites
            .iter()
            .find(|site| site.id == StellarId(stellar))
            .expect("shown");
        session.player.position = site.position;
        session.player.velocity = Vec2::ZERO;
        assert_eq!(land_now(session), Ok(StellarId(stellar)));
        settle(session, catalog);
    }

    /// Mission `id`, offered at Earth, to Luna and back there.
    fn to_luna(id: i16) -> MissionRecord {
        MissionRecord {
            avail_stel: 128,
            travel_stel: 129,
            ..mission(id)
        }
    }

    #[test]
    fn ship_done_picks_the_ship_done_text() {
        let texts = MissionTexts {
            ship_done: Some("Done.".to_owned()),
            comp: Some("Comp.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        };
        assert_eq!(MissionTextKind::ShipDone.of(&texts), Some("Done."));
        assert_eq!(
            MissionTextKind::ShipDone.of(&MissionTexts::named(MissionId(130), "Run")),
            None
        );
    }

    fn text(kind: MissionTextKind, mission: i16, text: &str) -> MissionText {
        MissionText {
            kind,
            mission: MissionId(mission),
            text: text.to_owned(),
        }
    }

    fn run(session: &mut Session, script: &str) {
        session.run_set(&SetExpr::parse(script).expect("parses"), &mut NeverFires);
    }

    #[test]
    fn a_mission_completed_shows_its_comp_text_filled_before_its_slot_is_freed() {
        let catalog = screens(
            vec![to_luna(130)],
            vec![MissionTexts {
                comp: Some("Delivered to <DST>.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        assert_eq!(session.take_mission_texts(), []);
        land_on(&mut session, &catalog, 129);
        assert!(session.pilot().missions().get(0).is_none(), "completed");
        assert_eq!(
            session.take_mission_texts(),
            [text(MissionTextKind::Comp, 130, "Delivered to Luna.")]
        );
        assert_eq!(session.take_mission_texts(), [], "taken");
    }

    #[test]
    fn a_mission_failed_at_its_return_shows_its_fail_text_filled_after_its_slot_is_freed() {
        let catalog = screens(
            vec![to_luna(130)],
            vec![MissionTexts {
                fail: Some("Failed at <DST>.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        run(&mut session, "F130");
        land_on(&mut session, &catalog, 129);
        assert!(session.pilot().missions().get(0).is_none(), "failed");
        assert_eq!(
            session.take_mission_texts(),
            [text(MissionTextKind::Fail, 130, "Failed at [Error].")]
        );
    }

    #[test]
    fn a_refusal_shows_its_refuse_text_with_no_mission_before_on_refuse_runs() {
        let catalog = screens(
            vec![
                MissionRecord {
                    on_refuse: Script::parse("S131"),
                    ..to_mars(130)
                },
                MissionRecord {
                    avail_loc: -1,
                    ..to_mars(131)
                },
            ],
            vec![
                MissionTexts {
                    refuse: Some("No trip to <DST>.".to_owned()),
                    ..MissionTexts::named(MissionId(130), "Run")
                },
                MissionTexts {
                    brief: Some("Off to <DST>.".to_owned()),
                    ..MissionTexts::named(MissionId(131), "Walk")
                },
            ],
        );
        let mut session = docked(&catalog);
        session
            .refuse_mission(MissionId(130), &mut NeverFires)
            .expect("refused");
        assert_eq!(
            session.take_mission_texts(),
            [
                text(MissionTextKind::Refuse, 130, "No trip to [Error]."),
                text(MissionTextKind::Brief, 131, "Off to Mars."),
            ],
            "_OfferOneMission munges it with index -1 (@0xa2eba-0xa2ec9)"
        );
    }

    #[test]
    fn an_accept_shows_its_brief_text_filled_for_its_slot_after_on_accept() {
        let catalog = screens(
            vec![MissionRecord {
                on_accept: Script::parse("A130"),
                ..to_mars(130)
            }],
            vec![MissionTexts {
                brief: Some("Off to <DST>.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        assert_eq!(
            session.take_mission_texts(),
            [text(MissionTextKind::Brief, 130, "Off to [Error].")],
            "its OnAccept aborted it, freeing the slot"
        );
        let plain = screens(
            vec![to_mars(130)],
            vec![MissionTexts {
                brief: Some("Off to <DST>.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&plain);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        assert_eq!(
            session.take_mission_texts(),
            [text(MissionTextKind::Brief, 130, "Off to Mars.")]
        );
    }

    #[test]
    fn a_mission_without_the_text_shows_nothing() {
        let catalog = screens(
            vec![
                to_luna(130),
                to_luna(131),
                to_luna(132),
                MissionRecord {
                    avail_loc: -1,
                    ..to_luna(133)
                },
            ],
            vec![MissionTexts::named(MissionId(130), "Run")],
        );
        let mut session = docked(&catalog);
        for id in [130, 131] {
            session
                .accept_mission(MissionId(id), &mut NeverFires)
                .expect("accepted");
        }
        session
            .refuse_mission(MissionId(132), &mut NeverFires)
            .expect("refused");
        run(&mut session, "F131");
        land_on(&mut session, &catalog, 129);
        assert!(session.pilot().missions().iter().next().is_none());
        assert_eq!(session.take_mission_texts(), []);
    }

    /// Mission `id`, offered at the bar, to Mars for 900.
    fn at_the_bar(id: i16) -> MissionRecord {
        MissionRecord {
            avail_loc: BAR,
            ..to_mars(id)
        }
    }

    /// Mission `id`'s texts: named "Mission `id`", offering `offer`, with
    /// these raw labels.
    fn offering(id: i16, offer: &str, accept: &str, refuse: &str) -> MissionTexts {
        MissionTexts {
            offer: Some(offer.to_owned()),
            accept_button: accept.to_owned(),
            refuse_button: refuse.to_owned(),
            ..MissionTexts::named(MissionId(id), &format!("Mission {id}"))
        }
    }

    fn prompted(offer: Option<VenueOffer>) -> MissionId {
        match offer {
            Some(VenueOffer::Prompt(prompt)) => prompt.mission,
            other => panic!("not a prompt: {other:?}"),
        }
    }

    /// Takes every mission slot with mission 150.
    fn fill_slots(session: &mut Session) {
        let offer = Offer {
            mission: MissionId(150),
            ..session.mission_offers(Venue::Outside)[0].clone()
        };
        for _ in 0..SLOTS {
            let active = ActiveMission::accepted(&mission(150), &offer, session.date());
            session.pilot.missions.fill(active);
        }
    }

    #[test]
    fn the_bar_offers_its_first_offer_in_display_order_filled_with_its_labels() {
        let catalog = screens(
            vec![
                MissionRecord {
                    avail_loc: SPACEPORT,
                    ..to_mars(140)
                },
                MissionRecord {
                    disp_weight: -1,
                    ..at_the_bar(141)
                },
                at_the_bar(142),
            ],
            vec![offering(142, "Ferry this to <DST>.", "I'm in", "")],
        );
        let mut session = docked(&catalog);
        assert_eq!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Prompt(OfferPrompt {
                mission: MissionId(142),
                text: "Ferry this to Mars.".to_owned(),
                labels: OfferLabels {
                    accept: "I'm in".to_owned(),
                    refuse: Some(NO_LABEL.to_owned()),
                },
            })),
            "142 weighs 0, 141 below it; 140 is the spaceport's"
        );
        assert!(
            matches!(session.next_offer(BAR, &mut NeverFires), Some(VenueOffer::Prompt(prompt)) if prompt.can_refuse()),
            "the same again, until it is answered"
        );
        session
            .refuse_mission(MissionId(142), &mut NeverFires)
            .expect("refused");
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(141)
        );
        session
            .refuse_mission(MissionId(141), &mut NeverFires)
            .expect("refused");
        assert_eq!(session.next_offer(BAR, &mut NeverFires), None, "none left");
        assert_eq!(
            prompted(session.next_offer(SPACEPORT, &mut NeverFires)),
            MissionId(140)
        );
    }

    #[test]
    fn in_flight_nothing_is_offered() {
        let catalog = screens(vec![at_the_bar(141)], Vec::new());
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.next_offer(BAR, &mut NeverFires), None);
    }

    #[test]
    fn an_offer_that_cannot_be_refused_has_one_button_and_with_no_text_is_accepted_at_once() {
        let unrefusable = |id| MissionRecord {
            flags: UNREFUSABLE,
            ..at_the_bar(id)
        };
        let catalog = screens(
            vec![unrefusable(141), unrefusable(142)],
            vec![
                offering(141, "", "", ""),
                offering(142, "Come along.", "", "No"),
            ],
        );
        let mut session = docked(&catalog);
        assert_eq!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Accepted {
                mission: MissionId(141),
                accepted: Ok(0),
            })
        );
        assert_eq!(session.pilot().missions().missions(), [MissionId(141)]);
        let next = session.next_offer(BAR, &mut NeverFires);
        let Some(VenueOffer::Prompt(prompt)) = next else {
            panic!("a prompt: {next:?}");
        };
        assert_eq!(prompt.mission, MissionId(142));
        assert_eq!(
            prompt.labels,
            OfferLabels {
                accept: OKAY_LABEL.to_owned(),
                refuse: None,
            }
        );
        assert!(!prompt.can_refuse());
    }

    #[test]
    fn the_accept_at_once_drops_its_offer_but_reaches_nothing_and_refilters_nothing() {
        let catalog = screens(
            vec![
                MissionRecord {
                    flags: UNREFUSABLE,
                    avail_stel: 128,
                    travel_stel: 128,
                    on_accept: crate::control::Script::parse("b5"),
                    ..at_the_bar(141)
                },
                MissionRecord {
                    avail_bits: crate::control::Test::parse("!b5"),
                    ..at_the_bar(143)
                },
            ],
            vec![offering(141, "", "", "")],
        );
        let mut session = docked(&catalog);
        let ids = |session: &Session| -> Vec<MissionId> {
            session
                .mission_offers(Venue::Outside)
                .iter()
                .map(|offer| offer.mission)
                .collect()
        };
        assert_eq!(ids(&session), [MissionId(141), MissionId(143)]);
        assert!(matches!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Accepted {
                accepted: Ok(0),
                ..
            })
        ));
        let active = session.pilot().missions().get(0).expect("held");
        assert!(!active.reached, "the dialog never opened (@0xa230a)");
        assert_eq!(
            ids(&session),
            [MissionId(143)],
            "the caller drops the accepted offer (@0xa3765-0xa37a1); the \
             others stay unfiltered"
        );
    }

    #[test]
    fn an_outside_offer_whose_accept_is_refused_is_suppressed() {
        let catalog = screens(
            vec![
                MissionRecord {
                    flags: UNREFUSABLE,
                    ..at_the_bar(141)
                },
                at_the_bar(142),
                at_the_bar(143),
            ],
            vec![offering(141, "", "", "")],
        );
        let mut session = docked(&catalog);
        fill_slots(&mut session);
        assert_eq!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Accepted {
                mission: MissionId(141),
                accepted: Err(AcceptError::Refused(AcceptRefusal::SlotsFull)),
            })
        );
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(142)
        );
        assert_eq!(
            session.accept_mission(MissionId(142), &mut NeverFires),
            Err(AcceptError::Refused(AcceptRefusal::SlotsFull))
        );
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(143),
            "141 and 142 suppressed"
        );
    }

    #[test]
    fn an_outside_offer_accepted_is_not_suppressed() {
        let catalog = screens(vec![at_the_bar(141)], Vec::new());
        let mut session = docked(&catalog);
        let slot = session
            .accept_mission(MissionId(141), &mut NeverFires)
            .expect("accepted");
        session
            .abort_mission(slot, &mut NeverFires)
            .expect("aborted");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        session.player.velocity = Vec2::ZERO;
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(141),
            "offered again on the next landing"
        );
    }

    #[test]
    fn suppression_lasts_until_a_venue_other_than_the_last_but_the_spaceport_is_entered() {
        let catalog = screens(vec![at_the_bar(142), at_the_bar(143)], Vec::new());
        let mut session = docked(&catalog);
        session.enter_offer_venue(BAR);
        let mut full = docked(&catalog);
        full.enter_offer_venue(BAR);
        fill_slots(&mut full);
        assert!(
            full.accept_mission(MissionId(142), &mut NeverFires)
                .is_err()
        );
        let first = |session: &mut Session| prompted(session.next_offer(BAR, &mut NeverFires));
        assert_eq!(first(&mut full), MissionId(143));
        full.enter_offer_venue(BAR);
        assert_eq!(first(&mut full), MissionId(143), "the same visit");
        full.enter_offer_venue(SPACEPORT);
        assert!(
            full.suppressed.contains(&MissionId(142)),
            "the spaceport keeps it"
        );
        full.enter_offer_venue(BAR);
        assert_eq!(
            first(&mut full),
            MissionId(142),
            "back in the bar, forgotten"
        );
        assert!(
            full.accept_mission(MissionId(142), &mut NeverFires)
                .is_err()
        );
        let flown = Session::fly(&catalog, full.pilot().clone()).expect("flies");
        let mut flown = flown;
        assert_eq!(land_now(&mut flown), Ok(StellarId(128)));
        settle(&mut flown, &catalog);
        assert_eq!(first(&mut flown), MissionId(142), "never saved");
        assert_eq!(first(&mut session), MissionId(142));
    }

    /// Mission `id`, offered at venue `loc`, to Mars for 900.
    fn at_venue(id: i16, loc: i16) -> MissionRecord {
        MissionRecord {
            avail_loc: loc,
            ..to_mars(id)
        }
    }

    /// Mission `id`, offered at venue `loc`, that cannot be refused and
    /// has no offer text, so its offer is an accept at once.
    fn unrefusable_at(id: i16, loc: i16) -> MissionRecord {
        MissionRecord {
            flags: UNREFUSABLE,
            ..at_venue(id, loc)
        }
    }

    const VENUES: [i16; 5] = [BAR, SPACEPORT, TRADE_CENTER, SHIPYARD, OUTFITTER];

    #[test]
    fn next_offer_at_the_spaceport_offers_an_availloc_3_mission() {
        let catalog = screens(vec![at_venue(141, SPACEPORT)], Vec::new());
        for loc in VENUES.into_iter().filter(|&loc| loc != SPACEPORT) {
            let mut session = docked(&catalog);
            assert_eq!(session.next_offer(loc, &mut NeverFires), None, "{loc}");
        }
        let mut session = docked(&catalog);
        assert!(matches!(
            session.next_offer(SPACEPORT, &mut NeverFires),
            Some(VenueOffer::Prompt(prompt)) if prompt.mission == MissionId(141) && prompt.can_refuse()
        ));
    }

    #[test]
    fn next_offer_at_each_shop_offers_only_its_own_missions() {
        let catalog = screens(
            vec![
                at_venue(141, TRADE_CENTER),
                at_venue(142, SHIPYARD),
                at_venue(143, OUTFITTER),
            ],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        for (loc, id) in [(TRADE_CENTER, 141), (SHIPYARD, 142), (OUTFITTER, 143)] {
            assert_eq!(
                prompted(session.next_offer(loc, &mut NeverFires)),
                MissionId(id),
                "{loc}"
            );
        }
        assert_eq!(session.next_offer(BAR, &mut NeverFires), None);
        assert_eq!(session.next_offer(SPACEPORT, &mut NeverFires), None);
    }

    #[test]
    fn the_next_offer_enters_its_venue_forgetting_the_suppressed_offers() {
        let catalog = screens(vec![unrefusable_at(141, BAR)], Vec::new());
        let mut session = docked(&catalog);
        fill_slots(&mut session);
        assert!(matches!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Accepted {
                accepted: Err(_),
                ..
            })
        ));
        assert_eq!(session.next_offer(BAR, &mut NeverFires), None, "suppressed");
        assert_eq!(session.next_offer(OUTFITTER, &mut NeverFires), None);
        assert!(
            matches!(
                session.next_offer(BAR, &mut NeverFires),
                Some(VenueOffer::Accepted {
                    mission: MissionId(141),
                    ..
                })
            ),
            "the outfitter entered forgot it"
        );
    }

    #[test]
    fn a_spaceport_offer_refused_stays_suppressed_until_another_venue_is_offered_at() {
        let catalog = screens(vec![unrefusable_at(141, SPACEPORT)], Vec::new());
        let mut session = docked(&catalog);
        fill_slots(&mut session);
        assert!(matches!(
            session.next_offer(SPACEPORT, &mut NeverFires),
            Some(VenueOffer::Accepted {
                accepted: Err(_),
                ..
            })
        ));
        assert_eq!(session.next_offer(SPACEPORT, &mut NeverFires), None);
        assert_eq!(session.next_offer(BAR, &mut NeverFires), None);
        assert!(
            matches!(
                session.next_offer(SPACEPORT, &mut NeverFires),
                Some(VenueOffer::Accepted {
                    mission: MissionId(141),
                    ..
                })
            ),
            "the bar entered forgot it"
        );
    }

    /// A bar offer refused for want of a slot stays suppressed while no
    /// other venue is offered at, and offering at the spaceport in between
    /// forgets it: so the router must not ask at the spaceport again as
    /// the player comes back to it from the bar.
    #[test]
    fn a_bar_offer_stays_suppressed_while_nothing_is_offered_at_in_between() {
        let catalog = screens(vec![unrefusable_at(141, BAR)], Vec::new());
        let mut session = docked(&catalog);
        fill_slots(&mut session);
        assert!(session.next_offer(BAR, &mut NeverFires).is_some());
        assert_eq!(
            session.next_offer(BAR, &mut NeverFires),
            None,
            "nothing offered at in between"
        );
        assert_eq!(session.next_offer(SPACEPORT, &mut NeverFires), None);
        assert!(
            matches!(
                session.next_offer(BAR, &mut NeverFires),
                Some(VenueOffer::Accepted {
                    mission: MissionId(141),
                    ..
                })
            ),
            "the spaceport offered at forgot it"
        );
    }

    #[test]
    fn a_refusal_that_makes_a_later_offer_unavailable_stops_it_being_offered() {
        let catalog = screens(
            vec![
                MissionRecord {
                    on_refuse: Script::parse("b5"),
                    ..at_the_bar(141)
                },
                MissionRecord {
                    avail_bits: Test::parse("!b5"),
                    ..at_the_bar(142)
                },
            ],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(141)
        );
        session
            .refuse_mission(MissionId(141), &mut NeverFires)
            .expect("refused");
        assert_eq!(
            session.mission_offers(Venue::Outside).len(),
            1,
            "142 is still listed"
        );
        assert_eq!(session.next_offer(BAR, &mut NeverFires), None);
    }

    fn info_row(slot: u8, name: &str, failed: bool, text: &str, abortable: bool) -> InfoRow {
        InfoRow {
            slot,
            name: name.to_owned(),
            failed,
            text: text.to_owned(),
            abortable,
        }
    }

    #[test]
    fn mission_info_lists_the_shown_active_missions_filled_for_their_slots() {
        let catalog = screens(
            vec![
                MissionRecord {
                    time_limit: 7,
                    ..to_mars(130)
                },
                MissionRecord {
                    flags: HIDDEN,
                    ..to_mars(131)
                },
                MissionRecord {
                    can_abort: 0,
                    ..to_mars(132)
                },
            ],
            vec![
                MissionTexts {
                    quick_brief: Some("Deliver to <DST> by <DL>.".to_owned()),
                    ..MissionTexts::named(MissionId(130), "Run to <DST>")
                },
                MissionTexts::named(MissionId(131), "Hidden"),
                MissionTexts::named(MissionId(132), "Stay"),
            ],
        );
        let mut session = docked(&catalog);
        for id in [130, 131, 132] {
            session
                .accept_mission(MissionId(id), &mut NeverFires)
                .expect("accepted");
        }
        run(&mut session, "F132");
        assert_eq!(
            session.mission_info(),
            MissionInfo {
                rows: vec![
                    info_row(
                        0,
                        "Run to Mars",
                        false,
                        "Deliver to Mars by June 30th, 1177 NC.",
                        true
                    ),
                    info_row(2, "Stay", true, "", false),
                ],
                today: session.med_date_text(),
            },
            "131 has Flags 0x0400; 132 failed by F, and cannot be aborted"
        );
    }

    #[test]
    fn a_slot_whose_mission_is_gone_reads_with_no_text() {
        let catalog = screens(
            vec![to_mars(130)],
            vec![MissionTexts {
                quick_brief: Some("Go.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        let gone = screens(Vec::new(), Vec::new());
        let flown = Session::fly(&gone, session.pilot().clone()).expect("flies");
        assert_eq!(
            flown.mission_info().rows,
            [info_row(0, "", false, "", true)]
        );
        assert_eq!(
            Session::start(&gone).expect("starts").mission_info().rows,
            []
        );
    }

    #[test]
    fn mission_info_aborts_only_where_its_row_says() {
        let catalog = screens(
            vec![
                to_mars(130),
                MissionRecord {
                    can_abort: 0,
                    ..to_mars(131)
                },
            ],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        for id in [130, 131] {
            session
                .accept_mission(MissionId(id), &mut NeverFires)
                .expect("accepted");
        }
        let info = session.mission_info();
        let refused = info.rows.iter().find(|row| !row.abortable).expect("131");
        assert_eq!(
            session.abort_mission(refused.slot, &mut NeverFires),
            Err(AbortError::Refused(AbortRefusal::CannotAbort))
        );
        let allowed = info.rows.iter().find(|row| row.abortable).expect("130");
        assert_eq!(session.abort_mission(allowed.slot, &mut NeverFires), Ok(()));
        let left: Vec<_> = session
            .mission_info()
            .rows
            .iter()
            .map(|row| row.slot)
            .collect();
        assert_eq!(left, [refused.slot]);
    }

    #[test]
    fn each_refusal_says_what_the_original_says() {
        assert_eq!(
            ComputerRefusal::SlotsFull.message(),
            AcceptRefusal::SlotsFull.message()
        );
        assert_eq!(ComputerRefusal::NoneAvailable.message(), NO_MISSIONS_HERE);
    }
}
