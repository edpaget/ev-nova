//! The startup sequence, end to end over mocks of every port: the
//! command line, the settings file, the data on disk, the open panel,
//! stderr's terminal, the startup log and the error alert.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use nova::startup::{
    DataSource, FatalAlert, Launch, NovaFilesDialog, Opened, Started, StartupLog, Terminal,
    resolve_pick, run,
};
use nova_audio::SettingsStore;
use nova_audio::recording::MemorySettings;

/// Directories holding the game's data; every other one is empty. Records
/// each directory asked for.
#[derive(Default)]
struct Disk {
    holding: Vec<PathBuf>,
    asked: RefCell<Vec<PathBuf>>,
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
}

/// The open panel: its picks in turn, then a cancel; records each prompt.
#[derive(Default)]
struct Panel {
    picks: VecDeque<PathBuf>,
    prompts: Vec<String>,
}

impl NovaFilesDialog for Panel {
    fn pick(&mut self, prompt: &str) -> Option<PathBuf> {
        self.prompts.push(prompt.to_owned());
        self.picks.pop_front()
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

#[derive(Default)]
struct Alerts(Vec<String>);

impl FatalAlert for Alerts {
    fn show_fatal(&mut self, message: &str) {
        self.0.push(message.to_owned());
    }
}

/// One launch with no argument and no `NOVA_DATA`.
struct Machine {
    disk: Disk,
    panel: Panel,
    alerts: Alerts,
    log: Log,
    store: MemorySettings,
    terminal: bool,
}

impl Machine {
    fn new(store: &MemorySettings, holding: &[&Path], picks: &[&Path]) -> Self {
        Self {
            disk: Disk {
                holding: holding.iter().map(|dir| dir.to_path_buf()).collect(),
                asked: RefCell::default(),
            },
            panel: Panel {
                picks: picks.iter().map(|pick| pick.to_path_buf()).collect(),
                prompts: Vec::new(),
            },
            alerts: Alerts::default(),
            log: Log::default(),
            store: store.clone(),
            terminal: false,
        }
    }

    fn launch(&mut self) -> Result<Started<(), PathBuf>, u8> {
        let terminal = Stderr {
            terminal: self.terminal,
        };
        run(
            Launch {
                args: Vec::<OsString>::new(),
                nova_data: None,
                usual: usual(),
                terminal: &terminal,
                log: Some(Box::new(self.log.clone())),
                stderr: Box::new(io::sink()),
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

#[test]
fn a_cancel_from_a_terminal_shows_no_alert() {
    let mut machine = Machine::new(&MemorySettings::new(), &[], &[]);
    machine.terminal = true;
    assert_eq!(machine.launch().err(), Some(2));
    assert_eq!(machine.alerts.0, Vec::<String>::new());
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
