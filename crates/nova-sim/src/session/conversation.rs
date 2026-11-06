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
//! The story's state is the pilot's: each page that steps cleanly keeps
//! its save under the stellar's Ink file in [`Pilot::dialogs`], and
//! opening restores it and sets the story's boolean variables from
//! [`Pilot::flags`] before entering the knot (see
//! [`conversation`](crate::dialog::conversation)). Leaving and landing
//! again opens the conversation at its knot once more, but with the file's
//! read counts and spent choices kept, and they go into the pilot's save.
//! The stock game names no conversations, so nothing here runs for it.

use std::fmt;
use std::rc::Rc;

use crate::catalog::StellarId;
use crate::dialog::conversation::{Conversation, ConversationPage, DialogFile};
use crate::dialog::{Dialog, DialogError, DialogLoader};
use crate::pilot::Pilot;

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
    /// at, from its knot, replacing any open one, with `pilot`'s state of
    /// its file and its flags: its first page, or `None` in flight or
    /// where the stellar names none.
    pub(super) fn open_conversation(
        &mut self,
        statics: &Statics,
        setup: &Setup,
        pilot: &mut Pilot,
        landed: Option<StellarId>,
    ) -> Option<ConversationPage> {
        let stellar = landed?;
        let at = statics.stellar_dialogs.place(stellar)?;
        let story = DialogFile {
            file: at.file,
            compiled: at.compiled,
        };
        let conversation = Conversation::open(
            setup.dialog_loader(),
            story,
            at.knot,
            &mut pilot.dialogs,
            &pilot.flags,
        );
        let page = conversation.page().clone();
        self.conversation = Some(OpenConversation {
            stellar,
            conversation,
        });
        Some(page)
    }

    /// Takes the choice at `index` on the open conversation's page,
    /// keeping the new page's state in `pilot`: the next page, or `None`
    /// with none open at `landed`, on an ended page, or past its choices,
    /// which changes nothing.
    pub(super) fn choose_in_conversation(
        &mut self,
        statics: &Statics,
        setup: &Setup,
        pilot: &mut Pilot,
        landed: Option<StellarId>,
        index: usize,
    ) -> Option<ConversationPage> {
        let open = self
            .conversation
            .as_mut()
            .filter(|open| Some(open.stellar) == landed)?;
        let at = statics.stellar_dialogs.place(open.stellar)?;
        let story = DialogFile {
            file: at.file,
            compiled: at.compiled,
        };
        open.conversation
            .choose(setup.dialog_loader(), story, &mut pilot.dialogs, index)
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
