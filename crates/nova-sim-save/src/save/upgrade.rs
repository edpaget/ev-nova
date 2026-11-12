//! The upgrade from each save version to the next, a pure change to the
//! JSON that gives the fields the version added their defaults.
//!
//! - Version 1: the name, ship, system, the stellar last landed on, date,
//!   cash, reserves and course.
//! - Version 2: adds the explored systems and the legal records, none in
//!   a version 1 save.
//! - Version 3: adds the cargo held and the planetary events under way,
//!   none in an older save. A good or event whose record no longer
//!   exists is kept as saved.
//! - Version 4: adds the outfits the ship carries. An older save's ship
//!   carried its class's default items, which only the game data knows,
//!   so the upgrade saves them as `null`, "the ship's default items", and
//!   flying the pilot reads them (`nova_sim::Session::fly`);
//!   the next save lists them. An outfit whose record no longer exists is
//!   kept as saved.
//! - Version 5: adds the escorts, none in an older save.
//! - Version 6: adds each escort's standing order, none in an older save.
//! - Version 7: adds whether each escort is a carried fighter out of its
//!   bay, none in an older save.
//! - Version 8: adds each escort's daily wage, none (not hired) in an
//!   older save.
//! - Version 9: adds the persons gone for good and those holding a
//!   grudge against the player, none in an older save.
//! - Version 10: adds the person each escort is, none in an older save.
//! - Version 11: adds the control bits, every one clear in an older save,
//!   and the player's gender, male in an older save.
//! - Version 12: adds the ship's paint, unpainted in an older save.
//! - Version 13: adds the ship's name. An older save's ship was never
//!   named, so the upgrade saves `null`, and flying the pilot names it
//!   after its class (`nova_sim::Session::fly`). The next
//!   save keeps that name.
//! - Version 14: a ship's stock weapons and their `AmmoLoad` are outfits
//!   it owns. An older save's ship mounted its class's stock weapons
//!   beside its outfits, which only the game data knows, so the upgrade
//!   marks them not fitted (`stock_weapons_fitted`), and flying the pilot
//!   adds them to the outfits saved
//!   (`nova_sim::Session::fly`); the next save lists them.
//! - Version 15: adds the stellar each event under way is at, kept from
//!   the day it starts, as the original keeps and saves it. An older
//!   save's events were all at their record's `Stellar`, which only the
//!   game data knows, so the upgrade saves `null`, read as the record's
//!   own stellar. The next save keeps `null`.
//! - Version 16: adds the once-only story subscriptions that have fired,
//!   none in an older save.
//! - Version 17: adds the galaxy overlay, what play has changed of the
//!   galaxy, nothing in an older save.
//! - Version 18: a destroyed stellar holds its regeneration countdown. A
//!   version 17 save held only whether it was destroyed, which no version
//!   17 writer ever set, so the upgrade gives a destroyed one 0 days left:
//!   due to regenerate on its next day, as a `Y` with a `DeadTime` of 0
//!   leaves it.
//! - Version 19: adds the combat rating. No older save kept one, so the
//!   upgrade gives 0, the original's value for a pilot with no `chär`
//!   (@0x76f70).
//! - Version 20: adds the active missions, none in an older save.
//! - Version 21: adds the legal record in each system (`system_legal`,
//!   each its `system` and `record`, or `null` until derived). An older
//!   save kept only the record with each government, so the upgrade
//!   saves `null`, and flying the pilot derives each system's
//!   (`nova_sim::Session::fly`).
//! - Version 22: adds each active mission's special ships (`ships`) and
//!   auxiliary ships (`aux`: their `countdown`, how many `arrived` and
//!   how many are `left` to come). An older save kept none of their
//!   state, so the upgrade gives each mission none left, nowhere, with no
//!   timer and no name, and none destroyed, boarded, disabled or chased
//!   off: an old save's mission spawns no ship, and its `<SN>` reads
//!   empty. It gives each no auxiliary ship left to come, its countdown
//!   0, so none comes until a restart refills them (`Flags` 0x0010).
//! - Version 23: adds the crons running or holding off, each its `crön`
//!   ID, `days` and `holdoff`; none in an older save.
//! - Version 24: adds the active ranks; none in an older save.
//! - Version 25: adds whether the pilot plays Strict Play
//!   (`strict_play`). Every older pilot was made with the new-pilot
//!   dialog's Strict Play box greyed and clear, so the upgrade gives
//!   `false`.
//! - Version 26: adds the one ship type each active mission's special
//!   ships fly under `Flags` 0x0800 (`uniform_type` in its `ships`). An
//!   older save kept none, so the upgrade gives `null`, and such a
//!   mission draws its type at its ships' next spawn.
//! - Version 27: adds the system the player last jumped from
//!   (`jumped_from`, a `sÿst` ID, or `null` before the first jump), the
//!   side mission ships coming in enter from. An older save kept none,
//!   so the upgrade gives `null`, and after its load they come in at a
//!   drawn angle until the next jump.
//! - Version 28: adds whether each active mission's goals-met moment has
//!   come (`ship_done` in its `ships`), read by the other reading of
//!   [`RuleKey::ShipDoneOnce`](nova_sim_base::rulebook::RuleKey::ShipDoneOnce). An older
//!   save kept none, and the upgrade can't read a mission's `ShipGoal` to
//!   tell goals met by the moment from an escort goal's met at its
//!   return with none, so it gives `false` to every mission.
//! - Version 29: adds the systems charted (`charted`, `sÿst` IDs): those
//!   at explored level 2, which a take-off or a map gives, as the
//!   original's pilot file keeps each system's level (`_LoadPilotData`
//!   @0x7553f-0x75565; see `Pilot::explored_level`). An older save kept
//!   only whether each system was explored, so the upgrade gives `[]` and
//!   its explored systems load at level 1. Each charted system loads
//!   explored too.
//! - Version 30: adds the reinforcement countdowns (`reinforcement_days`,
//!   each its `system` and `days` left), as the original's pilot file
//!   keeps every system's +0xc4 (`_WritePilotData` @0x72aa0-0x72ac9,
//!   `_LoadPilotData` @0x76027-0x7605b; see the session's
//!   `reinforcements` module). The timer of a fleet on its way (+0xc8) is
//!   not saved, as the original's isn't. An older save kept none, so the
//!   upgrade gives `[]`. A load drops an entry of 0 days.
//! - Version 31: adds whether the pilot has seen the intro screen
//!   (`seen_intro`), as the original's pilot file keeps `seenIntroScreen`
//!   at +0x3086 (`_WritePilotData` @0x72970, `_LoadPilotData` @0x75ee2).
//!   An older save can't tell whether its pilot ever entered play, and
//!   showing a long-played pilot the intro is worse than skipping it for
//!   one never flown, so the upgrade gives `true`.
//! - Version 32: adds the course's start (`course_start`, a system or
//!   `null`), the original's `_hyperRoute` slot 0 (0x3b51c8): the system
//!   the course was last started or advanced from, which the rewrite at
//!   each rebuild point reads (`_HandleStellarSystemVisibility`
//!   @0x32aaa-0x32b0d). Every writer of slot 0 but `M` and `N` leaves it
//!   at the player's system, so the upgrade gives the pilot's `system`
//!   when the course has hops, and `null` when it has none.
//! - Version 33: adds the pilot's nickname (`nickname`, its text, empty
//!   for none), the original's `_playerNickname`, which its pilot file
//!   keeps as 0x40 bytes at +0x5d98 (`_WritePilotData` @0x727cc-0x727e9,
//!   `_LoadPilotData` @0x75d1a-0x75d37). Every older pilot was made with
//!   the New Pilot dialog's Nickname field inert, so the upgrade gives `""`.
//! - Version 34: adds each escort's mark for the next shipyard (`mark`,
//!   `upgrade`, `sell` or `null`), the original's pilot `+0xe82e[i]` and
//!   `+0xe8ae[i]` (`_LoadPilotData` @0x75a75-0x75ad3), set in the escort
//!   dialog (see `nova_sim::escort_dialog`). No older
//!   save could mark one, so the upgrade gives `null`.
//! - Version 35: adds each overlaid stellar's domination (`dominated` on
//!   each `galaxy.stellars` entry), the original's +0x46 that
//!   `_WritePilotData` keeps at pilot +0xdf2e + i (@0x72576-0x72580), and
//!   the stellars' defence ships left (`defence_ships`, each `stellar`,
//!   `left` and its `def_count` as the save knew it, by ascending ID),
//!   +0x466 at +0x6 + 2i (@0x7282d, @0x7285e); a stellar with no entry
//!   has its decode. The session's load drops either for a stellar the
//!   data no longer lists and zeroes a dominated stellar's count (see the
//!   session's `stellar_defence` module). No older save kept them, so the
//!   upgrade gives `false` and `[]`.
//! - Version 36: adds each escort's voice variant (`voice`: `even`, `odd`
//!   or `null`), the original's pilot `+0xe92e + 2i`. `_WritePilotData`
//!   writes it for every active ship the player leads, hired escorts too
//!   (@0x72717-0x7276c), but `_LoadPilotData` reads it only for a carried
//!   fighter (@0x75b65-0x75b85): a hired escort's copy has no reader, and
//!   is `null` here. No older save kept one, so the upgrade gives `null`
//!   and a fighter from it holds its berth's word.
//! - Version 37: adds the class table (`class_orders`), the order kept
//!   for each escort class, the original's table at 0x3b5394, which
//!   `_WritePilotData` copies raw to pilot +0x5d90 + 2i (@0x72b01-0x72b25)
//!   and `_LoadPilotData` reads back with any negative word as -1
//!   (@0x760c2-0x760ea), before it writes each loaded escort's order word
//!   from it. The original loads no file without it (it refuses every
//!   older version, @0x75c67-0x75cc7), so an older save's reading is the
//!   engine's: the upgrade gives `[-1, -1, -1, -1]`, a pilot who hasn't
//!   flown, and its escorts hold -1 until their first think.
//! - Version 38: adds each escort's berth (`berth`, a ship-table slot, or
//!   `null`), the slot `i` the original's save indexes its escort and
//!   fighter tables by (+0xe72e and +0xe7ae, `_WritePilotData`
//!   @0x725f6-0x72711), which `_LoadPilotData` walks up to spawn them in
//!   (@0x759af-0x75b91): a load packs the escorts into berths 1 up in
//!   that order (see the session's `loaded_berths` module). No older save
//!   kept it, so the upgrade gives `null`, and its escorts load in fleet
//!   order.
//! - Version 39: adds the new game's world flags (`flags`) and each Ink
//!   file's story state (`dialogs`), which the original has no
//!   counterpart of (see the `story_state` module). No older save kept
//!   them, so the upgrade gives `[]` and `[]`.
//! - Version 40: adds the new game's story (`story`): the day of the game,
//!   where each mission is, the jump points play has changed and the day
//!   each news item broke, which the original has no counterpart of (see
//!   the `storylines` module). No older save kept it, so the upgrade gives
//!   day 0 and three empty lists.
//! - Version 41: adds the news items the pilot has read (`read`, in
//!   `story`), which the original has no counterpart of (see the
//!   `storylines` module). No older save kept it, so the upgrade gives
//!   `[]`, and the pilot's next landing where anything is heard opens the
//!   news log once.

