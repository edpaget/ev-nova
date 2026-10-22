//! The catalog data [`Session::load`](super::Session::load) reads once and
//! nothing writes again, shared by every clone of the session.

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::catalog::{
    CronRecord, DateAffixes, DudeId, ListedStellar, MissionId, MissionRecord, MissionTexts,
    NebulaRecord, OutfitId, OutfitRecord, PlaceNames, RankRecord, ShipRecord, StellarId,
    StellarLife, SystemId, SystemVisibility, WeaponId,
};
use crate::combat::armament::Arsenal;
use crate::govt::Governments;
use crate::market::Goods;
use crate::mission::StellarMarks;
use crate::traffic::table::SpawnDude;

#[cfg(test)]
use super::Session;

/// The catalog data a session reads as it starts, in `Session`'s former
/// field order.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Statics {
    /// Every `oütf`, read when the session starts.
    pub(super) outfits: Vec<OutfitRecord>,
    /// Every `shïp`, read when the session starts.
    pub(super) ships: Vec<ShipRecord>,
    /// The warp sounds' durations, in Mac ticks, read when the session
    /// starts: `snd ` 128's and 129's, `_jumpSoundLength`, of which
    /// [`PlayerShip::warp_sound`](super::PlayerShip::warp_sound) gives the
    /// d of the warp's clock
    /// ([`warp::sound_duration`](crate::warp::sound_duration)).
    pub(super) warp_sounds: [u32; 2],
    /// The warp-out's duration, in Mac ticks, read when the session
    /// starts from `snd ` 130 ([`warp::warp_out_duration`](crate::warp::warp_out_duration)).
    pub(super) warp_out_sound: u32,
    /// The escorts' voices, read when the session starts: every voice
    /// clip with its length and each government's voice (see the
    /// `voices` module).
    pub(super) voices: crate::voice::Voices,
    /// The goods traded and the events that move their prices, read when
    /// the session starts.
    pub(super) goods: Goods,
    /// What the date is wrapped in when it is displayed, read when the
    /// session starts.
    pub(super) date_affixes: DateAffixes,
    /// Every weapon and ship type's combat fields, read when the session
    /// starts.
    pub(super) arsenal: Arsenal,
    /// Every government and their relations, read when the session
    /// starts.
    pub(super) govts: Governments,
    /// Each ammunition outfit, with the weapon it is the rounds of.
    pub(super) ammo_outfits: Vec<(WeaponId, OutfitId)>,
    /// Every asteroid type, by kind, read when the session starts.
    pub(super) asteroid_types: Rc<[crate::asteroid::AsteroidType; crate::asteroid::KINDS]>,
    /// Each box graphic's frame size, read when the session starts, which
    /// a scoop tests ships against (see [`scoop`](crate::scoop)).
    pub(super) box_frames: crate::scoop::BoxFrames,
    /// Every system's asteroid field, read when the session starts; a
    /// system not here holds none.
    pub(super) asteroid_fields: BTreeMap<SystemId, crate::asteroid::FieldSpec>,
    /// Every stellar's life, by ID, read when the session starts: what `Y`
    /// and the daily step read of a stellar's destruction and
    /// regeneration (see the `stellars` module).
    pub(super) lives: BTreeMap<StellarId, StellarLife>,
    /// Graphic type 0's sheet's frame size, or `None` when it cannot be
    /// read, read from the catalog as the session starts: what a rebuild
    /// point fills an empty sprite slot with, and the box a click on that
    /// slot reads (see the `stellars` module).
    pub(super) fallback_frame: Option<(u32, u32)>,
    /// Every `mïsn`, read when the session starts.
    pub(super) missions: Vec<MissionRecord>,
    /// The `düde`s of every `mïsn` of `Flags` 0x0800 with a `ShipDude`
    /// that can be read, read when the session starts: what such a
    /// mission draws its ships' one type from at accept (see the
    /// `mission_ships` module).
    pub(super) uniform_dudes: BTreeMap<DudeId, SpawnDude>,
    /// Every `crön`, read when the session starts, shared so the daily
    /// pass holds them while their scripts run (see the `crons` module).
    pub(super) crons: Rc<[CronRecord]>,
    /// Every `ränk` 128-255, read when the session starts (see the
    /// `ranks` module).
    pub(super) ranks: Rc<[RankRecord]>,
    /// What the screens show of each `mïsn`, read when the session
    /// starts (see the `mission_screens` module).
    pub(super) mission_texts: BTreeMap<MissionId, MissionTexts>,
    /// Every stellar some system lists, read when the session starts:
    /// what a mission's stellars are chosen from, with the galaxy overlay.
    pub(super) listed_stellars: Vec<ListedStellar>,
    /// Every system's `Visibility`, read when the session starts: what the
    /// galaxy refresh evaluates (see the `story` module).
    pub(super) visibility: Rc<[SystemVisibility]>,
    /// Every `nëbu` 128-159, read when the session starts: their
    /// `ActiveOn`, which the galaxy refresh evaluates, and their
    /// `OnExplore` (see the `nebulae` module).
    pub(super) nebulae: Rc<[NebulaRecord]>,
    /// Every stellar's and system's name, read when the session starts:
    /// what the text tags read (see the `missions` module).
    pub(super) place_names: PlaceNames,
    /// Each system's first stellar that can be read, read when the session
    /// starts: what a landed `M` selects (see the `nav` module).
    pub(super) first_stellars: BTreeMap<SystemId, StellarId>,
    /// Every stellar's place and name, read when the session starts: what
    /// a mission's stop is matched by (see the `missions` module).
    pub(super) stellar_marks: StellarMarks,
}

#[cfg(test)]
impl Session {
    /// The session's catalog data, to change in a test: unshared from
    /// every clone first ([`Rc::make_mut`]).
    pub(super) fn statics_mut(&mut self) -> &mut Statics {
        Rc::make_mut(&mut self.statics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloning_a_session_shares_its_statics() {
        let session = Session::start(&crate::testkit::catalog()).expect("starts");
        let clone = session.clone();
        assert!(Rc::ptr_eq(&session.statics, &clone.statics));
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn a_class_speed_is_its_ships_speed_alone_and_none_without_a_record() {
        use crate::catalog::ShipId;
        use crate::handling::ShipFields;
        use crate::testkit::{FAST, ship};
        let mut catalog = crate::testkit::catalog();
        catalog
            .ship_records
            .push(ship(200, ShipFields { speed: 350, ..FAST }));
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.statics.class_speed(ShipId(200)), 3.5);
        assert_eq!(session.statics.class_speed(ShipId(999)), 0.0, "no record");
    }
}
