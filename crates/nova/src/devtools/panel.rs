//! The developer tools windows, drawn with egui from `nova_view::devtools`'
//! model: "Developer tools", the frame-time readout and the resource
//! browser, and beside it "Pilot", the pilot editor over the pilot flying.
//! Every string they show comes from the model.

use egui::{
    Context, FullOutput, Image, Label, RawInput, RichText, ScrollArea, Slider, TextEdit, Ui,
    Window, vec2,
};
use nova_sim::Reserve;
pub use nova_view::devtools::NO_PILOT;
use nova_view::devtools::{
    DevOverlay, Draft, PilotDesk, PilotEditor, ResourceBrowser, ResourceCatalog, bit_label,
};

use super::textures::{PreviewKey, PreviewTextures};

/// The window's title.
pub const TITLE: &str = "Developer tools";
/// The search field's hint: type, ID and name are all searchable.
pub const HINT: &str = "Search by type, ID or name: rlëD 128, Shuttle";
/// The label of a sprite sheet's frame slider.
pub const SLIDER: &str = "frame";

/// The pilot editor's window title.
pub const PILOT_TITLE: &str = "Pilot";
/// The button that applies the field beside it, as Enter in it does.
pub const SET: &str = "Set";
/// The button that moves the pilot to the stellar picked.
pub const MOVE_TO: &str = "Move to";
/// The system filter's hint.
pub const SYSTEM_HINT: &str = "Filter systems by name or ID";
/// The bit query's hint.
pub const BIT_HINT: &str = "Bit number, 0-9999";

/// How tall the results list and the preview may grow, in points.
const RESULTS_HEIGHT: f32 = 180.0;
const PREVIEW_HEIGHT: f32 = 240.0;
/// How tall the pilot editor's lists may grow, in points.
const PLACES_HEIGHT: f32 = 90.0;
const BITS_HEIGHT: f32 = 90.0;
/// How wide the pilot editor's fields are, in points.
const FIELD_WIDTH: f32 = 140.0;

/// The developer tools windows over a [`ResourceCatalog`]: their own egui
/// context, the resource browser, the preview texture, the query being
/// typed, and the pilot editor.
pub struct DevPanel<C> {
    ctx: Context,
    browser: ResourceBrowser<C>,
    textures: PreviewTextures,
    query: String,
    editor: PilotEditor,
}

impl<C: ResourceCatalog> DevPanel<C> {
    /// A panel browsing `catalog`, whose index it reads once.
    pub fn new(catalog: C) -> Self {
        Self {
            ctx: Context::default(),
            browser: ResourceBrowser::new(catalog),
            textures: PreviewTextures::default(),
            query: String::new(),
            editor: PilotEditor::new(),
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

    /// The pilot editor.
    pub fn editor(&self) -> &PilotEditor {
        &self.editor
    }

    /// Runs one egui frame of the windows over `input`, showing
    /// `overlay`'s frame times and editing the pilot flying through
    /// `desk`, if any, and returns what to paint and whether the pilot
    /// changed.
    pub fn run(
        &mut self,
        input: RawInput,
        overlay: &DevOverlay,
        mut desk: Option<&mut dyn PilotDesk>,
    ) -> (FullOutput, bool) {
        let ctx = self.ctx.clone();
        let mut edited = false;
        let output = ctx.run_ui(input, |ui| {
            Window::new(TITLE)
                .default_pos([16.0, 16.0])
                .default_width(440.0)
                .resizable(true)
                .show(ui.ctx(), |ui| self.contents(ui, overlay));
            Window::new(PILOT_TITLE)
                .default_pos([480.0, 16.0])
                .default_width(320.0)
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    edited |= self.pilot(ui, desk.as_deref_mut());
                });
        });
        (output, edited)
    }

