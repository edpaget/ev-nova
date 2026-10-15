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
//! - [`PlayerFate::Holding`], [`PlayerFate::Fading`] and
//!   [`PlayerFate::GameOver`]: phase 6.
//! - [`PlayerFate::Ejected`]: phase 9.
//!
//! The columns of [`FateRow`], and their phases:
//!
//! - [`FateRow::drawn`] ([`Drawn`]): phase 1.
//! - [`FateRow::drag`] (a disabled hull's 0.995 slowdown): phase 3.
//! - [`FateRow::input`] ([`Takes`], which [`PlayerCommand`]s a state
//!   accepts): phase 4.
//! - [`FateRow::saves`] (whether the pilot may be saved, and so whether
//!   the player is dying for phase 7's strict-play delete): phase 6.
//!
//! Condition changes stay where they are, in the fight's
//! `update_condition`; a fate is only read off them, so it adds no
//! second transition path. [`PlayerFate::BreakingUp`]'s timer is the
//! original's countdown there (phase 2): 3 x the `DeathDelay` for the
//! player, set on the breakup's first tick. After the explosion tick the
//! session keeps how far the game's end has got, an [`Ending`], which the
//! fate of a destroyed player reads.

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
    /// It has made its final explosion ([`Condition::Destroyed`]), on
    /// this tick: the tick the fight raised its `CombatEvent::Destroyed`.
    Exploded,
    /// The hold after the explosion: the scene runs on without the ship.
    /// It starts as the ship goes inactive, which `_HandlePlayer` tests
    /// (0x68700), and counts down in its dead branch (0x6871e-0x68734)
    /// once a tick; `ticks_left` is the hold ticks still to run, from
    /// [`HOLD_TICKS`]. An expired hold still reads `ticks_left: 0`, until
    /// the fade begins: [`Ending::expired`] tells the two apart.
    Holding {
        /// The hold ticks still to run.
        ticks_left: u32,
    },
    /// The fade to black, from the hold's expiry: the original sets
    /// `gPause = 1` (0x68838), `_PlayGame` exits (0x465eb) and
    /// `_FadeScreenOut` (0x465f7) fades the display out.
    Fading,
    /// The fade has ended and the main screen returns: the window painted
    /// black (0x4664a) and `_EnterGameFromMainScreen` (0x1a471).
    GameOver,
    /// In the escape pod after an eject, counting down to the rescue: the
    /// original's pod countdown +0xc908 while it is above 0, set to
    /// [`POD_TICKS`] by the eject (@0x6f669) and taken down a tick by the
    /// pod's flight (@0x68d95-0x68dcd). A running pod wins over the
    /// condition: the pod's frames return (@0x68ddd) before anything else
    /// `_HandlePlayer` does.
    Ejected {
        /// The pod ticks still to run before the rescue.
        ticks_left: u32,
    },
}

/// The escape pod's countdown, in ticks: the eject sets +0xc908 to 350
/// (0x15e, @0x6f669).
pub const POD_TICKS: u32 = 350;

/// The hold after the explosion, in ticks: the dead branch takes 1.0
/// (0xdd068) off the death timer while it is above -240.0 (0xdd958),
/// starting from the 2.0 the ship explodes at, so it decrements on
/// 2.0 - (-240.0) = 242 ticks and expires on the 243rd.
pub const HOLD_TICKS: u32 = 242;

/// How far a destroyed player's game has got after its explosion tick:
/// the stage the session keeps, which [`PlayerFate::of`] reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// The hold, with `ticks_left` still to count down.
    Holding {
        /// The hold ticks still to run.
        ticks_left: u32,
    },
    /// The hold has run out, and the fade has not begun: the seam where
    /// the sabotage dialogs show over the flight scene. It reads
    /// as [`PlayerFate::Holding`] with no ticks left.
    Expired,
    /// [`PlayerFate::Fading`].
    Fading,
    /// [`PlayerFate::GameOver`].
    GameOver,
}

