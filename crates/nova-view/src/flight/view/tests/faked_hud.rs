//! The HUD's, the radar's and the target panel's component tests on
//! [`FakeFlight`] (rdm:phase/view-sim-seam/phase-8-hud-and-target-frame):
//! a view over a scripted frame, its status bar and target panel drawn
//! from the frame's HUD and target parts, and the keys and loads they
//! send through the port. Kept apart from `tests/mod.rs`, which is large.

use super::faked_world::{in_sol_with, npc_129, the_player_at, world, world_by};
use super::*;
use crate::flight::radar::RadarShip;
use nova_sim::flight_frame::{FrameShip, FrameTarget};
use nova_sim::{AlertIcon, ShipIff, StellarIff, TargetStatus, TargetTag};

/// A view over a flight scripted to show `frame`, read from `catalog` by
/// `rules` ([`world_by`]), with the status bar the catalog gives a ship of
/// no government, as a session's view chooses it.
pub(super) fn barred_by(catalog: FakeCatalog, frame: FlightFrame, rules: &Rulebook) -> Faked {
    let mut view = world_by(catalog, frame, rules);
    view.status_bar = hud::choose_status_bar(&*view.catalog, None);
    view.bar_government = Some(super::super::BarFor(None));
    view
}

/// [`barred_by`] by the default rules.
pub(super) fn barred(catalog: FakeCatalog, frame: FlightFrame) -> Faked {
    barred_by(catalog, frame, &Rulebook::default())
}

/// What `view` draws of its flight, the fades left out: the world, then
/// the bar, from the kept frame, which must be the flight's.
pub(super) fn flown(view: &Faked) -> DrawList {
    view.check_frame_kept();
    let mut list = DrawList::new();
    assert!(view.draw_flight(&mut list, String::new), "a scene to draw");
    list
}

/// One tick's HUD work, as a flight's tick ends: the clock moved on a
/// tick, then the HUD advanced ([`FlightView::advance_hud`]).
pub(super) fn hud_tick(view: &mut Faked) {
    view.step_stellars(TICK);
    view.advance_hud();
}

/// Shows `frame` from now on, as a step leaves it
/// ([`super::faked_world::shows`]).
pub(super) fn shows(view: &mut Faked, frame: FlightFrame) {
    super::faked_world::shows(view, frame);
}

/// NPC `id` of ship 129 at (100, -100), facing up, of government 140.
pub(super) fn npc_of_140(id: u32) -> FrameShip {
    FrameShip {
        govt: Some(GovtId(140)),
        ..npc_129(id, Vec2::new(100.0, -100.0), 0.0)
    }
}

/// A frame in Sol holding the player and `npcs`, with ship 129's traffic,
/// whose target cards are read.
pub(super) fn among(npcs: Vec<FrameShip>) -> FlightFrame {
    let mut ships = vec![the_player_at(Vec2::ZERO, 0.0)];
    ships.extend(npcs);
    FlightFrame {
        traffic_ships: vec![ShipId(129)],
        ..in_sol_with(ships)
    }
}

/// [`among`], with the target as `target` says.
pub(super) fn targeted(npcs: Vec<FrameShip>, target: FrameTarget) -> FlightFrame {
    FlightFrame {
        target: Some(target),
        ..among(npcs)
    }
}

#[test]
fn a_description_is_read_from_the_catalog_and_loaded_through_the_flight() {
    let movie = nova_sim::DescMovie::from_desc("Port.mov", 0x0001).expect("named");
    let catalog = FakeCatalog {
        descriptions: vec![(3000, "raw".to_owned())],
        desc_movies: vec![(3000, movie.clone())],
        ..catalog()
    };
    let mut view = faked_in(catalog, in_sol());
    view.sim.queue_description("loaded");
    assert_eq!(view.load_description(3000).as_deref(), Ok("loaded"));
    assert_eq!(
        view.load_description(9999),
        Err("no dësc 9999".to_owned()),
        "a missing one"
    );
    assert_eq!(
        view.sim.descriptions_loaded(),
        [(Some("raw".to_owned()), Some(movie)), (None, None)],
        "the missing one loaded as missing"
    );
}

