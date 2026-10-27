//! The flight screen: the player's ship flying from its starting system,
//! over the original's starfield and among the system's stellars, with the
//! camera following the ship, and jumping through hyperspace along the
//! course plotted on the galaxy map.
//!
//! The screen owns a [`nova_sim::Session`], flying a new pilot
//! ([`FlightView::new`]) or a given one ([`FlightView::with_pilot`]), and
//! its catalog. It reads, once
//! when it is built: the session's system, through the [`SystemCatalog`]
//! port, the ship's sprite sheet, through the [`ShipSprites`](super::catalog::ShipSprites) port, the
//! HUD's status bar for the player's government, through the
//! [`StatusBars`](super::catalog::StatusBars) port, the galaxy for its course map, through the
//! [`GalaxyCatalog`](crate::galaxy::GalaxyCatalog) port, and the look of every weapon and explosion type,
//! through the [`CombatLooks`](super::catalog::CombatLooks) port. After that it reads only when the
//! session tells it entered a system afresh: that system, and when the
//! player's ship changes class: its sprite sheet, and when the system's NPC
//! traffic is populated: its traffic, through the [`TrafficCatalog`](nova_sim::TrafficCatalog)
//! port, and the target cards and codes of the ship types and governments
//! it brings. Drawing and input never read anything.
//!
//! The session populates the system's NPC traffic when it says to (see
//! [`Session::game_work`]), and as the ship takes off
//! ([`Session::take_off`]), rolled on the screen's [`SharedChance`].
//! Each time, the sprite sheet of each ship type the traffic can spawn is
//! read, once per type for the life of the screen. Each step runs an
//! advance pass in the original's order ([`Session::game_work`]): the
//! player's fire and move, the fight's collision passes, the traffic's AI,
//! its NPCs deciding as the screen's [`Behaviour`](nova_sim::Behaviour) says
//! ([`FlightView::with_behaviour`]; [`NovaAi`](nova_sim::NovaAi) by default), the shots,
//! the traffic's motion, the NPCs' fire and the beams, the fight's ships
//! disabled as the screen's
//! [`DisableRule`](nova_sim::DisableRule) says ([`FlightView::with_disable_rule`];
//! [`NovaDisable`](nova_sim::NovaDisable) by default), their point defence engaging every
//! missile fired at them, or by the other reading of the `pd_hostility`
//! rule those its [`PointDefenceRule`](nova_sim::PointDefenceRule) calls hostile
//! ([`FlightView::with_point_defence_rule`]; [`Allegiance`](nova_sim::Allegiance) by default),
//! and the player's crimes judged by its [`LegalCode`](nova_sim::LegalCode)
//! ([`FlightView::with_law`]; [`NovaLaw`](nova_sim::NovaLaw) by default). What could not be read of the looks
//! (each once, when the screen is built), then the session's diagnostics
//! about game data it does not handle yet, pass through
//! [`Screen::take_diagnostics`](crate::Screen::take_diagnostics) for the app to write out. Each NPC is
//! drawn with its own ship's sprite, after the stellars and before the
//! player, smoothed between its last two steps as the player's ship is
//! (a crossed box when its sheet cannot be read), and as a dim blip on the
//! radar. One waiting at the hypergate or wormhole it comes out of is not
//! drawn until its last 16 ticks there, over which it fades in; it is on
//! the radar all along, as the original's radar does not ask.
//!
//! In double time ([`FlightView::set_double_time`]), each step runs the
//! session's second pass ([`Session::tick_again`]) after the advance
//! pass, and steps the effects and
//! drains the fight's events after it as after the advance pass
//! (`_HandleExplods` runs on both, @0x444e1). The second pass answers
//! its hits as they land and what its close struck after it
//! ([`Session::answer_strikes`]), and says the attack quotes they
//! raise, then rolls the hail quotes ([`Session::tick_quotes_again`]),
//! as `_HandleShot` (@0x44385) and `_HandleShip` (@0x44483) run on both
//! passes; it gives no help and tells no comm. The mode is the screen's own
//! and outlives the hails, the map, the jumps and the sessions it flies;
//! while it is on, `cïcn` 20000 is drawn at the top left over the Red
//! Alert icon (see the `alert` module).
//!
//! # The view's size
//!
//! Flight asks to be widened ([`ViewShape::Widened`](crate::ViewShape)):
//! the app tells it the size, 768 high and as wide as the window makes it
//! ([`FlightView::set_view_size`]), Nova's 1024 x 768 until then. The
//! original's flight is the window's size, with every piece placed from
//! the window's edges or the play area left of the status bar
//! (`_SetupWind` @0x1b720, `_InitSprites`, `_SetupSpriteWorld` @0x21b54),
//! so each place here takes the view's size:
//!
//! - the camera keeps the ship at the play area's centre, the original's
//!   `_screenCenter`, (415, 384) at 1024 x 768, or by
//!   [`RuleKey::FlightAnchor`](nova_sim::RuleKey::FlightAnchor)'s other
//!   reading at the view's centre ([`FlightView::camera`],
//!   [`Anchor`](crate::system::camera::Anchor)); the play area, the
//!   brackets, clicks in space and the starfield (see the `stars` module)
//!   are placed from the same anchor;
//! - the status bar against the right edge
//!   ([`bar_origin`](super::hud::bar_origin)), the message box across the
//!   bottom ([`message_box`](super::hud::message_box)), the Red Alert
//!   icon at the play area's top right
//!   ([`alert_origin`](super::alert::alert_origin)) and the floating map
//!   from the right edge ([`floating::frame`](super::floating::frame));
//!   the escort menu and the double-time icon stay at the left;
//! - the play area the session is told each step batch, x from 0 to the
//!   width less [`STATUS_BAR_WIDTH`], for cloaks, scanners, observed
//!   goals, brackets and the asteroid field; a click in space, which the
//!   bar's [`STATUS_BAR_WIDTH`] at the right edge doesn't take; the
//!   original's stars, scattered and wrapped over the play area, or by
//!   [`RuleKey::FlightStarfield`](nova_sim::RuleKey::FlightStarfield)'s
//!   other reading the stand-in's dots, which cover the whole view; and
//!   the fades, which cover it.
//!
//! The course map, the bar's and the mission dialogs' included, is drawn
//! in its 1024 x 768 frame, centred over a widened flight and taking the
//! pointer moved back to match, and where the classic spaceport has it in
//! port. The status bar drawn under the spaceport
//! ([`FlightView::draw_status_bar`]) is at the classic width.
//!
//! Each step's fight events are drained into the [`Effects`]: its
//! explosions, debris and sounds, rolled on their own [`SharedChance`]
//! ([`FlightView::with_effects_chance`]), never on the simulation's. The
//! beams live before the fight's step are passed along, so a looped
//! weapon is heard once a beam. The fight is drawn in the sprite world's
//! layers, as `_SetupSpriteWorld` orders them
//! ([`nova_sim::sprite_layer::WORLD_PASSES`], `world.rs`): over the
//! stellars, the beams that go under the ships, the asteroid field
//! ([`asteroids`](super::asteroids), smoothed as the cargo boxes are), the
//! disabled ships, the cargo boxes a jettison left ([`boxes`](super::boxes),
//! smoothed as the shots are) and the escape pods, the smoke, the shots and
//! the ships in their layers (each smoothed as the ships are), the
//! explosions, the target layer (the brackets round the target, the cursor
//! and the red-alert and double-time icons), the other beams, the
//! floaters (the message line's text, the floating map and the escort
//! menu) and last the particles ([`weapons`](super::weapons),
//! [`effects`](super::effects), [`target`](super::target)). The HUD shows
//! the target panel and the secondary weapon's line, its text laid out
//! by the screen's [`TextMetrics`] ([`FlightView::with_metrics`]).
//! Landing and arriving clear the effects.
//!
//! Each day a jump takes rolls the planetary events on the screen's
//! [`SharedChance`] ([`FlightView::with_chance`]); without one, nothing
//! random happens. Landed at a trade center, the session's exchange can be
//! read ([`FlightView::market`]) and traded on ([`FlightView::trade`]);
//! landed at an outfitter, so can its outfitter ([`FlightView::outfitter`],
//! [`FlightView::outfit`]); and landed at a shipyard, a new ship can be
//! bought ([`FlightView::shipyard`], [`FlightView::buy_ship`]), whose
//! sprite sheet is read then.
//!
//! The screen redraws from what the session tells it
//! ([`SessionEvent`](nova_sim::SessionEvent)), never from which of the session's methods ran or
//! what it gave. Each frame first takes the session's events, so a change
//! made outside the screen's own methods (an edit through the pilot desk,
//! a set expression run by a control bit) is drawn by the next frame; each
//! of the screen's own methods that drives the session takes them last,
//! so the change is drawn at once. A change of ship, after a purchase, a
//! capture, or a `C`, `E` or `H` set operator, reads the player's sprite
//! sheet afresh when the class is another than the one it was read for,
//! once; a system entered afresh, by a jump, a gate, a move or a
//! relocation, is laid out; and a move to another stellar of the system
//! shown draws the ship there.
//!
//! Once the player's ship has exploded, the scene runs on without it
//! through the session's hold ([`nova_sim::HOLD_TICKS`]). On the step the
//! hold expires flight stops stepping, whatever the frame's length, and
//! the expiry is held for the router ([`FlightView::take_hold_expired`]),
//! which begins the fade when it is ready ([`FlightView::begin_fade`]):
//! the scene, frozen, fades to black over [`DEATH_FADE_FOR`](crate::fade::DEATH_FADE_FOR), HUD
//! included, as the original's synchronous `_FadeScreenOut` does, and then
//! the game is over ([`FlightView::game_over`]).
//!
//! The HUD is drawn over everything but a jump's fade and the death's: the status bar against the
//! right edge, its radar showing the stellars and ships around the ship
//! as drawn, its bars the session's shield, armour and fuel, and its nav
//! area the session's nav mode and selection. Without a status bar it
//! says why, and flight goes on. By
//! [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence)'s engine
//! reading the radar shows what it held at its last refresh, every 8
//! ticks ([`HudCadence`]) or when the original's `_RedrawStatus` would
//! redraw it (a new system, landing, take-off, or closing the map, the
//! comm dialog or a mission dialog); by the other, every frame. The nav
//! area likewise holds its text and colours between refreshes every 15
//! ticks, and when `_RedrawStatus` would redraw it, but in flight shows a
//! change of selection at once. The target panel likewise holds what it showed
//! between its redraws every 45 ticks, at the next radar refresh once the
//! target's shield or ship type changes, and when `_RedrawStatus` would
//! redraw it or the plunder dialog closes, but shows a change of target
//! at once ([`retake_target`](super::cadence::retake_target)). Its target blinks bright on the
//! same 8 ticks either way, and it shows no stellars or ships while the
//! ship is landed. The player's outfits give it IFF colours and a density
//! scanner ([`FrameHud::radar_gear`](nova_sim::flight_frame::FrameHud::radar_gear)); far from the system's centre an
//! arrow points back to it; and each tick the interference in the system
//! ([`FrameHud::interference`](nova_sim::flight_frame::FrameHud::interference)) may fill it with static, rolled on the
//! effects' chance.
//!
//! In port the app draws the status bar alone under the spaceport
//! ([`FlightView::draw_status_bar`]), its radar blank. Flight doesn't
//! advance there, so by the engine's reading the bar shows what the
//! port's redraws last took: landing and a docked start take everything,
//! as do the mission dialogs and texts, a port dialog closing, and the
//! bar, News, player info, mission info, a ship description and the
//! shipyard's name prompt closing ([`FlightView::refresh_status`]); an
//! outfitter order takes the radar alone; a ship purchase the radar, nav
//! and target, and the gauges too under another `ïntf`; and a recharge
//! the fuel gauge alone. The shield,
//! armour and fuel gauges, live in flight, hold while docked between
//! those points (`FlightView::reserves_shown`), as the original's port
//! draws them only at its redraws. So does the nav area
//! (`FlightView::nav_shown`): a landed `M` or `N`'s selection, written at
//! its operator, shows from the next of them, named by its `spöb` name
//! when the flight scene, still the system docked in, doesn't hold it. A
//! mission text's redraw shows the nav as the text noted it when it was
//! queued, unless ours has redrawn since ([`FlightView::take_mission_texts`]),
//! and the mission dialogs' and a ship purchase's redraws come after the
//! script run with them, as the original's do. The outfitter, exchange,
//! shipyard, the bar's hire screen and the mission computer save the nav
//! as they open and restore it as they close
//! ([`FlightView::close_port_dialog`]), so a landed `M` or `N` run inside
//! one is undone: shown at once after the outfitter, the exchange and the
//! computer, whose close redraws the nav; after the shipyard and the hire
//! screen, the close's redraw stays until the port's next draw. By the
//! other reading, everything follows every frame.
//!
//! Each [`Screen::tick`](crate::Screen::tick) runs the simulation's fixed-step clock: the
//! frame's time becomes whole steps of 1/30 s, each flown with the keys
//! held, and what is left over is how far the display is between the last
//! two steps. The ship, and the camera with it, are drawn that far from
//! the step before the last towards the last, so motion is smooth at any
//! frame rate while the simulation itself never depends on it.
//!
//! The ship is drawn as its sheet's rotation frame for the heading shown,
//! then, at the same centre and on the same frame (modulo the layer's
//! frame count), its engine glow and its running lights. The glow is
//! drawn at the level [`nova_sim::glow_level`] gives for the session's
//! glow base ([`Session::engine_glow`]) at the flight's time in ticks: it
//! fades in over 24 ticks of thrust, flickers, and fades out over 24
//! ticks after Up is released, and landing and beginning a jump put it
//! out. The lights are drawn at the level the session's display drew
//! them ([`Session::player_drawn`]'s, and each NPC's `drawn`), its
//! ported lights state stepped each display (`nova_sim::combat::ship_lights`,
//! random lights rolling on the flight's chance). Each is
//! drawn at [`lights_tint`](crate::draw::lights_tint) of its level, or not at all while it is hidden
//! or off. Both are combined by OR with what is beneath
//! ([`Blend::Or`](crate::Blend::Or)) at every level, scaled by level/32,
//! like the original's `_BlitPixieRLETranslucent` (0xc1568) below full and
//! `_BlitPixieRLEAddOver` (0xc24bf) at full: see [`lights_tint`](crate::draw::lights_tint) for the
//! full record. The flight's time
//! stops while the course map is open, so the lights blink and the glow
//! flickers in game time. The glow's flicker rolls on [`HashedRolls`]
//! from seed 1. A layer
//! that cannot be shown is left out silently.
//!
//! Input, the original's default keys:
//!
//! - Up thrusts, Left and Right turn, and Down turns to face against the
//!   ship's motion, while held: a press (or its key repeats) holds the key
//!   and its release lets it go. Left and Right together cancel. A
//!   (`FACE_TARGET_KEY`, the original's `autopilot`) turns to face the
//!   ship target, or the navigation stellar with no target or with Alt
//!   (Option) held. While Down or A aims the ship, Left and Right do
//!   nothing, as the original's turn block holds them off (see
//!   [`player_turn`](nova_sim::player_turn)).
//! - L, once a press (its repeats do nothing), requests clearance, then
//!   lands, as [`landing`](nova_sim::landing) says, in two presses: with
//!   no navigation target it selects the nearest stellar, and
//!   the press requests the stellar selected. Away from it traffic
//!   control answers with the request's line ([`request_line`]); within
//!   249 pixels it says nothing. On the ticks after, the dock approach
//!   greets the pilot ([`approach_greeting`]) as its own message, and a
//!   second L lands. While L is held the approach waits
//!   ([`Session::set_land_key`]), as the original's does. An uninhabited
//!   stellar answers every press "No response." ([`NO_RESPONSE`]) and
//!   lands in that press when it can; a refusal in the same press
//!   replaces it on the message line. The router takes the landing
//!   ([`FlightView::take_landing`]) and shows the spaceport. The line,
//!   the greeting and "No response." show for 250 ticks
//!   ([`SimMessage::ticks`]), and why a landing was refused for
//!   [`landing_refusal_ticks`], in the original's words (`STR#` 2002).
//!   The words' picks roll on the flight's chance.
//!   Over a hypergate or wormhole, the second L enters it instead
//!   ([`nova_sim::gate`]): a hypergate opens the course map as the
//!   hypergate map ([`MapMode::Hypergate`](crate::galaxy::MapMode::Hypergate)), offering its linked
//!   systems, and closing the map (M, or the router's Escape) enters it
//!   for the one picked, or cancels; a wormhole passes the ship through at
//!   once. The ship comes out in the new system, which is read and laid
//!   out, the message line says so ([`arrival_message_with`]), and the
//!   screen fades in from white out of a hypergate
//!   ([`JumpEffect::emerging`]) or flashes white out of a wormhole
//!   ([`JumpEffect::flash`]). The request's line, the greeting and the
//!   denial at a hypergate, and why a gate cannot be entered, are in the
//!   gate's own words ([`request_line`], [`approach_greeting`],
//!   [`HYPERGATE_DENIED`], [`gate_refusal_message`]).
//! - M (a press, not its repeats) opens the course map, a [`GalaxyMap`]
//!   in [`MapMode::Course`](crate::galaxy::MapMode::Course), and lets go
//!   of the flight keys. While it is open flight is paused, as in the
//!   original, only the map is drawn and every input goes to it, except
//!   that M closes it ([`FlightView::close_map`] closes it too, for the
//!   router's Escape), and so do Return and, while M is held, a key-down
//!   the original's `_MapFilter` doesn't take (@0x13cdd-0x13d07,
//!   @0x144db-0x14587). Tab or `\` on it (repeats too) cycles the
//!   hyperspace selection through the current system's links
//!   ([`Session::cycle_map_selection`]): the map draws the new selected
//!   link and selects its system, and the nav area is redrawn at once; on
//!   the hypergate map it steps the pick through the gate's destinations,
//!   Shift stepping back ([`Session::hypergate_cycle`]); docked, it does
//!   nothing. It beeps as M opens it ([`UiSound::MapOpened`],
//!   `_beepSnd[2]`, `_HandlePlayer` @0x6a809) and as it closes
//!   ([`UiSound::MapClosed`], `_beepSnd[0]`, @0x6a85a); a refused M says
//!   nothing, and the hypergate map neither opens nor closes with a beep.
//!   As it opens it selects the system the original's
//!   `_DoSystemMap` does ([`Session::map_preselection`]): the one
//!   selected in nav mode 3, or the current system in any other mode,
//!   and its panel shows that system. It draws the session's drawing
//!   ([`Session::map_drawing`], [`GalaxyMap::show_drawing`]): what
//!   `_DrawMap` draws of the known and explored systems, coloured by
//!   `_ColorCodeSystem`, with the mission systems taken as it opens
//!   ([`Session::map_mission_systems`]), taken again on the frame after
//!   the galaxy, the explored, charted or known systems, the course or
//!   the nav change, and after a plot, a Tab or a hypergate map click. A
//!   system clicked on the map becomes the destination: the session plots
//!   the course there and the map shows it, and the hop it cues as the
//!   selected link. Its info panels are the session's for the system it
//!   selects ([`Session::map_panel`], [`GalaxyMap::show_panel`]), taken
//!   with each drawing and whenever a click, a Tab or a gate pick changes
//!   the selection, measured by the flight's text metrics. The
//!   map's nebulae are the session's galaxy's ([`Session::galaxy`]),
//!   active and explored, rebuilt on the frame after each change to it;
//!   its panel's course line is the course as it is flown
//!   ([`Session::course_shown`]), each hop the visible system standing
//!   for it. Space centres it on the current system and selects it
//!   ([`UiSound::MapCentred`]), and Done closes it as Return does. F, S
//!   or its Find button open Find, which takes every input while it is
//!   open, M and Escape included; the session searches what the
//!   original's `_DoSystemSearch` does ([`Session::find_map_system`]),
//!   and the map selects and centres on a system found
//!   ([`UiSound::MapSystemFound`], or [`UiSound::MapSystemNotFound`]).
//!   Closing the map drops an open Find. A mission dialog opens the same
//!   map on its map key, docked or in flight
//!   ([`FlightView::open_dialog_map`]): marked with the green mission
//!   system and the arrow flag the dialog gives it, opening and closing
//!   in silence, its close followed by the dialog's own restore of the
//!   nav (see [`MapOpener`]'s module). So does the spaceport, a shop,
//!   the shipyard or the hire screen, and the ship description
//!   ([`FlightView::open_port_screen_map`]), silently and unmarked (see
//!   [`PortScreen`]'s module).
//! - The HUD's nav area shows the session's nav mode and its selection
//!   ([`Session::nav_mode`]): the nav system off, a stellar, or a system
//!   (see [`hud`](super::hud)). As in the original
//!   (`_HandlePlayer` @0x68390; see [`navigation`](nova_sim::navigation)),
//!   1-4 and F1-F4 ([`STELLAR_SLOT_KEYS`]) select the stellar in
//!   that slot of the system's nav defaults, 1-4 only while the escort
//!   menu is shut; F5 ([`NEAREST_STELLAR_KEY`]) and, with the menu shut,
//!   Command-5 ([`NEAREST_STELLAR_COMMAND_KEY`]) the nearest stellar, as L
//!   does with none. They share one latch and are polled once a frame, as
//!   the original's block is (the `slot_block` module): one pick a frame, by
//!   the block's order, and none while another of them is held. A left click in space
//!   ([`Session::click_in_space`]) picks a ship first: one under it
//!   becomes the target, a click on the player's own ship lets go of the
//!   target, and only a click that leaves the target as it was selects
//!   the stellar under it. Neither the status panel, the right
//!   [`STATUS_BAR_WIDTH`] of the screen, where the radar takes no clicks,
//!   nor the open escort menu's frame takes a click in space. Backquote, Nav Off
//!   ([`NAV_OFF_KEY`]), turns the nav system off, keeping the course; with
//!   Option held it lets go of the ship target instead. H
//!   ([`HYPERSPACE_MODE_KEY`]), Hyperspace Mode, selects the course's next
//!   hop, or no destination. Closing the course map cues the course's
//!   next hop too. The original has no stellar cycle key: Tab is its
//!   Target Select, cycling the ships. During a jump none of these keys
//!   does anything and a click selects no stellar (@0x69ad4, @0x69d2e,
//!   @0x6a5ef), but Option-\` still lets go of the ship target
//!   (@0x69a17-0x69a90) and a click on a ship still targets it, as the
//!   click's ship walk reads no jump state (@0x6a353-0x6a5e4).
//! - `\` (a press, not its repeats), the original's Hyper Select, selects
//!   the next system linked with this one, as
//!   [`Session::select_next_system`] says, so the nav area shows that
//!   system and J jumps there. By the engine's reading the course is
//!   kept, and the course map draws the selection as its own link.
//! - The floating map ([`floating`](super::floating)) is shown on every
//!   frame H is held outside a jump, and every frame `\` is held in nav
//!   mode 3, during a jump too, each from its press's own frame; then it
//!   stays full for 118 ticks, fades out by 150 and is hidden once more
//!   than 500 have passed, or by Return. It is the course map's drawing
//!   centred on the current system, with the session's course and mode-3
//!   selection, in a 256 x 256 frame at (558, 150) bordered in `cölr`
//!   `FloatingMap`, drawn after the double-time icon and before the escort
//!   menu. While it is up, faded out or not, a left click in its frame is
//!   its own: on the current system's centre it selects mode 3 with
//!   nothing, on a link's box mode 3 with that link, each with `snd ` 152
//!   ([`Session::select_on_floating_map`]), except during a jump, and
//!   anything else picks nothing. `-` zooms it out a step and `=` or the keypad's
//!   `+` in ([`ZOOM_OUT_KEY`], [`ZOOM_IN_KEY`], [`ZOOM_IN_KEYPAD_KEY`]),
//!   with `snd ` 150, during a jump too, but only while no key the flight
//!   holds is down. A hit or a zoom shows it again.
//! - J (a press) begins the pre-jump turn and slow-down towards the
//!   system selected when the session allows a jump, and otherwise
//!   says why in the original's words (`STR#` 2002), as a refused landing
//!   does. Once J is accepted the session flies the ship round to the
//!   jump's bearing ([`Session::preparing_jump`]) without the steering
//!   and thrust keys, and its fire keys fire nothing until arrival. While
//!   a jump is under way, the stage braking for it included, only the keys
//!   the original's `_HandlePlayer` does not gate on its jump state are
//!   taken ([`FlightView::taken_during_jump`]): Tab, R, W, Option-\`, B,
//!   Y, Command-K, the escort menu's keys and the floating map's zoom
//!   keys. The rest, J included, are
//!   ignored, P is refused, and a click in space selects no stellar,
//!   though it still targets a ship under it. The keys
//!   held are still tracked through the jump, so a steering, thrust or fire
//!   key still held at arrival acts on the first tick after it, as the
//!   original reads the keys live. H, \`, `\` and J are polled each tick
//!   with the original's latches ([`FlightView::held_keys_frame`]): H or
//!   \` held through a jump's end, and a `\` first pressed (or pressed
//!   again) during the jump and held, act once on the first tick after
//!   the arrival tick, with their sounds; a `\` held from before the jump,
//!   and J held through it, do nothing after it. J held into a disable, or
//!   first pressed while disabled, acts on the first tick the ship is
//!   intact again. The jump plays its [`JumpEffect`] from when the session
//!   says the jump has begun ([`Session::jumping`]), at once for a ship
//!   already facing that way and slow enough: the session flies the warp,
//!   the ship rushing off along its bearing, and the screen, HUD included,
//!   fades to white once the session lights it, as the original's
//!   whole-display fade does; then the ship arrives at full white when the
//!   session ends the warp, the new system is read and laid out, and the
//!   message line says so in the original's words ([`arrival_message`]). Flight resumes as the new system fades in
//!   from white: every key works again and the session flies, J included,
//!   as under the original's asynchronous display fade. A multi-jump plays one
//!   effect, toward the first system, and the scene loaded is the last
//!   system it passes. With the Hyperspace Effects preference off
//!   ([`FlightView::with_hyperspace_effects`],
//!   [`FlightView::set_hyperspace_effects`]) a jump skips the fades, as
//!   the original's does: the ship warps away unfaded, arrives when the
//!   session ends the warp, and the arrival frame shows solid white once,
//!   with flight going on under it.
//! - Space fires the primary weapons and Control the secondary selected,
//!   while held. W (a press) selects the next secondary weapon, and with
//!   Alt (Option) the one before. Tab (a press) targets the next ship in
//!   turn, R the nearest threat, and Alt-R (Option-R) the nearest ship
//!   ([`TargetPick`](nova_sim::TargetPick)). Shift-Tab targets back through the ships, and
//!   Shift-Alt-Tab (Shift-Option-Tab) back through the escorts, as the
//!   original's `_FindPrevShipInSystem` does. Every Tab press beeps
//!   ([`SimSound::TargetSelected`](nova_sim::SimSound::TargetSelected)),
//!   as does R picking a ship other than the target; R finding none
//!   beeps the refusal. A held key's repeat does nothing, as the
//!   original's per-press latches. Tab, R and W work during a
//!   jump too, picking from the system being left until arrival lets go
//!   of the target; Space and Control fire nothing then (@0x6aaf0,
//!   @0x6ab75).
//! - B (a press) boards the target when the session allows it
//!   ([`Session::board`]), by the screen's [`BoardingRule`](nova_sim::BoardingRule)
//!   ([`FlightView::with_boarding_rule`]; [`NovaBoarding`](nova_sim::NovaBoarding) by default),
//!   and otherwise says why in the original's words (`STR#` 2002
//!   #130-132), or nothing when there is no target or the player does not
//!   face it. A boarding lets go of the flight keys and is held for the
//!   router ([`FlightView::take_boarding`]), which opens the plunder
//!   dialog over the paused flight; each press there goes through
//!   [`FlightView::plunder`], and a capture's assignment through
//!   [`FlightView::assign`], each saying what it did as a message. After
//!   "Use As My Ship" the ship is drawn where it is, not on its way from
//!   the old one, and the session tells the change of ship, so the new
//!   ship's sprite sheet is read, once. Boarding a
//!   person that grants outfits says what it retrieved
//!   ([`grant_message`]) for [`GRANT_TICKS`]. A person of `Flags`
//!   0x0200 whose mission is available offers it in place of the plunder
//!   ([`Boarding::Offered`](nova_sim::Boarding::Offered)), held for the router as a hail's offer is.
//!   B works during a jump too, as `_HandlePlayerBoardAttempt` reads no
//!   jump state (0x65000-0x66064): its checks run on the ship as it brakes,
//!   or as it warps.
//! - Y (a press) hails the target ([`Session::hail_key`]), and with no
//!   ship target, or Alt (Option) held, the navigation stellar: one that
//!   answers opens nothing yet, where the original opens its comm dialog
//!   (`_DoPlanetCommDialog`), and any other says "No response.". A person
//!   whose mission is available offers it in place of the comm dialog,
//!   held for the router ([`FlightView::take_offer`]), which opens the
//!   offer dialog over the paused flight and answers it through
//!   [`FlightView::accept_mission`] or [`FlightView::refuse_mission`]; a
//!   mission that cannot be refused, with no offer text, is accepted at
//!   once and shows nothing. Any other hail opens the comm dialog,
//!   listing the screen's [`HailOptions`]
//!   ([`FlightView::with_hail_options`]; Nova's by default), its replies
//!   read through the [`CommCatalog`](nova_sim::CommCatalog) port, and otherwise says why in the
//!   original's words (`STR#` 2002 #53-54), or nothing with no target. A
//!   hail answered lets go of the flight keys and is held for the router
//!   ([`FlightView::take_hail`]), which opens the comm dialog over the
//!   paused flight; each press there goes through [`FlightView::answer`],
//!   each haggle through [`FlightView::haggle`], and closing the channel
//!   through [`FlightView::hang_up`]. Each step, the NPCs refuelling the
//!   player give their help in the traffic's tick (a repair is the
//!   traffic's, by the screen's [`DisableRule`](nova_sim::DisableRule)), and each that is done
//!   says so, as a message ([`comm_message`]), taken after the step's
//!   advance pass, which settles the notes in slot order with the frame's
//!   other lines ([`FlightEvent::Comm`](nova_sim::flight_frame::FlightEvent::Comm)). During a jump, its pre-jump stage included, Y is
//!   taken and the session refuses the hail, silently, as
//!   `_HandlePlayerCommunication` refuses while the jump state is set
//!   (@0x61f87; its beep is player-death phase 16's).
//! - P ([`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY), a press), outside the map, asks the router
//!   for the player info screen over the paused flight
//!   ([`FlightView::take_player_info_request`], with
//!   [`FlightView::player_info`]), letting go of the flight keys, as
//!   `_HandlePlayer` does (@0x702bf-0x703c9). During a jump, its pre-jump
//!   stage included, while the ship breaks up, and for
//!   [`TAKEOFF_GRACE_TICKS`](nova_sim::TAKEOFF_GRACE_TICKS) after a
//!   take-off or a gate's exit, the session refuses it with the shared
//!   refusal beep ([`Session::press_player_info`],
//!   [`SimSound::Refused`](nova_sim::SimSound::Refused), @0x7039a-0x703bd);
//!   once the ship has exploded, and in the escape pod, it does nothing
//!   and says nothing, as no key handler runs there. Its Jettison
//!   jettisons everything ([`FlightView::jettison_all`]).
//!   `_DoPlayerInfoDialog`'s own gate (@0x5a013-0x5a064) refuses nothing
//!   more: its `_screenFaded` holds only while a jump fades out, and
//!   nothing sets its `_fadePending` (only `_ResetPlayer` writes it, a 0,
//!   @0x1d7c3).
//! - I ([`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY), a press), outside the map, asks the router
//!   for the mission info screen over the paused flight
//!   ([`FlightView::take_mission_info_request`]), letting go of the
//!   flight keys, when the session shows an active mission there; with
//!   none it says "You have no active missions." (`STR#` 2002 #354) after
//!   the shared refusal beep
//!   ([`SimSound::Refused`](nova_sim::SimSound::Refused), @0x70557). The
//!   session decides ([`Session::press_mission_info`]), and refuses it in
//!   silence during a jump, its pre-jump stage included (+0x50,
//!   @0x70424), in the escape pod (+0xc908, @0x7042e), while the ship
//!   breaks up (`_IsDying` @0x70439, +0x3c @0x7044b), once it has
//!   exploded, and for
//!   [`TAKEOFF_GRACE_TICKS`](nova_sim::TAKEOFF_GRACE_TICKS) after a
//!   take-off or a gate's exit (`_frameCounter`, @0x7045e). A disabled
//!   ship takes it.
//! - The player's escorts fly beside it (see [`Session`]), their ship
//!   types' sprite sheets read with the traffic's. F, D, V and C (presses)
//!   command them ([`Session::command_escorts`]): attack the player's
//!   target, defend the player, hold position, and recall them to
//!   formation; Option-C is Return to Hangar, sending the fighters out of
//!   the player's bays home to dock and the others back to formation.
//!   A command goes to the group selected in the escort menu, or to every
//!   escort while it is shut, and says what it changed
//!   ([`escort_command_message`](super::escorts::escort_command_message)); one that changed nothing says nothing.
//!   E opens and closes the escort menu ([`EscortMenu`]), or says "You
//!   don't have any escorts." with none; while it is open, 1-5 select
//!   its group (rather than a stellar) and Return closes it. The menu is drawn over the flight,
//!   under the HUD, in the colours the [`EscortMenuLooks`](super::escorts::EscortMenuLooks) port gives,
//!   read when the screen is built. Option-Tab targets the next escort
//!   ([`TargetPick::NextEscort`](nova_sim::TargetPick::NextEscort)), and Shift-Option-Tab the one before
//!   ([`TargetPick::PreviousEscort`](nova_sim::TargetPick::PreviousEscort)), as for a hail. The escorts' standing
//!   orders reset, or not, on entering a system as the session's rule
//!   set says ([`FlightView::with_rules`]). While a jump is under way,
//!   the ship braking and turning for it or warping, the
//!   escort menu's keys are taken, among the few that are (see J above),
//!   as the original runs its escort block on every frame of a jump but
//!   the arrival (`_HandlePlayer` @0x6dd00-0x6e452): a command then says what the
//!   escorts will do ("will hold position."), and the menu still times
//!   out.
//! - The player's fighters launch from its bays as its secondary weapon
//!   (W selects a bay, Ctrl launches), and the fighter types every bay in
//!   the system launches have their sheets read with the traffic's. What
//!   a fighter launched does first, and what becomes of the fighters out
//!   as the player leaves a system, follow the session's rule set
//!   ([`FlightView::with_rules`]); fighters abandoned in a jump
//!   are told on arrival ([`fighters_abandoned_message`](super::escorts::fighters_abandoned_message)).
//! - Landed at an outfitter or a shipyard, the router asks the flight
//!   for its list ([`FlightView::outfitter`], [`FlightView::shipyard`])
//!   and trades through it ([`FlightView::outfit`],
//!   [`FlightView::outfit_counted`], [`FlightView::buy_ship`]), each item's roll for the day drawn on the
//!   flight's chance; the router tells the flight each time the
//!   outfitter opens ([`FlightView::open_outfitter`]). Where the shops,
//!   the exchange and the tribbles and perishables in flight follow a
//!   disputed rule, they read it from the session's rule set
//!   ([`FlightView::with_rules`]).
//! - Landed at a bar, the router asks the flight for the ships for hire
//!   and hires them ([`FlightView::escorts_for_hire`],
//!   [`FlightView::hire`]), the day's rolls drawn on the flight's chance,
//!   and has it draw the visit's news as the bar opens
//!   ([`FlightView::open_bar`], [`FlightView::news`]);
//!   the session's hire terms and control bits are set on the screen
//!   ([`FlightView::with_hire_terms`], [`FlightView::with_control_bits`]),
//!   and its disputed hiring rules come from its rule set
//!   ([`FlightView::with_rules`]). Hired escorts who defect unpaid
//!   on arrival or at take-off are told ([`defection_message`]).
//! - Persons appear as the session's rules say
//!   ([`FlightView::with_person_rules`]); the target panel shows a
//!   person's own name and subtitle, a person hailed says its comm quote
//!   as the session's rule set says ([`FlightView::with_rules`]), and
//!   each step a person may
//!   say its hail quote ([`Session::tick_quotes`], right after the
//!   fight and the answer to its hits, [`Session::answer_strikes`]),
//!   shown in the message line for
//!   the session's count ([`PersonQuote::ticks`](nova_sim::PersonQuote::ticks)),
//!   the last one said winning.
//! - Each step ends with the asteroid field's work
//!   ([`Session::tick_asteroids`], which [`Session::game_work`] runs
//!   after the fight): a spawn at the field's edge and the field stepped,
//!   on the flight's chance, so the next step's shots meet the asteroids
//!   where they are drawn. Before any step, and so
//!   before the first frame is drawn, a session started or loaded in
//!   flight places its field as its rules say
//!   ([`Session::settle_asteroid_field`]).
//! - Return ([`MENU_CLOSE_KEY`], the original's `ackComm`) clears the
//!   message line, closes the escort menu and hides the floating map.
//! - While the player's ship breaks up, M, L and I do nothing, saying
//!   nothing, as `_HandlePlayer`'s breakup-timer gates refuse them
//!   (0x6a7b3, 0x6a8cb, 0x7044b); every other key is taken, and the
//!   session's own refusals (a hail, a boarding, a jump) answer as they
//!   do for a disabled ship. Once it has exploded no key is taken, as the
//!   original's dead branch (0x6870d-0x6888c) skips every handler. The
//!   screen asks the session which commands it takes
//!   ([`Session::takes`]).
//! - Escape belongs to the app's router, which closes the map or leaves
//!   flight. The screen never quits.
//!
//! The message line is drawn as `_DisplayComm` (@0x90201) draws it: one
//! slot, a new message replacing the old and its time, in the rect
//! `_InitSprites` sets up ([`hud::message_box`](super::hud::message_box)), wrapped, in the system
//! font at 12 pt in white ([`hud::draw_message_text`](super::hud::draw_message_text)). It is drawn over the
//! starfield and under everything else, as the original draws it on the
//! sprite world's background. Each message stays for as many ticks of
//! flight as its original caller asks ([`MESSAGE_TICKS`] and the
//! constants after it, [`SimMessage::ticks`], [`CommNote::ticks`](nova_sim::CommNote::ticks),
//! [`PersonQuote::ticks`](nova_sim::PersonQuote::ticks)). The dialogs drawn over
//! flight (the comm, offer and plunder dialogs, the map) leave it as it
//! is, and it shows again afterwards for the ticks it had left, as
//! `_RedrawComm` (@0x903a0) redraws it; the plunder dialog counts it down
//! one tick a press that leaves the dialog open, as its loop calls
//! `_HandleComm` (@0x949a1). Starting a jump clears it only
//! when it says [`TOO_CLOSE`], [`TRACTORED`], [`FAR_ENOUGH`] or
//! [`NOTHING_OUT_HERE`]
//! (`_HandlePlayer` @0x6b231-0x6b3d9), and
//! opening the escort menu only when it says [`NO_ESCORTS`](super::escorts::NO_ESCORTS)
//! (@0x6dedb-0x6df29). The system's title and the [`HELP`] line are the
//! port's own, drawn only when turned on ([`FlightOverlays`],
//! [`FlightView::with_overlays`]).
//!
//! # The world layer
//!
//! Flight is the one screen that draws a world layer
//! ([`DrawList::world`](crate::DrawList::world),
//! [`DrawLayer`](crate::DrawLayer)): the stars, the
//! stellars, asteroids and boxes, the beams, the ships with their glow and
//! lights, the shots, the effects and the smoke, everything the original
//! draws into its sprite world. The brackets, the alerts, the floating
//! map, the escort menu, the title and help, the status bar, the cursor,
//! the fades and the course map are overlay, and so is every text: the
//! message line and a placeholder's reason too, though they are drawn
//! among the world's commands. Drawn at the window's resolution the
//! layers change nothing, as each command is drawn in the list's order.
//! Under a frame upscaler, which draws the world first and the overlay
//! over it, the message line would show over the ships, where the
//! original has it under them.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::rc::Rc;
use std::time::Duration;

