//! The scroll arrows of a text dialog: items 5 and 6 of `DLOG` 3003 and
//! 3004.

/// One of a text's two scroll arrows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollArrow {
    /// Toward the text's start: item 5.
    Up,
    /// Toward the text's end: item 6.
    Down,
}
