//! [`FlightView`]'s frame of steps over the [`FlightSim`] port: the input
//! taken from the keys held, preferences and window ([`StepInput`]), one
//! [`FlightSim::step`] a frame, then the view's own effects, flashes,
//! glows, shot looks and stars stepped from each pass the report holds,
//! in the order the view stepped them after each pass when it ran the
//! passes itself.
//!
//! **One call a frame, replayed after it.** The flight runs the frame's
//! steps in one call (`nova_sim::Session::step`), and the view then
//! replays its report pass by pass, but for a double-time frame's second
//! pass, held for the frame after with what is drawn (see the
//! `between_passes` module). Nothing the view does between passes
//! feeds back into the flight: its effects, glows, shot looks and stars
//! are its own, drawn on its own chance, so stepping them after the call
//! gives what stepping them between the passes gave. Before the first
//! step each ship's state is kept for the drawing to move from
//! ([`StepRun::before`]), and a rescue keeps the ship where it put it.
//!
//! **The comm and the quotes.** The flight took a step's comm at its
//! advance pass's end and each pass's quotes at the pass's end; the
//! inbox's own, drained earlier in the frame, are taken first, as the
//! view's first take of each emptied the inbox too, but only when that
//! take was reached: a step for the comm, a pass for the quotes. Both are
//! then relayed together after the steps
//! ([`FlightView::relay_comm_and_quotes`]), by kind, and the messages
//! after them; then the line the steps raised last, whatever its kind
//! ([`StepRun::last_line`], [`FlightView::relay_last_line`]), so the
//! line shows it, as `_DisplayComm` replaces its slot with each line.
//!
//! **The presentation rules** the effects, stars, world, HUD and wording
//! read ([`Presentation`]) are the view's own, held from the rulebook the
//! view is built with and as its rules change.
//!
//! **The player's wrap** (`_WrapPlayerPosition`, `nova_sim`'s
//! [`PlayerWrap`]). The view keeps some tables the routine moves, and the
//! places each step began from, so it carries them as it reaches the pass
//! that wrapped ([`PassReport::wrap`]), before that pass's effects:
//! [`StepRun::before`]'s places, each NPC's by the wrap's test, so the
//! drawing moves on from them; the explosions, pods and smoke; and the
//! stars are marked as moved, so none streaks across the 25,000. As the
//! scroll reads the ship's place after the carry, a wrap on the first pass
//! scrolls by the ship's own move, and one on double time's second leaves
//! the first's alone, as the original's second-pass write of
//! `_lastFramePlayerPos` (@0x6836b-0x68386) is overwritten unread.
//!
//! An NPC's place is tested as the step began where the session tests it
//! at the wrap. They can disagree only for one within a pass's flight of
//! 15,000 from the player on double time's second pass, whose drawn path
//! then runs 15,000 to 40,000 from the player, off screen, so the report
//! names no NPCs.
//!
//! **The frame after the steps.** Once the report is replayed the kept
//! frame is filled afresh ([`FlightView::refresh_frame`]), so the frame's
//! ships are where the steps left them, the places each step began from
//! ([`StepRun::before`]) being where the drawing moves them from. A
//! double-time frame is drawn from the frame its last step reported
//! between its passes ([`StepRun::between_passes`]) instead.

use nova_sim::flight_frame::{PassReport, StepInput, StepRun};
use nova_sim::player_wrap::PlayerWrap;
use nova_sim::{Controls, Face, PlayerCommand, RuleKey, RuleSource, Rulebook, ShipRef, Turn, Vec2};

use crate::Key;
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::effects::{LoopedSounds, Scene};
use crate::flight::sim_port::FlightSim;
use crate::system::camera::Anchor;
use crate::system::catalog::SystemCatalog;

use super::between_passes::{self, BetweenPasses};
use super::{
    AFTERBURNER_KEY, CLOAK_KEY, DESELECT_KEY, EJECT_KEY, FACE_TARGET_KEY, FIRE_KEY, FlightView,
    LAND_KEY, SECONDARY_KEY, SELF_DESTRUCT_KEY, dying, glow_decay, play_area, point, show_combat,
    show_frozen_frames, show_gate_frames, stars, window_width,
};

