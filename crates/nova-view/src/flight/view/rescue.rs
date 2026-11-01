//! [`FlightView`]'s escape pod rescue on screen, over the [`FlightSim`]
//! port: the [`RescueScreen`] (see its module for the stages and the
//! original's ranges) run in place of flight's steps.
//!
//! - **Start.** The frame's steps that end in the rescue
//!   ([`StepRun::rescued`](nova_sim::flight_frame::StepRun::rescued)) are
//!   run as any others, their lines said, and the screen begins over the
//!   frame the pod showed last, drawn as their tick began
//!   ([`FlightView::pod_frame`]): its scene and HUD, the cursor sprite
//!   included, with what the repaint under the texts reads of the pod.
//! - **The message line.** The repaint under a text an `OnAbort` queued
//!   draws the message the slot holds as the rescue begins, as
//!   `_RefreshSAT`'s `_RedrawComm` (@0x6c24, @0x6db6) redraws it; then
//!   the slot is emptied, as `_ResetPlayer` sets `_commCount` to 0
//!   (@0x1d904), so no later repaint, and no frame after the rescue,
//!   shows it. Nothing in the rescue after the reset says a line.
//! - **The repaint under the first text** reads the state where the
//!   rescue was as that text was queued
//!   ([`StepRun::rescue_texts`](nova_sim::flight_frame::StepRun::rescue_texts),
//!   [`FirstText`]): before `_ResetPlayer`, the pod's system, bar and
//!   message line as the pod last drew them; after it, by the
//!   `OnPurchase`, the pod's system and radar (its ships not yet
//!   deleted, @0x68f69) with the reset ship's bar; after the move, by the
//!   days, the new system and its radar with that bar. A text queued
//!   after the reset shows the gauges and nav it noted as it was queued
//!   ([`TextBar`]); the rest of the bar, the target panel, the cargo and
//!   the secondary weapon, is the rescued frame's, which nothing after the
//!   reset changes. The repaint is taken once the screen is black, the new
//!   system laid out.
//! - **The floaters.** Each repaint draws the escort menu while it
//!   shows, at the level it stood at, as `_FloaterDrawCallback` blits it
//!   (@0x38c0f-0x38c5b): its fade counts flight steps, and none runs.
//!   The floating map is drawn in none: its level counts `TickCount`
//!   (@0x38ba0-0x38bce) and is 0 from 150 ticks after its last show, and
//!   the rescue's three synchronous fades alone take 180.
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

use nova_sim::{ClassRow, NavMode, TextBar};

use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::hud;
use crate::flight::radar::RadarScene;
use crate::flight::rescue_screen::{FirstText, Repaint, RescueScreen, RescueStage};
use crate::flight::sim_port::FlightSim;
use crate::input::Input;
use crate::system::catalog::SystemCatalog;
use crate::{Color, DrawList};

use super::cockpit::BarParts;
use super::world::message_background;
use super::{FlightView, radar_ships};

/// What the rescue keeps of the frame the pod showed last
/// ([`FlightView::pod_frame`]).
pub(super) struct PodFrame {
    /// The whole frame, its scene and HUD, which the rescue fades out
    /// over.
    scene: DrawList,
    /// The pod's system's `BkgndColor`, as the repaints paint it.
    background: Color,
    /// The status bar as the pod last drew it.
    bar: DrawList,
    /// The radar of the pod's system about the pod, with no target, as
    /// `_ResetPlayer` leaves it (the targets go) before the pod is moved.
    radar: RadarScene,
    /// The escort menu's class rows.
    rows: [ClassRow; 4],
}

