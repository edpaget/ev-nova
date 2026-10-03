//! The system catalog over the game data: a thin mapping from `GameData`'s
//! `sÿst` and `spöb` records and its stellar sprite lookup to
//! [`SystemContents`].

use std::num::NonZeroU16;

use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::{GameData, Record, StellarSprite, StoreEntry};

use super::catalog::{
    AnimationData, StellarContents, StellarId, StellarSheet, SystemCatalog, SystemContents,
    SystemId,
};
use crate::galaxy::data::display_name;

/// `Flags2`'s "animate only when destroyed" bit.
const ONLY_WHEN_DESTROYED: u16 = 0x0080;

/// Reads the system afresh on every call; a view asks once, when it opens.
impl SystemCatalog for GameData {
    fn system(&self, id: SystemId) -> SystemContents {
        let mut problems = Vec::new();
        let (name, stellars) = match self.get::<System>(id.0) {
            Some(Ok(entry)) => (
                display_name(entry.name, "sÿst", id.0),
                stellars(self, &entry, &mut problems),
            ),
            Some(Err(err)) => {
                problems.push(err.to_string());
                // An undecodable sÿst still has the name in its resource map.
                let name = self
                    .resource(System::TYPE, id.0)
                    .and_then(|system| system.resource.name());
                (display_name(name, "sÿst", id.0), Vec::new())
            }
            None => {
                problems.push(format!("no sÿst {}", id.0));
                (display_name(None, "sÿst", id.0), Vec::new())
            }
        };
        SystemContents {
            id,
            name,
            stellars,
            problems,
        }
    }
}

/// Every stellar of the system that can be placed, in navigation order. A
/// `spöb` that is missing or does not decode has no position, so it is
/// left out and reported.
fn stellars(
    data: &GameData,
    system: &StoreEntry<'_, System>,
    problems: &mut Vec<String>,
) -> Vec<StellarContents> {
    let mut stellars = Vec::new();
    for &id in system.record.nav_def.iter().flatten() {
        match data.get::<Stellar>(id.0) {
            Some(Ok(stellar)) => stellars.push(contents(data, id, &stellar)),
            Some(Err(err)) => problems.push(format!("sÿst {}: {err}", system.id)),
            None => problems.push(format!("sÿst {}: no spöb {}", system.id, id.0)),
        }
    }
    stellars
}

fn contents(data: &GameData, id: StellarId, stellar: &StoreEntry<'_, Stellar>) -> StellarContents {
    let record = stellar.record;
    StellarContents {
        id,
        name: display_name(stellar.name, "spöb", id.0),
        x: record.x_pos,
        y: record.y_pos,
        sprite: data
            .stellar_sprite(id)
            .map(|sprite| sheet(&sprite))
            .map_err(|err| err.to_string()),
        animation: AnimationData {
            delay: record.anim_delay,
            frame0_bias: record.frame0_bias,
            only_when_destroyed: record.flags2.bits() & ONLY_WHEN_DESTROYED != 0,
        },
    }
}

