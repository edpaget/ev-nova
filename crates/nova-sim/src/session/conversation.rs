//! The landed visit's conversations: each opened in turn as the spaceport
//! opens and after each closes, a choice at a time, until the player
//! closes it or takes off.
//!
//! The new game names a conversation on a stellar (the catalog's
//! `stellar_dialogs`, kept on [`Statics`]), and on each mission's offer
//! and completion (its `storylines`). The view opens the next one with
//! [`Converse::Open`](crate::flight_frame::Converse::Open) once it has
//! landed, and again after each closes: completions due, then offers,
//! then the stellar's own, each once a visit, and refused once none is
//! left (see the `storylines` module, which also reads an offer's
//! outcome). It sends each choice the player takes, and closes it when
//! the story has ended (see the `apply/conversation` module). The session
//! keeps the open one on the [`Port`] as a plain value
//! ([`Conversation`]), its story saved between choices and opened again
//! for each through the dialog port the game wires
//! ([`Session::with_dialog_loader`]), so a clone of the session never
//! shares a live story. A session with no loader shows the conversation
//! as one that can't open.
//!
//! The story's state is the pilot's: each page that steps cleanly keeps
//! its save under the stellar's Ink file in [`Pilot::dialogs`], and
//! opening restores it and sets the story's boolean variables from
//! [`Pilot::flags`] before entering the knot (see
//! [`conversation`](crate::dialog::conversation)). Leaving and landing
//! again opens the conversation at its knot once more, but with the file's
//! read counts and spent choices kept, and they go into the pilot's save.
//! A mission's scenes are played the same way, keyed by their own Ink
//! file, so they share its state with every other scene of that file. The
//! stock game names no conversations, so nothing here runs for it.

use std::fmt;
use std::rc::Rc;

use crate::catalog::StellarId;
use crate::dialog::conversation::{Conversation, ConversationPage, DialogFile};
use crate::dialog::{Dialog, DialogError, DialogLoader};
use crate::pilot::Pilot;
use crate::storylines::lifecycle::Scene;

use super::storylines::{scene_place, settle_outcome};
use super::{Port, Session, Setup, Statics};

/// The dialog port a session keeps, shared: equal to another when it is
/// the same one.
#[derive(Clone)]
pub(super) struct Loader(Rc<dyn DialogLoader>);

impl fmt::Debug for Loader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DialogLoader")
    }
}

impl PartialEq for Loader {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The loader a session has before the game wires one: it opens nothing.
struct NoLoader;

impl DialogLoader for NoLoader {
    fn open(&self, _compiled: &str) -> Result<Box<dyn Dialog>, DialogError> {
        Err(DialogError::BadStory(
            "no dialog runtime is wired".to_owned(),
        ))
    }
}

/// The conversation open at a stellar: where, which scene, how far it has
/// played, and whether its offer has been decided.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct OpenConversation {
    stellar: StellarId,
    /// The scene it plays: the stellar's own, or a mission's offer or
    /// completion.
    pub(super) scene: Scene,
    pub(super) conversation: Conversation,
    /// Whether its page has reached an outcome, which decided its offer
    /// (see the `storylines` module).
    pub(super) decided: bool,
}

impl OpenConversation {
    /// `conversation`, of `scene`, just opened at `stellar`, undecided.
    pub(super) fn new(stellar: StellarId, scene: Scene, conversation: Conversation) -> Self {
        Self {
            stellar,
            scene,
            conversation,
            decided: false,
        }
    }
}

impl Setup {
    /// The loader the game wired, or one that opens nothing.
    pub(super) fn dialog_loader(&self) -> &dyn DialogLoader {
        self.dialogs.as_ref().map_or(&NoLoader, |loader| &*loader.0)
    }
}

impl Port {
    /// Takes the choice at `index` on the open conversation's page,
    /// keeping the new page's state in `pilot`, then decides its offer by
    /// the outcome reached, against `room` tons free (see the
    /// `storylines` module): the next page, or `None` with none open at
    /// `landed`, on an ended page, or past its choices, which changes
    /// nothing.
    pub(super) fn choose_in_conversation(
        &mut self,
        statics: &Statics,
        setup: &Setup,
        pilot: &mut Pilot,
        landed: Option<StellarId>,
        index: usize,
        room: i64,
    ) -> Option<ConversationPage> {
        let open = self
            .conversation
            .as_mut()
            .filter(|open| Some(open.stellar) == landed)?;
        let at = scene_place(statics, &open.scene)?;
        let story = DialogFile {
            file: at.file,
            compiled: at.compiled,
        };
        open.conversation
            .choose(setup.dialog_loader(), story, &mut pilot.dialogs, index)
            .ok()?;
        settle_outcome(open, statics, pilot, room);
        Some(open.conversation.page().clone())
    }

    /// Closes the open conversation: whether one was open.
    pub(super) fn close_conversation(&mut self) -> bool {
        self.conversation.take().is_some()
    }
}

impl Session {
    /// This session with `loader` opening the stellars' compiled
    /// conversations (see the module docs): none by default, so each shows
    /// that it can't open. The game wires `nova-ink`'s.
    #[must_use]
    pub fn with_dialog_loader(mut self, loader: Rc<dyn DialogLoader>) -> Self {
        self.setup.dialogs = Some(Loader(loader));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::fake::FakeLoader;

    #[test]
    fn a_loader_is_equal_only_to_itself_and_debugs_by_its_port() {
        let one: Rc<dyn DialogLoader> = Rc::new(FakeLoader::default());
        let other: Rc<dyn DialogLoader> = Rc::new(FakeLoader::default());
        assert_eq!(Loader(Rc::clone(&one)), Loader(Rc::clone(&one)));
        assert_ne!(Loader(one), Loader(other.clone()));
        assert_eq!(format!("{:?}", Loader(other)), "DialogLoader");
    }

    #[test]
    fn a_setup_with_no_loader_opens_nothing() {
        let error = Setup::default().dialog_loader().open("{}").err();
        assert_eq!(
            error,
            Some(DialogError::BadStory(
                "no dialog runtime is wired".to_owned()
            ))
        );
    }
}
