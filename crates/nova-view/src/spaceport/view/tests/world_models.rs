//! The spaceport given the world models
//! ([`SpaceportView::with_world_models`]): the shipyard it opens draws a
//! class with no `PICT` as its model (the shipyard's `picture` module);
//! with an empty table it draws as before.

use super::*;
use crate::flight::world_models::{ShipModel, WorldModels};
use crate::model::ModelKey;

const HEAVY: ModelKey = ModelKey(31);

/// The Heavy Shuttle's (129) hull model.
fn models() -> Rc<WorldModels> {
    Rc::new(WorldModels {
        ships: [(
            ShipId(129),
            ShipModel {
                key: HEAVY,
                scale: 1.0,
                base_transp: 0,
                radius: 3.0,
            },
        )]
        .into(),
        ..WorldModels::default()
    })
}

/// The models `commands` draw.
fn model_keys(commands: &[DrawCommand]) -> Vec<ModelKey> {
    commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Model { model, .. } => Some(*model),
            _ => None,
        })
        .collect()
}

#[test]
fn the_shipyard_it_opens_draws_a_modelled_class_as_its_model() {
    let mut view = shipbuying(Ok(shipyard_template())).with_world_models(models());
    click_item(&mut view, 9);
    assert!(view.open_shipyard().is_some());
    assert_eq!(model_keys(&drawn(&view)), [HEAVY], "its grid icon");
}

#[test]
fn the_hire_screen_its_bar_opens_draws_a_modelled_class_as_its_model() {
    let mut view = drinking(true).with_world_models(models());
    click_item(&mut view, 11);
    view.input(&key(Key::KeyH));
    assert!(view.take_hire_request());
    view.open_hire(hirelings(2, true), None);
    assert!(view.open_bar().and_then(BarScreen::hire_screen).is_some());
    assert_eq!(model_keys(&drawn(&view)), [HEAVY], "ship 129's icon");
}

#[test]
fn with_an_empty_table_the_shipyard_draws_as_before() {
    let mut before = shipbuying(Ok(shipyard_template()));
    let mut empty = shipbuying(Ok(shipyard_template())).with_world_models(Rc::default());
    click_item(&mut before, 9);
    click_item(&mut empty, 9);
    assert_eq!(drawn(&empty), drawn(&before));
    assert_eq!(model_keys(&drawn(&before)), []);
}
