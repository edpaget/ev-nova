//! The stellars destroyed and regenerated on the session: `Y` and `U`,
//! handed to `Session::apply_op`, each read from the stellar's
//! [`StellarLife`](crate::StellarLife) the session read when it started.
//! Each writes the galaxy overlay through the writer's catalog-free entry,
//! `Session::set_stellar_state`, so every reader sees it at once (see the
//! [`story`](crate::story) module's "Galaxy overlay").
//!
//! - `Yxxx` destroys the stellar with its countdown, its `DeadTime` or 1
//!   when that is negative, whether it was destroyed or not, so a second
//!   `Y` restarts the countdown (@0x15ac8-0x15ade). By
//!   [`RuleKey::DestroyInvincible`]'s engine reading an invincible stellar
//!   (`Strength` 0 or -1) is left as it was. A stellar with no record, or
//!   outside the original's slots, 128 to 2175, is left alone. When the
//!   stellar is among those shown in flight and was not destroyed before,
//!   it makes the stellar's explosion at its centre, invincible or not
//!   (@0x15a59-0x15ab4). By [`RuleKey::YOnDestroy`]'s engine reading it
//!   runs no `OnDestroy`; by the Bible's it runs it once it has destroyed
//!   a whole stellar, unless that stellar's `OnDestroy` is running
//!   already, which is reported as a `ScriptNote` instead.
//! - `Uxxx` regenerates the stellar at once and runs no `OnRegen`, which
//!   runs only as it regenerates on its own (@0x15aeb-0x15b20).
//!
//! The daily step's sixth participant counts each destroyed stellar down
//! a day, by ascending ID (@0xb5c4-0xb71e): one of negative `DeadTime`
//! stays destroyed for ever; any other regenerates once its countdown
//! reaches 0, and runs its `OnRegen` there and then (@0xb6ee), so a
//! `DayPassed` subscription that day sees what it set. It repairs every
//! whole stellar first (@0xb700): nothing damages one within a day, so
//! forgetting all the damage at once is the original's pass by index.
//!
//! **Shots.** The fight's tick ([`Session::tick_combat`]) hands the
//! combat the stellars shown that shots can hit (whole, of `Strength`
//! above 0, with a sprite, as big as their disc), each with its current
//! strength, and keeps what the shots leave of it: the session holds the
//! current strength only of a whole stellar that has taken damage, never
//! saved, as the original saves none (@0x72ad0-0x72aff). `Y` and `U`
//! forget it. A stellar a shot takes below 0 is shot down
//! (`_CheckShotProximities` @0x37409-0x3751f): its explosion at its
//! centre, destroyed with the countdown `Y` gives it, then its `OnDestroy`
//! at once, its `M`, `N`, `Q` and `S` deferred to the next settling as any
//! hook's are, and destroyed with that countdown again whatever the
//! script did. A shot of the player's convicts it ten times of killing a
//! ship of the stellar's government, as overlaid; an NPC's convicts no one.
//!
//! A new pilot ([`Session::begin`]) starts with every stellar whose
//! `Flags2` has 0x0040 destroyed, with the countdown `Y` gives it, before
//! its `OnStart` runs (`_DoNewPilot` @0x192d6-0x1931c); a pilot loaded has
//! its stellars as saved.
//!
//! A stellar the player is docked at stays the one docked at when `Y`
//! destroys it: the spaceport stays open and the take-off works, as the
//! original's port never asks; only landing there again is refused.
//!
//! **Rebuild points.** The original rebuilds the stellar sprites of the
//! player's system (`_HandleStellarSystemVisibility` @0x32aa5) at its
//! rebuild points, and the session keeps which stellars shown that rebuild
//! found destroyed until a flight frame finds them whole
//! ([`Session::dead_since_rebuild`]; see the `story` module's "Built by
//! phase 9" and `story::rebuild`), with the stellar masters loaded and
//! the sprite slots left with no frame, which the player's landing
//! distance reads ([`Session::land`], by
//! [`RuleKey::LandingDistance`]; "Built by phase 10"). Each point
//! rebuilds after the
//! transition's own writes, and tells
//! [`SessionEvent::StellarsRebuilt`](super::SessionEvent::StellarsRebuilt)
//! when the set changed:
//!
//! - every way into a system but a landed `M` or `N`, as the last step of
//!   `Session::enter_system`: an in-flight `M` or `N` (@0x159a3), even
//!   into the same system, a gate's or a wormhole's exit (@0x63e8c,
//!   @0x6473e), a jump's arrival (@0x6c203 and @0x6c597, one point) and
//!   the developer tools' relocation, which docks as a landing does;
//! - a landing, as it docks ([`Session::land`]; `_DoPortDialog`
//!   @0x5fbf0), and the take-off ([`Session::take_off`]; @0x674c7 and
//!   @0x67653, one point), after the stay's days and the stellars that
//!   waited for it;
//! - a new pilot, after its stellars that start destroyed and its
//!   `OnStart` ([`Session::begin`]; `_DoNewPilot` @0x1937f);
//! - a pilot flown (`Session::fly`: a load, @0x751cf and @0x75324, and the
//!   main screen, @0x1a354, @0x1a8ca, @0x1b003 and @0x457ee), which tells
//!   nothing, the view reading the state as it builds.
//!
//! A landed `M` or `N`, `Y`, `U`, the daily step, the shots and the galaxy
//! overlay's changes are not rebuild points. The original's escape-pod
//! rescue rebuilds too (@0x69135): the engine has none, and whoever builds
//! it calls the rebuild. Its debug key that explores every system
//! (@0x6dad1) has no counterpart. The flight frame that drops a stellar
//! seen whole is the end of [`Session::tick_combat`] in flight, after the
//! shots, as the original's sprite tick (@0x444d2) follows
//! `_CheckShotProximities` (@0x44015).

