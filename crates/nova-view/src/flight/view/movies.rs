//! [`FlightView`]'s `dësc` movies: a stellar's movie played on landing
//! and on entering a hypergate or wormhole, as `_PlayQTMovieFromDesc`
//! (@0x55f11-0x55f5c) plays it for `_PlayerLandOnStellar` (@0x632a7),
//! `_PlayerEnterHypergate` (@0x639d8) and `_PlayerEnterWormhole`
//! (@0x6427d).
//!
//! The wrapper loads the `dësc` (`_LoadDescription` @0x55f2b), discards
//! its text, and returns when it names no movie (@0x55f39); otherwise it
//! plays it (`_PlayQTMovie` @0x55f56, [`MovieStage`]). Each caller calls
//! it only with the Movies preference on (@0x6328a, @0x639bb, @0x64260),
//! so with movies off nothing is loaded. The call is synchronous and the
//! movie modal: flight is frozen while it plays, so here the stage takes
//! every input and tick and the flight is neither stepped nor sent
//! anything until it ends ([`FlightView::playing`]). What the caller does
//! next ([`AfterMovie`]) runs once it has ended.
//!
//! - **Landing.** The flight has docked as the press answered: its refuel,
//!   its landing beep and its story events are done. The original plays
//!   the movie before `_DoAutoRefuel` (@0x632c5), the beep (@0x632ef) and
//!   the bar's redraw after them (@0x632fb). So the flight's sounds the
//!   original plays after the movie, which the flight's crate marks
//!   ([`SimSound::follows_landing_movie`]: the beep; the refuel is silent),
//!   are withheld while it plays, the status bar stays as the press found
//!   it, and the landing reaches the router ([`FlightView::take_landing`])
//!   only once the movie ends, when those sounds are released and the bar
//!   redrawn. The press's other sounds come before the movie in the
//!   original, as `_HandlePlayerDockRequest` makes them (the voice kill,
//!   `_KillVoiceSound` @0x66daf, and the thrust's stop among them), so
//!   they come out on the press's frame. Nothing else the docking changed
//!   is seen before the movie ends, so no boundary the player sees is
//!   crossed early. The land checks' texts wait too: the original shows
//!   none between the movie and the port's own loop but those
//!   `_DoPortDialog` (@0x5f911) shows after it has drawn the spaceport
//!   (`_ShowWindowNoFlash` @0x5faff, then `_MissionLandCheck` @0x5fbeb),
//!   so they come out ([`FlightView::take_mission_texts`]) once the movie
//!   ends, for the router to show over the spaceport it opens.
//!   Under the movie is what `_PlayerLandOnStellar` leaves on screen
//!   (@0x63221-0x63280): every sprite hidden, the stars among them, so the
//!   play area is bare, and the status bar. This movie's load is the
//!   original's @0x632a7 one, sent with the press. The port's own `dësc`
//!   (`_DoPortDialog` @0x5f96b) and the land checks (@0x5fbeb) follow it
//!   in the original's order: the flight loads the port's as the landing
//!   settles, before the land checks, and the spaceport opening after
//!   the movie is served that text, never loading it a second time
//!   ([`FlightView::load_port_description`]).
//! - **Gates.** The movie plays past the entry's refusals and before the
//!   come-out, so the view asks which `dësc` the entry loads
//!   ([`ScreenAsk::GateEntryDesc`]), `None` for an entry that would be
//!   refused, and holds the entry command behind the movie. Under it is
//!   the old system's flight, frozen. The load goes before the entry, so
//!   the rank governments it resets and the last movie it leaves come
//!   before the come-out's texts, as in the original.
//!
//! Neither function plays a sound.

#[cfg(doc)]
use nova_sim::SimSound;
use nova_sim::flight_frame::{Answer, FlightCommand, GateEntry, ScreenAnswer, ScreenAsk};
use nova_sim::{StellarId, SystemId};

