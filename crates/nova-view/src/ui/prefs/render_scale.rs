//! Preferences' render-scale list: a [`Picker`] of [`RenderScale`]s in
//! check box 22's row ("Check For Updates" in the stock dialog).
//!
//! - **Its row** is the item's own bounds, not run on to the dialog's right
//!   margin as the display picker's and the Scaling list's are: "Sound
//!   Volume:" starts on the same line, in the right column.
//! - **Its entries** are [`RenderScale::list`] of the scale the dialog
//!   opened on, each labelled "Render Scale: " and Auto or its percentage,
//!   as the row has no caption of its own.
//! - **Each step is a change**, reported at once as a volume's is
//!   ([`PrefsDialog::take_change`]), the dialog still open. Whoever opened
//!   the dialog sets it on the renderer
//!   (`nova_render::Renderer::set_render_scale`), which rebuilds nothing,
//!   so the world behind the dialog changes as the player steps. That is
//!   why it need not wait for the dialog to close as the Scaling list does,
//!   and why it shows no relaunch notice.
//!
//! The original has no render scale: this is the engine's own control, as
//! the Scaling list and the display picker are.

use crate::render_scale::RenderScale;

use super::super::picker::Picker;
use super::PrefsDialog;

/// The render-scale list's item ("Check For Updates" in the stock dialog).
pub const RENDER_SCALE_ITEM: usize = 22;

