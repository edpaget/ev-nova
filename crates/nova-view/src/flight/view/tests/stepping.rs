//! The view's frame of steps over a scripted flight ([`FakeFlight`]):
//! the input it gives each frame, and what it steps after each pass the
//! flight reports (`nova_sim::Session::step` pins the passes themselves):
//! the effects, the shield flashes, the weapon glows, the shot looks, the
//! stars, the comm and quotes, the ships kept for the drawing and the
//! rescue.

use nova_sim::catalog::Explosion;
use nova_sim::combat::projectile::ShotId;
use nova_sim::flight_frame::ShipPlace;
use nova_sim::flight_frame::{
    FlightEvent, FrameShip, FrameShot, PassReport, RaisedLine, StepInput, StepReport, StepRun,
};
use nova_sim::{CommNote, FireSound, Gauge, PersonQuote};

use super::faked_travel::{faked_lasts, shown_to_flight};

use crate::flight::stars::Star;
use crate::system::camera::Anchor;

use super::*;

/// The NPC the reports carry, beside the player.
const NPC: ShipRef = ShipRef::Npc(NpcId(0));

/// Where a frame's steps draw the ship: the original's play area centre.
fn anchor() -> Anchor {
    Anchor::in_flight(RuleSource::Engine)
}

/// A view over a scripted flight in Sol, read from `catalog` by `rules`,
/// with what a flight reads as it starts: the weapons' looks, the
/// player's ship 128's sheet and the NPCs' 129's.
fn stepping_by(catalog: FakeCatalog, rules: &Rulebook) -> Faked {
    let looks = Looks::read(&catalog, catalog.weapons.iter().map(|weapon| weapon.id));
    let sheet = catalog.ship_sheet(ShipId(128));
    let npc = catalog.ship_sheet(ShipId(129));
    let mut fake = FakeFlight::new();
    fake.set_frame(in_sol());
    let mut view = scripted_over(catalog, fake, rules);
    view.looks = looks;
    view.sheet = sheet;
    view.npc_sheets.insert(ShipId(129), npc);
    view
}

/// [`stepping_by`] Nova's rules.
fn stepping(catalog: FakeCatalog) -> Faked {
    stepping_by(catalog, &Rulebook::default())
}

/// Runs a frame of `view` whose flight reports `report`, asked for as
/// many steps as it holds: whether the rescue ended it.
fn frame(view: &mut Faked, report: StepReport) -> bool {
    let steps = u32::try_from(report.steps.len()).expect("a frame's steps");
    view.sim.queue_report(report);
    view.step_frame(steps, anchor())
}

/// A frame of one step of `passes`.
fn one_step(passes: Vec<PassReport>) -> StepReport {
    StepReport {
        steps: vec![step(passes)],
        stopped: false,
    }
}

/// A step of `passes`, the player before it at the origin.
fn step(passes: Vec<PassReport>) -> StepRun {
    StepRun {
        before: vec![(ShipRef::Player, ShipPlace::default())],
        passes,
        ..StepRun::default()
    }
}

/// The player's ship 128 at `at`, intact, its 30 of shield full.
fn the_player(at: Vec2) -> FrameShip {
    FrameShip {
        ship: ShipRef::Player,
        class: ShipId(128),
        position: at,
        condition: Condition::Intact,
        breakup: None,
        shield: Gauge::full(30.0),
        class_shield: Some(30.0),
        ..FrameShip::default()
    }
}

/// [`NPC`], of ship 129, at `at`, intact, 29 of its 30 shield left.
fn the_npc(at: Vec2) -> FrameShip {
    FrameShip {
        ship: NPC,
        class: ShipId(129),
        shield: Gauge {
            now: 29.0,
            max: 30.0,
        },
        ..the_player(at)
    }
}

/// A pass of the player at the origin and [`NPC`] 100 above it, raising
/// `combat`.
fn fight(combat: Vec<CombatEvent>) -> PassReport {
    PassReport {
        ships: vec![the_player(Vec2::ZERO), the_npc(Vec2::new(0.0, -100.0))],
        combat,
        ..PassReport::default()
    }
}

/// A hit on `ship`, shown `late` or not.
fn hit(ship: ShipRef, late: bool) -> CombatEvent {
    CombatEvent::Hit {
        ship,
        weapon: Some(BLASTER),
        at: Vec2::new(0.0, -100.0),
        passes_shields: false,
        late,
    }
}

