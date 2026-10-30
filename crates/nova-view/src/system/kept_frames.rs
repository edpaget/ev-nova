//! Each stellar's animation state kept across the systems a flight
//! passes through, as the original keeps it on the stellar's `spöb`
//! record (current frame +0x476, next frame +0x478, timer +0x490), not on
//! its sprite slot (stellar-fidelity phase 12).
//!
//! A stellar not shown stands still: the sprite tick walks only the
//! sprite slots shown (`_HandleStellarSprites` @0x2e713-0x2e73e), and no
//! other routine writes the three fields while it is elsewhere but the
//! take-off's reset, which zeroes every record's current and next frames
//! at every take-off and gate exit ([`KeptFrames::reset`],
//! `_HandlePlayerDockRequest` @0x675ce-0x675fa,
//! rdm:phase/play-feel-4/phase-14-takeoff-stellar-frame-reset). When it is
//! shown again, the rebuild sets its slot to the current frame modulo the
//! slot's frame count (`_HandleStellarSystemVisibility` @0x33096), and its
//! timer carries on from where it stood. So the flight keeps each
//! stellar's [`Frames`] as its scene is let go ([`KeptFrames::keep`]) and
//! gives them back to the next scene that shows it
//! ([`SystemScene::resume`]). A stellar the flight has never shown starts
//! afresh ([`Frames::new`]), or, once a reset has run, as the reset leaves
//! it ([`Frames::after_reset`]).
//!
//! The store is the flight's: a load or a new pilot is a new flight, so
//! every stellar starts afresh then, as after a launch.

use std::collections::BTreeMap;

use super::animation::Frames;
use super::catalog::StellarId;
use super::scene::SystemScene;

/// Each stellar's animation state as the scene that last showed it left
/// it, by `spöb` ID.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptFrames {
    frames: BTreeMap<StellarId, Frames>,
    /// Whether the flight has taken off or come out of a gate since it
    /// began ([`KeptFrames::reset`]).
    reset: bool,
}

impl KeptFrames {
    /// A store that has kept nothing: every stellar starts afresh.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Keeps each of `scene`'s stellars' animation state as it stands,
    /// over what was kept for it before; every other stellar's stays.
    pub fn keep(&mut self, scene: &SystemScene) {
        for stellar in scene.stellars() {
            self.frames.insert(stellar.id, stellar.frames);
        }
    }

    /// The take-off's reset (`_HandlePlayerDockRequest` @0x675ce-0x675fa,
    /// play-feel-4 phase 14): every kept stellar's current and next
    /// frames to 0, its timer kept; and, as the original's loop zeroes all
    /// 2048 `spöb` records, every stellar never kept starts from then on
    /// with its frames and timer all 0 ([`Frames::after_reset`]).
    pub fn reset(&mut self) {
        for frames in self.frames.values_mut() {
            frames.take_off_reset();
        }
        self.reset = true;
    }

