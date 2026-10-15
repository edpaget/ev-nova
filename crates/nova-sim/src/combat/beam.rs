//! Beams: fired from a ship, held on it for their life, and hitting the
//! nearest ship along them every tick.
//!
//! The original (`_SpawnBeam` and `_HandleBeams` in the `EV Nova`
//! executable) keeps a beam for its weapon's `Count` ticks, following its
//! firer, and every tick damages the nearest ship it may hit within its
//! `BeamLength` plus the ship's hit radius, inside a cone about its
//! heading. The simulation keeps that reach and swaps the cone for the
//! beam's segment against each ship's circle: deterministic, and what the
//! drawn beam shows. Which ships it may hit is `_HandleBeams`' own filter
//! ([`Sides::beam_can_hit`]), not a shot's: no government test, and a
//! breaking-up ship is hit, so a beam stops on a hulk.
//!
//! A beam lives its weapon's `Count` ticks, and a decaying one (`Decay`
//! above none) fades 15 − `Falloff` ticks more, hitting all the while
//! ([`Beam::live`]). Its life is spent first in the tick, so it is still
//! drawn on the tick of its last hit, and gone, with no hit, the next.
//!
//! A beam starts at the exit point it left from (see
//! [`exits`](super::exits)) on the frame its firer is drawn on that tick,
//! as `_HandleBeams` places it after every ship is drawn
//! (@0x3076a-0x30922): at its firer's centre when it has none. A beam (0)
//! points along that frame's angle (@0x303bb-0x3042e). A turreted beam
//! (3) points at its target's bearing from its start, a whole degree,
//! every tick (@0x30432-0x306bb), while the target is active, breaking up
//! included, and ends once it is destroyed or gone. A point-defence beam
//! (10) is held on its missile the same way, ends once the missile is
//! gone, and hits no ship. Any but a point-defence beam then turns by its
//! jitter, `Rand(2i) − i` degrees for its `Inaccuracy` i, drawn anew every
//! tick (@0x306ec-0x30727). Nothing ends a beam for its firer's sake: it
//! follows a disabled or breaking-up firer, and once the firer is
//! destroyed or out of the fight it stays where the firer last was, as
//! the original's inactive slot stays put, until its life ends.

use super::ShipRef;
use super::armament::inaccuracy;
use super::exits::{Compression, ExitKind, ExitPoints, drawn_frame, frame_angle};
use super::projectile::{Shot, ShotId, Target, contact};
use super::sides::{Sides, Ties};
use super::weapon::{Guidance, WeaponSpec};
use crate::chance::Chance;
use crate::flight::{ShipState, facing};
use crate::geometry::Vec2;
use crate::trig::intercept_angle;

/// The most a decaying beam's fade plus its `Falloff` may come to for it
/// to fade a tick more (`_HandleBeams` @0x30352).
pub const FADE_END: i64 = 15;

/// What a beam is held on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aiming {
    /// The frame its firer is drawn on: a beam (0).
    Ahead,
    /// This ship's bearing: a turreted beam (3).
    Ship(ShipRef),
    /// This shot's bearing: a point-defence beam (10).
    Shot(ShotId),
}

/// A beam, held on its firer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beam {
    /// Its weapon.
    pub weapon: WeaponSpec,
    /// The ship that fired it.
    pub firer: ShipRef,
    /// The firer's fleet.
    pub fleet: ShipRef,
    /// A copy of the firer's ties as it fired, which the hit rules read
    /// should the firer leave the fight (see [`sides`](super::sides)).
    pub ties: Ties,
    /// What it is held on.
    pub aiming: Aiming,
    /// Where it starts: its firer's exit, or where its firer last was.
    pub start: Vec2,
    /// The angle, in degrees, of the frame its firer was drawn on when
    /// last seen (the original's -0x48, `_HandleBeams` @0x303bb-0x3042e):
    /// whole degrees, or the firer's heading itself for a hull with no
    /// frames. A beam held [`Aiming::Ahead`] points along it.
    pub frame: f32,
    /// The heading it points along this tick, in degrees: its aim, turned
    /// by its jitter and never wrapped, so it may lie outside 0-360.
    pub heading: f32,
    /// Where it ends: its length along its heading, or where it hit.
    pub end: Vec2,
    /// The ticks of life it has left (the original's beam +0x10): its
    /// weapon's `Count` at launch, spent a tick at a time ([`Beam::live`]).
    pub life: u32,
    /// The ticks it has faded at the end of its life (beam +0x12), which
    /// the view draws its corona or its lightning by.
    pub fade: u32,
    /// The ship it was fired at, if any (the original's beam +0x18,
    /// `_SpawnBeam` @0x45253: its firer's target as it fired), which
    /// decides whether a hit is intended
    /// ([`Strike::intended`](super::Strike::intended)).
    pub aimed_at: Option<ShipRef>,
    /// Whether it never destroys the ships it hits (the original's beam
    /// +0x20): its weapon's [`disable_only`](WeaponSpec::disable_only),
    /// set at launch (`_SpawnBeam` @0x45260).
    pub disable_only: bool,
    /// The set of its firer's exit points it left from, its weapon's
    /// `ExitType` (the original's beam +0x1c); none for the centre.
    pub kind: Option<ExitKind>,
    /// The exit of that set it left from (beam +0x1a,
    /// [`ExitCursors::beam_exit`](super::exits::ExitCursors::beam_exit));
    /// none for the centre.
    pub exit: Option<usize>,
}

