//! The spaceport over a backed template (the new game's, which has no
//! `BACKGROUND` picture): the template's backing is drawn first, under
//! the background, the landscape, the name bar and the items.

use super::*;
use crate::ui::panel::PanelBacking;
use crate::ui::panel::testing::assert_backed_first;

#[test]
fn a_backed_spaceport_draws_its_backing_under_everything_else() {
    let backed = DialogTemplate {
        backing: Some(PanelBacking::NEW_GAME),
        ..template()
    };
    let view = SpaceportView::new(
        &catalog(),
        StellarId(140),
        Ok((backed, Rc::new(MonoMetrics))),
    );
    let bounds = view.dialog().expect("laid out").bounds();
    assert_backed_first(&drawn(&view), bounds);
}
