//! The system catalog over the game data: a thin mapping from `GameData`'s
//! `sÿst` and `spöb` records and its stellar sprite lookup to
//! [`SystemContents`], with graphic type 0's sheet (`spïn` 1000), which a
//! destroyed stellar with no `DeadType` sheet may draw after a rebuild
//! point (story-runtime phase 9).

use std::num::NonZeroU16;

use nova_data::graphics::SpriteSheet;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::{GameData, Record, StandIn, StandInOf, StellarSprite, StoreEntry, stellar_spin_id};

use super::catalog::{
    StellarAnimation, StellarContents, StellarId, StellarSheet, SystemCatalog, SystemContents,
    SystemId,
};
use crate::color::Color;
use crate::flight::data::view_outline;
use crate::galaxy::data::display_name;
use crate::outline::StellarStandIn;
use crate::sheet_image::SheetImage;

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
            type_zero_sprite: type_zero_sprite(self),
            problems,
        }
    }
}

/// Graphic type 0's sheet, `spïn` 1000, or why it cannot be shown.
fn type_zero_sprite(data: &GameData) -> Result<StellarSheet, String> {
    let spin = stellar_spin_id(0).expect("graphic type 0 has a spïn");
    data.spin_sheet(spin)
        .map(|found| {
            let image = SheetImage::of(found.kind, found.spin_id, found.image_id);
            sheet(image, &found.sheet, found.stellar_frame_count())
        })
        .map_err(|err| err.to_string())
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
            .map(|sprite| stellar_sheet(&sprite))
            .map_err(|err| err.to_string()),
        dead_sprite: data
            .stellar_dead_sprite(id)
            .map(|sprite| sprite.as_ref().map(stellar_sheet))
            .map_err(|err| err.to_string())
            .transpose(),
        animation: StellarAnimation {
            delay: record.anim_delay,
            frame0_bias: record.frame0_bias,
            flags2: record.flags2.bits(),
        },
        stand_in: data.stand_in(StandInOf::Stellar(id)).map(stand_in),
    }
}

/// A stellar's stand-in from the data, in the view's terms.
fn stand_in(stand_in: &StandIn) -> StellarStandIn {
    let opaque = |[r, g, b]: [u8; 3]| Color::rgba(r, g, b, 255);
    match stand_in {
        StandIn::Outline(outline) => StellarStandIn::Outline(view_outline(outline)),
        StandIn::Disc { color } => StellarStandIn::Disc(opaque(*color)),
        StandIn::Ring { color } => StellarStandIn::Ring(opaque(*color)),
    }
}

/// A resolved stellar sprite's sheet.
fn stellar_sheet(sprite: &StellarSprite<'_>) -> StellarSheet {
    let image = SheetImage::of(sprite.kind, sprite.spin_id, sprite.image_id);
    sheet(image, &sprite.sheet, sprite.frame_count())
}

/// The sheet `image`, decoded as `sheet`, animating over `frames` of it
/// (the store's `master_frame_count`), with its frame size.
fn sheet(image: SheetImage, sheet: &SpriteSheet, frames: usize) -> StellarSheet {
    // Neither decoder makes a sheet with no frames
    // (`GraphicsError::NoFrames`). An `rlëD` header counts its frames in a
    // u16, but a `PICT` pair cut into small frames can hold more: the
    // count saturates.
    let count = u16::try_from(frames).unwrap_or(u16::MAX);
    StellarSheet {
        image,
        frames: NonZeroU16::new(count).unwrap_or(NonZeroU16::MIN),
        frame_width: sheet.frame_width(),
        frame_height: sheet.frame_height(),
        image_size: (sheet.image_width(), sheet.image_height()),
        eight_bit: sheet.depth() == 8,
        clip_quirks: sheet.clip_quirks(),
    }
}

#[cfg(test)]
mod tests {

    use std::num::NonZeroU16;

    use nova_data::graphics::RLED;
    use nova_data::graphics::fixture::RledBuilder;
    use nova_data::records::spin::Spin;
    use nova_data::records::stellar::Stellar;
    use nova_data::records::system::System;
    use nova_data::store::fixture::one_file;

    use nova_data::{GameData, Record};
    use nova_rsrc::ResType;
    use nova_rsrc::fixture::ForkBuilder;

    use super::*;
    use crate::system::catalog::{StellarAnimation, StellarContents, StellarId, StellarSheet};

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
        one_file(fork).load()
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

