//! Immediate-mode widgets and modal dialogs.
//!
//! The widgets are plain values with no retained scene graph and no
//! callbacks: the screen that owns one calls its `input` from its own
//! [`Screen::input`](crate::Screen::input) and its `draw` from its own
//! [`Screen::draw`](crate::Screen::draw). Text is measured through the
//! [`TextMetrics`](crate::text::TextMetrics) port, so layout, hit-testing,
//! wrapping and scrolling need no renderer.
//!
//! - [`slice`](mod@slice): three-slice geometry, shared by buttons and dialog frames.
//! - [`button`]: buttons drawn with Nova's button graphics at any size,
//!   and `TrackControl`-style pointer tracking.
//! - [`scroll_text`]: text wrapped into a box and scrolled a line at a
//!   time.
//! - [`dialog`]: a modal [`Dialog`] laid out from a `DLOG`/`DITL`
//!   [`DialogTemplate`].
//! - [`desc`]: the "Desc Dialog" ([`DescDialog`]), a long description in
//!   a scrolling box with a Done button.
//! - [`toggle`]: an on/off check box with a label.
//! - [`volume`]: a volume level with arrows that step it.
//! - [`prefs`]: the Preferences dialog ([`PrefsDialog`], "new prefs
//!   dialog"): sound and music on or off, and their volumes.
//! - [`catalog`]: the ports the widgets read: [`DialogResources`] and
//!   [`DescriptionSource`].
//! - [`data`]: the ports' adapters over `nova_data`'s `InterfaceData` and
//!   `GameData`.

pub mod button;
pub mod catalog;
pub mod data;
pub mod desc;
pub mod dialog;
pub mod prefs;
pub mod scroll_text;
pub mod slice;
pub mod toggle;
pub mod volume;

pub use button::{Button, ButtonImages, ButtonSkin, ButtonStyle, ButtonTracker};
pub use catalog::{DescriptionSource, DialogResources};
pub use desc::DescDialog;
pub use dialog::{
    Dialog, DialogEvent, DialogFrame, DialogTemplate, ItemSpec, ItemTemplate, Placement, Role,
};
pub use prefs::PrefsDialog;
pub use scroll_text::ScrollText;
pub use slice::{Axis, Piece, three_slice};
