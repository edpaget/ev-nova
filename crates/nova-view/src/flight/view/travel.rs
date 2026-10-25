//! [`FlightView`]'s travel: fades and death, the course map, the jump and
//! its steps, arrival, take-off, landing, the shipyard and outfitter
//! orders.

use std::time::Duration;

use nova_sim::SessionEvent;
use nova_sim::gate::next_gate_pick;
use nova_sim::{
    Assigned, Assignment, AsteroidCatalog, CombatCatalog, CommCatalog, Condition, Departure,
    FighterNote, GateKind, GateRefusal, LandOutcome, LandPress, LandingRefusal, OutfitOrder,
    OutfitRefusal, OutfitRun, PilotCatalog, RuleKey, RuleSource, Rules, Session, Settled, ShipId,
    ShipPurchase, ShipRefusal, SimMessage, StellarId, SystemId, TrafficCatalog,
};
use nova_sim::{PlayerCommand, PlayerFate};

use nova_sim::flight_frame::{Answer, FlightCommand, MapClick};

use crate::devtools::SessionDesk;
use crate::fade::{DEATH_FADE_FOR, ScreenFade};
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::effects::{LoopedSounds, Scene};
use crate::flight::escorts::{EscortMenuLooks, fighters_abandoned_message};
use crate::flight::jump::JumpEffect;
use crate::flight::sim_port::FlightSim;
use crate::galaxy::{GalaxyCatalog, MapCycle, MapMode, PendingPress, PressAnswer};
use crate::system::catalog::SystemCatalog;
use crate::view_shape::classic_offset;
use crate::{DrawList, Input, Key, Point, Screen, UiSound};

use super::dialog_map::{MapOpened, MapOpening};
use super::stars;
use super::{
    BOARD_KEY, CLOAK_KEY, DESELECT_KEY, EJECT_KEY, FAR_ENOUGH, FIRE_KEY, FlightView,
    GATE_REFUSAL_TICKS, HYPERGATE_CANCELLED, JUMP_KEY, LAND_KEY, MAP_KEY, MESSAGE_TICKS,
    NOTHING_OUT_HERE, NavFlash, Redrawn, SCRIPT_MESSAGE_TICKS, SECONDARY_KEY, SELF_DESTRUCT_KEY,
    Said, TOO_CLOSE, TRACTORED, Then, arrival_lead, arrival_message_with, assigned_message,
    assigned_ticks, comm_message, dying, gate_refusal_message, glow_decay, jump_refusal_message,
    jump_refusal_ticks, landing_refusal_ticks, message_text, no_outfitter, pay_said, play_area,
    point, refusal_message, show_combat, show_destroyed, show_gate_frames, shown_scene,
    stellar_name, window_width,
};

/// What [`FlightView::held_triggers`] holds for the steps.
// Each is its own key or chord, held on its own.
#[allow(clippy::struct_excessive_bools)]
struct HeldTriggers {
    /// The primary and secondary fire keys.
    fire: (bool, bool),
    /// The eject chord.
    eject: bool,
    /// The self-destruct chord.
    self_destruct: bool,
    /// The cloak key.
    cloak: bool,
    /// The weapon safety key.
    safety: bool,
}

impl<
    C: PilotCatalog
        + TrafficCatalog
        + CombatCatalog
        + AsteroidCatalog
        + CommCatalog
        + CombatLooks
        + SystemCatalog
        + ShipSprites
        + StatusBars
        + GalaxyCatalog
        + EscortMenuLooks,