/// The combat sounds `view` gives.
fn combat_heard(view: &mut Faked) -> Vec<CombatSound> {
    view.drained_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Combat(sound) => Some(sound),
            _ => None,
        })
        .collect()
}

/// `view`'s shield flash on `ship` as the frame showed it.
fn flash(view: &Faked, ship: ShipRef) -> Option<f32> {
    view.flashes
        .get(&ship)
        .map(crate::flight::flash::ShieldFlash::shown)
        .filter(|&shown| shown > 0.0)
}

// The input.

#[test]
fn each_frame_hands_the_flight_the_keys_held_steps_or_none() {
    let mut view = stepping(catalog());
    for k in [
        Key::Up,
        FIRE_KEY,
        SECONDARY_KEY,
        CLOAK_KEY,
        DESELECT_KEY,
        LAND_KEY,
    ] {
        view.held.insert(k);
    }
    view.double_time = true;
    view.step_frame(0, anchor());
    let input = view.sim.inputs()[0];
    assert_eq!(
        input,
        StepInput {
            controls: Controls {
                thrust: true,
                ..Controls::default()
            },
            play_area: super::super::play_area(VIEW_SIZE, anchor()),
            window_width: 1024,
            fire: (true, true),
            eject: false,
            self_destruct: false,
            cloak: true,
            weapon_safety: true,
            double_time_key: true,
            hyperspace_effects: true,
            land_key: true,
        }
    );
    assert_eq!(
        (view.sim.steps(), view.sim.steps_asked()),
        (1, 0),
        "no step"
    );
}

#[test]
fn command_and_minus_hold_the_self_destruct_and_command_and_x_the_eject() {
    // The chords, `_QuickKeyCheck` (@0x705b7): Command with `-`, and
    // Command with X.
    assert_eq!(SELF_DESTRUCT_KEY, Key::Char('-'));
    let held = |keys: &[Key]| {
        let mut view = stepping(catalog());
        view.held.extend(keys.iter().copied());
        view.step_input(anchor())
    };
    let destruct = held(&[Key::Command, SELF_DESTRUCT_KEY]);
    assert!(destruct.self_destruct && !destruct.eject);
    let eject = held(&[Key::Command, EJECT_KEY]);
    assert!(eject.eject && !eject.self_destruct);
    for keys in [&[SELF_DESTRUCT_KEY][..], &[EJECT_KEY], &[Key::Command], &[]] {
        let input = held(keys);
        assert!(!input.self_destruct && !input.eject, "{keys:?}");
    }
}

#[test]
fn a_command_the_flight_refuses_holds_none_of_its_keys() {
    let mut view = stepping(catalog());
    view.held.extend([
        Key::Command,
        SELF_DESTRUCT_KEY,
        EJECT_KEY,
        CLOAK_KEY,
        DESELECT_KEY,
    ]);
    for command in [
        PlayerCommand::SelfDestruct,
        PlayerCommand::Eject,
        PlayerCommand::Cloak,
        PlayerCommand::DeselectWeapon,
    ] {
        view.change_sim(|sim| sim.refuse(command));
    }
    let input = view.step_input(anchor());
    assert!(!input.self_destruct && !input.eject && !input.cloak && !input.weapon_safety);
    // U needs no Command.
    view.change_sim(|sim| sim.take(PlayerCommand::Cloak));
    view.held.remove(&Key::Command);
    assert!(view.step_input(anchor()).cloak);
}

// The effects, flashes and glows.

#[test]
fn each_pass_of_each_step_reaches_the_effects() {
    let mut view = stepping(armed(0, &[BLASTER]));
    let fired = CombatEvent::Fired {
        ship: ShipRef::Player,
        weapon: BLASTER,
        at: Vec2::ZERO,
        sound: Some(FireSound::at(nova_sim::combat::PLAYER_FIRE_PRIORITY)),
    };
    let report = StepReport {
        steps: vec![
            step(vec![fight(vec![fired])]),
            step(vec![fight(vec![fired]), fight(vec![fired])]),
        ],
        stopped: false,
    };
    frame(&mut view, report);
    let heard = CombatSound {
        sound: nova_sim::SoundId(208),
        offset: (0, 0),
        priority: u16::from(nova_sim::combat::PLAYER_FIRE_PRIORITY),
        until_stopped: false,
    };
    assert_eq!(combat_heard(&mut view), [heard; 3]);
}

