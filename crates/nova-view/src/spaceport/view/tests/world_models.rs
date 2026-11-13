//! The spaceport given the world models
//! ([`SpaceportView::with_world_models`]): the shipyard and the hire
//! screen it opens draw a class with no `PICT` as its model (the
//! shipyard's `picture` module), and its landscape draws its stellar's
//! model in its stand-in's place (the `landscape` module); with an empty
//! table each draws as before.

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

const HUB: ModelKey = ModelKey(32);

/// Earth (140) with a disc stand-in and no landscape `PICT`, its model
/// the hub, given `models` or not.
fn modelled_earth(models: Option<Rc<WorldModels>>) -> SpaceportView {
    let catalog = FakePort {
        port: Ok(PortRecord {
            stand_in: Some(crate::outline::StellarStandIn::Disc(Color::WHITE)),
            ..catalog().port.expect("a record")
        }),
        pictures: Vec::new(),
        ..catalog()
    };
    let view = view_of(&catalog);
    match models {
        Some(models) => view.with_world_models(models),
        None => view,
    }
}

fn hub() -> Rc<WorldModels> {
    Rc::new(WorldModels {
        stellars: [(
            StellarId(140),
            crate::flight::world_models::StellarModel::Model {
                key: HUB,
                reach: 4.0,
            },
        )]
        .into(),
        ..WorldModels::default()
    })
}

#[test]
fn a_modelled_stellar_s_landscape_draws_its_model_in_the_stand_in_s_place() {
    let view = modelled_earth(Some(hub()));
    let commands = drawn(&view);
    let area = item(&view, LANDSCAPE_ITEM);
    let models: Vec<(ModelKey, Point)> = commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Model { model, pose, .. } => Some((*model, pose.center)),
            _ => None,
        })
        .collect();
    assert_eq!(models, [(HUB, area.center())]);
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Triangle { .. })),
        "no stand-in"
    );
    // Between the background and the name, where the stand-in was.
    assert!(matches!(commands[1], DrawCommand::Model { .. }));
    assert!(matches!(&commands[2], DrawCommand::Text { text, .. } if text == "Earth"));
    assert_eq!(view.missing_landscape(), Some(10_003));
}

#[test]
fn with_an_empty_table_the_landscape_draws_as_before() {
    let before = modelled_earth(None);
    let empty = modelled_earth(Some(Rc::default()));
    assert_eq!(drawn(&empty), drawn(&before));
    let other = modelled_earth(Some(Rc::new(WorldModels {
        stellars: [(
            StellarId(141),
            crate::flight::world_models::StellarModel::Model {
                key: HUB,
                reach: 4.0,
            },
        )]
        .into(),
        ..WorldModels::default()
    })));
    assert_eq!(drawn(&other), drawn(&before), "another stellar's model");
}

#[test]
fn the_landscape_draws_its_own_stellar_s_key_when_another_holds_another() {
    // Earth and another stellar on one model in two liveries: two keys.
    const PAINTED_HUB: ModelKey = ModelKey(33);
    let models = |earth: ModelKey, other: ModelKey| {
        let model = |key| crate::flight::world_models::StellarModel::Model { key, reach: 4.0 };
        Rc::new(WorldModels {
            stellars: [
                (StellarId(140), model(earth)),
                (StellarId(141), model(other)),
            ]
            .into(),
            ..WorldModels::default()
        })
    };
    for (earth, other) in [(HUB, PAINTED_HUB), (PAINTED_HUB, HUB)] {
        let view = modelled_earth(Some(models(earth, other)));
        assert_eq!(model_keys(&drawn(&view)), [earth], "{earth:?}");
    }
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
