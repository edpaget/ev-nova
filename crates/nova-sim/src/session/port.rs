//! The landing, the take-off and the spaceport visit under way: the dock
//! request and the land key, the take-off grace and where the ship
//! touched down, the stay, the bar's news, the outfitter visit, the day's
//! hire, outfit and ship rolls, what the ranks' port prices are taken
//! from, and the port's own `dësc` (see the `port_desc` module). Landing and taking off themselves (`land`, `dock`, `take_off`)
//! coordinate the rest and stay on `Session`.

use std::collections::BTreeSet;

use crate::catalog::{OutfitId, ShipId, StellarId};
use crate::geometry::Vec2;
use crate::news::News;
use crate::outfitter::OutfitVisit;
use crate::story::day::Stay;
use crate::wares::DayRolls;

use super::ranks;

/// The landing, the take-off and the spaceport visit under way, in
/// `Session`'s former field order.
// The fields keep the names they had on `Session`.
#[allow(clippy::struct_field_names)]
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Port {
    /// Each ship class's roll for hire today: drawn by the daily step, or
    /// else the first time the bar's list asks it (see
    /// [`RuleKey::DayRolls`](crate::rulebook::RuleKey::DayRolls)). Not saved.
    pub(super) hire_rolls: DayRolls<ShipId>,
    /// The news of the bar visit under way, drawn as the bar opened (see
    /// the `news` module); forgotten as the docking changes, and never
    /// saved.
    pub(super) bar_news: Option<News>,
    /// Each outfit's roll for sale today: drawn by the daily step, or else
    /// the first time the outfitter's list asks it (see
    /// [`RuleKey::DayRolls`](crate::rulebook::RuleKey::DayRolls)). Not saved.
    pub(super) outfit_rolls: DayRolls<OutfitId>,
    /// The outfitter visit under way ([`OutfitVisit`]): the
    /// once-an-opening flags, which limit what is sold as
    /// [`RuleKey::OutfitLimit`](crate::rulebook::RuleKey::OutfitLimit) reads, and the counts owned when it
    /// opened, which a sale refunds against as [`RuleKey::OutfitRefund`](crate::rulebook::RuleKey::OutfitRefund)
    /// reads. None of it is saved.
    pub(super) outfit_visit: OutfitVisit,
    /// The outfit a relisting selected, `oütf` 128, while it may still be
    /// selected: the engine's `*0xded58` after an order that changes the
    /// listing, which Buy and Sell answer for, listed or not (see
    /// [`RuleKey::RelistedSelection`](crate::rulebook::RuleKey::RelistedSelection)). The outfitter carries its row even
    /// when it is not listed ([`Outfitter::unlisted`](crate::outfitter::Outfitter::unlisted)). An order of
    /// another outfit, and each opening, lets it go. It is not saved.
    pub(super) outfit_reselected: Option<OutfitId>,
    /// What this spaceport stay has done that passes days as it is left
    /// (see [`Session::take_off`](super::Session::take_off)). A landing starts it afresh. It is not
    /// saved: the original keeps it in flags `_DoPortDialog` clears on
    /// opening (@0x5f93d-0x5f95a), and loading a save passes no time.
    pub(super) stay: Stay,
    /// Each ship class's roll for sale today: drawn by the daily step, or
    /// else the first time the shipyard's list asks it (see
    /// [`RuleKey::DayRolls`](crate::rulebook::RuleKey::DayRolls)). Not saved.
    pub(super) ship_rolls: DayRolls<ShipId>,
    /// The ship classes whose roll is drawn again the next time the
    /// shipyard opens ([`Session::open_shipyard`](super::Session::open_shipyard)):
    /// each one bought or declined since it last opened. The original
    /// builds the list once each time the shipyard opens
    /// (`_SetupPortAvailableShipTypes`, called only @0x5e68f), so a roll
    /// drawn again shows only then. A landing with no day passing forgets
    /// their rolls, and a day's draw, which draws them afresh, empties it.
    pub(super) ship_redraws: BTreeSet<ShipId>,
    /// The classes the shipyard listed when it last opened, in display
    /// order, while it may still be open: the original's `*0x3b5014`,
    /// which `_SetupPortAvailableShipTypes` (@0xbbe3-0xbeda) writes only
    /// on `_DoShipyardDialog`'s entry (@0x5e68f). The shipyard relists
    /// them, every other value read afresh (see the `shipyard_listing`
    /// module). The next opening replaces it, and a landing, a relocation
    /// and a take-off forget it. It is not saved.
    pub(super) shipyard_listing: Option<Vec<ShipId>>,
    /// The dock request the land key made, if any: the original's
    /// `_dockTarget` and `_dockPermission` (see the `clearance` module);
    /// not saved, as a flight start sets both to −1.
    pub(super) dock: Option<crate::clearance::DockRequest>,
    /// Whether the land key is held, as the view last said: the
    /// original's `_dockTryFlag`, which holds the dock approach back (see
    /// the `clearance` module).
    pub(super) land_key: bool,
    /// The take-off grace's ticks still to run (see the `takeoff_grace`
    /// module); not saved, as the original's pilot file does not keep its
    /// frame counter.
    pub(super) takeoff_grace: u32,
    /// Whether the next take-off keeps the ship where it touched down
    /// (the original's `_dontMovePlayerAfterLanding`); never saved. Set by
    /// a docked `N`, or by an `N` in flight under `keep_flag`'s other
    /// reading (see the `script_effects` module): the original clears it
    /// at every landing (`_PlayerLandOnStellar` @0x631f1-0x631f6).
    pub(super) hold_position: bool,
    /// Where the ship was as it last landed, before it was docked at the
    /// stellar's centre.
    pub(super) touchdown: Vec2,
    /// What the ranks' price multipliers are taken from as the spaceport
    /// opens, `_DoPortDialog`'s inputs (see the `ranks` module). Never
    /// saved, as `_priceFluxMult` lives in memory only.
    pub(super) port_ranks: ranks::PortRanks,
    /// The port's own `dësc`, which `_DoPortDialog` loads as the spaceport
    /// opens (@0x5f96b), before the land checks (see the `port_desc`
    /// module). It is not saved: the load itself lands nothing a save
    /// keeps.
    pub(super) own_desc: OwnDesc,
}

