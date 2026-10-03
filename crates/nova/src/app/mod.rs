//! The app layer: window events in, frames out, through ports.
//!
//! [`App`] owns the viewport and the current screen, an [`AppScreen`] that
//! routes to whichever of the game's screens is showing; new screens are
//! added there, so the platform adapter never names one. The adapter
//! hands it [`WindowEvent`]s, already in the core's terms, together with
//! the [`WindowPort`] and a [`Gpu`]; the app turns them into screen input,
//! ticks and draws the screen, and renders each frame through
//! `nova-render`. It never blocks and never names a winit or wgpu type.

use std::time::Duration;

use nova_render::{Gpu, ImageError, ImageSource, LOGICAL, Renderer, Viewport};
use nova_view::{DrawList, ImageKey, Input, Key, MouseButton, Screen, ScreenAction};

pub mod screen;

pub use screen::{AppScreen, start_screen};

/// What the app needs from the window.
pub trait WindowPort {
    /// The window's inner size in physical pixels.
    fn size_px(&self) -> (u32, u32);
    /// Physical pixels per point on the window's display.
    fn scale_factor(&self) -> f64;
    /// Asks for another [`WindowEvent::Redraw`].
    fn request_redraw(&mut self);
}

/// A window event, in the core's terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowEvent {
    /// The window's size or display scale changed.
    Resized {
        /// The new inner size in physical pixels.
        size_px: (u32, u32),
        /// Physical pixels per point.
        scale_factor: f64,
    },
    /// A key went down or up.
    Key {
        /// The key.
        key: Key,
        /// Down (`true`) or up.
        pressed: bool,
    },
    /// The pointer moved to a window position in physical pixels.
    PointerMoved {
        /// Where, in physical pixels from the window's top-left.
        px: (f64, f64),
    },
    /// A mouse button went down or up at the last pointer position.
    PointerButton {
        /// The button.
        button: MouseButton,
        /// Down (`true`) or up.
        pressed: bool,
    },
    /// Time to draw. The clock is read at the edge and passed in.
    Redraw {
        /// Time since the app started.
        elapsed: Duration,
    },
    /// The user asked to close the window.
    CloseRequested,
}

/// Whether the app goes on after an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// Keep running.
    Continue,
    /// Quit.
    Exit,
}

/// The app: routes window events to the current screen and draws it.
///
/// The program runs `App<S>`, whose screen is the [`AppScreen`] router.
/// The screen stays a type parameter only so the app's own tests can drive
/// it with a screen that records what reaches it.
pub struct App<S, C = AppScreen> {
    renderer: Renderer<S>,
    viewport: Viewport,
    screen: C,
    last_redraw: Duration,
    pointer: Option<(f64, f64)>,
    failures: Vec<(ImageKey, ImageError)>,
}

impl<S: ImageSource, C: Screen> App<S, C> {
    /// An app drawing `screen` with images from `images` into `window`.
    pub fn new(window: &impl WindowPort, images: S, screen: C) -> Self {
        Self {
            renderer: Renderer::new(images),
            viewport: Viewport::new(LOGICAL, window.size_px(), window.scale_factor()),
            screen,
            last_redraw: Duration::ZERO,
            pointer: None,
            failures: Vec::new(),
        }
    }

    /// The current screen.
    pub fn screen(&self) -> &C {
        &self.screen
    }

    /// The image source.
    pub fn images(&self) -> &S {
        self.renderer.images()
    }

    /// The logical space's fit into the window.
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// Handles one window event.
    ///
    /// A resize refits the viewport. Keys go to the screen, except that
    /// pressing Escape quits. Pointer events go to the screen in logical
    /// units, and are dropped in the bars. A redraw ticks the screen by the
    /// time since the last redraw, renders its draw list through `gpu` and
    /// asks the window for the next redraw. Closing the window, or a screen
    /// asking to quit, exits.
    pub fn handle(
        &mut self,
        event: WindowEvent,
        window: &mut impl WindowPort,
        gpu: &mut impl Gpu,
    ) -> Control {
        match event {
            WindowEvent::Resized {
                size_px,
                scale_factor,
            } => {
                self.viewport = Viewport::new(LOGICAL, size_px, scale_factor);
                Control::Continue
            }
            WindowEvent::Key {
                key: Key::Escape,
                pressed: true,
            }
            | WindowEvent::CloseRequested => Control::Exit,
            WindowEvent::Key { key, pressed } => self.route(Input::Key { key, pressed }),
            WindowEvent::PointerMoved { px } => {
                self.pointer = Some(px);
                match self.viewport.window_to_logical(px) {
                    Some(at) => self.route(Input::PointerMoved(at)),
                    None => Control::Continue,
                }
            }
            WindowEvent::PointerButton { button, pressed } => {
                let at = self
                    .pointer
                    .and_then(|px| self.viewport.window_to_logical(px));
                match at {
                    Some(at) => self.route(Input::PointerButton {
                        button,
                        pressed,
                        at,
                    }),
                    None => Control::Continue,
                }
            }
            WindowEvent::Redraw { elapsed } => {
                self.screen.tick(elapsed.saturating_sub(self.last_redraw));
                self.last_redraw = self.last_redraw.max(elapsed);
                let mut list = DrawList::new();
                self.screen.draw(&mut list);
                let report = self.renderer.render(&list, &self.viewport, gpu);
                self.failures.extend(report.new_failures);
                window.request_redraw();
                Control::Continue
            }
        }
    }

