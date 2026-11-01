//! The app layer: window events in, frames out, through ports.
//!
//! [`App`] owns the viewport and the current screen, an [`AppScreen`] that
//! routes to whichever of the game's screens is showing; new screens are
//! added there, so the platform adapter never names one. The adapter
//! hands it [`WindowEvent`]s, already in the core's terms, together with
//! the [`WindowPort`] and a [`Gpu`]; the app turns them into screen input
//! (every key, Escape included, is the screen's to act on, and so is the
//! text each key press types),
//! ticks and draws the screen, and renders each frame through
//! `nova-render`. It never blocks and never names a winit or wgpu type.
//!
//! # The developer overlay
//!
//! An app built [`App::with_dev_overlay`] holds a [`DevOverlay`]. Its
//! toggle key (backquote) shows and hides it; while it shows, it takes
//! every key and pointer event, and the screen sees none of them.
//! [`App::handle_routed`] says where each event went, and
//! [`App::overlay_wants`] whether the overlay's drawing (egui, in the
//! `dev-tools` build) should see the raw event too. Without an overlay,
//! every event goes to the game as before. Each frame of the overlay's
//! drawing runs through [`App::pilot_desk_frame`], which hands it the
//! pilot flying to edit, and catches the game up after an edit.
//!
//! # Sound
//!
//! An app built [`App::with_audio`] holds the audio core. After every
//! input it routes and every frame it ticks, it hands the core the screen
//! shown ([`Screen::now_showing`]) and the sounds the screen made
//! ([`Screen::take_sounds`]); the core decides what plays. Without one,
//! the sounds are let go.
//!
//! # Settings
//!
//! When the screen reports new preferences
//! ([`Screen::take_prefs`], from the Preferences dialog), the app
//! applies the sound ones to the audio core, if it has one, and saves
//! them all, sound and Hyperspace Effects alike, as one
//! [`GameSettings`] through the settings keeper it was built
//! [`App::with_settings`], if any. The
//! app prints nothing: a save that fails is kept as a warning until the
//! caller takes it ([`App::take_warnings`]) and shows it. So are the
//! warnings the screen reports ([`Screen::take_warnings`]), which the app
//! takes after every input and frame.
//!
//! # Fullscreen
//!
//! The game runs fullscreen borderless, or exclusive in a display mode the
//! player picked ([`FullscreenSetting`]). Off macOS the platform's
//! fullscreen shortcut (Alt+Enter or F11) toggles between the window and
//! the saved kind through the [`WindowPort`], whatever shows, and never
//! reaches the screen or the overlay. The system can change the state
//! itself, and says so with [`WindowEvent::FullscreenChanged`]: on macOS
//! that is the only way it changes, through the View menu's Enter Full
//! Screen (Control+Command+F, which the menu bar takes before the
//! window), Globe+F or the green button. Leaving, either way, gives the
//! window back its size from before, and keeps the kind. Each change is
//! saved as [`GameSettings::fullscreen`] through the settings keeper, if
//! any, and [`App::restore_window`] opens the game the way the player left
//! it.
//!
//! - **The picker.** Each time the Preferences dialog opens, the app
//!   offers it the modes the window's monitor lists
//!   ([`listed_modes`](nova_view::listed_modes)), the kind fullscreen
//!   runs in, and the kind chosen ([`Screen::offer_display`]). On macOS
//!   it offers no modes and borderless, the native Space, as both kinds:
//!   a saved exclusive kind opens borderless there, quietly. A kind the player picks there
//!   ([`Screen::take_display_choice`]) is entered once the event is
//!   handled, and saved, with the monitor's name, as
//!   [`GameSettings::fullscreen_kind`].
//! - **Fallback.** A saved mode the monitor does not offer, or one chosen
//!   on another monitor, opens borderless with a warning naming it, and the
//!   session stays borderless. The saved kind is kept, and still chosen,
//!   so the next launch on that monitor gets the mode back. The dialog
//!   shows borderless, as it runs, and a pick of borderless or of the
//!   saved mode is entered and saved.
//! - **Focus.** Losing focus while exclusive leaves the mode until focus
//!   comes back ([`WindowEvent::FocusGained`]), when it is entered again.
//!   Focus coming back is told to the screen too
//!   ([`Screen::focus_gained`](nova_view::Screen::focus_gained)), and so
//!   is losing it ([`Screen::focus_lost`](nova_view::Screen::focus_lost)),
//!   whose sounds reach the audio at once. While the window is in the
//!   background the app asks for no redraws, as the original sleeps
//!   there with the game paused: losing focus asks for one, to draw what
//!   the switch-out leaves, a menu command asks for one, to show what it
//!   opened, and focus coming back starts them again. Each redraw ticks
//!   the screen by the time since the last, so the screen counts the time
//!   away.
//! - **Exit.** Before every exit, an exclusive mode is left, so the
//!   display gets its own mode back. Nothing is saved on the way out.
//!
//! # Scaling
//!
//! The player picks one Scaling ([`Scaling`](nova_view::Scaling)), which
//! works at one of two sites ([`Scaling::sites`](nova_view::Scaling::sites)):
//! a per-frame upscaler drawing flight's world (`nova_render`'s port,
//! [`Upscaler`](nova_view::Upscaler)), or a load-time enlarger turning
//! each sprite and picture as it loads into the atlas
//! (`nova_render::Enlarger`). Super-xBR, the one load-time entry, draws the
//! world natively; every per-frame entry loads images natively.
//!
//! - **Restore.** Once the GPU exists, [`App::restore_scaling`] asks it
//!   which upscalers the device can use, applies the Scaling the settings
//!   keep ([`GameSettings::frame_upscaler`]) at both sites (an enlarger
//!   only when it isn't native, before the first frame), and offers the
//!   screen the list, native first and Super-xBR last, and the one running
//!   ([`Screen::offer_scaling`]), for the Preferences dialog.
//! - **Fallback.** A stored choice this game does not have, or a per-frame
//!   one the device cannot use, starts native with one warning naming it.
//!   The stored choice is never changed by that, nor by a later change of
//!   another preference, so it comes back on a device that has it.
//! - **The pick.** The dialog reports its pick as the preferences'
//!   Scaling ([`Screen::take_prefs`]); one that is not the one running is
//!   saved with the other preferences, and switches the sites whose half
//!   changed. A new enlarger is set on the renderer at once, and the next
//!   frame starts the atlas afresh and loads what it draws through it; a
//!   new upscaler is set on the GPU just before the next frame is drawn.
//!   Switching between per-frame scalers changes no enlarger, so it never
//!   rebuilds the atlas.
//!
//! # Display shapes
//!
//! The screen says which shape it draws in ([`Screen::view_shape`]):
//! flight widens to the window's aspect ratio, 768 high and as wide as
//! the window makes it, and every other screen keeps Nova's 1024 x 768,
//! letterboxed. The app fits the viewport to the shape
//! ([`Viewport::for_shape`]) when it is built, on every resize, after each
//! input the screen takes and before each frame, so a change of shape
//! (flight entered, a landing) shows from the next frame, and the pointer
//! maps through the shape shown; a screen drawing its own pointer is told
//! again where it is. The screen is told the widened size the window
//! gives ([`Screen::set_widened_size`]) at the start and on every resize,
//! whatever it shows.
//!
//! # Diagnostics
//!
//! An app built [`App::with_diagnostics`] writes each diagnostic the
//! screen reports ([`Screen::take_diagnostics`]: game data the screen could
//! not read or the simulation does not handle yet) as a line, `nova: ` and the diagnostic, through
//! the writer it was given; it takes them where it takes the sounds, after
//! every input and frame. Without a writer, they are let go.
//!
//! # Updates
//!
//! An app built [`App::with_updater`] holds the platform's [`Updater`],
//! started as the app takes it; without one it holds [`NoUpdater`]. The
//! menu bar's Check for Updates… ([`WindowEvent::CheckForUpdates`]) asks
//! it to check now and never reaches the screen, and the menu bar offers
//! the item only when the updater can check ([`App::menu_enabled`]).
//! [`choose`] picks the updater at launch from what the platform offers
//! ([`UpdaterSource`]). When the updater's installer needs the game gone,
//! its request arrives as [`LoopEvent::UpdaterQuit`], which is the Quit
//! command.
//!
//! # Quitting
//!
//! Whichever way the app quits (the window closed, or the screen asking
//! to quit), it first tells the screen ([`Screen::quit`]), so the screen
//! can keep what should outlive it, such as the pilot.

use std::io::Write;
use std::time::Duration;

use nova_audio::{Audio, AudioCore};
use nova_render::{Gpu, ImageError, ImageSource, Renderer, Viewport};
use nova_view::devtools::{DevOverlay, PilotDesk, Routing};
use nova_view::view_shape::widened_size;
use nova_view::{
    DrawList, FullscreenKind, ImageKey, Input, Key, MenuCommand, MonitorMode, MouseButton, Point,
    Screen, ScreenAction, ViewShape,
};

mod fullscreen;
pub mod keyboard_access;
mod loop_event;
mod menu;
mod scaling;
pub mod screen;
mod updates;
mod wheel;

pub use keyboard_access::{KeyboardAccess, NoKeyboardAccessSetting};
pub use loop_event::LoopEvent;
pub use menu::{MenuPick, MenuPort, MenuSync};
pub use screen::{AppScreen, Showing, start_screen};
pub use updates::{Chosen, NoUpdater, Updater, UpdaterSource, choose};
pub use wheel::WheelDelta;

use crate::config::Os;
use crate::settings::{FullscreenSetting, GameKeeper, GameSettings, ScalingSetting};
use fullscreen::{Fullscreen, Shortcut};
use scaling::ScalingChoice;

/// The widened view's size, in logical units, for a window `window_px`
/// in size ([`widened_size`]).
fn widened_logical(window_px: (u32, u32)) -> (f32, f32) {
    let (w, h) = widened_size(window_px);
    (w as f32, h as f32)
}

/// What the app needs from the window.
pub trait WindowPort {
    /// The window's inner size in physical pixels.
    fn size_px(&self) -> (u32, u32);
    /// Physical pixels per point on the window's display.
    fn scale_factor(&self) -> f64;
    /// Asks for another [`WindowEvent::Redraw`].
    fn request_redraw(&mut self);
    /// Shows or hides the system's pointer while it is over the window.
    /// Does nothing by default, for a window with no pointer of its own.
    fn set_pointer_visible(&mut self, visible: bool) {
        let _ = visible;
    }
    /// Makes the window fullscreen on its current display, borderless or
    /// exclusive in a mode, or a window again (`None`), which also leaves
    /// an exclusive mode, giving the display its own back. Does nothing by
    /// default, for a window that cannot go fullscreen.
    fn set_fullscreen(&mut self, fullscreen: Option<FullscreenKind>) {
        let _ = fullscreen;
    }
    /// The display modes the window's current monitor offers for
    /// exclusive fullscreen, as the platform lists them: none by default,
    /// and none where the platform cannot enter them.
    fn video_modes(&self) -> Vec<MonitorMode> {
        Vec::new()
    }
    /// The name of the window's current monitor, if the platform gives
    /// one; `None` by default.
    fn monitor_name(&self) -> Option<String> {
        None
    }
    /// Whether the window is fullscreen, as the system now says; `false`
    /// by default.
    fn is_fullscreen(&self) -> bool {
        false
    }
    /// Asks for the window's inner size to become `size_px`, in physical
    /// pixels as [`WindowPort::size_px`]. The system may refuse or adjust
    /// it; a [`WindowEvent::Resized`] says what it did. Does nothing by
    /// default.
    fn request_inner_size(&mut self, size_px: (u32, u32)) {
        let _ = size_px;
    }
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
        /// Whether this press is the OS repeating a held key; see
        /// [`Input::Key`].
        repeat: bool,
    },
    /// A printable character was typed: it follows the [`WindowEvent::Key`]
    /// press that typed it.
    Text(char),
    /// The pointer moved to a window position in physical pixels.
    PointerMoved {
        /// Where, in physical pixels from the window's top-left.
        px: (f64, f64),
    },
    /// The pointer left the window.
    PointerLeft,
    /// A mouse button went down or up at the last pointer position.
    PointerButton {
        /// The button.
        button: MouseButton,
        /// Down (`true`) or up.
        pressed: bool,
    },
    /// The mouse wheel turned, or a trackpad scrolled, at the last pointer
    /// position. Only the vertical axis is kept.
    Scroll {
        /// How far, toward the text's end for positive.
        delta: WheelDelta,
    },
    /// Time to draw. The clock is read at the edge and passed in.
    Redraw {
        /// Time since the app started.
        elapsed: Duration,
    },
    /// The window stopped receiving keyboard input; the releases of any
    /// keys held down will not arrive.
    FocusLost,
    /// The window receives keyboard input again.
    FocusGained,
    /// Whether the window is fullscreen, as the system now reports it. The
    /// system can change it without the app asking (a window button, a
    /// menu), so the adapter reports it often, the same state again
    /// included; the app acts only on a change.
    FullscreenChanged {
        /// Whether it is now fullscreen.
        fullscreen: bool,
    },
    /// The user asked to close the window.
    CloseRequested,
    /// An item of the platform's menu bar was picked (see `menu`).
    Menu(MenuCommand),
    /// The menu bar's Check for Updates… was picked: the updater checks
    /// now (see `updates`).
    CheckForUpdates,
}

/// Whether the app goes on after an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// Keep running.
    Continue,
    /// Quit.
    Exit,
}

/// What handling one window event did: whether the app goes on, and where
/// the event went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handled {
    /// Whether the app goes on.
    pub control: Control,
    /// Where the event went: [`Routing::Game`] for every event without an
    /// overlay, and for events that are not input.
    pub routing: Routing,
}

/// The app: routes window events to the current screen and draws it.
///
/// The program runs `App<S>`, whose screen is the [`AppScreen`] router.
/// The screen stays a type parameter only so the app's own tests can drive
/// it with a screen that records what reaches it.
#[allow(clippy::struct_excessive_bools)] // Each a separate thing tracked.
pub struct App<S, C = AppScreen> {
    renderer: Renderer<S>,
    viewport: Viewport,
    /// The shape [`App::viewport`] fits: the screen's, as last asked.
    shape: ViewShape,
    screen: C,
    last_redraw: Duration,
    /// Whether the window is in the background: from a focus loss to the
    /// focus coming back. Redraws then ask for no next one.
    background: bool,
    pointer: Option<(f64, f64)>,
    /// Whether the system's pointer is hidden over the window, as the app
    /// last told the window.
    pointer_hidden: bool,
    /// Whether the screen has been told where the pointer is since it
    /// last started drawing its own ([`App::draws_pointer`]).
    cursor_synced: bool,
    failures: Vec<(ImageKey, ImageError)>,
    overlay: Option<DevOverlay>,
    audio: Option<AudioCore<Box<dyn Audio>>>,
    settings: Option<GameKeeper>,
    warnings: Vec<String>,
    diagnostics: Option<Box<dyn Write>>,
    /// The platform's updater ([`App::with_updater`]); [`NoUpdater`] until
    /// one is given.
    updater: Box<dyn Updater>,
    fullscreen: Fullscreen,
    /// A fullscreen kind the Preferences dialog chose, to apply once the
    /// event that reported it is handled.
    chosen_fullscreen: Option<FullscreenKind>,
    /// Whether the open Preferences dialog has been offered the display
    /// modes.
    display_offered: bool,
    /// The Scaling running, and the switches a pick left to make.
    scaling: ScalingChoice,
    /// Whether the screen has been told the app is quitting: it is told
    /// once, however many ways the quit arrives ([`App::exiting`]).
    quit: bool,
}

impl<S: ImageSource, C: Screen> App<S, C> {
    /// An app drawing `screen` with images from `images` into `window`,
    /// telling it the widened size the window gives
    /// ([`Screen::set_widened_size`]).
    pub fn new(window: &impl WindowPort, images: S, mut screen: C) -> Self {
        let shape = screen.view_shape();
        screen.set_widened_size(widened_logical(window.size_px()));
        Self {
            renderer: Renderer::new(images),
            viewport: Viewport::for_shape(shape, window.size_px(), window.scale_factor()),
            shape,
            screen,
            last_redraw: Duration::ZERO,
            background: false,
            pointer: None,
            pointer_hidden: false,
            cursor_synced: false,
            failures: Vec::new(),
            overlay: None,
            audio: None,
            settings: None,
            warnings: Vec::new(),
            diagnostics: None,
            updater: Box::new(NoUpdater),
            fullscreen: Fullscreen::new(Os::current(), window.size_px()),
            chosen_fullscreen: None,
            display_offered: false,
            scaling: ScalingChoice::default(),
            quit: false,
        }
    }

