//! `nova`: opens the game data and shows it in a window, starting on the
//! main menu. New Pilot asks for the pilot's name and flies a new pilot
//! from the first `chär`; Open Pilot lists the saved pilots and resumes
//! the one chosen where it was; Quit (or Escape) quits, and so does
//! Command-Q, at once. Closing the window on the main menu asks whether
//! to quit first. On macOS the menu bar has the app menu (About,
//! Preferences…, Check for Updates…, Hide, Quit), View (Enter Full Screen)
//! and Window. In flight, Escape
//! goes back to the menu. I shows the About text in the game's own dialog,
//! and P the Preferences dialog, which turns sound and music on or off and
//! sets their volumes.
//!
//! Pilots are saved, one JSON file each, in a `Pilots` directory: on
//! landing, on taking off, after each change in the spaceport (each trade
//! in the Trade Center among them), on going back to the menu and on
//! quitting. On macOS and Windows it is beside the settings file (below),
//! and elsewhere it is `$XDG_DATA_HOME/nova/Pilots` (or
//! `~/.local/share/nova/Pilots`). Pilots an earlier version saved beside
//! the settings there are moved into it at launch, keeping every file,
//! with a notice in the startup report ([`nova::pilot_move`]). Without a
//! place to save them, the game runs with a warning and saves nothing.
//!
//! Planetary events start at random, and NPC traffic comes and goes at
//! random, on a generator seeded from the clock when the game starts; the
//! NPCs fly Nova's peaceful traffic: traders land, others jump out. Ships
//! are disabled as Nova disables them. In flight, Space and Control fire,
//! W selects the secondary weapon, and Tab and R pick the target; the
//! fight's explosions and debris roll on a generator of their own, seeded
//! beside the first, and its sounds are heard by how far away they are. Each weapon feature the simulation
//! does not handle yet is reported once, as a line on standard error.
//!
//! Tab, on the menu, goes to the developer's ship browser and galaxy map,
//! and switches between them. On the map, Return enters the selected
//! system; Escape goes back, and at the top back to the menu. F there
//! flies a fresh pilot that is never saved.
//!
//! With `--features dev-tools` (`mise run dev`), `` ` `` toggles the
//! developer tools: the resource browser, and the Pilot window, which
//! edits the pilot flying (credits, reserves, date, location and control
//! bits), saved as any change is.
//!
//! Text in Charcoal uses the game's own `Fonts/Charcoal.ttf` beside
//! `Nova Files` when it is there and usable, and the bundled font
//! otherwise (with a warning if the file is there but unusable).
//!
//! Dialogs come from the interface file beside `Nova Files`
//! (`Nova-DF.rsrc`, or the Windows `Nova.rez`). Without one, the game runs
//! with a warning, and I and P do nothing; the new game, without one, uses
//! its placeholder dialogs instead, with no warning when neither file is
//! there, and a warning naming the file and why when one is there but
//! doesn't load ([`nova::interface`]).
//!
//! A `dësc`'s QuickTime movie plays, unless the QuickTime Movies
//! preference is off, from the file its name names: looked for by bare
//! name in the Plug-Ins folder (`~/Library/Application Support/EV
//! Nova/Plug-Ins` on macOS, `Plug-ins` beside `Nova Files` elsewhere),
//! then in `Nova Files`, and decoded with the FFmpeg linked in
//! ([`nova::movie`]). A movie found in neither doesn't play. A ship's
//! `MovieFile` loops in the shipyard's and the hire screen's picture
//! item, found the same way, on a player of its own.
//!
//! Sound effects come from the game data's `snd ` resources and the music
//! from `Nova Music.mp3` in `Nova Files`, played on the default audio
//! device. Without the music the game plays none, with a warning; without
//! an audio device it runs silently, with a warning. The new game plays
//! its own clips and the track its `game.md` names as `music`
//! ([`nova::audio::game_music`]), and never looks for `Nova Music.mp3`.
//!
//! The settings (sound, and Hyperspace Effects) are saved in
//! `settings.json` in a `nova` directory under the platform's
//! configuration directory: `~/Library/Application Support` on macOS,
//! `%APPDATA%` on Windows, and `$XDG_CONFIG_HOME` (or `~/.config`)
//! elsewhere, where only the settings are. Missing settings start at the defaults, and so
//! do settings that cannot be read, with a warning.
//!
//! The same file chooses, for each rule where the Nova Bible and the
//! original engine disagree, which one the game follows: `"rules"`,
//! `"engine"` (the default) or `"bible"`, for all of them, and
//! `"rule_overrides"`, an object, for any one by its key, such as
//! `{"crime_gains": "bible"}`, or `{"dialog_focus_ring": "engine"}` for
//! the original's dialogs, with no keyboard focus ring. Every key, and
//! what each reading does, is listed in `nova_sim::rulebook`'s table. They are set by editing the
//! file; a sound change in the Preferences dialog keeps them.
//!
//! The same file turns on the developer's flight overlays, which the
//! original does not draw: `"developer"`, an object, with
//! `"flight_title": true` for the system's name and ID at the top left of
//! flight, and `"help_line": true` for the line of keys at its bottom.
//! Both are off otherwise ([`nova::developer`]).
//!
//! The same file turns on the new game's stock ship look, which draws the
//! new game's hulls in stock EV Nova sprites from the player's own
//! `NOVA_DATA` in place of their stand-ins: `"stock_ship_look"`, an
//! object, with `"on": true`, and `"ships"`, an object mapping a bible
//! ship id to the stock `shïp` ID it borrows, such as
//! `{"light-freighter": 136}`. Off otherwise; the stock game ignores it.
//! Nothing it reads is written anywhere ([`nova::stock_look`]).
//!
//! When the game data loads, each data file that could not be loaded and
//! each control-bit expression that does not parse is reported as a line
//! on standard error, and the game runs without them.
//!
//! Usage: `nova [NOVA_FILES_DIR]`, or set `NOVA_DATA` to the `Nova Files`
//! directory, or `GAME_DATA` to the new game's data. Either of the first
//! two wins over everything below. Without them the game
//! opens the `Nova Files` it remembers (the `"nova_files"` key of the
//! settings file), else those of `EV Nova.app` in `/Applications` or
//! `~/Applications` on macOS, when the loader loads at least one data file
//! from them. A remembered directory that no longer does is forgotten, with
//! a warning. Failing all of those, an open panel asks the player to
//! "Locate your EV Nova folder": `EV Nova.app` or its `Nova Files` can be
//! picked, and the pick is remembered for the next launch
//! ([`nova::startup`]). On macOS a quit (the Dock's Quit, a logout,
//! `osascript`'s `quit app`) while the panel is up ends the launch as its
//! Cancel does, with exit status 2 and the same "found no EV Nova data"
//! line on standard error and in the startup log, but with no alert.
//!
//! `GAME_DATA`, set and not empty, starts the new game instead of EV Nova
//! ([`nova::ruleset`]): `NOVA_DATA` is ignored (but for the stock ship
//! look's art, when that is on), an argument is a usage
//! error, and no `Nova Files` are looked for, asked for or remembered.
//! The folder is read in the bible's format (`nova_data::new_game`): its
//! factions, systems and stellars become the game's records. Each file
//! that doesn't load or map is a warning, as are media that are Git LFS
//! pointers (with the `git lfs` commands that fetch them) and a folder
//! with no systems; the game starts with what loaded (a folder that
//! names no music plays none and says nothing, and a named track that
//! doesn't load is one of those warnings; a missing interface file
//! doesn't warn, as the placeholder dialogs stand in for it, but one
//! there that doesn't load does). A folder that
//! can't be listed ends the launch with exit status 2. Its pilots are
//! saved in `New Game Pilots`, beside `Pilots`, so a stock pilot never
//! opens over the new game's data. An empty `GAME_DATA` is unset.
//!
//! Every startup error and warning goes to standard error and to the
//! startup log, which starts with the version and the build ID `nova` was
//! compiled with (`NOVA_BUILD_ID`), or "development build" without one.
//! It is rewritten each launch: `~/Library/Logs/nova/nova.log` on
//! macOS, `%LOCALAPPDATA%\nova\logs\nova.log` on Windows, and
//! `$XDG_STATE_HOME/nova/nova.log` (or `~/.local/state/nova/nova.log`)
//! elsewhere. When standard error is not a terminal (a double-clicked app), a
//! fatal startup error is shown in an alert too, and the startup warnings
//! are listed in one sheet over the game window once it opens. The
//! simulation's diagnostics while the game runs stay on standard error.
//!
//! On macOS a nightly `Nova.app` updates itself with Sparkle, from the
//! bundle's `Contents/Frameworks`: it checks its channel's feed (the
//! `SUFeedURL` its `Info.plist` names) in the background once a day, and
//! Check for Updates… checks at once. Installing quits the game as the
//! Dock's Quit does, saving the pilot flying, and relaunches the new build.
//! A development build, `cargo run` or a bundle without a feed, has no
//! updater and greys the item; a bundle that names a feed but can't load
//! Sparkle says so in the startup report.
//!
//! On Windows an installed nightly updates itself with WinSparkle, from
//! the `WinSparkle.dll` the installer puts beside `nova.exe`: it checks
//! its channel's feed (compiled in from `NOVA_CHANNEL`,
//! [`nova::winsparkle::FEED`]) in the background once a day. Installing
//! an update quits the game as the Quit command does, saving the pilot
//! flying, runs the new installer, and relaunches the game. The portable
//! zip (no DLL) and a development build don't update; a DLL that is there
//! but won't load says so in the startup report.
//!
//! On Windows `nova.exe` is a Windows-subsystem program, so Explorer opens
//! no console for it. Started from `cmd`, PowerShell or Windows Terminal,
//! it attaches to that terminal's console, and its lines print there,
//! after the prompt, which comes back at once
//! ([`nova::platform::console`]). There the alert and the warnings box are
//! message boxes, shown only when standard error goes nowhere: a file or a
//! pipe it was given counts as read. The executable carries its icon,
//! version and manifest (`packaging/windows/nova.exe.manifest`: per-monitor
//! DPI awareness, the UTF-8 code page and long paths).
//!
//! Exits 2 on a usage error and when no `Nova Files` are found or picked,
//! and 1 when the data named cannot be listed or the window cannot be
//! opened.

