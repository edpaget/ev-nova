//! The adapter for the dialog port ([`nova_sim_story::dialog`]): compiled
//! Ink stories, played by the `bladeink` runtime.
//!
//! [`InkLoader`] opens `inklecate`'s JSON, and [`InkDialog`] forwards each
//! of the port's calls to one `bladeink` call, adding only what the port
//! promises and `bladeink` doesn't check: a leading byte order mark is
//! skipped, an unknown knot, a choice past the end and an undeclared
//! variable are refused as such (an unknown knot before anything changes),
//! lines with neither text nor tags (the empty output of a divert) are
//! stepped over, and nothing steps before a knot is entered, though
//! `bladeink` would play the file's top. A [`SavedDialog`] is `bladeink`'s
//! saved state behind a first line that says whether a knot had been
//! entered.
//!
//! Only this crate may depend on `bladeink` (`deny.toml`). Its story is
//! built on `Rc`s, so an [`InkDialog`] is not `Send`.

use bladeink::story::Story;
use bladeink::value_type::ValueType;
use nova_sim_story::dialog::{Choice, Dialog, DialogError, DialogLoader, Line, SavedDialog, Value};

/// Opens compiled Ink stories with `bladeink`.
///
/// Every story it opens is seeded ([`Story::new_with_seed`]): `bladeink`'s
/// unseeded constructor draws its seed from the thread's random number
/// generator, so a story's shuffles and `RANDOM`s would differ from run to
/// run, and from test to test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InkLoader {
    seed: i32,
}

impl InkLoader {
    /// A loader whose stories draw their random numbers from `seed`.
    #[must_use]
    pub fn seeded(seed: i32) -> Self {
        Self { seed }
    }
}

impl DialogLoader for InkLoader {
    fn open(&self, compiled: &str) -> Result<Box<dyn Dialog>, DialogError> {
        // `inklecate` may write a byte order mark, which `bladeink`'s JSON
        // reader doesn't skip.
        let json = compiled.strip_prefix('\u{feff}').unwrap_or(compiled);
        let story = Story::new_with_seed(json, self.seed)
            .map_err(|e| DialogError::BadStory(e.to_string()))?;
        Ok(Box::new(InkDialog {
            story,
            entered: false,
        }))
    }
}

/// A compiled Ink story, played by `bladeink`.
pub struct InkDialog {
    story: Story,
    /// Whether a knot has been entered. `bladeink` opens a story at its
    /// top, where the file's own content (often a divert into a
    /// walkthrough) would play, so nothing steps and no choice is offered
    /// until the first [`Dialog::enter`] succeeds.
    entered: bool,
}

impl Dialog for InkDialog {
    fn enter(&mut self, knot: &str) -> Result<(), DialogError> {
        // `choose_path_string` resets the call stack and clears the choices
        // before it finds the path doesn't resolve, so the path is checked
        // first. `get_visit_count_at_path_string` resolves a knot's path as
        // it does, and only reads.
        if self.story.get_visit_count_at_path_string(knot).is_err() {
            return Err(DialogError::NoSuchKnot(knot.to_owned()));
        }
        self.story
            .choose_path_string(knot, true, None)
            .map_err(runtime)?;
        self.entered = true;
        Ok(())
    }

    fn step(&mut self) -> Result<Option<Line>, DialogError> {
        if !self.entered {
            return Ok(None);
        }
        while self.story.can_continue() {
            let text = self.story.cont().map_err(runtime)?;
            let tags = self.story.get_current_tags().map_err(runtime)?;
            let text = text.strip_suffix('\n').unwrap_or(&text).to_owned();
            if !text.is_empty() || !tags.is_empty() {
                return Ok(Some(Line { text, tags }));
            }
        }
        Ok(None)
    }