> FlightView<C, crate::flight::SessionFlight<C>>
{
    /// Covers the view, widened or not, in the arrival's fade-in, then in
    /// the jump's fade-out, then in the death's fade to black, as far as
    /// each shows.
    pub(super) fn draw_fades(&self, list: &mut DrawList) {
        for effect in [&self.fade_in, &self.jump].into_iter().flatten() {
            effect.draw_fade(list, self.view);
        }
        if let Some(fade) = &self.death_fade {
            fade.draw(list, self.view);
        }
    }

    /// Whether the hold after the player's explosion has run out
    /// ([`Session::hold_expired`]): flight steps no more.
    pub(super) fn hold_expired(&self) -> bool {
        self.sim.session.as_ref().is_ok_and(Session::hold_expired)
    }

    /// Whether any flight key is held down.
    #[must_use]
    pub fn holds_keys(&self) -> bool {
        !self.held.is_empty()
    }

    /// The `dësc` IDs a sabotaged player sees as it dies
    /// ([`Session::sabotage_death_descs`]); none without a session.
    #[must_use]
    pub fn sabotage_death_descs(&self) -> Vec<i16> {
        self.sim
            .session
            .as_ref()
            .map_or_else(|_| Vec::new(), Session::sabotage_death_descs)
    }

    /// Begins the fade to black after the hold has expired, over
    /// [`DEATH_FADE_FOR`]; nothing before then ([`Session::begin_fade`]).
    pub fn begin_fade(&mut self) {
        let Ok(session) = &mut self.sim.session else {
            return;
        };
        session.begin_fade();
        if session.player_fate() == PlayerFate::Fading && self.death_fade.is_none() {
            self.death_fade = Some(ScreenFade::to_black(DEATH_FADE_FOR));
        }
    }

    /// Whether the death's fade is over and play has ended: the router
    /// returns to the main screen.
    #[must_use]
    pub fn game_over(&self) -> bool {
        self.sim
            .session
            .as_ref()
            .is_ok_and(|session| session.player_fate() == PlayerFate::GameOver)
    }

    /// Whether the pilot may be saved now, as [`Session::may_save`] says:
    /// not while the player is dying or dead, nor without a session.
    #[must_use]
    pub fn may_save(&self) -> bool {
        self.sim.session.as_ref().is_ok_and(Session::may_save)
    }

    /// Whether the pilot is dying, as `_IsDying` (@0x253d) says: its ship
    /// breaking up or destroyed, which [`FlightView::may_save`] answers.
    /// Enter Ship refuses such a pilot. Never without a session.
    #[must_use]
    pub fn is_dying(&self) -> bool {
        self.sim.session.is_ok() && !self.may_save()
    }

    /// Whether ending play now deletes the pilot, as
    /// [`Session::deletes_pilot_on_ending_play`] says: a strict-play pilot
    /// dying outside the escape pod. Never without a session.
    #[must_use]
    pub fn deletes_pilot_on_ending_play(&self) -> bool {
        self.sim
            .session
            .as_ref()
            .is_ok_and(Session::deletes_pilot_on_ending_play)
    }

    /// Moves the death's fade on by `dt`, and ends play once it is over;
    /// nothing else moves.
    pub(super) fn advance_death_fade(&mut self, dt: Duration) {
        let Some(fade) = &mut self.death_fade else {
            return;
        };
        fade.advance(dt);
        if fade.done()
            && let Ok(session) = &mut self.sim.session
        {
            session.end_play();
        }
    }

    /// The input to the map opened over a screen: in port
    /// ([`FlightView::open_bar_map`], [`FlightView::open_port_screen_map`]),
    /// or by a mission dialog docked or in flight
    /// ([`FlightView::open_dialog_map`]): a press of [`MAP_KEY`]
    /// closes it ([`FlightView::close_map`]), unless
    /// Find is open, where it types, and anything else goes to the map, so
    /// flight's own keys and cursor stay out of the port.
    pub fn port_map_input(&mut self, input: &Input) {
        let closing = Input::Key {
            key: MAP_KEY,
            pressed: true,
            repeat: false,
        };
        if *input == closing && !self.map.finding() {
            self.close_map();
        } else {
            self.map_input(input);
        }
    }

    /// The map's input, the rest of `_MapFilter` (@0x13c63-0x147ab) that
    /// the map leaves to its owner:
    ///
    /// - A key-down (not a repeat) of a key the filter doesn't take
    ///   ([`FlightView::map_filter_takes`]) while [`MAP_KEY`] is held
    ///   closes the map ([`FlightView::close_map`]), as the map key's close
    ///   reads the key held (@0x144db-0x14587). M's own press closes it
    ///   before this, in flight and in port.
    /// - A press on it that hits systems is the session's click
    ///   ([`FlightView::answer_map_press`]), and C or Clear Route clears
    ///   the route ([`FlightView::clear_route`]).
    /// - Tab or `\` cycles the selection ([`FlightView::cycle_map`]).
    /// - Return, or a click on Done, closes the map
    ///   ([`FlightView::close_map`]): with the close beep and the cue in
    ///   flight, entering or cancelling the hypergate silently, and putting
    ///   back the bar's nav in port, as M does.
    /// - A text Find confirmed is searched as the in-flight map's Find
    ///   searches ([`Session::find_map_system`]), and the map shows what it
    ///   found ([`GalaxyMap::show_search`](crate::galaxy::GalaxyMap::show_search)).
    /// - The map's beeps (Space's and Find's) join the flight's.
    ///
    /// While Find is open every input goes to the map, which gives it to
    /// Find: a key with [`MAP_KEY`] held types, and closes nothing.
    pub(super) fn map_input(&mut self, input: &Input) {
        self.map_input_then(input, Self::close_map);
    }

    /// Begins a jump, or shows why not when the player is told
    /// ([`Session::jump_refusal_told`]). Starting it clears the message
    /// line only when it says [`TOO_CLOSE`], [`TRACTORED`], [`FAR_ENOUGH`]
    /// or [`NOTHING_OUT_HERE`] (`_HandlePlayer` @0x6b231-0x6b3d9, #36 at
    /// @0x6b2ac-0x6b30d). The original's second jump
    /// path clears them again (@0x6d601-0x6d7b1), once the pilot is past
    /// the tutorial, which the arrival line replaces anyway. The keys
    /// held stay held: the session flies the jump without them, and a
    /// steering, thrust or fire key still held at arrival acts then, as
    /// the original reads those live. J held through the jump does
    /// nothing after it: its latch is set on every frame of the jump
    /// ([`FlightView::jump_frame`]).
    fn jump(&mut self) {
        self.settle_given_up_jump();
        let Ok(session) = &mut self.sim.session else {
            return;
        };
        match session.begin_jump() {
            Ok(_) => {
                for text in [TOO_CLOSE, TRACTORED, FAR_ENOUGH, NOTHING_OUT_HERE] {
                    self.clear_message(text);
                }
                self.start_jump_effect();
            }
            Err(refusal) => {
                if session.jump_refusal_told(&refusal)
                    && let Some(text) = jump_refusal_message(&refusal)
                {
                    self.say_for(text, jump_refusal_ticks(&refusal));
                }
            }
        }
    }

    /// The triggers held as the steps run: the primary and secondary
    /// fire keys, the eject chord, the self-destruct chord, the cloak key
    /// and the weapon safety key, each chord, the cloak and the safety
    /// only while the player's fate takes its command. The cloak key needs no Command (`_QuickKeyCheck`
    /// @0x705b7).
    fn held_triggers(&self) -> HeldTriggers {
        let fire = (
            self.held.contains(&FIRE_KEY),
            self.held.contains(&SECONDARY_KEY),
        );
        let chord = |command, key| {
            self.takes(command) && self.held.contains(&Key::Command) && self.held.contains(&key)
        };
        HeldTriggers {
            fire,
            eject: chord(PlayerCommand::Eject, EJECT_KEY),
            self_destruct: chord(PlayerCommand::SelfDestruct, SELF_DESTRUCT_KEY),
            cloak: self.takes(PlayerCommand::Cloak) && self.held.contains(&CLOAK_KEY),
            safety: self.takes(PlayerCommand::DeselectWeapon) && self.held.contains(&DESELECT_KEY),
        }
    }

    /// Runs up to `steps` of the session's steps, each only while
    /// `go_on` holds of the session before it: the advance pass under the
    /// keys held, in the original's order ([`Session::game_work`]: the
    /// player, the fight's collision passes, the traffic's AI, the shots,
    /// the traffic's motion, the NPCs' fire, the beams), the answers to
    /// what its close struck and the quotes, then the effects; in double
    /// time, the second pass ([`Session::tick_again`]) with the same, and
    /// the effects again. Then says the comm
    /// notes, quotes and messages they raised, and reads the sheets of a
    /// system the session populated afresh. Flight and the jump's warp
    /// both step this way. The eject and self-destruct chords, the land key
    /// and double time are held as the keys say, and a pass whose ship
    /// tick brings the escape pod's rescue ends the stepping there, the
    /// view laid out afresh where the rescue put the ship.
    #[allow(clippy::too_many_lines)] // each pass's steps, in the original's order
    pub(super) fn run_steps(&mut self, steps: u32, go_on: fn(&Session) -> bool) {
        let controls = self.controls();
        let HeldTriggers {
            fire,
            eject,
            self_destruct,
            cloak,
            safety,
        } = self.held_triggers();
        let mut notes = Vec::new();
        let mut quotes = Vec::new();
        let mut rescued = false;
        let anchor = self.anchor();
        if let Ok(session) = &mut self.sim.session {
            session.set_play_area(play_area(self.view, anchor));
            session.set_window_width(window_width(self.view));
            session.hold_fire(fire.0, fire.1);
            session.hold_eject(eject);
            session.hold_self_destruct(self_destruct);
            session.hold_cloak(cloak);
            session.hold_weapon_safety(safety);
            session.set_double_time_key(self.double_time);
            session.set_hyperspace_effects(self.hyperspace_effects);
            session.set_land_key(self.held.contains(&LAND_KEY));
            'steps: for _ in 0..steps {
                if !go_on(session) {
                    break;
                }
                self.previous = *session.player();
                self.npc_previous = session
                    .npcs()
                    .iter()
                    .map(|npc| (npc.id, npc.state))
                    .collect();
                let rules = Rules {
                    disable: &*self.sim.disable_rule,
                    defence: &*self.sim.defence_rule,
                    law: &*self.sim.law,
                };
                for again in [false, true] {
                    if again && !session.double_time() {
                        break;
                    }
                    // The rescue settles on the pass the pod's countdown
                    // runs out, and stops the stepping so its dësc opens
                    // before the next: the original's `_HandlePlayer` runs
                    // it in place of the rest of that pass's player
                    // handling (@0x68ddd).
                    let rescued_now = if again {
                        session.tick_again(
                            controls,
                            &*self.sim.behaviour,
                            rules,
                            &mut self.sim.chance,
                        );
                        session.rescue_if_due(&*self.catalog, &mut self.sim.chance)
                    } else {
                        let behaviour = &*self.sim.behaviour;
                        let rescued_now = session.game_work(
                            controls,
                            &*self.catalog,
                            behaviour,
                            rules,
                            &mut self.sim.chance,
                        );
                        self.inbox.drain_session(session);
                        notes.extend(self.inbox.take_comm());
                        rescued_now
                    };
                    if rescued_now {
                        self.previous = *session.player();
                        rescued = true;
                        break 'steps;
                    }
                    // The pass answers its collision passes' and shots'
                    // hits as they land, as `_HandleShot` does (@0x44385);
                    // what its close struck (the beams, the shockwaves)
                    // is answered here, before the frame is drawn. Each
                    // pass then says the attack quotes its answers raise
                    // (`_ShowPersonHailQuote` @0x89ce1) and runs
                    // `_HandleShip`'s quote roll (@0x33f0f-0x33fba), the
                    // quote clock moving on the advance pass alone.
                    session.answer_strikes(&*self.sim.behaviour);
                    if again {
                        session.tick_quotes_again(&*self.catalog, &mut self.sim.chance);
                    } else {
                        session.tick_quotes(&*self.catalog, &mut self.sim.chance);
                    }
                    self.inbox.drain_session(session);
                    quotes.extend(self.inbox.take_quotes());
                    let dying = dying(session, &self.sheet, &self.npc_sheets);
                    let chance = &mut self.effects_chance;
                    let scene = effects_scene(session);
                    self.effects.step(&dying, &scene, &self.looks, chance);
                    let shown = (&mut self.effects, &mut self.flashes);
                    let sheets = (&self.sheet, &self.npc_sheets);
                    let lit = show_combat(session, &scene, shown, (&self.looks, sheets), chance);
                    let (player, sheets) = (&self.sheet, &self.npc_sheets);
                    let session_now: &Session = session;
                    self.glows
                        .step(&lit, |ship| glow_decay(session_now, player, sheets, ship));
                    let effects = (&mut self.effects, scene.caps);
                    let trails = self.smoke_trails;
                    self.shot_looks
                        .step(session, &self.looks, effects, trails, chance);
                    let first = (!again).then_some(self.previous.position);
                    let place = (self.view, anchor, &self.looks);
                    stars::step(&mut self.stars, session, place, first, chance);
                }
            }
            if let Some(scene) = &mut self.scene {
                show_gate_frames(scene, session);
            }
        }
        // The session set its line as it raised each of these, and the
        // steps have counted it down since: relayed, not set again.
        for note in &notes {
            self.relay(comm_message(note), note.ticks());
        }
        for quote in quotes {
            let ticks = quote.ticks();
            self.relay(quote.text, ticks);
        }
        if rescued {
            // Laid out in the system the rescue took the pod to, from where
            // it put the ship, before the frame is drawn; the course map
            // looking there too (`_HandlePlayer` @0x68fab).
            self.catch_up();
            self.map.recentre();
        }
        self.relay_messages();
        // The session may have populated its system afresh.
        self.read_npc_sheets();
    }

    /// Starts the jump's effect once the session says the jump has begun,
    /// or lights the screen, unless it is playing already; and starts its
    /// fade to white once the session's `_hyperGamma` is above 0
    /// ([`Session::hyper_gamma`]).
    pub(super) fn start_jump_effect(&mut self) {
        let Ok(session) = &self.sim.session else {
            return;
        };
        // `_hyperGamma` goes back to -1 on the first pass after the
        // arrival, so it lights only a jump still under way.
        let under_way = session.jumping().is_some() || session.preparing_jump().is_some();
        let lit = under_way && session.hyper_gamma() > 0.0;
        if self.jump.is_none() && (session.jumping().is_some() || lit) {
            self.jump = Some(JumpEffect::warp(self.hyperspace_effects));
        }
        if lit && let Some(effect) = &mut self.jump {
            effect.light();
        }
    }

    /// Settles the jump's effect once the session has given its jump up
    /// before arrival (its warp collapsed under a disabled player, or cut
    /// by a sabotage kill outfit): lit, the screen fades back from white;
    /// unlit, the effect is dropped ([`JumpEffect::given_up`]).
    ///
    /// It runs at the top of a tick, before the keys, and before a jump
    /// begins ([`FlightView::jump`]), as `_HandlePlayer` calls
    /// `_FadeWhiteOut` at the top of its first pass with +0x50 no longer
    /// above 0 (@0x6840b-0x6841f): the frame drawn after the cancelling
    /// tick still shows the fade to white where it had got, as the
    /// original's asynchronous `CGDisplayFade` runs on under it.
    pub(super) fn settle_given_up_jump(&mut self) {
        if self.jump.is_none() || self.jump_under_way() {
            return;
        }
        if let Some(fade) = self.jump.take().and_then(JumpEffect::given_up) {
            self.fade_in = Some(fade);
        }
    }

    /// Whether the ship is braking and turning before a jump.
    #[cfg(test)]
    pub(super) fn preparing_jump(&self) -> bool {
        self.sim
            .session
            .as_ref()
            .is_ok_and(|session| session.preparing_jump().is_some())
    }

    /// The keys `_HandlePlayer` polls with a latch, once a frame, with
    /// those held down, in its order: Option-\` and the Nav Off /
    /// Hyperspace Mode block ([`FlightView::nav_keys_frame`]), Hyper
    /// Select ([`FlightView::select_frame`]), the stellar slot keys
    /// ([`FlightView::slot_keys_frame`]), the Board key
    /// ([`FlightView::board_frame`]), then the jump key
    /// ([`FlightView::jump_frame`]), each while the session takes its
    /// command. A key held across a jump's end, a release of Option or a
    /// repair acts here, as a press would, on the first frame its latch
    /// lets it.
    ///
    /// It runs before the tick's steps, as `_HandlePlayer` runs before the
    /// ships' `_HandleShip` (@0x43e19, @0x44483): a repair made in one
    /// tick's steps is acted on at the next, the frame the original sees
    /// it. And it runs before the jump effect's arrival, which is where
    /// the original's arrival frame runs its keys (+0x50 still above 0):
    /// the arrival tick still counts as jumping, and the first tick after
    /// it is the original's first frame to act (@0x6c625).
    pub(super) fn held_keys_frame(&mut self) {
        let down = |key: Key| self.held.contains(&key);
        let (jump, board) = (down(JUMP_KEY), down(BOARD_KEY));
        self.nav_held_frame();
        if self.takes(PlayerCommand::Board) {
            self.board_frame(board);
        }
        if self.takes(PlayerCommand::Jump) {
            self.jump_frame(jump);
        }
    }

    /// One frame of the jump key, with J down or not, as `_HandlePlayer`
    /// runs it (@0x6b0da-0x6badf) on [`FlightView::jump_latch`]: during a
    /// jump, the latch set (@0x6bad3), so J held through a jump does
    /// nothing after it; with J down and the player intact, a jump begun
    /// or refused when the latch is clear (@0x6b86d, @0x6b955), and the
    /// latch set; otherwise, J up or the player disabled or breaking up
    /// (`_IsDisabled`), the latch cleared (@0x6badf). So J held into a
    /// disable, or first pressed while disabled, acts on the first frame
    /// the player is intact again.
    pub(super) fn jump_frame(&mut self, down: bool) {
        if self.jump_under_way() {
            self.jump_latch = true;
            return;
        }
        let intact = self
            .sim
            .session
            .as_ref()
            .is_ok_and(|session| session.player_condition() == Condition::Intact);
        if down && intact {
            if !self.jump_latch {
                self.jump();
            }
            self.jump_latch = true;
        } else {
            self.jump_latch = false;
        }
    }

    /// Ends the jump: the ship arrives in the next system, caught up with
    /// as [`FlightView::arrived`] says.
    pub(super) fn arrive(&mut self) {
        let Ok(session) = &mut self.sim.session else {
            return;
        };
        if session
            .arrive(&*self.catalog, &mut self.sim.chance)
            .is_some()
        {
            self.arrived();
        }
    }

    /// Catches up with the ship having arrived in a system
    /// ([`FlightView::catch_up`], which lays it out from the session's
    /// event, drawn from where the ship arrives), and shows the session's
    /// arrival message in the original's words, naming the scene laid
    /// out, or the system's message buoy in its place
    /// ([`SimMessage::Buoy`], its text as the session gives it), followed
    /// by how many fighters were abandoned and how many hired escorts
    /// defected unpaid, if any, the fighters first. A buoy's arrival
    /// carries no fighter notes: the session drops them, as the buoy
    /// replaces the arrival line's suffix too.
    fn arrived(&mut self) {
        self.catch_up();
        self.show_arrival_explosions();
        // Every arrival sets `_mapScroll` to the system arrived in: a
        // jump's (`_HandlePlayer` @0x6c073), a hypergate's
        // (`_PlayerEnterHypergate` @0x63a5d) and a wormhole's
        // (`_PlayerEnterWormhole` @0x64307).
        self.map.recentre();
        let Ok(session) = &mut self.sim.session else {
            return;
        };
        self.inbox.drain_session(session);
        let notes = self.inbox.take_fighter_notes();
        let pay = self.inbox.take_pay_notes();
        let date = session.long_date_text();
        let mut messages = std::mem::take(&mut self.pending_leads);
        messages.extend(self.inbox.take_messages());
        let scene = self.scene.as_ref();
        let chance = &mut self.sim.chance;
        let worded: Vec<(String, u16)> = messages
            .iter()
            .filter_map(|message| {
                Some((
                    message_text(message, session, scene, chance)?,
                    message.ticks(),
                ))
            })
            .collect();
        self.blank_message();
        let Some(scene) = &self.scene else {
            return;
        };
        let stellars = !scene.stellars().is_empty();
        let mut said: Vec<(String, u16)> = messages
            .iter()
            .filter_map(|message| {
                let text = match message {
                    SimMessage::Buoy(text) => text.clone(),
                    _ => {
                        arrival_message_with(arrival_lead(message)?, scene.name(), &date, stellars)
                    }
                };
                Some((text, message.ticks()))
            })
            .next_back()
            .into_iter()
            .collect();
        said.extend(worded);
        said.extend(notes.into_iter().map(|FighterNote::Abandoned(count)| {
            (fighters_abandoned_message(count), MESSAGE_TICKS)
        }));
        said.extend(pay_said(&pay));
        self.say_all(&said);
    }

    /// Catches the screen up with what the session has told since this
    /// was last called ([`Session::take_events`]), and gives what it
    /// redrew. Several events of a kind ask for one reaction, decided by
    /// the events alone and never by what the session was last asked to
    /// do: a change of ship reads the player's sprite sheet afresh
    /// ([`FlightView::follow_ship`]); a system entered afresh is laid out
    /// ([`FlightView::lay_out`]); otherwise a move to another stellar of
    /// the system shown draws the ship there ([`FlightView::place_ship`]);
    /// a rebuild point that changed which stellars are shown, with no
    /// system entered, reads the scene again with them ([`shown_scene`]);
    /// and a change to the galaxy draws the session's galaxy on the course
    /// map ([`GalaxyMap::show_galaxy`](crate::galaxy::GalaxyMap::show_galaxy)) with the course as it is flown.
    ///
    /// The only caller of [`Session::take_events`]: each frame calls it
    /// first, so a change made outside the view's own methods (a
    /// developer tools' edit, a set expression a control bit runs) is
    /// drawn by the next frame; and each of the view's methods that drives
    /// the session between frames calls it last, so its caller sees the
    /// screen caught up at once. Taking the events again finds none.
    pub(super) fn catch_up(&mut self) -> Redrawn {
        let Ok(session) = &mut self.sim.session else {
            return Redrawn::default();
        };
        let mut redrawn = Redrawn::default();
        self.inbox.drain_session(session);
        for event in self.inbox.take_events() {
            match event {
                SessionEvent::ShipChanged => redrawn.ship = true,
                SessionEvent::SystemChanged => redrawn.system = true,
                SessionEvent::StellarChanged => redrawn.stellar = true,
                SessionEvent::GalaxyChanged => redrawn.galaxy = true,
                SessionEvent::StellarsRebuilt => redrawn.rebuilt = true,
                SessionEvent::StarfieldReset => redrawn.stars = true,
            }
        }
        if redrawn.galaxy {
            self.map.show_galaxy(session.galaxy());
        }
        let system = session.system();
        let reshown = redrawn.rebuilt
            && !redrawn.system
            && self.scene.as_ref().is_some_and(|scene| {
                !scene
                    .stellars()
                    .iter()
                    .map(|stellar| stellar.id)
                    .eq(session.stellars_shown())
            });
        if redrawn.galaxy {
            // With the galaxy, before anything else is drawn: only reads
            // came between.
            self.show_course();
        }
        if redrawn.ship {
            self.follow_ship();
        }
        self.follow_bar();
        if redrawn.system {
            self.lay_out(system);
        } else if redrawn.stellar {
            self.place_ship();
        }
        if reshown && let Ok(session) = &self.sim.session {
            self.scene = Some(shown_scene(&*self.catalog, session));
        }
        if redrawn.galaxy || redrawn.system || redrawn.rebuilt {
            self.show_destroyed();
        }
        if redrawn.system {
            self.redraw_status();
        }
        if std::mem::take(&mut self.stars_due) || redrawn.stars {
            self.reset_stars();
        }
        // The open map's drawing follows the galaxy, the explored and known
        // sets, the course and the nav. The flight is paused while it is
        // open, so this is rare: a set expression or the developer tools.
        if self.map_open
            && self.sim.session.is_ok()
            && (redrawn.galaxy || self.map_seen.as_ref() != Some(&self.frame_now().map_key))
        {
            self.redraw_map();
        }
        redrawn
    }

    /// Scatters the stars afresh about the ship and places them once, on
    /// the effects' chance, when the session draws the original's
    /// starfield (see the `stars` module).
    fn reset_stars(&mut self) {
        let Ok(session) = &self.sim.session else {
            return;
        };
        if stars::original(session, &self.looks) {
            let at = stars::surroundings(session, self.view, self.anchor(), &self.looks);
            stars::reset(
                &mut self.stars,
                &at,
                self.parallax_starfield,
                &mut self.effects_chance,
            );
        }
    }

    /// Draws each stellar of the scene shown as the session holds it
    /// ([`show_destroyed`]). The scene is not laid out again.
    pub(super) fn show_destroyed(&mut self) {
        if let (Ok(session), Some(scene)) = (&self.sim.session, &mut self.scene) {
            show_destroyed(scene, session);
        }
    }

    /// Lays out `system`, the session's: its scene, read from the catalog
    /// with the stellars the session shows ([`shown_scene`]), the course
    /// map's course, and the ship drawn
    /// from where it is, the last system's NPCs and effects let go.
    fn lay_out(&mut self, system: SystemId) {
        let Ok(session) = &self.sim.session else {
            return;
        };
        self.scene = Some({
            let mut scene = shown_scene(&*self.catalog, session);
            show_gate_frames(&mut scene, session);
            scene
        });
        let course = self.frame_now().course_shown;
        self.map.show_course(system, &course);
        self.previous = *session.player();
        self.alpha = 0.0;
        self.npc_previous.clear();
        self.effects.clear();
        self.flashes.clear();
        self.held_alts.get_mut().clear();
        self.glows.clear();
        self.shot_looks.clear();
        self.read_npc_sheets();
    }

    /// Shows the explosions the jump left pending, a crön's `Y` on its
    /// days among them, in the scene just laid out, so they are on the
    /// arrival's first frame: the original keeps its explod table through
    /// the arrival (only `_ResetPlayer` empties it, @0x1d7fb), and its
    /// `_PrepareAllSprites` advances them once (@0x6c59c, `_HandleExplods`
    /// @0x3a7ce) before the frame's own pass, which the steps' order from
    /// spawn to draw stands for. A `Y`'s carries no sound of its own (see
    /// [`Effects::apply`](crate::flight::effects::Effects::apply)).
    fn show_arrival_explosions(&mut self) {
        let Ok(session) = &mut self.sim.session else {
            return;
        };
        let scene = effects_scene(session);
        let chance = &mut self.effects_chance;
        let events = session.take_combat_events();
        for event in &events {
            self.effects.apply(event, &scene, &self.looks, chance);
        }
        if !events.is_empty() {
            self.effects.step(&[], &scene, &self.looks, chance);
        }
    }

    /// The pilot flying, as the developer tools' desk: its edits reach
    /// the session, moves read the catalog, and the course map's galaxy
    /// names the places. `None` when the session failed.
    pub fn pilot_desk(&mut self) -> Option<SessionDesk<'_, C>> {
        let session = self.sim.session.as_mut().ok()?;
        Some(SessionDesk::new(session, &*self.catalog, self.map.model()))
    }

    /// Draws the ship where the session has it, not on its way from where
    /// it was, and shows the session's course again on the course map:
    /// after a move to another stellar of the system shown, and a
    /// take-off.
    pub(super) fn place_ship(&mut self) {
        let Ok(session) = &self.sim.session else {
            return;
        };
        self.previous = *session.player();
        self.show_course();
        self.alpha = 0.0;
    }

    /// Catches the screen up with an edit made through the pilot desk now,
    /// rather than on the next frame, from the session's events (see the
    /// module docs): a move to another system is laid out as an arrival
    /// is, with no message, and one to another stellar of the system shown
    /// draws the ship there, the course map showing the session's course
    /// again. Nothing is read when nothing moved.
    pub fn resync(&mut self) {
        self.catch_up();
    }

    /// Takes off from the stellar landed on, and gives it; `None` when the
    /// ship has not landed. The session leaves the port as its Leave does
    /// ([`Session::leave_port`]), settling the take-off's events at once.
    /// The next frame draws the ship where it is, at
    /// the stellar, not on its way from where it was, in the system a move
    /// made while landed went to, laid out afresh as the session tells
    /// ([`SessionEvent`]); it shows no message
    /// from before the landing, but greets the pilot past the tutorial
    /// ([`take_off_greeting`](super::take_off_greeting)), or shows a held `Q`'s message, for
    /// [`SCRIPT_MESSAGE_TICKS`], in place of the greeting
    /// (`_PlayerLandOnStellar` @0x63588-0x635e3), followed by how many
    /// hired escorts defected
    /// for want of the take-off's pay ([`defection_message`](super::defection_message)); the
    /// session has populated the system's traffic afresh
    /// ([`Session::take_off`]), so no frame draws the ships from before
    /// the landing. Leaving passes the stay's days, their
    /// events rolled on the flight's chance, and what settling them did
    /// shows as [`FlightView::settle_script`] shows it.
    pub fn take_off(&mut self) -> Option<StellarId> {
        let session = self.sim.session.as_mut().ok()?;
        let departure = session.leave_port(&*self.catalog, &mut self.sim.chance)?;
        Some(self.departed(departure))
    }

    /// Takes off for a `Q` run while landed, if its leave is due
    /// ([`Session::leave_if_due`]), as [`FlightView::take_off`] does for
    /// the port's Leave, and gives the stellar left; `None`, and nothing
    /// changes, when no leave is due. The router calls it once nothing
    /// shown over the spaceport holds the leave.
    pub fn leave_if_due(&mut self) -> Option<StellarId> {
        let session = self.sim.session.as_mut().ok()?;
        let departure = session.leave_if_due(&*self.catalog, &mut self.sim.chance)?;
        Some(self.departed(departure))
    }

    /// Settles the escorts' marks as the ship leaves the stellar it is
    /// landed at ([`Session::land_escorts`]), drawing on the flight's
    /// chance, and gives the report to show over the spaceport; none when
    /// nothing was sold or upgraded. The router calls it before either
    /// leave.
    pub fn land_escorts(&mut self) -> Option<String> {
        let session = self.sim.session.as_mut().ok()?;
        session.land_escorts(&mut self.sim.chance)
    }

    /// Catches the screen up with `departure`: the take-off's line, then
    /// what settling it did; gives the stellar left.
    fn departed(&mut self, departure: Departure) -> StellarId {
        let Departure { stellar, settled } = departure;
        let mut said: Vec<(String, u16)> = self
            .drain()
            .take_leave_message()
            .into_iter()
            .map(|text| (text, SCRIPT_MESSAGE_TICKS))
            .collect();
        let (line, rest, system) = self.took_off();
        said.extend(line);
        self.say_after(&said, rest);
        self.show_settled(settled, system);
        stellar
    }

    /// Whether a `Q` run while landed waits for the take-off
    /// ([`Session::leave_due`]): whoever shows the spaceport leaves it
    /// ([`FlightView::leave_if_due`]), as its Leave does, once nothing
    /// shown over it holds the leave.
    #[must_use]
    pub fn leave_due(&self) -> bool {
        self.sim.session.as_ref().is_ok_and(Session::leave_due)
    }

    /// Catches the screen up with the session having taken off: the ship
    /// drawn where it is, in the system laid out afresh if the session
    /// tells a move while landed changed it, the traffic populated afresh
    /// drawn where it is with its ship types' sheets read, no message from
    /// before kept. Gives the line to say, each part with its ticks: the
    /// take-off greeting past the tutorial ([`SimMessage::TookOff`]), if
    /// any, then what paying the escorts for the take-off has to say; and
    /// the session's other messages, each said after the line and
    /// replacing it, as they come in later frames in the original.
    ///
    /// The original keeps the slot and replaces it with its greeting
    /// (`_PlayerLandOnStellar` @0x63732), which a newbie, or a `Q`'s
    /// message, goes without; a message from before the landing would be
    /// further from what the player sees than none.
    ///
    /// The whole bar is redrawn once the take-off has let go of the nav
    /// selection. Under [`RuleKey::HudCadence`]'s engine reading, the nav
    /// area then shows the nav as the take-off began
    /// ([`Session::take_takeoff_nav`]) in the first frame drawn after it
    /// and to the end of that frame's tick ([`NavFlash`],
    /// [`FlightView::advance_hud`]), when that differs from the nav now,
    /// as after a landed `M`: built by operator decision on 2026-10-15.
    /// The flash isn't timed from the take-off's own tick: the take-off
    /// comes between frames, and the next frame's tick may already be
    /// past it.
    ///
    /// The original redraws the whole bar after the port closes
    /// (`_PlayerLandOnStellar` @0x63399-0x633a0, `_RedrawStatus(1)`) with
    /// +0x6c still the landed selection, and flushes it at once
    /// (`QDFlushPortBuffer` @0x4cd4d-0x4cd71, unless
    /// `_useLegacyRenderingPipeline`). The deselect (@0x674d2) and the
    /// redraws after it (`_RefreshSAT` @0x6765d, `_RedrawStatus(1)`
    /// @0x67662-0x67669) come inside the same `_HandlePlayerDockRequest`
    /// call (@0x66f29 → @0x67669). No game frame passes between the two
    /// redraws: the flash lasts from one flush to the next, at least one
    /// display refresh, under one of ours' 30 Hz ticks. Ours holds it for
    /// at least one presented frame of ours, up to 1/30 s past that
    /// frame's start.
    ///
    /// Every take-off reaches it, not only a landed `M`'s. While docked
    /// the original holds the selection the ship landed with: the land
    /// key puts mode 2 over any mode, a course's mode-3 cue too, with the
    /// nearest stellar (`_HandlePlayer` @0x6a8ed-0x6a932); the dock
    /// request lands on +0x6c (`_StellarIsLandable` @0x66968-0x66977);
    /// and `_PlayerLandOnStellar` passes +0x6c on as the stellar docked
    /// at, to `_DoAutoRefuel` (@0x632b2-0x632c5), `_DoPortDialog`
    /// (@0x6336e-0x63381) and `_SavePilotFile` (@0x6353e-0x6354b). Nothing
    /// between the landing and @0x633a0 writes the player's +0x6a or
    /// +0x6c but a landed `M` or `N` (`_EvalSetExp` @0x15705, @0x157e3,
    /// @0x158fd): the port dialogs put back what they saved
    /// (`_DoPortDialog` @0x5fcf0-0x5fd1b, @0x60084-0x60096, and the
    /// `nav` module's restores), and `_PlayerLandOnStellar`'s and
    /// `_HandlePlayerDockRequest`'s other writes at +0x6c (@0x63520,
    /// @0x672ac) are the auxiliary ships'. So the take-off's redraw names
    /// the landed stellar in mode 2 (`_DrawStatusNav` @0x4a09e-0x4a18c),
    /// or a landed `M`'s (the seven stock finales' `M472 Q25048` name
    /// "Our Spiel"), until the deselect's redraw shows "No Destination",
    /// or the course cued again in mode 3 (`_CueNextHyperRouteDest`
    /// @0xe33c-0xe344, from @0x674d8). Only a take-off whose held
    /// selection is let go already, after an `N` or an `M` to a system
    /// with no stellar, shows no flash.
    ///
    /// Only a set expression run in that window could hold the bar
    /// longer, by starting a mission whose text shows (`S` →
    /// `_DoMissionAccept` @0x15407, its texts @0xa1bb2, @0xa1f0e,
    /// @0xa1fb5): a `crön`'s `OnStart` or `OnEnd` (@0xb58b), a `spöb`'s
    /// `OnRegen` (@0xb6ee), a `nëbu`'s `OnExplore` (@0x634da), or an
    /// auto-aborting mission's `OnAbort` (@0x4317e → @0x99c3e). No stock
    /// one does. Ours runs the stay's days inside the take-off, so a text
    /// they queue is newer than the take-off's held nav: the count of
    /// texts noted at the port's last nav redraw is kept, and
    /// [`FlightView::take_mission_texts`] draws that text's nav, noted
    /// before the deselect, while the flash lasts.
    ///
    /// It gives, last, whether the system was laid out afresh.
    fn took_off(&mut self) -> (Said, Said, bool) {
        let inbox = self.drain();
        let (pay, messages, held) = (
            inbox.take_pay_notes(),
            inbox.take_messages(),
            inbox.take_takeoff_nav(),
        );
        let nav_texts = self.nav_texts;
        let system = self.catch_up().system;
        self.place_ship();
        self.read_npc_sheets();
        self.blank_message();
        self.redraw_status();
        if let Some(held) = held
            && self.hud_waits()
            && held != self.nav_mode()
        {
            self.nav_scene = Some((held, self.nav_display(held)));
            self.nav_flash = Some(NavFlash::Held);
            self.nav_texts = nav_texts;
        }
        let (greeting, rest): (Vec<_>, Vec<_>) = messages
            .into_iter()
            .partition(|message| matches!(message, SimMessage::TookOff { .. }));
        let mut line = self.word(greeting);
        line.extend(pay_said(&pay));
        (line, self.word(rest), system)
    }

    /// Says `line` as [`FlightView::say_all`] does, then each of `rest` in
    /// turn, replacing it.
    fn say_after(&mut self, line: &[(String, u16)], rest: Vec<(String, u16)>) {
        self.say_all(line);
        for (text, ticks) in rest {
            self.say_for(text, ticks);
        }
    }

    /// Shows the texts of `said`, joined by two spaces, when there is
    /// anything, for the ticks of the first: the others are appended to
    /// it, as `_HandlePlayer` appends the abandoned fighters to the
    /// arrival line (@0x6c3c4).
    fn say_all(&mut self, said: &[(String, u16)]) {
        let Some(&(_, ticks)) = said.first() else {
            return;
        };
        let texts: Vec<&str> = said.iter().map(|(text, _)| text.as_str()).collect();
        self.say_for(texts.join("  "), ticks);
    }

    /// Settles what the set expressions run since queued, as
    /// [`Session::settle_script`] does, reading the catalog and drawing on
    /// the flight's chance. A `Q` run while landed never takes off here:
    /// it is held ([`FlightView::leave_due`]) for the router to leave the
    /// spaceport once nothing shown over it holds the leave
    /// ([`FlightView::leave_if_due`]), which shows its message. The screen
    /// catches up with what the session told ([`SessionEvent`]): a
    /// move in flight lays out the system the ship is in afresh, even the
    /// same one, as the original kills its explosions and smoke, and how
    /// many fighters were abandoned, if any, is shown after a `Q`'s
    /// message; a move while landed shows once the ship takes off; and a
    /// `C`, `E` or `H` reads the player's sprite sheet afresh. Then the
    /// session's messages raised as it settled show at once, as a
    /// deadline missed on the take-off does, before a text dialog can
    /// pause flight; an arrival's waits for the arrival as on a tick.
    ///
    /// Docked, the settling is a run of docked scripts as the bar sees it
    /// ([`FlightView::stage_bar`]): a landing's `_MissionLandCheck`
    /// (@0x5fbeb) shows each `CompText`, `FailText`, `ShipDoneText`,
    /// `LoadCargText` and `DumpCargoText` over the last bar drawn, and the
    /// `C`, `E` and `H` that `OnSuccess`, `OnFailure`, `OnShipDone`, an
    /// `S`'s `OnAccept` and the crons of a success's days run redraw at
    /// their operators, ordered against those texts. With no dialog of
    /// its own, nothing redraws after the last text.
    pub fn settle_script(&mut self) {
        if self.sim.session.is_err() {
            return;
        }
        let settled = self.run_docked(Then::Nothing, |view| {
            view.sim
                .session
                .as_mut()
                .map(|session| session.settle_script(&*view.catalog, &mut view.sim.chance))
                .unwrap_or_default()
        });
        self.show_settled(settled, false);
    }

    /// Shows what a settling did ([`FlightView::settle_script`]), the
    /// screen caught up first; `laid_out` says the system was laid out
    /// afresh since the settling, before this catches up.
    fn show_settled(&mut self, settled: Settled, laid_out: bool) {
        let flying = self
            .sim
            .session
            .as_ref()
            .is_ok_and(|session| session.landed().is_none());
        let mut said: Vec<(String, u16)> = settled
            .message
            .into_iter()
            .map(|text| (text, SCRIPT_MESSAGE_TICKS))
            .collect();
        if (self.catch_up().system || laid_out) && flying {
            let notes = self.drain().take_fighter_notes();
            said.extend(notes.into_iter().map(|FighterNote::Abandoned(count)| {
                (fighters_abandoned_message(count), MESSAGE_TICKS)
            }));
        }
        self.say_all(&said);
        self.say_messages();
    }

    /// Plays `effect` for the ship having come out of a gate, caught up
    /// with as an arrival is ([`FlightView::arrived`]).
    fn came_through(&mut self, effect: JumpEffect) {
        self.held.clear();
        self.arrived();
        self.fade_in = Some(effect);
    }

    /// Presses the land key: requests clearance, lands, enters a gate, or
    /// shows why not. What the press said (the request's line, or "No
    /// response.") shows first, so a refusal in the same press replaces
    /// it. Over a cleared hypergate the course map opens offering its
    /// links, and over a wormhole the ship passes through at once.
    pub(super) fn land(&mut self) {
        let Ok(session) = &mut self.sim.session else {
            return;
        };
        let pressed = session.land();
        self.say_messages();
        let Ok(session) = &mut self.sim.session else {
            return;
        };
        match pressed {
            Ok(LandPress::Outcome(LandOutcome::Requested { .. })) => {}
            Ok(LandPress::Outcome(LandOutcome::Landed(stellar))) => {
                self.pending_landing = Some(stellar);
                self.blank_message();
                self.effects.clear();
                self.flashes.clear();
                self.held_alts.get_mut().clear();
                self.glows.clear();
                self.shot_looks.clear();
                self.redraw_status();
            }
            Ok(LandPress::AtGate {
                kind: GateKind::Hypergate,
                ..
            }) => match session.open_hypergate(&*self.catalog) {
                Ok(offered) => {
                    self.map.offer_gates(offered);
                    self.open_map();
                }
                Err(refusal) => self.gate_refused(Some(refusal)),
            },
            Ok(LandPress::AtGate {
                kind: GateKind::Wormhole,
                ..
            }) => match session.enter_wormhole(&*self.catalog, &mut self.sim.chance) {
                Ok(_) => self.came_through(JumpEffect::flash()),
                Err(refusal) => self.gate_refused(Some(refusal)),
            },
            Err(refusal) => {
                let stellar = match refusal {
                    LandingRefusal::TooFar { stellar, .. }
                    | LandingRefusal::TooFast { stellar, .. }
                    | LandingRefusal::NotLandable { stellar, .. }
                    | LandingRefusal::Denied { stellar, .. } => Some(stellar),
                    LandingRefusal::Jumping
                    | LandingRefusal::NoStellars
                    | LandingRefusal::NothingLandable
                    | LandingRefusal::Disabled
                    | LandingRefusal::Cloaked => None,
                };
                let gate = stellar.and_then(|stellar| session.gate_kind(stellar));
                let name = stellar.map_or("", |stellar| stellar_name(self.scene.as_ref(), stellar));
                let text = refusal_message(&refusal, gate, name);
                if let Some(text) = text {
                    self.say_for(text, landing_refusal_ticks(&refusal));
                }
            }
        }
    }

    /// Shows what the original says when entering a gate is refused, if
    /// anything: a cancelled hypergate jump, or a wormhole leading
    /// nowhere. A hypergate without links says nothing.
    fn gate_refused(&mut self, refusal: Option<GateRefusal>) {
        match refusal {
            Some(GateRefusal::Cancelled) => self.say_for(HYPERGATE_CANCELLED, GATE_REFUSAL_TICKS),
            Some(GateRefusal::NoExit) => {
                self.say_for(gate_refusal_message(GateKind::Wormhole), GATE_REFUSAL_TICKS);
            }
            Some(GateRefusal::NoLinks | GateRefusal::NotAtGate) | None => {}
        }
    }

    /// Closes the course map, abandoning any gesture on it, and goes back
    /// to flight. Closing the hypergate map enters the hypergate for the
    /// system picked on it: the ship comes out there, and the new system
    /// fades in from white ([`JumpEffect::emerging`], or flashes white
    /// with the Hyperspace Effects preference off); with no pick, the jump
    /// is cancelled ([`HYPERGATE_CANCELLED`]). Closing the course map cues
    /// the course's next hop ([`Session::cue_course`]), as the original's
    /// map does unless it is the hypergate's (`_DoSystemMap` @0x13c16),
    /// and beeps [`UiSound::MapClosed`], as `_HandlePlayer` does once
    /// `_DoSystemMap` returns (@0x6a85a). The hypergate map closes
    /// silently: `_PlayerEnterHypergate` plays no beep around it
    /// (@0x6385b).
    ///
    /// Docked, its close is `_DoSystemMap`'s in port: the whole bar is
    /// redrawn (`_RefreshSAT` @0x13bc9), showing the nav as the map left
    /// it, and the course is not cued (@0x13be2-0x13c0a skips
    /// `_CueNextHyperRouteDest` in port). Then the selection the map
    /// opened with is put back, the mode kept, with no redraw
    /// ([`Session::restore_map_selection`], `_DoSystemMap` @0x12349,
    /// @0x13bee-0x13c06), undoing a Clear Route. It beeps nothing: the
    /// [`UiSound::MapClosed`] beep is `_HandlePlayer`'s, around the map M
    /// opens in flight. A mission dialog's map beeps in flight neither.
    ///
    /// Then whoever opened it puts back what it saved: when the bar opened
    /// it ([`FlightView::open_bar_map`]), the nav the bar opened with
    /// ([`Session::restore_bar_nav`], `_DoBarDialog` @0x492b7-0x492d3),
    /// with no redraw, as `_UpdateAllPortDialogs` (@0x492d7) draws no
    /// status, so the nav area shows the map's selection until the port's
    /// next nav redraw, such as the bar's close; a port screen its own
    /// ([`FlightView::open_port_screen_map`]), with no redraw either: the
    /// exchange and the outfitter the nav they opened with, and the
    /// spaceport, the shipyard, the hire screen and the ship description
    /// the nav as the map opened; a mission dialog its own
    /// ([`FlightView::open_dialog_map`]): the offer's, the computer's and
    /// the briefing's the nav as the map opened, docked or in flight, the
    /// computer's then redrawing the nav area, and mission info's nothing.
    pub fn close_map(&mut self) {
        // `_SaveMapBordersState` for a change, whatever the map's mode,
        // unless docked (@0x13c21-0x13c3f).
        let docked = self.frame_now().docked;
        if !docked && self.map.borders_changed_since_open() {
            self.map_borders_save = Some(self.map.borders_shown());
        }
        self.map.cancel_pointer();
        self.map.release_keys();
        self.map.close_find();
        self.map_open = false;
        self.map_key_held = false;
        let opened = self.map_opened.take();
        if self.map.mode() != MapMode::Hypergate {
            if docked {
                self.redraw_status();
                if let Some(nav) = opened.and_then(|opened| opened.nav) {
                    self.sim.apply(FlightCommand::RestoreMapSelection(nav));
                }
                self.restore_opener_nav(opened);
                return;
            }
            let by_dialog =
                opened.is_some_and(|opened| matches!(opened.by, MapOpening::Dialog(..)));
            if !by_dialog {
                self.ui_sounds.push(UiSound::MapClosed);
            }
            self.sim.apply(FlightCommand::CueCourse);
            self.redraw_status();
            self.restore_opener_nav(opened);
            return;
        }
        self.redraw_status();
        let choice = self.map.take_gate_choice();
        let Ok(session) = &mut self.sim.session else {
            return;
        };
        match session.enter_hypergate(choice, &*self.catalog, &mut self.sim.chance) {
            Ok(_) => {
                let effect = JumpEffect::emerging(self.hyperspace_effects);
                self.came_through(effect);
            }
            Err(refusal) => self.gate_refused(Some(refusal)),
        }
    }

    /// Buys a ship named `name` as [`Session::buy_ship`] does, it and its
    /// hooks drawing on the flight's chance, and catches up with the
    /// change of ship it tells ([`SessionEvent::ShipChanged`]): the sprite
    /// sheet of the class the session then flies (the one bought, or the
    /// one its hooks changed it to) is read, once, so the new hull is
    /// drawn once it takes off; a refused purchase tells nothing and
    /// reads nothing, and a session that failed has no shipyard.
    ///
    /// A purchase redraws the radar, the nav area and the target panel,
    /// as the original's does inside the purchase (`_DrawStatus`
    /// @0x5f0b1, after marking nav, target, weapons and cargo,
    /// `_statusRedraw[0]`, `[6]`, `[3]` and `[2]`, @0x5f03a-0x5f04a), so a ship whose default outfits carry IFF
    /// shows it at once, and the nav area shows what the ship's
    /// `OnPurchase` (@0x5f00d) wrote, a landed `M` or `N`'s selection,
    /// whatever its operators' order, as it runs first.
    ///
    /// The gauges (`[1]`, `[4]`) are not marked. A `C`, `E` or `H` in the old
    /// class's `OnRetire` (@0x5ecff) or the new one's `OnPurchase`
    /// (@0x5f00d) that puts the ship under another `ïntf` redraws the
    /// whole bar at that operator, so the gauges show as they stood
    /// there, before the purchase's fill (@0x5f051-0x5f06b). Then the
    /// purchase's own `_LoadIntfResource` (@0x5f0ac) loads the class's
    /// bar and redraws the gauges only when the ship's `ïntf` is still
    /// another than the one loaded (`hud::interface_for`), as its
    /// whole-bar `_RedrawStatus` (@0x739bd) runs only for an ID other than
    /// the one loaded (`_currentIntf`, @0x73663-0x73672); otherwise they
    /// wait for the shipyard to close ([`FlightView::refresh_status`]).
    ///
    /// An `S` in either hook shows its mission's texts inside the
    /// purchase (`_EvalSetExp` @0x153ff → `_DoMissionAccept` @0x15407), so
    /// the hooks' redraws are ordered against them
    /// ([`FlightView::stage_bar`]): those before the first text redraw at
    /// once, each text shows over the last bar drawn and its close
    /// redraws as the text noted it, then at the hooks' operators after
    /// it; and the purchase's own load and draw come only after the last
    /// text has closed ([`Then::Purchase`]). So under the text the old
    /// class's bar shows, with the gauges from before the purchase.
    ///
    /// Before all that, the whole bar is redrawn as the name prompt
    /// closes on OK (`_RefreshSAT` @0x5eb73, right after
    /// `_EVTextInputDialog` returns @0x5eb6c and before the purchase
    /// @0x5ebc0), whatever the purchase then does. The app checks every
    /// refusal before the prompt opens ([`FlightView::ship_naming`]), so
    /// no refusal follows it there.
    pub fn buy_ship(&mut self, ship: ShipId, name: &str) -> Result<ShipPurchase, ShipRefusal> {
        self.redraw_status();
        let before = self.interface_now();
        let session = self
            .sim
            .session
            .as_mut()
            .map_err(|_| ShipRefusal::NoShipyard)?;
        let bought = session.buy_ship(ship, name, &mut self.sim.chance)?;
        let after = self.interface_now();
        let (changes, loaded) = self.interface_changes(before.as_ref());
        self.stage_bar(
            changes,
            Then::Purchase {
                reserves: after != loaded,
            },
        );
        self.catch_up();
        Ok(bought)
    }

    /// Buys or sells an outfit as [`Session::outfit`] does, it and its
    /// hook drawing on the flight's chance; a session that failed has no
    /// outfitter. The bar is redrawn after it
    /// (`FlightView::after_outfit_order`).
    pub fn outfit(&mut self, order: OutfitOrder) -> Result<(), OutfitRefusal> {
        self.outfit_counted(order, 1).map(|_| ())
    }

    /// Buys or sells up to `count` of an outfit as
    /// [`Session::outfit_counted`] does, on the flight's chance, giving
    /// how many went through; a session that failed has no outfitter.
    /// The bar is redrawn after it (`FlightView::after_outfit_order`).
    pub fn outfit_counted(&mut self, order: OutfitOrder, count: u32) -> Result<u32, OutfitRefusal> {
        let before = self.interface_now();
        let run = match &mut self.sim.session {
            Ok(session) => session.outfit_counted_run(order, count, &mut self.sim.chance),
            Err(_) => no_outfitter(),
        };
        self.after_outfit_order(before.as_ref(), run.button_passed);
        run.run.counted()
    }

    /// Buys or sells up to `count` of an outfit as
    /// [`Session::outfit_order`] does, on the flight's chance, giving the
    /// whole run and its alert; a session that failed has no outfitter,
    /// refused at the first pass with no alert. The bar is redrawn after
    /// it (`FlightView::after_outfit_order`).
    pub fn outfit_order(&mut self, order: OutfitOrder, count: u32) -> OutfitRun {
        let before = self.interface_now();
        let run = match &mut self.sim.session {
            Ok(session) => session.outfit_order(order, count, &mut self.sim.chance),
            Err(_) => no_outfitter(),
        };
        self.after_outfit_order(before.as_ref(), run.button_passed);
        run
    }

    /// Redraws the bar after an outfitter order, given the `ïntf` the
    /// ship flew under before it ([`FlightView::interface_now`]) and
    /// whether it passed its button's own check (`passed`, the run's
    /// [`OutfitRun::button_passed`]).
    ///
    /// The order's `OnPurchase` (`_EvalMissionBitSetString` @0x5c2cb) or
    /// `OnSell` (@0x5cff6) runs inside it. When its `C`, `E` or `H`
    /// (`_EvalSetExp` @0x15493-0x156c7) puts the ship under another
    /// `ïntf`, `_LoadIntfResource` (@0x156c7) redraws the whole bar at
    /// once (`_RedrawStatus(1)` @0x739bd), held gauges, nav and target
    /// included ([`FlightView::redraw_status`]). Otherwise only the radar
    /// is redrawn ([`FlightView::retake_radar`]). A refused order changes
    /// no ship, so it redraws only the radar. The redraw runs inside the
    /// order's call, before any frame is drawn.
    ///
    /// The redraw is at the `C`, `E` or `H` itself, as the original's is
    /// ([`FlightView::stage_bar`]): the nav and gauges
    /// show as they stood at the last operator that changed the `ïntf`,
    /// so an `M` or `N` after it in the same script (@0x156c7 → @0x739bd,
    /// then @0x156f2-0x15705) shows only at the port's next nav redraw,
    /// and one before it shows at once. An `ïntf` changed and changed
    /// back within one order still redraws, though the one after the
    /// order is the one before it.
    ///
    /// An `S` in the hook shows its mission's texts inside the order
    /// (`_EvalSetExp` @0x153ff → `_DoMissionAccept` @0x15407, each text
    /// closing with `_RefreshSAT` @0xa1f13), so the hook's redraws are
    /// ordered against them as every docked script's are
    /// ([`FlightView::stage_bar`]): the `C`, `E` and `H` before the first
    /// text redraw at once, each text shows over the last bar drawn and
    /// its close redraws the bar as the text noted it, then at the
    /// operators after it; and the order's own draw (@0x5c5a1, @0x5d328,
    /// after `_SystemInfoToShipStats` and `_SetupPortAvailableItems`)
    /// comes only after the last text has closed ([`Then::Outfit`]). That
    /// text's close has drawn the nav, letting go of the shipyard's mark,
    /// so the order's draw then redraws only the radar. No stock outfit
    /// reaches it: `oütf` 363's `S731` starts a mission with
    /// no `BriefText`.
    ///
    /// The buy's and the sell's own `_DrawStatus` (@0x5c5a1, @0x5d328)
    /// draw the nav too when the shipyard's restore marked it
    /// ([`FlightView::close_port_dialog`]), but only for an order that
    /// went through: a buy `_CanBuyOutfitItem` refuses (@0x5c087,
    /// @0x5c09f-0x5c0a2) jumps past the buy's draw to @0x5c5ca, and a
    /// sell with nothing to sell (the flag at 0xded1c, @0x5c5d5-0x5c5dd)
    /// past the sell's to @0x5d351. A run stopped after its first pass
    /// still reaches them, and so does a Cancel's count of 0 once the
    /// button's check passed, as the loop then runs no pass and falls
    /// through to the draw (`_DoQuantityDialog` @0x5c1aa, @0x5c2fc-0x5c2ff).
    fn after_outfit_order(&mut self, before: Option<&Result<i16, String>>, passed: bool) {
        let (changes, _) = self.interface_changes(before);
        self.stage_bar(changes, Then::Outfit { passed });
    }

    /// Assigns the ship captured, as [`Session::assign`] does, and says
    /// what it did; after "Use As My Ship" it is drawn where it is, not on
    /// its way from the old ship, and the change of ship the session tells
    /// is caught up with ([`SessionEvent::ShipChanged`]), reading the new
    /// ship's sprite sheet, once. `None` when no capture awaits its
    /// assignment.
    pub fn assign(&mut self, choice: Assignment) -> Option<Assigned> {
        let session = self.sim.session.as_mut().ok()?;
        let assigned = session.assign(choice, &mut self.sim.chance)?;
        if assigned == Assigned::MyShip {
            self.previous = *session.player();
            self.alpha = 0.0;
        }
        self.catch_up();
        self.say_for(assigned_message(assigned), assigned_ticks(assigned));
        Some(assigned)
    }

    /// Reads the player's sprite sheet afresh when the session flies a
    /// class other than the one it was read for, so a ship bought,
    /// captured or changed by a set expression is drawn in its own
    /// sprites; called only when the session tells the ship changed
    /// ([`FlightView::catch_up`]), which it may tell of the class it was.
    /// A sheet that cannot be read is kept against its class too, so it is
    /// not tried again. The status bar follows on its own
    /// ([`FlightView::follow_bar`]).
    fn follow_ship(&mut self) {
        let Ok(session) = &self.sim.session else {
            return;
        };
        let ship = session.ship();
        if self.sheet_ship != Some(ship) {
            self.sheet = self.catalog.ship_sheet(ship);
            self.sheet_ship = Some(ship);
        }
    }
}

