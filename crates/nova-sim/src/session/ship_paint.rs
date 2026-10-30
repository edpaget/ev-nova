//! A ship's paint, as `_GetShipPaintColor` (@0x7d0c-0x7dc0 in `EV Nova.app`'s
//! `EV Nova`, i386) reads it, whole. Three sources, in order:
//!
//! 1. **The player** (ship +0x86 == 0, @0x7d13): the player record's three
//!    words (`*0x3b5084` +0/+2/+4, @0x7d1d-0x7d29), the pilot's
//!    [`paint`](crate::pilot::Pilot::paint).
//! 2. **A person** (ship +0xc8d0 != -1, @0x7d2f-0x7d60): its `përs`
//!    `Color` as `_LoadObjectData` unpacks it (@0x7c2a4-0x7c2e4,
//!    [`Rgb15::of_rgb`]). A person whose colour unpacks to (0, 0, 0) does
//!    not fall back to its government: it is unpainted.
//! 3. **A government** (ship +0x98 != -1, @0x7d62-0x7d88): its `gövt`
//!    `ShipColor`, unpacked the same way (@0x7bcf9-0x7bd33). A ship of no
//!    person and no government is unpainted (@0x7d6d).
//!
//! All three channels 0 read as (32, 32, 32), unpainted (@0x7d8c-0x7da7):
//! the engine's `None`.
//!
//! Only two callers read it: `_HandleShipDisplay` for every ship's hull
//! and alt layer (@0x2be78; the view draws it from
//! [`FrameShip::paint`](crate::flight_frame::FrameShip::paint)), and
//! `_DoShipCapture`'s copy onto the player (@0x41759; see
//! [`Session::assign`](super::Session::assign)). No other screen draws
//! paint, and none of it plays a sound.
//!
//! An NPC's person is looked up in the system's traffic table
//! ([`Traffic::person`](crate::traffic::Traffic::person)), as the hail,
//! the offers and the escorts look it up, rather than copied onto the NPC
//! as it spawns, so every spawn and respawn path reads it alike. A person
//! the table no longer holds reads as unpainted.

use super::{Space, Statics};
use crate::outfit_effects::Rgb15;
use crate::traffic::npc::Npc;

impl Space {
    /// NPC `npc`'s paint (see the module docs): its person's `Color`
    /// while it carries a person, else its government's `ShipColor`;
    /// `None`, unpainted, for (0, 0, 0) or for an independent ship.
    pub(super) fn npc_paint(&self, npc: &Npc, statics: &Statics) -> Option<Rgb15> {
        let colour = match npc.person {
            Some(person) => self.traffic.person(person.id)?.record.color,
            None => statics.govts.get(npc.govt?)?.ship_color,
        };
        Rgb15::of_rgb(colour)
    }
}

#[cfg(test)]
pub(in crate::session) mod tests {
    use super::super::Session;
    use crate::catalog::{GovtId, GovtRecord, PersonId, PersonRecord, SystemId, SystemTraffic};
    use crate::chance::NeverFires;
    use crate::outfit_effects::Rgb15;
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, FakePilotCatalog, govt, person, ship};
    use crate::traffic::npc::NpcId;

    /// Government 128, its ships painted (31, 2, 1).
    const RULING: GovtId = GovtId(128);
    /// `ShipColor` 0x00F81008: (31, 2, 1).
    const RULING_COLOUR: u32 = 0x00F8_1008;

    /// System 130 has no traffic of its own and one Person slot, naming
    /// `përs` 600 of `Color` `colour` at 100 %, flying ship 129 for
    /// government 128, whose ships are painted (31, 2, 1).
    pub(in crate::session) fn painted(colour: u32) -> FakePilotCatalog {
        let mut slots: [(Option<PersonId>, i16); 8] = Default::default();
        slots[0] = (Some(PersonId(600)), 100);
        FakePilotCatalog {
            traffic: vec![(
                SystemId(130),
                SystemTraffic {
                    dude_types: [(-1, 0); 8],
                    avg_ships: 0,
                    persons: slots,
                },
            )],
            persons: vec![PersonRecord {
                govt: Some(RULING),
                color: colour,
                ..person(600, 129)
            }],
            govts: vec![GovtRecord {
                ship_color: RULING_COLOUR,
                ..govt(RULING.0)
            }],
            ship_records: vec![ship(129, FAST)],
            ..crate::testkit::catalog()
        }
    }

    /// A session over `catalog`, populated: the person its slot names
    /// flies in the system.
    pub(in crate::session) fn populated(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.populate(catalog, &mut NeverFires);
        session
    }

    /// The paint of the NPC `session` holds whose person is 600.
    fn persons_paint(session: &Session) -> Option<Rgb15> {
        let npc = session
            .npcs()
            .iter()
            .find(|npc| npc.person.map(|person| person.id) == Some(PersonId(600)))
            .expect("the person flies");
        session.space.npc_paint(npc, &session.statics)
    }

    /// A ship of no person, of government `govt`, added to `session`.
    fn plain(session: &mut Session, govt: Option<GovtId>) -> NpcId {
        let mut npc = crate::testkit::npc(0, ShipStats::default());
        npc.govt = govt;
        session.space.traffic.add_npc_at(npc, 5)
    }

    fn paint_of(session: &Session, id: NpcId) -> Option<Rgb15> {
        let npc = session.space.npc(id).expect("there");
        session.space.npc_paint(npc, &session.statics)
    }

    #[test]
    fn a_person_is_painted_in_its_colour() {
        let session = populated(&painted(0x0010_2030));
        assert_eq!(persons_paint(&session), Some(Rgb15 { r: 2, g: 4, b: 6 }));
    }

    #[test]
    fn a_person_of_no_colour_is_unpainted_though_its_government_paints() {
        // A person's (0, 0, 0) is no fallback to its government (@0x7d60).
        let session = populated(&painted(0));
        let npc = session.npcs().first().expect("the person flies");
        assert_eq!(npc.govt, Some(RULING), "in the painting government");
        assert_eq!(persons_paint(&session), None);
        let faint = populated(&painted(0x0007_0707));
        assert_eq!(persons_paint(&faint), None, "only low bits: (0, 0, 0)");
    }

    #[test]
    fn a_ship_of_no_person_takes_its_governments_ship_colour() {
        let mut session = populated(&painted(0));
        let id = plain(&mut session, Some(RULING));
        assert_eq!(paint_of(&session, id), Rgb15::of_rgb(RULING_COLOUR));
        assert_eq!(paint_of(&session, id), Some(Rgb15 { r: 31, g: 2, b: 1 }));
    }

    #[test]
    fn an_independent_ship_of_no_person_is_unpainted() {
        let mut session = populated(&painted(0));
        let id = plain(&mut session, None);
        assert_eq!(paint_of(&session, id), None);
        let unknown = plain(&mut session, Some(GovtId(999)));
        assert_eq!(paint_of(&session, unknown), None, "no such government");
    }
}
