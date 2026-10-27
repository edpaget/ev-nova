//! Each stellar's animation state kept across the systems a flight
//! passes through, as the original keeps it on the stellar's `spöb`
//! record (current frame +0x476, next frame +0x478, timer +0x490), not on
//! its sprite slot (stellar-fidelity phase 12).
//!
//! A stellar not shown stands still: the sprite tick walks only the
//! sprite slots shown (`_HandleStellarSprites` @0x2e713-0x2e73e), and no
//! other routine writes the three fields while it is elsewhere. When it is
//! shown again, the rebuild sets its slot to the current frame modulo the
//! slot's frame count (`_HandleStellarSystemVisibility` @0x33096), and its
//! timer carries on from where it stood. So the flight keeps each
//! stellar's [`Frames`] as its scene is let go ([`KeptFrames::keep`]) and
//! gives them back to the next scene that shows it
//! ([`SystemScene::resume`]). A stellar the flight has never shown starts
//! afresh ([`Frames::new`]).
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

    /// The animation state kept for stellar `id`; `None` for one never
    /// kept.
    #[must_use]
    pub fn get(&self, id: StellarId) -> Option<Frames> {
        self.frames.get(&id).copied()
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
}