// No console window from Explorer; started from a terminal, the game
// attaches to its console (`nova::platform::console`). Not in tests, so a
// test harness built from this file still prints.
#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use nova::app::{AppScreen, start_screen};
use nova::audio::{game_audio, game_music, voice_limit};
use nova::chance::SplitMix;
use nova::config::{Os, legacy_pilots_dir, log_path, pilots_dir, settings_path};
use nova::exit::OpenFailure;
use nova::fonts::game_fonts;
use nova::load::data_warnings;
use nova::movie::{movie_folders, movie_player};
use nova::pilot_move::{DiskFiles, move_pilots, report_lines};
use nova::platform::{
    Dialogs, Runner, event_loop, loop_channel, menu_key_equivalents, system_key_names,
    system_keyboard_access,
};
use nova::ruleset::Ruleset;
use nova::saves::FilePilots;
use nova::startup::{
    DiskData, Fatal, FileLog, Launch, Report, Started, StartupLog, Terminal, usual_places,
};
use nova::stock_look::StockLook;
use nova_audio::{Audio, AudioCore, FileSettings, KiraAudio, SettingsStore};
use nova_data::fonts::open_charcoal;
use nova_data::{GameData, StockArt, open_interface};
use nova_render::GameImages;
use nova_render::wgpu::GlyphonMetrics;
use nova_sim::{Chance, PilotKeeper, PilotStore, Rulebook};
use nova_view::SharedBindings;
use nova_view::flight::{SharedChance, StockShipLooks};
use nova_view::text::TextMetrics;
use winit::event_loop::EventLoop;

