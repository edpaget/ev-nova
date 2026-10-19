//! The player's dock request and the dock approach (see
//! [`clearance`](crate::clearance) for the rules): how the land key's
//! press is applied ([`Session::land`]), the approach run on every flight
//! frame, and every reset of the request. All addresses are in `EV
//! Nova.app`'s `EV Nova` (i386).
//!
//! **The press** ([`Session::land`]) applies what
//! [`land_or_select`](crate::land_or_select) gives: the stellar it
//! selected, the request after it, and what it said. "No response." is
//! raised as [`SimMessage::NoResponse`], with no sound (@0x66bcd). The
//! request's line is [`SimMessage::DockRequested`] with
//! [`SimSound::TrafficControl`], `snd ` 151 (@0x67a69), and a denial
//! raises the same sound (@0x67ec6), the view wording the refusal.
//!
//! **The approach** ([`Session::dock_approach`]) runs in the player's
//! part of every game-work pass, so on both of double time's passes, as
//! `_HandlePlayer` runs on both, and whatever the ship's condition, after
//! the ship moves. The original reaches the approach (@0x6a97a) before
//! its position step (@0x6d4b0): @0x6864d, once read as the move, is
//! `_WrapPlayerPosition` (rdm weapons-fidelity phase 28's findings; filed
//! as a hud-fidelity-followups phase). It is
//! skipped while the land key is held ([`Session::set_land_key`], the
//! original's `_dockTryFlag`, @0x6a965-0x6a97a). A request whose stellar
//! is no longer among the system's is dropped (@0x647c5-0x647e3);
//! otherwise [`approach`] counts it, and its
//! greeting raises [`SimMessage::DockCleared`] and
//! [`SimSound::TrafficControl`] (@0x649a9), and its expiry drops it
//! (@0x64fd3-0x64ff0).
//!
//! **The resets**: the request is dropped
//!
//! - at a flight start (`_InitObjects` @0x1d071-0x1d085, `_ResetPlayer`
//!   @0x1d774-0x1d77e, `_ResetGameData` @0x1df6e-0x1df78): a session
//!   starts with none;
//! - on Nav Off and Hyperspace Mode (@0x69b1e-0x69b3c,
//!   [`Session::nav_mode_keys`]);
//! - when a stellar key or a click changes the selection or the mode
//!   (@0x69efd-0x69f12, @0x6a6e7-0x6a6fc, [`Session::select_stellar`]),
//!   not when it finds the stellar already selected in mode 2
//!   (@0x69ec4-0x69ed6);
//! - on every frame in nav mode 3 (@0x6affd-0x6b017), after the approach
//!   as the original runs it;
//! - on a hyperspace arrival (@0x6bfb5-0x6bfc4; the `entry` module's
//!   table);
//! - on the take-off (`_PlayerLandOnStellar` @0x6374a-0x63759);
//! - when the press lands or reaches a gate. The original sets the
//!   counter to 0 there (@0x6705f, @0x66ef4-0x66f03) and leaves the
//!   stellar until the take-off or the system check; no flight frame
//!   runs between, so dropping it is the same.
//!
//! A script's `M` or `N` lets go of the selection but not of the request
//! (`_EvalSetExp` @0x157e3, @0x158fd), so a counter left at 0x2ee or more
//! clears the next request at another stellar
//! ([`Standing::granted`](crate::Standing::granted)).
//!
//! Left out: the bribe's counter and the domination writes
//! (`_DoPlanetCommDialog` @0x97112-0x97125, @0x973e1-0x973fe,
//! @0x978be-0x978d3), rdm:task/planet-domination's.
//!
//! [`gate_sprite::held_open`](crate::gate_sprite::held_open) reads the
//! counter for the player's dock hypergate (@0x2e867-0x2e87e).
//!
//! **Ordering.** The original's request sets `_dockTryFlag`
//! (@0x6a938-0x6a956), so its approach never runs in the frame of the
//! request, only from the first frame the land key is up. Ours runs the
//! request on the press, between ticks, and the approach from the next
//! tick the land key is up, so the two agree.

use super::{Outbox, Session};
use crate::clearance::{DockKind, approach, approach_clears};
use crate::landing::{DockPress, DockSaid, LandOutcome, LandingRefusal};
use crate::message::SimMessage;
use crate::navigation::NavMode;
use crate::sound::SimSound;

