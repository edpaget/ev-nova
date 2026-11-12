//! A hull's [`Condition`] and the pure rules it is read by, from
//! `nova_sim_combat::combat::hull`, which re-exports them and keeps the
//! hull's spec and the disable rule.

use nova_sim_base::reserves::Gauge;
use nova_sim_catalog::catalog::ShipId;

/// The escape pod's ship class, `shïp` 895 "Escape Pod": the original's
/// class index 0x2ff (the ID less 128), which `_EnterGameFromMainScreen`
/// tests (@0x1a4c5) to spare a strict pilot who ejected in it. Only the
/// pod branch of the player's eject sets it (@0x6f64a), but data may name
/// it for any NPC, and a hail counts a target of it mute (@0x62145). It
/// never breaks up (`_HandleShipDisplay` @0x2d803), and it and every class
/// after it are never damaged (`nova_sim_combat::combat::hull::immune`).
pub const ESCAPE_POD_SHIP: ShipId = ShipId(895);

/// How a ship is holding up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Condition {
    /// Flying and fighting.
    #[default]
    Intact,
    /// Disabled: it drifts, and cannot fire, land or jump. Further damage
    /// can still destroy it.
    Disabled,
    /// Its armour is gone, and it breaks up, counting down the original's
    /// death timer (field 0x3c) to its final explosion (see
    /// `nova_sim_combat::combat::Combat`'s step 9 and its display step,
    /// `_HandleShipDisplay` @0x2d7ef-0x2dcfc).
    Dying {
        /// The death timer: 0 while it is unset (the original's 0.0 or
        /// below), as a hit or any loss of armour leaves it; its
        /// `DeathDelay`, times `nova_sim_combat::combat::hull::PLAYER_BREAKUP_FACTOR` for the player,
        /// once the ship's display step sets it (@0x2d834-0x2d850); less
        /// a tick for each of the ship's own steps since
        /// ([`Condition::count_down`]). The display step explodes the
        /// ship once it is at 1 to `nova_sim_combat::combat::hull::EXPLOSION_BAND`.
        ticks_left: u32,
        /// Whether the timer is unset, or this pass's display step set it:
        /// the original's "dying, timer not yet set" frame, on which
        /// `_HandleShipDisplay` sets it and does nothing else. The ship's
        /// next death-timer tick clears it, or, on a step that skips the
        /// tick, [`Condition::settle_start`]; under `nova_sim_combat::combat::hull::Breakup::Stalls` the
        /// display sets it again every time it finds the timer run down.
        /// The eject reads it, as `_HandlePlayer` (@0x6f248-0x6f286) runs
        /// before `_HandleShipDisplay` sets the timer, and so does the
        /// view's breakup puffs.
        starting: bool,
    },
    /// Destroyed.
    Destroyed,
}

impl Condition {
    /// Whether it still stands: intact or disabled, neither breaking up
    /// nor destroyed. A homing shot loses a target that no longer stands,
    /// a turreted beam lets go of it, and the AI no longer aims at it.
    #[must_use]
    pub fn standing(self) -> bool {
        matches!(self, Self::Intact | Self::Disabled)
    }

    /// Whether its slot is active: anything but destroyed, breaking up
    /// included (the original's +0xb8). Shots, beams and blasts reach
    /// every active ship (`nova_sim_combat::combat::sides`): `_ShotCanHitShip`,
    /// `_HandleBeams`' filter and the blast pass test the slot alone, never
    /// `_IsDying`.
    #[must_use]
    pub fn active(self) -> bool {
        self != Self::Destroyed
    }

    /// The death timer's tick at the ship's own step, the player's in
    /// `_HandlePlayer` (@0x6d7e6-0x6d7f8), an NPC's in `_HandleShip` after
    /// its pick (@0x35083-0x350a3): a timer above 0 loses a tick, and the
    /// display step's set (`starting`) is past. Anything else is left as
    /// it is.
    ///
    /// Left out: `_HandleShip` skips the NPC's tick while `_gameFrozen` is
    /// set (@0x35091-0x35099), and the engine has no frozen game.
    pub fn count_down(&mut self) {
        if let Self::Dying { ticks_left, .. } = *self
            && ticks_left > 0
        {
            *self = Self::Dying {
                ticks_left: ticks_left - 1,
                starting: false,
            };
        }
    }

