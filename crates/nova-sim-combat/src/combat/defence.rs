//! Point defence: a ship's point-defence weapons (guidance 9 and 10)
//! shooting down the missiles fired at it, and else firing at the ships
//! fighting it, with no target needed.
//!
//! The values are the original's (`_HandleShipPointDefense`,
//! `_PointDefenseCollisionHandler` and `_HandleBeams` in the `EV Nova`
//! executable):
//!
//! - Each tick its trigger asks
//!   ([`Trigger::point_defence`](super::armament::Trigger::point_defence)):
//!   the player's every tick while intact, an NPC's when its AI asks for a
//!   turret (`_AIFireTurret` @0x80b0d), each intact ship whose jump is not
//!   under way fires its first ready point-defence weapon it can pay for,
//!   by weapon ID, at the nearest candidate missile ([`choose`]), else at
//!   the nearest candidate ship ([`choose_ship`]), and nothing when there
//!   is neither: a later weapon is never tried. Its timer
//!   takes `Reload / count` on top of what is left of it, so two Quads
//!   fire every two and a half ticks
//!   ([`Armament::fire_point_defence`](super::armament::Armament::fire_point_defence)).
//! - A candidate is a homing shot that is not jammed (lost), that point
//!   defence can target (no `Flags` 0x0080), whose damage is aimed at this
//!   ship or at its fleet's lead (an escort defends its lead), that is
//!   within the weapon's [`engagement_range`] and not in a blind spot (see
//!   [`aim`](super::aim)), its squared distance truncated. The nearest
//!   wins; of two as near, the one fired first. The original has no
//!   hostility test: "fired at me or my lead" is the whole rule, so by
//!   default the player's point defence engages its own escort's stray
//!   missile, and police an allied trader's. By the other reading of
//!   [`RuleKey::PdHostility`](crate::RuleKey::PdHostility) the
//!   [`PointDefenceRule`] must also call the firer hostile: Nova's
//!   [`Allegiance`] calls a missile hostile unless its firer is in the
//!   defender's fleet, or both have governments that are allies.
//! - Only with no candidate missile, however near a ship is, does it
//!   fire at a ship (@0x394c3): one of a class of `Flags2` 0x0008
//!   ([`HullSpec::pd_target`]), neither the defender nor its leader,
//!   intact and not held disabled (`_IsDisabled`), seen by the defender
//!   (`_ShipVisibleToShip`: not in its gate, and uncloaked or within 200
//!   pixels on each axis of a cloak scanner on screen), within the same
//!   range, by truncated squared distance, and not in a blind spot; and
//!   fighting the defender: the player while the defender threatens it
//!   (`_ExtendedIsThreatToPlayer`), any other ship while it attacks or
//!   snipes at the defender or the defender's leader (`_IsShipFightingMe`,
//!   never fleeing). The nearest wins; of two as near, the first in the
//!   fight's order. These are the whole rule, with no hostility test
//!   under either reading of
//!   [`RuleKey::PdHostility`](crate::RuleKey::PdHostility). So a player
//!   escort's point defence never fires at the player, and a hostile
//!   carrier's fires at a player flying a `Flags2` 0x0008 ship, the stock
//!   Shuttle among them.
//! - A point-defence turret (9) fires a shot at the missile's bearing, no
//!   lead, plus its inaccuracy; a point-defence beam (10) is held on the
//!   missile, with no inaccuracy. At a ship, the shot leads it from its
//!   exit, plus its inaccuracy, with no target and aimed at none, and
//!   moves to the shot layer ([`ShotLayer`](super::projectile::ShotLayer));
//!   the beam is held on the ship and aimed at it, with no inaccuracy, and
//!   hits ships as a turreted beam does.
//! - A point-defence shot meets missiles by their sprites, as the
//!   original's collision pass does (`_SWCollideSpriteLayer`
//!   @0xacb4b-0xace4e): each sprite is a box of its frame's size
//!   ([`WeaponSpec::frame`], its sheet's, with no collision inset or hot
//!   spot), its top left half its frame's *width* up and left of its
//!   offset from the player, each truncated to whole pixels
//!   ([`SpriteBox`], `_HandleShot` @0x35c48-0x35cc0), and two meet when
//!   their boxes overlap, strictly ([`SpriteBox::meets`]). They are tested
//!   where both end the tick, not swept, so a shot can pass through a
//!   missile between ticks as in the original; a shot of no sprite never
//!   meets. A point-defence shot meets every homing missile its box
//!   overlaps, with no other test: lost, untargetable, friendly or aimed
//!   elsewhere (`_PointDefenseCollisionHandler` @0x37776-0x3785d), but not
//!   one that touched a ship on its move, whose hit comes first
//!   ([`Combat`](super::Combat), step 2).
//! - Each hit takes [`pd_damage`], `MassDmg` plus half the `EnergyDmg`
//!   (truncated), off the missile's durability while that is above none;
//!   the hit after that destroys it ([`Shot::take_pd_hit`]), shot down
//!   where the point-defence shot is, with no detonation. A
//!   point-defence shot is spent on the missiles it meets; a beam hits the
//!   missile it is held on every tick.
//! - A point-defence shot fired at a missile meets a ship only by its
//!   proximity fuse ([`Shot::hit`]): the Quad Light Blaster Turret's
//!   (`ProxRadius` 5) does, the Storm Chaingun's (0) never. One fired at
//!   a ship, in the shot layer, meets ships by contact too, and meets no
//!   missile.

use std::fmt::Debug;

use super::ShipRef;
use super::aim::blind_spot;
use super::hull::{Condition, HullSpec};
use super::projectile::{Jam, Shot, ShotId, Target};
use super::weapon::{Guidance, WeaponSpec};
use crate::catalog::{FrameSize, GovtId};
use crate::cloak::{self, Observer};
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::trig;

/// A point-defence turret's engagement range, per pixel of its range.
pub const PD_RANGE_FACTOR: f32 = 1.5;

/// A ship, its fleet (the lead it escorts, or itself) and its government.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Side {
    /// The ship.
    pub ship: ShipRef,
    /// Its fleet.
    pub fleet: ShipRef,
    /// Its government, or `None` for an independent ship or the player.
    pub govt: Option<GovtId>,
}