    /// The image failures reported since the last call, each once.
    pub fn take_failures(&mut self) -> Vec<(ImageKey, ImageError)> {
        std::mem::take(&mut self.failures)
    }

    fn route(&mut self, input: Input) -> Control {
        match self.screen.input(&input) {
            ScreenAction::None => Control::Continue,
            ScreenAction::Quit => Control::Exit,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::time::Duration;

    use nova_data::graphics::Image;
    use nova_render::recording::RecordingGpu;
    use nova_render::{ImageError, ImageSource, PixelRect};
    use nova_view::{
        Color, DrawList, ImageKey, ImageKind, Input, Key, MouseButton, Point, Screen, ScreenAction,
    };

    use super::*;

    /// A window of settable size that counts redraw requests.
    struct FakeWindow {
        size_px: (u32, u32),
        scale_factor: f64,
        redraws: usize,
    }

    impl FakeWindow {
        fn new(size_px: (u32, u32), scale_factor: f64) -> Self {
            Self {
                size_px,
                scale_factor,
                redraws: 0,
            }
        }
    }

    impl WindowPort for FakeWindow {
        fn size_px(&self) -> (u32, u32) {
            self.size_px
        }

        fn scale_factor(&self) -> f64 {
            self.scale_factor
        }

        fn request_redraw(&mut self) {
            self.redraws += 1;
        }
    }

    /// No images at all.
    struct NoImages;

    impl ImageSource for NoImages {
        fn frames(&self, _kind: ImageKind, _id: i16) -> Result<Vec<Image>, ImageError> {
            Err(ImageError::Missing)
        }
    }

    /// Records its inputs and ticks; quits on `quit_on`; draws one sprite.
    #[derive(Default)]
    struct RecordingScreen {
        inputs: Vec<Input>,
        ticks: Vec<Duration>,
        quit_on: Option<Key>,
    }

    impl Screen for RecordingScreen {
        fn input(&mut self, input: &Input) -> ScreenAction {
            self.inputs.push(*input);
            match input {
                Input::Key { key, .. } if Some(*key) == self.quit_on => ScreenAction::Quit,
                _ => ScreenAction::None,
            }
        }

        fn tick(&mut self, dt: Duration) {
            self.ticks.push(dt);
        }

        fn draw(&self, list: &mut DrawList) {
            list.sprite(ImageKey::sprite(1, 0), Point::new(0.0, 0.0), Color::WHITE);
        }
    }

    type TestApp = App<NoImages, RecordingScreen>;

    fn app(window: &FakeWindow) -> TestApp {
        App::new(window, NoImages, RecordingScreen::default())
    }

    fn handle(app: &mut TestApp, window: &mut FakeWindow, event: WindowEvent) -> Control {
        app.handle(event, window, &mut RecordingGpu::new())
    }

    fn key(key: Key, pressed: bool) -> WindowEvent {
        WindowEvent::Key { key, pressed }
    }

    #[test]
    fn the_viewport_starts_at_the_windows_size() {
        let window = FakeWindow::new((2560, 1536), 2.0);
        let app = app(&window);
        assert_eq!(app.viewport().window_px(), (2560, 1536));
        assert_eq!(app.viewport().scale_factor(), 2.0);
        assert_eq!(app.viewport().logical(), nova_render::LOGICAL);
    }

    #[test]
    fn a_resize_rebuilds_the_viewport() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        let resized = WindowEvent::Resized {
            size_px: (1280, 768),
            scale_factor: 1.5,
        };
        assert_eq!(handle(&mut app, &mut window, resized), Control::Continue);
        assert_eq!(app.viewport().window_px(), (1280, 768));
        assert_eq!(app.viewport().scale_factor(), 1.5);
        assert_eq!(
            app.viewport().content().map(|(rect, _)| rect),
            Some(PixelRect {
                x: 128,
                y: 0,
                w: 1024,
                h: 768
            })
        );
    }

    #[test]
    fn keys_go_to_the_screen() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Up, true)),
            Control::Continue
        );
        handle(&mut app, &mut window, key(Key::Up, false));
        handle(&mut app, &mut window, key(Key::Escape, false));
        assert_eq!(
            app.screen().inputs,
            [
                Input::Key {
                    key: Key::Up,
                    pressed: true
                },
                Input::Key {
                    key: Key::Up,
                    pressed: false
                },
                Input::Key {
                    key: Key::Escape,
                    pressed: false
                },
            ]
        );
    }

    #[test]
    fn pressing_escape_exits_without_reaching_the_screen() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Escape, true)),
            Control::Exit
        );
        assert_eq!(app.screen().inputs, []);
    }

    #[test]
    fn a_screen_quit_exits() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let screen = RecordingScreen {
            quit_on: Some(Key::Enter),
            ..RecordingScreen::default()
        };
        let mut app = App::new(&window, NoImages, screen);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Enter, true)),
            Control::Exit
        );
    }

    #[test]
    fn closing_the_window_exits() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert_eq!(
            handle(&mut app, &mut window, WindowEvent::CloseRequested),
            Control::Exit
        );
    }

    #[test]
    fn pointer_events_reach_the_screen_in_logical_units() {
        let mut window = FakeWindow::new((2560, 1536), 2.0);
        let mut app = app(&window);
        // Pillarboxed: content x 256..2304 at scale 2.
        let moved = WindowEvent::PointerMoved { px: (456.0, 100.0) };
        let click = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed: true,
        };
        assert_eq!(handle(&mut app, &mut window, moved), Control::Continue);
        assert_eq!(handle(&mut app, &mut window, click), Control::Continue);
        assert_eq!(
            app.screen().inputs,
            [
                Input::PointerMoved(Point::new(100.0, 50.0)),
                Input::PointerButton {
                    button: MouseButton::Left,
                    pressed: true,
                    at: Point::new(100.0, 50.0)
                },
            ]
        );
    }

    #[test]
    fn pointer_events_in_the_bars_are_dropped() {
        let mut window = FakeWindow::new((2560, 1536), 2.0);
        let mut app = app(&window);
        let click = WindowEvent::PointerButton {
            button: MouseButton::Right,
            pressed: false,
        };
        // No pointer position yet.
        handle(&mut app, &mut window, click);
        handle(
            &mut app,
            &mut window,
            WindowEvent::PointerMoved { px: (10.0, 10.0) },
        );
        handle(&mut app, &mut window, click);
        assert_eq!(app.screen().inputs, []);
    }

    #[test]
    fn a_screen_quit_on_a_pointer_event_exits() {
        struct QuitOnPointer;
        impl Screen for QuitOnPointer {
            fn input(&mut self, _input: &Input) -> ScreenAction {
                ScreenAction::Quit
            }
            fn tick(&mut self, _dt: Duration) {}
            fn draw(&self, _list: &mut DrawList) {}
        }
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = App::new(&window, NoImages, QuitOnPointer);
        let moved = WindowEvent::PointerMoved { px: (10.0, 10.0) };
        let click = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed: true,
        };
        assert_eq!(
            app.handle(moved, &mut window, &mut RecordingGpu::new()),
            Control::Exit
        );
        assert_eq!(
            app.handle(click, &mut window, &mut RecordingGpu::new()),
            Control::Exit
        );
    }

    #[test]
    fn redraws_tick_by_the_time_since_the_last_one() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        for ms in [100, 250, 200, 300] {
            let redraw = WindowEvent::Redraw {
                elapsed: Duration::from_millis(ms),
            };
            assert_eq!(handle(&mut app, &mut window, redraw), Control::Continue);
        }
        // The clock never runs backwards; if it seems to, no time passes.
        assert_eq!(
            app.screen().ticks,
            [100, 150, 0, 50].map(Duration::from_millis)
        );
        assert_eq!(window.redraws, 4);
    }

    #[test]
    fn a_redraw_submits_the_screens_frame_and_reports_failures_once() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        let mut gpu = RecordingGpu::new();
        let redraw = WindowEvent::Redraw {
            elapsed: Duration::ZERO,
        };

        app.handle(redraw, &mut window, &mut gpu);

        assert_eq!(gpu.submits().len(), 1);
        assert_eq!(
            app.take_failures(),
            [(ImageKey::sprite(1, 0), ImageError::Missing)]
        );
        assert_eq!(app.take_failures(), []);
        app.handle(redraw, &mut window, &mut gpu);
        assert_eq!(gpu.submits().len(), 2);
        assert_eq!(app.take_failures(), []);
    }
}