/// The scene a pass's effects play in: around the player in `session`,
/// as its rulebook reads [`RuleKey::ShipDebris`], [`RuleKey::CombatCaps`]
/// and [`RuleKey::LoopedSoundWait`].
fn effects_scene(session: &Session) -> Scene {
    let rules = session.rules();
    Scene {
        player: point(session.player().position),
        debris: rules.source_for(RuleKey::ShipDebris) == RuleSource::Bible,
        caps: rules.source_for(RuleKey::CombatCaps) == RuleSource::Engine,
        looped: match rules.source_for(RuleKey::LoopedSoundWait) {
            RuleSource::Engine => LoopedSounds::Anywhere,
            RuleSource::Bible => LoopedSounds::PerFirer,
        },
    }
}

/// The course map, over the [`FlightSim`] port: its opening, drawing,
/// panels, input and Clear Route read the flight through its frame
/// ([`FlightView::frame_now`]) and [`MapSource`](crate::flight::sim_port::MapSource),
/// and send their commands through it. Its close stays concrete
/// ([`FlightView::close_map`]), as the hypergate entry it may run is still
/// the session's: [`FlightView::map_input_taken`] says when it is due.
impl<C, S: FlightSim> FlightView<C, S> {
    /// Shows the course as it is flown on the course map, from the
    /// current system ([`FlightFrame::course_shown`](nova_sim::flight_frame::FlightFrame::course_shown)).
    pub(super) fn show_course(&mut self) {
        let frame = self.frame_now();
        self.map.show_course(frame.system, &frame.course_shown);
    }

