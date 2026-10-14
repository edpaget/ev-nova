//! The player's nav mode and its selection ([`NavMode`]), the original's
//! player +0x6a (the mode) and +0x6c (its selection), and every setter
//! of them the engine models. All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! **Readers.** The nav area draws by the mode (`_DrawStatusNav` −1 / 2 /
//! 3 @0x4a044, @0x4a09e, @0x4a198), and the radar hides its centre arrow
//! in mode 3, whatever the selection (@0x49cf5). J jumps only in mode 3
//! to the system selected, when the ship can jump there
//! (@0x6b11e-0x6b131): [`Session::begin_jump`] and
//! [`Session::jump_readiness`] read the selection, resolved by place, not
//! the course. The stellar selection ([`Session::nav_target`]) is read in
//! mode 2 only: by the nav brackets (`_HandleNavTargetSprites`
//! @0x2fcbf-0x2fccf), the land key (@0x6a8ed, @0x6a945), the person
//! offers and hailing (`_HandlePlayerCommunication` @0x61fac-0x61fbf).
//! A hyperspace selection is resolved by place whenever it is read, as
//! the original resolves its link slot through
//! `_FindActiveCoLocatedSystem` (@0x4be0); one with no visible system at
//! its place reads as none, so a hidden system is never named.
//!
//! **Setters** (mode / selection):
//!
//! - Flight start, a new or a loaded pilot (`_ResetPlayer` @0x1d4cd-0x1d4d5):
//!   off. The mode is not saved.
//! - Nav Off, backquote ([`Session::nav_off`], @0x69a94-0x69b63): off,
//!   the course kept, with [`SimSound::NavSelected`] on every press
//!   (@0x69b19).
//! - Option-backquote ([`Session::clear_target`], @0x69a17-0x69a90): the
//!   ship target let go, with [`SimSound::TargetCleared`] when there was
//!   one (@0x69a7d). The nav mode is untouched.
//! - Hyperspace Mode, H ([`Session::hyperspace_mode`], @0x69b76-0x69b89):
//!   the beep, then, unless the mode is 3 already, mode 3 with nothing
//!   selected and the course cued.
//! - The cue ([`Session::cue_course`], `_CueNextHyperRouteDest`
//!   @0xe2dc-0xe35f): when the course's first hop is a system the ship
//!   can jump to from here, mode 3 with it selected (@0xe33c-0xe34d);
//!   otherwise nothing changes. It makes no sound.
//! - Hyper Select, `\` ([`Session::select_next_system`],
//!   @0x69bb2-0x69cec): with links, [`SimSound::HyperSelected`]
//!   (@0x69c89), and mode 3 with the next link selected, cycling from the
//!   one selected in mode 3 and from the first in any other (@0x69c98,
//!   @0x69ca5-0x69cce). With none, the selection let go, the mode kept,
//!   in silence (@0x69cd4).
//! - The stellar keys 1-4, F1-F4 and F5 ([`Session::select_stellar`],
//!   @0x69e1f-0x69f17): the beep whenever they find a stellar, even the
//!   one selected (@0x69ebc); then, unless it is selected in mode 2
//!   already (@0x69ec4-0x69ed6), mode 2 with it, its brackets flying in.
//! - A click on a stellar (@0x6a68f-0x6a6fc): nothing when it is selected
//!   in mode 2 already; otherwise the beep (@0x6a6c1), mode 2 with it and
//!   the fly-in.
//! - The land key ([`Session::land`], @0x6a8ed-0x6a932): with no stellar
//!   selected in mode 2, mode 2 with the nearest, in silence.
//! - Landing writes neither: the ship lands in mode 2, the land key
//!   having selected first.
//! - The take-off's tail ([`Session::take_off`], `_HandlePlayerDockRequest`
//!   @0x674d2-0x674d8): the selection let go, the mode kept, then the
//!   cue.
//! - A hyperspace arrival (@0x6be25-0x6be2b, @0x6c4cd): off, then, after
//!   the course's step, the cue, so mode 3 comes back while hops remain
//!   (the `entry` module's table).
//! - A gate's exit, which joins the take-off's tail (@0x66edb-0x66f40):
//!   the selection let go, the mode kept; the course is cleared, so the
//!   cue finds nothing. A wormhole writes nothing more (`_PlayerEnterWormhole`
//!   @0x64005), and its refusal lets go of the selection too.
//! - A hypergate (`_PlayerEnterHypergate`): off on every outcome
//!   (@0x637e4-0x637f2, @0x63fa2-0x63faa), and when it has no links.
//! - `M` or `N` in flight (`_EvalSetExp` @0x157e3, @0x158fd): the
//!   selection let go, the mode kept; landed, neither is written here
//!   (see below). The developer tools' relocation, which has no
//!   original, does the same as in flight.
//! - A map click ([`Session::plot_course`], `_DoSystemMap`
//!   @0x136d7-0x137c7): a route cues its first hop, the original's click on
//!   a system the current one lists (@0x137b3-0x137bb), or its cue as the
//!   map closes for a longer route (@0x13c16); no route lets go of the
//!   selection and keeps the mode (@0x13788-0x13790).
//!
//! **Left out.** The floating map that shows while H or `\` is held, its
//! clicks (@0x6a1d5-0x6a1e4, @0x6a719-0x6a762) and its preselect
//! (`_ShowFloatingMap` @0x11488); the map dialog's Tab / `\` cycling
//! (`_MapFilter` @0x13f0f-0x13fac); the map drawing the selected link, its
//! info panel and its preselect (@0xea79, @0xebdf, @0x10b70, @0x124ac),
//! and with it Hyper Select keeping the course and moving the selection
//! alone (@0x69c8e-0x69cd2), where ours leaves a one-jump course to it;
//! and the destroyed player's and the ship swap's per-frame clears
//! (@0x68712-0x68718, @0x6f41b-0x6f421). The landed `M` / `N`'s +0x6c, the
//! stellar docked at (@0x156f2-0x15709), shows nowhere while docked and
//! the take-off lets go of it. Mode 4, the hypergate map's, is never
//! drawn. `_dockTarget` / `_dockPermission`, which Nav Off and H reset,
//! have no counterpart: the landing clearance keeps no state here.

