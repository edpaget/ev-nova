//! The startup sequence, end to end over mocks of every port: the
//! command line, the settings file, the data on disk, the open panel,
//! stderr's terminal, the startup log and the error alert.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use nova::platform::console::{ConsoleTerminal, StderrGoes};
use nova::ruleset::Ruleset;
use nova::startup::{
    Answer, DataSource, FatalAlert, Launch, NewGameLoaded, NewGameOpened, NovaFilesDialog, Opened,
    Started, StartupLog, Terminal, resolve_pick, run,
};
use nova_audio::SettingsStore;
use nova_audio::recording::MemorySettings;

/// Directories holding the game's data; every other one is empty, and
/// holds nothing else. Records each directory asked for. The new game's
/// folder opens as `new_game` says, and each one asked for is recorded
/// apart.
#[derive(Default)]
struct Disk {
    holding: Vec<PathBuf>,
    asked: RefCell<Vec<PathBuf>>,
    new_game: Option<NewGameOpened<PathBuf>>,
    new_game_asked: RefCell<Vec<PathBuf>>,
}

impl DataSource for Disk {
    type Data = PathBuf;

    fn open(&self, dir: &Path) -> Opened<PathBuf> {
        self.asked.borrow_mut().push(dir.to_path_buf());
        if self.holding.iter().any(|held| held == dir) {
            Opened::Loaded(dir.to_path_buf())
        } else {
            Opened::Empty(dir.to_path_buf())
        }
    }

    fn has_entry(&self, dir: &Path, name: &str) -> bool {
        let entry = dir.join(name);
        self.holding.contains(&entry)
    }

    /// The canned outcome, else the folder itself as its data, with one
    /// system and nothing wrong.
    fn open_new_game(&self, dir: &Path) -> NewGameOpened<PathBuf> {
        self.new_game_asked.borrow_mut().push(dir.to_path_buf());
        self.new_game
            .clone()
            .unwrap_or_else(|| NewGameOpened::Loaded(loaded(dir.to_path_buf())))
    }
}

/// A new game's folder that loaded `data`, one system, with nothing wrong.
fn loaded(data: PathBuf) -> NewGameLoaded<PathBuf> {
    NewGameLoaded {
        data,
        problems: Vec::new(),
        lfs_pointers: Vec::new(),
        systems: 1,
    }
}

/// The open panel: its answers in turn, then a cancel; records each
/// prompt.
#[derive(Default)]
struct Panel {
    answers: VecDeque<Answer>,
    prompts: Vec<String>,
}

impl NovaFilesDialog for Panel {
    fn pick(&mut self, prompt: &str) -> Answer {
        self.prompts.push(prompt.to_owned());
        self.answers.pop_front().unwrap_or(Answer::Cancelled)
    }
}

struct Stderr {
    terminal: bool,
}

impl Terminal for Stderr {
    fn stderr_is_terminal(&self) -> bool {
        self.terminal
    }
}

#[derive(Clone, Default)]
struct Log(Rc<RefCell<Vec<String>>>);

impl StartupLog for Log {
    fn write_line(&mut self, line: &str) -> io::Result<()> {
        self.0.borrow_mut().push(line.to_owned());
        Ok(())
    }
}

/// A stderr whose bytes the test can read.
#[derive(Clone, Default)]
struct Written(Rc<RefCell<Vec<u8>>>);

impl Written {
    fn text(&self) -> String {
        String::from_utf8(self.0.borrow().clone()).expect("UTF-8")
    }
}

impl io::Write for Written {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct Alerts(Vec<String>);

impl FatalAlert for Alerts {
    fn show_fatal(&mut self, message: &str) {
        self.0.push(message.to_owned());
    }
}

/// One launch, with no argument, `NOVA_DATA` or `GAME_DATA` unless a test
/// sets them.
struct Machine {
    args: Vec<OsString>,
    nova_data: Option<OsString>,
    game_data: Option<OsString>,
    disk: Disk,
    panel: Panel,
    alerts: Alerts,
    log: Log,
    store: MemorySettings,
    terminal: bool,
    /// Where a Windows console says stderr goes, over `terminal`.
    console: Option<StderrGoes>,
    stderr: Written,
}

impl Machine {
    fn new(store: &MemorySettings, holding: &[&Path], picks: &[&Path]) -> Self {
        Self {
            args: Vec::new(),
            nova_data: None,
            game_data: None,
            disk: Disk {
                holding: holding.iter().map(|dir| dir.to_path_buf()).collect(),
                ..Disk::default()
            },
            panel: Panel {
                answers: picks
                    .iter()
                    .map(|pick| Answer::Picked(pick.to_path_buf()))
                    .collect(),
                prompts: Vec::new(),
            },
            alerts: Alerts::default(),
            log: Log::default(),
            store: store.clone(),
            terminal: false,
            console: None,
            stderr: Written::default(),
        }
    }

