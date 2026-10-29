//! The session's side of a ship's repair of the disabled player
//! (`_LowLevelAIHandler` @0x86d84-0x86f37 in the `EV Nova` executable),
//! which the traffic queues as [`PirateEvent::PlayerRepaired`] once the
//! ship has docked and waited its dock (see
//! [`repair_dock`](crate::traffic::repair_dock)), on the line at the
//! repairer's berth, and [`Session::answer_pirate`] answers in slot order
//! with the frame's other lines as the pass settles them (see the
//! `comm_calls` module).
//!
//! - The player is unboarded (+0xb9 = 0, @0x86d92).
//! - While the player's ship is disabled, its armour is raised by
//!   [`raise_armor`] (@0x86d9c-0x86dcf) under the session's
//!   [`DisableRule`], and its condition is decided again at once on the
//!   armour the repair left. A player no longer disabled gains nothing,
//!   as the test comes before the first step.
//! - "`CommName`:  Repairs complete, `ship name`." is said
//!   ([`CommNote`], `STR#` 2002 #384, the player's ship's name from
//!   `*0x3b5210`), holding the message line for 240 ticks (`_DisplayComm`
//!   @0x86ef4), with `_beepSnd[1]` ([`SimSound::RepairsComplete`],
//!   `snd ` 151, @0x86ddf-0x86e0a), whatever the repair raised.
//!
//! A ship whose target is a disabled NPC (a hit's retarget at a ship
//! since disabled) repairs that ship instead, which the traffic queues as
//! [`PirateEvent::NpcRepaired`] ([`Space::npc_repaired`]): the NPC is
//! unboarded (@0x86d92), and while disabled its armour is raised by
//! [`raise_armor`] under the same rule, a mission's rescue ship that
//! `_IsDisabled` holds disabled at any armour rising to its most (the
//! original's loop never ends for one). Its condition is decided
//! again at once. Nothing is said and no sound plays: the line and
//! `snd ` 151 are for the player's nav alone (@0x86dd4).
//!
//! The original says it inside the repairer's think; here it is said
//! after the traffic's tick, before the frame is drawn
//! (rdm:phase/combat-ai-fidelity/phase-22-frame-lines-in-slot-order owns
//! the slot order). An NPC's repair settles in the same place, after
//! the traffic's tick and before the frame is drawn: no boundary is
//! crossed. The `_statusRedraw` writes are left out, as the status
//! display reads the live state.

use super::{Session, Setup, Space, Statics};
use crate::catalog::ShipId;
#[cfg(any(doc, test))]
use crate::combat::hull::DisableRule;
use crate::combat::hull::{Condition, HullSpec};
#[cfg(doc)]
use crate::hail::CommNote;
use crate::hail::Help;
use crate::pilot::Pilot;
use crate::reserves::Gauge;
use crate::sound::SimSound;
use crate::traffic::npc::NpcId;
#[cfg(doc)]
use crate::traffic::plunder::PirateEvent;
use crate::traffic::repair_dock::raise_armor;

impl Session {
    /// NPC `helper`, of ship type `ship`, has repaired the player (see the
    /// module docs).
    pub(super) fn repaired_by(&mut self, helper: NpcId, ship: ShipId) {
        self.ship.fate.player_boarded = false;
        if self.ship.condition == Condition::Disabled {
            let rule = std::rc::Rc::clone(&self.setup.disable_rule.0);
            let armor = &mut self.pilot.reserves.armor;
            raise_armor(armor, &self.ship.fit.hull, |armor, hull| {
                rule.disabled(armor, hull)
            });
            if !rule.disabled(*armor, &self.ship.fit.hull) {
                self.ship.condition = Condition::Intact;
            }
        }
        self.space.say_help_done(
            &self.pilot,
            &self.statics,
            &mut self.out,
            helper,
            ship,
            Help::Repair,
            SimSound::RepairsComplete,
        );
    }
}