#[test]
fn a_hit_flashes_the_ships_shield_32_on_its_pass_then_31() {
    let mut view = stepping(armed(1, &[BLASTER]));
    frame(&mut view, one_step(vec![fight(vec![hit(NPC, false)])]));
    assert_eq!(flash(&view, NPC), Some(32.0));
    frame(&mut view, one_step(vec![fight(vec![])]));
    assert_eq!(flash(&view, NPC), Some(31.0));
    assert_eq!(flash(&view, ShipRef::Player), None, "not hit");
}

#[test]
fn a_late_hit_flashes_from_the_pass_after() {
    let mut view = stepping(armed(1, &[BLASTER]));
    frame(&mut view, one_step(vec![fight(vec![hit(NPC, true)])]));
    assert_eq!(flash(&view, NPC), None, "shown the pass after");
    frame(&mut view, one_step(vec![fight(vec![])]));
    assert_eq!(flash(&view, NPC), Some(32.0));
}

#[test]
fn a_ships_flash_is_dropped_once_a_pass_reports_it_gone() {
    let mut view = stepping(armed(1, &[BLASTER]));
    frame(&mut view, one_step(vec![fight(vec![hit(NPC, false)])]));
    assert_eq!(flash(&view, NPC), Some(32.0));
    let gone = PassReport {
        ships: vec![the_player(Vec2::ZERO)],
        ..PassReport::default()
    };
    frame(&mut view, one_step(vec![gone]));
    assert!(!view.flashes.contains_key(&NPC));
}

/// A burst of `count` orange particles of life 30 on the NPC.
fn orange(count: u32) -> CombatEvent {
    CombatEvent::Particles(nova_sim::combat::ParticleBurst {
        at: Vec2::new(0.0, -100.0),
        speed: 1.0,
        variation: 0,
        life: (30, 30),
        color: 0x00FF_8000,
        count,
        spread: 0.0,
    })
}

#[test]
fn a_passs_particles_are_sprayed_in_their_colour() {
    let mut view = stepping(armed(1, &[BLASTER]));
    frame(&mut view, one_step(vec![fight(vec![orange(5)])]));
    let particles = view.effects.particles();
    assert_eq!(particles.len(), 5, "{particles:?}");
    let colour = Color::rgba(0xFF, 0x80, 0x00, 255);
    assert!(particles.iter().all(|piece| piece.color == colour));
}

#[test]
fn under_the_combat_caps_a_pass_keeps_at_most_1024_particles() {
    let sprayed = |rules: &Rulebook| {
        let mut view = stepping_by(armed(1, &[BLASTER]), rules);
        frame(&mut view, one_step(vec![fight(vec![orange(1100)])]));
        view.effects.particles().len()
    };
    assert_eq!(sprayed(&Rulebook::default()), 1100, "uncapped by default");
    let caps = Rulebook::default().with_override(RuleKey::CombatCaps, RuleSource::Engine);
    assert_eq!(sprayed(&caps), 1024);
}

/// NPC 0 destroyed 100 above the player, exploding as `bööm` 128.
fn destroyed() -> CombatEvent {
    CombatEvent::Destroyed {
        ship: NPC,
        ship_type: ShipId(129),
        at: Vec2::new(0.0, -100.0),
        velocity: Vec2::ZERO,
        explosion: Some(Explosion {
            boom: BoomId(128),
            extra: false,
        }),
        size: 0.0,
    }
}

#[test]
fn a_destroyed_ship_explodes_heard_from_the_player_and_scatters_debris() {
    let mut view = stepping(armed(1, &[BLASTER]));
    let pass = PassReport {
        player_position: Vec2::new(0.0, -40.0),
        ..fight(vec![destroyed()])
    };
    frame(&mut view, one_step(vec![pass]));
    let explosions = view.effects.explosions();
    assert_eq!(explosions.len(), 1);
    assert_eq!(explosions[0].boom, BoomId(128));
    assert_eq!(view.effects.particles().len(), 16, "its debris");
    let heard = combat_heard(&mut view);
    assert!(
        heard
            .iter()
            .any(|sound| sound.sound == nova_sim::SoundId(302) && sound.offset == (0, -60)),
        "heard from where the pass left the player: {heard:?}"
    );
}

#[test]
fn under_the_originals_debris_reading_a_destroyed_ship_scatters_none() {
    let rules = Rulebook::default().with_override(RuleKey::ShipDebris, RuleSource::Engine);
    let mut view = stepping_by(armed(1, &[BLASTER]), &rules);
    frame(&mut view, one_step(vec![fight(vec![destroyed()])]));
    assert_eq!(view.effects.explosions().len(), 1, "its Explode2 still");
    assert_eq!(view.effects.particles(), [], "no debris");
}

