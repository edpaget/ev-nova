//! `_HandlePlayer`'s class-order bookkeeping (@0x6e2e8-0x6e452 in `EV
//! Nova.app`'s `EV Nova`): the order kept for each escort class
//! ([`Escorts::class_orders`](super::space::Escorts), the original's
//! table at 0x3b5394) is cleared in flight once no ship of the class can
//! hold it ([`Space::settle_class_orders`]).
//!
//! **The mark.** A ship of the player's that takes a command whose order
//! differs from its own is marked ([`Escorts::commanded`](super::space::Escorts),
//! the original's +0xc4, `_IssueNewEscortCommand` @0x66183-0x66190; see
//! `Session::command_escorts`). A ship that isn't a carried fighter given
//! Return to Hangar is marked, though it keeps formation. Every placement,
//! respawn and launch starts unmarked (@0x3c6ff, @0x3d546, @0x3d9c2).
//!
//! **The bookkeeping** runs on every pass, both of double time's
//! (`_DoPlayGameWork` calls `_HandlePlayer` on both, @0x43e19), after the
//! escort menu's fade (@0x6df4a-0x6e052) and the escort keys
//! (@0x6e057-0x6e2e2), whether the menu shows or not. Neither loop tests
//! whether a ship is disabled or dying.
//!
//! 1. **Return to Hangar** (@0x6e31c-0x6e3b5): each class whose kept order
//!    is Return to Hangar, with no carried fighter (AI type 5) of it the
//!    player leads that is marked, goes to formation (@0x6e37d).
//! 2. **Classes with no marked ship** (@0x6e3bb-0x6e452): each class that
//!    keeps any order but formation, with no ship of it the player leads
//!    that is marked, goes to formation (@0x6e43f). The original's -1
//!    ("unset") counts as an order here, so a kept Return a lone launch
//!    turned to -1 (`_LaunchFighter` @0x3db4d) is cleared on the same pass;
//!    the engine has no -1, and the launch reports that clear itself (see
//!    the `fighters` module).
//!
//! `_EscortAI` copies the class's order into every ship of it the player
//! leads on each think (@0x83afb), so a clear sends every escort of the
//! class to formation, not just the order kept for joiners: each hired
//! escort's record and duty, and each mission escort's mark.
//!
//! **The menu.** While the escort menu shows, each clear calls
//! `_ShowEscortMenu` (@0x6e388, @0x6e446), which sets the menu's level to
//! 32 and draws it (@0x5664a-0x56658) with no stamp and no sound: a fade
//! under way starts again from 32, its timeout still running from the old
//! stamp. A fade that reached 0 earlier in the same `_HandlePlayer` has
//! shut the menu already (`jle` @0x6e386, @0x6e444). The step reports each
//! clear ([`StepRun::class_orders_cleared`](crate::flight_frame::StepRun::class_orders_cleared))
//! for the view to show the menu again. Its rows show each class's kept
//! order as the bookkeeping leaves it ([`Space::standing_class_order`]).
//!
//! **Ordering.** A command arrives between steps
//! (`Session::command_escorts`), where the original gives it in the same
//! `_HandlePlayer` as the bookkeeping (@0x6e192-0x6e2e2), so a class
//! cleared on the frame of its command (Return to Hangar given to the
//! warships, say) is cleared on the next step's pass. The frame filled
//! after the command shows the row in formation already, as the row reads
//! the kept order as the bookkeeping will leave it, so nothing crosses a
//! drawn frame.
//!
//! **Filed.**
//! - A ship `_GenericRandomShipSpawn` places keeps its berth's last mark
//!   (rdm:phase/play-feel-5-followups/phase-5-berth-keeps-command-mark).
//! - A jump's arrival frame skips the bookkeeping, and the menu fades on
//!   both of double time's passes
//!   (rdm:phase/play-feel-5-followups/phase-7-handle-player-double-fade-and-arrival-orders).

