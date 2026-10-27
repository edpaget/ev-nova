//! Where the `Nova Files` are: the directory the player named, else the
//! one remembered, else the first of the usual places that holds the
//! game's data, else the one the player picks when asked.
//!
//! The decisions are here, over two ports: [`DataSource`], which opens a
//! directory the way the game loads it, and [`NovaFilesDialog`], which asks
//! the player. [`DiskData`] is the real data source; the dialogs are the
//! platform's (`crate::platform::dialogs`).

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use nova_audio::SettingsStore;
use nova_data::GameData;
use nova_data::store::OpenError;

use crate::cli::Usage;
use crate::config::Os;
use crate::settings::{GameSettings, SettingsKeeper};

/// What the open panel asks first.
pub const PROMPT: &str = "Locate your EV Nova folder";

/// What opening a directory as the game's data found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opened<D> {
    /// The loader loaded at least one data file from it.
    Loaded(D),
    /// It listed, but no data file in it loaded.
    Empty(D),
    /// It could not be listed: why.
    Unreadable(String),
}

/// Opens a directory as the game's `Nova Files`: the port the lookup tests
/// candidates through.
pub trait DataSource {
    /// The data an open directory gives.
    type Data;

    /// Opens `dir`.
    fn open(&self, dir: &Path) -> Opened<Self::Data>;

    /// Whether `dir` holds an entry, a file or a folder, called `name`.
    fn has_entry(&self, dir: &Path, name: &str) -> bool;
}

/// The game's data on disk: a directory is opened with
/// [`GameData::open`], the loader's own walk and file loads, so a
/// directory counts exactly when the game would load something from it.
/// The classic resource-fork copy, whose files have no loadable data
/// fork, is [`Opened::Empty`].
#[derive(Clone, Copy, Debug, Default)]
pub struct DiskData;

impl DataSource for DiskData {
    type Data = GameData;

    fn open(&self, dir: &Path) -> Opened<GameData> {
        classify(GameData::open(dir, None))
    }

    fn has_entry(&self, dir: &Path, name: &str) -> bool {
        std::fs::symlink_metadata(dir.join(name)).is_ok()
    }
}

/// What opening the data gave, as the lookup sees it.
fn classify(opened: Result<GameData, OpenError>) -> Opened<GameData> {
    match opened {
        Ok(data) if data.files().is_empty() => Opened::Empty(data),
        Ok(data) => Opened::Loaded(data),
        Err(error) => Opened::Unreadable(error.to_string()),
    }
}

/// Asks the player where the `Nova Files` are: the port over the
/// platform's open panel.
pub trait NovaFilesDialog {
    /// Asks with `prompt`; the path picked, or `None` when the player
    /// cancelled (or there is no dialog on this platform).
    fn pick(&mut self, prompt: &str) -> Option<PathBuf>;
}

/// The `Nova Files` inside an `EV Nova.app` at `app`.
fn inside_app(app: PathBuf) -> PathBuf {
    let mut dir = app;
    dir.extend(["Contents", "Resources", "Nova Files"]);
    dir
}

/// The places a player keeps the game on `os`, in the order they are
/// tried, from the environment variables `env` looks up:
///
/// - macOS: `/Applications/EV Nova.app/Contents/Resources/Nova Files`,
///   then the same under `$HOME/Applications` (left out when `HOME` is
///   unset or empty).
/// - Windows ([`windows_places`]): the `Nova Files` of an `EV Nova`
///   folder in `%ProgramFiles%`, then `%ProgramFiles(x86)%`, then under
///   `Ambrosia Software` in each, then in `%LOCALAPPDATA%\Programs`. A
///   missing or empty variable drops its places, and a place named twice
///   (`ProgramFiles(x86)` the same as `ProgramFiles`) is tried once.
/// - Elsewhere: none yet. The Linux platform roadmap adds its own here.
pub fn usual_places(os: Os, env: impl Fn(&str) -> Option<OsString>) -> Vec<PathBuf> {
    match os {
        Os::MacOs => {
            let system = inside_app(Path::new("/Applications").join("EV Nova.app"));
            let user = env("HOME")
                .filter(|home| !home.is_empty())
                .map(|home| inside_app(Path::new(&home).join("Applications").join("EV Nova.app")));
            std::iter::once(system).chain(user).collect()
        }
        Os::Windows => windows_places(env),
        Os::Other => Vec::new(),
    }
}

