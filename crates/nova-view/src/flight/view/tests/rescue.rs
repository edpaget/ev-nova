//! The escape pod's rescue on screen, on [`FakeFlight`] (see the
//! `rescue` module): no step while it shows, the pod's last frame fading
//! out, the layout once it is black, the rescue's `dësc` and texts held
//! until their stages, the faded flag, the closing fade, the pointer and
//! what is pressed under it. Kept apart from `tests/mod.rs`, which is
//! large.

use std::time::Duration;

use super::faked_hud::barred;
use super::*;
use crate::flight::rescue_screen::RESCUE_FADE_FOR;
use nova_sim::flight_frame::{FlightEvent, RescueTexts, ShipPlace, StepReport, StepRun};
use nova_sim::{MissionTextKind, PlayerFate, SessionEvent, TextBar};

/// The pod's system's `BkgndColor`.
const POD_BACKGROUND: u32 = 0x0000_0010;

/// The `BkgndColor` of the system the rescue takes the pod to.
const NEW_BACKGROUND: u32 = 0x0010_0000;

/// A view over Sol with the player in the escape pod, its looks read and
/// its status bar chosen.
fn in_the_pod() -> Faked {
    barred(
        catalog(),
        FlightFrame {
            fate: PlayerFate::Ejected { ticks_left: 1 },
            system_background: Some(POD_BACKGROUND),
            ..sol()
        },
    )
}

/// The frame the rescue leaves: the player flying again in system 131,
/// no stellar on the radar.
fn rescued_frame() -> FlightFrame {
    FlightFrame {
        system: SystemId(131),
        ships: vec![the_player_at(Vec2::new(50.0, 50.0), 90.0)],
        radar_stellars: Vec::new(),
        system_background: Some(NEW_BACKGROUND),
        ..sol()
    }
}

/// A mission text the rescue queued.
fn briefed() -> MissionShown {
    MissionShown::Text(nova_sim::MissionText {
        kind: MissionTextKind::Brief,
        mission: MissionId(131),
        text: "Briefed.".to_owned(),
        graphic: None,
        movie: None,
        map: None,
    })
}

/// Ticks `view` through the step that rescues its pod, the rescue
/// queueing `texts`.
fn rescue_with(view: &mut Faked, texts: Vec<MissionShown>) {
    let texts = texts.into_iter().map(|shown| (shown, None)).collect();
    rescue_noting(view, texts, None);
}

/// Ticks `view` through the step that rescues its pod, the rescue
/// queueing `texts`, each with the bar it noted, and noting `marks`.
fn rescue_noting(
    view: &mut Faked,
    texts: Vec<(MissionShown, Option<TextBar>)>,
    marks: Option<RescueTexts>,
) {
    let mut events = vec![
        FlightEvent::Session(SessionEvent::SystemChanged),
        FlightEvent::RescueDesc(rescued(13999)),
    ];
    events.extend(
        texts
            .into_iter()
            .map(|(shown, bar)| FlightEvent::MissionShown { shown, bar }),
    );
    view.sim.queue_report_telling(
        StepReport {
            steps: vec![StepRun {
                rescued: Some(ShipPlace {
                    position: Vec2::new(50.0, 50.0),
                    heading: 90.0,
                }),
                rescue_texts: marks,
                ..StepRun::default()
            }],
            stopped: true,
        },
        rescued_frame(),
        events,
    );
    view.tick(TICK);
}

/// The reserves the texts below note: unlike the pod's or the rescued
/// frame's.
fn noted_reserves() -> nova_sim::Reserves {
    let mut reserves = sol().hud.reserves;
    reserves.fuel.now += 50.0;
    reserves.fuel.max += 100.0;
    reserves
}

/// [`briefed`], the third text the session queued, noting
/// [`noted_reserves`] and the nav off.
fn briefed_third() -> (MissionShown, Option<TextBar>) {
    let bar = TextBar {
        texts: 3,
        nav: nova_sim::NavMode::Off,
        reserves: noted_reserves(),
    };
    (briefed(), Some(bar))
}