use serde_json::Value;

use super::CURRENT;

/// The upgrade from each version to the next: `UPGRADES[n - 1]` takes a
/// version `n` save to version `n + 1`.
pub(super) const UPGRADES: [fn(&mut Value); (CURRENT - 1) as usize] = [
    explored_and_legal,
    cargo_and_events,
    default_outfits,
    escorts,
    escort_orders,
    carried_fighters,
    hired_escorts,
    persons,
    person_escorts,
    bits_and_gender,
    paint,
    ship_names,
    stock_weapons,
    event_stellars,
    story_fired,
    galaxy,
    countdowns,
    combat_rating,
    active_missions,
    system_records,
    mission_ships,
    crons,
    ranks,
    strict_play,
    uniform_types,
    jumped_from,
    ship_done,
    charted,
    reinforcement_days,
    seen_intro,
    course_start,
    nickname,
    escort_marks,
    dominations,
    fighter_voices,
    class_orders,
    escort_berths,
    flags_and_dialogs,
    story_state_40,
    news_read_41,
];

/// Version 40 to 41: no news item read, as no older save kept which the
/// pilot had read.
pub(super) fn news_read_41(save: &mut Value) {
    if let Some(story) = save.get_mut("story").and_then(Value::as_object_mut) {
        story.insert("read".to_owned(), Value::Array(Vec::new()));
    }
}

/// Version 39 to 40: the new game's story empty, on day 0, as no older
/// save kept it.
pub(super) fn story_state_40(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert(
            "story".to_owned(),
            serde_json::json!({"day": 0, "missions": [], "points": [], "news": []}),
        );
    }
}

/// Version 38 to 39: no world flag set and no Ink file's story state, as
/// no older save kept them.
pub(super) fn flags_and_dialogs(save: &mut Value) {
    add_empty(save, &["flags", "dialogs"]);
}

/// Version 37 to 38: no escort's berth, as no older save kept it, so its
/// escorts load in fleet order.
fn escort_berths(save: &mut Value) {
    add_to_each(save, "escorts", "berth", &Value::Null);
}

/// Version 36 to 37: the class table unset, as a pilot who hasn't flown
/// holds it (`_InitObjects` @0x1cebd), since no older save kept it.
fn class_orders(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert(
            "class_orders".to_owned(),
            serde_json::json!([-1, -1, -1, -1]),
        );
    }
}

/// Version 35 to 36: no fighter's voice variant, as no older save kept it,
/// so a fighter from one holds its berth's word.
fn fighter_voices(save: &mut Value) {
    add_to_each(save, "escorts", "voice", &Value::Null);
}

/// Version 34 to 35: no stellar dominated, and every defence count its
/// decode, as no older save kept them.
fn dominations(save: &mut Value) {
    if let Some(stellars) = save
        .pointer_mut("/galaxy/stellars")
        .and_then(Value::as_array_mut)
    {
        for stellar in stellars.iter_mut().filter_map(Value::as_object_mut) {
            stellar.insert("dominated".to_owned(), Value::Bool(false));
        }
    }
    add_empty(save, &["defence_ships"]);
}

/// Version 33 to 34: no escort is marked for an upgrade or a sale, as
/// the escort dialog that marks them was not built.
fn escort_marks(save: &mut Value) {
    add_to_each(save, "escorts", "mark", &Value::Null);
}

