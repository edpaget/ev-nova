//! `nova`: opens the game data and shows it in a window, starting on the
//! ship browser; Tab switches to the galaxy map and back. On the map,
//! Return enters the selected system; Escape goes back, or quits. F flies
//! the player's ship, I shows the About text in the game's own dialog, and
//! P the Preferences dialog, which turns sound and music on or off and
//! sets their volumes.
//!
//! With `--features dev-tools` (`mise run dev`), `` ` `` toggles the
//! developer tools.
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
//! The sound settings are saved in `settings.json` in a `nova` directory
//! under the platform's configuration directory: `~/Library/Application
//! Support` on macOS, `%APPDATA%` on Windows, and `$XDG_CONFIG_HOME` (or
//! `~/.config`) elsewhere. Missing settings start at the defaults, and so
//! do settings that cannot be read, with a warning.
//!
//! Usage: `nova [NOVA_FILES_DIR]`, or set `NOVA_DATA` to the `Nova Files`
//! directory. Exits 2 on a usage error and 1 when the data or the window
//! cannot be opened.

use std::ffi::OsString;
use std::process::ExitCode;
use std::rc::Rc;

use nova::app::start_screen;
use nova::audio::{game_audio, game_settings, music_warning};
use nova::config::{Os, settings_path};
use nova::fonts::game_fonts;
use nova::platform::Runner;
use nova::{cli, exit};
use nova_audio::{FileSettings, KiraAudio, SettingsStore};
use nova_data::fonts::open_charcoal;
use nova_data::music::open_music;
use nova_data::{GameData, open_interface};
use nova_render::wgpu::GlyphonMetrics;
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
    let (fonts, warning) = game_fonts(open_charcoal(&dir));
    if let Some(warning) = warning {
        eprintln!("{warning}");
    }
    // The screens and the renderer read the same game data.
    let data = Rc::new(data);
    let store = settings_path(Os::current(), |name| std::env::var_os(name))
        .map(|path| Box::new(FileSettings::new(path)) as Box<dyn SettingsStore>);
    let (keeper, settings, warning) = game_settings(store);
    if let Some(warning) = warning {
        eprintln!("{warning}");
    }
    let mut screen = start_screen(Rc::clone(&data)).with_sound_prefs(settings.prefs());
    match open_interface(&dir) {
        Ok(interface) => {
            // Dialog text is laid out by the faces the window draws it in.
            let metrics: Rc<dyn TextMetrics> = Rc::new(GlyphonMetrics::new(&fonts));
            let dialogs: Rc<dyn DialogResources> = Rc::new(interface);
            screen = screen.with_dialogs(dialogs, metrics);
        }
        Err(error) => eprintln!("nova: running without dialogs: {error}"),
    }
    let (music, warning) = music_warning(open_music(&dir));
    if let Some(warning) = warning {
        eprintln!("{warning}");
    }
    let (audio, warning) = game_audio(KiraAudio::open(Rc::clone(&data), music));
    if let Some(warning) = warning {
        eprintln!("{warning}");
    }
    let runner = Runner::new(Rc::clone(&data), screen, fonts);
    let runner = match audio {
        Some(core) => runner.with_audio(core.with_settings(settings)),
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