use nova_sim::DrawnBox;
#[cfg(doc)]
use nova_sim::Session;
use nova_sim::flight_frame::{FlightFrame, FrameShip, GateFade, ShipPlace, StepReport};
use nova_sim::order_loop::{OrderRun, Stopped};
use nova_sim::traffic::emerge::{FULL_LEVEL, emerge_level};
use nova_sim::traffic::gate_entry::enter_level;
use nova_sim::{
    Chance, CombatEvent, Condition, FixedStep, GovtId, HailOptions, HailView, HashedRolls, NavMode,
    NeverFires, NpcId, OutfitRefusal, OutfitRun, Reselection, Reserves, ShipId, ShipRef,
    SimMessage, StellarId, SystemId, Vec2, flight::normalized, flight::shortest_turn,
};
use nova_sim::{InterfaceLoad, MissionInfo, OfferPrompt, TextBar};

use super::cadence::{Drawn, HudCadence};
use super::catalog::{Looks, ShipSheet, TargetCard};
use super::cursor::HoverCursor;
use super::effects::{Dying, Effects, Scene};
use super::escorts::{EscortMenu, EscortMenuColors};
use super::flash::ShieldFlash;
use super::floating::FloatingMap;
use super::hud::{NavDisplay, STATUS_BAR_WIDTH, StatusBar};
use super::jump::JumpEffect;
use super::radar::{RadarScene, RadarShip};
use super::sim_adapter::SessionFlight;
use super::sim_port::FlightSim;
use super::stars::Starfield;
use super::target::TargetPanel;
use super::weapon_glow::WeaponGlows;
use crate::fade::ScreenFade;
use crate::galaxy::GalaxyMap;
use crate::galaxy::map::MapPicture;
use crate::system::camera::{Anchor, Camera};
use crate::system::catalog::SystemCatalog;
use crate::system::kept_frames::KeptFrames;
use crate::system::scene::{PLACEHOLDER_SIZE, StellarLook, SystemScene};
use crate::text::TextMetrics;
use crate::{Color, Diagnostic, Key, Point, SoundPrefs, UiSound};
use dialog_map::MapOpened;

