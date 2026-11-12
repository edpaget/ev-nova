//! What a conversation screen shows ([`ConversationPage`]), from
//! `nova_sim_story::dialog::conversation`, which re-exports it and keeps
//! the conversation that steps it.

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