impl PrefsDialog {
    /// The render-scale list.
    #[must_use]
    pub fn render_scale(&self) -> &Picker<RenderScale> {
        &self.render_scale
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Whole pixels.
mod tests {
    use std::rc::Rc;

    use super::super::tests::{button, click, drawn, key, placed, prefs, start, template};
    use super::super::{Focus, KEY_SETTINGS_ITEM, NOTICE_AREA, OK_ITEM};
    use super::*;
    use crate::draw::{DrawCommand, DrawList};
    use crate::input::{Input, Key};
    use crate::preferences::Prefs;
    use crate::scaling::Scaling;
    use crate::screen::Screen;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::button::ButtonStyle;
    use crate::ui::picker::PickerRects;
    use crate::ui::toggle::Toggle;
    use crate::ui::volume::Arrow;

    const AUTO: RenderScale = RenderScale::Auto;
    const P: fn(u8) -> RenderScale = RenderScale::Percent;

    /// The dialog opened drawing at `render_scale`, the rest as `start`.
    fn opened_on(render_scale: RenderScale) -> PrefsDialog {
        PrefsDialog::new(
            &template(),
            Prefs {
                render_scale,
                ..start()
            },
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds")
    }

    /// Tabs to the render-scale list.
    fn focus_it(dialog: &mut PrefsDialog) {
        for _ in 0..11 {
            dialog.input(&key(Key::Tab));
        }
        assert_eq!(dialog.focus(), Some(Focus::RenderScale));
    }

    #[test]
    fn it_takes_check_for_updates_own_area() {
        let dialog = prefs();
        assert_eq!(RENDER_SCALE_ITEM, 22);
        assert_eq!(
            dialog.render_scale().rects(),
            PickerRects::in_row(placed(11.0, 165.0, 156.0, 183.0)),
            "the item's own bounds: Sound Volume: shares the line"
        );
        assert_eq!(
            dialog.render_scale().rects().up,
            placed(12.0, 165.0, 23.0, 174.0)
        );
        assert!(dialog.inert().is_empty(), "no box is left greyed");
    }

    #[test]
    fn it_is_clear_of_every_other_part() {
        let dialog = prefs().with_scaling(&Scaling::ALL);
        let list = dialog.render_scale().rects().bounds();
        let ok = dialog.dialog().item_bounds(OK_ITEM).expect("OK");
        let keys = dialog
            .dialog()
            .item_bounds(KEY_SETTINGS_ITEM)
            .expect("Key Settings");
        let (left, top, right, bottom) = NOTICE_AREA;
        let effects = dialog.effects_volume().rects();
        let parts = [
            dialog.music(),
            dialog.sound(),
            dialog.hyperspace_effects(),
            dialog.quicktime_movies(),
            dialog.ship_animations(),
            dialog.engine_glows(),
            dialog.running_lights(),
            dialog.weapon_effects(),
            dialog.smoke_trails(),
            dialog.parallax_starfield(),
        ]
        .map(Toggle::rect)
        .into_iter()
        .chain([
            dialog.display().rects().bounds(),
            dialog.scaling().rects().bounds(),
            effects.label,
            effects.bounds(),
            dialog.music_volume().rects().bounds(),
            placed(left, top, right, bottom),
            ok,
            keys,
        ]);
        for rect in parts {
            let apart = rect.max.x <= list.min.x
                || rect.min.x >= list.max.x
                || rect.max.y <= list.min.y
                || rect.min.y >= list.max.y;
            assert!(apart, "{rect:?} under the list {list:?}");
        }
        let bounds = dialog.dialog().bounds();
        assert!(bounds.contains(list.min) && bounds.contains(list.max));
    }

    #[test]
    fn it_opens_on_the_scale_the_prefs_hold_listing_its_list() {
        let dialog = prefs();
        assert_eq!(dialog.render_scale().entries(), RenderScale::list(AUTO));
        assert_eq!(dialog.render_scale().selected(), AUTO);
        assert_eq!(dialog.prefs().render_scale, AUTO);
        let half = opened_on(P(50));
        assert_eq!(half.render_scale().entries(), RenderScale::list(P(50)));
        assert_eq!(half.render_scale().selected(), P(50));
        assert_eq!(half.prefs().render_scale, P(50));
        let odd = opened_on(P(60));
        assert_eq!(odd.render_scale().entries(), RenderScale::list(P(60)));
        assert_eq!(odd.render_scale().selected(), P(60), "shown, off the list");
        assert_eq!(odd.prefs().render_scale, P(60));
    }

    #[test]
    fn a_click_on_an_arrow_steps_it_and_reports_the_change_at_once() {
        let mut dialog = prefs();
        let rects = dialog.render_scale().rects();
        click(&mut dialog, rects.up.center());
        assert_eq!(dialog.take_change(), None, "Auto is the top");
        click(&mut dialog, rects.down.center());
        assert_eq!(dialog.prefs().render_scale, P(100));
        assert_eq!(
            dialog.take_change(),
            Some(Prefs {
                render_scale: P(100),
                ..start()
            })
        );
        assert_eq!(dialog.take_change(), None, "once");
        assert!(!dialog.closed(), "with the dialog still open");
        click(&mut dialog, rects.down.center());
        assert_eq!(
            dialog.take_change().map(|prefs| prefs.render_scale),
            Some(P(75))
        );
        click(&mut dialog, rects.up.center());
        click(&mut dialog, rects.up.center());
        assert_eq!(
            dialog.take_change().map(|prefs| prefs.render_scale),
            Some(AUTO)
        );
        click(&mut dialog, rects.value.center());
        assert_eq!(dialog.take_change(), None, "only the arrows take clicks");
        let mut smallest = opened_on(P(25));
        click(&mut smallest, rects.down.center());
        assert_eq!(smallest.take_change(), None, "25% is the bottom");
        assert_eq!(smallest.take_sounds(), [], "silent, as the volumes are");
    }

    #[test]
    fn up_and_down_step_it_while_it_has_the_focus() {
        let mut dialog = prefs();
        dialog.input(&key(Key::ArrowDown));
        assert_eq!(dialog.take_change(), None, "nothing focused");
        focus_it(&mut dialog);
        dialog.input(&key(Key::Space));
        dialog.input(&key(Key::ArrowUp));
        assert_eq!(dialog.take_change(), None, "Space and the top do nothing");
        dialog.input(&key(Key::ArrowDown));
        assert_eq!(
            dialog.take_change(),
            Some(Prefs {
                render_scale: P(100),
                ..start()
            })
        );
        dialog.input(&Input::Key {
            key: Key::ArrowDown,
            pressed: true,
            repeat: true,
        });
        assert_eq!(dialog.prefs().render_scale, P(75), "a repeat steps on");
        dialog.input(&key(Key::ArrowUp));
        dialog.input(&key(Key::ArrowUp));
        assert_eq!(
            dialog.take_change().map(|prefs| prefs.render_scale),
            Some(AUTO)
        );
        assert!(!dialog.closed());
        dialog.input(&key(Key::Tab));
        assert_eq!(dialog.focus(), Some(Focus::Display), "the next row down");
        dialog.input(&key(Key::ArrowDown));
        assert_eq!(dialog.prefs().render_scale, AUTO, "no longer focused");
    }

    #[test]
    fn another_controls_change_reports_the_scale_showing() {
        let mut dialog = opened_on(P(60));
        let point = dialog.music().rect().center();
        click(&mut dialog, point);
        let reported = dialog.take_change().expect("the music toggle");
        assert!(reported.sound.music);
        assert_eq!(reported.render_scale, P(60), "kept, though off the list");
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_change(), None, "closing reports no step");
    }

    #[test]
    fn its_step_and_the_scaling_lists_are_held_apart() {
        const SHARP: Scaling = Scaling::SharpBilinear;
        let mut dialog = prefs().with_scaling(&[Scaling::Native, SHARP]);
        let down = dialog.scaling().rects().down.center();
        click(&mut dialog, down);
        assert_eq!(
            dialog.take_change(),
            None,
            "the Scaling waits for the close"
        );
        let down = dialog.render_scale().rects().down.center();
        click(&mut dialog, down);
        assert_eq!(
            dialog.take_change(),
            Some(Prefs {
                render_scale: P(100),
                ..start()
            }),
            "the render scale at once, the Scaling still the one running"
        );
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_change(),
            Some(Prefs {
                render_scale: P(100),
                scaling: SHARP,
                ..start()
            }),
            "the Scaling at the close"
        );
        let mut alone = prefs().with_scaling(&[Scaling::Native, SHARP]);
        let down = alone.render_scale().rects().down.center();
        click(&mut alone, down);
        alone.take_change();
        alone.input(&key(Key::Enter));
        assert_eq!(alone.take_change(), None, "nothing waits for the close");
        assert_eq!(alone.take_display_choice(), None);
    }

    #[test]
    fn a_cancelled_click_on_it_steps_nothing() {
        let mut dialog = prefs();
        let down = dialog.render_scale().rects().down.center();
        dialog.input(&button(true, down));
        assert!(dialog.render_scale().pressed(Arrow::Down), "held");
        dialog.cancel_pointer();
        assert!(
            !dialog.render_scale().pressed(Arrow::Down),
            "the cancel lets the arrow go"
        );
        dialog.input(&button(false, down));
        assert_eq!(dialog.render_scale().selected(), AUTO);
        assert_eq!(dialog.take_change(), None);
    }

    #[test]
    fn it_is_drawn_after_the_scaling_list_and_outlined_when_focused() {
        let mut dialog = opened_on(P(67));
        let mut expected = DrawList::new();
        dialog.scaling().draw(false, &mut expected);
        dialog.render_scale().draw(false, &mut expected);
        dialog.effects_volume().draw(false, &mut expected);
        let expected: Vec<DrawCommand> = expected.iter().cloned().collect();
        let shown = drawn(&dialog);
        assert!(shown.windows(expected.len()).any(|w| w == expected));
        let label = DrawCommand::Text {
            text: "Render Scale: 67%".to_owned(),
            font: crate::font::Font::Geneva,
            origin: dialog.render_scale().rects().value.min,
            size: crate::ui::volume::TEXT_SIZE,
            wrap_width: None,
            color: crate::ui::volume::COLOR,
        };
        assert!(shown.contains(&label));
        assert!(
            !shown.iter().any(|command| matches!(
                command,
                DrawCommand::Text { text, .. } if text == "Check For Updates"
            )),
            "the stock box is not drawn"
        );
        focus_it(&mut dialog);
        let mut ring = DrawList::new();
        dialog.render_scale().draw(true, &mut ring);
        let ring: Vec<DrawCommand> = ring.iter().cloned().collect();
        let focused = drawn(&dialog);
        assert_eq!(focused.len(), shown.len() + 4, "one outline");
        assert!(focused.windows(ring.len()).any(|w| w == ring));
    }

    #[test]
    fn a_template_without_item_22_is_an_error() {
        let mut template = template();
        template.items.truncate(RENDER_SCALE_ITEM - 1);
        let error = PrefsDialog::new(&template, start(), ButtonStyle::STOCK, Rc::new(MonoMetrics))
            .expect_err("item 22 is missing");
        assert_eq!(error, "DITL 4003 has no item 22");
    }
}
