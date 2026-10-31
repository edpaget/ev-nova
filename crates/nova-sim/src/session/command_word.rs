//! What the player's command reads and writes of each ship it reaches
//! (`_IssueNewEscortCommand` @0x66183-0x6622c in `EV Nova.app`'s `EV Nova`):
//! the ship's order word ([`Npc::order_word`](crate::Npc::order_word),
//! +0xc90a), not its standing order.
//!
//! **The word.** It's what the ship's last think left
//! (`nova_sim_ai`'s `think_word`), or what the last command wrote. So a
//! command equal to a word the out-of-ammunition fallback wrote (3 for a
//! fighter, 0 for any other ship) changes nothing for that ship, and the
//! ship's old standing order given again differs. A ship that doesn't think
//! (gliding in, in its gate, on the jump path, disabled) holds its word for
//! as long as that lasts, and a ship that hasn't thought holds -1, which
//! differs from every command ([`word_change`]).
//!
//! **Its table** (@0x66183-0x661ed):
//!
//! | Ship | Command | Word before | Word after | A change, and voice | Message |
//! |---|---|---|---|---|---|
//! | any | the word's own | equal | kept | no, nor marked | as given |
//! | fighter | any other | differs | the command | yes | as given |
//! | not a fighter | not Return | differs | the command | yes | as given |
//! | not a fighter | Return | 0 | 0 (@0x661ab) | no, but marked | as given |
//! | not a fighter | Return | anything else | 0 (@0x661b5) | yes | Recall (@0x661d4) |
//!
//! Past this comparison the target block (@0x66233) and the returning
//! fighter's (@0x6627a) run on every ship (`obey` in the `escorts` module),
//! so a ship skipped here still takes a target. `_HandlePlayer` writes the
//! class table first (@0x6e23e-0x6e26b), so the class takes the command
//! whether or not any ship changes.
//!
//! **The standing order** ([`standing`]). The engine's stand-in for the
//! class table as `_EscortAI` filters it. Return stands only on a fighter
//! with +0xc4 set, as every think turns it to 0 otherwise
//! (@0x83b18-0x83b2b), so a fighter the fallback sent home that has taken no
//! command and is given Return is skipped here, stays unmarked, and flies
//! formation from its next think: it doesn't dock, and the fallback, which
//! reads words 1 and 2 alone, doesn't put it back. The original's table
//! holds 3 until `_HandlePlayer`'s loop 1 clears it the same frame
//! (@0x6e31c-0x6e3b5; see the `class_orders` module).
//!
//! No sound of its own: `_beepSnd[1]` (@0x665e1) and the voice follow
//! [`WordChange::reordered`].

use crate::escort::{EscortCommand, EscortOrder};
use crate::fleet::OrderWord;

/// What a command does to one ship's word (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WordChange {
    /// The word the ship holds after it.
    pub word: OrderWord,
    /// Whether it sets the ship's +0xc4 (@0x66190).
    pub marks: bool,
    /// Whether it counts as a change, and for the voice (@0x661cd,
    /// @0x66225-0x6622c).
    pub reordered: bool,
    /// Whether the message's word turns to Recall (@0x661d4).
    pub recalled: bool,
}

/// `command` given a ship holding `word`, a fighter (AI type 5) as `fighter`
/// says: `_IssueNewEscortCommand` @0x66183-0x6622c. It compares raw, so -1
/// and a word a stray beam stepped differ from every command.
pub(super) fn word_change(word: OrderWord, command: EscortCommand, fighter: bool) -> WordChange {
    let given = OrderWord::of(command.order());
    if given == word {
        return WordChange {
            word,
            marks: false,
            reordered: false,
            recalled: false,
        };
    }
    if command == EscortCommand::Dock && !fighter {
        let recalled = word != OrderWord(0);
        return WordChange {
            word: OrderWord(0),
            marks: true,
            reordered: recalled,
            recalled,
        };
    }
    WordChange {
        word: given,
        marks: true,
        reordered: true,
        recalled: false,
    }
}

