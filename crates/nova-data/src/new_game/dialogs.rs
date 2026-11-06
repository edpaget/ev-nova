//! The new game's conversations: the compiled Ink in `dialog/`, and each
//! stellar's `dialog` ref mapped onto its `spöb`.
//!
//! The author writes `dialog/<file>.ink` and compiles it with `inklecate`
//! to `dialog/<file>.ink.json`; the loader reads only the compiled JSON.
//! [`read`] lists `dialog/*.json` through the [`Source`] and keeps the
//! names ending in `.ink.json`, each keyed by the Ink file a ref names
//! (`dialog/<file>.ink`). The format's directory adapter matches one
//! extension, so it is asked for `json` rather than `ink.json`.
//!
//! A stellar's ref whose compiled file is missing is a problem against the
//! stellar's file, and that stellar has no conversation. One file that
//! isn't text fails the whole listing: that is one problem against
//! `dialog/`, and each ref's own file is then read on its own, so a ref is
//! reported missing only when its file is, and unreadable when it can't be
//! read. Whether the knot
//! the ref names is in the story is a question only an Ink runtime can
//! answer, so it is asked through a port, [`DialogCheck`], which
//! [`NewGame::check_dialogs`] runs after loading: `nova` adapts it over
//! `nova-ink`'s `nova_ink::InkLoader`. With the `fixture` feature (and in
//! this crate's tests) `fixture` publishes its fake (`FakeCheck`) and
//! the contract every check passes (`check_contract`), over the fixture
//! folder's own `dialog/dock-job.ink.json`.
//!
//! Only stellars' refs are mapped and checked: missions' `offer` and
//! `complete` refs wait for the mission model (engine-story phase 4).
//!
//! # Regenerating the fixture's compiled story
//!
//! `tests/fixtures/new-game/dialog/dock-job.ink.json` is compiled by hand
//! with Inky 0.15.2's `inklecate` (`inkVersion` 21), so no test needs
//! `inklecate`. After editing `dock-job.ink`, run from the workspace root:
//!
//! ```sh
//! "/Applications/Inky.app/Contents/Resources/app.asar.unpacked/main-process/ink/inklecate_mac" \
//!     -o crates/nova-data/tests/fixtures/new-game/dialog/dock-job.ink.json \
//!     crates/nova-data/tests/fixtures/new-game/dialog/dock-job.ink
//! ```

use std::collections::BTreeMap;
use std::sync::Arc;

use nova_galaxy_format::load::Galaxy;
use nova_galaxy_format::source::Source;

use super::{IdTable, NewGame, Problem};
use crate::store::{DialogPlace, Dialogs};
use crate::wire::id::StellarId;

/// Checks that a compiled story has a knot: the one question about Ink the
/// loader asks.
pub trait DialogCheck {
    /// `Ok` when the story compiled to `compiled` opens and has the knot
    /// or `knot.stitch` `knot`.
    ///
    /// # Errors
    ///
    /// Why not, for the author: the story doesn't open, or names no such
    /// knot.
    fn check(&self, compiled: &str, knot: &str) -> Result<(), String>;
}

