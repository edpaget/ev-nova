//! Image keys: which decoded image a draw command shows.

/// The kinds of image resource the renderer can resolve. Closed, so an
/// unsupported resource type cannot be named.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImageKind {
    /// A `PICT` picture: one frame.
    Pict,
    /// A `PICT` picture from the interface file (`Nova-DF.rsrc`, or
    /// `Nova.rez` on Windows), such as a dialog's picture item: one frame.
    /// It resolves only in the interface file, never in the game data, so a
    /// game-data `PICT` with the same ID never shadows it (nor it one).
    InterfacePict,
    /// An `rlëD` sprite sheet: one or more equal-sized frames.
    Rled,
    /// An `rlëD`'s solid runs alone (its opcode-4 runs of one colour), one
    /// frame per sheet frame: drawn plainly over the sheet's own frame, as
    /// the original's blits fill a run raw over every blend and colorize.
    RledSolid,
    /// A `PICT` picture whose alpha comes from another `PICT`, its mask:
    /// opaque where the mask is dark, clear where it is light. Nova's
    /// button caps are drawn this way.
    MaskedPict {
        /// The mask `PICT`'s ID.
        mask: i16,
    },
    /// A `ppat` pixel pattern: one frame, drawn tiled.
    Pattern,
    /// A `cicn` colour icon: one frame, its mask as alpha.
    Icon,
    /// The frames `GameData::spin_sheet` resolves `spïn` `id` to, used for
    /// a `spïn` drawn from a `PICT` pair, which has no `rlëD` to key it
    /// by (see [`SheetImage`](crate::SheetImage)).
    SpinSheet,
    /// The frames `GameData::ship_image` resolves `shïp` `id`'s `image`
    /// to, used for a ship image drawn from a `PICT` pair, which has no
    /// `rlëD` to key it by (see [`SheetImage`](crate::SheetImage)).
    ShipSheet {
        /// Which of the ship's images.
        image: nova_data::ShipImage,
    },
    /// The frames stock `shïp` `id`'s `image` resolves to in the player's
    /// own stock data (`nova_data::StockArt`), scaled by `scale`: a
    /// new-game hull's borrowed look (`nova_data::stock_look`). It
    /// resolves only in the stock art, never in the game data, so a
    /// game-data image with the same ID never stands in for it (nor it
    /// for one).
    StockShip {
        /// Which of the stock ship's images.
        image: nova_data::ShipImage,
        /// How much each frame is scaled.
        scale: nova_data::stock_look::Scale,
    },
    /// An `rlëD`'s frames as the original draws a frame whose box the
    /// world's edge cuts (`_BlitPixieRLEClipped`, see
    /// `nova_data::graphics::decode_rled_clipped`): each the box's size,
    /// its `top` rows and `left` pixels, outside the world, clear.
    RledClipped {
        /// The box's rows above the world's top edge.
        top: u16,
        /// The box's pixels left of the world's left edge.
        left: u16,
    },
    /// An `rlëD`'s solid runs alone ([`ImageKind::RledSolid`]) as the
    /// clipped blit draws them at a cut ([`ImageKind::RledClipped`]): the
    /// clipped draw's runs, drawn plainly over its frame, as the original's
    /// clipped blits fill a run raw too.
    RledClippedSolid {
        /// The box's rows above the world's top edge.
        top: u16,
        /// The box's pixels left of the world's left edge.
        left: u16,
    },
}

/// One frame of one image resource: (resource type, ID, frame).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageKey {
    /// The resource type.
    pub kind: ImageKind,
    /// The resource ID.
    pub id: i16,
    /// The frame within the resource; always 0 for a picture.
    pub frame: u16,
}

impl ImageKey {
    /// `PICT` `id`.
    #[must_use]
    pub const fn picture(id: i16) -> Self {
        Self {
            kind: ImageKind::Pict,
            id,
            frame: 0,
        }
    }

    /// The interface file's `PICT` `id` ([`ImageKind::InterfacePict`]).
    #[must_use]
    pub const fn interface_picture(id: i16) -> Self {
        Self {
            kind: ImageKind::InterfacePict,
            id,
            frame: 0,
        }
    }

