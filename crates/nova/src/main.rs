//! `nova`: opens the game data and shows it in a window, starting on the
//! ship browser.
//!
//! Usage: `nova [NOVA_FILES_DIR]`, or set `NOVA_DATA` to the `Nova Files`
//! directory. Exits 2 on a usage error and 1 when the data or the window
//! cannot be opened.

use std::ffi::OsString;
use std::process::ExitCode;
use std::rc::Rc;

use nova::app::start_screen;
use nova::platform::Runner;
use nova::{cli, exit};
use nova_data::GameData;
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
    // The ship browser and the renderer read the same game data.
    let data = Rc::new(data);
    let screen = start_screen(Rc::clone(&data));
    let mut runner = Runner::new(data, screen);
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