impl Side {
    /// The side that fired `shot`.
    #[must_use]
    pub fn of(shot: &Shot) -> Self {
        Self {
            ship: shot.firer,
            fleet: shot.fleet,
            govt: shot.govt(),
        }
    }
}

/// Which missiles point defence engages, by who fired them: asked only by
/// the other reading of [`RuleKey::PdHostility`](crate::RuleKey::PdHostility),
/// as the original has no hostility test ([`choose`]).
pub trait PointDefenceRule: Debug {
    /// Whether a missile fired by `firer` is hostile to `defender`, with
    /// the relations between governments in `govts`.
    fn hostile(&self, defender: Side, firer: Side, govts: &Governments) -> bool;
}

/// Nova's rule (see the module docs): a missile is hostile unless its
/// firer is in the defender's fleet, or both have governments that are
/// allies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Allegiance;

impl PointDefenceRule for Allegiance {
    fn hostile(&self, defender: Side, firer: Side, govts: &Governments) -> bool {
        let allied = defender.govt.is_some()
            && firer.govt.is_some()
            && govts.allies(defender.govt, firer.govt);
        firer.fleet != defender.fleet && !allied
    }
}

/// How near, in pixels, a missile must be for point defence `spec` to
/// engage it: a beam's length, or a turret's range down its sub-munition
/// chain ([`WeaponSpec::range`]), truncated, times [`PD_RANGE_FACTOR`],
/// truncated again.
#[must_use]
pub fn engagement_range(spec: &WeaponSpec) -> f32 {
    if spec.guidance == Guidance::PointDefenceBeam {
        spec.beam_length
    } else {
        (spec.range().trunc() * PD_RANGE_FACTOR).trunc()
    }
}

/// The missile among `shots` that point defence `spec` on `defender`, at
/// `at` with `hull`, engages, if any (`_HandleShipPointDefense`
/// @0x393b1-0x394bd): a homing shot not jammed ([`Jam::Clear`]), that point
/// defence can target (no `Flags` 0x0080), whose damage is aimed at the
/// defender or at its fleet's lead ([`Shot::aimed_at`], shot +0x2a, not
/// its quarry), within the weapon's [`engagement_range`] and not in a
/// blind spot ([`blind_spot`], by the table's bearing to it,
/// `_CalcInterceptAngle` @0x39477). Its squared distance is truncated before the range test
/// and before the nearest test, so a missile under a square pixel past
/// the range is in it, and of two whose squared distances truncate alike
/// the one fired first wins. By the engine (`rule` none) there is no
/// other test; by the other reading
/// ([`RuleKey::PdHostility`](crate::RuleKey::PdHostility)), `rule` must
/// also call the firer hostile, with the relations in `govts`.
///
/// The original truncates the squared distance into 32 bits, so a missile
/// past 2³¹ (about 46,341 pixels) reads in range; here it is not wrapped.
#[must_use]
pub fn choose<'a>(
    defender: Side,
    at: &ShipState,
    hull: &HullSpec,
    spec: &WeaponSpec,
    shots: &'a [Shot],
    rule: Option<&dyn PointDefenceRule>,
    govts: &Governments,
) -> Option<&'a Shot> {
    let range = engagement_range(spec);
    let in_range = range * range;
    let blind_spots = spec.flags | hull.blind_spots;
    let mut best: Option<(&Shot, f32)> = None;
    for shot in shots {
        let off = shot.position - at.position;
        let distance = (off.x * off.x + off.y * off.y).trunc();
        let aimed = shot.aimed_at == Some(defender.ship) || shot.aimed_at == Some(defender.fleet);
        let candidate = shot.weapon.guidance == Guidance::Homing
            && shot.jam == Jam::Clear
            && !shot.weapon.pd_immune()
            && aimed
            && distance <= in_range
            && best.is_none_or(|(_, nearest)| distance < nearest)
            && !blind_spot(
                blind_spots,
                at.heading,
                trig::intercept_angle(at.position, shot.position),
            )
            && rule.is_none_or(|rule| rule.hostile(defender, Side::of(shot), govts));
        if candidate {
            best = Some((shot, distance));
        }
    }
    best.map(|(shot, _)| shot)
}

/// What point defence fires at: a missile, or a ship ([`choose_ship`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Quarry {
    /// The missile numbered so.
    Missile(ShotId),
    /// This ship, as it was when point defence picked it.
    Ship(Target),
}

/// A ship as point defence weighs it ([`choose_ship`]), as it was when
/// the tick's point defence began.
// Each is one of the original's tests, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    /// The ship as shots see it: which, where, how it moves and how it
    /// holds up. A shot fired at it leads it from this.
    pub target: Target,
    /// Whether point defence fires on its class, its `Flags2` 0x0008
    /// ([`HullSpec::pd_target`]).
    pub pd_target: bool,
    /// Whether it is held disabled whatever its armour.
    pub held_disabled: bool,
    /// Whether it is cloaked.
    pub cloaked: bool,
    /// Whether it waits in the gate it comes out of, unseen.
    pub emerging: bool,
    /// The ship it attacks or snipes at, the original's +0x70 in AI state
    /// 4 (`_IsShipFightingMe` @0x827fa); none for the player, whose own
    /// quarry is never read.
    pub attacking: Option<ShipRef>,
}

/// The defender as point defence's ship branch reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guard {
    /// Its ship.
    pub ship: ShipRef,
    /// The ship it follows (the original's +0x9a): the player for a ship
    /// that escorts the player; none for the player.
    pub leader: Option<ShipRef>,
    /// Whether it threatens the player (`_ExtendedIsThreatToPlayer`);
    /// never the player's own.
    pub threatens_player: bool,
    /// Whether it has a cloak scanner on screen
    /// ([`Scanner::on_screen`](crate::cloak::Scanner::on_screen)).
    pub scanner: bool,
}

