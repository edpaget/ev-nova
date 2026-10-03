//! The resource browser: the index read once, a search over it, and the
//! selected resource with its record, files and preview.

use nova_data::graphics::Image;

use super::catalog::{
    Origin, RecordView, ResType, ResourceCatalog, ResourceDetail, ResourceSummary, SourceInfo,
    type_code,
};
use super::preview::{Preview, preview};
use super::search::{Query, fold};

/// One index entry, with its type code and name folded once for search.
#[derive(Clone, Debug)]
struct IndexEntry {
    summary: ResourceSummary,
    ty_folded: String,
    name_folded: String,
}

/// Browses every resource of a [`ResourceCatalog`]: searches the index and
/// inspects the selected resource.
#[derive(Debug)]
pub struct ResourceBrowser<C> {
    catalog: C,
    index: Vec<IndexEntry>,
    query: String,
    /// Indexes into `index`, in result order.
    results: Vec<usize>,
    selection: Option<Selection>,
}

impl<C: ResourceCatalog> ResourceBrowser<C> {
    /// Reads `catalog`'s index once. The query starts empty, so every
    /// resource is a result, and nothing is selected.
    pub fn new(catalog: C) -> Self {
        let index: Vec<IndexEntry> = catalog
            .resources()
            .into_iter()
            .map(|summary| IndexEntry {
                ty_folded: fold(&summary.ty.to_string()),
                name_folded: fold(summary.name.as_deref().unwrap_or_default()),
                summary,
            })
            .collect();
        let results = (0..index.len()).collect();
        Self {
            catalog,
            index,
            query: String::new(),
            results,
            selection: None,
        }
    }

    /// How many resources the index holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Whether the index is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// The current query, as typed.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// The resources matching the query, best first.
    pub fn results(&self) -> impl ExactSizeIterator<Item = &ResourceSummary> + '_ {
        self.results.iter().map(|&i| &self.index[i].summary)
    }

    /// How many resources match the query.
    #[must_use]
    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    /// How many resources match the query, out of how many.
    #[must_use]
    pub fn count_line(&self) -> String {
        format!("{} of {} resources", self.result_count(), self.len())
    }

    /// The `i`th result.
    #[must_use]
    pub fn result(&self, i: usize) -> Option<&ResourceSummary> {
        self.results.get(i).map(|&i| &self.index[i].summary)
    }

    /// The selected resource.
    #[must_use]
    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// The selected resource, to choose its preview frame.
    pub fn selection_mut(&mut self) -> Option<&mut Selection> {
        self.selection.as_mut()
    }

    /// Searches for `query` (see [`super::search`]) unless it is the
    /// current one; returns whether it changed. The selection stays, even
    /// if the new results leave it out.
    pub fn set_query(&mut self, query: &str) -> bool {
        if query == self.query {
            return false;
        }
        query.clone_into(&mut self.query);
        let parsed = Query::parse(query);
        let mut ranked: Vec<(u32, usize)> = self
            .index
            .iter()
            .enumerate()
            .filter_map(|(i, entry)| {
                parsed
                    .rank(&entry.ty_folded, entry.summary.id, &entry.name_folded)
                    .map(|rank| (rank, i))
            })
            .collect();
        ranked.sort_by_key(|&(rank, _)| rank);
        self.results = ranked.into_iter().map(|(_, i)| i).collect();
        true
    }

    /// Selects the `i`th result, as [`ResourceBrowser::select`] does; past
    /// the results, clears the selection and returns `false`.
    pub fn select_result(&mut self, i: usize) -> bool {
        if let Some(summary) = self.result(i) {
            let (ty, id) = (summary.ty, summary.id);
            self.select(ty, id)
        } else {
            self.selection = None;
            false
        }
    }

    /// Selects resource (`ty`, `id`), inspecting it unless it is already
    /// selected, and previews its first frame. A resource that is not in
    /// the index, or that the catalog no longer has, clears the selection
    /// and returns `false`.
    pub fn select(&mut self, ty: ResType, id: i16) -> bool {
        if self
            .selection
            .as_ref()
            .is_some_and(|s| (s.summary.ty, s.summary.id) == (ty, id))
        {
            return true;
        }
        self.selection = self
            .index
            .iter()
            .find(|entry| (entry.summary.ty, entry.summary.id) == (ty, id))
            .and_then(|entry| {
                let detail = self.catalog.inspect(ty, id)?;
                Some(Selection::new(entry.summary.clone(), detail))
            });
        self.selection.is_some()
    }
}

