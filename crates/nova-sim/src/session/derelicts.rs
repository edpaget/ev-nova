//! A Derelict government's ships, held disabled at any armour, as
//! `_IsDisabled` reads them (rdm weapons-fidelity phase 42).
//!
//! # `_IsDisabled` (@0x2ce6-0x2e0d), the whole function
//!
//! 1. **Derelict** (@0x2cf1-0x2d14): a ship whose government +0x98 is not
//!    -1 and whose government's byte +0x21 has bit 0x08 is disabled,
//!    whatever its armour. The table is 0x88-stride at `*0x3b5164`; byte
//!    +0x21 bit 0x08 is `Flags` 0x0800, `govt::DERELICT`. The player's
//!    +0x98 is -1, so it never applies to the player. Built here
//!    ([`Session::held_disabled`]).
//! 2. **Mission hold** (@0x2d1a-0x2d69, NPCs only, +0x86 above 0): a
//!    rescue ship not yet boarded ([`Session::mission_ship_held`]).
//! 3. **Defence ship** (@0x2d6f-0x2d77): an NPC guarding a stellar (+0x8c
//!    not -1) is never disabled by armour. Left out: the engine has no
//!    defence ships; task `planet-domination` records the branch.
//! 4. **Armour line** (@0x2d79-0x2e00): armour × 100 below 33 % (a tough
//!    ship's other share) of `_ShipArmorCapacity`, the session's
//!    [`DisableRule`](crate::DisableRule).
//!
//! No sound. The same byte test is read by `_ShipIsDerelict` @0x824f,
//! `_SlapWithPenalty` @0x9bf2, the spawns (`_RandomShipSpawn` @0x3c75f,
//! `_GenericRandomShipSpawn` @0x3cc46, `_SpawnPerson` @0x40fc9 and
//! @0x4109a, `_SetupShipsInSystem` @0x4323e and @0x43429) and
//! `_HandlePlayer` @0x6abe9.
//!
//! **Stock reach.** Governments 159 Wraith, 160 Derelicts, 177
//! Association of Free Traders and 180 Derelicts have `Flags` 0x0800:
//! mïsn 155's Wraith (`düde` 186), 709's Association ships (227), 729's
//! Leviathan (230) and 830's Auroran Cruiser (243), the Tutorial Derelict
//! (238), and the Drifting Derelict persons. A `düde` ship comes in at
//! full armour, as no spawn path cuts it but `_SpawnPerson`'s and the
//! rescue setup's (`nova_sim_ai`'s `derelict_armor`), so it is this hold
//! alone that leaves it disabled from the start.
//!
//! Every reader of `_IsDisabled` sees it: the fight
//! ([`Fighter::held_disabled`](crate::combat::Fighter::held_disabled):
//! the turret miss, regeneration, the can't-disable floor), self-repair,
//! and the ship's stored [`Condition`], set to disabled as it comes in
//! ([`Session::hold_derelicts`]), which the AI, the IFF colour, the draw
//! layer and the target panel read.

use super::Session;
use crate::combat::hull::Condition;
use crate::traffic::npc::Npc;

impl Session {
    /// Whether `npc` is held disabled whatever its armour, as
    /// `_IsDisabled` reads it (see the module docs): a ship of a Derelict
    /// government (@0x2cf1-0x2d14), or a rescue mission's ship not yet
    /// boarded ([`Session::mission_ship_held`]).
    pub(crate) fn held_disabled(&self, npc: &Npc) -> bool {
        self.govts.derelict(npc.govt) || self.mission_ship_held(npc)
    }