/// The rules the view shows the steps' effects, the stars, the world, the
/// HUD and its messages' numbers by, held from the rulebook the view is
/// built with (`FlightView::scripted`, `FlightView::with_rules` and the
/// concrete constructors): [`RuleKey::ShipDebris`],
/// [`RuleKey::CombatCaps`], [`RuleKey::LoopedSoundWait`],
/// [`RuleKey::FlightStarfield`], and the world's
/// [`RuleKey::TranslucentShotFade`], [`RuleKey::FlightAnchor`],
/// [`RuleKey::LightningZigZag`] and [`RuleKey::IonTintMurkColour`]
/// (rdm:phase/view-sim-seam/phase-7-world-frame, which pulled these four
/// forward from rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks),
/// and the HUD's [`RuleKey::HudCadence`]
/// (rdm:phase/view-sim-seam/phase-8-hud-and-target-frame, which pulled it
/// forward from phase 12 too), and the wording's [`RuleKey::PayMillions`]
/// (rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Presentation {
    /// [`RuleKey::ShipDebris`]'s reading.
    pub(super) debris: RuleSource,
    /// [`RuleKey::CombatCaps`]'s reading.
    pub(super) caps: RuleSource,
    /// [`RuleKey::LoopedSoundWait`]'s reading.
    pub(super) looped: RuleSource,
    /// [`RuleKey::FlightStarfield`]'s reading.
    pub(super) starfield: RuleSource,
    /// [`RuleKey::TranslucentShotFade`]'s reading.
    pub(super) shot_fade: RuleSource,
    /// [`RuleKey::FlightAnchor`]'s reading.
    pub(super) anchor: RuleSource,
    /// [`RuleKey::LightningZigZag`]'s reading.
    pub(super) zig_zag: RuleSource,
    /// [`RuleKey::IonTintMurkColour`]'s reading.
    pub(super) ion_tint_murk: RuleSource,
    /// [`RuleKey::HudCadence`]'s reading.
    pub(super) hud_cadence: RuleSource,
    /// [`RuleKey::PayMillions`]'s reading: how the messages write a
    /// number from a million.
    pub(super) millions: RuleSource,
}

impl Presentation {
    /// The ten as `rules` read them.
    pub(super) fn of(rules: &Rulebook) -> Self {
        Self {
            debris: rules.source_for(RuleKey::ShipDebris),
            caps: rules.source_for(RuleKey::CombatCaps),
            looped: rules.source_for(RuleKey::LoopedSoundWait),
            starfield: rules.source_for(RuleKey::FlightStarfield),
            shot_fade: rules.source_for(RuleKey::TranslucentShotFade),
            anchor: rules.source_for(RuleKey::FlightAnchor),
            zig_zag: rules.source_for(RuleKey::LightningZigZag),
            ion_tint_murk: rules.source_for(RuleKey::IonTintMurkColour),
            hud_cadence: rules.source_for(RuleKey::HudCadence),
            millions: rules.source_for(RuleKey::PayMillions),
        }
    }

    /// The scene a pass's effects play in: around the player's ship at
    /// `player`, as the rules read [`RuleKey::ShipDebris`],
    /// [`RuleKey::CombatCaps`] and [`RuleKey::LoopedSoundWait`].
    pub(super) fn scene(self, player: Vec2) -> Scene {
        Scene {
            player: point(player),
            debris: self.debris == RuleSource::Bible,
            caps: self.caps == RuleSource::Engine,
            looped: match self.looped {
                RuleSource::Engine => LoopedSounds::Anywhere,
                RuleSource::Bible => LoopedSounds::PerFirer,
            },
        }
    }
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// The controls for the keys held.
    pub(super) fn controls(&self) -> Controls {
        let holding = |key| self.held.contains(&key);
        let turn = match (holding(Key::Left), holding(Key::Right)) {
            (true, false) => Turn::Left,
            (false, true) => Turn::Right,
            _ => Turn::None,
        };
        let face = match (holding(FACE_TARGET_KEY), holding(Key::Alt)) {
            (false, _) => Face::Off,
            (true, false) => Face::Target,
            (true, true) => Face::Stellar,
        };
        Controls {
            thrust: holding(Key::Up),
            turn,
            reverse: holding(Key::Down),
            face,
            afterburner: holding(AFTERBURNER_KEY),
        }
    }

