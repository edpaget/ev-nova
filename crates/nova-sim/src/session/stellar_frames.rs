//! Which frame each stellar shown draws, as the original's sprite tick
//! steps it (`_HandleStellarSprites`, rendering phase 11): every other
//! stellar's by the sequential branch
//! ([`stellar_frames`](crate::stellar_frames)), a hypergate's by its own
//! (see the `gate_sprites` module).
//!
//! **Where it runs.** `_DoPlayGameWork` calls `_HandleStellarSprites` at
//! @0x444d2, after the ship loop and the +0x47 pass, before
//! `_HandleStellarWeapons` (@0x444d7), `_HandleAsteroids` (@0x444f5),
//! `_HandleGravity` (@0x44509) and `_HandleDeadlyStellars` (@0x4450e), on
//! both passes: the second pass's skip (`cmpb $0x0,-0x19(%ebp); je
//! 0x44350`, @0x44026-0x4402a) jumps to before it. So
//! [`Session::stellar_sprite_tick`] runs there in every pass of a frame,
//! double time's second included, and the deadly-stellar touch reads the
//! frame it left (see the `deadly_masks` module). Game entry's two
//! priming passes run it too, each after its ship loop (see the
//! `priming_passes` module). Landed, the pass runs its landed work
//! instead, and nothing steps. Each `_PrepareAllSprites` and
//! game entry runs the same walk at its own gain (see the
//! `stellar_passes` module).
//!
//! **In order** ([`Session::stellar_sprite_tick`]): the slot's clone and
//! placement, `_SWCloneFramesFromMaster` (@0x2e7be-0x2e7e5) and
//! @0x2ee35-0x2ef1a (`Session::see_stellars`), so the freeze a clone
//! brings is known before the step, as the refused
//! `_SWSetCurrentFrameIndex` keeps a frozen slot's frame (@0x2ee20-0x2ee30,
//! @0xa90e7-0xa90ee); then a record kept for each stellar shown; then
//! each slot's look settled ([`Space::settle_stellar_looks`]); then every
//! other stellar's frames stepped over its counted sheet's frame count,
//! shown on its drawn sheet's, random picks drawn on the session's
//! `Chance`; then the gates (see the `gate_sprites` module). The original
//! clones and steps stellar by stellar inside one loop; the engine clones
//! every stellar, then settles, then steps, all at @0x444d2 in the same
//! pass. Nothing reads in between, so no boundary is crossed.
//!
//! **Kept across systems.** The original keeps a stellar's frame, next
//! frame and timer on its `spöb` record (+0x476, +0x478, +0x490), not its
//! sprite slot, and the sprite tick walks only the slots shown
//! (@0x2e713-0x2e73e), so a stellar not shown stands still
//! (stellar-fidelity phase 12). The session keeps a [`KeptStellar`] for
//! every stellar the flight has shown, with the look its slot had when it
//! was last settled, `None` while it is not shown. A stellar shown again
//! shows its current frame modulo the sheet it now draws, as the rebuild
//! sets it (`_HandleStellarSystemVisibility` @0x33096), or frame 0 on
//! graphic type 0's (@0x332d0). One the flight has never shown starts as
//! the launch leaves it ([`StellarFrames::new`], `_InitObjects`
//! @0x1c8c5-0x1c8d1), or as a take-off's reset leaves it once one has run
//! ([`StellarFrames::after_reset`]; see the `takeoff_frames` module).
//! `Space` is not saved, so a load, a resumed pilot and a new pilot start
//! every stellar fresh, as they start every gate closed.
//!
//! **The looks** ([`Space::settle_stellar_looks`]) port what the view did
//! as the session told it of each change: a stellar first shown, shown
//! again, or whose look changed at a rebuild point takes
//! [`StellarFrames::reshow`]; one whose sheet drawn changed otherwise
//! takes [`StellarFrames::swap`] (`_SWCloneFramesFromMaster`
//! @0xac504-0xac532); a hypergate whose slot freezes holds the frame its
//! gate branch showed before this pass's step ([`StellarFrames::hold`]).
//!
//! **The frame reported** ([`Session::stellar_frame`]) is the frame of
//! the drawn sheet its slot shows: a hypergate's whose slot is not frozen
//! is its gate sprite's, modulo the drawn sheet's frame count; every
//! other's is its [`StellarFrames::shown`], or 0 when that is out of the
//! drawn sheet's range (a sheet swapped since the last settle, as
//! [`StellarFrames::swap`] would show it). The flight frame carries it to
//! the view (`FrameStellar::frame`), and the deadly-stellar touch reads
//! it.
//!
//! The sprite tick plays no sound (see the [`gate_sprite`](crate::gate_sprite)
//! module docs).

