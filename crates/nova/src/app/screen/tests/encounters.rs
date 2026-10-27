use super::*;

// Boarding.

/// Every ship that holds armour is disabled: the trader (100), never
/// the player (none).
#[derive(Debug)]
struct Downs;

impl DisableRule for Downs {
    fn disabled(&self, _armor: Gauge, _hull: &HullSpec) -> bool {
        true
    }
}

/// Never repels, gives odds of 75, and always captures; a person
/// carries its `Credits`, and grants nothing.
#[derive(Debug)]
struct Sure;

impl BoardingRule for Sure {
    fn repels(&self, _booty: u16) -> bool {
        false
    }

    fn capture_odds(&self, _crew: &nova_sim::board::CaptureCrew, _chance: &mut dyn Chance) -> u8 {
        75
    }

    fn captures(&self, _odds: u8, _chance: &mut dyn Chance) -> bool {
        true
    }

    fn person_credits(&self, credits: i32, _chance: &mut dyn Chance) -> i64 {
        i64::from(credits)
    }

    fn grant(
        &self,
        _grant: &nova_sim::grant::PersonGrant,
        _stock: &[nova_sim::grant::GrantStock],
        _free_mass: i64,
        _grant_max: nova_sim::RuleSource,
        _chance: &mut dyn Chance,
    ) -> Option<nova_sim::grant::Granted> {
        None
    }
}

/// Draws its script, then the last outcome.
struct Script(VecDeque<u32>);

impl Chance for Script {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0.pop_front().unwrap_or(n - 1)
    }
}

/// The traffic's setup draws: the trader under the player at the
/// centre, facing up; then the last outcomes.
fn under_the_player() -> SharedChance {
    scripted_under_the_player().1
}

/// [`under_the_player`]'s chance, with its script, to draw more from.
fn scripted_under_the_player() -> (Rc<RefCell<Script>>, SharedChance) {
    let script = Rc::new(RefCell::new(Script([6, 6, 0, 0, 750, 750, 0, 0].into())));
    let shared: Rc<RefCell<dyn Chance>> = script.clone();
    (script, SharedChance::new(shared))
}

/// The draws a new pilot makes: its name's and nickname's defaults and its
/// christening's default, each the first, then its flight's: the offers' roll table its beginning draws, each
/// the last outcome, then the traffic's setup,
/// the trader under a new pilot's ship at (50, 50), where
/// [`nova_sim::Session::begin`] puts it, facing up; then the last
/// outcomes.
fn under_a_new_pilot() -> SharedChance {
    let christening = [0, 0, 0].into_iter();
    let table = std::iter::repeat_n(99, nova_sim::mission::ROLLED);
    let setup = [6, 6, 0, 0, 800, 800, 0, 0];
    let script: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Script(
        christening.chain(table).chain(setup).collect(),
    )));
    SharedChance::new(script)
}

/// [`boarding_data_with`] no cash.
pub(super) fn boarding_data() -> Rc<GameData> {
    boarding_data_with(0)
}

