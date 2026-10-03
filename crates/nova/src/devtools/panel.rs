//! The developer tools window: the frame-time readout and the resource
//! browser, drawn with egui from `nova_view::devtools`' model. Every string
//! it shows comes from the model.

use egui::{
    Context, FullOutput, Image, Label, RawInput, RichText, ScrollArea, Slider, TextEdit, Ui,
    Window, vec2,
};
use nova_view::devtools::{DevOverlay, ResourceBrowser, ResourceCatalog};

use super::textures::{PreviewKey, PreviewTextures};

/// The window's title.
pub const TITLE: &str = "Developer tools";
/// The search field's hint: type, ID and name are all searchable.
pub const HINT: &str = "Search by type, ID or name: rlëD 128, Shuttle";
/// The label of a sprite sheet's frame slider.
pub const SLIDER: &str = "frame";

/// How tall the results list and the preview may grow, in points.
const RESULTS_HEIGHT: f32 = 180.0;
const PREVIEW_HEIGHT: f32 = 240.0;

/// The developer tools window over a [`ResourceCatalog`]: its own egui
/// context, the resource browser, the preview texture and the query being
/// typed.
pub struct DevPanel<C> {
    ctx: Context,
    browser: ResourceBrowser<C>,
    textures: PreviewTextures,
    query: String,
}

impl<C: ResourceCatalog> DevPanel<C> {
    /// A panel browsing `catalog`, whose index it reads once.
    pub fn new(catalog: C) -> Self {
        Self {
            ctx: Context::default(),
            browser: ResourceBrowser::new(catalog),
            textures: PreviewTextures::default(),
            query: String::new(),
        }
    }

    /// The panel's egui context.
    pub fn context(&self) -> &Context {
        &self.ctx
    }

    /// The resource browser.
    pub fn browser(&self) -> &ResourceBrowser<C> {
        &self.browser
    }

    /// Runs one egui frame of the window over `input`, showing `overlay`'s
    /// frame times, and returns what to paint.
    pub fn run(&mut self, input: RawInput, overlay: &DevOverlay) -> FullOutput {
        let ctx = self.ctx.clone();
        ctx.run_ui(input, |ui| {
            Window::new(TITLE)
                .default_pos([16.0, 16.0])
                .default_width(440.0)
                .resizable(true)
                .show(ui.ctx(), |ui| self.contents(ui, overlay));
        })
    }

    fn contents(&mut self, ui: &mut Ui, overlay: &DevOverlay) {
        ui.label(overlay.frame_times().readout());
        ui.separator();
        ui.add(TextEdit::singleline(&mut self.query).hint_text(HINT));
        self.browser.set_query(&self.query);
        ui.label(self.browser.count_line());
        self.results(ui);
        self.selection(ui);
    }

    /// The results list; a click selects one.
    fn results(&mut self, ui: &mut Ui) {
        let browser = &self.browser;
        let selected = browser
            .selection()
            .map(|s| (s.summary().ty, s.summary().id));
        let row_height = ui.spacing().interact_size.y;
        let mut clicked = None;
        ScrollArea::vertical()
            .id_salt("results")
            .max_height(RESULTS_HEIGHT)
            .auto_shrink([false, true])
            .show_rows(ui, row_height, browser.result_count(), |ui, rows| {
                for i in rows {
                    let summary = browser.result(i).expect("a row is a result");
                    let checked = selected == Some((summary.ty, summary.id));
                    if ui.selectable_label(checked, summary.label()).clicked() {
                        clicked = Some(i);
                    }
                }
            });
        if let Some(i) = clicked {
            self.browser.select_result(i);
        }
    }

