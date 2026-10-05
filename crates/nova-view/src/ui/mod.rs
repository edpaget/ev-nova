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
//! - [`text_field`]: a one-line [`TextField`] that typed characters fill
//!   and Backspace empties.
//! - [`new_pilot`]: the New Pilot dialog ([`NewPilotDialog`], "Create a
//!   new pilot:"): the new pilot's name.
//! - [`toggle`]: an on/off check box with a label.
//! - [`volume`]: a volume level with arrows that step it.
//! - [`prefs`]: the Preferences dialog ([`PrefsDialog`], "new prefs
//!   dialog"): sound and music on or off, and their volumes.
//! - [`plunder`]: the plunder dialog ([`PlunderDialog`]), taking what is
//!   on board a ship boarded, and the captured-ship assignment dialog
//!   ([`AssignmentDialog`]).
//! - [`comm`]: the comm dialog ([`CommDialog`]), talking to a ship
//!   hailed, and the haggle dialog ([`HaggleDialog`]) over its price.
//! - [`catalog`]: the ports the widgets read: [`DialogResources`] and
//!   [`DescriptionSource`].
//! - [`data`]: the ports' adapters over `nova_data`'s `InterfaceData` and
//!   `GameData`.

pub mod button;
pub mod catalog;
pub mod comm;
pub mod data;
pub mod desc;
pub mod dialog;
pub mod new_pilot;
pub mod plunder;
pub mod prefs;
pub mod scroll_text;
pub mod slice;
pub mod text_field;
pub mod toggle;
pub mod volume;

pub use button::{Button, ButtonImages, ButtonSkin, ButtonStyle, ButtonTracker};
pub use catalog::{DescriptionSource, DialogResources};
pub use comm::{CommDialog, CommPress, HaggleDialog};
pub use desc::DescDialog;
pub use dialog::{
    Dialog, DialogEvent, DialogFrame, DialogTemplate, ItemSpec, ItemTemplate, Placement, Role,
};
pub use new_pilot::{NewPilotDialog, NewPilotOutcome};
pub use plunder::{AssignmentDialog, PlunderDialog, PlunderShown};
pub use prefs::PrefsDialog;
pub use scroll_text::ScrollText;
pub use slice::{Axis, Piece, three_slice};
pub use text_field::TextField;
