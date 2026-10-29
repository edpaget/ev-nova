//! The stellars destroyed and regenerated on the session: `Y` and `U`,
//! handed to `Session::apply_op`, each read from the stellar's
//! [`StellarLife`](crate::StellarLife) the session read when it started.
//! Each writes the galaxy overlay through the writer's catalog-free entry,
//! `Chart::set_stellar_state`, so every reader sees it at once (see the
//! [`story`](crate::story) module's "Galaxy overlay").
//!
//! - `Yxxx` destroys the stellar with its countdown, its `DeadTime` or 1
//!   when that is negative, whether it was destroyed or not, so a second
//!   `Y` restarts the countdown (@0x15ac8-0x15ade). By
//!   [`RuleKey::DestroyInvincible`]'s engine reading an invincible stellar
//!   (`Strength` 0 or -1) is left as it was. A stellar with no record, or
//!   outside the original's slots, 128 to 2175, is left alone. When the
//!   stellar is the player's system's and was not destroyed before, it
//!   makes the stellar's explosion at its own position, invincible or
//!   not, heard at once, docked or not, and drawn only in flight
//!   (@0x15a59-0x15ab4; see the `y_explosion` module). By [`RuleKey::YOnDestroy`]'s engine reading it
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
//! **Shots.** Each pass's fight (see the `game_work` module) hands the
//! combat the stellars shown that shots can hit (whole, of `Strength`
//! above 0, drawing a sheet that can be read, as big as the disc of its
//! frame, see [`Session::drawn_sheet`]), each with its current
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
//! (`Session::dead_since_rebuild`; see the `story` module's "Built by
//! phase 9" and `story::rebuild`), with the stellar masters loaded and
//! the sprite slots left with no frame, which the player's landing
//! distance reads ([`Session::land`], by
//! [`RuleKey::LandingDistance`]; "Built by phase 10"), and the sheet whose
//! frame count each stellar shown animates over
//! ([`Session::counted_sheet`], by [`RuleKey::StellarFrameCount`];
//! "Built by stellar-fidelity phase 1"), and the sheet each slot holds,
//! which a stellar showing no sheet that can be read draws
//! ([`Session::drawn_sheet`], by [`RuleKey::DeadTypeFallback`] and
//! [`RuleKey::TypeSheetFallback`]; "Built by stellar-fidelity phase 3"),
//! and, for a shown graphic whose `spïn` counts no frames, the slot left
//! with no frame or the frame it kept, frozen, and the stellars that count
//! none ([`Session::drawn_sheet`], [`Session::frame_frozen`],
//! [`Session::counts_no_frames`], by [`RuleKey::UncountedSheet`]; "Built
//! by stellar-fidelity phase 8").
//! Each point rebuilds after the transition's own writes, and tells
//! [`SessionEvent::StellarsRebuilt`](super::SessionEvent::StellarsRebuilt)
//! when the set, a counted sheet, a held sheet, a slot's frame or a count
//! changed:
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
//! Each rebuild point first rewrites the course, docked or not: by the
//! engine reading of
//! [`RuleKey::RouteRewrite`](crate::RuleKey::RouteRewrite), each hidden
//! hop becomes the system at its place and the course is cut at the first
//! hop with none (@0x32ab3-0x32b0d; see the `course` module). That rewrite
//! raises no event and tells nothing.
//!
//! It then shares exploration by place, docked or not: by
//! the engine reading of
//! [`RuleKey::ExploredByPlace`](crate::RuleKey::ExploredByPlace), every
//! system at an explored head's place is explored, hidden or not
//! (@0x32b13-0x32b8f; see the `explored` module), so a system the bits
//! swap in is shown explored from the next rebuild point. That share
//! raises no event and tells nothing.
//!
//! It then rescues, docked or not: by the engine reading of
//! [`RuleKey::RescueUntilRefresh`], each stellar the player's system
//! lists that no system owns and no system has rescued is kept as the
//! player's system's until the next refresh (@0x32c58-0x32ca1; see the
//! `kept_rescues` module), so it stays that system's across a gate's
//! exit or an `M`.
//!
//! In flight, each rebuild point then derives which stellars are shown
//! afresh, as `_HandleStellarSystemVisibility` sets them active and owned
//! (see the `entry` module), and tells `StellarsRebuilt` when that
//! changed them too. Docked, it derives nothing: the take-off's point does.
//!
//! A landed `M` or `N`, `Y`, `U`, the daily step, the shots and the galaxy
//! overlay's changes are not rebuild points. The original's escape-pod
//! rescue rebuilds too (@0x69135): the engine has none, and whoever builds
//! it calls the rebuild. Its debug key that explores every system
//! (@0x6dad1) has no counterpart. The flight frame that drops a stellar
//! seen whole is the end of the pass's fight in flight
//! ([`Session::fight_tail`]), after the shots, as the original's sprite
//! tick (@0x444d2) follows `_CheckShotProximities` (@0x44015).

use super::{Chart, Outbox, Session, Setup, Space, Statics};
use crate::catalog::{LandingSite, StellarId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::stellar::StellarTarget;
use crate::control::{Script, ScriptNote};
use crate::gate::GateKind;
use crate::geometry::Vec2;
use crate::landing::{LandingReach, StellarShown, TypeWidth, frame_radius, landing_reach};
use crate::legal::{Crime, LegalCode};
use crate::navigation::SpriteRect;
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource};
use crate::story::rebuild::Fallback;
use crate::story::sprite_pool;
use crate::story::wreck::{self, Tick};
use crate::story::{Countdown, DrawnSheet};
use std::collections::BTreeMap;

/// How many killings the player is convicted of for shooting a stellar
/// down: `_SlapWithPenalty` runs ten times (@0x374e1-0x3751f).
const STELLAR_KILLINGS: usize = 10;

/// The side `_GetSpriteXSize` and `_GetSpriteYSize` give a sprite slot
/// with no frame, or whose frame index is not below its count
/// (@0x26e5-0x26ef, @0x2714-0x271e).
pub(super) const NO_SPRITE_SIDE: u32 = 32;