impl Ending {
    /// Whether the hold has run out: [`Ending::Expired`] and every stage
    /// after it.
    #[must_use]
    pub fn expired(self) -> bool {
        !matches!(self, Self::Holding { .. })
    }
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
    ///   and `LandingRefusal::Disabled` once the land key has selected and
    ///   checked landability), so the view keeps sending them and those
    ///   refusals still show.
    /// - Breaking up: all but the map, land and mission info, which
    ///   `_HandlePlayer` gates on the breakup timer +0x3c > 0 (0x6a7b3,
    ///   0x6a8cb, 0x7044b), all three silently. Hail, board and jump stay
    ///   taken: the session's own refusals answer them, as the original's
    ///   handlers do (`_HandlePlayerCommunication` 0x61f75,
    ///   `_HandlePlayerBoardAttempt` 0x650c9, `_IsDisabled` at 0x6b0f5).
    /// - Exploded: none. The dead branch (0x6870d-0x6888c) skips every
    ///   key handler.
    /// - Holding, fading and game over: none, the same dead branch.
    /// - Ejected: none. The pod's frames return (@0x68ddd) before any key
    ///   handler, and `_PlayGame`'s pause gate refuses pause in the pod
    ///   (class 0x2ff @0x45ee3, +0xc908 > 0 @0x45ef5): a pause key, when
    ///   one is built, asks this row and is refused too.
    pub input: Takes,
    /// Whether the pilot may be saved: only while the player is not dying
    /// or dead, as `_IsDying` (@0x253d: the breakup timer above 0, or the
    /// armour at 0 or below) reads it. The original's saves can't come
    /// while dying (`_SavePilotFile` @0x76876 is called only on creation,
    /// landing and pod rescue), and nothing saves after the death, so no
    /// route here saves a dying or dead pilot either. The strict-play
    /// delete (`Session::deletes_pilot_on_ending_play`, phase 7) reads the
    /// same answer as `_IsDying`: a pilot that may not be saved is dying.
    ///
    /// Nor may a pilot in the escape pod be saved: the original saves only
    /// on landing, creation and the pod's arrival, and the pod countdown
    /// is never saved, so a save from the pod would reload a pilot stuck
    /// in `shïp` 895 with no rescue to come. The strict-play delete still
    /// spares the pod, by its own test of the class (`_EnterGameFromMainScreen`
    /// @0x1a4c5; `Session::deletes_pilot_on_ending_play`).
    pub saves: bool,
}

/// A command the flight view sends the session for one of the player's
/// keys (or a click), named after its keymap slot in the original.
///
/// Pause (slot 0x2e), cloak (slot 0x52) and self-destruct (slot 0x24)
/// have no variant: each joins when its key is built, by the phase that
/// owns it (rdm:task/cloaking-device for the cloak,
/// rdm:phase/ship-handling-fidelity/phase-13-player-self-destruct for the
/// self-destruct). The breaking-up row refuses pause (0x45ef0) and the
/// cloak (0x705d6, silently), and takes the self-destruct: only its start
/// is gated on dying (0x6fdf0), which the session refuses, and a
/// countdown already running goes on (0x6fedf, 0x7019d). The eject (slot
/// 0x22) and the player info (slot 0x32) have theirs.
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
    /// The eject: Command + `ejectKey` (slot 0x22), held together
    /// (`_HandlePlayer` @0x6f29e-0x6f2c1). Flying players take it, and the
    /// session refuses it silently unless the ship is disabled or dying
    /// (@0x6f22c).
    Eject,
    /// The player info dialog: `playerInfoKey` (slot 0x32, read at
    /// 0x702cb). Flying, disabled and breaking-up players take it, and the
    /// session refuses it, with the shared refusal beep, during a jump and
    /// while the ship breaks up
    /// ([`Session::press_player_info`](crate::Session::press_player_info)),
    /// as `_HandlePlayer` does (@0x702f7, @0x7031d). The dead branch and
    /// the pod's frames reach no key handler, so the other rows refuse it
    /// in silence.
    PlayerInfo,
}

