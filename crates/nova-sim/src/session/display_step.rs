//! The player's step from its death timer to its display, as
//! `_HandlePlayer` ends and `_DoPlayGameWork` displays it
//! ([`Session::player_display_step`]), and what the session does with each
//! ship's display step (see the fight's `display` module).
//!
//! **The order** (`_HandlePlayer` after its move, then `_DoPlayGameWork`
//! @0x43e29), after the player's move and before the collision passes, on
//! both passes:
//!
//! 1. The death timer ticks (@0x6d7e6-0x6d7f8,
//!    [`Condition::count_down`]).
//! 2. The klaxon (@0x6f1e4-0x6f227): [`SimSound::Klaxon`] while the
//!    timer, after its tick, is above 1.0. A timer the display set on the
//!    last pass has ticked, so the klaxon sounds from the pass after the
//!    set; a breakup's unset timer and a stalled one's sound none. On the
//!    explosion pass the timer ticks to 2, the klaxon sounds, and then the
//!    display explodes the ship.
//! 3. The eject check (@0x6f22c, see the `eject` module), reading the
//!    timer after its tick; then the self-destruct's countdown (block Z
//!    @0x6fdb4, see the `self_destruct` module), reading whether the
//!    player was dying at the top of `_HandlePlayer`.
//! 4. The player's display step ([`Combat::display_player`]): its
//!    condition settles from its armour, its breakup's countdown starts,
//!    or its final explosion goes off, with its shockwave.
//! 5. The player laid out in the layer its display read
//!    ([`Session::lay_out_displayed`], `_HandleShipDisplay`'s layer block
//!    @0x2bd24-0x2bdad), just before the collision passes walk it.
//! 6. An explosion holds the game's end, and halts
//!    [`PLAYER_EXPLOSION_HALTS`](crate::sound::PLAYER_EXPLOSION_HALTS)
//!    after the klaxon (@0x2d86d-0x2d8ea).
//!
//! The rest of the final explosion's work (the fates, the take-out) runs
//! in [`Session::fight_tail`], later in the same pass and before the
//! frame is drawn.

use super::Session;
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::combat::{Combat, GamePass, Rules};
use crate::player_fate::{Ending, HOLD_TICKS};
use crate::sound::SimSound;

impl Session {
    /// The player's step from its death timer to its display on `pass` by
    /// `rules`, the player `dying_at_head` of the pass or not, drawing on
    /// `chance` (see the module docs).
    pub(super) fn player_display_step(
        &mut self,
        pass: GamePass,
        rules: Rules,
        dying_at_head: bool,
        chance: &mut dyn Chance,
    ) {
        self.ship.condition.count_down();
        if matches!(
            self.ship.condition,
            Condition::Dying {
                ticks_left: 2..,
                starting: false,
            }
        ) {
            self.out.sounds.push(SimSound::Klaxon);
        }
        self.eject_check(chance);
        self.self_destruct_tick(dying_at_head);
        let was_destroyed = self.ship.condition == Condition::Destroyed;
        self.land_phase(pass, rules, chance, |combat, fighters, arena, _| {
            Combat::display_player(combat, fighters, arena);
        });
        self.lay_out_displayed();
        if !was_destroyed && self.ship.condition == Condition::Destroyed {
            self.settle_player_explosion();
        }
    }

    /// The player's final explosion: it holds the game's end, and halts
    /// [`PLAYER_EXPLOSION_HALTS`](crate::sound::PLAYER_EXPLOSION_HALTS),
    /// after the klaxon its step sounded (@0x2d86d-0x2d8ea).
    fn settle_player_explosion(&mut self) {
        // Set only, never cleared here: double time's second pass keeps
        // an explosion the advance pass saw (see
        // `player_exploded_this_tick`).
        self.ship.fate.player_exploded_this_tick = true;
        self.player_end = Some(Ending::Holding {
            ticks_left: HOLD_TICKS,
        });
        self.ship
            .halt_sounds(&mut self.out, &crate::sound::PLAYER_EXPLOSION_HALTS);
    }
}

#[cfg(test)]
mod tests {
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::combat::hull::Condition;
    use crate::flight::Controls;
    use crate::session::Session;
    use crate::session::tests::{Firing, deadly_in, facing_an_npc};
    use crate::sound::{PLAYER_EXPLOSION_HALTS, SimSound};
    use crate::testkit::FakePilotCatalog;

    /// The player's condition and the session's sounds after each pass of
    /// the game's work under fire from `catalog`'s NPC, from the pass its
    /// armour goes on, for `passes` passes.
    fn from_the_breakup(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        passes: usize,
    ) -> Vec<(Condition, Vec<SimSound>)> {
        let mut seen = Vec::new();
        for _ in 0..300 {
            session.take_sounds();
            let rescued = session.game_work(
                Controls::default(),
                catalog,
                &Firing,
                Rules::default(),
                &mut NeverFires,
            );
            assert!(!rescued);
            let condition = session.player_condition();
            if !seen.is_empty() || !matches!(condition, Condition::Intact | Condition::Disabled) {
                seen.push((condition, session.take_sounds()));
            }
            if seen.len() == passes {
                return seen;
            }
        }
        panic!("the player's armour never went");
    }

    #[test]
    fn the_player_downed_by_a_shot_breaks_up_from_its_next_display_and_explodes_after_its_klaxon() {
        // `DeathDelay` 2, a timer of 6. The collision passes of pass t
        // leave it unset; pass t+1's display (@0x43e29) sets it, with no
        // klaxon; from pass t+2 each tick (@0x6d7e6) leaves it above 1.0
        // and the klaxon sounds (@0x6f1e4); on pass t+5 it ticks to 2,
        // the klaxon sounds, and the display explodes the ship and halts
        // the klaxon and warp sounds after it (@0x2d86d-0x2d8ea).
        let catalog = deadly_in(2);
        let mut session = facing_an_npc(&catalog, 180);
        let seen = from_the_breakup(&mut session, &catalog, 7);
        let dying = |ticks_left, starting| Condition::Dying {
            ticks_left,
            starting,
        };
        let klaxon = || vec![SimSound::Klaxon];
        assert_eq!(
            seen[..5],
            [
                (
                    dying(0, true),
                    vec![SimSound::ShipDowned, SimSound::ShipDowned]
                ),
                (dying(6, true), vec![]),
                (dying(5, false), klaxon()),
                (dying(4, false), klaxon()),
                (dying(3, false), klaxon()),
            ]
        );
        assert_eq!(
            seen[5],
            (
                Condition::Destroyed,
                vec![SimSound::Klaxon, SimSound::Halt(&PLAYER_EXPLOSION_HALTS)]
            )
        );
        assert_eq!(seen[6], (Condition::Destroyed, vec![]), "none after");
    }
}
