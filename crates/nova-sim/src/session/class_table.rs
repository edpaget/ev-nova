//! The class-order table as the save sees it
//! ([`Pilot::class_orders`](crate::pilot::Pilot), the original's table at
//! 0x3b5394, saved raw at pilot +0x5d90 + 2i by `_WritePilotData`
//! @0x72b01-0x72b25), and the words a loaded pilot's escorts take from it.
//!
//! **In flight** the engine keeps the table on
//! [`Escorts::class_orders`](super::space::Escorts) as orders, with no -1
//! (see the [`class_orders`](super::class_orders) module). The pilot's copy
//! holds the raw words, written where the original writes the table:
//! - a command writes the entries of the classes it reaches, and only
//!   those (`_HandlePlayer` @0x6e23e-0x6e26b, [`record_command`]), so a
//!   pilot who hasn't flown keeps -1 on the others until the pass;
//! - each pass's bookkeeping writes all four
//!   ([`Space::record_class_orders`]): its loop 2 clears every class with
//!   no marked ship, -1 included, to 0 (@0x6e43f), and every ship placed or
//!   loaded starts unmarked while a command writes its class, so no class
//!   holds -1 after a pass.
//!
//! Two writes of the in-flight table aren't copied, as the next pass
//! overwrites them before any save: the engine reading's reset at an entry
//! ([`RuleKey::EscortOrders`](crate::rulebook::RuleKey::EscortOrders),
//! which the original clears on the next pass by loop 2), and a lone
//! launch's -1 (`_LaunchFighter` @0x3db3e-0x3db4d), which loop 2 clears on
//! the same pass.
//!
//! **A load** seeds the in-flight table from the saved one, -1 and 0 as no
//! order (`_LoadPilotData` @0x760c2-0x760ea, [`Space::seed_class_orders`]),
//! and each escort placed with no NPC, a loaded pilot's, takes its class's
//! saved word raw ([`table_word`], @0x760ec-0x76147): -1 for a pilot who
//! hasn't flown and 3 on a ship that is no fighter under a saved Return,
//! which only its first think (`_EscortAI` @0x83ae4-0x83b2b) turns to 0. A
//! command before that think compares with the table's word (see the
//! [`command_word`](super::command_word) module).

use super::Space;
use crate::escort::{EscortClass, EscortCommand, EscortGroup};
use crate::fleet::OrderWord;
use crate::pilot::Pilot;

/// The index of `class` in the table, [`EscortClass::ALL`] order.
fn index_of(class: EscortClass) -> usize {
    EscortClass::ALL
        .iter()
        .position(|&each| each == class)
        .unwrap_or_default()
}

/// The word an escort of `class` placed with no NPC takes from `pilot`'s
/// table, raw (`_LoadPilotData` @0x760ec-0x76147).
pub(super) fn table_word(pilot: &Pilot, class: EscortClass) -> OrderWord {
    pilot.class_orders[index_of(class)]
}

/// `command` given `group` writes its word into the entry of each class
/// the group reaches in `pilot`'s table, as `_HandlePlayer` writes the
/// table (@0x6e23e-0x6e26b).
pub(super) fn record_command(pilot: &mut Pilot, group: EscortGroup, command: EscortCommand) {
    for (class, word) in EscortClass::ALL.into_iter().zip(&mut pilot.class_orders) {
        if group.holds(class) {
            *word = OrderWord::of(command.order());
        }
    }
}

impl Space {
    /// The in-flight table seeded from `pilot`'s saved one on a load, a
    /// word of -1 or 0 as no order (@0x760c2-0x760ea).
    pub(super) fn seed_class_orders(&mut self, pilot: &Pilot) {
        self.escorts.class_orders = pilot.class_orders.map(OrderWord::order);
    }

    /// `pilot`'s table written from the in-flight one, as each pass leaves
    /// it (see the module docs).
    pub(super) fn record_class_orders(&self, pilot: &mut Pilot) {
        pilot.class_orders = self.escorts.class_orders.map(OrderWord::of);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::escort::{Commanded, EscortOrder};
    use crate::flight::Controls;
    use crate::pilot::Escort;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::save::{decode, encode};
    use crate::session::Session;
    use crate::session::fighters::tests::{carrying, fleet_npcs, flying_by, out, pilot, warship};
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, land_now, work};

