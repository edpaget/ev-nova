use super::*;

// Death.

/// Every NPC idles and holds its trigger.
#[derive(Debug)]
struct Firing;

impl Behaviour for Firing {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }

    fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> nova_sim::Trigger {
        nova_sim::Trigger {
            primary: true,
            secondary: None,
            only: None,
            turrets_only: false,
            bays: false,
            point_defence: false,
            keep: None,
        }
    }
}

/// Disables no ship: the player breaks up once its armour is gone.
#[derive(Debug)]
struct Never;

impl DisableRule for Never {
    fn disabled(&self, _armor: Gauge, _hull: &HullSpec) -> bool {
        false
    }
}

/// The first `chär` flies ship 128 ("First": no shield, 10 armour, a
/// `DeathDelay` of 30, so it breaks up for 88 ticks) from system 128,
/// over the planet Alpha Prime (128) at (0, 0) with a bar. The
/// system's one `düde`, at 100 %, flies ship 129 ("Killer": 100
/// armour and no speed, so it stays where setup places it), carrying
/// one `wëap` 128: unguided, every 5 ticks, 20 pixels a tick for 30,
/// 1000 mass damage through shields, so one shot breaks the player up.
fn deadly_data() -> Rc<GameData> {
    deadly_data_with(&[])
}

/// A sabotage outfit: its `oütf` ID, the `dësc` its `ModType` 47
/// slot names, and that `dësc`'s text, if it has one.
type Saboteur<'a> = (i16, i16, Option<&'a str>);

/// The sabotage `dësc` that shows a picture, and its `Graphic`: the
/// others show none.
const PICTURED_SABOTAGE: (i16, i16) = (5000, 300);

/// [`deadly_data`], the player's ship carrying one of each of
/// `saboteurs` as its default items, their `dësc`s pictured as
/// [`PICTURED_SABOTAGE`] says.
fn deadly_data_with(saboteurs: &[Saboteur]) -> Rc<GameData> {
    let items: Vec<(i16, [i16; 3])> = saboteurs
        .iter()
        .map(|&(outfit, desc, _)| (outfit, [nova_sim::SABOTAGE_KILL, desc, 1]))
        .collect();
    let fork = saboteurs
        .iter()
        .fold(deadly_fork(&items), |fork, &(_, desc, text)| match text {
            Some(text) => {
                let graphic = if desc == PICTURED_SABOTAGE.0 {
                    PICTURED_SABOTAGE.1
                } else {
                    -1
                };
                fork.resource(Desc::TYPE, desc, None, &pictured_desc_of(text, graphic))
            }
            None => fork,
        });
    loaded(&fork)
}

/// [`deadly_data`]'s fork, the player's ship carrying one of each of
/// `items` as its default items: each an `oütf` ID with its first
/// `ModType`, `ModVal` and the field after them.
fn deadly_fork(items: &[(i16, [i16; 3])]) -> ForkBuilder {
    let put = |bytes: &mut [u8], at: usize, values: &[i16]| {
        for (i, value) in values.iter().enumerate() {
            bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
        }
    };
    let ship = |armor, weapon| {
        let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
        put(&mut bytes, 0x00, &[0, 0, 300, 300, 10, 300, 0, armor]);
        put(&mut bytes, 0x12, &[weapon, -1, -1, -1]);
        put(&mut bytes, 0x1A, &[1, 0, 0, 0]);
        put(&mut bytes, 0x42, &[1, 10]);
        put(&mut bytes, 0x4E, &[-1; 4]);
        put(&mut bytes, 0x370, &[-1; 4]);
        put(&mut bytes, 0x6CE, &[-1; 4]);
        bytes
    };
    let mut player = ship(10, -1);
    put(&mut player, 0x34, &[30]);
    for (slot, &(outfit, _)) in items.iter().enumerate() {
        put(&mut player, 0x4E + 2 * slot, &[outfit]);
        put(&mut player, 0x56 + 2 * slot, &[1]);
    }
    let mut killer = ship(100, 128);
    put(&mut killer, 0x06, &[0]);
    let mut weapon = vec![0; Weapon::SIZE.expect("fixed")];
    put(&mut weapon, 0x00, &[5, 30, 1000, 0, -1, 2000, -1, 0, 0, -1]);
    put(&mut weapon, 0x16, &[-1]);
    put(&mut weapon, 0x1C, &[0x0020]);
    let mut anim = vec![0; ShipAnim::SIZE.expect("fixed")];
    put(&mut anim, 0x00, &[1000, 0, 1]);
    put(&mut anim, 0x34, &[4]);
    let sheet = (0..4)
        .fold(RledBuilder::new(1, 1), |sheet, _| {
            sheet.frame(|f| f.line().pixels(&[0x7C00]))
        })
        .build();
    let mut character = vec![0; Character::SIZE.expect("fixed")];
    put(&mut character, 0x04, &[128, 128, -1, -1, -1]);
    put(&mut character, 0x0E, &[-1; 4]);
    let mut system = system(0, &[128]);
    put(&mut system, 0x44, &[128, -1, -1, -1, -1, -1, -1, -1]);
    put(&mut system, 0x54, &[100]);
    put(&mut system, 0x64, &[1]);
    let mut dude = vec![0; Dude::SIZE.expect("fixed")];
    put(&mut dude, 0x00, &[1, -1]);
    put(&mut dude, 0x08, &[-1; 16]);
    put(&mut dude, 0x08, &[129]);
    put(&mut dude, 0x28, &[100]);
    let mut spin = vec![0; Spin::SIZE.expect("fixed")];
    spin[0..2].copy_from_slice(&1000_i16.to_be_bytes());
    let fork = items.iter().fold(
        with_main_screen(ForkBuilder::new()),
        |fork, &(outfit, mods)| {
            let mut record = vec![0; Outfit::SIZE.expect("fixed")];
            put(&mut record, 0x06, &mods);
            fork.resource(Outfit::TYPE, outfit, Some(b"Bomb"), &record)
        },
    );
    fork.resource(Ship::TYPE, 128, Some(b"First"), &player)
        .resource(Ship::TYPE, 129, Some(b"Killer"), &killer)
        .resource(Weapon::TYPE, 128, Some(b"Blaster"), &weapon)
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character)
        .resource(ShipAnim::TYPE, 128, None, &anim)
        .resource(ShipAnim::TYPE, 129, None, &anim)
        .resource(RLED, 1000, None, &sheet)
        .resource(System::TYPE, 128, Some(b"Alpha"), &system)
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &landable())
        .resource(Spin::TYPE, 1000, None, &spin)
        .resource(Dude::TYPE, 128, Some(b"Killers"), &dude)
}