use crate::draw::fill_rect;
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::jump::JumpEffect;
use crate::flight::sim_port::FlightSim;
use crate::movie::{MovieStage, Movies};
use crate::system::catalog::SystemCatalog;
use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};
use crate::view_shape::classic_offset;
use crate::{Color, DrawList, Input, Point};

use super::FlightView;

/// What runs once a flight's movie has ended: the rest of the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AfterMovie {
    /// The landing on the stellar reaches the router, the press's sounds
    /// are released and the status bar redrawn.
    Land(StellarId),
    /// The hypergate is entered for the system picked
    /// ([`FlightCommand::EnterHypergate`]).
    EnterHypergate(Option<SystemId>),
    /// The wormhole is entered ([`FlightCommand::EnterWormhole`]).
    EnterWormhole,
}

/// A movie playing in front of a frozen flight, and what follows it.
#[derive(Clone, Debug)]
pub(super) struct FlightMovie {
    stage: MovieStage,
    then: AfterMovie,
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// The flight playing its stellars' `dësc` movies through `movies`,
    /// while they are on (see the module docs). None by default.
    #[must_use]
    pub fn with_movies(self, movies: Movies) -> Self {
        Self { movies, ..self }
    }

    /// Whether a movie plays in front of flight: while it does, flight is
    /// frozen and the movie takes every input.
    #[must_use]
    pub fn playing(&self) -> bool {
        self.movie.is_some()
    }

    /// Whether the movie in front hides the system's pointer: a cinematic
    /// one, as `_PlayQTMovie` hides the cursor (@0x55a73).
    #[must_use]
    pub fn movie_hides_pointer(&self) -> bool {
        self.movie
            .as_ref()
            .is_some_and(|movie| movie.stage.hides_cursor())
    }

    /// What was pressed under the last movie, taken once, for the router
    /// to hand to what comes next, the spaceport after a landing and
    /// flight after a gate, as the original's queued events reach the next
    /// event loop.
    pub fn take_movie_replay(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.movie_replay)
    }

    /// Whether the landing's movie plays, holding back what follows it
    /// (see the module docs): the press's sounds that follow it and the
    /// land checks' texts.
    pub(super) fn holds_landing(&self) -> bool {
        matches!(
            self.movie,
            Some(FlightMovie {
                then: AfterMovie::Land(_),
                ..
            })
        )
    }

    /// Draws what is shown while a movie plays: under the landing's,
    /// the bare play area and the status bar as the press found it; under
    /// a gate's, the flight frozen; then the movie over it, in its
    /// 1024 x 768 frame centred in a widened view, a cinematic one's
    /// window and fades covering the whole of the [`Movies`]' shared view
    /// ([`Movies::view`]). In the app the router writes that view, not
    /// [`FlightView::set_view_size`]: a held flight resized while the
    /// spaceport shows must not widen the spaceport's movies. The frame's
    /// offset here comes from this flight's own view, the cover's from the
    /// shared one, so the two must agree whenever flight shows a movie:
    /// the router keeps the shared view this flight's size while flight
    /// shows (`nova`'s `AppScreen::movie_view`), and a caller that sets one
    /// sets the other. Whether a movie plays.
    pub(super) fn draw_movie(&self, list: &mut DrawList, frozen: impl FnOnce(&mut DrawList)) -> bool
    where
        C: StatusBars,
    {
        let Some(movie) = &self.movie else {
            return false;
        };
        if let AfterMovie::Land(_) = movie.then {
            fill_rect(list, self.play_rect(), Color::BLACK);
            if let Ok(bar) = &self.status_bar {
                self.draw_bar(list, (bar, self.view.0), &self.frame);
            }
        } else {
            frozen(list);
        }
        let mut shown = DrawList::new();
        movie.stage.draw(&mut shown);
        list.extend_translated(&shown, Point::new(classic_offset(self.view.0), 0.0));
        true
    }
}

impl<
    C: SystemCatalog
        + ShipSprites
        + CombatLooks
        + StatusBars
        + DescriptionSource
        + DescriptionGraphics,
    S: FlightSim,
