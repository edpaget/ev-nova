//! The missions' runtime state and the story runtime: the offers of the
//! visit and their rolls, the venue they were entered at, the offers
//! suppressed and those from ships, the offer dialog's state, the avail
//! cache, the slots due a goal check or a swap, the slot whose script
//! runs and those ending, the rank tags' governments and the rank last
//! activated, the movie last loaded, what the set expressions queued, and
//! the story runtime's raised events, subscriptions and watched ships.
//! The coordinators that read other state too stay on `Session`:
//! `run_mission_script`, the offer rolls (`roll_offers`,
//! `reoffer_missions`), `accept_mission`, `drop_offered`,
//! `in_offer_dialog`, `settle_script`, and the text parsing that hands
//! `Session` to `TextSource::parse`.

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{DescMovie, MissionId, RankId, StellarId};
use crate::mission::Offers;
use crate::text_tags::RankTagGovts;
use crate::traffic::npc::NpcId;

use super::{person_offers, script_effects, story};

/// The missions' runtime state and the story runtime, in `Session`'s
/// former field order.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct MissionRun {
    /// What the set expressions left for later: for
    /// [`Session::settle_script`](super::Session::settle_script), and the next flight tick; never saved.
    pub(super) queued: script_effects::Queued,
    /// The story runtime: the events raised and the ships watched (see the
    /// [`story`](crate::story) module); never saved.
    pub(super) story: story::StoryRuntime,
    /// The rank last activated, which `<RRK>` reads, until `L` or a sweep
    /// clears it. Never saved: `None` on every [`Session::fly`](super::Session::fly), as the
    /// original's `ResetPlayer` (@0x1db44) and `ResetPlayerRecord`
    /// (@0x1dbf1) reset it (see the `ranks` module).
    pub(super) recent_rank: Option<RankId>,
    /// The offers of the visit under way and the stellar they were rolled
    /// at (see the `missions` module); never saved, as the original saves
    /// none of it.
    pub(super) offers: Offers,
    /// Whether a mission succeeded on the landing under way, so the
    /// offers are re-offered after its last slot (see the `missions`
    /// module); never saved.
    pub(super) reoffer_due: bool,
    /// The slots of the missions accepted in flight, whose ship goals are
    /// checked once the settling that accepted them is done (see the
    /// `mission_ships` module); never saved.
    pub(super) goal_checks_due: Vec<u8>,
    /// The person's NPC an accept on its hail swaps for the ship of the
    /// mission in the slot, once the settling that accepted it begins
    /// (see the `mission_ships` module); never saved.
    pub(super) swap_due: Option<(NpcId, u8)>,
    /// The stellar a move ([`Session::relocate`](super::Session::relocate)) docked at, whose
    /// offers the next settling rolls if the ship is still landed there
    /// (see the `edit` module); never saved.
    pub(super) offer_roll_due: Option<StellarId>,
    /// The slot whose mission's script is running, if any: what a `Q`
    /// fills its mission tags from (see the `missions` module); never
    /// saved.
    pub(super) script_slot: Option<u8>,
    /// The mission slots whose `OnAbort` is running, so that none ends
    /// again inside itself (see the `missions` module); never saved.
    pub(super) ending: BTreeSet<u8>,
    /// The governments the last `<PRKnnn>` and `<SRKnnn>` taken in named
    /// (`_descRankGovt`; see the `missions` module's text sources), as
    /// the crossed rank tags read them; never saved, as the original
    /// saves none of it.
    pub(super) rank_govts: RankTagGovts,
    /// The movie the last `dësc` loaded names: the original's `MovieFile`
    /// and `Flags` globals (*0x3b52f4, *0x3b5644), which each
    /// `_LoadDescription` overwrites (@0x71895-0x718de) and a missing
    /// `dësc` empties (@0x71a5e-0x71a77), and which every movie check
    /// reads as they stand ([`RuleKey::DescMovieGlobals`](crate::rulebook::RuleKey::DescMovieGlobals)). Written with
    /// the governments above by [`Session::load_description`](super::Session::load_description), so the two
    /// can't drift; none at a new session, and never saved.
    pub(super) loaded_movie: Option<DescMovie>,
    /// The ship `<OSN>` names by the engine (`_offeringShipIndex`; see
    /// the `person_offers` module); never saved, and none at a new pilot
    /// or a load (`_InitMissions` @0x97bbf).
    pub(super) offering_ship: Option<person_offers::OfferingShip>,
    /// Each mission's `AvailBits` verdict as the original last cached it
    /// (`mïsn`+0x16; see [`Session::propagate_avail_bits`](super::Session::propagate_avail_bits)); never saved,
    /// as a load refreshes it.
    pub(super) avail_cache: BTreeMap<MissionId, bool>,
    /// The outside offers whose accept was refused, passed over until
    /// another venue is entered (see the `mission_screens` module); never
    /// saved, as the original saves none of it.
    pub(super) suppressed: BTreeSet<MissionId>,
    /// The venue offers were last entered at, an `AvailLoc`; never saved.
    pub(super) offer_venue: Option<i16>,
    /// The offers from a ship, by mission, each resolved as a person
    /// linked to it spawned (see the `person_offers` module); never saved,
    /// as the original saves none of it.
    pub(super) ship_offers: person_offers::ShipOffers,
    /// The offer from a ship the offer dialog shows, until the player
    /// answers it (see the `person_offers` module); never saved.
    pub(super) offering: Option<person_offers::Offering>,
    /// Whether an offer dialog is open, as the original's `_missionDialog`
    /// is set: while the player's accept or refusal runs, so an `S` in
    /// that `OnAccept` or `OnRefuse` sees it (see the `missions` module);
    /// never saved.
    pub(super) offer_dialog_open: bool,
}

impl MissionRun {
    /// The missions' runtime as a session starts or loads: nothing queued,
    /// raised or watched, no rank activated, no offers rolled, entered,
    /// suppressed or open, no slot due or running a script, no rank tag's
    /// government, no movie loaded and nothing cached.
    pub(super) fn new() -> Self {
        Self {
            queued: script_effects::Queued::default(),
            story: story::StoryRuntime::default(),
            recent_rank: None,
            offers: Offers::default(),
            reoffer_due: false,
            goal_checks_due: Vec::new(),
            swap_due: None,
            offer_roll_due: None,
            script_slot: None,
            ending: BTreeSet::new(),
            rank_govts: RankTagGovts::NONE,
            loaded_movie: None,
            offering_ship: None,
            avail_cache: BTreeMap::new(),
            suppressed: BTreeSet::new(),
            offer_venue: None,
            ship_offers: person_offers::ShipOffers::new(),
            offering: None,
            offer_dialog_open: false,
        }
    }
}