/// The rescue's `dësc` text in [`pod_data`].
const RESCUE_TEXT: &str = "After several weeks of sensory deprivation";

/// The rescue's `dësc`'s `Graphic` in [`pod_data`].
const RESCUE_GRAPHIC: i16 = 400;

/// [`deadly_data`] where the player's ship carries an escape pod
/// (`oütf` 200, `ModType` 11) and auto-eject (201, `ModType` 20), with
/// the pod's class (`shïp` 895) and, when `desc` gives its text, the
/// rescue's `dësc` 13999.
fn escape_pod_data(desc: Option<&str>) -> Rc<GameData> {
    let fork = escape_pod_fork();
    match desc {
        Some(text) => loaded(&fork.resource(
            Desc::TYPE,
            13999,
            None,
            &pictured_desc_of(text, RESCUE_GRAPHIC),
        )),
        None => loaded(&fork),
    }
}

/// [`escape_pod_data`]'s fork, with no rescue `dësc`.
fn escape_pod_fork() -> ForkBuilder {
    let put = |bytes: &mut [u8], at: usize, values: &[i16]| {
        for (i, value) in values.iter().enumerate() {
            bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
        }
    };
    let mut pod = vec![0; Ship::SIZE.expect("fixed")];
    put(&mut pod, 0x00, &[0, 0, 300, 300, 10, 300, 0, 10]);
    put(&mut pod, 0x12, &[-1; 4]);
    put(&mut pod, 0x4E, &[-1; 4]);
    put(&mut pod, 0x370, &[-1; 4]);
    put(&mut pod, 0x6CE, &[-1; 4]);
    deadly_fork(&[(200, [11, 0, 1]), (201, [20, 0, 1])]).resource(
        Ship::TYPE,
        895,
        Some(b"Escape Pod"),
        &pod,
    )
}

/// The router over [`escape_pod_data`] (with its `dësc` of the text
/// `desc` gives, if any), flying with the killer above the player,
/// until the player is in the escape pod.
fn ejected(store: &MemoryPilots, desc: Option<&str>) -> AppScreen {
    ejected_by(store, desc, create)
}

/// [`ejected`], the pilot made by `create`.
fn ejected_by(
    store: &MemoryPilots,
    desc: Option<&str>,
    create: fn(&mut AppScreen, &str),
) -> AppScreen {
    ejected_from(store, with_dialogs(escape_pod_data(desc)), create)
}

