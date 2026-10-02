//! Reader for classic Mac OS resource forks, as stored in EV Nova's `.ndat` files.
//!
//! # Design decision
//!
//! This crate parses resource forks with a small in-house strict parser
//! rather than wrapping the [`macbinary`](https://crates.io/crates/macbinary)
//! crate. `macbinary` is kept as a dev-dependency only, as an independent
//! second reading of the stock data in the `stock` tests.
//!
//! The decision came from a spike that ran `macbinary` 0.2.1 over the 21
//! stock `.ndat` files and probed its public API with corrupted forks:
//!
//! - On stock data `macbinary` reads well: across 30 types and 8,362
//!   resources, every type's iterated count equals the count declared in the
//!   file's type list, and the known totals match (288 `shïp`, 791 `mïsn`,
//!   545 `sÿst`, 671 `PICT`).
//! - Its strictness gaps cannot be closed from outside the crate. An
//!   out-of-bounds data entry silently ends the `Resources` iterator early
//!   (only a short count is visible, not which resource failed). An
//!   out-of-bounds reference list shows up only as a `(0, None)` size hint.
//!   An out-of-bounds *name* offset becomes `name: None`, which is
//!   indistinguishable from an unnamed resource. The attribute byte (and so
//!   the compressed flag), all offsets and the declared counts are private.
//!
//! Nova plug-ins are hand-edited, so malformed input must be reported rather
//! than dropped, and the format is small. Sending the `.ok() // FIXME` fixes
//! upstream to `macbinary` remains possible future work; if that lands, this
//! crate could switch to it behind the same public API.