/// Version 32 to 33: no nickname. Every older pilot was made with the New
/// Pilot dialog's Nickname field inert, so none had one.
fn nickname(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("nickname".to_owned(), Value::String(String::new()));
    }
}

/// Version 31 to 32: the course's start. Every writer of the original's
/// slot 0 but `M` and `N` leaves it at the player's system, so a course
/// with hops starts there, and no course has no start.
fn course_start(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        let has_hops = object
            .get("course")
            .and_then(Value::as_array)
            .is_some_and(|course| !course.is_empty());
        let start = if has_hops {
            object.get("system").cloned().unwrap_or(Value::Null)
        } else {
            Value::Null
        };
        object.insert("course_start".to_owned(), start);
    }
}

/// Version 30 to 31: the intro screen seen. An older save can't tell
/// whether its pilot ever entered play, and showing a long-played pilot
/// the intro is worse than skipping it for one never flown.
fn seen_intro(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("seen_intro".to_owned(), Value::Bool(true));
    }
}

/// Version 29 to 30: no reinforcement countdown. An older save kept
/// none, so every system's fleet can be called at once, as after a
/// countdown run out.
fn reinforcement_days(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("reinforcement_days".to_owned(), Value::Array(Vec::new()));
    }
}

/// Version 28 to 29: nothing charted. An older save kept only whether
/// each system had been explored, which every level-1 writer gives, so
/// each explored system loads at level 1.
fn charted(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("charted".to_owned(), Value::Array(Vec::new()));
    }
}

/// Version 27 to 28: no active mission's goals-met moment has come. The
/// upgrade reads no `mïsn`, so it can't tell goals met by a moment from
/// an escort goal's met at its return with none
/// ([`met_at_return`](nova_sim_story::mission::goals::met_at_return)); giving
/// `false` never holds back a `ShipDone` still to run, and at most lets
/// one already run run once more, and only under the other reading of
/// [`RuleKey::ShipDoneOnce`](nova_sim_base::rulebook::RuleKey::ShipDoneOnce).
fn ship_done(save: &mut Value) {
    let Some(entries) = save.get_mut("missions").and_then(Value::as_array_mut) else {
        return;
    };
    for ships in entries
        .iter_mut()
        .filter_map(|entry| entry.get_mut("ships")?.as_object_mut())
    {
        ships.insert("ship_done".to_owned(), Value::Bool(false));
    }
}

/// Version 26 to 27: no system jumped from, so mission ships coming in
/// before the next jump enter at a drawn angle.
fn jumped_from(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("jumped_from".to_owned(), Value::Null);
    }
}

/// Version 25 to 26: no active mission's ships keep a type, so a
/// mission of `Flags` 0x0800 draws it at its ships' next spawn.
fn uniform_types(save: &mut Value) {
    let Some(entries) = save.get_mut("missions").and_then(Value::as_array_mut) else {
        return;
    };
    for ships in entries
        .iter_mut()
        .filter_map(|entry| entry.get_mut("ships")?.as_object_mut())
    {
        ships.insert("uniform_type".to_owned(), Value::Null);
    }
}

/// Version 24 to 25: the pilot does not play Strict Play, as every older
/// pilot was made with the box greyed and clear.
fn strict_play(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("strict_play".to_owned(), Value::Bool(false));
    }
}

/// Version 1 to 2: nothing explored, and no legal records.
fn explored_and_legal(save: &mut Value) {
    add_empty(save, &["explored", "legal"]);
}

/// Version 2 to 3: no cargo, and no events under way.
fn cargo_and_events(save: &mut Value) {
    add_empty(save, &["cargo", "events"]);
}

/// Version 3 to 4: the ship's default items, not yet read.
fn default_outfits(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("outfits".to_owned(), Value::Null);
    }
}

/// Version 4 to 5: no escorts.
fn escorts(save: &mut Value) {
    add_empty(save, &["escorts"]);
}

/// Version 5 to 6: no escort has a standing order.
fn escort_orders(save: &mut Value) {
    add_to_each(save, "escorts", "order", &Value::Null);
}

/// Version 6 to 7: no escort is a carried fighter.
fn carried_fighters(save: &mut Value) {
    add_to_each(save, "escorts", "carried", &Value::Bool(false));
}

/// Version 7 to 8: no escort was hired, so none has a wage.
fn hired_escorts(save: &mut Value) {
    add_to_each(save, "escorts", "wage", &Value::Null);
}

/// Version 8 to 9: every person alive, and no grudges.
fn persons(save: &mut Value) {
    add_empty(save, &["gone_persons", "grudges"]);
}

/// Version 9 to 10: no escort is a person.
fn person_escorts(save: &mut Value) {
    add_to_each(save, "escorts", "person", &Value::Null);
}

/// Version 10 to 11: the ship's stock weapons, not yet fitted as outfits.
fn stock_weapons(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("stock_weapons_fitted".to_owned(), Value::Bool(false));
    }
}

/// Version 10 to 11: every control bit clear, and the player male.
fn bits_and_gender(save: &mut Value) {
    add_empty(save, &["bits"]);
    if let Some(object) = save.as_object_mut() {
        object.insert("gender".to_owned(), Value::from("male"));
    }
}

/// Version 11 to 12: the ship unpainted.
fn paint(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("paint".to_owned(), Value::Null);
    }
}

/// Version 12 to 13: the ship not yet named.
fn ship_names(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("ship_name".to_owned(), Value::Null);
    }
}

/// Version 14 to 15: every event at its record's own stellar.
fn event_stellars(save: &mut Value) {
    add_to_each(save, "events", "stellar", &Value::Null);
}

/// Version 15 to 16: no once-only story subscription has fired.
fn story_fired(save: &mut Value) {
    add_empty(save, &["story_fired"]);
}

/// Version 16 to 17: the galaxy as the catalog has it.
fn galaxy(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert(
            "galaxy".to_owned(),
            serde_json::json!({"systems": [], "stellars": [], "nebulae": []}),
        );
    }
}

/// Version 17 to 18: a destroyed stellar due to regenerate on its next
/// day, one not destroyed with no countdown.
fn countdowns(save: &mut Value) {
    let Some(stellars) = save
        .pointer_mut("/galaxy/stellars")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for stellar in stellars {
        if let Some(destroyed) = stellar.get_mut("destroyed") {
            *destroyed = match destroyed {
                Value::Bool(true) => serde_json::json!({"days_left": 0}),
                _ => Value::Null,
            };
        }
    }
}

/// Version 18 to 19: a combat rating of 0.
fn combat_rating(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("combat_rating".to_owned(), Value::from(0));
    }
}

/// Version 19 to 20: no mission active.
fn active_missions(save: &mut Value) {
    add_empty(save, &["missions"]);
}

/// Version 20 to 21: the records in each system not yet derived.
fn system_records(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("system_legal".to_owned(), Value::Null);
    }
}

/// Version 21 to 22: each active mission with no special ships left,
/// nowhere, with no timer and no name, none of them destroyed, boarded,
/// disabled or chased off; and no auxiliary ship left to come, none
/// arrived, its countdown 0.
fn mission_ships(save: &mut Value) {
    let none = serde_json::json!({
        "system": null,
        "left": 0,
        "arrived": 0,
        "appear": -1,
        "name": "",
        "subtitle": "",
        "destroyed": 0,
        "boarded": 0,
        "disabled": 0,
        "chased_off": 0
    });
    add_to_each(save, "missions", "ships", &none);
    let no_aux = serde_json::json!({"countdown": 0, "arrived": 0, "left": 0});
    add_to_each(save, "missions", "aux", &no_aux);
}

