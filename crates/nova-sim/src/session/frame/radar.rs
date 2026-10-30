//! The radar's half of [`Session::fill_frame`]: a blip for every stellar
//! the system lists, shown or not, in its `NavDefs` order, as
//! `_DrawStatusRadar`'s stellar loop draws them (@0x498a0-0x49a8e; see
//! the `story` module's "Checked for play-feel-4 phase 15").

use super::super::Session;
use crate::catalog::LandingSite;
use crate::flight_frame::FrameRadarStellar;
use crate::gate::GateKind;
use crate::landing::StellarFlags;

impl Session {
    /// Fills `into` with each stellar the system lists, shown or not, in
    /// its `NavDefs` order (the chart's `listed`), each with its place, its
    /// shape, the width of its `Type` master (`Space::radar_type_width`)
    /// and its IFF colour ([`Session::stellar_iff`]).
    pub(super) fn fill_radar_stellars(&self, into: &mut Vec<FrameRadarStellar>) {
        into.clear();
        into.extend(self.chart.listed.iter().map(|site| FrameRadarStellar {
            id: site.id,
            position: site.position,
            square: is_square(site),
            type_width: self.space.radar_type_width(site),
            iff: self.stellar_iff(site.id),
        }));
    }
}

/// Whether `site`'s blip is a square: a station (`Flags` 0x10), a
/// hypergate or a wormhole (`Flags2` 0x3000, [`GateKind::of`];
/// @0x49939-0x49946). The radar reads no master for the shape.
fn is_square(site: &LandingSite) -> bool {
    site.flags & StellarFlags::STATION != 0 || GateKind::of(site.flags2).is_some()
}

