//! A conversation played a page at a time over the dialog port.
//!
//! A [`Conversation`] is what a screen shows of a story: a page of lines,
//! then the choices the story waits on ([`ConversationPage`]). Taking a
//! choice ([`Conversation::choose`]) gives the next page, until the story
//! ends.
//!
//! The conversation holds no live [`Dialog`]: a dialog is neither `Clone`
//! nor comparable, and a clone of a conversation must not step the same
//! story as the original. It keeps the story's [`SavedDialog`] instead, and
//! each choice opens the compiled text again, restores the save, takes the
//! choice, steps to the next stop and saves again. The port's contract pins
//! that a choice taken in a fresh dialog restored from a save plays on.
//!
//! # The story's state and the world flags
//!
//! A story is one Ink file ([`DialogFile`]), and the pilot keeps one state
//! per file ([`DialogStates`]), so read counts and spent once-only choices
//! carry from one scene of the file to the next. Opening a conversation
//! restores the file's kept state, if any, then sets each boolean variable
//! the story declares from the world flags (`true` while its flag is set,
//! `false` otherwise), then enters the knot. A variable of another type,
//! or one the port can't carry (a list), is the story's own and is left
//! alone; a flag the story doesn't declare is ignored. Flags reach the
//! story only as a knot is entered, not on each choice.
//!
//! The bridge runs one way: nothing reads a variable back into the flags.
//! A scene that sets one changes only its own file's state, until the next
//! entry sets it from the flags again.
//!
//! After each page that steps cleanly, opened or chosen, its save is kept
//! as the file's state. A page that fails keeps nothing, so the file's
//! state stays as it was before that page. A kept state that won't
//! restore ([`DialogError::BadSave`], say after the file was recompiled)
//! is dropped: the story opens afresh and plays on, and its new state is
//! kept in place of the old.
//!
//! A page shows the lines' text only. The raw [`Line`]s, tags included
//! (`outcome: accept`), stay readable through [`Conversation::lines`]. A
//! [`DialogError`] while opening, stepping or choosing ends the page, with
//! the error as its [`problem`](ConversationPage::problem), so a broken
//! story shows as broken rather than as no story.

use super::states::DialogStates;
use super::{Dialog, DialogError, DialogLoader, Line, SavedDialog, Value};
use crate::flags::WorldFlags;
pub use nova_sim_state::shown::dialog::conversation::ConversationPage;

/// The story a conversation plays: the Ink file, as a ref names it
/// (`dialog/dock-job.ink`), which keys its kept state, and the story
/// compiled from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DialogFile<'a> {
    /// The Ink file.
    pub file: &'a str,
    /// The story, compiled by `inklecate`.
    pub compiled: &'a str,
}

/// A choice the conversation can't take: the page is ended, or `index` is
/// past its choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("no choice {index}: {count} on offer")]
pub struct NoSuchChoice {
    /// The index asked for.
    pub index: usize,
    /// How many choices the page offers.
    pub count: usize,
}

/// A story played a page at a time, kept as its save between choices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversation {
    saved: Option<SavedDialog>,
    page: ConversationPage,
    lines: Vec<Line>,
}

impl Conversation {
    /// Opens `story`, restores the state `kept` holds for its file, sets
    /// its boolean variables from `flags`, enters `knot` (a knot or a
    /// `knot.stitch`) and steps to its first choices or its end, then keeps
    /// the page's save as the file's state (see the module docs).
    #[must_use]
    pub fn open(
        loader: &dyn DialogLoader,
        story: DialogFile<'_>,
        knot: &str,
        kept: &mut DialogStates,
        flags: &WorldFlags,
    ) -> Self {
        let mut conversation = Self {
            saved: None,
            page: ConversationPage::default(),
            lines: Vec::new(),
        };
        let opened =
            restored(loader, story.compiled, kept.get(story.file)).and_then(|mut dialog| {
                set_flags(dialog.as_mut(), flags)?;
                dialog.enter(knot)?;
                Ok(dialog)
            });
        conversation.play(opened);
        conversation.keep(story.file, kept);
        conversation
    }

