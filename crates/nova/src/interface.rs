//! The game's dialogs: the interface file beside the data, or, for the
//! new game without one, its placeholder dialogs.
//!
//! [`wire`] takes the ruleset and what opening the interface file gave,
//! and decides what every screen reads its templates through and what
//! the renderer reads the file's pictures from:
//!
//! - **The file opened:** the file serves every dialog, for either
//!   ruleset. No placeholder is mixed in, so a dialog the file lacks is
//!   the file's error, as it always was.
//! - **No file, the new game:** its placeholder dialogs
//!   (`nova_data::new_game::interface`), through the same seam the file
//!   goes through, with no warning, as the game has dialogs. They have no
//!   art, so the renderer gets no file.
//! - **No file, stock:** no dialogs, with a warning. The stock game always
//!   has its file, and never sees the placeholders.
//!
//! `main` calls it in place of a branch of its own; the wiring tests over
//! the new game's fixture call it as `main` does.

use std::fmt::Display;
use std::rc::Rc;

use nova_data::InterfaceData;
use nova_data::new_game::interface::default_dialogs;
use nova_view::ui::DialogResources;

use crate::ruleset::Ruleset;

/// What the screens and the renderer read, and what to warn about.
pub struct Interface {
    /// What every screen reads its dialog templates through; none
    /// without any.
    pub dialogs: Option<Rc<dyn DialogResources>>,
    /// The interface file, for the renderer's pictures; none without one.
    pub file: Option<Rc<InterfaceData>>,
    /// The warnings to print, each one line.
    pub warnings: Vec<String>,
}

/// The dialogs for `ruleset`, from `opened`, the result of opening the
/// interface file: the file when it opened; else, for the new game, its
/// placeholder dialogs; else none, with a warning naming why the file
/// didn't open.
#[must_use]
pub fn wire(ruleset: Ruleset, opened: Result<InterfaceData, impl Display>) -> Interface {
    match (opened, ruleset) {
        (Ok(file), _) => {
            let file = Rc::new(file);
            Interface {
                dialogs: Some(Rc::clone(&file) as Rc<dyn DialogResources>),
                file: Some(file),
                warnings: Vec::new(),
            }
        }
        (Err(_), Ruleset::NewGame) => Interface {
            dialogs: Some(Rc::new(default_dialogs())),
            file: None,
            warnings: Vec::new(),
        },
        (Err(error), Ruleset::Stock) => Interface {
            dialogs: None,
            file: None,
            warnings: vec![format!("nova: running without dialogs: {error}")],
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_data::Record;
    use nova_data::records::dialog::Dlog;
    use nova_data::records::dialog_items::Ditl;
    use nova_data::store::fixture::one_file_at;
    use nova_rsrc::fixture::ForkBuilder;
    use nova_view::spaceport::layout::SPACEPORT_DIALOG;
    use nova_view::ui::desc::DESC_DIALOG;

    use super::*;

    /// An interface file holding one empty `DLOG`, 3003, 100 x 50.
    fn file() -> InterfaceData {
        let mut dlog: Vec<u8> = [0_i16, 0, 50, 100]
            .iter()
            .flat_map(|v| v.to_be_bytes())
            .collect();
        dlog.extend([0, 1, 1, 0, 0, 0, 0, 0, 0, 0]);
        dlog.extend(DESC_DIALOG.to_be_bytes());
        dlog.push(0);
        let empty = (-1_i16).to_be_bytes().to_vec();
        let fork = ForkBuilder::new()
            .resource(Dlog::TYPE, DESC_DIALOG, None, &dlog)
            .resource(Ditl::TYPE, DESC_DIALOG, None, &empty)
            .build()
            .bytes;
        let path = Path::new("/Nova-DF.rsrc");
        InterfaceData::load(&one_file_at(path, fork), path).expect("loads")
    }

    fn missing() -> Result<InterfaceData, &'static str> {
        Err("no Nova-DF.rsrc")
    }

    #[test]
    fn the_new_game_without_a_file_reads_its_placeholders_silently() {
        let wired = wire(Ruleset::NewGame, missing());
        let dialogs = wired.dialogs.expect("dialogs");
        assert!(dialogs.dialog_template(SPACEPORT_DIALOG).is_ok());
        assert!(wired.file.is_none(), "the placeholders have no art");
        assert_eq!(wired.warnings, Vec::<String>::new());
    }

    #[test]
    fn stock_without_a_file_runs_without_dialogs_and_says_why() {
        let wired = wire(Ruleset::Stock, missing());
        assert!(wired.dialogs.is_none());
        assert!(wired.file.is_none());
        assert_eq!(
            wired.warnings,
            ["nova: running without dialogs: no Nova-DF.rsrc"]
        );
    }

    #[test]
    fn an_opened_file_serves_every_dialog_for_either_ruleset() {
        for ruleset in [Ruleset::Stock, Ruleset::NewGame] {
            let wired = wire(ruleset, Ok::<_, &str>(file()));
            let dialogs = wired.dialogs.expect("dialogs");
            assert_eq!(
                dialogs.dialog_template(DESC_DIALOG),
                file().dialog_template(DESC_DIALOG),
                "{ruleset:?}"
            );
            assert_eq!(
                dialogs.dialog_template(SPACEPORT_DIALOG),
                Err("no DLOG 1000".to_owned()),
                "{ruleset:?}: no placeholder is mixed in"
            );
            let file = wired.file.expect("the file, for the renderer");
            assert!(file.dialog(DESC_DIALOG).is_some());
            assert_eq!(wired.warnings, Vec::<String>::new());
        }
    }
}
