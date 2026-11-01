//! Where a glide in or a burst out of a gate starts: the burst word +0x34
//! a ship carries as sub 0xa takes it (`_LowLevelAIHandler` @0x88114-0x8813d,
//! addresses in the `EV Nova` executable).
//!
//! **The original.** Sub 0xa writes +0x34 = -50.0 only over a word at or
//! above 0.0 (@0x8811e-0x8812c), and +0x30 = -1.165 every frame
//! (@0x88136). `_HandleShip`'s burst block (@0x34a54-0x34d93) runs the
//! hyperspace glide and the gate's burst alike, from |+0x34| down by
//! 1.165 a frame to the ship's cap (@0x34d10-0x34d2a). So:
//!
//! - a ship coming in from hyperspace carries no negative word, and glides
//!   from 50 ([`JUMP_IN_SPEED`]);
//! - a ship out of a gate bursts from the -30.0 or -15.0 its emerge left
//!   (@0x895aa-0x895bd, [`exit_speed`]);
//! - a ship that took goal 0x15 from a lost lead
//!   ([`succession`](super::succession), @0x893c8-0x893d0) carries no
//!   emerge's word but whatever it already owed ([`Npc::owed_burst`]): it
//!   glides or bursts from that, else from 50. One that took it under a
//!   wait is marked so ([`Npc::bursts_own_word`]) for the gate burst it
//!   runs once the wait is out.
//!
//! **Here.** [`Npc::burst_word`] is the word, [`Npc::burst_speed`] the
//! speed on each tick, which the glide, the burst, a glide or burst cut
//! short ([`Npc::leaves_arrival`]) and a hold at the gate ending
//! ([`Npc::end_wait`]) all read.
//!
//! Nothing here plays a sound or draws a random number.

use super::emerge::exit_speed;
use super::npc::{Mode, Npc};
use super::spawn::{JUMP_IN_SLOWING, JUMP_IN_SPEED};

impl Npc {
    /// Its burst word, |+0x34| while it is negative, as sub 0xa finds it
    /// (see the module docs): in or at a gate, or bursting out of one, the
    /// emerge's 30 or 15, but for a ship [marked](Npc::bursts_own_word) as
    /// carrying its own; otherwise, and for such a ship, what it owes
    /// ([`Npc::owed_burst`]), if anything.
    #[must_use]
    pub fn burst_word(&self) -> Option<f32> {
        let at_gate = matches!(
            self.mode,
            Mode::Emerging { .. } | Mode::HeldAtGate { .. } | Mode::LeavingGate { .. }
        );
        if at_gate && !self.bursts_own_word {
            Some(exit_speed(self.led_out, 0))
        } else {
            self.owed_burst
        }
    }

    /// Its speed on tick `tick` of its glide or burst: from its
    /// [`burst_word`](Npc::burst_word), or from [`JUMP_IN_SPEED`] with
    /// none, as sub 0xa starts it (@0x8811e-0x8812c), [`JUMP_IN_SLOWING`]
    /// slower each tick.
    #[must_use]
    pub fn burst_speed(&self, tick: u32) -> f32 {
        let start = self.burst_word().unwrap_or(JUMP_IN_SPEED);
        // A glide's ticks stay far below f32's exact range.
        #[allow(clippy::cast_precision_loss)]
        JUMP_IN_SLOWING.mul_add(-(tick as f32), start)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::StellarId;
    use crate::stats::ShipStats;
    use crate::traffic::emerge::{EXIT_SPEED, LED_EXIT_SPEED};
    use crate::traffic::spawn::glide_speed;

    fn in_mode(mode: Mode, owed: Option<f32>) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::default());
        npc.mode = mode;
        npc.owed_burst = owed;
        npc
    }

    #[test]
    fn a_glide_starts_from_what_it_owes_else_from_the_top() {
        let gliding = Mode::JumpingIn { tick: 0 };
        assert_eq!(in_mode(gliding, None).burst_speed(0), JUMP_IN_SPEED);
        assert_eq!(in_mode(gliding, None).burst_speed(3), glide_speed(3));
        let owing = in_mode(gliding, Some(12.0));
        assert_eq!(owing.burst_speed(0), 12.0);
        assert_eq!(owing.burst_speed(2), JUMP_IN_SLOWING.mul_add(-2.0, 12.0));
    }

    #[test]
    fn a_ship_at_its_gate_carries_its_emerges_word() {
        for mode in [
            Mode::Emerging { ticks_left: 3 },
            Mode::HeldAtGate { ticks_left: 3 },
            Mode::LeavingGate { tick: 2 },
        ] {
            let mut npc = in_mode(mode, Some(12.0));
            npc.emerges_from = Some(StellarId(300));
            assert_eq!(npc.burst_word(), Some(EXIT_SPEED), "{mode:?}");
            assert_eq!(npc.burst_speed(2), exit_speed(false, 2), "{mode:?}");
            npc.led_out = true;
            assert_eq!(npc.burst_word(), Some(LED_EXIT_SPEED), "{mode:?}");
        }
    }

    #[test]
    fn a_ship_marked_as_carrying_its_own_word_bursts_from_what_it_owes_else_from_the_top() {
        for mode in [
            Mode::Emerging { ticks_left: 3 },
            Mode::HeldAtGate { ticks_left: 3 },
            Mode::LeavingGate { tick: 0 },
        ] {
            for owed in [None, Some(12.0)] {
                let mut npc = in_mode(mode, owed);
                npc.bursts_own_word = true;
                npc.led_out = true;
                assert_eq!(npc.burst_word(), owed, "{mode:?}");
                assert_eq!(npc.burst_speed(0), owed.unwrap_or(JUMP_IN_SPEED));
            }
        }
    }

    #[test]
    fn a_flying_ship_carries_what_it_owes() {
        assert_eq!(in_mode(Mode::Flying, Some(4.0)).burst_word(), Some(4.0));
        assert_eq!(in_mode(Mode::Flying, None).burst_word(), None);
    }
}
