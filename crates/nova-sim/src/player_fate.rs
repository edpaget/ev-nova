//! The player's lifecycle: one [`PlayerFate`] for each stage of flying,
//! breaking up and dying, derived from the player's [`Condition`], and
//! one table row ([`FateRow`]) for what each stage allows.
//!
//! A "tick" here is the original's game frame: `_HandleShipDisplay`
//! (0x2b514) runs once per frame, which is the engine's sim tick.
//!
//! The states, and the `player-death` roadmap phase that owns each:
//!
//! - [`PlayerFate::Flying`], [`PlayerFate::Disabled`],
//!   [`PlayerFate::BreakingUp`] and [`PlayerFate::Exploded`]: phase 1.
//! - `Holding`, `Fading` and `GameOver`: phase 6.
//! - `Ejected`: phase 9.
//!
//! The columns of [`FateRow`], and their phases:
//!
//! - [`FateRow::drawn`] ([`Drawn`]): phase 1.
//! - drift (a disabled hull's 0.995 slowdown): phase 3.
//! - input (which keys a state accepts): phase 4.
//!
//! Condition changes stay where they are, in the fight's
//! `update_condition`; a fate is only read off them, so it adds no
//! second transition path. [`PlayerFate::BreakingUp`]'s timer is the
//! original's countdown there (phase 2): 3 x the `DeathDelay` for the
//! player, set on the breakup's first tick.

use crate::combat::hull::Condition;

/// Where the player's ship is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerFate {
    /// Flying and fighting ([`Condition::Intact`]).
    Flying,
    /// Disabled ([`Condition::Disabled`]).
    Disabled,
    /// Its armour is gone and it is breaking up ([`Condition::Dying`]).
    BreakingUp {
        /// The original's death timer: 3 x its `DeathDelay` as the
        /// breakup starts, exploding once it is down to 2.
        ticks_left: u32,
        /// Whether the timer was set on this tick, the original's "dying,
        /// timer not yet set" frame; phase 9's eject reads it.
        starting: bool,
    },
    /// It has made its final explosion ([`Condition::Destroyed`]).
    Exploded {
        /// Whether this is the tick it exploded on: the tick the fight
        /// raised its `CombatEvent::Destroyed`.
        first_tick: bool,
    },
}

/// How much of the player's ship is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drawn {
    /// The main sprite, the engine glow and the running lights.
    Full,
    /// The engine glow and the running lights, but not the main sprite:
    /// on the explosion frame the original hides only the main sprite
    /// (`SWSetSpriteVisible(sprite, 0)` at 0x2dcf7).
    SpriteHidden,
    /// Nothing: from the frame after the explosion the original's
    /// `active == 0` path hides every sprite layer each frame (test at
    /// 0x2b525, hides at 0x2b555-0x2b605).
    Hidden,
}

/// What one [`PlayerFate`] allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FateRow {
    /// How much of the ship is drawn.
    pub drawn: Drawn,
}

impl PlayerFate {
    /// The fate of a player whose ship is in `condition`;
    /// `exploded_this_tick` says whether the ship was destroyed on the
    /// latest tick, and matters only to a destroyed ship.
    #[must_use]
    pub fn of(condition: Condition, exploded_this_tick: bool) -> Self {
        match condition {
            Condition::Intact => Self::Flying,
            Condition::Disabled => Self::Disabled,
            Condition::Dying {
                ticks_left,
                starting,
            } => Self::BreakingUp {
                ticks_left,
                starting,
            },
            Condition::Destroyed => Self::Exploded {
                first_tick: exploded_this_tick,
            },
        }
    }

    /// This fate's row of the table.
    #[must_use]
    pub fn row(self) -> FateRow {
        match self {
            Self::Flying | Self::Disabled | Self::BreakingUp { .. } => {
                FateRow { drawn: Drawn::Full }
            }
            Self::Exploded { first_tick: true } => FateRow {
                drawn: Drawn::SpriteHidden,
            },
            Self::Exploded { first_tick: false } => FateRow {
                drawn: Drawn::Hidden,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_condition_maps_to_its_fate() {
        for exploded in [false, true] {
            assert_eq!(
                PlayerFate::of(Condition::Intact, exploded),
                PlayerFate::Flying
            );
            assert_eq!(
                PlayerFate::of(Condition::Disabled, exploded),
                PlayerFate::Disabled
            );
            for starting in [false, true] {
                assert_eq!(
                    PlayerFate::of(
                        Condition::Dying {
                            ticks_left: 7,
                            starting
                        },
                        exploded
                    ),
                    PlayerFate::BreakingUp {
                        ticks_left: 7,
                        starting
                    }
                );
            }
        }
        assert_eq!(
            PlayerFate::of(Condition::Destroyed, true),
            PlayerFate::Exploded { first_tick: true }
        );
        assert_eq!(
            PlayerFate::of(Condition::Destroyed, false),
            PlayerFate::Exploded { first_tick: false }
        );
    }

    #[test]
    fn flying_disabled_and_breaking_up_players_are_drawn_in_full() {
        for fate in [
            PlayerFate::Flying,
            PlayerFate::Disabled,
            PlayerFate::BreakingUp {
                ticks_left: 3,
                starting: false,
            },
            PlayerFate::BreakingUp {
                ticks_left: 15,
                starting: true,
            },
        ] {
            assert_eq!(fate.row().drawn, Drawn::Full, "{fate:?}");
        }
    }

    #[test]
    fn an_exploded_player_hides_its_sprite_on_its_first_tick() {
        assert_eq!(
            PlayerFate::Exploded { first_tick: true }.row().drawn,
            Drawn::SpriteHidden
        );
    }

    #[test]
    fn an_exploded_player_is_hidden_from_the_tick_after() {
        assert_eq!(
            PlayerFate::Exploded { first_tick: false }.row().drawn,
            Drawn::Hidden
        );
    }
}
