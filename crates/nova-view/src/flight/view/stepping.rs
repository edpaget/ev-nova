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
//! The lines a double-time last step's second pass raised are not among
//! them: the hold keeps them for the frame after
//! ([`StepRun::second_pass_lines`], the `between_passes` module), and the
//! release relays them before the next report's.
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
use nova_sim::{
    Amounts, Controls, Face, PlayerCommand, RuleKey, RuleSource, Rulebook, ShipRef, Turn, Vec2,
};

use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::effects::{Breakups, LoopedSounds, Scene};
use crate::flight::sim_port::FlightSim;
use crate::keymap::FlightCommand as F;
use crate::system::camera::Anchor;
use crate::system::catalog::SystemCatalog;

use super::between_passes::{self, BetweenPasses};
use super::{
    FlightView, dying, glow_decay, play_area, point, show_combat, show_frozen_frames,
    show_gate_frames, stars, window_width,
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
    /// [`RuleKey::HintKeyNames`]'s reading: how the tutorial's hints
    /// name a key.
    pub(super) hint_names: RuleSource,
}

impl Presentation {
    /// The eleven as `rules` read them.
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
            hint_names: rules.source_for(RuleKey::HintKeyNames),
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

/// The turn and the thrust flight holds ([`FlightView::steering`]).
struct Steering {
    /// Which way it turns.
    way: Turn,
    /// How far the turn is held, 0 to 1.
    turn: f32,
    /// How far the thrust is held, 0 to 1.
    thrust: f32,
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// The controls for the commands held and the sticks. Autopilot with
    /// Option ([`F::FaceStellar`]) faces the navigation stellar, whether
    /// its key is Autopilot's with Option or one of its own. The turn and
    /// the thrust are [`FlightView::steering`]'s, held while their amount
    /// is above none.
    pub(super) fn controls(&self) -> Controls {
        let steering = self.steering();
        let face = if self.held(F::FaceStellar) {
            Face::Stellar
        } else if self.held(F::Autopilot) {
            Face::Target
        } else {
            Face::Off
        };
        Controls {
            thrust: steering.thrust > 0.0,
            turn: steering.way,
            reverse: self.held(F::ReverseCourse),
            face,
            afterburner: self.held(F::Afterburner),
        }
    }

    /// How far the turn and the thrust are held: a key's (or the d-pad's)
    /// fully, a stick's by its amount.
    pub(super) fn amounts(&self) -> Amounts {
        let steering = self.steering();
        Amounts {
            turn: steering.turn,
            thrust: steering.thrust,
        }
    }

    /// The turn and the thrust from the commands held and the sticks. The
    /// turn is the key's way at 1 while exactly one turn command is held,
    /// otherwise the turn stick's ([`F::TurnAxis`]), its sign the way and
    /// its size the amount. The thrust is 1 while Accelerate is held,
    /// otherwise the throttle stick's ([`F::ThrottleAxis`]) forward part:
    /// pulled back, it thrusts none (and past the press threshold holds
    /// Reverse Course, a command). So a key or d-pad press beats a smaller
    /// stick amount, and with keys alone each is 0 or 1.
    fn steering(&self) -> Steering {
        let (way, turn) = match (self.held(F::RotateLeft), self.held(F::RotateRight)) {
            (true, false) => (Turn::Left, 1.0),
            (false, true) => (Turn::Right, 1.0),
            _ => {
                let stick = self.keys.axis(F::TurnAxis);
                let way = if stick > 0.0 {
                    Turn::Right
                } else if stick < 0.0 {
                    Turn::Left
                } else {
                    Turn::None
                };
                (way, stick.abs())
            }
        };
        let thrust = if self.held(F::Accelerate) {
            1.0
        } else {
            self.keys.axis(F::ThrottleAxis).max(0.0)
        };
        Steering { way, turn, thrust }
    }

