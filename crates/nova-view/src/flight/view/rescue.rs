//! [`FlightView`]'s escape pod rescue on screen, over the [`FlightSim`]
//! port: the [`RescueScreen`] (see its module for the stages and the
//! original's ranges) run in place of flight's steps.
//!
//! - **Start.** The frame's steps that end in the rescue
//!   ([`StepRun::rescued`](nova_sim::flight_frame::StepRun::rescued)) are
//!   run as any others, and the screen begins over the frame the pod
//!   showed last, drawn as their tick began
//!   ([`FlightView::pod_frame`]): its scene and HUD, the cursor sprite
//!   included, with the bar as it stood and the pod's system's
//!   `BkgndColor` for the repaint under the texts.
//! - **The layout.** The new system is laid out where the rescue put the
//!   ship, the course map looking there too (`_HandlePlayer` @0x68fab),
//!   only once the fade out is over and the screen is black.
//! - **Stepping and drawing.** Flight steps no more until the screen is
//!   over: each tick moves the screen on and ends there, as
//!   `FlightView::death_frame` ends a dead flight's. The screen is drawn
//!   in place of the scene.
//! - **Held outputs.** The rescue's `dësc` is given once the black is
//!   over ([`FlightView::take_rescue_desc`]), and the texts it queued
//!   once the second fade is over ([`FlightView::take_mission_texts`]).
//! - **The faded flag.** The screen reads faded from the second fade
//!   until a text fades it in ([`FlightView::screen_faded`],
//!   [`FlightView::screen_faded_in`]), or the closing fade ends.
//! - **The pointer.** The system's pointer shows from the rescue's `dësc`
//!   on ([`Screen::hides_pointer`](crate::Screen::hides_pointer)).
//! - **Input.** Everything pressed under the screen is kept and given
//!   back for the router to hand to what shows next
//!   ([`FlightView::take_rescue_replay`]).
//! - **The closing.** As the closing repaint begins, the whole bar is
//!   redrawn from the rescued frame, as `_RedrawStatus(1)` (@0x69219)
//!   redraws it.
//!
//! The seconds with no step are the view's gap (`FlightView::since_step`),
//! so the first step after the rescue holds the gain, as the original's
//! frame after it sees a gap of 10.0 or more.

use std::time::Duration;

use nova_sim::PlayerFate;

use crate::DrawList;
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::rescue_screen::{Repaint, RescueScreen, RescueStage};
use crate::flight::sim_port::FlightSim;
use crate::input::Input;
use crate::system::catalog::SystemCatalog;

use super::FlightView;
use super::world::message_background;

impl<C, S: FlightSim> FlightView<C, S> {
    /// The frame the pod shows, drawn as a tick begins while the player
    /// is in the escape pod, before anything of the tick moves, for the
    /// rescue to fade out over should the tick's steps end in it, and the
    /// repaint under its texts: the bar as it stands and the pod's
    /// system's `BkgndColor`. None out of the pod.
    pub(super) fn pod_frame(&self) -> Option<(DrawList, Repaint)> {
        if !matches!(self.frame.fate, PlayerFate::Ejected { .. }) {
            return None;
        }
        let mut whole = DrawList::new();
        if self.draw_flight(&mut whole, || self.failure.clone().unwrap_or_default()) {
            self.draw_fades(&mut whole);
        }
        Some((whole, self.repaint()))
    }

    /// The rescue's repaint of the frame drawn: its system's `BkgndColor`
    /// and the bar.
    fn repaint(&self) -> Repaint {
        let mut bar = DrawList::new();
        self.draw_bar_or_why(&mut bar);
        Repaint {
            background: message_background(self.drawn().system_background),
            bar,
        }
    }

    /// The frame's steps ended in the pod's rescue: its screen begins
    /// over `pod`, what [`FlightView::pod_frame`] drew before them.
    pub(super) fn begin_rescue(&mut self, pod: Option<(DrawList, Repaint)>) {
        let (pod, under) = pod.unwrap_or_else(|| (DrawList::new(), self.repaint()));
        self.rescue = Some(RescueScreen::new(pod, under));
    }

    /// Draws the rescue's screen while it shows, the closing repainting
    /// the rescued frame: whether it showed.
    pub(super) fn draw_rescue(&self, list: &mut DrawList) -> bool {
        let Some(screen) = &self.rescue else {
            return false;
        };
        screen.draw(list, (self.view, self.play_rect()), || self.repaint());
        true
    }

    /// Keeps `input` while the rescue shows: whether it did.
    pub(super) fn hold_under_rescue(&mut self, input: &Input) -> bool {
        let Some(screen) = &mut self.rescue else {
            return false;
        };
        screen.hold(input);
        true
    }

    /// What was pressed under the rescue, given back as its `dësc`, its
    /// texts or flight come next, taken once: the router hands it to what
    /// it shows then, as the original's queued events reach the next
    /// event loop.
    pub fn take_rescue_replay(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.rescue_replay)
    }

    /// Whether the rescue shows the system's pointer.
    pub(super) fn rescue_shows_pointer(&self) -> bool {
        self.rescue
            .as_ref()
            .is_some_and(RescueScreen::shows_pointer)
    }

    /// Whether the rescue holds back its `dësc`.
    pub(super) fn rescue_holds_desc(&self) -> bool {
        self.rescue
            .as_ref()
            .is_some_and(|screen| !screen.desc_due())
    }

    /// Whether the rescue holds back the texts it queued.
    pub(super) fn rescue_holds_texts(&self) -> bool {
        self.rescue
            .as_ref()
            .is_some_and(|screen| !screen.texts_due())
    }

    /// Whether the rescue leaves the screen faded.
    pub(super) fn rescue_faded(&self) -> bool {
        self.rescue.as_ref().is_some_and(RescueScreen::faded)
    }

    /// A text has faded the rescue's screen in.
    pub(super) fn rescue_faded_in(&mut self) {
        if let Some(screen) = &mut self.rescue {
            screen.faded_in();
        }
    }
}

impl<C: SystemCatalog + ShipSprites + CombatLooks + StatusBars, S: FlightSim> FlightView<C, S> {
    /// A tick's rescue, as `Screen::tick` runs it before anything else of
    /// flight: the screen moves on by `dt`, the view laid out as its
    /// black begins, the bar redrawn as its closing does, and dropped once
    /// it is over. Gives whether the rescue showed, so the tick ends
    /// there.
    pub(super) fn rescue_frame(&mut self, dt: Duration) -> bool {
        let Some(screen) = &mut self.rescue else {
            return false;
        };
        let entered = screen.tick(dt);
        self.rescue_replay.extend(screen.take_replay());
        for stage in entered {
            match stage {
                RescueStage::Black => {
                    // Laid out in the system the rescue took the pod to,
                    // from where it put the ship; the course map looking
                    // there too (`_HandlePlayer` @0x68fab).
                    self.catch_up();
                    self.map.recentre();
                }
                RescueStage::Closing => self.redraw_status(),
                RescueStage::Done => self.rescue = None,
                _ => {}
            }
        }
        true
    }
}