mod build;
mod cockpit;
mod dialog_map;
mod docked;
mod drawing;
mod inbox;
mod interface;
mod messages;
mod outputs;
mod port_screen_map;
mod readouts;
mod screen;
mod shots;
mod slot_block;
mod sprites;
mod stars;
mod stellar_looks;
mod stepping;
mod travel;
mod world;

pub use dialog_map::MapOpener;
pub(crate) use dialog_map::is_map_press;
pub use messages::*;
pub use port_screen_map::PortScreen;

/// The developer overlays: the system's title and the help line.
const TITLE: Point = Point::new(16.0, 32.0);
const TITLE_SIZE: f32 = 20.0;
const HELP_AT: Point = Point::new(16.0, 744.0);
const OVERLAY_SIZE: f32 = 14.0;
/// How far below the ship's placeholder the reason goes.
const MESSAGE_GAP: f32 = 22.0;
/// The help line.
pub const HELP: &str = "Arrows: fly   A: face target   Space: fire   Ctrl: secondary   W: weapon   Tab: next target   R: nearest   B: board   Y: hail   Cmd-K: jettison   F/D/V/C: escorts   Alt-C: dock   E: escort menu   L: land   M: map   J: jump   \\: next system   P: player info   I: mission info   Cmd-,: preferences   Esc: leave";
/// The developer settings flight carries: two overlays the original does
/// not draw, and cheat mode. All are off by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlightOverlays {
    /// The system's name and `sÿst` ID at the top left.
    pub title: bool,
    /// The [`HELP`] line of keys at the bottom.
    pub help: bool,
    /// The original's cheat mode (`_cheatsActive`, which nothing in 1.1
    /// turns on): the player info shows its IDs
    /// ([`PlayerInfo::cheats`](nova_sim::player_info::PlayerInfo::cheats)).
    pub cheats: bool,
}

