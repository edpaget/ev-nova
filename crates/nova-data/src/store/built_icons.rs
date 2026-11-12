//! Icons a new game builds, kept on the store.
//!
//! A new game's data holds no `cïcn` resources, but the engine draws some
//! art as `cïcn`s by ID, such as the bracket corners round the nav and
//! ship targets. The new game's loader builds those images in code
//! (`crate::new_game::brackets`, filled by `crate::new_game::load`) and
//! keeps them here by the `cïcn` ID each stands for, through
//! [`GameData::with_built_icons`].
//!
//! [`GameData::icon`] answers an ID from the store's own `cïcn` first:
//! whenever any file holds one at that ID, it is that resource decoded
//! (or why it doesn't decode), and a built icon at the same ID is never
//! seen. Only an ID no file holds is answered by a built icon. The stock
//! game's store, and [`GameData::default`], hold no built icons, so every
//! icon there is its `cïcn` or nothing.

use std::collections::BTreeMap;

use super::GameData;
use crate::graphics::{CICN, GraphicsError, Image, decode_cicn};

impl GameData {
    /// The store with `icons` as its built icons, by `cïcn` ID, replacing
    /// any it had (see the module docs).
    #[must_use]
    pub fn with_built_icons(mut self, icons: BTreeMap<i16, Image>) -> Self {
        self.built_icons = icons;
        self
    }

    /// The icon at `id`: the store's own `cïcn` decoded, or why it doesn't
    /// decode, when any file holds one; otherwise the built icon at `id`;
    /// otherwise `None` (see the module docs).
    #[must_use]
    pub fn icon(&self, id: i16) -> Option<Result<Image, GraphicsError>> {
        match self.resource(CICN, id) {
            Some(found) => Some(decode_cicn(found.resource.data())),
            None => self.built_icons.get(&id).cloned().map(Ok),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::graphics::fixture::{Cicn, Ctab};
    use crate::store::fixture::store_of;

    /// A 2 x 1 icon: `rgb` (16-bit components), then a masked-out pixel.
    fn cicn(rgb: [u16; 3]) -> Vec<u8> {
        let ctab = Ctab {
            device: false,
            entries: vec![(0, rgb), (1, [0; 3])],
        };
        Cicn::new(2, 1, 8, &ctab, &[0, 1], &[true, false]).bytes()
    }

    /// A 1 x 1 image of `rgba`.
    fn image(rgba: [u8; 4]) -> Image {
        Image::from_rgba(1, 1, rgba.to_vec()).expect("one pixel")
    }

    fn icons(entries: &[(i16, Image)]) -> BTreeMap<i16, Image> {
        entries.iter().cloned().collect()
    }

    #[test]
    fn a_store_with_built_icons_answers_them_by_id() {
        let red = image([255, 0, 0, 255]);
        let data = GameData::from_decoded(PathBuf::from("/game"), Vec::new())
            .with_built_icons(icons(&[(10000, red.clone())]));
        assert_eq!(data.icon(10000), Some(Ok(red)));
        assert_eq!(data.icon(10001), None);
    }

    #[test]
    fn new_built_icons_replace_the_old() {
        let (red, blue) = (image([255, 0, 0, 255]), image([0, 0, 255, 255]));
        let data = GameData::default()
            .with_built_icons(icons(&[(10000, red)]))
            .with_built_icons(icons(&[(10001, blue.clone())]));
        assert_eq!(data.icon(10000), None, "replaced");
        assert_eq!(data.icon(10001), Some(Ok(blue)));
    }

    #[test]
    fn a_store_s_own_cicn_wins_over_a_built_icon() {
        let bytes = cicn([0xFFFF, 0, 0]);
        let (other, built) = (image([0, 255, 0, 255]), image([0, 0, 255, 255]));
        let data = store_of(&[(CICN, 10000, bytes.clone())])
            .with_built_icons(icons(&[(10000, other), (10001, built.clone())]));
        assert_eq!(data.icon(10000), Some(decode_cicn(&bytes)));
        assert!(matches!(data.icon(10000), Some(Ok(_))), "decodes");
        assert_eq!(data.icon(10001), Some(Ok(built)));
    }

    #[test]
    fn a_stock_store_has_no_built_icons() {
        let bytes = cicn([0, 0xFFFF, 0]);
        let data = store_of(&[(CICN, 10000, bytes.clone())]);
        assert_eq!(data.icon(10000), Some(decode_cicn(&bytes)));
        assert!(matches!(data.icon(10000), Some(Ok(_))), "decodes");
        assert_eq!(data.icon(10001), None);
        assert_eq!(GameData::default().icon(10000), None);
    }

    #[test]
    fn a_cicn_that_does_not_decode_is_its_error() {
        let mut bytes = cicn([0xFFFF, 0, 0]);
        bytes.truncate(10);
        let data = store_of(&[(CICN, 10000, bytes)])
            .with_built_icons(icons(&[(10000, image([0, 0, 255, 255]))]));
        assert!(
            matches!(data.icon(10000), Some(Err(_))),
            "{:?}",
            data.icon(10000)
        );
    }
}