/// The bar a view of `frame` draws, its reserves [`noted_reserves`] and
/// its nav off: what `_RedrawStatus(1)` draws of a text's bar.
fn bar_noted_over(frame: FlightFrame) -> DrawList {
    let mut frame = frame;
    frame.hud.reserves = noted_reserves();
    frame.nav_mode = nova_sim::NavMode::Off;
    let view = barred(catalog(), frame);
    let mut bar = DrawList::new();
    view.draw_bar_or_why(&mut bar);
    bar
}

/// What `view` draws under its first text once it has faded in.
fn under_the_first_text(view: &mut Faked) -> DrawList {
    ticks(view, 2 * RESCUE_FADE_FOR);
    ticks(view, RESCUE_FADE_FOR);
    assert_eq!(view.take_mission_texts().len(), 1, "the texts are due");
    view.screen_faded_in();
    drawn(view)
}

/// [`rescue_with`] no texts.
fn rescue(view: &mut Faked) {
    rescue_with(view, Vec::new());
}

/// Ticks `view` for `time`, a tick at a time, the last one short.
fn ticks(view: &mut Faked, time: Duration) {
    let mut left = time;
    while !left.is_zero() {
        let dt = left.min(TICK);
        view.tick(dt);
        left -= dt;
    }
}

/// A view rescued with `texts`, ticked until the rescue's `dësc` is due.
fn at_the_desc(texts: Vec<MissionShown>) -> Faked {
    let mut view = in_the_pod();
    rescue_with(&mut view, texts);
    ticks(&mut view, 2 * RESCUE_FADE_FOR);
    view
}

/// A view rescued with `texts`, its `dësc` shown and closed, ticked until
/// the texts are due: the tick after the `dësc`'s, then the second fade.
fn under_the_texts(texts: Vec<MissionShown>) -> Faked {
    let mut view = at_the_desc(texts);
    assert!(view.take_rescue_desc().is_some());
    ticks(&mut view, RESCUE_FADE_FOR);
    view
}

/// The alpha of the black over the whole view, if the list ends in one.
fn black_over(list: &DrawList) -> Option<u8> {
    let mut tail = DrawList::new();
    tail.push(list.iter().last()?.clone());
    let (_, color) = fade(&tail)?;
    ((color.r, color.g, color.b) == (0, 0, 0)).then_some(color.a)
}

/// The space area painted `background`, as the rescue's repaints paint
/// it.
fn space_painted(view: &Faked, background: u32) -> DrawCommand {
    crate::draw::fill_command(view.play_rect(), Color::from_rgb24_as_rgb555(background))
}

#[test]
fn the_rescue_fades_the_pods_last_frame_to_black_and_steps_no_more() {
    let mut view = in_the_pod();
    view.tick(TICK);
    let pod = drawn(&view);
    rescue(&mut view);
    assert_eq!(drawn(&view), pod, "the pod's last frame, not yet dark");
    let stepped = view.sim.steps();
    ticks(&mut view, RESCUE_FADE_FOR / 2);
    assert_eq!(view.sim.steps(), stepped, "nothing steps");
    let list = drawn(&view);
    assert_eq!(black_over(&list), Some(128));
    assert_eq!(
        list.iter().take(pod.len()).collect::<Vec<_>>(),
        pod.iter().collect::<Vec<_>>()
    );
}

#[test]
fn the_rescue_is_laid_out_only_once_the_fade_out_is_over() {
    let mut view = in_the_pod();
    rescue(&mut view);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    ticks(&mut view, RESCUE_FADE_FOR.saturating_sub(ms(1)));
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    ticks(&mut view, ms(1));
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(black_over(&drawn(&view)), Some(255), "black");
}

