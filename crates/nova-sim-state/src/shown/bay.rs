//! What became of the player's fighters, [`FighterNote`], and the lines
//! that tell it ([`ABANDONED_ONE`], [`ABANDONED_MANY`]), from
//! `nova_sim_combat::bay`, which re-exports them and keeps the bays.

/// `STR#` 2002 #164: one fighter abandoned.
pub const ABANDONED_ONE: &str = "fighter abandoned";

/// `STR#` 2002 #165: several fighters abandoned.
pub const ABANDONED_MANY: &str = "fighters abandoned";

/// What became of the player's fighters, for the flight to tell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FighterNote {
    /// This many fighters were abandoned as the player left the system.
    Abandoned(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strings_are_str_2002s() {
        assert_eq!(ABANDONED_ONE, "fighter abandoned");
        assert_eq!(ABANDONED_MANY, "fighters abandoned");
    }
}
