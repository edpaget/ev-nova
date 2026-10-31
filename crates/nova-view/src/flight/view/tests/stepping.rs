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
use nova_sim::player_wrap::{PlayerWrap, WRAP_EDGE, WRAP_SHIFT};
use nova_sim::{CommNote, FireSound, Gauge, PersonQuote};

use super::faked_travel::{faked_lasts, shown_to_flight};

use crate::flight::stars::Star;
use crate::system::camera::{Anchor, Camera};

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

/// A frame of one double-time step of passes `first` and `second`, with
/// `between` the flight as it stood between them.
pub(super) fn doubled(first: PassReport, second: PassReport, between: FlightFrame) -> StepReport {
    StepReport {
        steps: vec![StepRun {
            between_passes: Some(Box::new(between)),
            ..step(vec![first, second])
        }],
        stopped: false,
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
    let mut both = detonated();
    let report = doubled(fight(vec![]), fight(vec![]), sol());
    frame(&mut both, report);
    frame(&mut both, stopped());
    assert_eq!(both.effects.explosions()[0].frame, 2.0);
    assert_eq!(both.effects, twice.effects);
}

#[test]
fn a_detonation_on_the_second_pass_is_shown_the_frame_after() {
    let mut view = stepping(armed(1, &[BLASTER]));
    let report = doubled(fight(vec![]), fight(vec![detonation()]), sol());
    frame(&mut view, report);
    assert_eq!(view.effects.explosions().len(), 0, "held with its pass");
    frame(&mut view, one_step(vec![fight(vec![])]));
    assert_eq!(view.effects.explosions().len(), 1);
}

// A double-time frame, drawn between its passes.

/// A frame whose stepping stopped before its first step.
fn stopped() -> StepReport {
    StepReport {
        steps: vec![],
        stopped: true,
    }
}

/// `sol()` with the player's ship at `x`.
fn sol_at(x: f32) -> FlightFrame {
    let mut frame = sol();
    frame.ships[0].position = Vec2::new(x, 0.0);
    frame
}

/// A view over a flight whose double-time step left the ship at 20, the
/// first pass having left it at 10: what it holds, and the flight now.
fn held() -> (Faked, FlightFrame, FlightFrame) {
    let mut view = stepping(armed(1, &[BLASTER]));
    let (first, now) = (sol_at(10.0), sol_at(20.0));
    view.sim.set_frame(now.clone());
    let report = doubled(fight(vec![]), fight(vec![]), first.clone());
    frame(&mut view, report);
    (view, first, now)
}

#[test]
fn a_double_time_frame_is_drawn_on_the_first_passs_state() {
    let (view, first, now) = held();
    assert_eq!(view.drawn(), &first);
    assert_eq!(view.frame, now, "the kept frame is the flight as it stands");
    view.check_frame_kept();
}

#[test]
fn the_first_passs_state_stays_drawn_on_a_frame_with_no_step() {
    let (mut view, first, _) = held();
    view.step_frame(0, anchor());
    assert_eq!(view.drawn(), &first);
}

#[test]
fn a_command_leaves_the_first_passs_state_drawn() {
    let (mut view, first, _) = held();
    view.send(FlightCommand::Jump);
    assert_eq!(view.drawn(), &first);
}

#[test]
fn the_next_step_replaces_it() {
    let (mut view, _, now) = held();
    frame(&mut view, one_step(vec![fight(vec![])]));
    assert_eq!(view.drawn(), &now);
}

#[test]
fn a_stop_with_no_step_releases_it() {
    let (mut view, _, now) = held();
    frame(&mut view, stopped());
    assert_eq!(view.drawn(), &now);
}

#[test]
fn a_step_with_no_second_pass_replays_its_one_pass_on_its_frame() {
    let mut view = stepping(armed(1, &[BLASTER]));
    let mut report = one_step(vec![fight(vec![detonation()])]);
    report.steps[0].between_passes = Some(Box::new(sol()));
    frame(&mut view, report);
    assert_eq!(view.effects.explosions().len(), 1, "only a second is held");
}

#[test]
fn a_single_time_step_draws_the_frame_as_it_stands() {
    let mut view = stepping(armed(1, &[BLASTER]));
    let now = sol_at(20.0);
    view.sim.set_frame(now.clone());
    frame(&mut view, one_step(vec![fight(vec![])]));
    assert_eq!(view.drawn(), &now);
}

#[test]
fn a_layout_a_placing_a_takeover_a_landing_and_the_holds_expiry_drop_it() {
    let (mut view, ..) = held();
    super::faked_screen::warp_ends(&mut view);
    view.arrive();
    assert_eq!(view.drawn(), &in_alpha(), "laid out afresh");

    let (mut view, _, now) = held();
    view.place_ship();
    assert_eq!(view.drawn(), &now, "placed");
    assert_eq!(view.previous, now.player().place());

    let (mut view, ..) = held();
    let taken = sol_at(40.0);
    view.sim
        .queue_answer_showing(Answer::Assigned(Assigned::MyShip), taken.clone());
    view.assign(Assignment::MyShip);
    assert_eq!(view.drawn(), &taken, "taken over");

    let (mut view, ..) = held();
    let landed = FlightFrame {
        landed: Some(StellarId(128)),
        ..sol_at(20.0)
    };
    view.sim.set_frame(landed.clone());
    view.refresh_frame();
    assert_eq!(view.drawn(), &landed, "landed");

    let (mut view, ..) = held();
    let expired = FlightFrame {
        hold_expired: true,
        ..sol_at(20.0)
    };
    view.sim.set_frame(expired.clone());
    view.refresh_frame();
    assert_eq!(view.drawn(), &expired, "the hold run out");
}

#[test]
fn a_many_step_frame_holds_its_last_step_and_replays_the_others_in_full() {
    let mut view = stepping(armed(1, &[BLASTER]));
    let run = |between: &FlightFrame| StepRun {
        between_passes: Some(Box::new(between.clone())),
        ..step(vec![fight(vec![]), fight(vec![detonation()])])
    };
    let last = sol_at(30.0);
    let report = StepReport {
        steps: vec![run(&sol_at(10.0)), run(&last)],
        stopped: false,
    };
    view.sim.set_frame(sol_at(40.0));
    frame(&mut view, report);
    assert_eq!(view.drawn(), &last);
    assert_eq!(view.effects.explosions().len(), 1, "the earlier step's");
    frame(&mut view, stopped());
    assert_eq!(view.effects.explosions().len(), 2, "the last one's too");
}

#[test]
fn a_double_time_frame_smooths_over_two_passes() {
    let mut view = stepping(armed(1, &[BLASTER]));
    let mut first = sol_at(10.0);
    first.ships[0].heading = 10.0;
    first.ships.push(the_npc(Vec2::new(110.0, -100.0)));
    view.sim.set_frame(sol_at(20.0));
    let mut report = doubled(fight(vec![]), fight(vec![]), first);
    report.steps[0].before = vec![
        (ShipRef::Player, place(Vec2::ZERO)),
        (NPC, place(Vec2::new(100.0, -100.0))),
    ];
    frame(&mut view, report);
    for (alpha, x, heading) in [(0.0, -10.0, 350.0), (0.5, 0.0, 0.0), (1.0, 10.0, 10.0)] {
        view.alpha = alpha;
        assert_eq!(view.shown_position(), Point::new(x, 0.0), "{alpha}");
        assert_eq!(view.shown_heading(), heading, "{alpha}");
        let npc = view.drawn().ships[1];
        let (at, _) = view.shown_npc(&npc);
        assert_eq!(at, Point::new(100.0 + x, -100.0), "{alpha}");
    }
    assert_eq!(view.alpha(), 1.0, "the display's own stays raw");
}

#[test]
fn a_double_time_frame_steps_shots_boxes_and_asteroids_back_two_passes() {
    let (at, velocity) = (Vec2::new(100.0, 50.0), Vec2::new(8.0, -4.0));
    let first = FlightFrame {
        shots: vec![FrameShot {
            weapon: WeaponId(9999),
            position: at,
            velocity,
            ..record::shot()
        }],
        cargo_boxes: vec![nova_sim::flight_frame::FrameBox {
            velocity,
            ..super::faked_world::box_at(at)
        }],
        asteroids: vec![nova_sim::flight_frame::FrameAsteroid {
            velocity,
            ..super::faked_world::asteroid_at(at)
        }],
        ..sol()
    };
    let mut view = stepping(armed(1, &[BLASTER]));
    view.sim.set_frame(sol());
    let report = doubled(fight(vec![]), fight(vec![]), first);
    frame(&mut view, report);
    let camera = Camera::centred_on_in(Point::new(0.0, 0.0), VIEW_SIZE);
    for (alpha, back) in [(1.0, 0.0), (0.5, 1.0), (0.0, 2.0)] {
        view.alpha = alpha;
        let drawn = camera.whole_point(Point::new(100.0 - 8.0 * back, 50.0 + 4.0 * back));
        assert_eq!(view.shown_shots(&camera)[0].at, drawn, "{alpha}");
        assert_eq!(view.shown_boxes(&camera)[0].at, drawn, "{alpha}");
        assert_eq!(view.shown_asteroids(&camera)[0].at, drawn, "{alpha}");
    }
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
    starry_about(Vec2::ZERO)
}

/// [`starry_stepping`], its stars scattered about `ship`.
fn starry_about(ship: Vec2) -> Faked {
    let catalog = starry();
    let mut view = stepping(catalog);
    view.looks = Looks::read(&starry(), []);
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Middling::default()));
    view.effects_chance = SharedChance::new(chance);
    let surroundings =
        super::super::stars::surroundings_at((ship, 0), VIEW_SIZE, anchor(), &view.looks);
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