/// The first `chär` flies ship 128 ("First": average, a crew of 10)
/// in system 128 with `cash`, whose one `düde`, at 100 %, flies ship
/// 129 ("Trader": a crew of 3, 100 armour, 20 holds, 300 fuel, `Cost`
/// 150,000, and no speed, so it stays where setup places it) for food
/// and money (`Booty` 0x0041), independent. Food is commodity 0.
fn boarding_data_with(cash: i32) -> Rc<GameData> {
    let put = |bytes: &mut [u8], at: usize, values: &[i16]| {
        for (i, value) in values.iter().enumerate() {
            bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
        }
    };
    let ship = |holds, armor, crew| {
        let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
        put(&mut bytes, 0x00, &[holds, 0, 300, 300, 10, 300, 0, armor]);
        put(&mut bytes, 0x12, &[-1; 4]);
        put(&mut bytes, 0x42, &[1, crew]);
        put(&mut bytes, 0x4E, &[-1; 4]);
        put(&mut bytes, 0x370, &[-1; 4]);
        put(&mut bytes, 0x6CE, &[-1; 4]);
        bytes
    };
    let mut trader = ship(20, 100, 3);
    put(&mut trader, 0x06, &[0]);
    trader[0x30..0x34].copy_from_slice(&150_000_i32.to_be_bytes());
    let mut anim = vec![0; ShipAnim::SIZE.expect("fixed")];
    put(&mut anim, 0x00, &[1000, 0, 1]);
    put(&mut anim, 0x34, &[4]);
    let sheet = (0..4)
        .fold(RledBuilder::new(1, 1), |sheet, _| {
            sheet.frame(|f| f.line().pixels(&[0x7C00]))
        })
        .build();
    let mut character = vec![0; Character::SIZE.expect("fixed")];
    character[0x00..0x04].copy_from_slice(&cash.to_be_bytes());
    put(&mut character, 0x04, &[128, 128, -1, -1, -1]);
    put(&mut character, 0x0E, &[-1; 4]);
    let mut system = vec![0; System::SIZE.expect("fixed")];
    put(&mut system, 0x04, &[-1; 32]);
    put(&mut system, 0x44, &[128, -1, -1, -1, -1, -1, -1, -1]);
    put(&mut system, 0x54, &[100]);
    put(&mut system, 0x64, &[1, -1]);
    let mut dude = vec![0; Dude::SIZE.expect("fixed")];
    put(&mut dude, 0x00, &[1, -1, 0x0041]);
    put(&mut dude, 0x08, &[-1; 16]);
    put(&mut dude, 0x08, &[129]);
    put(&mut dude, 0x28, &[100]);
    let fork = ForkBuilder::new()
        .resource(Ship::TYPE, 128, Some(b"First"), &ship(0, 0, 10))
        .resource(Ship::TYPE, 129, Some(b"Trader"), &trader)
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character)
        .resource(ShipAnim::TYPE, 128, None, &anim)
        .resource(ShipAnim::TYPE, 129, None, &anim)
        .resource(RLED, 1000, None, &sheet)
        .resource(System::TYPE, 128, Some(b"Alpha"), &system)
        .resource(Dude::TYPE, 128, Some(b"Traders"), &dude)
        .resource(StrList::TYPE, 4000, None, &str_list(&["Food"]))
        .resource(StrList::TYPE, 4004, None, &str_list(&["75"]))
        .resource(
            StrList::TYPE,
            3000,
            None,
            &str_list(&as_strs(&comm_strings())),
        )
        .resource(StrList::TYPE, 2002, None, &str_list(&as_strs(&messages())))
        .resource(
            StrList::TYPE,
            9000,
            None,
            &str_list(&["Pay up.", "Paid.", "Declined.", "Short."]),
        )
        .build()
        .bytes;
    let file = OneFile(fork);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

/// The ship comm strings: "Channel open." in every variant of the
/// first group, and "c<n>" for each other.
fn comm_strings() -> Vec<String> {
    (1..=200)
        .map(|n| {
            if n <= 5 {
                "Channel open.".to_owned()
            } else {
                format!("c{n}")
            }
        })
        .collect()
}

/// The game's messages: "Greetings." at #175, "m<n>" for each other.
fn messages() -> Vec<String> {
    (1..=200)
        .map(|n| {
            if n == 175 {
                "Greetings.".to_owned()
            } else {
                format!("m{n}")
            }
        })
        .collect()
}

fn as_strs(strings: &[String]) -> Vec<&str> {
    strings.iter().map(String::as_str).collect()
}

/// `screen` over [`boarding_data`], its trader under the player,
/// idling and disabled at once.
fn boardable(screen: AppScreen) -> AppScreen {
    screen
        .with_chance(under_the_player())
        .with_behaviour(Rc::new(Idle))
        .with_disable_rule(Rc::new(Downs))
}

/// Ticks once (the system is set up and the trader disabled), targets
/// the trader with Tab and boards it with B.
fn board(screen: &mut AppScreen) {
    screen.tick(TICK);
    screen.input(&key(Key::Tab, true));
    screen.input(&key(Key::Tab, false));
    // Tab's beep, so a test hears only what follows.
    screen.take_sounds();
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Char('b'), false));
}

/// The router over [`boarding_data`] with dialogs, in the developer's
/// flight, boarded, its sounds taken.
pub(super) fn boarded() -> AppScreen {
    let mut screen = boardable(with_dialogs(boarding_data()));
    fly(&mut screen);
    let _ = screen.take_sounds();
    board(&mut screen);
    assert_eq!(screen.showing(), Showing::Plunder);
    assert!(
        screen
            .take_sounds()
            .contains(&Sound::Sim(SimSound::Boarded)),
        "the airlock, as the dialog opens"
    );
    screen
}

