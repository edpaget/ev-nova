//! The dialog port: compiled Ink stories, played a line at a time.
//!
//! The new game writes its conversations in Ink (`data/dialog/<file>.ink`),
//! and the author compiles each with `inklecate` to JSON. The story layer
//! plays that JSON through this port, in its own terms: open a compiled
//! story ([`DialogLoader::open`]), enter a knot or a stitch
//! ([`Dialog::enter`]), step its lines with their tags ([`Dialog::step`]),
//! list the choices it waits on ([`Dialog::choices`]), take one
//! ([`Dialog::choose`]), list, read and write its variables, and save and restore
//! its state ([`Dialog::save`], [`Dialog::restore`]).
//!
//! The adapter is `nova-ink`'s `nova_ink::InkLoader`, over the `bladeink`
//! runtime; only that crate may reach `bladeink` (`deny.toml`). With the
//! `fixture` feature (and in this crate's tests) this module publishes the
//! hand-written fake (`fake`) and the contract every implementation passes
//! (`fixture::check_contract`), played over the contract story
//! (`fixture::CONTRACT_JSON`). They are named as inline code because the
//! default build, which rustdoc documents, leaves them out.
//!
//! A dialog holds no `Send` bound: `bladeink`'s story is built on `Rc`s.
//! A dialog that must cross a thread is kept as its [`SavedDialog`] and
//! its compiled text, and opened again on the other side.
//!
//! [`conversation`] plays a story a page at a time over this port, for a
//! screen to show: the lines, then the choices, kept as a save between
//! choices so the conversation is a plain value.
//!
//! [`states`] keeps one story state per Ink file
//! ([`DialogStates`](states::DialogStates)), so a file's read counts and
//! spent choices carry from one of its scenes to the next.

pub mod conversation;
#[cfg(any(test, feature = "fixture"))]
pub mod fake;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod states;

/// Opens compiled Ink stories.
pub trait DialogLoader {
    /// Opens the story compiled to `compiled` (`inklecate`'s JSON), at no
    /// knot: nothing steps until [`Dialog::enter`] picks one. Its
    /// variables hold their declared values.
    ///
    /// # Errors
    ///
    /// [`DialogError::BadStory`] when `compiled` isn't a compiled story.
    fn open(&self, compiled: &str) -> Result<Box<dyn Dialog>, DialogError>;
}

/// One open story, played a line at a time.
pub trait Dialog {
    /// Moves to `knot`, a knot's name or a `knot.stitch`, dropping the
    /// call stack and any choices that were pending.
    ///
    /// # Errors
    ///
    /// [`DialogError::NoSuchKnot`] when the story has no such knot. A failed
    /// enter changes nothing: the dialog stays where it was, with the same
    /// choices pending.
    fn enter(&mut self, knot: &str) -> Result<(), DialogError>;

    /// The next line, with its trailing newline trimmed, or `None` once
    /// the story waits on a choice or has ended. Lines with no text and
    /// no tags are skipped.
    ///
    /// # Errors
    ///
    /// [`DialogError::Runtime`] when the story fails as it runs.
    fn step(&mut self) -> Result<Option<Line>, DialogError>;

    /// The choices the story waits on, in order: empty while lines remain
    /// or once it has ended.
    fn choices(&self) -> Vec<Choice>;

    /// Takes the choice at `index` in [`Dialog::choices`].
    ///
    /// # Errors
    ///
    /// [`DialogError::NoSuchChoice`] when `index` is past the choices.
    fn choose(&mut self, index: usize) -> Result<(), DialogError>;

    /// The value of the global variable `name`, or `None` if the story
    /// declares no such variable.
    ///
    /// # Errors
    ///
    /// [`DialogError::Unsupported`] when the variable holds a list, a
    /// divert target or a variable pointer.
    fn variable(&self, name: &str) -> Result<Option<Value>, DialogError>;

    /// The names of the global variables the story declares (its `VAR`s,
    /// lists included), sorted by name, each once.
    fn variables(&self) -> Vec<String>;

    /// Sets the global variable `name`.
    ///
    /// # Errors
    ///
    /// [`DialogError::Undeclared`] when the story declares no such
    /// variable.
    fn set_variable(&mut self, name: &str, value: Value) -> Result<(), DialogError>;

    /// The story's whole state: its variables, its read counts and where
    /// it is.
    ///
    /// # Errors
    ///
    /// [`DialogError::Runtime`] when the state can't be written.
    fn save(&self) -> Result<SavedDialog, DialogError>;

    /// Puts back a state [`Dialog::save`] wrote, from this dialog or
    /// another opened from the same compiled story.
    ///
    /// # Errors
    ///
    /// [`DialogError::BadSave`] when `saved` isn't such a state.
    fn restore(&mut self, saved: &SavedDialog) -> Result<(), DialogError>;
}

/// A line of the story: its text and its raw tags (`speaker: clerk`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// The text, with no trailing newline.
    pub text: String,
    /// The line's tags, as written after `#`, trimmed.
    pub tags: Vec<String>,
}

/// A choice the story offers: its text and its raw tags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// The text the player picks.
    pub text: String,
    /// The choice's own tags (written inside its brackets).
    pub tags: Vec<String>,
}

/// A scalar value of an Ink variable. Ink's integers and floats are 32
/// bits wide.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `true` or `false`.
    Bool(bool),
    /// A whole number.
    Int(i32),
    /// A decimal number.
    Float(f32),
    /// A string.
    Text(String),
}

/// A dialog's saved state, opaque to everything but the dialog that wrote
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedDialog(pub String);

/// What can go wrong playing a dialog.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DialogError {
    /// The text isn't a compiled story.
    #[error("not a compiled Ink story: {0}")]
    BadStory(String),
    /// The story has no such knot or stitch.
    #[error("no knot or stitch `{0}`")]
    NoSuchKnot(String),
    /// The choice index is past the choices on offer.
    #[error("no choice {index}: {count} on offer")]
    NoSuchChoice {
        /// The index asked for.
        index: usize,
        /// How many choices there are.
        count: usize,
    },
    /// The story declares no such variable.
    #[error("no variable `{0}` is declared")]
    Undeclared(String),
    /// The variable holds a list, a divert target or a variable pointer.
    #[error("variable `{0}` holds a value the port doesn't carry")]
    Unsupported(String),
    /// The story failed as it ran.
    #[error("the story failed: {0}")]
    Runtime(String),
    /// The saved state isn't one this story wrote.
    #[error("not a saved state of this story: {0}")]
    BadSave(String),
}
