//! The system catalog port: one star system's contents, in the view's own
//! terms.

use std::num::NonZeroU16;
use std::rc::Rc;

pub use nova_data::{StellarId, SystemId};

/// A stellar's resolved sprite sheet: its `rlëD` ID, how many frames it
/// has and each frame's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StellarSheet {
    /// The `rlëD`'s ID.
    pub image_id: i16,
    /// How many frames it has; never zero.
    pub frames: NonZeroU16,
    /// Each frame's width in pixels.
    pub frame_width: u32,
    /// Each frame's height in pixels.
    pub frame_height: u32,
}

/// A stellar's animation fields, raw from its `spöb`. The view decides
/// what they mean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimationData {
    /// `AnimDelay`: how many 1/30 s ticks each frame shows for.
    pub delay: i16,
    /// `Frame0Bias`: how many times longer frame 0 shows for.
    pub frame0_bias: i16,
    /// `Flags2` 0x0080: animate only when destroyed.
    pub only_when_destroyed: bool,
}

/// One stellar object, placed in its system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StellarContents {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// Its display name.
    pub name: String,
    /// Its X position in the system; x grows right.
    pub x: i16,
    /// Its Y position in the system; y grows down.
    pub y: i16,
    /// Its sprite sheet, or why it cannot be shown.
    pub sprite: Result<StellarSheet, String>,
    /// Its animation fields.
    pub animation: AnimationData,
}

/// One star system's contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemContents {
    /// The `sÿst`'s ID.
    pub id: SystemId,
    /// Its display name.
    pub name: String,
    /// Every stellar that could be placed, in the system's navigation
    /// order.
    pub stellars: Vec<StellarContents>,
    /// What could not be read, each a message ready to display.
    pub problems: Vec<String>,
}

/// The star systems a system view can show.
pub trait SystemCatalog {
    /// Everything in system `id`.
    fn system(&self, id: SystemId) -> SystemContents;
}

/// A borrowed catalog is a catalog.
impl<T: SystemCatalog + ?Sized> SystemCatalog for &T {
    fn system(&self, id: SystemId) -> SystemContents {
        (**self).system(id)
    }
}

/// A shared catalog is a catalog, so a view and the renderer can read the
/// same game data.
impl<T: SystemCatalog + ?Sized> SystemCatalog for Rc<T> {
    fn system(&self, id: SystemId) -> SystemContents {
        (**self).system(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every system is empty and named after its ID.
    struct Empty;

    impl SystemCatalog for Empty {
        fn system(&self, id: SystemId) -> SystemContents {
            SystemContents {
                id,
                name: format!("System {}", id.0),
                stellars: Vec::new(),
                problems: Vec::new(),
            }
        }
    }

    fn name(catalog: impl SystemCatalog, id: i16) -> String {
        catalog.system(SystemId(id)).name
    }

    #[test]
    fn borrowed_and_shared_catalogs_are_catalogs() {
        assert_eq!(name(&Empty, 130), "System 130");
        assert_eq!(name(Rc::new(Empty), 131), "System 131");
    }
}