    /// The app behaving as on `os`, rather than the system it was built
    /// for, in fullscreen: it reads `os`'s fullscreen shortcut (none on
    /// macOS, whose View menu takes Control+Command+F), and offers
    /// exclusive fullscreen only off macOS. On [`Os::MacOs`] the
    /// Preferences picker lists no modes, and a saved exclusive kind is
    /// entered borderless, the native Space; other systems offer the
    /// monitor's exclusive modes.
    #[must_use]
    pub fn with_os(mut self, os: Os) -> Self {
        self.fullscreen.set_os(os);
        self
    }

    /// Puts the window, just opened, in the state the player left it in:
    /// fullscreen, in the kind saved, when the settings kept say so. Its
    /// size now is the one given back on leaving. A saved mode the monitor
    /// does not offer opens borderless, with a warning kept for the caller
    /// to show; nothing is saved.
    pub fn restore_window(&mut self, window: &mut impl WindowPort) {
        let saved = self
            .settings
            .as_ref()
            .is_some_and(|keeper| keeper.settings().fullscreen);
        if saved && !self.fullscreen.is_on() {
            let warning = self.fullscreen.toggle(window);
            self.warnings.extend(warning);
        }
    }

    /// Enters the fullscreen kind the Preferences dialog chose, if it
    /// chose one, on the window's current monitor, and saves it, fullscreen
    /// on, through the keeper, if any.
    fn apply_chosen_fullscreen(&mut self, window: &mut impl WindowPort) {
        let Some(kind) = self.chosen_fullscreen.take() else {
            return;
        };
        let setting = match kind {
            FullscreenKind::Borderless => FullscreenSetting::Borderless,
            FullscreenKind::Exclusive(mode) => FullscreenSetting::Exclusive {
                mode,
                monitor: window.monitor_name(),
            },
        };
        let warning = self.fullscreen.choose(setting.clone(), window);
        self.warnings.extend(warning);
        if let Some(keeper) = &mut self.settings {
            let settings = GameSettings {
                fullscreen: true,
                fullscreen_kind: setting,
                ..keeper.settings()
            };
            if let Err(warning) = keeper.change(settings) {
                self.warnings.push(warning);
            }
        }
    }

    /// Offers the Preferences dialog, once each time it opens, the modes
    /// the window's monitor lists, the kind fullscreen runs in, and the
    /// kind chosen, the saved one even when this session fell back from it
    /// ([`Screen::offer_display`]). On macOS, no modes and borderless.
    fn offer_display(&mut self, window: &impl WindowPort) {
        let open = self.screen.now_showing() == Some(Showing::Preferences);
        if open && !self.display_offered {
            let modes = self.fullscreen.listed_modes(window);
            let chosen = self.fullscreen.chosen_kind();
            self.screen
                .offer_display(&modes, self.fullscreen.live_kind(), chosen);
        }
        self.display_offered = open;
    }

    /// Saves whether the game is fullscreen through the keeper, if there
    /// is one; a failed save is kept as a warning.
    fn save_fullscreen(&mut self) {
        if let Some(keeper) = &mut self.settings {
            let settings = GameSettings {
                fullscreen: self.fullscreen.is_on(),
                ..keeper.settings()
            };
            if let Err(warning) = keeper.change(settings) {
                self.warnings.push(warning);
            }
        }
    }

    /// The app with each diagnostic the screen reports written as a line
    /// through `out`.
    #[must_use]
    pub fn with_diagnostics(mut self, out: Box<dyn Write>) -> Self {
        self.diagnostics = Some(out);
        self
    }

    /// The app with a developer overlay, hidden until its toggle key.
    #[must_use]
    pub fn with_dev_overlay(mut self) -> Self {
        self.overlay = Some(DevOverlay::new());
        self
    }

    /// The app with sound: after each input it routes and each frame it
    /// ticks, it gives `core` the screen shown and the sounds the screen
    /// made. The core takes in the screen it starts on at once.
    #[must_use]
    pub fn with_audio(mut self, core: AudioCore<Box<dyn Audio>>) -> Self {
        self.audio = Some(core);
        self.feed_audio();
        self
    }

    /// The app with the player's settings kept by `keeper`: each change
    /// of the preferences is saved through it.
    #[must_use]
    pub fn with_settings(mut self, keeper: GameKeeper) -> Self {
        self.fullscreen.set_kind(keeper.settings().fullscreen_kind);
        self.settings = Some(keeper);
        self
    }

    /// The app with the platform's updater, which starts its background
    /// checks as the app takes it. Check for Updates… then asks it to check
    /// now, and is offered when it can check.
    #[must_use]
    pub fn with_updater(mut self, updater: Box<dyn Updater>) -> Self {
        self.updater = updater;
        self.updater.start();
        self
    }

    /// The warnings since they were last taken, such as a failed save,
    /// for the caller to show; taking them empties the list.
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// Gives the audio core, if there is one, the screen shown and the
    /// sounds the screen has made; without one, the sounds are let go.
    /// Then writes out the screen's diagnostics.
    ///
    /// First, a change of the preferences the screen reports is applied to
    /// the core (the sound ones) and saved through the keeper (all of
    /// them but the fullscreen kind), each when there is one. Its Scaling
    /// is a switch when it is not the one running: a new enlarger is set
    /// on the renderer at once, a new upscaler on the GPU before the next
    /// frame is drawn, and the choice is saved with the rest; otherwise
    /// the stored choice is kept as it is. A fullscreen
    /// kind the player picked ([`Screen::take_display_choice`]) is held, to
    /// be entered and saved once the event is handled, which has the
    /// window; then a galaxy map borders setting
    /// its map's close reports ([`Screen::take_map_borders_save`]) is
    /// saved through the keeper with the other settings as they are. A
    /// failed save is kept as a warning.
    ///
    /// Then, unless the core is still waiting on a keyed click (its 600
    /// playing, [`AudioCore::click_waiting`]), the screen is told the
    /// click has ended ([`Screen::keyed_click_ended`]), and what that runs
    /// is taken in the same way at once: its sounds and the screen it
    /// opens reach the core before the frame is drawn. The original spins
    /// until 600 ends (`_PlayMainScreenClickSound` @0x177e0-0x177f2); here
    /// the end is noticed at the next feed, an input or a frame, which is
    /// still before any frame shows the menu's choice undone.
    fn feed_audio(&mut self) {
        if let Some(prefs) = self.screen.take_prefs() {
            let current = match (&self.settings, &self.audio) {
                (Some(keeper), _) => keeper.settings(),
                (None, Some(core)) => GameSettings {
                    audio: core.settings(),
                    ..GameSettings::default()
                },
                (None, None) => GameSettings::default(),
            };
            let mut settings = current.with_prefs(prefs);
            if self.pick_scaling(prefs.scaling) {
                settings.frame_upscaler = ScalingSetting::from(prefs.scaling);
            }
            if let Some(core) = &mut self.audio {
                core.apply(settings.audio);
            }
            if let Some(keeper) = &mut self.settings
                && let Err(warning) = keeper.change(settings)
            {
                self.warnings.push(warning);
            }
        }
        if let Some(kind) = self.screen.take_display_choice() {
            self.chosen_fullscreen = Some(kind);
        }
        if let Some(map_borders) = self.screen.take_map_borders_save()
            && let Some(keeper) = &mut self.settings
        {
            let settings = GameSettings {
                map_borders,
                ..keeper.settings()
            };
            if let Err(warning) = keeper.change(settings) {
                self.warnings.push(warning);
            }
        }
        self.update_audio();
        if !self.audio.as_ref().is_some_and(AudioCore::click_waiting) {
            self.screen.keyed_click_ended();
            self.update_audio();
        }
        self.write_diagnostics();
    }

    /// Gives the audio core, if there is one, the screen shown and the
    /// sounds the screen has made; without one, the sounds are let go.
    fn update_audio(&mut self) {
        let sounds = self.screen.take_sounds();
        if let Some(core) = &mut self.audio {
            core.update(self.screen.now_showing(), &sounds);
        }
    }

    /// Writes each diagnostic the screen reports as a line through the
    /// writer, if there is one; without one, they are let go. A line that
    /// cannot be written is let go too: there is nowhere else to say so.
    fn write_diagnostics(&mut self) {
        let diagnostics = self.screen.take_diagnostics();
        if let Some(out) = &mut self.diagnostics {
            for diagnostic in diagnostics {
                let _ = writeln!(out, "nova: {diagnostic}");
            }
        }
    }

    /// The developer overlay, if the app has one.
    pub fn dev_overlay(&self) -> Option<&DevOverlay> {
        self.overlay.as_ref()
    }

    /// Whether the overlay's drawing should see the raw window event that
    /// was `handled` as given, or `None` for one the app does not take: true
    /// while the overlay shows, unless the event was its toggle key's.
    pub fn overlay_wants(&self, handled: Option<&Handled>) -> bool {
        self.overlay.as_ref().is_some_and(DevOverlay::visible)
            && handled.is_none_or(|handled| handled.routing != Routing::Toggle)
    }

    /// The current screen.
    pub fn screen(&self) -> &C {
        &self.screen
    }

