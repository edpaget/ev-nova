//! The session's side of the berths vacated with their records intact
//! (`nova_sim_ai::traffic::vacated`): a person swapped for its mission ship
//! on the hail ([`Session::swap_in_mission_ship`]) leaves its record at its
//! berth, which rank battle help and a paid helper still count and pick
//! ([`RuleKey::BattleHelpGoneSlots`](crate::RuleKey::BattleHelpGoneSlots)),
//! until a ship takes the berth again or the system is left. Tests only.

#[cfg(test)]
mod tests {
    use crate::ai::{Goal, NovaAi};
    use crate::catalog::MissionId;
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::hail::HailOptions;
    use crate::person::{LEAVES_ON_ACCEPT, REPLACED_BY_MISSION_SHIP};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::hail::tests::{hail, reinforced};
    use crate::session::mission_ships::tests::{hailing_ace, settle, swapping};
    use crate::testkit::{Draws, FakePilotCatalog};
    use crate::traffic::npc::{Npc, NpcId};

    /// [`hailing_ace`] of `flags`, Ace (NPC 0) attacking the player as
    /// mission 140 is accepted, and settled: the session and Ace's berth.
    fn swapped_hostile(flags: u16) -> (Session, u32) {
        let catalog = swapping(flags);
        let mut session = hailing_ace(&catalog);
        let ace = &mut session.space.traffic.npcs_mut()[0];
        assert_eq!(ace.id, NpcId(0));
        ace.goal = Goal::Attack(ShipRef::Player);
        ace.target = Some(ShipRef::Player);
        let berth = ace.berth;
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, &catalog);
        (session, berth)
    }

    #[test]
    fn a_hostile_person_swapped_for_its_mission_ship_keeps_its_berth_as_a_battle_help_threat() {
        // @0x62520 clears only +0xb8: state 4, +0x70 = 0, +0x74 and the
        // armour stand.
        let (session, berth) = swapped_hostile(REPLACED_BY_MISSION_SHIP);
        assert!(session.npcs().iter().all(|npc| npc.id != NpcId(0)), "out");
        let vacated = session.space.traffic.vacated();
        assert_eq!(vacated.len(), 1);
        assert_eq!(vacated[0].id, NpcId(0));
        assert_eq!(vacated[0].berth, berth);
        assert_eq!(vacated[0].goal, Goal::Attack(ShipRef::Player));
        assert!(vacated[0].threatens_player());
    }

    #[test]
    fn a_person_leaving_on_accept_is_swapped_out_no_threat() {
        // The accept's `Flags` 0x0800 runs `_AIMakeShipLeave` (@0x6234d:
        // state 2, +0x70 = -1, @0x7e2c1) before the swap.
        let (session, _) = swapped_hostile(REPLACED_BY_MISSION_SHIP | LEAVES_ON_ACCEPT);
        let vacated = session.space.traffic.vacated();
        assert_eq!(vacated.len(), 1);
        assert_ne!(vacated[0].goal, Goal::Attack(ShipRef::Player));
        assert!(!vacated[0].threatens_player());
    }

    #[test]
    fn a_ship_brought_into_the_swapped_persons_berth_clears_it() {
        let (mut session, berth) = swapped_hostile(REPLACED_BY_MISSION_SHIP);
        let came = session.space.traffic.add_npc(session.npcs()[0].clone());
        let taken = session.npcs().iter().find(|npc| npc.id == came).unwrap();
        assert_eq!(taken.berth, berth, "the first free berth, Ace's");
        assert_eq!(session.space.traffic.vacated(), []);
    }

    /// [`reinforced`]'s session, its Federation warship (NPC 0) of rank
    /// battle help hailed, beside its pirate and a second pirate attacking
    /// the player that left its berth by [`Traffic::vacate`] (as a swap
    /// does): the catalog, the session, the live pirate and the gone one.
    ///
    /// [`Traffic::vacate`]: crate::traffic::Traffic::vacate
    fn helping_beside_a_gone_pirate() -> (FakePilotCatalog, Session, NpcId, NpcId) {
        let (catalog, mut session, pirate) = reinforced();
        let live = session
            .npcs()
            .iter()
            .find(|npc| npc.id == pirate)
            .unwrap()
            .clone();
        let gone = session.space.traffic.add_npc(Npc {
            target: Some(ShipRef::Player),
            ..live
        });
        session.space.traffic.vacate(gone);
        (catalog, session, pirate, gone)
    }

    /// Presses Request Assistance (Nova's second option) of `rulebook`,
    /// drawing `draws`: the reply and the draws asked.
    fn request(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        rulebook: &Rulebook,
        draws: &[u32],
    ) -> (String, Vec<u32>) {
        let options = HailOptions::nova(rulebook);
        let mut chance = Draws::of(draws);
        let view = session
            .answer(1, catalog, &options, &mut chance)
            .expect("open");
        (view.reply, chance.asked)
    }

    fn helper(session: &Session) -> &Npc {
        &session.npcs()[0]
    }

    #[test]
    fn a_warship_sent_at_a_swapped_out_person_says_on_my_way_and_fights_no_threat() {
        let engine = Rulebook::default();
        let (catalog, mut session, _, gone) = helping_beside_a_gone_pirate();
        hail(&mut session, &catalog);
        let (reply, asked) = request(&mut session, &catalog, &engine, &[1]);
        assert_eq!(reply, "Okay, I'm on my way.");
        assert_eq!(asked, [2], "the live pirate and the gone one");
        assert_eq!(helper(&session).goal, Goal::Attack(ShipRef::Npc(gone)));
        assert_eq!(helper(&session).target, Some(ShipRef::Npc(gone)));
        session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
        assert_ne!(helper(&session).goal.attacking(), Some(ShipRef::Npc(gone)));
        assert_ne!(helper(&session).target, Some(ShipRef::Npc(gone)));
        // No ship in the system answers to the one it was sent at, where
        // the same set, the draw picking the pirate, sends it at a ship
        // here.
        assert!(session.npcs().iter().all(|npc| npc.id != gone));
        let (catalog, mut session, pirate, _) = helping_beside_a_gone_pirate();
        hail(&mut session, &catalog);
        let (_, asked) = request(&mut session, &catalog, &engine, &[0]);
        assert_eq!(asked, [2]);
        assert_eq!(helper(&session).goal, Goal::Attack(ShipRef::Npc(pirate)));
        assert!(session.npcs().iter().any(|npc| npc.id == pirate));
    }

    #[test]
    fn a_ship_brought_into_the_gone_pirates_berth_clears_it_from_battle_help() {
        let (catalog, mut session, pirate, gone) = helping_beside_a_gone_pirate();
        let berth = session.space.traffic.vacated()[0].berth;
        let mut idle = session.npcs()[0].clone();
        idle.goal = Goal::Idle;
        let came = session.space.traffic.add_npc(idle);
        let taken = session.npcs().iter().find(|npc| npc.id == came).unwrap();
        assert_eq!(taken.berth, berth, "the gone pirate's berth");
        assert_eq!(session.space.traffic.vacated(), []);
        hail(&mut session, &catalog);
        let (_, asked) = request(&mut session, &catalog, &Rulebook::default(), &[0]);
        assert_eq!(asked, [1], "the live pirate alone");
        assert_eq!(helper(&session).goal, Goal::Attack(ShipRef::Npc(pirate)));
        assert_ne!(pirate, gone);
    }

    #[test]
    fn by_the_other_reading_a_swapped_out_person_is_never_picked() {
        let bible =
            Rulebook::default().with_override(RuleKey::BattleHelpGoneSlots, RuleSource::Bible);
        let (catalog, mut session, pirate, _) = helping_beside_a_gone_pirate();
        hail(&mut session, &catalog);
        let (reply, asked) = request(&mut session, &catalog, &bible, &[0]);
        assert_eq!(reply, "Okay, I'm on my way.");
        assert_eq!(asked, [1], "the live pirate alone");
        assert_eq!(helper(&session).goal, Goal::Attack(ShipRef::Npc(pirate)));
    }
}
