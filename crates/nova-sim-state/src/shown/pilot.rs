//! How far a system is explored ([`ExploredLevel`]), from
//! `nova_sim_story::pilot`, which re-exports it and keeps the pilot.

/// How far a system is explored: the original's `sÿst` +0x90, which
/// takes 0, 1 or 2 and is only ever raised.
///
/// A jump's arrival, a gate's exit, `X` and a new pilot raise a system to
/// [`Explored`](Self::Explored) (level 1); a take-off and every map grant
/// to [`Charted`](Self::Charted) (level 2). Only the galaxy map's panel
/// tells the two apart: a level-1 system's goods and services read
/// `<Unknown>` there (`_MapDialogUpdate` @0x10f34, @0x1107d, @0x11110).
/// Every other reader asks only whether a system is explored at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExploredLevel {
    /// Level 0: never been in, never charted.
    Unexplored,
    /// Level 1: been in, or explored by `X`.
    Explored,
    /// Level 2: taken off from, or charted by a map.
    Charted,
}
