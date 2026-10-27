//! Boarding the player's own disabled escorts and fighters back: the
//! session's side of [`board_repair`](crate::board_repair), the own-ship
//! paths of `_HandlePlayerBoardAttempt` (@0x65843-0x65eb3).
//!
//! **When.** After the velocity match and the boarding's crime, and
//! after a person's offer: a person whose `LinkMission` and `Flags`
//! 0x0200 make it the offering ship never takes one, offered or
//! plundered. The path is
//! [`recovery`](crate::board_repair::recovery)'s, from who flies the
//! ship, what it remembers being ([`Npc::former`]), whether the fleet has
//! room ([`Session::can_hire_escorts`]) and whether a bay carrying its
//! type has room ([`Session::can_take_fighter`]). An own-ship path takes
//! the place of the plunder: no Good Samaritan, no airlock, no plunder
//! and no grant.
//!
//! **Escort repaired** (@0x65b8a-0x65d2c). The ship rejoins the fleet
//! where it is, as the record it left with
//! ([`FormerEscort::rejoined`](crate::self_repair::FormerEscort::rejoined)),
//! hired with its wage or captured, led by the player (+0x88 6, +0x9a 0).
//! Its armour goes to [`repaired_armor`](crate::self_repair::repaired_armor)
//! (@0x65bf2-0x65c66) and its condition is decided again, by the
//! self-repair's own [`Session::repair_hull`]; its shield
//! stays as it is. `_AISetup` (@0x65c76) clears its order word (+0xc90a),
//! which is also its standing order, so it keeps formation
//! ([`Npc::order_word`] [`OrderWord::NONE`], the record's order none),
//! unlike a self-repair's rejoin, which keeps the order. The six
//! commodities in its own hold are added to the player's
//! (@0x65c7b-0x65ca2), with no hold check and no reset, its own hold left
//! as it is: the share it took as it was lost (see the `escorts`
//! module's `destroy_cargo_share`) comes back. A save is due.
//!
//! **Fighter repaired** and **Fighter captured** (@0x65a65-0x65b80,
//! @0x65d31-0x65e62): the ship docks at once in the player's bay that
//! carries its type (`_AIAddFighterToParent` @0x65b80, its slot let go
//! @0x80404), a round of that bay's fighter outfit, as a fighter docking
//! gives ([`Session::dock_fighter`]), and leaves the system. The resets
//! the tail writes before the dock (shield, armour, +0x6c) are never read.
//!
//! **Every own-ship path** ends at the board's tail (@0x65f43): the ship
//! is marked boarded ([`Session::mark_boarded`], `_AIShipBoardedByPlayer`
//! @0x65f6d), and the player is told "Escort repaired.", "Fighter
//! repaired." or "Fighter captured." for 250 ticks with `_beepSnd[4]` at
//! priority 8 (`snd ` 154, [`SimSound::EscortRepaired`], @0x65d27,
//! @0x65b1e, @0x65e51), the self-repair rejoin's sound.
//!
//! **Memory.** A ship that left the fleet disabled remembers what it was
//! ([`Npc::former`], the original's +0xc8de). Only the own-ship paths
//! clear it (@0x65b2f, @0x65be8, @0x65e62): boarding it on the plunder,
//! offer or mission-ship paths leaves it, so a former escort plundered
//! while the fleet is full can be boarded again, plundered again (a fresh
//! plunder and a fresh crime), and repaired once there is room.

use super::Session;
use crate::board_repair::{FlownBy, Recovery, recovery};
use crate::catalog::ShipId;
use crate::fleet::OrderWord;
use crate::market::Good;
use crate::person::MISSION_ON_BOARD;
use crate::pilot::Escort;
use crate::self_repair::FormerEscort;
use crate::sound::SimSound;
use crate::traffic::npc::{Npc, NpcId};

impl Session {
    /// Boards `npc` back as the player's own, when one of the own-ship
    /// paths takes it (see the module docs): the path, or none for the
    /// plunder, with nothing changed.
    pub(super) fn recover(&mut self, npc: &Npc) -> Option<Recovery> {
        let flown_by = npc
            .person
            .map_or(FlownBy::NoPerson, |person| FlownBy::Person {
                link_mission: person.mission,
                offers_on_board: person.flags & MISSION_ON_BOARD != 0,
            });
        let former = npc.former.map(|former| former.kind());
        let path = recovery(
            flown_by,
            former,
            self.can_hire_escorts(),
            self.can_take_fighter(npc.ship),
        )?;
        match (path, npc.former) {
            (Recovery::EscortRepaired, Some(former)) => self.repair_escort(npc.id, former),
            _ => self.dock_boarded(npc.id, npc.ship),
        }
        self.space.say(&mut self.out, path.message());
        self.out.sounds.push(SimSound::EscortRepaired);
        Some(path)
    }