    /// The same launch, its panel quit (a quit Apple event) after its
    /// picks.
    fn then_quit(mut self) -> Self {
        self.panel.answers.push_back(Answer::Quit);
        self
    }

    fn launch(&mut self) -> Result<Started<(), PathBuf>, u8> {
        let stderr = Stderr {
            terminal: self.terminal,
        };
        let console = self.console.map(ConsoleTerminal);
        let terminal: &dyn Terminal = match &console {
            Some(console) => console,
            None => &stderr,
        };
        run(
            Launch {
                args: self.args.clone(),
                nova_data: self.nova_data.clone(),
                game_data: self.game_data.clone(),
                usual: usual(),
                terminal,
                log: Some(Box::new(self.log.clone())),
                stderr: Box::new(self.stderr.clone()),
                settings: Some(Box::new(self.store.clone()) as Box<dyn SettingsStore>),
                source: &self.disk,
                dialog: &mut self.panel,
                alert: &mut self.alerts,
            },
            || Ok(()),
        )
    }
}

fn usual() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/Applications/Nova"),
        PathBuf::from("/Users/p/Nova"),
    ]
}

#[test]
fn with_nothing_named_remembered_or_usual_the_player_is_asked() {
    let mut machine = Machine::new(&MemorySettings::new(), &[], &[]);
    let started = machine.launch();
    assert!(started.is_err(), "nothing was picked");
    assert_eq!(machine.panel.prompts, ["Locate your EV Nova folder"]);
    assert_eq!(*machine.disk.asked.borrow(), usual());
}

#[test]
fn a_good_pick_is_remembered_and_tried_first_next_launch() {
    let store = MemorySettings::new();
    let app = Path::new("/Games").join("EV Nova.app");
    let nova_files = resolve_pick(&app);
    let mut first = Machine::new(&store, &[&nova_files], &[&app]);
    let started = first.launch().expect("starts");
    assert_eq!(started.dir, nova_files);
    assert_eq!(started.data, nova_files);

    let mut second = Machine::new(&store, &[&nova_files], &[]);
    let started = second.launch().expect("starts");
    assert_eq!(started.dir, nova_files);
    assert_eq!(*second.disk.asked.borrow(), [nova_files]);
    assert_eq!(
        second.panel.prompts,
        Vec::<String>::new(),
        "not asked again"
    );
}

#[test]
fn a_picked_ev_nova_folder_remembers_its_nova_files() {
    let store = MemorySettings::new();
    let ev_nova = Path::new("/Games").join("EV Nova");
    let nova_files = ev_nova.join("Nova Files");
    let mut first = Machine::new(&store, &[&nova_files], &[&ev_nova]);
    let started = first.launch().expect("starts");
    assert_eq!(started.dir, nova_files);

    let mut second = Machine::new(&store, &[&nova_files], &[]);
    assert_eq!(second.launch().expect("starts").dir, nova_files);
    assert_eq!(*second.disk.asked.borrow(), [nova_files]);
    assert_eq!(second.panel.prompts, Vec::<String>::new());
}

#[test]
fn a_cancel_without_a_terminal_shows_the_fatal_alert() {
    let mut machine = Machine::new(&MemorySettings::new(), &[], &[]);
    let code = machine.launch().err().expect("fails");
    assert_eq!(code, 2);
    let [message] = machine.alerts.0.as_slice() else {
        panic!("one alert: {:?}", machine.alerts.0);
    };
    assert!(
        message.starts_with("nova: found no EV Nova data in "),
        "{message}"
    );
    assert_eq!(machine.log.0.borrow().last(), Some(message), "logged too");
}

/// A store remembering `dir`.
fn remembering(dir: &Path) -> MemorySettings {
    let store = MemorySettings::new();
    store
        .clone()
        .write(&format!(
            "{{\"nova_files\": {}}}",
            serde_json::to_string(&dir.display().to_string()).expect("JSON")
        ))
        .expect("seeds");
    store
}

