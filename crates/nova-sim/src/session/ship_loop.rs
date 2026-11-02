//! `_DoPlayGameWork`'s ship loop (@0x4445c-0x444c4), as the original runs
//! it: for each slot 1-63 in order, `_HandleShip` (@0x44483) when the slot
//! is active and in the player's system, then `_HandleShipDisplay`
//! (@0x444ad) ([`Session::ship_loop`]).
//!
//! **Each NPC in slot order** first runs `_HandleShip`'s slot test
//! (@0x33865-0x33888,
//! [`let_go_at_step`](crate::traffic::Traffic::let_go_at_step)): a target
//! whose slot is inactive is let go, whatever else the pass does. Then it
//! flies (its position step @0x34274-0x342aa,
//! after `_AdjustInertialessShipVelocity` @0x3425f; the traffic's
//! [`fly_npc`](crate::traffic::Traffic::fly_npc)), then takes its step:
//! its reloads, regeneration (@0x34424), fire (@0x34ffd) and death timer
//! (@0x35083); then, for a dying NPC, `_HandleShip`'s dying block
//! (@0x35080-0x3557b): its dying branch, which takes a mission escort it
//! still is out of the formation once its timer is set
//! ([`Session::lose_dying_mission_escort`], @0x350a8-0x35134), then its
//! escape pods and escape-ship roll (see the `npc_escape` module); then
//! its display, which may make its final
//! explosion ([`Combat::handle_npc`]; for a dying NPC
//! [`Combat::step_npc`], the dying block, then [`Combat::display_npc`]);
//! then, from the fight's record of that display, its frame: its sets,
//! lights and ion tint, and its berth's blink timer
//! ([`Space::display_npc_frames`], see the `ship_display` module). The
//! loop returns the NPCs it displayed, for the pass's tail to step those
//! it didn't reach ([`Space::display_undisplayed`]). So:
//!
//! - **Each NPC's frame** is this pass's from its display on: the deadly
//!   stellars (@0x4450e) read it, and a later slot's shockwave or fire
//!   that disables or ionizes it shows from the next frame, as in the
//!   original. One its own display destroys steps once, as disabled.
//! - **A final explosion's push** (`_DamageShip` @0x2da89) reaches each
//!   ship of a later slot where it still stands, and that ship's own
//!   flight, later in the loop, moves it by the pushed velocity: it is
//!   drawn pushed that frame. A ship of an earlier slot has moved; it
//!   moves by the push on the next pass.
//! - **Each NPC's fire** reads the ships of earlier slots where their own
//!   flight left them, and those of later slots where they stood.
//! - **A slot's let-go** reads the ships as the loop has left them: a ship
//!   of a later slot than a wreck or a gate entry lets go in that pass, an
//!   earlier one at its step in the next, and its idle bays hold from there
//!   (`_HandleShip` @0x3390a-0x3394e). A ship that jumped out or a fighter
//!   that docked left at its slot of the AI loop, before the ship loop, so
//!   every slot, an earlier one too, lets go of it at its step, before its
//!   fire (see the traffic's `dispatch_leave` and `gone` modules).
//! - **The let-go before the flight** is the original's order, but the
//!   ship flies the same either way: a flight steers by the ship its goal
//!   names and reads no target (+0x70), and the fly half's own reads of it
//!   ([`Traffic::begin_fly`](crate::traffic::Traffic::begin_fly)) come
//!   before every slot's test. A ship whose high level was skipped steers
//!   at a gone target's record by the note its think took, which the
//!   let-go doesn't clear (the traffic's `skipped_high_level`).
//! - **A ship whose flight ends it** (it enters a gate, or a mission ship
//!   leaves as it lands) leaves at once: its slot takes no step, and no
//!   later slot's fire or shockwave sees it.
//! - **An escape ship's fighter** (`_LaunchFighter` @0x354ea, its slot
//!   active at once @0x3c90b, @0x3d680) launched into a slot above its
//!   carrier's is reached later in the same loop, as the loop reads each
//!   next NPC afresh: as `_AIDispatch` ran before the loop it has had no
//!   low level, so it moves by its launch velocity with no thrust, turning
//!   toward its berth's last heading, the words its spawn left
//!   ([`Traffic::coast_npc`](crate::traffic::Traffic::coast_npc)), takes
//!   its step and is displayed, so it is drawn on its launch frame. One
//!   launched below waits for the next pass. Its carrier's display comes
//!   after the launch, so a final explosion on that tick reaches it. A
//!   wreck whose display ran earlier in the loop has freed its slot
//!   (@0x2dee0), though the take-out comes after the loop, so the launch
//!   may take it (@0x3c8c6-0x3c8e9); every ship whose step hasn't come and
//!   whose target names the old ship then fights the newcomer
//!   ([`Traffic::set_ship_loop`](crate::traffic::Traffic::set_ship_loop)).
//!
//! Each flight flies the word its low level left at its slot of the AI
//! loop, which read the other ships there, as the original's steering is
//! `_AIDispatch`'s (`_LowLevelAIHandler`), before the loop moves any ship;
//! `_HandleShip` itself reads no other ship's position. Each slot's hits are settled at its step
//! ([`Session::land_phase`]), closer to `_DamageShip` settling each as it
//! lands.
//!
//! [`Combat::handle_npc`]: crate::combat::Combat::handle_npc
//! [`Combat::step_npc`]: crate::combat::Combat::step_npc
//! [`Combat::display_npc`]: crate::combat::Combat::display_npc