// The player's wrap: what the view carries as it reaches the pass that ran
// it (see the `stepping` module).

/// Where the ship stood as it wrapped.
const STOOD: Vec2 = Vec2::new(WRAP_EDGE + 10.0, 40.0);

/// What the wrap added.
const SHIFT: Vec2 = Vec2::new(-WRAP_SHIFT, 0.0);

/// The ship's move a pass.
const MOVE: Vec2 = Vec2::new(8.0, -4.0);

fn wrap() -> PlayerWrap {
    PlayerWrap::of(STOOD).expect("past the edge")
}

/// A pass that ended with the ship at `at`, `wrap` run at its head.
fn wrapping(at: Vec2, wrap: Option<PlayerWrap>) -> PassReport {
    PassReport {
        ships: vec![the_player(at)],
        player_position: at,
        wrap,
        ..PassReport::default()
    }
}

/// The ship's place before a step, at `at`.
fn place(at: Vec2) -> ShipPlace {
    ShipPlace {
        position: at,
        ..ShipPlace::default()
    }
}

/// The frame after a wrap on a step's one pass: the ship [`MOVE`]d on past
/// where it landed, and [`NPC`], from `npc` before the step to `npc_now`,
/// in the frame.
fn after_a_wrap(view: &mut Faked, npc: Vec2, npc_now: Vec2) {
    let at = STOOD + SHIFT + MOVE;
    let mut flown = in_sol();
    flown.ships = vec![the_player(at), the_npc(npc_now)];
    view.sim.set_frame(flown);
    let run = StepRun {
        before: vec![(ShipRef::Player, place(STOOD)), (NPC, place(npc))],
        passes: vec![wrapping(at, Some(wrap()))],
        ..StepRun::default()
    };
    frame(
        view,
        StepReport {
            steps: vec![run],
            stopped: false,
        },
    );
}