    /// The input `input` as the course map takes it: in its own
    /// 1024 x 768 frame, which a widened view over flight draws centred,
    /// moved only across ([`FlightView::map_offset`]).
    pub(super) fn map_frame_input(&self, input: &Input) -> Input {
        let offset = self.map_offset();
        input.translated(Point::new(-offset.x, 0.0))
    }

    /// [`FlightView::map_input`]'s work, with `close` the map's close
    /// ([`FlightView::close_map`] for a session's flight), run at once
    /// when the input closes the map ([`FlightView::map_input_taken`]).
    /// Then a click, a Tab cycle or a gate pick that changed the
    /// selection takes the panels again ([`FlightView::retake_map_panel`]),
    /// as the original's next `_MapDialogUpdate` draws them for its
    /// `_selSystem`. After an input that closed the map the panels are
    /// taken for nothing: the map isn't drawn until it opens again, which
    /// takes them.
    pub(super) fn map_input_then(&mut self, input: &Input, close: impl FnOnce(&mut Self)) {
        let selected = self.map.selected();
        let input = self.map_frame_input(input);
        if self.map_input_taken(&input) {
            close(self);
        }
        if self.map.selected() != selected {
            self.retake_map_panel();
        }
    }

    /// The open course map, drawn into `list`: its 1024 x 768 frame moved
    /// right by [`FlightView::map_offset`].
    pub(super) fn draw_open_map(&self, list: &mut DrawList) {
        let mut map = DrawList::new();
        self.map.draw(&mut map);
        list.extend_translated(&map, self.map_offset());
    }