    const WARSHIPS: EscortGroup = EscortGroup::Class(EscortClass::Warship);

    /// Words in class order.
    fn words(words: [i16; 4]) -> [OrderWord; 4] {
        words.map(OrderWord)
    }

    /// `escorts` loaded with the saved table `table`, flown by `source`,
    /// the system set up, the sounds taken.
    fn loaded(
        catalog: &FakePilotCatalog,
        escorts: Vec<Escort>,
        table: [i16; 4],
        source: RuleSource,
    ) -> Session {
        let mut pilot = pilot(catalog, 0, escorts);
        pilot.class_orders = words(table);
        let rules = Rulebook::default().with_override(RuleKey::EscortOrders, source);
        let mut session = flying_by(catalog, pilot, rules);
        session.take_sounds();
        session
    }

    fn word(session: &Session, index: usize) -> OrderWord {
        fleet_npcs(session)[index].order_word
    }

    fn command(
        session: &mut Session,
        group: EscortGroup,
        given: EscortCommand,
    ) -> Option<Commanded> {
        session.command_escorts(group, given, &mut NeverFires)
    }

    #[test]
    fn a_saved_return_loads_as_word_3_on_a_ship_that_is_no_fighter() {
        let catalog = carrying();
        let uncarried = Escort {
            carried: false,
            ..out()
        };
        let docking = Escort {
            order: Some(EscortOrder::Dock),
            ..out()
        };
        let mut session = loaded(
            &catalog,
            vec![docking, uncarried],
            [3, 0, 0, 0],
            RuleSource::Engine,
        );
        assert_eq!(word(&session, 0), OrderWord(3), "the carried fighter");
        assert_eq!(word(&session, 1), OrderWord(3), "raw, though no fighter");
        // Return equals word 3 on both, so it changes and marks nothing
        // (`je` @0x6618a, ahead of the fighter test); on word 0 the ship
        // that is no fighter was marked (@0x66190).
        assert_eq!(
            command(&mut session, EscortGroup::All, EscortCommand::Dock),
            None
        );
        assert_eq!(session.take_sounds(), []);
        assert!(session.space.escorts.commanded.is_empty(), "unmarked");
        // A Recall differs from 3, where word 0 would have kept it silent.
        let told =
            command(&mut session, EscortGroup::All, EscortCommand::Recall).expect("a change");
        assert_eq!(told.command, EscortCommand::Recall);
        assert_eq!(session.take_sounds(), [SimSound::EscortsCommanded]);
        assert_eq!(word(&session, 0), OrderWord(0));
        assert_eq!(word(&session, 1), OrderWord(0));
    }

    #[test]
    fn a_pilot_who_has_not_flown_loads_its_escorts_on_word_minus_1() {
        let catalog = carrying();
        let mut session = loaded(&catalog, vec![warship(None)], [-1; 4], RuleSource::Engine);
        assert_eq!(word(&session, 0), OrderWord::NONE);
        let told = command(&mut session, EscortGroup::All, EscortCommand::Dock).expect("a change");
        assert_eq!(told.command, EscortCommand::Recall);
        assert_eq!(session.take_sounds(), [SimSound::EscortsCommanded]);
    }

