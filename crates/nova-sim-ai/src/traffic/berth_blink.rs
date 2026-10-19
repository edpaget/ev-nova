//! The blink timer (+0xc8ec) each ship-table slot keeps after its ship is
//! gone.
//!
//! No routine but `_HandleShipDisplay`'s lights block writes +0xc8ec (its
//! only accesses are @0x2c328-0x2c55b): every spawn zeroes the lights'
//! intensity and count (`_RandomShipSpawn` @0x3c46e-0x3c478,
//! `_GenericRandomShipSpawn` @0x3ca04-0x3ca0a, `_LaunchFighter`
//! @0x3d8bd-0x3d8c3) but leaves the timer, and the table is a
//! `NewPtr(0x325000)` only the system's zero fill sets. So a new ship
//! brought into a slot (a [`Npc::berth`](crate::traffic::npc::Npc::berth))
//! blinks its running lights on the timer the slot's last ship left (see
//! [`ship_lights`](crate::combat::ship_lights)).
//!
//! [`BerthBlink`], a [`BerthKept`], keeps that timer for each berth,
//! written after each display ([`Traffic::keep_berth_blink`]), and gives a
//! new ship its berth's by the reading of
//! [`RuleKey::ReusedSlotBlinkTimer`](crate::RuleKey::ReusedSlotBlinkTimer):
//! the last ship's by the engine, 0 by the other reading. An escort
//! entering a system with its carried display keeps its own. The
//! player's own timer is the session's to carry.
//!
//! It lives as long as the [`Traffic`] holding it, across systems but not
//! across a new traffic. The original keeps a slot's timer through a
//! pilot load or a new game until the app quits; that is recorded only,
//! as [`berth_exits`](super::berth_exits) records it.

use crate::rulebook::RuleSource;

use super::Traffic;
use super::berth_kept::BerthKept;

/// The blink timer each berth's last ship left, and the reading a new
/// ship takes it by (see the module docs).
pub type BerthBlink = BerthKept<f32>;

impl Traffic {
    /// Writes each NPC's blink timer as its berth's, after a display has
    /// moved it on: the destroyed too, as they are still in the system
    /// until taken out.
    pub fn keep_berth_blink(&mut self) {
        for npc in &self.npcs {
            let timer = npc.display.frames.lights.timer;
            self.berth_blink.write(npc.berth, Some(timer));
        }
    }

    /// Gives each new ship its berth's last blink timer by `reading`
    /// ([`RuleKey::ReusedSlotBlinkTimer`](crate::RuleKey::ReusedSlotBlinkTimer)):
    /// by the engine, as no spawn writes +0xc8ec; 0 by the other reading.
    pub fn set_reused_slot_blink_timer(&mut self, reading: RuleSource) {
        self.berth_blink.set_reading(reading);
    }

    /// The reading new ships take their berth's last blink timer by (see
    /// [`Traffic::set_reused_slot_blink_timer`]).
    #[must_use]
    pub fn reused_slot_blink_timer(&self) -> RuleSource {
        self.berth_blink.reading()
    }