use super::Session;
use crate::StellarPick;
use crate::catalog::{StellarId, SystemId};
use crate::hyperspace::next_hyper_destination;
use crate::landing::nearest_landable;
use crate::navigation::{NavMode, stellar_under};
use crate::sound::SimSound;

impl Session {
    /// The player's nav mode and its selection, a hyperspace selection
    /// resolved by place: the visible system at its place, or none when
    /// there is none (see the module docs).
    #[must_use]
    pub fn nav_mode(&self) -> NavMode {
        match self.nav {
            NavMode::Hyperspace(selected) => {
                NavMode::Hyperspace(selected.and_then(|system| self.galaxy.same_place(system)))
            }
            mode => mode,
        }
    }

    /// The stellar selected in mode 2, if any: the one every reader of the
    /// stellar selection reads.
    #[must_use]
    pub fn nav_target(&self) -> Option<StellarId> {
        self.nav.stellar()
    }

    /// The system J jumps to: the one selected in mode 3, resolved by
    /// place, when the ship can jump there from here (@0x6b11e-0x6b131).
    pub(super) fn jump_selection(&self) -> Option<SystemId> {
        let NavMode::Hyperspace(Some(selected)) = self.nav_mode() else {
            return None;
        };
        let jumps = self.galaxy.map().jumps(self.pilot.system, self.hyperlinks);
        jumps.contains(&selected).then_some(selected)
    }

    /// Lets go of the selection and keeps the mode.
    pub(super) fn deselect(&mut self) {
        self.nav = self.nav.deselected();
    }

    /// Cues the course's next hop, as `_CueNextHyperRouteDest` (@0xe2dc)
    /// does: when the course's first hop as it is flown
    /// ([`Session::course_shown`]) is a system the ship can jump to from
    /// here, mode 3 with it selected; otherwise nothing changes. It makes
    /// no sound.
    pub fn cue_course(&mut self) {
        let Some(&hop) = self.course_shown().first() else {
            return;
        };
        if self
            .galaxy
            .map()
            .jumps(self.pilot.system, self.hyperlinks)
            .contains(&hop)
        {
            self.nav = NavMode::Hyperspace(Some(hop));
        }
    }