#[test]
fn the_effects_roll_on_their_own_chance() {
    let (effects, effects_chance) = scripted(&[]);
    let mut view = stepping(armed(1, &[BLASTER]));
    view.effects_chance = effects_chance;
    frame(&mut view, one_step(vec![fight(vec![destroyed()])]));
    assert_eq!(
        effects.borrow().asked,
        [360, 101, 31].repeat(16),
        "the debris, drawn on the effects' own chance"
    );
}

/// Where the first explosion of `ship` breaking up at (30, -100) goes
/// off, in a view whose player's sheet is `width` pixels wide.
fn breaking_up_at(ship: ShipRef, width: u32) -> Point {
    let mut catalog = armed(1, &[BLASTER]);
    catalog.sheet = Ok(ShipSheet {
        frame_width: width,
        ..sheet()
    });
    let mut view = stepping(catalog);
    let at = Vec2::new(30.0, -100.0);
    let breaking = FrameShip {
        ship,
        class: if ship == ShipRef::Player {
            ShipId(128)
        } else {
            ShipId(129)
        },
        condition: Condition::Dying {
            ticks_left: 18,
            starting: false,
        },
        breakup: Some(Explosion {
            boom: BoomId(128),
            extra: false,
        }),
        ..the_npc(at)
    };
    let pass = PassReport {
        ships: vec![breaking],
        ..PassReport::default()
    };
    for _ in 0..20 {
        frame(&mut view, one_step(vec![pass.clone()]));
        if let Some(explosion) = view.effects.explosions().first() {
            return explosion.at;
        }
    }
    panic!("no explosion as it breaks up");
}

#[test]
fn a_ship_breaking_up_goes_off_about_where_the_pass_left_it() {
    // Never firing, the effects' chance draws the last outcome: a
    // quarter of the sprite less one out, each way: the NPC's 40 pixels
    // across, the player's 80.
    assert_eq!(breaking_up_at(NPC, 80), Point::new(39.0, -91.0));
    assert_eq!(breaking_up_at(ShipRef::Player, 80), Point::new(49.0, -81.0));
}

#[test]
fn a_ships_weapon_glow_lights_on_its_pass_and_falls_by_its_classs_decay() {
    let mut view = stepping(armed(1, &[BLASTER]));
    let lit = CombatEvent::WeaponGlow {
        ship: ShipRef::Player,
        late: false,
    };
    frame(&mut view, one_step(vec![fight(vec![lit])]));
    let full = view.glows.drawn(ShipRef::Player);
    assert!(full.is_some(), "lit");
    frame(&mut view, one_step(vec![fight(vec![])]));
    assert!(view.glows.drawn(ShipRef::Player) <= full, "falling");
}

#[test]
fn an_npcs_weapon_glow_falls_by_its_own_classs_decay() {
    let mut view = stepping(armed(1, &[BLASTER]));
    view.sheet = Ok(ShipSheet {
        weap_decay: 0,
        ..sheet()
    });
    view.npc_sheets.insert(
        ShipId(129),
        Ok(ShipSheet {
            weap_decay: 6,
            ..sheet()
        }),
    );
    let lit = CombatEvent::WeaponGlow {
        ship: NPC,
        late: false,
    };
    frame(&mut view, one_step(vec![fight(vec![lit])]));
    let full = view.glows.drawn(NPC);
    for _ in 0..3 {
        frame(&mut view, one_step(vec![fight(vec![])]));
    }
    let fallen = view.glows.drawn(NPC);
    assert!(
        fallen < full,
        "by ship 129's decay, not the player's: {full:?} {fallen:?}"
    );
}

/// A view firing the player's looped beam, of a 3-tick sound, by `rules`.
fn firing_looped(rules: &Rulebook) -> Faked {
    let mut catalog = armed(0, &[UNDER]);
    catalog.looks[4].1 = Ok(WeaponLook {
        flags: weapons::LOOPED_SOUND,
        sound: Some(nova_sim::SoundId(220)),
        sound_ticks: Some(3),
        ..WeaponLook::default()
    });
    let mut view = stepping_by(catalog, rules);
    let fired = CombatEvent::Fired {
        ship: ShipRef::Player,
        weapon: UNDER,
        at: Vec2::ZERO,
        sound: Some(FireSound::at(nova_sim::combat::PLAYER_FIRE_PRIORITY)),
    };
    for _ in 0..4 {
        frame(&mut view, one_step(vec![fight(vec![fired])]));
    }
    view
}