fn close(a: Point, b: Point, within: f32) -> bool {
    (a.x - b.x).hypot(a.y - b.y) <= within
}

#[test]
fn across_a_wrap_the_ships_and_the_camera_are_drawn_within_a_passs_move_of_the_frame() {
    let mut view = stepping(catalog());
    let near = STOOD + Vec2::new(-100.0, 30.0);
    after_a_wrap(&mut view, near, near + SHIFT + MOVE);
    let (player, npc) = (view.frame.ships[0], view.frame.ships[1]);
    let step = MOVE.length();
    for alpha in [0.0, 0.25, 0.5, 1.0] {
        view.alpha = alpha;
        let frame_at = |ship: &FrameShip| Point::new(ship.position.x, ship.position.y);
        let shown = view.shown_position();
        assert!(close(shown, frame_at(&player), step), "{alpha}: {shown:?}");
        let (shown, _) = view.shown_npc(&npc);
        assert!(close(shown, frame_at(&npc), step), "{alpha}: {shown:?}");
        let camera = view.camera();
        let (ship, screen) = (frame_at(&player), camera.world_to_screen(frame_at(&player)));
        let drawn = camera.world_to_screen(view.shown_position());
        assert!(close(drawn, camera.anchor(), 1e-3), "{alpha}: {ship:?}");
        assert!(close(screen, drawn, step + 1.0), "{alpha}: {screen:?}");
    }
}