    /// The selected resource: its heading, files, size, warning, preview
    /// and record.
    fn selection(&mut self, ui: &mut Ui) {
        let Some(selection) = self.browser.selection_mut() else {
            self.textures.release();
            return;
        };
        ui.separator();
        ui.strong(selection.heading());
        ui.label(selection.file_line());
        for line in selection.overridden_lines() {
            ui.label(line);
        }
        ui.label(selection.size_line());
        if let Some(warning) = selection.warning_line() {
            ui.colored_label(ui.visuals().warn_fg_color, warning);
        }
        let summary = selection.summary();
        let key = PreviewKey {
            ty: summary.ty,
            id: summary.id,
            frame: selection.frame(),
        };
        match selection.image() {
            Some(image) => {
                let texture = self.textures.texture_for(ui.ctx(), key, image);
                // Shown at its own size, scaled down to fit and never up.
                ui.add(
                    Image::new(texture)
                        .fit_to_original_size(1.0)
                        .max_size(vec2(ui.available_width(), PREVIEW_HEIGHT)),
                );
            }
            None => self.textures.release(),
        }
        if let Some(caption) = selection.preview_caption() {
            ui.label(caption);
        }
        if selection.frame_count() > 1 {
            let mut frame = selection.frame();
            ui.add(
                Slider::new(&mut frame, 0..=selection.last_frame())
                    .show_value(false)
                    .text(SLIDER),
            );
            selection.set_frame(frame);
        }
        ui.separator();
        let record = selection.record_text();
        ScrollArea::vertical()
            .id_salt("record")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.add(Label::new(RichText::new(record).monospace()))
            });
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use egui::epaint::Shape;
    use egui::{Color32, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};
    use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
    use nova_data::graphics::{PICT, RLED};
    use nova_view::devtools::{
        DevOverlay, Origin, RecordView, ResType, ResourceCatalog, ResourceDetail, ResourceSummary,
        SourceInfo,
    };

    use super::*;

    const SPIN: ResType = ResType::new([b's', b'p', 0x95, b'n']);

    /// A 4 × 2 picture.
    fn picture() -> Vec<u8> {
        let bounds = [0, 0, 2, 4];
        PictBuilder::new(bounds)
            .direct_bits(&DirectBits::rgb555(bounds, &[0x7C00; 8]))
            .end()
            .build()
    }

    /// A sheet of 3 frames of 6 × 5.
    fn sheet() -> Vec<u8> {
        (0..3u16)
            .fold(RledBuilder::new(6, 5), |sheet, n| {
                sheet.frame(|f| (0..5).fold(f, |f, _| f.line().pixels(&[0x0400 * n; 6])))
            })
            .build()
    }

    /// PICT 128 "Planet", rlëD 200 "Shuttle" and spïn 300 "Spinner".
    struct Catalog;

    impl ResourceCatalog for Catalog {
        fn resources(&self) -> Vec<ResourceSummary> {
            [
                (PICT, 128, "Planet"),
                (RLED, 200, "Shuttle"),
                (SPIN, 300, "Spinner"),
            ]
            .into_iter()
            .map(|(ty, id, name)| ResourceSummary {
                ty,
                id,
                name: Some(name.to_owned()),
            })
            .collect()
        }

        fn inspect(&self, ty: ResType, id: i16) -> Option<ResourceDetail> {
            let (data, record) = match (ty, id) {
                (PICT, 128) => (picture(), RecordView::NotARecord),
                (RLED, 200) => (sheet(), RecordView::NotARecord),
                (SPIN, 300) => (
                    vec![0; 12],
                    RecordView::Json {
                        text: "{\n  \"x_tiles\": 6\n}".to_owned(),
                        warning: None,
                    },
                ),
                _ => return None,
            };
            Some(ResourceDetail {
                data,
                source: SourceInfo {
                    path: PathBuf::from("/data/Nova Data"),
                    origin: Origin::Data,
                },
                overridden: Vec::new(),
                record,
            })
        }
    }

    /// The text drawn in one frame, with where it was drawn, and the filled
    /// rectangles.
    struct Drawn {
        texts: Vec<(String, Rect)>,
        fills: Vec<(Rect, Color32)>,
    }

    impl Drawn {
        fn has(&self, text: &str) -> bool {
            self.texts.iter().any(|(t, _)| t == text)
        }

        fn contains(&self, part: &str) -> bool {
            self.texts.iter().any(|(t, _)| t.contains(part))
        }

        fn rect(&self, text: &str) -> Rect {
            self.texts
                .iter()
                .find(|(t, _)| t == text)
                .unwrap_or_else(|| panic!("{text:?} not drawn in {:?}", self.all()))
                .1
        }

        fn all(&self) -> Vec<&str> {
            self.texts.iter().map(|(t, _)| t.as_str()).collect()
        }
    }

    fn collect(shape: &Shape, drawn: &mut Drawn) {
        match shape {
            Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, drawn);
                }
            }
            Shape::Text(text) => drawn.texts.push((
                text.galley.text().to_owned(),
                text.galley.rect.translate(text.pos.to_vec2()),
            )),
            Shape::Rect(rect) => drawn.fills.push((rect.rect, rect.fill)),
            _ => {}
        }
    }

    /// A 1024 × 768 screen at one pixel per point.
    fn input(events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1024.0, 768.0))),
            max_texture_side: Some(8192),
            events,
            ..RawInput::default()
        }
    }

    struct Harness {
        panel: DevPanel<Catalog>,
        overlay: DevOverlay,
    }

    impl Harness {
        /// A panel run twice, so its new window is laid out and shows.
        fn new() -> Self {
            let mut harness = Self {
                panel: DevPanel::new(Catalog),
                overlay: DevOverlay::new(),
            };
            harness.frame(Vec::new());
            harness.frame(Vec::new());
            harness
        }

        fn frame(&mut self, events: Vec<Event>) -> Drawn {
            let output = self.panel.run(input(events), &self.overlay);
            let mut drawn = Drawn {
                texts: Vec::new(),
                fills: Vec::new(),
            };
            for clipped in &output.shapes {
                collect(&clipped.shape, &mut drawn);
            }
            output.drop_without_applying_deltas();
            drawn
        }

        fn idle(&mut self) -> Drawn {
            self.frame(Vec::new())
        }

        /// Clicks the middle of `rect`: a move, a press and a release, each
        /// in its own frame.
        fn click(&mut self, rect: Rect) -> Drawn {
            let at = rect.center();
            let button = |pressed| Event::PointerButton {
                pos: at,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            self.frame(vec![Event::PointerMoved(at)]);
            self.frame(vec![button(true)]);
            self.frame(vec![button(false)]);
            self.idle()
        }

        /// Clicks the text `label`, wherever it was last drawn.
        fn click_text(&mut self, label: &str) -> Drawn {
            let rect = self.idle().rect(label);
            self.click(rect)
        }

        fn type_text(&mut self, text: &str) -> Drawn {
            let field = self.idle().rect(HINT);
            self.click(field);
            self.frame(vec![Event::Text(text.to_owned())]);
            self.idle()
        }

        /// The size of the allocated texture named `name`, if there is one.
        fn texture(&self, name: &str) -> Option<[usize; 2]> {
            let manager = self.panel.context().tex_manager();
            let manager = manager.read();
            manager
                .allocated()
                .find(|(_, meta)| meta.name == name)
                .map(|(_, meta)| meta.size)
        }
    }

    #[test]
    fn the_frame_time_readout_shows() {
        let mut harness = Harness::new();
        harness.overlay.frame(Duration::ZERO);
        harness.overlay.frame(Duration::from_millis(20));
        let drawn = harness.idle();
        let readout = harness.overlay.frame_times().readout();
        assert!(drawn.has(&readout), "{readout:?} in {:?}", drawn.all());
    }

    #[test]
    fn every_result_and_the_count_show() {
        let drawn = Harness::new().idle();
        for label in ["PICT 128 Planet", "rlëD 200 Shuttle", "spïn 300 Spinner"] {
            assert!(drawn.has(label), "{label:?} in {:?}", drawn.all());
        }
        assert!(drawn.has("3 of 3 resources"), "{:?}", drawn.all());
        assert!(drawn.has(HINT), "{:?}", drawn.all());
    }

    #[test]
    fn only_the_selected_result_is_highlighted() {
        let mut harness = Harness::new();
        harness.click_text("rlëD 200 Shuttle");
        harness.frame(vec![Event::PointerGone]);
        let drawn = harness.idle();
        let labels = ["PICT 128 Planet", "rlëD 200 Shuttle", "spïn 300 Spinner"];
        let highlighted: Vec<&str> = labels
            .into_iter()
            .filter(|label| {
                let at = drawn.rect(label).center();
                // A row's own background: one row high, and not
                // transparent. Nothing is hovered.
                drawn.fills.iter().any(|(rect, fill)| {
                    rect.contains(at) && rect.height() < 30.0 && *fill != Color32::TRANSPARENT
                })
            })
            .collect();
        assert_eq!(highlighted, ["rlëD 200 Shuttle"]);
    }

    #[test]
    fn typing_narrows_the_results() {
        let mut harness = Harness::new();
        let drawn = harness.type_text("rled");
        assert_eq!(harness.panel.browser().query(), "rled");
        assert!(drawn.has("rlëD 200 Shuttle"), "{:?}", drawn.all());
        assert!(!drawn.has("PICT 128 Planet"), "{:?}", drawn.all());
        assert!(drawn.has("1 of 3 resources"), "{:?}", drawn.all());
    }

    #[test]
    fn clicking_a_result_shows_its_heading_file_and_record() {
        let mut harness = Harness::new();
        let drawn = harness.click_text("spïn 300 Spinner");
        let selection = harness.panel.browser().selection().expect("selected");
        assert_eq!(selection.summary().id, 300);
        for text in [
            selection.heading(),
            selection.file_line(),
            selection.size_line(),
        ] {
            assert!(drawn.has(&text), "{text:?} in {:?}", drawn.all());
        }
        assert!(drawn.contains("\"x_tiles\": 6"), "{:?}", drawn.all());
        assert_eq!(harness.texture("spïn 300 frame 0"), None);
    }

    #[test]
    fn selecting_a_picture_loads_a_texture_of_its_size() {
        let mut harness = Harness::new();
        let drawn = harness.click_text("PICT 128 Planet");
        assert_eq!(harness.texture("PICT 128 frame 0"), Some([4, 2]));
        assert!(drawn.has("4 × 2 pixels"), "{:?}", drawn.all());
        assert!(!drawn.has(SLIDER), "a picture has no frames to step");
        harness.click_text("spïn 300 Spinner");
        assert_eq!(harness.texture("PICT 128 frame 0"), None, "released");
    }

    #[test]
    fn only_a_sheet_has_a_frame_slider() {
        let mut harness = Harness::new();
        let drawn = harness.click_text("rlëD 200 Shuttle");
        assert!(drawn.has(SLIDER), "{:?}", drawn.all());
        assert!(drawn.has("frame 1 of 3, 6 × 5 pixels"), "{:?}", drawn.all());
        assert_eq!(harness.texture("rlëD 200 frame 0"), Some([6, 5]));
        let drawn = harness.click_text("spïn 300 Spinner");
        assert!(!drawn.has(SLIDER), "{:?}", drawn.all());
    }

    #[test]
    fn the_frame_slider_steps_the_sheet() {
        let mut harness = Harness::new();
        let drawn = harness.click_text("rlëD 200 Shuttle");
        // The slider's rail ends left of its label.
        let label = drawn.rect(SLIDER);
        let rail_end = Rect::from_center_size(
            Pos2::new(label.left() - 12.0, label.center().y),
            vec2(4.0, 4.0),
        );
        let drawn = harness.click(rail_end);
        harness.idle();
        let selection = harness.panel.browser().selection().expect("selected");
        assert_eq!(selection.frame(), 2, "{:?}", drawn.all());
        assert!(harness.idle().has("frame 3 of 3, 6 × 5 pixels"));
        assert_eq!(harness.texture("rlëD 200 frame 2"), Some([6, 5]));
    }
}
