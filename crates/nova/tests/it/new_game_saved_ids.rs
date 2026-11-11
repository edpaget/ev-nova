//! Acceptance: a pilot saved over the pinned sample fixture
//! (`nova_data::new_game::fixture::sample_folder`) loads into the same
//! system, stellar and ship after the folder grows a system, a stellar, a
//! jump point, a hull and a start, each sorting first in its kind, where
//! it would have renumbered the records after it without the folder's
//! `nova-ids.json`.
//!
//! A wiring test over the real nova-data: the numbering rules are pinned
//! there (`new_game::pins`, `tests/it/new_game_ids.rs`).

use std::fs;
use std::path::Path;

use nova_data::new_game::{self, NewGame};
use nova_sim::{Escort, Session, ShipId, StellarId, SystemId};

use crate::new_game_travel::{new_game_rules, nova_of, pilot_for};

/// Copies the folder `from` into `to`, recursively.
pub(crate) fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("creates");
    for entry in fs::read_dir(from).expect("lists") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a type").is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copies");
        }
    }
}

/// Adds a system, a stellar, a one-way jump point, a hull and a start to
/// the folder `root`, each first in its kind.
fn grow(root: &Path) {
    let write = |dir: &str, file: &str, text: &str| {
        fs::write(root.join(dir).join(file), text).expect("writes");
    };
    write(
        "systems",
        "aaa-reach.md",
        "---\nname: Reach\nregion: commonwealth-rim\nmap: [40, 40]\n---\n",
    );
    write(
        "stellars",
        "aaa-moon.md",
        "---\nname: Moon\nkind: planet\nsystem: luyten-crossing\nposition: [300, 300]\n---\n",
    );
    let brightwater = root.join("systems").join("brightwater.md");
    let text = fs::read_to_string(&brightwater).expect("reads");
    let text = text.replacen(
        "jump_points:\n",
        "jump_points:\n  - id: aaa-spur\n    position: [100, 100]\n    \
         to: marrow/far-gate\n    stability: stable\n",
        1,
    );
    fs::write(&brightwater, text).expect("writes");
    write(
        "ships",
        "aaa-hauler.md",
        "---\nname: Hauler\nstyle: civilian\nrole: freighter\nmodel: light-freighter\n\
         size: {length: 30, beam: 20, collision_radius: 15}\nstats: {hull: 300, cargo: 40}\n---\n",
    );
    write(
        "starts",
        "aaa-start.md",
        "---\nname: Early start\nsystem: luyten-crossing\nship: light-freighter\ncash: 500\n---\n",
    );
}

/// The places the test reads, by their new-game ids.
fn place(game: &NewGame) -> (SystemId, StellarId, ShipId) {
    (
        SystemId(nova_of(&game.ids.systems, "luyten-crossing")),
        StellarId(nova_of(&game.ids.stellars, "luyten-exchange")),
        ShipId(nova_of(&game.ids.ships, "light-freighter")),
    )
}

#[test]
fn a_pilot_saved_before_the_data_grew_loads_into_the_same_place() {
    let temp = tempfile::tempdir().expect("a temporary directory");
    let root = temp.path().join("sample");
    copy(&new_game::fixture::sample_folder(), &root);
    let before = new_game::open(&root).expect("lists");
    let (system, stellar, ship) = place(&before);
    let mut pilot = pilot_for(&before);
    pilot.system = system;
    pilot.stellar = Some(stellar);
    pilot.ship = ship;
    pilot.escorts.push(Escort {
        ship,
        reserves: pilot.reserves,
        order: None,
        carried: false,
        wage: Some(0),
        person: None,
        mark: None,
        voice: None,
        berth: None,
    });
    let saved = nova_sim::save::encode(&pilot);

    grow(&root);
    let after = new_game::open(&root).expect("lists");
    let lines: Vec<_> = after.problems.iter().map(ToString::to_string).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("nova-ids.json: it leaves out 5 records"),
        "the additions are unpinned: {lines:?}"
    );
    assert_eq!(
        place(&after),
        (system, stellar, ship),
        "the tables kept them"
    );

    let decoded = nova_sim::save::decode(&saved).expect("a pilot");
    let session = Session::load(&after.data, decoded, &new_game_rules()).expect("loads");
    let loaded = session.pilot();
    assert_eq!(
        (loaded.system, loaded.stellar, loaded.ship),
        (system, Some(stellar), ship)
    );
    let escorts: Vec<_> = loaded.escorts.iter().map(|escort| escort.ship).collect();
    assert_eq!(escorts, [ship]);
}