impl Space {
    /// NPC `ship` has been repaired by a ship docked with it (see the
    /// module docs): unboarded, then, while disabled, its armour raised
    /// as the player's is, `_IsDisabled` holding a mission's rescue ship
    /// disabled at any armour; silently.
    pub(super) fn npc_repaired(
        &mut self,
        pilot: &Pilot,
        statics: &Statics,
        setup: &Setup,
        ship: NpcId,
    ) {
        let rule = std::rc::Rc::clone(&setup.disable_rule.0);
        let Some(npc) = self.npc_mut(ship) else {
            return;
        };
        npc.boarded = false;
        if npc.condition != Condition::Disabled {
            return;
        }
        let held = self
            .npc(ship)
            .is_some_and(|npc| super::mission_ships::mission_ship_held(pilot, statics, npc));
        let Some(npc) = self.npc_mut(ship) else {
            return;
        };
        let disabled = |armor: Gauge, hull: &HullSpec| held || rule.disabled(armor, hull);
        raise_armor(&mut npc.reserves.armor, &npc.hull, disabled);
        if !disabled(npc.reserves.armor, &npc.hull) {
            npc.condition = Condition::Intact;
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::ai::wimpy::WimpyTrader;

    use crate::combat::ShipRef;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::hail::CommNote;
    use crate::session::mission_ships::tests::{goal_catalog, in_flight};
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::work_with;
    use crate::testkit::{Draws, FakePilotCatalog};
    use crate::traffic::plunder::PirateEvent;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// [`armed`] with its NPC's ship's `CommName` "Shuttle".
    fn catalog() -> FakePilotCatalog {
        let mut catalog = armed();
        for record in &mut catalog.ship_records {
            record.comm_name = "Shuttle".to_owned();
        }
        catalog
    }

    /// Records the armour it is asked about, disabled below 20.
    #[derive(Debug, Default)]
    struct Below20 {
        asked: RefCell<Vec<f32>>,
    }

    impl DisableRule for Below20 {
        fn disabled(&self, armor: Gauge, _hull: &HullSpec) -> bool {
            self.asked.borrow_mut().push(armor.now);
            armor.now < 20.0
        }
    }

    /// Has every ship disabled, whatever its armour.
    #[derive(Debug)]
    struct Always;

    impl DisableRule for Always {
        fn disabled(&self, _armor: Gauge, _hull: &HullSpec) -> bool {
            true
        }
    }

    /// `catalog`'s session under `rule`, the player's ship the "Kestrel",
    /// disabled and boarded with 10 armour of 45, its sounds taken, and
    /// its NPC.
    fn downed(catalog: &FakePilotCatalog, rule: Rc<dyn DisableRule>) -> (Session, NpcId) {
        let mut session = facing_an_npc(catalog, 180).with_disable_rule(rule);
        session.pilot.ship_name = Some("Kestrel".to_owned());
        session.ship.condition = Condition::Disabled;
        session.ship.fate.player_boarded = true;
        session.pilot.reserves.armor = Gauge {
            now: 10.0,
            max: 45.0,
        };
        let _ = session.take_sounds();
        let helper = session.npcs()[0].id;
        (session, helper)
    }

    /// `session` answering a repair by `helper`.
    fn answer(session: &mut Session, helper: NpcId) {
        let ship = session.space.npc(helper).expect("here").ship;
        session
            .space
            .traffic
            .queue_pirate_event(PirateEvent::PlayerRepaired { helper, ship });
        session.settle_line_queue(crate::Rules::default().law, &mut Draws::of(&[]));
    }

    #[test]
    fn a_repair_unboards_the_player_raises_its_armour_until_it_flies_and_says_so_with_snd_151() {
        let catalog = catalog();
        let rule = Rc::new(Below20::default());
        let (mut session, helper) = downed(&catalog, rule.clone());
        answer(&mut session, helper);
        // How far each step goes is `raise_armor`'s, pinned in nova-sim-ai.
        let armor = session.reserves().armor.now;
        assert!(armor >= 20.0, "raised until the rule lets it fly: {armor}");
        assert_eq!(
            rule.asked.borrow().last(),
            Some(&armor),
            "the condition decided again on the armour the repair left"
        );
        assert_eq!(session.player_condition(), Condition::Intact);
        assert!(!session.player_boarded());
        assert_eq!(
            session.take_comm(),
            [CommNote {
                from: helper,
                comm_name: "Shuttle".to_owned(),
                ship_name: "Kestrel".to_owned(),
                done: Help::Repair,
            }]
        );
        assert_eq!(session.take_sounds(), [SimSound::RepairsComplete]);
        for tick in 1..240 {
            session.space.count_down_comm_line();
            assert!(!session.comm_line().free_for_calls(), "{tick}");
        }
        session.space.count_down_comm_line();
        assert!(session.comm_line().free_for_calls(), "240 ticks");
    }

    #[test]
    fn a_repairs_note_is_the_line_raised_last_for_its_ticks() {
        let catalog = catalog();
        let (mut session, helper) = downed(&catalog, Rc::new(Below20::default()));
        session.space.say(
            &mut session.out,
            crate::SimMessage::FleetArrivalQuote("Hail".to_owned()),
        );
        answer(&mut session, helper);
        let note = CommNote {
            from: helper,
            comm_name: "Shuttle".to_owned(),
            ship_name: "Kestrel".to_owned(),
            done: Help::Repair,
        };
        let mut line = crate::comm_line::CommLine::default();
        line.show(note.ticks());
        assert_eq!(session.comm_line(), line);
        assert_eq!(
            session.out.last_line,
            Some(crate::flight_frame::RaisedLine::Comm(note))
        );
    }

    /// `session` with a second NPC, a copy of its first, disabled and
    /// boarded with 10 armour of 45, answering its repair by the first.
    fn npc_repaired(session: &mut Session, helper: NpcId) -> NpcId {
        npc_repaired_in(session, helper, Condition::Disabled)
    }

    /// [`npc_repaired`], the second NPC in `condition`.
    fn npc_repaired_in(session: &mut Session, helper: NpcId, condition: Condition) -> NpcId {
        let mut downed = session.space.npc(helper).expect("here").clone();
        downed.condition = condition;
        downed.boarded = true;
        downed.reserves.armor = Gauge {
            now: 10.0,
            max: 45.0,
        };
        let ship = session.space.traffic.add_npc(downed);
        session
            .space
            .traffic
            .queue_pirate_event(PirateEvent::NpcRepaired { helper, ship });
        session.settle_line_queue(crate::Rules::default().law, &mut Draws::of(&[]));
        ship
    }

    #[test]
    fn an_npc_repair_raises_its_armour_out_of_disabled_and_unboards_it_silently() {
        let catalog = catalog();
        let rule = Rc::new(Below20::default());
        let (mut session, helper) = downed(&catalog, rule.clone());
        let ship = npc_repaired(&mut session, helper);
        let repaired = session.space.npc(ship).expect("here");
        let armor = repaired.reserves.armor.now;
        assert!(armor >= 20.0, "raised until the rule lets it fly: {armor}");
        assert_eq!(repaired.condition, Condition::Intact);
        assert!(!repaired.boarded, "@0x86d92");
        assert_eq!(session.take_comm(), [], "no line, @0x86dd4");
        assert_eq!(session.take_sounds(), [], "no snd 151");
        assert_eq!(session.reserves().armor.now, 10.0, "not the player's");
        assert!(session.player_boarded(), "the player stays boarded");
    }

    #[test]
    fn an_npc_repair_raises_the_armour_no_further_than_its_most() {
        let catalog = catalog();
        let (mut session, helper) = downed(&catalog, Rc::new(Always));
        let ship = npc_repaired(&mut session, helper);
        let repaired = session.space.npc(ship).expect("here");
        assert_eq!(repaired.reserves.armor.now, 45.0);
        assert_eq!(repaired.condition, Condition::Disabled);
        assert!(!repaired.boarded);
    }

    #[test]
    fn an_npc_repair_of_a_ship_no_longer_disabled_unboards_it_and_leaves_its_armour() {
        let catalog = catalog();
        let rule = Rc::new(Below20::default());
        let (mut session, helper) = downed(&catalog, rule.clone());
        let ship = npc_repaired_in(&mut session, helper, Condition::Intact);
        let repaired = session.space.npc(ship).expect("here");
        assert!(!repaired.boarded, "@0x86d92, before the test");
        assert_eq!(repaired.reserves.armor.now, 10.0, "nothing raised");
        assert_eq!(repaired.condition, Condition::Intact);
        assert!(rule.asked.borrow().is_empty(), "the rule is never asked");
    }

    #[test]
    fn an_npc_repair_of_a_ship_no_longer_here_changes_nothing() {
        let catalog = catalog();
        let (mut session, helper) = downed(&catalog, Rc::new(Below20::default()));
        let gone = NpcId(u32::MAX);
        assert!(session.space.npc(gone).is_none());
        let npcs = session.npcs().to_vec();
        session
            .space
            .traffic
            .queue_pirate_event(PirateEvent::NpcRepaired { helper, ship: gone });
        session.settle_line_queue(crate::Rules::default().law, &mut Draws::of(&[]));
        assert_eq!(session.npcs(), npcs);
        assert_eq!(session.reserves().armor.now, 10.0);
        assert!(session.player_boarded());
        assert_eq!(session.take_comm(), []);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn an_npc_repair_of_a_held_rescue_ship_fills_its_armour_and_leaves_it_disabled_unboarded() {
        let catalog = goal_catalog(5, 0);
        let mut session = in_flight(&catalog);
        let [helper, ship] = [0, 1].map(|index| session.npcs()[index].id);
        let npc = session.space.npc_mut(ship).expect("here");
        // Boarded, it is not held; the repair unboards it before the test.
        npc.boarded = true;
        npc.reserves.armor.now = 1.0;
        let most = npc.reserves.armor.max;
        let hull = npc.hull;
        let full = Gauge {
            now: most,
            max: most,
        };
        assert!(
            !session.setup.disable_rule.0.disabled(full, &hull),
            "the rule alone would let it fly at its most"
        );
        session
            .space
            .traffic
            .queue_pirate_event(PirateEvent::NpcRepaired { helper, ship });
        session.settle_line_queue(crate::Rules::default().law, &mut Draws::of(&[]));
        let repaired = session.space.npc(ship).expect("here");
        assert_eq!(repaired.reserves.armor.now, most, "to its most");
        assert_eq!(repaired.condition, Condition::Disabled, "held");
        assert!(!repaired.boarded);
        assert!(session.mission_ship_held(repaired));
    }

    #[test]
    fn a_repair_raises_the_armour_no_further_than_its_most() {
        let catalog = catalog();
        let (mut session, helper) = downed(&catalog, Rc::new(Always));
        answer(&mut session, helper);
        assert_eq!(session.reserves().armor.now, 45.0, "full, and done");
        assert_eq!(session.player_condition(), Condition::Disabled);
        assert_eq!(session.take_comm().len(), 1);
    }

    #[test]
    fn a_repair_says_its_line_though_the_player_repaired_itself_during_the_dock() {
        let catalog = catalog();
        let rule = Rc::new(Below20::default());
        let (mut session, helper) = downed(&catalog, rule.clone());
        session.ship.condition = Condition::Intact;
        answer(&mut session, helper);
        assert_eq!(session.reserves().armor.now, 10.0);
        assert!(rule.asked.borrow().is_empty());
        assert_eq!(session.player_condition(), Condition::Intact);
        assert_eq!(session.take_comm().len(), 1);
        assert_eq!(session.take_sounds(), [SimSound::RepairsComplete]);
    }

    #[test]
    fn a_ship_repairing_the_player_through_the_session_repairs_it_once_and_drifts() {
        let catalog = catalog();
        let (mut session, helper) = downed(&catalog, Rc::new(Below20::default()));
        session.ship.fate.player_boarded = false;
        let npc = session.space.npc_mut(helper).expect("here");
        npc.goal = Goal::Assist(Help::Repair);
        npc.target = Some(ShipRef::Player);
        npc.state.position = Vec2::new(2.0, 0.0);
        npc.state.velocity = Vec2::ZERO;
        let tick = |session: &mut Session| {
            work_with(
                session,
                &catalog,
                Controls::default(),
                &WimpyTrader::default(),
                crate::Rules::default(),
                &mut Draws::of(&[0; 64]),
            );
            session.take_comm()
        };
        let mut boarded = false;
        let mut notes = Vec::new();
        for _ in 0..200 {
            notes = tick(&mut session);
            boarded |= session.player_boarded();
            if !notes.is_empty() {
                break;
            }
        }
        assert!(boarded, "the dock boarded the player");
        assert_eq!(notes.len(), 1, "{notes:?}");
        let armor = session.reserves().armor.now;
        assert!(armor >= 20.0, "raised until the rule lets it fly: {armor}");
        assert_eq!(session.player_condition(), Condition::Intact);
        assert!(!session.player_boarded());
        assert!(session.take_sounds().contains(&SimSound::RepairsComplete));
        assert_eq!(
            session.space.npc(helper).map(|npc| npc.goal),
            Some(Goal::Drift)
        );
        for _ in 0..50 {
            assert!(tick(&mut session).is_empty(), "repaired once");
        }
        assert_eq!(
            session.space.npc(helper).map(|npc| npc.goal),
            Some(Goal::Drift),
            "a wimpy trader keeps its drift"
        );
    }
}
