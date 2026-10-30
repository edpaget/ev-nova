//! What of each ship's sprites the session loads: the `ShipAnimations`
//! and `RunningLights` preferences as [`SpriteLoad`] carries them (see
//! `nova_sim::combat::sprite_load`), applied to every hull that takes a
//! class's frames into play.
//!
//! The load is a setting of [`Setup`], never saved, which the view sends
//! as the flight is built ([`FlightCommand::SetSpriteLoad`](crate::flight_frame::FlightCommand::SetSpriteLoad))
//! and again when the player's choice reaches it. [`Statics`] stays as it
//! was read: its arsenal holds each class's frames raw, and the session
//! loads them as it takes them ([`Setup::hull`], [`Setup::frames_of`]):
//!
//! 1. the player's hull at each refit, a change of ship among them;
//! 2. each kind [`Setup::ship_kind`] builds: a fighter launched, an
//!    escort placed, the eject's hulk and a hire;
//! 3. each kind and person of the system's spawn table, as it is drawn up
//!    and as missions add their `düde`s to it;
//! 4. the first class, ID 128, which most spawns roll their animation on.
//!
//! No session code reads a class's frames from the arsenal raw for a ship
//! in play. The hull's other fields (`strength`, `tough`) don't depend on
//! the load, and their reads stay raw.
//!
//! [`Session::set_sprite_load`] changes the load and refits every live
//! copy from the raw arsenal: the player, the first class, and each NPC
//! and spawnable kind, so a load set back restores the class's sets. Only
//! the frames are rewritten. A ship's fold or anim set left from more sets
//! is harmless: its display draws set 0 while its class has one.
//!
//! The original loads its sprites once, at launch, and its preferences
//! never change them in play; when a change reaches the session is the
//! view's choice (`RuleKey::WeaponEffectsNextLaunch`). No sound plays in
//! any of it.

use super::{PlayerShip, Session, Setup, Space, Statics};
use crate::catalog::ShipId;
use crate::combat::SpriteLoad;
use crate::combat::hull::HullSpec;
use crate::combat::ship_frames::FrameSpec;
use crate::traffic::table::SpawnTable;

impl Setup {
    /// Ship class `ship`'s hull, its frames as the load loads them.
    pub(super) fn hull(&self, statics: &Statics, ship: ShipId) -> HullSpec {
        let mut hull = statics.arsenal.hull(ship);
        hull.frames = hull.frames.loaded(self.sprite_load);
        hull
    }

    /// Ship class `ship`'s frames, as the load loads them.
    pub(super) fn frames_of(&self, statics: &Statics, ship: ShipId) -> FrameSpec {
        statics.arsenal.hull(ship).frames.loaded(self.sprite_load)
    }

    /// Gives each kind and person of `table` its class's frames as the
    /// load loads them.
    pub(super) fn load_frames(&self, statics: &Statics, table: &mut SpawnTable) {
        table.refit_frames(&|ship| self.frames_of(statics, ship));
    }
}

impl Session {
    /// Loads each ship's sprites as `load` says from now on, and refits
    /// every ship and kind in play to it (see the module docs).
    pub fn set_sprite_load(&mut self, load: SpriteLoad) {
        self.setup.sprite_load = load;
        self.ship
            .refit_frames(&self.statics, &self.setup, self.pilot.ship);
        self.space.set_first_class(&self.statics, &self.setup);
        self.space.refit_frames(&self.statics, &self.setup);
    }

    /// What of each ship's sprites the session loads.
    #[must_use]
    pub fn sprite_load(&self) -> SpriteLoad {
        self.setup.sprite_load
    }
}

impl PlayerShip {
    /// Gives the player's hull class `ship`'s frames as `setup` loads
    /// them, the rest of it as it is.
    pub(super) fn refit_frames(&mut self, statics: &Statics, setup: &Setup, ship: ShipId) {
        self.fit.hull.frames = setup.frames_of(statics, ship);
    }
}

impl Space {
    /// Gives each NPC, and each kind the traffic can spawn, its class's
    /// frames as `setup` loads them.
    pub(super) fn refit_frames(&mut self, statics: &Statics, setup: &Setup) {
        self.traffic
            .refit_frames(&|ship| setup.frames_of(statics, ship));
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{
        DudeId, DudeRecord, HullRecord, ShipDisplay, ShipExits, SystemId, SystemTraffic,
    };
    use crate::chance::NeverFires;
    use crate::combat::ship_frames::{ANIMATED, BANKS, FOLDS};
    use crate::flight::{Controls, Turn};
    use crate::session::ReservePolicy;
    use crate::session::ship_change::ClassChange;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, hull, ship, work};
    use nova_sim_base::blink::Blink;

    /// Animations off, lights on.
    const STILL: SpriteLoad = SpriteLoad {
        ship_animations: false,
        running_lights: true,
    };

    /// Animations on, lights off.
    const DARK: SpriteLoad = SpriteLoad {
        ship_animations: true,
        running_lights: false,
    };

