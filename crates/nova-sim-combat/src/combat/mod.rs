//! Combat: ships firing weapons, shots and beams flying and hitting, and
//! the damage that disables and destroys ships.
//!
//! - [`weapon`]: a weapon's [`WeaponSpec`](weapon::WeaponSpec), its `wëap`
//!   in the simulation's units.
//! - [`ai_fire`]: how an NPC fires its pick, `_FireAIShipWeapon`'s
//!   decisions: its copies, which spawn, its reload and its recoil.
//! - [`aim`]: which way each guidance fires, and at what: the bearing, the
//!   lead angle, the turrets' arcs and blind spots.
//! - [`armament`]: a ship's [`Armament`](armament::Armament), firing its
//!   weapons on a [`Trigger`](armament::Trigger) and its point defence,
//!   and the [`Arsenal`](armament::Arsenal) armaments are built from.
//! - [`beam`]: a [`Beam`](beam::Beam), held on its firer and hitting the
//!   nearest ship along it.
//! - [`damage`]: what a [`Hit`](damage::Hit) does to a ship's reserves,
//!   and which ships a [`Blast`](damage::Blast) reaches.
//! - [`defence`]: point defence, which missiles it engages, as the
//!   [`PointDefenceRule`](defence::PointDefenceRule) port says, and what
//!   its hits do to them.
//! - [`exits`]: the weapon exit points a ship's shots and beams leave
//!   from, turning with the frame it is drawn on.
//! - [`flags`]: what the simulation does with each documented weapon flag,
//!   and which of a weapon's are not done yet.
//! - [`hull`]: a ship type's [`HullSpec`](hull::HullSpec), its
//!   [`Condition`](hull::Condition), and the [`DisableRule`](hull::DisableRule)
//!   port with Nova's [`NovaDisable`](hull::NovaDisable).
//! - [`impact`]: how a hit, blast, beam tick or shockwave pushes the ship
//!   it lands on, and how a tractor beam holds its target or pulls its
//!   firer ([`TractorLock`](impact::TractorLock)).
//! - [`projectile`]: a [`Shot`](projectile::Shot) in flight, homing ones
//!   steering at their target, and the [`Target`](projectile::Target)s it
//!   may hit.
//! - [`report`]: the [`SimDiagnostic`](report::SimDiagnostic)s a session
//!   reports once each about game data it does not handle yet.
//! - [`shockwave`]: what the shockwave of a heavy ship's final explosion
//!   does to each ship it reaches, and how it pushes it.
//! - [`shot_layer`]: the sprite layer `_SpawnShot` puts a shot in, which
//!   decides its collision passes and where it is drawn.
//! - [`sides`]: who a shot or a beam may hit, by the ships' leaders,
//!   governments and `düde`s.
//! - [`stellar`]: which shots hit a destroyable stellar, and what they
//!   take off it.
//! - [`stellar_hazards`]: gravity's pull and the deadly stellars' kills
//!   inside the tick, before the beams.
//! - [`submunition`]: the shots a shot releases on a hit and at the end
//!   of its life.
//!
//! [`Combat`] holds the shots and beams in flight. Each tick
//! ([`Combat::tick`]) runs over every ship in the fight, each a
//! [`Fighter`] with the ship it targets, by the fight's [`Rules`], in this
//! order, each step one of the tick's public phases (see [`phases`]),
//! which a session may run with its own work between them:
//!
//! 1. The player's point defence fires ([`Combat::defend_player`]), then
//!    the player fires the weapons its trigger holds that are ready and
//!    its reloads count down a tick ([`Combat::fire_player`]), as
//!    `_HandlePlayer` runs them (`_HandleShipPointDefense` @0x68c74, then
//!    `_FirePlayerWeapon` @0x6ab55 and @0x6abc7, then the countdown
//!    @0x6ae25-0x6ae61) before the frame's collision passes
//!    (`_DoPlayGameWork` @0x43e19). So point defence takes a free shot or
//!    beam slot, and steps an exit cursor it shares with a gun, before the
//!    trigger, and reads the ship before the trigger's recoil. Each weapon
//!    is aimed as its guidance aims it at the ship's target ([`aim`]); a
//!    turret that cannot fire spends nothing. Each shot or beam leaves
//!    from the ship's exit point for its weapon's `ExitType`, on the frame
//!    the ship was last drawn on ([`exits`], [`Fighter::drawn_heading`]),
//!    a turret leading its target from there (see `Combat::launch`). A
//!    turret or point-defence shot rolls its miss in its ship's class
//!    ([`sides`]), and an NPC's bomb or rocket leaves at its ship's
//!    velocity and its `Speed`, the player's at four fifths or all of its
//!    ship's velocity ([`BombLaunch`](projectile::BombLaunch)). Every shot
//!    rolls its weapon's `JamVuln` ([`jamming`]), and a `Seeker` 0x0008
//!    homing one the [`Field`]'s `Interference` ([`Shot::roll_at_launch`]).
//!    A fighter bay's launch is no shot but a [`Sortie`], which the session
//!    takes ([`Combat::take_sorties`]) and puts in the system as a ship:
//!    the player's after its trigger, an NPC's after step 4.
//!    The player fires each weapon its trigger picks as `_FirePlayerWeapon`
//!    does ([`Armament::fire_player`]): its copies together, sounding once
//!    a call at [`PLAYER_FIRE_PRIORITY`] or
//!    [`PLAYER_SECONDARY_FIRE_PRIORITY`], and a `Recoil` above none pushing
//!    it back. A shot of `AmmoType` [`SELF_DESTRUCT`] that spawns destroys
//!    its firer, which breaks up at its display step (step 9). Under the original's caps
//!    ([`RuleKey::CombatCaps`](crate::RuleKey::CombatCaps)'s engine
//!    reading, not the default) a shot or beam finding its table full
//!    ([`MAX_SHOTS`], [`MAX_BEAMS`]) fails quietly, still spent and heard.
//!
//!    A ship's point defence fires at the missile it picks, with no target
//!    needed ([`defence`]), or, with none, at a ship of `Flags2` 0x0008
//!    fighting it or its leader, leading it, from its exit point, aimed
//!    from there, sounding at [`PD_FIRE_PRIORITY`]; a ship whose jump is
//!    under way ([`Fighter::jumping`]) runs none. It asks the fight's
//!    [`PointDefenceRule`] which missiles are hostile only by the other
//!    reading of [`RuleKey::PdHostility`](crate::RuleKey::PdHostility), and
//!    never of a ship.
//! 2. The hit pass (`Combat::hit_pass`, [`Combat::test_hits`]): each shot
//!    that moved in the last tick is tested on that move, as the original's next frame tests
//!    it before `_HandleShot` moves it again; a shot fired since has not
//!    moved, and is not tested. The pass meets every ship where it is now,
//!    but for contact and point defence's sprites, which meet the player
//!    where it stood as the last tick closed. In the original's order (see
//!    the `collision` module):
//!    - The contact passes: for each shot layer in turn (the shots, the
//!      homing missiles, then the turret shots drawn over the ships), the
//!      ships are walked by layer, top first, and within a layer in the
//!      order they are drawn, bottom first ([`Fighter::sprite`]; a hidden
//!      ship is not walked); each ship meets the layer's shots in firing
//!      order, and a shot that touches a ship it may hit ([`sides`]) hits
//!      it. So of two ships a move touches, the one walked first takes
//!      the shot. Then the asteroids, in slot order, meet the layer's
//!      shots that touched them (see 2a). Then the asteroids meet point
//!      defence's layer.
//!    - A point-defence shot still in point defence's layer (fired at a
//!      missile; one fired at a ship is in the shot layer, contact-tested
//!      like any shot and meeting no missile,
//!      [`ShotLayer`](projectile::ShotLayer)) and not spent on an
//!      asteroid, whose sprite overlaps a homing missile's where both
//!      ended their moves, seen from where the player stood ([`defence`]),
//!      is spent on every such missile, whoever fired it and whatever it
//!      is aimed at, but one the contact passes spent on a ship or an
//!      asteroid; a missile hit after its durability is used up is shot
//!      down where the point-defence shot is, and vanishes from the shots
//!      in flight.
//!    - The proximity pass goes by slot. Each shot point defence ended,
//!      this pass's and the missiles the last tick's point-defence beams
//!      downed, is kept hidden at life 0, neither drawn nor counted, and
//!      takes its turn in its slot ([`hidden`]). Each planet-type shot
//!      meets the stellars first (see 2b), and one that strikes goes no
//!      further. Then each live shot whose proximity fuse a ship sets off
//!      at the end of its move hits it, as
//!      [`RuleKey::ProximityFuse`](crate::RuleKey::ProximityFuse) reads it
//!      ([`Shot::fused`]: by the engine, the first ship in the fight's
//!      order within its reach, the hit at the shot); a hidden one meets
//!      no ship. Then an asteroid in its armed fuse's reach sets it off.
//!    - Each shot whose move spent its life is tested against no ship,
//!      fuse or stellar, but meets an asteroid it touched, and point
//!      defence; met by none, it waits in the table for its end of life
//!      in step 5.
//!
//!    A shot that hits damages the ship, less a point of mass and of energy
//!    for each decay step it has taken ([`DecayStep`](projectile::DecayStep)),
//!    explodes and blasts the ships around it with its weapon's whole
//!    damage, and is gone; set off by its proximity fuse, it releases its
//!    sub-munitions at that ship, and touching it, none. `ProxSafety` holds
//!    a touch until the shot's age reaches it and the fuse until its age is
//!    past it. Each age read is the one its move left, as the pass comes
//!    before the shots move again. Sub-munitions released here take their
//!    first move in step 5.
//!
//!    2a. Each shot meets the asteroids of the field the session lends the
//!    tick ([`AsteroidField`]), slot by slot,
//!    where the last pass's step left them, as the original collides them
//!    (`_DoPlayGameWork` @0x43e70-0x43ffa, then `_CheckShotProximities`
//!    @0x44015): a ship it touched first, then an asteroid it touched,
//!    the one in the lower slot of two, then a ship that set its fuse off,
//!    then an asteroid that set it off ([`asteroid_hit`]). Point-defence
//!    shots meet them too, before any missile. A hit on an asteroid
//!    explodes, throws off the weapon's hit particles
//!    ([`CombatEvent::Particles`]), blasts the ships about it if it is the
//!    player's, takes the weapon's mass damage off the asteroid's strength
//!    and pushes it, and the shot is used up with no sub-munitions. An
//!    asteroid below 0 is destroyed at once in the field: its fragments
//!    are seated, its particles and explosion raised, and its remains kept
//!    for the session's boxes ([`Combat::take_asteroid_remains`]). The
//!    rest of the walk that destroyed it still meets it, a later walk
//!    neither it nor its fragments, and the proximity pass its fragments
//!    ([`asteroid_walk`]).
//!
//!    2b. Each planet-type shot, past its `ProxSafety`, whose move met a
//!    stellar still standing among those the session hands the tick
//!    ([`StellarTarget`]), the first in the system's list, is spent on
//!    it: it explodes where it hit, with no blast and no sub-munitions,
//!    and takes its mass and energy damage off the stellar's strength. A
//!    stellar below 0 is skipped by the later shots. This comes in the
//!    proximity pass, as the original's stellar check
//!    (`_CheckShotProximities` @0x37224-0x37527) does, after the contact
//!    passes and before the shot's fuse; beams never hit a stellar by the
//!    engine's reading of
//!    [`RuleKey::BeamStellars`](crate::RuleKey::BeamStellars) (step 8).
//!    The session takes each hit as a [`StellarStrike`]
//!    ([`Combat::take_stellar_strikes`]) and shoots down those it
//!    destroyed.
//! 3. Each of the pass's hits lands as the walk finds it (`Combat::land`),
//!    with its blast, as `_HandleShipHit` lands it before the walk's next
//!    pair: so a later test in the pass reads the armour the hits before
//!    it left, and a turret shot rolling over a fighter that an earlier
//!    hit disabled hits it ([`Target::disabled`]). After the pass, each
//!    ship struck settles its condition as step 9 does, but for a
//!    breakup's countdown: it is disabled, or starts breaking up, and the
//!    pass's last strike on it says so. So an NPC downed by a shot runs no
//!    point defence and fires nothing more that tick, as `_AIDispatch` and
//!    `_HandleShip` read `_IsDisabled` after the passes.
//! 4. Every NPC's point defence fires and every NPC launches its fighters
//!    ([`Combat::defend_npcs`]), as `_AIDispatch` runs them in its slot
//!    loop (@0x44270-0x442ae), on the advance pass alone (@0x4402a): by
//!    slot, each NPC's goal-4 launch (`_AILaunchFighter` @0x8e6fa), its
//!    point defence (`_AIFireTurret` @0x80b0d), then the snipe's launch
//!    (@0x8732f), each launch sounding at [`BAY_LAUNCH_PRIORITY`] (see
//!    `ai_dispatch`). So a missile an NPC fires is first met by an NPC's
//!    point defence on the next tick, and under the caps it finds the
//!    slots of the shots the collision passes ended, and of the spent
//!    ones, still held ([`Combat::shot_room`]).
//! 5. `_HandleShot` (@0x44385, [`Combat::fly_shots`]) visits the shot
//!    slots in order ([`Combat::handle_shots`]), freeing each slot held
//!    since its last pass at its visit, and doing all of each slot's work
//!    there, so every later visit reads what an earlier one landed. Each
//!    shot whose last move spent its life comes to its end of life where
//!    it ended (@0x36195): it releases its sub-munitions at the ship its
//!    damage is aimed at, then detonates, blasting the ships about it
//!    with its weapon's whole damage, or fizzles; its blast meets the
//!    ships as the hit pass does, lands at once, and settles as in step 3.
//!    Every other shot flies a tick at its visit (a sub-munition released
//!    into a slot the visits had passed sits it out), homing ones
//!    steering at their target, breaking up or not, unless it jams them,
//!    their firer cannot see it, or a `Seeker` 0x4000 lock fails, and
//!    confused ones circling by the [`Field`]'s frame ([`projectile`]);
//!    each ship's jamming and sight are read as the hits so far left them
//!    ([`Sight`]). Each keeps where its move began, for the
//!    next tick's hit pass ([`Shot::moved_from`]). A shot of no life
//!    (`Count` 0 or below) is dropped unmoved and unseen. A hit by a shot
//!    turned on its firer is by no ship ([`Strike::by`]). Before it moves,
//!    any shot but point defence whose ship has gone (destroyed, or out of
//!    the fight) is lost ([`Jam::Lost`](projectile::Jam::Lost)), and hits
//!    no ship from then on. A player's homing missile not lost, aimed at
//!    an NPC outside the player's fleet, locks on it as its age passes
//!    [`LOCK_AGE`](projectile::LOCK_AGE): a strike of no damage the NPC
//!    answers ([`Strike::lock`]), landing at once (@0x35855). Each shot of
//!    a weapon that fires one shot at a time (`Flags3` 0x0004) holds its
//!    firer's reload for it at its `Reload` ([`Armament::hold`]).
//! 6. `_HandleShip` (@0x44483, [`Combat::handle_npcs`]), each NPC in
//!    turn by slot, as `_DoPlayGameWork`'s ship loop runs it
//!    (@0x4445c-0x444c4; one NPC's, [`Combat::handle_npc`], which the
//!    session runs right after that NPC's flight): its reloads count down a tick (@0x33896), its
//!    shields and armour regenerate (step 10), then it fires the weapons
//!    its trigger holds that are ready, as in step 1 (but no fighter
//!    bay's launch, which step 4 made), reading the conditions the passes
//!    and the ends of life left, and the ships as the slots before it
//!    left them; then its death timer ticks and its display step runs
//!    (step 9). Its shots first move in the next tick's
//!    step 5. An NPC fires its pick as `_FireAIShipWeapon` does
//!    ([`ai_fire`]): a copy that spawns nothing still spends, by the
//!    engine, and a recoil sets the firer's velocity, reported as a
//!    [`Shove`]. Then the [`Scenery`]'s stellar hazards: gravity pulls,
//!    then the deadly stellars kill ([`stellar_hazards`], `_HandleGravity`
//!    @0x44509 and `_HandleDeadlyStellars` @0x4450e, [`Combat::hazards`]),
//!    before every beam step, so a ship a deadly stellar kills is out of
//!    the beams. Then, in `_HandleBeams` (@0x4453d,
//!    [`Combat::close_tick`]), each beam spends a tick of its life, or
//!    fades a tick at its end ([`Beam::live`]), one that has ended going
//!    before it hits or is drawn again. The rest follow their firers
//!    ([`Beam::follow`]), each from its exit on the frame its firer is
//!    drawn on this tick, pointing along that frame, at its target or at
//!    its missile, turned by its jitter drawn on the tick's chance. A beam
//!    outlives its firer disabled or breaking up, and stays where its
//!    firer last was once the firer is destroyed or out of the fight; a
//!    beam whose target is destroyed or gone, or whose missile is gone,
//!    goes with it.
//! 7. A point-defence beam hits the missile it is held on the same way as
//!    a point-defence shot; a missile downed leaves the shots in flight
//!    before the next hit pass tests its move, kept hidden for that pass's
//!    stellars and asteroids ([`hidden`]), as `_HandleBeams` runs last in the
//!    frame (@0x4453d).
//! 8. The beams' hits land. A beam damages
//!    the nearest ship it may hit in its cone, or the first its segment
//!    meets, as the fight's rules say ([`beam`]), every tick until its
//!    life is over, and ends where it hit. It never hits a stellar by the
//!    engine's reading of
//!    [`RuleKey::BeamStellars`](crate::RuleKey::BeamStellars); by the
//!    Bible's, a planet-type beam meeting a stellar nearer than its ship
//!    strikes the stellar instead ([`beam_stellar`]). A beam that hits no
//!    ship, or a mining NPC's, which passes every ship, goes on to the
//!    asteroids the session lends ([`Combat::close_tick_in`]): it cuts,
//!    wears down, breaks, pushes or tractors the one it reaches before the
//!    next beam is tested ([`asteroid_beam`]).
//!
//!    Shots, beams and blasts reach a ship breaking up as they do any other
//!    not destroyed; a hit on it is a [`Strike`] that downs nothing, and
//!    its countdown goes on. The hits land one at a time, each taken as
//!    [`damage::take`] says: a disable-only weapon's shot, blast or beam
//!    never destroys ([`Shot::disable_only`], [`Beam::disable_only`]), and
//!    a ship counts as disabled before a hit when it is held disabled, not
//!    intact, or disabled by its armour as the hits before it this tick
//!    left it (`_IsDisabled` @0x3a9ad).
//!
//!    Each hit first pushes its ship by its weapon's `Impact` over the
//!    ship's `Mass` ([`impact`]): a shot's from where it met the ship, a
//!    blast's from its centre (never the ship its shot hit), a beam's on
//!    every tick it hits from its start; a breaking-up ship too, and none
//!    of a massless, planet-type or jumping one, or the escape pod. A
//!    beam of negative `Impact` is a tractor beam: its firer holds its
//!    target, or, lighter, holds itself and pulls itself toward it. The
//!    session takes each push as a [`Shove`] and sets the ship's velocity.
//!
//!    Each hit but a missile lock also ionizes its ship by its weapon's
//!    `Ionization` in its `IonizeColor` ([`ion`]): a shot's direct hit and
//!    a beam's tick wholly, however decayed, a blast by its share over its
//!    circle; the escape pod too, which nothing else here touches.
//! 9. Conditions follow the damage, settled after each step that lands
//!    hits, for each ship struck: a ship with no armour left (of the
//!    armour it holds) starts breaking up, its death timer unset, and any
//!    other that holds armour is disabled, or not, as the
//!    [`DisableRule`](hull::DisableRule) says; a ship that holds none is
//!    never disabled. A ship held disabled ([`Fighter::held_disabled`], a
//!    rescue mission's ship not yet boarded, as `_IsDisabled` reads it
//!    @0x2d24-0x2d69) is kept disabled at any armour, and is never newly
//!    disabled, so it reports no [`CombatEvent::Disabled`]; once it is no
//!    longer held, its armour decides again, with no event as it comes
//!    back intact.
//!
//!    Each ship's display step (`_HandleShipDisplay`, the `display`
//!    module) runs the countdown: the player's before the hit pass
//!    ([`Combat::display_player`], @0x43e29), each NPC's after its own
//!    step 6 (@0x444ad). It settles the ship's condition from its armour
//!    (catching armour lost with no hit), sets an unset timer to its
//!    `DeathDelay` (three times it for the player), and destroys a ship
//!    whose timer is down to 2; a timer that never rises above 0 goes as
//!    the fight's [`Breakup`](hull::Breakup) says. The timer ticks at the
//!    ship's own step before its display (@0x6d7e6, @0x35083). So a
//!    breakup begun after a ship's display (a hit on the player, a beam)
//!    starts at its next display.
//!
//!    9b. Each ship destroyed at its display that is heavy enough sends
//!    out a shockwave at once ([`shockwave`]): every ship within its
//!    explosion's size, breaking up or not, takes its damage, never
//!    destroyed or newly disabled by it, and is pushed away from the
//!    wreck. The session takes each push as a [`Shove`]
//!    ([`Combat::take_shoves`]) and sets the ship's velocity. Each intact
//!    or disabled ship hit then settles its condition as in step 9.
//! 10. Shields and armour regenerate only on an intact ship, each up to
//!     what it holds: a disabled or breaking-up one regenerates neither.
//!     Each ship regenerates at its own step, before the beams' hits: the
//!     player after its trigger ([`Combat::regenerate_player`],
//!     `_HandlePlayer` @0x6cf86-0x6d093), each NPC at its own
//!     `_HandleShip` in step 6 (@0x34424).
//!
//! After step 10 each ship's place is kept: the next tick's hit pass meets
//! each NPC there (`Combat::hit_pass`).
//!
//! **Frame to tick.** Tick t is the original's frame t
//! (`_DoPlayGameWork` @0x43de5-0x4453d, drawn after it returns,
//! `_PrepareAllSprites` @0x45834): step 1 is its `_HandlePlayer` point
//! defence, trigger and reload countdown (@0x68c74, @0x6ab55, @0x6ae25);
//! steps 2 and 3 its collision passes (`_SWCollideSpriteLayer`
//! @0x43e70-0x44010, `_CheckShotProximities` @0x44015), testing tick
//! t−1's moves; step 4 `_AIDispatch`'s point defence (@0x4429d); step 5
//! `_HandleShot` (@0x44385): each slot's end of life (@0x36195), or its
//! move and lock (@0x35855), at its visit; step 6 `_HandleShip`'s countdown, regeneration
//! and fire (@0x33896, @0x34424, @0x34ffd, loop @0x44483), each NPC's
//! display (@0x444ad) and `_HandleBeams`' moves; steps 7 and 8
//! `_HandleBeams`' hits (@0x4453d). The player's display step (@0x43e29)
//! comes between steps 1 and 2. So a shot that meets a ship is
//! drawn over it, or at its last point, for a frame before its hit, as
//! the original draws it, and an NPC's shot is drawn where it left for a
//! frame before it moves.
//!
//! The session (`nova_sim::Session::game_work`) runs the player's move
//! after step 1, the traffic's AI between steps 3 and 4, and the NPCs'
//! motion between steps 5 and 6, as `_DoPlayGameWork` does.
//!
//! What remains of the original's order crosses no boundary the player
//! sees, and is not filed:
//!
//! - Step 4 runs after every NPC's decision, where `_AIDispatch`
//!   interleaves them slot by slot.
//!
//! The ships within the hit pass are walked in the layers the last tick's
//! layout left, which puts a ship the beams or the late steps disabled in
//! its new layer a frame early (weapons-fidelity phase 39, item 3). See
//! the `collision` module.

//! A tick runs on a [`GamePass`] ([`Combat::tick_on`]): double time's second
//! pass runs every step but the NPCs' point defence, which it skips, and
//! the beams', which only spend their life on it, and the advance pass in
//! double time doubles the beams' damage.
//!
//! Each hit on a ship (a shot's, a beam's, a blast's, a shockwave's) is
//! kept as a [`Strike`] by the ship whose it was, with the shield and
//! armour it took; the last on a ship disabled or breaking up that tick
//! says so ([`Downed`]); only a shot's or beam's is direct, which the hit
//! ship may answer ([`Strike::direct`]), and intended when it is on the
//! ship it was meant for ([`Strike::intended`]). The session takes them
//! ([`Combat::take_strikes`]) for the player's crimes and the NPCs'
//! answers.
//!
//! Each step is reported as a [`CombatEvent`] for the view to show, and
//! each unimplemented weapon feature once as a
//! [`SimDiagnostic`](report::SimDiagnostic). Nothing here is saved: the
//! shots, their targets, durability and generations live only in flight,
//! and a stellar's strength is the session's, which saves none either.

mod ai_dispatch;
pub mod ai_fire;
pub mod aim;
pub mod armament;
pub mod asteroid_beam;
pub mod asteroid_hit;
pub mod asteroid_seek;
pub mod asteroid_walk;
pub mod beam;
pub mod beam_stellar;
mod collision;
pub mod damage;
pub mod defence;
mod display;
mod escort_loss;
pub mod exits;
mod expiry;
pub mod finishing_hit;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod flags;
pub mod hidden;
pub mod hull;
pub mod impact;
pub mod ion;
pub mod ion_tint;
pub mod jamming;
mod lineup;
pub mod phases;
pub mod planet_type;
mod player_berths;
pub mod plunder;
pub mod projectile;
mod proximity;
pub mod report;
pub mod script_explosions;
pub mod ship_frames;
pub mod ship_lights;
pub mod shockwave;
pub mod shot_layer;
pub mod shot_slots;
pub mod sides;
mod source_slot;
pub mod spin_timer;
pub mod stellar;
pub mod stellar_guns;
pub mod stellar_hazards;
pub mod submunition;
pub mod tractor_hold;
pub mod weapon;

use std::collections::BTreeMap;

use aim::Pointing;
use armament::{Armament, Arsenal, Launch, Rounds, Shooter, Trigger};
use asteroid_hit::AsteroidFuse;
use asteroid_walk::AsteroidField;
use beam::{Aiming, Beam, BeamHitTest, BeamRules, ConeAngle};
pub use collision::SpriteRank;
use damage::{Blast, Hit};
use defence::{Candidate, Guard, PointDefenceRule, Quarry, Side};
pub use display::Displayed;
pub use escort_loss::EscortLoss;
use exits::{Compression, ExitCursors, ExitPoints, StrayWord};
pub use finishing_hit::FinishingHit;
use hull::{
    Breakup, Condition, DisableRule, HullSpec, armour_gone, holds_armour, immune, never_breaks_up,
};
use impact::{Body, Tractor, TractorLock};
use ion::{Burst, Dose, Ion, IonLockout};
use jamming::Jamming;
use lineup::Lineup;
pub use phases::Arena;
use planet_type::{BeamPlanetFlag, BlastKind, PlanetBlasts};
use plunder::Plunderer;
use projectile::{
    BombLaunch, DecayStep, Jam, LockAngle, ProximityFuse, Shot, ShotId, ShotLayer, Sight, Source,
    Target,
};
use report::{Reports, SimDiagnostic};
use shot_slots::ShotSlots;
use sides::{PassThrough, Sides, Ties};
use stellar::{StellarStrike, StellarTarget};
use stellar_hazards::{HazardResistance, StellarHazards, StellarKill};
use weapon::{Explosion, Guidance};

use crate::asteroid::Asteroids;
use crate::bay::FIGHTER_BAY;
use crate::catalog::{GovtId, ShipId, StellarId, WeaponId};
use crate::chance::Chance;
use crate::flight::{ShipState, normalized};
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::legal::{LegalCode, NovaLaw};
use crate::reserves::{Gauge, Reserves};
use crate::rulebook::RuleSource;
use crate::ship_layer::Slot;
use crate::speed_cap::SpeedCap;

/// An NPC's number in its system, unique while the player stays there:
/// the order it appeared in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NpcId(pub u32);

/// A ship in a fight: the player's, or an NPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShipRef {
    /// The player's ship.
    Player,
    /// This NPC.
    Npc(NpcId),
}

/// Something that happened in a fight, for the view to show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CombatEvent {
    /// A ship fired a weapon, or a shot of its released sub-munitions.
    Fired {
        /// The ship.
        ship: ShipRef,
        /// The weapon.
        weapon: WeaponId,
        /// Where it was fired from: the ship, or the shot releasing them.
        at: Vec2,
        /// How its weapon's `Sound` plays for it, if at all: its priority,
        /// and whether a looped sound waits while it plays anywhere. None
        /// for a call's later copies, which the original plays once after
        /// them all.
        sound: Option<FireSound>,
    },
    /// Point defence destroyed a missile.
    ShotDown {
        /// Where.
        at: Vec2,
    },
    /// A shot ended its life without detonating, its weapon of an
    /// `ExplodType` above none ([`WeaponSpec::fizzles`](weapon::WeaponSpec::fizzles)):
    /// it fizzles out where it is, as `_SpawnExplod(x, y, 0, 0, 0)`
    /// (`_HandleShot` @0x36149-0x36175), explosion type 0, unheard.
    Fizzled {
        /// Where.
        at: Vec2,
    },
    /// A shot or beam exploded where it hit, or a shot detonated.
    Exploded {
        /// Where.
        at: Vec2,
        /// Its explosion.
        explosion: Explosion,
        /// Its size (`_CreateExplosion`'s fourth argument, @0x3f698): how
        /// far an [`Explosion::extra`] one's extras scatter, and how many
        /// there are. A weapon's hit, beam tick or detonation passes its
        /// `BlastRadius` ([`WeaponSpec::blast_radius`](weapon::WeaponSpec::blast_radius),
        /// wëap +0x18); every other explosion passes 0
        /// (`Combat::explode`: an asteroid or a stellar destroyed, a
        /// sabotage bomb). The original's is signed, while the
        /// loaded `BlastRadius` is floored at none: a negative size
        /// scatters none either way.
        size: f32,
    },
    /// An explosion of size 0 whose sound the session has raised already,
    /// as a `SimSound::Explosion`: a stellar a `Y` destroyed (`_EvalSetExp`
    /// @0x15ab4; see the [`script_explosions`] module). Drawn as an
    /// [`Exploded`](Self::Exploded) is, its sound not played again.
    ExplodedHeard {
        /// Where.
        at: Vec2,
        /// Its explosion.
        explosion: Explosion,
    },
    /// A ship was disabled.
    Disabled {
        /// The ship.
        ship: ShipRef,
    },
    /// A ship's armour is gone, and it is breaking up.
    BreakingUp {
        /// The ship.
        ship: ShipRef,
        /// Where it is.
        at: Vec2,
        /// Its `Explode1`, if any.
        explosion: Option<Explosion>,
    },
    /// Particles were thrown off: a shot's hit on a ship, an asteroid or a
    /// stellar, a beam's on a ship, or an asteroid destroyed (see
    /// [`asteroid_hit::hit_burst`]).
    Particles(ParticleBurst),
    /// A hit landed on a ship and reached `_DamageShip`'s shield-flash
    /// write (@0x3bef9-0x3bf0c): a shot's direct hit, each ship its blast
    /// reaches, a beam tick, a death's shockwave, or a missile lock, which
    /// does no damage but flashes its target's shield all the same
    /// (@0x35855). Every path past `_DamageShip`'s entry guards reaches the
    /// write, a breaking-up ship's too; the escape pod and every class
    /// after it take none ([`immune`], @0x3a87c-0x3a882). The sabotage
    /// bomb passes shields (@0x699b2) and raises none.
    Hit {
        /// The ship hit.
        ship: ShipRef,
        /// The weapon whose shot, blast or beam it was; none for a
        /// shockwave.
        weapon: Option<WeaponId>,
        /// Where it landed, loosely: where its shot met the ship, its
        /// blast's centre, its beam's end, or the wreck whose shockwave
        /// it was.
        at: Vec2,
        /// Whether its weapon passes shields (`Flags` 0x0020), which
        /// leaves the flash as it was.
        passes_shields: bool,
        /// Whether the original shows its flash a frame later, the ship's
        /// `_HandleShipDisplay` having run that frame before the hit: any
        /// hit on the player, whose display comes first (@0x43e29); a
        /// beam's, as `_HandleBeams` runs last (@0x4453d); and a
        /// shockwave's on a ship before its wreck in the fight's order,
        /// the original's ship table's.
        late: bool,
    },
    /// A ship's weapon glow lit (`Flags2` 0x0200, ship +0xc8e8 = 32.0):
    /// the view shows its `shän` `WeapImageID` sprite at full and lets it
    /// fade by its `WeapDecay`. The player's call that fired a glow weapon
    /// lights it (`_FirePlayerWeapon` @0x62c78-0x62c98); a fighter bay's
    /// call only once a fighter it launched is put in the system
    /// (@0x62c61-0x62c6e), which the session decides and lights through
    /// [`Combat::light_glow`]; and an NPC's pick that spawned one
    /// (`_FireAIShipWeapon` @0x88dc0-0x88de4), once a call; and each
    /// live beam of one, on every advance-pass tick, for its firer
    /// (`_HandleBeams` @0x30b1f-0x30b37), but a point-defence beam's at a
    /// missile (@0x309ab-0x309bb), an escort's formation pick among them.
    /// Point defence's firing, an NPC's bay launch and a sub-munition
    /// release never light it.
    WeaponGlow {
        /// The ship.
        ship: ShipRef,
        /// Whether the original shows it a frame later, as with
        /// [`CombatEvent::Hit::late`]: a beam's, as `_HandleBeams` runs
        /// after every ship's `_HandleShipDisplay` (@0x4453d), so it shows
        /// the next frame, and once after the beam's last tick. A firing
        /// comes before its ship's display (@0x43e19, @0x44483) and shows
        /// that frame.
        late: bool,
    },
    /// A ship was destroyed: the view draws its explosion and debris.
    Destroyed {
        /// The ship.
        ship: ShipRef,
        /// Its ship type.
        ship_type: ShipId,
        /// Where it was.
        at: Vec2,
        /// How it was moving.
        velocity: Vec2,
        /// Its `Explode2`, if any.
        explosion: Option<Explosion>,
        /// The explosion's size ([`HullSpec::death_size`]): how far its
        /// extra explosions scatter, and how many there are.
        size: f32,
    },
    /// A dying NPC ejected an escape pod, one of its crew's or its
    /// person's (`_SpawnEscapePod` @0x45547-0x456cc, called from
    /// `_HandleShip` @0x351c3 and @0x352c9; see `nova_sim::npc_escape`).
    /// The pod is a sprite, no ship: the view sets it adrift from here and
    /// plays `_escapePodSnd` from here, `snd ` 372
    /// ([`ESCAPE_POD_SOUND`](nova_sim_catalog::sound::ESCAPE_POD_SOUND)),
    /// whether or not it has room for the pod, as `_HandleShip` plays it
    /// after the spawn at priority 5 (`_PlaySoundDistance` @0x351fa,
    /// @0x35300), which the view doesn't play by yet.
    PodEjected {
        /// The ship it left.
        ship: ShipRef,
        /// Where the ship was.
        at: Vec2,
        /// How the ship was moving.
        velocity: Vec2,
    },
    /// A stellar's gun fired a shot (`_HandleStellarWeapons`
    /// @0x2f553-0x2f59a, see [`stellar_guns`]): its weapon's `Sound` is
    /// heard from the stellar, once a shot, at
    /// [`STELLAR_GUN_PRIORITY`], with no looped test. A stellar is no
    /// ship, so [`CombatEvent::Fired`] can't carry it.
    StellarFired {
        /// The stellar.
        stellar: StellarId,
        /// Its weapon.
        weapon: WeaponId,
        /// Where it fired from: the stellar.
        at: Vec2,
        /// How its weapon's `Sound` plays.
        sound: FireSound,
    },
}

/// How a weapon's firing sounds: its `Sound` (`snd ` 200 + `Sound`,
/// which the view reads from the weapon's looks), played at the priority
/// the function that fires it gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FireSound {
    /// The priority it plays at (`_PlaySoundDistance`'s third argument):
    /// how much it matters when the audio side's voices are full.
    pub priority: u8,
    /// Whether the function firing it skips a looped weapon's sound
    /// (`Flags` 0x0010) while that `snd ` plays on any voice
    /// (`ST_IsSoundPlaying`), as `_FireAIShipWeapon` (@0x88e00-0x88e1d),
    /// `_FirePlayerWeapon` (@0x62d98-0x62dbf), `_HandleShipPointDefense`
    /// (@0x39981-0x3999e) and `_SpawnShotSubmunitions` (@0x3f25b-0x3f26c)
    /// do. `_AILaunchFighter` tests nothing (@0x81429-0x81466). The view or
    /// the audio side applies the test.
    pub unless_playing: bool,
}

impl FireSound {
    /// A sound at `priority`, skipped while a looped one plays.
    #[must_use]
    pub const fn at(priority: u8) -> Self {
        Self {
            priority,
            unless_playing: true,
        }
    }
}

/// The priority an NPC's pick plays its weapon's sound at, once a call
/// (`_FireAIShipWeapon` @0x88e30, `_PlaySoundDistance`). The original also
/// passes channel 3 for guidance 0 and 3, which `_PlaySoundDistance`
/// (@0xe007) never reads.
pub const AI_FIRE_PRIORITY: u8 = 4;

/// The priority an NPC's fighter bay plays its sound at, once each
/// fighter launched (`_AILaunchFighter` @0x81433, played @0x81466 after
/// `_LaunchFighter` succeeds @0x8141c-0x81423), with no looped test.
pub const BAY_LAUNCH_PRIORITY: u8 = 5;

/// The priority a stellar's gun plays its weapon's sound at, once a shot
/// (`_HandleStellarWeapons` `_PlaySoundDistance(-1, snd, 5, …)`
/// @0x2f59a), with no looped test ([`CombatEvent::StellarFired`]).
pub const STELLAR_GUN_PRIORITY: u8 = 5;

/// The priority a shot's released sub-munitions play their weapon's sound
/// at, once a release, at the parent (`_SpawnShotSubmunitions` @0x3f23b-0x3f2f2,
/// `_PlaySoundDistance(-1, snd, 4, …)`). Under the sub-munition's `Flags`
/// 0x0010 the original plays it only when that sound is not already
/// playing (`_ST_IsSoundPlaying` @0x3f264), which the view or the audio
/// side applies ([`FireSound::unless_playing`]).
pub const SUBMUNITION_PRIORITY: u8 = 4;

/// The priority point defence plays its weapon's sound at, for a shot or
/// a beam (`_HandleShipPointDefense` @0x399b0, @0x399df,
/// `_PlaySoundDistance(-1, snd, 3, …)`): `snd ` 200 + `Sound`, 212 for
/// the stock Quad Light Blaster Turret and 223 for the Storm Chaingun.
/// Under `Flags` 0x0010 the original plays it only when that sound is not
/// already playing (@0x39981-0x3999e), which the view or the audio side
/// applies ([`FireSound::unless_playing`]).
pub const PD_FIRE_PRIORITY: u8 = 3;

/// A burst of particles: `_SpawnParticles`' arguments (@0x44b70-0x44d4e),
/// for the view to draw. Each of `count` particles starts at `at`, or
/// within `spread` of it, and flies off at `speed`, varied by
/// `variation` percent either way, in a direction of its own, living a
/// tick count from `life`'s first to its second. None takes on any
/// source's velocity.
///
/// It carries no alpha: the original's argument after the colour
/// (0x1c(%esp)), passed straight on to `_NewParticle` (+0x24, @0xc50de),
/// is only tested against 0x20 by `_DrawParticlesOffscreen` (@0xc5a61,
/// @0xc5b2b), whose paths draw the same either way, every particle opaque
/// and then fading over its last 32 ticks of life. A shot's trail
/// (@0x35ea6), the only caller passing another, is the view's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleBurst {
    /// Where it goes off.
    pub at: Vec2,
    /// Each particle's speed, in pixels a tick.
    pub speed: f32,
    /// How far, in percent, a particle's speed varies either way: none
    /// unless 1-99.
    pub variation: u32,
    /// A particle's least and most life, in ticks.
    pub life: (u32, u32),
    /// Its colour, `00RRGGBB`.
    pub color: u32,
    /// How many particles.
    pub count: u32,
    /// How far from `at` a particle may start, in pixels: none for all
    /// at `at`.
    pub spread: f32,
}

/// What a fight's tick may hit besides ships: the destroyable stellars
/// shown, which the session writes back what the shots left of, and the
/// asteroid field, lent whole.
#[derive(Debug, Default)]
pub struct Scenery<'a> {
    /// The stellars ([`stellar`]).
    pub stellars: &'a mut [StellarTarget],
    /// The asteroid field the shots wear and destroy in place
    /// ([`asteroid_walk`]): none when none is lent.
    pub asteroids: Option<AsteroidField<'a>>,
    /// The stellar hazards ([`stellar_hazards`]): none when there are
    /// none.
    pub hazards: Option<&'a StellarHazards>,
}

/// How a strike left the ship it hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Downed {
    /// It disabled the ship.
    Disabled,
    /// It took the last of the ship's armour: the ship is breaking up.
    BreakingUp,
}

/// A hit on a ship: who made it, what it took, and whether it downed
/// the ship.
// Each flag is its own, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strike {
    /// The ship hit.
    pub ship: ShipRef,
    /// The ship whose shot, beam, blast or shockwave it was, or that
    /// boarded it; none for a shot no ship launched (`_DamageShip`'s
    /// source -1), such as one turned on its firer, whose hit draws no
    /// target (it still provokes its victim and the victim's lead, see
    /// `nova_sim::ai::react::victim`), is no crime and credits no kill.
    pub by: Option<ShipRef>,
    /// The shield and armour it took.
    pub damage: f32,
    /// How it left the ship, for the last hit on a ship whose condition
    /// changed that tick; none otherwise.
    pub downed: Option<Downed>,
    /// Whether it was a direct hit, which the ship hit may answer: the
    /// original's `_DamageShip` arg +0x20 ("real"). A shot's or beam's hit
    /// is (`_HandleShipHit` @0x36c43, `_HandleBeams` @0x314be); a
    /// blast's, whether it went off on a hit or as its shot expired
    /// (@0x36e80, @0x360cb), and a death shockwave's ([`shockwave`]) are
    /// not, nor is a boarding's Good Samaritan call
    /// (`nova_sim::Session::board`), which is answered but never passes
    /// `_DamageShip`, and which the ship boarded never answers. So only a direct hit raises a grudge,
    /// cloaks or uncloaks its ship and provokes an AI (the reaction block
    /// @0x3b44d and the decloak @0x3bf1f skip the rest), though the player
    /// is convicted of what any hit does (@0x3ad4f-0x3ae53).
    pub direct: bool,
    /// Whether the ship hit is the one it was meant for: `_DamageShip`'s
    /// arg +0x24. A shot's direct hit is on the ship its damage is aimed
    /// at ([`Shot::aimed_at`], shot +0x2a, @0x36bc5-0x36c37); a beam's on
    /// the ship it was fired at ([`Beam::aimed_at`], beam +0x18), or, for
    /// one fired at nothing, its firer's target at the hit (+0x70,
    /// @0x31463-0x315b4); a boarding's call always is. No blast or
    /// shockwave is.
    /// Only an intended strike by the player calls the Good Samaritans
    /// (@0x3bed2, see `nova_sim::ai::react::answer`).
    pub intended: bool,
    /// Whether it is a missile lock's: the no-damage strike a player's
    /// homing missile makes on its target as it passes
    /// [`LOCK_AGE`](projectile::LOCK_AGE) (`_HandleShot` @0x35855), which
    /// the original makes with damage -32000, a sentinel `_DamageShip`'s
    /// zero-damage rule lets through (@0x3b716). It is direct and
    /// intended, kills and disables nothing, and downs no ship.
    pub lock: bool,
    /// The damage it was passed with, not what it took: `_DamageShip`'s
    /// args +0x14 and +0x18, which the answer flag's no-damage branch
    /// reads (see `nova_sim::ai::answer_flag`). A shot's direct hit
    /// passes its weapon's damage less its decay steps (`_HandleShipHit`
    /// @0x36ac6-0x36afd), a beam its damage on the pass, a blast its
    /// weapon's whole damage, a shockwave its damage both ways
    /// (@0x2da89), and a missile lock none, which the flag excepts by
    /// [`lock`](Strike::lock) as the original's -32000 passes (@0x3b716).
    pub nominal: Nominal,
    /// What a player's stray hit adds, × 1.75, to its ship's stray
    /// provocation: `_DamageShip`'s arg +0x34, the in-memory `wëap` +0x30
    /// that `_LoadObjectData` fills with `float(Reload)` (@0x792cf). A
    /// shot's direct hit, its expiry blast (@0x360a2) and the missile lock
    /// (@0x357f8) pass their weapon's `Reload`, a beam 1, and a blast on a
    /// hit, an asteroid's blast and a shockwave none.
    pub weight: f32,
    /// Whether the ship hit answers it: `_DamageShip`'s answer flag
    /// (-0x178) and the reaction block's entry test (@0x3bb20-0x3bb74), as
    /// the session's settle step works it out (see
    /// `nova_sim::ai::answer_flag`). The fight leaves it unset. Only an
    /// answered strike draws a reaction, a grudge, cloak-when-hit or a
    /// Good Samaritan. Every answered hit is [`direct`](Strike::direct)
    /// (@0x3b44d); the boarding's call alone is answered but not direct,
    /// drawing the Good Samaritans and no reaction.
    pub answered: bool,
}

/// The damage a hit was passed with ([`Strike::nominal`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Nominal {
    /// Damage to armour: `_DamageShip`'s arg +0x14.
    pub mass: f32,
    /// Damage to shields: `_DamageShip`'s arg +0x18.
    pub energy: f32,
}

/// A fighter launched from a bay this tick, for the session to put in
/// the system (see [`bay`](crate::bay)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sortie {
    /// The ship that launched it.
    pub carrier: ShipRef,
    /// The bay it left from.
    pub bay: WeaponId,
    /// Its ship type.
    pub ship: ShipId,
    /// Where the carrier was and how it moved.
    pub from: ShipState,
    /// The heading it leaves along, the bay's inaccuracy included, in
    /// degrees.
    pub heading: f32,
    /// The carrier's target, if any.
    pub target: Option<ShipRef>,
}

/// A ship the fight pushed this tick, for the session to set its
/// velocity: a [`Fighter`]'s state is a copy. Every hit, blast and beam
/// tick of a weapon of some `Impact`, a tractor beam's self-pull and a
/// death shockwave push ([`impact`]), and so does an NPC's recoil
/// ([`ai_fire`]); a ship pushed more than once in a tick has a shove for
/// each, in order, the last holding where they left it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shove {
    /// The ship pushed.
    pub ship: ShipRef,
    /// Its velocity after the push.
    pub velocity: Vec2,
}

/// One ship in a fight, for a tick: what it is, where, what it holds and
/// how it is holding up, borrowed from wherever the ship is kept.
// Each flag is its own fact about the ship, read on its own.
#[allow(clippy::struct_excessive_bools)]
pub struct Fighter<'a> {
    /// Which ship.
    pub ship: ShipRef,
    /// Its ship type.
    pub ship_type: ShipId,
    /// Its fleet: the lead it escorts, or itself.
    pub fleet: ShipRef,
    /// Its government, or `None` for an independent ship or the player.
    pub govt: Option<GovtId>,
    /// The ship it follows (the original's +0x9a): the player for a ship
    /// that escorts the player, else the lead it escorts or the carrier
    /// it left; none for the player and a ship led by no one.
    pub leader: Option<ShipRef>,
    /// Whether it is a ship of the player's fleet: led by the player with
    /// no mission mark (`_DamageShip` @0x3af8f-0x3afa5). The hit that
    /// leaves it disabled clears its leader in place, so the rest of the
    /// tick reads it unled (see the `escort_loss` module). Never the
    /// player's own.
    pub fleet_escort: bool,
    /// Its `düde`'s `Booty`, or `None` when it has no `düde` (the player
    /// has none).
    pub booty: Option<u16>,
    /// Where it is and how it moves.
    pub state: ShipState,
    /// Its hull.
    pub hull: HullSpec,
    /// The shield points it regenerates a tick.
    pub shield_regen: f32,
    /// The armour points it regenerates a tick: for the player, none
    /// while its repair inhibit holds
    /// ([`RepairInhibit`](crate::self_repair::RepairInhibit)).
    pub armor_regen: f32,
    /// The fire command it holds.
    pub trigger: Trigger,
    /// The ship it targets, if any.
    pub target: Option<ShipRef>,
    /// Its shield, armour and fuel.
    pub reserves: &'a mut Reserves,
    /// How it is holding up.
    pub condition: &'a mut Condition,
    /// Whether it is held disabled whatever its armour: a rescue
    /// mission's ship not yet boarded (see the session's `mission_ships`
    /// module). It is kept disabled until its armour is gone, never
    /// newly disabled, and the [`DisableRule`] is not asked of it.
    pub held_disabled: bool,
    /// Whether it is cloaked: it fires only its weapons that fire while
    /// cloaked ([`Armament::has_ammo`]; the player's fire,
    /// `_FirePlayerWeapon` @0x627f5-0x62814, and its point defence
    /// @0x3931c-0x3932f, too), and point defence sees it only with a cloak
    /// scanner ([`defence::choose_ship`]).
    pub cloaked: bool,
    /// The ship it attacks or snipes at, which point defence reads as
    /// fighting that ship (`_IsShipFightingMe` @0x827fa: the original's
    /// +0x70 in AI state 4; [`defence::choose_ship`]); fleeing never
    /// counts. None for the player, whose own is never read.
    pub attacking: Option<ShipRef>,
    /// Whether it threatens the player (`_ExtendedIsThreatToPlayer`
    /// @0x81b70): only then does its point defence fire at the player
    /// ([`defence::choose_ship`]). Never the player's own.
    pub threatens_player: bool,
    /// Whether it plunders, a pirate's goal 0xd (`_AIShipIsPlundering`
    /// @0x89841): its shots leave disable-only, its beams at its target
    /// too, and its and its escorts' hits on their targets can't kill
    /// ([`plunder`]). Never the player.
    pub plundering: bool,
    /// Whether it is docked with the ship it plunders, a pirate's goal 4
    /// with sub-goal 0xf (`_AIShipIsPlunderingShip`'s other half,
    /// @0x83f6e): its shots and beams at its target leave disable-only
    /// while that target is intact, but its hits can kill and its shots
    /// at other ships are its weapons' own ([`plunder`]). Never the
    /// player.
    pub docked: bool,
    /// Whether it is an NPC mining asteroid slot 0, the original's state
    /// 16 (`_AIShipIsAttackingAsteroid` @0x8982b): its shots and beams
    /// pass through every ship ([`Ties::mining`], see
    /// [`sides`](self::sides)). Never the player.
    pub mining: bool,
    /// Whether it waits in the gate it comes out of: point defence never
    /// sees it (`_ShipVisibleToShip` @0x972d). The player never does.
    pub emerging: bool,
    /// Whether it is jumping in, goal 8 (`_AIIsShipJumpingIn` @0x82f12):
    /// gliding in, or bursting out of its gate from its first moving tick.
    /// A stellar's gun holds fire at it (`_IsThreatToStellar`
    /// @0x9f16-0x9f2a, [`stellar_guns`]). Never the player.
    pub jumping_in: bool,
    /// Whether it has a cloak scanner on screen
    /// ([`Scanner::on_screen`](crate::cloak::Scanner::on_screen)): its
    /// point defence sees a cloaked ship within 200 pixels on each axis
    /// (`_ShipVisibleToShip` @0x97a4-0x97f8), and so do its homing shots
    /// ([`Sight::scanner`]).
    pub cloak_scanner: bool,
    /// Whether it has a cloak (`_HasCloak`): a homing shot it fires at its
    /// leader sees the leader cloaked (`_ShipVisibleToShip` @0x9788-0x97a2,
    /// [`Sight::has_cloak`]). An NPC's is its own cloak, or the area branch
    /// (@0x964c-0x96bf): its leader's area cloak it rides in close
    /// formation ([`area_cloak::has_cloak`](crate::area_cloak::has_cloak)).
    /// The player's reads the outfits it owns; it has no leader, so nothing
    /// reads it yet.
    pub has_cloak: bool,
    /// How strongly it jams each jamming type (`_ShipECM`, see
    /// [`jamming`]), whatever its condition: a disabled ship jams nothing.
    pub jamming: Jamming,
    /// Its class's top speed, in pixels a tick: its `shïp` `Speed` alone,
    /// none of its outfits' (the original's class +0x40), which every
    /// push soft-caps against ([`impact::push`]).
    pub class_speed: f32,
    /// Its top speed, in pixels a tick, outfits and lead included (the
    /// original's `_ShipMaxSpeed`): every push is clamped to it on each
    /// axis.
    pub top_speed: f32,
    /// Whether its jump is under way, the pre-jump stage included (the
    /// original's +0x50 above 0): no hit, blast, beam or shockwave pushes
    /// it, though a tractor still holds it and it still pulls itself
    /// along its own tractor beam, and its point defence rests
    /// (`_HandleShipPointDefense` @0x392d6). The player's from J on, an
    /// NPC's through its warp stage, its own jump or its leader's.
    pub jumping: bool,
    /// Whether it is the player with its afterburner widening a hit's, a
    /// blast's and the shockwave's clamp to 1.8 times its top speed
    /// ([`Body::afterburner`](impact::Body::afterburner)); never an NPC's.
    /// Its session decides it, a gravity source in the system as
    /// `nova_sim::RuleKey::GravityAfterburner` reads.
    pub afterburner: bool,
    /// Whether it is the player with its afterburner widening a beam
    /// tick's clamp, as [`Fighter::afterburner`] does the others: only
    /// with no gravity source in the system, as `_HandleBeams` runs after
    /// `_HandleGravity` sets `_gravitySourcesPresent` (@0x316a1,
    /// @0x39c27); never an NPC's.
    pub afterburner_beam: bool,
    /// Its tractor lock, if a tractor beam holds it, or it holds itself
    /// along its own (the original's +0xc8dc and +0xb4): the fight sets
    /// and touches it ([`impact`]); what it does to the ship is the
    /// session's to come.
    pub tractor: &'a mut Option<TractorLock>,
    /// Its ionization (the original's +0x5c and +0xb0): the fight raises
    /// it ([`ion`]) and holds its `Seeker` 0x0020 weapons by it
    /// ([`IonLockout`]); its session sheds it and slows the ship by it.
    pub ion: &'a mut Ion,
    /// The ion charge at which it is fully ionized, its ionization
    /// factor's divisor (`_ShipIonizationFactor`): its class's
    /// `IonizeMax`, and the player's Ion Absorbers'.
    pub ion_capacity: f32,
    /// Its weapons.
    pub armament: &'a mut Armament,
    /// Its rounds of ammunition.
    pub rounds: &'a mut dyn Rounds,
    /// The heading it was last drawn at: its heading as the last tick
    /// ended. Its shots leave from the exit points of the frame drawn at
    /// it, as every ship fires before the original draws it again
    /// (`_DoPlayGameWork` @0x43e19-0x43e29, @0x44483-0x444ad; see
    /// [`exits`]).
    pub drawn_heading: f32,
    /// Where its slot sprite stands as the collision passes walk the
    /// ships: its layer and its entry there ([`SpriteRank`]), read once as
    /// the pass begins, or none for a hidden ship, whose sprite
    /// `_HandleShipDisplay` hides, so no contact pass meets it (@0x2cc6b,
    /// `_SWCollideSpriteLayer` @0xacba3), though a proximity fuse still
    /// can.
    pub sprite: Option<SpriteRank>,
    /// Its ship slot: the player's, or its berth in the ship table. A
    /// sub-munition reads whatever ship holds its parent's source slot as
    /// it spawns ([`Shot::firer_slot`]).
    pub slot: Slot,
    /// Which stellar hazards it resists ([`stellar_hazards`]).
    pub resists: HazardResistance,
}

/// The rules a fight is fought by: when a ship is disabled, which
/// missiles point defence engages by the other reading of
/// [`RuleKey::PdHostility`](crate::RuleKey::PdHostility), and what the
/// player's crimes do to its legal record.
#[derive(Clone, Copy, Debug)]
pub struct Rules<'a> {
    /// When a ship is disabled.
    pub disable: &'a dyn DisableRule,
    /// Which missiles point defence engages, asked only by the other
    /// reading of [`RuleKey::PdHostility`](crate::RuleKey::PdHostility).
    pub defence: &'a dyn PointDefenceRule,
    /// What the player's crimes do to its legal record.
    pub law: &'a dyn LegalCode,
}

impl Default for Rules<'static> {
    /// Nova's: [`NovaDisable`](hull::NovaDisable), point defence by
    /// [`Allegiance`](defence::Allegiance) where the fight asks it, and
    /// [`NovaLaw`].
    fn default() -> Self {
        Self {
            disable: &hull::NovaDisable,
            defence: &defence::Allegiance,
            law: &NovaLaw {
                crime_gains: RuleSource::Engine,
            },
        }
    }
}

/// What a fight reads of when and where it is fought, each tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Field {
    /// The frame counter (`_frameCounter`) as `_DoPlayGameWork` sees it,
    /// before `_PlayGame` adds 1: what an interfered shot circles by
    /// ([`Jam::Interfered`]).
    pub frame: i32,
    /// The player's system's `sÿst` `Interference`, raw (+0x96), not less
    /// any anti-interference: what a `Seeker` 0x0008 homing shot rolls
    /// against as it is launched, whoever fires it (`_SpawnShot`
    /// @0x3eab2-0x3eb66).
    pub interference: i16,
    /// The player's combat rating against `shïp` 128's `Strength`: what
    /// stretches the reload of an NPC's pick fired at the player
    /// ([`ai_fire::PlayerRating::stretch`], `_FireAIShipWeapon`
    /// @0x88fb3-0x8901b). The default stretches nothing.
    pub player_rating: ai_fire::PlayerRating,
}

/// A hit that landed on a ship in the hit pass, step 5 or step 8, before it
/// is taken.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Landed {
    /// The ship hit.
    ship: ShipRef,
    /// The ship whose shot, blast or beam it was, if a ship launched it
    /// ([`Strike::by`]).
    by: Option<ShipRef>,
    /// The weapon whose shot, blast or beam it was.
    weapon: Option<WeaponId>,
    /// Where it landed, loosely: where its shot
    /// met the ship, its blast's centre, its beam's end, or, for a missile
    /// lock, the ship.
    at: Vec2,
    /// What it does.
    hit: Hit,
    /// Whether it was a shot's or beam's own hit ([`Strike::direct`]).
    direct: bool,
    /// Whether it is on the ship it was meant for ([`Strike::intended`]).
    intended: bool,
    /// Whether it is a missile lock's ([`Strike::lock`]).
    lock: bool,
    /// What it weighs as a player's stray hit ([`Strike::weight`]).
    weight: f32,
    /// How it pushes the ship.
    push: Push,
    /// What it ionizes the ship by (`_IonizeShip`'s weapon and point):
    /// none for a missile lock.
    ion: Option<Dose>,
}

/// How a hit that landed pushes its ship ([`impact`]): `_DamageShip`'s
/// point and `Impact`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Push {
    /// Where it pushes the ship from (`_DamageShip`'s arg +0xc): where
    /// its shot met the ship, its blast's centre, or its beam's start.
    from: Vec2,
    /// Its weapon's `Impact` ([`WeaponSpec::impact`](weapon::WeaponSpec::impact)):
    /// none for a missile lock.
    impact: f32,
    /// Whether it is a beam's, whose negative `Impact` makes it a tractor
    /// beam ([`impact::tractor`]).
    beam: bool,
}

/// The share of a shot's mass and energy damage that counts as aimed at
/// its ship (0.5 @0xdd128, `_CalculateTargetedDamage`).
pub const AIMED_DAMAGE_SHARE: f64 = 0.5;

/// Which of the original's game-work passes a fight's tick is
/// (`_DoPlayGameWork`'s argument, @0x43dee-0x43df2; see the session's
/// `double_time` module).
///
/// Every step runs on both passes but the beams' (`_HandleBeams`
/// @0x30295): on the second pass a beam only spends its life, fading as
/// on the advance pass but spending only a life above none, so none ends
/// on it ([`Beam::live_on`], @0x30358-0x30388); it is neither moved onto
/// its firer nor hits (@0x303a8-0x303af). On the advance pass in double time, a beam's
/// damage to a ship (@0x31191-0x311a3) and a point-defence beam's to a
/// missile (@0x30a70-0x30a7f) are doubled, both damage fields, to make
/// up for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamePass {
    /// The frame's own pass (`_DoPlayGameWork(1)`), double time on or
    /// not.
    Advance {
        /// Whether double time is on (`_doubleTime`).
        double_time: bool,
    },
    /// Double time's second pass (`_DoPlayGameWork(0)`).
    Again,
}

impl GamePass {
    /// Whether the beams follow their firers and hit on this pass.
    fn beams_act(self) -> bool {
        matches!(self, Self::Advance { .. })
    }

    /// What a beam's damage is multiplied by on this pass.
    fn beam_damage(self) -> f32 {
        f32::from(self.beam_times())
    }

    /// [`GamePass::beam_damage`], whole, as an asteroid's strength takes
    /// it ([`asteroid_beam`]).
    fn beam_times(self) -> i16 {
        match self {
            Self::Advance { double_time: true } => 2,
            _ => 1,
        }
    }
}

/// The shots and beams in flight, and what the fight has reported.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Combat {
    shots: Vec<Shot>,
    beams: Vec<Beam>,
    events: Vec<CombatEvent>,
    strikes: Vec<Strike>,
    /// The ships of the player's fleet lost as the strikes landed, each
    /// with its strike's index ([`Combat::take_landed`]).
    escort_losses: Vec<EscortLoss>,
    /// The hits that started their ships dying as the strikes landed,
    /// each with its strike's index ([`Combat::take_landed`]).
    finished: Vec<FinishingHit>,
    stellar_strikes: Vec<StellarStrike>,
    /// The words outside the fight that stray beams stepped this tick,
    /// with their firers ([`Combat::take_stray_steps`]).
    stray_steps: Vec<(ShipRef, StrayWord)>,
    sorties: Vec<Sortie>,
    shoves: Vec<Shove>,
    /// The display steps run since they were last taken
    /// ([`Combat::take_displays`]).
    displays: Vec<Displayed>,
    reports: Reports,
    /// The next shot's number.
    next_shot: u32,
    /// What becomes of a breakup that never reaches the explosion band.
    breakup: Breakup,
    /// Who counts as the player for the pass-through flags.
    pass_through: PassThrough,
    /// Which ships a blast reaches by their planet type.
    planet_blasts: PlanetBlasts,
    /// Which of its weapon's bits makes a beam planet-type.
    beam_planet_flag: BeamPlanetFlag,
    /// Whether a planet-type beam hits a stellar
    /// ([`RuleKey::BeamStellars`](crate::RuleKey::BeamStellars)).
    beam_stellars: RuleSource,
    /// How often a decaying shot takes a step.
    decay: DecayStep,
    /// Whether an NPC's pick spends on a copy that spawns nothing
    /// ([`RuleKey::AiSpendWithoutShot`](crate::RuleKey::AiSpendWithoutShot)).
    ai_spend: RuleSource,
    /// The ship berths free for the player's fighters this tick
    /// ([`Combat::set_player_berths`]).
    player_berths: player_berths::PlayerBerths,
    /// How an NPC's quadrant turret measures its arc
    /// ([`RuleKey::QuadrantTurretArc`](crate::RuleKey::QuadrantTurretArc)).
    quadrant_arc: RuleSource,
    /// Which compression an exit point takes
    /// ([`RuleKey::ExitCompression`](crate::RuleKey::ExitCompression)).
    compression: Compression,
    /// How many exits a point-defence shot moves its cursor on
    /// ([`RuleKey::PdExitStep`](crate::RuleKey::PdExitStep)).
    pd_exit_step: RuleSource,
    /// Whether a beam of stray `ExitType` steps the word it lands on
    /// ([`RuleKey::StrayBeamExit`](crate::RuleKey::StrayBeamExit)).
    stray_beam_exit: RuleSource,
    /// How beams hit ([`RuleKey::BeamHitTest`](crate::RuleKey::BeamHitTest),
    /// [`RuleKey::BeamConeQuirks`](crate::RuleKey::BeamConeQuirks)).
    beam_rules: BeamRules,
    /// How an NPC's bombs and rockets leave it
    /// ([`RuleKey::BombRocketLaunch`](crate::RuleKey::BombRocketLaunch)).
    bomb_launch: RuleSource,
    /// Whether a bomb's heading is wrapped after it turns into the wind
    /// ([`RuleKey::BombHeadingWrap`](crate::RuleKey::BombHeadingWrap)).
    bomb_heading_wrap: RuleSource,
    /// Whether point defence asks the fight's
    /// [`PointDefenceRule`] which missiles are hostile
    /// ([`RuleKey::PdHostility`](crate::RuleKey::PdHostility)).
    pd_hostility: RuleSource,
    /// The field of the tick under way, as [`Combat::tick`] was given it.
    field: Field,
    /// How a `Seeker` 0x4000 shot measures its lock
    /// ([`RuleKey::SeekerLockAngle`](crate::RuleKey::SeekerLockAngle)).
    lock_angle: LockAngle,
    /// Which asteroids an armed proximity fuse is set off by.
    asteroid_fuse: AsteroidFuse,
    /// How a proximity fuse is set off by a ship
    /// ([`RuleKey::ProximityFuse`](crate::RuleKey::ProximityFuse)).
    proximity_fuse: ProximityFuse,
    /// Where the player stood as the last tick closed, which the next
    /// tick's contact and point defence's sprites meet it at (see
    /// `Combat::hit_pass`): `_HandlePlayer` moves it before the collision
    /// passes. Every NPC is met where it is, as the original reads each
    /// ship's record live (`_HandleShot`'s blasts @0x3603c-0x3605b).
    player_stood: Option<Vec2>,
    /// What the asteroids the shots and the beams destroyed left, since
    /// it was last taken, in the order they were destroyed
    /// ([`Combat::take_asteroid_remains`]).
    asteroid_remains: Vec<crate::asteroid::Remains>,
    /// Whether the player's quadrant turrets test their blind spots
    /// ([`RuleKey::QuadrantTurretBlindSpots`](crate::RuleKey::QuadrantTurretBlindSpots)).
    quadrant_blind: RuleSource,
    /// Whether a player's weapon of negative `Recoil` pushes it forward
    /// ([`RuleKey::PlayerForwardRecoil`](crate::RuleKey::PlayerForwardRecoil)).
    forward_recoil: RuleSource,
    /// Whether the original's shot and beam tables cap them
    /// ([`RuleKey::CombatCaps`](crate::RuleKey::CombatCaps)'s engine
    /// reading); by default, not.
    capped: bool,
    /// How a push clamps the speed of the ship it moves
    /// ([`RuleKey::SpeedCap`](crate::RuleKey::SpeedCap)): by default on
    /// each axis, as `_SpeedLimit` does.
    speed_cap: SpeedCap,
    /// Which weapons an ionized ship can't fire
    /// ([`RuleKey::IonFireLockout`](crate::RuleKey::IonFireLockout)).
    ion_lockout: IonLockout,
    /// The original's shot slots ([`shot_slots`]). Each shot that leaves
    /// the table holds its slot until the next `_HandleShot` visits it
    /// (see [`Combat::shot_room`]): from the late part of a tick (step 7)
    /// to the next tick's `_HandleShot` ([`Combat::fly_shots`]), the
    /// missiles point-defence beams downed; from the collision passes on,
    /// those and the passes' own ends, as each is ended. Those point
    /// defence ended are also kept in `hidden` until a proximity pass
    /// tests them, and hold their slots here alone.
    slots: ShotSlots,
    /// The order each shot layer is drawn and walked in
    /// ([`RuleKey::ShotOverlap`](crate::RuleKey::ShotOverlap)): by the
    /// engine, the order its slots entered it; by the other reading,
    /// firing order.
    shot_overlap: RuleSource,
    /// Whether a shot that writes no miss roll carries its slot's leftover
    /// ([`RuleKey::ReusedShotSlotRoll`](crate::RuleKey::ReusedShotSlotRoll)):
    /// by the engine, it does.
    reused_roll: RuleSource,
    /// Whether a shot starts on its slot's leftover spin timer
    /// ([`RuleKey::ReusedShotSlotSpin`](crate::RuleKey::ReusedShotSlotSpin)):
    /// by the engine, it does.
    reused_spin: RuleSource,
    /// Each ship slot's record of the last ship in it, which a
    /// sub-munition reads as it spawns ([`source_slot`]).
    slot_ships: BTreeMap<Slot, source_slot::SlotShip>,
    /// Whether a sub-munition reads its source slot's ship
    /// ([`RuleKey::SubmunitionSourceSlot`](crate::RuleKey::SubmunitionSourceSlot)):
    /// by the engine, it does.
    source_slot: RuleSource,
    /// The shots point defence ended at life 0, their sprites hidden,
    /// kept for the next proximity pass, where each takes its turn in its
    /// slot among the live shots (see [`hidden`]).
    hidden: Vec<Shot>,
    /// The beams that ended in the last tick's step 6 and after: under the
    /// caps each still holds its slot through the next tick's firing (see
    /// [`Combat::beam_room`]).
    ended_beams: usize,
    /// The ships deadly stellars killed since they were last taken
    /// ([`Combat::take_stellar_kills`]).
    stellar_kills: Vec<StellarKill>,
}

/// The original's shot table (`_SpawnShot` @0x3e5a3-0x3e5d5): under the
/// caps ([`RuleKey::CombatCaps`](crate::RuleKey::CombatCaps)) a shot finding
/// it full does not spawn.
pub const MAX_SHOTS: usize = 128;

/// The original's beam table (`_SpawnBeam` @0x4505e-0x452e2): under the
/// caps a beam finding it full does not spawn.
pub const MAX_BEAMS: usize = 64;

/// The priority the player's weapon plays its sound at, once a call
/// (`_FirePlayerWeapon` @0x62e74, `_PlaySoundDistance(-1, snd, 5, …)`):
/// a primary weapon's, but a beam's.
pub const PLAYER_FIRE_PRIORITY: u8 = 5;

/// The priority the player's beam (guidance 0 and 3), fighter bay or
/// secondary weapon plays its sound at (`_FirePlayerWeapon` @0x62dff,
/// @0x62e58). A beam passes channel 2 (@0x62e1a), which
/// `_PlaySoundDistance` never reads.
pub const PLAYER_SECONDARY_FIRE_PRIORITY: u8 = 6;

/// The `AmmoType` that destroys its firer as its shot spawns
/// (`_SpawnShot` @0x3f005, the binary's only compare with -999).
pub const SELF_DESTRUCT: i16 = -999;

impl Combat {
    /// Advances the fight among `fighters` and `scenery` a tick (see the
    /// module docs), by `rules`, in `field`, with the relations between
    /// governments in `govts`, each sub-munition's weapon read from
    /// `arsenal` and each shot's inaccuracy and spread drawn on `chance`.
    /// Each stellar's strength, and each asteroid's strength and velocity,
    /// is left as the shots left it. It is the advance pass at single
    /// speed ([`Combat::tick_on`]).
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        fighters: &mut [Fighter],
        arsenal: &Arsenal,
        govts: &Governments,
        rules: Rules,
        field: Field,
        scenery: Scenery<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let pass = GamePass::Advance { double_time: false };
        self.tick_on(
            pass, fighters, arsenal, govts, rules, field, scenery, chance,
        );
    }

    /// Advances the fight a tick as [`Combat::tick`] does, on `pass`,
    /// which the beams' step reads (see [`GamePass`]). It lends the beams
    /// no asteroid field ([`Combat::close_tick`]): the session runs the
    /// phases itself and lends it ([`Combat::close_tick_in`]).
    #[allow(clippy::too_many_arguments)]
    pub fn tick_on(
        &mut self,
        pass: GamePass,
        fighters: &mut [Fighter],
        arsenal: &Arsenal,
        govts: &Governments,
        rules: Rules,
        field: Field,
        scenery: Scenery<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let hazards = scenery.hazards;
        let arena = Arena {
            pass,
            arsenal,
            govts,
            rules,
            field,
        };
        self.displays.clear();
        self.defend_player(fighters, arena, chance);
        self.fire_player(fighters, arena, chance);
        self.regenerate_player(fighters, arena);
        for fighter in fighters
            .iter_mut()
            .filter(|fighter| fighter.ship == ShipRef::Player)
        {
            fighter.condition.count_down();
        }
        self.display_player(fighters, arena);
        let Scenery {
            stellars,
            asteroids,
            hazards: _,
        } = scenery;
        let collided = Scenery {
            stellars: &mut *stellars,
            asteroids,
            hazards,
        };
        self.test_hits(fighters, arena, collided, chance);
        self.defend_npcs(fighters, arena, None, false, chance);
        self.fly_shots(fighters, arena, chance);
        self.handle_npcs(fighters, arena, chance);
        if let Some(hazards) = hazards {
            self.hazards(fighters, hazards);
        }
        self.close_tick(fighters, arena, stellars, chance);
    }

    /// Marks the last strike on `ship` since `first_strike`, but a missile
    /// lock's, as `downed` it, if it was.
    fn mark_downed(&mut self, first_strike: usize, ship: ShipRef, downed: Option<Downed>) {
        let last = self.strikes[first_strike..]
            .iter_mut()
            .rev()
            .find(|strike| strike.ship == ship && !strike.lock);
        if let (Some(downed), Some(last)) = (downed, last) {
            last.downed = Some(downed);
        }
    }

    /// Step 6's beams: each spends a tick of its life on `pass`, and on
    /// the advance pass follows its firer among `fighters`, at `targets`,
    /// turned by its jitter drawn on `chance` (see the module docs), and
    /// lights its firer's weapon glow if it glows ([`Beam::glows`]), a
    /// frame late ([`CombatEvent::WeaponGlow`]).
    fn move_beams(
        &mut self,
        fighters: &[Fighter],
        targets: &[Target],
        pass: GamePass,
        chance: &mut dyn Chance,
    ) {
        let shots = &self.shots;
        let compression = self.compression;
        let advance = pass.beams_act();
        self.beams.retain_mut(|beam| {
            if !beam.live_on(advance) {
                return false;
            }
            // On double time's second pass a beam only spends its life: it
            // neither follows nor hits (`_HandleBeams` @0x303a8-0x303af).
            if !advance {
                return true;
            }
            let firer = fighters
                .iter()
                .find(|fighter| fighter.ship == beam.firer && fighter.condition.active())
                .map(|firer| (&firer.state, firer.hull.exits.as_ref()));
            beam.follow(firer, compression, targets, shots, chance)
        });
        if advance {
            let glowing = self.beams.iter().filter(|beam| beam.glows());
            let lit: Vec<CombatEvent> = glowing
                .map(|beam| CombatEvent::WeaponGlow {
                    ship: beam.firer,
                    late: true,
                })
                .collect();
            self.events.extend(lit);
        }
    }

    /// Step 6's stamp: on the advance pass each live beam stamps its
    /// firer's last shot now, as `_HandleBeams` does (+0xa8 =
    /// `TickCount` @0x30b62-0x30b67), so a ship that unfolds to fire holds
    /// off its refold while its beam lasts ([`Armament::stamp_fire`]).
    fn stamp_beam_firers(&self, fighters: &mut [Fighter]) {
        for beam in &self.beams {
            if let Some(firer) = fighters
                .iter_mut()
                .find(|fighter| fighter.ship == beam.firer)
            {
                firer.armament.stamp_fire();
            }
        }
    }

    /// The shockwave of `wreck`'s final explosion, at its display step
    /// ([`shockwave`], `_HandleShipDisplay` @0x2d8ef-0x2da9c), the ships in
    /// `shown` displayed before it this pass. It reaches every ship not
    /// destroyed, breaking up or not, within the wreck's
    /// [`HullSpec::death_size`] on each axis, the wreck spared; each takes
    /// [`shockwave::hit`] as a [`Strike`] by the wreck, neither direct nor
    /// intended, and is pushed by [`shockwave::IMPACT`] from the wreck
    /// ([`impact::push`]), reported as a [`Shove`]; an escape pod or a
    /// later class ([`immune`]) is neither hit nor pushed. Each intact or
    /// disabled ship hit then settles its condition
    /// ([`Combat::settle_condition`]), its strike marked when it is newly
    /// disabled. A hit on a ship in `shown` flashes the next frame
    /// ([`CombatEvent::Hit`]'s `late`): the player, for an NPC's wreck, and
    /// every NPC in a lower slot.
    ///
    /// The ship loop interleaves each NPC's move with the displays
    /// (`nova_sim::Session::ship_loop`), so a ship of a later slot is
    /// reached where it still stands and moves by the push in the same
    /// frame; one of an earlier slot has moved, and moves by it the
    /// next.
    fn shock(
        &mut self,
        fighters: &mut [Fighter],
        wreck: ShipRef,
        shown: &[ShipRef],
        rule: &dyn DisableRule,
    ) {
        let Some((at, hull)) = fighters
            .iter()
            .find(|fighter| fighter.ship == wreck)
            .map(|fighter| (fighter.state.position, fighter.hull))
        else {
            return;
        };
        let blast = Blast {
            at,
            radius: hull.death_size(),
            direct: None,
            firer: Some(wreck),
            spares_player: true,
        };
        let reached = blast.reaches(
            fighters
                .iter()
                .filter(|fighter| *fighter.condition != Condition::Destroyed)
                .map(|fighter| (fighter.ship, fighter.state.position)),
        );
        let damage = shockwave::damage(&hull);
        for ship in reached {
            let Some(index) = fighters.iter().position(|fighter| fighter.ship == ship) else {
                continue;
            };
            let late = shown.contains(&ship);
            let fighter = &mut fighters[index];
            let condition = *fighter.condition;
            let before = taken(fighter.reserves);
            let hurt = !immune(fighter.ship_type);
            if hurt {
                shockwave::hit(fighter.reserves, condition, &fighter.hull, rule, damage);
            }
            if !hurt {
                continue;
            }
            // `_DamageShip` @0x2da89 passes no pass-through flag.
            self.events.push(CombatEvent::Hit {
                ship,
                weapon: None,
                at,
                passes_shields: false,
                late,
            });
            let cap = self.speed_cap;
            if let Some(velocity) = impact::push(&fighter.body(), at, shockwave::IMPACT, None, cap)
            {
                self.shove(fighter, velocity);
            }
            let downed = if condition.standing() {
                self.settle_condition(fighter, rule)
            } else {
                None
            };
            self.strikes.push(Strike {
                ship,
                by: Some(wreck),
                damage: before - taken(fighter.reserves),
                downed,
                direct: false,
                intended: false,
                lock: false,
                nominal: Nominal {
                    mass: damage,
                    energy: damage,
                },
                weight: 0.0,
                answered: false,
            });
        }
    }

    /// Has a breakup that never reaches the explosion band go as
    /// `breakup` says: by default, [`Breakup::Explodes`].
    pub fn set_breakup(&mut self, breakup: Breakup) {
        self.breakup = breakup;
    }

    /// Has the pass-through flags count the player as `pass_through`
    /// says (see [`sides`]): by default, [`PassThrough::PlayerSide`].
    pub fn set_pass_through(&mut self, pass_through: PassThrough) {
        self.pass_through = pass_through;
    }

    /// Has a blast reach ships by their planet type as `reading` says
    /// (see [`planet_type`]): by default, [`PlanetBlasts::Original`].
    pub fn set_planet_blasts(&mut self, reading: PlanetBlasts) {
        self.planet_blasts = reading;
    }

    /// Has a beam read its planet type from the bit `reading` says (see
    /// [`planet_type`]): by default, [`BeamPlanetFlag::Flags`].
    pub fn set_beam_planet_flag(&mut self, reading: BeamPlanetFlag) {
        self.beam_planet_flag = reading;
    }

    /// Has a planet-type beam hit a stellar as `reading` says (see
    /// [`beam_stellar`]): by default, the engine's, which never does.
    pub fn set_beam_stellars(&mut self, reading: RuleSource) {
        self.beam_stellars = reading;
    }

    /// The readings the hit rules take ([`Sides`]): who counts as the
    /// player for the pass-through flags, and which bit makes a beam
    /// planet-type.
    fn hit_readings(&self) -> (PassThrough, BeamPlanetFlag) {
        (self.pass_through, self.beam_planet_flag)
    }

    /// Has a decaying shot take a step as `step` says: by default,
    /// [`DecayStep::EveryDecayPlusOne`].
    pub fn set_decay(&mut self, step: DecayStep) {
        self.decay = step;
    }

    /// Has an NPC's pick spend on a copy that spawns nothing as `spend`
    /// says ([`Armament::fire_pick`]): by default, the engine's.
    pub fn set_ai_spend(&mut self, spend: RuleSource) {
        self.ai_spend = spend;
    }

    /// Has an NPC's quadrant turret measure its arc as `arc` says
    /// ([`ai_fire::spawn`]): by default, the engine's.
    pub fn set_quadrant_arc(&mut self, arc: RuleSource) {
        self.quadrant_arc = arc;
    }

    /// Has an exit point take its compression as `source` says
    /// ([`Compression`]): by default, the engine's.
    pub fn set_exit_compression(&mut self, source: RuleSource) {
        self.compression = Compression::from(source);
    }

    /// Has a point-defence shot move its cursor on as `source` says
    /// ([`RuleKey::PdExitStep`](crate::RuleKey::PdExitStep)): by default,
    /// the engine's, two exits a shot.
    pub fn set_pd_exit_step(&mut self, source: RuleSource) {
        self.pd_exit_step = source;
    }

    /// Has a beam of `ExitType` -1 step its ship's fold set as `source`
    /// says ([`RuleKey::StrayBeamExit`](crate::RuleKey::StrayBeamExit)): by
    /// default, the engine's, as `_SpawnBeam` does ([`exits::step_stray`]).
    pub fn set_stray_beam_exit(&mut self, source: RuleSource) {
        self.stray_beam_exit = source;
    }

    /// Has a beam find the ship it hits as `source` says
    /// ([`BeamHitTest`]): by default, the engine's cone.
    pub fn set_beam_hit_test(&mut self, source: RuleSource) {
        self.beam_rules.hit_test = BeamHitTest::from(source);
    }

    /// Has the beam cone measure a ship's bearing as `source` says
    /// ([`ConeAngle`]): by default, the other reading's wrap, the engine's
    /// plain difference restored by [`RuleSource::Engine`].
    pub fn set_beam_cone_quirks(&mut self, source: RuleSource) {
        self.beam_rules.angle = ConeAngle::from(source);
    }

    /// Has an NPC's bombs and rockets leave it as `source` says
    /// ([`BombLaunch::of`]): by default, the engine's, thrown at the
    /// ship's velocity and their `Speed`.
    pub fn set_bomb_rocket_launch(&mut self, source: RuleSource) {
        self.bomb_launch = source;
    }

    /// Has a bomb's heading wrapped after it turns into the wind as
    /// `source` says
    /// ([`RuleKey::BombHeadingWrap`](crate::RuleKey::BombHeadingWrap)): by
    /// default, the engine's, never.
    pub fn set_bomb_heading_wrap(&mut self, source: RuleSource) {
        self.bomb_heading_wrap = source;
    }

    /// Has point defence choose its missiles as `source` says
    /// ([`RuleKey::PdHostility`](crate::RuleKey::PdHostility)): by default,
    /// the engine's, with no hostility test; by the other reading, only
    /// those the fight's [`PointDefenceRule`] calls hostile.
    pub fn set_pd_hostility(&mut self, source: RuleSource) {
        self.pd_hostility = source;
    }

    /// Has a `Seeker` 0x4000 shot measure how far its target is off ahead
    /// as `source` says ([`LockAngle`]): by default, the other reading's
    /// short way round, the engine's plain angle restored by
    /// [`RuleSource::Engine`].
    pub fn set_seeker_lock_angle(&mut self, source: RuleSource) {
        self.lock_angle = LockAngle::from(source);
    }

    /// Has an armed proximity fuse set off by asteroids as `fuse` says:
    /// by default, [`AsteroidFuse::Original`].
    pub fn set_asteroid_fuse(&mut self, fuse: AsteroidFuse) {
        self.asteroid_fuse = fuse;
    }

    /// Has an armed proximity fuse set off by a ship as `source` says
    /// ([`ProximityFuse`], [`Shot::fused`]): by default, the engine's
    /// point test at the shot's end point, in slot order.
    pub fn set_proximity_fuse(&mut self, source: RuleSource) {
        self.proximity_fuse = ProximityFuse::from(source);
    }

    /// Holds an ionized ship's `Seeker` 0x0020 weapons as `source` says
    /// ([`IonLockout`]): by default, the engine's.
    pub fn set_ion_lockout(&mut self, source: RuleSource) {
        self.ion_lockout = IonLockout::from(source);
    }

    /// Has the player's quadrant turrets (7 and 8) test their blind spots
    /// as `source` says ([`aim::aim`]): by default, the engine's, never.
    pub fn set_quadrant_blind_spots(&mut self, source: RuleSource) {
        self.quadrant_blind = source;
    }

    /// Has a player's weapon of negative `Recoil` push it as `source`
    /// says ([`RuleKey::PlayerForwardRecoil`](crate::RuleKey::PlayerForwardRecoil),
    /// read in `Combat::fire_player`): by default, the engine's, not at all.
    pub fn set_player_forward_recoil(&mut self, source: RuleSource) {
        self.forward_recoil = source;
    }

    /// Has the original's shot and beam tables cap them as `source` says
    /// ([`MAX_SHOTS`], [`MAX_BEAMS`]): by the engine, capped; by the other
    /// reading, the default, not.
    pub fn set_combat_caps(&mut self, source: RuleSource) {
        self.capped = source == RuleSource::Engine;
    }

    /// Has a push, the death shockwave's and a tractor's self-pull clamp
    /// the speed of the ship it moves as `source` reads
    /// ([`RuleKey::SpeedCap`](crate::RuleKey::SpeedCap), [`SpeedCap`]): by
    /// the engine, the default, on each axis; by the other reading, to the
    /// circle of its top speed.
    pub fn set_speed_cap(&mut self, source: RuleSource) {
        self.speed_cap = SpeedCap::from(source);
    }

    /// Whether a shot fired now finds a free slot: always uncapped; under
    /// the caps, while the shots in flight and those whose slots are still
    /// held ([`ShotSlots::hold`]) leave one of [`MAX_SHOTS`] free
    /// ([`ShotSlots::first_free`]). The original
    /// frees a shot's slot only on the `_HandleShot` pass after it ends
    /// (life -1.0 or 0 then -2.0, @0x356c1, @0x30ab1, @0x3617c; free below
    /// -1.0, @0x3e5be), which comes after `_HandlePlayer` and `_AIDispatch`
    /// and before `_HandleShip` (`_DoPlayGameWork` @0x43e19, @0x4429d,
    /// @0x44385, @0x44483). So:
    ///
    /// - the player's point defence and trigger find the missiles the last
    ///   tick's point-defence beams downed still held;
    /// - an NPC's point defence finds those and every shot the collision
    ///   passes ended still held, and a spent shot still in the table;
    /// - an NPC's pick, after `_HandleShot` ([`Combat::fly_shots`]), finds
    ///   them all freed, the spent shots' too, and a shot of no life is
    ///   freed by the `_HandleShot` that first visits it, so it holds none.
    ///
    /// A shot point defence ended holds its slot once, here, while it is
    /// kept hidden for the proximity pass ([`hidden`]).
    fn shot_room(&self) -> bool {
        self.first_free_slot().is_some()
    }

    /// Whether a beam fired now finds a free slot: always uncapped; under
    /// the caps, while the beams held, the last tick's ended ones counted,
    /// are fewer than [`MAX_BEAMS`]. The original marks a beam ended
    /// (+0x10 = -1) and frees it (-2) on the next `_HandleBeams` pass
    /// (@0x31df6), the frame's last (@0x4453d), so it holds its slot
    /// through every ship's firing the next tick.
    fn beam_room(&self) -> bool {
        !self.capped || self.beams.len() + self.ended_beams < MAX_BEAMS
    }

    /// The next shot's number.
    fn next_id(&mut self) -> ShotId {
        let id = ShotId(self.next_shot);
        self.next_shot += 1;
        id
    }

    /// The damage aimed at each ship by the shots in flight
    /// (`_CalculateTargetedDamage` @0x3a153-0x3a232): for each shot not
    /// jammed ([`Jam::Clear`]), with life left, and aimed at a ship
    /// ([`Shot::aimed_at`]), its mass and energy
    /// damage times [`AIMED_DAMAGE_SHARE`], added to that ship's total
    /// and truncated after each add. A shot whose last move spent its
    /// life, waiting for its end of life in `_HandleShot`, counts for
    /// nothing, as the original skips a life of none or below
    /// (@0x3a1ad-0x3a1b5). Beams are not shots and count for
    /// nothing. The original keeps each total in 16 bits; here it is not
    /// wrapped.
    ///
    /// The original builds the totals once a frame, after the collision
    /// passes and before the AI runs (`_DoPlayGameWork` @0x44223, before
    /// `_AIDispatch` @0x4429d), so no shot the passes hit counts. The
    /// session's `nova_sim::Session::game_work` reads them there, after
    /// [`Combat::test_hits`].
    #[must_use]
    pub fn targeted_damage(&self) -> BTreeMap<ShipRef, i32> {
        let mut aimed = BTreeMap::new();
        let live = |shot: &&Shot| shot.jam == Jam::Clear && !shot.expired();
        for shot in self.shots.iter().filter(live) {
            let Some(ship) = shot.aimed_at else {
                continue;
            };
            let damage = f64::from(shot.weapon.mass_damage) + f64::from(shot.weapon.energy_damage);
            let total: &mut i32 = aimed.entry(ship).or_default();
            // A sum of `i16` halves stays far within an `i32`.
            #[allow(clippy::cast_possible_truncation)]
            {
                *total = damage
                    .mul_add(AIMED_DAMAGE_SHARE, f64::from(*total))
                    .trunc() as i32;
            }
        }
        aimed
    }

    /// Puts `launch` in flight from `side`, from `muzzle`, its damage aimed
    /// at `aimed_at` among `targets`, sounding as `sound` says, its miss
    /// roll drawn on `chance`: a fighter's
    /// [`Sortie`] from a bay, a beam held on point defence's `quarry`, on
    /// its target, or ahead, or a shot of its government. Its firing is heard
    /// at the ship's centre (`_PlaySoundDistance(&ship)`).
    ///
    /// A shot or beam whose weapon names a set of exit points its ship has
    /// leaves from one ([`exits`]), as `_SpawnShot` (@0x3eb96-0x3ec95) and
    /// `_SpawnBeam` (@0x450e4-0x4523d) pick it:
    ///
    /// - A weapon of `Flags3` 0x0010 takes the exit nearest the ship its
    ///   damage is aimed at, `_SpawnShot`'s and `_SpawnBeam`'s target.
    /// - A shot's heading is worked out once, from where it leaves, in
    ///   whole degrees ([`Launch::shot_heading`]): the ship's truncated
    ///   heading (@0x3e677), or for a turret that leads its target (4, 7
    ///   and 8) the lead angle from the exit (`_CalcLeadAngle` @0x3ecdc),
    ///   turned by its inaccuracy and wrapped once (@0x3ed44-0x3edc1). A
    ///   fighter's is from the centre, unwrapped ([`Launch::heading_from`]). A beam takes
    ///   neither its launch's heading nor its inaccuracy: it points anew
    ///   each tick ([`Beam::follow`]), as `_SpawnBeam` never reads the
    ///   angle it is given.
    /// - A point-defence shot steps its cursor twice by the engine, as
    ///   `_HandleShipPointDefense` sets its shot from `_SpawnShot`'s exit
    ///   back to the centre and takes the next (@0x396e5, @0x39707-0x3976c),
    ///   so a ship's point defence fires from every other exit; once by
    ///   the other reading. It takes the exit nearest its missile under
    ///   `Flags3` 0x0010, and is aimed from it (`_CalcInterceptAngle`
    ///   @0x39790). A point-defence beam at a missile never takes the
    ///   nearest (@0x398d2, kind 1).
    /// - A point-defence shot at a ship takes its exit the same way, the
    ///   ship standing in for the original's toward point, the shot slot
    ///   numbered like the ship (@0x39740-0x3975e), which the engine has no
    ///   slots for and no stock point defence reads (`Flags3` 0x0010). It
    ///   leads the ship from its exit (`_CalcLeadAngle` @0x397c6), has no
    ///   target and is aimed at none (`_SpawnShot(.., -1, ..)` @0x396e5),
    ///   and moves to the shot layer ([`ShotLayer::Shots`], @0x398a7). A
    ///   point-defence beam at a ship is held on it and aimed at it, and
    ///   under `Flags3` 0x0010 takes the exit nearest it (kind -1,
    ///   @0x398fb-0x3990a).
    /// - A shot's velocity is [`projectile::launch_velocity`]'s along its
    ///   truncated heading (`_Accel` @0x3eeea), a bomb's or rocket's as the fight's [`BombLaunch`] reading
    ///   gives for its ship: an NPC's, an escort's too, thrown at the ship's
    ///   velocity and its `Speed` by the engine, the player's dropped
    ///   (`_SpawnShot` @0x3e73a-0x3e783, @0x3ede8-0x3ee0a). A bomb's
    ///   velocity goes along its aim before its inaccuracy, which turns only
    ///   its facing (@0x3ef0a-0x3efed).
    /// - A negative `Inaccuracy` flies the shot out from its ship's heading
    ///   by as many degrees, to the side of its exit's x, facing as aimed
    ///   (@0x3ee10-0x3eec0); a shot from an exit on the centre line, or
    ///   from none, flies as aimed, and so does point defence. A dropped
    ///   bomb or rocket reads no heading, so keeps its launch velocity, as
    ///   the gate it fails holds the broadside too.
    /// - A shot is put in its sprite layer by its guidance
    ///   ([`ShotLayer::spawned`], @0x3f0a9-0x3f143), a turret (4) of a ship
    ///   of `Flags3` 0x0040 over the ships, and carries that bit for its
    ///   sub-munitions ([`Shot::turret_over`]).
    /// - A ship's turret (4) or point-defence (9) shot rolls its miss,
    ///   `below(class) + 1` in its ship's
    ///   [`HullSpec::miss_class`] (@0x3e89e-0x3e918), drawn on `chance`;
    ///   any other carries none (@0x3e95c). See [`sides`] for the misses.
    /// - A beam's exit is kept, and it starts there every tick
    ///   ([`Beam::follow`]).
    ///
    /// Under the caps a shot or beam finding no free slot
    /// ([`Combat::shot_room`], [`Combat::beam_room`]) fails quietly, as
    /// `_SpawnShot` (@0x3e5d0) and `_SpawnBeam` (@0x452e2) return before
    /// they place it or step its exit: its firing is still heard, and
    /// nothing flies. So does a shot of a weapon index or `Graphic` above
    /// 255, capped or not ([`WeaponSpec::spawns_shot`](weapon::WeaponSpec::spawns_shot), @0x3e56b-0x3e5a1). Whether a shot was put in flight, which an `AmmoType`
    /// of [`SELF_DESTRUCT`] reads; never for a beam or a fighter.
    #[allow(clippy::too_many_arguments)]
    fn launch(
        &mut self,
        (side, ties, plunderer): (Side, Ties, Plunderer),
        muzzle: &mut Muzzle,
        launch: &Launch,
        (quarry, aimed_at): (Option<Quarry>, Option<ShipRef>),
        sound: Option<FireSound>,
        targets: &[Target],
        chance: &mut dyn Chance,
    ) -> bool {
        let Side { ship, fleet, .. } = side;
        let from = muzzle.from;
        self.events.push(CombatEvent::Fired {
            ship,
            weapon: launch.weapon.id,
            at: from.position,
            sound,
        });
        let weapon = launch.weapon;
        let toward = aimed_at
            .filter(|_| weapon.nearest_exit)
            .and_then(|ship| position_of(targets, ship));
        if let Some(carried) = weapon.carried {
            self.sorties.push(Sortie {
                carrier: ship,
                bay: weapon.id,
                ship: carried,
                from,
                heading: launch.heading_from(&from),
                target: launch.target,
            });
            false
        } else if weapon.is_beam() {
            if !self.beam_room() {
                return false;
            }
            let aiming = match (quarry, launch.target) {
                (Some(Quarry::Missile(missile)), _) => Aiming::Shot(missile),
                (Some(Quarry::Ship(target)), _) => Aiming::Ship(target.ship),
                (None, Some(target)) => Aiming::Ship(target),
                (None, None) => Aiming::Ahead,
            };
            let mut beam = Beam {
                ties,
                aimed_at,
                ..Beam::launch(weapon, ship, fleet, &from, aiming)
            };
            beam.disable_only |= plunder::beam_disables(plunderer, aimed_at, targets);
            if let Some(word) = self.beam_exit(muzzle, &mut beam, toward, chance) {
                self.stray_steps.push((ship, word));
            }
            self.beams.push(beam);
            false
        } else {
            if !self.shot_room() || !weapon.spawns_shot() {
                return false;
            }
            let defended = quarry.and_then(|quarry| match quarry {
                Quarry::Missile(id) => self
                    .shots
                    .iter()
                    .find(|shot| shot.id == id)
                    .map(|shot| shot.position),
                Quarry::Ship(target) => Some(target.position),
            });
            let (start, side_of_exit) = self.shot_exit(muzzle, launch, defended, toward);
            let leaving = ShipState {
                position: start,
                ..from
            };
            let heading = launch.shot_heading(&leaving);
            let thrown = BombLaunch::of(ship != ShipRef::Player, self.bomb_launch);
            // A bomb flies along its whole-degree aim, its inaccuracy turning
            // only its facing, after its velocity is set (@0x3ef0a-0x3efed).
            let along = if weapon.guidance == Guidance::FreefallBomb {
                f32::from(launch.pointing.whole_from(&leaving))
            } else {
                heading
            };
            let rolls = matches!(weapon.guidance, Guidance::Turret | Guidance::PointDefence);
            let mut shot = Shot {
                id: self.next_id(),
                target: launch.target,
                aimed_at,
                source: Source::Ship(ties),
                miss_roll: rolls.then(|| chance.below(muzzle.miss_class) + 1),
                // Every ship's, the player's (slot 0) too (@0x3f0a9-0x3f143).
                layer: ShotLayer::spawned(weapon.guidance, true, muzzle.turret_over),
                turret_over: muzzle.turret_over,
                firer_slot: muzzle.firer_slot,
                ..Shot::launch(weapon, ship, fleet, &leaving, heading)
            };
            shot.disable_only |= plunder::shot_disables(plunderer, aimed_at, targets);
            shot.roll_at_launch(self.field.interference, chance);
            // A dropped bomb or rocket reads no heading, so it is never sent
            // broadside, as the gate it fails holds the broadside
            // (@0x3ede8-0x3ee0a).
            let out = if weapon.broadside > 0 && side_of_exit != 0.0 && quarry.is_none() {
                // Wrapped in float, then truncated (@0x3ee7e-0x3eeb8).
                normalized(from.heading + side_of_exit * weapon.broadside as f32).trunc()
            } else {
                along
            };
            shot.velocity = projectile::launch_velocity(&weapon, &from, out, thrown);
            let Some(taken) = self.put_in_flight(shot) else {
                return false;
            };
            // `_HandleShipPointDefense` moves a point-defence shot fired at
            // a ship to the shot layer (@0x398bd), after `_SpawnShot` put it
            // in its own: so at the shot layer's tail.
            if let Some(Quarry::Ship(_)) = quarry {
                self.move_to_layer(taken, ShotLayer::Shots);
            }
            true
        }
    }

    /// Takes `beam`'s exit from `muzzle`, as `_SpawnBeam` does
    /// (@0x450e4-0x4523d): an `ExitType` of 0-3 takes its cursor's exit, or
    /// the one nearest `toward`, and moves the cursor on. A point-defence
    /// beam at a missile is aimed at no ship, so it never takes the nearest
    /// exit (kind 1, @0x398d2). A hull with no exit points still steps the
    /// cursor (`_SpawnBeam` never reads the class), the beam starting at
    /// the centre. An `ExitType` of -1 indexes the word before the cursors,
    /// the fold set, and steps it as one by the engine (@0x450fa-0x4523d;
    /// [`exits::step_stray`]), the beam starting at the centre
    /// (`_HandleBeams` @0x30469-0x3049f). An `ExitType` of 5 lands on
    /// +0xc908, outside the fight (the crew pods left, or the player's pod
    /// countdown), and one of 6 on +0xc90a (the order word): by the engine
    /// each is returned for the caller to raise
    /// for the firer ([`Combat::take_stray_steps`]), the beam starting at
    /// the centre ([`StrayWord`]).
    fn beam_exit(
        &self,
        muzzle: &mut Muzzle,
        beam: &mut Beam,
        toward: Option<Vec2>,
        chance: &mut dyn Chance,
    ) -> Option<StrayWord> {
        let weapon = &beam.weapon;
        if let Some(kind) = weapon.exit {
            let from = muzzle.from;
            let exit = muzzle.cursors.beam_exit(
                muzzle.exits,
                kind,
                from.position,
                from.heading,
                toward,
                self.compression,
            );
            if muzzle.exits.is_some() {
                beam.kind = Some(kind);
                beam.exit = Some(exit);
            }
        } else if self.stray_beam_exit == RuleSource::Engine {
            match weapon.stray {
                Some(StrayWord::FoldSet) => exits::step_stray(muzzle.fold, chance),
                other => return other,
            }
        }
        None
    }

    /// Where a shot of `launch` leaves `muzzle` from, and the side of the
    /// exit it leaves from ([`ExitPoints::side`]): from the centre, on no
    /// side, without an exit; else from the exit its cursor or the nearest
    /// to `toward` gives, a point-defence shot at the missile or ship at
    /// `defended` taking two steps by the engine (see `Combat::launch`).
    fn shot_exit(
        &self,
        muzzle: &mut Muzzle,
        launch: &Launch,
        defended: Option<Vec2>,
        toward: Option<Vec2>,
    ) -> (Vec2, f32) {
        let from = muzzle.from;
        // A hull with no exit points still steps the cursor, from the
        // centre (`_ModifyShotStartPosition` never checks the points).
        let points = muzzle.exits;
        let Some(kind) = launch.weapon.exit else {
            return (from.position, 0.0);
        };
        let toward = if defended.is_some() {
            if self.pd_exit_step == RuleSource::Engine {
                // `_SpawnShot`'s own exit, toward no ship, then undone.
                muzzle.cursors.shot_start(
                    points,
                    kind,
                    from.position,
                    muzzle.angle,
                    None,
                    self.compression,
                );
            }
            defended.filter(|_| launch.weapon.nearest_exit)
        } else {
            toward
        };
        let (start, index) = muzzle.cursors.shot_start(
            points,
            kind,
            from.position,
            muzzle.angle,
            toward,
            self.compression,
        );
        let side = points.map_or(0.0, |points| points.side(kind, index));
        (start, side)
    }

    /// Step 1, after point defence ([`Combat::defend`]): each of the
    /// `firers` fires what its trigger holds ([`Combat::fire_one`]),
    /// aimed at its target among `targets` while that can be hit. The
    /// player fires in step 1, before the hit pass, as `_HandlePlayer`
    /// calls `_FirePlayerWeapon` after `_HandleShipPointDefense`
    /// ([`Combat::fire_player`]).
    fn fire(
        &mut self,
        fighters: &mut [Fighter],
        targets: &[Target],
        chance: &mut dyn Chance,
        firers: Firers,
    ) {
        for fighter in fighters
            .iter_mut()
            .filter(|fighter| firers.take(fighter.ship))
        {
            self.fire_one(fighter, targets, chance);
        }
    }

    /// `fighter` fires what its trigger holds, aimed at its target among
    /// `targets` while that can be hit: the player as step 1 fires it
    /// ([`Combat::fire_player`]), an NPC at its own step of `_HandleShip`
    /// (@0x34ffd, [`Combat::handle_npcs`]).
    /// An NPC fires a primary or secondary trigger through
    /// [`Armament::fire`], its quadrant turrets testing their blind spots
    /// as the original's AI picks them (`_AIFireTurret` @0x80d0b), each
    /// weapon sounding at [`AI_FIRE_PRIORITY`] (no AI sets one now: an
    /// escort's formation fire is `_AIFireTurret`'s pick, rdm
    /// escort-fleet-fidelity phase 1), then fires its pick
    /// ([`Trigger::only`]) as `_FireAIShipWeapon` does
    /// ([`Armament::fire_pick`], [`ai_fire`]), its first copy sounding at
    /// [`AI_FIRE_PRIORITY`], lighting its glow; a recoil sets its
    /// velocity, reported as a [`Shove`]. An escort's formation pick goes
    /// this way too, glow and recoil alike. A shot of `AmmoType` [`SELF_DESTRUCT`] that spawns
    /// destroys its firer ([`self_destruct`]).
    fn fire_one(&mut self, fighter: &mut Fighter, targets: &[Target], chance: &mut dyn Chance) {
        let target = ai_dispatch::standing(fighter.target, targets);
        if fighter.ship == ShipRef::Player {
            self.pull_player_trigger(fighter, target, targets, chance);
            return;
        }
        let (state, hull) = (fighter.state, fighter.hull);
        let trigger = Trigger {
            only: None,
            ..fighter.trigger
        };
        let launches = fighter.armament.fire(
            trigger,
            *fighter.condition,
            fighter.shooter(),
            fighter.rounds,
            &mut fighter.reserves.fuel,
            chance,
            &mut self.reports,
            &mut |spec| aim::aim(spec, &state, &hull, target, RuleSource::Bible),
        );
        // Each weapon launches at most once a call, its copies as one,
        // and sounds as `_FireAIShipWeapon` does, once a call.
        let sound = |_| Some(FireSound::at(AI_FIRE_PRIORITY));
        self.launch_volley(fighter, &launches, sound, targets, chance);
        let Some(pick) = fighter.trigger.only else {
            return;
        };
        let arc = self.quadrant_arc;
        let volley = fighter.armament.fire_pick(
            pick,
            *fighter.condition,
            fighter.shooter(),
            fighter.rounds,
            &mut fighter.reserves.fuel,
            chance,
            &mut self.reports,
            self.ai_spend,
            &mut |spec| ai_fire::spawn(spec, &state, &hull, target, arc),
            // Only a ship whose target is slot 0, the player, is stretched
            // (@0x88fb3).
            (fighter.target == Some(ShipRef::Player)).then_some(self.field.player_rating),
        );
        let Some(volley) = volley else {
            return;
        };
        let sound = first_copy_sounds_at(AI_FIRE_PRIORITY);
        self.launch_volley(fighter, &volley.launches, sound, targets, chance);
        self.glow(fighter.ship, &volley.weapon);
        let pushed = ai_fire::recoil(&state, &hull, fighter.class_speed, &volley.weapon);
        if let Some(velocity) = pushed {
            self.shove(fighter, velocity);
        }
    }

    /// The player's trigger, as `_HandlePlayer` fires it (@0x6ab39-0x6abc7):
    /// each weapon it picks ([`Armament::player_picks`]) fired in turn as
    /// `_FirePlayerWeapon` does ([`Armament::fire_player`]), each aimed at
    /// `target` among `targets` as [`aim::aim`] says, its quadrant turrets
    /// testing their blind spots as the fight's
    /// [`RuleKey::QuadrantTurretBlindSpots`](crate::RuleKey::QuadrantTurretBlindSpots)
    /// reading says. Of each call:
    ///
    /// - Its first copy sounds, at [`PLAYER_SECONDARY_FIRE_PRIORITY`] for a
    ///   beam (0 and 3), a fighter bay or a secondary, and at
    ///   [`PLAYER_FIRE_PRIORITY`] otherwise (@0x62d8d-0x62e9d); its later
    ///   copies carry no sound.
    /// - Each copy launches in turn, a copy finding no free slot failing
    ///   quietly ([`Combat::launch`]) though it was spent and counted; but
    ///   a fighter bay's copy is attempted only into a free ship berth
    ///   (see the `player_berths` module).
    /// - A shot of `AmmoType` [`SELF_DESTRUCT`] that spawns destroys the
    ///   player ([`self_destruct`]).
    /// - A glow weapon lights the player's weapon glow ([`Combat::glow`]),
    ///   but a fighter bay, whose glow waits on its fighter being put in
    ///   the system, which the session lights ([`Combat::light_glow`]).
    /// - A `Recoil` above none pushes the player back, opposite its heading,
    ///   by `Recoil` over its mass up to its class speed, once a call
    ///   (@0x62ea2-0x62f35, the signed `jle` @0x62eb5), reported as a
    ///   [`Shove`]; by the other reading of
    ///   [`RuleKey::PlayerForwardRecoil`](crate::RuleKey::PlayerForwardRecoil),
    ///   a negative one but -1 pushes it forward, as an NPC's does
    ///   ([`ai_fire::recoil`]).
    ///
    /// Each call reads the ship as the call before left it, as the
    /// original's does: a recoil moves the later weapons' shots on.
    fn pull_player_trigger(
        &mut self,
        fighter: &mut Fighter,
        target: Option<&Target>,
        targets: &[Target],
        chance: &mut dyn Chance,
    ) {
        let blind = self.quadrant_blind;
        let mut berths = self.player_berths;
        for weapon in fighter.armament.player_picks(fighter.trigger) {
            let (state, hull) = (fighter.state, fighter.hull);
            let volley = fighter.armament.fire_player(
                weapon,
                *fighter.condition,
                fighter.shooter(),
                fighter.rounds,
                &mut fighter.reserves.fuel,
                chance,
                &mut self.reports,
                &mut |spec| {
                    if berths.take(spec) {
                        aim::aim(spec, &state, &hull, target, blind)
                    } else {
                        None
                    }
                },
            );
            self.player_berths = berths;
            let Some(volley) = volley else {
                continue;
            };
            let sound = first_copy_sounds_at(player_fire_priority(&volley.weapon));
            self.launch_volley(fighter, &volley.launches, sound, targets, chance);
            // A bay's call glows only for a fighter put in the system,
            // which the session decides (`Combat::light_glow`).
            if volley.weapon.carried.is_none() {
                self.glow(fighter.ship, &volley.weapon);
            }
            let recoils = match self.forward_recoil {
                RuleSource::Engine => volley.weapon.recoil > 0.0,
                RuleSource::Bible => volley.weapon.recoils(),
            };
            let pushed = ai_fire::recoil(&state, &hull, fighter.class_speed, &volley.weapon);
            if let Some(velocity) = pushed.filter(|_| recoils) {
                self.shove(fighter, velocity);
            }
        }
    }

    /// Launches `launches`, one volley of `fighter`'s, in turn from its
    /// muzzle as it stands ([`Combat::launch`]), each aimed at its target
    /// ([`aimed_at`]), its copy `n` sounding as `sound(n)` says. A shot of
    /// `AmmoType` [`SELF_DESTRUCT`] among them that spawns then destroys
    /// `fighter`, once ([`self_destructs`]).
    fn launch_volley(
        &mut self,
        fighter: &mut Fighter,
        launches: &[Launch],
        sound: impl Fn(usize) -> Option<FireSound>,
        targets: &[Target],
        chance: &mut dyn Chance,
    ) {
        let (state, hull) = (fighter.state, fighter.hull);
        let sides = (fighter.side(), fighter.ties(), Plunderer::of(fighter));
        let drawn = fighter.drawn_heading;
        let firing_from = fighter.slot;
        let mut muzzle = Muzzle::new(state, &hull, drawn, fighter.armament.exits_mut());
        muzzle.firer_slot = Some(firing_from);
        let mut destructs = false;
        for (copy, launch) in launches.iter().enumerate() {
            let aimed_at = aimed_at(launch, fighter.target);
            let shot = self.launch(
                sides,
                &mut muzzle,
                launch,
                (None, aimed_at),
                sound(copy),
                targets,
                chance,
            );
            destructs |= self_destructs(launch, shot);
        }
        if destructs {
            self_destruct(fighter);
        }
    }

    /// Lights `ship`'s weapon glow when `weapon` glows, once a call that
    /// fired it ([`CombatEvent::WeaponGlow`]), shown the frame it fired.
    fn glow(&mut self, ship: ShipRef, weapon: &weapon::WeaponSpec) {
        if weapon.glows() {
            self.events
                .push(CombatEvent::WeaponGlow { ship, late: false });
        }
    }

    /// Sets `fighter`'s velocity to `velocity`, reporting it as a
    /// [`Shove`].
    fn shove(&mut self, fighter: &mut Fighter, velocity: Vec2) {
        fighter.state.velocity = velocity;
        self.shoves.push(Shove {
            ship: fighter.ship,
            velocity,
        });
    }

    /// Steps 1 and 4, before the firing ([`Combat::fire`]), as
    /// `_HandlePlayer` runs the player's (@0x68c74) before its trigger
    /// (@0x6ab55) and `_AIDispatch` every NPC's (@0x4429d, by slot among
    /// its launches, `Combat::dispatch_npcs`) before `_HandleShip` fires any
    /// pick (@0x44483): the point defence of each of the `firers` its
    /// trigger asks for
    /// ([`Trigger::point_defence`](armament::Trigger::point_defence))
    /// fires at the missile it picks ([`defence::choose`]), asking `rule`
    /// which are hostile, with the relations in `govts`, only by the
    /// other reading of
    /// [`RuleKey::PdHostility`](crate::RuleKey::PdHostility); with none, at
    /// the ship it picks among the fighters as they were when the step
    /// began ([`defence::choose_ship`]), asking no rule, its exit's toward
    /// point among `targets`. A ship whose jump is under way
    /// ([`Fighter::jumping`]) runs none (`_HandleShipPointDefense`
    /// @0x392d6-0x392de). Each shot or beam sounds at
    /// [`PD_FIRE_PRIORITY`], through the same tail for a missile or a ship
    /// (@0x39931-0x39bb3). Each reads its ship before the trigger's
    /// recoil pushes it, and a point-defence shot leaves with that
    /// velocity ([`projectile::launch_velocity`]).
    fn defend(
        &mut self,
        fighters: &mut [Fighter],
        targets: &[Target],
        (rule, govts): (&dyn PointDefenceRule, &Governments),
        chance: &mut dyn Chance,
        firers: Firers,
    ) {
        let candidates: Vec<Candidate> = fighters.iter().map(Fighter::as_candidate).collect();
        for fighter in fighters.iter_mut().filter(|fighter| {
            firers.take(fighter.ship) && fighter.trigger.point_defence && !fighter.jumping
        }) {
            self.defend_ship(fighter, &candidates, targets, (rule, govts), chance);
        }
    }

    /// `fighter`'s point defence, as [`Combat::defend`] runs it for each
    /// ship whose trigger asks: at the missile it picks, asking `rule`
    /// which are hostile, with the relations in `govts`, only by the other
    /// reading of [`RuleKey::PdHostility`](crate::RuleKey::PdHostility);
    /// with none, at the ship it picks among `candidates`, its exit's
    /// toward point among `targets`; sounding at [`PD_FIRE_PRIORITY`].
    fn defend_ship(
        &mut self,
        fighter: &mut Fighter,
        candidates: &[Candidate],
        targets: &[Target],
        (rule, govts): (&dyn PointDefenceRule, &Governments),
        chance: &mut dyn Chance,
    ) {
        let (state, hull) = (fighter.state, fighter.hull);
        let side = fighter.side();
        let ties = fighter.ties();
        let plunderer = Plunderer::of(fighter);
        let guard = fighter.guard();
        let shots = &self.shots;
        let rule = (self.pd_hostility == RuleSource::Bible).then_some(rule);
        let room = self.shot_room();
        let fired = fighter.armament.fire_point_defence(
            *fighter.condition,
            fighter.shooter(),
            fighter.rounds,
            &mut fighter.reserves.fuel,
            chance,
            &mut self.reports,
            room,
            &mut |spec| {
                if let Some(missile) =
                    defence::choose(side, &state, &hull, spec, shots, rule, govts)
                {
                    return Some((Pointing::At(missile.position), Quarry::Missile(missile.id)));
                }
                let ship = defence::choose_ship(guard, &state, &hull, spec, candidates)?;
                // A beam reads no launch heading (`Beam::launch`).
                let pointing = aim::led(spec, &ship.target).pointing;
                Some((pointing, Quarry::Ship(ship.target)))
            },
        );
        if let Some((launch, quarry)) = fired {
            // `_SpawnBeam`'s target, beam +0x18. A shot is aimed at none
            // (`_SpawnShot(.., -1, ..)` @0x396e5), as its every step
            // leaves a point-defence shot anyway (`Shot::step`).
            let aimed_at = match quarry {
                Quarry::Ship(target) if launch.weapon.is_beam() => Some(target.ship),
                _ => None,
            };
            let drawn = fighter.drawn_heading;
            let firing_from = fighter.slot;
            let mut muzzle = Muzzle::new(state, &hull, drawn, fighter.armament.exits_mut());
            muzzle.firer_slot = Some(firing_from);
            let shot = self.launch(
                (side, ties, plunderer),
                &mut muzzle,
                &launch,
                (Some(quarry), aimed_at),
                Some(FireSound::at(PD_FIRE_PRIORITY)),
                targets,
                chance,
            );
            if self_destructs(&launch, shot) {
                self_destruct(fighter);
            }
        }
    }

    /// Step 5's hold: each shot in flight of a weapon that fires one shot
    /// at a time (`Flags3` 0x0004) holds its firer's reload for that
    /// weapon at its whole `Reload` ([`Armament::hold`]), on every tick
    /// it moves, the move that spends it or brings it to its victim
    /// included, but not the tick it hits, as the hit pass has taken it
    /// out by then and `_HandleShot`'s end of life never reaches the hold
    /// (@0x35c09). A shot no ship fired holds nothing.
    ///
    /// The player counts its reload down before `_HandleShot` sets it
    /// ([`Combat::fire_player`], @0x6ae25) and an NPC after
    /// ([`Combat::handle_npcs`], @0x33896): so the player fires it again
    /// `Reload` + 1 ticks after the tick of its last move, and an NPC
    /// `Reload` − 1 ticks after it.
    fn hold_reloads(&mut self, fighters: &mut [Fighter]) {
        for shot in self.shots.iter().filter(|shot| shot.weapon.one_at_a_time()) {
            if !matches!(shot.source, Source::Ship(_)) {
                continue;
            }
            if let Some(firer) = fighters
                .iter_mut()
                .find(|fighter| fighter.ship == shot.firer)
            {
                firer.armament.hold(shot.weapon.id);
            }
        }
    }

    /// Steps 2 and 3, the hit pass: each shot's last move, made the tick
    /// before (its [`Shot::moved_from`]), is tested as the original's next
    /// frame tests it, before `_HandleShot` moves it again
    /// (`_DoPlayGameWork` @0x43e70-0x44015, @0x44385). A shot that has not
    /// moved yet is not tested, as its sprite is not shown. The tests go
    /// in the original's order ([`collision`]):
    ///
    /// 1. the contact passes, each shot layer against the ships, walked by
    ///    layer and entry, then the asteroids, walked by slot, against the
    ///    layer; then the asteroids against point defence's layer;
    /// 2. point defence's layer against the missiles
    ///    ([`Combat::intercept`]): a shot spent in 1 is hidden by then and
    ///    never met, and each it ends is kept hidden at life 0 ([`hidden`]);
    /// 3. the proximity pass ([`Combat::proximity_pass`]), slot by slot,
    ///    the shots point defence ended, this pass's and the last tick's
    ///    beam kills, among the live ones: each slot's stellar, then, live,
    ///    its fuse's ship, then its fuse's asteroid.
    ///
    /// All three run in [`Combat::resolve`]. A shot whose move spent it
    /// meets only the asteroids it touches and point defence, and is kept
    /// for its end of life in `_HandleShot` ([`Combat::handle_shots`]).
    /// Point defence's [`CombatEvent::ShotDown`] follows the contact
    /// passes' events.
    ///
    /// The pass meets each NPC where it is, as its last `_HandleShip` left
    /// it (@0x3428f-0x342aa), and the player where it is now, as
    /// `_HandlePlayer` has moved it. Contact and the point-defence layer
    /// meet the player where it stood as the last tick closed, or where it
    /// is if it was not in that tick's fight: their sprites are placed
    /// relative to the player as it stood when the shots moved
    /// (`_HandleShot` @0x35c48-0x35cc0), and the player's own is always
    /// at the screen's centre (@0x2cd32-0x2cd9d). The fuse, the blasts
    /// and the hits' pushes and ionization read world positions, the
    /// player's where it is now. Each sub-munition's weapon is read from
    /// `arsenal`, its spread drawn on `chance`; the relations between
    /// governments are `govts`'.
    ///
    /// Each hit lands as the walk finds it ([`Combat::land`]), as
    /// `_HandleShipHit` lands it before the walk's next pair, so a later
    /// test reads the armour it left ([`Target::disabled`]). Each ship
    /// struck then settles its condition as step 9 does but for a
    /// breakup's countdown, its last strike marked as downing it, as
    /// `rule` says: so an NPC downed by a shot fires nothing more that
    /// tick, as `_HandleShip`'s fire reads `_IsDisabled` after the passes.
    /// It keeps the ships whose breakup it began, whose first tick takes no
    /// tick off (step 9, see `Combat::settle_struck`). The original
    /// settles each in `_DamageShip`; nothing in the pass reads the
    /// condition but whether a ship is destroyed, which no hit there
    /// changes, so settling after it crosses no boundary.
    fn hit_pass(
        &mut self,
        fighters: &mut [Fighter],
        (govts, arsenal): (&Governments, &Arsenal),
        rule: &dyn DisableRule,
        scenery: Scenery<'_>,
        chance: &mut dyn Chance,
    ) {
        let Scenery {
            stellars,
            asteroids,
            hazards: _,
        } = scenery;
        let mut none = Asteroids::default();
        let mut field = asteroids.unwrap_or(AsteroidField {
            asteroids: &mut none,
            types: &asteroid_walk::NO_TYPES,
        });
        let player_stood = self.player_stood;
        let touchable_at: Vec<Vec2> = fighters
            .iter()
            .map(|fighter| match player_stood {
                Some(position) if fighter.ship == ShipRef::Player => position,
                _ => fighter.state.position,
            })
            .collect();
        let mut lineup = Lineup::new(fighters, &touchable_at, govts, self.hit_readings());
        let first_strike = self.strikes.len();
        let entry = self.entry_order();
        let terms = collision::WalkTerms { arsenal, rule };
        self.resolve(
            (fighters, &mut lineup),
            &entry,
            &terms,
            chance,
            (stellars, &mut field),
        );
        self.settle_struck(fighters, first_strike, rule);
    }

    /// Each ship among `fighters` struck since `first_strike` and still
    /// standing settles its condition as step 9 does, as `rule` says
    /// ([`Combat::settle_condition`]), its last strike marked as downing
    /// it: a breakup it begins is left unset, for the ship's next display
    /// step to start (see the `display` module).
    fn settle_struck(
        &mut self,
        fighters: &mut [Fighter],
        first_strike: usize,
        rule: &dyn DisableRule,
    ) {
        for fighter in fighters.iter_mut() {
            let struck = self.strikes[first_strike..]
                .iter()
                .any(|strike| strike.ship == fighter.ship);
            if !struck || !fighter.condition.standing() {
                continue;
            }
            truncate_downed_player(fighter, rule);
            let downed = self.settle_condition(fighter, rule);
            self.mark_downed(first_strike, fighter.ship, downed);
        }
    }

    /// Step 7: each point-defence beam hits the missile it is held on, its
    /// damage multiplied by `factor` ([`GamePass`]); a missile destroyed is
    /// shot down and leaves the shots in flight, counted among the shots
    /// whose slots the original frees only at its next `_HandleShot`
    /// (@0x30ab1, see [`Combat::shot_room`]); it is kept hidden at life 0
    /// for the next tick's proximity pass ([`hidden`]). The beams held on a missile no longer
    /// in flight go. This comes after the shots fly, as `_HandleBeams`
    /// runs last in the frame (@0x4453d), so a beam downs a missile before
    /// the next hit pass tests its move.
    fn hold_on_missiles(&mut self, factor: f32) {
        let mut gone = vec![false; self.shots.len()];
        for beam in &self.beams {
            let Aiming::Shot(id) = beam.aiming else {
                continue;
            };
            let Some(missile) = self.shots.iter().position(|shot| shot.id == id) else {
                continue;
            };
            let damage = defence::pd_damage(&beam.weapon) * factor;
            if !gone[missile] && self.shots[missile].take_pd_hit(damage) {
                gone[missile] = true;
                self.events.push(CombatEvent::ShotDown {
                    at: self.shots[missile].position,
                });
            }
        }
        let shots = std::mem::take(&mut self.shots);
        for (shot, gone) in shots.into_iter().zip(gone) {
            if gone {
                self.slots.hold(shot.slot);
                self.hide(shot);
            } else {
                self.shots.push(shot);
            }
        }
        let shots = &self.shots;
        self.beams.retain(|beam| match beam.aiming {
            Aiming::Shot(id) => shots.iter().any(|shot| shot.id == id),
            _ => true,
        });
    }

    /// `shot`'s explosion where `blast` goes off, if its weapon has one,
    /// of its `BlastRadius` (`_CreateExplosion(.., ExplodType,
    /// BlastRadius, 1)`, `_HandleShipHit` @0x36aa9, `_HandleShot`
    /// @0x35f97).
    fn explode_shot(&mut self, shot: &Shot, blast: &Blast) {
        if let Some(explosion) = shot.weapon.explosion {
            self.events.push(CombatEvent::Exploded {
                at: blast.at,
                explosion,
                size: shot.weapon.blast_radius,
            });
        }
    }

    /// `weapon`'s hit particles at `at`, if it has any, their speed varied
    /// by `variation` percent ([`asteroid_hit::hit_burst`]).
    fn spray(&mut self, weapon: &weapon::WeaponSpec, at: Vec2, variation: u32) {
        if let Some(particles) = weapon.hit_particles {
            self.particles(asteroid_hit::hit_burst(at, &particles, variation));
        }
    }

    /// Step 8: the hit beam `index` makes this tick among `sides`, if
    /// any, the beam ending where it hit and exploding there; an intended
    /// hit is on the ship it was fired at, or, for one fired at nothing,
    /// its firer's target among `fighters`. Its mass and energy damage are
    /// multiplied by `factor` ([`GamePass`]), cone hits and segment hits
    /// alike; its `Impact` and `Ionization` never are. The caller lands it
    /// before the next beam is tested, as `_HandleBeams` damages the ship
    /// in its beam loop (@0x316a1). By the Bible's reading of
    /// [`RuleKey::BeamStellars`](crate::RuleKey::BeamStellars), a
    /// planet-type beam meeting one of `stellars` nearer than the ship it
    /// hits strikes the stellar instead, and hits no ship ([`beam_stellar`]);
    /// by the engine's it never does.
    fn beam_hit(
        &mut self,
        index: usize,
        (fighters, stellars): (&[Fighter], &mut [StellarTarget]),
        sides: &Sides,
        factor: f32,
    ) -> Option<Landed> {
        let beam_rules = self.beam_rules;
        let hit = self.beams[index].hit(sides, beam_rules);
        if self.beam_stellars == RuleSource::Bible
            && self.beam_strikes_stellar(index, hit.map(|(_, at)| at), stellars, factor)
        {
            return None;
        }
        let (ship, at) = hit?;
        let beam = &mut self.beams[index];
        beam.end = at;
        let beam = *beam;
        let meant = beam.aimed_at.or_else(|| {
            fighters
                .iter()
                .find(|fighter| fighter.ship == beam.firer)
                .and_then(|firer| firer.target)
        });
        let hit = Hit {
            cant_kill: beam.disable_only,
            ..Hit::of(&beam.weapon)
        };
        if let Some(explosion) = beam.weapon.explosion {
            self.events.push(CombatEvent::Exploded {
                at,
                explosion,
                size: beam.weapon.blast_radius,
            });
        }
        if let Some(particles) = beam.weapon.hit_particles {
            let variation = asteroid_hit::BEAM_HIT_PART_VARIATION;
            let burst = asteroid_hit::hit_burst(at, &particles, variation);
            self.events.push(CombatEvent::Particles(burst));
        }
        Some(Landed {
            ship,
            by: Some(beam.firer),
            weapon: Some(beam.weapon.id),
            at,
            hit: Hit {
                mass: hit.mass * factor,
                energy: hit.energy * factor,
                ..hit
            },
            direct: true,
            intended: meant == Some(ship),
            lock: false,
            weight: BEAM_WEIGHT,
            push: Push {
                from: beam.start,
                impact: beam.weapon.impact,
                beam: true,
            },
            ion: Some(Dose::of(&beam.weapon, None)),
        })
    }

    /// Takes each of `hits` in turn off the ship it landed on among
    /// `fighters`, as `rule` says when a ship is disabled, keeping each as
    /// a [`Strike`] (see the module docs' steps 3 and 8). Each pushes its ship by
    /// its `Impact` first, as `_DamageShip` does (@0x3a9b8-0x3ab84,
    /// [`impact::push`]), the push reported as a [`Shove`]; a beam's of
    /// negative `Impact` is a tractor's, which holds the ship or pulls its
    /// firer before that ([`Combat::tractor`]). A direct hit's negative
    /// `Impact` is gated from its firer's centre, which a firer no longer
    /// in the fight has none of.
    ///
    /// The original writes each push to the ship's velocity at once, and
    /// the ship moves by it the next frame; here the session sets the
    /// velocities from the shoves after the fight's tick
    /// ([`Combat::take_shoves`]), before the next movement, which crosses
    /// no boundary the player sees. The hits compound in the order they
    /// land: the hit pass's in its walk's order ([`collision`]), then
    /// `_HandleShot`'s, the blasts and locks slot by slot
    /// ([`Combat::handle_shots`]), then beams.
    ///
    /// Each hit but a lock ionizes its ship ([`Ion::take`], `_IonizeShip`)
    /// at the ship's place, the escape pod included, whatever the tractor
    /// branch passes on. The original ionizes before `_DamageShip` on a
    /// direct hit and a beam tick and after it on a blast; nothing reads
    /// the charge between the two, nor before the handling's next step,
    /// so one place serves all, crossing no boundary the player sees.
    ///
    /// A ship of the player's fleet that a hit leaves disabled leaves its
    /// leader as the hit lands (`_DamageShip` @0x3afdd,
    /// [`escort_loss::leave_fleet_if_disabled`]), so every later test of
    /// the tick reads it unled; the escape pod, which takes no hit, never
    /// does.
    fn land(&mut self, fighters: &mut [Fighter], hits: Vec<Landed>, rule: &dyn DisableRule) {
        // A shot passes through the escape pod and every class after it
        // (`_ShotCanHitShip` @0x447b3-0x447d9), and a beam through the pod
        // itself (@0x30dd2); a blast still reaches it, or a beam a later
        // class, and `_DamageShip` then does nothing to it ([`immune`]).
        for landed in hits {
            let Some(index) = fighters
                .iter()
                .position(|fighter| fighter.ship == landed.ship)
            else {
                continue;
            };
            let Push { from, impact, beam } = landed.push;
            let impact = if beam && impact < 0.0 {
                // A beam always has a firer; with none it would pull nothing.
                landed
                    .by
                    .map_or(0.0, |by| self.tractor(fighters, index, by, from, impact))
            } else {
                impact
            };
            let source = landed
                .by
                .filter(|_| landed.direct)
                .and_then(|by| fighters.iter().find(|fighter| fighter.ship == by))
                .map(|firer| firer.state.position);
            // A plunderer's real hit on its target (`_DamageShip`
            // @0x3a895-0x3a9a3).
            let spared = landed.direct && plunder::cant_kill(landed.by, landed.ship, fighters);
            let hit = Hit {
                cant_kill: landed.hit.cant_kill || spared,
                ..landed.hit
            };
            let fighter = &mut fighters[index];
            if let Some(dose) = landed.ion {
                fighter.ion.take(dose, fighter.state.position);
            }
            if immune(fighter.ship_type) {
                continue;
            }
            self.events.push(CombatEvent::Hit {
                ship: landed.ship,
                weapon: landed.weapon,
                at: landed.at,
                passes_shields: landed.hit.passes_shields,
                late: landed.ship == ShipRef::Player || beam,
            });
            if let Some(velocity) = impact::push(
                &fighter.knocked_body(beam),
                from,
                impact,
                source,
                self.speed_cap,
            ) {
                self.shove(fighter, velocity);
            }
            let before = taken(fighter.reserves);
            let was_disabled = fighter.disabled_now(rule);
            let was_dying = fighter.condition.dying_or_destroyed(fighter.reserves.armor);
            damage::take(fighter.reserves, hit, &fighter.hull, rule, was_disabled);
            // `_HandleShipHit`'s victory voice (@0x36c7f-0x36d45): a
            // shot's direct hit that starts its ship dying. A beam's hit is
            // direct too, but `_HandleBeams` speaks none; a lock takes no
            // armour, so it never starts one dying.
            if landed.direct
                && !beam
                && !was_dying
                && fighter.condition.dying_or_destroyed(fighter.reserves.armor)
            {
                self.finished.push(FinishingHit {
                    ship: landed.ship,
                    strike: self.strikes.len(),
                });
            }
            if escort_loss::leave_fleet_if_disabled(fighter, rule) {
                self.escort_losses.push(EscortLoss {
                    ship: landed.ship,
                    strike: self.strikes.len(),
                });
            }
            self.strikes.push(Strike {
                ship: landed.ship,
                by: landed.by,
                damage: before - taken(fighter.reserves),
                downed: None,
                direct: landed.direct,
                intended: landed.intended,
                lock: landed.lock,
                nominal: Nominal {
                    mass: landed.hit.mass,
                    energy: landed.hit.energy,
                },
                weight: landed.weight,
                answered: false,
            });
        }
    }

    /// The tractor branch of a hit on `fighters[target]` by the beam of
    /// `by` from `start`, of negative `impact` ([`impact::tractor`],
    /// `_HandleBeams` @0x311d0-0x31450): the firer holds the target,
    /// touching its lock, or holds itself and pulls itself toward it,
    /// reported as a [`Shove`]. Gives the `Impact` the hit passes on: its
    /// own when the firer holds the target, none otherwise.
    ///
    /// A firer no longer in the fight leaves no slot to read, so its beam
    /// does nothing and passes none on. The original reads whatever is in
    /// the slot (beam +0x16) then; stock's one tractor beam (`wëap` 171,
    /// `Count` 1, no `Decay`) hits only on the tick its firer fires it.
    fn tractor(
        &mut self,
        fighters: &mut [Fighter],
        target: usize,
        by: ShipRef,
        start: Vec2,
        impact: f32,
    ) -> f32 {
        let Some(firer) = fighters.iter().position(|fighter| fighter.ship == by) else {
            return 0.0;
        };
        let body = fighters[firer].body();
        match impact::tractor(
            &fighters[target].body(),
            &body,
            start,
            impact,
            self.speed_cap,
        ) {
            // The original's i = 0 (@0x31450); the push would refuse a
            // massless or planet-type target anyway.
            Tractor::Slips => 0.0,
            Tractor::Holds => {
                *fighters[target].tractor = Some(TractorLock::by(by));
                impact
            }
            Tractor::Pulls(pulled) => {
                let firer = &mut fighters[firer];
                TractorLock::touch_self(firer.tractor, by);
                if let Some(velocity) = pulled {
                    self.shove(firer, velocity);
                }
                0.0
            }
        }
    }

    /// Puts in flight the sub-munitions `parent` releases at `target`
    /// among `targets`, their weapon read from `arsenal` (none when it
    /// cannot be read), heard from where the parent is. Released, they
    /// report their weapon's unimplemented flags as a weapon fired does;
    /// of a guidance not flown yet, they report it and none fly. Under the
    /// caps the release ends at the first that finds no free slot, the
    /// shots ended this tick, the parent among them, still holding theirs
    /// (`_SpawnShotSubmunitions` @0x3f32d-0x3f331): those already placed
    /// fly, and the release is heard all the same, its sound coming before
    /// its loop. A sub-munition whose weapon places no shot
    /// ([`WeaponSpec::spawns_shot`](weapon::WeaponSpec::spawns_shot)) ends it at the first, so none fly.
    fn release(
        &mut self,
        parent: &Shot,
        (target, fighters): (Option<ShipRef>, &[Fighter]),
        sides: &Sides,
        arsenal: &Arsenal,
        chance: &mut dyn Chance,
    ) {
        let Some(sub) = parent
            .weapon
            .submunitions
            .and_then(|subs| arsenal.weapon(subs.weapon))
        else {
            return;
        };
        // `_SpawnShotSubmunitions` spawns as the parent's source slot
        // (+0x2c): its ship's class and plunder as they are now.
        let (turret_over, owner) = self.source_slot_reads(parent, fighters, arsenal);
        let shots = submunition::release(parent, sub, (target, turret_over));
        if shots.is_empty() {
            return;
        }
        let unflown = match sub.guidance {
            Guidance::Other(guidance) => Some(guidance),
            Guidance::FighterBay => Some(FIGHTER_BAY),
            _ => None,
        };
        if let Some(guidance) = unflown {
            self.reports.report(SimDiagnostic::UnimplementedGuidance {
                weapon: sub.id,
                guidance,
            });
            return;
        }
        self.reports.fired(sub);
        self.events.push(CombatEvent::Fired {
            ship: parent.firer,
            weapon: sub.id,
            at: parent.position,
            sound: Some(FireSound::at(SUBMUNITION_PRIORITY)),
        });
        for (mut shot, n) in shots.into_iter().zip(0..) {
            // `_SpawnShot` failing ends the loop (@0x3f32d-0x3f331).
            if !sub.spawns_shot() {
                break;
            }
            if self.seat(&mut shot).is_none() {
                break;
            }
            shot.id = self.next_id();
            shot.disable_only |= plunder::shot_disables(owner, target, sides.targets);
            shot.roll_at_launch(self.field.interference, chance);
            // Each seeks once seated, with its own roll and jam (@0x3f3cf).
            let shot = submunition::aim(parent, shot, n, sides, chance);
            self.insert_seated(&shot);
        }
    }

    /// Step 9, and every settling of a ship's condition (the hit pass's,
    /// the shots', a shockwave's, the display step's): `fighter`'s
    /// condition follows its armour, as `rule` says. How it was downed,
    /// if it was.
    ///
    /// - Intact or disabled with no armour left, it starts breaking up
    ///   ([`Condition::breaking_up`]): [`CombatEvent::BreakingUp`], and
    ///   `Dying` with its timer unset, which its next display step sets
    ///   (see the `display` module).
    /// - Held disabled ([`Fighter::held_disabled`]), it is kept disabled.
    /// - Otherwise it is disabled, or intact, as `rule` says, a ship that
    ///   holds no armour never disabled; newly disabled, it raises
    ///   [`CombatEvent::Disabled`].
    /// - Breaking up or destroyed, it is left as it is: the countdown is
    ///   the ship's own step's and its display's.
    ///
    /// Class 0x2ff, the escape pod, skips the whole dying block
    /// (`_HandleShipDisplay` @0x2d803, [`never_breaks_up`]): with its
    /// armour gone its condition stays as it is, with no breakup.
    fn settle_condition(
        &mut self,
        fighter: &mut Fighter,
        rule: &dyn DisableRule,
    ) -> Option<Downed> {
        if never_breaks_up(fighter.ship_type) && armour_gone(fighter.reserves.armor) {
            return None;
        }
        let ship = fighter.ship;
        if let Some(dying) = fighter.condition.breaking_up(fighter.reserves.armor) {
            self.events.push(CombatEvent::BreakingUp {
                ship,
                at: fighter.state.position,
                explosion: fighter.hull.breakup,
            });
            *fighter.condition = dying;
            return Some(Downed::BreakingUp);
        }
        match *fighter.condition {
            Condition::Intact | Condition::Disabled if fighter.held_disabled => {
                *fighter.condition = Condition::Disabled;
                None
            }
            Condition::Intact | Condition::Disabled => {
                let armor = fighter.reserves.armor;
                let disabled = holds_armour(armor) && rule.disabled(armor, &fighter.hull);
                let downed = (disabled && *fighter.condition == Condition::Intact).then(|| {
                    self.events.push(CombatEvent::Disabled { ship });
                    Downed::Disabled
                });
                *fighter.condition = if disabled {
                    Condition::Disabled
                } else {
                    Condition::Intact
                };
                downed
            }
            Condition::Dying { .. } | Condition::Destroyed => None,
        }
    }

    /// Destroys `fighter`, raising its [`CombatEvent::Destroyed`].
    fn destroy(&mut self, fighter: &mut Fighter) {
        *fighter.condition = Condition::Destroyed;
        self.events.push(CombatEvent::Destroyed {
            ship: fighter.ship,
            ship_type: fighter.ship_type,
            at: fighter.state.position,
            velocity: fighter.state.velocity,
            explosion: fighter.hull.explosion,
            size: fighter.hull.death_size(),
        });
    }

    /// The shots in flight, in slot order ([`Shot::slot`]).
    #[must_use]
    pub fn shots(&self) -> &[Shot] {
        &self.shots
    }

    /// The shots in flight, to change as nothing in play does at will:
    /// for a session test that needs a shot in a state only a rare draw
    /// reaches.
    #[cfg(any(test, feature = "fixture"))]
    pub fn shots_mut(&mut self) -> &mut [Shot] {
        &mut self.shots
    }

    /// The beams being fired.
    #[must_use]
    pub fn beams(&self) -> &[Beam] {
        &self.beams
    }

    /// What has happened since this was last taken, in order; taking it
    /// empties the list.
    pub fn take_events(&mut self) -> Vec<CombatEvent> {
        std::mem::take(&mut self.events)
    }

    /// The strikes made since they were last taken, in order; taking them
    /// empties the list, and drops the escort losses and finishing hits
    /// noted with them ([`Combat::take_landed`]).
    pub fn take_strikes(&mut self) -> Vec<Strike> {
        self.take_landed().0
    }

    /// The strikes made since they were last taken, in order; the ships
    /// of the player's fleet they lost, each with the index of the strike
    /// that lost it, in order ([`EscortLoss`]); and the hits that started
    /// their ships dying, each with its strike's index, in order
    /// ([`FinishingHit`]), which `_HandleShipHit`'s victory voice reads.
    /// Taking them empties the three lists.
    pub fn take_landed(&mut self) -> (Vec<Strike>, Vec<EscortLoss>, Vec<FinishingHit>) {
        (
            std::mem::take(&mut self.strikes),
            std::mem::take(&mut self.escort_losses),
            std::mem::take(&mut self.finished),
        )
    }

    /// The words outside the fight that beams of stray `ExitType` stepped
    /// since they were last taken, each with its firer, in order, for the
    /// session to step ([`StrayWord`]); taking them empties the list.
    pub fn take_stray_steps(&mut self) -> Vec<(ShipRef, StrayWord)> {
        std::mem::take(&mut self.stray_steps)
    }

    /// The hits on stellars made since they were last taken, in order;
    /// taking them empties the list.
    pub fn take_stellar_strikes(&mut self) -> Vec<StellarStrike> {
        std::mem::take(&mut self.stellar_strikes)
    }

    /// Reports an explosion at `at` that is no weapon's: a stellar's as it
    /// is destroyed, or a sabotage bomb's aboard the player's ship
    /// (`_HandlePlayer` @0x698ef). It is of size 0, as
    /// `_CreateExplosion(.., 0, 1)` makes it, so no extra explosions
    /// scatter from it.
    pub fn explode(&mut self, at: Vec2, explosion: Explosion) {
        self.events.push(CombatEvent::Exploded {
            at,
            explosion,
            size: 0.0,
        });
    }

    /// Lights `ship`'s weapon glow, shown the frame it is lit
    /// ([`CombatEvent::WeaponGlow`]): the player's fighter bay's, once the
    /// session has put a fighter it launched in the system, as
    /// `_FirePlayerWeapon` reaches the glow from a bay only when
    /// `_LaunchFighter` returns one (@0x62c61-0x62c6e, @0x62c78-0x62c98).
    pub fn light_glow(&mut self, ship: ShipRef) {
        self.events
            .push(CombatEvent::WeaponGlow { ship, late: false });
    }

    /// Reports an escape pod `ship` ejected at `at`, moving at `velocity`
    /// ([`CombatEvent::PodEjected`]).
    pub fn eject_pod(&mut self, ship: ShipRef, at: Vec2, velocity: Vec2) {
        self.events
            .push(CombatEvent::PodEjected { ship, at, velocity });
    }

    /// Reports `burst`'s particles: a shot's hit on an asteroid's, or an
    /// asteroid's as it is destroyed.
    pub fn particles(&mut self, burst: ParticleBurst) {
        self.events.push(CombatEvent::Particles(burst));
    }

    /// The fighters launched since they were last taken, in order; taking
    /// them empties the list.
    pub fn take_sorties(&mut self) -> Vec<Sortie> {
        std::mem::take(&mut self.sorties)
    }

    /// The ships the fight pushed since they were last taken, in order,
    /// each with its velocity after the push; taking them empties the
    /// list.
    pub fn take_shoves(&mut self) -> Vec<Shove> {
        std::mem::take(&mut self.shoves)
    }

    /// The diagnostics made since they were last taken, each once a
    /// fight; taking them empties the list.
    pub fn take_diagnostics(&mut self) -> Vec<SimDiagnostic> {
        self.reports.take()
    }

    /// Clears the shots and beams, as the ship leaves the system.
    pub fn clear(&mut self) {
        self.shots.clear();
        self.hidden.clear();
        self.beams.clear();
        self.slots.clear();
        self.ended_beams = 0;
        self.player_stood = None;
    }
}

impl Fighter<'_> {
    /// The ship as `_WeaponHasAmmo` asks of it: an NPC or the player,
    /// cloaked or not, and its class's `KeyCarried`.
    fn shooter(&self) -> Shooter {
        Shooter {
            npc: matches!(self.ship, ShipRef::Npc(_)),
            cloaked: self.cloaked,
            key_carried: self.hull.key_carried,
            folds: self.hull.frames.unfolds_to_fire(),
        }
    }

    /// The ship's side: itself, its fleet and its government.
    fn side(&self) -> Side {
        Side {
            ship: self.ship,
            fleet: self.fleet,
            govt: self.govt,
        }
    }

    /// The ship's ties: its leader, government and `düde`.
    fn ties(&self) -> Ties {
        Ties {
            leader: self.leader,
            govt: self.govt,
            booty: self.booty,
            planet_type: self.hull.planet_type,
            mining: self.mining,
        }
    }

    /// The ship as a push reads it ([`impact`]).
    fn body(&self) -> Body {
        Body {
            position: self.state.position,
            velocity: self.state.velocity,
            mass: self.hull.mass,
            planet_type: self.hull.planet_type,
            class_speed: self.class_speed,
            top_speed: self.top_speed,
            jumping: self.jumping,
            afterburner: self.afterburner,
        }
    }

    /// The ship as a hit's push reads it: its afterburner's widening is
    /// [`Fighter::afterburner_beam`] for a `beam` tick, else
    /// [`Fighter::afterburner`].
    fn knocked_body(&self, beam: bool) -> Body {
        Body {
            afterburner: if beam {
                self.afterburner_beam
            } else {
                self.afterburner
            },
            ..self.body()
        }
    }

    /// The ship as point defence weighs it as a quarry
    /// ([`defence::choose_ship`]).
    fn as_candidate(&self) -> Candidate {
        Candidate {
            target: self.as_target(),
            pd_target: self.hull.pd_target,
            held_disabled: self.held_disabled,
            cloaked: self.cloaked,
            emerging: self.emerging,
            attacking: self.attacking,
        }
    }

    /// The ship as its point defence's ship branch reads it.
    fn guard(&self) -> Guard {
        Guard {
            ship: self.ship,
            leader: self.leader,
            threatens_player: self.threatens_player,
            scanner: self.cloak_scanner,
        }
    }

    /// What homing guidance reads of the ship (`_ShipECM` and the cloak
    /// test, @0x321b7-0x32335): its jamming, whether it is disabled as
    /// `_IsDisabled` reads it (not intact, or held disabled), and its sight
    /// facts. Read as the tick begins, and again after each landing on the
    /// ship within a loop ([`Lineup::refresh`](lineup::Lineup::refresh)),
    /// once `Combat::settle_struck` has settled its condition, so a later
    /// visit's shot reads a ship an earlier landing disabled as jamming
    /// nothing, as the original reads the live record after `_DamageShip`.
    fn sight(&self) -> Sight {
        Sight {
            ship: self.ship,
            jamming: self.jamming,
            disabled: self.disabled_as_tick_began(),
            cloaked: self.cloaked,
            emerging: self.emerging,
            scanner: self.cloak_scanner,
            has_cloak: self.has_cloak,
        }
    }

    /// Whether `_IsDisabled` (@0x2ce6-0x2e0d) held for the ship as the
    /// tick began, its condition read for its armour: held disabled, or
    /// not intact. [`Fighter::sight`] and [`Fighter::as_target`] read it,
    /// and [`Fighter::disabled_now`] adds the armour the pass's hits left.
    fn disabled_as_tick_began(&self) -> bool {
        self.held_disabled || *self.condition != Condition::Intact
    }

    /// Whether `_IsDisabled` holds for the ship now (@0x2ce6-0x2e0d), as
    /// `rule` says: held disabled, not intact, or its armour under the
    /// line ([`damage::disabled`]). Unlike [`Fighter::as_target`]'s, it
    /// reads the armour the hits landed so far have left, before its
    /// condition settles.
    fn disabled_now(&self, rule: &dyn DisableRule) -> bool {
        self.disabled_as_tick_began() || damage::disabled(self.reserves.armor, &self.hull, rule)
    }

    /// The ship as shots and beams see it.
    fn as_target(&self) -> Target {
        Target {
            ship: self.ship,
            ship_type: self.ship_type,
            fleet: self.fleet,
            ties: self.ties(),
            position: self.state.position,
            velocity: self.state.velocity,
            radius: self.hull.hit_radius,
            sprite_x: self.hull.sprite_size.unwrap_or(beam::NO_SPRITE_WIDTH),
            sheet: self.hull.sprite_size.is_some(),
            condition: *self.condition,
            disabled: self.disabled_as_tick_began(),
            miss_class: self.hull.miss_class,
            slot: self.slot,
        }
    }
}

/// The priority the player's `spec` plays its sound at (see
/// `Combat::fire_player`).
fn player_fire_priority(spec: &weapon::WeaponSpec) -> u8 {
    let beam = matches!(spec.guidance, Guidance::Beam | Guidance::TurretBeam);
    if beam || spec.guidance_99() || spec.secondary() {
        PLAYER_SECONDARY_FIRE_PRIORITY
    } else {
        PLAYER_FIRE_PRIORITY
    }
}

/// What a shot of `AmmoType` [`SELF_DESTRUCT`] does to `fighter`, its
/// firer, as it spawns (`_SpawnShot` @0x3efed-0x3f04d): its shield and
/// armour go to none, for a ship's own shot (source 0-63), a point
/// defence's too, never a sub-munition's (`_SpawnShot`'s fourth argument)
/// nor a beam's or a fighter's, which `_SpawnShot` never places. Step 9
/// then starts its breakup in the same tick. No strike is made: nothing
/// hit it, and no one is to blame. The original also marks the HUD's
/// reserves dirty (@0x3f04d), which redraws them anyway.
fn self_destruct(fighter: &mut Fighter) {
    fighter.reserves.shield.now = 0.0;
    fighter.reserves.armor.now = 0.0;
}

/// Whether `launch`, put in flight as a shot when `shot`
/// ([`Combat::launch`]), destroys its firer: a shot of `AmmoType`
/// [`SELF_DESTRUCT`] that spawned ([`self_destruct`]).
fn self_destructs(launch: &Launch, shot: bool) -> bool {
    shot && launch.weapon.ammo_type == SELF_DESTRUCT
}

/// A volley's sounds: its first copy's at `priority`, its later copies
/// none.
fn first_copy_sounds_at(priority: u8) -> impl Fn(usize) -> Option<FireSound> {
    move |copy| (copy == 0).then_some(FireSound::at(priority))
}

/// The ship `launch`'s damage is aimed at, its firer's `target`: none for
/// a front-quadrant turret firing ahead (@0x88bf0).
fn aimed_at(launch: &Launch, target: Option<ShipRef>) -> Option<ShipRef> {
    let ahead = launch.weapon.guidance == Guidance::FrontTurret && launch.target.is_none();
    if ahead { None } else { target }
}

/// Which ships defend and fire in a step of the tick, point defence
/// first: the player in step 1, as `_HandlePlayer` runs its point defence
/// and then its trigger before the collision passes. The NPCs' point
/// defence runs in `_AIDispatch`'s slot loop (`Combat::dispatch_npcs`),
/// and their picks at each one's own `_HandleShip`
/// ([`Combat::handle_npcs`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Firers {
    /// The player alone.
    Player,
}

impl Firers {
    /// Whether `ship` fires in this step.
    fn take(self, ship: ShipRef) -> bool {
        match self {
            Self::Player => ship == ShipRef::Player,
        }
    }
}

/// Where a ship's launches leave from this tick: the ship as it fires,
/// its hull's exit points, the angle of the frame it was last drawn on,
/// and its cursors ([`exits`]).
struct Muzzle<'a> {
    /// The ship as it fires.
    from: ShipState,
    /// Its hull's exit points, if any.
    exits: Option<&'a ExitPoints>,
    /// The angle, in whole degrees, of the frame it was last drawn on.
    angle: f32,
    /// Its cursors.
    cursors: &'a mut ExitCursors,
    /// Its fold set, which a beam of `ExitType` -1 steps
    /// ([`Armament::fold_set`]).
    fold: &'a mut u16,
    /// Its hull's turret miss class, the range its turret and
    /// point-defence shots roll in.
    miss_class: u32,
    /// Whether its hull's turret shots are drawn over the ships
    /// ([`HullSpec::turret_shots_over`]).
    turret_over: bool,
    /// The ship slot it fires from ([`Shot::firer_slot`]).
    firer_slot: Option<Slot>,
}

impl<'a> Muzzle<'a> {
    /// The muzzle of a ship flying `from` with `hull`, last drawn facing
    /// `drawn_heading`, moving `cursors` and the fold set `fold` on.
    fn new(
        from: ShipState,
        hull: &'a HullSpec,
        drawn_heading: f32,
        (cursors, fold): (&'a mut ExitCursors, &'a mut u16),
    ) -> Self {
        let exits = hull.exits.as_ref();
        let angle = exits.map_or(0.0, |exits| exits.drawn_angle(drawn_heading));
        Self {
            from,
            exits,
            angle,
            cursors,
            fold,
            miss_class: hull.miss_class,
            turret_over: hull.turret_shots_over,
            firer_slot: None,
        }
    }
}

/// Where `ship` is among `targets`, whatever its condition, as the
/// original reads its slot's position.
fn position_of(targets: &[Target], ship: ShipRef) -> Option<Vec2> {
    targets
        .iter()
        .find(|target| target.ship == ship)
        .map(|target| target.position)
}

/// The shield and armour `reserves` hold, together: what a hit takes is
/// how much less this is after it.
fn taken(reserves: &Reserves) -> f32 {
    reserves.shield.now + reserves.armor.now
}

/// Truncates the armour of `fighter`, struck this tick, toward zero when
/// it is the player, intact before the hit and now disabled or with its
/// armour gone, as `rule` says: `_DamageShip`'s newly disabled player
/// (+0x86 = 0, `cvttss2si`/`cvtsi2ss` @0x3b193-0x3b1a1), where
/// `_IsDisabled` holds at armour 0 or below. It comes before the
/// condition is decided, as the original's `_IsDying` test after it
/// (@0x3b1e5) reads the truncated armour: armour truncated to none starts
/// the breakup in the same hit. The session tells the player (see its
/// `player_downed` module).
fn truncate_downed_player(fighter: &mut Fighter, rule: &dyn DisableRule) {
    if fighter.ship != ShipRef::Player || *fighter.condition != Condition::Intact {
        return;
    }
    let armor = fighter.reserves.armor;
    if armour_gone(armor) || holds_armour(armor) && rule.disabled(armor, &fighter.hull) {
        fighter.reserves.armor.now = armor.now.trunc();
    }
}

/// What a beam's hit weighs as a player's stray hit ([`Strike::weight`]):
/// `_HandleBeams` passes 1 as `_DamageShip`'s arg +0x34.
pub const BEAM_WEIGHT: f32 = 1.0;

/// What a missile lock strikes with: `_DamageShip(.., -32000, -32000, ..,
/// can't kill 1, can't disable 1, ..)`, no damage (`_DamageShip` skips
/// damage at or below none), with both floors. With no damage the floors
/// never act, as they act only on armour the hit itself takes; it is a
/// constant so that no mutant of its flags, which no test could tell
/// apart, is made.
const LOCK_HIT: Hit = Hit {
    mass: 0.0,
    energy: 0.0,
    passes_shields: false,
    cant_kill: true,
    cant_disable: true,
};

/// The missile lock `shot` makes this tick among `sides`, if any
/// (`_HandleShot` @0x35627-0x35863): a homing shot the player fired, not
/// jammed, aimed at an NPC that neither the player leads nor shares the
/// player's root (`_ShipsShareParent`), on the tick it passes
/// [`LOCK_AGE`](projectile::LOCK_AGE) ([`Shot::passes_lock_age`]), strikes that NPC with no
/// damage, as `_DamageShip(target, .., -32000, -32000, source, real 1,
/// intended 1, can't kill 1, can't disable 1, ..)`. The ship hit answers
/// it as it would a hit, and the Good Samaritans come, but it costs no
/// damage and is no crime ([`Strike::lock`]).
///
/// The original locks in `_HandleShot`, after the move and before the
/// next frame's collision passes test that move; here too it is made
/// right after the shot's move and lands at its own visit, between the
/// other slots' ends of life ([`Combat::handle_shots`]), before the NPCs
/// fire. The original's target sets +0x70 then, in `_DamageShip`'s
/// reaction block; the session answers the strike after the shot step
/// (`nova_sim::Session::game_work`).
fn lock(shot: &Shot, sides: &Sides) -> Option<Landed> {
    let players = shot.firer == ShipRef::Player && matches!(shot.source, Source::Ship(_));
    if shot.weapon.guidance != Guidance::Homing
        || !players
        || shot.jam != Jam::Clear
        || !shot.passes_lock_age()
    {
        return None;
    }
    let ship = shot
        .aimed_at
        .filter(|ship| matches!(ship, ShipRef::Npc(_)))?;
    let target = sides.targets.iter().find(|target| target.ship == ship)?;
    if target.ties.leader == Some(ShipRef::Player) || sides.share_parent(ship, ShipRef::Player) {
        return None;
    }
    Some(Landed {
        ship,
        by: Some(ShipRef::Player),
        weapon: Some(shot.weapon.id),
        at: target.position,
        hit: LOCK_HIT,
        direct: true,
        intended: true,
        lock: true,
        weight: shot.weapon.reload,
        push: Push {
            from: target.position,
            impact: 0.0,
            beam: false,
        },
        ion: None,
    })
}

/// The hits `shot`'s `blast` lands, each of `hit` and `weight`
/// ([`Strike::weight`]), on the ships among `targets` it [`reached`] whose
/// planet type a blast of `kind` reaches by `reading`
/// ([`planet_type::blast_reaches`]): by the shot's attacker, neither
/// direct nor intended, pushing from the blast's centre and ionizing over
/// its circle.
fn blasted(
    shot: &Shot,
    blast: &Blast,
    (hit, weight): (Hit, f32),
    targets: &[Target],
    (kind, reading): (BlastKind, PlanetBlasts),
) -> Vec<Landed> {
    let planet_weapon = shot.weapon.planet_type();
    let reachable: Vec<Target> = targets
        .iter()
        .filter(|target| {
            planet_type::blast_reaches(kind, reading, planet_weapon, target.ties.planet_type)
        })
        .copied()
        .collect();
    reached(blast, &reachable)
        .into_iter()
        .map(|ship| Landed {
            ship,
            by: shot.attacker(),
            weapon: Some(shot.weapon.id),
            at: blast.at,
            hit,
            direct: false,
            intended: false,
            lock: false,
            weight,
            push: Push {
                from: blast.at,
                impact: shot.weapon.impact,
                beam: false,
            },
            ion: Some(Dose::of(
                &shot.weapon,
                Some(Burst {
                    centre: blast.at,
                    radius: blast.radius,
                }),
            )),
        })
        .collect()
}

/// The ships among `targets` still active, breaking up or not, that
/// `blast` reaches: `_HandleShot`'s blast pass (@0x35fae-0x36106) tests
/// the slot alone (@0x3600a), never `_IsDying`.
fn reached(blast: &Blast, targets: &[Target]) -> Vec<ShipRef> {
    blast.reaches(
        targets
            .iter()
            .filter(|target| target.condition.active())
            .map(|target| (target.ship, target.position)),
    )
}

/// An NPC's regeneration, at its own `_HandleShip` (@0x34424-0x34677,
/// [`Combat::handle_npcs`]): `fighter`'s shields and armour regenerate
/// only while `_IsDisabled` does not hold as its `_HandleShip` began
/// (-0x13e), as `rule` says ([`Fighter::disabled_now`]: held disabled, not
/// intact, or its armour under the line). So a disabled, breaking-up or
/// destroyed ship regenerates neither. It comes before that frame's
/// `_HandleBeams`, so a beam that disables the ship after it leaves it
/// regenerated. A ship whose shield and armour are full has nothing to
/// regenerate, and the rule is not asked of it.
fn regenerate(fighter: &mut Fighter, rule: &dyn DisableRule) {
    let reserves = &fighter.reserves;
    let short = below_full(reserves.shield) || below_full(reserves.armor);
    if short && !fighter.disabled_now(rule) {
        recharge(&mut fighter.reserves.shield, fighter.shield_regen);
        recharge(&mut fighter.reserves.armor, fighter.armor_regen);
    }
}

/// The player's regeneration, in `_HandlePlayer` after its trigger
/// (@0x6cf86-0x6d093, [`Combat::regenerate_player`]): `fighter`'s shields
/// regenerate while it was intact as `_HandlePlayer` began (its flags
/// -0x2cb and -0x2ca, @0x6cfa2; [`Fighter::disabled_as_tick_began`]); its
/// armour only if `_IsDisabled` does not hold now either, as `rule` says
/// (@0x6d035, [`Fighter::disabled_now`]), so armour taken to none since
/// the frame began (a sabotage kill, a self-destruct) regenerates none.
/// Its repair inhibit holds the armour back through
/// [`Fighter::armor_regen`]. As the original tests the capacity first
/// (@0x6cf99, @0x6d01d), a full gauge asks nothing.
fn regenerate_player(fighter: &mut Fighter, rule: &dyn DisableRule) {
    let began_whole = !fighter.disabled_as_tick_began();
    if began_whole && below_full(fighter.reserves.shield) {
        recharge(&mut fighter.reserves.shield, fighter.shield_regen);
    }
    if began_whole && below_full(fighter.reserves.armor) && !fighter.disabled_now(rule) {
        recharge(&mut fighter.reserves.armor, fighter.armor_regen);
    }
}

/// Whether `gauge` is below what it holds, with something to regenerate.
fn below_full(gauge: Gauge) -> bool {
    gauge.now < gauge.max
}

/// Adds `rate` to `gauge` up to what it holds, never taking any away.
fn recharge(gauge: &mut Gauge, rate: f32) {
    gauge.now = gauge.now.max((gauge.now + rate).min(gauge.max));
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::combat::armament::LauncherRounds;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::{BoomId, ShipExits, WeaponRecord};
    use crate::chance::NeverFires;
    use crate::combat::defence::Side;
    use crate::combat::exits::{Compression, ExitCursors, ExitKind, ExitPoints};
    use crate::combat::hull::{DEFAULT_HIT_RADIUS, ESCAPE_POD_SHIP, EXPLOSION_BAND, NovaDisable};
    use crate::combat::weapon::{
        LOSES_LOCK_OFF_AHEAD, SENSOR_INTERFERENCE, TURNS_ON_FIRER, WeaponSpec,
    };
    use crate::ship_layer::ShipLayer;
    use crate::trig;
    use nova_sim_base::fixture::Ranged;
    use nova_sim_catalog::fixture::weapon;

    mod ai_dispatch;
    mod asteroid_beams;
    mod asteroid_contact_first;
    mod asteroid_seekers;
    mod asteroid_walk;
    mod collision_walk;
    mod display_step;
    mod escort_lost_in_place;
    mod exit_steps;
    mod finishing_hits;
    mod fold_stamp;
    mod hidden_shots;
    mod idle_bays;
    mod ion_lockout_tick;
    mod lineup;
    mod phase;
    mod planet_hits;
    mod plunder_fire;
    mod proximity_slots;
    mod rating_stretch;
    mod shot_slots;
    mod shot_visits;
    mod stellar_guns;
    mod stellar_hazards;
    mod strike_terms;
    mod tick_phases;
    mod weapon_glow;

    /// A ship in a test fight, owning what a fighter borrows.
    #[allow(clippy::struct_excessive_bools)]
    struct Ship {
        id: ShipRef,
        class: ShipId,
        fleet: ShipRef,
        govt: Option<GovtId>,
        leader: Option<ShipRef>,
        fleet_escort: bool,
        booty: Option<u16>,
        target: Option<ShipRef>,
        state: ShipState,
        hull: HullSpec,
        shield_regen: f32,
        armor_regen: f32,
        trigger: Trigger,
        reserves: Reserves,
        condition: Condition,
        held_disabled: bool,
        cloaked: bool,
        attacking: Option<ShipRef>,
        threatens_player: bool,
        plundering: bool,
        docked: bool,
        mining: bool,
        emerging: bool,
        jumping_in: bool,
        cloak_scanner: bool,
        has_cloak: bool,
        jamming: Jamming,
        class_speed: f32,
        top_speed: f32,
        jumping: bool,
        afterburner: bool,
        afterburner_beam: bool,
        tractor: Option<TractorLock>,
        ion: Ion,
        ion_capacity: f32,
        armament: Armament,
        rounds: LauncherRounds,
        /// The heading it was last drawn at, when not its heading.
        drawn_heading: Option<f32>,
        /// Where its sprite stands for the collision walk.
        sprite: Option<SpriteRank>,
        /// Its ship slot: by default its berth, its NPC number.
        slot: Slot,
        resists: HazardResistance,
    }

    /// A trigger holding no weapon, its point defence running, as a test
    /// ship's always did before an NPC's ran only when its AI asks.
    const QUIET: Trigger = Trigger {
        primary: false,
        secondary: None,
        only: None,
        bays: false,
        bays_after: false,
        point_defence: true,
        keep: None,
    };

    impl Ship {
        /// NPC `id` at (`x`, `y`), at rest, facing right, unarmed, with
        /// 10 shield and 30 armour, a default hull and a class and top
        /// speed of 10.
        fn at(id: u32, x: f32, y: f32) -> Self {
            Self {
                id: ShipRef::Npc(NpcId(id)),
                class: ShipId(128),
                fleet: ShipRef::Npc(NpcId(id)),
                govt: None,
                leader: None,
                fleet_escort: false,
                booty: None,
                target: None,
                state: ShipState {
                    position: Vec2::new(x, y),
                    velocity: Vec2::ZERO,
                    heading: 90.0,
                },
                hull: HullSpec::default(),
                shield_regen: 0.0,
                armor_regen: 0.0,
                trigger: QUIET,
                reserves: Reserves::full(10.0, 30.0, 100.0),
                condition: Condition::Intact,
                held_disabled: false,
                cloaked: false,
                attacking: None,
                threatens_player: false,
                plundering: false,
                docked: false,
                mining: false,
                emerging: false,
                jumping_in: false,
                cloak_scanner: false,
                has_cloak: false,
                jamming: Jamming::NONE,
                class_speed: 10.0,
                top_speed: 10.0,
                jumping: false,
                afterburner: false,
                afterburner_beam: false,
                tractor: None,
                ion: Ion::default(),
                ion_capacity: 0.0,
                armament: Armament::default(),
                rounds: LauncherRounds::default(),
                drawn_heading: None,
                // As `_InitSprites` leaves the slots (`LayerEntries::new`).
                sprite: Some(SpriteRank {
                    layer: ShipLayer::Ships,
                    entry: -1 - i64::from(id),
                }),
                slot: Slot::Berth(id),
                resists: HazardResistance::default(),
            }
        }

        /// The ship armed with `record`, its trigger held.
        fn armed(self, record: WeaponRecord) -> Self {
            Self {
                armament: Armament::new([(WeaponSpec::new(&record), 1)]),
                trigger: Trigger {
                    primary: true,
                    secondary: None,
                    only: None,
                    bays: false,
                    bays_after: false,
                    point_defence: true,
                    keep: None,
                },
                ..self
            }
        }

        fn fighter(&mut self) -> Fighter<'_> {
            Fighter {
                ship: self.id,
                ship_type: self.class,
                fleet: self.fleet,
                govt: self.govt,
                leader: self.leader,
                fleet_escort: self.fleet_escort,
                booty: self.booty,
                state: self.state,
                hull: self.hull,
                shield_regen: self.shield_regen,
                armor_regen: self.armor_regen,
                trigger: self.trigger,
                target: self.target,
                reserves: &mut self.reserves,
                condition: &mut self.condition,
                held_disabled: self.held_disabled,
                cloaked: self.cloaked,
                attacking: self.attacking,
                threatens_player: self.threatens_player,
                plundering: self.plundering,
                emerging: self.emerging,
                jumping_in: self.jumping_in,
                cloak_scanner: self.cloak_scanner,
                has_cloak: self.has_cloak,
                jamming: self.jamming,
                class_speed: self.class_speed,
                top_speed: self.top_speed,
                jumping: self.jumping,
                afterburner: self.afterburner,
                afterburner_beam: self.afterburner_beam,
                tractor: &mut self.tractor,
                ion: &mut self.ion,
                ion_capacity: self.ion_capacity,
                armament: &mut self.armament,
                rounds: &mut self.rounds,
                drawn_heading: self.drawn_heading.unwrap_or(self.state.heading),
                docked: self.docked,
                mining: self.mining,
                sprite: self.sprite,
                slot: self.slot,
                resists: self.resists,
            }
        }
    }

    fn tick(combat: &mut Combat, ships: &mut [Ship], rule: &dyn DisableRule) {
        let rules = Rules {
            disable: rule,
            ..Rules::default()
        };
        tick_with(combat, ships, &Arsenal::default(), rules);
    }

    fn tick_with(combat: &mut Combat, ships: &mut [Ship], arsenal: &Arsenal, rules: Rules) {
        tick_among(combat, ships, arsenal, &Governments::default(), rules);
    }

    fn tick_among(
        combat: &mut Combat,
        ships: &mut [Ship],
        arsenal: &Arsenal,
        govts: &Governments,
        rules: Rules,
    ) {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        combat.tick(
            &mut fighters,
            arsenal,
            govts,
            rules,
            Field::default(),
            Scenery::default(),
            &mut NeverFires,
        );
    }

    /// A tick among `ships` on `pass`, by the default rules.
    fn tick_on(combat: &mut Combat, ships: &mut [Ship], pass: GamePass) {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        combat.tick_on(
            pass,
            &mut fighters,
            &Arsenal::default(),
            &Governments::default(),
            Rules::default(),
            Field::default(),
            Scenery::default(),
            &mut NeverFires,
        );
    }

    /// A tick among `ships` and `stellars`, with `arsenal`.
    fn tick_by(
        combat: &mut Combat,
        ships: &mut [Ship],
        arsenal: &Arsenal,
        stellars: &mut [StellarTarget],
    ) {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        combat.tick(
            &mut fighters,
            arsenal,
            &Governments::default(),
            Rules::default(),
            Field::default(),
            Scenery {
                stellars,
                ..Scenery::default()
            },
            &mut NeverFires,
        );
    }

    /// A blaster firing every tick, 15 pixels a tick for 13 ticks, doing
    /// 4 mass and 10 energy damage.
    fn blaster() -> WeaponRecord {
        WeaponRecord {
            count: 13,
            speed: 1500,
            mass_dmg: 4,
            energy_dmg: 10,
            ..weapon(128)
        }
    }

    const A: ShipRef = ShipRef::Npc(NpcId(1));
    const B: ShipRef = ShipRef::Npc(NpcId(2));
    const C: ShipRef = ShipRef::Npc(NpcId(3));

    /// The events about `ship`, other than its firing.
    fn about(events: &[CombatEvent], ship: ShipRef) -> Vec<CombatEvent> {
        events
            .iter()
            .filter(|event| match **event {
                CombatEvent::Disabled { ship: s }
                | CombatEvent::BreakingUp { ship: s, .. }
                | CombatEvent::Destroyed { ship: s, .. } => s == ship,
                _ => false,
            })
            .copied()
            .collect()
    }

    fn fired(events: &[CombatEvent], ship: ShipRef) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, CombatEvent::Fired { ship: s, .. } if *s == ship))
            .count()
    }

    /// The shots in flight after `ship`, firing `record`, fires a tick
    /// among `others`.
    fn launched_by(ship: Ship, record: WeaponRecord, others: Vec<Ship>) -> Combat {
        let mut combat = Combat::default();
        let firer = ship.id;
        let mut ships = vec![ship.armed(record)];
        ships.extend(others);
        phase::fire(&mut combat, &mut ships, firer, &NovaDisable);
        combat
    }

    /// The shots in flight after `ship`, firing `record`, fires among
    /// `others` and then lets them take their first move
    /// ([`phase::fly_shots`]).
    fn launched_and_flown(ship: Ship, record: WeaponRecord, others: Vec<Ship>) -> Combat {
        let mut combat = Combat::default();
        let firer = ship.id;
        let mut ships = vec![ship.armed(record)];
        ships.extend(others);
        phase::fire(&mut combat, &mut ships, firer, &NovaDisable);
        phase::fly_shots(&mut combat, &mut ships, &NovaDisable);
        combat
    }

    #[test]
    fn every_shot_a_ship_fires_is_aimed_at_its_target() {
        let homing = WeaponRecord {
            guidance: 1,
            guided_turn: 70,
            ..blaster()
        };
        for record in [blaster(), homing] {
            let combat = launched_by(
                Ship::at(1, 0.0, 0.0).targeting(B),
                record,
                vec![Ship::at(2, 0.0, -300.0)],
            );
            assert_eq!(combat.shots().len(), 1);
            assert_eq!(combat.shots()[0].aimed_at, Some(B), "{:?}", record.guidance);
        }
        let combat = launched_by(Ship::at(1, 0.0, 0.0), blaster(), Vec::new());
        assert_eq!(combat.shots()[0].aimed_at, None, "no target");
        let gone = launched_and_flown(Ship::at(1, 0.0, 0.0).targeting(C), blaster(), Vec::new());
        assert_eq!(
            (gone.shots()[0].aimed_at, gone.shots()[0].jam),
            (None, Jam::Lost),
            "its target not in the fight: lost on its first move"
        );
    }

    #[test]
    fn a_front_turret_firing_ahead_is_aimed_at_nothing() {
        let front = WeaponRecord {
            guidance: 7,
            ..blaster()
        };
        // Facing right: B ahead, C behind.
        let ahead = launched_by(
            Ship::at(1, 0.0, 0.0).targeting(B),
            front,
            vec![Ship::at(2, 300.0, 0.0)],
        );
        assert_eq!(ahead.shots()[0].aimed_at, Some(B));
        let astern = launched_by(
            Ship::at(1, 0.0, 0.0).targeting(C),
            front,
            vec![Ship::at(3, -300.0, 0.0)],
        );
        assert_eq!(astern.shots()[0].aimed_at, None, "fired straight ahead");
    }

    /// A shot of `record` by A, aimed at B.
    fn aimed(record: WeaponRecord) -> Shot {
        Shot {
            aimed_at: Some(B),
            ..Shot::launch(WeaponSpec::new(&record), A, A, &ShipState::default(), 0.0)
        }
    }

    #[test]
    fn the_damage_aimed_at_a_ship_is_half_its_shots_mass_and_energy_truncated_after_each() {
        let five = WeaponRecord {
            mass_dmg: 2,
            energy_dmg: 3,
            ..blaster()
        };
        let mut combat = Combat {
            shots: vec![aimed(five), aimed(five), aimed(five)],
            ..Combat::default()
        };
        assert_eq!(
            combat.targeted_damage(),
            BTreeMap::from([(B, 6)]),
            "2.5 three times, truncated each time: not 7"
        );
        let missile = WeaponRecord {
            guidance: 1,
            mass_dmg: 30,
            energy_dmg: 11,
            ..blaster()
        };
        combat.shots = vec![
            aimed(missile),
            aimed(blaster()),
            Shot {
                aimed_at: Some(C),
                ..aimed(five)
            },
            Shot {
                aimed_at: None,
                ..aimed(missile)
            },
            Shot {
                jam: Jam::Lost,
                ..aimed(missile)
            },
        ];
        // 20.5 truncated to 20, then 20 + 7 = 27.
        assert_eq!(combat.targeted_damage(), BTreeMap::from([(B, 27), (C, 2)]));
        let negative = WeaponRecord {
            mass_dmg: -3,
            energy_dmg: 0,
            ..blaster()
        };
        combat.shots = vec![aimed(negative), aimed(five)];
        assert_eq!(
            combat.targeted_damage(),
            BTreeMap::from([(B, 1)]),
            "-1.5 truncates to -1, then 1.5 to 1"
        );
        assert_eq!(Combat::default().targeted_damage(), BTreeMap::new());
        assert_eq!(AIMED_DAMAGE_SHARE, 0.5);
    }

    #[test]
    fn point_defence_is_aimed_at_no_ship() {
        let quad = WeaponRecord {
            guidance: 9,
            ..blaster()
        };
        let mut combat = Combat::default();
        let missile = WeaponRecord {
            guidance: 1,
            guided_turn: 70,
            count: 100,
            speed: 100,
            ..weapon(134)
        };
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(quad).targeting(B),
            Ship::at(2, 150.0, 0.0).armed(missile).targeting(A),
        ];
        missile_in_flight(&mut combat, &mut ships);
        tick(&mut combat, &mut ships, &NovaDisable);
        let defence: Vec<&Shot> = combat
            .shots()
            .iter()
            .filter(|shot| shot.weapon.guidance == Guidance::PointDefence)
            .collect();
        assert_eq!(defence.len(), 1, "{:?}", combat.shots());
        assert_eq!(defence[0].aimed_at, None);
    }

    /// How many shots `ship`, firing `record`, fires in a tick.
    fn shots_of(ship: Ship, record: WeaponRecord) -> usize {
        let mut combat = Combat::default();
        let id = ship.id;
        let mut ships = [ship.armed(record)];
        phase::fire(&mut combat, &mut ships, id, &NovaDisable);
        fired(&combat.take_events(), id)
    }

    #[test]
    fn a_fighter_fires_as_weapon_has_ammo_says_of_its_ship() {
        use crate::combat::weapon::{AI_WONT_USE, FIRES_CLOAKED, NEEDS_KEY_SHIP};
        let wont = WeaponRecord {
            flags2: AI_WONT_USE,
            ..blaster()
        };
        assert_eq!(shots_of(Ship::at(1, 0.0, 0.0), wont), 0, "an NPC");
        let player = Ship {
            id: ShipRef::Player,
            fleet: ShipRef::Player,
            ..Ship::at(1, 0.0, 0.0)
        };
        assert_eq!(shots_of(player, wont), 1, "the player");
        let cloaked = || Ship {
            cloaked: true,
            ..Ship::at(1, 0.0, 0.0)
        };
        assert_eq!(shots_of(cloaked(), blaster()), 0, "cloaked");
        let fires_cloaked = WeaponRecord {
            flags2: FIRES_CLOAKED,
            ..blaster()
        };
        assert_eq!(shots_of(cloaked(), fires_cloaked), 1);
        // A key ship needed, its class's `KeyCarried` read from the hull.
        let keyed = WeaponRecord {
            flags2: NEEDS_KEY_SHIP,
            ..blaster()
        };
        let carrier = |key| {
            let mut ship = Ship::at(1, 0.0, 0.0).armed(keyed);
            ship.hull.key_carried = key;
            ship.armament = Armament::new([
                (WeaponSpec::new(&keyed), 1),
                (
                    WeaponSpec::new(&WeaponRecord {
                        guidance: 99,
                        ammo_type: 200,
                        ..weapon(150)
                    }),
                    1,
                ),
            ]);
            ship.rounds = LauncherRounds::from([(WeaponId(150), 1)]);
            let mut combat = Combat::default();
            let mut ships = [ship];
            tick(&mut combat, &mut ships, &NovaDisable);
            fired(&combat.take_events(), A)
        };
        assert_eq!(carrier(Some(ShipId(200))), 1);
        assert_eq!(carrier(None), 0);
    }

    /// The reload a ship's `Seeker` 0x0020 blaster (reload 0) is left at
    /// after a tick's fire and reload, its ship `player` or an NPC of
    /// ionization factor `factor`, under `lockout`; and how many shots it
    /// fired.
    fn ion_held_reload(player: bool, factor: f32, lockout: RuleSource) -> (f32, usize) {
        let mut combat = Combat::default();
        combat.set_ion_lockout(lockout);
        let mut ship = Ship::at(1, 0.0, 0.0).armed(WeaponRecord {
            reload: 0,
            seeker: ion::CANT_FIRE_IONIZED,
            ..blaster()
        });
        if player {
            ship.id = ShipRef::Player;
            ship.fleet = ShipRef::Player;
        }
        ship.ion_capacity = 100.0;
        ship.ion.charge = factor * 100.0;
        let id = ship.id;
        let mut ships = [ship];
        tick(&mut combat, &mut ships, &NovaDisable);
        let shots = fired(&combat.take_events(), id);
        (ships[0].armament.mounts()[0].reload, shots)
    }

    #[test]
    fn the_reload_step_holds_an_ionized_ships_seeker_0x0020_weapon_by_the_lockout() {
        let engine = RuleSource::Engine;
        assert_eq!(
            ion_held_reload(true, 0.5, engine),
            (ion::HELD_RELOAD, 1),
            "player: fires, then held"
        );
        assert_eq!(
            ion_held_reload(false, 0.5, engine),
            (0.0, 1),
            "an NPC under 1"
        );
        // An NPC's hold comes with its reload step, before its fire, and
        // its 1.0 stays until its next step counts it down (@0x338f0).
        assert_eq!(
            ion_held_reload(false, 1.0, engine),
            (ion::HELD_RELOAD, 0),
            "an NPC at 1"
        );
        let bible = RuleSource::Bible;
        assert_eq!(ion_held_reload(true, 1.0, bible), (0.0, 1), "unguided");
        assert_eq!(ion_held_reload(false, 1.0, bible), (0.0, 1), "unguided");
    }

    #[test]
    fn an_npcs_shot_waits_at_its_launch_point_the_tick_it_is_fired() {
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster())];
        ships[0].govt = Some(GovtId(140));
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            combat.take_events(),
            [CombatEvent::Fired {
                ship: A,
                weapon: WeaponId(128),
                at: Vec2::ZERO,
                sound: Some(crate::combat::FireSound::at(
                    crate::combat::AI_FIRE_PRIORITY
                )),
            }]
        );
        assert_eq!(combat.take_events(), [], "taken");
        assert_eq!(combat.shots().len(), 1);
        assert_eq!(combat.shots()[0].govt(), Some(GovtId(140)), "its firer's");
        // `_HandleShip` fires it after `_HandleShot` has moved the shots
        // (@0x44483 after @0x44385): it first moves the next tick.
        assert_eq!(combat.shots()[0].position, Vec2::ZERO, "unmoved");
        assert_eq!(combat.beams(), []);
        for _ in 1..14 {
            tick(&mut combat, &mut [Ship::at(1, 0.0, 0.0)], &NovaDisable);
        }
        // Its 13th move, on the 14th tick, spent it: it is drawn at its last point for a
        // tick, and its end of life comes in the next tick's hit pass.
        assert_eq!(combat.shots().len(), 1, "spent, at its last point");
        assert!((combat.shots()[0].position.x - 195.0).abs() < 1e-3);
        tick(&mut combat, &mut [Ship::at(1, 0.0, 0.0)], &NovaDisable);
        assert_eq!(combat.shots(), [], "gone the tick after its count");
    }

    #[test]
    fn a_ship_is_disabled_then_breaks_up_then_is_destroyed_by_further_hits() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0).armed(blaster());
        target.hull.death_delay = 3;
        target.hull.explosion = Explosion::decode(5);
        target.hull.breakup = Explosion::decode(4);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster()), target];
        let mut events = Vec::new();
        let mut log = Vec::new();
        for tick_no in 0..40 {
            tick(&mut combat, &mut ships, &NovaDisable);
            let new = combat.take_events();
            log.push((tick_no, fired(&new, B), about(&new, B)));
            events.extend(new);
        }
        let seen: Vec<(u32, CombatEvent)> = log
            .iter()
            .flat_map(|(tick_no, _, about)| about.iter().map(|event| (*tick_no, *event)))
            .collect();
        assert_eq!(seen.len(), 3, "{seen:?}");
        let (disabled_on, disabled) = seen[0];
        let (breaking_on, breaking) = seen[1];
        let (destroyed_on, destroyed) = seen[2];
        assert_eq!(disabled, CombatEvent::Disabled { ship: B });
        assert_eq!(
            breaking,
            CombatEvent::BreakingUp {
                ship: B,
                at: Vec2::new(100.0, 0.0),
                explosion: Explosion::decode(4)
            }
        );
        assert_eq!(
            destroyed,
            CombatEvent::Destroyed {
                ship: B,
                ship_type: ShipId(128),
                at: Vec2::new(100.0, 0.0),
                velocity: Vec2::ZERO,
                explosion: Some(Explosion {
                    boom: BoomId(133),
                    extra: false
                }),
                size: 0.0
            }
        );
        // Six hits take the shield and 24 armour (6 left, below a third):
        // disabled; two more leave it at -2.
        assert_eq!(breaking_on - disabled_on, 2);
        assert_eq!(
            destroyed_on - breaking_on,
            1,
            "its DeathDelay less 2: an NPC's timer of 3 takes 1 off to 2"
        );
        assert!(
            log.iter()
                .filter(|(tick_no, _, _)| *tick_no > disabled_on)
                .all(|(_, fired, _)| *fired == 0),
            "a disabled ship stops firing"
        );
        assert!(
            log.iter()
                .take(disabled_on as usize)
                .all(|(_, fired, _)| *fired == 1)
        );
        assert_eq!(ships[1].condition, Condition::Destroyed);
        assert!(ships[1].reserves.armor.now < 0.0);
        assert_eq!(ships[0].condition, Condition::Intact, "B's shots flew away");
    }

    #[test]
    fn a_ship_without_a_death_delay_is_destroyed_as_it_breaks_up() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 0.0, 0.0);
        target.reserves.armor.now = 0.0;
        let mut ships = [target];
        tick(&mut combat, &mut ships, &NovaDisable);
        let events = combat.take_events();
        assert!(matches!(events[0], CombatEvent::BreakingUp { ship: B, .. }));
        assert!(matches!(events[1], CombatEvent::Destroyed { ship: B, .. }));
        assert_eq!(ships[0].condition, Condition::Destroyed);
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.take_events(), [], "destroyed once");
    }

    /// A ship whose armour is gone, breaking up for `death_delay`: the
    /// player's, or NPC 2's.
    fn breaking(player: bool, death_delay: u32) -> Ship {
        let mut ship = Ship::at(2, 0.0, 0.0);
        if player {
            ship.id = ShipRef::Player;
            ship.fleet = ShipRef::Player;
        }
        ship.reserves.armor.now = 0.0;
        ship.hull.death_delay = death_delay;
        ship
    }

    /// A fight whose breakups below the explosion band go as `breakup`
    /// says.
    fn breaking_up_by(breakup: Breakup) -> Combat {
        let mut combat = Combat::default();
        combat.set_breakup(breakup);
        combat
    }

    /// How many ticks after the one it breaks up on `ship` is destroyed,
    /// under `breakup`: none within 400.
    fn breakup_ticks(ship: Ship, breakup: Breakup) -> Option<u32> {
        let mut combat = breaking_up_by(breakup);
        let mut ships = [ship];
        for tick_no in 0..400 {
            tick(&mut combat, &mut ships, &NovaDisable);
            let events = combat.take_events();
            if tick_no == 0 {
                assert!(
                    matches!(events.first(), Some(CombatEvent::BreakingUp { .. })),
                    "{events:?}"
                );
            }
            if events
                .iter()
                .any(|event| matches!(event, CombatEvent::Destroyed { .. }))
            {
                assert_eq!(ships[0].condition, Condition::Destroyed);
                return Some(tick_no);
            }
        }
        None
    }

    #[test]
    fn a_players_breakup_lasts_three_times_its_death_delay_less_two() {
        for breakup in [Breakup::Explodes, Breakup::Stalls] {
            for (death_delay, after) in [(1, 1), (2, 4), (60, 178)] {
                assert_eq!(
                    breakup_ticks(breaking(true, death_delay), breakup),
                    Some(after),
                    "{breakup:?}: {death_delay}"
                );
            }
        }
    }

    #[test]
    fn an_npcs_breakup_lasts_its_death_delay_less_two_and_one_tick_at_two() {
        for breakup in [Breakup::Explodes, Breakup::Stalls] {
            for (death_delay, after) in [(2, 1), (3, 1), (4, 2), (60, 58)] {
                assert_eq!(
                    breakup_ticks(breaking(false, death_delay), breakup),
                    Some(after),
                    "{breakup:?}: {death_delay}"
                );
            }
        }
    }

    #[test]
    fn the_breakup_tick_sets_the_timer_and_marks_it_starting() {
        for (player, timer) in [(true, 15), (false, 5)] {
            let mut combat = Combat::default();
            let mut ships = [breaking(player, 5)];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(
                ships[0].condition,
                Condition::Dying {
                    ticks_left: timer,
                    starting: true
                },
                "player {player}"
            );
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(
                ships[0].condition,
                Condition::Dying {
                    ticks_left: timer - 1,
                    starting: false
                },
                "player {player}"
            );
        }
    }

    #[test]
    fn an_ejected_pod_is_raised_from_where_its_ship_is_at_its_velocity() {
        let mut combat = Combat::default();
        let (at, velocity) = (Vec2::new(10.0, -4.0), Vec2::new(1.5, 0.5));
        combat.eject_pod(ShipRef::Npc(NpcId(3)), at, velocity);
        assert_eq!(
            combat.take_events(),
            [CombatEvent::PodEjected {
                ship: ShipRef::Npc(NpcId(3)),
                at,
                velocity,
            }]
        );
    }

    #[test]
    fn an_npc_with_a_death_delay_of_one_explodes_the_tick_after_by_default() {
        let mut combat = Combat::default();
        let mut ships = [breaking(false, 1)];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            ships[0].condition,
            Condition::Dying {
                ticks_left: 1,
                starting: true
            }
        );
        assert_eq!(
            breakup_ticks(breaking(false, 1), Breakup::default()),
            Some(1)
        );
    }

    #[test]
    fn a_player_without_a_death_delay_is_destroyed_as_it_breaks_up_by_default() {
        assert_eq!(
            breakup_ticks(breaking(true, 0), Breakup::default()),
            Some(0)
        );
    }

    #[test]
    fn by_the_originals_reading_a_breakup_below_the_band_never_ends() {
        for (player, death_delay) in [(true, 0), (false, 0), (false, 1)] {
            let mut combat = breaking_up_by(Breakup::Stalls);
            let mut ships = [breaking(player, death_delay)];
            let mut events = Vec::new();
            for _ in 0..300 {
                tick(&mut combat, &mut ships, &NovaDisable);
                events.extend(combat.take_events());
            }
            assert_eq!(
                ships[0].condition,
                Condition::Dying {
                    ticks_left: death_delay,
                    starting: true
                },
                "player {player}, {death_delay}: set again every tick"
            );
            assert!(
                matches!(events[..], [CombatEvent::BreakingUp { .. }]),
                "player {player}, {death_delay}: {events:?}"
            );
        }
    }

    #[test]
    fn a_ship_that_holds_no_armour_never_breaks_up() {
        let mut combat = Combat::default();
        let mut armourless = Ship::at(2, 0.0, 0.0);
        armourless.reserves.armor = Gauge::full(0.0);
        let mut ships = [armourless];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].condition, Condition::Intact);
        assert_eq!(combat.take_events(), []);
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].condition, Condition::Intact, "none to lose");
        ships[0].reserves.armor = Gauge { now: 0.0, max: 0.5 };
        tick(&mut combat, &mut ships, &NovaDisable);
        assert!(matches!(ships[0].condition, Condition::Destroyed));
    }

    #[test]
    fn a_ship_that_holds_no_armour_is_never_disabled_by_a_hit() {
        let mut combat = Combat::default();
        let mut armourless = Ship::at(2, 100.0, 0.0);
        armourless.reserves.shield.now = 0.0;
        armourless.reserves.armor = Gauge::full(0.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster()), armourless];
        let mut events = Vec::new();
        for _ in 0..20 {
            tick(&mut combat, &mut ships, &NovaDisable);
            events.extend(combat.take_events());
        }
        assert!(
            events
                .iter()
                .any(|event| matches!(event, CombatEvent::Fired { ship: A, .. })),
            "{events:?}"
        );
        assert!(
            ships[1].reserves.shield.now < 0.0,
            "hit with its shields down"
        );
        assert_eq!(ships[1].reserves.armor, Gauge::full(0.0), "none to take");
        assert_eq!(ships[1].condition, Condition::Intact);
        assert_eq!(about(&events, B), []);
        let disabling = Recording {
            disabled: true,
            ..Recording::default()
        };
        ships[1].reserves.armor.now = -1.0;
        tick(&mut combat, &mut ships, &disabling);
        assert_eq!(ships[1].condition, Condition::Intact, "whatever the rule");
        assert_eq!(disabling.asked.take().len(), 1, "only of the armoured");
        assert_eq!(about(&combat.take_events(), B), []);
    }

    #[test]
    fn a_heavy_ship_is_destroyed_in_an_explosion_its_mass_sizes() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 0.0, 0.0);
        target.reserves.armor.now = -1.0;
        target.hull.death_delay = 60;
        target.hull.mass = 400.0;
        let mut ships = [target];
        for _ in 0..=60 {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        let events = combat.take_events();
        assert!(
            matches!(
                events.last(),
                Some(CombatEvent::Destroyed { size, .. }) if *size == 80.0
            ),
            "{events:?}"
        );
    }

    /// A ship of `mass` tons at (`x`, `y`) whose armour is gone, with no
    /// death delay, so it breaks up and explodes on the next tick: `id`,
    /// an NPC or the player.
    fn wreck(id: ShipRef, mass: f32, x: f32, y: f32) -> Ship {
        let mut ship = Ship::at(9, x, y);
        ship.id = id;
        ship.fleet = id;
        ship.reserves.armor.now = 0.0;
        ship.hull.mass = mass;
        ship
    }

    /// NPC `id` at (`x`, `y`), at rest, with `shield` shields and `armor`
    /// of `armor_max` armour.
    fn holding(id: u32, x: f32, y: f32, shield: f32, armor: f32, armor_max: f32) -> Ship {
        let mut ship = Ship::at(id, x, y);
        ship.reserves = Reserves::full(shield, armor_max, 100.0);
        ship.reserves.armor.now = armor;
        ship
    }

    /// The shields and armour `ship` holds now.
    fn left(ship: &Ship) -> (f32, f32) {
        (ship.reserves.shield.now, ship.reserves.armor.now)
    }

    const WRECK: ShipRef = ShipRef::Npc(NpcId(9));

    #[test]
    fn a_heavy_ships_final_explosion_hits_a_ship_65_pixels_off_and_not_66() {
        let mut combat = Combat::default();
        let mut ships = [
            wreck(WRECK, 200.0, 0.0, 0.0),
            holding(1, 65.0, -65.0, 100.0, 300.0, 300.0),
            holding(2, 66.0, 0.0, 100.0, 300.0, 300.0),
            holding(3, -30.0, 66.0, 100.0, 300.0, 300.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].condition, Condition::Destroyed);
        assert_eq!(left(&ships[1]), (68.0, 300.0));
        assert_eq!(left(&ships[2]), (100.0, 300.0), "66 off on x");
        assert_eq!(left(&ships[3]), (100.0, 300.0), "66 off on y");
        let strikes = combat.take_strikes();
        assert_eq!(
            strikes,
            [Strike {
                ship: A,
                by: Some(WRECK),
                damage: 32.0,
                downed: None,
                direct: false,
                intended: false,
                lock: false,
                nominal: Nominal {
                    mass: 32.0,
                    energy: 32.0,
                },
                weight: 0.0,
                answered: false,
            }]
        );
    }

    #[test]
    fn a_planet_type_or_light_ship_explodes_without_a_shockwave() {
        let mut planet = wreck(WRECK, 400.0, 0.0, 0.0);
        planet.hull.planet_type = true;
        for wrecked in [planet, wreck(WRECK, 99.0, 0.0, 0.0)] {
            let mut combat = Combat::default();
            let mut ships = [wrecked, holding(1, 10.0, 0.0, 100.0, 300.0, 300.0)];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(ships[0].condition, Condition::Destroyed);
            assert_eq!(left(&ships[1]), (100.0, 300.0));
            assert_eq!(combat.take_strikes(), []);
            assert_eq!(combat.take_shoves(), []);
            let events = combat.take_events();
            assert!(
                events.iter().any(
                    |event| matches!(event, CombatEvent::Destroyed { size, .. } if *size == 0.0)
                ),
                "{events:?}"
            );
        }
    }

    #[test]
    fn the_players_final_explosion_hits_npcs_and_not_itself() {
        let mut combat = Combat::default();
        let mut npc = holding(1, 40.0, 0.0, 100.0, 300.0, 300.0);
        npc.hull.mass = 200.0;
        let mut ships = [wreck(ShipRef::Player, 200.0, 0.0, 0.0), npc];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].condition, Condition::Destroyed);
        assert_eq!(left(&ships[1]), (68.0, 300.0));
        let strikes = combat.take_strikes();
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        assert_eq!((strikes[0].ship, strikes[0].by), (A, Some(ShipRef::Player)));
        let shoves = combat.take_shoves();
        assert_eq!(shoves.len(), 1, "{shoves:?}");
        assert_eq!(shoves[0].ship, A);
    }

    #[test]
    fn an_npcs_final_explosion_hits_the_player() {
        let mut combat = Combat::default();
        let mut player = holding(1, 0.0, 40.0, 100.0, 300.0, 300.0);
        player.id = ShipRef::Player;
        player.fleet = ShipRef::Player;
        player.hull.mass = 200.0;
        let mut ships = [wreck(WRECK, 200.0, 0.0, 0.0), player];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(left(&ships[1]), (68.0, 300.0));
        let strikes = combat.take_strikes();
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        assert_eq!(
            (strikes[0].ship, strikes[0].by),
            (ShipRef::Player, Some(WRECK))
        );
        let shoves = combat.take_shoves();
        assert_eq!(shoves.len(), 1, "{shoves:?}");
        assert_eq!(shoves[0].ship, ShipRef::Player);
        assert!(
            (shoves[0].velocity - Vec2::new(0.0, 3.75)).length() < 1e-4,
            "away from the wreck: {shoves:?}"
        );
    }

    #[test]
    fn a_shockwave_strikes_by_the_wreck_and_says_when_it_disabled() {
        let mut combat = Combat::default();
        let mut ships = [
            wreck(WRECK, 200.0, 0.0, 0.0),
            holding(1, 20.0, 0.0, 0.0, 300.0, 300.0),
            holding(2, -20.0, 0.0, 0.0, 13_350.0, 40_000.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(left(&ships[1]), (0.0, 268.0));
        assert_eq!(ships[1].condition, Condition::Intact);
        assert_eq!(left(&ships[2]), (0.0, 13_333.0), "floored, still disabled");
        assert_eq!(ships[2].condition, Condition::Disabled);
        assert_eq!(
            combat.take_strikes(),
            [
                Strike {
                    ship: A,
                    by: Some(WRECK),
                    damage: 32.0,
                    downed: None,
                    direct: false,
                    intended: false,
                    lock: false,
                    nominal: Nominal {
                        mass: 32.0,
                        energy: 32.0,
                    },
                    weight: 0.0,
                    answered: false,
                },
                Strike {
                    ship: B,
                    by: Some(WRECK),
                    damage: 17.0,
                    downed: Some(Downed::Disabled),
                    direct: false,
                    intended: false,
                    lock: false,
                    nominal: Nominal {
                        mass: 32.0,
                        energy: 32.0,
                    },
                    weight: 0.0,
                    answered: false,
                },
            ]
        );
        let events = combat.take_events();
        let disabled: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, CombatEvent::Disabled { .. }))
            .collect();
        assert_eq!(disabled, [&CombatEvent::Disabled { ship: B }]);
    }

    #[test]
    fn a_shockwave_shoves_a_breaking_up_ship_too_and_counts_it_down_once() {
        let mut combat = Combat::default();
        let mut dying = holding(1, 40.0, 0.0, 0.0, -1.0, 300.0);
        dying.hull.mass = 200.0;
        dying.condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
        let mut ships = [wreck(WRECK, 200.0, 0.0, 0.0), dying];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(left(&ships[1]), (0.0, -33.0));
        assert_eq!(
            ships[1].condition,
            Condition::Dying {
                ticks_left: 8,
                starting: false
            }
        );
        let shoves = combat.take_shoves();
        assert_eq!(shoves.len(), 1, "{shoves:?}");
        assert_eq!(shoves[0].ship, A);
        assert!(
            (shoves[0].velocity - Vec2::new(3.75, 0.0)).length() < 1e-4,
            "{shoves:?}"
        );
        let strikes = combat.take_strikes();
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        assert_eq!((strikes[0].damage, strikes[0].downed), (32.0, None));
    }

    #[test]
    fn a_ship_whose_jump_is_under_way_is_hit_but_not_shoved() {
        let mut combat = Combat::default();
        let mut jumping = holding(1, 40.0, 0.0, 100.0, 300.0, 300.0);
        jumping.hull.mass = 200.0;
        jumping.jumping = true;
        let mut ships = [wreck(WRECK, 200.0, 0.0, 0.0), jumping];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(left(&ships[1]), (68.0, 300.0));
        assert_eq!(combat.take_shoves(), []);
    }

    #[test]
    fn a_shove_is_capped_at_the_ships_class_speed_and_clamped_to_its_top_speed() {
        let mut combat = Combat::default();
        let mut slow = holding(1, 40.0, 0.0, 100.0, 300.0, 300.0);
        slow.hull.mass = 100.0;
        slow.state.velocity = Vec2::new(-9.0, 7.0);
        slow.class_speed = 4.0;
        slow.top_speed = 6.0;
        let mut ships = [wreck(WRECK, 200.0, 0.0, 0.0), slow];
        tick(&mut combat, &mut ships, &NovaDisable);
        let shoves = combat.take_shoves();
        assert_eq!(shoves.len(), 1, "{shoves:?}");
        // -9 + 7.5 is -1.5, short of the class speed; 7 is clamped to 6.
        assert!(
            (shoves[0].velocity - Vec2::new(-1.5, 6.0)).length() < 1e-4,
            "{shoves:?}"
        );
    }

    #[test]
    fn under_the_circle_a_shove_is_clamped_to_its_top_speeds_circle() {
        let mut combat = Combat::default();
        combat.set_speed_cap(RuleSource::Bible);
        let mut slow = holding(1, 40.0, 0.0, 100.0, 300.0, 300.0);
        slow.hull.mass = 100.0;
        slow.state.velocity = Vec2::new(-9.0, 7.0);
        slow.class_speed = 4.0;
        slow.top_speed = 6.0;
        let mut ships = [wreck(WRECK, 200.0, 0.0, 0.0), slow];
        tick(&mut combat, &mut ships, &NovaDisable);
        let shoves = combat.take_shoves();
        assert_eq!(shoves.len(), 1, "{shoves:?}");
        let pushed = Vec2::new(-1.5, 7.0);
        let expected = pushed * (6.0 / pushed.length());
        assert!(
            (shoves[0].velocity - expected).length() < 1e-4,
            "{shoves:?}"
        );
        let mut engine = Combat::default();
        engine.set_speed_cap(RuleSource::Engine);
        assert_eq!(engine, Combat::default(), "the engine's is the default");
    }

    /// Says every ship is `disabled`, recording what it was asked.
    #[derive(Debug, Default)]
    struct Recording {
        disabled: bool,
        asked: RefCell<Vec<(Gauge, HullSpec)>>,
    }

    impl DisableRule for Recording {
        fn disabled(&self, armor: Gauge, hull: &HullSpec) -> bool {
            self.asked.borrow_mut().push((armor, *hull));
            self.disabled
        }
    }

    #[test]
    fn the_disable_rule_given_decides_and_is_asked_of_every_ship_still_whole() {
        let mut combat = Combat::default();
        let mut tough = Ship::at(2, 50.0, 0.0);
        tough.hull.tough = true;
        tough.reserves.armor.now = 29.0;
        let mut dying = Ship::at(3, 90.0, 0.0);
        dying.condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
        let mut ships = [Ship::at(1, 0.0, 0.0), tough, dying];
        let rule = Recording {
            disabled: true,
            ..Recording::default()
        };
        tick(&mut combat, &mut ships, &rule);
        let asked = rule.asked.take();
        // Slot by slot (@0x4445c): A at its display, its condition
        // settled; the tough ship at its `_HandleShip`'s regeneration,
        // which reads `_IsDisabled` live (@0x34424), its armour short of
        // full, then at its display; never the dying.
        assert_eq!(asked.len(), 3, "not of the dying");
        assert_eq!(asked[1].0, asked[2].0, "the tough ship's regeneration");
        assert_eq!(
            asked[1].0,
            Gauge {
                now: 29.0,
                max: 30.0
            }
        );
        assert!(asked[1].1.tough);
        assert_eq!(
            about(&combat.take_events(), A),
            [CombatEvent::Disabled { ship: A }]
        );
        assert_eq!(ships[1].condition, Condition::Disabled);
        tick(&mut combat, &mut ships, &rule);
        assert_eq!(combat.take_events(), [], "disabled once");
        let well = Recording::default();
        tick(&mut combat, &mut ships, &well);
        assert_eq!(ships[1].condition, Condition::Intact, "as the rule says");
        assert_eq!(combat.take_events(), []);
    }

    #[test]
    fn a_held_ship_at_full_armour_is_stored_disabled_with_no_event_and_nothing_back() {
        let mut combat = Combat::default();
        let mut held = Ship::at(1, 0.0, 0.0);
        held.held_disabled = true;
        held.reserves.armor.now = 20.0;
        held.armor_regen = 1.0;
        held.shield_regen = 1.0;
        held.reserves.shield.now = 0.0;
        let mut ships = [held];
        let well = Recording::default();
        for _ in 0..3 {
            tick(&mut combat, &mut ships, &well);
            assert_eq!(ships[0].condition, Condition::Disabled);
        }
        assert_eq!(combat.take_events(), [], "never newly disabled");
        assert_eq!(well.asked.take(), [], "the rule is not asked");
        assert_eq!(ships[0].reserves.armor.now, 20.0, "no armour back");
        assert_eq!(
            ships[0].reserves.shield.now, 0.0,
            "nor shields: it is disabled"
        );
        ships[0].reserves.armor.now = 30.0;
        tick(&mut combat, &mut ships, &well);
        assert_eq!(ships[0].condition, Condition::Disabled, "at full armour");
    }

    #[test]
    fn a_held_ship_hit_strikes_no_downing() {
        let mut combat = Combat::default();
        let mut held = Ship::at(2, 100.0, 0.0);
        held.held_disabled = true;
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster()), held];
        let mut strikes = Vec::new();
        for _ in 0..12 {
            tick(&mut combat, &mut ships, &NovaDisable);
            strikes.extend(combat.take_strikes());
            if ships[1].reserves.armor.now <= 0.0 {
                break;
            }
        }
        let on_b: Vec<Option<Downed>> = strikes
            .iter()
            .filter(|strike| strike.ship == B)
            .map(|strike| strike.downed)
            .collect();
        assert!(!on_b.is_empty());
        assert!(
            on_b.iter().all(|downed| *downed != Some(Downed::Disabled)),
            "{on_b:?}"
        );
    }

    #[test]
    fn a_held_ship_whose_armour_runs_out_breaks_up_and_is_destroyed() {
        let mut combat = Combat::default();
        let mut held = Ship::at(2, 0.0, 0.0);
        held.held_disabled = true;
        held.condition = Condition::Disabled;
        held.reserves.armor.now = 0.0;
        let mut ships = [held];
        tick(&mut combat, &mut ships, &NovaDisable);
        let events = combat.take_events();
        assert!(matches!(events[0], CombatEvent::BreakingUp { ship: B, .. }));
        assert!(matches!(events[1], CombatEvent::Destroyed { ship: B, .. }));
        assert_eq!(ships[0].condition, Condition::Destroyed);
    }

    #[test]
    fn a_ship_no_longer_held_reads_its_armour_again_with_no_event() {
        let mut combat = Combat::default();
        let mut released = Ship::at(1, 0.0, 0.0);
        released.condition = Condition::Disabled;
        let mut ships = [released];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].condition, Condition::Intact, "full armour");
        assert_eq!(combat.take_events(), []);
        ships[0].condition = Condition::Disabled;
        ships[0].reserves.armor.now = 5.0;
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].condition, Condition::Disabled, "below a third");
        assert_eq!(combat.take_events(), [], "not newly disabled");
    }

    /// A hit of `mass` through shields, which may kill, never newly
    /// disabling its ship when `cant_disable`.
    fn armour_hit(mass: f32, cant_disable: bool) -> Hit {
        Hit {
            mass,
            passes_shields: true,
            cant_disable,
            ..Hit::default()
        }
    }

    /// The armour `ship` is left with once each of `hits`, from A, has
    /// landed on it in turn in one tick, as [`NovaDisable`] says.
    fn landed_armour(mut ship: Ship, hits: &[Hit]) -> f32 {
        let mut combat = Combat::default();
        let landed = hits
            .iter()
            .map(|&hit| Landed {
                ship: ship.id,
                by: Some(A),
                weapon: None,
                at: Vec2::ZERO,
                hit,
                direct: true,
                intended: true,
                lock: false,
                weight: 0.0,
                push: Push {
                    from: Vec2::ZERO,
                    impact: 0.0,
                    beam: false,
                },
                ion: None,
            })
            .collect();
        combat.land(&mut [ship.fighter()], landed, &NovaDisable);
        ship.reserves.armor.now
    }

    /// The armour `id`, unshielded at (100, 0) with `armor` of 30 in
    /// `condition`, is left with after the first tick A's blaster hits it.
    fn armour_after_a_blaster_hit(id: ShipRef, armor: f32, condition: Condition) -> f32 {
        let mut combat = Combat::default();
        let mut struck = holding(2, 100.0, 0.0, 0.0, armor, 30.0);
        struck.id = id;
        struck.fleet = id;
        struck.condition = condition;
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster()), struck];
        for _ in 0..20 {
            tick(&mut combat, &mut ships, &NovaDisable);
            if combat.take_strikes().iter().any(|strike| strike.ship == id) {
                return ships[1].reserves.armor.now;
            }
        }
        panic!("never hit");
    }

    #[test]
    fn a_hit_truncates_the_armour_of_the_player_it_newly_disables_toward_zero() {
        let close = |now: f32, want: f32| (now - want).abs() < 1e-4;
        let player =
            |armor, condition| armour_after_a_blaster_hit(ShipRef::Player, armor, condition);
        assert_eq!(player(10.6, Condition::Intact), 6.0, "6.6, now disabled");
        let whole = player(25.6, Condition::Intact);
        assert!(close(whole, 21.6), "hit, not disabled: {whole}");
        let before = player(9.6, Condition::Disabled);
        assert!(close(before, 5.6), "disabled before: {before}");
        let npc = armour_after_a_blaster_hit(B, 10.6, Condition::Intact);
        assert!(close(npc, 6.6), "an NPC's is never truncated: {npc}");
    }

    #[test]
    fn a_cant_disable_hit_sets_an_intact_ship_it_newly_disables_back_to_the_repair_share() {
        let armour = landed_armour(Ship::at(2, 0.0, 0.0), &[armour_hit(25.0, true)]);
        assert_eq!(armour, crate::self_repair::repaired_armor(30.0, false));
    }

    #[test]
    fn a_cant_disable_hit_leaves_a_held_ship_where_it_took_it() {
        let mut held = Ship::at(2, 0.0, 0.0);
        held.held_disabled = true;
        let armour = landed_armour(held, &[armour_hit(25.0, true)]);
        assert_eq!(armour, 5.0, "held disabled at full armour, so not newly");
    }

    #[test]
    fn a_cant_disable_hit_leaves_a_ship_not_intact_where_it_took_it() {
        let mut disabled = Ship::at(2, 0.0, 0.0);
        disabled.condition = Condition::Disabled;
        disabled.reserves.armor.now = 20.0;
        let armour = landed_armour(disabled, &[armour_hit(15.0, true)]);
        assert_eq!(armour, 5.0, "disabled above a third, so not newly");
    }

    #[test]
    fn a_cant_disable_hit_leaves_a_ship_an_earlier_hit_this_tick_disabled_where_it_took_it() {
        let armour = landed_armour(
            Ship::at(2, 0.0, 0.0),
            &[armour_hit(25.0, false), armour_hit(1.0, true)],
        );
        assert_eq!(armour, 4.0, "disabled by the first hit, still intact");
    }

    #[test]
    fn shields_regenerate_to_their_most_and_stop() {
        let mut combat = Combat::default();
        let mut ship = Ship::at(1, 0.0, 0.0);
        ship.reserves.shield.now = 5.0;
        ship.reserves.armor.now = 20.0;
        ship.shield_regen = 2.0;
        let mut ships = [ship];
        let mut shields = Vec::new();
        for _ in 0..4 {
            tick(&mut combat, &mut ships, &NovaDisable);
            shields.push(ships[0].reserves.shield.now);
        }
        assert_eq!(shields, [7.0, 9.0, 10.0, 10.0]);
        ships[0].reserves.shield.now = 12.0;
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].reserves.shield.now, 12.0, "over-full, kept");
        assert_eq!(ships[0].reserves.armor.now, 20.0, "no ArmorRech, no armour");
    }

    #[test]
    fn armour_and_shields_regenerate_only_while_intact() {
        let mut combat = Combat::default();
        let mut ship = Ship::at(1, 0.0, 0.0);
        ship.reserves.shield.now = -1.0;
        ship.reserves.armor.now = 28.5;
        ship.shield_regen = 0.5;
        ship.armor_regen = 1.0;
        let mut ships = [ship];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            (ships[0].reserves.shield.now, ships[0].reserves.armor.now),
            (-0.5, 29.5)
        );
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].reserves.armor.now, 30.0, "up to its most");
        let disabling = Recording {
            disabled: true,
            ..Recording::default()
        };
        ships[0].reserves.armor.now = 5.0;
        tick(&mut combat, &mut ships, &disabling);
        tick(&mut combat, &mut ships, &disabling);
        assert_eq!(ships[0].condition, Condition::Disabled);
        assert_eq!(ships[0].reserves.armor.now, 5.0, "not while disabled");
        assert_eq!(ships[0].reserves.shield.now, 0.0, "nor shields");
        ships[0].condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
        tick(&mut combat, &mut ships, &disabling);
        assert_eq!(
            (ships[0].reserves.shield.now, ships[0].reserves.armor.now),
            (0.0, 5.0),
            "nor while breaking up"
        );
    }

    /// A 200-pixel beam lasting 3 ticks, firing every 10, doing 5 energy
    /// damage.
    fn laser() -> WeaponRecord {
        WeaponRecord {
            guidance: 0,
            count: 3,
            reload: 10,
            beam_length: 200,
            energy_dmg: 5,
            ..weapon(146)
        }
    }

    #[test]
    fn a_beam_damages_the_ship_it_touches_every_tick_it_lasts() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield = Gauge::full(100.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(laser()), target];
        let mut shields = Vec::new();
        for _ in 0..5 {
            tick(&mut combat, &mut ships, &NovaDisable);
            shields.push(ships[1].reserves.shield.now);
        }
        assert_eq!(shields, [95.0, 90.0, 85.0, 85.0, 85.0]);
        assert_eq!(combat.beams(), [], "over");
        assert_eq!(combat.shots(), []);
    }

    #[test]
    fn a_beam_truncates_the_armour_of_the_player_it_newly_disables_toward_zero() {
        let beamed = |id: ShipRef| {
            let mut combat = Combat::default();
            let mut struck = holding(2, 100.0, 0.0, 0.0, 10.6, 30.0);
            struck.id = id;
            struck.fleet = id;
            let laser = WeaponRecord {
                mass_dmg: 4,
                ..laser()
            };
            let mut ships = [Ship::at(1, 0.0, 0.0).armed(laser), struck];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(combat.take_strikes().len(), 1, "{id:?}");
            ships[1].reserves.armor.now
        };
        assert_eq!(beamed(ShipRef::Player), 6.0, "6.6, now disabled");
        assert!(
            (beamed(B) - 6.6).abs() < 1e-4,
            "an NPC's is never truncated"
        );
    }

    #[test]
    fn a_beam_whose_firer_left_the_fight_still_passes_the_firers_leader() {
        let mut combat = Combat::default();
        let mut firer = Ship::at(1, 0.0, 0.0).armed(laser());
        firer.leader = Some(B);
        let mut leader = Ship::at(2, 100.0, 0.0);
        leader.reserves.shield = Gauge::full(100.0);
        let mut ships = [firer, leader];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.beams().len(), 1);
        tick(&mut combat, &mut ships[1..], &NovaDisable);
        assert_eq!(combat.beams().len(), 1, "it lasts");
        assert_eq!(
            ships[1].reserves.shield.now, 100.0,
            "passed by the ties it kept"
        );
        assert_eq!(combat.take_strikes(), []);
    }

    /// Each tick of `ticks` with `weapon` fired once from (0, 0) at a ship
    /// of 100 shield at (100, 0): the beams there after it, their fade,
    /// and the ship's shield.
    fn beam_ticks(weapon: WeaponRecord, ticks: usize) -> Vec<(Vec<u32>, f32)> {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield = Gauge::full(100.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(weapon), target];
        (0..ticks)
            .map(|_| {
                tick(&mut combat, &mut ships, &NovaDisable);
                ships[0].trigger = QUIET;
                let fades = combat.beams().iter().map(|beam| beam.fade).collect();
                (fades, ships[1].reserves.shield.now)
            })
            .collect()
    }

    #[test]
    fn a_beam_is_kept_through_its_last_hit_tick_and_ends_the_next_without_hitting() {
        assert_eq!(
            beam_ticks(laser(), 4),
            [
                (vec![0], 95.0),
                (vec![0], 90.0),
                (vec![0], 85.0),
                (vec![], 85.0)
            ]
        );
    }

    #[test]
    fn a_beam_strikes_every_tick_of_its_fade() {
        let fading = WeaponRecord {
            decay: 1,
            falloff: 13,
            ..laser()
        };
        assert_eq!(
            beam_ticks(fading, 6),
            [
                (vec![0], 95.0),
                (vec![0], 90.0),
                (vec![0], 85.0),
                (vec![1], 80.0),
                (vec![2], 75.0),
                (vec![], 75.0)
            ]
        );
    }

    /// A beam of `Count` 3 and `Decay` 1 at `Falloff` 10 lives 3 + (15 −
    /// 10) = 8 ticks. Fired every 4 with the trigger held, a new beam
    /// comes while the last is in its fade, so two are live from the
    /// second on, and each hits every tick it lives.
    #[test]
    fn a_fast_firing_fading_beams_overlap() {
        let pulsing = WeaponRecord {
            decay: 1,
            falloff: 10,
            reload: 4,
            ..laser()
        };
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield = Gauge::full(1000.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(pulsing), target];
        let mut ticks = Vec::new();
        for _ in 0..13 {
            let before = ships[1].reserves.shield.now;
            tick(&mut combat, &mut ships, &NovaDisable);
            let mut fades: Vec<u32> = combat.beams().iter().map(|beam| beam.fade).collect();
            fades.sort_unstable();
            ticks.push((fades, before - ships[1].reserves.shield.now));
        }
        let expected: Vec<(Vec<u32>, f32)> = vec![
            (vec![0], 5.0),
            (vec![0], 5.0),
            (vec![0], 5.0),
            (vec![1], 5.0),
            (vec![0, 2], 10.0),
            (vec![0, 3], 10.0),
            (vec![0, 4], 10.0),
            (vec![1, 5], 10.0),
            (vec![0, 2], 10.0),
            (vec![0, 3], 10.0),
            (vec![0, 4], 10.0),
            (vec![1, 5], 10.0),
            (vec![0, 2], 10.0),
        ];
        assert_eq!(ticks, expected);
    }

    /// The ships `combat` has the laser of a ship at (0, 0) facing
    /// `heading` hit on its first tick, among `ships`.
    fn beamed(mut combat: Combat, heading: f32, ships: Vec<Ship>) -> Vec<ShipRef> {
        let firer = Ship::at(1, 0.0, 0.0).facing(heading).armed(laser());
        let mut ships: Vec<Ship> = std::iter::once(firer).chain(ships).collect();
        tick(&mut combat, &mut ships, &NovaDisable);
        combat
            .take_strikes()
            .iter()
            .map(|strike| strike.ship)
            .collect()
    }

    /// NPC `id` at (`x`, `y`), `width` pixels wide or of no sprite, its
    /// hit radius a third of that.
    fn wide(id: u32, x: f32, y: f32, width: Option<f32>) -> Ship {
        let mut ship = Ship::at(id, x, y);
        ship.hull.sprite_size = width;
        ship.hull.hit_radius = width.map_or(ship.hull.hit_radius, |width| width * 0.33);
        ship
    }

    #[test]
    fn a_ship_of_no_sprite_reads_75_wide_to_a_beams_cone() {
        // 200 + trunc(49 / 2), and 200 + trunc(31 / 2).
        let reached =
            |x, width| !beamed(Combat::default(), 90.0, vec![wide(2, x, 0.0, width)]).is_empty();
        assert!(reached(224.0, None) && !reached(225.0, None));
        assert!(reached(215.0, Some(48.0)) && !reached(216.0, Some(48.0)));
    }

    #[test]
    fn the_beam_hit_test_and_quirk_keys_change_which_ship_a_beam_hits() {
        let big_and_small = || {
            vec![
                wide(2, 60.0, 50.0, Some(200.0)),
                wide(3, 40.0, 0.0, Some(20.0)),
            ]
        };
        assert_eq!(
            beamed(Combat::default(), 90.0, big_and_small()),
            [ShipRef::Npc(NpcId(3))],
            "the cone's nearest centre"
        );
        let mut segment = Combat::default();
        segment.set_beam_hit_test(RuleSource::Bible);
        assert_eq!(
            beamed(segment, 90.0, big_and_small()),
            [ShipRef::Npc(NpcId(2))],
            "the segment's first contact"
        );
        // Bearing 358 from the firer, which faces 2.
        let at_358 = || vec![wide(2, -4.0, -100.0, None)];
        assert_eq!(
            beamed(Combat::default(), 2.0, at_358()),
            [ShipRef::Npc(NpcId(2))]
        );
        let mut quirks = Combat::default();
        quirks.set_beam_cone_quirks(RuleSource::Engine);
        assert_eq!(beamed(quirks, 2.0, at_358()), [], "no wrap");
        let mut fixed = Combat::default();
        fixed.set_beam_cone_quirks(RuleSource::Bible);
        assert_eq!(fixed, Combat::default());
        let mut cone = Combat::default();
        cone.set_beam_hit_test(RuleSource::Engine);
        assert_eq!(cone, Combat::default());
    }

    #[test]
    fn a_beam_explodes_where_it_meets_the_ship_every_tick_it_touches_it_and_is_drawn_to_there() {
        let flamer = WeaponRecord {
            explod_type: 0,
            ..laser()
        };
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield = Gauge::full(100.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(flamer), target];
        let mut explosions = Vec::new();
        let mut ends = Vec::new();
        for _ in 0..5 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ends.extend(combat.beams().iter().map(|beam| beam.end));
            explosions.push(
                combat
                    .take_events()
                    .into_iter()
                    .filter_map(|event| match event {
                        CombatEvent::Exploded { at, explosion, .. } => Some((at, explosion)),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            );
        }
        let boom = Explosion {
            boom: BoomId(128),
            extra: false,
        };
        // 100 less a fifth of a placeholder hull's 75 pixels.
        let edge = Vec2::new(85.0, 0.0);
        for (tick_no, exploded) in explosions.iter().enumerate() {
            if tick_no < 3 {
                assert_eq!(exploded.len(), 1, "tick {tick_no}: {exploded:?}");
                let (at, explosion) = exploded[0];
                assert!((at - edge).length() < 1e-3, "tick {tick_no}: {at:?}");
                assert_eq!(ends[tick_no], at, "drawn to it");
                assert_eq!(explosion, boom);
            } else {
                assert_eq!(exploded, &[], "tick {tick_no}: the beam is over");
            }
        }
    }

    #[test]
    fn a_beam_follows_its_firer_and_outlives_its_disabling() {
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(laser())];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].state.position = Vec2::new(5.0, 6.0);
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.beams()[0].start, Vec2::new(5.0, 6.0));
        ships[0].condition = Condition::Disabled;
        ships[0].state.position = Vec2::new(7.0, 8.0);
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            combat.beams()[0].start,
            Vec2::new(7.0, 8.0),
            "its firer disabled"
        );
    }

    #[test]
    fn a_beam_outlives_its_breaking_up_firer() {
        let long = WeaponRecord {
            count: 10,
            ..laser()
        };
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(long)];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
        ships[0].condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
        for x in [10.0, 20.0] {
            ships[0].state.position = Vec2::new(x, 0.0);
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(combat.beams()[0].start, Vec2::new(x, 0.0), "followed");
        }
    }

    /// Each tick of `ticks` after a beam of 4 ticks fired from (0, 0)
    /// facing right at a ship of 100 shield at (100, 0), its firer then
    /// left at `left`, which may move it or take it away: the beams'
    /// starts and ends, and the ship's shield.
    fn beam_left(left: impl Fn(&mut [Ship; 2]) -> usize) -> Vec<(Vec<(Vec2, Vec2)>, f32)> {
        let four = WeaponRecord {
            count: 4,
            ..laser()
        };
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield = Gauge::full(100.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(four), target];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
        let from = left(&mut ships);
        (0..4)
            .map(|_| {
                tick(&mut combat, &mut ships[from..], &NovaDisable);
                let beams = combat
                    .beams()
                    .iter()
                    .map(|beam| {
                        (
                            beam.start,
                            Vec2::new(beam.end.x.round(), beam.end.y.round()),
                        )
                    })
                    .collect();
                (beams, ships[1].reserves.shield.now)
            })
            .collect()
    }

    #[test]
    fn a_beam_whose_firer_is_destroyed_or_gone_stays_where_it_was_until_its_life_ends() {
        let there = vec![(Vec2::ZERO, Vec2::new(85.0, 0.0))];
        let stays = [
            (there.clone(), 90.0),
            (there.clone(), 85.0),
            (there, 80.0),
            (Vec::new(), 80.0),
        ];
        let destroyed = beam_left(|ships| {
            ships[0].condition = Condition::Destroyed;
            ships[0].state.position = Vec2::new(0.0, 50.0);
            ships[0].state.heading = 180.0;
            0
        });
        assert_eq!(destroyed, stays, "destroyed");
        assert_eq!(beam_left(|_| 1), stays, "gone from the fight");
    }

    #[test]
    fn a_beam_jitters_on_the_fights_chance() {
        let shaky = WeaponRecord {
            inaccuracy: 3,
            ..laser()
        };
        let mut combat = Combat::default();
        tick(
            &mut combat,
            &mut [Ship::at(1, 0.0, 0.0).armed(shaky)],
            &NovaDisable,
        );
        // `NeverFires` draws the last of `Rand(6)`: 5 - 3.
        assert_eq!(combat.beams()[0].heading, 92.0);
    }

    const DOUBLE_TIME: GamePass = GamePass::Advance { double_time: true };

    /// The laser's firer and a ship 100 pixels ahead of it holding 100
    /// shield.
    fn laser_and_target() -> [Ship; 2] {
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield = Gauge::full(100.0);
        [Ship::at(1, 0.0, 0.0).armed(laser()), target]
    }

    #[test]
    fn on_the_second_pass_a_beam_ages_but_neither_follows_nor_hits() {
        let mut combat = Combat::default();
        let mut ships = laser_and_target();
        tick_on(&mut combat, &mut ships, DOUBLE_TIME);
        assert_eq!(ships[1].reserves.shield.now, 90.0);
        ships[0].state.position = Vec2::new(5.0, 6.0);
        combat.take_strikes();
        tick_on(&mut combat, &mut ships, GamePass::Again);
        let beam = combat.beams()[0];
        assert_eq!(beam.start, Vec2::ZERO, "not moved onto its firer");
        assert_eq!(beam.life, 1, "aged");
        assert_eq!(ships[1].reserves.shield.now, 90.0, "no hit");
        assert_eq!(combat.take_strikes(), []);
        tick_on(&mut combat, &mut ships, DOUBLE_TIME);
        assert_eq!(ships[1].reserves.shield.now, 80.0, "its last hit");
        assert_eq!(combat.beams()[0].life, 0, "drawn on its last hit's tick");
        tick_on(&mut combat, &mut ships, DOUBLE_TIME);
        assert_eq!(combat.beams(), [], "over after its third tick");
        assert_eq!(ships[1].reserves.shield.now, 80.0);
    }

    #[test]
    fn a_beam_fired_on_the_second_pass_hits_only_on_the_advance_pass() {
        let mut combat = Combat::default();
        let mut ships = laser_and_target();
        tick_on(&mut combat, &mut ships, GamePass::Again);
        assert_eq!(combat.beams().len(), 1, "fired");
        assert_eq!(ships[1].reserves.shield.now, 100.0, "no hit yet");
        tick_on(&mut combat, &mut ships, DOUBLE_TIME);
        assert_eq!(ships[1].reserves.shield.now, 90.0);
        tick_on(&mut combat, &mut ships, GamePass::Again);
        // The second pass spends only a life above none (@0x30371-0x30383).
        assert_eq!(
            combat.beams()[0].life,
            0,
            "its third tick, on the second pass: spent, and kept"
        );
        assert_eq!(ships[1].reserves.shield.now, 90.0);
        tick_on(&mut combat, &mut ships, DOUBLE_TIME);
        assert_eq!(combat.beams(), [], "the advance pass ends it, with no hit");
        assert_eq!(ships[1].reserves.shield.now, 90.0);
    }

    #[test]
    fn in_double_time_a_beam_does_double_damage_on_the_advance_pass() {
        let mut combat = Combat::default();
        let mut ships = laser_and_target();
        tick_on(&mut combat, &mut ships, DOUBLE_TIME);
        assert_eq!(ships[1].reserves.shield.now, 90.0, "twice its 5 energy");
        let cutter = WeaponRecord {
            mass_dmg: 3,
            ..laser()
        };
        let mut combat = Combat::default();
        let mut bare = Ship::at(2, 100.0, 0.0);
        bare.reserves.shield = Gauge::full(0.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(cutter), bare];
        tick_on(&mut combat, &mut ships, DOUBLE_TIME);
        assert_eq!(ships[1].reserves.armor.now, 24.0, "twice its 3 mass");
    }

    #[test]
    fn without_double_time_beam_damage_is_unchanged() {
        let mut combat = Combat::default();
        let mut ships = laser_and_target();
        tick_on(
            &mut combat,
            &mut ships,
            GamePass::Advance { double_time: false },
        );
        assert_eq!(ships[1].reserves.shield.now, 95.0);
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            ships[1].reserves.shield.now, 90.0,
            "a plain tick is the same"
        );
    }

    /// `rounds` ticks of a blaster duel on `pass`: each ship's reserves
    /// after each, and the events.
    fn duel(pass: GamePass, rounds: usize) -> (Vec<[Reserves; 2]>, Vec<CombatEvent>) {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()).targeting(B),
            Ship::at(2, 60.0, 0.0)
                .armed(blaster())
                .facing(270.0)
                .targeting(A),
        ];
        ships[1].shield_regen = 0.5;
        let mut seen = Vec::new();
        let mut events = Vec::new();
        for _ in 0..rounds {
            tick_on(&mut combat, &mut ships, pass);
            seen.push([ships[0].reserves, ships[1].reserves]);
            events.extend(combat.take_events());
        }
        (seen, events)
    }

    #[test]
    fn the_second_pass_fires_flies_and_hits_with_shots_as_the_advance_pass_does() {
        let (advance, events) = duel(GamePass::Advance { double_time: false }, 12);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, CombatEvent::Fired { .. })),
            "{events:?}"
        );
        assert!(advance.last().unwrap()[1].shield.now < 10.0, "{advance:?}");
        assert_eq!(duel(GamePass::Again, 12), (advance, events));
    }

    #[test]
    fn a_hit_explodes_and_blasts_the_ships_around_it() {
        let shell = WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            ..blaster()
        };
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(shell),
            Ship::at(2, 100.0, 0.0),
            Ship::at(3, 100.0, 25.0),
        ];
        let mut events = Vec::new();
        // Fired on the first tick, its sixth move, on the seventh, meets
        // the ship, and the eighth tick's hit pass tests it.
        for _ in 0..8 {
            tick(&mut combat, &mut ships, &NovaDisable);
            events.extend(combat.take_events());
            ships[0].trigger = QUIET;
        }
        let explosions: Vec<_> = events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Exploded { at, explosion, .. } => Some((at, explosion)),
                _ => None,
            })
            .collect();
        assert_eq!(explosions.len(), 1, "{events:?}");
        let (at, explosion) = explosions[0];
        assert!(
            (at.x - 84.0).abs() < 1e-3 && at.y.abs() < 1e-3,
            "at its edge: {at:?}"
        );
        assert_eq!(explosion.boom, BoomId(128));
        assert_eq!(ships[1].reserves.shield.now, 0.0, "hit directly");
        assert_eq!(ships[2].reserves.shield.now, 0.0, "blasted");
        assert_eq!(ships[2].reserves.armor.now, 26.0);
        assert_eq!(ships[0].reserves.shield.now, 10.0, "out of the blast");
        assert_eq!(combat.shots(), []);
    }

    #[test]
    fn a_ships_shots_pass_through_its_government_to_the_enemy_behind() {
        let mut combat = Combat::default();
        let mut shooter = Ship::at(1, 0.0, 0.0).armed(blaster());
        shooter.govt = Some(GovtId(140));
        let mut wingman = Ship::at(2, 100.0, 0.0);
        wingman.govt = Some(GovtId(140));
        let mut enemy = Ship::at(3, 150.0, 0.0);
        enemy.govt = Some(GovtId(141));
        let mut ships = [shooter, wingman, enemy];
        for _ in 0..12 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
        }
        assert_eq!(left(&ships[1]), (10.0, 30.0), "passed through");
        assert_ne!(left(&ships[2]), (10.0, 30.0), "the enemy behind is hit");
        let struck: Vec<ShipRef> = combat
            .take_strikes()
            .iter()
            .map(|strike| strike.ship)
            .collect();
        assert_eq!(struck, [C]);
    }

    #[test]
    fn the_shots_in_flight_are_changed_in_place_through_shots_mut() {
        let mut combat = launched_by(Ship::at(1, 0.0, 0.0), blaster(), vec![]);
        assert_eq!(combat.shots_mut().len(), 1);
        combat.shots_mut()[0].age = 9;
        assert_eq!(combat.shots()[0].age, 9);
    }

    #[test]
    fn the_pass_through_reading_set_decides_whether_an_escorts_shots_pass_as_the_players() {
        let govts = Governments::new([crate::catalog::GovtRecord {
            flags: crate::govt::PLAYER_SHOTS_PASS,
            ..nova_sim_catalog::fixture::govt(142)
        }]);
        let struck = |pass_through: PassThrough| {
            let mut combat = Combat::default();
            combat.set_pass_through(pass_through);
            let mut escort = Ship::at(1, 0.0, 0.0).armed(blaster());
            escort.leader = Some(ShipRef::Player);
            let mut passed = Ship::at(2, 100.0, 0.0);
            passed.govt = Some(GovtId(142));
            let mut ships = [escort, passed];
            for _ in 0..12 {
                let rules = Rules::default();
                tick_among(&mut combat, &mut ships, &Arsenal::default(), &govts, rules);
                ships[0].trigger = QUIET;
            }
            combat.take_strikes().len()
        };
        assert_eq!(struck(PassThrough::PlayerSide), 0, "the player's side");
        assert_eq!(struck(PassThrough::PlayerOnly), 1, "the player alone");
    }

    #[test]
    fn a_shot_on_a_breaking_up_ship_strikes_without_downing_it_and_its_countdown_goes_on() {
        let mut combat = Combat::default();
        let mut hulk = Ship::at(2, 100.0, 0.0);
        hulk.reserves.armor.now = 0.0;
        hulk.condition = Condition::Dying {
            ticks_left: 50,
            starting: false,
        };
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster()), hulk];
        let mut strikes = Vec::new();
        for _ in 0..8 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
            strikes.extend(combat.take_strikes());
        }
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        assert_eq!((strikes[0].ship, strikes[0].by), (B, Some(A)));
        assert_eq!(strikes[0].downed, None, "it was down already");
        assert_eq!(
            strikes[0].damage, 14.0,
            "its shield, then its mass on the armour"
        );
        assert_eq!(
            ships[1].condition,
            Condition::Dying {
                ticks_left: 42,
                starting: false
            },
            "a tick off each tick, hit or not"
        );
    }

    #[test]
    fn a_blast_hurts_a_breaking_up_ship() {
        let shell = WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            ..blaster()
        };
        let mut combat = Combat::default();
        let mut dying = Ship::at(3, 100.0, 25.0);
        dying.condition = Condition::Dying {
            ticks_left: 50,
            starting: false,
        };
        let mut disabled = Ship::at(4, 100.0, -25.0);
        disabled.condition = Condition::Disabled;
        disabled.reserves.armor.now = 5.0;
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(shell),
            Ship::at(2, 100.0, 0.0),
            dying,
            disabled,
        ];
        // The sixth move, on the seventh tick, meets ship 2, tested in the
        // eighth.
        for _ in 0..8 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
        }
        assert_eq!(ships[1].reserves.shield.now, 0.0, "hit directly");
        assert_eq!(left(&ships[2]), (0.0, 26.0), "a dying ship is blasted too");
        assert_eq!(
            (ships[3].reserves.shield.now, ships[3].reserves.armor.now),
            (0.0, 1.0),
            "a disabled ship is blasted"
        );
        assert_eq!(ships[3].condition, Condition::Disabled);
    }

    #[test]
    fn a_shot_detonates_at_the_end_of_its_life_if_its_weapon_says_so() {
        let flak = |flags| WeaponRecord {
            count: 3,
            speed: 1000,
            explod_type: 2,
            blast_radius: 40,
            flags,
            ..blaster()
        };
        for (flags, blasted) in [(0x8000, true), (0, false)] {
            let mut combat = Combat::default();
            let mut ships = [
                Ship::at(1, 0.0, 0.0).armed(flak(flags)),
                Ship::at(3, 30.0, 35.0),
            ];
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
            // Its third move, on the fourth tick, spends it; its end of
            // life comes the tick after.
            for _ in 0..4 {
                tick(&mut combat, &mut ships, &NovaDisable);
            }
            assert_eq!(combat.shots(), [], "{flags:#x}");
            let exploded = combat
                .take_events()
                .into_iter()
                .any(|event| matches!(event, CombatEvent::Exploded { .. }));
            assert_eq!(exploded, blasted, "{flags:#x}");
            assert_eq!(ships[1].reserves.shield.now < 10.0, blasted, "{flags:#x}");
        }
    }

    /// `Flags2` 0x1000: a disable-only weapon, as stock 142, 201, 233 and
    /// 234.
    const DISABLE_ONLY: u16 = 0x1000;

    /// Whether `events` hold a breakup or destruction of any of `ships`.
    fn broke_up(events: &[CombatEvent], ships: &[ShipRef]) -> bool {
        events.iter().any(|event| match *event {
            CombatEvent::BreakingUp { ship, .. } | CombatEvent::Destroyed { ship, .. } => {
                ships.contains(&ship)
            }
            _ => false,
        })
    }

    #[test]
    fn a_disable_only_shot_its_blast_and_a_beam_leave_their_victims_at_1_armour_never_breaking_up()
    {
        let shell = WeaponRecord {
            blast_radius: 30,
            mass_dmg: 50,
            energy_dmg: 50,
            flags2: DISABLE_ONLY,
            ..blaster()
        };
        let flak = WeaponRecord {
            count: 3,
            speed: 1000,
            blast_radius: 40,
            mass_dmg: 50,
            energy_dmg: 50,
            flags: 0x8000,
            flags2: DISABLE_ONLY,
            ..blaster()
        };
        let ion = WeaponRecord {
            mass_dmg: 50,
            energy_dmg: 50,
            flags2: DISABLE_ONLY,
            ..laser()
        };
        let cases = [
            (
                "a direct shot and its blast",
                shell,
                vec![(2, 100.0, 0.0), (3, 100.0, 25.0)],
            ),
            ("an expiry detonation", flak, vec![(3, 30.0, 35.0)]),
            ("a beam", ion, vec![(2, 100.0, 0.0)]),
        ];
        for (case, record, victims) in cases {
            let mut combat = Combat::default();
            let mut ships = vec![Ship::at(1, 0.0, 0.0).armed(record)];
            ships.extend(victims.iter().map(|&(id, x, y)| Ship::at(id, x, y)));
            let mut events = Vec::new();
            for _ in 0..40 {
                tick(&mut combat, &mut ships, &NovaDisable);
                events.extend(combat.take_events());
            }
            for victim in &ships[1..] {
                assert_eq!(victim.reserves.armor.now, damage::SPARED_ARMOR, "{case}");
                assert_eq!(victim.condition, Condition::Disabled, "{case}");
            }
            let ids: Vec<ShipRef> = victims
                .iter()
                .map(|&(id, ..)| ShipRef::Npc(NpcId(id)))
                .collect();
            assert!(!broke_up(&events, &ids), "{case}: {events:?}");
        }
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(WeaponRecord { flags2: 0, ..shell }),
            Ship::at(2, 100.0, 0.0),
            Ship::at(3, 100.0, 25.0),
        ];
        let mut events = Vec::new();
        for _ in 0..40 {
            tick(&mut combat, &mut ships, &NovaDisable);
            events.extend(combat.take_events());
        }
        assert!(broke_up(&events, &[B]), "without the flag it kills");
        assert!(broke_up(&events, &[C]), "and its blast too");
    }

    #[test]
    fn a_blast_strike_is_neither_direct_nor_intended_and_a_direct_hit_is_direct() {
        let shell = WeaponRecord {
            blast_radius: 30,
            ..blaster()
        };
        let struck = |target: ShipRef| {
            let mut combat = Combat::default();
            let mut ships = [
                Ship::at(1, 0.0, 0.0).armed(shell).targeting(target),
                Ship::at(2, 100.0, 0.0),
                Ship::at(3, 100.0, 25.0),
                Ship::at(4, 0.0, 500.0),
            ];
            let mut strikes = Vec::new();
            for _ in 0..8 {
                tick(&mut combat, &mut ships, &NovaDisable);
                ships[0].trigger = QUIET;
                strikes.extend(combat.take_strikes());
            }
            strikes
                .iter()
                .map(|strike| (strike.ship, strike.direct, strike.intended))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            struck(B),
            [(B, true, true), (C, false, false)],
            "the aimed-at ship hit, and a bystander blasted"
        );
        let far = ShipRef::Npc(NpcId(4));
        assert_eq!(
            struck(far),
            [(B, true, false), (C, false, false)],
            "a ship crossing the shot's path is hit directly, but not intended"
        );
        assert_eq!(
            struck(C),
            [(B, true, false), (C, false, false)],
            "blasted, so not intended"
        );
    }

    #[test]
    fn an_expiry_detonation_strike_is_neither_direct_nor_intended() {
        let flak = WeaponRecord {
            count: 3,
            speed: 1000,
            blast_radius: 40,
            flags: 0x8000,
            ..blaster()
        };
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(flak).targeting(C),
            Ship::at(3, 30.0, 35.0),
        ];
        let mut strikes = Vec::new();
        for _ in 0..5 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
            strikes.extend(combat.take_strikes());
        }
        let flags: Vec<_> = strikes
            .iter()
            .map(|strike| (strike.ship, strike.direct, strike.intended))
            .collect();
        assert_eq!(flags, [(C, false, false)]);
    }

    #[test]
    fn a_beam_hit_is_intended_only_on_its_firers_target() {
        let far = ShipRef::Npc(NpcId(4));
        // Each tick's firer target, the beam fired on the first.
        let intended = |targets: [Option<ShipRef>; 3]| {
            let mut combat = Combat::default();
            let mut ships = [
                Ship::at(1, 0.0, 0.0).armed(laser()),
                Ship::at(2, 100.0, 0.0),
                Ship::at(4, 0.0, 500.0),
            ];
            ships[1].reserves.shield = Gauge::full(100.0);
            let mut seen = Vec::new();
            for target in targets {
                ships[0].target = target;
                tick(&mut combat, &mut ships, &NovaDisable);
                ships[0].trigger = QUIET;
                let strikes = combat.take_strikes();
                assert_eq!(strikes.len(), 1, "{strikes:?}");
                assert_eq!((strikes[0].ship, strikes[0].direct), (B, true));
                seen.push(strikes[0].intended);
            }
            seen
        };
        assert_eq!(intended([Some(B); 3]), [true; 3], "on its target");
        assert_eq!(
            intended([Some(far); 3]),
            [false; 3],
            "across another's path"
        );
        assert_eq!(
            intended([Some(far), Some(B), Some(B)]),
            [false; 3],
            "the beam's own target is the one it was fired at (beam +0x18)"
        );
        assert_eq!(
            intended([None, Some(B), None]),
            [false, true, false],
            "a beam fired at nothing reads its firer's target at each hit"
        );
    }

    #[test]
    fn diagnostics_are_taken_once() {
        let exclusive = WeaponRecord {
            flags3: 0x0020,
            ..blaster()
        };
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(exclusive)];
        for _ in 0..3 {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        assert_eq!(combat.take_diagnostics().len(), 1);
        assert_eq!(combat.take_diagnostics(), []);
    }

    // Guided weapons and turrets.

    impl Ship {
        /// The ship facing `heading`.
        fn facing(self, heading: f32) -> Self {
            Self {
                state: ShipState {
                    heading,
                    ..self.state
                },
                ..self
            }
        }

        /// The ship targeting `target`.
        fn targeting(self, target: ShipRef) -> Self {
            Self {
                target: Some(target),
                ..self
            }
        }
    }

    /// A turret of `guidance` firing every 5 ticks, 20 pixels a tick for
    /// 30, doing 5 mass and 10 energy damage.
    fn turret(guidance: i16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            reload: 5,
            count: 30,
            speed: 2000,
            mass_dmg: 5,
            energy_dmg: 10,
            ..weapon(130)
        }
    }

    #[test]
    fn a_turret_destroys_a_target_behind_its_firer() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(turret(4)).targeting(B),
            Ship::at(2, -150.0, 0.0),
        ];
        for _ in 0..60 {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        assert_eq!(ships[1].condition, Condition::Destroyed);
        let mut idle = [
            Ship::at(1, 0.0, 0.0).armed(turret(4)),
            Ship::at(2, -150.0, 0.0),
        ];
        let mut combat = Combat::default();
        tick(&mut combat, &mut idle, &NovaDisable);
        assert_eq!(combat.take_events(), [], "no target, no fire");
        assert_eq!(combat.shots(), []);
    }

    #[test]
    fn a_front_quadrant_turret_without_a_target_fires_along_the_heading() {
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(turret(7)).facing(30.0)];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.shots().len(), 1);
        assert_eq!(combat.shots()[0].heading, 30.0);
        assert_eq!(combat.shots()[0].target, None);
    }

    #[test]
    fn a_turreted_beam_tracks_a_breaking_up_target_and_ends_with_its_destruction() {
        let beam = WeaponRecord {
            guidance: 3,
            count: 10,
            beam_length: 300,
            energy_dmg: 1,
            ..weapon(142)
        };
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(beam).targeting(B),
            Ship::at(2, 0.0, 100.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
        ships[1].state.position = Vec2::new(100.0, 0.0);
        tick(&mut combat, &mut ships, &NovaDisable);
        let end = combat.beams()[0].end;
        assert!(
            (end - Vec2::new(85.0, 0.0)).length() < 1e-3,
            "at its hit: {end:?}"
        );
        assert_eq!(ships[1].reserves.shield.now, 8.0, "hit both ticks");
        ships[1].condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.beams().len(), 1, "its target breaking up");
        assert_eq!(ships[1].reserves.shield.now, 7.0, "and hit");
        ships[1].condition = Condition::Destroyed;
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.beams(), [], "its target destroyed");
    }

    /// A missile 10 pixels a tick for 100 ticks, turning 7 degrees a tick,
    /// doing 20 mass and 10 energy damage, standing 4 of point defence.
    fn missile() -> WeaponRecord {
        WeaponRecord {
            guidance: 1,
            reload: 1000,
            count: 100,
            speed: 1000,
            guided_turn: 70,
            durability: 4,
            mass_dmg: 20,
            energy_dmg: 10,
            ..weapon(134)
        }
    }

    #[test]
    fn a_homing_missile_chases_and_hits_a_crossing_target() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0)
                .armed(missile())
                .facing(0.0)
                .targeting(B),
            Ship::at(2, 0.0, -300.0),
        ];
        let mut hit = false;
        for _ in 0..100 {
            ships[1].state.velocity = Vec2::new(3.0, 0.0);
            ships[1].state.position = ships[1].state.position + ships[1].state.velocity;
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
            if ships[1].reserves.shield.now < 10.0 {
                hit = true;
                break;
            }
        }
        assert!(hit, "{:?}", combat.shots());
        assert_eq!(combat.shots(), [], "spent");
    }

    #[test]
    fn a_missile_whose_target_is_destroyed_flies_on_and_hits_nothing() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0)
                .armed(missile())
                .facing(90.0)
                .targeting(B),
            Ship::at(2, 0.0, 500.0),
            Ship::at(3, 400.0, 0.0),
        ];
        for _ in 0..10 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
        }
        assert_eq!(combat.shots()[0].target, Some(B), "fired at B");
        ships[1].condition = Condition::Destroyed;
        let mut flown = 0;
        while !combat.shots().is_empty() {
            tick(&mut combat, &mut ships, &NovaDisable);
            flown += 1;
        }
        // Fired on the first of the ten ticks, it first moved on the
        // second (`_HandleShot` before `_HandleShip`).
        assert_eq!(flown, 92, "flew its life, and ended the tick after");
        assert_eq!(
            ships[2].reserves,
            Reserves::full(10.0, 30.0, 100.0),
            "flown through"
        );
    }

    #[test]
    fn a_shot_whose_ship_was_destroyed_is_lost_from_the_next_tick() {
        // B, far off, is destroyed at the end of the tick the shot is
        // fired: its death timer is at the explosion band's edge.
        let mut combat = Combat::default();
        let mut wreck = Ship::at(2, 0.0, 900.0);
        wreck.condition = Condition::Dying {
            ticks_left: EXPLOSION_BAND + 1,
            starting: false,
        };
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster()).targeting(B), wreck];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
        assert_eq!(ships[1].condition, Condition::Destroyed);
        assert_eq!(
            (combat.shots()[0].jam, combat.shots()[0].aimed_at),
            (Jam::Clear, Some(B)),
            "clear through the tick B was destroyed"
        );
        let was = combat.shots()[0].position;
        tick(&mut combat, &mut ships, &NovaDisable);
        let shot = combat.shots()[0];
        assert_eq!((shot.jam, shot.aimed_at), (Jam::Lost, None));
        assert!((shot.position - was).length() > 1.0, "lost, then moved");
    }

    #[test]
    fn a_volley_at_a_ship_that_left_the_fight_passes_through_the_ships_behind_it() {
        let mut combat = Combat::default();
        let mut ships = vec![
            Ship::at(1, 0.0, 0.0).armed(blaster()).targeting(B),
            Ship::at(2, 0.0, 900.0),
            Ship::at(3, 100.0, 0.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
        // B lands or jumps: it is no longer among the fighters.
        ships.remove(1);
        let mut strikes = Vec::new();
        while !combat.shots().is_empty() {
            tick(&mut combat, &mut ships, &NovaDisable);
            strikes.extend(combat.take_strikes());
        }
        assert_eq!(strikes, [], "passed through C");
        assert_eq!(ships[1].reserves, Reserves::full(10.0, 30.0, 100.0));
    }

    #[test]
    fn a_homing_missile_steers_into_a_breaking_up_target_and_hits_it() {
        // Fired at B, which then starts breaking up.
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0)
                .armed(missile())
                .facing(90.0)
                .targeting(B),
            Ship::at(2, 0.0, -300.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
        ships[1].condition = Condition::Dying {
            ticks_left: 500,
            starting: false,
        };
        let mut strikes = Vec::new();
        for _ in 0..100 {
            tick(&mut combat, &mut ships, &NovaDisable);
            strikes.extend(combat.take_strikes());
            if !strikes.is_empty() {
                break;
            }
        }
        assert_eq!(strikes.len(), 1, "{:?}", combat.shots());
        assert_eq!((strikes[0].ship, strikes[0].downed), (B, None));
        assert!(matches!(ships[1].condition, Condition::Dying { .. }));
    }

    #[test]
    fn a_lost_shot_still_detonates_and_blasts_at_the_end_of_its_life() {
        let flak = WeaponRecord {
            count: 3,
            speed: 1000,
            explod_type: 2,
            blast_radius: 40,
            flags: 0x8000,
            ..blaster()
        };
        let mut combat = Combat::default();
        let mut ships = vec![
            Ship::at(1, 0.0, 0.0).armed(flak).targeting(B),
            Ship::at(2, 0.0, 900.0),
            Ship::at(3, 30.0, 35.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
        ships.remove(1);
        let mut strikes = Vec::new();
        let mut events = Vec::new();
        // Three moves, the first the tick after it is fired, then its end
        // of life.
        for _ in 0..4 {
            tick(&mut combat, &mut ships, &NovaDisable);
            strikes.extend(combat.take_strikes());
            events.extend(combat.take_events());
        }
        assert_eq!(combat.shots(), []);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, CombatEvent::Exploded { .. })),
            "{events:?}"
        );
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        assert_eq!((strikes[0].ship, strikes[0].direct), (C, false));
    }

    #[test]
    fn an_unguided_shells_expiry_submunitions_are_aimed_at_the_ship_its_damage_was() {
        let (shell, arsenal) = cluster(0);
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(shell).targeting(C),
            Ship::at(3, 0.0, 900.0),
        ];
        for _ in 0..5 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            ships[0].trigger = QUIET;
        }
        let subs = sub_shots(&combat);
        assert_eq!(subs.len(), 2);
        for sub in subs {
            assert_eq!((sub.target, sub.aimed_at), (Some(C), Some(C)));
        }
    }

    #[test]
    fn a_lost_shots_expiry_submunitions_are_aimed_at_nothing() {
        let (shell, arsenal) = cluster(0);
        let mut combat = Combat::default();
        let mut ships = vec![
            Ship::at(1, 0.0, 0.0).armed(shell).targeting(B),
            Ship::at(2, 0.0, 900.0),
        ];
        tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        ships[0].trigger = QUIET;
        ships.remove(1);
        for _ in 0..4 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        }
        let subs = sub_shots(&combat);
        assert_eq!(subs.len(), 2);
        for sub in subs {
            assert_eq!((sub.target, sub.aimed_at), (None, None));
        }
    }

    // The missile lock (`_HandleShot` @0x35627-0x35863).

    /// The player at the centre, facing right, armed with `record`,
    /// targeting B.
    fn player_with(record: WeaponRecord) -> Ship {
        Ship {
            id: ShipRef::Player,
            fleet: ShipRef::Player,
            slot: Slot::Player,
            ..Ship::at(0, 0.0, 0.0)
        }
        .armed(record)
        .facing(90.0)
        .targeting(B)
    }

    /// The strikes over `ticks` ticks after `ships[0]` fires once, each
    /// with the tick it came on, the first tick being 1.
    fn strikes_over(ships: &mut [Ship], arsenal: &Arsenal, ticks: u32) -> Vec<(u32, Strike)> {
        let mut combat = Combat::default();
        let mut strikes = Vec::new();
        for tick in 1..=ticks {
            tick_with(&mut combat, ships, arsenal, Rules::default());
            ships[0].trigger = QUIET;
            strikes.extend(
                combat
                    .take_strikes()
                    .into_iter()
                    .map(|strike| (tick, strike)),
            );
        }
        strikes
    }

    /// B, far off to the right, where no missile reaches it in 40 ticks.
    fn far_b() -> Ship {
        Ship::at(2, 5000.0, 0.0)
    }

    #[test]
    fn a_players_homing_missile_strikes_its_npc_target_once_with_no_damage_on_the_tick_its_age_passes_30()
     {
        let mut ships = [player_with(missile()), far_b()];
        let strikes = strikes_over(&mut ships, &Arsenal::default(), 40);
        assert_eq!(
            strikes,
            [(
                31,
                Strike {
                    ship: B,
                    by: Some(ShipRef::Player),
                    damage: 0.0,
                    downed: None,
                    direct: true,
                    intended: true,
                    lock: true,
                    nominal: Nominal {
                        mass: 0.0,
                        energy: 0.0,
                    },
                    weight: 1000.0,
                    answered: false,
                }
            )]
        );
        assert_eq!(ships[1].reserves, Reserves::full(10.0, 30.0, 100.0));
        assert_eq!(projectile::LOCK_AGE, 30);
    }

    #[test]
    fn a_missile_lock_needs_a_player_homing_shot_still_locked_on_an_npc_outside_the_players_fleet()
    {
        let locks = |mut ships: Vec<Ship>| {
            strikes_over(&mut ships, &Arsenal::default(), 40)
                .iter()
                .filter(|(_, strike)| strike.lock)
                .count()
        };
        assert_eq!(locks(vec![player_with(missile()), far_b()]), 1);
        let npc = Ship::at(1, 0.0, 0.0)
            .armed(missile())
            .facing(90.0)
            .targeting(B);
        assert_eq!(locks(vec![npc, far_b()]), 0, "an NPC firer");
        let unguided = WeaponRecord {
            guidance: -1,
            ..missile()
        };
        assert_eq!(locks(vec![player_with(unguided), far_b()]), 0, "unguided");
        let turret = WeaponRecord {
            guidance: 4,
            ..missile()
        };
        assert_eq!(locks(vec![player_with(turret), far_b()]), 0, "a turret");
        let escort = Ship {
            leader: Some(ShipRef::Player),
            ..far_b()
        };
        assert_eq!(
            locks(vec![player_with(missile()), escort]),
            0,
            "led by the player"
        );
        let fighter = Ship {
            leader: Some(C),
            ..far_b()
        };
        let carrier = Ship {
            leader: Some(ShipRef::Player),
            ..Ship::at(3, 0.0, 5000.0)
        };
        assert_eq!(
            locks(vec![player_with(missile()), fighter, carrier]),
            0,
            "an escort's fighter, its root the player"
        );
        let other_fleet = Ship {
            leader: Some(C),
            ..far_b()
        };
        assert_eq!(
            locks(vec![
                player_with(missile()),
                other_fleet,
                Ship::at(3, 0.0, 5000.0)
            ]),
            1,
            "a fighter of another ship"
        );
        assert_eq!(
            locks(vec![player_with(missile()), pod(far_b(), ESCAPE_POD_SHIP)]),
            0,
            "the escape pod"
        );
    }

    #[test]
    fn a_lost_missile_never_locks() {
        let mut combat = Combat::default();
        let mut ships = vec![player_with(missile()), far_b(), Ship::at(3, 0.0, 5000.0)];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
        ships.remove(1);
        let mut strikes = Vec::new();
        for _ in 0..40 {
            tick(&mut combat, &mut ships, &NovaDisable);
            strikes.extend(combat.take_strikes());
        }
        assert_eq!(strikes, []);
    }

    #[test]
    fn a_missile_of_lifetime_30_or_31_locks_on_its_last_move_and_one_of_29_never_does() {
        // `_HandleShot` puts a spent shot's life at -1 before the second
        // test, so the last move counts its age as its Count + 1.
        for (count, locked) in [(29, None), (30, Some(30)), (31, Some(31)), (32, Some(31))] {
            let short = WeaponRecord { count, ..missile() };
            let mut ships = [player_with(short), far_b()];
            let strikes = strikes_over(&mut ships, &Arsenal::default(), 40);
            let at: Vec<u32> = strikes
                .iter()
                .filter(|(_, strike)| strike.lock)
                .map(|&(tick, _)| tick)
                .collect();
            assert_eq!(at, Vec::from_iter(locked), "Count {count}");
        }
    }

    #[test]
    fn a_players_homing_submunition_locks_30_ticks_after_its_release() {
        // A shell of 3 ticks releasing one missile at its end.
        let shell = WeaponRecord {
            reload: 1000,
            count: 3,
            speed: 1000,
            sub_count: 1,
            sub_type: Some(WeaponId(134)),
            ..weapon(182)
        };
        let arsenal = Arsenal::new(&[shell, missile()], Vec::new());
        let mut ships = [player_with(shell), far_b()];
        let strikes = strikes_over(&mut ships, &arsenal, 50);
        let at: Vec<u32> = strikes
            .iter()
            .filter(|(_, strike)| strike.lock)
            .map(|&(tick, _)| tick)
            .collect();
        // The shell's third move spends it; tick 4's hit pass releases the
        // missile, which takes its first move that tick, as `_HandleShot`
        // moves what `_CheckShotProximities` released in the same frame,
        // and locks on its 31st.
        assert_eq!(at, [4 + 30]);
    }

    #[test]
    fn a_real_hit_lands_before_the_ticks_lock_and_carries_the_downing() {
        // B, at 1 armour, sits where a blaster shot's last move ended: the
        // lock's tick's hit pass tests it, before the shots fly and the
        // missile locks. Its breakup lasts, so the missile still has it
        // to lock on.
        let mut ships = [player_with(missile()), far_b()];
        ships[1].reserves.shield.now = 0.0;
        ships[1].reserves.armor.now = 1.0;
        ships[1].hull.death_delay = 10;
        let mut combat = Combat::default();
        for _ in 0..30 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
        }
        combat.take_strikes();
        combat.put_in_flight(Shot {
            id: ShotId(99),
            position: Vec2::new(5000.0, 0.0),
            velocity: Vec2::new(10.0, 0.0),
            age: 1,
            moved_from: Some(Vec2::new(4990.0, 0.0)),
            ..aimed(blaster())
        });
        tick(&mut combat, &mut ships, &NovaDisable);
        let strikes = combat.take_strikes();
        assert_eq!(strikes.len(), 2, "{strikes:?}");
        assert!(!strikes[0].lock, "{strikes:?}");
        assert_eq!(strikes[0].downed, Some(Downed::BreakingUp));
        assert!(
            strikes[1].lock && strikes[1].downed.is_none(),
            "{strikes:?}"
        );
    }

    #[test]
    fn a_ship_downed_with_only_a_lock_on_it_that_tick_carries_no_downing() {
        let mut ships = [player_with(missile()), far_b()];
        let mut combat = Combat::default();
        for _ in 0..30 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
        }
        combat.take_strikes();
        ships[1].reserves.armor.now = 0.0;
        tick(&mut combat, &mut ships, &NovaDisable);
        let strikes = combat.take_strikes();
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        assert!(strikes[0].lock);
        assert_eq!(strikes[0].downed, None);
        assert_ne!(ships[1].condition, Condition::Intact, "downed all the same");
    }

    // Decay (`_HandleShot` @0x35eb2-0x35efc, `_HandleShipHit`
    // @0x36add-0x36afd).

    /// A slug 10 pixels a tick for 50 ticks, doing 10 mass and 7 energy
    /// damage, losing a point of each every 2 ticks, with `flags`.
    fn slug(flags: u16) -> WeaponRecord {
        WeaponRecord {
            reload: 1000,
            count: 50,
            speed: 1000,
            mass_dmg: 10,
            energy_dmg: 7,
            decay: 1,
            flags,
            ..weapon(156)
        }
    }

    /// The reserves B, 95 pixels ahead, and C, at (`cx`, `cy`), are left
    /// with once A's one shot of `record` is spent, its decay as `step`
    /// says.
    fn after_slug(record: WeaponRecord, step: DecayStep, (cx, cy): (f32, f32)) -> [Reserves; 2] {
        let mut combat = Combat::default();
        combat.set_decay(step);
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(record),
            Ship::at(2, 95.0, 0.0),
            Ship::at(3, cx, cy),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
        while !combat.shots().is_empty() {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        [ships[1].reserves, ships[2].reserves]
    }

    const FAR_OFF: (f32, f32) = (0.0, 900.0);

    #[test]
    fn a_decayed_shots_direct_hit_takes_its_steps_off_mass_and_energy_each_floored_at_none() {
        // It meets B on its 8th tick: 4 steps by the original, 8 by the
        // Bible.
        let [armour, _] = after_slug(slug(0x0020), DecayStep::EveryDecayPlusOne, FAR_OFF);
        assert_eq!(
            (armour.shield.now, armour.armor.now),
            (10.0, 24.0),
            "10 - 4"
        );
        let [bible, _] = after_slug(slug(0x0020), DecayStep::EveryDecay, FAR_OFF);
        assert_eq!(bible.armor.now, 28.0, "10 - 8");
        let [shield, _] = after_slug(slug(0), DecayStep::EveryDecayPlusOne, FAR_OFF);
        assert_eq!((shield.shield.now, shield.armor.now), (7.0, 30.0), "7 - 4");
        let [gone, _] = after_slug(slug(0), DecayStep::EveryDecay, FAR_OFF);
        assert_eq!(gone.shield.now, 10.0, "7 - 8, floored at none");
        let steady = WeaponRecord {
            decay: 0,
            ..slug(0x0020)
        };
        let [whole, _] = after_slug(steady, DecayStep::EveryDecayPlusOne, FAR_OFF);
        assert_eq!(whole.armor.now, 20.0, "no decay");
    }

    #[test]
    fn a_decayed_shots_blasts_take_the_full_damage() {
        let shell = WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            ..slug(0x0020)
        };
        let [hit, blasted] = after_slug(shell, DecayStep::EveryDecayPlusOne, (95.0, 20.0));
        assert_eq!(hit.armor.now, 24.0, "the direct hit decayed");
        assert_eq!(blasted.armor.now, 20.0, "the blast whole");
        // Spent at (60, 0) on its 6th tick, 3 steps on.
        let flak = WeaponRecord {
            count: 6,
            flags: 0x8020,
            ..shell
        };
        let [missed, detonated] = after_slug(flak, DecayStep::EveryDecayPlusOne, (60.0, 25.0));
        assert_eq!(missed.armor.now, 30.0, "out of its reach");
        assert_eq!(detonated.armor.now, 20.0, "the expiry blast whole");
    }

    // One shot at a time (`Flags3` 0x0004, `_HandleShot` @0x35c09-0x35c42).

    /// The ticks of the first `ticks` on which `ships[0]` fires.
    fn fire_ticks(ships: &mut [Ship], ticks: u32) -> Vec<u32> {
        let mut combat = Combat::default();
        (0..ticks)
            .filter(|_| {
                tick(&mut combat, ships, &NovaDisable);
                fired(&combat.take_events(), ships[0].id) > 0
            })
            .collect()
    }

    #[test]
    fn a_one_at_a_time_weapon_fires_again_only_its_reload_after_its_shot_is_gone() {
        // Reload 3, 10 pixels a tick for 10 ticks.
        let single = |flags3| WeaponRecord {
            reload: 3,
            count: 10,
            speed: 1000,
            flags3,
            ..weapon(128)
        };
        let at = |x| Ship::at(2, x, 0.0);
        let firer = |flags3| Ship::at(1, 0.0, 0.0).armed(single(flags3));
        assert_eq!(
            fire_ticks(&mut [firer(0), at(900.0)], 14),
            [0, 3, 6, 9, 12],
            "every Reload ticks"
        );
        // An NPC's one-at-a-time shot: see the `tick_phases` tests, as its
        // reload counts down after `_HandleShot` holds it.
        let mut player = Ship {
            id: ShipRef::Player,
            fleet: ShipRef::Player,
            ..firer(0x0004)
        };
        player.state.position = Vec2::ZERO;
        assert_eq!(
            fire_ticks(&mut [player, at(900.0)], 14),
            [0, 13],
            "the player's alike"
        );
    }

    /// The Quad Light Blaster Turret: point defence firing every 5 ticks,
    /// 20 pixels a tick for 12, doing 1 mass and 4 energy damage (3 to a
    /// missile), on its stock 35-pixel sprite.
    fn quad() -> WeaponRecord {
        WeaponRecord {
            guidance: 9,
            reload: 5,
            count: 12,
            speed: 2000,
            mass_dmg: 1,
            energy_dmg: 4,
            frame: Some(crate::catalog::FrameSize {
                width: 35,
                height: 35,
            }),
            ..weapon(133)
        }
    }

    /// Says every missile is `hostile`, recording what it was asked and
    /// the governments it was asked with.
    #[derive(Debug, Default)]
    struct Hostility {
        hostile: bool,
        asked: RefCell<Vec<(Side, Side)>>,
        govts: RefCell<Vec<Governments>>,
    }

    impl PointDefenceRule for Hostility {
        fn hostile(&self, defender: Side, firer: Side, govts: &Governments) -> bool {
            self.asked.borrow_mut().push((defender, firer));
            self.govts.borrow_mut().push(govts.clone());
            self.hostile
        }
    }

    /// The defender A at the centre with `defence`, no target and no
    /// trigger, and B 200 pixels above, facing it, firing a missile at
    /// it on the first tick, on the IR Missile's 18-pixel sprite.
    fn missile_attack(defence: WeaponRecord) -> [Ship; 2] {
        let mut defender = Ship::at(1, 0.0, 0.0).armed(defence).facing(0.0);
        defender.trigger = QUIET;
        [
            defender,
            Ship::at(2, 0.0, -200.0)
                .armed(drawn(
                    WeaponRecord {
                        speed: 500,
                        ..missile()
                    },
                    18,
                ))
                .facing(180.0)
                .targeting(A),
        ]
    }

    /// Runs the ship loop once, so the missile `ships[1]` fires is in
    /// flight before a test's first defended tick, as an NPC's point
    /// defence (`_AIDispatch` @0x4429d) first meets a missile an NPC fires
    /// (`_HandleShip` @0x44483) on the next tick. `ships[1]` fires no
    /// more.
    fn missile_in_flight(combat: &mut Combat, ships: &mut [Ship]) {
        phase::handle_npcs(combat, ships, &NovaDisable);
        ships[1].trigger = QUIET;
    }

    #[test]
    fn a_ship_whose_trigger_does_not_ask_runs_no_point_defence() {
        let rule = Hostility {
            hostile: true,
            ..Hostility::default()
        };
        let rules = Rules {
            defence: &rule,
            ..Rules::default()
        };
        let defended = |point_defence: bool| {
            let mut combat = Combat::default();
            let mut ships = missile_attack(quad());
            ships[0].trigger = Trigger {
                point_defence,
                ..Trigger::default()
            };
            missile_in_flight(&mut combat, &mut ships);
            tick_among(
                &mut combat,
                &mut ships,
                &Arsenal::default(),
                &Governments::default(),
                rules,
            );
            combat
                .take_events()
                .iter()
                .filter(|e| matches!(e, CombatEvent::Fired { ship: A, .. }))
                .count()
        };
        assert_eq!(defended(false), 0, "not asked");
        assert_eq!(defended(true), 1, "asked");
    }

    #[test]
    fn a_ship_whose_jump_is_under_way_runs_no_point_defence() {
        let defended = |jumping: bool| {
            let pd = WeaponRecord {
                burst_count: 4,
                burst_reload: 20,
                ammo_type: 27,
                exit_type: 1,
                ..quad()
            };
            let [defender, attacker] = missile_attack(pd);
            let mut defender = defender.of(exits_at(1, [(10, 0), (20, 0), (30, 0), (40, 0)]));
            defender.rounds = LauncherRounds::from([(WeaponId(133), 5)]);
            defender.jumping = jumping;
            let mut ships = [defender, attacker];
            let mut combat = Combat::default();
            missile_in_flight(&mut combat, &mut ships);
            tick(&mut combat, &mut ships, &NovaDisable);
            let mount = ships[0].armament.mounts()[0];
            (
                fired(&combat.take_events(), A),
                mount.burst,
                ships[0].rounds.slot(WeaponId(133)),
                *ships[0].armament.exit_cursors(),
                mount.reload,
            )
        };
        let (fired, burst, rounds, cursors, reload) = defended(true);
        assert_eq!((fired, burst, rounds), (0, 0, 5), "nothing fired or spent");
        assert_eq!(cursors, ExitCursors::default(), "no exit taken");
        assert_eq!(reload, 0.0, "no reload");
        let (fired, burst, rounds, cursors, reload) = defended(false);
        assert_eq!((fired, burst, rounds), (1, 1, 4), "not jumping, it fires");
        assert_ne!(cursors, ExitCursors::default());
        assert_eq!(reload, 4.0, "5, a tick gone");
    }

    /// The durability of the one missile in flight, if any.
    fn missile_durability(combat: &Combat) -> Option<f32> {
        let missiles: Vec<_> = combat
            .shots()
            .iter()
            .filter(|shot| shot.weapon.id == WeaponId(134))
            .collect();
        assert!(missiles.len() <= 1);
        missiles.first().map(|shot| shot.durability)
    }

    #[test]
    fn point_defence_shoots_down_a_missile_by_the_hit_after_its_durability_is_gone() {
        let mut combat = Combat::default();
        let mut ships = missile_attack(quad());
        let mut durabilities = Vec::new();
        let mut events = Vec::new();
        missile_in_flight(&mut combat, &mut ships);
        for _ in 0..30 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[1].trigger = QUIET;
            let new = combat.take_events();
            let defended = new
                .iter()
                .filter(|e| matches!(e, CombatEvent::Fired { ship: A, .. }))
                .count();
            let spent_before = combat
                .shots()
                .iter()
                .filter(|s| s.weapon.id == WeaponId(133))
                .count();
            events.extend(new);
            durabilities.push((missile_durability(&combat), defended, spent_before));
        }
        // 3 to a missile of 4: 4, 1, -2, then gone. Its shots, each fired
        // 5 ticks apart from the tick after the lead-in, closing on it at
        // 25 pixels a tick, end their moves within reach of it on ticks 7,
        // 11 and 15: their 35-pixel sprites overlap its 18-pixel one under
        // 26 pixels apart. Each meeting comes in the next tick's hit pass.
        let seen: Vec<Option<f32>> = durabilities.iter().map(|(d, _, _)| *d).collect();
        assert_eq!(&seen[..7], [Some(4.0); 7]);
        assert_eq!(&seen[7..11], [Some(1.0); 4]);
        assert_eq!(&seen[11..15], [Some(-2.0); 4]);
        assert_eq!(&seen[15..], [None; 15]);
        let downed: Vec<CombatEvent> = events
            .iter()
            .filter(|e| matches!(e, CombatEvent::ShotDown { .. }))
            .copied()
            .collect();
        assert_eq!(downed.len(), 1, "{events:?}");
        let CombatEvent::ShotDown { at } = downed[0] else {
            unreachable!()
        };
        assert!(
            (at - Vec2::new(0.0, -100.0)).length() < 1e-3,
            "at the third shot, not the missile at -120: {at:?}"
        );
        let fired: Vec<usize> = durabilities.iter().map(|(_, f, _)| *f).collect();
        assert_eq!(&fired[..15], [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0]);
        assert_eq!(&fired[15..], [0; 15], "nothing left to shoot at");
        let pd_shots: Vec<usize> = durabilities.iter().map(|(_, _, s)| *s).collect();
        assert_eq!(pd_shots[6], 2, "two in flight");
        assert_eq!(pd_shots[7], 1, "the first spent on the missile");
        assert_eq!(
            ships[0].reserves,
            Reserves::full(10.0, 30.0, 100.0),
            "untouched"
        );
    }

    #[test]
    fn each_shot_in_flight_has_its_own_number() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()),
            Ship::at(2, 0.0, 500.0).armed(blaster()),
        ];
        for _ in 0..3 {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        let mut ids: Vec<ShotId> = combat.shots().iter().map(|shot| shot.id).collect();
        assert_eq!(ids.len(), 6);
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 6, "{:?}", combat.shots());
    }

    #[test]
    fn a_shot_of_no_point_defence_meets_no_missile() {
        let gun = WeaponRecord {
            guidance: -1,
            ..quad()
        };
        let mut combat = Combat::default();
        let mut ships = missile_attack(gun);
        ships[0].trigger = Trigger {
            primary: true,
            secondary: None,
            only: None,
            bays: false,
            bays_after: false,
            point_defence: true,
            keep: None,
        };
        for _ in 0..12 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[1].trigger = QUIET;
        }
        assert_eq!(missile_durability(&combat), Some(4.0), "flown through");
        assert!(
            !combat
                .take_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::ShotDown { .. }))
        );
    }

    #[test]
    fn point_defence_asks_the_rule_who_is_hostile_only_by_the_other_reading() {
        let govts = Governments::new([nova_sim_catalog::fixture::govt(140)]);
        let fought = |source: RuleSource| {
            let mut combat = Combat::default();
            combat.set_pd_hostility(source);
            let mut ships = missile_attack(quad());
            ships[1].govt = Some(GovtId(140));
            let rule = Hostility::default();
            let rules = Rules {
                defence: &rule,
                ..Rules::default()
            };
            missile_in_flight(&mut combat, &mut ships);
            for _ in 0..3 {
                tick_among(&mut combat, &mut ships, &Arsenal::default(), &govts, rules);
                ships[1].trigger = QUIET;
            }
            (combat.shots().len(), rule.asked.take(), rule.govts.take())
        };
        let (shots, asked, _) = fought(RuleSource::Engine);
        assert_eq!(shots, 2, "by the engine, the missile and a shot at it");
        assert_eq!(asked, [], "never asked");
        let (shots, asked, asked_with) = fought(RuleSource::Bible);
        assert_eq!(shots, 1, "by the other reading, only the missile");
        let defender = Side {
            ship: A,
            fleet: A,
            govt: None,
        };
        let attacker = Side {
            ship: B,
            fleet: B,
            govt: Some(GovtId(140)),
        };
        assert_eq!(asked, [(defender, attacker); 3], "asked each tick");
        assert!(asked_with.iter().all(|asked| *asked == govts));
        assert_eq!(Combat::default().pd_hostility, RuleSource::Engine);
    }

    #[test]
    fn in_double_time_a_point_defence_beam_does_double_damage_to_a_missile() {
        let mut combat = Combat::default();
        let mut ships = missile_attack(pd_beam());
        let mut seen = Vec::new();
        missile_in_flight(&mut combat, &mut ships);
        for _ in 0..2 {
            tick_on(&mut combat, &mut ships, DOUBLE_TIME);
            ships[1].trigger = QUIET;
            seen.push(missile_durability(&combat));
        }
        assert_eq!(seen, [Some(-2.0), None], "6 a hit, from 4");
    }

    #[test]
    fn on_the_second_pass_a_point_defence_beam_does_no_damage() {
        let mut combat = Combat::default();
        let mut ships = missile_attack(pd_beam());
        missile_in_flight(&mut combat, &mut ships);
        tick_on(&mut combat, &mut ships, DOUBLE_TIME);
        ships[1].trigger = QUIET;
        tick_on(&mut combat, &mut ships, GamePass::Again);
        assert_eq!(missile_durability(&combat), Some(-2.0));
        assert_eq!(combat.beams().len(), 1);
    }

    #[test]
    fn a_point_defence_beam_shoots_down_a_missile_a_tick_at_a_time() {
        let beam = WeaponRecord {
            guidance: 10,
            reload: 30,
            count: 5,
            beam_length: 300,
            mass_dmg: 1,
            energy_dmg: 4,
            ..weapon(150)
        };
        let mut combat = Combat::default();
        // The missile is 200 above A as the beam first fires: fired the
        // tick before, it first moves after the NPCs' point defence
        // (`_AIDispatch` @0x4429d before `_HandleShot` @0x44385).
        let mut ships = missile_attack(beam);
        let mut seen = Vec::new();
        let mut events = Vec::new();
        missile_in_flight(&mut combat, &mut ships);
        for _ in 0..4 {
            tick_with(
                &mut combat,
                &mut ships,
                &Arsenal::default(),
                Rules::default(),
            );
            ships[1].trigger = QUIET;
            seen.push((missile_durability(&combat), combat.beams().len()));
            events.extend(combat.take_events());
        }
        assert_eq!(
            seen,
            [(Some(1.0), 1), (Some(-2.0), 1), (None, 0), (None, 0)]
        );
        let downed: Vec<Vec2> = events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::ShotDown { at } => Some(at),
                _ => None,
            })
            .collect();
        assert_eq!(downed.len(), 1, "{events:?}");
        assert!(
            (downed[0] - Vec2::new(0.0, -185.0)).length() < 1e-3,
            "{downed:?}"
        );
        assert_eq!(ships[0].reserves, Reserves::full(10.0, 30.0, 100.0));
    }

    // Point defence meeting missiles, by their sprites.

    /// `record` drawn on a `size`-pixel square sprite.
    fn drawn(record: WeaponRecord, size: u32) -> WeaponRecord {
        WeaponRecord {
            frame: Some(crate::catalog::FrameSize {
                width: size,
                height: size,
            }),
            ..record
        }
    }

    /// A point-defence shot on a 10-pixel sprite, still unless it is
    /// given a velocity, doing the Quad's 3 to a missile.
    fn pd_shot() -> WeaponRecord {
        drawn(WeaponRecord { speed: 0, ..quad() }, 10)
    }

    /// A homing missile on a 10-pixel sprite, still, that the first hit
    /// of point defence downs.
    fn still_missile() -> WeaponRecord {
        drawn(
            WeaponRecord {
                speed: 0,
                durability: 0,
                ..missile()
            },
            10,
        )
    }

    /// Puts a shot of `record` in flight, fired by `firer`, whose first
    /// move took it from `at` by `velocity` (a homing one at its speed,
    /// still here), for the next tick's hit pass to test, and gives its
    /// number.
    fn put(
        combat: &mut Combat,
        record: WeaponRecord,
        firer: ShipRef,
        at: Vec2,
        velocity: Vec2,
    ) -> ShotId {
        let from = ShipState {
            position: at,
            ..ShipState::default()
        };
        let shot = Shot {
            id: combat.next_id(),
            velocity,
            position: at + velocity,
            moved_from: Some(at),
            age: 1,
            ..Shot::launch(WeaponSpec::new(&record), firer, firer, &from, 0.0)
        };
        combat.put_in_flight(shot);
        shot.id
    }

    /// The shot numbered `id` still in flight, if any.
    fn flying(combat: &Combat, id: ShotId) -> Option<&Shot> {
        combat.shots().iter().find(|shot| shot.id == id)
    }

    /// Where `events` say point defence shot missiles down.
    fn shot_down(events: &[CombatEvent]) -> Vec<Vec2> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::ShotDown { at } => Some(at),
                _ => None,
            })
            .collect()
    }

    /// A tick of `combat` among `ships`, by Nova's rules: its events.
    fn met(combat: &mut Combat, ships: &mut [Ship]) -> Vec<CombatEvent> {
        tick(combat, ships, &NovaDisable);
        combat.take_events()
    }

    #[test]
    fn a_point_defence_shot_downs_a_lost_immune_friendly_or_misaimed_missile_it_overlaps() {
        type Making = (&'static str, fn(&mut Shot));
        let variants: [Making; 5] = [
            ("lost", |shot| shot.jam = Jam::Lost),
            ("immune to point defence", |shot| shot.weapon.flags = 0x0080),
            ("its own ship's", |shot| {
                shot.firer = A;
                shot.fleet = A;
            }),
            ("its fleet's", |shot| shot.fleet = A),
            ("aimed at another ship", |shot| {
                shot.target = Some(C);
                shot.aimed_at = Some(C);
            }),
        ];
        for (what, made) in variants {
            let mut combat = Combat::default();
            let pd = put(&mut combat, pd_shot(), A, Vec2::ZERO, Vec2::ZERO);
            let missile = put(
                &mut combat,
                still_missile(),
                B,
                Vec2::new(3.0, 0.0),
                Vec2::ZERO,
            );
            made(&mut combat.shots[1]);
            let mut ships = [Ship::at(3, 1000.0, 1000.0)];
            let events = met(&mut combat, &mut ships);
            assert_eq!(flying(&combat, missile), None, "{what}");
            assert_eq!(flying(&combat, pd), None, "{what}: spent");
            assert_eq!(shot_down(&events), [Vec2::ZERO], "{what}");
        }
    }

    #[test]
    fn a_point_defence_shot_meets_every_missile_it_overlaps() {
        let mut combat = Combat::default();
        put(&mut combat, pd_shot(), A, Vec2::ZERO, Vec2::ZERO);
        put(
            &mut combat,
            still_missile(),
            B,
            Vec2::new(3.0, 0.0),
            Vec2::ZERO,
        );
        put(
            &mut combat,
            still_missile(),
            B,
            Vec2::new(-3.0, 2.0),
            Vec2::ZERO,
        );
        let events = met(&mut combat, &mut []);
        assert_eq!(combat.shots(), [], "both downed, the shot spent");
        assert_eq!(shot_down(&events), [Vec2::ZERO, Vec2::ZERO]);
    }

    #[test]
    fn a_missile_downed_by_one_point_defence_shot_is_not_met_by_the_next_which_flies_on() {
        let mut combat = Combat::default();
        let first = put(&mut combat, pd_shot(), A, Vec2::ZERO, Vec2::ZERO);
        let second = put(&mut combat, pd_shot(), A, Vec2::new(1.0, 0.0), Vec2::ZERO);
        let missile = put(
            &mut combat,
            still_missile(),
            B,
            Vec2::new(3.0, 0.0),
            Vec2::ZERO,
        );
        let events = met(&mut combat, &mut []);
        assert_eq!(flying(&combat, missile), None);
        assert_eq!(flying(&combat, first), None, "spent");
        assert!(flying(&combat, second).is_some(), "flies on");
        assert_eq!(shot_down(&events), [Vec2::ZERO], "by the first");
    }

    #[test]
    fn a_missile_a_hit_leaves_standing_is_hit_again_by_the_next_shot_that_tick() {
        let mut combat = Combat::default();
        let first = put(&mut combat, pd_shot(), A, Vec2::ZERO, Vec2::ZERO);
        let second = put(&mut combat, pd_shot(), A, Vec2::new(1.0, 0.0), Vec2::ZERO);
        let tough = WeaponRecord {
            durability: 4,
            ..still_missile()
        };
        let missile = put(&mut combat, tough, B, Vec2::new(3.0, 0.0), Vec2::ZERO);
        let events = met(&mut combat, &mut []);
        assert_eq!(
            flying(&combat, missile).map(|shot| shot.durability),
            Some(-2.0),
            "4, 1, then -2"
        );
        assert_eq!(
            (flying(&combat, first), flying(&combat, second)),
            (None, None)
        );
        assert_eq!(shot_down(&events), []);
    }

    #[test]
    fn a_missile_is_shot_down_where_the_point_defence_shot_is() {
        let mut combat = Combat::default();
        put(
            &mut combat,
            pd_shot(),
            A,
            Vec2::new(-4.0, 1.0),
            Vec2::new(2.0, 0.0),
        );
        put(
            &mut combat,
            still_missile(),
            B,
            Vec2::new(4.0, 0.0),
            Vec2::ZERO,
        );
        let events = met(&mut combat, &mut []);
        assert_eq!(shot_down(&events), [Vec2::new(-2.0, 1.0)], "not at 4, 0");
    }

    #[test]
    fn a_point_defence_shot_meets_a_missile_by_its_sprite_where_both_end_the_tick() {
        let meets = |pd: WeaponRecord, from: Vec2, velocity: Vec2, missile: WeaponRecord, at| {
            let mut combat = Combat::default();
            put(&mut combat, pd, A, from, velocity);
            let missile = put(&mut combat, missile, B, at, Vec2::ZERO);
            met(&mut combat, &mut []);
            flying(&combat, missile).is_none()
        };
        let still = Vec2::ZERO;
        assert!(
            !meets(
                pd_shot(),
                Vec2::new(-50.0, 0.0),
                Vec2::new(100.0, 0.0),
                still_missile(),
                still
            ),
            "passed through between ticks"
        );
        let small = |record| drawn(record, 2);
        assert!(
            !meets(
                small(pd_shot()),
                still,
                still,
                small(still_missile()),
                Vec2::new(5.0, 0.0)
            ),
            "5 pixels off, 2-pixel sprites apart"
        );
        let large = |record| drawn(record, 40);
        assert!(
            meets(
                large(pd_shot()),
                still,
                still,
                large(still_missile()),
                Vec2::new(30.0, 0.0)
            ),
            "30 pixels off, 40-pixel sprites overlapping"
        );
        assert!(
            !meets(
                pd_shot(),
                still,
                still,
                WeaponRecord {
                    frame: None,
                    ..still_missile()
                },
                still
            ),
            "a missile of no sprite"
        );
    }

    #[test]
    fn a_point_defence_shot_meets_no_shot_but_a_homing_missile() {
        let mut combat = Combat::default();
        let mut shots = vec![put(&mut combat, pd_shot(), A, Vec2::ZERO, Vec2::ZERO)];
        for guidance in [-1, 4, 5] {
            let shell = drawn(
                WeaponRecord {
                    speed: 0,
                    ..turret(guidance)
                },
                10,
            );
            shots.push(put(&mut combat, shell, B, Vec2::new(3.0, 0.0), Vec2::ZERO));
        }
        shots.push(put(
            &mut combat,
            pd_shot(),
            B,
            Vec2::new(-3.0, 0.0),
            Vec2::ZERO,
        ));
        let events = met(&mut combat, &mut []);
        let left: Vec<ShotId> = combat.shots().iter().map(|shot| shot.id).collect();
        assert_eq!(left, shots);
        assert_eq!(shot_down(&events), []);
    }

    #[test]
    fn missiles_whose_sprites_overlap_never_meet_each_other() {
        let mut combat = Combat::default();
        let shell = drawn(
            WeaponRecord {
                speed: 0,
                ..turret(-1)
            },
            10,
        );
        let mut shots = vec![put(&mut combat, shell, A, Vec2::ZERO, Vec2::ZERO)];
        shots.push(put(&mut combat, still_missile(), A, Vec2::ZERO, Vec2::ZERO));
        shots.push(put(
            &mut combat,
            still_missile(),
            B,
            Vec2::new(3.0, 0.0),
            Vec2::ZERO,
        ));
        let events = met(&mut combat, &mut []);
        let left: Vec<ShotId> = combat.shots().iter().map(|shot| shot.id).collect();
        assert_eq!(left, shots);
        assert_eq!(shot_down(&events), []);
    }

    #[test]
    fn a_missile_that_touches_a_ship_hits_it_and_point_defence_never_meets_it() {
        let mut combat = Combat::default();
        let pd = put(&mut combat, pd_shot(), A, Vec2::new(498.0, 0.0), Vec2::ZERO);
        let missile = put(
            &mut combat,
            still_missile(),
            C,
            Vec2::new(500.0, 0.0),
            Vec2::ZERO,
        );
        combat.shots[1].target = Some(B);
        combat.shots[1].aimed_at = Some(B);
        let mut ships = [Ship::at(2, 500.0, 0.0)];
        let events = met(&mut combat, &mut ships);
        assert_eq!(flying(&combat, missile), None, "gone on the ship");
        assert!(flying(&combat, pd).is_some(), "not spent");
        assert_eq!(shot_down(&events), []);
        let strikes = combat.take_strikes();
        assert!(
            strikes
                .iter()
                .any(|strike| strike.ship == B && strike.direct),
            "{strikes:?}"
        );
    }

    #[test]
    fn a_missile_expiring_on_a_ship_is_met_by_point_defence_as_it_hits_nothing() {
        let mut combat = Combat::default();
        let expiring = WeaponRecord {
            count: 1,
            ..still_missile()
        };
        let missile = put(&mut combat, expiring, C, Vec2::new(500.0, 0.0), Vec2::ZERO);
        combat.shots[0].target = Some(B);
        combat.shots[0].aimed_at = Some(B);
        put(&mut combat, pd_shot(), A, Vec2::new(498.0, 0.0), Vec2::ZERO);
        let mut ships = [Ship::at(2, 500.0, 0.0)];
        let events = met(&mut combat, &mut ships);
        assert_eq!(flying(&combat, missile), None);
        assert_eq!(shot_down(&events), [Vec2::new(498.0, 0.0)]);
        assert_eq!(combat.take_strikes(), []);
    }

    #[test]
    fn a_missile_a_hit_leaves_standing_still_sets_off_its_proximity_fuse() {
        let mut combat = Combat::default();
        let fused = WeaponRecord {
            durability: 4,
            prox_radius: 10,
            ..still_missile()
        };
        let missile = put(&mut combat, fused, C, Vec2::new(520.0, 0.0), Vec2::ZERO);
        combat.shots[0].target = Some(B);
        combat.shots[0].aimed_at = Some(B);
        let pd = put(&mut combat, pd_shot(), A, Vec2::new(522.0, 0.0), Vec2::ZERO);
        let mut ships = [Ship::at(2, 500.0, 0.0)];
        met(&mut combat, &mut ships);
        assert_eq!(flying(&combat, pd), None, "spent on it");
        assert_eq!(flying(&combat, missile), None, "gone on the ship");
        let strikes = combat.take_strikes();
        assert!(
            strikes
                .iter()
                .any(|strike| strike.ship == B && strike.direct),
            "20 pixels off, within 16 and its fuse's 10: {strikes:?}"
        );
    }

    #[test]
    fn a_missile_downed_as_it_expires_vanishes_and_an_expiring_shot_still_meets() {
        let flak = |record: WeaponRecord| WeaponRecord {
            count: 1,
            flags: 0x8000,
            explod_type: 2,
            ..record
        };
        let exploded = |events: &[CombatEvent]| {
            events
                .iter()
                .any(|event| matches!(event, CombatEvent::Exploded { .. }))
        };
        let mut alone = Combat::default();
        put(&mut alone, flak(still_missile()), B, Vec2::ZERO, Vec2::ZERO);
        assert!(exploded(&met(&mut alone, &mut [])), "it would detonate");
        let mut combat = Combat::default();
        let expiring_pd = WeaponRecord {
            count: 1,
            ..pd_shot()
        };
        put(&mut combat, expiring_pd, A, Vec2::ZERO, Vec2::ZERO);
        put(
            &mut combat,
            flak(still_missile()),
            B,
            Vec2::ZERO,
            Vec2::ZERO,
        );
        let events = met(&mut combat, &mut []);
        assert_eq!(combat.shots(), []);
        assert_eq!(shot_down(&events), [Vec2::ZERO]);
        assert!(!exploded(&events), "vanished: {events:?}");
    }

    #[test]
    fn point_defence_sees_the_sprites_from_where_the_player_is() {
        let meets = |player_x: f32| {
            let mut combat = Combat::default();
            put(&mut combat, pd_shot(), A, Vec2::new(0.5, 100.0), Vec2::ZERO);
            let missile = put(
                &mut combat,
                still_missile(),
                B,
                Vec2::new(10.4, 100.0),
                Vec2::ZERO,
            );
            let mut player = Ship::at(9, player_x, 0.0);
            player.id = ShipRef::Player;
            player.fleet = ShipRef::Player;
            met(&mut combat, &mut [player]);
            flying(&combat, missile).is_none()
        };
        assert!(!meets(0.0), "0 and 10: an edge apart");
        assert!(meets(0.6), "0 and 9: overlapping");
    }

    /// A shell 10 pixels a tick for 3 ticks releasing 2 of weapon 148 with
    /// `flags2`, and that weapon.
    fn cluster(flags2: u16) -> (WeaponRecord, Arsenal) {
        let shell = WeaponRecord {
            reload: 1000,
            count: 3,
            speed: 1000,
            energy_dmg: 1,
            sub_count: 2,
            sub_type: Some(WeaponId(148)),
            sub_theta: -10,
            flags2,
            ..weapon(182)
        };
        let sub = WeaponRecord {
            count: 50,
            speed: 500,
            ..weapon(148)
        };
        (shell, Arsenal::new(&[shell, sub], Vec::new()))
    }

    fn sub_shots(combat: &Combat) -> Vec<&Shot> {
        combat
            .shots()
            .iter()
            .filter(|shot| shot.weapon.id == WeaponId(148))
            .collect()
    }

    #[test]
    fn a_shot_releases_its_sub_munitions_at_the_end_of_its_life() {
        let (shell, arsenal) = cluster(0);
        let turret_shell = WeaponRecord {
            guidance: 4,
            ..shell
        };
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(turret_shell).targeting(C),
            Ship::at(3, 500.0, 0.0),
        ];
        let mut events = Vec::new();
        // Its third move, on the fourth tick, spends it; the next tick
        // releases them where it ended, and they take their first move
        // that tick.
        for _ in 0..5 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            events.extend(combat.take_events());
        }
        let subs = sub_shots(&combat);
        assert_eq!(subs.len(), 2);
        let at = subs[0].moved_from.expect("flown");
        assert!((at - Vec2::new(30.0, 0.0)).length() < 1e-3, "{at:?}");
        for sub in &subs {
            assert_eq!(sub.moved_from, Some(at));
            assert_eq!(sub.age, 1, "a move in the tick it is released");
            assert_eq!(sub.generation, 1);
            assert_eq!(sub.target, Some(C), "the shell's target");
            assert_eq!(sub.aimed_at, Some(C), "aimed at its own target");
        }
        assert_eq!((subs[0].heading, subs[1].heading), (85.0, 95.0));
        assert_ne!(subs[0].id, subs[1].id);
        assert_eq!(
            events.last(),
            Some(&CombatEvent::Fired {
                ship: A,
                weapon: WeaponId(148),
                at,
                sound: Some(FireSound::at(SUBMUNITION_PRIORITY)),
            })
        );
        let (shell, arsenal) = cluster(0x0020);
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell)];
        for _ in 0..5 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        }
        assert_eq!(sub_shots(&combat).len(), 0, "none on expiry");
        let (shell, _) = cluster(0);
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell)];
        for _ in 0..5 {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        assert_eq!(combat.shots(), [], "a sub-munition that cannot be read");
    }

    #[test]
    fn a_proximity_hit_releases_its_submunitions_at_the_ship_hit() {
        // B sits 20 pixels off the shell's path, outside its 16-pixel
        // radius. The shell's second move ends at 20, 20 by 20 from B,
        // within its fuse's reach of trunc(20 + 32 x 0.333) = 30; the
        // first ended 30 by 20 from it, out of reach.
        let (shell, arsenal) = cluster(0x0020);
        let fused = WeaponRecord {
            prox_radius: 20,
            ..shell
        };
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(fused), Ship::at(2, 40.0, 20.0)];
        let mut events = Vec::new();
        // Fired on the first tick, its second move is on the third.
        for _ in 0..4 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            events.extend(combat.take_events());
        }
        let subs = sub_shots(&combat);
        assert_eq!(subs.len(), 2, "{events:?}");
        assert_eq!(subs[0].target, Some(B));
        assert_eq!(subs[0].aimed_at, Some(B));
        let at = subs[0].moved_from.expect("flown in the tick released");
        assert!(
            (at - Vec2::new(20.0, 0.0)).length() < 1e-3,
            "where the shell's move ended: {at:?}"
        );
        assert!(events.contains(&CombatEvent::Fired {
            ship: A,
            weapon: WeaponId(148),
            at,
            sound: Some(FireSound::at(SUBMUNITION_PRIORITY)),
        }));
        assert_eq!(SUBMUNITION_PRIORITY, 4);
    }

    #[test]
    fn a_contact_hit_releases_no_submunitions_and_the_shot_releases_none_later() {
        // The shell releases at the end of its life too, but it meets B on
        // its 2nd tick.
        let (shell, arsenal) = cluster(0);
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell), Ship::at(2, 30.0, 0.0)];
        let mut strikes = Vec::new();
        for _ in 0..6 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            ships[0].trigger = QUIET;
            strikes.extend(combat.take_strikes());
        }
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        assert_eq!(strikes[0].ship, B);
        assert_eq!(combat.shots(), [], "no sub-munitions, then or later");
    }

    #[test]
    fn a_shot_that_meets_a_ship_by_contact_and_proximity_in_one_tick_hits_by_contact() {
        // Its first move touches B on the way and ends within its fuse's
        // reach of C, which comes first in the order.
        let (shell, arsenal) = cluster(0);
        let fast = WeaponRecord {
            speed: 10_000,
            count: 10,
            prox_radius: 20,
            ..shell
        };
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(fast),
            Ship::at(3, 110.0, 25.0),
            Ship::at(2, 80.0, 0.0),
        ];
        tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        ships[0].trigger = QUIET;
        assert_eq!(combat.take_strikes(), [], "unmoved on the tick it is fired");
        tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        assert_eq!(combat.take_strikes(), [], "not tested on the tick it moves");
        tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        let strikes = combat.take_strikes();
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        assert_eq!(strikes[0].ship, B);
        assert_eq!(sub_shots(&combat).len(), 0, "a contact hit releases none");
    }

    #[test]
    fn a_shot_is_never_hit_tested_on_its_last_move() {
        // The blaster's last move, its 13th, from 180 to 195, is the only
        // one to come within B's 16 pixels.
        let flak = WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            flags: 0x8000,
            ..blaster()
        };
        for (record, blasted) in [(blaster(), false), (flak, true)] {
            let mut combat = Combat::default();
            let mut ships = [Ship::at(1, 0.0, 0.0).armed(record), Ship::at(2, 205.0, 0.0)];
            let mut strikes = Vec::new();
            // Fired on tick 1, it first moves on tick 2.
            for tick in 1..=15 {
                tick_with(
                    &mut combat,
                    &mut ships,
                    &Arsenal::default(),
                    Rules::default(),
                );
                ships[0].trigger = QUIET;
                if tick == 13 {
                    assert!((combat.shots()[0].position.x - 180.0).abs() < 1e-3);
                }
                strikes.extend(combat.take_strikes());
            }
            assert_eq!(
                combat.shots(),
                [],
                "spent on its 14th tick, gone on the 15th"
            );
            let direct: Vec<bool> = strikes.iter().map(|strike| strike.direct).collect();
            let expected: &[bool] = if blasted { &[false] } else { &[] };
            assert_eq!(direct, expected, "{strikes:?}");
        }
    }

    #[test]
    fn the_ticks_hits_land_contact_proximity_expiry_then_lock() {
        // In one tick: the hit pass finds that a blaster shot's last move
        // touched C, a fused one's ended within reach of D, and a flak
        // shell's spent it beside E; then the player's missile moves and
        // locks on B. The shots are in flight the other way round.
        let player = Ship {
            id: ShipRef::Player,
            fleet: ShipRef::Player,
            ..Ship::at(0, 0.0, 0.0)
        };
        let d = ShipRef::Npc(NpcId(4));
        let e = ShipRef::Npc(NpcId(5));
        let mut ships = [
            player,
            far_b(),
            Ship::at(3, 1000.0, 0.0),
            Ship::at(4, 2000.0, 0.0),
            Ship::at(5, 3000.0, 0.0),
        ];
        // A shot whose last move, its `age`th, took it from (x, y) 10
        // pixels on.
        let flown = |record: WeaponRecord, x: f32, y: f32, age: u32| Shot {
            position: Vec2::new(x + 10.0, y),
            moved_from: Some(Vec2::new(x, y)),
            velocity: Vec2::new(10.0, 0.0),
            heading: 90.0,
            age,
            ..Shot::launch(
                WeaponSpec::new(&record),
                ShipRef::Player,
                ShipRef::Player,
                &ShipState::default(),
                90.0,
            )
        };
        let flak = WeaponRecord {
            count: 5,
            explod_type: 0,
            blast_radius: 30,
            flags: 0x8000,
            ..blaster()
        };
        let fused = WeaponRecord {
            prox_radius: 20,
            ..blaster()
        };
        let missile = Shot {
            target: Some(B),
            aimed_at: Some(B),
            position: Vec2::new(4000.0, 0.0),
            moved_from: None,
            ..flown(missile(), 3990.0, 0.0, 30)
        };
        let mut combat = Combat::default();
        for (n, shot) in [
            flown(flak, 2990.0, 20.0, 5),
            flown(fused, 1990.0, 25.0, 2),
            flown(blaster(), 975.0, 0.0, 2),
            missile,
        ]
        .into_iter()
        .enumerate()
        {
            combat.put_in_flight(Shot {
                id: ShotId(n as u32),
                ..shot
            });
        }
        tick(&mut combat, &mut ships, &NovaDisable);
        let struck: Vec<(ShipRef, bool, bool)> = combat
            .take_strikes()
            .iter()
            .map(|strike| (strike.ship, strike.lock, strike.direct))
            .collect();
        assert_eq!(
            struck,
            [
                (C, false, true),
                (d, false, true),
                (e, false, false),
                (B, true, true)
            ]
        );
    }

    /// A shell like [`cluster`]'s, fired every tick, releasing 2 of `sub`
    /// at the end of its life, and that sub-munition.
    fn cluster_of(sub: WeaponRecord) -> (WeaponRecord, Arsenal) {
        let (shell, _) = cluster(0);
        let shell = WeaponRecord { reload: 0, ..shell };
        let sub = WeaponRecord {
            id: WeaponId(148),
            ..sub
        };
        (shell, Arsenal::new(&[shell, sub], Vec::new()))
    }

    /// The layers of the turret sub-munitions a shell fired by ship 1 of
    /// class 128 releases, its class's turret shots `over` the ships or
    /// not, the firer `leaving` the fight before the release or not, by
    /// `source`'s reading of the source slot.
    fn released_layers(over: bool, leaving: bool, source: RuleSource) -> Vec<ShotLayer> {
        let (shell, _) = cluster(0);
        let sub = WeaponRecord {
            guidance: 4,
            count: 50,
            speed: 500,
            ..weapon(148)
        };
        let class = crate::catalog::HullRecord {
            flags3: if over { 0x0040 } else { 0 },
            ..nova_sim_catalog::fixture::hull(128)
        };
        let arsenal = Arsenal::new(&[shell, sub], vec![class]);
        let mut combat = Combat::default();
        combat.set_submunition_source_slot(source);
        let mut ships = vec![Ship {
            hull: HullSpec {
                turret_shots_over: true,
                ..HullSpec::default()
            },
            ..Ship::at(1, 0.0, 0.0).armed(shell)
        }];
        tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        ships[0].trigger = QUIET;
        if leaving {
            ships.clear();
        }
        for _ in 0..4 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        }
        sub_shots(&combat).iter().map(|shot| shot.layer).collect()
    }

    #[test]
    fn a_sub_munition_takes_its_firers_class_or_the_bit_its_parent_carried() {
        for source in RuleSource::ALL {
            // The firer's class as the arsenal reads it.
            let over = released_layers(true, false, source);
            assert_eq!(over, [ShotLayer::OverShips; 2], "{source:?}");
            let under = released_layers(false, false, source);
            assert_eq!(under, [ShotLayer::Shots; 2], "{source:?}");
        }
        // Gone from the fight: by the engine the class its empty slot
        // keeps (`_HandleShip` @0x3359a); by the other reading the bit the
        // shell carried, its hull's.
        let engine = released_layers(false, true, RuleSource::Engine);
        assert_eq!(engine, [ShotLayer::Shots; 2]);
        let other = released_layers(false, true, RuleSource::Bible);
        assert_eq!(other, [ShotLayer::OverShips; 2]);
    }

    #[test]
    fn a_sub_munition_reports_its_unimplemented_flags_once_when_first_released() {
        let (shell, arsenal) = cluster_of(WeaponRecord {
            guidance: 1,
            count: 50,
            speed: 500,
            flags: 0x0040,
            ..weapon(148)
        });
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell)];
        for _ in 0..2 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        }
        assert_eq!(combat.take_diagnostics(), [], "not before it is released");
        for _ in 0..5 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        }
        assert!(sub_shots(&combat).len() > 2, "released more than once");
        assert_eq!(
            combat.take_diagnostics(),
            [SimDiagnostic::UnimplementedWeaponFlag {
                weapon: WeaponId(148),
                field: flags::FlagField::Flags,
                bit: 0x0040,
            }]
        );
    }

    #[test]
    fn a_sub_munition_of_an_unimplemented_guidance_is_reported_and_not_released() {
        // An undocumented guidance, and a fighter bay, which only a ship
        // launches from.
        for (guidance, ammo_type) in [(11, -1), (99, 144)] {
            let (shell, arsenal) = cluster_of(WeaponRecord {
                guidance,
                ammo_type,
                count: 50,
                speed: 500,
                ..weapon(148)
            });
            let mut combat = Combat::default();
            let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell)];
            let mut events = Vec::new();
            for _ in 0..2 {
                tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            }
            assert_eq!(combat.take_diagnostics(), [], "not before it is released");
            for _ in 0..5 {
                tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
                events.extend(combat.take_events());
            }
            assert_eq!(sub_shots(&combat).len(), 0, "none in flight");
            assert_eq!(combat.take_sorties(), [], "and no fighter");
            assert!(
                !events.iter().any(|event| matches!(
                    event,
                    CombatEvent::Fired {
                        weapon: WeaponId(148),
                        ..
                    }
                )),
                "{events:?}"
            );
            assert_eq!(
                combat.take_diagnostics(),
                [SimDiagnostic::UnimplementedGuidance {
                    weapon: WeaponId(148),
                    guidance,
                }]
            );
        }
    }

    #[test]
    fn clearing_takes_the_shots_and_beams_away() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()),
            Ship::at(2, 0.0, 500.0).armed(laser()),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!((combat.shots().len(), combat.beams().len()), (1, 1));
        combat.clear();
        assert_eq!((combat.shots().len(), combat.beams().len()), (0, 0));
    }

    // Strikes.

    /// Ticks until a strike is made, or 40 ticks pass, and gives the
    /// strikes of that tick.
    fn first_strikes(combat: &mut Combat, ships: &mut [Ship], arsenal: &Arsenal) -> Vec<Strike> {
        for _ in 0..40 {
            tick_with(combat, ships, arsenal, Rules::default());
            let strikes = combat.take_strikes();
            if !strikes.is_empty() {
                return strikes;
            }
        }
        Vec::new()
    }

    #[test]
    fn a_shot_that_hits_strikes_by_its_firer_with_the_shield_and_armour_it_took() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()).targeting(B),
            Ship::at(2, 100.0, 0.0),
        ];
        let strikes = first_strikes(&mut combat, &mut ships, &Arsenal::default());
        assert_eq!(
            strikes,
            [Strike {
                ship: B,
                by: Some(A),
                damage: 14.0,
                downed: None,
                direct: true,
                intended: true,
                lock: false,
                nominal: Nominal {
                    mass: 4.0,
                    energy: 10.0,
                },
                weight: 0.0,
                answered: false,
            }],
            "10 off the shield, then 4 off the armour"
        );
        assert_eq!(combat.take_strikes(), [], "taken");
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            combat.take_strikes(),
            [Strike {
                ship: B,
                by: Some(A),
                damage: 5.0,
                downed: None,
                direct: true,
                intended: true,
                lock: false,
                nominal: Nominal {
                    mass: 4.0,
                    energy: 10.0,
                },
                weight: 0.0,
                answered: false,
            }],
            "the shield down to its floor, a tenth below none, then 4 off the armour"
        );
    }

    #[test]
    fn a_beam_strikes_every_tick_it_touches() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield = Gauge::full(100.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(laser()).targeting(B), target];
        let mut struck = Vec::new();
        for _ in 0..5 {
            tick(&mut combat, &mut ships, &NovaDisable);
            struck.push(combat.take_strikes());
        }
        let hit = vec![Strike {
            ship: B,
            by: Some(A),
            damage: 5.0,
            downed: None,
            direct: true,
            intended: true,
            lock: false,
            nominal: Nominal {
                mass: 0.0,
                energy: 5.0,
            },
            weight: 1.0,
            answered: false,
        }];
        assert_eq!(struck, [hit.clone(), hit.clone(), hit, vec![], vec![]]);
    }

    #[test]
    fn a_blast_strikes_each_ship_it_reaches() {
        let shell = WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            ..blaster()
        };
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(shell),
            Ship::at(2, 100.0, 0.0),
            Ship::at(3, 100.0, 25.0),
        ];
        ships[0].armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                reload: 1000,
                ..shell
            }),
            1,
        )]);
        ships[0].target = Some(B);
        let strikes = first_strikes(&mut combat, &mut ships, &Arsenal::default());
        let of = |ship, direct| Strike {
            ship,
            by: Some(A),
            damage: 14.0,
            downed: None,
            direct,
            intended: direct,
            lock: false,
            nominal: Nominal {
                mass: 4.0,
                energy: 10.0,
            },
            weight: if direct { 1000.0 } else { 0.0 },
            answered: false,
        };
        assert_eq!(
            strikes,
            [of(B, true), of(C, false)],
            "the hit, then the blast"
        );
    }

    #[test]
    fn a_sub_munition_strikes_by_its_parents_firer() {
        let shell = WeaponRecord {
            reload: 1000,
            count: 3,
            speed: 1000,
            sub_count: 1,
            sub_type: Some(WeaponId(148)),
            ..weapon(182)
        };
        let sub = WeaponRecord {
            count: 50,
            speed: 500,
            energy_dmg: 3,
            ..weapon(148)
        };
        let arsenal = Arsenal::new(&[shell, sub], Vec::new());
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell), Ship::at(2, 80.0, 0.0)];
        let strikes = first_strikes(&mut combat, &mut ships, &arsenal);
        assert_eq!(
            strikes,
            [Strike {
                ship: B,
                by: Some(A),
                damage: 3.0,
                downed: None,
                direct: true,
                intended: false,
                lock: false,
                nominal: Nominal {
                    mass: 0.0,
                    energy: 3.0,
                },
                weight: 0.0,
                answered: false,
            }],
            "released at no target"
        );
    }

    #[test]
    fn the_hit_that_disables_a_ship_says_so_and_the_one_that_breaks_it_up_too() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()),
            Ship::at(2, 100.0, 0.0),
        ];
        let mut struck = Vec::new();
        for _ in 0..30 {
            tick(&mut combat, &mut ships, &NovaDisable);
            struck.extend(combat.take_strikes());
        }
        let downed: Vec<Option<Downed>> = struck.iter().map(|strike| strike.downed).collect();
        // Six hits take the shield and 24 armour (6 left, below a third):
        // disabled; two more leave it at -2.
        assert_eq!(
            downed,
            [
                None,
                None,
                None,
                None,
                None,
                Some(Downed::Disabled),
                None,
                Some(Downed::BreakingUp)
            ],
            "{struck:?}"
        );
        assert!(
            struck
                .iter()
                .all(|strike| strike.ship == B && strike.by == Some(A))
        );
    }

    #[test]
    fn a_ship_broken_up_at_once_is_only_breaking_up() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield.now = 0.0;
        target.reserves.armor.now = 3.0;
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster()), target];
        let strikes = first_strikes(&mut combat, &mut ships, &Arsenal::default());
        assert_eq!(
            strikes
                .iter()
                .map(|strike| strike.downed)
                .collect::<Vec<_>>(),
            [Some(Downed::BreakingUp)]
        );
    }

    #[test]
    fn only_the_last_strike_on_a_ship_downed_that_tick_says_so() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield.now = 0.0;
        target.reserves.armor.now = 12.0;
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()),
            target,
            Ship::at(3, 200.0, 0.0).armed(blaster()).facing(270.0),
        ];
        let strikes = first_strikes(&mut combat, &mut ships, &Arsenal::default());
        assert_eq!(
            strikes,
            [
                Strike {
                    ship: B,
                    by: Some(A),
                    damage: 5.0,
                    downed: None,
                    direct: true,
                    intended: false,
                    lock: false,
                    nominal: Nominal {
                        mass: 4.0,
                        energy: 10.0,
                    },
                    weight: 0.0,
                    answered: false,
                },
                Strike {
                    ship: B,
                    by: Some(C),
                    damage: 4.0,
                    downed: Some(Downed::Disabled),
                    direct: true,
                    intended: false,
                    lock: false,
                    nominal: Nominal {
                        mass: 4.0,
                        energy: 10.0,
                    },
                    weight: 0.0,
                    answered: false,
                }
            ],
            "12 armour less 8 is below a third of 30"
        );
    }

    #[test]
    fn a_ship_downed_without_a_strike_makes_none() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 0.0, 0.0);
        target.reserves.armor.now = 0.0;
        tick(&mut combat, &mut [target], &NovaDisable);
        assert_eq!(combat.take_strikes(), []);
    }

    // Fighter bays.

    /// The carrier's fighter bay, 149, of ship 144, 10 degrees inaccurate.
    fn carrier_bay() -> WeaponRecord {
        WeaponRecord {
            guidance: 99,
            ammo_type: 144,
            flags: 0x0002,
            inaccuracy: 10,
            reload: 60,
            speed: 400,
            ..weapon(149)
        }
    }

    /// Ship A at (10, 20), moving (1, 2) and facing right, launching its
    /// fighters as its AI does (`_AILaunchFighter` @0x8e6fa,
    /// [`Trigger::bays`]) from a fighter bay, 149, of ship 144, 10 degrees
    /// inaccurate, holding 2 fighters, and targeting B, which is at
    /// (300, 0).
    fn carrier() -> [Ship; 2] {
        let mut carrier = Ship::at(1, 10.0, 20.0).armed(carrier_bay());
        carrier.state.velocity = Vec2::new(1.0, 2.0);
        carrier.trigger = Trigger {
            bays: true,
            ..Trigger::default()
        };
        carrier.rounds = LauncherRounds::from([(WeaponId(149), 2)]);
        carrier.target = Some(B);
        [carrier, Ship::at(2, 300.0, 0.0)]
    }

    #[test]
    fn a_bays_launch_is_a_sortie_not_a_shot() {
        let mut combat = Combat::default();
        let mut ships = carrier();
        let from = ships[0].state;
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.shots(), [], "no shot");
        assert_eq!(combat.beams(), []);
        assert_eq!(
            combat.take_events(),
            [CombatEvent::Fired {
                ship: A,
                weapon: WeaponId(149),
                at: Vec2::new(10.0, 20.0),
                sound: Some(FireSound {
                    priority: BAY_LAUNCH_PRIORITY,
                    unless_playing: false,
                }),
            }],
            "heard at the carrier, at _AILaunchFighter's 5 with no looped test"
        );
        assert_eq!(BAY_LAUNCH_PRIORITY, 5);
        assert_eq!(
            combat.take_sorties(),
            [Sortie {
                carrier: A,
                bay: WeaponId(149),
                ship: ShipId(144),
                from,
                heading: 99.0,
                target: Some(B),
            }],
            "heading 90, and the inaccuracy drawn: 19 of 20, less 10"
        );
        assert_eq!(combat.take_sorties(), [], "taken");
        assert_eq!(ships[0].rounds.slot(WeaponId(149)), 1, "a round spent");
    }

    #[test]
    fn a_sortie_without_a_target_has_none() {
        let mut combat = Combat::default();
        let [mut carrier, _] = carrier();
        carrier.target = None;
        tick(&mut combat, &mut [carrier], &NovaDisable);
        let sorties = combat.take_sorties();
        assert_eq!(sorties.len(), 1);
        assert_eq!(sorties[0].target, None);
    }

    #[test]
    fn an_npcs_trigger_sounds_each_weapon_once_a_volley_at_priority_4() {
        let mut combat = Combat::default();
        let other = WeaponRecord {
            id: WeaponId(131),
            ..turret(4)
        };
        let mut firer = Ship::at(1, 0.0, 0.0).armed(turret(4)).targeting(B);
        firer.armament = Armament::new([
            (WeaponSpec::new(&turret(4)), 3),
            (WeaponSpec::new(&other), 1),
        ]);
        let mut ships = [firer, Ship::at(2, -150.0, 0.0)];
        tick(&mut combat, &mut ships, &NovaDisable);
        let heard = Some(FireSound::at(AI_FIRE_PRIORITY));
        assert_eq!(
            sounds(&combat.take_events()),
            [(WeaponId(130), heard), (WeaponId(131), heard)],
            "once per weapon, its 3 copies firing as one, as _FireAIShipWeapon sounds once a call"
        );
    }

    #[test]
    fn a_formation_escorts_turret_pick_fires_that_one_weapon_sounding_its_first_copy_at_4() {
        // `_AIEscortFireUnprovoked` → `_AIFireTurret`'s single pick, fired
        // by `_FireAIShipWeapon` (rdm escort-fleet-fidelity phase 1).
        let mut combat = Combat::default();
        let other = WeaponRecord {
            id: WeaponId(131),
            ..turret(4)
        };
        let mut firer = Ship::at(1, 0.0, 0.0).picking(130).targeting(B);
        firer.armament = Armament::new([
            (WeaponSpec::new(&turret(4)), 1),
            (WeaponSpec::new(&other), 1),
        ]);
        let mut ships = [firer, Ship::at(2, -150.0, 0.0)];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            sounds(&combat.take_events()),
            [(WeaponId(130), Some(FireSound::at(AI_FIRE_PRIORITY)))],
            "its pick alone, once a call"
        );
        assert!(
            combat
                .shots()
                .iter()
                .all(|shot| shot.weapon.id == WeaponId(130)),
            "{:?}",
            combat.shots()
        );
    }

    // An NPC's pick (`_FireAIShipWeapon`).

    impl Ship {
        /// The ship firing `weapon` alone, as an NPC's pick.
        fn picking(self, weapon: i16) -> Self {
            Self {
                trigger: Trigger {
                    only: Some(WeaponId(weapon)),
                    ..QUIET
                },
                ..self
            }
        }
    }

    /// The `Fired` events among `events`, each with its sound.
    fn sounds(events: &[CombatEvent]) -> Vec<(WeaponId, Option<FireSound>)> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Fired { weapon, sound, .. } => Some((weapon, sound)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn point_defence_sounds_its_shots_and_beams_at_priority_3() {
        let beam = WeaponRecord {
            guidance: 10,
            reload: 30,
            count: 5,
            beam_length: 300,
            ..weapon(150)
        };
        for defence in [quad(), beam] {
            let id = defence.id;
            let mut combat = Combat::default();
            let mut ships = missile_attack(defence);
            missile_in_flight(&mut combat, &mut ships);
            tick(&mut combat, &mut ships, &NovaDisable);
            let heard: Vec<_> = sounds(&combat.take_events())
                .into_iter()
                .filter(|&(weapon, _)| weapon == id)
                .collect();
            assert_eq!(heard, [(id, Some(FireSound::at(3)))], "{id:?}");
        }
        assert_eq!(PD_FIRE_PRIORITY, 3);
    }

    #[test]
    fn an_npcs_recoiling_weapon_shoves_it_back() {
        let kicking = WeaponRecord {
            recoil: 100,
            ..blaster()
        };
        // Facing right, of mass 50: pushed left 2 a tick.
        let mut ship = Ship::at(1, 0.0, 0.0).armed(kicking).picking(128);
        ship.hull.mass = 50.0;
        let mut combat = Combat::default();
        let mut ships = [ship];
        tick(&mut combat, &mut ships, &NovaDisable);
        let shoves = combat.take_shoves();
        assert_eq!(shoves.len(), 1, "{shoves:?}");
        assert_eq!(shoves[0].ship, A);
        assert!(
            (shoves[0].velocity - Vec2::new(-2.0, 0.0)).length() < 1e-4,
            "{shoves:?}"
        );
        // No recoil without one, nor when nothing spawned.
        let mut still = Ship::at(1, 0.0, 0.0).armed(blaster()).picking(128);
        still.hull.mass = 50.0;
        let mut combat = Combat::default();
        tick(&mut combat, &mut [still], &NovaDisable);
        assert_eq!(combat.take_shoves(), []);
        // A recoiling turret, its target ahead or astern in its blind
        // spot: a call that spawns nothing pushes nothing (@0x88dc0).
        let turret = WeaponRecord {
            recoil: 100,
            flags: crate::combat::aim::BLIND_REAR,
            ..turret(4)
        };
        let pushes = |x: f32| {
            let mut shooter = Ship::at(1, 0.0, 0.0)
                .armed(turret)
                .picking(130)
                .targeting(B);
            shooter.hull.mass = 50.0;
            let mut combat = Combat::default();
            let mut ships = [shooter, Ship::at(2, x, 0.0)];
            tick(&mut combat, &mut ships, &NovaDisable);
            combat.take_shoves()
        };
        assert_eq!(pushes(100.0).len(), 1, "ahead");
        assert_eq!(pushes(-100.0), [], "astern, in its blind spot");
    }

    #[test]
    fn on_a_launch_tick_the_kept_bay_fires_nothing_more() {
        // The launch in `_AIDispatch` (@0x8e6fa) sets the bay's reload to
        // 60; `_HandleShip` counts it down to 59 (@0x33896) before the
        // pick, which finds it reloading.
        let mut combat = Combat::default();
        let mut ships = carrier();
        ships[0].trigger = Trigger {
            bays: true,
            only: Some(WeaponId(149)),
            ..QUIET
        };
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.take_sorties().len(), 1, "one sortie");
        assert_eq!(ships[0].rounds.slot(WeaponId(149)), 1, "one round spent");
        // Kept and ready with no launch, it drains a fighter.
        let mut ships = carrier();
        ships[0].trigger = Trigger {
            only: Some(WeaponId(149)),
            ..QUIET
        };
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.take_sorties(), []);
        assert_eq!(ships[0].rounds.slot(WeaponId(149)), 1, "drained");
    }

    #[test]
    fn an_npc_weapon_sounds_once_a_call_at_priority_4() {
        let together = WeaponRecord {
            flags: crate::combat::weapon::SIMULTANEOUS,
            ..blaster()
        };
        let mut ship = Ship::at(1, 0.0, 0.0).picking(128);
        ship.armament = Armament::new([(WeaponSpec::new(&together), 3)]);
        let mut combat = Combat::default();
        tick(&mut combat, &mut [ship], &NovaDisable);
        let npc = Some(FireSound::at(AI_FIRE_PRIORITY));
        assert_eq!(
            sounds(&combat.take_events()),
            [
                (WeaponId(128), npc),
                (WeaponId(128), None),
                (WeaponId(128), None)
            ]
        );
        // The player's sound at its own priority (`_FirePlayerWeapon`).
        let player = Ship {
            id: ShipRef::Player,
            fleet: ShipRef::Player,
            ..Ship::at(1, 0.0, 0.0).armed(blaster())
        };
        let mut combat = Combat::default();
        tick(&mut combat, &mut [player], &NovaDisable);
        assert_eq!(
            sounds(&combat.take_events()),
            [(WeaponId(128), Some(FireSound::at(PLAYER_FIRE_PRIORITY)))]
        );
        assert_eq!(AI_FIRE_PRIORITY, 4);
    }

    #[test]
    fn a_glowing_weapon_raises_fired_only_when_something_spawned() {
        use crate::combat::aim::BLIND_REAR;
        let glowing = WeaponRecord {
            flags2: 0x0200,
            flags: BLIND_REAR,
            ..turret(4)
        };
        let fired_at = |x: f32| {
            let mut combat = Combat::default();
            let mut ships = [
                Ship::at(1, 0.0, 0.0)
                    .armed(glowing)
                    .picking(130)
                    .targeting(B),
                Ship::at(2, x, 0.0),
            ];
            tick(&mut combat, &mut ships, &NovaDisable);
            fired(&combat.take_events(), A)
        };
        assert_eq!(fired_at(100.0), 1, "ahead");
        assert_eq!(fired_at(-100.0), 0, "astern, in its blind spot");
    }

    #[test]
    fn the_fight_spends_and_measures_an_npcs_pick_as_its_rules_say() {
        // A turret with its target astern, in its blind spot.
        let blind = WeaponRecord {
            flags: crate::combat::aim::BLIND_REAR,
            ammo_type: 10,
            ..turret(4)
        };
        let spent = |spend: RuleSource| {
            let mut combat = Combat::default();
            combat.set_ai_spend(spend);
            let mut shooter = Ship::at(1, 0.0, 0.0).armed(blind).picking(130).targeting(B);
            shooter.rounds = LauncherRounds::from([(WeaponId(130), 5)]);
            let mut ships = [shooter, Ship::at(2, -100.0, 0.0)];
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].rounds.slot(WeaponId(130))
        };
        assert_eq!(spent(RuleSource::Engine), 4);
        assert_eq!(spent(RuleSource::Bible), 5);
        // A front turret facing up, its target 10 degrees to the left.
        let front = turret(7);
        let fires = |arc: RuleSource| {
            let mut combat = Combat::default();
            combat.set_quadrant_arc(arc);
            let mut ships = [
                Ship::at(1, 0.0, 0.0)
                    .facing(0.0)
                    .armed(front)
                    .picking(130)
                    .targeting(B),
                Ship::at(2, -17.632_7, -100.0),
            ];
            tick(&mut combat, &mut ships, &NovaDisable);
            fired(&combat.take_events(), A)
        };
        assert_eq!(fires(RuleSource::Engine), 0, "across the seam");
        assert_eq!(fires(RuleSource::Bible), 1);
    }

    // Stellars.

    use crate::catalog::StellarId;
    use crate::combat::weapon::PLANET_TYPE;

    /// [`blaster`], planet-type, exploding in `bööm` 130.
    fn planet_gun() -> WeaponRecord {
        WeaponRecord {
            flags2: PLANET_TYPE,
            explod_type: 2,
            ..blaster()
        }
    }

    /// Stellar 128 at (`x`, `y`), of `radius` and `strength`.
    fn stellar_at(x: f32, y: f32, radius: f32, strength: i64) -> StellarTarget {
        StellarTarget {
            stellar: StellarId(128),
            position: Vec2::new(x, y),
            radius,
            strength,
        }
    }

    /// Ship A at the centre fires `weapon` once to the right, among
    /// `others`; then the fight goes on for `ticks` ticks more, among
    /// `stellars`, with `arsenal`, the shot taking its first move on the
    /// first of them, as an NPC's shot first moves the tick after its pick
    /// (`_HandleShot` @0x44385 before `_HandleShip` @0x44483). The events,
    /// and the strikes on stellars, in order.
    fn shoot_once(
        weapon: WeaponRecord,
        others: Vec<Ship>,
        stellars: &mut [StellarTarget],
        arsenal: &Arsenal,
        ticks: u32,
    ) -> (Combat, Vec<Ship>, Vec<CombatEvent>, Vec<StellarStrike>) {
        let mut combat = Combat::default();
        let mut ships = vec![Ship::at(1, 0.0, 0.0).armed(weapon)];
        ships.extend(others);
        let (mut events, mut strikes) = (Vec::new(), Vec::new());
        for _ in 0..=ticks {
            tick_by(&mut combat, &mut ships, arsenal, stellars);
            ships[0] = Ship::at(1, 0.0, 0.0);
            events.extend(combat.take_events());
            strikes.extend(combat.take_stellar_strikes());
        }
        (combat, ships, events, strikes)
    }

    fn strike(damage: i64, destroyed: bool) -> StellarStrike {
        StellarStrike {
            stellar: StellarId(128),
            by: Some(A),
            damage,
            destroyed,
        }
    }

    fn explosions(events: &[CombatEvent]) -> Vec<(Vec2, Explosion)> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Exploded { at, explosion, .. } => Some((at, explosion)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_planet_type_shot_entering_a_stellar_damages_it_by_its_mass_and_energy_and_is_gone() {
        let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
        let (combat, _, events, strikes) = shoot_once(
            planet_gun(),
            Vec::new(),
            &mut stellars,
            &Arsenal::default(),
            7,
        );
        assert_eq!(strikes, [strike(14, false)], "4 mass and 10 energy");
        assert_eq!(stellars[0].strength, 86);
        assert_eq!(combat.shots(), [], "gone");
        let seen = explosions(&events);
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(
            (seen[0].0 - Vec2::new(80.0, 0.0)).length() < 1e-3,
            "{seen:?}"
        );
        assert_eq!(Some(seen[0].1), Explosion::decode(2));
    }

    #[test]
    fn a_shot_turned_on_its_firer_strikes_a_stellar_for_no_one() {
        let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
        let mut combat = Combat::default();
        let mut ships = vec![Ship::at(1, 0.0, 0.0).armed(planet_gun())];
        phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
        // A moves off, out of the shot's way; then the shot turns on it.
        ships[0] = Ship::at(1, 0.0, 1000.0);
        for shot in &mut combat.shots {
            shot.source = Source::Other;
            shot.target = Some(A);
            shot.aimed_at = Some(A);
        }
        // Its sixth move, from 75 to 90, enters the stellar.
        for _ in 0..6 {
            phase::fly_shots(&mut combat, &mut ships, &NovaDisable);
        }
        phase::run(
            &mut combat,
            &mut ships,
            &NovaDisable,
            |combat, fighters, arena, chance| {
                let scenery = Scenery {
                    stellars: &mut stellars,
                    ..Scenery::default()
                };
                combat.test_hits(fighters, arena, scenery, chance);
            },
        );
        let strikes = combat.take_stellar_strikes();
        assert_eq!(
            strikes,
            [StellarStrike {
                by: None,
                ..strike(14, false)
            }],
            "its source slot is no ship's (@0x374e1)"
        );
    }

    #[test]
    fn a_shot_of_another_weapon_passes_through_a_stellar() {
        let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
        let (combat, _, events, strikes) =
            shoot_once(blaster(), Vec::new(), &mut stellars, &Arsenal::default(), 8);
        assert_eq!(strikes, []);
        assert_eq!(stellars[0].strength, 100);
        assert_eq!(combat.shots().len(), 1, "flying on");
        assert_eq!(explosions(&events), []);
    }

    #[test]
    fn a_planet_type_ship_in_the_shots_path_is_hit_before_the_stellar() {
        // Only a planet-type ship: rule 8 passes the shot through an
        // ordinary one (`_ShotCanHitShip` @0x44a8e-0x44acc).
        let mut planet = Ship::at(2, 100.0, 0.0);
        planet.hull.planet_type = true;
        let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
        let (_, ships, _, strikes) = shoot_once(
            planet_gun(),
            vec![planet],
            &mut stellars,
            &Arsenal::default(),
            8,
        );
        assert_eq!(strikes, []);
        assert_eq!(stellars[0].strength, 100);
        assert!(ships[1].reserves.shield.now < 10.0, "the ship took it");
    }

    /// A shot 100 pixels a tick from the centre to the right, among
    /// `stellars`: 129 at (100, 0), edge at 80, and 128 at (50, 0), edge
    /// at 40, both on its first tick's path. Its strikes, and where it
    /// exploded.
    fn through_two(stellars: &mut [StellarTarget]) -> (Vec<StellarStrike>, Vec<Vec2>) {
        let fast = WeaponRecord {
            speed: 10_000,
            ..planet_gun()
        };
        // Its first move, tested the next tick.
        let (_, _, events, strikes) =
            shoot_once(fast, Vec::new(), stellars, &Arsenal::default(), 2);
        let at = explosions(&events).iter().map(|(at, _)| *at).collect();
        (strikes, at)
    }

    fn far(strength: i64) -> StellarTarget {
        StellarTarget {
            stellar: StellarId(129),
            ..stellar_at(100.0, 0.0, 20.0, strength)
        }
    }

    fn near(strength: i64) -> StellarTarget {
        stellar_at(50.0, 0.0, 10.0, strength)
    }

    fn close(seen: &[Vec2], x: f32) -> bool {
        seen.len() == 1 && (seen[0] - Vec2::new(x, 0.0)).length() < 1e-3
    }

    #[test]
    fn a_shot_strikes_the_first_stellar_in_the_systems_order_not_the_nearest() {
        // `_CheckShotProximities` walks the system's list and stops at the
        // first it hits (@0x37230-0x37527).
        let mut stellars = [far(100), near(100)];
        let (strikes, seen) = through_two(&mut stellars);
        assert_eq!(
            strikes,
            [StellarStrike {
                stellar: StellarId(129),
                ..strike(14, false)
            }],
            "129, listed first"
        );
        assert_eq!((stellars[0].strength, stellars[1].strength), (86, 100));
        assert!(close(&seen, 80.0), "on the farther's edge: {seen:?}");
        let mut stellars = [near(100), far(100)];
        let (strikes, _) = through_two(&mut stellars);
        assert_eq!(strikes, [strike(14, false)], "128, listed first");
    }

    #[test]
    fn a_shot_passes_a_stellar_shot_down_for_the_next_along_its_path() {
        let mut stellars = [near(-1), far(100)];
        let (strikes, seen) = through_two(&mut stellars);
        assert_eq!(
            strikes,
            [StellarStrike {
                stellar: StellarId(129),
                ..strike(14, false)
            }]
        );
        assert_eq!((stellars[0].strength, stellars[1].strength), (-1, 86));
        assert!(close(&seen, 80.0), "on the farther's edge: {seen:?}");
    }

    #[test]
    fn a_fast_shot_hits_a_stellar_it_passes_between_ticks() {
        let fast = WeaponRecord {
            speed: 10_000,
            ..planet_gun()
        };
        let mut stellars = [stellar_at(150.0, 0.0, 10.0, 100)];
        let (_, _, events, strikes) =
            shoot_once(fast, Vec::new(), &mut stellars, &Arsenal::default(), 3);
        assert_eq!(strikes, [strike(14, false)]);
        let seen = explosions(&events);
        assert!(
            (seen[0].0 - Vec2::new(140.0, 0.0)).length() < 1e-3,
            "{seen:?}"
        );
    }

    #[test]
    fn a_shot_hitting_a_stellar_blasts_no_ship_and_releases_no_sub_munitions() {
        let (shell, arsenal) = cluster(PLANET_TYPE);
        let shell = WeaponRecord {
            blast_radius: 100,
            flags: crate::combat::weapon::DETONATES,
            ..shell
        };
        let mut stellars = [stellar_at(15.0, 0.0, 10.0, 100)];
        let (combat, ships, _, strikes) = shoot_once(
            shell,
            vec![Ship::at(2, 15.0, 40.0)],
            &mut stellars,
            &arsenal,
            5,
        );
        assert_eq!(strikes.len(), 1);
        assert_eq!(
            ships[1].reserves,
            Reserves::full(10.0, 30.0, 100.0),
            "no blast"
        );
        assert_eq!(combat.shots(), [], "no sub-munitions");
    }

    #[test]
    fn a_shot_hits_a_stellar_only_once_it_has_flown_past_its_proximity_safety() {
        let safe = WeaponRecord {
            prox_safety: 3,
            ..planet_gun()
        };
        let mut stellars = [stellar_at(0.0, 0.0, 1000.0, 100)];
        // Its fourth move, the first past its safety, is tested on the
        // fifth tick.
        for ticks in 1..=4 {
            let (_, _, _, strikes) =
                shoot_once(safe, Vec::new(), &mut stellars, &Arsenal::default(), ticks);
            assert_eq!(strikes, [], "{ticks}");
        }
        let (_, _, _, strikes) =
            shoot_once(safe, Vec::new(), &mut stellars, &Arsenal::default(), 5);
        assert_eq!(strikes, [strike(14, false)], "its fourth move");
        let mut stellars = [stellar_at(0.0, 0.0, 1000.0, 100)];
        let (_, _, _, strikes) = shoot_once(
            planet_gun(),
            Vec::new(),
            &mut stellars,
            &Arsenal::default(),
            2,
        );
        assert_eq!(strikes, [strike(14, false)], "none: its first move");
    }

    /// Ships A, at the centre, and B, 50 pixels below, each fire a
    /// [`planet_gun`] once, to the right, inside stellar 128 of
    /// `strength`. Its strikes, and the
    /// shots left.
    fn two_shots_at(strength: i64) -> (Vec<StellarStrike>, usize, i64) {
        let mut stellars = [stellar_at(0.0, 0.0, 1000.0, strength)];
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(planet_gun()),
            Ship::at(2, 0.0, 50.0).armed(planet_gun()),
        ];
        tick_by(&mut combat, &mut ships, &Arsenal::default(), &mut stellars);
        // Their first moves, the next tick, tested the tick after.
        for ship in &mut ships {
            ship.trigger = QUIET;
        }
        for _ in 0..2 {
            tick_by(&mut combat, &mut ships, &Arsenal::default(), &mut stellars);
        }
        (
            combat.take_stellar_strikes(),
            combat.shots().len(),
            stellars[0].strength,
        )
    }

    #[test]
    fn a_stellar_shot_below_none_is_destroyed_once_and_later_shots_pass_through() {
        let (strikes, left, strength) = two_shots_at(10);
        assert_eq!(strikes, [strike(14, true)]);
        assert_eq!((left, strength), (1, -4), "the second flies on");
        let (strikes, left, strength) = two_shots_at(14);
        assert_eq!(
            strikes,
            [
                strike(14, false),
                StellarStrike {
                    by: Some(B),
                    ..strike(14, true)
                }
            ],
            "none left is still standing"
        );
        assert_eq!((left, strength), (0, -14));
    }

    #[test]
    fn a_beam_never_hits_a_stellar() {
        let beam = WeaponRecord {
            flags2: PLANET_TYPE,
            ..laser()
        };
        let mut stellars = [stellar_at(0.0, 0.0, 1000.0, 100)];
        let (_, _, _, strikes) =
            shoot_once(beam, Vec::new(), &mut stellars, &Arsenal::default(), 3);
        assert_eq!(strikes, []);
        assert_eq!(stellars[0].strength, 100);
    }

    #[test]
    fn combat_reports_an_explosion_it_is_told_of() {
        let mut combat = Combat::default();
        let boom = Explosion::decode(7).expect("a bööm");
        combat.explode(Vec2::new(3.0, 4.0), boom);
        assert_eq!(
            combat.take_events(),
            [CombatEvent::Exploded {
                at: Vec2::new(3.0, 4.0),
                explosion: boom,
                size: 0.0,
            }]
        );
    }
    /// `ship` as the escape pod, or a class after it.
    fn pod(ship: Ship, class: ShipId) -> Ship {
        Ship { class, ..ship }
    }

    #[test]
    fn a_shot_beam_blast_or_shockwave_does_no_damage_to_the_escape_pod() {
        let shell = WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            ..blaster()
        };
        let after = ShipId(ESCAPE_POD_SHIP.0 + 1);
        let hurt = |ship: &Ship| left(ship) != (10.0, 30.0);
        // A shot passes through the pod and every class after it to the
        // ship behind, unspent; a beam only through the pod itself.
        for (ship_type, immune, shot_passes, beam_passes) in [
            (ESCAPE_POD_SHIP, true, true, true),
            (after, true, true, false),
            (ShipId(ESCAPE_POD_SHIP.0 - 1), false, false, false),
        ] {
            // A fires at B, D behind it.
            for (record, passes) in [(blaster(), shot_passes), (laser(), beam_passes)] {
                let mut combat = Combat::default();
                let mut ships = [
                    Ship::at(1, 0.0, 0.0).armed(record),
                    pod(Ship::at(2, 100.0, 0.0), ship_type),
                    Ship::at(4, 150.0, 0.0),
                ];
                for _ in 0..12 {
                    tick(&mut combat, &mut ships, &NovaDisable);
                    if record.beam_length == 0 {
                        ships[0].trigger = QUIET;
                    }
                }
                assert_eq!(hurt(&ships[1]), !immune, "{ship_type:?} {record:?}");
                assert_eq!(hurt(&ships[2]), passes, "behind, {ship_type:?} {record:?}");
                let struck = combat.take_strikes().iter().any(|strike| strike.ship == B);
                assert_eq!(struck, !immune, "{ship_type:?} {record:?}");
            }
            // A blast still reaches it: A's shell hits D, C beside it.
            let mut combat = Combat::default();
            let mut ships = [
                Ship::at(1, 0.0, 0.0).armed(shell),
                Ship::at(4, 100.0, 0.0),
                pod(Ship::at(3, 100.0, 25.0), ship_type),
            ];
            for _ in 0..12 {
                tick(&mut combat, &mut ships, &NovaDisable);
                ships[0].trigger = QUIET;
            }
            assert!(hurt(&ships[1]), "hit directly, {ship_type:?}");
            assert_eq!(hurt(&ships[2]), !immune, "blasted, {ship_type:?}");
            let struck = combat.take_strikes().iter().any(|strike| strike.ship == C);
            assert_eq!(struck, !immune, "blasted, {ship_type:?}");
            // A shockwave neither hurts nor pushes it: its push is
            // `_DamageShip`'s, past the early-out (@0x3a87c), the only call
            // in the branch (@0x2da89).
            let mut combat = Combat::default();
            let mut pushed = pod(holding(1, 40.0, 0.0, 100.0, 300.0, 300.0), ship_type);
            pushed.hull.mass = 200.0;
            let mut ships = [wreck(WRECK, 200.0, 0.0, 0.0), pushed];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(
                left(&ships[1]) == (100.0, 300.0),
                immune,
                "shockwave, {ship_type:?}"
            );
            assert_eq!(combat.take_strikes().is_empty(), immune);
            assert_eq!(
                combat.take_shoves().is_empty(),
                immune,
                "pushed, {ship_type:?}"
            );
        }
    }

    #[test]
    fn an_escape_pod_at_zero_armour_neither_breaks_up_nor_explodes() {
        for id in [ShipRef::Player, A] {
            let mut combat = Combat::default();
            let mut ship = pod(Ship::at(1, 0.0, 0.0), ESCAPE_POD_SHIP);
            ship.id = id;
            ship.fleet = id;
            ship.reserves.armor.now = 0.0;
            let mut ships = [ship];
            for _ in 0..20 {
                tick(&mut combat, &mut ships, &NovaDisable);
            }
            assert_eq!(ships[0].condition, Condition::Intact, "{id:?}");
            assert_eq!(about(&combat.take_events(), id), [], "{id:?}");
        }
        // Not so the class before it.
        let mut combat = Combat::default();
        let mut ship = pod(Ship::at(1, 0.0, 0.0), ShipId(ESCAPE_POD_SHIP.0 - 1));
        ship.reserves.armor.now = 0.0;
        let mut ships = [ship];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_ne!(ships[0].condition, Condition::Intact);
    }

    // Exit points (`exits`).

    /// A hull whose `ExitType` `kind`'s four exit points are `points`, of 36
    /// frames, with no compression and no z.
    fn exits_at(kind: usize, points: [(i16, i16); 4]) -> HullSpec {
        let mut raw = ShipExits {
            up_compress: (100, 100),
            down_compress: (100, 100),
            frames: 36,
            ..ShipExits::default()
        };
        raw.points[kind] = points;
        hull_of(raw)
    }

    fn hull_of(raw: ShipExits) -> HullSpec {
        HullSpec {
            exits: ExitPoints::new(&raw),
            ..HullSpec::default()
        }
    }

    impl Ship {
        /// The ship with `hull`, taken whole as the field it fills.
        #[allow(clippy::large_types_passed_by_value)]
        fn of(self, hull: HullSpec) -> Self {
            Self { hull, ..self }
        }

        /// The ship last drawn facing `heading`.
        fn drawn_at(self, heading: f32) -> Self {
            Self {
                drawn_heading: Some(heading),
                ..self
            }
        }
    }

    /// A gun firing from its ship's gun exits every tick, 10 pixels a
    /// tick for 10 ticks.
    fn gun() -> WeaponRecord {
        WeaponRecord {
            count: 10,
            speed: 1000,
            exit_type: 0,
            ..weapon(128)
        }
    }

    /// Guns 3 pixels either side of the nose, 10 ahead.
    const NOSE_GUNS: [(i16, i16); 4] = [(3, 10), (-3, 10), (3, 10), (-3, 10)];

    /// Where `shot` left from, if it has moved at most once: an NPC's shot
    /// fired this tick waits where it left, and the player's has flown a
    /// tick from there.
    fn start(shot: &Shot) -> Vec2 {
        shot.moved_from.unwrap_or(shot.position)
    }

    fn close_to(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    /// Where the newest shot left from, after a tick among `ships`.
    fn next_start(combat: &mut Combat, ships: &mut [Ship]) -> Vec2 {
        tick(combat, ships, &NovaDisable);
        start(combat.shots().last().expect("a shot"))
    }

    #[test]
    fn a_ships_guns_fire_from_its_exit_points_in_turn() {
        let mut combat = Combat::default();
        let ship = Ship::at(1, 50.0, 50.0).armed(gun()).facing(0.0);
        let mut ships = [ship.of(exits_at(0, NOSE_GUNS))];
        let starts: Vec<Vec2> = (0..3)
            .map(|_| next_start(&mut combat, &mut ships))
            .collect();
        for (at, expected) in starts
            .iter()
            .zip([(53.0, 40.0), (47.0, 40.0), (53.0, 40.0)])
        {
            assert!(
                close_to(*at, Vec2::new(expected.0, expected.1)),
                "{starts:?}"
            );
        }
        assert_eq!(combat.shots()[0].heading, 0.0, "along the heading");
    }

    #[test]
    fn the_player_fires_from_its_exit_points() {
        let mut combat = Combat::default();
        let player = Ship {
            id: ShipRef::Player,
            fleet: ShipRef::Player,
            ..Ship::at(0, 0.0, 0.0).armed(gun()).facing(90.0)
        };
        let mut ships = [player.of(exits_at(0, NOSE_GUNS))];
        assert!(close_to(
            next_start(&mut combat, &mut ships),
            Vec2::new(10.0, 3.0)
        ));
        assert!(close_to(
            next_start(&mut combat, &mut ships),
            Vec2::new(10.0, -3.0)
        ));
    }

    #[test]
    fn a_weapon_of_no_exit_type_and_a_ship_of_no_exits_fire_from_the_centre() {
        let mut combat = Combat::default();
        let centre = WeaponRecord {
            exit_type: -1,
            ..gun()
        };
        let mut ships = [Ship::at(1, 5.0, 5.0)
            .armed(centre)
            .facing(0.0)
            .of(exits_at(0, NOSE_GUNS))];
        assert!(close_to(
            next_start(&mut combat, &mut ships),
            Vec2::new(5.0, 5.0)
        ));
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 5.0, 5.0).armed(gun()).facing(0.0)];
        assert!(close_to(
            next_start(&mut combat, &mut ships),
            Vec2::new(5.0, 5.0)
        ));
    }

    #[test]
    fn an_npcs_pick_fires_each_simultaneous_copy_from_the_next_exit() {
        let pair = WeaponRecord {
            flags: crate::combat::weapon::SIMULTANEOUS,
            ..gun()
        };
        let mut combat = Combat::default();
        let mut ships = [Ship {
            armament: Armament::new([(WeaponSpec::new(&pair), 2)]),
            ..Ship::at(1, 0.0, 0.0)
                .facing(0.0)
                .picking(128)
                .of(exits_at(0, NOSE_GUNS))
        }];
        tick(&mut combat, &mut ships, &NovaDisable);
        let starts: Vec<Vec2> = combat.shots().iter().map(start).collect();
        assert_eq!(starts.len(), 2);
        assert!(close_to(starts[0], Vec2::new(3.0, -10.0)), "{starts:?}");
        assert!(close_to(starts[1], Vec2::new(-3.0, -10.0)), "{starts:?}");
    }

    #[test]
    fn a_ship_fires_from_the_frame_it_was_last_drawn_on() {
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0)
            .armed(gun())
            .facing(90.0)
            .drawn_at(4.0)
            .of(exits_at(0, NOSE_GUNS))];
        assert!(
            close_to(next_start(&mut combat, &mut ships), Vec2::new(3.0, -10.0)),
            "frame 0, as drawn"
        );
        assert_eq!(combat.shots()[0].heading, 90.0, "along its heading");
    }

    #[test]
    fn fired_is_heard_at_the_ships_centre() {
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 20.0, 30.0)
            .armed(gun())
            .facing(0.0)
            .of(exits_at(0, NOSE_GUNS))];
        tick(&mut combat, &mut ships, &NovaDisable);
        let at: Vec<Vec2> = combat
            .take_events()
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Fired { at, .. } => Some(at),
                _ => None,
            })
            .collect();
        assert_eq!(at, [Vec2::new(20.0, 30.0)]);
    }

    #[test]
    fn a_turret_leads_its_target_from_its_exit_point() {
        let mut combat = Combat::default();
        let turret = WeaponRecord {
            exit_type: 1,
            ..turret(4)
        };
        let mut ships = [
            Ship::at(1, 0.0, 0.0)
                .armed(turret)
                .facing(0.0)
                .targeting(B)
                .of(exits_at(1, [(60, 0); 4])),
            Ship {
                state: ShipState {
                    position: Vec2::new(0.0, -200.0),
                    velocity: Vec2::new(8.0, 0.0),
                    heading: 0.0,
                },
                ..Ship::at(2, 0.0, -200.0)
            },
        ];
        let target = ships[1].fighter().as_target();
        tick(&mut combat, &mut ships, &NovaDisable);
        let shot = &combat.shots()[0];
        let from = |position| ShipState {
            position,
            ..ShipState::default()
        };
        let spec = WeaponSpec::new(&turret);
        let from_exit = lead_from(&from(Vec2::new(60.0, 0.0)), &target, &spec);
        let from_centre = lead_from(&from(Vec2::ZERO), &target, &spec);
        assert!(close_to(start(shot), Vec2::new(60.0, 0.0)), "{shot:?}");
        assert_eq!(shot.heading, from_exit, "{shot:?}");
        assert!(
            (from_exit - from_centre).abs() > 1.0,
            "{from_exit} {from_centre}"
        );
    }

    #[test]
    fn a_negative_inaccuracy_fires_broadside_by_the_side_of_its_exit() {
        let broadside = WeaponRecord {
            inaccuracy: -90,
            ..gun()
        };
        let mut combat = Combat::default();
        let mut ships = [Ship {
            state: ShipState {
                position: Vec2::ZERO,
                velocity: Vec2::new(1.0, 2.0),
                heading: 0.0,
            },
            ..Ship::at(1, 0.0, 0.0)
                .armed(broadside)
                .of(exits_at(0, [(3, 10), (-3, 10), (0, 10), (0, 10)]))
        }];
        let mut velocities = Vec::new();
        for _ in 0..3 {
            tick(&mut combat, &mut ships, &NovaDisable);
            let shot = combat.shots().last().expect("a shot");
            assert_eq!(shot.heading, 0.0, "facing as aimed");
            velocities.push(shot.velocity);
        }
        assert!(
            close_to(velocities[0], Vec2::new(11.0, 2.0)),
            "to starboard: {velocities:?}"
        );
        assert!(
            close_to(velocities[1], Vec2::new(-9.0, 2.0)),
            "to port: {velocities:?}"
        );
        assert!(
            close_to(velocities[2], Vec2::new(1.0, -8.0)),
            "on the centre line, ahead: {velocities:?}"
        );
    }

    #[test]
    fn a_broadside_angle_wraps_before_it_is_truncated() {
        let broadside = WeaponRecord {
            inaccuracy: -90,
            ..gun()
        };
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0)
            .armed(broadside)
            .facing(45.5)
            .of(exits_at(0, [(-3, 10); 4]))];
        tick(&mut combat, &mut ships, &NovaDisable);
        let velocity = combat.shots()[0].velocity;
        // 45.5 - 90 wraps to 315.5 in float (@0x3ee7e-0x3eea4), then
        // `cvttss2si` (@0x3eeb8) takes 315, not trunc(-44.5) + 360 = 316.
        assert_eq!(velocity, trig::accel(315, 10.0));
    }

    #[test]
    fn a_gun_of_no_broadside_fires_from_a_side_exit_along_its_truncated_aim() {
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0)
            .armed(gun())
            .facing(45.5)
            .of(exits_at(0, [(3, 10); 4]))];
        tick(&mut combat, &mut ships, &NovaDisable);
        let shot = combat.shots()[0];
        assert_eq!(shot.heading, 45.0, "trunc(45.5) (@0x3e677)");
        assert_eq!(shot.velocity, trig::accel(45, 10.0));
    }

    /// The velocity and heading of the shot of `guidance` and `inaccuracy`
    /// that `ship`, moving (1, 2) and facing up, fires from a starboard
    /// exit, after its first move: the player's on the tick it is fired,
    /// an NPC's on the next.
    fn fired_broadside(ship: Ship, guidance: i16, inaccuracy: i16) -> (Vec2, f32) {
        let mut combat = Combat::default();
        let record = WeaponRecord {
            guidance,
            inaccuracy,
            ..gun()
        };
        let mut ships = [Ship {
            state: ShipState {
                position: Vec2::ZERO,
                velocity: Vec2::new(1.0, 2.0),
                heading: 0.0,
            },
            ..ship.armed(record).of(exits_at(0, [(3, 10); 4]))
        }];
        tick(&mut combat, &mut ships, &NovaDisable);
        if combat
            .shots()
            .last()
            .is_some_and(|shot| shot.moved_from.is_none())
        {
            ships[0].trigger = QUIET;
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        let shot = combat.shots().last().expect("a shot");
        (shot.velocity, shot.heading)
    }

    #[test]
    fn an_npcs_broadside_bomb_or_rocket_flies_out_to_its_side() {
        // (1, 2) and 10 to starboard, 90 degrees off its heading of 0.
        let out = Vec2::new(11.0, 2.0);
        let (bomb, facing) = fired_broadside(Ship::at(1, 0.0, 0.0), 5, -90);
        assert!(close_to(bomb, out), "{bomb:?}");
        assert_eq!(facing, 1.0, "aimed ahead, then a degree into the wind");
        let (rocket, facing) = fired_broadside(Ship::at(1, 0.0, 0.0), 6, -90);
        let flown = out * projectile::ROCKET_KEEP
            + crate::flight::facing(0.0) * (10.0 * projectile::ROCKET_PUSH);
        assert!(close_to(rocket, flown), "{rocket:?}");
        assert_eq!(facing, 0.0, "aimed ahead");
    }

    #[test]
    fn the_players_broadside_bomb_or_rocket_keeps_its_launch_velocity() {
        let player = || Ship {
            id: ShipRef::Player,
            fleet: ShipRef::Player,
            ..Ship::at(0, 0.0, 0.0)
        };
        for guidance in [5, 6] {
            assert_eq!(
                fired_broadside(player(), guidance, -90),
                fired_broadside(player(), guidance, 0),
                "{guidance}"
            );
        }
    }

    #[test]
    fn point_defence_never_fires_broadside() {
        let mut combat = Combat::default();
        let [defender, attacker] = defended();
        let broadside = WeaponRecord {
            inaccuracy: -90,
            ..exit_defence()
        };
        let mut ships = [defender.armed(broadside), attacker];
        missile_in_flight(&mut combat, &mut ships);
        tick(&mut combat, &mut ships, &NovaDisable);
        let shot = combat
            .shots()
            .iter()
            .find(|shot| shot.weapon.guidance == Guidance::PointDefence)
            .expect("a point-defence shot");
        let bearing = trig::intercept_angle(start(shot), Vec2::new(150.0, 100.0));
        assert_eq!(shot.heading, f32::from(bearing), "{shot:?}");
        assert_eq!(shot.velocity, trig::accel(bearing, 10.0), "not sideways");
    }

    /// A beam weapon of `guidance` firing from its ship's beam exits for
    /// 10 ticks, 300 pixels long.
    fn exit_beam(guidance: i16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            count: 10,
            beam_length: 300,
            exit_type: 3,
            ..weapon(142)
        }
    }

    #[test]
    fn a_beam_starts_at_its_exit_point_and_stays_on_it_as_its_firer_turns() {
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 100.0, 100.0)
            .armed(exit_beam(0))
            .facing(0.0)
            .of(exits_at(3, [(10, 20); 4]))];
        tick(&mut combat, &mut ships, &NovaDisable);
        let beam = combat.beams()[0];
        assert!(close_to(beam.start, Vec2::new(110.0, 80.0)), "{beam:?}");
        assert!(close_to(beam.end, Vec2::new(110.0, -220.0)), "{beam:?}");
        ships[0].trigger = QUIET;
        ships[0].state.heading = 95.0;
        tick(&mut combat, &mut ships, &NovaDisable);
        let beam = combat.beams()[0];
        assert!(
            close_to(beam.start, Vec2::new(120.0, 110.0)),
            "on frame 9's exit, this tick's: {beam:?}"
        );
    }

    #[test]
    fn a_turret_beam_aims_from_its_exit_point() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0)
                .armed(exit_beam(3))
                .facing(0.0)
                .targeting(B)
                .of(exits_at(3, [(30, 0); 4])),
            Ship::at(2, 0.0, 100.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        let beam = combat.beams()[0];
        assert!(close_to(beam.start, Vec2::new(30.0, 0.0)), "{beam:?}");
        // A whole degree from the exit: 196.7 truncates to 196.
        let toward = crate::trig::intercept_angle(beam.start, Vec2::new(0.0, 100.0));
        assert_eq!((toward, beam.heading), (196, 196.0), "{beam:?}");
        // To its hit, trunc(104.4 - 15) along.
        let along = crate::flight::facing(beam.heading);
        assert!(close_to(beam.end, beam.start + along * 89.0), "{beam:?}");
    }

    /// Beam exits: one ahead (up 25), one 30 to starboard, one behind
    /// (down 25) and one 30 to port, as the Manticore's.
    fn manticore() -> HullSpec {
        let mut raw = ShipExits {
            up_compress: (100, 71),
            down_compress: (100, 71),
            frames: 36,
            ..ShipExits::default()
        };
        raw.points[3] = [(0, 0), (30, 0), (0, 0), (-30, 0)];
        raw.z[3] = [25, 0, -25, 0];
        hull_of(raw)
    }

    #[test]
    fn a_nearest_exit_turret_beam_leaves_from_the_exit_nearest_its_target() {
        let ion = WeaponRecord {
            flags3: crate::combat::weapon::NEAREST_EXIT,
            ..exit_beam(3)
        };
        for (x, exit, at) in [(100.0, 1, 30.0), (-100.0, 3, -30.0)] {
            let mut combat = Combat::default();
            let mut ships = [
                Ship::at(1, 0.0, 0.0)
                    .armed(ion)
                    .facing(0.0)
                    .targeting(B)
                    .of(manticore()),
                Ship::at(2, x, 0.0),
            ];
            tick(&mut combat, &mut ships, &NovaDisable);
            let beam = combat.beams()[0];
            assert_eq!(beam.exit, Some(exit), "{x}");
            assert!(close_to(beam.start, Vec2::new(at, 0.0)), "{x}: {beam:?}");
            assert_eq!(
                ships[0].armament.exit_cursors().next(ExitKind::Beam),
                1,
                "stepped from 0, not set"
            );
        }
    }

    #[test]
    fn a_nearest_exit_gun_fires_from_the_exit_nearest_its_target() {
        let nearest = WeaponRecord {
            flags3: crate::combat::weapon::NEAREST_EXIT,
            ..gun()
        };
        let guns = [(0, 20), (30, 0), (0, -20), (-30, 0)];
        for (x, at, next) in [(100.0, 30.0, 2), (-100.0, -30.0, 0)] {
            let mut combat = Combat::default();
            let mut ships = [
                Ship::at(1, 0.0, 0.0)
                    .armed(nearest)
                    .facing(0.0)
                    .targeting(B)
                    .of(exits_at(0, guns)),
                Ship::at(2, x, 0.0),
            ];
            let from = next_start(&mut combat, &mut ships);
            assert!(close_to(from, Vec2::new(at, 0.0)), "{x}: {from:?}");
            assert_eq!(
                ships[0].armament.exit_cursors().next(ExitKind::Gun),
                next,
                "{x}: on from the exit taken (@0x73d9-0x742c)"
            );
        }
    }

    /// A point-defence turret firing from its ship's turret exits every
    /// tick, 10 pixels a tick for 30 ticks.
    fn exit_defence() -> WeaponRecord {
        WeaponRecord {
            guidance: 9,
            count: 30,
            speed: 1000,
            exit_type: 1,
            ..weapon(161)
        }
    }

    /// A point-defence ship facing up, with turret exits 10, 20, 30 and 40
    /// pixels to its right, and a ship firing a missile at it from
    /// (150, 100).
    fn defended() -> [Ship; 2] {
        let missile = WeaponRecord {
            guidance: 1,
            guided_turn: 70,
            count: 100,
            speed: 100,
            ..weapon(134)
        };
        [
            Ship::at(1, 0.0, 0.0)
                .armed(exit_defence())
                .facing(0.0)
                .targeting(B)
                .of(exits_at(1, [(10, 0), (20, 0), (30, 0), (40, 0)])),
            Ship::at(2, 150.0, 100.0).armed(missile).targeting(A),
        ]
    }

    /// The start and heading of the point-defence shots in flight.
    fn defence_shots(combat: &Combat) -> Vec<(Vec2, f32)> {
        combat
            .shots()
            .iter()
            .filter(|shot| shot.weapon.guidance == Guidance::PointDefence)
            .map(|shot| (start(shot), shot.heading))
            .collect()
    }

    #[test]
    fn point_defence_fires_from_every_other_exit_at_the_missile() {
        let mut combat = Combat::default();
        let mut ships = defended();
        missile_in_flight(&mut combat, &mut ships);
        tick(&mut combat, &mut ships, &NovaDisable);
        let fired = defence_shots(&combat);
        assert_eq!(fired.len(), 1, "{:?}", combat.shots());
        let (at, heading) = fired[0];
        assert!(
            close_to(at, Vec2::new(20.0, 0.0)),
            "the second exit: {at:?}"
        );
        let bearing = trig::intercept_angle(at, Vec2::new(150.0, 100.0));
        assert_eq!(heading, f32::from(bearing), "the table's (@0x39790)");
        assert_eq!(
            ships[0].armament.exit_cursors().next(ExitKind::Turret),
            2,
            "stepped twice"
        );
    }

    #[test]
    fn by_the_other_reading_point_defence_steps_its_exits_once() {
        let mut combat = Combat::default();
        combat.set_pd_exit_step(RuleSource::Bible);
        let mut ships = defended();
        missile_in_flight(&mut combat, &mut ships);
        tick(&mut combat, &mut ships, &NovaDisable);
        let fired = defence_shots(&combat);
        assert!(close_to(fired[0].0, Vec2::new(10.0, 0.0)), "{fired:?}");
        assert_eq!(ships[0].armament.exit_cursors().next(ExitKind::Turret), 1);
    }

    #[test]
    fn the_players_point_defence_steps_a_shared_exit_cursor_before_its_trigger() {
        // `_HandlePlayer` runs `_HandleShipPointDefense` @0x68c74 before
        // `_FirePlayerWeapon` @0x6ab55, so point defence steps the turret
        // exits' cursor first (`_ModifyShotStartPosition`).
        let turret_gun = WeaponRecord {
            exit_type: 1,
            ..gun()
        };
        for (source, defence_from, gun_from, cursor) in [
            (
                RuleSource::Engine,
                Vec2::new(20.0, 0.0),
                Vec2::new(30.0, 0.0),
                3,
            ),
            (
                RuleSource::Bible,
                Vec2::new(10.0, 0.0),
                Vec2::new(20.0, 0.0),
                2,
            ),
        ] {
            let mut combat = Combat::default();
            combat.set_pd_exit_step(source);
            let mut ships = defended();
            ships[0].id = ShipRef::Player;
            ships[0].fleet = ShipRef::Player;
            ships[0].target = None;
            ships[1].target = Some(ShipRef::Player);
            ships[0].armament = Armament::new([
                (WeaponSpec::new(&exit_defence()), 1),
                (WeaponSpec::new(&turret_gun), 1),
            ]);
            // B's missile in flight first, the player quiet.
            ships[0].trigger = Trigger {
                point_defence: false,
                ..QUIET
            };
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(combat.shots().len(), 1, "{source:?}: the missile");
            ships[1].trigger = QUIET;
            ships[0].trigger = PRIMARY_HELD;
            tick(&mut combat, &mut ships, &NovaDisable);
            let fired = defence_shots(&combat);
            assert_eq!(fired.len(), 1, "{source:?}: {:?}", combat.shots());
            let at = fired[0].0;
            assert!(
                close_to(at, defence_from),
                "{source:?}: point defence from {at:?}"
            );
            let guns: Vec<Vec2> = combat
                .shots()
                .iter()
                .filter(|shot| shot.firer == ShipRef::Player && shot.weapon.id == WeaponId(128))
                .map(start)
                .collect();
            assert_eq!(guns.len(), 1, "{source:?}: {guns:?}");
            assert!(
                close_to(guns[0], gun_from),
                "{source:?}: the gun from {:?}",
                guns[0]
            );
            assert_eq!(
                ships[0].armament.exit_cursors().next(ExitKind::Turret),
                cursor,
                "{source:?}"
            );
        }
    }

    #[test]
    fn by_the_bible_an_exit_is_compressed_by_the_heading() {
        let raw = ShipExits {
            up_compress: (100, 50),
            down_compress: (50, 100),
            frames: 36,
            points: [[(0, -10); 4], [(0, 0); 4], [(0, 0); 4], [(0, 0); 4]],
            ..ShipExits::default()
        };
        for (source, behind) in [(RuleSource::Engine, 10.0), (RuleSource::Bible, 5.0)] {
            let mut combat = Combat::default();
            combat.set_exit_compression(source);
            let mut ships = [Ship::at(1, 0.0, 0.0)
                .armed(gun())
                .facing(0.0)
                .of(hull_of(raw))];
            let at = next_start(&mut combat, &mut ships);
            assert!(close_to(at, Vec2::new(0.0, behind)), "{source:?}: {at:?}");
        }
        assert_eq!(Compression::from(RuleSource::Bible), Compression::ByHeading);
    }

    // Knockback and tractor beams (`_DamageShip` @0x3a9b8-0x3ab84,
    // `_HandleBeams` @0x311d0-0x31450; see the `impact` module).

    /// A tick among `ships`, each pushed ship's velocity then set from the
    /// fight's shoves as the session sets them; gives the shoves.
    fn tick_shoving(combat: &mut Combat, ships: &mut [Ship]) -> Vec<Shove> {
        tick(combat, ships, &NovaDisable);
        let shoves = combat.take_shoves();
        for shove in &shoves {
            if let Some(ship) = ships.iter_mut().find(|ship| ship.id == shove.ship) {
                ship.state.velocity = shove.velocity;
            }
        }
        shoves
    }

    /// `ship` of `mass`, with a class and top speed of 100.
    fn of_mass(ship: Ship, mass: f32) -> Ship {
        let mut ship = ship;
        ship.hull.mass = mass;
        ship.class_speed = 100.0;
        ship.top_speed = 100.0;
        ship
    }

    /// The push `impact` from `from` gives a ship of `mass` at rest at `at`,
    /// under its caps: `impact` / `mass` along the whole-degree bearing.
    fn push_of(from: Vec2, at: Vec2, impact: f32, mass: f32) -> Vec2 {
        crate::flight::facing(f32::from(crate::trig::intercept_angle(from, at))) * (impact / mass)
    }

    /// The velocity of each of `shoves` on `ship`, in order.
    fn shoves_on(shoves: &[Shove], ship: ShipRef) -> Vec<Vec2> {
        shoves
            .iter()
            .filter(|shove| shove.ship == ship)
            .map(|shove| shove.velocity)
            .collect()
    }

    fn close_by(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    /// Ticks `ships` until `ships[0]`'s one shot or beam is gone, at most
    /// `ticks`, its trigger let go after the first: every shove and the
    /// first explosion's place.
    fn shoves_after_one(ships: &mut [Ship], ticks: u32) -> (Vec<Shove>, Option<Vec2>) {
        let mut combat = Combat::default();
        let mut shoves = Vec::new();
        let mut at = None;
        for _ in 0..ticks {
            shoves.extend(tick_shoving(&mut combat, ships));
            ships[0].trigger = QUIET;
            at = at.or_else(|| {
                combat.take_events().iter().find_map(|event| match *event {
                    CombatEvent::Exploded { at, .. } => Some(at),
                    _ => None,
                })
            });
        }
        (shoves, at)
    }

    /// The blaster with `impact` and an explosion.
    fn pushing(impact: i16) -> WeaponRecord {
        WeaponRecord {
            impact,
            explod_type: 0,
            ..blaster()
        }
    }

    #[test]
    fn a_direct_hit_pushes_its_ship_by_impact_over_mass_from_where_it_met() {
        let target = of_mass(Ship::at(2, 100.0, 10.0), 10.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(pushing(50)), target];
        let (shoves, at) = shoves_after_one(&mut ships, 12);
        let at = at.expect("it hit");
        let pushed = shoves_on(&shoves, B);
        let expected = push_of(at, Vec2::new(100.0, 10.0), 50.0, 10.0);
        assert_eq!(pushed.len(), 1, "{shoves:?}");
        assert!(close_by(pushed[0], expected), "{pushed:?}, {expected:?}");
        let from_firer = push_of(Vec2::ZERO, Vec2::new(100.0, 10.0), 50.0, 10.0);
        assert!(!close_by(pushed[0], from_firer), "not from the firer");
        assert_eq!(ships[1].state.velocity, pushed[0]);
        assert!(ships[1].reserves.shield.now < 10.0, "hurt too");
        assert_eq!((ships[0].tractor, ships[1].tractor), (None, None));
    }

    #[test]
    fn a_decayed_shot_still_pushes_by_its_whole_impact() {
        let mut combat = Combat::default();
        combat.set_decay(DecayStep::EveryDecayPlusOne);
        let slug = WeaponRecord {
            impact: 50,
            ..slug(0x0020)
        };
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(slug),
            of_mass(Ship::at(2, 95.0, 0.0), 10.0),
        ];
        let mut shoves = tick_shoving(&mut combat, &mut ships);
        ships[0].trigger = QUIET;
        while !combat.shots().is_empty() {
            shoves.extend(tick_shoving(&mut combat, &mut ships));
        }
        assert_eq!(ships[1].reserves.armor.now, 24.0, "the damage decayed");
        let pushed = shoves_on(&shoves, B);
        assert_eq!(pushed.len(), 1, "{shoves:?}");
        assert!(close_by(pushed[0], Vec2::new(5.0, 0.0)), "{pushed:?}");
    }

    #[test]
    fn a_hit_blast_pushes_each_ship_it_reaches_from_its_centre_but_not_the_ship_hit() {
        let shell = WeaponRecord {
            blast_radius: 30,
            ..pushing(50)
        };
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(shell),
            of_mass(Ship::at(2, 100.0, 0.0), 10.0),
            of_mass(Ship::at(3, 100.0, 25.0), 10.0),
        ];
        let (shoves, at) = shoves_after_one(&mut ships, 12);
        let at = at.expect("it hit");
        let hit = shoves_on(&shoves, B);
        assert_eq!(hit.len(), 1, "the direct hit alone: {shoves:?}");
        assert!(close_by(
            hit[0],
            push_of(at, Vec2::new(100.0, 0.0), 50.0, 10.0)
        ));
        let blasted = shoves_on(&shoves, C);
        assert_eq!(blasted.len(), 1, "{shoves:?}");
        let expected = push_of(at, Vec2::new(100.0, 25.0), 50.0, 10.0);
        assert!(close_by(blasted[0], expected), "{blasted:?}, {expected:?}");
        assert!(blasted[0].y > 0.0, "away from the centre");
    }

    #[test]
    fn an_expiry_blast_pushes_from_where_its_shot_ended() {
        let flak = WeaponRecord {
            count: 3,
            speed: 1000,
            explod_type: 2,
            blast_radius: 40,
            flags: 0x8000,
            impact: 50,
            ..blaster()
        };
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(flak),
            of_mass(Ship::at(3, 30.0, 35.0), 10.0),
        ];
        let (shoves, at) = shoves_after_one(&mut ships, 6);
        let at = at.expect("it detonated");
        let pushed = shoves_on(&shoves, C);
        assert_eq!(pushed.len(), 1, "{shoves:?}");
        let expected = push_of(at, Vec2::new(30.0, 35.0), 50.0, 10.0);
        assert!(close_by(pushed[0], expected), "{pushed:?}, {expected:?}");
    }

    /// The laser with `impact`.
    fn beam_of(impact: i16) -> WeaponRecord {
        WeaponRecord { impact, ..laser() }
    }

    #[test]
    fn a_beam_pushes_from_its_start_on_every_tick_it_hits_compounding() {
        let mut combat = Combat::default();
        let at = Vec2::new(100.0, 10.0);
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(beam_of(50)),
            of_mass(Ship::at(2, at.x, at.y), 100.0),
        ];
        let step = push_of(Vec2::ZERO, at, 50.0, 100.0);
        let mut ends = Vec::new();
        let mut velocities = Vec::new();
        for _ in 0..5 {
            let shoves = tick_shoving(&mut combat, &mut ships);
            ships[0].trigger = QUIET;
            ends.extend(combat.beams().iter().map(|beam| beam.end));
            velocities.extend(shoves_on(&shoves, B));
        }
        assert_eq!(velocities.len(), 3, "a push a hit tick: {velocities:?}");
        for (k, velocity) in velocities.iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let hits = (k + 1) as f32;
            assert!(close_by(*velocity, step * hits), "{k}: {velocity:?}");
        }
        let end = ends[0];
        assert!(end.y.abs() < 1e-3, "it ends on its ray: {end:?}");
        assert!(
            !close_by(step, push_of(end, at, 50.0, 100.0)),
            "not from its end"
        );
        assert_eq!((ships[0].tractor, ships[1].tractor), (None, None));
    }

    /// The x velocity a ship of mass 1 and top speed 10 at (100, 0) is
    /// left with by one hit of `weapon` from a ship at the centre facing
    /// it, its afterburner widening its shots' cap as `shots` says and its
    /// beams' as `beams` says.
    fn knocked_with_caps(weapon: WeaponRecord, shots: bool, beams: bool) -> f32 {
        let mut target = of_mass(Ship::at(2, 100.0, 0.0), 1.0);
        target.top_speed = 10.0;
        target.afterburner = shots;
        target.afterburner_beam = beams;
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(weapon), target];
        let (shoves, _) = shoves_after_one(&mut ships, 12);
        let pushed = shoves_on(&shoves, B);
        assert!(!pushed.is_empty(), "it hit");
        pushed[0].x
    }

    #[test]
    fn a_beams_push_reads_its_own_afterburner_cap_and_a_shots_the_other() {
        let burnt = crate::afterburner::afterburner_top(10.0);
        let beam = || beam_of(50);
        assert_eq!(knocked_with_caps(beam(), true, false), 10.0, "plain");
        assert_eq!(knocked_with_caps(beam(), false, true), burnt, "widened");
        let shot = || pushing(50);
        assert_eq!(knocked_with_caps(shot(), true, false), burnt, "widened");
        assert_eq!(knocked_with_caps(shot(), false, true), 10.0, "plain");
    }

    #[test]
    fn a_missile_lock_pushes_nothing() {
        let heavy = WeaponRecord {
            impact: 500,
            ..missile()
        };
        let mut combat = Combat::default();
        let mut ships = [player_with(heavy), of_mass(far_b(), 10.0)];
        let mut shoves = Vec::new();
        let mut locks = 0;
        for _ in 0..40 {
            shoves.extend(tick_shoving(&mut combat, &mut ships));
            ships[0].trigger = QUIET;
            locks += combat
                .take_strikes()
                .iter()
                .filter(|strike| strike.lock)
                .count();
        }
        assert_eq!(locks, 1);
        assert_eq!(shoves, []);
    }

    #[test]
    fn a_breaking_up_ship_is_pushed_by_a_hit() {
        let mut dying = of_mass(Ship::at(2, 100.0, 0.0), 10.0);
        dying.reserves.armor.now = -1.0;
        dying.condition = Condition::Dying {
            ticks_left: 50,
            starting: false,
        };
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(pushing(50)), dying];
        let (shoves, _) = shoves_after_one(&mut ships, 12);
        let pushed = shoves_on(&shoves, B);
        assert_eq!(pushed.len(), 1, "{shoves:?}");
        assert!(close_by(pushed[0], Vec2::new(5.0, 0.0)), "{pushed:?}");
    }

    #[test]
    fn an_escape_pod_reached_by_a_blast_is_not_pushed() {
        let shell = WeaponRecord {
            blast_radius: 30,
            ..pushing(50)
        };
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(shell),
            of_mass(Ship::at(4, 100.0, 0.0), 10.0),
            pod(of_mass(Ship::at(3, 100.0, 25.0), 10.0), ESCAPE_POD_SHIP),
        ];
        let (shoves, _) = shoves_after_one(&mut ships, 12);
        assert_eq!(shoves_on(&shoves, ShipRef::Npc(NpcId(4))).len(), 1);
        assert_eq!(shoves_on(&shoves, C), [], "the pod");
    }

    #[test]
    fn a_ship_whose_jump_is_under_way_is_hit_directly_but_not_pushed() {
        let mut jumping = of_mass(Ship::at(2, 100.0, 0.0), 10.0);
        jumping.jumping = true;
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(pushing(50)), jumping];
        let (shoves, _) = shoves_after_one(&mut ships, 12);
        assert!(ships[1].reserves.shield.now < 10.0, "hit");
        assert_eq!(shoves, []);
    }

    #[test]
    fn two_hits_on_a_ship_in_a_tick_compound_in_order() {
        let mut combat = Combat::default();
        // Two shots, 20 above and below B's centre, meet it on one tick.
        let mut wide = of_mass(Ship::at(2, 100.0, 0.0), 10.0);
        wide.hull.hit_radius = 50.0;
        let mut ships = [
            Ship::at(1, 0.0, -20.0).armed(pushing(50)),
            Ship::at(3, 0.0, 20.0).armed(pushing(50)),
            wide,
        ];
        let mut shoves = Vec::new();
        for _ in 0..12 {
            shoves.extend(tick_shoving(&mut combat, &mut ships));
            ships[0].trigger = QUIET;
            ships[1].trigger = QUIET;
        }
        let pushed = shoves_on(&shoves, B);
        assert_eq!(pushed.len(), 2, "{shoves:?}");
        assert!(pushed[0].y > 0.0, "down, from above: {pushed:?}");
        assert!(
            close_by(pushed[1], Vec2::new(pushed[0].x * 2.0, 0.0)),
            "the second added to the first: {pushed:?}"
        );
        assert_eq!(ships[2].state.velocity, pushed[1]);
    }

    #[test]
    fn a_negative_direct_hit_pulls_only_from_a_firer_off_both_axes_and_a_blast_always() {
        // Fired down the diagonal, 100 off on each axis: pulled toward
        // where it met.
        let diagonal = of_mass(Ship::at(2, 100.0, 100.0), 10.0);
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(pushing(-50)).facing(135.0),
            diagonal,
        ];
        let (shoves, at) = shoves_after_one(&mut ships, 12);
        let pulled = shoves_on(&shoves, B);
        assert_eq!(pulled.len(), 1, "{shoves:?}");
        let at = at.expect("it hit");
        assert!(close_by(
            pulled[0],
            push_of(at, Vec2::new(100.0, 100.0), -50.0, 10.0)
        ));
        assert!(
            pulled[0].x < 0.0 && pulled[0].y < 0.0,
            "toward it: {pulled:?}"
        );
        assert_eq!(
            (ships[0].tractor, ships[1].tractor),
            (None, None),
            "no tractor"
        );
        // Dead ahead, on the firer's y: the hit pulls nothing, but its
        // blast, no direct hit, pulls the ship beside it.
        let shell = WeaponRecord {
            blast_radius: 30,
            ..pushing(-50)
        };
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(shell),
            of_mass(Ship::at(2, 100.0, 0.0), 10.0),
            of_mass(Ship::at(3, 100.0, 25.0), 10.0),
        ];
        let (shoves, _) = shoves_after_one(&mut ships, 12);
        assert_eq!(shoves_on(&shoves, B), [], "gated");
        let blasted = shoves_on(&shoves, C);
        assert_eq!(blasted.len(), 1, "{shoves:?}");
        assert!(blasted[0].y < 0.0, "toward the centre: {blasted:?}");
    }

    /// A tractor beam: the laser at `Impact` -5.
    fn tractor_beam() -> WeaponRecord {
        beam_of(-5)
    }

    /// A tick of A, of `firer_mass` at the centre facing `heading`, firing a
    /// tractor beam at B of `target_mass` at `at`: the shoves.
    fn tractored(ships: &mut [Ship; 2]) -> Vec<Shove> {
        let mut combat = Combat::default();
        tick_shoving(&mut combat, ships)
    }

    /// A, of `mass`, at the centre facing `heading`, its tractor beam
    /// held.
    fn tractor_firer(mass: f32, heading: f32) -> Ship {
        of_mass(Ship::at(1, 0.0, 0.0).armed(tractor_beam()), mass).facing(heading)
    }

    #[test]
    fn a_tractor_slips_off_a_massless_or_planet_type_target_which_it_still_hurts() {
        let massless = of_mass(Ship::at(2, 100.0, 100.0), 0.0);
        let mut planet = of_mass(Ship::at(2, 100.0, 100.0), 100.0);
        planet.hull.planet_type = true;
        // Only a beam of `Flags` 0x0400 may hit a planet-type ship
        // (`_HandleBeams` @0x30d34-0x30d82).
        let mut planet_tractor = tractor_firer(200.0, 135.0);
        planet_tractor.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                flags: planet_type::BEAM_PLANET_FLAG,
                ..tractor_beam()
            }),
            1,
        )]);
        for (firer, target) in [
            (tractor_firer(200.0, 135.0), massless),
            (planet_tractor, planet),
        ] {
            let mut ships = [firer, target];
            let shoves = tractored(&mut ships);
            assert_eq!(shoves, []);
            assert_eq!((ships[0].tractor, ships[1].tractor), (None, None));
            assert_eq!(ships[1].reserves.shield.now, 5.0, "hurt");
        }
    }

    #[test]
    fn a_beam_of_no_impact_neither_holds_nor_pulls() {
        let firer = of_mass(Ship::at(1, 0.0, 0.0).armed(beam_of(0)), 200.0).facing(90.0);
        let mut ships = [firer, of_mass(Ship::at(2, 100.0, 0.0), 100.0)];
        assert_eq!(tractored(&mut ships), []);
        assert_eq!((ships[0].tractor, ships[1].tractor), (None, None));
        assert!(ships[1].reserves.shield.now < 10.0, "hit");
    }

    #[test]
    fn a_firer_of_three_quarters_its_targets_mass_holds_it() {
        let mut ships = [
            tractor_firer(75.0, 90.0),
            of_mass(Ship::at(2, 100.0, 0.0), 100.0),
        ];
        tractored(&mut ships);
        assert_eq!(ships[1].tractor, Some(TractorLock::by(A)));
        assert_eq!(ships[0].tractor, None);
        let mut ships = [
            tractor_firer(74.0, 90.0),
            of_mass(Ship::at(2, 100.0, 0.0), 100.0),
        ];
        tractored(&mut ships);
        assert_eq!(ships[1].tractor, None);
        assert_eq!(
            ships[0].tractor,
            Some(TractorLock::by(A)),
            "it holds itself"
        );
    }

    #[test]
    fn a_heavy_tractor_pulls_its_target_toward_its_start_only_off_both_of_its_firers_axes() {
        let at = Vec2::new(100.0, 100.0);
        let mut ships = [
            tractor_firer(200.0, 135.0),
            of_mass(Ship::at(2, at.x, at.y), 100.0),
        ];
        let shoves = tractored(&mut ships);
        let pulled = shoves_on(&shoves, B);
        assert_eq!(pulled.len(), 1, "{shoves:?}");
        assert!(close_by(pulled[0], push_of(Vec2::ZERO, at, -5.0, 100.0)));
        assert!(
            pulled[0].x < 0.0 && pulled[0].y < 0.0,
            "toward it: {pulled:?}"
        );
        assert_eq!(shoves_on(&shoves, A), [], "the firer stays");
        // Dead ahead, on the firer's y: held, not pulled.
        let mut ships = [
            tractor_firer(200.0, 90.0),
            of_mass(Ship::at(2, 100.0, 0.0), 100.0),
        ];
        assert_eq!(tractored(&mut ships), []);
        assert_eq!(ships[1].tractor, Some(TractorLock::by(A)));
        // A ship whose jump is under way: held, not pulled.
        let mut jumping = of_mass(Ship::at(2, at.x, at.y), 100.0);
        jumping.jumping = true;
        let mut ships = [tractor_firer(200.0, 135.0), jumping];
        assert_eq!(tractored(&mut ships), []);
        assert_eq!(ships[1].tractor, Some(TractorLock::by(A)));
    }

    #[test]
    fn a_heavy_tractor_takes_its_targets_lock_from_an_earlier_holder_and_touches_it() {
        // The original writes T +0xc8dc = S and T +0xb4 = TickCount
        // whatever T held (@0x3125e-0x31272).
        let mut held = of_mass(Ship::at(2, 100.0, 0.0), 100.0);
        held.tractor = Some(TractorLock {
            holder: C,
            ticks_since_touch: 7,
        });
        let mut ships = [tractor_firer(200.0, 90.0), held];
        tractored(&mut ships);
        assert_eq!(ships[1].tractor, Some(TractorLock::by(A)));
        assert_eq!(ships[0].tractor, None);
    }

    #[test]
    fn a_light_firer_holds_itself_unless_held_and_is_touched_either_way() {
        let mut held = tractor_firer(6.0, 90.0);
        held.tractor = Some(TractorLock {
            holder: C,
            ticks_since_touch: 7,
        });
        let mut ships = [held, of_mass(Ship::at(2, 100.0, 0.0), 15.0)];
        tractored(&mut ships);
        assert_eq!(ships[0].tractor, Some(TractorLock::by(C)));
        assert_eq!(ships[1].tractor, None);
    }

    #[test]
    fn a_light_firer_pulls_itself_toward_its_target_and_leaves_the_target_alone() {
        let mut ships = [
            tractor_firer(6.0, 90.0),
            of_mass(Ship::at(2, 100.0, 0.0), 15.0),
        ];
        let shoves = tractored(&mut ships);
        assert_eq!(shoves_on(&shoves, B), [], "the target");
        let pulled = shoves_on(&shoves, A);
        assert_eq!(pulled.len(), 1, "{shoves:?}");
        assert!(close_by(pulled[0], Vec2::new(5.0 / 6.0, 0.0)), "{pulled:?}");
        assert_eq!(ships[0].state.velocity, pulled[0]);
        assert_eq!(ships[0].tractor, Some(TractorLock::by(A)));
        assert_eq!(ships[1].reserves.shield.now, 5.0, "hurt");
        // Clamped to its top speed.
        let mut slow = tractor_firer(1.0, 90.0);
        slow.top_speed = 0.5;
        let mut ships = [slow, of_mass(Ship::at(2, 100.0, 0.0), 15.0)];
        let pulled = shoves_on(&tractored(&mut ships), A);
        assert!(close_by(pulled[0], Vec2::new(0.5, 0.0)), "{pulled:?}");
        // Off both of the firer's axes, where a hit could pull it, the
        // target is still left alone: the beam's Impact is spent.
        let mut ships = [
            tractor_firer(6.0, 135.0),
            of_mass(Ship::at(2, 100.0, 100.0), 15.0),
        ];
        let shoves = tractored(&mut ships);
        assert_eq!(shoves_on(&shoves, B), [], "the target, diagonal");
        assert_eq!(shoves_on(&shoves, A).len(), 1, "{shoves:?}");
    }

    #[test]
    fn a_tractor_still_holds_or_pulls_along_a_class_after_the_escape_pod_it_leaves_unharmed() {
        // A beam passes through the pod alone (@0x30dd2), and
        // `_HandleBeams`' tractor branch (@0x311d0-0x31450) tests no class
        // before `_DamageShip` (@0x316a1) does nothing to the pod or any
        // class after it (@0x3a87c).
        let after = ShipId(ESCAPE_POD_SHIP.0 + 1);
        let unharmed = left(&Ship::at(2, 0.0, 0.0));
        // Off both of its axes, a heavy firer would pull an ordinary
        // target; this one it holds and leaves where it is.
        let mut ships = [
            tractor_firer(200.0, 135.0),
            pod(of_mass(Ship::at(2, 100.0, 100.0), 100.0), after),
        ];
        assert_eq!(tractored(&mut ships), []);
        assert_eq!(ships[1].tractor, Some(TractorLock::by(A)));
        assert_eq!(ships[0].tractor, None);
        assert_eq!(left(&ships[1]), unharmed);
        // A light firer pulls itself toward it and holds itself.
        let mut ships = [
            tractor_firer(6.0, 90.0),
            pod(of_mass(Ship::at(2, 100.0, 0.0), 15.0), after),
        ];
        let shoves = tractored(&mut ships);
        assert_eq!(shoves_on(&shoves, B), [], "the target");
        let pulled = shoves_on(&shoves, A);
        assert_eq!(pulled.len(), 1, "{shoves:?}");
        assert!(close_by(pulled[0], Vec2::new(5.0 / 6.0, 0.0)), "{pulled:?}");
        assert_eq!(ships[0].tractor, Some(TractorLock::by(A)));
        assert_eq!(ships[1].tractor, None);
        assert_eq!(left(&ships[1]), unharmed);
    }

    #[test]
    fn a_light_firer_is_not_pulled_near_its_target_on_both_axes_massless_or_planet_type() {
        let near = of_mass(Ship::at(2, 30.0, 0.0), 15.0);
        let mut ships = [tractor_firer(6.0, 90.0), near];
        assert_eq!(tractored(&mut ships), []);
        assert_eq!(
            ships[0].tractor,
            Some(TractorLock::by(A)),
            "held all the same"
        );
        let mut planet = tractor_firer(6.0, 90.0);
        planet.hull.planet_type = true;
        for firer in [tractor_firer(0.0, 90.0), planet] {
            let mut ships = [firer, of_mass(Ship::at(2, 100.0, 0.0), 15.0)];
            assert_eq!(tractored(&mut ships), []);
            assert_eq!(ships[0].tractor, Some(TractorLock::by(A)));
        }
    }

    #[test]
    fn a_tractor_beam_whose_firer_has_left_the_fight_neither_holds_nor_pulls() {
        let mut combat = Combat::default();
        let mut ships = vec![
            tractor_firer(200.0, 135.0),
            of_mass(Ship::at(2, 100.0, 100.0), 100.0),
        ];
        tick_shoving(&mut combat, &mut ships);
        let mut target = ships.remove(1);
        target.tractor = None;
        target.state.velocity = Vec2::ZERO;
        let mut ships = [target];
        let shield = ships[0].reserves.shield.now;
        assert_eq!(tick_shoving(&mut combat, &mut ships), []);
        assert_eq!(ships[0].tractor, None);
        assert_eq!(
            ships[0].reserves.shield.now,
            shield - 5.0,
            "hurt all the same"
        );
    }

    // Ionization (`_IonizeShip` @0x8409-0x84d1; see the `ion` module).

    /// `record` ionizing by `ionization` in `colour`.
    fn ionizing(record: WeaponRecord, ionization: i16, colour: u32) -> WeaponRecord {
        WeaponRecord {
            ionization,
            ionize_color: colour,
            ..record
        }
    }

    /// The dose `ionization` gives a ship `off` from a blast's centre of
    /// `radius`, by the `ion` module's own reckoning.
    fn share(ionization: i32, radius: f32, off: Vec2) -> f32 {
        let mut ion = Ion::default();
        ion.take(
            ion::Dose {
                ionization,
                colour: 0,
                blast: Some(ion::Burst {
                    centre: Vec2::ZERO,
                    radius,
                }),
            },
            off,
        );
        ion.charge
    }

    const GREEN: u32 = 0x0000_ff00;

    #[test]
    fn a_contact_hit_ionizes_its_ship_by_the_whole_ionization_in_its_colour() {
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(ionizing(blaster(), 30, GREEN)),
            Ship::at(2, 100.0, 0.0),
        ];
        let (_, at) = shoves_after_one(&mut ships, 12);
        assert!(at.is_none(), "no blast, no explosion");
        assert_eq!(
            ships[1].ion,
            Ion {
                charge: 30.0,
                colour: GREEN,
                ..Ion::default()
            }
        );
        assert_eq!(ships[0].ion, Ion::default(), "not its firer");
    }

    #[test]
    fn a_proximity_hit_ionizes_its_ship_by_the_whole_ionization() {
        // B sits 25 pixels off the bolt's path, beyond its hit radius; the
        // bolt's sixth move ends at 90, 10 by 25 from it, within its fuse's
        // reach of trunc(20 + 32 x 0.333) = 30.
        let fused = WeaponRecord {
            prox_radius: 20,
            ..ionizing(blaster(), 30, GREEN)
        };
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(fused), Ship::at(2, 100.0, 25.0)];
        let mut strikes = Vec::new();
        for _ in 0..12 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
            strikes.extend(combat.take_strikes());
        }
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        const { assert!(DEFAULT_HIT_RADIUS < 25.0, "not by contact") };
        assert_eq!(ships[1].ion.charge, 30.0);
    }

    #[test]
    fn a_decayed_shot_still_ionizes_by_its_whole_ionization() {
        let mut combat = Combat::default();
        combat.set_decay(DecayStep::EveryDecayPlusOne);
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(ionizing(slug(0x0020), 50, GREEN)),
            Ship::at(2, 95.0, 0.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
        while !combat.shots().is_empty() {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        assert_eq!(ships[1].reserves.armor.now, 24.0, "the damage decayed");
        assert_eq!(ships[1].ion.charge, 50.0);
    }

    #[test]
    fn a_hit_blast_ionizes_the_ship_hit_once_and_each_ship_in_its_circle_by_its_share() {
        let shell = WeaponRecord {
            blast_radius: 30,
            ..ionizing(pushing(0), 100, GREEN)
        };
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(shell),
            Ship::at(2, 100.0, 0.0),
            Ship::at(3, 100.0, 25.0),
            Ship::at(4, 100.0, -28.0),
        ];
        let (_, at) = shoves_after_one(&mut ships, 12);
        let at = at.expect("it hit");
        assert_eq!(ships[1].ion.charge, 100.0, "the direct hit alone");
        let near = Vec2::new(100.0, 25.0) - at;
        let expected = share(100, 30.0, near);
        assert!(expected > 0.0 && expected < 100.0, "{near:?}: {expected}");
        assert_eq!(
            ships[2].ion,
            Ion {
                charge: expected,
                colour: GREEN,
                ..Ion::default()
            }
        );
        let corner = Vec2::new(100.0, -28.0) - at;
        assert!(
            corner.x * corner.x + corner.y * corner.y > 900.0,
            "{corner:?}"
        );
        assert!(ships[3].reserves.shield.now < 10.0, "in the blast's square");
        assert_eq!(ships[3].ion, Ion::default(), "outside its circle");
    }

    #[test]
    fn an_expiry_blast_ionizes_by_the_share_from_where_its_shot_ended() {
        let flak = WeaponRecord {
            count: 3,
            speed: 1000,
            explod_type: 2,
            blast_radius: 40,
            flags: 0x8000,
            ..ionizing(blaster(), 80, GREEN)
        };
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(flak), Ship::at(3, 30.0, 35.0)];
        let (_, at) = shoves_after_one(&mut ships, 6);
        let at = at.expect("it detonated");
        let expected = share(80, 40.0, Vec2::new(30.0, 35.0) - at);
        assert!(expected > 0.0 && expected < 80.0, "{at:?}: {expected}");
        assert_eq!(ships[1].ion.charge, expected);
    }

    #[test]
    fn a_beam_ionizes_by_the_whole_ionization_on_every_tick_it_hits() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(ionizing(laser(), 4, 0x00ff_00ff)),
            Ship::at(2, 100.0, 0.0),
        ];
        ships[1].reserves.shield = Gauge::full(100.0);
        let mut charges = Vec::new();
        for _ in 0..5 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
            charges.push(ships[1].ion.charge);
        }
        assert_eq!(charges, [4.0, 8.0, 12.0, 12.0, 12.0]);
        assert_eq!(ships[1].ion.colour, 0x00ff_00ff);
    }

    #[test]
    fn a_tractor_beam_slipping_off_a_massless_target_still_ionizes_it() {
        let mut ships = [
            of_mass(
                Ship::at(1, 0.0, 0.0).armed(ionizing(tractor_beam(), 7, GREEN)),
                200.0,
            )
            .facing(135.0),
            of_mass(Ship::at(2, 100.0, 100.0), 0.0),
        ];
        assert_eq!(tractored(&mut ships), []);
        assert_eq!(ships[1].ion.charge, 7.0);
    }

    #[test]
    fn a_missile_lock_ionizes_nothing() {
        let mut combat = Combat::default();
        let mut ships = [player_with(ionizing(missile(), 100, GREEN)), far_b()];
        let mut locks = 0;
        for _ in 0..40 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
            locks += combat
                .take_strikes()
                .iter()
                .filter(|strike| strike.lock)
                .count();
        }
        assert_eq!(locks, 1);
        assert_eq!(ships[1].ion, Ion::default());
    }

    #[test]
    fn an_escape_pod_reached_by_a_blast_is_ionized_though_not_hurt() {
        let shell = WeaponRecord {
            blast_radius: 30,
            ..ionizing(pushing(0), 100, GREEN)
        };
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(shell),
            Ship::at(4, 100.0, 0.0),
            pod(Ship::at(3, 100.0, 10.0), ESCAPE_POD_SHIP),
        ];
        let (_, at) = shoves_after_one(&mut ships, 12);
        let at = at.expect("it hit");
        assert_eq!(ships[2].reserves.shield.now, 10.0, "unhurt");
        let expected = share(100, 30.0, Vec2::new(100.0, 10.0) - at);
        assert!(expected > 0.0, "{at:?}");
        assert_eq!(ships[2].ion.charge, expected);
    }

    #[test]
    fn a_shot_on_the_player_ionizes_the_player() {
        let player = Ship {
            id: ShipRef::Player,
            fleet: ShipRef::Player,
            ..Ship::at(0, 100.0, 0.0)
        };
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(ionizing(blaster(), 30, GREEN)),
            player,
        ];
        shoves_after_one(&mut ships, 12);
        assert_eq!(ships[1].ion.charge, 30.0);
    }

    // The turret miss (`_SpawnShot` @0x3e89e-0x3e963, `_ShotCanHitShip`
    // @0x448ad-0x448d5).

    /// A tick among `ships` drawing on `chance`.
    fn tick_drawing(combat: &mut Combat, ships: &mut [Ship], chance: &mut dyn Chance) {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        combat.tick(
            &mut fighters,
            &Arsenal::default(),
            &Governments::default(),
            Rules::default(),
            Field::default(),
            Scenery::default(),
            chance,
        );
    }

    /// A hull of turret miss class `class`.
    fn of_class(class: u32) -> HullSpec {
        HullSpec {
            miss_class: class,
            ..HullSpec::default()
        }
    }

    /// The miss roll of the shot `ship` fires at B, at (0, -300), with
    /// `record`, on the draws `draws`, and the ranges asked.
    fn rolled_by(ship: Ship, record: WeaponRecord, draws: &[u32]) -> (Option<u32>, Vec<u32>) {
        let mut combat = Combat::default();
        let mut ships = [ship.armed(record).targeting(B), Ship::at(2, 0.0, -300.0)];
        let mut chance = nova_sim_base::fixture::Draws::of(draws);
        tick_drawing(&mut combat, &mut ships, &mut chance);
        let shot = combat.shots().first().expect("a shot");
        (shot.miss_roll, chance.asked)
    }

    #[test]
    fn a_turret_shot_rolls_its_miss_in_its_firers_class() {
        for (class, asked) in [(100, 100), (90, 90), (80, 80)] {
            let firer = Ship::at(1, 0.0, 0.0).of(of_class(class));
            assert_eq!(
                rolled_by(firer, turret(4), &[42]),
                (Some(43), vec![asked]),
                "class {class}"
            );
        }
        let firer = Ship::at(1, 0.0, 0.0).of(of_class(100));
        assert_eq!(rolled_by(firer, turret(4), &[80]).0, Some(81));
    }

    #[test]
    fn the_players_turret_rolls_in_the_players_class() {
        let player = Ship {
            id: ShipRef::Player,
            fleet: ShipRef::Player,
            ..Ship::at(0, 0.0, 0.0).of(of_class(90))
        };
        assert_eq!(rolled_by(player, turret(4), &[89]), (Some(90), vec![90]));
    }

    #[test]
    fn only_a_turret_or_point_defence_shot_rolls_its_miss() {
        let missile = WeaponRecord {
            guidance: 1,
            guided_turn: 70,
            ..blaster()
        };
        for guidance in [7, 8, -1, 5, 6] {
            let record = WeaponRecord {
                guidance,
                ..turret(4)
            };
            // B, up the screen, ahead or, for the rear turret, behind.
            let facing = if guidance == 8 { 180.0 } else { 0.0 };
            let firer = Ship::at(1, 0.0, 0.0).facing(facing).of(of_class(100));
            assert_eq!(rolled_by(firer, record, &[]), (None, vec![]), "{guidance}");
        }
        let firer = Ship::at(1, 0.0, 0.0).of(of_class(100));
        assert_eq!(rolled_by(firer, missile, &[]), (None, vec![]), "homing");
    }

    #[test]
    fn a_point_defence_shot_rolls_its_miss_in_its_defenders_class() {
        let [defender, attacker] = defended();
        let hull = HullSpec {
            miss_class: 90,
            ..defender.hull
        };
        let mut ships = [defender.of(hull), attacker];
        let mut combat = Combat::default();
        let mut chance = nova_sim_base::fixture::Draws::of(&[60]);
        missile_in_flight(&mut combat, &mut ships);
        tick_drawing(&mut combat, &mut ships, &mut chance);
        let defence = combat
            .shots()
            .iter()
            .find(|shot| shot.weapon.guidance == Guidance::PointDefence)
            .expect("a point-defence shot");
        assert_eq!(defence.miss_roll, Some(61));
        assert_eq!(chance.asked, [90]);
    }

    /// The ships a heavy NPC's turret shot, rolling `draw` + 1, hits over
    /// 30 ticks: a Fighter-class C, `condition` (with its armour down to 5
    /// when disabled), 100 pixels up its path, and a Warship B 200 behind
    /// it.
    fn struck_through(draw: u32, condition: Condition) -> Vec<ShipRef> {
        let mut fighter = Ship {
            condition,
            ..Ship::at(3, 0.0, -100.0).of(of_class(80))
        };
        if condition == Condition::Disabled {
            fighter.reserves.armor.now = 5.0;
        }
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0)
                .armed(turret(4))
                .targeting(B)
                .of(of_class(100)),
            Ship::at(2, 0.0, -300.0).of(of_class(100)),
            fighter,
        ];
        let mut chance = nova_sim_base::fixture::Draws::of(&[draw]);
        let mut struck = Vec::new();
        for _ in 0..30 {
            tick_drawing(&mut combat, &mut ships, &mut chance);
            ships[0].trigger = QUIET;
            struck.extend(combat.take_strikes().iter().map(|strike| strike.ship));
        }
        struck
    }

    #[test]
    fn a_heavy_turrets_shot_flies_through_an_intact_fighter_it_rolled_over() {
        assert_eq!(struck_through(80, Condition::Intact), [B], "roll 81");
        assert_eq!(struck_through(79, Condition::Intact), [C], "roll 80");
    }

    #[test]
    fn a_heavy_turrets_shot_hits_a_disabled_fighter_whatever_its_roll() {
        assert_eq!(struck_through(99, Condition::Disabled), [C]);
    }

    #[test]
    fn a_point_defence_shots_fuse_passes_an_intact_fighter_it_rolled_over() {
        // A point-defence shot meets a ship only by its proximity fuse.
        let fused = WeaponRecord {
            prox_radius: 5,
            ..exit_defence()
        };
        let struck = |draw| {
            let [defender, attacker] = defended();
            let defender = defender.armed(fused);
            let hull = HullSpec {
                miss_class: 100,
                ..defender.hull
            };
            let fighter = Ship {
                hull: HullSpec {
                    hit_radius: 40.0,
                    ..of_class(80)
                },
                ..Ship::at(3, 70.0, 50.0)
            };
            let mut ships = [defender.of(hull), attacker, fighter];
            let mut combat = Combat::default();
            let mut chance = nova_sim_base::fixture::Draws::of(&[draw]);
            let mut struck = Vec::new();
            missile_in_flight(&mut combat, &mut ships);
            for _ in 0..12 {
                tick_drawing(&mut combat, &mut ships, &mut chance);
                ships[0].trigger = QUIET;
                ships[0].trigger.point_defence = false;
                struck.extend(combat.take_strikes().iter().map(|strike| strike.ship));
            }
            struck
        };
        assert_eq!(struck(80), [], "roll 81 passes the fighter");
        assert_eq!(struck(79), [C], "roll 80 hits it");
    }

    // Bomb and rocket launch (`_SpawnShot` @0x3e73a-0x3e783,
    // @0x3ede8-0x3ef0a, @0x3ef0a-0x3efed).

    /// A bomb or rocket of `guidance`, 10 pixels a tick, `inaccuracy`.
    fn dropping(guidance: i16, inaccuracy: i16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            inaccuracy,
            count: 60,
            speed: 1000,
            ..weapon(128)
        }
    }

    /// The velocity and heading, after its first move, of the shot `ship`
    /// (moving `velocity`, facing right) fires with `record`, by `source`'s
    /// reading of the bomb and rocket launch, on `draws`: the player's
    /// moves on the tick it is fired, an NPC's on the next.
    fn launched(
        ship: Ship,
        velocity: Vec2,
        record: WeaponRecord,
        source: RuleSource,
        draws: &[u32],
    ) -> (Vec2, f32) {
        let mut combat = Combat::default();
        combat.set_bomb_rocket_launch(source);
        let mut ships = [Ship {
            state: ShipState {
                velocity,
                ..ship.state
            },
            ..ship.armed(record)
        }];
        let mut draws = nova_sim_base::fixture::Draws::of(draws);
        tick_drawing(&mut combat, &mut ships, &mut draws);
        if combat
            .shots()
            .last()
            .is_some_and(|shot| shot.moved_from.is_none())
        {
            ships[0].trigger = QUIET;
            tick_drawing(&mut combat, &mut ships, &mut draws);
        }
        let shot = combat.shots().last().expect("a shot");
        (shot.velocity, shot.heading)
    }

    fn the_player() -> Ship {
        Ship {
            id: ShipRef::Player,
            fleet: ShipRef::Player,
            ..Ship::at(0, 0.0, 0.0)
        }
    }

    /// A rocket's velocity after its first tick from `v`, heading right at
    /// 10 pixels a tick.
    fn rocket_after(v: Vec2) -> Vec2 {
        v * projectile::ROCKET_KEEP + Vec2::new(10.0 * projectile::ROCKET_PUSH, 0.0)
    }

    #[test]
    fn an_npcs_rocket_leaves_at_its_ships_velocity_and_its_speed_and_the_players_at_its_ships() {
        let v = Vec2::new(1.0, 2.0);
        let rocket = dropping(6, 0);
        let (npc, _) = launched(Ship::at(1, 0.0, 0.0), v, rocket, RuleSource::Engine, &[]);
        assert!(close_to(npc, rocket_after(Vec2::new(11.0, 2.0))), "{npc:?}");
        let (player, _) = launched(the_player(), v, rocket, RuleSource::Engine, &[]);
        assert!(close_to(player, rocket_after(v)), "{player:?}");
    }

    #[test]
    fn an_npcs_bomb_leaves_at_its_ships_velocity_and_its_speed_and_the_players_at_four_fifths() {
        let v = Vec2::new(1.0, 2.0);
        let bomb = dropping(5, 0);
        let (npc, _) = launched(Ship::at(1, 0.0, 0.0), v, bomb, RuleSource::Engine, &[]);
        assert!(close_to(npc, Vec2::new(11.0, 2.0)), "{npc:?}");
        let (player, _) = launched(the_player(), v, bomb, RuleSource::Engine, &[]);
        assert!(close_to(player, Vec2::new(0.8, 1.6)), "{player:?}");
    }

    #[test]
    fn an_escorts_bomb_and_rocket_leave_as_any_npcs() {
        let escort = Ship {
            leader: Some(ShipRef::Player),
            fleet: ShipRef::Player,
            ..Ship::at(1, 0.0, 0.0)
        };
        let v = Vec2::new(1.0, 2.0);
        let (bomb, _) = launched(escort, v, dropping(5, 0), RuleSource::Engine, &[]);
        assert!(close_to(bomb, Vec2::new(11.0, 2.0)), "{bomb:?}");
        let escort = Ship {
            leader: Some(ShipRef::Player),
            fleet: ShipRef::Player,
            ..Ship::at(1, 0.0, 0.0)
        };
        let (rocket, _) = launched(escort, v, dropping(6, 0), RuleSource::Engine, &[]);
        assert!(
            close_to(rocket, rocket_after(Vec2::new(11.0, 2.0))),
            "{rocket:?}"
        );
    }

    #[test]
    fn by_the_bible_every_ships_bomb_and_rocket_leave_as_the_players() {
        let v = Vec2::new(1.0, 2.0);
        let (bomb, _) = launched(
            Ship::at(1, 0.0, 0.0),
            v,
            dropping(5, 0),
            RuleSource::Bible,
            &[],
        );
        assert!(close_to(bomb, Vec2::new(0.8, 1.6)), "{bomb:?}");
        let (rocket, _) = launched(
            Ship::at(1, 0.0, 0.0),
            v,
            dropping(6, 0),
            RuleSource::Bible,
            &[],
        );
        assert!(close_to(rocket, rocket_after(v)), "{rocket:?}");
        let (gun, _) = launched(
            Ship::at(1, 0.0, 0.0),
            v,
            dropping(-1, 0),
            RuleSource::Bible,
            &[],
        );
        assert!(
            close_to(gun, Vec2::new(11.0, 2.0)),
            "any other as ever: {gun:?}"
        );
    }

    #[test]
    fn a_bombs_inaccuracy_turns_only_its_facing_and_a_rockets_its_flight_too() {
        // Inaccuracy 9 draws below 18, less 9: 14 is +5.
        let at_rest = Vec2::ZERO;
        let (bomb, facing) = launched(
            Ship::at(1, 0.0, 0.0),
            at_rest,
            dropping(5, 9),
            RuleSource::Engine,
            &[14],
        );
        assert!(
            close_to(bomb, Vec2::new(10.0, 0.0)),
            "along the aim: {bomb:?}"
        );
        assert_eq!(facing, 94.0, "95, then a degree back into the wind");
        let (rocket, facing) = launched(
            Ship::at(1, 0.0, 0.0),
            at_rest,
            dropping(6, 9),
            RuleSource::Engine,
            &[14],
        );
        let off = crate::flight::facing(95.0) * 10.0;
        assert!(close_to(rocket, off), "turned with it: {rocket:?} {off:?}");
        assert_eq!(facing, 95.0);
        let moving = Vec2::new(5.0, 0.0);
        let (bomb, facing) = launched(
            the_player(),
            moving,
            dropping(5, 9),
            RuleSource::Engine,
            &[14],
        );
        assert!(close_to(bomb, Vec2::new(4.0, 0.0)), "{bomb:?}");
        assert_eq!(facing, 94.0, "the player's facing takes it too");
    }

    #[test]
    fn the_fight_turns_a_bomb_unwrapped_by_the_engine_and_wrapped_by_the_other_reading() {
        // Thrown from a ship facing up moving left: its velocity bears
        // left of up, so it turns back from 0.
        for (wrap, heading) in [(RuleSource::Engine, -1.0), (RuleSource::Bible, 359.0)] {
            let mut combat = Combat::default();
            combat.set_bomb_heading_wrap(wrap);
            let mut ships = [Ship {
                state: ShipState {
                    velocity: Vec2::new(-5.0, 0.0),
                    ..ShipState::default()
                },
                ..Ship::at(1, 0.0, 0.0).armed(dropping(5, 0))
            }];
            // Thrown on the first tick, it first moves on the second.
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(combat.shots()[0].heading, heading, "{wrap:?}");
        }
    }

    /// The layer of the first shot of `record` that a ship of `hull`
    /// fires at ship 2, 300 to its right.
    #[allow(clippy::large_types_passed_by_value)] // a test's fixture, built in place
    fn fired_layer(record: WeaponRecord, hull: HullSpec) -> ShotLayer {
        let mut combat = Combat::default();
        let mut ships = [
            Ship {
                hull,
                ..Ship::at(1, 0.0, 0.0)
                    .armed(record)
                    .targeting(B)
                    .facing(90.0)
            },
            Ship::at(2, 300.0, 0.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        combat.shots()[0].layer
    }

    #[test]
    fn a_ships_shot_takes_its_layer_by_its_guidance_and_its_class() {
        let over = HullSpec {
            turret_shots_over: true,
            ..HullSpec::default()
        };
        assert_eq!(fired_layer(turret(4), over), ShotLayer::OverShips);
        assert_eq!(
            fired_layer(turret(4), HullSpec::default()),
            ShotLayer::Shots
        );
        assert_eq!(fired_layer(turret(7), over), ShotLayer::Shots, "4 alone");
        assert_eq!(fired_layer(gun(), over), ShotLayer::Shots);
        let homing = WeaponRecord {
            guidance: 1,
            ..gun()
        };
        assert_eq!(fired_layer(homing, over), ShotLayer::Homing);
        let mut combat = Combat::default();
        let mut ships = [
            Ship {
                hull: over,
                ..player_with(turret(4)).targeting(B)
            },
            Ship::at(2, 300.0, 0.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            combat.shots()[0].layer,
            ShotLayer::OverShips,
            "the player's"
        );
        assert!(
            combat.shots()[0].turret_over,
            "carried for its sub-munitions"
        );
    }

    #[test]
    fn a_shot_leaves_on_its_ships_truncated_heading_and_flies_along_it() {
        let mut combat = Combat::default();
        let mut ships = [Ship {
            state: ShipState {
                position: Vec2::ZERO,
                velocity: Vec2::new(1.0, 2.0),
                heading: 45.5,
            },
            ..Ship::at(1, 0.0, 0.0).armed(gun())
        }];
        tick(&mut combat, &mut ships, &NovaDisable);
        let shot = combat.shots()[0];
        assert_eq!(shot.heading, 45.0, "trunc(45.5) (@0x3e677)");
        assert_eq!(
            shot.velocity,
            Vec2::new(1.0, 2.0) + trig::accel(45, 10.0),
            "_Accel(trunc(heading)) (@0x3eeea)"
        );
    }

    #[test]
    fn a_turret_at_a_resting_target_fires_at_its_table_bearing_turned_and_wrapped() {
        // B reads 0 by the table from A; inaccuracy 9 draws below 18, less
        // 9: 4 is -5, taken round to 355.
        let wild = WeaponRecord {
            inaccuracy: 9,
            ..turret(4)
        };
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(wild).targeting(B).facing(90.0),
            Ship::at(2, 10.0, -1000.0),
        ];
        assert_eq!(
            trig::intercept_angle(Vec2::ZERO, Vec2::new(10.0, -1000.0)),
            0
        );
        let mut chance = nova_sim_base::fixture::Draws::of(&[4]);
        tick_drawing(&mut combat, &mut ships, &mut chance);
        let shot = combat.shots()[0];
        assert_eq!(shot.heading, 355.0);
        assert_eq!(shot.velocity, trig::accel(355, 20.0));
    }

    #[test]
    fn an_npcs_thrown_bomb_flies_along_its_whole_degree_aim_before_its_inaccuracy() {
        // Facing 90.7: along 90, then +5 turns its facing only.
        let ship = Ship {
            state: ShipState {
                heading: 90.7,
                ..ShipState::default()
            },
            ..Ship::at(1, 0.0, 0.0)
        };
        let (bomb, facing) = launched(ship, Vec2::ZERO, dropping(5, 9), RuleSource::Engine, &[14]);
        assert_eq!(bomb, trig::accel(90, 10.0), "along trunc(90.7)");
        assert_eq!(facing, 94.0, "95, then a degree back into the wind");
    }

    // Point defence against ships (`_HandleShipPointDefense`
    // @0x394ce-0x3969d).

    impl Ship {
        /// The ship, of a class point defence fires on (`Flags2` 0x0008),
        /// attacking `ship`.
        fn attacking(self, ship: ShipRef) -> Self {
            Self {
                hull: HullSpec {
                    pd_target: true,
                    ..self.hull
                },
                attacking: Some(ship),
                ..self
            }
        }

        /// The ship moving `velocity` a tick.
        fn moving(self, velocity: Vec2) -> Self {
            Self {
                state: ShipState {
                    velocity,
                    ..self.state
                },
                ..self
            }
        }
    }

    /// The defender A at the centre facing up with `defence`, its trigger
    /// asking for point defence alone, and B at (`x`, `y`) attacking it.
    fn fighter_attack(defence: WeaponRecord, x: f32, y: f32) -> [Ship; 2] {
        let mut defender = Ship::at(1, 0.0, 0.0).armed(defence).facing(0.0);
        defender.trigger = QUIET;
        [defender, Ship::at(2, x, y).attacking(A)]
    }

    /// The point-defence shots in flight.
    fn pd_shots(combat: &Combat) -> Vec<Shot> {
        combat
            .shots()
            .iter()
            .filter(|shot| shot.weapon.guidance == Guidance::PointDefence)
            .copied()
            .collect()
    }

    /// A point-defence beam 300 pixels long, held 5 ticks, firing every
    /// 30, doing 1 mass and 4 energy damage.
    fn pd_beam() -> WeaponRecord {
        WeaponRecord {
            guidance: 10,
            reload: 30,
            count: 5,
            beam_length: 300,
            mass_dmg: 1,
            energy_dmg: 4,
            ..weapon(150)
        }
    }

    /// The whole-degree lead angle from `from` at `target` for a shot of
    /// `spec` (`_CalcLeadAngle`).
    fn lead_from(from: &ShipState, target: &Target, spec: &WeaponSpec) -> f32 {
        let at = (target.position, target.velocity);
        f32::from(trig::lead_angle(
            from.position,
            from.velocity,
            at,
            spec.lead_shot(),
        ))
    }

    /// `ship` as point defence leads it.
    fn led_target(ship: &mut Ship) -> Target {
        ship.fighter().as_target()
    }

    #[test]
    fn point_defence_fires_at_a_fighter_attacking_its_ship_leading_it() {
        let mut combat = Combat::default();
        let mut ships = fighter_attack(quad(), 0.0, -200.0);
        ships[0] =
            std::mem::replace(&mut ships[0], Ship::at(9, 0.0, 0.0)).moving(Vec2::new(1.0, 2.0));
        ships[1] =
            std::mem::replace(&mut ships[1], Ship::at(9, 0.0, 0.0)).moving(Vec2::new(15.0, 0.0));
        let target = led_target(&mut ships[1]);
        let from = ships[0].state;
        tick(&mut combat, &mut ships, &NovaDisable);
        let fired = pd_shots(&combat);
        assert_eq!(fired.len(), 1, "{:?}", combat.shots());
        let shot = fired[0];
        let lead = lead_from(&from, &target, &WeaponSpec::new(&quad()));
        let bearing = trig::intercept_angle(from.position, target.position);
        assert_eq!(shot.heading, lead, "the whole-degree lead (@0x397c6)");
        assert_eq!(shot.layer, ShotLayer::Shots, "moved to 0x3b52f0 (@0x398a7)");
        assert!(
            (lead - f32::from(bearing)).abs() > 3.0,
            "the lead matters here"
        );
        let velocity = Vec2::new(1.0, 2.0) + trig::accel(trig::whole(lead), 20.0);
        assert_eq!(shot.velocity, velocity);
        assert_eq!(
            (shot.target, shot.aimed_at),
            (None, None),
            "aimed at nothing"
        );
    }

    #[test]
    fn point_defence_at_a_fighter_goes_through_the_shared_tail() {
        for defence in [quad(), pd_beam()] {
            let id = defence.id;
            let mut combat = Combat::default();
            let mut ships = fighter_attack(defence, 0.0, -100.0);
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(
                sounds(&combat.take_events()),
                [(id, Some(FireSound::at(PD_FIRE_PRIORITY)))],
                "{id:?}"
            );
            let mount = ships[0].armament.mounts()[0];
            assert_eq!(mount.reload, mount.spec.reload - 1.0, "{id:?}: reloading");
        }
    }

    #[test]
    fn point_defence_fires_at_a_candidate_missile_however_near_a_fighter_is() {
        let mut combat = Combat::default();
        let mut ships = fighter_attack(quad(), 50.0, 0.0);
        let missile = put(
            &mut combat,
            drawn(missile(), 18),
            C,
            Vec2::new(0.0, -300.0),
            Vec2::ZERO,
        );
        combat.shots[0].aimed_at = Some(A);
        tick(&mut combat, &mut ships, &NovaDisable);
        let fired = pd_shots(&combat);
        assert_eq!(fired.len(), 1);
        let to_missile = aim::bearing(Vec2::ZERO, flying(&combat, missile).unwrap().position);
        assert!(
            (fired[0].heading - to_missile).abs() < 1.0,
            "at the missile: {}",
            fired[0].heading
        );
    }

    #[test]
    fn a_missile_out_of_range_leaves_point_defence_to_a_fighter_in_range() {
        let mut combat = Combat::default();
        let mut ships = fighter_attack(quad(), 50.0, 0.0);
        put(
            &mut combat,
            drawn(missile(), 18),
            C,
            Vec2::new(0.0, -400.0),
            Vec2::ZERO,
        );
        combat.shots[0].aimed_at = Some(A);
        tick(&mut combat, &mut ships, &NovaDisable);
        let fired = pd_shots(&combat);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].heading, 90.0, "at the fighter");
    }

    #[test]
    fn point_defence_tries_its_first_ready_weapon_alone_at_a_ship() {
        let long = WeaponRecord {
            beam_length: 500,
            ..pd_beam()
        };
        let fires = |records: &[WeaponRecord]| {
            let mut combat = Combat::default();
            let [mut defender, attacker] = fighter_attack(quad(), 0.0, -400.0);
            defender.armament =
                Armament::new(records.iter().map(|record| (WeaponSpec::new(record), 1)));
            let mut ships = [defender, attacker];
            tick(&mut combat, &mut ships, &NovaDisable);
            fired(&combat.take_events(), A)
        };
        assert_eq!(fires(&[long]), 1, "the beam reaches it");
        assert_eq!(fires(&[quad(), long]), 0, "the Quad first, and it can't");
    }

    #[test]
    fn a_point_defence_shot_at_a_ship_is_turned_by_its_inaccuracy() {
        let wild = WeaponRecord {
            inaccuracy: 10,
            ..quad()
        };
        let mut combat = Combat::default();
        let mut ships = fighter_attack(wild, 100.0, 0.0);
        let mut chance = nova_sim_base::fixture::Draws::of(&[3]);
        tick_drawing(&mut combat, &mut ships, &mut chance);
        assert_eq!(chance.asked.first(), Some(&20), "Rand(2i)");
        assert_eq!(pd_shots(&combat)[0].heading, 83.0, "90, then 3 - 10");
    }

    #[test]
    fn point_defence_leads_a_ship_from_its_exit() {
        let defence = WeaponRecord {
            exit_type: 1,
            ..quad()
        };
        let mut combat = Combat::default();
        let [defender, attacker] = fighter_attack(defence, 0.0, -200.0);
        let defender = defender.of(exits_at(1, [(10, 0), (20, 0), (30, 0), (40, 0)]));
        let mut ships = [defender, attacker.moving(Vec2::new(15.0, 0.0))];
        let target = led_target(&mut ships[1]);
        tick(&mut combat, &mut ships, &NovaDisable);
        let shot = pd_shots(&combat)[0];
        let at = start(&shot);
        assert!(
            close_to(at, Vec2::new(20.0, 0.0)),
            "the second exit: {at:?}"
        );
        let leaving = ShipState {
            position: at,
            ..ShipState::default()
        };
        let spec = WeaponSpec::new(&quad());
        let lead = lead_from(&leaving, &target, &spec);
        assert_eq!(shot.heading, lead);
        let centre = lead_from(&ShipState::default(), &target, &spec);
        assert!((lead - centre).abs() > 1.0, "the exit matters here");
    }

    /// The Storm Chaingun: point defence firing every 5 ticks, 12 pixels a
    /// tick for 20, doing 3 mass and 3 energy damage, with no proximity
    /// fuse.
    fn chaingun() -> WeaponRecord {
        WeaponRecord {
            guidance: 9,
            reload: 5,
            count: 20,
            speed: 1200,
            mass_dmg: 3,
            energy_dmg: 3,
            ..weapon(161)
        }
    }

    /// The strikes until one lands, at most 20 ticks of `ships`.
    fn strikes_until_a_hit(combat: &mut Combat, ships: &mut [Ship]) -> Vec<Strike> {
        for _ in 0..20 {
            tick(combat, ships, &NovaDisable);
            let strikes = combat.take_strikes();
            if !strikes.is_empty() {
                return strikes;
            }
        }
        Vec::new()
    }

    #[test]
    fn a_point_defence_shot_at_a_ship_hits_it_by_contact_directly_and_unintended() {
        let mut combat = Combat::default();
        let mut ships = fighter_attack(chaingun(), 0.0, -100.0);
        let strikes = strikes_until_a_hit(&mut combat, &mut ships);
        assert_eq!(strikes.len(), 1, "no proximity fuse: by contact");
        let strike = strikes[0];
        assert_eq!((strike.ship, strike.by), (B, Some(A)));
        assert!(strike.direct, "direct");
        assert!(!strike.intended, "aimed at no ship");
        assert!(strike.damage > 0.0);
    }

    #[test]
    fn a_point_defence_shot_at_a_ship_still_rolls_its_miss() {
        let mut combat = Combat::default();
        let [defender, attacker] = fighter_attack(chaingun(), 0.0, -100.0);
        // The defender's class rolls 100 here; a fighter's is 80.
        let mut ships = [defender, attacker.of(of_class(80)).attacking(A)];
        assert_eq!(strikes_until_a_hit(&mut combat, &mut ships), []);
    }

    #[test]
    fn a_point_defence_shot_at_a_ship_meets_no_missile() {
        let mut combat = Combat::default();
        let mut ships = fighter_attack(quad(), 0.0, -100.0);
        // Still, where the shot ends the tick: aimed at C, so no candidate.
        let missile = put(
            &mut combat,
            still_missile(),
            B,
            Vec2::new(0.0, -20.0),
            Vec2::ZERO,
        );
        combat.shots[0].aimed_at = Some(C);
        combat.shots[0].target = Some(C);
        let events = met(&mut combat, &mut ships);
        assert_eq!(pd_shots(&combat).len(), 1, "fired at B, not spent");
        assert_eq!(shot_down(&events), []);
        assert!(flying(&combat, missile).is_some());
    }

    #[test]
    fn a_point_defence_beam_held_on_a_fighter_attacking_its_ship_hits_it() {
        let mut combat = Combat::default();
        let mut ships = fighter_attack(pd_beam(), 0.0, -100.0);
        tick(&mut combat, &mut ships, &NovaDisable);
        let beam = combat.beams()[0];
        assert_eq!((beam.aiming, beam.aimed_at), (Aiming::Ship(B), Some(B)));
        let strikes = combat.take_strikes();
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        let strike = strikes[0];
        assert_eq!((strike.ship, strike.by), (B, Some(A)));
        assert!(strike.direct && strike.intended, "{strike:?}");
        assert!(strike.damage > 0.0, "its damage taken: {strike:?}");
    }

    #[test]
    fn a_point_defence_beam_on_a_ship_never_jitters() {
        let wild = WeaponRecord {
            inaccuracy: 10,
            ..pd_beam()
        };
        let mut combat = Combat::default();
        let mut ships = fighter_attack(wild, 100.0, -100.0);
        let mut chance = nova_sim_base::fixture::Draws::of(&[0, 0, 0]);
        tick_drawing(&mut combat, &mut ships, &mut chance);
        let beam = combat.beams()[0];
        let bearing = crate::trig::intercept_angle(beam.start, Vec2::new(100.0, -100.0));
        assert_eq!(beam.heading, f32::from(bearing), "the bearing exactly");
    }

    #[test]
    fn a_point_defence_beam_on_a_ship_ends_when_the_ship_is_destroyed() {
        let mut combat = Combat::default();
        let mut ships = fighter_attack(pd_beam(), 0.0, -100.0);
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.beams().len(), 1);
        ships[1].condition = Condition::Destroyed;
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.beams(), []);
    }

    #[test]
    fn a_nearest_exit_point_defence_beam_leaves_from_the_exit_nearest_its_ship() {
        let beam = WeaponRecord {
            flags3: crate::combat::weapon::NEAREST_EXIT,
            exit_type: 3,
            ..pd_beam()
        };
        for (x, exit) in [(100.0, 1), (-100.0, 3)] {
            let mut combat = Combat::default();
            let [defender, attacker] = fighter_attack(beam, x, 0.0);
            let mut ships = [defender.of(manticore()), attacker];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(combat.beams()[0].exit, Some(exit), "{x}");
        }
    }

    #[test]
    fn point_defence_weighs_each_fighter_by_its_own_terms() {
        type Making = (&'static str, fn(&mut [Ship; 2]), usize);
        let variants: [Making; 9] = [
            ("as it is", |_| {}, 1),
            ("held disabled", |ships| ships[1].held_disabled = true, 0),
            ("cloaked", |ships| ships[1].cloaked = true, 0),
            (
                "cloaked, seen by a scanner",
                |ships| {
                    ships[1].cloaked = true;
                    ships[0].cloak_scanner = true;
                },
                1,
            ),
            ("waiting in its gate", |ships| ships[1].emerging = true, 0),
            (
                "of no Flags2 0x0008",
                |ships| ships[1].hull.pd_target = false,
                0,
            ),
            ("attacking no one", |ships| ships[1].attacking = None, 0),
            (
                "the defender's leader",
                |ships| ships[0].leader = Some(B),
                0,
            ),
            (
                "attacking the defender's leader",
                |ships| {
                    ships[0].leader = Some(C);
                    ships[1].attacking = Some(C);
                },
                1,
            ),
        ];
        for (name, make, shots) in variants {
            let mut combat = Combat::default();
            let mut ships = fighter_attack(quad(), 0.0, -100.0);
            make(&mut ships);
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(fired(&combat.take_events(), A), shots, "{name}");
        }
    }

    #[test]
    fn an_npcs_point_defence_fires_at_the_player_only_while_it_threatens_the_player() {
        for threatens_player in [false, true] {
            let mut combat = Combat::default();
            let [mut defender, _] = fighter_attack(quad(), 0.0, -100.0);
            defender.threatens_player = threatens_player;
            let mut player = Ship::at(9, 0.0, -100.0).attacking(A);
            player.id = ShipRef::Player;
            player.fleet = ShipRef::Player;
            let mut ships = [player, defender];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(
                fired(&combat.take_events(), A),
                usize::from(threatens_player),
                "{threatens_player}"
            );
        }
    }

    #[test]
    fn by_either_reading_point_defence_asks_no_rule_of_a_ship() {
        let rule = Hostility::default();
        let rules = Rules {
            defence: &rule,
            ..Rules::default()
        };
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let mut combat = Combat::default();
            combat.set_pd_hostility(source);
            let mut ships = fighter_attack(quad(), 0.0, -100.0);
            tick_with(&mut combat, &mut ships, &Arsenal::default(), rules);
            assert_eq!(fired(&combat.take_events(), A), 1, "{source:?}");
        }
        assert_eq!(*rule.asked.borrow(), [], "never asked");
    }

    // Jamming and sensor interference (`_SpawnShot` @0x3eab2-0x3eb66 and
    // @0x3f051-0x3f0a7, `_HandleShotGuidance` @0x320dc-0x32aa4).

    /// A tick among `ships` in `field`, with `arsenal`, drawing on
    /// `chance`.
    fn tick_in(
        combat: &mut Combat,
        ships: &mut [Ship],
        (arsenal, field): (&Arsenal, Field),
        chance: &mut dyn Chance,
    ) {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        combat.tick(
            &mut fighters,
            arsenal,
            &Governments::default(),
            Rules::default(),
            field,
            Scenery::default(),
            chance,
        );
    }

    /// [`missile`] with `seeker` and `jam_vuln`.
    fn vulnerable(seeker: u16, jam_vuln: [i16; 4]) -> WeaponRecord {
        WeaponRecord {
            seeker,
            jam_vuln,
            ..missile()
        }
    }

    /// The shots in flight after A fires `record` once at B, at (0, -300),
    /// in a system of `interference`, drawing on `chance`.
    fn fired_in(record: WeaponRecord, interference: i16, chance: &mut dyn Chance) -> Vec<Shot> {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(record).targeting(B),
            Ship::at(2, 0.0, -300.0),
        ];
        let field = Field {
            interference,
            ..Field::default()
        };
        tick_in(
            &mut combat,
            &mut ships,
            (&Arsenal::default(), field),
            chance,
        );
        combat.shots().to_vec()
    }

    #[test]
    fn every_shot_rolls_its_weapons_jam_vuln_as_it_is_launched() {
        let mut chance = Ranged::of(&[(41, 17)]);
        let shots = fired_in(vulnerable(0, [40, 0, 7, 0]), 0, &mut chance);
        assert_eq!(shots[0].jam_rolls, [17, 0, 7, 0]);
        assert!(chance.was_asked(41) && chance.was_asked(8));
        let unguided = WeaponRecord {
            jam_vuln: [3, 0, 0, 0],
            ..blaster()
        };
        let shots = fired_in(unguided, 0, &mut Ranged::of(&[]));
        assert_eq!(shots[0].jam_rolls, [3, 0, 0, 0], "every shot rolls");
    }

    #[test]
    fn a_seeker_0x0008_missile_is_interfered_when_its_roll_falls_within_the_interference() {
        let sensitive = vulnerable(SENSOR_INTERFERENCE, [0; 4]);
        for (interference, draw, jam) in [
            (50, 49, Jam::Interfered),
            (50, 50, Jam::Clear),
            (100, 99, Jam::Interfered),
            (0, 0, Jam::Clear),
            (-5, 0, Jam::Clear),
        ] {
            let mut chance = Ranged::of(&[(100, draw)]);
            let shots = fired_in(sensitive, interference, &mut chance);
            assert_eq!(shots[0].jam, jam, "{interference} {draw}");
        }
        for record in [
            vulnerable(0, [0; 4]),
            WeaponRecord {
                seeker: SENSOR_INTERFERENCE,
                ..blaster()
            },
        ] {
            let mut chance = Ranged::of(&[(100, 0)]);
            let shots = fired_in(record, 100, &mut chance);
            assert_eq!(shots[0].jam, Jam::Clear, "{record:?}");
            assert!(!chance.was_asked(100), "{record:?}");
        }
    }

    /// A shell releasing a homing sub-munition 148 of `seeker` and
    /// `jam_vuln` at the end of its 3 ticks.
    fn seeding(seeker: u16, jam_vuln: [i16; 4]) -> (WeaponRecord, Arsenal) {
        let shell = WeaponRecord {
            reload: 1000,
            count: 3,
            speed: 1000,
            sub_count: 1,
            sub_type: Some(WeaponId(148)),
            ..weapon(182)
        };
        let sub = WeaponRecord {
            id: WeaponId(148),
            ..vulnerable(seeker, jam_vuln)
        };
        (shell, Arsenal::new(&[shell, sub], Vec::new()))
    }

    #[test]
    fn a_sub_munition_rolls_its_own_weapons_jam_vuln_and_interference() {
        let (shell, arsenal) = seeding(SENSOR_INTERFERENCE, [5, 0, 0, 9]);
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(shell).targeting(B),
            Ship::at(2, 0.0, -900.0),
        ];
        let field = Field {
            interference: 100,
            ..Field::default()
        };
        for _ in 0..5 {
            tick_in(
                &mut combat,
                &mut ships,
                (&arsenal, field),
                &mut Ranged::of(&[]),
            );
            ships[0].trigger = QUIET;
        }
        let subs = sub_shots(&combat);
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].jam_rolls, [5, 0, 0, 9]);
        assert_eq!(subs[0].jam, Jam::Interfered);
    }

    /// The heading of the missile `ships[0]` fires on the first tick,
    /// after `ticks` moves in `field`, the first on the tick after it is
    /// fired, drawing on `chance`, or `None` once it is gone.
    fn missile_heading(
        ships: &mut [Ship],
        ticks: u32,
        field: Field,
        chance: &mut dyn Chance,
    ) -> Option<f32> {
        let mut combat = Combat::default();
        for _ in 0..=ticks {
            tick_in(&mut combat, ships, (&Arsenal::default(), field), chance);
            ships[0].trigger = QUIET;
        }
        combat.shots().first().map(|shot| shot.heading)
    }

    /// A fires a missile of `JamVuln` [100, 0, 0, 0] at B straight below
    /// it, 90 degrees off its heading; B jams `jamming`.
    fn at_a_jammer(jamming: [i16; 4]) -> [Ship; 2] {
        [
            Ship::at(1, 0.0, 0.0)
                .armed(vulnerable(0, [100, 0, 0, 0]))
                .targeting(B),
            Ship {
                jamming: Jamming(jamming),
                ..Ship::at(2, 0.0, 1000.0)
            },
        ]
    }

    #[test]
    fn a_missile_at_a_ship_whose_jamming_beats_its_roll_stops_turning() {
        let field = Field::default();
        let jammed = missile_heading(
            &mut at_a_jammer([30, 0, 0, 0]),
            16,
            field,
            &mut Ranged::of(&[(101, 71)]),
        );
        assert_eq!(jammed, Some(90.0));
        let missed = missile_heading(
            &mut at_a_jammer([30, 0, 0, 0]),
            16,
            field,
            &mut Ranged::of(&[(101, 70)]),
        );
        assert!((missed.unwrap() - 97.0).abs() < 1e-4, "{missed:?}");
    }

    #[test]
    fn a_disabled_breaking_up_or_held_disabled_ship_jams_nothing() {
        for (condition, held) in [
            (Condition::Disabled, false),
            (
                Condition::Dying {
                    ticks_left: 90,
                    starting: false,
                },
                false,
            ),
            (Condition::Intact, true),
        ] {
            let mut ships = at_a_jammer([100, 100, 100, 100]);
            let mut combat = Combat::default();
            let mut chance = Ranged::of(&[(101, 100)]);
            let field = (&Arsenal::default(), Field::default());
            // Fired at B intact, which is then downed.
            tick_in(&mut combat, &mut ships, field, &mut chance);
            ships[0].trigger = QUIET;
            ships[1].condition = condition;
            ships[1].held_disabled = held;
            ships[1].reserves.armor.now = 1.0;
            ships[1].hull.death_delay = 90;
            // Its 15 moves, from the tick after it is fired.
            for _ in 1..17 {
                tick_in(&mut combat, &mut ships, field, &mut chance);
            }
            assert_ne!(ships[1].condition, Condition::Intact, "{condition:?}");
            let heading = combat.shots()[0].heading;
            assert!(
                (heading - 97.0).abs() < 1e-4,
                "{condition:?} {held}: {heading:?}"
            );
        }
    }

    #[test]
    fn a_jammed_missile_still_hits_its_target_and_its_damage_still_counts_as_aimed_at_it() {
        // B straight ahead, jamming every type.
        let mut ships = at_a_jammer([100, 100, 100, 100]);
        ships[1].state.position = Vec2::new(250.0, 0.0);
        let mut combat = Combat::default();
        let mut chance = Ranged::of(&[(101, 100)]);
        let mut aimed = BTreeMap::new();
        let mut strikes = Vec::new();
        for tick in 1..=26 {
            tick_in(
                &mut combat,
                &mut ships,
                (&Arsenal::default(), Field::default()),
                &mut chance,
            );
            ships[0].trigger = QUIET;
            // Its 20th move, fired on tick 1.
            if tick == 21 {
                aimed = combat.targeted_damage();
            }
            strikes.extend(combat.take_strikes());
        }
        assert_eq!(aimed, BTreeMap::from([(B, 15)]));
        assert!(
            strikes
                .iter()
                .any(|strike| strike.ship == B && strike.direct),
            "{strikes:?}"
        );
    }

    #[test]
    fn a_players_jammed_missile_still_locks() {
        let jammer = Ship {
            jamming: Jamming([100, 100, 100, 100]),
            ..far_b()
        };
        let mut ships = [player_with(vulnerable(0, [100, 0, 0, 0])), jammer];
        let mut combat = Combat::default();
        let mut chance = Ranged::of(&[(101, 100)]);
        let mut locks = 0;
        for _ in 0..40 {
            tick_in(
                &mut combat,
                &mut ships,
                (&Arsenal::default(), Field::default()),
                &mut chance,
            );
            ships[0].trigger = QUIET;
            locks += combat
                .take_strikes()
                .iter()
                .filter(|strike| strike.lock)
                .count();
        }
        assert_eq!(locks, 1);
    }

    /// A fires a plain missile at B straight below it, cloaked.
    fn at_a_cloak() -> [Ship; 2] {
        [
            Ship::at(1, 0.0, 0.0).armed(missile()).targeting(B),
            Ship {
                cloaked: true,
                ..Ship::at(2, 0.0, 1000.0)
            },
        ]
    }

    #[test]
    fn a_missile_stops_turning_at_a_target_its_firer_cannot_see() {
        let unseen = missile_heading(&mut at_a_cloak(), 16, Field::default(), &mut NeverFires);
        assert_eq!(unseen, Some(90.0));
        let mut emerging = at_a_cloak();
        emerging[1].cloaked = false;
        emerging[1].emerging = true;
        assert_eq!(
            missile_heading(&mut emerging, 16, Field::default(), &mut NeverFires),
            Some(90.0),
            "coming out of a gate"
        );
        // The firer is within 200 pixels of the target, with a scanner.
        for (y, seen) in [(200.0, true), (201.0, false)] {
            let mut scanning = at_a_cloak();
            scanning[1].state.position = Vec2::new(0.0, y);
            scanning[0].cloak_scanner = true;
            let heading = missile_heading(&mut scanning, 16, Field::default(), &mut NeverFires);
            assert_eq!(heading.unwrap() > 90.0, seen, "{y}: {heading:?}");
        }
    }

    #[test]
    fn an_escort_with_a_cloak_steers_its_missile_at_its_cloaked_leader() {
        for (has_cloak, turns) in [(true, true), (false, false)] {
            let mut ships = at_a_cloak();
            ships[0].leader = Some(B);
            ships[0].has_cloak = has_cloak;
            let heading = missile_heading(&mut ships, 16, Field::default(), &mut NeverFires);
            assert_eq!(heading.unwrap() > 90.0, turns, "{has_cloak}");
        }
    }

    #[test]
    fn an_interfered_missile_circles_by_the_fields_frame() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0)
                .armed(vulnerable(SENSOR_INTERFERENCE, [0; 4]))
                .targeting(B),
            Ship::at(2, 0.0, 1000.0),
        ];
        let field = |frame| Field {
            frame,
            interference: 100,
            ..Field::default()
        };
        // Fired on frame 0, it moves on frames 1 to 15.
        for frame in 0..16 {
            tick_in(
                &mut combat,
                &mut ships,
                (&Arsenal::default(), field(frame)),
                &mut NeverFires,
            );
            ships[0].trigger = QUIET;
        }
        assert_eq!(combat.shots()[0].jam, Jam::Interfered);
        assert_eq!(combat.shots()[0].heading, 90.0);
        tick_in(
            &mut combat,
            &mut ships,
            (&Arsenal::default(), field(149)),
            &mut NeverFires,
        );
        assert!((combat.shots()[0].heading - 83.0).abs() < 1e-4);
        tick_in(
            &mut combat,
            &mut ships,
            (&Arsenal::default(), field(150)),
            &mut NeverFires,
        );
        assert!((combat.shots()[0].heading - 90.0).abs() < 1e-4);
    }

    #[test]
    fn the_seeker_lock_angle_follows_the_rulebook_the_short_way_by_default() {
        // A faces 355; on the missile's 16th tick B is 100 pixels off at a
        // bearing of 5.
        let from = Vec2::ZERO + crate::flight::facing(355.0) * 150.0;
        let b = from + crate::flight::facing(5.0) * 100.0;
        let locked = WeaponRecord {
            seeker: LOSES_LOCK_OFF_AHEAD,
            ..missile()
        };
        for (source, kept) in [
            (None, true),
            (Some(RuleSource::Bible), true),
            (Some(RuleSource::Engine), false),
        ] {
            let mut combat = Combat::default();
            if let Some(source) = source {
                combat.set_seeker_lock_angle(source);
            }
            let mut ships = [
                Ship::at(1, 0.0, 0.0)
                    .armed(locked)
                    .facing(355.0)
                    .targeting(B),
                Ship::at(2, b.x, b.y),
            ];
            // Its 16 moves, from the tick after it is fired.
            for _ in 0..17 {
                tick(&mut combat, &mut ships, &NovaDisable);
                ships[0].trigger = QUIET;
            }
            assert_eq!(combat.shots()[0].target.is_some(), kept, "{source:?}");
        }
    }

    #[test]
    fn a_missile_turned_on_its_firer_strikes_it_as_a_hit_by_no_ship() {
        let mut ships = at_a_jammer([100, 0, 0, 0]);
        ships[0] = Ship::at(1, 0.0, 0.0)
            .armed(vulnerable(TURNS_ON_FIRER, [100, 0, 0, 0]))
            .targeting(B);
        let mut combat = Combat::default();
        let mut chance = Ranged::of(&[(101, 100), (500, 0)]);
        let mut strikes = Vec::new();
        for _ in 0..100 {
            tick_in(
                &mut combat,
                &mut ships,
                (&Arsenal::default(), Field::default()),
                &mut chance,
            );
            ships[0].trigger = QUIET;
            strikes.extend(combat.take_strikes());
        }
        let on_firer: Vec<&Strike> = strikes.iter().filter(|strike| strike.ship == A).collect();
        assert_eq!(on_firer.len(), 1, "{strikes:?}");
        let strike = on_firer[0];
        assert_eq!(strike.by, None);
        assert!(
            strike.damage > 0.0 && strike.direct && strike.intended,
            "{strike:?}"
        );
        assert!(ships[0].reserves.shield.now < 10.0, "the damage lands");
    }

    #[test]
    fn the_blast_of_a_shot_no_ship_launched_spares_no_one() {
        let shell = WeaponRecord {
            blast_radius: 40,
            ..blaster()
        };
        for (source, by, firer_hit) in [
            (Source::Other, None, true),
            (Source::Ship(Ties::default()), Some(A), false),
        ] {
            let shot = Shot {
                source,
                aimed_at: Some(B),
                ..Shot::launch(
                    WeaponSpec::new(&shell),
                    A,
                    A,
                    &Ship::at(1, 0.0, 0.0).state,
                    90.0,
                )
            };
            let mut combat = Combat {
                shots: vec![shot],
                ..Combat::default()
            };
            let mut ships = [Ship::at(1, 0.0, 0.0), Ship::at(2, 20.0, 0.0)];
            // Its first move, tested the tick after.
            tick(&mut combat, &mut ships, &NovaDisable);
            tick(&mut combat, &mut ships, &NovaDisable);
            let strikes = combat.take_strikes();
            assert!(
                strikes
                    .iter()
                    .any(|strike| strike.ship == B && strike.by == by && strike.direct),
                "{source:?}: {strikes:?}"
            );
            assert_eq!(
                strikes
                    .iter()
                    .any(|strike| strike.ship == A && strike.by == by),
                firer_hit,
                "{source:?}: {strikes:?}"
            );
        }
    }

    #[test]
    fn a_one_at_a_time_missile_turned_on_its_firer_no_longer_holds_its_reload() {
        let single = WeaponRecord {
            reload: 3,
            flags3: 0x0004,
            ..vulnerable(TURNS_ON_FIRER, [100, 0, 0, 0])
        };
        for (odds, again) in [(0, true), (1, false)] {
            let mut ships = at_a_jammer([100, 0, 0, 0]);
            ships[0] = Ship::at(1, 0.0, 0.0).armed(single).targeting(B);
            let mut combat = Combat::default();
            let mut chance = Ranged::of(&[(101, 100), (500, odds)]);
            let mut fires = Vec::new();
            for tick in 0..30 {
                tick_in(
                    &mut combat,
                    &mut ships,
                    (&Arsenal::default(), Field::default()),
                    &mut chance,
                );
                if fired(&combat.take_events(), A) > 0 {
                    fires.push(tick);
                }
            }
            assert_eq!(fires.len() > 1, again, "{odds}: {fires:?}");
        }
    }

    // Asteroids.

    use crate::asteroid::{Asteroid, AsteroidType, Asteroids, KINDS, Remains};
    use crate::combat::asteroid_hit::AsteroidFuse;
    use crate::combat::asteroid_walk::AsteroidField;

    /// An asteroid of kind 0 in slot `slot` at (`x`, `y`), of strength 10,
    /// at rest.
    fn rock(slot: usize, x: f32, y: f32) -> (usize, Asteroid) {
        worn(slot, x, y, 10)
    }

    /// [`rock`] with `strength` left.
    fn worn(slot: usize, x: f32, y: f32, strength: i16) -> (usize, Asteroid) {
        let asteroid = Asteroid {
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            frame: 0.0,
            spin: 0.0,
            strength,
            kind: 0,
        };
        (slot, asteroid)
    }

    /// An asteroid field lent to a test's fight: its slots, and its
    /// kinds, of which kind 0 is 20 across, so of radius 10, of `Mass` 20,
    /// and leaves nothing as it is destroyed.
    #[derive(Clone, Debug)]
    struct Rocks {
        field: Asteroids,
        types: [AsteroidType; KINDS],
    }

    impl Rocks {
        /// A field holding each of `placed` in its slot.
        fn of(placed: impl IntoIterator<Item = (usize, Asteroid)>) -> Self {
            let mut types = [AsteroidType::MISSING; KINDS];
            types[0] = AsteroidType {
                size: (20.0, 20.0),
                mass: 20,
                ..AsteroidType::MISSING
            };
            let mut field = Asteroids::default();
            for (slot, asteroid) in placed {
                field.seat(slot, asteroid);
            }
            Self { field, types }
        }

        /// The asteroid in `slot`, if any.
        fn get(&self, slot: usize) -> Option<Asteroid> {
            self.field.get(slot).copied()
        }

        /// The asteroid in `slot`.
        fn at(&self, slot: usize) -> Asteroid {
            self.get(slot)
                .unwrap_or_else(|| panic!("slot {slot}: {self:?}"))
        }

        /// The field, lent.
        fn lend(&mut self) -> AsteroidField<'_> {
            AsteroidField {
                asteroids: &mut self.field,
                types: &self.types,
            }
        }
    }

    /// Where each of `remains` was, in order.
    fn remains_at(remains: &[Remains]) -> Vec<Vec2> {
        remains.iter().map(|left| left.at).collect()
    }

    /// [`blaster`] exploding in `bööm` 130 with an `Impact` of 10, its
    /// hits throwing off 5 particles of `HitPartVel` 150, `HitPartLife`
    /// 30 and orange.
    fn rock_gun() -> WeaponRecord {
        WeaponRecord {
            explod_type: 2,
            impact: 10,
            hit_particles: 5,
            hit_part_vel: 150,
            hit_part_life: 30,
            hit_part_color: 0x00FF_8000,
            ..blaster()
        }
    }

    /// A tick among `ships` and `rocks`, with `arsenal`, by `rules`.
    fn tick_amid(
        combat: &mut Combat,
        ships: &mut [Ship],
        arsenal: &Arsenal,
        rules: Rules,
        rocks: &mut Rocks,
    ) {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        let scenery = Scenery {
            asteroids: Some(rocks.lend()),
            ..Scenery::default()
        };
        combat.tick(
            &mut fighters,
            arsenal,
            &Governments::default(),
            rules,
            Field::default(),
            scenery,
            &mut NeverFires,
        );
    }

    /// `firer` at the centre fires `record` once to the right, among
    /// `others`; then the fight goes on among `rocks` until the shot has
    /// had `ticks` ticks of flight, the player's from the tick it fires and
    /// an NPC's from the next (`_HandleShot` @0x44385 before `_HandleShip`
    /// @0x44483), with `record` and its sub-munition, weapon 148, in the
    /// arsenal. The ships and the events, in order.
    fn shoot_rocks(
        combat: &mut Combat,
        firer: ShipRef,
        record: WeaponRecord,
        others: Vec<Ship>,
        rocks: &mut Rocks,
        ticks: u32,
    ) -> (Vec<Ship>, Vec<CombatEvent>) {
        let sub = WeaponRecord {
            count: 50,
            speed: 500,
            ..weapon(148)
        };
        let arsenal = Arsenal::new(&[record, sub], Vec::new());
        let mut gun = Ship::at(1, 0.0, 0.0).armed(record);
        gun.id = firer;
        gun.fleet = firer;
        let mut ships = vec![gun];
        ships.extend(others);
        let mut events = Vec::new();
        let waits = u32::from(firer != ShipRef::Player);
        for _ in 0..ticks + waits {
            tick_amid(combat, &mut ships, &arsenal, Rules::default(), rocks);
            ships[0].trigger = QUIET;
            events.extend(combat.take_events());
        }
        (ships, events)
    }

    /// The events but the firing.
    fn unfired(events: &[CombatEvent]) -> Vec<CombatEvent> {
        events
            .iter()
            .filter(|event| !matches!(event, CombatEvent::Fired { .. }))
            .copied()
            .collect()
    }

    #[test]
    fn a_shot_that_meets_an_asteroid_damages_pushes_explodes_sprays_and_is_gone() {
        let mut combat = Combat::default();
        let mut rocks = Rocks::of([rock(3, 100.0, 0.0)]);
        let shell = WeaponRecord {
            sub_count: 2,
            sub_type: Some(WeaponId(148)),
            ..rock_gun()
        };
        let (_, events) = shoot_rocks(&mut combat, A, shell, Vec::new(), &mut rocks, 8);
        let left = rocks.at(3);
        assert_eq!(left.strength, 6, "its 4 mass damage, none of its 10 energy");
        assert!(
            (left.velocity - Vec2::new(0.5, 0.0)).length() < 1e-5,
            "{left:?}"
        );
        assert_eq!(combat.shots(), [], "used up");
        let seen = unfired(&events);
        let Some(&CombatEvent::Exploded { at, .. }) = seen.first() else {
            panic!("{seen:?}");
        };
        assert!((at - Vec2::new(90.0, 0.0)).length() < 1e-3, "{at:?}");
        assert_eq!(
            seen,
            [
                CombatEvent::Exploded {
                    at,
                    explosion: Explosion::decode(2).expect("bööm 130"),
                    size: 0.0,
                },
                CombatEvent::Particles(ParticleBurst {
                    at,
                    speed: (150.0_f64 * 0.01) as f32,
                    variation: 20,
                    life: (30, 37),
                    color: 0x00FF_8000,
                    count: 5,
                    spread: 0.0,
                }),
            ]
        );
        assert_eq!(fired(&events, A), 1, "no sub-munitions released");
        assert_eq!(combat.take_asteroid_remains(), [], "none destroyed");
    }

    #[test]
    fn a_hit_without_hit_particles_or_an_explosion_raises_neither() {
        let mut combat = Combat::default();
        let mut rocks = Rocks::of([rock(0, 100.0, 0.0)]);
        let (_, events) = shoot_rocks(&mut combat, A, blaster(), Vec::new(), &mut rocks, 8);
        assert_eq!(rocks.at(0).strength, 6);
        assert_eq!(unfired(&events), []);
    }

    #[test]
    fn an_asteroid_is_not_pushed_without_an_impact_or_a_mass() {
        for (impact, mass) in [(0, 20), (10, 0), (10, -5)] {
            let mut combat = Combat::default();
            let (slot, drifting) = rock(0, 100.0, 0.0);
            let mut rocks = Rocks::of([(
                slot,
                Asteroid {
                    velocity: Vec2::new(0.25, 0.0),
                    ..drifting
                },
            )]);
            rocks.types[0].mass = mass;
            let gun = WeaponRecord {
                impact,
                ..blaster()
            };
            shoot_rocks(&mut combat, A, gun, Vec::new(), &mut rocks, 8);
            assert_eq!(rocks.at(0).strength, 6, "{impact} {mass}");
            assert_eq!(
                rocks.at(0).velocity,
                Vec2::new(0.25, 0.0),
                "{impact} {mass}"
            );
        }
    }

    /// [`blaster`] at 100 pixels a tick, with a `ProxRadius` of `prox`.
    fn fast_gun(prox: i16) -> WeaponRecord {
        WeaponRecord {
            speed: 10_000,
            prox_radius: prox,
            ..blaster()
        }
    }

    #[test]
    fn a_ship_the_shot_touches_is_hit_before_an_asteroid_nearer_along_its_path() {
        let mut combat = Combat::default();
        let mut rocks = Rocks::of([rock(0, 40.0, 0.0)]);
        let (ships, _) = shoot_rocks(
            &mut combat,
            A,
            fast_gun(0),
            vec![Ship::at(2, 90.0, 0.0)],
            &mut rocks,
            2,
        );
        assert_eq!(rocks.at(0).strength, 10, "untouched");
        assert!(ships[1].reserves.shield.now < 10.0, "the ship took it");
    }

    #[test]
    fn an_asteroid_the_shot_touches_is_hit_before_a_ship_that_only_sets_its_fuse_off() {
        let mut combat = Combat::default();
        let mut rocks = Rocks::of([rock(0, 80.0, 0.0)]);
        // The move ends 30 from B: within its fuse's reach of
        // trunc(30 + 32 x 0.333) = 40, beyond its 16-pixel radius.
        let (ships, _) = shoot_rocks(
            &mut combat,
            A,
            fast_gun(30),
            vec![Ship::at(2, 100.0, 30.0)],
            &mut rocks,
            2,
        );
        assert_eq!(rocks.at(0).strength, 6);
        assert_eq!(ships[1].reserves, Reserves::full(10.0, 30.0, 100.0));
    }

    #[test]
    fn a_ship_that_sets_the_fuse_off_is_hit_before_an_asteroid_in_the_fuses_reach() {
        let mut combat = Combat::default();
        // Off the path, but 25 from where the shot ends.
        let mut rocks = Rocks::of([rock(0, 100.0, 25.0)]);
        let (ships, _) = shoot_rocks(
            &mut combat,
            A,
            fast_gun(30),
            vec![Ship::at(2, 100.0, 30.0)],
            &mut rocks,
            2,
        );
        assert_eq!(rocks.at(0).strength, 10);
        assert!(ships[1].reserves.shield.now < 10.0);
    }

    #[test]
    fn an_armed_fuse_with_no_ship_in_reach_takes_the_asteroid_where_the_shot_is() {
        let mut combat = Combat::default();
        let mut rocks = Rocks::of([rock(4, 100.0, 25.0)]);
        let gun = WeaponRecord {
            explod_type: 2,
            ..fast_gun(30)
        };
        let (_, events) = shoot_rocks(&mut combat, A, gun, Vec::new(), &mut rocks, 2);
        assert_eq!(rocks.at(4).strength, 6);
        let seen = unfired(&events);
        assert_eq!(seen.len(), 1, "{seen:?}");
        let CombatEvent::Exploded { at, explosion, .. } = seen[0] else {
            panic!("{seen:?}");
        };
        assert!((at - Vec2::new(100.0, 0.0)).length() < 1e-3, "{at:?}");
        assert_eq!(Some(explosion), Explosion::decode(2));
        assert_eq!(combat.shots(), []);
    }

    #[test]
    fn the_fuse_rule_decides_whether_flags2_0x0004_spares_asteroids() {
        for (fuse, hit) in [(AsteroidFuse::Original, true), (AsteroidFuse::Bible, false)] {
            let mut combat = Combat::default();
            combat.set_asteroid_fuse(fuse);
            let mut rocks = Rocks::of([rock(0, 100.0, 25.0)]);
            let gun = WeaponRecord {
                flags2: 0x0004,
                ..fast_gun(30)
            };
            shoot_rocks(&mut combat, A, gun, Vec::new(), &mut rocks, 2);
            assert_eq!(rocks.at(0).strength < 10, hit, "{fuse:?}");
            assert_eq!(combat.shots().is_empty(), hit, "{fuse:?}");
        }
    }

    #[test]
    fn a_second_shot_of_the_walk_is_spent_on_the_asteroid_the_first_destroyed() {
        // `_SWCollideSpriteLayer` reads the asteroid's visibility once, as
        // its turn begins (@0xacba3): the second shot of the same walk
        // meets its struct, below 0 still, and destroys it again
        // (@0x3691a).
        let mut combat = Combat::default();
        let mut rocks = Rocks::of([worn(5, 100.0, 0.0, 2)]);
        let arsenal = Arsenal::default();
        // A and its escort C side by side, neither's shots hitting the
        // other (`sides`: a ship led by the firer, or the firer's leader).
        let mut escort = Ship::at(3, 0.0, 0.0).armed(fast_gun(0));
        escort.fleet = A;
        escort.leader = Some(A);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(fast_gun(0)), escort];
        // Both meet it on their first move, the tick after they are fired.
        tick_amid(
            &mut combat,
            &mut ships,
            &arsenal,
            Rules::default(),
            &mut rocks,
        );
        for ship in &mut ships {
            ship.trigger = QUIET;
        }
        for _ in 0..2 {
            tick_amid(
                &mut combat,
                &mut ships,
                &arsenal,
                Rules::default(),
                &mut rocks,
            );
        }
        assert_eq!(rocks.get(5), None, "destroyed");
        assert_eq!(combat.shots(), [], "both spent on it");
        let at = Vec2::new(100.0, 0.0);
        assert_eq!(remains_at(&combat.take_asteroid_remains()), [at, at]);
        assert_eq!(combat.take_asteroid_remains(), [], "taken");
    }

    #[test]
    fn asteroids_destroyed_in_a_tick_leave_their_remains_in_the_order_they_were_destroyed() {
        let mut combat = Combat::default();
        let mut rocks = Rocks::of([worn(2, 100.0, 50.0, 1), worn(5, 100.0, 0.0, 1)]);
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(fast_gun(0)),
            Ship::at(3, 0.0, 50.0).armed(fast_gun(0)),
        ];
        for _ in 0..3 {
            tick_amid(
                &mut combat,
                &mut ships,
                &Arsenal::default(),
                Rules::default(),
                &mut rocks,
            );
            for ship in &mut ships {
                ship.trigger = QUIET;
            }
        }
        // The asteroids are walked in slot order, slot 2's first, though
        // A's shot comes first in the shots layer.
        assert_eq!(
            remains_at(&combat.take_asteroid_remains()),
            [Vec2::new(100.0, 50.0), Vec2::new(100.0, 0.0)]
        );
        assert_eq!(rocks.field.active(), 0);
    }

    /// A gun of the player's with a `BlastRadius` of 30 and these `Flags`
    /// and `Flags2`.
    fn blasting(flags: u16, flags2: u16) -> WeaponRecord {
        WeaponRecord {
            blast_radius: 30,
            flags,
            flags2,
            ..blaster()
        }
    }

    /// The ships struck when `firer` shoots `weapon` at an asteroid at
    /// (100, 0), with NPCs B in the blast's square, C outside it and D,
    /// planet-type, in it: the strikes, in order.
    fn blasted(firer: ShipRef, weapon: WeaponRecord) -> Vec<Strike> {
        let mut combat = Combat::default();
        let mut rocks = Rocks::of([rock(0, 100.0, 0.0)]);
        let mut planet = Ship::at(4, 100.0, -25.0);
        planet.hull.planet_type = true;
        let others = vec![Ship::at(2, 110.0, 20.0), Ship::at(3, 100.0, 60.0), planet];
        shoot_rocks(&mut combat, firer, weapon, others, &mut rocks, 8);
        combat.take_strikes()
    }

    #[test]
    fn a_players_shot_at_an_asteroid_blasts_the_ships_in_its_square_but_a_planet_type_one() {
        let strikes = blasted(ShipRef::Player, blasting(0, 0));
        assert_eq!(
            strikes,
            [Strike {
                ship: B,
                by: Some(ShipRef::Player),
                damage: 14.0,
                downed: None,
                // `_DamageShip` @0x36888: real 0, intended 0.
                direct: false,
                intended: false,
                lock: false,
                nominal: Nominal {
                    mass: 4.0,
                    energy: 10.0,
                },
                weight: 0.0,
                answered: false,
            }]
        );
    }

    #[test]
    fn a_disable_only_blast_at_an_asteroid_never_destroys_the_ship_it_reaches() {
        let mut combat = Combat::default();
        let mut rocks = Rocks::of([rock(0, 100.0, 0.0)]);
        let frail = holding(2, 110.0, 20.0, 0.0, 2.0, 30.0);
        let (ships, _) = shoot_rocks(
            &mut combat,
            ShipRef::Player,
            blasting(0, DISABLE_ONLY),
            vec![frail],
            &mut rocks,
            8,
        );
        assert_eq!(combat.take_strikes().len(), 1);
        assert_eq!(ships[1].reserves.armor.now, damage::SPARED_ARMOR);
    }

    #[test]
    fn an_npcs_shot_or_a_planet_type_weapons_blasts_nobody_through_an_asteroid() {
        assert_eq!(blasted(A, blasting(0, 0)), []);
        assert_eq!(blasted(ShipRef::Player, blasting(0, PLANET_TYPE)), []);
    }

    #[test]
    fn the_players_blast_at_an_asteroid_hurts_the_player_unless_its_weapon_spares_it() {
        for (flags, hurt) in [(0, true), (0x0100, false)] {
            let mut combat = Combat::default();
            // Met at (10, 0): the player in the square.
            let mut rocks = Rocks::of([rock(0, 20.0, 0.0)]);
            shoot_rocks(
                &mut combat,
                ShipRef::Player,
                blasting(flags, 0),
                Vec::new(),
                &mut rocks,
                2,
            );
            let strikes = combat.take_strikes();
            assert_eq!(
                strikes.iter().any(|strike| strike.ship == ShipRef::Player),
                hurt,
                "{flags:#x}: {strikes:?}"
            );
        }
    }

    #[test]
    fn a_point_defence_shot_meets_an_asteroid_on_its_way_to_a_missile() {
        let rule = Hostility {
            hostile: true,
            ..Hostility::default()
        };
        let rules = Rules {
            defence: &rule,
            ..Rules::default()
        };
        let mut combat = Combat::default();
        let mut ships = missile_attack(quad());
        let mut rocks = Rocks::of([rock(0, 0.0, -40.0)]);
        for _ in 0..12 {
            tick_amid(
                &mut combat,
                &mut ships,
                &Arsenal::default(),
                rules,
                &mut rocks,
            );
            ships[1].trigger = QUIET;
        }
        assert!(rocks.at(0).strength < 10, "{rocks:?}");
        let downed = combat
            .take_events()
            .iter()
            .filter(|event| matches!(event, CombatEvent::ShotDown { .. }))
            .count();
        assert_eq!(downed, 0, "its shots spent on the asteroid");
    }

    // Hits and explosions (weapons-fidelity phase 14).

    /// `record` exploding as Hellhound's do: `ExplodType` 1014, extras
    /// about `bööm` 142, and a `BlastRadius` of 145.
    fn hellhound(record: WeaponRecord) -> WeaponRecord {
        WeaponRecord {
            explod_type: 1014,
            blast_radius: 145,
            ..record
        }
    }

    /// The size of each explosion among `events`, in order.
    fn sizes(events: &[CombatEvent]) -> Vec<f32> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Exploded { size, .. } => Some(size),
                _ => None,
            })
            .collect()
    }

    /// The events of the tick A at the centre fires `record` once to the
    /// right among `others`, and of `ticks` ticks more, its shot first
    /// moving on the first of them, as an NPC's shot first moves the tick
    /// after its pick (`_HandleShot` @0x44385 before `_HandleShip`
    /// @0x44483).
    fn fight_once(record: WeaponRecord, others: Vec<Ship>, ticks: u32) -> Vec<CombatEvent> {
        let mut combat = Combat::default();
        let mut ships = vec![Ship::at(1, 0.0, 0.0).armed(record)];
        ships.extend(others);
        let mut events = Vec::new();
        for _ in 0..=ticks {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
            events.extend(combat.take_events());
        }
        events
    }

    #[test]
    fn a_shots_contact_hit_explodes_at_its_blast_radius() {
        let events = fight_once(hellhound(blaster()), vec![Ship::at(2, 100.0, 0.0)], 8);
        assert_eq!(sizes(&events), [145.0], "{events:?}");
    }

    #[test]
    fn a_shots_proximity_hit_explodes_at_its_blast_radius() {
        // The first move ends 30 from B: within trunc(30 + 32 x 0.333) =
        // 40, beyond its 16-pixel radius. The next tick's pass tests it.
        let events = fight_once(hellhound(fast_gun(30)), vec![Ship::at(2, 100.0, 30.0)], 2);
        assert_eq!(sizes(&events), [145.0], "{events:?}");
    }

    #[test]
    fn a_shot_detonating_at_its_end_explodes_at_its_blast_radius() {
        let flak = WeaponRecord {
            count: 3,
            flags: 0x8000,
            ..hellhound(blaster())
        };
        let events = fight_once(flak, Vec::new(), 4);
        assert_eq!(sizes(&events), [145.0], "{events:?}");
    }

    #[test]
    fn a_beams_hit_on_a_ship_explodes_at_its_blast_radius_every_tick() {
        let events = fight_once(hellhound(laser()), vec![Ship::at(2, 100.0, 0.0)], 5);
        assert_eq!(sizes(&events), [145.0; 3], "{events:?}");
    }

    #[test]
    fn a_shots_hit_on_an_asteroid_explodes_at_its_blast_radius() {
        let mut combat = Combat::default();
        let mut rocks = Rocks::of([rock(0, 100.0, 0.0)]);
        let (_, events) = shoot_rocks(
            &mut combat,
            A,
            hellhound(blaster()),
            Vec::new(),
            &mut rocks,
            8,
        );
        assert_eq!(sizes(&events), [145.0], "{events:?}");
    }

    #[test]
    fn a_planet_type_shots_hit_on_a_stellar_explodes_at_its_blast_radius() {
        let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
        let (_, _, events, _) = shoot_once(
            hellhound(planet_gun()),
            Vec::new(),
            &mut stellars,
            &Arsenal::default(),
            7,
        );
        assert_eq!(sizes(&events), [145.0], "{events:?}");
    }

    /// A hit raised among `events`: the ship, the weapon, where, and
    /// whether it passes shields.
    type Raised = (ShipRef, Option<WeaponId>, Vec2, bool);

    /// The hits raised among `events`, in order.
    fn raised(events: &[CombatEvent]) -> Vec<Raised> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Hit {
                    ship,
                    weapon,
                    at,
                    passes_shields,
                    ..
                } => Some((ship, weapon, at, passes_shields)),
                _ => None,
            })
            .collect()
    }

    /// Each hit raised among `events`: the ship, and whether the original
    /// shows its flash a frame late.
    fn lateness(events: &[CombatEvent]) -> Vec<(ShipRef, bool)> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Hit { ship, late, .. } => Some((ship, late)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_shots_hit_on_an_npc_shows_its_flash_that_frame_and_a_beams_the_next() {
        let events = fight_once(blaster(), vec![Ship::at(2, 100.0, 0.0)], 8);
        assert_eq!(lateness(&events), [(B, false)]);
        let events = fight_once(laser(), vec![Ship::at(2, 100.0, 0.0)], 5);
        assert_eq!(lateness(&events), [(B, true); 3]);
    }

    #[test]
    fn any_hit_on_the_player_shows_its_flash_the_next_frame() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()),
            Ship {
                id: ShipRef::Player,
                fleet: ShipRef::Player,
                ..Ship::at(0, 100.0, 0.0)
            },
        ];
        let mut events = Vec::new();
        for _ in 0..8 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
            events.extend(combat.take_events());
        }
        assert_eq!(lateness(&events), [(ShipRef::Player, true)]);
    }

    #[test]
    fn a_missile_lock_raises_a_hit_on_its_target_that_flashes_its_shield() {
        let mut combat = Combat::default();
        let mut ships = [player_with(missile()), far_b()];
        let mut events = Vec::new();
        for _ in 0..40 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
            events.extend(combat.take_events());
        }
        assert_eq!(
            raised(&events),
            [(B, Some(WeaponId(134)), Vec2::new(5000.0, 0.0), false)],
            "no damage, but passes no shields"
        );
        assert_eq!(lateness(&events), [(B, false)]);
    }

    /// Whether `a` and `b` are within a thousandth of a pixel.
    fn within(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn a_shots_direct_hit_and_its_blast_raise_a_hit_on_each_ship_they_land_on() {
        let shell = WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            ..blaster()
        };
        let events = fight_once(
            shell,
            vec![Ship::at(2, 100.0, 0.0), Ship::at(3, 100.0, 25.0)],
            7,
        );
        let hits = raised(&events);
        let ships: Vec<ShipRef> = hits.iter().map(|hit| hit.0).collect();
        assert_eq!(ships, [B, C], "the direct hit, then the blast: {hits:?}");
        for (_, weapon, at, passes) in hits {
            assert_eq!(weapon, Some(WeaponId(128)));
            assert!(within(at, Vec2::new(84.0, 0.0)), "where it met B: {at:?}");
            assert!(!passes);
        }
    }

    #[test]
    fn a_beam_raises_a_hit_at_its_end_every_tick_it_touches_a_ship() {
        let events = fight_once(laser(), vec![Ship::at(2, 100.0, 0.0)], 5);
        let hits = raised(&events);
        assert_eq!(hits.len(), 3, "{hits:?}");
        for (ship, weapon, at, passes) in hits {
            assert_eq!((ship, weapon, passes), (B, Some(WeaponId(146)), false));
            assert!(within(at, Vec2::new(85.0, 0.0)), "{at:?}");
        }
    }

    #[test]
    fn a_hit_of_a_weapon_that_passes_shields_says_so() {
        let piercing = WeaponRecord {
            flags: crate::combat::weapon::PASSES_SHIELDS,
            ..blaster()
        };
        for (record, passes) in [(piercing, true), (blaster(), false)] {
            let events = fight_once(record, vec![Ship::at(2, 100.0, 0.0)], 8);
            let hits = raised(&events);
            assert_eq!(hits.len(), 1, "{hits:?}");
            assert_eq!(hits[0].3, passes);
        }
    }

    #[test]
    fn a_players_blast_at_an_asteroid_raises_a_hit_of_its_weapon_at_the_blasts_centre() {
        let piercing = crate::combat::weapon::PASSES_SHIELDS;
        for (flags, passes) in [(0, false), (piercing, true)] {
            let mut combat = Combat::default();
            let mut rocks = Rocks::of([rock(0, 107.0, 0.0)]);
            let (_, events) = shoot_rocks(
                &mut combat,
                ShipRef::Player,
                blasting(flags, 0),
                vec![Ship::at(2, 110.0, 20.0)],
                &mut rocks,
                8,
            );
            let hits = raised(&events);
            assert_eq!(hits.len(), 1, "{flags:#x}: {hits:?}");
            let (ship, weapon, at, passes_shields) = hits[0];
            assert_eq!(
                (ship, weapon, passes_shields),
                (B, Some(WeaponId(128)), passes),
                "{flags:#x}"
            );
            // Where the shot met the asteroid's edge, not where it flew
            // to by its 15-pixel steps.
            assert!(within(at, Vec2::new(97.0, 0.0)), "{flags:#x}: {at:?}");
        }
    }

    #[test]
    fn a_shockwave_raises_a_hit_of_no_weapon_at_the_wreck_on_each_ship_it_hurts() {
        let mut combat = Combat::default();
        let mut ships = [
            wreck(WRECK, 200.0, 0.0, 0.0),
            holding(1, 40.0, 0.0, 100.0, 300.0, 300.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        let hits = raised(&combat.take_events());
        assert_eq!(hits, [(A, None, Vec2::ZERO, false)]);
    }

    #[test]
    fn the_escape_pod_and_the_classes_after_it_raise_no_hit() {
        let shell = WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            ..blaster()
        };
        let after = ShipId(ESCAPE_POD_SHIP.0 + 1);
        for ship_type in [ESCAPE_POD_SHIP, after] {
            // A blast reaches it, and a beam the class after the pod.
            let events = fight_once(
                shell,
                vec![
                    Ship::at(4, 100.0, 0.0),
                    pod(Ship::at(3, 100.0, 25.0), ship_type),
                ],
                7,
            );
            let ships: Vec<ShipRef> = raised(&events).iter().map(|hit| hit.0).collect();
            assert_eq!(ships, [ShipRef::Npc(NpcId(4))], "{ship_type:?}");
            let events = fight_once(laser(), vec![pod(Ship::at(2, 100.0, 0.0), after)], 5);
            assert_eq!(raised(&events), [], "{ship_type:?}");
            let mut combat = Combat::default();
            let mut ships = [
                wreck(WRECK, 200.0, 0.0, 0.0),
                pod(holding(1, 40.0, 0.0, 100.0, 300.0, 300.0), ship_type),
            ];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(raised(&combat.take_events()), [], "{ship_type:?}");
        }
    }

    /// The bursts of particles among `events`, in order.
    fn bursts(events: &[CombatEvent]) -> Vec<ParticleBurst> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Particles(burst) => Some(burst),
                _ => None,
            })
            .collect()
    }

    /// `record` throwing off [`rock_gun`]'s particles where it hits.
    fn spraying(record: WeaponRecord) -> WeaponRecord {
        let gun = rock_gun();
        WeaponRecord {
            hit_particles: gun.hit_particles,
            hit_part_vel: gun.hit_part_vel,
            hit_part_life: gun.hit_part_life,
            hit_part_color: gun.hit_part_color,
            ..record
        }
    }

    /// [`rock_gun`]'s burst at `at`, its speed varied by `variation`.
    fn sprayed(at: Vec2, variation: u32) -> ParticleBurst {
        ParticleBurst {
            at,
            speed: (150.0_f64 * 0.01) as f32,
            variation,
            life: (30, 37),
            color: 0x00FF_8000,
            count: 5,
            spread: 0.0,
        }
    }

    #[test]
    fn a_shots_contact_hit_on_a_ship_sprays_its_particles_where_it_met_it() {
        let events = fight_once(spraying(blaster()), vec![Ship::at(2, 100.0, 0.0)], 8);
        let seen = bursts(&events);
        assert_eq!(seen.len(), 1, "{events:?}");
        assert!(within(seen[0].at, Vec2::new(84.0, 0.0)), "{:?}", seen[0]);
        assert_eq!(seen[0], sprayed(seen[0].at, 20));
    }

    #[test]
    fn a_shots_proximity_hit_on_a_ship_sprays_its_particles_where_the_shot_is() {
        let events = fight_once(spraying(fast_gun(30)), vec![Ship::at(2, 100.0, 30.0)], 2);
        let seen = bursts(&events);
        assert_eq!(seen.len(), 1, "{events:?}");
        let hits = raised(&events);
        assert_eq!(seen[0], sprayed(hits[0].2, 20), "at the meeting");
    }

    #[test]
    fn a_beam_tick_sprays_its_particles_at_its_end_varied_25_percent() {
        let events = fight_once(spraying(laser()), vec![Ship::at(2, 100.0, 0.0)], 5);
        let seen = bursts(&events);
        assert_eq!(seen.len(), 3, "a burst a tick: {events:?}");
        for burst in seen {
            assert!(within(burst.at, Vec2::new(85.0, 0.0)), "{burst:?}");
            assert_eq!(burst, sprayed(burst.at, 25));
        }
    }

    #[test]
    fn a_planet_type_shots_hit_on_a_stellar_sprays_its_particles() {
        let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
        let (_, _, events, _) = shoot_once(
            spraying(planet_gun()),
            Vec::new(),
            &mut stellars,
            &Arsenal::default(),
            7,
        );
        let seen = bursts(&events);
        assert_eq!(seen.len(), 1, "{events:?}");
        let at = explosions(&events)[0].0;
        assert_eq!(seen[0], sprayed(at, 20), "where it exploded");
    }

    #[test]
    fn a_weapon_without_hit_particles_sprays_none() {
        let events = fight_once(blaster(), vec![Ship::at(2, 100.0, 0.0)], 8);
        assert_eq!(raised(&events).len(), 1, "it hit");
        assert_eq!(bursts(&events), []);
        let events = fight_once(laser(), vec![Ship::at(2, 100.0, 0.0)], 5);
        assert_eq!(bursts(&events), []);
    }

    #[test]
    fn the_ships_a_blast_reaches_spray_no_particles_of_their_own() {
        let shell = WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            ..spraying(blaster())
        };
        let events = fight_once(
            shell,
            vec![Ship::at(2, 100.0, 0.0), Ship::at(3, 100.0, 25.0)],
            7,
        );
        assert_eq!(raised(&events).len(), 2, "B hit, C blasted");
        assert_eq!(bursts(&events).len(), 1, "{events:?}");
    }

    /// Where each shot fizzled among `events`, in order.
    fn fizzles(events: &[CombatEvent]) -> Vec<Vec2> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Fizzled { at } => Some(at),
                _ => None,
            })
            .collect()
    }

    /// [`blaster`] of `explod_type` living 3 ticks at 10 pixels a tick,
    /// under `flags`.
    fn short_shot(explod_type: i16, flags: u16) -> WeaponRecord {
        WeaponRecord {
            count: 3,
            speed: 1000,
            explod_type,
            flags,
            ..blaster()
        }
    }

    #[test]
    fn a_shot_that_ends_its_life_without_detonating_fizzles_where_it_is() {
        for explod_type in [1, 500, 1014] {
            let events = fight_once(short_shot(explod_type, 0), Vec::new(), 4);
            let seen = fizzles(&events);
            assert_eq!(seen.len(), 1, "{explod_type}: {events:?}");
            assert!(within(seen[0], Vec2::new(30.0, 0.0)), "{seen:?}");
            assert!(sizes(&events).is_empty(), "{explod_type}: no explosion");
        }
    }

    #[test]
    fn a_shot_of_explod_type_0_or_below_never_fizzles() {
        for explod_type in [0, -1] {
            let events = fight_once(short_shot(explod_type, 0), Vec::new(), 4);
            assert_eq!(fizzles(&events), [], "{explod_type}");
        }
    }

    #[test]
    fn a_detonating_shot_explodes_and_does_not_fizzle() {
        let events = fight_once(short_shot(1, 0x8000), Vec::new(), 4);
        assert_eq!(fizzles(&events), []);
        assert_eq!(sizes(&events).len(), 1, "it exploded");
    }

    #[test]
    fn a_shot_that_hits_never_fizzles() {
        let events = fight_once(
            WeaponRecord {
                explod_type: 1,
                ..blaster()
            },
            vec![Ship::at(2, 100.0, 0.0)],
            16,
        );
        assert_eq!(raised(&events).len(), 1, "it hit");
        assert_eq!(fizzles(&events), []);
    }

    #[test]
    fn a_missile_point_defence_destroys_never_fizzles() {
        let [defender, attacker] = missile_attack(quad());
        let missile = WeaponRecord {
            speed: 500,
            explod_type: 1,
            ..missile()
        };
        let mut ships = [defender, attacker.armed(drawn(missile, 18))];
        let mut combat = Combat::default();
        let mut events = Vec::new();
        for _ in 0..30 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[1].trigger = QUIET;
            events.extend(combat.take_events());
        }
        assert!(
            events
                .iter()
                .any(|event| matches!(event, CombatEvent::ShotDown { .. })),
            "{events:?}"
        );
        assert_eq!(fizzles(&events), []);
    }

    #[test]
    fn a_shot_of_no_life_is_never_shown_so_ends_without_fizzle_detonation_or_release() {
        let sub = WeaponRecord {
            count: 50,
            speed: 500,
            ..weapon(148)
        };
        for (count, shown) in [(0, false), (-3, false), (1, true)] {
            let record = WeaponRecord {
                count,
                sub_count: 2,
                sub_type: Some(WeaponId(148)),
                ..short_shot(1, 0)
            };
            let detonating = WeaponRecord {
                flags: 0x8000,
                ..record
            };
            let arsenal = Arsenal::new(&[record, detonating, sub], Vec::new());
            for record in [record, detonating] {
                let mut combat = Combat::default();
                let mut ships = [Ship::at(1, 0.0, 0.0).armed(record)];
                let mut events = Vec::new();
                for _ in 0..3 {
                    tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
                    ships[0].trigger = QUIET;
                    events.extend(combat.take_events());
                }
                let ended = fizzles(&events).len() + sizes(&events).len();
                assert_eq!(ended == 1, shown, "{count} {:#x}", record.flags);
                let released = events
                    .iter()
                    .any(|event| matches!(event, CombatEvent::Fired { weapon, .. } if *weapon == WeaponId(148)));
                assert_eq!(released, shown, "{count} {:#x}", record.flags);
            }
        }
    }

    // The player's fire (`_FirePlayerWeapon`), `AmmoType` -999 and the
    // caps (`_SpawnShot`, `_SpawnBeam`).

    use crate::combat::aim::{BLIND_FRONT, BLIND_REAR};
    use crate::combat::weapon::{SECONDARY, SIMULTANEOUS};

    /// The player at the centre, facing right, carrying `count` of
    /// `record` with 9 rounds of its own, its triggers as `trigger`.
    fn player_carrying(record: WeaponRecord, count: u32, trigger: Trigger) -> Ship {
        let mut player = player_with(record);
        player.armament = Armament::new([(WeaponSpec::new(&record), count)]);
        player.rounds = LauncherRounds::from([(record.id, 9)]);
        player.trigger = trigger;
        player
    }

    const PRIMARY_HELD: Trigger = Trigger {
        primary: true,
        ..QUIET
    };

    #[test]
    fn the_players_weapon_sounds_once_a_call_at_5_and_a_beam_bay_or_secondary_at_6() {
        let heard = |player: Ship| -> Vec<Option<u8>> {
            let mut combat = Combat::default();
            let mut ships = [player, Ship::at(2, 100.0, 0.0)];
            tick(&mut combat, &mut ships, &NovaDisable);
            sounds(&combat.take_events())
                .into_iter()
                .map(|(_, sound)| sound.map(|sound| sound.priority))
                .collect()
        };
        let together = WeaponRecord {
            flags: SIMULTANEOUS,
            ..blaster()
        };
        let player = player_carrying(together, 3, PRIMARY_HELD);
        assert_eq!(heard(player), [Some(5), None, None], "later copies unheard");
        for guidance in [0, 3] {
            let beam = WeaponRecord {
                guidance,
                count: 5,
                beam_length: 300,
                ..weapon(146)
            };
            let player = player_carrying(beam, 1, PRIMARY_HELD);
            assert_eq!(heard(player), [Some(6)], "{guidance}");
        }
        let missile = WeaponRecord {
            flags: SECONDARY,
            ..blaster()
        };
        let secondary = Trigger {
            secondary: Some(WeaponId(128)),
            ..QUIET
        };
        assert_eq!(heard(player_carrying(missile, 1, secondary)), [Some(6)]);
        let bay = WeaponRecord {
            guidance: 99,
            ammo_type: 200,
            flags: SECONDARY,
            ..weapon(150)
        };
        let launching = Trigger {
            secondary: Some(WeaponId(150)),
            ..QUIET
        };
        assert_eq!(heard(player_carrying(bay, 1, launching)), [Some(6)]);
        // A bay is heard at 6 for its guidance alone, primary or not.
        let primary_bay = WeaponRecord { flags: 0, ..bay };
        assert_eq!(
            heard(player_carrying(primary_bay, 1, PRIMARY_HELD)),
            [Some(6)]
        );
        assert_eq!(
            (PLAYER_FIRE_PRIORITY, PLAYER_SECONDARY_FIRE_PRIORITY),
            (5, 6)
        );
    }

    /// The pushes on the player, of mass 50, after it fires 2 of `record`
    /// together a tick, its forward recoil read as `forward`.
    fn recoiled(record: WeaponRecord, forward: RuleSource) -> Vec<Shove> {
        let together = WeaponRecord {
            flags: SIMULTANEOUS,
            ..record
        };
        let mut player = player_carrying(together, 2, PRIMARY_HELD);
        player.hull.mass = 50.0;
        let mut combat = Combat::default();
        combat.set_player_forward_recoil(forward);
        tick(&mut combat, &mut [player], &NovaDisable);
        combat.take_shoves()
    }

    #[test]
    fn the_players_recoil_shoves_it_back_once_a_call() {
        let kicking = WeaponRecord {
            recoil: 100,
            ..blaster()
        };
        for forward in RuleSource::ALL {
            let back = recoiled(kicking, forward);
            assert_eq!(back.len(), 1, "once, whatever its copies: {back:?}");
            assert_eq!(back[0].ship, ShipRef::Player);
            assert!(
                (back[0].velocity - Vec2::new(-2.0, 0.0)).length() < 1e-4,
                "{back:?}"
            );
        }
        // A recoiling turret with no target attempts nothing, and pushes
        // nothing.
        let turret = WeaponRecord {
            recoil: 100,
            ..turret(4)
        };
        let mut player = player_carrying(turret, 1, PRIMARY_HELD);
        player.target = None;
        player.hull.mass = 50.0;
        let mut combat = Combat::default();
        tick(&mut combat, &mut [player], &NovaDisable);
        assert_eq!(combat.take_shoves(), []);
    }

    #[test]
    fn a_players_negative_recoil_pushes_it_forward_only_by_the_other_reading() {
        let pulling = WeaponRecord {
            recoil: -100,
            ..blaster()
        };
        assert_eq!(recoiled(pulling, RuleSource::Engine), [], "the engine's");
        let forward = recoiled(pulling, RuleSource::Bible);
        assert_eq!(forward.len(), 1, "{forward:?}");
        assert!(
            (forward[0].velocity - Vec2::new(2.0, 0.0)).length() < 1e-4,
            "{forward:?}"
        );
        let none = WeaponRecord {
            recoil: -1,
            ..blaster()
        };
        assert_eq!(recoiled(none, RuleSource::Bible), [], "-1 is no recoil");
    }

    #[test]
    fn a_players_recoil_moves_the_shots_of_its_later_weapons() {
        let kicking = WeaponRecord {
            recoil: 100,
            ..blaster()
        };
        let later = WeaponRecord {
            speed: 0,
            id: WeaponId(129),
            ..blaster()
        };
        let mut player = player_with(kicking);
        player.armament =
            Armament::new([(WeaponSpec::new(&later), 1), (WeaponSpec::new(&kicking), 1)]);
        player.hull.mass = 50.0;
        player.target = None;
        let mut combat = Combat::default();
        tick(&mut combat, &mut [player], &NovaDisable);
        let still = combat
            .shots()
            .iter()
            .find(|shot| shot.weapon.id == WeaponId(129))
            .expect("fired");
        assert!(
            (still.velocity - Vec2::new(-2.0, 0.0)).length() < 1e-4,
            "fired after 128's recoil: {:?}",
            still.velocity
        );
    }

    /// The shots `id`, carrying a front-quadrant turret blind in front,
    /// fires on `trigger` at B dead ahead, its quadrant turrets read as
    /// `reading`.
    fn blind_front_shots(id: ShipRef, trigger: Trigger, reading: RuleSource) -> usize {
        let blind = WeaponRecord {
            flags: BLIND_FRONT,
            ..turret(7)
        };
        let shooter = Ship {
            id,
            fleet: id,
            trigger,
            ..Ship::at(1, 0.0, 0.0).armed(blind).targeting(B)
        };
        let mut combat = Combat::default();
        combat.set_quadrant_blind_spots(reading);
        let mut ships = [shooter, Ship::at(2, 100.0, 0.0)];
        tick(&mut combat, &mut ships, &NovaDisable);
        combat
            .shots()
            .iter()
            .filter(|shot| shot.firer == id)
            .count()
    }

    #[test]
    fn the_players_quadrant_turret_tests_its_blind_spots_only_by_the_other_reading() {
        let player = ShipRef::Player;
        assert_eq!(
            blind_front_shots(player, PRIMARY_HELD, RuleSource::Engine),
            1
        );
        assert_eq!(
            blind_front_shots(player, PRIMARY_HELD, RuleSource::Bible),
            0
        );
        // An NPC's primary trigger (`Armament::fire`); its pick is tested
        // for blind spots as its AI picks it (`_AIFireTurret` @0x80d0b).
        for reading in RuleSource::ALL {
            assert_eq!(
                blind_front_shots(A, PRIMARY_HELD, reading),
                0,
                "an NPC's primary trigger: {reading:?}"
            );
        }
        // A full turret holds fire in its blind spot under both readings.
        for reading in RuleSource::ALL {
            let blind = WeaponRecord {
                flags: BLIND_REAR,
                ..turret(4)
            };
            let mut combat = Combat::default();
            combat.set_quadrant_blind_spots(reading);
            let mut ships = [
                Ship {
                    id: ShipRef::Player,
                    fleet: ShipRef::Player,
                    ..Ship::at(1, 0.0, 0.0).armed(blind).targeting(B)
                },
                Ship::at(2, -100.0, 0.0),
            ];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(combat.shots(), [], "{reading:?}");
        }
    }

    /// `record` with `AmmoType` -999.
    fn self_destructing(record: WeaponRecord) -> WeaponRecord {
        WeaponRecord {
            ammo_type: -999,
            ..record
        }
    }

    #[test]
    fn a_shot_of_ammo_type_minus_999_breaks_its_firer_up_as_it_spawns() {
        let player = player_with(self_destructing(blaster()));
        let npc = Ship::at(1, 0.0, 0.0)
            .armed(self_destructing(blaster()))
            .picking(128);
        for shooter in [player, npc] {
            let id = shooter.id;
            let mut combat = Combat::default();
            let mut ships = [shooter, Ship::at(2, 0.0, -3000.0)];
            tick(&mut combat, &mut ships, &NovaDisable);
            let reserves = ships[0].reserves;
            assert_eq!((reserves.shield.now, reserves.armor.now), (0.0, 0.0));
            assert!(
                about(&combat.take_events(), id)
                    .iter()
                    .any(|event| matches!(event, CombatEvent::BreakingUp { .. })),
                "{id:?}"
            );
            assert_eq!(combat.shots().len(), 1, "the shot still flies");
            assert_eq!(combat.take_strikes(), [], "no strike: no one hit it");
        }
        assert_eq!(SELF_DESTRUCT, -999);
    }

    #[test]
    fn a_release_at_an_end_of_life_finds_the_slots_the_collision_passes_ended_freed() {
        let (shell, arsenal) = cluster(0);
        // A shot whose last move, its `age`th, ended at `at` from 10 to
        // the left, fired by A.
        let flown = |record: WeaponRecord, at: Vec2, age: u32| Shot {
            position: at,
            moved_from: Some(at - Vec2::new(10.0, 0.0)),
            age,
            ..Shot::launch(WeaponSpec::new(&record), A, A, &ShipState::default(), 90.0)
        };
        // One ends on B, and one inside the stellar, in the collision
        // passes; the shell's end of life comes after, in `_HandleShot`
        // (@0x36195), which has freed their slots (@0x3617c) by then.
        let endings = [
            ("on a ship", flown(blaster(), Vec2::new(100.0, 0.0), 1)),
            (
                "on a stellar",
                flown(planet_gun(), Vec2::new(0.0, 500.0), 1),
            ),
        ];
        for (what, ending) in endings {
            // 125 others, the shot that ends, and the shell its last move
            // spent: the shell alone still holds its slot as it releases.
            for capped in [false, true] {
                let released = 2;
                let mut combat = crowded(MAX_SHOTS - 3, capped);
                combat.put_in_flight(ending);
                combat.put_in_flight(flown(shell, Vec2::new(0.0, -3000.0), 3));
                let mut ships = [Ship::at(1, 0.0, 0.0), Ship::at(2, 100.0, 0.0)];
                let mut stellars = [stellar_at(0.0, 500.0, 20.0, 100)];
                tick_by(&mut combat, &mut ships, &arsenal, &mut stellars);
                assert_eq!(sub_shots(&combat).len(), released, "{what} {capped}");
            }
        }
    }

    #[test]
    fn a_release_at_an_end_of_life_finds_the_asteroid_and_point_defence_ends_freed() {
        let (shell, arsenal) = cluster(0);
        let flown = |record: WeaponRecord, at: Vec2, age: u32| Shot {
            position: at,
            moved_from: Some(at - Vec2::new(10.0, 0.0)),
            age,
            ..Shot::launch(WeaponSpec::new(&record), A, A, &ShipState::default(), 90.0)
        };
        let spent_shell = flown(shell, Vec2::new(0.0, -3000.0), 3);
        // A shot spent on an asteroid; and a point-defence shot and the
        // missile it downs, both ended.
        let on_a_rock = vec![flown(blaster(), Vec2::new(500.0, 0.0), 1)];
        let defended = vec![
            Shot {
                moved_from: Some(Vec2::new(0.0, 300.0)),
                age: 1,
                ..Shot::launch(
                    WeaponSpec::new(&pd_shot()),
                    A,
                    A,
                    &ShipState {
                        position: Vec2::new(0.0, 300.0),
                        ..ShipState::default()
                    },
                    0.0,
                )
            },
            Shot {
                moved_from: Some(Vec2::new(3.0, 300.0)),
                age: 1,
                ..Shot::launch(
                    WeaponSpec::new(&still_missile()),
                    B,
                    B,
                    &ShipState {
                        position: Vec2::new(3.0, 300.0),
                        ..ShipState::default()
                    },
                    0.0,
                )
            },
        ];
        for (what, endings) in [
            ("on an asteroid", on_a_rock),
            ("to point defence", defended),
        ] {
            // The others, the shots that end, and the shell: the shots
            // the collision passes ended are freed before the shell's end
            // of life, leaving room for both.
            for capped in [false, true] {
                let released = 2;
                let mut combat = crowded(MAX_SHOTS - 2 - endings.len(), capped);
                for ending in &endings {
                    combat.put_in_flight(*ending);
                }
                combat.put_in_flight(spent_shell);
                let mut ships = [Ship::at(1, 0.0, 1000.0)];
                let mut rocks = Rocks::of([rock(0, 500.0, 0.0)]);
                tick_amid(
                    &mut combat,
                    &mut ships,
                    &arsenal,
                    Rules::default(),
                    &mut rocks,
                );
                assert_eq!(sub_shots(&combat).len(), released, "{what} {capped}");
            }
        }
    }

    #[test]
    fn a_point_defence_shot_of_ammo_type_minus_999_breaks_its_firer_up() {
        let mut combat = Combat::default();
        let mut ships = missile_attack(self_destructing(quad()));
        missile_in_flight(&mut combat, &mut ships);
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].reserves.armor.now, 0.0);
        assert!(
            about(&combat.take_events(), A)
                .iter()
                .any(|event| matches!(event, CombatEvent::BreakingUp { .. }))
        );
    }

    #[test]
    fn a_sub_munition_or_beam_of_ammo_type_minus_999_spares_its_firer() {
        let (shell, _) = cluster(0);
        let sub = self_destructing(WeaponRecord {
            count: 50,
            speed: 500,
            ..weapon(148)
        });
        let arsenal = Arsenal::new(&[shell, sub], Vec::new());
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell)];
        // Released as the shell's end of life comes, the tick after its
        // third move, which is the fourth tick.
        for _ in 0..5 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            ships[0].trigger = QUIET;
        }
        assert_eq!(sub_shots(&combat).len(), 2, "released");
        assert_eq!(ships[0].reserves.armor.now, 30.0, "spared");
        let mut combat = Combat::default();
        let mut ships = [player_with(self_destructing(laser()))];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.beams().len(), 1);
        assert_eq!(ships[0].reserves.armor.now, 30.0, "a beam spares it");
    }

    /// `n` shots of a long-lived blaster, fired by no ship in the fight,
    /// in flight far from every ship.
    fn filler(n: usize) -> Vec<Shot> {
        let weapon = WeaponSpec::new(&WeaponRecord {
            count: 1000,
            speed: 0,
            ..weapon(140)
        });
        let far = ShipState {
            position: Vec2::new(50_000.0, 50_000.0),
            ..ShipState::default()
        };
        (0..n)
            .map(|_| Shot::launch(weapon, C, C, &far, 0.0))
            .collect()
    }

    /// A combat with `n` [`filler`] shots in flight, capped or not.
    fn crowded(n: usize, capped: bool) -> Combat {
        let mut combat = Combat::default();
        for shot in filler(n) {
            combat.put_in_flight(shot);
        }
        if capped {
            combat.set_combat_caps(RuleSource::Engine);
        }
        combat
    }

    /// A launcher reloading every 10 ticks, firing its own rounds.
    fn launcher() -> WeaponRecord {
        WeaponRecord {
            reload: 10,
            ammo_type: 10,
            ..blaster()
        }
    }

    #[test]
    fn uncapped_by_default_the_players_129th_shot_flies() {
        let mut combat = crowded(MAX_SHOTS, false);
        assert_eq!(combat, crowded(MAX_SHOTS, false), "the default");
        tick(&mut combat, &mut [player_with(blaster())], &NovaDisable);
        assert_eq!(combat.shots().len(), MAX_SHOTS + 1);
        let mut set = Combat::default();
        set.set_combat_caps(RuleSource::Bible);
        assert_eq!(set, Combat::default(), "the other reading");
        assert_eq!((MAX_SHOTS, MAX_BEAMS), (128, 64));
    }

    #[test]
    fn under_the_caps_a_shot_finding_the_table_full_fails_quietly_and_still_spends() {
        let player = player_with(launcher());
        let npc = Ship::at(1, 0.0, 0.0).armed(launcher()).picking(128);
        for mut shooter in [player, npc] {
            let id = shooter.id;
            // The player reads its ammunition's slot, an NPC its own.
            shooter.rounds = LauncherRounds::from([(WeaponId(128), 5), (WeaponId(138), 5)]);
            let mut combat = crowded(MAX_SHOTS, true);
            let mut ships = [shooter];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(combat.shots().len(), MAX_SHOTS, "{id:?}: nothing flies");
            let heard = sounds(&combat.take_events());
            assert_eq!(heard.len(), 1, "{id:?}: still heard");
            assert!(heard[0].1.is_some(), "{id:?}: sounded");
            let rounds = &ships[0].rounds;
            let left = rounds.slot(WeaponId(128)) + rounds.slot(WeaponId(138));
            assert_eq!(left, 9, "{id:?}: still spent");
            // The player counts its reload down after its trigger
            // (@0x6ae25), an NPC before its pick (@0x33896).
            let reload = if id == ShipRef::Player { 9.0 } else { 10.0 };
            assert_eq!(ships[0].armament.mounts()[0].reload, reload, "{id:?}");
        }
        let mut combat = crowded(MAX_SHOTS - 1, true);
        tick(&mut combat, &mut [player_with(blaster())], &NovaDisable);
        assert_eq!(combat.shots().len(), MAX_SHOTS, "the 128th flies");
    }

    #[test]
    fn under_the_caps_a_minus_999_shot_finding_the_table_full_spares_its_firer() {
        let mut combat = crowded(MAX_SHOTS, true);
        let mut ships = [player_with(self_destructing(blaster()))];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].reserves.armor.now, 30.0);
        assert_eq!(ships[0].condition, Condition::Intact);
    }

    /// `n` beams of a long-lived laser held by no ship in the fight.
    fn held_beams(n: usize) -> Vec<Beam> {
        let laser = WeaponSpec::new(&WeaponRecord {
            count: 1000,
            ..laser()
        });
        (0..n)
            .map(|_| Beam::launch(laser, C, C, &ShipState::default(), Aiming::Ahead))
            .collect()
    }

    #[test]
    fn under_the_caps_a_beam_finding_the_table_full_fails_quietly() {
        for capped in [false, true] {
            let mut combat = Combat {
                beams: held_beams(MAX_BEAMS),
                ..Combat::default()
            };
            if capped {
                combat.set_combat_caps(RuleSource::Engine);
            }
            let player = player_with(laser());
            let npc = Ship::at(1, 0.0, 0.0).armed(laser()).picking(146);
            let mut ships = [player, npc];
            tick(&mut combat, &mut ships, &NovaDisable);
            let fired = combat.beams().iter().filter(|beam| beam.firer != C).count();
            assert_eq!(fired, if capped { 0 } else { 2 }, "{capped}");
            assert_eq!(sounds(&combat.take_events()).len(), 2, "both heard");
            assert_eq!(ships[0].armament.mounts()[0].reload, 9.0);
        }
    }

    #[test]
    fn under_the_caps_a_release_stops_at_the_first_sub_munition_finding_no_slot() {
        let (shell, arsenal) = cluster(0);
        // 126 others, the shell and its first sub-munition: the shell holds
        // its slot as it releases them.
        for (capped, released) in [(false, 2), (true, 1)] {
            let mut combat = crowded(MAX_SHOTS - 2, capped);
            let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell)];
            let mut events = Vec::new();
            for _ in 0..5 {
                tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
                ships[0].trigger = QUIET;
                events.extend(combat.take_events());
            }
            assert_eq!(sub_shots(&combat).len(), released, "{capped}");
            let heard = sounds(&events)
                .into_iter()
                .filter(|&(weapon, _)| weapon == WeaponId(148))
                .count();
            assert_eq!(heard, 1, "the release is heard: {capped}");
        }
        let mut combat = crowded(MAX_SHOTS - 1, true);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell)];
        for _ in 0..5 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            ships[0].trigger = QUIET;
        }
        assert_eq!(
            sub_shots(&combat),
            [] as [&Shot; 0],
            "no slot for the first"
        );
    }

    #[test]
    fn under_the_caps_point_defence_finding_the_shot_table_full_fires_and_spends_nothing() {
        let launching = WeaponRecord {
            ammo_type: 10,
            burst_count: 3,
            ..quad()
        };
        for capped in [false, true] {
            // The missile takes the 128th slot.
            let mut combat = crowded(MAX_SHOTS - 1, capped);
            let mut ships = missile_attack(launching);
            ships[0].rounds = LauncherRounds::from([(WeaponId(133), 5)]);
            missile_in_flight(&mut combat, &mut ships);
            tick(&mut combat, &mut ships, &NovaDisable);
            let pd = sounds(&combat.take_events())
                .into_iter()
                .filter(|&(weapon, _)| weapon == WeaponId(133))
                .count();
            assert_eq!(pd, usize::from(!capped), "{capped}");
            assert_eq!(ships[0].rounds.slot(WeaponId(133)), 5 - i32::from(!capped));
            let mount = ships[0].armament.mounts()[0];
            assert_eq!(mount.burst, u32::from(!capped), "{capped}");
            assert_eq!(mount.reload > 0.0, !capped, "{capped}");
        }
    }

    #[test]
    fn a_shot_of_a_graphic_or_index_above_255_fails_quietly_and_still_spends() {
        let graphic = |graphic| WeaponRecord {
            graphic,
            ..launcher()
        };
        let index = WeaponRecord {
            id: WeaponId(384),
            ..launcher()
        };
        for record in [graphic(256), graphic(-1), index] {
            let id = record.id;
            for mut shooter in [
                player_with(record),
                Ship::at(1, 0.0, 0.0).armed(record).picking(id.0),
            ] {
                let ship = shooter.id;
                shooter.rounds = LauncherRounds::from([(id, 5), (WeaponId(138), 5)]);
                let mut combat = Combat::default();
                let mut ships = [shooter];
                tick(&mut combat, &mut ships, &NovaDisable);
                assert_eq!(combat.shots(), [], "{ship:?} {record:?}: nothing flies");
                let heard = sounds(&combat.take_events());
                assert_eq!(heard.len(), 1, "{ship:?}: still heard");
                let rounds = &ships[0].rounds;
                assert_eq!(
                    rounds.slot(id) + rounds.slot(WeaponId(138)),
                    9,
                    "{ship:?}: spent"
                );
                // The player counts its reload down after its trigger
                // (@0x6ae25), an NPC before its pick (@0x33896).
                let reload = if ship == ShipRef::Player { 9.0 } else { 10.0 };
                assert_eq!(ships[0].armament.mounts()[0].reload, reload, "{ship:?}");
            }
        }
        let mut combat = Combat::default();
        let drawn = WeaponRecord {
            graphic: 255,
            ..blaster()
        };
        tick(&mut combat, &mut [player_with(drawn)], &NovaDisable);
        assert_eq!(combat.shots().len(), 1, "Graphic 255 flies");
        let mut combat = Combat::default();
        let last = WeaponRecord {
            id: WeaponId(383),
            ..blaster()
        };
        tick(&mut combat, &mut [player_with(last)], &NovaDisable);
        assert_eq!(combat.shots().len(), 1, "index 255 flies");
    }

    #[test]
    fn a_minus_999_shot_of_a_graphic_above_255_spares_its_firer() {
        let mut ships = [player_with(self_destructing(WeaponRecord {
            graphic: 256,
            ..blaster()
        }))];
        tick(&mut Combat::default(), &mut ships, &NovaDisable);
        assert_eq!(ships[0].reserves.armor.now, 30.0);
    }

    #[test]
    fn point_defence_of_a_graphic_above_255_fires_and_spends_nothing() {
        let launching = WeaponRecord {
            ammo_type: 10,
            graphic: 300,
            ..quad()
        };
        let mut combat = Combat::default();
        let mut ships = missile_attack(launching);
        ships[0].rounds = LauncherRounds::from([(WeaponId(133), 5)]);
        tick(&mut combat, &mut ships, &NovaDisable);
        let pd = sounds(&combat.take_events())
            .into_iter()
            .filter(|&(weapon, _)| weapon == WeaponId(133))
            .count();
        assert_eq!(pd, 0, "not heard");
        assert_eq!(ships[0].rounds.slot(WeaponId(133)), 5, "not spent");
        let mount = ships[0].armament.mounts()[0];
        assert_eq!((mount.burst, mount.reload), (0, 0.0));
    }

    #[test]
    fn a_release_whose_sub_munition_has_a_graphic_above_255_is_heard_and_releases_nothing() {
        let (shell, arsenal) = cluster_of(WeaponRecord {
            count: 50,
            speed: 500,
            graphic: 256,
            ..weapon(148)
        });
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell)];
        let mut events = Vec::new();
        for _ in 0..5 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            ships[0].trigger = QUIET;
            events.extend(combat.take_events());
        }
        assert_eq!(sub_shots(&combat), [] as [&Shot; 0], "none released");
        let heard = sounds(&events)
            .into_iter()
            .filter(|&(weapon, _)| weapon == WeaponId(148))
            .count();
        assert_eq!(heard, 1, "the release is heard");
    }

    #[test]
    fn under_the_caps_a_point_defence_beam_runs_its_tail_though_no_beam_is_kept() {
        let beam = WeaponRecord {
            guidance: 10,
            reload: 30,
            count: 5,
            beam_length: 300,
            ..weapon(150)
        };
        let mut combat = Combat {
            beams: held_beams(MAX_BEAMS),
            ..Combat::default()
        };
        combat.set_combat_caps(RuleSource::Engine);
        let mut ships = missile_attack(beam);
        missile_in_flight(&mut combat, &mut ships);
        tick(&mut combat, &mut ships, &NovaDisable);
        assert!(
            combat.beams().iter().all(|beam| beam.firer == C),
            "none kept"
        );
        let heard = sounds(&combat.take_events())
            .into_iter()
            .filter(|&(weapon, _)| weapon == WeaponId(150))
            .count();
        assert_eq!(heard, 1, "heard");
        assert!(ships[0].armament.mounts()[0].reload > 0.0, "reloading");
    }

    #[test]
    fn under_the_caps_a_shot_spent_last_tick_holds_its_slot_while_the_player_fires() {
        let ending = WeaponSpec::new(&WeaponRecord {
            count: 1,
            speed: 0,
            ..weapon(141)
        });
        let far = ShipState {
            position: Vec2::new(-50_000.0, 0.0),
            ..ShipState::default()
        };
        for capped in [false, true] {
            let mut combat = crowded(MAX_SHOTS - 1, capped);
            combat.put_in_flight(Shot::launch(ending, C, C, &far, 0.0));
            // Its move spends it this tick, nothing firing: it is still in
            // flight, drawn at its last point.
            let mut ships = [player_with(blaster()), Ship::at(1, 0.0, 300.0)];
            ships[0].trigger = QUIET;
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(combat.shots().len(), MAX_SHOTS, "spent");
            // The player finds its slot still held; the NPC after the hit
            // pass has ended it finds it free.
            ships[0].trigger = PRIMARY_HELD;
            ships[1] = Ship::at(1, 0.0, 300.0).armed(blaster()).picking(128);
            tick(&mut combat, &mut ships, &NovaDisable);
            let by = |ship| {
                combat
                    .shots()
                    .iter()
                    .filter(|shot| shot.firer == ship)
                    .count()
            };
            assert_eq!(by(ShipRef::Player), usize::from(!capped), "{capped}");
            assert_eq!(by(A), 1, "{capped}");
        }
    }

    #[test]
    fn under_the_caps_a_spent_shot_holds_its_slot_from_every_ships_point_defence() {
        let ending = WeaponSpec::new(&WeaponRecord {
            count: 1,
            speed: 0,
            ..weapon(141)
        });
        let far = ShipState {
            position: Vec2::new(-50_000.0, 0.0),
            ..ShipState::default()
        };
        for defender in [ShipRef::Player, A] {
            for capped in [false, true] {
                let mut combat = crowded(MAX_SHOTS - 2, capped);
                let mut ships = missile_attack(quad());
                ships[0].id = defender;
                ships[0].fleet = defender;
                ships[1].target = Some(defender);
                let (defending, attacking) = (ships[0].trigger, ships[1].trigger);
                ships[0].trigger = Trigger {
                    point_defence: false,
                    ..QUIET
                };
                // The missile takes the 127th slot a tick ahead: the
                // player's point defence fires before the NPCs do.
                tick(&mut combat, &mut ships, &NovaDisable);
                ships[1].trigger = Trigger {
                    primary: false,
                    ..attacking
                };
                // A shot of one tick takes the 128th, and its move spends
                // it, nothing firing.
                combat.put_in_flight(Shot::launch(ending, C, C, &far, 0.0));
                tick(&mut combat, &mut ships, &NovaDisable);
                assert_eq!(combat.shots().len(), MAX_SHOTS, "spent");
                combat.take_events();
                // Every ship's point defence finds the spent shot still
                // holding the 128th slot: `_HandleShot` ends it (@0x36195)
                // after `_HandlePlayer` and `_AIDispatch` (@0x4429d).
                ships[0].trigger = defending;
                tick(&mut combat, &mut ships, &NovaDisable);
                let missiles = combat.shots().iter().filter(|shot| shot.firer == B);
                assert_eq!(missiles.count(), 1, "{defender:?} {capped}: incoming");
                let pd = sounds(&combat.take_events())
                    .into_iter()
                    .filter(|&(weapon, _)| weapon == WeaponId(133))
                    .count();
                assert_eq!(pd, usize::from(!capped), "{defender:?} {capped}");
            }
        }
    }

    /// A blaster reloading every 10 ticks, so a firing shows on its mount.
    fn slow_blaster() -> WeaponRecord {
        WeaponRecord {
            reload: 10,
            ..blaster()
        }
    }

    /// [`missile_attack`] of `quad()`, its defender `defender` armed with
    /// a [`slow_blaster`] as well, after the ship loop in which B's missile
    /// takes the 127th slot among [`crowded`]'s `MAX_SHOTS - 2`, the
    /// defender quiet and its point defence off; B fires no more.
    fn last_slot_left(defender: ShipRef, capped: bool) -> (Combat, [Ship; 2]) {
        let mut combat = crowded(MAX_SHOTS - 2, capped);
        let mut ships = missile_attack(quad());
        ships[0].id = defender;
        ships[0].fleet = defender;
        ships[1].target = Some(defender);
        ships[0].armament = Armament::new([
            (WeaponSpec::new(&quad()), 1),
            (WeaponSpec::new(&slow_blaster()), 1),
        ]);
        ships[0].trigger = Trigger {
            point_defence: false,
            ..QUIET
        };
        phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.shots().len(), MAX_SHOTS - 1, "the missile's 127th");
        ships[1].trigger = QUIET;
        combat.take_events();
        (combat, ships)
    }

    /// What `ship` has in flight: its point-defence shots and its others.
    fn in_flight_by(combat: &Combat, ship: ShipRef) -> (usize, usize) {
        let by = combat.shots().iter().filter(|shot| shot.firer == ship);
        let (pd, other): (Vec<&Shot>, Vec<&Shot>) =
            by.partition(|shot| shot.weapon.id == WeaponId(133));
        (pd.len(), other.len())
    }

    #[test]
    fn under_the_caps_the_players_point_defence_takes_the_last_slot_before_its_trigger() {
        // `_HandlePlayer` runs `_HandleShipPointDefense` @0x68c74 before
        // `_FirePlayerWeapon` @0x6ab55: point defence takes the 128th.
        for capped in [false, true] {
            let (mut combat, mut ships) = last_slot_left(ShipRef::Player, capped);
            ships[0].trigger = PRIMARY_HELD;
            tick(&mut combat, &mut ships, &NovaDisable);
            let (pd, blaster) = in_flight_by(&combat, ShipRef::Player);
            assert_eq!(pd, 1, "{capped}: point defence flies");
            assert_eq!(blaster, usize::from(!capped), "{capped}: the blaster");
            let heard = sounds(&combat.take_events());
            assert!(
                heard.iter().any(|&(weapon, _)| weapon == WeaponId(128)),
                "{capped}: the blaster is heard: {heard:?}"
            );
            assert_eq!(
                ships[0].armament.mounts()[1].reload,
                9.0,
                "{capped}: and spent"
            );
        }
    }

    #[test]
    fn under_the_caps_an_npcs_point_defence_takes_the_last_slot_before_an_npcs_pick() {
        // Every NPC's point defence runs in `_AIDispatch`'s loop
        // (@0x4429d, `_AIFireTurret` @0x80b0d), before `_HandleShip`'s
        // picks (`_FireAIShipWeapon` @0x34ffd, loop @0x44483).
        for capped in [false, true] {
            let (mut combat, mut ships) = last_slot_left(A, capped);
            ships[0].trigger = Trigger {
                only: Some(WeaponId(128)),
                ..QUIET
            };
            tick(&mut combat, &mut ships, &NovaDisable);
            let (pd, blaster) = in_flight_by(&combat, A);
            assert_eq!(pd, 1, "{capped}: point defence flies");
            assert_eq!(blaster, usize::from(!capped), "{capped}: the blaster");
            let heard = sounds(&combat.take_events());
            assert!(
                heard.iter().any(|&(weapon, _)| weapon == WeaponId(128)),
                "{capped}: the blaster is heard: {heard:?}"
            );
            // Counted down before the pick (@0x33896), not after.
            assert_eq!(
                ships[0].armament.mounts()[1].reload,
                10.0,
                "{capped}: and spent"
            );
        }
    }

    #[test]
    fn under_the_caps_a_beam_ended_last_tick_holds_its_slot_through_the_next_firing() {
        let short = WeaponSpec::new(&WeaponRecord {
            count: 0,
            ..laser()
        });
        for capped in [false, true] {
            let mut combat = Combat {
                beams: held_beams(MAX_BEAMS - 1),
                ..Combat::default()
            };
            if capped {
                combat.set_combat_caps(RuleSource::Engine);
            }
            combat.beams.push(Beam::launch(
                short,
                C,
                C,
                &ShipState::default(),
                Aiming::Ahead,
            ));
            let mut ships = [Ship::at(1, 0.0, 0.0).armed(laser()).picking(146)];
            ships[0].trigger = QUIET;
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(combat.beams().len(), MAX_BEAMS - 1, "ended");
            ships[0].trigger = Trigger {
                only: Some(WeaponId(146)),
                ..QUIET
            };
            tick(&mut combat, &mut ships, &NovaDisable);
            let fired = combat.beams().iter().filter(|beam| beam.firer == A).count();
            assert_eq!(fired, usize::from(!capped), "{capped}");
            // A tick on, reloaded, the slot is free.
            for _ in 0..10 {
                ships[0].armament.reload();
            }
            tick(&mut combat, &mut ships, &NovaDisable);
            let fired = combat.beams().iter().filter(|beam| beam.firer == A).count();
            assert_eq!(fired, 2 - usize::from(capped), "{capped}: a tick later");
        }
    }

    #[test]
    fn clearing_the_fight_frees_the_slots_ended_shots_and_beams_hold() {
        let mut combat = crowded(1, true);
        for slot in 1..6 {
            combat.slots.hold(slot);
        }
        combat.ended_beams = 5;
        combat.player_stood = Some(Vec2::new(1.0, 2.0));
        let ended = combat.shots[0];
        combat.hide(ended);
        combat.clear();
        assert_eq!((combat.slots.held().count(), combat.ended_beams), (0, 0));
        assert_eq!(combat.shots(), []);
        assert_eq!(combat.hidden, [], "the shots point defence ended");
        assert_eq!(combat.player_stood, None, "where the player stood");
    }

    // The hit pass: each move tested in the tick after it is made, as
    // the original's next frame tests it.

    /// The events among `events` of a hit landing: a ship's hit, an
    /// explosion or a burst of particles.
    fn hit_events(events: &[CombatEvent]) -> Vec<CombatEvent> {
        events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    CombatEvent::Hit { .. }
                        | CombatEvent::Exploded { .. }
                        | CombatEvent::Particles(_)
                )
            })
            .copied()
            .collect()
    }

    #[test]
    fn a_shot_that_meets_a_ship_is_in_flight_at_its_end_point_for_a_tick_before_it_hits() {
        let shell = spraying(WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            ..blaster()
        });
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell), Ship::at(2, 100.0, 0.0)];
        let mut events = Vec::new();
        // Its sixth move, from 75 to 90 on the seventh tick, comes within
        // B's 16 pixels.
        for _ in 0..7 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = QUIET;
            events.extend(combat.take_events());
        }
        assert_eq!(hit_events(&events), [], "no hit yet");
        assert_eq!(combat.shots().len(), 1, "drawn at its end point");
        assert!(close_to(combat.shots()[0].position, Vec2::new(90.0, 0.0)));
        assert_eq!(left(&ships[1]), (10.0, 30.0), "whole");
        assert_eq!(combat.take_strikes(), []);
        tick(&mut combat, &mut ships, &NovaDisable);
        let events = combat.take_events();
        let landed = hit_events(&events);
        assert!(
            landed
                .iter()
                .any(|event| matches!(event, CombatEvent::Hit { ship: B, .. })),
            "{events:?}"
        );
        assert!(
            landed
                .iter()
                .any(|event| matches!(event, CombatEvent::Exploded { .. }))
        );
        assert!(
            landed
                .iter()
                .any(|event| matches!(event, CombatEvent::Particles(_)))
        );
        assert_eq!(combat.shots(), [], "gone as it hits");
        assert_ne!(left(&ships[1]), (10.0, 30.0), "struck");
    }

    #[test]
    fn a_detonating_shot_ends_the_tick_after_the_move_that_spends_it() {
        let (shell, cluster_arsenal) = cluster(0);
        let flak = WeaponRecord {
            blast_radius: 40,
            ..short_shot(2, 0x8000)
        };
        let cases = [
            ("detonates", flak, Arsenal::default()),
            ("fizzles", short_shot(1, 0), Arsenal::default()),
            ("releases", shell, cluster_arsenal),
        ];
        for (what, record, arsenal) in cases {
            let mut combat = Combat::default();
            let mut ships = [Ship::at(1, 0.0, 0.0).armed(record), Ship::at(3, 30.0, 35.0)];
            let mut events = Vec::new();
            // Fired on the first tick, it moves on the next two.
            for _ in 0..4 {
                tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
                ships[0].trigger = QUIET;
                events.extend(combat.take_events());
            }
            assert_eq!(combat.shots().len(), 1, "{what}: spent, still in flight");
            let spent = combat.shots()[0];
            assert!(spent.expired(), "{what}");
            assert!(close_to(spent.position, Vec2::new(30.0, 0.0)), "{what}");
            let ended = |events: &[CombatEvent]| {
                events.iter().any(|event| {
                    matches!(
                        event,
                        CombatEvent::Exploded { .. } | CombatEvent::Fizzled { .. }
                    ) || matches!(event, CombatEvent::Fired { weapon, .. } if *weapon == WeaponId(148))
                })
            };
            assert!(!ended(&events), "{what}: not yet");
            assert_eq!(left(&ships[1]), (10.0, 30.0), "{what}");
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            let events = combat.take_events();
            assert!(ended(&events), "{what}: {events:?}");
            match what {
                "detonates" => assert_ne!(left(&ships[1]), (10.0, 30.0), "blasted"),
                "fizzles" => assert!(events.contains(&CombatEvent::Fizzled { at: spent.position })),
                _ => assert_eq!(sub_shots(&combat).len(), 2),
            }
            assert!(
                combat.shots().iter().all(|shot| shot.id != spent.id),
                "{what}: gone"
            );
        }
    }

    /// A shot of `record` fired by C, out of the fight, whose last move
    /// ended at `at` from 15 to its left.
    fn flown_by_c(record: WeaponRecord, at: Vec2) -> Shot {
        Shot {
            position: at,
            moved_from: Some(at - Vec2::new(15.0, 0.0)),
            age: 1,
            ..Shot::launch(WeaponSpec::new(&record), C, C, &ShipState::default(), 90.0)
        }
    }

    /// The ships struck when, after a tick's close with `ships` where
    /// they stood, they move as `moved` says and the hit pass tests `shot`.
    fn struck_after(ships: &mut [Ship], moved: impl Fn(&mut [Ship]), shot: &Shot) -> Vec<ShipRef> {
        let mut combat = Combat::default();
        phase::close_tick(&mut combat, ships, &NovaDisable);
        moved(ships);
        combat.put_in_flight(*shot);
        phase::test_hits(&mut combat, ships, &NovaDisable);
        combat
            .take_strikes()
            .iter()
            .map(|strike| strike.ship)
            .collect()
    }

    fn the_player_at(x: f32, y: f32) -> Ship {
        Ship {
            id: ShipRef::Player,
            fleet: ShipRef::Player,
            ..Ship::at(0, x, y)
        }
    }

    #[test]
    fn the_hit_pass_meets_an_npc_where_it_is_and_the_players_touch_where_it_stood_last_tick() {
        let touching = flown_by_c(blaster(), Vec2::new(10.0, 0.0));
        let to =
            |x: f32, y: f32| move |ships: &mut [Ship]| ships[0].state.position = Vec2::new(x, y);
        // An NPC is met where it is, as the collision passes read its
        // sprite where its `_HandleShip` left it (@0x3428f-0x342aa).
        assert_eq!(
            struck_after(&mut [Ship::at(2, 0.0, 0.0)], to(500.0, 0.0), &touching),
            []
        );
        assert_eq!(
            struck_after(&mut [Ship::at(2, 500.0, 0.0)], to(0.0, 0.0), &touching),
            [B]
        );
        // The player's touch is where it stood as the shot moved, its
        // sprite at the screen's centre.
        assert_eq!(
            struck_after(&mut [the_player_at(0.0, 0.0)], to(500.0, 0.0), &touching),
            [ShipRef::Player]
        );
        assert_eq!(
            struck_after(&mut [the_player_at(500.0, 0.0)], to(0.0, 0.0), &touching),
            []
        );
        // Its fuse, in world terms, where it is now: 25 off the end
        // point, within trunc(20 + 32 x 0.333) = 30.
        let fused = flown_by_c(
            WeaponRecord {
                prox_radius: 20,
                ..blaster()
            },
            Vec2::new(10.0, 0.0),
        );
        assert_eq!(
            struck_after(&mut [the_player_at(500.0, 0.0)], to(10.0, 25.0), &fused),
            [ShipRef::Player]
        );
        assert_eq!(
            struck_after(&mut [the_player_at(10.0, 25.0)], to(500.0, 0.0), &fused),
            []
        );
        // A player not in the last tick's fight is met where it is.
        let mut combat = Combat::default();
        combat.put_in_flight(touching);
        let mut ships = [the_player_at(0.0, 0.0)];
        phase::test_hits(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.take_strikes().len(), 1, "met where it is");
    }

    #[test]
    fn point_defence_measures_its_sprites_from_where_the_player_stood_last_tick() {
        // As `point_defence_sees_the_sprites_from_where_the_player_is`:
        // seen from 0, the sprites are an edge apart; from 0.6, they meet.
        let meets = |stood: f32, now: f32| {
            let mut combat = Combat::default();
            let mut ships = [the_player_at(stood, 0.0)];
            phase::close_tick(&mut combat, &mut ships, &NovaDisable);
            put(&mut combat, pd_shot(), A, Vec2::new(0.5, 100.0), Vec2::ZERO);
            let missile = put(
                &mut combat,
                still_missile(),
                B,
                Vec2::new(10.4, 100.0),
                Vec2::ZERO,
            );
            ships[0].state.position = Vec2::new(now, 0.0);
            phase::test_hits(&mut combat, &mut ships, &NovaDisable);
            flying(&combat, missile).is_none()
        };
        assert!(meets(0.6, 0.0), "seen from where it stood");
        assert!(!meets(0.0, 0.6));
    }

    #[test]
    fn an_npc_downed_by_a_shot_fires_nothing_that_tick_and_the_player_fires_first() {
        let mut player = player_with(blaster()).targeting(B);
        player.reserves = Reserves::full(0.0, 1.0, 100.0);
        player.hull.death_delay = 10;
        let mut npc = Ship::at(2, 500.0, 0.0)
            .armed(blaster())
            .targeting(ShipRef::Player);
        npc.reserves = Reserves::full(0.0, 1.0, 100.0);
        npc.hull.death_delay = 10;
        let mut combat = Combat::default();
        let mut ships = [player, npc];
        ships[0].trigger = QUIET;
        ships[1].trigger = QUIET;
        tick(&mut combat, &mut ships, &NovaDisable);
        // Both armed; a shot's last move ended on each.
        ships[0].trigger = PRIMARY_HELD;
        ships[1] = Ship {
            reserves: ships[1].reserves,
            ..Ship::at(2, 500.0, 0.0)
                .armed(blaster())
                .targeting(ShipRef::Player)
        };
        ships[1].hull.death_delay = 10;
        combat
            .shots
            .push(flown_by_c(blaster(), Vec2::new(5.0, 0.0)));
        combat
            .shots
            .push(flown_by_c(blaster(), Vec2::new(505.0, 0.0)));
        tick(&mut combat, &mut ships, &NovaDisable);
        let events = combat.take_events();
        assert_eq!(fired(&events, ShipRef::Player), 1, "the player fired first");
        assert_eq!(fired(&events, B), 0, "B, downed, fires nothing");
        assert!(matches!(ships[1].condition, Condition::Dying { .. }));
        assert!(matches!(ships[0].condition, Condition::Dying { .. }));
        let first_fire = events
            .iter()
            .position(|event| matches!(event, CombatEvent::Fired { .. }));
        let first_hit = events
            .iter()
            .position(|event| matches!(event, CombatEvent::Hit { .. }));
        assert!(first_fire < first_hit, "{events:?}");
    }

    #[test]
    fn a_breakup_the_hit_pass_begins_is_counted_down_from_the_next_tick() {
        let mut combat = Combat::default();
        let mut hulk = Ship::at(2, 0.0, 0.0);
        hulk.reserves = Reserves::full(0.0, 1.0, 100.0);
        hulk.hull.death_delay = 5;
        let mut ships = [hulk];
        combat
            .shots
            .push(flown_by_c(blaster(), Vec2::new(5.0, 0.0)));
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            ships[0].condition,
            Condition::Dying {
                ticks_left: 5,
                starting: true
            },
            "no tick off on its first"
        );
        let mut destroyed_on = None;
        for tick_number in 1..=5 {
            tick(&mut combat, &mut ships, &NovaDisable);
            if ships[0].condition == Condition::Destroyed && destroyed_on.is_none() {
                destroyed_on = Some(tick_number);
            }
        }
        assert_eq!(destroyed_on, Some(3), "DeathDelay 5 less 2 ticks after");
    }

    #[test]
    fn the_players_point_defence_meets_an_npcs_missile_the_tick_after_it_is_fired() {
        // The tick each pairing's point defence first fires, the missile
        // fired on the first.
        let first_defence = |defender: ShipRef, attacker: ShipRef| {
            let mut ships = missile_attack(quad());
            ships[0].id = defender;
            ships[0].fleet = defender;
            ships[1].id = attacker;
            ships[1].fleet = attacker;
            ships[1].target = Some(defender);
            let mut combat = Combat::default();
            for tick_number in 1..=3 {
                tick(&mut combat, &mut ships, &NovaDisable);
                ships[1].trigger = QUIET;
                let pd = sounds(&combat.take_events())
                    .into_iter()
                    .any(|(weapon, _)| weapon == WeaponId(133));
                if pd {
                    return Some(tick_number);
                }
            }
            None
        };
        assert_eq!(
            first_defence(ShipRef::Player, B),
            Some(2),
            "the player's point defence runs before the NPCs fire"
        );
        assert_eq!(
            first_defence(A, B),
            Some(2),
            "an NPC's before the NPCs fire (`_AIDispatch` @0x4429d, then `_HandleShip` @0x44483)"
        );
        assert_eq!(
            first_defence(A, ShipRef::Player),
            Some(1),
            "an NPC's after the player fires"
        );
    }

    #[test]
    fn an_npcs_point_defence_meets_a_missile_fired_this_tick_only_on_the_next() {
        // `_AIDispatch` @0x4429d runs every NPC's point defence before
        // `_HandleShip` @0x44483 fires any NPC's pick: B's missile first
        // exists for A's `_HandleShipPointDefense` in the next frame.
        let mut combat = Combat::default();
        let mut ships = missile_attack(quad());
        let defended = |combat: &mut Combat| {
            let pd_heard = sounds(&combat.take_events())
                .into_iter()
                .any(|(weapon, _)| weapon == WeaponId(133));
            (pd_heard, in_flight_by(combat, A).0)
        };
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(in_flight_by(&combat, B).1, 1, "the missile is in flight");
        assert_eq!(defended(&mut combat), (false, 0), "tick 1: no defence");
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(defended(&mut combat), (true, 1), "tick 2: defended");
    }

    #[test]
    fn the_players_point_defence_launches_from_its_velocity_before_its_recoil() {
        // Point defence (@0x68c74) reads the ship before
        // `_FirePlayerWeapon`'s recoil (@0x62ea2-0x62f35) pushes it, and a
        // point-defence shot carries its firer's velocity (`_SpawnShot`).
        let mut ships = missile_attack(quad());
        ships[0].id = ShipRef::Player;
        ships[0].fleet = ShipRef::Player;
        ships[0].hull.mass = 50.0;
        ships[1].target = Some(ShipRef::Player);
        let kicking = WeaponRecord {
            recoil: 100,
            ..slow_blaster()
        };
        ships[0].armament = Armament::new([
            (WeaponSpec::new(&quad()), 1),
            (WeaponSpec::new(&kicking), 1),
        ]);
        ships[0].trigger = Trigger {
            point_defence: false,
            ..QUIET
        };
        let mut combat = Combat::default();
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[1].trigger = QUIET;
        ships[0].trigger = PRIMARY_HELD;
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.take_shoves().len(), 1, "the recoil");
        let pd: Vec<&Shot> = combat
            .shots()
            .iter()
            .filter(|shot| shot.weapon.id == WeaponId(133))
            .collect();
        assert_eq!(pd.len(), 1, "{:?}", combat.shots());
        let at_rest = projectile::launch_velocity(
            &pd[0].weapon,
            &ShipState::default(),
            pd[0].heading,
            BombLaunch::Dropped,
        );
        assert!(
            (pd[0].velocity - at_rest).length() < 1e-4,
            "launched at rest: {:?} against {at_rest:?}",
            pd[0].velocity
        );
    }

    #[test]
    fn a_point_defence_beam_downs_a_missile_before_its_move_is_tested() {
        // A homing missile 40 above A, closing 30 a tick: its move enters
        // A's 16 pixels, and A's point-defence beam holds on it that tick.
        let struck = |point_defence: bool| {
            let fast = WeaponRecord {
                speed: 3000,
                durability: 0,
                ..missile()
            };
            let mut defender = Ship::at(1, 0.0, 0.0).armed(pd_beam());
            defender.trigger = Trigger {
                point_defence,
                ..QUIET
            };
            let mut ships = [defender];
            let mut combat = Combat::default();
            combat.put_in_flight(Shot {
                target: Some(A),
                aimed_at: Some(A),
                ..Shot::launch(
                    WeaponSpec::new(&fast),
                    C,
                    C,
                    &ShipState {
                        position: Vec2::new(0.0, -40.0),
                        ..ShipState::default()
                    },
                    180.0,
                )
            });
            let mut events = Vec::new();
            for _ in 0..2 {
                tick(&mut combat, &mut ships, &NovaDisable);
                events.extend(combat.take_events());
            }
            let downed = events
                .iter()
                .any(|event| matches!(event, CombatEvent::ShotDown { .. }));
            (downed, left(&ships[0]) != (10.0, 30.0))
        };
        assert_eq!(struck(true), (true, false), "downed before its contact");
        assert_eq!(struck(false), (false, true), "without, it hits");
    }

    #[test]
    fn a_sub_munition_released_by_a_proximity_hit_flies_in_the_tick_it_is_released() {
        let (shell, arsenal) = cluster(0x0020);
        let fused = WeaponRecord {
            prox_radius: 20,
            ..shell
        };
        let mut combat = Combat::default();
        combat.put_in_flight(Shot {
            target: Some(B),
            aimed_at: Some(B),
            ..flown_by_c(fused, Vec2::new(20.0, 0.0))
        });
        let mut ships = [Ship::at(2, 40.0, 20.0)];
        tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        let subs = sub_shots(&combat);
        assert_eq!(subs.len(), 2, "released in the hit pass");
        for sub in subs {
            assert_eq!(sub.age, 1, "and flown once");
            assert_eq!(sub.moved_from, Some(Vec2::new(20.0, 0.0)));
        }
    }

    #[test]
    fn a_shot_of_no_life_is_dropped_at_its_first_step_unseen() {
        // A shell of no life, detonating and releasing at its end.
        let (shell, _) = cluster(0);
        let lifeless = WeaponRecord {
            count: 0,
            explod_type: 2,
            blast_radius: 40,
            flags: 0x8000,
            ..shell
        };
        let sub = WeaponRecord {
            count: 50,
            speed: 500,
            ..weapon(148)
        };
        let arsenal = Arsenal::new(&[lifeless, sub], Vec::new());
        let mut combat = Combat::default();
        let mut ships = [player_with(lifeless), Ship::at(3, 0.0, 20.0)];
        tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        ships[0].trigger = QUIET;
        let mut events = combat.take_events();
        assert_eq!(fired(&events, ShipRef::Player), 1);
        assert_eq!(combat.shots(), [], "dropped unmoved");
        tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        events.extend(combat.take_events());
        assert!(
            !events.iter().any(|event| matches!(
                event,
                CombatEvent::Exploded { .. } | CombatEvent::Fizzled { .. }
            )),
            "{events:?}"
        );
        assert_eq!(fired(&events, ShipRef::Player), 1, "no release");
        assert_eq!(left(&ships[1]), (10.0, 30.0), "no blast");
        // It holds no slot after: the original frees it in the
        // `_HandleShot` that first visits it.
        for capped in [false, true] {
            let mut combat = crowded(MAX_SHOTS - 1, capped);
            let mut ships = [player_with(lifeless)];
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            ships[0].trigger = PRIMARY_HELD;
            ships[0].armament = Armament::new([(WeaponSpec::new(&blaster()), 1)]);
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            let fired = combat
                .shots()
                .iter()
                .filter(|shot| shot.firer == ShipRef::Player)
                .count();
            assert_eq!(fired, 1, "{capped}");
        }
    }

    #[test]
    fn the_damage_aimed_at_a_ship_counts_no_spent_shot() {
        let aimed_at_b = |age: u32| Shot {
            aimed_at: Some(B),
            age,
            ..aimed(blaster())
        };
        let combat = Combat {
            shots: vec![aimed_at_b(12), aimed_at_b(13)],
            ..Combat::default()
        };
        assert_eq!(
            combat.targeted_damage(),
            BTreeMap::from([(B, 7)]),
            "the one with life left: (4 + 10) / 2"
        );
    }

    #[test]
    fn under_the_caps_a_missile_a_point_defence_beam_downed_holds_its_slot_from_the_players_next_fire()
     {
        let fragile = drawn(
            WeaponRecord {
                speed: 500,
                durability: 0,
                ..missile()
            },
            18,
        );
        for capped in [false, true] {
            // 127 others, and B's missile, which A's beam downs the tick
            // after it is fired.
            let mut combat = crowded(MAX_SHOTS - 1, capped);
            let mut defender = Ship::at(1, 0.0, 0.0).armed(pd_beam());
            defender.trigger = Trigger {
                point_defence: false,
                ..QUIET
            };
            let attacker = Ship::at(2, 0.0, -200.0)
                .armed(fragile)
                .facing(180.0)
                .targeting(A);
            let mut player = the_player_at(0.0, 300.0).armed(blaster());
            player.trigger = QUIET;
            let mut ships = [player, defender, attacker];
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[1].trigger = QUIET;
            ships[2].trigger = QUIET;
            tick(&mut combat, &mut ships, &NovaDisable);
            let downed = combat
                .take_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::ShotDown { .. }));
            assert!(downed, "{capped}");
            assert_eq!(combat.shots().len(), MAX_SHOTS - 1, "{capped}: gone");
            // The player finds its slot still held, as the original frees
            // it only at its next `_HandleShot`.
            ships[0].trigger = PRIMARY_HELD;
            ships[2].trigger = QUIET;
            tick(&mut combat, &mut ships, &NovaDisable);
            let fired = combat
                .shots()
                .iter()
                .filter(|shot| shot.firer == ShipRef::Player)
                .count();
            assert_eq!(fired, usize::from(!capped), "{capped}");
        }
    }

    #[test]
    fn the_proximity_fuse_reads_its_rule() {
        // B beside the move's middle, within the swept reach of 16 + 20,
        // 40 from its end point.
        let fused = WeaponRecord {
            prox_radius: 20,
            speed: 8000,
            ..blaster()
        };
        for (source, hit) in [(RuleSource::Engine, false), (RuleSource::Bible, true)] {
            let mut combat = Combat::default();
            combat.set_proximity_fuse(source);
            combat.put_in_flight(Shot {
                moved_from: Some(Vec2::ZERO),
                ..flown_by_c(fused, Vec2::new(80.0, 0.0))
            });
            let mut ships = [Ship::at(2, 40.0, 30.0)];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(!combat.take_strikes().is_empty(), hit, "{source:?}");
        }
    }
}