use std::num::NonZeroU16;

use super::stellar_passes::GateHolds;
use super::{Session, Setup, Space, Statics};
use crate::SpeedMult;
use crate::catalog::{LandingSite, StellarId};
use crate::chance::Chance;
use crate::gate::HYPERGATE;
use crate::gate_sprite::GateAnimation;
use crate::rulebook::{RuleKey, RuleSource};
use crate::stellar_frames::{StellarAnimation, StellarFrames};
use crate::story::DrawnSheet;

/// A stellar's frames as the flight keeps them, shown or not, and how its
/// slot looked when they were last settled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct KeptStellar {
    /// Its animation state.
    pub(super) frames: StellarFrames,
    /// How its slot looked at the last settle; `None` while it is not
    /// shown, or before it has been settled.
    pub(super) look: Option<SlotLook>,
}

impl KeptStellar {
    /// A stellar first kept, with `frames`, not settled yet.
    fn new(frames: StellarFrames) -> Self {
        Self { frames, look: None }
    }
}

/// How a stellar's sprite slot looks: what its frames are stepped and
/// shown by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // independent facts of a slot
pub(super) struct SlotLook {
    /// Whether it is destroyed, which the animation reads.
    pub(super) destroyed: bool,
    /// The sheet it draws (`Session::drawn_sheet`).
    pub(super) drawn: DrawnSheet,
    /// The sheet whose frame count it animates over
    /// (`Session::counted_sheet`).
    pub(super) counted: DrawnSheet,
    /// Whether its slot shows a frame frozen (`Session::frame_frozen`).
    pub(super) frozen: bool,
    /// Whether it animates over no frames (`Session::counts_no_frames`).
    pub(super) counts_none: bool,
}

impl Session {
    /// The sprite tick at @0x444d2 (see the module docs), on every pass in
    /// flight: the slots cloned and placed, then every stellar shown
    /// stepped at the pass's gain, the gates held by the ships and the
    /// player's dock request as they stand, random picks drawn on
    /// `chance`.
    pub(super) fn stellar_sprite_tick(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.see_stellars();
        self.step_stellars_by(self.time.gain(), GateHolds::AsTheyStand, chance);
    }

    /// One pass of every stellar shown at `gain`, gates held as `holds`
    /// says, random picks drawn on `chance`: a record kept for each, its
    /// look settled, every other stellar's frames stepped, then the gates
    /// (see the module docs). The walk the sprite tick and each stellar
    /// pass share.
    pub(super) fn step_stellars_by(
        &mut self,
        gain: SpeedMult,
        holds: GateHolds,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.keep_stellar_frames();
        self.space
            .settle_stellar_looks(&self.statics, &self.setup, &self.chart.sites, false);
        self.space.step_stellar_frames(
            &self.statics,
            &self.setup,
            &self.chart.sites,
            gain,
            &mut &mut *chance,
        );
        self.step_gates_by(gain, holds, chance);
    }

    /// Keeps a record for each stellar shown, and a gate sprite for each
    /// hypergate shown ([`Space::keep_stellar_frames`]).
    pub(super) fn keep_stellar_frames(&mut self) {
        self.space.keep_stellar_frames(&self.chart.sites);
    }

    /// A rebuild point's settle (see the module docs): each stellar shown
    /// whose look changed shows its current frame on the sheet now drawn.
    pub(super) fn settle_rebuilt_stellars(&mut self) {
        self.keep_stellar_frames();
        self.space
            .settle_stellar_looks(&self.statics, &self.setup, &self.chart.sites, true);
    }

    /// The frame of the sheet drawn that `stellar`'s slot shows (see the
    /// module docs); 0 for a stellar not shown.
    #[must_use]
    pub(crate) fn stellar_frame(&self, stellar: StellarId) -> u16 {
        self.chart
            .sites
            .iter()
            .find(|site| site.id == stellar)
            .map_or(0, |site| {
                self.space.stellar_frame(&self.statics, &self.setup, site)
            })
    }
}