#[test]
fn by_default_a_looped_weapon_is_heard_every_pass_waiting_on_any_voice() {
    let mut view = firing_looped(&Rulebook::default());
    let heard = combat_heard(&mut view);
    assert_eq!(heard.len(), 4, "the audio side decides: {heard:?}");
    assert!(heard.iter().all(|sound| sound.until_stopped), "{heard:?}");
}

#[test]
fn by_the_other_reading_a_looped_weapon_is_heard_again_once_its_sound_has_played_out() {
    let rules = Rulebook::default().with_override(RuleKey::LoopedSoundWait, RuleSource::Bible);
    let mut view = firing_looped(&rules);
    let heard = combat_heard(&mut view);
    assert_eq!(heard.len(), 2, "on the first and fourth");
    assert!(heard.iter().all(|sound| !sound.until_stopped), "{heard:?}");
}

// Double time.

/// A detonation of the blaster's at the NPC, as `bööm` 128.
fn detonation() -> CombatEvent {
    CombatEvent::Exploded {
        at: Vec2::new(0.0, -100.0),
        explosion: Explosion {
            boom: BoomId(128),
            extra: false,
        },
        size: 0.0,
    }
}

#[test]
fn double_times_second_pass_ages_the_effects_again() {
    let detonated = || {
        let mut view = stepping(armed(1, &[BLASTER]));
        frame(&mut view, one_step(vec![fight(vec![detonation()])]));
        view
    };
    let mut twice = detonated();
    frame(&mut twice, one_step(vec![fight(vec![])]));
    frame(&mut twice, one_step(vec![fight(vec![])]));
    let mut doubled = detonated();
    frame(&mut doubled, one_step(vec![fight(vec![]), fight(vec![])]));
    assert_eq!(doubled.effects.explosions()[0].frame, 2.0);
    assert_eq!(doubled.effects, twice.effects);
}

#[test]
fn a_detonation_on_the_second_pass_is_shown_that_frame() {
    let mut view = stepping(armed(1, &[BLASTER]));
    frame(
        &mut view,
        one_step(vec![fight(vec![]), fight(vec![detonation()])]),
    );
    assert_eq!(view.effects.explosions().len(), 1);
}

// The shots' looks.

/// [`armed`] with the blaster's look set by `edit`.
fn blaster_looking(edit: impl FnOnce(&mut WeaponLook)) -> FakeCatalog {
    let mut catalog = armed(0, &[BLASTER]);
    let (_, look) = catalog
        .looks
        .iter_mut()
        .find(|(id, _)| *id == BLASTER.0)
        .expect("the blaster's look");
    edit(look.as_mut().expect("read"));
    catalog
}

/// A pass with the blaster's shot `id` in flight 50 above the player,
/// facing up, and `smoke_frame` as given.
fn shooting(id: u32, smoke_frame: bool) -> PassReport {
    PassReport {
        shots: vec![FrameShot {
            id: ShotId(id),
            weapon: BLASTER,
            position: Vec2::new(0.0, -50.0),
            ..record::shot()
        }],
        smoke_frame,
        ..fight(vec![])
    }
}

#[test]
fn a_spinning_shot_draws_its_start_once_on_the_effects_chance() {
    let catalog = blaster_looking(|look| {
        look.flags = crate::flight::weapons::SPINS;
        look.beam_width = 1;
    });
    let (effects, effects_chance) = scripted(&[20, 13]);
    let mut view = stepping(catalog);
    view.effects_chance = effects_chance;
    // It lives only through the first pass of a double-time step.
    frame(&mut view, one_step(vec![shooting(7, false), fight(vec![])]));
    assert_eq!(effects.borrow().asked, [36], "Rand(36), once");
    frame(&mut view, one_step(vec![shooting(8, false)]));
    assert_eq!(view.shot_looks.spin_starts.start(ShotId(8)), 13);
    frame(&mut view, one_step(vec![shooting(8, false)]));
    assert_eq!(effects.borrow().asked, [36, 36], "drawn once a shot");
}