/// The repaint under a first text queued after `_ResetPlayer`, taken once
/// the screen is black (see the module docs).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct LaterRepaint {
    /// Whether the text was queued after the move, by the days.
    moved: bool,
    /// The bar the text noted as it was queued.
    bar: TextBar,
    /// The pod's system's `BkgndColor`.
    background: Color,
    /// The pod's radar with no target ([`PodFrame::radar`]).
    radar: RadarScene,
    /// The escort menu's class rows in the pod's system.
    rows: [ClassRow; 4],
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// The frame the pod shows, drawn as a tick begins while the player
    /// is in the escape pod, before anything of the tick moves, for the
    /// rescue to fade out over should the tick's steps end in it, and
    /// what the repaints under its texts read of the pod. None out of the
    /// pod.
    pub(super) fn pod_frame(&self) -> Option<PodFrame> {
        if !matches!(self.frame.fate, PlayerFate::Ejected { .. }) {
            return None;
        }
        let mut scene = DrawList::new();
        if self.draw_flight(&mut scene, || self.failure.clone().unwrap_or_default()) {
            self.draw_fades(&mut scene);
        }
        Some(self.pod_frame_drawing(scene))
    }

    /// [`PodFrame`] of the frame drawn, over `scene`.
    fn pod_frame_drawing(&self, scene: DrawList) -> PodFrame {
        let mut bar = DrawList::new();
        self.draw_bar_or_why(&mut bar);
        let frame = self.drawn();
        PodFrame {
            scene,
            background: message_background(frame.system_background),
            bar,
            radar: RadarScene {
                ships: radar_ships(None, &self.shown_npcs()),
                ..self.radar_now(frame)
            },
            rows: frame.hud.escort_rows,
        }
    }

    /// The rescue's closing repaint of the frame drawn: its system's
    /// `BkgndColor`, its floaters and the bar.
    fn repaint(&self) -> Repaint {
        let mut bar = DrawList::new();
        self.draw_bar_or_why(&mut bar);
        let frame = self.drawn();
        let background = message_background(frame.system_background);
        Repaint {
            background,
            floaters: self.repaint_floaters(background, &frame.hud.escort_rows),
            bar,
        }
    }

    /// The comm line and the floaters a repaint draws over a space area
    /// in `background` (see the module docs): the message while one
    /// shows, its box erased in `background` and its background OR-ed in
    /// as flight draws them ([`FlightView::draw_floaters`]), then the
    /// escort menu with `rows`. Not world: the repaint's space area is
    /// no sprite world.
    fn repaint_floaters(&self, background: Color, rows: &[ClassRow; 4]) -> DrawList {
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        let mut list = DrawList::new();
        if let Some(message) = self.message() {
            hud::draw_message_box(&mut list, self.view, background);
            hud::or_message_background(&mut list, self.view, background);
            hud::draw_message_text(&mut list, message, self.view, metrics);
        }
        self.escort_menu
            .draw(&mut list, rows, self.escort_colors, metrics);
        list
    }

    /// The frame's steps ended in the pod's rescue, their lines said: its
    /// screen begins over `pod`, what [`FlightView::pod_frame`] drew
    /// before them, the repaint under its first text read as it was
    /// queued, and the message slot is emptied (see the module docs).
    pub(super) fn begin_rescue(&mut self, pod: Option<PodFrame>) {
        let pod = pod.unwrap_or_else(|| self.pod_frame_drawing(DrawList::new()));
        let first = self.drain().text_bars().first().copied();
        let marks = self
            .step_report
            .steps
            .iter()
            .find_map(|run| run.rescue_texts)
            .unwrap_or_default();
        let under = Repaint {
            background: pod.background,
            floaters: self.repaint_floaters(pod.background, &pod.rows),
            bar: pod.bar,
        };
        self.rescue_later = first.and_then(|bar| {
            let moved = match FirstText::of(Some(bar.texts), marks) {
                FirstText::BeforeReset => return None,
                FirstText::AfterReset => false,
                FirstText::AfterMove => true,
            };
            Some(LaterRepaint {
                moved,
                bar,
                background: pod.background,
                radar: pod.radar,
                rows: pod.rows,
            })
        });
        self.rescue = Some(RescueScreen::new(pod.scene, under));
        self.blank_message();
    }

    /// The repaint under a first text queued after `_ResetPlayer`, of the
    /// frame drawn and the new system laid out (see the module docs):
    /// after the move, the new system's; before it, the pod's system and
    /// radar, the rest of the bar the reset ship's.
    fn later_repaint(&self, later: &LaterRepaint) -> Repaint {
        let frame = self.drawn();
        let (background, rows, radar) = if later.moved {
            let background = message_background(frame.system_background);
            (background, frame.hud.escort_rows, self.radar_now(frame))
        } else {
            let gear = frame.hud.radar_gear;
            let radar = RadarScene {
                iff: gear.iff,
                density_scanner: gear.density_scanner,
                ..later.radar.clone()
            };
            (later.background, later.rows, radar)
        };
        let radar = RadarScene {
            hyperspace_mode: matches!(later.bar.nav, NavMode::Hyperspace(_)),
            ..radar
        };
        let mut bar = DrawList::new();
        match &self.status_bar {
            Ok(status) => {
                let target = self.target_now(frame);
                let parts = BarParts {
                    radar: &radar,
                    reserves: later.bar.reserves,
                    nav: self.nav_display(later.bar.nav),
                    target: target.as_ref(),
                };
                self.draw_bar_of(&mut bar, (status, self.view.0), frame, parts);
            }
            Err(reason) => hud::draw_unavailable(&mut bar, reason, self.view.0),
        }
        Repaint {
            background,
            floaters: self.repaint_floaters(background, &rows),
            bar,
        }
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
                    if let Some(later) = self.rescue_later.take() {
                        let under = self.later_repaint(&later);
                        if let Some(screen) = &mut self.rescue {
                            screen.repaint_under(under);
                        }
                    }
                }
                RescueStage::Closing => self.redraw_status(),
                RescueStage::Done => self.rescue = None,
                _ => {}
            }
        }
        true
    }
}
