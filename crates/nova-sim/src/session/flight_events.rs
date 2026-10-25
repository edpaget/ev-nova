//! [`Session::drain_events`]: the session's one-shot outputs, drained as
//! one stream of [`FlightEvent`]s for the flight view's `FlightSim` port
//! (see [`flight_frame`](crate::flight_frame)).
//!
//! Each kind is moved across with its own take, so the stream holds
//! exactly what the takes would have given, and taking a kind after a
//! drain finds none. The kinds come in the [`Outbox`]'s field order, then
//! the two held elsewhere:
//!
//! 1. the sounds ([`FlightEvent::Sound`]), the messages
//!    ([`FlightEvent::Message`]) and the session events
//!    ([`FlightEvent::Session`]);
//! 2. the save due ([`FlightEvent::SaveDue`]) and the hold's expiry
//!    ([`FlightEvent::HoldExpired`]), each once when set;
//! 3. the diagnostics ([`FlightEvent::Diagnostic`]): the fight's, then the
//!    session's own reports;
//! 4. the grant ([`FlightEvent::Grant`]) and the board line
//!    ([`FlightEvent::BoardLine`]);
//! 5. the comm notes, the fighter notes, the pay notes and the quotes
//!    ([`FlightEvent::Comm`], [`FlightEvent::FighterNote`],
//!    [`FlightEvent::PayNote`], [`FlightEvent::Quote`]);
//! 6. the mission texts and offer movies, each text with the bar it noted
//!    ([`FlightEvent::MissionShown`]);
//! 7. the interface loads ([`FlightEvent::InterfaceLoad`]), after
//!    [`FlightEvent::InterfaceLoadsCleared`] when a run of docked scripts
//!    began since the last drain;
//! 8. the take-off's nav ([`FlightEvent::TakeoffNav`]);
//! 9. the rescue's `dësc` ([`FlightEvent::RescueDesc`], on the
//!    `PlayerShip`) and the leave message ([`FlightEvent::LeaveMessage`],
//!    on the `MissionRun`).
//!
//! Within a kind the order is the session's. The script notes are not
//! drained: no view takes them. Nor are the fight's events, which come
//! with each step.
//!
//! **The interface loads' clear.** Each run of docked scripts lets go of
//! the loads the run before left untaken ([`Outbox::begin_docked_run`]).
//! A view may drain those loads before the run begins and take them
//! after, so the clear rides the stream: whoever holds drained loads drops
//! them at [`FlightEvent::InterfaceLoadsCleared`], and holds exactly what
//! the session's take would have given.

use crate::flight_frame::FlightEvent;

use super::{MissionShown, Outbox, Session};

impl Session {
    /// Moves every one-shot output raised since the last drain (or take)
    /// onto the end of `into`, in the order of the module docs, leaving
    /// none to drain or take again.
    pub fn drain_events(&mut self, into: &mut Vec<FlightEvent>) {
        into.extend(self.take_sounds().into_iter().map(FlightEvent::Sound));
        into.extend(self.take_messages().into_iter().map(FlightEvent::Message));
        into.extend(self.take_events().into_iter().map(FlightEvent::Session));
        if self.take_save_due() {
            into.push(FlightEvent::SaveDue);
        }
        if self.take_hold_expired() {
            into.push(FlightEvent::HoldExpired);
        }
        into.extend(
            self.take_diagnostics()
                .into_iter()
                .map(FlightEvent::Diagnostic),
        );
        into.extend(self.take_grant().map(FlightEvent::Grant));
        into.extend(self.take_board_line().map(FlightEvent::BoardLine));
        into.extend(self.take_comm().into_iter().map(FlightEvent::Comm));
        into.extend(
            self.take_fighter_notes()
                .into_iter()
                .map(FlightEvent::FighterNote),
        );
        into.extend(self.take_pay_notes().into_iter().map(FlightEvent::PayNote));
        into.extend(self.take_quotes().into_iter().map(FlightEvent::Quote));
        let mut bars = self.text_bars().to_vec().into_iter();
        into.extend(self.take_mission_texts().into_iter().map(|shown| {
            let bar = match shown {
                MissionShown::Text(_) => bars.next(),
                MissionShown::Movie(_) => None,
            };
            FlightEvent::MissionShown { shown, bar }
        }));
        if std::mem::take(&mut self.out.interface_loads_cleared) {
            into.push(FlightEvent::InterfaceLoadsCleared);
        }
        into.extend(
            self.take_interface_loads()
                .into_iter()
                .map(FlightEvent::InterfaceLoad),
        );
        into.extend(self.take_takeoff_nav().map(FlightEvent::TakeoffNav));
        into.extend(self.take_rescue_desc().map(FlightEvent::RescueDesc));
        into.extend(self.take_leave_message().map(FlightEvent::LeaveMessage));
    }
}

