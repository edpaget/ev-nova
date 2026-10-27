//! Each ship's display step, the part of `_HandleShipDisplay`
//! (@0x2b514-0x2df84) the fight models: the condition settled from the
//! armour, the breakup's countdown started, the final explosion and its
//! shockwave, and the `_IsDisabled` its layer block reads.
//!
//! **Where it runs.** `_DoPlayGameWork` displays the player right after
//! `_HandlePlayer` (@0x43e29), before the collision passes, on both
//! passes ([`Combat::display_player`]); and each NPC right after its own
//! `_HandleShip`, slot by slot (@0x444ad, the ship loop @0x4445c-0x444c4),
//! before `_HandleBeams` ([`Combat::handle_npc`], which the session runs
//! right after that NPC's flight, `nova_sim::Session::ship_loop`). So a
//! breakup begun
//! after a ship's display (a hit on the player, a beam, a shockwave on a
//! ship shown before its wreck) starts its countdown at that ship's next
//! display.
//!
//! **The dying block** (@0x2d7ef-0x2dcfc) runs while `_IsDying` (@0x253d)
//! holds: the death timer above 0, or the armour at none or below. The
//! escape pod (class 0x2ff, [`never_breaks_up`]) skips it.
//!
//! - A timer at 0 or below is set to the class's `DeathDelay`
//!   (@0x2d834), three times it for the player ([`PLAYER_BREAKUP_FACTOR`],
//!   @0x2d842-0x2d850), and nothing else happens. A `DeathDelay` that
//!   leaves it at 0, or a timer an NPC of `DeathDelay` 1 ran down to 0,
//!   goes as the fight's [`Breakup`] says: [`Breakup::Explodes`] destroys
//!   the ship; [`Breakup::Stalls`] sets it again, as the original's reset
//!   does.
//! - At 1 to [`EXPLOSION_BAND`] the ship makes its final explosion
//!   (@0x2d86d-0x2dcfc): [`CombatEvent::Destroyed`](super::CombatEvent::Destroyed) and, for a heavy
//!   ship, its shockwave at once (@0x2d8ef-0x2da9c, `Combat::shock`), so
//!   it strikes a later slot before that slot's own step.
//! - Above it, the breakup's puffs, which the view rolls (`Effects::step`
//!   in nova-view), reading `starting` to tell a timer set this pass.
//!
//! The timer's tick is not here: each ship's own step runs it before its
//! display ([`Condition::count_down`]).
//!
//! **The rest of the function** is listed branch by branch in the phase
//! body of rdm:phase/weapons-fidelity/phase-39-left-out-of-phase-28. What
//! it left out is built by
//! rdm:phase/weapons-fidelity/phase-46-left-out-of-phase-39: the
//! inactive-slot branch's let-go of the player's target (the session's
//! `lost_target` module) and the ship loop's flight at each slot
//! (`nova_sim::Session::ship_loop`).

use super::hull::{Breakup, Condition, DisableRule, EXPLOSION_BAND, PLAYER_BREAKUP_FACTOR};
use super::{Combat, Fighter, ShipRef, never_breaks_up};
use crate::combat::Arena;
use crate::ship_layer::{self, Slot};

/// One ship's display step, as the session lays the ship out from it: the
/// ship, its slot, and whether `_IsDisabled` held as the step read it
/// before its layer block (@0x2b61f, kept at -0x152), after its condition
/// settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Displayed {
    /// Which ship.
    pub ship: ShipRef,
    /// Its ship slot.
    pub slot: Slot,
    /// Whether it was disabled or breaking up ([`ship_layer::disabled`]).
    pub disabled: bool,
}

impl Combat {
    /// The player's display step (`_DoPlayGameWork` @0x43e29), after
    /// `_HandlePlayer`'s death-timer tick (the session's,
    /// `nova_sim::Session::player_display_step`) and before the collision
    /// passes, by
    /// `arena`'s rules: its condition settled from its armour, its dying
    /// block, its wreck's shockwave at once, which strikes the NPCs not
    /// yet displayed this pass and spares the player, and its record kept
    /// ([`Combat::take_displays`]).
    pub fn display_player(&mut self, fighters: &mut [Fighter], arena: Arena<'_>) {
        self.field = arena.field;
        self.note_slots(fighters);
        let Some(index) = fighters
            .iter()
            .position(|fighter| fighter.ship == ShipRef::Player)
        else {
            return;
        };
        self.display(fighters, index, arena.rules.disable, &[]);
    }

    /// The display steps run since they were last taken, in the order
    /// they ran; taking them empties the list.
    pub fn take_displays(&mut self) -> Vec<Displayed> {
        std::mem::take(&mut self.displays)
    }

    /// The display step of the ship at `index` among `fighters`, by
    /// `rule`, the ships in `shown` displayed before it this pass: a
    /// destroyed ship's slot is inactive, and nothing is done (the
    /// original's inactive-slot branch, @0x2b522-0x2b60a, hides its
    /// sprites). Otherwise its condition settles from its armour, as
    /// `_IsDying` reads the armour live (@0x253d): a breakup any loss of
    /// armour began, a hit's or not (a sabotage kill, a self-destruct), is
    /// caught here. Its record is kept, then its dying block runs (see
    /// the module docs), and a final explosion's shockwave goes off at
    /// once.
    pub(super) fn display(
        &mut self,
        fighters: &mut [Fighter],
        index: usize,
        rule: &dyn DisableRule,
        shown: &[ShipRef],
    ) {
        let fighter = &mut fighters[index];
        if *fighter.condition == Condition::Destroyed {
            return;
        }
        self.settle_condition(fighter, rule);
        self.displays.push(Displayed {
            ship: fighter.ship,
            slot: fighter.slot,
            disabled: ship_layer::disabled(*fighter.condition),
        });
        if self.dying_block(fighter) {
            let wreck = fighter.ship;
            self.shock(fighters, wreck, shown, rule);
        }
    }

    /// `_HandleShipDisplay`'s dying block for `fighter` (@0x2d7ef-0x2dcfc,
    /// see the module docs). Whether it destroyed the ship.
    fn dying_block(&mut self, fighter: &mut Fighter) -> bool {
        let Condition::Dying {
            ticks_left,
            starting,
        } = *fighter.condition
        else {
            return false;
        };
        if never_breaks_up(fighter.ship_type) {
            return false;
        }
        let delay = if fighter.ship == ShipRef::Player {
            fighter.hull.death_delay * PLAYER_BREAKUP_FACTOR
        } else {
            fighter.hull.death_delay
        };
        let set = Condition::Dying {
            ticks_left: delay,
            starting: true,
        };
        match ticks_left {
            0 if starting && delay > 0 => {
                *fighter.condition = set;
                false
            }
            0 if self.breakup == Breakup::Stalls => {
                *fighter.condition = set;
                false
            }
            0..=EXPLOSION_BAND => {
                self.destroy(fighter);
                true
            }
            _ => false,
        }
    }
}