/// The keys flight holds: the original's defaults, Alt, which turns the
/// weapon select key back, Shift, which turns the target select key back,
/// the weapon select key, whose hold fades the cursor, the weapon safety
/// key, which lets go of the secondary on every pass it is held,
/// Command, which the jettison, eject and self-destruct keys need, the
/// eject key and the self-destruct key, which work only held with Command,
/// the cloak key, which toggles once a hold, the land key, which holds the dock
/// approach back while it is down, H and `\`, which show the floating map
/// while they are down, \` and J, and B, which `_HandlePlayer` polls every
/// frame, setting the cursor's fade while it is down (@0x6a98b-0x6a999).
/// H, `\`, \` and J act again when held across a jump's end, a release
/// of Option or a repair, and B once a target is taken or the cloak drops,
/// as the original polls them with a latch each
/// ([`FlightView::held_keys_frame`]); the original's
/// `_quickKeyCheckResult` counts \`, J and B as held too (@0x69aa1,
/// @0x6b0e6, @0x6a98b), which the floating map's zoom keys read. The
/// stellar slot keys 1-4, F1-F4, F5 and the 5 are held for their block's
/// latch and frame (see the `slot_block` module); they fade the cursor and
/// hold the zoom back only while `_HandlePlayer` polls them
/// ([`FlightView::slot_key_polled`]).
const FLIGHT_KEYS: [Key; 32] = [
    Key::Up,
    Key::Left,
    Key::Right,
    Key::Down,
    FACE_TARGET_KEY,
    AFTERBURNER_KEY,
    FIRE_KEY,
    SECONDARY_KEY,
    SELECT_KEY,
    DESELECT_KEY,
    Key::Alt,
    Key::Shift,
    Key::Command,
    LAND_KEY,
    EJECT_KEY,
    SELF_DESTRUCT_KEY,
    CLOAK_KEY,
    HYPERSPACE_MODE_KEY,
    HYPER_SELECT_KEY,
    NAV_OFF_KEY,
    JUMP_KEY,
    BOARD_KEY,
    STELLAR_SLOT_KEYS[0][0],
    STELLAR_SLOT_KEYS[1][0],
    STELLAR_SLOT_KEYS[2][0],
    STELLAR_SLOT_KEYS[3][0],
    NEAREST_STELLAR_COMMAND_KEY,
    STELLAR_SLOT_KEYS[0][1],
    STELLAR_SLOT_KEYS[1][1],
    STELLAR_SLOT_KEYS[2][1],
    STELLAR_SLOT_KEYS[3][1],
    NEAREST_STELLAR_KEY,
];