/// The selected resource: what the catalog said about it, its preview and
/// which frame of it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    summary: ResourceSummary,
    detail: ResourceDetail,
    preview: Preview,
    frame: usize,
}

impl Selection {
    fn new(summary: ResourceSummary, detail: ResourceDetail) -> Self {
        let preview = preview(summary.ty, &detail.data);
        Self {
            summary,
            detail,
            preview,
            frame: 0,
        }
    }

    /// The resource's type, ID and name.
    #[must_use]
    pub fn summary(&self) -> &ResourceSummary {
        &self.summary
    }

    /// Its bytes, files and record.
    #[must_use]
    pub fn detail(&self) -> &ResourceDetail {
        &self.detail
    }

    /// Its image preview.
    #[must_use]
    pub fn preview(&self) -> &Preview {
        &self.preview
    }

    /// How many frames the preview has: 0 without frames.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        match &self.preview {
            Preview::Frames(frames) => frames.len(),
            _ => 0,
        }
    }

    /// The frame showing, from 0.
    #[must_use]
    pub fn frame(&self) -> usize {
        self.frame
    }

    /// The last frame: one before the count, and 0 without frames.
    #[must_use]
    pub fn last_frame(&self) -> usize {
        self.frame_count().saturating_sub(1)
    }

    /// Shows frame `frame`, clamped to the last.
    pub fn set_frame(&mut self, frame: usize) {
        self.frame = frame.min(self.last_frame());
    }

    /// The frame showing, if the preview has frames.
    #[must_use]
    pub fn image(&self) -> Option<&Image> {
        match &self.preview {
            Preview::Frames(frames) => frames.get(self.frame),
            _ => None,
        }
    }

    /// The type, the ID, and the name in quotes or "(unnamed)".
    #[must_use]
    pub fn heading(&self) -> String {
        let ty = type_code(self.summary.ty);
        match &self.summary.name {
            Some(name) => format!("{ty} {} “{name}”", self.summary.id),
            None => format!("{ty} {} (unnamed)", self.summary.id),
        }
    }

    /// The file it came from, and which folder that is in.
    #[must_use]
    pub fn file_line(&self) -> String {
        format!("from {}", file_text(&self.detail.source))
    }

    /// One line for each earlier file it overrides, in load order.
    #[must_use]
    pub fn overridden_lines(&self) -> Vec<String> {
        self.detail
            .overridden
            .iter()
            .map(|file| format!("overrides {}", file_text(file)))
            .collect()
    }

    /// Its size in bytes.
    #[must_use]
    pub fn size_line(&self) -> String {
        format!("{} bytes", self.detail.data.len())
    }

    /// The record's pretty JSON, or why there is none.
    #[must_use]
    pub fn record_text(&self) -> String {
        match &self.detail.record {
            RecordView::Json { text, .. } => text.clone(),
            RecordView::NotARecord => {
                format!("{} has no record layout", type_code(self.summary.ty))
            }
            RecordView::Error(error) => format!("does not decode: {error}"),
        }
    }

    /// The decoder's warning, if it raised one.
    #[must_use]
    pub fn warning_line(&self) -> Option<String> {
        match &self.detail.record {
            RecordView::Json {
                warning: Some(warning),
                ..
            } => Some(format!("warning: {warning}")),
            _ => None,
        }
    }

    /// What the preview shows: a picture's size, a sheet's frame and size,
    /// or why there is no image; `None` for a type with no preview.
    #[must_use]
    pub fn preview_caption(&self) -> Option<String> {
        match &self.preview {
            Preview::None => None,
            Preview::Frames(frames) => {
                let size = self.image().map_or_else(String::new, |image| {
                    format!("{} × {} pixels", image.width(), image.height())
                });
                Some(if frames.len() == 1 {
                    size
                } else {
                    format!("frame {} of {}, {size}", self.frame + 1, frames.len())
                })
            }
            Preview::TooLarge { width, height } => {
                Some(format!("too large to preview: {width} × {height} pixels"))
            }
            Preview::Failed(error) => Some(format!("image does not decode: {error}")),
        }
    }
}