    /// `PICT` `id` with its alpha taken from `PICT` `mask`.
    #[must_use]
    pub const fn masked_picture(id: i16, mask: i16) -> Self {
        Self {
            kind: ImageKind::MaskedPict { mask },
            id,
            frame: 0,
        }
    }

    /// `ppat` `id`.
    #[must_use]
    pub const fn pattern(id: i16) -> Self {
        Self {
            kind: ImageKind::Pattern,
            id,
            frame: 0,
        }
    }

    /// `cicn` `id`.
    #[must_use]
    pub const fn icon(id: i16) -> Self {
        Self {
            kind: ImageKind::Icon,
            id,
            frame: 0,
        }
    }

    /// Frame `frame` of `rlëD` `id`.
    #[must_use]
    pub const fn sprite(id: i16, frame: u16) -> Self {
        Self {
            kind: ImageKind::Rled,
            id,
            frame,
        }
    }

    /// Frame `frame` of the sheet `spïn` `id` resolves to
    /// ([`ImageKind::SpinSheet`]).
    #[must_use]
    pub const fn spin_sheet(id: i16, frame: u16) -> Self {
        Self {
            kind: ImageKind::SpinSheet,
            id,
            frame,
        }
    }

    /// Frame `frame` of the sheet `shïp` `ship`'s `image` resolves to
    /// ([`ImageKind::ShipSheet`]).
    #[must_use]
    pub const fn ship_sheet(ship: i16, image: nova_data::ShipImage, frame: u16) -> Self {
        Self {
            kind: ImageKind::ShipSheet { image },
            id: ship,
            frame,
        }
    }

    /// Frame `frame` of the sheet stock `shïp` `ship`'s `image` resolves
    /// to in the stock art, scaled by `scale` ([`ImageKind::StockShip`]).
    #[must_use]
    pub const fn stock_ship(
        ship: i16,
        image: nova_data::ShipImage,
        scale: nova_data::stock_look::Scale,
        frame: u16,
    ) -> Self {
        Self {
            kind: ImageKind::StockShip { image, scale },
            id: ship,
            frame,
        }
    }

    /// Frame `frame` of `rlëD` `id`'s solid runs alone
    /// ([`ImageKind::RledSolid`]).
    #[must_use]
    pub const fn solid_runs(id: i16, frame: u16) -> Self {
        Self {
            kind: ImageKind::RledSolid,
            id,
            frame,
        }
    }
}

impl ImageKey {
    /// Frame `frame` of `rlëD` `id` as the original draws it with the box
    /// cut by `(top, left)` ([`ImageKind::RledClipped`]).
    #[must_use]
    pub const fn clipped_sprite(id: i16, frame: u16, (top, left): (u16, u16)) -> Self {
        Self {
            kind: ImageKind::RledClipped { top, left },
            id,
            frame,
        }
    }