/// The line a Cancel in the panel ends a launch with, from the same
/// machine as `quit` but cancelled there: what a quit must end with too.
fn cancel_line(store: &MemorySettings, picks: &[&Path]) -> String {
    let mut cancelled = Machine::new(store, &[], picks);
    cancelled.terminal = true;
    assert_eq!(cancelled.launch().err(), Some(2));
    cancelled.log.0.borrow().last().cloned().expect("logged")
}

#[test]
fn a_quit_during_the_panel_ends_the_startup_with_the_no_data_status_and_no_alert() {
    let remembered = Path::new("/Games").join("Gone");
    let store = remembering(&remembered);
    let mut machine = Machine::new(&store, &[], &[]).then_quit();
    assert_eq!(machine.launch().err(), Some(2), "the no-data status");
    assert_eq!(
        machine.alerts.0,
        Vec::<String>::new(),
        "no alert, though stderr is no terminal"
    );
    assert_eq!(machine.panel.prompts, ["Locate your EV Nova folder"]);
    let line = cancel_line(&remembering(&remembered), &[]);
    assert!(
        line.starts_with("nova: found no EV Nova data in "),
        "{line}"
    );
    assert_eq!(
        machine.log.0.borrow().last(),
        Some(&line),
        "the Cancel's line"
    );
    assert!(
        machine.stderr.text().ends_with(&format!("{line}\n")),
        "{}",
        machine.stderr.text()
    );
    assert!(
        !store.text().expect("saved").contains("nova_files"),
        "a gone remembered directory is still forgotten, as a Cancel does"
    );
}

#[test]
fn a_quit_with_nothing_remembered_saves_nothing() {
    let store = MemorySettings::new();
    let mut machine = Machine::new(&store, &[], &[]).then_quit();
    assert_eq!(machine.launch().err(), Some(2));
    assert_eq!(store.writes(), 0, "nothing saved");
}

#[test]
fn a_quit_at_a_reask_after_an_empty_pick_ends_the_startup_too() {
    let empty = Path::new("/Games").join("Empty");
    let store = MemorySettings::new();
    let mut machine = Machine::new(&store, &[], &[&empty]).then_quit();
    assert_eq!(machine.launch().err(), Some(2));
    assert_eq!(machine.panel.prompts.len(), 2, "asked, then asked again");
    assert_eq!(machine.alerts.0, Vec::<String>::new());
    let line = cancel_line(&MemorySettings::new(), &[&empty]);
    assert_eq!(machine.log.0.borrow().last(), Some(&line));
    assert_eq!(store.writes(), 0, "nothing remembered");
}

#[test]
fn a_cancel_from_a_terminal_shows_no_alert() {
    let mut machine = Machine::new(&MemorySettings::new(), &[], &[]);
    machine.terminal = true;
    assert_eq!(machine.launch().err(), Some(2));
    assert_eq!(machine.alerts.0, Vec::<String>::new());
}

#[test]
fn a_windows_stderr_going_nowhere_shows_the_fatal_error_in_the_box_and_the_log() {
    let mut machine = Machine::new(&MemorySettings::new(), &[], &[]);
    machine.console = Some(StderrGoes::Nowhere);
    assert_eq!(machine.launch().err(), Some(2));
    let [message] = machine.alerts.0.as_slice() else {
        panic!("one box: {:?}", machine.alerts.0);
    };
    assert!(message.starts_with("nova: found no EV Nova data in "));
    assert_eq!(machine.log.0.borrow().last(), Some(message), "logged too");
}

#[test]
fn a_windows_stderr_given_or_attached_gets_the_fatal_error_and_no_box() {
    for goes in [StderrGoes::Redirected, StderrGoes::Attached] {
        let mut machine = Machine::new(&MemorySettings::new(), &[], &[]);
        machine.console = Some(goes);
        assert_eq!(machine.launch().err(), Some(2), "{goes:?}");
        assert_eq!(machine.alerts.0, Vec::<String>::new(), "{goes:?}");
        let logged = machine.log.0.borrow().last().cloned().expect("logged");
        assert!(logged.starts_with("nova: found no EV Nova data in "));
        assert_eq!(machine.stderr.text(), format!("{logged}\n"), "{goes:?}");
    }
}

