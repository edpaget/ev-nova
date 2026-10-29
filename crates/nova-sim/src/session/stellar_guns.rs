//! The stellar defence guns in flight (see
//! [`stellar_guns`](crate::combat::stellar_guns)): `_HandleStellarWeapons`
//! (@0x2ef3a-0x2f5d1), which `_DoPlayGameWork` calls after
//! `_HandleStellarSprites` (@0x444d7), after the ship loop and before the
//! stellar hazards and `_HandleBeams`, on both passes.
//!
//! **Which stellars.** Each the player's system lists, in `NavDefs` order
//! (the chart's listing, stamped through the galaxy overlay so its
//! government and destroyed state are live), that is active (+0x44: its
//! holder is visible, [`OverlaidGalaxy::active_holder`]), not destroyed
//! (`_StellarIsDestroyed` @0x2efd4) and has a gun: a `Weapon` the arsenal
//! holds. The original tests no owner, so a stellar the player's system
//! lists while another visible system holds it still fires, though it is
//! not shown; nor `Flags`, hypergate or `Strength`. A destroyed stellar
//! fires again once it regenerates, its timer as it left it.
//!
//! **What each reads** ([`StellarGun`]):
//!
//! - dominated (+0x46, @0x9f0c): by its `Flags2` [`DOMINATED`] bit or
//!   by Demand Tribute ([`crate::domination::dominated`]);
//! - defended (+0x47): whether it is under attack, as this frame's +0x47
//!   pass left it, just before the guns: one of its defence ships is in
//!   the system (see the `stellar_defence` module). A `Flags2` 0x0200
//!   stellar (Earth, Port Kane, Pyrogenesis I, New Ireland) fires only
//!   then;
//! - the player's standing: `G`, its own government or else the system's;
//!   the record as [`RuleKey::LegalRecords`] reads it at the stellar
//!   (`_IsThreatToStellar` @0x9fb8, the per-system record by the engine);
//!   the player's class's combat government; whether its own government
//!   blesses peace ([`Session::blessing`]); whether `G` carries an IFF
//!   scrambler's mark (see the `iff_scramblers` module); and whether it is
//!   the stellar the player asked to dock at.
//!
//! **The timers** ([`GunTimer`], the original's +0x494 and +0x2e) are kept
//! on [`Space`] by stellar, across systems, and zeroed for
//! every active stellar by the daily step (`_IncrementGameTime` @0xb5df,
//! @0xb5ed). They are never saved, so a new or loaded pilot starts with
//! none, as `_InitObjects` zeroes them (@0x1c902, @0x1c90f).
//!
//! [`OverlaidGalaxy::active_holder`]: crate::story::OverlaidGalaxy::active_holder
//! [`RuleKey::LegalRecords`]: crate::rulebook::RuleKey::LegalRecords

use super::{Session, Space};
use crate::ai::Blessed;
use crate::catalog::StellarId;
use crate::chance::Chance;
use crate::combat::stellar_guns::{GunTimer, Standing, StellarGun};
use crate::combat::{GamePass, Rules};
use crate::story::listers_of;

impl Session {
    /// `_HandleStellarWeapons` on `pass` by `rules`, drawing on `chance`:
    /// each armed stellar of the player's system fires at its target as
    /// its timer allows ([`Combat::fire_stellar_guns`](crate::combat::Combat::fire_stellar_guns)),
    /// and its timer is kept (see the module docs).
    pub(super) fn stellar_guns(&mut self, pass: GamePass, rules: Rules, chance: &mut dyn Chance) {
        let mut guns = self.armed_stellars();
        if guns.is_empty() {
            return;
        }
        self.with_fighters(pass, rules, chance, |combat, fighters, arena, chance| {
            combat.fire_stellar_guns(fighters, arena, &mut guns, chance);
        });
        self.space.keep_gun_timers(&guns);
    }