#[cfg(test)]
mod tests {
    use crate::catalog::{LandingSite, StellarId, SystemId};
    use crate::control::SetExpr;
    use crate::flight_frame::{FlightFrame, FrameRadarStellar};
    use crate::geometry::Vec2;
    use crate::iff::StellarIff;
    use crate::landing::{HYPERGATE, StellarFlags, WORMHOLE};
    use crate::session::Session;
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, hideable, life, planet};

    /// The testkit catalog with stellar 200, of graphic type `graphic`,
    /// listed by both systems, 130 last and 131 between 140 and 141, and
    /// every system hideable: 130, the lowest visible lister, owns it, so
    /// 131 lists it but does not show it.
    fn twice_listed(graphic: i16) -> FakePilotCatalog {
        let mut catalog = catalog();
        let shared = LandingSite {
            graphic,
            min_status: 10,
            ..planet(200, 500.0, 600.0)
        };
        for (system, sites) in &mut catalog.sites {
            if *system == SystemId(131) {
                sites.push(shared);
                sites.push(planet(141, 10.0, 20.0));
            } else {
                sites.push(shared);
            }
        }
        hideable(&mut catalog);
        catalog
    }

    /// A session of `catalog`, jumped to 131.
    fn in_131(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        crate::testkit::jump(&mut session, catalog, 131).expect("arrives");
        session
    }

    fn radar(session: &Session) -> Vec<FrameRadarStellar> {
        let mut frame = FlightFrame::default();
        session.fill_frame(&mut frame);
        frame.radar_stellars
    }

    fn blip(session: &Session, stellar: i16) -> FrameRadarStellar {
        radar(session)
            .into_iter()
            .find(|blip| blip.id == StellarId(stellar))
            .expect("listed")
    }

    fn run(session: &mut Session, text: &str) {
        session.run_set(
            &SetExpr::parse(text).expect("parses"),
            &mut Scripted::default(),
        );
    }

    /// A rebuild point: an in-flight `M` into the same system.
    fn rebuild(session: &mut Session, catalog: &FakePilotCatalog) {
        run(session, "M130");
        let _ = session.settle_script(catalog, &mut Scripted::default());
    }

    #[test]
    fn the_radar_lists_a_stellar_another_system_owns_in_navdefs_order() {
        // `_DrawStatusRadar` walks `NavDefs` 0..15, skipping only -1
        // (@0x498ad-0x498c5), with no active or owner test.
        let catalog = twice_listed(0);
        let session = in_131(&catalog);
        assert!(
            session
                .stellars_shown()
                .eq([StellarId(140), StellarId(141)]),
            "131 doesn't show 200"
        );
        let blips = radar(&session);
        assert_eq!(
            blips.iter().map(|blip| blip.id).collect::<Vec<_>>(),
            [StellarId(140), StellarId(200), StellarId(141)]
        );
        assert_eq!(blips[1].position, Vec2::new(500.0, 600.0));
        assert_eq!(blips[2].position, Vec2::new(10.0, 20.0));
    }

    #[test]
    fn a_stellar_not_shown_reads_its_type_master_kept_loaded_by_a_shown_one_or_none() {
        // The master table indexed by `Type`, with no owner test
        // (@0x499e8-0x49a0a): 140 is of graphic type 0 too.
        let shared = in_131(&twice_listed(0));
        assert_eq!(blip(&shared, 200).type_width, Some(100));
        let alone = in_131(&twice_listed(7));
        assert_eq!(blip(&alone, 200).type_width, None, "freed at the jump");
    }

    /// [`catalog`] with system 130 holding stellar 128, 200 square, of
    /// graphic type 10 and `DeadType` 11, back after 5 days, and planet
    /// 129 of graphic type `other_graphic`.
    fn ruin(other_graphic: i16) -> FakePilotCatalog {
        let mut catalog = catalog().living([life(128, 5)]);
        catalog.sites[0].1 = vec![
            LandingSite {
                frame_size: Some((200, 200)),
                graphic: 10,
                dead_graphic: Some(11),
                dead_frame_size: Some((300, 300)),
                ..planet(128, 0.0, 0.0)
            },
            LandingSite {
                graphic: other_graphic,
                ..planet(129, 2000.0, 0.0)
            },
        ];
        catalog
    }

    #[test]
    fn a_destroyed_stellar_reads_its_type_width_until_a_rebuild_point_then_none() {
        // The radar reads the `Type` master, never the `DeadType` sheet
        // drawn (@0x499e8-0x499f9), and a master changes only at a
        // rebuild point.
        let catalog = ruin(12);
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(blip(&session, 128).type_width, Some(200), "whole");
        run(&mut session, "Y128");
        assert_eq!(
            blip(&session, 128).type_width,
            Some(200),
            "destroyed, before a rebuild point"
        );
        rebuild(&mut session, &catalog);
        assert_eq!(blip(&session, 128).type_width, None, "after one");
    }

    #[test]
    fn a_destroyed_stellar_sharing_its_type_with_a_whole_one_keeps_its_width() {
        let catalog = ruin(10);
        let mut session = Session::start(&catalog).expect("starts");
        run(&mut session, "Y128");
        rebuild(&mut session, &catalog);
        assert_eq!(blip(&session, 128).type_width, Some(200));
    }

    #[test]
    fn a_loaded_type_whose_spin_counts_no_frames_reads_32() {
        // `_GetSpriteXSize` gives 32 for a frame index out of its count
        // (@0x26d4-0x26fc).
        let mut catalog = catalog();
        catalog.sites[0].1[0].type_counts_frames = false;
        catalog.sites[0].1[0].frame_size = Some((200, 200));
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(blip(&session, 128).type_width, Some(32));
    }

    #[test]
    fn a_station_hypergate_or_wormhole_is_a_square() {
        // `Flags` 0x10 or `Flags2` 0x3000 (@0x49939-0x49946); no other
        // bit makes a square.
        let shapes = [
            (0x0000_0000, 0x0000, false),
            (StellarFlags::STATION, 0x0000, true),
            (0x0000_0000, HYPERGATE, true),
            (0x0000_0000, WORMHOLE, true),
            (StellarFlags::STATION, HYPERGATE | WORMHOLE, true),
            (!StellarFlags::STATION, !(HYPERGATE | WORMHOLE), false),
        ];
        let mut catalog = catalog();
        catalog.sites[0].1 = (150..)
            .zip(shapes)
            .map(|(id, (flags, flags2, _))| LandingSite {
                flags,
                flags2,
                ..planet(id, f32::from(id), 0.0)
            })
            .collect();
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            radar(&session)
                .iter()
                .map(|blip| blip.square)
                .collect::<Vec<_>>(),
            shapes.map(|(_, _, square)| square)
        );
    }

    #[test]
    fn a_stellar_not_shown_is_coloured_by_its_holders_record() {
        // `_ColorCodeStellar` reads `_playerRecord[stellar +0x14]`
        // (@0xaad5, @0xaaed), with no owner or visibility test: 130's
        // record, as 130 holds 200.
        let catalog = twice_listed(0);
        let mut session = in_131(&catalog);
        session.pilot.set_system_record(SystemId(130), 10);
        assert_eq!(
            session.stellar_iff(StellarId(200)),
            Some(StellarIff::Cleared)
        );
        assert_eq!(blip(&session, 200).iff, Some(StellarIff::Cleared));
        session.pilot.set_system_record(SystemId(130), 0);
        session.pilot.set_system_record(SystemId(131), 10);
        assert_eq!(blip(&session, 200).iff, Some(StellarIff::Denied));
    }
}
