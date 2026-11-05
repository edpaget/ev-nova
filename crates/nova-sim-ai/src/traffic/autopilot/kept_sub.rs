//! The autopilot's low level under the AI wait: `_AIDispatch` tail-jumps
//! to `_LowLevelAIHandler` (@0x901f4 in the `EV Nova` executable) after a
//! high level that returned at its wait gate (@0x8d4b4-0x8d4d2) for every
//! goal but 0xa and 0xe, so the low level runs the block of the sub-goal
//! +0xc8ca kept, with no high level to rewrite it ([`under_wait`]).
//!
//! The engine keeps no +0xc8ca, so the sub is read from what the last
//! unwaited flight stored. Each unwaited flight stores at most one of
//! these, so the order of the reads decides nothing:
//!
//! - **Sub 1** ([`Npc::stopping`]): the stop's whole flight
//!   ([`leave_flight::stop`]), its retro heading and brake's thrust as its
//!   word, its cuts and settle at the slot (@0x8536d-0x85633).
//! - **Orders 0x13, 0x14 and 0x15** ([`Npc::mine_order`]), whatever the
//!   goal, as a miner's own think under its wait writes goal 0 and leaves
//!   the sub (`_MinerAI` @0x8b45d): 0x13's chase word and 0x14's ease,
//!   nudge and lead by asteroid slot 0 (@0x88140-0x88443,
//!   [`mine_flight::by_order`]), and 0x15's steer or brake toward the
//!   nearest box (@0x8844b-0x88642, [`mine_flight::scoop`]), which with no
//!   box stores sub 0 (@0x88636).
//! - **A fight sub, 0xe included** ([`waiting_pursuit::held_sub`]): the
//!   frame the think's [`waiting_pursuit::under_wait`] stored, its last
//!   body flown at the ship +0x70 names, there and not disabled
//!   (@0x870ab-0x870b9) ([`fight`](low_level::fight)), so sub
//!   0xe's damping at rest (@0x8725a-0x8728e), its inertialess brake word
//!   (@0x87121-0x87137) and each fight body's word. Not through the
//!   pursuit's frame for the goal (`pursuit::frame_of`), which works a
//!   frame out again for another goal, nor state 3's far branch, which is
//!   the high level's.
//! - **Sub 0** otherwise: the head's word alone.
//!
//! `_HandleShip` skips the turn and thrust while the wait runs
//! (@0x34a54-0x34a72), so the direct writes show at once and the word
//! where the countdown ends the wait on double time's advance pass: the
//! second pass runs no `_AIDispatch` (@0x4402a) and flies it.
//!
//! **Left out** (rdm:phase/npc-ai-2/phase-6-subs-kept-under-the-ai-wait):
//! sub 0xc under a goal a think wrote over 0xa (@0x878fc-0x87e09), the
//! subs the engine keeps no store for (2, 3, 8, 9, 0xa, 0xb, 0x16), and
//! orders 0x13 and 0x14 with slot 0 emptied, whose stale record the
//! original reads (no activity test @0x881ca-0x881e0). No stock writer of
//! +0x4c starts a wait holding any of them. None of these blocks plays a
//! sound.

use super::low_level;
use super::{Beside, Readings};
use crate::ai::Goal;
use crate::ai::mine_orders::MineOrder;
use crate::ai::waiting_pursuit;
use crate::combat::armament::Arsenal;
use crate::flight::ShipState;
use crate::hail::Help;
use crate::handling::Handling;
use crate::traffic::held_burst::Flight;
use crate::traffic::inertialess::Inertia;
use crate::traffic::leave_flight;
use crate::traffic::mine_flight;
use crate::traffic::npc::Npc;

/// `npc`'s low level under its AI wait (see the module docs), the ship
/// +0x70 names at `quarry` when it is there, disabled as `quarry_disabled`
/// says, with `beside` its asteroid slot 0 and nearest box, on `handling`, as
/// `inertia` says, by the disputed rules' `readings`, leading by
/// `weapons`: the flight of the sub it kept.
pub(super) fn under_wait(
    npc: &mut Npc,
    (quarry, quarry_disabled, beside): (Option<&ShipState>, bool, Beside),
    handling: &Handling,
    inertia: Inertia,
    (readings, weapons): (Readings, &Arsenal),
) -> Flight {
    if npc.stopping {
        // Goal 9, the refuel, is cut off its retro heading too (+0xc8c8
        // @0x85595).
        let refuelling = matches!(npc.goal, Goal::Assist(Help::Refuel));
        return leave_flight::stop(npc, handling, inertia, refuelling);
    }
    let gain = readings.speed_mult;
    match npc.mine_order {
        MineOrder::Stop => {}
        MineOrder::Scoop => {
            if beside.scoop_box.is_none() {
                npc.mine_order = MineOrder::Stop;
            }
            return mine_flight::scoop(npc, beside.scoop_box, handling, gain);
        }
        order @ (MineOrder::Chase | MineOrder::Kill) => {
            if let Some(flight) = (beside.rock.as_ref())
                .and_then(|rock| mine_flight::by_order(npc, order, rock, handling, weapons, gain))
            {
                return flight;
            }
            return low_level::none(npc);
        }
    }
    // Each fight block needs +0x70 there and not disabled (sub 0xe
    // @0x870ab-0x870b9, sub 0x10 @0x8641e-0x8642c).
    if waiting_pursuit::held_sub(&npc.pursuit).is_some()
        && !quarry_disabled
        && let Some(target) = quarry
        && let Some(frame) = npc.pursuit.frame
        && let Some(body) = frame.last()
    {
        let by = (readings.snipe_lead, weapons);
        return low_level::fight(npc, target, (body, frame.speed_cap), handling, inertia, by);
    }
    low_level::none(npc)
}