    fn choices(&self) -> Vec<Choice> {
        if !self.entered {
            return Vec::new();
        }
        self.story
            .get_current_choice_infos()
            .into_iter()
            .map(|choice| Choice {
                text: choice.text,
                tags: choice.tags,
            })
            .collect()
    }

    fn choose(&mut self, index: usize) -> Result<(), DialogError> {
        let count = self.story.get_current_choice_infos().len();
        if index >= count {
            return Err(DialogError::NoSuchChoice { index, count });
        }
        self.story.choose_choice_index(index).map_err(runtime)
    }

    fn variable(&self, name: &str) -> Result<Option<Value>, DialogError> {
        Ok(Some(match self.story.get_variable(name) {
            None => return Ok(None),
            Some(ValueType::Bool(b)) => Value::Bool(b),
            Some(ValueType::Int(n)) => Value::Int(n),
            Some(ValueType::Float(f)) => Value::Float(f),
            Some(ValueType::String(s)) => Value::Text(s.string),
            Some(_) => return Err(DialogError::Unsupported(name.to_owned())),
        }))
    }

    fn variables(&self) -> Vec<String> {
        self.story.get_global_variables()
    }

    fn set_variable(&mut self, name: &str, value: Value) -> Result<(), DialogError> {
        if self.story.get_variable(name).is_none() {
            return Err(DialogError::Undeclared(name.to_owned()));
        }
        let value = match value {
            Value::Bool(b) => ValueType::from(b),
            Value::Int(n) => ValueType::from(n),
            Value::Float(f) => ValueType::from(f),
            Value::Text(t) => ValueType::from(t.as_str()),
        };
        self.story.set_variable(name, &value).map_err(runtime)
    }

    fn save(&self) -> Result<SavedDialog, DialogError> {
        let mark = if self.entered { ENTERED } else { UNENTERED };
        let state = self.story.save_state().map_err(runtime)?;
        Ok(SavedDialog(format!("{mark}\n{state}")))
    }

    fn restore(&mut self, saved: &SavedDialog) -> Result<(), DialogError> {
        let (entered, state) = match saved.0.split_once('\n') {
            Some((ENTERED, state)) => (true, state),
            Some((UNENTERED, state)) => (false, state),
            _ => return Err(DialogError::BadSave("no entered mark".to_owned())),
        };
        self.story
            .load_state(state)
            .map_err(|e| DialogError::BadSave(e.to_string()))?;
        self.entered = entered;
        Ok(())
    }
}

/// The first line of a [`SavedDialog`] written after a knot was entered;
/// `bladeink`'s own saved state follows on the next line.
const ENTERED: &str = "entered";

/// The first line of a [`SavedDialog`] written before any knot was
/// entered, which restores to a dialog where nothing steps.
const UNENTERED: &str = "unentered";

/// A `bladeink` error raised as the story runs.
fn runtime(error: impl std::fmt::Display) -> DialogError {
    DialogError::Runtime(error.to_string())
}

