//! Scooping asteroid boxes into the hold: which ships scoop, which boxes
//! they take and what each box adds. The session runs it each flight pass
//! (`Session::scoop_boxes`, in the session's `cargo_boxes` module).
//!
//! # The original
//!
//! In `EV Nova.app/Contents/MacOS/EV Nova`:
//!
//! - **The player's scoop flag** (ship +0xc3, [`player_scoops`]). No key
//!   or control sets it. `_ResetPlayerPrecalcedValues` clears it
//!   (@0xc462), sets it when the player owns an outfit with `ModType`
//!   [`MOD_ASTEROID_SCOOP`] in any of its four mod slots
//!   ([`has_asteroid_scoop`], @0xc5af-0xc5b6, @0xc6fe-0xc70e), and clears
//!   it again when `_TotalFleetHolds` − `_TotalCargo(player)` ≤ 0
//!   (@0xc775-0xc7a7). With a full hold the player doesn't scoop, and the
//!   box drifts on until it expires.
//! - **When the collision runs.** `_DoPlayGameWork` collides `_boxLayer`
//!   with the ship layer `*0x3b541c` (@0x43e46-0x43e70) only while
//!   `_miningBoxesActive` is set, which `_HandleBoxes` clears
//!   (@0x2df8e-0x2dfa1) and each live scoopable box sets
//!   (@0x2e26c-0x2e283). `_HandleShipDisplay` (@0x2bd24-0x2bd47) puts a
//!   disabled ship in `*0x3b53a0` and the player's escorts and fighters
//!   (parent +0x9a == 0) in `*0x3b52a8` (see
//!   [`ship_layer`](crate::ship_layer)), so neither scoops; every other
//!   ship, the player included, is in `*0x3b541c` ([`npc_in_ship_layer`]).
//! - **The scoop branch of `_ShipCollisionHandler`** (@0x370ab-0x371ac).
//!   The other sprite must be in `_boxLayer`. The ship must be scooping:
//!   for the player its +0xc3 (@0x370c2); for an NPC
//!   `_AIShipIsScooping` (@0x88727, the goal 0x11, a miner's state 17,
//!   [`Goal::Scoop`](crate::ai::Goal::Scoop); see [`Npc::is_scooping`]),
//!   with no +0xc3 test (@0x370cb-0x370d5). The
//!   box must be scoopable (@0x370fb) and must pass `_SWRLECollision`
//!   (@0x3710c) over the two sprites' placed rects, the box's up and left
//!   of its point as `_HandleBoxes` places it (@0x2e1c2-0x2e250), which
//!   the engine stands in for with [`touches`], on that same rect. Then the
//!   box's life becomes −1 and its sprite is hidden (@0x37119-0x3712e);
//!   contents 0-5 add a ton of that commodity to the ship's own hold
//!   (@0x3713a-0x37158); otherwise an NPC stops there (@0x3715f); for the
//!   player, contents 1000-1127 add a ton of that `jünk`
//!   (@0x37169-0x37188); any other contents use the box up for nothing
//!   ([`haul`]). For the player alone, the status redraw flag is set
//!   (@0x37199) and `_ResetPlayerPrecalcedValues` runs (@0x371ac).
//! - **No sound and no message.** The player sees only the cargo readout
//!   change.
//! - **An NPC's scoop flag** (ship +0xc3): `_ShipCanScoop`
//!   (@0x75c7-0x766b), the same `ModType` 31 test against its `shïp`'s
//!   default outfits, which the NPC spawners call and only `_MinerAI`
//!   reads ([`ShipKind::can_scoop`](crate::traffic::table::ShipKind),
//!   [`Npc::can_scoop`], see [`miner`](crate::ai::miner)). The scoop
//!   branch doesn't read it.
//!
//! The murk (`_HandleSpriteMurkiness` @0x2e15e-0x2e181) is the view's:
//! `nova_view::flight::murk` mixes each box toward the background
//! (rdm:phase/play-feel-4/phase-17-murk-other-sprites).

use std::collections::BTreeMap;