/// Reads the compiled stories in `dialog/` and maps each stellar's ref
/// onto its `spöb`, reporting what doesn't map to `problems`.
pub fn read(
    galaxy: &Galaxy,
    ids: &IdTable,
    source: &impl Source,
    problems: &mut Vec<Problem>,
) -> Dialogs {
    let (mut compiled, listed): (BTreeMap<String, Arc<str>>, bool) =
        match source.files("dialog", "json") {
            Ok(files) => (
                files
                    .into_iter()
                    .filter_map(|file| {
                        // Refs are lower case, so a name in another case
                        // matches none: it isn't read.
                        let stem = file.path.strip_suffix(".ink.json")?;
                        Some((format!("{stem}.ink"), Arc::from(file.text)))
                    })
                    .collect(),
                true,
            ),
            Err(e) => {
                problems.push(Problem::new("dialog/", format!("listing failed: {e}")));
                (BTreeMap::new(), false)
            }
        };
    let mut stellars = BTreeMap::new();
    for (id, doc) in &galaxy.stellars {
        let (Some(dialog), Some(&nova)) = (&doc.data.dialog, ids.stellars.get(id)) else {
            continue;
        };
        let file = dialog.file();
        let json = format!("{file}.json");
        // A folder that failed to list (one unreadable file fails it all)
        // still yields each ref's own file, read on its own.
        if !listed && !compiled.contains_key(file) {
            match read_one(source, &json) {
                Ok(Some(text)) => {
                    compiled.insert(file.to_owned(), Arc::from(text));
                }
                Ok(None) => {}
                Err(why) => {
                    problems.push(Problem::new(
                        &doc.path,
                        format!("dialog {dialog}: {json} couldn't be read: {why}"),
                    ));
                    continue;
                }
            }
        }
        if compiled.contains_key(file) {
            stellars.insert(
                StellarId(nova),
                DialogPlace {
                    file: file.to_owned(),
                    knot: dialog.path().to_owned(),
                },
            );
        } else {
            problems.push(Problem::new(
                &doc.path,
                format!("dialog {dialog}: no {json}; compile {file} with inklecate"),
            ));
        }
    }
    Dialogs { compiled, stellars }
}