use std::collections::BTreeSet;

use super::game_work::PassStart;
use super::npc_escape::{AtStep, escape_may_run};
use super::{Session, Setup, Space, Statics, TrafficMarks};
use crate::ai::Behaviour;
use crate::chance::Chance;
use crate::combat::{GamePass, Rules, ShipRef};
use crate::traffic::npc::NpcId;
use crate::traffic::{FlyHalf, FlyPass};

/// How the traffic moves in a pass's ship loop.
pub(super) enum Motion {
    /// The advance pass's fly half, read against the marks its think
    /// left.
    Fly(TrafficMarks),
    /// None: the advance pass populated the system, whose ships move from
    /// the next.
    Populated,
    /// Double time's second pass's fly half.
    Again,
}

impl Session {
    /// `_DoPlayGameWork`'s ship loop on `pass` by `rules`, the traffic
    /// moving as `motion` says, drawing on `chance` (see the module docs):
    /// every NPC's stray provocation eases (@0x35374-0x353b2); then, slot
    /// by slot, each NPC lets go of a target whose slot is inactive
    /// ([`Traffic::let_go_at_step`](crate::traffic::Traffic::let_go_at_step)),
    /// flies and takes its step, a dying one running its
    /// escape pods and escape-ship roll against its condition as the pass
    /// `start`ed (see the `npc_escape` module), the words the pass's stray
    /// beams stepped settled first, between its step and its display, and
    /// each one displayed has its frame stepped; then what the fly half
    /// leaves: its end, and the session's work after it; the ships laid
    /// out ([`Session::lay_out_displayed`]); every NPC's cloak fade steps
    /// ([`Traffic::step_fades`](crate::traffic::Traffic::step_fades)); and
    /// the pick fired is cleared or kept
    /// ([`Traffic::after_firing`](crate::traffic::Traffic::after_firing)).
    /// The NPCs it displayed.
    ///
    /// A fighter docks, and goes aboard, at its own slot of the AI loop
    /// before this ([`Session::dispatch_npcs`], [`Session::dock_fighter`]),
    /// as `_LowLevelAIHandler` calls `_AIAddFighterToParent` there
    /// (@0x87fef): the player's leaves the fleet before any step settles
    /// its hits, whose escorts lost ([`Session::lose_escorts`]) would
    /// otherwise take the fighter gone from the system for a loss and make
    /// a save due.
    ///
    /// The rest of what the departures leave the session (each mission
    /// ship that left raising its fate, the persons' offers, the fighters
    /// orphaned, the watched ships observed), the fly half's end
    /// (the docks started, the repairs queued, the waits counted down, the
    /// followers released, the fuel), the docks' and repairs' work on the
    /// line ([`Session::settle_line_queue`]), the reinforcement clock and
    /// the gates' sprites all settle after the loop, where the original
    /// runs them in each NPC's own `_HandleShip` or `_AIDispatch`
    /// (`_HandleReinforcements` itself comes after the loop, @0x44504).
    /// No step in the loop reads them, and all settle before the frame is
    /// drawn. An escort of the player's fleet that lands, jumps or enters a
    /// gate settles nothing after the loop: the next hits settled lose it
    /// ([`Session::lose_escorts`]), with the same save due wherever they
    /// fall.
    pub(super) fn ship_loop(
        &mut self,
        behaviour: &(impl Behaviour + ?Sized),
        start: &PassStart,
        motion: Motion,
        pass: GamePass,
        rules: Rules,
        chance: &mut dyn Chance,
    ) -> BTreeSet<NpcId> {
        self.space.traffic.ease_stray(self.speed_mult());
        let again_marks = matches!(motion, Motion::Again)
            .then(|| self.space.mission_marks())
            .filter(|_| !self.space.traffic_due);
        let mut fly = match &motion {
            Motion::Fly(_) => Some(self.begin_fly(&BTreeSet::new(), FlyHalf::Advance)),
            Motion::Again if again_marks.is_some() => {
                Some(self.begin_fly(&BTreeSet::new(), FlyHalf::Again))
            }
            Motion::Again | Motion::Populated => None,
        };
        let mut shown = vec![ShipRef::Player];
        let mut displayed = BTreeSet::new();
        let mut after = None;
        // A wreck frees its slot at its display, and an escape ship's
        // launch later in the loop may take it (see the module docs).
        self.space.traffic.set_ship_loop(true);
        while let Some((berth, id)) = self.space.next_in_loop(after) {
            after = Some(berth);
            // `_HandleShip`'s slot test (@0x33865) comes before its move
            // (@0x34274). No flight reads the target (see the module docs).
            let player_active = self.ship.condition.active();
            self.space.traffic.let_go_at_step(id, player_active);
            if let Some(fly) = fly.as_mut() {
                self.fly_in_loop(fly, id, chance);
            }
            let Some(npc) = self.space.npc(id) else {
                continue;
            };
            let at_step = AtStep::of(npc);
            let ship = ShipRef::Npc(id);
            let was = start
                .condition_of(id)
                .filter(|&was| escape_may_run(was, at_step));
            let Some(was) = was else {
                self.land_phase(
                    behaviour,
                    pass,
                    rules,
                    chance,
                    |combat, fighters, arena, chance| {
                        combat.handle_npc(fighters, arena, ship, &mut shown, chance);
                    },
                );
                self.space.step_displayed_npc(
                    id,
                    &mut displayed,
                    &self.statics,
                    &self.setup,
                    chance,
                );
                continue;
            };
            // `_HandleShip`'s dying block (@0x35080-0x3557b) comes between
            // its death timer (@0x35083) and its display (@0x444ad).
            self.land_phase(
                behaviour,
                pass,
                rules,
                chance,
                |combat, fighters, arena, chance| {
                    combat.step_npc(fighters, arena, ship, chance);
                },
            );
            self.step_stray_words(chance);
            // Its dying branch (@0x350a8-0x35134), before its pods.
            self.lose_dying_mission_escort(id, was);
            self.npc_escape(was, at_step, chance);
            self.land_phase(
                behaviour,
                pass,
                rules,
                chance,
                |combat, fighters, arena, _| {
                    combat.display_npc(fighters, arena, ship, &mut shown);
                },
            );
            self.space
                .step_displayed_npc(id, &mut displayed, &self.statics, &self.setup, chance);
        }
        self.space.traffic.set_ship_loop(false);
        if let Some(fly) = fly {
            self.space.traffic.end_fly(fly);
        }
        match motion {
            Motion::Fly(marks) => {
                self.after_traffic_motion(&marks, &mut &mut *chance);
                // The docks and repairs the fly half made, on the line.
                self.settle_line_queue(rules.law, chance);
                self.traffic_tail(&mut &mut *chance);
            }
            Motion::Populated => {
                self.traffic_tail(&mut &mut *chance);
            }
            Motion::Again => {
                if let Some(marks) = again_marks {
                    self.mission_ships_left(&marks);
                }
            }
        }
        self.lay_out_displayed();
        self.space.traffic.step_fades();
        self.space.traffic.after_firing(&self.statics.arsenal);
        displayed
    }