/// The face-target key: the original's default A for its `autopilot`
/// (keyPrefs +0xe, named by `_loadKeys` @0xcd8c5-0xcd935). Held, it turns
/// the ship to face its target, or with Alt (Option) or no target the
/// navigation stellar ([`Face`](nova_sim::Face)); a flight key, so it
/// fades the cursor, as its press sets *0x3b5550 (@0x6aeeb).
pub const FACE_TARGET_KEY: Key = Key::Char('a');
/// The afterburner key: the original's default, Mac keycode 6, Z (keyPrefs
/// +0x30, `_loadKeys` @0xcd899-0xcd8bc). Held, it engages the afterburner
/// ([`Controls::afterburner`](nova_sim::Controls::afterburner)); a flight
/// key, so it fades the cursor, as its press sets *0x3b5550 whether or not
/// the ship carries an afterburner (@0x6cb03).
pub const AFTERBURNER_KEY: Key = Key::Char('z');
/// The primary fire key: the original's default (`fireKey0`, `_loadKeys`
/// @0xcd5f8).
pub const FIRE_KEY: Key = Key::Space;
/// The secondary fire key: the original's default (`fireKey1`).
pub const SECONDARY_KEY: Key = Key::Control;
/// The secondary weapon select key: the original's default
/// (`weapSelect`); with Alt (Option) it selects the one before, in `wëap`
/// ID order, on the next pass (see `nova_sim::session`'s
/// `secondary_select`). Held, it fades the cursor, as `_HandlePlayer` sets
/// *0x3b5550 on every pass it is down (@0x6ac2a).
pub const SELECT_KEY: Key = Key::Char('w');
/// The weapon safety key: the original's default for key-preference slot
/// 0x2 (`weapSafe`, Keys.nib's "Weapon Safety:"), keycode 0x01, S, which
/// `_loadKeys` falls back to when the defaults hold none
/// (@0xcdde9-0xcde0c). Held, it lets go of the secondary selected, on
/// every pass with one selected (`_HandlePlayer` @0x6adc2-0x6ae1b; see
/// [`Session::hold_weapon_safety`](nova_sim::Session::hold_weapon_safety)).
pub const DESELECT_KEY: Key = Key::Char('s');
/// The target select key: the original's default (`targSel`), which
/// picks the next ship in turn.
pub const TARGET_KEY: Key = Key::Tab;
/// The closest target key: the original's default (`closeTarg`), which
/// picks the nearest threat, and with Alt (Option) the nearest ship.
pub const NEAREST_KEY: Key = Key::Char('r');

/// The land key: the original's default (`STR#` 129, and `STR#` 2002
/// #25).
pub const LAND_KEY: Key = Key::Char('l');
/// The galaxy map key: the original's documented default (`Keys.nib`'s
/// `mapKey`; `STR#` 2002 #26-28 name a map key).
pub const MAP_KEY: Key = Key::Char('m');
/// The hyperspace jump key: the original's documented default
/// (`Keys.nib`'s `jumpKey`).
pub const JUMP_KEY: Key = Key::Char('j');
/// The board key: the original's default (`Keys.nib`'s `boardKey`), one
/// attempt a press.
pub const BOARD_KEY: Key = Key::Char('b');
/// The hail key: the original's default (`Keys.nib`'s `hailKey`), one
/// hail a press.
pub const HAIL_KEY: Key = Key::Char('y');
/// The jettison key: the original's default (`dumpCargo`, `_keyPrefs`
/// +0x1e, keycode 0x28, `_loadKeys` @0xce33d-0xce360), which jettisons
/// only with Command held (`_HandlePlayer` @0x6fd54-0x6fdaf): everything,
/// or with Option (Alt) also held, all but the mission cargo. It works
/// during a jump too: `_JettisonCargo` (0x3dffd-0x3e3c4) reads no jump
/// state.
pub const JETTISON_KEY: Key = Key::Char('k');
/// The eject key: the original's default for key-preference slot 0x22
/// (`ejectKey`), keycode 0x07, X, which ejects only held together with
/// Command (`_HandlePlayer` @0x6f29e-0x6f2c1; see the session's eject).
pub const EJECT_KEY: Key = Key::Char('x');
/// The self-destruct key, `-`: key-preference slot 0x24 (`destructKey`),
/// which `_loadKeys` reads (@0xce458) and then forces to keycode 0x1b
/// whatever the preference says (@0xce71c). It acts only held together
/// with Command (`_HandlePlayer` @0x6fdb4-0x6fddb; see the session's
/// self-destruct), so it never clashes with the floating map's zoom out,
/// the same key, which refuses any press with Command held
/// (@0x70fa7-0x70fb5). The keypad's `-` (0x4e) arrives as the same key,
/// a platform difference accepted as [`ZOOM_OUT_KEY`]'s is.
pub const SELF_DESTRUCT_KEY: Key = Key::Char('-');
/// The cloak key: the original's default for key-preference slot 0x52
/// (`cloakKey`), keycode 0x20, U (`_loadKeys` @0xce461-0xce4d4). Held,
/// it cloaks or uncloaks once a hold, with no Command
/// (`_HandlePlayer` @0x705b7; see the session's `player_cloak`).
pub const CLOAK_KEY: Key = Key::Char('u');
/// The escorts' attack key: the original's default (`Keys.nib`'s
/// `escortCmdKey0`, "Attack Target").
pub const ESCORT_ATTACK_KEY: Key = Key::Char('f');
/// The escorts' defend key (`escortCmdKey1`, "Defend Me").
pub const ESCORT_DEFEND_KEY: Key = Key::Char('d');
/// The escorts' hold key (`escortCmdKey2`, "Hold Position").
pub const ESCORT_HOLD_KEY: Key = Key::Char('v');
/// The escorts' recall key (`escortCmdKey3`, "Recall: (option = dock)").
pub const ESCORT_RECALL_KEY: Key = Key::Char('c');
/// What closes the escort menu besides its own key: Return
/// (`ackComm`).
pub const MENU_CLOSE_KEY: Key = Key::Enter;

/// The stellar slot keys: each pair selects the stellar in that slot of
/// the system's navigation defaults, from the first. The original reads
/// keys 1-4 (Mac keycodes 0x12-0x15) and F1-F4 (`_loadKeys` @0xce6c8
/// hard-codes 0x7a, 0x78, 0x63, 0x76 at +0x34..+0x3a) in `_HandlePlayer`
/// @0x69d2b-0x69f17, with F5 and Command-5, as one block with one latch
/// (the `slot_block` module). While the escort menu shows,
/// `_QuickKeyCheck` hides 1-4 from the block (@0xa106-0xa124) and they
/// pick its groups instead
/// ([`GROUP_KEYS`](super::escorts::GROUP_KEYS)); F1-F4 always pick a
/// stellar.
pub const STELLAR_SLOT_KEYS: [[Key; 2]; 4] = [
    [Key::Char('1'), Key::Function(1)],
    [Key::Char('2'), Key::Function(2)],
    [Key::Char('3'), Key::Function(3)],
    [Key::Char('4'), Key::Function(4)],
];
/// The nearest stellar key: F5 (Mac keycode 0x60), which the original
/// reads beside the slot keys (@0x69cf6-0x69f17) and which selects the
/// stellar its land key would with nothing selected. Command-5
/// ([`NEAREST_STELLAR_COMMAND_KEY`]) is its twin.
pub const NEAREST_STELLAR_KEY: Key = Key::Function(5);
/// The other nearest stellar key: the top-row 5 (Mac keycode 0x17), held
/// together with Command (0x37), which `_HandlePlayer` polls only while F5
/// is up (@0x69dbc-0x69dda) and picks with as F5 does. Both are
/// hard-coded, not key preferences. While the escort menu shows,
/// `_QuickKeyCheck` hides the 5 from this block (@0xa106-0xa124), so
/// Command-5 then picks the menu's Freighters instead
/// ([`GROUP_KEYS`](super::escorts::GROUP_KEYS)); 5 alone picks nothing.
pub const NEAREST_STELLAR_COMMAND_KEY: Key = Key::Char('5');
/// The Nav Off key, which turns the nav system off, the course kept: the
/// original's default, backquote (`_loadKeys` @0xcd5f8 `navOff` 0x32,
/// handled in `_HandlePlayer` @0x69a94-0x69b63). With Option held it
/// lets go of the ship target instead ([`Session::clear_target`],
/// @0x69a17-0x69a90). It shares H's latch: held through a jump's end it
/// turns the nav off after it, Option let go with it still down turns
/// the nav off, and Option-\` held lets go of a target picked meanwhile
/// on the next frame ([`FlightView::nav_keys_frame`]). While the dev
/// overlay is built in, backquote toggles the overlay instead and never
/// reaches flight.
pub const NAV_OFF_KEY: Key = Key::Char('`');

/// The Hyper Select key, which cycles the hyperspace selection through
/// the systems the current one links to, keeping the course by the
/// engine's reading: the original's default, `\`
/// (`Keys.nib`'s `hyperSel`, "Hyper Select:", default keycode 0x2a in
/// `_loadKeys`). H is the original's separate Hyperspace Mode key. Held
/// in nav mode 3, during a jump too, it shows the floating map
/// (@0x69cdd-0x69ce7, @0x69be7; see the [`floating`](super::floating)
/// module). One first pressed during a jump, or let go and pressed again
/// then, and held past its end selects on the first frame after it; one
/// held from before the jump does nothing after it
/// ([`FlightView::select_frame`]).
pub const HYPER_SELECT_KEY: Key = Key::Char('\\');

/// The Hyperspace Mode key, which puts the nav system in hyperspace mode
/// and selects the course's next hop ([`Session::hyperspace_mode`]): the
/// original's default, H (`_loadKeys` @0xcd5f8 `hyperMode`, keycode
/// 0x04, handled in `_HandlePlayer` @0x69b76-0x69b89). Held outside a
/// jump, it shows the floating map (@0x69b8e-0x69ba8; see the
/// [`floating`](super::floating) module). Held through a jump's end, it
/// acts as a press on the first frame after it, as its latch is cleared
/// on every frame of a jump ([`FlightView::nav_keys_frame`]).
pub const HYPERSPACE_MODE_KEY: Key = Key::Char('h');

/// The floating map's zoom-out key, `-` and the keypad's `-` (keycodes
/// 0x1b and 0x4e, `_HandlePlayer` @0x70f4e-0x71110), which the platform
/// both gives as this.
pub const ZOOM_OUT_KEY: Key = Key::Char('-');
/// The floating map's zoom-in key, `=` (keycode 0x18).
pub const ZOOM_IN_KEY: Key = Key::Char('=');
/// The floating map's other zoom-in key, the keypad's `+` (keycode 0x45).
pub const ZOOM_IN_KEYPAD_KEY: Key = Key::Char('+');

/// Where a ship is drawn `alpha` of the way from `from` to `to`.
fn shown_position(from: ShipPlace, to: ShipPlace, alpha: f32) -> Point {
    let (from, to) = (from.position, to.position);
    Point::new(
        (to.x - from.x).mul_add(alpha, from.x),
        (to.y - from.y).mul_add(alpha, from.y),
    )
}

/// Which way a ship is drawn facing `alpha` of the way, the short way
/// round, from `from` to `to`.
fn shown_heading(from: ShipPlace, to: ShipPlace, alpha: f32) -> f32 {
    normalized(shortest_turn(from.heading, to.heading).mul_add(alpha, from.heading))
}

/// How an NPC at `gate` is tinted: `None` while it waits hidden at the gate
/// it comes out of; white with the alpha of its [`emerge_level`] over
/// its last [`FADE_TICKS`](nova_sim::traffic::emerge::FADE_TICKS) there;
/// white with the alpha of its [`enter_level`] while it enters a gate to
/// leave, fading out; plain white otherwise, held at its gate by the
/// player's hit too (its goal 4 is not the fade's goal 0x15).
///
/// The original (`_HandleShipDisplay` @0x2c090-0x2c18e) sets the hull's
/// translucency t to 32 less this level, entering a gate as
/// trunc(−2.0 × w + 32.0) (@0x2bfb2-0x2c007) and coming out as 32 − 2d,
/// d = trunc(16.0 − w) (@0x2c0c8-0x2c12b), and blits it washed toward
/// white ([`gate_colorize`](crate::flight::gate_colorize)) and then
/// crossfaded at (32 − t)/32, which this alpha draws. An NPC gets no
/// screen flash, as the player's own gate arrival does (see
/// [`jump`](super::jump)).
fn npc_tint(gate: GateFade) -> Option<Color> {
    let level = match gate {
        GateFade::Emerging { ticks_left } => emerge_level(ticks_left)?,
        GateFade::Entering { ticks_left } => enter_level(ticks_left),
        GateFade::None => return Some(Color::WHITE),
    };
    let level = u16::from(level);
    let full = u16::from(FULL_LEVEL);
    let alpha = u8::try_from((level * 255 + full / 2) / full).unwrap_or(u8::MAX);
    Some(Color::rgba(255, 255, 255, alpha))
}