/// The audio core over the default device, playing `data`'s sounds and
/// `ruleset`'s music (the soundtrack in `dir` for the stock game, the one
/// `data` names for the new game), with as many voices as `rulebook`
/// says; none when the device did not open. Each warning goes to
/// `report`.
fn open_audio(
    data: &Rc<GameData>,
    dir: &Path,
    ruleset: Ruleset,
    rulebook: &Rulebook,
    report: &mut Report,
) -> Option<AudioCore<Box<dyn Audio>>> {
    let (music, warning) = game_music(ruleset, data, dir);
    if let Some(warning) = warning {
        report.warn(&warning);
    }
    let voices = voice_limit(rulebook);
    let device = KiraAudio::open(Rc::clone(data), music).map(|audio| audio.with_voices(voices));
    let (audio, warning) = game_audio(device);
    if let Some(warning) = warning {
        report.warn(&warning);
    }
    audio
}

/// The pilot keeper over `ruleset`'s pilots directory `env` names, or
/// none, with a warning to `report`, when there is nowhere to save them.
/// First moves the pilots an earlier version saved elsewhere into the
/// stock game's directory, each line of the move to `report`.
fn open_pilots(
    env: impl Fn(&str) -> Option<std::ffi::OsString>,
    ruleset: Ruleset,
    report: &mut Report,
) -> Option<PilotKeeper<Box<dyn PilotStore>>> {
    // Before the keeper reads any, so the last pilot opens from the new
    // place. Only stock pilots were ever saved there.
    if let (Some(old), Some(new)) = (
        legacy_pilots_dir(Os::current(), &env),
        pilots_dir(Os::current(), &env, Ruleset::Stock),
    ) {
        for line in report_lines(&move_pilots(&mut DiskFiles, &old, &new)) {
            report.warn(&line);
        }
    }
    let pilots = pilots_dir(Os::current(), env, ruleset)
        .map(|dir| PilotKeeper::new(Box::new(FilePilots::new(dir)) as Box<dyn PilotStore>));
    if pilots.is_none() {
        report.warn("nova: there is nowhere to save pilots (no home directory); saving nothing");
    }
    pilots
}

