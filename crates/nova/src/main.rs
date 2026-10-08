//! `nova`: opens the game data and shows it in a window, starting on the
//! main menu. New Pilot asks for the pilot's name and flies a new pilot
//! from the first `chär`; Open Pilot lists the saved pilots and resumes
//! the one chosen where it was; Quit (or Escape) quits. In flight, Escape
//! goes back to the menu. I shows the About text in the game's own dialog,
//! and P the Preferences dialog, which turns sound and music on or off and
//! sets their volumes.
//!
//! Pilots are saved, one JSON file each, in a `Pilots` directory beside
//! the settings file (below): on landing, on taking off, after each change
//! in the spaceport (each trade in the Trade Center among them), on going
//! back to the menu and on quitting. Without a place to save them, the
//! game runs with a warning and saves nothing.
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
//! with a warning, and I and P do nothing.
//!
//! Sound effects come from the game data's `snd ` resources and the music
//! from `Nova Music.mp3` in `Nova Files`, played on the default audio
//! device. Without the music the game plays none, with a warning; without
//! an audio device it runs silently, with a warning.
//!
//! The settings (sound, and Hyperspace Effects) are saved in `settings.json` in a `nova` directory
//! under the platform's configuration directory: `~/Library/Application
//! Support` on macOS, `%APPDATA%` on Windows, and `$XDG_CONFIG_HOME` (or
//! `~/.config`) elsewhere. Missing settings start at the defaults, and so
//! do settings that cannot be read, with a warning.
//!
//! The same file chooses, for each rule where the Nova Bible and the
//! original engine disagree, which one the game follows: `"rules"`,
//! `"engine"` (the default) or `"bible"`, for all of them, and
//! `"rule_overrides"`, an object, for any one by its key:
//! `"crime_gains"` (whether a crime against a government's ship improves
//! the record with every government not allied with it, as the engine
//! does, or only with its enemies, as the Bible says), `"empty_booty"`
//! (whether a ship of `Booty` 0 opens the plunder dialog anyway, or
//! repels the boarders), `"crewless_capture"` (whether a ship of no crew
//! can still capture), `"piracy_police"` (whether warships as well as
//! interceptors answer the player's attack or boarding, or interceptors
//! only), `"quiet_hails"` (whether a quiet government's ships answer
//! Greetings and ignore the middle button, as the engine does, or answer
//! "No response." to Greetings, as the Bible says) and `"long_advice"`
//! (whether an advice line exactly 42 characters long reads "Nice to meet
//! you.", as the engine's slip has it, or is shown as written), and
//! the hiring rules: `"hire_require"` (whether an unmet `Require` refuses
//! a hire), `"take_off_pay"` (whether each take-off pays the hired
//! escorts a day's wages), `"hire_fee"` (whether a hire takes the
//! engine's extra credit or exactly the fee shown) and `"escort_wage"`
//! (whether a hired escort is paid the wage its ship type gives now or
//! the wage it was hired at), and the persons' rules: `"person_odds"`
//! (how often a person appears), `"system_persons"` (whether a system's
//! Person slot always brings its person), `"link_syst_slip"` (whether a
//! person linked to a system also appears in the one the engine's slip
//! gives), `"shield_mod"` (whether `ShieldMod` scales the armour too),
//! `"person_coward"` (which persons run at their `Coward`),
//! `"person_credits"` (the credits a person carries), `"comm_quote"`
//! (whether a person's comm quote answers Greetings or opens the hail)
//! and `"person_join"` (whether no person offers to join, as the engine
//! has it until missions exist, or a person whose record allows it lists
//! Use As Escort and joins the fleet as itself), and the boarding
//! grants: `"grant_count"` (whether a grant is half its `GrantCount` to
//! all of it, as the engine and the Bible have it, or one to all of it)
//! and `"grant_max"` (whether a grant, by boarding or by `G`, may pass
//! the outfit's `Max`, or is held to it and to the free mass), and the
//! outfit effects: `"map_explore"` (whether a map explores the engine's
//! depth-first way, or every system within its jumps), `"invalid_map"`
//! (whether a map that explores nothing is used up, or kept as an
//! outfit) and `"remove_refund"` (whether `D` pays nothing for the
//! outfit it removes, or what selling it would); every key is listed in
//! `nova_sim::rulebook`. They are set by editing the file; a sound change
//! in the Preferences dialog keeps them.
//!
//! When the game data loads, each data file that could not be loaded and
//! each control-bit expression that does not parse is reported as a line
//! on standard error, and the game runs without them.
//!
//! Usage: `nova [NOVA_FILES_DIR]`, or set `NOVA_DATA` to the `Nova Files`
//! directory. Exits 2 on a usage error and 1 when the data or the window
//! cannot be opened.