use super::{Space, Statics};
use crate::escort::{EscortClass, EscortOrder};
use crate::pilot::Pilot;
use crate::traffic::npc::{Npc, NpcId};

impl Space {
    /// Marks NPC `id` as having taken a command since it was placed (see
    /// the module docs).
    pub(super) fn mark_commanded(&mut self, id: NpcId) {
        self.escorts.commanded.insert(id);
    }

    /// Marks each ship of the player's in the system on entering it, a
    /// fleet escort placed or a mission's ship, whose class keeps an
    /// order, as having taken a command: the other reading of
    /// [`RuleKey::EscortOrders`](crate::rulebook::RuleKey::EscortOrders)
    /// keeps the orders through an entry, which the bookkeeping would
    /// otherwise clear on the next pass, as each ship placed is new and
    /// unmarked (see the module docs).
    pub(super) fn mark_kept_orders(&mut self) {
        let orders = self.escorts.class_orders;
        let keeps = |class: EscortClass| {
            EscortClass::ALL
                .iter()
                .zip(orders)
                .any(|(&each, order)| each == class && order.is_some())
        };
        let kept: Vec<NpcId> = self
            .traffic
            .npcs()
            .iter()
            .filter(|npc| self.escorts.fleet.contains(&Some(npc.id)) || npc.mission.is_some())
            .filter(|npc| keeps(npc.class))
            .map(|npc| npc.id)
            .collect();
        self.escorts.commanded.extend(kept);
    }

    /// `_HandlePlayer`'s class-order bookkeeping (see the module docs),
    /// the escorts' records in `pilot`, their classes read through
    /// `statics`: whether it cleared any class.
    pub(super) fn settle_class_orders(&mut self, pilot: &mut Pilot, statics: &Statics) -> bool {
        let mut cleared = Vec::new();
        for (index, class) in EscortClass::ALL.into_iter().enumerate() {
            // Both loops' test, @0x6e31c-0x6e3b5 and @0x6e3bb-0x6e452: a kept
            // Return with no marked ship has no marked carried fighter.
            let lapsed =
                self.escorts.class_orders[index].is_some_and(|order| self.lapses(class, order));
            if lapsed {
                self.escorts.class_orders[index] = None;
                cleared.push(class);
            }
        }
        for &class in &cleared {
            self.send_class_to_formation(pilot, statics, class);
        }
        !cleared.is_empty()
    }

    /// Whether `class`'s kept `order` falls to the bookkeeping: Return to
    /// Hangar with no marked carried fighter of the class out
    /// (@0x6e31c-0x6e3b5), any other order with no marked ship of it
    /// (@0x6e3bb-0x6e452).
    fn lapses(&self, class: EscortClass, order: EscortOrder) -> bool {
        !self.holds_class(class, |npc| {
            order != EscortOrder::Dock || npc.carrier.is_some()
        })
    }

    /// The order kept for `class`, read raw.
    pub(super) fn kept_for(&self, class: EscortClass) -> Option<EscortOrder> {
        EscortClass::ALL
            .iter()
            .position(|&each| each == class)
            .and_then(|index| self.escorts.class_orders[index])
    }

    /// The order `_DrawEscortMenuInFrame` shows for `class` (@0x56534): the
    /// one kept for it as the bookkeeping leaves it (see the module docs).
    /// None for a class no ship of which is out, as a kept order needs a
    /// marked ship.
    pub(super) fn standing_class_order(&self, class: EscortClass) -> Option<EscortOrder> {
        self.kept_for(class)
            .filter(|&order| !self.lapses(class, order))
    }

