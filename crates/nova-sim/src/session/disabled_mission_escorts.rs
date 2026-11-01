//! A disabled mission escort, read as a stranger while it is disabled.
//!
//! **The writes.** `_AIDispatch`'s re-lead (@0x8fbcd-0x8fc2b) gives a
//! `ShipBehav` 1 ship of an active mission slot AI type 6 and parent 0 at
//! the head of every dispatch, unless it is `_IsDisabled` (@0x8fc0c). Its
//! disabled reset (@0x8fece-0x8ff13) writes parent -1, leaves +0x88 and
//! jumps past the type think (@0x90032). So from its first dispatch
//! disabled to its first dispatch whole a mission escort's parent is -1 at
//! every reader, and its AI type 6. The engine's reset marks it
//! ([`Npc::mission_dropped`](crate::traffic::npc::Npc::mission_dropped),
//! see `nova_sim_ai::traffic::disabled_reset`), and each traffic tick's
//! re-lead clears the mark of an intact one
//! (`Space::let_go_of_mission_escorts`, see
//! [`dying_player_escorts`](super::dying_player_escorts)). A captured
//! mission ship of another `ShipBehav`, which no re-lead takes, is out of
//! the formation for good, its mark kept. A disabled ship on the jump path
//! or gliding in isn't reset, and stays led.
//!
//! **Its readers**, each through [`Npc::mission_escort`] and what is built
//! on it ([`Npc::escorts_player`], [`Session::player_led`]):
//!
//! | Reader | Address | While it is dropped |
//! |---|---|---|
//! | The escort menu's rows | `_DrawEscortMenuInFrame` @0x56392-0x563ca | its class's row goes ([`Space::escort_rows`]) |
//! | The E key's test | `_HandlePlayer` @0x6ddde-0x6ddf9 | alone, no escort: "You don't have any escorts." and its beep |
//! | The flying count | `_HandlePlayer` @0x6df59-0x6dfa7 | a disabled ship never counts ([`Space::escorts_flying`]) |
//! | Escort commands | `_IssueNewEscortCommand` @0x6613c | skipped, its order kept for when it is led again ([`Session::command_escorts`]) |
//! | The formation | the marking pass @0x44160-0x441c0, `_AICalcFormationPositions` @0x850bc | left out, the rest close up ([`Session::formation_slots`]) |
//! | The hit rules | `_ShipsShareParent` @0x7926-0x79c1, `_AlliedWithPlayer` @0x6985-0x69d0 | a stranger's fleet ([`Npc::fleet_among`], the fight's terms) |
//! | The comm status line | `_CommDialogUpdate` @0x90688-0x906ff | no "Escort" |
//! | The pod's walk | `_HandlePlayer` @0x6f71d | not released, so led again once whole (see the `eject` module) |
//! | Its mission's end | @0x8fc3a-0x8fc4b, `_AbortMission` @0x99ad4-0x99adc | only the mark goes ([`Space::unmark_mission_ship`]) |
//! | `_ShipMaxSpeed` | @0x382e-0x384f | its own top speed ([`Npc::top_speed`]) |
//! | The player's jump | `_MakeEscortsHyperWithParent` | not taken along |
//! | The draw layer | `_HandleShipDisplay` @0x2bd24-0x2bd47 | unchanged: the disabled layer is chosen first |
//! | The IFF radar | `nova_sim_port::iff::ship_iff` | unchanged: disabled is read first |
//! | The player's death | `_EscortAI` @0x83917 | not reached while disabled, so not let go; once whole, led again, then let go |
//!
//! **Its break-up.** `_HandleShip`'s dying branch (@0x350a8-0x35134) runs
//! only while the death timer is set (@0x35083-0x3508b), from the pass
//! after the display that started the breakup, and tests parent 0
//! (@0x350ab). An advance pass's dispatch resets a ship breaking up
//! first, so the branch takes a share of the fleet's cargo only where no
//! reset ran in between: double time's second pass, or a ship on the jump
//! path or gliding in ([`Session::lose_dying_mission_escort`], run at the
//! ship's step of the ship loop).
//!
//! **Ordering.** The original's formation calc leaves the dropped ship out
//! from the second `_HandlePlayer` after the reset (@0x6f1cb); the
//! engine's re-form runs at the next pass's head
//! ([`Session::reform_on_swarm_change`]), a frame earlier. That is one
//! tick of the other escorts' steering target, which no player sees.
//!
//! Nothing here plays a sound or draws a random number.
//!
//! [`Npc::mission_escort`]: crate::traffic::npc::Npc::mission_escort
//! [`Npc::escorts_player`]: crate::traffic::npc::Npc::escorts_player
//! [`Npc::fleet_among`]: crate::traffic::npc::Npc::fleet_among
//! [`Npc::top_speed`]: crate::traffic::npc::Npc::top_speed
//! [`Space::escort_rows`]: super::Space::escort_rows
//! [`Space::escorts_flying`]: super::Space::escorts_flying
//! [`Space::unmark_mission_ship`]: super::Space::unmark_mission_ship
//! [`Session::player_led`]: super::Session::player_led
//! [`Session::command_escorts`]: super::Session::command_escorts
//! [`Session::formation_slots`]: super::Session::formation_slots
//! [`Session::lose_dying_mission_escort`]: super::Session::lose_dying_mission_escort
//! [`Session::reform_on_swarm_change`]: super::Session::reform_on_swarm_change

