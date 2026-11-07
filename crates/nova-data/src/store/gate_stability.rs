//! Each jump point's stability, kept on the store beside its gate.
//!
//! The new game's jump points are hypergate `spöb`s (`crate::new_game`'s
//! `gates`), and a `spöb` has no word for how well a jump through it works.
//! The loader keeps each point's own [`GateStability`] here instead, keyed
//! by its gate's `spöb`: [`GameData::with_gate_stabilities`] sets them, and
//! [`GameData::gate_stability`] answers them. A jump between two points
//! works as well as the worse of its two ends
//! (`nova_galaxy_format::model::jump_stability`); the store keeps each end.
//!
//! The stock game's store holds none: every stock gate answers `None`.

use std::collections::BTreeMap;

/// How well jumps through a jump point work: the bible's own `stability`,
/// ordered stable, then unstable, then dead.
pub use nova_galaxy_format::model::Stability as GateStability;

use super::GameData;
use crate::wire::id::StellarId;

impl GameData {
    /// The store with `stabilities`, each jump point's own by its gate's
    /// `spöb`, replacing any it had.
    #[must_use]
    pub fn with_gate_stabilities(
        mut self,
        stabilities: BTreeMap<StellarId, GateStability>,
    ) -> Self {
        self.gate_stabilities = stabilities;
        self
    }

    /// The stability of the jump point whose gate is `gate`, if the store
    /// keeps one: only the new game's do (see the module docs).
    #[must_use]
    pub fn gate_stability(&self, gate: StellarId) -> Option<GateStability> {
        self.gate_stabilities.get(&gate).copied()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn a_store_of_decoded_records_keeps_no_stability() {
        let data = GameData::from_decoded(PathBuf::from("/game"), Vec::new());
        assert_eq!(data.gate_stability(StellarId(128)), None);
    }

    #[test]
    fn a_stock_store_keeps_no_stability() {
        assert_eq!(GameData::default().gate_stability(StellarId(128)), None);
    }

    #[test]
    fn a_store_with_stabilities_answers_each_gate_its_own() {
        let data = GameData::default().with_gate_stabilities(BTreeMap::from([
            (StellarId(128), GateStability::Unstable),
            (StellarId(130), GateStability::Dead),
        ]));
        assert_eq!(
            data.gate_stability(StellarId(128)),
            Some(GateStability::Unstable)
        );
        assert_eq!(
            data.gate_stability(StellarId(130)),
            Some(GateStability::Dead)
        );
        assert_eq!(data.gate_stability(StellarId(129)), None);
    }

    #[test]
    fn stabilities_replace_those_set_before() {
        let data = GameData::default()
            .with_gate_stabilities(BTreeMap::from([(StellarId(128), GateStability::Dead)]))
            .with_gate_stabilities(BTreeMap::from([(StellarId(129), GateStability::Stable)]));
        assert_eq!(data.gate_stability(StellarId(128)), None);
        assert_eq!(
            data.gate_stability(StellarId(129)),
            Some(GateStability::Stable)
        );
    }
}