#[test]
fn a_shot_in_flight_trails_each_pass_and_smokes_on_a_smoke_frame() {
    let catalog = blaster_looking(|look| {
        look.flags = crate::flight::smoke::SMALL_SMOKE;
        look.smoke_set = 1;
        look.particles = 3;
        look.part_vel = 100;
        look.part_life = (20, 20);
        look.part_color = 0x0010_40FF;
    });
    let mut view = stepping(catalog);
    let puffs = |view: &Faked| view.shot_looks.smoke().puffs().len();
    frame(&mut view, one_step(vec![shooting(7, false)]));
    assert_eq!(view.effects.particles().len(), 3, "a burst a pass");
    assert_eq!(puffs(&view), 0, "no smoke frame");
    frame(
        &mut view,
        one_step(vec![shooting(7, true), shooting(7, true)]),
    );
    assert_eq!(view.effects.particles().len(), 9);
    assert_eq!(puffs(&view), 2, "a puff each smoke-frame pass");
    let newest = view.effects.particles()[8];
    let start = crate::flight::trails::trail_start(Vec2::new(0.0, -50.0), 0.0, 0);
    assert_eq!(newest.at, Point::new(start.x, start.y));
}

#[test]
fn with_smoke_trails_off_a_shot_leaves_no_trail_and_no_smoke() {
    let catalog = blaster_looking(|look| {
        look.flags = crate::flight::smoke::SMALL_SMOKE;
        look.smoke_set = 1;
        look.particles = 3;
    });
    let mut view = stepping(catalog);
    view.smoke_trails = false;
    frame(&mut view, one_step(vec![shooting(7, true)]));
    assert_eq!(view.effects.particles(), []);
    assert_eq!(view.shot_looks.smoke().puffs(), []);
}

// The stars.

/// Draws in the middle of each range, so every star is scattered onto
/// the ship and none wraps, and rolls each `Rand` a step up.
#[derive(Default)]
struct Middling {
    rolls: u32,
}

impl Chance for Middling {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n / 2
    }

    fn roll(&mut self, sides: u16) -> u16 {
        self.rolls += 1;
        u16::try_from(self.rolls % u32::from(sides.max(1))).expect("below sides")
    }
}

/// A view of the stars' sheet, its stars scattered about the origin on a
/// [`Middling`] chance.
fn starry_stepping() -> Faked {
    let catalog = starry();
    let mut view = stepping(catalog);
    view.looks = Looks::read(&starry(), []);
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Middling::default()));
    view.effects_chance = SharedChance::new(chance);
    let surroundings =
        super::super::stars::surroundings_at((Vec2::ZERO, 0), VIEW_SIZE, anchor(), &view.looks);
    super::super::stars::reset(
        &mut view.stars,
        &surroundings,
        true,
        &mut view.effects_chance,
    );
    view
}

fn stars_now(view: &Faked) -> [Star; 20] {
    view.stars.stars()
}

#[test]
fn a_frame_scrolls_the_stars_by_its_first_passs_move_twice_over_in_double_time() {
    for double in [false, true] {
        let mut view = starry_stepping();
        let before = stars_now(&view);
        assert!(before.iter().all(|star| star.shown));
        // The ship moved from (8, -4) to the origin over the first pass.
        let run = StepRun {
            before: vec![(
                ShipRef::Player,
                ShipPlace {
                    position: Vec2::new(8.0, -4.0),
                    ..ShipPlace::default()
                },
            )],
            passes: vec![
                PassReport {
                    double_time: double,
                    ..PassReport::default()
                };
                if double { 2 } else { 1 }
            ],
            ..StepRun::default()
        };
        frame(
            &mut view,
            StepReport {
                steps: vec![run],
                stopped: false,
            },
        );
        let times = if double { 2.0 } else { 1.0 };
        let mut scrolled = 0;
        for (star, was) in stars_now(&view).iter().zip(before) {
            if was.factor > 0.0 {
                scrolled += 1;
            }
            assert_eq!(star.x, was.x - was.factor * (8.0 * times), "{double}");
            assert_eq!(star.y, was.y - was.factor * (-4.0 * times), "{double}");
        }
        assert!(scrolled > 0, "some star has a factor");
    }
}

#[test]
fn by_the_other_starfield_reading_no_pass_steps_the_stars() {
    let rules = Rulebook::default().with_override(RuleKey::FlightStarfield, RuleSource::Bible);
    let mut view = starry_stepping();
    view.presentation = super::super::stepping::Presentation::of(&rules);
    let before = stars_now(&view);
    let run = StepRun {
        before: vec![(
            ShipRef::Player,
            ShipPlace {
                position: Vec2::new(8.0, -4.0),
                ..ShipPlace::default()
            },
        )],
        passes: vec![PassReport::default()],
        ..StepRun::default()
    };
    frame(
        &mut view,
        StepReport {
            steps: vec![run],
            stopped: false,
        },
    );
    assert_eq!(stars_now(&view), before);
}

