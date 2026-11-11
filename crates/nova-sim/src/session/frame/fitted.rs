//! The player's kits as the frame carries them
//! ([`FlightFrame::player_kits`](crate::flight_frame::FlightFrame::player_kits)):
//! the outfits it carries, counted as its hardpoints are, that are kits
//! in the fittings, by ascending `oütf`. They are the same set, in the
//! same order, that its hull fires from ([`Statics::spots`]), so the view
//! draws a kit's mount exactly when the ship can fire from it. Stock data
//! has no fittings, so a stock pilot carries none.

use std::collections::BTreeMap;

use super::super::{Session, Statics};
use crate::catalog::OutfitId;

impl Statics {
    /// The kits among `carried`, by ascending `oütf`: each outfit carried
    /// (a count above 0) that the fittings list as a kit.
    pub(in crate::session) fn kits_carried<'a>(
        &'a self,
        carried: &'a BTreeMap<OutfitId, u16>,
    ) -> impl Iterator<Item = OutfitId> + 'a {
        carried
            .iter()
            .filter(|&(_, &count)| count > 0)
            .map(|(&outfit, _)| outfit)
            .filter(|outfit| self.fittings.kits.contains_key(outfit))
    }
}

impl Session {
    /// Fills `into` with the player's kits (see the module docs).
    pub(super) fn fill_player_kits(&self, into: &mut Vec<OutfitId>) {
        into.clear();
        let carried = self.ship.counted_outfits(&self.pilot);
        into.extend(self.statics.kits_carried(&carried));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nova_sim_catalog::fittings::{Fittings, Kit};

    use super::super::tests::stale;
    use crate::catalog::OutfitId;
    use crate::session::Session;
    use crate::testkit::{FakePilotCatalog, catalog};

    const BED: OutfitId = OutfitId(131);
    const RACK: OutfitId = OutfitId(140);
    const PLATING: OutfitId = OutfitId(133);
    const SOLD: OutfitId = OutfitId(135);

    /// The test catalog with kits [`BED`], [`RACK`] and [`SOLD`].
    fn fitted() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.fittings = Fittings {
            hulls: BTreeMap::new(),
            kits: [BED, RACK, SOLD]
                .into_iter()
                .map(|kit| (kit, Kit::default()))
                .collect(),
            ..Fittings::default()
        };
        catalog
    }

    /// The kits `catalog`'s session carrying `outfits` fills over a stale
    /// frame.
    fn kits_of(catalog: &FakePilotCatalog, outfits: &[(OutfitId, u16)]) -> Vec<OutfitId> {
        let mut session = Session::start(catalog).expect("starts");
        session.pilot.outfits.extend(outfits.iter().copied());
        let mut frame = stale();
        session.fill_frame(&mut frame);
        frame.player_kits
    }

    #[test]
    fn the_player_s_kits_are_those_carried_by_ascending_id() {
        let carried = [(RACK, 1), (PLATING, 2), (SOLD, 0), (BED, 3)];
        assert_eq!(kits_of(&fitted(), &carried), [BED, RACK]);
    }

    #[test]
    fn a_stock_store_s_player_carries_no_kits() {
        assert_eq!(kits_of(&catalog(), &[(BED, 1), (RACK, 1)]), []);
    }
}