/// `screen`, over [`escape_pod_data`] or its like, flying with the
/// killer above the player, the pilot made by `create`, until the
/// player is in the escape pod.
fn ejected_from(
    store: &MemoryPilots,
    screen: AppScreen,
    create: fn(&mut AppScreen, &str),
) -> AppScreen {
    let mut screen = screen
        .with_chance(above_the_player())
        .with_behaviour(Rc::new(Firing))
        .with_disable_rule(Rc::new(Never))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics));
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    screen.input(&key(Key::Escape, true));
    tick_until(&mut screen, 240, |screen| {
        matches!(fate(screen), nova_sim::PlayerFate::Ejected { .. })
    });
    screen
}

#[test]
fn a_rescue_desc_that_cannot_be_read_is_skipped_with_a_warning() {
    let store = MemoryPilots::new();
    let mut screen = ejected(&store, None);
    tick_until(&mut screen, nova_sim::POD_TICKS + 1, |screen| {
        fate(screen) == nova_sim::PlayerFate::Flying
    });
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(
        screen.take_warnings(),
        ["nova: cannot show the rescue dësc 13999: no dësc 13999"]
    );
}

#[test]
fn the_rescue_shows_its_desc_over_flight_and_stops_stepping() {
    let store = MemoryPilots::new();
    let mut screen = ejected(&store, Some(RESCUE_TEXT));
    let ticks = tick_until(&mut screen, nova_sim::POD_TICKS + 1, |screen| {
        screen.showing() == Showing::MissionText
    });
    assert_eq!(ticks, nova_sim::POD_TICKS, "on the countdown's last tick");
    assert_eq!(shown_text(&screen).as_deref(), Some(RESCUE_TEXT));
    assert_eq!(
        shown_picture(&screen),
        Some((300.0, Some(RESCUE_GRAPHIC))),
        "DLOG 3004, its graphic shown"
    );
    assert_eq!(fate(&screen), nova_sim::PlayerFate::Flying);
    let at = ship(&screen);
    screen.tick(Duration::from_secs(2));
    assert_eq!(ship(&screen), at, "nothing moves under the dialog");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight, "flying on");
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
}

#[test]
fn the_rescues_desc_plays_its_movie_over_flight() {
    let store = MemoryPilots::new();
    let data = loaded(&escape_pod_fork().resource(
        Desc::TYPE,
        13999,
        None,
        &filmed_desc_of(RESCUE_TEXT, "Rescue.mov", 0),
    ));
    let reel = Rc::new(Reel::default());
    let screen = with_dialogs(data).with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>);
    let mut screen = ejected_from(&store, screen, create);
    assert!(reeled(&reel).is_empty());
    tick_until(&mut screen, nova_sim::POD_TICKS + 1, |screen| {
        screen.showing() != Showing::Flight
    });
    assert_eq!(screen.showing(), Showing::Movie, "before its text");
    assert_eq!(reeled(&reel), ["start Rescue.mov"]);
    assert_eq!(fate(&screen), nova_sim::PlayerFate::Flying);
    tick_until(&mut screen, 10, |screen| {
        screen.showing() == Showing::MissionText
    });
    assert_eq!(reeled(&reel), ["start Rescue.mov", "stop"]);
    assert_eq!(shown_text(&screen).as_deref(), Some(RESCUE_TEXT));
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight, "flying on");
}

#[test]
fn the_rescue_desc_reads_its_conditional_text_through_flight() {
    let store = MemoryPilots::new();
    let mut screen = ejected(&store, Some("Rescued, {G \"sir\" \"ma'am\"}."));
    tick_until(&mut screen, nova_sim::POD_TICKS + 1, |screen| {
        screen.showing() == Showing::MissionText
    });
    assert_eq!(
        shown_text(&screen).as_deref(),
        Some("Rescued, sir."),
        "loaded as _LoadDescription loads it (@0x68e90)"
    );
}

#[test]
fn a_strict_pilot_rescued_from_the_pod_is_saved_before_the_rescue_is_drawn() {
    let store = MemoryPilots::new();
    let mut screen = ejected_by(&store, Some(RESCUE_TEXT), create_strict);
    let before = store.writes();
    tick_until(&mut screen, nova_sim::POD_TICKS + 1, |screen| {
        screen.showing() == Showing::MissionText
    });
    assert_eq!(store.writes(), before + 1, "saved on the rescue's tick");
    let saved = saved(&store, "Ada");
    assert_eq!(saved.stellar(), Some(nova_sim::StellarId(128)));
    assert_eq!(saved.system(), nova_sim::SystemId(128));
}

#[test]
fn a_pilot_rescued_from_the_pod_without_strict_play_is_not_saved() {
    let store = MemoryPilots::new();
    let mut screen = ejected_by(&store, Some(RESCUE_TEXT), create);
    let before = store.writes();
    tick_until(&mut screen, nova_sim::POD_TICKS + 1, |screen| {
        screen.showing() == Showing::MissionText
    });
    assert_eq!(store.writes(), before, "no save on the rescue's tick");
}