/// The standing order a ship keeps after `command`: Return stands only on a
/// fighter with +0xc4 set (`marked`), as every think then turns it to 0
/// (@0x83b18-0x83b2b).
pub(super) fn standing(command: EscortCommand, fighter: bool, marked: bool) -> Option<EscortOrder> {
    command
        .order()
        .filter(|&order| order != EscortOrder::Dock || (fighter && marked))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::chance::NeverFires;
    use crate::escort::{Commanded, EscortGroup};
    use crate::flight::Controls;
    use crate::pilot::Escort;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::fighters::tests::{carrying, fleet_npcs, flying_by, out, pilot, warship};
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, land_now, work};
    use crate::traffic::npc::NpcId;

    const DEFEND: Option<EscortOrder> = Some(EscortOrder::Defend);
    const DOCK: Option<EscortOrder> = Some(EscortOrder::Dock);

    const COMMANDS: [EscortCommand; 5] = [
        EscortCommand::Attack,
        EscortCommand::Defend,
        EscortCommand::Hold,
        EscortCommand::Recall,
        EscortCommand::Dock,
    ];

    /// A change to `word` that counts, and marks.
    fn changed_to(word: i16) -> WordChange {
        WordChange {
            word: OrderWord(word),
            marks: true,
            reordered: true,
            recalled: false,
        }
    }

    /// No change, its word `word`.
    fn skipped(word: i16) -> WordChange {
        WordChange {
            word: OrderWord(word),
            marks: false,
            reordered: false,
            recalled: false,
        }
    }

    #[test]
    fn a_command_equal_to_the_word_changes_nothing_on_any_ship() {
        // `je 0x66233` @0x6618a, ahead of the mark (@0x66190).
        for command in COMMANDS {
            let word = OrderWord::of(command.order());
            for fighter in [false, true] {
                assert_eq!(
                    word_change(word, command, fighter),
                    skipped(word.0),
                    "{command:?}"
                );
            }
        }
    }

    #[test]
    fn a_fighter_takes_any_command_that_differs_from_its_word() {
        for command in COMMANDS {
            let given = OrderWord::of(command.order());
            for held in [-1, 0, 1, 2, 3, 4, 7] {
                if held != given.0 {
                    assert_eq!(
                        word_change(OrderWord(held), command, true),
                        changed_to(given.0),
                        "{command:?} over {held}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_ship_that_is_no_fighter_takes_any_command_but_return_as_a_fighter_does() {
        for command in COMMANDS {
            if command == EscortCommand::Dock {
                continue;
            }
            let given = OrderWord::of(command.order());
            for held in [-1, 0, 1, 2, 3, 4, 7] {
                if held != given.0 {
                    assert_eq!(
                        word_change(OrderWord(held), command, false),
                        changed_to(given.0),
                        "{command:?} over {held}"
                    );
                }
            }
        }
    }

    #[test]
    fn return_to_a_ship_that_is_no_fighter_at_word_0_marks_it_and_changes_nothing() {
        // @0x661ab skips it, after the mark (@0x66190).
        assert_eq!(
            word_change(OrderWord(0), EscortCommand::Dock, false),
            WordChange {
                marks: true,
                ..skipped(0)
            }
        );
    }

    #[test]
    fn return_to_a_ship_that_is_no_fighter_at_any_other_word_writes_0_and_is_a_recall() {
        // @0x661b5, @0x661d4; -1 and a stray word too.
        for held in [-1, 1, 2, 4, 7] {
            assert_eq!(
                word_change(OrderWord(held), EscortCommand::Dock, false),
                WordChange {
                    recalled: true,
                    ..changed_to(0)
                },
                "{held}"
            );
        }
    }

    #[test]
    fn return_stands_only_on_a_marked_fighter() {
        let dock = EscortCommand::Dock;
        assert_eq!(standing(dock, true, true), DOCK);
        assert_eq!(standing(dock, true, false), None);
        assert_eq!(standing(dock, false, true), None);
        assert_eq!(standing(dock, false, false), None);
    }

    #[test]
    fn every_other_command_stands_as_given() {
        for command in COMMANDS {
            if command == EscortCommand::Dock {
                continue;
            }
            for (fighter, marked) in [(false, false), (false, true), (true, false), (true, true)] {
                assert_eq!(standing(command, fighter, marked), command.order());
            }
        }
    }

    // Through `Session::command_escorts`. Each sets the word directly, as
    // `nova_sim_ai` owns what the think writes (see "Crate coupling").

    /// `escorts` flying, its orders kept through the entry, each holding
    /// its word in `words`, none marked, the sounds taken.
    fn holding(catalog: &FakePilotCatalog, escorts: Vec<Escort>, words: &[i16]) -> Session {
        let rules = Rulebook::default().with_override(RuleKey::EscortOrders, RuleSource::Bible);
        let mut session = flying_by(catalog, pilot(catalog, 0, escorts), rules);
        for (npc, &word) in fleet_ids(&session).iter().zip(words) {
            session.space.npc_mut(*npc).expect("there").order_word = OrderWord(word);
        }
        session.take_sounds();
        session
    }

    fn fleet_ids(session: &Session) -> Vec<NpcId> {
        fleet_npcs(session).iter().map(|npc| npc.id).collect()
    }

    fn command(session: &mut Session, command: EscortCommand) -> Option<Commanded> {
        session.command_escorts(EscortGroup::All, command, &mut NeverFires)
    }

    fn marked(session: &Session) -> BTreeSet<NpcId> {
        session.space.escorts.commanded.clone()
    }

    /// Every ship of the fleet, marked or not.
    fn all_marked(session: &Session) -> BTreeSet<NpcId> {
        fleet_ids(session).into_iter().collect()
    }

    fn word(session: &Session, index: usize) -> OrderWord {
        fleet_npcs(session)[index].order_word
    }

    /// Escort `index`'s order on its record and its duty.
    fn order_of(session: &Session, index: usize) -> [Option<EscortOrder>; 2] {
        [
            session.pilot().escorts()[index].order,
            fleet_npcs(session)[index].escort.expect("an escort").order,
        ]
    }

    /// Whether `command` reported, as given and beeping once, and left
    /// nothing else to hear.
    fn reports(session: &mut Session, given: EscortCommand, as_given: EscortCommand) {
        let told = command(session, given).expect("a change");
        assert_eq!(told.command, as_given);
        assert_eq!(session.take_sounds(), [SimSound::EscortsCommanded]);
    }

    /// Whether `command` said nothing and beeped nothing.
    fn says_nothing(session: &mut Session, given: EscortCommand) {
        assert_eq!(command(session, given), None, "{given:?}");
        assert_eq!(session.take_sounds(), [], "{given:?}");
    }

    #[test]
    fn a_defending_ship_the_fallback_sent_to_formation_skips_a_recall() {
        let catalog = carrying();
        let mut session = holding(&catalog, vec![warship(DEFEND)], &[0]);
        says_nothing(&mut session, EscortCommand::Recall);
        assert_eq!(marked(&session), BTreeSet::new());
        assert_eq!(word(&session, 0), OrderWord(0));
        assert_eq!(order_of(&session, 0), [None, None], "the class's order");
    }

    #[test]
    fn a_defending_ship_the_fallback_sent_to_formation_is_marked_by_a_return_and_silent() {
        let catalog = carrying();
        let mut session = holding(&catalog, vec![warship(DEFEND)], &[0]);
        says_nothing(&mut session, EscortCommand::Dock);
        assert_eq!(marked(&session), all_marked(&session));
        assert_eq!(word(&session, 0), OrderWord(0));
        assert_eq!(order_of(&session, 0), [None, None]);
    }

    #[test]
    fn a_ship_the_fallback_sent_to_formation_given_its_old_order_again_takes_it() {
        // The engine compared with the standing order, and stayed silent.
        let catalog = carrying();
        let mut session = holding(&catalog, vec![warship(DEFEND)], &[0]);
        reports(&mut session, EscortCommand::Defend, EscortCommand::Defend);
        assert_eq!(marked(&session), all_marked(&session));
        assert_eq!(word(&session, 0), OrderWord(1));
        assert_eq!(order_of(&session, 0), [DEFEND, DEFEND]);
    }

    #[test]
    fn a_fighter_the_fallback_sent_home_skips_a_return_and_takes_its_old_order_again() {
        let catalog = carrying();
        let mut session = holding(
            &catalog,
            vec![Escort {
                order: DEFEND,
                ..out()
            }],
            &[3],
        );
        says_nothing(&mut session, EscortCommand::Dock);
        assert_eq!(marked(&session), BTreeSet::new());
        reports(&mut session, EscortCommand::Defend, EscortCommand::Defend);
        assert_eq!(marked(&session), all_marked(&session));
        assert_eq!(word(&session, 0), OrderWord(1));
    }

    #[test]
    fn the_word_decides_a_command_not_the_order_changed_since() {
        let catalog = carrying();
        let mut session = holding(&catalog, vec![warship(DEFEND)], &[4]);
        says_nothing(&mut session, EscortCommand::Hold);
        reports(&mut session, EscortCommand::Defend, EscortCommand::Defend);
    }

    #[test]
    fn a_second_command_before_any_think_reads_the_firsts_word() {
        let catalog = carrying();
        let mut session = holding(&catalog, vec![warship(None)], &[1]);
        reports(&mut session, EscortCommand::Hold, EscortCommand::Hold);
        says_nothing(&mut session, EscortCommand::Hold);
        reports(&mut session, EscortCommand::Defend, EscortCommand::Defend);
    }

    #[test]
    fn a_ship_placed_and_not_yet_thought_differs_from_every_command() {
        let catalog = carrying();
        for given in COMMANDS {
            let mut session = holding(&catalog, vec![warship(None)], &[-1]);
            let as_given = if given == EscortCommand::Dock {
                EscortCommand::Recall
            } else {
                given
            };
            reports(&mut session, given, as_given);
            assert_eq!(marked(&session), all_marked(&session), "{given:?}");
        }
    }

    #[test]
    fn a_group_reports_the_ship_that_changed_and_marks_only_it() {
        let catalog = carrying();
        let fleet = vec![warship(DEFEND), warship(DEFEND)];
        let mut session = holding(&catalog, fleet, &[0, 1]);
        reports(&mut session, EscortCommand::Recall, EscortCommand::Recall);
        let ids = fleet_ids(&session);
        assert_eq!(marked(&session), BTreeSet::from([ids[1]]));
        assert_eq!(word(&session, 0), OrderWord(0));
        assert_eq!(word(&session, 1), OrderWord(0));
    }

    #[test]
    fn a_mission_escorts_word_decides_as_a_hired_escorts_does() {
        use crate::session::mission_escorts::tests::{escorting, flying};
        use crate::session::mission_ships::tests::ships_catalog;
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        let ids: Vec<NpcId> = session.player_led().map(|npc| npc.id).collect();
        assert_eq!(ids.len(), 2);
        let set = |session: &mut Session, words: [i16; 2]| {
            for (id, word) in ids.iter().zip(words) {
                session.space.npc_mut(*id).expect("there").order_word = OrderWord(word);
            }
            session.space.escorts.commanded.clear();
            session.take_sounds();
        };
        let word_of =
            |session: &Session, id: NpcId| session.space.npc(id).map(|npc| npc.order_word.0);
        set(&mut session, [0, 0]);
        says_nothing(&mut session, EscortCommand::Recall);
        assert_eq!(marked(&session), BTreeSet::new());
        says_nothing(&mut session, EscortCommand::Dock);
        assert_eq!(marked(&session), BTreeSet::from_iter(ids.iter().copied()));
        set(&mut session, [0, 1]);
        reports(&mut session, EscortCommand::Recall, EscortCommand::Recall);
        assert_eq!(marked(&session), BTreeSet::from([ids[1]]));
        assert_eq!(word_of(&session, ids[1]), Some(0));
        set(&mut session, [0, 1]);
        reports(&mut session, EscortCommand::Dock, EscortCommand::Recall);
        assert_eq!(word_of(&session, ids[1]), Some(0), "a recall writes 0");
        set(&mut session, [3, 1]);
        reports(&mut session, EscortCommand::Hold, EscortCommand::Hold);
        assert_eq!(word_of(&session, ids[0]), Some(4));
    }

    #[test]
    fn a_record_with_no_ship_placed_compares_its_order() {
        let catalog = carrying();
        let mut session = holding(&catalog, vec![warship(DEFEND)], &[0]);
        session.space.escorts.fleet = vec![None];
        says_nothing(&mut session, EscortCommand::Defend);
        reports(&mut session, EscortCommand::Hold, EscortCommand::Hold);
        assert_eq!(marked(&session), BTreeSet::new());
    }

    #[test]
    fn a_ship_hired_in_flight_holds_no_word_until_its_first_think() {
        // `_SpawnEscort` @0x3ffb8.
        let catalog = carrying();
        let mut session = holding(&catalog, Vec::new(), &[]);
        session.pilot.escorts.push(warship(None));
        let placed = session.place_escort(0, &mut NeverFires);
        session.space.escorts.fleet.push(placed);
        assert_eq!(word(&session, 0), OrderWord::NONE);
        reports(&mut session, EscortCommand::Recall, EscortCommand::Recall);
    }

    // Through an entry: `_RespawnEscort` writes no word (@0x3d4b8-0x3d62c).

    #[test]
    fn after_a_take_off_a_command_before_the_first_think_reads_the_kept_word() {
        let catalog = carrying();
        let mut session = holding(&catalog, vec![warship(None)], &[4]);
        land_now(&mut session).expect("lands");
        session.take_off(&catalog, &mut NeverFires);
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(word(&session, 0), OrderWord(4));
        session.take_sounds();
        says_nothing(&mut session, EscortCommand::Hold);
    }

    #[test]
    fn a_loaded_pilots_first_placement_holds_its_records_order_by_either_reading() {
        // `_LoadPilotData` writes the table raw (@0x76102-0x7613b), which
        // the record stands in for; the engine's reset comes after it.
        let attack = Some(EscortOrder::Attack);
        for (source, kept) in [(RuleSource::Engine, None), (RuleSource::Bible, attack)] {
            let catalog = carrying();
            let rules = Rulebook::default().with_override(RuleKey::EscortOrders, source);
            let session = flying_by(&catalog, pilot(&catalog, 0, vec![warship(attack)]), rules);
            assert_eq!(word(&session, 0), OrderWord(2), "{source:?}");
            assert_eq!(order_of(&session, 0), [kept, kept], "{source:?}");
        }
    }

    // The standing order after a Return.

    #[test]
    fn a_marked_fighter_on_word_3_given_return_keeps_docking() {
        let catalog = carrying();
        let mut session = holding(
            &catalog,
            vec![Escort {
                order: DOCK,
                ..out()
            }],
            &[3],
        );
        let ids = fleet_ids(&session);
        session.space.mark_commanded(ids[0]);
        says_nothing(&mut session, EscortCommand::Dock);
        assert_eq!(order_of(&session, 0), [DOCK, DOCK]);
    }

    #[test]
    fn an_unmarked_fighter_on_word_3_given_return_keeps_formation_and_its_class_clears() {
        let catalog = carrying();
        let mut session = holding(
            &catalog,
            vec![Escort {
                order: DEFEND,
                ..out()
            }],
            &[3],
        );
        says_nothing(&mut session, EscortCommand::Dock);
        assert_eq!(order_of(&session, 0), [None, None]);
        assert_eq!(session.space.escorts.class_orders[0], DOCK, "the table's");
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.space.escorts.class_orders[0], None, "@0x6e37d");
        assert!(session.out.class_orders_cleared);
        assert_eq!(order_of(&session, 0), [None, None]);
    }

    #[test]
    fn with_a_marked_fellow_out_the_class_keeps_return_and_the_skipped_fighter_stays_in_formation()
    {
        let catalog = carrying();
        let mut session = holding(
            &catalog,
            vec![
                Escort {
                    order: DEFEND,
                    ..out()
                },
                out(),
            ],
            &[3, 3],
        );
        let ids = fleet_ids(&session);
        session.space.mark_commanded(ids[1]);
        says_nothing(&mut session, EscortCommand::Dock);
        session
            .space
            .settle_class_orders(&mut session.pilot, &session.statics);
        assert_eq!(session.space.escorts.class_orders[0], DOCK);
        assert_eq!(order_of(&session, 0), [None, None], "skipped");
        assert_eq!(order_of(&session, 1), [DOCK, DOCK], "marked");
    }

    #[test]
    fn an_unmarked_fighter_on_word_0_given_return_is_marked_and_docks() {
        let catalog = carrying();
        let mut session = holding(&catalog, vec![out()], &[0]);
        reports(&mut session, EscortCommand::Dock, EscortCommand::Dock);
        assert_eq!(marked(&session), all_marked(&session));
        assert_eq!(word(&session, 0), OrderWord(3));
        assert_eq!(order_of(&session, 0), [DOCK, DOCK]);
    }

    #[test]
    fn a_record_with_no_ship_placed_keeps_return() {
        let catalog = carrying();
        let mut session = holding(
            &catalog,
            vec![Escort {
                order: DOCK,
                ..out()
            }],
            &[3],
        );
        session.space.escorts.fleet = vec![None];
        command(&mut session, EscortCommand::Dock);
        assert_eq!(session.pilot().escorts()[0].order, DOCK);
    }
}