impl Space {
    /// Gives each of `sites`, the stellars shown, a record when the flight
    /// has none for it yet (as the launch or the take-off's reset leaves
    /// it, `stellar_frames_afresh`), and each hypergate among them a
    /// gate sprite likewise (see the `gate_sprites` module). A stellar no
    /// longer shown keeps its frames, its look forgotten, so that it is
    /// shown again on its current frame.
    pub(super) fn keep_stellar_frames(&mut self, sites: &[LandingSite]) {
        for (id, kept) in &mut self.stellar_frames {
            if !sites.iter().any(|site| site.id == *id) {
                kept.look = None;
            }
        }
        let afresh = self.stellar_frames_afresh;
        for site in sites {
            self.stellar_frames
                .entry(site.id)
                .or_insert_with(|| KeptStellar::new(afresh));
        }
        self.keep_gate_sprites(
            sites
                .iter()
                .filter(|site| site.flags2 & HYPERGATE != 0)
                .map(|site| site.id),
        );
    }

    /// How `site`'s slot looks now (see [`SlotLook`]).
    pub(super) fn slot_look(
        &self,
        statics: &Statics,
        setup: &Setup,
        site: &LandingSite,
    ) -> SlotLook {
        SlotLook {
            destroyed: site.destroyed,
            drawn: self.site_sheet(statics, setup, site),
            counted: self.counted_sheet_by_rules(statics, setup, site),
            frozen: self.frame_frozen(setup, site.id),
            counts_none: self.counts_no_frames(setup, site.id),
        }
    }

    /// Settles each of `sites`' frames against its slot's look now, at a
    /// rebuild point when `at_rebuild` (see the module docs). Each must
    /// be kept ([`Space::keep_stellar_frames`]).
    pub(super) fn settle_stellar_looks(
        &mut self,
        statics: &Statics,
        setup: &Setup,
        sites: &[LandingSite],
        at_rebuild: bool,
    ) {
        for site in sites {
            let now = self.slot_look(statics, setup, site);
            let drawn = sheet_frames(statics, now.drawn, site);
            let gate_shown = self.gate_shown(site);
            let Some(kept) = self.stellar_frames.get_mut(&site.id) else {
                continue;
            };
            let reshown = match kept.look {
                None => true,
                Some(was) => at_rebuild && was != now,
            };
            if reshown {
                kept.frames.reshow(drawn, now.drawn == DrawnSheet::TypeZero);
            } else if let Some(was) = kept.look {
                if was.drawn != now.drawn {
                    kept.frames.swap(drawn);
                }
                if let Some(frame) = gate_shown
                    && now.frozen
                    && !was.frozen
                {
                    kept.frames.hold(frame % drawn.get());
                }
            }
            kept.look = Some(now);
        }
    }

    /// One pass at `gain` of every stellar of `sites` but the hypergates,
    /// over the frames its look counts, shown on the sheet it draws; a
    /// frozen slot's frame stays. Random picks are drawn on `chance`.
    fn step_stellar_frames(
        &mut self,
        statics: &Statics,
        setup: &Setup,
        sites: &[LandingSite],
        gain: SpeedMult,
        chance: &mut dyn Chance,
    ) {
        for site in sites.iter().filter(|site| site.flags2 & HYPERGATE == 0) {
            let look = self.slot_look(statics, setup, site);
            let Some(kept) = self.stellar_frames.get_mut(&site.id) else {
                continue;
            };
            let anim = StellarAnimation::of(site);
            let counted = if look.counts_none {
                NonZeroU16::MIN
            } else {
                sheet_frames(statics, look.counted, site)
            };
            if look.frozen {
                kept.frames
                    .step_frozen_by(gain, anim, look.destroyed, counted, chance);
            } else {
                let drawn = sheet_frames(statics, look.drawn, site);
                kept.frames
                    .step_by(gain, anim, look.destroyed, counted, drawn, chance);
            }
        }
    }

    /// [`Session::stellar_frame`] for `site`, one of the stellars shown.
    pub(super) fn stellar_frame(
        &self,
        statics: &Statics,
        setup: &Setup,
        site: &LandingSite,
    ) -> u16 {
        let drawn = sheet_frames(statics, self.site_sheet(statics, setup, site), site).get();
        if !self.frame_frozen(setup, site.id)
            && let Some(frame) = self.gate_shown(site)
        {
            return frame % drawn;
        }
        self.stellar_frames
            .get(&site.id)
            .map(|kept| kept.frames.shown())
            .filter(|&frame| frame < drawn)
            .unwrap_or(0)
    }

    /// The frame `site`'s gate branch shows, for a hypergate (see the
    /// `gate_sprites` module); `None` for any other stellar.
    fn gate_shown(&self, site: &LandingSite) -> Option<u16> {
        (site.flags2 & HYPERGATE != 0).then(|| {
            self.gate_sprites.get(&site.id).map_or(0, |sprite| {
                sprite.shown(&GateAnimation::of(site), site.destroyed)
            })
        })
    }