fn plunder_click(screen: &mut AppScreen, item: usize) -> Vec<Sound> {
    let at = screen
        .plunder()
        .expect("the plunder dialog is open")
        .dialog()
        .item_bounds(item)
        .expect("an item")
        .center();
    let mut sounds = Vec::new();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
        sounds.extend(screen.take_sounds());
    }
    sounds
}

fn assignment_click(screen: &mut AppScreen, item: usize) {
    let at = screen
        .assignment()
        .expect("the assignment dialog is open")
        .dialog()
        .item_bounds(item)
        .expect("an item")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
}

#[test]
fn b_on_a_disabled_target_opens_the_plunder_dialog_over_paused_flight() {
    let mut screen = boarded();
    let dialog = screen.plunder().expect("open");
    assert_eq!(dialog.lines()[2], "Cargo: 19 tons of Food");
    assert_eq!(dialog.lines()[4], "Capture Odds: 28%");
    let alpha = flight(&screen).alpha();
    screen.tick(TICK / 2);
    assert_eq!(
        flight(&screen).alpha().to_bits(),
        alpha.to_bits(),
        "flight is paused"
    );
    let list = drawn(&screen);
    let texts: Vec<String> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|text| text == "Capture Ship"), "{texts:?}");
    assert!(
        texts.iter().position(|text| text == "Capture Ship")
            > texts.iter().position(|text| text.contains("Alpha")),
        "over flight"
    );
}

#[test]
fn b_held_before_a_target_is_taken_opens_the_plunder_dialog_on_the_next_frame() {
    // `_HandlePlayer` boards every frame B is down until the attempt
    // passes the no-target return (@0x6a98b, @0x65031): the frame's
    // board opens the dialog as a press's does.
    let mut screen = boardable(with_dialogs(boarding_data()));
    fly(&mut screen);
    screen.tick(TICK);
    screen.input(&key(Key::Char('b'), true));
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::Flight, "no target");
    screen.input(&key(Key::Tab, true));
    screen.input(&key(Key::Tab, false));
    assert_eq!(screen.showing(), Showing::Flight);
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::Plunder, "with no input after it");
}

#[test]
fn each_take_goes_through_the_flight_and_refreshes_the_dialog() {
    let mut screen = boarded();
    let cash = flight(&screen).pilot().expect("a pilot").cash();
    let sounds = plunder_click(&mut screen, 3);
    assert_eq!(
        sounds,
        [Sound::Sim(SimSound::PlunderBeep)],
        "the take's beep, the click itself silent"
    );
    assert_eq!(
        flight(&screen).pilot().expect("a pilot").cash(),
        cash + 5750
    );
    assert_eq!(
        flight(&screen).message(),
        Some("You stole all the credits from this ship.")
    );
    assert_eq!(screen.showing(), Showing::Plunder, "still open");
    let sounds = plunder_click(&mut screen, 3);
    assert_eq!(
        flight(&screen).pilot().expect("a pilot").cash(),
        cash + 5750,
        "greyed: taken once"
    );
    assert_eq!(sounds, [], "a greyed button is a silent miss");
    plunder_click(&mut screen, 2);
    let dialog = screen.plunder().expect("open");
    assert_eq!(dialog.lines()[2], "Cargo: N/A", "refreshed");
}

/// A click that reaches no enabled button is a miss (item -1, which
/// `_PlunderFilter` still passes on, @0x92fdf): the dialog stays open,
/// for a click on its text and for one outside its frame alike.
#[test]
fn a_miss_keeps_the_plunder_dialog_open() {
    let mut screen = boarded();
    let sounds = plunder_click(&mut screen, 5);
    assert_eq!(sounds, []);
    assert_eq!(screen.showing(), Showing::Plunder, "the text");
    assert!(flight(&screen).boarding().is_some());
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: Point::new(2.0, 2.0),
        });
    }
    assert_eq!(screen.showing(), Showing::Plunder, "outside its frame");
    assert!(flight(&screen).boarding().is_some());
}

#[test]
fn abort_closes_the_dialog_back_into_flight() {
    let mut screen = boarded();
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.plunder().is_none());
    assert_eq!(flight(&screen).boarding(), None);
    screen.input(&key(Key::Escape, false));
    assert_eq!(
        screen.showing(),
        Showing::Flight,
        "the release is the dialog's"
    );
}