    /// Takes the choice at `index` on the page, steps to the next choices
    /// or the end, and keeps the new page's save as `story`'s file's state
    /// in `kept`.
    ///
    /// # Errors
    ///
    /// [`NoSuchChoice`] when the page is ended or `index` is past its
    /// choices. The conversation and `kept` are then unchanged.
    pub fn choose(
        &mut self,
        loader: &dyn DialogLoader,
        story: DialogFile<'_>,
        kept: &mut DialogStates,
        index: usize,
    ) -> Result<(), NoSuchChoice> {
        let count = self.page.choices.len();
        // An ended page has no save to play on from, and offers nothing.
        let Some(saved) = self.saved.clone().filter(|_| index < count) else {
            return Err(NoSuchChoice { index, count });
        };
        let reopened = loader.open(story.compiled).and_then(|mut dialog| {
            dialog.restore(&saved)?;
            dialog.choose(index)?;
            Ok(dialog)
        });
        self.play(reopened);
        self.keep(story.file, kept);
        Ok(())
    }

    /// Keeps the page's save as `file`'s state in `kept`, when the page
    /// stepped cleanly; a failed page keeps nothing.
    fn keep(&self, file: &str, kept: &mut DialogStates) {
        if let Some(saved) = &self.saved {
            kept.keep(file, saved.clone());
        }
    }

    /// Starts a new page from `dialog`, stepping it to its next stop and
    /// saving it there, or ends the page with the error.
    fn play(&mut self, dialog: Result<Box<dyn Dialog>, DialogError>) {
        self.page = ConversationPage::default();
        self.lines.clear();
        self.saved = None;
        if let Err(error) = dialog.and_then(|mut dialog| self.step_to_a_stop(dialog.as_mut())) {
            self.page = ConversationPage {
                lines: std::mem::take(&mut self.page.lines),
                choices: Vec::new(),
                ended: true,
                problem: Some(error.to_string()),
            };
        }
    }

    /// Steps `dialog` to its choices or its end, filling the page, then
    /// saves it.
    fn step_to_a_stop(&mut self, dialog: &mut dyn Dialog) -> Result<(), DialogError> {
        while let Some(line) = dialog.step()? {
            self.page.lines.push(line.text.clone());
            self.lines.push(line);
        }
        self.page.choices = dialog.choices().into_iter().map(|c| c.text).collect();
        self.page.ended = self.page.choices.is_empty();
        self.saved = Some(dialog.save()?);
        Ok(())
    }

    /// The page to show.
    #[must_use]
    pub fn page(&self) -> &ConversationPage {
        &self.page
    }

    /// The page's lines as the story stepped them, tags kept.
    #[must_use]
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }
}

/// The story compiled to `compiled`, opened with `kept` restored into it,
/// or opened afresh when `kept` won't restore.
fn restored(
    loader: &dyn DialogLoader,
    compiled: &str,
    kept: Option<&SavedDialog>,
) -> Result<Box<dyn Dialog>, DialogError> {
    let mut dialog = loader.open(compiled)?;
    if let Some(saved) = kept {
        match dialog.restore(saved) {
            Ok(()) => {}
            // The pilot's state, not the author's story, is at fault: drop
            // it rather than lock the player out of the file for good.
            Err(DialogError::BadSave(_)) => dialog = loader.open(compiled)?,
            Err(error) => return Err(error),
        }
    }
    Ok(dialog)
}

