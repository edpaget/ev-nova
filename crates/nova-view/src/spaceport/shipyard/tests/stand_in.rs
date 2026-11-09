//! A class with no `PICT` drawn by its stand-in, in the picture item and
//! its grid cell (the shipyard's `picture` module), on the shipyard.

use super::*;
use crate::outline::OutlineTriangle;
use crate::spaceport::grid::{NO_PICTURE, PICTURE_AVAILABLE};
use crate::spaceport::shipyard::picture::ShipPicture;

/// A hull 3 wide and 8 long.
fn hull() -> Outline {
    let at = Point::new;
    Outline {
        triangles: vec![OutlineTriangle {
            corners: [at(0.0, 4.0), at(1.5, -4.0), at(-1.5, -4.0)],
            color: Color::rgba(255, 0, 0, 255),
        }],
    }
}

/// [`art`], where the Heavy Shuttle (129, no picture) and the Shuttle
/// (128, its picture) have stand-ins.
fn stood_in() -> Rc<FakeArt> {
    let art = Rc::try_unwrap(art()).ok().expect("unshared");
    Rc::new(FakeArt {
        stand_ins: vec![(ShipId(128), hull()), (ShipId(129), hull())],
        ..art
    })
}

/// The triangles among `commands`.
fn triangles(commands: &[DrawCommand]) -> Vec<DrawCommand> {
    commands
        .iter()
        .filter(|command| matches!(command, DrawCommand::Triangle { .. }))
        .cloned()
        .collect()
}

/// What `picture` draws in `area`.
fn drawn_in(picture: &ShipPicture, area: Bounds) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    picture.draw(&mut list, area, &MonoMetrics);
    list.iter().cloned().collect()
}

/// What `picture` draws in `cell`.
fn drawn_in_cell(picture: &ShipPicture, cell: Bounds) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    picture.draw_cell(&mut list, cell);
    list.iter().cloned().collect()
}

#[test]
fn a_class_with_no_picture_draws_its_stand_in_in_the_picture_item_and_not_no_picture() {
    let mut screen = screen_with(shipyard(), &stood_in());
    press(&mut screen, Key::ArrowRight);
    assert_eq!(screen.selected(), Some(1), "the Heavy Shuttle");
    let picture = item(&screen, PICTURE_ITEM);
    let commands = drawn(&screen);
    let words: Vec<String> = texts(&commands)
        .into_iter()
        .filter(|(_, at, _)| picture.contains(*at))
        .map(|(words, _, _)| words)
        .collect();
    assert!(!words.contains(&NO_PICTURE.to_owned()), "{words:?}");
    assert!(!words.contains(&PICTURE_AVAILABLE.to_owned()), "{words:?}");
    let expected = drawn_in(&ShipPicture::StandIn(hull()), picture);
    assert!(!expected.is_empty());
    assert!(
        expected.iter().all(|command| commands.contains(command)),
        "{commands:?}"
    );
}

#[test]
fn a_class_with_no_picture_draws_its_stand_in_in_its_grid_cell() {
    let screen = screen_with(shipyard(), &stood_in());
    let commands = drawn(&screen);
    let cell = screen.cell_bounds(1).expect("the Heavy Shuttle's cell");
    let expected = drawn_in_cell(&ShipPicture::StandIn(hull()), cell);
    assert_eq!(triangles(&expected), expected, "only the outline");
    assert!(
        expected.iter().all(|command| commands.contains(command)),
        "{commands:?}"
    );
    let black = drawn_in_cell(&ShipPicture::Missing, cell);
    assert!(
        !black.iter().any(|command| commands.contains(command)),
        "no black icon"
    );
}

#[test]
fn a_class_with_a_picture_keeps_it_over_its_stand_in() {
    let screen = screen_with(shipyard(), &stood_in());
    assert_eq!(screen.selected(), Some(0), "the Shuttle");
    let picture = item(&screen, PICTURE_ITEM);
    let commands = drawn(&screen);
    assert!(commands.contains(&DrawCommand::StretchedPicture {
        image: ImageKey::picture(5000),
        top_left: picture.min,
        width: 200.0,
        height: 200.0,
    }));
    let unpictured = drawn_in(&ShipPicture::StandIn(hull()), picture);
    assert!(
        !unpictured.iter().any(|command| commands.contains(command)),
        "no stand-in"
    );
}
