//! What is in the system around the player: the traffic, the fight and
//! its strikes, the cargo boxes jettisons left, the asteroid field, the
//! stellars' damage and the sprites shown, the message line, the
//! escorts' and encounters' state ([`Escorts`]), and the escorts' voice
//! channel (see the `voices` module). The coordinators that
//! read other state too (`step_traffic`, `tick_combat`, `ship_loop`,
//! `populate`, `reform`, `world` and `settle_line_queue`) stay on
//! `Session`.

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{GovtId, StellarId};
use crate::combat::{Combat, ShipRef, Strike};
use crate::comm_line::CommLine;
use crate::gate_sprite::GateSprite;
use crate::story::rebuild::Rebuilt;
use crate::story::sprite_pool::StellarSpritePool;
use crate::traffic::Traffic;
use crate::traffic::npc::NpcId;

use super::{Aboard, hail, persons, reinforcements};

/// What is in the system around the player, in `Session`'s former field
/// order, then its part.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Space {
    /// The sprite of each hypergate the flight has shown, as it opens and
    /// closes, kept while it is not shown (see the `gate_sprites`
    /// module).
    pub(super) gate_sprites: BTreeMap<StellarId, GateSprite>,
    /// The message line's countdown, `_commCount`: set by every line
    /// shown, counted down each frame (see the `comm_calls` module).
    pub(super) comm_line: CommLine,
    /// Each system's reinforcement timer and the inhibitors (see the
    /// `reinforcements` module); never saved, as the original saves
    /// neither.
    pub(super) reinforcements: reinforcements::Reinforcements,
    /// Whether the reinforcement clock's tick of the last traffic tick is
    /// held behind the frame's work on the line, for `settle_line_queue`
    /// to run (see [`Session::tick_traffic`](super::Session::tick_traffic)).
    pub(super) reinforcements_held: bool,
    /// The governments an IFF scrambler has marked (see the
    /// `iff_scramblers` module); never saved, as the original saves none.
    pub(super) iff_marked: BTreeSet<GovtId>,
    /// The NPCs in the system.
    pub(super) traffic: Traffic,
    /// Whether the system is to be populated on the next traffic tick in
    /// flight: from the session's start, and after a relocation that
    /// resets the traffic while landed. A take-off populates at once.
    pub(super) traffic_due: bool,
    /// The heading each ship in the fight was drawn at after the last
    /// [`Session::tick_combat`](super::Session::tick_combat): its heading as that tick ended, which the
    /// view draws before the next. Each ship's shots leave from the exit
    /// points of that frame (see [`Fighter::drawn_heading`](crate::combat::Fighter::drawn_heading)); a ship not
    /// in it, new to the fight, fires from its own heading's. Dropped while
    /// the ship is landed or jumping, and never saved.
    pub(super) drawn: BTreeMap<ShipRef, f32>,
    /// The shots and beams in flight, and what the fight reports.
    pub(super) combat: Combat,
    /// The cargo boxes jettisons left in space (see the `cargo_boxes`
    /// module); never saved, as the original saves none.
    pub(super) boxes: crate::cargo_box::CargoBoxes,
    /// The asteroid field about the player (see the `asteroids` module);
    /// never saved, as the original saves none.
    pub(super) asteroids: crate::asteroid::Asteroids,
    /// The pass frame a kill of the asteroid field holds through, as
    /// `_killAsteroids` holds to the frame's end (see the `asteroids`
    /// module); never saved.
    pub(super) asteroid_kill: Option<i16>,
    /// Whether a session started or loaded in flight is still to place
    /// its asteroid field, as the other reading of
    /// [`RuleKey::AsteroidEntryField`](crate::rulebook::RuleKey::AsteroidEntryField) places it (see the `asteroids`
    /// module); never saved.
    pub(super) asteroid_field_due: bool,
    /// What the view shows of space about the player's ship (see the
    /// `cloaks` module).
    pub(super) play_area: crate::cloak::Area,
    /// The fight's strikes since the last traffic tick, for the NPCs to
    /// answer.
    pub(super) strikes: Vec<Strike>,
    /// The original's frame counter, which times the tribbles and
    /// perishables step (see [`market`](crate::market)). The original's value when a
    /// flight starts depends on how long the program has run, so 0 here
    /// is a choice; a take-off or a gate's exit sets it anyway. Not
    /// saved, as the
    /// original's pilot file does not keep it.
    pub(super) frame: i16,
    /// The frame counter's value the last advance pass ran on, which the
    /// second pass runs on too (see the `double_time` module): the counter
    /// `_DoPlayGameWork` sees, as `_PlayGame` adds 1 after it (@0x45f36),
    /// which the fight's interfered shots circle by ([`Field::frame`](crate::combat::Field::frame)).
    pub(super) pass_frame: i16,
    /// Each ship slot's draw layer and when it entered it (see
    /// [`Session::draw_order`](super::Session::draw_order)), and the frames it last cloned (see
    /// [`Session::framed_by`](super::Session::framed_by)): kept from the session's start, across
    /// every system entry and every ship leaving; never saved.
    pub(super) slot_sprites: crate::ship_layer::SlotSprites,
    /// The ship each slot last displayed and showed (see
    /// [`Space::is_displayed`]); never carried into another session, as
    /// every entry into flight displays every slot again.
    pub(super) slot_shown: BTreeMap<crate::ship_layer::Slot, ShipRef>,
    /// The ship each slot last displayed, shown or hidden: the one its
    /// laid-out place is of (see [`Session::hover_place`](super::Session::hover_place)); never carried
    /// into another session, as `slot_shown`.
    pub(super) slot_placed: BTreeMap<crate::ship_layer::Slot, ShipRef>,
    /// The current strength of each whole stellar shots have damaged, by
    /// ID; one not here has its full `Strength` (see the `stellars`
    /// module). Never saved, as the original saves none.
    pub(super) strength: BTreeMap<StellarId, i64>,
    /// The stellars whose `OnDestroy` is running, so that none runs again
    /// inside itself (see the `stellars` module); never saved.
    pub(super) destroying: BTreeSet<StellarId>,
    /// The stellars shown that have been dead since the last rebuild point
    /// (see the `stellars` module); never saved, as a load is itself a
    /// rebuild point.
    pub(super) rebuilt: Rebuilt,
    /// The stellar sprite pool, each stellar's last slot and each slot's
    /// last place (see the `stale_sprites` module); never saved, as it
    /// lives as long as the original's process.
    pub(super) stellar_sprites: StellarSpritePool,
    /// The escorts' and encounters' state.
    pub(super) escorts: Escorts,
    /// The one voice channel the escorts speak through: the message
    /// waiting and the clip playing (see the `voices` module); never
    /// saved, as the original saves neither.
    pub(super) voice: crate::voice::VoiceChannel,
}

