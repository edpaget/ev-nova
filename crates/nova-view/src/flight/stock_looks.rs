//! The opt-in stock look's adapter: the new game's hulls drawn in stock
//! sprites borrowed from the player's own `Nova Files`
//! (`nova_data::stock_look`), over the existing [`ShipSprites`] port.
//!
//! [`StockShipLooks`] answers a hull's sheet with its borrow's: the stock
//! `shïp`'s base and its five layers (glow, lights, weapon, alt, shield),
//! their frame counts and rotation count, `BaseTransp`, `WeapDecay`, the
//! depth and the clip quirks, as the stock sheet has them, keyed so they
//! resolve only in the stock art ([`SheetImage::Stock`]) and scaled to the
//! hull's size ([`Scale::apply`] on every frame and image size). Solid runs
//! aren't overlaid (`solid` is false): a borrowed key has no `rlëD` ID for
//! them, and the runs still draw as part of the frame.
//!
//! The flight consults it after the catalog's own sheet and before the
//! outline ([`FlightView::with_ship_looks`]), so the new game's own sheet
//! always wins and the look replaces stand-ins only. Everything the
//! simulation reads (the hull's size, collision, exits, mass, handling,
//! sets, banking, animation and blink) stays the new game's: the sim never
//! sees this adapter. It never answers an outline, and a hull with no
//! borrow, or whose stock sheet doesn't read, is an error, so the flight
//! falls through to the hull's stand-in.
//!
//! Nothing it reads is written anywhere: it holds the stock art in an `Rc`
//! for the run and hands out only sheets, which name stock IDs.
//!
//! [`FlightView::with_ship_looks`]: super::FlightView::with_ship_looks

use std::collections::BTreeMap;
use std::rc::Rc;

use nova_data::StockArt;
use nova_data::stock_look::{Borrow, Scale};

use super::catalog::{LayerSheet, ShipId, ShipSheet, ShipSprites};
use super::data::sheet_of;
use crate::outline::Outline;
use crate::sheet_image::SheetImage;

/// The borrowed stock look of each new-game hull that borrows (see the
/// module docs).
#[derive(Debug)]
pub struct StockShipLooks {
    art: Rc<StockArt>,
    borrows: BTreeMap<ShipId, Borrow>,
}

impl StockShipLooks {
    /// The look of each hull in `borrows`, drawn from `art`.
    #[must_use]
    pub fn new(art: Rc<StockArt>, borrows: BTreeMap<ShipId, Borrow>) -> Self {
        Self { art, borrows }
    }
}

impl ShipSprites for StockShipLooks {
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
        let Borrow { stock, scale } = *self
            .borrows
            .get(&id)
            .ok_or_else(|| format!("shïp {} borrows no stock look", id.0))?;
        let sprite = self
            .art
            .ship_sprite_info(stock)
            .map_err(|err| format!("the stock look's {err}"))?;
        let sheet = sheet_of(
            &sprite,
            self.art.ship_layer_infos(stock),
            self.art.ship_anim(stock),
            |_, image, _| SheetImage::Stock {
                ship: stock.0,
                image,
                scale,
            },
        );
        Ok(scaled(sheet, scale))
    }

    /// Never: a borrowed look has no outline, the hull's own stand-in is
    /// the catalog's.
    fn ship_outline(&self, _id: ShipId) -> Option<Outline> {
        None
    }
}

/// `sheet` with every frame and image size scaled by `scale`, and no
/// solid runs (see the module docs).
fn scaled(sheet: ShipSheet, scale: Scale) -> ShipSheet {
    let both = |(width, height): (u32, u32)| (scale.apply(width), scale.apply(height));
    let layer = |layer: Option<LayerSheet>| {
        layer.map(|layer| {
            let (frame_width, frame_height) = both(layer.size());
            LayerSheet {
                frame_width,
                frame_height,
                image_size: both(layer.image_size),
                solid: false,
                ..layer
            }
        })
    };
    let (frame_width, frame_height) = both(sheet.size());
    ShipSheet {
        frame_width,
        frame_height,
        image_size: both(sheet.image_size),
        glow: layer(sheet.glow),
        lights: layer(sheet.lights),
        weapon: layer(sheet.weapon),
        alt: layer(sheet.alt),
        shield: layer(sheet.shield),
        solid: false,
        ..sheet
    }
}

#[cfg(test)]
mod tests {
    use nova_data::ShipImage;
    use nova_data::stock_look::fixture::{ROTATIONS, stock_art, stock_store};

    use super::*;
    use crate::flight::sheet_check::check_sheets;

    /// Stock ship 300, 40 px, and 301, 32 px.
    fn art() -> Rc<StockArt> {
        Rc::new(stock_art(&[(300, 40), (301, 32)]))
    }

