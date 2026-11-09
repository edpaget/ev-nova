//! The flight's second ship sprites: a look consulted where the catalog
//! has no sheet, before the outline ([`FlightView::with_ship_looks`]).
//!
//! A ship type is read once, in this order:
//!
//! 1. the catalog's own sheet, which always wins;
//! 2. when that can't be read, the looks' sheet (the opt-in stock look,
//!    `StockShipLooks`), and no outline;
//! 3. when that can't be read either, the catalog's outline, or with none
//!    the crossed box, kept with the catalog's reason.
//!
//! The looks are a collaborator of the same [`ShipSprites`] port, given by
//! the edge, and only the view reads them: the flight behind the
//! [`FlightSim`] port never sees them, so the simulation runs and saves the
//! same with them or without. Without looks, the view reads and draws as
//! it always did.

use std::fmt;
use std::rc::Rc;

use super::{FlightView, Outline, ShipId, ShipSheet};
use crate::flight::catalog::ShipSprites;
use crate::flight::sim_port::FlightSim;

/// The looks the flight may consult, if the edge gave any.
#[derive(Clone, Default)]
pub(super) struct ShipLooks(Option<Rc<dyn ShipSprites>>);

impl fmt::Debug for ShipLooks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ShipLooks").field(&self.0.is_some()).finish()
    }
}

/// Ship type `ship`'s sheet and, when it can't be shown, its outline:
/// `catalog`'s sheet, else `looks`'s, else `catalog`'s outline (see the
/// module docs).
pub(super) fn read_ship<C: ShipSprites + ?Sized>(
    catalog: &C,
    looks: &ShipLooks,
    ship: ShipId,
) -> (Result<ShipSheet, String>, Option<Outline>) {
    let sheet = catalog.ship_sheet(ship);
    if sheet.is_err()
        && let Some(Ok(borrowed)) = looks.0.as_ref().map(|looks| looks.ship_sheet(ship))
    {
        return (Ok(borrowed), None);
    }
    let outline = super::outline_for(catalog, ship, &sheet);
    (sheet, outline)
}

impl<C: ShipSprites, S: FlightSim> FlightView<C, S> {
    /// The flight with `looks` consulted for each ship type the catalog
    /// has no sheet for, before its outline (see the module docs). The
    /// player's sheet and every NPC type's sheet already read are read
    /// again, so it may come before or after any other builder.
    #[must_use]
    pub fn with_ship_looks(mut self, looks: Rc<dyn ShipSprites>) -> Self {
        self.ship_looks = ShipLooks(Some(looks));
        if let Some(ship) = self.sheet_ship {
            (self.sheet, self.outline) = read_ship(&*self.catalog, &self.ship_looks, ship);
        }
        let read: Vec<ShipId> = self.npc_sheets.keys().copied().collect();
        for ship in read {
            let (sheet, outline) = read_ship(&*self.catalog, &self.ship_looks, ship);
            match outline {
                Some(outline) => self.npc_outlines.insert(ship, outline),
                None => self.npc_outlines.remove(&ship),
            };
            self.npc_sheets.insert(ship, sheet);
        }
        self
    }
}