use super::Session;
use crate::catalog::{LandingSite, StellarId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::stellar::StellarTarget;
use crate::control::{Script, ScriptNote};
use crate::landing::{LandingReach, StellarShown, landing_radius, landing_reach};
use crate::legal::{self, Crime, LegalCode};
use crate::rulebook::{RuleKey, RuleSource};
use crate::story::wreck::{self, Tick};
use crate::story::{Countdown, DrawnSheet};

/// How many killings the player is convicted of for shooting a stellar
/// down: `_SlapWithPenalty` runs ten times (@0x374e1-0x3751f).
const STELLAR_KILLINGS: usize = 10;

impl Session {
    /// Whether `stellar` has been dead since the last rebuild point: it
    /// is among the stellars shown, the last rebuild point found it
    /// destroyed, and no flight frame has found it whole since (see the
    /// module docs). A stellar with no `DeadType` sheet draws graphic type
    /// 0 while this holds, by
    /// [`RuleKey::DeadTypeFallback`]'s engine reading.
    #[must_use]
    pub fn dead_since_rebuild(&self, stellar: StellarId) -> bool {
        self.rebuilt.dead(stellar)
    }

    /// A rebuild point (see the module docs): the stellars shown that are
    /// destroyed now are the ones dead since it, telling
    /// [`SessionEvent::StellarsRebuilt`](super::SessionEvent::StellarsRebuilt)
    /// when that changed them.
    pub(super) fn rebuild_stellars(&mut self) {
        if self.rebuild_quietly() {
            self.emit(super::SessionEvent::StellarsRebuilt);
        }
    }

    /// A rebuild point that tells nothing: whether it changed which
    /// stellars shown are dead since the last one.
    pub(super) fn rebuild_quietly(&mut self) -> bool {
        let galaxy = &self.galaxy;
        self.rebuilt.rebuild(
            &self.sites,
            |id| galaxy.destroyed(id),
            self.fallback_frame.is_some(),
        )
    }

    /// Which sheet `stellar` draws (see the module docs): its `DeadType`
    /// sheet while destroyed, when that can be read; graphic type 0's
    /// while destroyed and dead since the last rebuild point, by
    /// [`RuleKey::DeadTypeFallback`]'s engine reading, whether or not
    /// graphic type 0's sheet can be read; its `Type` sheet otherwise, and
    /// for a stellar that is not among those shown. The click box
    /// ([`Session::select_stellar`]) and the views read this one answer.
    #[must_use]
    pub fn drawn_sheet(&self, stellar: StellarId) -> DrawnSheet {
        self.sites
            .iter()
            .find(|site| site.id == stellar)
            .map_or(DrawnSheet::Type, |site| self.site_sheet(site))
    }

    /// [`Session::drawn_sheet`] for `site`, one of the stellars shown.
    fn site_sheet(&self, site: &LandingSite) -> DrawnSheet {
        if !site.destroyed {
            DrawnSheet::Type
        } else if site.dead_frame_size.is_some() {
            DrawnSheet::Dead
        } else if self.rebuilt.dead(site.id)
            && self.rules.source_for(RuleKey::DeadTypeFallback) == RuleSource::Engine
        {
            DrawnSheet::TypeZero
        } else {
            DrawnSheet::Type
        }
    }

    /// The frame size of the sheet drawn for `site`, one of the stellars
    /// shown ([`Session::drawn_sheet`]), or `None` when no sheet is: the
    /// box a click on it reads (see the module docs).
    pub(super) fn drawn_frame_size(&self, site: &LandingSite) -> Option<(u32, u32)> {
        match self.site_sheet(site) {
            DrawnSheet::Type => site.frame_size,
            DrawnSheet::Dead => site.dead_frame_size,
            DrawnSheet::TypeZero => self.fallback_frame,
        }
    }

    /// A flight frame's sprite tick (see the module docs): every stellar
    /// it finds whole is no longer dead since the last rebuild point.
    pub(super) fn see_stellars(&mut self) {
        let galaxy = &self.galaxy;
        self.rebuilt.frame(&self.sites, |id| galaxy.destroyed(id));
    }

    /// How close the ship must be to `site`, one of the stellars shown, to
    /// land on it, by [`RuleKey::LandingDistance`]: the original's box
    /// reads what the last rebuild point and the flight frames since left
    /// of its sprite slot and its `Type` master (see the module docs).
    pub(super) fn landing_reach(&self, site: &LandingSite) -> LandingReach {
        let shown = StellarShown {
            frame: self.rebuilt.shows_frame(site.id),
            type_width: self.rebuilt.type_width(site),
        };
        landing_reach(site, shown, self.rules.source_for(RuleKey::LandingDistance))
    }

    /// `Y`: destroys `stellar` (see the module docs), drawing on `chance`
    /// for an `OnDestroy` the Bible's reading runs.
    pub(super) fn destroy_stellar(&mut self, stellar: StellarId, chance: &mut dyn Chance) {
        if !wreck::in_range(stellar) {
            return;
        }
        let Some(life) = self.lives.get(&stellar).cloned() else {
            return;
        };
        let destroyed_before = self.galaxy.destroyed(stellar);
        let site = self.sites.iter().find(|site| site.id == stellar).copied();
        let shown = site.is_some() && self.landed.is_none();
        if let Some(site) = site
            && let Some(explosion) = wreck::explodes_on_y(&life, shown, destroyed_before)
        {
            self.combat.explode(site.position, explosion);
        }
        self.strength.remove(&stellar);
        if wreck::destroys(&life, self.rules.source_for(RuleKey::DestroyInvincible)) {
            let countdown = wreck::countdown_on_destroy(&life);
            self.set_stellar_state(stellar, Some(countdown));
            if !destroyed_before && self.rules.source_for(RuleKey::YOnDestroy) == RuleSource::Bible
            {
                self.run_on_destroy(stellar, &life.on_destroy, chance);
            }
        }
    }

    /// `U`: regenerates `stellar` (see the module docs). Only `Y` and a
    /// new pilot's stellars destroy one, each within the original's
    /// slots, so any other is never destroyed.
    pub(super) fn regenerate_stellar(&mut self, stellar: StellarId) {
        self.strength.remove(&stellar);
        self.set_stellar_state(stellar, None);
    }

    /// Runs `stellar`'s `OnDestroy`, `script`, drawing on `chance`, unless
    /// it is running already: then it is reported once as
    /// [`ScriptNote::OnDestroyReentered`] and not run again, so a script
    /// that regenerates and destroys its own stellar, or two that destroy
    /// each other, end.
    fn run_on_destroy(
        &mut self,
        stellar: StellarId,
        script: &Script,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if !self.destroying.insert(stellar) {
            self.script_notes
                .push(ScriptNote::OnDestroyReentered { stellar });
            return;
        }
        self.run_script(script, chance);
        self.destroying.remove(&stellar);
    }

    /// The stellars shown that shots can hit this tick: each whole one of
    /// `Strength` above 0 with a sprite, as big as its disc
    /// ([`landing_radius`]), with its current strength.
    pub(super) fn stellar_targets(&self) -> Vec<StellarTarget> {
        self.sites
            .iter()
            .filter(|site| !site.destroyed && site.frame_size.is_some())
            .filter_map(|site| {
                let life = self.lives.get(&site.id)?;
                wreck::destroyable(life).then(|| StellarTarget {
                    stellar: site.id,
                    position: site.position,
                    radius: landing_radius(site),
                    strength: self
                        .strength
                        .get(&site.id)
                        .copied()
                        .unwrap_or_else(|| wreck::full_strength(life)),
                })
            })
            .collect()
    }

    /// Keeps the strength each of `stellars` is left with after the
    /// fight's tick, then shoots down each the fight's strikes destroyed,
    /// in order, under `law`, drawing on `chance`.
    pub(super) fn strike_stellars(
        &mut self,
        stellars: &[StellarTarget],
        law: &dyn LegalCode,
        chance: &mut (impl Chance + ?Sized),
    ) {
        for target in stellars {
            let full = self.lives.get(&target.stellar).map(wreck::full_strength);
            if full == Some(target.strength) {
                self.strength.remove(&target.stellar);
            } else {
                self.strength.insert(target.stellar, target.strength);
            }
        }
        for strike in self.combat.take_stellar_strikes() {
            if strike.destroyed {
                self.shoot_down(strike.stellar, strike.by, law, chance);
            }
        }
    }

    /// Destroys the shown `stellar` a shot of `by` took below 0, as
    /// `_CheckShotProximities` does (@0x37409-0x3751f): its explosion at
    /// its centre, destroyed with the countdown `Y` gives it, then its
    /// `OnDestroy`, drawing on `chance`, and destroyed with that countdown
    /// again whatever the script did. The player, and no other ship, is
    /// convicted ten times of killing a ship of its government, as `law`
    /// says.
    pub(super) fn shoot_down(
        &mut self,
        stellar: StellarId,
        by: ShipRef,
        law: &dyn LegalCode,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let (Some(life), Some(site)) = (
            self.lives.get(&stellar).cloned(),
            self.sites.iter().find(|site| site.id == stellar).copied(),
        ) else {
            return;
        };
        if let Some(explosion) = life.explosion {
            self.combat.explode(site.position, explosion);
        }
        let countdown = wreck::countdown_on_destroy(&life);
        self.set_stellar_state(stellar, Some(countdown));
        self.run_on_destroy(stellar, &life.on_destroy, chance);
        self.set_stellar_state(stellar, Some(countdown));
        self.strength.remove(&stellar);
        if by == ShipRef::Player {
            let changes = law.penalties(Crime::Kill, site.govt, &self.govts);
            for _ in 0..STELLAR_KILLINGS {
                legal::convict(&mut self.pilot, &changes);
            }
        }
    }

    /// The daily step's stellars (see the module docs): each destroyed
    /// stellar in turn, by ascending ID, counts a day down, or regenerates
    /// and runs its `OnRegen` at once, through [`Session::run_set`],
    /// drawing on `chance`. The overlay is read again after each, so a
    /// stellar an `OnRegen` destroys is counted the same day when its ID is
    /// higher, as the original's pass by index does. A destroyed stellar
    /// with no record is left as it is.
    pub(super) fn stellars_day(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.strength.clear();
        let mut after = None;
        while let Some((stellar, countdown)) = self.next_destroyed(after) {
            after = Some(stellar);
            let Some(life) = self.lives.get(&stellar).cloned() else {
                continue;
            };
            match wreck::day(&life, countdown) {
                Tick::Stays(countdown) => {
                    self.set_stellar_state(stellar, Some(countdown));
                }
                Tick::Regenerates => {
                    self.set_stellar_state(stellar, None);
                    self.run_script(&life.on_regen, chance);
                }
            }
        }
    }

    /// Destroys a new pilot's stellars that start the game destroyed
    /// (`Flags2` 0x0040), within the original's slots, each with the
    /// countdown `Y` gives it, as `_DoNewPilot` does before its `OnStart`
    /// (@0x192d6-0x1931c); by [`RuleKey::DestroyInvincible`]'s engine
    /// reading an invincible one starts whole, as `_StellarIsDestroyed`
    /// holds it.
    pub(super) fn destroy_from_the_start(&mut self) {
        let reading = self.rules.source_for(RuleKey::DestroyInvincible);
        let ruined: Vec<_> = self
            .lives
            .values()
            .filter(|life| {
                wreck::starts_destroyed(life)
                    && wreck::in_range(life.id)
                    && wreck::destroys(life, reading)
            })
            .map(|life| (life.id, wreck::countdown_on_destroy(life)))
            .collect();
        for (stellar, countdown) in ruined {
            self.set_stellar_state(stellar, Some(countdown));
        }
    }

    /// The destroyed stellar of lowest ID above `after` (any, with none),
    /// with its countdown.
    fn next_destroyed(&self, after: Option<StellarId>) -> Option<(StellarId, Countdown)> {
        self.pilot
            .galaxy
            .stellars()
            .filter(|&(id, _)| after.is_none_or(|after| id > after))
            .find_map(|(id, change)| Some((id, change.destroyed?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RelocateRefusal;
    use crate::ai::{Goal, Peaceful};
    use crate::catalog::{StellarLife, SystemId};
    use crate::chance::NeverFires;
    use crate::control::{Bit, Script, SetExpr, SetOp};
    use crate::landing::{LandingRefusal, StellarFlags};
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::session::{SessionEvent, StellarPick};
    use crate::story::Countdown;
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, land_now, life};

    /// [`catalog`] with 128 back after 2 days, 129 after 5, and 140
    /// never.
    fn wrecking() -> FakePilotCatalog {
        catalog().living([life(128, 2), life(129, 5), life(140, -1)])
    }

    fn run(session: &mut Session, text: &str) {
        session.run_set(
            &SetExpr::parse(text).expect("parses"),
            &mut Scripted::default(),
        );
    }

    /// `stellar`'s countdown, while it is destroyed.
    fn destroyed(session: &Session, stellar: i16) -> Option<Countdown> {
        session
            .galaxy()
            .overlay()
            .stellar(StellarId(stellar))
            .destroyed
    }

    fn left(days_left: u16) -> Countdown {
        Countdown { days_left }
    }

    /// Whether the stellar shown `stellar` is destroyed.
    fn shown_destroyed(session: &Session, stellar: i16) -> bool {
        session
            .sites
            .iter()
            .find(|site| site.id == StellarId(stellar))
            .expect("shown")
            .destroyed
    }

    fn quiet(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.take_events();
        session.take_save_due();
        session
    }

    #[test]
    fn the_session_reads_the_stellars_lives_once_as_it_starts() {
        let catalog = wrecking();
        assert_eq!(*catalog.life_reads.borrow(), 0);
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(*catalog.life_reads.borrow(), 1);
        run(&mut session, "Y128 Y129 U128");
        session.pass_days(6, &mut Scripted::default());
        assert_eq!(destroyed(&session, 129), None, "regenerated meanwhile");
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(*catalog.life_reads.borrow(), 1, "never again");
    }

    #[test]
    fn y_destroys_a_stellar_for_its_dead_time_as_every_reader_sees_it() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y129");
        assert_eq!(destroyed(&session, 129), Some(left(5)));
        assert_eq!(session.pilot().galaxy, *session.galaxy().overlay());
        assert!(shown_destroyed(&session, 129));
        assert!(!shown_destroyed(&session, 128), "the other");
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
        assert!(session.take_save_due());
        run(&mut session, "Y140");
        assert_eq!(destroyed(&session, 140), Some(left(1)), "never back: a day");
    }

    #[test]
    fn the_land_key_refuses_a_destroyed_stellar() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        assert_eq!(
            session.select_stellar(StellarPick::Slot(0)),
            Some(StellarId(128))
        );
        assert_eq!(
            session.land(),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(128),
                station: false,
            })
        );
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn f5_skips_a_destroyed_stellar() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        assert_eq!(
            session.select_stellar(StellarPick::Nearest),
            Some(StellarId(128))
        );
        run(&mut session, "Y128");
        assert_eq!(
            session.select_stellar(StellarPick::Nearest),
            Some(StellarId(129))
        );
        run(&mut session, "U128");
        assert_eq!(
            session.select_stellar(StellarPick::Nearest),
            Some(StellarId(128))
        );
    }

    /// [`wrecking`] with 128 and 140 landable only once destroyed.
    fn ruins() -> FakePilotCatalog {
        let mut catalog = wrecking();
        for (_, sites) in &mut catalog.sites {
            for site in sites {
                if matches!(site.id.0, 128 | 140) {
                    site.flags |= StellarFlags::ONLY_WHEN_DESTROYED;
                }
            }
        }
        catalog
    }

    #[test]
    fn a_stellar_landable_only_once_destroyed_becomes_landable_and_dockable() {
        let catalog = ruins();
        let mut session = quiet(&catalog);
        session.select_stellar(StellarPick::Slot(0));
        assert!(matches!(
            session.land(),
            Err(LandingRefusal::NotLandable { .. })
        ));
        run(&mut session, "Y128");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(
            session.relocate(&catalog, SystemId(131), StellarId(140)),
            Err(RelocateRefusal::NotLandable)
        );
        run(&mut session, "Y140");
        assert_eq!(
            session.relocate(&catalog, SystemId(131), StellarId(140)),
            Ok(())
        );
        session.assert_docked_among_shown();
        assert_eq!(session.landed(), Some(StellarId(140)));
    }

    #[test]
    fn a_second_y_restarts_the_countdown() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        session.set_stellar_state(StellarId(128), Some(left(1)));
        session.take_events();
        run(&mut session, "Y128");
        assert_eq!(destroyed(&session, 128), Some(left(2)));
        assert_eq!(session.take_events(), [], "destroyed already");
        assert!(session.take_save_due(), "a new countdown");
    }

    /// [`wrecking`] with 128 of `strength`.
    fn invincible(strength: i32) -> FakePilotCatalog {
        catalog().living([StellarLife {
            strength,
            ..life(128, 2)
        }])
    }

    #[test]
    fn by_the_engine_y_leaves_an_invincible_stellar_as_it_was() {
        for strength in [0, -1] {
            let catalog = invincible(strength);
            let mut session = quiet(&catalog);
            run(&mut session, "Y128");
            assert_eq!(destroyed(&session, 128), None, "{strength}");
            assert_eq!(session.take_events(), [], "{strength}");
            assert!(!session.take_save_due(), "{strength}");
            assert_eq!(
                session.rules().source_for(RuleKey::DestroyInvincible),
                RuleSource::Engine,
                "by default"
            );
        }
    }

    #[test]
    fn by_the_bible_y_destroys_an_invincible_stellar() {
        let rules =
            Rulebook::default().with_override(RuleKey::DestroyInvincible, RuleSource::Bible);
        for strength in [0, -1] {
            let catalog = invincible(strength);
            let mut session = quiet(&catalog).with_rules(rules);
            run(&mut session, "Y128");
            assert_eq!(destroyed(&session, 128), Some(left(2)), "{strength}");
        }
    }

    #[test]
    fn y_on_a_stellar_with_no_record_or_outside_the_slots_does_nothing() {
        let catalog = catalog().living([life(127, 2), life(2176, 2), life(2175, 2)]);
        let mut session = quiet(&catalog);
        run(&mut session, "Y300");
        for id in [127, 2176] {
            session
                .apply_op(
                    &SetOp::DestroyStellar(StellarId(id)),
                    &mut Scripted::default(),
                )
                .expect("handled");
        }
        assert!(session.galaxy().overlay().is_empty());
        assert_eq!(session.take_events(), []);
        assert!(!session.take_save_due());
        run(&mut session, "Y2175");
        assert_eq!(destroyed(&session, 2175), Some(left(2)), "the last slot");
    }

    #[test]
    fn u_regenerates_a_stellar_and_runs_no_on_regen() {
        let catalog = catalog().living([StellarLife {
            on_regen: Script::parse("b5"),
            ..life(128, 2)
        }]);
        let mut session = quiet(&catalog);
        run(&mut session, "U128");
        assert_eq!(session.take_events(), [], "whole already");
        assert!(!session.take_save_due());
        run(&mut session, "Y128 U128");
        assert_eq!(destroyed(&session, 128), None);
        assert!(!shown_destroyed(&session, 128));
        assert!(
            !session.control_bit(Bit::new(5).expect("a bit")),
            "no OnRegen"
        );
        assert_eq!(
            session.take_events(),
            [SessionEvent::GalaxyChanged, SessionEvent::GalaxyChanged]
        );
        assert!(session.galaxy().overlay().is_empty());
    }

    #[test]
    fn y_on_the_stellar_docked_at_leaves_the_player_docked_until_the_take_off() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        run(&mut session, "Y128");
        assert_eq!(session.landed(), Some(StellarId(128)), "still docked");
        session.assert_docked_among_shown();
        assert_eq!(session.pilot().stellar, Some(StellarId(128)));
        assert_eq!(
            session.take_off(&mut Scripted::default()),
            Some(StellarId(128))
        );
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(
            session.land(),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(128),
                station: false,
            }),
            "not again"
        );
    }

    // The daily step's stellars.

    /// [`catalog`] with these lives, 128 and 129's `OnRegen` as given.
    fn regenerating(lives: [(i16, i16, &str); 2]) -> FakePilotCatalog {
        catalog().living(lives.map(|(id, dead_time, on_regen)| StellarLife {
            on_regen: Script::parse(on_regen),
            ..life(id, dead_time)
        }))
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("a bit")
    }

    #[test]
    fn a_destroyed_stellar_counts_its_days_down_and_regenerates_running_its_on_regen() {
        use crate::story::{Consumer, EventFilter, StoryEventKind, Subscription, SubscriptionId};
        let catalog = regenerating([(129, 2, "b7"), (128, 0, "")]);
        let mut session = quiet(&catalog);
        let id = SubscriptionId {
            consumer: Consumer::Fixture,
            record: 1,
            hook: 0,
        };
        session
            .register_story(Subscription {
                guard: crate::control::Test::parse("b7"),
                ..Subscription::new(
                    id,
                    EventFilter::Any(StoryEventKind::DayPassed),
                    Script::parse("b8"),
                )
            })
            .expect("registers");
        land_now(&mut session).expect("lands");
        run(&mut session, "Y129");
        session
            .take_off(&mut Scripted::default())
            .expect("takes off");
        assert_eq!(destroyed(&session, 129), Some(left(1)), "a day down");
        assert!(!session.control_bit(bit(7)));
        land_now(&mut session).expect("lands");
        let _ = session.settle_script(&catalog, &mut Scripted::default());
        assert!(!session.control_bit(bit(8)), "not yet");
        session.take_events();
        session
            .take_off(&mut Scripted::default())
            .expect("takes off");
        assert_eq!(destroyed(&session, 129), None, "regenerated");
        assert!(!shown_destroyed(&session, 129));
        assert!(session.control_bit(bit(7)), "its OnRegen, in the day");
        assert!(
            session.take_events().contains(&SessionEvent::GalaxyChanged),
            "told"
        );
        let _ = session.settle_script(&catalog, &mut Scripted::default());
        assert!(session.control_bit(bit(8)), "the day's DayPassed saw it");
    }

    #[test]
    fn a_stellar_of_dead_time_0_or_1_regenerates_on_its_next_day() {
        for dead_time in [0, 1] {
            let catalog = regenerating([(129, dead_time, "b7"), (128, 0, "")]);
            let mut session = quiet(&catalog);
            run(&mut session, "Y129");
            session.pass_days(1, &mut Scripted::default());
            assert_eq!(destroyed(&session, 129), None, "{dead_time}");
            assert!(session.control_bit(bit(7)), "{dead_time}");
        }
    }

    #[test]
    fn days_passed_together_each_count_a_day() {
        let catalog = regenerating([(129, 5, ""), (128, 0, "")]);
        let mut session = quiet(&catalog);
        session.stats.jump_days = 3;
        run(&mut session, "Y129");
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        assert_eq!(destroyed(&session, 129), Some(left(2)));
    }

    #[test]
    fn a_stellar_of_negative_dead_time_stays_destroyed() {
        let catalog = regenerating([(140, -1, "b7"), (128, 0, "")]);
        let mut session = quiet(&catalog);
        run(&mut session, "Y140");
        session.pass_days(10, &mut Scripted::default());
        assert_eq!(destroyed(&session, 140), Some(left(1)));
        assert!(!session.control_bit(bit(7)));
    }

    #[test]
    fn an_on_regen_destroying_a_higher_stellar_has_it_counted_the_same_day() {
        let catalog = regenerating([(128, 1, "Y129"), (129, 1, "b9")]);
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), None);
        assert_eq!(destroyed(&session, 129), None, "counted the same day");
        assert!(session.control_bit(bit(9)));
    }

    #[test]
    fn an_on_regen_destroying_a_lower_stellar_has_it_counted_from_the_next_day() {
        let catalog = regenerating([(128, 1, "b9"), (129, 1, "Y128")]);
        let mut session = quiet(&catalog);
        run(&mut session, "Y129");
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(destroyed(&session, 129), None);
        assert_eq!(destroyed(&session, 128), Some(left(1)), "passed already");
        assert!(!session.control_bit(bit(9)));
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), None);
        assert!(session.control_bit(bit(9)));
    }

    #[test]
    fn a_destroyed_stellar_with_no_record_is_left_as_it_is() {
        let catalog = regenerating([(129, 1, ""), (128, 0, "")]);
        let mut session = quiet(&catalog);
        session.set_stellar_state(StellarId(300), Some(left(4)));
        run(&mut session, "Y129");
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(destroyed(&session, 300), Some(left(4)));
        assert_eq!(destroyed(&session, 129), None, "the next one counted");
    }

    #[test]
    fn by_the_bible_a_destroyed_invincible_stellar_counts_down_as_any() {
        let rules =
            Rulebook::default().with_override(RuleKey::DestroyInvincible, RuleSource::Bible);
        let catalog = catalog().living([StellarLife {
            strength: 0,
            ..life(128, 2)
        }]);
        let mut session = quiet(&catalog).with_rules(rules);
        run(&mut session, "Y128");
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), Some(left(1)));
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), None);
    }

    // The NPCs' destinations.

    /// [`wrecking`] with two ships of düde 128, fast ship 129, flying in
    /// 130.
    fn trafficked() -> FakePilotCatalog {
        use crate::catalog::{DudeId, DudeRecord, GovtId, ShipId, SystemTraffic};
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        FakePilotCatalog {
            traffic: vec![(
                SystemId(130),
                SystemTraffic {
                    dude_types,
                    avg_ships: 2,
                    persons: Default::default(),
                },
            )],
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type: 1,
                    govt: Some(GovtId(140)),
                    ships: vec![(ShipId(129), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )],
            ship_records: vec![crate::testkit::ship(129, crate::testkit::FAST)],
            ..wrecking()
        }
    }

    /// The goal of every NPC in `session`'s system after one traffic tick,
    /// each bound for 128 before it.
    fn goals_bound_for_128(session: &mut Session, catalog: &FakePilotCatalog) -> Vec<Goal> {
        for npc in session.traffic.npcs_mut() {
            npc.goal = Goal::Land(StellarId(128));
        }
        session.tick_traffic(catalog, &Peaceful, &mut NeverFires);
        session.npcs().iter().map(|npc| npc.goal).collect()
    }

    #[test]
    fn an_npc_never_heads_for_a_stellar_y_destroyed() {
        let catalog = trafficked();
        let mut session = quiet(&catalog);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert!(!session.npcs().is_empty(), "populated");
        assert!(
            goals_bound_for_128(&mut session, &catalog)
                .iter()
                .all(|&goal| goal == Goal::Land(StellarId(128))),
            "kept while whole"
        );
        run(&mut session, "Y128");
        let goals = goals_bound_for_128(&mut session, &catalog);
        assert!(!goals.is_empty());
        assert!(
            goals.iter().all(|&goal| goal == Goal::Land(StellarId(129))),
            "{goals:?}: decided again, for the one landable"
        );
    }

    // A landed `M`.

    /// [`wrecking`]'s session landed at 128, after `script` and a landed
    /// `M131` settle: the pilot's stellar is then 131's first, 140.
    fn moved_landed(catalog: &FakePilotCatalog, script: &str) -> Session {
        let mut session = quiet(catalog);
        land_now(&mut session).expect("lands");
        run(&mut session, script);
        let _ = session.settle_script(catalog, &mut Scripted::default());
        session.assert_docked_among_shown();
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.pilot().stellar, Some(StellarId(140)), "{script}");
        assert_eq!(
            session.landed(),
            Some(StellarId(128)),
            "the spaceport open stays 128's"
        );
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(catalog, saved).expect("flies");
        assert_eq!(
            flown.landed(),
            Some(StellarId(140)),
            "{script}: docked there"
        );
        flown.assert_docked_among_shown();
        session
    }

    #[test]
    fn a_landed_m_docks_at_the_first_stellar_destroyed_or_not() {
        let catalog = wrecking();
        let mut session = moved_landed(&catalog, "Y140 M131");
        assert!(session.next_sites.as_deref().expect("waiting")[0].destroyed);
        assert_eq!(
            session.take_off(&mut Scripted::default()),
            Some(StellarId(128))
        );
        assert_eq!(
            session.select_stellar(StellarPick::Slot(0)),
            Some(StellarId(140))
        );
        assert_eq!(
            session.land(),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(140),
                station: false,
            }),
            "not while it is destroyed"
        );
        let mut session = moved_landed(&catalog, "M131");
        session.take_off(&mut Scripted::default());
        session.select_stellar(StellarPick::Slot(0));
        assert!(session.land().is_ok(), "whole");
    }

    #[test]
    fn a_landed_m_docks_at_a_first_stellar_landable_only_once_destroyed_destroyed_or_not() {
        let mut catalog = wrecking();
        catalog.sites[1].1[0].flags |= StellarFlags::ONLY_WHEN_DESTROYED;
        assert_eq!(catalog.sites[1].1[0].id, StellarId(140));
        for script in ["Y140 M131", "M131"] {
            let mut session = moved_landed(&catalog, script);
            session.take_off(&mut Scripted::default());
            session.select_stellar(StellarPick::Slot(0));
            assert_eq!(
                session.land().is_ok(),
                script.starts_with('Y'),
                "{script}: landable only once destroyed"
            );
        }
    }

    // A new pilot's stellars.

    /// [`catalog`] whose `chär` runs `on_start`, with 128 (back after 3
    /// days), 129 (never back) and 2176 starting the game destroyed, and
    /// 140 whole.
    fn ruined_from_the_start(on_start: &str) -> FakePilotCatalog {
        let mut catalog = catalog().living(
            [
                (128, 3, true),
                (129, -1, true),
                (140, 2, false),
                (2176, 2, true),
            ]
            .map(|(id, dead_time, starts_destroyed)| StellarLife {
                // `Flags2` 0x0040 starts it destroyed; 0x0020, which
                // starts it dominated, does not.
                flags2: if starts_destroyed { 0x0060 } else { 0x0020 },
                ..life(id, dead_time)
            }),
        );
        if let Ok(start) = &mut catalog.character {
            start.on_start = Script::parse(on_start);
        }
        catalog
    }

    #[test]
    fn a_new_pilot_begins_with_the_stellars_that_start_the_game_destroyed() {
        let catalog = ruined_from_the_start("");
        let mut session = quiet(&catalog);
        assert!(session.galaxy().overlay().is_empty(), "flying alone");
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), Some(left(3)));
        assert_eq!(destroyed(&session, 129), Some(left(1)));
        assert_eq!(destroyed(&session, 140), None);
        assert_eq!(destroyed(&session, 2176), None, "outside the slots");
        assert!(shown_destroyed(&session, 128));
        assert!(session.take_save_due());
    }

    #[test]
    fn a_new_pilots_on_start_sees_its_stellars_destroyed() {
        let catalog = ruined_from_the_start("U128");
        let mut session = quiet(&catalog);
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), None, "regenerated by OnStart");
        assert_eq!(destroyed(&session, 129), Some(left(1)));
    }

    #[test]
    fn by_the_engine_an_invincible_stellar_does_not_start_the_game_destroyed() {
        let mut catalog = ruined_from_the_start("");
        catalog.lives[0].strength = 0;
        let mut session = quiet(&catalog);
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), None);
        let rules =
            Rulebook::default().with_override(RuleKey::DestroyInvincible, RuleSource::Bible);
        let mut session = quiet(&catalog).with_rules(rules);
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), Some(left(3)), "by the Bible");
    }

    #[test]
    fn a_loaded_pilot_starts_no_stellar_destroyed() {
        let catalog = ruined_from_the_start("");
        let mut session = quiet(&catalog);
        session.begin(&catalog, &mut Scripted::default());
        run(&mut session, "U128");
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(destroyed(&flown, 128), None, "as saved");
        assert_eq!(destroyed(&flown, 129), Some(left(1)), "as saved");
    }

    #[test]
    fn a_pilot_saved_docked_at_a_stellar_destroyed_since_resumes_docked_there() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        land_now(&mut session).expect("lands");
        run(&mut session, "Y128");
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(flown.landed(), Some(StellarId(128)));
        assert!(shown_destroyed(&flown, 128));
        assert_eq!(destroyed(&flown, 128), Some(left(2)));
    }

    // Stellars shot down.

    use crate::catalog::{
        GovtId, GovtRecord, HullRecord, Penalties, StockWeapon, WeaponId, WeaponRecord,
    };
    use crate::combat::armament::{MOD_WEAPON, Trigger};
    use crate::combat::weapon::{Explosion, PLANET_TYPE};
    use crate::combat::{CombatEvent, Rules, ShipRef};
    use crate::geometry::Vec2;
    use crate::govt::{DERELICT, Governments};
    use crate::legal::{Crime, LegalCode, NovaLaw, RECORD_LIMIT};
    use crate::testkit::{govt, hull, outfit, weapon};
    use crate::traffic::npc::NpcId;

    /// A planet-type gun firing every tick, 15 pixels a tick, doing 30
    /// mass and 20 energy damage and exploding in `bööm` 130 where it
    /// hits.
    fn planet_gun() -> WeaponRecord {
        WeaponRecord {
            count: 30,
            speed: 1500,
            mass_dmg: 30,
            energy_dmg: 20,
            flags2: PLANET_TYPE,
            explod_type: 2,
            ..weapon(128)
        }
    }

    /// [`catalog`] with `lives`, its ship carrying a [`planet_gun`]. The
    /// ship starts over 128, so each shot hits 128 on the tick it is
    /// fired: 128 of `Strength` 100 is shot down by the third.
    fn shooting(lives: impl IntoIterator<Item = StellarLife>) -> FakePilotCatalog {
        FakePilotCatalog {
            weapons: vec![planet_gun()],
            hulls: vec![HullRecord {
                weapons: vec![StockWeapon {
                    weapon: WeaponId(128),
                    count: 1,
                    ammo: 0,
                }],
                ..hull(128)
            }],
            outfits: vec![outfit(250, &[(MOD_WEAPON, 128)])],
            ..catalog().living(lives)
        }
    }

    /// 128, back after 2 days, running `on_destroy` as a shot destroys
    /// it.
    fn destroying(on_destroy: &str) -> StellarLife {
        StellarLife {
            on_destroy: Script::parse(on_destroy),
            ..life(128, 2)
        }
    }

    const FIRE: Trigger = Trigger {
        primary: true,
        secondary: None,
        only: None,
        turrets_only: false,
        bays: false,
    };

    /// Fires every tick for `ticks` ticks under `law`, then lets go: the
    /// fight's events.
    fn fire_by(session: &mut Session, ticks: u32, law: &dyn LegalCode) -> Vec<CombatEvent> {
        session.hold_trigger(FIRE);
        let mut events = Vec::new();
        for _ in 0..ticks {
            let rules = Rules {
                law,
                ..Rules::default()
            };
            session.tick_combat(rules, &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        session.hold_trigger(Trigger::default());
        events
    }

    fn fire(session: &mut Session, ticks: u32) -> Vec<CombatEvent> {
        fire_by(session, ticks, &NovaLaw::default())
    }

    /// The explosions among `events` of `bööm` `kind`, where each was.
    fn booms(events: &[CombatEvent], kind: i16) -> Vec<Vec2> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Exploded { at, explosion }
                    if Some(explosion) == Explosion::decode(kind) =>
                {
                    Some(at)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn shots_that_take_a_stellar_below_none_destroy_it_and_run_its_on_destroy_once() {
        let catalog = shooting([destroying("^b5")]);
        let mut session = quiet(&catalog);
        fire(&mut session, 2);
        assert_eq!(destroyed(&session, 128), None, "none left: still whole");
        assert_eq!(session.take_events(), []);
        assert!(!session.take_save_due());
        fire(&mut session, 1);
        assert_eq!(destroyed(&session, 128), Some(left(2)));
        assert!(shown_destroyed(&session, 128));
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
        assert!(session.take_save_due());
        assert!(session.control_bit(bit(5)), "its OnDestroy");
        fire(&mut session, 3);
        assert!(
            session.control_bit(bit(5)),
            "once: a toggle run again clears it"
        );
        assert_eq!(destroyed(&session, 128), Some(left(2)));
        assert_eq!(
            session.select_stellar(StellarPick::Slot(0)),
            Some(StellarId(128))
        );
        assert_eq!(
            session.land(),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(128),
                station: false,
            })
        );
    }

    #[test]
    fn an_on_destroy_regenerating_its_stellar_leaves_it_destroyed_all_the_same() {
        let catalog = shooting([destroying("U128 b5")]);
        let mut session = quiet(&catalog);
        fire(&mut session, 3);
        assert!(session.control_bit(bit(5)));
        assert_eq!(destroyed(&session, 128), Some(left(2)));
    }

    #[test]
    fn an_on_destroy_sees_its_stellar_destroyed() {
        let catalog = shooting([StellarLife {
            explosion: Explosion::decode(4),
            ..destroying("Y128")
        }]);
        let mut session = quiet(&catalog);
        let events = fire(&mut session, 3);
        assert_eq!(destroyed(&session, 128), Some(left(2)));
        assert_eq!(
            booms(&events, 4),
            [Vec2::new(30.0, -40.0)],
            "the shot's destruction alone: its Y found it destroyed already"
        );
    }

    #[test]
    fn an_on_destroys_move_waits_for_the_settling() {
        let catalog = shooting([destroying("M131")]);
        let mut session = quiet(&catalog);
        fire(&mut session, 3);
        assert_eq!(session.system(), SystemId(130), "not yet");
        let _ = session.settle_script(&catalog, &mut Scripted::default());
        assert_eq!(session.system(), SystemId(131));
    }

    /// Fines each government a `kill` penalty of -3 for a killing, and
    /// records each crime it is asked about with the victim's
    /// government.
    #[derive(Debug, Default)]
    struct Fine {
        seen: std::cell::RefCell<Vec<(Crime, Option<GovtId>)>>,
    }

    impl LegalCode for Fine {
        fn penalties(
            &self,
            crime: Crime,
            victim: Option<GovtId>,
            _govts: &Governments,
        ) -> Vec<(GovtId, i32)> {
            self.seen.borrow_mut().push((crime, victim));
            vec![(GovtId(140), -3)]
        }
    }

    /// [`shooting`]'s session with 128 given to government 141 by the
    /// galaxy overlay.
    fn overlaid(catalog: &FakePilotCatalog) -> Session {
        let mut session = quiet(catalog);
        session.change_galaxy(catalog, |overlay| {
            overlay.set_stellar_govt(StellarId(128), Some(Some(GovtId(141))));
        });
        session.take_events();
        session
    }

    #[test]
    fn the_player_shooting_a_stellar_down_is_convicted_of_ten_killings_of_its_government() {
        let catalog = shooting([life(128, 2)]);
        let mut session = overlaid(&catalog);
        let fine = Fine::default();
        fire_by(&mut session, 2, &fine);
        assert_eq!(fine.seen.take(), [], "no crime until it is down");
        fire_by(&mut session, 1, &fine);
        assert!(
            fine.seen
                .take()
                .iter()
                .all(|&seen| seen == (Crime::Kill, Some(GovtId(141)))),
            "a killing, of the overlay's government"
        );
        assert_eq!(session.pilot().legal_record(GovtId(140)), -30, "ten times");
        let mut session = overlaid(&catalog);
        let floor = -RECORD_LIMIT as i16;
        session.pilot.set_legal_record(GovtId(140), floor + 10);
        fire_by(&mut session, 3, &fine);
        assert_eq!(
            session.pilot().legal_record(GovtId(140)),
            floor,
            "each held to the limit"
        );
    }

    #[test]
    fn an_npc_shooting_a_stellar_down_convicts_no_one() {
        let catalog = shooting([destroying("b5")]);
        let mut session = quiet(&catalog);
        let fine = Fine::default();
        session.shoot_down(
            StellarId(128),
            ShipRef::Npc(NpcId(1)),
            &fine,
            &mut NeverFires,
        );
        assert_eq!(destroyed(&session, 128), Some(left(2)));
        assert!(session.control_bit(bit(5)));
        assert_eq!(fine.seen.take(), []);
        assert_eq!(session.pilot().legal_record(GovtId(140)), 0);
    }

    #[test]
    fn shooting_down_a_stellar_of_a_derelict_government_is_no_crime() {
        for (flags, crime) in [(DERELICT, false), (0, true)] {
            let mut catalog = shooting([life(128, 2)]);
            catalog.govts = vec![
                GovtRecord {
                    flags,
                    penalties: Penalties {
                        kill: 5,
                        ..Penalties::default()
                    },
                    ..govt(141)
                },
                govt(142),
            ];
            let mut session = overlaid(&catalog);
            fire(&mut session, 3);
            assert_eq!(destroyed(&session, 128), Some(left(2)));
            assert_eq!(
                session
                    .pilot()
                    .legal_records()
                    .any(|(_, record)| record != 0),
                crime,
                "{flags:#x}"
            );
        }
    }

    #[test]
    fn a_stellar_shot_down_explodes_at_its_centre() {
        let catalog = shooting([StellarLife {
            explosion: Explosion::decode(4),
            ..life(128, 2)
        }]);
        let mut session = quiet(&catalog);
        assert_eq!(booms(&fire(&mut session, 2), 4), []);
        let events = fire(&mut session, 1);
        assert_eq!(booms(&events, 4), [Vec2::new(30.0, -40.0)]);
        assert_eq!(booms(&events, 2).len(), 1, "the shot's own");
    }

    /// [`shooting`]'s session with the ship 200 pixels left of 128's
    /// centre and `off` below it, at rest, facing right, after one shot
    /// and the 20 ticks it then flies: the fight's events.
    fn shot_from_afar(off: f32) -> Vec<CombatEvent> {
        let catalog = shooting([life(128, 2)]);
        let mut session = quiet(&catalog);
        session.player = crate::flight::ShipState {
            position: Vec2::new(30.0 - 200.0, -40.0 + off),
            velocity: Vec2::ZERO,
            heading: 90.0,
        };
        let mut events = fire(&mut session, 1);
        for _ in 0..20 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        events
    }

    #[test]
    fn a_shot_strikes_a_stellar_on_the_edge_of_its_disc() {
        let seen = booms(&shot_from_afar(0.0), 2);
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(
            (seen[0] - Vec2::new(30.0 - 50.0, -40.0)).length() < 1e-3,
            "half its 100-pixel frame from its centre: {seen:?}"
        );
    }

    #[test]
    fn a_shot_passing_just_outside_a_stellars_disc_misses_it() {
        assert_eq!(booms(&shot_from_afar(51.0), 2), []);
    }

    #[test]
    fn shots_pass_through_an_invincible_stellar_and_one_with_no_sprite() {
        let catalog = shooting([StellarLife {
            strength: 0,
            ..life(128, 2)
        }]);
        let mut session = quiet(&catalog);
        let events = fire(&mut session, 5);
        assert_eq!(booms(&events, 2), [], "no hit");
        assert_eq!(destroyed(&session, 128), None);
        let mut catalog = shooting([life(128, 2)]);
        catalog.sites[0].1[0].frame_size = None;
        let mut session = quiet(&catalog);
        let events = fire(&mut session, 5);
        assert_eq!(booms(&events, 2), [], "no hit");
        assert_eq!(destroyed(&session, 128), None);
    }

    // The daily repair.

    #[test]
    fn a_damaged_stellar_is_repaired_by_the_day() {
        let catalog = shooting([life(128, 2)]);
        let mut session = quiet(&catalog);
        fire(&mut session, 1);
        session.pass_days(1, &mut Scripted::default());
        fire(&mut session, 2);
        assert_eq!(destroyed(&session, 128), None, "repaired: 100 again");
        fire(&mut session, 1);
        assert_eq!(destroyed(&session, 128), Some(left(2)));
    }

    #[test]
    fn y_forgets_a_stellars_damage() {
        let catalog = shooting([life(128, 2)]);
        let mut session = quiet(&catalog);
        fire(&mut session, 1);
        assert_eq!(session.strength.get(&StellarId(128)), Some(&50));
        run(&mut session, "Y128");
        assert!(
            session.strength.is_empty(),
            "destroyed: no current strength"
        );
    }

    #[test]
    fn u_repairs_a_damaged_stellar_destroyed_or_not() {
        for script in ["U128", "Y128 U128"] {
            let catalog = shooting([life(128, 2)]);
            let mut session = quiet(&catalog);
            fire(&mut session, 1);
            run(&mut session, script);
            fire(&mut session, 2);
            assert_eq!(destroyed(&session, 128), None, "{script}");
            fire(&mut session, 1);
            assert_eq!(destroyed(&session, 128), Some(left(2)), "{script}");
        }
    }

    #[test]
    fn a_jumps_days_repair_the_damaged_stellars() {
        let catalog = shooting([life(128, 2)]);
        let mut session = quiet(&catalog);
        fire(&mut session, 1);
        assert!(!session.strength.is_empty(), "damaged");
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        assert!(session.strength.is_empty());
    }

    // `Y`'s explosion.

    /// [`catalog`] with 128 and 140 (in 131) exploding in `bööm` 132, of
    /// `strength`.
    fn exploding(strength: i32) -> FakePilotCatalog {
        catalog().living([128, 140].map(|id| StellarLife {
            strength,
            explosion: Explosion::decode(4),
            ..life(id, 2)
        }))
    }

    #[test]
    fn y_explodes_a_stellar_of_the_players_system_in_flight() {
        let catalog = exploding(100);
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        assert_eq!(
            booms(&session.take_combat_events(), 4),
            [Vec2::new(30.0, -40.0)]
        );
        run(&mut session, "Y128");
        assert_eq!(
            booms(&session.take_combat_events(), 4),
            [],
            "destroyed already"
        );
        run(&mut session, "Y140");
        assert_eq!(
            booms(&session.take_combat_events(), 4),
            [],
            "another system"
        );
    }

    #[test]
    fn by_the_engine_y_explodes_an_invincible_stellar_it_leaves_whole() {
        let catalog = exploding(0);
        let mut session = quiet(&catalog);
        for _ in 0..2 {
            run(&mut session, "Y128");
            assert_eq!(destroyed(&session, 128), None);
            assert_eq!(
                booms(&session.take_combat_events(), 4),
                [Vec2::new(30.0, -40.0)],
                "never destroyed: every time"
            );
        }
    }

    #[test]
    fn y_makes_no_explosion_for_a_stellar_without_one_or_while_landed() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        assert_eq!(session.take_combat_events(), []);
        let catalog = exploding(100);
        let mut session = quiet(&catalog);
        land_now(&mut session).expect("lands");
        run(&mut session, "Y128");
        assert_eq!(session.take_combat_events(), [], "landed");
    }

    // `y_on_destroy`.

    #[test]
    fn by_the_engine_y_runs_no_on_destroy() {
        let catalog = catalog().living([destroying("b5")]);
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        assert!(!session.control_bit(bit(5)));
        assert_eq!(
            session.rules().source_for(RuleKey::YOnDestroy),
            RuleSource::Engine,
            "by default"
        );
    }

    #[test]
    fn by_the_bible_y_runs_on_destroy_as_it_destroys_a_stellar() {
        let rules = Rulebook::default().with_override(RuleKey::YOnDestroy, RuleSource::Bible);
        let catalog = catalog().living([destroying("^b5")]);
        let mut session = quiet(&catalog).with_rules(rules);
        run(&mut session, "Y128");
        assert!(session.control_bit(bit(5)), "its OnDestroy");
        assert_eq!(destroyed(&session, 128), Some(left(2)));
        run(&mut session, "Y128");
        assert!(session.control_bit(bit(5)), "not again: destroyed already");
        let catalog = crate::testkit::catalog().living([StellarLife {
            strength: 0,
            ..destroying("b5")
        }]);
        let mut session = quiet(&catalog).with_rules(rules);
        run(&mut session, "Y128");
        assert!(
            !session.control_bit(bit(5)),
            "an invincible stellar stays whole"
        );
    }

    #[test]
    fn by_the_bible_y_runs_on_destroy_with_its_stellar_destroyed_already() {
        let rules = Rulebook::default().with_override(RuleKey::YOnDestroy, RuleSource::Bible);
        let catalog = catalog().living([destroying("U128")]);
        let mut session = quiet(&catalog).with_rules(rules);
        run(&mut session, "Y128");
        assert_eq!(
            destroyed(&session, 128),
            None,
            "its U found it destroyed, and regenerated it"
        );
        assert_eq!(
            session.take_events(),
            [SessionEvent::GalaxyChanged, SessionEvent::GalaxyChanged]
        );
    }

    // An `OnDestroy` running already.

    use crate::control::ScriptNote;

    fn bible() -> Rulebook {
        Rulebook::default().with_override(RuleKey::YOnDestroy, RuleSource::Bible)
    }

    fn reentered(stellar: i16) -> ScriptNote {
        ScriptNote::OnDestroyReentered {
            stellar: StellarId(stellar),
        }
    }

    #[test]
    fn by_the_bible_an_on_destroy_destroying_its_stellar_again_runs_once() {
        let catalog = catalog().living([destroying("^b5 U128 Y128")]);
        let mut session = quiet(&catalog).with_rules(bible());
        run(&mut session, "Y128");
        assert_eq!(destroyed(&session, 128), Some(left(2)));
        assert!(
            session.control_bit(bit(5)),
            "once: a toggle run again clears it"
        );
        assert_eq!(session.take_script_notes(), [reentered(128)]);
        assert!(session.destroying.is_empty(), "done");
        run(&mut session, "U128 Y128");
        assert!(!session.control_bit(bit(5)), "run again once it had ended");
    }

    #[test]
    fn by_the_bible_two_on_destroys_destroying_each_other_run_once_each() {
        let catalog = catalog().living([
            destroying("^b5 U129 Y129"),
            StellarLife {
                on_destroy: Script::parse("^b6 U128 Y128"),
                ..life(129, 3)
            },
        ]);
        let mut session = quiet(&catalog).with_rules(bible());
        run(&mut session, "Y128");
        assert_eq!(destroyed(&session, 128), Some(left(2)));
        assert_eq!(destroyed(&session, 129), Some(left(3)));
        assert!(session.control_bit(bit(5)) && session.control_bit(bit(6)));
        assert_eq!(session.take_script_notes(), [reentered(128)]);
    }

    #[test]
    fn by_the_bible_a_shot_downs_on_destroy_destroying_its_stellar_again_runs_once() {
        let catalog = shooting([destroying("^b5 U128 Y128")]);
        let mut session = quiet(&catalog).with_rules(bible());
        fire(&mut session, 3);
        assert_eq!(destroyed(&session, 128), Some(left(2)));
        assert!(session.control_bit(bit(5)));
        assert_eq!(session.take_script_notes(), [reentered(128)]);
        assert!(session.destroying.is_empty());
    }

    // Rebuild points.

    /// Whether each of `ids` is dead since the last rebuild point.
    fn rebuilt(session: &Session, ids: &[i16]) -> Vec<bool> {
        ids.iter()
            .map(|&id| session.dead_since_rebuild(StellarId(id)))
            .collect()
    }

    /// Runs `text` and settles any move it queued.
    fn settle(session: &mut Session, catalog: &FakePilotCatalog, text: &str) {
        run(session, text);
        let _ = session.settle_script(catalog, &mut Scripted::default());
    }

    #[test]
    fn a_stellar_destroyed_by_y_is_not_dead_since_a_rebuild_until_one() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y129");
        assert_eq!(rebuilt(&session, &[128, 129]), [false, false]);
        assert!(
            !session
                .take_events()
                .contains(&SessionEvent::StellarsRebuilt)
        );
    }

    #[test]
    fn a_move_in_flight_is_a_rebuild_point_even_into_the_same_system() {
        for op in ["M130", "N130"] {
            let catalog = wrecking();
            let mut session = quiet(&catalog);
            run(&mut session, "Y129");
            settle(&mut session, &catalog, op);
            assert_eq!(session.system(), SystemId(130), "{op}");
            assert_eq!(rebuilt(&session, &[128, 129]), [false, true], "{op}");
        }
    }

    #[test]
    fn a_move_in_flight_into_another_system_rebuilds_its_stellars() {
        for op in ["M131", "N131"] {
            let catalog = wrecking();
            let mut session = quiet(&catalog);
            run(&mut session, "Y129 Y140");
            settle(&mut session, &catalog, op);
            assert_eq!(rebuilt(&session, &[129, 140]), [false, true], "{op}");
        }
    }

    #[test]
    fn a_hypergate_exit_is_a_rebuild_point() {
        let catalog = crate::session::tests::gated().living([life(140, -1)]);
        let mut session = crate::session::tests::at_gate(&catalog);
        run(&mut session, "Y140");
        assert!(!session.dead_since_rebuild(StellarId(140)), "not shown");
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
            .expect("enters");
        assert!(session.dead_since_rebuild(StellarId(140)));
    }

    #[test]
    fn a_wormhole_exit_is_a_rebuild_point() {
        let catalog = crate::session::tests::wormholes().living([life(140, -1)]);
        let mut session = crate::session::tests::at_wormhole(&catalog);
        run(&mut session, "Y140");
        session
            .enter_wormhole(&catalog, &mut Scripted::rolling(&[0]))
            .expect("comes out");
        assert_eq!(session.system(), SystemId(131));
        assert!(session.dead_since_rebuild(StellarId(140)));
    }

    #[test]
    fn a_jumps_arrival_is_a_rebuild_point() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y140");
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        assert!(session.dead_since_rebuild(StellarId(140)));
    }

    #[test]
    fn a_landing_and_the_take_off_are_rebuild_points() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y129");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(rebuilt(&session, &[128, 129]), [false, true], "landed");
        run(&mut session, "Y128 U129");
        assert_eq!(rebuilt(&session, &[128, 129]), [false, true], "no frame");
        session
            .take_off(&mut Scripted::default())
            .expect("takes off");
        assert_eq!(rebuilt(&session, &[128, 129]), [true, false], "taken off");
    }

    #[test]
    fn a_relocation_is_a_rebuild_point() {
        let catalog = ruins();
        let mut session = quiet(&catalog);
        assert_eq!(
            session.select_stellar(StellarPick::Slot(1)),
            Some(StellarId(129))
        );
        session.player.position = Vec2::new(2000.0, 0.0);
        assert_eq!(land_now(&mut session), Ok(StellarId(129)));
        run(&mut session, "Y140");
        assert_eq!(
            session.relocate(&catalog, SystemId(131), StellarId(140)),
            Ok(())
        );
        assert!(session.dead_since_rebuild(StellarId(140)));
    }

    #[test]
    fn a_landed_move_is_no_rebuild_point_and_its_stellars_wait_for_the_take_off() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        run(&mut session, "Y129 Y140");
        settle(&mut session, &catalog, "M131");
        assert_eq!(session.landed(), Some(StellarId(128)), "still docked");
        assert_eq!(rebuilt(&session, &[129, 140]), [false, false]);
        session
            .take_off(&mut Scripted::default())
            .expect("takes off");
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(rebuilt(&session, &[129, 140]), [false, true]);
    }

    #[test]
    fn a_new_pilot_rebuilds_after_its_stellars_that_start_destroyed_and_its_on_start() {
        let catalog = FakePilotCatalog {
            character: Ok(crate::catalog::CharacterStart {
                on_start: Script::parse("Y129"),
                ..catalog().character.expect("a chär")
            }),
            ..catalog().living([
                StellarLife {
                    flags2: 0x0040,
                    ..life(128, 2)
                },
                life(129, 2),
            ])
        };
        let mut session = quiet(&catalog);
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(rebuilt(&session, &[128, 129]), [true, true]);
        assert!(
            session
                .take_events()
                .contains(&SessionEvent::StellarsRebuilt)
        );
    }

    #[test]
    fn a_new_pilot_with_no_char_still_rebuilds() {
        let catalog = FakePilotCatalog {
            character: Err(crate::catalog::StartError::NoCharacter),
            ..catalog().living([StellarLife {
                flags2: 0x0040,
                ..life(128, 2)
            }])
        };
        let mut session = Session::fly(
            &catalog,
            crate::pilot::Pilot::new(&wrecking(), "").expect("a pilot"),
        )
        .expect("flies");
        session.begin(&catalog, &mut Scripted::default());
        assert!(session.dead_since_rebuild(StellarId(128)));
    }

    #[test]
    fn a_pilot_flown_with_a_stellar_destroyed_is_dead_since_the_rebuild_at_once_untold() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        let mut flown = Session::fly(&catalog, session.pilot().clone()).expect("flies");
        assert_eq!(rebuilt(&flown, &[128, 129]), [true, false]);
        assert_eq!(flown.take_events(), []);
    }

    #[test]
    fn a_rebuild_point_tells_stellars_rebuilt_only_when_it_changed_them() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        settle(&mut session, &catalog, "M130");
        assert_eq!(
            session.take_events(),
            [SessionEvent::SystemChanged],
            "nothing destroyed"
        );
        run(&mut session, "Y129");
        session.take_events();
        settle(&mut session, &catalog, "M130");
        assert_eq!(
            session.take_events(),
            [SessionEvent::SystemChanged, SessionEvent::StellarsRebuilt]
        );
        settle(&mut session, &catalog, "M130");
        assert_eq!(
            session.take_events(),
            [SessionEvent::SystemChanged],
            "the same again"
        );
    }

    #[test]
    fn a_flight_frame_drops_a_stellar_seen_whole_and_keeps_one_still_destroyed() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y128 Y129");
        settle(&mut session, &catalog, "M130");
        run(&mut session, "U128");
        assert_eq!(rebuilt(&session, &[128, 129]), [true, true], "no frame yet");
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(rebuilt(&session, &[128, 129]), [false, true]);
        run(&mut session, "Y128");
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(rebuilt(&session, &[128, 129]), [false, true], "not back");
    }

    #[test]
    fn a_landed_frame_drops_nothing() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y129");
        land_now(&mut session).expect("lands");
        run(&mut session, "U129");
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert!(session.dead_since_rebuild(StellarId(129)));
    }

    #[test]
    fn y_u_the_daily_step_and_a_galaxy_change_are_no_rebuild_points() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y129");
        settle(&mut session, &catalog, "M130");
        session.take_events();
        run(&mut session, "Y128 U129");
        assert_eq!(rebuilt(&session, &[128, 129]), [false, true], "Y and U");
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(rebuilt(&session, &[128, 129]), [false, true], "a day");
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_stellar_govt(StellarId(128), Some(None));
        });
        assert_eq!(rebuilt(&session, &[128, 129]), [false, true], "the overlay");
        assert!(
            !session
                .take_events()
                .contains(&SessionEvent::StellarsRebuilt)
        );
    }

    #[test]
    fn a_frame_samples_after_its_shots_so_one_shot_down_in_it_stays_dead_since_the_rebuild() {
        let catalog = shooting([life(128, 2)]);
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        settle(&mut session, &catalog, "M130");
        run(&mut session, "U128");
        session.strength.insert(StellarId(128), 0);
        fire(&mut session, 1);
        assert_eq!(destroyed(&session, 128), Some(left(2)), "shot down");
        assert!(session.dead_since_rebuild(StellarId(128)));
    }

    // The landing distance after a rebuild point.

    use crate::flight::ShipState;
    use crate::landing::{LandOutcome, TypeWidth};
    use crate::session::LandPress;

    /// The flags of a stellar landable only once destroyed, as a planet or
    /// a station.
    fn ruin_flags(station: bool) -> u32 {
        let flags = StellarFlags::CAN_LAND | StellarFlags::ONLY_WHEN_DESTROYED;
        if station {
            flags | StellarFlags::STATION
        } else {
            flags
        }
    }

    /// [`catalog`] with system 130 holding stellar 128 at the centre,
    /// `width` square, of graphic type 10 and `DeadType` `dead_graphic`,
    /// with `flags`, back after 5 days; and planet 129 far away, of
    /// graphic type `other_graphic`.
    fn landing_at(
        width: u32,
        flags: u32,
        dead_graphic: Option<i16>,
        other_graphic: i16,
    ) -> FakePilotCatalog {
        let mut catalog = catalog().living([life(128, 5)]);
        catalog.sites[0].1 = vec![
            LandingSite {
                frame_size: Some((width, width)),
                flags,
                graphic: 10,
                dead_graphic,
                ..crate::testkit::planet(128, 0.0, 0.0)
            },
            LandingSite {
                graphic: other_graphic,
                ..crate::testkit::planet(129, 2000.0, 0.0)
            },
        ];
        catalog
    }

    /// Stellar 128 of `width`, landable only once destroyed, with a
    /// `DeadType` sheet, sharing no graphic.
    fn ruin(width: u32, station: bool) -> FakePilotCatalog {
        landing_at(width, ruin_flags(station), Some(11), 12)
    }

    /// A rebuild point: an in-flight `M` into the same system.
    fn rebuild(session: &mut Session, catalog: &FakePilotCatalog) {
        settle(session, catalog, "M130");
    }

    /// A flight frame.
    fn frame(session: &mut Session) {
        session.tick_combat(Rules::default(), &mut NeverFires);
    }

    /// The land key pressed in a session of `catalog` with `rules`, after
    /// `setup`, with 128 targeted and the ship at rest at `offset` from
    /// its centre: the stellar landed on, or the refusal.
    fn land_at(
        catalog: &FakePilotCatalog,
        rules: &Rulebook,
        setup: &dyn Fn(&mut Session),
        offset: Vec2,
    ) -> Result<StellarId, LandingRefusal> {
        let mut session = quiet(catalog).with_rules(*rules);
        setup(&mut session);
        session.player = ShipState {
            position: offset,
            ..ShipState::default()
        };
        session.nav_target = Some(StellarId(128));
        match session.land()? {
            LandPress::Outcome(LandOutcome::Landed(stellar)) => Ok(stellar),
            other => panic!("not a landing: {other:?}"),
        }
    }

    /// Whether, after `setup`, the ship lands at `inside` on either axis
    /// and is too far at `outside`.
    fn reaches(
        catalog: &FakePilotCatalog,
        rules: &Rulebook,
        setup: &dyn Fn(&mut Session),
        inside: f32,
        outside: f32,
    ) -> bool {
        let at = |offset| land_at(catalog, rules, setup, offset);
        [Vec2::new(inside, 0.0), Vec2::new(0.0, -inside)]
            .into_iter()
            .all(|offset| at(offset) == Ok(StellarId(128)))
            && [Vec2::new(outside, 0.0), Vec2::new(0.0, -outside)]
                .into_iter()
                .all(|offset| matches!(at(offset), Err(LandingRefusal::TooFar { .. })))
    }

    fn engine() -> Rulebook {
        Rulebook::default()
    }

    fn other_reading() -> Rulebook {
        Rulebook::default().with_override(RuleKey::LandingDistance, RuleSource::Bible)
    }

    #[test]
    fn a_destroyed_stellar_lands_within_its_type_width_until_a_rebuild_point_then_150() {
        for station in [false, true] {
            // 100 and 200 wide: narrower and wider than 150.
            for (width, inside, outside) in [(100, 174.0, 175.0), (200, 349.0, 350.0)] {
                let catalog = ruin(width, station);
                let rules = engine();
                let destroyed = |session: &mut Session| run(session, "Y128");
                assert!(
                    reaches(&catalog, &rules, &destroyed, inside, outside),
                    "{width} {station}: before a rebuild point"
                );
                let rebuilt = |session: &mut Session| {
                    run(session, "Y128");
                    frame(session);
                    rebuild(session, &catalog);
                };
                assert!(
                    reaches(&catalog, &rules, &rebuilt, 261.0, 262.0),
                    "{width} {station}: after one"
                );
                let seen_whole = |session: &mut Session| {
                    run(session, "Y128");
                    rebuild(session, &catalog);
                    run(session, "U128");
                    frame(session);
                    run(session, "Y128");
                };
                assert!(
                    reaches(&catalog, &rules, &seen_whole, inside, outside),
                    "{width} {station}: a frame found it whole"
                );
            }
        }
    }

    #[test]
    fn a_destroyed_stellar_sharing_its_type_with_a_whole_one_keeps_its_width() {
        let catalog = landing_at(100, ruin_flags(false), Some(11), 10);
        let rebuilt = |session: &mut Session| {
            run(session, "Y128");
            rebuild(session, &catalog);
        };
        assert!(reaches(&catalog, &engine(), &rebuilt, 174.0, 175.0));
    }

    #[test]
    fn by_the_other_reading_a_destroyed_stellar_always_lands_within_its_type_circle() {
        for (width, radius) in [(100, 50.0), (200, 100.0)] {
            let catalog = ruin(width, false);
            let rules = other_reading();
            let destroyed = |session: &mut Session| run(session, "Y128");
            let rebuilt = |session: &mut Session| {
                run(session, "Y128");
                rebuild(session, &catalog);
            };
            for setup in [&destroyed as &dyn Fn(&mut Session), &rebuilt] {
                assert!(reaches(&catalog, &rules, setup, radius, radius + 0.5));
                assert!(
                    matches!(
                        land_at(
                            &catalog,
                            &rules,
                            setup,
                            Vec2::new(radius * 0.8, radius * 0.8)
                        ),
                        Err(LandingRefusal::TooFar { .. })
                    ),
                    "a circle, not a box"
                );
            }
        }
    }

    /// `Y128`, then a rebuild point, reading `catalog`.
    fn destroyed_and_rebuilt(catalog: &FakePilotCatalog) -> impl Fn(&mut Session) + '_ {
        move |session: &mut Session| {
            run(session, "Y128");
            rebuild(session, catalog);
        }
    }

    #[test]
    fn a_slot_left_with_no_frame_reaches_75() {
        let mut catalog = landing_at(100, ruin_flags(false), None, 12);
        catalog.fallback_frame = None;
        let rebuilt = destroyed_and_rebuilt(&catalog);
        assert!(reaches(&catalog, &engine(), &rebuilt, 74.0, 75.0));
        assert!(
            reaches(&catalog, &other_reading(), &rebuilt, 50.0, 50.5),
            "the other reading's circle"
        );
        // With graphic type 0 to fill the slot, it shows a frame, and the
        // freed `Type` reads 150.
        let filled = landing_at(100, ruin_flags(false), None, 12);
        let rebuilt = destroyed_and_rebuilt(&filled);
        assert!(reaches(&filled, &engine(), &rebuilt, 261.0, 262.0));
    }

    #[test]
    fn a_loaded_type_whose_spin_counts_no_frames_reaches_56() {
        let mut catalog = ruin(100, false);
        catalog.sites[0].1[0].flags = StellarFlags::CAN_LAND;
        catalog.sites[0].1[0].type_counts_frames = false;
        let whole = |_: &mut Session| {};
        assert!(reaches(&catalog, &engine(), &whole, 55.0, 56.0));
        assert!(reaches(&catalog, &other_reading(), &whole, 50.0, 50.5));
    }

    #[test]
    fn a_session_flown_has_rebuilt_its_masters_at_once() {
        let catalog = ruin(100, false);
        let session = quiet(&catalog);
        let site = session.sites[0];
        assert_eq!(session.rebuilt.type_width(&site), TypeWidth::Frame);
        assert_eq!(
            session.landing_reach(&site),
            crate::landing::LandingReach::Box(175.0)
        );
    }

    // The click after a destruction.

    /// Stellar 128 at the centre, 40 square, with a `DeadType` sheet of
    /// `dead_frame_size` when that is `Some`, and graphic type 0's sheet
    /// of `fallback_frame`.
    fn clicking(
        dead_frame_size: Option<(u32, u32)>,
        fallback_frame: Option<(u32, u32)>,
    ) -> FakePilotCatalog {
        let mut catalog = landing_at(40, StellarFlags::CAN_LAND, dead_frame_size.map(|_| 11), 12);
        catalog.sites[0].1[0].dead_frame_size = dead_frame_size;
        catalog.fallback_frame = fallback_frame;
        catalog
    }

    /// The stellar a click at (`x`, `y`) picks.
    fn click(session: &mut Session, x: f32, y: f32) -> Option<StellarId> {
        session.select_stellar(StellarPick::At(Vec2::new(x, y)))
    }

    #[test]
    fn a_destroyed_stellar_is_clicked_by_its_dead_type_sheet() {
        let catalog = clicking(Some((200, 200)), Some((100, 100)));
        let mut session = quiet(&catalog);
        // Whole, its `Type` box is 20 + 16 a side.
        assert_eq!(click(&mut session, 90.0, 0.0), None, "whole");
        assert_eq!(click(&mut session, 30.0, 0.0), Some(StellarId(128)));
        run(&mut session, "Y128");
        // Its `DeadType` box is 100 a side, with no margin.
        assert_eq!(click(&mut session, 90.0, 0.0), Some(StellarId(128)));
        assert_eq!(click(&mut session, 0.0, -90.0), Some(StellarId(128)));
        assert_eq!(click(&mut session, 110.0, 0.0), None);
        rebuild(&mut session, &catalog);
        assert_eq!(
            click(&mut session, 90.0, 0.0),
            Some(StellarId(128)),
            "its own sheet, not graphic type 0's, after a rebuild point"
        );
        run(&mut session, "U128");
        assert_eq!(click(&mut session, 90.0, 0.0), None, "regenerated");
        assert_eq!(click(&mut session, 30.0, 0.0), Some(StellarId(128)));
    }

    #[test]
    fn a_destroyed_stellar_with_no_dead_sheet_is_clicked_by_graphic_type_0_after_a_rebuild() {
        let catalog = clicking(None, Some((200, 200)));
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        assert_eq!(
            click(&mut session, 90.0, 0.0),
            None,
            "its `Type` until a rebuild point"
        );
        assert_eq!(click(&mut session, 34.0, 0.0), Some(StellarId(128)));
        rebuild(&mut session, &catalog);
        assert_eq!(click(&mut session, 90.0, 0.0), Some(StellarId(128)));
        assert_eq!(click(&mut session, 110.0, 0.0), None);
        run(&mut session, "U128");
        assert_eq!(
            click(&mut session, 90.0, 0.0),
            None,
            "whole, though no frame has seen it yet"
        );
    }

    #[test]
    fn by_the_bible_a_destroyed_stellar_with_no_dead_sheet_is_clicked_by_its_type() {
        let catalog = clicking(None, Some((200, 200)));
        let mut session = quiet(&catalog).with_rules(
            Rulebook::default().with_override(RuleKey::DeadTypeFallback, RuleSource::Bible),
        );
        run(&mut session, "Y128");
        rebuild(&mut session, &catalog);
        assert_eq!(click(&mut session, 90.0, 0.0), None);
        assert_eq!(click(&mut session, 34.0, 0.0), Some(StellarId(128)));
    }

    // The sheet drawn (story-runtime phase 12's one owner).

    /// The sheet stellar 128 draws.
    fn sheet_128(session: &Session) -> DrawnSheet {
        session.drawn_sheet(StellarId(128))
    }

    #[test]
    fn a_stellar_with_a_dead_type_sheet_draws_it_while_destroyed() {
        let catalog = clicking(Some((200, 200)), Some((100, 100)));
        let mut session = quiet(&catalog);
        assert_eq!(sheet_128(&session), DrawnSheet::Type, "whole");
        run(&mut session, "Y128");
        assert_eq!(sheet_128(&session), DrawnSheet::Dead);
        rebuild(&mut session, &catalog);
        assert_eq!(sheet_128(&session), DrawnSheet::Dead, "a rebuild point");
        run(&mut session, "U128");
        assert_eq!(sheet_128(&session), DrawnSheet::Type, "regenerated");
    }

    #[test]
    fn by_the_engine_a_stellar_with_no_dead_type_sheet_draws_type_zero_while_dead_since_a_rebuild()
    {
        for fallback in [Some((200, 200)), None] {
            let catalog = clicking(None, fallback);
            let mut session = quiet(&catalog);
            run(&mut session, "Y128");
            assert_eq!(sheet_128(&session), DrawnSheet::Type, "{fallback:?}");
            rebuild(&mut session, &catalog);
            assert_eq!(sheet_128(&session), DrawnSheet::TypeZero, "{fallback:?}");
            frame(&mut session);
            assert_eq!(sheet_128(&session), DrawnSheet::TypeZero, "{fallback:?}");
            // `U`, then `Y` before a frame sees it whole.
            run(&mut session, "U128");
            assert_eq!(sheet_128(&session), DrawnSheet::Type, "{fallback:?}");
            run(&mut session, "Y128");
            assert_eq!(sheet_128(&session), DrawnSheet::TypeZero, "{fallback:?}");
            // `U`, a frame while whole, then `Y`.
            run(&mut session, "U128");
            frame(&mut session);
            run(&mut session, "Y128");
            assert_eq!(sheet_128(&session), DrawnSheet::Type, "{fallback:?}");
        }
    }

    #[test]
    fn by_the_bible_a_stellar_with_no_dead_type_sheet_draws_its_type() {
        let catalog = clicking(None, Some((200, 200)));
        let mut session = quiet(&catalog).with_rules(
            Rulebook::default().with_override(RuleKey::DeadTypeFallback, RuleSource::Bible),
        );
        run(&mut session, "Y128");
        rebuild(&mut session, &catalog);
        assert_eq!(sheet_128(&session), DrawnSheet::Type);
        // The Bible's reading changes nothing for a `DeadType` sheet.
        let catalog = clicking(Some((200, 200)), Some((100, 100)));
        let mut session = quiet(&catalog).with_rules(
            Rulebook::default().with_override(RuleKey::DeadTypeFallback, RuleSource::Bible),
        );
        run(&mut session, "Y128");
        rebuild(&mut session, &catalog);
        assert_eq!(sheet_128(&session), DrawnSheet::Dead);
    }

    #[test]
    fn a_stellar_not_shown_draws_its_type() {
        let catalog = clicking(Some((200, 200)), Some((100, 100))).living([life(140, 5)]);
        let mut session = quiet(&catalog);
        run(&mut session, "Y140");
        rebuild(&mut session, &catalog);
        assert!(session.galaxy().destroyed(StellarId(140)));
        assert_eq!(session.drawn_sheet(StellarId(140)), DrawnSheet::Type);
    }

    #[test]
    fn a_slot_left_with_no_frame_is_clicked_on_the_64_pixel_placeholder() {
        let catalog = clicking(None, None);
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        rebuild(&mut session, &catalog);
        // The placeholder is 32 a side; the `Type` box would be 36.
        assert_eq!(click(&mut session, 31.0, 0.0), Some(StellarId(128)));
        assert_eq!(click(&mut session, 34.0, 0.0), None);
        assert_eq!(click(&mut session, 90.0, 0.0), None);
    }
}