    /// A class keeping no order takes its first hired escort's: by the
    /// other reading of `RuleKey::EscortOrders`, the orders come back on
    /// the records of a pilot loaded, as `class_orders` isn't saved, where
    /// the original reloads its table (`_LoadPilotData` @0x760c2-0x760ea).
    /// A class with no ship placed takes none: no ship of it could hold the
    /// order, and the bookkeeping's clear would wipe its escorts' records.
    pub(super) fn keep_record_orders(&mut self, pilot: &Pilot, statics: &Statics) {
        for (index, class) in EscortClass::ALL.into_iter().enumerate() {
            let placed = self.player_led().any(|npc| npc.class == class);
            if placed && self.escorts.class_orders[index].is_none() {
                self.escorts.class_orders[index] = (0..pilot.escorts.len())
                    .find(|&escort| self.escort_class(pilot, statics, escort) == class)
                    .and_then(|escort| pilot.escorts[escort].order);
            }
        }
    }

    /// Whether a ship of `class` the player leads, marked as having taken a
    /// command and as `also` says, is out, disabled or dying too.
    fn holds_class(&self, class: EscortClass, also: impl Fn(&Npc) -> bool) -> bool {
        self.player_led()
            .any(|npc| npc.class == class && also(npc) && self.escorts.commanded.contains(&npc.id))
    }

    /// Every escort of `class` keeps formation, as each takes the class's
    /// cleared order on its next think (`_EscortAI` @0x83afb): each hired
    /// escort's record in `pilot` and its NPC's duty, its class read
    /// through `statics`, and each mission escort's mark.
    fn send_class_to_formation(
        &mut self,
        pilot: &mut Pilot,
        statics: &Statics,
        class: EscortClass,
    ) {
        let hired: Vec<usize> = (0..pilot.escorts.len())
            .filter(|&index| self.escort_class(pilot, statics, index) == class)
            .collect();
        for index in hired {
            pilot.escorts[index].order = None;
            let placed = self.escorts.fleet.get(index).copied().flatten();
            if let Some(duty) = placed
                .and_then(|id| self.npc_mut(id))
                .and_then(|npc| npc.escort.as_mut())
            {
                duty.order = None;
            }
        }
        for npc in self.traffic.npcs_mut() {
            if npc.class == class
                && let Some(mark) = npc.mission.as_mut().filter(|mark| mark.formation.is_some())
            {
                mark.order = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::chance::NeverFires;
    use crate::escort::{EscortClass, EscortCommand, EscortGroup, EscortOrder};
    use crate::flight::Controls;
    use crate::pilot::Escort;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::fighters::tests::{
        carrying, fleet_npcs, flying_by, launch, out, pilot, warship,
    };
    use crate::testkit::{FakePilotCatalog, work};
    use crate::traffic::npc::NpcId;

    /// `escorts` flying with two Vipers aboard, the bay selected,
    /// [`RuleKey::EscortOrders`] by `source`.
    fn flying(catalog: &FakePilotCatalog, escorts: Vec<Escort>, source: RuleSource) -> Session {
        let rules = Rulebook::default().with_override(RuleKey::EscortOrders, source);
        flying_by(catalog, pilot(catalog, 2, escorts), rules)
    }

    /// The fleet's NPCs' numbers, in fleet order.
    fn fleet_ids(session: &Session) -> Vec<NpcId> {
        fleet_npcs(session).iter().map(|npc| npc.id).collect()
    }

    /// The NPCs marked as having taken a command.
    fn marked(session: &Session) -> BTreeSet<NpcId> {
        session.space.escorts.commanded.clone()
    }

    /// Gives every escort of `group` `command`.
    fn command(session: &mut Session, group: EscortGroup, command: EscortCommand) {
        session.command_escorts(group, command, &mut NeverFires);
    }

    // The mark (`_IssueNewEscortCommand` @0x66183-0x66190).

    #[test]
    fn an_escort_given_an_order_it_does_not_hold_is_marked() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        assert_eq!(marked(&session), BTreeSet::new(), "placed unmarked");
        command(&mut session, EscortGroup::All, EscortCommand::Hold);
        assert_eq!(marked(&session), BTreeSet::from_iter(fleet_ids(&session)));
    }

    #[test]
    fn an_escort_given_the_order_it_holds_is_not_marked() {
        let catalog = carrying();
        let mut session = flying(
            &catalog,
            vec![warship(Some(EscortOrder::Hold))],
            RuleSource::Bible,
        );
        session.space.escorts.commanded.clear();
        command(&mut session, EscortGroup::All, EscortCommand::Hold);
        assert_eq!(marked(&session), BTreeSet::new());
    }

    #[test]
    fn an_escort_that_is_no_carried_fighter_given_return_to_hangar_is_marked() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        command(&mut session, EscortGroup::All, EscortCommand::Dock);
        assert_eq!(session.pilot().escorts()[0].order, None, "in formation");
        assert_eq!(marked(&session), BTreeSet::from_iter(fleet_ids(&session)));
    }

    #[test]
    fn a_carried_fighter_already_returning_given_return_to_hangar_is_not_marked() {
        let catalog = carrying();
        let returning = Escort {
            order: Some(EscortOrder::Dock),
            ..out()
        };
        let mut session = flying(&catalog, vec![returning], RuleSource::Bible);
        session.space.escorts.commanded.clear();
        command(&mut session, EscortGroup::All, EscortCommand::Dock);
        assert_eq!(marked(&session), BTreeSet::new());
        command(&mut session, EscortGroup::All, EscortCommand::Hold);
        assert_eq!(marked(&session), BTreeSet::from_iter(fleet_ids(&session)));
    }

    #[test]
    fn a_mission_escort_given_a_new_order_is_marked() {
        use crate::session::mission_escorts::tests::{escorting, flying};
        use crate::session::mission_ships::tests::ships_catalog;
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        let escorts: BTreeSet<NpcId> = session.player_led().map(|npc| npc.id).collect();
        assert_eq!(escorts.len(), 2);
        command(&mut session, EscortGroup::All, EscortCommand::Defend);
        assert_eq!(marked(&session), escorts);
    }

    #[test]
    fn a_mission_escort_given_the_order_it_holds_is_not_marked() {
        use crate::session::mission_escorts::tests::{escorting, flying};
        use crate::session::mission_ships::tests::ships_catalog;
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        command(&mut session, EscortGroup::All, EscortCommand::Defend);
        session.space.escorts.commanded.clear();
        command(&mut session, EscortGroup::All, EscortCommand::Defend);
        assert_eq!(marked(&session), BTreeSet::new());
    }

    #[test]
    fn an_escort_placed_again_on_entering_a_system_starts_unmarked() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        command(&mut session, EscortGroup::All, EscortCommand::Hold);
        let before = fleet_ids(&session);
        session.populate(&catalog, &mut NeverFires);
        let after = fleet_ids(&session);
        assert_ne!(after, before, "placed anew");
        assert!(marked(&session).is_disjoint(&BTreeSet::from_iter(after)));
    }