/// Draws the last outcome of every roll but the traffic's placement:
/// its two 1500-wide draws place a ship at (0, -100), 100 pixels
/// above the player at the centre, and its heading draw faces it down
/// at the player. However many rolls landing and taking off make
/// first, the killer is placed there.
#[derive(Default)]
struct AboveThePlayer {
    placed: usize,
}

impl Chance for AboveThePlayer {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        match n {
            1500 => {
                self.placed += 1;
                if self.placed % 2 == 1 { 750 } else { 650 }
            }
            360 => 180,
            _ => n - 1,
        }
    }
}

fn above_the_player() -> SharedChance {
    let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(AboveThePlayer::default()));
    SharedChance::new(shared)
}

/// The router over [`deadly_data`] with the main menu over `store`,
/// dialogs, the killer placed above the player and firing, and no ship
/// disabled.
fn deadly_menu(store: &MemoryPilots) -> AppScreen {
    with_dialogs(deadly_data())
        .with_chance(above_the_player())
        .with_behaviour(Rc::new(Firing))
        .with_disable_rule(Rc::new(Never))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
}

fn fate(screen: &AppScreen) -> nova_sim::PlayerFate {
    flight(screen).session().expect("flying").player_fate()
}

/// Ticks `screen` one sim tick at a time until `done`, at most `limit`
/// ticks, and says how many it took.
fn tick_until(screen: &mut AppScreen, limit: u32, done: impl Fn(&AppScreen) -> bool) -> u32 {
    for n in 1..=limit {
        screen.tick(TICK);
        if done(screen) {
            return n;
        }
    }
    panic!("not within {limit} ticks: {:?}", screen.showing())
}

/// Ticks `screen` until the player's ship breaks up.
fn until_breaking_up(screen: &mut AppScreen) {
    tick_until(screen, 120, |screen| {
        matches!(fate(screen), nova_sim::PlayerFate::BreakingUp { .. })
    });
}

/// Ticks `screen` until the player's ship has exploded.
pub(super) fn until_exploded(screen: &mut AppScreen) {
    tick_until(screen, 240, |screen| {
        fate(screen) == nova_sim::PlayerFate::Exploded
    });
}

/// The alpha of the black covering the whole screen, if the last
/// thing `screen` draws is one.
#[allow(clippy::float_cmp)] // The screen's height, exactly as drawn.
fn black_over(screen: &AppScreen) -> Option<u8> {
    match drawn(screen).iter().last() {
        Some(DrawCommand::Line { width, color, .. })
            if *width == 768.0 && (color.r, color.g, color.b) == (0, 0, 0) =>
        {
            Some(color.a)
        }
        _ => None,
    }
}

/// A pilot, "Ada", created on [`deadly_menu`] over `store`, landed and
/// taken off again, before the killer is in the system.
fn landed_and_took_off(store: &MemoryPilots) -> AppScreen {
    landed_and_took_off_by(store, create)
}

/// [`landed_and_took_off`] with a Strict Play pilot.
fn strict_landed_and_took_off(store: &MemoryPilots) -> AppScreen {
    landed_and_took_off_by(store, create_strict)
}

/// [`landed_and_took_off`], the pilot made by `create`.
fn landed_and_took_off_by(store: &MemoryPilots, create: fn(&mut AppScreen, &str)) -> AppScreen {
    let mut screen = deadly_menu(store);
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight, "taken off");
    screen
}

/// [`landed_and_took_off`], then the killer breaks its ship up.
pub(super) fn landed_took_off_and_breaking_up(store: &MemoryPilots) -> AppScreen {
    let mut screen = landed_and_took_off(store);
    until_breaking_up(&mut screen);
    screen
}

#[test]
fn a_pilot_that_landed_took_off_and_died_is_saved_by_the_landing_and_take_off_only() {
    let store = MemoryPilots::new();
    let mut screen = landed_and_took_off(&store);
    let took_off = pilot(&screen).clone();
    until_exploded(&mut screen);
    tick_until(&mut screen, 400, |screen| {
        screen.showing() == Showing::MainMenu
    });
    let written = store.written();
    assert_eq!(written.len(), 3, "creation, landing, take-off: {written:?}");
    assert!(written.iter().all(|(key, _)| key == "Ada"));
    let last = nova_sim::save::decode(&written[2].1).expect("a pilot");
    assert_eq!(last, took_off, "the take-off's save is the last");
    assert_eq!(store.deleted(), Vec::<String>::new(), "not strict: kept");
    assert_eq!(store.keys(), ["Ada"]);
    let mut screen = deadly_menu(&store);
    choose(&mut screen, MenuChoice::OpenPilot);
    screen.input(&key(Key::Enter, true));
    assert_eq!(pilot(&screen), &last, "reopened as at the take-off");
}