    /// The end of a set timer's starting frame on a step that skips the
    /// death timer's tick, the player's pod frames (`_HandlePlayer`
    /// returns @0x68ddd before its tick @0x6d7e6): a timer above 0 keeps
    /// its ticks, and the display step's set (`starting`) is past, as
    /// `_HandleShipDisplay` sets the timer only while it is at 0.0 or
    /// below (@0x2d80d-0x2d855) and otherwise reaches its puff branch
    /// (@0x2dd01). Anything else is left as it is.
    pub fn settle_start(&mut self) {
        if let Self::Dying {
            ticks_left,
            starting: true,
        } = *self
            && ticks_left > 0
        {
            *self = Self::Dying {
                ticks_left,
                starting: false,
            };
        }
    }

    /// Whether it is disabled as the original's `_IsDisabled` reads it of
    /// a ship that defends no stellar: disabled or breaking up. A pirate
    /// plunders only such a ship, and fires disable-only at one that is
    /// not. A ship that may defend one reads
    /// [`stored_disabled`](Self::stored_disabled) instead (an NPC through
    /// `nova_sim_ai::traffic::npc::Npc::is_disabled`), which a defence
    /// ship breaking up is not.
    #[must_use]
    pub fn disabled(self) -> bool {
        matches!(self, Self::Disabled | Self::Dying { .. })
    }

    /// The breakup a standing ship with `armor` starts, if it does: `Dying`
    /// with its timer unset, which the ship's next display step sets (see
    /// the `display` module), when it is intact or disabled and its armour
    /// is gone ([`armour_gone`]). A ship breaking up or destroyed is left
    /// as it is: `None`.
    ///
    /// It is `_IsDying`'s (@0x253d) armour test, the timer above 0 or the
    /// armour at 0.0 or below, read off a ship whose condition has not yet
    /// followed its armour, with the engine's [`holds_armour`] guard: a
    /// ship type that holds none has none to lose. The fight's settling
    /// of a condition (`Combat::settle_condition`) and the player's eject
    /// check (`nova_sim::Session::eject_check`) both read it, so the rule
    /// has one home.
    ///
    /// It leaves out, on purpose:
    ///
    /// - The escape pod's exemption (`nova_sim_combat::combat::hull::never_breaks_up`). It is
    ///   `_HandleShipDisplay`'s, which skips its dying block for class
    ///   0x2ff (@0x2d803), not `_IsDying`'s, which never reads the class.
    ///   The fight's settling checks it before asking this; the eject
    ///   check reads `_IsDying` as `_HandlePlayer` caches it (@0x6f22c),
    ///   with no class test, and so does not.
    /// - `nova_sim_combat::combat::Fighter::held_disabled`. It
    ///   holds a standing ship disabled only while its armour is left: a
    ///   held ship whose armour is gone breaks up all the same, so it
    ///   never changes this answer.
    #[must_use]
    pub fn breaking_up(self, armor: Gauge) -> Option<Self> {
        (self.standing() && armour_gone(armor)).then_some(Self::Dying {
            ticks_left: 0,
            starting: true,
        })
    }

    /// `_IsDying` (@0x253d): whether a ship in this condition with `armor`
    /// is dying, its death timer running or, though nothing has settled
    /// its condition yet, its armour gone ([`breaking_up`](Self::breaking_up)).
    /// A ship type that holds no armour never reads dying by its armour,
    /// and a destroyed ship never reads dying. The NPC escape-ship check
    /// in the session's ship loop (`nova_sim::Session::game_work`) reads
    /// it.
    #[must_use]
    pub fn is_dying(self, armor: Gauge) -> bool {
        matches!(self, Self::Dying { .. }) || self.breaking_up(armor).is_some()
    }

    /// Whether a ship in this condition with `armor` is dying
    /// ([`is_dying`](Self::is_dying)) or destroyed: `_EscortAI`'s lost
    /// parent (@0x83917-0x8397b) as the engine reads it of the player,
    /// which frees every ship the player leads while it breaks up or once
    /// it is destroyed. The session's release of the player's escorts
    /// (`nova_sim::Session::free_dying_players_escorts`) and the escort AI's
    /// mission escort (`nova_sim_ai::ai::escort`) both read it.
    #[must_use]
    pub fn dying_or_destroyed(self, armor: Gauge) -> bool {
        self == Self::Destroyed || self.is_dying(armor)
    }
}

