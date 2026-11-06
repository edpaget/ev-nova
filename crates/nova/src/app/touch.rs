//! The app's touches: a finger's physical pixels mapped to logical points
//! and handed to the touch router (`nova_view::touch`), whose output is
//! delivered with the mouse's own rules.
//!
//! - **The dev overlay first.** While the overlay takes the pointer, every
//!   touch event is the overlay's alone and never reaches the router, as
//!   a mouse event isn't the screen's. Opening the overlay, and losing
//!   focus, cancel every finger ([`App::cancel_touches`]).
//! - **Logical points.** A start maps through
//!   [`Viewport::window_to_logical`](nova_render::Viewport::window_to_logical): in a letterbox bar it is dropped, as
//!   a mouse press there is, so the router never learns that finger and
//!   its later events route nothing. Moves and ends map past the content's
//!   edges ([`Viewport::window_to_logical_unbounded`](nova_render::Viewport::window_to_logical_unbounded)), so a claimed
//!   finger or a pinch can follow a finger into a bar.
//! - **Delivery.** A claimed finger's touch and a pinch go to the screen
//!   as they are. The primary finger is the mouse at the event's own
//!   pixel: its move runs the mouse's move, its press and release the
//!   left button's, and its cancel abandons the screen's press
//!   ([`Screen::cancel_pointer`]). So it gets the mouse's letterbox rules
//!   exactly: a leave, then moves outside while held, and a release in a
//!   bar abandons the press.
//! - **The mouse while a finger is down** reaches no screen and switches
//!   no device ([`App::drops_mouse`]): X11 moves the mouse with the first
//!   touch, and Windows probably promotes a touch to mouse clicks, so
//!   without this a touch would be seen twice.

use nova_render::ImageSource;
use nova_view::devtools::Routing;
use nova_view::touch::{PointerStep, Routed};
use nova_view::{FingerId, Input, MouseButton, Screen, ScreenAction, Touch, TouchPhase};

use super::{App, Control, Handled, WindowEvent};

/// A finger's event as the router takes it: the finger, its phase at a
/// logical point, and the pixel it happened at, if any.
type FingerEvent = (FingerId, TouchPhase, Option<(f64, f64)>);

impl<S: ImageSource, C: Screen> App<S, C> {
    /// A finger's event: the overlay's while it takes the pointer, else
    /// the router's, delivered.
    pub(super) fn touch_event(&mut self, event: WindowEvent) -> Handled {
        if self.pointer_routing() == Routing::Overlay {
            return Handled {
                control: Control::Continue,
                routing: Routing::Overlay,
            };
        }
        let game = |control| Handled {
            control,
            routing: Routing::Game,
        };
        let Some((finger, phase, px)) = self.touch_phase(event) else {
            return game(Control::Continue);
        };
        let input = Input::Touch(Touch { finger, phase });
        let screen = &self.screen;
        let routed = self.touch.route(&input, &|at| screen.claims_touch(at));
        game(self.deliver_touch(routed, px))
    }

    /// `event`'s finger, its phase at a logical point, and its pixel;
    /// `None` for a start in a letterbox bar, or a point in a window with
    /// no area.
    fn touch_phase(&self, event: WindowEvent) -> Option<FingerEvent> {
        let unbounded = |px| self.viewport.window_to_logical_unbounded(px);
        Some(match event {
            WindowEvent::TouchStarted { finger, px } => (
                finger,
                TouchPhase::Started(self.viewport.window_to_logical(px)?),
                Some(px),
            ),
            WindowEvent::TouchMoved { finger, px } => {
                (finger, TouchPhase::Moved(unbounded(px)?), Some(px))
            }
            WindowEvent::TouchEnded { finger, px } => {
                (finger, TouchPhase::Ended(unbounded(px)?), Some(px))
            }
            WindowEvent::TouchCancelled { finger } => (finger, TouchPhase::Cancelled, None),
            _ => return None,
        })
    }

    /// Delivers what the router routed for an event at pixel `px`.
    fn deliver_touch(&mut self, routed: Vec<Routed>, px: Option<(f64, f64)>) -> Control {
        for each in routed {
            let control = match each {
                Routed::Claimed(input) => self.route(input),
                Routed::Pinch(pinch) => self.route(Input::Pinch(pinch)),
                // Every move the router makes is at its event's own point.
                Routed::Pointer(PointerStep::Moved(_)) => {
                    px.map_or(Control::Continue, |px| self.pointer_moved(px).control)
                }
                Routed::Pointer(PointerStep::Pressed(_)) => {
                    self.pointer_button(MouseButton::Left, true).control
                }
                Routed::Pointer(PointerStep::Released(_)) => {
                    self.pointer_button(MouseButton::Left, false).control
                }
                Routed::Pointer(PointerStep::Cancelled) => {
                    self.left_held = false;
                    self.screen.cancel_pointer();
                    self.after_input(ScreenAction::None)
                }
                Routed::Pointer(PointerStep::Left) => Control::Continue,
            };
            if control == Control::Exit {
                return Control::Exit;
            }
        }
        Control::Continue
    }

    /// Cancels every finger, as the overlay opens or the focus is lost:
    /// the claimed fingers' cancels and a pinch's end reach the screen.
    /// The primary finger's press is the caller's to abandon, with the
    /// mouse's ([`Screen::cancel_pointer`]), so it isn't cancelled twice.
    /// A quit the screen asks for here is kept ([`App::after_input`]) and
    /// exits at the next event that can.
    pub(super) fn cancel_touches(&mut self) {
        for each in self.touch.cancel_all() {
            match each {
                Routed::Claimed(input) => {
                    self.route(input);
                }
                Routed::Pinch(pinch) => {
                    self.route(Input::Pinch(pinch));
                }
                Routed::Pointer(_) => {}
            }
        }
    }

    /// Whether `event` is the mouse's while a finger is down: it then
    /// reaches no screen and switches no device. The overlay still gets
    /// the mouse while it shows, as no finger is down then: opening it
    /// cancels them all, and touches go to it alone.
    pub(super) fn drops_mouse(&self, event: &WindowEvent) -> bool {
        matches!(
            event,
            WindowEvent::PointerMoved { .. }
                | WindowEvent::PointerButton { .. }
                | WindowEvent::PointerLeft
        ) && self.touch.any_down()
    }
}
