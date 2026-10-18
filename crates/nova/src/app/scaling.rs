//! The player's Scaling: the one the game starts with, from the stored
//! choice and what the device can use, and the switches a pick in the
//! Preferences dialog makes at each of its two sites, the per-frame
//! upscaler and the load-time enlarger.

use nova_render::{Gpu, ImageSource, UpscalerProbe, enlarger_for, listed};
use nova_view::{Enlargement, Scaling, Screen, Upscaler};

use super::App;
use crate::settings::ScalingSetting;

/// The scaling to start with from the `stored` choice, given the list the
/// device offers, `listed`, and a warning to show when it is not the one
/// stored: the stored one when it is known and listed, and native
/// otherwise. Super-xBR is always listed; a known scaling that isn't is a
/// per-frame upscaler the device lacks.
pub(super) fn resolve(stored: &ScalingSetting, listed: &[Scaling]) -> (Scaling, Option<String>) {
    let name = stored.name();
    match stored.known() {
        Some(scaling) if listed.contains(&scaling) => (scaling, None),
        Some(_) => (
            Scaling::Native,
            Some(format!(
                "nova: the frame upscaler \"{name}\" in the settings is not available on \
                 this device; drawing natively"
            )),
        ),
        None => (
            Scaling::Native,
            Some(format!(
                "nova: the frame upscaler \"{name}\" in the settings is not one this game \
                 has; drawing natively"
            )),
        ),
    }
}

/// The scaling running now, and the switches a pick left to make at each
/// of its sites ([`Scaling::sites`]): the upscaler on the GPU before the
/// next frame, and the enlarger on the renderer.
#[derive(Debug, Default)]
pub(super) struct ScalingChoice {
    running: Scaling,
    switch: Option<Upscaler>,
    enlargement: Option<Enlargement>,
}

impl ScalingChoice {
    /// Starts with `scaling`, both its sites already set.
    pub(super) fn start(&mut self, scaling: Scaling) {
        self.running = scaling;
        self.switch = None;
        self.enlargement = None;
    }

    /// The player picked `scaling`: a change, `true`, when it is not the
    /// one running. Each site whose half differs gets a switch to make.
    pub(super) fn pick(&mut self, scaling: Scaling) -> bool {
        if scaling == self.running {
            return false;
        }
        let (upscaler, enlargement) = scaling.sites();
        let (was_upscaler, was_enlargement) = self.running.sites();
        if upscaler != was_upscaler {
            self.switch = Some(upscaler);
        }
        if enlargement != was_enlargement {
            self.enlargement = Some(enlargement);
        }
        self.running = scaling;
        true
    }

    /// The upscaler switch still to be made, once.
    pub(super) fn take_switch(&mut self) -> Option<Upscaler> {
        self.switch.take()
    }

    /// The enlargement switch still to be made, once.
    pub(super) fn take_enlargement(&mut self) -> Option<Enlargement> {
        self.enlargement.take()
    }
}

impl<S: ImageSource, C: Screen> App<S, C> {
    /// Applies the Scaling the settings kept, if the device can use it, and
    /// native otherwise, with a warning naming the stored choice kept for
    /// the caller to show: sets `gpu` drawing flight's world with its
    /// upscaler and, for a load-time scaling, the renderer enlarging with
    /// its enlarger (before the first frame, so the atlas is still empty).
    /// Then offers the screen the Scaling list (native, the upscalers
    /// `gpu` can use, then Super-xBR) and the one running
    /// ([`Screen::offer_scaling`]). The stored choice is never changed
    /// here, so it comes back on a device that has it.
    pub fn restore_scaling(&mut self, gpu: &mut (impl Gpu + UpscalerProbe)) {
        let stored = self
            .settings
            .as_ref()
            .map(|keeper| keeper.settings().frame_upscaler)
            .unwrap_or_default();
        let list = Scaling::list(&listed(&gpu.upscalers()));
        let (running, warning) = resolve(&stored, &list);
        let (upscaler, enlargement) = running.sites();
        gpu.set_upscaler(upscaler);
        if enlargement != Enlargement::Native {
            self.renderer.set_enlarger(enlarger_for(enlargement));
        }
        self.scaling.start(running);
        self.warnings.extend(warning);
        self.screen.offer_scaling(&list, running);
    }