/// Windows' usual places, from `env`.
///
/// Each is the `Nova Files` folder, never the `EV Nova` folder above it:
/// the loader reads only the top level of the folder it is given, so the
/// `EV Nova` folder would "load" its interface file, `Nova.rez`, as the
/// game's data. EV Nova CE unzips to an `EV Nova` folder, which the
/// community keeps in `Program Files`; Ambrosia's own 32-bit installer's
/// folder isn't known, so its vendor folder is a guess, under both
/// `Program Files` roots. `%LOCALAPPDATA%\Programs` is Windows' place for
/// a program installed without an administrator.
fn windows_places(env: impl Fn(&str) -> Option<OsString>) -> Vec<PathBuf> {
    let var = |name: &str| {
        env(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let program_files: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)"]
        .into_iter()
        .filter_map(var)
        .collect();
    let vendors: [&[&str]; 2] = [&[], &["Ambrosia Software"]];
    let installed = vendors.into_iter().flat_map(|vendor| {
        program_files.iter().map(move |root| {
            let mut dir = root.clone();
            dir.extend(vendor);
            dir
        })
    });
    let per_user = var("LOCALAPPDATA").map(|local| local.join("Programs"));
    let mut places = Vec::new();
    for mut dir in installed.chain(per_user) {
        dir.extend(["EV Nova", "Nova Files"]);
        if !places.contains(&dir) {
            places.push(dir);
        }
    }
    places
}

/// Whether `path` names an app bundle: any `.app`, in any case.
fn is_app(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
}

/// The directory a pick names, before the folder levels of
/// [`pick_dirs`]: an `EV Nova.app` (any `.app`, in any case) is its
/// `Contents/Resources/Nova Files`; anything else is used as it is.
#[must_use]
pub fn resolve_pick(path: &Path) -> PathBuf {
    if is_app(path) {
        inside_app(path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

/// The directories a pick may name, in the order they are tried:
///
/// - An app ([`resolve_pick`]): its `Nova Files`, alone.
/// - A folder: its `Nova Files`, so the `EV Nova` folder above them can
///   be picked; then the folder itself, so the `Nova Files` can be. The
///   folder itself is left out when it holds a `Nova Files` or the
///   interface file `Nova.rez`: it is then an `EV Nova` folder, and the
///   loader, which reads only the top level of the folder it is given,
///   would "load" `Nova.rez` as the game's data.
fn pick_dirs(pick: &Path, source: &impl DataSource) -> Vec<PathBuf> {
    if is_app(pick) {
        return vec![resolve_pick(pick)];
    }
    let nova_files = pick.join("Nova Files");
    let ev_nova = source.has_entry(pick, "Nova Files") || source.has_entry(pick, "Nova.rez");
    if ev_nova {
        vec![nova_files]
    } else {
        vec![nova_files, pick.to_path_buf()]
    }
}

/// How the `Nova Files` were found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via {
    /// The argument or `NOVA_DATA`.
    Named,
    /// The settings' remembered directory.
    Remembered,
    /// One of the usual places.
    Usual,
    /// The player picked it when asked.
    Picked,
}

/// The `Nova Files` found, and their data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found<D> {
    /// The directory.
    pub dir: PathBuf,
    /// Its data, opened once.
    pub data: D,
    /// How it was found.
    pub via: Via,
}

/// No `Nova Files` were found anywhere, and none was picked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotFound {
    /// The usual places that were tried.
    pub places: Vec<PathBuf>,
}

impl fmt::Display for NotFound {
    /// Names the usual places tried, says none was chosen, and gives the
    /// usage line.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "nova: found no EV Nova data")?;
        let mut places = self.places.iter();
        if let Some(first) = places.next() {
            write!(f, " in {}", first.display())?;
            for place in places {
                write!(f, " or {}", place.display())?;
            }
        }
        write!(f, ", and no Nova Files folder was chosen; {Usage}")
    }
}

/// Why the lookup found no data to start with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotLocated {
    /// Nothing named, remembered, usual or picked held the data.
    NotFound(NotFound),
    /// The directory the player named could not be listed.
    Unreadable {
        /// The directory.
        dir: PathBuf,
        /// Why.
        error: String,
    },
}

/// What to do with the settings' remembered directory after the lookup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Remember {
    /// Leave it as it is.
    Keep,
    /// Drop it: it no longer holds the data.
    Forget,
    /// Remember this one, which the player picked.
    Remember(PathBuf),
}

/// What the lookup gives: the data or why there is none, the warnings to
/// report, and the change to the remembered directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located<D> {
    /// The data, or why there is none.
    pub outcome: Result<Found<D>, NotLocated>,
    /// One line each, for the startup report.
    pub warnings: Vec<String>,
    /// The change to the remembered directory.
    pub remember: Remember,
}

impl<D> Located<D> {
    fn ending(
        outcome: Result<Found<D>, NotLocated>,
        warnings: Vec<String>,
        remember: Remember,
    ) -> Self {
        Self {
            outcome,
            warnings,
            remember,
        }
    }
}