use std::cell::RefCell;
use std::ffi::OsString;
use std::process::ExitCode;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use nova::app::start_screen;
use nova::audio::{game_audio, music_warning};
use nova::chance::SplitMix;
use nova::config::{Os, pilots_dir, settings_path};
use nova::fonts::game_fonts;
use nova::load::data_warnings;
use nova::platform::Runner;
use nova::rulebook::game_rulebook;
use nova::saves::FilePilots;
use nova::settings::game_settings;
use nova::{cli, exit};
use nova_audio::{FileSettings, KiraAudio, SettingsStore};
use nova_data::fonts::open_charcoal;
use nova_data::music::open_music;
use nova_data::{GameData, open_interface};
use nova_render::wgpu::GlyphonMetrics;
use nova_sim::{Chance, PilotKeeper, PilotStore};
use nova_view::flight::SharedChance;
use nova_view::text::TextMetrics;
use nova_view::ui::DialogResources;
use winit::event_loop::EventLoop;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let dir = match cli::data_dir(&args, std::env::var_os("NOVA_DATA")) {
        Ok(dir) => dir,
        Err(usage) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    };
    let data = match GameData::open(&dir, None) {
        Ok(data) => data,
        Err(error) => {
            eprintln!("nova: {error}");
            return ExitCode::from(1);
        }
    };
    for warning in data_warnings(&data) {
        eprintln!("{warning}");
    }
    let warn = |warning: Option<String>| {
        if let Some(warning) = warning {
            eprintln!("{warning}");
        }
    };
    let (fonts, warning) = game_fonts(open_charcoal(&dir));
    warn(warning);
    // The screens and the renderer read the same game data.
    let data = Rc::new(data);
    let mut store = settings_path(Os::current(), |name| std::env::var_os(name))
        .map(|path| Box::new(FileSettings::new(path)) as Box<dyn SettingsStore>);
    let (rulebook, warnings) = game_rulebook(store.as_deref_mut());
    for warning in warnings {
        eprintln!("{warning}");
    }
    let (keeper, settings, warning) = game_settings(store);
    warn(warning);
    // Dialog and menu text is laid out by the faces the window draws it in.
    let metrics: Rc<dyn TextMetrics> = Rc::new(GlyphonMetrics::new(&fonts));
    let pilots = pilots_dir(Os::current(), |name| std::env::var_os(name))
        .map(|dir| PilotKeeper::new(Box::new(FilePilots::new(dir)) as Box<dyn PilotStore>));
    if pilots.is_none() {
        eprintln!("nova: there is nowhere to save pilots (no home directory); saving nothing");
    }
    // Nanoseconds since 1970, wrapped: a different seed each run.
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            since
                .as_secs()
                .wrapping_mul(1_000_000_000)
                .wrapping_add(u64::from(since.subsec_nanos()))
        });
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(SplitMix::new(seed)));
    // The effects' own: drawing on it never changes the simulation's draws.
    let effects: Rc<RefCell<dyn Chance>> =
        Rc::new(RefCell::new(SplitMix::new(seed.wrapping_add(1))));
    let screen = start_screen(Rc::clone(&data))
        .with_prefs(settings.prefs())
        .with_pilots(pilots, Rc::clone(&metrics))
        .with_chance(SharedChance::new(chance))
        .with_effects_chance(SharedChance::new(effects));
    // Every disputed rule as the settings' rulebook chooses.
    let mut screen = screen.with_rulebook(&rulebook);
    match open_interface(&dir) {
        Ok(interface) => {
            let dialogs: Rc<dyn DialogResources> = Rc::new(interface);
            screen = screen.with_dialogs(dialogs, metrics);
        }
        Err(error) => eprintln!("nova: running without dialogs: {error}"),
    }
    let (music, warning) = music_warning(open_music(&dir));
    warn(warning);
    let (audio, warning) = game_audio(KiraAudio::open(Rc::clone(&data), music));
    warn(warning);
    let runner =
        Runner::new(Rc::clone(&data), screen, fonts).with_diagnostics(Box::new(std::io::stderr()));
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
    let mut runner = runner;
    let result = EventLoop::new().and_then(|event_loop| event_loop.run_app(&mut runner));
    match result {
        Ok(()) => {
            let exit = exit::exit(runner.open_failure());
            if let Some(message) = exit.message {
                eprintln!("{message}");
            }
            ExitCode::from(exit.code)
        }
        Err(error) => {
            eprintln!("nova: {error}");
            ExitCode::from(1)
        }
    }
}
