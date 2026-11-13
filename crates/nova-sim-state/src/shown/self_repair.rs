//! What a former escort was, [`FormerKind`], from
//! `nova_sim_combat::self_repair`, which re-exports it, keeps the
//! self-repair and the rejoin, and reads a kind off an escort's record
//! (`nova_sim_combat::self_repair::FormerKindOf`).

/// What a former escort was, as `_DamageShip` sets +0xc8de: 0, 1 or 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormerKind {
    /// 0: a fighter the player launched (AI type 5).
    Fighter,
    /// 1: an escort hired in the bar (the hired flag +0xbb).
    Hired,
    /// 2: any other escort: captured, the old ship kept after "Use As My
    /// Ship", or a person enlisted by a hail, which the original never
    /// marks hired.
    Captured,
}