    /// The pilot editor over `desk`; returns whether the pilot changed.
    fn pilot(&mut self, ui: &mut Ui, desk: Option<&mut (dyn PilotDesk + '_)>) -> bool {
        let Some(desk) = desk else {
            self.editor.forget();
            ui.label(NO_PILOT);
            return false;
        };
        self.editor.refresh(desk);
        if self.editor.sheet().is_none() {
            ui.label(NO_PILOT);
            return false;
        }
        let editor = &mut self.editor;
        let mut edited = false;
        ui.label(editor.credits_line());
        if field(ui, editor, Draft::Credits) {
            edited |= editor.apply_credits(desk);
        }
        for reserve in [Reserve::Shield, Reserve::Armor, Reserve::Fuel] {
            ui.label(editor.reserve_line(reserve));
            if field(ui, editor, Draft::Reserve(reserve)) {
                edited |= editor.apply_reserve(desk, reserve);
            }
        }
        ui.label(editor.date_line());
        if field(ui, editor, Draft::Date) {
            edited |= editor.apply_date(desk);
        }
        ui.separator();
        edited |= location(ui, editor, desk);
        ui.separator();
        edited |= bits(ui, editor, desk);
        ui.separator();
        ui.label(editor.status());
        edited
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

/// A field holding `draft`, with its Set button; returns whether to apply
/// it: Set was clicked, or Enter pressed in the field.
fn field(ui: &mut Ui, editor: &mut PilotEditor, draft: Draft) -> bool {
    ui.horizontal(|ui| {
        let response = ui.add(
            TextEdit::singleline(editor.draft_mut(draft))
                .id_salt(("pilot draft", format!("{draft:?}")))
                .desired_width(FIELD_WIDTH),
        );
        let entered =
            response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        let set = ui.button(SET).clicked();
        entered || set
    })
    .inner
}

/// Where the pilot is, and the systems and stellars to move it to;
/// returns whether the pilot changed.
fn location(ui: &mut Ui, editor: &mut PilotEditor, desk: &mut dyn PilotDesk) -> bool {
    ui.label(editor.location_line());
    ui.add(
        TextEdit::singleline(editor.draft_mut(Draft::SystemFilter))
            .id_salt("pilot system filter")
            .hint_text(SYSTEM_HINT),
    );
    let mut system = None;
    ScrollArea::vertical()
        .id_salt("pilot systems")
        .max_height(PLACES_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for place in editor.filtered_systems() {
                let checked = editor.selected_system() == Some(place.id);
                if ui.selectable_label(checked, place.label()).clicked() {
                    system = Some(place.id);
                }
            }
        });
    if let Some(system) = system {
        editor.select_system(desk, system);
    }
    let mut stellar = None;
    ScrollArea::vertical()
        .id_salt("pilot stellars")
        .max_height(PLACES_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for place in editor.stellars() {
                let checked = editor.selected_stellar() == Some(place.id);
                if ui.selectable_label(checked, place.label()).clicked() {
                    stellar = Some(place.id);
                }
            }
        });
    if let Some(stellar) = stellar {
        editor.select_stellar(stellar);
    }
    ui.button(MOVE_TO).clicked() && editor.move_to(desk)
}

/// The control bits: the one the query names, to toggle, and those set,
/// each to clear; returns whether the pilot changed.
fn bits(ui: &mut Ui, editor: &mut PilotEditor, desk: &mut dyn PilotDesk) -> bool {
    ui.add(
        TextEdit::singleline(editor.draft_mut(Draft::BitQuery))
            .id_salt("pilot bit query")
            .hint_text(BIT_HINT),
    );
    let mut toggled = None;
    match editor.bit_lookup() {
        Ok((bit, set)) => {
            let mut checked = set;
            if ui.checkbox(&mut checked, bit_label(bit)).changed() {
                toggled = Some(bit);
            }
        }
        Err(problem) => {
            ui.label(problem);
        }
    }
    ui.label(editor.bits_line());
    let set: Vec<_> = editor
        .sheet()
        .map(|sheet| sheet.bits.clone())
        .unwrap_or_default();
    ScrollArea::vertical()
        .id_salt("pilot bits")
        .max_height(BITS_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for bit in set {
                let mut checked = true;
                if ui.checkbox(&mut checked, bit_label(bit)).changed() {
                    toggled = Some(bit);
                }
            }
        });
    toggled.is_some_and(|bit| editor.toggle_bit(desk, bit))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use egui::epaint::Shape;
    use egui::{Color32, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};
    use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
    use nova_data::graphics::{PICT, RLED};
    use nova_sim::catalog::{StellarId, SystemId};
    use nova_sim::{Bit, GameDate, Gauge};
    use nova_view::devtools::{
        DevOverlay, EditRefusal, Origin, PilotEdit, PilotSheet, Place, RecordView, ResType,
        ResourceCatalog, ResourceDetail, ResourceSummary, SourceInfo,
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

        /// Where each `text` was drawn, top to bottom.
        fn rects(&self, text: &str) -> Vec<Rect> {
            self.texts
                .iter()
                .filter(|(t, _)| t == text)
                .map(|(_, rect)| *rect)
                .collect()
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

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    fn place<Id>(id: Id, name: &str) -> Place<Id> {
        Place {
            id,
            name: name.to_owned(),
        }
    }

    /// Ada, landed on Earth in Sol with 12345 credits, bit 7 set; Sol
    /// and Alpha, holding Proxima; every edit carried out, or refused
    /// with `refusal`, and recorded.
    struct MockDesk {
        refusal: Option<EditRefusal>,
        edits: Vec<PilotEdit>,
    }

    impl MockDesk {
        fn new() -> Self {
            Self {
                refusal: None,
                edits: Vec::new(),
            }
        }
    }

    impl PilotDesk for MockDesk {
        fn sheet(&self) -> Option<PilotSheet> {
            Some(PilotSheet {
                name: "Ada".to_owned(),
                credits: 12_345,
                shield: Gauge::full(100.0),
                armor: Gauge::full(50.0),
                fuel: Gauge::full(300.0),
                date: GameDate::new(1177, 6, 23).expect("a date"),
                date_text: "June 23, 1177 NC".to_owned(),
                system: place(SystemId(130), "Sol"),
                landed: Some(place(StellarId(128), "Earth")),
                bits: vec![bit(7)],
            })
        }

        fn systems(&self) -> Vec<Place<SystemId>> {
            vec![place(SystemId(130), "Sol"), place(SystemId(131), "Alpha")]
        }

        fn stellars(&self, system: SystemId) -> Vec<Place<StellarId>> {
            match system.0 {
                131 => vec![place(StellarId(140), "Proxima")],
                _ => vec![place(StellarId(128), "Earth")],
            }
        }

        fn edit(&mut self, edit: PilotEdit) -> Result<(), EditRefusal> {
            self.edits.push(edit);
            self.refusal.map_or(Ok(()), Err)
        }
    }

    struct Harness {
        panel: DevPanel<Catalog>,
        overlay: DevOverlay,
        /// The pilot flying, if any.
        desk: Option<MockDesk>,
        /// Whether any frame so far said it edited the pilot.
        edited: bool,
    }

    impl Harness {
        /// A panel run twice, so its new window is laid out and shows.
        fn new() -> Self {
            Self::with_desk(None)
        }

        /// [`Harness::new`] over `desk`.
        fn with_desk(desk: Option<MockDesk>) -> Self {
            let mut harness = Self {
                panel: DevPanel::new(Catalog),
                overlay: DevOverlay::new(),
                desk,
                edited: false,
            };
            harness.frame(Vec::new());
            harness.frame(Vec::new());
            harness
        }

        /// The edits the desk received.
        fn edits(&self) -> &[PilotEdit] {
            &self.desk.as_ref().expect("a desk").edits
        }

        fn frame(&mut self, events: Vec<Event>) -> Drawn {
            let desk = self.desk.as_mut().map(|desk| desk as &mut dyn PilotDesk);
            let (output, edited) = self.panel.run(input(events), &self.overlay, desk);
            self.edited |= edited;
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
            self.type_into(HINT, text)
        }

        /// Clicks the field drawn showing `shown` (its text or its hint)
        /// and types `text` into it.
        fn type_into(&mut self, shown: &str, text: &str) -> Drawn {
            let field = self.idle().rect(shown);
            self.click(field);
            self.frame(vec![Event::Text(text.to_owned())]);
            self.idle()
        }

        /// Presses and releases `key`.
        fn press(&mut self, key: egui::Key) -> Drawn {
            let event = |pressed| Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: Modifiers::NONE,
            };
            self.frame(vec![event(true)]);
            self.frame(vec![event(false)]);
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

    // The Pilot window.

    #[test]
    fn with_no_desk_the_pilot_window_says_no_pilot_flies() {
        let drawn = Harness::new().idle();
        assert!(drawn.has(PILOT_TITLE), "{:?}", drawn.all());
        assert!(drawn.has(NO_PILOT), "{:?}", drawn.all());
        assert!(drawn.has(TITLE), "the resource window still shows");
        assert!(drawn.has("3 of 3 resources"), "{:?}", drawn.all());
    }

    #[test]
    fn the_pilot_window_shows_the_sheet() {
        let drawn = Harness::with_desk(Some(MockDesk::new())).idle();
        for text in [
            "Credits 12345",
            "Shield 100 / 100",
            "Armour 50 / 50",
            "Fuel 300 / 300",
            "June 23, 1177 NC",
            "Sol: Earth, landed",
            "Bits set: 7",
            "Bit 7",
            "Sol (130)",
            "Alpha (131)",
        ] {
            assert!(drawn.has(text), "{text:?} in {:?}", drawn.all());
        }
        assert!(!drawn.has(NO_PILOT));
    }

    #[test]
    fn typing_credits_and_enter_sets_them() {
        let mut harness = Harness::with_desk(Some(MockDesk::new()));
        let field = harness.idle().rect("12345");
        harness.click(field);
        harness.press(egui::Key::End);
        harness.frame(vec![Event::Text("6".to_owned())]);
        harness.press(egui::Key::Enter);
        assert_eq!(harness.edits(), [PilotEdit::Credits(123_456)]);
        assert!(harness.edited);
        assert!(harness.idle().has("Credits set to 123456"));
    }

    #[test]
    fn each_set_applies_its_own_field() {
        let mut harness = Harness::with_desk(Some(MockDesk::new()));
        let sets = harness.idle().rects(SET);
        assert_eq!(sets.len(), 5, "credits, three reserves and the date");
        for set in sets {
            harness.click(set);
        }
        assert_eq!(
            harness.edits(),
            [
                PilotEdit::Credits(12_345),
                PilotEdit::Reserve(Reserve::Shield, 100.0),
                PilotEdit::Reserve(Reserve::Armor, 50.0),
                PilotEdit::Reserve(Reserve::Fuel, 300.0),
                PilotEdit::Date(GameDate::new(1177, 6, 23).expect("a date")),
            ]
        );
    }

    #[test]
    fn nothing_typed_edits_nothing() {
        let mut harness = Harness::with_desk(Some(MockDesk::new()));
        harness.idle();
        assert_eq!(harness.edits(), []);
        assert!(!harness.edited);
    }

    #[test]
    fn the_bit_query_toggles_the_bit_it_names() {
        let mut harness = Harness::with_desk(Some(MockDesk::new()));
        let drawn = harness.type_into(BIT_HINT, "42");
        assert!(drawn.has("Bit 42"), "{:?}", drawn.all());
        harness.click_text("Bit 42");
        assert_eq!(harness.edits(), [PilotEdit::Bit(bit(42), true)]);
        assert!(harness.edited);
        let drawn = harness.type_into("42", "000");
        assert!(drawn.has("Bits run 0-9999"), "{:?}", drawn.all());
    }

    #[test]
    fn a_set_bit_in_the_list_clears_it() {
        let mut harness = Harness::with_desk(Some(MockDesk::new()));
        harness.click_text("Bit 7");
        assert_eq!(harness.edits(), [PilotEdit::Bit(bit(7), false)]);
    }

    #[test]
    fn a_move_to_the_stellar_picked_tells_its_refusal() {
        let mut harness = Harness::with_desk(Some(MockDesk {
            refusal: Some(EditRefusal::InFlight),
            ..MockDesk::new()
        }));
        harness.click_text("Alpha (131)");
        harness.click_text("Proxima (140)");
        let drawn = harness.click_text(MOVE_TO);
        assert_eq!(
            harness.edits(),
            [PilotEdit::MoveTo {
                system: SystemId(131),
                stellar: StellarId(140)
            }]
        );
        assert!(!harness.edited, "refused");
        assert!(
            drawn.has("Refused: the ship must be landed to move"),
            "{:?}",
            drawn.all()
        );
    }

    #[test]
    fn the_system_filter_narrows_the_list() {
        let mut harness = Harness::with_desk(Some(MockDesk::new()));
        let drawn = harness.type_into(SYSTEM_HINT, "alp");
        assert!(drawn.has("Alpha (131)"), "{:?}", drawn.all());
        assert!(!drawn.has("Sol (130)"), "{:?}", drawn.all());
    }

    #[test]
    fn a_pilot_gone_is_forgotten() {
        let mut harness = Harness::with_desk(Some(MockDesk::new()));
        harness.desk = None;
        let drawn = harness.idle();
        assert!(drawn.has(NO_PILOT), "{:?}", drawn.all());
        assert!(harness.panel.editor().sheet().is_none());
    }
}
