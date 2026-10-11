//! A mission's ship goals: what its `ShipGoal` asks of the player
//! ([`ShipGoal`]), the check that says whether it is met or failed
//! ([`check_goals`], `_MissionObjectivesCheck` @0x9e79e), the moment a
//! goal is met ([`goals_met_moment`]), and the losses that fail a mission
//! at once ([`fails_on_destroyed`], [`fails_on_disabled`]). They are pure
//! rules over the counts the slot keeps of its ships
//! ([`SpecialShips`](super::SpecialShips)); when they are checked, and
//! what the moment shows and runs, is the session's `mission_ships`
//! module's.

use super::active::AUTO_ABORT;
use super::screens::HIDDEN;

/// A mission's `ShipGoal`, read: what the player must do to its special
/// ships.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipGoal {
    /// -1: none.
    None,
    /// 0: destroy them all.
    Destroy,
    /// 1: disable them all, destroying none.
    Disable,
    /// 2: board them all.
    Board,
    /// 3: escort them: met while one is with the player, failed once one
    /// is destroyed or disabled.
    Escort,
    /// 4: observe one.
    Observe,
    /// 5: rescue them all, by boarding them.
    Rescue,
    /// 6: chase them off (rdm:phase/mission-ships/phase-5-chase-off-goal).
    ChaseOff,
    /// Any other code: nothing to do, and nothing checked.
    Other,
}

impl ShipGoal {
    /// The `ShipGoal` `code` names.
    #[must_use]
    pub const fn from_code(code: i16) -> Self {
        match code {
            -1 => Self::None,
            0 => Self::Destroy,
            1 => Self::Disable,
            2 => Self::Board,
            3 => Self::Escort,
            4 => Self::Observe,
            5 => Self::Rescue,
            6 => Self::ChaseOff,
            _ => Self::Other,
        }
    }
}

/// What one check of a mission's ship goals reads: its `mïsn`'s
/// `ShipGoal`, `ShipCount` and `Flags`, the counts its slot keeps, where
/// its ships are, and whether its goals were met as the check began.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // each is one fact the check reads
pub struct GoalFacts {
    /// Its `ShipGoal`.
    pub goal: ShipGoal,
    /// Its `ShipCount` (+0x32).
    pub count: i16,
    /// Its `Flags`.
    pub flags: u16,
    /// Its ships left (+0x6).
    pub left: i16,
    /// Its ships arrived (+0x2e).
    pub arrived: i16,
    /// Its ships destroyed (+0x28).
    pub destroyed: i16,
    /// Its ships boarded (+0x2a).
    pub boarded: i16,
    /// Its ships disabled (+0x2c).
    pub disabled: i16,
    /// Whether its ships are in the player's system, or follow the player
    /// (@0x9e9bb-0x9e9f1).
    pub here: bool,
    /// Whether one of its ships is in the player's system now
    /// (@0x9e931-0x9e962).
    pub escorted: bool,
    /// Whether one of its ships was just seen by the player
    /// (@0x9ea1a-0x9eb15).
    pub sighted: bool,
    /// Whether its goals were met as the check began (+2, @0x9e821).
    pub met: bool,
}

/// What one check of a mission's ship goals finds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GoalCheck {
    /// Whether its goals are met now (+2).
    pub met: bool,
    /// Whether the check failed it (+3): a disable goal with one of its
    /// ships destroyed, or an escort goal with one destroyed or disabled.
    pub failed: bool,
}

