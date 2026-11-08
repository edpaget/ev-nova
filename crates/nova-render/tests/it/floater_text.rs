//! Flight's floaters' text under a frame upscaler, drawn through the wgpu
//! adapter: it draws over the over-ship beams and under the particles, as
//! `_PostDrawCallback` (0x38c80) draws `_FloaterDrawCallback` after every
//! sprite layer and the over-ship beams and then `_DrawParticlesOffscreen`.
//! The world's tail (`DrawList::start_world_tail`) starts at the floaters,
//! so the floaters, their text and the particles draw at the window's
//! resolution over the enlarged world, in the list's order. The scene is
//! built by hand from `DrawList` primitives in flight's order; that flight
//! draws them so is nova-view's, pinned there. Needs a GPU adapter; skips,
//! passing, without one.

use nova_render::Upscaler;
use nova_view::geometry::Bounds;
use nova_view::{Color, DrawList, Font, Point};

use super::common::fill;
use super::flight_frame::{LOGICAL, fitted};
use super::message_line::{
    BLACK, BOX, SHIP, STARS, WHITE, area, drawn, inside, message_box, pixel, pixels_in, rgba, ship,
    w555,
};

/// An over-ship beam's colour: green.
const BEAM: [u8; 3] = [0, 31, 0];
/// A particle's colour: yellow.
const PARTICLE: [u8; 3] = [31, 31, 0];

/// The over-ship beam, a band over the box right of the ship, in logical
/// units.
fn beam() -> Bounds {
    area(16.0, 14.0, 22.0, 22.0)
}

/// The particle's centre and side: a 4-unit square over the first glyph's
/// right edge, the gap after it and the beam's right end.
const PARTICLE_AT: Point = Point::new(22.0, 18.0);
const PARTICLE_SIDE: f32 = 4.0;

/// The particle's square, in logical units.
fn particle() -> Bounds {
    let half = PARTICLE_SIDE / 2.0;
    area(
        PARTICLE_AT.x - half,
        PARTICLE_AT.y - half,
        PARTICLE_AT.x + half,
        PARTICLE_AT.y + half,
    )
}

/// The floaters' text, in the block font, across the beam and under the
/// particle.
fn text(list: &mut DrawList) {
    list.text_in(
        Font::Charcoal,
        "MM",
        Point::new(15.0, 13.0),
        8.0,
        None,
        Color::WHITE,
    );
}

/// Flight's world in its order: the stars, the message box over them, a
/// ship, an over-ship beam; then the world's tail: the box's colour OR-ed
/// over the sprites (the floaters' blit), the text, and a particle.
fn floater_scene() -> DrawList {
    let mut list = DrawList::new();
    list.world_in(area(0.0, 0.0, 32.0, 24.0), |list| {
        fill(list, area(0.0, 0.0, 32.0, 24.0), rgba(w555(STARS)));
        fill(list, message_box(), rgba(w555(BOX)));
        fill(list, ship(), rgba(w555(SHIP)));
        fill(list, beam(), rgba(w555(BEAM)));
        list.start_world_tail();
        list.or_fill(message_box(), rgba(w555(BOX)));
        text(list);
        list.dot(PARTICLE_AT, PARTICLE_SIDE, rgba(w555(PARTICLE)));
    });
    list
}

/// The text alone, unmarked, over the black clear: where its glyphs cover.
fn text_alone() -> DrawList {
    let mut list = DrawList::new();
    text(&mut list);
    list
}

#[test]
fn under_a_frame_upscaler_the_floaters_text_is_under_the_particles_and_over_the_beams() {
    let viewport = fitted(LOGICAL, super::message_line::WINDOW);
    let Some(glyphs) = drawn(&text_alone(), &viewport, Upscaler::Native) else {
        return;
    };
    let ored_beam = w555([BEAM[0] | BOX[0], BEAM[1] | BOX[1], BEAM[2] | BOX[2]]);
    for upscaler in [Upscaler::SharpBilinear, Upscaler::Native] {
        let Some(frame) = drawn(&floater_scene(), &viewport, upscaler) else {
            return;
        };
        let (mut glyph_under_particle, mut bare_particle) = (0, 0);
        let (mut glyph_over_beam, mut bare_beam) = (0, 0);
        for at in pixels_in(message_box()) {
            let got = pixel(&frame, at);
            let glyph = pixel(&glyphs, at);
            if inside(particle(), at) {
                // The particle over everything, the text included.
                assert_eq!(got, w555(PARTICLE), "{upscaler:?}: the particle at {at:?}");
                match glyph {
                    WHITE => glyph_under_particle += 1,
                    BLACK => bare_particle += 1,
                    _ => {}
                }
            } else if inside(beam(), at) {
                match glyph {
                    // The text over the over-ship beam.
                    WHITE => {
                        assert_eq!(got, WHITE, "{upscaler:?}: a glyph over the beam at {at:?}");
                        glyph_over_beam += 1;
                    }
                    // The box's colour OR-ed over the beam.
                    BLACK => {
                        assert_eq!(got, ored_beam, "{upscaler:?}: the beam at {at:?}");
                        bare_beam += 1;
                    }
                    // A glyph's edge.
                    _ => {}
                }
            }
        }
        assert!(
            glyph_under_particle > 0,
            "{upscaler:?}: no glyph under the particle"
        );
        assert!(bare_particle > 0, "{upscaler:?}: no bare particle");
        assert!(glyph_over_beam > 0, "{upscaler:?}: no glyph over the beam");
        assert!(bare_beam > 0, "{upscaler:?}: no bare beam");
    }
}