    /// How far right the course map's 1024 x 768 frame is drawn, and its
    /// clicks taken: centred in the widened view over flight
    /// ([`classic_offset`]), and at 0 over the spaceport, which is
    /// classic, where the bar and the docked mission dialogs open it.
    pub(super) fn map_offset(&self) -> Point {
        let in_port = self.frame_now().docked;
        let x = if in_port {
            0.0
        } else {
            classic_offset(self.view.0)
        };
        Point::new(x, 0.0)
    }

    /// Lets go of the flight keys and shows the course map, with the
    /// system the original's `_DoSystemMap` selects as it opens
    /// ([`Session::map_preselection`]) selected, and the system selected
    /// in nav mode 3 drawn as its link. The hypergate map opens this way
    /// too, its pick still only an offered system.
    ///
    /// Clear Route is enabled off the hypergate map with a course
    /// (`showClearRouteButton`, @0x12396-0x123c3), and never on it.
    pub(super) fn open_map(&mut self) {
        self.open_map_as(MapOpening::Plain);
    }

    /// [`FlightView::open_map`], opened `by` whoever opens it (flight, the
    /// bar, a port screen or a mission dialog), which gives the map its
    /// marks ([`FlightView::open_dialog_map`]) and its close's restore, and
    /// the nav noted as it opens (`_DoSystemMap` @0x12349).
    pub(super) fn open_map_as(&mut self, by: MapOpening) {
        self.map_opened = Some(MapOpened {
            by,
            nav: self.opening_nav(),
        });
        self.held.clear();
        self.map_open = true;
        if let Some(sources) = self.sim.map_border_sources() {
            self.map.show_border_sources(sources);
        }
        self.map.opened();
        self.map.preselect(self.sim.map_preselection());
        self.map_missions = self.sim.map_mission_systems();
        self.map.set_clear_route(
            self.map.mode() != MapMode::Hypergate && self.frame_now().course_plotted,
        );
        self.redraw_map();
    }