    /// The blink timer a ship brought into `berth` starts with (see the
    /// module docs).
    pub(super) fn berth_blink_timer(&self, berth: u32) -> f32 {
        self.berth_blink.seed(berth).unwrap_or(0.0)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::ShipId;
    use crate::combat::ship_frames::{DisplayRules, FrameSpec};
    use crate::flight::ShipState;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::AiType;
    use crate::traffic::spawn::{Entry, NewShip};
    use crate::traffic::table::{ShipKind, SpawnTable};
    use crate::traffic::{SPAWN_RESERVE, Traffic};
    use nova_sim_base::blink::Blink;
    use nova_sim_base::fixture::Draws;

    /// The Shuttle's square wave: lit for 3 ticks, twice, then a gap of 22.
    const SHUTTLE: Blink = Blink {
        mode: 1,
        a: 4,
        b: 1,
        c: 2,
        d: 20,
    };

    /// The blink timer of each NPC, by its berth, in order.
    fn timers(traffic: &Traffic) -> Vec<(u32, f32)> {
        traffic
            .npcs()
            .iter()
            .map(|npc| (npc.berth, npc.display.frames.lights.timer))
            .collect()
    }

    /// Traffic whose first NPC, at berth 1, left a blink timer of 15 and
    /// then went out, with a second at berth 2 still in, reading reused
    /// berths' timers by `reading`.
    fn left_blinking(reading: RuleSource) -> Traffic {
        let mut traffic = Traffic::new();
        traffic.set_reused_slot_blink_timer(reading);
        let first = traffic.add_npc(npc(0, ShipStats::default()));
        traffic.add_npc(npc(0, ShipStats::default()));
        traffic.npcs_mut()[0].display.frames.lights.timer = 15.0;
        traffic.keep_berth_blink();
        traffic.remove(first);
        traffic
    }

    #[test]
    fn a_new_ship_takes_its_berths_blink_timer_by_the_engine_and_zero_by_the_other_reading() {
        for (reading, timer) in [(RuleSource::Engine, 15.0), (RuleSource::Bible, 0.0)] {
            let mut traffic = left_blinking(reading);
            assert_eq!(traffic.reused_slot_blink_timer(), reading);
            traffic.add_npc(npc(0, ShipStats::default()));
            traffic.add_npc(npc(0, ShipStats::default()));
            assert_eq!(
                timers(&traffic),
                [(2, 0.0), (1, timer), (3, 0.0)],
                "{reading:?}: berth 1's left, berth 3 fresh"
            );
        }
    }

    #[test]
    fn a_placed_ships_own_timer_gives_way_to_its_berths() {
        let mut traffic = Traffic::new();
        let mut blinking = npc(0, ShipStats::default());
        blinking.display.frames.lights.timer = 9.0;
        traffic.add_npc(blinking);
        assert_eq!(timers(&traffic), [(1, 0.0)], "no spawn writes it");
    }

    /// A placed ship of type 200, its class blinking the Shuttle's wave.
    fn arrival() -> NewShip {
        NewShip {
            ship: ShipId(200),
            govt: None,
            ai_type: AiType::WimpyTrader,
            lead: None,
            state: ShipState::default(),
            entry: Entry::Placed,
            aggression: 0,
            booty: None,
            info_types: 0,
            person: None,
            mission: None,
            aux: None,
            derelict: false,
            held: false,
            boarded: false,
            cloaked: false,
            quote: None,
            cargo: None,
        }
    }

    /// `traffic` spawning ship 200, a Shuttle-like class with lights.
    fn spawning(traffic: &mut Traffic) {
        let kind = ShipKind {
            hull: crate::combat::hull::HullSpec {
                frames: FrameSpec {
                    blink: SHUTTLE,
                    lights: true,
                    ..FrameSpec::NONE
                },
                ..crate::combat::hull::HullSpec::default()
            },
            ..ShipKind::default()
        };
        traffic.table = SpawnTable {
            ships: BTreeMap::from([(ShipId(200), kind)]),
            ..SpawnTable::default()
        };
        traffic.bring_in_within(
            vec![arrival()],
            SPAWN_RESERVE,
            &crate::govt::Governments::empty(),
            &mut crate::chance::NeverFires,
        );
    }

    #[test]
    fn a_ship_spawned_into_a_reused_berth_blinks_on_its_last_ships_timer() {
        for (reading, timer) in [(RuleSource::Engine, 15.0), (RuleSource::Bible, 0.0)] {
            let mut traffic = left_blinking(reading);
            spawning(&mut traffic);
            assert_eq!(timers(&traffic), [(2, 0.0), (1, timer)], "{reading:?}");
        }
    }

    #[test]
    fn a_square_wave_spawned_on_a_timer_of_15_first_lights_once_it_runs_out() {
        let lit_on = |reading| {
            let mut traffic = left_blinking(reading);
            spawning(&mut traffic);
            (0..30)
                .position(|_| {
                    traffic.display_ships(|_| false, DisplayRules::ENGINE, &mut Draws::of(&[]));
                    traffic.npcs()[1].display.drawn.lights.is_some()
                })
                .expect("lit")
        };
        assert_eq!(lit_on(RuleSource::Bible), 1, "a fresh slot's: tick 1");
        assert_eq!(lit_on(RuleSource::Engine), 16, "15 down to -1 first");
    }

    #[test]
    fn each_display_writes_every_ships_timer_as_its_berths() {
        let mut traffic = Traffic::new();
        traffic.add_npc(npc(0, ShipStats::default()));
        traffic.npcs_mut()[0].hull.frames = FrameSpec {
            blink: SHUTTLE,
            lights: true,
            ..FrameSpec::NONE
        };
        traffic.display_ships(|_| false, DisplayRules::ENGINE, &mut Draws::of(&[]));
        traffic.display_ships(|_| false, DisplayRules::ENGINE, &mut Draws::of(&[]));
        let timer = traffic.npcs()[0].display.frames.lights.timer;
        assert_eq!(timer, 1.0, "lit for B on its second tick");
        assert_eq!(traffic.berth_blink.left(1), Some(timer));
    }
}