    /// Former escort NPC `id`, boarded, rejoins the fleet as `former`
    /// says, repaired, its hold added to the player's cargo (see the
    /// module docs).
    fn repair_escort(&mut self, id: NpcId, former: FormerEscort) {
        let rule = std::rc::Rc::clone(&self.setup.disable_rule.0);
        self.repair_hull(id, &*rule);
        let Some(npc) = self.space.npc_mut(id) else {
            return;
        };
        npc.order_word = OrderWord::NONE;
        let hold: Vec<(Good, u32)> = npc.cargo.held().collect();
        let record = Escort {
            order: None,
            ..former.rejoined(npc.reserves)
        };
        for (good, tons) in hold {
            let carried = self.pilot.cargo.entry(good).or_default();
            *carried = carried.saturating_add(tons);
        }
        self.enlist(id, record);
        self.mark_boarded(id);
    }

    /// NPC `id`, of ship type `ship`, boarded, docks in the player's bay
    /// carrying its type and leaves the system (see the module docs).
    fn dock_boarded(&mut self, id: NpcId, ship: ShipId) {
        self.mark_boarded(id);
        self.ship.stow_fighter(&mut self.pilot, &self.statics, ship);
        self.leave(id);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::flight::Controls;
    use crate::testkit::work;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use super::*;
    use crate::board::{Boarding, MAX_ESCORTS, Take, Taken};
    use crate::catalog::GovtId;

    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort::EscortOrder;
    use crate::legal::NovaLaw;
    use crate::message::SimMessage;
    use crate::reserves::Gauge;
    use crate::self_repair::repaired_armor;
    use crate::session::fighters::tests::{VIPER, VIPERS, carrying, fleet, launch};
    use crate::session::tests::{alongside, board_by, boardable, flown_by, take, trader_escort};
    use crate::traffic::npc::NpcHold;

    /// A former escort's memory: the trader, hired at 30 a day, defending.
    fn former_trader() -> FormerEscort {
        FormerEscort {
            record: Escort {
                wage: Some(30),
                order: Some(EscortOrder::Defend),
                ..trader_escort()
            },
        }
    }

    /// Fills the fleet with [`MAX_ESCORTS`] interceptors, unplaced.
    fn fill_fleet(session: &mut Session) {
        session.pilot.escorts = vec![
            Escort {
                ship: ShipId(130),
                ..trader_escort()
            };
            MAX_ESCORTS
        ];
    }

    /// [`alongside`]'s trader (NPC 0, armour 10 of 45) remembering being
    /// [`former_trader`], its order word 3, with no message, sound or
    /// save due.
    fn former_alongside(catalog: &crate::testkit::FakePilotCatalog) -> Session {
        let mut session = alongside(catalog);
        let trader = &mut session.space.traffic.npcs_mut()[0];
        trader.former = Some(former_trader());
        trader.order_word = OrderWord(3);
        let _ = (session.take_messages(), session.take_sounds());
        session.take_save_due();
        session
    }

    #[test]
    fn boarding_a_disabled_former_hired_escort_repairs_and_rehires_it() {
        let catalog = boardable();
        let mut session = former_alongside(&catalog);
        let shield = session.npcs()[0].reserves.shield;
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Recovered(Recovery::EscortRepaired))
        );
        let armor = repaired_armor(45.0, false);
        let npc = &session.npcs()[0];
        assert_eq!(
            npc.reserves.armor,
            Gauge {
                now: armor,
                max: 45.0
            }
        );
        assert_eq!(npc.reserves.shield, shield, "the shield isn't written");
        assert_eq!(npc.condition, Condition::Intact, "decided again");
        assert!(npc.escort.is_some(), "led");
        assert_eq!(npc.fleet(), ShipRef::Player);
        assert!(npc.boarded, "marked boarded again at the tail");
        assert_eq!(npc.former, None);
        assert_eq!(npc.order_word, OrderWord::NONE, "`_AISetup`");
        assert_eq!(
            session.pilot().escorts(),
            [Escort {
                wage: Some(30),
                order: None,
                reserves: npc.reserves,
                ..trader_escort()
            }],
            "its wage kept, its standing order cleared"
        );
        assert!(session.is_escort(NpcId(0)));
        assert_eq!(session.take_messages(), [SimMessage::EscortRepaired]);
        assert_eq!(
            session.take_sounds(),
            [SimSound::EscortRepaired],
            "no airlock"
        );
        assert_eq!(session.space.strikes, [], "no Good Samaritan");
        assert_eq!(session.boarding(), None, "no plunder");
        assert_eq!(session.take_grant(), None);
        assert!(session.take_save_due());
        assert_eq!(
            session.pilot().legal_record(GovtId(140)),
            -5,
            "convicted before the repair (@0x6582b)"
        );
    }

    #[test]
    fn a_former_captured_escort_is_repaired_unhired() {
        let catalog = boardable();
        let mut session = former_alongside(&catalog);
        session.space.traffic.npcs_mut()[0].former = Some(FormerEscort {
            record: trader_escort(),
        });
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Recovered(Recovery::EscortRepaired))
        );
        assert_eq!(session.pilot().escorts()[0].wage, None);
    }

    #[test]
    fn a_repaired_escort_brings_its_hold_into_the_players_cargo_unchecked() {
        let catalog = boardable();
        let mut session = former_alongside(&catalog);
        let hold = NpcHold::from_words([5, 0, 7, 0, 0, 0]);
        session.space.traffic.npcs_mut()[0].cargo = hold;
        session.pilot.cargo = BTreeMap::from([(Good::Commodity(0), 18)]);
        assert!(matches!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Recovered(_))
        ));
        assert_eq!(
            session.pilot().cargo,
            BTreeMap::from([(Good::Commodity(0), 23), (Good::Commodity(2), 7)]),
            "past the 20 holds, with no trim"
        );
        assert_eq!(session.npcs()[0].cargo, hold, "its own hold left");
    }

    #[test]
    fn a_former_escort_with_the_fleet_full_is_plundered_instead() {
        let catalog = boardable();
        let mut session = former_alongside(&catalog);
        fill_fleet(&mut session);
        assert!(matches!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Opened(_))
        ));
        assert!(session.boarding().is_some());
        assert_eq!(session.take_sounds(), [SimSound::Boarded], "the airlock");
        assert_eq!(session.take_messages(), []);
        assert_eq!(session.space.strikes.len(), 1, "the Good Samaritan");
        assert_eq!(session.npcs()[0].former, Some(former_trader()));
        assert_eq!(session.npcs()[0].escort, None);
    }

    #[test]
    fn a_former_escort_plundered_keeps_its_memory_and_can_be_boarded_again() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        fill_fleet(&mut session);
        session.space.traffic.npcs_mut()[0].former = Some(former_trader());
        assert!(matches!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Opened(_))
        ));
        let npc = &session.npcs()[0];
        assert!(npc.boarded);
        assert_eq!(npc.former, Some(former_trader()), "the plunder keeps it");
        assert_eq!(take(&mut session, Take::Abort, &[]).0, Taken::Aborted);
        assert!(
            matches!(
                board_by(&mut session, &NovaLaw::default()),
                Ok(Boarding::Opened(_))
            ),
            "boarded again (@0x65064-0x6507a)"
        );
        assert_eq!(session.npcs()[0].former, Some(former_trader()));
        assert_eq!(
            session.pilot().legal_record(GovtId(140)),
            -10,
            "a fresh crime"
        );
        session.pilot.escorts.clear();
        assert_eq!(take(&mut session, Take::Abort, &[]).0, Taken::Aborted);
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Recovered(Recovery::EscortRepaired)),
            "repaired once there is room"
        );
    }

    #[test]
    fn a_ship_boarded_before_that_never_was_an_escort_is_refused() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.space.traffic.npcs_mut()[0].boarded = true;
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Err(crate::board::BoardRefusal::CantBoard)
        );
    }

    /// Disables every ship whatever its armour.
    #[derive(Debug)]
    struct AlwaysDisabled;

    impl crate::combat::hull::DisableRule for AlwaysDisabled {
        fn disabled(&self, _armor: Gauge, _hull: &crate::combat::hull::HullSpec) -> bool {
            true
        }
    }

    #[test]
    fn a_repair_the_disable_rule_still_disables_rejoins_disabled() {
        let catalog = boardable();
        let mut session = former_alongside(&catalog).with_disable_rule(Rc::new(AlwaysDisabled));
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Recovered(Recovery::EscortRepaired))
        );
        let npc = &session.npcs()[0];
        assert_eq!(npc.condition, Condition::Disabled);
        assert!(session.is_escort(npc.id), "rejoins all the same");
    }

    #[test]
    fn a_repair_of_a_ship_held_disabled_rejoins_disabled() {
        // Decided again as a self-repair decides it: `_IsDisabled` holds a
        // Derelict government's ship whatever its armour.
        let mut catalog = boardable();
        catalog.govts[0].flags |= crate::govt::DERELICT;
        let mut session = former_alongside(&catalog);
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Recovered(Recovery::EscortRepaired))
        );
        assert_eq!(session.npcs()[0].condition, Condition::Disabled);
    }

    #[test]
    fn a_person_flown_former_escort_takes_the_person_path() {
        // The person test (@0x65847) comes first.
        let catalog = boardable();
        let mut session = former_alongside(&catalog);
        session.space.traffic.npcs_mut()[0].person = Some(flown_by(600));
        assert!(matches!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Opened(_))
        ));
        assert!(session.pilot().escorts().is_empty());
        assert_eq!(session.npcs()[0].former, Some(former_trader()));
    }

    #[test]
    fn repairing_a_lost_trader_gives_its_share_back() {
        // Its share was written into its hold as it was lost (@0xcf76) and
        // comes back with the repair (@0x65c7b-0x65ca2).
        use crate::rulebook::RuleSource;
        use crate::session::escorts::tests::{FOOD, fleeted, hit, laden, trader_fleet};
        let catalog = fleeted();
        let mut session = laden(&catalog, trader_fleet(), RuleSource::Engine);
        let id = session.space.escorts.fleet[1].expect("placed");
        hit(&mut session, 1, 10.0);
        assert!(!session.is_escort(id));
        assert_eq!(session.pilot().held(FOOD), 5, "4 of 9 destroyed");
        crate::fixture::put_player_over(&mut session, id);
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Recovered(Recovery::EscortRepaired))
        );
        assert_eq!(session.pilot().held(FOOD), 9, "given back");
        assert!(session.is_escort(id));
    }

    // Fighters.

    /// [`carrying`] with `vipers` Vipers aboard and one launched, then
    /// disabled: it left the fleet remembering being a fighter, the player
    /// over it and targeting it. Its NPC.
    fn disabled_viper(catalog: &crate::testkit::FakePilotCatalog, vipers: u16) -> (Session, NpcId) {
        let mut session = fleet(catalog, vipers, Vec::new());
        launch(&mut session, catalog);
        let id = session
            .npcs()
            .iter()
            .find(|npc| npc.ship == VIPER)
            .expect("launched")
            .id;
        session.space.npc_mut(id).expect("there").reserves.armor.now = 1.0;
        work(&mut session, &catalog, Controls::default());
        assert!(!session.is_escort(id), "lost to the fleet");
        assert_eq!(
            session
                .space
                .npc(id)
                .and_then(|npc| npc.former)
                .map(|f| f.kind()),
            Some(crate::self_repair::FormerKind::Fighter)
        );
        crate::fixture::put_player_over(&mut session, id);
        let _ = (session.take_messages(), session.take_sounds());
        session.take_save_due();
        (session, id)
    }

    #[test]
    fn boarding_ones_own_disabled_fighter_stows_it_in_its_bay() {
        let catalog = carrying();
        let (mut session, id) = disabled_viper(&catalog, 3);
        assert_eq!(session.pilot().owned(VIPERS), 2);
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Recovered(Recovery::FighterRepaired))
        );
        assert_eq!(session.pilot().owned(VIPERS), 3, "a round back");
        assert!(session.space.npc(id).is_none(), "docked");
        assert_eq!(session.target(), None, "let go");
        assert!(session.pilot().escorts().is_empty());
        assert_eq!(session.take_messages(), [SimMessage::FighterRepaired]);
        assert_eq!(session.take_sounds(), [SimSound::EscortRepaired]);
        assert_eq!(session.space.strikes, []);
        assert_eq!(session.boarding(), None);
    }

    #[test]
    fn boarding_a_disabled_ship_of_a_carried_type_captures_it_into_the_bay() {
        let catalog = carrying();
        let (mut session, id) = disabled_viper(&catalog, 3);
        session.space.npc_mut(id).expect("there").former = None;
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Recovered(Recovery::FighterCaptured))
        );
        assert_eq!(session.pilot().owned(VIPERS), 3);
        assert!(session.space.npc(id).is_none());
        assert_eq!(session.take_messages(), [SimMessage::FighterCaptured]);
        assert_eq!(session.take_sounds(), [SimSound::EscortRepaired]);
    }

    #[test]
    fn a_full_bay_leaves_the_fighter_to_the_plunder() {
        let catalog = carrying();
        let (mut session, id) = disabled_viper(&catalog, 3);
        session.pilot.outfits.insert(VIPERS, 4);
        assert!(matches!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Opened(_))
        ));
        assert!(session.space.npc(id).is_some());
        assert_eq!(session.pilot().owned(VIPERS), 4);
        assert_eq!(session.take_sounds(), [SimSound::Boarded]);
    }

    #[test]
    fn a_person_without_a_link_mission_boards_its_fighter_back_and_one_with_0x0200_does_not() {
        let catalog = carrying();
        let (mut session, id) = disabled_viper(&catalog, 3);
        session.space.npc_mut(id).expect("there").person = Some(flown_by(600));
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Recovered(Recovery::FighterRepaired)),
            "no LinkMission (@0x65a65-0x65b85)"
        );

        let (mut session, id) = disabled_viper(&catalog, 3);
        session.space.npc_mut(id).expect("there").person = Some(crate::traffic::npc::NpcPerson {
            mission: true,
            ..flown_by(600)
        });
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Recovered(Recovery::FighterRepaired)),
            "a LinkMission, no 0x0200 (@0x65a21-0x65a60)"
        );

        let (mut session, id) = disabled_viper(&catalog, 3);
        session.space.npc_mut(id).expect("there").person = Some(crate::traffic::npc::NpcPerson {
            mission: true,
            flags: MISSION_ON_BOARD,
            ..flown_by(600)
        });
        assert!(
            matches!(
                board_by(&mut session, &NovaLaw::default()),
                Ok(Boarding::Opened(_))
            ),
            "a LinkMission and 0x0200: its plunder (@0x65867)"
        );
        assert!(session.space.npc(id).is_some());
    }

    // Persons 1150 and 1151.

    #[test]
    fn the_bounty_hunter_is_boarded_uncharged_with_the_good_samaritan_whatever_its_link_mission() {
        // +0xc8d0 0x3fe skips the slap, the person path and the own-ship
        // paths for the Good Samaritan, the airlock and the plunder
        // (@0x657e9-0x657fe → @0x65ece-0x65f27).
        let mut catalog = crate::session::person_offers::tests::boardable(0);
        catalog.persons[0].id = crate::board::BOUNTY_HUNTER;
        for (_, traffic) in &mut catalog.traffic {
            traffic.persons[0].0 = Some(crate::board::BOUNTY_HUNTER);
        }
        let mut session = crate::session::person_offers::tests::alongside_ace(&catalog);
        let ace = &mut session.space.traffic.npcs_mut()[0];
        assert_eq!(
            ace.person.map(|person| person.id),
            Some(crate::board::BOUNTY_HUNTER)
        );
        ace.former = Some(former_trader());
        let records: Vec<_> = session.pilot().legal_records().collect();
        let _ = session.take_sounds();
        let (boarded, _) = crate::session::person_offers::tests::board(&mut session);
        assert!(matches!(boarded, Ok(Boarding::Opened(_))), "{boarded:?}");
        assert_eq!(
            session.pilot().legal_records().collect::<Vec<_>>(),
            records,
            "no crime"
        );
        assert_eq!(session.space.strikes.len(), 1, "the Good Samaritan");
        assert_eq!(session.take_sounds(), [SimSound::Boarded], "the airlock");
        assert!(session.pilot().escorts().is_empty(), "no repair");
        assert!(session.pilot().missions().missions().is_empty(), "no offer");
    }
}