/// The text of the file at `path`, `None` if there is none, or why it
/// couldn't be read as text.
fn read_one(source: &impl Source, path: &str) -> Result<Option<String>, String> {
    match source.bytes(path) {
        Ok(Some(bytes)) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|e| e.to_string()),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

impl NewGame {
    /// Asks `check` about every stellar's conversation. Each one it
    /// refuses is a problem against the stellar's file, and the stellar
    /// keeps no conversation.
    pub fn check_dialogs(&mut self, check: &dyn DialogCheck) {
        let mut dialogs = self.data.dialogs().clone();
        let ids = &self.ids;
        let problems = &mut self.problems;
        dialogs.stellars.retain(|&stellar, place| {
            let compiled = dialogs.compiled.get(&place.file).map_or("", |text| text);
            let Err(why) = check.check(compiled, &place.knot) else {
                return true;
            };
            problems.push(Problem::new(
                stellar_file(ids, stellar),
                format!("dialog {}: {why}", place.as_ref_text()),
            ));
            false
        });
        self.data = std::mem::take(&mut self.data).with_dialogs(dialogs);
    }
}

/// The file of the stellar whose `spöb` is `stellar`.
fn stellar_file(ids: &IdTable, stellar: StellarId) -> String {
    let id = ids
        .stellars
        .iter()
        .find_map(|(id, &nova)| (StellarId(nova) == stellar).then_some(id.as_str()))
        .unwrap_or_default();
    format!("stellars/{id}.md")
}

/// The [`DialogCheck`] port's fake and contract, for this crate's tests
/// and, with the `fixture` feature, for the adapter's.
#[cfg(any(test, feature = "fixture"))]
pub mod fixture {
    use super::DialogCheck;

    /// The fixture folder's offer conversation
    /// (`tests/fixtures/new-game/dialog/dock-job.ink`), compiled.
    pub const DOCK_JOB_JSON: &str =
        include_str!("../../tests/fixtures/new-game/dialog/dock-job.ink.json");

    /// A [`DialogCheck`] that knows the knots it was given, each story by
    /// its exact compiled text.
    #[derive(Clone, Debug, Default)]
    pub struct FakeCheck {
        stories: Vec<(String, Vec<String>)>,
    }

    impl FakeCheck {
        /// Knows the story compiled to `compiled`, with the knots and
        /// `knot.stitch` paths `knots`.
        #[must_use]
        pub fn story(mut self, compiled: &str, knots: &[&str]) -> Self {
            let knots = knots.iter().map(|&k| k.to_owned()).collect();
            self.stories.push((compiled.to_owned(), knots));
            self
        }

        /// Knows [`DOCK_JOB_JSON`]'s knots.
        #[must_use]
        pub fn meeting_the_contract() -> Self {
            Self::default().story(DOCK_JOB_JSON, &["walkthrough", "scene_end", "job_offer"])
        }
    }

    impl DialogCheck for FakeCheck {
        fn check(&self, compiled: &str, knot: &str) -> Result<(), String> {
            let (_, knots) = self
                .stories
                .iter()
                .find(|(text, _)| text == compiled)
                .ok_or("not a compiled Ink story")?;
            if knots.iter().any(|k| k == knot) {
                Ok(())
            } else {
                Err(format!("no knot or stitch `{knot}`"))
            }
        }
    }

    /// Checks the port's contract against `check`, over [`DOCK_JOB_JSON`]:
    ///
    /// 1. Its knots `walkthrough`, `scene_end` and `job_offer` pass.
    /// 2. A knot it lacks (`nowhere`), or a stitch its offer lacks
    ///    (`job_offer.nowhere`), is refused with a message naming it.
    /// 3. Text that isn't a story is refused.
    ///
    /// # Errors
    ///
    /// A message naming the first clause `check` breaks.
    pub fn check_contract(check: &impl DialogCheck) -> Result<(), String> {
        for knot in ["walkthrough", "scene_end", "job_offer"] {
            check
                .check(DOCK_JOB_JSON, knot)
                .map_err(|e| format!("a knot it has: `{knot}` was refused: {e}"))?;
        }
        for knot in ["nowhere", "job_offer.nowhere"] {
            match check.check(DOCK_JOB_JSON, knot) {
                Err(why) if why.contains(knot) => {}
                Err(why) => {
                    return Err(format!(
                        "a knot it lacks: `{knot}` was refused as {why:?}, which doesn't name it"
                    ));
                }
                Ok(()) => return Err(format!("a knot it lacks: `{knot}` passed")),
            }
        }
        match check.check("not a story", "job_offer") {
            Err(_) => Ok(()),
            Ok(()) => Err("junk: text that isn't a story passed".to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_galaxy_format::fixture;
    use nova_galaxy_format::source::{MemSource, SourceFile};

    use super::fixture::{DOCK_JOB_JSON, FakeCheck, check_contract};
    use super::*;
    use crate::new_game::load;

    const YARD: &str = "---\nname: Alpha Yard\nkind: station\nsystem: alpha\nposition: [0, 0]\n\
                        owner: league\nvisual: {asset: spire}\ndialog: dialog/yard.ink#greet\n---\n";

    fn loaded(source: &MemSource) -> NewGame {
        load(source, Path::new("sample"))
    }

    fn with_yard_dialog() -> MemSource {
        fixture::galaxy()
            .with("stellars/alpha-yard.md", YARD)
            .with("dialog/yard.ink.json", "{yard}")
    }

    fn yard(game: &NewGame) -> StellarId {
        StellarId(game.ids.stellars[&id("alpha-yard")])
    }

    fn id(text: &str) -> nova_galaxy_format::id::Id {
        nova_galaxy_format::id::Id::try_from(text.to_owned()).expect("an id")
    }

    fn place(file: &str, knot: &str) -> DialogPlace {
        DialogPlace {
            file: file.to_owned(),
            knot: knot.to_owned(),
        }
    }

    #[test]
    fn a_stellar_s_ref_maps_to_its_place_and_the_text_is_kept_by_file() {
        let game = loaded(&with_yard_dialog().with("dialog/other.ink.json", "{other}"));
        assert_eq!(game.problems, []);
        let dialogs = game.data.dialogs();
        assert_eq!(
            dialogs.stellars,
            BTreeMap::from([(yard(&game), place("dialog/yard.ink", "greet"))])
        );
        let files: Vec<_> = dialogs.compiled.keys().map(String::as_str).collect();
        assert_eq!(files, ["dialog/other.ink", "dialog/yard.ink"]);
        assert_eq!(&*dialogs.compiled["dialog/yard.ink"], "{yard}");
    }

    #[test]
    fn a_stitch_is_kept_with_its_knot() {
        let source = with_yard_dialog().with(
            "stellars/alpha-yard.md",
            &YARD.replace("#greet", "#greet.again"),
        );
        let game = loaded(&source);
        assert_eq!(
            game.data.dialogs().stellars[&yard(&game)],
            place("dialog/yard.ink", "greet.again")
        );
    }

    #[test]
    fn json_that_is_not_compiled_ink_and_ink_source_are_not_read() {
        let source = with_yard_dialog()
            .with("dialog/notes.json", "{}")
            .with("dialog/x.ink", "Hello.")
            .with("dialog/deeper/y.ink.json", "{}");
        let files: Vec<_> = loaded(&source)
            .data
            .dialogs()
            .compiled
            .keys()
            .cloned()
            .collect();
        assert_eq!(files, ["dialog/yard.ink"]);
    }

    #[test]
    fn a_missing_compiled_file_is_a_problem_against_the_stellar_and_maps_nothing() {
        let game = loaded(&with_yard_dialog().without("dialog/yard.ink.json"));
        assert_eq!(
            game.problems,
            [Problem::new(
                "stellars/alpha-yard.md",
                "dialog dialog/yard.ink#greet: no dialog/yard.ink.json; \
                 compile dialog/yard.ink with inklecate"
            )]
        );
        assert_eq!(game.data.dialogs().stellars, BTreeMap::new());
    }

    #[test]
    fn a_folder_that_fails_to_list_is_one_problem_and_each_ref_is_read_on_its_own() {
        // One bad file fails the whole listing; the ref's own file still
        // reads, so it isn't reported missing.
        let source = with_yard_dialog().with_bytes("dialog/bad.ink.json", &[0xff, 0xfe]);
        let game = loaded(&source);
        assert_eq!(game.problems.len(), 1, "{:?}", game.problems);
        assert_eq!(game.problems[0].file, "dialog/");
        assert!(
            game.problems[0].message.starts_with("listing failed: "),
            "{}",
            game.problems[0].message
        );
        let dialogs = game.data.dialogs();
        assert_eq!(
            dialogs.stellars,
            BTreeMap::from([(yard(&game), place("dialog/yard.ink", "greet"))])
        );
        assert_eq!(&*dialogs.compiled["dialog/yard.ink"], "{yard}");
    }

    #[test]
    fn a_ref_s_own_unreadable_file_is_reported_as_unreadable_not_missing() {
        let source = with_yard_dialog().with_bytes("dialog/yard.ink.json", &[0xff, 0xfe]);
        let game = loaded(&source);
        assert_eq!(game.problems.len(), 2, "{:?}", game.problems);
        assert_eq!(game.problems[0].file, "dialog/");
        assert_eq!(game.problems[1].file, "stellars/alpha-yard.md");
        let message = &game.problems[1].message;
        assert!(
            message.starts_with(
                "dialog dialog/yard.ink#greet: dialog/yard.ink.json couldn't be read: "
            ),
            "{message}"
        );
        assert!(!message.contains("inklecate"), "{message}");
        assert_eq!(game.data.dialogs(), &Dialogs::default());
    }

    #[test]
    fn a_folder_that_lists_is_trusted_and_no_ref_is_read_behind_it() {
        /// The galaxy, whose `dialog/` lists nothing but whose `bytes` would
        /// serve the yard's compiled file.
        struct Unlisted(MemSource);
        impl Source for Unlisted {
            fn files(&self, dir: &str, ext: &str) -> std::io::Result<Vec<SourceFile>> {
                if dir == "dialog" {
                    return Ok(Vec::new());
                }
                self.0.files(dir, ext)
            }
            fn bytes(&self, path: &str) -> std::io::Result<Option<Vec<u8>>> {
                self.0.bytes(path)
            }
        }
        let game = load(&Unlisted(with_yard_dialog()), Path::new("sample"));
        assert_eq!(game.data.dialogs(), &Dialogs::default());
        assert_eq!(game.problems.len(), 1, "{:?}", game.problems);
        assert!(game.problems[0].message.contains("no dialog/yard.ink.json"));
    }

    #[test]
    fn with_the_folder_unlisted_a_missing_ref_is_still_missing() {
        let source = with_yard_dialog()
            .without("dialog/yard.ink.json")
            .with_bytes("dialog/bad.ink.json", &[0xff, 0xfe]);
        let game = loaded(&source);
        assert_eq!(game.problems.len(), 2, "{:?}", game.problems);
        assert_eq!(
            game.problems[1],
            Problem::new(
                "stellars/alpha-yard.md",
                "dialog dialog/yard.ink#greet: no dialog/yard.ink.json; \
                 compile dialog/yard.ink with inklecate"
            )
        );
    }

    #[test]
    fn a_stellar_left_out_brings_no_conversation_or_second_problem() {
        let source = with_yard_dialog().with(
            "stellars/alpha-yard.md",
            &YARD.replace("system: alpha", "system: nowhere"),
        );
        let game = loaded(&source);
        assert_eq!(game.data.dialogs().stellars, BTreeMap::new());
        assert!(
            game.problems.iter().all(|p| !p.message.contains("dialog")),
            "{:?}",
            game.problems
        );
    }

    #[test]
    fn check_dialogs_keeps_the_knots_the_check_passes() {
        let mut game = loaded(&with_yard_dialog());
        game.check_dialogs(&FakeCheck::default().story("{yard}", &["greet"]));
        assert_eq!(game.problems, []);
        assert_eq!(game.data.dialogs().stellars.len(), 1);
    }

    #[test]
    fn check_dialogs_reports_a_refused_knot_against_the_stellar_and_drops_it() {
        let mut game = loaded(&with_yard_dialog());
        game.check_dialogs(&FakeCheck::default().story("{yard}", &["other"]));
        assert_eq!(
            game.problems,
            [Problem::new(
                "stellars/alpha-yard.md",
                "dialog dialog/yard.ink#greet: no knot or stitch `greet`"
            )]
        );
        assert_eq!(game.data.dialogs().stellars, BTreeMap::new());
        assert_eq!(game.data.dialogs().compiled.len(), 1, "the text stays");
    }

    #[test]
    fn check_dialogs_asks_about_the_stellar_s_own_story() {
        let mut game = loaded(&with_yard_dialog().with("dialog/other.ink.json", "{other}"));
        game.check_dialogs(&FakeCheck::default().story("{other}", &["greet"]));
        assert_eq!(game.problems.len(), 1, "{:?}", game.problems);
        assert_eq!(
            game.problems[0].message,
            "dialog dialog/yard.ink#greet: not a compiled Ink story"
        );
    }

    #[test]
    fn the_fake_meets_the_contract() {
        assert_eq!(check_contract(&FakeCheck::meeting_the_contract()), Ok(()));
    }

    #[test]
    fn the_contract_rejects_checks_that_break_it() {
        struct Passes;
        impl DialogCheck for Passes {
            fn check(&self, _: &str, _: &str) -> Result<(), String> {
                Ok(())
            }
        }
        struct Mute;
        impl DialogCheck for Mute {
            fn check(&self, compiled: &str, knot: &str) -> Result<(), String> {
                FakeCheck::meeting_the_contract()
                    .check(compiled, knot)
                    .map_err(|_| "no".to_owned())
            }
        }
        struct Refuses;
        impl DialogCheck for Refuses {
            fn check(&self, _: &str, _: &str) -> Result<(), String> {
                Err("no".to_owned())
            }
        }
        struct OpensJunk;
        impl DialogCheck for OpensJunk {
            fn check(&self, compiled: &str, knot: &str) -> Result<(), String> {
                let compiled = if compiled == "not a story" {
                    DOCK_JOB_JSON
                } else {
                    compiled
                };
                FakeCheck::meeting_the_contract().check(compiled, knot)
            }
        }
        let refused = check_contract(&Refuses).unwrap_err();
        assert!(refused.contains("a knot it has"), "{refused}");
        let passed = check_contract(&Passes).unwrap_err();
        assert!(passed.contains("`nowhere` passed"), "{passed}");
        let mute = check_contract(&Mute).unwrap_err();
        assert!(mute.contains("doesn't name it"), "{mute}");
        let junk = check_contract(&OpensJunk).unwrap_err();
        assert!(junk.starts_with("junk"), "{junk}");
    }
}