/// The new game's stock ship look over the player's own stock data
/// `stock`, as `look` sets it: the renderer's stock art and the flight's
/// looks, which share it, each warning to `report`. Nothing it reads is
/// written anywhere ([`nova::stock_look`]).
fn stock_look_wired(
    data: &GameData,
    look: &StockLook,
    stock: Option<GameData>,
    report: &mut Report,
) -> (Option<Rc<StockArt>>, Option<StockShipLooks>) {
    let (wired, warnings) = nova::stock_look::wire(data, look, stock);
    for warning in &warnings {
        report.warn(warning);
    }
    wired.unzip()
}

/// Nanoseconds since 1970, wrapped: a different seed each run.
fn run_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            since
                .as_secs()
                .wrapping_mul(1_000_000_000)
                .wrapping_add(u64::from(since.subsec_nanos()))
        })
}

/// `runner` with the gamepads, through gilrs. Without them (no udev on a
/// Linux host, say) the game runs on the keyboard and mouse, with a
/// warning to `report`.
fn with_gamepads(
    runner: Runner<GameImages<Rc<GameData>>>,
    report: &RefCell<Report>,
) -> Runner<GameImages<Rc<GameData>>> {
    match nova::platform::gamepads::GilrsGamepads::new() {
        Ok(pads) => runner.with_gamepads(Box::new(pads)),
        Err(warning) => {
            report.borrow_mut().warn(&warning);
            runner
        }
    }
}

