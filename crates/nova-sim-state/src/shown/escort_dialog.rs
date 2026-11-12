//! The escort dialog's view ([`EscortDialogView`], [`UpgradeLine`],
//! [`SaleLine`]), its buttons ([`EscortButton`]) and their labels, from
//! `nova_sim_port::escort_dialog`, which re-exports them and keeps the
//! view's constructor from the fleet's escort, the marks and their
//! settling at a shipyard.

use nova_sim_catalog::catalog::ShipId;

use super::fleet::EscortMark;
use super::hail::ShownWage;

/// `STR#` 2002 #167: a captured escort's title.
pub const CAPTURED_ESCORT: &str = "Captured Escort";

/// `STR#` 2002 #292.
pub const WILL_BE_UPGRADED: &str = "Will be upgraded at next shipyard";

/// `STR#` 2002 #293.
pub const UPGRADE_COST: &str = "Upgrade Cost:";

/// `STR#` 2002 #294.
pub const CANNOT_BE_UPGRADED: &str = "This ship class cannot be upgraded.";

/// `STR#` 2002 #295.
pub const WILL_BE_SOLD: &str = "Will be sold off at next shipyard";

/// `STR#` 2002 #296.
pub const SELL_PRICE: &str = "Sell Price:";

/// `_creditCString`, `STR#` 2002 #32: the report's unit for 1 or less
/// (@0x4068f).
pub const CREDIT: &str = "credit";

/// `_creditsCString`, `STR#` 2002 #33: the report's unit above 1.
pub const CREDITS: &str = "credits";

/// `STR#` 150 #32: the Release button.
pub const RELEASE: &str = "Release";

/// `STR#` 150 #52.
pub const UPGRADE_ESCORT: &str = "Upgrade Escort";

/// `STR#` 150 #53: the Upgrade button while the upgrade mark stands.
pub const CANCEL_UPGRADE: &str = "Cancel Upgrade";

/// `STR#` 150 #54.
pub const SELL_ESCORT: &str = "Sell Escort";

/// `STR#` 150 #55: the Sell button while the sell mark stands.
pub const CANCEL_SALE: &str = "Cancel Sale";

/// A button of the escort dialog, its item (`_EscortFilter` @0x950e9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscortButton {
    /// Item 1, "Close Channel": the dialog closes.
    Close,
    /// Item 2, "Release": the escort is released at once, and the dialog
    /// closes.
    Release,
    /// Item 3, "Upgrade Escort" or "Cancel Upgrade".
    Upgrade,
    /// Item 4, "Sell Escort" or "Cancel Sale".
    Sell,
}

/// The escort dialog's upgrade line (@0x94d0d-0x94e0e).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpgradeLine {
    /// "Will be upgraded at next shipyard", in light grey.
    Marked,
    /// "Upgrade Cost:" in medium grey, then the cost in white.
    Cost(ShownWage),
    /// "This ship class cannot be upgraded.", in light grey.
    Cannot,
}

/// The escort dialog's sale line (@0x94e13-0x94f15).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaleLine {
    /// "Will be sold off at next shipyard", in light grey.
    Marked,
    /// "Sell Price:" in medium grey, then the price in white.
    Price(ShownWage),
}

/// What the escort dialog shows of an escort (see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EscortDialogView {
    /// Its ship class, whose picture the dialog shows (class +0xa0a).
    pub ship: ShipId,
    /// Whether it was hired: the title's "Hired Escort", else "Captured
    /// Escort".
    pub hired: bool,
    /// Its class's `CommName`, the title's second line.
    pub comm_name: String,
    /// Its class's `Subtitle`, the title's third line; none when empty.
    pub subtitle: Option<String>,
    /// The upgrade line.
    pub upgrade: UpgradeLine,
    /// The sale line; none for an escort that can't be sold.
    pub sale: Option<SaleLine>,
    /// A hired escort's pay line; none for any other.
    pub pay: Option<ShownWage>,
    /// Whether the Upgrade button is enabled.
    pub can_upgrade: bool,
    /// Whether the Sell button is enabled.
    pub can_sell: bool,
    /// Its mark, which names the buttons "Cancel Upgrade" or "Cancel
    /// Sale".
    pub mark: Option<EscortMark>,
}