#[test]
fn a_dead_pilot_holds_fades_and_returns_to_the_main_menu() {
    let store = MemoryPilots::new();
    let mut screen = landed_took_off_and_breaking_up(&store);
    until_exploded(&mut screen);
    for tick in 1..=nova_sim::HOLD_TICKS {
        screen.tick(TICK);
        assert_eq!(screen.showing(), Showing::Flight, "dead tick {tick}");
        assert_eq!(black_over(&screen), None, "dead tick {tick}");
    }
    screen.tick(TICK);
    assert_eq!(
        fate(&screen),
        nova_sim::PlayerFate::Fading,
        "on the expiry's tick, straight through the seam"
    );
    assert_eq!(screen.showing(), Showing::Flight, "no dialog");
    screen.tick(Duration::from_millis(500));
    assert_eq!(black_over(&screen), Some(128));
    assert_eq!(screen.showing(), Showing::Flight);
    screen.tick(Duration::from_millis(500));
    assert_eq!(screen.showing(), Showing::MainMenu, "back to the menu");
    assert_eq!(black_over(&screen), Some(255), "from black");
    screen.tick(Duration::from_millis(500));
    assert_eq!(black_over(&screen), Some(128), "fading in");
    screen.tick(Duration::from_millis(500));
    assert_eq!(black_over(&screen), None, "faded in");
    assert_eq!(
        choose(&mut screen, MenuChoice::NewPilot),
        ScreenAction::None
    );
    assert_eq!(screen.showing(), Showing::NewPilot, "the menu takes input");
}

#[test]
fn the_main_menu_takes_input_under_its_fade_in() {
    let store = MemoryPilots::new();
    let mut screen = landed_took_off_and_breaking_up(&store);
    until_exploded(&mut screen);
    tick_until(&mut screen, 400, |screen| {
        screen.showing() == Showing::MainMenu
    });
    assert!(black_over(&screen).is_some());
    assert_eq!(
        choose(&mut screen, MenuChoice::NewPilot),
        ScreenAction::None
    );
    assert_eq!(screen.showing(), Showing::NewPilot);
}

#[test]
fn after_the_return_no_flight_is_left_to_re_enter() {
    let store = MemoryPilots::new();
    let mut screen = landed_took_off_and_breaking_up(&store);
    until_exploded(&mut screen);
    tick_until(&mut screen, 400, |screen| {
        screen.showing() == Showing::MainMenu
    });
    assert!(screen.flight_view().is_none());
    assert!(screen.spaceport_view().is_none());
    screen.quit();
    assert_eq!(store.writes(), 3, "nothing left to save");
}

#[test]
fn without_a_main_menu_a_dead_pilot_returns_to_the_side_flight_was_entered_from() {
    let mut screen = with_dialogs(deadly_data())
        .with_chance(above_the_player())
        .with_behaviour(Rc::new(Firing))
        .with_disable_rule(Rc::new(Never));
    fly(&mut screen);
    until_exploded(&mut screen);
    tick_until(&mut screen, 400, |screen| {
        screen.showing() != Showing::Flight
    });
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    assert!(screen.flight_view().is_none());
    assert_eq!(black_over(&screen), Some(255), "from black");
}

#[test]
fn escape_during_the_breakup_writes_no_save_and_drops_flight() {
    let store = MemoryPilots::new();
    let mut screen = landed_took_off_and_breaking_up(&store);
    let writes = store.writes();
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(store.writes(), writes, "no save");
    assert!(screen.flight_view().is_none(), "flight dropped");
    assert!(screen.spaceport_view().is_none());
}

#[test]
fn quitting_during_the_breakup_or_the_hold_writes_no_save() {
    let store = MemoryPilots::new();
    let mut screen = landed_took_off_and_breaking_up(&store);
    let writes = store.writes();
    screen.quit();
    assert_eq!(store.writes(), writes, "breaking up");
    until_exploded(&mut screen);
    screen.tick(TICK);
    assert!(matches!(
        fate(&screen),
        nova_sim::PlayerFate::Holding { .. }
    ));
    screen.quit();
    assert_eq!(store.writes(), writes, "holding");
}

#[test]
fn a_save_due_while_dying_is_not_written() {
    use nova_view::devtools::{PilotDesk, PilotEdit};
    let store = MemoryPilots::new();
    let mut screen = landed_took_off_and_breaking_up(&store);
    let writes = store.writes();
    screen
        .pilot_desk()
        .expect("flying")
        .edit(PilotEdit::Credits(5000))
        .expect("edits in flight");
    screen.tick(TICK);
    assert_eq!(store.writes(), writes, "due, but not written");
    screen.after_pilot_edit();
    assert_eq!(store.writes(), writes, "nor after the edit");
}

