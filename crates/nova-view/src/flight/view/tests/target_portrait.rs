//! The target panel's portrait (rdm:phase/rendering-3d/phase-8-models-in-other-screens):
//! a target whose ship type has a model in the flight's table
//! ([`FlightView::with_world_models`]) and no picture on its card draws
//! its [`ShipPortrait`] in the picture's 128 x 64 rectangle, in the
//! overlay, under the HUD's cadence snapshot too; a card's picture wins;
//! and an empty table draws as before. On [`FakeFlight`], with a
//! hand-built table.

use std::collections::BTreeMap;
use std::rc::Rc;

use nova_sim_frame::flight_frame::FrameTarget;
use nova_sim_frame::{TargetStatus, TargetTag};

use super::faked_hud::{barred, flown, npc_of_140};
use super::*;
use crate::DrawLayer;
use crate::flight::target::{PICTURE_HEIGHT, PICTURE_WIDTH};
use crate::flight::world_models::{ShipModel, WorldModels};
use crate::model::ModelKey;
use crate::model_picture::ShipPortrait;

const HULL: ModelKey = ModelKey(9);

/// Ship 129, the NPC's type, with a hull model.
fn table() -> WorldModels {
    WorldModels {
        ships: BTreeMap::from([(
            ShipId(129),
            ShipModel {
                key: HULL,
                scale: 1.5,
                base_transp: 0,
                radius: 3.0,
            },
        )]),
        ..WorldModels::default()
    }
}

/// [`armed`], ship 129's card with no picture unless `picture`.
fn catalog(picture: bool) -> FakeCatalog {
    let mut catalog = armed(1, &[]);
    if !picture {
        catalog.cards[0].1.picture = None;
    }
    catalog
}

/// A view targeting NPC 0, of ship 129, with its bar, by `models`.
fn targeting(picture: bool, models: WorldModels) -> Faked {
    let target = FrameTarget {
        npc: NpcId(0),
        name: "Ace".to_owned(),
        subtitle: None,
        status: TargetStatus::Disabled,
        tag: Some(TargetTag::Code(GovtId(140))),
    };
    barred(catalog(picture), targeted(vec![npc_of_140(0)], target))
        .with_world_models(Rc::new(models))
}

/// The picture's rectangle on the bar, as the target panel places it.
fn picture_area(view: &Faked) -> Bounds {
    let bar = view.status_bar.as_ref().expect("a bar");
    let area = bar.layout.targ.offset(hud::bar_origin(bar, VIEW_SIZE.0));
    let (x, y) = (
        f32::midpoint(area.min.x, area.max.x).trunc(),
        f32::midpoint(area.min.y, area.max.y).trunc(),
    );
    Bounds::at(
        Point::new(x - PICTURE_WIDTH / 2.0, y - PICTURE_HEIGHT / 2.0),
        PICTURE_WIDTH,
        PICTURE_HEIGHT,
    )
}

/// Each model `list` draws, with its layer and centre.
fn models(list: &DrawList) -> Vec<(DrawLayer, ModelKey, Point)> {
    list.layered()
        .filter_map(|(layer, command)| match command {
            DrawCommand::Model { model, pose, .. } => Some((layer, *model, pose.center)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_modelled_target_with_no_picture_draws_its_portrait_in_the_overlay() {
    let mut view = targeting(false, table());
    let portrait = ShipPortrait::of(&table(), ShipId(129));
    assert!(portrait.is_some());
    assert_eq!(
        view.target_now(&view.frame).and_then(|panel| panel.model),
        portrait
    );
    let centre = picture_area(&view).center();
    let want = [(DrawLayer::Overlay, HULL, centre)];
    assert_eq!(models(&flown(&view)), want);
    // The HUD's cadence shows its snapshot, which keeps the portrait.
    assert!(view.hud_waits());
    view.retake_target();
    assert_eq!(models(&flown(&view)), want, "the snapshot's");
}

#[test]
fn the_portrait_is_the_one_its_rectangle_fits() {
    let view = targeting(false, table());
    let area = picture_area(&view);
    let mut want = DrawList::new();
    assert!(
        ShipPortrait::of(&table(), ShipId(129))
            .expect("a portrait")
            .draw_in(&mut want, area)
    );
    let want: Vec<DrawCommand> = want.iter().cloned().collect();
    let drawn: Vec<DrawCommand> = flown(&view).iter().cloned().collect();
    assert!(
        drawn.windows(want.len()).any(|run| run == want.as_slice()),
        "{want:?}"
    );
}

#[test]
fn a_card_s_picture_wins_over_a_model() {
    let view = targeting(true, table());
    let panel = view.target_now(&view.frame).expect("a target");
    assert_eq!(panel.picture, Some(3001));
    assert_eq!(panel.model, None);
    assert_eq!(models(&flown(&view)), []);
}

#[test]
fn an_empty_table_draws_the_hud_as_with_no_model() {
    let view = targeting(false, WorldModels::default());
    let panel = view.target_now(&view.frame).expect("a target");
    assert_eq!(panel.model, None);
    assert_eq!(models(&flown(&view)), []);
    let unmodelled = barred(
        catalog(false),
        targeted(
            vec![npc_of_140(0)],
            FrameTarget {
                npc: NpcId(0),
                name: "Ace".to_owned(),
                subtitle: None,
                status: TargetStatus::Disabled,
                tag: Some(TargetTag::Code(GovtId(140))),
            },
        ),
    );
    let commands = |view: &Faked| flown(view).iter().cloned().collect::<Vec<_>>();
    assert_eq!(commands(&view), commands(&unmodelled));
}