/// Finds the `Nova Files`, opening each candidate through `source`:
///
/// 1. `named` (the argument or `NOVA_DATA`) wins over everything: it is
///    used even when no file in it loads, as it always was, and one that
///    cannot be listed is [`NotLocated::Unreadable`]. Nothing else is
///    consulted.
/// 2. `remembered`, when it loads; one that no longer does is dropped
///    ([`Remember::Forget`]) with a warning naming it, and the lookup goes
///    on.
/// 3. The first of the `usual` places that loads. A usual place is a
///    guess, so one that doesn't load is skipped silently, and the one
///    found is not remembered: it is found again next launch.
/// 4. The player, through `dialog`. A pick names one or two directories
///    ([`pick_dirs`]: an app's `Nova Files`, or a folder's `Nova Files`
///    and then, unless it is an `EV Nova` folder, the folder itself), and
///    the first that can be listed decides: when it loads it is used and
///    remembered, and when it is empty the player is asked again, naming
///    it. When none can be listed the player is asked again, naming the
///    last tried. A cancel is [`NotLocated::NotFound`].
pub fn locate<S: DataSource>(
    named: Option<&Path>,
    remembered: Option<&Path>,
    usual: &[PathBuf],
    source: &S,
    dialog: &mut impl NovaFilesDialog,
) -> Located<S::Data> {
    let mut warnings = Vec::new();
    if let Some(dir) = named {
        let outcome = match source.open(dir) {
            Opened::Loaded(data) | Opened::Empty(data) => Ok(Found {
                dir: dir.to_path_buf(),
                data,
                via: Via::Named,
            }),
            Opened::Unreadable(error) => Err(NotLocated::Unreadable {
                dir: dir.to_path_buf(),
                error,
            }),
        };
        return Located::ending(outcome, warnings, Remember::Keep);
    }
    let mut remember = Remember::Keep;
    if let Some(dir) = remembered {
        let reason = match source.open(dir) {
            Opened::Loaded(data) => {
                let found = Found {
                    dir: dir.to_path_buf(),
                    data,
                    via: Via::Remembered,
                };
                return Located::ending(Ok(found), warnings, Remember::Keep);
            }
            Opened::Empty(_) => "no data file in them loads".to_owned(),
            Opened::Unreadable(error) => error,
        };
        warnings.push(format!(
            "nova: the remembered Nova Files {} no longer hold EV Nova data ({reason}); \
             forgetting them",
            dir.display()
        ));
        remember = Remember::Forget;
    }
    for dir in usual {
        if let Opened::Loaded(data) = source.open(dir) {
            let found = Found {
                dir: dir.clone(),
                data,
                via: Via::Usual,
            };
            return Located::ending(Ok(found), warnings, remember);
        }
    }
    let mut prompt = PROMPT.to_owned();
    while let Some(pick) = dialog.pick(&prompt) {
        let mut tried = pick.clone();
        for dir in pick_dirs(&pick, source) {
            match source.open(&dir) {
                Opened::Loaded(data) => {
                    let found = Found {
                        dir: dir.clone(),
                        data,
                        via: Via::Picked,
                    };
                    return Located::ending(Ok(found), warnings, Remember::Remember(dir));
                }
                Opened::Empty(_) => {
                    tried = dir;
                    break;
                }
                Opened::Unreadable(_) => tried = dir,
            }
        }
        prompt = format!("{} holds no EV Nova data. {PROMPT}", tried.display());
    }
    let not_found = NotLocated::NotFound(NotFound {
        places: usual.to_vec(),
    });
    Located::ending(Err(not_found), warnings, remember)
}

