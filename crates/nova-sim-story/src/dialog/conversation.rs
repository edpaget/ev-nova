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
//! A page shows the lines' text only. The raw [`Line`]s, tags included
//! (`outcome: accept`), stay readable through [`Conversation::lines`]. A
//! [`DialogError`] while opening, stepping or choosing ends the page, with
//! the error as its [`problem`](ConversationPage::problem), so a broken
//! story shows as broken rather than as no story.

use super::{Dialog, DialogError, DialogLoader, Line, SavedDialog};

/// What a conversation screen shows: the page's lines, then its choices.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConversationPage {
    /// The lines stepped since the last choice, text only.
    pub lines: Vec<String>,
    /// The choices the story waits on, text only, in order. Empty once it
    /// has ended.
    pub choices: Vec<String>,
    /// Whether the story has ended (no choices remain), so the page only
    /// closes.
    pub ended: bool,
    /// The story's error, when it failed: the page is then ended.
    pub problem: Option<String>,
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
    /// Opens the story compiled to `compiled`, enters `knot` (a knot or a
    /// `knot.stitch`) and steps to its first choices or its end.
    #[must_use]
    pub fn open(loader: &dyn DialogLoader, compiled: &str, knot: &str) -> Self {
        let mut conversation = Self {
            saved: None,
            page: ConversationPage::default(),
            lines: Vec::new(),
        };
        let opened = loader.open(compiled).and_then(|mut dialog| {
            dialog.enter(knot)?;
            Ok(dialog)
        });
        conversation.play(opened);
        conversation
    }

    /// Takes the choice at `index` on the page, and steps to the next
    /// choices or the end.
    ///
    /// # Errors
    ///
    /// [`NoSuchChoice`] when the page is ended or `index` is past its
    /// choices. The conversation is then unchanged.
    pub fn choose(
        &mut self,
        loader: &dyn DialogLoader,
        compiled: &str,
        index: usize,
    ) -> Result<(), NoSuchChoice> {
        let count = self.page.choices.len();
        // An ended page has no save to play on from, and offers nothing.
        let Some(saved) = self.saved.clone().filter(|_| index < count) else {
            return Err(NoSuchChoice { index, count });
        };
        let reopened = loader.open(compiled).and_then(|mut dialog| {
            dialog.restore(&saved)?;
            dialog.choose(index)?;
            Ok(dialog)
        });
        self.play(reopened);
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::fake::{DialogCall, FakeChoice, FakeDialog, FakeKnot, FakeLoader};

    const STORY: &str = "story";

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

    #[test]
    fn opening_steps_every_line_to_the_first_choices() {
        let conversation = Conversation::open(&loader(), STORY, "offer");
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
        let conversation = Conversation::open(&loader(), STORY, "offer");
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
        let mut conversation = Conversation::open(&loader, STORY, "offer");
        assert_eq!(conversation.choose(&loader, STORY, 1), Ok(()));
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
        let mut conversation = Conversation::open(&loader, STORY, "offer");
        conversation.choose(&loader, STORY, 1).unwrap();
        conversation.choose(&loader, STORY, 1).unwrap();
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
        let mut conversation = Conversation::open(&loader, STORY, "offer");
        let before = conversation.clone();
        assert_eq!(
            conversation.choose(&loader, STORY, 3),
            Err(NoSuchChoice { index: 3, count: 3 })
        );
        assert_eq!(conversation, before);
    }

    #[test]
    fn choosing_on_an_ended_page_is_refused() {
        let loader = loader();
        let mut conversation = Conversation::open(&loader, STORY, "offer");
        conversation.choose(&loader, STORY, 0).unwrap();
        assert!(conversation.page().ended);
        let before = conversation.clone();
        assert_eq!(
            conversation.choose(&loader, STORY, 0),
            Err(NoSuchChoice { index: 0, count: 0 })
        );
        assert_eq!(conversation, before);
    }

    #[test]
    fn a_story_that_fails_to_open_ends_with_its_problem() {
        let conversation = Conversation::open(&loader(), "not a story", "offer");
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
        let conversation = Conversation::open(&loader(), STORY, "nowhere");
        assert_eq!(
            conversation.page().problem.as_deref(),
            Some("no knot or stitch `nowhere`")
        );
        assert!(conversation.page().ended);
    }

    #[test]
    fn a_story_that_fails_to_step_ends_with_its_problem_and_the_lines_before_it() {
        let loader = loader();
        let mut conversation = Conversation::open(&loader, STORY, "offer");
        assert_eq!(conversation.choose(&loader, STORY, 2), Ok(()));
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
        let mut conversation = Conversation::open(&loader, STORY, "offer");
        assert_eq!(conversation.choose(&loader, "changed", 0), Ok(()));
        assert!(conversation.page().ended);
        assert_eq!(
            conversation.page().problem.as_deref(),
            Some("not a compiled Ink story: the fake has no such story")
        );
    }

    #[test]
    fn a_clone_that_chooses_leaves_the_original_page_unchanged() {
        let loader = loader();
        let original = Conversation::open(&loader, STORY, "offer");
        let mut clone = original.clone();
        clone.choose(&loader, STORY, 1).unwrap();
        assert_eq!(original.page().choices, ["Take", "Ask", "Broken"]);
        let mut original = original;
        original.choose(&loader, STORY, 0).unwrap();
        assert_eq!(original.page(), &page(&["\"Bay two.\""], &[], true));
    }

    #[test]
    fn each_choice_reopens_the_story_and_restores_it() {
        let loader = loader();
        let mut conversation = Conversation::open(&loader, STORY, "offer");
        conversation.choose(&loader, STORY, 1).unwrap();
        conversation.choose(&loader, STORY, 0).unwrap();
        assert_eq!(
            loader.log(),
            [
                DialogCall::Enter("offer".to_owned()),
                DialogCall::Choose(1),
                DialogCall::Choose(0),
            ]
        );
        assert_eq!(conversation.page(), &page(&["\"Then go.\""], &[], true));
    }
}
