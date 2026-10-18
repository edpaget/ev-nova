//! The frame upscaler drawing flight's world: the one the game starts
//! with, from the stored choice and what the device can use, and a switch
//! the Preferences dialog picks.

use nova_render::{Gpu, ImageSource, UpscalerProbe, listed};
use nova_view::{Screen, Upscaler};

use super::App;
use crate::settings::UpscalerSetting;

/// The upscaler to start with from the `stored` choice, given the
/// upscalers the device can use, `listed`, and a warning to show when it
/// is not the one stored: the stored one when it is known and listed, and
/// native otherwise.
pub(super) fn resolve(stored: &UpscalerSetting, listed: &[Upscaler]) -> (Upscaler, Option<String>) {
    let name = stored.name();
    match stored.known() {
        Some(upscaler) if listed.contains(&upscaler) => (upscaler, None),
        Some(_) => (
            Upscaler::Native,
            Some(format!(
                "nova: the frame upscaler \"{name}\" in the settings is not available on \
                 this device; drawing natively"
            )),
        ),
        None => (
            Upscaler::Native,
            Some(format!(
                "nova: the frame upscaler \"{name}\" in the settings is not one this game \
                 has; drawing natively"
            )),
        ),
    }
}

/// The upscaler drawing flight's world now, and a switch to it still to
/// be made on the GPU.
#[derive(Debug, Default)]
pub(super) struct FrameUpscaler {
    running: Upscaler,
    switch: Option<Upscaler>,
}

impl FrameUpscaler {
    /// Starts with `upscaler`, already set on the GPU.
    pub(super) fn start(&mut self, upscaler: Upscaler) {
        self.running = upscaler;
        self.switch = None;
    }

    /// The player picked `upscaler`: a switch, `true`, when it is not the
    /// one running, to be made before the next frame.
    pub(super) fn pick(&mut self, upscaler: Upscaler) -> bool {
        if upscaler == self.running {
            return false;
        }
        self.running = upscaler;
        self.switch = Some(upscaler);
        true
    }

    /// The switch still to be made, once.
    pub(super) fn take_switch(&mut self) -> Option<Upscaler> {
        self.switch.take()
    }
}

impl<S: ImageSource, C: Screen> App<S, C> {
    /// Sets `gpu` drawing flight's world with the upscaler the settings
    /// kept choose, if `gpu` can use it, and natively otherwise, with a
    /// warning naming the stored choice kept for the caller to show; then
    /// offers the screen the upscalers `gpu` can use, native first, and
    /// the one running ([`Screen::offer_upscalers`]). The stored choice is
    /// never changed here, so it comes back on a device that has it.
    pub fn restore_upscaler(&mut self, gpu: &mut (impl Gpu + UpscalerProbe)) {
        let stored = self
            .settings
            .as_ref()
            .map(|keeper| keeper.settings().frame_upscaler)
            .unwrap_or_default();
        let listed = listed(&gpu.upscalers());
        let (running, warning) = resolve(&stored, &listed);
        gpu.set_upscaler(running);
        self.upscaler.start(running);
        self.warnings.extend(warning);
        self.screen.offer_upscalers(&listed, running);
    }