#[test]
fn a_star_of_factor_0_stays_fixed_in_world_units() {
    for double in [false, true] {
        let mut view = stepping(starry());
        view.looks = Looks::read(&starry(), []);
        let surroundings =
            super::super::stars::surroundings_at((Vec2::ZERO, 0), VIEW_SIZE, anchor(), &view.looks);
        super::super::stars::reset(
            &mut view.stars,
            &surroundings,
            false,
            &mut view.effects_chance,
        );
        let before = stars_now(&view);
        assert!(before.iter().all(|star| star.shown && star.factor == 0.0));
        let run = StepRun {
            before: vec![(
                ShipRef::Player,
                ShipPlace {
                    position: Vec2::new(8.0, -4.0),
                    ..ShipPlace::default()
                },
            )],
            passes: vec![
                PassReport {
                    double_time: double,
                    ..PassReport::default()
                };
                2
            ],
            ..StepRun::default()
        };
        frame(
            &mut view,
            StepReport {
                steps: vec![run],
                stopped: false,
            },
        );
        assert_eq!(stars_now(&view), before, "{double}");
    }
}

// The comm and the quotes.

/// A note of NPC 0's repairs.
fn repaired() -> CommNote {
    CommNote {
        from: NpcId(0),
        comm_name: "Ship 129".to_owned(),
        ship_name: "Shuttle".to_owned(),
        done: nova_sim::Help::Repair,
    }
}

/// The text `view`'s message line holds, shown or not.
fn line(view: &Faked) -> Option<&str> {
    view.message.as_ref().map(|message| message.text.as_str())
}

#[test]
fn a_steps_comm_is_said_after_the_steps() {
    let mut view = stepping(catalog());
    let report = StepReport {
        steps: vec![StepRun {
            comm: vec![repaired()],
            ..step(vec![fight(vec![])])
        }],
        stopped: false,
    };
    frame(&mut view, report);
    let said = comm_message(&repaired());
    assert_eq!(line(&view), Some(said.as_str()));
}

#[test]
fn a_passs_quote_is_said_after_the_steps_after_the_comm() {
    let mut view = stepping(catalog());
    let quote = PersonQuote {
        npc: NpcId(0),
        text: "Prepare to die!".to_owned(),
    };
    let report = StepReport {
        steps: vec![StepRun {
            comm: vec![repaired()],
            ..step(vec![PassReport {
                quotes: vec![quote],
                ..fight(vec![])
            }])
        }],
        stopped: false,
    };
    frame(&mut view, report);
    assert_eq!(line(&view), Some("Prepare to die!"), "the last said");
}

#[test]
fn the_inbox_comm_and_quotes_go_with_the_first_step_and_pass_that_ran() {
    let quote = PersonQuote {
        npc: NpcId(0),
        text: "Halt!".to_owned(),
    };
    let held = vec![
        FlightEvent::Comm(repaired()),
        FlightEvent::Quote(quote.clone()),
    ];
    // No step: both wait.
    let mut view = stepping(catalog());
    view.inbox.receive(held.clone());
    frame(&mut view, StepReport::default());
    assert_eq!(line(&view), None);
    assert_eq!(view.inbox.take_comm(), [repaired()]);
    assert_eq!(view.inbox.take_quotes(), std::slice::from_ref(&quote));
    // A step rescued on its advance pass: the comm goes, the quotes wait.
    let mut view = stepping(catalog());
    view.inbox.receive(held.clone());
    let rescued = StepRun {
        rescued: Some(ShipPlace::default()),
        ..step(vec![])
    };
    frame(
        &mut view,
        StepReport {
            steps: vec![rescued],
            stopped: true,
        },
    );
    assert_eq!(line(&view), Some(comm_message(&repaired()).as_str()));
    assert_eq!(view.inbox.take_comm(), []);
    assert_eq!(view.inbox.take_quotes(), std::slice::from_ref(&quote));
    // A pass: both go.
    let mut view = stepping(catalog());
    view.inbox.receive(held);
    frame(&mut view, one_step(vec![fight(vec![])]));
    assert_eq!(line(&view), Some("Halt!"));
    assert_eq!(view.inbox.take_quotes(), []);
}

// The frame's last-raised line: `_DisplayComm` replaces its slot with
// each line (@0x90251-0x90265), so the line shows the last raised.

/// A hail quote of NPC 0's.
fn halt() -> PersonQuote {
    PersonQuote {
        npc: NpcId(0),
        text: "Halt!".to_owned(),
    }
}