use crate::cargo_box::CargoBoxes;
use crate::catalog::outfit_mods;
use crate::catalog::{BOX_GRAPHICS, JunkId, OutfitId, OutfitRecord};
use crate::geometry::Vec2;
use crate::ship_layer::{ShipLayer, disabled};
use crate::traffic::npc::{Npc, NpcHold};

/// The `ModType` of an asteroid scoop.
pub const MOD_ASTEROID_SCOOP: i16 = 31;

/// The last commodity a box can hold (@0x3713a).
const LAST_COMMODITY: i16 = 5;

/// The contents of a box holding `jünk` 128, the first.
const FIRST_JUNK_CONTENTS: i16 = 1000;

/// The contents of a box holding the last `jünk`, 255.
const LAST_JUNK_CONTENTS: i16 = 1127;

/// The first `jünk`'s ID.
const FIRST_JUNK: i16 = 128;

/// Whether the outfits `owned` (of `records`) hold an asteroid scoop: an
/// outfit with a count above 0 and a `ModType` [`MOD_ASTEROID_SCOOP`] in
/// any of its four slots (@0xc5af-0xc5b6, @0xc6fe-0xc70e).
#[must_use]
pub fn has_asteroid_scoop(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> bool {
    outfit_mods(owned, records)
        .iter()
        .any(|m| m.count > 0 && m.mod_type == MOD_ASTEROID_SCOOP)
}

/// The player's scoop flag (ship +0xc3): it owns a scoop (`has_scoop`)
/// and the fleet's holds `fleet_holds` are past the `total_cargo` it
/// holds (@0xc775-0xc7a7).
#[must_use]
pub fn player_scoops(has_scoop: bool, fleet_holds: u32, total_cargo: i64) -> bool {
    has_scoop && i64::from(fleet_holds) - total_cargo > 0
}

/// Which ship scoops a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scooper {
    /// The player.
    Player,
    /// An NPC.
    Npc,
}

/// What a scooped box adds to the ship's hold: a ton of one good, or
/// nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Haul {
    /// A ton of this commodity, 0-5.
    Commodity(u8),
    /// A ton of this `jünk`: the player's alone.
    Junk(JunkId),
    /// Nothing: the box is used up all the same.
    Nothing,
}

/// What a box holding `contents` adds when `by` scoops it
/// (@0x3713a-0x37188): a commodity 0-5 to any ship; a `jünk` 1000-1127,
/// `jünk` 128 + (contents − 1000), to the player alone (an NPC stops at
/// @0x3715f); nothing for any other contents.
#[must_use]
pub fn haul(contents: Option<i16>, by: Scooper) -> Haul {
    match (contents, by) {
        (Some(commodity @ 0..=LAST_COMMODITY), _) => {
            // In 0..=5.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let commodity = commodity as u8;
            Haul::Commodity(commodity)
        }
        (Some(junk @ FIRST_JUNK_CONTENTS..=LAST_JUNK_CONTENTS), Scooper::Player) => {
            Haul::Junk(JunkId(FIRST_JUNK + (junk - FIRST_JUNK_CONTENTS)))
        }
        _ => Haul::Nothing,
    }
}

/// Each box graphic's frame width and height, `spïn` 500 + graphic, as
/// the catalog reads them
/// ([`AsteroidCatalog::box_frame_sizes`](crate::catalog::AsteroidCatalog::box_frame_sizes)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BoxFrames([Option<(u32, u32)>; BOX_GRAPHICS]);

impl BoxFrames {
    /// The frames of `sizes`, by graphic.
    #[must_use]
    pub fn new(sizes: [Option<(u32, u32)>; BOX_GRAPHICS]) -> Self {
        Self(sizes)
    }

    /// Half graphic `graphic`'s frame, across and down: none when its
    /// size is not known or there is no such graphic, so the box is
    /// tested as its point alone.
    #[must_use]
    pub fn half(&self, graphic: u16) -> Vec2 {
        match self.0.get(usize::from(graphic)).copied().flatten() {
            // Sprite sizes, far inside f32's exact range.
            #[allow(clippy::cast_precision_loss)]
            Some((width, height)) => Vec2::new(width as f32 * 0.5, height as f32 * 0.5),
            None => Vec2::ZERO,
        }
    }
}

