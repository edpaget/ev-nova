//! Image keys: which decoded image a draw command shows.

/// The kinds of image resource the renderer can resolve. Closed, so an
/// unsupported resource type cannot be named.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImageKind {
    /// A `PICT` picture: one frame.
    Pict,
    /// An `rlëD` sprite sheet: one or more equal-sized frames.
    Rled,
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

    /// Frame `frame` of `rlëD` `id`.
    #[must_use]
    pub const fn sprite(id: i16, frame: u16) -> Self {
        Self {
            kind: ImageKind::Rled,
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
}