    /// What the frame's steps are given, the ship drawn at `anchor`: the
    /// controls for the keys held; the play area and the window's width;
    /// the primary and secondary fire keys, the eject chord, the
    /// self-destruct chord, the cloak key and the weapon safety key, each
    /// chord, the cloak and the safety only while the flight takes its
    /// command (the cloak key needs no Command, `_QuickKeyCheck`
    /// @0x705b7); double time and Hyperspace Effects as the screen was
    /// told; and the land key.
    pub(super) fn step_input(&self, anchor: Anchor) -> StepInput {
        let held = |key| self.held.contains(&key);
        let chord = |command, key| self.takes(command) && held(Key::Command) && held(key);
        StepInput {
            controls: self.controls(),
            play_area: play_area(self.view, anchor),
            window_width: window_width(self.view),
            fire: (held(FIRE_KEY), held(SECONDARY_KEY)),
            eject: chord(PlayerCommand::Eject, EJECT_KEY),
            self_destruct: chord(PlayerCommand::SelfDestruct, SELF_DESTRUCT_KEY),
            cloak: self.takes(PlayerCommand::Cloak) && held(CLOAK_KEY),
            weapon_safety: self.takes(PlayerCommand::DeselectWeapon) && held(DESELECT_KEY),
            double_time_key: self.double_time,
            hyperspace_effects: self.hyperspace_effects,
            land_key: held(LAND_KEY),
        }
    }

    /// Runs the frame's `steps` through the flight, the ship drawn at
    /// `anchor` (see the module docs), steps the view's effects after each
    /// pass reported, relays the comm and quotes they raised and fills the
    /// kept frame afresh. A step or a stop first lets go what the last
    /// double-time frame held, and a double-time last step holds its
    /// frame and its second pass, whose quotes it says now (see the
    /// `between_passes` module). Whether the escape pod's rescue ended the
    /// stepping.
    pub(super) fn step_frame(&mut self, steps: u32, anchor: Anchor) -> bool {
        let input = self.step_input(anchor);
        let mut report = std::mem::take(&mut self.step_report);
        self.sim.step(&input, steps, &mut report);
        let mut notes = Vec::new();
        let mut quotes = Vec::new();
        if !report.steps.is_empty() {
            notes.extend(self.inbox.take_comm());
        }
        if report.steps.iter().any(|run| !run.passes.is_empty()) {
            quotes.extend(self.inbox.take_quotes());
        }
        if !report.steps.is_empty() || report.stopped {
            self.release_between_passes(anchor);
        }
        let held = report.steps.last_mut().and_then(BetweenPasses::taken_from);
        let mut rescued = false;
        for run in &report.steps {
            notes.extend(run.comm.iter().cloned());
            self.keep_before(run);
            for (at, pass) in run.passes.iter().enumerate() {
                quotes.extend(pass.quotes.iter().cloned());
                if let Some(wrap) = pass.wrap {
                    self.carry_wrap(&wrap);
                }
                let first = (at == 0).then_some(self.previous.position);
                self.step_effects(pass, first, anchor);
            }
            if let Some(ship) = run.rescued {
                self.previous = ship;
                rescued = true;
            }
        }
        quotes.extend(held.iter().flat_map(BetweenPasses::quotes).cloned());
        self.between_passes = held.or_else(|| self.between_passes.take());
        self.step_report = report;
        self.relay_comm_and_quotes(&notes, quotes);
        self.refresh_frame();
        rescued
    }

    /// Keeps each ship's place before `run`, for the drawing to move
    /// from, in its buffer.
    fn keep_before(&mut self, run: &StepRun) {
        self.npc_previous.clear();
        for &(ship, place) in &run.before {
            match ship {
                ShipRef::Player => self.previous = place,
                ShipRef::Npc(id) => self.npc_previous.push((id, place)),
            }
        }
    }