    /// The guns of the player's system as they stand now, in `NavDefs`
    /// order (see the module docs).
    fn armed_stellars(&self) -> Vec<StellarGun> {
        let blessing = self.blessing();
        let class_govt = self
            .statics
            .ship_record(self.pilot.ship)
            .and_then(|record| record.combat_govt);
        let system = self.pilot.system;
        let system_govt = self.chart.galaxy.govt(system);
        let docking = self.port.dock.map(|dock| dock.stellar);
        self.chart
            .listed
            .iter()
            .filter_map(|&listed| {
                let site = self.chart.galaxy.site(listed);
                let weapon = *self.statics.stellar_weapons.get(&site.id)?;
                self.statics.arsenal.weapon(weapon)?;
                if site.destroyed || !self.stellar_active(site.id) {
                    return None;
                }
                let judge = site.govt.or(system_govt);
                let record = self
                    .chart
                    .record_in(&self.pilot, &self.setup, system, Some(&site));
                Some(StellarGun {
                    stellar: site.id,
                    position: site.position,
                    weapon,
                    govt: site.govt,
                    flags2: site.flags2,
                    dominated: crate::domination::dominated(&site),
                    defended: self.space.traffic.defences().under_attack(site.id),
                    standing: Standing {
                        govt: judge,
                        record,
                        class_govt,
                        blessed: blessing.blesses(site.govt, Blessed::Peace),
                        iff_marked: judge.is_some_and(|govt| self.space.iff_marked.contains(&govt)),
                        docking: docking == Some(site.id),
                    },
                    timer: self
                        .space
                        .gun_timers
                        .get(&site.id)
                        .copied()
                        .unwrap_or_default(),
                })
            })
            .collect()
    }

    /// Whether `stellar` is active (+0x44): its holder, with the player in
    /// its system, is visible (`_HandleStellarSystemVisibility`
    /// @0x32c58-0x32cdf).
    pub(super) fn stellar_active(&self, stellar: StellarId) -> bool {
        let listers = listers_of(&self.statics.listed_stellars, stellar);
        self.chart
            .galaxy
            .active_holder(stellar, listers, self.pilot.system)
            .is_some()
    }

    /// The daily step's reset of the stellar guns (`_IncrementGameTime`
    /// @0xb5df, @0xb5ed): every active stellar's timer zeroed.
    pub(super) fn reset_gun_timers(&mut self) {
        let timers = std::mem::take(&mut self.space.gun_timers);
        self.space.gun_timers = timers
            .into_iter()
            .filter(|&(stellar, _)| !self.stellar_active(stellar))
            .collect();
    }
}