    #[test]
    fn a_loaded_escort_takes_the_tables_word_not_its_records_by_either_reading() {
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let catalog = carrying();
            let session = loaded(
                &catalog,
                vec![warship(Some(EscortOrder::Attack))],
                [0, 0, 4, 0],
                source,
            );
            assert_eq!(word(&session, 0), OrderWord(4), "{source:?}");
        }
    }

    #[test]
    fn the_table_comes_back_through_a_save_and_a_load() {
        let catalog = carrying();
        let mut session = loaded(&catalog, vec![warship(None)], [-1; 4], RuleSource::Engine);
        command(&mut session, WARSHIPS, EscortCommand::Hold);
        work(&mut session, &catalog, Controls::default());
        land_now(&mut session).expect("lands");
        let text = encode(session.pilot());
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(value["class_orders"], serde_json::json!([0, 0, 4, 0]));
        let pilot = decode(&text).expect("loads");
        let session = flying_by(&catalog, pilot, Rulebook::default());
        assert_eq!(word(&session, 0), OrderWord(4), "before any think");
    }

    #[test]
    fn a_command_writes_only_its_classes_and_a_pass_writes_all_four() {
        let catalog = carrying();
        let mut session = loaded(&catalog, vec![warship(None)], [-1; 4], RuleSource::Engine);
        command(&mut session, WARSHIPS, EscortCommand::Hold);
        assert_eq!(session.pilot().class_orders, words([-1, -1, 4, -1]));
        session
            .space
            .settle_class_orders(&mut session.pilot, &session.statics);
        assert_eq!(session.pilot().class_orders, words([0, 0, 4, 0]));
    }

    #[test]
    fn a_command_to_all_writes_every_entry_and_a_recall_writes_0() {
        let catalog = carrying();
        let mut session = loaded(&catalog, vec![warship(None)], [-1; 4], RuleSource::Engine);
        command(&mut session, EscortGroup::All, EscortCommand::Dock);
        assert_eq!(session.pilot().class_orders, words([3; 4]));
        command(&mut session, EscortGroup::All, EscortCommand::Recall);
        assert_eq!(session.pilot().class_orders, words([0; 4]));
    }

    #[test]
    fn the_engine_readings_entry_reset_leaves_the_saved_table_until_the_next_pass() {
        let catalog = carrying();
        let mut session = loaded(&catalog, vec![warship(None)], [-1; 4], RuleSource::Engine);
        command(&mut session, WARSHIPS, EscortCommand::Hold);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.pilot().class_orders, words([0, 0, 4, 0]));
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(session.space.escorts.class_orders, [None; 4], "reset");
        assert_eq!(session.pilot().class_orders, words([0, 0, 4, 0]));
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.pilot().class_orders, words([0; 4]));
    }

    #[test]
    fn by_the_other_reading_a_loaded_tables_order_stands_through_the_first_pass() {
        let hold = Some(EscortOrder::Hold);
        let catalog = carrying();
        let mut session = loaded(
            &catalog,
            vec![warship(hold)],
            [-1, 0, 4, 3],
            RuleSource::Bible,
        );
        assert_eq!(
            session.space.escorts.class_orders,
            [None, None, hold, Some(EscortOrder::Dock)],
            "-1 and 0 as no order"
        );
        let id = fleet_npcs(&session)[0].id;
        assert!(session.space.escorts.commanded.contains(&id), "marked");
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.space.escorts.class_orders[2], hold);
        assert_eq!(session.pilot().class_orders, words([0, 0, 4, 0]));
        let session = loaded(
            &catalog,
            vec![warship(hold)],
            [0, 0, 4, 0],
            RuleSource::Engine,
        );
        assert_eq!(session.space.escorts.class_orders, [None; 4], "reset");
    }

    #[test]
    fn the_first_pass_after_a_load_sets_the_system_up_before_the_bookkeeping_reads_it() {
        // `_LoadPilotData` spawns the escorts before any pass; the engine
        // places them on the first pass's traffic tick, after the
        // bookkeeping's turn, which waits for them.
        let hold = Some(EscortOrder::Hold);
        let catalog = carrying();
        let mut saved = pilot(&catalog, 0, vec![warship(hold)]);
        saved.class_orders = words([-1, 0, 4, 3]);
        let rules = Rulebook::default().with_override(RuleKey::EscortOrders, RuleSource::Bible);
        let mut session = Session::fly(&catalog, saved)
            .expect("flies")
            .with_rules(rules);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(fleet_npcs(&session).len(), 1, "placed");
        assert_eq!(word(&session, 0), OrderWord(4));
        assert_eq!(session.space.escorts.class_orders[2], hold, "not cleared");
        assert_eq!(session.pilot().class_orders, words([-1, 0, 4, 3]));
        assert_eq!(session.pilot().escorts()[0].order, hold);
    }

    #[test]
    fn a_class_out_of_the_table_reads_its_own_entry() {
        let mut pilot = pilot(&carrying(), 0, Vec::new());
        pilot.class_orders = words([1, 2, 3, 4]);
        let read: Vec<i16> = EscortClass::ALL
            .into_iter()
            .map(|class| table_word(&pilot, class).0)
            .collect();
        assert_eq!(read, [1, 2, 3, 4]);
    }
}