    /// The Shuttle's square wave, `shän` 128.
    const SHUTTLE: Blink = Blink {
        mode: 1,
        a: 4,
        b: 1,
        c: 2,
        d: 20,
    };

    /// Class `id` of `shän` `Flags` `flags`, `sets` base sets and three
    /// alt sets, `AnimDelay` 5, 36 frames a turn, blinking its lights.
    fn shän(id: i16, flags: u16, sets: i16) -> HullRecord {
        HullRecord {
            display: Some(ShipDisplay {
                flags,
                base_sets: sets,
                anim_delay: 5,
                alt_sets: 3,
                alt_image: true,
                blink: SHUTTLE,
                lights_image: true,
            }),
            exits: Some(ShipExits {
                frames: 36,
                ..ShipExits::default()
            }),
            ..hull(id)
        }
    }

    /// The test catalog, the player's 128 banking and folding through
    /// three sets, ship 129 animated through six, and system 130's
    /// traffic one ship of 129 on average.
    fn displaying() -> FakePilotCatalog {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        FakePilotCatalog {
            hulls: vec![shän(128, BANKS | FOLDS, 3), shän(129, ANIMATED, 6)],
            ship_records: vec![ship(128, FAST), ship(129, FAST)],
            traffic: vec![(
                SystemId(130),
                SystemTraffic {
                    dude_types,
                    avg_ships: 1,
                    persons: Default::default(),
                },
            )],
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type: 1,
                    govt: None,
                    ships: vec![(ShipId(129), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )],
            ..catalog()
        }
    }

    /// The frames of class `ship` of [`displaying`], raw.
    fn raw(session: &Session, ship: i16) -> FrameSpec {
        session.statics.arsenal.hull(ShipId(ship)).frames
    }

    /// The frames of each NPC of class 129 in `session`.
    fn npc_frames(session: &Session) -> Vec<FrameSpec> {
        session
            .space
            .traffic
            .npcs()
            .iter()
            .filter(|npc| npc.ship == ShipId(129))
            .map(|npc| npc.hull.frames)
            .collect()
    }

    #[test]
    fn the_players_hull_takes_the_load_and_the_default_restores_it() {
        let mut session = Session::start(&displaying()).expect("starts");
        assert_eq!(session.sprite_load(), SpriteLoad::default());
        let whole = raw(&session, 128);
        assert_eq!(session.hull().frames, whole);
        session.set_sprite_load(STILL);
        assert_eq!(session.sprite_load(), STILL);
        assert_eq!(session.hull().frames, whole.loaded(STILL));
        assert_eq!(session.hull().frames.base_sets, 1);
        session.set_sprite_load(DARK);
        assert_eq!(session.hull().frames, whole.loaded(DARK));
        assert!(!session.hull().frames.lights);
        session.set_sprite_load(SpriteLoad::default());
        assert_eq!(session.hull().frames, whole, "the raw sets back");
        assert_eq!(
            session.hull(),
            session.statics.arsenal.hull(ShipId(128)),
            "the rest of the hull untouched"
        );
    }

    #[test]
    fn a_change_of_ship_takes_the_load() {
        let mut session = Session::start(&displaying()).expect("starts");
        session.set_sprite_load(STILL);
        let record = session
            .statics
            .ship_record(ShipId(129))
            .cloned()
            .expect("ship 129");
        let change = ClassChange {
            outfits: session.pilot.outfits.clone(),
            reserves: ReservePolicy::Hold,
        };
        session.become_class(&record, change, |_| {});
        assert_eq!(session.hull().frames, raw(&session, 129).loaded(STILL));
    }

    #[test]
    fn the_traffic_spawned_after_the_load_takes_it() {
        let catalog = displaying();
        let mut session = Session::start(&catalog).expect("starts");
        session.set_sprite_load(STILL);
        session.populate(&catalog, &mut NeverFires);
        let loaded = raw(&session, 129).loaded(STILL);
        let frames = npc_frames(&session);
        assert!(!frames.is_empty(), "a ship of 129 came in");
        assert!(frames.iter().all(|&spec| spec == loaded), "{frames:?}");
        assert_eq!(
            session.space.traffic.table().ships[&ShipId(129)]
                .hull
                .frames,
            loaded,
            "the table its later spawns are drawn from"
        );
    }

    #[test]
    fn a_kind_the_session_builds_takes_the_load() {
        let catalog = displaying();
        let mut session = Session::start(&catalog).expect("starts");
        session.set_sprite_load(DARK);
        let record = session
            .statics
            .ship_record(ShipId(129))
            .cloned()
            .expect("ship 129");
        assert_eq!(
            session
                .setup
                .ship_kind(&session.statics, &record)
                .hull
                .frames,
            raw(&session, 129).loaded(DARK),
            "a fighter's, an escort's, a hulk's or a hire's"
        );
        session
            .pilot
            .escorts
            .push(crate::session::tests::trader_escort());
        session.populate(&catalog, &mut NeverFires);
        let id = session.space.escorts.fleet[0].expect("placed");
        let escort = session.space.npc(id).expect("in the system");
        assert!(!escort.hull.frames.lights, "the escort placed");
    }

