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
//! - [`FateRow::drag`] (a disabled hull's 0.995 slowdown): phase 3.
//! - [`FateRow::input`] ([`Takes`], which [`PlayerCommand`]s a state
//!   accepts): phase 4.
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
    /// Whether the hull slows by [`crate::flight::DISABLED_DRAG`] each tick
    /// (`_HandlePlayer` @0x68abf-0x68af8, gated only by the `_IsDisabled`
    /// flag -0x2cb, which holds for a breaking-up ship through its armour
    /// test). A destroyed player is stopped dead instead.
    pub drag: bool,
    /// Which of the player's commands the session takes.
    ///
    /// - Flying and disabled: every one. The session refuses on its own
    ///   what the original refuses a disabled ship (`JumpRefusal::Disabled`,
    ///   `LandingRefusal::Disabled`), so the view keeps sending them and
    ///   the refusals still show.
    /// - Breaking up: all but the map, land and mission info, which
    ///   `_HandlePlayer` gates on the breakup timer +0x3c > 0 (0x6a7b3,
    ///   0x6a8cb, 0x7044b), all three silently. Hail, board and jump stay
    ///   taken: the session's own refusals answer them, as the original's
    ///   handlers do (`_HandlePlayerCommunication` 0x61f75,
    ///   `_HandlePlayerBoardAttempt` 0x650c9, `_IsDisabled` at 0x6b0f5).
    /// - Exploded: none. The dead branch (0x6870d-0x6888c) skips every
    ///   key handler.
    pub input: Takes,
}

/// A command the flight view sends the session for one of the player's
/// keys (or a click), named after its keymap slot in the original.
///
/// Player info (slot 0x32), pause (slot 0x2e), cloak (slot 0x52) and
/// self-destruct (slot 0x24) have no variant: each joins when its key is
/// built. Their refusals while breaking up are recorded in the
/// `player-death` roadmap's phase 4: pause at 0x45ef0, player info at
/// 0x7031d, cloak at 0x705d6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCommand {
    /// The galaxy map: `mapKey` (slot 0x12).
    Map,
    /// A landing request: `landReq` (slot 0xa).
    Land,
    /// A hyperspace jump: `jumpKey` (slot 0x1c).
    Jump,
    /// The mission info dialog: `missionInfoKey` (slot 0x50).
    MissionInfo,
    /// Hailing the target: `hailKey` (slot 0x8).
    Hail,
    /// Boarding the target: `boardKey` (slot 0x20).
    Board,
    /// Jettisoning cargo: Command + `dumpCargo` (slot 0x1e).
    Jettison,
    /// Choosing a target: `targSel` (slot 0x14) and `closeTarg` (slot 0x16).
    Target,
    /// Choosing a nav target: the stellar keys (1-5, F1-F5), the nearest
    /// stellar, `navOff` (slot 0x10) and a click in space.
    Nav,
    /// Choosing the next system: `hyperSel` (slot 0x1a).
    SelectSystem,
    /// Choosing a secondary weapon: `weapSelect` (slot 0x0).
    SelectWeapon,
    /// The escort menu, its acknowledgement and group keys, and the escort
    /// commands (slots 0x54, 0x60-0x66).
    Escort,
}

impl PlayerCommand {
    /// Every command, for tests that go over them all.
    pub const ALL: [PlayerCommand; 12] = [
        Self::Map,
        Self::Land,
        Self::Jump,
        Self::MissionInfo,
        Self::Hail,
        Self::Board,
        Self::Jettison,
        Self::Target,
        Self::Nav,
        Self::SelectSystem,
        Self::SelectWeapon,
        Self::Escort,
    ];
}

/// Which [`PlayerCommand`]s a [`FateRow`] takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Takes {
    /// Every command.
    Every,
    /// Every command but these.
    AllBut(&'static [PlayerCommand]),
    /// No command.
    Nothing,
}

