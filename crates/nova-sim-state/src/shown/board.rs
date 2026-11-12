//! What boarding shows and answers: why the player cannot board
//! ([`BoardRefusal`]), what a board did ([`Boarding`],
//! [`MissionBoardLine`]), the plunder dialog's view, presses and their
//! outcomes ([`PlunderView`], [`AmmoOffer`], [`Take`], [`Taken`]) and the
//! captured ship's assignment ([`Assignment`], [`Assigned`]), from
//! `nova_sim_port::board`, which re-exports them and keeps the boarding
//! checks, the plunder's roll and the capture.

use nova_sim_catalog::catalog::{OutfitId, ShipId, WeaponId};

use super::board_repair::Recovery;
use super::combat::NpcId;
use super::mission::screens::VenueOffer;
use crate::market::Good;

/// Why the player cannot board its target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardRefusal {
    /// There is no target.
    NoTarget,
    /// "You can't board this ship." (`STR#` 2002 #130).
    CantBoard,
    /// A `PickupMode` 2 mission's special ship whose cargo has no room:
    /// `_DoMissionCargoPickup`'s refusal text (@0x982ff), then #130 as
    /// [`BoardRefusal::CantBoard`] says it (@0x65366 → @0x65fdb), and
    /// `_SetAllKeyCmdFlags` between them
    /// ([`BoardRefusal::sets_every_latch`]).
    NoRoomForCargo,
    /// "You're moving too fast to board this ship." (#132).
    TooFast,
    /// "You're not close enough to board this ship." (#131).
    TooFar,
    /// The player does not face the target's way, nor its reverse.
    Misaligned,
    /// The player is cloaked: refused silently, with a target, before
    /// every check (`_ShipIsCloaked` @0x6503f → 0x6605a, the no-target
    /// return @0x65031 first).
    Cloaked,
}

impl BoardRefusal {
    /// Whether the player is told nothing: no target, not facing it, or
    /// cloaked.
    #[must_use]
    pub fn silent(self) -> bool {
        matches!(self, Self::NoTarget | Self::Misaligned | Self::Cloaked)
    }

    /// Whether the refusal sets the board latch, so that holding the
    /// Board key tries no more until it is released: every refusal but no
    /// target and cloaked, as `_HandlePlayerBoardAttempt` sets `*0xdee18`
    /// (@0x6504c) only after those two returns (@0x65031, @0x6503f). A
    /// board that passes the checks latches too.
    #[must_use]
    pub fn latches(self) -> bool {
        !matches!(self, Self::NoTarget | Self::Cloaked)
    }

    /// Whether `_SetAllKeyCmdFlags` (@0x61e6d) ran before the refusal,
    /// latching every key down, so the dialog it showed closes on them
    /// latched: the pickup refused for room alone, as
    /// `_HandlePlayerBoardAttempt` calls it after `_DoMissionCargoPickup`
    /// whatever it answered (@0x6535a). Every other refusal returns
    /// before any site.
    #[must_use]
    pub fn sets_every_latch(self) -> bool {
        self == Self::NoRoomForCargo
    }
}

/// The rounds a boarded ship offers: the weapon whose slot they are, how many,
/// and the outfit the dialog names them by
/// (`nova_sim_port::board::HeldRounds::named`). The Ammo press finds the outfit
/// it takes them as afresh (`nova_sim_port::board::ammo_outfit`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AmmoOffer {
    /// The weapon whose slot they are.
    pub slot: WeaponId,
    /// How many.
    pub rounds: u32,
    /// The outfit the dialog names them by, if any.
    pub named: Option<OutfitId>,
}

/// A press in the plunder dialog: a button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Take {
    /// "Cargo" (item 2).
    Cargo,
    /// "Credits" (item 3).
    Credits,
    /// "Ammo" (item 4).
    Ammo,
    /// "Energy" (item 6).
    Energy,
    /// "Capture Ship" (item 7).
    Capture,
    /// "Abort" (item 1), and Return, Enter and Escape.
    Abort,
    /// A press that reaches no enabled button (item -1): one on a greyed
    /// button, off every button, or released off the button it began on,
    /// as `_TrackPlunderButtons` (@0x2732a) hit-tests only the enabled
    /// buttons (@0x273d3-0x27426) and `_PlunderFilter` maps anything else
    /// to item -1 and still ends the modal wait (@0x92fdf).
    Miss,
}