/// A [`Chance`] shared by whoever holds a copy: the app's one source of
/// randomness, handed to each flight. The default never fires and always
/// rolls the first outcome.
#[derive(Clone)]
pub struct SharedChance(Rc<RefCell<dyn Chance>>);

impl SharedChance {
    /// The source `chance`, shared.
    #[must_use]
    pub fn new(chance: Rc<RefCell<dyn Chance>>) -> Self {
        Self(chance)
    }
}

impl Default for SharedChance {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(NeverFires)))
    }
}

impl std::fmt::Debug for SharedChance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Chance")
    }
}

impl Chance for SharedChance {
    fn fires(&mut self, percent: u8) -> bool {
        self.0.borrow_mut().fires(percent)
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0.borrow_mut().below(n)
    }

    fn roll(&mut self, sides: u16) -> u16 {
        self.0.borrow_mut().roll(sides)
    }
}

/// How long the take-off's held nav lasts ([`FlightView::took_off`]): it
/// shows in the first frame drawn after the take-off, and on to the end of
/// the tick that frame lands in, so about one tick and never less than a
/// frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NavFlash {
    /// Held by the take-off, before any tick has advanced the HUD.
    Held,
    /// Shown from the given tick, the first frame's, through to its end.
    Through(u128),
    /// Shown already, under a mission text's dialog
    /// ([`FlightView::take_mission_texts`]), whose closing redraws the
    /// bar: the next advance ends it.
    UnderText,
}

impl NavFlash {
    /// The flash after the HUD advances to tick `now`: stamped with `now`
    /// on the first advance after the take-off, which comes before the
    /// first frame is drawn; ended on the first advance past that tick, or
    /// on the first after a mission text showed over it.
    fn advance(self, now: u128) -> Option<Self> {
        match self {
            Self::Held => Some(Self::Through(now)),
            Self::Through(tick) => (now <= tick).then_some(self),
            Self::UnderText => None,
        }
    }
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// The flight's frame as it stands now ([`FlightSim::frame`]), filled
    /// afresh on each read, for the nav keys', the map's and the jump
    /// key's reads mid-input. The world reads the frame the view keeps
    /// ([`FlightView::frame`](FlightView#structfield.frame)) instead, but
    /// these stay fresh: while the strangler lasts the concrete blocks still
    /// change the session directly between two of their reads in one input
    /// or tick (a jump begun, a landing, a boarding) without refreshing
    /// the kept frame, which a read here would then find stale. Once every
    /// change goes through [`FlightSim::apply`] and [`FlightSim::step`]
    /// (rdm:phase/view-sim-seam/phase-9-travel-on-port and
    /// rdm:phase/view-sim-seam/phase-11-spaceport-on-port), these can read
    /// the kept frame too.
    pub(super) fn frame_now(&self) -> FlightFrame {
        let mut frame = FlightFrame::default();
        self.sim.frame(&mut frame);
        frame
    }

    /// Fills the frame the view keeps afresh from the flight
    /// ([`FlightSim::frame`]), into its own buffers: after each step,
    /// after each command ([`FlightView::send`]), as the flight is caught
    /// up with, and at the end of each input and tick, so whatever is drawn
    /// next draws the flight as it stands.
    pub(super) fn refresh_frame(&mut self) {
        self.sim.frame(&mut self.frame);
    }

    /// Checks, in this crate's tests, that the kept frame is the flight's
    /// as it stands: every point that changes the flight before a draw
    /// refreshes it ([`FlightView::refresh_frame`]). A test that changes
    /// the session behind the view's back resyncs it first
    /// ([`FlightView::resync`]).
    #[cfg(test)]
    pub(super) fn check_frame_kept(&self) {
        let fresh = self.frame_now();
        if self.frame != fresh {
            let differs = frame_fields_differing(&self.frame, &fresh);
            panic!("the kept frame is stale in {differs:?}");
        }
    }

    /// The player's ship's place in the frame, filled afresh first: for
    /// the concrete code that changes the session and then draws the ship
    /// where it is ([`FlightView::place_ship`], `FlightView::lay_out`,
    /// `FlightView::assign`), which reads it before or without a catch-up.
    pub(super) fn player_place_now(&mut self) -> ShipPlace {
        self.refresh_frame();
        self.frame.player().place()
    }

    /// Sends `command` to the flight ([`FlightSim::apply`]) and gives its
    /// answer, the kept frame filled afresh after it
    /// ([`FlightView::refresh_frame`]).
    pub(super) fn send(
        &mut self,
        command: nova_sim::flight_frame::FlightCommand,
    ) -> nova_sim::flight_frame::Answer {
        let answer = self.sim.apply(command);
        self.refresh_frame();
        answer
    }

    /// Drains the flight into the inbox ([`FlightSim::drain`]) and gives
    /// it, for the caller to take its own kind from at its own place (see
    /// the `inbox` module).
    fn drain(&mut self) -> &mut inbox::Inbox {
        self.inbox.drain(&mut self.sim);
        &mut self.inbox
    }
}

/// The player's ship in flight, reading the game data from `C`, the
/// flight itself through the [`FlightSim`] port `S`: by default the
/// [`SessionFlight`] adapter over a real session.
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)] // each is one fact the screen keeps
pub struct FlightView<C, S: FlightSim = SessionFlight<C>> {
    /// What the screen reads: when it is built, and on each arrival. The
    /// flight holds the same handle for its answers that read the catalog
    /// ([`SessionFlight`]'s, shared as the view is read).
    catalog: Rc<C>,
    /// The flight, with the collaborators its calls are handed.
    sim: S,
    /// What was drained from the flight and not yet taken, by kind (see
    /// the `inbox` module).
    inbox: inbox::Inbox,
    /// What the flight's steps reported on the last frame, kept to fill
    /// again ([`FlightSim::step`]).
    step_report: StepReport,
    /// The rules the steps' effects and stars are shown by, held from the
    /// flight's rulebook ([`stepping::Presentation`]).
    presentation: stepping::Presentation,
    /// The flight's frame as last filled ([`FlightView::refresh_frame`]):
    /// what the world is drawn from, its ships, shots, beams, boxes,
    /// asteroids, stellars, murk and flags.
    frame: FlightFrame,
    /// The session's system, laid out; `None` when the session failed.
    scene: Option<SystemScene>,
    /// Each stellar's animation state as the scene that last showed it
    /// left it: the original's per-`spöb` current frame, next frame and
    /// timer (+0x476, +0x478, +0x490), kept per flight so a stellar shown
    /// again resumes where it stood ([`KeptFrames`], stellar-fidelity
    /// phase 12). A load or a new pilot is a new flight, so every stellar
    /// starts afresh then.
    kept_frames: KeptFrames,
    /// The player ship's sheet, or why it cannot be shown.
    sheet: Result<ShipSheet, String>,
    /// The class `sheet` and `status_bar` were read for; `None` when the
    /// session failed.
    sheet_ship: Option<ShipId>,
    /// The HUD's status bar, or why it cannot be shown.
    status_bar: Result<StatusBar, String>,
    /// The government whose `Interface` `status_bar` was chosen for
    /// ([`FlightView::choose_bar`]); `None` when the session failed.
    bar_government: Option<BarFor>,
    /// Turns frame times into simulation steps.
    clock: FixedStep,
    /// The player's ship's place a step before the frame's.
    previous: ShipPlace,
    /// How far the display is from `previous` to the session's ship.
    alpha: f32,
    /// Time since the view opened, which drives the stellars' animations
    /// and the running lights' blinking. It stops while the map is open.
    elapsed: Duration,
    /// The flight keys held down.
    held: HashSet<Key>,
    /// `_navModeSelectFlag` (0xdee04), the latch the Nav Off and
    /// Hyperspace Mode block and Option-\` share: set as one of them acts,
    /// so a key held acts once; cleared on a frame neither \` without
    /// Option nor H is down, and on every frame of a jump
    /// ([`FlightView::nav_keys_frame`]).
    nav_latch: bool,
    /// `_navTargetSelectFlag` (0xdee48), Hyper Select's latch: set as `\`
    /// acts, cleared on a frame it is up, and left as it is during a jump
    /// ([`FlightView::select_frame`]).
    select_latch: bool,
    /// The stellar slot keys' block: its latch (0xdede0) and the presses
    /// its next frame takes ([`FlightView::slot_keys_frame`]).
    slot_block: slot_block::SlotBlock,
    /// The jump key's latch (0xdee44): set on every frame J runs its
    /// block, a jump's included; cleared on a frame J is up, or down while
    /// the player is not intact ([`FlightView::jump_frame`]).
    jump_latch: bool,
    /// The Board key's latch (0xdee18): set by a board attempt past its
    /// no-target and cloak returns, cleared on a frame B is up
    /// ([`FlightView::board_frame`]).
    board_latch: bool,
    /// The stellar landed on, until the router takes it.
    pending_landing: Option<StellarId>,
    /// The message line's one slot (`_DisplayComm`'s 0x3b565c): a new
    /// message replaces the old and its time. Its ticks count the
    /// flight's, so they stop while the map is open and while the router
    /// holds flight under a dialog, as `_HandleComm` (@0x301b9) counts
    /// only in the flight loop, but for the plunder dialog, whose loop
    /// calls it once a press that leaves it open (@0x949a1; see
    /// [`FlightView::plunder_held`]). The original's
    /// `_HandlePlayerDockRequest` counts one tick more just before
    /// landing (@0x66da5), which no player sees.
    message: Option<Message>,
    /// The port's own overlays drawn.
    overlays: FlightOverlays,
    /// Whether a boarding has opened that the router has not taken.
    pending_boarding: bool,
    /// The hail just answered, for the router.
    pending_hail: Option<HailView>,
    /// The escort dialog a fleet escort hailed just opened, for the
    /// router.
    pending_escort_dialog: Option<nova_sim::escort_dialog::EscortDialogView>,
    /// The mission info [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY) asked the screen be opened
    /// on, for the router.
    pending_mission_info: Option<MissionInfo>,
    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) asked for the player info screen, for
    /// the router.
    pending_player_info: bool,
    /// The mission a person hailed or boarded just offered, for the
    /// router.
    pending_offer: Option<OfferPrompt>,
    /// The session's arrival messages taken before their arrival is
    /// caught up with, which leads them.
    pending_leads: Vec<SimMessage>,
    /// The options the comm dialog lists.
    hail_options: HailOptions,
    /// The course map, shown or not.
    map: GalaxyMap,
    /// Whether the course map is shown.
    map_open: bool,
    /// Whether [`MAP_KEY`] is held while the map is shown, as
    /// `_KeyCheck(prefs+0x12)` reads it for the map key's close
    /// (`_MapFilter` @0x144db-0x14587): from the press that opened it to
    /// its release, and again from any later press of it on the map.
    map_key_held: bool,
    /// How the map shown was opened (by M, the bar or a mission dialog,
    /// with its marks) and the nav as it opened: taken as each map opens,
    /// dropped as it closes ([`FlightView::close_map`]).
    map_opened: Option<MapOpened>,
    /// The borders setting the course map's last undocked close changed
    /// to, not yet taken ([`FlightView::take_map_borders_save`]).
    map_borders_save: Option<bool>,
    /// The mission systems the course map's red arrows mark, taken as it
    /// opens, as `_DoSystemMap` runs `_RecalcMissionSystsForMap` then
    /// (@0x124a0; [`Session::map_mission_systems`]).
    map_missions: Vec<SystemId>,
    /// What the course map's drawing was last taken from
    /// ([`FlightView::redraw_map`]): while the map is open, a change to
    /// any of it takes the drawing again (see `FlightView::catch_up`).
    map_seen: Option<nova_sim::flight_frame::MapKey>,
    /// What the floating map draws, taken at its last show
    /// ([`FlightView::show_floating_map`]).
    floating_picture: Option<MapPicture>,
    /// The jump's effect, until the ship arrives: the warp and its fade to
    /// white.
    jump: Option<JumpEffect>,
    /// The rest of an arrival's effect, once the ship has arrived: the
    /// fade-in from white, or the white flash. Flight goes on under it.
    fade_in: Option<JumpEffect>,
    /// The fade to black after the player's death, once begun
    /// ([`FlightView::begin_fade`]).
    death_fade: Option<ScreenFade>,
    /// Whether jumps play their white fades, as the Hyperspace Effects
    /// preference says.
    hyperspace_effects: bool,
    /// Whether shots leave their particle trails and smoke, as the Smoke
    /// Trails preference says.
    smoke_trails: bool,
    /// Whether each scattering gives the stars their own parallax
    /// factors, as the Parallax Starfield preference says.
    parallax_starfield: bool,
    /// Whether ships show their weapon glow, as the Weapon Effects
    /// preference says.
    weapon_effects: bool,
    /// The player's sound preferences, which say whether the Red Alert
    /// blinks its icon ([`SoundPrefs::alert_quiet`]), as the session is
    /// told.
    sound_prefs: SoundPrefs,
    /// `cïcn` 18000's width, which places the Red Alert icon, or `None`
    /// when it cannot be read, and the icon is not drawn.
    alert_width: Option<u32>,
    /// The view's size in logical units: Nova's 1024 x 768 until the app
    /// widens it ([`FlightView::set_view_size`]; see the module docs).
    view: (f32, f32),
    /// Whether double time is on, as the screen was last told; handed to
    /// each session it flies before its steps. Never let go with the
    /// flight keys.
    double_time: bool,
    /// What the player's engine glow's flicker rolls on: seed 1.
    glow_rolls: HashedRolls,
    /// Each NPC ship type's sheet, or why it cannot be shown, read once.
    npc_sheets: BTreeMap<ShipId, Result<ShipSheet, String>>,
    /// Each NPC's place a step before the frame's, in the system's order
    /// as the step reported them ([`StepRun::before`](nova_sim::flight_frame::StepRun::before)).
    npc_previous: Vec<(NpcId, ShipPlace)>,
    /// The weapons' and explosions' looks, read once.
    looks: Looks,
    /// What could not be read of the looks, until it is taken.
    unread_looks: Vec<Diagnostic>,
    /// Each NPC ship type's target card, read once.
    cards: BTreeMap<ShipId, TargetCard>,
    /// Each NPC government's target code, read once.
    codes: BTreeMap<GovtId, Option<String>>,
    /// The fight's explosions, debris and sounds.
    effects: Effects,
    /// What the effects are rolled on.
    effects_chance: SharedChance,
    /// The original's starfield (see the `stars` module), rolled on the
    /// effects' chance.
    stars: Starfield,
    /// Whether the stars are to be scattered at the next catch-up: once
    /// the flight is built, play entered or a game begun, when its chance
    /// and view are set.
    stars_due: bool,
    /// Each ship's shield flash while its timer runs.
    flashes: BTreeMap<ShipRef, ShieldFlash>,
    /// Each ship's alt layer as last drawn, which a disabled 0x0020 ship
    /// keeps showing ([`HeldAlts`](crate::flight::held_alt::HeldAlts));
    /// kept as each frame draws it.
    held_alts: RefCell<crate::flight::held_alt::HeldAlts>,
    /// Each ship's weapon glow while it is lit ([`WeaponGlows`]).
    glows: WeaponGlows,
    /// What the view keeps of the shots in flight, drawn on the effects'
    /// chance ([`ShotLooks`](shots::ShotLooks)).
    shot_looks: shots::ShotLooks,
    /// What the HUD's text is measured by, if anything.
    metrics: Option<Metrics>,
    /// The escort menu, shut or open.
    escort_menu: EscortMenu,
    /// The interface's beeps not yet taken ([`Screen::take_sounds`](crate::Screen::take_sounds)): the
    /// escort menu's, the floating map's zooms, and the player info key
    /// refused during a jump.
    ui_sounds: Vec<UiSound>,
    /// The floating map shown while H or `\` is held (see the
    /// [`floating`](super::floating) module).
    floating: FloatingMap,
    /// The escort menu's colours, read once.
    escort_colors: EscortMenuColors,
    /// When the HUD refreshes, and the radar target's blink.
    hud_cadence: HudCadence,
    /// What the radar showed at its last refresh, which it shows until the
    /// next under
    /// [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence)'s
    /// engine reading; `None` before the first.
    radar_scene: Option<RadarScene>,
    /// What the nav area showed at its last refresh, and the nav mode it
    /// showed it for, which it shows until the next under
    /// [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence)'s
    /// engine reading; `None` before the first.
    nav_scene: Option<(NavMode, NavDisplay)>,
    /// How long the nav area shows the take-off's snapshot from before its
    /// deselect ([`FlightView::took_off`]); `None` when none is held.
    nav_flash: Option<NavFlash>,
    /// Whether the nav area is marked to redraw at the port's next
    /// `_DrawStatus`, as the shipyard's restore leaves it
    /// (`_statusRedraw[0]` @0x5f8fe; see [`FlightView::close_port_dialog`]).
    nav_marked: bool,
    /// How many mission texts the session had queued when the nav area
    /// was last taken ([`Session::texts_queued`]): a text queued since is
    /// newer than that redraw ([`FlightView::take_mission_texts`]).
    nav_texts: u64,
    /// What the target panel showed at its last redraw, and what it drew
    /// it for, which it shows until the next under
    /// [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence)'s
    /// engine reading; `None` before the first.
    target_scene: Option<(Option<Drawn>, Option<TargetPanel>)>,
    /// The shield, armour and fuel gauges at their last redraw, which
    /// they show while docked until the next under
    /// [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence)'s
    /// engine reading; `None` before the first.
    reserves_scene: Option<Reserves>,
    /// The bar's redraws held for the docked mission texts shown, one for
    /// each, made as each closes ([`FlightView::close_mission_text`]).
    bar_steps: VecDeque<BarStep>,
    /// The last of the texts' bars ([`FlightEvent::MissionShown`](nova_sim::flight_frame::FlightEvent::MissionShown)) staged into
    /// [`FlightView::bar_steps`], by its [`TextBar::texts`].
    bars_staged: u64,
    /// The radar's static this tick, a `ppat` ID, rolled on the effects'
    /// chance ([`radar::static_frame`](super::radar::static_frame)); `None` without.
    radar_static: Option<i16>,
    /// The mouse cursor sprite (see the `cursor` module).
    cursor: HoverCursor,
}