#[test]
fn the_plunder_dialog_comes_from_the_interface_file_when_there_is_one() {
    /// "Plunder" (1011), as stock, and nothing else.
    struct Plunder;

    impl DialogResources for Plunder {
        fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
            if id != PLUNDER_DIALOG {
                return Err(format!("no DLOG {id}"));
            }
            let user = |x, y, w, h, enabled| ItemTemplate {
                bounds: Bounds::at(Point::new(x, y), w, h),
                enabled,
                kind: ItemSpec::User,
            };
            Ok(DialogTemplate {
                bounds: Bounds::at(Point::new(0.0, 0.0), 309.0, 198.0),
                placement: Placement::Fixed,
                items: vec![
                    user(91.0, 166.0, 126.0, 25.0, true),
                    user(110.0, 110.0, 89.0, 25.0, true),
                    user(35.0, 138.0, 89.0, 25.0, true),
                    user(204.0, 110.0, 89.0, 25.0, true),
                    user(11.0, 7.0, 287.0, 96.0, false),
                    user(16.0, 110.0, 89.0, 25.0, true),
                    user(129.0, 138.0, 146.0, 25.0, true),
                ],
            })
        }
    }

    let mut screen = boardable(
        AppScreen::new(boarding_data()).with_dialogs(Rc::new(Plunder), Rc::new(MonoMetrics)),
    );
    fly(&mut screen);
    board(&mut screen);
    let pictures: Vec<_> = drawn(&screen)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::StretchedPicture {
                image, top_left, ..
            } => Some((*image, *top_left)),
            _ => None,
        })
        .collect();
    assert!(
        pictures.contains(&(PLUNDER_PICTURE, Point::new(0.0, 0.0))),
        "{pictures:?}"
    );
}

#[test]
fn without_any_metrics_a_boarding_is_let_go() {
    let mut screen = boardable(AppScreen::new(boarding_data()));
    fly(&mut screen);
    board(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(flight(&screen).boarding(), None, "aborted");
    assert!(
        !screen
            .take_sounds()
            .contains(&Sound::Sim(SimSound::PlunderBeep)),
        "let go silently"
    );
    assert_eq!(
        screen.take_warnings(),
        ["nova: cannot show the plunder dialog: no metrics to lay it out"]
    );
}

/// The router over [`boarding_data`] with dialogs and pilots kept in
/// `store`, capturing surely, flying the new pilot "Ada", boarded and
/// captured.
fn captured(store: &MemoryPilots) -> AppScreen {
    let mut screen = boardable(
        with_dialogs(boarding_data())
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
            .appeared(),
    )
    .with_chance(under_a_new_pilot())
    .with_boarding_rule(Rc::new(Sure));
    create(&mut screen, "Ada");
    board(&mut screen);
    assert_eq!(screen.showing(), Showing::Plunder);
    plunder_click(&mut screen, 7);
    assert_eq!(screen.showing(), Showing::Assignment);
    assert!(screen.plunder().is_none());
    screen
}

#[test]
fn a_capture_asks_its_assignment_and_use_as_escort_joins_the_fleet_and_saves() {
    let store = MemoryPilots::new();
    let mut screen = captured(&store);
    let alpha = flight(&screen).alpha();
    screen.tick(TICK / 2);
    assert_eq!(
        flight(&screen).alpha().to_bits(),
        alpha.to_bits(),
        "flight is paused"
    );
    assignment_click(&mut screen, ESCORT_ITEM);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(
        flight(&screen).message(),
        Some("You assigned this ship to your fleet of escorts.")
    );
    assert_eq!(pilot(&screen).escorts().len(), 1);
    assert_eq!(saved(&store, "Ada").escorts().len(), 1, "saved");
    assert_eq!((PLUNDER_DIALOG, ASSIGNMENT_DIALOG), (1011, 1018));
}

#[test]
fn use_as_my_ship_flies_the_captured_ship() {
    let store = MemoryPilots::new();
    let mut screen = captured(&store);
    assignment_click(&mut screen, MY_SHIP_ITEM);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(pilot(&screen).ship(), nova_sim::ShipId(129));
    assert_eq!(
        flight(&screen).message(),
        Some("You retained your old ship as an escort.")
    );
    assert_eq!(saved(&store, "Ada").ship(), nova_sim::ShipId(129));
}

#[test]
fn the_overlays_take_the_pointer_cancels_and_the_key_releases() {
    let store = MemoryPilots::new();
    let mut screen = captured(&store);
    let at = screen
        .assignment()
        .expect("open")
        .dialog()
        .item_bounds(ESCORT_ITEM)
        .expect("an item")
        .center();
    screen.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at,
    });
    screen.cancel_pointer();
    screen.release_keys();
    screen.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: false,
        at,
    });
    assert_eq!(screen.showing(), Showing::Assignment, "abandoned");
    let mut screen = boarded();
    let at = screen
        .plunder()
        .expect("open")
        .dialog()
        .item_bounds(3)
        .expect("an item")
        .center();
    screen.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at,
    });
    screen.cancel_pointer();
    screen.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: false,
        at,
    });
    assert_eq!(
        flight(&screen).boarding().map(|view| view.credits),
        Some(5750),
        "abandoned"
    );
}

