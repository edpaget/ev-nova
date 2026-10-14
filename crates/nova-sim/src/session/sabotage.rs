//! The player's sabotage outfits (see [`sabotage`](crate::sabotage)).

use super::Session;

impl Session {
    /// The `dësc` IDs the player sees as it dies, from the outfits it
    /// owns ([`death_descs`](crate::sabotage::death_descs)).
    #[must_use]
    pub fn sabotage_death_descs(&self) -> Vec<i16> {
        crate::sabotage::death_descs(&self.pilot.outfits, &self.outfits)
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::OutfitId;
    use crate::sabotage::SABOTAGE_KILL;
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::outfit;

    #[test]
    fn a_sessions_sabotage_death_descs_come_from_the_pilots_outfits() {
        let mut catalog = armed();
        catalog.outfits.push(outfit(261, &[(SABOTAGE_KILL, 4100)]));
        let mut session = facing_an_npc(&catalog, 180);
        assert_eq!(session.sabotage_death_descs(), Vec::<i16>::new());
        session.pilot.outfits.insert(OutfitId(261), 1);
        assert_eq!(session.sabotage_death_descs(), [4100]);
    }
}