/// The strict pilot's death, ticked from the explosion to the main
/// menu.
fn until_the_main_menu(screen: &mut AppScreen) {
    tick_until(screen, 400, |screen| screen.showing() == Showing::MainMenu);
}

#[test]
fn a_strict_pilot_destroyed_is_deleted_before_the_main_menu_shows() {
    let store = MemoryPilots::new();
    let mut screen = strict_landed_and_took_off(&store);
    assert!(pilot(&screen).strict_play());
    let writes = store.writes();
    until_exploded(&mut screen);
    tick_until(&mut screen, 400, |screen| {
        let shown = screen.showing() == Showing::MainMenu;
        assert_eq!(
            store.deleted().is_empty(),
            !shown,
            "deleted as the menu shows, not before"
        );
        shown
    });
    assert_eq!(store.deleted(), ["Ada"]);
    assert_eq!(store.keys(), Vec::<String>::new());
    assert_eq!(store.writes(), writes, "nothing written after the take-off");
    choose(&mut screen, MenuChoice::OpenPilot);
    assert_eq!(
        screen.pilot_list().expect("open").keys(),
        Vec::<String>::new()
    );
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
}

#[test]
fn escape_while_a_strict_pilot_breaks_up_deletes_it() {
    let store = MemoryPilots::new();
    let mut screen = strict_landed_and_took_off(&store);
    until_breaking_up(&mut screen);
    let writes = store.writes();
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(store.deleted(), ["Ada"]);
    assert_eq!(store.keys(), Vec::<String>::new());
    screen.quit();
    assert_eq!(store.writes(), writes, "nothing written");
    assert_eq!(store.keys(), Vec::<String>::new(), "nor written back");
}

#[test]
fn quitting_while_a_strict_pilot_breaks_up_or_holds_deletes_it() {
    let store = MemoryPilots::new();
    let mut screen = strict_landed_and_took_off(&store);
    until_breaking_up(&mut screen);
    let writes = store.writes();
    screen.quit();
    assert_eq!(store.deleted(), ["Ada"], "breaking up");
    assert_eq!(store.keys(), Vec::<String>::new());
    assert_eq!(store.writes(), writes);

    let store = MemoryPilots::new();
    let mut screen = strict_landed_and_took_off(&store);
    let writes = store.writes();
    until_exploded(&mut screen);
    screen.tick(TICK);
    assert!(matches!(
        fate(&screen),
        nova_sim::PlayerFate::Holding { .. }
    ));
    screen.quit();
    assert_eq!(store.deleted(), ["Ada"], "holding");
    assert_eq!(store.keys(), Vec::<String>::new());
    assert_eq!(store.writes(), writes);
}

#[test]
fn a_save_due_while_a_strict_pilot_dies_writes_nothing_after_the_delete() {
    use nova_view::devtools::{PilotDesk, PilotEdit};
    let store = MemoryPilots::new();
    let mut screen = strict_landed_and_took_off(&store);
    until_breaking_up(&mut screen);
    let writes = store.writes();
    screen
        .pilot_desk()
        .expect("flying")
        .edit(PilotEdit::Credits(5000))
        .expect("edits in flight");
    screen.tick(TICK);
    screen.after_pilot_edit();
    until_exploded(&mut screen);
    until_the_main_menu(&mut screen);
    assert_eq!(store.deleted(), ["Ada"]);
    assert_eq!(store.writes(), writes, "nothing written");
    assert_eq!(store.keys(), Vec::<String>::new());
}

#[test]
fn a_strict_pilot_put_away_flying_is_saved_not_deleted() {
    let store = MemoryPilots::new();
    let mut screen = strict_landed_and_took_off(&store);
    let writes = store.writes();
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(store.writes(), writes + 1, "saved");
    assert_eq!(store.deleted(), Vec::<String>::new());
    assert!(saved(&store, "Ada").strict_play());
}

#[test]
fn a_failed_delete_becomes_a_warning() {
    let store = MemoryPilots::new();
    let mut screen = strict_landed_and_took_off(&store);
    store.fail_deletes(true);
    until_exploded(&mut screen);
    until_the_main_menu(&mut screen);
    assert_eq!(
        screen.take_warnings(),
        ["nova: cannot delete the pilot Ada in memory (the disk is locked)"]
    );
    assert_eq!(store.keys(), ["Ada"], "still there");
}

// Sabotage.