/// The sheet's ID, frame count and frame size.
fn sheet(sprite: &StellarSprite<'_>) -> StellarSheet {
    // An `rlëD` header counts its frames in a u16, and the decoder rejects
    // a sheet with none (`GraphicsError::NoFrames`).
    let frames = NonZeroU16::new(sprite.sheet.frames().len() as u16)
        .expect("a decoded rlëD has at least one frame");
    StellarSheet {
        image_id: sprite.image_id,
        frames,
        frame_width: sprite.sheet.frame_width(),
        frame_height: sprite.sheet.frame_height(),
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::num::NonZeroU16;
    use std::path::Path;

    use nova_data::graphics::RLED;
    use nova_data::graphics::fixture::RledBuilder;
    use nova_data::records::spin::Spin;
    use nova_data::records::stellar::Stellar;
    use nova_data::records::system::System;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_data::{GameData, Record};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};

    use super::*;
    use crate::system::catalog::{AnimationData, StellarContents, StellarId, StellarSheet};

    /// One data file, `/data/Nova Data`, holding a fork.
    struct OneFile(Vec<u8>);

    impl DirLister for OneFile {
        fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
            Ok(vec![Listing {
                name: "Nova Data".into(),
                kind: EntryKind::File,
            }])
        }
    }

    impl ForkReader for OneFile {
        fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            Ok((fork == Fork::Data).then(|| self.0.clone()))
        }
    }

    /// A resource: type, ID, name and bytes.
    type Res = (ResType, i16, Option<&'static str>, Vec<u8>);

    fn store(resources: &[Res]) -> GameData {
        let fork = resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, name, data)| {
                fork.resource(*ty, *id, name.map(str::as_bytes), data)
            })
            .build()
            .bytes;
        let file = OneFile(fork);
        GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
    }

    fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
        for (i, value) in values.iter().enumerate() {
            let start = at + 2 * i;
            bytes[start..start + 2].copy_from_slice(&value.to_be_bytes());
        }
    }

    /// A 428-byte `sÿst` with these stellars; every other hyperlink and
    /// stellar slot is -1.
    fn system(stellars: &[i16]) -> Vec<u8> {
        let mut bytes = vec![0; 428];
        assert_eq!(Some(bytes.len()), System::SIZE);
        put_i16s(&mut bytes, 0x04, &[-1; 32]);
        put_i16s(&mut bytes, 0x24, stellars);
        put_i16s(&mut bytes, 0x66, &[-1]);
        bytes
    }

    /// A 1118-byte `spöb` at (`x`, `y`) of graphic type `graphic_type`.
    fn stellar(x: i16, y: i16, graphic_type: i16) -> Vec<u8> {
        let mut bytes = vec![0; 1118];
        assert_eq!(Some(bytes.len()), Stellar::SIZE);
        put_i16s(&mut bytes, 0x00, &[x, y, graphic_type]);
        bytes
    }

    /// `stellar` with its `Flags2`, `AnimDelay` and `Frame0Bias` set.
    fn animated(x: i16, y: i16, graphic_type: i16, flags2: u16, delay: i16, bias: i16) -> Vec<u8> {
        let mut bytes = stellar(x, y, graphic_type);
        bytes[0x20..0x22].copy_from_slice(&flags2.to_be_bytes());
        put_i16s(&mut bytes, 0x22, &[delay, bias]);
        bytes
    }

    /// A 12-byte `spïn` naming `rlëD` `image`, `x_tiles` across.
    fn spin(image: i16, x_tiles: i16) -> Vec<u8> {
        let mut bytes = vec![0; 12];
        assert_eq!(Some(bytes.len()), Spin::SIZE);
        put_i16s(&mut bytes, 0x00, &[image, -1, 0, 0, x_tiles, 1]);
        bytes
    }

    /// An `rlëD` of `frames` frames, each `size` x `size`.
    fn sheet(frames: u16, size: u16) -> Vec<u8> {
        (0..frames)
            .fold(RledBuilder::new(size, size), |sheet, _| {
                sheet.frame(|f| {
                    (0..size).fold(f, |f, _| f.line().pixels(&vec![0x7C00; size.into()]))
                })
            })
            .build()
    }

    fn sheet_of(image_id: i16, frames: u16, size: u32) -> StellarSheet {
        StellarSheet {
            image_id,
            frames: NonZeroU16::new(frames).expect("non-zero"),
            frame_width: size,
            frame_height: size,
        }
    }

    /// Sol (130) holding Earth (128) at (0, 0), type 0, one 8 x 8 frame;
    /// and a Wormhole (129) at (900, -600), type 1, 4 frames of 6 x 6
    /// animating every 2 ticks with frame 0 unbiased.
    fn sol() -> Vec<Res> {
        vec![
            (System::TYPE, 130, Some("Sol"), system(&[129, 128])),
            (Stellar::TYPE, 128, Some("Earth"), stellar(0, 0, 0)),
            (
                Stellar::TYPE,
                129,
                Some("Wormhole;far out"),
                animated(900, -600, 1, 0x2000, 2, -1),
            ),
            (Spin::TYPE, 1000, None, spin(1000, 1)),
            (Spin::TYPE, 1001, None, spin(1001, 4)),
            (RLED, 1000, None, sheet(1, 8)),
            (RLED, 1001, None, sheet(4, 6)),
        ]
    }

    fn earth() -> StellarContents {
        StellarContents {
            id: StellarId(128),
            name: "Earth".to_owned(),
            x: 0,
            y: 0,
            sprite: Ok(sheet_of(1000, 1, 8)),
            animation: AnimationData::default(),
        }
    }

    #[test]
    fn a_full_system_has_every_stellar_in_navigation_order() {
        let data = store(&sol());
        assert_eq!(
            data.system(SystemId(130)),
            SystemContents {
                id: SystemId(130),
                name: "Sol".to_owned(),
                stellars: vec![
                    StellarContents {
                        id: StellarId(129),
                        name: "Wormhole".to_owned(),
                        x: 900,
                        y: -600,
                        sprite: Ok(sheet_of(1001, 4, 6)),
                        animation: AnimationData {
                            delay: 2,
                            frame0_bias: -1,
                            only_when_destroyed: false,
                        },
                    },
                    earth(),
                ],
                problems: Vec::new(),
            }
        );
    }

    #[test]
    fn stellars_skip_empty_slots_anywhere_in_the_record() {
        let mut resources = sol();
        let mut bytes = system(&[]);
        put_i16s(&mut bytes, 0x24 + 2 * 15, &[128]);
        resources[0] = (System::TYPE, 130, Some("Sol"), bytes);
        let stellars = store(&resources).system(SystemId(130)).stellars;
        assert_eq!(stellars, [earth()]);
    }

    #[test]
    fn a_missing_or_undecodable_stellar_is_left_out_with_a_problem() {
        let mut short = stellar(0, 0, 0);
        short.pop();
        let mut resources = sol();
        resources[0] = (System::TYPE, 130, Some("Sol"), system(&[150, 128, 151]));
        resources.push((Stellar::TYPE, 151, Some("Broken"), short));
        let contents = store(&resources).system(SystemId(130));
        assert_eq!(contents.stellars, [earth()]);
        assert_eq!(contents.problems.len(), 2, "{:?}", contents.problems);
        assert_eq!(contents.problems[0], "sÿst 130: no spöb 150");
        assert!(
            contents.problems[1].starts_with("sÿst 130: spöb 151 \"Broken\""),
            "{:?}",
            contents.problems
        );
    }

    #[test]
    fn a_sprite_that_cannot_be_resolved_says_why() {
        let mut resources = sol();
        resources.push((System::TYPE, 131, None, system(&[140, 141])));
        resources.push((Stellar::TYPE, 140, Some("Odd"), stellar(5, 6, -1)));
        resources.push((Stellar::TYPE, 141, Some("Lost"), stellar(7, 8, 2)));
        let contents = store(&resources).system(SystemId(131));
        let sprites: Vec<_> = contents.stellars.iter().map(|s| s.sprite.clone()).collect();
        assert_eq!(
            sprites,
            [
                Err("spöb 140: graphic type -1 is outside 0 to 255".to_owned()),
                Err("spöb 141: no spïn 1002 for its graphic type 2".to_owned()),
            ]
        );
        let places: Vec<_> = contents.stellars.iter().map(|s| (s.x, s.y)).collect();
        assert_eq!(places, [(5, 6), (7, 8)]);
        assert_eq!(contents.problems, Vec::<String>::new());
    }

    #[test]
    fn flags2_0x0080_means_animate_only_when_destroyed() {
        let mut resources = sol();
        resources.push((System::TYPE, 131, None, system(&[140, 141, 142])));
        for (id, flags2) in [(140, 0x0080), (141, 0x0040 | 0x0200), (142, 0xFF7F)] {
            resources.push((Stellar::TYPE, id, None, animated(0, 0, 0, flags2, 3, 2)));
        }
        let contents = store(&resources).system(SystemId(131));
        let flags: Vec<AnimationData> = contents.stellars.iter().map(|s| s.animation).collect();
        let data = |only_when_destroyed| AnimationData {
            delay: 3,
            frame0_bias: 2,
            only_when_destroyed,
        };
        assert_eq!(flags, [data(true), data(false), data(false)]);
    }

    #[test]
    fn a_missing_system_is_empty_and_a_problem() {
        let contents = store(&sol()).system(SystemId(200));
        assert_eq!(
            contents,
            SystemContents {
                id: SystemId(200),
                name: "sÿst 200".to_owned(),
                stellars: Vec::new(),
                problems: vec!["no sÿst 200".to_owned()],
            }
        );
    }

    #[test]
    fn an_undecodable_system_is_empty_with_its_error_and_its_name() {
        let mut short = system(&[128]);
        short.pop();
        let mut resources = sol();
        resources.push((System::TYPE, 131, Some("Short;note"), short.clone()));
        resources.push((System::TYPE, 132, None, short));
        let data = store(&resources);

        let named = data.system(SystemId(131));
        assert_eq!(named.name, "Short");
        assert_eq!(named.stellars, []);
        assert_eq!(named.problems.len(), 1);
        assert!(
            named.problems[0].starts_with("sÿst 131 \"Short;note\""),
            "{:?}",
            named.problems
        );

        let unnamed = data.system(SystemId(132));
        assert_eq!(unnamed.name, "sÿst 132");
        assert!(
            unnamed.problems[0].starts_with("sÿst 132"),
            "{:?}",
            unnamed.problems
        );
    }

    #[test]
    fn names_fall_back_to_the_type_and_id() {
        let mut resources = sol();
        resources.push((System::TYPE, 131, Some(" ;note"), system(&[140])));
        resources.push((Stellar::TYPE, 140, None, stellar(0, 0, 0)));
        let contents = store(&resources).system(SystemId(131));
        assert_eq!(contents.name, "sÿst 131");
        assert_eq!(contents.stellars[0].name, "spöb 140");
    }
}
