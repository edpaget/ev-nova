//! The session's keeper of the fight's ships between the stretches of a
//! phase ([`Turns`], the fight's [`HitTurns`] port; see
//! [`hit_turns`](crate::combat::hit_turns)): the collision passes and
//! `_HandleBeams` stop after each direct hit, and the keeper settles and
//! answers that stretch's hits ([`Session::settle_landed`]) before it
//! lends the fighters again, built afresh from what the answers wrote
//! ([`Session::with_fighters`]). So each hit's can't-kill reads its
//! attacker's target and goal, and its leader's goal, as the reactions of
//! the hits before it left them, as `_DamageShip`'s prologue reads them
//! live (@0x3a95b-0x3a9a3), and so does each later read of a fighter
//! (see `Session::collision_passes` and `Session::beams_phase`).
//!
//! Each stretch settles against the session as it was lent (`was`, the
//! player's condition): the NPCs and the player as the last stretch's
//! settle left them. The beams' snapshot comes after the stellar hazards,
//! which run in their own lend, so no settle between stretches reads a
//! deadly stellar's kill as a hit's downing; the kills' hold
//! ([`Session::hold_stellar_kills`]) runs once, after the last stretch.
//! The last stretch's hits are the phase's to settle, with its tail.
//! Lending afresh draws no chance and raises nothing: the fighters'
//! terms ([`Session::fight_terms`]) are reads.
//!
//! *Ordering against the original.* Between the stretches the asteroid
//! field is out on loan to the phase, so an answer's `World` holds no
//! asteroid slot 0; only the dispatch autopilot reads
//! [`World::mining_rock`](crate::traffic::World::mining_rock), never an
//! answer. A hit that is never answered (a blast) settles with the
//! stretch after it, before the next answer and the phase's end, where
//! `_DamageShip` settles it as it lands: no boundary is crossed.

use super::{NpcBefore, Session};
use crate::ai::Behaviour;
use crate::chance::Chance;
use crate::combat::hit_turns::{HitTurns, Stretch, StretchFn};
use crate::combat::hull::Condition;
use crate::combat::{GamePass, Rules};

/// The session as the keeper of a fight's phase run in stretches: it
/// lends the fighters to each stretch on `pass` by `rules`, drawing on
/// `chance`, and settles and answers each stretch's hits, the NPCs
/// answering as `behaviour` says.
pub(super) struct Turns<'s, B: ?Sized> {
    session: &'s mut Session,
    behaviour: &'s B,
    pass: GamePass,
    rules: Rules<'s>,
    chance: &'s mut dyn Chance,
    /// Every NPC as the last stretch was lent.
    was: Vec<NpcBefore>,
    /// The player's condition as the last stretch was lent.
    player_was: Condition,
}

impl<'s, B: Behaviour + ?Sized> Turns<'s, B> {
    /// The keeper of `session`'s phase on `pass` by `rules`, drawing on
    /// `chance`, its NPCs answering as `behaviour` says.
    pub(super) fn new(
        session: &'s mut Session,
        behaviour: &'s B,
        (pass, rules): (GamePass, Rules<'s>),
        chance: &'s mut dyn Chance,
    ) -> Self {
        let was = session.space.npcs_before();
        let player_was = session.ship.condition;
        Self {
            session,
            behaviour,
            pass,
            rules,
            chance,
            was,
            player_was,
        }
    }

    /// What the last stretch's hits settle against: every NPC, and the
    /// player's condition, as it was lent.
    pub(super) fn last(self) -> (Vec<NpcBefore>, Condition) {
        (self.was, self.player_was)
    }
}

impl<B: Behaviour + ?Sized> HitTurns for Turns<'_, B> {
    fn lend(&mut self, stretch: &mut StretchFn<'_>) -> Stretch {
        self.was = self.session.space.npcs_before();
        self.player_was = self.session.ship.condition;
        let mut ended = Stretch::Done;
        self.session.with_fighters(
            self.pass,
            self.rules,
            &mut *self.chance,
            |combat, fighters, arena, chance| ended = stretch(combat, fighters, arena, chance),
        );
        ended
    }

