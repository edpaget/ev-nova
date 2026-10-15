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
//! - [`row_list`]: a [`RowList`] of one-line rows in a box, one of them
//!   selected by a click or the arrow keys.
//! - [`dialog`]: a modal [`Dialog`] laid out from a `DLOG`/`DITL`
//!   [`DialogTemplate`].
//! - [`desc`]: the "Desc Dialog" ([`DescDialog`]), a long description in
//!   a scrolling box with a Done button.
//! - [`note`]: the note alert ([`NoteDialog`], `DLOG` 3000): a line of
//!   text and an OK button.
//! - [`text_field`]: a one-line [`TextField`] that typed characters fill
//!   and Backspace empties.
//! - [`edit_dialog`]: the edit-field modal shell ([`EditDialog`]) the
//!   "Text Input" and quantity dialogs share, each with its own
//!   [`Confirm`] policy.
//! - [`text_input`]: the "Text Input" dialog ([`TextInputDialog`]): a
//!   prompt and a line of text to type, such as the new ship's name.
//! - [`quantity`]: the quantity dialog ([`QuantityDialog`]): how many
//!   to buy or sell, up to a maximum.
//! - [`new_pilot`]: the New Pilot dialog ([`NewPilotDialog`], "Create a
//!   new pilot:"): the new pilot's name.
//! - [`toggle`]: an on/off check box with a label.
//! - [`volume`]: a volume level with arrows that step it.
//! - [`prefs`]: the Preferences dialog ([`PrefsDialog`], "new prefs
//!   dialog"): sound and music on or off, and their volumes.
//! - [`player_info`]: the player info screen ([`PlayerInfoScreen`]):
//!   the pilot, the ship, its cargo, extras, ranks and honors.
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
pub mod edit_dialog;
pub mod new_pilot;
pub mod note;
pub mod player_info;
pub mod plunder;
pub mod prefs;
pub mod quantity;
pub mod row_list;
pub mod scroll_text;
pub mod slice;
pub mod text_field;
pub mod text_input;
pub mod toggle;
pub mod volume;

pub use button::{Button, ButtonImages, ButtonSkin, ButtonStyle, ButtonTracker};
pub use catalog::{DescriptionSource, DialogResources};
pub use comm::{CommDialog, CommPress, HaggleDialog};
pub use desc::DescDialog;
pub use dialog::{
    Dialog, DialogEvent, DialogFrame, DialogTemplate, ItemSpec, ItemTemplate, Placement, Role,
};
pub use edit_dialog::{Confirm, EditDialog, EditItems};
pub use new_pilot::{NewPilotDialog, NewPilotOutcome};
pub use note::NoteDialog;
pub use player_info::{PlayerInfoButton, PlayerInfoScreen, Tab};
pub use plunder::{AssignmentDialog, PlunderDialog, PlunderShown};
pub use prefs::PrefsDialog;
pub use quantity::QuantityDialog;
pub use row_list::RowList;
pub use scroll_text::ScrollText;
pub use slice::{Axis, Piece, three_slice};
pub use text_field::TextField;
pub use text_input::{TextInputDialog, TextInputOutcome};