/// The escorts' and encounters' state: the fleet and its class orders,
/// the boarding and the hail open, and the persons' quote clock.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Escorts {
    /// The boarding under way, if any.
    pub(super) aboard: Option<Aboard>,
    /// The hail under way, if any.
    pub(super) talk: Option<hail::Talk>,
    /// Each escort of the fleet's NPC in the system, lined up with the
    /// pilot's escorts while in flight; none for one not placed.
    pub(super) fleet: Vec<Option<NpcId>>,
    /// Whether the person escorts are restocked full to their fitted
    /// ships as they are placed next: after a take-off, and when a pilot
    /// is flown.
    pub(super) restock_persons: bool,
    /// The last order the player gave each escort class, in
    /// [`EscortClass::ALL`](crate::escort::EscortClass::ALL) order: the
    /// original's one order per class (0x3b5394), which a ship joining
    /// the class takes (see [`Space::class_order`]). Not saved, as the
    /// hired escorts keep theirs on their records and mission escorts are
    /// never saved; reset with their orders on entering a system by the
    /// engine's [`RuleKey::EscortOrders`](crate::rulebook::RuleKey::EscortOrders).
    pub(super) class_orders: [Option<crate::escort::EscortOrder>; 4],
    /// The hail quotes' clock.
    pub(super) quote_clock: persons::QuoteClock,
}

impl Space {
    /// The system's contents as a session starts or loads: an empty
    /// system, to be populated on the first traffic tick in flight.
    pub(super) fn new() -> Self {
        Self {
            gate_sprites: BTreeMap::new(),
            comm_line: CommLine::default(),
            reinforcements: reinforcements::Reinforcements::default(),
            reinforcements_held: false,
            iff_marked: BTreeSet::new(),
            traffic: Traffic::new(),
            traffic_due: true,
            drawn: BTreeMap::new(),
            combat: Combat::default(),
            boxes: crate::cargo_box::CargoBoxes::default(),
            asteroids: crate::asteroid::Asteroids::default(),
            asteroid_kill: None,
            // A session starts or loads in flight, as `_DoNewPilot`
            // (@0x19384) and the main screen's entries (@0x1a8cf,
            // @0x1aff0) do, and places a field (see the `asteroids`
            // module).
            asteroid_field_due: true,
            play_area: crate::cloak::DEFAULT_PLAY_AREA,
            strikes: Vec::new(),
            frame: 0,
            pass_frame: 0,
            slot_sprites: crate::ship_layer::SlotSprites::default(),
            slot_shown: BTreeMap::new(),
            slot_placed: BTreeMap::new(),
            strength: BTreeMap::new(),
            destroying: BTreeSet::new(),
            rebuilt: Rebuilt::default(),
            stellar_sprites: StellarSpritePool::default(),
            escorts: Escorts {
                aboard: None,
                talk: None,
                fleet: Vec::new(),
                restock_persons: false,
                class_orders: [None; 4],
                quote_clock: persons::QuoteClock::default(),
            },
            voice: crate::voice::VoiceChannel::default(),
        }
    }
}