#[test]
fn an_npc_the_wrap_left_is_drawn_where_the_frame_has_it() {
    let mut view = stepping(catalog());
    let far = STOOD - Vec2::new(WRAP_EDGE * 2.0, 0.0);
    after_a_wrap(&mut view, far, far);
    let npc = view.frame.ships[1];
    for alpha in [0.0, 0.5, 1.0] {
        view.alpha = alpha;
        assert_eq!(view.shown_npc(&npc).0, Point::new(far.x, far.y), "{alpha}");
    }
}

/// A frame of one step of `passes` whose ship stood at `from`.
fn step_from(view: &mut Faked, from: Vec2, passes: Vec<PassReport>) {
    let run = StepRun {
        before: vec![(ShipRef::Player, place(from))],
        passes,
        ..StepRun::default()
    };
    let report = StepReport {
        steps: vec![run],
        stopped: false,
    };
    frame(view, report);
}

/// Asserts each of the stars moved `by` the ship's move, its factor's.
fn scrolled_by(view: &Faked, before: &[Star; 20], by: Vec2) {
    let mut scrolled = 0;
    for (star, was) in stars_now(view).iter().zip(before) {
        if was.factor > 0.0 {
            scrolled += 1;
        }
        assert_eq!(star.x, was.x - was.factor * by.x);
        assert_eq!(star.y, was.y - was.factor * by.y);
    }
    assert!(scrolled > 0, "some star has a factor");
}

#[test]
fn a_first_pass_wrap_scrolls_the_stars_by_the_ships_move_and_not_the_shift() {
    let mut view = starry_stepping();
    let before = stars_now(&view);
    let at = STOOD + SHIFT + MOVE;
    step_from(&mut view, STOOD, vec![wrapping(at, Some(wrap()))]);
    scrolled_by(&view, &before, Vec2::ZERO - MOVE);
}

#[test]
fn a_second_pass_wrap_leaves_the_first_passs_scroll_alone() {
    let (start, first) = (STOOD - MOVE - MOVE, STOOD - MOVE);
    let mut view = starry_about(first);
    let before = stars_now(&view);
    let passes = vec![
        PassReport {
            double_time: true,
            ..wrapping(first, None)
        },
        PassReport {
            double_time: true,
            ..wrapping(STOOD + SHIFT + MOVE, Some(wrap()))
        },
    ];
    step_from(&mut view, start, passes);
    scrolled_by(&view, &before, Vec2::ZERO - MOVE * 2.0);
    assert_eq!(view.previous.position, start + SHIFT);
}

/// A double-time step from `start` to the ship past the wrap, its first
/// pass leaving it at `first`, that pass's frame held.
fn wrapped_on_the_second_pass(start: Vec2, first: Vec2) -> StepReport {
    let pass = |pass: PassReport| PassReport {
        double_time: true,
        ..pass
    };
    StepReport {
        steps: vec![StepRun {
            before: vec![(ShipRef::Player, place(start))],
            between_passes: Some(Box::new(sol_at(first.x))),
            passes: vec![
                pass(wrapping(first, None)),
                pass(wrapping(STOOD + SHIFT + MOVE, Some(wrap()))),
            ],
            ..StepRun::default()
        }],
        stopped: false,
    }
}

#[test]
fn a_second_pass_wrap_is_carried_on_the_frame_after() {
    let (start, first) = (STOOD - MOVE - MOVE, STOOD - MOVE);
    let mut view = stepping(armed(1, &[BLASTER]));
    let near = PassReport {
        combat: vec![CombatEvent::Exploded {
            at: first + Vec2::new(-100.0, 0.0),
            explosion: Explosion {
                boom: BoomId(128),
                extra: false,
            },
            size: 0.0,
        }],
        ..wrapping(first, None)
    };
    frame(&mut view, one_step(vec![near]));
    let boom = view.effects.explosions()[0].at;
    frame(&mut view, wrapped_on_the_second_pass(start, first));
    assert_eq!(view.previous.position, start, "uncarried while it shows");
    assert_eq!(view.effects.explosions()[0].at, boom);
    frame(&mut view, stopped());
    assert_eq!(view.previous.position, start + SHIFT);
    let carried = Point::new(boom.x + SHIFT.x, boom.y + SHIFT.y);
    assert_eq!(view.effects.explosions()[0].at, carried);
}