> FlightView<C, S>
{
    /// Loads `dësc` `id` as `_PlayQTMovieFromDesc` does
    /// ([`FlightView::load_description`], its text discarded) and plays
    /// its movie in front of flight, `then` to follow it: true. False,
    /// having loaded it, when it can't be read, names no movie, or its
    /// movie can't start; then the caller goes on at once.
    fn play_desc_movie(&mut self, id: i16, then: AfterMovie) -> bool {
        if self.load_description(id).is_err() {
            return false;
        }
        let Some(mut stage) = self
            .catalog
            .description_movie(id)
            .and_then(|movie| self.movies.stage(&movie))
        else {
            return false;
        };
        stage.begin();
        if stage.ended() {
            return false;
        }
        self.movie = Some(FlightMovie { stage, then });
        true
    }

    /// The landing on `stellar` the press answered: with movies on, its
    /// `dësc` is loaded and its movie played first (see the module docs);
    /// otherwise, or with none to play, it lands at once.
    pub(super) fn land_with_movie(&mut self, stellar: StellarId) {
        if self.movies.on() && self.play_desc_movie(stellar.0, AfterMovie::Land(stellar)) {
            return;
        }
        self.landed_on(stellar);
    }

    /// Enters the gate `entry`, as `then` sends it: with movies on, the
    /// `dësc` the flight says the entry loads
    /// ([`ScreenAsk::GateEntryDesc`]) is loaded and its movie played
    /// first; with movies off, an entry that would be refused, or no movie
    /// to play, at once.
    pub(super) fn enter_with_movie(&mut self, entry: GateEntry) {
        let then = match entry {
            GateEntry::Hypergate(choice) => AfterMovie::EnterHypergate(choice),
            GateEntry::Wormhole => AfterMovie::EnterWormhole,
        };
        if self.movies.on()
            && let ScreenAnswer::GateEntryDesc(Some(id)) =
                self.sim.screen(ScreenAsk::GateEntryDesc(entry))
            && self.play_desc_movie(id, then)
        {
            return;
        }
        self.after_movie(then);
    }

    /// Runs `then`, the rest of the movie's caller.
    fn after_movie(&mut self, then: AfterMovie) {
        match then {
            AfterMovie::Land(stellar) => self.landed_on(stellar),
            AfterMovie::EnterHypergate(choice) => {
                match self.send(FlightCommand::EnterHypergate(choice)) {
                    Answer::Arrived { combat, .. } => {
                        let effect = JumpEffect::emerging(self.hyperspace_effects);
                        self.came_through(effect, &combat);
                    }
                    Answer::GateRefused(refusal) => self.gate_refused(refusal),
                    _ => {}
                }
            }
            AfterMovie::EnterWormhole => match self.send_gated(FlightCommand::EnterWormhole) {
                Answer::Arrived { combat, .. } => {
                    self.came_through(JumpEffect::flash(), &combat);
                }
                Answer::GateRefused(refusal) => self.gate_refused(refusal),
                _ => {}
            },
        }
    }

    /// The movie in front takes `input`; once it has ended, what follows
    /// it runs (`FlightView::end_movie`).
    pub(super) fn movie_input(&mut self, input: &Input) {
        if let Some(movie) = &mut self.movie {
            movie.stage.input(input);
        }
        self.end_movie();
    }

    /// The movie in front moves on by `dt`; once it has ended, what
    /// follows it runs (`FlightView::end_movie`).
    pub(super) fn movie_tick(&mut self, dt: std::time::Duration) {
        if let Some(movie) = &mut self.movie {
            movie.stage.tick(dt);
        }
        self.end_movie();
    }

    /// Once the movie in front has ended, lets it go and runs what follows
    /// it, keeping what was pressed under it for what comes next
    /// ([`FlightView::take_movie_replay`]).
    fn end_movie(&mut self) {
        let Some(mut movie) = self.movie.take_if(|movie| movie.stage.ended()) else {
            return;
        };
        self.movie_replay = movie.stage.take_replay();
        self.after_movie(movie.then);
    }
}
