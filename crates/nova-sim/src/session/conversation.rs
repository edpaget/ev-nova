//! The landed stellar's conversation: opened as the spaceport opens, a
//! choice at a time, until the player closes it or takes off.
//!
//! The new game names a conversation on a stellar (the catalog's
//! `stellar_dialogs`, kept on [`Statics`]). The view opens it with
//! [`Converse::Open`](crate::flight_frame::Converse::Open) once it has
//! landed, sends each choice the player takes, and closes it when the
//! story has ended (see the `apply/conversation` module). The session
//! keeps the open one on the [`Port`] as a plain value
//! ([`Conversation`]), its story saved between choices and opened again
//! for each through the dialog port the game wires
//! ([`Session::with_dialog_loader`]), so a clone of the session never
//! shares a live story. A session with no loader shows the conversation
//! as one that can't open.
//!
//! Leaving and landing again opens the conversation afresh from its knot:
//! nothing of it is saved (engine-story phase 3 keeps the story's state).
//! The stock game names no conversations, so nothing here runs for it.

use std::fmt;
use std::rc::Rc;

use crate::catalog::StellarId;
use crate::dialog::conversation::{Conversation, ConversationPage};
use crate::dialog::{Dialog, DialogError, DialogLoader};

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

/// The conversation open at a stellar: where, and how far it has played.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct OpenConversation {
    stellar: StellarId,
    conversation: Conversation,
}

impl Setup {
    /// The loader the game wired, or one that opens nothing.
    fn dialog_loader(&self) -> &dyn DialogLoader {
        self.dialogs.as_ref().map_or(&NoLoader, |loader| &*loader.0)
    }
}

impl Port {
    /// Opens the conversation of `landed`, the stellar the ship is landed
    /// at, from its knot, replacing any open one: its first page, or
    /// `None` in flight or where the stellar names none.
    pub(super) fn open_conversation(
        &mut self,
        statics: &Statics,
        setup: &Setup,
        landed: Option<StellarId>,
    ) -> Option<ConversationPage> {
        let stellar = landed?;
        let (compiled, knot) = statics.stellar_dialogs.place(stellar)?;
        let conversation = Conversation::open(setup.dialog_loader(), compiled, knot);
        let page = conversation.page().clone();
        self.conversation = Some(OpenConversation {
            stellar,
            conversation,
        });
        Some(page)
    }

    /// Takes the choice at `index` on the open conversation's page: the
    /// next page, or `None` with none open at `landed`, on an ended page,
    /// or past its choices, which changes nothing.
    pub(super) fn choose_in_conversation(
        &mut self,
        statics: &Statics,
        setup: &Setup,
        landed: Option<StellarId>,
        index: usize,
    ) -> Option<ConversationPage> {
        let open = self
            .conversation
            .as_mut()
            .filter(|open| Some(open.stellar) == landed)?;
        let (compiled, _) = statics.stellar_dialogs.place(open.stellar)?;
        open.conversation
            .choose(setup.dialog_loader(), compiled, index)
            .ok()?;
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