    /// A 1118-byte `spöb` at (`x`, `y`) of graphic type `graphic_type`,
    /// with no different graphic when destroyed (`DeadType` -1).
    fn stellar(x: i16, y: i16, graphic_type: i16) -> Vec<u8> {
        let mut bytes = vec![0; 1118];
        assert_eq!(Some(bytes.len()), Stellar::SIZE);
        put_i16s(&mut bytes, 0x00, &[x, y, graphic_type]);
        put_i16s(&mut bytes, 0x240, &[-1]);
        bytes
    }

    /// `stellar` at (0, 0), type 0, with `DeadType` `dead_type`.
    fn dead(dead_type: i16) -> Vec<u8> {
        let mut bytes = stellar(0, 0, 0);
        put_i16s(&mut bytes, 0x240, &[dead_type]);
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

    /// [`spin`] with `y_tiles` down as well.
    fn spin_tiles(image: i16, x_tiles: i16, y_tiles: i16) -> Vec<u8> {
        let mut bytes = spin(image, x_tiles);
        put_i16s(&mut bytes, 0x0A, &[y_tiles]);
        bytes
    }

    #[test]
    fn a_spin_counting_no_frames_reads_its_sheet_with_no_problem() {
        // `_HandleStellarSystemVisibility` only writes crash-log lines for
        // a slot with no frame or one not locked (@0x332d5-0x3330b): no
        // alert, so no problem line.
        for (x, y) in [(0, 1), (2, 0), (-1, 2)] {
            let mut resources = sol();
            let earth_spin = resources
                .iter_mut()
                .find(|(ty, id, ..)| *ty == Spin::TYPE && *id == 1000)
                .expect("Earth's spïn");
            earth_spin.3 = spin_tiles(1000, x, y);
            let data = store(&resources);
            let contents = data.system(SystemId(130));
            assert_eq!(contents.problems, Vec::<String>::new(), "{x} x {y}");
            let earth = contents
                .stellars
                .iter()
                .find(|stellar| stellar.id == StellarId(128))
                .expect("Earth");
            assert!(earth.sprite.is_ok(), "{x} x {y}: {:?}", earth.sprite);
            let scene = crate::system::SystemScene::load(&data, SystemId(130));
            assert!(scene.problems().is_empty(), "{x} x {y}");
        }
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
            image: crate::SheetImage::Rled(image_id),
            frames: NonZeroU16::new(frames).expect("non-zero"),
            frame_width: size,
            frame_height: size,
            image_size: (size, size),
            eight_bit: false,
            clip_quirks: false,
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
            dead_sprite: None,
            animation: StellarAnimation::default(),
            stand_in: None,
        }
    }

    #[test]
    fn a_stellar_s_stand_in_comes_from_the_store_in_the_view_s_terms() {
        let dart = nova_data::Outline {
            triangles: vec![nova_data::OutlineTriangle {
                corners: [[0.0, 2.5], [1.0, -1.0], [-1.0, -1.0]],
                color: [9, 8, 7],
            }],
        };
        let data = store(&sol()).with_stand_ins(std::collections::BTreeMap::from([
            (
                StandInOf::Stellar(StellarId(128)),
                StandIn::Disc { color: [1, 2, 3] },
            ),
            (StandInOf::Stellar(StellarId(129)), StandIn::Outline(dart)),
        ]));
        let sol = data.system(SystemId(130));
        let stand_in = |id: i16| {
            sol.stellars
                .iter()
                .find(|stellar| stellar.id == StellarId(id))
                .expect("listed")
                .stand_in
                .clone()
        };
        assert_eq!(
            stand_in(128),
            Some(StellarStandIn::Disc(Color::rgba(1, 2, 3, 255)))
        );
        assert_eq!(
            stand_in(129),
            Some(StellarStandIn::Outline(crate::outline::Outline {
                triangles: vec![crate::outline::OutlineTriangle {
                    corners: [
                        crate::Point::new(0.0, 2.5),
                        crate::Point::new(1.0, -1.0),
                        crate::Point::new(-1.0, -1.0),
                    ],
                    color: Color::rgba(9, 8, 7, 255),
                }],
            }))
        );
    }

    #[test]
    fn a_gate_s_ring_comes_from_the_store() {
        let data = store(&sol()).with_stand_ins(std::collections::BTreeMap::from([(
            StandInOf::Stellar(StellarId(128)),
            StandIn::Ring { color: [4, 5, 6] },
        )]));
        let sol = data.system(SystemId(130));
        assert!(sol.stellars.iter().any(
            |stellar| stellar.stand_in == Some(StellarStandIn::Ring(Color::rgba(4, 5, 6, 255)))
        ));
    }