/// The government whose `Interface` a status bar was chosen for
/// ([`FlightView::choose_bar`]), none or one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BarFor(Option<GovtId>);

/// The bar's redraws as a docked mission text closes
/// ([`FlightView::close_mission_text`]).
#[derive(Clone, Debug, PartialEq)]
struct BarStep {
    /// The whole bar as the text noted it as it was queued, which its
    /// caller's `_RefreshSAT` draws as it closes.
    bar: TextBar,
    /// The `C`, `E` and `H` run after the text and before the next that
    /// each changed the `ïntf`: a whole-bar redraw at each operator.
    loads: Vec<InterfaceLoad>,
    /// What the dialog or order whose script queued the text draws once
    /// it has closed, after the last text of its run.
    then: Then,
}

impl BarStep {
    /// Whether it holds the bar's graphic until it closes: a `C`, `E` or
    /// `H` after its text loads another `ïntf`, or a ship purchase's own
    /// load comes after it (@0x5f0ac).
    fn holds_bar(&self) -> bool {
        !self.loads.is_empty() || matches!(self.then, Then::Purchase { .. })
    }
}

/// What a run of docked scripts' caller draws of the bar once its scripts
/// and their mission texts are done ([`FlightView::stage_bar`]): at once,
/// or as its last text closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Then {
    /// Nothing: a landing's settling, an abort, an offer accepted at once.
    Nothing,
    /// The whole bar, as the offer dialog (@0xa2e44, @0xa365c) and the
    /// mission computer's close before its re-entry (@0xa40d7) redraw it.
    Redraw,
    /// An outfitter order's own draw (`_DrawStatus` @0x5c5a1 after a buy,
    /// @0x5d328 after a sale): the radar, and the nav the shipyard's
    /// restore marked when `passed`, the order having gone through
    /// ([`FlightView::after_outfit_order`]).
    Outfit {
        /// Whether the order passed its button's check.
        passed: bool,
    },
    /// A ship purchase's own load and draw: the bar of the class bought
    /// (`_LoadIntfResource` @0x5f0ac), the gauges when its `ïntf` is
    /// another than the one its hooks left loaded (`reserves`), then the
    /// radar, nav and target (`_DrawStatus` @0x5f0b1;
    /// [`FlightView::buy_ship`]).
    Purchase {
        /// Whether the load redraws the whole bar, gauges and all.
        reserves: bool,
    },
}

/// A message on the message line.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Message {
    /// What it says.
    text: String,
    /// The flight's tick it was said on.
    said_at: u128,
    /// How many ticks it stays.
    ticks: u16,
}

/// What [`FlightView::catch_up`] redrew, from the session's events.
// Each flag is one kind of event, folded on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Redrawn {
    /// The ship changed: its sprite sheet followed.
    ship: bool,
    /// A system was entered afresh: it was laid out.
    system: bool,
    /// The ship was put at another stellar of the system shown.
    stellar: bool,
    /// The galaxy overlay changed: the course map was rebuilt.
    galaxy: bool,
    /// A rebuild point changed which stellars are shown, which shown are
    /// dead since it, or the sheet whose frame count one of them animates
    /// over: the stellars were drawn again, each changed one at its
    /// current frame, and read again when they are others.
    rebuilt: bool,
    /// The stars were scattered afresh.
    stars: bool,
}

/// How the flight holds stellar `id` as `frame` records it
/// ([`FrameStellar`](nova_sim::flight_frame::FrameStellar)): destroyed as
/// its galaxy holds it, with the sheet the session says it draws
/// (`nova_sim::Session::drawn_sheet`: its `DeadType` sheet, graphic type
/// 0's or its `Type` sheet, story-runtime phases 7 and 9, stellar-fidelity
/// phase 1), which the click box reads too, and animating over the frame
/// count the session says (`nova_sim::Session::counted_sheet`,
/// stellar-fidelity phase 1), or over none
/// (`nova_sim::Session::counts_no_frames`), its frame frozen when the
/// session says (`nova_sim::Session::frame_frozen`, stellar-fidelity phase
/// 8). A stellar the frame doesn't record, one its system doesn't list,
/// looks whole, on its `Type` sheet, never frozen, as the session holds a
/// stellar it doesn't show.
fn look(frame: &FlightFrame, id: StellarId) -> StellarLook {
    frame.stellar(id).map_or(
        StellarLook {
            destroyed: false,
            drawn: nova_sim::DrawnSheet::Type,
            counted: nova_sim::DrawnSheet::Type,
            frozen: false,
            counts_none: false,
        },
        |stellar| StellarLook {
            destroyed: stellar.destroyed,
            drawn: stellar.drawn,
            counted: stellar.counted,
            frozen: stellar.frozen,
            counts_none: stellar.counts_none,
        },
    )
}

/// Draws each stellar of `scene` as `frame` holds it ([`look`]), as the
/// original's per-frame sprite tick swaps its sheet.
fn show_destroyed(scene: &mut SystemScene, frame: &FlightFrame) {
    scene.set_destroyed(|id| look(frame, id));
}

/// Marks each stellar of `scene` frozen as `frame` holds it
/// ([`FrameStellar::frozen`](nova_sim::flight_frame::FrameStellar::frozen)):
/// after the steps of each tick, as a flight frame freezes or thaws a slot
/// with no event (stellar-fidelity phase 8).
fn show_frozen_frames(scene: &mut SystemScene, frame: &FlightFrame) {
    scene.set_frozen(|id| look(frame, id).frozen);
}

/// Draws each hypergate of `scene` on the frame `frame` gives it
/// ([`FrameStellar::gate_frame`](nova_sim::flight_frame::FrameStellar::gate_frame)),
/// opening while an NPC comes out of it: after each tick of the traffic,
/// and as the system is laid out.
fn show_gate_frames(scene: &mut SystemScene, frame: &FlightFrame) {
    scene.set_gate_frames(|id| frame.stellar(id).and_then(|stellar| stellar.gate_frame));
}