// Hailing.

/// An option that asks a price, saying `STR#` 9000's strings: "Pay
/// up." asked, "Paid." paid, "Declined." declined and "Short." short.
#[derive(Debug)]
struct Sell;

impl HailOption for Sell {
    fn label(&self) -> String {
        "Sell".to_owned()
    }

    fn key(&self) -> Option<char> {
        Some('S')
    }

    fn applies(&self, _hail: &Hail) -> bool {
        true
    }

    fn press(&self, _hail: &Hail, _chance: &mut dyn Chance) -> Answer {
        let line = |index| Reply::Line { list: 9000, index };
        Answer {
            ask: Some(Ask {
                paid: (line(2), None),
                declined: (line(3), None),
                short: line(4),
            }),
            ..Answer::say(line(1))
        }
    }
}

/// `screen` over [`boarding_data`], its trader under the player,
/// idling and intact, and Nova's options with [`Sell`].
fn hailable(screen: AppScreen) -> AppScreen {
    screen
        .with_chance(under_the_player())
        .with_behaviour(Rc::new(Idle))
        .with_hail_options(HailOptions::default().with(Rc::new(Sell)))
}

/// The router over [`boarding_data`] with dialogs, in the developer's
/// flight, hailing the trader.
pub(super) fn hailing() -> AppScreen {
    let mut screen = hailable(with_dialogs(boarding_data()));
    fly(&mut screen);
    hail(&mut screen);
    assert_eq!(screen.showing(), Showing::Comm);
    screen
}

fn tap(screen: &mut AppScreen, k: Key) {
    screen.input(&key(k, true));
    screen.input(&key(k, false));
}

fn reply(screen: &AppScreen) -> String {
    flight(screen).hailing().expect("hailing").reply
}

#[test]
fn y_opens_the_comm_dialog_over_paused_flight() {
    let screen = &mut hailing();
    assert_eq!(reply(screen), "Channel open.");
    let alpha = flight(screen).alpha();
    screen.tick(TICK / 2);
    assert_eq!(
        flight(screen).alpha().to_bits(),
        alpha.to_bits(),
        "flight is paused"
    );
    let texts: Vec<String> = drawn(screen)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    for label in ["Greetings", "Request Assistance", "Sell", "Close Channel"] {
        assert!(texts.iter().any(|text| text == label), "{label}: {texts:?}");
    }
    assert!(
        texts.iter().position(|text| text == "Close Channel")
            > texts.iter().position(|text| text.contains("Alpha")),
        "over flight"
    );
    assert!(screen.comm().is_some());
}

#[test]
fn each_press_goes_through_the_flight_and_refreshes_the_dialog() {
    let screen = &mut hailing();
    tap(screen, Key::Char('g'));
    assert_eq!(screen.showing(), Showing::Comm, "still open");
    assert_eq!(reply(screen), "Greetings.", "the trader's advice: none");
    let shown: Vec<String> = drawn(screen)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(shown.iter().any(|text| text == "Greetings."), "{shown:?}");
}

#[test]
fn a_price_asked_opens_the_haggle_dialog_whose_choice_settles_it() {
    let screen = &mut hailing();
    tap(screen, Key::Char('s'));
    assert_eq!(screen.showing(), Showing::Haggle);
    assert_eq!(reply(screen), "Pay up.");
    let dialog = screen.haggle().expect("open");
    assert_eq!(dialog.lines(), ["Pay me 1,000 credits."]);
    tap(screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Haggle, "Escape does nothing");
    tap(screen, Key::Enter);
    assert_eq!(screen.showing(), Showing::Comm, "back to the comm dialog");
    assert!(screen.haggle().is_none());
    assert_eq!(reply(screen), "Short.", "no cash");
}