/// Whether a ship's disc, `radius` about `ship_at`, touches a box at
/// `box_at` whose frame is `half` × 2 across and down: no farther than
/// `radius` from the frame's rectangle, `[box_at − half × 2, box_at]`.
/// `_HandleBoxes` places the sprite up and left of the box's point
/// (@0x2e1c2-0x2e250), and `_SWRLECollision` (@0x3710c) tests the rect so
/// placed, pixel by pixel. So the frame's centre is `box_at − half`, not
/// the box's point, which is the rectangle's corner: a ship on the point
/// still touches, as a miner flying at it (`_MinerAI` reads the position)
/// must. The disc is the engine's approximation of the pixels; the engine
/// has no pixel masks, as `nova_sim::story` approximates it for stellars.
#[must_use]
pub fn touches(ship_at: Vec2, radius: f32, box_at: Vec2, half: Vec2) -> bool {
    let off = ship_at - (box_at - half);
    let outside = Vec2::new(
        (off.x.abs() - half.x).max(0.0),
        (off.y.abs() - half.y).max(0.0),
    );
    outside.length() <= radius
}

/// Whether `npc` is in the ship layer `*0x3b541c`, the one boxes collide
/// with (`_HandleShipDisplay` @0x2bd24-0x2bd47, see
/// [`ShipLayer::of`]): not disabled, nor breaking up but for a
/// stellar's defence ship ([`disabled`]), and not one of the player's escorts and fighters,
/// whose parent (+0x9a) is the player ([`Npc::escorts_player`]).
#[must_use]
pub fn npc_in_ship_layer(npc: &Npc) -> bool {
    ShipLayer::of(
        disabled(npc.condition, npc.defends.is_some()),
        npc.escorts_player(),
    ) == ShipLayer::Ships
}