/// Applies the lookup's `change` to the remembered directory through
/// `keeper`; a warning when it could not be saved. A path whose name is
/// not UTF-8 is not saved (the settings file is JSON), with a warning.
/// With no keeper (nowhere to save the settings, which the game has
/// already warned of) it does nothing.
#[must_use]
pub fn apply_remember<S: SettingsStore>(
    keeper: Option<&mut SettingsKeeper<S>>,
    change: &Remember,
) -> Option<String> {
    let keeper = keeper?;
    let nova_files = match change {
        Remember::Keep => return None,
        Remember::Forget => None,
        Remember::Remember(dir) if dir.to_str().is_none() => {
            return Some(format!(
                "nova: cannot remember {}: its name is not UTF-8",
                dir.display()
            ));
        }
        Remember::Remember(dir) => Some(dir.clone()),
    };
    let settings = GameSettings {
        nova_files,
        ..keeper.settings()
    };
    keeper.change(settings).err()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::fs;

    use nova_audio::recording::MemorySettings;
    use nova_rsrc::ResType;
    use nova_rsrc::fixture::{ForkBuilder, RezBuilder};

    use super::*;

    /// What a mock directory holds.
    #[derive(Clone, Copy)]
    enum Holds {
        Data,
        Nothing,
        Unlistable,
    }

    /// Directories with canned contents, and files; any other directory is
    /// missing, so unlistable. Records each directory it is asked to open.
    /// The data is the directory.
    #[derive(Default)]
    struct Dirs {
        holding: HashMap<PathBuf, Holds>,
        files: HashSet<PathBuf>,
        asked: RefCell<Vec<PathBuf>>,
    }

    impl Dirs {
        fn with(mut self, dir: &Path, holds: Holds) -> Self {
            self.holding.insert(dir.to_path_buf(), holds);
            self
        }

        fn file(mut self, path: &Path) -> Self {
            self.files.insert(path.to_path_buf());
            self
        }

        fn asked(&self) -> Vec<PathBuf> {
            self.asked.borrow().clone()
        }
    }

    impl DataSource for Dirs {
        type Data = PathBuf;

        fn open(&self, dir: &Path) -> Opened<PathBuf> {
            self.asked.borrow_mut().push(dir.to_path_buf());
            match self.holding.get(dir) {
                Some(Holds::Data) => Opened::Loaded(dir.to_path_buf()),
                Some(Holds::Nothing) => Opened::Empty(dir.to_path_buf()),
                Some(Holds::Unlistable) | None => {
                    Opened::Unreadable("no such directory".to_owned())
                }
            }
        }

        fn has_entry(&self, dir: &Path, name: &str) -> bool {
            let entry = dir.join(name);
            self.holding.contains_key(&entry) || self.files.contains(&entry)
        }
    }

    /// A dialog that gives its picks in turn, then cancels; records each
    /// prompt.
    #[derive(Default)]
    struct Picks {
        picks: VecDeque<PathBuf>,
        prompts: Vec<String>,
    }

    impl Picks {
        fn of(picks: &[&Path]) -> Self {
            Self {
                picks: picks.iter().map(|pick| pick.to_path_buf()).collect(),
                prompts: Vec::new(),
            }
        }
    }

    impl NovaFilesDialog for Picks {
        fn pick(&mut self, prompt: &str) -> Option<PathBuf> {
            self.prompts.push(prompt.to_owned());
            self.picks.pop_front()
        }
    }

    fn path(text: &str) -> PathBuf {
        PathBuf::from(text)
    }

    fn usual() -> Vec<PathBuf> {
        vec![path("/usual/one"), path("/usual/two")]
    }

    /// An environment holding `vars`, and nothing else.
    fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let vars: Vec<(String, OsString)> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        move |name| {
            vars.iter()
                .find(|(var, _)| var == name)
                .map(|(_, value)| value.clone())
        }
    }

    fn nova_files_in(base: &str) -> PathBuf {
        let mut dir = PathBuf::from(base);
        dir.extend(["EV Nova.app", "Contents", "Resources", "Nova Files"]);
        dir
    }

    #[test]
    fn an_app_resolves_to_its_nova_files_in_any_case() {
        for app in ["EV Nova.app", "EV Nova.APP", "Other.App"] {
            let picked = Path::new("/Games").join(app);
            let mut expected = picked.clone();
            expected.extend(["Contents", "Resources", "Nova Files"]);
            assert_eq!(resolve_pick(&picked), expected, "{app}");
        }
    }

    #[test]
    fn a_folder_pick_is_used_as_it_is() {
        for dir in [
            "/Games/EV Nova/Nova Files",
            "/Games/apps",
            "/Games/x.application",
        ] {
            assert_eq!(resolve_pick(Path::new(dir)), path(dir));
        }
    }

    #[test]
    fn on_macos_the_usual_places_are_the_applications_folders() {
        let mut user = PathBuf::from("/Users/p");
        user.extend([
            "Applications",
            "EV Nova.app",
            "Contents",
            "Resources",
            "Nova Files",
        ]);
        assert_eq!(
            usual_places(Os::MacOs, env(&[("HOME", "/Users/p")])),
            [nova_files_in("/Applications"), user]
        );
    }

    #[test]
    fn without_a_home_only_the_system_applications_folder_is_usual() {
        for vars in [env(&[]), env(&[("HOME", "")])] {
            assert_eq!(
                usual_places(Os::MacOs, vars),
                [nova_files_in("/Applications")]
            );
        }
    }

    #[test]
    fn on_other_systems_there_are_no_usual_places_yet() {
        assert_eq!(
            usual_places(Os::Other, env(&[("HOME", "/home/p")])),
            Vec::<PathBuf>::new()
        );
    }

    /// The `Nova Files` of an `EV Nova` folder at `parts` under `root`.
    fn windows_place(root: &str, parts: &[&str]) -> PathBuf {
        let mut dir = Path::new(root).to_path_buf();
        dir.extend(parts);
        dir.extend(["EV Nova", "Nova Files"]);
        dir
    }

    const PROGRAM_FILES: (&str, &str) = ("ProgramFiles", "/pf");
    const PROGRAM_FILES_X86: (&str, &str) = ("ProgramFiles(x86)", "/pf86");
    const LOCAL_APP_DATA: (&str, &str) = ("LOCALAPPDATA", "/local");

    #[test]
    fn on_windows_the_usual_places_are_the_program_files_folders_then_the_users() {
        assert_eq!(
            usual_places(
                Os::Windows,
                env(&[PROGRAM_FILES, PROGRAM_FILES_X86, LOCAL_APP_DATA])
            ),
            [
                windows_place("/pf", &[]),
                windows_place("/pf86", &[]),
                windows_place("/pf", &["Ambrosia Software"]),
                windows_place("/pf86", &["Ambrosia Software"]),
                windows_place("/local", &["Programs"]),
            ]
        );
    }

    #[test]
    fn on_windows_a_missing_or_empty_variable_drops_its_places() {
        for vars in [
            env(&[PROGRAM_FILES_X86, LOCAL_APP_DATA]),
            env(&[("ProgramFiles", ""), PROGRAM_FILES_X86, LOCAL_APP_DATA]),
        ] {
            assert_eq!(
                usual_places(Os::Windows, vars),
                [
                    windows_place("/pf86", &[]),
                    windows_place("/pf86", &["Ambrosia Software"]),
                    windows_place("/local", &["Programs"]),
                ]
            );
        }
        assert_eq!(
            usual_places(
                Os::Windows,
                env(&[
                    PROGRAM_FILES,
                    ("ProgramFiles(x86)", ""),
                    ("LOCALAPPDATA", "")
                ])
            ),
            [
                windows_place("/pf", &[]),
                windows_place("/pf", &["Ambrosia Software"]),
            ]
        );
        assert_eq!(
            usual_places(Os::Windows, env(&[LOCAL_APP_DATA])),
            [windows_place("/local", &["Programs"])]
        );
    }

    #[test]
    fn on_32_bit_windows_the_one_program_files_folder_is_tried_once() {
        assert_eq!(
            usual_places(
                Os::Windows,
                env(&[PROGRAM_FILES, ("ProgramFiles(x86)", "/pf"), LOCAL_APP_DATA])
            ),
            [
                windows_place("/pf", &[]),
                windows_place("/pf", &["Ambrosia Software"]),
                windows_place("/local", &["Programs"]),
            ]
        );
    }

    #[test]
    fn on_windows_with_no_variables_there_are_no_usual_places() {
        assert_eq!(
            usual_places(Os::Windows, env(&[("HOME", "/home/p")])),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn a_named_directory_wins_and_nothing_else_is_consulted() {
        let named = path("/named");
        let dirs = Dirs::default()
            .with(&named, Holds::Data)
            .with(Path::new("/remembered"), Holds::Data)
            .with(Path::new("/usual/one"), Holds::Data);
        let mut dialog = Picks::of(&[Path::new("/picked")]);
        let located = locate(
            Some(&named),
            Some(Path::new("/remembered")),
            &usual(),
            &dirs,
            &mut dialog,
        );
        assert_eq!(
            located,
            Located {
                outcome: Ok(Found {
                    dir: named.clone(),
                    data: named.clone(),
                    via: Via::Named,
                }),
                warnings: Vec::new(),
                remember: Remember::Keep,
            }
        );
        assert_eq!(dirs.asked(), [named]);
        assert_eq!(dialog.prompts, Vec::<String>::new());
    }

    #[test]
    fn a_named_directory_with_no_data_is_still_used() {
        let named = path("/named");
        let dirs = Dirs::default()
            .with(&named, Holds::Nothing)
            .with(Path::new("/usual/one"), Holds::Data);
        let mut dialog = Picks::default();
        let located = locate(Some(&named), None, &usual(), &dirs, &mut dialog);
        assert_eq!(located.outcome.map(|found| found.dir), Ok(named.clone()));
        assert_eq!(located.remember, Remember::Keep);
        assert_eq!(dirs.asked(), [named]);
        assert_eq!(dialog.prompts, Vec::<String>::new());
    }

    #[test]
    fn a_named_directory_that_cannot_be_listed_is_unreadable() {
        let named = path("/named");
        let dirs = Dirs::default()
            .with(&named, Holds::Unlistable)
            .with(Path::new("/usual/one"), Holds::Data);
        let mut dialog = Picks::of(&[Path::new("/usual/one")]);
        let located = locate(
            Some(&named),
            Some(Path::new("/usual/one")),
            &usual(),
            &dirs,
            &mut dialog,
        );
        assert_eq!(
            located.outcome,
            Err(NotLocated::Unreadable {
                dir: named.clone(),
                error: "no such directory".to_owned()
            })
        );
        assert_eq!(located.warnings, Vec::<String>::new());
        assert_eq!(located.remember, Remember::Keep);
        assert_eq!(dirs.asked(), [named]);
        assert_eq!(dialog.prompts, Vec::<String>::new());
    }

    #[test]
    fn a_remembered_directory_that_loads_is_kept_quietly() {
        let remembered = path("/remembered");
        let dirs = Dirs::default()
            .with(&remembered, Holds::Data)
            .with(Path::new("/usual/one"), Holds::Data);
        let mut dialog = Picks::default();
        let located = locate(None, Some(&remembered), &usual(), &dirs, &mut dialog);
        assert_eq!(
            located,
            Located {
                outcome: Ok(Found {
                    dir: remembered.clone(),
                    data: remembered.clone(),
                    via: Via::Remembered,
                }),
                warnings: Vec::new(),
                remember: Remember::Keep,
            }
        );
        assert_eq!(dirs.asked(), [remembered]);
    }

    #[test]
    fn a_remembered_directory_that_no_longer_loads_is_forgotten_with_a_warning() {
        let remembered = Path::new("/old").join("Nova Files");
        for (holds, reason) in [
            (Holds::Nothing, "no data file in them loads"),
            (Holds::Unlistable, "no such directory"),
        ] {
            let dirs = Dirs::default()
                .with(&remembered, holds)
                .with(Path::new("/usual/one"), Holds::Data);
            let mut dialog = Picks::default();
            let located = locate(None, Some(&remembered), &usual(), &dirs, &mut dialog);
            assert_eq!(
                located,
                Located {
                    outcome: Ok(Found {
                        dir: path("/usual/one"),
                        data: path("/usual/one"),
                        via: Via::Usual,
                    }),
                    warnings: vec![format!(
                        "nova: the remembered Nova Files {} no longer hold EV Nova data \
                         ({reason}); forgetting them",
                        remembered.display()
                    )],
                    remember: Remember::Forget,
                },
                "{reason}"
            );
            assert_eq!(dirs.asked(), [remembered.clone(), path("/usual/one")]);
        }
    }

    #[test]
    fn a_usual_place_that_does_not_load_is_skipped_silently() {
        for holds in [Holds::Nothing, Holds::Unlistable] {
            let dirs = Dirs::default()
                .with(Path::new("/usual/one"), holds)
                .with(Path::new("/usual/two"), Holds::Data);
            let mut dialog = Picks::default();
            let located = locate(None, None, &usual(), &dirs, &mut dialog);
            assert_eq!(
                located,
                Located {
                    outcome: Ok(Found {
                        dir: path("/usual/two"),
                        data: path("/usual/two"),
                        via: Via::Usual,
                    }),
                    warnings: Vec::new(),
                    remember: Remember::Keep,
                }
            );
            assert_eq!(dirs.asked(), usual());
            assert_eq!(dialog.prompts, Vec::<String>::new());
        }
    }

    #[test]
    fn with_nothing_found_the_player_is_asked_and_a_picked_app_remembered() {
        let picked_app = Path::new("/Games").join("EV Nova.app");
        let resolved = resolve_pick(&picked_app);
        let dirs = Dirs::default().with(&resolved, Holds::Data);
        let mut dialog = Picks::of(&[&picked_app]);
        let located = locate(None, None, &usual(), &dirs, &mut dialog);
        assert_eq!(
            located,
            Located {
                outcome: Ok(Found {
                    dir: resolved.clone(),
                    data: resolved.clone(),
                    via: Via::Picked,
                }),
                warnings: Vec::new(),
                remember: Remember::Remember(resolved.clone()),
            }
        );
        assert_eq!(dialog.prompts, ["Locate your EV Nova folder"]);
        assert_eq!(
            dirs.asked(),
            [path("/usual/one"), path("/usual/two"), resolved]
        );
    }

    #[test]
    fn a_bad_pick_asks_again_naming_it() {
        let bad = path("/Games/Empty");
        let good = path("/Games/Nova Files");
        let dirs = Dirs::default()
            .with(&bad, Holds::Nothing)
            .with(&good, Holds::Data);
        let mut dialog = Picks::of(&[&bad, &good]);
        let located = locate(None, None, &[], &dirs, &mut dialog);
        assert_eq!(located.outcome.map(|found| found.dir), Ok(good.clone()));
        assert_eq!(located.remember, Remember::Remember(good));
        assert_eq!(
            dialog.prompts,
            [
                "Locate your EV Nova folder".to_owned(),
                format!(
                    "{} holds no EV Nova data. Locate your EV Nova folder",
                    bad.display()
                )
            ]
        );
    }

    /// `X/EV Nova` and its `Nova Files`.
    fn ev_nova_folder() -> (PathBuf, PathBuf) {
        let ev_nova = Path::new("/X").join("EV Nova");
        let nova_files = ev_nova.join("Nova Files");
        (ev_nova, nova_files)
    }

    fn reask(dir: &Path) -> String {
        format!(
            "{} holds no EV Nova data. Locate your EV Nova folder",
            dir.display()
        )
    }

    #[test]
    fn picking_the_ev_nova_folder_finds_and_remembers_its_nova_files() {
        let (ev_nova, nova_files) = ev_nova_folder();
        let dirs = Dirs::default()
            .with(&ev_nova, Holds::Data)
            .with(&nova_files, Holds::Data)
            .file(&ev_nova.join("Nova.rez"));
        let mut dialog = Picks::of(&[&ev_nova]);
        let located = locate(None, None, &[], &dirs, &mut dialog);
        assert_eq!(
            located,
            Located {
                outcome: Ok(Found {
                    dir: nova_files.clone(),
                    data: nova_files.clone(),
                    via: Via::Picked,
                }),
                warnings: Vec::new(),
                remember: Remember::Remember(nova_files.clone()),
            }
        );
        assert_eq!(dirs.asked(), [nova_files]);
    }

    #[test]
    fn picking_the_nova_files_folder_itself_still_finds_it() {
        let (_, nova_files) = ev_nova_folder();
        let dirs = Dirs::default().with(&nova_files, Holds::Data);
        let mut dialog = Picks::of(&[&nova_files]);
        let located = locate(None, None, &[], &dirs, &mut dialog);
        assert_eq!(
            located.outcome,
            Ok(Found {
                dir: nova_files.clone(),
                data: nova_files.clone(),
                via: Via::Picked,
            })
        );
        assert_eq!(located.remember, Remember::Remember(nova_files.clone()));
        assert_eq!(
            dirs.asked(),
            [nova_files.join("Nova Files"), nova_files.clone()]
        );
    }

    #[test]
    fn an_ev_nova_folder_whose_nova_files_are_empty_asks_again_naming_them() {
        let (ev_nova, nova_files) = ev_nova_folder();
        let dirs = Dirs::default()
            .with(&ev_nova, Holds::Data)
            .with(&nova_files, Holds::Nothing);
        let mut dialog = Picks::of(&[&ev_nova]);
        let located = locate(None, None, &[], &dirs, &mut dialog);
        assert_eq!(
            located.outcome,
            Err(NotLocated::NotFound(NotFound { places: Vec::new() }))
        );
        assert_eq!(located.remember, Remember::Keep);
        assert_eq!(dialog.prompts, [PROMPT.to_owned(), reask(&nova_files)]);
        assert_eq!(dirs.asked(), [nova_files]);
    }

    #[test]
    fn an_ev_nova_folder_with_its_interface_file_but_no_nova_files_asks_again() {
        let (ev_nova, nova_files) = ev_nova_folder();
        let dirs = Dirs::default()
            .with(&ev_nova, Holds::Data)
            .file(&ev_nova.join("Nova.rez"));
        let mut dialog = Picks::of(&[&ev_nova]);
        let located = locate(None, None, &[], &dirs, &mut dialog);
        assert_eq!(
            located.outcome,
            Err(NotLocated::NotFound(NotFound { places: Vec::new() }))
        );
        assert_eq!(located.remember, Remember::Keep);
        assert_eq!(dialog.prompts, [PROMPT.to_owned(), reask(&nova_files)]);
        assert_eq!(
            dirs.asked(),
            [nova_files],
            "the EV Nova folder isn't opened"
        );
    }

    #[test]
    fn an_ev_nova_folder_whose_nova_files_cannot_be_listed_asks_again() {
        let (ev_nova, nova_files) = ev_nova_folder();
        let dirs = Dirs::default()
            .with(&ev_nova, Holds::Data)
            .with(&nova_files, Holds::Unlistable);
        let mut dialog = Picks::of(&[&ev_nova]);
        let located = locate(None, None, &[], &dirs, &mut dialog);
        assert_eq!(
            located.outcome,
            Err(NotLocated::NotFound(NotFound { places: Vec::new() }))
        );
        assert_eq!(located.remember, Remember::Keep);
        assert_eq!(dialog.prompts, [PROMPT.to_owned(), reask(&nova_files)]);
        assert_eq!(
            dirs.asked(),
            [nova_files],
            "the EV Nova folder isn't opened"
        );
    }

    #[test]
    fn a_pick_that_cannot_be_listed_asks_again_naming_it() {
        let pick = Path::new("/Games").join("Gone");
        let dirs = Dirs::default();
        let mut dialog = Picks::of(&[&pick]);
        let located = locate(None, None, &[], &dirs, &mut dialog);
        assert!(located.outcome.is_err());
        assert_eq!(dialog.prompts, [PROMPT.to_owned(), reask(&pick)]);
        assert_eq!(dirs.asked(), [pick.join("Nova Files"), pick]);
    }

    #[test]
    fn a_cancel_finds_nothing_and_still_forgets_a_gone_remembered_directory() {
        let dirs = Dirs::default();
        let mut dialog = Picks::default();
        let located = locate(None, Some(Path::new("/gone")), &usual(), &dirs, &mut dialog);
        assert_eq!(
            located.outcome,
            Err(NotLocated::NotFound(NotFound { places: usual() }))
        );
        assert_eq!(located.remember, Remember::Forget);
        assert_eq!(located.warnings.len(), 1);
        assert_eq!(dialog.prompts, ["Locate your EV Nova folder"]);
    }

    #[test]
    fn not_found_names_the_usual_places_and_the_usage() {
        let places = usual();
        let message = NotFound {
            places: places.clone(),
        }
        .to_string();
        assert_eq!(
            message,
            format!(
                "nova: found no EV Nova data in {} or {}, and no Nova Files folder was chosen; {}",
                places[0].display(),
                places[1].display(),
                Usage
            )
        );
        assert_eq!(
            NotFound { places: Vec::new() }.to_string(),
            format!("nova: found no EV Nova data, and no Nova Files folder was chosen; {Usage}")
        );
    }

    /// A data file the loader reads: a flattened fork, as data-fork files
    /// are.
    fn data_file() -> Vec<u8> {
        ForkBuilder::new()
            .resource(ResType(*b"STR#"), 128, None, &[0, 0])
            .build()
            .bytes
    }

    #[test]
    fn a_directory_holding_a_loadable_data_file_is_loaded() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        fs::write(dir.path().join("Nova Data 1.ndat"), data_file()).expect("writes");
        match DiskData.open(dir.path()) {
            Opened::Loaded(data) => assert_eq!(data.files().len(), 1),
            other => panic!("loaded: {:?}", kind(&other)),
        }
    }

    #[test]
    fn a_directory_whose_files_do_not_load_is_empty() {
        for contents in [&b""[..], &b"not a resource file"[..]] {
            let dir = tempfile::tempdir().expect("a temporary directory");
            fs::write(dir.path().join("Nova Data 1.ndat"), contents).expect("writes");
            assert_eq!(kind(&DiskData.open(dir.path())), "empty", "{contents:?}");
        }
        let dir = tempfile::tempdir().expect("a temporary directory");
        assert_eq!(kind(&DiskData.open(dir.path())), "empty", "no files");
    }

    #[test]
    fn a_missing_directory_is_unreadable() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let missing = dir.path().join("Nova Files");
        match DiskData.open(&missing) {
            Opened::Unreadable(error) => {
                assert!(error.contains(&missing.display().to_string()), "{error}");
            }
            other => panic!("unreadable: {:?}", kind(&other)),
        }
    }

    /// A Windows `.rez` file the loader reads.
    fn rez_file() -> Vec<u8> {
        RezBuilder::new()
            .resource(ResType(*b"STR#"), 128, None, &[0, 0])
            .build()
            .bytes
    }

    /// Lays out an `EV Nova` folder as EV Nova CE unzips it, under `root`:
    /// its interface file, and a `Nova Files` holding `files`.
    fn ce_install(root: &Path, files: &[(&str, Vec<u8>)]) -> PathBuf {
        let ev_nova = root.join("EV Nova");
        let nova_files = ev_nova.join("Nova Files");
        fs::create_dir_all(&nova_files).expect("makes the folders");
        fs::write(ev_nova.join("Nova.rez"), rez_file()).expect("writes");
        for (name, bytes) in files {
            fs::write(nova_files.join(name), bytes).expect("writes");
        }
        nova_files
    }

    /// A fake Windows environment whose three roots are in `tmp`.
    fn windows_env(tmp: &Path) -> impl Fn(&str) -> Option<OsString> + use<> {
        let roots = [
            ("ProgramFiles", tmp.join("pf")),
            ("ProgramFiles(x86)", tmp.join("pf86")),
            ("LOCALAPPDATA", tmp.join("local")),
        ];
        move |name| {
            roots
                .iter()
                .find(|(var, _)| *var == name)
                .map(|(_, dir)| dir.clone().into_os_string())
        }
    }

    #[test]
    fn a_ce_install_in_program_files_x86_is_found_on_disk_without_asking() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        fs::create_dir_all(tmp.path().join("pf")).expect("makes the folder");
        let nova_files = ce_install(&tmp.path().join("pf86"), &[("Nova Data 1.rez", rez_file())]);
        let usual = usual_places(Os::Windows, windows_env(tmp.path()));
        let mut dialog = Picks::default();
        let located = locate(None, None, &usual, &DiskData, &mut dialog);
        let found = located.outcome.expect("found");
        assert_eq!((found.dir, found.via), (nova_files, Via::Usual));
        assert_eq!(found.data.files().len(), 1);
        assert_eq!(located.remember, Remember::Keep);
        assert_eq!(dialog.prompts, Vec::<String>::new());
    }

    #[test]
    fn a_nova_files_with_only_music_and_movies_is_not_a_usual_place() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        ce_install(
            &tmp.path().join("pf86"),
            &[
                ("Nova Music.mp3", b"ID3\x04\0\0\0\0\0\0".to_vec()),
                ("Race 1.mov", b"\0\0\0\x14ftypqt  ".to_vec()),
            ],
        );
        let per_user = ce_install(
            &tmp.path().join("local").join("Programs"),
            &[("Nova Data 1.rez", rez_file())],
        );
        let usual = usual_places(Os::Windows, windows_env(tmp.path()));
        let mut dialog = Picks::default();
        let located = locate(None, None, &usual, &DiskData, &mut dialog);
        let found = located.outcome.expect("found");
        assert_eq!((found.dir, found.via), (per_user, Via::Usual));
        assert_eq!(dialog.prompts, Vec::<String>::new());
    }

    #[test]
    fn an_entry_on_disk_is_there_whether_a_file_or_a_folder() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        fs::write(tmp.path().join("Nova.rez"), b"").expect("writes");
        fs::create_dir(tmp.path().join("Nova Files")).expect("makes the folder");
        assert!(DiskData.has_entry(tmp.path(), "Nova.rez"));
        assert!(DiskData.has_entry(tmp.path(), "Nova Files"));
        assert!(!DiskData.has_entry(tmp.path(), "Plug-ins"));
        assert!(!DiskData.has_entry(&tmp.path().join("gone"), "Nova Files"));
    }

    #[test]
    fn a_picked_ce_folder_on_disk_gives_its_nova_files() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let nova_files = ce_install(tmp.path(), &[("Nova Data 1.rez", rez_file())]);
        let ev_nova = tmp.path().join("EV Nova");
        let mut dialog = Picks::of(&[&ev_nova]);
        let located = locate(None, None, &[], &DiskData, &mut dialog);
        assert_eq!(located.remember, Remember::Remember(nova_files.clone()));
        assert_eq!(located.outcome.map(|found| found.dir), Ok(nova_files));
    }

    #[test]
    fn a_picked_ce_folder_on_disk_with_no_nova_files_is_not_loaded() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let ev_nova = tmp.path().join("EV Nova");
        fs::create_dir(&ev_nova).expect("makes the folder");
        fs::write(ev_nova.join("Nova.rez"), rez_file()).expect("writes");
        assert_eq!(kind(&DiskData.open(&ev_nova)), "loaded", "the trap");
        let mut dialog = Picks::of(&[&ev_nova]);
        let located = locate(None, None, &[], &DiskData, &mut dialog);
        assert!(located.outcome.is_err());
        assert_eq!(located.remember, Remember::Keep);
        assert_eq!(dialog.prompts.len(), 2, "asked again");
    }

    fn kind<D>(opened: &Opened<D>) -> &'static str {
        match opened {
            Opened::Loaded(_) => "loaded",
            Opened::Empty(_) => "empty",
            Opened::Unreadable(_) => "unreadable",
        }
    }

    fn keeper_over(store: &MemorySettings) -> SettingsKeeper<MemorySettings> {
        SettingsKeeper::open(store.clone()).0
    }

    #[test]
    fn a_pick_is_remembered_in_the_settings() {
        let store = MemorySettings::holding(r#"{"rules": "bible"}"#);
        let mut keeper = keeper_over(&store);
        let dir = Path::new("/Games").join("Nova Files");
        let warning = apply_remember(Some(&mut keeper), &Remember::Remember(dir.clone()));
        assert_eq!(warning, None);
        assert_eq!(keeper_over(&store).settings().nova_files, Some(dir));
        assert!(store.text().expect("saved").contains("bible"));
    }

    #[test]
    fn a_gone_directory_is_forgotten_in_the_settings() {
        let store = MemorySettings::holding(r#"{"nova_files": "/gone"}"#);
        let mut keeper = keeper_over(&store);
        assert_eq!(apply_remember(Some(&mut keeper), &Remember::Forget), None);
        assert_eq!(store.writes(), 1);
        assert_eq!(keeper_over(&store).settings().nova_files, None);
    }

    #[test]
    fn keeping_writes_nothing() {
        let store = MemorySettings::holding(r#"{"nova_files": "/kept"}"#);
        let mut keeper = keeper_over(&store);
        assert_eq!(apply_remember(Some(&mut keeper), &Remember::Keep), None);
        assert_eq!(store.writes(), 0);
        assert_eq!(
            keeper_over(&store).settings().nova_files,
            Some(path("/kept"))
        );
    }

    #[test]
    fn a_failed_save_is_a_warning() {
        let store = MemorySettings::new();
        let mut keeper = keeper_over(&store);
        store.fail_writes(true);
        let warning = apply_remember(Some(&mut keeper), &Remember::Remember(path("/x")));
        assert_eq!(
            warning,
            Some("nova: cannot save the settings: the disk is full".to_owned())
        );
    }

    #[test]
    fn with_nowhere_to_save_nothing_is_said() {
        for change in [
            Remember::Keep,
            Remember::Forget,
            Remember::Remember(path("/x")),
        ] {
            assert_eq!(apply_remember::<MemorySettings>(None, &change), None);
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_pick_whose_name_is_not_utf8_is_not_remembered() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let store = MemorySettings::new();
        let mut keeper = keeper_over(&store);
        let dir = PathBuf::from(OsStr::from_bytes(b"/Nova \xff Files"));
        let warning = apply_remember(Some(&mut keeper), &Remember::Remember(dir.clone()));
        assert_eq!(
            warning,
            Some(format!(
                "nova: cannot remember {}: its name is not UTF-8",
                dir.display()
            ))
        );
        assert_eq!(store.writes(), 0);
    }
}