    /// Presses Nav Off (backquote): the nav system off, the course kept,
    /// with [`SimSound::NavSelected`] on every press. During a jump
    /// (pre-jump stage too) nothing changes and it says so.
    pub fn nav_off(&mut self) -> bool {
        if self.jump.is_some() {
            return false;
        }
        self.sounds.push(SimSound::NavSelected);
        self.nav = NavMode::Off;
        true
    }

    /// Presses Hyperspace Mode (H): [`SimSound::NavSelected`], then,
    /// unless the mode is 3 already, mode 3 with nothing selected and the
    /// course's next hop cued ([`Session::cue_course`]). During a jump
    /// (pre-jump stage too) nothing changes and it says so.
    pub fn hyperspace_mode(&mut self) -> bool {
        if self.jump.is_some() {
            return false;
        }
        self.sounds.push(SimSound::NavSelected);
        if !matches!(self.nav, NavMode::Hyperspace(_)) {
            self.nav = NavMode::Hyperspace(None);
            self.cue_course();
        }
        true
    }

    /// Presses Option-backquote: lets go of the ship target, with
    /// [`SimSound::TargetCleared`], and says whether there was one. With
    /// none it is silent. The nav mode is untouched, and the original
    /// takes it during a jump too.
    pub fn clear_target(&mut self) -> bool {
        let had = self.target.take().is_some();
        if had {
            self.sounds.push(SimSound::TargetCleared);
        }
        had
    }

    /// Makes the stellar `pick` finds the stellar selected, as the
    /// original's stellar slot keys, F5 and a click in space do, and
    /// gives it. The course is left alone. `None`, and nothing changes,
    /// during a jump (pre-jump stage too, as the original skips its nav
    /// inputs while jumping) or when `pick` finds nothing: an empty slot,
    /// no landable stellar, or a click on none.
    ///
    /// A key that finds a stellar raises [`SimSound::NavSelected`], even
    /// for the one selected. Selecting changes the mode to 2 and starts
    /// the brackets' fly-in ([`Session::nav_brackets`]) only when it is
    /// not the stellar selected in mode 2 already (@0x69ec4-0x69ed6). A
    /// click on that stellar does nothing at all, and makes no sound
    /// (@0x6a68f-0x6a69d).
    pub fn select_stellar(&mut self, pick: StellarPick) -> Option<StellarId> {
        if self.jump.is_some() {
            return None;
        }
        let picked = match pick {
            StellarPick::Slot(slot) => self.sites.get(slot).map(|site| site.id),
            StellarPick::Nearest => nearest_landable(self.player.position, &self.sites),
            StellarPick::At(at) => {
                stellar_under(&self.sites, at, |site| self.drawn_frame_size(site))
            }
        }?;
        let selected = self.nav == NavMode::Stellar(Some(picked));
        if selected && matches!(pick, StellarPick::At(_)) {
            return Some(picked);
        }
        self.sounds.push(SimSound::NavSelected);
        if !selected {
            self.nav_fly_in.start();
            self.nav = NavMode::Stellar(Some(picked));
        }
        Some(picked)
    }