/// What a press in the plunder dialog did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Taken {
    /// No boarding is under way, or a capture awaits its assignment: the
    /// press is not the dialog's.
    Nothing,
    /// The press took nothing and the boarding goes on: a [`Take::Miss`],
    /// or a take whose value is not on board, which beeps its refusal.
    /// Either still rolls an armed self-destruct first, and disarms it.
    Missed,
    /// The cargo was taken: `stored` tons of `good`, as much as the hold
    /// had room for.
    Cargo {
        /// The good.
        good: Good,
        /// The tons stored; none when the hold was full.
        stored: u32,
    },
    /// These credits were taken.
    Credits(i64),
    /// `count` of the ammunition outfit `outfit` were taken, as many as
    /// the free mass, the outfit's `Max` and the rounds on board allow.
    Ammo {
        /// The ammunition outfit.
        outfit: OutfitId,
        /// How many were taken; none when there was no room.
        count: u32,
    },
    /// The energy was taken: `stored` of the `offered` fuel, as much as
    /// the tank had room for.
    Energy {
        /// The fuel on board.
        offered: u32,
        /// The fuel stored.
        stored: u32,
        /// Whether the tank is full after it, which the original's
        /// message tells apart from all of it stored.
        full: bool,
    },
    /// The self-destruct went off: the ship is breaking up, and the
    /// boarding is over.
    Tripped,
    /// The capture succeeded; `nova_sim::Session::assign`
    /// says what becomes of the ship.
    Captured,
    /// The capture succeeded, and with no crew to trade places with, the
    /// ship joined the fleet at once.
    Escorted,
    /// The capture failed, and the boarding is over.
    CaptureFailed,
    /// The capture succeeded, but the fleet is full: nothing changes, and
    /// the boarding goes on.
    FleetFull,
    /// The boarding is over.
    Aborted,
}

/// What boarding a ship did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Boarding {
    /// The plunder dialog opens on what is on board.
    Opened(PlunderView),
    /// The crew repelled the boarders: nothing can be taken.
    Repelled,
    /// The person boarded offers its mission in place of the plunder
    /// (`_HandlePlayerBoardAttempt` @0x65936): the offer dialog, or the
    /// mission accepted at once.
    Offered(VenueOffer),
    /// A mission's special ship was boarded, with no plunder, no crime
    /// and no person's offer (@0x657bf-0x657e3 → @0x65f43), saying the
    /// pickup path's or the assist path's comm line when it took one.
    MissionShip(Option<MissionBoardLine>),
    /// The player's own disabled escort or fighter, or a ship of a type
    /// its bays carry, came back into the fleet or a bay in place of the
    /// plunder (see `nova_sim_port::board_repair`).
    Recovered(Recovery),
}

/// What the comm line says as a mission's special ship is boarded on the
/// pickup path or the assist path (see the session's `mission_ships`
/// module). Not `Copy`: the pickup's line carries its cargo's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissionBoardLine {
    /// `STR#` 2002 #126, "Target ship has been boarded."
    Boarded,
    /// The ship boarded, of class `ship`, thanks the player for the fuel:
    /// "<its `shïp` name>:  " `STR#` 2002 #3 ", <the player's ship's
    /// name>." (@0x655ae-0x65663).
    Refuelled {
        /// The class of the ship boarded, which names it.
        ship: ShipId,
    },
    /// The mission's cargo, picked up as its ship of `PickupMode` 2 is
    /// boarded: `STR#` 2002 #106, a space, #107 and a space unless the
    /// cargo's name starts with `*`, the name without its `*`, a space,
    /// #108 (@0x653fb-0x65543).
    Retrieved {
        /// The cargo's `STR#` 4001 name, raw, a leading `*` kept, as the
        /// session resolved it from the mission's cargo type where the
        /// line is raised: none for a cargo of no type or a type the list
        /// has no entry for.
        cargo: Option<String>,
    },
}

/// What is on board the ship being boarded, as the plunder dialog shows
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlunderView {
    /// The ship boarded.
    pub npc: NpcId,
    /// Its ship class.
    pub ship: ShipId,
    /// The credits on board; none to take when 0.
    pub credits: i64,
    /// The cargo on board.
    pub cargo: Option<(Good, u32)>,
    /// The ammunition on board: the weapon, how many rounds, and the
    /// outfit the dialog names them by.
    pub ammo: Option<AmmoOffer>,
    /// The energy on board, in fuel units.
    pub fuel: u32,
    /// The capture odds, in percent.
    pub odds: u8,
}

impl PlunderView {
    /// Whether `take`'s button is enabled: its value is on board. Abort
    /// always is; a [`Take::Miss`] is no button.
    #[must_use]
    pub fn offers(&self, take: Take) -> bool {
        match take {
            Take::Cargo => self.cargo.is_some(),
            Take::Credits => self.credits > 0,
            Take::Ammo => self.ammo.is_some(),
            Take::Energy => self.fuel > 0,
            Take::Capture => self.odds > 0,
            Take::Abort => true,
            Take::Miss => false,
        }
    }
}

