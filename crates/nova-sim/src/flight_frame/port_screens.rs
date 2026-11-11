//! The spaceport's merged screen answers: what the spaceport reads of its
//! shops as it opens and at each refresh ([`PortShops`],
//! [`ScreenAsk::Shops`](super::ScreenAsk::Shops)), the services among
//! them with nothing to list ([`EmptyServices`]), and the bar's news as
//! News is pressed ([`BarNews`], [`ScreenAsk::News`](super::ScreenAsk::News)).
//! Each is plain data the session builds from its own reads, borrowing
//! nothing of the session's.

use crate::{Market, News, Outfitter, Shipyard};

/// What the spaceport reads of its shops, asked with no command between
/// the reads ([`ScreenAsk::Shops`](super::ScreenAsk::Shops)): as it opens
/// (`AppScreen::show_spaceport`), and at each refresh after an order, a
/// hire, an accept or a pilot edit, which ignores the room to hire. The
/// session reads them in today's order: the exchange, the outfitter, then
/// the shipyard. The default is a flight with nothing open: no shops and
/// no room to hire.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PortShops {
    /// The exchange of the stellar landed on
    /// ([`Session::market`](crate::Session::market)).
    pub market: Option<Market>,
    /// The outfitter of the stellar landed on, the day's rolls not drawn
    /// yet drawn on the flight's chance
    /// ([`Session::outfitter`](crate::Session::outfitter)).
    pub outfitter: Option<Outfitter>,
    /// The shipyard of the stellar landed on, built afresh: the classes
    /// bought or declined since its last build redrawn first, and the
    /// day's rolls not drawn yet drawn on the flight's chance
    /// ([`Session::shipyard`](crate::Session::shipyard)).
    pub shipyard: Option<Shipyard>,
    /// Whether the bar's hire screen may hire escorts now
    /// ([`Session::can_hire_escorts`](crate::Session::can_hire_escorts),
    /// `_CanHireEscorts`), which the bar is told as it opens.
    pub can_hire_escorts: bool,
    /// Which services have nothing to list, for the spaceport and its bar
    /// to grey ([`EmptyServices`]), read after the shipyard on the same
    /// chance; [`EmptyServices::default`] by the engine reading of
    /// [`RuleKey::EmptyServices`](crate::rulebook::RuleKey::EmptyServices),
    /// drawing nothing. Unlike the room, it is read again at each refresh.
    pub empty: EmptyServices,
}

/// Which of the port's services have nothing to list, for the view to
/// grey ([`RuleKey::EmptyServices`](crate::rulebook::RuleKey::EmptyServices)).
/// By the engine reading (the default) neither is ever set: the Mission
/// BBS of every inhabited stellar stays enabled (`_DrawPlanetButtons`
/// @0x2968d) and answers #353 with nothing available, and Hire Escort is
/// greyed only by the room to hire (`_DrawBarButtons` @0x26e1d) and
/// answers #224 with nothing for hire. By the other reading each is set
/// while its service has nothing to list. The default, as in flight,
/// names nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EmptyServices {
    /// The Mission BBS: landed, and no offer of the visit at the mission
    /// computer is available now. A free slot isn't needed: a full set of
    /// slots with offers to list still opens and says so.
    pub mission_bbs: bool,
    /// The bar's Hire Escort: landed at a bar that lists no ship for hire
    /// today, whatever the room or the cash.
    pub hire_escort: bool,
}

/// The news of the bar visit under way as it was drawn, and the picture
/// it is drawn over ([`ScreenAsk::News`](super::ScreenAsk::News)): a copy
/// of the visit's kept news, its tags unfilled
/// ([`Session::news`](crate::Session::news)), and the `PICT` of the
/// stellar docked at, read as News is pressed
/// ([`Session::news_picture`](crate::Session::news_picture)), so a change
/// of owner since the bar opened shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BarNews {
    /// The commercial line and the story, as drawn.
    pub news: News,
    /// The `PICT` it is drawn over.
    pub picture: i16,
}