    /// Presses Hyper Select (the original's `\` key): selects in mode 3
    /// the next system the session's [`HyperSelectRule`](crate::HyperSelectRule)
    /// cycles to ([`next_hyper_destination`]), from the one selected in
    /// mode 3 and from the start in any other, with
    /// [`SimSound::HyperSelected`], plots a one-jump course to it and
    /// gives it. With no system to offer it lets go of the selection,
    /// keeps the mode and makes no sound (@0x69cd4), and gives `None`.
    /// `None`, and nothing changes, during a jump (pre-jump stage too).
    ///
    /// The original keeps the plotted route and only moves its selection,
    /// so cycling back to the route's first hop there keeps the whole
    /// route; ours' map draws only the course, so a press here leaves a
    /// one-jump course.
    pub fn select_next_system(&mut self) -> Option<SystemId> {
        if self.jump.is_some() {
            return None;
        }
        let selected = match self.nav_mode() {
            NavMode::Hyperspace(selected) => selected,
            _ => None,
        };
        let Some(next) = next_hyper_destination(
            self.galaxy.map(),
            self.pilot.system,
            &self.course_shown(),
            selected,
            self.hyper_select,
            self.hyperlinks,
        ) else {
            self.deselect();
            return None;
        };
        self.sounds.push(SimSound::HyperSelected);
        self.pilot.course = vec![next];
        self.nav = NavMode::Hyperspace(Some(next));
        Some(next)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::flight::Controls;
    use crate::gate::GateRefusal;
    use crate::hyperspace::{HyperSelectRule, JumpReadiness, JumpRefusal, RouteError};
    use crate::landing::LandOutcome;
    use crate::pilot::Pilot;
    use crate::session::LandPress;
    use crate::session::tests::{
        alternates, armed, at_gate, at_wormhole, facing_an_npc, gated, hide, hub, three_stellars,
        wormholes,
    };
    use crate::sound::SimSound;
    use crate::targeting::TargetPick;
    use crate::testkit::{FakePilotCatalog, begin_jump_now, catalog, fly_out, land_now, starting};

    fn ids(route: &[i16]) -> Vec<SystemId> {
        route.iter().copied().map(SystemId).collect()
    }

    fn hyperspace(system: i16) -> NavMode {
        NavMode::Hyperspace(Some(SystemId(system)))
    }

    fn stellar(id: i16) -> NavMode {
        NavMode::Stellar(Some(StellarId(id)))
    }

    /// The session with its sounds taken.
    fn quiet(mut session: Session) -> Session {
        session.take_sounds();
        session
    }

    // Start and load.

    #[test]
    fn a_new_pilot_flies_with_the_nav_system_off() {
        let session = Session::start(&three_stellars()).expect("starts");
        assert_eq!(session.nav_mode(), NavMode::Off);
    }

    #[test]
    fn a_pilot_loaded_with_a_course_flies_with_the_nav_system_off_and_cannot_jump() {
        let catalog = catalog();
        let mut pilot = Pilot::new(&catalog, "").expect("a pilot");
        pilot.course = ids(&[131, 132]);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.nav_mode(), NavMode::Off);
        assert_eq!(session.course(), ids(&[131, 132]), "the course kept");
        fly_out(&mut session);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    }

    // Nav Off.