    /// Takes the session's drawing onto the course map
    /// ([`Session::map_drawing`]), as `_MapDialogUpdate` runs `_DrawMap`:
    /// on the hypergate map with the gate's offers arrowed, each of which
    /// that draw marks known ([`Session::mark_known`], `_DrawMap`
    /// @0xf457-0xf45f). So the first draw of the hypergate map shows an
    /// offer the player knew nothing of without its oval, and the next one,
    /// at a Tab, a click or the course map opened again before the next
    /// rebuild point, shows it, as the original's next `_MapDialogUpdate`
    /// does. What the drawing was taken from is noted after the mark
    /// ([`MapKey`](nova_sim::flight_frame::MapKey)), so the mark itself
    /// doesn't take it again. The
    /// panels are taken with it ([`FlightView::retake_map_panel`]).
    pub(super) fn redraw_map(&mut self) {
        let offered = (self.map.mode() == MapMode::Hypergate).then(|| self.map.offered().to_vec());
        let marks = MapOpened::marks(self.map_opened.as_ref());
        let Some(drawing) = self
            .sim
            .map_drawing(offered.as_deref(), &self.map_missions, marks)
        else {
            return;
        };
        self.map.show_drawing(&drawing);
        if offered.is_some() {
            self.sim
                .apply(FlightCommand::MarkKnown(drawing.gate_arrows.clone()));
        }
        if let Some(sources) = self.sim.map_border_sources() {
            self.map.show_border_sources(sources);
        }
        self.map_seen = Some(self.frame_now().map_key);
        self.retake_map_panel();
    }