    /// Hull 128 borrows 300 at 3:4, hull 129 borrows 301 at 1:1, hull 130
    /// borrows 302, which the art doesn't hold.
    fn looks() -> StockShipLooks {
        StockShipLooks::new(
            art(),
            BTreeMap::from([
                (
                    ShipId(128),
                    Borrow {
                        stock: ShipId(300),
                        scale: Scale::of(30, 40),
                    },
                ),
                (
                    ShipId(129),
                    Borrow {
                        stock: ShipId(301),
                        scale: Scale::ONE,
                    },
                ),
                (
                    ShipId(130),
                    Borrow {
                        stock: ShipId(302),
                        scale: Scale::ONE,
                    },
                ),
            ]),
        )
    }

    #[test]
    fn a_hull_that_borrows_draws_the_stock_sheet_keyed_to_the_stock_art_and_scaled() {
        let scale = Scale::of(30, 40);
        let stock = stock_store(&[(300, 40)]);
        let own = stock.ship_sheet(ShipId(300)).expect("the stock sheet");
        let found = looks().ship_sheet(ShipId(128)).expect("a borrowed sheet");
        let key = |image| SheetImage::Stock {
            ship: 300,
            image,
            scale,
        };
        assert_eq!(found.image, key(ShipImage::Base));
        assert_eq!(found.rotations.get(), ROTATIONS);
        assert_eq!(found.rotations, own.rotations);
        assert_eq!(
            found.size(),
            (scale.apply(own.frame_width), scale.apply(own.frame_height))
        );
        assert_eq!(
            found.image_size,
            (scale.apply(own.image_size.0), scale.apply(own.image_size.1))
        );
        let glow = found.glow.expect("the stock glow");
        let stock_glow = own.glow.expect("its glow");
        assert_eq!(glow.image, key(ShipImage::Glow));
        assert_eq!(glow.frames, stock_glow.frames);
        assert_eq!(
            glow.size(),
            (
                scale.apply(stock_glow.frame_width),
                scale.apply(stock_glow.frame_height)
            )
        );
        assert_eq!(
            glow.image_size,
            (
                scale.apply(stock_glow.image_size.0),
                scale.apply(stock_glow.image_size.1)
            )
        );
        assert_eq!(
            (found.lights, found.weapon, found.alt, found.shield),
            (None, None, None, None),
            "as the stock sheet has them"
        );
        assert_eq!(
            (
                found.base_transp,
                found.weap_decay,
                found.eight_bit,
                found.clip_quirks
            ),
            (
                own.base_transp,
                own.weap_decay,
                own.eight_bit,
                own.clip_quirks
            )
        );
    }

    #[test]
    fn a_borrowed_sheet_overlays_no_solid_runs_though_the_stock_one_does() {
        use nova_data::graphics::RLED;
        use nova_data::graphics::fixture::RledBuilder;
        use nova_data::stock_look::fixture::{base_rled, ship_resources};
        // Ship 302's base: two 2 x 1 frames, each a pixel then a solid run.
        let runs = RledBuilder::new(2, 1)
            .frame(|f| f.line().pixels(&[0x7C00]).run(1, 0x7FFF, 0x7FFF))
            .frame(|f| f.line().pixels(&[0x001F]).run(1, 0x7FFF, 0x7FFF))
            .build();
        let resources: Vec<_> = ship_resources(302, 2)
            .into_iter()
            .map(|(ty, id, bytes)| {
                let bytes = if ty == RLED && id == base_rled(302) {
                    runs.clone()
                } else {
                    bytes
                };
                (ty, id, bytes)
            })
            .collect();
        let stock = nova_data::store::fixture::store_of(&resources);
        assert!(stock.ship_sheet(ShipId(302)).expect("a sheet").solid);
        let looks = StockShipLooks::new(
            Rc::new(StockArt::new(stock)),
            BTreeMap::from([(
                ShipId(128),
                Borrow {
                    stock: ShipId(302),
                    scale: Scale::ONE,
                },
            )]),
        );
        let found = looks.ship_sheet(ShipId(128)).expect("a borrowed sheet");
        assert!(!found.solid);
        assert!(!found.glow.expect("a glow").solid);
        assert_eq!(found.size(), (2, 1), "1:1");
    }

    #[test]
    fn a_hull_that_borrows_nothing_or_whose_stock_ship_is_missing_has_no_sheet() {
        let looks = looks();
        assert!(looks.ship_sheet(ShipId(131)).is_err(), "no borrow");
        assert!(looks.ship_sheet(ShipId(130)).is_err(), "not in the art");
    }

    #[test]
    fn it_never_answers_an_outline() {
        let looks = looks();
        for ship in [128, 129, 130, 131] {
            assert_eq!(looks.ship_outline(ShipId(ship)), None);
        }
    }

    #[test]
    fn its_sheets_and_the_game_data_s_pass_the_sheet_check() {
        let ships = [ShipId(128), ShipId(129), ShipId(130), ShipId(300)];
        assert_eq!(check_sheets(&looks(), &ships), Ok(()));
        assert_eq!(check_sheets(&stock_store(&[(300, 40)]), &ships), Ok(()));
    }
}