/// A fleet's arrival quote, said by the frame's steps.
fn fleet_hail() -> SimMessage {
    SimMessage::FleetArrivalQuote("Hail".to_owned())
}

/// Runs `view`'s steps over one frame whose flight raised `messages` and
/// reports `report`.
fn raised(view: &mut Faked, messages: Vec<SimMessage>, report: StepReport) {
    for message in messages {
        view.sim.queue_event(FlightEvent::Message(message));
    }
    let steps = u32::try_from(report.steps.len()).expect("a frame's steps");
    view.sim.queue_report(report);
    view.run_steps(steps);
}

#[test]
fn a_hail_quote_raised_after_a_fleets_arrival_quote_shows_for_its_ticks() {
    let mut view = stepping(catalog());
    let quote = halt();
    let report = StepReport {
        steps: vec![StepRun {
            last_line: Some(RaisedLine::Quote(quote.clone())),
            ..step(vec![PassReport {
                quotes: vec![quote.clone()],
                ..fight(vec![])
            }])
        }],
        stopped: false,
    };
    raised(&mut view, vec![fleet_hail()], report);
    assert_eq!(
        shown_to_flight(&view),
        Vec::<u16>::new(),
        "the flight set its own line"
    );
    faked_lasts(&mut view, &quote.text, quote.ticks());
}

#[test]
fn a_note_raised_after_a_quote_in_one_step_shows() {
    let mut view = stepping(catalog());
    let quote = halt();
    let report = StepReport {
        steps: vec![StepRun {
            comm: vec![repaired()],
            last_line: Some(RaisedLine::Comm(repaired())),
            ..step(vec![PassReport {
                quotes: vec![quote],
                ..fight(vec![])
            }])
        }],
        stopped: false,
    };
    raised(&mut view, vec![], report);
    faked_lasts(&mut view, &comm_message(&repaired()), repaired().ticks());
}

#[test]
fn a_message_raised_last_shows_over_the_quote() {
    let mut view = stepping(catalog());
    let quote = halt();
    let report = StepReport {
        steps: vec![StepRun {
            last_line: Some(RaisedLine::Message(fleet_hail())),
            ..step(vec![PassReport {
                quotes: vec![quote],
                ..fight(vec![])
            }])
        }],
        stopped: false,
    };
    raised(&mut view, vec![fleet_hail()], report);
    faked_lasts(&mut view, "Hail", fleet_hail().ticks());
}

#[test]
fn the_last_steps_line_wins_over_an_earlier_steps() {
    let mut view = stepping(catalog());
    let quote = halt();
    let report = StepReport {
        steps: vec![
            StepRun {
                last_line: Some(RaisedLine::Quote(quote.clone())),
                ..step(vec![PassReport {
                    quotes: vec![quote],
                    ..fight(vec![])
                }])
            },
            StepRun {
                comm: vec![repaired()],
                last_line: Some(RaisedLine::Comm(repaired())),
                ..step(vec![fight(vec![])])
            },
            step(vec![fight(vec![])]),
        ],
        stopped: false,
    };
    raised(&mut view, vec![fleet_hail()], report);
    faked_lasts(&mut view, &comm_message(&repaired()), repaired().ticks());
}

// The ships kept for the drawing, and the rescue.

#[test]
fn each_step_keeps_its_ships_as_they_were_before_it() {
    let mut view = stepping(catalog());
    let state = |x: f32| ShipPlace {
        position: Vec2::new(x, 0.0),
        ..ShipPlace::default()
    };
    let run = |x: f32| StepRun {
        before: vec![(ShipRef::Player, state(x)), (NPC, state(x + 1.0))],
        passes: vec![fight(vec![])],
        ..StepRun::default()
    };
    let rescued = frame(
        &mut view,
        StepReport {
            steps: vec![run(1.0), run(5.0)],
            stopped: false,
        },
    );
    assert!(!rescued);
    assert_eq!(view.previous, state(5.0), "the last step's");
    assert_eq!(view.npc_previous, [(NpcId(0), state(6.0))]);
}

#[test]
fn a_rescue_keeps_the_ship_where_it_put_it_and_says_so() {
    let mut view = stepping(catalog());
    let there = ShipPlace {
        position: Vec2::new(50.0, 50.0),
        ..ShipPlace::default()
    };
    let run = StepRun {
        rescued: Some(there),
        ..step(vec![])
    };
    let rescued = frame(
        &mut view,
        StepReport {
            steps: vec![run],
            stopped: true,
        },
    );
    assert!(rescued);
    assert_eq!(view.previous, there);
}