/// The ship among `candidates` that point defence `spec` on the defender
/// `guard`, at `at` with `hull`, fires at when it finds no missile
/// (`_HandleShipPointDefense` @0x394ce-0x3969d, see the module docs): of
/// a class it fires on, neither the defender nor its leader, intact and
/// not held disabled, seen by it ([`cloak::seen`], with no cloaked-escort
/// clause, which the leader test makes unreachable), within
/// [`engagement_range`], the nearer by a strict `<` of truncated squared
/// distances, not in a blind spot ([`blind_spot`], by the table's bearing
/// to it, `_CalcInterceptAngle` @0x395cb), and fighting it: the player while the
/// defender threatens the player, and any other ship attacking the
/// defender or its leader. Every test is the original's; none asks the
/// [`PointDefenceRule`], under either reading of
/// [`RuleKey::PdHostility`](crate::RuleKey::PdHostility).
///
/// The candidates go in the fight's order, the player and then the NPCs
/// in traffic order, where the original walks its ship slots: the two
/// differ only for ships whose truncated squared distances tie, which
/// crosses no boundary the player sees. A slot the original holds
/// inactive is a destroyed ship here, which the intact test drops too.
/// The squared distance is not wrapped past 2³¹, as for [`choose`].
#[must_use]
pub fn choose_ship<'a>(
    guard: Guard,
    at: &ShipState,
    hull: &HullSpec,
    spec: &WeaponSpec,
    candidates: &'a [Candidate],
) -> Option<&'a Candidate> {
    let range = engagement_range(spec);
    let in_range = range * range;
    let blind_spots = spec.flags | hull.blind_spots;
    let observer = Observer {
        at: at.position,
        scanner: guard.scanner,
        cloaked_escort: false,
    };
    let mut best: Option<(&Candidate, f32)> = None;
    for candidate in candidates {
        let target = &candidate.target;
        let off = target.position - at.position;
        let distance = (off.x * off.x + off.y * off.y).trunc();
        let fighting = if target.ship == ShipRef::Player {
            guard.threatens_player
        } else {
            candidate
                .attacking
                .is_some_and(|ship| ship == guard.ship || Some(ship) == guard.leader)
        };
        let chosen = target.ship != guard.ship
            && candidate.pd_target
            && Some(target.ship) != guard.leader
            && target.condition == Condition::Intact
            && !candidate.held_disabled
            && cloak::seen(
                candidate.cloaked,
                target.position,
                candidate.emerging,
                observer,
            )
            && distance <= in_range
            && best.is_none_or(|(_, nearest)| distance < nearest)
            && !blind_spot(
                blind_spots,
                at.heading,
                trig::intercept_angle(at.position, target.position),
            )
            && fighting;
        if chosen {
            best = Some((candidate, distance));
        }
    }
    best.map(|(candidate, _)| candidate)
}

/// What a hit of point defence `spec` takes off a missile's durability:
/// its mass damage and half its energy damage, truncated.
#[must_use]
pub fn pd_damage(spec: &WeaponSpec) -> f32 {
    spec.mass_damage + (spec.energy_damage / 2.0).trunc()
}

/// A shot's sprite as the original's collision pass sees it, in whole
/// pixels relative to the camera: its frame, placed `_HandleShot`'s way
/// (see the module docs). Each edge is a pixel boundary: a box takes the
/// pixels from `left` up to `right` and from `top` up to `bottom`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpriteBox {
    /// Its left edge.
    pub left: i64,
    /// Its top edge.
    pub top: i64,
    /// Its right edge, its left plus its frame's width.
    pub right: i64,
    /// Its bottom edge, its top plus its frame's height.
    pub bottom: i64,
}

impl SpriteBox {
    /// The box of a sprite of `frame` at `position`, seen from `camera`:
    /// its offset from the camera truncated on each axis, less half its
    /// frame's width, truncated, on both (`_HandleShot` @0x35c48-0x35cc0
    /// reads `_GetSpriteXSize` alone), then its frame's width and height
    /// on (`_SWSetSpriteLocation` @0xab640, a frame's hot spot 0).
    #[must_use]
    pub fn of(position: Vec2, frame: FrameSize, camera: Vec2) -> Self {
        let off = position - camera;
        // A sprite's offset in whole pixels: far within an `i64`, and
        // saturated past it.
        #[allow(clippy::cast_possible_truncation)]
        let (x, y) = (off.x.trunc() as i64, off.y.trunc() as i64);
        let (width, height) = (i64::from(frame.width), i64::from(frame.height));
        let left = x - width / 2;
        let top = y - width / 2;
        Self {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }
    }

    /// Whether it overlaps `other`: strictly on each axis, so two boxes
    /// that only share an edge do not meet (`_SWCollideSpriteLayer`
    /// @0xacc30-0xacc81).
    #[must_use]
    pub fn meets(self, other: Self) -> bool {
        self.top < other.bottom
            && self.bottom > other.top
            && self.left < other.right
            && self.right > other.left
    }
}

/// Whether the sprites of shots `a` and `b` meet, seen from `camera`
/// ([`SpriteBox`]): never for a shot with no sprite ([`WeaponSpec::frame`]).
#[must_use]
pub fn sprites_meet(a: &Shot, b: &Shot, camera: Vec2) -> bool {
    let sprite = |shot: &Shot| {
        shot.weapon
            .frame
            .map(|frame| SpriteBox::of(shot.position, frame, camera))
    };
    sprite(a).zip(sprite(b)).is_some_and(|(a, b)| a.meets(b))
}

impl Shot {
    /// Takes a point-defence hit of `damage`: off its durability while
    /// that is above none, and otherwise it is destroyed. Whether it was.
    pub fn take_pd_hit(&mut self, damage: f32) -> bool {
        if self.durability > 0.0 {
            self.durability -= damage;
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::catalog::{GovtId, WeaponRecord};
    use crate::combat::NpcId;
    use crate::combat::aim::BLIND_REAR;
    use crate::combat::hull::Condition;
    use crate::combat::projectile::{ShotId, Source, Target};
    use crate::combat::sides::Ties;
    use nova_sim_catalog::fixture::weapon;

    const DEFENDER: ShipRef = ShipRef::Npc(NpcId(1));
    const LEAD: ShipRef = ShipRef::Npc(NpcId(9));
    const ENEMY: ShipRef = ShipRef::Npc(NpcId(5));

    /// The Quad Light Blaster Turret: 20 pixels a tick for 12 ticks.
    fn quad(flags: u16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance: 9,
            speed: 2000,
            count: 12,
            mass_dmg: 1,
            energy_dmg: 4,
            flags,
            ..weapon(133)
        })
    }

