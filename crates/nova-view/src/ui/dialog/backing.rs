//! The template's backing: drawn over the placed window before the frame
//! and the items, and nothing for an unbacked template.

use std::rc::Rc;

use super::*;
use crate::text::fixture::MonoMetrics;

fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
    Bounds::at(Point::new(x, y), w, h)
}

/// A 300 x 120 template at (10, 10), centred so its placement moves it,
/// with a button and a line of static text.
fn template(backing: Option<PanelBacking>) -> DialogTemplate {
    let item = |bounds, kind| ItemTemplate {
        bounds,
        enabled: true,
        kind,
    };
    DialogTemplate {
        backing,
        ..DialogTemplate::new(
            rect(10.0, 10.0, 300.0, 120.0),
            Placement::Center,
            vec![
                item(rect(220.0, 90.0, 70.0, 20.0), ItemSpec::Button("OK".into())),
                item(
                    rect(10.0, 10.0, 280.0, 40.0),
                    ItemSpec::StaticText("Hello there.".into()),
                ),
            ],
        )
    }
}

fn dialog(backing: Option<PanelBacking>) -> Dialog {
    Dialog::new(&template(backing), &[], Rc::new(MonoMetrics))
}

fn commands(draw: impl FnOnce(&mut DrawList)) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    draw(&mut list);
    list.iter().cloned().collect()
}

#[test]
fn new_is_the_literal_with_no_backing() {
    let bounds = rect(1.0, 2.0, 3.0, 4.0);
    assert_eq!(
        DialogTemplate::new(bounds, Placement::Fixed, Vec::new()),
        DialogTemplate {
            bounds,
            placement: Placement::Fixed,
            items: Vec::new(),
            backing: None,
        }
    );
}

#[test]
fn a_backed_dialog_reports_its_templates_backing() {
    assert_eq!(
        dialog(Some(PanelBacking::NEW_GAME)).backing(),
        Some(PanelBacking::NEW_GAME)
    );
    assert_eq!(dialog(None).backing(), None);
}

#[test]
fn a_backed_dialog_draws_its_backing_over_the_placed_window_then_its_items() {
    let dialog = dialog(Some(PanelBacking::NEW_GAME));
    assert_ne!(
        dialog.bounds().min,
        Point::new(10.0, 10.0),
        "placement moves it"
    );
    let backing = commands(|list| PanelBacking::NEW_GAME.draw(list, dialog.bounds()));
    let items = commands(|list| dialog.draw_items(list));
    assert!(!items.is_empty());
    assert_eq!(commands(|list| dialog.draw_backing(list)), backing);
    assert_eq!(
        commands(|list| dialog.draw(list)),
        [backing, items].concat()
    );
}

#[test]
fn an_unbacked_dialog_draws_only_its_items() {
    let dialog = dialog(None);
    assert!(commands(|list| dialog.draw_backing(list)).is_empty());
    assert_eq!(
        commands(|list| dialog.draw(list)),
        commands(|list| dialog.draw_items(list))
    );
}

#[test]
fn a_backed_dialog_draws_its_backing_before_its_frames_pictures() {
    let frame = DialogFrame {
        top: ImageKey::interface_picture(1),
        middle: ImageKey::interface_picture(2),
        bottom: ImageKey::interface_picture(3),
        top_height: 10.0,
        bottom_height: 10.0,
    };
    let dialog = dialog(Some(PanelBacking::NEW_GAME)).with_frame(frame);
    let drawn = commands(|list| dialog.draw(list));
    let backing = commands(|list| PanelBacking::NEW_GAME.draw(list, dialog.bounds()));
    assert_eq!(drawn[..backing.len()], backing[..]);
    assert!(matches!(
        drawn[backing.len()],
        DrawCommand::Picture { .. } | DrawCommand::StretchedPicture { .. }
    ));
}

#[test]
fn a_rebuilt_dialog_keeps_its_backing() {
    let template = template(Some(PanelBacking::NEW_GAME));
    let rebuilt = Dialog::new(&template, &[(1, Role::Hidden)], Rc::new(MonoMetrics));
    assert_eq!(rebuilt.backing(), Some(PanelBacking::NEW_GAME));
}
