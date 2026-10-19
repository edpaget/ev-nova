//! A gate's colorize: the white an NPC is washed toward while it enters a
//! hypergate or comes out of one, as `_HandleShipDisplay`
//! (@0x2bf94-0x2c18e) writes it in the `EV Nova` executable (rdm
//! ship-handling-fidelity phase 23, findings 12-15).
//!
//! - **The block** runs after the ion tint (@0x2beb9-0x2bf8a) and for NPCs
//!   only (`cmpw $0,0x86` @0x2bf94 skips the player's slot). Its floats:
//!   0xdd650 (−2.0), 0xdd654 (32.0), 0xdd048 (16.0) and 0xdd068 (1.0); w
//!   is the AI wait (+0x4c).
//! - **Entering** (`_AIShipIsEnteringHypergate` @0x89502, sub-goal 0x17,
//!   [`Mode::EnteringGate`]): the hull's t = trunc(−2.0 × w + 32.0),
//!   clamped to 0..32 (@0x2bfb2-0x2c007); at 16-bit its colorize level is
//!   min(2t, 32) (@0x2c021-0x2c045), toward 0xffff (@0x2c05c).
//! - **Emerging** (`_AIShipIsEmergingFromHypergate` @0x7e2e8, goal 0x15,
//!   [`Mode::Emerging`]): w above 16.0 hides the ship (@0x2c0ad,
//!   @0x2c18e); otherwise d = trunc(16.0 − w), clamped to 0..16
//!   (@0x2c0c8-0x2c0e2), t = 32 − 2d, clamped to 0..32
//!   (@0x2c0e5-0x2c12b), and the level min(2t, 32) toward 0xffff
//!   (@0x2c142-0x2c17b).
//! - **In the engine's terms** t is 32 less the fade the view draws as the
//!   hull's alpha (`npc_tint`: [`enter_level`] = min(2w, 32),
//!   [`emerge_level`] = 32 − 2w), so the level is
//!   min(2 × (32 − fade), 32) ([`gate_colorize`]): entering, min(64 − 4w,
//!   32) once w is below 16, full from w ≤ 8; emerging, min(4w, 32). The
//!   colour, RGB555 0xffff, is white (31/31/31). The factors stay those
//!   written before it, so the blit is the colorize and then the
//!   (32 − t)/32 crossfade.
//! - **Over the rest.** It overrides the murk and the ion tint, both
//!   written before it, and the layers copy it after it, as they copy an
//!   ion tint ([`ShipDraw::colorize`](super::ship_layers::ShipDraw::colorize)):
//!   the alt and shield layers at its level and colour; the glow and
//!   weapon glow at its level and the lights at trunc(level × 0.333), each
//!   held below trunc(32 − 1.5 × level), so they draw nothing from level
//!   22.
//!
//! The predicates read only the goals, so a ship passing through a
//! wormhole is washed white as well; the engine's two modes cover both.
//! The block plays no sound. The 8-bit paths (no colorize, an emerging
//! ship hidden through its fade) aren't modelled.
//!
//! [`enter_level`]: nova_sim::traffic::gate_entry::enter_level
//! [`emerge_level`]: nova_sim::traffic::emerge::emerge_level

use nova_sim::traffic::emerge::{FULL_LEVEL, emerge_level};
use nova_sim::traffic::gate_entry::enter_level;
use nova_sim::traffic::npc::Mode;

use crate::Colorize;

/// The RGB555 pixel a gate washes a ship toward: white (0xffff @0x2c05c,
/// @0x2c17b).
pub const GATE_WHITE: u16 = 0xffff;

/// The colorize an NPC in `mode` takes from its gate: toward white at
/// min(2 × (32 − fade), 32), its fade the level [`enter_level`] or
/// [`emerge_level`] gives; none while it waits hidden, or when it isn't
/// at a gate (see the module docs).
///
/// [`enter_level`]: nova_sim::traffic::gate_entry::enter_level
/// [`emerge_level`]: nova_sim::traffic::emerge::emerge_level
#[must_use]
pub fn gate_colorize(mode: &Mode) -> Option<Colorize> {
    let fade = match *mode {
        Mode::EnteringGate { ticks_left, .. } => enter_level(ticks_left),
        Mode::Emerging { ticks_left } => emerge_level(ticks_left)?,
        _ => return None,
    };
    let level = (2 * (FULL_LEVEL - fade.min(FULL_LEVEL))).min(FULL_LEVEL);
    Some(Colorize::toward_pixel(level, GATE_WHITE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entering(ticks_left: u32) -> Mode {
        Mode::EnteringGate {
            gate: nova_sim::StellarId(130),
            ticks_left,
        }
    }

    fn level(mode: Mode) -> Option<u8> {
        gate_colorize(&mode).map(|colorize| colorize.level)
    }

    #[test]
    fn entering_a_gate_washes_it_white_from_16_ticks_out_and_fully_from_8() {
        let levels: Vec<_> = [16, 12, 8, 0].iter().map(|&t| level(entering(t))).collect();
        assert_eq!(levels, [Some(0), Some(16), Some(32), Some(32)]);
    }

    #[test]
    fn emerging_it_is_hidden_then_unwashed_by_4_a_tick() {
        assert_eq!(level(Mode::Emerging { ticks_left: 17 }), None, "hidden");
        let levels: Vec<_> = [16, 8, 4, 1, 0]
            .iter()
            .map(|&ticks_left| level(Mode::Emerging { ticks_left }))
            .collect();
        assert_eq!(levels, [Some(32), Some(32), Some(16), Some(4), Some(0)]);
    }

    #[test]
    fn a_ship_at_no_gate_takes_none() {
        assert_eq!(gate_colorize(&Mode::Flying), None);
        assert_eq!(gate_colorize(&Mode::Adrift { ticks_left: 3 }), None);
    }

    #[test]
    fn the_wash_is_white() {
        let colorize = gate_colorize(&Mode::Emerging { ticks_left: 3 }).expect("washed");
        assert_eq!(colorize.rgb5, [31, 31, 31]);
    }
}