#[test]
fn the_rescues_desc_is_given_after_two_seconds_of_fades_once() {
    let mut view = in_the_pod();
    rescue(&mut view);
    ticks(&mut view, (2 * RESCUE_FADE_FOR).saturating_sub(ms(1)));
    assert_eq!(view.take_rescue_desc(), None);
    ticks(&mut view, ms(1));
    assert_eq!(view.take_rescue_desc(), Some(rescued(13999)));
    assert_eq!(view.take_rescue_desc(), None, "once");
    assert_eq!(black_over(&drawn(&view)), Some(255), "over black");
}

#[test]
fn the_rescues_texts_wait_a_second_after_its_desc_closes() {
    let mut view = at_the_desc(vec![briefed()]);
    assert!(view.take_rescue_desc().is_some());
    assert_eq!(view.take_mission_texts(), []);
    ticks(&mut view, RESCUE_FADE_FOR.saturating_sub(ms(1)));
    assert_eq!(view.take_mission_texts(), [], "the second fade");
    assert_eq!(black_over(&drawn(&view)), Some(255));
    ticks(&mut view, ms(1));
    assert_eq!(view.take_mission_texts(), [briefed()]);
}

#[test]
fn the_screen_is_faded_from_the_second_fade_until_a_text_fades_it_in() {
    let mut view = at_the_desc(vec![briefed()]);
    assert!(!view.screen_faded(), "the desc shows unfaded");
    view.tick(TICK);
    assert!(view.screen_faded(), "the second fade");
    ticks(&mut view, RESCUE_FADE_FOR);
    assert!(view.screen_faded(), "under the texts");
    view.screen_faded_in();
    assert!(!view.screen_faded());
}

#[test]
fn under_the_texts_the_pods_system_and_bar_are_repainted() {
    let mut view = in_the_pod();
    let mut bar = DrawList::new();
    view.draw_bar_or_why(&mut bar);
    rescue_with(&mut view, vec![briefed()]);
    ticks(&mut view, 2 * RESCUE_FADE_FOR);
    ticks(&mut view, RESCUE_FADE_FOR);
    assert_eq!(black_over(&drawn(&view)), Some(255), "faded");
    let mut rescued_bar = DrawList::new();
    view.draw_bar_or_why(&mut rescued_bar);
    assert_ne!(rescued_bar, bar, "the radar shows the rescue's system");
    view.screen_faded_in();
    let list = drawn(&view);
    let mut expected = DrawList::new();
    crate::draw::fill_rect(
        &mut expected,
        crate::fade::whole_view(VIEW_SIZE),
        Color::BLACK,
    );
    expected.push(space_painted(&view, POD_BACKGROUND));
    expected.extend_translated(&bar, Point::new(0.0, 0.0));
    assert_eq!(list, expected, "empty, in the pod's colour, the pod's bar");
}

#[test]
fn under_an_abort_text_the_pods_message_line_is_repainted_then_none_shows() {
    let mut view = in_the_pod();
    view.say_for("Hello", 600);
    view.tick(TICK);
    rescue_with(&mut view, vec![briefed()]);
    assert_eq!(view.message(), None, "_ResetPlayer (@0x1d904)");
    let list = under_the_first_text(&mut view);
    let text = first(&list, |command| *command == message_drawn("Hello"));
    let space = first(&list, |command| {
        *command == space_painted(&view, POD_BACKGROUND)
    });
    assert!(
        space < text,
        "_RefreshSAT's _RedrawComm (@0x6daf) over the space"
    );
}

#[test]
fn neither_the_closing_nor_flight_after_it_shows_the_pods_message() {
    let mut view = in_the_pod();
    view.say_for("Hello", 600);
    view.tick(TICK);
    rescue(&mut view);
    ticks(&mut view, 3 * RESCUE_FADE_FOR);
    ticks(&mut view, RESCUE_FADE_FOR / 2);
    assert!(view.rescue.is_some(), "the closing fade");
    assert!(!texts(&drawn(&view)).contains(&"Hello".to_owned()));
    ticks(&mut view, RESCUE_FADE_FOR);
    assert!(view.rescue.is_none(), "flying on");
    assert!(!texts(&drawn(&view)).contains(&"Hello".to_owned()));
}