    /// Frame `frame` of `rlëD` `id`'s solid runs as the original draws
    /// them with the box cut by `(top, left)`
    /// ([`ImageKind::RledClippedSolid`]).
    #[must_use]
    pub const fn clipped_solid_runs(id: i16, frame: u16, (top, left): (u16, u16)) -> Self {
        Self {
            kind: ImageKind::RledClippedSolid { top, left },
            id,
            frame,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_frame_zero_of_a_pict() {
        let key = ImageKey::picture(128);
        assert_eq!(
            key,
            ImageKey {
                kind: ImageKind::Pict,
                id: 128,
                frame: 0
            }
        );
    }

    #[test]
    fn an_interface_picture_is_frame_zero_of_an_interface_pict() {
        let key = ImageKey::interface_picture(134);
        assert_eq!(
            key,
            ImageKey {
                kind: ImageKind::InterfacePict,
                id: 134,
                frame: 0
            }
        );
        assert_ne!(key, ImageKey::picture(134));
    }

    #[test]
    fn a_masked_picture_names_its_mask() {
        let key = ImageKey::masked_picture(7500, 7600);
        assert_eq!(
            key,
            ImageKey {
                kind: ImageKind::MaskedPict { mask: 7600 },
                id: 7500,
                frame: 0
            }
        );
        assert_ne!(key, ImageKey::picture(7500));
        assert_ne!(key, ImageKey::masked_picture(7500, 7601));
    }

    #[test]
    fn a_pattern_is_frame_zero_of_a_ppat() {
        assert_eq!(
            ImageKey::pattern(128),
            ImageKey {
                kind: ImageKind::Pattern,
                id: 128,
                frame: 0
            }
        );
        assert_ne!(ImageKey::pattern(128), ImageKey::picture(128));
    }

    #[test]
    fn an_icon_is_frame_zero_of_a_cicn() {
        assert_eq!(
            ImageKey::icon(18000),
            ImageKey {
                kind: ImageKind::Icon,
                id: 18000,
                frame: 0
            }
        );
        assert_ne!(ImageKey::icon(18000), ImageKey::picture(18000));
    }

    #[test]
    fn a_sprite_keeps_its_id_and_frame() {
        let key = ImageKey::sprite(-200, 35);
        assert_eq!(
            key,
            ImageKey {
                kind: ImageKind::Rled,
                id: -200,
                frame: 35
            }
        );
    }

    #[test]
    fn a_ship_sheet_is_keyed_by_its_ship_and_image() {
        use nova_data::ShipImage;
        let key = ImageKey::ship_sheet(128, ShipImage::Glow, 3);
        assert_eq!(
            key,
            ImageKey {
                kind: ImageKind::ShipSheet {
                    image: ShipImage::Glow
                },
                id: 128,
                frame: 3
            }
        );
        assert_ne!(key, ImageKey::sprite(128, 3));
        assert_ne!(key, ImageKey::spin_sheet(128, 3));
        assert_ne!(key, ImageKey::ship_sheet(128, ShipImage::Lights, 3));
        assert_ne!(key, ImageKey::ship_sheet(129, ShipImage::Glow, 3));
    }

    #[test]
    fn a_stock_ship_is_keyed_by_its_stock_ship_image_and_scale() {
        use nova_data::ShipImage;
        use nova_data::stock_look::Scale;
        let half = Scale::of(1, 2);
        let key = ImageKey::stock_ship(136, ShipImage::Base, half, 3);
        assert_eq!(
            key,
            ImageKey {
                kind: ImageKind::StockShip {
                    image: ShipImage::Base,
                    scale: half
                },
                id: 136,
                frame: 3
            }
        );
        assert_ne!(key, ImageKey::ship_sheet(136, ShipImage::Base, 3));
        assert_ne!(key, ImageKey::sprite(136, 3));
        assert_ne!(
            key,
            ImageKey::stock_ship(136, ShipImage::Base, Scale::ONE, 3)
        );
        assert_ne!(key, ImageKey::stock_ship(136, ShipImage::Glow, half, 3));
    }

    #[test]
    fn a_sprites_solid_runs_are_their_own_image() {
        let key = ImageKey::solid_runs(1930, 13);
        assert_eq!(
            key,
            ImageKey {
                kind: ImageKind::RledSolid,
                id: 1930,
                frame: 13
            }
        );
        assert_ne!(key, ImageKey::sprite(1930, 13));
    }

    #[test]
    fn a_clipped_sprite_names_its_cut() {
        let key = ImageKey::clipped_sprite(1930, 13, (2, 5));
        assert_eq!(
            key,
            ImageKey {
                kind: ImageKind::RledClipped { top: 2, left: 5 },
                id: 1930,
                frame: 13
            }
        );
        assert_ne!(key, ImageKey::sprite(1930, 13));
        assert_ne!(key, ImageKey::clipped_sprite(1930, 13, (5, 2)));
    }

    #[test]
    fn a_clipped_sprites_solid_runs_name_their_cut() {
        let key = ImageKey::clipped_solid_runs(1930, 13, (2, 5));
        assert_eq!(
            key,
            ImageKey {
                kind: ImageKind::RledClippedSolid { top: 2, left: 5 },
                id: 1930,
                frame: 13
            }
        );
        assert_ne!(key, ImageKey::solid_runs(1930, 13));
        assert_ne!(key, ImageKey::clipped_sprite(1930, 13, (2, 5)));
        assert_ne!(key, ImageKey::clipped_solid_runs(1930, 13, (5, 2)));
    }
}