    /// NPC `id`'s flight in the fly half `fly`, drawing on `chance`: its
    /// flight; or, for one launched since the half began, its spawn's word
    /// ([`Traffic::coast_npc`](crate::traffic::Traffic::coast_npc)). No
    /// flight docks a fighter: it docks at its slot of the AI loop
    /// ([`Session::dispatch_npcs`]).
    fn fly_in_loop(&mut self, fly: &mut FlyPass, id: NpcId, chance: &mut dyn Chance) {
        if !fly.order().contains(&id) {
            self.step_traffic(|traffic, world| traffic.coast_npc(fly, id, &world, &mut *chance));
            return;
        }
        self.step_traffic(|traffic, world| traffic.fly_npc(fly, id, &world, &mut *chance));
    }

    /// Begins the traffic's fly half of `half`, the NPCs in `still`
    /// keeping their place, in the world it flies in.
    fn begin_fly(&mut self, still: &BTreeSet<NpcId>, half: FlyHalf) -> FlyPass {
        self.step_traffic(|traffic, world| traffic.begin_fly(&world, still, half))
    }
}

impl Space {
    /// NPC `id`'s frames, lights and ion tint stepped right after its
    /// display in the ship loop (`_HandleShipDisplay` @0x444ad), read as
    /// that display's `_IsDisabled` (@0x2b61f), and `id` noted in
    /// `displayed`, drawing on `chance`; nothing for an NPC its display
    /// didn't reach, already destroyed (the original's inactive-slot
    /// branch, @0x2b522-0x2b60a). The fight's display (the settle, then
    /// the dying block) ran before it, where the original's set step and
    /// lights (@0x2b7d8, @0x2c2ea) come between them: reading the record
    /// makes the two equal, so a ship its own display destroys steps as
    /// disabled.
    fn step_displayed_npc(
        &mut self,
        id: NpcId,
        displayed: &mut BTreeSet<NpcId>,
        statics: &Statics,
        setup: &Setup,
        chance: &mut dyn Chance,
    ) {
        let Some(shown) = self.combat.displayed(ShipRef::Npc(id)) else {
            return;
        };
        displayed.insert(id);
        self.display_npc_frames(id, shown.disabled, statics, setup, chance);
    }

    /// The NPC in the lowest berth above `after` (any, with none), and
    /// its berth, read afresh from the traffic: so the loop reaches a
    /// fighter launched into a berth above the one it is at, and passes
    /// one launched below.
    fn next_in_loop(&self, after: Option<u32>) -> Option<(u32, NpcId)> {
        self.traffic
            .npcs()
            .iter()
            .map(|npc| (npc.berth, npc.id))
            .filter(|&(berth, _)| after.is_none_or(|after| berth > after))
            .min()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::bay::Carrier;
    use crate::catalog::WeaponRecord;
    use crate::chance::{Chance, NeverFires};
    use crate::combat::armament::Trigger;
    use crate::combat::hull::Condition;
    use crate::combat::{CombatEvent, Rules, ShipRef};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::rulebook::RuleSource;
    use crate::session::Session;
    use crate::session::fighters::tests::{VIPER, carrier_npc};
    use crate::session::tests::{FIRE, armed, blaster, facing_an_npc};
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, FakePilotCatalog, npc};
    use crate::traffic::autopilot::Outcome;
    use crate::traffic::npc::{Mode, Npc, NpcId};

    /// Every NPC keeps its goal and holds no trigger.
    #[derive(Debug)]
    struct Keep;