    /// `Session::counted_sheet` for `site`, one of the stellars shown,
    /// by [`RuleKey::StellarFrameCount`]: by the Bible the sheet it draws,
    /// by the engine the one its slot held at the last rebuild point.
    pub(super) fn counted_sheet_by_rules(
        &self,
        statics: &Statics,
        setup: &Setup,
        site: &LandingSite,
    ) -> DrawnSheet {
        if setup.rules.source_for(RuleKey::StellarFrameCount) == RuleSource::Bible {
            self.site_sheet(statics, setup, site)
        } else {
            self.counted_sheet(setup, site)
        }
    }
}

/// How many frames `sheet` of `site` has, as the catalog counts its
/// master (`SiteAnimation::type_sheet_frames`): its `DeadType` sheet's, or
/// its `Type`'s when it has none; graphic type 0's for
/// [`DrawnSheet::TypeZero`]; 1 for a slot showing no frame, or a sheet
/// that counts none or does not read.
pub(super) fn sheet_frames(statics: &Statics, sheet: DrawnSheet, site: &LandingSite) -> NonZeroU16 {
    let count = match sheet {
        DrawnSheet::Dead if site.dead_graphic.is_some() => site.animation.dead_sheet_frames,
        DrawnSheet::Type | DrawnSheet::Dead => site.animation.type_sheet_frames,
        DrawnSheet::TypeZero => statics.fallback_frames,
        DrawnSheet::NoFrame => 1,
    };
    NonZeroU16::new(count).unwrap_or(NonZeroU16::MIN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SiteAnimation;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::control::SetExpr;
    use crate::flight::Controls;
    use crate::gate_sprite::{ONLY_WHEN_DESTROYED, RANDOM};
    use crate::testkit::{
        FakePilotCatalog, Passive, Scripted, catalog, land_now, life, planet, work, work_with,
    };

    /// The animated stellar.
    const ANIMATED: StellarId = StellarId(140);

    /// Draws the first outcome for every draw, fires nothing: unlike the
    /// sources the session's tests otherwise run on, so a frame drawn on
    /// another source shows.
    struct First;

    impl Chance for First {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, _n: u32) -> u32 {
            0
        }
    }

    /// [`catalog`] with system 130 holding planet 128 and stellar 140 at
    /// (300, 0), of `Flags2` `flags2`, stepping every `delay` passes over
    /// its `Type` sheet of `frames` frames, with a `DeadType` sheet
    /// (graphic 11) of `dead` frames, and a `DeadTime` of 2.
    fn animated(flags2: u16, delay: i16, frames: u16, dead: u16) -> FakePilotCatalog {
        let mut catalog = catalog().living([life(140, 2)]);
        catalog.sites[0].1 = vec![
            planet(128, 0.0, 0.0),
            LandingSite {
                flags2,
                dead_graphic: Some(11),
                dead_frame_size: Some((100, 100)),
                animation: SiteAnimation {
                    delay,
                    type_frames: frames,
                    dead_frames: dead,
                    type_sheet_frames: frames,
                    dead_sheet_frames: dead,
                    ..SiteAnimation::default()
                },
                ..planet(140, 300.0, 0.0)
            },
        ];
        catalog
    }

    /// Stellar 140's animation fields, as `session` shows it.
    fn anim(session: &Session) -> StellarAnimation {
        StellarAnimation::of(
            session
                .chart
                .sites
                .iter()
                .find(|site| site.id == ANIMATED)
                .expect("shown"),
        )
    }

    fn count(n: u16) -> NonZeroU16 {
        NonZeroU16::new(n).expect("non-zero")
    }

    fn run(session: &mut Session, text: &str) {
        session.run_set(
            &SetExpr::parse(text).expect("parses"),
            &mut Scripted::default(),
        );
    }

    /// A pass drawing on `chance`.
    fn pass_on(session: &mut Session, catalog: &FakePilotCatalog, chance: &mut dyn Chance) {
        work_with(
            session,
            catalog,
            Controls::default(),
            &Passive,
            Rules::default(),
            chance,
        );
    }

    #[test]
    fn a_shown_stellar_is_stepped_once_per_pass_on_the_sessions_chance() {
        let catalog = animated(RANDOM, 1, 5, 0);
        let mut session = Session::start(&catalog).expect("starts");
        let anim = anim(&session);
        let (mut oracle, mut elsewhere) = (StellarFrames::new(), StellarFrames::new());
        let mut drawn_elsewhere = false;
        for pass in 0..8 {
            pass_on(&mut session, &catalog, &mut First);
            oracle.step(anim, false, count(5), count(5), &mut First);
            elsewhere.step(anim, false, count(5), count(5), &mut NeverFires);
            assert_eq!(session.stellar_frame(ANIMATED), oracle.shown(), "{pass}");
            drawn_elsewhere |= elsewhere.shown() != oracle.shown();
        }
        assert!(drawn_elsewhere, "another source would show otherwise");
    }

    #[test]
    fn double_times_second_pass_steps_every_stellar_again() {
        let catalog = animated(0, 1, 4, 0);
        let mut session = Session::start(&catalog).expect("starts");
        let anim = anim(&session);
        let mut oracle = StellarFrames::new();
        for pass in 0..3 {
            work(&mut session, &catalog, Controls::default());
            oracle.step(anim, false, count(4), count(4), &mut NeverFires);
            assert_eq!(session.stellar_frame(ANIMATED), oracle.shown(), "{pass}");
            session.tick_again(
                Controls::default(),
                &Passive,
                Rules::default(),
                &mut NeverFires,
            );
            oracle.step(anim, false, count(4), count(4), &mut NeverFires);
            assert_eq!(
                session.stellar_frame(ANIMATED),
                oracle.shown(),
                "{pass} again"
            );
        }
    }

    #[test]
    fn nothing_steps_while_landed() {
        let catalog = animated(0, 1, 4, 0);
        let mut session = Session::start(&catalog).expect("starts");
        work(&mut session, &catalog, Controls::default());
        land_now(&mut session).expect("lands");
        let landed = session.space.stellar_frames[&ANIMATED];
        for _ in 0..5 {
            work(&mut session, &catalog, Controls::default());
        }
        assert_eq!(session.space.stellar_frames[&ANIMATED], landed);
    }

    #[test]
    fn a_stellar_keeps_its_frame_across_systems() {
        let catalog = animated(0, 2, 4, 0);
        let mut session = Session::start(&catalog).expect("starts");
        let anim = anim(&session);
        let mut oracle = StellarFrames::new();
        for _ in 0..5 {
            work(&mut session, &catalog, Controls::default());
            oracle.step(anim, false, count(4), count(4), &mut NeverFires);
        }
        let shown = session.chart.sites.clone();
        session.chart.sites.retain(|site| site.id != ANIMATED);
        for _ in 0..7 {
            work(&mut session, &catalog, Controls::default());
        }
        assert_eq!(session.stellar_frame(ANIMATED), 0, "not shown");
        session.chart.sites = shown;
        oracle.reshow(count(4), false);
        for pass in 0..3 {
            work(&mut session, &catalog, Controls::default());
            oracle.step(anim, false, count(4), count(4), &mut NeverFires);
            assert_eq!(session.stellar_frame(ANIMATED), oracle.shown(), "{pass}");
        }
    }

    /// Stellar 140, of `dead` `DeadType` frames, after 3 passes: frame 3
    /// of its 4, then destroyed; and the frames it stood on.
    fn destroyed_on_frame_3(catalog: &FakePilotCatalog) -> (Session, StellarFrames) {
        let mut session = Session::start(catalog).expect("starts");
        let anim = anim(&session);
        let mut oracle = StellarFrames::new();
        for _ in 0..3 {
            work(&mut session, catalog, Controls::default());
            oracle.step(anim, false, count(4), count(4), &mut NeverFires);
        }
        assert_eq!(oracle.shown(), 3);
        run(&mut session, "Y140");
        (session, oracle)
    }

    #[test]
    fn a_rebuild_shows_the_current_frame_on_the_sheet_now_drawn() {
        for dead in [2, 3] {
            let catalog = animated(0, 1, 4, dead);
            let (mut session, mut oracle) = destroyed_on_frame_3(&catalog);
            session.rebuild_stellars();
            oracle.reshow(count(dead), false);
            assert_eq!(session.stellar_frame(ANIMATED), oracle.shown(), "{dead}");
        }
    }

    #[test]
    fn a_sheet_swapped_between_rebuilds_keeps_the_frame_shown_in_range() {
        for dead in [2, 4] {
            let catalog = animated(0, 1, 4, dead);
            let (mut session, mut oracle) = destroyed_on_frame_3(&catalog);
            session.keep_stellar_frames();
            let space = &mut session.space;
            space.settle_stellar_looks(
                &session.statics,
                &session.setup,
                &session.chart.sites,
                false,
            );
            oracle.swap(count(dead));
            assert_eq!(
                session.space.stellar_frames[&ANIMATED].frames, oracle,
                "{dead}"
            );
            assert_eq!(session.stellar_frame(ANIMATED), oracle.shown(), "{dead}");
        }
    }

    #[test]
    fn a_frozen_slot_keeps_the_frame_from_before_the_pass_that_froze_it() {
        // Animating only while destroyed, its `Type` counting no frames:
        // `U` after its `DeadType` frames ran freezes the slot on one.
        let mut catalog = animated(ONLY_WHEN_DESTROYED, 1, 4, 4);
        catalog.sites[0].1[1].type_counts_frames = false;
        let mut session = Session::start(&catalog).expect("starts");
        let anim = anim(&session);
        run(&mut session, "Y140");
        session.rebuild_stellars();
        let mut oracle = StellarFrames::new();
        for _ in 0..2 {
            work(&mut session, &catalog, Controls::default());
            oracle.step(anim, true, count(4), count(4), &mut NeverFires);
        }
        assert_eq!(session.stellar_frame(ANIMATED), oracle.shown());
        assert_ne!(oracle.shown(), 0);
        run(&mut session, "U140");
        work(&mut session, &catalog, Controls::default());
        assert!(session.frame_frozen(ANIMATED));
        oracle.step_frozen(anim, false, count(4), &mut NeverFires);
        assert_eq!(session.stellar_frame(ANIMATED), oracle.shown());
        assert_eq!(session.space.stellar_frames[&ANIMATED].frames, oracle);
    }

    /// Stellar 900, not shown, kept as 5 passes of `AnimDelay` 2 over 4
    /// frames leave it.
    const KEPT: StellarId = StellarId(900);

    fn stepped() -> StellarFrames {
        let anim = StellarAnimation {
            delay: 2,
            ..StellarAnimation::default()
        };
        let mut frames = StellarFrames::new();
        for _ in 0..5 {
            frames.step(anim, false, count(4), count(4), &mut NeverFires);
        }
        frames
    }

    #[test]
    fn the_take_off_reset_reaches_every_kept_stellar() {
        let catalog = animated(0, 2, 4, 0);
        let mut session = Session::start(&catalog).expect("starts");
        let anim = anim(&session);
        for _ in 0..3 {
            work(&mut session, &catalog, Controls::default());
        }
        land_now(&mut session).expect("lands");
        session
            .space
            .stellar_frames
            .insert(KEPT, KeptStellar::new(stepped()));
        let mut shown = session.space.stellar_frames[&ANIMATED].frames;
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        let mut kept = stepped();
        kept.take_off_reset();
        assert_eq!(session.space.stellar_frames[&KEPT].frames, kept, "kept");
        // Shown: the reset, then the take-off's two passes at gain 0.
        shown.take_off_reset();
        for _ in 0..2 {
            shown.step_by(
                SpeedMult::ZERO,
                anim,
                false,
                count(4),
                count(4),
                &mut NeverFires,
            );
        }
        assert_eq!(
            session.space.stellar_frames[&ANIMATED].frames, shown,
            "shown"
        );
        assert_eq!(
            session.space.stellar_frames_afresh,
            StellarFrames::after_reset()
        );
    }

    #[test]
    fn a_stellar_first_shown_after_a_reset_starts_as_the_reset_leaves_it() {
        let site = planet(141, 0.0, 0.0);
        let mut space = Space::new();
        space.keep_stellar_frames(&[site]);
        assert_eq!(space.stellar_frames[&site.id].frames, StellarFrames::new());
        space.reset_stellar_frames(None);
        let later = planet(142, 0.0, 0.0);
        space.keep_stellar_frames(&[site, later]);
        assert_eq!(
            space.stellar_frames[&later.id].frames,
            StellarFrames::after_reset()
        );
    }

    #[test]
    fn the_frame_reports_each_stellars_shown_frame() {
        use crate::clearance::{CLEARED, DockRequest};
        use crate::session::frame::tests::filled;
        // A hypergate of 6 frames on a sheet of 4, held open.
        let mut catalog = animated(0, 1, 4, 0);
        catalog.sites[0].1.push(LandingSite {
            flags2: HYPERGATE,
            animation: SiteAnimation {
                delay: 1,
                type_frames: 6,
                type_sheet_frames: 4,
                ..SiteAnimation::default()
            },
            ..planet(300, -300.0, 0.0)
        });
        let gate = StellarId(300);
        let mut session = Session::start(&catalog).expect("starts");
        session.port.dock = Some(DockRequest {
            stellar: gate,
            permission: CLEARED,
        });
        for _ in 0..5 {
            work(&mut session, &catalog, Controls::default());
        }
        let frame = filled(&session);
        let reported = |id| frame.stellar(id).expect("listed").frame;
        assert_eq!(reported(ANIMATED), session.stellar_frame(ANIMATED));
        assert_ne!(reported(ANIMATED), 0);
        let gate_frame = session.gate_frame(gate).expect("a gate");
        assert!(gate_frame >= 4, "past the sheet's end: {gate_frame}");
        assert_eq!(reported(gate), gate_frame % 4);
        // Listed, not shown: frame 0.
        session.chart.sites.retain(|site| site.id != ANIMATED);
        let frame = filled(&session);
        assert_eq!(
            frame.stellar(ANIMATED).map(|stellar| stellar.frame),
            Some(0)
        );
    }

    #[test]
    fn a_frame_out_of_the_sheet_drawn_reports_frame_0_until_it_settles() {
        // Frame 3 of a `DeadType` sheet of 3: out of range, as a swap
        // would show it.
        let catalog = animated(0, 1, 4, 3);
        let (session, oracle) = destroyed_on_frame_3(&catalog);
        assert_eq!(oracle.shown(), 3);
        assert_eq!(session.stellar_frame(ANIMATED), 0);
    }

    #[test]
    fn a_stellar_no_longer_shown_forgets_its_look_and_one_still_shown_keeps_it() {
        let catalog = animated(0, 1, 4, 0);
        let mut session = Session::start(&catalog).expect("starts");
        work(&mut session, &catalog, Controls::default());
        let look = |session: &Session, id: i16| session.space.stellar_frames[&StellarId(id)].look;
        assert!(look(&session, 140).is_some() && look(&session, 128).is_some());
        let sites: Vec<LandingSite> = session
            .chart
            .sites
            .iter()
            .filter(|site| site.id != ANIMATED)
            .copied()
            .collect();
        session.space.keep_stellar_frames(&sites);
        assert_eq!(look(&session, 140), None, "not shown");
        assert!(look(&session, 128).is_some(), "still shown");
    }

    #[test]
    fn a_dead_sheet_counts_the_type_sheets_frames_when_the_stellar_has_none() {
        let site = LandingSite {
            animation: SiteAnimation {
                type_sheet_frames: 4,
                dead_sheet_frames: 0,
                ..SiteAnimation::default()
            },
            ..planet(141, 0.0, 0.0)
        };
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            sheet_frames(&session.statics, DrawnSheet::Dead, &site),
            count(4)
        );
        let with_dead = LandingSite {
            dead_graphic: Some(11),
            ..site
        };
        assert_eq!(
            sheet_frames(&session.statics, DrawnSheet::Dead, &with_dead),
            NonZeroU16::MIN
        );
    }

    #[test]
    fn a_slot_counting_no_frames_stays_on_frame_0_until_a_rebuild_point_that_counts() {
        // Animating only while destroyed, its `Type` counting no frames:
        // the start, a rebuild point, copies a count of none. Destroyed,
        // a frame clones in its `DeadType` of 4, which counts, and it
        // draws that, but the count copied stays until the next point.
        let mut catalog = animated(ONLY_WHEN_DESTROYED, 1, 4, 4);
        catalog.sites[0].1[1].type_counts_frames = false;
        let mut session = Session::start(&catalog).expect("starts");
        let anim = anim(&session);
        run(&mut session, "Y140");
        let mut oracle = StellarFrames::new();
        for pass in 0..5 {
            work(&mut session, &catalog, Controls::default());
            assert!(session.counts_no_frames(ANIMATED), "{pass}");
            assert_eq!(session.drawn_sheet(ANIMATED), DrawnSheet::Dead);
            oracle.step(anim, true, NonZeroU16::MIN, count(4), &mut NeverFires);
            assert_eq!(session.stellar_frame(ANIMATED), oracle.shown(), "{pass}");
        }
        assert_eq!(oracle.shown(), 0, "never animated");
        session.rebuild_stellars();
        assert!(!session.counts_no_frames(ANIMATED), "its `DeadType` counts");
        oracle.reshow(count(4), false);
        let mut animated_again = false;
        for pass in 0..3 {
            work(&mut session, &catalog, Controls::default());
            oracle.step(anim, true, count(4), count(4), &mut NeverFires);
            assert_eq!(
                session.stellar_frame(ANIMATED),
                oracle.shown(),
                "{pass} rebuilt"
            );
            animated_again |= oracle.shown() != 0;
        }
        assert!(animated_again);
    }

    #[test]
    fn a_destroyed_stellar_drawing_graphic_type_0_is_reshown_on_frame_0_and_shown_over_its_frames()
    {
        // Animating only while destroyed over a `Type` of 5, with no
        // `DeadType` sheet: a rebuild point while destroyed fills its slot
        // with graphic type 0, of 3 frames here, the count the catalog
        // reports and the start reads.
        let mut catalog = animated(ONLY_WHEN_DESTROYED, 1, 5, 0);
        catalog.fallback_frames = 3;
        let site = &mut catalog.sites[0].1[1];
        site.dead_graphic = None;
        site.dead_frame_size = None;
        let mut session = Session::start(&catalog).expect("starts");
        let anim = anim(&session);
        run(&mut session, "Y140");
        let mut oracle = StellarFrames::new();
        for _ in 0..2 {
            work(&mut session, &catalog, Controls::default());
            oracle.step(anim, true, count(5), count(5), &mut NeverFires);
        }
        assert_eq!(session.stellar_frame(ANIMATED), oracle.shown());
        assert_eq!(oracle.shown() % 3, 2, "off frame 0 of graphic type 0's");
        let three = count(3);
        session.rebuild_stellars();
        assert_eq!(session.drawn_sheet(ANIMATED), DrawnSheet::TypeZero);
        oracle.reshow(three, true);
        assert_eq!(session.stellar_frame(ANIMATED), oracle.shown(), "reshown");
        assert_eq!(oracle.shown(), 0);
        // Counting its `Type`, the sheet its slot held at the rebuild
        // point, shown on graphic type 0's.
        let mut seen = Vec::new();
        for pass in 0..6 {
            work(&mut session, &catalog, Controls::default());
            oracle.step(anim, true, count(5), three, &mut NeverFires);
            assert_eq!(session.stellar_frame(ANIMATED), oracle.shown(), "{pass}");
            seen.push(oracle.shown());
        }
        assert!(seen.contains(&2), "over 3 frames: {seen:?}");
        // From the next rebuild point it counts graphic type 0's.
        session.rebuild_stellars();
        oracle.reshow(three, true);
        assert_eq!(session.stellar_frame(ANIMATED), oracle.shown(), "again");
        for pass in 0..4 {
            work(&mut session, &catalog, Controls::default());
            oracle.step(anim, true, three, three, &mut NeverFires);
            assert_eq!(
                session.stellar_frame(ANIMATED),
                oracle.shown(),
                "{pass} counting graphic type 0's"
            );
        }
    }

    #[test]
    fn a_stellar_steps_over_the_sheet_the_frame_count_rule_counts_and_wraps_on_the_one_drawn() {
        // Animating only while destroyed, a `Type` of 5 and a `DeadType`
        // of 2: destroyed with no rebuild point since, its slot held its
        // `Type`, and it draws its `DeadType`.
        let catalog = animated(ONLY_WHEN_DESTROYED, 1, 5, 2);
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let rules = crate::rulebook::Rulebook::default()
                .with_override(RuleKey::StellarFrameCount, reading);
            let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
            let anim = anim(&session);
            run(&mut session, "Y140");
            // By the engine its slot's `Type` at the last rebuild point,
            // by the Bible the `DeadType` it draws.
            let (counted, other_count) = if reading == RuleSource::Engine {
                (count(5), count(2))
            } else {
                (count(2), count(5))
            };
            let (mut oracle, mut other) = (StellarFrames::new(), StellarFrames::new());
            let mut stepped_alike = true;
            for pass in 0..8 {
                work(&mut session, &catalog, Controls::default());
                oracle.step(anim, true, counted, count(2), &mut NeverFires);
                other.step(anim, true, other_count, count(2), &mut NeverFires);
                assert_eq!(
                    session.stellar_frame(ANIMATED),
                    oracle.shown(),
                    "{reading:?} {pass}"
                );
                stepped_alike &= other.shown() == oracle.shown();
            }
            assert!(
                !stepped_alike,
                "{reading:?}: the other count shows otherwise"
            );
        }
    }

    #[test]
    fn a_stellar_not_shown_reports_frame_0() {
        let catalog = animated(0, 1, 4, 0);
        let mut session = Session::start(&catalog).expect("starts");
        work(&mut session, &catalog, Controls::default());
        assert_ne!(session.stellar_frame(ANIMATED), 0);
        assert_eq!(session.stellar_frame(StellarId(999)), 0);
    }
}