    #[test]
    fn a_fighter_launched_after_a_command_starts_unmarked() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        command(&mut session, EscortGroup::All, EscortCommand::Hold);
        launch(&mut session, &catalog);
        let ids = fleet_ids(&session);
        assert_eq!(ids.len(), 2, "launched");
        assert!(marked(&session).contains(&ids[0]), "the warship");
        assert!(!marked(&session).contains(&ids[1]), "the fighter");
        assert_eq!(fleet_npcs(&session)[1].class, EscortClass::Fighter);
    }

    // The bookkeeping (`_HandlePlayer` @0x6e2e8-0x6e452).

    /// The bookkeeping run once on `session`: whether it cleared a class.
    fn settle(session: &mut Session) -> bool {
        session
            .space
            .settle_class_orders(&mut session.pilot, &session.statics)
    }

    /// The order kept for `class`.
    fn kept(session: &Session, class: EscortClass) -> Option<EscortOrder> {
        let index = EscortClass::ALL
            .iter()
            .position(|&each| each == class)
            .expect("a class");
        session.space.escorts.class_orders[index]
    }

    /// The order kept for each class, in the menu's order.
    fn kept_orders(session: &Session) -> [Option<EscortOrder>; 4] {
        session.space.escorts.class_orders
    }

    const WARSHIPS: EscortGroup = EscortGroup::Class(EscortClass::Warship);
    const FIGHTERS: EscortGroup = EscortGroup::Class(EscortClass::Fighter);

    #[test]
    fn return_to_hangar_given_a_class_with_no_carried_fighter_goes_to_formation() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        command(&mut session, WARSHIPS, EscortCommand::Dock);
        assert_eq!(
            kept(&session, EscortClass::Warship),
            Some(EscortOrder::Dock)
        );
        assert!(settle(&mut session), "cleared (@0x6e37d)");
        assert_eq!(kept_orders(&session), [None; 4]);
        assert!(!settle(&mut session), "nothing left to clear");
    }

    #[test]
    fn return_to_hangar_stands_while_a_marked_carried_fighter_of_the_class_is_out() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![out()], RuleSource::Engine);
        command(&mut session, FIGHTERS, EscortCommand::Dock);
        assert!(!settle(&mut session));
        assert_eq!(
            kept(&session, EscortClass::Fighter),
            Some(EscortOrder::Dock)
        );
        assert_eq!(session.pilot().escorts()[0].order, Some(EscortOrder::Dock));
        session.drop_fleet_escort(0);
        assert!(settle(&mut session), "docked: none left out");
        assert_eq!(kept(&session, EscortClass::Fighter), None);
    }

    #[test]
    fn return_to_hangar_needs_a_carried_fighter_that_is_marked() {
        let catalog = carrying();
        let uncarried = Escort {
            carried: false,
            ..out()
        };
        let mut session = flying(&catalog, vec![uncarried], RuleSource::Engine);
        command(&mut session, FIGHTERS, EscortCommand::Dock);
        assert_eq!(fleet_npcs(&session)[0].class, EscortClass::Fighter);
        assert!(!marked(&session).is_empty(), "marked though in formation");
        assert!(settle(&mut session), "a fighter but not a carried one");
        assert_eq!(kept(&session, EscortClass::Fighter), None);
        let mut session = flying(&catalog, vec![out()], RuleSource::Engine);
        command(&mut session, FIGHTERS, EscortCommand::Dock);
        session.space.escorts.commanded.clear();
        assert!(settle(&mut session), "carried but unmarked");
        assert_eq!(kept(&session, EscortClass::Fighter), None);
        assert_eq!(
            session.pilot().escorts()[0].order,
            None,
            "sent to formation"
        );
    }

    #[test]
    fn return_to_hangar_stands_only_for_the_class_of_its_carried_fighter() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![out()], RuleSource::Engine);
        command(&mut session, EscortGroup::All, EscortCommand::Dock);
        assert!(settle(&mut session));
        assert_eq!(
            kept_orders(&session),
            [Some(EscortOrder::Dock), None, None, None]
        );
    }

    #[test]
    fn an_order_stands_only_for_a_class_with_a_marked_ship() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        command(&mut session, EscortGroup::All, EscortCommand::Hold);
        assert_eq!(kept_orders(&session), [Some(EscortOrder::Hold); 4]);
        assert!(settle(&mut session), "the classes with no ship (@0x6e43f)");
        assert_eq!(
            kept_orders(&session),
            [None, None, Some(EscortOrder::Hold), None]
        );
        assert_eq!(session.pilot().escorts()[0].order, Some(EscortOrder::Hold));
        assert!(!settle(&mut session), "it stands");
    }

    #[test]
    fn a_class_whose_marked_ships_are_gone_goes_to_formation_with_every_escort_of_it() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        command(&mut session, WARSHIPS, EscortCommand::Attack);
        // A warship joins the class unmarked, holding its order, as a
        // ship placed with it does; and so does a mission escort.
        session
            .pilot
            .escorts
            .push(warship(Some(EscortOrder::Attack)));
        let joiner = session.place_escort(1, &mut NeverFires).expect("placed");
        session.space.escorts.fleet.push(Some(joiner));
        let class = fleet_npcs(&session)[1].class;
        assert_eq!(class, EscortClass::Warship);
        let mut escort = crate::testkit::npc(0, crate::stats::ShipStats::default());
        escort.class = class;
        escort.mission = Some(crate::traffic::npc::MissionMark {
            slot: 0,
            behav: crate::mission::ships::ShipBehav::EscortPlayer,
            formation: Some(crate::escort::FormationSlot::default()),
            order: Some(EscortOrder::Attack),
        });
        let mission = session.space.traffic.add_npc(escort);
        assert!(!settle(&mut session), "the marked one is out");
        session.drop_fleet_escort(0);
        assert!(settle(&mut session), "none marked is left");
        assert_eq!(kept_orders(&session), [None; 4]);
        assert_eq!(session.pilot().escorts()[0].order, None, "its record");
        let duty = fleet_npcs(&session)[0].escort.expect("an escort");
        assert_eq!(duty.order, None, "its duty");
        let mark = session.space.npc(mission).and_then(|npc| npc.mission);
        assert_eq!(mark.and_then(|mark| mark.order), None, "the mission escort");
    }

    #[test]
    fn a_class_cleared_leaves_the_other_classes_escorts_alone() {
        let catalog = carrying();
        let uncarried = Escort {
            carried: false,
            order: Some(EscortOrder::Hold),
            ..out()
        };
        let mut session = flying(&catalog, vec![warship(None), uncarried], RuleSource::Bible);
        command(&mut session, WARSHIPS, EscortCommand::Hold);
        let warship = fleet_ids(&session)[0];
        session.space.escorts.commanded.remove(&warship);
        assert!(settle(&mut session));
        let orders: Vec<_> = session.pilot().escorts().iter().map(|e| e.order).collect();
        assert_eq!(orders, [None, Some(EscortOrder::Hold)]);
    }

    #[test]
    fn a_class_that_keeps_formation_is_never_cleared() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        assert!(!settle(&mut session), "no order kept");
        command(&mut session, EscortGroup::All, EscortCommand::Hold);
        command(&mut session, EscortGroup::All, EscortCommand::Recall);
        session.space.escorts.commanded.clear();
        assert!(!settle(&mut session), "recalled: formation");
    }

    #[test]
    fn a_class_both_loops_clear_is_cleared_once() {
        let catalog = carrying();
        let mut session = flying(&catalog, Vec::new(), RuleSource::Engine);
        command(&mut session, WARSHIPS, EscortCommand::Dock);
        assert!(settle(&mut session));
        assert_eq!(kept_orders(&session), [None; 4]);
    }

    #[test]
    fn a_disabled_marked_escort_still_holds_its_classs_order() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        command(&mut session, WARSHIPS, EscortCommand::Hold);
        let id = fleet_ids(&session)[0];
        session.space.npc_mut(id).expect("there").condition =
            crate::combat::hull::Condition::Disabled;
        assert!(!settle(&mut session), "neither loop tests it");
        assert_eq!(
            kept(&session, EscortClass::Warship),
            Some(EscortOrder::Hold)
        );
    }

    // The menu's rows (`_DrawEscortMenuInFrame` @0x5637f-0x565ac).

    /// The menu row of `class`.
    fn row(session: &Session, class: EscortClass) -> crate::escort::ClassRow {
        session
            .space
            .escort_rows()
            .into_iter()
            .find(|row| row.class == class)
            .expect("every class has a row")
    }

    /// A fighter that is no carried one.
    fn uncarried() -> Escort {
        Escort {
            carried: false,
            ..out()
        }
    }

    #[test]
    fn a_mixed_class_reads_return_to_hangar_while_its_marked_carried_fighter_is_out() {
        let catalog = carrying();
        let escorts = vec![uncarried(), out()];
        let mut session = flying(&catalog, escorts, RuleSource::Engine);
        command(&mut session, FIGHTERS, EscortCommand::Dock);
        let fighters = row(&session, EscortClass::Fighter);
        assert_eq!(fighters.order, Some(EscortOrder::Dock));
        assert_eq!(session.pilot().escorts()[0].order, None, "in formation");
        session.drop_fleet_escort(1);
        settle(&mut session);
        assert_eq!(row(&session, EscortClass::Fighter).order, None);
        assert_eq!(kept(&session, EscortClass::Fighter), None);
    }

    #[test]
    fn a_carried_fighter_that_is_unmarked_holds_no_row_order() {
        let catalog = carrying();
        let escorts = vec![uncarried(), out()];
        let mut session = flying(&catalog, escorts, RuleSource::Engine);
        command(&mut session, FIGHTERS, EscortCommand::Dock);
        session.space.escorts.commanded.clear();
        assert_eq!(row(&session, EscortClass::Fighter).order, None);
    }

    #[test]
    fn a_disabled_marked_carried_fighter_keeps_its_row_order() {
        let catalog = carrying();
        let escorts = vec![uncarried(), out()];
        let mut session = flying(&catalog, escorts, RuleSource::Engine);
        command(&mut session, FIGHTERS, EscortCommand::Dock);
        let id = fleet_ids(&session)[1];
        session.space.npc_mut(id).expect("there").condition =
            crate::combat::hull::Condition::Disabled;
        assert_eq!(
            row(&session, EscortClass::Fighter).order,
            Some(EscortOrder::Dock)
        );
    }

    #[test]
    fn a_row_never_shows_an_order_the_same_pass_clears() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        command(&mut session, WARSHIPS, EscortCommand::Dock);
        assert_eq!(
            kept(&session, EscortClass::Warship),
            Some(EscortOrder::Dock)
        );
        assert_eq!(row(&session, EscortClass::Warship).order, None);
        command(&mut session, EscortGroup::All, EscortCommand::Hold);
        assert_eq!(kept_orders(&session), [Some(EscortOrder::Hold); 4]);
        for class in EscortClass::ALL {
            let expected = (class == EscortClass::Warship).then_some(EscortOrder::Hold);
            assert_eq!(row(&session, class).order, expected, "{class:?}");
        }
    }

    #[test]
    fn a_record_with_no_ship_placed_shows_no_row() {
        let catalog = carrying();
        let mut session = flying(&catalog, Vec::new(), RuleSource::Engine);
        session.pilot.escorts.push(warship(None));
        for class in EscortClass::ALL {
            assert!(!row(&session, class).present, "{class:?}");
        }
    }

    #[test]
    fn a_dying_escort_keeps_its_row() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        let id = fleet_ids(&session)[0];
        session.space.npc_mut(id).expect("there").condition =
            crate::combat::hull::Condition::Dying {
                ticks_left: 10,
                starting: false,
            };
        assert!(row(&session, EscortClass::Warship).present);
    }

    #[test]
    fn by_the_other_reading_a_record_seeds_its_classs_kept_order() {
        let catalog = carrying();
        let hold = Some(EscortOrder::Hold);
        let escorts = vec![warship(hold), warship(Some(EscortOrder::Attack))];
        let session = flying(&catalog, escorts.clone(), RuleSource::Bible);
        assert_eq!(kept(&session, EscortClass::Warship), hold);
        assert_eq!(row(&session, EscortClass::Warship).order, hold);
        assert_eq!(marked(&session), BTreeSet::from_iter(fleet_ids(&session)));
        let mut seeded = session;
        assert!(!settle(&mut seeded));
        let orders: Vec<_> = seeded.pilot().escorts().iter().map(|e| e.order).collect();
        assert_eq!(orders, [hold, Some(EscortOrder::Attack)]);
        let session = flying(&catalog, escorts, RuleSource::Engine);
        assert_eq!(kept(&session, EscortClass::Warship), None);
    }

    // On each pass (`_DoPlayGameWork` @0x43e19).

    #[test]
    fn the_advance_pass_clears_a_class_and_reports_it() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        command(&mut session, WARSHIPS, EscortCommand::Dock);
        assert!(!session.out.class_orders_cleared, "not by the command");
        session.take_sounds();
        work(&mut session, &catalog, Controls::default());
        assert_eq!(kept(&session, EscortClass::Warship), None);
        assert!(session.out.class_orders_cleared);
        assert!(session.take_sounds().is_empty(), "silent");
        session.out.class_orders_cleared = false;
        work(&mut session, &catalog, Controls::default());
        assert!(!session.out.class_orders_cleared, "nothing more to clear");
    }

    #[test]
    fn double_times_second_pass_clears_a_class_and_reports_it() {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], RuleSource::Engine);
        work(&mut session, &catalog, Controls::default());
        command(&mut session, WARSHIPS, EscortCommand::Dock);
        session.tick_again(
            Controls::default(),
            &crate::testkit::Passive,
            crate::combat::Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(kept(&session, EscortClass::Warship), None);
        assert!(session.out.class_orders_cleared);
    }

    // Entering a system, by `RuleKey::EscortOrders`.

    /// The order kept for the warships and the warship's own after Hold
    /// is given the warships, the system entered again and a pass run, by
    /// `source`; and whether the warship placed anew is marked.
    fn held_through_an_entry(
        source: RuleSource,
    ) -> (Option<EscortOrder>, Option<EscortOrder>, bool) {
        let catalog = carrying();
        let mut session = flying(&catalog, vec![warship(None)], source);
        command(&mut session, WARSHIPS, EscortCommand::Hold);
        session.populate(&catalog, &mut NeverFires);
        let placed = fleet_ids(&session)[0];
        let marked = marked(&session).contains(&placed);
        work(&mut session, &catalog, Controls::default());
        let own = session.pilot().escorts()[0].order;
        (kept(&session, EscortClass::Warship), own, marked)
    }

    #[test]
    fn by_the_engine_an_entry_resets_the_orders_and_by_the_other_reading_they_stand() {
        assert_eq!(
            held_through_an_entry(RuleSource::Engine),
            (None, None, false)
        );
        let hold = Some(EscortOrder::Hold);
        assert_eq!(held_through_an_entry(RuleSource::Bible), (hold, hold, true));
    }

    #[test]
    fn by_the_other_reading_a_kept_order_wins_over_the_record_seed_through_an_entry() {
        let catalog = carrying();
        let escorts = vec![uncarried(), out()];
        let mut session = flying(&catalog, escorts, RuleSource::Bible);
        command(&mut session, FIGHTERS, EscortCommand::Dock);
        assert_eq!(session.pilot().escorts()[0].order, None, "in formation");
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(
            kept(&session, EscortClass::Fighter),
            Some(EscortOrder::Dock)
        );
    }

    #[test]
    fn by_the_other_reading_a_mission_escorts_order_stands_through_an_entry() {
        use crate::session::mission_escorts::tests::escorting;
        use crate::session::mission_ships::tests::{accepted, ships_catalog, take_off, tick};
        for (source, kept) in [
            (RuleSource::Engine, None),
            (RuleSource::Bible, Some(EscortOrder::Hold)),
        ] {
            let catalog = ships_catalog(escorting(-1, 0));
            let rules = Rulebook::default().with_override(RuleKey::EscortOrders, source);
            let mut session = accepted(&catalog).with_rules(rules);
            take_off(&mut session, &catalog);
            command(&mut session, EscortGroup::All, EscortCommand::Hold);
            crate::testkit::land_now(&mut session).expect("lands");
            take_off(&mut session, &catalog);
            tick(&mut session, &catalog);
            let orders: Vec<_> = session
                .player_led()
                .map(crate::traffic::npc::Npc::escort_order)
                .collect();
            assert_eq!(orders, [kept; 2], "{source:?}");
            let class = session.player_led().next().expect("escorting").class;
            assert_eq!(row(&session, class).order, kept, "{source:?}");
        }
    }
}