impl PlayerCommand {
    /// Every command, for tests that go over them all.
    pub const ALL: [PlayerCommand; 14] = [
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
        Self::Eject,
        Self::PlayerInfo,
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
    /// latest tick, and `ending` how far the game's end has got since.
    /// Both matter only to a destroyed ship; one with no ending recorded
    /// is at the hold's start. A running `pod` countdown wins over all of
    /// them: the player is [`PlayerFate::Ejected`].
    #[must_use]
    pub fn of(
        condition: Condition,
        exploded_this_tick: bool,
        ending: Option<Ending>,
        pod: Option<u32>,
    ) -> Self {
        if let Some(ticks_left) = pod {
            return Self::Ejected { ticks_left };
        }
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
            Condition::Destroyed if exploded_this_tick => Self::Exploded,
            Condition::Destroyed => match ending {
                None => Self::Holding {
                    ticks_left: HOLD_TICKS,
                },
                Some(Ending::Holding { ticks_left }) => Self::Holding { ticks_left },
                Some(Ending::Expired) => Self::Holding { ticks_left: 0 },
                Some(Ending::Fading) => Self::Fading,
                Some(Ending::GameOver) => Self::GameOver,
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
                saves: true,
            },
            Self::Disabled => FateRow {
                drawn: Drawn::Full,
                drag: true,
                input: Takes::Every,
                saves: true,
            },
            Self::BreakingUp { .. } => FateRow {
                drawn: Drawn::Full,
                drag: true,
                input: Takes::AllBut(&[
                    PlayerCommand::Map,
                    PlayerCommand::Land,
                    PlayerCommand::MissionInfo,
                ]),
                saves: false,
            },
            Self::Exploded => FateRow {
                drawn: Drawn::SpriteHidden,
                drag: false,
                input: Takes::Nothing,
                saves: false,
            },
            Self::Holding { .. } | Self::Fading | Self::GameOver => FateRow {
                drawn: Drawn::Hidden,
                drag: false,
                input: Takes::Nothing,
                saves: false,
            },
            Self::Ejected { .. } => FateRow {
                drawn: Drawn::Full,
                drag: false,
                input: Takes::Nothing,
                saves: false,
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
            for ending in [None, Some(Ending::Fading)] {
                assert_eq!(
                    PlayerFate::of(Condition::Intact, exploded, ending, None),
                    PlayerFate::Flying
                );
                assert_eq!(
                    PlayerFate::of(Condition::Disabled, exploded, ending, None),
                    PlayerFate::Disabled
                );
                for starting in [false, true] {
                    assert_eq!(
                        PlayerFate::of(
                            Condition::Dying {
                                ticks_left: 7,
                                starting
                            },
                            exploded,
                            ending,
                            None
                        ),
                        PlayerFate::BreakingUp {
                            ticks_left: 7,
                            starting
                        }
                    );
                }
            }
        }
        for ending in [None, Some(Ending::Holding { ticks_left: 9 })] {
            assert_eq!(
                PlayerFate::of(Condition::Destroyed, true, ending, None),
                PlayerFate::Exploded
            );
        }
    }

    #[test]
    fn a_destroyed_player_after_its_explosion_tick_reads_its_ending() {
        let after = |ending| PlayerFate::of(Condition::Destroyed, false, ending, None);
        assert_eq!(
            after(Some(Ending::Holding { ticks_left: 9 })),
            PlayerFate::Holding { ticks_left: 9 }
        );
        assert_eq!(
            after(Some(Ending::Expired)),
            PlayerFate::Holding { ticks_left: 0 }
        );
        assert_eq!(after(Some(Ending::Fading)), PlayerFate::Fading);
        assert_eq!(after(Some(Ending::GameOver)), PlayerFate::GameOver);
        assert_eq!(
            after(None),
            PlayerFate::Holding {
                ticks_left: HOLD_TICKS
            },
            "no ending recorded: the hold's start"
        );
    }

    #[test]
    fn the_hold_is_242_ticks() {
        assert_eq!(HOLD_TICKS, 242);
    }

    #[test]
    fn only_an_expired_or_later_ending_has_expired() {
        assert!(!Ending::Holding { ticks_left: 0 }.expired());
        assert!(!Ending::Holding { ticks_left: 5 }.expired());
        for ending in [Ending::Expired, Ending::Fading, Ending::GameOver] {
            assert!(ending.expired(), "{ending:?}");
        }
    }

    const AFTER_THE_EXPLOSION: [PlayerFate; 4] = [
        PlayerFate::Holding { ticks_left: 0 },
        PlayerFate::Holding { ticks_left: 100 },
        PlayerFate::Fading,
        PlayerFate::GameOver,
    ];

    #[test]
    fn holding_fading_and_game_over_hide_the_ship_take_no_command_and_do_not_drag() {
        for fate in AFTER_THE_EXPLOSION {
            let row = fate.row();
            assert_eq!(row.drawn, Drawn::Hidden, "{fate:?}");
            assert!(!row.drag, "{fate:?}");
            for command in PlayerCommand::ALL {
                assert!(!row.input.takes(command), "{fate:?} {command:?}");
            }
        }
    }

    #[test]
    fn only_flying_and_disabled_players_may_be_saved() {
        for fate in [PlayerFate::Flying, PlayerFate::Disabled] {
            assert!(fate.row().saves, "{fate:?}");
        }
        for fate in BREAKING_UP
            .into_iter()
            .chain([PlayerFate::Exploded])
            .chain(AFTER_THE_EXPLOSION)
        {
            assert!(!fate.row().saves, "{fate:?}");
        }
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
        for fate in [PlayerFate::Flying, PlayerFate::Exploded] {
            assert!(!fate.row().drag, "{fate:?}");
        }
    }

    #[test]
    fn an_exploded_player_hides_its_sprite_on_its_first_tick() {
        assert_eq!(PlayerFate::Exploded.row().drawn, Drawn::SpriteHidden);
    }

    #[test]
    fn a_holding_player_is_hidden() {
        assert_eq!(
            PlayerFate::Holding {
                ticks_left: HOLD_TICKS
            }
            .row()
            .drawn,
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
        let fate = PlayerFate::Exploded;
        for command in PlayerCommand::ALL {
            assert!(!fate.row().input.takes(command), "{fate:?} {command:?}");
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
    #[test]
    fn a_running_pod_is_ejected_whatever_the_condition() {
        let conditions = [
            Condition::Intact,
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 7,
                starting: false,
            },
            Condition::Dying {
                ticks_left: 0,
                starting: true,
            },
        ];
        for condition in conditions {
            for ticks_left in [0, 1, POD_TICKS] {
                assert_eq!(
                    PlayerFate::of(condition, false, None, Some(ticks_left)),
                    PlayerFate::Ejected { ticks_left },
                    "{condition:?}"
                );
            }
        }
    }

    #[test]
    fn ejected_is_drawn_in_full_takes_no_command_does_not_drag_and_may_not_be_saved() {
        let row = PlayerFate::Ejected {
            ticks_left: POD_TICKS,
        }
        .row();
        assert_eq!(row.drawn, Drawn::Full);
        assert!(!row.drag);
        assert!(!row.saves);
        for command in PlayerCommand::ALL {
            assert!(!row.input.takes(command), "{command:?}");
        }
    }

    #[test]
    fn only_flying_disabled_and_breaking_up_players_take_the_eject_command() {
        for fate in [PlayerFate::Flying, PlayerFate::Disabled]
            .into_iter()
            .chain(BREAKING_UP)
        {
            assert!(fate.row().input.takes(PlayerCommand::Eject), "{fate:?}");
        }
        for fate in [PlayerFate::Exploded, PlayerFate::Ejected { ticks_left: 9 }]
            .into_iter()
            .chain(AFTER_THE_EXPLOSION)
        {
            assert!(!fate.row().input.takes(PlayerCommand::Eject), "{fate:?}");
        }
    }

    #[test]
    fn only_flying_disabled_and_breaking_up_players_take_the_player_info_key() {
        for fate in [PlayerFate::Flying, PlayerFate::Disabled]
            .into_iter()
            .chain(BREAKING_UP)
        {
            assert!(
                fate.row().input.takes(PlayerCommand::PlayerInfo),
                "{fate:?}"
            );
        }
        for fate in [PlayerFate::Exploded, PlayerFate::Ejected { ticks_left: 9 }]
            .into_iter()
            .chain(AFTER_THE_EXPLOSION)
        {
            assert!(
                !fate.row().input.takes(PlayerCommand::PlayerInfo),
                "{fate:?}"
            );
        }
    }

    #[test]
    fn the_pod_counts_350_ticks() {
        assert_eq!(POD_TICKS, 350);
    }
}
