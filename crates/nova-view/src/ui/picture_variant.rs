//! The picture variant of a dialog: a plain `DLOG` and one with a
//! picture, the second chosen when a `dësc` (or record) `Graphic` is above
//! [`LAST_UNSHOWN_GRAPHIC`].
//!
//! The text dialog ([`DescTemplates`](crate::ui::desc::DescTemplates)),
//! the mission offer
//! ([`OfferTemplates`](crate::spaceport::offer::OfferTemplates)), the bar
//! ([`BarTemplates`](crate::spaceport::bar::BarTemplates)) and the
//! shipyard's info panel
//! ([`InfoTemplates`](crate::spaceport::shipyard::InfoTemplates)) each
//! hold their two templates and share the choice through
//! [`VariantTemplates`]. They draw the picture with
//! [`Dialog::draw_stretched_over`](crate::ui::dialog::Dialog::draw_stretched_over)
//! and [`Dialog::draw_picture_in`](crate::ui::dialog::Dialog::draw_picture_in),
//! each in its own order.
//!
//! The four stay separate types, so one screen's templates can't be
//! handed to another:
//!
//! ```compile_fail
//! use nova_view::spaceport::offer::OfferTemplates;
//! use nova_view::ui::desc::DescTemplates;
//!
//! fn text(_: DescTemplates) {}
//! text(OfferTemplates {
//!     plain: Err(String::new()),
//!     picture: Err(String::new()),
//! });
//! ```
//!
//! while each still takes its own:
//!
//! ```
//! use nova_view::ui::desc::DescTemplates;
//!
//! fn text(_: DescTemplates) {}
//! text(DescTemplates {
//!     plain: Err(String::new()),
//!     picture: Err(String::new()),
//! });
//! ```

use super::dialog::DialogTemplate;

/// The highest `dësc` `Graphic` that shows no picture: the original shows
/// one only above it (`cmpw $0x7f` in `_ShowTextDialog` @0x5a89a,
/// `_OfferOneMission` @0xa2344, `_DoBarDialog` @0x48faa and
/// `_DoShipyardDescription` @0x5b6db-0x5b6e4).
pub const LAST_UNSHOWN_GRAPHIC: i16 = 127;

/// The picture `graphic` shows: itself when above
/// [`LAST_UNSHOWN_GRAPHIC`], else none.
#[must_use]
pub fn shown_graphic(graphic: Option<i16>) -> Option<i16> {
    graphic.filter(|&id| id > LAST_UNSHOWN_GRAPHIC)
}

/// A screen's two dialog templates, plain and with a picture, or why each
/// is missing, and the choice between them.
pub trait VariantTemplates {
    /// The plain dialog's template, or why it is missing.
    fn plain_template(&self) -> &Result<DialogTemplate, String>;

    /// The picture dialog's template, or why it is missing.
    fn picture_template(&self) -> &Result<DialogTemplate, String>;

    /// The picture variant's template when `picture`, else the plain one.
    fn template(&self, picture: bool) -> &Result<DialogTemplate, String> {
        if picture {
            self.picture_template()
        } else {
            self.plain_template()
        }
    }

    /// The graphic shown for a `Graphic` of `graphic` ([`shown_graphic`]),
    /// and the template of the variant it picks: on the ID alone, as the
    /// text and offer dialogs pick it.
    fn pick(&self, graphic: Option<i16>) -> (Option<i16>, &Result<DialogTemplate, String>) {
        let shown = shown_graphic(graphic);
        (shown, self.template(shown.is_some()))
    }

    /// As [`pick`](Self::pick), but the picture variant only when `exists`
    /// says its `PICT` loads, as the bar picks it; otherwise the plain
    /// variant and no graphic. `exists` is asked only above
    /// [`LAST_UNSHOWN_GRAPHIC`].
    fn pick_loaded(
        &self,
        graphic: Option<i16>,
        exists: impl Fn(i16) -> bool,
    ) -> (Option<i16>, &Result<DialogTemplate, String>) {
        let shown = shown_graphic(graphic).filter(|&id| exists(id));
        (shown, self.template(shown.is_some()))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    /// Templates told apart by why each is missing.
    struct Fake {
        plain: Result<DialogTemplate, String>,
        picture: Result<DialogTemplate, String>,
    }

    impl VariantTemplates for Fake {
        fn plain_template(&self) -> &Result<DialogTemplate, String> {
            &self.plain
        }

        fn picture_template(&self) -> &Result<DialogTemplate, String> {
            &self.picture
        }
    }

    fn fake() -> Fake {
        Fake {
            plain: Err("plain".into()),
            picture: Err("picture".into()),
        }
    }

    fn which(template: &Result<DialogTemplate, String>) -> &str {
        template.as_ref().err().map_or("loaded", String::as_str)
    }

    #[test]
    fn template_is_the_picture_one_only_for_a_picture() {
        let templates = fake();
        assert_eq!(which(templates.template(false)), "plain");
        assert_eq!(which(templates.template(true)), "picture");
    }

    #[test]
    fn pick_takes_the_picture_variant_above_127_on_the_id_alone() {
        let templates = fake();
        for graphic in [None, Some(-1), Some(0), Some(127)] {
            let (shown, template) = templates.pick(graphic);
            assert_eq!((shown, which(template)), (None, "plain"), "{graphic:?}");
        }
        for id in [128, 5008, i16::MAX] {
            let (shown, template) = templates.pick(Some(id));
            assert_eq!((shown, which(template)), (Some(id), "picture"), "{id}");
        }
    }

    #[test]
    fn pick_loaded_takes_the_picture_variant_only_when_its_pict_loads() {
        let templates = fake();
        let (shown, template) = templates.pick_loaded(Some(5008), |id| id == 5008);
        assert_eq!((shown, which(template)), (Some(5008), "picture"));
        let (shown, template) = templates.pick_loaded(Some(5008), |_| false);
        assert_eq!((shown, which(template)), (None, "plain"));
    }

    #[test]
    fn pick_loaded_asks_whether_the_pict_loads_only_above_127() {
        let templates = fake();
        let asked = RefCell::new(Vec::new());
        let exists = |id| {
            asked.borrow_mut().push(id);
            true
        };
        for graphic in [None, Some(0), Some(127)] {
            let (shown, template) = templates.pick_loaded(graphic, exists);
            assert_eq!((shown, which(template)), (None, "plain"), "{graphic:?}");
        }
        assert_eq!(*asked.borrow(), Vec::<i16>::new());
        let _ = templates.pick_loaded(Some(128), exists);
        assert_eq!(*asked.borrow(), vec![128]);
    }
}
