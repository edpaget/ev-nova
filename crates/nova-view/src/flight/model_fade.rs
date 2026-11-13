//! How faded a ship's 3D model draws ([`model_alpha`]): the new game's
//! reading of the 2D hull's fade, for a ship whose class draws a model
//! (the flight view's `ship_models` module).
//!
//! The 2D hull fades three ways, the first that applies winning, and the
//! model follows the same precedence:
//!
//! 1. **A cloak** ([`CloakShown`], drawn as [`CloakDraw`](super::cloak_fade::CloakDraw)
//!    draws it): fading, the hull keeps [`hull_factor`]`(paint_c, fade)`
//!    32nds of each channel, OR-ed over what is beneath; fully faded but
//!    seen, its [`translucent_factors`] 32nds; hidden, nothing.
//! 2. **A gate**: an NPC entering one or coming out of one draws at its
//!    gate tint's alpha (the flight view's `npc_tint`), and nothing while it
//!    waits hidden at the gate it comes out of.
//! 3. **Its `BaseTransp`**: [`base_alpha`], full at 0, nothing from 32.
//!
//! A model is lit and depth-tested, so the sprite's OR and translucent-copy
//! blends have no counterpart on it. It draws instead at a straight alpha:
//! a cloak's the mean, over the three channels, of the share of the hull
//! the 2D blend keeps; a gate's and a `BaseTransp`'s the hull's own alpha.
//! That is the nearest a model comes to the sprite. Only the new game's
//! classes have models, so it needs no rulebook key: stock ships draw
//! their sprites. An alpha that comes out 0 draws nothing, as `None` does.

use nova_sim_frame::Rgb15;
use nova_sim_frame::cloak::{CloakShown, FADE_FULL};
use nova_sim_frame::flight_frame::GateFade;

use super::cloak_fade::{hull_factor, translucent_factors};
use super::paint::paint_channels;
use super::ship_layers::base_alpha;
use super::view::npc_tint;

/// The alpha, 0 to 1, a ship's model draws at, cloaked as `cloak` shows
/// it, painted `paint`, at `gate`, of `BaseTransp` `base_transp` (see the
/// module docs), or `None` when it draws nothing.
#[must_use]
pub fn model_alpha(
    cloak: CloakShown,
    paint: Option<Rgb15>,
    gate: GateFade,
    base_transp: i16,
) -> Option<f32> {
    let paint = paint_channels(paint);
    let alpha = match cloak {
        CloakShown::Hidden => return None,
        CloakShown::Translucent(written) => share(translucent_factors(paint, written)),
        CloakShown::Fading(fade) => share(paint.map(|channel| hull_factor(channel, fade))),
        CloakShown::Opaque if gate.at_gate() => f32::from(npc_tint(gate)?.a) / 255.0,
        CloakShown::Opaque => f32::from(base_alpha(base_transp)) / 255.0,
    };
    Some(alpha).filter(|&alpha| alpha > 0.0)
}

/// The mean of `factors`, each in 32nds, as a share of 1.
fn share(factors: [u8; 3]) -> f32 {
    let sum: f32 = factors.iter().map(|&factor| f32::from(factor)).sum();
    sum / 3.0 / FADE_FULL
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const PAINT: Rgb15 = Rgb15 { r: 31, g: 16, b: 4 };

    fn of(cloak: CloakShown, gate: GateFade, base_transp: i16) -> Option<f32> {
        model_alpha(cloak, None, gate, base_transp)
    }

    fn alpha_of(level: u8) -> f32 {
        f32::from(level) / 255.0
    }

    fn entering() -> GateFade {
        GateFade::Entering { ticks_left: 9.0 }
    }

    #[test]
    fn an_opaque_ship_of_base_transp_0_draws_whole() {
        assert_eq!(of(CloakShown::Opaque, GateFade::None, 0), Some(1.0));
    }

    #[test]
    fn a_base_transparent_ship_draws_at_its_base_alpha() {
        for base_transp in [1, 10, 31] {
            assert_eq!(
                of(CloakShown::Opaque, GateFade::None, base_transp),
                Some(alpha_of(base_alpha(base_transp))),
                "{base_transp}"
            );
        }
        assert_ne!(alpha_of(base_alpha(10)), 1.0);
    }

    #[test]
    fn a_base_transp_of_32_or_more_draws_nothing() {
        for base_transp in [32, 40] {
            assert_eq!(of(CloakShown::Opaque, GateFade::None, base_transp), None);
        }
    }

    #[test]
    fn a_ship_at_a_gate_draws_at_the_gates_alpha_in_place_of_its_base() {
        let emerging = GateFade::Emerging { ticks_left: 3.0 };
        for gate in [entering(), emerging] {
            let tint = npc_tint(gate).expect("seen");
            assert_ne!(tint.a, 255, "{gate:?}");
            for base_transp in [0, 10] {
                assert_eq!(
                    of(CloakShown::Opaque, gate, base_transp),
                    Some(alpha_of(tint.a)),
                    "{gate:?} {base_transp}"
                );
            }
        }
    }

    #[test]
    fn a_ship_waiting_hidden_at_its_gate_draws_nothing() {
        let waiting = GateFade::Emerging { ticks_left: 100.0 };
        assert_eq!(npc_tint(waiting), None);
        assert_eq!(of(CloakShown::Opaque, waiting, 0), None);
    }

    #[test]
    fn a_fading_cloak_draws_at_the_hulls_share_in_place_of_a_gate_or_base() {
        let fade = 21.5;
        let unpainted = f32::from(hull_factor(32, fade)) / 32.0;
        for (gate, base_transp) in [(GateFade::None, 0), (entering(), 10)] {
            assert_eq!(
                of(CloakShown::Fading(fade), gate, base_transp),
                Some(unpainted)
            );
        }
        let channels = paint_channels(Some(PAINT)).map(|c| f32::from(hull_factor(c, fade)));
        assert_eq!(
            model_alpha(CloakShown::Fading(fade), Some(PAINT), GateFade::None, 0),
            Some((channels[0] + channels[1] + channels[2]) / 3.0 / 32.0)
        );
        assert_ne!(channels[0], channels[2], "the channels differ");
    }

    #[test]
    fn a_translucent_cloak_draws_at_the_full_fades_share() {
        let level = 0x1e;
        let share = |paint| {
            let factors = translucent_factors(paint_channels(paint), level).map(f32::from);
            (factors[0] + factors[1] + factors[2]) / 3.0 / 32.0
        };
        assert_eq!(
            of(CloakShown::Translucent(level), entering(), 10),
            Some(share(None))
        );
        assert_eq!(
            model_alpha(
                CloakShown::Translucent(level),
                Some(PAINT),
                GateFade::None,
                0
            ),
            Some(share(Some(PAINT)))
        );
        assert_ne!(share(None), share(Some(PAINT)));
    }

    #[test]
    fn a_hidden_cloak_draws_nothing() {
        assert_eq!(of(CloakShown::Hidden, GateFade::None, 0), None);
    }

    #[test]
    fn an_alpha_of_0_draws_nothing() {
        // Unpainted below 16 on every channel, a full fade's factors are
        // 0 at translucency 32.
        let dark = Rgb15 { r: 4, g: 4, b: 4 };
        assert_eq!(translucent_factors(paint_channels(Some(dark)), 32), [0; 3]);
        assert_eq!(
            model_alpha(CloakShown::Translucent(32), Some(dark), GateFade::None, 0),
            None
        );
    }
}