impl Beam {
    /// A beam of `weapon` fired by `firer` of `fleet`, flying `from`, held
    /// on what `aiming` says. It points along its firer's heading from its
    /// centre until it first follows ([`Beam::follow`]): `_SpawnBeam`
    /// never reads the angle it is given, so a beam keeps no launch
    /// heading and no launch inaccuracy.
    #[must_use]
    pub fn launch(
        weapon: WeaponSpec,
        firer: ShipRef,
        fleet: ShipRef,
        from: &ShipState,
        aiming: Aiming,
    ) -> Self {
        Self {
            weapon,
            firer,
            fleet,
            ties: Ties::default(),
            aiming,
            start: from.position,
            frame: from.heading,
            heading: from.heading,
            end: from.position + facing(from.heading) * weapon.beam_length,
            life: weapon.lifetime,
            fade: 0,
            aimed_at: None,
            disable_only: weapon.disable_only(),
            kind: None,
            exit: None,
        }
    }

    /// Puts the beam on its `firer`, flying as it is with its exit points,
    /// at its exit, compressed as `compression` says, and points it at
    /// what it aims at among `targets` and `shots`, turned by its jitter
    /// drawn on `chance`, as `_HandleBeams` does every tick
    /// (@0x303bb-0x30922):
    ///
    /// - Its firer gone (none: destroyed, or out of the fight), it stays
    ///   where the firer last was, with that frame: the original's slot
    ///   keeps its place and frame once inactive.
    /// - A beam (0) points along the frame its firer is drawn on. A
    ///   turreted beam (3) points at its target's bearing from its start,
    ///   a whole degree (`_CalcInterceptAngle`), while the target is
    ///   active, breaking up included; a point-defence beam (10) at its
    ///   missile's while the missile is in flight. Either gone, the beam
    ///   has ended: false, and it is left as it was.
    /// - Any but a point-defence beam then turns by `Rand(2i) − i` degrees
    ///   for its `Inaccuracy` i above none (@0x306ec-0x30727), about its
    ///   start, the sum never wrapped.
    pub fn follow(
        &mut self,
        firer: Option<(&ShipState, Option<&ExitPoints>)>,
        compression: Compression,
        targets: &[Target],
        shots: &[Shot],
        chance: &mut dyn Chance,
    ) -> bool {
        if let Some((at, exits)) = firer {
            self.frame = exits.map_or(at.heading, |exits| {
                let frames = exits.frames();
                frame_angle(drawn_frame(at.heading, frames), frames)
            });
            self.start = at.position + self.exit_offset(exits, compression);
        }
        let toward = match self.aiming {
            Aiming::Ahead => None,
            Aiming::Ship(ship) => {
                let target = targets
                    .iter()
                    .find(|target| target.ship == ship && target.condition.active());
                match target {
                    Some(target) => Some(target.position),
                    None => return false,
                }
            }
            Aiming::Shot(id) => match shots.iter().find(|shot| shot.id == id) {
                Some(shot) => Some(shot.position),
                None => return false,
            },
        };
        let aim = toward.map_or(self.frame, |toward| {
            f32::from(intercept_angle(self.start, toward))
        });
        let jitter = if self.weapon.guidance == Guidance::PointDefenceBeam {
            0.0
        } else {
            inaccuracy(self.weapon.inaccuracy, chance)
        };
        self.heading = aim + jitter;
        self.end = self.start + facing(self.heading) * self.weapon.beam_length;
        true
    }

    /// How far off its firer's centre it starts: its exit among `exits`
    /// on the frame the firer is drawn on, and none without an exit
    /// (@0x30775-0x3079b).
    fn exit_offset(&self, exits: Option<&ExitPoints>, compression: Compression) -> Vec2 {
        let (Some(exits), Some(kind), Some(exit)) = (exits, self.kind, self.exit) else {
            return Vec2::ZERO;
        };
        exits.offset(self.frame, kind, exit, compression)
    }