    fn answer(&mut self) {
        let lent = (self.player_was, None);
        self.session.settle_landed(
            self.behaviour,
            &self.was,
            lent,
            self.rules,
            &mut *self.chance,
        );
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::NovaAi;
    use crate::chance::NeverFires;
    use crate::combat::hit_turns;

    #[test]
    fn the_sessions_keeper_keeps_the_contract() {
        let catalog = crate::session::tests::armed();
        let mut session = crate::session::tests::facing_an_npc(&catalog, 180);
        let shield = session.pilot.reserves.shield.now;
        let pass = GamePass::Advance { double_time: false };
        let ai = NovaAi::default();
        let mut chance = NeverFires;
        let mut turns = Turns::new(&mut session, &ai, (pass, Rules::default()), &mut chance);
        assert_eq!(hit_turns::check_contract(&mut turns), Ok(()));
        drop(turns);
        assert_eq!(session.pilot.reserves.shield.now, shield, "given back");
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod wired {
    use super::*;
    use crate::ai::{Goal, NovaAi};
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::reserves::Reserves;
    use crate::testkit::FakePilotCatalog;
    use crate::testkit::work;
    use crate::traffic::npc::NpcId;

    /// `catalog`'s session in flight with no traffic of its own, the player
    /// having fired two shots, still flying; then P plundering V, both
    /// shown, P in `p_berth` and V in `v_berth`, each of 10 shield and 30
    /// armour: the session, P and V.
    fn plunder_scene(
        catalog: &FakePilotCatalog,
        p_berth: u32,
        v_berth: u32,
    ) -> (Session, NpcId, NpcId) {
        let mut session = Session::start(catalog).expect("starts");
        session.space.traffic_due = false;
        session.hold_fire(true, false);
        for _ in 0..10 {
            if session.shots().len() >= 2 {
                break;
            }
            work(&mut session, catalog, Controls::default());
        }
        assert_eq!(session.shots().len(), 2, "fired");
        session.hold_fire(false, false);
        session.take_combat_events();
        let ship = |x: f32, y: f32| {
            let mut npc = crate::testkit::npc(0, crate::stats::ShipStats::default());
            npc.reserves = Reserves::full(10.0, 30.0, 100.0);
            npc.state.position = Vec2::new(x, y);
            npc
        };
        let quarry = session.space.traffic.add_npc_at(ship(300.0, 0.0), v_berth);
        let mut pirate = ship(0.0, 300.0);
        pirate.goal = Goal::Plunder {
            ship: ShipRef::Npc(quarry),
            hopeless: false,
        };
        pirate.target = Some(ShipRef::Npc(quarry));
        let pirate = session.space.traffic.add_npc_at(pirate, p_berth);
        session.prepare_all_sprites();
        (session, pirate, quarry)
    }

    /// One pass's collision passes over [`plunder_scene`], P in berth 5 so
    /// walked before V in berth 2 (the higher berth lower in the layer),
    /// with the two shots set on their last moves: P's shell of 50 mass
    /// and 50 energy, fired while it attacked so not disable-only, ending
    /// on V; and, when `players_hit`, the player's blaster aimed at P
    /// ending on it, or else far off. The session after, P and V.
    fn one_collision_pass(players_hit: bool) -> (Session, NpcId, NpcId) {
        let catalog = crate::session::tests::armed();
        let (mut session, pirate, quarry) = plunder_scene(&catalog, 5, 2);
        let at = |id| session.space.npc(id).expect("there").state.position;
        let (pirate_at, quarry_at) = (at(pirate), at(quarry));
        let step = Vec2::new(10.0, 0.0);
        let players_end = if players_hit {
            pirate_at
        } else {
            Vec2::new(-3000.0, -3000.0)
        };
        let shots = session.space.combat.shots_mut();
        shots[0].moved_from = Some(players_end - step);
        shots[0].position = players_end;
        // Aimed at P, so no stray hit, which P would forgive.
        shots[0].aimed_at = Some(ShipRef::Npc(pirate));
        let shell = &mut shots[1];
        shell.firer = ShipRef::Npc(pirate);
        shell.fleet = ShipRef::Npc(pirate);
        shell.weapon.mass_damage = 50.0;
        shell.weapon.energy_damage = 50.0;
        shell.disable_only = false;
        shell.moved_from = Some(quarry_at - step);
        shell.position = quarry_at;
        let pass = GamePass::Advance { double_time: false };
        session.collision_passes(&NovaAi::default(), pass, Rules::default(), &mut NeverFires);
        (session, pirate, quarry)
    }

    #[test]
    fn a_plunderer_the_players_hit_turned_kills_its_quarry_later_in_the_same_pass() {
        // The player's hit on P is answered as it lands
        // (`_AIMakeShipAttackPlayer` @0x89d4d, goal 0xd to 4 @0x89d3e), so
        // P's shell, met later in the walk, reads P no longer plundering V
        // (`_DamageShip` @0x3a95b-0x3a964).
        let (session, pirate, quarry) = one_collision_pass(true);
        let pirate = session.space.npc(pirate).expect("there");
        assert_eq!(pirate.target, Some(ShipRef::Player));
        assert_eq!(pirate.goal.plundering(), None);
        let quarry = session.space.npc(quarry).expect("there");
        assert!(
            matches!(
                quarry.condition,
                Condition::Dying { .. } | Condition::Destroyed
            ),
            "{:?}",
            quarry.condition
        );
        let (session, _, quarry) = one_collision_pass(false);
        let quarry = session.space.npc(quarry).expect("there");
        assert_eq!(quarry.condition, Condition::Disabled, "spared");
        assert_eq!(
            quarry.reserves.armor.now,
            crate::combat::damage::SPARED_ARMOR
        );
    }

    /// A beam of 200 pixels and three ticks, firing every 10, of 5 mass
    /// and 10 energy a tick.
    fn ray() -> crate::catalog::WeaponRecord {
        crate::catalog::WeaponRecord {
            guidance: 0,
            count: 3,
            reload: 10,
            speed: 0,
            beam_length: 200,
            mass_dmg: 5,
            energy_dmg: 10,
            ..crate::session::tests::blaster()
        }
    }

    #[test]
    fn a_beam_hit_answered_after_a_deadly_stellars_kill_reads_no_downing_in_it() {
        // `_HandleDeadlyStellars` (@0x4450e) runs before `_HandleBeams`
        // (@0x4453d): K, killed, is out of the beams, and the settle at the
        // beam's stop reads it as the kill left it, so it raises no
        // disabled fate.
        let catalog = FakePilotCatalog {
            weapons: vec![ray()],
            ..crate::session::tests::armed()
        };
        let mut session = crate::session::tests::facing_an_npc(&catalog, 180);
        let killed = session.npcs()[0].id;
        session.space.npc_mut(killed).expect("there").state.position = Vec2::new(0.0, -80.0);
        let deadly = crate::catalog::LandingSite {
            flags2: crate::deadly_stellar::DEADLY_STELLAR,
            frame_size: Some((40, 40)),
            ..crate::testkit::planet(140, 0.0, -80.0)
        };
        session.chart.listed = vec![deadly];
        session.chart.sites = vec![deadly];
        let mut behind = crate::testkit::npc(0, crate::stats::ShipStats::default());
        behind.reserves = Reserves::full(10.0, 30.0, 100.0);
        behind.state.position = Vec2::new(0.0, -180.0);
        let behind = session.space.traffic.add_npc_at(behind, 5);
        session.hold_fire(true, false);
        let pass = GamePass::Advance { double_time: false };
        session.player_fires(pass, Rules::default(), &mut NeverFires);
        assert_eq!(session.space.combat.beams().len(), 1, "fired");
        session.take_combat_events();
        session.beams_phase(&NovaAi::default(), pass, Rules::default(), &mut NeverFires);
        let hit: Vec<ShipRef> = session
            .take_combat_events()
            .iter()
            .filter_map(|event| match *event {
                crate::combat::CombatEvent::Hit { ship, .. } => Some(ship),
                _ => None,
            })
            .collect();
        assert_eq!(hit, [ShipRef::Npc(behind)], "past the killed ship");
        let fated: Vec<_> = session
            .story_raised()
            .into_iter()
            .filter(|event| matches!(event, crate::story::StoryEvent::ShipFate { .. }))
            .collect();
        assert_eq!(fated, [], "no disabled fate for K");
        let k = session.space.npc(killed).expect("there until the take-out");
        assert_eq!(k.condition, Condition::Intact, "as the hits left it");
    }

    /// [`plunder_scene`]'s two ships, plundering nothing, E in berth 5,
    /// walked first, and X in berth 2, with the two shots set on their
    /// last moves, both ending on E: the first `first`'s, of 25 mass, which
    /// disables it (5 armour of 30 left), the second the player's blaster,
    /// aimed at it. One pass's collision passes over them, each direct hit
    /// answered by Nova's AI before the next. The session after, E and X.
    fn twice_on_one_ship(
        first: impl Fn(NpcId, NpcId) -> ShipRef,
        escort: bool,
    ) -> (Session, NpcId, NpcId) {
        let catalog = crate::session::tests::armed();
        let (mut session, e, x) = plunder_scene(&catalog, 5, 2);
        let npc = session.space.npc_mut(e).expect("there");
        npc.goal = Goal::Idle;
        npc.target = None;
        if escort {
            let record = crate::pilot::Escort {
                ship: npc.ship,
                reserves: npc.reserves,
                order: None,
                carried: false,
                wage: None,
                person: None,
                mark: None,
                voice: None,
                berth: None,
            };
            session.enlist(e, record);
            assert_eq!(
                session.space.npc(e).expect("there").leader_ref(),
                Some(ShipRef::Player)
            );
        }
        let e_at = session.space.npc(e).expect("there").state.position;
        let step = Vec2::new(10.0, 0.0);
        let by = first(e, x);
        let shots = session.space.combat.shots_mut();
        for shot in shots.iter_mut() {
            shot.moved_from = Some(e_at - step);
            shot.position = e_at;
            shot.aimed_at = Some(ShipRef::Npc(e));
        }
        shots[0].firer = by;
        shots[0].fleet = by;
        shots[0].weapon.mass_damage = 25.0;
        shots[0].weapon.energy_damage = 10.0;
        let pass = GamePass::Advance { double_time: false };
        session.collision_passes(&NovaAi::default(), pass, Rules::default(), &mut NeverFires);
        (session, e, x)
    }

    /// The fates `session` raised for `ship`.
    fn fates_of(session: &Session, ship: NpcId) -> Vec<crate::story::Fate> {
        session
            .story_raised()
            .into_iter()
            .filter_map(|event| match event {
                crate::story::StoryEvent::ShipFate { ship: of, fate } if of == ship => Some(fate),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn each_stretch_settles_against_the_ships_as_the_last_left_them() {
        // The first hit disables E and raises its fate at its stop; the
        // second's settle reads E disabled already, so raises none again.
        let (session, e, _) = twice_on_one_ship(|_, _| ShipRef::Player, false);
        assert_eq!(fates_of(&session, e), [crate::story::Fate::Disabled]);
    }

    #[test]
    fn an_escort_a_hit_disables_is_lost_at_its_stop_so_the_players_next_shot_strikes_it() {
        // `_DamageShip`'s escort block (@0x3af8f-0x3b0d7) runs in the hit
        // that disables it: lent afresh after that hit's settle, E is no
        // ship of the player's fleet, so the player's shot no longer passes
        // through it (`_ShotCanHitShip` @0x447b3).
        let (mut session, e, _) = twice_on_one_ship(|_, x| ShipRef::Npc(x), true);
        let hits = session
            .take_combat_events()
            .iter()
            .filter(|event| {
                matches!(event, crate::combat::CombatEvent::Hit { ship, .. } if *ship == ShipRef::Npc(e))
            })
            .count();
        assert_eq!(hits, 2, "X's hit, then the player's");
        assert_ne!(
            session.space.npc(e).expect("there").leader_ref(),
            Some(ShipRef::Player)
        );
    }

    /// One `_HandleShot` over [`plunder_scene`], P in berth 5 and V in
    /// berth 2, with the two shots set: P's spent shell in slot 1 at V,
    /// which releases two blasters on its expiry (@0x35f49) and then
    /// detonates, its blast of 25 mass and 10 energy disabling V (5 armour
    /// of 30 left); and, when `locks`, the player's homing missile in slot
    /// 0 making its lock on P this visit (@0x35855), or else a blaster far
    /// off. The session after, P, V, and the disable-only flags of P's
    /// sub-munitions.
    fn one_shot_loop(locks: bool) -> (Session, NpcId, NpcId, Vec<bool>) {
        use crate::combat::projectile::LOCK_AGE;
        use crate::combat::weapon::{Guidance, Submunitions};
        let catalog = crate::session::tests::armed();
        let (mut session, pirate, quarry) = plunder_scene(&catalog, 5, 2);
        let p = ShipRef::Npc(pirate);
        let quarry_at = session.space.npc(quarry).expect("there").state.position;
        let far = Vec2::new(-3000.0, -3000.0);
        let step = Vec2::new(10.0, 0.0);
        let shots = session.space.combat.shots_mut();
        assert_eq!(
            shots.iter().map(|shot| shot.slot).collect::<Vec<_>>(),
            [0, 1]
        );
        let missile = &mut shots[0];
        missile.moved_from = Some(far - step);
        missile.position = far;
        if locks {
            missile.weapon.guidance = Guidance::Homing;
            missile.weapon.lifetime = LOCK_AGE * 2;
            missile.age = LOCK_AGE;
            missile.aimed_at = Some(p);
            missile.target = Some(p);
        }
        let shell = &mut shots[1];
        let sub = shell.weapon.id;
        shell.firer = p;
        shell.fleet = p;
        shell.firer_slot = Some(crate::ship_layer::Slot::Berth(5));
        shell.weapon.submunitions = Some(Submunitions {
            weapon: sub,
            count: 2,
            theta: 0,
            limit: None,
        });
        shell.weapon.flags |= crate::combat::weapon::DETONATES;
        shell.weapon.blast_radius = 40.0;
        shell.weapon.mass_damage = 25.0;
        shell.weapon.energy_damage = 10.0;
        shell.moved_from = Some(quarry_at - step);
        shell.position = quarry_at;
        shell.age = shell.weapon.lifetime;
        let pass = GamePass::Advance { double_time: false };
        session.shots_phase(&NovaAi::default(), pass, Rules::default(), &mut NeverFires);
        let released = session
            .shots()
            .iter()
            .filter(|shot| shot.firer == p && shot.weapon.id == sub)
            .map(|shot| shot.disable_only)
            .collect();
        (session, pirate, quarry, released)
    }

    #[test]
    fn a_later_slots_sub_munitions_read_their_owner_as_the_locks_answer_left_it() {
        // The lock's `_DamageShip` (@0x35855) answers before slot 1's end
        // of life (@0x35f49): `_AIMakeShipAttackPlayer` (@0x89d4d, goal 0xd
        // to 4 @0x89d3e) has turned P, so `_SpawnShot` (@0x3ea1b) reads it
        // no longer plundering.
        let (session, pirate, quarry, released) = one_shot_loop(true);
        let npc = session.space.npc(pirate).expect("there");
        assert_eq!(npc.target, Some(ShipRef::Player));
        assert_eq!(npc.goal.plundering(), None);
        assert_eq!(released, [false, false]);
        // The blast after the lock lands in the last stretch, which the
        // phase's own settle raises V's fate for.
        assert_eq!(fates_of(&session, quarry), [crate::story::Fate::Disabled]);
        let (session, _, quarry, released) = one_shot_loop(false);
        assert_eq!(released, [true, true], "P still plunders");
        assert_eq!(fates_of(&session, quarry), [crate::story::Fate::Disabled]);
    }

    #[test]
    fn the_last_stretch_settles_a_blast_its_stop_left_to_come() {
        // The player's bomb hits E, and the walk stops there; its blast
        // lands in the last stretch, disabling Y, which the phase's own
        // settle raises the fate of, against Y as the last stretch was lent.
        let catalog = crate::session::tests::armed();
        let (mut session, e, y) = plunder_scene(&catalog, 5, 2);
        let npc = session.space.npc_mut(e).expect("there");
        npc.goal = Goal::Idle;
        npc.target = None;
        let e_at = npc.state.position;
        let bystander = session.space.npc_mut(y).expect("there");
        bystander.state.position = e_at + Vec2::new(0.0, 20.0);
        bystander.reserves.shield.now = 0.0;
        bystander.reserves.armor.now = 12.0;
        let step = Vec2::new(10.0, 0.0);
        let shots = session.space.combat.shots_mut();
        shots[0].moved_from = Some(e_at - step);
        shots[0].position = e_at;
        shots[0].aimed_at = Some(ShipRef::Npc(e));
        shots[0].weapon.blast_radius = 40.0;
        shots[0].weapon.mass_damage = 5.0;
        shots[0].weapon.energy_damage = 0.0;
        let far = Vec2::new(-3000.0, -3000.0);
        shots[1].moved_from = Some(far - step);
        shots[1].position = far;
        let pass = GamePass::Advance { double_time: false };
        session.collision_passes(&NovaAi::default(), pass, Rules::default(), &mut NeverFires);
        assert_eq!(
            session.space.npc(y).expect("there").condition,
            Condition::Disabled
        );
        assert_eq!(fates_of(&session, y), [crate::story::Fate::Disabled]);
    }
}
