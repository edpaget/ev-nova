//! The new game's story state in a save, from version 39: the pilot's
//! world flags (`flags`) and each Ink file's story state (`dialogs`).
//!
//! The flags are their names, ascending (`["dock_regular"]`). The story
//! states are listed by file ascending, each its `file` as a ref names it
//! and its `state`, the dialog adapter's own saved text, kept opaque as a
//! JSON string (`{"file": "dialog/dock-job.ink", "state": "…"}`). A stock
//! pilot has neither, and saves two empty lists.

use serde::{Deserialize, Serialize};

use nova_sim_story::dialog::SavedDialog;
use nova_sim_story::dialog::states::DialogStates;
use nova_sim_story::flags::WorldFlags;

/// One Ink file's story state, as a save keeps it.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedDialogState {
    file: String,
    state: String,
}

/// The flags set in `flags`, by name ascending, as a save keeps them.
pub(super) fn saved_flags(flags: &WorldFlags) -> Vec<String> {
    flags.iter().map(str::to_owned).collect()
}

/// The flags a save lists.
pub(super) fn flags_of(saved: Vec<String>) -> WorldFlags {
    saved.into_iter().collect()
}

/// Each file's story state in `dialogs`, by file ascending, as a save
/// keeps them.
pub(super) fn saved_dialogs(dialogs: &DialogStates) -> Vec<SavedDialogState> {
    dialogs
        .iter()
        .map(|(file, saved)| SavedDialogState {
            file: file.to_owned(),
            state: saved.0.clone(),
        })
        .collect()
}

/// The story states a save lists, each under its file.
pub(super) fn dialogs_of(saved: Vec<SavedDialogState>) -> DialogStates {
    saved
        .into_iter()
        .map(|saved| (saved.file, SavedDialog(saved.state)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::tests::seasoned;
    use super::super::{CURRENT, SaveError, decode, encode, flags_and_dialogs};
    use super::*;
    use nova_sim_story::pilot::Pilot;

    /// [`seasoned`] with two flags set and two files' story states kept,
    /// each added out of order.
    fn storied() -> Pilot {
        let mut pilot = seasoned();
        pilot.flags.set("verge_open");
        pilot.flags.set("dock_regular");
        pilot
            .dialogs
            .keep("dialog/verge.ink", SavedDialog("verge\nstate".to_owned()));
        pilot.dialogs.keep(
            "dialog/dock-job.ink",
            SavedDialog("{\"dock\": 1}".to_owned()),
        );
        pilot
    }

    fn json(text: &str) -> serde_json::Value {
        serde_json::from_str(text).expect("JSON")
    }

    #[test]
    fn flags_and_story_states_survive_a_round_trip() {
        let pilot = storied();
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn flags_and_story_states_are_saved_ascending() {
        let value = json(&encode(&storied()));
        assert_eq!(value["version"], CURRENT);
        assert_eq!(
            value["flags"],
            serde_json::json!(["dock_regular", "verge_open"])
        );
        assert_eq!(
            value["dialogs"],
            serde_json::json!([
                {"file": "dialog/dock-job.ink", "state": "{\"dock\": 1}"},
                {"file": "dialog/verge.ink", "state": "verge\nstate"}
            ])
        );
    }

    #[test]
    fn a_pilot_with_neither_saves_two_empty_lists() {
        let value = json(&encode(&seasoned()));
        assert_eq!(value["flags"], serde_json::json!([]));
        assert_eq!(value["dialogs"], serde_json::json!([]));
    }

    /// `pilot` saved at version 38: no flags or story states kept.
    fn at_version_38(pilot: &Pilot) -> String {
        let mut value = json(&encode(pilot));
        value["version"] = serde_json::Value::from(38);
        let object = value.as_object_mut().expect("an object");
        object.remove("flags");
        object.remove("dialogs");
        value.to_string()
    }

    #[test]
    fn a_version_38_save_loads_with_no_flag_and_no_story_state() {
        let pilot = decode(&at_version_38(&storied())).expect("loads");
        assert_eq!(pilot, seasoned());
        let value = json(&encode(&pilot));
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["flags"], serde_json::json!([]));
        assert_eq!(value["dialogs"], serde_json::json!([]));
    }

    #[test]
    fn the_upgrade_from_38_adds_no_flags_and_no_story_states() {
        let mut value = serde_json::json!({"name": "Ada"});
        flags_and_dialogs(&mut value);
        assert_eq!(
            value,
            serde_json::json!({"name": "Ada", "flags": [], "dialogs": []})
        );
    }

    #[test]
    fn a_current_save_missing_flags_or_story_states_is_unusable() {
        for field in ["flags", "dialogs"] {
            let mut value = json(&encode(&storied()));
            value.as_object_mut().expect("an object").remove(field);
            match decode(&value.to_string()) {
                Err(SaveError::Unusable(reason)) => {
                    assert!(reason.contains(field), "{field}: {reason}");
                }
                other => panic!("{field}: {other:?}"),
            }
        }
    }
}