    /// Carries what the view keeps by `wrap`, as the pass that ran it is
    /// reached (see the module docs).
    pub(super) fn carry_wrap(&mut self, wrap: &PlayerWrap) {
        self.previous.position = wrap.carried(self.previous.position);
        for (_, place) in &mut self.npc_previous {
            wrap.carry(&mut place.position);
        }
        self.effects.carry(wrap);
        self.shot_looks.carry(wrap);
        self.stars.mark_jumped();
    }

    /// Steps the view's effects, flashes, glows, shot looks and stars
    /// after `pass`, as the view stepped them after each pass: `first`
    /// the ship's position before the step on its first pass, the ship
    /// drawn at `anchor`.
    pub(super) fn step_effects(&mut self, pass: &PassReport, first: Option<Vec2>, anchor: Anchor) {
        let dying = dying(&pass.ships, &self.sheet, &self.npc_sheets);
        let chance = &mut self.effects_chance;
        let scene = self.presentation.scene(pass.player_position);
        self.effects.step(&dying, &scene, &self.looks, chance);
        let shown = (&mut self.effects, &mut self.flashes);
        let sheets = (&self.sheet, &self.npc_sheets);
        let fight = (pass.combat.as_slice(), pass.ships.as_slice());
        let lit = show_combat(fight, &scene, shown, (&self.looks, sheets), chance);
        let (player, sheets) = (&self.sheet, &self.npc_sheets);
        self.glows
            .step(&lit, |ship| glow_decay(&pass.ships, player, sheets, ship));
        let effects = (&mut self.effects, scene.caps);
        let trails = self.smoke_trails;
        self.shot_looks
            .step(pass, &self.looks, effects, trails, chance);
        let starfield = (self.presentation.starfield, &self.looks);
        stars::step(
            &mut self.stars,
            pass,
            starfield,
            (self.view, anchor),
            first,
            chance,
        );
    }
}

/// The frame's steps whole: the steps, the gates' and frozen stellars'
/// frames, the rescue's layout, the messages and the traffic's sheets.
impl<C: SystemCatalog + ShipSprites + CombatLooks + StatusBars, S: FlightSim> FlightView<C, S> {
    /// Runs up to `steps` of the flight's steps for the frame, under the
    /// keys held ([`FlightView::step_frame`]): each stopped before it
    /// once the warp has ended or the hold run out, each the advance pass
    /// in the original's order, the answers to what its close struck and
    /// the quotes, then in double time the second pass with the same
    /// (`nova_sim::Session::step`), the view's effects stepped after each
    /// pass and the comm notes and quotes said. Then each hypergate draws
    /// the frame the steps left it on, and a pass whose ship tick brought
    /// the escape pod's rescue, which ends the stepping there, has the
    /// view laid out afresh where the rescue put the ship. Then the
    /// messages the steps raised are said, then the line they raised last
    /// again, whatever its kind ([`FlightView::relay_last_line`]), and the
    /// sheets of a system the session populated afresh are read. Flight and the jump's warp both
    /// step this way.
    ///
    /// The comm notes and quotes are said before the gates' frames are
    /// shown, where the view once showed the frames first: the two touch
    /// nothing in common, and both are set before the frame is drawn.
    pub(super) fn run_steps(&mut self, steps: u32) {
        let rescued = self.step_frame(steps, self.anchor());
        let drawn = between_passes::drawn(self.between_passes.as_ref(), &self.frame);
        if let Some(scene) = &mut self.scene {
            show_frozen_frames(scene, drawn);
            show_gate_frames(scene, drawn);
        }
        if rescued {
            // Laid out in the system the rescue took the pod to, from where
            // it put the ship, before the frame is drawn; the course map
            // looking there too (`_HandlePlayer` @0x68fab).
            self.catch_up();
            self.map.recentre();
        }
        self.relay_messages();
        self.relay_last_line();
        // The session may have populated its system afresh.
        self.read_npc_sheets();
    }
}