    /// The current screen, to change: for this crate's integration tests,
    /// whose harnesses ask the flight's screens through the view, as a
    /// screen's ask takes `&mut` (`FlightView::hailing`).
    #[cfg(any(test, feature = "fixture"))]
    pub fn screen_mut(&mut self) -> &mut C {
        &mut self.screen
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
    /// A resize refits the viewport to the screen's shape and tells the
    /// screen the widened size (see the module docs). Keys go to the screen, Escape
    /// included: what it does is the screen's to decide. So does the text
    /// a key press types. Losing focus tells the screen to let go of
    /// the keys it holds down, since their releases will not arrive. Pointer and scroll events go to the screen in logical
    /// units, and are dropped in the bars. A redraw ticks the screen by the
    /// time since the last redraw, renders its draw list through `gpu` and
    /// asks the window for the next redraw. Closing the window asks the
    /// screen first ([`Screen::close_requested`]): it exits at once unless
    /// the screen keeps it open, to quit later if it asks to, or at once
    /// if the feed after the close asks ([`Screen::take_quit`]). A screen
    /// asking to quit exits. On Windows Alt+F4 arrives as that same close:
    /// winit 0.30 passes `WM_SYSKEYDOWN` with `VK_F4` to `DefWindowProc`,
    /// which turns it into `SC_CLOSE` and `WM_CLOSE`, which winit sends as
    /// `CloseRequested`; the keys themselves quit nothing.
    ///
    /// The original's `_MyModalDialog` also redraws the whole status bar
    /// on an update event for its game window (@0x59b17-0x59be6:
    /// `_RedrawStatus(0)` and `_RefreshSAT`). Under Mac OS X's buffered
    /// windows that window gets one only when something invalidates it:
    /// the main screen, a resume from the background in full screen
    /// (@0x59a58), or a Quit the player declines (`_QuitAEVTHandler`
    /// @0x56228-0x56258). Ours redraws the whole frame every frame, in a
    /// window or fullscreen, so a resume needs no redraw of its own, and a
    /// declined close redraws nothing either; that redraw isn't ported. The main
    /// screen's leaves the bar to be drawn at the start of flight, as ours
    /// does.
    ///
    /// With a developer overlay, see [`App::handle_routed`].
    pub fn handle(
        &mut self,
        event: WindowEvent,
        window: &mut impl WindowPort,
        gpu: &mut impl Gpu,
    ) -> Control {
        self.handle_routed(event, window, gpu).control
    }

    /// Handles one window event as [`App::handle`] does, and says where it
    /// went.
    ///
    /// With a developer overlay, keys go through it first: its toggle key
    /// shows or hides it and goes nowhere else, and opening it makes the
    /// screen abandon its pointer gesture and let go of its keys. While it
    /// shows, every other key, pointer and scroll event goes to it alone, wherever
    /// the pointer is, though the pointer's position is still recorded. Every
    /// redraw also gives the overlay its time.
    ///
    /// After the event, a fullscreen kind the Preferences dialog chose is
    /// entered, and a Preferences dialog just opened is offered the display
    /// modes. Before the app exits, an exclusive mode is left, so the
    /// display gets its own back; nothing is saved, so the next launch
    /// opens as this one quit.
    pub fn handle_routed(
        &mut self,
        event: WindowEvent,
        window: &mut impl WindowPort,
        gpu: &mut impl Gpu,
    ) -> Handled {
        let handled = self.dispatch(event, window, gpu);
        self.apply_chosen_fullscreen(window);
        if handled.control == Control::Exit {
            self.fullscreen.exiting(window);
        } else {
            self.offer_display(window);
        }
        handled
    }

    /// The window went to the background, the original's suspend event,
    /// which runs `_SwitchedOut` (@0x4f381): an exclusive mode is left,
    /// the screen's press is abandoned and its keys are let go of, as a
    /// release in the background reaches no screen, and the screen is
    /// told, halting the warp-ups and stopping the playing music; its
    /// sounds are fed to the audio at once, not at the next redraw. A
    /// focus loss reported before the window was ever focused would spend
    /// the music at launch, where the original's suspend only follows a
    /// resume; winit reports none such.
    ///
    /// The app is then in the background: it asks for one redraw, so the
    /// picture the switch-out leaves (the main menu's rollover on its none
    /// frame) is drawn, and that redraw asks for no next one (see
    /// [`App::redraw`]).
    fn focus_lost(&mut self, window: &mut impl WindowPort) {
        self.fullscreen.focus_lost(window);
        self.screen.cancel_pointer();
        self.screen.release_keys();
        self.screen.focus_lost();
        self.feed_audio();
        self.background = true;
        window.request_redraw();
    }

    /// The window came back to the front: an exclusive mode left on
    /// losing focus is entered again, the screen is told, and a redraw is
    /// asked for, which starts the redraws again.
    fn focus_gained(&mut self, window: &mut impl WindowPort) {
        self.fullscreen.focus_gained(window);
        self.screen.focus_gained();
        self.background = false;
        window.request_redraw();
    }

    /// Handles one window event as [`App::handle_routed`] does, before its
    /// after-event steps.
    fn dispatch(
        &mut self,
        event: WindowEvent,
        window: &mut impl WindowPort,
        gpu: &mut impl Gpu,
    ) -> Handled {
        let game = |control| Handled {
            control,
            routing: Routing::Game,
        };
        let overlay = Handled {
            control: Control::Continue,
            routing: Routing::Overlay,
        };
        match event {
            WindowEvent::Resized {
                size_px,
                scale_factor,
            } => {
                self.fullscreen.resized(size_px);
                self.screen.set_widened_size(widened_logical(size_px));
                self.viewport = Viewport::for_shape(self.shape, size_px, scale_factor);
                self.refit_shape();
                game(Control::Continue)
            }
            WindowEvent::CloseRequested => {
                if self.screen.close_requested() {
                    self.quit();
                    return game(Control::Exit);
                }
                self.feed_audio();
                self.take_screen_warnings();
                // A close held behind a keyed click that ended in this
                // feed can quit at once.
                if self.screen.take_quit() {
                    self.quit();
                    return game(Control::Exit);
                }
                game(Control::Continue)
            }
            WindowEvent::FocusLost => {
                self.focus_lost(window);
                game(Control::Continue)
            }
            WindowEvent::FocusGained => {
                self.focus_gained(window);
                game(Control::Continue)
            }
            WindowEvent::FullscreenChanged { fullscreen } => {
                if self.fullscreen.observed(fullscreen, window) {
                    self.save_fullscreen();
                }
                game(Control::Continue)
            }
            WindowEvent::Key {
                key,
                pressed,
                repeat,
            } => self.handle_key(key, pressed, repeat, window),
            WindowEvent::Text(_) if self.fullscreen.drops_text() => game(Control::Continue),
            WindowEvent::Text(c) => {
                if self.overlay.as_ref().is_some_and(DevOverlay::visible) {
                    return overlay;
                }
                game(self.route(Input::Text(c)))
            }
            WindowEvent::PointerMoved { px } => {
                let was_in_view = self.pointer_at().is_some();
                self.pointer = Some(px);
                if self.pointer_routing() == Routing::Overlay {
                    return overlay;
                }
                // The original hides its arrow only inside the game rect
                // (`_PtInRect` @0x38007-0x38017): leaving the viewport for
                // a letterbox bar leaves the screen, as leaving the window
                // does, and `show_pointer` shows the system pointer there.
                game(match self.viewport.window_to_logical(px) {
                    Some(at) => self.route(Input::PointerMoved(at)),
                    None if was_in_view => self.route(Input::PointerLeft),
                    None => Control::Continue,
                })
            }
            WindowEvent::PointerLeft => {
                self.pointer = None;
                if self.pointer_routing() == Routing::Overlay {
                    return overlay;
                }
                game(self.route(Input::PointerLeft))
            }
            WindowEvent::Scroll { delta } => self.scroll(delta),
            WindowEvent::PointerButton { button, pressed } => {
                if self.pointer_routing() == Routing::Overlay {
                    return overlay;
                }
                game(match self.pointer_at() {
                    Some(at) => self.route(Input::PointerButton {
                        button,
                        pressed,
                        at,
                    }),
                    // The primary button let go of outside the game, in a
                    // bar or off the window, ends the screen's press there,
                    // as `_TrackMainScreenButtons` ends on `Button()` going
                    // false wherever the pointer is: on no button, so it is
                    // abandoned.
                    None if button == MouseButton::Left && !pressed => {
                        self.screen.cancel_pointer();
                        Control::Continue
                    }
                    None => Control::Continue,
                })
            }
            WindowEvent::Redraw { elapsed } => game(self.redraw(elapsed, window, gpu)),
            WindowEvent::Menu(command) => game(self.route_menu(command, window)),
            WindowEvent::CheckForUpdates => game(self.check_for_updates()),
        }
    }

    /// A redraw at `elapsed`: ticks the screen and the overlay, then draws
    /// the frame and asks for the next, unless the screen asked to quit or
    /// the app is in the background.
    ///
    /// In the background the original sleeps 30 ticks in `WaitNextEvent`
    /// (`_IterateInGameEventLoop` @0x56885, `_DoMainScreen` @0x1b1dc,
    /// `_MyModalDialog` @0x59965, `_PlayQTMovie` @0x55e23,
    /// `_LoopingMovieIdle` @0x55902), waking twice a second to draw
    /// nothing new, as the game is paused there. The engine asks for no
    /// redraws instead, until the platform or a menu command asks for one
    /// or the focus comes back. Each redraw still ticks the screen by the
    /// time since the last one, so the screen counts the time away.
    fn redraw(
        &mut self,
        elapsed: Duration,
        window: &mut impl WindowPort,
        gpu: &mut impl Gpu,
    ) -> Control {
        if let Some(overlay) = &mut self.overlay {
            overlay.frame(elapsed);
        }
        self.screen.tick(elapsed.saturating_sub(self.last_redraw));
        self.forget_sync_unless_drawing();
        self.feed_audio();
        self.take_screen_warnings();
        self.last_redraw = self.last_redraw.max(elapsed);
        if self.screen.take_quit() {
            self.quit();
            return Control::Exit;
        }
        // A tick can change the shape, as death's fade ends flight on the
        // main menu: this frame draws in the new one.
        self.refit_shape();
        let resync = self.resync_pointer();
        // A dialog a keyed click opened shows its modes from its first
        // frame.
        self.offer_display(window);
        let mut list = DrawList::new();
        self.screen.draw(&mut list);
        // An upscaler picked since the last frame draws this one.
        self.switch_upscaler(gpu);
        let report = self.renderer.render(&list, &self.viewport, gpu);
        self.failures.extend(report.new_failures);
        // What drawing it raised (an upscaler's fallback, say), to show.
        self.warnings.extend(gpu.take_warnings());
        self.show_pointer(window);
        if !self.background {
            window.request_redraw();
        }
        resync
    }

    /// Handles a key: the fullscreen shortcut first, whatever shows, and
    /// otherwise the overlay or the screen, as [`App::route_key`] says.
    fn handle_key(
        &mut self,
        key: Key,
        pressed: bool,
        repeat: bool,
        window: &mut impl WindowPort,
    ) -> Handled {
        let taken = Handled {
            control: Control::Continue,
            routing: Routing::Game,
        };
        match self.fullscreen.key(key, pressed, repeat) {
            Shortcut::Toggle => {
                let warning = self.fullscreen.toggle(window);
                self.warnings.extend(warning);
                self.save_fullscreen();
                return taken;
            }
            Shortcut::Swallowed => return taken,
            Shortcut::None => {}
        }
        match self.route_key(key, pressed, repeat) {
            Routing::Game => Handled {
                control: self.route(Input::Key {
                    key,
                    pressed,
                    repeat,
                }),
                routing: Routing::Game,
            },
            routing => Handled {
                control: Control::Continue,
                routing,
            },
        }
    }

    /// Where a key goes; opening the overlay lets go of the screen.
    fn route_key(&mut self, key: Key, pressed: bool, repeat: bool) -> Routing {
        let Some(overlay) = &mut self.overlay else {
            return Routing::Game;
        };
        let was_visible = overlay.visible();
        let routing = overlay.key(key, pressed, repeat);
        if overlay.visible() && !was_visible {
            self.screen.cancel_pointer();
            self.screen.release_keys();
        }
        routing
    }

    /// Hides the system's pointer over the window while the screen draws
    /// its own ([`Screen::hides_pointer`]) and the developer overlay is
    /// not showing, and shows it otherwise, telling `window` only when
    /// that changes. Over the letterbox bars, outside the game, it is
    /// always shown.
    fn show_pointer(&mut self, window: &mut impl WindowPort) {
        let in_bars = self.pointer.is_some() && self.pointer_at().is_none();
        let hide = self.draws_pointer() && !in_bars;
        if hide != self.pointer_hidden {
            window.set_pointer_visible(!hide);
            self.pointer_hidden = hide;
        }
    }

    /// Whether the screen draws its own pointer: it says it does
    /// ([`Screen::hides_pointer`]) and the developer overlay is not over
    /// it. Over the letterbox bars the screen has already been told the
    /// pointer left, so the bars don't count here.
    fn draws_pointer(&self) -> bool {
        self.screen.hides_pointer() && !self.overlay.as_ref().is_some_and(DevOverlay::visible)
    }

    /// Clears [`App::cursor_synced`] when the screen isn't drawing its own
    /// pointer, so that it is resynced when it starts again.
    fn forget_sync_unless_drawing(&mut self) {
        if !self.draws_pointer() {
            self.cursor_synced = false;
        }
    }

    /// Tells a screen that has started drawing its own pointer again (say
    /// flight, after a dialog over it or the spaceport) where the real
    /// pointer is: a move to it, or a leave when it is outside the game.
    /// It runs before the frame is drawn, so the first frame that shows
    /// the screen uncovered already draws its cursor under the pointer.
    ///
    /// This stands in for `_HandleMouse`'s `GetMouse` each frame
    /// (@0x3801c). A pointer that moved while flight was covered reads as
    /// a move there too, since its point differs from the last one read,
    /// so the cursor shows whole. One edge differs: the original would not
    /// restamp the cursor's fade had the pointer come back to the very
    /// point it last read.
    fn resync_pointer(&mut self) -> Control {
        if !self.draws_pointer() || self.cursor_synced {
            return Control::Continue;
        }
        let input = self
            .pointer_at()
            .map_or(Input::PointerLeft, Input::PointerMoved);
        let control = self.route(input);
        self.cursor_synced = true;
        control
    }

    /// Fits the viewport to the screen's shape when it has changed since
    /// the viewport was fitted: after each input the screen takes, and
    /// before each frame, so a screen that widens (flight entered) or goes
    /// back to classic (landing) draws its next frame in its own shape.
    /// A screen drawing its own pointer is then told again where the
    /// pointer is ([`App::resync_pointer`]), as the same pixel now maps to
    /// another logical point.
    fn refit_shape(&mut self) {
        let shape = self.screen.view_shape();
        if shape == self.shape {
            return;
        }
        self.shape = shape;
        self.viewport = Viewport::for_shape(
            shape,
            self.viewport.window_px(),
            self.viewport.scale_factor(),
        );
        self.cursor_synced = false;
    }

    /// The pointer's logical position, when it is over the game.
    fn pointer_at(&self) -> Option<Point> {
        self.pointer
            .and_then(|px| self.viewport.window_to_logical(px))
    }

    /// A wheel or trackpad scroll by `delta`: the overlay's alone while it
    /// takes the pointer, else the screen's at the pointer. One over a
    /// letterbox bar or off the window is dropped, as a click there is.
    fn scroll(&mut self, delta: WheelDelta) -> Handled {
        if self.pointer_routing() == Routing::Overlay {
            return Handled {
                control: Control::Continue,
                routing: Routing::Overlay,
            };
        }
        let input = self
            .pointer_at()
            .and_then(|at| wheel::wheel_input(delta, &self.viewport, at));
        Handled {
            control: input.map_or(Control::Continue, |input| self.route(input)),
            routing: Routing::Game,
        }
    }

    fn pointer_routing(&self) -> Routing {
        self.overlay
            .as_ref()
            .map_or(Routing::Game, DevOverlay::pointer)
    }

    /// The image failures reported since the last call, each once.
    pub fn take_failures(&mut self) -> Vec<(ImageKey, ImageError)> {
        std::mem::take(&mut self.failures)
    }

    /// Gives `input` to the screen. A pointer input given while the
    /// screen draws its own pointer keeps it in sync; any route that ends
    /// with the screen not drawing its pointer forgets that.
    fn route(&mut self, input: Input) -> Control {
        let pointer = matches!(input, Input::PointerMoved(_) | Input::PointerLeft);
        if pointer && self.draws_pointer() {
            self.cursor_synced = true;
        }
        let action = self.screen.input(&input);
        self.after_input(action)
    }

    /// What follows the screen's answer to an input, `action`: the
    /// viewport refitted to its shape, the audio fed, its warnings kept,
    /// and an exit when it quits, now or through [`Screen::take_quit`].
    fn after_input(&mut self, action: ScreenAction) -> Control {
        self.refit_shape();
        self.forget_sync_unless_drawing();
        self.feed_audio();
        self.take_screen_warnings();
        if action == ScreenAction::Quit || self.screen.take_quit() {
            self.quit();
            return Control::Exit;
        }
        Control::Continue
    }

    /// The event loop is ending (on macOS, `terminate:`, a quit that never
    /// reached the game): tells the screen the app is quitting, so it puts
    /// the pilot away, keeps what it reports, and leaves an exclusive mode,
    /// as a quit through [`App::handle_routed`] does.
    ///
    /// The event loop ends after every quit, the app's own included, so
    /// this also follows those: after the app has quit it does nothing.
    pub fn exiting(&mut self, window: &mut impl WindowPort) {
        if self.quit {
            return;
        }
        self.quit();
        self.fullscreen.exiting(window);
    }

    /// Tells the screen the app is quitting, and keeps what it reports;
    /// after the first time, nothing.
    fn quit(&mut self) {
        if self.quit {
            return;
        }
        self.quit = true;
        self.screen.quit();
        self.take_screen_warnings();
    }

    /// Keeps the warnings the screen reports, for the caller to take.
    fn take_screen_warnings(&mut self) {
        self.warnings.extend(self.screen.take_warnings());
    }
}

impl<S: ImageSource> App<S, AppScreen> {
    /// Runs one frame of the developer tools: `frame` sees the overlay and
    /// the pilot flying as a desk (none before flight), and says whether it
    /// edited the pilot. After an edit the screens catch up with it and the
    /// pilot is saved as after any change ([`AppScreen::after_pilot_edit`]),
    /// a failure kept as a warning. Without an overlay, `frame` does not
    /// run.
    pub fn pilot_desk_frame(
        &mut self,
        frame: impl FnOnce(&DevOverlay, Option<&mut dyn PilotDesk>) -> bool,
    ) {
        let Some(overlay) = &self.overlay else {
            return;
        };
        let mut desk = self.screen.pilot_desk();
        let edited = frame(
            overlay,
            desk.as_mut().map(|desk| desk as &mut dyn PilotDesk),
        );
        if edited {
            self.screen.after_pilot_edit();
            self.take_screen_warnings();
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::time::Duration;

    use nova_audio::recording::{AudioLog, MemorySettings, RecordingAudio};
    use nova_audio::{Audio, AudioCommand, AudioCore, AudioSettings, SettingsStore, Volume};
    use nova_data::graphics::Image;
    use nova_render::recording::RecordingGpu;
    use nova_render::{ImageError, ImageSource, PixelRect};
    use nova_view::sound::SimSound;
    use nova_view::{
        Color, Diagnostic, DrawList, ImageKey, ImageKind, Input, Key, MouseButton, Point, Prefs,
        Screen, ScreenAction, Showing, Sound, SoundPrefs, UiSound, ViewShape,
    };

    use super::*;

    /// A window of settable size that counts redraw requests and records
    /// each time the pointer is shown or hidden, each fullscreen request
    /// and each size request.
    struct FakeWindow {
        size_px: (u32, u32),
        scale_factor: f64,
        redraws: usize,
        pointer_shown: Vec<bool>,
        fullscreen: Vec<Option<FullscreenKind>>,
        size_requests: Vec<(u32, u32)>,
        modes: Vec<MonitorMode>,
        monitor: Option<String>,
    }

    impl FakeWindow {
        fn new(size_px: (u32, u32), scale_factor: f64) -> Self {
            Self {
                size_px,
                scale_factor,
                redraws: 0,
                pointer_shown: Vec::new(),
                fullscreen: Vec::new(),
                size_requests: Vec::new(),
                modes: Vec::new(),
                monitor: None,
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

        fn set_pointer_visible(&mut self, visible: bool) {
            self.pointer_shown.push(visible);
        }

        fn set_fullscreen(&mut self, fullscreen: Option<FullscreenKind>) {
            self.fullscreen.push(fullscreen);
        }

        fn video_modes(&self) -> Vec<MonitorMode> {
            self.modes.clone()
        }

        fn monitor_name(&self) -> Option<String> {
            self.monitor.clone()
        }

        fn request_inner_size(&mut self, size_px: (u32, u32)) {
            self.size_requests.push(size_px);
        }
    }

    /// No images at all.
    struct NoImages;

    impl ImageSource for NoImages {
        fn frames(&self, _kind: ImageKind, _id: i16) -> Result<Vec<Image>, ImageError> {
            Err(ImageError::Missing)
        }
    }

    /// Records its inputs and ticks; quits on `quit_on`; draws one sprite;
    /// counts the times it is told the app quits; hands out `warnings` and
    /// `diagnostics`.
    #[derive(Default)]
    struct RecordingScreen {
        inputs: Vec<Input>,
        ticks: Vec<Duration>,
        quit_on: Option<Key>,
        releases: usize,
        /// How many times it was told the window came back to the front.
        focuses: usize,
        /// How many times it was told the window went to the background.
        focus_losses: usize,
        cancels: usize,
        quits: usize,
        warnings: Vec<String>,
        diagnostics: Vec<Diagnostic>,
        hides_pointer: bool,
        /// Whether to ask to quit, once, outside an input's action.
        asks_quit: bool,
        /// Whether a close keeps the window open.
        keeps_open: bool,
        /// How many closes it was asked about.
        closes: usize,
        /// How many inputs had arrived at each draw.
        drawn_after: std::cell::RefCell<Vec<usize>>,
        /// The shape it draws in.
        shape: ViewShape,
        /// Each widened size it was told, in order.
        widened: Vec<(f32, f32)>,
        /// The shape to switch to on its next input, if any.
        shape_on_input: Option<ViewShape>,
        /// The shape to switch to on its next tick, if any.
        shape_on_tick: Option<ViewShape>,
    }

    impl Screen for RecordingScreen {
        fn input(&mut self, input: &Input) -> ScreenAction {
            self.inputs.push(*input);
            if let Some(shape) = self.shape_on_input.take() {
                self.shape = shape;
            }
            match input {
                Input::Key { key, .. } if Some(*key) == self.quit_on => ScreenAction::Quit,
                _ => ScreenAction::None,
            }
        }

        fn tick(&mut self, dt: Duration) {
            self.ticks.push(dt);
            if let Some(shape) = self.shape_on_tick.take() {
                self.shape = shape;
            }
        }

        fn draw(&self, list: &mut DrawList) {
            self.drawn_after.borrow_mut().push(self.inputs.len());
            list.sprite(ImageKey::sprite(1, 0), Point::new(0.0, 0.0), Color::WHITE);
        }

        fn release_keys(&mut self) {
            self.releases += 1;
        }

        fn focus_gained(&mut self) {
            self.focuses += 1;
        }

        fn focus_lost(&mut self) {
            assert!(self.releases > self.focus_losses, "after the keys' release");
            self.focus_losses += 1;
        }

        fn cancel_pointer(&mut self) {
            self.cancels += 1;
        }

        fn quit(&mut self) {
            self.quits += 1;
            self.warnings.push(format!("quit {}", self.quits));
        }

        fn take_warnings(&mut self) -> Vec<String> {
            std::mem::take(&mut self.warnings)
        }

        fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
            std::mem::take(&mut self.diagnostics)
        }

        fn hides_pointer(&self) -> bool {
            self.hides_pointer
        }

        fn take_quit(&mut self) -> bool {
            std::mem::take(&mut self.asks_quit)
        }

        fn close_requested(&mut self) -> bool {
            self.closes += 1;
            self.warnings.push(format!("close {}", self.closes));
            !self.keeps_open
        }

        fn view_shape(&self) -> ViewShape {
            self.shape
        }

        fn set_widened_size(&mut self, size: (f32, f32)) {
            self.widened.push(size);
        }
    }

    type TestApp = App<NoImages, RecordingScreen>;

    mod background;
    mod display_shapes;
    mod exiting;
    mod menu;
    mod pointer_release;
    mod updates;
    mod wheel;

    /// Bytes written, kept where the test can read them.
    #[derive(Clone, Default)]
    struct Written(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

    impl std::io::Write for Written {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Written {
        fn text(&self) -> String {
            String::from_utf8(self.0.borrow().clone()).expect("UTF-8")
        }
    }

    /// Fails every write.
    struct Broken;

    impl std::io::Write for Broken {
        fn write(&mut self, _bytes: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("closed"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// A sim diagnostic about `weapon`; its text is the sim's own.
    fn unflown(weapon: i16) -> nova_sim::SimDiagnostic {
        nova_sim::SimDiagnostic::UnimplementedGuidance {
            weapon: nova_sim::WeaponId(weapon),
            guidance: 99,
        }
    }

    /// The line `unflown(weapon)` is written as.
    fn unflown_line(weapon: i16) -> String {
        format!("nova: {}\n", unflown(weapon))
    }

    #[test]
    fn each_diagnostic_is_written_as_a_line_after_a_frame_or_an_input() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let written = Written::default();
        let mut app = app(&window).with_diagnostics(Box::new(written.clone()));
        let unreadable = Diagnostic::Unreadable("wëap 140: no spïn 3005".to_owned());
        app.screen.diagnostics = vec![
            Diagnostic::Sim(unflown(181)),
            unreadable,
            Diagnostic::Sim(unflown(165)),
        ];
        let redraw = WindowEvent::Redraw {
            elapsed: Duration::from_millis(16),
        };
        handle(&mut app, &mut window, redraw);
        assert_eq!(
            written.text(),
            format!(
                "{}nova: wëap 140: no spïn 3005\n{}",
                unflown_line(181),
                unflown_line(165)
            )
        );
        app.screen.diagnostics = vec![Diagnostic::Sim(unflown(167))];
        handle(&mut app, &mut window, key(Key::Up, true));
        assert!(
            written
                .text()
                .ends_with(&format!("\n{}", unflown_line(167))),
            "{}",
            written.text()
        );
        assert_eq!(app.screen.diagnostics, [], "taken");
    }

    #[test]
    fn without_a_writer_or_with_a_broken_one_diagnostics_are_let_go() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        app.screen.diagnostics = vec![Diagnostic::Sim(unflown(181))];
        handle(&mut app, &mut window, key(Key::Up, true));
        assert_eq!(app.screen.diagnostics, [], "taken all the same");
        let mut broken = self::app(&window).with_diagnostics(Box::new(Broken));
        broken.screen.diagnostics = vec![Diagnostic::Sim(unflown(181))];
        handle(&mut broken, &mut window, key(Key::Up, true));
        assert_eq!(broken.screen.diagnostics, []);
        assert_eq!(broken.take_warnings(), Vec::<String>::new());
    }

    /// The app over `window`, as on Windows whatever the host, so its
    /// monitor offers exclusive modes.
    fn app(window: &FakeWindow) -> TestApp {
        App::new(window, NoImages, RecordingScreen::default()).with_os(Os::Windows)
    }

    fn handle(app: &mut TestApp, window: &mut FakeWindow, event: WindowEvent) -> Control {
        app.handle(event, window, &mut RecordingGpu::new())
    }

    fn key(key: Key, pressed: bool) -> WindowEvent {
        WindowEvent::Key {
            key,
            pressed,
            repeat: false,
        }
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
        let held = WindowEvent::Key {
            key: Key::Up,
            pressed: true,
            repeat: true,
        };
        handle(&mut app, &mut window, held);
        handle(&mut app, &mut window, key(Key::Up, false));
        handle(&mut app, &mut window, key(Key::Escape, false));
        assert_eq!(
            app.screen().inputs,
            [
                Input::Key {
                    key: Key::Up,
                    pressed: true,
                    repeat: false
                },
                Input::Key {
                    key: Key::Up,
                    pressed: true,
                    repeat: true
                },
                Input::Key {
                    key: Key::Up,
                    pressed: false,
                    repeat: false
                },
                Input::Key {
                    key: Key::Escape,
                    pressed: false,
                    repeat: false
                },
            ]
        );
    }

    #[test]
    fn escape_reaches_the_screen() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Escape, true)),
            Control::Continue
        );
        assert_eq!(
            app.screen().inputs,
            [Input::Key {
                key: Key::Escape,
                pressed: true,
                repeat: false
            }]
        );
    }

    #[test]
    fn a_screen_quit_tells_the_screen_and_exits() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let screen = RecordingScreen {
            quit_on: Some(Key::Enter),
            ..RecordingScreen::default()
        };
        let mut app = App::new(&window, NoImages, screen);
        handle(&mut app, &mut window, key(Key::Up, true));
        assert_eq!(app.screen().quits, 0);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Enter, true)),
            Control::Exit
        );
        assert_eq!(app.screen().quits, 1);
        assert_eq!(app.take_warnings(), ["quit 1"], "what quitting reported");
    }

    #[test]
    fn alt_f4_as_keys_quits_nothing_by_itself() {
        // On Windows it comes as CloseRequested, the close tests' path.
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        for (which, pressed) in [
            (Key::Alt, true),
            (Key::Function(4), true),
            (Key::Function(4), false),
            (Key::Alt, false),
        ] {
            assert_eq!(
                handle(&mut app, &mut window, key(which, pressed)),
                Control::Continue,
                "{which:?} {pressed}"
            );
        }
        assert_eq!(app.screen().quits, 0);
        assert_eq!(app.take_warnings(), Vec::<String>::new(), "no close");
    }

    #[test]
    fn a_close_the_screen_keeps_open_goes_on_until_it_asks_to_quit() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = App::new(
            &window,
            NoImages,
            RecordingScreen {
                keeps_open: true,
                quit_on: Some(Key::Char('y')),
                ..RecordingScreen::default()
            },
        );
        assert_eq!(
            handle(&mut app, &mut window, WindowEvent::CloseRequested),
            Control::Continue
        );
        assert_eq!(app.screen().quits, 0, "not quitting");
        assert_eq!(app.take_warnings(), ["close 1"], "what it reported");
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Char('y'), true)),
            Control::Exit,
            "the answer quits"
        );
        assert_eq!(app.screen().quits, 1);
    }

    #[test]
    fn a_close_the_screen_keeps_open_but_then_quits_on_exits_at_once() {
        // As a close held behind a keyed click that ends in the same feed,
        // with no question to ask in.
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = App::new(
            &window,
            NoImages,
            RecordingScreen {
                keeps_open: true,
                asks_quit: true,
                ..RecordingScreen::default()
            },
        );
        assert_eq!(
            handle(&mut app, &mut window, WindowEvent::CloseRequested),
            Control::Exit
        );
        assert_eq!(app.screen().quits, 1);
        assert_eq!(app.take_warnings(), ["close 1", "quit 1"]);
    }

    #[test]
    fn typed_text_reaches_the_screen_after_its_key() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Char('p'), true)),
            Control::Continue
        );
        assert_eq!(
            handle(&mut app, &mut window, WindowEvent::Text('P')),
            Control::Continue
        );
        assert_eq!(
            app.screen().inputs,
            [
                Input::Key {
                    key: Key::Char('p'),
                    pressed: true,
                    repeat: false
                },
                Input::Text('P'),
            ]
        );
    }

    #[test]
    fn a_screen_quit_on_typed_text_exits() {
        struct QuitOnText;
        impl Screen for QuitOnText {
            fn input(&mut self, input: &Input) -> ScreenAction {
                if matches!(input, Input::Text(_)) {
                    ScreenAction::Quit
                } else {
                    ScreenAction::None
                }
            }
            fn tick(&mut self, _dt: Duration) {}
            fn draw(&self, _list: &mut DrawList) {}
        }
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = App::new(&window, NoImages, QuitOnText);
        assert_eq!(
            app.handle(
                WindowEvent::Text('q'),
                &mut window,
                &mut RecordingGpu::new()
            ),
            Control::Exit
        );
    }

    #[test]
    fn the_screens_warnings_are_kept_after_each_input_and_redraw() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let screen = RecordingScreen {
            warnings: vec!["first".to_owned()],
            ..RecordingScreen::default()
        };
        let mut app = App::new(&window, NoImages, screen);
        assert_eq!(app.take_warnings(), Vec::<String>::new(), "none taken yet");
        handle(&mut app, &mut window, key(Key::Up, true));
        assert_eq!(app.take_warnings(), ["first"]);
        app.screen.warnings.push("second".to_owned());
        handle(
            &mut app,
            &mut window,
            WindowEvent::Redraw {
                elapsed: Duration::from_millis(16),
            },
        );
        assert_eq!(app.take_warnings(), ["second"]);
        app.screen.warnings.push("third".to_owned());
        handle(&mut app, &mut window, WindowEvent::Text('a'));
        assert_eq!(app.take_warnings(), ["third"]);
        app.screen.warnings.push("fourth".to_owned());
        handle(
            &mut app,
            &mut window,
            WindowEvent::PointerMoved { px: (5.0, 5.0) },
        );
        assert_eq!(app.take_warnings(), ["fourth"]);
    }

    #[test]
    fn losing_focus_releases_the_screens_keys_and_carries_on() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        handle(&mut app, &mut window, key(Key::Right, true));
        assert_eq!(app.screen().releases, 0);
        assert_eq!(
            handle(&mut app, &mut window, WindowEvent::FocusLost),
            Control::Continue
        );
        assert_eq!(app.screen().releases, 1);
        assert_eq!(app.screen().inputs.len(), 1, "no input for it");
    }

    #[test]
    fn losing_focus_tells_the_screen() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        handle(&mut app, &mut window, WindowEvent::FocusLost);
        assert_eq!(app.screen().focus_losses, 1);
        assert_eq!(app.screen().inputs, [], "no input for it");
    }

    #[test]
    fn gaining_focus_tells_the_screen_and_carries_on() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert_eq!(
            handle(&mut app, &mut window, WindowEvent::FocusGained),
            Control::Continue
        );
        assert_eq!(app.screen().focuses, 1);
        assert_eq!(app.screen().inputs, [], "no input for it");
        assert_eq!(app.screen().releases, 0);
    }

    #[test]
    fn closing_the_window_tells_the_screen_and_exits() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert_eq!(
            handle(&mut app, &mut window, WindowEvent::CloseRequested),
            Control::Exit
        );
        assert_eq!(app.screen().quits, 1);
        assert_eq!(app.screen().closes, 1, "asked first");
        assert_eq!(
            app.take_warnings(),
            ["close 1", "quit 1"],
            "what quitting reported"
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
    fn the_pointer_leaving_the_window_reaches_the_screen_and_forgets_where_it_was() {
        let mut window = FakeWindow::new((2560, 1536), 2.0);
        let mut app = app(&window);
        let moved = WindowEvent::PointerMoved { px: (456.0, 100.0) };
        let click = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed: true,
        };
        handle(&mut app, &mut window, moved);
        assert_eq!(
            handle(&mut app, &mut window, WindowEvent::PointerLeft),
            Control::Continue
        );
        handle(&mut app, &mut window, click);
        assert_eq!(
            app.screen().inputs,
            [
                Input::PointerMoved(Point::new(100.0, 50.0)),
                Input::PointerLeft
            ],
            "a click with no pointer is dropped"
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

    fn overlay_app(window: &FakeWindow) -> TestApp {
        let screen = RecordingScreen {
            quit_on: Some(Key::Escape),
            ..RecordingScreen::default()
        };
        App::new(window, NoImages, screen).with_dev_overlay()
    }

    fn routed(app: &mut TestApp, window: &mut FakeWindow, event: WindowEvent) -> Handled {
        app.handle_routed(event, window, &mut RecordingGpu::new())
    }

    fn handled(control: Control, routing: Routing) -> Handled {
        Handled { control, routing }
    }

    const BACKQUOTE: Key = Key::Char('`');

    fn visible(app: &TestApp) -> bool {
        app.dev_overlay().expect("an overlay").visible()
    }

    #[test]
    fn without_an_overlay_backquote_reaches_the_screen() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert!(app.dev_overlay().is_none());
        assert_eq!(
            routed(&mut app, &mut window, key(BACKQUOTE, true)),
            handled(Control::Continue, Routing::Game)
        );
        assert_eq!(app.screen().inputs.len(), 1);
        assert!(!app.overlay_wants(None));
        let game = handled(Control::Continue, Routing::Game);
        assert!(!app.overlay_wants(Some(&game)));
    }

    #[test]
    fn handle_gives_the_routed_control() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Escape, true)),
            Control::Exit
        );
    }

    #[test]
    fn the_toggle_key_opens_the_overlay_and_lets_go_of_the_screen() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        assert!(!visible(&app), "starts hidden");
        assert_eq!(
            routed(&mut app, &mut window, key(BACKQUOTE, true)),
            handled(Control::Continue, Routing::Toggle)
        );
        assert!(visible(&app));
        assert_eq!(app.screen().inputs, []);
        assert_eq!((app.screen().cancels, app.screen().releases), (1, 1));
    }

    fn redraw(app: &mut TestApp, window: &mut FakeWindow) {
        let elapsed = Duration::from_millis(16);
        routed(app, window, WindowEvent::Redraw { elapsed });
    }

    #[test]
    fn a_frame_hides_the_system_pointer_while_the_screen_draws_its_own_telling_only_changes() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        redraw(&mut app, &mut window);
        assert_eq!(
            window.pointer_shown,
            Vec::<bool>::new(),
            "shown from the start"
        );
        app.screen.hides_pointer = true;
        handle(&mut app, &mut window, key(Key::Up, true));
        assert_eq!(
            window.pointer_shown,
            Vec::<bool>::new(),
            "not until the frame"
        );
        redraw(&mut app, &mut window);
        redraw(&mut app, &mut window);
        assert_eq!(window.pointer_shown, [false], "once");
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        redraw(&mut app, &mut window);
        assert_eq!(window.pointer_shown, [false, true], "over the overlay");
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        redraw(&mut app, &mut window);
        app.screen.hides_pointer = false;
        redraw(&mut app, &mut window);
        assert_eq!(window.pointer_shown, [false, true, false, true]);
    }

    #[test]
    fn the_pointer_moving_into_the_bars_leaves_the_screen_and_shows_the_system_pointer() {
        // Pillarboxed: content x 256..2304 at scale 2.
        let mut window = FakeWindow::new((2560, 1536), 2.0);
        let mut app = app(&window);
        app.screen.hides_pointer = true;
        let into_view = WindowEvent::PointerMoved { px: (456.0, 100.0) };
        let into_bar = WindowEvent::PointerMoved { px: (10.0, 10.0) };
        handle(&mut app, &mut window, into_view);
        redraw_plain(&mut app, &mut window);
        assert_eq!(window.pointer_shown, [false], "hidden over the game");
        handle(&mut app, &mut window, into_bar);
        handle(&mut app, &mut window, into_bar);
        redraw_plain(&mut app, &mut window);
        assert_eq!(window.pointer_shown, [false, true], "shown over a bar");
        handle(&mut app, &mut window, into_view);
        redraw_plain(&mut app, &mut window);
        assert_eq!(window.pointer_shown, [false, true, false], "hidden again");
        assert_eq!(
            app.screen().inputs,
            [
                Input::PointerMoved(Point::new(100.0, 50.0)),
                Input::PointerLeft,
                Input::PointerMoved(Point::new(100.0, 50.0)),
            ],
            "one leave on the way out"
        );
    }

    fn moved(x: f64, y: f64) -> WindowEvent {
        WindowEvent::PointerMoved { px: (x, y) }
    }

    #[test]
    fn a_screen_drawing_its_pointer_again_is_told_where_the_pointer_is_before_the_frame() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        app.screen.hides_pointer = true;
        handle(&mut app, &mut window, moved(100.0, 50.0));
        redraw_plain(&mut app, &mut window);
        assert_eq!(
            app.screen().inputs,
            [Input::PointerMoved(Point::new(100.0, 50.0))]
        );
        app.screen.hides_pointer = false;
        handle(&mut app, &mut window, moved(300.0, 200.0));
        redraw_plain(&mut app, &mut window);
        assert_eq!(app.screen().inputs.len(), 2, "the cover had the move");
        app.screen.hides_pointer = true;
        redraw_plain(&mut app, &mut window);
        assert_eq!(
            app.screen().inputs[2..],
            [Input::PointerMoved(Point::new(300.0, 200.0))],
            "exactly one resync"
        );
        assert_eq!(
            app.screen().drawn_after.borrow().last(),
            Some(&3),
            "before the frame"
        );
    }

    #[test]
    fn with_the_pointer_outside_the_game_the_resync_is_a_leave() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        app.screen.hides_pointer = true;
        handle(&mut app, &mut window, moved(100.0, 50.0));
        redraw_plain(&mut app, &mut window);
        app.screen.hides_pointer = false;
        handle(&mut app, &mut window, WindowEvent::PointerLeft);
        redraw_plain(&mut app, &mut window);
        app.screen.hides_pointer = true;
        redraw_plain(&mut app, &mut window);
        assert_eq!(
            app.screen().inputs,
            [
                Input::PointerMoved(Point::new(100.0, 50.0)),
                Input::PointerLeft,
                Input::PointerLeft,
            ],
            "outside the window"
        );

        // Pillarboxed: content x 256..2304 at scale 2.
        let mut window = FakeWindow::new((2560, 1536), 2.0);
        let mut app: TestApp = App::new(&window, NoImages, RecordingScreen::default());
        app.screen.hides_pointer = true;
        handle(&mut app, &mut window, moved(456.0, 100.0));
        redraw_plain(&mut app, &mut window);
        app.screen.hides_pointer = false;
        handle(&mut app, &mut window, moved(10.0, 10.0));
        redraw_plain(&mut app, &mut window);
        app.screen.hides_pointer = true;
        redraw_plain(&mut app, &mut window);
        assert_eq!(
            app.screen().inputs,
            [
                Input::PointerMoved(Point::new(100.0, 50.0)),
                Input::PointerLeft,
                Input::PointerLeft,
            ],
            "over a bar"
        );
    }

    /// A cover and uncover between two frames resyncs only when a route or
    /// a tick ended with the cover up. One that opens and closes inside a
    /// single route is not resynced, which is harmless: flight's openers
    /// call `cancel_pointer`, which already hid the cursor until the next
    /// move.
    #[test]
    fn no_resync_while_the_screen_keeps_its_pointer() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        app.screen.hides_pointer = true;
        handle(&mut app, &mut window, moved(100.0, 50.0));
        for _ in 0..3 {
            redraw_plain(&mut app, &mut window);
        }
        assert_eq!(app.screen().inputs.len(), 1, "no resync");
        app.screen.hides_pointer = false;
        handle(&mut app, &mut window, key(Key::Up, true));
        app.screen.hides_pointer = true;
        redraw_plain(&mut app, &mut window);
        redraw_plain(&mut app, &mut window);
        assert_eq!(
            app.screen().inputs[1..],
            [
                Input::Key {
                    key: Key::Up,
                    pressed: true,
                    repeat: false,
                },
                Input::PointerMoved(Point::new(100.0, 50.0)),
            ],
            "one resync after a route that ended covered"
        );
    }

    #[test]
    fn a_key_given_to_a_screen_drawing_its_pointer_does_not_tell_it_where_the_pointer_is() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        handle(&mut app, &mut window, moved(100.0, 50.0));
        redraw_plain(&mut app, &mut window);
        app.screen.hides_pointer = true;
        handle(&mut app, &mut window, key(Key::Up, true));
        redraw_plain(&mut app, &mut window);
        assert_eq!(
            app.screen().inputs.last(),
            Some(&Input::PointerMoved(Point::new(100.0, 50.0))),
            "resynced after the key"
        );
    }

    #[test]
    fn a_screen_that_does_not_draw_its_pointer_gets_no_resync() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        handle(&mut app, &mut window, moved(100.0, 50.0));
        redraw_plain(&mut app, &mut window);
        redraw_plain(&mut app, &mut window);
        assert_eq!(app.screen().inputs.len(), 1);
    }

    #[test]
    fn closing_the_overlay_resyncs_the_pointer() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        app.screen.hides_pointer = true;
        handle(&mut app, &mut window, moved(100.0, 50.0));
        redraw(&mut app, &mut window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        redraw(&mut app, &mut window);
        handle(&mut app, &mut window, moved(300.0, 200.0));
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        assert_eq!(app.screen().inputs.len(), 1, "the overlay had the move");
        redraw(&mut app, &mut window);
        assert_eq!(
            app.screen().inputs[1..],
            [Input::PointerMoved(Point::new(300.0, 200.0))]
        );
    }

    fn redraw_plain(app: &mut TestApp, window: &mut FakeWindow) {
        let elapsed = Duration::from_millis(16);
        handle(app, window, WindowEvent::Redraw { elapsed });
    }

    #[test]
    fn the_toggle_keys_repeat_and_release_touch_nothing() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        let repeat = WindowEvent::Key {
            key: BACKQUOTE,
            pressed: true,
            repeat: true,
        };
        for event in [repeat, key(BACKQUOTE, false)] {
            assert_eq!(
                routed(&mut app, &mut window, event),
                handled(Control::Continue, Routing::Toggle)
            );
        }
        assert!(visible(&app));
        assert_eq!(app.screen().inputs, []);
        assert_eq!((app.screen().cancels, app.screen().releases), (1, 1));
    }

    #[test]
    fn keys_go_to_the_overlay_alone_while_it_shows() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        for k in [Key::Escape, Key::Tab, Key::Right, Key::Char('d')] {
            for pressed in [true, false] {
                assert_eq!(
                    routed(&mut app, &mut window, key(k, pressed)),
                    handled(Control::Continue, Routing::Overlay),
                    "{k:?}"
                );
            }
        }
        assert_eq!(app.screen().inputs, []);
        assert!(visible(&app), "Escape does not close it");
    }

    #[test]
    fn typed_text_goes_to_the_overlay_alone_while_it_shows() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        assert_eq!(
            routed(&mut app, &mut window, WindowEvent::Text('a')),
            handled(Control::Continue, Routing::Game)
        );
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        assert_eq!(
            routed(&mut app, &mut window, WindowEvent::Text('b')),
            handled(Control::Continue, Routing::Overlay)
        );
        assert_eq!(app.screen().inputs, [Input::Text('a')]);
    }

    #[test]
    fn closing_the_overlay_releases_nothing_and_gives_keys_back() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        routed(&mut app, &mut window, key(BACKQUOTE, false));
        assert_eq!(
            routed(&mut app, &mut window, key(BACKQUOTE, true)),
            handled(Control::Continue, Routing::Toggle)
        );
        assert!(!visible(&app));
        assert_eq!((app.screen().cancels, app.screen().releases), (1, 1));
        assert_eq!(
            routed(&mut app, &mut window, key(Key::Tab, true)),
            handled(Control::Continue, Routing::Game)
        );
        assert_eq!(
            routed(&mut app, &mut window, key(Key::Escape, true)),
            handled(Control::Exit, Routing::Game)
        );
        assert_eq!(app.screen().inputs.len(), 2);
    }

    #[test]
    fn the_pointer_goes_to_the_overlay_alone_while_it_shows_even_in_the_bars() {
        let mut window = FakeWindow::new((2560, 1536), 2.0);
        let mut app = overlay_app(&window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        let click = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed: true,
        };
        for px in [(456.0, 100.0), (10.0, 10.0)] {
            assert_eq!(
                routed(&mut app, &mut window, WindowEvent::PointerMoved { px }),
                handled(Control::Continue, Routing::Overlay)
            );
            assert_eq!(
                routed(&mut app, &mut window, click),
                handled(Control::Continue, Routing::Overlay)
            );
        }
        assert_eq!(
            routed(&mut app, &mut window, WindowEvent::PointerLeft),
            handled(Control::Continue, Routing::Overlay)
        );
        assert_eq!(app.screen().inputs, []);
    }

    #[test]
    fn after_closing_a_click_lands_where_the_pointer_is() {
        let mut window = FakeWindow::new((2560, 1536), 2.0);
        let mut app = overlay_app(&window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        routed(
            &mut app,
            &mut window,
            WindowEvent::PointerMoved { px: (456.0, 100.0) },
        );
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        let click = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed: true,
        };
        assert_eq!(
            routed(&mut app, &mut window, click),
            handled(Control::Continue, Routing::Game)
        );
        assert_eq!(
            app.screen().inputs,
            [Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at: Point::new(100.0, 50.0)
            }]
        );
    }

    #[test]
    fn redraws_feed_the_overlays_frame_times_hidden_or_visible() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        let redraw = |ms| WindowEvent::Redraw {
            elapsed: Duration::from_millis(ms),
        };
        assert_eq!(
            routed(&mut app, &mut window, redraw(0)),
            handled(Control::Continue, Routing::Game)
        );
        routed(&mut app, &mut window, redraw(16));
        let times = app.dev_overlay().expect("an overlay").frame_times();
        assert_eq!(times.count(), 1);
        assert_eq!(times.last(), Some(Duration::from_millis(16)));
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        assert_eq!(
            routed(&mut app, &mut window, redraw(33)),
            handled(Control::Continue, Routing::Game)
        );
        let times = app.dev_overlay().expect("an overlay").frame_times();
        assert_eq!(times.count(), 2);
        assert_eq!(times.last(), Some(Duration::from_millis(17)));
        assert_eq!(app.screen().ticks.len(), 3, "the screen still ticks");
        assert_eq!(window.redraws, 3);
    }

    #[test]
    fn other_events_go_to_the_game() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        let resized = WindowEvent::Resized {
            size_px: (1280, 768),
            scale_factor: 1.0,
        };
        assert_eq!(
            routed(&mut app, &mut window, resized),
            handled(Control::Continue, Routing::Game)
        );
        assert_eq!(
            routed(&mut app, &mut window, WindowEvent::FocusLost),
            handled(Control::Continue, Routing::Game)
        );
        assert_eq!(
            routed(&mut app, &mut window, WindowEvent::CloseRequested),
            handled(Control::Exit, Routing::Game)
        );
    }

    #[test]
    fn the_overlay_wants_every_event_but_its_toggle_while_it_shows() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        let overlay = handled(Control::Continue, Routing::Overlay);
        let game = handled(Control::Continue, Routing::Game);
        let toggle = handled(Control::Continue, Routing::Toggle);
        for event in [None, Some(&overlay), Some(&game), Some(&toggle)] {
            assert!(!app.overlay_wants(event), "hidden: {event:?}");
        }
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        assert!(app.overlay_wants(None));
        assert!(app.overlay_wants(Some(&overlay)));
        assert!(app.overlay_wants(Some(&game)));
        assert!(!app.overlay_wants(Some(&toggle)));
    }

    // Audio.

    /// Shows `showing`, which Tab changes to the ship browser; sounds a
    /// button going down on each input and coming up on each tick.
    struct SoundingScreen {
        showing: Showing,
        sounds: Vec<Sound>,
    }

    impl Screen for SoundingScreen {
        fn input(&mut self, input: &Input) -> ScreenAction {
            if let Input::Key { key: Key::Tab, .. } = input {
                self.showing = Showing::ShipBrowser;
            }
            self.sounds.push(Sound::Ui(UiSound::ButtonDown));
            ScreenAction::None
        }

        fn tick(&mut self, _dt: Duration) {
            self.sounds.push(Sound::Ui(UiSound::ButtonUp));
        }

        fn draw(&self, _list: &mut DrawList) {}

        fn take_sounds(&mut self) -> Vec<Sound> {
            std::mem::take(&mut self.sounds)
        }

        fn now_showing(&self) -> Option<Showing> {
            Some(self.showing)
        }
    }

    /// `id` played at the default effects volume and `priority`.
    fn play(id: i16, priority: nova_audio::Priority) -> AudioCommand {
        AudioCommand::Play {
            sound: nova_data::SoundId(id),
            volume: AudioSettings::default().effects_volume,
            priority,
        }
    }

    /// The cue `sound` plays, by its own say.
    fn cue_of(sound: Sound) -> nova_view::sound::SoundCue {
        match sound {
            Sound::Sim(sim) => sim.cue(),
            Sound::Ui(ui) => ui.cue(),
            Sound::Combat(_) => None,
        }
        .unwrap_or_else(|| panic!("{sound:?} plays a cue"))
    }

    /// What `sound` plays at the default effects volume: its cue's `snd `
    /// at its priority.
    fn cued(sound: Sound) -> AudioCommand {
        let cue = cue_of(sound);
        play(cue.sound.0, nova_audio::Priority::new(cue.priority))
    }

    /// A main-menu button's click, its two halves.
    const DOWN: Sound = Sound::Ui(UiSound::ButtonDown);
    const UP: Sound = Sound::Ui(UiSound::ButtonUp);

    /// The warp-up the sounding screen raises.
    const JUMPED: Sound = Sound::Sim(SimSound::JumpBegan { rate: 0x1_0000 });

    /// The music started at its default volume.
    fn music() -> AudioCommand {
        AudioCommand::StartMusic {
            volume: AudioSettings::default().music_volume,
        }
    }

    /// An app over a screen on the main menu, with audio recorded.
    fn sounding(window: &FakeWindow) -> (App<NoImages, SoundingScreen>, AudioLog) {
        let screen = SoundingScreen {
            showing: Showing::MainMenu,
            sounds: vec![JUMPED],
        };
        let audio = RecordingAudio::new();
        let log = audio.log();
        let core = AudioCore::new(Box::new(audio) as Box<dyn Audio>);
        (App::new(window, NoImages, screen).with_audio(core), log)
    }

    #[test]
    fn attaching_audio_takes_in_the_screen_and_its_sounds_at_once() {
        let window = FakeWindow::new((1024, 768), 1.0);
        let (app, log) = sounding(&window);
        assert_eq!(*log.borrow(), [music(), cued(JUMPED)]);
        assert_eq!(app.screen().sounds, [], "taken");
    }

    #[test]
    fn each_input_and_redraw_feeds_the_screens_sounds_to_the_audio() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let (mut app, log) = sounding(&window);
        log.borrow_mut().clear();
        let mut gpu = RecordingGpu::new();
        app.handle(key(Key::Up, true), &mut window, &mut gpu);
        assert_eq!(*log.borrow(), [cued(DOWN)]);
        app.handle(
            WindowEvent::Redraw {
                elapsed: Duration::from_millis(16),
            },
            &mut window,
            &mut gpu,
        );
        assert_eq!(*log.borrow(), [cued(DOWN), cued(UP)]);
        app.handle(
            WindowEvent::PointerMoved { px: (5.0, 5.0) },
            &mut window,
            &mut gpu,
        );
        assert_eq!(log.borrow().len(), 3, "pointer input too");
        app.handle(key(Key::Tab, true), &mut window, &mut gpu);
        assert_eq!(
            log.borrow()[3..],
            [
                AudioCommand::FadeOutMusic(nova_audio::music_fade(
                    AudioSettings::default().music_volume
                )),
                cued(DOWN)
            ],
            "the screen shown, then its sounds"
        );
    }

    #[test]
    fn without_audio_the_screens_sounds_are_let_go() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let screen = SoundingScreen {
            showing: Showing::GalaxyMap,
            sounds: Vec::new(),
        };
        let mut app = App::new(&window, NoImages, screen);
        let mut gpu = RecordingGpu::new();
        app.handle(key(Key::Up, true), &mut window, &mut gpu);
        assert_eq!(app.screen().sounds, [], "not left to pile up");
        let redraw = WindowEvent::Redraw {
            elapsed: Duration::from_millis(16),
        };
        app.handle(redraw, &mut window, &mut gpu);
        assert_eq!(app.screen().sounds, []);
    }

    /// Clicks a key on each key press, holding an action for the click;
    /// when told the click has ended, runs it: counts it, notes what the
    /// audio had been asked for by then, and sounds a jump.
    struct KeyedScreen {
        held: bool,
        ran: usize,
        logged_when_run: Vec<AudioCommand>,
        log: AudioLog,
        sounds: Vec<Sound>,
    }

    impl Screen for KeyedScreen {
        fn input(&mut self, input: &Input) -> ScreenAction {
            if let Input::Key { pressed: true, .. } = input {
                self.held = true;
                self.sounds.push(Sound::Ui(UiSound::ButtonKeyed));
            }
            ScreenAction::None
        }

        fn tick(&mut self, _dt: Duration) {}

        fn draw(&self, _list: &mut DrawList) {}

        fn keyed_click_ended(&mut self) {
            if std::mem::take(&mut self.held) {
                self.ran += 1;
                self.logged_when_run = self.log.borrow().clone();
                self.sounds.push(JUMPED);
            }
        }

        fn take_sounds(&mut self) -> Vec<Sound> {
            std::mem::take(&mut self.sounds)
        }

        fn now_showing(&self) -> Option<Showing> {
            Some(Showing::ShipBrowser)
        }
    }

    /// An app over a [`KeyedScreen`], its audio recorded, and a handle on
    /// the audio to say what is playing.
    fn keyed(window: &FakeWindow) -> (App<NoImages, KeyedScreen>, AudioLog, RecordingAudio) {
        let audio = RecordingAudio::new();
        let log = audio.log();
        let handle = audio.clone();
        let screen = KeyedScreen {
            held: false,
            ran: 0,
            logged_when_run: Vec::new(),
            log: log.clone(),
            sounds: Vec::new(),
        };
        let core = AudioCore::new(Box::new(audio) as Box<dyn Audio>);
        (
            App::new(window, NoImages, screen).with_audio(core),
            log,
            handle,
        )
    }

    #[test]
    fn a_keyed_click_is_ended_only_at_a_feed_after_600_has_stopped() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let (mut app, log, audio) = keyed(&window);
        let mut gpu = RecordingGpu::new();
        audio.set_playing(cue_of(DOWN).sound, true);
        app.handle(key(Key::Up, true), &mut window, &mut gpu);
        assert_eq!(*log.borrow(), [cued(DOWN)]);
        assert_eq!(app.screen().ran, 0, "600 playing");
        let redraw = |ms| WindowEvent::Redraw {
            elapsed: Duration::from_millis(ms),
        };
        app.handle(redraw(16), &mut window, &mut gpu);
        app.handle(key(Key::Up, false), &mut window, &mut gpu);
        assert_eq!(app.screen().ran, 0, "still playing");
        audio.set_playing(cue_of(DOWN).sound, false);
        app.handle(redraw(32), &mut window, &mut gpu);
        assert_eq!(app.screen().ran, 1, "600 has stopped");
        assert_eq!(
            app.screen().logged_when_run,
            [cued(DOWN), cued(UP)],
            "after 601 has started"
        );
        assert_eq!(
            *log.borrow(),
            [cued(DOWN), cued(UP), cued(JUMPED)],
            "what it ran sounded in the same pass"
        );
        app.handle(redraw(48), &mut window, &mut gpu);
        assert_eq!(app.screen().ran, 1, "once");
    }

    #[test]
    fn with_sound_off_or_no_audio_a_keyed_click_is_ended_in_its_own_input() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut gpu = RecordingGpu::new();
        let (mut app, log, audio) = keyed(&window);
        audio.set_playing(cue_of(DOWN).sound, true);
        app.audio.as_mut().expect("audio").set_sound(false);
        app.handle(key(Key::Up, true), &mut window, &mut gpu);
        assert_eq!(app.screen().ran, 1, "sound off");
        assert_eq!(*log.borrow(), [AudioCommand::StopEffects], "nothing played");

        let screen = KeyedScreen {
            held: false,
            ran: 0,
            logged_when_run: Vec::new(),
            log: AudioLog::default(),
            sounds: Vec::new(),
        };
        let mut app = App::new(&window, NoImages, screen);
        app.handle(key(Key::Up, true), &mut window, &mut gpu);
        assert_eq!(app.screen().ran, 1, "no audio");
        assert_eq!(app.screen().sounds, [], "the sounds let go");
    }

    #[test]
    fn a_quit_asked_on_a_tick_exits_before_the_frame_is_drawn() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        app.screen.asks_quit = true;
        let redraw = WindowEvent::Redraw {
            elapsed: Duration::from_millis(16),
        };
        assert_eq!(handle(&mut app, &mut window, redraw), Control::Exit);
        assert_eq!(app.screen.quits, 1, "told the app quits");
        assert_eq!(app.screen.ticks.len(), 1, "after its tick");
        assert!(app.screen.drawn_after.borrow().is_empty(), "not drawn");
        assert_eq!(app.take_warnings(), ["quit 1"]);
    }

    #[test]
    fn a_quit_asked_after_an_input_exits() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        app.screen.asks_quit = true;
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Up, true)),
            Control::Exit
        );
        assert_eq!(app.screen.quits, 1);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Up, false)),
            Control::Continue,
            "asked once"
        );
    }

    // Settings.

    /// Reports `prefs` once, after the next input.
    struct PrefsScreen {
        prefs: Vec<Prefs>,
        pending: Option<Prefs>,
    }

    impl Screen for PrefsScreen {
        fn input(&mut self, _input: &Input) -> ScreenAction {
            if !self.prefs.is_empty() {
                self.pending = Some(self.prefs.remove(0));
            }
            ScreenAction::None
        }

        fn tick(&mut self, _dt: Duration) {}

        fn draw(&self, _list: &mut DrawList) {}

        fn take_prefs(&mut self) -> Option<Prefs> {
            self.pending.take()
        }

        fn now_showing(&self) -> Option<Showing> {
            Some(Showing::Preferences)
        }
    }

    fn prefs_app(window: &FakeWindow, prefs: &[Prefs]) -> App<NoImages, PrefsScreen> {
        let screen = PrefsScreen {
            prefs: prefs.to_vec(),
            pending: None,
        };
        App::new(window, NoImages, screen)
    }

    fn keeper(store: &MemorySettings) -> GameKeeper {
        let (keeper, warning) = crate::settings::SettingsKeeper::open(
            Box::new(store.clone()) as Box<dyn SettingsStore>
        );
        assert_eq!(warning, None);
        keeper
    }

    const MUSIC_OFF: Prefs = Prefs {
        sound: SoundPrefs {
            sound: true,
            music: false,
            effects_level: 7,
            music_level: 7,
        },
        hyperspace_effects: true,
        quicktime_movies: true,
        smoke_trails: true,
        weapon_effects: true,
        ship_animations: true,
        engine_glows: true,
        running_lights: true,
        parallax_starfield: true,
        fullscreen: nova_view::FullscreenKind::Borderless,
        scaling: nova_view::Scaling::Native,
    };

    /// `MUSIC_OFF` with the sound effects at `level`.
    fn effects_at(level: u8) -> Prefs {
        Prefs {
            sound: SoundPrefs {
                effects_level: level,
                ..MUSIC_OFF.sound
            },
            ..MUSIC_OFF
        }
    }

    #[test]
    fn a_change_of_prefs_plays_and_is_saved() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let audio = RecordingAudio::new();
        let log = audio.log();
        let core = AudioCore::new(Box::new(audio) as Box<dyn Audio>);
        let store = MemorySettings::new();
        let quieter = effects_at(4);
        let mut app = prefs_app(&window, &[MUSIC_OFF, quieter])
            .with_audio(core)
            .with_settings(keeper(&store));
        let mut gpu = RecordingGpu::new();
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        assert_eq!(*log.borrow(), [], "music was not playing");
        let (saved, _) = crate::settings::SettingsKeeper::open(store.clone());
        assert!(!saved.settings().audio.music);
        assert_eq!(store.writes(), 1);
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        assert_eq!(store.writes(), 2);
        let (saved, _) = crate::settings::SettingsKeeper::open(store.clone());
        assert_eq!(
            saved.settings().audio.effects_volume,
            nova_audio::settings::effects_level_volume(4)
        );
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        assert_eq!(store.writes(), 2, "no change, no save");
        assert_eq!(app.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn a_hyperspace_change_is_saved_with_the_audio_settings() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let audio = RecordingAudio::new();
        let log = audio.log();
        let core = AudioCore::new(Box::new(audio) as Box<dyn Audio>);
        let store = MemorySettings::new();
        let mut app = prefs_app(&window, &[MUSIC_OFF, effects_at(4)])
            .with_audio(core)
            .with_settings(keeper(&store));
        let mut gpu = RecordingGpu::new();
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        log.borrow_mut().clear();
        let no_effects = Prefs {
            hyperspace_effects: false,
            quicktime_movies: true,
            ..effects_at(4)
        };
        app.screen.prefs = vec![no_effects];
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        assert_eq!(*log.borrow(), [], "the audio is unchanged");
        assert_eq!(store.writes(), 3);
        let (saved, _) = crate::settings::SettingsKeeper::open(store.clone());
        assert_eq!(
            saved.settings(),
            GameSettings {
                audio: AudioSettings::default().with_prefs(effects_at(4).sound),
                hyperspace_effects: false,
                fullscreen: false,
                fullscreen_kind: crate::settings::FullscreenSetting::Borderless,
                quicktime_movies: true,
                map_borders: true,
                smoke_trails: true,
                weapon_effects: true,
                ship_animations: true,
                engine_glows: true,
                running_lights: true,
                parallax_starfield: true,
                frame_upscaler: crate::settings::ScalingSetting::default(),
                nova_files: None,
            }
        );
        let saved: serde_json::Value =
            serde_json::from_str(&store.text().expect("saved")).expect("JSON");
        assert_eq!(saved["hyperspace_effects"], false);
        assert_eq!(saved["music"], false);
    }

    /// With no keeper: the change still plays.
    #[test]
    fn a_change_reaches_the_audio_core_and_what_it_plays() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let audio = RecordingAudio::new();
        let log = audio.log();
        let core = AudioCore::new(Box::new(audio) as Box<dyn Audio>);
        let screen = SoundingScreen {
            showing: Showing::MainMenu,
            sounds: Vec::new(),
        };
        let mut app = App::new(
            &window,
            NoImages,
            PrefsAndSounds {
                screen,
                prefs: Some(MUSIC_OFF),
            },
        )
        .with_audio(core);
        log.borrow_mut().clear();
        let mut gpu = RecordingGpu::new();
        app.handle(key(Key::Up, true), &mut window, &mut gpu);
        assert_eq!(
            *log.borrow(),
            [
                AudioCommand::SetMusicVolume(nova_audio::settings::music_level_volume(7)),
                AudioCommand::StopMusic,
                AudioCommand::Play {
                    sound: cue_of(DOWN).sound,
                    volume: nova_audio::settings::effects_level_volume(7),
                    priority: nova_audio::Priority::new(cue_of(DOWN).priority),
                },
            ],
            "the change, then the sounds"
        );
    }

    /// With no keeper, the change starts from the core's own settings: a
    /// volume between two levels, at the level chosen, stays as it is.
    #[test]
    fn without_a_keeper_the_change_starts_from_the_cores_settings() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let audio = RecordingAudio::new();
        let log = audio.log();
        let between = Volume::new(0.26);
        let core = AudioCore::new(Box::new(audio) as Box<dyn Audio>).with_settings(AudioSettings {
            effects_volume: between,
            ..AudioSettings::default()
        });
        let screen = SoundingScreen {
            showing: Showing::MainMenu,
            sounds: Vec::new(),
        };
        let mut app = App::new(
            &window,
            NoImages,
            PrefsAndSounds {
                screen,
                prefs: Some(effects_at(4)),
            },
        )
        .with_audio(core);
        log.borrow_mut().clear();
        app.handle(key(Key::Up, true), &mut window, &mut RecordingGpu::new());
        assert_eq!(
            *log.borrow(),
            [
                AudioCommand::SetMusicVolume(nova_audio::settings::music_level_volume(7)),
                AudioCommand::StopMusic,
                AudioCommand::Play {
                    sound: cue_of(DOWN).sound,
                    volume: between,
                    priority: nova_audio::Priority::new(cue_of(DOWN).priority),
                },
            ],
            "effects level 4 already: the effects volume is kept"
        );
    }

    /// A sounding screen that also reports `prefs` once, after an input.
    struct PrefsAndSounds {
        screen: SoundingScreen,
        prefs: Option<Prefs>,
    }

    impl Screen for PrefsAndSounds {
        fn input(&mut self, input: &Input) -> ScreenAction {
            self.screen.input(input)
        }

        fn tick(&mut self, dt: Duration) {
            self.screen.tick(dt);
        }

        fn draw(&self, list: &mut DrawList) {
            self.screen.draw(list);
        }

        fn take_sounds(&mut self) -> Vec<Sound> {
            self.screen.take_sounds()
        }

        fn take_prefs(&mut self) -> Option<Prefs> {
            if self.screen.sounds.is_empty() {
                None
            } else {
                self.prefs.take()
            }
        }

        fn now_showing(&self) -> Option<Showing> {
            self.screen.now_showing()
        }
    }

    #[test]
    fn without_audio_the_settings_still_save() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::new();
        let mut app = prefs_app(&window, &[MUSIC_OFF]).with_settings(keeper(&store));
        app.handle(key(Key::Space, true), &mut window, &mut RecordingGpu::new());
        let (saved, _) = crate::settings::SettingsKeeper::open(store);
        assert!(!saved.settings().audio.music);
    }

    #[test]
    fn each_failed_save_is_one_warning_until_taken() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::new();
        let quieter = effects_at(1);
        let mut app = prefs_app(&window, &[MUSIC_OFF, quieter, Prefs::default()])
            .with_settings(keeper(&store));
        store.fail_writes(true);
        let mut gpu = RecordingGpu::new();
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        let message = "nova: cannot save the settings: the disk is full".to_owned();
        assert_eq!(app.take_warnings(), [message.clone(), message]);
        assert_eq!(app.take_warnings(), Vec::<String>::new(), "taken");
        store.fail_writes(false);
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        assert_eq!(app.take_warnings(), Vec::<String>::new(), "saved");
        assert_eq!(store.writes(), 1);
    }

    /// Reports a borders save and a change of prefs, each once, as a test
    /// sets them.
    struct BordersScreen {
        save: Option<bool>,
        prefs: Option<Prefs>,
    }

    impl Screen for BordersScreen {
        fn input(&mut self, _input: &Input) -> ScreenAction {
            ScreenAction::None
        }

        fn tick(&mut self, _dt: Duration) {}

        fn draw(&self, _list: &mut DrawList) {}

        fn take_prefs(&mut self) -> Option<Prefs> {
            self.prefs.take()
        }

        fn take_map_borders_save(&mut self) -> Option<bool> {
            self.save.take()
        }
    }

    fn borders_app(window: &FakeWindow, store: &MemorySettings) -> App<NoImages, BordersScreen> {
        let screen = BordersScreen {
            save: None,
            prefs: None,
        };
        App::new(window, NoImages, screen).with_settings(keeper(store))
    }

    fn saved_json(store: &MemorySettings) -> serde_json::Value {
        serde_json::from_str(&store.text().expect("saved")).expect("JSON")
    }

    #[test]
    fn a_borders_save_is_written_with_the_other_settings_kept() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::holding(r#"{"music": false}"#);
        let mut app = borders_app(&window, &store);
        let mut gpu = RecordingGpu::new();
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        assert_eq!(store.writes(), 0, "none reported, none written");
        app.screen.save = Some(false);
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        assert_eq!(store.writes(), 1);
        assert_eq!(saved_json(&store)["show_map_borders"], false);
        assert_eq!(saved_json(&store)["music"], false, "kept");
        app.screen.prefs = Some(effects_at(4));
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        assert_eq!(store.writes(), 2);
        assert_eq!(
            saved_json(&store)["show_map_borders"],
            false,
            "a later Preferences change keeps it"
        );
        assert_eq!(app.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn a_failed_borders_save_is_a_warning() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::new();
        let mut app = borders_app(&window, &store);
        store.fail_writes(true);
        app.screen.save = Some(false);
        app.handle(key(Key::Space, true), &mut window, &mut RecordingGpu::new());
        assert_eq!(
            app.take_warnings(),
            ["nova: cannot save the settings: the disk is full".to_owned()]
        );
    }

    #[test]
    fn a_change_reported_on_a_redraw_is_taken_too() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::new();
        let mut app = prefs_app(&window, &[]).with_settings(keeper(&store));
        app.screen.pending = Some(MUSIC_OFF);
        app.handle(
            WindowEvent::Redraw {
                elapsed: Duration::from_millis(16),
            },
            &mut window,
            &mut RecordingGpu::new(),
        );
        assert_eq!(store.writes(), 1);
    }

    // Fullscreen.

    use crate::config::Os;

    /// The Windows shortcut, Alt+Enter.
    const SHORTCUT: [Key; 2] = [Key::Alt, Key::Enter];

    /// macOS's Control+Command+F, which its View menu's key equivalent
    /// takes before the window.
    const MAC_FULL_SCREEN: [Key; 3] = [Key::Control, Key::Command, Key::Char('f')];

    const BORDERLESS: Option<FullscreenKind> = Some(FullscreenKind::Borderless);

    const NOT_FULLSCREEN: Vec<Option<FullscreenKind>> = Vec::new();

    /// Presses `chord`'s keys in order, then releases them in reverse.
    fn chord<C: Screen>(app: &mut App<NoImages, C>, window: &mut FakeWindow, chord: &[Key]) {
        let mut gpu = RecordingGpu::new();
        for key in chord {
            app.handle_routed(self::key(*key, true), window, &mut gpu);
        }
        for key in chord.iter().rev() {
            app.handle_routed(self::key(*key, false), window, &mut gpu);
        }
    }

    fn reported(fullscreen: bool) -> WindowEvent {
        WindowEvent::FullscreenChanged { fullscreen }
    }

    fn saved_fullscreen(store: &MemorySettings) -> bool {
        let (saved, _) = crate::settings::SettingsKeeper::open(store.clone());
        saved.settings().fullscreen
    }

    fn key_input(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    #[test]
    fn a_window_that_cannot_go_fullscreen_says_it_is_not() {
        assert!(!FakeWindow::new((1024, 768), 1.0).is_fullscreen());
    }

    #[test]
    fn a_window_with_no_monitor_of_its_own_lists_no_modes_and_no_name() {
        struct Bare;
        impl WindowPort for Bare {
            fn size_px(&self) -> (u32, u32) {
                (1024, 768)
            }
            fn scale_factor(&self) -> f64 {
                1.0
            }
            fn request_redraw(&mut self) {}
        }
        assert_eq!(Bare.video_modes(), []);
        assert_eq!(Bare.monitor_name(), None);
    }

    #[test]
    fn the_shortcut_goes_fullscreen_and_never_reaches_the_screen() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window).with_os(Os::Windows);
        let mut gpu = RecordingGpu::new();
        for key in SHORTCUT {
            let handled = app.handle_routed(self::key(key, true), &mut window, &mut gpu);
            assert_eq!(handled, handled_game(), "{key:?}");
        }
        handle(&mut app, &mut window, WindowEvent::Text('\r'));
        handle(&mut app, &mut window, key(Key::Enter, false));
        assert_eq!(window.fullscreen, [BORDERLESS]);
        assert_eq!(
            app.screen().inputs,
            [key_input(Key::Alt, true)],
            "the modifier only"
        );
        handle(&mut app, &mut window, key(Key::Alt, false));
        handle(&mut app, &mut window, WindowEvent::Text('g'));
        assert_eq!(
            app.screen().inputs[1..],
            [key_input(Key::Alt, false), Input::Text('g')],
            "later text arrives"
        );
    }

    #[test]
    fn on_macos_control_command_f_is_the_games_keys_and_toggles_nothing() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window).with_os(Os::MacOs);
        let mut gpu = RecordingGpu::new();
        for key in MAC_FULL_SCREEN {
            app.handle_routed(self::key(key, true), &mut window, &mut gpu);
        }
        handle(&mut app, &mut window, WindowEvent::Text('f'));
        for key in MAC_FULL_SCREEN.iter().rev() {
            app.handle_routed(self::key(*key, false), &mut window, &mut gpu);
        }
        assert_eq!(window.fullscreen, NOT_FULLSCREEN, "the View menu's to take");
        assert_eq!(
            app.screen().inputs,
            [
                key_input(Key::Control, true),
                key_input(Key::Command, true),
                key_input(Key::Char('f'), true),
                Input::Text('f'),
                key_input(Key::Char('f'), false),
                key_input(Key::Command, false),
                key_input(Key::Control, false),
            ]
        );
    }

    #[test]
    fn on_macos_a_system_toggle_is_saved_each_way_and_gives_back_the_windows_size() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::new();
        let mut app = app(&window)
            .with_os(Os::MacOs)
            .with_settings(keeper(&store));
        let resized = |size_px| WindowEvent::Resized {
            size_px,
            scale_factor: 2.0,
        };
        handle(&mut app, &mut window, resized((1280, 960)));
        // The View menu's item, Globe+F and the green button all arrive
        // as the system's report.
        handle(&mut app, &mut window, reported(true));
        assert!(saved_fullscreen(&store));
        handle(&mut app, &mut window, resized((3024, 1964)));
        handle(&mut app, &mut window, reported(false));
        assert!(!saved_fullscreen(&store));
        assert_eq!(store.writes(), 2);
        assert_eq!(window.size_requests, [(1280, 960)]);
        assert_eq!(window.fullscreen, NOT_FULLSCREEN, "the system did it");
    }

    fn handled_game() -> Handled {
        handled(Control::Continue, Routing::Game)
    }

    #[test]
    fn alt_enter_and_f11_toggle_on_windows() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window).with_os(Os::Windows);
        chord(&mut app, &mut window, &[Key::Alt, Key::Enter]);
        chord(&mut app, &mut window, &[Key::Function(11)]);
        assert_eq!(window.fullscreen, [BORDERLESS, None]);
        assert_eq!(
            app.screen().inputs,
            [key_input(Key::Alt, true), key_input(Key::Alt, false)]
        );
    }

    #[test]
    fn the_shortcut_saves_each_change_and_gives_back_the_old_size() {
        let mut window = FakeWindow::new((2048, 1536), 2.0);
        let store = MemorySettings::new();
        let mut app = app(&window)
            .with_os(Os::Windows)
            .with_settings(keeper(&store));
        chord(&mut app, &mut window, &SHORTCUT);
        assert!(saved_fullscreen(&store));
        assert_eq!(store.writes(), 1);
        window.size_px = (3024, 1964);
        chord(&mut app, &mut window, &SHORTCUT);
        assert!(!saved_fullscreen(&store));
        assert_eq!(store.writes(), 2);
        assert_eq!(window.fullscreen, [BORDERLESS, None]);
        assert_eq!(window.size_requests, [(2048, 1536)]);
        assert_eq!(app.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn the_systems_changes_are_saved_and_a_repeated_report_does_nothing() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::new();
        let mut app = app(&window).with_settings(keeper(&store));
        assert_eq!(
            app.handle_routed(reported(false), &mut window, &mut RecordingGpu::new()),
            handled_game()
        );
        assert_eq!(store.writes(), 0, "windowed already");
        handle(&mut app, &mut window, reported(true));
        assert!(saved_fullscreen(&store));
        handle(&mut app, &mut window, reported(true));
        assert_eq!(store.writes(), 1, "no change, no save");
        handle(&mut app, &mut window, reported(false));
        assert!(!saved_fullscreen(&store));
        assert_eq!(window.size_requests, [(1024, 768)]);
        assert_eq!(window.fullscreen, NOT_FULLSCREEN, "the system did it");
        for _ in 0..3 {
            handle(&mut app, &mut window, reported(false));
        }
        assert_eq!(store.writes(), 2);
        assert_eq!(window.size_requests.len(), 1);
        assert_eq!(app.screen().inputs, [], "the screen hears nothing of it");
    }

    #[test]
    fn a_report_on_every_frame_saves_nothing_and_asks_nothing_until_it_changes() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::new();
        let mut app = app(&window)
            .with_os(Os::Windows)
            .with_settings(keeper(&store));
        chord(&mut app, &mut window, &SHORTCUT);
        for frame in 1..=5 {
            handle(&mut app, &mut window, reported(true));
            let elapsed = Duration::from_millis(16 * frame);
            handle(&mut app, &mut window, WindowEvent::Redraw { elapsed });
        }
        assert_eq!(store.writes(), 1);
        assert_eq!(window.fullscreen, [BORDERLESS]);
        assert_eq!(window.size_requests, Vec::<(u32, u32)>::new());
        handle(&mut app, &mut window, reported(false));
        assert_eq!(store.writes(), 2, "the system's exit, seen on a frame");
        assert_eq!(window.size_requests, [(1024, 768)]);
    }

    #[test]
    fn the_size_given_back_is_the_windows_not_the_displays() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        let resized = |size_px| WindowEvent::Resized {
            size_px,
            scale_factor: 2.0,
        };
        handle(&mut app, &mut window, resized((1600, 1200)));
        handle(&mut app, &mut window, reported(true));
        handle(&mut app, &mut window, resized((3024, 1964)));
        assert_eq!(app.viewport().window_px(), (3024, 1964), "drawn fullscreen");
        handle(&mut app, &mut window, reported(false));
        assert_eq!(window.size_requests, [(1600, 1200)]);
    }

    #[test]
    fn the_shortcut_works_with_the_overlay_showing() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window).with_os(Os::Windows);
        handle(
            &mut app,
            &mut window,
            key(nova_view::devtools::TOGGLE_KEY, true),
        );
        assert!(visible(&app));
        chord(&mut app, &mut window, &[Key::Function(11)]);
        assert_eq!(window.fullscreen, [BORDERLESS]);
    }

    #[test]
    fn losing_focus_forgets_the_shortcuts_modifiers() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window).with_os(Os::Windows);
        handle(&mut app, &mut window, key(Key::Alt, true));
        handle(&mut app, &mut window, WindowEvent::FocusLost);
        handle(&mut app, &mut window, key(Key::Enter, true));
        assert_eq!(window.fullscreen, NOT_FULLSCREEN);
        assert_eq!(app.screen().releases, 1);
    }

    #[test]
    fn without_a_keeper_the_shortcut_still_toggles_and_a_failed_save_is_a_warning() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window).with_os(Os::Windows);
        chord(&mut app, &mut window, &[Key::Function(11)]);
        assert_eq!(window.fullscreen, [BORDERLESS]);
        assert_eq!(app.take_warnings(), Vec::<String>::new());
        let store = MemorySettings::new();
        let mut app = self::app(&window)
            .with_os(Os::Windows)
            .with_settings(keeper(&store));
        store.fail_writes(true);
        chord(&mut app, &mut window, &[Key::Function(11)]);
        handle(&mut app, &mut window, reported(false));
        let message = "nova: cannot save the settings: the disk is full".to_owned();
        assert_eq!(app.take_warnings(), [message.clone(), message]);
    }

    #[test]
    fn restoring_the_window_goes_fullscreen_only_when_saved_so() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::holding(r#"{"fullscreen": true}"#);
        let mut app = app(&window).with_settings(keeper(&store));
        app.restore_window(&mut window);
        assert_eq!(window.fullscreen, [BORDERLESS]);
        assert_eq!(store.writes(), 0, "already saved so");
        handle(&mut app, &mut window, reported(true));
        assert_eq!(window.fullscreen, [BORDERLESS], "once");
        assert_eq!(store.writes(), 0);
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::holding(r#"{"fullscreen": false}"#);
        let mut app = self::app(&window).with_settings(keeper(&store));
        app.restore_window(&mut window);
        let mut unkept = self::app(&window);
        unkept.restore_window(&mut window);
        assert_eq!(window.fullscreen, NOT_FULLSCREEN);
    }

    #[test]
    fn choosing_preferences_while_fullscreen_keeps_it_saved() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::new();
        let mut app = prefs_app(&window, &[MUSIC_OFF]).with_settings(keeper(&store));
        handle_prefs(&mut app, &mut window, reported(true));
        handle_prefs(&mut app, &mut window, key(Key::Space, true));
        let (saved, _) = crate::settings::SettingsKeeper::open(store.clone());
        assert!(!saved.settings().audio.music, "the preference saved");
        assert!(saved.settings().fullscreen, "and fullscreen kept");
    }

    fn handle_prefs(
        app: &mut App<NoImages, PrefsScreen>,
        window: &mut FakeWindow,
        event: WindowEvent,
    ) {
        app.handle(event, window, &mut RecordingGpu::new());
    }

    // Exclusive display modes.

    use nova_view::VideoMode;

    const DELL: &str = "DELL U2720Q";

    fn fhd() -> VideoMode {
        VideoMode {
            width: 1920,
            height: 1080,
            refresh_millihertz: 60_000,
        }
    }

    fn qhd() -> VideoMode {
        VideoMode {
            width: 2560,
            height: 1440,
            refresh_millihertz: 144_000,
        }
    }

    #[allow(clippy::unnecessary_wraps)] // As the window records a request.
    fn exclusive(mode: VideoMode) -> Option<FullscreenKind> {
        Some(FullscreenKind::Exclusive(mode))
    }

    /// A 1024x768 window on the DELL, which offers QHD at 144 Hz and FHD
    /// at 60 Hz, each at two depths, and modes too small to list.
    fn on_monitor() -> FakeWindow {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        window.monitor = Some(DELL.to_owned());
        let small = VideoMode {
            width: 800,
            height: 600,
            refresh_millihertz: 60_000,
        };
        window.modes = [fhd(), small, qhd(), fhd(), qhd()]
            .into_iter()
            .zip([32, 32, 32, 16, 24])
            .map(|(mode, bit_depth)| MonitorMode { mode, bit_depth })
            .collect();
        window
    }

    /// Settings holding FHD on `monitor` as the kind, and `fullscreen`.
    fn saved_fhd(fullscreen: bool, monitor: &str) -> MemorySettings {
        MemorySettings::holding(
            &serde_json::json!({
                "fullscreen": fullscreen,
                "fullscreen_kind": {"exclusive": {
                    "width": 1920,
                    "height": 1080,
                    "refresh_millihertz": 60_000,
                    "monitor": monitor,
                }},
            })
            .to_string(),
        )
    }

    fn stored(store: &MemorySettings) -> GameSettings {
        crate::settings::SettingsKeeper::open(store.clone())
            .0
            .settings()
    }

    fn fhd_on(monitor: &str) -> FullscreenSetting {
        FullscreenSetting::Exclusive {
            mode: fhd(),
            monitor: Some(monitor.to_owned()),
        }
    }

    #[test]
    fn a_saved_exclusive_mode_the_monitor_offers_reopens_and_saves_nothing() {
        let mut window = on_monitor();
        let store = saved_fhd(true, DELL);
        let mut app = app(&window).with_settings(keeper(&store));
        app.restore_window(&mut window);
        assert_eq!(window.fullscreen, [exclusive(fhd())]);
        assert_eq!(store.writes(), 0);
        assert_eq!(app.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn a_saved_mode_the_monitor_lacks_opens_borderless_with_a_warning_naming_it() {
        for (window, monitor) in [
            (FakeWindow::new((1024, 768), 1.0), DELL),
            (on_monitor(), "LG HDR 4K"),
        ] {
            let mut window = window;
            let store = saved_fhd(true, monitor);
            let mut app = app(&window).with_settings(keeper(&store));
            app.restore_window(&mut window);
            assert_eq!(window.fullscreen, [BORDERLESS], "{monitor}");
            let warnings = app.take_warnings();
            assert_eq!(warnings.len(), 1, "{monitor}");
            assert!(
                warnings[0].contains("1920 x 1080, 60 Hz") && warnings[0].contains(monitor),
                "{warnings:?}"
            );
            assert_eq!(store.writes(), 0, "the kind stays saved");
            assert_eq!(stored(&store).fullscreen_kind, fhd_on(monitor));
        }
    }

    #[test]
    fn after_a_windowed_launch_the_shortcut_enters_the_saved_mode_and_saves_only_the_state() {
        let mut window = on_monitor();
        let store = saved_fhd(false, DELL);
        let mut app = app(&window)
            .with_os(Os::Windows)
            .with_settings(keeper(&store));
        app.restore_window(&mut window);
        assert_eq!(window.fullscreen, NOT_FULLSCREEN);
        chord(&mut app, &mut window, &[Key::Function(11)]);
        assert_eq!(window.fullscreen, [exclusive(fhd())]);
        assert!(stored(&store).fullscreen);
        chord(&mut app, &mut window, &[Key::Function(11)]);
        assert_eq!(window.fullscreen, [exclusive(fhd()), None]);
        assert_eq!(window.size_requests, [(1024, 768)]);
        let saved = stored(&store);
        assert!(!saved.fullscreen);
        assert_eq!(saved.fullscreen_kind, fhd_on(DELL));
        assert_eq!(store.writes(), 2);
    }

    /// What the dialog reports on a close: the prefs, and the kind
    /// picked, if any.
    type Report = (Prefs, Option<FullscreenKind>);

    /// An offer of the display modes: the modes, the kind the game runs
    /// in, and the kind chosen.
    type Offer = (Vec<VideoMode>, FullscreenKind, FullscreenKind);

    /// Opens Preferences on P, closes it on Escape, and reports the next
    /// of `reports` on Space; records each offer of the display modes.
    #[derive(Default)]
    struct DisplayScreen {
        open: bool,
        reports: Vec<Report>,
        pending: Option<Report>,
        offers: Vec<Offer>,
        /// Opens Preferences on the next tick, as a keyed click does.
        opens_on_tick: bool,
        /// How many offers had arrived at each draw.
        drawn_after: std::cell::RefCell<Vec<usize>>,
    }

    impl Screen for DisplayScreen {
        fn input(&mut self, input: &Input) -> ScreenAction {
            match *input {
                Input::Key {
                    key: Key::Char('p'),
                    pressed: true,
                    ..
                } => self.open = true,
                Input::Key {
                    key: Key::Escape,
                    pressed: true,
                    ..
                } => self.open = false,
                Input::Key {
                    key: Key::Space,
                    pressed: true,
                    ..
                } if !self.reports.is_empty() => self.pending = Some(self.reports.remove(0)),
                _ => {}
            }
            ScreenAction::None
        }

        fn tick(&mut self, _dt: Duration) {
            self.open |= std::mem::take(&mut self.opens_on_tick);
        }

        fn draw(&self, _list: &mut DrawList) {
            self.drawn_after.borrow_mut().push(self.offers.len());
        }

        fn take_prefs(&mut self) -> Option<Prefs> {
            self.pending.map(|(prefs, _)| prefs)
        }

        fn take_display_choice(&mut self) -> Option<FullscreenKind> {
            self.pending.take().and_then(|(_, choice)| choice)
        }

        fn now_showing(&self) -> Option<Showing> {
            Some(if self.open {
                Showing::Preferences
            } else {
                Showing::Flight
            })
        }

        fn offer_display(
            &mut self,
            modes: &[VideoMode],
            current: FullscreenKind,
            chosen: FullscreenKind,
        ) {
            self.offers.push((modes.to_vec(), current, chosen));
        }
    }

    type DisplayApp = App<NoImages, DisplayScreen>;

    fn display_app(window: &FakeWindow, reports: &[Report]) -> DisplayApp {
        let screen = DisplayScreen {
            reports: reports.to_vec(),
            ..DisplayScreen::default()
        };
        App::new(window, NoImages, screen).with_os(Os::Windows)
    }

    fn press_on(app: &mut DisplayApp, window: &mut FakeWindow, key: Key) {
        let mut gpu = RecordingGpu::new();
        for pressed in [true, false] {
            app.handle(self::key(key, pressed), window, &mut gpu);
        }
    }

    #[test]
    fn opening_preferences_offers_the_listed_modes_and_the_live_kind_once_each_time() {
        let mut window = on_monitor();
        let mut app = display_app(&window, &[]);
        handle_display(&mut app, &mut window, WindowEvent::FocusGained);
        assert_eq!(app.screen().offers, [], "not open");
        press_on(&mut app, &mut window, Key::Char('p'));
        let listed = vec![qhd(), fhd()];
        assert_eq!(
            app.screen().offers,
            [(
                listed.clone(),
                FullscreenKind::Borderless,
                FullscreenKind::Borderless
            )]
        );
        press_on(&mut app, &mut window, Key::Up);
        handle_display(
            &mut app,
            &mut window,
            WindowEvent::Redraw {
                elapsed: Duration::from_millis(16),
            },
        );
        assert_eq!(app.screen().offers.len(), 1, "not again while open");
        press_on(&mut app, &mut window, Key::Escape);
        app.fullscreen.set_kind(fhd_on(DELL));
        window.modes.clear();
        press_on(&mut app, &mut window, Key::Char('p'));
        assert_eq!(
            app.screen().offers[1],
            (
                vec![],
                FullscreenKind::Exclusive(fhd()),
                FullscreenKind::Exclusive(fhd())
            ),
            "offered again, with no modes from a monitor listing none"
        );
    }

    #[test]
    fn opening_preferences_on_macos_offers_no_modes_and_borderless() {
        let mut window = on_monitor();
        let mut app = display_app(&window, &[]).with_os(Os::MacOs);
        app.fullscreen.set_kind(fhd_on(DELL));
        press_on(&mut app, &mut window, Key::Char('p'));
        assert_eq!(
            app.screen().offers,
            [(
                vec![],
                FullscreenKind::Borderless,
                FullscreenKind::Borderless
            )]
        );
    }

    #[test]
    fn a_dialog_opened_on_a_tick_is_offered_before_its_first_frame() {
        let mut window = on_monitor();
        let mut app = display_app(&window, &[]);
        app.screen.opens_on_tick = true;
        handle_display(
            &mut app,
            &mut window,
            WindowEvent::Redraw {
                elapsed: Duration::from_millis(16),
            },
        );
        assert_eq!(*app.screen().drawn_after.borrow(), [1]);
    }

    fn handle_display(
        app: &mut DisplayApp,
        window: &mut FakeWindow,
        event: WindowEvent,
    ) -> Control {
        app.handle(event, window, &mut RecordingGpu::new())
    }

    /// The dialog closed on a pick of `kind`, with the music turned off.
    fn choosing(kind: FullscreenKind) -> Report {
        (
            Prefs {
                fullscreen: kind,
                ..MUSIC_OFF
            },
            Some(kind),
        )
    }

    /// The dialog closed with the music turned off and no pick, showing
    /// `shown`.
    fn audio_only(shown: FullscreenKind) -> Report {
        (
            Prefs {
                fullscreen: shown,
                ..MUSIC_OFF
            },
            None,
        )
    }

    #[test]
    fn a_kind_the_dialog_chooses_is_entered_and_saved_with_the_monitors_name() {
        let mut window = on_monitor();
        let store = MemorySettings::new();
        let mut app = display_app(&window, &[choosing(FullscreenKind::Exclusive(qhd()))])
            .with_settings(keeper(&store));
        press_on(&mut app, &mut window, Key::Char('p'));
        press_on(&mut app, &mut window, Key::Space);
        assert_eq!(window.fullscreen, [exclusive(qhd())]);
        let saved = stored(&store);
        assert!(saved.fullscreen);
        assert_eq!(
            saved.fullscreen_kind,
            FullscreenSetting::Exclusive {
                mode: qhd(),
                monitor: Some(DELL.to_owned()),
            }
        );
        assert!(!saved.audio.music, "the rest of the prefs saved too");
        assert_eq!(app.take_warnings(), Vec::<String>::new());
        assert_eq!(*app.fullscreen.kind(), saved.fullscreen_kind, "live");
    }

    #[test]
    fn choosing_borderless_from_an_exclusive_mode_switches_to_it() {
        let mut window = on_monitor();
        let store = saved_fhd(true, DELL);
        let mut app = display_app(&window, &[choosing(FullscreenKind::Borderless)])
            .with_settings(keeper(&store));
        app.restore_window(&mut window);
        press_on(&mut app, &mut window, Key::Space);
        assert_eq!(window.fullscreen, [exclusive(fhd()), BORDERLESS]);
        assert_eq!(window.size_requests, NO_SIZES, "fullscreen to fullscreen");
        assert_eq!(
            stored(&store).fullscreen_kind,
            FullscreenSetting::Borderless
        );
    }

    const NO_SIZES: Vec<(u32, u32)> = Vec::new();

    #[test]
    fn an_audio_change_after_a_fallback_asks_nothing_and_keeps_the_saved_kind() {
        // The monitor lacks the saved mode: the session falls back to
        // borderless. The dialog is offered borderless, as it runs, and
        // the saved kind as chosen, and reports the audio alone.
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = saved_fhd(true, DELL);
        let mut app = display_app(&window, &[audio_only(FullscreenKind::Borderless)])
            .with_settings(keeper(&store));
        app.restore_window(&mut window);
        app.take_warnings();
        press_on(&mut app, &mut window, Key::Char('p'));
        assert_eq!(
            app.screen().offers,
            [(
                vec![],
                FullscreenKind::Borderless,
                FullscreenKind::Exclusive(fhd())
            )],
            "what runs, and the saved kind"
        );
        press_on(&mut app, &mut window, Key::Space);
        assert_eq!(window.fullscreen, [BORDERLESS], "nothing more asked");
        let saved = stored(&store);
        assert!(!saved.audio.music, "the audio saved");
        assert_eq!(saved.fullscreen_kind, fhd_on(DELL), "kept");
        assert_eq!(app.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn prefs_naming_another_kind_without_a_pick_enter_nothing() {
        let mut window = on_monitor();
        let store = MemorySettings::new();
        let exclusive = FullscreenKind::Exclusive(qhd());
        let mut app = display_app(&window, &[audio_only(exclusive)]).with_settings(keeper(&store));
        press_on(&mut app, &mut window, Key::Space);
        assert_eq!(window.fullscreen, NOT_FULLSCREEN);
        assert_eq!(
            stored(&store).fullscreen_kind,
            FullscreenSetting::Borderless
        );
    }

    #[test]
    fn the_saved_mode_picked_after_a_monitor_name_fallback_enters_and_saves_this_monitor() {
        // The DELL lists the mode saved on the LG: the launch falls back,
        // and picking the mode enters it here.
        let mut window = on_monitor();
        let store = saved_fhd(true, "LG HDR 4K");
        let mut app = display_app(&window, &[choosing(FullscreenKind::Exclusive(fhd()))])
            .with_settings(keeper(&store));
        app.restore_window(&mut window);
        assert_eq!(app.take_warnings().len(), 1);
        press_on(&mut app, &mut window, Key::Char('p'));
        let (_, current, chosen) = &app.screen().offers[0];
        assert_eq!(
            (*current, *chosen),
            (FullscreenKind::Borderless, FullscreenKind::Exclusive(fhd())),
            "borderless shown, the saved mode chosen"
        );
        press_on(&mut app, &mut window, Key::Space);
        assert_eq!(window.fullscreen, [BORDERLESS, exclusive(fhd())]);
        let saved = stored(&store);
        assert!(saved.fullscreen);
        assert_eq!(saved.fullscreen_kind, fhd_on(DELL), "this monitor's name");
        assert_eq!(app.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn borderless_picked_after_a_fallback_is_saved_and_the_next_launch_is_quiet() {
        let mut window = on_monitor();
        let store = saved_fhd(true, "LG HDR 4K");
        let mut app = display_app(&window, &[choosing(FullscreenKind::Borderless)])
            .with_settings(keeper(&store));
        app.restore_window(&mut window);
        assert_eq!(app.take_warnings().len(), 1);
        press_on(&mut app, &mut window, Key::Char('p'));
        press_on(&mut app, &mut window, Key::Space);
        let saved = stored(&store);
        assert!(saved.fullscreen);
        assert_eq!(saved.fullscreen_kind, FullscreenSetting::Borderless);
        assert_eq!(*window.fullscreen.last().expect("entered"), BORDERLESS);
        assert_eq!(app.take_warnings(), Vec::<String>::new());
        let mut relaunched = display_app(&window, &[]).with_settings(keeper(&store));
        let mut window = on_monitor();
        relaunched.restore_window(&mut window);
        assert_eq!(window.fullscreen, [BORDERLESS]);
        assert_eq!(relaunched.take_warnings(), Vec::<String>::new(), "quiet");
    }

    #[test]
    fn the_shortcut_into_a_saved_mode_the_monitor_lacks_warns_and_keeps_the_kind() {
        let mut window = on_monitor();
        let store = saved_fhd(false, "LG HDR 4K");
        let mut app = app(&window)
            .with_os(Os::Windows)
            .with_settings(keeper(&store));
        app.restore_window(&mut window);
        chord(&mut app, &mut window, &[Key::Function(11)]);
        assert_eq!(window.fullscreen, [BORDERLESS]);
        let warnings = app.take_warnings();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("1920 x 1080, 60 Hz"), "{warnings:?}");
        let saved = stored(&store);
        assert!(saved.fullscreen);
        assert_eq!(saved.fullscreen_kind, fhd_on("LG HDR 4K"), "kept");
    }

    #[test]
    fn a_chosen_mode_the_monitor_no_longer_lists_enters_borderless_with_a_warning() {
        let mut window = on_monitor();
        let store = MemorySettings::new();
        let mut app = display_app(&window, &[choosing(FullscreenKind::Exclusive(qhd()))])
            .with_settings(keeper(&store));
        press_on(&mut app, &mut window, Key::Char('p'));
        window.modes.clear();
        press_on(&mut app, &mut window, Key::Space);
        assert_eq!(window.fullscreen, [BORDERLESS]);
        let warnings = app.take_warnings();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("2560 x 1440, 144 Hz"), "{warnings:?}");
    }

    #[test]
    fn quitting_from_an_exclusive_mode_leaves_it_first_and_saves_nothing() {
        type Quit = fn(&mut TestApp, &mut FakeWindow) -> Control;
        let close: Quit = |app, window| handle(app, window, WindowEvent::CloseRequested);
        let screen_quit: Quit = |app, window| {
            app.screen.quit_on = Some(Key::Char('q'));
            handle(app, window, key(Key::Char('q'), true))
        };
        let quit_on_tick: Quit = |app, window| {
            app.screen.asks_quit = true;
            let elapsed = Duration::from_millis(16);
            handle(app, window, WindowEvent::Redraw { elapsed })
        };
        for (name, quit) in [
            ("close", close),
            ("screen", screen_quit),
            ("tick", quit_on_tick),
        ] {
            let mut window = on_monitor();
            let store = saved_fhd(true, DELL);
            let mut app = app(&window).with_settings(keeper(&store));
            app.restore_window(&mut window);
            assert_eq!(quit(&mut app, &mut window), Control::Exit, "{name}");
            assert_eq!(window.fullscreen, [exclusive(fhd()), None], "{name}");
            handle(&mut app, &mut window, reported(false));
            assert_eq!(window.size_requests, NO_SIZES, "{name}");
            assert_eq!(store.writes(), 0, "{name}");
            assert!(stored(&store).fullscreen, "{name}: reopens exclusive");
        }
    }

    #[test]
    fn quitting_windowed_or_borderless_asks_nothing() {
        let mut window = on_monitor();
        let mut app = app(&window).with_os(Os::Windows);
        handle(&mut app, &mut window, WindowEvent::CloseRequested);
        let mut borderless = self::app(&window).with_os(Os::Windows);
        chord(&mut borderless, &mut window, &[Key::Function(11)]);
        handle(&mut borderless, &mut window, WindowEvent::CloseRequested);
        assert_eq!(window.fullscreen, [BORDERLESS]);
    }

    #[test]
    fn losing_focus_in_an_exclusive_mode_leaves_it_until_focus_comes_back() {
        let mut window = on_monitor();
        let store = saved_fhd(true, DELL);
        let mut app = app(&window).with_settings(keeper(&store));
        app.restore_window(&mut window);
        handle(&mut app, &mut window, WindowEvent::FocusLost);
        assert_eq!(window.fullscreen, [exclusive(fhd()), None]);
        handle(&mut app, &mut window, reported(false));
        assert_eq!(window.size_requests, NO_SIZES);
        assert_eq!(store.writes(), 0);
        assert_eq!(
            app.handle_routed(
                WindowEvent::FocusGained,
                &mut window,
                &mut RecordingGpu::new()
            ),
            handled_game()
        );
        assert_eq!(
            window.fullscreen,
            [exclusive(fhd()), None, exclusive(fhd())]
        );
        assert_eq!(app.screen().inputs, [], "the screen hears nothing");
        assert_eq!(app.screen().releases, 1, "but lets go of its keys");
    }

    // The pilot desk.

    use nova_sim::fixture::MemoryPilots;
    use nova_view::devtools::PilotEdit;

    use super::screen::tests::{create, menu, saved};

    /// An app over the main menu keeping pilots in `store`, with a
    /// developer overlay.
    fn desk_app(window: &FakeWindow, store: &MemoryPilots) -> App<NoImages> {
        App::new(window, NoImages, menu(store)).with_dev_overlay()
    }

    #[test]
    fn the_pilot_desk_frame_has_a_desk_only_in_flight() {
        let window = FakeWindow::new((1024, 768), 1.0);
        let store = MemoryPilots::new();
        let mut app = desk_app(&window, &store);
        let mut seen = Vec::new();
        app.pilot_desk_frame(|overlay, desk| {
            seen.push((overlay.visible(), desk.is_some()));
            false
        });
        create(&mut app.screen, "Ada");
        app.pilot_desk_frame(|_, desk| {
            let sheet = desk.expect("flying").sheet().expect("a pilot");
            seen.push((false, sheet.name == "Ada"));
            false
        });
        assert_eq!(seen, [(false, false), (false, true)]);
    }

    #[test]
    fn an_edit_in_the_pilot_desk_frame_is_saved() {
        let window = FakeWindow::new((1024, 768), 1.0);
        let store = MemoryPilots::new();
        let mut app = desk_app(&window, &store);
        create(&mut app.screen, "Ada");
        let writes = store.writes();
        app.pilot_desk_frame(|_, desk| {
            desk.expect("flying")
                .edit(PilotEdit::Credits(321))
                .expect("made");
            true
        });
        assert_eq!(store.writes(), writes + 1);
        assert_eq!(saved(&store, "Ada").cash(), 321);
    }

    #[test]
    fn a_pilot_desk_edit_that_cannot_be_saved_is_a_warning() {
        let window = FakeWindow::new((1024, 768), 1.0);
        let store = MemoryPilots::new();
        let mut app = desk_app(&window, &store);
        create(&mut app.screen, "Ada");
        assert_eq!(app.take_warnings(), Vec::<String>::new(), "created");
        store.fail_writes(true);
        app.pilot_desk_frame(|_, desk| {
            desk.expect("flying")
                .edit(PilotEdit::Credits(321))
                .expect("made");
            true
        });
        assert_eq!(
            app.take_warnings(),
            ["nova: cannot save the pilot Ada in memory (the disk is full)"]
        );
    }

    #[test]
    fn a_pilot_desk_frame_with_no_edit_saves_nothing() {
        let window = FakeWindow::new((1024, 768), 1.0);
        let store = MemoryPilots::new();
        let mut app = desk_app(&window, &store);
        create(&mut app.screen, "Ada");
        let writes = store.writes();
        app.pilot_desk_frame(|_, desk| {
            // An edit the frame does not report waits for the next save.
            desk.expect("flying")
                .edit(PilotEdit::Credits(5))
                .expect("made");
            false
        });
        assert_eq!(store.writes(), writes);
    }

    #[test]
    fn without_an_overlay_the_pilot_desk_frame_does_not_run() {
        let window = FakeWindow::new((1024, 768), 1.0);
        let store = MemoryPilots::new();
        let mut app = App::new(&window, NoImages, menu(&store));
        let mut ran = false;
        app.pilot_desk_frame(|_, _| {
            ran = true;
            true
        });
        assert!(!ran);
    }
}