    /// Takes the session's info panels onto the course map
    /// ([`Session::map_panel`]) for the system it selects, as
    /// `_MapDialogUpdate` draws them after `_DrawMap`: on the hypergate
    /// map with the gate's offers, which title a destination.
    pub(super) fn retake_map_panel(&mut self) {
        let offered = (self.map.mode() == MapMode::Hypergate).then(|| self.map.offered());
        if let Some(panel) = self.sim.map_panel(self.map.selected(), offered) {
            self.map.show_panel(panel);
        }
    }

    /// Opens the course map on a press of [`MAP_KEY`] in flight, held from
    /// then on. `_HandlePlayer`'s `_beepSnd[2]` (@0x6a809) is the key's,
    /// not the map's: the hypergate's map opens without it.
    pub(super) fn open_map_by_key(&mut self) {
        self.ui_sounds.push(UiSound::MapOpened);
        self.open_map();
        self.map_key_held = true;
    }

    /// Opens the course map from the bar, as `_DoBarDialog`'s item 8 does
    /// (@0x492a3-0x492b2), over the spaceport: in
    /// [`MapMode::Course`](crate::galaxy::MapMode::Course), as the bar
    /// clears the hypergate flag first (@0x492aa-0x492af). Ours' map is
    /// in that mode whenever it is closed, as the hypergate map's close
    /// takes its pick ([`GalaxyMap::take_gate_choice`](crate::galaxy::GalaxyMap::take_gate_choice)). Its close is
    /// [`FlightView::close_map`]'s in port.
    pub fn open_bar_map(&mut self) {
        self.open_map_as(MapOpening::Bar);
    }

