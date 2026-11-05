//! Starting the game: choosing the ruleset ([`crate::ruleset`]), finding
//! the `Nova Files` for the stock game or opening the new game's folder,
//! and reporting what went wrong on the way, from a terminal or from a
//! double-clicked app.
//!
//! - [`mod@locate`]: the lookup order (the argument or `NOVA_DATA`, the
//!   remembered directory, the usual places, then the open panel) over the
//!   [`DataSource`] and [`NovaFilesDialog`] ports, and the change it makes
//!   to the remembered directory.
//! - [`new_game`]: what opening the new game's `GAME_DATA` folder gave,
//!   through the [`DataSource`] port, and the warnings it starts with.
//! - [`report`]: where each startup line goes ([`Report`]), over the
//!   [`Terminal`], [`StartupLog`] and [`FatalAlert`] ports, and the exit
//!   each fatal error ends with ([`Fatal`]).
//! - [`run`]: the startup sequence itself, over all of those ports; `main`
//!   only builds the real adapters and hands them in.

pub mod locate;
pub mod new_game;
pub mod report;

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

use nova_audio::SettingsStore;
use nova_sim::Rulebook;
use nova_view::flight::FlightOverlays;

pub use locate::{
    Answer, DataSource, DiskData, Found, Located, NotFound, NotLocated, NovaFilesDialog, Opened,
    Remember, Via, answer, apply_remember, locate, resolve_pick, usual_places,
};
pub use new_game::{NewGameLoaded, NewGameOpened};
pub use report::{Fatal, FatalAlert, FileLog, Report, Shown, StartupLog, StdTerminal, Terminal};

use crate::cli;
use crate::developer::flight_overlays;
use crate::rulebook::ruleset_rulebook;
use crate::ruleset::{Ruleset, chosen};
use crate::settings::{GameKeeper, GameSettings, game_settings};

/// What the startup sequence is given: the command line and environment
/// it reads, and the ports it works through.
pub struct Launch<'a, D, G, A> {
    /// The arguments, without the program's name.
    pub args: Vec<OsString>,
    /// The value of `NOVA_DATA`.
    pub nova_data: Option<OsString>,
    /// The value of `GAME_DATA`, which chooses the ruleset ([`chosen`]).
    pub game_data: Option<OsString>,
    /// The usual places to look, in order ([`usual_places`]).
    pub usual: Vec<PathBuf>,
    /// Says whether stderr is a terminal.
    pub terminal: &'a dyn Terminal,
    /// The startup log, if there is anywhere to write it.
    pub log: Option<Box<dyn StartupLog>>,
    /// stderr.
    pub stderr: Box<dyn Write>,
    /// The settings file, if there is anywhere to keep it.
    pub settings: Option<Box<dyn SettingsStore>>,
    /// Opens a directory as the game's data.
    pub source: &'a D,
    /// Asks the player where the data is.
    pub dialog: &'a mut G,
    /// Shows a fatal startup error.
    pub alert: &'a mut A,
}

/// What the game goes on with once it has started.
pub struct Started<E, D> {
    /// The game started.
    pub ruleset: Ruleset,
    /// The report, for the rest of startup and the runner.
    pub report: Report,
    /// The event loop `open_loop` made.
    pub event_loop: E,
    /// The rules the settings and the ruleset's readings choose.
    pub rulebook: Rulebook,
    /// The developer's flight overlays the settings turn on.
    pub overlays: FlightOverlays,
    /// The settings keeper, if there is anywhere to save.
    pub keeper: Option<GameKeeper>,
    /// The settings the game starts with.
    pub settings: GameSettings,
    /// The `Nova Files` directory found, or the new game's `GAME_DATA`
    /// folder.
    pub dir: PathBuf,
    /// Its data: for the new game, what loaded from its folder
    /// ([`DataSource::open_new_game`]).
    pub data: D,
}