/// What the player makes of a ship captured: the assignment dialog's
/// buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assignment {
    /// "Use As Escort" (item 2).
    Escort,
    /// "Use As My Ship" (item 1).
    MyShip,
}

/// What an assignment did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assigned {
    /// The ship joined the fleet (`STR#` 2002 #123).
    Escort,
    /// The player took the ship over, and the old one joined the fleet
    /// (#304).
    MyShip,
    /// No ship slot was free for the old ship, so the swap was dropped and
    /// the prize lost (#305).
    Abandoned,
    /// Use As My Ship was chosen, and the ship awaits its new name before it
    /// is taken over (`nova_sim_port::capture_naming`); nothing is said yet.
    Naming,
    /// The rename was cancelled: nothing was taken over and the prize is
    /// lost (`STR#` 2002 #122).
    Declined,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The offer of `rounds` rounds of weapon `slot`, named by `named`.
    fn offer(slot: i16, rounds: u32, named: Option<i16>) -> AmmoOffer {
        AmmoOffer {
            slot: WeaponId(slot),
            rounds,
            named: named.map(OutfitId),
        }
    }

    #[test]
    fn only_no_target_and_cloaked_leave_the_board_latch_clear() {
        // The latch is set @0x6504c, after the no-target (@0x65031) and
        // cloak (@0x6503f) returns.
        assert!(!BoardRefusal::NoTarget.latches());
        assert!(!BoardRefusal::Cloaked.latches());
        for refusal in [
            BoardRefusal::CantBoard,
            BoardRefusal::NoRoomForCargo,
            BoardRefusal::TooFast,
            BoardRefusal::TooFar,
            BoardRefusal::Misaligned,
        ] {
            assert!(refusal.latches(), "{refusal:?}");
        }
    }

    #[test]
    fn only_the_pickup_refused_for_room_is_followed_by_set_all_key_cmd_flags() {
        // `_DoMissionCargoPickup` @0x6533d, then `_SetAllKeyCmdFlags`
        // @0x6535a, then #130 (@0x65366 → @0x65fdb); every other refusal
        // returns before any site.
        assert!(BoardRefusal::NoRoomForCargo.sets_every_latch());
        for refusal in [
            BoardRefusal::NoTarget,
            BoardRefusal::CantBoard,
            BoardRefusal::TooFast,
            BoardRefusal::TooFar,
            BoardRefusal::Misaligned,
            BoardRefusal::Cloaked,
        ] {
            assert!(!refusal.sets_every_latch(), "{refusal:?}");
        }
    }

    #[test]
    fn a_button_is_enabled_while_its_value_is_on_board() {
        let empty = PlunderView {
            npc: NpcId(1),
            ship: ShipId(129),
            credits: 0,
            cargo: None,
            ammo: None,
            fuel: 0,
            odds: 0,
        };
        for take in [
            Take::Cargo,
            Take::Credits,
            Take::Ammo,
            Take::Energy,
            Take::Capture,
        ] {
            assert!(!empty.offers(take), "{take:?}");
        }
        assert!(empty.offers(Take::Abort));
        let full = PlunderView {
            credits: 1,
            cargo: Some((Good::Commodity(0), 1)),
            ammo: Some(offer(140, 1, Some(200))),
            fuel: 1,
            odds: 1,
            ..empty
        };
        for take in [
            Take::Cargo,
            Take::Credits,
            Take::Ammo,
            Take::Energy,
            Take::Capture,
            Take::Abort,
        ] {
            assert!(full.offers(take), "{take:?}");
        }
        assert!(!full.offers(Take::Miss), "a miss is no button");
        assert!(!empty.offers(Take::Miss));
    }

    #[test]
    fn misaligned_and_no_target_are_silent() {
        assert!(BoardRefusal::NoTarget.silent());
        assert!(BoardRefusal::Misaligned.silent());
        for loud in [
            BoardRefusal::CantBoard,
            BoardRefusal::NoRoomForCargo,
            BoardRefusal::TooFast,
            BoardRefusal::TooFar,
        ] {
            assert!(!loud.silent(), "{loud:?}");
        }
    }

    #[test]
    fn a_cloaked_refusal_is_silent() {
        // `_ShipIsCloaked` @0x6503f returns through 0x6605a before the
        // latch is set (@0x6504c): no beep, no line.
        assert!(BoardRefusal::Cloaked.silent());
    }
}
