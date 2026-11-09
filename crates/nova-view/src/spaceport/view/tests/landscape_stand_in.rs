//! A stellar's stand-in in the spaceport's landscape (see
//! [`landscape`](crate::spaceport::landscape)): drawn in the landscape
//! item when the data has no landscape `PICT`, in the picture's place in
//! the draw order, and never over a picture that is there.

use super::*;
use crate::outline::{Outline, OutlineTriangle, StellarStandIn};

const GOLD: Color = Color::rgba(0xc0, 0x80, 0x40, 255);
const TEAL: Color = Color::rgba(0x10, 0x80, 0x80, 255);

/// Earth with `stand_in` and the landscape `PICT`s in `pictures`.
fn earth_with(stand_in: StellarStandIn, pictures: Vec<i16>) -> SpaceportView {
    let catalog = FakePort {
        port: Ok(PortRecord {
            stand_in: Some(stand_in),
            ..catalog().port.expect("a record")
        }),
        pictures,
        ..catalog()
    };
    view_of(&catalog)
}

/// A wide outline: a gold and a teal triangle.
fn hull() -> StellarStandIn {
    let at = Point::new;
    StellarStandIn::Outline(Outline {
        triangles: vec![
            OutlineTriangle {
                corners: [at(0.0, 2.0), at(6.0, -2.0), at(-6.0, -2.0)],
                color: GOLD,
            },
            OutlineTriangle {
                corners: [at(0.0, -2.0), at(3.0, -3.0), at(-3.0, -3.0)],
                color: TEAL,
            },
        ],
    })
}

/// Each triangle drawn: its corners and its colour.
fn triangles(commands: &[DrawCommand]) -> Vec<([Point; 3], Color)> {
    commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Triangle { corners, color } => Some((*corners, *color)),
            _ => None,
        })
        .collect()
}

fn no_landscape_picture(commands: &[DrawCommand]) -> bool {
    !commands.iter().any(|command| {
        matches!(command, DrawCommand::StretchedPicture { image, .. }
            if *image == ImageKey::picture(10_003))
    })
}

#[test]
fn with_no_landscape_a_disc_stand_in_is_drawn_round_the_item_s_centre() {
    let view = earth_with(StellarStandIn::Disc(GOLD), Vec::new());
    let area = item(&view, LANDSCAPE_ITEM);
    let commands = drawn(&view);
    let drawn = triangles(&commands);
    assert!(!drawn.is_empty());
    assert!(drawn.iter().all(|(corners, color)| *color == GOLD
        && corners[0] == area.center()
        && corners.iter().all(|&p| area.contains(p))));
    assert!(no_landscape_picture(&commands));
}

#[test]
fn with_no_landscape_an_outline_stand_in_is_drawn_in_the_item_in_its_colours() {
    let view = earth_with(hull(), Vec::new());
    let area = item(&view, LANDSCAPE_ITEM);
    let commands = drawn(&view);
    let drawn = triangles(&commands);
    assert_eq!(
        drawn.iter().map(|(_, color)| *color).collect::<Vec<_>>(),
        [GOLD, TEAL]
    );
    assert!(
        drawn
            .iter()
            .all(|(corners, _)| corners.iter().all(|&p| area.contains(p)))
    );
    assert!(no_landscape_picture(&commands));
}

#[test]
fn a_landscape_that_is_there_is_drawn_rather_than_the_stand_in() {
    let view = earth_with(StellarStandIn::Disc(GOLD), vec![10_003]);
    let commands = drawn(&view);
    assert!(triangles(&commands).is_empty());
    assert!(!no_landscape_picture(&commands));
    assert_eq!(view.missing_landscape(), None);
}

#[test]
fn the_stand_in_comes_between_the_background_and_the_name() {
    let view = earth_with(hull(), Vec::new());
    let commands = drawn(&view);
    assert!(matches!(
        commands[0],
        DrawCommand::Picture { image, .. } if image == ImageKey::picture(8500)
    ));
    assert!(matches!(commands[1], DrawCommand::Triangle { color, .. } if color == GOLD));
    assert!(matches!(commands[2], DrawCommand::Triangle { color, .. } if color == TEAL));
    assert!(matches!(&commands[3], DrawCommand::Text { text, .. } if text == "Earth"));
}

#[test]
fn the_landscape_is_still_missing_with_a_stand_in_drawn_for_it() {
    let view = earth_with(StellarStandIn::Disc(GOLD), Vec::new());
    assert_eq!(view.missing_landscape(), Some(10_003));
}