#[cfg(test)]
mod tests {
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort::{EscortCommand, EscortGroup, EscortOrder};
    use crate::mission::ships::ShipBehav;
    use crate::session::Session;
    use crate::session::mission_escorts::tests::{
        assert_released, escorting, flying, flying_with_escorts, hired_slot, inherent_catalog,
        mission_escorts, row,
    };
    use crate::session::mission_ships::tests::{ships_catalog, tick};
    use crate::traffic::npc::{AiType, Npc, NpcId};

    fn npc(session: &Session, id: NpcId) -> &Npc {
        session.space.npc(id).expect("here")
    }

    /// NPC `id` in `condition`, its armour under the line while it is not
    /// intact and full when it is, so the fight's settling keeps it so.
    fn disable(session: &mut Session, id: NpcId, condition: Condition) {
        let npc = session.space.npc_mut(id).expect("here");
        npc.condition = condition;
        let armor = &mut npc.reserves.armor;
        armor.now = if condition == Condition::Intact {
            armor.max
        } else {
            1.0
        };
    }

    /// Each ship in the player's formation and its place, as the next
    /// re-form gives them.
    fn placed(session: &Session) -> Vec<(NpcId, u8, u8)> {
        session
            .formation_slots()
            .into_iter()
            .map(|(id, place)| (id, place.slot, place.ships))
            .collect()
    }

    #[test]
    fn disabled_it_leaves_the_formation_and_whole_again_it_rejoins() {
        let catalog = ships_catalog(escorting(-1, 0));
        let (mut session, ids) = flying_with_escorts(&catalog);
        let hired = session.space.escorts.fleet[0].expect("placed");
        assert_eq!(hired_slot(&session), (2, 4));
        disable(&mut session, ids[0], Condition::Disabled);
        tick(&mut session, &catalog);
        assert_eq!(
            placed(&session),
            [(hired, 2, 3), (ids[1], 3, 3)],
            "the others close up"
        );
        tick(&mut session, &catalog);
        assert_eq!(hired_slot(&session), (2, 3), "re-formed");
        let dropped = npc(&session, ids[0]);
        assert!(!dropped.mission_escort());
        assert_eq!(
            dropped
                .mission
                .and_then(|mark| mark.formation)
                .map(|place| (place.slot, place.ships)),
            Some((3, 4)),
            "its mark and its slot kept"
        );
        disable(&mut session, ids[0], Condition::Intact);
        tick(&mut session, &catalog);
        assert!(npc(&session, ids[0]).mission_escort(), "led again");
        assert_eq!(hired_slot(&session), (2, 4), "the formation takes it back");
    }

    #[test]
    fn disabled_its_classs_row_leaves_the_menu_and_none_is_flying() {
        // `_DrawEscortMenuInFrame` reads +0x9a 0 (@0x56392-0x563ca), and so
        // does the E key's test (@0x6ddde-0x6ddf9).
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        let class = mission_escorts(&session)[0].class;
        let ids: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        for &id in &ids {
            disable(&mut session, id, Condition::Disabled);
        }
        assert!(row(&session, class).present, "until its dispatch");
        tick(&mut session, &catalog);
        for other in session.space.escort_rows() {
            assert!(!other.present, "{:?}", other.class);
        }
        assert!(!session.space.escorts_flying());
        disable(&mut session, ids[1], Condition::Intact);
        tick(&mut session, &catalog);
        assert!(row(&session, class).present);
        assert!(session.space.escorts_flying());
    }