/// Runs `runner` on `event_loop` until it exits, the menu bar's picks sent
/// back through the loop and Sparkle offered as the updater on macOS, and
/// WinSparkle on Windows, its quit sent back through the loop; gives back
/// why the window could not open, if it could not. The loop takes the
/// runner and drops it as it ends.
fn run(
    runner: Runner<GameImages<Rc<GameData>>>,
    event_loop: EventLoop,
) -> (
    Option<OpenFailure>,
    Result<(), winit::error::EventLoopError>,
) {
    let (sender, events) = loop_channel(event_loop.create_proxy());
    let runner = runner.with_loop_events(events);
    #[cfg(target_os = "macos")]
    let runner = runner
        .with_menu_sender(sender)
        .with_updater_source(Box::new(nova::platform::SparkleSource));
    #[cfg(windows)]
    let runner = runner.with_updater_source(Box::new(nova::platform::winsparkle_source(sender)));
    // Nothing sends loop events elsewhere.
    #[cfg(not(any(target_os = "macos", windows)))]
    drop(sender);
    let failure = runner.failure_slot();
    let result = event_loop.run_app(runner);
    let failure = failure.borrow_mut().take();
    (failure, result)
}

/// Where the startup report says stderr goes: on Windows, once stderr is
/// connected to a console if it can be, before anything is written.
#[cfg(windows)]
fn stderr_terminal() -> impl Terminal {
    use nova::platform::console::{ConsoleTerminal, Win32Console, connect_stderr};
    ConsoleTerminal(connect_stderr(&mut Win32Console))
}

/// Where the startup report says stderr goes: the real stderr's word.
#[cfg(not(windows))]
fn stderr_terminal() -> impl Terminal {
    nova::startup::StdTerminal
}

/// `screen` with what Preferences asks the platform: whether the player
/// has turned on keyboard access to every control (its focus ring
/// follows it), and, for the keys screen, the menu bar's key equivalents,
/// which it binds to nothing. The layout's key names ride on the bindings
/// ([`player_bindings`]).
fn with_platform(screen: AppScreen) -> AppScreen {
    screen
        .with_keyboard_access(system_keyboard_access())
        .with_reserved_combos(menu_key_equivalents())
}

/// The app's one handle on `saved`, the keys the settings file saves,
/// with the player's layout on it, which names them wherever a text names
/// a key: the one place the layout is wired.
fn player_bindings(saved: nova_view::Bindings) -> SharedBindings {
    SharedBindings::new(saved).with_key_names(system_key_names())
}