/// The text of the mission text shown over flight, if any.
fn shown_text(screen: &AppScreen) -> Option<String> {
    screen
        .flight_mission_text()
        .map(|text| text.lines().join(" "))
}

/// The width of the text dialog shown over flight, and its picture.
fn shown_picture(screen: &AppScreen) -> Option<(f32, Option<i16>)> {
    screen
        .flight_mission_text()
        .map(|text| (text.dialog().bounds().width(), text.picture()))
}

/// [`landed_and_took_off`] over [`deadly_data_with`] `saboteurs`.
fn sabotaged_and_took_off(store: &MemoryPilots, saboteurs: &[Saboteur]) -> AppScreen {
    took_off_over(store, with_dialogs(deadly_data_with(saboteurs)))
}

/// `screen`, over [`deadly_data`] or its like, landed and taken off
/// with the killer above the player.
fn took_off_over(store: &MemoryPilots, screen: AppScreen) -> AppScreen {
    let mut screen = screen
        .with_chance(above_the_player())
        .with_behaviour(Rc::new(Firing))
        .with_disable_rule(Rc::new(Never))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics));
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight, "taken off");
    screen
}

/// Two sabotage outfits, 128 and 129, naming `dësc`s 5000 and 5001.
const TWO_SABOTEURS: [Saboteur; 2] = [
    (128, 5000, Some("Your ship was sabotaged.")),
    (129, 5001, Some("Twice over.")),
];

#[test]
fn a_sabotage_desc_reads_its_conditional_text_through_flight() {
    let store = MemoryPilots::new();
    let mut screen = sabotaged_and_took_off(
        &store,
        &[(128, 5000, Some("Sabotaged, {G \"sir\" \"ma'am\"}."))],
    );
    until_exploded(&mut screen);
    tick_until(&mut screen, nova_sim::HOLD_TICKS + 2, |screen| {
        screen.showing() == Showing::MissionText
    });
    assert_eq!(
        shown_text(&screen).as_deref(),
        Some("Sabotaged, sir."),
        "loaded as _LoadDescription loads it (@0x687c9)"
    );
}

#[test]
fn a_sabotaged_pilot_sees_each_outfits_desc_after_the_hold_and_then_fades() {
    let store = MemoryPilots::new();
    let mut screen = sabotaged_and_took_off(&store, &TWO_SABOTEURS);
    until_exploded(&mut screen);
    for tick in 1..=nova_sim::HOLD_TICKS {
        screen.tick(TICK);
        assert_eq!(screen.showing(), Showing::Flight, "dead tick {tick}");
    }
    screen.tick(TICK);
    assert_eq!(
        screen.showing(),
        Showing::MissionText,
        "on the expiry's tick"
    );
    assert_eq!(
        shown_text(&screen).as_deref(),
        Some("Your ship was sabotaged.")
    );
    assert_eq!(
        shown_picture(&screen),
        Some((300.0, Some(PICTURED_SABOTAGE.1))),
        "DLOG 3004, its graphic shown"
    );
    let held = nova_sim::PlayerFate::Holding { ticks_left: 0 };
    assert_eq!(fate(&screen), held, "not fading under the dialog");
    assert!(flight(&screen).session().expect("flying").hold_expired());
    screen.tick(Duration::from_secs(2));
    assert_eq!(fate(&screen), held, "nothing moves under the dialog");
    assert_eq!(screen.showing(), Showing::MissionText);
    assert_eq!(black_over(&screen), None);
    screen.input(&key(Key::Enter, true));
    assert_eq!(shown_text(&screen).as_deref(), Some("Twice over."));
    assert_eq!(shown_picture(&screen), Some((200.0, None)), "no graphic");
    assert_eq!(fate(&screen), held, "one dialog left");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight, "the last closed");
    assert_eq!(fate(&screen), nova_sim::PlayerFate::Fading);
    screen.tick(Duration::from_millis(500));
    assert_eq!(black_over(&screen), Some(128));
    screen.tick(Duration::from_millis(500));
    assert_eq!(screen.showing(), Showing::MainMenu, "back to the menu");
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
}

#[test]
fn the_sabotage_dialogs_let_go_of_flights_keys() {
    let store = MemoryPilots::new();
    let mut screen = sabotaged_and_took_off(&store, &TWO_SABOTEURS);
    until_exploded(&mut screen);
    for _ in 1..=nova_sim::HOLD_TICKS {
        screen.tick(TICK);
    }
    screen.input(&key(Key::Up, true));
    assert!(flight(&screen).holds_keys(), "held before the expiry");
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::MissionText);
    assert!(!flight(&screen).holds_keys(), "let go");
}