#[test]
fn a_remembered_directory_that_no_longer_loads_gives_one_warnings_alert() {
    let gone = Path::new("/Games").join("Moved");
    let store = MemorySettings::new();
    store
        .clone()
        .write(&format!(
            "{{\"nova_files\": {}}}",
            serde_json::to_string(&gone.display().to_string()).expect("JSON")
        ))
        .expect("seeds");
    let found = usual()[0].clone();
    let mut machine = Machine::new(&store, &[&found], &[]);
    let mut started = machine.launch().expect("starts");
    assert_eq!(started.dir, found);
    assert_eq!(
        started.report.take_alert(),
        Some(format!(
            "nova: the remembered Nova Files {} no longer hold EV Nova data (no data file in \
             them loads); forgetting them",
            gone.display()
        ))
    );
    assert_eq!(started.report.take_alert(), None, "one alert body, once");
    assert!(
        !store.text().expect("saved").contains("nova_files"),
        "forgotten"
    );
}

#[test]
fn a_good_pick_that_cannot_be_saved_starts_and_warns_in_the_log_and_the_alert() {
    let store = MemorySettings::new();
    store.fail_writes(true);
    let app = Path::new("/Games").join("EV Nova.app");
    let nova_files = resolve_pick(&app);
    let mut machine = Machine::new(&store, &[&nova_files], &[&app]);
    let mut started = machine.launch().expect("starts");
    assert_eq!(started.dir, nova_files);
    let warning = "nova: cannot save the settings: the disk is full";
    assert_eq!(started.report.take_alert().as_deref(), Some(warning));
    assert!(
        machine.log.0.borrow().iter().any(|line| line == warning),
        "logged: {:?}",
        machine.log.0.borrow()
    );
    assert_eq!(store.text(), None, "nothing was saved");
}

