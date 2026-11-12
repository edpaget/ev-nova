//! The Preferences dialog's labels measured with the bundled fonts'
//! real glyphs ([`GlyphonMetrics`]), which only this crate has: each
//! fits the row nova-view lays out for it over the stock "new prefs
//! dialog" (`DITL` 4003).

use std::rc::Rc;

use nova_render::FontFaces;
use nova_render::wgpu::GlyphonMetrics;
use nova_view::preferences::Prefs;
use nova_view::text::TextMetrics;
use nova_view::ui::PrefsDialog;
use nova_view::ui::button::ButtonStyle;
use nova_view::ui::prefs::fixture::stock_template;
use nova_view::ui::volume::TEXT_SIZE;
use nova_view::{Font, RenderScale, Scaling};

#[test]
fn the_widest_scaling_label_fits_its_row() {
    let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
    let dialog = PrefsDialog::new(
        &stock_template(),
        Prefs::default(),
        ButtonStyle::STOCK,
        Rc::clone(&metrics) as Rc<dyn TextMetrics>,
    )
    .expect("builds");
    let row = dialog.scaling().rects().value;
    for scaling in Scaling::ALL {
        let width = metrics.width(Font::Geneva, TEXT_SIZE, scaling.label());
        assert!(
            width <= row.width(),
            "{scaling:?}: {width} in {}",
            row.width()
        );
    }
}

/// The render-scale list's row is "Check For Updates"'s own area, narrower
/// than the Scaling list's: every label it can show fits, the percentages
/// it steps through and one the settings file holds between them.
#[test]
fn the_widest_render_scale_label_fits_its_row() {
    let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
    let dialog = PrefsDialog::new(
        &stock_template(),
        Prefs::default(),
        ButtonStyle::STOCK,
        Rc::clone(&metrics) as Rc<dyn TextMetrics>,
    )
    .expect("builds");
    let row = dialog.render_scale().rects().value;
    let listed = RenderScale::list(RenderScale::Percent(88));
    assert!(listed.contains(&RenderScale::Percent(RenderScale::MAX)));
    for scale in listed {
        let width = metrics.width(Font::Geneva, TEXT_SIZE, &scale.label());
        assert!(
            width > 0.0 && width <= row.width(),
            "{scale:?}: {width} in {}",
            row.width()
        );
    }
}