#[test]
fn a_sabotage_desc_that_cannot_be_read_is_skipped_with_a_warning() {
    let store = MemoryPilots::new();
    let mut screen = sabotaged_and_took_off(
        &store,
        &[(128, 5000, None), (129, 5001, Some("Twice over."))],
    );
    until_exploded(&mut screen);
    tick_until(&mut screen, nova_sim::HOLD_TICKS + 1, |screen| {
        screen.showing() != Showing::Flight
    });
    assert_eq!(shown_text(&screen).as_deref(), Some("Twice over."));
    assert_eq!(
        screen.take_warnings(),
        ["nova: cannot show the sabotage dësc 5000: no dësc 5000"]
    );
    screen.input(&key(Key::Enter, true));
    assert_eq!(fate(&screen), nova_sim::PlayerFate::Fading);
}

#[test]
fn with_no_sabotage_desc_to_read_the_fade_begins_on_the_expirys_tick() {
    let store = MemoryPilots::new();
    let mut screen = sabotaged_and_took_off(&store, &[(128, 5000, None)]);
    until_exploded(&mut screen);
    for _ in 0..=nova_sim::HOLD_TICKS {
        screen.tick(TICK);
    }
    assert_eq!(screen.showing(), Showing::Flight, "no dialog");
    assert_eq!(fate(&screen), nova_sim::PlayerFate::Fading);
    assert_eq!(screen.take_warnings().len(), 1);
}

/// The router sabotaged by one outfit, 128, whose `dësc` 5000,
/// "Sabotaged.", names the movie "Sabotage.mov" of `Flags` `flags`,
/// its movies played through a new [`Reel`], taken off and ticked
/// until the player's ship has exploded.
fn filmed_sabotage(store: &MemoryPilots, flags: u16) -> (AppScreen, Rc<Reel>) {
    let data = loaded(
        &deadly_fork(&[(128, [nova_sim::SABOTAGE_KILL, 5000, 1])]).resource(
            Desc::TYPE,
            5000,
            None,
            &filmed_desc_of("Sabotaged.", "Sabotage.mov", flags),
        ),
    );
    let reel = Rc::new(Reel::default());
    let screen = with_dialogs(data).with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>);
    let mut screen = took_off_over(store, screen);
    until_exploded(&mut screen);
    (screen, reel)
}

#[test]
fn a_sabotage_descs_movie_starts_as_the_hold_expires_and_the_fade_waits_for_it_and_its_text() {
    let store = MemoryPilots::new();
    let (mut screen, reel) = filmed_sabotage(&store, 0);
    for _ in 1..=nova_sim::HOLD_TICKS {
        screen.tick(TICK);
    }
    assert!(reeled(&reel).is_empty(), "not before the expiry");
    screen.tick(TICK);
    assert_eq!(
        reeled(&reel),
        ["start Sabotage.mov"],
        "on the expiry's tick"
    );
    assert_eq!(screen.showing(), Showing::Movie, "before its text");
    let held = nova_sim::PlayerFate::Holding { ticks_left: 0 };
    assert_eq!(fate(&screen), held, "not fading under the movie");
    tick_until(&mut screen, 10, |screen| {
        screen.showing() == Showing::MissionText
    });
    assert_eq!(reeled(&reel), ["start Sabotage.mov", "stop"]);
    assert_eq!(shown_text(&screen).as_deref(), Some("Sabotaged."));
    assert_eq!(fate(&screen), held, "nor under the text");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(
        fate(&screen),
        nova_sim::PlayerFate::Fading,
        "once both close"
    );
}

#[test]
fn a_sabotage_descs_after_movie_holds_the_fade_until_the_tick_it_ends() {
    let store = MemoryPilots::new();
    let (mut screen, reel) = filmed_sabotage(&store, 0x0001);
    tick_until(&mut screen, nova_sim::HOLD_TICKS + 2, |screen| {
        screen.showing() != Showing::Flight
    });
    assert_eq!(shown_text(&screen).as_deref(), Some("Sabotaged."));
    assert!(reeled(&reel).is_empty(), "after its text");
    screen.input(&key(Key::Enter, true));
    assert_eq!(reeled(&reel), ["start Sabotage.mov"]);
    assert_eq!(screen.showing(), Showing::Movie);
    let held = nova_sim::PlayerFate::Holding { ticks_left: 0 };
    for _ in 0..10 {
        if reeled(&reel).len() > 1 {
            break;
        }
        assert_eq!(fate(&screen), held, "not while it plays");
        screen.tick(TICK);
    }
    assert_eq!(reeled(&reel), ["start Sabotage.mov", "stop"]);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(
        fate(&screen),
        nova_sim::PlayerFate::Fading,
        "on the tick the movie ended"
    );
}