#[test]
fn the_escort_menu_up_in_the_pod_is_repainted_and_its_floating_map_is_not() {
    let mut view = in_the_pod();
    let mut rows = view.frame.hud.escort_rows;
    rows[0].present = true;
    view.escort_menu.toggle(view.elapsed, &rows);
    view.sim.queue_screen(floating_shown(sol_drawing(), None));
    view.hyperspace_key(HYPERSPACE_MODE_KEY);
    view.tick(TICK);
    assert!(
        !floating_border(&drawn(&view)).is_empty(),
        "shown in the pod"
    );
    rescue_with(&mut view, vec![briefed()]);
    let list = under_the_first_text(&mut view);
    assert!(
        list.iter().any(escort_menu_top),
        "_FloaterDrawCallback (@0x38c0f)"
    );
    assert!(
        floating_border(&list).is_empty(),
        "its 150 ticks run out under the rescue's fades (@0x38ba0)"
    );
}

#[test]
fn a_first_text_the_on_purchase_queued_shows_over_the_pods_system_and_the_reset_bar() {
    let mut view = in_the_pod();
    view.tick(TICK);
    let marks = RescueTexts { reset: 2, moved: 3 };
    rescue_noting(&mut view, vec![briefed_third()], Some(marks));
    let list = under_the_first_text(&mut view);
    let mut expected = black(255);
    expected.push(space_painted(&view, POD_BACKGROUND));
    // The rescued frame's bar, but the pod's ships and stellars about the
    // pod on the radar: deleted only after the `OnPurchase` (@0x68f69).
    let reset = FlightFrame {
        ships: sol().ships,
        radar_stellars: sol().radar_stellars,
        ..rescued_frame()
    };
    expected.extend_translated(&bar_noted_over(reset), Point::new(0.0, 0.0));
    assert_eq!(
        list, expected,
        "the pod's radar, the text's gauges and nav (@0x5af96-0x5b038)"
    );
}

#[test]
fn a_first_text_the_days_queued_shows_over_the_new_system_and_its_bar() {
    let mut view = in_the_pod();
    view.tick(TICK);
    let marks = RescueTexts { reset: 1, moved: 2 };
    rescue_noting(&mut view, vec![briefed_third()], Some(marks));
    let list = under_the_first_text(&mut view);
    let mut expected = black(255);
    expected.push(space_painted(&view, NEW_BACKGROUND));
    expected.extend_translated(&bar_noted_over(rescued_frame()), Point::new(0.0, 0.0));
    assert_eq!(list, expected, "after the move (@0x6912a)");
}

#[test]
fn still_faded_the_rescue_fades_in_onto_the_new_system_then_flies_on() {
    let mut view = under_the_texts(Vec::new());
    let stepped = view.sim.steps();
    ticks(&mut view, RESCUE_FADE_FOR / 2);
    let list = drawn(&view);
    assert_eq!(black_over(&list), Some(128), "the closing fade");
    assert!(
        list.iter()
            .any(|c| *c == space_painted(&view, NEW_BACKGROUND))
    );
    assert!(view.screen_faded());
    ticks(&mut view, RESCUE_FADE_FOR / 2);
    assert!(!view.screen_faded());
    assert_eq!(view.sim.steps(), stepped, "the rescue's last tick");
    view.tick(TICK);
    assert_eq!(view.sim.steps(), stepped + 1, "flying on");
}

#[test]
fn once_a_text_faded_in_the_rescue_ends_with_no_closing_fade() {
    let mut view = under_the_texts(vec![briefed()]);
    assert_eq!(view.take_mission_texts(), [briefed()]);
    view.screen_faded_in();
    let stepped = view.sim.steps();
    view.tick(TICK);
    assert_eq!(
        view.sim.steps(),
        stepped,
        "the text's close: the rescue ends"
    );
    assert_eq!(black_over(&drawn(&view)), None, "no fade");
    view.tick(TICK);
    assert_eq!(view.sim.steps(), stepped + 1, "flying on");
}