#[test]
fn a_bad_rules_value_warns_in_the_log_and_the_alert() {
    let store = MemorySettings::holding(r#"{"rules": "nonsense"}"#);
    let found = usual()[0].clone();
    let mut machine = Machine::new(&store, &[&found], &[]);
    let mut started = machine.launch().expect("starts");
    assert_eq!(started.dir, found);
    let warning = "nova: the saved \"rules\" in memory is not \"engine\" or \"bible\"; ignoring it";
    assert_eq!(started.report.take_alert().as_deref(), Some(warning));
    assert!(
        machine.log.0.borrow().iter().any(|line| line == warning),
        "logged: {:?}",
        machine.log.0.borrow()
    );
}

#[test]
fn a_named_directory_wins_over_the_remembered_one() {
    let store = MemorySettings::holding(r#"{"nova_files": "/remembered"}"#);
    let named = PathBuf::from("/named");
    let mut machine = Machine::new(&store, &[Path::new("/remembered")], &[]);
    let terminal = Stderr { terminal: true };
    let started = run(
        Launch {
            args: vec![named.clone().into_os_string()],
            nova_data: Some(OsString::from("/env")),
            game_data: None,
            usual: usual(),
            terminal: &terminal,
            log: None,
            stderr: Box::new(io::sink()),
            settings: Some(Box::new(store.clone()) as Box<dyn SettingsStore>),
            source: &machine.disk,
            dialog: &mut machine.panel,
            alert: &mut machine.alerts,
        },
        || Ok(()),
    )
    .expect("starts");
    assert_eq!(started.dir, named);
    assert_eq!(*machine.disk.asked.borrow(), [named]);
    assert_eq!(store.writes(), 0, "the remembered one is kept");
}

#[test]
fn an_event_loop_that_cannot_be_made_is_fatal_before_anything_is_asked() {
    let mut machine = Machine::new(&MemorySettings::new(), &[], &[]);
    let terminal = Stderr { terminal: false };
    let code = run::<(), _, _, _>(
        Launch {
            args: Vec::new(),
            nova_data: None,
            game_data: None,
            usual: usual(),
            terminal: &terminal,
            log: None,
            stderr: Box::new(io::sink()),
            settings: None,
            source: &machine.disk,
            dialog: &mut machine.panel,
            alert: &mut machine.alerts,
        },
        || Err("no display".to_owned()),
    )
    .err();
    assert_eq!(code, Some(1));
    assert_eq!(machine.alerts.0, ["nova: no display"]);
    assert_eq!(machine.panel.prompts, Vec::<String>::new());
    assert_eq!(*machine.disk.asked.borrow(), Vec::<PathBuf>::new());
}

#[test]
fn more_than_one_argument_is_a_usage_error() {
    let mut machine = Machine::new(&MemorySettings::new(), &[], &[]);
    let terminal = Stderr { terminal: true };
    let code = run(
        Launch {
            args: vec![OsString::from("/a"), OsString::from("/b")],
            nova_data: None,
            game_data: None,
            usual: usual(),
            terminal: &terminal,
            log: None,
            stderr: Box::new(io::sink()),
            settings: None,
            source: &machine.disk,
            dialog: &mut machine.panel,
            alert: &mut machine.alerts,
        },
        || Ok(()),
    )
    .err();
    assert_eq!(code, Some(2));
    assert_eq!(*machine.disk.asked.borrow(), Vec::<PathBuf>::new());
}

// The new game, chosen by `GAME_DATA`.

/// The new game's data folder the tests name in `GAME_DATA`.
fn new_game_dir() -> PathBuf {
    Path::new("/Games").join("New Game")
}

#[test]
fn without_game_data_or_with_it_empty_the_stock_game_starts_as_before() {
    for game_data in [None, Some(OsString::new())] {
        let found = usual()[0].clone();
        let mut machine = Machine::new(&MemorySettings::new(), &[&found], &[]);
        machine.game_data = game_data.clone();
        let started = machine.launch().expect("starts");
        assert_eq!(started.ruleset, Ruleset::Stock, "{game_data:?}");
        assert_eq!(started.dir, found);
        assert_eq!(started.data, found);
        assert_eq!(*machine.disk.asked.borrow(), [found]);
    }
}

/// A launch with `GAME_DATA` naming [`new_game_dir`], whose folder opens
/// as `opened`.
fn new_game(store: &MemorySettings, opened: NewGameOpened<PathBuf>) -> Machine {
    let mut machine = Machine::new(store, &[], &[]);
    machine.game_data = Some(new_game_dir().into_os_string());
    machine.disk.new_game = Some(opened);
    machine
}

#[test]
fn game_data_opens_its_folder_through_the_port_and_starts_over_its_data() {
    let store = MemorySettings::new();
    let data = Path::new("/loaded").to_path_buf();
    let mut machine = new_game(&store, NewGameOpened::Loaded(loaded(data.clone())));
    machine.nova_data = Some(OsString::from("/env/Nova Files"));
    let mut started = machine.launch().expect("starts");
    assert_eq!(started.ruleset, Ruleset::NewGame);
    assert_eq!(started.dir, new_game_dir());
    assert_eq!(started.data, data, "what the port returned");
    assert_eq!(*machine.disk.new_game_asked.borrow(), [new_game_dir()]);
    assert_eq!(
        *machine.disk.asked.borrow(),
        Vec::<PathBuf>::new(),
        "no Nova Files looked for"
    );
    assert_eq!(machine.panel.prompts, Vec::<String>::new(), "never asked");
    assert_eq!(machine.alerts.0, Vec::<String>::new());
    assert_eq!(store.writes(), 0, "nothing saved");
    assert_eq!(machine.log.0.borrow().len(), 1, "the header alone");
    assert_eq!(machine.stderr.text(), "");
    assert_eq!(started.report.take_alert(), None);
}

#[test]
fn each_load_problem_is_one_warning_in_the_log_stderr_and_the_alert() {
    let opened = NewGameOpened::Loaded(NewGameLoaded {
        problems: vec![
            "systems/Gamma.md: the file name is not an id".to_owned(),
            "stellars/yard.md: its system `gamma` is not in systems/".to_owned(),
        ],
        ..loaded(PathBuf::from("/loaded"))
    });
    let mut machine = new_game(&MemorySettings::new(), opened);
    let mut started = machine.launch().expect("starts");
    let warnings = [
        "nova: the new game's data: systems/Gamma.md: the file name is not an id".to_owned(),
        "nova: the new game's data: stellars/yard.md: its system `gamma` is not in systems/"
            .to_owned(),
    ];
    assert_eq!(machine.log.0.borrow()[1..], warnings);
    assert_eq!(machine.stderr.text(), format!("{}\n", warnings.join("\n")));
    assert_eq!(started.report.take_alert(), Some(warnings.join("\n")));
}

#[test]
fn lfs_pointers_are_one_warning_naming_the_count_the_first_and_the_fix() {
    let opened = NewGameOpened::Loaded(NewGameLoaded {
        lfs_pointers: vec![
            "media/export/pack/hull.glb".to_owned(),
            "media/export/pack/surface.png".to_owned(),
        ],
        ..loaded(PathBuf::from("/loaded"))
    });
    let mut machine = new_game(&MemorySettings::new(), opened);
    let started = machine.launch().expect("starts");
    let warning = format!(
        "nova: 2 media files in {} are Git LFS pointers, not the files, starting with \
         media/export/pack/hull.glb; run `git lfs install --local` and `git lfs pull` in the \
         game-bible checkout",
        new_game_dir().display()
    );
    assert_eq!(machine.log.0.borrow()[1..], [warning]);
    assert_eq!(started.dir, new_game_dir(), "the game starts");
}

#[test]
fn a_folder_that_cannot_be_listed_is_fatal_naming_game_data_and_the_folder() {
    let opened = NewGameOpened::Unreadable("No such file or directory".to_owned());
    let mut machine = new_game(&MemorySettings::new(), opened);
    assert_eq!(machine.launch().err(), Some(2));
    let message = format!(
        "nova: listing the new game's data directory {} (GAME_DATA): No such file or directory",
        new_game_dir().display()
    );
    assert_eq!(machine.log.0.borrow()[1..], *std::slice::from_ref(&message));
    assert_eq!(machine.alerts.0, [message]);
    assert_eq!(*machine.disk.asked.borrow(), Vec::<PathBuf>::new());
    assert_eq!(machine.panel.prompts, Vec::<String>::new());
}

#[test]
fn a_folder_with_no_systems_warns_and_starts() {
    let opened = NewGameOpened::Loaded(NewGameLoaded {
        systems: 0,
        ..loaded(PathBuf::from("/loaded"))
    });
    let mut machine = new_game(&MemorySettings::new(), opened);
    let started = machine.launch().expect("starts");
    let warning = format!(
        "nova: the new game's data in {} has no systems; starting with an empty galaxy",
        new_game_dir().display()
    );
    assert_eq!(machine.log.0.borrow()[1..], [warning]);
    assert_eq!(started.ruleset, Ruleset::NewGame);
}

#[test]
fn the_new_game_keeps_the_remembered_nova_files() {
    let remembered = Path::new("/Games").join("Nova Files");
    let store = remembering(&remembered);
    let before = (store.text(), store.writes());
    let mut machine = Machine::new(&store, &[&remembered], &[]);
    machine.game_data = Some(new_game_dir().into_os_string());
    let started = machine.launch().expect("starts");
    assert_eq!(started.ruleset, Ruleset::NewGame);
    assert_eq!(*machine.disk.asked.borrow(), Vec::<PathBuf>::new());
    assert_eq!((store.text(), store.writes()), before, "untouched");
}

#[test]
fn the_new_game_still_reads_the_settings() {
    let store = MemorySettings::holding(r#"{"rules": "nonsense"}"#);
    let mut machine = Machine::new(&store, &[], &[]);
    machine.game_data = Some(new_game_dir().into_os_string());
    machine.launch().expect("starts");
    assert_eq!(
        machine.log.0.borrow()[1..],
        ["nova: the saved \"rules\" in memory is not \"engine\" or \"bible\"; ignoring it"]
    );
}

#[test]
fn a_nova_files_argument_with_game_data_says_game_data_rejected_it() {
    let mut machine = Machine::new(&MemorySettings::new(), &[], &[]);
    machine.terminal = true;
    machine.game_data = Some(new_game_dir().into_os_string());
    machine.args = vec![OsString::from("/Games/Nova Files")];
    assert_eq!(machine.launch().err(), Some(2));
    let rejected = format!(
        "nova: GAME_DATA is set to {}, so the new game starts and takes no Nova Files \
         argument; unset GAME_DATA to open /Games/Nova Files, or drop the argument",
        new_game_dir().display()
    );
    assert_eq!(machine.log.0.borrow()[1..], [rejected]);
    assert_eq!(*machine.disk.asked.borrow(), Vec::<PathBuf>::new());
    assert_eq!(
        *machine.disk.new_game_asked.borrow(),
        Vec::<PathBuf>::new(),
        "the folder isn't opened"
    );
    assert_eq!(machine.panel.prompts, Vec::<String>::new());
}