    /// Picks `scaling`, as the Preferences dialog reported it: `true` when
    /// it is a change. A change of the enlargement is made on the renderer
    /// at once, which starts the atlas afresh at the next frame; a change
    /// of the upscaler waits for [`App::switch_upscaler`]. A switch between
    /// per-frame scalers changes no enlargement, so it never rebuilds the
    /// atlas.
    pub(super) fn pick_scaling(&mut self, scaling: Scaling) -> bool {
        let changed = self.scaling.pick(scaling);
        if let Some(enlargement) = self.scaling.take_enlargement() {
            self.renderer.set_enlarger(enlarger_for(enlargement));
        }
        changed
    }

    /// Makes the upscaler switch the player picked on `gpu`, if one is
    /// still to be made, so the frame drawn next is drawn with it.
    pub(super) fn switch_upscaler(&mut self, gpu: &mut impl Gpu) {
        if let Some(upscaler) = self.scaling.take_switch() {
            gpu.set_upscaler(upscaler);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use nova_audio::SettingsStore;
    use nova_audio::recording::MemorySettings;
    use nova_render::recording::{GpuCall, RecordingGpu};
    use nova_render::{Frame, ImageError, ImageSource, WorldSpace};
    use nova_view::{
        Color, DrawList, Enlargement, ImageKey, ImageKind, Input, Key, Point, Prefs, Scaling,
        Screen, ScreenAction, SoundPrefs, Upscaler,
    };

    use super::*;
    use crate::app::{App, WindowEvent, WindowPort};
    use crate::settings::{GameKeeper, ScalingSetting, SettingsKeeper};

    const NATIVE: Upscaler = Upscaler::Native;
    const SHARP: Upscaler = Upscaler::SharpBilinear;

    fn named(name: &str) -> ScalingSetting {
        ScalingSetting::from(name.to_owned())
    }

    /// The Scaling list a probe answering `upscalers` gives.
    fn list(upscalers: &[Upscaler]) -> Vec<Scaling> {
        Scaling::list(&listed(upscalers))
    }

    #[test]
    fn a_known_listed_choice_runs_without_a_warning() {
        let both = list(&[NATIVE, SHARP]);
        assert_eq!(
            resolve(&named("sharp_bilinear"), &both),
            (Scaling::SharpBilinear, None)
        );
        assert_eq!(resolve(&named("native"), &both), (Scaling::Native, None));
        assert_eq!(
            resolve(&named("native"), &list(&[NATIVE])),
            (Scaling::Native, None)
        );
    }

    #[test]
    fn super_xbr_runs_without_a_warning_on_any_device() {
        assert_eq!(
            resolve(&named("super_xbr"), &list(&[NATIVE])),
            (Scaling::SuperXbr, None)
        );
    }

    #[test]
    fn a_known_choice_the_device_lacks_draws_natively_with_a_warning_naming_it() {
        assert_eq!(
            resolve(&named("sharp_bilinear"), &list(&[NATIVE])),
            (
                Scaling::Native,
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
        for name in ["warp_drive", "hd_pack_v9"] {
            let warning = format!(
                "nova: the frame upscaler \"{name}\" in the settings is not one this game \
                 has; drawing natively"
            );
            assert_eq!(
                resolve(&named(name), &list(&[NATIVE, SHARP])),
                (Scaling::Native, Some(warning))
            );
        }
    }

    /// `choice` after a pick of `scaling`: whether it was a change, then
    /// the upscaler switch and the enlargement it left to make.
    fn picked(
        choice: &mut ScalingChoice,
        scaling: Scaling,
    ) -> (bool, Option<Upscaler>, Option<Enlargement>) {
        let changed = choice.pick(scaling);
        (changed, choice.take_switch(), choice.take_enlargement())
    }

    #[test]
    fn a_pick_switches_only_the_sites_it_changes() {
        use Scaling::{MetalFxSpatial, Native, SharpBilinear, SuperXbr};
        for (from, to, switch, enlargement) in [
            (Native, SharpBilinear, Some(SHARP), None),
            (
                SharpBilinear,
                MetalFxSpatial,
                Some(Upscaler::MetalFxSpatial),
                None,
            ),
            (Native, SuperXbr, None, Some(Enlargement::SuperXbr)),
            (
                SharpBilinear,
                SuperXbr,
                Some(NATIVE),
                Some(Enlargement::SuperXbr),
            ),
            (
                SuperXbr,
                MetalFxSpatial,
                Some(Upscaler::MetalFxSpatial),
                Some(Enlargement::Native),
            ),
        ] {
            let mut choice = ScalingChoice::default();
            choice.start(from);
            assert_eq!(
                picked(&mut choice, to),
                (true, switch, enlargement),
                "{from:?} to {to:?}"
            );
            assert_eq!(picked(&mut choice, to), (false, None, None), "running");
        }
    }

    #[test]
    fn each_switch_is_taken_once_and_a_start_drops_them() {
        let mut choice = ScalingChoice::default();
        assert_eq!(choice.running, Scaling::Native);
        assert!(!choice.pick(Scaling::Native));
        assert!(choice.pick(Scaling::SuperXbr));
        assert!(choice.pick(Scaling::SharpBilinear));
        assert_eq!(choice.running, Scaling::SharpBilinear);
        assert_eq!(choice.take_switch(), Some(SHARP));
        assert_eq!(choice.take_switch(), None, "once");
        assert_eq!(
            choice.take_enlargement(),
            Some(Enlargement::Native),
            "the last pick's"
        );
        assert_eq!(choice.take_enlargement(), None, "once");
        assert!(choice.pick(Scaling::SuperXbr));
        choice.start(Scaling::SharpBilinear);
        assert_eq!(choice.running, Scaling::SharpBilinear);
        assert_eq!(choice.take_switch(), None, "a start drops a switch");
        assert_eq!(choice.take_enlargement(), None, "and an enlargement");
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

    /// One sprite: `rlëD` 1, a single 3x2 frame of one colour.
    struct OneSprite;

    impl ImageSource for OneSprite {
        fn frames(
            &self,
            kind: ImageKind,
            id: i16,
        ) -> Result<Vec<nova_data::graphics::Image>, ImageError> {
            if (kind, id) != (ImageKind::Rled, 1) {
                return Err(ImageError::Missing);
            }
            let pixels = [90, 160, 230, 255].repeat(6);
            Ok(vec![
                nova_data::graphics::Image::from_rgba(3, 2, pixels).expect("3x2"),
            ])
        }
    }

    /// Draws a world dot and the sprite, reports `report` once as the
    /// preferences chosen, and keeps every offer of the Scaling list.
    #[derive(Default)]
    struct ScalingScreen {
        report: Option<Prefs>,
        offers: Vec<(Vec<Scaling>, Scaling)>,
    }

    impl Screen for ScalingScreen {
        fn input(&mut self, _input: &Input) -> ScreenAction {
            ScreenAction::None
        }

        fn tick(&mut self, _dt: Duration) {}

        fn draw(&self, list: &mut DrawList) {
            list.world(|list| {
                list.dot(Point::new(10.0, 10.0), 2.0, Color::WHITE).sprite(
                    ImageKey::sprite(1, 0),
                    Point::new(40.0, 30.0),
                    Color::WHITE,
                );
            });
        }

        fn take_prefs(&mut self) -> Option<Prefs> {
            self.report.take()
        }

        fn offer_scaling(&mut self, listed: &[Scaling], running: Scaling) {
            self.offers.push((listed.to_vec(), running));
        }
    }

    type TestApp = App<OneSprite, ScalingScreen>;

    fn keeper(store: &MemorySettings) -> GameKeeper {
        let (keeper, warning) =
            SettingsKeeper::open(Box::new(store.clone()) as Box<dyn SettingsStore>);
        assert_eq!(warning, None);
        keeper
    }

    /// The app over `store`, its Scaling restored on `gpu`.
    fn restored(store: &MemorySettings, gpu: &mut RecordingGpu) -> TestApp {
        let mut app =
            App::new(&Window, OneSprite, ScalingScreen::default()).with_settings(keeper(store));
        app.restore_scaling(gpu);
        app
    }

    /// The calls drawing the frame a redraw at `millis` makes, from the
    /// last frame's on.
    fn frame_calls(app: &mut TestApp, gpu: &mut RecordingGpu, millis: u64) -> Vec<GpuCall> {
        gpu.take();
        frame(app, gpu, millis);
        gpu.take()
    }

    /// The sizes of the images `calls` upload, in order.
    fn uploads(calls: &[GpuCall]) -> Vec<(u32, u32)> {
        calls
            .iter()
            .filter_map(|call| match call {
                GpuCall::Upload { width, height, .. } => Some((*width, *height)),
                _ => None,
            })
            .collect()
    }

    /// The sprite at its own size, and doubled.
    const NATIVE_SIZE: (u32, u32) = (3, 2);
    const DOUBLED: (u32, u32) = (6, 4);

    /// The world space of the frame `calls` submit.
    fn world_of(calls: &[GpuCall]) -> WorldSpace {
        match calls.last() {
            Some(GpuCall::Submit(frame)) => frame.world_space,
            other => panic!("not a frame: {other:?}"),
        }
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
        assert_eq!(
            app.screen.offers,
            [(list(&[NATIVE, SHARP]), Scaling::SharpBilinear)]
        );
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
        assert_eq!(app.screen.offers, [(list(&[NATIVE]), Scaling::Native)]);
    }

    const METAL_FX: Upscaler = Upscaler::MetalFxSpatial;

    #[test]
    fn a_gpu_warning_is_passed_on_after_the_frame_that_raised_it_once() {
        let (store, _) = holding("metalfx_spatial");
        let mut gpu = RecordingGpu::probing(&[SHARP, METAL_FX]);
        let mut app = restored(&store, &mut gpu);
        gpu.warn("nova: MetalFX can't scale the world");
        assert_eq!(app.take_warnings(), Vec::<String>::new(), "no frame yet");
        frame(&mut app, &mut gpu, 16);
        assert_eq!(app.take_warnings(), ["nova: MetalFX can't scale the world"]);
        frame(&mut app, &mut gpu, 32);
        assert_eq!(app.take_warnings(), Vec::<String>::new(), "once");
    }

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
        assert_eq!(app.screen.offers, [(list(&[SHARP]), Scaling::Native)]);
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
            [(
                vec![
                    Scaling::Native,
                    Scaling::SharpBilinear,
                    Scaling::MetalFxSpatial,
                    Scaling::SuperXbr
                ],
                Scaling::MetalFxSpatial
            )]
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
        assert_eq!(
            app.screen.offers,
            [(list(&[NATIVE, SHARP]), Scaling::Native)]
        );
    }

    #[test]
    fn without_settings_the_world_is_drawn_natively_and_quietly() {
        let mut gpu = RecordingGpu::probing(&[SHARP]);
        let mut app = App::new(&Window, OneSprite, ScalingScreen::default());
        app.restore_scaling(&mut gpu);
        assert_eq!(gpu.upscaler(), NATIVE);
        assert_eq!(app.take_warnings(), Vec::<String>::new());
        assert_eq!(app.screen.offers, [(list(&[SHARP]), Scaling::Native)]);
        assert_eq!(uploads(&frame_calls(&mut app, &mut gpu, 16)), [NATIVE_SIZE]);
    }

    #[test]
    fn a_stored_super_xbr_draws_the_world_natively_and_its_first_frame_enlarged() {
        let (store, _) = holding("super_xbr");
        let mut gpu = RecordingGpu::probing(&[NATIVE]);
        let mut app = restored(&store, &mut gpu);
        assert_eq!(gpu.upscaler(), NATIVE);

        let calls = frame_calls(&mut app, &mut gpu, 16);

        assert_eq!(world_of(&calls), WorldSpace::Target);
        assert!(!calls.contains(&GpuCall::ClearPages), "nothing to clear");
        assert_eq!(uploads(&calls), [DOUBLED]);
        assert_eq!(app.take_warnings(), Vec::<String>::new());
        assert_eq!(app.screen.offers, [(list(&[NATIVE]), Scaling::SuperXbr)]);
    }

    #[test]
    fn a_stored_per_frame_scaler_loads_sprites_natively() {
        let (store, _) = holding("sharp_bilinear");
        let mut gpu = RecordingGpu::probing(&[SHARP]);
        let mut app = restored(&store, &mut gpu);

        let mut calls = frame_calls(&mut app, &mut gpu, 16);
        calls.extend(frame_calls(&mut app, &mut gpu, 32));

        assert_eq!(world_of(&calls), texture());
        assert!(!calls.contains(&GpuCall::ClearPages));
        assert_eq!(uploads(&calls), [NATIVE_SIZE]);
    }

    /// Reports `scaling` chosen, then presses Return.
    fn pick(app: &mut TestApp, gpu: &mut RecordingGpu, scaling: Scaling) {
        app.screen.report = Some(reported(scaling, true));
        let enter = WindowEvent::Key {
            key: Key::Enter,
            pressed: true,
            repeat: false,
        };
        app.handle(enter, &mut Window, gpu);
    }

    #[test]
    fn choosing_super_xbr_or_leaving_it_rebuilds_the_atlas_for_the_next_frame() {
        let (store, _) = holding("native");
        let mut gpu = RecordingGpu::probing(&[NATIVE, SHARP]);
        let mut app = restored(&store, &mut gpu);
        assert_eq!(uploads(&frame_calls(&mut app, &mut gpu, 16)), [NATIVE_SIZE]);

        pick(&mut app, &mut gpu, Scaling::SuperXbr);

        assert_eq!(saved(&store)["frame_upscaler"], "super_xbr");
        let calls = frame_calls(&mut app, &mut gpu, 32);
        assert_eq!(calls[0], GpuCall::ClearPages);
        assert_eq!(uploads(&calls), [DOUBLED]);
        assert_eq!(world_of(&calls), WorldSpace::Target);

        pick(&mut app, &mut gpu, Scaling::SharpBilinear);

        assert_eq!(saved(&store)["frame_upscaler"], "sharp_bilinear");
        let calls = frame_calls(&mut app, &mut gpu, 48);
        assert_eq!(calls[0], GpuCall::ClearPages);
        assert_eq!(uploads(&calls), [NATIVE_SIZE]);
        assert_eq!(world_of(&calls), texture());
        assert_eq!(app.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn switching_between_per_frame_scalers_never_rebuilds_the_atlas() {
        let (store, _) = holding("sharp_bilinear");
        let mut gpu = RecordingGpu::probing(&[SHARP, METAL_FX]);
        let mut app = restored(&store, &mut gpu);
        frame_calls(&mut app, &mut gpu, 16);
        frame_calls(&mut app, &mut gpu, 32);

        let mut calls = Vec::new();
        for (millis, scaling) in [(48, Scaling::MetalFxSpatial), (64, Scaling::Native)] {
            pick(&mut app, &mut gpu, scaling);
            calls.extend(frame_calls(&mut app, &mut gpu, millis));
        }

        assert!(!calls.contains(&GpuCall::ClearPages));
        assert_eq!(uploads(&calls), []);
        assert_eq!(gpu.upscaler(), NATIVE);
        assert_eq!(saved(&store)["frame_upscaler"], "native");
    }

    #[test]
    fn a_scaling_pick_keeps_an_unknown_stored_name_only_until_it_changes_it() {
        let (store, _) = holding("hd_pack_v9");
        let mut gpu = RecordingGpu::probing(&[NATIVE]);
        let mut app = restored(&store, &mut gpu);
        assert_eq!(app.take_warnings().len(), 1);
        app.screen.report = Some(reported(Scaling::Native, false));
        frame(&mut app, &mut gpu, 16);
        assert_eq!(saved(&store)["frame_upscaler"], "hd_pack_v9", "kept");
        pick(&mut app, &mut gpu, Scaling::SuperXbr);
        assert_eq!(saved(&store)["frame_upscaler"], "super_xbr");
    }

    /// The default preferences, scaled by `scaling`, the music `music`.
    fn reported(scaling: Scaling, music: bool) -> Prefs {
        Prefs {
            sound: SoundPrefs {
                music,
                ..Prefs::default().sound
            },
            scaling,
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
        app.screen.report = Some(reported(Scaling::SharpBilinear, true));
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
        app.screen.report = Some(reported(Scaling::Native, true));
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
        app.screen.report = Some(reported(Scaling::Native, false));
        assert_eq!(
            frame(&mut app, &mut gpu, 16).world_space,
            WorldSpace::Target
        );
        assert_eq!(saved(&store)["music"], false, "the change saved");
        assert_eq!(saved(&store)["frame_upscaler"], "sharp_bilinear", "kept");
        assert_eq!(gpu.upscaler(), NATIVE);
    }
}