/// The router as [`hailing`] gives it, the pilot holding 10,000
/// credits, so the trader's price is 3,000 (3,600 at a mood of 1.2,
/// at most a third of the cash, in thousands), and Sell pressed with
/// the haggle roll `roll`.
fn haggling(roll: u32) -> AppScreen {
    let (script, chance) = scripted_under_the_player();
    let mut screen = hailable(with_dialogs(boarding_data_with(10_000))).with_chance(chance);
    fly(&mut screen);
    hail(&mut screen);
    assert_eq!(screen.showing(), Showing::Comm);
    script.borrow_mut().0 = [roll].into();
    tap(&mut screen, Key::Char('s'));
    assert_eq!(screen.showing(), Showing::Haggle);
    let dialog = screen.haggle().expect("open");
    assert_eq!(dialog.lines(), ["Pay me 3,000 credits."]);
    screen
}

#[test]
fn a_won_haggle_keeps_the_haggle_dialog_open_at_the_lowered_price() {
    let screen = &mut haggling(35);
    tap(screen, Key::Char('l'));
    assert_eq!(screen.showing(), Showing::Haggle, "still haggling");
    let dialog = screen.haggle().expect("open");
    assert_eq!(
        dialog.lines(),
        ["Pay me 2,200 credits."],
        "0.75 of 3,000, in hundreds, of the ship"
    );
    assert_eq!(reply(screen), "Pay up.", "unchanged");
    assert_eq!(cash(screen), 10_000);
    tap(screen, Key::Enter);
    assert_eq!(screen.showing(), Showing::Comm, "back to the comm dialog");
    assert!(screen.haggle().is_none());
    assert_eq!(reply(screen), "Paid.");
    assert_eq!(cash(screen), 7_800, "the lowered price paid");
}

#[test]
fn a_lost_haggle_declines_and_the_comm_dialog_takes_the_next_press() {
    let screen = &mut haggling(36);
    let at = screen
        .haggle()
        .expect("open")
        .dialog()
        .item_bounds(LOWER_ITEM)
        .expect("an item")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
    assert_eq!(screen.showing(), Showing::Comm, "back to the comm dialog");
    assert!(screen.haggle().is_none());
    assert_eq!(reply(screen), "Declined.");
    assert_eq!(cash(screen), 10_000, "nothing paid");
    tap(screen, Key::Char('g'));
    assert_eq!(reply(screen), "Greetings.", "not stuck");
    tap(screen, Key::Char('s'));
    assert_eq!(screen.showing(), Showing::Haggle);
    let dialog = screen.haggle().expect("open");
    assert_eq!(dialog.lines(), ["Pay me 4,000 credits."], "raised");
}

#[test]
fn close_channel_hangs_up_and_resumes_flight() {
    let screen = &mut hailing();
    let at = screen
        .comm()
        .expect("open")
        .dialog()
        .item_bounds(CLOSE_ITEM)
        .expect("an item")
        .center();
    let mut sounds = Vec::new();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
        sounds.extend(screen.take_sounds());
    }
    assert_eq!(sounds, []);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.comm().is_none());
    assert_eq!(flight(screen).hailing(), None, "hung up");
    let alpha = flight(screen).alpha();
    screen.tick(TICK / 2);
    assert_ne!(flight(screen).alpha().to_bits(), alpha.to_bits(), "resumed");
}

#[test]
fn escape_closes_the_channel_too() {
    let screen = &mut hailing();
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
    screen.input(&key(Key::Escape, false));
    assert_eq!(
        screen.showing(),
        Showing::Flight,
        "the release is the dialog's"
    );
}