    #[test]
    fn a_point_defence_turrets_engagement_range_runs_down_its_sub_munition_chain() {
        let flak = WeaponRecord {
            guidance: 9,
            speed: 1050,
            count: 3,
            sub_count: 2,
            sub_type: Some(crate::catalog::WeaponId(162)),
            ..weapon(161)
        };
        let burst = WeaponRecord {
            speed: 1000,
            count: 2,
            ..weapon(162)
        };
        let arsenal = crate::combat::armament::Arsenal::new(&[flak, burst], Vec::new());
        let spec = arsenal.weapon(crate::catalog::WeaponId(161)).expect("read");
        assert_eq!(spec.range(), 51.5, "31.5 and 20");
        assert_eq!(engagement_range(spec), 76.0, "51.5 to 51, then 76.5 to 76");
    }

    #[test]
    fn a_turrets_engagement_range_is_half_again_its_range_and_a_beams_its_length() {
        assert_eq!(engagement_range(&quad(0)), 360.0);
        let storm = WeaponSpec::new(&WeaponRecord {
            guidance: 9,
            speed: 1200,
            count: 15,
            ..weapon(161)
        });
        assert_eq!(engagement_range(&storm), 270.0);
        let odd = WeaponSpec::new(&WeaponRecord {
            guidance: 9,
            speed: 1050,
            count: 3,
            ..weapon(161)
        });
        assert_eq!(engagement_range(&odd), 46.0, "31.5 to 31, then 46.5 to 46");
        let beam = WeaponSpec::new(&WeaponRecord {
            guidance: 10,
            speed: 2000,
            count: 12,
            beam_length: 150,
            ..weapon(150)
        });
        assert_eq!(engagement_range(&beam), 150.0);
        assert_eq!(PD_RANGE_FACTOR, 1.5);
    }

