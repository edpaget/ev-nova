//! Errors from parsing and loading resource forks.

/// Why a byte buffer is not a valid resource fork.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// The buffer is shorter than the 16-byte header.
    HeaderTruncated,
}
