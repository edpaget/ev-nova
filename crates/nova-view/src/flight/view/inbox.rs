//! [`Inbox`]: what the view drained from its flight ([`FlightSim::drain`])
//! and has not taken yet, by kind.
//!
//! Every place the view used to take a kind of one-shot output from the
//! session drains the flight into the inbox, then takes that kind from it,
//! at the same place in the code. For each kind, what the flight still
//! holds plus what the inbox holds is what the session alone held before,
//! so each take gives what it gave, however often the view drained in
//! between. Lists append; flags and `Option`s keep the latest value, as
//! the session's do. The session's own reset of the interface loads rides
//! the stream ([`FlightEvent::InterfaceLoadsCleared`]): the inbox drops
//! its untaken loads there. And the mission texts carry the bar each
//! noted, so the inbox answers [`Inbox::text_bars`] and
//! [`Inbox::text_nav`] over the texts not yet taken, as the session's did.
//!
//! **One ordering difference, no boundary crossed.** The session gives
//! its diagnostics the fight's first, then its own reports; drained
//! twice before a take, the inbox holds them in the order of the drains,
//! so a fight's diagnostic and a report can interleave in the app's log.
//! Nothing the player sees depends on it.

use nova_sim::flight_frame::FlightEvent;
use nova_sim::{
    CommNote, FighterNote, Granted, InterfaceLoad, MissionBoardLine, MissionShown, NavMode,
    PayNote, PersonQuote, SessionEvent, SimDiagnostic, SimMessage, SimSound, TextBar,
};

use crate::flight::sim_port::FlightSim;

/// What the view drained from its flight and has not taken yet (see the
/// module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Inbox {
    /// The sounds, in order.
    sounds: Vec<SimSound>,
    /// The messages, in order.
    messages: Vec<SimMessage>,
    /// The session events, in order.
    events: Vec<SessionEvent>,
    /// Whether a save fell due.
    save_due: bool,
    /// Whether the hold after the player's explosion ran out.
    hold_expired: bool,
    /// The diagnostics, in the order drained.
    diagnostics: Vec<SimDiagnostic>,
    /// What the last boarding granted.
    grant: Option<Granted>,
    /// The last board's line.
    board_line: Option<MissionBoardLine>,
    /// The comm notes, in order.
    comm: Vec<CommNote>,
    /// The fighter notes, in order.
    fighter_notes: Vec<FighterNote>,
    /// The pay notes, in order.
    pay_notes: Vec<PayNote>,
    /// The hail quotes, in order.
    quotes: Vec<PersonQuote>,
    /// The mission texts and offer movies, each text with its bar.
    texts: Vec<(MissionShown, Option<TextBar>)>,
    /// The interface loads since the last run of docked scripts began.
    interface_loads: Vec<InterfaceLoad>,
    /// The nav mode as the last take-off began.
    takeoff_nav: Option<NavMode>,
    /// The rescue's `dësc`.
    rescue_desc: Option<i16>,
    /// The message the last take-off left on.
    leave_message: Option<String>,
}

impl Inbox {
    /// Drains `sim` into the inbox ([`FlightSim::drain`]).
    pub(super) fn drain(&mut self, sim: &mut impl FlightSim) {
        let mut events = Vec::new();
        sim.drain(&mut events);
        self.receive(events);
    }

    /// Holds each of `events` by its kind (see the module docs).
    pub(super) fn receive(&mut self, events: Vec<FlightEvent>) {
        for event in events {
            match event {
                FlightEvent::Sound(sound) => self.sounds.push(sound),
                FlightEvent::Message(message) => self.messages.push(message),
                FlightEvent::Session(event) => self.events.push(event),
                FlightEvent::SaveDue => self.save_due = true,
                FlightEvent::HoldExpired => self.hold_expired = true,
                FlightEvent::Diagnostic(diagnostic) => self.diagnostics.push(diagnostic),
                FlightEvent::Grant(granted) => self.grant = Some(granted),
                FlightEvent::BoardLine(line) => self.board_line = Some(line),
                FlightEvent::Comm(note) => self.comm.push(note),
                FlightEvent::FighterNote(note) => self.fighter_notes.push(note),
                FlightEvent::PayNote(note) => self.pay_notes.push(note),
                FlightEvent::Quote(quote) => self.quotes.push(quote),
                FlightEvent::MissionShown { shown, bar } => self.texts.push((shown, bar)),
                FlightEvent::InterfaceLoadsCleared => self.interface_loads.clear(),
                FlightEvent::InterfaceLoad(load) => self.interface_loads.push(load),
                FlightEvent::TakeoffNav(nav) => self.takeoff_nav = Some(nav),
                FlightEvent::RescueDesc(desc) => self.rescue_desc = Some(desc),
                FlightEvent::LeaveMessage(text) => self.leave_message = Some(text),
            }
        }
    }

