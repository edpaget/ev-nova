//! The spaceport's layout decisions: which item of the interface file's
//! "Spaceport" dialog (`DLOG`/`DITL` 1000) shows what, the button each
//! service gets and its label, and which `PICT` is the landscape.
//!
//! The stock dialog is 618 x 517, centred, and drawn over `PICT` 8500,
//! whose painted frames show the items' places: the landscape (item 5)
//! across the top, the name bar (item 3) and description box (item 6)
//! under it, and a column of four buttons down each side (items 11, 10, 7
//! and 13 on the left, 9, 8, 4 and 12 on the right, top to bottom).

use nova_sim::Service;

use crate::font::Font;
use crate::image::ImageKey;

/// The spaceport dialog's `DLOG` (and `DITL`) ID.
pub const SPACEPORT_DIALOG: i16 = 1000;

/// The picture the dialog is drawn over: "Spaceport", in Nova Graphics.
pub const BACKGROUND: ImageKey = ImageKey::picture(8500);

/// The landscape's item.
pub const LANDSCAPE_ITEM: usize = 5;

/// The stellar's name's item.
pub const NAME_ITEM: usize = 3;

/// The description's item.
pub const TEXT_ITEM: usize = 6;

/// The Leave button's item: bottom right, as in the original.
pub const LEAVE_ITEM: usize = 12;

/// The Recharge button's item, shown only where fuel is sold: right
/// column, third down, as in the original.
pub const RECHARGE_ITEM: usize = 4;

/// The button each service gets. The data does not record this; it is
/// chosen to match the original's screen, and kept in this one table.
pub const SERVICE_ITEMS: [(Service, usize); 5] = [
    (Service::MissionBbs, 11),
    (Service::Bar, 10),
    (Service::TradeCenter, 7),
    (Service::Shipyard, 9),
    (Service::Outfitter, 8),
];

/// The name's font.
pub const NAME_FONT: Font = Font::Charcoal;
/// The name's size.
pub const NAME_SIZE: f32 = 12.0;

/// `STR#` 150 #1.
pub const LEAVE_LABEL: &str = "Leave";
/// `STR#` 150 #5.
pub const DONE_LABEL: &str = "Done";
/// `STR#` 150 #6.
pub const RECHARGE_LABEL: &str = "Recharge";
/// `STR#` 150 #7.
pub const TRADE_CENTER_LABEL: &str = "Trade Center";
/// `STR#` 150 #8.
pub const OUTFITTER_LABEL: &str = "Outfitter";
/// `STR#` 150 #9.
pub const SHIPYARD_LABEL: &str = "Shipyard";
/// `STR#` 150 #10.
pub const BAR_LABEL: &str = "Bar";
/// `STR#` 150 #16.
pub const MISSION_BBS_LABEL: &str = "Mission BBS";

/// The first custom landscape `PICT`: a `CustPicID` below it names none.
const FIRST_CUSTOM_PICTURE: i16 = 128;

/// The standard landscapes' first `PICT`, for `Type` 0.
const STANDARD_LANDSCAPES: i16 = 10_000;

/// The landscape `PICT` of a stellar with this `CustPicID` and `Type`: its
/// custom picture when it names one (128 and up), otherwise the standard
/// landscape for its type, 10000 + `Type`.
#[must_use]
pub fn landscape_id(cust_pic_id: i16, graphic_type: i16) -> i16 {
    if cust_pic_id >= FIRST_CUSTOM_PICTURE {
        cust_pic_id
    } else {
        STANDARD_LANDSCAPES.saturating_add(graphic_type)
    }
}

/// `service`'s button item.
#[must_use]
pub fn service_item(service: Service) -> usize {
    SERVICE_ITEMS
        .iter()
        .find(|(each, _)| *each == service)
        .map(|(_, item)| *item)
        .expect("every service has an item")
}

/// The service whose button is item `item`, if any.
#[must_use]
pub fn item_service(item: usize) -> Option<Service> {
    SERVICE_ITEMS
        .iter()
        .find(|(_, each)| *each == item)
        .map(|(service, _)| *service)
}

/// `service`'s button label.
#[must_use]
pub fn label(service: Service) -> &'static str {
    match service {
        Service::TradeCenter => TRADE_CENTER_LABEL,
        Service::Outfitter => OUTFITTER_LABEL,
        Service::Shipyard => SHIPYARD_LABEL,
        Service::Bar => BAR_LABEL,
        Service::MissionBbs => MISSION_BBS_LABEL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_named_values() {
        assert_eq!(SPACEPORT_DIALOG, 1000);
        assert_eq!(BACKGROUND, ImageKey::picture(8500));
        assert_eq!(
            [
                LANDSCAPE_ITEM,
                NAME_ITEM,
                TEXT_ITEM,
                LEAVE_ITEM,
                RECHARGE_ITEM
            ],
            [5, 3, 6, 12, 4]
        );
        assert_eq!((NAME_FONT, NAME_SIZE), (Font::Charcoal, 12.0));
    }

    #[test]
    fn a_custom_picture_is_the_landscape() {
        assert_eq!(landscape_id(128, 5), 128);
        assert_eq!(landscape_id(10_000, 0), 10_000);
        assert_eq!(landscape_id(9000, 3), 9000);
    }

    #[test]
    fn otherwise_the_standard_landscape_for_the_type() {
        assert_eq!(landscape_id(127, 5), 10_005);
        assert_eq!(landscape_id(-1, 0), 10_000);
        assert_eq!(landscape_id(0, 255), 10_255);
    }

    #[test]
    fn an_impossible_type_does_not_overflow() {
        assert_eq!(landscape_id(-1, i16::MAX), i16::MAX);
    }

    #[test]
    fn each_service_has_its_own_button_and_the_original_label() {
        use Service::{Bar, MissionBbs, Outfitter, Shipyard, TradeCenter};
        assert_eq!(
            SERVICE_ITEMS,
            [
                (MissionBbs, 11),
                (Bar, 10),
                (TradeCenter, 7),
                (Shipyard, 9),
                (Outfitter, 8),
            ]
        );
        for (service, item) in SERVICE_ITEMS {
            assert_eq!(service_item(service), item);
            assert_eq!(item_service(item), Some(service));
        }
        for item in [0, 1, 3, 4, 5, 6, 12, 13, 99] {
            assert_eq!(item_service(item), None, "{item}");
        }
        assert_eq!(
            [TradeCenter, Outfitter, Shipyard, Bar, MissionBbs].map(label),
            [
                "Trade Center",
                "Outfitter",
                "Shipyard",
                "Bar",
                "Mission BBS"
            ]
        );
        assert_eq!((LEAVE_LABEL, DONE_LABEL), ("Leave", "Done"));
        assert_eq!(RECHARGE_LABEL, "Recharge");
    }
}