// The radar.

#[test]
fn the_radar_shows_each_ship_by_its_records_iff_and_the_frames_target() {
    let threat = FrameShip {
        iff: ShipIff::Threat,
        ..npc_of_140(0)
    };
    let off = FrameShip {
        on_radar: false,
        ..npc_129(1, Vec2::new(-50.0, 0.0), 0.0)
    };
    let heavy = FrameShip {
        iff: ShipIff::Escort,
        mass: 100.0,
        ..npc_129(2, Vec2::new(0.0, 80.0), 0.0)
    };
    let view = world(
        armed(1, &[]),
        targeted(vec![threat, off, heavy], record::target(0)),
    );
    let radar = view.radar_now(&view.frame);
    let ships: Vec<_> = radar
        .ships
        .iter()
        .map(
            |&RadarShip {
                 targeted,
                 iff,
                 heavy,
                 ..
             }| (targeted, iff, heavy),
        )
        .collect();
    assert_eq!(
        ships,
        [
            (true, ShipIff::Threat, false),
            (false, ShipIff::Escort, true)
        ],
        "the one off the radar left out"
    );
}

#[test]
fn a_stellars_radar_colour_is_its_records_iff_unlandable_without_one() {
    let mut frame = in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]);
    frame.stellars[0].iff = Some(StellarIff::Hostile);
    frame.stellars[1].iff = None;
    let view = world(catalog(), frame);
    let iffs: Vec<_> = view
        .radar_now(&view.frame)
        .stellars
        .iter()
        .map(|stellar| stellar.iff)
        .collect();
    assert_eq!(iffs, [StellarIff::Hostile, StellarIff::Unlandable]);
}

#[test]
fn the_radar_is_blank_docked_and_shows_the_frames_gear_and_static() {
    let mut frame = in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]);
    frame.docked = true;
    frame.hud.radar_gear = nova_sim::RadarGear {
        iff: true,
        density_scanner: true,
        anti_interference: 0,
    };
    frame.hud.interference = 100;
    frame.nav_mode = NavMode::Hyperspace(None);
    let mut view = world(catalog(), frame);
    view.effects_chance = rolling_one();
    hud_tick(&mut view);
    let radar = view.radar_now(&view.frame);
    assert!(radar.blank && radar.iff && radar.density_scanner);
    assert!(radar.hyperspace_mode);
    assert!(radar.static_pattern.is_some(), "full interference");
    view.sim.frame_mut().hud.interference = 0;
    view.refresh_frame();
    hud_tick(&mut view);
    assert_eq!(view.radar_now(&view.frame).static_pattern, None);
}

// The target panel.

#[test]
fn the_target_panel_shows_the_frames_target_with_its_types_card_and_its_code() {
    let target = FrameTarget {
        npc: NpcId(0),
        name: "Ace".to_owned(),
        subtitle: None,
        status: TargetStatus::Disabled,
        tag: Some(TargetTag::Code(GovtId(140))),
    };
    let view = world(armed(1, &[]), targeted(vec![npc_of_140(0)], target.clone()));
    assert_eq!(
        view.target_now(&view.frame),
        Some(TargetPanel {
            name: "Ace".to_owned(),
            subtitle: "Light Transport".to_owned(),
            picture: Some(3001),
            status: TargetStatus::Disabled,
            tag: Some(Tag::Code("Fed.".to_owned())),
        })
    );
    let subtitled = FrameTarget {
        subtitle: Some("Bounty Hunter".to_owned()),
        tag: Some(TargetTag::Escort),
        ..target
    };
    let view = world(armed(1, &[]), targeted(vec![npc_of_140(0)], subtitled));
    let panel = view.target_now(&view.frame).expect("a target");
    assert_eq!(panel.subtitle, "Bounty Hunter", "in place of its type's");
    assert_eq!(panel.tag, Some(Tag::Escort));
    let view = world(armed(1, &[]), among(vec![npc_of_140(0)]));
    assert_eq!(view.target_now(&view.frame), None, "no target");
}