    /// Whether it can hit `target`, among the fight's `sides`
    /// ([`Sides::beam_can_hit`]).
    #[must_use]
    pub fn can_hit(&self, target: &Target, sides: &Sides) -> bool {
        sides.beam_can_hit(self, target)
    }

    /// The nearest of the ships among `sides` it can hit that it reaches,
    /// and where it meets it.
    #[must_use]
    pub fn hit(&self, sides: &Sides) -> Option<(ShipRef, Vec2)> {
        if let Aiming::Shot(_) = self.aiming {
            return None;
        }
        sides
            .targets
            .iter()
            .filter(|target| self.can_hit(target, sides))
            .filter_map(|target| {
                let t = contact(self.start, self.end, target.position, target.radius)?;
                Some((t, target.ship))
            })
            .min_by(|(a, _), (b, _)| a.total_cmp(b))
            .map(|(t, ship)| (ship, self.start + (self.end - self.start) * t))
    }

    /// Spends a tick of its life, as `_HandleBeams` does before the beam
    /// follows and hits (@0x30315-0x30388): a tick off its life while any
    /// is left; at none left, a decaying beam (its `Decay` above none)
    /// fades a tick instead while its fade plus its loaded `Falloff`
    /// ([`WeaponSpec::falloff`]) is at most [`FADE_END`]; and otherwise it
    /// has ended: false, and it neither hits nor is drawn again. So a beam
    /// of `Count` n hits on n ticks, and a decaying one on 15 − `Falloff`
    /// more, each at full damage.
    ///
    /// The second pass of the original's double-time frame (main's
    /// `GamePass`, not on this branch) fades but spends only a life above
    /// none (@0x30371-0x30383).
    pub fn live(&mut self) -> bool {
        if self.life == 0 && self.weapon.decay > 0 {
            self.fade += 1;
            if i64::from(self.weapon.falloff) + i64::from(self.fade) <= FADE_END {
                return true;
            }
        }
        match self.life.checked_sub(1) {
            Some(life) => {
                self.life = life;
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::catalog::{GovtId, GovtRecord, ShipId};
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::combat::projectile::ShotId;
    use crate::combat::sides::PassThrough;
    use crate::govt::Governments;
    use crate::testkit::{Draws, among, govt, weapon};
    use crate::traffic::npc::NpcId;

    const FIRER: ShipRef = ShipRef::Npc(NpcId(1));

    /// A 100-pixel beam lasting 3 ticks.
    fn laser_record() -> WeaponRecord {
        WeaponRecord {
            guidance: 0,
            count: 3,
            beam_length: 100,
            ..weapon(146)
        }
    }

    fn laser() -> WeaponSpec {
        WeaponSpec::new(&laser_record())
    }

    fn at(x: f32, y: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(7.0, 7.0),
            heading,
        }
    }

    /// Has `beam` follow its firer flying `at`, with no exits, among
    /// `targets` and `shots`, drawing no jitter.
    fn follow(beam: &mut Beam, at: ShipState, targets: &[Target], shots: &[Shot]) -> bool {
        beam.follow(
            Some((&at, None)),
            Compression::default(),
            targets,
            shots,
            &mut NeverFires,
        )
    }

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn a_beam_is_disable_only_as_its_weapon_is_and_aimed_at_nothing_until_told() {
        for (flags2, disable_only) in [(0x1000, true), (0, false)] {
            let weapon = WeaponSpec::new(&WeaponRecord {
                flags2,
                ..laser_record()
            });
            let beam = Beam::launch(weapon, FIRER, FIRER, &at(0.0, 0.0, 90.0), Aiming::Ahead);
            assert_eq!(beam.disable_only, disable_only, "{flags2:#x}");
            assert_eq!(beam.aimed_at, None);
        }
    }

    #[test]
    fn a_beam_follows_its_firer_as_it_moves_and_turns_and_stays_once_it_is_gone() {
        let mut beam = Beam::launch(laser(), FIRER, FIRER, &at(10.0, 20.0, 90.0), Aiming::Ahead);
        assert_eq!(beam.start, Vec2::new(10.0, 20.0));
        assert!(close(beam.end, Vec2::new(110.0, 20.0)), "{beam:?}");
        assert!(follow(&mut beam, at(-5.0, 0.0, 180.0), &[], &[]));
        assert_eq!(beam.start, Vec2::new(-5.0, 0.0));
        assert!(close(beam.end, Vec2::new(-5.0, 100.0)), "{beam:?}");
        let left = beam;
        assert!(beam.follow(None, Compression::default(), &[], &[], &mut NeverFires));
        assert_eq!(beam, left, "where its firer last was");
    }

    /// Beam exits on 36 frames: exit 1 30 pixels to the right of the
    /// centre, the rest on it.
    fn beam_exits() -> ExitPoints {
        let raw = crate::catalog::ShipExits {
            points: [
                [(0, 0); 4],
                [(0, 0); 4],
                [(0, 0); 4],
                [(0, 0), (30, 0), (0, 0), (0, 0)],
            ],
            up_compress: (100, 100),
            down_compress: (100, 100),
            frames: 36,
            ..crate::catalog::ShipExits::default()
        };
        ExitPoints::new(&raw).expect("exits")
    }

    #[test]
    fn a_fixed_beam_points_along_its_firers_drawn_frame() {
        let exits = beam_exits();
        let mut beam = Beam::launch(laser(), FIRER, FIRER, &at(0.0, 0.0, 350.0), Aiming::Ahead);
        let mut heading = |at: ShipState, exits| {
            let firer = Some((&at, exits));
            assert!(beam.follow(firer, Compression::default(), &[], &[], &mut NeverFires));
            assert!(
                close(beam.end, at.position + facing(beam.heading) * 100.0),
                "{beam:?}"
            );
            (beam.frame, beam.heading)
        };
        assert_eq!(heading(at(0.0, 0.0, 99.0), Some(&exits)), (90.0, 90.0));
        assert_eq!(heading(at(5.0, 0.0, 359.0), Some(&exits)), (350.0, 350.0));
        assert_eq!(
            heading(at(0.0, 0.0, 99.0), None),
            (99.0, 99.0),
            "a hull with no frames"
        );
        assert_eq!(
            heading(at(0.0, 0.0, 0.0), None),
            (0.0, 0.0),
            "no launch offset"
        );
    }

    #[test]
    fn a_beam_jitters_each_tick_by_its_inaccuracy_about_its_start() {
        let exits = beam_exits();
        let shaky = WeaponSpec::new(&WeaponRecord {
            inaccuracy: 2,
            ..laser_record()
        });
        let mut beam = Beam {
            kind: Some(ExitKind::Beam),
            exit: Some(1),
            ..Beam::launch(shaky, FIRER, FIRER, &at(0.0, 0.0, 0.0), Aiming::Ahead)
        };
        let mut draws = Draws::of(&[0, 3, 1]);
        let mut ticks = Vec::new();
        for _ in 0..3 {
            let firer = Some((&at(100.0, 100.0, 99.0), Some(&exits)));
            assert!(beam.follow(firer, Compression::ByOffset, &[], &[], &mut draws));
            assert!(close(beam.start, Vec2::new(100.0, 130.0)), "{beam:?}");
            assert!(close(beam.end, beam.start + facing(beam.heading) * 100.0));
            ticks.push(beam.heading);
        }
        assert_eq!(ticks, [88.0, 91.0, 89.0], "frame 9's 90, turned");
        assert_eq!(draws.asked, [4, 4, 4]);
        let mut draws = Draws::of(&[]);
        let mut steady = lasting(3, 0, 0, 0);
        let firer = Some((&at(0.0, 0.0, 0.0), None));
        assert!(steady.follow(firer, Compression::default(), &[], &[], &mut draws));
        assert!(draws.asked.is_empty(), "no inaccuracy, no draw");
    }

    #[test]
    fn a_turreted_beam_jitters_about_its_bearing_unwrapped() {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance: 3,
            inaccuracy: 4,
            ..laser_record()
        });
        let mut beam = Beam::launch(
            spec,
            FIRER,
            FIRER,
            &at(0.0, 0.0, 90.0),
            Aiming::Ship(TARGET),
        );
        let mut heading = |draw| {
            let firer = Some((&at(0.0, 0.0, 90.0), None));
            let targets = [target(2, 0.0, -50.0)];
            let mut draws = Draws::of(&[draw]);
            assert!(beam.follow(firer, Compression::default(), &targets, &[], &mut draws));
            assert_eq!(draws.asked, [8]);
            beam.heading
        };
        assert_eq!(heading(0), -4.0);
        assert_eq!(heading(7), 3.0);
    }