/// Where the port's own `dësc` stands (see the `port_desc` module).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum OwnDesc {
    /// No landing has made it due since the session started or loaded.
    #[default]
    None,
    /// A landing at this stellar has made it due, and it is not loaded
    /// yet.
    Due(StellarId),
    /// Loaded for this stellar: its text taken in.
    Loaded {
        /// The stellar landed on.
        stellar: StellarId,
        /// The text loaded, empty for a missing `dësc`, as
        /// `_LoadDescription` empties the buffer (@0x71a5e).
        text: String,
    },
}

impl Port {
    /// The port as a session starts or loads: nothing rolled, no news, no
    /// outfitter visit, shipyard listing or stay under way, no dock request, the land key
    /// up, no take-off grace, the ship free to move as it takes off, and
    /// `touchdown`, where the ship resumes, as where it touched down.
    pub(super) fn new(touchdown: Vec2) -> Self {
        Self {
            hire_rolls: DayRolls::default(),
            bar_news: None,
            outfit_rolls: DayRolls::default(),
            outfit_visit: OutfitVisit::default(),
            outfit_reselected: None,
            stay: Stay::default(),
            ship_rolls: DayRolls::default(),
            ship_redraws: BTreeSet::new(),
            shipyard_listing: None,
            dock: None,
            land_key: false,
            takeoff_grace: 0,
            hold_position: false,
            touchdown,
            port_ranks: ranks::PortRanks::default(),
            own_desc: OwnDesc::None,
        }
    }

    /// A landing at `stellar` makes its own `dësc` due.
    pub(super) fn due_own_desc(&mut self, stellar: StellarId) {
        self.own_desc = OwnDesc::Due(stellar);
    }

    /// The stellar whose own `dësc` a landing has made due and is not
    /// loaded yet.
    pub(super) fn own_desc_due(&self) -> Option<StellarId> {
        match self.own_desc {
            OwnDesc::Due(stellar) => Some(stellar),
            OwnDesc::None | OwnDesc::Loaded { .. } => None,
        }
    }

    /// `stellar`'s own `dësc` as it was loaded, empty for a missing one:
    /// `None` while it is not loaded for `stellar`.
    pub(super) fn own_desc_text(&self, stellar: StellarId) -> Option<&str> {
        match &self.own_desc {
            OwnDesc::Loaded {
                stellar: loaded,
                text,
            } if *loaded == stellar => Some(text),
            _ => None,
        }
    }

    /// Notes `stellar`'s own `dësc` loaded as `text`.
    pub(super) fn own_desc_loaded(&mut self, stellar: StellarId, text: String) {
        self.own_desc = OwnDesc::Loaded { stellar, text };
    }
}
