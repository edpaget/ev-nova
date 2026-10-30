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
//! - [`text_box`]: a static text box, as `TETextBox` draws one: its last
//!   line clipped to the box. The news, exchange, flight message,
//!   shipyard weapons, player info and shop descriptions draw through it.
//! - [`row_list`]: a [`RowList`] of one-line rows in a box, one of them
//!   selected by a click or the arrow keys.
//! - [`dialog`]: a modal [`Dialog`] laid out from a `DLOG`/`DITL`
//!   [`DialogTemplate`].
//! - [`desc`]: the "Desc Dialog" ([`DescDialog`]), a long description in
//!   a scrolling box with a Done button.
//! - [`note`]: the note alert ([`NoteDialog`], `DLOG` 3000): a line of
//!   text and an OK button.
//! - [`yes_no`]: the Yes/No dialog ([`YesNoDialog`], `DLOG` 3002): a
//!   question, OK for yes and Cancel for no.
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
//! - [`picker`]: a row [`Picker`](picker::Picker) whose arrows step
//!   through a list of entries, such as the frame upscalers.
//! - [`mode_picker`]: a [`ModePicker`](mode_picker::ModePicker) of how
//!   the game runs fullscreen, its arrows stepping through the modes.
//! - [`prefs`]: the Preferences dialog ([`PrefsDialog`], "new prefs
//!   dialog"): sound and music on or off, and their volumes.
//! - [`mission_info`]: the key that asks for the mission info screen
//!   ([`MISSION_INFO_KEY`](mission_info::MISSION_INFO_KEY)).
//! - [`mission_dialogs`]: the mission texts and the mission info screen
//!   under them ([`MissionDialogs`](mission_dialogs::MissionDialogs)),
//!   over the spaceport or over flight.
//! - [`picture_variant`]: the choice between a dialog's plain and
//!   picture templates ([`VariantTemplates`](picture_variant::VariantTemplates)),
//!   which the text, offer, bar and shipyard info dialogs share.
//! - [`player_info`]: the player info screen ([`PlayerInfoScreen`]):
//!   the pilot, the ship, its cargo, extras, ranks and honors.
//! - [`plunder`]: the plunder dialog ([`PlunderDialog`]), taking what is
//!   on board a ship boarded, and the captured-ship assignment dialog
//!   ([`AssignmentDialog`]).
//! - [`comm`]: the comm dialog ([`CommDialog`]), talking to a ship
//!   hailed, and the haggle dialog ([`HaggleDialog`]) over its price.
//! - [`escort_dialog`]: the escort dialog
//!   ([`EscortDialog`](escort_dialog::EscortDialog)) a fleet escort
//!   hailed opens, with its Upgrade, Sell and Release.
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
pub mod escort_dialog;
pub mod mission_dialogs;
pub mod mission_info;
pub mod mode_picker;
pub mod new_pilot;
pub mod note;
pub mod picker;
pub mod picture_variant;
pub mod planet_comm;
pub mod player_info;
pub mod plunder;
pub mod prefs;
pub mod quantity;
pub mod row_list;
pub mod scroll_text;
pub mod slice;
pub mod text_box;
pub mod text_field;
pub mod text_input;
pub(crate) mod text_shrink;
pub mod toggle;
pub mod volume;
pub mod yes_no;

pub use button::{Button, ButtonImages, ButtonSkin, ButtonStyle, ButtonTracker};
pub use catalog::{DescriptionSource, DialogResources, PictureSizes};
pub use comm::{CommDialog, CommPress, HaggleDialog};
pub use desc::DescDialog;
pub use dialog::{
    CENTER_WINDOW_DLOGS, Dialog, DialogEvent, DialogFrame, DialogTemplate, FocusRing, ItemSpec,
    ItemTemplate, Placement, Role, TextBacking,
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
pub use yes_no::YesNoDialog;