    /// What the frame's steps are given, the ship drawn at `anchor`: the
    /// controls for the keys held; the play area and the window's width;
    /// the primary and secondary fire keys, the eject chord, the
    /// self-destruct chord, the cloak key and the weapon safety key, each
    /// chord, the cloak and the safety only while the flight takes its
    /// command (the cloak key needs no Command, `_QuickKeyCheck`
    /// @0x705b7); double time and Hyperspace Effects as the screen was
    /// told; the land key held and L's latch, `_dockTryFlag`, which holds
    /// the dock approach back (the `faded_keys` module); the jettison
    /// chord, `Some(all)` while Jettison Cargo is held and the flight
    /// takes it, `all` unless it is held with Option too
    /// ([`F::JettisonNonMission`]), or a tap of it since the last step
    /// ([`FlightView::jettison_tap`]); and whether 334 ms or
    /// more of the wall time handed to the view went by without a step,
    /// which holds the next step's gain.
    pub(super) fn step_input(&self, anchor: Anchor) -> StepInput {
        let chord = |gate, command| self.takes(gate) && self.held(command);
        let non_mission = self.held(F::JettisonNonMission);
        let jettison = (self.takes(PlayerCommand::Jettison)
            && (self.held(F::JettisonCargo) || non_mission))
            .then_some(!non_mission)
            .or(self.jettison_tapped);
        StepInput {
            controls: self.controls(),
            amounts: self.amounts(),
            play_area: play_area(self.view, anchor),
            window_width: window_width(self.view),
            fire: (self.held(F::FirePrimary), self.held(F::FireSecondary)),
            eject: chord(PlayerCommand::Eject, F::Eject),
            self_destruct: chord(PlayerCommand::SelfDestruct, F::SelfDestruct),
            cloak: chord(PlayerCommand::Cloak, F::EngageCloak),
            weapon_safety: chord(PlayerCommand::DeselectWeapon, F::WeaponSafety),
            double_time_key: self.double_time,
            hyperspace_effects: self.hyperspace_effects,
            land_key: self.held(F::LandDock),
            dock_try: self.faded_keys.land_latched(),
            jettison,
            held: nova_sim::speed_mult::holds_after(self.since_step),
        }
    }