#[test]
fn the_comm_dialogs_come_from_the_interface_file_when_there_is_one() {
    /// "Comm" (1007) and "Haggle" (1008), as the built-in ones, fixed.
    struct Comm;

    impl DialogResources for Comm {
        fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
            let user = |x, y, w, h, enabled| ItemTemplate {
                bounds: Bounds::at(Point::new(x, y), w, h),
                enabled,
                kind: ItemSpec::User,
            };
            match id {
                COMM_DIALOG => Ok(DialogTemplate {
                    bounds: Bounds::at(Point::new(0.0, 0.0), 423.0, 215.0),
                    placement: Placement::Fixed,
                    items: vec![
                        user(21.0, 181.0, 166.0, 26.0, true),
                        user(21.0, 153.0, 166.0, 26.0, true),
                        user(21.0, 125.0, 166.0, 26.0, true),
                        user(46.0, 241.0, 200.0, 25.0, true),
                        user(7.0, 320.0, 200.0, 25.0, true),
                        user(199.0, 335.0, 200.0, 25.0, true),
                        user(178.0, 261.0, 200.0, 25.0, true),
                        user(178.0, 289.0, 200.0, 25.0, true),
                        user(34.0, 299.0, 112.0, 16.0, false),
                        user(11.0, 8.0, 192.0, 58.0, false),
                        user(216.0, 7.0, 200.0, 200.0, false),
                        user(40.0, 73.0, 134.0, 46.0, false),
                    ],
                }),
                HAGGLE_DIALOG => Ok(DialogTemplate {
                    bounds: Bounds::at(Point::new(0.0, 0.0), 262.0, 107.0),
                    placement: Placement::Fixed,
                    items: vec![
                        user(58.0, 74.0, 146.0, 26.0, true),
                        user(58.0, 39.0, 146.0, 26.0, true),
                        user(7.0, 6.0, 248.0, 25.0, false),
                    ],
                }),
                _ => Err(format!("no DLOG {id}")),
            }
        }
    }

    let mut screen =
        hailable(AppScreen::new(boarding_data()).with_dialogs(Rc::new(Comm), Rc::new(MonoMetrics)));
    fly(&mut screen);
    hail(&mut screen);
    let pictures = |screen: &AppScreen| -> Vec<_> {
        drawn(screen)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::StretchedPicture {
                    image, top_left, ..
                } => Some((*image, *top_left)),
                _ => None,
            })
            .collect()
    };
    assert!(
        pictures(&screen).contains(&(nova_view::ui::comm::COMM_PICTURE, Point::new(0.0, 0.0))),
        "{:?}",
        pictures(&screen)
    );
    tap(&mut screen, Key::Char('s'));
    assert!(
        pictures(&screen).contains(&(nova_view::ui::comm::HAGGLE_PICTURE, Point::new(0.0, 0.0))),
        "{:?}",
        pictures(&screen)
    );
    assert!(screen.take_warnings().is_empty());
}