// The status bar.

#[test]
fn the_bar_lists_the_frames_cargo_and_names_its_secondary_with_its_count() {
    let mut frame = among(Vec::new());
    frame.hud.cargo.cash = 12_345;
    let mut view = barred(armed(0, &[BLASTER, ROCKET]), frame);
    let list = texts(&flown(&view));
    assert!(list.contains(&"12,345".to_owned()), "{list:?}");
    assert!(list.contains(&NO_SECONDARY.to_owned()), "{list:?}");
    assert!(list.contains(&NO_TARGET.to_owned()), "{list:?}");
    let mut frame = view.frame.clone();
    frame.hud.secondary = Some(ROCKET);
    frame.hud.secondary_line_count = Some(3);
    shows(&mut view, frame);
    let list = texts(&flown(&view));
    assert!(list.contains(&"Rocket - 3".to_owned()), "{list:?}");
}

#[test]
fn the_port_bar_draws_the_flight_as_it_stands_not_the_kept_frame() {
    let mut view = barred(armed(1, &[]), among(Vec::new()));
    view.sim.frame_mut().hud.cargo.cash = 777;
    let mut bar = DrawList::new();
    view.draw_status_bar(&mut bar);
    assert!(texts(&bar).contains(&"777".to_owned()), "{:?}", texts(&bar));
}

// The target layer.

#[test]
fn the_red_alert_icon_is_the_frames() {
    let mut frame = among(Vec::new());
    frame.hud.red_alert = Some(AlertIcon::Red);
    let mut view = world(catalog(), frame);
    view.alert_width = Some(32);
    let icon = ImageKey::icon(AlertIcon::Red.cicn());
    let drawn_icon = |list: &DrawList| {
        list.iter()
            .any(|command| matches!(command, DrawCommand::Picture { image, .. } if *image == icon))
    };
    assert!(drawn_icon(&super::faked_world::world_list(&view)));
    let mut frame = view.frame.clone();
    frame.hud.red_alert = None;
    shows(&mut view, frame);
    assert!(!drawn_icon(&super::faked_world::world_list(&view)));
}

// The keys and the bar's choice.

#[test]
fn weapon_select_sends_its_press_backwards_with_option() {
    let mut view = faked(in_sol());
    view.press_weapon_select();
    view.track_held(&key(Key::Alt, true));
    view.press_weapon_select();
    assert_eq!(
        sent(&view),
        [
            FlightCommand::SelectWeapon { backwards: false },
            FlightCommand::SelectWeapon { backwards: true },
        ]
    );
}

#[test]
fn the_bar_follows_the_government_the_frame_chooses_it_by() {
    let mut view = barred(governed(catalog(), 128, 129), among(Vec::new()));
    assert_eq!(
        view.status_bar.as_ref().map(|bar| &bar.layout),
        Ok(&layout())
    );
    view.sim.frame_mut().hud.interface_government = Some(GovtId(150));
    view.follow_bar();
    assert_eq!(
        view.status_bar.as_ref().map(|bar| &bar.layout),
        Ok(&bar_129()),
        "gövt 150's Interface"
    );
    assert_eq!(view.interface_now(), Some(Ok(129)));
    let mut failed = faked(in_sol());
    failed.sim.frame_mut().hud.interface_government = Some(GovtId(150));
    failed.follow_bar();
    assert!(failed.status_bar.is_err(), "no bar chosen, none followed");
    assert_eq!(failed.interface_now(), None);
}

#[test]
fn the_hud_waits_for_its_refreshes_by_the_presentations_cadence() {
    assert!(faked(in_sol()).hud_waits(), "the engine's");
    let rules = Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Bible);
    let view = world_by(catalog(), among(Vec::new()), &rules);
    assert!(!view.hud_waits(), "the Bible's");
}
