//! The in-flight maps' pad cursor (rdm:phase/key-bindings/phase-20-gamepad-map-cursor).
//!
//! The original has no cursor: a click hits only the 20 x 20 boxes that
//! hold its point (`_DoSystemMap` @0x12f3e-0x12f53), with no snapping, and
//! is silent. So the cursor is a new feature with no `Rulebook` key, and
//! what it presses is the click's own code.
//!
//! - **Control.** The right stick, read as the map's two axis commands
//!   ([`MapCommand::CursorX`], [`MapCommand::CursorY`]) through the flight
//!   dead zone. Pushed up, it moves up the screen. The map's axes shadow
//!   the shared UI's paging on that stick
//!   ([`axis_shadows`](crate::keymap::contexts::axis_shadows)).
//! - **Speed.** [`CURSOR_SPEED`] map-frame points a second at full
//!   deflection, linear in the dead-zoned amount, on the map's ticks only:
//!   none while Find is open or the map waits to close. It stays inside
//!   item 3, and stays put on the screen while the map pans or zooms under
//!   it. It never scrolls the map at an edge: the left stick pans.
//! - **Snap.** Its target is the nearest visible system whose whole-pixel
//!   point lies within [`SNAP_RADIUS`] of it, ties to the lowest ID,
//!   worked out afresh on each press and draw, so it never sticks.
//! - **South** presses the map as a left click does: at the target's
//!   point, or the cursor's own with none, its press through the click's
//!   press and its release through the click's release, so the press held
//!   for the owner, the drag an unaccepted press starts and the borders
//!   pass at its end are all the click's. **West held** is Plot Course, so
//!   West and South are a Shift-click.
//! - **Which South presses** is the pad's ([`MapPad`](super::pad::MapPad)): whichever the pad
//!   moved last, the cursor or the buttons' focus.
//! - **North** ([`MapCommand::Centre`]) does what Space does, and puts the
//!   cursor back at item 3's centre, where the current system now draws.
//! - **Drawing.** Only while the map was told the pad was used last,
//!   clipped to item 3, in the pad highlight's colour: a ring round the
//!   target's hit box, or a crosshair at the cursor with none.
//!
//! The developer viewer has no cursor, as it has no pad focus.

use std::time::Duration;

use super::catalog::SystemId;
use super::click::hit_box;
use super::dialog::{map_area, map_centre};
use super::map::GalaxyMap;
use super::model::GalaxyModel;
use super::pad::PadStep;
use super::view::Transform;
use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::geometry::{Bounds, Point};
use crate::input::Device;
use crate::keymap::MapCommand;
use crate::ui::pad_focus::draw_highlight;

/// How fast the cursor moves at full deflection: map-frame points a
/// second. Item 3 is 458 points across, about a second and a half.
pub const CURSOR_SPEED: f32 = 300.0;

/// How near a system's point the cursor snaps to it: map-frame points,
/// so it feels the same at every zoom. More than the distance from a
/// system's point to its hit box's far corner
/// ([`HIT_HALF`](super::click::HIT_HALF) x sqrt 2,
/// about 14.1), so a cursor anywhere in a box has a target.
pub const SNAP_RADIUS: f32 = 16.0;

/// How long each arm of the crosshair is.
const ARM: f32 = 5.0;

/// How far from the cursor each arm starts.
const GAP: f32 = 3.0;

/// The cursor: a point in the map's 1024 x 768 frame, inside item 3.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapCursor {
    at: Point,
}

impl Default for MapCursor {
    /// At item 3's centre, where the scroll draws.
    fn default() -> Self {
        Self { at: map_centre() }
    }
}

/// The points the cursor may rest on: item 3, its right and bottom edges
/// out, as `PtInRect` tests it.
fn area() -> Bounds {
    let area = map_area();
    Bounds {
        min: area.min,
        max: Point::new(area.max.x - 1.0, area.max.y - 1.0),
    }
}

impl MapCursor {
    /// The cursor at `at`, kept inside item 3.
    #[must_use]
    pub fn at_point(at: Point) -> Self {
        Self {
            at: area().clamp(at),
        }
    }

    /// Where it is.
    #[must_use]
    pub const fn at(&self) -> Point {
        self.at
    }

