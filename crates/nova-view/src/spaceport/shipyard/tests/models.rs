//! A class with no `PICT` drawn as its model, in the picture item and its
//! grid cell (the shipyard's `picture` module), on the shipyard given the
//! world models ([`ShipyardScreen::with_world_models`]); and an empty
//! table drawing the shipyard as before.

use super::*;
use crate::draw::DrawLayer;
use crate::flight::world_models::{ShipModel, WorldModels};
use crate::model::ModelKey;
use crate::model_picture::ShipPortrait;
use crate::spaceport::grid::{NO_PICTURE, icon_bounds};

const HEAVY: ModelKey = ModelKey(21);
const COURIER: ModelKey = ModelKey(22);
const SHUTTLE: ModelKey = ModelKey(23);

fn model(key: ModelKey) -> ShipModel {
    ShipModel {
        key,
        scale: 1.0,
        base_transp: 0,
        radius: 4.0,
    }
}

/// Models for the Shuttle (128, its picture), the Heavy Shuttle (129) and
/// the Courier (130).
fn models() -> WorldModels {
    WorldModels {
        ships: [
            (ShipId(128), model(SHUTTLE)),
            (ShipId(129), model(HEAVY)),
            (ShipId(130), model(COURIER)),
        ]
        .into(),
        ..WorldModels::default()
    }
}

/// [`art`], where only the Shuttle's `PICT` 5000 (and the Viper's, which
/// shares its base image) exists: the Heavy Shuttle and the Courier have
/// none.
fn unpictured() -> Rc<FakeArt> {
    let art = Rc::try_unwrap(art()).ok().expect("unshared");
    Rc::new(FakeArt {
        pictures: vec![5000],
        bases: vec![
            (ShipId(128), 1000),
            (ShipId(129), 1001),
            (ShipId(130), 1002),
            (ShipId(131), 1000),
        ],
        ..art
    })
}

fn portrait(ship: i16) -> ShipPortrait {
    ShipPortrait::of(&models(), ShipId(ship)).expect("a portrait")
}

/// What `portrait` draws in `area`.
fn portrait_in(portrait: &ShipPortrait, area: Bounds) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    assert!(portrait.draw_in(&mut list, area));
    list.iter().cloned().collect()
}

/// The models `commands` draw, each with its index.
fn model_indices(commands: &[DrawCommand]) -> Vec<usize> {
    commands
        .iter()
        .enumerate()
        .filter(|(_, command)| matches!(command, DrawCommand::Model { .. }))
        .map(|(index, _)| index)
        .collect()
}

#[test]
fn a_modelled_class_with_no_picture_draws_its_model_in_the_picture_item() {
    let mut screen = screen_with(shipyard(), &unpictured()).with_world_models(Rc::new(models()));
    press(&mut screen, Key::ArrowRight);
    assert_eq!(screen.selected(), Some(1), "the Heavy Shuttle");
    let picture = item(&screen, PICTURE_ITEM);
    let commands = drawn(&screen);
    let want = portrait_in(&portrait(129), picture);
    assert!(
        commands
            .windows(want.len())
            .any(|run| run == want.as_slice()),
        "{commands:?}"
    );
    let words: Vec<String> = texts(&commands)
        .into_iter()
        .filter(|(_, at, _)| picture.contains(*at))
        .map(|(words, _, _)| words)
        .collect();
    assert!(!words.contains(&NO_PICTURE.to_owned()), "{words:?}");
}

#[test]
fn the_grid_s_model_icons_are_one_run_after_its_cells() {
    let screen = opened_with(shipyard(), &unpictured()).with_world_models(Rc::new(models()));
    assert_eq!(screen.selected(), None, "nothing in item 8");
    let mut list = DrawList::new();
    screen.draw(&mut list);
    let commands: Vec<DrawCommand> = list.iter().cloned().collect();
    let models = model_indices(&commands);
    assert_eq!(models.len(), 2, "the Heavy Shuttle's and the Courier's");
    assert_eq!(models[1], models[0] + 1, "consecutive");
    for (index, key) in [(1, HEAVY), (2, COURIER)] {
        let cell = screen.cell_bounds(index).expect("a cell");
        let want = portrait_in(
            &portrait(i16::try_from(128 + index).unwrap()),
            icon_bounds(cell),
        );
        assert!(commands.contains(&want[0]), "{key:?}");
    }
    assert!(
        list.layered().all(|(layer, _)| layer == DrawLayer::Overlay),
        "all in the overlay"
    );
    // The Shuttle keeps its picture over its model.
    let shuttle = screen.cell_bounds(0).expect("a cell");
    assert!(commands.contains(&DrawCommand::StretchedPicture {
        image: ImageKey::picture(5000),
        top_left: icon_bounds(shuttle).min,
        width: grid::ICON_SIZE,
        height: grid::ICON_SIZE,
    }));
}

#[test]
fn an_empty_table_draws_the_shipyard_as_before() {
    let before = screen_with(shipyard(), &unpictured());
    let empty = screen_with(shipyard(), &unpictured()).with_world_models(Rc::default());
    assert_eq!(drawn(&empty), drawn(&before));
    assert_eq!(model_indices(&drawn(&before)), [0usize; 0]);
}