    /// Makes the switch the player picked on `gpu`, if one is still to be
    /// made, so the frame drawn next is drawn with it.
    pub(super) fn switch_upscaler(&mut self, gpu: &mut impl Gpu) {
        if let Some(upscaler) = self.upscaler.take_switch() {
            gpu.set_upscaler(upscaler);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use nova_audio::SettingsStore;
    use nova_audio::recording::MemorySettings;
    use nova_render::recording::RecordingGpu;
    use nova_render::{Frame, ImageError, ImageSource, WorldSpace};
    use nova_view::{
        DrawList, ImageKind, Input, Key, Point, Prefs, Screen, ScreenAction, SoundPrefs, Upscaler,
    };

    use super::*;
    use crate::app::{App, WindowEvent, WindowPort};
    use crate::settings::{GameKeeper, SettingsKeeper, UpscalerSetting};

    const NATIVE: Upscaler = Upscaler::Native;
    const SHARP: Upscaler = Upscaler::SharpBilinear;

    fn named(name: &str) -> UpscalerSetting {
        UpscalerSetting::from(name.to_owned())
    }

    #[test]
    fn a_known_listed_choice_runs_without_a_warning() {
        let both = [NATIVE, SHARP];
        assert_eq!(resolve(&named("sharp_bilinear"), &both), (SHARP, None));
        assert_eq!(resolve(&named("native"), &both), (NATIVE, None));
        assert_eq!(resolve(&named("native"), &[NATIVE]), (NATIVE, None));
    }

    #[test]
    fn a_known_choice_the_device_lacks_draws_natively_with_a_warning_naming_it() {
        assert_eq!(
            resolve(&named("sharp_bilinear"), &[NATIVE]),
            (
                NATIVE,
                Some(
                    "nova: the frame upscaler \"sharp_bilinear\" in the settings is not \
                     available on this device; drawing natively"
                        .to_owned()
                )
            )
        );
    }

    #[test]
    fn an_unknown_choice_draws_natively_with_a_warning_naming_it() {
        let warning = "nova: the frame upscaler \"warp_drive\" in the settings is not one \
                       this game has; drawing natively";
        assert_eq!(
            resolve(&named("warp_drive"), &[NATIVE, SHARP]),
            (NATIVE, Some(warning.to_owned()))
        );
    }

    #[test]
    fn a_pick_is_a_switch_only_when_it_differs_from_the_one_running() {
        let mut upscaler = FrameUpscaler::default();
        assert_eq!(upscaler.running, NATIVE);
        assert!(!upscaler.pick(NATIVE));
        assert_eq!(upscaler.take_switch(), None);
        assert!(upscaler.pick(SHARP));
        assert_eq!(upscaler.running, SHARP);
        assert!(!upscaler.pick(SHARP), "already running");
        assert_eq!(upscaler.take_switch(), Some(SHARP));
        assert_eq!(upscaler.take_switch(), None, "once");
        assert!(upscaler.pick(NATIVE));
        upscaler.start(SHARP);
        assert_eq!(upscaler.running, SHARP);
        assert_eq!(upscaler.take_switch(), None, "a start drops a switch");
    }

    /// A 1024 x 768 window at scale 1.
    struct Window;

    impl WindowPort for Window {
        fn size_px(&self) -> (u32, u32) {
            (1024, 768)
        }

        fn scale_factor(&self) -> f64 {
            1.0
        }

        fn request_redraw(&mut self) {}
    }

    /// No images at all.
    struct NoImages;

    impl ImageSource for NoImages {
        fn frames(
            &self,
            _kind: ImageKind,
            _id: i16,
        ) -> Result<Vec<nova_data::graphics::Image>, ImageError> {
            Err(ImageError::Missing)
        }
    }

    /// Draws a world dot, reports `report` once as the preferences chosen,
    /// and keeps every offer of upscalers.
    #[derive(Default)]
    struct UpscalerScreen {
        report: Option<Prefs>,
        offers: Vec<(Vec<Upscaler>, Upscaler)>,
    }

    impl Screen for UpscalerScreen {
        fn input(&mut self, _input: &Input) -> ScreenAction {
            ScreenAction::None
        }

        fn tick(&mut self, _dt: Duration) {}

        fn draw(&self, list: &mut DrawList) {
            list.world(|list| {
                list.dot(Point::new(10.0, 10.0), 2.0, nova_view::Color::WHITE);
            });
        }

        fn take_prefs(&mut self) -> Option<Prefs> {
            self.report.take()
        }

        fn offer_upscalers(&mut self, listed: &[Upscaler], running: Upscaler) {
            self.offers.push((listed.to_vec(), running));
        }
    }

    type TestApp = App<NoImages, UpscalerScreen>;

    fn keeper(store: &MemorySettings) -> GameKeeper {
        let (keeper, warning) =
            SettingsKeeper::open(Box::new(store.clone()) as Box<dyn SettingsStore>);
        assert_eq!(warning, None);
        keeper
    }

    /// The app over `store`, its upscaler restored on `gpu`.
    fn restored(store: &MemorySettings, gpu: &mut RecordingGpu) -> TestApp {
        let mut app =
            App::new(&Window, NoImages, UpscalerScreen::default()).with_settings(keeper(store));
        app.restore_upscaler(gpu);
        app
    }

    /// The frame a redraw at `elapsed` submits.
    fn frame(app: &mut TestApp, gpu: &mut RecordingGpu, millis: u64) -> Frame {
        let redraw = WindowEvent::Redraw {
            elapsed: Duration::from_millis(millis),
        };
        app.handle(redraw, &mut Window, gpu);
        (*gpu.submits().last().expect("a frame")).clone()
    }

    fn texture() -> WorldSpace {
        WorldSpace::Texture { size: (1024, 768) }
    }

    fn holding(name: &str) -> (MemorySettings, String) {
        let text = format!(r#"{{"music": true, "frame_upscaler": "{name}"}}"#);
        (MemorySettings::holding(&text), text)
    }

    #[test]
    fn a_stored_choice_the_device_offers_runs_from_the_first_frame() {
        let (store, _) = holding("sharp_bilinear");
        let mut gpu = RecordingGpu::probing(&[SHARP]);
        let mut app = restored(&store, &mut gpu);
        assert_eq!(gpu.upscaler(), SHARP);
        assert_eq!(frame(&mut app, &mut gpu, 16).world_space, texture());
        assert_eq!(app.take_warnings(), Vec::<String>::new());
        assert_eq!(app.screen.offers, [(vec![NATIVE, SHARP], SHARP)]);
    }

    #[test]
    fn a_stored_choice_the_device_lacks_starts_native_with_one_warning_and_is_kept() {
        let (store, text) = holding("sharp_bilinear");
        let mut gpu = RecordingGpu::probing(&[NATIVE]);
        let mut app = restored(&store, &mut gpu);
        assert_eq!(gpu.upscaler(), NATIVE);
        assert_eq!(
            frame(&mut app, &mut gpu, 16).world_space,
            WorldSpace::Target
        );
        let warnings = app.take_warnings();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("\"sharp_bilinear\""), "{warnings:?}");
        assert_eq!(store.text(), Some(text), "unwritten");
        assert_eq!(store.writes(), 0);
        assert_eq!(app.screen.offers, [(vec![NATIVE], NATIVE)]);
    }

    const METAL_FX: Upscaler = Upscaler::MetalFxSpatial;

    #[test]
    fn a_stored_metal_fx_on_a_device_without_it_starts_native_with_one_warning() {
        let (store, text) = holding("metalfx_spatial");
        let mut gpu = RecordingGpu::probing(&[SHARP]);
        let mut app = restored(&store, &mut gpu);
        assert_eq!(gpu.upscaler(), NATIVE);
        assert_eq!(
            frame(&mut app, &mut gpu, 16).world_space,
            WorldSpace::Target
        );
        assert_eq!(
            app.take_warnings(),
            [
                "nova: the frame upscaler \"metalfx_spatial\" in the settings is not \
                 available on this device; drawing natively"
            ]
        );
        assert_eq!(store.text(), Some(text), "kept");
        assert_eq!(app.screen.offers, [(vec![NATIVE, SHARP], NATIVE)]);
    }

    #[test]
    fn a_stored_metal_fx_on_a_device_with_it_runs_and_all_three_are_listed() {
        let (store, _) = holding("metalfx_spatial");
        let mut gpu = RecordingGpu::probing(&[SHARP, METAL_FX]);
        let mut app = restored(&store, &mut gpu);
        assert_eq!(gpu.upscaler(), METAL_FX);
        assert_eq!(frame(&mut app, &mut gpu, 16).world_space, texture());
        assert_eq!(app.take_warnings(), Vec::<String>::new());
        assert_eq!(
            app.screen.offers,
            [(vec![NATIVE, SHARP, METAL_FX], METAL_FX)]
        );
    }

    #[test]
    fn an_unknown_stored_choice_starts_native_with_one_warning_and_is_kept() {
        let (store, text) = holding("warp_drive");
        let mut gpu = RecordingGpu::probing(&[NATIVE, SHARP]);
        let mut app = restored(&store, &mut gpu);
        assert_eq!(
            frame(&mut app, &mut gpu, 16).world_space,
            WorldSpace::Target
        );
        let warnings = app.take_warnings();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("\"warp_drive\""), "{warnings:?}");
        assert_eq!(store.text(), Some(text));
        assert_eq!(app.screen.offers, [(vec![NATIVE, SHARP], NATIVE)]);
    }

    #[test]
    fn without_settings_the_world_is_drawn_natively_and_quietly() {
        let mut gpu = RecordingGpu::probing(&[SHARP]);
        let mut app = App::new(&Window, NoImages, UpscalerScreen::default());
        app.restore_upscaler(&mut gpu);
        assert_eq!(gpu.upscaler(), NATIVE);
        assert_eq!(app.take_warnings(), Vec::<String>::new());
        assert_eq!(app.screen.offers, [(vec![NATIVE, SHARP], NATIVE)]);
    }

    /// The default preferences, drawn by `upscaler`, the music `music`.
    fn reported(upscaler: Upscaler, music: bool) -> Prefs {
        Prefs {
            sound: SoundPrefs {
                music,
                ..Prefs::default().sound
            },
            upscaler,
            ..Prefs::default()
        }
    }

    fn saved(store: &MemorySettings) -> serde_json::Value {
        serde_json::from_str(&store.text().expect("saved")).expect("JSON")
    }

    #[test]
    fn a_pick_reported_on_an_input_switches_before_the_next_frame_and_is_saved() {
        let (store, _) = holding("native");
        let mut gpu = RecordingGpu::probing(&[NATIVE, SHARP]);
        let mut app = restored(&store, &mut gpu);
        assert_eq!(
            frame(&mut app, &mut gpu, 16).world_space,
            WorldSpace::Target
        );
        app.screen.report = Some(reported(SHARP, true));
        let enter = WindowEvent::Key {
            key: Key::Enter,
            pressed: true,
            repeat: false,
        };
        app.handle(enter, &mut Window, &mut gpu);
        assert_eq!(saved(&store)["frame_upscaler"], "sharp_bilinear");
        assert_eq!(frame(&mut app, &mut gpu, 32).world_space, texture());
        assert_eq!(gpu.upscaler(), SHARP);
        assert_eq!(app.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn a_pick_noticed_in_a_redraw_switches_that_frame() {
        let (store, _) = holding("sharp_bilinear");
        let mut gpu = RecordingGpu::probing(&[NATIVE, SHARP]);
        let mut app = restored(&store, &mut gpu);
        assert_eq!(frame(&mut app, &mut gpu, 16).world_space, texture());
        app.screen.report = Some(reported(NATIVE, true));
        assert_eq!(
            frame(&mut app, &mut gpu, 32).world_space,
            WorldSpace::Target
        );
        assert_eq!(saved(&store)["frame_upscaler"], "native");
    }

    #[test]
    fn after_a_fallback_another_change_keeps_the_stored_choice() {
        let (store, _) = holding("sharp_bilinear");
        let mut gpu = RecordingGpu::probing(&[NATIVE]);
        let mut app = restored(&store, &mut gpu);
        app.take_warnings();
        app.screen.report = Some(reported(NATIVE, false));
        assert_eq!(
            frame(&mut app, &mut gpu, 16).world_space,
            WorldSpace::Target
        );
        assert_eq!(saved(&store)["music"], false, "the change saved");
        assert_eq!(saved(&store)["frame_upscaler"], "sharp_bilinear", "kept");
        assert_eq!(gpu.upscaler(), NATIVE);
    }
}