impl Outbox {
    /// A run of docked scripts begins: the `C`, `E` and `H` loads the run
    /// before left untaken are let go (see
    /// [`Session::take_interface_loads`]), and the next drain says so
    /// ([`FlightEvent::InterfaceLoadsCleared`]).
    pub(super) fn begin_docked_run(&mut self) {
        self.interface_loads.clear();
        self.interface_loads_cleared = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bay::FighterNote;
    use crate::board::MissionBoardLine;
    use crate::catalog::{DescMovie, MissionId, OutfitId, StellarId, WeaponId};
    use crate::chance::NeverFires;
    use crate::combat::report::SimDiagnostic;
    use crate::grant::Granted;
    use crate::hail::CommNote;
    use crate::hire::PayNote;
    use crate::message::SimMessage;
    use crate::navigation::NavMode;
    use crate::session::tests::{buy, outfitted, shipbuying, three_stellars};
    use crate::session::{
        MissionText, MissionTextKind, PersonQuote, SessionEvent, StellarPick, TextBar,
    };
    use crate::sound::SimSound;
    use crate::testkit::{catalog, land_now};
    use crate::traffic::npc::NpcId;

    fn started() -> Session {
        Session::start(&catalog()).expect("starts")
    }

    /// What one drain of `session` gives.
    fn drained(session: &mut Session) -> Vec<FlightEvent> {
        let mut into = Vec::new();
        session.drain_events(&mut into);
        into
    }

    fn text(mission: i16) -> MissionText {
        MissionText {
            kind: MissionTextKind::Brief,
            mission: MissionId(mission),
            text: format!("text {mission}"),
            graphic: None,
            movie: None,
            map: None,
        }
    }

    fn movie() -> DescMovie {
        DescMovie {
            file: "offer.mov".to_owned(),
            after: true,
            double_size: false,
            cinematic: false,
        }
    }

    /// Queues `text` as the session's own queueing does: counted, with the
    /// bar noted as it stands.
    fn queue_text(session: &mut Session, text: MissionText) -> TextBar {
        session.out.texts_queued += 1;
        let bar = TextBar {
            texts: session.out.texts_queued,
            nav: session.nav_mode(),
            reserves: session.reserves(),
        };
        session.out.text_bars.push(bar);
        session.out.shown_texts.push(MissionShown::Text(text));
        bar
    }

    #[test]
    fn a_new_session_drains_nothing() {
        assert_eq!(drained(&mut started()), []);
    }

    #[test]
    fn a_sound_raised_is_drained() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_stellar(StellarPick::Slot(0));
        assert!(drained(&mut session).contains(&FlightEvent::Sound(SimSound::NavSelected)));
        assert_eq!(session.take_sounds(), [], "drained, not left to take");
    }

    #[test]
    fn a_message_raised_is_drained() {
        let mut session = started();
        let message = SimMessage::Buoy("Welcome.".to_owned());
        session.space.say(&mut session.out, message.clone());
        assert_eq!(drained(&mut session), [FlightEvent::Message(message)]);
        assert_eq!(session.take_messages(), []);
    }

    #[test]
    fn a_session_event_is_drained() {
        let mut session = started();
        session.out.emit(SessionEvent::GalaxyChanged);
        assert_eq!(
            drained(&mut session),
            [FlightEvent::Session(SessionEvent::GalaxyChanged)]
        );
        assert_eq!(session.take_events(), []);
    }

    #[test]
    fn a_landing_drains_a_save_due_once() {
        let mut session = started();
        land_now(&mut session).expect("lands");
        let events = drained(&mut session);
        assert_eq!(
            events
                .iter()
                .filter(|event| **event == FlightEvent::SaveDue)
                .count(),
            1
        );
        assert!(!session.take_save_due());
    }

    #[test]
    fn the_holds_expiry_is_drained_once() {
        let mut session = started();
        session.out.hold_expiry_due = true;
        assert_eq!(drained(&mut session), [FlightEvent::HoldExpired]);
        assert!(!session.take_hold_expired());
    }

    #[test]
    fn a_diagnostic_is_drained() {
        let mut session = started();
        let diagnostic = SimDiagnostic::UnimplementedGuidance {
            weapon: WeaponId(128),
            guidance: 42,
        };
        session.out.reports.report(diagnostic);
        assert_eq!(drained(&mut session), [FlightEvent::Diagnostic(diagnostic)]);
        assert_eq!(session.take_diagnostics(), []);
    }

    #[test]
    fn a_grant_is_drained() {
        let mut session = started();
        let granted = Granted {
            outfit: OutfitId(128),
            count: 2,
        };
        session.out.granted = Some(granted);
        assert_eq!(drained(&mut session), [FlightEvent::Grant(granted)]);
        assert_eq!(session.take_grant(), None);
    }