    /// Back at item 3's centre.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Moves it by the dead-zoned stick amounts (x right, y up) over
    /// `dt`, at [`CURSOR_SPEED`] for a full push, kept inside item 3.
    pub fn advance(&mut self, (x, y): (f32, f32), dt: Duration) {
        let by = CURSOR_SPEED * dt.as_secs_f32();
        *self = Self::at_point(Point::new(self.at.x + x * by, self.at.y - y * by));
    }

    /// The system it snaps to through `transform`, and its whole-pixel
    /// point: the nearest of `model`'s visible ones within
    /// [`SNAP_RADIUS`], the lowest ID on a tie.
    #[must_use]
    pub fn target(&self, model: &GalaxyModel, transform: &Transform) -> Option<(SystemId, Point)> {
        let mut best: Option<(f32, SystemId, Point)> = None;
        for system in model.systems() {
            let (h, v) = transform.pixel(system.position());
            #[allow(clippy::cast_precision_loss)] // Screen pixels.
            let point = Point::new(h as f32, v as f32);
            let distance = (point.x - self.at.x).hypot(point.y - self.at.y);
            if distance > SNAP_RADIUS {
                continue;
            }
            let id = system.entry.id;
            if best.is_none_or(|(near, near_id, _)| (distance, id) < (near, near_id)) {
                best = Some((distance, id, point));
            }
        }
        best.map(|(_, id, point)| (id, point))
    }

    /// Where South presses: its target's point, or its own with none.
    #[must_use]
    pub fn press_point(&self, model: &GalaxyModel, transform: &Transform) -> Point {
        self.target(model, transform)
            .map_or(self.at, |(_, point)| point)
    }

    /// Draws it in `color`: a ring round its target's hit box, or a
    /// crosshair at its point with none.
    pub fn draw(
        &self,
        list: &mut DrawList,
        model: &GalaxyModel,
        transform: &Transform,
        color: Color,
    ) {
        if let Some((_, point)) = self.target(model, transform) {
            #[allow(clippy::cast_possible_truncation)] // Whole pixels.
            let (h, v) = (point.x as i32, point.y as i32);
            draw_highlight(list, hit_box(h, v), color);
            return;
        }
        let (x, y) = (self.at.x.trunc(), self.at.y.trunc());
        for arm in [
            Bounds::at(Point::new(x + GAP, y), ARM, 1.0),
            Bounds::at(Point::new(x - GAP - ARM + 1.0, y), ARM, 1.0),
            Bounds::at(Point::new(x, y + GAP), 1.0, ARM),
            Bounds::at(Point::new(x, y - GAP - ARM + 1.0), 1.0, ARM),
        ] {
            fill_rect(list, arm, color);
        }
    }
}

impl GalaxyMap {
    /// Where an in-flight map's pad cursor is; `None` on the viewer.
    #[must_use]
    pub fn cursor(&self) -> Option<Point> {
        self.place().map(|_| self.pad.cursor().at())
    }

    /// The map's frame for the cursor: on an in-flight map with Find
    /// shut, moved by the cursor's axes as they are pushed now.
    pub(super) fn cursor_tick(&mut self, dt: Duration) {
        if self.place().is_none() || self.finding() {
            return;
        }
        let amounts = (
            self.keys.axis(MapCommand::CursorX),
            self.keys.axis(MapCommand::CursorY),
        );
        self.pad.advance_cursor(amounts, dt);
    }

    /// South's press or release on the map ([`PadStep::PressMap`],
    /// [`PadStep::ReleaseMap`]): a left button's in item 3, at the
    /// cursor's press point, which the release keeps.
    pub(super) fn cursor_step(&mut self, step: PadStep) {
        match step {
            PadStep::PressMap => {
                let at = self
                    .pad
                    .cursor()
                    .press_point(&self.model, &self.transform());
                self.pad.map_pressed(at);
                self.press_map(at);
            }
            PadStep::ReleaseMap(at) => self.release(at),
            _ => {}
        }
    }

    /// Ends South's press on the map, if one is down, as its release
    /// would: the keys let go of.
    pub(super) fn cursor_let_go(&mut self) {
        if let Some(at) = self.pad.take_map_press() {
            self.release(at);
        }
    }

    /// Draws an in-flight map's cursor through `transform` in `color`,
    /// while the pad was used last.
    pub(super) fn draw_cursor(&self, transform: &Transform, color: Color, list: &mut DrawList) {
        if matches!(self.pad.device(), Device::Gamepad(_)) {
            self.pad.cursor().draw(list, &self.model, transform, color);
        }
    }
}

#[cfg(test)]
mod tests;