    #[test]
    fn a_stock_store_s_stellars_have_no_stand_ins() {
        let sol = store(&sol()).system(SystemId(130));
        assert!(!sol.stellars.is_empty());
        assert!(
            sol.stellars
                .iter()
                .all(|stellar| stellar.stand_in.is_none())
        );
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
                        dead_sprite: None,
                        animation: StellarAnimation {
                            delay: 2,
                            frame0_bias: -1,
                            flags2: 0x2000,
                        },
                        stand_in: None,
                    },
                    earth(),
                ],
                type_zero_sprite: Ok(sheet_of(1000, 1, 8)),
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
    fn a_dead_type_gives_the_sheet_drawn_while_destroyed_or_none() {
        let mut resources = sol();
        resources.push((System::TYPE, 131, None, system(&[140, 141, 142, 143])));
        resources.push((Stellar::TYPE, 140, None, dead(3)));
        resources.push((Stellar::TYPE, 141, None, dead(-1)));
        resources.push((Stellar::TYPE, 142, None, dead(256)));
        resources.push((Stellar::TYPE, 143, None, dead(2)));
        resources.push((Spin::TYPE, 1003, None, spin(1003, 2)));
        resources.push((RLED, 1003, None, sheet(2, 4)));
        let contents = store(&resources).system(SystemId(131));
        let dead: Vec<_> = contents
            .stellars
            .iter()
            .map(|s| s.dead_sprite.clone())
            .collect();
        assert_eq!(
            dead,
            [
                Some(Ok(sheet_of(1003, 2, 4))),
                None,
                None,
                Some(Err(
                    "spöb 143: no spïn 1002 for its graphic type 2".to_owned()
                )),
            ]
        );
        let alive: Vec<_> = contents.stellars.iter().map(|s| s.sprite.clone()).collect();
        assert_eq!(
            alive,
            vec![Ok(sheet_of(1000, 1, 8)); 4],
            "Type is unchanged"
        );
    }

    #[test]
    fn the_animation_carries_flags2_raw() {
        let mut resources = sol();
        resources.push((System::TYPE, 131, None, system(&[140, 141, 142, 143])));
        let all = [
            (140, 0x0080),
            (141, 0x0040 | 0x0200),
            (142, 0xFF7F),
            (143, 0x1203),
        ];
        for (id, flags2) in all {
            resources.push((Stellar::TYPE, id, None, animated(0, 0, 0, flags2, 3, 2)));
        }
        let contents = store(&resources).system(SystemId(131));
        let flags: Vec<StellarAnimation> = contents.stellars.iter().map(|s| s.animation).collect();
        let data = |flags2| StellarAnimation {
            delay: 3,
            frame0_bias: 2,
            flags2,
        };
        assert_eq!(
            flags,
            [data(0x0080), data(0x0240), data(0xFF7F), data(0x1203)]
        );
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
                type_zero_sprite: Ok(sheet_of(1000, 1, 8)),
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

    /// Sol with the Wormhole's `spïn` 1001 tiled 2 x 1 over its 4-frame
    /// `rlëD`, and graphic type 0's `spïn` 1000 tiled 2 x 1 over a 4-frame
    /// `rlëD` too.
    fn two_tiles_over_four_frames() -> Vec<Res> {
        let mut resources = sol();
        for (ty, id, _, data) in &mut resources {
            match (*ty, *id) {
                (t, 1000 | 1001) if t == Spin::TYPE => *data = spin_tiles(*id, 2, 1),
                (t, 1000) if t == RLED => *data = sheet(4, 8),
                _ => {}
            }
        }
        resources
    }

    #[test]
    fn a_stellar_with_two_tiles_over_four_frames_animates_over_frames_0_and_1() {
        let data = store(&two_tiles_over_four_frames());
        let contents = data.system(SystemId(130));
        let wormhole = contents
            .stellars
            .iter()
            .find(|stellar| stellar.id == StellarId(129))
            .expect("the Wormhole");
        assert_eq!(wormhole.sprite.as_ref().map(|s| s.frames.get()), Ok(2));
        let mut scene = crate::system::SystemScene::load(&data, SystemId(130));
        let mut shown = std::collections::BTreeSet::new();
        for _ in 0..40 {
            scene.step(1, &mut nova_sim::NeverFires);
            let stellar = scene
                .stellars()
                .iter()
                .find(|stellar| stellar.id == StellarId(129))
                .expect("the Wormhole");
            shown.insert(stellar.frames.shown());
        }
        assert_eq!(shown, [0, 1].into());
    }

    #[test]
    fn graphic_type_0s_sheet_counts_the_frames_the_store_gives_it() {
        let data = store(&two_tiles_over_four_frames());
        let expected = data
            .spin_sheet(1000)
            .expect("resolves")
            .stellar_frame_count();
        let sprite = data
            .system(SystemId(130))
            .type_zero_sprite
            .expect("a sheet");
        assert_eq!(usize::from(sprite.frames.get()), expected);
    }

    #[test]
    fn graphic_type_0s_sheet_comes_with_the_system_or_why_not() {
        let mut resources = sol();
        resources.push((System::TYPE, 131, None, system(&[129])));
        let data = store(&resources);
        assert_eq!(
            data.system(SystemId(131)).type_zero_sprite,
            Ok(sheet_of(1000, 1, 8)),
            "whether a stellar of the system has type 0 or not"
        );
        resources.retain(|(ty, id, _, _)| (*ty, *id) != (Spin::TYPE, 1000));
        assert_eq!(
            store(&resources).system(SystemId(131)).type_zero_sprite,
            Err("no spïn 1000".to_owned())
        );
        let mut resources = sol();
        resources.retain(|(ty, id, _, _)| (*ty, *id) != (RLED, 1000));
        assert_eq!(
            store(&resources).system(SystemId(130)).type_zero_sprite,
            Err("spïn 1000: no rlëD or PICT 1000 for its sprites".to_owned())
        );
    }

    /// A sheet's image size is the decoded sheet's, apart from its frame
    /// box: the store decides how far a 32-bit `rlëD` draws past its box
    /// (rendering phase 1), and the catalog passes that on.
    #[test]
    fn a_sheet_s_image_size_is_the_decoded_sheet_s_apart_from_its_box() {
        let wide = RledBuilder::new(2, 2)
            .depth(32)
            .count_lines()
            .frame(|f| {
                f.line()
                    .pixels32(&[0x7C00, 0x7C00])
                    .line()
                    .pixels32(&[0x7C00, 0x7C00])
            })
            .build();
        let mut resources = sol();
        resources
            .retain(|(ty, id, _, _)| ![(Spin::TYPE, 1001), (RLED, 1001)].contains(&(*ty, *id)));
        resources.push((Spin::TYPE, 1001, None, spin(1001, 1)));
        resources.push((RLED, 1001, None, wide));
        let data = store(&resources);
        let decoded = data
            .stellar_sprite(StellarId(129))
            .expect("the wormhole's sheet decodes")
            .sheet;
        let wormhole = data
            .system(SystemId(130))
            .stellars
            .into_iter()
            .find(|stellar| stellar.id == StellarId(129))
            .expect("the wormhole")
            .sprite
            .expect("its sheet");
        assert_eq!(
            wormhole.image_size,
            (decoded.image_width(), decoded.image_height())
        );
        assert_eq!(
            (wormhole.frame_width, wormhole.frame_height),
            (decoded.frame_width(), decoded.frame_height())
        );
    }

    /// An 8-bit `rlëD`'s sheet says so, so the scene can draw it as the
    /// draw proc's depth-8 branches do; a 16-bit one's does not.
    #[test]
    fn an_8_bit_rled_s_sheet_is_eight_bit() {
        let eight = RledBuilder::new(2, 1)
            .depth(8)
            .frame(|f| f.line().pixels8(&[0x00, 0x7C]))
            .build();
        let mut resources = sol();
        resources.retain(|(ty, id, _, _)| (*ty, *id) != (RLED, 1001));
        resources.push((RLED, 1001, None, eight));
        let data = store(&resources);
        let sprites: Vec<_> = data
            .system(SystemId(130))
            .stellars
            .into_iter()
            .map(|stellar| (stellar.id, stellar.sprite.expect("its sheet").eight_bit))
            .collect();
        assert_eq!(sprites, [(StellarId(129), true), (StellarId(128), false)]);
    }

    /// A sheet whose clipped draw can differ from a crop says so, as the
    /// decoded sheet does, so the scene can ask for the clipped draw at the
    /// world's edge; a stock-shaped one does not.
    #[test]
    fn a_sheet_with_clip_quirks_says_so() {
        // A run whose word's halves differ, its line counted.
        let two_colour = RledBuilder::new(2, 1)
            .count_lines()
            .frame(|f| f.line().run(2, 0x7C00, 0x001F))
            .build();
        let mut resources = sol();
        resources.retain(|(ty, id, _, _)| (*ty, *id) != (RLED, 1001));
        resources.push((RLED, 1001, None, two_colour));
        let data = store(&resources);
        let decoded = data
            .stellar_sprite(StellarId(129))
            .expect("the wormhole's sheet decodes")
            .sheet;
        assert!(decoded.clip_quirks());
        let sprites: Vec<_> = data
            .system(SystemId(130))
            .stellars
            .into_iter()
            .map(|stellar| (stellar.id, stellar.sprite.expect("its sheet").clip_quirks))
            .collect();
        assert_eq!(sprites, [(StellarId(129), true), (StellarId(128), false)]);
    }
}