/// Starts the game, in order:
///
/// 1. Builds the [`Report`] over `terminal`, `log` and `stderr`.
/// 2. Makes the event loop with `open_loop`, before any dialog, so the
///    platform's windowing sets the application up itself.
/// 3. Chooses the ruleset from `GAME_DATA` ([`chosen`]) and reads the
///    settings: the rulebook (with the ruleset's readings), the flight
///    overlays and the keeper, so the remembered `Nova Files` are known.
/// 4. For the new game, takes no argument, opens its folder
///    ([`DataSource::open_new_game`]) and starts over what loaded,
///    reporting each problem, the media that are Git LFS pointers and a
///    galaxy with no systems as warnings ([`new_game::warnings`]): no
///    lookup, no open panel and no change to the remembered directory. A
///    folder that can't be listed is fatal.
/// 5. For the stock game, reads the command line ([`cli::data_dir`]) and
///    finds the data ([`locate()`]), reporting each warning, and saves the
///    change to the remembered directory ([`apply_remember`]).
///
/// # Errors
///
/// The exit code, once a fatal error has been reported
/// ([`Report::fail`]): a usage error, an event loop that could not be
/// made, no data found, or named data (or the new game's folder) that
/// could not be listed.
pub fn run<E, D, G, A>(
    launch: Launch<'_, D, G, A>,
    open_loop: impl FnOnce() -> Result<E, String>,
) -> Result<Started<E, D::Data>, u8>
where
    D: DataSource,
    G: NovaFilesDialog,
    A: FatalAlert,
{
    let Launch {
        args,
        nova_data,
        game_data,
        usual,
        terminal,
        log,
        stderr,
        mut settings,
        source,
        dialog,
        alert,
    } = launch;
    let mut report = Report::new(terminal, log, stderr);
    let event_loop = match open_loop() {
        Ok(event_loop) => event_loop,
        Err(error) => return Err(report.fail(&Fatal::EventLoop(error), alert)),
    };
    let (ruleset, game_dir) = chosen(game_data);
    let (rulebook, mut warnings) =
        ruleset_rulebook(settings.as_deref_mut(), ruleset.profile().readings);
    let (overlays, more) = flight_overlays(settings.as_deref_mut());
    warnings.extend(more);
    let (mut keeper, game, warning) = game_settings(settings);
    warnings.extend(warning);
    for warning in &warnings {
        report.warn(warning);
    }
    if let Some(dir) = game_dir {
        if !args.is_empty() {
            let fatal = Fatal::ArgumentsWithGameData { dir, args };
            return Err(report.fail(&fatal, alert));
        }
        let loaded = match source.open_new_game(&dir) {
            NewGameOpened::Loaded(loaded) => loaded,
            NewGameOpened::Unreadable(error) => {
                let fatal = Fatal::NewGameUnreadable { dir, error };
                return Err(report.fail(&fatal, alert));
            }
        };
        for warning in new_game::warnings(&dir, &loaded) {
            report.warn(&warning);
        }
        return Ok(Started {
            ruleset,
            report,
            event_loop,
            rulebook,
            overlays,
            keeper,
            settings: game,
            dir,
            data: loaded.data,
        });
    }
    let named = match cli::data_dir(&args, nova_data) {
        Ok(named) => named,
        Err(cli::Usage) => return Err(report.fail(&Fatal::Usage, alert)),
    };
    let located = locate(
        named.as_deref(),
        game.nova_files.as_deref(),
        &usual,
        source,
        dialog,
    );
    for warning in &located.warnings {
        report.warn(warning);
    }
    if let Some(warning) = apply_remember(keeper.as_mut(), &located.remember) {
        report.warn(&warning);
    }
    match located.outcome {
        Ok(found) => Ok(Started {
            ruleset,
            report,
            event_loop,
            rulebook,
            overlays,
            keeper,
            settings: game,
            dir: found.dir,
            data: found.data,
        }),
        Err(not) => Err(report.fail(&Fatal::from(not), alert)),
    }
}
