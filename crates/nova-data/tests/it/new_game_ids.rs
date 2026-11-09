//! Acceptance: adding a system, a stellar, a jump point, a hull, a start
//! or a good to a pinned folder moves no existing record's Nova ID. Each
//! case adds to a temporary copy of a pinned fixture, with its
//! `nova-ids.json`, a record whose id sorts first in its kind, where it
//! would have renumbered every record after it without the table: the
//! sample fixture (`new_game::fixture::sample_folder`), or, for a good,
//! the new-game fixture (`new_game::fixture::folder`), whose seven goods
//! fill the six standard slots and one `jünk`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use nova_data::new_game::{self, IdTable};

/// Copies the folder `from` into `to`, recursively.
fn copy(from: &Path, to: &Path) {
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

/// One addition to the sample.
#[derive(Clone, Copy, Debug)]
enum Addition {
    System,
    Stellar,
    JumpPoint,
    Hull,
    Start,
    Good,
}

const EVERY: [Addition; 5] = [
    Addition::System,
    Addition::Stellar,
    Addition::JumpPoint,
    Addition::Hull,
    Addition::Start,
];

/// Writes `addition` into the folder `root`.
fn add(root: &Path, addition: Addition) {
    let write = |dir: &str, file: &str, text: &str| {
        fs::write(root.join(dir).join(file), text).expect("writes");
    };
    match addition {
        Addition::System => write(
            "systems",
            "aaa-reach.md",
            "---\nname: Reach\nregion: commonwealth-rim\nmap: [40, 40]\n---\n",
        ),
        Addition::Stellar => write(
            "stellars",
            "aaa-moon.md",
            "---\nname: Moon\nkind: planet\nsystem: luyten-crossing\nposition: [300, 300]\n---\n",
        ),
        Addition::JumpPoint => {
            // A one-way point, first in Brightwater and so first of all.
            let path = root.join("systems").join("brightwater.md");
            let text = fs::read_to_string(&path).expect("reads");
            let text = text.replacen(
                "jump_points:\n",
                "jump_points:\n  - id: aaa-spur\n    position: [100, 100]\n    \
                 to: marrow/far-gate\n    stability: stable\n",
                1,
            );
            fs::write(&path, text).expect("writes");
        }
        Addition::Hull => write(
            "ships",
            "aaa-hauler.md",
            "---\nname: Hauler\nstyle: civilian\nrole: freighter\nmodel: light-freighter\n\
             size: {length: 30, beam: 20, collision_radius: 15}\nstats: {hull: 300, cargo: 40}\n---\n",
        ),
        Addition::Start => write(
            "starts",
            "aaa-start.md",
            "---\nname: Early start\nsystem: luyten-crossing\nship: light-freighter\ncash: 500\n---\n",
        ),
        Addition::Good => write("goods", "aaa-ale.md", "---\nname: Ale\nprice: 20\n---\n"),
    }
}

/// Each entry of `table`, as `kind/key` and its ID.
fn flat(table: &IdTable) -> BTreeMap<String, i16> {
    fn of<K: ToString>(kind: &str, map: &BTreeMap<K, i16>) -> Vec<(String, i16)> {
        map.iter()
            .map(|(key, &nova)| (format!("{kind}/{}", key.to_string()), nova))
            .collect()
    }
    [
        of("factions", &table.factions),
        of("systems", &table.systems),
        of("stellars", &table.stellars),
        of("gates", &table.gates),
        of("ships", &table.ships),
        of("starts", &table.starts),
        of("outfits", &table.outfits),
        of("weapons", &table.weapons),
        of("kits", &table.kits),
        of("commodities", &table.commodities),
        of("junk", &table.junk),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The key `addition` gives its record in the flat table.
fn added_key(addition: Addition) -> &'static str {
    match addition {
        Addition::System => "systems/aaa-reach",
        Addition::Stellar => "stellars/aaa-moon",
        Addition::JumpPoint => "gates/brightwater/aaa-spur",
        Addition::Hull => "ships/aaa-hauler",
        Addition::Start => "starts/aaa-start",
        Addition::Good => "junk/aaa-ale",
    }
}

/// Opens a copy of the sample with `additions` made, and checks every
/// original record kept its ID and each addition got one.
fn check(additions: &[Addition]) {
    check_in(&new_game::fixture::sample_folder(), additions);
}

/// [`check`], over a copy of `folder`.
fn check_in(folder: &Path, additions: &[Addition]) {
    let original = new_game::open(folder).expect("lists");
    let temp = tempfile::tempdir().expect("a temporary directory");
    let root = temp.path().join("copy");
    copy(folder, &root);
    for &addition in additions {
        add(&root, addition);
    }
    let grown = new_game::open(&root).expect("lists");
    let before = flat(&original.ids);
    let after = flat(&grown.ids);
    for (key, nova) in &before {
        assert_eq!(after.get(key), Some(nova), "{additions:?}: {key}");
    }
    for &addition in additions {
        let key = added_key(addition);
        assert!(
            after.contains_key(key),
            "{additions:?}: {key} got an ID: {:?}",
            grown.problems
        );
    }
    for &addition in additions {
        let nova = after[added_key(addition)];
        assert!(
            !shares_range(added_key(addition), &before, nova),
            "{additions:?}: {} took a pinned ID {nova}",
            added_key(addition)
        );
    }
}

/// Whether `nova` is held in `before` by a record of `key`'s range.
fn shares_range(key: &str, before: &BTreeMap<String, i16>, nova: i16) -> bool {
    let range = |key: &str| {
        match key.split('/').next().expect("a kind") {
            "gates" | "stellars" => "stellars",
            "kits" | "outfits" => "outfits",
            kind => kind,
        }
        .to_owned()
    };
    before
        .iter()
        .any(|(other, &held)| held == nova && range(other) == range(key))
}

#[test]
fn adding_a_system_moves_no_id() {
    check(&[Addition::System]);
}

#[test]
fn adding_a_stellar_moves_no_id() {
    check(&[Addition::Stellar]);
}

#[test]
fn adding_a_jump_point_moves_no_id() {
    check(&[Addition::JumpPoint]);
}

#[test]
fn adding_a_hull_moves_no_id() {
    check(&[Addition::Hull]);
}

#[test]
fn adding_a_start_moves_no_id() {
    check(&[Addition::Start]);
}

/// The six slots are pinned, so the good takes the `jünk` after the
/// pinned one, where unpinned it would have taken slot 0 and moved every
/// standard good up.
#[test]
fn adding_a_good_moves_no_id() {
    check_in(&new_game::fixture::folder(), &[Addition::Good]);
}

#[test]
fn adding_all_five_together_moves_no_id() {
    check(&EVERY);
}