    /// Runs the frame's `steps` through the flight, the ship drawn at
    /// `anchor` (see the module docs), steps the view's effects after each
    /// pass reported, relays the comm and quotes they raised and fills the
    /// kept frame afresh. A step or a stop first lets go what the last
    /// double-time frame held, and a double-time last step holds its
    /// frame and its second pass, heard now, its lines worded for the
    /// frame after (see the `between_passes` module). Whether the escape pod's rescue ended the
    /// stepping.
    pub(super) fn step_frame(&mut self, steps: u32, anchor: Anchor) -> bool {
        let input = self.step_input(anchor);
        let mut report = std::mem::take(&mut self.step_report);
        self.sim.step(&input, steps, &mut report);
        if !report.steps.is_empty() {
            self.since_step = std::time::Duration::ZERO;
            // A step has read the tap (`FlightView::jettison_tap`).
            self.jettison_tapped = None;
        }
        let mut notes = Vec::new();
        let mut quotes = Vec::new();
        if !report.steps.is_empty() {
            notes.extend(self.inbox.take_comm());
        }
        if report.steps.iter().any(|run| !run.passes.is_empty()) {
            quotes.extend(self.inbox.take_quotes());
        }
        if !report.steps.is_empty() || report.stopped {
            // A stop for the warp's end leaves them for the steps its
            // arrival runs, which relay them over the arrival's line.
            if !(report.steps.is_empty() && self.frame.warp_end_due) {
                self.release_held_lines();
            }
            self.release_between_passes(anchor);
        }
        let mut held = report.steps.last_mut().and_then(BetweenPasses::taken_from);
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
                self.step_effects(pass, first, anchor, None);
            }
            if let Some(ship) = run.rescued {
                self.previous = ship;
                rescued = true;
            }
        }
        if let Some(held) = &mut held {
            self.hear_held_pass(held);
        }
        self.between_passes = held.or_else(|| self.between_passes.take());
        let lines = report
            .steps
            .last_mut()
            .map(|run| std::mem::take(&mut run.second_pass_lines))
            .unwrap_or_default();
        self.step_report = report;
        self.relay_comm_and_quotes(&notes, quotes);
        self.refresh_frame();
        self.hold_lines(lines);
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
    /// drawn at `anchor`. The post-animate a frame left moves the
    /// particles first, and a first pass leaves one, as a frame stands
    /// after it (see the `scroll_offset` module). A pass `heard` already,
    /// its breakups rolled, is only shown (see the `between_passes`
    /// module).
    pub(super) fn step_effects(
        &mut self,
        pass: &PassReport,
        first: Option<Vec2>,
        anchor: Anchor,
        heard: Option<Breakups>,
    ) {
        if self.scroll.begin_pass() {
            self.effects.move_particles();
        }
        let chance = &mut self.effects_chance;
        let scene = self.presentation.scene(pass.player_position);
        let was_heard = heard.is_some();
        let breakups = if let Some(breakups) = heard {
            breakups
        } else {
            let dying = dying(&pass.ships, &self.sheet, &self.npc_sheets);
            self.effects.hear_step(&dying, &scene, &self.looks, chance)
        };
        // The pass's gain, `_gSpeedMult`: `_HandleExplods`' for the
        // explosions (@0x2f655, @0x2f811), `_HandleEscapePods`' for the
        // pods (@0x39e92, @0x39ebe, @0x39fcc), a new pod's first step among
        // them, and `_HandleShipDisplay`'s for the flashes (@0x2c964) and
        // the glows (@0x2c7ed). The shot looks read it from `pass` for
        // `_HandleSmoke` (@0x2e3d6).
        let gain = pass.speed_mult;
        self.effects.show_step(&breakups, &scene, &self.looks, gain);
        let shown = (&mut self.effects, &mut self.flashes);
        let sheets = (&self.sheet, &self.npc_sheets);
        let fight = (pass.combat.as_slice(), pass.ships.as_slice());
        let looks = (&self.looks, sheets);
        let heard_at = (was_heard, gain);
        let lit = show_combat(fight, &scene, shown, looks, heard_at, chance);
        let (player, sheets) = (&self.sheet, &self.npc_sheets);
        self.glows.step(
            &lit,
            |ship| glow_decay(&pass.ships, player, sheets, ship),
            gain,
        );
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
        if first.is_some() {
            self.scroll.drawn(point(pass.player_position));
        }
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
    /// the frame the steps left it on, the messages the steps raised are
    /// said, then the line they raised last again, whatever its kind
    /// ([`FlightView::relay_last_line`]). A pass whose ship tick brought
    /// the escape pod's rescue, which ends the stepping there, then
    /// begins the rescue's screen over `pod`, the frame the pod showed as
    /// the tick began ([`FlightView::pod_frame`], the `rescue` module), or,
    /// for a rescue on double time's second pass, the frame between its
    /// passes ([`FlightView::pod_frame_between_passes`]), the lines that
    /// pass held relayed after it,
    /// its repaint reading the message line those lines left, which lays
    /// the view out afresh where the rescue put the ship once it has faded
    /// out. Last the sheets of a system the session populated afresh are
    /// read. Flight and the jump's warp both step this way.
    ///
    /// The comm notes and quotes are said before the gates' frames are
    /// shown, where the view once showed the frames first: the two touch
    /// nothing in common, and both are set before the frame is drawn.
    pub(super) fn run_steps(&mut self, steps: u32, pod: Option<super::rescue::PodFrame>) {
        let rescued = self.step_frame(steps, self.anchor());
        let drawn = between_passes::drawn(self.between_passes.as_ref(), &self.frame);
        if let Some(scene) = &mut self.scene {
            show_frozen_frames(scene, drawn);
            show_gate_frames(scene, drawn);
        }
        self.relay_messages();
        self.relay_last_line();
        if rescued {
            // A rescue on double time's second pass fades out over the
            // frame between its passes, its lines under the first text.
            let pod = match self.pod_frame_between_passes() {
                Some(between) => {
                    self.release_held_lines();
                    Some(between)
                }
                None => pod,
            };
            self.begin_rescue(pod);
        }
        // The session may have populated its system afresh.
        self.read_npc_sheets();
    }
}