#[test]
fn a_held_second_pass_wrap_leaves_the_first_passs_scroll_alone() {
    let (start, first) = (STOOD - MOVE - MOVE, STOOD - MOVE);
    let mut view = starry_about(first);
    let before = stars_now(&view);
    frame(&mut view, wrapped_on_the_second_pass(start, first));
    scrolled_by(&view, &before, Vec2::ZERO - MOVE * 2.0);
    frame(&mut view, stopped());
    scrolled_by(&view, &before, Vec2::ZERO - MOVE * 2.0);
}

#[test]
fn a_wrap_marks_the_stars_so_none_streaks_across_it() {
    let mut view = starry_stepping();
    frame(&mut view, one_step(vec![wrapping(Vec2::ZERO, None)]));
    let at = STOOD + SHIFT + MOVE;
    step_from(&mut view, STOOD, vec![wrapping(at, Some(wrap()))]);
    // Drawn at its placement at any alpha, as a jumped star is.
    let list = |view: &Faked, alpha: f32| {
        let mut view_list = DrawList::new();
        let sheet = view.looks.star_sheet().expect("the stars' sheet");
        view.stars.draw(&mut view_list, sheet, alpha);
        view_list
    };
    assert_eq!(list(&view, 0.0), list(&view, 1.0));
}

/// Effects at `near` and `far` of [`STOOD`]: an explosion, a pod and a puff
/// each, and particles at `near`.
fn effects_about(view: &mut Faked, near: Vec2, far: Vec2) {
    let shot_at = |id, at| FrameShot {
        id: ShotId(id),
        weapon: BLASTER,
        position: at,
        ..record::shot()
    };
    let exploded = |at| CombatEvent::Exploded {
        at,
        explosion: Explosion {
            boom: BoomId(128),
            extra: false,
        },
        size: 0.0,
    };
    let podded = |at| CombatEvent::PodEjected {
        ship: NPC,
        at,
        velocity: Vec2::ZERO,
    };
    let burst = CombatEvent::Particles(nova_sim::combat::ParticleBurst {
        at: near,
        speed: 0.0,
        variation: 0,
        life: (300, 300),
        color: 0x00FF_8000,
        count: 1,
        spread: 0.0,
    });
    let pass = PassReport {
        shots: vec![shot_at(1, near), shot_at(2, far)],
        smoke_frame: true,
        combat: vec![
            exploded(near),
            exploded(far),
            podded(near),
            podded(far),
            burst,
        ],
        ..wrapping(STOOD, None)
    };
    frame(view, one_step(vec![pass]));
}

/// [`effects_about`]'s places, as lists of where each explosion, pod and
/// puff is and each particle.
fn places(view: &Faked) -> (Vec<Point>, Vec<Point>, Vec<Vec2>, Point) {
    (
        view.effects
            .explosions()
            .iter()
            .map(|boom| boom.at)
            .collect(),
        view.effects.pods().iter().map(|pod| pod.at).collect(),
        view.shot_looks
            .smoke()
            .puffs()
            .iter()
            .map(|puff| puff.at)
            .collect(),
        view.effects.particles()[0].at,
    )
}

#[test]
fn explosions_pods_and_smoke_near_the_ship_keep_their_place_on_screen_and_those_far_stay() {
    let catalog = blaster_looking(|look| {
        look.flags = crate::flight::smoke::SMALL_SMOKE;
        look.smoke_set = 1;
    });
    let mut view = stepping(catalog);
    let (near, far) = (
        STOOD + Vec2::new(-100.0, 0.0),
        STOOD - Vec2::new(WRAP_EDGE * 2.0, 0.0),
    );
    effects_about(&mut view, near, far);
    let (booms, pods, puffs, particle) = places(&view);
    assert_eq!((booms.len(), pods.len(), puffs.len()), (2, 2, 2));
    let drift = view.effects.pods()[0].velocity.x;
    frame(
        &mut view,
        one_step(vec![wrapping(STOOD + SHIFT, Some(wrap()))]),
    );
    let shifted = |at: Point| Point::new(at.x + SHIFT.x, at.y + SHIFT.y);
    let (now_booms, now_pods, now_puffs, now_particle) = places(&view);
    assert_eq!(now_booms, [shifted(booms[0]), booms[1]]);
    assert!((now_pods[0].x - (shifted(pods[0]).x + drift)).abs() < 1e-2);
    assert!((now_pods[1].x - (pods[1].x + drift)).abs() < 1e-2);
    assert_eq!(now_puffs, [puffs[0] + SHIFT, puffs[1]]);
    assert_eq!(now_particle.x, particle.x, "the particles aren't carried");
}

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