#[test]
fn the_pointer_shows_from_the_rescues_desc_to_its_end() {
    let mut view = in_the_pod();
    assert!(view.hides_pointer(), "the cursor sprite in the pod");
    rescue(&mut view);
    ticks(&mut view, (2 * RESCUE_FADE_FOR).saturating_sub(ms(1)));
    assert!(view.hides_pointer(), "under the fade and the black");
    ticks(&mut view, ms(1));
    assert!(!view.hides_pointer(), "_ShowTheCursor (@0x68e3c)");
    view.tick(TICK);
    ticks(&mut view, RESCUE_FADE_FOR);
    assert!(!view.hides_pointer(), "under the texts");
    view.tick(TICK);
    assert!(!view.hides_pointer(), "the closing fade");
    ticks(&mut view, RESCUE_FADE_FOR);
    assert!(view.hides_pointer(), "flying on");
}

#[test]
fn a_press_under_the_fade_out_reaches_the_desc_not_flight() {
    let mut view = in_the_pod();
    rescue(&mut view);
    let sent_before = sent(&view).len();
    view.input(&key(LAND_KEY, true));
    assert_eq!(sent(&view).len(), sent_before, "flight takes nothing");
    ticks(&mut view, (2 * RESCUE_FADE_FOR).saturating_sub(ms(1)));
    assert_eq!(view.take_rescue_replay(), []);
    ticks(&mut view, ms(1));
    assert_eq!(view.take_rescue_replay(), [key(LAND_KEY, true)]);
    assert_eq!(view.take_rescue_replay(), [], "once");
}

#[test]
fn a_press_under_the_closing_fade_reaches_flight_once_it_ends() {
    let mut view = under_the_texts(Vec::new());
    view.tick(TICK);
    view.input(&key(LAND_KEY, true));
    assert_eq!(view.take_rescue_replay(), []);
    ticks(&mut view, RESCUE_FADE_FOR);
    assert_eq!(view.take_rescue_replay(), [key(LAND_KEY, true)]);
}

#[test]
fn the_first_step_after_the_rescue_holds_the_gain() {
    let mut view = under_the_texts(Vec::new());
    ticks(&mut view, RESCUE_FADE_FOR);
    view.tick(TICK);
    let input = view.sim.inputs().last().copied().expect("a step");
    assert!(input.held, "the seconds without a step hold the gain");
}

#[test]
fn the_closing_redraws_the_whole_bar_from_the_rescued_frame() {
    let mut view = under_the_texts(Vec::new());
    let mut changed = view.frame.hud.reserves;
    changed.fuel.now += 100.0;
    changed.fuel.max += 100.0;
    view.change_sim(|sim| sim.frame_mut().hud.reserves = changed);
    assert_ne!(view.reserves_scene, Some(changed), "not yet redrawn");
    view.tick(TICK);
    assert_eq!(
        view.reserves_scene,
        Some(changed),
        "_RedrawStatus(1) (@0x69219)"
    );
}

#[test]
fn the_rescues_black_covers_the_texts_while_the_screen_is_faded() {
    let mut view = under_the_texts(vec![briefed()]);
    let mut cover = DrawList::new();
    view.draw_faded_cover(&mut cover, VIEW_SIZE);
    assert_eq!(cover, black(255));
    view.screen_faded_in();
    let mut cover = DrawList::new();
    view.draw_faded_cover(&mut cover, VIEW_SIZE);
    assert!(cover.is_empty(), "faded in, and no jump");
}

/// Solid black over the whole of [`VIEW_SIZE`].
fn black(alpha: u8) -> DrawList {
    let mut list = DrawList::new();
    crate::draw::fill_rect(
        &mut list,
        crate::fade::whole_view(VIEW_SIZE),
        Color {
            a: alpha,
            ..Color::BLACK
        },
    );
    list
}