    impl Behaviour for Keep {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }
    }

    /// An advance pass of `session`, its NPCs as `behaviour` says.
    fn pass(session: &mut Session, catalog: &FakePilotCatalog, behaviour: &impl Behaviour) {
        let rescued = session.game_work(
            Controls::default(),
            catalog,
            behaviour,
            Rules::default(),
            &mut NeverFires,
        );
        assert!(!rescued);
    }

    /// Double time's second pass of `session`.
    fn again(session: &mut Session) {
        session.tick_again(
            Controls::default(),
            &Keep,
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// `catalog`'s session in flight, its traffic set up and emptied.
    fn empty(catalog: &FakePilotCatalog) -> Session {
        let mut session = facing_an_npc(catalog, 180);
        session.space.traffic.remove(NpcId(0));
        session
    }

    /// An idle NPC of mass 40 at berth `berth`, at `at` moving at
    /// `velocity`, in `session`.
    fn place(session: &mut Session, berth: u32, at: Vec2, velocity: Vec2) -> NpcId {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.hull.mass = 40.0;
        ship.state.position = at;
        ship.state.velocity = velocity;
        session.space.traffic.add_npc_at(ship, berth)
    }

    /// A ship of mass 200 at berth `berth` at `at`, at rest, breaking up
    /// with its timer at 2: its own step takes it to 1, and its display
    /// makes its final explosion, a shockwave of 32 over 65 pixels.
    fn wreck(session: &mut Session, berth: u32, at: Vec2) -> NpcId {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.hull.mass = 200.0;
        ship.hull.death_delay = 5;
        ship.reserves.armor.now = 0.0;
        ship.condition = Condition::Dying {
            ticks_left: 2,
            starting: false,
        };
        ship.state.position = at;
        session.space.traffic.add_npc_at(ship, berth)
    }

    fn state(session: &Session, id: NpcId) -> crate::flight::ShipState {
        session.space.npc(id).expect("there").state
    }

    const WRECKED: Vec2 = Vec2::new(600.0, 0.0);

    /// A lit square wave: lit for 1 tick, twice, then a gap of 20.
    const SQUARE: crate::blink::Blink = crate::blink::Blink {
        mode: 1,
        a: 4,
        b: 1,
        c: 2,
        d: 20,
    };

    /// An idle NPC at berth `berth` at `at`, of a 0x0040 class whose
    /// lights burn steady: drawn lit at once, and dark while disabled.
    fn steady_lights(session: &mut Session, berth: u32, at: Vec2) -> NpcId {
        let id = place(session, berth, at, Vec2::ZERO);
        let ship = session.space.npc_mut(id).expect("there");
        ship.hull.frames.blink = crate::blink::Blink::STEADY;
        ship.hull.frames.lights = true;
        ship.hull.frames.flags = 0x0040;
        id
    }

    #[test]
    fn an_npc_disabled_at_its_loop_display_steps_its_lights_as_disabled() {
        // Each NPC's lights (@0x2c2ea) read the `_IsDisabled` its own
        // display kept (@0x2b61f, tested @0x2c585): a 0x0040 class's go
        // out while it is disabled, and burn while it is not. For a ship
        // that stays in the system the fight's record and its condition
        // after the display agree; they differ only for one the display
        // destroyed, whose stepped lights and sets go with it at the
        // take-out, its berth's blink timer stepping alike either way.
        let catalog = armed();
        let mut session = empty(&catalog);
        let downed = steady_lights(&mut session, 1, WRECKED);
        let well = steady_lights(&mut session, 2, Vec2::new(-600.0, 0.0));
        crate::fixture::disable(&mut session, downed);
        pass(&mut session, &catalog, &Keep);
        let lights = |id| session.space.npc(id).expect("there").display.drawn.lights;
        assert_eq!(
            session.space.npc(downed).expect("there").condition,
            Condition::Disabled
        );
        assert_eq!(lights(downed), None, "out while disabled");
        assert_eq!(lights(well), Some(32), "steady");
    }

    #[test]
    fn a_wreck_its_display_explodes_steps_its_lights_and_leaves_its_berth_the_timer() {
        // `_HandleShipDisplay` runs the lights (@0x2c2ea) before its dying
        // block explodes the ship and frees its slot (@0x2d7ef, @0x2ded6),
        // and nothing clears +0xc8ec at the freeing: the next ship brought
        // into the slot blinks on the timer that final step left, one step
        // on from the pass before.
        let catalog = armed();
        let mut session = empty(&catalog);
        let gone = wreck(&mut session, 1, WRECKED);
        let ship = session.space.npc_mut(gone).expect("there");
        ship.hull.frames.blink = SQUARE;
        ship.hull.frames.lights = true;
        ship.hull.frames.flags = 0x0040;
        ship.display.frames.lights.intensity = 32.0;
        ship.display.frames.lights.timer = 5.0;
        pass(&mut session, &catalog, &Keep);
        assert!(session.space.npc(gone).is_none(), "exploded and taken out");
        let next = session
            .space
            .traffic
            .add_npc_at(npc(0, ShipStats::new(FAST, &[])), 1);
        let timer = session
            .space
            .npc(next)
            .expect("there")
            .display
            .frames
            .lights
            .timer;
        assert_eq!(timer, 4.0, "5, then the final step's tick");
    }

    /// Beyond 1000 of the centre, where goal 2's block keeps a ship at rest
    /// on sub 4 (@0x8dd99-0x8de57).
    const OUTSIDE: Vec2 = Vec2::new(1600.0, 0.0);

    #[test]
    fn a_later_slot_is_moved_by_a_wrecks_push_in_the_explosion_pass() {
        // Slot 1's display pushes slot 2 (`_DamageShip` @0x2da89) before
        // slot 2's own `_HandleShip` moves it (@0x34274-0x342aa), on either
        // pass.
        let catalog = armed();
        for second_pass in [false, true] {
            let mut session = empty(&catalog);
            let gone = wreck(&mut session, 1, WRECKED);
            let at = WRECKED + Vec2::new(40.0, 0.0);
            let pushed = place(&mut session, 2, at, Vec2::ZERO);
            if second_pass {
                again(&mut session);
            } else {
                pass(&mut session, &catalog, &Keep);
            }
            assert!(session.space.npc(gone).is_none(), "{second_pass}: exploded");
            let after = state(&session, pushed);
            assert!(after.velocity.x > 0.0, "{second_pass}: {after:?}");
            assert_eq!(after.position, at + after.velocity, "{second_pass}");
        }
    }

    #[test]
    fn an_earlier_slot_is_pushed_and_moves_by_it_on_the_next_pass() {
        // Slot 1 has moved already when slot 2's display pushes it.
        let catalog = armed();
        let mut session = empty(&catalog);
        let at = WRECKED + Vec2::new(40.0, 0.0);
        let pushed = place(&mut session, 1, at, Vec2::ZERO);
        wreck(&mut session, 2, WRECKED);
        pass(&mut session, &catalog, &Keep);
        let after = state(&session, pushed);
        assert_eq!(after.position, at, "not yet");
        assert!(after.velocity.x > 0.0, "{after:?}");
        pass(&mut session, &catalog, &Keep);
        assert!(state(&session, pushed).position.x > at.x, "the next pass");
    }

    /// Whether NPC `id` was hit by the pass's shockwave.
    fn hit(events: &[CombatEvent], id: NpcId) -> bool {
        events.iter().any(|event| {
            matches!(event, CombatEvent::Hit { ship, weapon: None, .. } if *ship == ShipRef::Npc(id))
        })
    }

    #[test]
    fn a_later_slot_flying_into_the_blast_this_pass_is_not_reached() {
        // 66 pixels out, beyond the 65 the shockwave reaches, flying in at
        // 2 a tick: a later slot still stands outside as the wreck's
        // display strikes (@0x2d8ef-0x2da9c); an earlier one has flown in.
        let catalog = armed();
        for (mover_berth, wreck_berth, reached) in [(2, 1, false), (1, 2, true)] {
            let mut session = empty(&catalog);
            let mover = place(
                &mut session,
                mover_berth,
                WRECKED + Vec2::new(66.0, 0.0),
                Vec2::new(-2.0, 0.0),
            );
            wreck(&mut session, wreck_berth, WRECKED);
            session.take_combat_events();
            pass(&mut session, &catalog, &Keep);
            assert!(
                state(&session, mover).position.x < WRECKED.x + 65.0,
                "flown in"
            );
            let events = session.take_combat_events();
            assert_eq!(hit(&events, mover), reached, "berth {mover_berth}");
        }
    }

    /// Fires its primary at NPC `at`.
    #[derive(Debug)]
    struct FiringAt(NpcId);

    impl Behaviour for FiringAt {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }

        fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
            if npc.id == self.0 {
                Trigger::default()
            } else {
                FIRE
            }
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            (npc.id != self.0).then_some(ShipRef::Npc(self.0))
        }
    }

    #[test]
    fn an_npcs_fire_reads_a_later_slot_where_it_stood() {
        // `_FireAIShipWeapon` (@0x34ffd) aims at the target's slot as the
        // loop left it: an earlier slot moved, a later one not yet. Both
        // ships move alike, so the turret's lead reads only where the
        // target is.
        let catalog = FakePilotCatalog {
            weapons: vec![WeaponRecord {
                guidance: 4,
                ..blaster()
            }],
            ..armed()
        };
        let heading = |shooter_berth: u32, target_berth: u32, moving: bool| {
            let mut session = facing_an_npc(&catalog, 180);
            let velocity = if moving {
                Vec2::new(2.0, 0.0)
            } else {
                Vec2::ZERO
            };
            let shooter = &mut session.space.traffic.npcs_mut()[0];
            shooter.berth = shooter_berth;
            shooter.state.position = WRECKED;
            shooter.state.velocity = velocity;
            let shooter = shooter.id;
            let target = place(
                &mut session,
                target_berth,
                WRECKED + Vec2::new(0.0, -40.0),
                velocity,
            );
            pass(&mut session, &catalog, &FiringAt(target));
            session
                .shots()
                .iter()
                .find(|shot| shot.firer == ShipRef::Npc(shooter))
                .map(|shot| shot.heading)
                .expect("fired")
        };
        let still = heading(1, 2, false);
        assert_eq!(heading(2, 1, true), still, "both moved");
        assert_ne!(heading(1, 2, true), still, "where it stood");
    }

    /// [`FiringAt`] NPC `at`, which keeps its own goal.
    #[derive(Debug)]
    struct FiringAtKeeping(NpcId);

    impl Behaviour for FiringAtKeeping {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            if npc.id == self.0 {
                npc.goal
            } else {
                FiringAt(self.0).decide(npc, around, chance)
            }
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            FiringAt(self.0).trigger(npc, around)
        }

        fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
            FiringAt(self.0).target(npc, around)
        }
    }

    #[test]
    fn on_a_jump_outs_tick_an_earlier_slot_lets_go_before_its_fire() {
        // Sub 4 frees slot 3 in the AI loop (@0x859b8), before slot 2's
        // `_HandleShip`, whose slot test (@0x33865-0x33888) clears its
        // target before its fire (@0x34ffd).
        let catalog = FakePilotCatalog {
            weapons: vec![WeaponRecord {
                guidance: 4,
                ..blaster()
            }],
            ..armed()
        };
        let fired_at = |jumps: bool| {
            let mut session = facing_an_npc(&catalog, 180);
            let shooter = &mut session.space.traffic.npcs_mut()[0];
            shooter.berth = 2;
            shooter.state.position = OUTSIDE;
            let shooter = shooter.id;
            let target = place(&mut session, 3, OUTSIDE + Vec2::new(0.0, -40.0), Vec2::ZERO);
            if jumps {
                let runner = session.space.npc_mut(target).expect("there");
                runner.goal = Goal::JumpOut;
                runner.begin_own_jump();
                runner.warp.stage = 3.0;
                runner.warp.since = 100_000;
            }
            pass(&mut session, &catalog, &FiringAtKeeping(target));
            assert_eq!(session.space.npc(target).is_none(), jumps, "{jumps}");
            let mut shots = session.shots().iter();
            shots.any(|shot| shot.firer == ShipRef::Npc(shooter))
        };
        assert!(fired_at(false), "it fires at a ship that stays");
        assert!(!fired_at(true), "it let go of the one gone");
    }

    /// Every NPC keeps its goal and target; each think records whether it
    /// reads a gone slot in its target.
    #[derive(Debug, Default)]
    struct ReadsGone(std::cell::RefCell<Vec<(NpcId, bool)>>);

    impl Behaviour for ReadsGone {
        fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            let held = around.held_gone(npc).is_some();
            self.0.borrow_mut().push((npc.id, held));
            npc.goal
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    #[test]
    fn a_follower_that_gives_up_a_jump_on_a_ship_gone_at_a_lower_slot_reads_no_gone_slot_on_the_next_tick()
     {
        // On the jump path the high level clears +0x70 before every goal
        // block (@0x8d4e4-0x8d50d), goal 0xb's give-up included
        // (@0x8deb9-0x8ded1, @0x8df31-0x8df49), and the AI wait's skip
        // leaves it to `_HandleShip`'s test that frame (@0x8d4b9-0x8d4d2):
        // no think of the follower reads the slot its target held.
        let catalog = armed();
        let mut session = empty(&catalog);
        let runner = place(&mut session, 2, Vec2::new(-2000.0, 0.0), Vec2::ZERO);
        let npc = session.space.npc_mut(runner).expect("there");
        npc.goal = Goal::JumpOut;
        npc.begin_own_jump();
        npc.warp.stage = 3.0;
        npc.warp.since = 100_000;
        let leader = place(&mut session, 3, Vec2::new(-900.0, 0.0), Vec2::ZERO);
        let follower = place(&mut session, 5, Vec2::new(-900.0, 50.0), Vec2::ZERO);
        let npc = session.space.npc_mut(follower).expect("there");
        npc.leader = Some(leader);
        npc.enter_leaders_jump(ShipRef::Npc(leader));
        npc.target = Some(ShipRef::Npc(runner));
        let reads = ReadsGone::default();
        pass(&mut session, &catalog, &reads);
        assert!(session.space.npc(runner).is_none(), "jumped out");
        let npc = session.space.npc(follower).expect("there");
        assert_eq!((npc.goal, npc.warp.goal), (Goal::Idle, None), "gave up");
        reads.0.take();
        pass(&mut session, &catalog, &reads);
        assert!(reads.0.take().contains(&(follower, false)));
    }

    /// `session`'s NPC `lander` put at rest over the system's first
    /// stellar, bound to land on it, and a wreck in slot `berth` beside it
    /// whose blast reaches it, a mission ship that lands leaving as
    /// `landed_leaves` says; after a pass of `catalog`.
    fn land_beside_a_wreck(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        lander: NpcId,
        berth: u32,
        landed_leaves: crate::rulebook::RuleSource,
    ) {
        session.setup.rules = session
            .setup
            .rules
            .with_override(crate::rulebook::RuleKey::LandedLeaves, landed_leaves);
        let site = session.chart.sites[0];
        let npc = session.space.npc_mut(lander).expect("there");
        npc.state.position = site.position;
        npc.state.velocity = Vec2::ZERO;
        npc.goal = Goal::Land(site.id);
        wreck(session, berth, site.position + Vec2::new(20.0, 0.0));
        session.take_combat_events();
        pass(session, catalog, &Keep);
    }

    #[test]
    fn an_npc_whose_flight_lands_stays_in_its_slot_and_a_later_blast_reaches_it() {
        // State 1's landing leaves its slot active (+0xb8 is not cleared,
        // @0x8dadc-0x8db5a), so a later slot's shockwave reaches it.
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let catalog = armed();
            let mut session = empty(&catalog);
            let lander = place(&mut session, 1, Vec2::ZERO, Vec2::ZERO);
            land_beside_a_wreck(&mut session, &catalog, lander, 2, reading);
            assert_eq!(session.departed(), [], "{reading:?}");
            let waiting = session.space.npc(lander).expect("it stays");
            assert!(waiting.wait_running(), "{reading:?}: waiting there");
            assert!(hit(&session.take_combat_events(), lander), "{reading:?}");
        }
    }

    #[test]
    fn a_mission_ship_that_leaves_as_it_lands_takes_no_step_and_no_later_blast() {
        // By `landed_leaves`' other reading its slot is inactive from its
        // own `_HandleShip`: the loop skips its display, and a later slot's
        // shockwave reaches it no more.
        use crate::session::mission_ships::tests::{goal_catalog, in_flight};
        // A chase-off goal's ships (`ShipGoal` 6).
        let catalog = goal_catalog(6, 0);
        let mut session = in_flight(&catalog);
        let lander = session.npcs()[0].id;
        let site = session.chart.sites[0];
        land_beside_a_wreck(&mut session, &catalog, lander, 30, RuleSource::Bible);
        assert_eq!(session.departed(), [(lander, Outcome::Landed(site.id))]);
        assert!(session.space.npc(lander).is_none(), "gone");
        assert!(!hit(&session.take_combat_events(), lander));
    }

    /// Every NPC keeps its goal and its target: no think drops either, so
    /// only the slot test can.
    #[derive(Debug)]
    struct Holds;

    impl Behaviour for Holds {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    /// An NPC at berth `berth`, well clear of [`WRECKED`], fighting `aim`.
    fn holding(session: &mut Session, berth: u32, aim: ShipRef) -> NpcId {
        let id = place(session, berth, Vec2::new(-900.0, 0.0), Vec2::ZERO);
        let npc = session.space.npc_mut(id).expect("there");
        npc.goal = Goal::Attack(aim);
        npc.target = Some(aim);
        id
    }

    fn target(session: &Session, id: NpcId) -> Option<ShipRef> {
        session.space.npc(id).expect("there").target
    }

    #[test]
    fn a_later_slot_lets_go_of_a_wreck_in_the_pass_it_explodes_and_an_earlier_on_the_next() {
        // `_HandleShip`'s slot test (@0x33865-0x33888), slot by slot: the
        // wreck's display (@0x2dee0) comes before the later slot's and
        // after the earlier slot's.
        let catalog = armed();
        let mut session = empty(&catalog);
        let gone = wreck(&mut session, 3, WRECKED);
        let aim = ShipRef::Npc(gone);
        let earlier = holding(&mut session, 2, aim);
        let later = holding(&mut session, 5, aim);
        pass(&mut session, &catalog, &Holds);
        assert!(session.space.npc(gone).is_none(), "exploded");
        assert_eq!(target(&session, earlier), Some(aim), "its step came first");
        assert_eq!(target(&session, later), None);
        let goal = session.space.npc(later).expect("there").goal;
        assert_eq!(goal, Goal::Attack(aim), "the test writes +0x70 alone");
        pass(&mut session, &catalog, &Holds);
        assert_eq!(target(&session, earlier), None, "its next step");
    }

    #[test]
    fn double_times_second_pass_lets_go_at_each_slot() {
        // The second pass runs the ship loop with no AI (@0x44350).
        let catalog = armed();
        let mut session = empty(&catalog);
        let gone = wreck(&mut session, 3, WRECKED);
        let aim = ShipRef::Npc(gone);
        let earlier = holding(&mut session, 2, aim);
        let later = holding(&mut session, 5, aim);
        again(&mut session);
        assert!(session.space.npc(gone).is_none(), "exploded");
        assert_eq!(target(&session, earlier), Some(aim));
        assert_eq!(target(&session, later), None);
        again(&mut session);
        assert_eq!(target(&session, earlier), None);
    }

    /// A carrier at berth `berth` aiming at `aim`, its bay held at its
    /// full `Reload`, which it gives.
    fn carrier(session: &mut Session, berth: u32, aim: ShipRef) -> (NpcId, f32) {
        let mut ship = carrier_npc(NpcId(0), 2);
        ship.leader = None;
        ship.state.position = Vec2::new(-900.0, 0.0);
        ship.goal = Goal::Attack(aim);
        ship.target = Some(aim);
        ship.armament.hold_idle_bays();
        let full = ship.armament.mounts()[0].reload;
        (session.space.traffic.add_npc_at(ship, berth), full)
    }

    fn reload(session: &Session, id: NpcId) -> f32 {
        session.space.npc(id).expect("there").armament.mounts()[0].reload
    }

    #[test]
    fn a_later_carrier_holds_its_bays_in_the_pass_its_target_explodes() {
        // The hold reads the -1 the test just wrote (@0x3390a-0x3394e).
        let catalog = armed();
        let mut session = empty(&catalog);
        let gone = wreck(&mut session, 3, WRECKED);
        let (later, full) = carrier(&mut session, 5, ShipRef::Npc(gone));
        pass(&mut session, &catalog, &Holds);
        assert_eq!(reload(&session, later), full);
    }

    #[test]
    fn an_earlier_carrier_counts_down_that_pass_and_holds_from_the_next() {
        let catalog = armed();
        let mut session = empty(&catalog);
        let gone = wreck(&mut session, 3, WRECKED);
        let (earlier, full) = carrier(&mut session, 2, ShipRef::Npc(gone));
        pass(&mut session, &catalog, &Holds);
        assert_eq!(reload(&session, earlier), full - 1.0, "its target stood");
        pass(&mut session, &catalog, &Holds);
        assert_eq!(reload(&session, earlier), full, "back at full");
    }

    #[test]
    fn a_ship_entering_a_gate_is_let_go_by_slot() {
        // Its display frees the slot as the fade ends (@0x2c078).
        let catalog = armed();
        let mut session = empty(&catalog);
        let leaving = place(&mut session, 3, WRECKED, Vec2::ZERO);
        let gate = session.chart.sites[0].id;
        session.space.npc_mut(leaving).expect("there").mode = Mode::EnteringGate {
            gate,
            ticks_left: 1,
        };
        let aim = ShipRef::Npc(leaving);
        let earlier = holding(&mut session, 2, aim);
        let later = holding(&mut session, 5, aim);
        pass(&mut session, &catalog, &Holds);
        assert_eq!(session.departed(), [(leaving, Outcome::EnteredGate(gate))]);
        assert_eq!(target(&session, earlier), Some(aim));
        assert_eq!(target(&session, later), None);
        pass(&mut session, &catalog, &Holds);
        assert_eq!(target(&session, earlier), None);
    }

    #[test]
    fn a_ship_jumping_out_is_let_go_by_every_slot_in_that_pass() {
        // The low level frees the slot in the AI loop (@0x859b8), before
        // every slot's `_HandleShip`.
        let catalog = armed();
        let mut session = empty(&catalog);
        let leaving = place(&mut session, 3, OUTSIDE, Vec2::ZERO);
        let runner = session.space.npc_mut(leaving).expect("there");
        runner.goal = Goal::JumpOut;
        runner.begin_own_jump();
        runner.warp.stage = 3.0;
        runner.warp.since = 1000;
        let aim = ShipRef::Npc(leaving);
        let earlier = holding(&mut session, 2, aim);
        let later = holding(&mut session, 5, aim);
        pass(&mut session, &catalog, &Holds);
        assert_eq!(session.departed(), [(leaving, Outcome::JumpedOut)]);
        assert_eq!(target(&session, earlier), None);
        assert_eq!(target(&session, later), None);
    }

    #[test]
    fn a_fighter_docking_is_let_go_by_every_slot_in_that_pass() {
        // `_AIAddFighterToParent` frees the slot in the AI loop (@0x80404).
        let catalog = armed();
        let mut session = empty(&catalog);
        let (bay, _) = carrier(&mut session, 7, ShipRef::Player);
        let at = state(&session, bay).position;
        let leaving = place(&mut session, 3, at, Vec2::ZERO);
        let fighter = session.space.npc_mut(leaving).expect("there");
        // A Viper, which the carrier's bay carries (`RuleKey::FighterNoBay`).
        fighter.ship = VIPER;
        fighter.goal = Goal::Dock(ShipRef::Npc(bay));
        fighter.carrier = Some(Carrier {
            ship: ShipRef::Npc(bay),
            window: 100.0,
            reach: 30.0,
        });
        let aim = ShipRef::Npc(leaving);
        let earlier = holding(&mut session, 2, aim);
        let later = holding(&mut session, 5, aim);
        pass(&mut session, &catalog, &Holds);
        assert!(session.space.npc(leaving).is_none(), "aboard");
        assert_eq!(target(&session, earlier), None);
        assert_eq!(target(&session, later), None);
    }

    #[test]
    fn an_arrival_in_a_wrecks_slot_takes_the_earlier_slots_aim() {
        // `_EnterMoreShips` (@0x44237) runs before the thinks and spawns
        // into the first free slot, leaving every other ship's +0x70.
        // `place` stands for that spawn, across the session's passes; the
        // traffic's `gone` tests run the arrival itself.
        let catalog = armed();
        let mut session = empty(&catalog);
        let gone = wreck(&mut session, 3, WRECKED);
        let aim = ShipRef::Npc(gone);
        let earlier = holding(&mut session, 2, aim);
        let later = holding(&mut session, 5, aim);
        pass(&mut session, &catalog, &Holds);
        let arrival = ShipRef::Npc(place(&mut session, 3, WRECKED, Vec2::ZERO));
        pass(&mut session, &catalog, &Holds);
        assert_eq!(target(&session, earlier), Some(arrival), "still fighting");
        let goal = session.space.npc(earlier).expect("there").goal;
        assert_eq!(goal, Goal::Attack(arrival));
        assert_eq!(target(&session, later), None);
    }

    #[test]
    fn the_players_explosion_lets_every_slot_go_of_it() {
        // Its final explosion reaches the same write (@0x2dee0), and the
        // loop's slots all come after it.
        let catalog = armed();
        let mut session = empty(&catalog);
        let earlier = holding(&mut session, 2, ShipRef::Player);
        let later = holding(&mut session, 5, ShipRef::Player);
        pass(&mut session, &catalog, &Holds);
        assert_eq!(target(&session, earlier), Some(ShipRef::Player));
        assert_eq!(target(&session, later), Some(ShipRef::Player));
        session.ship.condition = Condition::Destroyed;
        pass(&mut session, &catalog, &Holds);
        assert_eq!(target(&session, earlier), None);
        assert_eq!(target(&session, later), None);
    }

    #[test]
    fn nothing_flies_on_a_second_pass_before_the_system_is_set_up() {
        // Before the advance pass populates the system, its ships move
        // from the next pass.
        let catalog = armed();
        let mut session = Session::start(&catalog).expect("starts");
        let at = Vec2::new(300.0, 0.0);
        let drifting = place(&mut session, 1, at, Vec2::new(2.0, 0.0));
        again(&mut session);
        assert_eq!(state(&session, drifting).position, at);
    }
}