/// Sets each boolean variable `dialog` declares from `flags`: `true` while
/// its flag is set, `false` otherwise. Any other variable is left alone.
fn set_flags(dialog: &mut dyn Dialog, flags: &WorldFlags) -> Result<(), DialogError> {
    for name in dialog.variables() {
        if let Ok(Some(Value::Bool(_))) = dialog.variable(&name) {
            dialog.set_variable(&name, Value::Bool(flags.is_set(&name)))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::fake::{
        DialogCall, FakeChoice, FakeDialog, FakeKnot, FakeLoader, StateCall,
    };

    const STORY: &str = "story";
    const FILE: &str = "dialog/offer.ink";

    fn story() -> DialogFile<'static> {
        DialogFile {
            file: FILE,
            compiled: STORY,
        }
    }

    /// An offer: two lines, then three choices. `Ask` diverts to `route`,
    /// which offers two more; `Take` ends with an outcome; `Broken` diverts
    /// nowhere.
    fn loader() -> FakeLoader {
        let offer = FakeKnot::new("offer")
            .line("The dockmaster looks up.", &[])
            .line("\"A job?\"", &["mood: busy"])
            .choice(
                FakeChoice::sticky("Take", &["c: 1"]).line("\"Bay two.\"", &["outcome: accept"]),
            )
            .choice(
                FakeChoice::sticky("Ask", &[])
                    .line("\"Two gates.\"", &[])
                    .then("route"),
            )
            .choice(
                FakeChoice::sticky("Broken", &[])
                    .line("\"Hm.\"", &[])
                    .then("nowhere"),
            );
        let route = FakeKnot::new("route")
            .line("\"Either will do.\"", &[])
            .choice(FakeChoice::sticky("Take it", &[]).line("\"Then go.\"", &["outcome: accept"]))
            .choice(
                FakeChoice::sticky("Pass", &[]).line("\"Suit yourself.\"", &["outcome: decline"]),
            );
        FakeLoader::default().story(STORY, FakeDialog::default().knot(offer).knot(route))
    }

    /// Opens [`story`] at `knot` with nothing kept and no flag set.
    fn open(loader: &FakeLoader, knot: &str) -> Conversation {
        Conversation::open(
            loader,
            story(),
            knot,
            &mut DialogStates::default(),
            &WorldFlags::default(),
        )
    }

    /// Takes choice `index` of [`story`], keeping into a throwaway.
    fn choose(
        conversation: &mut Conversation,
        loader: &FakeLoader,
        index: usize,
    ) -> Result<(), NoSuchChoice> {
        conversation.choose(loader, story(), &mut DialogStates::default(), index)
    }

    fn page(lines: &[&str], choices: &[&str], ended: bool) -> ConversationPage {
        ConversationPage {
            lines: lines.iter().map(|&l| l.to_owned()).collect(),
            choices: choices.iter().map(|&c| c.to_owned()).collect(),
            ended,
            problem: None,
        }
    }

    fn line(text: &str, tags: &[&str]) -> Line {
        Line {
            text: text.to_owned(),
            tags: tags.iter().map(|&t| t.to_owned()).collect(),
        }
    }

    fn flags(names: &[&str]) -> WorldFlags {
        names.iter().map(|&n| n.to_owned()).collect()
    }

    fn set(name: &str, value: bool) -> StateCall {
        StateCall::Set(name.to_owned(), Value::Bool(value))
    }

    #[test]
    fn opening_steps_every_line_to_the_first_choices() {
        let conversation = open(&loader(), "offer");
        assert_eq!(
            conversation.page(),
            &page(
                &["The dockmaster looks up.", "\"A job?\""],
                &["Take", "Ask", "Broken"],
                false
            )
        );
    }

    #[test]
    fn a_page_shows_text_only_while_the_lines_keep_their_tags() {
        let conversation = open(&loader(), "offer");
        assert_eq!(
            conversation.lines(),
            [
                line("The dockmaster looks up.", &[]),
                line("\"A job?\"", &["mood: busy"])
            ]
        );
    }

    #[test]
    fn choosing_shows_only_the_lines_after_the_choice_with_the_new_choices() {
        let loader = loader();
        let mut conversation = open(&loader, "offer");
        assert_eq!(choose(&mut conversation, &loader, 1), Ok(()));
        assert_eq!(
            conversation.page(),
            &page(
                &["\"Two gates.\"", "\"Either will do.\""],
                &["Take it", "Pass"],
                false
            )
        );
    }

    #[test]
    fn the_last_page_is_ended_with_no_choices_and_keeps_its_outcome() {
        let loader = loader();
        let mut conversation = open(&loader, "offer");
        choose(&mut conversation, &loader, 1).unwrap();
        choose(&mut conversation, &loader, 1).unwrap();
        assert_eq!(
            conversation.page(),
            &page(&["\"Suit yourself.\""], &[], true)
        );
        assert_eq!(
            conversation.lines(),
            [line("\"Suit yourself.\"", &["outcome: decline"])]
        );
    }

    #[test]
    fn a_choice_past_the_end_is_refused_and_changes_nothing() {
        let loader = loader();
        let mut kept = DialogStates::default();
        let mut conversation =
            Conversation::open(&loader, story(), "offer", &mut kept, &WorldFlags::default());
        let (before, kept_before) = (conversation.clone(), kept.clone());
        assert_eq!(
            conversation.choose(&loader, story(), &mut kept, 3),
            Err(NoSuchChoice { index: 3, count: 3 })
        );
        assert_eq!(conversation, before);
        assert_eq!(kept, kept_before, "nothing new is kept");
    }

    #[test]
    fn choosing_on_an_ended_page_is_refused() {
        let loader = loader();
        let mut conversation = open(&loader, "offer");
        choose(&mut conversation, &loader, 0).unwrap();
        assert!(conversation.page().ended);
        let before = conversation.clone();
        assert_eq!(
            choose(&mut conversation, &loader, 0),
            Err(NoSuchChoice { index: 0, count: 0 })
        );
        assert_eq!(conversation, before);
    }

    #[test]
    fn a_story_that_fails_to_open_ends_with_its_problem() {
        let conversation = Conversation::open(
            &loader(),
            DialogFile {
                file: FILE,
                compiled: "not a story",
            },
            "offer",
            &mut DialogStates::default(),
            &WorldFlags::default(),
        );
        assert_eq!(
            conversation.page(),
            &ConversationPage {
                ended: true,
                problem: Some("not a compiled Ink story: the fake has no such story".to_owned()),
                ..ConversationPage::default()
            }
        );
    }

    #[test]
    fn a_missing_knot_ends_with_its_problem() {
        let conversation = open(&loader(), "nowhere");
        assert_eq!(
            conversation.page().problem.as_deref(),
            Some("no knot or stitch `nowhere`")
        );
        assert!(conversation.page().ended);
    }

    #[test]
    fn a_story_that_fails_to_step_ends_with_its_problem_and_the_lines_before_it() {
        let loader = loader();
        let mut conversation = open(&loader, "offer");
        assert_eq!(choose(&mut conversation, &loader, 2), Ok(()));
        assert_eq!(
            conversation.page(),
            &ConversationPage {
                lines: vec!["\"Hm.\"".to_owned()],
                ended: true,
                problem: Some("the story failed: no knot `nowhere` to divert to".to_owned()),
                ..ConversationPage::default()
            }
        );
    }

    #[test]
    fn a_choice_whose_story_no_longer_opens_ends_with_its_problem() {
        let loader = loader();
        let mut conversation = open(&loader, "offer");
        let changed = DialogFile {
            file: FILE,
            compiled: "changed",
        };
        assert_eq!(
            conversation.choose(&loader, changed, &mut DialogStates::default(), 0),
            Ok(())
        );
        assert!(conversation.page().ended);
        assert_eq!(
            conversation.page().problem.as_deref(),
            Some("not a compiled Ink story: the fake has no such story")
        );
    }

    #[test]
    fn a_clone_that_chooses_leaves_the_original_page_unchanged() {
        let loader = loader();
        let original = open(&loader, "offer");
        let mut clone = original.clone();
        choose(&mut clone, &loader, 1).unwrap();
        assert_eq!(original.page().choices, ["Take", "Ask", "Broken"]);
        let mut original = original;
        choose(&mut original, &loader, 0).unwrap();
        assert_eq!(original.page(), &page(&["\"Bay two.\""], &[], true));
    }

    #[test]
    fn each_choice_reopens_the_story_and_restores_it() {
        let loader = loader();
        let mut conversation = open(&loader, "offer");
        choose(&mut conversation, &loader, 1).unwrap();
        choose(&mut conversation, &loader, 0).unwrap();
        assert_eq!(
            loader.log(),
            [
                DialogCall::Enter("offer".to_owned()),
                DialogCall::Choose(1),
                DialogCall::Choose(0),
            ]
        );
        assert_eq!(loader.state_log(), [StateCall::Restore, StateCall::Restore]);
        assert_eq!(conversation.page(), &page(&["\"Then go.\""], &[], true));
    }

    // The story's state and the flags.

    /// A story declaring two flags, `regular` and `banned`, an int
    /// `count` and a once-only choice; its first line reads `regular`.
    fn flagged() -> FakeLoader {
        let offer = FakeKnot::new("offer")
            .line_when("regular", Value::Bool(true), "\"You again.\"", &[])
            .line("\"A job?\"", &[])
            .choice(FakeChoice::once("Ask", &[]).line("\"Two gates.\"", &[]))
            .choice(FakeChoice::sticky("Leave", &[]).line("\"Bye.\"", &[]));
        let dialog = FakeDialog::default()
            .declare("regular", Value::Bool(false))
            .declare("banned", Value::Bool(true))
            .declare("count", Value::Int(4))
            .knot(offer);
        FakeLoader::default().story(STORY, dialog)
    }

    /// The state [`flagged`]'s story saves with `regular` and `banned`
    /// set to `value` and its `Ask` spent.
    fn spent_with(value: bool) -> SavedDialog {
        let loader = flagged();
        let mut dialog = loader.open(STORY).unwrap();
        dialog.set_variable("regular", Value::Bool(value)).unwrap();
        dialog.set_variable("banned", Value::Bool(value)).unwrap();
        dialog.enter("offer").unwrap();
        while dialog.step().unwrap().is_some() {}
        dialog.choose(0).unwrap();
        while dialog.step().unwrap().is_some() {}
        dialog.save().unwrap()
    }

    /// What the dialog saves after `knot`'s first page, with `flags` set
    /// and nothing restored.
    fn first_page_save(loader: &FakeLoader, knot: &str, flags: &[&str]) -> SavedDialog {
        let mut dialog = loader.open(STORY).unwrap();
        for name in dialog.variables() {
            if let Ok(Some(Value::Bool(_))) = dialog.variable(&name) {
                let value = flags.contains(&name.as_str());
                dialog.set_variable(&name, Value::Bool(value)).unwrap();
            }
        }
        dialog.enter(knot).unwrap();
        while dialog.step().unwrap().is_some() {}
        dialog.save().unwrap()
    }

    #[test]
    fn with_nothing_kept_opening_sets_each_flag_enters_and_keeps_the_page() {
        let loader = flagged();
        let mut kept = DialogStates::default();
        let conversation = Conversation::open(
            &loader,
            story(),
            "offer",
            &mut kept,
            &flags(&["regular", "elsewhere"]),
        );
        assert_eq!(
            loader.state_log(),
            [set("banned", false), set("regular", true)],
            "no restore; each boolean set from the flags"
        );
        assert_eq!(loader.log(), [DialogCall::Enter("offer".to_owned())]);
        assert_eq!(
            conversation.page(),
            &page(&["\"You again.\"", "\"A job?\""], &["Ask", "Leave"], false)
        );
        assert_eq!(
            kept.get(FILE),
            Some(&first_page_save(&loader, "offer", &["regular"]))
        );
        assert_eq!(kept.get(FILE), conversation.saved.as_ref());
    }

    #[test]
    fn with_a_state_kept_opening_restores_it_then_sets_the_flags() {
        let loader = flagged();
        let mut kept = DialogStates::default();
        kept.keep(FILE, spent_with(true));
        let conversation =
            Conversation::open(&loader, story(), "offer", &mut kept, &WorldFlags::default());
        assert_eq!(
            loader.state_log(),
            [
                StateCall::Restore,
                set("banned", false),
                set("regular", false)
            ]
        );
        assert_eq!(
            conversation.page(),
            &page(&["\"A job?\""], &["Leave"], false),
            "the kept `true` reads `false` with the flag clear, and `Ask` stays spent"
        );
    }

    #[test]
    fn an_int_variable_and_a_flag_with_no_variable_are_left_alone() {
        let loader = flagged();
        let _ = Conversation::open(
            &loader,
            story(),
            "offer",
            &mut DialogStates::default(),
            &flags(&["count", "elsewhere"]),
        );
        assert!(
            loader
                .state_log()
                .iter()
                .all(|call| matches!(call, StateCall::Set(name, _) if name != "count" && name != "elsewhere")),
            "{:?}",
            loader.state_log()
        );
    }

    /// A dialog whose `mood` variable is a list: listed, but
    /// [`DialogError::Unsupported`] to read or set.
    struct Listed(Box<dyn Dialog>);

    impl Dialog for Listed {
        fn enter(&mut self, knot: &str) -> Result<(), DialogError> {
            self.0.enter(knot)
        }
        fn step(&mut self) -> Result<Option<Line>, DialogError> {
            self.0.step()
        }
        fn choices(&self) -> Vec<super::super::Choice> {
            self.0.choices()
        }
        fn choose(&mut self, index: usize) -> Result<(), DialogError> {
            self.0.choose(index)
        }
        fn variable(&self, name: &str) -> Result<Option<Value>, DialogError> {
            if name == "mood" {
                return Err(DialogError::Unsupported(name.to_owned()));
            }
            self.0.variable(name)
        }
        fn variables(&self) -> Vec<String> {
            let mut names = self.0.variables();
            names.push("mood".to_owned());
            names.sort();
            names
        }
        fn set_variable(&mut self, name: &str, value: Value) -> Result<(), DialogError> {
            if name == "mood" {
                return Err(DialogError::Unsupported(name.to_owned()));
            }
            self.0.set_variable(name, value)
        }
        fn save(&self) -> Result<SavedDialog, DialogError> {
            self.0.save()
        }
        fn restore(&mut self, saved: &SavedDialog) -> Result<(), DialogError> {
            self.0.restore(saved)
        }
    }

    struct ListedLoader(FakeLoader);

    impl DialogLoader for ListedLoader {
        fn open(&self, compiled: &str) -> Result<Box<dyn Dialog>, DialogError> {
            Ok(Box::new(Listed(self.0.open(compiled)?)))
        }
    }

    #[test]
    fn an_unsupported_variable_is_skipped_and_the_page_plays() {
        let loader = ListedLoader(flagged());
        let conversation = Conversation::open(
            &loader,
            story(),
            "offer",
            &mut DialogStates::default(),
            &flags(&["mood", "regular"]),
        );
        assert_eq!(
            conversation.page(),
            &page(&["\"You again.\"", "\"A job?\""], &["Ask", "Leave"], false)
        );
        assert_eq!(
            loader.0.state_log(),
            [set("banned", false), set("regular", true)]
        );
    }

    #[test]
    fn a_kept_state_that_will_not_restore_is_dropped_for_a_fresh_one() {
        let loader = flagged();
        let mut kept = DialogStates::default();
        kept.keep(FILE, SavedDialog("from an older story".to_owned()));
        let conversation =
            Conversation::open(&loader, story(), "offer", &mut kept, &WorldFlags::default());
        assert_eq!(conversation.page().problem, None);
        assert_eq!(
            conversation.page(),
            &page(&["\"A job?\""], &["Ask", "Leave"], false)
        );
        assert_eq!(
            kept.get(FILE),
            Some(&first_page_save(&loader, "offer", &[]))
        );
    }

    /// A loader whose dialogs refuse every restore with `error`.
    struct Refusing(FakeLoader, DialogError);

    struct RefusingDialog(Box<dyn Dialog>, DialogError);

    impl Dialog for RefusingDialog {
        fn enter(&mut self, knot: &str) -> Result<(), DialogError> {
            self.0.enter(knot)
        }
        fn step(&mut self) -> Result<Option<Line>, DialogError> {
            self.0.step()
        }
        fn choices(&self) -> Vec<super::super::Choice> {
            self.0.choices()
        }
        fn choose(&mut self, index: usize) -> Result<(), DialogError> {
            self.0.choose(index)
        }
        fn variable(&self, name: &str) -> Result<Option<Value>, DialogError> {
            self.0.variable(name)
        }
        fn variables(&self) -> Vec<String> {
            self.0.variables()
        }
        fn set_variable(&mut self, name: &str, value: Value) -> Result<(), DialogError> {
            self.0.set_variable(name, value)
        }
        fn save(&self) -> Result<SavedDialog, DialogError> {
            self.0.save()
        }
        fn restore(&mut self, _saved: &SavedDialog) -> Result<(), DialogError> {
            Err(self.1.clone())
        }
    }

    impl DialogLoader for Refusing {
        fn open(&self, compiled: &str) -> Result<Box<dyn Dialog>, DialogError> {
            Ok(Box::new(RefusingDialog(
                self.0.open(compiled)?,
                self.1.clone(),
            )))
        }
    }

    #[test]
    fn a_restore_that_fails_otherwise_ends_the_page_and_keeps_the_state() {
        let error = DialogError::Runtime("disk on fire".to_owned());
        let loader = Refusing(flagged(), error.clone());
        let mut kept = DialogStates::default();
        kept.keep(FILE, spent_with(true));
        let conversation =
            Conversation::open(&loader, story(), "offer", &mut kept, &WorldFlags::default());
        assert_eq!(conversation.page().problem, Some(error.to_string()));
        assert_eq!(kept.get(FILE), Some(&spent_with(true)));
        assert_eq!(loader.0.log(), [], "nothing entered");
    }

    #[test]
    fn each_choice_keeps_the_new_page_s_save() {
        let loader = loader();
        let mut kept = DialogStates::default();
        let mut conversation =
            Conversation::open(&loader, story(), "offer", &mut kept, &WorldFlags::default());
        let opened = kept.get(FILE).cloned().expect("kept");
        conversation.choose(&loader, story(), &mut kept, 1).unwrap();
        let chosen = kept.get(FILE).cloned().expect("kept");
        assert_ne!(chosen, opened);
        assert_eq!(Some(&chosen), conversation.saved.as_ref());
        conversation.choose(&loader, story(), &mut kept, 0).unwrap();
        assert_ne!(kept.get(FILE), Some(&chosen));
        assert_eq!(kept.get(FILE), conversation.saved.as_ref());
    }

    #[test]
    fn a_story_that_fails_to_open_or_step_keeps_the_file_s_state_as_it_was() {
        let loader = loader();
        let before = SavedDialog("before".to_owned());
        let mut kept = DialogStates::default();
        kept.keep(FILE, before.clone());
        let broken = DialogFile {
            file: FILE,
            compiled: "not a story",
        };
        let _ = Conversation::open(&loader, broken, "offer", &mut kept, &WorldFlags::default());
        assert_eq!(kept.get(FILE), Some(&before), "failed to open");

        let mut elsewhere = DialogStates::default();
        let mut conversation = Conversation::open(
            &loader,
            story(),
            "offer",
            &mut elsewhere,
            &WorldFlags::default(),
        );
        conversation.choose(&loader, story(), &mut kept, 2).unwrap();
        assert!(conversation.page().problem.is_some());
        assert_eq!(kept.get(FILE), Some(&before), "failed to step");
    }

    #[test]
    fn two_files_keep_two_states() {
        let loader = loader();
        let mut kept = DialogStates::default();
        let other = SavedDialog("other".to_owned());
        kept.keep("dialog/other.ink", other.clone());
        let mut conversation =
            Conversation::open(&loader, story(), "offer", &mut kept, &WorldFlags::default());
        conversation.choose(&loader, story(), &mut kept, 1).unwrap();
        assert_eq!(kept.get("dialog/other.ink"), Some(&other));
        assert_eq!(kept.get(FILE), conversation.saved.as_ref());
        assert_eq!(kept.iter().count(), 2);
    }

    #[test]
    fn a_clone_that_chooses_with_its_own_states_leaves_the_original_s_alone() {
        let loader = loader();
        let mut kept = DialogStates::default();
        let original =
            Conversation::open(&loader, story(), "offer", &mut kept, &WorldFlags::default());
        let mut theirs = kept.clone();
        let mut clone = original.clone();
        clone.choose(&loader, story(), &mut theirs, 1).unwrap();
        assert_eq!(kept.get(FILE), original.saved.as_ref());
        assert_ne!(theirs, kept);
    }

    #[test]
    fn a_once_only_choice_spent_in_one_conversation_is_spent_in_the_next() {
        let loader = flagged();
        let mut kept = DialogStates::default();
        let mut first =
            Conversation::open(&loader, story(), "offer", &mut kept, &WorldFlags::default());
        first.choose(&loader, story(), &mut kept, 0).unwrap();
        let second =
            Conversation::open(&loader, story(), "offer", &mut kept, &WorldFlags::default());
        assert_eq!(second.page().choices, ["Leave"]);
    }
}