/// Version 22 to 23: no crön running or holding off.
fn crons(save: &mut Value) {
    add_empty(save, &["crons"]);
}

/// Version 23 to 24: no active rank.
fn ranks(save: &mut Value) {
    add_empty(save, &["ranks"]);
}

/// Adds `field` to every entry of `save`'s `list`, as `value`.
pub(super) fn add_to_each(save: &mut Value, list: &str, field: &str, value: &Value) {
    let Some(entries) = save.get_mut(list).and_then(Value::as_array_mut) else {
        return;
    };
    for entry in entries {
        if let Some(entry) = entry.as_object_mut() {
            entry.insert(field.to_owned(), value.clone());
        }
    }
}

/// Adds each of `fields` to `save` as an empty list.
fn add_empty(save: &mut Value, fields: &[&str]) {
    if let Some(object) = save.as_object_mut() {
        for field in fields {
            object.insert((*field).to_owned(), Value::Array(Vec::new()));
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::super::tests::{
        VERSION_1, active, at_version_25, at_version_26, at_version_27, at_version_28,
        at_version_29, at_version_30, at_version_31, at_version_32, at_version_33, at_version_34,
        dominating, fired, marked, seasoned, unusable, with_a_fighter,
    };
    use super::super::*;
    use super::*;
    use nova_sim_ai::escort::EscortOrder;
    use nova_sim_base::reserves::{Gauge, Reserves};
    use nova_sim_catalog::catalog::{
        DisasterId, GovtId, OutfitId, PersonId, ShipId, StellarId, SystemId,
    };
    use nova_sim_catalog::control::ControlBitSet;
    use nova_sim_combat::fleet::OrderWord;
    use nova_sim_state::date::GameDate;
    use nova_sim_state::market::Good;
    use nova_sim_story::pilot::{Escort, ExploredLevel};
    use nova_sim_story::story::{Consumer, Countdown, GalaxyOverlay};

    /// [`seasoned`] as a save from before version 37 loads it: its class
    /// table unset.
    fn unflown() -> Pilot {
        Pilot {
            class_orders: [OrderWord::NONE; 4],
            ..seasoned()
        }
    }

    #[test]
    fn a_version_1_save_loads_with_nothing_explored_and_no_records() {
        let pilot = decode(VERSION_1).expect("loads");
        assert_eq!(pilot.name(), "Old Timer");
        assert_eq!(pilot.ship(), ShipId(128));
        assert_eq!(pilot.system(), SystemId(130));
        assert_eq!(pilot.stellar(), None);
        assert_eq!(pilot.date(), GameDate::new(1177, 6, 24).expect("a date"));
        assert_eq!(pilot.cash(), 1000);
        assert_eq!(
            pilot.reserves().fuel,
            Gauge {
                now: 200.0,
                max: 300.0
            }
        );
        assert_eq!(pilot.course(), [SystemId(131)]);
        assert_eq!(pilot.explored().count(), 0);
        assert_eq!(pilot.legal_records().count(), 0);
    }

    /// A version 2 save: before cargo and events.
    const VERSION_2: &str = r#"{
        "version": 2,
        "name": "Trader",
        "ship": 128,
        "system": 130,
        "stellar": 140,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 2500,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [130],
        "legal": [{"govt": 128, "record": 5}]
    }"#;

    #[test]
    fn a_version_2_save_loads_with_no_cargo_and_no_events() {
        let pilot = decode(VERSION_2).expect("loads");
        assert_eq!(pilot.name(), "Trader");
        assert_eq!(pilot.stellar(), Some(StellarId(140)));
        assert_eq!(pilot.cash(), 2500);
        assert_eq!(pilot.explored().collect::<Vec<_>>(), [SystemId(130)]);
        assert_eq!(pilot.legal_record(GovtId(128)), 5);
        assert_eq!(pilot.cargo().count(), 0);
        assert_eq!(pilot.events().count(), 0);
        let old = decode(VERSION_1).expect("loads");
        assert_eq!((old.cargo().count(), old.events().count()), (0, 0));
    }

    /// A version 3 save: before outfits.
    const VERSION_3: &str = r#"{
        "version": 3,
        "name": "Hauler",
        "ship": 128,
        "system": 130,
        "stellar": null,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 300,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [130],
        "legal": [],
        "cargo": [{"good": {"commodity": 0}, "tons": 4}],
        "events": []
    }"#;

    #[test]
    fn an_older_save_loads_with_the_ships_default_items_pending() {
        for text in [VERSION_3, VERSION_2, VERSION_1] {
            let pilot = decode(text).expect("loads");
            assert!(pilot.default_outfits_pending, "{text}");
            assert_eq!(pilot.outfits().count(), 0, "{text}");
        }
        let pilot = decode(VERSION_3).expect("loads");
        assert_eq!(pilot.held(Good::Commodity(0)), 4);
        // Saved again before it flies, it still says so.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["outfits"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    /// A version 4 save: before escorts.
    const VERSION_4: &str = r#"{
        "version": 4,
        "name": "Privateer",
        "ship": 129,
        "system": 131,
        "stellar": null,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 128, "count": 2}]
    }"#;

    #[test]
    fn a_version_4_save_loads_with_an_empty_fleet() {
        let pilot = decode(VERSION_4).expect("loads");
        assert_eq!(pilot.name(), "Privateer");
        assert_eq!(pilot.ship(), ShipId(129));
        assert_eq!(pilot.owned(OutfitId(128)), 2);
        assert!(!pilot.default_outfits_pending);
        assert_eq!(pilot.escorts(), []);
        // Saved again, it is a current save with no escorts.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["escorts"], serde_json::json!([]));
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    /// A version 5 save: before standing orders.
    const VERSION_5: &str = r#"{
        "version": 5,
        "name": "Admiral",
        "ship": 129,
        "system": 131,
        "stellar": null,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [],
        "escorts": [
            {
                "ship": 130,
                "reserves": {
                    "shield": {"now": 5.0, "max": 50.0},
                    "armor": {"now": 20.0, "max": 40.0},
                    "fuel": {"now": 100.0, "max": 200.0}
                }
            },
            {
                "ship": 141,
                "reserves": {
                    "shield": {"now": 1000.0, "max": 1000.0},
                    "armor": {"now": 500.0, "max": 500.0},
                    "fuel": {"now": 400.0, "max": 400.0}
                }
            }
        ]
    }"#;

    #[test]
    fn a_version_5_saves_escorts_load_with_no_standing_order() {
        let pilot = decode(VERSION_5).expect("loads");
        assert_eq!(pilot.name(), "Admiral");
        assert_eq!(
            pilot.escorts(),
            [
                Escort {
                    ship: ShipId(130),
                    reserves: Reserves {
                        shield: Gauge {
                            now: 5.0,
                            max: 50.0
                        },
                        armor: Gauge {
                            now: 20.0,
                            max: 40.0
                        },
                        fuel: Gauge {
                            now: 100.0,
                            max: 200.0
                        },
                    },
                    order: None,
                    carried: false,
                    wage: None,
                    person: None,
                    mark: None,
                    voice: None,
                    berth: None,
                },
                Escort {
                    ship: ShipId(141),
                    reserves: Reserves::full(1000.0, 500.0, 400.0),
                    order: None,
                    carried: false,
                    wage: None,
                    person: None,
                    mark: None,
                    voice: None,
                    berth: None,
                },
            ]
        );
        // Saved again, it is a current save whose escorts say so.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["escorts"][0]["order"], serde_json::Value::Null);
        assert_eq!(value["escorts"][1]["order"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    /// A version 6 save: before carried fighters.
    const VERSION_6: &str = r#"{
        "version": 6,
        "name": "Commodore",
        "ship": 129,
        "system": 131,
        "stellar": null,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 158, "count": 3}],
        "escorts": [
            {
                "ship": 130,
                "reserves": {
                    "shield": {"now": 5.0, "max": 50.0},
                    "armor": {"now": 20.0, "max": 40.0},
                    "fuel": {"now": 100.0, "max": 200.0}
                },
                "order": "attack"
            },
            {
                "ship": 141,
                "reserves": {
                    "shield": {"now": 1000.0, "max": 1000.0},
                    "armor": {"now": 500.0, "max": 500.0},
                    "fuel": {"now": 400.0, "max": 400.0}
                },
                "order": null
            }
        ]
    }"#;

    #[test]
    fn a_version_6_saves_escorts_load_as_escorts_not_carried_fighters() {
        let pilot = decode(VERSION_6).expect("loads");
        assert_eq!(pilot.name(), "Commodore");
        assert_eq!(pilot.owned(OutfitId(158)), 3);
        assert_eq!(
            pilot
                .escorts()
                .iter()
                .map(|escort| (escort.ship, escort.order, escort.carried))
                .collect::<Vec<_>>(),
            [
                (ShipId(130), Some(EscortOrder::Attack), false),
                (ShipId(141), None, false)
            ]
        );
        assert_eq!(pilot.escort_count(), 2);
        // Saved again, it is a current save whose escorts say so.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["escorts"][0]["carried"], false);
        assert_eq!(value["escorts"][1]["carried"], false);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1] {
            assert!(decode(text).is_ok(), "{text}");
        }
        let fleet = decode(VERSION_5).expect("loads");
        assert!(fleet.escorts().iter().all(|escort| !escort.carried));
    }

    /// A version 7 save: before hired escorts' wages.
    const VERSION_7: &str = r#"{
        "version": 7,
        "name": "Fleet Admiral",
        "ship": 129,
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 158, "count": 2}],
        "escorts": [
            {
                "ship": 130,
                "reserves": {
                    "shield": {"now": 5.0, "max": 50.0},
                    "armor": {"now": 20.0, "max": 40.0},
                    "fuel": {"now": 100.0, "max": 200.0}
                },
                "order": "hold",
                "carried": false
            },
            {
                "ship": 144,
                "reserves": {
                    "shield": {"now": 60.0, "max": 60.0},
                    "armor": {"now": 40.0, "max": 40.0},
                    "fuel": {"now": 300.0, "max": 300.0}
                },
                "order": null,
                "carried": true
            }
        ]
    }"#;

    #[test]
    fn a_version_7_saves_escorts_load_as_not_hired() {
        let pilot = decode(VERSION_7).expect("loads");
        assert_eq!(pilot.name(), "Fleet Admiral");
        assert_eq!(
            pilot
                .escorts()
                .iter()
                .map(|escort| (escort.ship, escort.order, escort.carried, escort.wage))
                .collect::<Vec<_>>(),
            [
                (ShipId(130), Some(EscortOrder::Hold), false, None),
                (ShipId(144), None, true, None)
            ]
        );
        assert!(pilot.escorts().iter().all(|escort| !escort.hired()));
        // Saved again, it is a current save whose escorts say so.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["escorts"][0]["wage"], serde_json::Value::Null);
        assert_eq!(value["escorts"][1]["wage"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert!(
                pilot.escorts().iter().all(|escort| escort.wage.is_none()),
                "{text}"
            );
        }
    }

    /// A version 8 save: before the persons gone and the grudges.
    const VERSION_8: &str = r#"{
        "version": 8,
        "name": "Mercenary",
        "ship": 129,
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [],
        "escorts": [
            {
                "ship": 128,
                "reserves": {
                    "shield": {"now": 30.0, "max": 30.0},
                    "armor": {"now": 45.0, "max": 45.0},
                    "fuel": {"now": 300.0, "max": 300.0}
                },
                "order": null,
                "carried": false,
                "wage": 100
            }
        ]
    }"#;

    #[test]
    fn a_version_8_save_loads_with_every_person_alive_and_no_grudge() {
        let pilot = decode(VERSION_8).expect("loads");
        assert_eq!(pilot.name(), "Mercenary");
        assert_eq!(pilot.escorts()[0].wage, Some(100));
        assert_eq!(pilot.gone_persons().count(), 0);
        assert_eq!(pilot.grudges().count(), 0);
        // Saved again, it is a current save that says so.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["gone_persons"], serde_json::json!([]));
        assert_eq!(value["grudges"], serde_json::json!([]));
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_7, VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert_eq!(
                (pilot.gone_persons().count(), pilot.grudges().count()),
                (0, 0),
                "{text}"
            );
        }
    }

    /// A version 9 save: before an escort's person.
    const VERSION_9: &str = r#"{
        "version": 9,
        "name": "Mercenary",
        "ship": 129,
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [],
        "escorts": [
            {
                "ship": 128,
                "reserves": {
                    "shield": {"now": 30.0, "max": 30.0},
                    "armor": {"now": 45.0, "max": 45.0},
                    "fuel": {"now": 300.0, "max": 300.0}
                },
                "order": null,
                "carried": false,
                "wage": 100
            }
        ],
        "gone_persons": [151],
        "grudges": [510]
    }"#;

    #[test]
    fn a_version_9_saves_escorts_load_as_no_person() {
        let pilot = decode(VERSION_9).expect("loads");
        assert_eq!(pilot.name(), "Mercenary");
        assert_eq!(pilot.escorts()[0].person, None);
        assert_eq!(pilot.escorts()[0].wage, Some(100));
        assert!(pilot.gone(PersonId(151)));
        assert!(pilot.grudge(PersonId(510)));
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["escorts"][0]["person"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_8, VERSION_7, VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert!(
                pilot.escorts().iter().all(|escort| escort.person.is_none()),
                "{text}"
            );
        }
    }

    /// A version 10 save: before the control bits and gender.
    const VERSION_10: &str = r#"{
        "version": 10,
        "name": "Wanderer",
        "ship": 129,
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [],
        "escorts": [
            {
                "ship": 128,
                "reserves": {
                    "shield": {"now": 30.0, "max": 30.0},
                    "armor": {"now": 45.0, "max": 45.0},
                    "fuel": {"now": 300.0, "max": 300.0}
                },
                "order": null,
                "carried": false,
                "wage": 100,
                "person": 128
            }
        ],
        "gone_persons": [151],
        "grudges": []
    }"#;

    #[test]
    fn a_version_10_save_loads_with_every_bit_clear_and_male() {
        let pilot = decode(VERSION_10).expect("loads");
        assert_eq!(pilot.name(), "Wanderer");
        assert_eq!(pilot.escorts()[0].person, Some(PersonId(128)));
        assert!(pilot.gone(PersonId(151)));
        assert_eq!(pilot.control_bits(), &ControlBitSet::new());
        assert_eq!(pilot.gender(), Gender::Male);
        // Saved again, it is a current save that says so.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["bits"], serde_json::json!([]));
        assert_eq!(value["gender"], "male");
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_9, VERSION_8, VERSION_7, VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2,
            VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.control_bits().iter().count(), 0, "{text}");
            assert_eq!(pilot.gender(), Gender::Male, "{text}");
        }
    }

    /// A version 11 save: before the ship's paint.
    const VERSION_11: &str = r#"{
        "version": 11,
        "name": "Painter",
        "ship": 129,
        "system": 131,
        "stellar": null,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [],
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female"
    }"#;

    #[test]
    fn a_version_11_save_loads_unpainted() {
        let pilot = decode(VERSION_11).expect("loads");
        assert_eq!(pilot.name(), "Painter");
        assert_eq!(pilot.gender(), Gender::Female);
        assert_eq!(pilot.paint(), None);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["paint"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [VERSION_10, VERSION_5, VERSION_1] {
            assert_eq!(decode(text).expect("loads").paint(), None, "{text}");
        }
    }

    /// A version 12 save: before the ship had a name.
    const VERSION_12: &str = r#"{
        "version": 12,
        "name": "Veteran",
        "ship": 129,
        "system": 131,
        "stellar": null,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 200, "count": 1}],
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": {"r": 1, "g": 2, "b": 3}
    }"#;

    #[test]
    fn a_version_12_save_loads_with_its_ship_not_yet_named() {
        let pilot = decode(VERSION_12).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert_eq!(pilot.ship_name(), None);
        assert_eq!(pilot.paint(), Some(Rgb15 { r: 1, g: 2, b: 3 }));
        assert_eq!(pilot.owned(OutfitId(200)), 1);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["ship_name"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [VERSION_11, VERSION_10, VERSION_5, VERSION_1] {
            assert_eq!(decode(text).expect("loads").ship_name(), None, "{text}");
        }
    }

    #[test]
    fn a_version_24_save_loads_without_strict_play() {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_25(&seasoned().with_strict_play(true))).expect("JSON");
        let object = value.as_object_mut().expect("an object");
        object.insert("version".to_owned(), serde_json::Value::from(24));
        object.remove("strict_play");
        let pilot = decode(&value.to_string()).expect("loads");
        assert!(!pilot.strict_play());
        assert_eq!(pilot, without_uniform_types(unflown()));
    }

    /// A version 13 save: before stock weapons were outfits.
    const VERSION_13: &str = r#"{
        "version": 13,
        "name": "Veteran",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 200, "count": 1}],
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": null
    }"#;

    #[test]
    fn a_version_13_save_loads_with_its_stock_weapons_pending() {
        let pilot = decode(VERSION_13).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert!(pilot.stock_weapons_pending);
        assert!(!pilot.default_outfits_pending);
        assert_eq!(pilot.owned(OutfitId(200)), 1);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["stock_weapons_fitted"], false);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_12, VERSION_11, VERSION_10, VERSION_9, VERSION_8, VERSION_7, VERSION_6,
            VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            assert!(decode(text).expect("loads").stock_weapons_pending, "{text}");
        }
        assert!(
            !decode(&encode(&seasoned()))
                .expect("loads")
                .stock_weapons_pending
        );
    }

    #[test]
    fn every_save_from_before_ship_names_loads_with_its_ship_not_yet_named() {
        let pilot = decode(VERSION_12).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert_eq!(pilot.ship_name(), None);
        assert_eq!(pilot.owned(OutfitId(200)), 1);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["ship_name"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_11, VERSION_10, VERSION_9, VERSION_8, VERSION_7, VERSION_6, VERSION_5,
            VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            assert_eq!(decode(text).expect("loads").ship_name(), None, "{text}");
        }
    }

    /// A version 14 save: before events kept the stellar they are at.
    const VERSION_14: &str = r#"{
        "version": 14,
        "name": "Veteran",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [{"disaster": 128, "days": 12}],
        "outfits": [{"outfit": 200, "count": 1}],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": null
    }"#;

    #[test]
    fn a_version_14_save_loads_its_events_at_their_records_stellar() {
        let pilot = decode(VERSION_14).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert_eq!(pilot.ship_name(), Some("Kestrel"));
        assert_eq!(
            pilot.events,
            BTreeMap::from([(DisasterId(128), active(12, None))])
        );
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(
            value["events"],
            serde_json::json!([{"disaster": 128, "days": 12, "stellar": null}])
        );
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_13, VERSION_12, VERSION_11, VERSION_10, VERSION_9, VERSION_8, VERSION_7,
            VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            assert_eq!(decode(text).expect("loads").events().count(), 0, "{text}");
        }
    }

    /// A version 15 save: before the story's once-fired subscriptions.
    const VERSION_15: &str = r#"{
        "version": 15,
        "name": "Veteran",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [{"disaster": 128, "days": 12, "stellar": 150}],
        "outfits": [{"outfit": 200, "count": 1}],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": null
    }"#;

    #[test]
    fn a_version_15_save_loads_with_no_story_subscription_fired() {
        let pilot = decode(VERSION_15).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert_eq!(
            pilot.events,
            BTreeMap::from([(DisasterId(128), active(12, Some(StellarId(150))))])
        );
        assert_eq!(pilot.story_fired, BTreeSet::new());
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["story_fired"], serde_json::json!([]));
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_14, VERSION_13, VERSION_12, VERSION_11, VERSION_10, VERSION_9, VERSION_8,
            VERSION_7, VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.story_fired, BTreeSet::new(), "{text}");
        }
    }

    /// A version 16 save: before the galaxy overlay.
    const VERSION_16: &str = r#"{
        "version": 16,
        "name": "Veteran",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [132],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 200, "count": 1}],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": null,
        "story_fired": [{"consumer": "missions", "record": 3, "hook": 1}]
    }"#;

    #[test]
    fn a_version_16_save_loads_with_the_galaxy_as_the_catalog_has_it() {
        let pilot = decode(VERSION_16).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert_eq!(pilot.course(), [SystemId(132)]);
        assert_eq!(
            pilot.story_fired,
            BTreeSet::from([fired(Consumer::Missions, 3, 1)])
        );
        assert_eq!(pilot.galaxy, GalaxyOverlay::default());
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(
            value["galaxy"],
            serde_json::json!({"systems": [], "stellars": [], "nebulae": []})
        );
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_15, VERSION_14, VERSION_13, VERSION_12, VERSION_11, VERSION_10, VERSION_9,
            VERSION_8, VERSION_7, VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.galaxy, GalaxyOverlay::default(), "{text}");
        }
    }

    /// A version 17 save: a destroyed stellar held no countdown.
    const VERSION_17: &str = r#"{
        "version": 17,
        "name": "Veteran",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [132],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 200, "count": 1}],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": null,
        "story_fired": [],
        "galaxy": {
            "systems": [{"system": 131, "visible": false, "govt": null}],
            "stellars": [
                {"stellar": 150, "govt": {"govt": 128}, "destroyed": false},
                {"stellar": 151, "govt": null, "destroyed": true}
            ],
            "nebulae": []
        }
    }"#;

    #[test]
    fn a_version_17_saves_destroyed_stellar_is_due_to_regenerate_on_its_next_day() {
        let pilot = decode(VERSION_17).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        let mut expected = GalaxyOverlay::default();
        expected.set_visible(SystemId(131), Some(false));
        expected.set_stellar_govt(StellarId(150), Some(Some(GovtId(128))));
        expected.set_destroyed(StellarId(151), Some(Countdown { days_left: 0 }));
        assert_eq!(pilot.galaxy, expected);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(
            value["galaxy"]["stellars"],
            serde_json::json!([
                {"stellar": 150, "govt": {"govt": 128}, "destroyed": null, "dominated": false},
                {"stellar": 151, "govt": null, "destroyed": {"days_left": 0}, "dominated": false}
            ])
        );
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_16, VERSION_15, VERSION_14, VERSION_13, VERSION_12, VERSION_11, VERSION_10,
            VERSION_9, VERSION_8, VERSION_7, VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2,
            VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.galaxy, GalaxyOverlay::default(), "{text}");
        }
    }

    /// A version 18 save: before the combat rating.
    const VERSION_18: &str = r#"{
        "version": 18,
        "name": "Veteran",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [132],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 200, "count": 1}],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": null,
        "story_fired": [],
        "galaxy": {
            "systems": [],
            "stellars": [{"stellar": 151, "govt": null, "destroyed": {"days_left": 2}}],
            "nebulae": []
        }
    }"#;

    /// A version 19 save: before the active missions.
    const VERSION_19: &str = r#"{
        "version": 19,
        "name": "Courier",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "combat_rating": 7,
        "cargo": [],
        "events": [],
        "outfits": [],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [],
        "gender": "male",
        "paint": null,
        "story_fired": [],
        "galaxy": {"systems": [], "stellars": [], "nebulae": []}
    }"#;

    /// A version 20 save: before the legal record in each system.
    const VERSION_20: &str = r#"{
        "version": 20,
        "name": "Lawful",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [{"govt": 128, "record": 25}],
        "combat_rating": 7,
        "cargo": [],
        "events": [],
        "outfits": [],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [],
        "gender": "male",
        "paint": null,
        "story_fired": [],
        "galaxy": {"systems": [], "stellars": [], "nebulae": []},
        "missions": []
    }"#;

    #[test]
    fn a_version_20_save_loads_with_its_system_records_pending() {
        let pilot = decode(VERSION_20).expect("loads");
        assert_eq!(pilot.name(), "Lawful");
        assert!(!pilot.system_records_derived());
        assert_eq!(pilot.legal_record(GovtId(128)), 25, "kept");
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["system_legal"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [VERSION_19, VERSION_18, VERSION_1] {
            let pilot = decode(text).expect("loads");
            assert!(!pilot.system_records_derived(), "{text}");
        }
    }

    /// A version 22 save: before the crons.
    const VERSION_22: &str = r#"{
        "version": 22,
        "name": "Lawful",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [{"govt": 128, "record": 25}],
        "system_legal": null,
        "combat_rating": 7,
        "cargo": [],
        "events": [],
        "outfits": [],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [],
        "gender": "male",
        "paint": null,
        "story_fired": [],
        "galaxy": {"systems": [], "stellars": [], "nebulae": []},
        "missions": []
    }"#;

    #[test]
    fn an_older_save_loads_with_no_cron_running() {
        let pilot = decode(VERSION_22).expect("loads");
        assert_eq!(pilot.name(), "Lawful");
        assert!(pilot.crons().is_empty());
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["crons"], serde_json::json!([]));
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [VERSION_21, VERSION_20, VERSION_19, VERSION_1] {
            let pilot = decode(text).expect("loads");
            assert!(pilot.crons().is_empty(), "{text}");
        }
    }

    /// A version 23 save: before the ranks.
    const VERSION_23: &str = r#"{
        "version": 23,
        "name": "Ranked",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [{"govt": 128, "record": 25}],
        "system_legal": null,
        "combat_rating": 7,
        "cargo": [],
        "events": [],
        "outfits": [],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [],
        "gender": "male",
        "paint": null,
        "story_fired": [],
        "galaxy": {"systems": [], "stellars": [], "nebulae": []},
        "missions": [],
        "crons": [{"cron": 156, "days": 105, "holdoff": 2}]
    }"#;

    #[test]
    fn a_version_23_save_upgrades_with_no_rank() {
        let pilot = decode(VERSION_23).expect("loads");
        assert_eq!(pilot.name(), "Ranked");
        assert_eq!(pilot.crons().len(), 1, "the rest kept");
        assert!(pilot.ranks().is_empty());
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["ranks"], serde_json::json!([]));
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [VERSION_22, VERSION_21, VERSION_1] {
            let pilot = decode(text).expect("loads");
            assert!(pilot.ranks().is_empty(), "{text}");
        }
    }

    #[test]
    fn an_older_save_loads_with_no_active_missions() {
        let pilot = decode(VERSION_19).expect("loads");
        assert_eq!(pilot.name(), "Courier");
        assert_eq!(pilot.combat_rating(), 7);
        assert_eq!(pilot.missions(), &ActiveMissions::default());
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["missions"], serde_json::json!([]));
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [VERSION_18, VERSION_17, VERSION_1] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.missions(), &ActiveMissions::default(), "{text}");
        }
    }

    /// A version 21 save: before the active missions' special ships.
    const VERSION_21: &str = r#"{
        "version": 21,
        "name": "Hunter",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "system_legal": null,
        "combat_rating": 7,
        "cargo": [],
        "events": [],
        "outfits": [],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [],
        "gender": "male",
        "paint": null,
        "story_fired": [],
        "galaxy": {"systems": [], "stellars": [], "nebulae": []},
        "missions": [{
            "slot": 4,
            "mission": 140,
            "destination": null,
            "return": null,
            "cargo_type": null,
            "cargo_tons": 0,
            "pay": 500,
            "days_left": null,
            "deadline": null,
            "reached": false,
            "goals_met": false,
            "aboard": false,
            "failed": null
        }]
    }"#;

    #[test]
    fn a_version_21_saves_missions_load_with_no_special_ships() {
        let pilot = decode(VERSION_21).expect("loads");
        let active = pilot.missions().get(4).expect("held");
        assert_eq!(active.mission, MissionId(140));
        assert_eq!(active.ships, SpecialShips::default());
        assert_eq!(active.ships.system, ShipSystem::Nowhere);
        assert_eq!(active.ships.chased_off, 0);
        assert_eq!(active.aux, AuxShips::default(), "none to come");
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn a_version_33_save_loads_with_no_escort_marked_and_saves_at_the_current_version() {
        let pilot = decode(&at_version_33(&marked())).expect("loads");
        assert!(pilot.escorts.iter().all(|escort| escort.mark.is_none()));
        assert_eq!(pilot, unflown());
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["escorts"][1]["mark"], serde_json::Value::Null);
    }

    #[test]
    fn a_version_34_save_loads_with_none_dominated_and_the_decoded_counts() {
        let pilot = decode(&at_version_34(&dominating())).expect("loads");
        assert_eq!(pilot, unflown());
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["galaxy"]["stellars"][0]["dominated"], false);
        assert_eq!(value["defence_ships"], serde_json::json!([]));
    }

    #[test]
    fn the_upgrade_from_34_adds_dominated_false_and_no_defence_ships() {
        let mut value: serde_json::Value = serde_json::json!({
            "galaxy": {"systems": [], "stellars": [{"stellar": 150}], "nebulae": []}
        });
        dominations(&mut value);
        assert_eq!(
            value,
            serde_json::json!({
                "galaxy": {
                    "systems": [],
                    "stellars": [{"stellar": 150, "dominated": false}],
                    "nebulae": []
                },
                "defence_ships": []
            })
        );
    }

    #[test]
    fn a_version_32_save_loads_with_no_nickname_and_saves_at_the_current_version() {
        let nicknamed = seasoned().with_nickname("Hunter".to_owned());
        let pilot = decode(&at_version_32(&nicknamed)).expect("loads");
        assert_eq!(pilot.nickname(), "");
        assert_eq!(pilot, unflown());
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["nickname"], "");
    }

    #[test]
    fn a_version_31_save_starts_a_course_with_hops_at_the_pilots_system() {
        let elsewhere = Pilot {
            course_start: Some(SystemId(200)),
            ..seasoned()
        };
        let pilot = decode(&at_version_31(&elsewhere)).expect("loads");
        assert_eq!(pilot.course_start(), Some(SystemId(131)));
        assert_eq!(pilot, unflown());
    }

    #[test]
    fn a_version_31_save_with_no_course_has_no_start() {
        let idle = Pilot {
            course: Vec::new(),
            course_start: Some(SystemId(131)),
            ..seasoned()
        };
        let pilot = decode(&at_version_31(&idle)).expect("loads");
        assert_eq!(pilot.course_start(), None);
    }

    #[test]
    fn a_version_30_save_loads_with_the_intro_screen_seen() {
        let unseen = Pilot {
            seen_intro: false,
            ..seasoned()
        };
        let pilot = decode(&at_version_30(&unseen)).expect("loads");
        assert!(pilot.seen_intro());
        assert_eq!(pilot, unflown());
    }

    /// `pilot` with no reinforcement countdown, as a save from before
    /// version 30 loads it.
    fn without_reinforcement_days(mut pilot: Pilot) -> Pilot {
        pilot.reinforcement_days.clear();
        pilot
    }

    #[test]
    fn a_version_29_save_loads_with_no_reinforcement_countdown() {
        let pilot = decode(&at_version_29(&seasoned())).expect("loads");
        assert!(pilot.reinforcement_days.is_empty());
        assert_eq!(pilot, without_reinforcement_days(unflown()));
    }

    /// `pilot` with nothing charted and no reinforcement countdown, as a
    /// save from before version 29 loads it.
    fn uncharted(pilot: Pilot) -> Pilot {
        let mut pilot = without_reinforcement_days(pilot);
        pilot.charted.clear();
        pilot
    }

    #[test]
    fn a_version_28_save_loads_with_nothing_charted() {
        let pilot = decode(&at_version_28(&seasoned())).expect("loads");
        assert_eq!(pilot.charted().count(), 0);
        assert_eq!(pilot.explored_level(SystemId(131)), ExploredLevel::Explored);
        assert_eq!(pilot, uncharted(unflown()));
    }

    /// `pilot` with no mission's goals-met moment come and nothing
    /// charted, as a save from before version 28 loads it.
    fn without_ship_done(pilot: Pilot) -> Pilot {
        let mut pilot = uncharted(pilot);
        for slot in 0..16 {
            if let Some(active) = pilot.missions.get_mut(slot) {
                active.ships.ship_done = false;
            }
        }
        pilot
    }

    #[test]
    fn a_version_27_save_loads_with_no_missions_ship_done_whatever_its_goals_met() {
        let pilot = decode(&at_version_27(&seasoned())).expect("loads");
        let met_and_done = |slot| {
            pilot
                .missions()
                .get(slot)
                .map(|active| (active.goals_met, active.ships.ship_done))
        };
        assert_eq!(met_and_done(2), Some((false, false)));
        // Slot 9's goals are met with its destination not reached, as an
        // escort goal met at its return leaves them with no moment
        // (`met_at_return`): a goal-blind upgrade can't tell it from one
        // met by a moment, so it leaves the moment to come.
        assert_eq!(met_and_done(9), Some((true, false)));
        assert_eq!(pilot, without_ship_done(unflown()));
    }

    #[test]
    fn a_version_26_save_loads_with_no_jumped_from() {
        let pilot = decode(&at_version_26(&seasoned())).expect("loads");
        assert_eq!(
            pilot,
            Pilot {
                jumped_from: None,
                ..without_ship_done(unflown())
            }
        );
    }

    #[test]
    fn a_version_25_save_loads_with_no_uniform_type() {
        let pilot = decode(&at_version_25(&seasoned())).expect("loads");
        assert_eq!(
            pilot
                .missions()
                .get(2)
                .map(|active| active.ships.uniform_type),
            Some(None),
            "drawn at its next spawn"
        );
        assert_eq!(pilot, without_uniform_types(unflown()));
    }

    /// `pilot` with no mission's ships keeping a type, no system
    /// jumped from and no mission's ship done, as a save from before
    /// version 26 loads it.
    fn without_uniform_types(pilot: Pilot) -> Pilot {
        let mut pilot = without_ship_done(pilot);
        pilot.jumped_from = None;
        for slot in 0..16 {
            if let Some(active) = pilot.missions.get_mut(slot) {
                active.ships.uniform_type = None;
            }
        }
        pilot
    }

    #[test]
    fn an_older_save_loads_with_a_combat_rating_of_0() {
        let pilot = decode(VERSION_18).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert_eq!(pilot.combat_rating(), 0);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["combat_rating"], 0);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_17, VERSION_16, VERSION_15, VERSION_14, VERSION_13, VERSION_12, VERSION_11,
            VERSION_10, VERSION_9, VERSION_8, VERSION_7, VERSION_6, VERSION_5, VERSION_4,
            VERSION_3, VERSION_2, VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.combat_rating(), 0, "{text}");
        }
    }

    #[test]
    fn an_older_save_loads_with_an_empty_fleet() {
        for text in [VERSION_3, VERSION_2, VERSION_1] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.escorts(), [], "{text}");
        }
    }

    #[test]
    fn a_version_35_saves_escorts_load_with_no_variant() {
        let pilot = with_a_fighter(seasoned(), VoiceVariant::Odd);
        let mut value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        value["version"] = serde_json::Value::from(35);
        for escort in value["escorts"].as_array_mut().expect("escorts") {
            escort.as_object_mut().expect("an escort").remove("voice");
        }
        let decoded = decode(&value.to_string()).expect("loads");
        assert_eq!(decoded.escorts.len(), 2);
        assert!(decoded.escorts.iter().all(|escort| escort.voice.is_none()));
    }

    #[test]
    fn a_version_37_saves_escorts_load_with_no_berth() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value["version"] = serde_json::Value::from(37);
        for escort in value["escorts"].as_array_mut().expect("escorts") {
            escort.as_object_mut().expect("an escort").remove("berth");
        }
        assert_eq!(decode(&value.to_string()), Ok(seasoned()));
    }

    #[test]
    fn a_version_36_save_loads_as_a_pilot_who_has_not_flown() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value["version"] = serde_json::Value::from(36);
        value
            .as_object_mut()
            .expect("an object")
            .remove("class_orders");
        let pilot = decode(&value.to_string()).expect("loads");
        assert_eq!(pilot, unflown());
    }

    #[test]
    fn a_version_1_save_missing_a_field_is_unusable() {
        let text = VERSION_1.replace(r#""course": [131]"#, r#""cours": [131]"#);
        assert!(unusable(&text).contains("course"));
    }
}
