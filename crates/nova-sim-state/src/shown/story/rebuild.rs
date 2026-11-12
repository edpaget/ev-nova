//! Which of its sheets a stellar shown draws ([`DrawnSheet`]), from
//! `nova_sim_story::story::rebuild`, which re-exports it and keeps the
//! rebuild that decides it.

/// Which of its sheets a stellar shown draws (see the `story` module's
/// "Built by phase 11" and "Built by stellar-fidelity phase 1"): decided
/// once, by
/// `Session::drawn_sheet`, and read by the click box and the views.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DrawnSheet {
    /// Its `Type` sheet.
    #[default]
    Type,
    /// Its `DeadType` sheet.
    Dead,
    /// Graphic type 0's sheet (`spïn` 1000), which the original's rebuild
    /// fills an empty sprite slot with.
    TypeZero,
    /// No frame: its sprite slot holds the frames of a sheet that reads
    /// but shows none of them, as a slot cloning a `spïn` whose `xTiles` ×
    /// `yTiles` is 0 or less is left from a rebuild point (see the `story`
    /// module's "Built by stellar-fidelity phase 8"). Nothing is drawn.
    NoFrame,
}