impl Session {
    /// Takes the land key's state: `on` while it is held, which holds the
    /// dock approach back (the original's `_dockTryFlag`, see the
    /// `clearance` module). Up until it says otherwise.
    pub fn set_land_key(&mut self, on: bool) {
        self.land_key = on;
    }

    /// The dock request, if any: the stellar requested and its counter
    /// (see the `clearance` module).
    #[must_use]
    pub fn dock_request(&self) -> Option<crate::clearance::DockRequest> {
        self.dock
    }

    /// Applies the land key's `press`: the stellar it selected, in mode
    /// 2 with the brackets' fly-in, the request after it, and what it
    /// said, and gives its outcome.
    pub(super) fn apply_press(&mut self, press: DockPress) -> Result<LandOutcome, LandingRefusal> {
        self.select_pressed(press.selected);
        self.dock = press.dock;
        match press.said {
            Some(DockSaid::NoResponse) => self.say(SimMessage::NoResponse),
            Some(DockSaid::Requested) => {
                if let Ok(LandOutcome::Requested { stellar, .. }) = press.outcome {
                    let kind = self.dock_kind(stellar);
                    self.out.sounds.push(SimSound::TrafficControl);
                    self.say(SimMessage::DockRequested { stellar, kind });
                }
            }
            None => {}
        }
        if let Err(LandingRefusal::Denied { .. }) = press.outcome {
            self.out.sounds.push(SimSound::TrafficControl);
        }
        press.outcome
    }

    /// Selects the stellar the land key `selected`, if it selected one, in
    /// mode 2 with the brackets' fly-in (@0x6a8ed-0x6a932).
    pub(super) fn select_pressed(&mut self, selected: Option<crate::catalog::StellarId>) {
        if let Some(stellar) = selected {
            self.set_nav(NavMode::Stellar(Some(stellar)));
            self.nav_fly_in.start();
        }
    }

    /// How the request's line and the greeting word `stellar`, one of the
    /// system's.
    fn dock_kind(&self, stellar: crate::catalog::StellarId) -> DockKind {
        self.sites
            .iter()
            .find(|site| site.id == stellar)
            .map_or(DockKind::Planet, DockKind::of_site)
    }

    /// One frame of the dock approach, then nav mode 3's reset (see the
    /// module docs).
    pub(crate) fn dock_approach(&mut self) {
        if !self.land_key
            && let Some(mut request) = self.dock
        {
            match self.sites.iter().find(|site| site.id == request.stellar) {
                None => self.dock = None,
                Some(site) => {
                    let blessing = self.blessing();
                    let clears =
                        approach_clears(site, self.standing(site, &blessing), request.permission);
                    let offset = self.player.position - site.position;
                    let done = approach(&mut request, site, offset, clears);
                    let kind = DockKind::of_site(site);
                    self.dock = (!done.expired).then_some(request);
                    if done.greet {
                        self.out.sounds.push(SimSound::TrafficControl);
                        self.say(SimMessage::DockCleared {
                            stellar: request.stellar,
                            kind,
                        });
                    }
                }
            }
        }
        if matches!(self.nav, NavMode::Hyperspace(_)) {
            self.dock = None;
        }
    }

    /// How many messages and sounds are queued: where
    /// [`Session::forget_dock_chatter`] starts forgetting.
    #[cfg(test)]
    pub(crate) fn chatter_marks(&self) -> (usize, usize) {
        self.out.chatter_marks()
    }

    /// Forgets the dock request's messages and traffic control's sound
    /// queued since `marks` ([`Session::chatter_marks`]), for fixtures that
    /// land without pinning them.
    #[cfg(test)]
    pub(crate) fn forget_dock_chatter(&mut self, marks: (usize, usize)) {
        self.out.forget_dock_chatter(marks);
    }
}

impl Outbox {
    /// How many messages and sounds are queued: where
    /// [`Outbox::forget_dock_chatter`] starts forgetting.
    #[cfg(test)]
    fn chatter_marks(&self) -> (usize, usize) {
        (self.messages.len(), self.sounds.len())
    }