impl Session {
    /// Whether `stellar` has been dead since the last rebuild point: it
    /// is among the stellars shown, the last rebuild point found it
    /// destroyed, and no flight frame has found it whole since (see the
    /// module docs). A stellar with no `DeadType` sheet draws graphic type
    /// 0 while this holds, by
    /// [`RuleKey::DeadTypeFallback`]'s engine reading.
    ///
    /// For the tests: no production code calls it.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn dead_since_rebuild(&self, stellar: StellarId) -> bool {
        self.space.dead_since_rebuild(stellar)
    }

    /// A rebuild point (see the module docs): in flight the stellars shown
    /// are derived afresh, those destroyed now are the ones dead since it,
    /// and each counts the frames of the sheet its slot now holds, telling
    /// [`SessionEvent::StellarsRebuilt`](super::SessionEvent::StellarsRebuilt)
    /// when that changed either.
    pub(super) fn rebuild_stellars(&mut self) {
        if self.rebuild_quietly() {
            self.out.emit(super::SessionEvent::StellarsRebuilt);
        }
    }

    /// A rebuild point that tells nothing: first, docked or not, the
    /// course rewritten (`Chart::rewrite_course`, as
    /// `_HandleStellarSystemVisibility` rewrites it first @0x32ab3-0x32b0d),
    /// the exploration shared by place (`Chart::share_explored`, as
    /// it then raises each system to its head's level @0x32b13-0x32b8f)
    /// the known set worked out afresh (`Chart::rework_known`,
    /// @0x32b7b, @0x32b91-0x32c10) and the rescue kept
    /// (`Chart::keep_rescues`, @0x32c58-0x32ca1), and every stellar's
    /// under-attack mark cleared (@0x32d5d), before its stellars;
    /// then, in flight, the
    /// stellars shown derived afresh (`Session::reshow`), then which of
    /// them are dead since it and the sheet each counts the frames of,
    /// then the sprite pool handed out (see the `stale_sprites` module).
    /// Whether it changed the stellars or a counted sheet.
    pub(super) fn rebuild_quietly(&mut self) -> bool {
        self.chart
            .rebuild_chart(&mut self.pilot, &self.statics, &self.setup);
        // Every stellar's +0x47 cleared (@0x32d5d; see the
        // `stellar_defence` module).
        self.space.traffic.defences_mut().clear();
        let reshown = self.reshow();
        let galaxy = &self.chart.galaxy;
        let rebuilt = self.space.rebuilt.rebuild(
            &self.chart.sites,
            |id| galaxy.destroyed(id),
            Fallback::new(
                self.statics.fallback_frame.is_some(),
                self.statics.fallback_counts,
            ),
            self.setup.rules.source_for(RuleKey::UncountedSheet),
        );
        self.rebuild_sprite_pool();
        reshown | rebuilt
    }

    /// Which sheet `stellar` draws (see the module docs): its `DeadType`
    /// sheet while destroyed and its `Type` sheet while whole, when that
    /// can be read. A stellar showing no sheet that can be read draws what
    /// its sprite slot holds, by the engine's reading of
    /// [`RuleKey::DeadTypeFallback`] while destroyed and of
    /// [`RuleKey::TypeSheetFallback`] while whole: graphic type 0's from
    /// the rebuild point that filled its slot (for a destroyed stellar
    /// with a `Type` sheet, exactly while it is dead since the last
    /// rebuild point), a `DeadType` sheet a flight frame cloned in since,
    /// or graphic type 0's whenever its slot shows no frame and that sheet
    /// cannot be read; its `Type` sheet by the other reading. By
    /// [`RuleKey::UncountedSheet`]'s engine reading, a stellar whose shown
    /// sheet reads but whose `spïn` counts no frames draws the frame its
    /// slot shows, and [`DrawnSheet::NoFrame`] when it shows none, as does
    /// one whose slot graphic type 0 filled with frames that count none.
    /// Its `Type` sheet for a stellar that is not among those shown. The
    /// click box ([`Session::select_stellar`]), the shots and the views
    /// read this one answer.
    #[must_use]
    pub(crate) fn drawn_sheet(&self, stellar: StellarId) -> DrawnSheet {
        self.chart
            .sites
            .iter()
            .find(|site| site.id == stellar)
            .map_or(DrawnSheet::Type, |site| {
                self.space.site_sheet(&self.statics, &self.setup, site)
            })
    }

    /// Whether `stellar`'s sprite slot shows a frame frozen, by
    /// [`RuleKey::UncountedSheet`]'s engine reading: a flight frame cloned
    /// into it a sheet whose `spïn` counts no frames, so it keeps the
    /// frame it showed ([`Session::drawn_sheet`]), never stepping it, and
    /// is placed as a 32-pixel sprite, its top left at the stellar's
    /// centre less 16 on each axis (see the `story` module's "Built by
    /// stellar-fidelity phase 8").
    #[must_use]
    pub(crate) fn frame_frozen(&self, stellar: StellarId) -> bool {
        self.space.frame_frozen(&self.setup, stellar)
    }

    /// Whether `stellar` animates over no frames until the next rebuild
    /// point, by the engine readings of [`RuleKey::UncountedSheet`] and
    /// [`RuleKey::StellarFrameCount`]: the last rebuild point copied a
    /// count of none from its slot (@0x331c6-0x331c9), so it never
    /// animates (`_HandleStellarSprites` @0x2e802), even when a later
    /// clone brings counting frames into the slot.
    #[must_use]
    pub(crate) fn counts_no_frames(&self, stellar: StellarId) -> bool {
        self.space.counts_no_frames(&self.setup, stellar)
    }

    /// The sheet whose frame count `stellar` animates over, by
    /// [`RuleKey::StellarFrameCount`] (see the module docs): by the
    /// engine, the one its sprite slot held at the last rebuild point
    /// (graphic type 0's read as its `Type` when the rules never let it
    /// draw graphic type 0: by [`RuleKey::DeadTypeFallback`]'s Bible
    /// reading or with a `DeadType` sheet, and by
    /// [`RuleKey::TypeSheetFallback`]'s other reading or with a `Type`
    /// sheet); by the Bible, the sheet it draws
    /// ([`Session::drawn_sheet`]). Its `Type` for a stellar not shown.
    #[must_use]
    pub(crate) fn counted_sheet(&self, stellar: StellarId) -> DrawnSheet {
        if self.setup.rules.source_for(RuleKey::StellarFrameCount) == RuleSource::Bible {
            return self.drawn_sheet(stellar);
        }
        self.chart
            .sites
            .iter()
            .find(|site| site.id == stellar)
            .map_or(DrawnSheet::Type, |site| {
                self.space.counted_sheet(&self.setup, site)
            })
    }

    /// A flight frame's sprite tick (see the module docs): every stellar
    /// it finds whole is no longer dead since the last rebuild point, and
    /// each owned sprite slot is placed (see the `stale_sprites` module).
    pub(super) fn see_stellars(&mut self) {
        let galaxy = &self.chart.galaxy;
        self.space.rebuilt.frame(
            &self.chart.sites,
            |id| galaxy.destroyed(id),
            self.setup.rules.source_for(RuleKey::UncountedSheet),
        );
        self.place_stellar_sprites();
    }

    /// The width each hypergate and wormhole shown takes its entry box
    /// by: the one `_GetStellarSpriteXSize` reads of its `Type` master
    /// (see [`gate_entry`](crate::traffic::gate_entry)). A gate the
    /// player's system does not hold ([`Chart::held_here`]) reads the
    /// unloaded width, 150, as the owner test comes first (@0xacb9-0xacc1).
    pub(super) fn gate_type_widths(&self) -> BTreeMap<StellarId, TypeWidth> {
        self.chart
            .sites
            .iter()
            .filter(|site| GateKind::of(site.flags2).is_some())
            .map(|site| {
                let held = self.chart.held_here(&self.pilot, &self.statics, site.id);
                (site.id, self.space.type_width(site, held))
            })
            .collect()
    }

    /// `Y`: destroys `stellar` (see the module docs), drawing on `chance`
    /// for an `OnDestroy` the Bible's reading runs.
    pub(super) fn destroy_stellar(&mut self, stellar: StellarId, chance: &mut dyn Chance) {
        if !wreck::in_range(stellar) {
            return;
        }
        let Some(life) = self.statics.lives.get(&stellar).cloned() else {
            return;
        };
        let destroyed_before = self.chart.galaxy.destroyed(stellar);
        // The explosion, heard at once, before the countdown
        // (@0x15a59-0x15ab4; see the `y_explosion` module).
        self.explode_on_y(stellar, &life, destroyed_before);
        self.space.strength.remove(&stellar);
        if wreck::destroys(
            &life,
            self.setup.rules.source_for(RuleKey::DestroyInvincible),
        ) {
            let countdown = wreck::countdown_on_destroy(&life);
            self.chart
                .set_stellar_state(&mut self.pilot, &mut self.out, stellar, Some(countdown));
            if !destroyed_before
                && self.setup.rules.source_for(RuleKey::YOnDestroy) == RuleSource::Bible
            {
                self.run_on_destroy(stellar, &life.on_destroy, chance);
            }
        }
    }

    /// `U`: regenerates `stellar` (see the module docs). Only `Y` and a
    /// new pilot's stellars destroy one, each within the original's
    /// slots, so any other is never destroyed.
    pub(super) fn regenerate_stellar(&mut self, stellar: StellarId) {
        self.space.strength.remove(&stellar);
        self.chart
            .set_stellar_state(&mut self.pilot, &mut self.out, stellar, None);
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
        if !self.space.destroying.insert(stellar) {
            self.out
                .script_notes
                .push(ScriptNote::OnDestroyReentered { stellar });
            return;
        }
        self.run_script(script, chance);
        self.space.destroying.remove(&stellar);
    }

    /// The stellars shown that shots can hit this tick: each whole one of
    /// `Strength` above 0 drawing a sheet that can be read, as big as the
    /// disc of that sheet's frame ([`frame_radius`], which for a `Type`
    /// sheet is [`landing_radius`](crate::landing::landing_radius)), with
    /// its current strength. A whole
    /// stellar with no `Type` sheet is hit by the frame its sprite slot
    /// holds (`_CheckShotProximities` @0x37288-0x372a8 collides with the
    /// slot's sprite), by [`RuleKey::TypeSheetFallback`]'s engine reading.
    pub(super) fn stellar_targets(&self) -> Vec<StellarTarget> {
        self.chart
            .sites
            .iter()
            .filter(|site| !site.destroyed)
            .filter_map(|site| {
                let frame = self
                    .space
                    .drawn_frame_size(&self.statics, &self.setup, site)?;
                let life = self.statics.lives.get(&site.id)?;
                wreck::destroyable(life).then(|| StellarTarget {
                    stellar: site.id,
                    position: site.position,
                    radius: frame_radius(frame),
                    strength: self
                        .space
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
            let full = self
                .statics
                .lives
                .get(&target.stellar)
                .map(wreck::full_strength);
            if full == Some(target.strength) {
                self.space.strength.remove(&target.stellar);
            } else {
                self.space.strength.insert(target.stellar, target.strength);
            }
        }
        for strike in self.space.combat.take_stellar_strikes() {
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
    /// convicted, and only by a shot whose source is its ship (`by` none
    /// for one turned on its firer, @0x374e1-0x374e7), ten times of killing a ship of its government, as `law`
    /// says, and loses the ranks that killing reaches (see
    /// [`rank`](crate::rank)'s **Crimes**).
    pub(super) fn shoot_down(
        &mut self,
        stellar: StellarId,
        by: Option<ShipRef>,
        law: &dyn LegalCode,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let (Some(life), Some(site)) = (
            self.statics.lives.get(&stellar).cloned(),
            self.chart
                .sites
                .iter()
                .find(|site| site.id == stellar)
                .copied(),
        ) else {
            return;
        };
        if let Some(explosion) = life.explosion {
            self.space.combat.explode(site.position, explosion);
        }
        let countdown = wreck::countdown_on_destroy(&life);
        self.chart
            .set_stellar_state(&mut self.pilot, &mut self.out, stellar, Some(countdown));
        self.run_on_destroy(stellar, &life.on_destroy, chance);
        self.chart
            .set_stellar_state(&mut self.pilot, &mut self.out, stellar, Some(countdown));
        self.space.strength.remove(&stellar);
        if by == Some(ShipRef::Player) {
            for _ in 0..STELLAR_KILLINGS {
                self.chart
                    .convict_of(&mut self.pilot, &self.statics, law, Crime::Kill, site.govt);
            }
            // `_CheckShotProximities` slaps ten times (@0x374e9), but
            // after the first pass no rank it takes is still active, so
            // one pass takes the same ranks.
            self.run.lose_ranks_to(
                &mut self.pilot,
                &self.statics,
                &self.setup,
                Crime::Kill,
                site.govt,
            );
        }
    }

    /// The daily step's stellars (see the module docs): every damaged
    /// stellar's strength restored, every active stellar's gun timer
    /// zeroed (see the `stellar_guns` module), every active dominated
    /// stellar's defence fleet regrown (see the `dominated_stellars`
    /// module), then each destroyed
    /// stellar in turn, by ascending ID, counts a day down, or regenerates
    /// and runs its `OnRegen` at once, through [`Session::run_set`],
    /// drawing on `chance`. The overlay is read again after each, so a
    /// stellar an `OnRegen` destroys is counted the same day when its ID is
    /// higher, as the original's pass by index does. A destroyed stellar
    /// with no record is left as it is.
    pub(super) fn stellars_day(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.space.strength.clear();
        self.reset_gun_timers();
        self.chart
            .regrow_defences(&mut self.pilot, &self.statics, chance);
        let mut after = None;
        while let Some((stellar, countdown)) = self.next_destroyed(after) {
            after = Some(stellar);
            let Some(life) = self.statics.lives.get(&stellar).cloned() else {
                continue;
            };
            match wreck::day(&life, countdown) {
                Tick::Stays(countdown) => {
                    self.chart.set_stellar_state(
                        &mut self.pilot,
                        &mut self.out,
                        stellar,
                        Some(countdown),
                    );
                }
                Tick::Regenerates => {
                    self.chart
                        .set_stellar_state(&mut self.pilot, &mut self.out, stellar, None);
                    self.run_script(&life.on_regen, chance);
                }
            }
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

impl Space {
    /// Whether `stellar` has been dead since the last rebuild point: it
    /// is among the stellars shown, the last rebuild point found it
    /// destroyed, and no flight frame has found it whole since (see the
    /// module docs). A stellar with no `DeadType` sheet draws graphic type
    /// 0 while this holds, by
    /// [`RuleKey::DeadTypeFallback`]'s engine reading.
    ///
    /// For the tests: no production code calls it.
    #[cfg(test)]
    #[must_use]
    fn dead_since_rebuild(&self, stellar: StellarId) -> bool {
        self.rebuilt.dead(stellar)
    }

    /// [`Session::drawn_sheet`] for `site`, one of the stellars shown: its
    /// `DeadType` sheet while destroyed and its `Type` sheet while whole,
    /// each when it can be read and, by [`RuleKey::UncountedSheet`]'s
    /// engine reading, counts frames. One that reads but counts none
    /// draws the frame its sprite slot shows (`Rebuilt::showing`), or
    /// [`DrawnSheet::NoFrame`] when it shows none. A stellar showing no
    /// sheet that can be read draws, by the engine's reading of the key
    /// that governs it ([`RuleKey::DeadTypeFallback`] while destroyed,
    /// [`RuleKey::TypeSheetFallback`] while whole), the frame its slot
    /// shows; when it shows none, graphic type 0 when that cannot be read,
    /// the view then drawing the placeholder with that sheet's reason,
    /// and otherwise no frame; and its `Type` by the other reading.
    fn site_sheet(&self, statics: &Statics, setup: &Setup, site: &LandingSite) -> DrawnSheet {
        let honours_counts = setup.rules.source_for(RuleKey::UncountedSheet) == RuleSource::Engine;
        let showing = self.rebuilt.showing(site.id);
        let readable = if site.destroyed {
            site.dead_frame_size
                .map(|_| (DrawnSheet::Dead, site.dead_counts_frames))
        } else {
            site.frame_size
                .map(|_| (DrawnSheet::Type, site.type_counts_frames))
        };
        if let Some((sheet, counts)) = readable {
            return if counts || !honours_counts {
                sheet
            } else {
                showing.unwrap_or(DrawnSheet::NoFrame)
            };
        }
        let key = if site.destroyed {
            RuleKey::DeadTypeFallback
        } else {
            RuleKey::TypeSheetFallback
        };
        if setup.rules.source_for(key) == RuleSource::Bible {
            DrawnSheet::Type
        } else if let Some(showing) = showing {
            showing
        } else if statics.fallback_frame.is_none() {
            DrawnSheet::TypeZero
        } else {
            DrawnSheet::NoFrame
        }
    }

    /// [`Session::frame_frozen`].
    fn frame_frozen(&self, setup: &Setup, stellar: StellarId) -> bool {
        setup.rules.source_for(RuleKey::UncountedSheet) == RuleSource::Engine
            && self.rebuilt.frozen(stellar)
    }

    /// [`Session::counts_no_frames`].
    fn counts_no_frames(&self, setup: &Setup, stellar: StellarId) -> bool {
        let engine = |key| setup.rules.source_for(key) == RuleSource::Engine;
        engine(RuleKey::UncountedSheet)
            && engine(RuleKey::StellarFrameCount)
            && self.rebuilt.still(stellar)
    }

    /// [`Session::counted_sheet`] by [`RuleKey::StellarFrameCount`]'s
    /// engine reading: the sheet `site`'s sprite slot held at the last
    /// rebuild point, graphic type 0's read as its `Type` unless `site`
    /// may draw graphic type 0 under the current rules (by the engine's
    /// reading of [`RuleKey::DeadTypeFallback`] with no `DeadType` sheet,
    /// or of [`RuleKey::TypeSheetFallback`] with no `Type` sheet).
    fn counted_sheet(&self, setup: &Setup, site: &LandingSite) -> DrawnSheet {
        let engine = |key| setup.rules.source_for(key) == RuleSource::Engine;
        let may_draw_type_zero = (site.dead_frame_size.is_none()
            && engine(RuleKey::DeadTypeFallback))
            || (site.frame_size.is_none() && engine(RuleKey::TypeSheetFallback));
        match self.rebuilt.counted(site.id) {
            DrawnSheet::TypeZero if !may_draw_type_zero => DrawnSheet::Type,
            sheet => sheet,
        }
    }

    /// The frame size of the sheet drawn for `site`, one of the stellars
    /// shown ([`Session::drawn_sheet`]), or `None` when no sheet is.
    pub(super) fn drawn_frame_size(
        &self,
        statics: &Statics,
        setup: &Setup,
        site: &LandingSite,
    ) -> Option<(u32, u32)> {
        sheet_frame_size(self.site_sheet(statics, setup, site), statics, site)
    }

    /// The rect a click and the hover read of `site`'s sprite slot (+0x1a,
    /// `_HandlePlayer` @0x6a63d, `_HandleMouse` @0x38233) with the player at
    /// `player`: none for a slot showing no frame ([`DrawnSheet::NoFrame`],
    /// a state the original never survives, its rebuild faulting first,
    /// `_SWSetSpriteLocation` @0xab665 from @0x331aa); else the frame drawn
    /// ([`Space::drawn_frame_size`], the [`PLACEHOLDER_FRAME`] when no
    /// sheet is) placed where the slot is: its top left at
    /// `sprite_pool::offset(site.position, player, slot size)` from the
    /// player (`_HandleStellarSprites` @0x2ee3d-0x2ef1a), the slot size
    /// [`Space::slot_sprite_size`] gives. A frame frozen is so placed as a
    /// 32-pixel sprite, its top left at the centre less 16, and keeps its
    /// stale frame's size (`_SWSetSpriteLocation` @0xab66d-0xab693 moves
    /// the rect without resizing it).
    pub(super) fn click_rect(
        &self,
        statics: &Statics,
        setup: &Setup,
        site: &LandingSite,
        player: Vec2,
    ) -> Option<SpriteRect> {
        let sheet = self.site_sheet(statics, setup, site);
        if sheet == DrawnSheet::NoFrame {
            return None;
        }
        let frame = sheet_frame_size(sheet, statics, site).unwrap_or(PLACEHOLDER_FRAME);
        let placed_as = self.slot_sprite_size(statics, setup, site).unwrap_or(frame);
        let (dx, dy) = sprite_pool::offset(site.position, player, placed_as);
        // A screen offset is far within an `f32`'s whole numbers.
        #[allow(clippy::cast_precision_loss)]
        let (dx, dy) = (dx as f32, dy as f32);
        Some(SpriteRect {
            left: player.x + dx,
            top: player.y + dy,
            width: frame.0,
            height: frame.1,
        })
    }

    /// The size `_GetSpriteXSize` and `_GetSpriteYSize` give `site`'s
    /// sprite slot (@0x26d4-0x2731): `None` when it shows no frame,
    /// [`NO_SPRITE_SIDE`] square when the frames it holds count none (a
    /// frame frozen, [`Session::frame_frozen`]), and otherwise the frame
    /// drawn's ([`Space::drawn_frame_size`]).
    pub(super) fn slot_sprite_size(
        &self,
        statics: &Statics,
        setup: &Setup,
        site: &LandingSite,
    ) -> Option<(u32, u32)> {
        if !self.rebuilt.shows_frame(site.id) {
            None
        } else if self.frame_frozen(setup, site.id) {
            Some((NO_SPRITE_SIDE, NO_SPRITE_SIDE))
        } else {
            self.drawn_frame_size(statics, setup, site)
        }
    }

    /// The width `_GetStellarSpriteXSize` reads of `site`'s `Type` master:
    /// the unloaded width, 150, when the player's system does not hold it
    /// (`held_here` false, [`Chart::held_here`]), as its owner test
    /// (@0xacb9-0xacc1) comes before its master tests (@0xacc3); else what
    /// the last rebuild point left of the master.
    fn type_width(&self, site: &LandingSite, held_here: bool) -> TypeWidth {
        if held_here {
            self.rebuilt.type_width(site)
        } else {
            TypeWidth::Unloaded
        }
    }

    /// How close the ship must be to `site`, one of the stellars shown, to
    /// land on it, by [`RuleKey::LandingDistance`]: the original's box
    /// reads what the last rebuild point and the flight frames since left
    /// of its sprite slot and its `Type` master (see the module docs). A
    /// stellar not `held_here` ([`Chart::held_here`]) reads the unloaded
    /// width, 150, whatever its master, as `_GetStellarSpriteXSize`'s owner
    /// test comes before its master tests (@0xacb9-0xacc1, @0xacc3).
    pub(super) fn landing_reach(
        &self,
        setup: &Setup,
        site: &LandingSite,
        held_here: bool,
    ) -> LandingReach {
        let shown = StellarShown {
            frame: self.rebuilt.shows_frame(site.id),
            type_width: self.type_width(site, held_here),
        };
        landing_reach(
            site,
            shown,
            setup.rules.source_for(RuleKey::LandingDistance),
        )
    }
}

/// The 64-pixel placeholder the views draw for a stellar with no sheet to
/// read (twice
/// [`UNKNOWN_STELLAR_RADIUS`](crate::landing::UNKNOWN_STELLAR_RADIUS)), and the frame a click reads of
/// it.
const PLACEHOLDER_FRAME: (u32, u32) = (64, 64);

/// The frame size of `sheet` as `site` draws it, or `None` when it has no
/// frame to read.
fn sheet_frame_size(
    sheet: DrawnSheet,
    statics: &Statics,
    site: &LandingSite,
) -> Option<(u32, u32)> {
    match sheet {
        DrawnSheet::Type => site.frame_size,
        DrawnSheet::Dead => site.dead_frame_size,
        DrawnSheet::TypeZero => statics.fallback_frame,
        DrawnSheet::NoFrame => None,
    }
}

impl Chart {
    /// A rebuild point's catalog-free parts, docked or not (see the module
    /// docs): the course rewritten, the exploration shared by place, the
    /// known set worked out afresh and the rescue kept for `pilot`'s
    /// system, which ends a wait for the rebuild. `Session::rebuild_quietly`
    /// runs them before its stellars, and a jump's in-flight move alone
    /// (see the `jump_moves` module).
    pub(super) fn rebuild_chart(&mut self, pilot: &mut Pilot, statics: &Statics, setup: &Setup) {
        self.rewrite_course(pilot, setup);
        self.share_explored(pilot, setup);
        self.rework_known(pilot);
        self.keep_rescues(pilot, statics, setup);
    }

    /// Destroys a new pilot's stellars that start the game destroyed
    /// (`Flags2` 0x0040), within the original's slots, each with the
    /// countdown `Y` gives it, as `_DoNewPilot` does before its `OnStart`
    /// (@0x192d6-0x1931c); by [`RuleKey::DestroyInvincible`]'s engine
    /// reading an invincible one starts whole, as `_StellarIsDestroyed`
    /// holds it.
    pub(super) fn destroy_from_the_start(
        &mut self,
        pilot: &mut Pilot,
        statics: &Statics,
        setup: &Setup,
        out: &mut Outbox,
    ) {
        let reading = setup.rules.source_for(RuleKey::DestroyInvincible);
        let ruined: Vec<_> = statics
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
            self.set_stellar_state(pilot, out, stellar, Some(countdown));
        }
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
    use crate::flight::Controls;
    use crate::landing::{LandingRefusal, StellarFlags};
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::session::{SessionEvent, StellarPick};
    use crate::story::Countdown;
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, land_now, life};
    use crate::testkit::{Passive, work, work_with};

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
            .chart
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

    /// F5 picks as `_FindNearestStellar` does (@0x276c-0x2888), a
    /// destroyed can-land stellar included, and beeps (@0x69e9a).
    #[test]
    fn f5_picks_a_destroyed_stellar_as_find_nearest_stellar_does() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        session.take_sounds();
        assert_eq!(
            session.select_stellar(StellarPick::Nearest),
            Some(StellarId(128))
        );
        assert!(
            session
                .take_sounds()
                .contains(&crate::SimSound::NavSelected)
        );
        run(&mut session, "U128");
        assert_eq!(
            session.select_stellar(StellarPick::Nearest),
            Some(StellarId(128))
        );
    }

    /// `catalog` with every stellar but 128 stripped of its flags.
    fn only_128_can_land(mut catalog: FakePilotCatalog) -> FakePilotCatalog {
        for (_, sites) in &mut catalog.sites {
            for site in sites {
                if site.id.0 != 128 {
                    site.flags = 0;
                }
            }
        }
        catalog
    }

    /// An untargeted L selects `_FindNearestStellar`'s pick silently
    /// (@0x276c-0x2888, @0x6a8ed-0x6a932) and the dock request refuses it
    /// as not landable (@0x66977 → 0x67cda).
    fn assert_l_selects_128_and_cannot_land(session: &mut Session) {
        session.take_sounds();
        assert_eq!(
            session.land(),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(128),
                station: false,
            })
        );
        assert_eq!(session.nav_target(), Some(StellarId(128)));
        assert_eq!(
            session.nav_mode(),
            crate::navigation::NavMode::Stellar(Some(StellarId(128)))
        );
        assert_eq!(session.dock_request(), None);
        assert!(
            !session
                .take_sounds()
                .contains(&crate::SimSound::NavSelected)
        );
    }

    #[test]
    fn an_untargeted_l_selects_the_only_can_land_stellar_destroyed_and_says_it_cannot_land() {
        let catalog = only_128_can_land(wrecking());
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        assert_l_selects_128_and_cannot_land(&mut session);
    }

    #[test]
    fn an_untargeted_l_selects_an_intact_stellar_landable_only_once_destroyed_and_says_it_cannot_land()
     {
        let catalog = only_128_can_land(ruins());
        let mut session = quiet(&catalog);
        assert_l_selects_128_and_cannot_land(&mut session);
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
        session
            .chart
            .assert_docked_among_shown(session.mode.landed());
        assert_eq!(session.landed(), Some(StellarId(140)));
    }

    #[test]
    fn a_second_y_restarts_the_countdown() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        session.chart.set_stellar_state(
            &mut session.pilot,
            &mut session.out,
            StellarId(128),
            Some(left(1)),
        );
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
            session.apply_op(
                &SetOp::DestroyStellar(StellarId(id)),
                &mut Scripted::default(),
            );
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
        session
            .chart
            .assert_docked_among_shown(session.mode.landed());
        assert_eq!(session.pilot().stellar, Some(StellarId(128)));
        assert_eq!(
            session.take_off(&catalog, &mut Scripted::default()),
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
            .take_off(&catalog, &mut Scripted::default())
            .expect("takes off");
        assert_eq!(destroyed(&session, 129), Some(left(1)), "a day down");
        assert!(!session.control_bit(bit(7)));
        land_now(&mut session).expect("lands");
        let _ = session.settle_script(&catalog, &mut Scripted::default());
        assert!(!session.control_bit(bit(8)), "not yet");
        session.take_events();
        session
            .take_off(&catalog, &mut Scripted::default())
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
        session.ship.fit.stats.jump_days = 3;
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
        session.chart.set_stellar_state(
            &mut session.pilot,
            &mut session.out,
            StellarId(300),
            Some(left(4)),
        );
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
    /// 130, its stellar 129 within the bound on a destination
    /// ([`far_129_reachable`](crate::testkit::far_129_reachable)).
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
            ..crate::testkit::far_129_reachable(wrecking())
        }
    }

    /// The goal of every NPC in `session`'s system after one traffic tick,
    /// each bound for 128 before it.
    fn goals_bound_for_128(session: &mut Session, catalog: &FakePilotCatalog) -> Vec<Goal> {
        for npc in session.space.traffic.npcs_mut() {
            npc.goal = Goal::Land(StellarId(128));
        }
        work_with(
            session,
            catalog,
            Controls::default(),
            &Peaceful,
            Rules::default(),
            &mut NeverFires,
        );
        session.npcs().iter().map(|npc| npc.goal).collect()
    }

    #[test]
    fn an_npc_never_heads_for_a_stellar_y_destroyed() {
        let catalog = trafficked();
        let mut session = quiet(&catalog);
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &Peaceful,
            Rules::default(),
            &mut NeverFires,
        );
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
        session
            .chart
            .assert_docked_among_shown(session.mode.landed());
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
            crate::testkit::resumed_on(&flown),
            Some(StellarId(140)),
            "{script}: on it"
        );
        session
    }

    #[test]
    fn a_landed_m_docks_at_the_first_stellar_destroyed_or_not() {
        let catalog = wrecking();
        let mut session = moved_landed(&catalog, "Y140 M131");
        assert!(session.chart.next_sites.as_deref().expect("waiting")[0].destroyed);
        assert_eq!(
            session.take_off(&catalog, &mut Scripted::default()),
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
        session.take_off(&catalog, &mut Scripted::default());
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
            session.take_off(&catalog, &mut Scripted::default());
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
    fn a_pilot_saved_docked_at_a_stellar_destroyed_since_resumes_on_it() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        land_now(&mut session).expect("lands");
        run(&mut session, "Y128");
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(crate::testkit::resumed_on(&flown), Some(StellarId(128)));
        assert!(shown_destroyed(&flown, 128));
        assert_eq!(destroyed(&flown, 128), Some(left(2)));
    }

    // Stellars shot down.

    use crate::catalog::{
        GovtId, GovtRecord, HullRecord, Penalties, StockWeapon, WeaponId, WeaponRecord,
    };
    use crate::combat::armament::{MOD_WEAPON, Trigger};
    use crate::combat::projectile::Source;
    use crate::combat::weapon::{Explosion, PLANET_TYPE};
    use crate::combat::{CombatEvent, Rules, ShipRef};
    use crate::geometry::Vec2;
    use crate::govt::{DERELICT, Governments};
    use crate::legal::{Crime, CrimeMap, LegalCode, NovaLaw, RECORD_LIMIT};
    use crate::testkit::{govt, hull, outfit, weapon};
    use crate::traffic::npc::NpcId;

    /// A planet-type gun firing every tick, 15 pixels a tick, doing 30
    /// mass and 20 energy damage and exploding in `bööm` 130 where it
    /// hits, of a `BlastRadius` of 25.
    fn planet_gun() -> WeaponRecord {
        WeaponRecord {
            count: 30,
            speed: 1500,
            mass_dmg: 30,
            energy_dmg: 20,
            flags2: PLANET_TYPE,
            explod_type: 2,
            blast_radius: 25,
            ..weapon(128)
        }
    }

    /// [`catalog`] with `lives`, its ship carrying a [`planet_gun`]. The
    /// ship starts over 128, so each shot hits 128 in the hit pass of the
    /// tick after it is fired, testing its first move: 128 of `Strength`
    /// 100 is shot down by the third.
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
        bays: false,
        bays_after: false,
        point_defence: false,
        keep: None,
    };

    /// Fires every tick for `ticks` ticks under `law`, then lets go for a
    /// tick more, whose hit pass tests the last shot's first move: the
    /// fight's events.
    fn fire_by(session: &mut Session, ticks: u32, law: &dyn LegalCode) -> Vec<CombatEvent> {
        session.hold_trigger(FIRE);
        let mut events = Vec::new();
        for tick in 0..=ticks {
            if tick == ticks {
                session.hold_trigger(Trigger::default());
            }
            let rules = Rules {
                law,
                ..Rules::default()
            };
            work_with(
                session,
                &crate::testkit::catalog(),
                Controls::default(),
                &Passive,
                rules,
                &mut NeverFires,
            );
            events.extend(session.take_combat_events());
        }
        events
    }

    fn fire(session: &mut Session, ticks: u32) -> Vec<CombatEvent> {
        fire_by(session, ticks, &NovaLaw::default())
    }

    /// The explosions among `events` of `bööm` `kind`, where each was.
    fn booms(events: &[CombatEvent], kind: i16) -> Vec<Vec2> {
        sized_booms(events, kind)
            .into_iter()
            .map(|(at, _)| at)
            .collect()
    }

    /// The explosions among `events` of `bööm` `kind`, where each was and
    /// its size: a `Y`'s, heard already, of size 0.
    fn sized_booms(events: &[CombatEvent], kind: i16) -> Vec<(Vec2, f32)> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Exploded {
                    at,
                    explosion,
                    size,
                } if Some(explosion) == Explosion::decode(kind) => Some((at, size)),
                CombatEvent::ExplodedHeard { at, explosion }
                    if Some(explosion) == Explosion::decode(kind) =>
                {
                    Some((at, 0.0))
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
    fn an_on_destroys_move_writes_the_system_at_once_and_enters_it_at_the_settling() {
        let catalog = shooting([destroying("M131")]);
        let mut session = quiet(&catalog);
        fire(&mut session, 3);
        assert_eq!(session.system(), SystemId(131), "at the operator");
        assert!(
            session.stellars_shown().any(|id| id == StellarId(128)),
            "130's until the entry"
        );
        let _ = session.settle_script(&catalog, &mut Scripted::default());
        assert_eq!(session.system(), SystemId(131));
        assert!(!session.stellars_shown().any(|id| id == StellarId(128)));
    }

    /// Fines government 140 a `kill` penalty of -3 for a killing, and
    /// along the walk the system the crime is in -2.6 and 131 +1.5;
    /// records each crime it is asked about with the victim's
    /// government, and each system a walk starts from.
    #[derive(Debug, Default)]
    struct Fine {
        seen: std::cell::RefCell<Vec<(Crime, Option<GovtId>)>>,
        from: std::cell::RefCell<Vec<SystemId>>,
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

        fn system_penalties(
            &self,
            _crime: Crime,
            _victim: Option<GovtId>,
            from: SystemId,
            _map: &dyn CrimeMap,
            _govts: &Governments,
        ) -> Vec<(SystemId, f32)> {
            self.from.borrow_mut().push(from);
            vec![(from, -2.6), (SystemId(131), 1.5)]
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
    fn shooting_a_stellar_down_walks_ten_times_from_the_players_system_each_truncating() {
        let catalog = shooting([life(128, 2)]);
        let mut session = overlaid(&catalog);
        session.pilot.set_system_record(SystemId(130), 5);
        let fine = Fine::default();
        fire_by(&mut session, 3, &fine);
        assert_eq!(fine.from.take(), [SystemId(130); 10], "the player's system");
        assert_eq!(
            session.pilot().system_record(SystemId(130)),
            -16,
            "5 - 2.6 to 2, then 0, -2 and on, each truncated in turn: not 5 - 26 \
             nor 5 - 10 x 2"
        );
        assert_eq!(
            session.pilot().system_record(SystemId(131)),
            10,
            "1.5 ten times, each truncated"
        );
    }

    #[test]
    fn an_npc_shooting_a_stellar_down_convicts_no_one() {
        let catalog = shooting([destroying("b5")]);
        let mut session = quiet(&catalog);
        let fine = Fine::default();
        session.shoot_down(
            StellarId(128),
            Some(ShipRef::Npc(NpcId(1))),
            &fine,
            &mut NeverFires,
        );
        assert_eq!(destroyed(&session, 128), Some(left(2)));
        assert!(session.control_bit(bit(5)));
        assert_eq!(fine.seen.take(), []);
        assert_eq!(session.pilot().legal_record(GovtId(140)), 0);
    }

    /// Rank 200 of government 141 (the overlay's), lost to any crime
    /// against it, and 201 of 142, lost to any crime against that.
    fn crime_ranks() -> Vec<crate::catalog::RankRecord> {
        [(200, 141), (201, 142)]
            .map(|(id, govt)| crate::catalog::RankRecord {
                affil_govt: Some(GovtId(govt)),
                flags: crate::rank::LOST_TO_CRIME,
                ..crate::testkit::rank(id)
            })
            .into()
    }

    /// The session's active ranks.
    fn ranks_held(session: &Session) -> Vec<i16> {
        session.pilot().ranks().iter().map(|id| id.0).collect()
    }

    /// `catalog`'s [`overlaid`] session, given [`crime_ranks`] and
    /// holding both.
    fn ranked(mut catalog: FakePilotCatalog) -> Session {
        catalog.ranks = crime_ranks();
        let mut session = overlaid(&catalog);
        session
            .pilot
            .ranks
            .extend([crate::catalog::RankId(200), crate::catalog::RankId(201)]);
        session
    }

    #[test]
    fn the_player_shooting_a_stellar_down_loses_its_governments_ranks() {
        let mut session = ranked(shooting([life(128, 2)]));
        fire(&mut session, 2);
        assert_eq!(ranks_held(&session), [200, 201], "not until it is down");
        fire(&mut session, 1);
        assert_eq!(ranks_held(&session), [201], "a killing of 141's");
    }

    #[test]
    fn an_npc_shooting_a_stellar_down_takes_no_rank() {
        let mut session = ranked(shooting([life(128, 2)]));
        session.shoot_down(
            StellarId(128),
            Some(ShipRef::Npc(NpcId(1))),
            &NovaLaw::default(),
            &mut NeverFires,
        );
        assert_eq!(destroyed(&session, 128), Some(left(2)));
        assert_eq!(ranks_held(&session), [200, 201]);
    }

    /// Whether `session`'s blessing has government 141 (the overlay's)
    /// bless landing.
    fn blessed_by_141(session: &Session) -> bool {
        session
            .blessing()
            .blesses(Some(GovtId(141)), crate::rank::Blessed::Landing)
    }

    #[test]
    fn a_rank_lost_to_shooting_a_stellar_down_outlasts_its_on_destroy_reset_by_the_engine() {
        for (source, after_kill) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
            // [`ranked`]'s session, but 200 blesses landing too.
            let mut catalog = shooting([destroying("b5")]);
            catalog.ranks = crime_ranks();
            catalog.ranks[0].flags |= crate::rank::BLESSES_LANDING;
            let mut session = overlaid(&catalog)
                .with_rules(Rulebook::default().with_override(RuleKey::BlessingRefresh, source));
            session
                .pilot
                .ranks
                .extend([crate::catalog::RankId(200), crate::catalog::RankId(201)]);
            session.reset_precalced();
            assert!(blessed_by_141(&session), "{source:?}: held and reset");
            session.shoot_down(
                StellarId(128),
                Some(ShipRef::Player),
                &NovaLaw::default(),
                &mut NeverFires,
            );
            assert!(session.control_bit(bit(5)), "{source:?}: its OnDestroy ran");
            assert_eq!(ranks_held(&session), [201], "{source:?}");
            assert_eq!(
                blessed_by_141(&session),
                after_kill,
                "{source:?}: OnDestroy's reset @0x374a9 runs before the slap @0x37518"
            );
            session.reset_precalced();
            assert!(!blessed_by_141(&session), "{source:?}: the next reset");
        }
    }

    /// [`ranked`]'s session, 128 taken to none by two shots, then shot
    /// down by a third fired from 200 pixels left of it, which a jamming
    /// draw has turned on its firer (`Source::Other`, aimed at the player)
    /// in flight when `turned`. The player moves out of its way, as the
    /// shot flies straight.
    fn shot_down_from_afar(turned: bool) -> Session {
        let mut session = ranked(shooting([life(128, 2)]));
        let fine = Fine::default();
        fire_by(&mut session, 2, &fine);
        session.ship.player = crate::flight::ShipState {
            position: Vec2::new(30.0 - 200.0, -40.0),
            velocity: Vec2::ZERO,
            heading: 90.0,
        };
        session.hold_trigger(FIRE);
        let rules = Rules {
            law: &fine,
            ..Rules::default()
        };
        work_with(
            &mut session,
            &crate::testkit::catalog(),
            Controls::default(),
            &Passive,
            rules,
            &mut NeverFires,
        );
        session.hold_trigger(Trigger::default());
        // The player moves off, and the hit pass meets it where it stood a
        // tick later, before the shot can turn on it.
        session.ship.player.position.y += 1000.0;
        work_with(
            &mut session,
            &crate::testkit::catalog(),
            Controls::default(),
            &Passive,
            rules,
            &mut NeverFires,
        );
        assert_eq!(destroyed(&session, 128), None, "still in flight");
        if turned {
            for shot in session.space.combat.shots_mut() {
                shot.source = Source::Other;
                shot.target = Some(ShipRef::Player);
                shot.aimed_at = Some(ShipRef::Player);
            }
        }
        for _ in 0..20 {
            let rules = Rules {
                law: &fine,
                ..Rules::default()
            };
            work_with(
                &mut session,
                &crate::testkit::catalog(),
                Controls::default(),
                &Passive,
                rules,
                &mut NeverFires,
            );
        }
        assert_eq!(destroyed(&session, 128), Some(left(2)), "shot down");
        session
    }

    #[test]
    fn a_players_shot_turned_on_its_firer_shooting_a_stellar_down_convicts_no_one() {
        let kept = shot_down_from_afar(false);
        assert_eq!(kept.pilot().legal_record(GovtId(140)), -30);
        assert_eq!(ranks_held(&kept), [201]);
        let turned = shot_down_from_afar(true);
        assert_eq!(
            turned.pilot().legal_record(GovtId(140)),
            0,
            "its source is no ship (`_CheckShotProximities` @0x374e1-0x374e7)"
        );
        assert_eq!(ranks_held(&turned), [200, 201]);
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
            let mut session = ranked(catalog);
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
            assert_eq!(
                ranks_held(&session).contains(&200),
                !crime,
                "{flags:#x}: @0x9bf2 returns before the ranks too"
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
        assert_eq!(
            sized_booms(&events, 4),
            [(Vec2::new(30.0, -40.0), 0.0)],
            "of no size, as `_CreateExplosion(.., 0, 1)` makes it"
        );
        let own: Vec<f32> = sized_booms(&events, 2)
            .into_iter()
            .map(|(_, size)| size)
            .collect();
        assert_eq!(own, [25.0], "the shot's own, of the gun's BlastRadius");
    }

    /// [`shooting`]'s session with the ship 200 pixels left of 128's
    /// centre and `off` below it, at rest, facing right, after one shot
    /// and the 20 ticks it then flies: the fight's events.
    fn shot_from_afar(off: f32) -> Vec<CombatEvent> {
        let catalog = shooting([life(128, 2)]);
        let mut session = quiet(&catalog);
        session.ship.player = crate::flight::ShipState {
            position: Vec2::new(30.0 - 200.0, -40.0 + off),
            velocity: Vec2::ZERO,
            heading: 90.0,
        };
        let mut events = fire(&mut session, 1);
        for _ in 0..20 {
            work(&mut session, &catalog, Controls::default());
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
        // No `Type` sheet, and no graphic type 0 to fill its slot.
        let mut catalog = shooting([life(128, 2)]);
        catalog.sites[0].1[0].frame_size = None;
        catalog.fallback_frame = None;
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
        assert_eq!(session.space.strength.get(&StellarId(128)), Some(&50));
        run(&mut session, "Y128");
        assert!(
            session.space.strength.is_empty(),
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
        assert!(!session.space.strength.is_empty(), "damaged");
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        assert!(session.space.strength.is_empty());
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
        assert!(session.space.destroying.is_empty(), "done");
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
        assert!(session.space.destroying.is_empty());
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
            .take_off(&catalog, &mut Scripted::default())
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
        session.ship.player.position = Vec2::new(2000.0, 0.0);
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
            .take_off(&catalog, &mut Scripted::default())
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
            [SessionEvent::SystemChanged, SessionEvent::StellarsRebuilt],
            "the same set, but 129 now counts graphic type 0's frames"
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
        work(&mut session, &catalog, Controls::default());
        assert_eq!(rebuilt(&session, &[128, 129]), [false, true]);
        run(&mut session, "Y128");
        work(&mut session, &catalog, Controls::default());
        assert_eq!(rebuilt(&session, &[128, 129]), [false, true], "not back");
    }

    #[test]
    fn a_landed_frame_drops_nothing() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y129");
        land_now(&mut session).expect("lands");
        run(&mut session, "U129");
        work(&mut session, &catalog, Controls::default());
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
        // A shot fired while it lies dead, which a frame samples dead.
        session.hold_trigger(FIRE);
        work(&mut session, &catalog, Controls::default());
        session.hold_trigger(Trigger::default());
        assert!(session.dead_since_rebuild(StellarId(128)));
        run(&mut session, "U128");
        session.space.strength.insert(StellarId(128), 0);
        // The next frame's hit pass shoots it down before the frame
        // samples it.
        work(&mut session, &catalog, Controls::default());
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
        work(session, &crate::testkit::catalog(), Controls::default());
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
        session.ship.player = ShipState {
            position: offset,
            ..ShipState::default()
        };
        session.chart.nav = crate::navigation::NavMode::Stellar(Some(StellarId(128)));
        session.port.dock = Some(crate::clearance::DockRequest {
            stellar: StellarId(128),
            permission: crate::clearance::CLEARED,
        });
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

    /// The land key pressed in a session of `catalog` with `rules`, 128
    /// targeted and cleared, the ship over its centre moving at
    /// `velocity`: the stellar landed on, or the refusal, and the sounds
    /// the press raised.
    fn land_moving(
        catalog: &FakePilotCatalog,
        rules: &Rulebook,
        velocity: Vec2,
    ) -> (Result<StellarId, LandingRefusal>, Vec<crate::SimSound>) {
        let mut session = quiet(catalog).with_rules(*rules);
        session.ship.player = ShipState {
            velocity,
            ..ShipState::default()
        };
        session.chart.nav = crate::navigation::NavMode::Stellar(Some(StellarId(128)));
        session.port.dock = Some(crate::clearance::DockRequest {
            stellar: StellarId(128),
            permission: crate::clearance::CLEARED,
        });
        let _ = session.take_sounds();
        let landed = session.land().map(|press| match press {
            LandPress::Outcome(LandOutcome::Landed(stellar)) => stellar,
            other => panic!("not a landing: {other:?}"),
        });
        (landed, session.take_sounds())
    }

    #[test]
    fn the_player_lands_by_the_per_axis_speed_by_default_and_by_the_overall_speed_by_the_other_reading()
     {
        let catalog = landing_at(100, StellarFlags::CAN_LAND, None, 12);
        let too_fast = |velocity: Vec2| {
            Err(LandingRefusal::TooFast {
                stellar: StellarId(128),
                station: false,
                speed: velocity.length(),
            })
        };
        let engine = Rulebook::default();
        assert_eq!(
            land_moving(&catalog, &engine, Vec2::new(0.7, 0.7)).0,
            Ok(StellarId(128))
        );
        let (refused, sounds) = land_moving(&catalog, &engine, Vec2::new(0.8, 0.0));
        assert_eq!(refused, too_fast(Vec2::new(0.8, 0.0)));
        assert_eq!(sounds, [crate::SimSound::Refused], "@0x676fe");

        let overall = Rulebook::default().with_override(RuleKey::LandingSpeed, RuleSource::Bible);
        assert_eq!(
            land_moving(&catalog, &overall, Vec2::new(0.8, 0.0)).0,
            Ok(StellarId(128))
        );
        assert_eq!(
            land_moving(&catalog, &overall, Vec2::new(0.74, 0.74)).0,
            too_fast(Vec2::new(0.74, 0.74))
        );
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

    /// Stellar 128, whole and landable, 100 square, whose `Type`'s `spïn`
    /// counts no frames, with a `DeadType` that counts.
    fn uncounted_type() -> FakePilotCatalog {
        let mut catalog = ruin(100, false);
        catalog.sites[0].1[0].flags = StellarFlags::CAN_LAND;
        catalog.sites[0].1[0].type_counts_frames = false;
        catalog
    }

    /// `Y128`, a frame, `U128` and a frame: the slot keeps the `DeadType`
    /// frame through the `Type` clone that counts none.
    fn keeps_a_dead_frame(session: &mut Session) {
        run(session, "Y128");
        frame(session);
        run(session, "U128");
        frame(session);
    }

    fn uncounted_sheet(reading: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::UncountedSheet, reading)
    }

    #[test]
    fn a_whole_stellar_whose_type_counts_no_frames_reaches_75_after_a_rebuild_point_and_56_keeping_an_earlier_frame()
     {
        let catalog = uncounted_type();
        let flown = |_: &mut Session| {};
        assert!(
            reaches(&catalog, &engine(), &flown, 74.0, 75.0),
            "a session flown is a rebuild point: no frame"
        );
        assert!(
            reaches(&catalog, &engine(), &keeps_a_dead_frame, 55.0, 56.0),
            "a frame kept, and the loaded `Type` reads 32"
        );
        let rebuilt = |session: &mut Session| rebuild(session, &catalog);
        assert!(reaches(&catalog, &engine(), &rebuilt, 74.0, 75.0));
        // The rules apply from the next rebuild point: the session was
        // flown under the default ones.
        let counted = uncounted_sheet(RuleSource::Bible);
        let kept_then_rebuilt = |session: &mut Session| {
            rebuild(session, &catalog);
            keeps_a_dead_frame(session);
        };
        for setup in [&rebuilt as &dyn Fn(&mut Session), &kept_then_rebuilt] {
            assert!(reaches(&catalog, &counted, setup, 55.0, 56.0));
        }
        assert!(reaches(&catalog, &other_reading(), &flown, 50.0, 50.5));
    }

    #[test]
    fn an_npc_enters_a_gate_by_the_width_its_type_master_gives() {
        // 128, destroyed, a hypergate 100 wide: a reach of 25, and of 37
        // once a rebuild point has unloaded its master (150 wide).
        let mut catalog = ruin(100, false);
        catalog.sites[0].1[0].flags2 = crate::gate::HYPERGATE;
        let entering = |rebuilt: bool| {
            let mut session = quiet(&catalog);
            run(&mut session, "Y128");
            if rebuilt {
                rebuild(&mut session, &catalog);
            }
            work_with(
                &mut session,
                &catalog,
                Controls::default(),
                &Peaceful,
                Rules::default(),
                &mut NeverFires,
            );
            let mut npc =
                crate::testkit::npc(0, crate::stats::ShipStats::new(crate::testkit::FAST, &[]));
            npc.goal = Goal::EnterGate(StellarId(128));
            npc.state.position = Vec2::new(30.0, 0.0);
            let id = session.space.traffic.add_npc(npc);
            work_with(
                &mut session,
                &catalog,
                Controls::default(),
                &Peaceful,
                Rules::default(),
                &mut NeverFires,
            );
            session.space.npc(id).expect("there").entering_gate()
        };
        assert!(!entering(false), "30 out of 25");
        assert!(entering(true), "30 within 37");
    }

    #[test]
    fn a_session_flown_has_rebuilt_its_masters_at_once() {
        let catalog = ruin(100, false);
        let session = quiet(&catalog);
        let site = session.chart.sites[0];
        assert_eq!(session.space.rebuilt.type_width(&site), TypeWidth::Frame);
        assert_eq!(
            session.space.landing_reach(&session.setup, &site, true),
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

    /// The stellar a click at (`x`, `y`) picks, with none selected
    /// before it (a click passes over the one selected).
    fn click(session: &mut Session, x: f32, y: f32) -> Option<StellarId> {
        session.chart.nav = crate::NavMode::Off;
        crate::testkit::click_stellar(session, Vec2::new(x, y))
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

    // The sheet drawn (stellar-fidelity phase 1's one owner).

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

    /// The sheet stellar 128 counts the frames of.
    fn counted_128(session: &Session) -> DrawnSheet {
        session.counted_sheet(StellarId(128))
    }

    /// Stellar 128's counted sheet whole, after `Y128`, after a rebuild
    /// point, after `U128`, and after another rebuild point, in a session
    /// of `catalog` with `rules`.
    fn counted_walk(catalog: &FakePilotCatalog, rules: Rulebook) -> [DrawnSheet; 5] {
        let mut session = quiet(catalog).with_rules(rules);
        let whole = counted_128(&session);
        run(&mut session, "Y128");
        let destroyed = counted_128(&session);
        rebuild(&mut session, catalog);
        let rebuilt = counted_128(&session);
        run(&mut session, "U128");
        let regenerated = counted_128(&session);
        rebuild(&mut session, catalog);
        [
            whole,
            destroyed,
            rebuilt,
            regenerated,
            counted_128(&session),
        ]
    }

    fn frame_count(reading: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::StellarFrameCount, reading)
    }

    use DrawnSheet::{Dead, Type, TypeZero};

    #[test]
    fn by_the_engine_a_stellar_counts_the_sheet_of_its_last_rebuild_point() {
        let catalog = clicking(Some((200, 200)), Some((100, 100)));
        assert_eq!(
            counted_walk(&catalog, Rulebook::default()),
            [Type, Type, Dead, Dead, Type]
        );
        assert_eq!(
            counted_walk(&catalog, frame_count(RuleSource::Engine)),
            [Type, Type, Dead, Dead, Type]
        );
    }

    #[test]
    fn by_the_bible_a_stellar_counts_the_sheet_it_draws() {
        let catalog = clicking(Some((200, 200)), Some((100, 100)));
        assert_eq!(
            counted_walk(&catalog, frame_count(RuleSource::Bible)),
            [Type, Dead, Dead, Type, Type]
        );
    }

    #[test]
    fn a_stellar_with_no_dead_type_counts_graphic_type_0_from_its_second_rebuild_point() {
        let catalog = clicking(None, Some((100, 100)));
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        rebuild(&mut session, &catalog);
        assert_eq!(counted_128(&session), Type, "its slot held Type");
        assert_eq!(sheet_128(&session), TypeZero, "drawing graphic type 0");
        rebuild(&mut session, &catalog);
        assert_eq!(counted_128(&session), TypeZero);
        // By the Bible's `dead_type_fallback` graphic type 0 is never
        // drawn, so never counted.
        let bible = Rulebook::default().with_override(RuleKey::DeadTypeFallback, RuleSource::Bible);
        assert_eq!(counted_128(&session.with_rules(bible)), Type);
        // By the Bible's `stellar_frame_count`, the sheet drawn at once.
        let mut session = quiet(&catalog).with_rules(frame_count(RuleSource::Bible));
        run(&mut session, "Y128");
        rebuild(&mut session, &catalog);
        assert_eq!(counted_128(&session), TypeZero);
        let both = frame_count(RuleSource::Bible)
            .with_override(RuleKey::DeadTypeFallback, RuleSource::Bible);
        assert_eq!(counted_128(&session.with_rules(both)), Type);
    }

    // A `spïn` that counts no frames (stellar-fidelity phase 8).

    /// [`clicking`]'s stellar 128, 40 square, with a `DeadType` sheet 200
    /// square that counts, whose `Type` counts no frames.
    fn frozen_on_dead() -> FakePilotCatalog {
        let mut catalog = clicking(Some((200, 200)), Some((100, 100)));
        catalog.sites[0].1[0].type_counts_frames = false;
        catalog
    }

    #[test]
    fn a_whole_stellar_whose_type_counts_no_frames_draws_no_frame_then_keeps_its_dead_frame_frozen()
    {
        let catalog = frozen_on_dead();
        let mut session = quiet(&catalog);
        assert_eq!(sheet_128(&session), DrawnSheet::NoFrame, "flown");
        assert!(!session.frame_frozen(StellarId(128)));
        frame(&mut session);
        assert_eq!(sheet_128(&session), DrawnSheet::NoFrame, "a frame");
        run(&mut session, "Y128");
        assert_eq!(sheet_128(&session), DrawnSheet::Dead);
        frame(&mut session);
        assert!(!session.frame_frozen(StellarId(128)));
        run(&mut session, "U128");
        frame(&mut session);
        assert_eq!(sheet_128(&session), DrawnSheet::Dead, "kept");
        assert!(session.frame_frozen(StellarId(128)));
        rebuild(&mut session, &catalog);
        assert_eq!(sheet_128(&session), DrawnSheet::NoFrame, "a rebuild point");
        assert!(!session.frame_frozen(StellarId(128)));
    }

    #[test]
    fn by_the_other_reading_a_type_that_counts_no_frames_draws_its_type() {
        let catalog = frozen_on_dead();
        let mut session = quiet(&catalog).with_rules(uncounted_sheet(RuleSource::Bible));
        rebuild(&mut session, &catalog);
        assert_eq!(sheet_128(&session), DrawnSheet::Type);
        keeps_a_dead_frame(&mut session);
        assert_eq!(sheet_128(&session), DrawnSheet::Type);
        assert!(!session.frame_frozen(StellarId(128)));
        assert!(!session.counts_no_frames(StellarId(128)));
    }

    #[test]
    fn a_destroyed_stellar_whose_dead_type_counts_no_frames_draws_no_frame_from_a_rebuild_point() {
        let mut catalog = frozen_on_dead();
        catalog.sites[0].1[0].type_counts_frames = true;
        catalog.sites[0].1[0].dead_counts_frames = false;
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        frame(&mut session);
        assert_eq!(
            sheet_128(&session),
            DrawnSheet::Type,
            "its `Type` frame kept"
        );
        assert!(session.frame_frozen(StellarId(128)));
        rebuild(&mut session, &catalog);
        assert_eq!(sheet_128(&session), DrawnSheet::NoFrame);
    }

    #[test]
    fn a_whole_stellar_with_no_type_sheet_keeps_graphic_type_0s_frame_through_an_uncounted_dead_type()
     {
        let mut catalog = no_type_sheet(Some((300, 300)), Some((200, 200)));
        catalog.sites[0].1[0].dead_counts_frames = false;
        let mut session = quiet(&catalog);
        assert_eq!(sheet_128(&session), TypeZero, "flown: a rebuild point");
        run(&mut session, "Y128");
        frame(&mut session);
        assert_eq!(
            sheet_128(&session),
            TypeZero,
            "its `DeadType` clone kept it"
        );
        assert!(session.frame_frozen(StellarId(128)));
        run(&mut session, "U128");
        frame(&mut session);
        assert_eq!(
            sheet_128(&session),
            TypeZero,
            "its `Type` clone fails, and the kept frame stays"
        );
        assert!(session.frame_frozen(StellarId(128)));
    }

    #[test]
    fn graphic_type_0_that_counts_no_frames_leaves_a_slot_it_fills_with_no_frame() {
        let mut catalog = clicking(None, Some((100, 100)));
        catalog.fallback_counts = false;
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        rebuild(&mut session, &catalog);
        assert_eq!(sheet_128(&session), DrawnSheet::NoFrame);
        // Phase 3's placeholder, with no graphic type 0 to read.
        let catalog = clicking(None, None);
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        rebuild(&mut session, &catalog);
        assert_eq!(sheet_128(&session), DrawnSheet::TypeZero);
    }

    #[test]
    fn a_stellar_counts_no_frames_from_a_rebuild_point_showing_an_uncounted_type_until_one_clones_a_counting_sheet()
     {
        let catalog = frozen_on_dead();
        let me = StellarId(128);
        let mut session = quiet(&catalog);
        assert!(session.counts_no_frames(me), "flown");
        keeps_a_dead_frame(&mut session);
        assert!(session.counts_no_frames(me), "until a rebuild point");
        run(&mut session, "Y128");
        frame(&mut session);
        assert!(session.counts_no_frames(me));
        rebuild(&mut session, &catalog);
        assert!(!session.counts_no_frames(me), "its `DeadType` counts");
        // By the other reading of either key, it counts.
        for rules in [
            uncounted_sheet(RuleSource::Bible),
            frame_count(RuleSource::Bible),
        ] {
            let mut session = quiet(&catalog).with_rules(rules);
            rebuild(&mut session, &catalog);
            assert!(!session.counts_no_frames(me), "{rules:?}");
        }
    }

    /// `session` with the player at (`x`, `y`).
    fn player_at(session: &mut Session, x: f32, y: f32) {
        session.ship.player.position = Vec2::new(x, y);
    }

    #[test]
    fn a_slot_showing_no_frame_has_no_frame_size_and_a_frozen_one_its_kept_sheets() {
        let catalog = frozen_on_dead();
        let mut session = quiet(&catalog);
        player_at(&mut session, 0.0, 0.0);
        let site = session.chart.sites[0];
        let frame_size = |session: &Session| {
            session
                .space
                .drawn_frame_size(&session.statics, &session.setup, &site)
        };
        assert_eq!(frame_size(&session), None);
        // No rect: never clicked.
        assert_eq!(click(&mut session, 0.0, 0.0), None);
        assert_eq!(click(&mut session, 31.0, 0.0), None);
        keeps_a_dead_frame(&mut session);
        assert_eq!(frame_size(&session), Some((200, 200)));
        // Clicked by its `DeadType` rect from the centre less 16.
        assert_eq!(click(&mut session, 90.0, 0.0), Some(StellarId(128)));
        assert_eq!(click(&mut session, -90.0, 0.0), None);
        assert_eq!(
            session
                .stellar_targets()
                .iter()
                .map(|target| target.radius)
                .collect::<Vec<_>>(),
            [100.0]
        );
    }

    #[test]
    fn a_frozen_slot_is_clicked_by_its_stale_frame_from_the_centre_less_16() {
        let catalog = frozen_on_dead();
        let mut session = quiet(&catalog);
        player_at(&mut session, 0.0, 0.0);
        keeps_a_dead_frame(&mut session);
        assert!(session.frame_frozen(StellarId(128)));
        // [-16, 184) on each axis, 200 wide so with no margin.
        for (x, y) in [(-16.0, -16.0), (90.0, 0.0), (183.5, 183.5)] {
            assert_eq!(
                click(&mut session, x, y),
                Some(StellarId(128)),
                "({x}, {y})"
            );
        }
        for (x, y) in [(-17.0, 0.0), (-90.0, 0.0), (184.0, 0.0), (0.0, 184.0)] {
            assert_eq!(click(&mut session, x, y), None, "({x}, {y})");
        }
    }

    #[test]
    fn a_frozen_small_frame_gets_the_margin_by_its_own_width() {
        let mut catalog = frozen_on_dead();
        catalog.sites[0].1[0].type_counts_frames = true;
        catalog.sites[0].1[0].dead_counts_frames = false;
        let mut session = quiet(&catalog);
        player_at(&mut session, 0.0, 0.0);
        run(&mut session, "Y128");
        frame(&mut session);
        assert!(session.frame_frozen(StellarId(128)));
        // Its 40-pixel `Type` frame at [-16, 24), grown to [-32, 40).
        for x in [-32.0, 39.5] {
            assert_eq!(click(&mut session, x, 0.0), Some(StellarId(128)), "{x}");
        }
        for x in [-32.5, 40.0] {
            assert_eq!(click(&mut session, x, 0.0), None, "{x}");
        }
    }

    #[test]
    fn a_frameless_slot_is_never_clicked_or_hovered() {
        let catalog = frozen_on_dead();
        let mut session = quiet(&catalog);
        player_at(&mut session, 0.0, 0.0);
        assert_eq!(sheet_128(&session), DrawnSheet::NoFrame);
        for x in [0.0, 31.0] {
            assert_eq!(click(&mut session, x, 0.0), None, "{x}");
            assert_eq!(
                session.hover(Vec2::new(x, 0.0), &crate::testkit::NoShips),
                None,
                "{x}"
            );
        }
    }

    #[test]
    fn by_the_other_reading_a_frozen_type_is_clicked_by_its_type_centred() {
        let catalog = frozen_on_dead();
        let mut session = quiet(&catalog).with_rules(uncounted_sheet(RuleSource::Bible));
        player_at(&mut session, 0.0, 0.0);
        rebuild(&mut session, &catalog);
        keeps_a_dead_frame(&mut session);
        assert_eq!(sheet_128(&session), DrawnSheet::Type);
        // 40 square centred, [-20, 20), grown to [-36, 36).
        for x in [-36.0, 35.5] {
            assert_eq!(click(&mut session, x, 0.0), Some(StellarId(128)), "{x}");
        }
        for x in [-36.5, 36.0] {
            assert_eq!(click(&mut session, x, 0.0), None, "{x}");
        }
    }

    #[test]
    fn a_slot_rect_sits_at_whole_pixels_from_the_player() {
        let catalog = clicking(Some((200, 200)), Some((100, 100)));
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        player_at(&mut session, 0.75, 0.25);
        assert_eq!(sheet_128(&session), DrawnSheet::Dead);
        // 0.75 + trunc(-0.75) - 100: its left at -99.25, its right at
        // 100.75.
        assert_eq!(click(&mut session, -99.25, 0.0), Some(StellarId(128)));
        assert_eq!(click(&mut session, -99.5, 0.0), None);
        assert_eq!(click(&mut session, 100.5, 0.0), Some(StellarId(128)));
        assert_eq!(click(&mut session, 100.75, 0.0), None);
    }

    #[test]
    fn shots_pass_through_a_stellar_whose_slot_shows_no_frame() {
        let mut catalog = shooting([life(128, 2)]);
        catalog.sites[0].1[0].type_counts_frames = false;
        let mut session = quiet(&catalog);
        assert_eq!(sheet_128(&session), DrawnSheet::NoFrame);
        let events = fire(&mut session, 5);
        assert_eq!(booms(&events, 2), [], "no hit");
        assert_eq!(destroyed(&session, 128), None);
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

    // A whole stellar whose `Type` sheet cannot be read (stellar-fidelity
    // phase 3).

    /// [`clicking`]'s catalog with 128's `Type` sheet unreadable.
    fn no_type_sheet(
        dead_frame_size: Option<(u32, u32)>,
        fallback_frame: Option<(u32, u32)>,
    ) -> FakePilotCatalog {
        let mut catalog = clicking(dead_frame_size, fallback_frame);
        catalog.sites[0].1[0].frame_size = None;
        catalog
    }

    fn type_sheet(reading: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::TypeSheetFallback, reading)
    }

    #[test]
    fn by_the_engine_a_whole_stellar_with_no_type_sheet_draws_type_zero_from_a_rebuild_point() {
        for fallback in [Some((200, 200)), None] {
            for rules in [Rulebook::default(), type_sheet(RuleSource::Engine)] {
                let catalog = no_type_sheet(None, fallback);
                let mut session = Session::start(&catalog).expect("starts");
                session.space.rebuilt = crate::story::rebuild::Rebuilt::default();
                let mut session = session.with_rules(rules);
                assert_eq!(sheet_128(&session), Type, "before any rebuild point");
                rebuild(&mut session, &catalog);
                assert_eq!(sheet_128(&session), TypeZero, "{fallback:?}");
                frame(&mut session);
                assert_eq!(sheet_128(&session), TypeZero, "{fallback:?}");
                // A session flown has rebuilt at once.
                assert_eq!(sheet_128(&quiet(&catalog)), TypeZero, "{fallback:?}");
            }
        }
    }

    #[test]
    fn by_the_other_reading_a_whole_stellar_with_no_type_sheet_draws_its_type() {
        for fallback in [Some((200, 200)), None] {
            let catalog = no_type_sheet(Some((300, 300)), fallback);
            let mut session = quiet(&catalog).with_rules(type_sheet(RuleSource::Bible));
            assert_eq!(sheet_128(&session), Type, "{fallback:?}");
            run(&mut session, "Y128");
            frame(&mut session);
            assert_eq!(sheet_128(&session), Dead, "{fallback:?}");
            run(&mut session, "U128");
            frame(&mut session);
            assert_eq!(sheet_128(&session), Type, "{fallback:?}");
            rebuild(&mut session, &catalog);
            assert_eq!(sheet_128(&session), Type, "{fallback:?}");
        }
    }

    #[test]
    fn y_and_a_frame_then_u_keeps_its_dead_type_sheet_until_a_rebuild_point() {
        let catalog = no_type_sheet(Some((300, 300)), Some((200, 200)));
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        assert_eq!(sheet_128(&session), Dead);
        frame(&mut session);
        run(&mut session, "U128");
        assert_eq!(sheet_128(&session), Dead, "its `Type` clone fails");
        frame(&mut session);
        assert_eq!(sheet_128(&session), Dead);
        assert_eq!(click(&mut session, 140.0, 0.0), Some(StellarId(128)));
        rebuild(&mut session, &catalog);
        assert_eq!(sheet_128(&session), TypeZero);
        assert_eq!(click(&mut session, 140.0, 0.0), None);
    }

    #[test]
    fn y_u_with_no_frame_between_keeps_type_zero() {
        let catalog = no_type_sheet(Some((300, 300)), Some((200, 200)));
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        run(&mut session, "U128");
        assert_eq!(sheet_128(&session), TypeZero);
        frame(&mut session);
        assert_eq!(sheet_128(&session), TypeZero);
    }

    #[test]
    fn with_graphic_type_0_unreadable_a_whole_stellar_with_no_type_sheet_draws_its_dead_type_held()
    {
        let catalog = no_type_sheet(Some((300, 300)), None);
        let mut session = quiet(&catalog);
        assert_eq!(sheet_128(&session), TypeZero, "its slot shows no frame");
        run(&mut session, "Y128");
        frame(&mut session);
        run(&mut session, "U128");
        assert_eq!(sheet_128(&session), Dead, "the frame filled its slot");
        rebuild(&mut session, &catalog);
        assert_eq!(sheet_128(&session), TypeZero, "emptied again");
    }

    #[test]
    fn it_is_clicked_by_graphic_type_0s_frame() {
        let catalog = no_type_sheet(None, Some((200, 200)));
        let mut session = quiet(&catalog);
        assert_eq!(click(&mut session, 90.0, 0.0), Some(StellarId(128)));
        assert_eq!(click(&mut session, 110.0, 0.0), None);
        // By the other reading, the 64-pixel placeholder.
        let mut session = quiet(&catalog).with_rules(type_sheet(RuleSource::Bible));
        assert_eq!(click(&mut session, 31.0, 0.0), Some(StellarId(128)));
        assert_eq!(click(&mut session, 34.0, 0.0), None);
        assert_eq!(click(&mut session, 90.0, 0.0), None);
    }

    #[test]
    fn it_lands_within_262_and_within_75_when_graphic_type_0_cannot_be_read() {
        let mut catalog = landing_at(100, StellarFlags::CAN_LAND, None, 12);
        catalog.sites[0].1[0].frame_size = None;
        let whole = |_: &mut Session| {};
        assert!(reaches(&catalog, &engine(), &whole, 261.0, 262.0));
        catalog.fallback_frame = None;
        assert!(reaches(&catalog, &engine(), &whole, 74.0, 75.0));
    }

    #[test]
    fn a_whole_stellar_with_no_type_sheet_counts_graphic_type_0_from_its_second_rebuild_point() {
        let catalog = no_type_sheet(Some((300, 300)), Some((100, 100)));
        let mut session = quiet(&catalog);
        assert_eq!(counted_128(&session), Type, "its slot held Type");
        rebuild(&mut session, &catalog);
        assert_eq!(counted_128(&session), TypeZero);
        assert_eq!(
            counted_128(&session.with_rules(type_sheet(RuleSource::Bible))),
            Type,
            "never drawn by the other reading, so never counted"
        );
        // A destroyed one with no `DeadType` may still draw graphic type 0
        // by its whole reading's key.
        let catalog = no_type_sheet(None, Some((100, 100)));
        let mut session = quiet(&catalog).with_rules(
            Rulebook::default().with_override(RuleKey::DeadTypeFallback, RuleSource::Bible),
        );
        rebuild(&mut session, &catalog);
        run(&mut session, "Y128");
        assert_eq!(counted_128(&session), TypeZero);
        assert_eq!(
            sheet_128(&session),
            Type,
            "by the Bible's dead_type_fallback"
        );
    }

    /// The explosions of a planet-type shot fired at 128 from 300 pixels
    /// to its left, in a session of `catalog` with `rules`.
    fn shot_at(catalog: &FakePilotCatalog, rules: Rulebook) -> Vec<Vec2> {
        let mut session = quiet(catalog).with_rules(rules);
        session.ship.player = crate::flight::ShipState {
            position: Vec2::new(30.0 - 300.0, -40.0),
            velocity: Vec2::ZERO,
            heading: 90.0,
        };
        let mut events = fire(&mut session, 1);
        for _ in 0..30 {
            work_with(
                &mut session,
                &crate::testkit::catalog(),
                Controls::default(),
                &Passive,
                Rules::default(),
                &mut NeverFires,
            );
            events.extend(session.take_combat_events());
        }
        booms(&events, 2)
    }

    #[test]
    fn a_planet_type_shot_hits_it_by_graphic_type_0s_disc() {
        let mut catalog = shooting([life(128, 2)]);
        catalog.sites[0].1[0].frame_size = None;
        catalog.fallback_frame = Some((200, 200));
        let seen = shot_at(&catalog, Rulebook::default());
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(
            (seen[0] - Vec2::new(30.0 - 100.0, -40.0)).length() < 1e-3,
            "half graphic type 0's 200-pixel frame from its centre: {seen:?}"
        );
        assert_eq!(
            shot_at(&catalog, type_sheet(RuleSource::Bible)),
            [],
            "no target by the other reading"
        );
    }

    // The stellars shown.

    /// [`catalog`] with stellar 200 at (500, 500) listed by both 130 and
    /// 131, at different places, and every system hideable.
    fn twice_listed() -> FakePilotCatalog {
        let mut catalog = catalog();
        for (_, sites) in &mut catalog.sites {
            sites.push(crate::testkit::planet(200, 500.0, 500.0));
        }
        crate::testkit::hideable(&mut catalog);
        catalog
    }

    /// The stellars `session` shows.
    fn shown(session: &Session) -> Vec<i16> {
        session.stellars_shown().map(|id| id.0).collect()
    }

    #[test]
    fn a_system_shows_only_the_stellars_it_holds() {
        let catalog = twice_listed();
        let mut session = quiet(&catalog);
        assert_eq!(shown(&session), [128, 129, 200], "130 owns 200");
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        assert_eq!(shown(&session), [140], "200 is 130's, though 131 lists it");
        session
            .chart
            .assert_docked_among_shown(session.mode.landed());
    }

    #[test]
    fn the_players_system_shows_a_stellar_no_one_owns() {
        let mut catalog = catalog();
        catalog
            .star_map
            .push(crate::testkit::star(134, (600.0, 0.0), &[]));
        catalog.systems.push(SystemId(134));
        catalog
            .sites
            .push((SystemId(134), vec![crate::testkit::planet(141, 0.0, 0.0)]));
        crate::testkit::hideable(&mut catalog);
        let mut pilot = crate::pilot::Pilot::new(&catalog, "Ada").expect("starts");
        pilot.system = SystemId(134);
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.system(), SystemId(134), "shadowed by 131, it stays");
        assert_eq!(session.galaxy().owner(&[SystemId(134)]), None);
        assert_eq!(shown(&session), [141], "rescued: 134 holds it");
        let mut session = quiet(&catalog);
        crate::session::tests::hide(&mut session, &catalog, &[131, 134]);
        settle(&mut session, &catalog, "M131");
        assert_eq!(session.system(), SystemId(131), "the chain head, shown");
        assert_eq!(session.galaxy().owner(&[SystemId(131)]), None);
        assert_eq!(shown(&session), [140], "rescued: 131 holds it");
    }

    #[test]
    fn which_stellars_are_shown_changes_only_at_a_rebuild_point() {
        let catalog = twice_listed();
        let mut session = quiet(&catalog);
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        crate::session::tests::hide(&mut session, &catalog, &[130]);
        assert_eq!(
            shown(&session),
            [140],
            "a galaxy change is no rebuild point"
        );
        session.ship.player.position = Vec2::ZERO;
        session.ship.player.velocity = Vec2::ZERO;
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        assert_eq!(
            shown(&session),
            [140],
            "docked, the landing shows nothing afresh"
        );
        session.take_events();
        session
            .take_off(&catalog, &mut Scripted::default())
            .expect("takes off");
        assert_eq!(shown(&session), [140, 200], "131 holds 200 now");
        assert!(
            session
                .take_events()
                .contains(&SessionEvent::StellarsRebuilt)
        );
        assert_eq!(
            session.nav_target(),
            None,
            "a flight rebuild point never keeps a navigation target"
        );
    }

    #[test]
    fn a_rebuild_point_that_shows_a_dead_stellar_afresh_tells_it_rebuilt() {
        let catalog = twice_listed().living([life(200, -1)]);
        let mut session = quiet(&catalog);
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        run(&mut session, "Y200");
        crate::session::tests::hide(&mut session, &catalog, &[130]);
        session.ship.player.position = Vec2::ZERO;
        session.ship.player.velocity = Vec2::ZERO;
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        session.take_events();
        session
            .take_off(&catalog, &mut Scripted::default())
            .expect("takes off");
        assert_eq!(shown(&session), [140, 200]);
        assert!(session.dead_since_rebuild(StellarId(200)), "shown and dead");
        assert!(
            session
                .take_events()
                .contains(&SessionEvent::StellarsRebuilt),
            "both the set and the dead changed"
        );
    }

    #[test]
    fn a_galaxy_change_that_regoverns_a_shown_stellar_keeps_the_set_shown() {
        let catalog = twice_listed();
        let mut session = quiet(&catalog);
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_visible(SystemId(130), Some(false));
            overlay.set_stellar_govt(StellarId(140), Some(Some(GovtId(150))));
        });
        assert_eq!(shown(&session), [140], "read again, not shown afresh");
        assert_eq!(session.chart.sites[0].govt, Some(GovtId(150)));
    }

    #[test]
    fn a_galaxy_change_reads_again_a_stellar_listed_but_not_shown() {
        let catalog = twice_listed();
        let mut session = quiet(&catalog);
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_stellar_govt(StellarId(200), Some(Some(GovtId(150))));
        });
        crate::session::tests::hide(&mut session, &catalog, &[130]);
        session.ship.player.position = Vec2::ZERO;
        session.ship.player.velocity = Vec2::ZERO;
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        session
            .take_off(&catalog, &mut Scripted::default())
            .expect("takes off");
        let site = session
            .chart
            .sites
            .iter()
            .find(|site| site.id == StellarId(200))
            .expect("shown");
        assert_eq!(site.govt, Some(GovtId(150)));
    }

    #[test]
    fn docked_the_stellar_docked_at_stays_shown() {
        let catalog = twice_listed();
        let mut pilot = crate::pilot::Pilot::new(&catalog, "Ada").expect("starts");
        pilot.system = SystemId(131);
        pilot.stellar = Some(StellarId(200));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(land_now(&mut session), Ok(StellarId(200)));
        assert_eq!(
            session.system(),
            SystemId(130),
            "a load resumes in its stellar's owner, whatever system was saved"
        );
        assert_eq!(shown(&session), [128, 129, 200], "130's, docked at 200");
        session
            .chart
            .assert_docked_among_shown(session.mode.landed());
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_stellar_govt(StellarId(200), Some(None));
        });
        session
            .chart
            .assert_docked_among_shown(session.mode.landed());
        session
            .take_off(&catalog, &mut Scripted::default())
            .expect("takes off");
        assert_eq!(
            shown(&session),
            [128, 129, 200],
            "in flight, what 130 holds"
        );
    }

    #[test]
    fn a_load_shows_the_stellars_its_galaxy_refresh_gives() {
        let catalog = twice_listed();
        let mut session = quiet(&catalog);
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        let hiding_130 = crate::testkit::hiding_bit(130).get();
        run(&mut session, &format!("b{hiding_130}"));
        assert!(session.galaxy().visible(SystemId(130)), "no refresh yet");
        let session = Session::fly(&catalog, session.pilot().clone()).expect("flies");
        assert!(
            !session.galaxy().visible(SystemId(130)),
            "the load's refresh"
        );
        assert_eq!(
            shown(&session),
            [140, 200],
            "the load's rebuild follows its refresh (@0x751ca, @0x751cf)"
        );
    }

    /// [`catalog`] with 136 at 133's place, 133 its head, each
    /// [`hideable`](crate::testkit::hideable).
    fn placed() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog
            .star_map
            .push(crate::testkit::star(136, (-600.0, 0.0), &[]));
        crate::testkit::hideable(&mut catalog);
        catalog
    }

    #[test]
    fn a_swapped_in_system_is_shown_explored_from_the_next_rebuild_point() {
        let catalog = placed();
        for source in RuleSource::ALL {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(Rulebook::default().with_override(RuleKey::ExploredByPlace, source));
            session.change_galaxy(&catalog, |overlay| {
                overlay.set_visible(SystemId(136), Some(false));
            });
            session.pilot.explore(SystemId(133));
            session.change_galaxy(&catalog, |overlay| {
                overlay.set_visible(SystemId(133), Some(false));
                overlay.set_visible(SystemId(136), Some(true));
            });
            assert!(
                !session.pilot().has_explored(SystemId(136)),
                "a galaxy change is no rebuild point"
            );
            land_now(&mut session).expect("lands");
            assert_eq!(
                session.pilot().has_explored(SystemId(136)),
                source == RuleSource::Engine,
                "{source:?}: the landing's rebuild point, docked"
            );
        }
    }

    #[test]
    fn a_take_off_shares_the_place_at_its_rebuild_point() {
        let catalog = placed();
        for source in RuleSource::ALL {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(Rulebook::default().with_override(RuleKey::ExploredByPlace, source));
            land_now(&mut session).expect("lands");
            session.pilot.explore(SystemId(133));
            session
                .take_off(&catalog, &mut Scripted::default())
                .expect("takes off");
            assert_eq!(
                session.pilot().has_explored(SystemId(136)),
                source == RuleSource::Engine,
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_load_shares_the_place() {
        let catalog = placed();
        let mut pilot = crate::pilot::Pilot::new(&catalog, "").expect("starts");
        pilot.explore(SystemId(133));
        pilot.bits_mut().set(crate::testkit::hiding_bit(133));
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert!(!session.galaxy().visible(SystemId(133)), "the refresh");
        assert!(session.galaxy().visible(SystemId(136)));
        assert!(
            session.pilot().has_explored(SystemId(136)),
            "the load's rebuild shares, under the default reading"
        );
    }

    #[test]
    fn stellars_shown_gives_the_sites_ids_in_navigation_order() {
        let mut catalog = twice_listed();
        catalog.sites[0].1.reverse();
        let session = quiet(&catalog);
        assert_eq!(shown(&session), [200, 129, 128]);
    }

    // A stellar shown outside the system that holds it.

    /// [`twice_listed`] with 200's `Type` frame 60 wide: a reach of
    /// trunc(60 × 1.75) = 105 where it is held.
    fn twice_listed_60() -> FakePilotCatalog {
        let mut catalog = twice_listed();
        for (_, sites) in &mut catalog.sites {
            for site in sites.iter_mut().filter(|site| site.id == StellarId(200)) {
                site.frame_size = Some((60, 60));
            }
        }
        catalog
    }

    /// A session of `catalog` with `rules`, started in 130, which owns
    /// 200, with the player's system then set to `system` and 200 kept
    /// among the sites: the original's state for a slot between a galaxy
    /// refresh and the next rebuild point.
    fn kept_200(catalog: &FakePilotCatalog, rules: &Rulebook, system: i16) -> Session {
        let mut session = quiet(catalog).with_rules(*rules);
        session.pilot.system = SystemId(system);
        session
    }

    /// 200 as the session keeps it among the sites.
    fn site_200(session: &Session) -> LandingSite {
        *session
            .chart
            .sites
            .iter()
            .find(|site| site.id == StellarId(200))
            .expect("kept")
    }

    /// 200's landing reach in `session`, held where the session reads it.
    fn reach_of_200(session: &Session) -> crate::landing::LandingReach {
        let site = site_200(session);
        let held = session
            .chart
            .held_here(&session.pilot, &session.statics, site.id);
        session.space.landing_reach(&session.setup, &site, held)
    }

    /// The land key pressed in [`kept_200`], with 200 targeted and
    /// cleared and the ship at rest at `offset` from its centre: the
    /// stellar landed on, or the refusal, and the sounds the press raised.
    fn land_on_200(
        catalog: &FakePilotCatalog,
        system: i16,
        offset: Vec2,
    ) -> (Result<StellarId, LandingRefusal>, Vec<crate::SimSound>) {
        let mut session = kept_200(catalog, &engine(), system);
        session.ship.player = ShipState {
            position: site_200(&session).position + offset,
            ..ShipState::default()
        };
        session.chart.nav = crate::navigation::NavMode::Stellar(Some(StellarId(200)));
        session.port.dock = Some(crate::clearance::DockRequest {
            stellar: StellarId(200),
            permission: crate::clearance::CLEARED,
        });
        let _ = session.take_sounds();
        let landed = session.land().map(|press| match press {
            LandPress::Outcome(LandOutcome::Landed(stellar)) => stellar,
            other => panic!("not a landing: {other:?}"),
        });
        (landed, session.take_sounds())
    }

    #[test]
    fn a_stellar_two_visible_systems_list_reaches_by_its_frame_in_its_owner_and_150_elsewhere() {
        use crate::landing::LandingReach;
        let catalog = twice_listed_60();
        assert_eq!(
            land_on_200(&catalog, 130, Vec2::new(104.0, 0.0)).0,
            Ok(StellarId(200))
        );
        assert!(matches!(
            land_on_200(&catalog, 130, Vec2::new(105.0, 0.0)).0,
            Err(LandingRefusal::TooFar { .. })
        ));
        assert_eq!(
            reach_of_200(&kept_200(&catalog, &engine(), 130)),
            LandingReach::Box(105.0),
            "130 owns 200: its frame"
        );
        assert_eq!(
            reach_of_200(&kept_200(&catalog, &engine(), 131)),
            LandingReach::Box(262.0),
            "131 lists 200 but 130 owns it: trunc(150 × 1.75) (@0xacb2-0xacc1)"
        );
        for system in [130, 131] {
            let session = kept_200(&catalog, &other_reading(), system);
            assert_eq!(
                reach_of_200(&session),
                LandingReach::Circle(crate::landing::landing_radius(&site_200(&session))),
                "{system}: the other reading ignores the owner"
            );
        }
    }

    #[test]
    fn the_land_press_refuses_a_stellar_another_system_owns_as_not_landable_before_any_reach() {
        let catalog = twice_listed_60();
        let (refused, sounds) = land_on_200(&catalog, 131, Vec2::ZERO);
        assert_eq!(
            refused,
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(200),
                station: false,
            }),
            "@0x6699a-0x669a4, though cleared, over it and at rest"
        );
        assert_eq!(sounds, [crate::SimSound::Refused], "snd 153 @0x67e8f");
    }

    #[test]
    fn a_gate_another_system_holds_reads_150_for_its_entry_box() {
        use crate::traffic::gate_entry::gate_reach;
        let mut catalog = twice_listed();
        for (_, sites) in &mut catalog.sites {
            for site in sites.iter_mut().filter(|site| site.id == StellarId(200)) {
                site.frame_size = Some((80, 80));
                site.flags2 = crate::gate::HYPERGATE;
            }
        }
        let entry = |system| {
            let session = kept_200(&catalog, &engine(), system);
            let widths = session.gate_type_widths();
            let width = widths[&StellarId(200)];
            (width, gate_reach(&site_200(&session), &widths))
        };
        assert_eq!(entry(130), (TypeWidth::Frame, 20), "130 owns it: 80 / 4");
        assert_eq!(
            entry(131),
            (TypeWidth::Unloaded, 37),
            "130 owns it, not 131: 150 / 4 (@0xacb9, @0x8dbdf)"
        );
    }
}