/// The check of a mission's ship goals on `facts`, as
/// `_MissionObjectivesCheck` (@0x9e79e) runs it:
///
/// - No goal (`ShipGoal` -1) is met, but with ships (`ShipCount` above
///   0) and `Flags` 0x0001, which
///   rdm:phase/mission-ships/phase-10-auto-abort-at-spawn reads and this
///   leaves as it was (@0x9e835-0x9e840, @0x9ed8e-0x9ed98).
/// - Any other goal with no ships is met (@0x9e849).
/// - While ships are still to come (`ShipCount` above those arrived)
///   and any are left, the goals are unmet (@0x9e852-0x9e85d →
///   @0x9eb7f).
/// - Destroy is met once all are destroyed (@0x9e880-0x9e88b).
/// - Disable fails once any is destroyed, and is otherwise met once all
///   are disabled (@0x9e89f-0x9e8c7).
/// - Board and rescue are met once all are boarded (@0x9e8cc-0x9e8fe).
/// - Escort fails once any is destroyed or disabled, and is otherwise
///   met while one of its ships is with the player and unmet while none
///   is (@0x9e90d-0x9e980).
/// - Observe, only while unmet, is met once one of its ships is seen
///   with its ships here and all of them arrived (@0x9e98f-0x9eb38).
/// - Chase off (phase 5) and any other goal leave it as it was.
#[must_use]
pub fn check_goals(facts: &GoalFacts) -> GoalCheck {
    let unchanged = GoalCheck {
        met: facts.met,
        failed: false,
    };
    let met = GoalCheck {
        met: true,
        failed: false,
    };
    let failed = GoalCheck {
        met: facts.met,
        failed: true,
    };
    if facts.goal == ShipGoal::None {
        return if facts.count > 0 && facts.flags & AUTO_ABORT != 0 {
            unchanged
        } else {
            met
        };
    }
    if facts.count <= 0 {
        return met;
    }
    if facts.count > facts.arrived && facts.left != 0 {
        return GoalCheck::default();
    }
    match facts.goal {
        ShipGoal::Destroy if facts.destroyed >= facts.count => met,
        ShipGoal::Disable if facts.destroyed > 0 => failed,
        ShipGoal::Disable if facts.disabled >= facts.count => met,
        ShipGoal::Board | ShipGoal::Rescue if facts.boarded >= facts.count => met,
        ShipGoal::Escort if facts.destroyed > 0 || facts.disabled > 0 => failed,
        ShipGoal::Escort => GoalCheck {
            met: facts.escorted,
            failed: false,
        },
        ShipGoal::Observe
            if !facts.met && facts.here && facts.arrived >= facts.left && facts.sighted =>
        {
            met
        }
        _ => unchanged,
    }
}

/// Whether a check that found `now` of a mission whose goals were met
/// `before` is the moment its goals are met: met now and not before, the
/// mission not failed (`failed`, before the check or by it) (@0x9ec70-0x9ec8c).
#[must_use]
pub const fn goals_met_moment(before: bool, now: GoalCheck, failed: bool) -> bool {
    now.met && !before && !now.failed && !failed
}

/// What a mission's loss of one of its ships reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LossFacts {
    /// Its `ShipGoal`.
    pub goal: ShipGoal,
    /// Its `Flags`.
    pub flags: u16,
    /// Whether it has failed already (+3).
    pub failed: bool,
    /// How many of its ships were lost that way before this one: those
    /// destroyed for a ship destroyed, those disabled for one disabled.
    pub before: i16,
    /// Whether the ship lost was boarded (+0xb9).
    pub boarded: bool,
}

/// Whether a ship destroyed fails its mission at once, as
/// `_HandleShipDisplay` decides (@0x2dae9-0x2db2b): the first destroyed,
/// the mission not failed and not hidden (`Flags` 0x0400), its goal to
/// disable or escort them, or to board or rescue them with this one not
/// boarded.
#[must_use]
pub const fn fails_on_destroyed(facts: LossFacts) -> bool {
    let goal_fails = match facts.goal {
        ShipGoal::Disable | ShipGoal::Escort => true,
        ShipGoal::Board | ShipGoal::Rescue => !facts.boarded,
        _ => false,
    };
    first_loss(facts) && goal_fails
}

/// Whether a ship newly disabled fails its mission at once, as
/// `_DamageShip` decides (@0x3aeb0-0x3aee2): the first disabled, the
/// mission not failed and not hidden (`Flags` 0x0400), its goal to
/// escort them.
#[must_use]
pub const fn fails_on_disabled(facts: LossFacts) -> bool {
    first_loss(facts) && matches!(facts.goal, ShipGoal::Escort)
}

/// What the comm line says as one of a mission's ships is boarded on the
/// assist path ([`assists`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssistLine {
    /// `STR#` 2002 #126, "Target ship has been boarded." (@0x65665-0x65685).
    Boarded,
    /// With `Flags` 0x0008, the boarded ship's thanks for the fuel: its
    /// `shïp` name, `STR#` 2002 #3 and the player's ship's name
    /// (@0x655ae-0x65663).
    Refuelled,
}

/// The `PickupMode` that picks the cargo up by boarding the special ship
/// (rdm:phase/mission-ships/phase-14-boarded-cargo).
const PICKUP_ON_BOARD: i16 = 2;
/// `Flags`: the auto-abort takes a jump's fuel, and the assist path says
/// so.
const TAKES_FUEL: u16 = 0x0008;

