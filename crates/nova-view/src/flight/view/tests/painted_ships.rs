//! Painted ships drawn as `_HandleShipDisplay` draws them (rdm
//! play-feel-4 phase 5): the hull and alt layer in the paint the frame's
//! record carries, at the hull's translucency, a gate's included, and a
//! cloak's channel factors taken from the paint (see the `paint` and
//! `cloak_fade` modules). On [`FakeFlight`]: which paint a ship has is
//! the session's (nova-sim's `session::ship_paint` tests); the view draws
//! what the record says.

use super::ship_drawing::{
    MURK_COLOUR, NPC_0, display, flying_with, npc_sheet, redrawn, shown, shown_by,
};
use super::*;
use crate::draw::{Blend, lights_tint_by_channel, translucent_tint_by_channel};
use crate::flight::cloak_fade::{full_fade_factor, hull_factor};
use crate::flight::murk::sprite_murk;
use crate::flight::paint::{painted_hull, translucency};
use crate::flight::ship_layers::base_alpha;
use nova_sim::Rgb15;
use nova_sim::catalog::ShipDisplay;
use nova_sim::cloak::CloakShown;
use nova_sim::combat::ship_frames::Alt;
use nova_sim::flight_frame::FrameShip;

/// A paint of red 31, green 16 and blue 4.
const PAINT: Rgb15 = Rgb15 { r: 31, g: 16, b: 4 };

/// Each sprite of `image` in `list`, plain or colorized: its tint and
/// blend.
fn hulls_of(list: &DrawList, image: i16) -> Vec<(Color, Blend)> {
    list.iter()
        .filter_map(|command| match *command {
            DrawCommand::Sprite {
                image: key,
                tint,
                blend,
                ..
            }
            | DrawCommand::Colorized {
                image: key,
                tint,
                blend,
                ..
            } if key.id == image => Some((tint, blend)),
            _ => None,
        })
        .collect()
}

/// A flight whose player flies a sheet of `BaseTransp` 10 with an alt
/// layer 2400 it draws on set 0.
fn alt_flight() -> Faked {
    let mut catalog = shown(
        ShipDisplay {
            alt_sets: 6,
            alt_image: true,
            ..display(0x0058, 1, 5)
        },
        display(0, 1, 0),
        None,
    );
    catalog.sheet = Ok(ShipSheet {
        alt: Some(layer(2400, 216)),
        base_transp: 10,
        ..sheet()
    });
    let mut view = flying_with(catalog, 0.0);
    redrawn(&mut view, 0, |player| player.drawn.alt = Alt::Drawn(0));
    view
}

#[test]
fn a_painted_players_hull_and_alt_layer_are_drawn_in_its_paint() {
    let mut view = alt_flight();
    let plain = Color {
        a: base_alpha(10),
        ..Color::WHITE
    };
    let world = |view: &Faked| super::faked_world::world_list(view);
    assert_eq!(hulls_of(&world(&view), 2000), [(plain, Blend::Normal)]);
    assert_eq!(
        hulls_of(&world(&view), 2400),
        [(plain, Blend::Normal)],
        "unpainted: as before"
    );
    redrawn(&mut view, 0, |player| player.paint = Some(PAINT));
    let painted = painted_hull(Some(PAINT), 10).expect("painted");
    assert_eq!(painted, translucent_tint_by_channel([21, 6, 0], 10));
    assert_eq!(
        hulls_of(&world(&view), 2000),
        [(painted, Blend::OrTranslucent)]
    );
    assert_eq!(
        hulls_of(&world(&view), 2400),
        [(painted, Blend::OrTranslucent)],
        "the alt layer copies the hull's factors (@0x2caa1-0x2cb0a)"
    );
}

/// NPC 0's record in a flight of [`npc_sheet`], painted `paint`.
fn npc_painted(paint: Option<Rgb15>) -> (Faked, FrameShip) {
    let view = flying_with(
        shown(display(0, 1, 0), display(0, 1, 0), Some(npc_sheet())),
        0.0,
    );
    let npc = FrameShip {
        paint,
        ..npc_record(&view, NPC_0)
    };
    (view, npc)
}

/// `npc`'s hull as `view` draws it: its tint and blend.
fn npc_hull(view: &Faked, npc: &FrameShip) -> Option<(Color, Blend)> {
    let mut list = DrawList::new();
    view.draw_npc(&mut list, npc, &|_| VIEW_CENTER, npc.heading);
    hulls_of(&list, 2001).first().copied()
}