    #[test]
    fn a_command_skips_it_and_its_order_stands_for_when_it_rejoins() {
        // `_IssueNewEscortCommand` reads +0x9a 0 (@0x6613c).
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        let ids: Vec<NpcId> = mission_escorts(&session).iter().map(|npc| npc.id).collect();
        session.command_escorts(EscortGroup::All, EscortCommand::Hold, &mut NeverFires);
        disable(&mut session, ids[0], Condition::Disabled);
        tick(&mut session, &catalog);
        assert!(
            session
                .command_escorts(EscortGroup::All, EscortCommand::Defend, &mut NeverFires)
                .is_some()
        );
        let order = |session: &Session, id| npc(session, id).mission.and_then(|mark| mark.order);
        assert_eq!(order(&session, ids[0]), Some(EscortOrder::Hold), "skipped");
        assert_eq!(order(&session, ids[1]), Some(EscortOrder::Defend));
        disable(&mut session, ids[0], Condition::Intact);
        tick(&mut session, &catalog);
        let rejoined = npc(&session, ids[0]);
        assert!(rejoined.mission_escort());
        assert_eq!(rejoined.escort_order(), Some(EscortOrder::Hold));
    }

    #[test]
    fn the_fight_reads_it_as_a_stranger_at_its_own_top_speed() {
        // `_ShipsShareParent` and `_AlliedWithPlayer` read +0x9a
        // (@0x7926-0x79c1, @0x6985-0x69d0), and so does `_ShipMaxSpeed`
        // (@0x382e-0x384f).
        let catalog = ships_catalog(escorting(-1, 0));
        let (mut session, ids) = flying_with_escorts(&catalog);
        let terms = |session: &Session| {
            let at = session
                .npcs()
                .iter()
                .position(|npc| npc.id == ids[0])
                .expect("here");
            let terms = &session.fight_terms().1[at];
            (terms.fleet, terms.top_speed)
        };
        let (fleet, led) = terms(&session);
        assert_eq!(fleet, ShipRef::Player);
        disable(&mut session, ids[0], Condition::Disabled);
        tick(&mut session, &catalog);
        let (fleet, own) = terms(&session);
        assert_eq!(fleet, ShipRef::Npc(ids[0]), "a stranger's");
        assert!(own < led, "{own} {led}");
    }

    #[test]
    fn its_mission_ending_while_it_is_disabled_clears_only_the_mark() {
        // With +0x9a -1 the slot-freed branch clears the mark alone
        // (@0x8fc3a-0x8fc4b); +0x88 stays 6 until its think whole, where
        // `_EscortAI`'s lost parent writes its `InherentAI` (@0x83c4a).
        let catalog = inherent_catalog(escorting(-1, 0));
        let (mut session, ids) = flying_with_escorts(&catalog);
        disable(&mut session, ids[0], Condition::Disabled);
        tick(&mut session, &catalog);
        assert!(session.pilot.missions.free(0).is_some(), "finished");
        tick(&mut session, &catalog);
        assert_released(&session, &ids[1..]);
        let unmarked = npc(&session, ids[0]);
        assert_eq!(unmarked.mission, None);
        assert!(!unmarked.mission_dropped);
        assert_ne!(unmarked.goal, crate::ai::Goal::JumpOut, "no leave");
        assert_eq!(unmarked.ai_type, AiType::WimpyTrader, "its ship's");
        assert_eq!(unmarked.original_ai_type(), 6);
        disable(&mut session, ids[0], Condition::Intact);
        tick(&mut session, &catalog);
        assert_eq!(npc(&session, ids[0]).original_ai_type(), 1);
    }

    #[test]
    fn a_disabled_captured_mission_ship_is_out_for_good_and_the_players_death_frees_it_not() {
        // No re-lead's (@0x8fc07 → @0x8fc2d), and its reset jumps past
        // `_EscortAI` (@0x8ff13 → @0x90032), so +0x88 stays 6.
        let catalog = ships_catalog(escorting(-1, 0));
        let (mut session, ids) = flying_with_escorts(&catalog);
        let captured = session.space.npc_mut(ids[0]).expect("here");
        if let Some(mark) = &mut captured.mission {
            mark.behav = ShipBehav::Own;
        }
        disable(&mut session, ids[0], Condition::Disabled);
        session.ship.condition = Condition::Dying {
            ticks_left: 90,
            starting: false,
        };
        tick(&mut session, &catalog);
        let hired = session.space.escorts.fleet.first().copied().flatten();
        assert_eq!(hired, None, "the player's death frees the hired escort");
        let out = npc(&session, ids[0]);
        assert_eq!(out.mission.map(|mark| mark.formation), Some(None));
        assert_eq!(out.original_ai_type(), 6, "not freed");
        disable(&mut session, ids[0], Condition::Intact);
        session.ship.condition = Condition::Intact;
        tick(&mut session, &catalog);
        let whole = npc(&session, ids[0]);
        assert!(!whole.mission_escort(), "not led again");
        assert_ne!(whole.original_ai_type(), 6, "its think whole");
    }
}