    /// The sounds, in order, but those `held` says to keep, which stay,
    /// in order, for a later take.
    pub(super) fn take_sounds(&mut self, held: impl Fn(SimSound) -> bool) -> Vec<SimSound> {
        let (kept, taken) = std::mem::take(&mut self.sounds)
            .into_iter()
            .partition(|&sound| held(sound));
        self.sounds = kept;
        taken
    }

    /// The messages, in order; taking them empties the list.
    pub(super) fn take_messages(&mut self) -> Vec<SimMessage> {
        std::mem::take(&mut self.messages)
    }

    /// The session events, in order; taking them empties the list.
    pub(super) fn take_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }

    /// Whether a save fell due; taking it clears it.
    pub(super) fn take_save_due(&mut self) -> bool {
        std::mem::take(&mut self.save_due)
    }

    /// Whether the hold ran out; true once.
    pub(super) fn take_hold_expired(&mut self) -> bool {
        std::mem::take(&mut self.hold_expired)
    }

    /// The diagnostics, in the order drained; taking them empties the
    /// list.
    pub(super) fn take_diagnostics(&mut self) -> Vec<SimDiagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// What the last boarding granted, once.
    pub(super) fn take_grant(&mut self) -> Option<Granted> {
        self.grant.take()
    }

    /// The last board's line, once.
    pub(super) fn take_board_line(&mut self) -> Option<MissionBoardLine> {
        self.board_line.take()
    }

    /// The comm notes, in order; taking them empties the list.
    pub(super) fn take_comm(&mut self) -> Vec<CommNote> {
        std::mem::take(&mut self.comm)
    }

    /// The fighter notes, in order; taking them empties the list.
    pub(super) fn take_fighter_notes(&mut self) -> Vec<FighterNote> {
        std::mem::take(&mut self.fighter_notes)
    }

    /// The pay notes, in order; taking them empties the list.
    pub(super) fn take_pay_notes(&mut self) -> Vec<PayNote> {
        std::mem::take(&mut self.pay_notes)
    }

    /// The hail quotes, in order; taking them empties the list.
    pub(super) fn take_quotes(&mut self) -> Vec<PersonQuote> {
        std::mem::take(&mut self.quotes)
    }

    /// The mission texts and offer movies, in order; taking them lets go
    /// of their bars ([`Inbox::text_bars`]).
    pub(super) fn take_mission_texts(&mut self) -> Vec<MissionShown> {
        std::mem::take(&mut self.texts)
            .into_iter()
            .map(|(shown, _)| shown)
            .collect()
    }

    /// The bar each text not yet taken noted as it was queued, in order:
    /// one for each text, none for a movie.
    pub(super) fn text_bars(&self) -> Vec<TextBar> {
        self.texts.iter().filter_map(|(_, bar)| *bar).collect()
    }

    /// The nav mode as the last text not yet taken was queued, `None` with
    /// none: the last of [`Inbox::text_bars`]'.
    pub(super) fn text_nav(&self) -> Option<NavMode> {
        self.texts
            .iter()
            .rev()
            .find_map(|(_, bar)| *bar)
            .map(|bar| bar.nav)
    }

    /// The interface loads since the last run of docked scripts began, in
    /// order; taking them empties the list.
    pub(super) fn take_interface_loads(&mut self) -> Vec<InterfaceLoad> {
        std::mem::take(&mut self.interface_loads)
    }

    /// The nav mode as the last take-off began, once.
    pub(super) fn take_takeoff_nav(&mut self) -> Option<NavMode> {
        self.takeoff_nav.take()
    }

    /// The rescue's `dësc`, once.
    pub(super) fn take_rescue_desc(&mut self) -> Option<i16> {
        self.rescue_desc.take()
    }

    /// The message the last take-off left on, once.
    pub(super) fn take_leave_message(&mut self) -> Option<String> {
        self.leave_message.take()
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::{
        Help, MissionId, MissionText, MissionTextKind, NpcId, OutfitId, Reserves, StellarId,
        SystemId, WeaponId,
    };

    use super::*;
    use crate::flight::sim_port::fixture::FakeFlight;

    fn received(events: Vec<FlightEvent>) -> Inbox {
        let mut inbox = Inbox::default();
        inbox.receive(events);
        inbox
    }

    fn text(mission: i16) -> MissionShown {
        MissionShown::Text(MissionText {
            kind: MissionTextKind::Brief,
            mission: MissionId(mission),
            text: format!("text {mission}"),
            graphic: None,
            movie: None,
            map: None,
        })
    }

    fn movie() -> MissionShown {
        MissionShown::Movie(nova_sim::DescMovie {
            file: "offer.mov".to_owned(),
            after: true,
            double_size: false,
            cinematic: false,
        })
    }

    fn bar(texts: u64, nav: NavMode) -> TextBar {
        TextBar {
            texts,
            nav,
            reserves: Reserves::default(),
        }
    }

    fn load(texts: u64) -> InterfaceLoad {
        InterfaceLoad {
            government: None,
            nav: NavMode::Off,
            reserves: Reserves::default(),
            texts,
        }
    }

    fn note(from: u32) -> CommNote {
        CommNote {
            from: NpcId(from),
            comm_name: "Shuttle".to_owned(),
            ship_name: "Kestrel".to_owned(),
            done: Help::Repair,
        }
    }

    #[test]
    fn an_empty_inbox_gives_nothing() {
        let mut inbox = Inbox::default();
        assert_eq!(inbox.take_sounds(|_| false), []);
        assert_eq!(inbox.take_messages(), []);
        assert_eq!(inbox.take_events(), []);
        assert!(!inbox.take_save_due());
        assert!(!inbox.take_hold_expired());
        assert_eq!(inbox.take_diagnostics(), []);
        assert_eq!(inbox.take_grant(), None);
        assert_eq!(inbox.take_board_line(), None);
        assert_eq!(inbox.take_comm(), []);
        assert_eq!(inbox.take_fighter_notes(), []);
        assert_eq!(inbox.take_pay_notes(), []);
        assert_eq!(inbox.take_quotes(), []);
        assert_eq!(inbox.take_mission_texts(), []);
        assert_eq!(inbox.text_bars(), []);
        assert_eq!(inbox.text_nav(), None);
        assert_eq!(inbox.take_interface_loads(), []);
        assert_eq!(inbox.take_takeoff_nav(), None);
        assert_eq!(inbox.take_rescue_desc(), None);
        assert_eq!(inbox.take_leave_message(), None);
    }

    #[test]
    fn each_list_is_taken_in_order_across_receives_once() {
        let mut inbox = received(vec![
            FlightEvent::Sound(SimSound::Refused),
            FlightEvent::Message(SimMessage::Buoy("one".to_owned())),
            FlightEvent::Session(SessionEvent::ShipChanged),
            FlightEvent::Diagnostic(SimDiagnostic::UnimplementedGuidance {
                weapon: WeaponId(128),
                guidance: 1,
            }),
            FlightEvent::Comm(note(1)),
            FlightEvent::FighterNote(FighterNote::Abandoned(1)),
            FlightEvent::PayNote(PayNote::Defected(1)),
            FlightEvent::Quote(PersonQuote {
                npc: NpcId(1),
                text: "one".to_owned(),
            }),
        ]);
        inbox.receive(vec![
            FlightEvent::Sound(SimSound::NavSelected),
            FlightEvent::Message(SimMessage::Buoy("two".to_owned())),
            FlightEvent::Session(SessionEvent::SystemChanged),
            FlightEvent::Diagnostic(SimDiagnostic::UnimplementedGuidance {
                weapon: WeaponId(129),
                guidance: 2,
            }),
            FlightEvent::Comm(note(2)),
            FlightEvent::FighterNote(FighterNote::Abandoned(2)),
            FlightEvent::PayNote(PayNote::Defected(2)),
            FlightEvent::Quote(PersonQuote {
                npc: NpcId(2),
                text: "two".to_owned(),
            }),
        ]);
        assert_eq!(
            inbox.take_sounds(|_| false),
            [SimSound::Refused, SimSound::NavSelected]
        );
        assert_eq!(
            inbox.take_messages(),
            [
                SimMessage::Buoy("one".to_owned()),
                SimMessage::Buoy("two".to_owned())
            ]
        );
        assert_eq!(
            inbox.take_events(),
            [SessionEvent::ShipChanged, SessionEvent::SystemChanged]
        );
        assert_eq!(
            inbox.take_diagnostics(),
            [
                SimDiagnostic::UnimplementedGuidance {
                    weapon: WeaponId(128),
                    guidance: 1
                },
                SimDiagnostic::UnimplementedGuidance {
                    weapon: WeaponId(129),
                    guidance: 2
                }
            ]
        );
        assert_eq!(inbox.take_comm(), [note(1), note(2)]);
        assert_eq!(
            inbox.take_fighter_notes(),
            [FighterNote::Abandoned(1), FighterNote::Abandoned(2)]
        );
        assert_eq!(
            inbox.take_pay_notes(),
            [PayNote::Defected(1), PayNote::Defected(2)]
        );
        let quotes: Vec<_> = inbox.take_quotes().into_iter().map(|q| q.text).collect();
        assert_eq!(quotes, ["one", "two"]);
        assert_eq!(inbox, Inbox::default(), "all taken");
    }

    #[test]
    fn the_flags_are_taken_once() {
        let mut inbox = received(vec![
            FlightEvent::SaveDue,
            FlightEvent::HoldExpired,
            FlightEvent::SaveDue,
        ]);
        assert!(inbox.take_save_due());
        assert!(!inbox.take_save_due(), "once");
        assert!(inbox.take_hold_expired());
        assert!(!inbox.take_hold_expired(), "once");
    }

    #[test]
    fn the_option_kinds_keep_the_latest_and_are_taken_once() {
        let mut inbox = received(vec![
            FlightEvent::Grant(Granted {
                outfit: OutfitId(128),
                count: 1,
            }),
            FlightEvent::BoardLine(MissionBoardLine::Boarded),
            FlightEvent::TakeoffNav(NavMode::Off),
            FlightEvent::RescueDesc(13999),
            FlightEvent::LeaveMessage("first".to_owned()),
        ]);
        let later = Granted {
            outfit: OutfitId(129),
            count: 2,
        };
        let refuelled = MissionBoardLine::Refuelled {
            ship: nova_sim::ShipId(128),
        };
        let nav = NavMode::Stellar(Some(StellarId(128)));
        inbox.receive(vec![
            FlightEvent::Grant(later),
            FlightEvent::BoardLine(refuelled.clone()),
            FlightEvent::TakeoffNav(nav),
            FlightEvent::RescueDesc(14000),
            FlightEvent::LeaveMessage("second".to_owned()),
        ]);
        assert_eq!(inbox.take_grant(), Some(later));
        assert_eq!(inbox.take_board_line(), Some(refuelled));
        assert_eq!(inbox.take_takeoff_nav(), Some(nav));
        assert_eq!(inbox.take_rescue_desc(), Some(14000));
        assert_eq!(inbox.take_leave_message().as_deref(), Some("second"));
        assert_eq!(inbox, Inbox::default(), "each once");
    }

    #[test]
    fn a_clear_drops_the_loads_before_it_and_keeps_those_after() {
        let mut inbox = received(vec![
            FlightEvent::InterfaceLoad(load(1)),
            FlightEvent::InterfaceLoad(load(2)),
        ]);
        inbox.receive(vec![
            FlightEvent::InterfaceLoadsCleared,
            FlightEvent::InterfaceLoad(load(3)),
        ]);
        assert_eq!(inbox.take_interface_loads(), [load(3)]);
        assert_eq!(inbox.take_interface_loads(), [], "taken");
    }

    #[test]
    fn the_bars_and_text_nav_are_over_the_texts_not_yet_taken() {
        let first = bar(1, NavMode::Off);
        let second = bar(2, NavMode::Hyperspace(Some(SystemId(130))));
        let mut inbox = received(vec![
            FlightEvent::MissionShown {
                shown: text(128),
                bar: Some(first),
            },
            FlightEvent::MissionShown {
                shown: text(129),
                bar: Some(second),
            },
            FlightEvent::MissionShown {
                shown: movie(),
                bar: None,
            },
        ]);
        assert_eq!(inbox.text_bars(), [first, second], "none for the movie");
        assert_eq!(
            inbox.text_nav(),
            Some(second.nav),
            "the last text's, past the movie"
        );
        assert_eq!(inbox.take_mission_texts(), [text(128), text(129), movie()]);
        assert_eq!(inbox.text_bars(), [], "taken with the texts");
        assert_eq!(inbox.text_nav(), None);
    }

    #[test]
    fn a_drain_holds_what_the_flight_hands_out() {
        let mut fake = FakeFlight::default();
        fake.queue_event(FlightEvent::Sound(SimSound::Refused));
        fake.queue_event(FlightEvent::SaveDue);
        let mut inbox = Inbox::default();
        inbox.drain(&mut fake);
        assert_eq!(inbox.take_sounds(|_| false), [SimSound::Refused]);
        assert!(inbox.take_save_due());
        inbox.drain(&mut fake);
        assert_eq!(inbox, Inbox::default(), "the flight handed them out once");
    }
}