    /// Forgets the dock request's messages and traffic control's sound
    /// queued since `marks` ([`Outbox::chatter_marks`]), for fixtures that
    /// land without pinning them.
    #[cfg(test)]
    fn forget_dock_chatter(&mut self, marks: (usize, usize)) {
        let (messages, sounds) = marks;
        let mut index = 0;
        self.messages.retain(|message| {
            index += 1;
            index <= messages
                || !matches!(
                    message,
                    SimMessage::DockRequested { .. }
                        | SimMessage::NoResponse
                        | SimMessage::DockCleared { .. }
                )
        });
        let mut index = 0;
        self.sounds.retain(|sound| {
            index += 1;
            index <= sounds || *sound != SimSound::TrafficControl
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StellarPick;
    use crate::catalog::{GovtId, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::clearance::{CLEARED, DockRequest, REQUESTED_NEAR};
    use crate::combat::Rules;
    use crate::combat::hull::Condition;
    use crate::flight::Controls;
    use crate::gate::{GateKind, HYPERGATE, WORMHOLE};
    use crate::geometry::Vec2;
    use crate::landing::StellarFlags;
    use crate::session::LandPress;
    use crate::testkit::{FakePilotCatalog, catalog, jump, land_now, planet};

    /// Planet 128 at (30, −40), under the ship, and planet 129 far off at
    /// (2000, 0).
    fn session() -> Session {
        quiet(Session::start(&catalog()).expect("starts"))
    }

    /// `session` with its messages and sounds taken.
    fn quiet(mut session: Session) -> Session {
        session.take_messages();
        session.take_sounds();
        session
    }

    fn tick(session: &mut Session) {
        session.tick(Controls::default());
    }

    // Wrapped to compare with what a press gives.
    #[allow(clippy::unnecessary_wraps)]
    fn requested(stellar: i16) -> Result<LandPress, LandingRefusal> {
        Ok(LandOutcome::Requested {
            stellar: StellarId(stellar),
            station: false,
        }
        .into())
    }

    // Wrapped to compare with what a press gives.
    #[allow(clippy::unnecessary_wraps)]
    fn landed(stellar: i16) -> Result<LandPress, LandingRefusal> {
        Ok(LandOutcome::Landed(StellarId(stellar)).into())
    }

    // Wrapped to compare with what a press gives.
    #[allow(clippy::unnecessary_wraps)]
    fn request(stellar: i16, permission: i16) -> Option<DockRequest> {
        Some(DockRequest {
            stellar: StellarId(stellar),
            permission,
        })
    }

    fn cleared_at(stellar: i16, kind: DockKind) -> SimMessage {
        SimMessage::DockCleared {
            stellar: StellarId(stellar),
            kind,
        }
    }

    /// `session` with planet 129 selected by its slot, the key's beep
    /// taken.
    fn on_129(mut session: Session) -> Session {
        session.select_stellar(StellarPick::Slot(1));
        quiet(session)
    }

    // The request.

    #[test]
    fn a_request_away_from_the_stellar_says_its_line_with_traffic_controls_sound() {
        let mut session = on_129(session());
        assert_eq!(session.land(), requested(129));
        assert_eq!(
            session.take_messages(),
            [SimMessage::DockRequested {
                stellar: StellarId(129),
                kind: DockKind::Planet,
            }]
        );
        assert_eq!(session.take_sounds(), [SimSound::TrafficControl]);
        assert_eq!(session.dock_request(), request(129, 0));
    }

    #[test]
    fn the_requests_line_holds_the_comm_line() {
        let mut session = on_129(session());
        assert!(session.comm_line().free_for_calls(), "free before");
        assert_eq!(session.land(), requested(129));
        assert!(!session.comm_line().free_for_calls(), "the line shows it");
    }

    #[test]
    fn the_requests_line_words_the_stellar_requested_by_its_kind() {
        let mut catalog = catalog();
        catalog.sites[0].1[1].flags |= StellarFlags::STATION;
        let mut session = on_129(quiet(Session::start(&catalog).expect("starts")));
        assert_eq!(
            session.land(),
            Ok(LandOutcome::Requested {
                stellar: StellarId(129),
                station: true,
            }
            .into())
        );
        assert_eq!(
            session.take_messages(),
            [SimMessage::DockRequested {
                stellar: StellarId(129),
                kind: DockKind::Station,
            }],
            "not planet 128's kind"
        );
    }

    #[test]
    fn a_request_over_the_stellar_is_silent_and_the_next_tick_greets() {
        let mut session = session();
        assert_eq!(session.land(), requested(128));
        assert_eq!(session.take_messages(), []);
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.dock_request(), request(128, REQUESTED_NEAR));
        tick(&mut session);
        assert_eq!(session.take_messages(), [cleared_at(128, DockKind::Planet)]);
        assert_eq!(session.take_sounds(), [SimSound::TrafficControl]);
        assert_eq!(session.dock_request(), request(128, CLEARED));
    }

    /// [`catalog`] whose planet 128 needs a record no pilot has.
    fn strict() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.sites[0].1[0].min_status = 32767;
        catalog
    }

    #[test]
    fn a_denial_sounds_traffic_control_drops_the_request_and_the_next_press_asks_again() {
        let mut session = quiet(Session::start(&strict()).expect("starts"));
        session.dock = request(129, CLEARED - 1);
        let denied = Err(LandingRefusal::Denied {
            stellar: StellarId(128),
            station: false,
            min_status: 32767,
        });
        assert_eq!(session.land(), denied);
        assert_eq!(session.take_sounds(), [SimSound::TrafficControl]);
        assert_eq!(session.take_messages(), [], "the view words the refusal");
        assert_eq!(session.dock_request(), None);
        assert_eq!(session.land(), denied, "asked again");
        assert_eq!(session.take_sounds(), [SimSound::TrafficControl]);
    }

    // The approach.

    #[test]
    fn the_approach_greets_once_on_the_tick_the_ship_comes_within_249_on_both_axes() {
        let mut session = on_129(session());
        session.land().expect("requests");
        quiet_in_place(&mut session);
        for at in [
            Vec2::new(2000.0 - 250.0, 0.0),
            Vec2::new(2000.0, 250.0),
            Vec2::new(2000.0, -250.0),
            Vec2::new(2000.0 + 250.0, 10.0),
        ] {
            session.player.position = at;
            tick(&mut session);
            assert_eq!(session.take_messages(), [], "{at:?}");
            assert_eq!(session.take_sounds(), [], "{at:?}");
            assert_eq!(session.dock_request(), request(129, 0), "{at:?}");
        }
        session.player.position = Vec2::new(2000.0 - 249.9, 249.9);
        tick(&mut session);
        assert_eq!(session.take_messages(), [cleared_at(129, DockKind::Planet)]);
        assert_eq!(session.take_sounds(), [SimSound::TrafficControl]);
        for _ in 0..3 {
            tick(&mut session);
        }
        assert_eq!(session.take_messages(), [], "once");
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.dock_request(), request(129, CLEARED + 3));
    }

    /// Takes `session`'s messages and sounds.
    fn quiet_in_place(session: &mut Session) {
        session.take_messages();
        session.take_sounds();
    }

    #[test]
    fn the_land_press_reads_the_counter_not_traffic_control() {
        let mut session = session();
        session.land().expect("requests");
        tick(&mut session);
        session.sites[0].min_status = 32767;
        assert_eq!(session.land(), landed(128), "after the greeting");

        // A far request, then the ship moved over the stellar with no
        // tick between: too far.
        let mut session = on_129(self::session());
        session.land().expect("requests");
        session.player.position = Vec2::new(2000.0, 0.0);
        assert_eq!(
            session.land(),
            Err(LandingRefusal::TooFar {
                stellar: StellarId(129),
                station: false,
            })
        );
        assert_eq!(session.dock_request(), request(129, 0), "kept");
        tick(&mut session);
        assert_eq!(session.land(), landed(129), "after the approach");
    }

    #[test]
    fn the_land_key_held_holds_the_approach_back() {
        let mut session = session();
        session.land().expect("requests");
        session.set_land_key(true);
        for _ in 0..5 {
            tick(&mut session);
        }
        assert_eq!(session.take_messages(), []);
        assert_eq!(session.dock_request(), request(128, REQUESTED_NEAR));
        session.set_land_key(false);
        tick(&mut session);
        assert_eq!(session.take_messages(), [cleared_at(128, DockKind::Planet)]);
    }

    /// `session` requested over 128 and greeted, its messages taken.
    fn greeted(mut session: Session) -> Session {
        session.land().expect("requests");
        tick(&mut session);
        assert_eq!(session.dock_request(), request(128, CLEARED));
        quiet(session)
    }

    #[test]
    fn the_request_expires_1298_ticks_after_the_greeting() {
        let mut session = greeted(session());
        for _ in 0..1297 {
            tick(&mut session);
        }
        assert_eq!(session.dock_request(), request(128, 0x7ff));
        let mut kept = session.clone();
        assert_eq!(kept.land(), landed(128));
        tick(&mut session);
        assert_eq!(session.dock_request(), None);
        assert_eq!(session.land(), requested(128), "requested afresh");
    }

    #[test]
    fn in_double_time_the_approach_runs_on_both_passes() {
        let mut session = session();
        session.land().expect("requests");
        session.set_double_time_key(true);
        session.tick(Controls::default());
        assert_eq!(session.dock_request(), request(128, CLEARED));
        session.tick_again(
            Controls::default(),
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(session.dock_request(), request(128, CLEARED + 1));
        assert_eq!(
            session.take_messages(),
            [cleared_at(128, DockKind::Planet)],
            "greeted once"
        );
        for _ in 0..648 {
            session.tick(Controls::default());
            session.tick_again(
                Controls::default(),
                &crate::ai::NovaAi::default(),
                Rules::default(),
                &mut NeverFires,
            );
        }
        assert_eq!(session.dock_request(), request(128, 0x7ff));
        session.tick(Controls::default());
        assert_eq!(session.dock_request(), None, "half the frames");
    }

    #[test]
    fn a_move_to_another_system_drops_the_request_on_the_next_tick() {
        let catalog = catalog();
        let mut session = greeted(session());
        session.run_set(
            &crate::control::SetExpr::parse("M131").expect("parses"),
            &mut NeverFires,
        );
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.dock_request(), request(128, CLEARED), "kept");
        tick(&mut session);
        assert_eq!(session.dock_request(), None);
    }

    #[test]
    fn a_disabled_players_approach_still_greets() {
        let mut session = session();
        session.land().expect("requests");
        session.condition = Condition::Disabled;
        tick(&mut session);
        assert_eq!(session.take_messages(), [cleared_at(128, DockKind::Planet)]);
    }

    // The resets.

    #[test]
    fn nav_off_drops_the_request() {
        let mut session = greeted(session());
        session.nav_off();
        assert_eq!(session.dock_request(), None);
        assert_eq!(session.land(), requested(128));
    }

    #[test]
    fn a_stellar_key_selecting_another_stellar_drops_the_request() {
        let mut session = greeted(session());
        session.select_stellar(StellarPick::Slot(1));
        assert_eq!(session.dock_request(), None);
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(session.land(), requested(128));
    }

    #[test]
    fn a_stellar_key_on_the_stellar_selected_keeps_the_request() {
        let mut session = greeted(session());
        for pick in [StellarPick::Slot(0), StellarPick::Nearest] {
            session.select_stellar(pick);
            assert_eq!(session.dock_request(), request(128, CLEARED), "{pick:?}");
        }
        crate::testkit::click_stellar(&mut session, Vec2::new(30.0, -40.0));
        assert_eq!(session.dock_request(), request(128, CLEARED), "a click");
        assert_eq!(session.land(), landed(128));
    }

    #[test]
    fn a_click_on_another_stellar_drops_the_request() {
        let mut session = greeted(session());
        crate::testkit::click_stellar(&mut session, Vec2::new(2000.0, 0.0));
        assert_eq!(session.dock_request(), None);
    }

    #[test]
    fn a_tick_in_hyperspace_mode_drops_the_request() {
        let mut session = greeted(session());
        session.hyperspace_mode();
        assert_eq!(session.dock_request(), None, "H alone");
        session.dock = request(128, CLEARED);
        session.set_land_key(true);
        tick(&mut session);
        assert_eq!(session.dock_request(), None, "even with L held");
        session.set_land_key(false);
        assert_eq!(session.land(), requested(128));
    }

    #[test]
    fn a_hyperspace_arrival_drops_the_request() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        crate::testkit::fly_out(&mut session);
        crate::testkit::begin_jump_now(&mut session).expect("jumps");
        session.dock = request(140, CLEARED);
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.dock_request(), None);
        assert_eq!(jump(&mut session, &catalog, 132), Some(SystemId(132)));
    }

    #[test]
    fn the_take_off_drops_the_request() {
        let mut session = session();
        land_now(&mut session).expect("lands");
        session.dock = request(128, CLEARED);
        session.take_off(&crate::session::entry::NoTraffic, &mut NeverFires);
        assert_eq!(session.dock_request(), None);
        assert_eq!(session.land(), requested(128));
    }

    #[test]
    fn a_landing_drops_the_request() {
        let mut session = greeted(session());
        assert_eq!(session.land(), landed(128));
        assert_eq!(session.dock_request(), None);
    }

    /// [`catalog`] whose only stellar is `site`, under the ship.
    fn only(site: crate::catalog::LandingSite) -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.sites[0].1 = vec![site];
        catalog
    }

    #[test]
    fn a_gate_reached_drops_the_request() {
        let catalog = only(crate::catalog::LandingSite {
            flags2: HYPERGATE,
            ..planet(300, 30.0, -40.0)
        });
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        assert_eq!(session.land(), requested(300));
        tick(&mut session);
        assert_eq!(
            session.take_messages(),
            [cleared_at(300, DockKind::Hypergate)],
            "an inhabited hypergate greets"
        );
        assert_eq!(
            session.land(),
            Ok(LandPress::AtGate {
                stellar: StellarId(300),
                kind: GateKind::Hypergate,
            })
        );
        assert_eq!(session.dock_request(), None);
    }

    // Uninhabited stellars.

    fn uninhabited(id: i16, x: f32, y: f32, flags2: u16) -> crate::catalog::LandingSite {
        crate::catalog::LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            min_status: 32767,
            flags2,
            ..planet(id, x, y)
        }
    }

    #[test]
    fn an_uninhabited_stellar_answers_every_press_no_response_and_the_too_far_beep() {
        let catalog = only(uninhabited(128, 2000.0, 0.0, 0));
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        for _ in 0..3 {
            assert_eq!(
                session.land(),
                Err(LandingRefusal::TooFar {
                    stellar: StellarId(128),
                    station: false,
                })
            );
            assert_eq!(session.take_messages(), [SimMessage::NoResponse]);
            assert_eq!(
                session.take_sounds(),
                [SimSound::Refused],
                "no traffic control, the too-far beep (@0x676aa)"
            );
            assert_eq!(session.dock_request(), request(128, CLEARED));
            tick(&mut session);
            assert_eq!(session.take_messages(), [], "no greeting");
        }
    }

    #[test]
    fn parked_over_an_uninhabited_stellar_one_press_lands() {
        let catalog = only(uninhabited(128, 30.0, -40.0, 0));
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        assert_eq!(session.land(), landed(128));
        assert_eq!(session.take_messages(), [SimMessage::NoResponse]);
    }

    #[test]
    fn parked_over_an_uninhabited_wormhole_one_press_enters_it() {
        let catalog = only(uninhabited(400, 30.0, -40.0, WORMHOLE));
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        assert_eq!(
            session.land(),
            Ok(LandPress::AtGate {
                stellar: StellarId(400),
                kind: GateKind::Wormhole,
            })
        );
        assert_eq!(session.dock_request(), None);
    }

    // The granted request.

    /// [`catalog`] whose planet 129 needs a record no pilot has, of
    /// government 140 requiring `require`.
    fn guarded_129(require: u64) -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.govts = vec![crate::catalog::GovtRecord {
            require,
            ..crate::testkit::govt(140)
        }];
        let site = &mut catalog.sites[0].1[1];
        site.min_status = 32767;
        site.govt = Some(GovtId(140));
        catalog
    }

    /// A session of `catalog` with its request at 128 counted to
    /// `permission`, then the ship moved over 129 and the selection let
    /// go, as a script's `N` lets go of it, then L pressed.
    fn carried_over(
        catalog: &FakePilotCatalog,
        permission: i16,
    ) -> Result<LandPress, LandingRefusal> {
        let mut session = quiet(Session::start(catalog).expect("starts"));
        session.land().expect("requests 128");
        session.dock = request(128, permission);
        session.player.position = Vec2::new(2000.0, 0.0);
        session.deselect();
        session.land()
    }

    #[test]
    fn a_counter_at_0x2ee_carried_to_another_stellar_clears_its_request_require_permitting() {
        assert_eq!(carried_over(&guarded_129(0), CLEARED), requested(129));
        let denied = Err(LandingRefusal::Denied {
            stellar: StellarId(129),
            station: false,
            min_status: 32767,
        });
        assert_eq!(carried_over(&guarded_129(0), REQUESTED_NEAR), denied);
        assert_eq!(carried_over(&guarded_129(0x4), CLEARED), denied);
    }
}