    #[test]
    fn a_board_line_is_drained() {
        let mut session = started();
        session.out.board_line = Some(MissionBoardLine::Boarded);
        assert_eq!(
            drained(&mut session),
            [FlightEvent::BoardLine(MissionBoardLine::Boarded)]
        );
        assert_eq!(session.take_board_line(), None);
    }

    fn note() -> CommNote {
        CommNote {
            from: NpcId(3),
            comm_name: "Shuttle".to_owned(),
            ship_name: "Kestrel".to_owned(),
            done: crate::ai::Help::Refuel,
        }
    }

    #[test]
    fn a_comm_note_is_drained() {
        let mut session = started();
        session.out.comm.push(note());
        assert_eq!(drained(&mut session), [FlightEvent::Comm(note())]);
        assert_eq!(session.take_comm(), []);
    }

    #[test]
    fn a_fighter_note_is_drained() {
        let mut session = started();
        session.out.fighter_notes.push(FighterNote::Abandoned(2));
        assert_eq!(
            drained(&mut session),
            [FlightEvent::FighterNote(FighterNote::Abandoned(2))]
        );
        assert_eq!(session.take_fighter_notes(), []);
    }

    #[test]
    fn a_pay_note_is_drained() {
        let mut session = started();
        session.out.pay_notes.push(PayNote::Defected(1));
        assert_eq!(
            drained(&mut session),
            [FlightEvent::PayNote(PayNote::Defected(1))]
        );
        assert_eq!(session.take_pay_notes(), []);
    }

    #[test]
    fn a_quote_is_drained() {
        let mut session = started();
        let quote = PersonQuote {
            npc: NpcId(4),
            text: "Hello.".to_owned(),
        };
        session.out.quotes.push(quote.clone());
        assert_eq!(drained(&mut session), [FlightEvent::Quote(quote)]);
        assert_eq!(session.take_quotes(), []);
    }

    #[test]
    fn each_text_is_drained_with_its_bar_and_a_movie_with_none() {
        let mut session = started();
        let first = queue_text(&mut session, text(128));
        session.out.shown_texts.push(MissionShown::Movie(movie()));
        session.chart.nav = NavMode::Stellar(Some(StellarId(128)));
        let second = queue_text(&mut session, text(129));
        assert_ne!(first.nav, second.nav, "each bar its own");
        assert_eq!(
            drained(&mut session),
            [
                FlightEvent::MissionShown {
                    shown: MissionShown::Text(text(128)),
                    bar: Some(first),
                },
                FlightEvent::MissionShown {
                    shown: MissionShown::Movie(movie()),
                    bar: None,
                },
                FlightEvent::MissionShown {
                    shown: MissionShown::Text(text(129)),
                    bar: Some(second),
                },
            ]
        );
        assert_eq!(session.take_mission_texts(), []);
        assert_eq!(session.text_bars(), [], "the bars went with them");
        assert_eq!(session.texts_queued(), 2, "never reset");
    }