    /// Stores each intact NPC of a Derelict government disabled, with no
    /// fate and no event: nothing ever sees it go from not disabled to
    /// disabled (`_DamageShip`'s -0x17a is set from the start).
    ///
    /// The session runs it as a system fills, after the AI's arrivals
    /// and as a pass's fight ends; a ship that arrives in the AI's step
    /// after the ships before it have thought is held before the frame
    /// is drawn, a lag no player sees.
    pub(super) fn hold_derelicts(&mut self) {
        let govts = &self.govts;
        for npc in self.traffic.npcs_mut() {
            if npc.condition == Condition::Intact && govts.derelict(npc.govt) {
                npc.condition = Condition::Disabled;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::catalog::GovtRecord;
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::combat::{CombatEvent, Rules, ShipRef};
    use crate::govt::DERELICT;
    use crate::session::Session;
    use crate::session::tests::{armed, facing_an_npc};
    use crate::ship_layer::ShipLayer;
    use crate::story::{Fate, StoryEvent};
    use crate::target_panel::TargetStatus;
    use crate::testkit::{Draws, FakePilotCatalog};
    use crate::traffic::npc::NpcId;
    use nova_sim_catalog::fixture::govt;

    /// [`armed`], its one NPC's government (140) of `flags`.
    fn governed(flags: u16) -> FakePilotCatalog {
        FakePilotCatalog {
            govts: vec![GovtRecord { flags, ..govt(140) }],
            ..armed()
        }
    }

    /// [`facing_an_npc`] with the NPC's government Derelict.
    fn derelict() -> (FakePilotCatalog, Session) {
        let catalog = governed(DERELICT);
        let session = facing_an_npc(&catalog, 180);
        (catalog, session)
    }

    #[test]
    fn a_derelict_governments_ship_comes_in_disabled_at_full_armour() {
        let (_, session) = derelict();
        let npc = &session.npcs()[0];
        assert_eq!(npc.condition, Condition::Disabled);
        assert_eq!(npc.reserves.armor.now, npc.reserves.armor.max);
        let fates: Vec<StoryEvent> = session
            .story_raised()
            .into_iter()
            .filter(|event| matches!(event, StoryEvent::ShipFate { .. }))
            .filter(|event| {
                !matches!(event, StoryEvent::ShipFate { fate, .. } if *fate == Fate::Arrived)
            })
            .collect();
        assert_eq!(fates, [], "no fate");
    }

    #[test]
    fn a_ship_of_another_government_comes_in_intact() {
        let catalog = governed(0);
        let session = facing_an_npc(&catalog, 180);
        assert_eq!(session.npcs()[0].condition, Condition::Intact);
        assert!(!session.held_disabled(&session.npcs()[0]));
    }

    #[test]
    fn a_ship_of_no_government_is_not_derelict() {
        let (_, mut session) = derelict();
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.govt = None;
        npc.condition = Condition::Intact;
        assert!(!session.held_disabled(&session.npcs()[0]));
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.npcs()[0].condition, Condition::Intact);
    }

    #[test]
    fn a_derelict_ship_is_held_disabled_in_the_fight() {
        let (_, mut session) = derelict();
        session.traffic.npcs_mut()[0].condition = Condition::Intact;
        assert!(session.held_disabled(&session.npcs()[0]));
        let (_, terms) = session.fight_terms();
        assert!(terms[0].held_disabled);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.npcs()[0].condition, Condition::Disabled);
        let downed = session
            .take_combat_events()
            .into_iter()
            .any(|event| matches!(event, CombatEvent::Disabled { .. }));
        assert!(!downed, "never newly disabled");
    }

    #[test]
    fn a_derelict_ship_regenerates_nothing() {
        let (_, mut session) = derelict();
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.stats.shield_regen = 1.0;
        npc.stats.armor_regen = 1.0;
        npc.reserves.shield.now = 1.0;
        npc.reserves.armor.now = npc.reserves.armor.max - 5.0;
        let before = npc.reserves;
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.npcs()[0].reserves, before);
    }

    #[test]
    fn a_derelict_ship_repaired_by_its_own_gear_stays_disabled() {
        let (_, mut session) = derelict();
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.self_repair = true;
        npc.reserves.armor.now = 1.0;
        let mut chance = Draws::of(&[0]);
        session.tick_combat(Rules::default(), &mut chance);
        let npc = &session.npcs()[0];
        assert!(npc.reserves.armor.now > 1.0, "repaired");
        assert_eq!(npc.condition, Condition::Disabled);
    }

    #[test]
    fn a_derelict_ship_is_drawn_in_the_disabled_layer() {
        let (_, mut session) = derelict();
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert!(
            session
                .draw_layers()
                .contains(&(ShipRef::Npc(NpcId(0)), ShipLayer::Disabled)),
            "{:?}",
            session.draw_layers()
        );
    }

    #[test]
    fn a_derelict_at_full_armour_shows_disabled_on_the_target_panel() {
        let (_, session) = derelict();
        let npc = &session.npcs()[0];
        assert_eq!(session.target_status(npc), TargetStatus::Disabled);
    }
}