#[test]
fn a_painted_npc_is_drawn_in_its_paint_even_at_its_gate() {
    // A person NPC's paint is its `përs` `Color`: the session's
    // (nova-sim's `a_person_is_painted_in_its_colour`).
    let (view, mut npc) = npc_painted(None);
    assert_eq!(
        npc_hull(&view, &npc),
        Some((Color::WHITE, Blend::Normal)),
        "unpainted: as before"
    );
    npc.paint = Some(PAINT);
    assert_eq!(
        npc_hull(&view, &npc),
        Some((
            translucent_tint_by_channel([31, 16, 4], 0),
            Blend::OrTranslucent
        )),
        "opaque: the multiply"
    );
    // A gate's fade writes the translucency and leaves the paint
    // (@0x2c090-0x2c18e).
    npc.gate = GateFade::Emerging { ticks_left: 3 };
    let faded = npc_tint(npc.gate).expect("shown").a;
    let t = translucency(faded);
    assert!(t > 4, "faded past the blue: {t}");
    assert_eq!(
        npc_hull(&view, &npc),
        Some((
            painted_hull(Some(PAINT), t).expect("painted"),
            Blend::OrTranslucent
        ))
    );
}

#[test]
fn a_murky_painted_npc_is_mixed_in_its_unmurked_paint_and_translucency() {
    // Effective murk 67 at (100, -100): 20,000 × 67 × 1.2e-5 = 16.08.
    // A `BaseTransp` of 10, so its translucency writes a destination
    // factor. (Not a gate's: a gate's colorize stands in for the murk.)
    let sheet = ShipSheet {
        base_transp: 10,
        ..npc_sheet()
    };
    let view = shown_by(
        shown(display(0, 1, 0), display(0, 1, 0), Some(sheet)),
        0.0,
        67,
        &Rulebook::default(),
    );
    let npc = FrameShip {
        paint: Some(PAINT),
        ..npc_record(&view, NPC_0)
    };
    assert_eq!(sprite_murk(npc.position, Vec2::ZERO, 67), 16, "half murked");
    let t = translucency(base_alpha(10));
    assert!(t > 4, "faded past the blue: {t}");
    // The paint's source factors and the destination factor t untouched:
    // the mix is of the texel, before them.
    let source = translucent_tint_by_channel([31, 16, 4].map(|c: u8| c.saturating_sub(t)), t);
    assert_eq!(source, painted_hull(Some(PAINT), t).expect("painted"));
    let mut list = DrawList::new();
    view.draw_npc(&mut list, &npc, &|_| VIEW_CENTER, npc.heading);
    let hull = list.iter().find_map(|command| match *command {
        DrawCommand::Colorized {
            image,
            tint,
            blend,
            colorize,
            ..
        } if image.id == 2001 => Some((tint, blend, colorize)),
        _ => None,
    });
    assert_eq!(
        hull,
        Some((
            source,
            Blend::OrTranslucent,
            crate::Colorize::toward_pixel(16, MURK_COLOUR)
        ))
    );
}

/// The hull tint a cloak at `shown` draws a ship painted [`PAINT`] at.
fn cloaked_paint(shown: CloakShown) -> Color {
    match shown {
        CloakShown::Fading(fade) => {
            lights_tint_by_channel([PAINT.r, PAINT.g, PAINT.b].map(|c| hull_factor(c, fade)))
        }
        CloakShown::Translucent(level) => translucent_tint_by_channel(
            [PAINT.r, PAINT.g, PAINT.b].map(|c| full_fade_factor(c) + 32 - level),
            level,
        ),
        _ => unreachable!("drawn"),
    }
}

#[test]
fn a_cloaking_painted_npcs_hull_follows_its_paint() {
    let (view, mut npc) = npc_painted(Some(PAINT));
    npc.cloak = CloakShown::Fading(25.0);
    let fading = cloaked_paint(npc.cloak);
    assert_eq!(fading, lights_tint_by_channel([6, 2, 0]));
    assert_eq!(npc_hull(&view, &npc), Some((fading, Blend::Or)));
    npc.cloak = CloakShown::Translucent(0x1e);
    let faint = cloaked_paint(npc.cloak);
    assert_eq!(faint, translucent_tint_by_channel([4, 4, 2], 30));
    assert_eq!(npc_hull(&view, &npc), Some((faint, Blend::OrTranslucent)));
}

#[test]
fn a_cloaking_painted_players_hull_follows_its_paint() {
    let mut view = alt_flight();
    let hull = |view: &Faked| hulls_of(&super::faked_world::world_list(view), 2000);
    redrawn(&mut view, 0, |player| {
        player.paint = Some(PAINT);
        player.cloak = CloakShown::Fading(25.0);
    });
    assert_eq!(
        hull(&view),
        [(cloaked_paint(CloakShown::Fading(25.0)), Blend::Or)]
    );
    redrawn(&mut view, 0, |player| {
        player.cloak = CloakShown::Translucent(0x1e);
    });
    assert_eq!(
        hull(&view),
        [(
            cloaked_paint(CloakShown::Translucent(0x1e)),
            Blend::OrTranslucent
        )]
    );
}