/// Whether boarding one of the special ships of a mission of `goal`,
/// `Flags` `flags` and `PickupMode` `pickup_mode`, with `left` of them
/// left, takes the assist path (`_HandlePlayerBoardAttempt`
/// @0x65575-0x65762) that counts it boarded, and what its comm line
/// says: a board or rescue goal with `Flags` 0x0001 and exactly one ship
/// left. A `PickupMode` of 2 takes the cargo pickup instead
/// (@0x652ee-0x65570, rdm:phase/mission-ships/phase-14-boarded-cargo).
#[must_use]
pub const fn assists(
    goal: ShipGoal,
    flags: u16,
    pickup_mode: i16,
    left: i16,
) -> Option<AssistLine> {
    let assisted = matches!(goal, ShipGoal::Board | ShipGoal::Rescue)
        && flags & AUTO_ABORT != 0
        && left == 1
        && pickup_mode != PICKUP_ON_BOARD;
    if !assisted {
        None
    } else if flags & TAKES_FUEL != 0 {
        Some(AssistLine::Refuelled)
    } else {
        Some(AssistLine::Boarded)
    }
}

/// The first loss of its kind, of a mission not failed and not hidden.
const fn first_loss(facts: LossFacts) -> bool {
    facts.before == 0 && !facts.failed && facts.flags & HIDDEN == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A destroy goal for two ships, both arrived and left, none counted,
    /// unmet, here, with none of its ships in the system or seen.
    fn facts(goal: ShipGoal) -> GoalFacts {
        GoalFacts {
            goal,
            count: 2,
            flags: 0,
            left: 2,
            arrived: 2,
            destroyed: 0,
            boarded: 0,
            disabled: 0,
            here: true,
            escorted: false,
            sighted: false,
            met: false,
        }
    }

    const MET: GoalCheck = GoalCheck {
        met: true,
        failed: false,
    };
    const UNMET: GoalCheck = GoalCheck {
        met: false,
        failed: false,
    };

    fn failed(met: bool) -> GoalCheck {
        GoalCheck { met, failed: true }
    }

    #[test]
    fn a_ship_goal_reads_minus_1_to_6_and_else_other() {
        let goals: Vec<ShipGoal> = (-2..=7).map(ShipGoal::from_code).collect();
        assert_eq!(
            goals,
            [
                ShipGoal::Other,
                ShipGoal::None,
                ShipGoal::Destroy,
                ShipGoal::Disable,
                ShipGoal::Board,
                ShipGoal::Escort,
                ShipGoal::Observe,
                ShipGoal::Rescue,
                ShipGoal::ChaseOff,
                ShipGoal::Other,
            ]
        );
    }

    #[test]
    fn no_ships_meet_any_goal() {
        for goal in [ShipGoal::Destroy, ShipGoal::Escort, ShipGoal::Other] {
            for count in [0, -1] {
                let none = GoalFacts {
                    count,
                    left: 0,
                    arrived: 0,
                    ..facts(goal)
                };
                assert_eq!(check_goals(&none), MET, "{goal:?} {count}");
            }
        }
    }

    #[test]
    fn no_goal_is_met_but_with_ships_and_flag_0x0001_it_is_left_to_phase_10() {
        let none = facts(ShipGoal::None);
        assert_eq!(check_goals(&none), MET);
        for count in [0, -1] {
            let shipless = GoalFacts {
                count,
                flags: AUTO_ABORT,
                ..none
            };
            assert_eq!(check_goals(&shipless), MET, "{count}");
        }
        for met in [false, true] {
            let auto = GoalFacts {
                flags: AUTO_ABORT | 0x0008,
                met,
                ..none
            };
            assert_eq!(check_goals(&auto), GoalCheck { met, failed: false });
        }
        let other_flags = GoalFacts {
            flags: 0x0008,
            ..none
        };
        assert_eq!(check_goals(&other_flags), MET);
    }

    #[test]
    fn ships_still_to_come_leave_the_goals_unmet() {
        let all_destroyed = GoalFacts {
            destroyed: 2,
            met: true,
            ..facts(ShipGoal::Destroy)
        };
        let coming = GoalFacts {
            arrived: 1,
            ..all_destroyed
        };
        assert_eq!(check_goals(&coming), UNMET);
        let none_left = GoalFacts { left: 0, ..coming };
        assert_eq!(check_goals(&none_left), MET, "none left to come");
        assert_eq!(check_goals(&all_destroyed), MET, "all arrived");
        let more_arrived = GoalFacts {
            arrived: 3,
            ..all_destroyed
        };
        assert_eq!(check_goals(&more_arrived), MET);
    }

    #[test]
    fn destroy_is_met_once_all_are_destroyed() {
        let one = GoalFacts {
            destroyed: 1,
            left: 1,
            ..facts(ShipGoal::Destroy)
        };
        assert_eq!(check_goals(&one), UNMET);
        let both = GoalFacts {
            destroyed: 2,
            left: 0,
            ..one
        };
        assert_eq!(check_goals(&both), MET);
        let kept = GoalFacts { met: true, ..one };
        assert_eq!(check_goals(&kept), MET, "a met goal stays met");
    }

    #[test]
    fn disable_is_met_once_all_are_disabled_and_fails_once_one_is_destroyed() {
        let disable = facts(ShipGoal::Disable);
        let one = GoalFacts {
            disabled: 1,
            ..disable
        };
        assert_eq!(check_goals(&one), UNMET);
        let both = GoalFacts {
            disabled: 2,
            ..disable
        };
        assert_eq!(check_goals(&both), MET);
        let destroyed = GoalFacts {
            destroyed: 1,
            ..both
        };
        assert_eq!(check_goals(&destroyed), failed(false));
        let was_met = GoalFacts {
            met: true,
            ..destroyed
        };
        assert_eq!(check_goals(&was_met), failed(true), "met is left as it was");
    }

    #[test]
    fn board_and_rescue_are_met_once_all_are_boarded() {
        for goal in [ShipGoal::Board, ShipGoal::Rescue] {
            let one = GoalFacts {
                boarded: 1,
                ..facts(goal)
            };
            assert_eq!(check_goals(&one), UNMET, "{goal:?}");
            let both = GoalFacts { boarded: 2, ..one };
            assert_eq!(check_goals(&both), MET, "{goal:?}");
            let destroyed = GoalFacts {
                destroyed: 2,
                disabled: 2,
                ..one
            };
            assert_eq!(check_goals(&destroyed), UNMET, "{goal:?}: no failure");
        }
    }

    #[test]
    fn escort_is_met_while_one_is_with_the_player_and_unmet_while_none_is() {
        let escort = facts(ShipGoal::Escort);
        let with = GoalFacts {
            escorted: true,
            ..escort
        };
        assert_eq!(check_goals(&with), MET);
        let gone = GoalFacts {
            met: true,
            ..escort
        };
        assert_eq!(check_goals(&gone), UNMET, "met goes back to unmet");
    }

    #[test]
    fn escort_fails_once_one_is_destroyed_or_disabled() {
        let with = GoalFacts {
            escorted: true,
            met: true,
            ..facts(ShipGoal::Escort)
        };
        let destroyed = GoalFacts {
            destroyed: 1,
            left: 1,
            ..with
        };
        assert_eq!(check_goals(&destroyed), failed(true));
        let disabled = GoalFacts {
            disabled: 1,
            met: false,
            ..with
        };
        assert_eq!(check_goals(&disabled), failed(false));
    }

    #[test]
    fn observe_is_met_once_one_is_seen_with_its_ships_here_and_arrived() {
        let seen = GoalFacts {
            sighted: true,
            ..facts(ShipGoal::Observe)
        };
        assert_eq!(check_goals(&seen), MET);
        let unseen = GoalFacts {
            sighted: false,
            ..seen
        };
        assert_eq!(check_goals(&unseen), UNMET);
        let elsewhere = GoalFacts {
            here: false,
            ..seen
        };
        assert_eq!(check_goals(&elsewhere), UNMET);
        // Fewer arrived than left, all the ship count arrived: only a
        // count above the arrived count holds the gate.
        let fewer = GoalFacts {
            count: 2,
            arrived: 2,
            left: 3,
            ..seen
        };
        assert_eq!(check_goals(&fewer), UNMET);
        let none_left = GoalFacts {
            left: 0,
            arrived: 2,
            ..seen
        };
        assert_eq!(check_goals(&none_left), MET);
    }

    #[test]
    fn observe_is_never_cleared() {
        let met = GoalFacts {
            met: true,
            ..facts(ShipGoal::Observe)
        };
        assert_eq!(check_goals(&met), MET);
        let elsewhere = GoalFacts { here: false, ..met };
        assert_eq!(check_goals(&elsewhere), MET);
    }

    #[test]
    fn chase_off_and_other_goals_are_left_as_they_were() {
        for goal in [ShipGoal::ChaseOff, ShipGoal::Other] {
            for met in [false, true] {
                let all = GoalFacts {
                    destroyed: 2,
                    boarded: 2,
                    disabled: 2,
                    escorted: true,
                    sighted: true,
                    met,
                    ..facts(goal)
                };
                assert_eq!(check_goals(&all), GoalCheck { met, failed: false });
            }
        }
    }

    #[test]
    fn the_moment_is_met_now_not_before_and_not_failed() {
        assert!(goals_met_moment(false, MET, false));
        assert!(!goals_met_moment(true, MET, false), "met before");
        assert!(!goals_met_moment(false, UNMET, false), "not met");
        assert!(!goals_met_moment(false, MET, true), "failed before");
        assert!(
            !goals_met_moment(false, failed(true), false),
            "failed by it"
        );
    }

    /// The first loss of a mission of `goal`, not failed and not hidden,
    /// of a ship not boarded.
    fn loss(goal: ShipGoal) -> LossFacts {
        LossFacts {
            goal,
            flags: 0,
            failed: false,
            before: 0,
            boarded: false,
        }
    }

    #[test]
    fn the_first_destroyed_fails_a_disable_escort_board_or_rescue_goal() {
        for goal in [
            ShipGoal::Disable,
            ShipGoal::Escort,
            ShipGoal::Board,
            ShipGoal::Rescue,
        ] {
            assert!(fails_on_destroyed(loss(goal)), "{goal:?}");
        }
        for goal in [
            ShipGoal::None,
            ShipGoal::Destroy,
            ShipGoal::Observe,
            ShipGoal::ChaseOff,
            ShipGoal::Other,
        ] {
            assert!(!fails_on_destroyed(loss(goal)), "{goal:?}");
        }
    }

    #[test]
    fn a_boarded_ship_destroyed_fails_no_board_or_rescue_goal() {
        for goal in [ShipGoal::Board, ShipGoal::Rescue] {
            let boarded = LossFacts {
                boarded: true,
                ..loss(goal)
            };
            assert!(!fails_on_destroyed(boarded), "{goal:?}");
        }
        for goal in [ShipGoal::Disable, ShipGoal::Escort] {
            let boarded = LossFacts {
                boarded: true,
                ..loss(goal)
            };
            assert!(fails_on_destroyed(boarded), "{goal:?}");
        }
    }

    #[test]
    fn only_the_first_loss_of_a_mission_not_failed_nor_hidden_fails_it() {
        let escort = loss(ShipGoal::Escort);
        for unfailing in [
            LossFacts {
                before: 1,
                ..escort
            },
            LossFacts {
                failed: true,
                ..escort
            },
            LossFacts {
                flags: HIDDEN | AUTO_ABORT,
                ..escort
            },
        ] {
            assert!(!fails_on_destroyed(unfailing), "{unfailing:?}");
            assert!(!fails_on_disabled(unfailing), "{unfailing:?}");
        }
        let other_flags = LossFacts {
            flags: AUTO_ABORT,
            ..escort
        };
        assert!(fails_on_destroyed(other_flags));
        assert!(fails_on_disabled(other_flags));
    }

    #[test]
    fn the_assist_path_takes_a_board_or_rescue_goal_of_flag_0x0001_with_one_ship_left() {
        for goal in [ShipGoal::Board, ShipGoal::Rescue] {
            assert_eq!(
                assists(goal, AUTO_ABORT, -1, 1),
                Some(AssistLine::Boarded),
                "{goal:?}"
            );
            assert_eq!(
                assists(goal, AUTO_ABORT | TAKES_FUEL | HIDDEN, 0, 1),
                Some(AssistLine::Refuelled),
                "{goal:?}"
            );
            assert_eq!(assists(goal, 0, -1, 1), None, "{goal:?}: no 0x0001");
            assert_eq!(assists(goal, TAKES_FUEL, -1, 1), None, "{goal:?}");
            for left in [0, 2] {
                assert_eq!(assists(goal, AUTO_ABORT, -1, left), None, "{goal:?} {left}");
            }
            assert_eq!(
                assists(goal, AUTO_ABORT, PICKUP_ON_BOARD, 1),
                None,
                "{goal:?}: the pickup's"
            );
        }
        for goal in [ShipGoal::Destroy, ShipGoal::Escort, ShipGoal::None] {
            assert_eq!(assists(goal, AUTO_ABORT, -1, 1), None, "{goal:?}");
        }
    }

    #[test]
    fn the_first_disabled_fails_only_an_escort_goal() {
        assert!(fails_on_disabled(loss(ShipGoal::Escort)));
        for goal in [
            ShipGoal::Destroy,
            ShipGoal::Disable,
            ShipGoal::Board,
            ShipGoal::Rescue,
            ShipGoal::Observe,
        ] {
            assert!(!fails_on_disabled(loss(goal)), "{goal:?}");
        }
    }
}
