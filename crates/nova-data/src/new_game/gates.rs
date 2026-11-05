//! The jump points as hypergate `spöb`s, and each system's `nav_def`
//! holding its stellars and gates.
//!
//! A jump point is a hypergate with one link, to its partner point's gate.
//! The original's `_PlayerEnterHypergate` leaves through the first of the
//! entered gate's own links whose system is the one picked
//! (@0x6389d-0x63999) and comes out at that linked `spöb`
//! (@0x63a7f-0x63aa6), whether or not it links back, so one link to the
//! partner's gate arrives at the partner, a one-way point included.
//!
//! The point's `stability` isn't read yet: unstable and dead points are
//! travelled as stable ones until `play-the-sample` phase 1.

use std::collections::BTreeMap;

use nova_galaxy_format::id::{Id, PointRef};
use nova_galaxy_format::load::{Galaxy, Problem};
use nova_galaxy_format::model::JumpPoint;

use super::ids::{IdTable, STELLAR_IDS};
use super::map::{absent, govt, position};
use super::records::neutral_stellar;
use crate::decode::Entry;
use crate::records::stellar::{Stellar, flags, flags2};
use crate::registry::AnyRecord;
use crate::wire::flags::{Flags16, Flags32};
use crate::wire::id::StellarId;

/// How many `spöb`s a `sÿst`'s `nav_def` holds.
pub const NAV_SLOTS: usize = 16;

/// Numbers the jump points of the systems in `systems`, in (system id,
/// point id) order, from the ID after `stellars`' last (128 with none),
/// within [`STELLAR_IDS`], reporting each past its end against its
/// system's file.
pub fn number(
    galaxy: &Galaxy,
    systems: &BTreeMap<Id, i16>,
    stellars: &BTreeMap<Id, i16>,
    problems: &mut Vec<Problem>,
) -> BTreeMap<PointRef, i16> {
    let first = stellars
        .values()
        .max()
        .map_or(*STELLAR_IDS.start(), |last| last.saturating_add(1));
    let last = *STELLAR_IDS.end();
    let mut next = first..=last;
    let mut gates = BTreeMap::new();
    for system in systems.keys() {
        let doc = &galaxy.systems[system];
        let mut points: Vec<&JumpPoint> = doc.data.jump_points.iter().collect();
        points.sort_by(|a, b| a.id.cmp(&b.id));
        for point in points {
            match next.next() {
                Some(nova) => {
                    gates.insert(PointRef::new(system, &point.id), nova);
                }
                None => problems.push(Problem::new(
                    &doc.path,
                    format!(
                        "its jump point `{}` is past the last spöb ID, {last}; it is left out",
                        point.id
                    ),
                )),
            }
        }
    }
    gates
}

/// The hypergate `spöb` of the jump point `point`, of the system in
/// `file`, numbered `nova`: at the
/// point's position, held by its owner, landable, its one link the
/// partner's gate, named for the partner's system, heading ships out at
/// random (`CustSndID` -1). A partner that is no jump point is reported
/// against the system's file and leaves the gate with no link, but not
/// one in a system that failed to load or was left out for its ID.
#[must_use]
pub fn gate(
    galaxy: &Galaxy,
    ids: &IdTable,
    file: &str,
    point: &JumpPoint,
    nova: i16,
    problems: &mut Vec<Problem>,
) -> Entry<AnyRecord> {
    let what = format!("jump point `{}`'s position", point.id);
    let (x_pos, y_pos) = position(point.position, &what, file, problems);
    let partner = ids.gates.get(&point.to).copied();
    let to_system = point.to.system();
    if partner.is_none()
        && (ids.systems.contains_key(&to_system)
            || absent(galaxy, &galaxy.systems, "systems", &to_system))
    {
        problems.push(Problem::new(
            file,
            format!(
                "its jump point `{}` leads to `{}`, which is no jump point; its gate has no link",
                point.id, point.to
            ),
        ));
    }
    let mut hyper_link = [None; 8];
    hyper_link[0] = partner.map(StellarId);
    let stellar = Stellar {
        x_pos,
        y_pos,
        flags: Flags32(flags::CAN_LAND),
        flags2: Flags16(flags2::HYPERGATE),
        hyper_link,
        govt: govt(galaxy, ids, point.owner.as_ref(), file, problems),
        ..neutral_stellar()
    };
    let name = galaxy
        .systems
        .get(&to_system)
        .map_or_else(|| to_system.to_string(), |system| system.data.name.clone());
    Entry {
        id: nova,
        name: Some(name),
        record: AnyRecord::Stellar(Box::new(stellar)),
    }
}

/// The `nav_def` of the system `id`, from the file `file`: its stellars in
/// ID order, then its gates in ID order. Past [`NAV_SLOTS`], the stellars
/// past the slots the gates leave are left out first, never a gate while
/// a stellar holds a slot: a gate with no slot can't be entered, so its
/// link would be dead. One problem names all left out.
#[must_use]
pub fn nav_def(
    galaxy: &Galaxy,
    ids: &IdTable,
    id: &Id,
    file: &str,
    problems: &mut Vec<Problem>,
) -> [Option<StellarId>; NAV_SLOTS] {
    // In ID order: the tables iterate in id order, which numbered them.
    let stellars: Vec<(i16, String)> = ids
        .stellars
        .iter()
        .filter(|(stellar, _)| galaxy.stellars[*stellar].data.system == *id)
        .map(|(stellar, &nova)| (nova, format!("`{stellar}`")))
        .collect();
    let gates: Vec<(i16, String)> = ids
        .gates
        .iter()
        .filter(|(point, _)| point.system() == *id)
        .map(|(point, &nova)| (nova, format!("`{point}`")))
        .collect();
    let kept_gates = gates.len().min(NAV_SLOTS);
    let kept_stellars = stellars.len().min(NAV_SLOTS - kept_gates);
    let left_out: Vec<&str> = stellars[kept_stellars..]
        .iter()
        .chain(&gates[kept_gates..])
        .map(|(_, name)| name.as_str())
        .collect();
    if !left_out.is_empty() {
        let verb = if left_out.len() == 1 { "has" } else { "have" };
        problems.push(Problem::new(
            file,
            format!(
                "it has {} and {}, more than a system's {NAV_SLOTS} nav slots; {} {verb} no \
                 place in it",
                counted(stellars.len(), "stellar"),
                counted(gates.len(), "jump point"),
                left_out.join(", ")
            ),
        ));
    }
    let mut nav_def = [None; NAV_SLOTS];
    let held = stellars[..kept_stellars]
        .iter()
        .chain(&gates[..kept_gates])
        .map(|&(nova, _)| Some(StellarId(nova)));
    for (slot, stellar) in nav_def.iter_mut().zip(held) {
        *slot = stellar;
    }
    nav_def
}

/// `count` `what`s, in words: "1 stellar", "2 stellars".
fn counted(count: usize, what: &str) -> String {
    if count == 1 {
        format!("1 {what}")
    } else {
        format!("{count} {what}s")
    }
}