impl Takes {
    /// Whether `command` is taken.
    #[must_use]
    pub fn takes(self, command: PlayerCommand) -> bool {
        match self {
            Self::Every => true,
            Self::AllBut(refused) => !refused.contains(&command),
            Self::Nothing => false,
        }
    }
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
            Self::Flying => FateRow {
                drawn: Drawn::Full,
                drag: false,
                input: Takes::Every,
            },
            Self::Disabled => FateRow {
                drawn: Drawn::Full,
                drag: true,
                input: Takes::Every,
            },
            Self::BreakingUp { .. } => FateRow {
                drawn: Drawn::Full,
                drag: true,
                input: Takes::AllBut(&[
                    PlayerCommand::Map,
                    PlayerCommand::Land,
                    PlayerCommand::MissionInfo,
                ]),
            },
            Self::Exploded { first_tick: true } => FateRow {
                drawn: Drawn::SpriteHidden,
                drag: false,
                input: Takes::Nothing,
            },
            Self::Exploded { first_tick: false } => FateRow {
                drawn: Drawn::Hidden,
                drag: false,
                input: Takes::Nothing,
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
    fn disabled_and_breaking_up_players_drag() {
        for fate in [
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
            assert!(fate.row().drag, "{fate:?}");
        }
    }

    #[test]
    fn flying_and_exploded_players_do_not_drag() {
        for fate in [
            PlayerFate::Flying,
            PlayerFate::Exploded { first_tick: true },
            PlayerFate::Exploded { first_tick: false },
        ] {
            assert!(!fate.row().drag, "{fate:?}");
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

    const BREAKING_UP: [PlayerFate; 2] = [
        PlayerFate::BreakingUp {
            ticks_left: 3,
            starting: false,
        },
        PlayerFate::BreakingUp {
            ticks_left: 15,
            starting: true,
        },
    ];

    const REFUSED_WHILE_BREAKING_UP: [PlayerCommand; 3] = [
        PlayerCommand::Map,
        PlayerCommand::Land,
        PlayerCommand::MissionInfo,
    ];

    #[test]
    fn flying_and_disabled_players_take_every_command() {
        for fate in [PlayerFate::Flying, PlayerFate::Disabled] {
            for command in PlayerCommand::ALL {
                assert!(fate.row().input.takes(command), "{fate:?} {command:?}");
            }
        }
    }

    #[test]
    fn breaking_up_players_refuse_the_map_landing_and_mission_info() {
        for fate in BREAKING_UP {
            for command in REFUSED_WHILE_BREAKING_UP {
                assert!(!fate.row().input.takes(command), "{fate:?} {command:?}");
            }
        }
    }

    #[test]
    fn breaking_up_players_take_targeting_nav_board_jettison_hail_jump_and_escort_commands() {
        for fate in BREAKING_UP {
            for command in PlayerCommand::ALL
                .into_iter()
                .filter(|command| !REFUSED_WHILE_BREAKING_UP.contains(command))
            {
                assert!(fate.row().input.takes(command), "{fate:?} {command:?}");
            }
        }
    }

    #[test]
    fn exploded_players_take_no_command() {
        for first_tick in [false, true] {
            let fate = PlayerFate::Exploded { first_tick };
            for command in PlayerCommand::ALL {
                assert!(!fate.row().input.takes(command), "{fate:?} {command:?}");
            }
        }
    }

    #[test]
    fn takes_every_takes_all_and_nothing_takes_none() {
        for command in PlayerCommand::ALL {
            assert!(Takes::Every.takes(command), "{command:?}");
            assert!(!Takes::Nothing.takes(command), "{command:?}");
        }
    }

    #[test]
    fn takes_all_but_refuses_only_the_listed_commands() {
        let takes = Takes::AllBut(&[PlayerCommand::Board, PlayerCommand::Hail]);
        for command in PlayerCommand::ALL {
            let listed = matches!(command, PlayerCommand::Board | PlayerCommand::Hail);
            assert_eq!(takes.takes(command), !listed, "{command:?}");
        }
    }
}