    /// The animation state kept for stellar `id`; for one never kept,
    /// `None` before the flight's first reset (it starts afresh,
    /// [`Frames::new`]) and [`Frames::after_reset`] after it.
    #[must_use]
    pub fn get(&self, id: StellarId) -> Option<Frames> {
        self.frames
            .get(&id)
            .copied()
            .or(self.reset.then(Frames::after_reset))
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use nova_sim::NeverFires;

    use super::*;
    use crate::system::catalog::{
        AnimationData, StellarContents, StellarSheet, SystemCatalog, SystemContents, SystemId,
    };

    /// System 130 holds stellar 129, system 131 stellar 140: each a
    /// 4-frame sheet stepping every `delay` ticks.
    struct FakeCatalog {
        delay: i16,
    }

    impl SystemCatalog for FakeCatalog {
        fn system(&self, id: SystemId) -> SystemContents {
            let stellar = match id.0 {
                130 => 129,
                131 => 140,
                other => panic!("asked for sÿst {other}"),
            };
            let sheet = |image_id| StellarSheet {
                image: crate::SheetImage::Rled(image_id),
                frames: NonZeroU16::new(4).expect("non-zero"),
                frame_width: 8,
                frame_height: 8,
            };
            SystemContents {
                id,
                name: format!("System {}", id.0),
                stellars: vec![StellarContents {
                    id: StellarId(stellar),
                    name: format!("Stellar {stellar}"),
                    x: 0,
                    y: 0,
                    sprite: Ok(sheet(1000 + stellar)),
                    dead_sprite: None,
                    animation: AnimationData {
                        delay: self.delay,
                        frame0_bias: 0,
                        flags2: 0,
                    },
                    radar_square: false,
                }],
                type_zero_sprite: Ok(sheet(3000)),
                problems: Vec::new(),
            }
        }
    }

    const A: SystemId = SystemId(130);
    const B: SystemId = SystemId(131);

    /// System `id` laid out from `catalog` and stepped `ticks` ticks.
    fn stepped(catalog: &FakeCatalog, id: SystemId, ticks: u128) -> SystemScene {
        let mut scene = SystemScene::load(catalog, id);
        scene.step(ticks, &mut NeverFires);
        scene
    }

    fn shown(scene: &SystemScene) -> u16 {
        scene.stellars()[0].frames.shown()
    }

    #[test]
    fn a_stellar_kept_resumes_its_frames() {
        let catalog = FakeCatalog { delay: 1 };
        let left = stepped(&catalog, A, 2);
        assert_eq!(shown(&left), 2);
        let mut kept = KeptFrames::new();
        kept.keep(&left);
        assert_eq!(kept.get(StellarId(129)), Some(left.stellars()[0].frames));
        let mut again = SystemScene::load(&catalog, A);
        again.resume(&kept);
        assert_eq!(shown(&again), 2, "where it was left");
        again.step(1, &mut NeverFires);
        assert_eq!(shown(&again), 3, "and on from there");
    }

    #[test]
    fn a_stellar_kept_resumes_its_timer() {
        // Every 2 ticks: 3 ticks in, frame 1 shows, its timer at 1.
        let catalog = FakeCatalog { delay: 2 };
        let left = stepped(&catalog, A, 3);
        assert_eq!(shown(&left), 1);
        let mut kept = KeptFrames::new();
        kept.keep(&left);
        let mut again = SystemScene::load(&catalog, A);
        again.resume(&kept);
        assert_eq!(again.stellars()[0].frames, left.stellars()[0].frames);
        again.step(1, &mut NeverFires);
        assert_eq!(shown(&again), 2, "one tick more reaches the delay");
    }

    #[test]
    fn a_stellar_never_kept_starts_afresh() {
        let catalog = FakeCatalog { delay: 1 };
        let mut kept = KeptFrames::new();
        assert_eq!(kept.get(StellarId(129)), None);
        kept.keep(&stepped(&catalog, A, 2));
        let mut other = SystemScene::load(&catalog, B);
        other.resume(&kept);
        assert_eq!(other.stellars()[0].frames, Frames::new());
        assert_eq!(kept.get(StellarId(140)), None);
    }

    #[test]
    fn keep_overwrites_and_keeps_others() {
        let catalog = FakeCatalog { delay: 1 };
        let mut kept = KeptFrames::new();
        kept.keep(&stepped(&catalog, A, 2));
        kept.keep(&stepped(&catalog, B, 1));
        let a = kept.get(StellarId(129)).expect("A's kept");
        assert_eq!(a.shown(), 2, "A's stellar kept beside B's");
        assert_eq!(kept.get(StellarId(140)).map(|f| f.shown()), Some(1));
        kept.keep(&stepped(&catalog, A, 3));
        let a = kept.get(StellarId(129)).expect("A's kept");
        assert_eq!(a.shown(), 3, "kept again over what it was");
    }

    #[test]
    fn a_reset_zeroes_every_kept_stellars_frames_and_keeps_its_timer() {
        // Every 2 ticks: 3 ticks in, frame 1 shows, its timer at 1.
        let catalog = FakeCatalog { delay: 2 };
        let mut kept = KeptFrames::new();
        kept.keep(&stepped(&catalog, A, 3));
        kept.keep(&stepped(&catalog, B, 5));
        kept.reset();
        for id in [StellarId(129), StellarId(140)] {
            let mut zeroed = kept.get(id).expect("kept");
            assert_eq!(zeroed.shown(), 0, "{id:?}");
            // Its timer at 1: one tick reaches the delay, showing next 0.
            zeroed.step(
                AnimationData {
                    delay: 2,
                    frame0_bias: 0,
                    flags2: 0,
                },
                false,
                NonZeroU16::new(4).expect("non-zero"),
                NonZeroU16::new(4).expect("non-zero"),
                &mut NeverFires,
            );
            assert_eq!(zeroed.shown(), 0, "{id:?}: next 0");
        }
        let mut again = SystemScene::load(&catalog, A);
        again.step(1, &mut NeverFires);
        again.resume(&kept);
        assert_eq!(shown(&again), 0, "resumed zeroed");
    }

    #[test]
    fn a_stellar_never_kept_starts_afresh_before_a_reset_and_with_next_0_after_one() {
        let mut kept = KeptFrames::new();
        assert_eq!(kept.get(StellarId(140)), None, "before: Frames::new()");
        kept.reset();
        assert_eq!(kept.get(StellarId(140)), Some(Frames::after_reset()));
        let catalog = FakeCatalog { delay: 1 };
        let mut other = SystemScene::load(&catalog, B);
        other.resume(&kept);
        assert_eq!(other.stellars()[0].frames, Frames::after_reset());
    }
}
