//! The messages a flight session raises, as events: what happened, not
//! the words. The view decides what each one says on the message line.

use crate::catalog::SystemId;

/// Something a [`Session`](crate::Session) did that the player is told of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimMessage {
    /// A jump ended in this system, the last of a multi-jump.
    Arrived(SystemId),
}