#[test]
fn without_any_metrics_a_hail_is_hung_up() {
    let mut screen = hailable(AppScreen::new(boarding_data()));
    fly(&mut screen);
    hail(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(flight(&screen).hailing(), None, "hung up");
    assert_eq!(
        screen.take_warnings(),
        ["nova: cannot show the comm dialog: no metrics to lay it out"]
    );
}

#[test]
fn every_rule_reaches_every_flight() {
    let every = RuleKey::ALL
        .into_iter()
        .fold(Rulebook::default(), |book, key| {
            book.with_override(key, RuleSource::Bible)
        });
    let reached = |screen: &AppScreen| {
        let rules = *flight(screen).session().expect("flying").rules();
        assert_eq!(rules, every);
        for key in RuleKey::ALL {
            assert_eq!(rules.source_for(key), RuleSource::Bible, "{key:?}");
        }
    };
    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    assert_eq!(
        flight(&screen).session().map(Session::rules),
        Ok(&Rulebook::default()),
        "the engine's by default"
    );
    let mut screen = AppScreen::new(data()).with_rulebook(&every);
    fly(&mut screen);
    reached(&screen);
    let store = MemoryPilots::new();
    let mut screen = menu(&store).with_rulebook(&every);
    create(&mut screen, "Ada");
    reached(&screen);
}

#[test]
fn the_routers_persons_rules_reach_every_flight() {
    let rules = nova_sim::NovaPersons {
        slots: RuleSource::Bible,
        ..nova_sim::NovaPersons::default()
    };
    let mut screen = AppScreen::new(data()).with_person_rules(Rc::new(rules));
    fly(&mut screen);
    let session = flight(&screen).session().expect("flying");
    assert_eq!(
        format!("{:?}", session.person_rules()),
        format!("{rules:?}")
    );
}

#[test]
fn each_flight_names_its_ship_from_the_game_datas_string_lists() {
    let mut screen = AppScreen::new(trading_data());
    fly(&mut screen);
    let strings = flight(&screen).session().expect("flying").strings();
    assert_eq!(strings.string_list(4000), ["Food"]);
}

/// The rule-bearing parts of `screen`, each by name, as they print:
/// its rule set and the strategies it hands each flight.
fn rules_of(screen: &AppScreen) -> Vec<(&'static str, String)> {
    vec![
        ("rules", format!("{:?}", screen.rules)),
        ("behaviour", format!("{:?}", screen.behaviour)),
        ("disable_rule", format!("{:?}", screen.disable_rule)),
        ("defence_rule", format!("{:?}", screen.defence_rule)),
        ("law", format!("{:?}", screen.law)),
        ("boarding_rule", format!("{:?}", screen.boarding_rule)),
        ("hail_options", format!("{:?}", screen.hail_options)),
        ("hire_terms", format!("{:?}", screen.hire_terms)),
        ("control_bits", format!("{:?}", screen.control_bits)),
        ("person_rules", format!("{:?}", screen.person_rules)),
    ]
}

/// What [`rules_of`] must find once the router is given `rulebook`:
/// the rule set, and each strategy built from it, by name, as they
/// print.
fn ports_of(rulebook: &Rulebook) -> Vec<(&'static str, String)> {
    vec![
        ("rules", format!("{rulebook:?}")),
        (
            "behaviour",
            format!("{:?}", NovaAi::from_rulebook(rulebook)),
        ),
        ("disable_rule", format!("{NovaDisable:?}")),
        ("defence_rule", format!("{Allegiance:?}")),
        ("law", format!("{:?}", NovaLaw::from_rulebook(rulebook))),
        (
            "boarding_rule",
            format!("{:?}", NovaBoarding::from_rulebook(rulebook)),
        ),
        ("hail_options", format!("{:?}", HailOptions::nova(rulebook))),
        (
            "hire_terms",
            format!("{:?}", NovaHire::from_rulebook(rulebook)),
        ),
        ("control_bits", format!("{NovaBits:?}")),
        (
            "person_rules",
            format!("{:?}", NovaPersons::from_rulebook(rulebook)),
        ),
    ]
}

#[test]
fn the_rulebook_routes_each_rule_to_its_own_setting() {
    assert_eq!(
        rules_of(&AppScreen::new(data())),
        ports_of(&Rulebook::default()),
        "the engine's are the router's defaults"
    );
    let every = RuleKey::ALL
        .into_iter()
        .fold(Rulebook::default(), |book, key| {
            book.with_override(key, RuleSource::Bible)
        });
    let books = RuleKey::ALL
        .into_iter()
        .map(|key| Rulebook::default().with_override(key, RuleSource::Bible))
        .chain([every]);
    for rulebook in books {
        assert_eq!(
            rules_of(&AppScreen::new(data()).with_rulebook(&rulebook)),
            ports_of(&rulebook),
            "{rulebook:?}"
        );
    }
    // The joining and grant rules reach the hail options' Use As
    // Escort, the boarding rule's count and the rule set's `Max`.
    let joined = Rulebook::default()
        .with_override(RuleKey::PersonJoin, RuleSource::Bible)
        .with_override(RuleKey::GrantCount, RuleSource::Bible)
        .with_override(RuleKey::GrantMax, RuleSource::Bible);
    let screen = AppScreen::new(data()).with_rulebook(&joined);
    let routed = rules_of(&screen);
    let setting = |name: &str| {
        routed
            .iter()
            .find(|(setting, _)| *setting == name)
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    };
    assert!(
        setting("hail_options").contains("JoinFleet { rule: Bible }"),
        "{routed:?}"
    );
    let boarding = setting("boarding_rule");
    assert!(boarding.contains("grant_count: Bible"), "{boarding}");
    assert!(!boarding.contains("grant_max"), "{boarding}");
    assert_eq!(
        screen.rules.source_for(RuleKey::GrantMax),
        RuleSource::Bible
    );
}

#[test]
fn the_routers_hail_options_reach_every_flight() {
    let mut screen = with_dialogs(boarding_data())
        .with_chance(under_the_player())
        .with_behaviour(Rc::new(Idle))
        .with_hail_options(HailOptions::empty());
    fly(&mut screen);
    hail(&mut screen);
    assert_eq!(flight(&screen).hailing().expect("hailing").options, []);
}