/// `_IsDisabled` (@0x2ce6-0x2e0d) of a ship in `condition`: `held`
/// disabled at any armour (a Derelict government's or a mission's hold,
/// branches 1 and 2, @0x2cf1-0x2d69), or else, for a ship whose armour
/// it reads ([`reads_armour`]), anything but intact; a stellar's defence
/// ship is never disabled by its armour, 0 or below included, so it is
/// not disabled while it breaks up (branch 3, @0x2d6f-0x2d77).
///
/// It and `nova_sim_combat::combat::hull::under_armour_line` are the one home of branch 3. Every
/// reader of `_IsDisabled` reaches it through one of them: the fight's
/// (`nova_sim_combat::combat::Fighter`'s readings, its settling of a condition
/// and the spared hits' floor), the stored-condition readers
/// ([`Condition::stored_disabled`]: the draw layer, the shockwave, and
/// the AI's `nova_sim_ai::traffic::npc::Npc::is_disabled`).
#[must_use]
pub fn is_disabled(condition: Condition, held: bool, defends: bool) -> bool {
    held || (reads_armour(defends) && condition != Condition::Intact)
}

/// Whether `_IsDisabled` reads the armour of a ship that `defends` a
/// stellar or not: branch 3 (@0x2d6f-0x2d77) returns "not disabled" for an
/// NPC whose +0x8c is not -1 before the armour line and the armour-0
/// test, so only for one that defends nothing.
#[must_use]
pub fn reads_armour(defends: bool) -> bool {
    !defends
}

impl Condition {
    /// `_IsDisabled` (@0x2ce6-0x2e0d) as a ship's stored condition answers
    /// it, for a ship that `defends` a stellar or not: [`is_disabled`],
    /// the hold read off the condition. For a ship that defends nothing it
    /// is anything but intact. The fight stores a defence ship disabled
    /// only while it is held (`Combat::settle_condition` never disables
    /// one by its armour, `nova_sim_combat::combat::hull::under_armour_line`), so a stored `Disabled`
    /// is its hold, and a defence ship breaking up or destroyed is not
    /// disabled.
    ///
    /// It is the reading of a ship whose hold is not to hand: the draw
    /// layer's (`ship_layer::disabled`), the shockwave's before its hit
    /// and the AI's (`nova_sim_ai::traffic::npc::Npc::is_disabled`). A
    /// breaking-up ship held by a Derelict government that defends a
    /// stellar reads not disabled here, where the original holds it
    /// disabled (branch 1); no stock stellar of a Derelict government has
    /// defence ships.
    #[must_use]
    pub fn stored_disabled(self, defends: bool) -> bool {
        is_disabled(self, self == Self::Disabled, defends)
    }
}

/// Whether `armor` is gone: at or below none, on a ship that holds any.
#[must_use]
pub fn armour_gone(armor: Gauge) -> bool {
    armor.now <= 0.0 && holds_armour(armor)
}