    #[test]
    fn nav_off_with_a_course_turns_the_nav_off_keeps_the_course_and_refuses_the_jump() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.nav_mode(), hyperspace(131));
        let mut session = quiet(session);
        assert!(session.nav_off());
        assert_eq!(session.nav_mode(), NavMode::Off);
        assert_eq!(session.course(), ids(&[131, 132]), "the course kept");
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        fly_out(&mut session);
        assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    }

    #[test]
    fn nav_off_beeps_on_every_press_and_makes_no_save_due() {
        let mut session = quiet(Session::start(&three_stellars()).expect("starts"));
        for _ in 0..2 {
            assert!(session.nav_off());
            assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        }
        assert!(!session.take_save_due());
    }

    #[test]
    fn nav_off_and_hyperspace_mode_are_refused_during_a_jump() {
        let mut session = Session::start(&hub()).expect("starts");
        session.select_next_system();
        fly_out(&mut session);
        session.player.heading = 90.0;
        assert_eq!(session.begin_jump(), Ok(SystemId(134)));
        let mut session = quiet(session);
        for stage in ["pre-jump", "in hyperspace"] {
            assert!(!session.nav_off(), "{stage}");
            assert!(!session.hyperspace_mode(), "{stage}");
            assert_eq!(session.nav_mode(), hyperspace(134), "{stage}");
            assert_eq!(session.take_sounds(), [], "{stage}");
            if session.jumping().is_none() {
                begin_jump_now(&mut session).expect("jumps");
                session.take_sounds();
            }
        }
    }

    // Hyperspace Mode.

    #[test]
    fn hyperspace_mode_with_no_course_selects_no_destination_and_the_jump_is_refused() {
        let mut session = quiet(Session::start(&three_stellars()).expect("starts"));
        session.select_stellar(StellarPick::Slot(0));
        session.take_sounds();
        assert!(session.hyperspace_mode());
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        fly_out(&mut session);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    }

    #[test]
    fn hyperspace_mode_cues_an_adjacent_first_hop_and_the_jump_goes_there() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        session.nav_off();
        assert!(session.hyperspace_mode());
        assert_eq!(session.nav_mode(), hyperspace(131));
        fly_out(&mut session);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
    }

    #[test]
    fn hyperspace_mode_in_mode_3_changes_nothing_but_beeps() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        session.nav = NavMode::Hyperspace(None);
        let mut session = quiet(session);
        assert!(session.hyperspace_mode());
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None), "not cued");
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
    }

    // Hyper Select.

    #[test]
    fn hyper_select_from_off_starts_at_the_first_link_and_cycles_in_mode_3() {
        let mut session = quiet(Session::start(&hub()).expect("starts"));
        for expected in [134, 131, 135, 134] {
            assert_eq!(session.select_next_system(), Some(SystemId(expected)));
            assert_eq!(session.nav_mode(), hyperspace(expected));
            assert_eq!(session.course(), ids(&[expected]));
            assert_eq!(session.take_sounds(), [SimSound::HyperSelected]);
        }
    }

    #[test]
    fn hyper_select_cycles_from_the_system_selected_not_the_course() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        session.nav = hyperspace(135);
        assert_eq!(session.select_next_system(), Some(SystemId(134)), "wraps");
    }

    #[test]
    fn hyper_select_from_a_stellar_starts_at_the_first_link() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(session.select_next_system(), Some(SystemId(134)));
        assert_eq!(session.nav_mode(), hyperspace(134));
    }

    /// [`hub`]'s session flown from 133, which lists no links.
    fn linkless() -> Session {
        let mut catalog = hub();
        catalog.character = starting([Some(133), None, None, None]).character;
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.system(), SystemId(133));
        quiet(session)
    }

    #[test]
    fn hyper_select_with_no_links_in_mode_3_selects_no_destination_in_silence() {
        let mut session = linkless();
        assert!(session.hyperspace_mode());
        session.nav = hyperspace(130);
        session.take_sounds();
        assert_eq!(session.select_next_system(), None);
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.course(), []);
    }

    #[test]
    fn hyper_select_with_no_links_lets_go_of_a_stellar_and_keeps_mode_2() {
        let mut session = linkless();
        session.nav = stellar(128);
        assert_eq!(session.select_next_system(), None);
        assert_eq!(session.nav_mode(), NavMode::Stellar(None));
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn hyper_select_by_the_one_jump_course_reading_beeps_and_selects_too() {
        let mut session = quiet(
            Session::start(&hub())
                .expect("starts")
                .with_hyper_select(HyperSelectRule::OneJumpCourse),
        );
        assert_eq!(session.select_next_system(), Some(SystemId(131)));
        assert_eq!(session.nav_mode(), hyperspace(131));
        assert_eq!(session.take_sounds(), [SimSound::HyperSelected]);
    }

    // The stellar keys, the click and the land key.

    #[test]
    fn a_stellar_key_beeps_whenever_it_finds_a_stellar_even_the_one_selected() {
        let mut session = quiet(Session::start(&three_stellars()).expect("starts"));
        for _ in 0..2 {
            assert_eq!(
                session.select_stellar(StellarPick::Slot(0)),
                Some(StellarId(131))
            );
            assert_eq!(session.nav_mode(), stellar(131));
            assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        }
        assert_eq!(
            session.select_stellar(StellarPick::Nearest),
            Some(StellarId(128))
        );
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        assert_eq!(session.select_stellar(StellarPick::Slot(3)), None, "empty");
        assert_eq!(session.take_sounds(), [], "an empty slot is silent");
        assert_eq!(session.nav_mode(), stellar(128));
    }

    #[test]
    fn a_click_on_the_stellar_selected_does_nothing_and_makes_no_sound() {
        let mut session = quiet(Session::start(&three_stellars()).expect("starts"));
        let on_131 = StellarPick::At(crate::geometry::Vec2::new(900.0, 0.0));
        assert_eq!(session.select_stellar(on_131), Some(StellarId(131)));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        session.tick(Controls::default());
        let expand = session.nav_brackets().expect("bracketed").expand;
        assert_eq!(session.select_stellar(on_131), Some(StellarId(131)));
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.nav_mode(), stellar(131));
        assert_eq!(
            session.nav_brackets().expect("bracketed").expand,
            expand,
            "no fly-in"
        );
    }

    #[test]
    fn a_click_on_a_stellar_from_mode_3_beeps_and_selects_it() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.hyperspace_mode();
        let mut session = quiet(session);
        let on_131 = StellarPick::At(crate::geometry::Vec2::new(900.0, 0.0));
        assert_eq!(session.select_stellar(on_131), Some(StellarId(131)));
        assert_eq!(session.nav_mode(), stellar(131));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
    }

    #[test]
    fn the_land_key_selects_the_nearest_in_silence_from_any_other_mode() {
        for before in [
            NavMode::Off,
            NavMode::Stellar(None),
            NavMode::Hyperspace(None),
            hyperspace(131),
        ] {
            let mut session = quiet(Session::start(&three_stellars()).expect("starts"));
            session.nav = before;
            assert!(matches!(
                session.land(),
                Ok(LandPress::Outcome(LandOutcome::Selected { .. }))
            ));
            assert_eq!(session.nav_mode(), stellar(128), "{before:?}");
            assert!(
                !session.take_sounds().contains(&SimSound::NavSelected),
                "{before:?}"
            );
        }
    }

    // The jump.

    #[test]
    fn the_jump_goes_to_the_system_selected_rather_than_the_courses_first_hop() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        session.nav = hyperspace(134);
        fly_out(&mut session);
        assert_eq!(session.jump_readiness(), JumpReadiness::Clear);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(134)));
    }

    #[test]
    fn a_selection_the_ship_cannot_jump_to_is_no_destination() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        for mode in [hyperspace(132), hyperspace(136), stellar(128), NavMode::Off] {
            session.nav = mode;
            fly_out(&mut session);
            assert_eq!(
                session.begin_jump(),
                Err(JumpRefusal::NoDestination),
                "{mode:?}"
            );
        }
    }

    // The arrival, the landing and the take-off.

    #[test]
    fn an_arrival_turns_the_nav_off_or_cues_the_next_hop_while_the_course_goes_on() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.course(), ids(&[132]));
        assert_eq!(session.nav_mode(), hyperspace(132), "cued");

        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.course(), []);
        assert_eq!(session.nav_mode(), NavMode::Off, "the course is done");
    }

    #[test]
    fn landing_keeps_the_stellar_selected_and_the_take_off_lets_go_of_it() {
        let catalog = three_stellars();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(session.nav_mode(), stellar(128), "landing writes neither");
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.nav_mode(), NavMode::Stellar(None));
        assert_eq!(session.nav_target(), None);
    }

    #[test]
    fn the_take_off_with_a_course_cues_its_first_hop() {
        let catalog = three_stellars();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.nav_mode(), hyperspace(131));
    }

    // Gates and set expressions.

    #[test]
    fn a_hypergate_turns_the_nav_off_on_exit_on_cancel_and_without_links() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Ok(SystemId(131))
        );
        assert_eq!(session.nav_mode(), NavMode::Off, "exit");

        let mut session = at_gate(&catalog);
        assert_eq!(
            session.enter_hypergate(None, &catalog, &mut NeverFires),
            Err(GateRefusal::Cancelled)
        );
        assert_eq!(session.nav_mode(), NavMode::Off, "cancel");

        let catalog = FakePilotCatalog {
            gates: vec![],
            ..gated()
        };
        let mut session = at_gate(&catalog);
        assert_eq!(session.open_hypergate(&catalog), Err(GateRefusal::NoLinks));
        assert_eq!(session.nav_mode(), NavMode::Off, "no links");
    }

    #[test]
    fn a_wormhole_lets_go_of_the_selection_and_keeps_the_mode() {
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog);
        assert_eq!(session.nav_mode(), stellar(400));
        session
            .enter_wormhole(&catalog, &mut crate::testkit::Scripted::rolling(&[0]))
            .expect("comes out");
        assert_eq!(session.nav_mode(), NavMode::Stellar(None));
    }

    /// Runs `M131` and settles it.
    fn move_to_131(session: &mut Session, catalog: &FakePilotCatalog) {
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    #[test]
    fn an_m_in_flight_lets_go_of_the_selection_and_keeps_the_mode() {
        for (before, after) in [
            (stellar(128), NavMode::Stellar(None)),
            (hyperspace(131), NavMode::Hyperspace(None)),
            (NavMode::Off, NavMode::Off),
        ] {
            let catalog = catalog();
            let mut session = Session::start(&catalog).expect("starts");
            session.plot_course(SystemId(132)).expect("a route");
            session.nav = before;
            move_to_131(&mut session, &catalog);
            assert_eq!(session.system(), SystemId(131));
            assert_eq!(session.nav_mode(), after, "{before:?}");
        }
    }

    #[test]
    fn an_m_landed_writes_neither() {
        let catalog = three_stellars();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        move_to_131(&mut session, &catalog);
        assert_eq!(session.pilot().system(), SystemId(131));
        assert_eq!(session.nav_mode(), stellar(128));
    }

    // The map.

    #[test]
    fn plotting_a_course_cues_its_first_hop_from_any_mode() {
        for before in [NavMode::Off, stellar(128), NavMode::Hyperspace(None)] {
            let mut session = Session::start(&three_stellars()).expect("starts");
            session.nav = before;
            session.plot_course(SystemId(132)).expect("a route");
            assert_eq!(session.nav_mode(), hyperspace(131), "{before:?}");
        }
    }

    #[test]
    fn a_failed_plot_lets_go_of_the_selection_and_keeps_the_mode() {
        for (before, after) in [
            (hyperspace(134), NavMode::Hyperspace(None)),
            (stellar(128), NavMode::Stellar(None)),
            (NavMode::Off, NavMode::Off),
        ] {
            let mut session = Session::start(&hub()).expect("starts");
            session.nav = before;
            assert_eq!(
                session.plot_course(SystemId(136)),
                Err(RouteError::Unreachable)
            );
            assert_eq!(session.nav_mode(), after, "{before:?}");
        }
    }

    #[test]
    fn cueing_selects_an_adjacent_first_hop_and_nothing_else() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.course = ids(&[131, 132]);
        session.cue_course();
        assert_eq!(session.nav_mode(), hyperspace(131));

        for before in [NavMode::Off, stellar(128), NavMode::Hyperspace(None)] {
            session.nav = before;
            session.pilot.course = ids(&[132]);
            session.cue_course();
            assert_eq!(session.nav_mode(), before, "132 is not linked from 130");
            session.pilot.course.clear();
            session.cue_course();
            assert_eq!(session.nav_mode(), before, "no course");
        }
        assert_eq!(session.take_sounds(), [], "silent");
    }

    #[test]
    fn the_selection_and_the_cue_resolve_a_hidden_system_by_place() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.nav_mode(), hyperspace(131));
        hide(&mut session, &catalog, &[131]);
        assert_eq!(
            session.nav_mode(),
            hyperspace(134),
            "134 stands at its place"
        );
        session.nav = NavMode::Off;
        session.cue_course();
        assert_eq!(session.nav_mode(), hyperspace(134));
        hide(&mut session, &catalog, &[134]);
        assert_eq!(
            session.nav_mode(),
            NavMode::Hyperspace(None),
            "nothing stands there: no hidden system named"
        );
    }

    // Option-backquote.

    #[test]
    fn clearing_the_target_beeps_only_when_there_was_one_and_leaves_the_nav_mode() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 0);
        session.select_stellar(StellarPick::Slot(0));
        assert!(session.select_target(TargetPick::Nearest).is_some());
        let mut session = quiet(session);
        assert!(session.clear_target());
        assert_eq!(session.target(), None);
        assert_eq!(session.take_sounds(), [SimSound::TargetCleared]);
        assert!(!session.clear_target(), "no target");
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.nav_mode(), stellar(128));
    }
}
