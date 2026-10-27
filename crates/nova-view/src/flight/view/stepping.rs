//! [`FlightView`]'s frame of steps over the [`FlightSim`] port: the input
//! taken from the keys held, preferences and window ([`StepInput`]), one
//! [`FlightSim::step`] a frame, then the view's own effects, flashes,
//! glows, shot looks and stars stepped from each pass the report holds,
//! in the order the view stepped them after each pass when it ran the
//! passes itself.
//!
//! **One call a frame, replayed after it.** The flight runs the frame's
//! steps in one call (`nova_sim::Session::step`), and the view then
//! replays its report pass by pass. Nothing the view does between passes
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
//! ([`FlightView::relay_comm_and_quotes`]).
//!
//! **The presentation rules** the effects, stars and world read
//! ([`Presentation`]) are the view's own, held from the flight's rulebook
//! as the view is read and as its rules change.
//!
//! **The frame after the steps.** Once the report is replayed the kept
//! frame is filled afresh ([`FlightView::refresh_frame`]), so the frame's
//! ships are where the steps left them, the places each step began from
//! ([`StepRun::before`]) being where the drawing moves them from.

use nova_sim::flight_frame::{PassReport, StepInput, StepRun};
use nova_sim::{Controls, Face, PlayerCommand, RuleKey, RuleSource, Rulebook, ShipRef, Turn, Vec2};

use crate::Key;
use crate::flight::effects::{LoopedSounds, Scene};
use crate::flight::sim_port::FlightSim;
use crate::system::camera::Anchor;

use super::{
    AFTERBURNER_KEY, CLOAK_KEY, DESELECT_KEY, EJECT_KEY, FACE_TARGET_KEY, FIRE_KEY, FlightView,
    LAND_KEY, SECONDARY_KEY, SELF_DESTRUCT_KEY, dying, glow_decay, play_area, point, show_combat,
    stars, window_width,
};

/// The rules the view shows the steps' effects, the stars and the world
/// by, held from the flight's rulebook: [`RuleKey::ShipDebris`],
/// [`RuleKey::CombatCaps`], [`RuleKey::LoopedSoundWait`],
/// [`RuleKey::FlightStarfield`], and the world's
/// [`RuleKey::TranslucentShotFade`], [`RuleKey::FlightAnchor`],
/// [`RuleKey::LightningZigZag`] and [`RuleKey::IonTintMurkColour`]
/// (rdm:phase/view-sim-seam/phase-7-world-frame, which pulled these four
/// forward from rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks).
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
}

impl Presentation {
    /// The eight as `rules` read them.
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
    /// kept frame afresh. Whether the escape pod's rescue ended the
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
        let mut rescued = false;
        for run in &report.steps {
            notes.extend(run.comm.iter().cloned());
            self.keep_before(run);
            for (at, pass) in run.passes.iter().enumerate() {
                quotes.extend(pass.quotes.iter().cloned());
                let first = (at == 0).then_some(self.previous.position);
                self.step_effects(pass, first, anchor);
            }
            if let Some(ship) = run.rescued {
                self.previous = ship;
                rescued = true;
            }
        }
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

    /// Steps the view's effects, flashes, glows, shot looks and stars
    /// after `pass`, as the view stepped them after each pass: `first`
    /// the ship's position before the step on its first pass, the ship
    /// drawn at `anchor`.
    fn step_effects(&mut self, pass: &PassReport, first: Option<Vec2>, anchor: Anchor) {
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