/// Whether a ship with `armor` holds any. A ship type with no armour at
/// all (stock `shïp` 895, plug-in or test data) has none to lose, so it
/// never breaks up, and the `nova_sim_combat::combat::hull::DisableRule` is never asked of it, so it is
/// never disabled either.
#[must_use]
pub fn holds_armour(armor: Gauge) -> bool {
    armor.max > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn armor(now: f32, max: f32) -> Gauge {
        Gauge { now, max }
    }

    const UNSET: Condition = Condition::Dying {
        ticks_left: 0,
        starting: true,
    };

    const BREAKING_UP: Condition = Condition::Dying {
        ticks_left: 3,
        starting: false,
    };

    const CONDITIONS: [Condition; 4] = [
        Condition::Intact,
        Condition::Disabled,
        BREAKING_UP,
        Condition::Destroyed,
    ];

    #[test]
    fn a_defence_ship_is_disabled_only_by_its_hold_in_any_condition() {
        for condition in CONDITIONS {
            assert!(
                !is_disabled(condition, false, true),
                "{condition:?}: branch 3 (@0x2d6f) before the armour"
            );
            assert!(
                is_disabled(condition, true, true),
                "{condition:?}: branches 1 and 2 come first"
            );
            assert_eq!(
                is_disabled(condition, false, false),
                condition != Condition::Intact,
                "{condition:?}: any other ship by its armour"
            );
        }
    }

    #[test]
    fn a_stored_disabled_is_a_defence_ships_hold_and_its_breakup_is_not_disabled() {
        for condition in CONDITIONS {
            assert_eq!(
                condition.stored_disabled(false),
                condition != Condition::Intact,
                "{condition:?}, defending nothing"
            );
            assert_eq!(
                condition.stored_disabled(true),
                condition == Condition::Disabled,
                "{condition:?}, a defence ship"
            );
        }
    }

    #[test]
    fn a_standing_ship_whose_armour_is_gone_starts_breaking_up_its_timer_unset() {
        for condition in [Condition::Intact, Condition::Disabled] {
            for now in [0.0, -1.0] {
                assert_eq!(
                    condition.breaking_up(armor(now, 100.0)),
                    Some(UNSET),
                    "{condition:?} at {now}"
                );
            }
        }
    }

    #[test]
    fn a_standing_ship_with_armour_left_or_none_to_lose_is_not_breaking_up() {
        for condition in [Condition::Intact, Condition::Disabled] {
            assert_eq!(condition.breaking_up(armor(0.5, 100.0)), None);
            assert_eq!(condition.breaking_up(armor(0.0, 0.0)), None);
        }
    }

    #[test]
    fn a_ship_breaking_up_or_destroyed_is_left_as_it_is() {
        let dying = Condition::Dying {
            ticks_left: 7,
            starting: false,
        };
        for condition in [dying, Condition::Destroyed] {
            assert_eq!(condition.breaking_up(armor(-1.0, 100.0)), None);
        }
    }

    #[test]
    fn is_dying_reads_a_breakup_or_a_standing_ship_whose_armour_is_gone() {
        // `_IsDying` (@0x253d): its timer runs, or its armour is at none or
        // below on a ship that holds any, though nothing settled it yet.
        let dying = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        assert!(dying.is_dying(armor(5.0, 100.0)), "breaking up");
        assert!(UNSET.is_dying(armor(5.0, 100.0)), "its timer unset");
        for condition in [Condition::Intact, Condition::Disabled] {
            assert!(condition.is_dying(armor(0.0, 100.0)), "{condition:?}");
            assert!(condition.is_dying(armor(-1.0, 100.0)), "{condition:?}");
            assert!(!condition.is_dying(armor(0.5, 100.0)), "{condition:?}");
            assert!(
                !condition.is_dying(armor(0.0, 0.0)),
                "{condition:?} holds no armour to lose"
            );
        }
        assert!(!Condition::Destroyed.is_dying(armor(-1.0, 100.0)), "gone");
    }

    #[test]
    fn dying_or_destroyed_reads_is_dying_or_a_destroyed_ship() {
        // The lost parent's test, which a destroyed ship passes too.
        let left = armor(5.0, 100.0);
        assert!(Condition::Destroyed.dying_or_destroyed(left), "gone");
        assert!(UNSET.dying_or_destroyed(left), "breaking up");
        for condition in [Condition::Intact, Condition::Disabled] {
            assert!(
                condition.dying_or_destroyed(armor(0.0, 100.0)),
                "{condition:?}"
            );
            assert!(!condition.dying_or_destroyed(left), "{condition:?}");
            assert!(
                !condition.dying_or_destroyed(armor(0.0, 0.0)),
                "{condition:?} holds no armour to lose"
            );
        }
    }

    #[test]
    fn armour_is_gone_at_or_below_none_on_a_ship_that_holds_any() {
        assert!(armour_gone(armor(0.0, 1.0)));
        assert!(armour_gone(armor(-1.0, 1.0)));
        assert!(!armour_gone(armor(0.1, 1.0)));
        assert!(!armour_gone(armor(0.0, 0.0)));
        assert!(holds_armour(armor(0.0, 0.1)));
        assert!(!holds_armour(armor(5.0, 0.0)));
    }
}