    /// [`FlightView::map_input`]'s input, translated into the map's frame,
    /// before the panels follow the selection: whether it closes the map,
    /// which the caller then does ([`FlightView::close_map`]), at once.
    /// A key-down the filter doesn't take with [`MAP_KEY`] held closes it
    /// before anything else; Return or Done after the rest.
    pub(super) fn map_input_taken(&mut self, input: &Input) -> bool {
        if let Input::Key {
            key: MAP_KEY,
            pressed,
            ..
        } = *input
        {
            self.map_key_held = pressed;
        }
        if let Input::Key {
            key,
            pressed: true,
            repeat: false,
        } = *input
            && self.map_key_held
            && !self.map.finding()
            && !self.map_filter_takes(key)
        {
            return true;
        }
        self.map.input(input);
        self.ui_sounds.extend(self.map.take_ui_sounds());
        if let Some(query) = self.map.take_find()
            && let Some(found) = self.sim.find_map_system(&query)
        {
            self.map.show_search(found);
            self.ui_sounds.extend(self.map.take_ui_sounds());
        }
        if let Some(press) = self.map.take_press() {
            self.answer_map_press(&press);
        }
        if self.map.take_clear_route() {
            self.clear_route();
        }
        if let Some(cycle) = self.map.take_cycle() {
            self.cycle_map(cycle);
        }
        self.map.take_close()
    }

    /// Answers a press on the map that hit systems
    /// ([`GalaxyMap::take_press`](crate::galaxy::GalaxyMap::take_press)) as
    /// `_DoSystemMap` takes item 3 ([`Session::map_click`]): the hypergate
    /// map passes the systems the gate offers, and every map the mission
    /// systems taken as it opened. With no session, or no hit accepted,
    /// the map drags. An accepted press draws the map again, as the
    /// original invalidates its items 2, 3 and 6 (@0x1315c-0x13259), so the
    /// hypergate map's offers show known; the course changed is shown, and
    /// the nav area is redrawn at once when the click wrote it
    /// (`_DrawStatus`), as [`FlightView::cycle_map`] does.
    fn answer_map_press(&mut self, press: &PendingPress) {
        let offered = (self.map.mode() == MapMode::Hypergate).then(|| self.map.offered().to_vec());
        let answer = self.sim.apply(FlightCommand::MapClick(MapClick {
            hits: press.hits.clone(),
            shift: press.shift,
            hypergate: offered,
            missions: self.map_missions.clone(),
            green: MapOpened::marks(self.map_opened.as_ref()).green,
        }));
        let clicked = match answer {
            Answer::Clicked(clicked) => Some(clicked),
            _ => None,
        };
        self.map.answer_press(clicked.map(|clicked| PressAnswer {
            selected: clicked.selected,
            gate_pick: clicked.gate_pick,
            clear_route: clicked.clear_route,
        }));
        let Some(clicked) = clicked else {
            return;
        };
        if clicked.course_changed {
            self.show_course();
        }
        self.redraw_map();
        if clicked.nav_redraw {
            self.retake_nav();
        }
    }

    /// Clear Route, from its button or C (`_DoSystemMap` item 8,
    /// @0x12c69-0x12dbf): the session's course cleared and its selection
    /// let go ([`Session::clear_route`]), Clear Route disabled and the
    /// hypergate map's pick dropped
    /// ([`GalaxyMap::route_cleared`](crate::galaxy::GalaxyMap::route_cleared)),
    /// the map drawn again and the nav area redrawn at once (`_DrawStatus`
    /// @0x12dba).
    fn clear_route(&mut self) {
        self.map.route_cleared();
        if !self.sim.apply(FlightCommand::ClearRoute).is_refused() {
            self.show_course();
        }
        self.redraw_map();
        self.retake_nav();
    }

    /// Whether `_MapFilter` takes `key` before its map-key close, which a
    /// key it takes never reaches (@0x13cdd-0x144d6): Return, Enter,
    /// Escape, Tab and `\` (off a planet, @0x13d20), Space, B, C, `-`,
    /// `=` and `+` (the keypad's too), the arrows, and F and S. The keys
    /// later phases port are taken already, so none of them closes the
    /// map. A modifier is taken too: a Mac sends no key-down for one
    /// alone, so the filter never sees it.
    fn map_filter_takes(&self, key: Key) -> bool {
        let docked = self.frame_now().docked;
        match key {
            Key::Shift
            | Key::Control
            | Key::Alt
            | Key::Command
            | Key::CapsLock
            | Key::Enter
            | Key::Escape
            | Key::Space
            | Key::Left
            | Key::Right
            | Key::Up
            | Key::Down => true,
            Key::Tab => !docked,
            Key::Char(c) => match c.to_ascii_lowercase() {
                '\\' => !docked,
                'b' | 'c' | '-' | '=' | '+' | 'f' | 's' => true,
                _ => false,
            },
            _ => false,
        }
    }

    /// A Tab or `\` on the map ([`MapCycle`]), as `_MapFilter` takes it
    /// off a planet (@0x13d0c-0x140d8). On the hypergate map, the map's
    /// selection steps to the next of the pending hypergate's
    /// destinations ([`Session::hypergate_cycle`],
    /// [`next_gate_pick`](nova_sim::gate::next_gate_pick)), back with
    /// Shift, held as it is so a hidden one steps on too; no nav mode is
    /// written and the nav area isn't redrawn, as the original's mode 4 is
    /// never drawn (@0x13dd8-0x13ebd). A pick redraws the map's item 3
    /// ([`GalaxyMap::redraw`](crate::galaxy::GalaxyMap::redraw),
    /// @0x13e6f); with no destination to pick nothing is invalidated
    /// (`jle` @0x13daf). On the course map, the session cycles the
    /// hyperspace selection ([`Session::cycle_map_selection`]); the map
    /// then draws the new selected link and selects the system the
    /// original's `_selSystem` names (@0x1408c-0x140d8,
    /// [`Session::map_preselection`]), item 3 is redrawn whether a link
    /// was picked or none (@0x14032), and the nav area is redrawn at
    /// once, as the filter runs `_DrawStatus` with nav marked
    /// (`_statusRedraw[0]`, @0x13fb2-0x1406e). Docked, as on the bar's
    /// map, nothing changes.
    fn cycle_map(&mut self, cycle: MapCycle) {
        if self.map.mode() == MapMode::Hypergate {
            let destinations = self.sim.hypergate_cycle();
            if let Some(pick) = next_gate_pick(&destinations, self.map.selected(), cycle.back) {
                self.map.select_gate(pick);
                self.map.redraw();
            }
            self.redraw_map();
            return;
        }
        if !self
            .sim
            .apply(FlightCommand::CycleMapSelection)
            .is_refused()
        {
            self.map.preselect(self.sim.map_preselection());
            self.map.redraw();
            self.redraw_map();
            self.retake_nav();
        }
    }
}