/// The system `frame`'s player is in, read from `catalog`, with only the
/// stellars the frame shows
/// ([`FlightFrame::stellars_shown`](nova_sim::flight_frame::FlightFrame::stellars_shown)),
/// in navigation order.
fn shown_scene(catalog: &impl SystemCatalog, frame: &FlightFrame) -> SystemScene {
    let shown: BTreeSet<StellarId> = frame.stellars_shown().collect();
    SystemScene::load_shown(catalog, frame.system, &shown)
}

/// The scene a flight opens on: the stellars `frame` shows, each
/// destroyed one as it looks destroyed and each hypergate on the frame
/// the session stepped it to.
fn opening_scene(catalog: &impl SystemCatalog, frame: &FlightFrame) -> SystemScene {
    let mut scene = shown_scene(catalog, frame);
    show_destroyed(&mut scene, frame);
    show_gate_frames(&mut scene, frame);
    scene
}

/// The names of the fields `kept` and `fresh` differ in.
#[cfg(test)]
fn frame_fields_differing(kept: &FlightFrame, fresh: &FlightFrame) -> Vec<&'static str> {
    let mut differs = Vec::new();
    let mut check = |name, same: bool| {
        if !same {
            differs.push(name);
        }
    };
    check("nav_mode", kept.nav_mode == fresh.nav_mode);
    check("system", kept.system == fresh.system);
    check("course_shown", kept.course_shown == fresh.course_shown);
    check(
        "course_plotted",
        kept.course_plotted == fresh.course_plotted,
    );
    check("docked", kept.docked == fresh.docked);
    check(
        "jump_under_way",
        kept.jump_under_way == fresh.jump_under_way,
    );
    check(
        "jump_readiness",
        kept.jump_readiness == fresh.jump_readiness,
    );
    check("texts_queued", kept.texts_queued == fresh.texts_queued);
    check("map_key", kept.map_key == fresh.map_key);
    check("ships", kept.ships == fresh.ships);
    check("drawn", kept.drawn == fresh.drawn);
    check("shots", kept.shots == fresh.shots);
    check("beams", kept.beams == fresh.beams);
    check("cargo_boxes", kept.cargo_boxes == fresh.cargo_boxes);
    check("asteroids", kept.asteroids == fresh.asteroids);
    check("stellars", kept.stellars == fresh.stellars);
    check(
        "effective_murk",
        kept.effective_murk == fresh.effective_murk,
    );
    check("system_murk", kept.system_murk == fresh.system_murk);
    check(
        "system_background",
        kept.system_background == fresh.system_background,
    );
    check("double_time", kept.double_time == fresh.double_time);
    check(
        "hyper_gamma",
        kept.hyper_gamma.to_bits() == fresh.hyper_gamma.to_bits(),
    );
    check("traffic_ships", kept.traffic_ships == fresh.traffic_ships);
    check("hud", kept.hud == fresh.hud);
    check("target", kept.target == fresh.target);
    differs
}

/// Text metrics, shared.
#[derive(Clone)]
struct Metrics(Rc<dyn TextMetrics>);

impl std::fmt::Debug for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Metrics")
    }
}

/// A ship's box drawn at `at` (in system coordinates) of `size`.
fn drawn_box(at: Point, (width, height): (u32, u32)) -> DrawnBox {
    DrawnBox {
        centre: Vec2::new(at.x, at.y),
        width,
        height,
    }
}

/// The frame size of an NPC drawn by `sheet`, the one read for its ship
/// type if any: its crossed box's when the sheet could not be read.
fn drawn_size(sheet: Option<&Result<ShipSheet, String>>) -> (u32, u32) {
    match sheet {
        Some(Ok(sheet)) => (sheet.frame_width, sheet.frame_height),
        _ => placeholder_size(),
    }
}

/// The crossed box's size, as a frame's.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn placeholder_size() -> (u32, u32) {
    let side = PLACEHOLDER_SIZE as u32;
    (side, side)
}

/// The simulation's `v` as a view point.
fn point(v: Vec2) -> Point {
    Point::new(v.x, v.y)
}

/// Shows the fight's events a pass raised, `combat`, in `scene`: each on
/// `effects`, as `looks` says, drawing on `chance`, and each hit on its
/// ship's flash among `flashes`, which then step a frame by the pass's
/// `ships` ([`step_flashes`]). An NPC's shot hits land before its
/// `_HandleShipDisplay`, so they show this frame; a hit the original lands
/// after its ship's display ([`CombatEvent::Hit`]'s `late`) is shown on the
/// next. Gives the weapon glows the fight lit, each ship's and whether it
/// is late ([`CombatEvent::WeaponGlow`]), for [`WeaponGlows::step`].
fn show_combat(
    (combat, ships): (&[CombatEvent], &[FrameShip]),
    scene: &Scene,
    (effects, flashes): (&mut Effects, &mut BTreeMap<ShipRef, ShieldFlash>),
    (looks, sheets): (&Looks, Sheets<'_>),
    chance: &mut dyn Chance,
) -> Vec<(ShipRef, bool)> {
    let mut late = Vec::new();
    let mut glows = Vec::new();
    for event in combat {
        if let CombatEvent::WeaponGlow { ship, late } = *event {
            glows.push((ship, late));
        }
        if let CombatEvent::Hit {
            ship,
            passes_shields,
            late: shown_late,
            ..
        } = *event
        {
            if shown_late {
                late.push((ship, passes_shields));
            } else {
                flashes.entry(ship).or_default().hit(passes_shields);
            }
        }
        effects.apply(event, scene, looks, chance);
    }
    step_flashes(flashes, ships, sheets);
    for (ship, passes_shields) in late {
        flashes.entry(ship).or_default().hit(passes_shields);
    }
    glows
}

/// How fast `ship`'s weapon glow falls a frame, out of 32
/// ([`ShipSheet::weapon_decay`]): its class's, from `player`'s sheet or,
/// for an NPC among a pass's `ships`, the NPCs' `sheets`, none for a sheet
/// not read; `None` for an NPC no longer in the system.
fn glow_decay(
    ships: &[FrameShip],
    player: &Result<ShipSheet, String>,
    sheets: &BTreeMap<ShipId, Result<ShipSheet, String>>,
    ship: ShipRef,
) -> Option<f32> {
    let sheet = match ship {
        ShipRef::Player => player.as_ref().ok(),
        ShipRef::Npc(_) => {
            let npc = ships.iter().find(|shown| shown.ship == ship)?;
            sheets.get(&npc.class).and_then(|sheet| sheet.as_ref().ok())
        }
    };
    Some(sheet.map_or(0.0, ShipSheet::weapon_decay))
}

/// The player's sheet and each NPC ship type's, or why each cannot be
/// read.
type Sheets<'a> = (
    &'a Result<ShipSheet, String>,
    &'a BTreeMap<ShipId, Result<ShipSheet, String>>,
);

/// Steps each ship's shield flash among `flashes` a frame by its shield
/// among a pass's `ships` ([`ShieldFlash::step`]), whether its class's
/// sheet among `sheets` has a shield layer, and its class's own loaded
/// `Shield` (class +0x5c @0x2c8aa, [`FrameShip::class_shield`]), dropping
/// the flashes of ships no longer in the system and those run out.
fn step_flashes(
    flashes: &mut BTreeMap<ShipRef, ShieldFlash>,
    ships: &[FrameShip],
    (player, sheets): Sheets<'_>,
) {
    flashes.retain(|&ship, flash| {
        let Some(shown) = ships.iter().find(|shown| shown.ship == ship) else {
            return false;
        };
        let sheet = match ship {
            ShipRef::Player => player.as_ref().ok(),
            ShipRef::Npc(_) => sheets
                .get(&shown.class)
                .and_then(|sheet| sheet.as_ref().ok()),
        };
        let has_layer = sheet.is_some_and(|sheet| sheet.shield.is_some());
        let base = shown.class_shield.unwrap_or(shown.shield.max);
        flash.step(shown.shield.now, base, has_layer);
        flash.shown() > 0.0
    });
}

/// The ships among a pass's `ships` breaking up, each with its sprite's
/// width from `player`'s sheet or the NPCs' `sheets` (a placeholder's
/// without one).
fn dying(
    ships: &[FrameShip],
    player: &Result<ShipSheet, String>,
    sheets: &BTreeMap<ShipId, Result<ShipSheet, String>>,
) -> Vec<Dying> {
    let width = |sheet: Option<&Result<ShipSheet, String>>| match sheet {
        Some(Ok(sheet)) => sheet.frame_width as f32,
        _ => PLACEHOLDER_SIZE,
    };
    ships
        .iter()
        .filter_map(|ship| match ship.condition {
            Condition::Dying {
                ticks_left,
                starting,
            } => Some(Dying {
                at: point(ship.position),
                sprite_width: width(match ship.ship {
                    ShipRef::Player => Some(player),
                    ShipRef::Npc(_) => sheets.get(&ship.class),
                }),
                explosion: ship.breakup,
                ticks_left,
                starting,
            }),
            _ => None,
        })
        .collect()
}

/// Each of `shown` the radar shows, where it is drawn, whether it is the
/// player's `target`, its IFF code and whether it is heavy: those its
/// record says are on the radar
/// ([`FrameShip::on_radar`], `nova_sim::Session::on_radar`), a cloaked
/// ship only when its cloak shows there or the player scans for cloaked
/// ships.
fn radar_ships(target: Option<NpcId>, shown: &[(&FrameShip, Point, f32)]) -> Vec<RadarShip> {
    shown
        .iter()
        .filter(|(npc, _, _)| npc.on_radar)
        .map(|&(npc, at, _)| RadarShip {
            position: at,
            targeted: target.is_some_and(|id| npc.ship == ShipRef::Npc(id)),
            iff: npc.iff,
            heavy: npc.mass > HEAVY_MASS,
        })
        .collect()
}

/// The class `Mass` a ship must be above to show as a square under a
/// density scanner (`cmpw $0x63` @0x49bb8-0x49c2f).
const HEAVY_MASS: f32 = 99.0;

/// The play area about the ship, in world units, in a view `view` in size:
/// the view left of the status bar, x from 0 to width − 194 as
/// `_SetupSpriteWorld` makes the sprite world (@0x21b54-0x21ba4), the ship
/// drawn at `anchor`, as the flight's camera draws it. The session reads it
/// to tell whether a ship is on screen, for cloaks, scanners and observed
/// goals, the brackets and the asteroid field.
fn play_area(view: (f32, f32), anchor: Anchor) -> nova_sim::cloak::Area {
    let camera = Camera::centred_on_in(Point::default(), view).with_anchor(anchor);
    let top_left = camera.screen_to_world(Point::default());
    let bottom_right = camera.screen_to_world(Point::new(view.0 - STATUS_BAR_WIDTH, view.1));
    nova_sim::cloak::Area {
        left: top_left.x,
        top: top_left.y,
        right: bottom_right.x,
        bottom: bottom_right.y,
    }
}

/// The game window's width, in a view `view` in size, whole, that each
/// system's `Asteroids` is stored by
/// (`nova_sim::asteroid::FieldSpec::stored`): the view's whole width, the
/// original's window drawing flight one pixel to a logical unit. Our
/// window can be resized in play, where the original's can't; the session
/// reads the width it is told last (see `nova_sim::Session::set_window_width`).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a view's width, positive and far below u32::MAX
fn window_width(view: (f32, f32)) -> u32 {
    view.0 as u32
}

/// The run of an outfitter order a session that failed gives: no
/// outfitter, refused at the first pass with no alert, its button
/// failing.
fn no_outfitter() -> OutfitRun {
    OutfitRun {
        run: OrderRun {
            done: 0,
            stopped: Some(Stopped {
                pass: 0,
                refusal: OutfitRefusal::NoOutfitter,
            }),
        },
        alert: None,
        after: Reselection::Kept,
        button_passed: false,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests;