    #[test]
    fn the_first_class_spawns_roll_on_takes_the_load() {
        let catalog = displaying();
        let mut session = Session::start(&catalog).expect("starts");
        let whole = raw(&session, 128);
        assert_eq!(session.space.traffic.first_class().spec, whole);
        session.set_sprite_load(STILL);
        let first = session.space.traffic.first_class().spec;
        assert_eq!(first, whole.loaded(STILL));
        assert_eq!(first.base_sets, 1, "a roll over one set");
        let stellar = session.chart.sites[0].id;
        session
            .land_at(&catalog, session.pilot.system, stellar)
            .expect("relocates");
        assert_eq!(
            session.space.traffic.first_class().spec,
            whole.loaded(STILL),
            "re-applied to a new traffic"
        );
    }

    #[test]
    fn a_change_of_load_refits_a_live_npc_and_the_table_and_back() {
        let catalog = displaying();
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        let whole = raw(&session, 129);
        assert!(npc_frames(&session).iter().all(|&spec| spec == whole));
        session.set_sprite_load(STILL);
        let frames = npc_frames(&session);
        assert!(!frames.is_empty());
        assert!(
            frames.iter().all(|&spec| spec == whole.loaded(STILL)),
            "{frames:?}"
        );
        assert_eq!(
            session.space.traffic.table().ships[&ShipId(129)]
                .hull
                .frames,
            whole.loaded(STILL)
        );
        session.set_sprite_load(SpriteLoad::default());
        assert!(npc_frames(&session).iter().all(|&spec| spec == whole));
        assert_eq!(
            session.space.traffic.table().ships[&ShipId(129)]
                .hull
                .frames,
            whole
        );
    }

    #[test]
    fn a_mission_düde_added_after_the_load_takes_it() {
        use crate::catalog::MissionShip;
        use crate::session::mission_ships::tests::{
            accepted, aux_catalog, aux_ships, take_off, ticks,
        };
        // Mission 140's three auxiliary ships of düde 151 fly ship 201,
        // a class no table of the system holds until they come.
        let mut catalog =
            crate::testkit::far_129_reachable(aux_catalog(MissionShip::NONE, aux_ships(3, -1), 0));
        catalog.hulls.push(shän(201, ANIMATED, 6));
        let mut session = accepted(&catalog);
        session.set_sprite_load(STILL);
        take_off(&mut session, &catalog);
        assert!(
            !session
                .space
                .traffic
                .table()
                .ships
                .contains_key(&ShipId(201)),
            "not in the table before they come"
        );
        ticks(&mut session, &catalog, 140);
        let loaded = raw(&session, 201).loaded(STILL);
        assert_ne!(loaded, raw(&session, 201), "the load changes the class");
        assert_eq!(
            session.space.traffic.table().ships[&ShipId(201)]
                .hull
                .frames,
            loaded,
            "the kind the düde added"
        );
        let frames: Vec<FrameSpec> = session
            .space
            .traffic
            .npcs()
            .iter()
            .filter(|npc| npc.aux.is_some())
            .map(|npc| npc.hull.frames)
            .collect();
        assert_eq!(frames, vec![loaded; 3], "the ships it brought in");
    }

    /// The right turn key held, thrusting.
    fn thrust_right() -> Controls {
        Controls {
            thrust: true,
            turn: Turn::Right,
            ..Controls::default()
        }
    }

    #[test]
    fn without_the_animations_a_banked_folding_player_still_flags_and_glows() {
        let catalog = displaying();
        let mut session = Session::start(&catalog).expect("starts");
        session.set_sprite_load(STILL);
        for _ in 0..7 {
            work(&mut session, &catalog, thrust_right());
        }
        assert_eq!(session.ship.ship_display.frames.flag, 1, "banked");
        assert_eq!(session.engine_glow(), 9, "banked on the 7th: +2");
        assert_eq!(session.player_drawn().set, 0, "drawn level");
    }

    #[test]
    fn without_the_lights_the_player_shows_none_and_its_lights_stand_still() {
        let catalog = displaying();
        let mut lit = Session::start(&catalog).expect("starts");
        let mut levels = Vec::new();
        for _ in 0..30 {
            work(&mut lit, &catalog, Controls::default());
            levels.push(lit.player_drawn().lights);
        }
        assert!(
            levels.iter().any(Option::is_some),
            "loaded whole, it blinks"
        );
        let mut session = Session::start(&catalog).expect("starts");
        session.set_sprite_load(DARK);
        let before = session.ship.ship_display.frames.lights;
        for _ in 0..30 {
            work(&mut session, &catalog, Controls::default());
            assert_eq!(session.player_drawn().lights, None);
        }
        assert_eq!(session.ship.ship_display.frames.lights, before, "still");
    }
}
