//! The opening's [`Names`]: the names fixed for a flight's life, built once
//! from what the session read as it started ([`Statics`]), for the view
//! to word its messages with (see [`FlightOpening`](crate::flight_frame::FlightOpening)).

use crate::flight_frame::{Names, OutfitNames, ShipNames};
use crate::market::{COMMODITIES, Good};

use super::super::Statics;

impl Statics {
    /// The names of every ship class, outfit, good traded, `jünk` and
    /// stellar the session read, each as the session's own lookup gives it
    /// (`Session::ship_name`, `Statics::outfit_names`, `Statics::good_name`,
    /// `Statics::junk_lc_name`, `TagNames::stellar_name`): the first
    /// record of an ID names it, as those lookups find the first.
    pub(in crate::session) fn names(&self) -> Names {
        let mut names = Names::default();
        for record in &self.ships {
            names.ships.entry(record.id).or_insert_with(|| ShipNames {
                name: record.name.clone(),
                long_name: record.long_name.clone(),
            });
        }
        for record in &self.outfits {
            names
                .outfits
                .entry(record.id)
                .or_insert_with(|| OutfitNames {
                    name: record.name.clone(),
                    lc_name: record.lc_name.clone(),
                    lc_plural: record.lc_plural.clone(),
                });
        }
        let junk = self.goods.junk();
        let goods = (0..COMMODITIES)
            .map(Good::Commodity)
            .chain(junk.iter().map(|record| Good::Junk(record.id)));
        for good in goods {
            if let Some(name) = self.goods.name(good) {
                names.goods.entry(good).or_insert_with(|| name.to_owned());
            }
        }
        for record in junk {
            names
                .junk
                .entry(record.id)
                .or_insert_with(|| record.lc_name.clone());
        }
        names.stellars.clone_from(&self.place_names.stellars);
        names
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{
        JunkId, JunkRecord, OutfitId, OutfitRecord, ShipId, ShipRecord, StellarId,
    };
    use crate::market::Good;
    use crate::session::Session;
    use crate::testkit::{FakePilotCatalog, outfit};
    use crate::text_tags::TagNames;

    /// The ship catalog with a long name on each class, two outfits, a
    /// `jünk` and two named stellars.
    fn named() -> FakePilotCatalog {
        let mut catalog = crate::session::tests::shipbuying();
        for record in &mut catalog.ship_records {
            record.long_name = format!("Long {}", record.name);
        }
        catalog.ship_records.push(ShipRecord {
            name: "Second 128".to_owned(),
            ..catalog.ship_records[0].clone()
        });
        catalog.outfits.push(OutfitRecord {
            lc_name: "blaster".to_owned(),
            lc_plural: "blasters".to_owned(),
            ..outfit(150, &[])
        });
        catalog.junk.push(JunkRecord {
            id: JunkId(150),
            name: "Bio-weapons".to_owned(),
            abbrev: "Bio".to_owned(),
            base_price: 1000,
            sold_at: Vec::new(),
            bought_at: Vec::new(),
            buy_on: crate::control::Test::default(),
            sell_on: crate::control::Test::default(),
            flags: 0,
            scan_mask: 0,
            lc_name: "bio-weapons".to_owned(),
        });
        catalog
            .place_names
            .stellars
            .insert(StellarId(128), "Earth".to_owned());
        catalog
            .place_names
            .stellars
            .insert(StellarId(129), "Luna".to_owned());
        catalog
    }

    #[test]
    fn the_openings_names_are_the_sessions_own() {
        let catalog = named();
        let session = Session::start(&catalog).expect("starts");
        let names = session.opening().names;
        let ships = [ShipId(128), ShipId(129), ShipId(999)];
        for ship in ships {
            assert_eq!(names.ship(ship), session.ship_name(ship), "{ship:?}");
            assert_eq!(
                names.ship_long(ship),
                session.statics.ship_long_name(ship),
                "{ship:?}"
            );
        }
        assert!(names.ship(ShipId(128)).is_some(), "a ship named");
        assert_eq!(names.ship(ShipId(128)), session.ship_name(ShipId(128)));
        assert_eq!(names.ship_long(ShipId(129)), Some("Long Ship 129"));
        let outfits = [OutfitId(150), OutfitId(999)]
            .into_iter()
            .chain(catalog.outfits.iter().map(|record| record.id));
        for outfit in outfits {
            assert_eq!(names.outfit(outfit), session.statics.outfit_name(outfit));
            assert_eq!(
                names.outfit_lc(outfit),
                session.statics.outfit_names(outfit)
            );
        }
        assert_eq!(
            names.outfit_lc(OutfitId(150)),
            Some(("blaster", "blasters"))
        );
        let goods = (0..=crate::market::COMMODITIES)
            .map(Good::Commodity)
            .chain([Good::Junk(JunkId(150)), Good::Junk(JunkId(999))]);
        for good in goods {
            assert_eq!(
                names.good(good),
                session.statics.good_name(good),
                "{good:?}"
            );
        }
        assert_eq!(names.good(Good::Junk(JunkId(150))), Some("Bio-weapons"));
        assert!(names.good(Good::Commodity(0)).is_some(), "food is traded");
        for junk in [JunkId(150), JunkId(999)] {
            assert_eq!(names.junk_lc(junk), session.statics.junk_lc_name(junk));
        }
        assert_eq!(names.junk_lc(JunkId(150)), Some("bio-weapons"));
        for stellar in [StellarId(128), StellarId(129), StellarId(999)] {
            assert_eq!(
                names.stellar(stellar).map(str::to_owned),
                TagNames::stellar_name(&session, stellar),
                "{stellar:?}"
            );
        }
        assert_eq!(names.stellar(StellarId(129)), Some("Luna"));
    }

    #[test]
    fn a_failed_flights_opening_names_nothing() {
        let failed = crate::flight_frame::FlightOpening::failed("no chär");
        assert_eq!(failed.names, crate::flight_frame::Names::default());
        assert_eq!(failed.names.ship(ShipId(128)), None);
        assert_eq!(failed.names.stellar(StellarId(128)), None);
    }
}