/// The adapter's own tests, over its own fixture story,
/// `tests/fixtures/recall.ink`: what the port's contract story can't show
/// without running Ink (conditional text, read counts, lists, seeded
/// random draws and runtime errors). The port's contract itself is played only through
/// `check_contract`.
///
/// `recall.ink.json` is compiled by hand with Inky 0.15.2's `inklecate`
/// (ink 1.2, `inkVersion` 21), so no test needs `inklecate`. After editing
/// `recall.ink`, run from the workspace root:
///
/// ```sh
/// "$INKY_INKLECATE" -o crates/nova-ink/tests/fixtures/recall.ink.json \
///     crates/nova-ink/tests/fixtures/recall.ink
/// ```
///
/// where `$INKY_INKLECATE` is Inky's bundled compiler (on macOS,
/// `/Applications/Inky.app/Contents/Resources/app.asar.unpacked/main-process/ink/inklecate_mac`).
#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_story::dialog::fixture::check_contract;

    const RECALL_JSON: &str = include_str!("../tests/fixtures/recall.ink.json");

    fn recall() -> Box<dyn Dialog> {
        InkLoader::seeded(0).open(RECALL_JSON).unwrap()
    }

    fn lines(dialog: &mut dyn Dialog) -> Vec<String> {
        let mut lines = Vec::new();
        while let Some(line) = dialog.step().unwrap() {
            lines.push(line.text);
        }
        lines
    }

    #[test]
    fn the_bladeink_adapter_meets_the_contract() {
        assert_eq!(check_contract(&InkLoader::seeded(0)), Ok(()));
    }

    #[test]
    fn a_byte_order_mark_is_skipped() {
        let json = format!("\u{feff}{}", RECALL_JSON.trim_start_matches('\u{feff}'));
        let mut dialog = InkLoader::seeded(0).open(&json).unwrap();
        dialog.enter("ask").unwrap();
        assert_eq!(lines(dialog.as_mut()), ["The clerk looks up."]);
    }

    #[test]
    fn conditional_text_reads_variables_set_before_entering() {
        let mut dialog = recall();
        dialog.enter("recall").unwrap();
        assert_eq!(lines(dialog.as_mut()), ["Who are you?", "You never asked."]);

        dialog.set_variable("met", Value::Bool(true)).unwrap();
        dialog
            .set_variable("name", Value::Text("Ash".to_owned()))
            .unwrap();
        dialog.enter("recall").unwrap();
        assert_eq!(
            lines(dialog.as_mut()),
            ["Welcome back, Ash.", "You never asked."]
        );
    }

    #[test]
    fn read_counts_survive_save_and_restore() {
        let mut dialog = recall();
        dialog.enter("ask").unwrap();
        lines(dialog.as_mut());
        dialog.choose(0).unwrap();
        lines(dialog.as_mut());
        let saved = dialog.save().unwrap();

        let mut fresh = recall();
        fresh.restore(&saved).unwrap();
        fresh.enter("recall").unwrap();
        assert_eq!(
            lines(fresh.as_mut()),
            ["Who are you?", "You asked about work."]
        );
    }

    #[test]
    fn a_list_variable_is_unsupported() {
        assert_eq!(
            recall().variable("mood"),
            Err(DialogError::Unsupported("mood".to_owned()))
        );
    }

    #[test]
    fn a_variable_set_after_a_restore_is_read_in_text() {
        let dialog = recall();
        let saved = dialog.save().unwrap();
        assert_eq!(dialog.variable("met"), Ok(Some(Value::Bool(false))));

        let mut fresh = recall();
        fresh.restore(&saved).unwrap();
        fresh.set_variable("met", Value::Bool(true)).unwrap();
        fresh.enter("recall").unwrap();
        assert_eq!(
            lines(fresh.as_mut()),
            ["Welcome back, Tally.", "You never asked."]
        );
    }

    #[test]
    fn variables_lists_every_global_list_included() {
        assert_eq!(recall().variables(), ["met", "mood", "name"]);
    }

    /// The lines `dice` steps, in a story opened with `seed`.
    fn dice(seed: i32) -> Vec<String> {
        let mut dialog = InkLoader::seeded(seed).open(RECALL_JSON).unwrap();
        dialog.enter("dice").unwrap();
        lines(dialog.as_mut())
    }

    #[test]
    fn the_seed_is_fixed() {
        assert_eq!(dice(7), dice(7));
    }

    #[test]
    fn another_seed_draws_differently() {
        assert_ne!(dice(7), dice(8));
    }

    #[test]
    fn runtime_errors_surface_as_errors() {
        let mut dialog = recall();
        dialog.enter("broken").unwrap();
        // `bladeink` reads ahead to the line's end, so the return out of a
        // tunnel never entered fails the line it ends.
        assert!(
            matches!(dialog.step(), Err(DialogError::Runtime(_))),
            "the tunnel return outside a tunnel fails"
        );
    }
}