impl Space {
    /// Keeps each of `guns`' timer, forgetting one ready with no burst
    /// under way, as none is the same.
    fn keep_gun_timers(&mut self, guns: &[StellarGun]) {
        for gun in guns {
            if gun.timer == GunTimer::default() {
                self.gun_timers.remove(&gun.stellar);
            } else {
                self.gun_timers.insert(gun.stellar, gun.timer);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{
        GovtId, GovtRecord, LandingSite, ShipRecord, SystemId, WeaponId, WeaponRecord,
    };
    use crate::chance::NeverFires;
    use crate::clearance::DockRequest;
    use crate::combat::CombatEvent;
    use crate::combat::projectile::Shot;
    use crate::control::SetExpr;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::market::DOMINATED;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::{
        FAST, FakePilotCatalog, catalog, govt, life, planet, rank, ship, star, weapon,
    };
    use crate::trig::{self, LeadShot};

    const FEDERATION: GovtId = GovtId(128);
    const TRADERS: GovtId = GovtId(140);
    const GUN: WeaponId = WeaponId(196);

    /// A turret of `Reload` `reload`, 10 pixels a tick for 40 ticks:
    /// reaching 400 pixels.
    fn turret(reload: i16) -> WeaponRecord {
        WeaponRecord {
            reload,
            count: 40,
            speed: 1000,
            mass_dmg: 4,
            energy_dmg: 10,
            ..weapon(196)
        }
    }

    /// `base` with the Federation (class 1, `CrimeTol` 6) governing system
    /// 130 and at war with the traders (140, class 2), and `stellars` of
    /// 130 each the Federation's, armed with [`turret`] of `reload`.
    fn gunned(mut base: FakePilotCatalog, stellars: &[i16], reload: i16) -> FakePilotCatalog {
        base.govts = vec![
            GovtRecord {
                classes: [1, -1, -1, -1],
                enemies: [2, -1, -1, -1],
                crime_tol: 6,
                ..govt(128)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..govt(140)
            },
        ];
        for system in &mut base.star_map {
            if system.id == SystemId(130) {
                system.govt = Some(FEDERATION);
            }
        }
        for (_, sites) in &mut base.sites {
            for site in sites
                .iter_mut()
                .filter(|site| stellars.contains(&site.id.0))
            {
                site.govt = Some(FEDERATION);
            }
        }
        base.weapons.push(turret(reload));
        base.stellar_weapons = stellars
            .iter()
            .map(|&stellar| (StellarId(stellar), GUN))
            .collect();
        base
    }

    /// Stellar 128, at (30, -40), the Federation's and armed, firing
    /// every other tick.
    fn armed_128() -> FakePilotCatalog {
        gunned(catalog(), &[128], 1)
    }

    /// `catalog`'s session with the player at rest 100 to the right of
    /// stellar 128, its record in system 130 `record`.
    fn beside_128(catalog: &FakePilotCatalog, record: i16) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        place_player(&mut session, record);
        session
    }

    fn place_player(session: &mut Session, record: i16) {
        session.ship.player.position = Vec2::new(130.0, -40.0);
        session.ship.player.velocity = Vec2::ZERO;
        session.pilot.set_system_record(SystemId(130), record);
    }

    /// A pass of the game's work.
    fn work(session: &mut Session, catalog: &FakePilotCatalog) {
        let behaviour = crate::ai::NovaAi::default();
        session.game_work(
            Controls::default(),
            catalog,
            &behaviour,
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// The gun shots in flight.
    fn gun_shots(session: &Session) -> Vec<Shot> {
        session
            .shots()
            .iter()
            .filter(|shot| shot.weapon.id == GUN)
            .copied()
            .collect()
    }

    /// The stellars that fired, as the events since they were last taken
    /// say.
    fn fired(session: &mut Session) -> Vec<StellarId> {
        session
            .take_combat_events()
            .into_iter()
            .filter_map(|event| match event {
                CombatEvent::StellarFired { stellar, .. } => Some(stellar),
                _ => None,
            })
            .collect()
    }

    /// Whether stellar 128 fires on a pass at the player beside it, its
    /// record `record`, in `catalog`'s session changed by `change`.
    fn fires_at(
        catalog: &FakePilotCatalog,
        record: i16,
        change: impl FnOnce(&mut Session),
    ) -> bool {
        let mut session = beside_128(catalog, record);
        change(&mut session);
        work(&mut session, catalog);
        !fired(&mut session).is_empty()
    }

    #[test]
    fn a_criminal_in_range_draws_a_shot_from_the_stellar_that_first_moves_on_the_next_pass() {
        let catalog = armed_128();
        let mut session = beside_128(&catalog, -7);
        work(&mut session, &catalog);
        assert_eq!(fired(&mut session), [StellarId(128)]);
        let shots = gun_shots(&session);
        assert_eq!(shots.len(), 1);
        assert_eq!(shots[0].position, Vec2::new(30.0, -40.0), "not yet moved");
        assert_eq!(shots[0].aimed_at, Some(crate::combat::ShipRef::Player));
        work(&mut session, &catalog);
        let moved = gun_shots(&session);
        let off = moved[0].position - Vec2::new(40.0, -40.0);
        assert!(off.length() < 1e-3, "east at 10: {:?}", moved[0].position);
        assert!(fired(&mut session).is_empty(), "reloading");
        assert_eq!(
            session.space.gun_timers.get(&StellarId(128)),
            None,
            "ready again, so forgotten"
        );
    }

    #[test]
    fn the_gun_fires_before_a_deadly_stellar_kills_its_mark() {
        let catalog = gunned(armed(), &[128], 1);
        let mut session = facing_an_npc(&catalog, 180);
        place_player(&mut session, 0);
        let deadly = LandingSite {
            flags2: crate::deadly_stellar::DEADLY_STELLAR,
            ..planet(140, 0.0, -150.0)
        };
        session.chart.listed.push(deadly);
        session.chart.sites.push(deadly);
        work(&mut session, &catalog);
        let events = session.take_combat_events();
        let at = |found: fn(&CombatEvent) -> bool| events.iter().position(found);
        let gun = at(|event| matches!(event, CombatEvent::StellarFired { .. }));
        let kill = at(|event| matches!(event, CombatEvent::Exploded { .. }));
        assert!(gun.is_some() && kill.is_some(), "{events:?}");
        assert!(
            gun < kill,
            "the gun before the hazards and beams: {events:?}"
        );
    }

    #[test]
    fn a_player_in_good_standing_draws_no_fire() {
        let catalog = armed_128();
        assert!(!fires_at(&catalog, -6, |_| {}), "at -CrimeTol");
        assert!(fires_at(&catalog, -7, |_| {}));
    }

    #[test]
    fn double_time_fires_on_both_passes_once_ready() {
        let catalog = gunned(catalog(), &[128], 0);
        let mut session = beside_128(&catalog, -7);
        work(&mut session, &catalog);
        session.tick_again(
            Controls::default(),
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(fired(&mut session), [StellarId(128); 2]);
    }

    #[test]
    fn an_npc_is_led_from_where_its_move_left_it() {
        let catalog = gunned(armed(), &[128], 1);
        let mut session = facing_an_npc(&catalog, 90);
        place_player(&mut session, 0);
        let npc = &mut session.space.traffic.npcs_mut()[0];
        npc.state.position = Vec2::new(30.0, -60.0);
        npc.state.velocity = Vec2::new(4.0, 0.0);
        let before = npc.state;
        work(&mut session, &catalog);
        let after = session.npcs()[0].state;
        assert_ne!(after.position, before.position, "it moved");
        let lead = |state: crate::flight::ShipState| {
            f32::from(trig::lead_angle(
                Vec2::new(30.0, -40.0),
                Vec2::ZERO,
                (state.position, state.velocity),
                Some(LeadShot {
                    speed: 10.0,
                    rocket: false,
                }),
            ))
        };
        assert_ne!(lead(before), lead(after));
        let shots = gun_shots(&session);
        assert_eq!(shots.len(), 1, "at the NPC, the player clean");
        assert_eq!(shots[0].heading, lead(after));
    }

    #[test]
    fn an_npc_jumping_in_draws_no_fire() {
        let catalog = gunned(armed(), &[128], 1);
        let fires = |mode: Option<crate::traffic::npc::Mode>| {
            let mut session = facing_an_npc(&catalog, 90);
            place_player(&mut session, 0);
            let npc = &mut session.space.traffic.npcs_mut()[0];
            npc.state.position = Vec2::new(30.0, -60.0);
            if let Some(mode) = mode {
                npc.mode = mode;
            }
            work(&mut session, &catalog);
            !fired(&mut session).is_empty()
        };
        assert!(fires(None), "at the NPC, the player clean");
        let gliding = crate::traffic::npc::Mode::JumpingIn { tick: 0 };
        assert!(!fires(Some(gliding)));
    }

    #[test]
    fn a_stellar_another_visible_system_holds_still_fires_unshown() {
        let mut catalog = gunned(catalog(), &[129], 1);
        catalog.systems.push(SystemId(125));
        catalog.star_map.push(star(125, (-600.0, 600.0), &[]));
        catalog
            .sites
            .push((SystemId(125), vec![planet(129, 2000.0, 0.0)]));
        let mut session = Session::start(&catalog).expect("starts");
        assert!(
            session
                .chart
                .sites
                .iter()
                .all(|site| site.id != StellarId(129)),
            "125 holds it"
        );
        session.ship.player.position = Vec2::new(2000.0, 100.0);
        session.pilot.set_system_record(SystemId(130), -7);
        work(&mut session, &catalog);
        assert_eq!(fired(&mut session), [StellarId(129)]);
    }

    #[test]
    fn a_stellar_no_visible_system_lists_holds_its_fire() {
        let mut catalog = armed_128();
        catalog.stellar_weapons.push((StellarId(150), GUN));
        let mut session = beside_128(&catalog, -7);
        session.chart.listed = vec![LandingSite {
            govt: Some(FEDERATION),
            ..planet(150, 130.0, 0.0)
        }];
        work(&mut session, &catalog);
        assert!(fired(&mut session).is_empty());
    }

    #[test]
    fn a_destroyed_stellar_holds_its_fire_until_it_regenerates() {
        let mut catalog = gunned(catalog(), &[128], 0);
        catalog.lives = vec![life(128, 5)];
        let mut session = beside_128(&catalog, -7);
        let run = |session: &mut Session, text: &str| {
            session.run_set(&SetExpr::parse(text).expect("parses"), &mut NeverFires);
        };
        run(&mut session, "Y128");
        work(&mut session, &catalog);
        assert!(fired(&mut session).is_empty(), "destroyed");
        run(&mut session, "U128");
        work(&mut session, &catalog);
        assert_eq!(fired(&mut session), [StellarId(128)], "regenerated");
    }

    #[test]
    fn the_day_zeroes_a_gun_mid_reload() {
        let catalog = gunned(catalog(), &[128], 30);
        let mut session = beside_128(&catalog, -7);
        work(&mut session, &catalog);
        work(&mut session, &catalog);
        assert_eq!(
            session.space.gun_timers.get(&StellarId(128)),
            Some(&GunTimer {
                reload: 29.0,
                burst: 0
            })
        );
        fired(&mut session);
        session.stellars_day(&mut NeverFires);
        assert!(session.space.gun_timers.is_empty());
        work(&mut session, &catalog);
        assert_eq!(fired(&mut session), [StellarId(128)], "ready at once");
    }

    #[test]
    fn the_day_leaves_an_inactive_stellars_timer() {
        let catalog = armed_128();
        let mut session = beside_128(&catalog, -7);
        let timer = GunTimer {
            reload: 9.0,
            burst: 1,
        };
        session.space.gun_timers.insert(StellarId(150), timer);
        session.space.gun_timers.insert(StellarId(128), timer);
        session.stellars_day(&mut NeverFires);
        assert_eq!(
            session.space.gun_timers.into_iter().collect::<Vec<_>>(),
            [(StellarId(150), timer)],
            "no system lists 150"
        );
    }

    #[test]
    fn by_the_bible_the_record_with_the_stellars_government_is_read() {
        let catalog = armed_128();
        let bible = Rulebook::default().with_override(RuleKey::LegalRecords, RuleSource::Bible);
        let wanted_by_the_federation = |session: &mut Session| {
            session.pilot.set_system_record(SystemId(130), 0);
            session.pilot.set_legal_record(FEDERATION, -7);
        };
        assert!(
            !fires_at(&catalog, 0, wanted_by_the_federation),
            "the engine"
        );
        assert!(fires_at(&catalog, 0, |session| {
            *session = session.clone().with_rules(bible);
            wanted_by_the_federation(session);
        }));
        assert!(!fires_at(&catalog, -7, |session| {
            *session = session.clone().with_rules(bible);
        }));
    }

    #[test]
    fn the_dock_target_an_iff_mark_and_domination_spare_a_criminal() {
        let catalog = armed_128();
        assert!(!fires_at(&catalog, -7, |session| {
            session.port.dock = Some(DockRequest {
                stellar: StellarId(128),
                permission: 0,
            });
        }));
        assert!(fires_at(&catalog, -7, |session| {
            session.port.dock = Some(DockRequest {
                stellar: StellarId(129),
                permission: 0,
            });
        }));
        assert!(!fires_at(&catalog, -7, |session| {
            session.space.iff_marked.insert(FEDERATION);
        }));
        assert!(fires_at(&catalog, -7, |session| {
            session.space.iff_marked.insert(TRADERS);
        }));
        let mut dominated = armed_128();
        dominated.sites[0].1[0].flags2 = DOMINATED;
        assert!(!fires_at(&dominated, -7, |_| {}));
        assert!(
            !fires_at(&catalog, -7, |session| {
                session.chart.set_dominated(
                    &mut session.pilot,
                    &mut session.out,
                    StellarId(128),
                    true,
                );
            }),
            "dominated at run time (@0x9f0c)"
        );
    }

    #[test]
    fn a_stellar_waiting_for_its_defenders_holds_fire_with_none_in_the_system() {
        let mut guarded = armed_128();
        guarded.sites[0].1[0].flags2 = crate::combat::stellar_guns::GUARDED_BY_DEFENDERS;
        assert!(!fires_at(&guarded, -1000, |_| {}));
    }

    /// `session` with NPC `id` defending `stellar`.
    fn defended_by(session: &mut Session, id: u32, stellar: i16) {
        let npc = crate::traffic::npc::Npc {
            defends: Some(StellarId(stellar)),
            ..crate::traffic::fixture::npc(id, crate::stats::ShipStats::new(FAST, &[]))
        };
        session.space.traffic.add_npc(npc);
    }

    /// Whether the guarded stellar 128 fires at a criminal beside it on
    /// the pass after a defender of `stellar` comes into the populated
    /// system.
    fn fires_once_defended_by(stellar: i16) -> bool {
        let mut guarded = armed_128();
        guarded.sites[0].1[0].flags2 = crate::combat::stellar_guns::GUARDED_BY_DEFENDERS;
        let mut session = beside_128(&guarded, -7);
        work(&mut session, &guarded);
        assert!(fired(&mut session).is_empty(), "no defender yet");
        defended_by(&mut session, 900, stellar);
        work(&mut session, &guarded);
        !fired(&mut session).is_empty()
    }

    #[test]
    fn a_stellar_waiting_for_its_defenders_fires_once_one_is_in_the_system() {
        assert!(
            fires_once_defended_by(128),
            "the frame's +0x47 pass marks it before the guns (@0x443d4)"
        );
        assert!(
            !fires_once_defended_by(129),
            "another stellar's defender doesn't count"
        );
    }

    #[test]
    fn a_class_of_an_enemy_government_draws_fire_in_good_standing() {
        let mut catalog = armed_128();
        catalog.ship_records.push(ShipRecord {
            combat_govt: Some(TRADERS),
            ..ship(128, FAST)
        });
        assert!(fires_at(&catalog, 0, |_| {}));
    }

    #[test]
    fn the_stellars_governments_blessing_spares_a_criminal() {
        let mut catalog = armed_128();
        catalog.ranks = vec![crate::catalog::RankRecord {
            affil_govt: Some(FEDERATION),
            flags: crate::rank::BLESSES_PEACE,
            ..rank(200)
        }];
        let blessing_now =
            Rulebook::default().with_override(RuleKey::BlessingRefresh, RuleSource::Bible);
        assert!(!fires_at(&catalog, -7, |session| {
            *session = session.clone().with_rules(blessing_now);
            session.pilot.ranks.insert(crate::catalog::RankId(200));
        }));
        assert!(fires_at(&catalog, -7, |session| {
            *session = session.clone().with_rules(blessing_now);
        }));
    }

    /// [`armed_128`] with stellar 128 independent: no government of its
    /// own, in the Federation's system 130.
    fn independent_128() -> FakePilotCatalog {
        let mut catalog = armed_128();
        let site = &mut catalog.sites[0].1[0];
        assert_eq!(site.id, StellarId(128));
        site.govt = None;
        catalog
    }

    #[test]
    fn an_independent_stellar_judges_by_its_systems_government() {
        let catalog = independent_128();
        assert!(
            !fires_at(&catalog, -6, |_| {}),
            "at the Federation's -CrimeTol"
        );
        assert!(fires_at(&catalog, -7, |_| {}));
    }

    #[test]
    fn an_iff_mark_on_its_systems_government_spares_a_criminal_from_an_independent_stellar() {
        let catalog = independent_128();
        assert!(!fires_at(&catalog, -7, |session| {
            session.space.iff_marked.insert(FEDERATION);
        }));
    }

    #[test]
    fn its_systems_governments_blessing_leaves_an_independent_stellar_firing() {
        let mut catalog = independent_128();
        catalog.ranks = vec![crate::catalog::RankRecord {
            affil_govt: Some(FEDERATION),
            flags: crate::rank::BLESSES_PEACE,
            ..rank(200)
        }];
        let blessing_now =
            Rulebook::default().with_override(RuleKey::BlessingRefresh, RuleSource::Bible);
        assert!(fires_at(&catalog, -7, |session| {
            *session = session.clone().with_rules(blessing_now);
            session.pilot.ranks.insert(crate::catalog::RankId(200));
        }));
    }
}
