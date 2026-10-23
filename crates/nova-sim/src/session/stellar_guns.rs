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
//! - dominated: its `Flags2` [`DOMINATED`] bit, the only domination the
//!   engine has (hail domination is task `planet-domination`'s);
//! - defended (+0x47): never, as the engine spawns no stellar defence
//!   ships (task `planet-domination`), so a `Flags2` 0x0200 stellar
//!   (Earth, Port Kane, Pyrogenesis I, New Ireland) never fires;
//! - the player's standing: `G`, its own government or else the system's;
//!   the record as [`RuleKey::LegalRecords`] reads it at the stellar
//!   (`_IsThreatToStellar` @0x9fb8, the per-system record by the engine);
//!   the player's class's combat government; whether its own government
//!   blesses peace ([`Session::blessing`]); whether `G` carries an IFF
//!   scrambler's mark (see the `iff_scramblers` module); and whether it is
//!   the stellar the player asked to dock at.
//!
//! **The timers** ([`GunTimer`], the original's +0x494 and +0x2e) are kept
//! on [`Space`](super::Space) by stellar, across systems, and zeroed for
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
use crate::market::DOMINATED;
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
                    dominated: site.flags2 & DOMINATED != 0,
                    defended: false,
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
            .active_holder(listers, self.pilot.system)
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

impl Session {
    /// Puts the player's ship at `at`, at rest, for
    /// [`fixture::place_player`](crate::fixture::place_player).
    #[cfg(any(test, feature = "fixture"))]
    pub(crate) fn place_player(&mut self, at: crate::geometry::Vec2) {
        self.ship.player.position = at;
        self.ship.player.velocity = crate::geometry::Vec2::ZERO;
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
    }

    #[test]
    fn a_stellar_waiting_for_its_defenders_never_fires() {
        let mut guarded = armed_128();
        guarded.sites[0].1[0].flags2 = crate::combat::stellar_guns::GUARDED_BY_DEFENDERS;
        assert!(!fires_at(&guarded, -1000, |_| {}));
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
}