    #[test]
    fn a_docked_load_is_drained_and_an_order_after_a_drain_drains_its_clear() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        run_expr(&mut session, "H129");
        let events = drained(&mut session);
        assert!(
            matches!(events.as_slice(), [.., FlightEvent::InterfaceLoad(_)]),
            "{events:?}"
        );
        assert!(!events.contains(&FlightEvent::InterfaceLoadsCleared));
        run_expr(&mut session, "H128");
        assert_eq!(session.outfit(buy(OutfitId(300)), &mut NeverFires), Ok(()));
        let events = drained(&mut session);
        assert!(
            events.contains(&FlightEvent::InterfaceLoadsCleared),
            "the order began a run: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, FlightEvent::InterfaceLoad(_))),
            "the H128 before it let go, as the take would: {events:?}"
        );
        assert_eq!(session.take_interface_loads(), []);
    }

    #[test]
    fn the_clear_comes_before_the_loads_of_the_run_it_began() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session.out.begin_docked_run();
        run_expr(&mut session, "H129");
        let events = drained(&mut session);
        let clear = events
            .iter()
            .position(|event| *event == FlightEvent::InterfaceLoadsCleared)
            .expect("the clear");
        let load = events
            .iter()
            .position(|event| matches!(event, FlightEvent::InterfaceLoad(_)))
            .expect("the load");
        assert!(clear < load, "{events:?}");
        assert!(
            !drained(&mut session).contains(&FlightEvent::InterfaceLoadsCleared),
            "once"
        );
    }

    #[test]
    fn the_take_offs_nav_is_drained() {
        let mut session = started();
        land_now(&mut session).expect("lands");
        let landed_nav = session.nav_mode();
        let _ = drained(&mut session);
        session
            .take_off(&catalog(), &mut NeverFires)
            .expect("takes off");
        let events = drained(&mut session);
        assert!(
            events.contains(&FlightEvent::TakeoffNav(landed_nav)),
            "{events:?}"
        );
        assert_eq!(session.take_takeoff_nav(), None);
    }

    #[test]
    fn the_rescues_desc_is_drained() {
        let mut session = started();
        session.ship.fate.rescue_desc_due = true;
        assert_eq!(drained(&mut session), [FlightEvent::RescueDesc(13999)]);
        assert_eq!(session.take_rescue_desc(), None);
    }

    #[test]
    fn the_leave_message_is_drained() {
        let mut session = started();
        session.run.leave_with("Off you go.");
        assert_eq!(
            drained(&mut session),
            [FlightEvent::LeaveMessage("Off you go.".to_owned())]
        );
        assert_eq!(session.take_leave_message(), None);
    }

    #[test]
    fn the_kinds_come_in_the_documented_order() {
        let mut session = started();
        session.ship.fate.rescue_desc_due = true;
        session.run.leave_with("Bye.");
        session.out.takeoff_nav = Some(NavMode::Off);
        session.out.begin_docked_run();
        let bar = queue_text(&mut session, text(128));
        session.out.quotes.push(PersonQuote {
            npc: NpcId(4),
            text: "Hi.".to_owned(),
        });
        session.out.pay_notes.push(PayNote::Defected(1));
        session.out.fighter_notes.push(FighterNote::Abandoned(1));
        session.out.comm.push(note());
        session.out.board_line = Some(MissionBoardLine::Boarded);
        let granted = Granted {
            outfit: OutfitId(128),
            count: 1,
        };
        session.out.granted = Some(granted);
        let diagnostic = SimDiagnostic::UnimplementedGuidance {
            weapon: WeaponId(128),
            guidance: 42,
        };
        session.out.reports.report(diagnostic);
        session.out.hold_expiry_due = true;
        session.out.save_due = true;
        session.out.emit(SessionEvent::ShipChanged);
        let message = SimMessage::Buoy("Hi.".to_owned());
        session.space.say(&mut session.out, message.clone());
        session.out.sounds.push(SimSound::Refused);
        assert_eq!(
            drained(&mut session),
            [
                FlightEvent::Sound(SimSound::Refused),
                FlightEvent::Message(message),
                FlightEvent::Session(SessionEvent::ShipChanged),
                FlightEvent::SaveDue,
                FlightEvent::HoldExpired,
                FlightEvent::Diagnostic(diagnostic),
                FlightEvent::Grant(granted),
                FlightEvent::BoardLine(MissionBoardLine::Boarded),
                FlightEvent::Comm(note()),
                FlightEvent::FighterNote(FighterNote::Abandoned(1)),
                FlightEvent::PayNote(PayNote::Defected(1)),
                FlightEvent::Quote(PersonQuote {
                    npc: NpcId(4),
                    text: "Hi.".to_owned(),
                }),
                FlightEvent::MissionShown {
                    shown: MissionShown::Text(text(128)),
                    bar: Some(bar),
                },
                FlightEvent::InterfaceLoadsCleared,
                FlightEvent::TakeoffNav(NavMode::Off),
                FlightEvent::RescueDesc(13999),
                FlightEvent::LeaveMessage("Bye.".to_owned()),
            ]
        );
    }

    #[test]
    fn a_kind_keeps_the_sessions_order() {
        let mut session = started();
        for sound in [SimSound::Refused, SimSound::NavSelected, SimSound::Refused] {
            session.out.sounds.push(sound);
        }
        session.out.emit(SessionEvent::SystemChanged);
        session.out.emit(SessionEvent::StarfieldReset);
        assert_eq!(
            drained(&mut session),
            [
                FlightEvent::Sound(SimSound::Refused),
                FlightEvent::Sound(SimSound::NavSelected),
                FlightEvent::Sound(SimSound::Refused),
                FlightEvent::Session(SessionEvent::SystemChanged),
                FlightEvent::Session(SessionEvent::StarfieldReset),
            ]
        );
    }

    #[test]
    fn a_second_drain_adds_nothing_and_keeps_what_was_there() {
        let mut session = started();
        session.out.sounds.push(SimSound::Refused);
        session.out.save_due = true;
        let mut into = vec![FlightEvent::HoldExpired];
        session.drain_events(&mut into);
        assert_eq!(
            into,
            [
                FlightEvent::HoldExpired,
                FlightEvent::Sound(SimSound::Refused),
                FlightEvent::SaveDue,
            ],
            "appended"
        );
        session.drain_events(&mut into);
        assert_eq!(into.len(), 3, "nothing more");
    }

    fn run_expr(session: &mut Session, text: &str) {
        let expr = crate::control::SetExpr::parse(text).expect("parses");
        session.run_set(&expr, &mut NeverFires);
    }
}
