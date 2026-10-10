//! The session's side of the mission screens (see the
//! [`mission`](crate::mission) module's "Screens"): what each mission's
//! screens show of it, read as the session starts, and the mission
//! computer's view of the visit's offers.

use super::Session;
use crate::catalog::{MissionId, MissionTexts};
use crate::chance::NeverFires;
use crate::mission::screens::NO_MISSIONS_HERE;
use crate::mission::{AcceptRefusal, AvailFacts, Offer, Venue, availability};

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
}

impl MissionTextKind {
    /// This text of `texts`, as written, if it has one.
    fn of(self, texts: &MissionTexts) -> Option<&str> {
        match self {
            Self::Brief => texts.brief.as_deref(),
            Self::Comp => texts.comp.as_deref(),
            Self::Fail => texts.fail.as_deref(),
            Self::Refuse => texts.refuse.as_deref(),
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

    /// Queues `mission`'s `kind` text to show, when it has one, filled
    /// for `slot` ([`Session::mission_text`]), or with no mission
    /// ([`Session::plain_text`]).
    pub(super) fn show_mission_text(
        &mut self,
        kind: MissionTextKind,
        mission: MissionId,
        slot: Option<u8>,
    ) {
        let Some(raw) = self
            .mission_texts
            .get(&mission)
            .and_then(|texts| kind.of(texts))
        else {
            return;
        };
        let text = match slot {
            Some(slot) => self.mission_text(slot, raw),
            None => self.plain_text(raw),
        };
        self.shown_texts.push(MissionText {
            kind,
            mission,
            text,
        });
    }

    /// The mission computer of the stellar docked at (`_DoMissionDialog`
    /// @0xa37cf): every offer at the computer in the order it shows, with
    /// no availability check (`_SetupMissionList` @0xa08c4-0xa09d6), each
    /// name and text filled for its offer, and today's medium date.
    ///
    /// # Errors
    ///
    /// [`ComputerRefusal::SlotsFull`] with every slot taken
    /// (@0xa37e9-0xa38b8); then [`ComputerRefusal::NoneAvailable`] in
    /// flight, or when none of the offers is available now
    /// (`_IsMissionAvailable(i, 1)` @0xa38bd-0xa3956), checked as an
    /// accept leaves them, drawing nothing.
    pub fn mission_computer(&self) -> Result<MissionComputer, ComputerRefusal> {
        if self.accept_refusal(&mut NeverFires) == Err(AcceptRefusal::SlotsFull) {
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
                let fill = |text: Option<&str>| {
                    self.offer_text(offer.mission, text.unwrap_or_default())
                        .unwrap_or_default()
                };
                ComputerRow {
                    mission: offer.mission,
                    name: fill(texts.map(|texts| texts.name.as_str())),
                    text: fill(texts.and_then(|texts| texts.offer.as_deref())),
                }
            })
            .collect();
        Ok(MissionComputer {
            rows,
            today: self.med_date_text(),
        })
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
    use crate::mission::{ActiveMission, SLOTS};
    use crate::testkit::{FakePilotCatalog, catalog, land_now, mission};

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
        session.take_off(&mut NeverFires).expect("takes off");
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
    fn a_refusal_shows_its_refuse_text_unfilled_before_on_refuse_runs() {
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
                text(MissionTextKind::Refuse, 130, "No trip to <DST>."),
                text(MissionTextKind::Brief, 131, "Off to Mars."),
            ]
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

    #[test]
    fn each_refusal_says_what_the_original_says() {
        assert_eq!(
            ComputerRefusal::SlotsFull.message(),
            AcceptRefusal::SlotsFull.message()
        );
        assert_eq!(ComputerRefusal::NoneAvailable.message(), NO_MISSIONS_HERE);
    }
}