/// The NPC side of the scoop branch: a scooping NPC's disc, `radius`
/// about `at`, takes every scoopable box in `boxes` it touches (by their
/// `frames`), each commodity box adding a ton of its commodity to `hold`
/// and any other box used up for nothing ([`haul`]). There is no
/// hold-space test: the NPC branch has none.
pub fn npc_scoop(
    boxes: &mut CargoBoxes,
    frames: &BoxFrames,
    at: Vec2,
    radius: f32,
    hold: &mut NpcHold,
) {
    while let Some(taken) = boxes.take_touching(at, radius, frames) {
        if let Haul::Commodity(commodity) = haul(taken.contents, Scooper::Npc) {
            hold.add(commodity, 1);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::market::Good;
    use crate::stats::ShipStats;
    use crate::traffic::ShipBehav;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{MissionMark, NpcHold, NpcId};
    use nova_sim_catalog::fixture::outfit;

    fn owning(outfits: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        outfits.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    #[test]
    fn the_scoop_is_an_owned_outfit_with_mod_type_31_in_any_slot() {
        let records = [
            outfit(200, &[(MOD_ASTEROID_SCOOP, 0)]),
            outfit(201, &[(1, 0), (2, 0), (3, 0), (MOD_ASTEROID_SCOOP, 0)]),
            outfit(202, &[(30, 0), (32, 0)]),
        ];
        let scoop = |owned: &[(i16, u16)]| has_asteroid_scoop(&owning(owned), &records);
        assert!(scoop(&[(200, 1)]));
        assert!(scoop(&[(201, 3)]), "the fourth slot counts");
        assert!(!scoop(&[(200, 0)]), "none owned");
        assert!(!scoop(&[(202, 1)]), "other mods");
        assert!(!scoop(&[]));
    }

    #[test]
    fn the_player_scoops_with_a_scoop_and_room_in_the_fleets_holds() {
        assert!(player_scoops(true, 20, 19), "a ton free");
        assert!(!player_scoops(true, 20, 20), "full");
        assert!(!player_scoops(true, 20, 23), "past full");
        assert!(!player_scoops(false, 20, 0), "no scoop");
        assert!(player_scoops(true, u32::MAX, -5), "no overflow");
    }

    #[test]
    fn a_box_holds_a_commodity_0_to_5() {
        for scooper in [Scooper::Player, Scooper::Npc] {
            assert_eq!(haul(Some(0), scooper), Haul::Commodity(0));
            assert_eq!(haul(Some(5), scooper), Haul::Commodity(5));
            for nothing in [
                Some(6),
                Some(-1),
                Some(999),
                Some(1128),
                Some(i16::MIN),
                None,
            ] {
                assert_eq!(haul(nothing, scooper), Haul::Nothing, "{nothing:?}");
            }
        }
    }

    #[test]
    fn a_junk_box_holds_junk_for_the_player_alone() {
        assert_eq!(haul(Some(1000), Scooper::Player), Haul::Junk(JunkId(128)));
        assert_eq!(haul(Some(1018), Scooper::Player), Haul::Junk(JunkId(146)));
        assert_eq!(haul(Some(1127), Scooper::Player), Haul::Junk(JunkId(255)));
        assert_eq!(haul(Some(1000), Scooper::Npc), Haul::Nothing);
        assert_eq!(haul(Some(1127), Scooper::Npc), Haul::Nothing);
    }

    #[test]
    fn a_box_frames_half_is_half_its_known_size() {
        let frames = BoxFrames::new([None, Some((22, 18)), Some((7, 9)), None, None]);
        assert_eq!(frames.half(1), Vec2::new(11.0, 9.0));
        assert_eq!(frames.half(2), Vec2::new(3.5, 4.5));
        assert_eq!(frames.half(0), Vec2::ZERO, "unknown");
        assert_eq!(frames.half(5), Vec2::ZERO, "past the graphics");
        assert_eq!(frames.half(u16::MAX), Vec2::ZERO);
        assert_eq!(BoxFrames::default().half(1), Vec2::ZERO);
    }

    #[test]
    fn a_disc_touches_a_box_within_its_radius_of_the_frame() {
        let half = Vec2::new(10.0, 5.0);
        let box_at = Vec2::new(100.0, 200.0);
        let touches_at = |x, y| touches(Vec2::new(x, y), 4.0, box_at, half);
        // The frame is up and left of the box's point: x 80 to 100, y 190
        // to 200.
        assert!(touches_at(100.0, 200.0), "on the box's own point");
        assert!(touches_at(90.0, 195.0), "on the frame's centre");
        assert!(touches_at(104.0, 195.0), "the radius off the right edge");
        assert!(touches_at(76.0, 195.0), "the radius off the left edge");
        assert!(touches_at(90.0, 204.0), "the radius off the bottom edge");
        assert!(touches_at(90.0, 186.0), "the radius off the top edge");
        assert!(!touches_at(104.1, 195.0), "just beyond the right edge");
        assert!(!touches_at(75.9, 195.0), "just beyond the left edge");
        assert!(!touches_at(90.0, 204.1), "just beyond the bottom edge");
        assert!(!touches_at(90.0, 185.9), "just beyond the top edge");
        // Off the corner (100, 200): 3.5 across and 3.5 down, each within
        // the radius, but 4.95 away.
        assert!(!touches_at(103.5, 203.5), "diagonal off a corner");
        assert!(touches_at(102.0, 202.0), "2.83 off a corner");
        assert!(
            !touches_at(109.0, 205.0),
            "inside a frame centred on the point, but nothing is drawn there"
        );
        let point = |x, y| touches(Vec2::new(x, y), 4.0, box_at, Vec2::ZERO);
        assert!(point(104.0, 200.0), "a frame of no size is its point");
        assert!(!point(103.0, 203.0));
    }

    fn duty() -> EscortDuty {
        EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        }
    }

    #[test]
    fn an_npc_is_in_the_ship_layer_unless_disabled_or_the_players() {
        let plain = npc(1, ShipStats::default());
        assert!(npc_in_ship_layer(&plain));
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
            Condition::Destroyed,
        ] {
            let downed = Npc {
                condition,
                ..plain.clone()
            };
            assert!(!npc_in_ship_layer(&downed), "{condition:?}");
        }
        let guard = Npc {
            defends: Some(crate::catalog::StellarId(128)),
            condition: Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
            ..plain.clone()
        };
        assert!(
            npc_in_ship_layer(&guard),
            "a defence ship breaking up is not disabled (@0x2d6f)"
        );
        let hired = Npc {
            escort: Some(duty()),
            ..plain.clone()
        };
        assert!(!npc_in_ship_layer(&hired), "a hired escort");
        let mission_escort = Npc {
            mission: Some(MissionMark {
                slot: 0,
                behav: ShipBehav::EscortPlayer,
                formation: Some(FormationSlot::default()),
                order: None,
            }),
            ..plain.clone()
        };
        assert!(mission_escort.escorts_player());
        assert!(!npc_in_ship_layer(&mission_escort), "a mission escort");
        let carrier = |ship| crate::bay::Carrier {
            ship,
            window: 10.0,
            reach: 30.0,
        };
        let fighter = Npc {
            escort: Some(duty()),
            carrier: Some(carrier(ShipRef::Player)),
            ..plain.clone()
        };
        assert!(!npc_in_ship_layer(&fighter), "the player's fighter");
        let theirs = Npc {
            leader: Some(NpcId(9)),
            carrier: Some(carrier(ShipRef::Npc(NpcId(9)))),
            ..plain
        };
        assert!(npc_in_ship_layer(&theirs), "another ship's fighter");
    }

    /// A scoopable box at the origin holding `contents`, of graphic 1.
    fn one_box(contents: Option<i16>) -> CargoBoxes {
        let mut boxes = CargoBoxes::default();
        boxes.spawn_scoopable(Vec2::ZERO, contents, 1, &mut NeverFires);
        boxes
    }

    #[test]
    fn a_scooping_npc_takes_a_ton_of_a_commodity_box() {
        let mut boxes = one_box(Some(2));
        let mut hold = NpcHold::default();
        npc_scoop(
            &mut boxes,
            &BoxFrames::default(),
            Vec2::ZERO,
            16.0,
            &mut hold,
        );
        assert_eq!(hold.held().collect::<Vec<_>>(), [(Good::Commodity(2), 1)]);
        assert!(boxes.boxes().is_empty());
    }

    #[test]
    fn a_scooping_npc_uses_up_a_junk_box_for_nothing() {
        for contents in [Some(1003), Some(6), None] {
            let mut boxes = one_box(contents);
            let mut hold = NpcHold::default();
            npc_scoop(
                &mut boxes,
                &BoxFrames::default(),
                Vec2::ZERO,
                16.0,
                &mut hold,
            );
            assert_eq!(hold, NpcHold::default(), "{contents:?}");
            assert!(boxes.boxes().is_empty(), "{contents:?}");
        }
    }

    #[test]
    fn a_scooping_npc_keeps_its_commodity_and_gains_another() {
        let mut boxes = one_box(Some(1));
        let mut hold = NpcHold::of(Some((Good::Commodity(3), 7)));
        npc_scoop(
            &mut boxes,
            &BoxFrames::default(),
            Vec2::ZERO,
            16.0,
            &mut hold,
        );
        assert_eq!(
            hold.held().collect::<Vec<_>>(),
            [(Good::Commodity(1), 1), (Good::Commodity(3), 7)]
        );
    }

    #[test]
    fn a_scooping_npc_takes_every_box_it_touches_and_no_other() {
        let mut boxes = CargoBoxes::default();
        for (x, contents) in [(0.0, 0), (5.0, 0), (30.0, 4), (-10.0, 4)] {
            boxes.spawn_scoopable(Vec2::new(x, 0.0), Some(contents), 1, &mut NeverFires);
        }
        let mut hold = NpcHold::default();
        // Graphic 1 is 10 across, up and left of the box: the box at 30
        // is 20 from its edge.
        let frames = BoxFrames::new([None, Some((10, 10)), None, None, None]);
        npc_scoop(&mut boxes, &frames, Vec2::ZERO, 16.0, &mut hold);
        assert_eq!((hold.tons(0), hold.tons(4)), (2, 1));
        assert_eq!(boxes.boxes().len(), 1);
        assert_eq!(boxes.boxes()[0].position, Vec2::new(30.0, 0.0));
    }
}