// Wiring alone: a line or two for each collaborator it hands on.
#[allow(clippy::too_many_lines)]
fn main() -> ExitCode {
    // First: on Windows nothing reaches a console before it is attached.
    let terminal = stderr_terminal();
    let env = |name: &str| std::env::var_os(name);
    let started = nova::startup::run(
        Launch {
            args: std::env::args_os().skip(1).collect(),
            nova_data: env("NOVA_DATA"),
            game_data: env("GAME_DATA"),
            usual: usual_places(Os::current(), env),
            terminal: &terminal,
            log: log_path(Os::current(), env)
                .map(|path| Box::new(FileLog::new(path)) as Box<dyn StartupLog>),
            stderr: Box::new(std::io::stderr()),
            settings: settings_path(Os::current(), env)
                .map(|path| Box::new(FileSettings::new(path)) as Box<dyn SettingsStore>),
            source: &DiskData,
            dialog: &mut Dialogs,
            alert: &mut Dialogs,
        },
        // Before any dialog, so winit sets the application up itself.
        || event_loop().map_err(|error| error.to_string()),
    );
    let Started {
        ruleset,
        mut report,
        event_loop,
        rulebook,
        overlays,
        keeper,
        settings,
        dir,
        data,
        stock_look,
        stock_art,
        world,
    } = match started {
        Ok(started) => started,
        Err(code) => return ExitCode::from(code),
    };
    for warning in data_warnings(&data) {
        report.warn(&warning);
    }
    let (fonts, warning) = game_fonts(open_charcoal(&dir));
    if let Some(warning) = warning {
        report.warn(&warning);
    }
    let (stock_art, ship_looks) = stock_look_wired(&data, &stock_look, stock_art, &mut report);
    // The screens and the renderer read the same game data.
    let data = Rc::new(data);
    // Dialog and menu text is laid out by the faces the window draws it in.
    let metrics: Rc<dyn TextMetrics> = Rc::new(GlyphonMetrics::new(&fonts));
    let pilots = open_pilots(env, ruleset, &mut report);
    let seed = run_seed();
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(SplitMix::new(seed)));
    // The effects' own: drawing on it never changes the simulation's draws.
    let effects: Rc<RefCell<dyn Chance>> =
        Rc::new(RefCell::new(SplitMix::new(seed.wrapping_add(1))));
    let movie_folders = movie_folders(Os::current(), env, &dir);
    let mut screen = start_screen(Rc::clone(&data))
        .with_prefs(settings.prefs())
        // After the preferences, whose QuickTime Movies setting it reads.
        .with_movie_player(movie_player(movie_folders.clone()))
        // The shipyard's looping ship movie plays on a player of its own.
        .with_looping_movie_player(movie_player(movie_folders))
        .with_map_borders(settings.map_borders)
        // The keys the settings file saves, read by every screen built.
        .with_bindings(player_bindings(settings.bindings.clone()))
        .with_pilots(pilots, Rc::clone(&metrics))
        // The new game's conversations play on the Ink runtime.
        .with_dialog_loader(nova::dialogs::loader(seed))
        .with_chance(SharedChance::new(chance))
        .with_effects_chance(SharedChance::new(effects))
        .with_flight_overlays(overlays)
        // Every disputed rule as the settings' rulebook chooses.
        .with_rulebook(&rulebook);
    if let Some(looks) = ship_looks {
        screen = screen.with_ship_looks(Rc::new(looks));
    }
    // The new game's models: the table to the flights, the meshes and
    // planets its keys name to the renderer.
    screen = screen.with_world_models(Rc::new(world.models));
    screen = with_platform(screen);
    // The interface file beside the data, or the new game's placeholders.
    let interface = nova::interface::wire(ruleset, open_interface(&dir));
    for warning in &interface.warnings {
        report.warn(warning);
    }
    if let Some(dialogs) = interface.dialogs {
        screen = screen.with_dialogs(dialogs, metrics);
    }
    // Last, once the rules and the chance its load reads are given.
    let screen = screen.open_last_pilot();
    let audio = open_audio(&data, &dir, ruleset, &rulebook, &mut report);
    let report = Rc::new(RefCell::new(report));
    // The simulation's diagnostics are the running game's, not startup's.
    let images = nova::stock_look::images(&data, &rulebook, interface.file, stock_art);
    let runner = with_gamepads(Runner::new(images, screen, fonts), &report)
        .with_diagnostics(Box::new(std::io::stderr()))
        .with_report(Rc::clone(&report))
        .with_world_assets(world.meshes, world.planets, world.skies);
    let runner = match audio {
        Some(core) => runner.with_audio(core.with_settings(settings.audio)),
        None => runner,
    };
    let runner = match keeper {
        Some(keeper) => runner.with_settings(keeper),
        None => runner,
    };
    #[cfg(feature = "dev-tools")]
    let runner = runner.with_dev_tools(Rc::clone(&data));
    let (failure, result) = run(runner, event_loop);
    let ended = Fatal::ending(result.map_err(|error| error.to_string()), failure.as_ref());
    match ended {
        Some(fatal) => ExitCode::from(report.borrow_mut().fail(&fatal, &mut Dialogs)),
        None => ExitCode::SUCCESS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_player_s_bindings_are_the_saved_ones_named_through_the_platform_s_layout() {
        let saved = nova_view::Bindings::default().unbound(nova_view::keymap::FlightCommand::Eject);
        let bindings = player_bindings(saved.clone());
        assert_eq!(bindings.get(), saved);
        assert_eq!(
            format!("{:?}", bindings.names()),
            format!("{:?}", system_key_names())
        );
    }
}