fn file_text(file: &SourceInfo) -> String {
    let folder = match file.origin {
        Origin::Data => "data folder",
        Origin::PlugIn => "plug-in",
    };
    format!("{} ({folder})", file.path.display())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
    use nova_data::graphics::{PICT, RLED};

    use super::*;
    use crate::devtools::catalog::{
        Origin, RecordView, ResType, ResourceCatalog, ResourceDetail, ResourceSummary, SourceInfo,
    };

    const SPIN: ResType = ResType::new([b's', b'p', 0x95, b'n']);
    const SND: ResType = ResType::new(*b"snd ");
    const GOVT: ResType = ResType::new([b'g', 0x9A, b'v', b't']);

    fn summary(ty: ResType, id: i16, name: Option<&str>) -> ResourceSummary {
        ResourceSummary {
            ty,
            id,
            name: name.map(str::to_owned),
        }
    }

    fn file(path: &str, origin: Origin) -> SourceInfo {
        SourceInfo {
            path: PathBuf::from(path),
            origin,
        }
    }

    /// A 4 × 2 picture.
    fn picture() -> Vec<u8> {
        let bounds = [0, 0, 2, 4];
        PictBuilder::new(bounds)
            .direct_bits(&DirectBits::rgb555(bounds, &[0x7C00; 8]))
            .end()
            .build()
    }

    /// A sheet of 3 frames of 6 × 5, frame `n` all colour `n`.
    fn sheet() -> Vec<u8> {
        (0..3u16)
            .fold(RledBuilder::new(6, 5), |sheet, n| {
                sheet.frame(|f| (0..5).fold(f, |f, _| f.line().pixels(&[0x0400 * n; 6])))
            })
            .build()
    }

    /// Canned summaries and details; records every `inspect`.
    struct FakeCatalog {
        summaries: Vec<ResourceSummary>,
        details: Vec<((ResType, i16), ResourceDetail)>,
        inspected: RefCell<Vec<(ResType, i16)>>,
        listed: RefCell<usize>,
    }

    impl ResourceCatalog for FakeCatalog {
        fn resources(&self) -> Vec<ResourceSummary> {
            *self.listed.borrow_mut() += 1;
            self.summaries.clone()
        }

        fn inspect(&self, ty: ResType, id: i16) -> Option<ResourceDetail> {
            self.inspected.borrow_mut().push((ty, id));
            self.details
                .iter()
                .find(|(key, _)| *key == (ty, id))
                .map(|(_, detail)| detail.clone())
        }
    }

    fn detail(data: Vec<u8>, record: RecordView) -> ResourceDetail {
        ResourceDetail {
            data,
            source: file("/data/Nova Data 1", Origin::Data),
            overridden: Vec::new(),
            record,
        }
    }

    fn catalog() -> FakeCatalog {
        FakeCatalog {
            summaries: vec![
                summary(PICT, 128, Some("Station 200")),
                summary(PICT, 200, Some("Planet")),
                summary(GOVT, 128, Some("Fédération")),
                summary(RLED, 128, Some("Shuttle")),
                summary(RLED, 200, None),
                summary(SND, 200, Some("Beep")),
                summary(SPIN, 300, None),
            ],
            details: vec![
                ((PICT, 128), detail(picture(), RecordView::NotARecord)),
                (
                    (RLED, 128),
                    ResourceDetail {
                        data: sheet(),
                        source: file("/plug/Ships/Over", Origin::PlugIn),
                        overridden: vec![
                            file("/data/Nova Data 1", Origin::Data),
                            file("/data/Nova Data 2", Origin::Data),
                        ],
                        record: RecordView::NotARecord,
                    },
                ),
                (
                    (GOVT, 128),
                    detail(
                        vec![0; 192],
                        RecordView::Json {
                            text: "{\n  \"x\": 1\n}".to_owned(),
                            warning: Some("4 trailing bytes ignored".to_owned()),
                        },
                    ),
                ),
                (
                    (SPIN, 300),
                    detail(vec![0; 5], RecordView::Error("unexpected end".to_owned())),
                ),
                ((SND, 200), detail(vec![1, 2, 3], RecordView::NotARecord)),
            ],
            inspected: RefCell::new(Vec::new()),
            listed: RefCell::new(0),
        }
    }

    fn keys<C: ResourceCatalog>(browser: &ResourceBrowser<C>) -> Vec<(ResType, i16)> {
        browser.results().map(|s| (s.ty, s.id)).collect()
    }

    #[test]
    fn the_index_is_read_once_in_the_catalogs_order() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        assert_eq!(browser.len(), 7);
        assert!(!browser.is_empty());
        assert_eq!(browser.query(), "");
        assert_eq!(browser.result_count(), 7);
        assert_eq!(
            browser.results().cloned().collect::<Vec<_>>(),
            catalog.summaries
        );
        browser.set_query("rled");
        browser.set_query("");
        assert_eq!(*catalog.listed.borrow(), 1);
        assert_eq!(catalog.inspected.borrow().len(), 0);
    }

    #[test]
    fn an_empty_catalog_is_empty() {
        let catalog = FakeCatalog {
            summaries: Vec::new(),
            ..catalog()
        };
        let browser = ResourceBrowser::new(&catalog);
        assert!(browser.is_empty());
        assert_eq!(browser.len(), 0);
        assert_eq!(browser.result(0), None);
    }

    #[test]
    fn queries_rank_structural_matches_first() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        assert!(browser.set_query("rleD 128"));
        assert_eq!(browser.query(), "rleD 128");
        assert_eq!(keys(&browser), [(RLED, 128)]);
        assert!(browser.set_query("200"));
        assert_eq!(
            keys(&browser),
            [(PICT, 200), (RLED, 200), (SND, 200), (PICT, 128)]
        );
        assert!(browser.set_query("FEDERATION"));
        assert_eq!(keys(&browser), [(GOVT, 128)]);
        assert!(browser.set_query("pict"));
        assert_eq!(keys(&browser), [(PICT, 128), (PICT, 200)]);
        assert!(browser.set_query("snd"));
        assert_eq!(keys(&browser), [(SND, 200)]);
        assert!(browser.set_query("nothing at all"));
        assert_eq!(keys(&browser), []);
        assert_eq!(browser.result_count(), 0);
    }

    #[test]
    fn the_count_line_gives_the_results_and_the_index_size() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        assert_eq!(browser.count_line(), "7 of 7 resources");
        browser.set_query("200");
        assert_eq!(browser.count_line(), "4 of 7 resources");
    }

    #[test]
    fn result_reads_one_result_in_order() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        browser.set_query("200");
        assert_eq!(browser.result(0), Some(&summary(PICT, 200, Some("Planet"))));
        assert_eq!(
            browser.result(3),
            Some(&summary(PICT, 128, Some("Station 200")))
        );
        assert_eq!(browser.result(4), None);
    }

    #[test]
    fn an_unchanged_query_does_nothing() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        assert!(!browser.set_query(""));
        assert!(browser.set_query("rled"));
        assert!(!browser.set_query("rled"));
        assert_eq!(keys(&browser), [(RLED, 128), (RLED, 200)]);
    }

    #[test]
    fn selecting_a_result_inspects_it_once() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        browser.set_query("fed");
        assert!(browser.select_result(0));
        let selection = browser.selection().expect("selected");
        assert_eq!(selection.summary(), &summary(GOVT, 128, Some("Fédération")));
        assert_eq!(selection.detail().data, [0; 192]);
        assert!(browser.select_result(0));
        assert!(browser.select(GOVT, 128));
        assert_eq!(*catalog.inspected.borrow(), [(GOVT, 128)]);
    }

    #[test]
    fn selecting_past_the_results_selects_nothing() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        browser.select(GOVT, 128);
        browser.set_query("fed");
        assert!(!browser.select_result(1));
        assert!(browser.selection().is_none());
        assert_eq!(*catalog.inspected.borrow(), [(GOVT, 128)]);
    }

    #[test]
    fn selecting_by_type_and_id_inspects_it() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        assert!(browser.select(SND, 200));
        assert!(browser.select(PICT, 128));
        assert_eq!(
            browser.selection().map(Selection::summary),
            Some(&summary(PICT, 128, Some("Station 200")))
        );
        assert_eq!(*catalog.inspected.borrow(), [(SND, 200), (PICT, 128)]);
    }

    #[test]
    fn the_selection_survives_a_query_that_hides_it() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        browser.select(SND, 200);
        browser.set_query("rled");
        assert_eq!(
            browser.selection().map(|s| s.summary().id),
            Some(200),
            "still selected"
        );
    }

    #[test]
    fn a_resource_the_catalog_no_longer_has_clears_the_selection() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        browser.select(SND, 200);
        assert!(!browser.select(PICT, 200), "listed but not inspectable");
        assert!(browser.selection().is_none());
        browser.select(SND, 200);
        assert!(!browser.select(PICT, 999), "not even listed");
        assert!(browser.selection().is_none());
    }

    #[test]
    fn a_picture_previews_its_one_frame() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        browser.select(PICT, 128);
        let selection = browser.selection().expect("selected");
        let Preview::Frames(frames) = selection.preview() else {
            panic!("no frames: {:?}", selection.preview());
        };
        assert_eq!(frames.len(), 1);
        assert_eq!(selection.frame_count(), 1);
        assert_eq!(selection.image(), Some(&frames[0]));
        assert_eq!(
            selection.image().map(|i| (i.width(), i.height())),
            Some((4, 2))
        );
    }

    #[test]
    fn a_sheet_previews_one_frame_at_a_time() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        browser.select(RLED, 128);
        let selection = browser.selection_mut().expect("selected");
        let Preview::Frames(frames) = selection.preview().clone() else {
            panic!("no frames");
        };
        assert_eq!(selection.frame_count(), 3);
        assert_eq!(selection.frame(), 0);
        assert_eq!(selection.image(), Some(&frames[0]));
        selection.set_frame(2);
        assert_eq!(selection.frame(), 2);
        assert_eq!(selection.image(), Some(&frames[2]));
        selection.set_frame(1);
        assert_eq!(selection.image(), Some(&frames[1]));
        selection.set_frame(99);
        assert_eq!(selection.frame(), 2);
        assert_eq!(selection.image(), Some(&frames[2]));
    }

    #[test]
    fn the_last_frame_is_one_before_the_count() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        assert_eq!(selected(&mut browser, RLED, 128).last_frame(), 2);
        assert_eq!(selected(&mut browser, PICT, 128).last_frame(), 0);
        assert_eq!(selected(&mut browser, SND, 200).last_frame(), 0);
    }

    #[test]
    fn a_new_selection_starts_at_frame_zero() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        browser.select(RLED, 128);
        browser.selection_mut().expect("selected").set_frame(2);
        browser.select(PICT, 128);
        browser.select(RLED, 128);
        assert_eq!(browser.selection().map(Selection::frame), Some(0));
    }

    #[test]
    fn a_resource_without_frames_has_no_image() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        browser.select(SND, 200);
        let selection = browser.selection_mut().expect("selected");
        assert_eq!(selection.preview(), &Preview::None);
        assert_eq!(selection.frame_count(), 0);
        selection.set_frame(3);
        assert_eq!(selection.frame(), 0);
        assert_eq!(selection.image(), None);
        assert_eq!(selection.preview_caption(), None);
    }

    fn selected(browser: &mut ResourceBrowser<&FakeCatalog>, ty: ResType, id: i16) -> Selection {
        assert!(browser.select(ty, id));
        browser.selection().expect("selected").clone()
    }

    #[test]
    fn the_heading_names_the_type_id_and_name() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        assert_eq!(
            selected(&mut browser, GOVT, 128).heading(),
            "gövt 128 “Fédération”"
        );
        assert_eq!(
            selected(&mut browser, SPIN, 300).heading(),
            "spïn 300 (unnamed)"
        );
        assert_eq!(selected(&mut browser, SND, 200).heading(), "snd 200 “Beep”");
    }

    #[test]
    fn the_file_lines_name_each_file_and_its_folder() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        let sheet = selected(&mut browser, RLED, 128);
        assert_eq!(sheet.file_line(), "from /plug/Ships/Over (plug-in)");
        assert_eq!(
            sheet.overridden_lines(),
            [
                "overrides /data/Nova Data 1 (data folder)",
                "overrides /data/Nova Data 2 (data folder)",
            ]
        );
        let sound = selected(&mut browser, SND, 200);
        assert_eq!(sound.file_line(), "from /data/Nova Data 1 (data folder)");
        assert_eq!(sound.overridden_lines(), Vec::<String>::new());
    }

    #[test]
    fn the_size_line_counts_the_bytes() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        assert_eq!(selected(&mut browser, SND, 200).size_line(), "3 bytes");
        assert_eq!(selected(&mut browser, GOVT, 128).size_line(), "192 bytes");
    }

    #[test]
    fn the_record_text_is_the_json_or_says_why_there_is_none() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        let govt = selected(&mut browser, GOVT, 128);
        assert_eq!(govt.record_text(), "{\n  \"x\": 1\n}");
        assert_eq!(
            govt.warning_line().as_deref(),
            Some("warning: 4 trailing bytes ignored")
        );
        let sound = selected(&mut browser, SND, 200);
        assert_eq!(sound.record_text(), "snd has no record layout");
        assert_eq!(sound.warning_line(), None);
        let spin = selected(&mut browser, SPIN, 300);
        assert_eq!(spin.record_text(), "does not decode: unexpected end");
        assert_eq!(spin.warning_line(), None);
    }

    #[test]
    fn the_preview_caption_describes_the_preview() {
        let catalog = catalog();
        let mut browser = ResourceBrowser::new(&catalog);
        assert_eq!(
            selected(&mut browser, PICT, 128)
                .preview_caption()
                .as_deref(),
            Some("4 × 2 pixels")
        );
        browser.select(RLED, 128);
        let sheet = browser.selection_mut().expect("selected");
        assert_eq!(
            sheet.preview_caption().as_deref(),
            Some("frame 1 of 3, 6 × 5 pixels")
        );
        sheet.set_frame(2);
        assert_eq!(
            sheet.preview_caption().as_deref(),
            Some("frame 3 of 3, 6 × 5 pixels")
        );
    }

    #[test]
    fn the_caption_says_when_a_preview_is_too_large_or_failed() {
        let mut selection = selected(&mut ResourceBrowser::new(&catalog()), SND, 200);
        selection.preview = Preview::TooLarge {
            width: 5000,
            height: 7,
        };
        assert_eq!(
            selection.preview_caption().as_deref(),
            Some("too large to preview: 5000 × 7 pixels")
        );
        assert_eq!(selection.image(), None);
        selection.preview = Preview::Failed("bad opcode".to_owned());
        assert_eq!(
            selection.preview_caption().as_deref(),
            Some("image does not decode: bad opcode")
        );
        assert_eq!(selection.image(), None);
        assert_eq!(selection.frame_count(), 0);
    }
}