    /// A shot numbered `id` of a homing missile fired by `ENEMY` of
    /// government 140 at `target`, its quarry and the ship its damage is
    /// aimed at, at (`x`, `y`).
    fn missile(id: u32, x: f32, y: f32, target: Option<ShipRef>) -> Shot {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance: 1,
            durability: 4,
            ..weapon(134)
        });
        Shot {
            id: ShotId(id),
            position: Vec2::new(x, y),
            target,
            aimed_at: target,
            source: Source::Ship(Ties {
                govt: Some(GovtId(140)),
                ..Ties::default()
            }),
            ..Shot::launch(spec, ENEMY, ENEMY, &ShipState::default(), 0.0)
        }
    }

    /// Says every firer is `hostile`, recording what it was asked and
    /// the governments it was asked with.
    #[derive(Debug, Default)]
    struct Recording {
        hostile: bool,
        asked: RefCell<Vec<(Side, Side)>>,
        govts: RefCell<Vec<Governments>>,
    }

    impl PointDefenceRule for Recording {
        fn hostile(&self, defender: Side, firer: Side, govts: &Governments) -> bool {
            self.asked.borrow_mut().push((defender, firer));
            self.govts.borrow_mut().push(govts.clone());
            self.hostile
        }
    }

    const ESCORT: Side = Side {
        ship: DEFENDER,
        fleet: LEAD,
        govt: None,
    };

    /// The missile the defender, an escort of `LEAD` at the centre facing
    /// up, picks among `shots` with `quad(flags)` and a hull with
    /// `blind_spots`, by the engine: with no hostility test.
    fn picked(shots: &[Shot], flags: u16, blind_spots: u16) -> Option<ShotId> {
        let hull = HullSpec {
            blind_spots,
            ..HullSpec::default()
        };
        choose(
            ESCORT,
            &ShipState::default(),
            &hull,
            &quad(flags),
            shots,
            None,
            &Governments::default(),
        )
        .map(|shot| shot.id)
    }

    #[test]
    fn a_missile_past_the_range_by_under_a_square_pixel_is_a_candidate() {
        // 360 pixels: 129,600 squared.
        let just_past = missile(1, 0.0, -129_600.5_f32.sqrt(), Some(DEFENDER));
        let off = just_past.position;
        assert!(off.x * off.x + off.y * off.y > 129_600.0, "{off:?}");
        assert_eq!(picked(&[just_past], 0, 0), Some(ShotId(1)), "truncated");
        let past = missile(1, 0.0, -129_601.0_f32.sqrt(), Some(DEFENDER));
        assert_eq!(picked(&[past], 0, 0), None);
    }

    #[test]
    fn missiles_whose_squared_distances_truncate_alike_go_to_the_one_fired_first() {
        let farther = missile(1, 0.0, -10_000.9_f32.sqrt(), Some(DEFENDER));
        let nearer = missile(2, 10_000.1_f32.sqrt(), 0.0, Some(DEFENDER));
        assert_eq!(
            picked(&[farther, nearer], 0, 0),
            Some(ShotId(1)),
            "both 10,000"
        );
        let nearest = missile(3, 9_999.9_f32.sqrt(), 0.0, Some(DEFENDER));
        assert_eq!(picked(&[farther, nearest], 0, 0), Some(ShotId(3)), "9,999");
    }

    #[test]
    fn point_defence_reads_the_ship_a_missiles_damage_is_aimed_at_not_its_quarry() {
        let unsteered = Shot {
            target: None,
            ..missile(1, 0.0, -100.0, Some(DEFENDER))
        };
        assert_eq!(
            picked(&[unsteered], 0, 0),
            Some(ShotId(1)),
            "fired at a ship breaking up"
        );
        let elsewhere = Shot {
            aimed_at: None,
            ..missile(1, 0.0, -100.0, Some(DEFENDER))
        };
        assert_eq!(picked(&[elsewhere], 0, 0), None);
    }

    #[test]
    fn by_the_engine_point_defence_engages_its_own_fleets_missile_aimed_at_it() {
        let own = Shot {
            firer: LEAD,
            fleet: LEAD,
            source: Source::Ship(Ties::default()),
            ..missile(1, 0.0, -100.0, Some(DEFENDER))
        };
        assert_eq!(picked(&[own], 0, 0), Some(ShotId(1)));
        let rule = Recording::default();
        let shots = [own];
        let chosen = choose(
            ESCORT,
            &ShipState::default(),
            &HullSpec::default(),
            &quad(0),
            &shots,
            Some(&rule),
            &Governments::default(),
        );
        assert_eq!(chosen, None, "by the other reading, the rule spares it");
        assert_eq!(rule.asked.take().len(), 1);
    }

    #[test]
    fn a_missile_at_the_defender_or_its_lead_is_a_candidate_and_one_at_another_ship_is_not() {
        let at = |target| [missile(1, 0.0, -100.0, target)];
        assert_eq!(picked(&at(Some(DEFENDER)), 0, 0), Some(ShotId(1)));
        assert_eq!(picked(&at(Some(LEAD)), 0, 0), Some(ShotId(1)), "its lead");
        assert_eq!(picked(&at(Some(ShipRef::Player)), 0, 0), None);
        assert_eq!(picked(&at(None), 0, 0), None, "at nothing");
        assert_eq!(picked(&[], 0, 0), None);
    }

    #[test]
    fn a_lost_untargetable_unguided_far_or_unseen_missile_is_not_a_candidate() {
        let lost = Shot {
            jam: Jam::Lost,
            ..missile(1, 0.0, -100.0, Some(DEFENDER))
        };
        assert_eq!(picked(&[lost], 0, 0), None, "lost");
        let mut immune = missile(1, 0.0, -100.0, Some(DEFENDER));
        immune.weapon.flags = 0x0080;
        assert_eq!(picked(&[immune], 0, 0), None, "0x0080");
        let mut unguided = missile(1, 0.0, -100.0, Some(DEFENDER));
        unguided.weapon.guidance = crate::combat::weapon::Guidance::Turret;
        assert_eq!(picked(&[unguided], 0, 0), None, "not guided");
        let at_range = missile(1, 0.0, -360.0, Some(DEFENDER));
        assert_eq!(picked(&[at_range], 0, 0), Some(ShotId(1)), "at its range");
        let beyond = missile(1, 0.0, 360.1, Some(DEFENDER));
        assert_eq!(picked(&[beyond], 0, 0), None, "beyond its range");
        let astern = [missile(1, 0.0, 100.0, Some(DEFENDER))];
        assert_eq!(
            picked(&astern, BLIND_REAR, 0),
            None,
            "the weapon's blind spot"
        );
        assert_eq!(
            picked(&astern, 0, BLIND_REAR),
            None,
            "the hull's blind spot"
        );
        assert_eq!(picked(&astern, 0, 0), Some(ShotId(1)));
        assert_eq!(
            picked(&astern, BLIND_REAR, BLIND_REAR),
            None,
            "blind both ways"
        );
    }

    #[test]
    fn a_missile_is_measured_from_where_the_defender_is() {
        let at = ShipState {
            position: Vec2::new(1000.0, 1000.0),
            ..ShipState::default()
        };
        let near = [missile(1, 1000.0, 700.0, Some(DEFENDER))];
        let none = Governments::default();
        let chosen = choose(
            ESCORT,
            &at,
            &HullSpec::default(),
            &quad(0),
            &near,
            None,
            &none,
        );
        assert_eq!(
            chosen.map(|shot| shot.id),
            Some(ShotId(1)),
            "300 pixels off"
        );
        let far = [missile(1, 0.0, 0.0, Some(DEFENDER))];
        let chosen = choose(
            ESCORT,
            &at,
            &HullSpec::default(),
            &quad(0),
            &far,
            None,
            &none,
        );
        assert_eq!(chosen, None, "1414 pixels off");
    }

    #[test]
    fn the_nearest_candidate_wins_and_a_tie_goes_to_the_one_fired_first() {
        let shots = [
            missile(1, 0.0, -200.0, Some(DEFENDER)),
            missile(2, 0.0, -50.0, Some(ShipRef::Player)),
            missile(3, 100.0, 0.0, Some(LEAD)),
            missile(4, -100.0, 0.0, Some(DEFENDER)),
        ];
        assert_eq!(picked(&shots, 0, 0), Some(ShotId(3)));
        let mut reversed = shots;
        reversed.reverse();
        assert_eq!(picked(&reversed, 0, 0), Some(ShotId(4)));
    }

    #[test]
    fn by_the_other_reading_the_rule_is_asked_of_each_candidate_and_one_not_hostile_is_skipped() {
        let shots = [
            missile(1, 0.0, -50.0, Some(DEFENDER)),
            Shot {
                firer: ShipRef::Player,
                fleet: ShipRef::Player,
                source: Source::Ship(Ties::default()),
                ..missile(2, 0.0, -80.0, Some(DEFENDER))
            },
            missile(3, 0.0, -1000.0, Some(DEFENDER)),
        ];
        let peaceful = Recording::default();
        let govts = Governments::new([nova_sim_catalog::fixture::govt(128)]);
        let chosen = choose(
            ESCORT,
            &ShipState::default(),
            &HullSpec::default(),
            &quad(0),
            &shots,
            Some(&peaceful),
            &govts,
        );
        assert_eq!(chosen, None);
        let enemy = Side {
            ship: ENEMY,
            fleet: ENEMY,
            govt: Some(GovtId(140)),
        };
        let player = Side {
            ship: ShipRef::Player,
            fleet: ShipRef::Player,
            govt: None,
        };
        assert_eq!(
            peaceful.asked.take(),
            [(ESCORT, enemy), (ESCORT, player)],
            "not of the one out of range"
        );
        assert_eq!(
            peaceful.govts.take(),
            [govts.clone(), govts],
            "asked with them"
        );
    }

    /// Governments 128 and 129 are allies, 130 is at war with 128, and
    /// 131 is neutral.
    fn relations() -> Governments {
        use crate::catalog::GovtRecord;
        use nova_sim_catalog::fixture::govt;
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [2, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..govt(129)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..govt(130)
            },
            govt(131),
        ])
    }

    fn side(ship: ShipRef, fleet: ShipRef, govt: Option<i16>) -> Side {
        Side {
            ship,
            fleet,
            govt: govt.map(GovtId),
        }
    }

    #[test]
    fn allegiance_spares_the_defenders_own_fleet_whatever_the_governments() {
        let govts = relations();
        let defender = side(DEFENDER, LEAD, Some(128));
        for govt in [Some(128), Some(130), None] {
            assert!(
                !Allegiance.hostile(defender, side(LEAD, LEAD, govt), &govts),
                "its lead"
            );
            assert!(
                !Allegiance.hostile(defender, side(ENEMY, LEAD, govt), &govts),
                "another escort"
            );
        }
        assert!(!Allegiance.hostile(defender, defender, &govts), "itself");
    }

    #[test]
    fn allegiance_spares_the_players_escorts_and_the_player_its_escorts() {
        let govts = relations();
        let player = side(ShipRef::Player, ShipRef::Player, None);
        let escort = side(ENEMY, ShipRef::Player, None);
        assert!(
            !Allegiance.hostile(player, escort, &govts),
            "its escort's missile"
        );
        assert!(!Allegiance.hostile(escort, player, &govts), "the player's");
        assert!(
            Allegiance.hostile(escort, side(LEAD, LEAD, None), &govts),
            "another fleet's"
        );
    }

    #[test]
    fn allegiance_spares_an_allied_government_and_its_own() {
        let govts = relations();
        let defender = side(DEFENDER, DEFENDER, Some(128));
        assert!(!Allegiance.hostile(defender, side(ENEMY, ENEMY, Some(129)), &govts));
        assert!(!Allegiance.hostile(defender, side(ENEMY, ENEMY, Some(128)), &govts));
        let ally = side(DEFENDER, DEFENDER, Some(129));
        assert!(!Allegiance.hostile(ally, side(ENEMY, ENEMY, Some(128)), &govts));
    }

    #[test]
    fn allegiance_calls_hostile_an_enemy_a_neutral_an_independent_and_the_player() {
        let govts = relations();
        let defender = side(DEFENDER, DEFENDER, Some(128));
        for govt in [Some(130), Some(131), None] {
            assert!(
                Allegiance.hostile(defender, side(ENEMY, ENEMY, govt), &govts),
                "{govt:?}"
            );
        }
        let player = side(ShipRef::Player, ShipRef::Player, None);
        assert!(Allegiance.hostile(player, side(ENEMY, ENEMY, Some(128)), &govts));
        let independent = side(DEFENDER, DEFENDER, None);
        assert!(
            Allegiance.hostile(independent, player, &govts),
            "two without a government are not allies"
        );
        assert!(Allegiance.hostile(independent, side(ENEMY, ENEMY, None), &govts));
    }

    /// A `width` x `height` frame.
    fn frame(width: u32, height: u32) -> FrameSize {
        FrameSize { width, height }
    }

    #[test]
    fn a_sprite_box_is_its_frame_half_its_width_up_and_left_of_its_truncated_offset() {
        let at = SpriteBox::of(Vec2::new(10.7, 20.2), frame(10, 30), Vec2::ZERO);
        assert_eq!(
            at,
            SpriteBox {
                left: 5,
                top: 15,
                right: 15,
                bottom: 45,
            },
            "half the width up, not half the height"
        );
        let off = SpriteBox::of(Vec2::new(-10.7, -20.2), frame(9, 4), Vec2::new(0.5, 0.5));
        assert_eq!(
            off,
            SpriteBox {
                left: -15,
                top: -24,
                right: -6,
                bottom: -20,
            },
            "-11.2 and -20.7 truncated toward none, less 4 each way"
        );
    }

    /// A box `width` square with its top left at (`left`, `top`).
    fn square(left: i64, top: i64, width: i64) -> SpriteBox {
        SpriteBox {
            left,
            top,
            right: left + width,
            bottom: top + width,
        }
    }

    #[test]
    fn two_boxes_meet_when_they_overlap_and_not_when_they_only_share_an_edge() {
        let at = square(0, 0, 10);
        assert!(at.meets(square(5, 5, 10)));
        assert!(at.meets(square(-9, -9, 10)), "by a pixel up and left");
        assert!(at.meets(square(9, 9, 10)), "by a pixel down and right");
        assert!(at.meets(square(2, 2, 2)), "within");
        for (left, top) in [(10, 0), (-10, 0), (0, 10), (0, -10), (10, 10)] {
            let beside = square(left, top, 10);
            assert!(!at.meets(beside), "({left}, {top})");
            assert!(!beside.meets(at), "({left}, {top}), the other way");
        }
    }

    /// A shot of `frame` at (`x`, 0).
    fn drawn(x: f32, frame: Option<FrameSize>) -> Shot {
        let mut shot = missile(1, x, 0.0, None);
        shot.weapon.frame = frame;
        shot
    }

    #[test]
    fn two_sprites_meet_by_their_offsets_from_the_camera_truncated() {
        let a = drawn(0.5, Some(frame(10, 10)));
        let b = drawn(10.4, Some(frame(10, 10)));
        assert!(!sprites_meet(&a, &b, Vec2::ZERO), "0 and 10: an edge apart");
        assert!(
            sprites_meet(&a, &b, Vec2::new(0.6, 0.0)),
            "0 (-0.1) and 9 (9.8): overlapping"
        );
        assert!(
            sprites_meet(&a, &b, Vec2::new(5.0, 0.0)),
            "-4 (-4.5) and 5 (5.4): toward none, not down"
        );
    }

    #[test]
    fn a_shot_with_no_sprite_never_meets() {
        let drawn_at = |x| drawn(x, Some(frame(10, 10)));
        let bare = drawn(0.0, None);
        assert!(sprites_meet(&drawn_at(0.0), &drawn_at(1.0), Vec2::ZERO));
        assert!(!sprites_meet(&bare, &drawn_at(1.0), Vec2::ZERO));
        assert!(!sprites_meet(&drawn_at(1.0), &bare, Vec2::ZERO));
        assert!(!sprites_meet(&bare, &bare, Vec2::ZERO));
    }

    #[test]
    fn a_hit_does_its_mass_and_half_its_energy_damage_truncated() {
        assert_eq!(pd_damage(&quad(0)), 3.0);
        let odd = WeaponSpec::new(&WeaponRecord {
            mass_dmg: 3,
            energy_dmg: 2 * 2 + 1,
            ..weapon(161)
        });
        assert_eq!(pd_damage(&odd), 5.0);
    }

    /// The hits of `damage` that destroy a missile of `durability`.
    fn hits_to_destroy(durability: f32, damage: f32) -> u32 {
        let mut shot = Shot {
            durability,
            ..missile(1, 0.0, 0.0, None)
        };
        let mut hits = 1;
        while !shot.take_pd_hit(damage) {
            hits += 1;
            assert!(hits < 100, "never destroyed");
        }
        hits
    }

    #[test]
    fn a_missile_is_destroyed_by_the_hit_after_its_durability_is_used_up() {
        assert_eq!(hits_to_destroy(0.0, 3.0), 1);
        assert_eq!(hits_to_destroy(4.0, 3.0), 3, "4, 1, -2, then gone");
        assert_eq!(hits_to_destroy(3.0, 3.0), 2, "3, 0, then gone");
        assert_eq!(hits_to_destroy(4.0, 1.0), 5);
        let mut shot = Shot {
            durability: 4.0,
            ..missile(1, 0.0, 0.0, None)
        };
        assert!(!shot.take_pd_hit(3.0));
        assert_eq!(shot.durability, 1.0);
    }

    const FELLOW: ShipRef = ShipRef::Npc(NpcId(6));

    /// A ship at (`x`, `y`), at rest, of a class point defence fires on,
    /// intact, seen and attacking `attacking`.
    fn fighter(ship: ShipRef, x: f32, y: f32, attacking: Option<ShipRef>) -> Candidate {
        Candidate {
            target: Target {
                ship,
                ship_type: crate::catalog::ShipId(167),
                fleet: ship,
                ties: Ties::default(),
                position: Vec2::new(x, y),
                velocity: Vec2::ZERO,
                radius: 10.0,
                sprite_x: 30.0,
                sheet: true,
                condition: Condition::Intact,
                miss_class: 80,
            },
            pd_target: true,
            held_disabled: false,
            cloaked: false,
            emerging: false,
            attacking,
        }
    }

    /// A ship at (`x`, `y`) attacking the defender.
    fn attacker(ship: ShipRef, x: f32, y: f32) -> Candidate {
        fighter(ship, x, y, Some(DEFENDER))
    }

    /// The defender, led by no one, no threat to the player, with no
    /// cloak scanner.
    const GUARD: Guard = Guard {
        ship: DEFENDER,
        leader: None,
        threatens_player: false,
        scanner: false,
    };

    /// The ship `guard`, at the centre facing up, picks among
    /// `candidates` with `spec` and a hull with `blind_spots`.
    fn ship_picked_by(
        guard: Guard,
        spec: &WeaponSpec,
        blind_spots: u16,
        candidates: &[Candidate],
    ) -> Option<ShipRef> {
        let hull = HullSpec {
            blind_spots,
            ..HullSpec::default()
        };
        choose_ship(guard, &ShipState::default(), &hull, spec, candidates)
            .map(|candidate| candidate.target.ship)
    }

    /// The ship `guard` picks among `candidates` with a Quad.
    fn ship_picked(guard: Guard, candidates: &[Candidate]) -> Option<ShipRef> {
        ship_picked_by(guard, &quad(0), 0, candidates)
    }

    #[test]
    fn point_defence_picks_a_ship_of_its_class_attacking_the_defender_in_range() {
        assert_eq!(
            ship_picked(GUARD, &[attacker(ENEMY, 0.0, -100.0)]),
            Some(ENEMY)
        );
        assert_eq!(ship_picked(GUARD, &[]), None);
    }

    #[test]
    fn point_defence_picks_the_ship_nearer_its_defender_wherever_the_defender_is() {
        let away = ShipState {
            position: Vec2::new(500.0, 500.0),
            ..ShipState::default()
        };
        let picked = |candidates: &[Candidate]| {
            choose_ship(GUARD, &away, &HullSpec::default(), &quad(0), candidates)
                .map(|candidate| candidate.target.ship)
        };
        let beside = attacker(ENEMY, 600.0, 500.0);
        let below = attacker(LEAD, 500.0, 590.0);
        assert_eq!(picked(&[beside, below]), Some(LEAD), "90 off, not 100");
        let farther = attacker(LEAD, 500.0, 610.0);
        assert_eq!(picked(&[beside, farther]), Some(ENEMY), "100 off, not 110");
    }

    #[test]
    fn point_defence_never_picks_its_own_ship() {
        assert_eq!(ship_picked(GUARD, &[attacker(DEFENDER, 0.0, 0.0)]), None);
    }

    #[test]
    fn point_defence_picks_no_ship_of_a_class_without_flags2_0x0008() {
        let unmarked = Candidate {
            pd_target: false,
            ..attacker(ENEMY, 0.0, -100.0)
        };
        assert_eq!(ship_picked(GUARD, &[unmarked]), None);
    }

    #[test]
    fn point_defence_never_picks_the_defenders_leader() {
        let escort = Guard {
            leader: Some(LEAD),
            ..GUARD
        };
        assert_eq!(ship_picked(escort, &[attacker(LEAD, 0.0, -100.0)]), None);
    }

    #[test]
    fn point_defence_picks_no_ship_disabled_held_disabled_breaking_up_or_destroyed() {
        let dying = Condition::Dying {
            ticks_left: 10,
            starting: false,
        };
        for condition in [Condition::Disabled, dying, Condition::Destroyed] {
            let mut down = attacker(ENEMY, 0.0, -100.0);
            down.target.condition = condition;
            assert_eq!(ship_picked(GUARD, &[down]), None, "{condition:?}");
        }
        let held = Candidate {
            held_disabled: true,
            ..attacker(ENEMY, 0.0, -100.0)
        };
        assert_eq!(ship_picked(GUARD, &[held]), None, "held disabled");
    }

    #[test]
    fn point_defence_picks_a_cloaked_ship_only_with_a_scanner_within_200_on_each_axis() {
        let cloaked = |x, y| Candidate {
            cloaked: true,
            ..attacker(ENEMY, x, y)
        };
        let scanning = Guard {
            scanner: true,
            ..GUARD
        };
        assert_eq!(ship_picked(GUARD, &[cloaked(0.0, -100.0)]), None);
        assert_eq!(ship_picked(scanning, &[cloaked(200.0, 200.0)]), Some(ENEMY));
        assert_eq!(ship_picked(scanning, &[cloaked(201.0, 0.0)]), None);
        assert_eq!(ship_picked(scanning, &[cloaked(0.0, -201.0)]), None);
        let waiting = Candidate {
            emerging: true,
            ..attacker(ENEMY, 0.0, -100.0)
        };
        assert_eq!(ship_picked(scanning, &[waiting]), None, "in its gate");
    }

    #[test]
    fn a_ship_past_the_range_by_under_a_square_pixel_is_picked_and_one_a_pixel_past_is_not() {
        // 360 pixels: 129,600 squared.
        let y = -129_600.5_f32.sqrt();
        assert!(y * y > 129_600.0);
        assert_eq!(ship_picked(GUARD, &[attacker(ENEMY, 0.0, y)]), Some(ENEMY));
        assert_eq!(ship_picked(GUARD, &[attacker(ENEMY, 0.0, -361.0)]), None);
        let beam = WeaponSpec::new(&WeaponRecord {
            guidance: 10,
            beam_length: 150,
            ..weapon(150)
        });
        let by_beam = |y| ship_picked_by(GUARD, &beam, 0, &[attacker(ENEMY, 0.0, y)]);
        assert_eq!(by_beam(-150.0), Some(ENEMY), "a beam's length");
        assert_eq!(by_beam(-151.0), None);
    }

    #[test]
    fn the_nearest_ship_wins_and_of_two_alike_once_truncated_the_first() {
        let near = attacker(ENEMY, 0.0, -100.0);
        let far = attacker(FELLOW, 100.0, -100.0);
        assert_eq!(ship_picked(GUARD, &[far, near]), Some(ENEMY));
        assert_eq!(ship_picked(GUARD, &[near, far]), Some(ENEMY));
        // 10,000.2 and 10,000.7 squared both truncate to 10,000.
        let first = attacker(FELLOW, 0.0, -10_000.2_f32.sqrt());
        let second = attacker(ENEMY, 0.0, 10_000.7_f32.sqrt());
        assert_eq!(ship_picked(GUARD, &[first, second]), Some(FELLOW));
        assert_eq!(ship_picked(GUARD, &[second, first]), Some(ENEMY));
    }

    #[test]
    fn point_defence_picks_no_ship_in_its_weapons_or_its_hulls_blind_spot() {
        let behind = [attacker(ENEMY, 0.0, 100.0)];
        assert_eq!(ship_picked_by(GUARD, &quad(0), 0, &behind), Some(ENEMY));
        assert_eq!(ship_picked_by(GUARD, &quad(BLIND_REAR), 0, &behind), None);
        assert_eq!(ship_picked_by(GUARD, &quad(0), BLIND_REAR, &behind), None);
        assert_eq!(
            ship_picked_by(GUARD, &quad(BLIND_REAR), BLIND_REAR, &behind),
            None,
            "both"
        );
        let ahead = [attacker(ENEMY, 0.0, -100.0)];
        assert_eq!(
            ship_picked_by(GUARD, &quad(BLIND_REAR), BLIND_REAR, &ahead),
            Some(ENEMY)
        );
    }

    #[test]
    fn point_defence_picks_a_ship_only_while_it_attacks_the_defender_or_its_leader() {
        let threat = Guard {
            threatens_player: true,
            ..GUARD
        };
        for attacking in [None, Some(FELLOW), Some(ShipRef::Player)] {
            let elsewhere = fighter(ENEMY, 0.0, -100.0, attacking);
            assert_eq!(ship_picked(threat, &[elsewhere]), None, "{attacking:?}");
        }
    }

    #[test]
    fn an_escort_picks_a_ship_attacking_its_leader_but_not_one_attacking_a_fellow_escort() {
        let escort = Guard {
            leader: Some(LEAD),
            ..GUARD
        };
        let at = |attacking| [fighter(ENEMY, 0.0, -100.0, Some(attacking))];
        assert_eq!(ship_picked(escort, &at(LEAD)), Some(ENEMY));
        assert_eq!(ship_picked(escort, &at(DEFENDER)), Some(ENEMY));
        assert_eq!(ship_picked(escort, &at(FELLOW)), None, "a fellow escort");
        let players = Guard {
            leader: Some(ShipRef::Player),
            ..GUARD
        };
        assert_eq!(
            ship_picked(players, &at(ShipRef::Player)),
            Some(ENEMY),
            "a player's escort"
        );
    }

    #[test]
    fn an_npc_picks_the_player_only_while_it_threatens_the_player() {
        let threat = Guard {
            threatens_player: true,
            ..GUARD
        };
        let player = |attacking| [fighter(ShipRef::Player, 0.0, -100.0, attacking)];
        assert_eq!(ship_picked(threat, &player(None)), Some(ShipRef::Player));
        assert_eq!(ship_picked(GUARD, &player(None)), None);
        assert_eq!(
            ship_picked(GUARD, &player(Some(DEFENDER))),
            None,
            "whatever the player's own quarry"
        );
        let unmarked = Candidate {
            pd_target: false,
            ..player(None)[0]
        };
        assert_eq!(ship_picked(threat, &[unmarked]), None, "its class too");
    }

    #[test]
    fn the_players_point_defence_picks_a_ship_attacking_the_player_but_not_its_escort() {
        let player = Guard {
            ship: ShipRef::Player,
            ..GUARD
        };
        let at = |attacking| [fighter(ENEMY, 0.0, -100.0, Some(attacking))];
        assert_eq!(ship_picked(player, &at(ShipRef::Player)), Some(ENEMY));
        assert_eq!(ship_picked(player, &at(DEFENDER)), None, "its escort");
    }
}