    #[test]
    fn a_point_defence_beam_never_jitters() {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance: 10,
            inaccuracy: 4,
            ..laser_record()
        });
        let mut beam = Beam::launch(
            spec,
            FIRER,
            FIRER,
            &at(0.0, 0.0, 0.0),
            Aiming::Shot(ShotId(5)),
        );
        let mut draws = Draws::of(&[]);
        let shots = [missile(5, 40.0, 0.0)];
        let firer = Some((&at(0.0, 0.0, 0.0), None));
        assert!(beam.follow(firer, Compression::default(), &[], &shots, &mut draws));
        assert!(draws.asked.is_empty());
        assert_eq!(beam.heading, 90.0);
    }

    /// A beam lasting `count` ticks, of `decay`, `falloff` and
    /// `li_density`.
    fn lasting(count: i16, decay: i16, falloff: i16, li_density: i16) -> Beam {
        let weapon = WeaponSpec::new(&WeaponRecord {
            count,
            decay,
            falloff,
            li_density,
            ..laser_record()
        });
        Beam::launch(weapon, FIRER, FIRER, &at(0.0, 0.0, 90.0), Aiming::Ahead)
    }

    /// The fade on each tick `beam` lives, until it ends.
    fn lives(mut beam: Beam) -> Vec<u32> {
        let mut fades = Vec::new();
        while beam.live() {
            fades.push(beam.fade);
            assert!(fades.len() < 100, "lives for ever");
        }
        fades
    }

    #[test]
    fn a_beam_of_decay_hits_15_less_falloff_ticks_past_its_count() {
        assert_eq!(lives(lasting(3, 0, 10, 0)), [0, 0, 0], "no decay");
        assert_eq!(lives(lasting(3, 1, 10, 0)), [0, 0, 0, 1, 2, 3, 4, 5]);
        assert_eq!(lives(lasting(3, 1, 14, 0)), [0, 0, 0, 1]);
        assert_eq!(lives(lasting(3, 1, 15, 0)), [0, 0, 0], "falloff 15");
        assert_eq!(lives(lasting(3, 1, 20, 0)), [0, 0, 0]);
        assert_eq!(lives(lasting(3, 1, 0, 0)), [0, 0, 0], "falloff 0 loads 16");
        assert_eq!(lives(lasting(3, -1, 10, 0)), [0, 0, 0], "decay below none");
        let lightning: Vec<u32> = [0, 0].into_iter().chain(1..=15).collect();
        assert_eq!(lives(lasting(2, 1, 8, 4)), lightning, "of no falloff");
        assert!(lives(lasting(0, 0, 10, 0)).is_empty(), "a count of none");
        assert_eq!(lives(lasting(0, 1, 13, 0)), [1, 2]);
    }

    #[test]
    fn a_beam_starts_at_its_exit_on_the_frame_its_firer_is_drawn_on() {
        let exits = beam_exits();
        let mut beam = Beam {
            kind: Some(ExitKind::Beam),
            exit: Some(1),
            ..Beam::launch(laser(), FIRER, FIRER, &at(0.0, 0.0, 0.0), Aiming::Ahead)
        };
        let mut start = |at: ShipState, exits| {
            assert!(beam.follow(
                Some((&at, exits)),
                Compression::ByOffset,
                &[],
                &[],
                &mut NeverFires
            ));
            beam.start
        };
        assert!(close(
            start(at(100.0, 100.0, 0.0), Some(&exits)),
            Vec2::new(130.0, 100.0)
        ));
        assert!(
            close(
                start(at(100.0, 100.0, 99.0), Some(&exits)),
                Vec2::new(100.0, 130.0)
            ),
            "on frame 9"
        );
        assert_eq!(
            start(at(100.0, 100.0, 99.0), None),
            Vec2::new(100.0, 100.0),
            "no exits"
        );
    }

    fn target(id: u32, x: f32, y: f32) -> Target {
        Target {
            ship: ShipRef::Npc(NpcId(id)),
            ship_type: ShipId(128),
            fleet: ShipRef::Npc(NpcId(id)),
            ties: Ties::default(),
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            radius: 8.0,
            condition: Condition::Intact,
        }
    }

    #[test]
    fn a_beam_hits_the_nearest_ship_it_reaches() {
        let beam = Beam::launch(laser(), FIRER, FIRER, &at(10.0, 5.0, 90.0), Aiming::Ahead);
        let (ship, point) = beam
            .hit(&among(&[
                target(2, 90.0, 5.0),
                target(3, 50.0, 8.0),
                target(4, 70.0, 4.0),
            ]))
            .expect("a hit");
        assert_eq!(ship, ShipRef::Npc(NpcId(3)));
        assert!(point.x > 40.0 && point.x < 44.0, "{point:?}");
        assert!((point.y - 5.0).abs() < 1e-3, "on the beam: {point:?}");
        assert_eq!(beam.hit(&among(&[])), None);
    }

    #[test]
    fn a_beam_reaches_its_length_and_the_ships_radius_and_no_further() {
        let beam = Beam::launch(laser(), FIRER, FIRER, &at(0.0, 0.0, 90.0), Aiming::Ahead);
        let one = |x, y| beam.hit(&among(&[target(2, x, y)])).map(|(ship, _)| ship);
        assert_eq!(one(107.9, 0.0), Some(ShipRef::Npc(NpcId(2))));
        assert_eq!(one(108.1, 0.0), None, "beyond its length and the radius");
        assert_eq!(one(50.0, 7.9), Some(ShipRef::Npc(NpcId(2))));
        assert_eq!(one(50.0, 8.1), None, "beside it");
        assert_eq!(one(-9.0, 0.0), None, "behind it");
    }

    const TARGET: ShipRef = ShipRef::Npc(NpcId(2));

    /// A turreted beam from (0, 0), launched with its firer facing right.
    fn turreted() -> Beam {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance: 3,
            ..laser_record()
        });
        Beam::launch(
            spec,
            FIRER,
            FIRER,
            &at(0.0, 0.0, 90.0),
            Aiming::Ship(TARGET),
        )
    }

    #[test]
    fn a_turreted_beam_aims_a_whole_degree_at_its_target_every_tick() {
        let mut beam = turreted();
        assert!(
            close(beam.end, Vec2::new(100.0, 0.0)),
            "as its firer faces: {beam:?}"
        );
        for (x, y, heading) in [
            (0.0, -50.0, 0.0),
            (50.0, 0.0, 90.0),
            (50.0, 50.0, 134.0),
            (-30.0, 30.0, 226.0),
        ] {
            assert!(follow(
                &mut beam,
                at(0.0, 0.0, 90.0),
                &[target(2, x, y)],
                &[]
            ));
            assert_eq!(beam.heading, heading, "({x}, {y})");
            assert!(
                close(beam.end, facing(heading) * 100.0),
                "({x}, {y}): {beam:?}"
            );
        }
        assert!(follow(
            &mut beam,
            at(10.0, 0.0, 0.0),
            &[target(2, 10.0, 50.0)],
            &[]
        ));
        assert_eq!(beam.start, Vec2::new(10.0, 0.0));
        assert!(
            close(beam.end, Vec2::new(10.0, 100.0)),
            "from its firer: {beam:?}"
        );
    }

    #[test]
    fn a_turreted_beam_tracks_a_breaking_up_target_and_ends_once_it_is_destroyed_or_gone() {
        let at_target = |condition| Target {
            condition,
            ..target(2, 0.0, -50.0)
        };
        let dying = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let firer = at(0.0, 0.0, 90.0);
        assert!(follow(&mut turreted(), firer, &[at_target(dying)], &[]));
        assert!(follow(
            &mut turreted(),
            firer,
            &[at_target(Condition::Disabled)],
            &[]
        ));
        assert!(!follow(
            &mut turreted(),
            firer,
            &[at_target(Condition::Destroyed)],
            &[]
        ));
        assert!(!follow(
            &mut turreted(),
            firer,
            &[target(3, 0.0, -50.0)],
            &[]
        ));
    }

    /// A shot of the IR Missile at (`x`, `y`), numbered `id`.
    fn missile(id: u32, x: f32, y: f32) -> Shot {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance: 1,
            ..weapon(134)
        });
        Shot {
            id: ShotId(id),
            position: Vec2::new(x, y),
            ..Shot::launch(spec, TARGET, TARGET, &at(0.0, 0.0, 0.0), 0.0)
        }
    }

    #[test]
    fn a_point_defence_beam_follows_its_shot_and_hits_no_ship() {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance: 10,
            ..laser_record()
        });
        let mut beam = Beam::launch(
            spec,
            FIRER,
            FIRER,
            &at(0.0, 0.0, 0.0),
            Aiming::Shot(ShotId(5)),
        );
        assert!(
            close(beam.end, Vec2::new(0.0, -100.0)),
            "as its firer faces"
        );
        let shots = [missile(4, 0.0, -40.0), missile(5, 0.0, 40.0)];
        assert!(follow(&mut beam, at(0.0, 0.0, 0.0), &[], &shots));
        assert!(
            close(beam.end, Vec2::new(0.0, 100.0)),
            "at shot 5: {beam:?}"
        );
        assert_eq!(beam.hit(&among(&[target(2, 0.0, 50.0)])), None);
        assert!(
            !follow(&mut beam, at(0.0, 0.0, 0.0), &[], &shots[..1]),
            "its shot gone"
        );
    }

    #[test]
    fn a_beam_ahead_ignores_any_target_and_keeps_no_launch_offset() {
        let mut beam = Beam::launch(laser(), FIRER, FIRER, &at(0.0, 0.0, 90.0), Aiming::Ahead);
        assert!(follow(
            &mut beam,
            at(0.0, 0.0, 0.0),
            &[target(2, 50.0, 0.0)],
            &[]
        ));
        assert!(close(beam.end, facing(0.0) * 100.0), "{beam:?}");
    }

    #[test]
    fn a_beam_never_hits_its_firer_or_a_destroyed_ship_and_hits_a_disabled_one() {
        let beam = Beam::launch(laser(), FIRER, FIRER, &at(0.0, 0.0, 90.0), Aiming::Ahead);
        let firer = Target {
            ship: FIRER,
            ..target(1, 0.0, 0.0)
        };
        let destroyed = Target {
            condition: Condition::Destroyed,
            ..target(3, 30.0, 0.0)
        };
        assert_eq!(beam.hit(&among(&[firer, destroyed])), None);
        let disabled = Target {
            condition: Condition::Disabled,
            ..target(4, 50.0, 0.0)
        };
        assert_eq!(
            beam.hit(&among(&[firer, disabled])).map(|(ship, _)| ship),
            Some(ShipRef::Npc(NpcId(4)))
        );
    }

    // Who a beam may hit (`_HandleBeams`' filter).

    fn npc(id: u32) -> ShipRef {
        ShipRef::Npc(NpcId(id))
    }

    fn led_by(leader: ShipRef) -> Ties {
        Ties {
            leader: Some(leader),
            ..Ties::default()
        }
    }

    /// `ship`, tied as `ties`, `x` pixels along the beam.
    fn on_beam(ship: ShipRef, x: f32, ties: Ties) -> Target {
        Target {
            ship,
            ties,
            ..target(0, x, 0.0)
        }
    }

    /// `ship`, tied as `ties`, off the beam.
    fn aside(ship: ShipRef, ties: Ties) -> Target {
        Target {
            ship,
            ties,
            ..target(0, 0.0, 500.0)
        }
    }

    /// The first of `targets` a beam from the origin along +x fired by
    /// `firer`, tied as `ties`, hits, among `govts`, as `pass_through`
    /// says.
    fn beam_by(
        firer: ShipRef,
        ties: Ties,
        targets: &[Target],
        govts: &Governments,
        pass_through: PassThrough,
    ) -> Option<ShipRef> {
        let beam = Beam {
            ties,
            ..Beam::launch(laser(), firer, firer, &at(0.0, 0.0, 90.0), Aiming::Ahead)
        };
        let sides = Sides {
            targets,
            govts,
            pass_through,
        };
        beam.hit(&sides).map(|(ship, _)| ship)
    }

    /// As [`beam_by`], by the original's reading and no governments.
    fn first(firer: ShipRef, ties: Ties, targets: &[Target]) -> Option<ShipRef> {
        beam_by(
            firer,
            ties,
            targets,
            &Governments::empty(),
            PassThrough::PlayerSide,
        )
    }

    #[test]
    fn a_beam_skips_its_firer_its_leader_and_the_ships_it_leads() {
        let targets = [
            aside(FIRER, led_by(npc(5))),
            on_beam(npc(5), 20.0, Ties::default()),
            on_beam(npc(2), 40.0, led_by(FIRER)),
            on_beam(npc(3), 60.0, Ties::default()),
        ];
        assert_eq!(first(FIRER, led_by(npc(5)), &targets), Some(npc(3)));
    }

    #[test]
    fn a_beam_hits_a_sibling_unless_both_are_player_allied() {
        let siblings = [
            aside(FIRER, led_by(npc(5))),
            on_beam(npc(2), 30.0, led_by(npc(5))),
        ];
        assert_eq!(first(FIRER, led_by(npc(5)), &siblings), Some(npc(2)));
        let escorts = [
            aside(FIRER, led_by(ShipRef::Player)),
            on_beam(npc(2), 30.0, led_by(ShipRef::Player)),
            on_beam(npc(3), 60.0, Ties::default()),
        ];
        assert_eq!(
            first(FIRER, led_by(ShipRef::Player), &escorts),
            Some(npc(3))
        );
        assert_eq!(
            first(ShipRef::Player, Ties::default(), &escorts),
            Some(npc(3)),
            "the player's beam passes its escort"
        );
        // An escort's fighter is allied, and led: the escort's beam passes
        // it.
        let fighter = [
            aside(npc(7), led_by(ShipRef::Player)),
            aside(FIRER, led_by(ShipRef::Player)),
            on_beam(npc(2), 30.0, led_by(npc(7))),
            on_beam(npc(3), 60.0, Ties::default()),
        ];
        assert_eq!(
            first(FIRER, led_by(ShipRef::Player), &fighter),
            Some(npc(3))
        );
    }

    #[test]
    fn a_beam_hits_a_ship_of_its_firers_government() {
        let of_140 = Ties {
            govt: Some(GovtId(140)),
            ..Ties::default()
        };
        let targets = [aside(FIRER, of_140), on_beam(npc(2), 30.0, of_140)];
        assert_eq!(first(FIRER, of_140, &targets), Some(npc(2)));
    }

    #[test]
    fn a_beam_passes_through_ship_895_and_stops_on_a_later_class() {
        let class = |ship, x, class| Target {
            ship_type: ShipId(class),
            ..on_beam(ship, x, Ties::default())
        };
        let targets = [class(npc(2), 30.0, 895), class(npc(3), 60.0, 896)];
        assert_eq!(first(FIRER, Ties::default(), &targets), Some(npc(3)));
    }

    #[test]
    fn a_beam_and_booty_0x0100_pairs() {
        let untouchable = Ties {
            booty: Some(0x0100),
            ..Ties::default()
        };
        let player_side = [
            aside(ShipRef::Player, Ties::default()),
            on_beam(npc(2), 30.0, untouchable),
            on_beam(npc(3), 60.0, Ties::default()),
        ];
        assert_eq!(
            first(ShipRef::Player, Ties::default(), &player_side),
            Some(npc(3))
        );
        let at_the_player = [
            aside(FIRER, untouchable),
            on_beam(ShipRef::Player, 20.0, Ties::default()),
            on_beam(npc(4), 40.0, led_by(ShipRef::Player)),
            on_beam(npc(3), 60.0, Ties::default()),
        ];
        assert_eq!(first(FIRER, untouchable, &at_the_player), Some(npc(3)));
        // Its leader's düde, when it has none of its own.
        let led = [
            aside(npc(6), untouchable),
            aside(FIRER, led_by(npc(6))),
            on_beam(ShipRef::Player, 20.0, Ties::default()),
            on_beam(npc(3), 60.0, Ties::default()),
        ];
        assert_eq!(first(FIRER, led_by(npc(6)), &led), Some(npc(3)));
    }

    #[test]
    fn a_beam_stops_on_a_breaking_up_ship_in_front_of_a_live_one() {
        let hulk = Target {
            condition: Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
            ..on_beam(npc(2), 30.0, Ties::default())
        };
        let targets = [hulk, on_beam(npc(3), 60.0, Ties::default())];
        assert_eq!(first(FIRER, Ties::default(), &targets), Some(npc(2)));
    }

    #[test]
    fn the_bibles_pass_through_counts_the_player_alone() {
        let govts = Governments::new([
            govt(140),
            GovtRecord {
                flags: crate::govt::NEVER_ATTACKS_PLAYER,
                ..govt(141)
            },
        ]);
        let never_attacks = Ties {
            govt: Some(GovtId(141)),
            ..Ties::default()
        };
        let untouchable = Ties {
            booty: Some(0x0100),
            ..Ties::default()
        };
        let targets = [
            aside(ShipRef::Player, Ties::default()),
            aside(FIRER, led_by(ShipRef::Player)),
            on_beam(npc(2), 20.0, never_attacks),
            on_beam(npc(3), 40.0, untouchable),
            on_beam(npc(4), 60.0, Ties::default()),
        ];
        let by = |firer, ties, pass_through| beam_by(firer, ties, &targets, &govts, pass_through);
        assert_eq!(
            by(ShipRef::Player, Ties::default(), PassThrough::PlayerSide),
            Some(npc(2)),
            "the original's beams test no government"
        );
        assert_eq!(
            by(ShipRef::Player, Ties::default(), PassThrough::PlayerOnly),
            Some(npc(4)),
            "the player's weapons can't hit them"
        );
        assert_eq!(
            by(FIRER, led_by(ShipRef::Player), PassThrough::PlayerSide),
            Some(npc(2)),
            "an escort's beam passes the düde"
        );
        let escorts = [targets[0], targets[1], targets[3], targets[4]];
        assert_eq!(
            beam_by(
                FIRER,
                led_by(ShipRef::Player),
                &escorts,
                &govts,
                PassThrough::PlayerOnly
            ),
            Some(npc(3)),
            "an escort is not the player"
        );
    }
}
