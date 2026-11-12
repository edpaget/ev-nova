//! A jump's fuel ([`JUMP_FUEL`], [`max_jumps`]) and whether the ship can
//! jump ([`JumpReadiness`], [`JumpRefusal`]), from
//! `nova_sim_ai::hyperspace`, which re-exports them and keeps the star
//! map, its routes and the jump checks.

/// The fuel a jump uses (the Bible: "100 is one jump").
pub const JUMP_FUEL: f32 = 100.0;

/// How many whole jumps `fuel` holds: none for no fuel or less.
#[must_use]
pub fn max_jumps(fuel: f32) -> u32 {
    // `as` saturates a float into an integer: a negative becomes 0.
    (fuel / JUMP_FUEL).floor() as u32
}

/// Whether the ship can jump to the next system on its course, as the
/// nav area shows it (`_DrawStatusNav` @0x49f43-0x4a451 draws the
/// destination dim unless the ship is clear or a jump is under way).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JumpReadiness {
    /// J would be refused now, for any [`JumpRefusal`].
    Blocked,
    /// J would be accepted.
    Clear,
    /// J has been accepted: the ship is braking and turning before the
    /// jump, or is in hyperspace.
    Underway,
}

/// Why the ship cannot jump.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JumpRefusal {
    /// The ship has landed: it must take off first. The
    /// `nova_sim::Session` gives this before `nova_sim_ai::hyperspace::check_jump` is
    /// asked.
    Landed,
    /// The ship is disabled, breaking up or destroyed. The
    /// `nova_sim::Session` gives this before `nova_sim_ai::hyperspace::check_jump` is
    /// asked. The player is never told of it: the original's J skips the
    /// jump block for a disabled ship (`_HandlePlayer` @0x6b0f5 →
    /// 0x6bada), with no message and no beep.
    Disabled,
    /// No destination has been chosen, or it has been reached.
    NoDestination,
    /// The ship is nearer the system's centre than its jump distance.
    TooClose {
        /// How far from the centre it is.
        distance: f32,
    },
    /// The ship has less fuel than [`JUMP_FUEL`].
    NoFuel {
        /// How much it has.
        fuel: f32,
    },
    /// Another ship holds it in a tractor beam (`nova_sim_ai::hyperspace::hold_jump`).
    Tractored,
}

#[cfg(test)]
mod tests {
    use super::*;

    // The pinned value is exact.
    #[allow(clippy::float_cmp)]
    #[test]
    fn a_jump_uses_100_fuel() {
        assert_eq!(JUMP_FUEL, 100.0);
    }

    #[test]
    fn the_jumps_fuel_holds_are_whole_jumps_of_it() {
        assert_eq!(max_jumps(300.0), 3);
        assert_eq!(max_jumps(199.9), 1);
        assert_eq!(max_jumps(100.0), 1);
        assert_eq!(max_jumps(99.9), 0);
        assert_eq!(max_jumps(0.0), 0);
        assert_eq!(max_jumps(-250.0), 0, "none below none");
        assert_eq!(max_jumps(f32::MAX), u32::MAX, "saturating");
    }
}
