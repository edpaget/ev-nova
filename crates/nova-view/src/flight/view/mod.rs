//! The flight screen: the player's ship flying from its starting system,
//! over the parallax starfield and among the system's stellars, with the
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
//! [`Session::tick_traffic`]), and as the ship takes off
//! ([`Session::take_off`]), rolled on the screen's [`SharedChance`].
//! Each time, the sprite sheet of each ship type the traffic can spawn is
//! read, once per type for the life of the screen. After each step of the
//! player's ship, the traffic takes a step,
//! its NPCs deciding as the screen's [`Behaviour`] says
//! ([`FlightView::with_behaviour`]; [`NovaAi`](nova_sim::NovaAi) by default), and then the
//! fight ([`Session::tick_combat`]), its ships disabled as the screen's
//! [`DisableRule`] says ([`FlightView::with_disable_rule`];
//! [`NovaDisable`](nova_sim::NovaDisable) by default), their point defence engaging every
//! missile fired at them, or by the other reading of the `pd_hostility`
//! rule those its [`PointDefenceRule`] calls hostile
//! ([`FlightView::with_point_defence_rule`]; [`Allegiance`](nova_sim::Allegiance) by default),
//! and the player's crimes judged by its [`LegalCode`]
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
//! pass's calls, and steps the effects and
//! drains the fight's events after it as after the advance pass
//! (`_HandleExplods` runs on both, @0x444e1). The second pass answers
//! its hits ([`Session::answer_strikes`]) and says the attack quotes they
//! raise, then rolls the hail quotes ([`Session::tick_quotes_again`]),
//! as `_HandleShot` (@0x44385) and `_HandleShip` (@0x44483) run on both
//! passes; it gives no help and tells no comm. The mode is the screen's own
//! and outlives the hails, the map, the jumps and the sessions it flies;
//! while it is on, `cïcn` 20000 is drawn at the top left over the Red
//! Alert icon (see the `alert` module).
//!
//! Each step's fight events are drained into the [`Effects`]: its
//! explosions, debris and sounds, rolled on their own [`SharedChance`]
//! ([`FlightView::with_effects_chance`]), never on the simulation's. The
//! beams live before the fight's step are passed along, so a looped
//! weapon is heard once a beam. The fight is drawn over the stellars:
//! the asteroid field ([`asteroids`](super::asteroids), smoothed as the cargo boxes are),
//! the cargo boxes a jettison left ([`boxes`](super::boxes), smoothed as the shots
//! are), beams that go under the ships, then the ships, then the shots
//! (each smoothed as the ships are) and the other beams, then the
//! explosions and debris, then the brackets round the target
//! ([`weapons`](super::weapons), [`effects`](super::effects), [`target`](super::target)). The HUD shows
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
//! says why, and flight goes on. By [`RuleKey::HudCadence`]'s engine
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
//! scanner ([`Session::radar_gear`]); far from the system's centre an
//! arrow points back to it; and each tick the interference in the system
//! ([`Session::interference`]) may fill it with static, rolled on the
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
//! out. The lights are drawn at the level [`nova_sim::lights_level`]
//! gives for the sheet's blink at the flight's time in ticks. Each is
//! drawn at [`lights_tint`](crate::draw::lights_tint) of its level, or not at all while it is hidden
//! or off. Both are combined by OR with what is beneath
//! ([`Blend::Or`](crate::Blend::Or)) at every level, scaled by level/32,
//! like the original's `_BlitPixieRLETranslucent` (0xc1568) below full and
//! `_BlitPixieRLEAddOver` (0xc24bf) at full: see [`lights_tint`](crate::draw::lights_tint) for the
//! full record. The flight's time
//! stops while the course map is open, so the lights blink and the glow
//! flickers in game time. Random blinking rolls on [`HashedRolls`] from
//! seed 0, and the glow's flicker on [`HashedRolls`] from seed 1. A layer
//! that cannot be shown is left out silently.
//!
//! Input, the original's default keys:
//!
//! - Up thrusts, Left and Right turn, and Down turns to face against the
//!   ship's motion, while held: a press (or its key repeats) holds the key
//!   and its release lets it go. Left and Right together cancel, and
//!   either overrides Down.
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
//!   the galaxy, the explored or known systems, the course or the nav
//!   change, and after a plot, a Tab or a hypergate map click. A system
//!   clicked on the map becomes the destination: the session plots the
//!   course there and the map shows it, and the hop it cues as the
//!   selected link. The panel says which systems are unexplored. The
//!   map's nebulae are the session's galaxy's ([`Session::galaxy`]),
//!   active and explored, rebuilt on the frame after each change to it;
//!   its panel's course line is the course as it is flown
//!   ([`Session::course_shown`]), each hop the visible system standing
//!   for it.
//! - The HUD's nav area shows the session's nav mode and its selection
//!   ([`Session::nav_mode`]): the nav system off, a stellar, or a system
//!   (see [`hud`](super::hud)). As in the original
//!   (`_HandlePlayer` @0x68390; see [`navigation`](nova_sim::navigation)),
//!   1-4 and F1-F4 ([`STELLAR_SLOT_KEYS`], presses) select the stellar in
//!   that slot of the system's nav defaults, 1-4 only while the escort
//!   menu is shut; F5 ([`NEAREST_STELLAR_KEY`]) the nearest stellar,
//!   as L does with none. A left click in space
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
//!   intact again. The stars streak when the session says
//!   the jump has begun ([`Session::jumping`]), at once for a ship already
//!   facing that way and slow enough. The jump plays its [`JumpEffect`]:
//!   the session waits while the stars streak and the screen, HUD
//!   included, fades to white, as the original's whole-display fade does;
//!   then the ship arrives at full white, the new system is read and laid
//!   out, and the message line says so in the original's words
//!   ([`arrival_message`]). Flight resumes as the new system fades in
//!   from white: every key works again and the session flies, J included,
//!   as under the original's asynchronous display fade. A multi-jump plays one
//!   effect, toward the first system, and the scene loaded is the last
//!   system it passes. With the Hyperspace Effects preference off
//!   ([`FlightView::with_hyperspace_effects`],
//!   [`FlightView::set_hyperspace_effects`]) a jump skips the fades, as
//!   the original's does: the stars streak, the ship arrives as the streak
//!   ends, and the arrival frame shows solid white once, with flight
//!   going on under it.
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
//!   ([`Session::board`]), by the screen's [`BoardingRule`]
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
//!   or as it was when the jump began once the stars streak.
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
//!   through [`FlightView::hang_up`]. Each step, the NPCs assisting the
//!   player give their help ([`Session::tick_assistance`], by the screen's
//!   [`DisableRule`]), and each says when it is done, as a message
//!   ([`comm_message`]). During a jump, its pre-jump stage included, Y is
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
//!   the ship braking and turning for it or the stars streaking, the
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
//!   [`HAIL_QUOTE_TICKS`], the last one said winning.
//! - Each step's fight starts with the asteroid field's work
//!   ([`Session::tick_asteroids`], which [`Session::tick_combat`] runs
//!   first): a spawn at the field's edge and the field stepped, on the
//!   flight's chance, so the shots meet the asteroids where they are
//!   drawn. Before any step, and so
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
//! font at 12 pt in white ([`hud::draw_message`](super::hud::draw_message)). It is drawn over the
//! starfield and under everything else, as the original draws it on the
//! sprite world's background. Each message stays for as many ticks of
//! flight as its original caller asks ([`MESSAGE_TICKS`] and the
//! constants after it, [`SimMessage::ticks`]). The dialogs drawn over
//! flight (the comm, offer and plunder dialogs, the map) leave it as it
//! is, and it shows again afterwards for the ticks it had left, as
//! `_RedrawComm` (@0x903a0) redraws it. Starting a jump clears it only
//! when it says [`TOO_CLOSE`], [`FAR_ENOUGH`] or [`NOTHING_OUT_HERE`]
//! (`_HandlePlayer` @0x6b231-0x6b3d9), and
//! opening the escort menu only when it says [`NO_ESCORTS`](super::escorts::NO_ESCORTS)
//! (@0x6dedb-0x6df29). The system's title and the [`HELP`] line are the
//! port's own, drawn only when turned on ([`FlightOverlays`],
//! [`FlightView::with_overlays`]).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::rc::Rc;
use std::time::Duration;

use nova_sim::cloak::{CloakShown, TRANSLUCENCY_STEPS};
use nova_sim::order_loop::{OrderRun, Stopped};
use nova_sim::traffic::emerge::{FULL_LEVEL, emerge_level};
use nova_sim::traffic::gate_entry::enter_level;
use nova_sim::traffic::npc::Mode;
use nova_sim::{
    Behaviour, BoardingRule, Chance, CombatEvent, Condition, DisableRule, FixedStep, GovtId,
    HailOptions, HailView, HashedRolls, LegalCode, NavMode, NeverFires, Npc, NpcId, OutfitRefusal,
    OutfitRun, PointDefenceRule, Reselection, Reserves, RuleKey, Session, ShipId, ShipRef,
    ShipState, SimMessage, StellarId, SystemId, Vec2, flight::normalized, flight::shortest_turn,
};
use nova_sim::{DrawnBox, DrawnShips};
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
use super::target::TargetPanel;
use crate::fade::ScreenFade;
use crate::galaxy::GalaxyMap;
use crate::galaxy::map::MapPicture;
use crate::system::camera::{Camera, VIEW_SIZE};
use crate::system::catalog::SystemCatalog;
use crate::system::scene::{PLACEHOLDER_SIZE, StellarLook, SystemScene};
use crate::text::TextMetrics;
use crate::{Color, Diagnostic, Key, Point, SoundPrefs, UiSound};

mod build;
mod cockpit;
mod docked;
mod interface;
mod messages;
mod readouts;
mod screen;
mod travel;

pub use messages::*;

/// The developer overlays: the system's title and the help line.
const TITLE: Point = Point::new(16.0, 32.0);
const TITLE_SIZE: f32 = 20.0;
const HELP_AT: Point = Point::new(16.0, 744.0);
const OVERLAY_SIZE: f32 = 14.0;
/// How far below the ship's placeholder the reason goes.
const MESSAGE_GAP: f32 = 22.0;
/// The help line.
pub const HELP: &str = "Arrows: fly   Space: fire   Ctrl: secondary   W: weapon   Tab: next target   R: nearest   B: board   Y: hail   Cmd-K: jettison   F/D/V/C: escorts   Alt-C: dock   E: escort menu   L: land   M: map   J: jump   \\: next system   P: player info   I: mission info   Cmd-,: preferences   Esc: leave";
/// The port's own overlays on the flight screen, which the original does
/// not draw: developer settings, both off by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlightOverlays {
    /// The system's name and `sÿst` ID at the top left.
    pub title: bool,
    /// The [`HELP`] line of keys at the bottom.
    pub help: bool,
}

/// The keys flight holds: the original's defaults, Alt, which turns the
/// weapon select key back, Shift, which turns the target select key back,
/// Command, which the jettison and eject keys need, the eject key, which
/// works only held with Command, the land key, which holds the dock
/// approach back while it is down, H and `\`, which show the floating map
/// while they are down, and \` and J. H, `\`, \` and J act again when
/// held across a jump's end, a release of Option or a repair, as the
/// original polls them with a latch each ([`FlightView::held_keys_frame`]);
/// the original's `_quickKeyCheckResult` counts \` and J as held too
/// (@0x69aa1, @0x6b0e6), which the floating map's zoom keys read.
const FLIGHT_KEYS: [Key; 15] = [
    Key::Up,
    Key::Left,
    Key::Right,
    Key::Down,
    FIRE_KEY,
    SECONDARY_KEY,
    Key::Alt,
    Key::Shift,
    Key::Command,
    LAND_KEY,
    EJECT_KEY,
    HYPERSPACE_MODE_KEY,
    HYPER_SELECT_KEY,
    NAV_OFF_KEY,
    JUMP_KEY,
];

/// The primary fire key: the original's default (`fireKey0`, `_loadKeys`
/// @0xcd5f8).
pub const FIRE_KEY: Key = Key::Space;
/// The secondary fire key: the original's default (`fireKey1`).
pub const SECONDARY_KEY: Key = Key::Control;
/// The secondary weapon select key: the original's default
/// (`weapSelect`); with Alt (Option) it selects the one before.
pub const SELECT_KEY: Key = Key::Char('w');
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
/// @0x69cf6-0x69f17. While the escort menu is open, 1-4 pick its groups
/// instead ([`GROUP_KEYS`](super::escorts::GROUP_KEYS)); F1-F4 always pick a stellar.
pub const STELLAR_SLOT_KEYS: [[Key; 2]; 4] = [
    [Key::Char('1'), Key::Function(1)],
    [Key::Char('2'), Key::Function(2)],
    [Key::Char('3'), Key::Function(3)],
    [Key::Char('4'), Key::Function(4)],
];
/// The nearest stellar key: F5 (Mac keycode 0x60), which the original
/// reads beside the slot keys (@0x69cf6-0x69f17) and which selects the
/// stellar its land key would with nothing selected. Its Cmd-5 twin has
/// no key here.
pub const NEAREST_STELLAR_KEY: Key = Key::Function(5);
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
fn shown_position(from: &ShipState, to: &ShipState, alpha: f32) -> Point {
    let (from, to) = (from.position, to.position);
    Point::new(
        (to.x - from.x).mul_add(alpha, from.x),
        (to.y - from.y).mul_add(alpha, from.y),
    )
}

/// Which way a ship is drawn facing `alpha` of the way, the short way
/// round, from `from` to `to`.
fn shown_heading(from: &ShipState, to: &ShipState, alpha: f32) -> f32 {
    normalized(shortest_turn(from.heading, to.heading).mul_add(alpha, from.heading))
}

/// How `npc`'s sprite is tinted: `None` while it waits hidden at the gate
/// it comes out of; white with the alpha of its [`emerge_level`] over
/// its last [`FADE_TICKS`](nova_sim::traffic::emerge::FADE_TICKS) there;
/// white with the alpha of its [`enter_level`] while it enters a gate to
/// leave, fading out; plain white otherwise, held at its gate by the
/// player's hit too (its goal 4 is not the fade's goal 0x15).
///
/// The original (`_HandleShipDisplay` @0x2c090-0x2c18e) fades the hull in
/// at 16-bit depth by a destination factor of twice the ticks left and a
/// bias of four times them, at most 32, and out again entering a gate
/// (@0x2bf94-0x2c088). Those translucent blits were not traced; this
/// draws the ship's alpha rising from none to full, or falling, as the
/// cloak's fade does. An NPC gets no screen flash, as the player's own
/// gate arrival does (see [`jump`](super::jump)).
fn npc_tint(npc: &Npc) -> Option<Color> {
    let level = match npc.mode {
        Mode::Emerging { ticks_left } => emerge_level(ticks_left)?,
        Mode::EnteringGate { ticks_left, .. } => enter_level(ticks_left),
        _ => return Some(Color::WHITE),
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

/// What the course map's drawing is taken from
/// ([`Session::map_drawing`]): while the map is open, a change to any of
/// it takes the drawing again (see `FlightView::catch_up`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct MapSeen {
    system: SystemId,
    explored: Vec<SystemId>,
    known: BTreeSet<SystemId>,
    course: Vec<SystemId>,
    nav: NavMode,
}

impl MapSeen {
    /// What `session` would draw the map from now.
    fn of(session: &Session) -> Self {
        Self {
            system: session.system(),
            explored: session.pilot().explored().collect(),
            known: session.known().clone(),
            course: session.course().to_vec(),
            nav: session.nav_mode(),
        }
    }
}

/// The player's ship in flight, reading the game data from `C`.
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)] // each is one fact the screen keeps
pub struct FlightView<C> {
    /// What the screen reads: when it is built, and on each arrival.
    catalog: C,
    /// The flight, or why it could not start.
    session: Result<Session, String>,
    /// The session's system, laid out; `None` when the session failed.
    scene: Option<SystemScene>,
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
    /// The player's ship as it was a step before the session's.
    previous: ShipState,
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
    /// The jump key's latch (0xdee44): set on every frame J runs its
    /// block, a jump's included; cleared on a frame J is up, or down while
    /// the player is not intact ([`FlightView::jump_frame`]).
    jump_latch: bool,
    /// The stellar landed on, until the router takes it.
    pending_landing: Option<StellarId>,
    /// The message line's one slot (`_DisplayComm`'s 0x3b565c): a new
    /// message replaces the old and its time. Its ticks count the
    /// flight's, so they stop while the map is open and while the router
    /// holds flight under a dialog, as `_HandleComm` (@0x301b9) counts
    /// only in the flight loop. The original's `_HandlePlayerDockRequest`
    /// counts one tick more just before landing (@0x66da5), which no
    /// player sees.
    message: Option<Message>,
    /// The port's own overlays drawn.
    overlays: FlightOverlays,
    /// Whether a boarding has opened that the router has not taken.
    pending_boarding: bool,
    /// The hail just answered, for the router.
    pending_hail: Option<HailView>,
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
    /// Who repels boarders, the capture odds and the capture roll.
    boarding_rule: Rc<dyn BoardingRule>,
    /// The course map, shown or not.
    map: GalaxyMap,
    /// Whether the course map is shown.
    map_open: bool,
    /// Whether [`MAP_KEY`] is held while the map is shown, as
    /// `_KeyCheck(prefs+0x12)` reads it for the map key's close
    /// (`_MapFilter` @0x144db-0x14587): from the press that opened it to
    /// its release, and again from any later press of it on the map.
    map_key_held: bool,
    /// Whether the map shown is the bar's ([`FlightView::open_bar_map`]),
    /// whose close puts back the nav the bar opened with; cleared as the
    /// map closes ([`FlightView::close_map`]).
    bar_map: bool,
    /// The mission systems the course map's red arrows mark, taken as it
    /// opens, as `_DoSystemMap` runs `_RecalcMissionSystsForMap` then
    /// (@0x124a0; [`Session::map_mission_systems`]).
    map_missions: Vec<SystemId>,
    /// What the course map's drawing was last taken from
    /// ([`FlightView::redraw_map`]): while the map is open, a change to
    /// any of it takes the drawing again.
    map_seen: Option<MapSeen>,
    /// What the floating map draws, taken at its last show
    /// ([`FlightView::show_floating_map`]).
    floating_picture: Option<MapPicture>,
    /// The jump's effect, while it holds flight: the streak and the
    /// fade-out, until the ship arrives.
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
    /// The player's sound preferences, which say whether the Red Alert
    /// blinks its icon ([`SoundPrefs::alert_quiet`]), as the session is
    /// told.
    sound_prefs: SoundPrefs,
    /// `cïcn` 18000's width, which places the Red Alert icon, or `None`
    /// when it cannot be read, and the icon is not drawn.
    alert_width: Option<u32>,
    /// Whether double time is on, as the screen was last told; handed to
    /// each session it flies before its steps. Never let go with the
    /// flight keys.
    double_time: bool,
    /// What each day's events and the traffic are rolled on.
    chance: SharedChance,
    /// What random running lights roll on.
    blink_rolls: HashedRolls,
    /// What the engine glow's flicker rolls on: seed 1, so it never
    /// mirrors random-mode lights, which roll on seed 0.
    glow_rolls: HashedRolls,
    /// How the NPCs decide.
    behaviour: Rc<dyn Behaviour>,
    /// When a ship in the fight is disabled.
    disable_rule: Rc<dyn DisableRule>,
    /// Which missiles the ships' point defence engages.
    defence_rule: Rc<dyn PointDefenceRule>,
    /// What the player's crimes do to its legal record.
    law: Rc<dyn LegalCode>,
    /// Each NPC ship type's sheet, or why it cannot be shown, read once.
    npc_sheets: BTreeMap<ShipId, Result<ShipSheet, String>>,
    /// Each NPC as it was a step before the session's.
    npc_previous: BTreeMap<NpcId, ShipState>,
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
    /// Each ship's shield flash while its timer runs.
    flashes: BTreeMap<ShipRef, ShieldFlash>,
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
    /// next under [`RuleKey::HudCadence`]'s engine reading; `None` before
    /// the first.
    radar_scene: Option<RadarScene>,
    /// What the nav area showed at its last refresh, and the nav mode it
    /// showed it for, which it shows until the next under
    /// [`RuleKey::HudCadence`]'s engine reading; `None` before the first.
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
    /// [`RuleKey::HudCadence`]'s engine reading; `None` before the first.
    target_scene: Option<(Option<Drawn>, Option<TargetPanel>)>,
    /// The shield, armour and fuel gauges at their last redraw, which
    /// they show while docked until the next under
    /// [`RuleKey::HudCadence`]'s engine reading; `None` before the first.
    reserves_scene: Option<Reserves>,
    /// The bar's redraws held for the docked mission texts shown, one for
    /// each, made as each closes ([`FlightView::close_mission_text`]).
    bar_steps: VecDeque<BarStep>,
    /// The last of [`Session::text_bars`] staged into
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
    /// A rebuild point changed which stellars are shown, or which shown
    /// are dead since it: the stellars were drawn again, and read again
    /// when they are others.
    rebuilt: bool,
}

/// Draws each stellar of `scene` as `session` holds it: by whether its
/// galaxy holds it destroyed, its `DeadType` sheet or its `Type` sheet, as
/// the original's per-frame sprite tick picks them (story-runtime phase
/// 7), and, by the session's reading of [`RuleKey::DeadTypeFallback`],
/// graphic type 0's sheet for one with no `DeadType` sheet that has been
/// dead since the last rebuild point (story-runtime phase 9).
fn show_destroyed(scene: &mut SystemScene, session: &Session) {
    scene.set_destroyed(
        |id| StellarLook {
            destroyed: session.galaxy().destroyed(id),
            dead_since_rebuild: session.dead_since_rebuild(id),
        },
        session.rules().source_for(RuleKey::DeadTypeFallback),
    );
}

/// Draws each hypergate of `scene` on the frame `session` gives it
/// (`Session::gate_frame`), opening while an NPC comes out of it: after
/// each tick of the traffic, and as the system is laid out.
fn show_gate_frames(scene: &mut SystemScene, session: &Session) {
    scene.set_gate_frames(|id| session.gate_frame(id));
}

/// The system `session`'s player is in, read from `catalog`, with only the
/// stellars the session shows ([`Session::stellars_shown`]), in navigation
/// order.
fn shown_scene(catalog: &impl SystemCatalog, session: &Session) -> SystemScene {
    let shown: BTreeSet<StellarId> = session.stellars_shown().collect();
    SystemScene::load_shown(catalog, session.system(), &shown)
}

/// The scene a flight opens on: the stellars `session` shows, each
/// destroyed one as it looks destroyed and each hypergate on the frame
/// the session stepped it to.
fn opening_scene(catalog: &impl SystemCatalog, session: &Session) -> SystemScene {
    let mut scene = shown_scene(catalog, session);
    show_destroyed(&mut scene, session);
    show_gate_frames(&mut scene, session);
    scene
}

/// Text metrics, shared.
#[derive(Clone)]
struct Metrics(Rc<dyn TextMetrics>);

impl std::fmt::Debug for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Metrics")
    }
}

/// The ships where the flight draws them, for the hover test: the
/// session's [`DrawnShips`] port. Every NPC is drawn, by its sheet or as
/// its crossed box, so each has a box, even one its cloak hides from the
/// player: [`Session::hover`] asks the cloak itself. Each box is the size
/// of a frame of the sheet its slot's sprite holds
/// ([`Session::framed_by`]): a shown ship's own type's. A hidden ship
/// ([`Session::is_hidden`]: its type's `Cost` is -1) is boxed where it
/// would be drawn, as the original still positions its hidden sprite
/// (`_HandleShipDisplay` @0x2cd17) and `_HandleMouse` skips only a slot
/// with no sprite (@0x38171), by the frames its slot last cloned, as
/// `_SWCloneFramesFromMaster` (@0x2b824) is skipped for it: often the
/// previous ship's at that slot ([`nova_sim::RuleKey::HiddenShipHoverBox`]
/// chooses).
struct ViewShips<'a, C> {
    /// The flight, which says where each ship is drawn.
    view: &'a FlightView<C>,
    /// The flight's session.
    session: &'a Session,
}

impl<C> DrawnShips for ViewShips<'_, C> {
    fn npc(&self, id: NpcId) -> Option<DrawnBox> {
        let npc = self.session.npcs().iter().find(|npc| npc.id == id)?;
        let (at, _) = self.view.shown_npc(npc);
        let framed = self.session.framed_by(ShipRef::Npc(id))?;
        let sheet = self.view.npc_sheets.get(&framed);
        Some(drawn_box(at, drawn_size(sheet)))
    }

    fn player(&self) -> Option<DrawnBox> {
        let framed = self.session.framed_by(ShipRef::Player);
        let size = if framed == Some(self.session.ship()) {
            match &self.view.sheet {
                Ok(sheet) => (sheet.frame_width, sheet.frame_height),
                Err(_) => placeholder_size(),
            }
        } else {
            drawn_size(framed.and_then(|ship| self.view.npc_sheets.get(&ship)))
        };
        Some(drawn_box(self.view.shown_position(), size))
    }
}

/// [`ViewShips`]' boxes as they stand, taken so the session can be
/// changed by what they decide: a click in space.
struct DrawnNow {
    /// Each NPC's box, by id.
    npcs: BTreeMap<NpcId, DrawnBox>,
    /// The player's box.
    player: Option<DrawnBox>,
}

impl DrawnNow {
    /// The boxes `ships` gives now, for every NPC of its session.
    fn of<C>(ships: &ViewShips<'_, C>) -> Self {
        Self {
            npcs: ships
                .session
                .npcs()
                .iter()
                .filter_map(|npc| Some((npc.id, ships.npc(npc.id)?)))
                .collect(),
            player: ships.player(),
        }
    }
}

impl DrawnShips for DrawnNow {
    fn npc(&self, id: NpcId) -> Option<DrawnBox> {
        self.npcs.get(&id).copied()
    }

    fn player(&self) -> Option<DrawnBox> {
        self.player
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

/// Shows the fight's events `session` raised this pass in `scene`: each on
/// `effects`, as `looks` says, drawing on `chance`, and each hit on its
/// ship's flash among `flashes`, which then step a frame
/// ([`step_flashes`]). An NPC's shot hits land before its
/// `_HandleShipDisplay`, so they show this frame; a hit the original lands
/// after its ship's display ([`CombatEvent::Hit`]'s `late`) is shown on the
/// next.
fn show_combat(
    session: &mut Session,
    scene: &Scene,
    (effects, flashes): (&mut Effects, &mut BTreeMap<ShipRef, ShieldFlash>),
    looks: &Looks,
    chance: &mut dyn Chance,
) {
    let mut late = Vec::new();
    for event in session.take_combat_events() {
        if let CombatEvent::Hit {
            ship,
            passes_shields,
            late: shown_late,
            ..
        } = event
        {
            if shown_late {
                late.push((ship, passes_shields));
            } else {
                flashes.entry(ship).or_default().hit(passes_shields);
            }
        }
        effects.apply(&event, scene, looks, chance);
    }
    step_flashes(flashes, session);
    for (ship, passes_shields) in late {
        flashes.entry(ship).or_default().hit(passes_shields);
    }
}

/// Steps each ship's shield flash among `flashes` a frame by its shield
/// in `session` ([`ShieldFlash::step`]), dropping the flashes of ships no
/// longer in the system and those run out.
///
/// No class has a shield layer yet: the shän shield layer is
/// rdm:phase/ship-handling-fidelity/phase-10's, and no stock `shän` has
/// one, so each flash only runs down, and the base it is given, which
/// only a drawn layer reads, is the ship's shield capacity for now. That
/// phase passes the class's own loaded `Shield` (class +0x5c @0x2c8aa)
/// with the layer.
fn step_flashes(flashes: &mut BTreeMap<ShipRef, ShieldFlash>, session: &Session) {
    let shield = |ship: ShipRef| match ship {
        ShipRef::Player => Some(session.reserves().shield),
        ShipRef::Npc(id) => session
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .map(|npc| npc.reserves.shield),
    };
    flashes.retain(|&ship, flash| {
        let Some(gauge) = shield(ship) else {
            return false;
        };
        flash.step(gauge.now, gauge.max, false);
        flash.shown() > 0.0
    });
}

/// The ships in `session` breaking up, each with its sprite's width from
/// `player`'s sheet or the NPCs' `sheets` (a placeholder's without one).
fn dying(
    session: &Session,
    player: &Result<ShipSheet, String>,
    sheets: &BTreeMap<ShipId, Result<ShipSheet, String>>,
) -> Vec<Dying> {
    let width = |sheet: Option<&Result<ShipSheet, String>>| match sheet {
        Some(Ok(sheet)) => sheet.frame_width as f32,
        _ => PLACEHOLDER_SIZE,
    };
    let breaking = |condition, at: Vec2, sprite_width, explosion| match condition {
        Condition::Dying {
            ticks_left,
            starting,
        } => Some(Dying {
            at: point(at),
            sprite_width,
            explosion,
            ticks_left,
            starting,
        }),
        _ => None,
    };
    let me = breaking(
        session.player_condition(),
        session.player().position,
        width(Some(player)),
        session.hull().breakup,
    );
    let npcs = session.npcs().iter().filter_map(|npc| {
        breaking(
            npc.condition,
            npc.state.position,
            width(sheets.get(&npc.ship)),
            npc.hull.breakup,
        )
    });
    me.into_iter().chain(npcs).collect()
}

/// The alpha a ship's sprite is drawn with for how its cloak shows it
/// ([`Session::cloak_shown`]), or `None` when it is not drawn: full when
/// opaque, and a translucency of n 64ths leaving (64 - n) 64ths of full
/// alpha, rounded to nearest. A ship waiting at the gate it comes out of
/// is hidden, then faded in, by its tint ([`npc_tint`]), as
/// `_HandleShipDisplay` (@0x2c090-0x2c18e) fades it. The radar still shows
/// a cloaked ship, as `_DrawStatusRadar` does not ask.
fn cloak_alpha(shown: CloakShown) -> Option<u8> {
    match shown {
        CloakShown::Opaque => Some(u8::MAX),
        CloakShown::Translucent(level) => {
            let steps = u16::from(TRANSLUCENCY_STEPS);
            let left = steps - u16::from(level.min(TRANSLUCENCY_STEPS));
            u8::try_from((left * 255 + steps / 2) / steps).ok()
        }
        CloakShown::Hidden => None,
    }
}

/// Each of `shown` the radar shows, where it is drawn, whether it is the
/// player's target, its IFF code and whether it is heavy: those
/// [`Session::on_radar`] says, a cloaked ship only when its cloak shows
/// there or the player scans for cloaked ships.
fn radar_ships(session: &Session, shown: &[(&Npc, Point, f32)]) -> Vec<RadarShip> {
    let target = session.target().map(|npc| npc.id);
    shown
        .iter()
        .filter(|(npc, _, _)| session.on_radar(npc))
        .map(|&(npc, at, _)| RadarShip {
            position: at,
            targeted: target == Some(npc.id),
            iff: session.ship_iff(npc),
            heavy: npc.hull.mass > HEAVY_MASS,
        })
        .collect()
}

/// The class `Mass` a ship must be above to show as a square under a
/// density scanner (`cmpw $0x63` @0x49bb8-0x49c2f).
const HEAVY_MASS: f32 = 99.0;

/// The play area about the ship, in world units: the screen left of the
/// status bar, the ship drawn at the screen's centre. The session reads it to
/// tell whether a ship is on screen.
fn play_area() -> nova_sim::cloak::Area {
    let camera = Camera::centred_on(Point::default());
    let top_left = camera.screen_to_world(Point::default());
    let bottom_right =
        camera.screen_to_world(Point::new(VIEW_SIZE.0 - STATUS_BAR_WIDTH, VIEW_SIZE.1));
    nova_sim::cloak::Area {
        left: top_left.x,
        top: top_left.y,
        right: bottom_right.x,
        bottom: bottom_right.y,
    }
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
mod tests {
    use std::cell::RefCell;
    use std::num::NonZeroU16;

    use crate::system::camera::VIEW_CENTER;
    use nova_sim::landing::StellarFlags;
    use nova_sim::{
        CharacterStart, CommodityStrings, DateAffixes, DisasterRecord, GateSite, Handling,
        JunkRecord, LandingSite, OutfitId, OutfitRecord, Reserves, ShipFields, ShipId, ShipRecord,
        ShipStats, SimDiagnostic, SimSound, StarSystem, StartDate, StartError, SystemId, TICK,
        Vec2, step,
    };

    use super::*;
    use crate::devtools::{PilotDesk, PilotEdit};
    use crate::draw::crossed_box;
    use crate::draw::lights_tint;
    use crate::flight::cadence::{NAV_PERIOD_TICKS, TARGET_PERIOD_TICKS};
    use crate::flight::catalog::{
        Blink, BoomLook, GovtId, LayerSheet, StatusBarLayout, TargetCard, WeaponLook,
    };
    use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
    use crate::flight::escorts::{EscortMenuLooks, GROUP_KEYS, MENU_KEY, NO_ESCORTS};
    use crate::flight::hud::{self, HudState, NavDisplay, StatusBar};
    use crate::flight::jump::JumpPhase;
    use crate::flight::radar::RadarStellar;
    use crate::flight::radar::{self, IRect, RadarColor, RadarMark};
    use crate::flight::sprite::rotation_frame;
    use crate::flight::target::Tag;
    use crate::flight::weapons;
    use crate::galaxy::GalaxyCatalog;
    use crate::galaxy::{Galaxy, MapMode, SystemEntry};
    use crate::sound::Sound;
    use crate::spaceport::PortDialog;
    use crate::system::scene::PLACEHOLDER;
    use crate::system::{scene, starfield};
    use crate::time::sixtieths;
    use crate::ui::PlunderShown;
    use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};
    use crate::ui::mission_info::MISSION_INFO_KEY;
    use crate::ui::player_info::PLAYER_INFO_KEY;
    use crate::{DrawList, ImageKey, Input, MouseButton, Screen, ScreenAction};
    use nova_sim::CommCatalog;
    use nova_sim::ShipIff;
    use nova_sim::mission::screens::NO_ACTIVE_MISSIONS;
    use nova_sim::ship_layer::SlotSprites;
    use nova_sim::{
        ALERT_CICN, AbortError, AcceptError, Assigned, Assignment, AsteroidCatalog, CombatCatalog,
        ComputerRefusal, Controls, FighterNote, MissionId, MissionShown, NotOffered, Pilot,
        PilotCatalog, PlayerCommand, RechargeRefusal, Rulebook, Rules, StellarIff, StellarPick,
        Take, Taken, TrafficCatalog, Turn, VenueOffer, lights_level,
    };

    use crate::system::catalog::{
        AnimationData, StellarContents, StellarId, StellarSheet, SystemContents,
    };
    use crate::{Blend, DrawCommand, Font};
    use nova_sim::hyperspace::MIN_JUMP_DISTANCE;
    use nova_sim::{BlinkChance, GLOW_CRUISE, HashedRolls, glow_level};
    use nova_sim::{BoardRefusal, DockKind, MissionBoardLine};
    use nova_sim::{RuleKey, RuleSource};

    /// The first `chär` flies ship 128 (an average ship that turns 3° a
    /// tick, with a 36-rotation, 40 x 40 sheet, `rlëD` 2000) from system
    /// 130, Sol: Earth at (0, -600) and Moon at (300, -200), which animates
    /// a frame a tick. On the map Sol is at (0, 0), linked both ways to Alpha
    /// Centauri (131) at (600, 0), which holds Proxima at its centre;
    /// Barnard (132) at (0, 600) is linked to nothing, unless `onward`.
    /// Records the systems read.
    // Each flag adds a system or a link of its own, read on its own.
    #[allow(clippy::struct_excessive_bools)]
    struct FakeCatalog {
        character: Result<CharacterStart, StartError>,
        fields: ShipFields,
        sheet: Result<ShipSheet, String>,
        /// `ïntf` 128: stock-like, or why it cannot be read.
        bar: Result<StatusBarLayout, String>,
        /// `cicn` 18000's size: 24 x 24, by default.
        alert_icon: Option<(u32, u32)>,
        /// Each `gövt`'s `Interface`: none, by default.
        interfaces: Vec<(i16, i16)>,
        /// The `ïntf`s past 128: none, by default.
        other_bars: Vec<(i16, StatusBarLayout)>,
        /// The `ïntf`s asked for.
        bars_asked: RefCell<Vec<i16>>,
        /// System 130's landing sites: Earth and Moon, by default.
        sites: Vec<LandingSite>,
        /// System 130's `NavDefs` slots on the map: none, by default.
        sol_navs: Vec<Option<(u32, u16)>>,
        /// System 130's `Interference`: none, by default.
        sol_interference: i16,
        /// System 131's `Message`: no buoy (−1), by default.
        alpha_buoy: i16,
        /// The stellars Sol lists after Earth and the Moon: none, by
        /// default.
        sol_more: Vec<StellarContents>,
        systems_read: RefCell<Vec<SystemId>>,
        /// The `dësc`s: none, by default.
        descriptions: Vec<(i16, String)>,
        /// The movies the `dësc`s name: none, by default.
        desc_movies: Vec<(i16, nova_sim::DescMovie)>,
        /// The commodities: none, by default.
        commodities: CommodityStrings,
        /// The planetary events: none, by default.
        disasters: Vec<DisasterRecord>,
        /// The outfits: none, by default.
        outfits: Vec<OutfitRecord>,
        /// The `jünk`s: none, by default.
        junk: Vec<JunkRecord>,
        /// The ship classes the shipyard reads: none, by default.
        ships: Vec<ShipRecord>,
        /// The ships whose sheets were asked for.
        sheets_asked: RefCell<Vec<ShipId>>,
        /// Each system's traffic: none, by default.
        traffic: Vec<(SystemId, nova_sim::SystemTraffic)>,
        /// The düdes: none, by default.
        dudes: Vec<(nova_sim::DudeId, nova_sim::DudeRecord)>,
        /// The weapons: none, by default.
        weapons: Vec<nova_sim::WeaponRecord>,
        /// The ship types' combat fields: none, by default.
        hulls: Vec<nova_sim::HullRecord>,
        /// The governments: none, by default.
        govts: Vec<nova_sim::GovtRecord>,
        /// The ranks: none, by default.
        ranks: Vec<nova_sim::catalog::RankRecord>,
        /// The weapons' looks: none, by default.
        looks: Vec<(i16, Result<WeaponLook, String>)>,
        /// The explosions' looks: none, by default.
        booms: Vec<(i16, Result<BoomLook, String>)>,
        /// The ship types' target cards: none, by default.
        cards: Vec<(i16, TargetCard)>,
        /// The governments' target codes: none, by default.
        codes: Vec<(i16, String)>,
        /// The jettisoned cargo box's sheet: `rlëD` 500 of 36 frames, by
        /// default.
        box_sheet: Result<EffectSheet, String>,
        /// The mouse cursor's sheet: `rlëD` 651 of 8 frames, by default.
        cursor_sheet: Result<EffectSheet, String>,
        /// The asteroid types: none, by default.
        asteroid_types: Vec<nova_sim::AsteroidTypeRecord>,
        /// Each system's asteroid field: none, by default.
        asteroid_fields: Vec<nova_sim::AsteroidFieldRecord>,
        /// The string lists: none, by default.
        strings: Vec<(i16, Vec<String>)>,
        /// The persons: none, by default.
        persons: Vec<nova_sim::PersonRecord>,
        /// Ship 128's default items: none, by default.
        defaults: Vec<(OutfitId, u16)>,
        /// Whether Alpha Centauri links on to Barnard, which has no
        /// stellars: not by default.
        onward: bool,
        /// Whether Sol lists Barnard too, after Alpha Centauri, so that
        /// Hyper Select has two systems to cycle through: not by default.
        fan: bool,
        /// Whether Proxima Near (134) exists at (40, 30), close enough to
        /// Sol to show on the floating map, listed by Sol after its other
        /// links and linking back, with no stellars: not by default.
        near: bool,
        /// Every stellar a gate may lead to: none, by default.
        gates: Vec<GateSite>,
        /// The gates each system lists beyond its stellars: none, by
        /// default.
        gate_listers: Vec<(SystemId, StellarId)>,
        /// Whether Alpha Reborn (133) exists, at Alpha Centauri's place,
        /// with no links and Proxima Reborn (141) at its centre: not by
        /// default.
        reborn: bool,
        /// Whether system 128 exists, the escape pod's rescue's fallback,
        /// with no links and no stellars: not by default.
        home: bool,
        /// The stellars that can be destroyed and regenerated: none, by
        /// default.
        lives: Vec<nova_sim::StellarLife>,
        /// The missions: none, by default.
        missions: Vec<nova_sim::catalog::MissionRecord>,
        /// What the mission screens show of each: none, by default.
        mission_texts: Vec<nova_sim::catalog::MissionTexts>,
        /// Each system's `Visibility`: none, by default, so every system
        /// is visible.
        visibility: Vec<nova_sim::catalog::SystemVisibility>,
        /// The session's `spöb` and `sÿst` names: none, by default.
        place_names: nova_sim::catalog::PlaceNames,
        /// The `crön`s: none, by default.
        crons: Vec<nova_sim::catalog::CronRecord>,
    }

    /// No strings unless a test sets them.
    impl nova_sim::CommCatalog for FakeCatalog {
        fn string_list(&self, id: i16) -> Vec<String> {
            self.strings
                .iter()
                .find(|(list, _)| *list == id)
                .map_or_else(Vec::new, |(_, strings)| strings.clone())
        }
    }

    impl FakeCatalog {
        /// Sol's links: Alpha Centauri, then Barnard with `fan`, then
        /// Proxima Near with `near`.
        fn sol_links(&self) -> Vec<i16> {
            let mut links = vec![131];
            if self.fan {
                links.push(132);
            }
            if self.near {
                links.push(134);
            }
            links
        }
    }

    type View = FlightView<FakeCatalog>;

    const FIELDS: ShipFields = ShipFields {
        speed: 300,
        accel: 300,
        maneuver: 30,
        shield: 40,
        armor: 60,
        fuel: 250,
        fuel_regen: 0,
        holds: 0,
        mass: 15,
        free_mass: 8,
        contribute: 1,
        shield_rech: 0,
        armor_rech: 0,
        flags2: 0,
    };

    fn sheet() -> ShipSheet {
        ShipSheet {
            image_id: 2000,
            rotations: NonZeroU16::new(36).expect("non-zero"),
            frame_width: 40,
            frame_height: 40,
            glow: None,
            lights: None,
            blink: Blink::STEADY,
        }
    }

    fn catalog() -> FakeCatalog {
        FakeCatalog {
            character: Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [None, Some(SystemId(130)), None, None],
                start: StartDate {
                    day: 23,
                    month: 6,
                    year: 1177,
                },
                ..CharacterStart::default()
            }),
            fields: FIELDS,
            sheet: Ok(sheet()),
            bar: Ok(layout()),
            alert_icon: Some((24, 24)),
            interfaces: Vec::new(),
            other_bars: Vec::new(),
            bars_asked: RefCell::default(),
            sites: vec![
                site(128, (0.0, -600.0), StellarFlags::CAN_LAND),
                site(129, (300.0, -200.0), StellarFlags::CAN_LAND),
            ],
            systems_read: RefCell::default(),
            descriptions: Vec::new(),
            desc_movies: Vec::new(),
            commodities: CommodityStrings::default(),
            disasters: Vec::new(),
            outfits: Vec::new(),
            junk: Vec::new(),
            ships: Vec::new(),
            sheets_asked: RefCell::default(),
            traffic: Vec::new(),
            dudes: Vec::new(),
            weapons: Vec::new(),
            hulls: Vec::new(),
            govts: Vec::new(),
            ranks: Vec::new(),
            looks: Vec::new(),
            booms: Vec::new(),
            cards: Vec::new(),
            codes: Vec::new(),
            box_sheet: Ok(EffectSheet {
                image_id: 500,
                frames: NonZeroU16::new(36).expect("non-zero"),
            }),
            cursor_sheet: Ok(EffectSheet {
                image_id: 651,
                frames: NonZeroU16::new(8).expect("non-zero"),
            }),
            asteroid_types: Vec::new(),
            asteroid_fields: Vec::new(),
            strings: Vec::new(),
            persons: Vec::new(),
            defaults: Vec::new(),
            onward: false,
            fan: false,
            near: false,
            reborn: false,
            home: false,
            sol_navs: Vec::new(),
            sol_interference: 0,
            alpha_buoy: -1,
            sol_more: Vec::new(),
            gates: Vec::new(),
            gate_listers: Vec::new(),
            lives: Vec::new(),
            missions: Vec::new(),
            mission_texts: Vec::new(),
            visibility: Vec::new(),
            place_names: nova_sim::catalog::PlaceNames::default(),
            crons: Vec::new(),
        }
    }

    /// Stellar `id` at `(x, y)`, 20 x 20 (radius 10), with `flags`.
    fn site(id: i16, (x, y): (f32, f32), flags: u32) -> LandingSite {
        LandingSite {
            id: StellarId(id),
            position: Vec2::new(x, y),
            frame_size: Some((20, 20)),
            flags,
            min_status: 0,
            tech_level: 1,
            special_tech: [0; 8],
            govt: None,
            flags2: 0,
            destroyed: false,
            graphic: 0,
            dead_graphic: None,
            dead_frame_size: None,
            type_counts_frames: true,
            exit_angle: -1,
            tribute: 0,
            animation: nova_sim::catalog::SiteAnimation::default(),
        }
    }

    /// The start's date, with stock's affixes.
    const DATE: &str = "June 23, 1177 NC";

    /// Stock `ïntf` 128's areas, with background `PICT` 700.
    fn layout() -> StatusBarLayout {
        let rect = |left, top, right, bottom| crate::geometry::Bounds {
            min: at(left, top),
            max: at(right, bottom),
        };
        StatusBarLayout {
            radar: rect(8.0, 8.0, 184.0, 184.0),
            shield: rect(35.0, 199.0, 184.0, 206.0),
            armor: rect(35.0, 216.0, 184.0, 223.0),
            fuel: rect(35.0, 234.0, 184.0, 241.0),
            nav: rect(8.0, 254.0, 184.0, 286.0),
            weap: rect(8.0, 300.0, 184.0, 315.0),
            targ: rect(8.0, 330.0, 184.0, 442.0),
            cargo: rect(8.0, 458.0, 184.0, 552.0),
            bright_text: Color::WHITE,
            dim_text: Color::DIM,
            bright_radar: Color::rgba(0, 255, 0, 255),
            dim_radar: Color::rgba(0, 128, 0, 255),
            shield_color: Color::rgba(0, 0, 255, 255),
            armor_color: Color::rgba(255, 0, 0, 255),
            fuel_full: Color::rgba(255, 255, 0, 255),
            fuel_partial: Color::rgba(128, 128, 0, 255),
            font: Font::Geneva,
            font_size: 12.0,
            subtitle_size: 10.0,
            status_bkgnd: 700,
        }
    }

    impl PilotCatalog for FakeCatalog {
        fn first_character(&self) -> Result<CharacterStart, StartError> {
            self.character.clone()
        }

        fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
            assert_eq!(id, ShipId(128));
            Ok(self.fields)
        }

        fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
            assert_eq!(id, ShipId(128));
            self.defaults.clone()
        }

        fn outfits(&self) -> Vec<OutfitRecord> {
            self.outfits.clone()
        }

        fn ships(&self) -> Vec<ShipRecord> {
            self.ships.clone()
        }

        /// Sol and Alpha Centauri; Barnard, which has no stellars, only
        /// when Alpha Centauri links on to it.
        fn system_exists(&self, id: SystemId) -> bool {
            matches!(id.0, 130 | 131)
                || (id.0 == 132 && self.onward)
                || (id.0 == 133 && self.reborn)
                || (id.0 == 128 && self.home)
                || (id.0 == 134 && self.near)
        }

        fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
            match system.0 {
                130 => self.sites.clone(),
                131 => vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)],
                132 if self.onward => Vec::new(),
                133 if self.reborn => vec![site(141, (0.0, 0.0), StellarFlags::CAN_LAND)],
                128 if self.home => Vec::new(),
                134 if self.near => Vec::new(),
                other => panic!("asked for sÿst {other}'s sites"),
            }
        }

        fn star_map(&self) -> Vec<StarSystem> {
            let star = |id, (x, y), links: &[i16]| StarSystem {
                id: SystemId(id),
                position: Vec2::new(x, y),
                links: links.iter().copied().map(SystemId).collect(),
                govt: None,
                stellars: Vec::new(),
                interference: 0,
                message: -1,
            };
            let onward: &[i16] = if self.onward { &[130, 132] } else { &[130] };
            let sol = self.sol_links();
            let mut systems = vec![
                StarSystem {
                    stellars: self.sol_navs.clone(),
                    interference: self.sol_interference,
                    ..star(130, (0.0, 0.0), &sol)
                },
                StarSystem {
                    message: self.alpha_buoy,
                    ..star(131, (600.0, 0.0), onward)
                },
                star(132, (0.0, 600.0), &[]),
            ];
            if self.reborn {
                systems.push(star(133, (600.0, 0.0), &[]));
            }
            if self.home {
                systems.push(star(128, (-600.0, 0.0), &[]));
            }
            if self.near {
                systems.push(star(134, (40.0, 30.0), &[130]));
            }
            systems
        }

        fn commodity_strings(&self) -> CommodityStrings {
            self.commodities.clone()
        }

        fn string_patch(&self, _id: i16) -> nova_sim::catalog::StringPatch {
            nova_sim::catalog::StringPatch::Absent
        }

        /// [`FakeCatalog::strings`]' `STR#` 1000.
        fn buoy_strings(&self) -> Vec<String> {
            self.string_list(nova_sim::buoy::BUOY_STRINGS)
        }

        fn junk(&self) -> Vec<JunkRecord> {
            self.junk.clone()
        }

        fn disasters(&self) -> Vec<DisasterRecord> {
            self.disasters.clone()
        }

        fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
            Vec::new()
        }

        /// Stock's: no prefix, and " NC".
        fn date_affixes(&self) -> DateAffixes {
            DateAffixes {
                prefix: String::new(),
                suffix: " NC".to_owned(),
            }
        }

        fn gate_sites(&self) -> Vec<GateSite> {
            self.gates.clone()
        }

        fn stellar_lives(&self) -> Vec<nova_sim::StellarLife> {
            self.lives.clone()
        }

        fn fallback_frame_size(&self) -> Option<(u32, u32)> {
            Some((100, 100))
        }

        fn missions(&self) -> Vec<nova_sim::catalog::MissionRecord> {
            self.missions.clone()
        }

        fn mission_texts(&self) -> Vec<nova_sim::catalog::MissionTexts> {
            self.mission_texts.clone()
        }

        /// Every stellar of each system's [`landing_sites`], with every
        /// system listing it, and the gates of `gate_listers`.
        fn listed_stellars(&self) -> Vec<nova_sim::catalog::ListedStellar> {
            let mut systems = vec![SystemId(130), SystemId(131)];
            if self.onward {
                systems.push(SystemId(132));
            }
            if self.reborn {
                systems.push(SystemId(133));
            }
            if self.near {
                systems.push(SystemId(134));
            }
            let sites: Vec<(SystemId, Vec<LandingSite>)> = systems
                .into_iter()
                .map(|system| (system, self.landing_sites(system)))
                .collect();
            let mut listed = nova_sim::fixture::listed_stellars(
                sites
                    .iter()
                    .map(|(system, sites)| (*system, sites.as_slice())),
            );
            nova_sim::fixture::list_also(&mut listed, self.gate_listers.iter().copied());
            listed
        }

        fn system_visibility(&self) -> Vec<nova_sim::catalog::SystemVisibility> {
            self.visibility.clone()
        }

        fn place_names(&self) -> nova_sim::catalog::PlaceNames {
            self.place_names.clone()
        }

        fn stellar_marks(&self) -> Vec<nova_sim::catalog::StellarMark> {
            Vec::new()
        }

        fn crons(&self) -> Vec<nova_sim::catalog::CronRecord> {
            self.crons.clone()
        }

        fn ranks(&self) -> Vec<nova_sim::catalog::RankRecord> {
            self.ranks.clone()
        }

        fn nebulae(&self) -> Vec<nova_sim::catalog::NebulaRecord> {
            Vec::new()
        }
    }

    /// The weapons, ship types' combat fields and governments given.
    impl CombatCatalog for FakeCatalog {
        fn weapons(&self) -> Vec<nova_sim::WeaponRecord> {
            self.weapons.clone()
        }

        fn hulls(&self) -> Vec<nova_sim::HullRecord> {
            self.hulls.clone()
        }

        fn governments(&self) -> Vec<nova_sim::GovtRecord> {
            self.govts.clone()
        }
    }

    /// The asteroid types and fields given.
    impl AsteroidCatalog for FakeCatalog {
        fn asteroid_types(&self) -> Vec<nova_sim::AsteroidTypeRecord> {
            self.asteroid_types.clone()
        }

        fn asteroid_fields(&self) -> Vec<nova_sim::AsteroidFieldRecord> {
            self.asteroid_fields.clone()
        }

        fn box_frame_sizes(&self) -> [Option<(u32, u32)>; nova_sim::BOX_GRAPHICS] {
            [None; nova_sim::BOX_GRAPHICS]
        }
    }

    /// The traffic, düdes and persons given; no fleets.
    impl TrafficCatalog for FakeCatalog {
        fn system_traffic(&self, id: SystemId) -> Option<nova_sim::SystemTraffic> {
            self.traffic
                .iter()
                .find(|(system, _)| *system == id)
                .map(|(_, traffic)| *traffic)
        }

        fn dude(&self, id: nova_sim::DudeId) -> Option<nova_sim::DudeRecord> {
            self.dudes
                .iter()
                .find(|(dude, _)| *dude == id)
                .map(|(_, record)| record.clone())
        }

        fn fleets(&self) -> Vec<nova_sim::FleetRecord> {
            Vec::new()
        }

        fn persons(&self) -> Vec<nova_sim::PersonRecord> {
            self.persons.clone()
        }
    }

    fn stellar(id: i16, name: &str, (x, y): (i16, i16), frames: u16) -> StellarContents {
        StellarContents {
            id: StellarId(id),
            name: name.to_owned(),
            x,
            y,
            sprite: Ok(StellarSheet {
                image_id: 1000 + id,
                frames: NonZeroU16::new(frames).expect("non-zero"),
                frame_width: 20,
                frame_height: 20,
            }),
            dead_sprite: None,
            animation: AnimationData {
                delay: 1,
                frame0_bias: 0,
                only_when_destroyed: false,
            },
            radar_square: false,
        }
    }

    /// Graphic type 0's sheet (`spïn` 1000) in every system: one 40 x 40
    /// frame.
    const TYPE_ZERO: i16 = 3000;

    /// `stellar`, drawing `DeadType` sheet 2000 + `id` (one 30 x 30 frame)
    /// while destroyed.
    fn dying(id: i16, name: &str, at: (i16, i16), frames: u16) -> StellarContents {
        StellarContents {
            dead_sprite: Some(Ok(StellarSheet {
                image_id: 2000 + id,
                frames: NonZeroU16::MIN,
                frame_width: 30,
                frame_height: 30,
            })),
            ..stellar(id, name, at, frames)
        }
    }

    impl SystemCatalog for FakeCatalog {
        fn system(&self, id: SystemId) -> SystemContents {
            self.systems_read.borrow_mut().push(id);
            let (name, stellars) = match id.0 {
                130 => (
                    "Sol",
                    [
                        dying(128, "Earth", (0, -600), 1),
                        stellar(129, "Moon", (300, -200), 4),
                    ]
                    .into_iter()
                    .chain(self.sol_more.iter().cloned())
                    .collect(),
                ),
                131 => ("Alpha Centauri", vec![dying(140, "Proxima", (0, 0), 1)]),
                132 if self.onward => ("Barnard", Vec::new()),
                133 if self.reborn => (
                    "Alpha Reborn",
                    vec![stellar(141, "Proxima Reborn", (0, 0), 1)],
                ),
                128 if self.home => ("Home", Vec::new()),
                134 if self.near => ("Proxima Near", Vec::new()),
                other => panic!("asked for sÿst {other}"),
            };
            SystemContents {
                id,
                name: name.to_owned(),
                stellars,
                type_zero_sprite: Ok(StellarSheet {
                    image_id: TYPE_ZERO,
                    frames: NonZeroU16::MIN,
                    frame_width: 40,
                    frame_height: 40,
                }),
                problems: Vec::new(),
            }
        }
    }

    impl GalaxyCatalog for FakeCatalog {
        fn galaxy(&self) -> Galaxy {
            let entry = |id, name: &str, (x, y), links: &[i16]| SystemEntry {
                id: SystemId(id),
                name: name.to_owned(),
                x,
                y,
                links: links.iter().copied().map(SystemId).collect(),
                govt: None,
                stellars: Vec::new(),
            };
            let onward: &[i16] = if self.onward { &[130, 132] } else { &[130] };
            let sol = self.sol_links();
            let mut systems = vec![
                entry(130, "Sol", (0, 0), &sol),
                entry(131, "Alpha Centauri", (600, 0), onward),
                entry(132, "Barnard", (0, 600), &[]),
            ];
            if self.reborn {
                systems.push(entry(133, "Alpha Reborn", (600, 0), &[]));
            }
            if self.near {
                systems.push(entry(134, "Proxima Near", (40, 30), &[130]));
            }
            Galaxy {
                systems,
                ..Galaxy::default()
            }
        }
    }

    impl ShipSprites for FakeCatalog {
        /// Ship 128's sheet is [`FakeCatalog::sheet`]; ship 129's is
        /// `rlëD` 2001's, with 72 rotations, glow `rlëD` 2101 and lights
        /// `rlëD` 2201; ship 130's cannot be read, nor can the escape
        /// pod's, 895.
        fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
            self.sheets_asked.borrow_mut().push(id);
            match id.0 {
                128 => self.sheet.clone(),
                895 => Err("no shän 895".to_owned()),
                129 => Ok(ShipSheet {
                    image_id: 2001,
                    rotations: NonZeroU16::new(72).expect("non-zero"),
                    glow: Some(layer(2101, 72)),
                    lights: Some(layer(2201, 72)),
                    ..sheet()
                }),
                130 => Err("no shän 130".to_owned()),
                131 => Ok(ShipSheet {
                    image_id: 2002,
                    frame_width: 100,
                    frame_height: 100,
                    ..sheet()
                }),
                other => panic!("asked for shïp {other}'s sheet"),
            }
        }
    }

    /// The looks, cards and codes given; any other weapon cannot be read,
    /// and there is no other explosion type.
    impl CombatLooks for FakeCatalog {
        fn weapon_look(&self, id: nova_sim::WeaponId) -> Result<WeaponLook, String> {
            self.looks
                .iter()
                .find(|(weapon, _)| *weapon == id.0)
                .map_or_else(
                    || Err(format!("no wëap {}", id.0)),
                    |(_, look)| look.clone(),
                )
        }

        fn boom_look(&self, id: nova_sim::BoomId) -> Option<Result<BoomLook, String>> {
            self.booms
                .iter()
                .find(|(boom, _)| *boom == id.0)
                .map(|(_, look)| look.clone())
        }

        fn target_card(&self, ship: ShipId) -> TargetCard {
            self.cards
                .iter()
                .find(|(id, _)| *id == ship.0)
                .map(|(_, card)| card.clone())
                .unwrap_or_default()
        }

        fn target_code(&self, govt: GovtId) -> Option<String> {
            self.codes
                .iter()
                .find(|(id, _)| *id == govt.0)
                .map(|(_, code)| code.clone())
        }

        /// Graphic 0's sheet as set; graphics 1-4, an asteroid's boxes',
        /// `rlëD` 501-504 of 36 frames.
        fn box_sheet(&self, graphic: u16) -> Result<EffectSheet, String> {
            match graphic {
                0 => self.box_sheet.clone(),
                1..=4 => Ok(EffectSheet {
                    image_id: 500 + graphic.cast_signed(),
                    frames: NonZeroU16::new(36).expect("non-zero"),
                }),
                other => Err(format!("no spïn {}", 500 + other)),
            }
        }

        /// Type `kind`'s sheet: `rlëD` 1800 + `kind`, of 36 frames.
        fn asteroid_sheet(&self, kind: u8) -> Result<EffectSheet, String> {
            Ok(EffectSheet {
                image_id: 1800 + i16::from(kind),
                frames: NonZeroU16::new(36).expect("non-zero"),
            })
        }

        fn cursor_sheet(&self) -> Result<EffectSheet, String> {
            self.cursor_sheet.clone()
        }
    }

    /// The escort menu in its own colours, so tests can tell them.
    impl EscortMenuLooks for FakeCatalog {
        fn escort_menu_colors(&self) -> EscortMenuColors {
            MENU_COLORS
        }
    }

    const MENU_COLORS: EscortMenuColors = EscortMenuColors {
        border: Color::from_rgb24(0x0012_3456),
        hilite: Color::from_rgb24(0x0065_4321),
    };

    /// `ïntf` 128 and the others listed; every government listed has an
    /// `Interface`, and the others have no record. `PICT` 700 (128's
    /// background), 705 and 128 are 194 x 767, and there are no others.
    impl StatusBars for FakeCatalog {
        fn government_interface(&self, id: GovtId) -> Result<Option<i16>, String> {
            Ok(self
                .interfaces
                .iter()
                .find(|(govt, _)| *govt == id.0)
                .map(|&(_, interface)| interface))
        }

        fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
            self.bars_asked.borrow_mut().push(id);
            if id == 128 {
                return self.bar.clone();
            }
            self.other_bars
                .iter()
                .find(|(bar, _)| *bar == id)
                .map(|(_, layout)| layout.clone())
                .ok_or_else(|| format!("no ïntf {id}"))
        }

        fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
            [700, 705, 128].contains(&id).then_some((194, 767))
        }

        fn icon_size(&self, id: i16) -> Option<(u32, u32)> {
            (id == ALERT_CICN).then_some(self.alert_icon).flatten()
        }
    }

    fn flight() -> View {
        FlightView::new(catalog())
    }

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    fn held(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    fn player(view: &View) -> ShipState {
        *view.session().expect("flying").player()
    }

    fn handling() -> Handling {
        ShipStats::new(FIELDS, &[]).handling
    }

    /// The ship as it starts: at rest at the centre, facing up.
    fn start() -> ShipState {
        ShipState::default()
    }

    fn reserves(view: &View) -> Reserves {
        view.session().expect("flying").reserves()
    }

    /// `state` after `ticks` steps under `controls`.
    fn stepped(mut state: ShipState, controls: Controls, ticks: u32) -> ShipState {
        for _ in 0..ticks {
            step(&mut state, &handling(), controls);
        }
        state
    }

    /// The second half of a tick: `TICK / 2` rounds down, so this is a
    /// nanosecond longer.
    const REST_OF_TICK: Duration = Duration::from_nanos(16_666_667);

    const THRUST: Controls = Controls {
        thrust: true,
        turn: Turn::None,
        reverse: false,
    };

    fn ticks(view: &mut View, n: u32) {
        for _ in 0..n {
            view.tick(TICK);
        }
    }

    // Building.

    #[test]
    fn it_starts_the_session_in_its_system_at_rest() {
        let view = flight();
        let session = view.session().expect("flying");
        assert_eq!(session.ship(), ShipId(128));
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.handling(), handling());
        assert_eq!(*session.player(), start());
        let scene = view.scene().expect("a scene");
        assert_eq!(scene.id(), SystemId(130));
        assert_eq!(scene.stellars().len(), 2);
        assert_eq!(view.camera().center(), at(0.0, 0.0));
        assert_eq!(view.shown_position(), at(0.0, 0.0));
        assert_eq!(view.frame(), Some(0));
    }

    impl DescriptionSource for FakeCatalog {
        fn description(&self, id: i16) -> Result<String, String> {
            self.descriptions
                .iter()
                .find(|(found, _)| *found == id)
                .map(|(_, text)| text.clone())
                .ok_or_else(|| format!("no dësc {id}"))
        }

        fn button_style(&self) -> crate::ui::ButtonStyle {
            crate::ui::ButtonStyle::STOCK
        }
    }

    impl DescriptionGraphics for FakeCatalog {
        fn description_graphic(&self, _: i16) -> Option<i16> {
            None
        }

        fn description_movie(&self, id: i16) -> Option<nova_sim::DescMovie> {
            self.desc_movies
                .iter()
                .find(|(found, _)| *found == id)
                .map(|(_, movie)| movie.clone())
        }
    }

    /// [`catalog`] with `dësc`s 3000 "{b5 "set" "clear"}, sir.", 3001
    /// "<SRK128>" and 3002 "{G "<SRK128>" "x"}".
    fn described() -> FakeCatalog {
        FakeCatalog {
            descriptions: vec![
                (3000, "{b5 \"set\" \"clear\"}, sir.".to_owned()),
                (3001, "<SRK128>".to_owned()),
                (3002, "{G \"x\" \"<SRK128>\"} ".to_owned()),
            ],
            ..catalog()
        }
    }

    #[test]
    fn a_loaded_description_reads_the_pilot_as_it_is_now() {
        let mut view = FlightView::new(described());
        assert_eq!(view.load_description(3000).as_deref(), Ok("clear, sir."));
        view.session
            .as_mut()
            .expect("flying")
            .set_control_bit(nova_sim::Bit::new(5).expect("a bit"), true);
        assert_eq!(view.load_description(3000).as_deref(), Ok("set, sir."));
    }

    #[test]
    fn a_loaded_description_resets_and_parses_the_rank_governments() {
        let mut view = FlightView::new(described());
        let srk = |view: &View| view.session().expect("flying").plain_text("<SRK128>");
        assert_eq!(srk(&view), "<SRK128>", "no government remembered");
        assert_eq!(view.load_description(3001).as_deref(), Ok("<SRK128>"));
        assert_eq!(srk(&view), "captain", "<SRK128> remembered, crossed");
        assert_eq!(
            view.load_description(9999),
            Err("no dësc 9999".to_owned()),
            "a missing one"
        );
        assert_eq!(srk(&view), "captain", "kept by a missing one");
        assert_eq!(view.load_description(3002).as_deref(), Ok("x "));
        assert_eq!(
            srk(&view),
            "<SRK128>",
            "reset, and the string not chosen never scanned"
        );
    }

    #[test]
    fn load_description_records_the_descs_movie() {
        let movie = nova_sim::DescMovie::from_desc("Port.mov", 0x0001).expect("named");
        let mut view = FlightView::new(FakeCatalog {
            desc_movies: vec![(3000, movie.clone())],
            ..described()
        });
        assert_eq!(view.loaded_after_movie(), None, "none loaded yet");
        let _ = view.load_description(3000);
        assert_eq!(
            view.loaded_after_movie(),
            Some(movie),
            "a STR# text shown now plays it after (@0x5b40b)"
        );
        let _ = view.load_description(3001);
        assert_eq!(view.loaded_after_movie(), None, "a dësc naming none");
    }

    #[test]
    fn an_unreadable_desc_empties_it() {
        let movie = nova_sim::DescMovie::from_desc("Port.mov", 0x0001).expect("named");
        let mut view = FlightView::new(FakeCatalog {
            desc_movies: vec![(3000, movie)],
            ..described()
        });
        let _ = view.load_description(3001);
        let _ = view.load_description(3000);
        assert!(view.load_description(9999).is_err());
        assert_eq!(
            view.loaded_after_movie(),
            None,
            "as a missing dësc empties the MovieFile (@0x71a67-0x71a77)"
        );
        let srk = view.session().expect("flying").plain_text("<SRK128>");
        assert_eq!(srk, "<SRK128>", "and keeps the governments 3000 left");
    }

    #[test]
    fn with_no_session_a_loaded_description_reads_a_new_pilot() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..described()
        });
        assert_eq!(view.load_description(3000).as_deref(), Ok("clear, sir."));
        assert_eq!(view.load_description(3002).as_deref(), Ok("x "));
    }

    #[test]
    fn a_session_that_cannot_start_says_why_and_draws_nothing_else() {
        let failed = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        let mut view = FlightView::new(failed);
        assert_eq!(view.session().err(), Some("no chär to start from"));
        assert!(view.scene().is_none());
        assert_eq!(view.frame(), None);
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(
            drawn(&view).iter().cloned().collect::<Vec<_>>(),
            [DrawCommand::Text {
                text: "Cannot start flight: no chär to start from".to_owned(),
                font: Font::Geneva,
                origin: TITLE,
                size: TITLE_SIZE,
                wrap_width: None,
                color: Color::ERROR,
            },]
        );
    }

    #[test]
    fn the_unstarted_flight_shows_help_only_when_turned_on() {
        let failed = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        let view = FlightView::new(failed).with_overlays(FlightOverlays {
            title: false,
            help: true,
        });
        let list = drawn(&view);
        let help = overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM);
        assert_eq!(list.iter().last(), Some(&help));
        assert_eq!(list.iter().count(), 2, "the error line and the help");
    }

    // Flying.

    #[test]
    fn holding_up_thrusts_the_ship_up_and_the_camera_follows() {
        let mut view = flight();
        assert_eq!(view.input(&key(Key::Up, true)), ScreenAction::None);
        view.tick(Duration::from_secs(1));
        let expected = stepped(start(), THRUST, 30);
        assert_eq!(player(&view), expected);
        assert!(expected.position.y < -40.0, "{expected:?}");
        assert_eq!(expected.position.x, 0.0);
        assert_eq!(view.camera().center(), view.shown_position());
        let shown = view.shown_position();
        // Drawn at most a step behind.
        let behind = shown.y - expected.position.y;
        assert!((0.0..=3.0 + 1e-4).contains(&behind), "{shown:?}");
    }

    #[test]
    fn releasing_up_coasts() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 10);
        view.input(&key(Key::Up, false));
        let coasting = player(&view);
        ticks(&mut view, 10);
        let later = player(&view);
        assert_eq!(later.velocity, coasting.velocity);
        assert_eq!(later, stepped(coasting, Controls::default(), 10));
    }

    #[test]
    fn left_and_right_turn_the_ship_and_its_frame() {
        let mut view = flight();
        view.input(&key(Key::Right, true));
        ticks(&mut view, 11);
        // 30° after 10 steps, drawn a step behind.
        assert_eq!(player(&view).heading, 33.0);
        assert_eq!(view.shown_heading(), 30.0);
        assert_eq!(view.frame(), Some(3));
        view.input(&key(Key::Right, false));
        view.input(&key(Key::Left, true));
        ticks(&mut view, 21);
        assert_eq!(player(&view).heading, 330.0);
        assert_eq!(view.frame(), Some(33));
        assert_eq!(player(&view).velocity, Vec2::ZERO, "turning alone");
    }

    #[test]
    fn left_and_right_together_cancel() {
        let mut view = flight();
        view.input(&key(Key::Left, true));
        view.input(&key(Key::Right, true));
        ticks(&mut view, 5);
        assert_eq!(player(&view).heading, 0.0);
    }

    #[test]
    fn down_turns_the_ship_against_its_motion() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 30);
        view.input(&key(Key::Up, false));
        view.input(&key(Key::Down, true));
        ticks(&mut view, 70);
        assert!(
            (player(&view).heading - 180.0).abs() < 1e-3,
            "{:?}",
            player(&view)
        );
        assert_eq!(view.frame(), Some(18));
        // Left or Right overrides it.
        view.input(&key(Key::Left, true));
        ticks(&mut view, 1);
        assert!((player(&view).heading - 177.0).abs() < 1e-3);
    }

    #[test]
    fn repeats_hold_a_key_and_releasing_the_keys_stops_the_thrust() {
        let mut view = flight();
        view.input(&held(Key::Up));
        ticks(&mut view, 5);
        assert_eq!(player(&view), stepped(start(), THRUST, 5));
        view.release_keys();
        let coasting = player(&view);
        ticks(&mut view, 5);
        assert_eq!(player(&view), stepped(coasting, Controls::default(), 5));
    }

    #[test]
    fn a_release_without_a_press_does_nothing() {
        let mut view = flight();
        view.input(&key(Key::Up, false));
        ticks(&mut view, 3);
        assert_eq!(player(&view), start());
    }

    #[test]
    fn other_input_changes_nothing_and_never_quits() {
        let mut view = flight();
        let others = [
            key(Key::Escape, true),
            key(Key::Space, true),
            key(Key::Tab, true),
            key(Key::Space, true),
            key(Key::Enter, true),
            key(Key::Char('w'), true),
            key(Key::Char('w'), true),
            key(Key::Other, true),
            Input::PointerMoved(at(1.0, 2.0)),
            Input::PointerButton {
                button: crate::MouseButton::Left,
                pressed: true,
                at: at(1.0, 2.0),
            },
        ];
        for input in others {
            assert_eq!(view.input(&input), ScreenAction::None, "{input:?}");
        }
        ticks(&mut view, 5);
        assert_eq!(player(&view), start());
    }

    // Interpolation.

    #[test]
    fn the_ship_is_drawn_alpha_of_the_way_from_the_last_step_to_this_one() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        view.tick(TICK);
        let first = stepped(start(), THRUST, 1);
        assert_eq!(player(&view), first);
        assert_eq!(view.alpha(), 0.0);
        assert_eq!(view.shown_position(), at(0.0, 0.0), "a step behind");
        view.tick(TICK / 2);
        assert!((view.alpha() - 0.5).abs() < 1e-6, "{}", view.alpha());
        assert_eq!(player(&view), first, "no step yet");
        let half = view.shown_position();
        assert!((half.y - first.position.y / 2.0).abs() < 1e-6, "{half:?}");
        assert_eq!(half.x, 0.0);
        assert_eq!(view.camera().center(), half);
        // The rest of the tick: one more step, and back to its start.
        view.tick(REST_OF_TICK);
        let second = stepped(first, THRUST, 1);
        assert_eq!(player(&view), second);
        assert_eq!(view.alpha(), 0.0);
        assert_eq!(
            view.shown_position(),
            at(first.position.x, first.position.y)
        );
    }

    #[test]
    fn mid_flight_the_ship_is_drawn_between_its_last_two_positions() {
        let mut view = flight();
        view.input(&key(Key::Right, true));
        ticks(&mut view, 15);
        view.input(&key(Key::Right, false));
        view.input(&key(Key::Up, true));
        ticks(&mut view, 20);
        let before = player(&view);
        ticks(&mut view, 1);
        let after = player(&view);
        view.tick(TICK / 4);
        let shown = view.shown_position();
        let expected = |from: f32, to: f32| from + (to - from) * view.alpha();
        assert!(
            before.position.x > 1.0 && before.position.y < -1.0,
            "{before:?}"
        );
        assert!((shown.x - expected(before.position.x, after.position.x)).abs() < 1e-4);
        assert!((shown.y - expected(before.position.y, after.position.y)).abs() < 1e-4);
    }

    #[test]
    fn the_heading_is_drawn_the_short_way_round_across_0() {
        let mut view = flight();
        view.input(&key(Key::Left, true));
        view.tick(TICK);
        assert_eq!(player(&view).heading, 357.0);
        view.tick(TICK / 2);
        assert!(
            (view.shown_heading() - 358.5).abs() < 1e-3,
            "{}",
            view.shown_heading()
        );
        view.input(&key(Key::Left, false));
        view.input(&key(Key::Right, true));
        view.tick(REST_OF_TICK);
        view.tick(TICK / 2);
        // From 357 to 0: half way is 358.5 again, not 178.5.
        assert_eq!(player(&view).heading, 0.0);
        assert!(
            (view.shown_heading() - 358.5).abs() < 1e-3,
            "{}",
            view.shown_heading()
        );
    }

    // Drawing.

    fn drawn(view: &View) -> DrawList {
        let mut list = DrawList::new();
        view.draw(&mut list);
        list
    }

    fn overlay(text: &str, origin: Point, size: f32, color: Color) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin,
            size,
            wrap_width: None,
            color,
        }
    }

    fn sprites(list: &DrawList) -> Vec<(ImageKey, Point)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { image, center, .. } => Some((*image, *center)),
                _ => None,
            })
            .collect()
    }

    fn lines(list: &DrawList) -> Vec<DrawCommand> {
        list.iter()
            .filter(|c| matches!(c, DrawCommand::Line { .. }))
            .cloned()
            .collect()
    }

    /// The radar's pixels drawn, in either radar colour, row by row as
    /// drawn.
    fn dots_on_radar(list: &DrawList) -> Vec<(i32, i32)> {
        let layout = layout();
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Dot {
                    center,
                    size: 1.0,
                    color,
                } if color == layout.dim_radar || color == layout.bright_radar => {
                    Some(((center.x - 0.5) as i32, (center.y - 0.5) as i32))
                }
                _ => None,
            })
            .collect()
    }

    /// The stock radar, with the bar at (830, 0): centred on (926, 96).
    const STOCK_RADAR: IRect = IRect::new(838, 8, 1014, 184);

    /// What the radar shows this frame, as marks on the stock radar.
    fn radar_marks(view: &View) -> Vec<RadarMark> {
        radar::marks(
            STOCK_RADAR,
            &view.radar_shown(),
            view.hud_cadence.blink_on(),
        )
    }

    /// The ships' blips on the radar this frame: its pixels but the
    /// player's, the last.
    fn blips(view: &View) -> Vec<RadarMark> {
        let mut marks: Vec<RadarMark> = radar_marks(view)
            .into_iter()
            .filter(|mark| matches!(mark, RadarMark::Pixel { .. }))
            .collect();
        marks.pop();
        marks
    }

    /// The 2×2 oval a small stellar at radar pixel (`x`, `y`) draws.
    fn small_oval(x: i32, y: i32) -> RadarMark {
        RadarMark::Oval {
            rect: IRect::around(x, y, 1),
            color: RadarColor::Dim,
        }
    }

    const PLAYER_PIXEL: RadarMark = RadarMark::Pixel {
        x: 926,
        y: 96,
        color: RadarColor::Bright,
    };

    fn texts(list: &DrawList) -> Vec<String> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// The port's title, and its help line, on.
    const TITLED: FlightOverlays = FlightOverlays {
        title: true,
        help: true,
    };

    #[test]
    fn the_title_and_help_are_not_drawn_by_default() {
        assert_eq!(
            FlightOverlays::default(),
            FlightOverlays {
                title: false,
                help: false
            }
        );
        assert_eq!(flight().overlays(), FlightOverlays::default());
        assert_eq!(flight().with_overlays(TITLED).overlays(), TITLED);
        let texts = texts(&drawn(&flight()));
        assert!(!texts.contains(&"Sol (sÿst 130)".to_owned()), "{texts:?}");
        assert!(!texts.contains(&HELP.to_owned()), "{texts:?}");
    }

    /// The flight's draw list's commands up to the HUD's first, whose
    /// status bar background is the first picture.
    fn before_the_hud(list: &DrawList) -> Vec<DrawCommand> {
        list.iter()
            .take_while(|c| !matches!(c, DrawCommand::Picture { .. }))
            .cloned()
            .collect()
    }

    #[test]
    fn the_title_draws_when_turned_on() {
        let titled = FlightOverlays {
            title: true,
            help: false,
        };
        let list = drawn(&flight().with_overlays(titled));
        let before = before_the_hud(&list);
        assert_eq!(
            before.last(),
            Some(&overlay("Sol (sÿst 130)", TITLE, TITLE_SIZE, Color::WHITE)),
            "over the flight, under the HUD"
        );
        assert!(!texts(&list).contains(&HELP.to_owned()));
    }

    #[test]
    fn the_help_line_draws_when_turned_on() {
        let helped = FlightOverlays {
            title: false,
            help: true,
        };
        let list = drawn(&flight().with_overlays(helped));
        let before = before_the_hud(&list);
        assert_eq!(
            before.last(),
            Some(&overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM)),
            "over the flight, under the HUD"
        );
        assert!(!texts(&list).contains(&"Sol (sÿst 130)".to_owned()));
        let both = drawn(&flight().with_overlays(TITLED));
        let before = before_the_hud(&both);
        assert_eq!(
            before[before.len() - 2..],
            [
                overlay("Sol (sÿst 130)", TITLE, TITLE_SIZE, Color::WHITE),
                overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM),
            ],
            "the title, then the help"
        );
    }

    #[test]
    fn it_draws_the_stars_the_stellars_the_ship_then_the_hud() {
        let view = flight();
        let list = drawn(&view);
        let mut expected = DrawList::new();
        starfield::draw(&mut expected, &view.camera());
        scene::draw_stellars(
            &mut expected,
            view.scene().expect("a scene"),
            &view.camera(),
            Duration::ZERO,
        );
        expected.sprite(ImageKey::sprite(2000, 0), VIEW_CENTER, Color::WHITE);
        hud::draw(
            &mut expected,
            view.status_bar().expect("a status bar"),
            &HudState {
                radar: &RadarScene {
                    player: at(0.0, 0.0),
                    stellars: vec![
                        RadarStellar {
                            position: at(0.0, -600.0),
                            square: false,
                            sprite_width: Some(20),
                            iff: StellarIff::Unlandable,
                        },
                        RadarStellar {
                            position: at(300.0, -200.0),
                            square: false,
                            sprite_width: Some(20),
                            iff: StellarIff::Unlandable,
                        },
                    ],
                    ..RadarScene::default()
                },
                blink_on: false,
                reserves: ShipStats::new(FIELDS, &[]).full(),
                nav: NavDisplay::Off,
                cargo: Some(&view.session().expect("flying").status_cargo()),
                disabled: false,
            },
            None,
        );
        let bar = view.status_bar().expect("a status bar");
        let origin = hud::bar_origin(bar);
        target::draw_target_panel(&mut expected, &bar.layout, origin, None, None);
        target::draw_secondary(&mut expected, &bar.layout, origin, None, None);
        assert_eq!(list, expected);
        assert!(matches!(list.iter().next(), Some(DrawCommand::Dot { .. })));
        assert_eq!(
            sprites(&list),
            [
                (ImageKey::sprite(1128, 0), at(512.0, -216.0)),
                (ImageKey::sprite(1129, 0), at(812.0, 184.0)),
                (ImageKey::sprite(2000, 0), VIEW_CENTER),
            ]
        );
        assert_eq!(
            HELP,
            "Arrows: fly   Space: fire   Ctrl: secondary   W: weapon   Tab: next target   R: nearest   B: board   Y: hail   Cmd-K: jettison   F/D/V/C: escorts   Alt-C: dock   E: escort menu   L: land   M: map   J: jump   \\: next system   P: player info   I: mission info   Cmd-,: preferences   Esc: leave"
        );
        assert_eq!((TITLE, HELP_AT), (at(16.0, 32.0), at(16.0, 744.0)));
    }

    #[test]
    fn the_ship_stays_at_the_screens_centre_as_the_stellars_go_by() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        view.input(&key(Key::Right, true));
        view.tick(Duration::from_millis(1500));
        let list = drawn(&view);
        let shown = sprites(&list);
        let camera = view.camera();
        assert_eq!(
            shown[2],
            (
                ImageKey::sprite(2000, view.frame().expect("a sheet")),
                VIEW_CENTER
            )
        );
        assert_ne!(view.frame(), Some(0));
        assert_eq!(shown[0].1, camera.world_to_screen(at(0.0, -600.0)));
        assert_ne!(shown[0].1, at(512.0, -216.0), "the stellars moved");
        let mut stars = DrawList::new();
        starfield::draw(&mut stars, &camera);
        assert!(
            list.iter().take(stars.len()).eq(stars.iter()),
            "stars under the camera"
        );
    }

    #[test]
    fn the_stellars_animate_with_time() {
        let mut view = flight();
        ticks(&mut view, 1);
        assert_eq!(sprites(&drawn(&view))[1].0, ImageKey::sprite(1129, 1));
        ticks(&mut view, 2);
        assert_eq!(sprites(&drawn(&view))[1].0, ImageKey::sprite(1129, 3));
    }

    #[test]
    fn a_ship_without_a_sheet_is_a_placeholder_with_the_reason() {
        let sheetless = FakeCatalog {
            sheet: Err("no shän 128 for shïp 128".to_owned()),
            ..catalog()
        };
        let view = FlightView::new(sheetless);
        assert_eq!(view.frame(), None);
        let list = drawn(&view);
        assert_eq!(sprites(&list).len(), 2, "the stellars only");
        let mut expected = DrawList::new();
        crossed_box(&mut expected, VIEW_CENTER, PLACEHOLDER_SIZE, PLACEHOLDER);
        assert_eq!(
            lines(&list)[..expected.len()],
            *lines(&expected),
            "then the HUD's"
        );
        let reason = list
            .iter()
            .find(|c| matches!(c, DrawCommand::Text { text, .. } if text.starts_with("Sprite")))
            .cloned();
        assert_eq!(
            reason,
            Some(overlay(
                "Sprite unavailable: no shän 128 for shïp 128",
                at(
                    VIEW_CENTER.x - PLACEHOLDER_SIZE / 2.0,
                    VIEW_CENTER.y + PLACEHOLDER_SIZE / 2.0 + MESSAGE_GAP
                ),
                OVERLAY_SIZE,
                Color::ERROR
            ))
        );
        assert!(matches!(list.iter().next(), Some(DrawCommand::Dot { .. })));
    }

    // The engine glow and running lights.

    fn layer(image_id: i16, frames: u16) -> LayerSheet {
        LayerSheet {
            image_id,
            frames: NonZeroU16::new(frames).expect("non-zero"),
        }
    }

    /// [`sheet`] with a 36-frame glow, `rlëD` 2100, and 12-frame lights,
    /// `rlëD` 2200.
    fn layered() -> FakeCatalog {
        FakeCatalog {
            sheet: Ok(ShipSheet {
                glow: Some(layer(2100, 36)),
                lights: Some(layer(2200, 12)),
                ..sheet()
            }),
            ..catalog()
        }
    }

    /// The ship's sprites drawn: those from `rlëD`s 2000 to 2299.
    fn ship_sprites(view: &View) -> Vec<(ImageKey, Point)> {
        sprites(&drawn(view))
            .into_iter()
            .filter(|(image, _)| (2000..2300).contains(&image.id))
            .collect()
    }

    #[test]
    fn a_coasting_ship_draws_its_lights_over_it_and_no_glow() {
        let mut view = FlightView::new(layered());
        assert_eq!(
            ship_sprites(&view),
            [
                (ImageKey::sprite(2000, 0), VIEW_CENTER),
                (ImageKey::sprite(2200, 0), VIEW_CENTER),
            ]
        );
        view.input(&key(Key::Up, true));
        assert_eq!(ship_sprites(&view).len(), 2, "held, but not yet flown");
        ticks(&mut view, 3);
        view.input(&key(Key::Up, false));
        // As many ticks coasting as thrusting: the glow has faded out.
        ticks(&mut view, 3);
        let session = view.session().expect("flying");
        assert!(!session.thrusting());
        assert_eq!(session.engine_glow(), 0);
        assert_eq!(
            ship_sprites(&view)
                .iter()
                .map(|(image, _)| image.id)
                .collect::<Vec<_>>(),
            [2000, 2200],
            "coasting"
        );
    }

    #[test]
    fn a_thrusting_ship_draws_its_glow_between_it_and_its_lights() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        // A base of 6: drawn at every roll.
        ticks(&mut view, 6);
        let centre = view.camera().world_to_screen(view.shown_position());
        assert_eq!(
            ship_sprites(&view),
            [
                (ImageKey::sprite(2000, 0), centre),
                (ImageKey::sprite(2100, 0), centre),
                (ImageKey::sprite(2200, 0), centre),
            ]
        );
    }

    /// The ship's sprites' image ids and blends, as [`ship_sprites`].
    fn ship_blends(view: &View) -> Vec<(i16, Blend)> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { image, blend, .. } if (2000..2300).contains(&image.id) => {
                    Some((image.id, *blend))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_thrusting_ships_glow_and_lights_or_onto_its_normal_sprite() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        ticks(&mut view, 6);
        assert_eq!(
            ship_blends(&view),
            [(2000, Blend::Normal), (2100, Blend::Or), (2200, Blend::Or),]
        );
        assert!(
            !drawn(&view).iter().any(|command| matches!(
                command,
                DrawCommand::Sprite {
                    blend: Blend::Additive,
                    ..
                }
            )),
            "nothing in a thrusting, lit frame is added"
        );
        view.input(&key(Key::Up, false));
        ticks(&mut view, 6);
        assert_eq!(view.session().expect("flying").engine_glow(), 0);
        assert_eq!(
            ship_blends(&view),
            [(2000, Blend::Normal), (2200, Blend::Or)],
            "coasting"
        );
    }

    #[test]
    fn the_layers_turn_with_the_ship_between_steps() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        view.input(&key(Key::Right, true));
        // 50 steps and a half: the display is between two steps.
        ticks(&mut view, 50);
        view.tick(TICK / 2);
        assert_ne!(view.shown_heading(), player(&view).heading, "between");
        let frame = view.frame().expect("a sheet");
        assert_eq!(frame, 14, "148.5, truncated: {}", view.shown_heading());
        let centre = view.camera().world_to_screen(view.shown_position());
        assert_eq!(
            ship_sprites(&view),
            [
                (ImageKey::sprite(2000, frame), centre),
                (ImageKey::sprite(2100, frame), centre),
                (ImageKey::sprite(2200, frame % 12), centre),
            ]
        );
    }

    #[test]
    fn landing_while_thrusting_puts_the_glow_out() {
        let mut view = FlightView::new(FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..layered()
        });
        view.input(&key(Key::Up, true));
        ticks(&mut view, 6);
        assert_eq!(ship_sprites(&view).len(), 3, "glowing");
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        assert_eq!(view.session().expect("flying").engine_glow(), 0);
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2000, 2200], "landed");
    }

    #[test]
    fn beginning_a_jump_while_thrusting_puts_the_glow_out() {
        let mut view = FlightView::new(layered());
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(Key::Up, true));
        ticks(&mut view, 6);
        assert_eq!(ship_sprites(&view).len(), 3, "glowing");
        jump_now(&mut view);
        assert_eq!(view.session().expect("flying").engine_glow(), 0);
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2000, 2200], "jumping");
    }

    #[test]
    fn a_ship_without_layers_draws_only_its_sprite() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert!(view.session().expect("flying").thrusting());
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2000]);
    }

    #[test]
    fn a_ship_without_a_sheet_draws_no_layers() {
        let mut view = FlightView::new(FakeCatalog {
            sheet: Err("no shän 128 for shïp 128".to_owned()),
            ..layered()
        });
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(ship_sprites(&view), []);
    }

    /// Every command drawing the glow, `rlëD` 2100, whatever its blend.
    fn glow_commands(view: &View) -> Vec<DrawCommand> {
        drawn(view)
            .iter()
            .filter(
                |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 2100),
            )
            .cloned()
            .collect()
    }

    /// The glow commands for a base of `base` at `tick`: the glow drawn by
    /// OR at the level [`glow_level`] gives over the ship's centre, on its
    /// frame, or none when it is hidden.
    fn glow_for(view: &View, base: u8, tick: u64) -> Vec<DrawCommand> {
        let frame = view.frame().expect("a sheet") % 36;
        let center = view.camera().world_to_screen(view.shown_position());
        glow_level(base, tick, &view.glow_rolls)
            .map(|level| DrawCommand::Sprite {
                image: ImageKey::sprite(2100, frame),
                center,
                tint: lights_tint(level),
                blend: Blend::Or,
            })
            .into_iter()
            .collect()
    }

    #[test]
    fn the_glow_ramps_in_and_flickers_at_the_glow_function_level() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        let mut cruising = Vec::new();
        for n in 1..=40_u8 {
            ticks(&mut view, 1);
            let base = n.min(GLOW_CRUISE);
            assert_eq!(view.session().expect("flying").engine_glow(), base);
            let want = glow_for(&view, base, u64::from(n));
            assert_eq!(glow_commands(&view), want, "tick {n}");
            if n >= 6 {
                assert_eq!(want.len(), 1, "a base of 6 or more always shows");
            }
            if n >= 24 {
                cruising.extend(want);
            }
        }
        cruising.dedup();
        assert!(cruising.len() >= 2, "it flickers: {cruising:?}");
    }

    #[test]
    fn releasing_up_fades_the_glow_out() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        ticks(&mut view, 30);
        assert_eq!(view.session().expect("flying").engine_glow(), GLOW_CRUISE);
        view.input(&key(Key::Up, false));
        for k in 1..=40_u8 {
            ticks(&mut view, 1);
            let base = GLOW_CRUISE.saturating_sub(k);
            assert_eq!(view.session().expect("flying").engine_glow(), base);
            let want = glow_for(&view, base, 30 + u64::from(k));
            assert_eq!(glow_commands(&view), want, "{k} ticks after");
            if k == 3 {
                let level = glow_level(21, 33, &view.glow_rolls).expect("still drawn");
                let alpha = lights_tint(level).a;
                assert!(alpha < 255 && alpha >= lights_tint(17).a, "{alpha}");
                assert_eq!(want.len(), 1, "fading, not out");
            }
            if k >= 24 {
                assert_eq!(want, [], "out {k} ticks after");
            }
        }
    }

    // The running lights' blinking.

    /// The Shuttle's blink: a double flash every 40 ticks, lit at ticks
    /// 1-3 and 10-12.
    const SHUTTLE: Blink = Blink {
        mode: 1,
        a: 4,
        b: 1,
        c: 2,
        d: 20,
    };

    /// [`layered`], with the lights blinking by `blink`.
    fn blinking(blink: Blink) -> FakeCatalog {
        FakeCatalog {
            sheet: Ok(ShipSheet {
                glow: Some(layer(2100, 36)),
                lights: Some(layer(2200, 12)),
                blink,
                ..sheet()
            }),
            ..catalog()
        }
    }

    /// The tints the lights, `rlëD` 2200, are drawn by OR with.
    fn lights_tints(view: &View) -> Vec<Color> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite {
                    image,
                    tint,
                    blend: Blend::Or,
                    ..
                } if image.id == 2200 => Some(*tint),
                _ => None,
            })
            .collect()
    }

    /// The lights' alphas drawn: one, or none when they are off.
    fn lights_alphas(view: &View) -> Vec<u8> {
        lights_tints(view).iter().map(|tint| tint.a).collect()
    }

    #[test]
    fn the_shuttles_lights_blink_with_game_time() {
        let mut view = FlightView::new(blinking(SHUTTLE));
        assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 0: off");
        assert_eq!(ship_sprites(&view)[0].0.id, 2000, "the ship is drawn");
        ticks(&mut view, 1);
        assert_eq!(lights_tints(&view), [Color::WHITE], "tick 1: on");
        ticks(&mut view, 3);
        assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 4: off");
        ticks(&mut view, 6);
        assert_eq!(lights_alphas(&view), [255], "tick 10: on again");
        ticks(&mut view, 3);
        assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 13: the gap");
    }

    #[test]
    fn a_pulsing_light_is_ored_at_its_level() {
        // The stock triangle: 10 to 31.
        let triangle = Blink {
            mode: 2,
            a: 10,
            b: 75,
            c: 32,
            d: 75,
        };
        let mut view = FlightView::new(blinking(triangle));
        assert_eq!(lights_alphas(&view), [0_u8; 0], "0.75: not drawn");
        ticks(&mut view, 1);
        assert_eq!(lights_alphas(&view), [8], "level 1");
        ticks(&mut view, 40);
        assert_eq!(lights_alphas(&view), [247], "level 31");
    }

    /// Every command drawing the lights, `rlëD` 2200, whatever its blend.
    fn lights_commands(view: &View) -> Vec<DrawCommand> {
        drawn(view)
            .iter()
            .filter(
                |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 2200),
            )
            .cloned()
            .collect()
    }

    /// The lights drawn by OR with `tint` over the coasting ship's centre.
    fn lights_at(tint: Color) -> DrawCommand {
        DrawCommand::Sprite {
            image: ImageKey::sprite(2200, 0),
            center: VIEW_CENTER,
            tint,
            blend: Blend::Or,
        }
    }

    #[test]
    fn lights_are_ored_scaled_by_level_below_full_and_whole_at_full() {
        // The stock triangle, 10 to 31, never full: the original draws
        // every level with its translucent blit, which ORs in the lights
        // scaled by level/32 (see `lights_tint`).
        let triangle = Blink {
            mode: 2,
            a: 10,
            b: 75,
            c: 32,
            d: 75,
        };
        let rolls = HashedRolls::new(0);
        let mut view = FlightView::new(blinking(triangle));
        let mut levels = Vec::new();
        for tick in 0..=101 {
            let level = lights_level(&triangle, tick, &rolls);
            let want: Vec<_> = level
                .map(|n| lights_at(lights_tint(n)))
                .into_iter()
                .collect();
            assert_eq!(lights_commands(&view), want, "tick {tick}");
            levels.extend(level);
            ticks(&mut view, 1);
        }
        assert!(levels.contains(&10), "the lowest level is drawn");
        assert!(levels.contains(&31), "the highest level is drawn");

        // A steady light is full: ORed whole.
        let view = FlightView::new(blinking(Blink::STEADY));
        assert_eq!(lights_commands(&view), [lights_at(Color::WHITE)]);
    }

    #[test]
    fn steady_lights_are_ored_at_full_from_the_start() {
        let view = FlightView::new(blinking(Blink::STEADY));
        assert_eq!(lights_tints(&view), [Color::WHITE]);
    }

    #[test]
    fn the_glow_still_shows_while_the_lights_are_off() {
        let mut view = FlightView::new(blinking(SHUTTLE));
        view.input(&key(Key::Up, true));
        // Tick 13: the Shuttle's lights are dark, and a base of 13 always
        // shows the glow.
        ticks(&mut view, 13);
        assert!(view.session().expect("flying").thrusting());
        assert_eq!(
            ship_blends(&view),
            [(2000, Blend::Normal), (2100, Blend::Or)]
        );
        let glow = drawn(&view).iter().find_map(|command| match command {
            DrawCommand::Sprite { image, tint, .. } if image.id == 2100 => Some(*tint),
            _ => None,
        });
        let level = glow_level(13, 13, &view.glow_rolls).expect("drawn");
        assert_eq!(glow, Some(lights_tint(level)));
    }

    #[test]
    fn the_blink_pauses_while_the_map_is_open() {
        let mut view = FlightView::new(blinking(SHUTTLE));
        ticks(&mut view, 3);
        assert_eq!(lights_alphas(&view), [255], "tick 3");
        view.input(&key(MAP_KEY, true));
        assert!(view.map_open());
        // Tick 103 would be dark.
        ticks(&mut view, 100);
        view.close_map();
        assert_eq!(lights_alphas(&view), [255], "still tick 3");
        ticks(&mut view, 1);
        assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 4");
    }

    #[test]
    fn random_lights_roll_on_hashed_rolls() {
        // A=5 B=12 C=3: one of 8 levels from 5, every 4 ticks.
        let random = Blink {
            mode: 3,
            a: 5,
            b: 12,
            c: 3,
            d: 0,
        };
        let rolls = HashedRolls::new(0);
        let expected = |change| lights_tint(5 + rolls.roll(change, 8) as u8).a;
        let mut view = FlightView::new(blinking(random));
        assert_eq!(lights_alphas(&view), [expected(0)]);
        ticks(&mut view, 4);
        assert_eq!(lights_alphas(&view), [expected(1)]);
    }

    // The HUD.

    #[test]
    fn a_new_pilot_shows_the_default_status_bar() {
        let view = flight();
        assert_eq!(
            view.status_bar(),
            Ok(&StatusBar {
                layout: layout(),
                background: Some(hud::Background {
                    id: 700,
                    width: 194.0,
                    height: 767.0,
                }),
            })
        );
    }

    #[test]
    fn the_status_bar_alone_is_flights_bar_and_nothing_else() {
        let view = landed_view().with_overlays(TITLED);
        let mut bar = DrawList::new();
        view.draw_status_bar(&mut bar);
        let background = DrawCommand::Picture {
            image: ImageKey::picture(700),
            top_left: at(830.0, 0.0),
        };
        assert_eq!(bar.iter().next(), Some(&background));
        assert_eq!(dots_on_radar(&bar), [(926, 96)], "the blank radar");
        assert_eq!(sprites(&bar), [], "no stellars or ship");
        assert!(!texts(&bar).contains(&HELP.to_owned()), "no help line");
        let flown = drawn(&view);
        let from = flown
            .iter()
            .position(|command| *command == background)
            .expect("flight draws the bar");
        assert!(from > 0, "flight draws the stars first");
        assert_eq!(
            bar.iter().collect::<Vec<_>>(),
            flown.iter().skip(from).collect::<Vec<_>>(),
            "the bar as flight draws it"
        );
        let barless = FakeCatalog {
            bar: Err("no ïntf 128".to_owned()),
            ..catalog()
        };
        let view = FlightView::with_pilot(barless, docked_pilot("Ada"));
        let mut bar = DrawList::new();
        view.draw_status_bar(&mut bar);
        assert_eq!(bar.len(), 0, "nothing, not even why");
    }

    #[test]
    fn the_hud_shows_the_status_bar_full_bars_and_the_stellars_on_radar() {
        let view = flight();
        let list = drawn(&view);
        assert!(
            list.iter().any(|c| *c
                == DrawCommand::Picture {
                    image: ImageKey::picture(700),
                    top_left: at(830.0, 0.0),
                }),
            "{list:?}"
        );
        // The shïp's shield 40, armour 60 and fuel 250: all of it in the
        // full colour, then the half jump past the two whole ones (from
        // 119.2 pixels) over it.
        let width = 149.0;
        let bar = |top: f32, from: f32, to: f32, color| DrawCommand::Line {
            from: at(865.0 + from, top + 3.5),
            to: at(865.0 + to, top + 3.5),
            width: 7.0,
            color,
        };
        let layout = layout();
        assert_eq!(
            lines(&list),
            [
                bar(199.0, 0.0, width, layout.shield_color),
                bar(216.0, 0.0, width, layout.armor_color),
                bar(234.0, 0.0, width, layout.fuel_full),
                bar(234.0, 119.0, width, layout.fuel_partial),
            ]
        );
        // Earth, 600 up, is 18.75 pixels up the radar, at 77.25; the Moon,
        // (300, -200), at (935.375, 89.75): small ovals, dim, then the
        // player, bright.
        assert_eq!(
            dots_on_radar(&list),
            [
                (925, 76),
                (926, 76),
                (925, 77),
                (926, 77),
                (934, 88),
                (935, 88),
                (934, 89),
                (935, 89),
                (926, 96)
            ]
        );
        assert_eq!(
            radar_marks(&view),
            [small_oval(926, 77), small_oval(935, 89), PLAYER_PIXEL]
        );
        // No system name in the nav area (the dev title still names it).
        assert!(!texts(&list).contains(&"Sol".to_owned()), "{list:?}");
    }

    #[test]
    fn a_stellars_radar_shape_and_sprite_width_reach_the_radar() {
        // `w` x `h` frames: the radar reads the width, never the height.
        let sized = |id, at, (w, h): (u32, u32), square| StellarContents {
            sprite: Ok(StellarSheet {
                image_id: 1000 + id,
                frames: NonZeroU16::MIN,
                frame_width: w,
                frame_height: h,
            }),
            radar_square: square,
            ..stellar(id, "Extra", at, 1)
        };
        let mut sites = catalog().sites;
        sites.extend([
            site(150, (640.0, 0.0), StellarFlags::STATION),
            site(151, (-640.0, 320.0), 0),
            site(152, (0.0, 640.0), 0),
        ]);
        let view = FlightView::new(FakeCatalog {
            sites,
            sol_more: vec![
                sized(150, (640, 0), (300, 300), true),
                sized(151, (-640, 320), (120, 60), false),
                sized(152, (0, 640), (250, 150), false),
            ],
            ..catalog()
        });
        let dim = RadarColor::Dim;
        // A station is a 2×2 square whatever its size; past 89 pixels wide
        // an oval grows 2 a side, past 199, 3.
        assert_eq!(
            radar_marks(&view),
            [
                small_oval(926, 77),
                small_oval(935, 89),
                RadarMark::Square {
                    rect: IRect::around(946, 96, 1),
                    color: dim,
                },
                RadarMark::Oval {
                    rect: IRect::around(906, 106, 2),
                    color: dim,
                },
                RadarMark::Oval {
                    rect: IRect::around(926, 116, 3),
                    color: dim,
                },
                PLAYER_PIXEL,
            ]
        );
    }

    /// The radar's player position: where it was when the radar last
    /// refreshed, or now.
    fn radar_player(view: &View) -> Point {
        view.radar_shown().player
    }

    #[test]
    fn by_the_engine_the_radar_holds_between_refreshes_eight_ticks_apart() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 1);
        let refreshed = view.shown_position();
        assert_eq!(radar_player(&view), refreshed, "refreshed on the first");
        for tick in 2..=8 {
            ticks(&mut view, 1);
            assert_eq!(radar_player(&view), refreshed, "held on tick {tick}");
        }
        assert_ne!(view.shown_position(), refreshed, "the ship moved");
        ticks(&mut view, 1);
        assert_eq!(radar_player(&view), view.shown_position(), "the 9th");
        assert_eq!(
            radar_marks(&view)[0],
            small_oval(
                926,
                radar::radar_offset(-600.0, view.shown_position().y, 96)
            )
        );
    }

    /// `view`'s flight with [`RuleKey::HudCadence`] read the Bible's way:
    /// the HUD every frame.
    fn every_frame(view: View) -> View {
        view.with_rules(Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Bible))
    }

    #[test]
    fn by_the_other_reading_the_radar_follows_the_ship_every_frame() {
        let mut view = every_frame(flight());
        let before = dots_on_radar(&drawn(&view));
        view.input(&key(Key::Up, true));
        for _ in 0..45 {
            ticks(&mut view, 1);
            assert_eq!(radar_player(&view), view.shown_position());
        }
        view.tick(TICK / 2);
        let shown = view.shown_position();
        assert!(shown.y < -10.0, "{shown:?}");
        assert_eq!(radar_player(&view), shown, "between ticks too");
        let after = dots_on_radar(&drawn(&view));
        let y = |world| radar::radar_offset(world, shown.y, 96);
        assert_eq!(
            radar_marks(&view),
            [
                small_oval(926, y(-600.0)),
                small_oval(935, y(-200.0)),
                PLAYER_PIXEL
            ]
        );
        assert!(after[0].1 > before[0].1, "{before:?} {after:?}");
    }

    #[test]
    fn while_landed_the_radar_shows_only_the_player_until_take_off() {
        for every in [false, true] {
            let view = flight_among(vec![
                site(128, (0.0, -600.0), StellarFlags::CAN_LAND),
                site(129, (6.0, -8.0), StellarFlags::CAN_LAND),
            ]);
            let mut view = if every { every_frame(view) } else { view };
            ticks(&mut view, 3);
            let flying = radar_marks(&view);
            assert_eq!(flying.len(), 3, "the Sun, the Moon and the player");
            land_now(&mut view);
            assert!(view.session().expect("flying").landed().is_some());
            assert!(view.radar_shown().blank, "at once, as _RedrawStatus");
            assert_eq!(radar_marks(&view), [PLAYER_PIXEL]);
            let list = drawn(&view);
            assert_eq!(dots_on_radar(&list), [(926, 96)]);
            view.take_off().expect("took off");
            assert!(!view.radar_shown().blank, "at once on take-off");
            assert_eq!(radar_marks(&view).len(), 3);
        }
    }

    #[test]
    fn a_new_system_shows_on_the_radar_at_once_leaving_the_timer_and_blink() {
        let mut view = landed_view();
        ticks(&mut view, 3);
        let cadence = view.hud_cadence;
        move_to(&mut view, 131, 140);
        view.resync();
        assert_eq!(view.hud_cadence, cadence, "neither timed nor flipped");
        let shown = view.radar_shown();
        assert_eq!(shown.player, view.shown_position());
        let stellars: Vec<Point> = shown.stellars.iter().map(|s| s.position).collect();
        let scene: Vec<Point> = view
            .scene()
            .expect("a scene")
            .stellars()
            .iter()
            .map(|s| s.position)
            .collect();
        assert_eq!(stellars, scene, "Alpha Centauri's");
    }

    #[test]
    fn an_arrival_shows_the_new_systems_stellars_at_once() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let leaving = view.session().expect("flying").system();
        for _ in 0..1000 {
            let cadence = view.hud_cadence;
            view.tick(TICK);
            if view.session().expect("flying").system() != leaving {
                let now = u64::try_from(crate::time::ticks(view.elapsed)).expect("in range");
                let mut probe = cadence;
                assert!(!probe.advance(now).radar, "not a refresh tick");
                assert_eq!(view.hud_cadence.blink_on(), cadence.blink_on());
                break;
            }
        }
        assert_ne!(view.session().expect("flying").system(), leaving);
        let stellars: Vec<Point> = view
            .radar_shown()
            .stellars
            .iter()
            .map(|s| s.position)
            .collect();
        assert_eq!(stellars, [at(0.0, 0.0)], "Alpha Centauri's");
        assert_eq!(view.radar_shown().player, view.shown_position());
    }

    #[test]
    fn the_targets_blip_blinks_bright_every_eight_ticks_under_both_readings() {
        for every in [false, true] {
            let (_, chance) = scripted(&placed(850, 650, 90));
            let view = FlightView::new(trafficked(&[130], 1, 129, 3))
                .with_chance(chance)
                .with_behaviour(Rc::new(Still));
            let mut view = if every { every_frame(view) } else { view };
            view.tick(TICK);
            tap(&mut view, TARGET_KEY);
            assert!(view.session().expect("flying").target().is_some());
            ticks(&mut view, 7);
            let mut seen = Vec::new();
            for _ in 0..24 {
                ticks(&mut view, 1);
                let [RadarMark::Pixel { color, .. }] = blips(&view)[..] else {
                    panic!("one blip: {:?}", blips(&view));
                };
                seen.push(color);
            }
            let mut expected = vec![RadarColor::Dim; 8];
            expected.extend([RadarColor::Bright; 8]);
            expected.extend([RadarColor::Dim; 8]);
            assert_eq!(seen, expected, "every frame: {every}");
            let list = drawn(&view);
            let bright = list.iter().filter(|command| {
                matches!(command, DrawCommand::Dot { color, .. } if *color == layout().bright_radar)
            });
            assert_eq!(bright.count(), 1, "the player alone, the target dim");
        }
    }

    #[test]
    fn the_bars_show_the_sessions_reserves() {
        let view = flight();
        let reserves = reserves(&view);
        assert_eq!(reserves, ShipStats::new(FIELDS, &[]).full());
        let mut expected = DrawList::new();
        hud::draw(
            &mut expected,
            view.status_bar().expect("a status bar"),
            &HudState {
                radar: &RadarScene::default(),
                blink_on: false,
                reserves,
                nav: NavDisplay::Off,
                cargo: None,
                disabled: false,
            },
            None,
        );
        assert_eq!(lines(&drawn(&view)), lines(&expected));
    }

    #[test]
    fn an_unreadable_status_bar_says_why_and_flight_goes_on() {
        let barless = FakeCatalog {
            bar: Err("no ïntf 128".to_owned()),
            ..catalog()
        };
        let mut view = FlightView::new(barless);
        assert_eq!(view.status_bar(), Err("no ïntf 128"));
        view.input(&key(Key::Up, true));
        ticks(&mut view, 5);
        assert_eq!(player(&view), stepped(start(), THRUST, 5));
        let list = drawn(&view);
        let mut reason = DrawList::new();
        hud::draw_unavailable(&mut reason, "no ïntf 128");
        assert_eq!(list.iter().last(), reason.iter().next());
        assert_eq!(lines(&list), [], "no bars");
        assert_eq!(dots_on_radar(&list), [], "no radar");
        assert_eq!(sprites(&list).len(), 3, "stellars and ship");
    }

    /// `ïntf` 129: [`layout`] in another shield colour, over `PICT` 705.
    fn bar_129() -> StatusBarLayout {
        StatusBarLayout {
            shield_color: Color::rgba(0, 255, 255, 255),
            status_bkgnd: 705,
            ..layout()
        }
    }

    /// `catalog` where ship type `ship` (given a record if it has none)
    /// belongs to gövt 150, whose `Interface` is `interface`, and with
    /// `ïntf` 129 ([`bar_129`]).
    fn governed(mut catalog: FakeCatalog, ship: i16, interface: i16) -> FakeCatalog {
        if !catalog.ships.iter().any(|record| record.id == ShipId(ship)) {
            let mut record = shipbuying().ships[0].clone();
            record.id = ShipId(ship);
            catalog.ships.push(record);
        }
        for record in &mut catalog.ships {
            if record.id == ShipId(ship) {
                record.inherent_govt = Some(GovtId(150));
            }
        }
        catalog.interfaces = vec![(150, interface)];
        catalog.other_bars = vec![(129, bar_129())];
        catalog
    }

    #[test]
    fn a_ship_type_whose_government_has_its_own_interface_shows_that_bar() {
        let view = FlightView::new(governed(catalog(), 128, 129));
        assert_eq!(
            view.status_bar(),
            Ok(&StatusBar {
                layout: bar_129(),
                background: Some(hud::Background {
                    id: 705,
                    width: 194.0,
                    height: 767.0,
                }),
            })
        );
        let list = drawn(&view);
        assert!(
            list.iter().any(|c| *c
                == DrawCommand::Picture {
                    image: ImageKey::picture(705),
                    top_left: at(830.0, 0.0),
                }),
            "{list:?}"
        );
        for interface in [127, 0, -1] {
            let view = FlightView::new(governed(catalog(), 128, interface));
            assert_eq!(
                view.status_bar().map(|bar| &bar.layout),
                Ok(&layout()),
                "{interface}"
            );
        }
    }

    #[test]
    fn a_session_that_cannot_start_has_no_status_bar() {
        let failed = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        let view = FlightView::new(failed);
        assert_eq!(view.status_bar(), Err("no chär to start from"));
    }

    // Frame rate.

    /// Flies a script for three seconds of frames at `fps`, each redraw at
    /// `n / fps` seconds to the nanosecond: thrust for a second, turn
    /// right for half a second, reverse for a second, then coast.
    fn fly(fps: u64) -> ShipState {
        let mut view = flight();
        let script = [
            (0, Key::Up, true),
            (2, Key::Up, false),
            (2, Key::Right, true),
            (3, Key::Right, false),
            (3, Key::Down, true),
            (5, Key::Down, false),
        ];
        let mut last = 0;
        for frame in 1..=3 * fps {
            let half_second = (frame - 1) * 2 / fps;
            if (frame - 1) * 2 % fps == 0 {
                for &(_, k, pressed) in script.iter().filter(|(at, ..)| *at == half_second) {
                    view.input(&key(k, pressed));
                }
            }
            let now = frame * 1_000_000_000 / fps;
            view.tick(Duration::from_nanos(now - last));
            last = now;
        }
        player(&view)
    }

    #[test]
    fn the_same_keys_fly_the_same_at_30_60_and_120_fps() {
        let at_30 = fly(30);
        assert!(at_30.heading > 0.0, "{at_30:?}");
        assert!(at_30.position.y < 0.0, "{at_30:?}");
        assert_eq!(fly(60), at_30);
        assert_eq!(fly(120), at_30);
    }

    // Landing.

    /// The fake catalog with system 130 holding just `sites`.
    fn flight_among(sites: Vec<LandingSite>) -> View {
        FlightView::new(FakeCatalog { sites, ..catalog() })
    }

    const LAND: Key = Key::Char('l');

    /// The message drawn at its place, if any.
    fn message(view: &View) -> Option<DrawCommand> {
        drawn(view)
            .iter()
            .find(|c| matches!(c, DrawCommand::Text { origin, .. } if *origin == message_at()))
            .cloned()
    }

    /// Where the message's first line is drawn: the message box's
    /// top-left.
    fn message_at() -> Point {
        hud::message_box(VIEW_SIZE).min
    }

    /// The message `text` as a flight without metrics draws it.
    fn message_drawn(text: &str) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: hud::MESSAGE_FONT,
            origin: at(25.0, 724.0),
            size: 12.0,
            wrap_width: Some(755.0),
            color: Color::WHITE,
        }
    }

    /// Asserts the message line says `text` now, for `n` ticks of flight
    /// in all, and not after.
    #[track_caller]
    fn assert_lasts(view: &mut View, text: &str, n: u16) {
        assert_eq!(view.message(), Some(text));
        ticks(view, u32::from(n) - 1);
        assert_eq!(
            view.message(),
            Some(text),
            "still shown after {} ticks",
            n - 1
        );
        ticks(view, 1);
        assert_eq!(view.message(), None, "gone after {n} ticks");
    }

    /// L pressed, a tick for the dock approach, and L pressed again:
    /// clearance, then landing.
    fn land_now(view: &mut View) {
        tap(view, LAND);
        ticks(view, 1);
        tap(view, LAND);
    }

    /// The name of the ship `view` flies, as traffic control says it.
    fn ship_name(view: &View) -> String {
        view.session()
            .expect("flying")
            .pilot()
            .ship_name()
            .unwrap_or_default()
            .to_owned()
    }

    #[test]
    fn l_requests_clearance_an_early_second_l_beeps_too_far_and_a_later_l_lands() {
        // The Moon (129), in the scene, over the ship.
        let mut view = flight_among(vec![site(129, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.input(&key(LAND, true)), ScreenAction::None);
        assert_eq!(view.take_landing(), None, "requested, not landed");
        assert_eq!(view.session().expect("flying").landed(), None);
        assert_eq!(nav_target(&view), Some(StellarId(129)));
        assert_eq!(view.message(), None, "requested over it: silent");
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"], "the HUD shows it");
        assert_eq!(view.take_sounds(), [], "no sound for a silent request");
        assert_eq!(view.input(&key(LAND, true)), ScreenAction::None);
        assert_eq!(view.take_landing(), None, "no approach yet");
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::Refused)],
            "too far before the approach clears: _beepSnd[3] (@0x66c23 -> 0x676db, @0x676fe)"
        );

        view.input(&key(LAND, false));
        ticks(&mut view, 1);
        let ship = ship_name(&view);
        let cleared = format!("Cleared to land, {ship}. Commence final approach.");
        assert_eq!(view.message(), Some(cleared.as_str()));
        assert_eq!(message(&view), Some(message_drawn(&cleared)));
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TrafficControl)]);
        assert_eq!(view.input(&key(LAND, true)), ScreenAction::None);
        assert_eq!(
            view.session().expect("flying").landed(),
            Some(StellarId(129))
        );
        assert_eq!(view.take_landing(), Some(StellarId(129)));
        assert_eq!(view.take_landing(), None, "taken");
        assert_eq!(view.message(), None);
        assert_eq!(message(&view), None);
        assert_eq!(LAND_KEY, LAND);
    }

    #[test]
    fn the_picks_roll_their_ranges_and_decide_only_their_words() {
        // A planet: Rand(3), then Rand(2); 0 names the stellar, and 0
        // adds the ship.
        for (draws, opening, named) in [([0, 0], 0, 0), ([2, 1], 2, 1), ([1, 0], 1, 0)] {
            let (script, mut chance) = scripted(&draws);
            let picks = RequestPicks::roll(DockKind::Planet, &mut chance);
            assert_eq!(picks, RequestPicks { opening, named });
            assert_eq!(script.borrow().asked, [3, 2]);
        }
        let (script, mut chance) = scripted(&[1, 1]);
        assert_eq!(
            RequestPicks::roll(DockKind::Hypergate, &mut chance),
            RequestPicks {
                opening: 1,
                named: 1
            }
        );
        assert_eq!(script.borrow().asked, [2, 2], "a hypergate's Rand(2)");
        let (script, mut chance) = scripted(&[2, 1]);
        assert_eq!(
            GreetingPicks::roll(DockKind::Station, &mut chance),
            GreetingPicks {
                opening: 2,
                welcome: 1
            }
        );
        assert_eq!(script.borrow().asked, [3, 2]);
        let (script, mut chance) = scripted(&[1]);
        assert_eq!(
            GreetingPicks::roll(DockKind::Hypergate, &mut chance).opening,
            1
        );
        assert_eq!(script.borrow().asked, [3], "no second sentence");
        // Only 0 names, only 0 adds the ship, and anything but 0 welcomes.
        let line = |opening, named| {
            request_line(
                DockKind::Planet,
                "Earth",
                "Kestrel",
                RequestPicks { opening, named },
            )
        };
        assert!(line(0, 1).starts_with("Earth "));
        assert!(!line(1, 1).contains("Earth"));
        assert!(!line(2, 1).contains("Earth"));
        assert!(line(1, 0).contains(", Kestrel"));
        assert!(!line(1, 1).contains("Kestrel"));
        let greeting = |welcome| {
            approach_greeting(
                DockKind::Planet,
                "Earth",
                "Kestrel",
                GreetingPicks {
                    opening: 2,
                    welcome,
                },
            )
        };
        assert!(greeting(0).ends_with(COMMENCE_FINAL_APPROACH));
        assert!(greeting(1).ends_with("Welcome to Earth. "));
        assert!(greeting(2).ends_with("Welcome to Earth. "));
    }

    #[test]
    fn the_selecting_l_shows_the_requests_line_then_a_tick_near_shows_the_greeting() {
        // The Moon (129) at (300, −200), the nearer of Sol's two.
        let mut view = flight();
        let ship = ship_name(&view);
        tap(&mut view, LAND);
        assert_eq!(nav_target(&view), Some(StellarId(129)));
        let line = format!("Moon traffic control reads you, {ship}. Begin initial approach.");
        assert_eq!(view.message(), Some(line.as_str()));
        assert_lasts(&mut view, &line, 250);
        // Flown within 249 pixels, the approach greets on that tick, as a
        // message of its own.
        let mut view = flight_among(vec![site(129, (0.0, -300.0), StellarFlags::CAN_LAND)]);
        tap(&mut view, LAND);
        let line = format!("Moon traffic control reads you, {ship}. Begin initial approach.");
        assert_eq!(view.message(), Some(line.as_str()));
        view.input(&key(Key::Up, true));
        let greeting = format!("Cleared to land, {ship}. Commence final approach.");
        let mut far = true;
        for _ in 0..200 {
            let before = view.session().expect("flying").player().position;
            ticks(&mut view, 1);
            if view.message() == Some(greeting.as_str()) {
                let at = view.session().expect("flying").player().position;
                assert!((at.y + 300.0).trunc().abs() <= 249.0, "{at:?}");
                assert!((before.y + 300.0).trunc().abs() > 249.0, "{before:?}");
                far = false;
                break;
            }
            assert_eq!(
                view.message(),
                Some(line.as_str()),
                "{before:?} {greeting:?}"
            );
        }
        assert!(!far, "never greeted");
        view.input(&key(Key::Up, false));
        assert_lasts(&mut view, &greeting, 250);
    }

    #[test]
    fn holding_l_holds_the_greeting_back_until_it_is_let_go() {
        let mut view = flight_among(vec![site(129, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        view.input(&key(LAND, true));
        assert_eq!(view.message(), None, "requested over it: silent");
        ticks(&mut view, 5);
        assert_eq!(view.message(), None, "held");
        view.input(&key(LAND, false));
        ticks(&mut view, 1);
        let ship = ship_name(&view);
        assert_eq!(
            view.message(),
            Some(format!("Cleared to land, {ship}. Commence final approach.").as_str())
        );
    }

    #[test]
    fn an_uninhabited_stellar_out_of_reach_says_too_far_over_no_response() {
        let uninhabited = StellarFlags::CAN_LAND | StellarFlags::UNINHABITED;
        let mut view = flight_among(vec![site(128, (0.0, -900.0), uninhabited)]);
        tap(&mut view, LAND);
        assert_lasts(&mut view, TOO_FAR_PLANET, TOO_FAR_TICKS);
        let mut view = flight_among(vec![site(128, (0.0, 0.0), uninhabited)]);
        tap(&mut view, LAND);
        assert_eq!(view.take_landing(), Some(StellarId(128)), "one press");
    }

    #[test]
    fn a_denial_says_so_and_selects_anyway() {
        let strict = LandingSite {
            min_status: 1,
            ..site(
                129,
                (0.0, 0.0),
                StellarFlags::CAN_LAND | StellarFlags::STATION,
            )
        };
        let mut view = flight_among(vec![strict]);
        tap(&mut view, LAND);
        assert_eq!(nav_target(&view), Some(StellarId(129)), "selected anyway");
        assert_lasts(&mut view, "Docking request denied.", 250);
    }

    #[test]
    fn a_repeat_or_release_of_l_does_nothing() {
        let mut view = flight_among(vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)]);
        view.input(&held(LAND));
        view.input(&key(LAND, false));
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.session().expect("flying").landed(), None);
        assert_eq!(nav_target(&view), None, "nothing selected");
        let mut far = flight();
        far.input(&held(LAND));
        assert_eq!(far.message(), None, "no refusal either");
    }

    #[test]
    fn each_refusal_says_why_in_the_originals_words() {
        // L, then L again: the first requests clearance where it can.
        let refused = |sites: Vec<LandingSite>| {
            let mut view = flight_among(sites);
            tap(&mut view, LAND);
            tap(&mut view, LAND);
            assert_eq!(view.take_landing(), None);
            view.message().map(str::to_owned)
        };
        let centre = |flags, min_status| LandingSite {
            min_status,
            ..site(140, (0.0, 0.0), flags)
        };
        let station = StellarFlags::CAN_LAND | StellarFlags::STATION;
        assert_eq!(
            refused(vec![site(140, (0.0, -600.0), StellarFlags::CAN_LAND)]).as_deref(),
            Some("You're too far away to land on this planet.")
        );
        assert_eq!(
            refused(vec![site(140, (0.0, -600.0), station)]).as_deref(),
            Some("You're too far away to dock at this station.")
        );
        assert_eq!(
            refused(vec![centre(StellarFlags::CAN_LAND, 1)]).as_deref(),
            Some("Landing request denied.")
        );
        assert_eq!(
            refused(vec![centre(station, 32767)]).as_deref(),
            Some("Docking request denied.")
        );
        // Only a selected stellar reaches the dock request's not-landable
        // line (@0x6a945-0x6a94c, @0x66977-0x669ae).
        let mut view = flight_among(vec![centre(0, 0)]);
        tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
        tap(&mut view, LAND);
        assert_eq!(nav_target(&view), Some(StellarId(140)), "kept");
        assert_eq!(
            view.message(),
            Some("Your ship is unable to land on  . The planet's environment is too hostile."),
            "140 is not in Sol's scene: no name"
        );
    }

    /// Says `earlier` in `view`, taps L, and asserts the message line
    /// still says `earlier`: the press said nothing and cleared nothing.
    #[track_caller]
    fn l_says_nothing(view: &mut View) {
        view.say_for("earlier", MESSAGE_TICKS);
        tap(view, LAND);
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.message(), Some("earlier"), "unchanged");
    }

    #[test]
    fn l_with_no_stellars_says_nothing() {
        // Nothing to select, so no dock request (@0x6a94c).
        let mut view = flight_among(Vec::new());
        l_says_nothing(&mut view);
    }

    #[test]
    fn l_with_nothing_landable_and_no_target_says_nothing() {
        // `_FindNearestStellar` gives -1: no dock request (@0x6a94c).
        let mut view = flight_among(vec![site(140, (0.0, 0.0), 0)]);
        l_says_nothing(&mut view);
        assert_eq!(nav_target(&view), None, "nothing selected");
        let mut fresh = flight_among(vec![site(140, (0.0, 0.0), 0)]);
        tap(&mut fresh, LAND);
        assert_eq!(fresh.message(), None, "no line either");
    }

    #[test]
    fn l_during_a_jump_says_nothing() {
        // The jump words send the land key to 0x6a95d (@0x6a8cb-0x6a8e3).
        for mut view in through_a_jump(flight) {
            l_says_nothing(&mut view);
        }
    }

    #[test]
    fn a_ship_moving_too_fast_is_told_so() {
        let big = |flags| LandingSite {
            frame_size: Some((400, 400)),
            ..site(140, (0.0, 0.0), flags)
        };
        for (flags, expected) in [
            (
                StellarFlags::CAN_LAND,
                "You're moving too fast to land on this planet.",
            ),
            (
                StellarFlags::CAN_LAND | StellarFlags::STATION,
                "You're moving too fast to dock at this station.",
            ),
        ] {
            let mut view = flight_among(vec![big(flags)]);
            view.input(&key(Key::Up, true));
            ticks(&mut view, 15);
            land_now(&mut view);
            assert_eq!(view.take_landing(), None);
            assert_eq!(view.message(), Some(expected));
            assert_eq!(nav_target(&view), Some(StellarId(140)), "kept: try again");
        }
    }

    #[test]
    fn a_new_message_replaces_the_old_and_its_time() {
        let mut view = flight();
        view.say_for("A", 420);
        ticks(&mut view, 10);
        view.say_for("B", 240);
        assert_lasts(&mut view, "B", 240);
    }

    #[test]
    fn a_message_waits_out_the_map() {
        let mut view = flight();
        view.say_for("A", 240);
        ticks(&mut view, 40);
        tap(&mut view, MAP_KEY);
        assert!(view.map_open());
        ticks(&mut view, 300);
        tap(&mut view, MAP_KEY);
        assert!(!view.map_open());
        assert_lasts(&mut view, "A", 200);
    }

    #[test]
    fn starting_a_jump_clears_only_too_close() {
        let mut view = flight();
        plot(&mut view, 131);
        tap(&mut view, JUMP);
        assert_eq!(view.message(), Some(TOO_CLOSE));
        fly_out(&mut view);
        assert_eq!(view.message(), Some(TOO_CLOSE), "still shown");
        tap(&mut view, JUMP);
        assert!(view.preparing_jump(), "accepted");
        assert_eq!(view.message(), None, "#42 cleared");

        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, JUMP);
        assert!(view.preparing_jump(), "accepted");
        assert_eq!(view.message(), Some(MISSION_FAILED), "kept");
    }

    #[test]
    fn too_far_at_a_hypergate_or_wormhole_says_so_for_360_ticks() {
        for (flags2, text) in [(0x1000, TOO_FAR_HYPERGATE), (0x2000, TOO_FAR_WORMHOLE)] {
            let mut view = flight_among(vec![LandingSite {
                flags2,
                ..site(128, (0.0, -600.0), StellarFlags::CAN_LAND)
            }]);
            tap(&mut view, LAND);
            tap(&mut view, LAND);
            assert_lasts(&mut view, text, TOO_FAR_TICKS);
        }
    }

    #[test]
    fn too_fast_at_a_hypergate_or_wormhole_says_so_for_360_ticks() {
        for (flags2, text) in [(0x1000, TOO_FAST_HYPERGATE), (0x2000, TOO_FAST_WORMHOLE)] {
            let mut view = flight_among(vec![LandingSite {
                flags2,
                frame_size: Some((400, 400)),
                ..site(128, (0.0, 0.0), StellarFlags::CAN_LAND)
            }]);
            view.input(&key(Key::Up, true));
            ticks(&mut view, 15);
            land_now(&mut view);
            assert_eq!(view.take_landing(), None);
            assert_lasts(&mut view, text, TOO_FAR_TICKS);
        }
    }

    #[test]
    fn a_not_landable_station_or_planet_is_named_for_250_ticks() {
        for (flags, text) in [
            (
                StellarFlags::STATION,
                "Your ship is unable to dock at  Earth. The station's hull integrity is too unstable.",
            ),
            (
                0,
                "Your ship is unable to land on  Earth. The planet's environment is too hostile.",
            ),
        ] {
            // Selected: only a selected stellar reaches the dock request
            // (@0x6a945-0x6a94c).
            let mut view = flight_among(vec![site(128, (0.0, 0.0), flags)]);
            tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
            tap(&mut view, LAND);
            assert_lasts(&mut view, text, LANDING_TICKS);
        }
    }

    #[test]
    fn the_refusal_is_drawn_until_its_ticks_have_passed() {
        let mut view = flight();
        ticks(&mut view, 3);
        land_now(&mut view);
        let shown = Some(message_drawn(TOO_FAR_PLANET));
        assert_eq!(message(&view), shown);
        ticks(&mut view, 359);
        assert_eq!(message(&view), shown);
        assert_eq!(view.message(), Some(TOO_FAR_PLANET));
        ticks(&mut view, 1);
        assert_eq!(message(&view), None);
        assert_eq!(view.message(), None);
        assert_eq!(TOO_FAR_TICKS, 360);
    }

    #[test]
    fn the_message_draws_over_the_starfield_and_under_the_ships() {
        let mut view = flight();
        land_now(&mut view);
        let list = drawn(&view);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        let mut stars = DrawList::new();
        starfield::draw(&mut stars, &view.camera());
        let stars: Vec<&DrawCommand> = stars.iter().collect();
        assert!(!stars.is_empty());
        assert_eq!(commands[..stars.len()], stars[..], "the starfield first");
        assert_eq!(
            commands[stars.len()],
            &message_drawn(TOO_FAR_PLANET),
            "then the message"
        );
        let sprite = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Sprite { .. }))
            .expect("the player's sprite");
        assert!(stars.len() < sprite, "the ship over it");
    }

    #[test]
    fn a_long_message_wraps_in_the_box_by_the_metrics() {
        let mut view = flight().with_metrics(Rc::new(crate::text::fixture::MonoMetrics));
        land_now(&mut view);
        let text = vec!["abcd"; 60].join(" ");
        view.say_for(text.clone(), MESSAGE_TICKS);
        let mut expected = DrawList::new();
        hud::draw_message(
            &mut expected,
            &text,
            Some(&crate::text::fixture::MonoMetrics),
        );
        let list = drawn(&view);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        let first = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { origin, .. } if *origin == message_at()))
            .expect("the message");
        let expected: Vec<&DrawCommand> = expected.iter().collect();
        assert_eq!(expected.len(), 2);
        assert_eq!(commands[first..first + 2], expected[..]);
    }

    #[test]
    fn a_new_refusal_shows_afresh() {
        let mut view = flight();
        land_now(&mut view);
        assert_eq!(view.message(), Some(TOO_FAR_PLANET));
        view.input(&key(LAND, true));
        ticks(&mut view, 300);
        view.input(&key(LAND, false));
        view.input(&key(LAND, true));
        ticks(&mut view, 100);
        assert_eq!(
            view.message(),
            Some(TOO_FAR_PLANET),
            "360 ticks from the second"
        );
    }

    #[test]
    fn taking_off_draws_the_ship_at_the_stellar_on_the_next_frame() {
        let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(140)));
        assert_eq!(view.take_off(), Some(StellarId(140)));
        assert_eq!(view.session().expect("flying").landed(), None);
        assert_eq!(view.alpha(), 0.0);
        assert_eq!(view.shown_position(), at(6.0, -8.0));
        assert_eq!(view.camera().center(), at(6.0, -8.0));
        assert_eq!(view.take_off(), None, "not landed");
        // It flies on from there.
        view.input(&key(Key::Up, true));
        ticks(&mut view, 2);
        assert!(player(&view).position.y < -8.0, "{:?}", player(&view));
    }

    #[test]
    fn a_refusal_is_forgotten_once_the_ship_lands() {
        // 20 wide: within 35 on each axis is over it.
        let mut view = flight_among(vec![site(140, (0.0, -40.0), StellarFlags::CAN_LAND)]);
        land_now(&mut view);
        assert_eq!(view.message(), Some(TOO_FAR_PLANET));
        // Nudge the ship and drift over the planet.
        view.input(&key(Key::Up, true));
        ticks(&mut view, 1);
        view.input(&key(Key::Up, false));
        for _ in 0..100 {
            if player(&view).position.y <= -6.0 {
                break;
            }
            ticks(&mut view, 1);
        }
        assert!(player(&view).position.y <= -6.0, "{:?}", player(&view));
        assert_eq!(view.message(), Some(TOO_FAR_PLANET), "still on screen");
        view.input(&key(LAND, true));
        assert_eq!(view.take_landing(), Some(StellarId(140)));
        assert_eq!(view.message(), None);
        assert_eq!(message(&view), None);
        assert_eq!(view.take_off(), Some(StellarId(140)));
        let greeting = view.message().expect("the greeting");
        assert!(greeting.starts_with(TAKE_OFF_LEADS[0]), "{greeting}");
    }

    #[test]
    fn landing_in_a_flight_that_never_started_does_nothing() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        view.input(&key(LAND, true));
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.message(), None);
        assert_eq!(view.take_off(), None);
    }

    /// Only the landing beep sounds at touchdown: the stellar's own sound
    /// is its spaceport's ambient sound, which plays once the spaceport is
    /// frontmost (see the spaceport's
    /// `the_ambient_sound_plays_on_the_first_frame_then_after_each_wait`).
    #[test]
    fn l_over_a_stellar_sounds_only_the_landing_at_touchdown() {
        let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        assert_eq!(view.take_sounds(), []);
        tap(&mut view, LAND);
        assert_eq!(view.take_sounds(), [], "a request over it is silent");
        ticks(&mut view, 1);
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::TrafficControl)],
            "the greeting"
        );
        tap(&mut view, LAND);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::Landed)]);
        assert_eq!(view.take_sounds(), [], "taken");
        view.take_off();
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TookOff)]);
    }

    #[test]
    fn m_i_and_p_are_refused_for_15_ticks_after_a_take_off() {
        let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(140)));
        assert_eq!(view.take_off(), Some(StellarId(140)));
        let _ = view.take_sounds();
        for tick in 0..15 {
            tap(&mut view, MAP_KEY);
            assert!(!view.map_open(), "after {tick} ticks");
            tap(&mut view, MISSION_INFO_KEY);
            assert_eq!(view.take_mission_info_request(), None, "after {tick} ticks");
            assert_ne!(view.message(), Some(NO_ACTIVE_MISSIONS), "silently");
            tap(&mut view, PLAYER_INFO_KEY);
            assert!(!view.take_player_info_request(), "after {tick} ticks");
            assert_eq!(
                view.take_sounds(),
                [Sound::Sim(SimSound::Refused)],
                "after {tick} ticks"
            );
            ticks(&mut view, 1);
        }
        tap(&mut view, MISSION_INFO_KEY);
        assert_eq!(view.message(), Some(NO_ACTIVE_MISSIONS), "after 15 ticks");
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::Refused)],
            "no active missions, with the beep (@0x70557)"
        );
        tap(&mut view, PLAYER_INFO_KEY);
        assert!(view.take_player_info_request(), "after 15 ticks");
        tap(&mut view, MAP_KEY);
        assert!(view.map_open(), "after 15 ticks");
    }

    #[test]
    fn thrust_sounds_as_it_starts_and_stops() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::ThrustStarted)]);
        view.input(&key(Key::Up, false));
        ticks(&mut view, 1);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::ThrustStopped)]);
    }

    #[test]
    fn a_flight_that_never_started_makes_no_sounds() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(view.take_sounds(), []);
    }

    // Hyperspace.

    const MAP: Key = Key::Char('m');
    const JUMP: Key = Key::Char('j');

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Presses and releases `k`.
    fn tap(view: &mut View, k: Key) {
        view.input(&key(k, true));
        view.input(&key(k, false));
    }

    /// Where system `id` is on the course map.
    fn on_map(view: &View, id: i16) -> Point {
        let map = view.course_map();
        let system = map.model().system(SystemId(id)).expect("on the map");
        map.view().world_to_screen(system.position())
    }

    /// Clicks the left button at `at`.
    fn click(view: &mut View, at: Point) {
        for pressed in [true, false] {
            view.input(&Input::PointerButton {
                button: crate::MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    /// Opens the map, picks system `id` and closes the map again.
    fn plot(view: &mut View, id: i16) {
        tap(view, MAP);
        let at = on_map(view, id);
        click(view, at);
        tap(view, MAP);
        assert!(!view.map_open());
    }

    /// Thrusts straight up until the ship is at least the minimum jump
    /// distance from the centre, then lets go of Up.
    fn fly_out(view: &mut View) {
        view.input(&key(Key::Up, true));
        for _ in 0..2000 {
            if player(view).position.length() >= MIN_JUMP_DISTANCE {
                break;
            }
            view.tick(TICK);
        }
        view.input(&key(Key::Up, false));
        assert!(
            player(view).position.length() >= MIN_JUMP_DISTANCE,
            "{:?}",
            player(view)
        );
    }

    #[test]
    fn the_course_map_starts_on_the_current_system_with_no_course() {
        let view = flight();
        assert!(!view.map_open());
        assert_eq!(view.course_map().mode(), MapMode::Course);
        assert_eq!(view.course_map().current(), Some(SystemId(130)));
        assert_eq!(view.course_map().route(), []);
        assert_eq!(view.jump_effect(), None);
        assert_eq!((MAP_KEY, JUMP_KEY), (MAP, JUMP));
    }

    #[test]
    fn m_opens_the_map_over_everything_and_lets_go_of_the_flight_keys() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        assert_eq!(view.input(&key(MAP, true)), ScreenAction::None);
        assert!(view.map_open());
        let mut map = DrawList::new();
        view.course_map().draw(&mut map);
        assert_eq!(drawn(&view), map, "only the map");
        // Holding M, or letting it go, keeps it open.
        view.input(&held(MAP));
        view.input(&key(MAP, false));
        assert!(view.map_open());
        view.input(&key(MAP, true));
        assert!(!view.map_open());
        ticks(&mut view, 5);
        assert_eq!(player(&view), start(), "Up was let go");
    }

    #[test]
    fn while_the_map_is_open_flight_is_paused_and_keys_go_to_the_map() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        let flying = player(&view);
        tap(&mut view, MAP);
        let fitted = *view.course_map().view();
        ticks(&mut view, 30);
        assert_eq!(player(&view), flying, "paused");
        view.input(&key(Key::Left, true));
        assert_ne!(*view.course_map().view(), fitted, "Left pans the map");
        view.input(&key(LAND, true));
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), None);
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.jump_effect(), None);
        tap(&mut view, MAP);
        ticks(&mut view, 1);
        assert_eq!(player(&view).heading, 0.0, "Left went to the map");
    }

    #[test]
    fn a_click_on_a_system_plots_the_course_there_and_the_map_shows_it() {
        let mut view = flight();
        tap(&mut view, MAP);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        assert!(view.map_open());
        let course = [SystemId(131)];
        assert_eq!(view.session().expect("flying").course(), course);
        assert_eq!(view.course_map().route(), course);
        assert_eq!(view.course_map().current(), Some(SystemId(130)));
        let barnard = on_map(&view, 132);
        click(&mut view, barnard);
        assert_eq!(view.session().expect("flying").course(), []);
        assert_eq!(view.course_map().route(), []);
        assert!(
            texts(&drawn(&view)).contains(&"No hyperspace route".to_owned()),
            "{:?}",
            texts(&drawn(&view))
        );
    }

    #[test]
    fn closing_the_map_abandons_its_drag() {
        let mut view = flight();
        tap(&mut view, MAP);
        let alpha = on_map(&view, 131);
        view.input(&Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed: true,
            at: alpha,
        });
        view.close_map();
        assert!(!view.map_open());
        tap(&mut view, MAP);
        view.input(&Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed: false,
            at: alpha,
        });
        assert_eq!(
            view.course_map().selected(),
            Some(SystemId(130)),
            "no click: the preselect"
        );
        assert_eq!(view.session().expect("flying").course(), []);
    }

    /// The mode-3 link the open course map draws, as (from, to) on the
    /// map; none while it is shut.
    fn hyper_selected_links(view: &View) -> Vec<(Point, Point)> {
        if !view.map_open() {
            return Vec::new();
        }
        view.course_map()
            .picture()
            .and_then(|picture| picture.mode3_link)
            .map(|(from, to)| (on_map(view, from.0), on_map(view, to.0)))
            .into_iter()
            .collect()
    }

    #[test]
    fn opening_the_map_selects_the_current_system_when_nothing_is_selected_in_mode_3() {
        let mut view = flight();
        tap(&mut view, MAP);
        assert_eq!(view.course_map().selected(), Some(SystemId(130)));
        assert!(texts(&drawn(&view)).contains(&"Sol (sÿst 130)".to_owned()));
        assert_eq!(hyper_selected_links(&view), []);
        tap(&mut view, MAP);
        plot(&mut view, 131);
        tap(&mut view, NAV_OFF_KEY);
        assert_eq!(nav_mode(&view), NavMode::Off);
        tap(&mut view, MAP);
        assert_eq!(
            view.course_map().selected(),
            Some(SystemId(130)),
            "the current system, not the course's 131"
        );
        assert_eq!(hyper_selected_links(&view), []);
    }

    #[test]
    fn opening_the_map_selects_and_draws_the_system_hyper_selected() {
        let mut view = FlightView::new(FakeCatalog {
            fan: true,
            ..catalog()
        });
        plot(&mut view, 131);
        tap(&mut view, Key::Char('\\'));
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(132))));
        tap(&mut view, MAP);
        let map = view.course_map();
        assert_eq!(map.selected(), Some(SystemId(132)));
        assert_eq!(map.route(), [SystemId(131)], "the course kept");
        assert_eq!(
            hyper_selected_links(&view),
            [(on_map(&view, 130), on_map(&view, 132))]
        );
        assert!(texts(&drawn(&view)).contains(&"Barnard (sÿst 132)".to_owned()));
    }

    #[test]
    fn a_click_on_the_map_draws_the_link_it_cues() {
        let mut view = flight();
        tap(&mut view, MAP);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        assert!(view.map_open());
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
        assert_eq!(hyper_selected_links(&view), [(on_map(&view, 130), alpha)]);
    }

    #[test]
    fn a_click_on_the_preselected_systems_place_selects_the_first_system_there() {
        // `_DoSystemMap`'s click (@0x12dd4-0x1314b) never reads the
        // preselect: it takes the first visible system at the place.
        let mut view = FlightView::new(FakeCatalog {
            reborn: true,
            ..catalog()
        });
        tap(&mut view, Key::Char('\\'));
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
        tap(&mut view, MAP);
        assert_eq!(view.course_map().selected(), Some(SystemId(131)));
        let alpha = on_map(&view, 131);
        assert_eq!(alpha, on_map(&view, 133), "Alpha Reborn stands there too");
        click(&mut view, alpha);
        assert_eq!(view.course_map().selected(), Some(SystemId(131)), "not 133");
        assert_eq!(course(&view), [SystemId(131)]);
        click(&mut view, alpha);
        assert_eq!(
            view.course_map().selected(),
            Some(SystemId(133)),
            "a click again moves on"
        );
    }

    const REFUSED: Sound = Sound::Sim(SimSound::Refused);

    #[test]
    fn j_says_why_a_jump_is_refused_in_the_originals_words() {
        let mut view = flight();
        view.take_sounds();
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), None, "a seasoned pilot, nothing selected");
        assert_eq!(view.take_sounds(), [], "and no beep");
        plot(&mut view, 131);
        view.take_sounds();
        view.input(&key(JUMP, true));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.take_sounds(), [REFUSED]);
        assert_eq!(message(&view), Some(message_drawn(TOO_CLOSE)));
        assert_lasts(&mut view, TOO_CLOSE, 360);

        let mut dry = dry();
        plot(&mut dry, 131);
        fly_out(&mut dry);
        dry.take_sounds();
        dry.input(&key(JUMP, true));
        assert_eq!(dry.jump_effect(), None);
        assert_eq!(dry.take_sounds(), [REFUSED]);
        assert_lasts(&mut dry, NO_FUEL, 240);
    }

    /// A flight whose ship holds 50 fuel, short of a jump's.
    fn dry() -> View {
        FlightView::new(FakeCatalog {
            fields: ShipFields { fuel: 50, ..FIELDS },
            ..catalog()
        })
    }

    /// `view` begun as a new pilot's, ticked once past the land hint, its
    /// message and sounds let go.
    fn new_pilot(mut view: View) -> View {
        view.begin();
        ticks(&mut view, 1);
        view.message = None;
        view.take_sounds();
        view
    }

    #[test]
    fn j_while_disabled_is_silent() {
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 100,
                starting: false,
            },
        ] {
            let mut view = new_pilot(flight());
            let Ok(session) = &mut view.session else {
                panic!("flying");
            };
            nova_sim::fixture::set_player_condition(session, condition);
            view.input(&key(JUMP, true));
            assert_eq!(view.message(), None, "{condition:?}");
            let session = view.session().expect("flying");
            assert_eq!(session.preparing_jump(), None, "{condition:?}");
            assert!(
                !view.take_sounds().contains(&REFUSED),
                "{condition:?}: no beep"
            );
            // Made intact again, the same J is told: the gate, not a dead key.
            let Ok(session) = &mut view.session else {
                panic!("flying");
            };
            nova_sim::fixture::set_player_condition(session, Condition::Intact);
            view.input(&key(JUMP, true));
            assert_eq!(view.take_sounds(), [REFUSED], "{condition:?}");
            assert_eq!(view.message(), Some(NO_DESTINATION), "{condition:?}");
        }
    }

    #[test]
    fn a_new_pilots_j_with_nothing_selected_says_no_destination_for_360() {
        let mut view = new_pilot(flight());
        view.input(&key(JUMP, true));
        assert_eq!(view.take_sounds(), [REFUSED]);
        assert_eq!(message(&view), Some(message_drawn(NO_DESTINATION)));
        assert_lasts(&mut view, NO_DESTINATION, 360);
    }

    #[test]
    fn a_new_pilot_with_a_hyperspace_selection_is_never_told_no_destination() {
        let mut view = new_pilot(flight());
        plot(&mut view, 131);
        view.take_sounds();
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), Some(TOO_CLOSE), "the zone's refusal");
        assert_eq!(view.take_sounds(), [REFUSED]);
    }

    #[test]
    fn a_new_pilot_short_of_fuel_is_not_told() {
        let mut view = new_pilot(dry());
        plot(&mut view, 131);
        fly_out(&mut view);
        view.message = None;
        view.take_sounds();
        view.input(&key(JUMP, true));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.message(), None);
        assert_eq!(view.take_sounds(), []);
    }

    #[test]
    fn a_new_pilot_short_of_fuel_is_told_by_the_other_reading() {
        let rules =
            Rulebook::default().with_override(RuleKey::NewbieFuelRefusal, RuleSource::Bible);
        let mut view = new_pilot(dry().with_rules(rules));
        plot(&mut view, 131);
        fly_out(&mut view);
        view.take_sounds();
        view.input(&key(JUMP, true));
        assert_eq!(view.take_sounds(), [REFUSED]);
        assert_eq!(view.message(), Some(NO_FUEL));
        assert_eq!(view.message.as_ref().map(|said| said.ticks), Some(240));
    }

    /// The land hint as the fixture words it, for the Moon, the nearest
    /// stellar to the centre.
    const MOON_HINT: &str = "Welcome to Nova - it would be a good idea to start by landing on Moon and having a look around. Hit 'L' to request landing clearance, then hit it again to land.";

    #[test]
    fn the_land_hint_names_the_stellar_and_l() {
        let mut view = flight();
        view.begin();
        view.take_sounds();
        ticks(&mut view, 1);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TutorialHint)]);
        assert_eq!(message(&view), Some(message_drawn(MOON_HINT)));
        assert_lasts(&mut view, MOON_HINT, 512);
    }

    #[test]
    fn the_land_hint_words_a_station_as_docking() {
        assert_eq!(
            land_hint(true, "Earth Station", "L"),
            "Welcome to Nova - it would be a good idea to start by docking at Earth Station and having a look around. Hit 'L' to request landing clearance, then hit it again to land."
        );
        assert_eq!(land_hint(false, "Moon", "L"), MOON_HINT);
    }

    #[test]
    fn the_hyperspace_hint_names_m_and_j() {
        let hint = "Now you should probably hyperspace to another system. Hit 'M' to access the map, select a nearby system, then move outward and hit 'J' to begin your jump.";
        assert_eq!(hyperspace_hint("M", "J"), hint);
        // At (50, 50), where a new pilot starts.
        let mut view = new_pilot(flight_among(vec![site(
            128,
            (50.0, 50.0),
            StellarFlags::CAN_LAND,
        )]));
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        view.take_off().expect("takes off");
        view.take_sounds();
        // The take-off's frame is -14: the hint comes at 0.
        ticks(&mut view, 14);
        assert_eq!(view.message(), None);
        ticks(&mut view, 1);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TutorialHint)]);
        assert_lasts(&mut view, hint, 512);
    }

    #[test]
    fn far_enough_and_nothing_out_here_last_512() {
        let mut view = new_pilot(flight());
        fly_far(&mut view);
        // The ship drifts on outward: #30 shows on the first sixtieth
        // frame past 1,414 px, until #31 replaces it past 3,162 px.
        let distance = |view: &View| player(view).position.length();
        let mut far = None;
        for tick in 0..2000 {
            ticks(&mut view, 1);
            match (view.message(), far) {
                (Some(FAR_ENOUGH), None) => far = Some((tick, distance(&view))),
                (Some(FAR_ENOUGH), Some(_)) => {}
                (Some(NOTHING_OUT_HERE), Some((since, at))) => {
                    assert!(at > 1414.3, "{at}");
                    assert!(distance(&view) > 3162.3, "{}", distance(&view));
                    assert!(tick - since < 512, "#30 shown throughout");
                    assert_lasts(&mut view, NOTHING_OUT_HERE, 512);
                    return;
                }
                (shown, _) => panic!("{shown:?} at tick {tick}"),
            }
        }
        panic!("#31 never came: {:?}", player(&view));
    }

    #[test]
    fn starting_a_jump_clears_the_far_enough_hint() {
        for text in [FAR_ENOUGH, NOTHING_OUT_HERE, TOO_CLOSE] {
            let mut view = flight();
            plot(&mut view, 131);
            fly_out(&mut view);
            view.say_for(text, 512);
            view.input(&key(JUMP, true));
            assert!(view.preparing_jump() || view.jump_effect().is_some());
            assert_eq!(view.message(), None, "{text}");
        }
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.say_for(NO_FUEL, 240);
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), Some(NO_FUEL), "any other stays");
    }

    #[test]
    fn the_hints_name_letter_keys_as_str_129_does() {
        assert_eq!(key_label(LAND_KEY).as_deref(), Some("L"));
        assert_eq!(key_label(MAP_KEY).as_deref(), Some("M"));
        assert_eq!(key_label(JUMP_KEY).as_deref(), Some("J"));
        assert_eq!(key_label(Key::Function(5)), None, "not yet named");
    }

    #[test]
    fn a_repeat_or_release_of_j_does_nothing() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&held(JUMP));
        view.input(&key(JUMP, false));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.message(), None);
    }

    #[test]
    fn j_while_landed_is_refused_until_take_off() {
        let mut view = flight_among(vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)]);
        plot(&mut view, 131);
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(140)));
        view.input(&key(JUMP, true));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.session().expect("flying").jumping(), None);
        assert_eq!(view.message(), Some(TAKE_OFF_FIRST));
        view.take_off();
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), Some(TOO_CLOSE), "flying again");
    }

    /// The fade quad's place in the list and its colour, if drawn.
    fn fade(list: &DrawList) -> Option<(usize, Color)> {
        list.iter().enumerate().find_map(|(at, c)| match *c {
            DrawCommand::Line {
                from, width, color, ..
            } if from == Point::new(0.0, 384.0) && width == 768.0 => Some((at, color)),
            _ => None,
        })
    }

    /// Where the HUD starts: its status bar picture.
    fn hud_at(list: &DrawList) -> usize {
        list.iter()
            .position(|c| matches!(c, DrawCommand::Picture { image, .. } if *image == ImageKey::picture(700)))
            .expect("the HUD")
    }

    /// Where the ship's sprite is drawn.
    fn ship_at(list: &DrawList) -> usize {
        list.iter()
            .position(|c| matches!(c, DrawCommand::Sprite { image, .. } if image.id == 2000))
            .expect("the ship")
    }

    #[test]
    fn a_jump_streaks_the_stars_then_fades_out_arrives_and_fades_in() {
        let mut view = flight().with_overlays(TITLED);
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let leaving = player(&view);
        let fuel_leaving = reserves(&view).fuel.now;
        assert_eq!(view.message(), None);
        let effect = view.jump_effect().expect("jumping");
        assert_eq!(effect.direction(), at(1.0, 0.0), "east, to Alpha Centauri");
        assert_eq!(
            view.session().expect("flying").jumping(),
            Some(SystemId(131))
        );

        // The stars streak; nothing fades.
        view.tick(ms(500));
        assert_eq!(player(&view), leaving, "frozen");
        let list = drawn(&view);
        let mut streaks = DrawList::new();
        starfield::draw_streaked(&mut streaks, &view.camera(), at(1.0, 0.0), 256.0);
        assert!(!streaks.is_empty());
        assert!(list.iter().take(streaks.len()).eq(streaks.iter()));
        assert_eq!(fade(&list), None);
        assert!(texts(&list).contains(&"Sol (sÿst 130)".to_owned()));

        // The old system fades out to white, HUD and all.
        view.tick(ms(625));
        let list = drawn(&view);
        let (at_fade, color) = fade(&list).expect("a fade");
        assert_eq!(color, Color::rgba(255, 255, 255, 64));
        assert!(ship_at(&list) < hud_at(&list) && hud_at(&list) < at_fade);
        assert_eq!(at_fade, list.len() - 1, "the fade is drawn last");
        assert!(texts(&list).contains(&"Sol (sÿst 130)".to_owned()));
        view.tick(ms(125));
        assert_eq!(fade(&drawn(&view)).map(|f| f.1.a), Some(128), "growing");
        assert_eq!(view.session().expect("flying").system(), SystemId(130));

        // Past 1.5 s it arrives, and the new system fades in.
        view.tick(ms(450));
        let session = view.session().expect("flying");
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.jumping(), None);
        assert_eq!(
            *view.catalog().systems_read.borrow(),
            [SystemId(130), SystemId(131)]
        );
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(view.course_map().current(), Some(SystemId(131)));
        assert_eq!(view.course_map().route(), []);
        let arrived = player(&view);
        assert_eq!(arrived.position, Vec2::new(-1001.0, 0.0));
        assert_eq!(arrived.velocity, Vec2::ZERO, "at rest");
        assert_eq!(reserves(&view).fuel.now, fuel_leaving - 100.0);
        assert_eq!(view.shown_position(), at(-1001.0, 0.0));
        let list = drawn(&view);
        let (at_fade, color) = fade(&list).expect("a fade");
        assert_eq!(color.a, 221);
        assert!(texts(&list).contains(&"Alpha Centauri (sÿst 131)".to_owned()));
        let mut hud = DrawList::new();
        hud::draw(
            &mut hud,
            view.status_bar().expect("a status bar"),
            &HudState {
                radar: &view.radar_shown(),
                blink_on: view.hud_cadence.blink_on(),
                reserves: reserves(&view),
                nav: NavDisplay::Off,
                cargo: Some(&view.session().expect("flying").status_cargo()),
                disabled: false,
            },
            None,
        );
        assert!(
            list.iter()
                .skip(hud_at(&list))
                .take(hud.len())
                .eq(hud.iter()),
            "the HUD, with a jump's fuel less and a day on"
        );
        assert_eq!(
            (at_fade, hud_at(&list) + hud.len() + 2),
            (list.len() - 1, at_fade),
            "then the target and weapon, and the fade over everything"
        );
        view.tick(ms(100));
        assert_eq!(fade(&drawn(&view)).map(|f| f.1.a), Some(204), "shrinking");
        assert_eq!(player(&view), arrived, "at rest, no keys held");

        // Then plain flight.
        view.tick(ms(1200));
        assert_eq!(view.jump_effect(), None);
        let list = drawn(&view);
        assert_eq!(fade(&list), None);
        let mut stars = DrawList::new();
        starfield::draw(&mut stars, &view.camera());
        assert!(list.iter().take(stars.len()).eq(stars.iter()));
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(player(&view), stepped(arrived, THRUST, 3));
        assert_ne!(player(&view), arrived, "it flies again");
    }

    #[test]
    fn hyperspace_effects_default_on() {
        let mut view = flight();
        assert!(view.hyperspace_effects());
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        view.tick(ms(1250));
        assert_eq!(fade(&drawn(&view)).map(|f| f.1.a), Some(128), "fading out");
    }

    /// Whether `list` draws the fade quad translucent: part of a fade.
    fn translucent_fade(list: &DrawList) -> bool {
        fade(list).is_some_and(|(_, color)| color.a < u8::MAX)
    }

    #[test]
    fn with_effects_off_a_jump_streaks_then_arrives_without_fading() {
        let mut view = flight().with_hyperspace_effects(false);
        assert!(!view.hyperspace_effects());
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let leaving = player(&view);

        // The stars streak, with no fade over them.
        view.tick(ms(500));
        assert_eq!(player(&view), leaving, "frozen");
        let list = drawn(&view);
        let mut streaks = DrawList::new();
        starfield::draw_streaked(&mut streaks, &view.camera(), at(1.0, 0.0), 256.0);
        assert!(list.iter().take(streaks.len()).eq(streaks.iter()));
        assert_eq!(fade(&list), None);
        view.tick(ms(499));
        assert_eq!(fade(&drawn(&view)), None);
        assert_eq!(view.session().expect("flying").system(), SystemId(130));

        // As the streak ends, the ship arrives and the screen is white once.
        view.tick(ms(1));
        let session = view.session().expect("flying");
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.jumping(), None);
        let list = drawn(&view);
        assert_eq!(
            fade(&list),
            Some((list.len() - 1, Color::WHITE)),
            "opaque white over everything"
        );
        assert!(view.jump_effect().is_some(), "flashing");
        assert_eq!(player(&view).position, Vec2::new(-1001.0, 0.0));

        // Then plain flight, with no fade in.
        view.tick(ms(33));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(fade(&drawn(&view)), None);
    }

    #[test]
    fn set_hyperspace_effects_changes_the_next_jump() {
        let mut view = flight().with_hyperspace_effects(false);
        view.set_hyperspace_effects(true);
        assert!(view.hyperspace_effects());
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        view.tick(ms(1250));
        assert!(translucent_fade(&drawn(&view)), "fading");

        let mut view = flight();
        view.set_hyperspace_effects(false);
        assert!(!view.hyperspace_effects());
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let mut faded = false;
        for _ in 0..200 {
            view.tick(TICK);
            faded |= translucent_fade(&drawn(&view));
        }
        assert!(!faded, "never fades");
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
    }

    /// Ship 128 carrying the stock Multi-Jumping Organ (`oütf` 275,
    /// multi-jump 10), with Alpha Centauri linked on to Barnard.
    fn multi_jumping() -> FakeCatalog {
        FakeCatalog {
            outfits: vec![OutfitRecord {
                id: OutfitId(275),
                name: "Multi-Jumping Organ".to_owned(),
                short_name: "Multi-Jumping Organ".to_owned(),
                disp_weight: 0,
                mass: 0,
                tech_level: 1,
                max: 1,
                flags: 0,
                cost: 0,
                mods: [(32, 10), (0, 0), (0, 0), (0, 0)],
                contribute: 0,
                require: 0,
                require_govt: -1,
                buy_random: 100,
                availability: nova_sim::Test::default(),
                item_class: 0,
                lc_name: "multi-jumping organ".to_owned(),
                lc_plural: "multi-jumping organs".to_owned(),
                scan_mask: 0,
                on_purchase: nova_sim::Script::default(),
                on_sell: nova_sim::Script::default(),
            }],
            defaults: vec![(OutfitId(275), 1)],
            onward: true,
            ..catalog()
        }
    }

    #[test]
    fn the_sabotage_death_descs_come_from_the_pilots_outfits() {
        let mut sabotaged = multi_jumping();
        sabotaged.outfits[0].mods[0] = (nova_sim::SABOTAGE_KILL, 5000);
        assert_eq!(FlightView::new(sabotaged).sabotage_death_descs(), [5000]);
        assert_eq!(flight().sabotage_death_descs(), Vec::<i16>::new());
    }

    #[test]
    fn a_multi_jump_plays_one_effect_and_loads_only_the_final_system() {
        let mut view = FlightView::new(multi_jumping()).with_overlays(TITLED);
        plot(&mut view, 132);
        assert_eq!(
            view.session().expect("flying").course(),
            [SystemId(131), SystemId(132)]
        );
        fly_out(&mut view);
        jump_now(&mut view);
        let fuel_leaving = reserves(&view).fuel.now;
        let effect = view.jump_effect().expect("jumping");
        assert_eq!(effect.direction(), at(1.0, 0.0), "east, to the first hop");

        view.tick(ms(1550));
        let session = view.session().expect("flying");
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.course(), []);
        assert_eq!(
            *view.catalog().systems_read.borrow(),
            [SystemId(130), SystemId(132)],
            "Alpha Centauri is never loaded"
        );
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(132)));
        assert_eq!(reserves(&view).fuel.now, fuel_leaving - 100.0);
        assert!(texts(&drawn(&view)).contains(&"Barnard (sÿst 132)".to_owned()));
        assert_eq!(
            view.message(),
            Some(
                "Jumping into the Barnard system on June 24, 1177 NC. No stellar objects present."
            ),
            "names the final system"
        );
        assert!(view.jump_effect().is_some(), "fading in");

        view.tick(ms(1450));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.session().expect("flying").jumping(), None);
        for _ in 0..30 {
            view.tick(TICK);
            assert_eq!(view.jump_effect(), None, "no second effect");
        }
        assert_eq!(view.session().expect("flying").system(), SystemId(132));
    }

    /// Taps J and ticks until the streak begins, as the session says the
    /// jump has begun.
    fn jump_now(view: &mut View) {
        tap(view, JUMP);
        for _ in 0..1000 {
            if view.jump_effect().is_some() {
                return;
            }
            view.tick(TICK);
        }
        panic!("the jump never began: {:?}", player(view));
    }

    /// Ticks until the ship has arrived in another system and the jump's
    /// effect is over.
    fn arrive_now(view: &mut View) {
        let leaving = view.session().expect("flying").system();
        for _ in 0..1000 {
            let arrived = view.session().expect("flying").system() != leaving;
            if arrived && view.jump_effect().is_none() {
                return;
            }
            view.tick(TICK);
        }
        panic!("never arrived: {:?}", player(view));
    }

    #[test]
    fn the_streak_waits_for_the_session_to_begin_the_jump() {
        // fly_out leaves the ship racing north, facing it; Alpha Centauri
        // is east.
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        assert_eq!(view.jump_effect(), None, "not on J");
        assert_eq!(view.message(), None);
        let session = view.session().expect("flying");
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.jumping(), None);
        let pressed = player(&view);
        view.tick(TICK);
        assert_ne!(player(&view), pressed, "the session keeps ticking");
        let mut waited = 1;
        while view.session().expect("flying").jumping().is_none() {
            assert_eq!(view.jump_effect(), None, "tick {waited}");
            view.tick(TICK);
            waited += 1;
            assert!(waited < 1000, "the jump never began");
        }
        let effect = view.jump_effect().expect("the streak, on that tick");
        assert_eq!(effect.direction(), at(1.0, 0.0), "east, to Alpha Centauri");
        assert_eq!(player(&view).heading, 90.0, "facing it");
        assert!(waited > 30, "braked and turned first: {waited}");
    }

    #[test]
    fn a_ship_ready_to_jump_streaks_on_j() {
        // Turned east at rest, then drifting out slowly that way: facing
        // Alpha Centauri and slow enough, it needs no pre-jump stage.
        let mut view = flight();
        plot(&mut view, 131);
        view.input(&key(Key::Right, true));
        ticks(&mut view, 30);
        view.input(&key(Key::Right, false));
        view.input(&key(Key::Up, true));
        ticks(&mut view, 10);
        view.input(&key(Key::Up, false));
        for _ in 0..2000 {
            if player(&view).position.length() >= MIN_JUMP_DISTANCE {
                break;
            }
            view.tick(TICK);
        }
        let ship = player(&view);
        assert_eq!(ship.heading, 90.0, "{ship:?}");
        assert!(ship.position.length() >= MIN_JUMP_DISTANCE, "{ship:?}");
        view.input(&key(JUMP, true));
        let effect = view.jump_effect().expect("streaking on J");
        assert_eq!(effect.direction(), at(1.0, 0.0));
        view.tick(TICK);
        assert_eq!(player(&view), ship, "frozen at once");
    }

    #[test]
    fn keys_are_ignored_while_the_ship_turns_to_jump() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        let pressed = player(&view);
        let mut alongside = flight();
        plot(&mut alongside, 131);
        fly_out(&mut alongside);
        alongside.input(&key(JUMP, true));
        for k in [Key::Up, Key::Left, Key::Down, MAP, LAND, JUMP] {
            assert_eq!(view.input(&key(k, true)), ScreenAction::None);
        }
        assert!(!view.map_open());
        assert_eq!(view.message(), None);
        for _ in 0..10 {
            view.tick(TICK);
            alongside.tick(TICK);
        }
        assert_eq!(player(&view), player(&alongside), "flown alike");
        assert_ne!(player(&view), pressed, "the session flies on");
        assert_eq!(view.jump_effect(), None);
        let session = view.session().expect("flying");
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.nav_target(), None);
        assert_eq!(view.take_landing(), None);
    }

    #[test]
    fn keys_are_ignored_while_the_jump_plays() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(Key::Up, true));
        jump_now(&mut view);
        view.tick(ms(300));
        view.input(&key(Key::Up, false));
        view.input(&key(Key::Left, true));
        view.input(&key(MAP, true));
        view.input(&key(LAND, true));
        view.input(&key(JUMP, true));
        assert!(!view.map_open());
        assert_eq!(view.message(), None);
        view.input(&key(Key::Left, false));
        view.tick(ms(2700));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.take_landing(), None);
        let arrived = player(&view);
        ticks(&mut view, 5);
        assert_eq!(
            player(&view),
            stepped(arrived, Controls::default(), 5),
            "neither thrusting nor turning"
        );
    }

    #[test]
    fn keys_are_ignored_while_the_old_system_fades_out() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        view.tick(ms(1250));
        assert!(matches!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::FadeOut(_))
        ));
        for k in [Key::Up, Key::Left, MAP, LAND, JUMP] {
            assert_eq!(view.input(&key(k, true)), ScreenAction::None);
            assert_eq!(view.input(&key(k, false)), ScreenAction::None);
        }
        assert!(!view.map_open());
        assert_eq!(view.message(), None);
        assert_eq!(view.take_landing(), None);
        arrive_now(&mut view);
        let arrived = player(&view);
        ticks(&mut view, 5);
        assert_eq!(
            player(&view),
            stepped(arrived, Controls::default(), 5),
            "neither thrusting nor turning"
        );
        assert_eq!(view.session().expect("flying").preparing_jump(), None);
    }

    /// Two of `build`'s flights, each with its course to Alpha Centauri
    /// plotted and flown out: the first braking and turning for the jump
    /// on a tap of J, the second 300 ms into the jump's streak. Boxed: two flights
    /// are too large for the stack together.
    fn through_a_jump(build: impl Fn() -> View) -> [Box<View>; 2] {
        let mut braking = Box::new(build());
        plot(&mut braking, 131);
        fly_out(&mut braking);
        tap(&mut braking, JUMP);
        assert!(braking.preparing_jump(), "braking");
        let mut streaking = Box::new(build());
        plot(&mut streaking, 131);
        fly_out(&mut streaking);
        jump_now(&mut streaking);
        streaking.tick(ms(300));
        assert!(streaking.jump_effect().is_some(), "streaking");
        [braking, streaking]
    }

    /// Which stage of the jump `view` is in, for a failing assertion.
    fn stage(view: &View) -> &'static str {
        if view.preparing_jump() {
            "braking"
        } else {
            "streaking"
        }
    }

    #[test]
    fn the_target_keys_are_taken_through_a_jump() {
        for mut view in through_a_jump(|| fighting(armed(2, &[]), &two_npcs())) {
            let stage = stage(&view);
            assert_eq!(target_of(&view), None, "{stage}");
            tap(&mut view, TARGET_KEY);
            assert_eq!(target_of(&view), Some(NpcId(0)), "{stage}");
            tap(&mut view, TARGET_KEY);
            assert_eq!(target_of(&view), Some(NpcId(1)), "{stage}: steps on");
            view.input(&key(Key::Shift, true));
            tap(&mut view, TARGET_KEY);
            view.input(&key(Key::Shift, false));
            assert_eq!(target_of(&view), Some(NpcId(0)), "{stage}: Shift-Tab");
            tap(&mut view, TARGET_KEY);
            tap(&mut view, TARGET_KEY);
            assert_eq!(target_of(&view), None, "{stage}: past the last");
            let session = view.session().expect("flying");
            let from = session.player().position;
            let nearest = session
                .npcs()
                .iter()
                .min_by(|a, b| {
                    let a = (a.state.position - from).length();
                    let b = (b.state.position - from).length();
                    a.total_cmp(&b)
                })
                .map(|npc| npc.id);
            alt_tap(&mut view, NEAREST_KEY);
            assert_eq!(target_of(&view), nearest, "{stage}: Option-R");
            assert!(nearest.is_some());
        }
    }

    #[test]
    fn option_backquote_clears_the_target_through_a_jump_and_nav_off_is_ignored() {
        let targeted = || {
            let mut view = fighting(armed(2, &[]), &two_npcs());
            tap(&mut view, TARGET_KEY);
            view
        };
        for mut view in through_a_jump(targeted) {
            let stage = stage(&view);
            let mode = nav_mode(&view);
            let _ = view.take_sounds();
            tap(&mut view, NAV_OFF_KEY);
            assert_eq!(nav_mode(&view), mode, "{stage}: Nav Off ignored");
            assert_eq!(target_of(&view), Some(NpcId(0)), "{stage}");
            alt_tap(&mut view, NAV_OFF_KEY);
            assert_eq!(target_of(&view), None, "{stage}: Option-`");
            assert_eq!(nav_mode(&view), mode, "{stage}");
            assert!(
                view.take_sounds()
                    .contains(&Sound::Sim(SimSound::TargetCleared)),
                "{stage}"
            );
        }
    }

    #[test]
    fn weapon_select_is_taken_through_a_jump() {
        for mut view in through_a_jump(|| fighting(armed(0, &[BLASTER, ROCKET, MISSILE]), &[])) {
            let stage = stage(&view);
            assert_eq!(secondary_of(&view), None, "{stage}");
            tap(&mut view, SELECT_KEY);
            assert_eq!(secondary_of(&view), Some(ROCKET), "{stage}");
            tap(&mut view, SELECT_KEY);
            assert_eq!(secondary_of(&view), Some(MISSILE), "{stage}");
            alt_tap(&mut view, SELECT_KEY);
            assert_eq!(secondary_of(&view), Some(ROCKET), "{stage}: Option-W");
        }
    }

    #[test]
    fn board_is_taken_through_a_jump() {
        for mut view in through_a_jump(|| beside(750, 750, 0, true)) {
            let stage = stage(&view);
            tap(&mut view, BOARD_KEY);
            assert_eq!(view.take_boarding(), None, "{stage}");
            assert_eq!(view.message(), Some(CANT_BOARD), "{stage}");
        }
    }

    #[test]
    fn hail_is_refused_silently_through_a_jump() {
        for mut view in through_a_jump(|| hailable(true)) {
            let stage = stage(&view);
            tap(&mut view, HAIL_KEY);
            assert_eq!(view.take_hail(), None, "{stage}");
            assert_eq!(view.take_offer(), None, "{stage}");
            assert_eq!(view.message(), None, "{stage}");
        }
    }

    /// [`trading`]'s flight, its hold bought full of food and taken off.
    fn laden() -> View {
        let mut view = FlightView::new(trading());
        land_now(&mut view);
        view.settle_script();
        assert!(view.trade(BUY_FOOD).is_ok());
        view.take_off().expect("takes off");
        assert!(cargo_aboard(&view) > 0);
        // Past the take-off grace, so the map opens for the course.
        ticks(&mut view, nova_sim::TAKEOFF_GRACE_TICKS);
        view.message = None;
        view
    }

    #[test]
    fn command_k_jettisons_through_a_jump() {
        for mut view in through_a_jump(laden) {
            let stage = stage(&view);
            command_tap(&mut view, JETTISON_KEY, false);
            assert_eq!(cargo_aboard(&view), 0, "{stage}");
            assert_eq!(view.message(), Some(CARGO_JETTISONED), "{stage}");
        }
    }

    /// Whether [`FlightView::taken_during_jump`] takes `key` with Option
    /// held or not and the escort menu open or shut, each of the four.
    fn taken_with(key: Key) -> [bool; 4] {
        [(false, false), (true, false), (false, true), (true, true)]
            .map(|(option, menu_open)| View::taken_during_jump(key, option, menu_open))
    }

    #[test]
    fn the_keys_a_jump_takes_are_the_originals_ungated_ones() {
        let always = [
            TARGET_KEY,
            NEAREST_KEY,
            SELECT_KEY,
            BOARD_KEY,
            HAIL_KEY,
            JETTISON_KEY,
            MENU_KEY,
            MENU_CLOSE_KEY,
            ESCORT_ATTACK_KEY,
            ESCORT_DEFEND_KEY,
            ESCORT_HOLD_KEY,
            ESCORT_RECALL_KEY,
            ZOOM_OUT_KEY,
            ZOOM_IN_KEY,
            ZOOM_IN_KEYPAD_KEY,
        ];
        for key in always {
            assert_eq!(taken_with(key), [true; 4], "{key:?}");
        }
        assert_eq!(
            taken_with(NAV_OFF_KEY),
            [false, true, false, true],
            "with Option"
        );
        for key in GROUP_KEYS {
            assert_eq!(
                taken_with(key),
                [false, false, true, true],
                "{key:?}: menu open"
            );
        }
        let never = [
            LAND_KEY,
            MAP_KEY,
            JUMP_KEY,
            MISSION_INFO_KEY,
            PLAYER_INFO_KEY,
            HYPERSPACE_MODE_KEY,
            HYPER_SELECT_KEY,
            Key::Function(1),
            Key::Function(4),
            NEAREST_STELLAR_KEY,
            Key::Up,
            FIRE_KEY,
        ];
        for key in never {
            assert_eq!(taken_with(key), [false; 4], "{key:?}");
        }
    }

    #[test]
    fn nav_and_screen_keys_are_ignored_through_a_jump() {
        for mut view in through_a_jump(flight) {
            let stage = stage(&view);
            let braking = view.preparing_jump();
            let mode = nav_mode(&view);
            let course = view.session().expect("flying").course().to_vec();
            for k in [
                LAND,
                MAP,
                JUMP,
                MISSION_INFO_KEY,
                HYPERSPACE_MODE_KEY,
                HYPER_SELECT_KEY,
                NAV_OFF_KEY,
                STELLAR_SLOT_KEYS[0][0],
                STELLAR_SLOT_KEYS[0][1],
                NEAREST_STELLAR_KEY,
            ] {
                tap(&mut view, k);
            }
            let earth = on_screen(&view, (0.0, -600.0));
            click(&mut view, earth);
            assert!(!view.map_open(), "{stage}");
            assert_eq!(view.take_landing(), None, "{stage}");
            assert!(view.take_mission_info_request().is_none(), "{stage}");
            assert_eq!(nav_mode(&view), mode, "{stage}");
            assert_eq!(nav_target(&view), None, "{stage}");
            let session = view.session().expect("flying");
            assert_eq!(session.course(), course, "{stage}");
            assert_eq!(view.message(), None, "{stage}");
            assert_eq!(view.preparing_jump(), braking, "{stage}");
            assert_eq!(view.jump_effect().is_some(), !braking, "{stage}");
        }
    }

    /// Ticks until the ship has arrived in another system, the jump's
    /// fade-in yet to play.
    fn tick_to_arrival(view: &mut View) {
        let leaving = view.session().expect("flying").system();
        for _ in 0..1000 {
            if view.session().expect("flying").system() != leaving {
                return;
            }
            view.tick(TICK);
        }
        panic!("never arrived: {:?}", player(view));
    }

    #[test]
    fn a_flight_key_held_through_arrival_acts_at_once() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        view.tick(ms(300));
        view.input(&key(Key::Up, true));
        tick_to_arrival(&mut view);
        let arrived = player(&view);
        view.tick(TICK);
        assert_eq!(player(&view), stepped(arrived, THRUST, 1), "thrusting");
    }

    #[test]
    fn a_key_held_when_j_is_pressed_stays_held() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(Key::Up, true));
        jump_now(&mut view);
        tick_to_arrival(&mut view);
        let arrived = player(&view);
        view.tick(TICK);
        assert_eq!(player(&view), stepped(arrived, THRUST, 1), "thrusting");
    }

    #[test]
    fn fire_held_through_a_jump_fires_only_after_arrival() {
        let mut view = fighting(armed(0, &[BLASTER]), &[]);
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        assert!(view.preparing_jump(), "braking");
        view.input(&key(FIRE_KEY, true));
        while view.jump_effect().is_none() {
            view.tick(TICK);
            assert_eq!(shots_of(&view, BLASTER), 0, "braking");
        }
        tick_to_arrival(&mut view);
        assert_eq!(shots_of(&view, BLASTER), 0);
        view.tick(TICK);
        assert_eq!(shots_of(&view, BLASTER), 1, "fired on the first tick");
    }

    /// Jumps from Sol to Alpha Centauri, on a course on to Barnard, and
    /// stops 50 ms into the arrival's fade-in.
    fn fading_in_on_course_to_barnard() -> View {
        let mut view = FlightView::new(FakeCatalog {
            onward: true,
            ..catalog()
        });
        plot(&mut view, 132);
        fly_out(&mut view);
        jump_now(&mut view);
        view.tick(ms(1550));
        assert_eq!(system_of(&view), SystemId(131));
        assert!(matches!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::FadeIn(_))
        ));
        view
    }

    /// The fade's alpha as drawn, if it is.
    fn fade_alpha(view: &View) -> Option<u8> {
        fade(&drawn(view)).map(|(_, color)| color.a)
    }

    #[test]
    fn j_during_the_arrival_fade_in_begins_the_next_jump() {
        let mut view = fading_in_on_course_to_barnard();
        let arrival = view.message().expect("the arrival message").to_owned();
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), Some(arrival.as_str()), "not refused");
        let session = view.session().expect("flying");
        assert!(
            session.preparing_jump() == Some(SystemId(132))
                || session.jumping() == Some(SystemId(132)),
            "{:?} {:?}",
            session.preparing_jump(),
            session.jumping()
        );
        let pressed = fade_alpha(&view).expect("still fading in");
        view.tick(TICK);
        let fading = fade_alpha(&view).expect("the old fade plays on");
        assert!(fading < pressed, "{fading} < {pressed}");
        for _ in 0..1000 {
            if view.session().expect("flying").jumping().is_some() {
                break;
            }
            view.tick(TICK);
        }
        assert_eq!(
            view.session().expect("flying").jumping(),
            Some(SystemId(132))
        );
        let effect = view.jump_effect().expect("the next jump's streak");
        assert!(matches!(effect.phase(), JumpPhase::Streak(_)));
        let toward = JumpEffect::toward(Vec2::new(600.0, 0.0), Vec2::new(0.0, 600.0));
        assert_eq!(
            effect.direction(),
            toward.direction(),
            "south-west, to Barnard"
        );
        arrive_now(&mut view);
        assert_eq!(system_of(&view), SystemId(132));
    }

    #[test]
    fn the_ship_flies_under_the_arrival_fade_in() {
        let mut view = fading_in_on_course_to_barnard();
        let arrived = player(&view);
        let before = fade_alpha(&view).expect("fading in");
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(player(&view), stepped(arrived, THRUST, 3));
        let after = fade_alpha(&view).expect("still fading in");
        assert!(after < before, "{after} < {before}");
    }

    #[test]
    fn the_arrival_fade_plays_on_and_shows_over_the_map() {
        let mut view = fading_in_on_course_to_barnard();
        tap(&mut view, MAP);
        assert!(view.map_open(), "the map opens under the fade");
        let list = drawn(&view);
        let (at_fade, color) = fade(&list).expect("the fade, over the map");
        assert_eq!(at_fade, list.len() - 1, "drawn last");
        let before = color.a;
        view.tick(ms(100));
        let after = fade_alpha(&view).expect("fading on with the map open");
        assert!(after < before, "{after} < {before}");
        view.tick(crate::flight::jump::FADE_IN_FOR);
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert_eq!(view.jump_effect(), None, "the fade played out");
        assert_eq!(fade(&drawn(&view)), None);
    }

    #[test]
    fn arriving_shows_the_jump_message_on_the_message_line() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let session = view.session().expect("flying");
        let leaving = session.system();
        for _ in 0..1000 {
            if view.session().expect("flying").system() != leaving {
                break;
            }
            view.tick(TICK);
        }
        let date = view.session().expect("flying").date_text();
        let name = view.scene().expect("arrived").name().to_owned();
        let text = arrival_message(&name, &date, true);
        assert!(text.contains("Alpha Centauri"), "{text}");
        assert_eq!(view.message(), Some(text.as_str()));
        assert_eq!(message(&view), Some(message_drawn(&text)));
        assert_lasts(&mut view, &text, 240);
        assert_eq!(message(&view), None);
    }

    #[test]
    fn a_refusal_shown_before_the_jump_is_replaced_by_the_arrival() {
        let mut view = new_pilot(flight());
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        view.input(&key(JUMP, false));
        assert_eq!(view.message(), Some(NO_DESTINATION));
        plot(&mut view, 131);
        assert_eq!(view.message(), Some(NO_DESTINATION), "still on screen");
        view.input(&key(JUMP, true));
        assert_eq!(
            view.message(),
            Some(NO_DESTINATION),
            "starting the jump clears only #42"
        );
        jump_now(&mut view);
        arrive_now(&mut view);
        let shown = view.message().expect("the arrival message");
        assert!(shown.starts_with(JUMPING_INTO), "{shown}");
    }

    #[test]
    fn map_and_jump_in_a_flight_that_never_started_do_nothing_much() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert_eq!(view.course_map().current(), None);
        view.input(&key(JUMP, true));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.message(), None);
        tap(&mut view, MAP);
        assert!(view.map_open());
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        assert_eq!(view.course_map().route(), []);
    }

    #[test]
    fn the_stellars_go_on_animating_while_the_jump_plays() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let moon = |view: &View| sprites(&drawn(view))[1].0.frame;
        let before = moon(&view);
        view.tick(TICK);
        assert_eq!(moon(&view), (before + 1) % 4, "a frame a tick");
        view.tick(TICK * 2);
        assert_eq!(moon(&view), (before + 3) % 4);
        assert!(view.jump_effect().is_some());
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_on_the_open_map() {
        let mut view = flight();
        tap(&mut view, MAP);
        let alpha = on_map(&view, 131);
        let left = |pressed| Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed,
            at: alpha,
        };
        view.input(&left(true));
        view.cancel_pointer();
        view.input(&left(false));
        assert_eq!(
            view.course_map().selected(),
            Some(SystemId(130)),
            "no click: the preselect"
        );
        assert_eq!(view.session().expect("flying").course(), []);
        assert!(view.map_open());
    }

    // The pilot.

    /// A pilot named `name` docked at Earth: landed over a planet 128 at
    /// the centre, then flown in the stock catalog, where Earth is at
    /// (0, -600).
    fn docked_pilot(name: &str) -> Pilot {
        let centred = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..catalog()
        };
        let pilot = Pilot::new(&centred, name).expect("starts");
        let mut view = FlightView::with_pilot(centred, pilot);
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        view.pilot().expect("flying").clone()
    }

    // Pilot edits.

    /// A view of Ada resumed over Earth and landed there, the landing,
    /// its sounds and its message taken, with the systems read so far
    /// forgotten.
    fn landed_view() -> View {
        let mut view = FlightView::with_pilot(catalog(), docked_pilot("Ada"));
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        view.take_sounds();
        view.message = None;
        view.catalog().systems_read.borrow_mut().clear();
        view
    }

    fn move_to(view: &mut View, system: i16, stellar: i16) {
        let mut desk = view.pilot_desk().expect("flying");
        desk.edit(PilotEdit::MoveTo {
            system: SystemId(system),
            stellar: StellarId(stellar),
        })
        .expect("moves");
    }

    #[test]
    fn a_session_that_failed_has_no_pilot_desk() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert!(view.pilot_desk().is_none());
        view.resync();
        assert!(view.scene().is_none());
    }

    #[test]
    fn the_pilot_desk_reads_the_session_and_names_from_the_map() {
        let mut view = landed_view();
        let desk = view.pilot_desk().expect("flying");
        let sheet = desk.sheet().expect("a pilot");
        assert_eq!(sheet.name, "Ada");
        assert_eq!(sheet.system.name, "Sol");
        assert_eq!(desk.systems().len(), 3);
    }

    #[test]
    fn after_a_move_resync_lays_out_the_new_system() {
        let mut view = landed_view();
        move_to(&mut view, 131, 140);
        view.resync();
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(131)]);
        assert_eq!(view.course_map().current(), Some(SystemId(131)));
        assert_eq!(view.course_map().route(), []);
        assert!(
            view.course_map()
                .explored()
                .is_some_and(|explored| explored.contains(&SystemId(131)))
        );
        assert_eq!(view.shown_position(), Point::new(0.0, 0.0));
        assert_eq!(view.message(), None, "no arrival message");
        view.resync();
        assert_eq!(
            *view.catalog().systems_read.borrow(),
            [SystemId(131)],
            "nothing moved, nothing read"
        );
    }

    #[test]
    fn after_a_move_within_the_system_resync_drops_the_course_and_reads_nothing() {
        let mut view = landed_view();
        plot(&mut view, 131);
        assert_eq!(view.course_map().route(), [SystemId(131)]);
        view.tick(TICK / 2);
        assert!(view.alpha() > 0.0, "between steps");
        move_to(&mut view, 130, 129);
        view.resync();
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
        assert_eq!(view.course_map().route(), []);
        assert_eq!(*view.catalog().systems_read.borrow(), []);
        assert_eq!(view.alpha(), 0.0);
        assert_eq!(view.shown_position(), Point::new(300.0, -200.0));
    }

    #[test]
    fn a_resync_with_nothing_moved_reads_nothing() {
        let mut view = landed_view();
        view.resync();
        assert_eq!(*view.catalog().systems_read.borrow(), []);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    }

    #[test]
    fn a_pilot_saved_at_a_stellar_resumes_in_flight_over_it_silently() {
        let pilot = docked_pilot("Ada");
        assert_eq!(pilot.stellar(), Some(StellarId(128)));
        let mut view = FlightView::with_pilot(catalog(), pilot.clone());
        assert_eq!(view.pilot(), Some(&pilot));
        assert_eq!(view.take_landing(), None, "nothing lands");
        assert_eq!(view.session().map(Session::landed), Ok(None));
        assert_eq!(player(&view).position, Vec2::new(0.0, -600.0));
        assert_eq!(view.shown_position(), Point::new(0.0, -600.0));
        assert_eq!(view.take_sounds(), [], "no landing sound");
        assert!(!view.take_save_due());
    }

    #[test]
    fn a_new_pilots_ship_is_first_shown_at_50_50() {
        let mut view = flight();
        view.begin();
        assert_eq!(player(&view).position, Vec2::new(50.0, 50.0));
        assert_eq!(
            view.shown_position(),
            Point::new(50.0, 50.0),
            "before any tick, not on its way from the centre"
        );
        view.tick(TICK / 2);
        assert_eq!(view.shown_position(), Point::new(50.0, 50.0), "at rest");
    }

    #[test]
    fn a_pilot_in_flight_resumes_at_the_centre() {
        let pilot = Pilot::new(&catalog(), "Bob").expect("starts");
        let mut view = FlightView::with_pilot(catalog(), pilot);
        assert_eq!(view.take_landing(), None);
        assert_eq!(player(&view), start());
        assert_eq!(view.pilot().map(Pilot::name), Some("Bob"));
    }

    #[test]
    fn a_pilot_whose_system_is_gone_flies_in_the_lowest_that_exists() {
        // In flight: a pilot last landed on a stellar resumes in its
        // stellar's system.
        let mut pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        pilot.explore(SystemId(132));
        let text = nova_sim::save::encode(&pilot).replace("\"system\": 130", "\"system\": 132");
        let moved = nova_sim::save::decode(&text).expect("a pilot");
        let view = FlightView::with_pilot(catalog(), moved);
        let session = view.session().expect("flies");
        assert_eq!(session.system(), SystemId(130));
        assert!(session.load_repaired());
    }

    /// [`docked_pilot`] Ada with her stellar rewritten to 999, which no
    /// system lists.
    fn unlisted_pilot() -> Pilot {
        let text = nova_sim::save::encode(&docked_pilot("Ada"))
            .replace("\"stellar\": 128", "\"stellar\": 999");
        let pilot = nova_sim::save::decode(&text).expect("a pilot");
        assert_eq!(pilot.stellar(), Some(StellarId(999)));
        pilot
    }

    #[test]
    fn a_loaded_flight_reports_a_repaired_load() {
        let view = FlightView::loaded(catalog(), unlisted_pilot(), Rulebook::default());
        assert!(view.load_repaired());
        assert_eq!(view.pilot().and_then(Pilot::stellar), None);
        let docked = FlightView::loaded(catalog(), docked_pilot("Ada"), Rulebook::default());
        assert!(!docked.load_repaired());
        assert!(!FlightView::with_pilot(catalog(), docked_pilot("Ada")).load_repaired());
        let failed = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert!(!failed.load_repaired(), "a failed session repaired nothing");
    }

    #[test]
    fn a_loaded_flight_reads_the_rules_it_is_given() {
        let mut pilot = unlisted_pilot();
        pilot.explore(SystemId(131));
        let text = nova_sim::save::encode(&pilot).replace("\"system\": 130", "\"system\": 131");
        let moved = nova_sim::save::decode(&text).expect("a pilot");
        let kept = FlightView::loaded(catalog(), moved.clone(), Rulebook::default());
        assert_eq!(kept.session().map(Session::system), Ok(SystemId(131)));
        let engine =
            Rulebook::default().with_override(RuleKey::UnlistedStellar, RuleSource::Engine);
        let lowest = FlightView::loaded(catalog(), moved, engine);
        assert_eq!(lowest.session().map(Session::system), Ok(SystemId(130)));
        assert!(lowest.load_repaired());
    }

    #[test]
    fn landing_taking_off_and_transactions_make_a_save_due() {
        let centred = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..catalog()
        };
        let mut view = FlightView::new(centred);
        assert!(!view.take_save_due());
        assert!(!view.transact(|pilot| pilot.set_cash(5)), "in flight");
        land_now(&mut view);
        assert!(view.take_save_due(), "landed");
        assert!(view.transact(|pilot| pilot.set_cash(5)));
        assert_eq!(view.pilot().map(Pilot::cash), Some(5));
        assert!(view.take_save_due(), "a transaction");
        assert!(!view.take_save_due(), "taken");
        view.take_off();
        assert!(view.take_save_due(), "took off");
    }

    #[test]
    fn a_view_that_cannot_fly_has_no_pilot_and_saves_nothing() {
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        let mut view = FlightView::new(broken);
        assert_eq!(view.pilot(), None);
        assert!(!view.transact(|pilot| pilot.set_cash(5)));
        assert!(!view.take_save_due());
    }

    #[test]
    fn the_course_map_shows_the_explored_systems_and_arriving_explores() {
        let mut view = flight();
        let explored = |view: &View| {
            view.course_map()
                .explored()
                .map(|set| set.iter().copied().collect::<Vec<_>>())
        };
        assert_eq!(explored(&view), Some(vec![SystemId(130)]));
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        arrive_now(&mut view);
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
        assert_eq!(explored(&view), Some(vec![SystemId(130), SystemId(131)]));
    }

    #[test]
    fn the_course_map_shows_systems_explored_between_frames() {
        let centred = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..catalog()
        };
        let mut view = FlightView::new(centred);
        land_now(&mut view);
        assert!(view.transact(|pilot| {
            pilot.explore(SystemId(131));
        }));
        let shown = |view: &View| view.course_map().explored().cloned();
        assert_eq!(shown(&view), Some([SystemId(130)].into()), "not yet");
        view.tick(TICK);
        assert_eq!(
            shown(&view),
            Some([SystemId(130), SystemId(131)].into()),
            "the next frame"
        );
    }

    // The exchange and the day's chances.

    use nova_sim::{Chance, Direction, DisasterId, Good, Lot, Order, TradeRefusal};

    /// Earth at the centre, a trade center trading food at 75, the first
    /// `chär` holding 1000 credits, and its ship 10 tons.
    fn trading() -> FakeCatalog {
        let flags = StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | 2 << 28;
        FakeCatalog {
            character: Ok(CharacterStart {
                cash: 1000,
                ..catalog().character.expect("a chär")
            }),
            fields: ShipFields {
                holds: 10,
                ..FIELDS
            },
            sites: vec![site(128, (0.0, 0.0), flags)],
            commodities: CommodityStrings {
                names: vec!["Food".to_owned()],
                name_patches: Default::default(),
                base_prices: vec!["75".to_owned()],
                price_patches: Default::default(),
            },
            ..catalog()
        }
    }

    const BUY_FOOD: Order = Order {
        row: 0,
        good: Good::Commodity(0),
        direction: Direction::Buy,
        lot: Lot::Click,
    };

    #[test]
    fn the_exchange_is_the_sessions_and_a_trade_goes_through_it() {
        let mut view = FlightView::new(trading());
        assert_eq!(view.market(), None, "in flight");
        assert_eq!(view.trade(BUY_FOOD), Err(TradeRefusal::NoMarket));
        land_now(&mut view);
        view.take_save_due();
        let market = view.market().expect("landed at a trade center");
        assert_eq!(
            market.row(Good::Commodity(0)).map(|row| row.price),
            Some(75)
        );
        assert_eq!(view.trade(BUY_FOOD), Ok(10), "a click buys up to 10");
        assert_eq!(view.pilot().map(Pilot::cash), Some(1000 - 10 * 75));
        assert!(view.take_save_due(), "a trade");
        assert_eq!(
            view.market()
                .and_then(|m| m.row(Good::Commodity(0)).map(|row| row.held)),
            Some(10)
        );
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..trading()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(broken.market(), None);
        assert_eq!(broken.trade(BUY_FOOD), Err(TradeRefusal::NoMarket));
    }

    use nova_sim::order_loop::{OrderRun, Stopped};
    use nova_sim::{OutfitOrder, OutfitRefusal, OutfitRun, Reselection};

    /// Earth at the centre, an outfitter selling a fuel tank (+100 fuel,
    /// a ton, 1000 credits), the first `chär` holding 1000 credits.
    fn outfitting() -> FakeCatalog {
        FakeCatalog {
            character: Ok(CharacterStart {
                cash: 1000,
                ..catalog().character.expect("a chär")
            }),
            sites: vec![site(
                128,
                (0.0, 0.0),
                StellarFlags::CAN_LAND | StellarFlags::OUTFITTER,
            )],
            outfits: vec![OutfitRecord {
                id: OutfitId(200),
                name: "Fuel Tank".to_owned(),
                short_name: "Fuel Tank".to_owned(),
                disp_weight: 0,
                mass: 1,
                tech_level: 1,
                max: 5,
                flags: 0,
                cost: 1000,
                mods: [(12, 100), (0, 0), (0, 0), (0, 0)],
                contribute: 0,
                require: 0,
                require_govt: -1,
                buy_random: 100,
                availability: nova_sim::Test::default(),
                item_class: 0,
                lc_name: "fuel tank".to_owned(),
                lc_plural: "fuel tanks".to_owned(),
                scan_mask: 0,
                on_purchase: nova_sim::Script::default(),
                on_sell: nova_sim::Script::default(),
            }],
            ..catalog()
        }
    }

    const BUY_TANK: OutfitOrder = OutfitOrder {
        outfit: OutfitId(200),
        direction: Direction::Buy,
    };

    #[test]
    fn the_outfitter_is_the_sessions_and_an_order_goes_through_it() {
        let mut view = FlightView::new(outfitting());
        assert_eq!(view.outfitter(), None, "in flight");
        assert_eq!(view.outfit(BUY_TANK), Err(OutfitRefusal::NoOutfitter));
        land_now(&mut view);
        view.take_save_due();
        let outfitter = view.outfitter().expect("landed at an outfitter");
        assert_eq!(
            outfitter.row(OutfitId(200)).map(|row| row.price),
            Some(1000)
        );
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(view.pilot().map(Pilot::cash), Some(0));
        assert!(view.take_save_due(), "a purchase");
        assert_eq!(
            view.outfitter()
                .and_then(|o| o.row(OutfitId(200)).map(|row| row.owned)),
            Some(1)
        );
        assert_eq!(view.reserves().fuel.max, 350.0, "the tank");
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..outfitting()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(broken.outfitter(), None);
        assert_eq!(broken.outfit(BUY_TANK), Err(OutfitRefusal::NoOutfitter));
    }

    #[test]
    fn a_counted_order_goes_through_the_session() {
        let mut catalog = outfitting();
        catalog.character = Ok(CharacterStart {
            cash: 3000,
            ..catalog.character.expect("a chär")
        });
        let mut view = FlightView::new(catalog);
        assert_eq!(
            view.outfit_counted(BUY_TANK, 2),
            Err(OutfitRefusal::NoOutfitter),
            "in flight"
        );
        land_now(&mut view);
        view.take_save_due();
        assert_eq!(view.outfit_counted(BUY_TANK, 2), Ok(2));
        assert_eq!(view.pilot().map(Pilot::cash), Some(1000));
        assert!(view.take_save_due(), "a purchase");
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..outfitting()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(
            broken.outfit_counted(BUY_TANK, 2),
            Err(OutfitRefusal::NoOutfitter)
        );
    }

    #[test]
    fn an_outfit_order_goes_through_the_session_with_its_run() {
        let mut catalog = outfitting();
        catalog.character = Ok(CharacterStart {
            cash: 3000,
            ..catalog.character.expect("a chär")
        });
        let refused = |refusal| OutfitRun {
            run: OrderRun {
                done: 0,
                stopped: Some(Stopped { pass: 0, refusal }),
            },
            alert: None,
            after: Reselection::Kept,
            button_passed: false,
        };
        let mut view = FlightView::new(catalog);
        assert_eq!(
            view.outfit_order(BUY_TANK, 2),
            refused(OutfitRefusal::NoOutfitter),
            "in flight"
        );
        land_now(&mut view);
        view.take_save_due();
        assert_eq!(
            view.outfit_order(BUY_TANK, 4),
            OutfitRun {
                run: OrderRun {
                    done: 3,
                    stopped: Some(Stopped {
                        pass: 3,
                        refusal: OutfitRefusal::CannotAfford,
                    }),
                },
                alert: None,
                after: Reselection::Kept,
                button_passed: true,
            }
        );
        assert_eq!(view.pilot().map(Pilot::cash), Some(0));
        assert!(view.take_save_due(), "a purchase");
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..outfitting()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(
            broken.outfit_order(BUY_TANK, 2),
            refused(OutfitRefusal::NoOutfitter)
        );
    }

    /// [`outfitting`] with 10,000 credits, a -10-ton pod (201) and an
    /// outfit (202) of `heavy` tons, each 100 credits.
    fn pod_outfitting(heavy: i16) -> FakeCatalog {
        let mut catalog = outfitting();
        catalog.character = Ok(CharacterStart {
            cash: 10_000,
            ..catalog.character.expect("a chär")
        });
        let tank = catalog.outfits[0].clone();
        for (id, mass) in [(201, -10), (202, heavy)] {
            catalog.outfits.push(OutfitRecord {
                id: OutfitId(id),
                mass,
                cost: 100,
                lc_name: "pod".to_owned(),
                lc_plural: "pods".to_owned(),
                ..tank.clone()
            });
        }
        catalog
    }

    #[test]
    fn a_counted_sale_refused_part_way_goes_through_the_session_with_its_alert() {
        let pod = OutfitId(201);
        let mut probe = FlightView::new(pod_outfitting(0));
        land_now(&mut probe);
        let free = probe.outfitter().expect("open").free_mass;
        // Three pods and the heavy outfit leave ten tons free: one pod
        // sold leaves none, and the next is refused at pass 1.
        let heavy = i16::try_from(free).expect("small") + 20;
        let mut view = FlightView::new(pod_outfitting(heavy));
        land_now(&mut view);
        let buy = |outfit| OutfitOrder {
            outfit,
            direction: Direction::Buy,
        };
        assert_eq!(view.outfit_order(buy(pod), 3).run.done, 3);
        assert_eq!(view.outfit_order(buy(OutfitId(202)), 1).run.done, 1);
        assert_eq!(view.outfitter().expect("open").free_mass, 10);
        view.take_save_due();
        let sell = OutfitOrder {
            outfit: pod,
            direction: Direction::Sell,
        };
        let order = view.outfit_order(sell, 3);
        assert_eq!(
            order.run,
            OrderRun {
                done: 1,
                stopped: Some(Stopped {
                    pass: 1,
                    refusal: OutfitRefusal::NegativeFreeMass,
                }),
            }
        );
        let alert = order.alert.expect("an alert");
        assert_eq!(alert.refusal, OutfitRefusal::NegativeFreeMass);
        assert!(alert.plural, "a pass remains");
        assert_eq!(view.pilot().map(|pilot| pilot.owned(pod)), Some(2));
        assert!(view.take_save_due(), "a sale");
    }

    #[test]
    fn opening_the_outfitter_lifts_the_sessions_once_an_opening_limit() {
        let mut catalog = outfitting();
        catalog.character = Ok(CharacterStart {
            cash: 5000,
            ..catalog.character.expect("a chär")
        });
        catalog.outfits[0].mods[0] = (nova_sim::outfitter::EXPLORES_MAP, 5);
        let mut view = FlightView::new(catalog);
        land_now(&mut view);
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(view.outfit(BUY_TANK), Err(OutfitRefusal::BoughtThisOpening));
        view.open_outfitter();
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..outfitting()
        };
        FlightView::new(broken).open_outfitter();
    }

    #[test]
    fn recharging_goes_through_the_session() {
        use nova_sim::RechargeRefusal;
        let mut view = FlightView::new(outfitting());
        assert_eq!(view.recharge(), Err(RechargeRefusal::NoFuel), "in flight");
        land_now(&mut view);
        view.take_save_due();
        assert_eq!(view.recharge(), Err(RechargeRefusal::Full));
        assert!(!view.take_save_due());
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..outfitting()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(broken.recharge(), Err(RechargeRefusal::NoFuel));
    }

    /// Fires every time, and records each percent it is asked; each draw
    /// is 0, and records its `n`.
    #[derive(Default)]
    struct Always {
        asked: Vec<u8>,
        drawn: Vec<u32>,
    }

    impl Chance for Always {
        fn fires(&mut self, percent: u8) -> bool {
            self.asked.push(percent);
            true
        }

        fn below(&mut self, n: u32) -> u32 {
            self.drawn.push(n);
            0
        }

        /// The last of the outcomes.
        fn roll(&mut self, sides: u16) -> u16 {
            sides.saturating_sub(1)
        }
    }

    #[test]
    fn a_shared_chance_draws_from_its_source() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut chance = SharedChance::new(shared);
        assert_eq!(chance.below(7), 0);
        assert_eq!(chance.below(500), 0);
        assert_eq!(always.borrow().drawn, [7, 500]);
        assert_eq!(SharedChance::default().below(256), 255, "never fires");
    }

    /// A food surplus at Proxima, 35 % a day.
    fn eventful() -> FakeCatalog {
        FakeCatalog {
            disasters: vec![DisasterRecord {
                id: DisasterId(128),
                name: "An enormous food surplus".to_owned(),
                stellar: 140,
                commodity: 0,
                price_delta: -15,
                duration: 30,
                freq: 35,
                activate_on: nova_sim::Test::default(),
            }],
            ..catalog()
        }
    }

    /// Jumps to Alpha Centauri and arrives.
    fn jump_to_alpha(view: &mut View) {
        plot(view, 131);
        fly_out(view);
        view.input(&key(JUMP, true));
        arrive_now(view);
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
    }

    #[test]
    fn a_jump_rolls_the_days_events_on_the_chance_given() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::new(eventful()).with_chance(SharedChance::new(shared));
        jump_to_alpha(&mut view);
        assert_eq!(always.borrow().asked, [35]);
        let events: Vec<_> = view.pilot().expect("a pilot").events().collect();
        assert_eq!(events, [(DisasterId(128), 30)]);
        assert_eq!(format!("{:?}", SharedChance::default()), "Chance");
    }

    #[test]
    fn leaving_rolls_the_stays_days_on_the_flights_chance() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::with_pilot(eventful(), docked_pilot("Ada"))
            .with_chance(SharedChance::new(shared));
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        always.borrow_mut().asked.clear();
        let landed = view.session().expect("docked").date();
        view.take_off().expect("took off");
        assert_eq!(always.borrow().asked, [35], "the take-off's day");
        assert_eq!(view.session().expect("flying").date(), landed.next_day());
    }

    #[test]
    fn without_a_chance_given_nothing_random_happens() {
        let mut view = FlightView::new(eventful());
        jump_to_alpha(&mut view);
        assert_eq!(view.pilot().expect("a pilot").events().count(), 0);
        let mut shared = SharedChance::default();
        assert!(!shared.fires(100));
        assert_eq!(shared.roll(8), 0);
    }

    #[test]
    fn a_shared_chance_rolls_on_its_source() {
        let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always::default()));
        assert_eq!(SharedChance::new(shared).roll(360), 359);
    }

    use nova_sim::ShipRefusal;

    /// [`outfitting`], where Earth is a shipyard too, selling ship 129 for
    /// 900 credits: twice as fast, with a fuel tank.
    fn shipbuying() -> FakeCatalog {
        let mut earth = site(
            128,
            (0.0, 0.0),
            StellarFlags::CAN_LAND | StellarFlags::OUTFITTER | StellarFlags::SHIPYARD,
        );
        earth.tech_level = 1;
        FakeCatalog {
            sites: vec![earth],
            ships: vec![ShipRecord {
                id: ShipId(129),
                name: "Fast".to_owned(),
                short_name: "Fast".to_owned(),
                long_name: String::new(),
                fields: ShipFields {
                    speed: 600,
                    ..FIELDS
                },
                defaults: vec![(OutfitId(200), 1)],
                cost: 900,
                tech_level: 1,
                buy_random: 100,
                hire_random: 0,
                require: 0,
                availability: nova_sim::Test::default(),
                appear_on: nova_sim::Test::default(),
                flags: 0,
                flags3: 0,
                disp_weight: 0,
                max_gun: 0,
                max_tur: 0,
                length: 0,
                crew: 0,
                inherent_ai: 1,
                comm_name: String::new(),
                inherent_govt: None,
                combat_govt: None,
                escort_type: -1,
                on_capture: nova_sim::Script::default(),
                on_purchase: nova_sim::Script::default(),
                on_retire: nova_sim::Script::default(),
            }],
            ..outfitting()
        }
    }

    #[test]
    fn the_shipyard_is_the_sessions_and_a_purchase_reloads_the_ships_sheet() {
        let mut view = FlightView::new(shipbuying());
        assert_eq!(view.shipyard(), None, "in flight");
        assert_eq!(
            view.buy_ship(ShipId(129), "Kestrel"),
            Err(ShipRefusal::NoShipyard)
        );
        land_now(&mut view);
        view.take_save_due();
        let shipyard = view.shipyard().expect("landed at a shipyard");
        assert_eq!(shipyard.row(ShipId(129)).map(|row| row.price), Some(900));
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
        assert_eq!(
            view.buy_ship(ShipId(999), "Kestrel"),
            Err(ShipRefusal::NotListed),
            "refused"
        );
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
        let bought = view.buy_ship(ShipId(129), "The Kestrel").expect("bought");
        assert_eq!(bought.price, 900);
        assert_eq!(
            view.pilot().and_then(Pilot::ship_name),
            Some("Kestrel"),
            "named as confirmed"
        );
        assert!(view.take_save_due(), "a purchase");
        assert_eq!(view.pilot().map(Pilot::ship), Some(ShipId(129)));
        assert_eq!(view.pilot().map(Pilot::cash), Some(1000 - 900));
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)]
        );
        assert_eq!(view.reserves().fuel.max, 350.0, "its tank");
        // Off again, the new hull is drawn, at the new speed.
        view.settle_script();
        view.take_off().expect("took off");
        view.settle_script();
        view.tick(TICK);
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)],
            "read once"
        );
        let mut list = DrawList::new();
        view.draw(&mut list);
        assert!(
            list.iter().any(|command| matches!(
                command,
                DrawCommand::Sprite { image, .. } if image.id == 2001
            )),
            "the new sheet"
        );
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2001, 2201], "and its lights");
        assert_eq!(
            view.session().map(|session| session.handling().max_speed),
            Ok(6.0)
        );
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..shipbuying()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(broken.shipyard(), None);
        assert_eq!(
            broken.buy_ship(ShipId(129), "Kestrel"),
            Err(ShipRefusal::NoShipyard)
        );
    }

    /// [`shipbuying`], with the fuel tank and ship 129 each for sale on a
    /// `BuyRandom` of 50.
    fn rolling() -> FakeCatalog {
        let mut catalog = shipbuying();
        catalog.outfits[0].buy_random = 50;
        catalog.ships[0].buy_random = 50;
        catalog
    }

    #[test]
    fn the_outfitter_and_shipyard_roll_on_the_flights_chance() {
        let mut view = FlightView::new(rolling()).with_chance(SharedChance::default());
        land_now(&mut view);
        let outfitter = view.outfitter().expect("an outfitter");
        assert!(outfitter.row(OutfitId(200)).is_none(), "off today");
        let shipyard = view.shipyard().expect("a shipyard");
        assert!(shipyard.row(ShipId(129)).is_none(), "off today");
        assert_eq!(view.outfit(BUY_TANK), Err(OutfitRefusal::NotListed));
        assert_eq!(
            view.buy_ship(ShipId(129), "Kestrel"),
            Err(ShipRefusal::NotListed)
        );
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::new(rolling()).with_chance(SharedChance::new(shared));
        land_now(&mut view);
        always.borrow_mut().asked.clear();
        let outfitter = view.outfitter().expect("an outfitter");
        assert!(outfitter.row(OutfitId(200)).is_some(), "on today");
        let shipyard = view.shipyard().expect("a shipyard");
        assert!(shipyard.row(ShipId(129)).is_some(), "on today");
        assert_eq!(always.borrow().asked, [50, 50]);
        assert!(view.buy_ship(ShipId(129), "Kestrel").is_ok());
    }

    #[test]
    fn the_naming_is_the_sessions_on_the_flights_chance() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::new(shipbuying()).with_chance(SharedChance::new(shared));
        assert_eq!(view.ship_naming(ShipId(129)), Err(ShipRefusal::NoShipyard));
        land_now(&mut view);
        view.take_save_due();
        assert_eq!(view.ship_naming(ShipId(999)), Err(ShipRefusal::NotListed));
        let naming = view.ship_naming(ShipId(129)).expect("a naming");
        assert_eq!(naming.ship, ShipId(129));
        assert_eq!(naming.prompt, "Please name your new : ");
        assert_eq!(naming.default, "Fast 999", "each digit the last of nine");
        assert!(!view.take_save_due());
        let mut broken = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..shipbuying()
        });
        assert_eq!(
            broken.ship_naming(ShipId(129)),
            Err(ShipRefusal::NoShipyard)
        );
        broken.decline_ship(ShipId(129));
    }

    #[test]
    fn declining_a_ship_draws_its_roll_again_on_the_flights_chance() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::new(rolling()).with_chance(SharedChance::new(shared));
        land_now(&mut view);
        view.take_save_due();
        view.shipyard().expect("a shipyard");
        always.borrow_mut().asked.clear();
        view.shipyard().expect("a shipyard");
        assert!(always.borrow().asked.is_empty(), "kept");
        view.decline_ship(ShipId(129));
        assert!(!view.take_save_due());
        view.shipyard().expect("a shipyard");
        assert_eq!(always.borrow().asked, [50], "drawn again");
    }

    #[test]
    fn every_rule_reaches_the_session_through_the_flight() {
        let every = RuleKey::ALL
            .into_iter()
            .fold(Rulebook::default(), |book, key| {
                book.with_override(key, RuleSource::Bible)
            });
        assert_eq!(
            flight().session().map(Session::rules),
            Ok(&Rulebook::default())
        );
        let view = flight().with_rules(every);
        let rules = view.session().expect("flying").rules();
        assert_eq!(rules, &every);
        for key in RuleKey::ALL {
            assert_eq!(rules.source_for(key), RuleSource::Bible, "{key:?}");
        }
    }

    // The set-expression hooks.

    /// Rolls the second outcome of every roll, so `R(a b)` takes `a`.
    #[derive(Debug)]
    struct RollsOne;

    impl Chance for RollsOne {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            n.min(1)
        }
    }

    fn rolling_one() -> SharedChance {
        SharedChance::new(Rc::new(RefCell::new(RollsOne)))
    }

    fn bit_set(view: &View, n: u16) -> bool {
        let bit = nova_sim::Bit::new(n).expect("a bit");
        view.pilot().is_some_and(|pilot| pilot.control_bit(bit))
    }

    #[test]
    fn an_outfits_on_purchase_runs_on_the_flights_chance() {
        let mut catalog = outfitting();
        catalog.outfits[0].on_purchase = nova_sim::Script::parse("R(b1 b2)");
        let mut view = FlightView::new(catalog).with_chance(rolling_one());
        land_now(&mut view);
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert!(bit_set(&view, 1));
        assert!(!bit_set(&view, 2));
    }

    #[test]
    fn a_ships_on_purchase_runs_on_the_flights_chance() {
        let mut catalog = shipbuying();
        catalog.ships[0].on_purchase = nova_sim::Script::parse("R(b3 b4)");
        let mut view = FlightView::new(catalog).with_chance(rolling_one());
        land_now(&mut view);
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert!(bit_set(&view, 3));
        assert!(!bit_set(&view, 4));
    }

    #[test]
    fn a_ships_on_purchase_h_reads_only_the_final_class_sheet_once() {
        let mut catalog = shipbuying();
        catalog.ships[0].on_purchase = nova_sim::Script::parse("H130");
        let other = ShipRecord {
            id: ShipId(130),
            ..catalog.ships[0].clone()
        };
        catalog.ships.push(other);
        let mut view = FlightView::new(catalog);
        land_now(&mut view);
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert_eq!(view.session().map(Session::ship), Ok(ShipId(130)));
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(130)]
        );
        view.settle_script();
        view.take_off().expect("took off");
        view.settle_script();
        view.tick(TICK);
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(130)],
            "a failed read is not tried again"
        );
        assert_eq!(view.frame(), None, "no sheet for 130");
    }

    /// [`shipbuying`], ship 129 carrying IFF ([`radar_outfits`]' 300)
    /// among its default outfits, landed.
    fn landed_shipyard_with_iff() -> View {
        let mut catalog = shipbuying();
        catalog.outfits.extend(radar_outfits());
        catalog.ships[0].defaults.push((OutfitId(300), 1));
        let mut view = FlightView::new(catalog);
        land_now(&mut view);
        view
    }

    #[test]
    fn a_ship_bought_with_iff_shows_it_at_once() {
        let mut view = landed_shipyard_with_iff();
        assert!(view.hud_waits(), "the engine's reading");
        assert!(!view.radar_shown().iff, "landing's");
        assert_eq!(
            view.buy_ship(ShipId(999), "Kestrel"),
            Err(ShipRefusal::NotListed)
        );
        assert!(!view.radar_shown().iff, "refused");
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert!(view.radar_shown().iff, "its default IFF");
        assert!(radar_filled(&view));
    }

    #[test]
    fn a_ship_bought_redraws_the_nav_area_and_the_target_panel() {
        let mut view = landed_shipyard_with_iff();
        let stale_nav = NavDisplay::Stellar(Some("Stale".to_owned()));
        let stale_target = TargetPanel {
            name: "Stale".to_owned(),
            subtitle: String::new(),
            picture: None,
            status: nova_sim::TargetStatus::Hidden,
            tag: None,
        };
        let stale = |view: &mut View| {
            view.nav_scene = Some((view.nav_mode(), stale_nav.clone()));
            view.target_scene = Some((None, Some(stale_target.clone())));
        };
        stale(&mut view);
        assert_eq!(view.nav_shown(), stale_nav, "held");
        assert_eq!(*view.target_shown(), Some(stale_target.clone()), "held");
        assert!(view.buy_ship(ShipId(999), "Kestrel").is_err());
        assert_eq!(view.nav_shown(), view.nav_now(), "the prompt's close");
        stale(&mut view);
        assert_eq!(view.nav_shown(), stale_nav, "held again");
        assert_eq!(*view.target_shown(), Some(stale_target.clone()), "held");
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert_eq!(view.nav_shown(), view.nav_now(), "redrawn");
        assert_ne!(view.nav_shown(), stale_nav);
        assert_eq!(*view.target_shown(), None, "redrawn");
    }

    // The nav area while docked.

    /// The texts drawn in the nav area of the port's status bar
    /// ([`FlightView::draw_status_bar`]), as [`nav`] reads flight's.
    fn port_nav(view: &View) -> Vec<String> {
        let area = layout().nav.offset(at(830.0, 0.0));
        let mut bar = DrawList::new();
        view.draw_status_bar(&mut bar);
        bar.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } if area.contains(*origin) => {
                    Some(text.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// The nav area naming `name` in mode 2.
    fn naming(name: &str) -> Vec<String> {
        vec![hud::NAV_STELLAR.to_owned(), name.to_owned()]
    }

    /// Outfit 201: [`outfitting`]'s fuel tank with no `OnPurchase`.
    const BUY_PLAIN_TANK: OutfitOrder = OutfitOrder {
        outfit: OutfitId(201),
        direction: Direction::Buy,
    };

    /// An outfit no outfitter sells, so its order is refused.
    const UNKNOWN_OUTFIT: OutfitOrder = OutfitOrder {
        outfit: OutfitId(999),
        direction: Direction::Buy,
    };

    /// [`shipbuying`]'s Earth, a trade center too selling food, the ship
    /// holding 10 tons; ship 130 under gövt 150, whose `Interface` is 129
    /// ([`governed`]); the fuel tank's `OnPurchase` an `H130`, and outfit
    /// 201 a tank with none; missions 130 and 131, errands at the mission
    /// computer, 131 with a `BriefText`; Barnard (132), with no stellar;
    /// and Proxima (140), Alpha Centauri's first stellar, named by the
    /// session alone, as the flight scene is Sol's while docked.
    fn port() -> FakeCatalog {
        let mut catalog = governed(shipbuying(), 130, 129);
        catalog.sites[0].flags |= trading().sites[0].flags;
        catalog.fields.holds = 10;
        catalog.commodities = trading().commodities;
        let plain = OutfitRecord {
            id: OutfitId(201),
            ..catalog.outfits[0].clone()
        };
        catalog.outfits[0].on_purchase = nova_sim::Script::parse("H130");
        catalog.outfits.push(plain);
        catalog.missions = vec![errand(130), errand(131)];
        catalog.mission_texts = vec![MissionTexts {
            brief: Some("Go.".to_owned()),
            ..MissionTexts::named(MissionId(131), "Walk")
        }];
        catalog.onward = true;
        catalog.place_names.stellars = BTreeMap::from([(StellarId(140), "Proxima".to_owned())]);
        catalog
    }

    /// A view of `catalog` landed on Earth in mode 2, settled, with the
    /// HUD's cadence read as `cadence` says.
    fn docked_at_earth(catalog: FakeCatalog, cadence: RuleSource) -> View {
        let rules = Rulebook::default().with_override(RuleKey::HudCadence, cadence);
        let mut view = FlightView::new(catalog).with_rules(rules);
        land_now(&mut view);
        view.settle_script();
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Stellar(Some(StellarId(128))))
        );
        assert_eq!(port_nav(&view), naming("Earth"));
        view
    }

    #[test]
    fn by_the_bible_the_port_names_a_landed_ms_stellar_outside_the_scene_at_once() {
        let mut view = docked_at_earth(port(), RuleSource::Bible);
        run_set(&mut view, "M131");
        assert_eq!(port_nav(&view), naming("Proxima"), "the session's name");
        assert_eq!(
            view.scene().map(SystemScene::id),
            Some(SystemId(130)),
            "Sol's scene"
        );
        run_set(&mut view, "M132");
        assert_eq!(port_nav(&view), naming(hud::NAV_NO_DESTINATION));
    }

    /// An accept's and a refusal's redraws are pinned by
    /// [`a_dialogs_redraw_shows_what_the_script_run_with_it_wrote`], as a
    /// move empties the offers.
    #[test]
    fn by_the_engine_a_landed_ms_stellar_shows_at_the_ports_next_nav_redraw() {
        type Step = fn(&mut View);
        let redraws: [(&str, Step); 6] = [
            ("a shop closing", FlightView::refresh_status),
            ("a name prompt's Cancel", |view| {
                view.decline_ship(ShipId(129));
            }),
            ("a refused purchase, after its prompt's close", |view| {
                assert!(view.buy_ship(ShipId(999), "Kestrel").is_err());
            }),
            ("a mission text closing", |view| {
                run_set(view, "S131");
                assert_eq!(port_nav(view), naming("Earth"), "queued, not shown");
                assert_eq!(view.take_mission_texts().len(), 1);
                assert_eq!(port_nav(view), naming("Earth"), "shown over the bar");
                view.close_mission_text();
            }),
            ("a ship purchase", |view| {
                view.buy_ship(ShipId(129), "Kestrel").expect("bought");
            }),
            ("an outfitter order changing the ïntf", |view| {
                assert_eq!(view.outfit(BUY_TANK), Ok(()));
                assert_eq!(view.session().map(Session::ship), Ok(ShipId(130)));
            }),
        ];
        let held: [(&str, Step); 3] = [
            ("a trade", |view| {
                assert_eq!(view.trade(BUY_FOOD), Ok(10));
            }),
            ("a recharge", |view| {
                desk_reserve(view, nova_sim::Reserve::Fuel, 100.0);
                assert!(view.recharge().is_ok());
            }),
            ("an outfitter order keeping the ïntf", |view| {
                assert_eq!(view.outfit(BUY_PLAIN_TANK), Ok(()));
            }),
        ];
        for (steps, shown) in [(&redraws[..], "Proxima"), (&held[..], "Earth")] {
            for (what, step) in steps {
                let mut view = docked_at_earth(port(), RuleSource::Engine);
                run_set(&mut view, "M131");
                view.settle_script();
                assert_eq!(
                    view.session().map(Session::nav_mode),
                    Ok(NavMode::Stellar(Some(StellarId(140))))
                );
                assert_eq!(port_nav(&view), naming("Earth"), "{what}: held");
                step(&mut view);
                assert_eq!(port_nav(&view), naming(shown), "{what}");
            }
        }
    }

    /// [`docked_at_earth`] by the engine's reading, with `dialog` opened,
    /// a landed `M131` run and settled inside it, and `dialog` closed.
    fn m_inside(dialog: PortDialog) -> View {
        m_inside_of(port(), dialog)
    }

    /// [`m_inside`] over `catalog`.
    fn m_inside_of(catalog: FakeCatalog, dialog: PortDialog) -> View {
        let mut view = docked_at_earth(catalog, RuleSource::Engine);
        view.open_port_dialog();
        run_set(&mut view, "M131");
        view.settle_script();
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Stellar(Some(StellarId(140)))),
            "{dialog:?}: written"
        );
        view.close_port_dialog(dialog);
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Stellar(Some(StellarId(128)))),
            "{dialog:?}: restored"
        );
        view
    }

    /// [`port`] with a bar on Earth, a view of it landed there by the
    /// engine's reading (as [`docked_at_earth`]), with `course` plotted
    /// in flight first if given, and the bar opened.
    fn in_the_bar(course: Option<i16>) -> View {
        let mut catalog = port();
        catalog.sites[0].flags |= StellarFlags::BAR;
        let rules = Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Engine);
        let mut view = FlightView::new(catalog).with_rules(rules);
        if let Some(to) = course {
            view.session
                .as_mut()
                .expect("flying")
                .plot_course(SystemId(to))
                .expect("a route");
        }
        land_now(&mut view);
        view.settle_script();
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Stellar(Some(StellarId(128)))),
            "the land key put mode 2 over any cue"
        );
        assert_eq!(port_nav(&view), naming("Earth"));
        assert!(view.open_bar());
        view
    }

    /// Clicks system `id` on the map open in port.
    fn port_click(view: &mut View, id: i16) {
        let at = on_map(view, id);
        for pressed in [true, false] {
            view.port_map_input(&Input::PointerButton {
                button: crate::MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    /// The nav area showing Alpha Centauri (131), unexplored, in mode 3.
    fn cueing_alpha() -> Vec<String> {
        vec![
            hud::NAV_HYPERSPACE.to_owned(),
            hud::NAV_UNEXPLORED.to_owned(),
        ]
    }

    #[test]
    fn the_bars_map_closing_shows_the_maps_selection_and_puts_back_the_bars_nav() {
        let mut view = in_the_bar(None);
        view.open_bar_map();
        assert!(view.map_open());
        assert_eq!(view.course_map().mode(), MapMode::Course);
        port_click(&mut view, 131);
        assert!(view.map_open(), "a click doesn't close it");
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Hyperspace(Some(SystemId(131)))),
            "the click cues the route"
        );
        view.port_map_input(&key(MAP, true));
        assert!(!view.map_open(), "M closes it");
        assert_eq!(port_nav(&view), cueing_alpha(), "the map's selection drawn");
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Stellar(Some(StellarId(128)))),
            "the bar's nav put back"
        );
        assert_eq!(
            view.session().map(|session| session.course().to_vec()),
            Ok(vec![SystemId(131)]),
            "the course kept"
        );
        view.refresh_status();
        assert_eq!(port_nav(&view), naming("Earth"), "the bar's close");
    }

    /// Only the bar's own map puts back the bar's nav: a map opened in
    /// port some other way (as from the other port screens will be)
    /// leaves the nav its click cued, with the bar's save still held,
    /// even after the bar's own map has opened and closed.
    #[test]
    fn a_map_in_port_the_bar_did_not_open_leaves_the_bars_nav_saved() {
        let mut view = in_the_bar(None);
        view.open_bar_map();
        view.port_map_input(&key(MAP, true));
        assert!(!view.map_open(), "the bar's map closed");
        view.open_map();
        port_click(&mut view, 131);
        view.port_map_input(&key(MAP, true));
        assert!(!view.map_open());
        assert_eq!(port_nav(&view), cueing_alpha(), "the map's selection drawn");
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Hyperspace(Some(SystemId(131)))),
            "the bar's nav not put back"
        );
        assert_eq!(
            view.session().map(|session| session.course().to_vec()),
            Ok(vec![SystemId(131)]),
            "the course kept"
        );
    }

    #[test]
    fn the_bars_map_key_repeat_or_release_leaves_it_open() {
        let mut view = in_the_bar(None);
        view.open_bar_map();
        view.port_map_input(&Input::Key {
            key: MAP,
            pressed: true,
            repeat: true,
        });
        view.port_map_input(&key(MAP, false));
        assert!(view.map_open());
    }

    /// The map's beeps are `_HandlePlayer`'s, around the map M opens in
    /// flight (@0x6a809, @0x6a85a): the map opened in port makes neither.
    #[test]
    fn the_map_in_port_beeps_neither_as_it_opens_nor_as_it_closes() {
        let mut view = in_the_bar(None);
        view.take_sounds();
        view.open_bar_map();
        assert_eq!(map_sounds(&mut view), [], "the bar's map opened");
        view.port_map_input(&key(MAP, true));
        assert!(!view.map_open());
        assert_eq!(map_sounds(&mut view), [], "the bar's map closed");
        view.open_map();
        view.close_map();
        assert_eq!(map_sounds(&mut view), [], "another map in port");
    }

    /// Ours cues a route's first hop at the click ([`Session::plot_course`]),
    /// so the port close's skipped cue (`_DoSystemMap` @0x13be2-0x13c0a)
    /// shows only with a course plotted before the map opened, whose cue
    /// the land key put mode 2 over.
    #[test]
    fn in_port_the_maps_close_cues_no_course() {
        let mut view = in_the_bar(Some(131));
        view.open_bar_map();
        view.port_map_input(&key(MAP, true));
        assert!(!view.map_open());
        assert_eq!(port_nav(&view), naming("Earth"), "no cue drawn");
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Stellar(Some(StellarId(128))))
        );
        assert_eq!(
            view.session().map(|session| session.course().to_vec()),
            Ok(vec![SystemId(131)]),
            "the course kept"
        );
    }

    #[test]
    fn an_empty_outfitter_draws_a_nav_the_shipyard_left_marked() {
        let mut view = m_inside_of(port(), PortDialog::Shipyard);
        assert_eq!(port_nav(&view), naming("Proxima"), "held after the close");
        view.empty_outfitter_returned();
        assert_eq!(port_nav(&view), naming("Earth"), "the restore drawn");
        run_set(&mut view, "M131");
        view.settle_script();
        view.empty_outfitter_returned();
        assert_eq!(port_nav(&view), naming("Earth"), "nothing marked");
    }

    #[test]
    fn a_landed_m_in_the_outfitter_exchange_or_computer_is_undone_and_shown_at_once() {
        for dialog in [
            PortDialog::Outfitter,
            PortDialog::Exchange,
            PortDialog::MissionComputer,
        ] {
            let view = m_inside(dialog);
            assert_eq!(port_nav(&view), naming("Earth"), "{dialog:?}");
        }
    }

    #[test]
    fn a_landed_m_in_the_shipyard_is_undone_but_its_close_shows_it_until_the_next_draw_status() {
        type Step = fn(&mut View);
        let draws: [(&str, Step); 10] = [
            ("a trade", |view| {
                assert_eq!(view.trade(BUY_FOOD), Ok(10));
            }),
            ("a recharge", |view| {
                desk_reserve(view, nova_sim::Reserve::Fuel, 100.0);
                assert!(view.recharge().is_ok());
            }),
            ("an outfitter order", |view| {
                assert_eq!(view.outfit(BUY_PLAIN_TANK), Ok(()));
            }),
            ("a counted outfitter order", |view| {
                assert_eq!(view.outfit_counted(BUY_PLAIN_TANK, 1), Ok(1));
            }),
            ("an outfitter run", |view| {
                assert_eq!(view.outfit_order(BUY_PLAIN_TANK, 1).run.done, 1);
            }),
            ("an outfitter run stopped after its first pass", |view| {
                let run = view.outfit_order(BUY_PLAIN_TANK, 10_000).run;
                assert!(run.done > 0, "{run:?}");
                assert!(run.stopped.is_some(), "{run:?}");
            }),
            ("a counted order's Cancel", |view| {
                assert_eq!(view.outfit_counted(BUY_PLAIN_TANK, 0), Ok(0));
            }),
            ("an outfitter run's Cancel", |view| {
                let run = view.outfit_order(BUY_PLAIN_TANK, 0).run;
                assert_eq!((run.done, run.stopped), (0, None));
            }),
            ("a ship purchase", |view| {
                view.buy_ship(ShipId(129), "Kestrel").expect("bought");
            }),
            ("a shop closing", FlightView::refresh_status),
        ];
        let refused: [(&str, Step); 8] = [
            ("a refused trade", |view| {
                let order = Order {
                    row: 99,
                    ..BUY_FOOD
                };
                assert!(view.trade(order).is_err());
            }),
            ("a refused recharge", |view| {
                assert_eq!(view.recharge(), Err(RechargeRefusal::Full));
            }),
            ("a refused outfitter order", |view| {
                assert!(view.outfit(UNKNOWN_OUTFIT).is_err());
            }),
            ("a refused counted outfitter order", |view| {
                assert!(view.outfit_counted(UNKNOWN_OUTFIT, 1).is_err());
            }),
            ("a refused outfitter run", |view| {
                assert_eq!(view.outfit_order(UNKNOWN_OUTFIT, 1).run.done, 0);
            }),
            ("a Cancel whose Buy fails its check", |view| {
                assert_eq!(view.outfit_order(UNKNOWN_OUTFIT, 0).run.done, 0);
            }),
            (
                "a counted order's Cancel whose Buy fails its check",
                |view| {
                    assert_eq!(view.outfit_counted(UNKNOWN_OUTFIT, 0), Ok(0));
                },
            ),
            ("a Cancel whose Sell fails its check", |view| {
                let sell = OutfitOrder {
                    direction: Direction::Sell,
                    ..BUY_PLAIN_TANK
                };
                assert_eq!(view.outfit_order(sell, 0).run.done, 0);
            }),
        ];
        for dialog in [PortDialog::Shipyard, PortDialog::Hire] {
            for (steps, shown) in [(&draws[..], "Earth"), (&refused[..], "Proxima")] {
                for (what, step) in steps {
                    let mut view = m_inside(dialog);
                    assert_eq!(
                        port_nav(&view),
                        naming("Proxima"),
                        "{dialog:?}, {what}: the close's"
                    );
                    step(&mut view);
                    assert_eq!(port_nav(&view), naming(shown), "{dialog:?}, {what}");
                }
            }
        }
    }

    #[test]
    fn a_redraw_clears_the_shipyards_nav_mark() {
        type Step = fn(&mut View);
        let redraws: [(&str, Step); 2] = [
            ("a whole-bar redraw", FlightView::refresh_status),
            ("a ship purchase", |view| {
                view.buy_ship(ShipId(129), "Kestrel").expect("bought");
                let mut desk = view.pilot_desk().expect("flying");
                desk.edit(PilotEdit::Credits(100_000)).expect("set");
            }),
        ];
        let draws: [(&str, Step); 3] = [
            ("a trade", |view| {
                assert!(view.trade(BUY_FOOD).is_ok());
            }),
            ("a recharge", |view| {
                desk_reserve(view, nova_sim::Reserve::Fuel, 100.0);
                assert!(view.recharge().is_ok());
            }),
            ("an outfitter order", |view| {
                assert_eq!(view.outfit(BUY_PLAIN_TANK), Ok(()));
            }),
        ];
        // Ship 129 holds food too, and the desk refills the credits the
        // purchase spent, so each draw after it goes through.
        let catalog = || {
            let mut catalog = port();
            catalog.ships[0].fields.holds = 10;
            catalog
        };
        for (redrawn, redraw) in &redraws {
            for (drawn, draw) in &draws {
                let mut view = m_inside_of(catalog(), PortDialog::Shipyard);
                assert_eq!(port_nav(&view), naming("Proxima"), "the close's");
                redraw(&mut view);
                assert_eq!(port_nav(&view), naming("Earth"), "{redrawn}");
                run_set(&mut view, "M131");
                view.settle_script();
                assert_eq!(port_nav(&view), naming("Earth"), "{redrawn}: held");
                draw(&mut view);
                assert_eq!(
                    port_nav(&view),
                    naming("Earth"),
                    "{redrawn}, then {drawn}: the mark was cleared"
                );
            }
        }
    }

    #[test]
    fn the_mission_computers_close_redraws_the_whole_bar() {
        let mut view = docked_at_earth(port(), RuleSource::Engine);
        run_set(&mut view, "M131");
        view.settle_script();
        desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
        assert_eq!(port_nav(&view), naming("Earth"), "held");
        assert_ne!(view.reserves_shown().fuel.now, 100.0, "held");
        view.open_port_dialog();
        view.close_port_dialog(PortDialog::MissionComputer);
        assert_eq!(
            port_nav(&view),
            naming("Proxima"),
            "redrawn, though the restore changed nothing"
        );
        assert_eq!(view.reserves_shown().fuel.now, 100.0, "redrawn");
    }

    #[test]
    fn by_the_engine_a_landed_n_shows_no_destination_at_the_next_redraw() {
        let mut view = docked_at_earth(port(), RuleSource::Engine);
        run_set(&mut view, "N131");
        view.settle_script();
        assert_eq!(port_nav(&view), naming("Earth"), "held");
        view.refresh_status();
        assert_eq!(port_nav(&view), naming(hud::NAV_NO_DESTINATION));
    }

    #[test]
    fn a_comp_text_shows_the_landed_stellar_and_the_next_redraw_its_on_successs_move() {
        let mut catalog = port();
        catalog.missions = vec![MissionRecord {
            travel_stel: 128,
            on_success: nova_sim::Script::parse("M131"),
            ..errand(130)
        }];
        catalog.mission_texts = vec![MissionTexts {
            comp: Some("Done.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        }];
        let mut view = docked_at_earth(catalog, RuleSource::Engine);
        run_set(&mut view, "S130");
        view.take_off().expect("takes off");
        view.settle_script();
        land_now(&mut view);
        view.settle_script();
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Stellar(Some(StellarId(140)))),
            "OnSuccess ran"
        );
        assert_eq!(view.take_mission_texts().len(), 1);
        assert_eq!(port_nav(&view), naming("Earth"), "as the text noted it");
        view.refresh_status();
        assert_eq!(port_nav(&view), naming("Proxima"));
    }

    #[test]
    fn a_dialogs_redraw_shows_what_the_script_run_with_it_wrote() {
        type Fit = fn(&mut FakeCatalog);
        type Step = fn(&mut View);
        let rows: [(&str, Fit, Step); 5] = [
            (
                "OnAccept",
                |catalog| catalog.missions[0].on_accept = nova_sim::Script::parse("M131"),
                |view| assert_eq!(view.accept_mission(MissionId(130)), Ok(0)),
            ),
            (
                "OnRefuse",
                |catalog| catalog.missions[0].on_refuse = nova_sim::Script::parse("M131"),
                |view| assert_eq!(view.refuse_mission(MissionId(130)).map(|_| ()), Ok(())),
            ),
            (
                "a RefuseText before OnRefuse",
                |catalog| {
                    catalog.missions[0].on_refuse = nova_sim::Script::parse("M131");
                    catalog.mission_texts = vec![MissionTexts {
                        refuse: Some("No.".to_owned()),
                        ..MissionTexts::named(MissionId(130), "Run")
                    }];
                },
                |view| {
                    assert_eq!(view.refuse_mission(MissionId(130)).map(|_| ()), Ok(()));
                    assert_eq!(view.take_mission_texts().len(), 1);
                    view.close_mission_text();
                },
            ),
            (
                "a BriefText after OnAccept",
                |catalog| {
                    catalog.missions[0].on_accept = nova_sim::Script::parse("M131");
                    catalog.mission_texts = vec![MissionTexts {
                        brief: Some("Go.".to_owned()),
                        ..MissionTexts::named(MissionId(130), "Run")
                    }];
                },
                |view| {
                    assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
                    assert_eq!(view.take_mission_texts().len(), 1);
                    view.close_mission_text();
                },
            ),
            (
                "a ship's OnPurchase",
                |catalog| {
                    let ship = catalog
                        .ships
                        .iter_mut()
                        .find(|ship| ship.id == ShipId(129))
                        .expect("ship 129");
                    ship.on_purchase = nova_sim::Script::parse("M131");
                },
                |view| {
                    view.buy_ship(ShipId(129), "Kestrel").expect("bought");
                },
            ),
        ];
        for (what, fit, step) in rows {
            let mut catalog = port();
            fit(&mut catalog);
            let mut view = docked_at_earth(catalog, RuleSource::Engine);
            step(&mut view);
            assert_eq!(port_nav(&view), naming("Proxima"), "{what}");
        }
    }

    #[test]
    fn a_text_never_shows_over_a_later_redraw() {
        let mut catalog = port();
        catalog.missions[0].on_accept = nova_sim::Script::parse("M131");
        catalog.mission_texts = vec![MissionTexts {
            brief: Some("Go.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        }];
        let mut view = docked_at_earth(catalog, RuleSource::Engine);
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        run_set(&mut view, "M132");
        view.refresh_status();
        assert_eq!(port_nav(&view), naming(hud::NAV_NO_DESTINATION));
        assert_eq!(
            view.take_mission_texts().len(),
            1,
            "the brief, noting Proxima"
        );
        assert_eq!(port_nav(&view), naming(hud::NAV_NO_DESTINATION));
        view.close_mission_text();
        assert_eq!(
            port_nav(&view),
            naming(hud::NAV_NO_DESTINATION),
            "the brief's close dropped with it"
        );
    }

    // A docked mission script's `C`, `E` and `H`, and the bar under its
    // texts.

    use nova_sim::mission::screens::SPACEPORT;

    /// [`port`] fitted by `fit`, docked at Earth by the engine's cadence.
    fn docked_scripted(fit: impl FnOnce(&mut FakeCatalog)) -> View {
        let mut catalog = port();
        fit(&mut catalog);
        docked_at_earth(catalog, RuleSource::Engine)
    }

    /// An `H` that puts the ship under another `ïntf` (ship 130, under
    /// 129), and one that keeps it (ship 129, under 128 as ship 128 is).
    const INTF_ROWS: [(&str, bool); 2] = [("H130", true), ("H129", false)];

    #[test]
    fn a_docked_on_abort_intf_change_redraws_the_whole_bar_at_the_operator() {
        for (h, changes) in INTF_ROWS {
            let mut view = docked_scripted(|catalog| {
                catalog.missions[0].on_abort = nova_sim::Script::parse(&format!("{h} M131"));
            });
            assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
            stale_bar(&mut view);
            view.abort_mission(0).expect("aborted");
            if changes {
                assert_eq!(
                    retaken(&view),
                    [true, false, true, true],
                    "{h}: the whole bar, the nav from before the M"
                );
                assert_eq!(port_nav(&view), naming("Earth"), "{h}");
            } else {
                assert_eq!(retaken(&view), [false; 4], "{h}: held");
            }
        }
    }

    #[test]
    fn a_docked_jettisons_on_failure_intf_change_redraws_the_whole_bar_at_the_operator() {
        for (h, changes) in INTF_ROWS {
            let mut view = docked_scripted(|catalog| {
                let mission = &mut catalog.missions[0];
                mission.cargo_type = 0;
                mission.cargo_qty = 1;
                mission.pickup_mode = 0;
                mission.on_failure = nova_sim::Script::parse(&format!("{h} M131"));
            });
            assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
            stale_bar(&mut view);
            view.jettison_all();
            assert_eq!(
                view.session()
                    .map(|session| session.pilot().missions().missions().len()),
                Ok(0),
                "{h}: failed"
            );
            if changes {
                assert_eq!(retaken(&view), [true, false, true, true], "{h}");
            } else {
                assert_eq!(retaken(&view), [false; 4], "{h}: held");
            }
        }
    }

    /// How an accept is made: from the mission computer, from the offer
    /// dialog with a refuse button, or at once with no dialog.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Accepted {
        Computer,
        Offer,
        AtOnce,
    }

    #[test]
    fn a_brief_text_shows_over_the_bar_as_on_accept_left_it() {
        for (h, changes) in INTF_ROWS {
            for how in [Accepted::Computer, Accepted::Offer, Accepted::AtOnce] {
                let what = format!("{h}, {how:?}");
                let mut view = docked_scripted(|catalog| {
                    let mission = &mut catalog.missions[1];
                    mission.on_accept = nova_sim::Script::parse(&format!("{h} G201 M131"));
                    if how != Accepted::Computer {
                        mission.avail_loc = SPACEPORT;
                    }
                    if how == Accepted::AtOnce {
                        mission.flags = nova_sim::mission::screens::UNREFUSABLE;
                    }
                });
                assert_eq!(
                    view.session()
                        .map(|session| session.refusable_offer_dialog(MissionId(131))),
                    Ok(how == Accepted::Offer),
                    "{what}"
                );
                let before = view.reserves();
                stale_bar(&mut view);
                if how == Accepted::AtOnce {
                    assert!(
                        matches!(
                            view.next_offer(SPACEPORT),
                            Some(VenueOffer::Accepted {
                                accepted: Ok(0),
                                ..
                            })
                        ),
                        "{what}"
                    );
                } else {
                    assert_eq!(view.accept_mission(MissionId(131)), Ok(0), "{what}");
                }
                assert_eq!(view.take_mission_texts().len(), 1, "{what}: the brief");
                let shown = view.reserves_shown();
                match (changes, how) {
                    (true, _) => {
                        assert_eq!(
                            shown.fuel.max,
                            view.reserves().fuel.max - 100.0,
                            "{what}: as at the H, before G201's tank"
                        );
                        assert_eq!(port_nav(&view), naming("Earth"), "{what}: before the M");
                    }
                    (false, Accepted::Offer) => {
                        assert_eq!(shown, before, "{what}: the dialog's redraw before it");
                        assert_eq!(port_nav(&view), naming("Earth"), "{what}");
                    }
                    (false, _) => {
                        assert_eq!(retaken(&view), [false; 4], "{what}: held");
                        assert_eq!(port_nav(&view), naming("Stale"), "{what}");
                    }
                }
                view.close_mission_text();
                assert_eq!(retaken(&view), WHOLE_BAR, "{what}: the brief closed");
                assert_eq!(port_nav(&view), naming("Proxima"), "{what}");
            }
        }
    }

    #[test]
    fn a_refuse_text_shows_over_the_bar_from_before_the_refusal() {
        for script in ["M131 G201", "H130 M131 G201"] {
            let mut view = docked_scripted(|catalog| {
                catalog.missions[0].avail_loc = SPACEPORT;
                catalog.missions[0].on_refuse = nova_sim::Script::parse(script);
                catalog.mission_texts.push(MissionTexts {
                    refuse: Some("No.".to_owned()),
                    ..MissionTexts::named(MissionId(130), "Run")
                });
            });
            let before = view.reserves();
            stale_bar(&mut view);
            assert_eq!(view.refuse_mission(MissionId(130)).map(|_| ()), Ok(()));
            assert_eq!(view.take_mission_texts().len(), 1, "{script}");
            assert_eq!(view.reserves_shown(), before, "{script}: before OnRefuse");
            assert_eq!(port_nav(&view), naming("Earth"), "{script}");
            view.close_mission_text();
            assert_eq!(retaken(&view), WHOLE_BAR, "{script}: the text closed");
            assert_eq!(port_nav(&view), naming("Proxima"), "{script}");
        }
    }

    /// [`port`] where mission 130 goes to Earth, its `CompText` "Done."
    /// and its `FailText` "Failed.", with `on_success` and `on_failure`,
    /// accepted by `S130` (and failed by `F130` when `fail`), then landed
    /// on again: the landing's gauges, and the view before its landing
    /// settles.
    fn relanded(on_success: &str, on_failure: &str, fail: bool) -> (Reserves, View) {
        relanded_by(RuleSource::Engine, on_success, on_failure, fail)
    }

    /// [`relanded`], the HUD's cadence read as `cadence` says.
    fn relanded_by(
        cadence: RuleSource,
        on_success: &str,
        on_failure: &str,
        fail: bool,
    ) -> (Reserves, View) {
        let mut catalog = port();
        catalog.missions = vec![MissionRecord {
            travel_stel: 128,
            on_success: nova_sim::Script::parse(on_success),
            on_failure: nova_sim::Script::parse(on_failure),
            ..errand(130)
        }];
        catalog.mission_texts = vec![MissionTexts {
            comp: Some("Done.".to_owned()),
            fail: Some("Failed.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        }];
        let mut view = docked_at_earth(catalog, cadence);
        run_set(&mut view, if fail { "S130 F130" } else { "S130" });
        view.take_off().expect("takes off");
        view.settle_script();
        land_now(&mut view);
        (view.reserves_shown(), view)
    }

    /// A text queued before an `H` onto ship 130 (under `ïntf` 129, whose
    /// background is `PICT` 705): a refusal's `RefuseText` before its
    /// `OnRefuse` (@0xa2f04 → @0xa2f41), a landing's `CompText` before
    /// its `OnSuccess` (@0xa048c → @0xa04f8), and a `ShipDoneText` before
    /// its `OnShipDone` (@0x9ed0f → @0x9ed70), of a mission to destroy
    /// no ships that an `S` in a landing's `OnSuccess` starts, whose goals
    /// the landing's trailing check meets. Each gives the view, docked
    /// with the text taken and a frame's catch-up made.
    fn texts_before_an_h(cadence: RuleSource) -> Vec<(&'static str, View)> {
        let refusal = {
            let mut catalog = port();
            catalog.missions[0].avail_loc = SPACEPORT;
            catalog.missions[0].on_refuse = nova_sim::Script::parse("H130");
            catalog.mission_texts.push(MissionTexts {
                refuse: Some("No.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            });
            let mut view = docked_at_earth(catalog, cadence);
            assert_eq!(bar_shown(&view), Some(700), "ship 128's");
            assert_eq!(view.refuse_mission(MissionId(130)).map(|_| ()), Ok(()));
            view
        };
        let landing = {
            let (_, mut view) = relanded_by(cadence, "H130", "", false);
            assert_eq!(bar_shown(&view), Some(700), "ship 128's");
            view.settle_script();
            view
        };
        let ship_done = {
            let mut catalog = port();
            catalog.missions = vec![
                MissionRecord {
                    travel_stel: 128,
                    on_success: nova_sim::Script::parse("S131"),
                    ..errand(130)
                },
                MissionRecord {
                    on_ship_done: nova_sim::Script::parse("H130"),
                    ships: MissionShip {
                        goal: 0,
                        count: 0,
                        ..MissionShip::NONE
                    },
                    ..errand(131)
                },
            ];
            catalog.mission_texts = vec![
                MissionTexts::named(MissionId(130), "Run"),
                MissionTexts {
                    ship_done: Some("Met.".to_owned()),
                    ..MissionTexts::named(MissionId(131), "Escort")
                },
            ];
            let mut view = docked_at_earth(catalog, cadence);
            run_set(&mut view, "S130");
            view.take_off().expect("takes off");
            view.settle_script();
            land_now(&mut view);
            assert_eq!(bar_shown(&view), Some(700), "ship 128's");
            view.settle_script();
            view
        };
        let views = vec![
            ("a RefuseText", refusal),
            ("a CompText", landing),
            ("a ShipDoneText", ship_done),
        ];
        views
            .into_iter()
            .map(|(what, mut view)| {
                assert_eq!(view.take_mission_texts().len(), 1, "{what}");
                view.catch_up();
                assert_eq!(
                    view.session().map(Session::ship),
                    Ok(ShipId(130)),
                    "{what}: the H ran"
                );
                (what, view)
            })
            .collect()
    }

    #[test]
    fn a_text_queued_before_an_h_shows_over_the_old_graphic_until_it_closes() {
        for (what, mut view) in texts_before_an_h(RuleSource::Engine) {
            assert_eq!(bar_shown(&view), Some(700), "{what}: the old graphic");
            view.tick(TICK);
            assert_eq!(bar_shown(&view), Some(700), "{what}: a frame on");
            view.close_mission_text();
            assert_eq!(bar_shown(&view), Some(705), "{what}: at the H, once closed");
            view.tick(TICK);
            assert_eq!(bar_shown(&view), Some(705), "{what}: kept");
        }
    }

    /// [`port`] where the tank's `OnPurchase` (or, `sell`, its `OnSell`) is
    /// `script`, docked, with a tank to sell when `sell`, and the nav
    /// marked by the shipyard's restore when `marked`; the bar held stale,
    /// then the order made and its texts taken.
    fn outfit_hooked(script: &str, sell: bool, marked: bool) -> View {
        let mut view = docked_scripted(|catalog| {
            let tank = &mut catalog.outfits[0];
            if sell {
                tank.on_purchase = nova_sim::Script::default();
                tank.on_sell = nova_sim::Script::parse(script);
            } else {
                tank.on_purchase = nova_sim::Script::parse(script);
            }
        });
        if sell {
            assert_eq!(view.outfit(BUY_TANK), Ok(()), "one to sell");
        }
        if marked {
            view.open_port_dialog();
            view.close_port_dialog(PortDialog::Shipyard);
        }
        stale_bar(&mut view);
        let order = OutfitOrder {
            direction: if sell {
                Direction::Sell
            } else {
                Direction::Buy
            },
            ..BUY_TANK
        };
        assert_eq!(view.outfit(order), Ok(()));
        assert_eq!(view.take_mission_texts().len(), 1, "S131's brief");
        view.catch_up();
        view
    }

    #[test]
    fn an_outfit_hooks_s_text_shows_over_the_bar_as_the_hook_left_it() {
        for sell in [false, true] {
            let what = format!("sell: {sell}");
            let mut view = outfit_hooked("H130 G201 S131 H129 M131", sell, false);
            assert_eq!(bar_shown(&view), Some(705), "{what}: at the H130");
            let noted = view.bar_steps.front().expect("staged").bar.reserves;
            assert_eq!(
                view.reserves_shown().fuel.max,
                noted.fuel.max - 100.0,
                "{what}: as at the H130, before G201's tank"
            );
            assert_eq!(port_nav(&view), naming("Earth"), "{what}: at the H130");
            view.close_mission_text();
            assert_eq!(bar_shown(&view), Some(700), "{what}: the H129, once closed");
            assert_eq!(
                view.reserves_shown(),
                view.reserves(),
                "{what}: at the H129"
            );
            assert_eq!(
                port_nav(&view),
                naming("Earth"),
                "{what}: at the H129, before the M"
            );
        }
    }

    /// The order's own draw (@0x5c5a1, @0x5d328) comes after its hook's
    /// texts, not under them. The text's close redraws the whole bar
    /// first (`_RefreshSAT` @0xa1f13), which draws the nav and so lets go
    /// of the shipyard's mark, so the order's draw then draws no nav.
    #[test]
    fn an_outfit_orders_own_draw_waits_for_its_hooks_texts() {
        for sell in [false, true] {
            for marked in [false, true] {
                let what = format!("sell: {sell}, nav marked: {marked}");
                let mut view = outfit_hooked("S131 M131", sell, marked);
                assert_eq!(retaken(&view), [false; 4], "{what}: held under the text");
                assert_eq!(port_nav(&view), naming("Stale"), "{what}");
                assert_eq!(view.nav_marked, marked, "{what}: the mark kept");
                view.close_mission_text();
                assert_eq!(
                    port_nav(&view),
                    naming("Earth"),
                    "{what}: the text's bar, before the M"
                );
                assert!(!view.nav_marked, "{what}: its redraw drew the nav");
                assert_eq!(view.radar_scene, Some(view.radar_now()), "{what}");
            }
        }
    }

    #[test]
    fn a_ship_hooks_s_text_shows_before_the_purchases_own_redraw() {
        for retire in [false, true] {
            let what = format!("retire: {retire}");
            let mut view = docked_scripted(|catalog| {
                for ship in &mut catalog.ships {
                    if retire && ship.id == ShipId(129) {
                        ship.on_retire = nova_sim::Script::parse("S131");
                    }
                    if !retire && ship.id == ShipId(130) {
                        ship.on_purchase = nova_sim::Script::parse("S131");
                    }
                }
            });
            if retire {
                let mut desk = view.pilot_desk().expect("flying");
                desk.edit(PilotEdit::Credits(100_000)).expect("set");
                view.buy_ship(ShipId(129), "First").expect("bought");
                assert_eq!(bar_shown(&view), Some(700), "{what}: 129's is 128's");
            }
            desk_reserve(&mut view, nova_sim::Reserve::Fuel, 1.0);
            view.refresh_status();
            let before = view.reserves();
            assert!(before.fuel.now < before.fuel.max, "{what}: not full");
            view.buy_ship(ShipId(130), "Kestrel").expect("bought");
            assert_eq!(view.take_mission_texts().len(), 1, "{what}: S131's brief");
            view.catch_up();
            assert_eq!(bar_shown(&view), Some(700), "{what}: the old graphic");
            assert_eq!(view.reserves_shown(), before, "{what}: before the purchase");
            view.close_mission_text();
            assert_eq!(bar_shown(&view), Some(705), "{what}: the new ship's");
            let filled = view.reserves();
            assert_eq!(filled.fuel.now, filled.fuel.max, "{what}: filled");
            assert_eq!(view.reserves_shown(), filled, "{what}: filled, once closed");
        }
    }

    #[test]
    fn under_the_bible_cadence_the_graphic_follows_at_once() {
        for (what, mut view) in texts_before_an_h(RuleSource::Bible) {
            assert_eq!(bar_shown(&view), Some(705), "{what}: at once");
            view.close_mission_text();
            assert_eq!(bar_shown(&view), Some(705), "{what}");
        }
    }

    #[test]
    fn a_comp_text_shows_the_gauges_from_before_on_success() {
        for (script, at_h) in [("G201 M131", false), ("H130 G201 M131", true)] {
            let (landed, mut view) = relanded(script, "", false);
            view.settle_script();
            assert_eq!(view.take_mission_texts().len(), 1, "{script}");
            assert_eq!(view.reserves_shown(), landed, "{script}: the landing's");
            assert_eq!(port_nav(&view), naming("Earth"), "{script}");
            view.close_mission_text();
            let shown = view.reserves_shown();
            if at_h {
                assert_eq!(
                    shown.fuel.max,
                    view.reserves().fuel.max - 100.0,
                    "{script}: as at the H"
                );
            } else {
                assert_eq!(shown, landed, "{script}: as at the text");
            }
            assert_eq!(port_nav(&view), naming("Earth"), "{script}: before the M");
            view.refresh_status();
            assert_eq!(retaken(&view), WHOLE_BAR, "{script}: the next redraw");
            assert_eq!(port_nav(&view), naming("Proxima"), "{script}");
        }
    }

    #[test]
    fn a_fail_text_shows_over_the_bar_as_on_failure_left_it() {
        for (h, changes) in INTF_ROWS {
            let (landed, mut view) = relanded("", &format!("{h} G201 M131"), true);
            view.settle_script();
            assert_eq!(view.take_mission_texts().len(), 1, "{h}: the fail text");
            let shown = view.reserves_shown();
            if changes {
                assert_eq!(
                    shown.fuel.max,
                    view.reserves().fuel.max - 100.0,
                    "{h}: as at the H"
                );
            } else {
                assert_eq!(shown, landed, "{h}: the landing's");
            }
            assert_eq!(port_nav(&view), naming("Earth"), "{h}");
            view.close_mission_text();
            assert_eq!(retaken(&view), WHOLE_BAR, "{h}: as the text noted it");
            assert_eq!(port_nav(&view), naming("Proxima"), "{h}");
        }
    }

    #[test]
    fn a_landings_two_texts_each_close_to_their_own_bar_and_the_loads_before_the_next() {
        let mut catalog = port();
        catalog.missions = vec![
            MissionRecord {
                travel_stel: 128,
                on_success: nova_sim::Script::parse("H130 G201"),
                ..errand(130)
            },
            MissionRecord {
                travel_stel: 128,
                on_success: nova_sim::Script::parse("M131 H129"),
                ..errand(131)
            },
        ];
        catalog.mission_texts = vec![
            MissionTexts {
                comp: Some("A done.".to_owned()),
                ..MissionTexts::named(MissionId(130), "A")
            },
            MissionTexts {
                comp: Some("B done.".to_owned()),
                ..MissionTexts::named(MissionId(131), "B")
            },
        ];
        let mut view = docked_at_earth(catalog, RuleSource::Engine);
        run_set(&mut view, "S130 S131");
        view.take_off().expect("takes off");
        view.settle_script();
        land_now(&mut view);
        let landed = view.reserves_shown();
        view.settle_script();
        assert_eq!(
            view.session().map(Session::ship),
            Ok(ShipId(129)),
            "both OnSuccess ran"
        );
        assert_eq!(
            view.take_mission_texts().len(),
            2,
            "A's CompText, then B's after A's OnSuccess"
        );
        assert_eq!(view.reserves_shown(), landed, "the landing's");
        assert_eq!(landed.fuel.max, 250.0, "the landing's ship");
        assert_eq!(port_nav(&view), naming("Earth"));
        view.close_mission_text();
        assert_eq!(
            view.reserves_shown().fuel.max,
            350.0,
            "A's text as it noted the bar, then A's H130 as at its operator, \
             before G201's tank"
        );
        assert_eq!(
            port_nav(&view),
            naming("Earth"),
            "B's M and H come after B's text, and no dialog redraws"
        );
        view.close_mission_text();
        assert_eq!(
            retaken(&view),
            WHOLE_BAR,
            "B's text, then B's H129 after its M"
        );
        assert_eq!(port_nav(&view), naming("Proxima"));
        assert!(view.bar_steps.is_empty());
    }

    #[test]
    fn an_accepts_two_texts_close_to_their_own_bars_and_the_dialog_redraws_after_the_last() {
        let mut view = docked_scripted(|catalog| {
            let mission = &mut catalog.missions[1];
            mission.cargo_type = 0;
            mission.cargo_qty = 1;
            mission.pickup_mode = 0;
            mission.on_accept = nova_sim::Script::parse("M131");
            catalog.mission_texts[0].load_cargo = Some("Loaded.".to_owned());
        });
        assert_eq!(view.accept_mission(MissionId(131)), Ok(0));
        let queued = view.reserves();
        assert_eq!(
            view.take_mission_texts().len(),
            2,
            "the BriefText, then the LoadCargText"
        );
        desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
        view.close_mission_text();
        assert_eq!(
            view.reserves_shown(),
            queued,
            "the brief as it noted the bar, and no redraw of the dialog's yet"
        );
        assert_eq!(port_nav(&view), naming("Proxima"), "OnAccept's M");
        view.close_mission_text();
        assert_eq!(
            retaken(&view),
            WHOLE_BAR,
            "the load's text, then the computer's redraw"
        );
        assert_eq!(view.reserves_shown().fuel.now, 100.0);
        assert!(view.bar_steps.is_empty());
    }

    #[test]
    fn leaving_the_dock_drops_the_texts_pending_redraws() {
        let mut view = docked_scripted(|catalog| {
            catalog.missions[1].on_accept = nova_sim::Script::parse("M131");
        });
        assert_eq!(view.accept_mission(MissionId(131)), Ok(0));
        assert_eq!(view.take_mission_texts().len(), 1);
        assert_eq!(view.bar_steps.len(), 1, "the brief's close");
        view.take_off().expect("takes off");
        assert!(view.bar_steps.is_empty(), "the take-off's redraw");
    }

    // The gauges while docked.

    /// Sets `reserve` through the pilot desk, which is no redraw point.
    fn desk_reserve(view: &mut View, reserve: nova_sim::Reserve, value: f32) {
        let mut desk = view.pilot_desk().expect("flying");
        desk.edit(PilotEdit::Reserve(reserve, value)).expect("set");
    }

    /// The fuel gauge's full-colour fill the HUD draws, if any.
    fn fuel_drawn(view: &View) -> Option<DrawCommand> {
        drawn(view)
            .iter()
            .find(|c| matches!(c, DrawCommand::Line { color, .. } if *color == layout().fuel_full))
            .cloned()
    }

    #[test]
    fn by_the_engine_the_gauges_hold_while_docked_until_a_shop_closes() {
        let mut view = FlightView::new(outfitting());
        land_now(&mut view);
        let landed = fuel_drawn(&view);
        assert!(landed.is_some());
        let held = view.reserves();
        desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(view.reserves().fuel.max, 350.0, "the tank");
        assert_eq!(view.reserves_shown(), held, "landing's");
        assert_eq!(fuel_drawn(&view), landed, "drawn held");
        view.refresh_status();
        assert_eq!(view.reserves_shown(), view.reserves(), "the shop closed");
        assert_ne!(fuel_drawn(&view), landed);
    }

    #[test]
    fn by_the_other_reading_the_gauges_follow_at_once_while_docked() {
        let rules = Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Bible);
        let mut view = FlightView::new(outfitting()).with_rules(rules);
        land_now(&mut view);
        let landed = fuel_drawn(&view);
        desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(view.reserves_shown(), view.reserves(), "at once");
        assert_ne!(fuel_drawn(&view), landed);
    }

    #[test]
    fn a_recharge_redraws_the_fuel_gauge_alone() {
        use nova_sim::Reserve;
        let mut view = FlightView::new(outfitting());
        land_now(&mut view);
        desk_reserve(&mut view, Reserve::Fuel, 100.0);
        view.refresh_status();
        assert_eq!(view.reserves_shown().fuel.now, 100.0, "held at 100");
        desk_reserve(&mut view, Reserve::Shield, 10.0);
        assert_eq!(view.reserves_shown().shield.now, 40.0, "held full");
        assert!(view.recharge().is_ok());
        assert_eq!(view.reserves().fuel.now, 250.0);
        assert_eq!(view.reserves_shown().fuel.now, 250.0, "redrawn at once");
        assert_eq!(view.reserves_shown().shield.now, 40.0, "still held");
        desk_reserve(&mut view, Reserve::Fuel, 100.0);
        let mut desk = view.pilot_desk().expect("flying");
        desk.edit(PilotEdit::Credits(0)).expect("set");
        assert!(view.recharge().is_err(), "nothing to pay with");
        assert_eq!(
            view.reserves_shown().fuel.now,
            250.0,
            "a refusal redraws nothing"
        );
    }

    #[test]
    fn a_ship_bought_under_the_same_bar_keeps_the_gauges_until_the_shipyard_closes() {
        let mut view = FlightView::new(shipbuying());
        land_now(&mut view);
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert_eq!(view.reserves().fuel.max, 350.0, "its tank");
        assert_eq!(view.reserves_shown().fuel.max, 250.0, "the old ship's");
        view.refresh_status();
        assert_eq!(view.reserves_shown().fuel.max, 350.0, "the shipyard closed");
    }

    #[test]
    fn a_purchase_redraws_the_whole_bar_before_it_buys() {
        let mut view = FlightView::new(shipbuying());
        land_now(&mut view);
        desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
        assert_eq!(view.reserves_shown().fuel.now, 250.0, "held");
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert_eq!(view.reserves().fuel.max, 350.0, "its tank");
        let shown = view.reserves_shown().fuel;
        assert_eq!(
            (shown.now, shown.max),
            (100.0, 250.0),
            "the prompt's close, before the purchase"
        );
    }

    #[test]
    fn a_ship_bought_under_another_bar_shows_its_gauges_at_once() {
        let mut view = FlightView::new(governed(shipbuying(), 129, 129));
        land_now(&mut view);
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert_eq!(bar_shown(&view), Some(705), "ïntf 129");
        assert_eq!(view.reserves_shown().fuel.max, 350.0, "at once");
    }

    // An outfitter order whose script changes the ship's `ïntf`.

    /// [`outfitting`], where ship 129 is under gövt 150, whose
    /// `Interface` is `interface` ([`governed`]), and the fuel tank's
    /// `OnPurchase` is `on_purchase` and its `OnSell` `on_sell`.
    fn upgrading(interface: i16, on_purchase: &str, on_sell: &str) -> FakeCatalog {
        let mut catalog = governed(outfitting(), 129, interface);
        catalog.outfits[0].on_purchase = nova_sim::Script::parse(on_purchase);
        catalog.outfits[0].on_sell = nova_sim::Script::parse(on_sell);
        catalog
    }

    /// Holds a stale snapshot of each of the status bar's parts: no
    /// gauges, nav naming "Stale", a target panel for "Stale", and a
    /// radar with the player off its centre.
    fn stale_bar(view: &mut View) {
        view.reserves_scene = Some(Reserves::default());
        view.nav_scene = Some((
            view.nav_mode(),
            NavDisplay::Stellar(Some("Stale".to_owned())),
        ));
        view.target_scene = Some((
            None,
            Some(TargetPanel {
                name: "Stale".to_owned(),
                subtitle: String::new(),
                picture: None,
                status: nova_sim::TargetStatus::Hidden,
                tag: None,
            }),
        ));
        view.radar_scene = Some(RadarScene {
            player: at(-1.0, -1.0),
            ..RadarScene::default()
        });
        assert_eq!(retaken(view), [false; 4], "stale");
    }

    /// Which of the bar's parts show as they are now: the gauges, the
    /// nav area, the target panel and the radar.
    fn retaken(view: &View) -> [bool; 4] {
        [
            view.reserves_shown() == view.reserves(),
            view.nav_shown() == view.nav_now(),
            *view.target_shown() == view.target_now(),
            view.radar_scene == Some(view.radar_now()),
        ]
    }

    /// The whole bar redrawn, or the radar alone.
    const WHOLE_BAR: [bool; 4] = [true; 4];
    const RADAR_ALONE: [bool; 4] = [false, false, false, true];

    #[test]
    fn an_outfit_bought_that_changes_the_intf_redraws_the_whole_bar_at_once() {
        for (interface, redrawn) in [(129, WHOLE_BAR), (128, RADAR_ALONE)] {
            let mut view = FlightView::new(upgrading(interface, "H129", ""));
            land_now(&mut view);
            stale_bar(&mut view);
            assert_eq!(view.outfit(BUY_TANK), Ok(()));
            assert_eq!(view.session().map(Session::ship), Ok(ShipId(129)));
            assert_eq!(retaken(&view), redrawn, "ïntf {interface}");
        }
    }

    #[test]
    fn an_outfit_sold_that_changes_the_intf_redraws_the_whole_bar_at_once() {
        let sell = OutfitOrder {
            direction: Direction::Sell,
            ..BUY_TANK
        };
        for (interface, redrawn) in [(129, WHOLE_BAR), (128, RADAR_ALONE)] {
            let mut view = FlightView::new(upgrading(interface, "", "H129"));
            land_now(&mut view);
            assert_eq!(view.outfit(BUY_TANK), Ok(()));
            stale_bar(&mut view);
            assert_eq!(view.outfit(sell), Ok(()));
            assert_eq!(view.session().map(Session::ship), Ok(ShipId(129)));
            assert_eq!(retaken(&view), redrawn, "ïntf {interface}");
        }
    }

    #[test]
    fn every_outfitter_order_redraws_the_whole_bar_on_an_intf_change_and_a_refusal_the_radar() {
        type Order = fn(&mut View) -> bool;
        let orders: [(&str, Order); 3] = [
            ("outfit", |view| view.outfit(BUY_TANK).is_ok()),
            ("outfit_counted", |view| {
                view.outfit_counted(BUY_TANK, 1) == Ok(1)
            }),
            ("outfit_order", |view| {
                view.outfit_order(BUY_TANK, 1).run.done == 1
            }),
        ];
        for (name, order) in orders {
            let mut view = FlightView::new(upgrading(129, "H129", ""));
            land_now(&mut view);
            stale_bar(&mut view);
            assert!(order(&mut view), "{name} went through");
            assert_eq!(retaken(&view), WHOLE_BAR, "{name}");
            let mut view = FlightView::new(upgrading(129, "H129", ""));
            land_now(&mut view);
            let mut desk = view.pilot_desk().expect("flying");
            desk.edit(PilotEdit::Credits(0)).expect("set");
            stale_bar(&mut view);
            assert!(!order(&mut view), "{name} refused");
            assert_eq!(view.session().map(Session::ship), Ok(ShipId(128)));
            assert_eq!(retaken(&view), RADAR_ALONE, "{name} refused");
        }
    }

    #[test]
    fn a_ship_bought_redraws_the_whole_bar_on_an_intf_change_and_all_but_the_gauges_otherwise() {
        for (interface, redrawn) in [(129, WHOLE_BAR), (128, [false, true, true, true])] {
            let mut view = FlightView::new(governed(shipbuying(), 129, interface));
            land_now(&mut view);
            stale_bar(&mut view);
            view.buy_ship(ShipId(129), "Kestrel").expect("bought");
            assert_eq!(retaken(&view), redrawn, "ïntf {interface}");
        }
    }

    #[test]
    fn an_outfit_orders_m_after_its_h_waits_for_the_next_nav_redraw_and_before_it_shows_at_once() {
        for (script, shown) in [("H130 M131", "Earth"), ("M131 H130", "Proxima")] {
            let mut catalog = port();
            catalog.outfits[0].on_purchase = nova_sim::Script::parse(script);
            let mut view = docked_at_earth(catalog, RuleSource::Engine);
            assert_eq!(view.outfit(BUY_TANK), Ok(()));
            assert_eq!(
                view.session().map(Session::nav_mode),
                Ok(NavMode::Stellar(Some(StellarId(140)))),
                "{script}: written"
            );
            assert_eq!(port_nav(&view), naming(shown), "{script}");
            view.refresh_status();
            assert_eq!(
                port_nav(&view),
                naming("Proxima"),
                "{script}: the next nav redraw"
            );
        }
    }

    #[test]
    fn an_outfit_order_changing_the_intf_and_back_redraws_the_bar_as_at_the_last_change() {
        let mut catalog = port();
        catalog.outfits[0].on_purchase = nova_sim::Script::parse("M131 H130 H129 G201");
        let mut view = docked_at_earth(catalog, RuleSource::Engine);
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(view.session().map(Session::ship), Ok(ShipId(129)));
        assert_eq!(port_nav(&view), naming("Proxima"), "redrawn at the H");
        assert_eq!(
            view.reserves_shown().fuel.max,
            view.reserves().fuel.max - 100.0,
            "the gauges as at H129, before G201's tank"
        );
    }

    #[test]
    fn a_ship_purchases_on_purchase_intf_change_shows_the_gauges_before_the_fill() {
        let mut catalog = port();
        assert_eq!(catalog.ships[0].id, ShipId(129));
        catalog.ships[0].on_purchase = nova_sim::Script::parse("H130");
        let mut view = docked_at_earth(catalog, RuleSource::Engine);
        desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert_eq!(view.session().map(Session::ship), Ok(ShipId(130)));
        let (shown, now) = (view.reserves_shown().fuel, view.reserves().fuel);
        assert_eq!(shown.max, now.max, "ship 130's, under its ïntf");
        assert!(shown.now < shown.max, "{shown:?}: not yet filled");
        assert_eq!(now.now, now.max, "the fill");
        assert_eq!(view.nav_shown(), view.nav_now(), "the purchase's own draw");
    }

    #[test]
    fn a_ship_purchases_on_purchase_m_shows_in_its_nav_draw_over_the_gauges_before_the_fill() {
        let mut catalog = port();
        assert_eq!(catalog.ships[0].id, ShipId(129));
        catalog.ships[0].on_purchase = nova_sim::Script::parse("H130 M131");
        let mut view = docked_at_earth(catalog, RuleSource::Engine);
        desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert_eq!(view.session().map(Session::ship), Ok(ShipId(130)));
        assert_eq!(
            port_nav(&view),
            naming("Proxima"),
            "the purchase's own nav draw (@0x5f0b1)"
        );
        let (shown, now) = (view.reserves_shown().fuel, view.reserves().fuel);
        assert_eq!(shown.max, now.max, "ship 130's, under its ïntf");
        assert!(shown.now < shown.max, "{shown:?}: not yet filled");
        assert_eq!(now.now, now.max, "the fill");
    }

    // The take-off's redraw before its deselect.

    /// The nav area naming `name` in mode 2, as flight shows it.
    fn flashing(name: &str) -> NavDisplay {
        NavDisplay::Stellar(Some(name.to_owned()))
    }

    #[test]
    fn the_take_off_after_a_landed_m_names_its_stellar_for_one_frame() {
        type TakeOff = fn(&mut View);
        let take_offs: [(&str, TakeOff); 2] = [
            ("the take-off", |view| {
                run_set(view, "M131");
                view.settle_script();
                assert_eq!(view.take_off(), Some(StellarId(128)));
            }),
            ("a Q", |view| {
                run_set(view, "M131 Q25048");
                view.settle_script();
                assert_eq!(view.leave_if_due(), Some(StellarId(128)));
            }),
        ];
        for (what, take_off) in take_offs {
            let mut view = told(docked_at_earth(port(), RuleSource::Engine));
            take_off(&mut view);
            assert_eq!(
                view.session().map(Session::nav_mode),
                Ok(NavMode::Stellar(None)),
                "{what}: let go"
            );
            assert_eq!(view.nav_shown(), flashing("Proxima"), "{what}: held");
            view.tick(Duration::from_millis(1));
            assert_eq!(
                view.nav_shown(),
                flashing("Proxima"),
                "{what}: a frame within the tick"
            );
            view.tick(TICK);
            assert_eq!(
                view.nav_shown(),
                NavDisplay::Stellar(None),
                "{what}: a tick on"
            );
            let mut view = told(docked_at_earth(port(), RuleSource::Bible));
            take_off(&mut view);
            assert_eq!(
                view.nav_shown(),
                NavDisplay::Stellar(None),
                "{what}: by the bible"
            );
        }
    }

    #[test]
    fn the_take_offs_held_nav_shows_in_the_first_frame_drawn_after_it_whatever_the_rate() {
        for fps in [120, 60, 30, 20] {
            let frame = Duration::from_secs(1) / fps;
            let mut view = told(docked_at_earth(port(), RuleSource::Engine));
            run_set(&mut view, "M131");
            view.settle_script();
            view.elapsed = Duration::from_nanos(333_333_329); // ten ticks less 1 ns
            assert_eq!(
                crate::time::ticks(view.elapsed + Duration::from_nanos(1)),
                10
            );
            assert_eq!(view.take_off(), Some(StellarId(128)));
            view.tick(frame);
            assert_eq!(
                view.nav_shown(),
                flashing("Proxima"),
                "{fps} fps: the first frame drawn, though it crosses a tick"
            );
            view.tick(TICK);
            assert_eq!(
                view.nav_shown(),
                NavDisplay::Stellar(None),
                "{fps} fps: a tick on"
            );
        }
    }

    #[test]
    fn a_take_off_with_nothing_let_go_holds_nothing() {
        let mut view = docked_at_earth(port(), RuleSource::Engine);
        run_set(&mut view, "M132");
        view.settle_script();
        assert_eq!(view.take_off(), Some(StellarId(128)));
        assert_eq!(view.nav_shown(), NavDisplay::Stellar(None));
        assert_eq!(view.nav_flash, None, "no flash to end");
    }

    #[test]
    fn a_plain_take_off_names_the_landed_stellar_for_one_frame() {
        let mut view = docked_at_earth(port(), RuleSource::Engine);
        assert_eq!(view.take_off(), Some(StellarId(128)));
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Stellar(None)),
            "let go"
        );
        assert_eq!(view.nav_shown(), flashing("Earth"), "held");
        view.tick(TICK);
        assert_eq!(view.nav_shown(), flashing("Earth"), "the first frame");
        view.tick(TICK);
        assert_eq!(view.nav_shown(), NavDisplay::Stellar(None), "a tick on");
        let mut view = docked_at_earth(port(), RuleSource::Bible);
        view.take_off();
        assert_eq!(view.nav_shown(), NavDisplay::Stellar(None), "by the bible");
    }

    #[test]
    fn a_take_off_with_a_course_names_the_landed_stellar_over_the_cue_for_one_frame() {
        let rules = Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Engine);
        let mut view = FlightView::new(port()).with_rules(rules);
        view.session
            .as_mut()
            .expect("flying")
            .plot_course(SystemId(131))
            .expect("a route");
        tap(&mut view, HYPERSPACE_MODE_KEY);
        land_now(&mut view);
        view.settle_script();
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Stellar(Some(StellarId(128)))),
            "the land key put mode 2 over the cue"
        );
        assert_eq!(view.take_off(), Some(StellarId(128)));
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Hyperspace(Some(SystemId(131)))),
            "cued"
        );
        assert_eq!(view.nav_shown(), flashing("Earth"), "held over the cue");
        view.tick(TICK);
        assert_eq!(view.nav_shown(), flashing("Earth"), "the first frame");
        view.tick(TICK);
        assert!(
            matches!(view.nav_shown(), NavDisplay::Hyperspace { .. }),
            "a tick on: {:?}",
            view.nav_shown()
        );
    }

    #[test]
    fn a_text_queued_in_the_take_offs_days_shows_over_its_held_nav() {
        let mut catalog = port();
        catalog.crons = vec![nova_sim::catalog::CronRecord {
            id: nova_sim::catalog::CronId(128),
            first_day: 0,
            first_month: 0,
            first_year: 0,
            last_day: 0,
            last_month: 0,
            last_year: 0,
            random: 100,
            duration: 0,
            pre_holdoff: 0,
            post_holdoff: 0,
            flags: 0,
            enable_on: Test::default(),
            on_start: nova_sim::Script::parse("S131"),
            on_end: nova_sim::Script::default(),
            contribute: 0,
            require: 0,
            news_govt: [-1; 4],
            govt_news: [-1; 4],
            ind_news: -1,
        }];
        let mut view = docked_at_earth(catalog, RuleSource::Engine);
        run_set(&mut view, "M131");
        view.settle_script();
        assert_eq!(view.take_off(), Some(StellarId(128)));
        assert_eq!(view.take_mission_texts().len(), 1, "the cron's mission");
        assert_eq!(view.nav_shown(), flashing("Proxima"), "under its text");
        view.tick(TICK);
        assert_eq!(view.nav_shown(), NavDisplay::Stellar(None));
    }

    #[test]
    fn a_pilot_resumed_and_landed_holds_the_gauges_from_its_landing() {
        let pilot = {
            let mut view = FlightView::new(outfitting());
            land_now(&mut view);
            view.pilot().expect("flying").clone()
        };
        let mut view = FlightView::with_pilot(outfitting(), pilot);
        assert_eq!(view.take_landing(), None, "in flight");
        land_now(&mut view);
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(view.reserves().fuel.max, 350.0, "the tank");
        assert_eq!(view.reserves_shown().fuel.max, 250.0, "the landing's");
    }

    #[test]
    fn landing_redraws_the_held_gauges() {
        let mut view = FlightView::new(outfitting());
        view.reserves_scene = Some(Reserves::default());
        desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
        land_now(&mut view);
        assert_eq!(view.reserves_shown(), view.reserves(), "landing's");
        assert_eq!(view.reserves_shown().fuel.now, 100.0);
    }

    #[test]
    fn in_flight_the_gauges_stay_live_by_the_engine() {
        let mut view = flight();
        assert!(view.hud_waits());
        view.redraw_status();
        let before = fuel_drawn(&view);
        desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
        assert_eq!(view.reserves_shown().fuel.now, 100.0, "at once");
        assert_ne!(fuel_drawn(&view), before);
    }

    /// The first `chär`'s `OnStart` is `on_start`.
    fn starting_with(on_start: &str) -> FakeCatalog {
        let base = catalog();
        FakeCatalog {
            character: Ok(CharacterStart {
                on_start: nova_sim::Script::parse(on_start),
                ..base.character.clone().expect("a chär")
            }),
            ..base
        }
    }

    #[test]
    fn beginning_runs_on_start_once_and_the_map_shows_what_it_explored() {
        let catalog = starting_with("R(b5 b6) ^b7 X131");
        let pilot = Pilot::new(&catalog, "Ada").expect("starts");
        let mut view = FlightView::with_pilot(catalog, pilot).with_chance(rolling_one());
        assert!(!bit_set(&view, 7), "flying runs nothing");
        view.begin();
        assert!(bit_set(&view, 5), "on the flight's chance");
        assert!(bit_set(&view, 7), "once");
        let explored = view
            .course_map()
            .explored()
            .map(|set| set.iter().copied().collect::<Vec<_>>());
        assert_eq!(explored, Some(vec![SystemId(130), SystemId(131)]));
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..starting_with("b5")
        };
        let mut broken = FlightView::new(broken);
        broken.begin();
        assert_eq!(broken.pilot(), None);
    }

    #[test]
    fn resuming_faces_the_heading_drawn_on_the_flights_chance() {
        let (script, chance) = scripted(&[123]);
        let mut view = FlightView::with_pilot(catalog(), docked_pilot("Ada")).with_chance(chance);
        assert_eq!(player(&view).heading, 0.0, "flying alone faces up");
        view.resume();
        assert_eq!(player(&view).heading, 123.0);
        let asked = &script.borrow().asked;
        assert_eq!(asked[0], 360, "the heading first");
        assert_eq!(
            asked[1..],
            [100; nova_sim::mission::ROLLED],
            "then the offers' roll table, on the same chance"
        );
    }

    #[test]
    fn beginning_draws_the_offer_roll_table_on_the_flights_chance() {
        let (script, chance) = scripted(&[]);
        let catalog = catalog();
        let pilot = Pilot::new(&catalog, "Ada").expect("starts");
        let mut view = FlightView::with_pilot(catalog, pilot).with_chance(chance);
        view.begin();
        let asked = &script.borrow().asked;
        assert!(asked.len() >= nova_sim::mission::ROLLED);
        assert_eq!(
            asked[asked.len() - nova_sim::mission::ROLLED..],
            [100; nova_sim::mission::ROLLED]
        );
    }

    #[test]
    fn beginning_a_new_pilot_leaves_it_facing_up() {
        let (_, chance) = scripted(&[123]);
        let catalog = catalog();
        let pilot = Pilot::new(&catalog, "Ada").expect("starts");
        let mut view = FlightView::with_pilot(catalog, pilot).with_chance(chance);
        view.begin();
        assert_eq!(player(&view).heading, 0.0);
    }

    #[test]
    fn a_failed_flight_resumes_nothing() {
        let (script, chance) = scripted(&[123]);
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        let mut broken = FlightView::new(broken).with_chance(chance);
        broken.resume();
        assert_eq!(broken.pilot(), None);
        assert!(script.borrow().asked.is_empty(), "nothing drawn");
    }
    // Traffic.

    use std::collections::VecDeque;

    use nova_sim::{Goal, NpcId};

    /// Draws its script, then the last outcome, so no roll fires;
    /// records each `n` asked.
    #[derive(Default)]
    struct Script {
        draws: VecDeque<u32>,
        asked: Vec<u32>,
    }

    impl Chance for Script {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked.push(n);
            self.draws.pop_front().unwrap_or(n - 1)
        }
    }

    fn scripted(draws: &[u32]) -> (Rc<RefCell<Script>>, SharedChance) {
        let script = Rc::new(RefCell::new(Script {
            draws: draws.iter().copied().collect(),
            asked: Vec::new(),
        }));
        let shared: Rc<RefCell<dyn Chance>> = script.clone();
        (script, SharedChance::new(shared))
    }

    /// One setup pass placing the düde's ship at (`x` - 750, `y` - 750)
    /// facing `heading`, of aggression 0.
    fn placed(x: u32, y: u32, heading: u32) -> [u32; 8] {
        [6, 6, 0, 0, x, y, heading, 2]
    }

    /// Decides nothing: every NPC idles.
    #[derive(Debug)]
    struct Still;

    impl Behaviour for Still {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Idle
        }
    }

    /// [`catalog`] with `avg` ships on average in each of `systems`, all of
    /// düde 128's ship `ship` with AI `ai_type`; ships 129 and 130 are
    /// records the traffic can fly.
    fn trafficked(systems: &[i16], avg: i16, ship: i16, ai_type: i16) -> FakeCatalog {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        let record = |id| ShipRecord {
            id: ShipId(id),
            name: format!("Ship {id}"),
            short_name: String::new(),
            long_name: String::new(),
            fields: ShipFields {
                speed: 600,
                ..FIELDS
            },
            defaults: Vec::new(),
            cost: 1,
            tech_level: 1,
            buy_random: 100,
            hire_random: 0,
            require: 0,
            availability: nova_sim::Test::default(),
            appear_on: nova_sim::Test::default(),
            flags: 0,
            flags3: 0,
            disp_weight: 0,
            max_gun: 0,
            max_tur: 0,
            length: 0,
            crew: 0,
            inherent_ai: 1,
            comm_name: String::new(),
            inherent_govt: None,
            combat_govt: None,
            escort_type: -1,
            on_capture: nova_sim::Script::default(),
            on_purchase: nova_sim::Script::default(),
            on_retire: nova_sim::Script::default(),
        };
        FakeCatalog {
            traffic: systems
                .iter()
                .map(|&id| {
                    (
                        SystemId(id),
                        nova_sim::SystemTraffic {
                            dude_types,
                            avg_ships: avg,
                            persons: Default::default(),
                        },
                    )
                })
                .collect(),
            dudes: vec![(
                nova_sim::DudeId(128),
                nova_sim::DudeRecord {
                    ai_type,
                    govt: None,
                    ships: vec![(ShipId(ship), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )],
            ships: vec![record(129), record(130)],
            ..catalog()
        }
    }

    fn npc_ids(view: &View) -> Vec<NpcId> {
        let session = view.session().expect("flying");
        session.npcs().iter().map(|npc| npc.id).collect()
    }

    fn npc_states(view: &View) -> Vec<ShipState> {
        let session = view.session().expect("flying");
        session.npcs().iter().map(|npc| npc.state).collect()
    }

    #[test]
    fn the_traffic_is_populated_when_the_flight_starts() {
        let (script, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        assert_eq!(npc_ids(&view), [], "not until the first tick");
        view.tick(TICK);
        let session = view.session().expect("flying");
        let npc = &session.npcs()[0];
        assert_eq!((npc.ship, npc.goal), (ShipId(129), Goal::Idle));
        assert_eq!(npc.state.position, Vec2::new(100.0, -100.0));
        assert_eq!(&script.borrow().asked[..7], [7, 7, 100, 1, 1500, 1500, 360]);
        ticks(&mut view, 3);
        assert_eq!(npc_ids(&view), [NpcId(0)], "populated once");
        let npc = &view.session().expect("flying").npcs()[0];
        assert_eq!(npc.goal, Goal::Idle, "deciding as the behaviour given says");
        assert_eq!(
            npc.state.position,
            Vec2::new(118.0, -100.0),
            "gliding right at its top speed, 6 a tick"
        );
    }

    #[test]
    fn an_npc_is_drawn_with_its_own_sprite_where_it_is_facing_its_heading() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let list = drawn(&view);
        let frame = rotation_frame(90.0, NonZeroU16::new(72).expect("non-zero"));
        assert_eq!(
            sprites(&list),
            [
                (ImageKey::sprite(1128, 0), at(512.0, -216.0)),
                (ImageKey::sprite(1129, 1), at(812.0, 184.0)),
                (ImageKey::sprite(2001, frame), at(612.0, 284.0)),
                (ImageKey::sprite(2000, 0), VIEW_CENTER),
            ],
            "after the stellars, before the player"
        );
    }

    #[test]
    fn a_ship_waiting_at_its_gate_is_drawn_by_its_tint_and_any_ship_by_its_cloak() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let session = view.session().expect("flying");
        let mut npc = session.npcs()[0].clone();
        let opaque = CloakShown::Opaque;
        assert_eq!(session.cloak_shown(&npc), opaque, "flying");
        npc.mode = nova_sim::traffic::npc::Mode::JumpingIn { tick: 0 };
        assert_eq!(session.cloak_shown(&npc), opaque, "jumping in");
        npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left: 3 };
        assert_eq!(session.cloak_shown(&npc), opaque, "fading in at its gate");
        npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left: 30 };
        assert_eq!(session.cloak_shown(&npc), opaque);
        assert_eq!(npc_tint(&npc), None, "hidden at its gate");
        // Held there by the player's hit, its goal is 4, not 0x15, so
        // `_HandleShipDisplay`'s fade (@0x2c090) passes it by.
        npc.mode = nova_sim::traffic::npc::Mode::HeldAtGate { ticks_left: 3 };
        assert_eq!(npc_tint(&npc), Some(Color::WHITE), "held: drawn plain");
        assert_eq!(session.cloak_shown(&npc), opaque, "held");
        npc.cloak = CLOAKED;
        npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left: 3 };
        assert_eq!(
            session.cloak_shown(&npc),
            CloakShown::Hidden,
            "cloaked at its gate"
        );
    }

    /// A full cloak.
    const CLOAKED: nova_sim::cloak::Cloak = nova_sim::cloak::Cloak {
        fade: nova_sim::cloak::FADE_FULL,
        direction: 0,
    };

    #[test]
    fn a_ship_the_player_cannot_see_for_its_cloak_is_not_drawn() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let session = view.session().expect("flying");
        let mut npc = session.npcs()[0].clone();
        npc.cloak = CLOAKED;
        assert_eq!(session.cloak_shown(&npc), CloakShown::Hidden, "cloaked");
        npc.cloak.direction = -1;
        npc.cloak.fade = 8.0;
        assert_eq!(
            session.cloak_shown(&npc),
            CloakShown::Opaque,
            "faded out enough to be seen"
        );
    }

    #[test]
    fn a_cloaked_ship_is_on_the_radar_only_when_the_session_says() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let session = view.session().expect("flying");
        let npc = session.npcs()[0].clone();
        let blip = RadarShip {
            position: at(100.0, -100.0),
            targeted: false,
            iff: ShipIff::Other,
            heavy: false,
        };
        let at_blip = at(100.0, -100.0);
        assert_eq!(radar_ships(session, &[(&npc, at_blip, 0.0)]), [blip]);
        let mut cloaked = npc.clone();
        cloaked.cloak = CLOAKED;
        assert_eq!(radar_ships(session, &[(&cloaked, at_blip, 0.0)]), []);
        cloaked.cloak_gear.on_radar = true;
        assert_eq!(radar_ships(session, &[(&cloaked, at_blip, 0.0)]), [blip]);
    }

    #[test]
    fn the_play_area_is_the_screen_left_of_the_status_bar_about_the_ship() {
        let area = play_area();
        assert_eq!(
            area,
            nova_sim::cloak::Area {
                left: -512.0,
                top: -384.0,
                right: 1024.0 - 194.0 - 512.0,
                bottom: 384.0,
            }
        );
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        assert_eq!(view.session().expect("flying").play_area(), area, "told");
    }

    #[test]
    fn an_npc_is_a_dim_blip_on_the_radar() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        // (100, -100) is 3.125 pixels each way: (929.125, 92.875).
        let blip = RadarMark::Pixel {
            x: 929,
            y: 92,
            color: RadarColor::Dim,
        };
        assert_eq!(blips(&view), [blip]);
        let list = drawn(&view);
        assert!(
            list.iter().any(|command| *command
                == DrawCommand::Dot {
                    center: at(929.5, 92.5),
                    size: 1.0,
                    color: layout().dim_radar,
                }),
            "{list:?}"
        );
    }

    /// The tints the NPC sprite (ship 129's sheet, 2001) is drawn with.
    fn npc_tints(list: &DrawList) -> Vec<Color> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { image, tint, .. } if image.id == 2001 => Some(*tint),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn an_npc_coming_out_of_a_gate_is_hidden_then_fades_in_but_always_on_the_radar() {
        let mut catalog = trafficked(&[130], 1, 129, 3);
        catalog.sites.push(LandingSite {
            flags2: nova_sim::gate::HYPERGATE,
            exit_angle: 90,
            ..site(141, (200.0, 100.0), StellarFlags::CAN_LAND)
        });
        // Setup brings nobody (an empty person roll); then an arrival
        // comes out of the gate, the third of the stellars.
        let (script, chance) = scripted(&[0, 0, 6, 6, 0, 0, 0, 2]);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        ticks(&mut view, 2);
        assert_eq!(&script.borrow().asked[1..9], [500, 7, 7, 100, 1, 360, 3, 3]);
        let mode = |view: &View| view.session().expect("flying").npcs()[0].mode;
        assert_eq!(mode(&view), Mode::Emerging { ticks_left: 59 });
        ticks(&mut view, 19);
        assert_eq!(mode(&view), Mode::Emerging { ticks_left: 40 });
        let list = drawn(&view);
        assert_eq!(npc_tints(&list), [], "hidden");
        assert_eq!(blips(&view).len(), 1, "on the radar all the same");
        ticks(&mut view, 32);
        assert_eq!(mode(&view), Mode::Emerging { ticks_left: 8 });
        let list = drawn(&view);
        assert_eq!(
            npc_tints(&list),
            [Color::rgba(255, 255, 255, 128)],
            "half in"
        );
        assert_eq!(blips(&view).len(), 1);
        ticks(&mut view, 8);
        assert_eq!(mode(&view), Mode::LeavingGate { tick: 0 });
        assert_eq!(npc_tints(&drawn(&view)), [Color::WHITE]);
    }

    /// Idles, and answers a hit on itself by taking its attacker.
    #[derive(Debug)]
    struct Answering;

    impl Behaviour for Answering {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Idle
        }

        fn react(
            &self,
            npc: &nova_sim::Npc,
            strike: &nova_sim::Strike,
            _around: &nova_sim::Surroundings,
        ) -> nova_sim::Reaction {
            if strike.ship == nova_sim::ShipRef::Npc(npc.id) {
                nova_sim::Reaction {
                    target: strike.by,
                    provoked: strike.damage,
                    goal: None,
                }
            } else {
                nova_sim::Reaction::default()
            }
        }
    }

    #[test]
    fn a_ship_the_player_hits_in_its_gate_is_drawn_on_the_frame_of_the_hit() {
        // The original answers the hit inside `_HandleShot` (@0x44385),
        // before the display pass draws the frame.
        let mut catalog = armed(1, &[BLASTER]);
        for weapon in &mut catalog.weapons {
            weapon.energy_dmg = 5;
        }
        catalog.sites.push(LandingSite {
            flags2: nova_sim::gate::HYPERGATE,
            exit_angle: 90,
            ..site(141, (0.0, -100.0), StellarFlags::CAN_LAND)
        });
        // Setup brings nobody; then an arrival comes out of the gate, the
        // third of the stellars, straight ahead of the player.
        let (_, chance) = scripted(&[0, 0, 6, 6, 0, 0, 0, 2]);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Answering));
        ticks(&mut view, 2);
        let npc = |view: &View| view.session().expect("flying").npcs()[0].clone();
        assert!(matches!(npc(&view).mode, Mode::Emerging { .. }));
        let left = |npc: &nova_sim::Npc| npc.reserves.shield.now + npc.reserves.armor.now;
        let mut shield = left(&npc(&view));
        view.input(&key(FIRE_KEY, true));
        for _ in 0..60 {
            view.tick(TICK);
            let hit = npc(&view);
            if left(&hit) < shield {
                assert!(
                    matches!(hit.mode, Mode::HeldAtGate { .. }),
                    "{:?}",
                    hit.mode
                );
                assert_eq!(npc_tints(&drawn(&view)), [Color::WHITE], "seen");
                return;
            }
            assert!(matches!(hit.mode, Mode::Emerging { .. }), "{:?}", hit.mode);
            shield = left(&hit);
        }
        panic!("never hit");
    }

    /// [`armed`] with blasters of 5 energy damage and hypergate 141 at
    /// (0, -110), straight ahead of the player, `with` changes made. A shot
    /// hits on the pass after the one it moves into its target on, and at
    /// this distance that move falls on a frame's advance pass, so the hit
    /// falls on its second.
    fn gate_110_ahead(with: impl FnOnce(&mut FakeCatalog)) -> FakeCatalog {
        let mut catalog = armed(1, &[BLASTER]);
        for weapon in &mut catalog.weapons {
            weapon.energy_dmg = 5;
        }
        catalog.sites.push(LandingSite {
            flags2: nova_sim::gate::HYPERGATE,
            exit_angle: 90,
            ..site(141, (0.0, -110.0), StellarFlags::CAN_LAND)
        });
        with(&mut catalog);
        catalog
    }

    /// `catalog`'s flight in double time, `draws` bringing an arrival out
    /// of gate 141, which answers a hit by taking its attacker; fired at
    /// until the frame on which the first hit lands, which, as a copy of
    /// the session run through each frame's passes shows, is that frame's
    /// second pass. Until then the arrival is hidden in the gate.
    fn hit_on_the_second_pass(catalog: FakeCatalog, draws: &[u32]) -> View {
        let (_, chance) = scripted(draws);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Answering))
            .with_double_time(true);
        ticks(&mut view, 2);
        let npc = |view: &View| view.session().expect("flying").npcs()[0].clone();
        assert!(matches!(npc(&view).mode, Mode::Emerging { .. }));
        let left = |npc: &nova_sim::Npc| npc.reserves.shield.now + npc.reserves.armor.now;
        let shield = left(&npc(&view));
        // Whether a frame from `session`, run on a copy, hits the NPC on
        // its advance pass, and by the end of its second.
        let passes = |session: &Session, catalog: &FakeCatalog| {
            let mut copy = session.clone();
            copy.tick(Controls::default());
            copy.tick_traffic(catalog, &Answering, &mut NeverFires);
            copy.tick_combat(Rules::default(), &mut NeverFires);
            let advance = left(&copy.npcs()[0]) < shield;
            copy.tick_again(Controls::default(), Rules::default(), &mut NeverFires);
            (advance, left(&copy.npcs()[0]) < shield)
        };
        view.input(&key(FIRE_KEY, true));
        for frame in 0..60 {
            let ahead = passes(view.session().expect("flying"), view.catalog());
            view.tick(TICK);
            if left(&npc(&view)) < shield {
                assert_eq!(ahead, (false, true), "frame {frame}: the second pass's hit");
                return view;
            }
            let mode = npc(&view).mode;
            assert!(matches!(mode, Mode::Emerging { .. }), "{mode:?}");
            assert_eq!(npc_tints(&drawn(&view)), [], "hidden");
        }
        panic!("never hit");
    }

    #[test]
    fn a_ship_the_player_hits_in_its_gate_on_the_second_pass_is_drawn_on_the_frame_of_the_hit() {
        // `_HandleShot` (@0x44385) runs on double time's second pass too,
        // with `_DamageShip`'s reaction, so the hit is answered there,
        // before the next frame is drawn. Setup brings nobody; then an
        // arrival comes out of the gate, the third of the stellars.
        let view = hit_on_the_second_pass(gate_110_ahead(|_| ()), &[0, 0, 6, 6, 0, 0, 0, 2]);
        let mode = view.session().expect("flying").npcs()[0].mode;
        assert!(matches!(mode, Mode::HeldAtGate { .. }), "{mode:?}");
        assert_eq!(npc_tints(&drawn(&view)), [Color::WHITE], "seen");
    }

    #[test]
    fn a_person_the_player_hits_in_its_gate_on_the_second_pass_says_its_quote_on_that_frame() {
        // `_DamageShip`'s reaction on the second pass turns it on the
        // player, and `_AIMakeShipAttackPlayer` says its quote there
        // (`_ShowPersonHailQuote` @0x89ce1), before the next frame.
        let catalog = gate_110_ahead(|catalog| {
            let ace = peopled();
            catalog.persons = ace.persons;
            catalog.persons[0].link_syst = -1;
            catalog.persons[0].flags = nova_sim::person::QUOTE_ON_ATTACK;
            catalog.strings = ace.strings;
        });
        // Setup's person roll comes up empty (`përs` 128); then Ace,
        // `përs` 600, comes out of the gate, the third of the stellars.
        let view = hit_on_the_second_pass(catalog, &[0, 0, 0, 0, 472, 0, 2]);
        let npc = &view.session().expect("flying").npcs()[0];
        assert!(
            matches!(npc.mode, Mode::HeldAtGate { .. }),
            "{:?}",
            npc.mode
        );
        assert_eq!(view.message(), Some("Ace: Prepare to die, !"));
    }

    /// `catalog` with hypergate 141 at (200, 100) in Sol, of 6 frames
    /// stepping every tick (`AnimDelay` 0, as stock's), drawing `rlëD` 1141.
    fn with_hypergate(mut catalog: FakeCatalog) -> FakeCatalog {
        catalog.sites.push(LandingSite {
            flags2: nova_sim::gate::HYPERGATE,
            exit_angle: 90,
            animation: nova_sim::catalog::SiteAnimation {
                type_frames: 6,
                ..nova_sim::catalog::SiteAnimation::default()
            },
            ..site(141, (200.0, 100.0), StellarFlags::CAN_LAND)
        });
        catalog.sol_more.push(stellar(141, "Gate", (200, 100), 6));
        catalog
    }

    /// The frames hypergate 141's sheet is drawn on.
    fn gate_frames(list: &DrawList) -> Vec<u16> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { image, .. } if image.id == 1141 => Some(image.frame),
                _ => None,
            })
            .collect()
    }

    /// Sends every NPC into hypergate 141.
    #[derive(Debug)]
    struct Entering;

    impl Behaviour for Entering {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::EnterGate(nova_sim::catalog::StellarId(141))
        }
    }

    #[test]
    fn a_ship_entering_a_gate_fades_out_by_two_32nds_a_tick() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let mut npc = view.session().expect("flying").npcs()[0].clone();
        let gate = nova_sim::catalog::StellarId(141);
        let entering = |ticks_left| Mode::EnteringGate { gate, ticks_left };
        npc.mode = entering(15);
        assert_eq!(
            npc_tint(&npc),
            Some(Color::rgba(255, 255, 255, 239)),
            "30/32"
        );
        npc.mode = entering(8);
        assert_eq!(
            npc_tint(&npc),
            Some(Color::rgba(255, 255, 255, 128)),
            "half"
        );
        npc.mode = entering(1);
        assert_eq!(npc_tint(&npc), Some(Color::rgba(255, 255, 255, 16)), "2/32");
        npc.mode = entering(16);
        assert_eq!(npc_tint(&npc), Some(Color::WHITE));
    }

    #[test]
    fn a_hypergate_animates_while_an_npc_heads_in_to_enter_it_and_the_npc_fades_into_it() {
        let busy = with_hypergate(trafficked(&[130], 1, 129, 3));
        // Setup places the ship at (700, 700), far from the gate.
        let (_, chance) = scripted(&placed(1450, 1450, 90));
        let mut view = FlightView::new(busy)
            .with_chance(chance)
            .with_behaviour(Rc::new(Entering));
        assert_eq!(gate_frames(&drawn(&view)), [0], "closed at first");
        let mut heading_in = Vec::new();
        let mut tints = Vec::new();
        let mut after = Vec::new();
        for _ in 0..2000 {
            ticks(&mut view, 1);
            let list = drawn(&view);
            let frames = gate_frames(&list);
            match view
                .session()
                .expect("flying")
                .npcs()
                .first()
                .map(|npc| npc.mode)
            {
                Some(Mode::Flying) => heading_in.extend(frames),
                Some(Mode::EnteringGate { .. }) => tints.extend(npc_tints(&list)),
                Some(_) => {}
                None => after.extend(frames),
            }
        }
        assert_eq!(heading_in.first(), Some(&0), "closed while it is far");
        assert!(heading_in.iter().any(|&frame| frame > 0), "{heading_in:?}");
        assert_eq!(tints.len(), 15, "drawn for 15 frames: {tints:?}");
        assert_eq!(tints.first(), Some(&Color::rgba(255, 255, 255, 239)));
        assert_eq!(tints.last(), Some(&Color::rgba(255, 255, 255, 16)));
        assert_eq!(after.last(), Some(&0), "closed once it is gone");
    }

    #[test]
    fn a_hypergate_animates_while_an_npc_waits_in_it_and_rests_otherwise() {
        let busy = with_hypergate(trafficked(&[130], 1, 129, 3));
        // As in the test above: an arrival comes out of the gate.
        let (_, chance) = scripted(&[0, 0, 6, 6, 0, 0, 0, 2]);
        let mut view = FlightView::new(busy)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        assert_eq!(gate_frames(&drawn(&view)), [0], "closed at first");
        ticks(&mut view, 2);
        let mode = |view: &View| view.session().expect("flying").npcs()[0].mode;
        assert_eq!(mode(&view), Mode::Emerging { ticks_left: 59 });
        let mut frames = Vec::new();
        while matches!(mode(&view), Mode::Emerging { .. }) {
            frames.extend(gate_frames(&drawn(&view)));
            ticks(&mut view, 1);
        }
        assert_eq!(frames[..3], [1, 2, 3], "opening a frame a tick");
        assert!(frames.iter().all(|&frame| frame > 0), "{frames:?}");
        assert!(frames[3..].iter().all(|&frame| frame >= 3), "{frames:?}");
        assert!(frames.contains(&5), "its loop plays: {frames:?}");

        let mut quiet = FlightView::new(with_hypergate(catalog()));
        for _ in 0..60 {
            ticks(&mut quiet, 1);
            assert_eq!(gate_frames(&drawn(&quiet)), [0], "no ship in it");
        }
    }

    #[test]
    fn an_npc_is_drawn_between_its_last_two_steps() {
        // At (0, 100) facing up, it heads for Earth, straight up.
        let mut draws = placed(750, 850, 0).to_vec();
        draws.push(0);
        let (_, chance) = scripted(&draws);
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 1)).with_chance(chance);
        // The first step sets the system up; the NPC moves from the next.
        view.tick(TICK);
        assert_eq!(npc_states(&view)[0].position, Vec2::new(0.0, 100.0));
        view.tick(TICK + TICK / 2);
        let npc = &view.session().expect("flying").npcs()[0];
        assert_eq!(
            npc.goal,
            Goal::Land(StellarId(128)),
            "Nova's AI, by default: a trader's business"
        );
        let moved = npc.state.position.y - 100.0;
        assert!(moved < 0.0, "{npc:?}");
        let alpha = view.alpha();
        assert!(alpha > 0.4, "{alpha}");
        let shown = sprites(&drawn(&view))[2].1;
        let expected = view
            .camera()
            .world_to_screen(at(0.0, moved.mul_add(alpha, 100.0)));
        assert!(
            (shown.y - expected.y).abs() < 1e-3,
            "{shown:?} {expected:?}"
        );
        assert_eq!(shown.x, expected.x);
    }

    #[test]
    fn an_npc_whose_sheet_cannot_be_read_is_a_crossed_box() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 130, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let list = drawn(&view);
        let mut boxed = DrawList::new();
        crossed_box(&mut boxed, at(612.0, 284.0), PLACEHOLDER_SIZE, PLACEHOLDER);
        let commands: Vec<_> = list.iter().cloned().collect();
        let expected: Vec<_> = boxed.iter().cloned().collect();
        assert!(
            commands.windows(expected.len()).any(|run| run == expected),
            "{commands:?}"
        );
        assert_eq!(sprites(&list).len(), 3, "two stellars and the player");
    }

    #[test]
    fn each_ship_types_sheet_is_read_once() {
        let mut view = FlightView::new(trafficked(&[130, 131], 3, 129, 1));
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
        ticks(&mut view, 5);
        assert_eq!(npc_ids(&view).len(), 3);
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)]
        );
        jump_to_alpha(&mut view);
        assert!(!npc_ids(&view).is_empty());
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)],
            "not again"
        );
    }

    #[test]
    fn arriving_repopulates_from_the_new_system() {
        let mut view = FlightView::new(trafficked(&[131], 2, 129, 1));
        view.tick(TICK);
        assert_eq!(npc_ids(&view), []);
        jump_to_alpha(&mut view);
        assert_eq!(npc_ids(&view).len(), 2);
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)],
            "read on arrival"
        );
    }

    /// Turns east at rest, then drifts out slowly that way until far
    /// enough to jump: facing Alpha Centauri and slow enough, J streaks at
    /// once.
    fn ready_to_jump(view: &mut View) {
        view.input(&key(Key::Right, true));
        ticks(view, 30);
        view.input(&key(Key::Right, false));
        view.input(&key(Key::Up, true));
        ticks(view, 10);
        view.input(&key(Key::Up, false));
        for _ in 0..2000 {
            if player(view).position.length() >= MIN_JUMP_DISTANCE {
                break;
            }
            view.tick(TICK);
        }
        assert_eq!(player(view).heading, 90.0);
    }

    #[test]
    fn the_traffic_flies_under_the_streak_through_the_warps_lead_in_then_stands_still() {
        // The traffic follows the ship out, then makes for planet 128 at
        // the centre as the ship jumps.
        let catalog = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..trafficked(&[130], 2, 129, 1)
        };
        let jumped = Rc::new(std::cell::Cell::new(false));
        let landing = Goal::Land(StellarId(128));
        let mut view = FlightView::new(catalog)
            .with_behaviour(Rc::new(InspectingUntil(Rc::clone(&jumped), landing)));
        plot(&mut view, 131);
        ready_to_jump(&mut view);
        jumped.set(true);
        view.input(&key(JUMP, true));
        assert!(view.jump_effect().is_some(), "streaking on J");
        let leaving = player(&view);
        let start = npc_states(&view);
        assert_eq!(start.len(), 2);
        // The lead-in lasts 36 ticks (`nova_sim::warp`): 1.2 s.
        view.tick(ms(500));
        let session = view.session().expect("flying");
        assert!(session.in_warp_lead_in());
        assert_ne!(
            npc_states(&view),
            start,
            "the traffic flies on under the streak"
        );
        assert_eq!(player(&view), leaving, "the ship stays where it is");
        view.tick(ms(800));
        let session = view.session().expect("flying");
        assert!(!session.in_warp_lead_in(), "past the lead-in");
        assert_eq!(session.system(), SystemId(130), "not yet arrived");
        let past = npc_states(&view);
        view.tick(ms(150));
        assert_eq!(npc_states(&view), past, "the traffic stands still");
        assert_eq!(view.session().expect("flying").system(), SystemId(130));
        // The arrival keeps the effect's time: past 1.5 s.
        view.tick(ms(100));
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
    }

    #[test]
    fn npcs_stand_still_while_the_map_is_open_or_the_ship_is_landed() {
        let catalog = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..trafficked(&[130], 2, 129, 1)
        };
        let mut view = FlightView::new(catalog);
        ticks(&mut view, 3);
        let before = npc_states(&view);
        assert_eq!(before.len(), 2);
        tap(&mut view, MAP);
        ticks(&mut view, 10);
        assert_eq!(npc_states(&view), before, "the map is open");
        tap(&mut view, MAP);
        land_now(&mut view);
        assert!(view.take_landing().is_some());
        let before = npc_states(&view);
        ticks(&mut view, 10);
        assert_eq!(npc_states(&view), before, "landed");
    }

    #[test]
    fn taking_off_repopulates_the_system() {
        let catalog = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..trafficked(&[130], 2, 129, 1)
        };
        let mut view = FlightView::new(catalog);
        view.tick(TICK);
        assert_eq!(npc_ids(&view), [NpcId(0), NpcId(1)]);
        land_now(&mut view);
        view.take_off().expect("took off");
        assert_eq!(npc_ids(&view), [NpcId(2), NpcId(3)], "before any tick");
        view.tick(TICK);
        assert_eq!(npc_ids(&view), [NpcId(2), NpcId(3)], "populated once");
    }

    /// [`trafficked`] with one ship 129 of AI 3 in 130, over planet 128
    /// at the centre.
    /// [`trafficked`] with the ship over an uninhabited 128, which it
    /// lands on in one press, with no tick to bring the traffic in.
    fn trafficked_over_128() -> FakeCatalog {
        FakeCatalog {
            sites: vec![site(
                128,
                (0.0, 0.0),
                StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            )],
            ..trafficked(&[130], 1, 129, 3)
        }
    }

    /// Whether `list` draws a sprite at world point `world`.
    fn sprite_at(view: &View, list: &DrawList, world: Vec2) -> bool {
        let screen = view.camera().world_to_screen(point(world));
        sprites(list).iter().any(|&(_, center)| center == screen)
    }

    #[test]
    fn the_first_frame_after_take_off_draws_none_of_the_ships_from_before_the_landing() {
        let mut draws = placed(850, 650, 90).to_vec();
        // The take-off's escort-strength seed, then its population.
        draws.push(0);
        draws.extend(placed(650, 850, 90));
        let (_, chance) = scripted(&draws);
        let mut view = FlightView::new(trafficked_over_128())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let before = npc_states(&view)[0].position;
        assert_eq!(before, Vec2::new(100.0, -100.0));
        tap(&mut view, LAND);
        view.take_off().expect("took off");
        let after = npc_states(&view)[0].position;
        assert_eq!(after, Vec2::new(-100.0, 100.0));

        let list = drawn(&view);
        assert!(!sprite_at(&view, &list, before), "with no tick");
        assert!(sprite_at(&view, &list, after), "with no tick");

        let states = npc_states(&view);
        view.tick(TICK / 4);
        assert_eq!(npc_states(&view), states, "no step");
        let list = drawn(&view);
        assert!(!sprite_at(&view, &list, before), "after a frame of no step");
        assert!(sprite_at(&view, &list, after), "after a frame of no step");
    }

    #[test]
    fn a_ship_type_first_flown_at_take_off_is_drawn_with_its_sprite_at_once() {
        // The take-off's escort-strength seed, then its population.
        let mut draws = vec![0];
        draws.extend(placed(650, 850, 90));
        let (_, chance) = scripted(&draws);
        let mut view = FlightView::new(trafficked_over_128())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        tap(&mut view, LAND);
        assert!(view.take_landing().is_some(), "landed before any traffic");
        view.take_off().expect("took off");
        let after = npc_states(&view)[0].position;
        let list = drawn(&view);
        let mut boxed = DrawList::new();
        crossed_box(
            &mut boxed,
            view.camera().world_to_screen(point(after)),
            PLACEHOLDER_SIZE,
            PLACEHOLDER,
        );
        let commands: Vec<_> = list.iter().cloned().collect();
        let expected: Vec<_> = boxed.iter().cloned().collect();
        assert!(
            !commands.windows(expected.len()).any(|run| run == expected),
            "no placeholder"
        );
        assert!(sprite_at(&view, &list, after), "its own sprite");
    }

    // Combat.

    /// Says no ship is disabled, counting the times it is asked.
    #[derive(Debug, Default)]
    struct Counting {
        asked: std::cell::Cell<usize>,
    }

    impl DisableRule for Counting {
        fn disabled(&self, _armor: nova_sim::Gauge, _hull: &nova_sim::HullSpec) -> bool {
            self.asked.set(self.asked.get() + 1);
            false
        }
    }

    #[test]
    fn the_fight_asks_the_disable_rule_given_each_step_in_flight() {
        let rule = Rc::new(Counting::default());
        let uninhabited = StellarFlags::CAN_LAND | StellarFlags::UNINHABITED;
        let mut view =
            flight_among(vec![site(140, (0.0, 0.0), uninhabited)]).with_disable_rule(rule.clone());
        view.tick(TICK);
        assert_eq!(rule.asked.get(), 1, "the player, the only ship");
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3);
        view.input(&key(MAP_KEY, true));
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3, "not while the map is open");
        view.input(&key(MAP_KEY, true));
        tap(&mut view, LAND);
        assert!(view.take_landing().is_some(), "one press, uninhabited");
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3, "not while landed");
    }

    /// Every NPC idles and holds its trigger.
    #[derive(Debug)]
    struct Firing;

    impl Behaviour for Firing {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Idle
        }

        fn trigger(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
        ) -> nova_sim::Trigger {
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

    #[test]
    fn the_sessions_diagnostics_come_through_the_screen() {
        let planet_type = nova_sim::WeaponRecord {
            id: nova_sim::WeaponId(181),
            reload: 20,
            count: 12,
            mass_dmg: 2,
            energy_dmg: 2,
            guidance: -1,
            speed: 1000,
            ammo_type: -1,
            inaccuracy: 0,
            impact: 5,
            explod_type: -1,
            prox_radius: 5,
            blast_radius: 6,
            flags: 0,
            seeker: 0,
            flags2: 0x0400,
            flags3: 0,
            decay: 0,
            beam_length: 0,
            falloff: 0,
            li_density: 0,
            burst_count: 0,
            burst_reload: 0,
            guided_turn: 0,
            durability: 0,
            sub_count: 0,
            sub_type: None,
            sub_theta: 0,
            sub_limit: 0,
            max_ammo: 0,
            prox_safety: 0,
            recoil: 0,
            exit_type: -1,
            ionization: 0,
            ionize_color: 0,
            jam_vuln: [0; 4],
            frame: None,
            hit_particles: 0,
            hit_part_life: 0,
            hit_part_vel: 0,
            hit_part_color: 0,
        };
        let hull = nova_sim::HullRecord {
            id: ShipId(129),
            flags: 0,
            flags2: 0,
            death_delay: 0,
            explode1: -1,
            explode2: -1,
            mass: 0,
            weapons: vec![nova_sim::StockWeapon {
                weapon: nova_sim::WeaponId(181),
                count: 1,
                ammo: 0,
            }],
            size: None,
            height: None,
            strength: 0,
            key_carried: None,
            cost: 0,
            speed: 0,
            exits: None,
            escort_class: nova_sim::EscortClass::Freighter,
        };
        let (_, chance) = scripted(&placed(850, 650, 90));
        let catalog = FakeCatalog {
            weapons: vec![planet_type],
            hulls: vec![hull],
            looks: vec![(181, Ok(WeaponLook::default()))],
            ..trafficked(&[130], 1, 129, 3)
        };
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Firing));
        assert_eq!(view.take_diagnostics(), []);
        ticks(&mut view, 3);
        assert_eq!(
            view.take_diagnostics(),
            [Diagnostic::Sim(SimDiagnostic::UnimplementedWeaponFlag {
                weapon: nova_sim::WeaponId(181),
                field: nova_sim::combat::flags::FlagField::Flags2,
                bit: 0x0400
            })]
        );
        ticks(&mut view, 30);
        assert_eq!(view.take_diagnostics(), [], "once");
    }

    #[test]
    fn a_look_whose_sheet_cannot_be_read_still_sounds_and_is_reported_once() {
        let mut catalog = armed(0, &[BLASTER]);
        catalog.looks[0].1 = Ok(WeaponLook {
            sheet: Some(Err("no spïn 3005".to_owned())),
            ..look("Blaster", 208, None)
        });
        catalog.booms.push((
            129,
            Ok(BoomLook {
                sheet: Err("no spïn 401".to_owned()),
                advance: 1.0,
                sound: None,
            }),
        ));
        let mut view = fighting(catalog, &[]);
        assert_eq!(
            view.take_diagnostics(),
            [
                Diagnostic::Unreadable("wëap 128: no spïn 3005".to_owned()),
                Diagnostic::Unreadable("no wëap 150".to_owned()),
                Diagnostic::Unreadable("bööm 129: no spïn 401".to_owned()),
            ],
            "and the unseen weapon, which has no look at all"
        );
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        let sounds: Vec<_> = combat_sounds(&mut view).iter().map(|s| s.sound).collect();
        assert_eq!(sounds, [nova_sim::SoundId(208)], "the blaster is heard");
        ticks(&mut view, 3);
        assert_eq!(view.take_diagnostics(), [], "once");
    }

    // Combat controls and display.

    use std::num::NonZeroU16 as Frames;

    use nova_sim::{BoomId, Condition, PlayerFate, WeaponId};

    use crate::flight::brackets::{self, Standing};
    use crate::flight::catalog::EffectSheet;
    use crate::flight::effects::{DEBRIS_COLOR, Effects};
    use crate::flight::target::{self, NO_SECONDARY, NO_TARGET};
    use crate::flight::weapons::{BEAM_UNDER_SHIPS, translucent};
    use crate::sound::CombatSound;

    const BLASTER: WeaponId = WeaponId(128);
    const ROCKET: WeaponId = WeaponId(140);
    const MISSILE: WeaponId = WeaponId(141);
    const TORCH: WeaponId = WeaponId(142);
    const UNDER: WeaponId = WeaponId(146);
    const OVER: WeaponId = WeaponId(147);
    const UNSEEN: WeaponId = WeaponId(150);

    /// `wëap` `id` firing every tick, 20 pixels a tick for 30 ticks, doing
    /// no damage, with `flags`, `guidance` and `explod_type`.
    fn gun(id: WeaponId, flags: u16, guidance: i16, explod_type: i16) -> nova_sim::WeaponRecord {
        nova_sim::WeaponRecord {
            id,
            reload: 0,
            count: 30,
            mass_dmg: 0,
            energy_dmg: 0,
            guidance,
            speed: 2000,
            ammo_type: -1,
            inaccuracy: 0,
            impact: 0,
            explod_type,
            prox_radius: 0,
            blast_radius: 0,
            flags,
            seeker: 0,
            flags2: 0,
            flags3: 0,
            decay: 0,
            beam_length: 50,
            falloff: 0,
            li_density: 0,
            burst_count: 0,
            burst_reload: 0,
            guided_turn: 0,
            durability: 0,
            sub_count: 0,
            sub_type: None,
            sub_theta: 0,
            sub_limit: 0,
            max_ammo: 0,
            prox_safety: 0,
            recoil: 0,
            exit_type: -1,
            ionization: 0,
            ionize_color: 0,
            jam_vuln: [0; 4],
            frame: None,
            hit_particles: 0,
            hit_part_life: 0,
            hit_part_vel: 0,
            hit_part_color: 0,
        }
    }

    /// Ship `id` carrying one of each of `weapons`, gone at once in
    /// `bööm` 128.
    fn hull_of(id: i16, weapons: &[WeaponId]) -> nova_sim::HullRecord {
        nova_sim::HullRecord {
            id: ShipId(id),
            flags: 0,
            flags2: 0,
            death_delay: 0,
            explode1: -1,
            explode2: 0,
            mass: 0,
            weapons: weapons
                .iter()
                .map(|&weapon| nova_sim::StockWeapon {
                    weapon,
                    count: 1,
                    ammo: 0,
                })
                .collect(),
            size: None,
            height: None,
            strength: 0,
            key_carried: None,
            cost: 0,
            speed: 0,
            exits: None,
            escort_class: nova_sim::EscortClass::Freighter,
        }
    }

    /// `oütf` `id`, "Gun `id`", holding a `weapon`: a ton, 1000 credits.
    fn holding(id: i16, weapon: WeaponId) -> OutfitRecord {
        OutfitRecord {
            id: OutfitId(id),
            name: format!("Gun {id}"),
            short_name: format!("Gun {id}"),
            disp_weight: 0,
            mass: 1,
            tech_level: 1,
            max: 10,
            flags: 0,
            cost: 1000,
            mods: [
                (nova_sim::combat::armament::MOD_WEAPON, weapon.0),
                (0, 0),
                (0, 0),
                (0, 0),
            ],
            contribute: 0,
            require: 0,
            require_govt: -1,
            buy_random: 100,
            availability: nova_sim::Test::default(),
            item_class: 0,
            lc_name: format!("gun {id}"),
            lc_plural: format!("guns {id}"),
            scan_mask: 0,
            on_purchase: nova_sim::Script::default(),
            on_sell: nova_sim::Script::default(),
        }
    }

    fn effect_sheet(image_id: i16, frames: u16) -> EffectSheet {
        EffectSheet {
            image_id,
            frames: Frames::new(frames).expect("non-zero"),
        }
    }

    /// A look named `name` sounding `snd ` `sound`, its shots on `rlëD`
    /// `image` of 36 frames, if any.
    fn look(name: &str, sound: i16, image: Option<i16>) -> WeaponLook {
        WeaponLook {
            name: name.to_owned(),
            sheet: image.map(|image| Ok(effect_sheet(image, 36))),
            sound: Some(nova_sim::SoundId(sound)),
            ..WeaponLook::default()
        }
    }

    /// [`trafficked`] (ships of type 129, warships) where the player's
    /// ship carries `weapons`, held by outfits 250 on, in that order; the
    /// blaster, rockets, missiles and torch have looks, and `bööm` 128 shows `rlëD` 400's 3 frames at a frame
    /// a step, sounding `snd ` 302. Ship 129 is a "Light Transport" with
    /// picture 3001, and govt 140's code is "Fed.".
    fn armed(avg: i16, weapons: &[WeaponId]) -> FakeCatalog {
        let mut catalog = trafficked(&[130], avg, 129, 3);
        catalog.dudes[0].1.govt = Some(GovtId(140));
        FakeCatalog {
            weapons: vec![
                gun(BLASTER, 0, -1, 0),
                gun(ROCKET, 0x0002, -1, -1),
                gun(MISSILE, 0x0002, -1, -1),
                gun(TORCH, 0x0002, -1, -1),
                gun(UNDER, 0, 0, -1),
                gun(OVER, 0, 0, -1),
                gun(UNSEEN, 0, -1, -1),
            ],
            hulls: vec![hull_of(128, weapons), hull_of(129, &[])],
            outfits: (250..)
                .zip(weapons)
                .map(|(id, &weapon)| holding(id, weapon))
                .collect(),
            looks: vec![
                (128, Ok(look("Blaster", 208, Some(3500)))),
                (140, Ok(look("Rocket", 209, Some(3501)))),
                (141, Ok(look("Missile", 210, None))),
                (142, Ok(look("Torch", 211, None))),
                (
                    146,
                    Ok(WeaponLook {
                        flags2: BEAM_UNDER_SHIPS,
                        beam_width: 1,
                        beam_color: 0x0000_00FF,
                        ..WeaponLook::default()
                    }),
                ),
                (
                    147,
                    Ok(WeaponLook {
                        beam_width: 1,
                        beam_color: 0x0000_FF00,
                        ..WeaponLook::default()
                    }),
                ),
            ],
            booms: vec![(
                128,
                Ok(BoomLook {
                    sheet: Ok(effect_sheet(400, 3)),
                    advance: 1.0,
                    sound: Some(nova_sim::SoundId(302)),
                }),
            )],
            cards: vec![(
                129,
                TargetCard {
                    subtitle: "Light Transport".to_owned(),
                    picture: Some(3001),
                },
            )],
            codes: vec![(140, "Fed.".to_owned())],
            ..catalog
        }
    }

    /// `catalog`'s flight with its NPCs idling, placed by `draws`, after
    /// the step that populates the system.
    fn fighting(catalog: FakeCatalog, draws: &[u32]) -> View {
        let (_, chance) = scripted(draws);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        view
    }

    /// NPC 0 at (100, -100) and NPC 1 at (-50, 0), nearer the player.
    fn two_npcs() -> Vec<u32> {
        [placed(850, 650, 90), placed(700, 750, 0)].concat()
    }

    fn target_of(view: &View) -> Option<NpcId> {
        view.session().expect("flying").target().map(|npc| npc.id)
    }

    /// Taps R while holding Alt (Option).
    fn alt_tap(view: &mut View, k: Key) {
        view.input(&key(Key::Alt, true));
        tap(view, k);
        view.input(&key(Key::Alt, false));
    }

    #[test]
    fn tab_targets_the_next_npc_and_alt_r_the_nearest() {
        let mut view = fighting(armed(2, &[]), &two_npcs());
        assert_eq!(target_of(&view), None);
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), Some(NpcId(0)));
        view.input(&held(Key::Tab));
        assert_eq!(target_of(&view), Some(NpcId(0)), "a repeat does nothing");
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), None, "past the last");
        alt_tap(&mut view, NEAREST_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        tap(&mut view, Key::Tab);
        view.input(&key(Key::Alt, true));
        view.input(&held(NEAREST_KEY));
        assert_eq!(target_of(&view), None, "a repeat does nothing");
        assert_eq!((TARGET_KEY, NEAREST_KEY), (Key::Tab, Key::Char('r')));
    }

    #[test]
    fn a_target_key_press_beeps_once_and_its_repeat_is_silent() {
        let mut view = fighting(armed(2, &[]), &two_npcs());
        view.take_sounds();
        view.input(&key(Key::Tab, true));
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TargetSelected)]);
        view.input(&held(Key::Tab));
        view.input(&key(Key::Tab, false));
        assert_eq!(view.take_sounds(), [], "a repeat of Tab");
        alt_tap(&mut view, NEAREST_KEY);
        view.take_sounds();
        alt_tap(&mut view, NEAREST_KEY);
        assert_eq!(view.take_sounds(), [], "a fresh Alt-R on the same ship");
        // No ship threatens, so an acting R would beep the refusal.
        let before = target_of(&view);
        view.input(&held(NEAREST_KEY));
        assert_eq!(view.take_sounds(), [], "a repeat of R");
        assert_eq!(target_of(&view), before);
        view.input(&key(NEAREST_KEY, false));
        tap(&mut view, NEAREST_KEY);
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::Refused)],
            "a fresh R finds no threat"
        );
    }

    /// NPC 0 attacks the player; the rest idle.
    #[derive(Debug)]
    struct Threatening;

    impl Behaviour for Threatening {
        fn decide(
            &self,
            npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            if npc.id == NpcId(0) {
                Goal::Attack(ShipRef::Player)
            } else {
                Goal::Idle
            }
        }
    }

    #[test]
    fn r_targets_the_nearest_threat_over_a_nearer_ship_and_brackets_it_red() {
        let mut view = fighting(armed(2, &[]), &two_npcs()).with_behaviour(Rc::new(Threatening));
        tap(&mut view, NEAREST_KEY);
        assert_eq!(target_of(&view), None, "no threat yet: unchanged");
        view.tick(TICK);
        tap(&mut view, NEAREST_KEY);
        assert_eq!(
            target_of(&view),
            Some(NpcId(0)),
            "the threat, though farther"
        );
        let list = drawn(&view);
        assert_eq!(bracket_icons(&list), [10008, 10009, 10010, 10011], "red");
        alt_tap(&mut view, NEAREST_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)), "Alt-R: the nearer");
        let list = drawn(&view);
        assert_eq!(bracket_icons(&list), [10012, 10013, 10014, 10015], "yellow");
    }

    /// How many of the shots in flight are of `weapon`.
    fn shots_of(view: &View, weapon: WeaponId) -> usize {
        let session = view.session().expect("flying");
        session
            .shots()
            .iter()
            .filter(|shot| shot.weapon.id == weapon)
            .count()
    }

    #[test]
    fn space_fires_the_primaries_and_control_the_secondary_while_held() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET]), &[]);
        tap(&mut view, SELECT_KEY);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 2);
        view.input(&held(FIRE_KEY));
        ticks(&mut view, 1);
        assert_eq!((shots_of(&view, BLASTER), shots_of(&view, ROCKET)), (3, 0));
        view.input(&key(FIRE_KEY, false));
        ticks(&mut view, 2);
        assert_eq!(shots_of(&view, BLASTER), 3, "let go");
        view.input(&key(SECONDARY_KEY, true));
        ticks(&mut view, 2);
        assert_eq!((shots_of(&view, BLASTER), shots_of(&view, ROCKET)), (3, 2));
        view.input(&key(SECONDARY_KEY, false));
        ticks(&mut view, 1);
        assert_eq!(shots_of(&view, ROCKET), 2, "let go");
        assert_eq!((FIRE_KEY, SECONDARY_KEY), (Key::Space, Key::Control));
    }

    #[test]
    fn the_flight_keys_are_held_until_let_go_or_released() {
        let mut view = flight();
        assert!(!view.holds_keys());
        view.input(&key(Key::Up, true));
        assert!(view.holds_keys());
        view.input(&key(Key::Up, false));
        assert!(!view.holds_keys(), "let go");
        view.input(&key(Key::Up, true));
        view.release_keys();
        assert!(!view.holds_keys(), "released");
    }

    #[test]
    fn letting_go_of_the_keys_lets_go_of_the_fire_keys() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET]), &[]);
        tap(&mut view, SELECT_KEY);
        view.input(&key(FIRE_KEY, true));
        view.input(&key(SECONDARY_KEY, true));
        ticks(&mut view, 1);
        view.release_keys();
        ticks(&mut view, 2);
        assert_eq!((shots_of(&view, BLASTER), shots_of(&view, ROCKET)), (1, 1));
    }

    fn secondary_of(view: &View) -> Option<WeaponId> {
        view.session().expect("flying").secondary()
    }

    #[test]
    fn w_selects_the_next_secondary_and_alt_w_the_one_before() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET, MISSILE, TORCH]), &[]);
        assert_eq!(secondary_of(&view), None);
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(MISSILE));
        view.input(&held(SELECT_KEY));
        assert_eq!(secondary_of(&view), Some(MISSILE), "a repeat does nothing");
        view.input(&key(Key::Alt, true));
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(TORCH), "wrapping");
        view.input(&key(Key::Alt, false));
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        view.input(&key(Key::Alt, true));
        view.release_keys();
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(MISSILE), "Alt let go");
        assert_eq!(SELECT_KEY, Key::Char('w'));
    }

    #[test]
    fn the_combat_keys_do_nothing_while_the_map_is_open() {
        let mut view = fighting(armed(2, &[BLASTER, ROCKET, MISSILE]), &two_npcs());
        tap(&mut view, MAP);
        for k in [Key::Tab, NEAREST_KEY, SELECT_KEY, FIRE_KEY, SECONDARY_KEY] {
            view.input(&key(k, true));
        }
        tap(&mut view, MAP);
        ticks(&mut view, 2);
        assert_eq!(target_of(&view), None);
        assert_eq!(secondary_of(&view), None);
        assert_eq!(view.session().expect("flying").shots(), []);
    }

    #[test]
    fn the_fire_keys_pressed_and_let_go_during_a_jump_fire_nothing() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET, MISSILE]), &[]);
        tap(&mut view, SELECT_KEY);
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        for k in [FIRE_KEY, SECONDARY_KEY] {
            view.input(&key(k, true));
        }
        while view.jump_effect().is_none() {
            view.tick(TICK);
            assert_eq!(view.session().expect("flying").shots(), [], "braking");
        }
        for k in [FIRE_KEY, SECONDARY_KEY] {
            view.input(&key(k, false));
        }
        arrive_now(&mut view);
        ticks(&mut view, 2);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        assert_eq!(view.session().expect("flying").shots(), []);
    }

    /// The combat sounds taken.
    fn combat_sounds(view: &mut View) -> Vec<CombatSound> {
        view.take_sounds()
            .into_iter()
            .filter_map(|sound| match sound {
                Sound::Combat(sound) => Some(sound),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn every_steps_fight_reaches_the_effects() {
        let mut view = fighting(armed(0, &[BLASTER]), &[]);
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK * 3);
        assert_eq!(shots_of(&view, BLASTER), 3, "three steps");
        let fired = CombatSound {
            sound: nova_sim::SoundId(208),
            offset: (0, 0),
        };
        assert_eq!(combat_sounds(&mut view), [fired; 3]);
        assert_eq!(combat_sounds(&mut view), [], "taken");
    }

    #[test]
    fn the_sounds_are_the_sessions_then_the_fights() {
        let mut view = fighting(armed(0, &[BLASTER]), &[]);
        view.input(&key(Key::Up, true));
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        assert_eq!(
            view.take_sounds(),
            [
                Sound::Sim(SimSound::ThrustStarted),
                Sound::Combat(CombatSound {
                    sound: nova_sim::SoundId(208),
                    offset: (0, 0),
                })
            ]
        );
    }

    /// The first command matching `wanted`'s position in `list`.
    fn first(list: &DrawList, wanted: impl Fn(&DrawCommand) -> bool) -> usize {
        list.iter()
            .position(wanted)
            .unwrap_or_else(|| panic!("not drawn: {list:?}"))
    }

    fn sprite_of(id: i16) -> impl Fn(&DrawCommand) -> bool {
        move |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == id)
    }

    fn line_in(color: Color) -> impl Fn(&DrawCommand) -> bool {
        move |command| matches!(command, DrawCommand::Line { color: c, .. } if *c == color)
    }

    /// Whether a command draws a `cïcn` in `ids`.
    fn icon_in(ids: std::ops::RangeInclusive<i16>) -> impl Fn(&DrawCommand) -> bool {
        move |command| {
            matches!(
                command,
                DrawCommand::Picture { image, .. }
                    if image.kind == crate::ImageKind::Icon && ids.contains(&image.id)
            )
        }
    }

    /// The bracket `cïcn`s drawn (10000-10023), in order.
    fn bracket_icons(list: &DrawList) -> Vec<i16> {
        list.iter()
            .filter(|command| icon_in(10000..=10023)(command))
            .map(|command| match command {
                DrawCommand::Picture { image, .. } => image.id,
                _ => unreachable!("a picture"),
            })
            .collect()
    }

    /// The flight with an NPC 100 pixels above the player, the player
    /// firing its blaster (which explodes where it hits) and two beams, one
    /// drawn under the ships and one over them, at it.
    fn firing_at_an_npc() -> View {
        let catalog = armed(1, &[BLASTER, UNDER, OVER]);
        let mut view = fighting(catalog, &placed(750, 650, 180));
        tap(&mut view, Key::Tab);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 5);
        view
    }

    #[test]
    fn the_fight_is_drawn_between_the_stellars_and_the_hud_in_order() {
        let view = firing_at_an_npc().with_overlays(TITLED);
        let list = drawn(&view);
        let stellar = first(&list, sprite_of(1128));
        let under = first(&list, line_in(Color::rgba(0, 0, 255, 255)));
        let npc = first(&list, sprite_of(2001));
        let ship = first(&list, sprite_of(2000));
        let shot = first(&list, sprite_of(3500));
        let over = first(&list, line_in(Color::rgba(0, 255, 0, 255)));
        let explosion = first(&list, sprite_of(400));
        let brackets = first(&list, icon_in(10008..=10023));
        let title = first(
            &list,
            |c| matches!(c, DrawCommand::Text { origin, .. } if *origin == TITLE),
        );
        let hud = hud_at(&list);
        let order = [
            stellar, under, npc, ship, shot, over, explosion, brackets, title, hud,
        ];
        assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
        let DrawCommand::Sprite { tint, .. } = list.iter().nth(explosion).cloned().expect("drawn")
        else {
            panic!("a sprite")
        };
        assert_eq!(tint, translucent());
    }

    #[test]
    fn a_shot_is_drawn_with_its_frame_where_it_flies() {
        let mut view = fighting(armed(0, &[BLASTER]), &[]);
        // Turned 45 degrees right, so the shot flies across and up.
        view.input(&key(Key::Right, true));
        ticks(&mut view, 15);
        view.input(&key(Key::Right, false));
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        view.tick(TICK / 2);
        let session = view.session().expect("flying");
        let shot = session.shots()[0];
        let alpha = view.alpha();
        let shown = at(
            shot.position.x - shot.velocity.x * (1.0 - alpha),
            shot.position.y - shot.velocity.y * (1.0 - alpha),
        );
        let list = drawn(&view);
        let drawn_at = sprites(&list)
            .into_iter()
            .find(|(image, _)| image.id == 3500)
            .expect("the shot");
        assert_eq!(drawn_at.0, ImageKey::sprite(3500, 4), "heading 45");
        let expected = view.camera().world_to_screen(shown);
        assert!(
            (drawn_at.1.x - expected.x).abs() < 1e-3 && (drawn_at.1.y - expected.y).abs() < 1e-3,
            "{drawn_at:?} {expected:?}"
        );
    }

    /// Every NPC heads for planet 128 and holds its trigger.
    #[derive(Debug)]
    struct LandingFiring;

    impl Behaviour for LandingFiring {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Land(StellarId(128))
        }

        fn trigger(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
        ) -> nova_sim::Trigger {
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

    /// Where the beams in `color` start, on screen: the lines a pixel wide,
    /// not the HUD's bars.
    fn beam_starts(list: &DrawList, color: Color) -> Vec<Point> {
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Line {
                    from,
                    color: c,
                    width,
                    ..
                } if c == color && width == 1.0 => Some(from),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_beams_fade_is_shown_as_the_session_counts_it() {
        let mut catalog = armed(0, &[OVER]);
        for weapon in &mut catalog.weapons {
            if weapon.id == OVER {
                *weapon = nova_sim::WeaponRecord {
                    count: 1,
                    reload: 100,
                    decay: 1,
                    falloff: 10,
                    ..*weapon
                };
            }
        }
        let mut view = fighting(catalog, &[]);
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        ticks(&mut view, 2);
        let fade = view.session().expect("flying").beams()[0].fade;
        assert!(fade > 0, "fading");
        let shown: Vec<u32> = view
            .shown_beams(&view.camera())
            .iter()
            .map(|beam| beam.fade)
            .collect();
        assert_eq!(shown, [fade]);
    }

    #[test]
    fn a_beam_is_drawn_from_its_firer_where_the_firer_is_drawn() {
        let mut catalog = armed(1, &[OVER]);
        catalog.hulls[1] = hull_of(129, &[UNDER]);
        let (_, chance) = scripted(&placed(850, 650, 0));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(LandingFiring));
        view.tick(TICK);
        view.input(&key(Key::Up, true));
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 5);
        view.tick(TICK / 2);
        let session = view.session().expect("flying");
        let npc = &session.npcs()[0];
        assert_ne!(
            view.npc_previous[&npc.id].position, npc.state.position,
            "moving"
        );
        assert_ne!(view.previous.position, session.player().position, "moving");
        let list = drawn(&view);
        let camera = view.camera();
        let player = camera.world_to_screen(view.shown_position());
        let over = beam_starts(&list, Color::rgba(0, 255, 0, 255));
        assert!(!over.is_empty());
        assert!(
            over.iter().all(|&start| start == player),
            "the player's, from the player as drawn: {over:?}"
        );
        let (npc_shown, _) = view.shown_npc(npc);
        let under = beam_starts(&list, Color::rgba(0, 0, 255, 255));
        assert!(!under.is_empty());
        let expected = camera.world_to_screen(npc_shown);
        assert!(
            under
                .iter()
                .all(|start| (start.x - expected.x).abs() < 1e-3
                    && (start.y - expected.y).abs() < 1e-3),
            "the NPC's, from the NPC as drawn: {under:?} {expected:?}"
        );
    }

    #[test]
    fn a_ship_breaking_up_goes_off_about_where_it_is() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        catalog.hulls[1].death_delay = 30;
        catalog.hulls[1].explode1 = 0;
        // 100 above, facing away: it flies on up the line of fire.
        let mut view = fighting(catalog, &placed(750, 650, 0));
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        let mut breaking = None;
        for _ in 0..40 {
            view.tick(TICK);
            let session = view.session().expect("flying");
            if let Some(npc) = session.npcs().first()
                && let Condition::Dying { ticks_left, .. } = npc.condition
                && ticks_left < 19
                && !view.effects().explosions().is_empty()
            {
                breaking = Some(npc.state.position - npc.state.velocity);
                break;
            }
        }
        // Where it was before the tick's flight step moved it on.
        let at = breaking.expect("an explosion as it breaks up");
        // Never firing, the effects' chance draws the last outcome: a
        // quarter of its 40-pixel sprite less one out, each way.
        assert_eq!(
            view.effects().explosions()[0].at,
            Point::new(at.x + 9.0, at.y + 9.0)
        );
    }

    #[test]
    fn the_players_ship_breaking_up_goes_off_about_where_it_is() {
        // The NPC, straight above the player and facing it, fires a
        // blaster that kills at a hit; the player's ship, 80 pixels wide
        // (the NPC's 40), of DeathDelay 30, breaks up for 88 ticks: three
        // times it, less 2.
        let mut catalog = armed(1, &[]);
        catalog.sheet = Ok(ShipSheet {
            frame_width: 80,
            ..sheet()
        });
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        catalog.hulls[0].death_delay = 30;
        catalog.hulls[0].explode1 = 0;
        catalog.hulls[1] = hull_of(129, &[BLASTER]);
        let mut view = fighting(catalog, &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        let mut breaking = None;
        for _ in 0..120 {
            view.tick(TICK);
            let session = view.session().expect("flying");
            if let Condition::Dying { ticks_left, .. } = session.player_condition()
                && ticks_left < 19
                && !view.effects().explosions().is_empty()
            {
                breaking = Some(session.player().position);
                break;
            }
        }
        let at = breaking.expect("an explosion as the player's ship breaks up");
        // Never firing, the effects' chance draws the last outcome: a
        // quarter of the player's 80-pixel sprite less one out, each way.
        assert_eq!(
            view.effects().explosions()[0].at,
            Point::new(at.x + 19.0, at.y + 19.0)
        );
    }

    /// The fight of [`armed`] with the NPC straight above the player and
    /// facing it, firing a blaster of `mass_dmg` that ignores shields at
    /// it, ticked until the player is no longer intact: the line then.
    fn downed_line(mass_dmg: i16) -> (Condition, Option<String>) {
        let mut catalog = armed(1, &[]);
        catalog.weapons[0].mass_dmg = mass_dmg;
        catalog.weapons[0].flags = 0x0020;
        catalog.hulls[0].death_delay = 30;
        catalog.hulls[1] = hull_of(129, &[BLASTER]);
        let mut view = fighting(catalog, &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        for _ in 0..400 {
            view.tick(TICK);
            let condition = view.session().expect("flying").player_condition();
            if condition != Condition::Intact {
                return (condition, view.message().map(str::to_owned));
            }
        }
        panic!("the player was never downed");
    }

    #[test]
    fn the_player_broken_up_at_a_hit_is_left_told_ship_destroyed() {
        let (condition, line) = downed_line(1000);
        assert!(
            matches!(condition, Condition::Dying { .. }),
            "{condition:?}"
        );
        assert_eq!(
            line.as_deref(),
            Some("Ship destroyed."),
            "#288 replaced #287"
        );
    }

    #[test]
    fn the_player_disabled_is_told_ship_disabled() {
        let (condition, line) = downed_line(1);
        assert_eq!(condition, Condition::Disabled);
        assert_eq!(line.as_deref(), Some("Ship disabled."));
    }

    /// [`armed`] where the NPC, straight above the player and facing it,
    /// fires a blaster that kills at a hit, and the player's ship breaks
    /// up in `death_delay` ticks, then explodes as `bööm` 128 (`rlëD` 400).
    fn killed_in(death_delay: i16) -> FakeCatalog {
        let mut catalog = armed(1, &[]);
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        catalog.hulls[0].death_delay = death_delay;
        catalog.hulls[1] = hull_of(129, &[BLASTER]);
        catalog
    }

    /// [`killed_in`] (the player's `DeathDelay` `death_delay`) where the
    /// player starts with each of `outfits` (188, `ModType` 11, the escape
    /// pod; 187, `ModType` 20, auto-eject), with records of its ship and
    /// of the pod.
    fn ejecting_in(death_delay: i16, outfits: &[i16]) -> FakeCatalog {
        let mut catalog = killed_in(death_delay);
        let fitted = |id, mod_type| OutfitRecord {
            mods: [(mod_type, 0), (0, 0), (0, 0), (0, 0)],
            ..holding(id, WeaponId(128))
        };
        catalog.outfits.extend([fitted(188, 11), fitted(187, 20)]);
        catalog.defaults = outfits.iter().map(|&id| (OutfitId(id), 1)).collect();
        catalog.ships.extend([ship_class(128), ship_class(895)]);
        catalog
    }

    /// `shïp` `id`'s record, of [`FIELDS`].
    fn ship_class(id: i16) -> ShipRecord {
        ShipRecord {
            id: ShipId(id),
            name: format!("Ship {id}"),
            short_name: format!("Ship {id}"),
            long_name: String::new(),
            fields: FIELDS,
            defaults: Vec::new(),
            cost: 900,
            tech_level: 1,
            buy_random: 100,
            hire_random: 0,
            require: 0,
            availability: nova_sim::Test::default(),
            appear_on: nova_sim::Test::default(),
            flags: 0,
            flags3: 0,
            disp_weight: 0,
            max_gun: 0,
            max_tur: 0,
            length: 0,
            crew: 0,
            inherent_ai: 1,
            comm_name: String::new(),
            inherent_govt: None,
            combat_govt: None,
            escort_type: -1,
            on_capture: nova_sim::Script::default(),
            on_purchase: nova_sim::Script::default(),
            on_retire: nova_sim::Script::default(),
        }
    }

    /// `catalog`'s view under its NPC's fire, holding each of `keys`.
    fn ejecting_view(catalog: FakeCatalog, keys: &[Key]) -> View {
        let mut view = fighting(catalog, &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        for &held in keys {
            view.input(&key(held, true));
        }
        view
    }

    /// Ticks `view` until the player is in the escape pod, or for 60
    /// ticks: whether it got there, and whether it was seen breaking up.
    fn until_ejected(view: &mut View) -> (bool, bool) {
        let mut broke_up = false;
        for _ in 0..60 {
            view.tick(TICK);
            match view.session().expect("flying").player_fate() {
                PlayerFate::Ejected { .. } => return (true, broke_up),
                PlayerFate::BreakingUp { .. } => broke_up = true,
                _ => {}
            }
        }
        (false, broke_up)
    }

    #[test]
    fn command_and_x_together_hold_the_eject() {
        assert_eq!(EJECT_KEY, Key::Char('x'));
        let mut view = ejecting_view(ejecting_in(100, &[188]), &[Key::Command, EJECT_KEY]);
        assert_eq!(
            until_ejected(&mut view),
            (true, false),
            "before any breakup"
        );
    }

    #[test]
    fn command_alone_or_x_alone_does_not() {
        for keys in [Key::Command, EJECT_KEY] {
            let keys = [keys];
            let mut view = ejecting_view(ejecting_in(100, &[188]), &keys);
            assert_eq!(until_ejected(&mut view), (false, true), "{keys:?}");
        }
    }

    #[test]
    fn an_ejected_player_takes_no_key() {
        let mut view = ejecting_view(ejecting_in(100, &[188, 187]), &[]);
        assert!(until_ejected(&mut view).0);
        view.input(&key(MAP_KEY, true));
        assert!(!view.map_open, "no map");
        view.input(&key(MAP_KEY, false));
        view.input(&key(LAND_KEY, true));
        let session = view.session().expect("flying");
        assert_eq!(session.nav_target(), None, "no landing request");
        assert!(!session.takes(PlayerCommand::Land));
    }

    #[test]
    fn the_pod_is_rescued_after_350_ticks_and_the_rescue_stops_stepping_with_its_desc() {
        let mut catalog = ejecting_in(100, &[188, 187]);
        catalog.home = true;
        let mut view = ejecting_view(catalog, &[]);
        assert!(until_ejected(&mut view).0);
        assert_eq!(view.take_rescue_desc(), None);
        let mut ticks = 0;
        let desc = loop {
            ticks += 1;
            view.tick(TICK);
            if let Some(desc) = view.take_rescue_desc() {
                break desc;
            }
            assert!(ticks <= nova_sim::POD_TICKS, "no rescue");
        };
        assert_eq!(desc, 13999);
        assert_eq!(ticks, nova_sim::POD_TICKS);
        assert_eq!(view.take_rescue_desc(), None, "once");
        let session = view.session().expect("flying");
        assert_eq!(session.ship(), ShipId(128));
        assert_eq!(session.system(), SystemId(128));
        assert_eq!(session.player().position, Vec2::new(50.0, 50.0));
        assert_eq!(
            view.previous.position,
            Vec2::new(50.0, 50.0),
            "laid out where the rescue put it: nothing else stepped"
        );
    }

    #[test]
    fn a_first_tick_eject_rolls_no_puff_for_the_player() {
        let mut view = ejecting_view(ejecting_in(100, &[188, 187]), &[]);
        assert_eq!(until_ejected(&mut view), (true, false));
        let session = view.session().expect("flying");
        let breaking = super::dying(session, &view.sheet, &view.npc_sheets);
        assert_eq!(breaking.len(), 1, "the hulk alone");
        assert_eq!((breaking[0].ticks_left, breaking[0].starting), (100, true));
    }

    /// Steps `view` until the player's ship explodes, calling
    /// `breaking_up` on each frame before that while it breaks up, and
    /// returns with the view on the explosion's tick.
    fn until_the_player_explodes(view: &mut View, breaking_up: impl Fn(&View)) {
        let mut broke_up = false;
        for _ in 0..60 {
            view.tick(TICK);
            match view.session().expect("flying").player_fate() {
                PlayerFate::BreakingUp { .. } => {
                    breaking_up(view);
                    broke_up = true;
                }
                PlayerFate::Exploded => {
                    assert!(broke_up, "it broke up first");
                    return;
                }
                PlayerFate::Flying | PlayerFate::Disabled => {}
                fate @ (PlayerFate::Holding { .. }
                | PlayerFate::Fading
                | PlayerFate::GameOver
                | PlayerFate::Ejected { .. }) => {
                    panic!("past the explosion's tick unseen: {fate:?}")
                }
            }
        }
        panic!("the player's ship never exploded");
    }

    /// The player's layers drawn, by `rlëD`: its ship 2000, glow 2100
    /// and lights 2200 (the NPC's are 2001, 2101 and 2201).
    fn player_layers(view: &View) -> Vec<i16> {
        ship_sprites(view)
            .iter()
            .map(|(image, _)| image.id)
            .filter(|id| [2000, 2100, 2200].contains(id))
            .collect()
    }

    #[test]
    fn the_players_ship_is_drawn_whole_while_breaking_up_then_loses_its_sprite_then_vanishes() {
        // A breakup of one tick, so the engine glow built up by the
        // thrust held from the start has not faded by the explosion: a
        // breaking-up ship takes no controls.
        let mut catalog = killed_in(1);
        catalog.sheet = blinking(Blink::STEADY).sheet;
        let mut view = fighting(catalog, &placed(750, 650, 180));
        // Thrusting with the NPC holding fire, so the glow builds up.
        view.input(&key(Key::Up, true));
        for _ in 0..10 {
            view.tick(TICK);
        }
        let mut view = view.with_behaviour(Rc::new(Firing));
        until_the_player_explodes(&mut view, |view| {
            let ids = player_layers(view);
            assert!(ids.contains(&2000), "the ship: {ids:?}");
            assert!(ids.contains(&2200), "its lights: {ids:?}");
        });
        // At a base of 6 or more the flicker (`glow_level`'s roll of 6,
        // less 4) cannot hide the glow.
        let glow = view.session().expect("flying").engine_glow();
        assert!(glow >= 6, "the glow is still lit as it explodes: {glow}");
        assert_eq!(
            player_layers(&view),
            [2100, 2200],
            "its glow and lights, but no ship, on the explosion's tick"
        );
        // The explosion, `bööm` 128's 3 frames, draws over its 3 ticks.
        for after in 0..5 {
            if after < 3 {
                assert!(!view.effects().explosions().is_empty(), "tick {after}");
                let list = drawn(&view);
                assert!(
                    list.iter().any(sprite_of(400)),
                    "the explosion is drawn on tick {after}: {list:?}"
                );
            }
            view.tick(TICK);
            assert!(
                matches!(fate_of(&view), PlayerFate::Holding { .. }),
                "{:?}",
                fate_of(&view)
            );
            assert_eq!(player_layers(&view), [0_i16; 0], "nothing of it after");
        }
    }

    #[test]
    fn the_explosion_tick_sounds_the_klaxon_then_halts_it_before_the_explosion() {
        // DeathDelay 2: a timer of 6, so 3 ticks of klaxon, then the
        // explosion's.
        let mut view =
            fighting(killed_in(2), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        let klaxon = Sound::Sim(SimSound::Klaxon);
        let mut breaking_up = Vec::new();
        for _ in 0..60 {
            view.tick(TICK);
            let sounds = view.take_sounds();
            match view.session().expect("flying").player_fate() {
                PlayerFate::BreakingUp { .. } => {
                    breaking_up.push(sounds.iter().filter(|&&sound| sound == klaxon).count());
                }
                PlayerFate::Exploded => {
                    assert_eq!(breaking_up, [0, 1, 1, 1], "none as it starts");
                    let first_combat = sounds
                        .iter()
                        .position(|sound| matches!(sound, Sound::Combat(_)));
                    let klaxons: Vec<usize> = (0..sounds.len())
                        .filter(|&at| sounds[at] == klaxon)
                        .collect();
                    assert!(first_combat.is_some(), "the explosion sounds: {sounds:?}");
                    assert_eq!(klaxons.len(), 1, "{sounds:?}");
                    let halt = Sound::Sim(SimSound::Halt(&nova_sim::PLAYER_EXPLOSION_HALTS));
                    let halts: Vec<usize> =
                        (0..sounds.len()).filter(|&at| sounds[at] == halt).collect();
                    assert_eq!(halts, [klaxons[0] + 1], "halted next: {sounds:?}");
                    assert!(halts[0] < first_combat.unwrap_or(0), "{sounds:?}");
                    return;
                }
                PlayerFate::Flying | PlayerFate::Disabled => {}
                fate @ (PlayerFate::Holding { .. }
                | PlayerFate::Fading
                | PlayerFate::GameOver
                | PlayerFate::Ejected { .. }) => {
                    panic!("past the explosion's tick unseen: {fate:?}")
                }
            }
        }
        panic!("the player's ship never exploded");
    }

    /// A flight under fire from [`killed_in`]`(30)`'s NPC, stepped until
    /// the player's ship breaks up: a breakup of 90 ticks, so it is still
    /// breaking up for the test.
    fn breaking_up() -> View {
        let mut view =
            fighting(killed_in(30), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        for _ in 0..60 {
            view.tick(TICK);
            if matches!(fate_of(&view), PlayerFate::BreakingUp { .. }) {
                return view;
            }
        }
        panic!("the player's ship never broke up");
    }

    fn fate_of(view: &View) -> PlayerFate {
        view.session().expect("flying").player_fate()
    }

    fn breakup_ticks_left(view: &View) -> u32 {
        match fate_of(view) {
            PlayerFate::BreakingUp { ticks_left, .. } => ticks_left,
            fate => panic!("not breaking up: {fate:?}"),
        }
    }

    #[test]
    fn the_map_does_not_open_while_the_player_breaks_up() {
        let mut view = breaking_up();
        let before = breakup_ticks_left(&view);
        tap(&mut view, MAP_KEY);
        assert!(!view.map_open());
        view.tick(TICK);
        assert!(
            breakup_ticks_left(&view) < before,
            "the flight goes on, unpaused"
        );
    }

    #[test]
    fn mission_info_and_landing_do_nothing_while_the_player_breaks_up() {
        let mut view = breaking_up();
        let nav = nav_target(&view);
        let _ = view.take_sounds();
        tap(&mut view, MISSION_INFO_KEY);
        assert_eq!(view.take_sounds(), [], "silently (@0x70445, @0x7044f)");
        tap(&mut view, LAND_KEY);
        assert!(view.take_mission_info_request().is_none());
        assert_eq!(view.take_landing(), None);
        assert_ne!(view.message(), Some(NO_ACTIVE_MISSIONS));
        assert_eq!(nav_target(&view), nav);
    }

    #[test]
    fn targeting_and_escort_keys_still_work_while_the_player_breaks_up() {
        let mut view = breaking_up();
        assert_eq!(target_of(&view), None);
        tap(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), Some(NpcId(0)));
        tap(&mut view, MENU_KEY);
        assert_eq!(
            view.message(),
            Some(NO_ESCORTS),
            "the escort menu's key is taken"
        );
    }

    /// A flight under fire from [`killed_in`]`(1)`'s NPC, stepped to the
    /// player's explosion and one tick past it.
    fn exploded() -> View {
        let mut view =
            fighting(killed_in(1), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        until_the_player_explodes(&mut view, |_| {});
        view.tick(TICK);
        view
    }

    #[test]
    fn an_exploded_player_takes_no_command() {
        let mut view = exploded();
        let target = target_of(&view);
        let nav = nav_target(&view);
        for k in [MAP_KEY, TARGET_KEY, LAND_KEY, MISSION_INFO_KEY, MENU_KEY] {
            tap(&mut view, k);
        }
        assert!(!view.map_open());
        assert_eq!(target_of(&view), target);
        assert_eq!(nav_target(&view), nav);
        assert!(view.take_mission_info_request().is_none());
        assert_eq!(view.take_landing(), None);
        assert_ne!(view.message(), Some(NO_ACTIVE_MISSIONS));
        assert_ne!(view.message(), Some(NO_ESCORTS));
    }

    /// Where `view` draws [`catalog`]'s stellar 129, checked to be in the
    /// space a click picks from, clear of the status panel.
    fn stellar_129_on_screen(view: &View) -> Point {
        let drawn = on_screen(view, (300.0, -200.0));
        assert!(
            (0.0..VIEW_SIZE.0 - STATUS_BAR_WIDTH).contains(&drawn.x)
                && (0.0..VIEW_SIZE.1).contains(&drawn.y),
            "{drawn:?} is in space"
        );
        drawn
    }

    #[test]
    fn a_click_still_picks_a_stellar_while_the_player_breaks_up() {
        let mut view = breaking_up();
        assert_eq!(nav_target(&view), None);
        let stellar = stellar_129_on_screen(&view);
        click(&mut view, stellar);
        assert_eq!(nav_target(&view), Some(StellarId(129)));
    }

    #[test]
    fn a_click_picks_no_stellar_once_the_player_has_exploded() {
        let mut view = exploded();
        let nav = nav_target(&view);
        assert_ne!(nav, Some(StellarId(129)));
        let stellar = stellar_129_on_screen(&view);
        click(&mut view, stellar);
        assert_eq!(nav_target(&view), nav);
    }

    /// Home (`Key::Other` on this platform) has no binding, so it does
    /// nothing in every state, including the dead one the original reads it
    /// in (0x6884c, `_ResetPlayer(1)`, left out). This pins the unbound key,
    /// not the dead-state lockout, which `an_exploded_player_takes_no_command`
    /// and `a_click_picks_no_stellar_once_the_player_has_exploded` cover.
    #[test]
    fn home_is_unbound_whether_flying_or_exploded() {
        let mut flying =
            fighting(killed_in(1), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        assert!(matches!(fate_of(&flying), PlayerFate::Flying));
        assert_eq!(flying.input(&key(Key::Other, true)), ScreenAction::None);
        assert_eq!(flying.input(&key(Key::Other, false)), ScreenAction::None);

        let mut view = exploded();
        let pilot = view.pilot().cloned();
        assert_eq!(view.input(&key(Key::Other, true)), ScreenAction::None);
        assert_eq!(view.input(&key(Key::Other, false)), ScreenAction::None);
        assert!(matches!(fate_of(&view), PlayerFate::Holding { .. }));
        assert!(view.take_mission_info_request().is_none());
        assert_eq!(view.take_landing(), None);
        assert!(view.take_hail().is_none());
        assert!(view.take_boarding().is_none());
        assert_eq!(view.pilot().cloned(), pilot);
    }

    /// A flight under fire from [`killed_in`]`(1)`'s NPC, stepped to the
    /// player's explosion's tick.
    fn at_the_explosion() -> View {
        let mut view =
            fighting(killed_in(1), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        until_the_player_explodes(&mut view, |_| {});
        view
    }

    /// The shots in flight in `view`'s session.
    fn shots_in(view: &View) -> Vec<nova_sim::combat::projectile::Shot> {
        view.session().expect("flying").shots().to_vec()
    }

    /// The alpha of the black covering the whole screen, if the last
    /// thing `view` draws is one.
    fn black_over(view: &View) -> Option<u8> {
        match drawn(view).iter().last() {
            Some(DrawCommand::Line { width, color, .. })
                if *width == VIEW_SIZE.1 && (color.r, color.g, color.b) == (0, 0, 0) =>
            {
                Some(color.a)
            }
            _ => None,
        }
    }

    /// [`at_the_explosion`], stepped one tick at a time through the hold
    /// to its expiry, which the view reports.
    fn held_to_expiry() -> View {
        let mut view = at_the_explosion();
        for _ in 0..=nova_sim::HOLD_TICKS {
            view.tick(TICK);
        }
        assert!(view.take_hold_expired());
        view
    }

    #[test]
    fn after_the_explosion_the_scene_runs_on_without_the_ship_until_the_hold_expires() {
        let mut view = at_the_explosion();
        let mut shots_moved = 0;
        for tick in 1..=nova_sim::HOLD_TICKS {
            let before = shots_in(&view);
            view.tick(TICK);
            assert_eq!(player_layers(&view), [0_i16; 0], "dead tick {tick}");
            assert!(
                ship_sprites(&view)
                    .iter()
                    .any(|(image, _)| image.id == 2001),
                "the NPC is drawn on dead tick {tick}"
            );
            assert!(!view.take_hold_expired(), "dead tick {tick}");
            if shots_in(&view) != before {
                shots_moved += 1;
            }
        }
        assert_eq!(shots_moved, nova_sim::HOLD_TICKS, "the fight runs on");
        assert_eq!(
            fate_of(&view),
            PlayerFate::Holding { ticks_left: 0 },
            "no ticks left, but not expired"
        );
        assert!(!view.game_over());
        let before = shots_in(&view);
        view.tick(TICK);
        assert_ne!(shots_in(&view), before, "the 243rd step runs");
        assert!(view.take_hold_expired(), "and the hold expires on it");
        assert!(!view.take_hold_expired(), "once");
    }

    #[test]
    fn a_long_frame_stops_stepping_on_the_hold_expirys_step() {
        let stepped = held_to_expiry();
        let mut view = at_the_explosion();
        for _ in 0..nova_sim::HOLD_TICKS - 2 {
            view.tick(TICK);
        }
        view.tick(TICK * 8);
        assert!(view.take_hold_expired());
        assert_eq!(shots_in(&view), shots_in(&stepped), "no step after it");
        assert_eq!(
            view.session().expect("flying").npcs(),
            stepped.session().expect("flying").npcs()
        );
    }

    #[test]
    fn at_hold_expiry_flight_stops_and_nothing_fades_until_the_fade_begins() {
        let mut view = held_to_expiry();
        let shots = shots_in(&view);
        for _ in 0..30 {
            view.tick(TICK);
            assert_eq!(shots_in(&view), shots, "nothing moves");
            assert_eq!(black_over(&view), None, "nothing fades");
            assert_eq!(fate_of(&view), PlayerFate::Holding { ticks_left: 0 });
            assert!(!view.game_over());
        }
        view.begin_fade();
        assert_eq!(fate_of(&view), PlayerFate::Fading);
    }

    #[test]
    fn the_death_fade_goes_to_black_over_a_second_then_the_game_is_over() {
        let mut view = held_to_expiry();
        view.begin_fade();
        let shots = shots_in(&view);
        view.tick(Duration::from_millis(500));
        assert_eq!(black_over(&view), Some(128), "half way, over the HUD");
        assert!(!view.game_over());
        assert_eq!(shots_in(&view), shots, "the scene is frozen under it");
        view.tick(Duration::from_millis(499));
        assert!(!view.game_over());
        assert_eq!(fate_of(&view), PlayerFate::Fading);
        view.tick(Duration::from_millis(1));
        assert_eq!(black_over(&view), Some(255));
        assert!(view.game_over());
        assert_eq!(fate_of(&view), PlayerFate::GameOver);
        assert_eq!(shots_in(&view), shots);
    }

    #[test]
    fn beginning_the_fade_before_the_hold_expires_does_nothing() {
        let mut view = at_the_explosion();
        view.tick(TICK);
        view.begin_fade();
        view.tick(Duration::from_millis(1500));
        assert_eq!(black_over(&view), None);
        assert!(!view.game_over());
        assert!(matches!(fate_of(&view), PlayerFate::Holding { .. }));
        while !view.take_hold_expired() {
            view.tick(TICK);
        }
        view.tick(Duration::from_millis(1500));
        assert_eq!(black_over(&view), None, "nor once the hold expires");
        assert!(!view.game_over());
    }

    #[test]
    fn only_a_living_pilot_may_be_saved() {
        let flying = fighting(killed_in(1), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        assert!(flying.may_save());
        assert!(!at_the_explosion().may_save());
        let mut failed = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..killed_in(1)
        });
        assert!(failed.session().is_err());
        assert!(!failed.may_save(), "no session, no pilot to save");
        assert!(!failed.take_hold_expired());
        assert!(!failed.game_over());
        failed.begin_fade();
        failed.tick(Duration::from_secs(2));
        assert!(!failed.game_over());
    }

    #[test]
    fn only_a_strict_pilot_ending_play_dying_is_deleted() {
        let strict = |catalog: FakeCatalog| {
            let pilot = Pilot::new(&catalog, "Ada")
                .expect("starts")
                .with_strict_play(true);
            let (_, chance) = scripted(&placed(750, 650, 180));
            let mut view = FlightView::with_pilot(catalog, pilot)
                .with_chance(chance)
                .with_behaviour(Rc::new(Firing));
            view.tick(TICK);
            view
        };
        let mut view = strict(killed_in(1));
        assert!(!view.deletes_pilot_on_ending_play(), "flying");
        until_the_player_explodes(&mut view, |_| {});
        assert!(view.deletes_pilot_on_ending_play(), "exploded");
        assert!(
            !at_the_explosion().deletes_pilot_on_ending_play(),
            "not a strict pilot"
        );
        let failed = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..killed_in(1)
        });
        assert!(
            !failed.deletes_pilot_on_ending_play(),
            "no session, no pilot to delete"
        );
    }

    /// Whether `list` draws `run` as one unbroken run of commands.
    fn draws_run(list: &DrawList, run: &DrawList) -> bool {
        let commands: Vec<_> = list.iter().cloned().collect();
        let expected: Vec<_> = run.iter().cloned().collect();
        commands
            .windows(expected.len())
            .any(|window| window == expected)
    }

    fn sprite_unavailable(list: &DrawList) -> bool {
        list.iter().any(
            |c| matches!(c, DrawCommand::Text { text, .. } if text.starts_with("Sprite unavailable")),
        )
    }

    #[test]
    fn an_exploded_player_without_a_sheet_draws_no_placeholder() {
        let mut catalog = killed_in(5);
        catalog.sheet = Err("no shän 128 for shïp 128".to_owned());
        let mut view = fighting(catalog, &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        let placeholder = |view: &View| {
            let mut boxed = DrawList::new();
            let at = view.camera().world_to_screen(view.shown_position());
            crossed_box(&mut boxed, at, PLACEHOLDER_SIZE, PLACEHOLDER);
            boxed
        };
        until_the_player_explodes(&mut view, |view| {
            let list = drawn(view);
            assert!(sprite_unavailable(&list), "while breaking up");
            assert!(draws_run(&list, &placeholder(view)), "{list:?}");
        });
        // The explosion's tick, then the ones after.
        for _ in 0..3 {
            let list = drawn(&view);
            assert!(!sprite_unavailable(&list), "{list:?}");
            assert!(!draws_run(&list, &placeholder(&view)), "{list:?}");
            view.tick(TICK);
        }
    }

    #[test]
    fn a_shot_whose_look_cannot_be_read_is_a_crossed_box() {
        let mut view = fighting(armed(0, &[UNSEEN]), &[]);
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        let list = drawn(&view);
        let shot = view.session().expect("flying").shots()[0];
        let mut boxed = DrawList::new();
        // A whole step on, it is drawn a step behind, where it left.
        assert_eq!(view.alpha(), 0.0);
        let left = shot.position - shot.velocity;
        let centre = view.camera().world_to_screen(at(left.x, left.y));
        crossed_box(
            &mut boxed,
            centre,
            weapons::SHOT_PLACEHOLDER_SIZE,
            PLACEHOLDER,
        );
        let commands: Vec<_> = list.iter().cloned().collect();
        let expected: Vec<_> = boxed.iter().cloned().collect();
        assert!(
            commands.windows(expected.len()).any(|run| run == expected),
            "{commands:?}"
        );
    }

    #[test]
    fn a_destroyed_npc_explodes_and_scatters_debris_on_the_frames_after() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        let mut view = fighting(catalog, &placed(750, 650, 180));
        tap(&mut view, Key::Tab);
        // Drifting up, so the explosion is heard from where the player is.
        view.input(&key(Key::Up, true));
        view.tick(TICK);
        view.input(&key(Key::Up, false));
        view.input(&key(FIRE_KEY, true));
        let mut destroyed = false;
        for _ in 0..10 {
            view.tick(TICK);
            if view.session().expect("flying").npcs().is_empty() {
                destroyed = true;
                break;
            }
        }
        assert!(destroyed, "{:?}", view.session().expect("flying").npcs());
        assert_eq!(target_of(&view), None);
        let effects = view.effects();
        assert_eq!(effects.explosions().len(), 1);
        assert_eq!(effects.explosions()[0].boom, BoomId(128));
        assert_eq!(effects.particles().len(), 16);
        let list = drawn(&view);
        let explosion = first(&list, sprite_of(400));
        assert_eq!(
            list.iter()
                .skip(explosion)
                .filter(|c| matches!(c, DrawCommand::Dot { color, .. } if *color == DEBRIS_COLOR))
                .count(),
            16,
            "drawn after the explosion"
        );
        let player = view.session().expect("flying").player().position;
        assert!(player.y < 0.0, "{player:?}");
        let boom = view.effects().explosions()[0].at;
        let heard = CombatSound {
            sound: nova_sim::SoundId(302),
            offset: (
                (boom.x - player.x).round() as i32,
                (boom.y - player.y).round() as i32,
            ),
        };
        assert!(
            combat_sounds(&mut view).contains(&heard),
            "the explosion is heard: {heard:?}"
        );
    }

    #[test]
    fn an_npc_destroyed_under_the_originals_debris_reading_scatters_none() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        let rules = Rulebook::default().with_override(RuleKey::ShipDebris, RuleSource::Engine);
        let mut view = fighting(catalog, &placed(750, 650, 180)).with_rules(rules);
        tap(&mut view, Key::Tab);
        view.input(&key(FIRE_KEY, true));
        for _ in 0..10 {
            view.tick(TICK);
            if view.session().expect("flying").npcs().is_empty() {
                break;
            }
        }
        assert!(view.session().expect("flying").npcs().is_empty());
        let effects = view.effects();
        assert_eq!(effects.explosions().len(), 1, "its Explode2 still");
        assert_eq!(effects.explosions()[0].boom, BoomId(128));
        assert_eq!(effects.particles(), [], "no debris");
    }

    /// [`armed`] with a blaster whose hits throw off 5 orange particles
    /// of `HitPartLife` 30, flown once at NPC 0 ahead: the flight on the
    /// tick its shot hits, and the NPC.
    fn hit_once() -> (View, ShipRef) {
        hit_once_by(5, Rulebook::default())
    }

    /// [`hit_once`] with `particles` thrown off, by `rules`.
    fn hit_once_by(particles: i16, rules: Rulebook) -> (View, ShipRef) {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].energy_dmg = 1;
        catalog.weapons[0].explod_type = -1;
        catalog.weapons[0].hit_particles = particles;
        catalog.weapons[0].hit_part_vel = 100;
        catalog.weapons[0].hit_part_life = 30;
        catalog.weapons[0].hit_part_color = 0x00FF_8000;
        let mut view = fighting(catalog, &placed(750, 650, 180)).with_rules(rules);
        tap(&mut view, Key::Tab);
        let npc = ShipRef::Npc(view.session().expect("flying").npcs()[0].id);
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        for _ in 0..20 {
            if view.shield_flash(npc).is_some() {
                return (view, npc);
            }
            view.tick(TICK);
        }
        panic!("never hit: {:?}", view.session().expect("flying").npcs());
    }

    #[test]
    fn an_npc_hit_flashes_its_shield_32_on_the_tick_of_the_hit_then_31() {
        let (mut view, npc) = hit_once();
        assert_eq!(view.shield_flash(npc), Some(32.0));
        view.tick(TICK);
        assert_eq!(view.shield_flash(npc), Some(31.0));
        assert_eq!(view.shield_flash(ShipRef::Player), None, "not hit");
    }

    #[test]
    fn a_hit_with_hit_particles_sprays_them_on_the_npc_in_its_colour() {
        let (view, _) = hit_once();
        let orange = crate::Color::rgba(0xFF, 0x80, 0x00, 255);
        let particles = view.effects().particles();
        assert_eq!(particles.len(), 5, "{particles:?}");
        assert!(particles.iter().all(|piece| piece.color == orange));
    }

    #[test]
    fn under_the_combat_caps_a_hit_keeps_at_most_1024_particles() {
        let (view, _) = hit_once_by(1100, Rulebook::default());
        assert_eq!(
            view.effects().particles().len(),
            1100,
            "uncapped by default"
        );
        let caps = Rulebook::default().with_override(RuleKey::CombatCaps, RuleSource::Engine);
        let (view, _) = hit_once_by(1100, caps);
        assert_eq!(view.effects().particles().len(), 1024);
    }

    #[test]
    fn a_beams_hit_flashes_the_npcs_shield_from_the_frame_after() {
        let mut catalog = armed(1, &[UNDER]);
        catalog.weapons[4].beam_length = 400;
        catalog.weapons[4].energy_dmg = 1;
        let mut view = fighting(catalog, &placed(750, 650, 180));
        tap(&mut view, Key::Tab);
        let shield = |view: &View| {
            view.session().expect("flying").npcs()[0]
                .reserves
                .shield
                .now
        };
        let full = shield(&view);
        let npc = ShipRef::Npc(view.session().expect("flying").npcs()[0].id);
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        let mut ticks = 0;
        while shield(&view) == full {
            assert!(ticks < 20, "never hit");
            view.tick(TICK);
            ticks += 1;
        }
        assert_eq!(view.shield_flash(npc), None, "shown the frame after");
        view.tick(TICK);
        assert_eq!(view.shield_flash(npc), Some(32.0));
    }

    #[test]
    fn a_destroyed_npcs_shield_flash_is_dropped() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].energy_dmg = 20;
        catalog.weapons[0].mass_dmg = 20;
        catalog.weapons[0].explod_type = -1;
        let mut view = fighting(catalog, &placed(750, 650, 180));
        tap(&mut view, Key::Tab);
        let npc = ShipRef::Npc(view.session().expect("flying").npcs()[0].id);
        view.input(&key(FIRE_KEY, true));
        let mut flashed = false;
        for _ in 0..20 {
            view.tick(TICK);
            if view.session().expect("flying").npcs().is_empty() {
                break;
            }
            flashed |= view.shield_flash(npc) == Some(32.0);
        }
        assert!(
            view.session().expect("flying").npcs().is_empty(),
            "destroyed"
        );
        assert!(flashed, "hit before it was destroyed");
        assert_eq!(view.shield_flash(npc), None);
        view.tick(TICK);
        assert_eq!(view.shield_flash(npc), None, "nor on the next tick");
    }

    /// [`armed`] with a blaster whose shots detonate after a tick.
    fn detonating() -> FakeCatalog {
        let mut catalog = armed(0, &[BLASTER]);
        catalog.weapons[0].flags = 0x8000;
        catalog.weapons[0].count = 1;
        catalog
    }

    /// Fires one detonating shot, which explodes as its end of life comes,
    /// the tick after its one move.
    fn detonate(view: &mut View) {
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        view.tick(TICK);
        assert_eq!(view.effects().explosions().len(), 1);
    }

    #[test]
    fn landing_clears_the_effects() {
        let mut catalog = detonating();
        catalog.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        let mut view = fighting(catalog, &[]);
        detonate(&mut view);
        land_now(&mut view);
        assert!(view.take_landing().is_some());
        assert_eq!(*view.effects(), Effects::default());
    }

    #[test]
    fn arriving_clears_the_effects() {
        let mut view = fighting(detonating(), &[]);
        plot(&mut view, 131);
        fly_out(&mut view);
        detonate(&mut view);
        jump_now(&mut view);
        arrive_now(&mut view);
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
        assert_eq!(*view.effects(), Effects::default());
    }

    /// [`detonating`] with a blast of radius 100 doing 1 energy damage,
    /// which reaches the player who fired it.
    fn self_blasting() -> FakeCatalog {
        let mut catalog = detonating();
        catalog.weapons[0].blast_radius = 100;
        catalog.weapons[0].energy_dmg = 1;
        catalog
    }

    /// Fires one [`self_blasting`] shot and ticks until its blast flashes
    /// the player's shield.
    fn flash_the_player(view: &mut View) {
        detonate(view);
        for _ in 0..5 {
            if view.shield_flash(ShipRef::Player).is_some() {
                return;
            }
            view.tick(TICK);
        }
        panic!("the player was never hit: {:?}", player(view));
    }

    #[test]
    fn landing_clears_the_shield_flashes() {
        let mut catalog = self_blasting();
        catalog.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        let mut view = fighting(catalog, &[]);
        flash_the_player(&mut view);
        land_now(&mut view);
        assert!(view.take_landing().is_some());
        assert_eq!(view.shield_flash(ShipRef::Player), None);
    }

    #[test]
    fn passing_through_a_wormhole_clears_the_shield_flashes() {
        let holes = holed();
        let catalog = FakeCatalog {
            sites: holes.sites,
            gates: holes.gates,
            gate_listers: holes.gate_listers,
            ..self_blasting()
        };
        let mut view = fighting(catalog, &[]);
        flash_the_player(&mut view);
        land_now(&mut view);
        assert_eq!(system_of(&view), SystemId(131));
        assert_eq!(view.shield_flash(ShipRef::Player), None);
    }

    #[test]
    fn the_target_is_bracketed_and_its_panel_shows_it() {
        let view = firing_at_an_npc();
        let session = view.session().expect("flying");
        let npc = &session.npcs()[0];
        let list = drawn(&view);
        let (shown, _) = view.shown_npc(npc);
        let Some(Ok(sheet)) = view.npc_sheets.get(&npc.ship) else {
            panic!("a sheet");
        };
        let mut expected = DrawList::new();
        brackets::draw_ship(
            &mut expected,
            view.camera().world_to_screen(shown),
            Some((sheet.frame_width, sheet.frame_height)),
            0.0,
            Standing::Neutral,
        );
        let commands: Vec<_> = list.iter().cloned().collect();
        let expected: Vec<_> = expected.iter().cloned().collect();
        assert!(
            commands.windows(4).any(|run| run == expected),
            "{commands:?}"
        );
        let bar = view.status_bar().expect("a status bar");
        let mut panel = DrawList::new();
        target::draw_target_panel(
            &mut panel,
            &bar.layout,
            hud::bar_origin(bar),
            Some(&TargetPanel {
                name: "Ship 129".to_owned(),
                subtitle: "Light Transport".to_owned(),
                picture: Some(3001),
                status: nova_sim::TargetStatus::Shield(100),
                tag: Some(Tag::Code("Fed.".to_owned())),
            }),
            None,
        );
        let expected: Vec<_> = panel.iter().cloned().collect();
        assert!(
            commands.windows(expected.len()).any(|run| run == expected),
            "{commands:?}"
        );
    }

    #[test]
    fn the_nav_brackets_draw_under_the_ship_brackets() {
        let mut view = firing_at_an_npc();
        tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
        let list = drawn(&view);
        let nav = first(&list, icon_in(10000..=10007));
        let ship = first(&list, icon_in(10008..=10023));
        let npc = first(&list, sprite_of(2001));
        assert!(npc < nav && nav + 4 == ship, "{npc} {nav} {ship}");
        assert_eq!(
            bracket_icons(&list),
            [10000, 10001, 10002, 10003, 10012, 10013, 10014, 10015]
        );
    }

    #[test]
    fn a_disabled_targets_brackets_are_grey() {
        let catalog = armed(1, &[]);
        let mut view =
            fighting(catalog, &placed(750, 650, 180)).with_disable_rule(Rc::new(Disabling));
        tap(&mut view, Key::Tab);
        view.tick(TICK);
        let session = view.session().expect("flying");
        assert_eq!(session.npcs()[0].condition, Condition::Disabled);
        let list = drawn(&view);
        assert_eq!(bracket_icons(&list), [10020, 10021, 10022, 10023]);
        assert!(texts(&list).contains(&target::DISABLED.to_owned()));
    }

    /// The ship bracket `cïcn`s drawn (10008-10023), and where.
    fn ship_bracket_corners(view: &View) -> Vec<(i16, Point)> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Picture { image, top_left } if icon_in(10008..=10023)(command) => {
                    Some((image.id, *top_left))
                }
                _ => None,
            })
            .collect()
    }

    /// Where `view` draws its target's sprite.
    fn target_on_screen(view: &View) -> Point {
        let session = view.session().expect("flying");
        let npc = session.target().expect("a target");
        view.camera().world_to_screen(view.shown_npc(npc).0)
    }

    #[test]
    fn targeting_a_ship_on_screen_flies_its_brackets_in_then_settles_them() {
        let mut view = fighting(armed(1, &[]), &placed(750, 650, 180));
        assert_eq!(ship_bracket_corners(&view), []);
        tap(&mut view, Key::Tab);
        // Ship 129, 40 x 40: half 20; neutral, from 10012.
        assert_eq!(
            ship_bracket_corners(&view),
            corners_at(10012, target_on_screen(&view), 20.0 + 196.0),
            "a frame drawn before the next tick"
        );
        for expand in [196.0, 136.0, 76.0, 16.0, 0.0, 0.0] {
            view.tick(TICK);
            assert_eq!(
                ship_bracket_corners(&view),
                corners_at(10012, target_on_screen(&view), 20.0 + expand),
                "{expand}"
            );
        }
    }

    #[test]
    fn a_target_whose_sheet_cannot_be_read_is_bracketed_round_its_crossed_box() {
        let mut view = fighting(trafficked(&[130], 1, 130, 3), &placed(850, 650, 90));
        tap(&mut view, Key::Tab);
        ticks(&mut view, 5);
        // Its 64 x 64 box: half 32.
        assert_eq!(
            ship_bracket_corners(&view),
            corners_at(10012, target_on_screen(&view), PLACEHOLDER_SIZE / 2.0)
        );
    }

    /// What the target panel's status line shows this frame, if there is
    /// a target.
    fn status_shown(view: &View) -> Option<nova_sim::TargetStatus> {
        view.target_shown()
            .as_ref()
            .as_ref()
            .map(|panel| panel.status)
    }

    /// What the target panel's status line would show of the target now.
    fn status_now(view: &View) -> Option<nova_sim::TargetStatus> {
        view.target_now().map(|panel| panel.status)
    }

    /// An unarmed flight targeting its one NPC, intact, its panel drawn
    /// two ticks after a timed redraw.
    fn targeting_after_a_redraw(view: View) -> View {
        let mut view = view;
        while now_tick(&view) % TARGET_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        tap(&mut view, Key::Tab);
        view.tick(TICK);
        assert_eq!(status_shown(&view), status_now(&view));
        assert!(
            matches!(status_shown(&view), Some(nova_sim::TargetStatus::Shield(_))),
            "{:?}",
            status_shown(&view)
        );
        view
    }

    #[test]
    fn the_target_panel_holds_a_disabling_until_its_next_timed_redraw() {
        let view = fighting(armed(1, &[]), &placed(750, 650, 180));
        let view = targeting_after_a_redraw(view);
        let held = status_shown(&view);
        let mut view = view.with_disable_rule(Rc::new(Disabling));
        view.tick(TICK);
        assert_eq!(status_now(&view), Some(nova_sim::TargetStatus::Disabled));
        while !now_tick(&view).is_multiple_of(TARGET_PERIOD_TICKS) {
            assert_eq!(status_shown(&view), held, "tick {}", now_tick(&view));
            assert!(!texts(&drawn(&view)).contains(&target::DISABLED.to_owned()));
            view.tick(TICK);
        }
        assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
        assert!(texts(&drawn(&view)).contains(&target::DISABLED.to_owned()));
    }

    #[test]
    fn read_the_bibles_way_the_target_panel_follows_every_frame() {
        let view = every_frame(fighting(armed(1, &[]), &placed(750, 650, 180)));
        let view = targeting_after_a_redraw(view);
        let mut view = view.with_disable_rule(Rc::new(Disabling));
        view.tick(TICK);
        assert_ne!(now_tick(&view) % TARGET_PERIOD_TICKS, 0);
        assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
    }

    #[test]
    fn redrawing_the_status_retakes_the_target_panel() {
        let view = fighting(armed(1, &[]), &placed(750, 650, 180));
        let view = targeting_after_a_redraw(view);
        let mut view = view.with_disable_rule(Rc::new(Disabling));
        view.tick(TICK);
        assert_ne!(status_shown(&view), status_now(&view), "held");
        view.redraw_status();
        assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
    }

    #[test]
    fn a_change_of_target_shows_at_once() {
        let mut view = fighting(armed(2, &[]), &two_npcs());
        tap(&mut view, Key::Tab);
        view.tick(TICK);
        let drawn_for = |view: &View| {
            view.target_scene
                .as_ref()
                .and_then(|(drawn, _)| drawn.map(|drawn| drawn.npc))
        };
        assert_eq!(drawn_for(&view), Some(NpcId(0)));
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        assert_eq!(drawn_for(&view), Some(NpcId(0)), "not yet retaken");
        assert_eq!(*view.target_shown(), view.target_now(), "shown at once");
        view.tick(TICK);
        assert_eq!(drawn_for(&view), Some(NpcId(1)), "retaken");
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), None);
        assert_eq!(*view.target_shown(), None, "lost at once");
        let list = texts(&drawn(&view));
        assert!(list.contains(&NO_TARGET.to_owned()), "{list:?}");
    }

    #[test]
    fn a_falling_shield_shows_at_the_radar_refreshes_only() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].energy_dmg = 1;
        let mut view = fighting(catalog, &placed(750, 650, 180));
        tap(&mut view, Key::Tab);
        view.input(&key(FIRE_KEY, true));
        let mut shown = status_shown(&view);
        let mut changes = 0;
        let mut held = 0;
        for _ in 0..240 {
            let (blink, before) = (view.hud_cadence.blink_on(), now_tick(&view));
            view.tick(TICK);
            let radar = view.hud_cadence.blink_on() != blink;
            let timed = now_tick(&view) / TARGET_PERIOD_TICKS > before / TARGET_PERIOD_TICKS;
            let now = status_shown(&view);
            if now != shown {
                changes += 1;
                assert!(radar || timed, "tick {}: {now:?}", now_tick(&view));
                assert_eq!(now, status_now(&view), "tick {}", now_tick(&view));
            }
            if now != status_now(&view) {
                held += 1;
            }
            shown = now;
            if target_of(&view).is_none() {
                break;
            }
        }
        view.input(&key(FIRE_KEY, false));
        assert!(changes > 1, "the shield fell: {changes}");
        assert!(held > 0, "and was held between refreshes");
    }

    #[test]
    fn a_disabled_player_has_no_fuel_gauge() {
        let mut catalog = armed(1, &[]);
        catalog.hulls[1] = hull_of(129, &[BLASTER]);
        let mut view = fighting(catalog, &placed(750, 650, 180))
            .with_behaviour(Rc::new(Firing))
            .with_disable_rule(Rc::new(Disabling));
        let colors = |view: &View| -> Vec<Color> {
            lines(&drawn(view))
                .into_iter()
                .filter_map(|command| match command {
                    DrawCommand::Line { color, .. } => Some(color),
                    _ => None,
                })
                .collect()
        };
        let layout = layout();
        assert!(colors(&view).contains(&layout.fuel_full), "intact");
        for _ in 0..40 {
            if view.session().expect("flying").player_condition() == Condition::Disabled {
                break;
            }
            view.tick(TICK);
        }
        assert_eq!(
            view.session().expect("flying").player_condition(),
            Condition::Disabled
        );
        let shown = colors(&view);
        assert!(!shown.contains(&layout.fuel_full), "{shown:?}");
        assert!(!shown.contains(&layout.fuel_partial), "{shown:?}");
        assert!(shown.contains(&layout.armor_color), "{shown:?}");
    }

    /// Disables every ship.
    #[derive(Debug)]
    struct Disabling;

    impl DisableRule for Disabling {
        fn disabled(&self, _armor: nova_sim::Gauge, _hull: &nova_sim::HullSpec) -> bool {
            true
        }
    }

    #[test]
    fn the_hud_shows_no_target_and_the_secondary_weapon() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET]), &[]);
        let list = texts(&drawn(&view));
        assert!(list.contains(&NO_TARGET.to_owned()), "{list:?}");
        assert!(list.contains(&NO_SECONDARY.to_owned()), "{list:?}");
        tap(&mut view, SELECT_KEY);
        let list = texts(&drawn(&view));
        assert!(list.contains(&"Rocket".to_owned()), "{list:?}");
        let unarmed = texts(&drawn(&fighting(armed(0, &[BLASTER]), &[])));
        assert!(unarmed.contains(&NO_SECONDARY.to_owned()), "{unarmed:?}");
    }

    #[test]
    fn the_panel_is_laid_out_by_the_metrics_given() {
        let view = fighting(armed(0, &[BLASTER]), &[])
            .with_metrics(Rc::new(crate::text::fixture::MonoMetrics));
        let list = drawn(&view);
        let origin = list
            .iter()
            .find_map(|c| match c {
                DrawCommand::Text { text, origin, .. } if text == NO_TARGET => Some(*origin),
                _ => None,
            })
            .expect("No Target");
        // Centred: 9 characters of 6 in the 176 across from 838.
        assert_eq!(origin.x, 899.0);
    }

    #[test]
    fn the_effects_roll_on_their_own_chance() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        let (_, chance) = scripted(&placed(750, 650, 180));
        let (effects, effects_chance) = scripted(&[]);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_effects_chance(effects_chance);
        view.tick(TICK);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 10);
        assert!(view.session().expect("flying").npcs().is_empty());
        assert_eq!(
            effects.borrow().asked,
            [360, 101, 31].repeat(16),
            "the debris, drawn on the effects' own chance"
        );
    }

    #[test]
    fn a_looped_weapon_is_heard_again_only_once_its_sound_has_played_out() {
        let mut catalog = armed(0, &[UNDER]);
        catalog.weapons[4].reload = 0;
        catalog.weapons[4].count = 5;
        catalog.looks[4].1 = Ok(WeaponLook {
            flags: weapons::LOOPED_SOUND,
            sound: Some(nova_sim::SoundId(220)),
            sound_ticks: Some(3),
            ..WeaponLook::default()
        });
        let mut view = fighting(catalog, &[]);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 4);
        assert_eq!(
            view.session().expect("flying").beams().len(),
            4,
            "a beam fired each step"
        );
        assert_eq!(combat_sounds(&mut view).len(), 2, "on the first and fourth");
        view.input(&key(FIRE_KEY, false));
        ticks(&mut view, 1);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 1);
        assert_eq!(combat_sounds(&mut view), [], "still playing");
        view.input(&key(FIRE_KEY, false));
        ticks(&mut view, 3);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 1);
        assert_eq!(combat_sounds(&mut view).len(), 1, "played out");
    }

    #[test]
    fn the_metrics_debug_as_their_name() {
        let metrics = Metrics(Rc::new(crate::text::fixture::MonoMetrics));
        assert_eq!(format!("{metrics:?}"), "Metrics");
    }

    #[test]
    fn the_help_line_names_the_combat_keys() {
        for name in ["Space", "Ctrl", "W", "Tab", "R:"] {
            assert!(HELP.contains(name), "{name}: {HELP}");
        }
    }

    // Point defence.

    /// Says no missile is hostile, counting the times it is asked.
    #[derive(Debug, Default)]
    struct Unalarmed {
        asked: std::cell::Cell<usize>,
    }

    impl nova_sim::PointDefenceRule for Unalarmed {
        fn hostile(
            &self,
            _defender: nova_sim::combat::defence::Side,
            _firer: nova_sim::combat::defence::Side,
            _govts: &nova_sim::Governments,
        ) -> bool {
            self.asked.set(self.asked.get() + 1);
            false
        }
    }

    /// Every NPC idles, targets the player and holds its trigger.
    #[derive(Debug)]
    struct Attacking;

    impl Behaviour for Attacking {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Idle
        }

        fn trigger(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
        ) -> nova_sim::Trigger {
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

        fn target(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
        ) -> Option<ShipRef> {
            Some(ShipRef::Player)
        }
    }

    const QUAD: WeaponId = WeaponId(133);
    const IR: WeaponId = WeaponId(134);

    /// [`armed`] with the player, over a planet at the centre, carrying a
    /// point-defence turret (held by outfit 250), and its traffic a
    /// missile it fires once, 5 pixels a tick.
    fn defending() -> FakeCatalog {
        let mut catalog = armed(1, &[QUAD]);
        catalog.sites = vec![site(
            140,
            (0.0, 0.0),
            StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
        )];
        let sprite = |side| {
            Some(nova_sim::FrameSize {
                width: side,
                height: side,
            })
        };
        catalog.weapons.push(nova_sim::WeaponRecord {
            reload: 5,
            count: 12,
            mass_dmg: 1,
            energy_dmg: 4,
            frame: sprite(35),
            ..gun(QUAD, 0, 9, -1)
        });
        catalog.weapons.push(nova_sim::WeaponRecord {
            reload: 1000,
            count: 200,
            speed: 500,
            frame: sprite(18),
            ..gun(IR, 0, 1, -1)
        });
        catalog.hulls = vec![hull_of(128, &[QUAD]), hull_of(129, &[IR])];
        catalog
    }

    /// `catalog`'s flight with its NPC attacking from 200 pixels above
    /// the player, after the step that populates the system.
    fn attacked(catalog: FakeCatalog) -> View {
        let (_, chance) = scripted(&placed(750, 550, 180));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Attacking));
        view.tick(TICK);
        view
    }

    #[test]
    fn the_fight_asks_the_point_defence_rule_given_each_step_in_flight() {
        let rule = Rc::new(Unalarmed::default());
        let asking = Rulebook::default().with_override(RuleKey::PdHostility, RuleSource::Bible);
        let mut view = attacked(defending())
            .with_rules(asking)
            .with_point_defence_rule(rule.clone());
        view.tick(TICK);
        assert_eq!(shots_of(&view, IR), 1, "fired at the player");
        assert_eq!(
            rule.asked.get(),
            0,
            "after the player's point defence, which runs before the NPCs fire"
        );
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 2, "of the one missile, each step");
        assert_eq!(shots_of(&view, QUAD), 0, "never hostile");
        view.input(&key(MAP_KEY, true));
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 2, "not while the map is open");
        view.input(&key(MAP_KEY, true));
        tap(&mut view, LAND);
        assert!(view.take_landing().is_some(), "one press, uninhabited");
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 2, "not while landed");
    }

    #[test]
    fn a_missile_shot_down_is_drawn_exploding_and_unheard() {
        let mut view = attacked(defending());
        let mut sounds = Vec::new();
        let mut exploded = false;
        for _ in 0..12 {
            view.tick(TICK);
            sounds.extend(combat_sounds(&mut view));
            let explosion = drawn(&view).iter().any(
                |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 400),
            );
            if explosion {
                exploded = true;
                break;
            }
        }
        assert!(exploded, "bööm 128 drawn");
        assert_eq!(shots_of(&view, IR), 0, "shot down");
        assert!(
            sounds
                .iter()
                .all(|sound| sound.sound != nova_sim::SoundId(302)),
            "{sounds:?}"
        );
    }

    /// [`armed`] with a blaster doing 10 energy and 10 mass damage, and
    /// govt 140 that costs 3 to disable a ship of.
    fn damaging() -> FakeCatalog {
        let catalog = armed(1, &[BLASTER]);
        FakeCatalog {
            weapons: vec![nova_sim::WeaponRecord {
                reload: 5,
                mass_dmg: 10,
                energy_dmg: 10,
                ..gun(BLASTER, 0, -1, -1)
            }],
            govts: vec![nova_sim::GovtRecord {
                id: GovtId(140),
                flags: 0,
                flags2: 0,
                crime_tol: 0,
                initial_rec: 0,
                penalties: nova_sim::Penalties {
                    disable: 3,
                    ..nova_sim::Penalties::default()
                },
                max_odds: 100,
                classes: [-1; 4],
                allies: [-1; 4],
                enemies: [-1; 4],
                comm_name: String::new(),
                scan_mask: 0,
                scan_fine: 0,
                news_pic: -1,
                require: 0,
                inh_jam: [0; 4],
            }],
            ..catalog
        }
    }

    #[test]
    fn by_default_npcs_fight_by_novas_ai_and_crimes_by_novas_law() {
        // The warship 100 above, facing away: it flies on up the line of
        // fire.
        let (_, chance) = scripted(&placed(750, 650, 0));
        let mut view = FlightView::new(damaging()).with_chance(chance);
        view.tick(TICK);
        view.input(&key(FIRE_KEY, true));
        let mut answered = false;
        for _ in 0..120 {
            view.tick(TICK);
            let session = view.session().expect("flying");
            answered |= session
                .npcs()
                .first()
                .is_some_and(|npc| npc.goal == Goal::Attack(ShipRef::Player));
            if session
                .npcs()
                .first()
                .is_some_and(|npc| npc.condition == Condition::Disabled)
            {
                break;
            }
        }
        assert!(answered, "the warship hit turned on the player");
        let session = view.session().expect("flying");
        assert_eq!(session.npcs()[0].condition, Condition::Disabled);
        assert_eq!(session.pilot().legal_record(GovtId(140)), -3);
    }

    /// Records each crime, and changes nothing.
    #[derive(Debug, Default)]
    struct Witness {
        seen: RefCell<Vec<nova_sim::Crime>>,
    }

    impl nova_sim::LegalCode for Witness {
        fn penalties(
            &self,
            crime: nova_sim::Crime,
            _victim: Option<GovtId>,
            _govts: &nova_sim::Governments,
        ) -> Vec<(GovtId, i32)> {
            self.seen.borrow_mut().push(crime);
            Vec::new()
        }

        fn system_penalties(
            &self,
            _crime: nova_sim::Crime,
            _victim: Option<GovtId>,
            _from: nova_sim::SystemId,
            _map: &dyn nova_sim::CrimeMap,
            _govts: &nova_sim::Governments,
        ) -> Vec<(nova_sim::SystemId, f32)> {
            Vec::new()
        }
    }

    #[test]
    fn the_flight_convicts_by_the_law_it_is_given() {
        let witness = Rc::new(Witness::default());
        // 100 above, facing away: it flies on up the line of fire.
        let (_, chance) = scripted(&placed(750, 650, 0));
        let mut view = FlightView::new(damaging())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_law(witness.clone());
        view.tick(TICK);
        view.input(&key(FIRE_KEY, true));
        for _ in 0..120 {
            view.tick(TICK);
            if view.session().expect("flying").npcs()[0].condition == Condition::Disabled {
                break;
            }
        }
        assert_eq!(*witness.seen.borrow(), [nova_sim::Crime::Disable]);
        let session = view.session().expect("flying");
        assert_eq!(session.pilot().legal_record(GovtId(140)), 0, "as it says");
    }

    // Boarding.

    use nova_sim::HullSpec;
    use nova_sim::reserves::Gauge;

    /// Disables every ship of some strength: the NPCs (ship 129, of
    /// strength 5), never the player (ship 128, of none).
    #[derive(Debug)]
    struct DisablesTheStrong;

    impl DisableRule for DisablesTheStrong {
        fn disabled(&self, _armor: Gauge, hull: &HullSpec) -> bool {
            hull.strength > 0.0
        }
    }

    /// Never repels, always gives odds of 75 and always captures; a person
    /// carries its `Credits`, and grants nothing.
    #[derive(Debug)]
    struct Sure;

    impl BoardingRule for Sure {
        fn repels(&self, _booty: u16) -> bool {
            false
        }

        fn capture_odds(
            &self,
            _crew: &nova_sim::board::CaptureCrew,
            _chance: &mut dyn Chance,
        ) -> u8 {
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

    /// Always repels boarders; never asked anything more.
    #[derive(Debug)]
    struct Repels;

    impl BoardingRule for Repels {
        fn repels(&self, _booty: u16) -> bool {
            true
        }

        fn capture_odds(
            &self,
            _crew: &nova_sim::board::CaptureCrew,
            _chance: &mut dyn Chance,
        ) -> u8 {
            unreachable!("repelled")
        }

        fn captures(&self, _odds: u8, _chance: &mut dyn Chance) -> bool {
            unreachable!("repelled")
        }

        fn person_credits(&self, _credits: i32, _chance: &mut dyn Chance) -> i64 {
            unreachable!("repelled")
        }

        fn grant(
            &self,
            _grant: &nova_sim::grant::PersonGrant,
            _stock: &[nova_sim::grant::GrantStock],
            _free_mass: i64,
            _grant_max: nova_sim::RuleSource,
            _chance: &mut dyn Chance,
        ) -> Option<nova_sim::grant::Granted> {
            unreachable!("repelled")
        }
    }

    /// [`trafficked`] with its ship 129 (of strength 5, a crew of 3 and a
    /// `Cost` of 150,000, and no speed, so it starts at rest) carrying
    /// money (`Booty` 0x0040), and the player's ship 128 a crew of 10.
    fn boardable() -> FakeCatalog {
        let mut catalog = trafficked(&[130], 1, 129, 1);
        catalog.dudes[0].1.booty = 0x0040;
        for record in &mut catalog.ships {
            if record.id == ShipId(129) {
                record.crew = 3;
                record.cost = 150_000;
            }
        }
        let mut player = catalog.ships[0].clone();
        player.id = ShipId(128);
        player.crew = 10;
        catalog.ships[0].fields.speed = 0;
        catalog.ships.push(player);
        catalog.hulls = vec![nova_sim::HullRecord {
            strength: 5,
            ..hull_of(129, &[])
        }];
        catalog
    }

    /// [`boardable`]'s flight with its one NPC placed at (`x` - 750,
    /// `y` - 750) facing `heading`, disabled by [`DisablesTheStrong`] on
    /// the first tick unless `intact`, and targeted.
    fn beside(x: u32, y: u32, heading: u32, intact: bool) -> View {
        let (_, chance) = scripted(&placed(x, y, heading));
        let mut view = FlightView::new(boardable())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        if !intact {
            view = view.with_disable_rule(Rc::new(DisablesTheStrong));
        }
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        let session = view.session().expect("flying");
        assert_eq!(session.target().map(|npc| npc.id), Some(NpcId(0)));
        view
    }

    #[test]
    fn b_on_a_disabled_aligned_slow_target_reports_a_boarding_once() {
        let mut view = beside(750, 750, 0, false);
        assert_eq!(view.take_boarding(), None);
        tap(&mut view, BOARD_KEY);
        let boarding = view.take_boarding().expect("boarded").view;
        assert_eq!((boarding.npc, boarding.ship), (NpcId(0), ShipId(129)));
        assert!(boarding.credits >= 1000, "{boarding:?}");
        assert_eq!(view.take_boarding(), None, "once");
        assert_eq!(view.boarding(), Some(boarding), "still under way");
        assert_eq!(view.message(), None);
        let session = view.session().expect("flying");
        assert!(session.npcs()[0].boarded);
        assert_eq!(BOARD_KEY, Key::Char('b'));
    }

    #[test]
    fn each_refused_boarding_says_why_in_the_originals_words() {
        let mut intact = beside(750, 750, 0, true);
        tap(&mut intact, BOARD_KEY);
        assert_eq!(intact.take_boarding(), None);
        assert_lasts(&mut intact, CANT_BOARD, 360);
        let mut far = beside(800, 750, 0, false);
        tap(&mut far, BOARD_KEY);
        assert_eq!(far.message(), Some(NOT_CLOSE_ENOUGH));
        let mut fast = beside(750, 750, 0, false);
        fast.input(&key(Key::Up, true));
        ticks(&mut fast, 10);
        fast.input(&key(Key::Up, false));
        tap(&mut fast, BOARD_KEY);
        assert_eq!(fast.message(), Some(TOO_FAST_TO_BOARD));
        let mut repelled = beside(750, 750, 0, false).with_boarding_rule(Rc::new(Repels));
        tap(&mut repelled, BOARD_KEY);
        assert_eq!(repelled.take_boarding(), None);
        assert_lasts(&mut repelled, REPELLED, MESSAGE_TICKS);
        assert_eq!(MESSAGE_TICKS, 240);
        assert_eq!(
            [CANT_BOARD, NOT_CLOSE_ENOUGH, TOO_FAST_TO_BOARD, REPELLED,],
            [
                "You can't board this ship.",
                "You're not close enough to board this ship.",
                "You're moving too fast to board this ship.",
                "You were repelled while attempting to board this ship.",
            ]
        );
    }

    #[test]
    fn a_misaligned_or_missing_target_says_nothing() {
        let mut aside = beside(750, 750, 90, false);
        tap(&mut aside, BOARD_KEY);
        assert_eq!(aside.message(), None);
        assert_eq!(aside.take_boarding(), None);
        let mut alone = flight();
        tap(&mut alone, BOARD_KEY);
        assert_eq!(alone.message(), None);
        assert_eq!(board_refusal_message(BoardRefusal::NoTarget), None);
        assert_eq!(board_refusal_message(BoardRefusal::Misaligned), None);
    }

    #[test]
    fn boarding_keeps_the_message() {
        let mut view = beside(750, 750, 0, false);
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, BOARD_KEY);
        view.take_boarding().expect("boarded");
        assert_eq!(view.message(), Some(MISSION_FAILED), "under the dialog");
        assert_eq!(view.plunder(Take::Abort), Taken::Aborted);
        assert_lasts(&mut view, MISSION_FAILED, 240);
    }

    #[test]
    fn plundering_goes_through_the_session_and_says_what_it_took() {
        let mut view = beside(750, 750, 0, false);
        tap(&mut view, BOARD_KEY);
        let boarding = view.take_boarding().expect("boarded").view;
        let cash = view.pilot().expect("a pilot").cash();
        assert_eq!(
            view.plunder(Take::Credits),
            Taken::Credits(boarding.credits)
        );
        assert_eq!(
            view.pilot().expect("a pilot").cash(),
            cash + boarding.credits
        );
        let said = format!(
            "You stole all the {} credits from this ship.",
            boarding.credits
        );
        assert_eq!(view.boarding().map(|now| now.credits), Some(0));
        assert_lasts(&mut view, &said, 240);
        assert_eq!(view.plunder(Take::Abort), Taken::Aborted);
        assert_eq!(view.boarding(), None);
    }

    #[test]
    fn the_plunder_messages_are_the_originals() {
        let food = Some("Food");
        let message = |taken| plunder_message(taken, food, Some("Rockets"));
        let cargo = |stored| Taken::Cargo {
            good: Good::Commodity(0),
            stored,
        };
        assert_eq!(message(cargo(0)).as_deref(), Some(NO_CARGO_STORED));
        assert_eq!(
            message(cargo(1)).as_deref(),
            Some("You salvaged 1 ton of Food from this ship.")
        );
        assert_eq!(
            message(cargo(12)).as_deref(),
            Some("You salvaged 12 tons of Food from this ship.")
        );
        assert_eq!(
            plunder_message(cargo(3), None, None).as_deref(),
            Some("You salvaged 3 tons of cargo from this ship."),
            "a good with no name"
        );
        assert_eq!(
            message(Taken::Credits(5750)).as_deref(),
            Some("You stole all the 5750 credits from this ship.")
        );
        let ammo = |count| Taken::Ammo {
            outfit: OutfitId(310),
            count,
        };
        assert_eq!(message(ammo(0)).as_deref(), Some(NO_AMMO_STORED));
        assert_eq!(
            message(ammo(7)).as_deref(),
            Some("You salvaged 7 Rockets from this ship.")
        );
        let energy = |stored, full| Taken::Energy {
            offered: 170,
            stored,
            full,
        };
        assert_eq!(message(energy(0, true)).as_deref(), Some(NO_ENERGY_STORED));
        assert_eq!(
            message(energy(170, false)).as_deref(),
            Some(ALL_ENERGY_STORED)
        );
        assert_eq!(message(energy(100, true)).as_deref(), Some(ENERGY_FILLED));
        assert_eq!(message(Taken::Tripped).as_deref(), Some(SELF_DESTRUCT));
        assert_eq!(
            message(Taken::CaptureFailed).as_deref(),
            Some(CAPTURE_FAILED)
        );
        assert_eq!(message(Taken::FleetFull).as_deref(), Some(FLEET_FULL));
        assert_eq!(message(Taken::Escorted).as_deref(), Some(ASSIGNED_ESCORT));
        for silent in [Taken::Captured, Taken::Aborted, Taken::Nothing] {
            assert_eq!(message(silent), None, "{silent:?}");
        }
        assert_eq!(
            [
                NO_CARGO_STORED,
                NO_AMMO_STORED,
                NO_ENERGY_STORED,
                ALL_ENERGY_STORED,
                ENERGY_FILLED,
                SELF_DESTRUCT,
                CAPTURE_FAILED,
                FLEET_FULL,
                ASSIGNED_ESCORT,
                RETAINED_OLD_SHIP,
                LOST_OLD_SHIP,
            ],
            [
                "You couldn't store any of the cargo you plundered from this ship.",
                "You couldn't store any of the ammo you plundered from this ship.",
                "You couldn't store any of the energy you transferred from this ship.",
                "You transferred all of this ship's energy to your reactors and batteries.",
                "You filled your reactors and batteries with energy from this ship.",
                "Oops! You tripped this ship's security self-destruct mechanism.",
                "Your attempt to capture this ship was unsuccessful.",
                "You already have the maximum possible number of escorts.",
                "You assigned this ship to your fleet of escorts.",
                "You retained your old ship as an escort.",
                "You were unable to retain your old ship as an escort.",
            ]
        );
        assert_eq!(assigned_message(Assigned::Escort), ASSIGNED_ESCORT);
        assert_eq!(assigned_message(Assigned::MyShip), RETAINED_OLD_SHIP);
        assert_eq!(assigned_message(Assigned::Abandoned), LOST_OLD_SHIP);
    }

    /// [`beside`], boarded by [`Sure`] and captured, awaiting its
    /// assignment.
    fn captured() -> View {
        captured_in(boardable())
    }

    /// [`captured`], from `catalog`, a [`boardable`] one.
    fn captured_in(catalog: FakeCatalog) -> View {
        let (_, chance) = scripted(&placed(750, 750, 0));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_disable_rule(Rc::new(DisablesTheStrong))
            .with_boarding_rule(Rc::new(Sure));
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        tap(&mut view, BOARD_KEY);
        let boarding = view.take_boarding().expect("boarded").view;
        assert_eq!(boarding.odds, 75, "by the rule given");
        assert_eq!(view.plunder(Take::Capture), Taken::Captured);
        assert_eq!(view.message(), None, "the assignment dialog says it");
        view
    }

    #[test]
    fn the_target_panel_is_redrawn_once_the_plunder_dialog_closes() {
        let mut view = beside(750, 750, 0, true);
        view.tick(TICK);
        while now_tick(&view) % TARGET_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        let mut view = view.with_disable_rule(Rc::new(DisablesTheStrong));
        view.tick(TICK);
        assert_eq!(status_now(&view), Some(nova_sim::TargetStatus::Disabled));
        assert_ne!(status_shown(&view), status_now(&view), "held");
        tap(&mut view, BOARD_KEY);
        let boarding = view.take_boarding().expect("boarded").view;
        assert_eq!(
            view.plunder(Take::Credits),
            Taken::Credits(boarding.credits)
        );
        assert_eq!(*view.target_shown(), view.target_now(), "redrawn at once");
        assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
    }

    #[test]
    fn use_as_escort_says_so() {
        let mut view = captured();
        view.tick(TICK / 2);
        let alpha = view.alpha();
        assert!(alpha > 0.0, "between steps");
        assert_eq!(view.assign(Assignment::Escort), Some(Assigned::Escort));
        assert_eq!(view.alpha(), alpha, "the old ship flies on");
        let pilot = view.pilot().expect("a pilot");
        assert_eq!(pilot.escorts().len(), 1);
        assert!(view.take_save_due());
        assert_lasts(&mut view, ASSIGNED_ESCORT, 240);
    }

    #[test]
    fn use_as_my_ship_flies_and_draws_the_captured_ship() {
        let mut view = captured();
        view.tick(TICK / 2);
        assert!(view.alpha() > 0.0, "between steps");
        let mut asked = view.catalog().sheets_asked.borrow().clone();
        assert_eq!(view.assign(Assignment::MyShip), Some(Assigned::MyShip));
        assert_eq!(view.alpha(), 0.0, "drawn where it is");
        assert_eq!(view.message(), Some(RETAINED_OLD_SHIP));
        assert_eq!(view.session().expect("flying").ship(), ShipId(129));
        asked.push(ShipId(129));
        assert_eq!(*view.catalog().sheets_asked.borrow(), asked, "read once");
        view.settle_script();
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            asked,
            "not again once settled"
        );
        let drawn_player = sprites(&drawn(&view))
            .into_iter()
            .filter(|&(_, center)| center == VIEW_CENTER)
            .map(|(image, _)| image)
            .collect::<Vec<_>>();
        assert_eq!(
            drawn_player,
            [ImageKey::sprite(2001, 0), ImageKey::sprite(2201, 0)],
            "ship 129's sheet, and its lights"
        );
        assert_eq!(view.assign(Assignment::MyShip), None, "nothing awaits");
        assert_lasts(&mut view, RETAINED_OLD_SHIP, 360);
    }

    #[test]
    fn the_plunder_taken_is_named_as_the_session_names_it() {
        let mut catalog = boardable();
        catalog.fields.holds = 20;
        catalog.dudes[0].1.booty = 0x0041;
        catalog.commodities = CommodityStrings {
            names: vec!["Food".to_owned()],
            name_patches: Default::default(),
            base_prices: vec!["75".to_owned()],
            price_patches: Default::default(),
        };
        for record in &mut catalog.ships {
            if record.id == ShipId(129) {
                record.fields.holds = 20;
            }
        }
        catalog.weapons = vec![nova_sim::WeaponRecord {
            ammo_type: 12,
            ..gun(ROCKET, 0x0002, -1, -1)
        }];
        let rockets = |id, ammo| nova_sim::HullRecord {
            strength: 5,
            weapons: vec![nova_sim::StockWeapon {
                weapon: ROCKET,
                count: 1,
                ammo,
            }],
            ..hull_of(id, &[])
        };
        catalog.hulls = vec![rockets(128, 0), rockets(129, 2)];
        catalog.hulls[0].strength = 0;
        catalog.outfits = vec![
            holding(250, ROCKET),
            OutfitRecord {
                id: OutfitId(310),
                name: "Rockets".to_owned(),
                short_name: "Rockets".to_owned(),
                disp_weight: 0,
                mass: 1,
                tech_level: 1,
                max: 10,
                flags: 0,
                cost: 100,
                mods: [
                    (nova_sim::combat::armament::MOD_AMMO, 140),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                ],
                contribute: 0,
                require: 0,
                require_govt: -1,
                buy_random: 100,
                availability: nova_sim::Test::default(),
                item_class: 0,
                lc_name: "rocket".to_owned(),
                lc_plural: "rockets".to_owned(),
                scan_mask: 0,
                on_purchase: nova_sim::Script::default(),
                on_sell: nova_sim::Script::default(),
            },
        ];
        let (_, chance) = scripted(&placed(750, 750, 0));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_disable_rule(Rc::new(DisablesTheStrong));
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        tap(&mut view, BOARD_KEY);
        let boarding = view.take_boarding().expect("boarded");
        assert_eq!(boarding.view.cargo, Some((Good::Commodity(0), 19)));
        assert_eq!(boarding.view.ammo, Some((OutfitId(310), 2)));
        let named = |shown: &PlunderShown| (shown.good.clone(), shown.outfit.clone());
        assert_eq!(
            named(&boarding),
            (Some("Food".to_owned()), Some("Rockets".to_owned())),
            "the dialog names them as the messages do"
        );
        assert_eq!(view.plunder_shown().as_ref(), Some(&boarding));
        view.plunder(Take::Cargo);
        assert_eq!(
            view.message(),
            Some("You salvaged 19 tons of Food from this ship.")
        );
        let shown = view.plunder_shown().expect("still under way");
        assert_eq!(named(&shown), (None, Some("Rockets".to_owned())));
        view.plunder(Take::Ammo);
        assert_eq!(
            view.message(),
            Some("You salvaged 2 Rockets from this ship.")
        );
        let shown = view.plunder_shown().expect("still under way");
        assert_eq!(named(&shown), (None, None), "nothing left to name");
    }

    #[test]
    fn the_help_line_names_board() {
        assert!(HELP.contains("B: board"), "{HELP}");
    }

    // Hailing.

    use nova_sim::{Haggle, HailOptions, HailRefusal, Help};

    /// [`boardable`]'s ship comm strings: "c<n>" for each, with "Channel
    /// open." in every variant of the first group.
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

    /// [`beside`] with the ship comm strings.
    fn hailable(intact: bool) -> View {
        let mut view = beside(750, 750, 0, intact);
        view.catalog.strings = vec![(3000, comm_strings())];
        view
    }

    #[test]
    fn y_on_a_hailable_target_reports_a_hail_once() {
        let mut view = hailable(true);
        assert_eq!(view.take_hail(), None);
        view.input(&key(Key::Up, true));
        tap(&mut view, HAIL_KEY);
        let hail = view.take_hail().expect("hailed");
        assert_eq!(hail.npc, NpcId(0));
        assert_eq!(hail.reply, "Channel open.");
        assert_eq!(view.take_hail(), None, "once");
        assert_eq!(view.hailing().map(|hail| hail.npc), Some(NpcId(0)));
        assert_eq!(view.message(), None);
        view.tick(TICK);
        assert_eq!(
            view.session().expect("flying").player().velocity,
            Vec2::ZERO,
            "the flight keys let go"
        );
        assert_eq!(HAIL_KEY, Key::Char('y'));
    }

    #[test]
    fn y_on_a_disabled_target_says_no_response() {
        let mut view = hailable(false);
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_hail(), None);
        assert_lasts(&mut view, NO_RESPONSE, 240);
        assert_eq!(
            hail_refusal_message(HailRefusal::NoResponse),
            Some("No response.")
        );
        assert_eq!(
            hail_refusal_message(HailRefusal::InHyperspace),
            Some("Unable to send hail - target ship is entering hyperspace.")
        );
        assert_eq!(hail_refusal_message(HailRefusal::NoTarget), None);
        let mut alone = flight();
        tap(&mut alone, HAIL_KEY);
        assert_eq!(alone.take_hail(), None);
        assert_eq!(alone.message(), None, "no target: nothing said");
    }

    #[test]
    fn a_hail_refused_beeps_through_the_view() {
        let mut view = hailable(false);
        view.take_sounds();
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::Refused)]);
        let mut alone = flight();
        alone.take_sounds();
        tap(&mut alone, HAIL_KEY);
        assert_eq!(
            alone.take_sounds(),
            [Sound::Sim(SimSound::Refused)],
            "no target"
        );
    }

    #[test]
    fn y_with_a_stellar_selected_and_no_ship_target_opens_nothing_and_says_nothing() {
        let mut view = flight();
        tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
        assert_eq!(nav_target(&view), Some(StellarId(128)));
        view.take_sounds();
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_hail(), None);
        assert_eq!(view.take_offer(), None);
        assert_eq!(view.message(), None, "the planet comm dialog is not built");
        assert_eq!(view.take_sounds(), [], "an answer beeps nothing");
    }

    #[test]
    fn y_with_an_uninhabited_stellar_selected_says_no_response() {
        let mut view = flight_among(vec![site(
            128,
            (0.0, -600.0),
            StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
        )]);
        tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_hail(), None);
        assert_lasts(&mut view, NO_RESPONSE, 240);
    }

    #[test]
    fn alt_y_hails_the_selected_stellar_over_the_ship_target() {
        let mut view = hailable(true);
        tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
        assert_eq!(nav_target(&view), Some(StellarId(128)));
        alt_tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_hail(), None, "no comm dialog for the ship");
        assert_eq!(view.message(), None);
        tap(&mut view, HAIL_KEY);
        assert!(view.take_hail().is_some(), "Y alone hails the ship");
    }

    #[test]
    fn a_hail_dialog_keeps_the_message() {
        let mut view = hailable(true);
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, HAIL_KEY);
        view.take_hail().expect("hailed");
        assert_eq!(view.message(), Some(MISSION_FAILED), "under the dialog");
        view.hang_up();
        assert_lasts(&mut view, MISSION_FAILED, 240);
    }

    #[test]
    fn answers_haggles_and_hanging_up_go_through_the_session() {
        let mut view = hailable(true);
        tap(&mut view, HAIL_KEY);
        view.take_hail().expect("hailed");
        let answered = view.answer(0).expect("open");
        assert_eq!(answered.npc, NpcId(0));
        assert_eq!(view.haggle(Haggle::Accept), None, "no price asked");
        view.hang_up();
        assert_eq!(view.hailing(), None);
        assert_eq!(view.answer(0), None);
    }

    #[test]
    fn hanging_up_a_hail_redraws_the_radar_at_once_leaving_the_timer_and_blink() {
        let mut view = hailable(true);
        ticks(&mut view, 3);
        tap(&mut view, HAIL_KEY);
        view.take_hail().expect("hailed");
        let stale = RadarScene::default();
        assert_ne!(view.radar_now(), stale);
        view.radar_scene = Some(stale);
        let cadence = view.hud_cadence;
        view.hang_up();
        assert_eq!(view.hailing(), None);
        assert_eq!(view.hud_cadence, cadence, "neither timed nor flipped");
        assert_eq!(*view.radar_shown(), view.radar_now(), "as _RedrawStatus");
    }

    /// An option that asks a price, saying nothing.
    #[derive(Debug)]
    struct Sell;

    impl nova_sim::HailOption for Sell {
        fn label(&self) -> String {
            "Sell".to_owned()
        }

        fn applies(&self, _hail: &nova_sim::Hail) -> bool {
            true
        }

        fn press(&self, _hail: &nova_sim::Hail, _chance: &mut dyn Chance) -> nova_sim::Answer {
            let line = nova_sim::Reply::Line { list: 1, index: 1 };
            nova_sim::Answer {
                ask: Some(nova_sim::Ask {
                    paid: (line, None),
                    declined: (line, None),
                    short: line,
                }),
                ..nova_sim::Answer::default()
            }
        }
    }

    #[test]
    fn a_price_asked_is_haggled_over_through_the_session() {
        let mut view = hailable(true).with_hail_options(HailOptions::empty().with(Rc::new(Sell)));
        tap(&mut view, HAIL_KEY);
        view.take_hail().expect("hailed");
        let asked = view.answer(0).expect("open");
        assert!(asked.asking.is_some(), "{asked:?}");
        let settled = view.haggle(Haggle::Accept).expect("open");
        assert_eq!(settled.asking, None, "settled");
    }

    #[test]
    fn the_screens_hail_options_are_listed() {
        let mut view = hailable(true);
        view = view.with_hail_options(HailOptions::empty());
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_hail().expect("hailed").options, []);
    }

    /// An option the ship answers by flying over to refuel the player.
    #[derive(Debug)]
    struct Refuel;

    impl nova_sim::HailOption for Refuel {
        fn label(&self) -> String {
            "Refuel".to_owned()
        }

        fn applies(&self, _hail: &nova_sim::Hail) -> bool {
            true
        }

        fn press(&self, _hail: &nova_sim::Hail, _chance: &mut dyn Chance) -> nova_sim::Answer {
            nova_sim::Answer {
                deed: Some(nova_sim::Deed::Help(Help::Refuel)),
                ..nova_sim::Answer::default()
            }
        }
    }

    #[test]
    fn a_helpers_note_shows_for_the_common_time() {
        let mut view = hailable(true).with_hail_options(HailOptions::empty().with(Rc::new(Refuel)));
        tap(&mut view, HAIL_KEY);
        view.take_hail().expect("hailed");
        view.answer(0).expect("open");
        view.hang_up();
        assert_eq!(view.message(), None);
        view.tick(TICK);
        let note = view.message().expect("refuelled").to_owned();
        assert!(note.ends_with(":  Energy transfer complete."), "{note}");
        assert_lasts(&mut view, &note, MESSAGE_TICKS);
    }

    #[test]
    fn the_help_line_names_hail() {
        assert!(HELP.contains("Y: hail"), "{HELP}");
    }

    // Escorts.

    use super::super::escorts::{ABSENT_ROW, MENU_AT, MENU_HEIGHT, MENU_WIDTH};
    use nova_sim::{EscortClass, EscortGroup, EscortOrder};

    /// `pilot`, saved and read back with a fleet of `ships`, each full.
    fn with_escorts(pilot: &Pilot, ships: &[i16]) -> Pilot {
        with_fleet(pilot, ships, false)
    }

    /// `pilot`, saved and read back with a fleet of `ships`, each full,
    /// and fighters out of a bay when `carried`.
    fn with_fleet(pilot: &Pilot, ships: &[i16], carried: bool) -> Pilot {
        let mut save: serde_json::Value =
            serde_json::from_str(&nova_sim::save::encode(pilot)).expect("JSON");
        let gauge = |max: f32| serde_json::json!({"now": max, "max": max});
        save["escorts"] = ships
            .iter()
            .map(|ship| {
                serde_json::json!({
                    "ship": ship,
                    "reserves": {"shield": gauge(40.0), "armor": gauge(60.0), "fuel": gauge(250.0)},
                    "order": null,
                    "carried": carried,
                    "wage": null,
                    "person": null
                })
            })
            .collect();
        nova_sim::save::decode(&save.to_string()).expect("a pilot")
    }

    /// [`boardable`] with ship 130 a fighter (`EscortType` 0) holding
    /// no fuel, and two of them out of the player's bays; its traffic
    /// ship (NPC 0) placed 100 above the player. A tick has placed the
    /// fighters (NPCs 1 and 2).
    fn fighters_out() -> View {
        fighters_out_of(boardable())
    }

    /// [`fighters_out`] from `catalog`, a [`boardable`] one.
    fn fighters_out_of(mut catalog: FakeCatalog) -> View {
        for record in &mut catalog.ships {
            if record.id == ShipId(130) {
                record.escort_type = 0;
                record.fields.fuel = 0;
            }
        }
        let pilot = with_fleet(
            &Pilot::new(&catalog, "Carrier").expect("starts"),
            &[130, 130],
            true,
        );
        let (_, chance) = scripted(&placed(750, 650, 0));
        let mut view = FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        assert!(view.session().expect("flying").is_escort(NpcId(2)));
        view
    }

    /// Taps `key` with Alt (Option) held.
    fn option(view: &mut View, key_: Key) {
        view.input(&key(Key::Alt, true));
        tap(view, key_);
        view.input(&key(Key::Alt, false));
    }

    #[test]
    fn each_key_asks_the_session_for_its_command() {
        let mut view = fighters_out();
        let commands = [
            (LAND_KEY, PlayerCommand::Land),
            (BOARD_KEY, PlayerCommand::Board),
            (HAIL_KEY, PlayerCommand::Hail),
            (MAP_KEY, PlayerCommand::Map),
            (JUMP_KEY, PlayerCommand::Jump),
            (JETTISON_KEY, PlayerCommand::Jettison),
            (EJECT_KEY, PlayerCommand::Eject),
            (MISSION_INFO_KEY, PlayerCommand::MissionInfo),
            (HYPER_SELECT_KEY, PlayerCommand::SelectSystem),
            (HYPERSPACE_MODE_KEY, PlayerCommand::Nav),
            (TARGET_KEY, PlayerCommand::Target),
            (NEAREST_KEY, PlayerCommand::Target),
            (MENU_KEY, PlayerCommand::Escort),
            (MENU_CLOSE_KEY, PlayerCommand::Escort),
            (ESCORT_ATTACK_KEY, PlayerCommand::Escort),
            (ESCORT_DEFEND_KEY, PlayerCommand::Escort),
            (ESCORT_HOLD_KEY, PlayerCommand::Escort),
            (ESCORT_RECALL_KEY, PlayerCommand::Escort),
            (SELECT_KEY, PlayerCommand::SelectWeapon),
            (NEAREST_STELLAR_KEY, PlayerCommand::Nav),
            (NAV_OFF_KEY, PlayerCommand::Nav),
            (Key::Function(1), PlayerCommand::Nav),
            (ZOOM_OUT_KEY, PlayerCommand::Nav),
            (ZOOM_IN_KEY, PlayerCommand::Nav),
            (ZOOM_IN_KEYPAD_KEY, PlayerCommand::Nav),
        ];
        for (k, command) in commands {
            assert_eq!(view.command_of(k), Some(command), "{k:?}");
        }
        for k in [FIRE_KEY, Key::Up, Key::Char('z'), Key::Other] {
            assert_eq!(view.command_of(k), None, "{k:?}");
        }
        assert!(!view.escort_menu().is_open());
        assert_eq!(view.command_of(Key::Char('1')), Some(PlayerCommand::Nav));
        assert_eq!(view.command_of(GROUP_KEYS[4]), None, "5 is no stellar slot");
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        assert_eq!(view.command_of(Key::Char('1')), Some(PlayerCommand::Escort));
        assert_eq!(view.command_of(GROUP_KEYS[4]), Some(PlayerCommand::Escort));
        assert_eq!(view.command_of(Key::Char('z')), None);
    }

    #[test]
    fn option_c_sends_the_fighters_home_and_c_alone_back_to_formation() {
        let mut view = fighters_out();
        option(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(orders(&view), [Some(EscortOrder::Dock); 2]);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships returning to hangar.")
        );
        tap(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(orders(&view), [None; 2], "C alone recalls");
        tap(&mut view, MENU_KEY);
        tap(&mut view, GROUP_KEYS[1]);
        option(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  Fighters returning to hangar.")
        );
    }

    #[test]
    fn option_c_sends_an_escort_of_its_own_back_to_formation() {
        let mut view = escorted();
        tap(&mut view, ESCORT_HOLD_KEY);
        option(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(orders(&view), [None]);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships returning to formation.")
        );
    }

    #[test]
    fn the_escort_menu_shows_return_to_hangar_for_fighters_returning() {
        let mut view = fighters_out();
        option(&mut view, ESCORT_RECALL_KEY);
        tap(&mut view, MENU_KEY);
        assert!(
            texts(&drawn(&view)).contains(&"Return to Hangar".to_owned()),
            "{:?}",
            texts(&drawn(&view))
        );
    }

    /// `catalog` with `STR#` 1000 "Message Buoy: hello." and Alpha
    /// Centauri's buoy `message`.
    fn buoyed(mut catalog: FakeCatalog, message: i16) -> FakeCatalog {
        catalog
            .strings
            .push((nova_sim::buoy::BUOY_STRINGS, vec![HELLO.to_owned()]));
        catalog.alpha_buoy = message;
        catalog
    }

    const HELLO: &str = "Message Buoy: hello.";

    /// How many ticks the message `view` shows stays shown.
    fn shown_for(view: &mut View) -> u32 {
        let mut n = 0;
        while view.message().is_some() {
            view.tick(TICK);
            n += 1;
            assert!(n < 10_000, "never cleared");
        }
        n
    }

    #[test]
    fn a_jump_into_a_buoyed_system_shows_the_buoy_for_480_ticks_in_place_of_the_arrival() {
        let mut view = FlightView::new(buoyed(catalog(), 1));
        jump_to_alpha(&mut view);
        assert_eq!(view.message(), Some(HELLO));
        let buoy = shown_for(&mut view);
        let mut plain = FlightView::new(buoyed(catalog(), -1));
        jump_to_alpha(&mut plain);
        assert_eq!(
            plain.message(),
            Some("Jumping into the Alpha Centauri system on June 24, 1177 NC.")
        );
        // The arrival's 240 ticks, less those the jump's end took.
        let arrival = shown_for(&mut plain);
        assert_eq!(buoy, arrival + 240, "480 ticks from the arrival");
    }

    #[test]
    fn an_empty_buoy_replaces_the_line_shown_before_the_jump() {
        let mut view = FlightView::new(buoyed(catalog(), 2));
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        view.say_for("Before the jump.", 10_000);
        arrive_now(&mut view);
        assert_eq!(view.message(), Some(""));
    }

    #[test]
    fn fighters_abandoned_into_a_buoyed_system_are_not_told() {
        let mut view = fighters_out_of(buoyed(boardable(), 1));
        jump_to_alpha(&mut view);
        assert_eq!(view.message(), Some(HELLO));
        assert_eq!(
            view.pilot().expect("flying").escorts(),
            [],
            "still abandoned"
        );
    }

    #[test]
    fn fighters_abandoned_in_a_jump_are_told_on_arrival() {
        let mut view = fighters_out();
        jump_to_alpha(&mut view);
        assert_eq!(
            view.message(),
            Some(
                "Jumping into the Alpha Centauri system on June 24, 1177 NC.  (Two fighters abandoned)"
            )
        );
        assert_eq!(view.pilot().expect("flying").escorts(), []);
    }

    /// [`boardable`] with ship 130, a warship escort (`EscortType` 2,
    /// `InherentAI` 3), whose sheet cannot be read, and the pilot's fleet
    /// one of them; its traffic ship (NPC 0) placed 100 above the player;
    /// all idle. A tick has placed the escort (NPC 1).
    fn escorted() -> View {
        escorted_with_traffic(750, 650)
    }

    /// [`escorted`] with its traffic ship (NPC 0) placed as
    /// [`placed`]`(x, y, 0)` places it.
    fn escorted_with_traffic(x: u32, y: u32) -> View {
        let mut catalog = boardable();
        for record in &mut catalog.ships {
            if record.id == ShipId(130) {
                record.escort_type = 2;
                record.inherent_ai = 3;
            }
        }
        let pilot = with_escorts(&Pilot::new(&catalog, "Escorted").expect("starts"), &[130]);
        let (_, chance) = scripted(&placed(x, y, 0));
        let mut view = FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let session = view.session().expect("flying");
        assert!(session.is_escort(NpcId(1)), "{:?}", session.npcs());
        view
    }

    #[test]
    fn the_players_escort_is_bracketed_green() {
        let mut view = escorted();
        option(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        assert_eq!(bracket_icons(&drawn(&view)), [10016, 10017, 10018, 10019]);
    }

    #[test]
    fn the_players_fighter_is_bracketed_green() {
        let mut view = fighters_out();
        option(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        assert_eq!(bracket_icons(&drawn(&view)), [10016, 10017, 10018, 10019]);
    }

    /// Where the target panel's label `label` is drawn, if it is: its
    /// top-left.
    fn label_drawn(view: &View, label: &str) -> Option<Point> {
        drawn(view).iter().find_map(|command| match command {
            DrawCommand::Text { text, origin, .. } if text == label => Some(*origin),
            _ => None,
        })
    }

    /// Where `view` sets a label `label` against the target panel's
    /// bottom right, measured by `MonoMetrics`: 7 in from the right,
    /// on the baseline 6 above the bottom.
    fn label_place(view: &View, label: &str) -> Point {
        let bar = view.status_bar().expect("a status bar");
        let layout = &bar.layout;
        let area = layout.targ.offset(hud::bar_origin(bar));
        let width = crate::text::TextMetrics::width(
            &crate::text::fixture::MonoMetrics,
            layout.font,
            layout.font_size,
            label,
        );
        at(
            area.max.x - 7.0 - width,
            area.max.y - 6.0 - layout.font_size,
        )
    }

    #[test]
    fn the_target_panel_labels_the_players_escort_and_fighter_and_omits_a_missing_code() {
        let mono = || Rc::new(crate::text::fixture::MonoMetrics);
        let mut view = escorted().with_metrics(mono());
        option(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        assert_eq!(
            label_drawn(&view, target::ESCORT),
            Some(label_place(&view, target::ESCORT)),
            "Escort"
        );
        let mut view = fighters_out().with_metrics(mono());
        option(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        assert_eq!(
            label_drawn(&view, target::FIGHTER),
            Some(label_place(&view, target::FIGHTER)),
            "Fighter"
        );
        assert_eq!(label_drawn(&view, target::ESCORT), None);
        // The traffic ship's government, 140, has the code "Fed.".
        let mut view = fighting(armed(1, &[]), &placed(750, 650, 180)).with_metrics(mono());
        tap(&mut view, TARGET_KEY);
        assert_eq!(
            label_drawn(&view, "Fed."),
            Some(label_place(&view, "Fed.")),
            "with a code"
        );
        let uncoded = FakeCatalog {
            codes: Vec::new(),
            ..armed(1, &[])
        };
        let mut view = fighting(uncoded, &placed(750, 650, 180)).with_metrics(mono());
        tap(&mut view, TARGET_KEY);
        assert!(target_of(&view).is_some());
        let bottom = label_place(&view, "").y;
        let at_bottom: Vec<String> = drawn(&view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } if origin.y == bottom => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(at_bottom, ["Shield: ", "100%"], "no code drawn");
    }

    fn orders(view: &View) -> Vec<Option<EscortOrder>> {
        view.pilot()
            .expect("flying")
            .escorts()
            .iter()
            .map(|escort| escort.order)
            .collect()
    }

    #[test]
    fn f_d_v_and_c_command_every_escort_and_say_what_changed() {
        let mut view = escorted();
        tap(&mut view, TARGET_KEY);
        assert_eq!(
            view.session().expect("flying").target().map(|npc| npc.id),
            Some(NpcId(0)),
            "Tab skips the escort"
        );
        tap(&mut view, ESCORT_ATTACK_KEY);
        assert_eq!(orders(&view), [Some(EscortOrder::Attack)]);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships attacking target.")
        );
        let escort = &view.session().expect("flying").npcs()[1];
        assert_eq!(escort.target, Some(ShipRef::Npc(NpcId(0))));
        tap(&mut view, ESCORT_DEFEND_KEY);
        assert_eq!(orders(&view), [Some(EscortOrder::Defend)]);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships defending.")
        );
        tap(&mut view, ESCORT_HOLD_KEY);
        assert_eq!(orders(&view), [Some(EscortOrder::Hold)]);
        tap(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(orders(&view), [None]);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships returning to formation.")
        );
        assert_eq!(
            [
                ESCORT_ATTACK_KEY,
                ESCORT_DEFEND_KEY,
                ESCORT_HOLD_KEY,
                ESCORT_RECALL_KEY
            ],
            ['f', 'd', 'v', 'c'].map(Key::Char)
        );
    }

    /// The interface sounds `view` holds, taken.
    fn ui_sounds(view: &mut View) -> Vec<UiSound> {
        view.take_sounds()
            .into_iter()
            .filter_map(|sound| match sound {
                Sound::Ui(sound) => Some(sound),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_escort_menus_keys_beep_as_the_original_does() {
        let mut view = escorted();
        view.take_sounds();
        tap(&mut view, MENU_KEY);
        assert_eq!(ui_sounds(&mut view), [UiSound::EscortMenuToggled], "opened");
        let rows = view.session().expect("flying").escort_menu();
        let present = |n: usize| match EscortGroup::ALL[n] {
            EscortGroup::All => true,
            EscortGroup::Class(class) => rows.iter().any(|row| row.class == class && row.present),
        };
        let absent = (1..5).find(|&n| !present(n)).expect("an absent class");
        let own = (1..5).find(|&n| present(n)).expect("the escort's class");
        tap(&mut view, GROUP_KEYS[absent]);
        assert_eq!(ui_sounds(&mut view), [], "an absent class is not selected");
        tap(&mut view, GROUP_KEYS[own]);
        assert_eq!(ui_sounds(&mut view), [UiSound::EscortGroupSelected]);
        tap(&mut view, ESCORT_HOLD_KEY);
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::EscortsCommanded)],
            "_beepSnd[1]"
        );
        tap(&mut view, MENU_KEY);
        assert_eq!(ui_sounds(&mut view), [UiSound::EscortMenuToggled], "closed");
        tap(&mut view, MENU_KEY);
        view.take_sounds();
        view.tick(super::super::escorts::MENU_TIMEOUT + TICK);
        assert!(!view.escort_menu().is_open());
        assert_eq!(ui_sounds(&mut view), [], "it fades silently");
    }

    #[test]
    fn a_click_in_the_open_escort_menu_over_a_ship_does_nothing() {
        let menu = Point::new(MENU_AT.x + MENU_WIDTH / 2.0, MENU_AT.y + MENU_HEIGHT / 2.0);
        let under = escorted().camera().screen_to_world(menu);
        let place = |at: f32| u32::try_from((at + 750.0).round() as i64).expect("placeable");
        let mut view = escorted_with_traffic(place(under.x), place(under.y));
        let traffic = NpcId(0);
        let over = npc_on_screen(&view, traffic);
        assert!(
            (over.x - menu.x).abs() < 2.0 && (over.y - menu.y).abs() < 2.0,
            "{over:?}"
        );
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        view.take_sounds();
        click(&mut view, over);
        assert_eq!(target_of(&view), None, "not targeted");
        assert_eq!(view.take_sounds(), [], "silent");
        tap(&mut view, MENU_KEY);
        assert!(!view.escort_menu().is_open());
        click(&mut view, over);
        assert_eq!(target_of(&view), Some(traffic), "shut, it targets");
    }

    #[test]
    fn e_with_no_escorts_beeps_no_escorts() {
        let catalog = boardable();
        let pilot = Pilot::new(&catalog, "Alone").expect("starts");
        let mut view = FlightView::with_pilot(catalog, pilot).with_behaviour(Rc::new(Still));
        view.tick(TICK);
        view.take_sounds();
        tap(&mut view, MENU_KEY);
        assert_eq!(view.message(), Some(NO_ESCORTS));
        assert_eq!(ui_sounds(&mut view), [UiSound::NoEscorts]);
    }

    #[test]
    fn a_command_that_changes_nothing_says_nothing() {
        let mut view = escorted();
        tap(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(view.message(), None, "already in formation");
        tap(&mut view, ESCORT_HOLD_KEY);
        assert_lasts(
            &mut view,
            "New escort orders assigned:  All ships holding position.",
            ESCORT_TICKS,
        );
        tap(&mut view, ESCORT_HOLD_KEY);
        assert_eq!(view.message(), None, "held already");
    }

    #[test]
    fn attack_with_no_target_will_attack() {
        let mut view = escorted();
        tap(&mut view, ESCORT_ATTACK_KEY);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships will attack.")
        );
    }

    #[test]
    fn with_the_escort_menu_open_1_to_4_pick_groups_not_stellars() {
        let mut view = escorted();
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        tap(&mut view, Key::Char('2'));
        assert_eq!(nav_target(&view), None, "the menu took it");
        tap(&mut view, Key::Function(2));
        assert_eq!(nav_target(&view), Some(StellarId(129)), "F2 never does");
        tap(&mut view, MENU_CLOSE_KEY);
        assert!(!view.escort_menu().is_open());
        tap(&mut view, Key::Char('1'));
        assert_eq!(nav_target(&view), Some(StellarId(128)), "shut: a stellar");
        assert_eq!(view.escort_menu().group(), EscortGroup::All);
    }

    #[test]
    fn return_clears_the_message_and_closes_the_menu() {
        let mut view = escorted();
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        assert_eq!(view.message(), Some(MISSION_FAILED), "kept by the menu");
        tap(&mut view, MENU_CLOSE_KEY);
        assert!(!view.escort_menu().is_open());
        assert_eq!(view.message(), None, "acknowledged");
        let mut view = flight();
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, MENU_CLOSE_KEY);
        assert_eq!(view.message(), None, "with the menu shut too");
    }

    #[test]
    fn opening_the_escort_menu_clears_no_escorts_only() {
        let mut view = escorted();
        view.say_for(NO_ESCORTS, ESCORT_TICKS);
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        assert_eq!(view.message(), None, "#51 cleared");
        view.say_for(NO_ESCORTS, ESCORT_TICKS);
        tap(&mut view, MENU_KEY);
        assert!(!view.escort_menu().is_open());
        assert_eq!(view.message(), Some(NO_ESCORTS), "closing does not clear");
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        assert_eq!(view.message(), Some(MISSION_FAILED), "kept");
    }

    #[test]
    fn e_opens_the_menu_and_its_keys_pick_the_group_commanded() {
        let mut view = escorted();
        tap(&mut view, GROUP_KEYS[3]);
        assert!(!view.escort_menu().is_open(), "1-5 leave a shut menu alone");
        assert_eq!(view.escort_menu().group(), EscortGroup::All);
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        tap(&mut view, GROUP_KEYS[1]);
        assert_eq!(view.escort_menu().group(), EscortGroup::All, "no fighters");
        tap(&mut view, GROUP_KEYS[3]);
        assert_eq!(
            view.escort_menu().group(),
            EscortGroup::Class(EscortClass::Warship)
        );
        tap(&mut view, ESCORT_HOLD_KEY);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  Warships holding position.")
        );
        tap(&mut view, MENU_CLOSE_KEY);
        assert!(!view.escort_menu().is_open(), "Return closes it");
        tap(&mut view, MENU_KEY);
        tap(&mut view, MENU_KEY);
        assert!(!view.escort_menu().is_open(), "E again closes it");
        assert_eq!(MENU_CLOSE_KEY, Key::Enter);
    }

    /// [`escorted`], its course to Alpha Centauri plotted and the ship
    /// flown out far enough to jump.
    fn escorted_out() -> View {
        let mut view = escorted();
        plot(&mut view, 131);
        fly_out(&mut view);
        view
    }

    #[test]
    fn escort_keys_are_taken_while_the_jump_plays() {
        let mut view = escorted_out();
        jump_now(&mut view);
        view.tick(ms(300));
        view.take_sounds();
        tap(&mut view, ESCORT_HOLD_KEY);
        assert_eq!(orders(&view), [Some(EscortOrder::Hold)]);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships will hold position.")
        );
        assert!(
            view.take_sounds()
                .contains(&Sound::Sim(SimSound::EscortsCommanded)),
            "_beepSnd[1]"
        );
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        assert_eq!(ui_sounds(&mut view), [UiSound::EscortMenuToggled]);
        tap(&mut view, GROUP_KEYS[3]);
        assert_eq!(
            view.escort_menu().group(),
            EscortGroup::Class(EscortClass::Warship)
        );
        assert_eq!(ui_sounds(&mut view), [UiSound::EscortGroupSelected]);
        tap(&mut view, ESCORT_DEFEND_KEY);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  Warships will defend.")
        );
        tap(&mut view, MENU_CLOSE_KEY);
        assert!(!view.escort_menu().is_open(), "Return closes it");
        assert!(view.jump_effect().is_some(), "the streak plays on");
        arrive_now(&mut view);
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
    }

    #[test]
    fn escort_keys_are_taken_while_the_ship_brakes_to_jump() {
        let mut view = escorted_out();
        view.input(&key(JUMP, true));
        assert!(view.preparing_jump(), "braking");
        tap(&mut view, ESCORT_DEFEND_KEY);
        assert_eq!(orders(&view), [Some(EscortOrder::Defend)]);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships will defend.")
        );
    }

    #[test]
    fn option_c_during_the_jump_is_return_to_hangar() {
        let mut view = escorted_out();
        tap(&mut view, ESCORT_HOLD_KEY);
        jump_now(&mut view);
        view.input(&key(Key::Alt, true));
        view.tick(ms(300));
        tap(&mut view, ESCORT_RECALL_KEY);
        view.input(&key(Key::Alt, false));
        assert_eq!(orders(&view), [None], "Return to Hangar: no fighter");
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships will return to formation.")
        );
        tap(&mut view, ESCORT_HOLD_KEY);
        tap(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(orders(&view), [None]);
        let mut fighters = fighters_out();
        plot(&mut fighters, 131);
        fly_out(&mut fighters);
        jump_now(&mut fighters);
        option(&mut fighters, ESCORT_RECALL_KEY);
        assert_eq!(orders(&fighters), [Some(EscortOrder::Dock); 2]);
        assert_eq!(
            fighters.message(),
            Some("New escort orders assigned:  All ships will return to hangar.")
        );
    }

    #[test]
    fn option_released_during_the_jump_lets_c_recall() {
        let mut view = fighters_out();
        plot(&mut view, 131);
        fly_out(&mut view);
        tap(&mut view, ESCORT_HOLD_KEY);
        jump_now(&mut view);
        view.input(&key(Key::Alt, true));
        view.tick(ms(300));
        view.input(&key(Key::Alt, false));
        tap(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(orders(&view), [None; 2], "C alone: formation, not Dock");
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships will return to formation.")
        );
    }

    #[test]
    fn the_escort_menu_times_out_during_the_streak() {
        // How long the ship brakes and turns, timed on a twin.
        let mut twin = escorted_out();
        let mut braking = Duration::ZERO;
        twin.input(&key(JUMP, true));
        while twin.jump_effect().is_none() {
            twin.tick(TICK);
            braking += TICK;
        }
        // Opened so that it times out half a second into hyperspace.
        let mut view = escorted_out();
        tap(&mut view, MENU_KEY);
        let wait = super::super::escorts::MENU_TIMEOUT
            .checked_sub(braking + ms(500))
            .expect("the braking is shorter than the menu's timeout");
        view.tick(wait);
        jump_now(&mut view);
        assert!(view.escort_menu().is_open(), "open as the streak begins");
        let in_hyperspace = |view: &View| view.session().expect("flying").jumping().is_some();
        while view.escort_menu().is_open() {
            assert!(in_hyperspace(&view), "shut before the ship arrives");
            view.tick(TICK);
        }
        assert!(in_hyperspace(&view), "shut within the streak");
    }

    #[test]
    fn the_menu_closes_8_seconds_after_its_last_command() {
        let mut view = escorted();
        tap(&mut view, MENU_KEY);
        view.tick(Duration::from_secs(6));
        tap(&mut view, ESCORT_HOLD_KEY);
        view.tick(Duration::from_secs(6));
        assert!(view.escort_menu().is_open(), "the command kept it open");
        view.tick(Duration::from_secs(3));
        assert!(!view.escort_menu().is_open());
    }

    #[test]
    fn e_with_no_escorts_says_so() {
        let mut view = flight();
        tap(&mut view, MENU_KEY);
        assert!(!view.escort_menu().is_open());
        assert_eq!(view.message(), Some("You don't have any escorts."));
        tap(&mut view, ESCORT_ATTACK_KEY);
        assert_eq!(
            view.message(),
            Some("You don't have any escorts."),
            "no command"
        );
        assert_lasts(&mut view, NO_ESCORTS, 250);
    }

    #[test]
    fn option_tab_targets_the_escorts() {
        let mut view = escorted();
        view.input(&key(Key::Alt, true));
        tap(&mut view, TARGET_KEY);
        view.input(&key(Key::Alt, false));
        assert_eq!(
            view.session().expect("flying").target().map(|npc| npc.id),
            Some(NpcId(1))
        );
    }

    #[test]
    fn shift_tab_targets_back_through_the_ships_and_with_alt_back_through_the_escorts() {
        let mut catalog = boardable();
        for record in &mut catalog.ships {
            if record.id == ShipId(130) {
                record.escort_type = 2;
                record.inherent_ai = 3;
            }
        }
        catalog.traffic[0].1.avg_ships = 2;
        let pilot = with_escorts(
            &Pilot::new(&catalog, "Escorted").expect("starts"),
            &[130, 130],
        );
        let mut draws = placed(750, 650, 0).to_vec();
        draws.extend(placed(850, 650, 0));
        let (_, chance) = scripted(&draws);
        let mut view = FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let session = view.session().expect("flying");
        let last = |escorts: bool| {
            session
                .npcs()
                .iter()
                .filter(|npc| session.is_escort(npc.id) == escorts)
                .max_by_key(|npc| npc.berth)
                .map(|npc| npc.id)
        };
        let (ship, escort) = (last(false), last(true));
        let escort_berth = session
            .npcs()
            .iter()
            .find(|npc| Some(npc.id) == escort)
            .map(|npc| npc.berth);
        // Tab from the last escort: the first ship above its berth.
        let after_escort = session
            .npcs()
            .iter()
            .filter(|npc| !session.is_escort(npc.id) && Some(npc.berth) > escort_berth)
            .min_by_key(|npc| npc.berth)
            .map(|npc| npc.id);
        assert!(after_escort.is_some(), "{:?}", session.npcs());
        let ships = session.npcs().len();
        assert!(ships >= 4, "two of each: {:?}", session.npcs());
        let target = |view: &View| view.session().expect("flying").target().map(|npc| npc.id);
        view.input(&key(Key::Shift, true));
        tap(&mut view, TARGET_KEY);
        assert_eq!(target(&view), ship);
        view.input(&key(Key::Alt, true));
        tap(&mut view, TARGET_KEY);
        assert_eq!(target(&view), escort);
        view.input(&key(Key::Shift, false));
        view.input(&key(Key::Alt, false));
        tap(&mut view, TARGET_KEY);
        assert_eq!(target(&view), after_escort, "Tab alone walks forward again");
    }

    #[test]
    fn the_escorts_sheets_are_read_with_the_traffics() {
        let view = escorted();
        let asked = view.catalog().sheets_asked.borrow();
        assert!(asked.contains(&ShipId(130)), "{asked:?}");
    }

    #[test]
    fn the_open_menu_is_drawn_over_the_flight_under_the_hud() {
        let mut view = escorted().with_overlays(TITLED);
        let mut shut = DrawList::new();
        view.draw(&mut shut);
        tap(&mut view, MENU_KEY);
        let mut open = DrawList::new();
        view.draw(&mut open);
        let commands: Vec<_> = open.iter().collect();
        let title = commands
            .iter()
            .position(|command| matches!(command, DrawCommand::Text { text, .. } if text == "Escort Commands"))
            .expect("the menu");
        let frame = commands
            .iter()
            .position(|command| {
                matches!(command, DrawCommand::Line { color, .. } if *color == MENU_COLORS.border)
            })
            .expect("its border");
        let help = commands
            .iter()
            .position(|command| **command == overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM))
            .expect("the help line");
        let player = commands
            .iter()
            .position(|command| matches!(command, DrawCommand::Sprite { image, .. } if *image == ImageKey::sprite(2000, 0)))
            .expect("the player");
        assert!(
            player < frame && frame < title && title < help,
            "{player} {frame} {title} {help}"
        );
        assert_eq!(
            open.len() - shut.len(),
            1 + 4 + 1 + 5 + 1 + 1,
            "fill, border, title, rows, bar, order"
        );
        assert!(commands.iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, color, .. } if text == "2) Fighters" && *color == ABSENT_ROW
        )));
        assert!(commands.iter().any(|command| matches!(
            command,
            DrawCommand::Line { from, .. } if from.x == MENU_AT.x && from.y == MENU_AT.y + MENU_HEIGHT / 2.0
        )), "filled across {MENU_WIDTH}");
    }

    #[test]
    fn the_help_line_names_the_escort_keys() {
        assert!(
            HELP.contains("F/D/V/C: escorts   Alt-C: dock   E: escort menu"),
            "{HELP}"
        );
    }

    // Hiring escorts and paying them.

    use nova_sim::hire::DEFECTED_ONE;
    use nova_sim::{ControlBits, HireRefusal, HireTerms};

    /// [`shipbuying`] with its stellar a bar too, and ship 129 (`Cost`
    /// 900) for hire half the days.
    fn hiring() -> FakeCatalog {
        let mut catalog = shipbuying();
        catalog.sites[0].flags |= StellarFlags::BAR;
        for record in &mut catalog.ships {
            if record.id == ShipId(129) {
                record.hire_random = 50;
            }
        }
        catalog
    }

    #[test]
    fn hiring_goes_through_the_session_on_the_flights_chance() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::new(hiring()).with_chance(SharedChance::new(shared));
        assert_eq!(view.escorts_for_hire(), None, "in flight");
        assert_eq!(view.hire(ShipId(129)), Err(HireRefusal::NoBar));
        land_now(&mut view);
        view.take_save_due();
        let list = view.escorts_for_hire().expect("a bar");
        let row = list.row(ShipId(129)).expect("for hire");
        assert_eq!((row.fee, row.wage), (90, 9));
        assert_eq!(always.borrow().asked, [50]);
        let hired = view.hire(ShipId(129)).expect("hired");
        assert_eq!((hired.fee, hired.wage), (90, 9));
        assert_eq!(view.pilot().map(Pilot::cash), Some(1000 - 90));
        assert_eq!(view.pilot().expect("flying").escorts()[0].wage, Some(9));
        assert!(view.take_save_due());
        let mut broken = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..hiring()
        });
        assert_eq!(broken.escorts_for_hire(), None);
        assert_eq!(broken.hire(ShipId(129)), Err(HireRefusal::NoBar));
    }

    #[test]
    fn the_bars_news_is_drawn_through_the_session_on_the_flights_chance() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let strings = FakeCatalog {
            strings: vec![
                (8100, vec!["C1".to_owned(), "C2".to_owned()]),
                (8101, vec!["G1".to_owned(), "G2".to_owned()]),
            ],
            ..catalog()
        };
        let mut view = FlightView::new(hiring())
            .with_chance(SharedChance::new(shared))
            .with_strings(Rc::new(strings));
        assert!(!view.open_bar(), "in flight");
        assert_eq!(view.news(), None);
        assert_eq!(view.news_picture(), None, "in flight");
        land_now(&mut view);
        assert_eq!(view.news_picture(), Some(9000), "no NewsPic");
        assert!(view.open_bar());
        assert_eq!(
            view.news(),
            Some(&nova_sim::News {
                commercial: "C2".to_owned(),
                story: "G2".to_owned(),
            }),
            "each the last string, as the flight's chance rolls"
        );
        let mut broken = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..hiring()
        });
        assert!(!broken.open_bar());
        assert_eq!(broken.news(), None);
        assert_eq!(broken.news_picture(), None);
    }

    // The mission screens.

    use nova_sim::MissionTextKind;
    use nova_sim::catalog::{MissionRecord, MissionShip, MissionTexts};
    use nova_sim::control::{Script as SetScript, Test};

    /// Mission `id`, offered at the mission computer of any stellar on
    /// every visit, to anyone, going nowhere.
    fn errand(id: i16) -> MissionRecord {
        MissionRecord {
            id: MissionId(id),
            avail_stel: -1,
            avail_loc: 0,
            avail_record: 0,
            avail_rating: 0,
            avail_random: 100,
            avail_ship_type: 0,
            avail_bits: Test::default(),
            require: 0,
            flags: 0,
            flags2: 0,
            travel_stel: -1,
            return_stel: -1,
            cargo_type: -1,
            cargo_qty: 0,
            pickup_mode: -1,
            drop_off_mode: -1,
            scan_mask: 0,
            pay_val: 0,
            time_limit: 0,
            disp_weight: 0,
            on_accept: SetScript::default(),
            on_refuse: SetScript::default(),
            on_success: SetScript::default(),
            on_failure: SetScript::default(),
            on_abort: SetScript::default(),
            on_ship_done: SetScript::default(),
            can_abort: 1,
            comp_govt: None,
            comp_reward: 0,
            date_post_inc: 0,
            ships: MissionShip::NONE,
            aux: nova_sim::catalog::AuxShip::NONE,
        }
    }

    #[test]
    fn the_mission_screens_go_through_the_session() {
        let mut errands = catalog();
        errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        errands.missions = vec![errand(130), errand(131)];
        errands.mission_texts = vec![
            MissionTexts {
                brief: Some("Go.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            },
            MissionTexts {
                refuse: Some("Fine.".to_owned()),
                ..MissionTexts::named(MissionId(131), "Walk")
            },
        ];
        let mut view = FlightView::new(errands);
        assert_eq!(
            view.mission_computer(),
            Err(ComputerRefusal::NoneAvailable),
            "in flight"
        );
        assert_eq!(
            view.accept_mission(MissionId(130)),
            Err(AcceptError::NotOffered)
        );
        assert_eq!(view.refuse_mission(MissionId(131)), Err(NotOffered));
        land_now(&mut view);
        view.settle_script();
        let computer = view.mission_computer().expect("opens");
        let names: Vec<_> = computer.rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["Run", "Walk"]);
        assert_eq!(view.open_mission_computer(), Ok(computer.clone()));
        assert_eq!(
            view.show_computer_row(MissionId(131)),
            Some(computer.rows[1].clone()),
            "loaded through the session"
        );
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        assert_eq!(view.refuse_mission(MissionId(131)).map(|_| ()), Ok(()));
        let texts: Vec<_> = view
            .take_mission_texts()
            .into_iter()
            .map(|shown| match shown {
                MissionShown::Text(text) => (text.kind, text.text),
                MissionShown::Movie(movie) => panic!("an offer movie: {movie:?}"),
            })
            .collect();
        assert_eq!(
            texts,
            [
                (MissionTextKind::Brief, "Go.".to_owned()),
                (MissionTextKind::Refuse, "Fine.".to_owned()),
            ]
        );
        assert_eq!(view.take_mission_texts(), [], "taken");
        let mut broken = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert_eq!(
            broken.mission_computer(),
            Err(ComputerRefusal::NoneAvailable)
        );
        assert_eq!(
            broken.open_mission_computer(),
            Err(ComputerRefusal::NoneAvailable)
        );
        assert_eq!(broken.show_computer_row(MissionId(130)), None);
        assert_eq!(
            broken.accept_mission(MissionId(130)),
            Err(AcceptError::NotOffered)
        );
        assert_eq!(broken.refuse_mission(MissionId(130)), Err(NotOffered));
        assert_eq!(broken.take_mission_texts(), []);
    }

    #[test]
    fn closing_a_mission_dialog_or_its_texts_redraws_the_hud() {
        let mut errands = catalog();
        errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        errands.missions = vec![errand(130), errand(131)];
        errands.mission_texts = vec![MissionTexts {
            brief: Some("Go.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        }];
        let mut view = FlightView::new(errands);
        let stale = RadarScene::default();
        let fresh = view.radar_now();
        assert_ne!(fresh, stale);
        view.radar_scene = Some(stale.clone());
        let _ = view.accept_mission(MissionId(130));
        assert_eq!(view.radar_scene.as_ref(), Some(&fresh), "accepted");
        view.radar_scene = Some(stale.clone());
        let _ = view.refuse_mission(MissionId(130));
        assert_eq!(view.radar_scene.as_ref(), Some(&fresh), "refused");
        view.radar_scene = Some(stale.clone());
        assert_eq!(view.take_mission_texts(), []);
        assert_eq!(view.radar_scene.as_ref(), Some(&stale), "nothing shown");
        land_now(&mut view);
        view.settle_script();
        // Docked, the held gauges are redrawn too.
        let held = Reserves::default();
        assert_ne!(view.reserves(), held);
        view.reserves_scene = Some(held);
        assert_eq!(view.reserves_shown(), held, "held");
        assert_eq!(view.refuse_mission(MissionId(131)).map(|_| ()), Ok(()));
        assert_eq!(view.reserves_shown(), view.reserves(), "refused docked");
        view.reserves_scene = Some(held);
        view.radar_scene = Some(stale.clone());
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        assert_eq!(view.take_mission_texts().len(), 1);
        assert_eq!(view.reserves_shown(), held, "the brief shown over the bar");
        assert_eq!(view.radar_scene.as_ref(), Some(&stale), "the brief shown");
        view.close_mission_text();
        assert_eq!(view.radar_scene, Some(view.radar_now()), "the brief closed");
        assert_eq!(view.reserves_shown(), view.reserves(), "the brief closed");
    }

    #[test]
    fn entering_another_venue_through_the_flight_offers_a_suppressed_offer_again() {
        let mut errands = catalog();
        errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        errands.missions = (130..146).map(errand).collect();
        errands.missions.push(MissionRecord {
            avail_loc: 1,
            flags: nova_sim::mission::screens::UNREFUSABLE,
            ..errand(150)
        });
        let mut view = FlightView::new(errands);
        land_now(&mut view);
        view.settle_script();
        for id in 130..146 {
            assert!(view.accept_mission(MissionId(id)).is_ok(), "{id}");
        }
        view.enter_offer_venue(1);
        let refused = Some(VenueOffer::Accepted {
            mission: MissionId(150),
            accepted: Err(AcceptError::Refused(
                nova_sim::mission::AcceptRefusal::SlotsFull,
            )),
        });
        assert_eq!(view.next_offer(1), refused);
        assert_eq!(view.next_offer(1), None, "suppressed");
        view.enter_offer_venue(3);
        view.enter_offer_venue(1);
        assert_eq!(view.next_offer(1), refused, "offered again");
    }

    /// A flight from [`trading`]'s Earth, its ship of 10 tons, that
    /// accepted mission 130, carrying 4 tons of food loaded at accept,
    /// `Flags` `flags`, and bought the other 6 tons full of food when
    /// `food`, then took off.
    fn carrying(flags: u16, food: bool) -> View {
        carrying_in(trading(), Rulebook::default(), flags, food)
    }

    /// [`carrying`], from `errands` under `rules`.
    fn carrying_in(mut errands: FakeCatalog, rules: Rulebook, flags: u16, food: bool) -> View {
        errands.missions = vec![MissionRecord {
            cargo_type: 0,
            cargo_qty: 4,
            pickup_mode: 0,
            flags,
            ..errand(130)
        }];
        let mut view = FlightView::new(errands).with_rules(rules);
        land_now(&mut view);
        view.settle_script();
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        if food {
            assert_eq!(view.trade(BUY_FOOD), Ok(6), "up to the free space");
        }
        view.take_off().expect("takes off");
        view.settle_script();
        // Past the take-off greeting.
        view.message = None;
        view
    }

    /// Holds Command (and Alt with `option`), taps `k`, and lets go.
    fn command_tap(view: &mut View, k: Key, option: bool) {
        view.input(&key(Key::Command, true));
        if option {
            view.input(&key(Key::Alt, true));
        }
        tap(view, k);
        if option {
            view.input(&key(Key::Alt, false));
        }
        view.input(&key(Key::Command, false));
    }

    fn cargo_aboard(view: &View) -> i64 {
        view.pilot().expect("flying").total_cargo()
    }

    #[test]
    fn command_k_jettisons_the_cargo_failing_the_mission_and_says_so() {
        let mut view = carrying(0, true);
        assert_eq!(cargo_aboard(&view), 10);
        command_tap(&mut view, JETTISON_KEY, false);
        assert_eq!(cargo_aboard(&view), 0);
        assert_eq!(view.pilot().expect("flying").missions().iter().count(), 0);
        assert_eq!(view.message(), Some(MISSION_FAILED));
        assert_eq!(JETTISON_KEY, Key::Char('k'));
        assert_eq!(MISSION_FAILED, "Mission failed.");
        assert!(HELP.contains("Cmd-K: jettison"), "{HELP}");
    }

    #[test]
    fn command_option_k_keeps_the_mission_cargo() {
        let mut view = carrying(0, true);
        command_tap(&mut view, JETTISON_KEY, true);
        assert_eq!(cargo_aboard(&view), 4, "the food bought went");
        assert_eq!(view.message(), Some(NON_MISSION_CARGO_JETTISONED));
        assert_eq!(
            NON_MISSION_CARGO_JETTISONED,
            "Non-mission cargo jettisoned."
        );
        let mut view = carrying(0, false);
        command_tap(&mut view, JETTISON_KEY, true);
        assert_eq!(cargo_aboard(&view), 4, "kept");
        assert_eq!(view.message(), None, "nothing else held");
    }

    #[test]
    fn jettison_all_empties_the_hold_and_says_so() {
        let mut view = carrying(0x0400, false);
        assert!(cargo_aboard(&view) > 0);
        view.jettison_all();
        assert_eq!(cargo_aboard(&view), 0);
        assert_eq!(view.message(), Some(CARGO_JETTISONED));
    }

    #[test]
    fn a_hidden_missions_cargo_jettisoned_says_the_cargo_went() {
        let mut view = carrying(0x0400, false);
        command_tap(&mut view, JETTISON_KEY, false);
        assert_eq!(cargo_aboard(&view), 0);
        assert_eq!(view.message(), Some(CARGO_JETTISONED));
        assert_eq!(CARGO_JETTISONED, "Cargo jettisoned.");
    }

    /// The cargo boxes' sprites drawn, `rlëD` 500's, in the order drawn.
    fn box_sprites(list: &DrawList) -> Vec<(ImageKey, Point)> {
        sprites(list)
            .into_iter()
            .filter(|(image, _)| image.id == 500)
            .collect()
    }

    #[test]
    fn command_k_drops_cargo_boxes_drawn_under_the_ship_where_they_drift() {
        let mut view = carrying(0, true);
        // Turned off straight up, so the boxes drift on both axes.
        view.input(&key(Key::Right, true));
        ticks(&mut view, 10);
        view.input(&key(Key::Right, false));
        assert!(box_sprites(&drawn(&view)).is_empty());
        let _ = view.take_sounds();
        command_tap(&mut view, JETTISON_KEY, false);
        let session = view.session().expect("flying");
        let boxes = session.cargo_boxes().to_vec();
        assert_eq!(boxes.len(), 2, "10 tons");
        assert!(
            boxes[0].velocity.x.abs() > 0.1 && boxes[0].velocity.y.abs() > 0.1,
            "{boxes:?}"
        );
        // A quarter of the way from the last step to this one.
        view.alpha = 0.25;
        let camera = view.camera();
        let behind = 0.75;
        let expected: Vec<_> = boxes
            .iter()
            .map(|made| {
                (
                    ImageKey::sprite(500, made.frame),
                    camera.world_to_screen(Point::new(
                        made.velocity.x.mul_add(-behind, made.position.x),
                        made.velocity.y.mul_add(-behind, made.position.y),
                    )),
                )
            })
            .collect();
        let list = drawn(&view);
        assert_eq!(box_sprites(&list), expected);
        let all = sprites(&list);
        let first_box = all.iter().position(|(image, _)| image.id == 500);
        let ship = all.iter().position(|(image, _)| image.id == 2000);
        assert!(first_box < ship, "under the ship: {all:?}");
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::CargoJettisoned)],
            "snd 154"
        );
    }

    /// `base` with asteroid type 0, its sheet `rlëD` 1800, and system 130
    /// holding 3 of it.
    fn rocky(base: FakeCatalog) -> FakeCatalog {
        FakeCatalog {
            asteroid_types: vec![nova_sim::AsteroidTypeRecord {
                kind: 0,
                strength: 50,
                spin_rate: 100,
                frames: 36,
                frame_size: Some((40, 40)),
                ..nova_sim::AsteroidTypeRecord::default()
            }],
            asteroid_fields: vec![nova_sim::AsteroidFieldRecord {
                system: SystemId(130),
                count: 3,
                types: 0x0001,
            }],
            ..base
        }
    }

    /// The rules with `key` read as the other reading.
    fn other_reading(key: RuleKey) -> Rulebook {
        Rulebook::default().with_override(key, RuleSource::Bible)
    }

    fn asteroid_count(view: &View) -> usize {
        view.session().expect("flying").asteroids().count()
    }

    #[test]
    fn each_step_spawns_an_asteroid_at_the_edge_until_the_systems_count() {
        // Just off screen, so none is culled as it spawns.
        let mut view = FlightView::new(rocky(catalog()))
            .with_rules(other_reading(RuleKey::AsteroidRespawnRing));
        assert_eq!(asteroid_count(&view), 0);
        for count in [1, 2, 3, 3] {
            ticks(&mut view, 1);
            assert_eq!(asteroid_count(&view), count);
        }
    }

    #[test]
    fn a_flight_started_in_flight_shows_its_placed_field_before_any_step() {
        let mut view = FlightView::new(rocky(catalog()))
            .with_rules(other_reading(RuleKey::AsteroidEntryField));
        view.tick(Duration::ZERO);
        assert_eq!(asteroid_count(&view), 3, "placed, no step run");
        let list = drawn(&view);
        let asteroids = sprites(&list)
            .into_iter()
            .filter(|(image, _)| image.id == 1800)
            .count();
        assert_eq!(asteroids, 3);
        let mut engine = FlightView::new(rocky(catalog()));
        engine.tick(Duration::ZERO);
        assert_eq!(asteroid_count(&engine), 0, "by the engine");
    }

    #[test]
    fn an_asteroid_is_its_types_frame_drawn_where_it_is_over_the_stellars_and_under_the_boxes() {
        let rules = other_reading(RuleKey::AsteroidRespawnRing);
        let mut view = carrying_in(rocky(trading()), rules, 0, true);
        ticks(&mut view, 2);
        command_tap(&mut view, JETTISON_KEY, false);
        view.alpha = 0.25;
        let session = view.session().expect("flying");
        let made: Vec<_> = session.asteroids().copied().collect();
        assert!(!made.is_empty());
        let camera = view.camera();
        let expected: Vec<_> = made
            .iter()
            .map(|asteroid| {
                (
                    ImageKey::sprite(1800, asteroid.frame as u16),
                    camera.world_to_screen(Point::new(
                        asteroid.velocity.x.mul_add(-0.75, asteroid.position.x),
                        asteroid.velocity.y.mul_add(-0.75, asteroid.position.y),
                    )),
                )
            })
            .collect();
        let list = drawn(&view);
        let drawn_asteroids: Vec<_> = sprites(&list)
            .into_iter()
            .filter(|(image, _)| image.id == 1800)
            .collect();
        assert_eq!(drawn_asteroids, expected);
        let stellar = first(&list, sprite_of(1128));
        let asteroid = first(&list, sprite_of(1800));
        let cargo_box = first(&list, sprite_of(500));
        assert!(stellar < asteroid && asteroid < cargo_box, "{list:?}");
    }

    #[test]
    fn a_landing_clears_the_cargo_boxes() {
        let mut view = carrying(0, true);
        command_tap(&mut view, JETTISON_KEY, false);
        assert_eq!(box_sprites(&drawn(&view)).len(), 2);
        land_now(&mut view);
        assert!(view.session().expect("flying").landed().is_some());
        assert!(view.session().expect("flying").cargo_boxes().is_empty());
        assert!(box_sprites(&drawn(&view)).is_empty());
    }

    #[test]
    fn a_jump_clears_the_cargo_boxes() {
        let mut view = carrying(0, true);
        ticks(&mut view, nova_sim::TAKEOFF_GRACE_TICKS);
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        if let Ok(session) = &mut view.session {
            session.jettison_cargo(true, &mut nova_sim::NeverFires);
        }
        assert_eq!(view.session().expect("flying").cargo_boxes().len(), 2);
        arrive_now(&mut view);
        assert!(box_sprites(&drawn(&view)).is_empty());
    }

    #[test]
    fn k_alone_jettisons_nothing() {
        let mut view = carrying(0, false);
        tap(&mut view, JETTISON_KEY);
        assert_eq!(cargo_aboard(&view), 4);
        assert_eq!(view.message(), None);
        view.input(&key(Key::Alt, true));
        tap(&mut view, JETTISON_KEY);
        view.input(&key(Key::Alt, false));
        assert_eq!(cargo_aboard(&view), 4, "Option alone");
        command_tap(&mut view, Key::Char('j'), false);
        assert_eq!(cargo_aboard(&view), 4, "another key with Command");
    }

    #[test]
    fn each_jettison_message_is_worded_and_the_arrivals_are_not_among_them() {
        let view = flight();
        let session = view.session().expect("flying");
        let text = |message| sim_message_text(&message, session);
        assert_eq!(
            text(SimMessage::MissionFailed).as_deref(),
            Some(MISSION_FAILED)
        );
        assert_eq!(
            text(SimMessage::CargoJettisoned).as_deref(),
            Some(CARGO_JETTISONED)
        );
        assert_eq!(
            text(SimMessage::NonMissionCargoJettisoned).as_deref(),
            Some(NON_MISSION_CARGO_JETTISONED)
        );
        assert_eq!(text(SimMessage::Arrived(SystemId(130))), None);
        assert_eq!(text(SimMessage::ExitedHypergate(SystemId(130))), None);
        assert_eq!(text(SimMessage::PassedWormhole(SystemId(130))), None);
        assert_eq!(text(SimMessage::Buoy(HELLO.to_owned())), None);
        for message in [
            SimMessage::MissionFailed,
            SimMessage::CargoJettisoned,
            SimMessage::NonMissionCargoJettisoned,
            SimMessage::MissionScanned,
            SimMessage::Smuggling {
                scanner: ShipId(129),
                ship_name: None,
                fine: 0,
            },
        ] {
            assert_eq!(arrival_lead(&message), None, "{message:?}");
        }
    }

    #[test]
    fn a_former_escort_rejoining_says_escort_or_fighter_repaired() {
        assert_eq!(ESCORT_REPAIRED, "Escort repaired.");
        assert_eq!(FIGHTER_REPAIRED, "Fighter repaired.");
        let view = flight();
        let session = view.session().expect("flying");
        let text = |message| sim_message_text(&message, session);
        assert_eq!(
            text(SimMessage::EscortRepaired).as_deref(),
            Some(ESCORT_REPAIRED)
        );
        assert_eq!(
            text(SimMessage::FighterRepaired).as_deref(),
            Some(FIGHTER_REPAIRED)
        );
        for message in [SimMessage::EscortRepaired, SimMessage::FighterRepaired] {
            assert_eq!(arrival_lead(&message), None, "{message:?}");
        }
    }

    /// A flight whose one outfit is called "bomb", "bombs".
    fn with_a_bomb() -> FlightView<FakeCatalog> {
        FlightView::new(FakeCatalog {
            outfits: vec![OutfitRecord {
                lc_name: "bomb".to_owned(),
                lc_plural: "bombs".to_owned(),
                ..multi_jumping().outfits[0].clone()
            }],
            ..catalog()
        })
    }

    #[test]
    fn a_sabotage_notice_names_the_outfit_for_its_count() {
        assert_eq!(
            (YOUR, HAS_EXPLODED, HAVE_EXPLODED),
            ("Your", "has exploded!", "have exploded!")
        );
        let view = with_a_bomb();
        let session = view.session().expect("flying");
        let sabotaged = |one| SimMessage::Sabotaged {
            outfit: Some((OutfitId(275), one)),
        };
        assert_eq!(
            sim_message_text(&sabotaged(true), session).as_deref(),
            Some("Your bomb has exploded!")
        );
        assert_eq!(
            sim_message_text(&sabotaged(false), session).as_deref(),
            Some("Your bombs have exploded!")
        );
        assert_eq!(arrival_lead(&sabotaged(true)), None);
    }

    #[test]
    fn a_bomb_notice_names_the_outfit_with_no_empty_name_check() {
        let view = with_a_bomb();
        let session = view.session().expect("flying");
        let exploded = |outfit, one| SimMessage::BombExploded {
            outfit: OutfitId(outfit),
            one,
        };
        assert_eq!(
            sim_message_text(&exploded(275, true), session).as_deref(),
            Some("Your bomb has exploded!")
        );
        assert_eq!(
            sim_message_text(&exploded(275, false), session).as_deref(),
            Some("Your bombs have exploded!")
        );
        assert_eq!(
            sim_message_text(&exploded(999, true), session).as_deref(),
            Some("Your  has exploded!"),
            "an empty name is spelled out as it is"
        );
        assert_eq!(arrival_lead(&exploded(275, true)), None);
    }

    #[test]
    fn the_leet_sabotage_notice_says_someone_set_up_the_bomb() {
        let view = flight();
        let session = view.session().expect("flying");
        assert_eq!(
            sim_message_text(&SimMessage::BombSetUp, session).as_deref(),
            Some("Someone set up us the bomb!")
        );
        assert_eq!(arrival_lead(&SimMessage::BombSetUp), None);
    }

    #[test]
    fn a_sabotage_notice_without_an_outfit_says_a_bomb_exploded() {
        let view = flight();
        let session = view.session().expect("flying");
        let message = SimMessage::Sabotaged { outfit: None };
        assert_eq!(
            sim_message_text(&message, session).as_deref(),
            Some("A bomb has exploded onboard your ship!")
        );
        assert_eq!(
            BOMB_EXPLODED_ONBOARD,
            "A bomb has exploded onboard your ship!"
        );
    }

    #[test]
    fn a_collapsed_jump_says_the_hyperspace_field_collapsed() {
        let view = flight();
        let session = view.session().expect("flying");
        assert_eq!(
            sim_message_text(&SimMessage::JumpCollapsed, session).as_deref(),
            Some("Ship disabled - hyperspace field collapsed.")
        );
        assert_eq!(
            JUMP_COLLAPSED,
            "Ship disabled - hyperspace field collapsed."
        );
        assert_eq!(arrival_lead(&SimMessage::JumpCollapsed), None);
    }

    #[test]
    fn the_escape_ship_says_escaping_in_its_long_name() {
        assert_eq!(ESCAPING_IN, "Escaping in");
        let mut catalog = shipbuying();
        catalog.ships[0].long_name = "Federation Viper".to_owned();
        let view = FlightView::new(catalog);
        let session = view.session().expect("flying");
        let escaping = |ship| SimMessage::EscapingIn { ship: ShipId(ship) };
        assert_eq!(
            sim_message_text(&escaping(129), session).as_deref(),
            Some("Escaping in Federation Viper.")
        );
        assert_eq!(
            sim_message_text(&escaping(999), session).as_deref(),
            Some("Escaping in ."),
            "a class with no record has no long name"
        );
        assert_eq!(arrival_lead(&escaping(129)), None);
    }

    #[test]
    fn a_mission_ship_boarded_on_the_assist_path_says_its_line() {
        assert_eq!(TARGET_BOARDED, "Target ship has been boarded.");
        assert_eq!(
            refuelled_message("Freighter", "Kestrel"),
            "Freighter:  Energy transfer complete, Kestrel."
        );
        let view = flight();
        let session = view.session().expect("flying");
        assert_eq!(
            mission_board_message(MissionBoardLine::Boarded, session),
            TARGET_BOARDED
        );
    }

    #[test]
    fn a_pickup_ships_line_reads_the_cargo_name_from_str_4001() {
        let strings = FakeCatalog {
            strings: vec![(4001, vec!["Food".to_owned(), "*Passengers".to_owned()])],
            ..catalog()
        };
        let view = flight().with_strings(Rc::new(strings));
        let session = view.session().expect("flying");
        let said = |cargo| mission_board_message(MissionBoardLine::Retrieved { cargo }, session);
        assert_eq!(said(Some(0)), "You retrieved  the Food from this ship.");
        assert_eq!(said(Some(1)), "You retrieved  Passengers from this ship.");
        assert_eq!(said(Some(2)), "You retrieved  the  from this ship.");
        assert_eq!(said(None), "You retrieved  the  from this ship.");
        assert_eq!(said(Some(256)), "You retrieved  the  from this ship.");
    }

    #[test]
    fn a_mission_ship_lost_says_str_2002_284() {
        let view = flight();
        let session = view.session().expect("flying");
        assert_eq!(
            sim_message_text(&SimMessage::MissionShipLost, session).as_deref(),
            Some(MISSION_FAILED)
        );
        assert_eq!(arrival_lead(&SimMessage::MissionShipLost), None);
    }

    #[test]
    fn a_fleets_arrival_quote_says_its_line() {
        let view = flight();
        let session = view.session().expect("flying");
        let quote = SimMessage::FleetArrivalQuote("Hail 9".to_owned());
        assert_eq!(sim_message_text(&quote, session).as_deref(), Some("Hail 9"));
        assert_eq!(arrival_lead(&quote), None);
    }

    #[test]
    fn the_players_downed_lines_are_str_2002_287_288_and_284() {
        let view = flight();
        let session = view.session().expect("flying");
        for (message, text) in [
            (SimMessage::ShipDisabled, "Ship disabled."),
            (SimMessage::ShipDestroyed, "Ship destroyed."),
            (SimMessage::DownedMissionFailed, "Mission failed."),
        ] {
            assert_eq!(
                sim_message_text(&message, session).as_deref(),
                Some(text),
                "{message:?}"
            );
            assert_eq!(arrival_lead(&message), None, "{message:?}");
        }
    }

    #[test]
    fn the_no_active_missions_message_is_str_2002_354() {
        let view = flight();
        let session = view.session().expect("flying");
        assert_eq!(
            sim_message_text(&SimMessage::NoActiveMissions, session).as_deref(),
            Some(NO_ACTIVE_MISSIONS)
        );
        assert_eq!(arrival_lead(&SimMessage::NoActiveMissions), None);
    }

    #[test]
    fn the_deadline_message_is_str_2002_285() {
        let view = flight();
        let session = view.session().expect("flying");
        assert_eq!(
            sim_message_text(&SimMessage::DeadlineMissed, session).as_deref(),
            Some(TIME_LIMIT_EXCEEDED)
        );
        assert_eq!(arrival_lead(&SimMessage::DeadlineMissed), None);
        assert_eq!(TIME_LIMIT_EXCEEDED, "Time limit exceeded - mission failed.");
    }

    /// Inspects the player on its first decision; then it has looked the
    /// player over, and idles.
    #[derive(Debug, Default)]
    struct Inspector(std::cell::Cell<bool>);

    impl Behaviour for Inspector {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            if self.0.replace(true) {
                Goal::Idle
            } else {
                Goal::Inspect(ShipRef::Player)
            }
        }
    }

    /// A flight in Sol with 1200 credits, its ship named Kestrel, with
    /// mission 130's 4 tons
    /// of food (`ScanMask` 0x0004, `Flags` `flags`) aboard in slot 0, and
    /// one interceptor (ship 129) of a government scanning for 0x0004 and
    /// fining `fine`, placed on the ship, that inspects it, its scan
    /// drawing 0.
    fn smuggling(flags: u16, fine: i16) -> View {
        let mut catalog = trafficked(&[130], 1, 129, 4);
        catalog.character = Ok(CharacterStart {
            cash: 1200,
            ..catalog.character.expect("a chär")
        });
        catalog.dudes[0].1.govt = Some(GovtId(140));
        catalog.govts = vec![nova_sim::GovtRecord {
            id: GovtId(140),
            flags: 0,
            flags2: 0,
            crime_tol: 0,
            initial_rec: 0,
            penalties: nova_sim::Penalties {
                smuggle: 5,
                ..nova_sim::Penalties::default()
            },
            max_odds: 100,
            classes: [-1; 4],
            allies: [-1; 4],
            enemies: [-1; 4],
            comm_name: String::new(),
            scan_mask: 0x0004,
            scan_fine: fine,
            news_pic: -1,
            require: 0,
            inh_jam: [0; 4],
        }];
        catalog.missions = vec![MissionRecord {
            cargo_type: 0,
            cargo_qty: 4,
            pickup_mode: 0,
            scan_mask: 0x0004,
            flags,
            ..errand(130)
        }];
        let pilot = Pilot::new(&catalog, "Ada").expect("a pilot");
        let mut save: serde_json::Value =
            serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
        save["ship_name"] = serde_json::json!("Kestrel");
        save["missions"] = serde_json::json!([{
            "slot": 0,
            "mission": 130,
            "destination": null,
            "return": null,
            "cargo_type": 0,
            "cargo_tons": 4,
            "pay": 0,
            "days_left": null,
            "deadline": null,
            "reached": false,
            "goals_met": true,
            "aboard": true,
            "failed": null,
            "ships": {"system": null, "left": 0, "arrived": 0, "appear": -1, "name": "", "subtitle": "", "destroyed": 0, "boarded": 0, "disabled": 0, "chased_off": 0, "uniform_type": null, "ship_done": false},
            "aux": {"countdown": 0, "arrived": 0, "left": 0},
        }]);
        let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
        let mut draws = placed(750, 750, 0).to_vec();
        draws.push(0);
        let (_, chance) = scripted(&draws);
        FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Inspector::default()))
    }

    /// Inspects the player until its flag is set; then it has looked the
    /// player over, and goes on to its goal.
    #[derive(Debug)]
    struct InspectingUntil(Rc<std::cell::Cell<bool>>, Goal);

    impl Behaviour for InspectingUntil {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            if self.0.get() {
                self.1
            } else {
                Goal::Inspect(ShipRef::Player)
            }
        }
    }

    #[test]
    fn a_scan_in_the_warps_lead_in_is_said_under_the_streak() {
        let done = Rc::new(std::cell::Cell::new(false));
        let mut view = smuggling(0, 500)
            .with_behaviour(Rc::new(InspectingUntil(Rc::clone(&done), Goal::Idle)));
        plot(&mut view, 131);
        ready_to_jump(&mut view);
        view.input(&key(JUMP, true));
        assert!(view.jump_effect().is_some(), "streaking on J");
        assert_eq!(view.pilot().expect("flying").cash(), 1200, "not yet");
        done.set(true);
        view.tick(TICK);
        assert_eq!(view.pilot().expect("flying").cash(), 700, "convicted");
        assert_eq!(
            view.message(),
            Some(
                "Ship 129:  Kestrel, your attempt to smuggle illegal cargo through this \
                 system has been detected. You have been fined 500 credits."
            ),
            "said under the streak"
        );
        assert!(view.jump_effect().is_some());
    }

    #[test]
    fn a_scan_that_fails_a_mission_says_so_on_the_message_line() {
        let mut view = smuggling(0x0020, 500);
        for _ in 0..3 {
            view.tick(TICK);
        }
        let pilot = view.pilot().expect("flying");
        assert_eq!(pilot.missions().iter().count(), 0, "failed at once");
        assert_lasts(&mut view, MISSION_SCANNED, 400);
        assert_eq!(
            MISSION_SCANNED,
            "Your ship has been scanned - mission failed."
        );
    }

    #[test]
    fn a_scan_that_finds_smuggling_names_the_scanner_and_the_ship_and_the_fine() {
        let mut view = smuggling(0, 500);
        assert_eq!(view.pilot().expect("flying").cash(), 1200);
        for _ in 0..3 {
            view.tick(TICK);
        }
        assert_eq!(view.pilot().expect("flying").cash(), 700);
        assert_eq!(
            view.message(),
            Some(
                "Ship 129:  Kestrel, your attempt to smuggle illegal cargo through this \
                 system has been detected. You have been fined 500 credits."
            )
        );
    }

    /// A contraband notice names the scanner by its person's name when it
    /// is one (`_ScanPlayer` @0x7ebd1-0x7ec0c), else its ship class, and
    /// the outfit or `jünk` by the session's record; it lasts 400 ticks.
    #[test]
    fn a_contraband_notice_reads_its_names_from_the_session() {
        let view = FlightView::new(FakeCatalog {
            outfits: vec![OutfitRecord {
                lc_name: "blaster".to_owned(),
                lc_plural: "blasters".to_owned(),
                ..multi_jumping().outfits[0].clone()
            }],
            junk: vec![JunkRecord {
                id: nova_sim::JunkId(150),
                name: "Bio-weapons".to_owned(),
                abbrev: "Bio".to_owned(),
                base_price: 1000,
                sold_at: Vec::new(),
                bought_at: Vec::new(),
                buy_on: nova_sim::Test::default(),
                sell_on: nova_sim::Test::default(),
                flags: 0,
                scan_mask: 0x0004,
                lc_name: "bio-weapons".to_owned(),
            }],
            ..catalog()
        });
        let session = view.session().expect("flying");
        let ship = session.pilot().ship_name().unwrap_or_default();
        let class = session.ship_name(ShipId(128)).unwrap_or_default();
        let outfit = |person: Option<&str>| SimMessage::IllegalOutfit {
            scanner: ShipId(128),
            person: person.map(str::to_owned),
            outfit: OutfitId(275),
            one: true,
            fine: 500,
        };
        assert_eq!(
            sim_message_text(&outfit(Some("Ace")), session),
            Some(illegal_outfit_message(
                "Ace", ship, true, "blaster", "blasters", 500
            ))
        );
        assert_eq!(
            sim_message_text(&outfit(None), session),
            Some(illegal_outfit_message(
                class, ship, true, "blaster", "blasters", 500
            ))
        );
        let junk = |id: i16| SimMessage::IllegalJunk {
            scanner: ShipId(128),
            person: Some("Ace".to_owned()),
            junk: nova_sim::JunkId(id),
            fine: 0,
        };
        assert_eq!(
            sim_message_text(&junk(150), session),
            Some(illegal_junk_message("Ace", ship, "bio-weapons", 0))
        );
        assert_eq!(
            sim_message_text(&junk(151), session),
            Some(illegal_junk_message("Ace", ship, "", 0)),
            "no record, no name"
        );
        assert_eq!(arrival_lead(&junk(150)), None);
        assert_eq!(arrival_lead(&outfit(None)), None);
    }

    /// The found mission's drawn ship name names the scanner when it has
    /// one (`_ScanPlayer` @0x7e7ce); otherwise its ship class does.
    #[test]
    fn the_smuggling_notice_names_the_scanner_by_the_found_missions_ship_name() {
        let view = flight();
        let session = view.session().expect("flying");
        let ship = session.pilot().ship_name().unwrap_or_default();
        let text = |ship_name: Option<&str>| {
            sim_message_text(
                &SimMessage::Smuggling {
                    scanner: ShipId(128),
                    ship_name: ship_name.map(str::to_owned),
                    fine: 500,
                },
                session,
            )
        };
        assert_eq!(
            text(Some("Rover")),
            Some(smuggling_message("Rover", ship, 500))
        );
        assert_eq!(
            text(None),
            Some(smuggling_message(
                session.ship_name(ShipId(128)).unwrap_or_default(),
                ship,
                500
            ))
        );
    }

    #[test]
    fn mission_info_and_aborts_go_through_the_session() {
        let mut errands = catalog();
        errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        errands.missions = vec![errand(130)];
        errands.mission_texts = vec![MissionTexts {
            quick_brief: Some("Quick.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        }];
        let mut view = FlightView::new(errands);
        assert_eq!(view.mission_info().rows, []);
        assert_eq!(view.abort_mission(0), Err(AbortError::NotActive));
        land_now(&mut view);
        view.settle_script();
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        let slots: Vec<_> = view
            .mission_info()
            .rows
            .iter()
            .map(|row| row.slot)
            .collect();
        assert_eq!(slots, [0]);
        assert_eq!(view.show_info_row(0).as_deref(), Some("Quick."));
        assert_eq!(view.show_info_row(1), None, "a free slot");
        assert_eq!(view.abort_mission(0), Ok(()));
        assert_eq!(view.mission_info().rows, []);
        let mut broken = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert_eq!(broken.mission_info().rows, []);
        assert_eq!(broken.show_info_row(0), None);
        assert_eq!(broken.abort_mission(0), Err(AbortError::NotActive));
    }

    #[test]
    fn the_bars_offers_go_through_the_session_on_the_flights_chance() {
        let mut errands = catalog();
        errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        errands.missions = vec![MissionRecord {
            avail_loc: 1,
            ..errand(141)
        }];
        let (script, chance) = scripted(&[]);
        let mut view = FlightView::new(errands).with_chance(chance);
        assert_eq!(view.next_offer(1), None, "in flight");
        land_now(&mut view);
        view.settle_script();
        script.borrow_mut().asked.clear();
        script.borrow_mut().draws.push_back(7);
        view.enter_offer_venue(1);
        let Some(VenueOffer::Prompt(prompt)) = view.next_offer(1) else {
            panic!("an offer");
        };
        assert_eq!(prompt.mission, MissionId(141));
        assert_eq!(view.offer_gap(), nova_sim::mission::screens::mac_ticks(37));
        assert_eq!(script.borrow().asked, [30]);
        let mut broken = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        broken.enter_offer_venue(1);
        assert_eq!(broken.next_offer(1), None);
    }

    /// `pilot`, saved and read back with `escorts`, each a ship, whether
    /// it is carried and its wage, each full.
    fn with_paid_fleet(pilot: &Pilot, escorts: &[(i16, bool, Option<i64>)]) -> Pilot {
        let mut save: serde_json::Value =
            serde_json::from_str(&nova_sim::save::encode(pilot)).expect("JSON");
        let gauge = |max: f32| serde_json::json!({"now": max, "max": max});
        save["escorts"] = escorts
            .iter()
            .map(|&(ship, carried, wage)| {
                serde_json::json!({
                    "ship": ship,
                    "reserves": {"shield": gauge(40.0), "armor": gauge(60.0), "fuel": gauge(250.0)},
                    "order": null,
                    "carried": carried,
                    "wage": wage,
                    "person": null
                })
            })
            .collect();
        nova_sim::save::decode(&save.to_string()).expect("a pilot")
    }

    /// [`boardable`] with ship 130 of `Cost` 10,000 (a wage of 100), and
    /// a pilot holding `cash` whose fleet is one hired ship 130, saved at
    /// stellar 128 and landed there after its flight's first tick when
    /// `docked`, the landing not yet taken; else its flight, its first
    /// tick done.
    fn paying(cash: i64, docked: bool) -> View {
        let mut catalog = boardable();
        for record in &mut catalog.ships {
            if record.id == ShipId(130) {
                record.cost = 10_000;
            }
        }
        let mut pilot = with_paid_fleet(
            &Pilot::new(&catalog, "Payer").expect("starts"),
            &[(130, false, Some(100))],
        );
        pilot.set_cash(cash);
        if docked {
            let text =
                nova_sim::save::encode(&pilot).replace("\"stellar\": null", "\"stellar\": 128");
            pilot = nova_sim::save::decode(&text).expect("a pilot");
        }
        let (_, chance) = scripted(&placed(750, 650, 0));
        let mut view = FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        if docked {
            land_now(&mut view);
        }
        view
    }

    #[test]
    fn an_escort_left_unpaid_by_a_jump_defects_and_the_flight_says_so() {
        let mut view = paying(50, false);
        assert_eq!(view.pilot().expect("flying").escorts().len(), 1);
        jump_to_alpha(&mut view);
        let said = format!(
            "{}  {DEFECTED_ONE}",
            "Jumping into the Alpha Centauri system on June 24, 1177 NC."
        );
        assert_eq!(view.message(), Some(said.as_str()), "after the arrival");
        assert_eq!(view.pilot().expect("flying").escorts(), []);
        let mut paid = paying(100, false);
        jump_to_alpha(&mut paid);
        assert_eq!(
            paid.message(),
            Some("Jumping into the Alpha Centauri system on June 24, 1177 NC."),
            "the arrival alone"
        );
        assert_eq!(paid.pilot().map(Pilot::cash), Some(0));
    }

    /// The greeting from Earth on the take-off of a docked [`paying`]
    /// flight, its lead the one the fixture's scripted draws leave.
    const PAYING_GREETING: &str = "Departing Earth on June 24, 1177 NC.";

    #[test]
    fn by_the_engine_an_escort_left_unpaid_at_take_off_defects_and_the_flight_says_so() {
        let mut view = paying(50, true);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        assert_eq!(view.take_off(), Some(StellarId(128)));
        let said = format!("{PAYING_GREETING}  {DEFECTED_ONE}");
        assert_eq!(view.message(), Some(said.as_str()), "after the greeting");
        assert_eq!(view.pilot().expect("flying").escorts(), []);
        let mut kept = paying(50, true)
            .with_rules(Rulebook::default().with_override(RuleKey::TakeOffPay, RuleSource::Bible));
        kept.take_off().expect("took off");
        assert_eq!(kept.message(), Some(PAYING_GREETING), "the greeting alone");
        assert_eq!(kept.pilot().expect("flying").escorts().len(), 1);
    }

    #[test]
    fn fighters_abandoned_and_escorts_defected_on_one_arrival_are_said_together() {
        let mut catalog = boardable();
        for record in &mut catalog.ships {
            if record.id == ShipId(130) {
                record.escort_type = 0;
                record.fields.fuel = 0;
            }
        }
        let pilot = with_paid_fleet(
            &Pilot::new(&catalog, "Carrier").expect("starts"),
            &[
                (130, true, None),
                (130, true, None),
                (129, false, Some(1500)),
            ],
        );
        let (_, chance) = scripted(&placed(750, 650, 0));
        let mut view = FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        jump_to_alpha(&mut view);
        assert_eq!(
            view.message(),
            Some(
                "Jumping into the Alpha Centauri system on June 24, 1177 NC.  (Two fighters abandoned)  Due to lack of pay, one of your escorts has defected."
            )
        );
        assert_eq!(view.pilot().expect("flying").escorts(), []);
    }

    /// Terms of a fee of 7 and a wage of 3 for every ship.
    #[derive(Debug)]
    struct Sevens;

    impl HireTerms for Sevens {
        fn fee(&self, _ship: &ShipRecord, _site: &LandingSite, _mult: f32) -> i64 {
            7
        }

        fn charge(&self, _ship: &ShipRecord, _site: &LandingSite, _mult: f32, _cash: i64) -> i64 {
            7
        }

        fn wage(&self, _ship: &ShipRecord) -> i64 {
            3
        }
    }

    /// Control bits where nothing holds.
    #[derive(Debug)]
    struct Nothing;

    impl ControlBits for Nothing {
        fn allows(&self, _test: &nova_sim::TestExpr, _pilot: &dyn nova_sim::PilotFacts) -> bool {
            false
        }
    }

    #[test]
    fn the_hire_ports_reach_the_session() {
        let always: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always::default()));
        let mut view = FlightView::new(hiring())
            .with_chance(SharedChance::new(always))
            .with_hire_terms(Rc::new(Sevens))
            .with_control_bits(Rc::new(Nothing));
        land_now(&mut view);
        let list = view.escorts_for_hire().expect("a bar");
        let row = list.row(ShipId(129)).expect("listed: no Flags3 hides it");
        assert_eq!((row.fee, row.wage), (7, 3));
        assert_eq!(row.hire, Err(HireRefusal::NotForHire));
    }

    // Persons.

    /// [`trafficked`] with no traffic of its own but a Person slot at
    /// 100 %, "Ace" (`përs` 600, subtitle "Top Gun"), flying ship 129 and
    /// saying `STR#` 7101 #1, "<OSN>: Prepare to die, <PN>!", now and
    /// then.
    fn peopled() -> FakeCatalog {
        let mut catalog = trafficked(&[130], 0, 129, 3);
        catalog.traffic[0].1.persons[0] = (Some(nova_sim::PersonId(600)), 100);
        catalog.persons = vec![nova_sim::PersonRecord {
            id: nova_sim::PersonId(600),
            name: "Ace".to_owned(),
            link_syst: -2,
            govt: None,
            ai_type: 3,
            aggress: 2,
            coward: 0,
            ship: Some(ShipId(129)),
            weapons: Vec::new(),
            credits: 0,
            shield_mod: 0,
            hail_pict: None,
            comm_quote: -1,
            hail_quote: 1,
            link_mission: None,
            flags: 0,
            active_on: nova_sim::Test::default(),
            subtitle: "Top Gun".to_owned(),
            flags2: 0,
            grant_class: 0,
            grant_count: 0,
            grant_prob: 0,
            mission_ship: None,
        }];
        catalog.strings = vec![(7101, vec!["<OSN>: Prepare to die, <PN>!".to_owned()])];
        catalog
    }

    /// The draws that list Ace and place it 100 above the player, then
    /// the first quote draw landing on 0.
    const ACE: [u32; 5] = [99, 750, 650, 0, 0];

    #[test]
    fn a_person_targeted_shows_its_name_and_subtitle_on_the_panel() {
        let (_, chance) = scripted(&ACE);
        let mut view = FlightView::new(peopled())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        tap(&mut view, Key::Tab);
        let texts: Vec<String> = drawn(&view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|text| text == "Ace"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "Top Gun"), "{texts:?}");
    }

    /// A flight from Sol after accepting mission 140, whose one special
    /// ship, a rescue held disabled, is set up on Earth under the player
    /// at take-off, and which carries 10 tons of food (`STR#` 4001 #1),
    /// picked up by boarding it; the player's ship holds 10 tons. With
    /// `full`, mission 141, of 10 tons loaded at its accept, fills them
    /// first. The ship is targeted.
    fn rescuing(full: bool) -> View {
        let mut hunted = trafficked(&[130], 0, 129, 3);
        hunted.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        hunted.sol_navs = vec![Some((0, 0))];
        hunted.fields.holds = 10;
        for record in &mut hunted.ships {
            record.crew = 3;
        }
        hunted.strings = vec![(4001, vec!["Food".to_owned()])];
        let cargo = |id, pickup_mode| MissionRecord {
            cargo_type: 0,
            cargo_qty: 10,
            pickup_mode,
            ..errand(id)
        };
        hunted.missions = vec![
            MissionRecord {
                ships: MissionShip {
                    count: 1,
                    goal: 5,
                    syst: -1,
                    dude: Some(nova_sim::DudeId(128)),
                    start: -1,
                    ..MissionShip::NONE
                },
                ..cargo(140, 2)
            },
            cargo(141, 0),
        ];
        hunted.mission_texts = vec![
            MissionTexts::named(MissionId(140), "Rescue"),
            MissionTexts::named(MissionId(141), "Haul"),
        ];
        let strings = FakeCatalog {
            strings: hunted.strings.clone(),
            ..catalog()
        };
        let mut view = FlightView::new(hunted)
            .with_strings(Rc::new(strings))
            .with_behaviour(Rc::new(Still));
        land_now(&mut view);
        view.settle_script();
        assert_eq!(view.accept_mission(MissionId(140)), Ok(0));
        if full {
            assert_eq!(view.accept_mission(MissionId(141)), Ok(1));
        }
        view.settle_script();
        assert!(view.take_off().is_some());
        view.settle_script();
        view.tick(TICK);
        let _ = view.take_mission_texts();
        tap(&mut view, TARGET_KEY);
        let session = view.session().expect("flying");
        assert!(session.target().is_some_and(|npc| npc.mission.is_some()));
        view
    }

    #[test]
    fn b_on_a_pickup_ship_says_the_cargo_retrieved_for_250_frames() {
        let mut view = rescuing(false);
        tap(&mut view, BOARD_KEY);
        let said = "You retrieved  the Food from this ship.";
        assert_eq!(view.message(), Some(said));
        assert_eq!(view.take_boarding(), None, "no plunder");
        assert_eq!(view.take_mission_texts(), [], "no dialog");
        assert_lasts(&mut view, said, BOARD_LINE_TICKS);
    }

    /// The refused pickup's dialog (`_DoMissionCargoPickup` @0x982ff) is
    /// queued as the B press returns, so the router shows it over flight
    /// as it settles that input, and the comm line says #130.
    #[test]
    fn b_on_a_pickup_ship_without_room_shows_the_dialog_at_once_and_says_cant_board() {
        let mut view = rescuing(true);
        tap(&mut view, BOARD_KEY);
        assert_eq!(view.message(), Some(CANT_BOARD));
        let texts: Vec<(MissionTextKind, String)> = view
            .take_mission_texts()
            .into_iter()
            .map(|shown| match shown {
                MissionShown::Text(text) => (text.kind, text.text),
                MissionShown::Movie(movie) => panic!("an offer movie: {movie:?}"),
            })
            .collect();
        assert_eq!(
            texts,
            [(
                MissionTextKind::NoRoom,
                nova_sim::mission::cargo::NO_SPACE_TO_LOAD.to_owned()
            )]
        );
        let session = view.session().expect("flying");
        assert!(!session.npcs()[0].boarded);
    }

    #[test]
    fn a_mission_ship_targeted_shows_its_missions_name_and_subtitle_and_no_tag() {
        let mut hunted = trafficked(&[130], 0, 129, 3);
        hunted.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        hunted.missions = vec![MissionRecord {
            ships: MissionShip {
                count: 1,
                syst: -1,
                dude: Some(nova_sim::DudeId(128)),
                start: 0,
                name_id: Some(7000),
                subtitle: Some(7001),
                ..MissionShip::NONE
            },
            ..errand(140)
        }];
        hunted.mission_texts = vec![MissionTexts::named(MissionId(140), "Hunt")];
        let strings = FakeCatalog {
            strings: vec![
                (7000, vec!["Rover".to_owned()]),
                (7001, vec!["Bounty Hunter".to_owned()]),
            ],
            ..catalog()
        };
        let mut view = FlightView::new(hunted)
            .with_strings(Rc::new(strings))
            .with_behaviour(Rc::new(Still));
        land_now(&mut view);
        view.settle_script();
        assert_eq!(view.accept_mission(MissionId(140)), Ok(0));
        view.settle_script();
        assert!(view.take_off().is_some());
        view.settle_script();
        view.tick(TICK);
        let session = view.session().expect("flying");
        assert!(
            session.npcs().iter().any(|npc| npc.mission.is_some()),
            "the mission ship is in the system"
        );
        tap(&mut view, Key::Tab);
        let texts: Vec<String> = drawn(&view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|text| text == "Rover"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text == "Bounty Hunter"),
            "{texts:?}"
        );
        assert!(!texts.iter().any(|text| text == "Ship 129"), "{texts:?}");
        assert!(
            !texts
                .iter()
                .any(|text| text.trim() == "Sp" || text.trim() == "Df"),
            "no cheats-only tag: {texts:?}"
        );
    }

    #[test]
    fn a_hail_quote_shows_in_the_message_line_for_420_ticks() {
        let (_, chance) = scripted(&ACE);
        let mut view = FlightView::new(peopled())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        assert_eq!(
            view.message(),
            Some("Ace: Prepare to die, !"),
            "an unnamed pilot"
        );
        assert_lasts(&mut view, "Ace: Prepare to die, !", 420);
        assert_eq!(HAIL_QUOTE_TICKS, 420);
    }

    #[test]
    fn in_double_time_a_person_rolls_its_hail_quote_on_both_passes() {
        // `_HandleShip` (@0x44483) runs on both passes and its quote block
        // (@0x33f0f-0x33fba) tests no pass.
        let (script, chance) = scripted(&[99, 750, 650, 0, 1, 0]);
        let mut view = FlightView::new(peopled())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_double_time(true);
        view.tick(TICK);
        assert_eq!(view.message(), Some("Ace: Prepare to die, !"));
        let asked = &script.borrow().asked;
        assert_eq!(asked[asked.len() - 2..], [140, 140], "{asked:?}");
    }

    #[test]
    fn the_persons_rules_reach_the_session() {
        let rules = nova_sim::NovaPersons {
            odds: RuleSource::Bible,
            ..nova_sim::NovaPersons::default()
        };
        let view = flight().with_person_rules(Rc::new(rules));
        assert_eq!(
            format!("{:?}", view.session().expect("flying").person_rules()),
            format!("{rules:?}")
        );
    }

    /// Runs set expression `text` on `view`'s session.
    fn run_set(view: &mut View, text: &str) {
        let expr = nova_sim::SetExpr::parse(text).expect("parses");
        let session = view.session.as_mut().expect("flying");
        session.run_set(&expr, &mut NeverFires);
    }

    #[test]
    fn a_move_in_flight_lays_out_the_system_moved_to_once_settled() {
        let mut view = flight();
        view.catalog().systems_read.borrow_mut().clear();
        run_set(&mut view, "M131");
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
        view.settle_script();
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(131)]);
        assert_eq!(view.course_map().current(), Some(SystemId(131)));
        assert_eq!(view.shown_position(), Point::new(0.0, 0.0), "on Proxima");
        assert_eq!(view.message(), None);
        view.settle_script();
        assert_eq!(
            *view.catalog().systems_read.borrow(),
            [SystemId(131)],
            "nothing more to settle"
        );
    }

    #[test]
    fn an_h_from_a_set_expression_redraws_the_new_hull_once_settled() {
        let mut view = FlightView::new(shipbuying());
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
        run_set(&mut view, "H129");
        assert_eq!(view.session().map(Session::ship), Ok(ShipId(129)));
        view.settle_script();
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)]
        );
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2001, 2201], "ship 129's sheet, and its lights");
        assert_eq!(view.frame(), Some(0), "of 72 rotations");
    }

    #[test]
    fn an_outfits_on_purchase_h_redraws_the_new_hull() {
        let mut catalog = shipbuying();
        catalog.outfits[0].on_purchase = nova_sim::Script::parse("H129");
        let mut view = FlightView::new(catalog);
        land_now(&mut view);
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(view.session().map(Session::ship), Ok(ShipId(129)));
        view.settle_script();
        view.settle_script();
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)],
            "once"
        );
        view.take_off().expect("took off");
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2001, 2201], "the new hull");
    }

    #[test]
    fn settling_reads_no_sheet_when_the_class_is_unchanged() {
        let mut view = FlightView::new(shipbuying());
        view.settle_script();
        ticks(&mut view, 3);
        run_set(&mut view, "b1");
        view.settle_script();
        view.tick(TICK);
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
    }

    #[test]
    fn a_move_within_the_system_lays_it_out_afresh() {
        let mut view = flight();
        view.catalog().systems_read.borrow_mut().clear();
        run_set(&mut view, "N130");
        view.settle_script();
        assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(130)]);
    }

    #[test]
    fn a_move_while_landed_is_laid_out_once_the_ship_takes_off() {
        let mut view = landed_view();
        run_set(&mut view, "M131");
        view.settle_script();
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
        assert_eq!(*view.catalog().systems_read.borrow(), []);
        assert_eq!(view.take_off(), Some(StellarId(128)));
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(view.shown_position(), Point::new(0.0, 0.0));
        assert!(
            view.course_map()
                .explored()
                .is_some_and(|explored| explored.contains(&SystemId(131))),
            "the take-off explores"
        );
    }

    /// `view` with `STR#` 25048 holding one message.
    fn told(view: View) -> View {
        let strings = FakeCatalog {
            strings: vec![(25048, vec!["Off you go, <PSN>.".to_owned()])],
            ..catalog()
        };
        view.with_strings(Rc::new(strings))
    }

    #[test]
    fn a_q_in_flight_shows_its_message() {
        let mut view = told(flight());
        run_set(&mut view, "Q25048");
        view.settle_script();
        assert!(!view.leave_due());
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::ScriptMessage)]);
        assert_lasts(&mut view, "Off you go, <PSN>.", SCRIPT_MESSAGE_TICKS);
    }

    #[test]
    fn a_landed_q_waits_for_the_take_off_which_shows_its_message() {
        let mut view = told(landed_view());
        run_set(&mut view, "M131 Q25048");
        view.settle_script();
        let session = view.session().expect("flying");
        assert_eq!(session.landed(), Some(StellarId(128)), "held");
        assert!(view.leave_due());
        assert_eq!(view.message(), None, "nothing yet");
        assert_eq!(view.take_off(), Some(StellarId(128)));
        assert!(!view.leave_due());
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(view.shown_position(), Point::new(0.0, 0.0));
        assert_lasts(&mut view, "Off you go, <PSN>.", 500);
    }

    #[test]
    fn a_due_leave_takes_off_as_the_ports_leave_does() {
        let mut view = told(landed_view());
        assert_eq!(view.leave_if_due(), None, "nothing due");
        assert_eq!(view.session().expect("ok").landed(), Some(StellarId(128)));
        run_set(&mut view, "M131 Q25048");
        view.settle_script();
        assert_eq!(view.leave_if_due(), Some(StellarId(128)));
        assert_eq!(view.session().expect("ok").landed(), None, "in flight");
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_lasts(&mut view, "Off you go, <PSN>.", 500);
        assert_eq!(view.leave_if_due(), None, "once");
    }

    #[test]
    fn a_landed_q_shows_the_take_offs_pay_after_its_message() {
        let mut view = told(paying(50, true));
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        run_set(&mut view, "Q25048");
        view.settle_script();
        assert_eq!(view.take_off(), Some(StellarId(128)));
        let said = format!("Off you go, <PSN>.  {DEFECTED_ONE}");
        assert_lasts(&mut view, &said, 500);
    }

    /// The greeting from Earth on the day after the fixture's start, its
    /// lead the first, as the flight's chance rolls.
    const EARTH_GREETING: &str = "Launching from Earth on June 24, 1177 NC.";

    #[test]
    fn take_off_says_the_greeting_for_240() {
        let mut view = landed_view();
        assert_eq!(view.take_off(), Some(StellarId(128)));
        assert_eq!(message(&view), Some(message_drawn(EARTH_GREETING)));
        assert_lasts(&mut view, EARTH_GREETING, 240);
        assert_eq!(
            take_off_greeting(2, "Earth", "June 24, 1177 NC"),
            "Taking off from Earth on June 24, 1177 NC."
        );
        let leads: Vec<String> = (0..5)
            .map(|lead| take_off_greeting(lead, "X", "D"))
            .collect();
        assert_eq!(
            leads,
            [
                "Launching from X on D.",
                "Blasting off from X on D.",
                "Taking off from X on D.",
                "Leaving X on D.",
                "Departing X on D."
            ]
        );
    }

    #[test]
    fn the_greeting_leads_the_pay_notes() {
        let mut view = paying(50, true);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        view.take_off().expect("takes off");
        let said = format!("{PAYING_GREETING}  {DEFECTED_ONE}");
        assert_lasts(&mut view, &said, 240);
    }

    #[test]
    fn a_q_message_takes_the_greetings_place() {
        let mut view = told(landed_view());
        run_set(&mut view, "Q25048");
        view.settle_script();
        assert_eq!(view.leave_if_due(), Some(StellarId(128)));
        assert_eq!(view.message(), Some("Off you go, <PSN>."));
        let rules = Rulebook::default().with_override(RuleKey::BlankLeave, RuleSource::Bible);
        let mut view = told(landed_view()).with_rules(rules);
        run_set(&mut view, "Q25049");
        view.settle_script();
        assert_eq!(view.leave_if_due(), Some(StellarId(128)));
        assert_eq!(view.message(), Some(EARTH_GREETING), "an empty one greets");
    }

    #[test]
    fn a_new_pilots_take_off_says_nothing() {
        let mut view = new_pilot(landed_view());
        assert_eq!(view.take_off(), Some(StellarId(128)));
        assert_eq!(view.message(), None);
    }

    #[test]
    fn a_take_off_with_no_q_shows_no_leave_message() {
        let mut view = told(landed_view());
        assert!(!view.leave_due());
        assert_eq!(view.take_off(), Some(StellarId(128)));
        assert_eq!(view.message(), Some(EARTH_GREETING), "the greeting alone");
    }

    #[test]
    fn fighters_abandoned_in_a_move_are_told_after_a_qs_message() {
        let rules = Rulebook::default().with_override(RuleKey::MoveArrival, RuleSource::Bible);
        let mut view = told(fighters_out()).with_rules(rules);
        run_set(&mut view, "Q25048 M131");
        view.settle_script();
        assert_eq!(
            view.message(),
            Some("Off you go, <PSN>.  (Two fighters abandoned)")
        );
        assert_eq!(view.pilot().expect("flying").escorts(), []);
        view.message = None;
        view.settle_script();
        assert_eq!(view.message(), None, "nothing more to tell");
    }

    // Session events.

    /// The ships whose sheets `view` has asked for.
    fn sheets(view: &View) -> Vec<ShipId> {
        view.catalog().sheets_asked.borrow().clone()
    }

    fn systems_read(view: &View) -> Vec<SystemId> {
        view.catalog().systems_read.borrow().clone()
    }

    /// Settles `view`'s session by itself, not through the view.
    fn settle_session(view: &mut View) {
        let session = view.session.as_mut().expect("flying");
        let _ = session.settle_script(&view.catalog, &mut NeverFires);
    }

    #[test]
    fn the_hull_redraws_from_the_event_on_the_next_frame() {
        let mut view = FlightView::new(shipbuying());
        run_set(&mut view, "H129");
        settle_session(&mut view);
        assert_eq!(sheets(&view), [ShipId(128)], "not before the frame");
        view.tick(TICK);
        assert_eq!(
            sheets(&view),
            [ShipId(128), ShipId(129)],
            "a set expression"
        );
        view.tick(TICK);
        assert_eq!(sheets(&view), [ShipId(128), ShipId(129)], "once");

        let mut view = captured();
        let asked = sheets(&view).len();
        let session = view.session.as_mut().expect("flying");
        assert_eq!(
            session.assign(Assignment::MyShip, &mut NeverFires),
            Some(Assigned::MyShip)
        );
        assert_eq!(view.sheet_ship, Some(ShipId(128)), "not before the frame");
        view.tick(TICK);
        assert_eq!(view.sheet_ship, Some(ShipId(129)), "a capture");
        let player = sheets(&view)[asked..]
            .iter()
            .filter(|&&ship| ship == ShipId(129))
            .count();
        assert_eq!(player, 1, "read once, beside the old ship's as an escort");

        let mut view = FlightView::new(shipbuying());
        land_now(&mut view);
        let session = view.session.as_mut().expect("flying");
        session
            .buy_ship(ShipId(129), "Kestrel", &mut NeverFires)
            .expect("bought");
        view.tick(TICK);
        assert_eq!(sheets(&view), [ShipId(128), ShipId(129)], "a purchase");
    }

    /// The background of the status bar `view` shows.
    fn bar_shown(view: &View) -> Option<i16> {
        view.status_bar().ok().map(|bar| bar.layout.status_bkgnd)
    }

    fn bars_read(view: &View) -> Vec<i16> {
        view.catalog().bars_asked.borrow().clone()
    }

    #[test]
    fn the_status_bar_follows_the_ship_type_on_the_next_frame() {
        let mut view = FlightView::new(governed(shipbuying(), 129, 129));
        run_set(&mut view, "H129");
        settle_session(&mut view);
        assert_eq!(bar_shown(&view), Some(700), "not before the frame");
        view.tick(TICK);
        assert_eq!(bar_shown(&view), Some(705), "a set expression");
        view.tick(TICK);
        assert_eq!(bars_read(&view), [128, 129], "once");

        let mut view = captured_in(governed(boardable(), 129, 129));
        let session = view.session.as_mut().expect("flying");
        assert_eq!(
            session.assign(Assignment::MyShip, &mut NeverFires),
            Some(Assigned::MyShip)
        );
        assert_eq!(bar_shown(&view), Some(700), "not before the frame");
        view.tick(TICK);
        assert_eq!(bar_shown(&view), Some(705), "a capture");

        let mut view = FlightView::new(governed(shipbuying(), 129, 129));
        land_now(&mut view);
        let session = view.session.as_mut().expect("flying");
        session
            .buy_ship(ShipId(129), "Kestrel", &mut NeverFires)
            .expect("bought");
        view.tick(TICK);
        assert_eq!(bar_shown(&view), Some(705), "a purchase");
    }

    #[test]
    fn a_new_ship_types_missing_bar_keeps_the_layout_over_picture_128() {
        let mut view = FlightView::new(governed(shipbuying(), 129, 131));
        run_set(&mut view, "H129");
        view.tick(TICK);
        assert_eq!(bars_read(&view), [128, 131]);
        assert_eq!(
            view.status_bar(),
            Ok(&StatusBar {
                layout: layout(),
                background: Some(hud::Background {
                    id: 128,
                    width: 194.0,
                    height: 767.0,
                }),
            })
        );
    }

    #[test]
    fn the_system_is_laid_out_from_the_event() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.catalog().systems_read.borrow_mut().clear();
        jump_now(&mut view);
        arrive_now(&mut view);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)], "a jump");

        let mut view = FlightView::new(holed());
        view.catalog().systems_read.borrow_mut().clear();
        land_now(&mut view);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)], "a wormhole");

        let mut view = flight();
        view.catalog().systems_read.borrow_mut().clear();
        run_set(&mut view, "M131");
        settle_session(&mut view);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)], "a set expression");

        let mut view = landed_view();
        move_to(&mut view, 131, 140);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)], "a relocation");
    }

    #[test]
    fn several_moves_before_a_frame_lay_out_once_the_last_system() {
        let mut view = FlightView::new(FakeCatalog {
            onward: true,
            ..catalog()
        });
        view.catalog().systems_read.borrow_mut().clear();
        run_set(&mut view, "M131 M132");
        settle_session(&mut view);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(132)));
        assert_eq!(systems_read(&view), [SystemId(132)], "one layout");
    }

    /// A stellar moved to, then a system entered, before one frame: one
    /// layout, of the system entered. (Placing the ship as well after the
    /// layout would change nothing to see: its writes are the layout's.)
    #[test]
    fn a_system_entered_supersedes_a_stellar_moved_to_before_the_frame() {
        let mut view = landed_view();
        move_to(&mut view, 130, 129);
        let session = view.session.as_mut().expect("flying");
        assert_eq!(
            session.take_off(&view.catalog, &mut NeverFires),
            Some(StellarId(129))
        );
        run_set(&mut view, "M131");
        settle_session(&mut view);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)], "one layout");
        assert_eq!(view.course_map().current(), Some(SystemId(131)));
        assert_eq!(view.shown_position(), Point::new(0.0, 0.0), "at Proxima");
    }

    // The stellars shown.

    /// The IDs of the stellars `view`'s scene draws.
    fn scene_ids(view: &View) -> Vec<StellarId> {
        view.scene()
            .expect("laid out")
            .stellars()
            .iter()
            .map(|stellar| stellar.id)
            .collect()
    }

    /// The stellars `view`'s session shows.
    fn shown_ids(view: &View) -> Vec<StellarId> {
        view.session().expect("flying").stellars_shown().collect()
    }

    /// [`catalog`] with Sol listing Proxima (140) too: Sol, the lower and
    /// visible, owns it, though Alpha Centauri lists it.
    fn proxima_in_sol() -> FakeCatalog {
        let mut catalog = catalog();
        catalog
            .sites
            .push(site(140, (0.0, 0.0), StellarFlags::CAN_LAND));
        catalog
    }

    #[test]
    fn the_flight_scene_shows_only_the_sessions_stellars() {
        let view = FlightView::new(FakeCatalog {
            character: Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [Some(SystemId(131)), None, None, None],
                ..CharacterStart::default()
            }),
            ..proxima_in_sol()
        });
        assert_eq!(system_of(&view), SystemId(131));
        assert_eq!(shown_ids(&view), [], "Proxima is Sol's");
        assert_eq!(scene_ids(&view), [], "Alpha Centauri lists it, unshown");
    }

    #[test]
    fn a_rebuild_that_changes_the_stellars_shown_lays_out_the_scene_again() {
        let mut view = FlightView::with_pilot(proxima_in_sol(), docked_pilot("Ada"));
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        move_to(&mut view, 131, 140);
        view.tick(TICK);
        assert_eq!(shown_ids(&view), [StellarId(140)], "docked at it");
        assert_eq!(scene_ids(&view), [StellarId(140)]);
        assert_eq!(view.take_off(), Some(StellarId(140)));
        assert_eq!(shown_ids(&view), [], "in flight, Sol's again");
        assert_eq!(scene_ids(&view), shown_ids(&view));
    }

    // The galaxy overlay.

    /// Changes `view`'s galaxy by itself, not through the view.
    fn change_galaxy(view: &mut View, change: impl FnOnce(&mut nova_sim::story::GalaxyOverlay)) {
        let session = view.session.as_mut().expect("flying");
        assert!(session.change_galaxy(&view.catalog, change));
    }

    /// Hides system `id`.
    fn hiding(id: i16) -> impl FnOnce(&mut nova_sim::story::GalaxyOverlay) {
        move |overlay| {
            overlay.set_visible(SystemId(id), Some(false));
        }
    }

    /// Whether `view`'s course map draws system `id`.
    fn on_the_course_map(view: &View, id: i16) -> bool {
        view.course_map().model().system(SystemId(id)).is_some()
    }

    #[test]
    fn a_pilot_flown_with_a_galaxy_overlay_shows_it_on_the_course_map() {
        // 132 hidden by its `Visibility`, as the load evaluates it.
        let catalog = FakeCatalog {
            visibility: vec![nova_sim::catalog::SystemVisibility {
                id: SystemId(132),
                visibility: nova_sim::Test::parse("b50"),
            }],
            ..catalog()
        };
        let session = Session::start(&catalog).expect("starts");
        assert!(!session.galaxy().visible(SystemId(132)));
        let view = FlightView::with_pilot(catalog, session.pilot().clone());
        assert!(!on_the_course_map(&view, 132));
        assert!(on_the_course_map(&view, 131));
    }

    #[test]
    fn a_change_to_the_galaxy_is_drawn_on_the_next_frame_and_not_before() {
        let mut view = flight();
        change_galaxy(&mut view, hiding(132));
        assert!(on_the_course_map(&view, 132), "not before the frame");
        view.tick(TICK);
        assert!(!on_the_course_map(&view, 132));
    }

    #[test]
    fn the_course_map_shows_the_course_as_it_is_flown() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        let mut view = FlightView::with_pilot(catalog, session.pilot().clone());
        assert_eq!(view.course_map().route(), [SystemId(131)]);
        change_galaxy(&mut view, hiding(131));
        view.tick(TICK);
        assert_eq!(view.course_map().route(), [], "no system at its place");
        let session = view.session.as_ref().expect("flying");
        assert_eq!(session.course(), [SystemId(131)], "kept");
    }

    /// A flight of `reborn` Sol, bound for Alpha Centauri, Alpha Reborn
    /// explored.
    fn bound_for_alpha_with_reborn_explored() -> View {
        let catalog = FakeCatalog {
            reborn: true,
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        let explore = nova_sim::SetExpr::parse("X133").expect("parses");
        session.run_set(&explore, &mut NeverFires);
        let mut view = FlightView::with_pilot(catalog, session.pilot().clone());
        // A pilot flies with the nav system off: H cues the course.
        tap(&mut view, HYPERSPACE_MODE_KEY);
        view
    }

    #[test]
    fn the_nav_area_shows_the_system_standing_for_a_hidden_next_hop() {
        let mut view = bound_for_alpha_with_reborn_explored();
        change_galaxy(&mut view, hiding(131));
        view.tick(TICK);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Alpha Reborn"]);
    }

    #[test]
    fn the_nav_area_shows_no_destination_for_a_hop_with_no_system_visible_at_its_place() {
        let mut view = bound_for_alpha_with_reborn_explored();
        change_galaxy(&mut view, |overlay| {
            overlay.set_visible(SystemId(131), Some(false));
            overlay.set_visible(SystemId(133), Some(false));
        });
        view.tick(TICK);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_NO_DESTINATION]);
        let session = view.session().expect("flying");
        assert_eq!(session.course(), [SystemId(131)], "kept");
    }

    #[test]
    fn several_galaxy_changes_before_a_frame_rebuild_the_map_once() {
        let mut view = flight();
        change_galaxy(&mut view, hiding(132));
        change_galaxy(&mut view, |overlay| {
            overlay.set_visible(SystemId(132), None);
            overlay.set_visible(SystemId(131), Some(false));
        });
        let redrawn = view.catch_up();
        assert!(redrawn.galaxy);
        assert!(on_the_course_map(&view, 132), "the last galaxy");
        assert!(!on_the_course_map(&view, 131));
        assert!(!view.catch_up().galaxy, "once");
    }

    // Destroyed stellars (story-runtime phase 7).

    /// Stellar `id` with `Strength` 100 and `DeadTime` 5, so that `Y`
    /// destroys it, and `Flags2` `flags2`.
    fn life(id: i16, flags2: u16) -> nova_sim::StellarLife {
        nova_sim::StellarLife {
            id: StellarId(id),
            strength: 100,
            dead_time: 5,
            flags2,
            explosion: None,
            on_destroy: nova_sim::Script::default(),
            on_regen: nova_sim::Script::default(),
        }
    }

    /// A flight in Sol whose Earth (128, `DeadType` sheet 2128), Moon
    /// (129, no different graphic) and Proxima (140, `DeadType` sheet
    /// 2140) can be destroyed.
    fn destructible() -> View {
        FlightView::new(FakeCatalog {
            lives: vec![life(128, 0), life(129, 0), life(140, 0)],
            ..catalog()
        })
    }

    /// Whether stellar `id` is drawn with its `Type` sheet (1000 + `id`)
    /// and whether with its `DeadType` sheet (2000 + `id`), at any of the
    /// first 4 frames.
    fn sheets_drawn(view: &View, id: i16) -> (bool, bool) {
        let images: Vec<ImageKey> = sprites(&drawn(view))
            .into_iter()
            .map(|(image, _)| image)
            .collect();
        let sheet = |image_id| (0..4).any(|f| images.contains(&ImageKey::sprite(image_id, f)));
        (sheet(1000 + id), sheet(2000 + id))
    }

    const TYPE: (bool, bool) = (true, false);
    const DEAD_TYPE: (bool, bool) = (false, true);

    #[test]
    fn a_stellar_destroyed_with_y_is_drawn_with_its_dead_type_from_the_next_frame_and_regenerated_with_u()
     {
        let mut view = destructible();
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 128), TYPE);
        view.catalog().systems_read.borrow_mut().clear();

        run_set(&mut view, "Y128");
        settle_session(&mut view);
        assert_eq!(sheets_drawn(&view, 128), TYPE, "not before the frame");
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 128), DEAD_TYPE, "from the next frame");
        assert_eq!(systems_read(&view), [], "not laid out again");

        run_set(&mut view, "U128");
        settle_session(&mut view);
        assert_eq!(sheets_drawn(&view, 128), DEAD_TYPE, "not before the frame");
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 128), TYPE, "regenerated");
        assert_eq!(systems_read(&view), []);
    }

    #[test]
    fn y_and_u_before_one_frame_draw_type() {
        let mut view = destructible();
        view.tick(TICK);
        run_set(&mut view, "Y128");
        run_set(&mut view, "U128");
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 128), TYPE);
        let scene = view.scene().expect("a scene");
        assert!(scene.stellars().iter().all(|stellar| !stellar.destroyed));
    }

    #[test]
    fn y_on_a_stellar_of_another_system_changes_nothing_drawn() {
        let mut view = destructible();
        view.tick(TICK);
        view.catalog().systems_read.borrow_mut().clear();
        run_set(&mut view, "Y140");
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 128), TYPE);
        let scene = view.scene().expect("a scene");
        assert!(scene.stellars().iter().all(|stellar| !stellar.destroyed));
        assert_eq!(systems_read(&view), []);
    }

    #[test]
    fn a_destroyed_stellar_with_no_dead_type_keeps_its_type() {
        let mut view = destructible();
        run_set(&mut view, "Y129");
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 129), TYPE);
        let scene = view.scene().expect("a scene");
        assert!(scene.stellars()[1].destroyed, "destroyed all the same");
    }

    #[test]
    fn a_system_entered_draws_its_destroyed_stellars_dead() {
        let mut view = destructible();
        run_set(&mut view, "Y140");
        run_set(&mut view, "M131");
        settle_session(&mut view);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(sheets_drawn(&view, 140), DEAD_TYPE);
    }

    #[test]
    fn a_system_entered_after_its_stellar_was_destroyed_draws_it_dead() {
        let mut view = destructible();
        run_set(&mut view, "Y140");
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
        run_set(&mut view, "M131");
        settle_session(&mut view);
        let redrawn = view.catch_up();
        assert!(redrawn.system, "laid out");
        assert!(!redrawn.galaxy, "by the system's reaction alone");
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(sheets_drawn(&view, 140), DEAD_TYPE);
    }

    #[test]
    fn a_new_pilot_with_a_stellar_that_starts_destroyed_draws_it_dead_from_the_first_frame() {
        let mut view = FlightView::new(FakeCatalog {
            lives: vec![life(128, 0x0040)],
            ..catalog()
        });
        assert_eq!(sheets_drawn(&view, 128), TYPE, "not begun yet");
        view.begin();
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 128), DEAD_TYPE);
    }

    #[test]
    fn a_pilot_flown_with_a_stellar_destroyed_draws_it_dead_at_once() {
        let catalog = FakeCatalog {
            lives: vec![life(128, 0)],
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        let destroy = nova_sim::SetExpr::parse("Y128").expect("parses");
        session.run_set(&destroy, &mut NeverFires);
        let view = FlightView::with_pilot(catalog, session.pilot().clone());
        assert_eq!(sheets_drawn(&view, 128), DEAD_TYPE, "before any frame");
    }

    // A `DeadType` with no sheet after a rebuild point (story-runtime
    // phase 9).

    /// Whether graphic type 0's sheet is drawn, at any of the first 4
    /// frames: Moon (129) is the only stellar that can draw it.
    fn type_zero_drawn(view: &View) -> bool {
        sprites(&drawn(view))
            .into_iter()
            .any(|(image, _)| (0..4).any(|f| image == ImageKey::sprite(TYPE_ZERO, f)))
    }

    /// Moon (129) is drawn with its `Type` sheet, or graphic type 0's.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Moon {
        Type,
        TypeZero,
    }

    fn moon(view: &View) -> Moon {
        match (sheets_drawn(view, 129), type_zero_drawn(view)) {
            (TYPE, false) => Moon::Type,
            ((false, false), true) => Moon::TypeZero,
            other => panic!("the Moon drawn as {other:?}"),
        }
    }

    /// [`destructible`] with `DeadTypeFallback` read as `reading`.
    fn reading(reading: RuleSource) -> View {
        destructible()
            .with_rules(Rulebook::default().with_override(RuleKey::DeadTypeFallback, reading))
    }

    /// The Moon after `Y129` and a frame; then after an in-flight `M130`
    /// and a frame; then `U129` and a frame; then `Y129` and a frame.
    fn moon_through_a_rebuild(mut view: View) -> [Moon; 4] {
        run_set(&mut view, "Y129");
        view.tick(TICK);
        let before = moon(&view);
        run_set(&mut view, "M130");
        settle_session(&mut view);
        view.tick(TICK);
        let rebuilt = moon(&view);
        run_set(&mut view, "U129");
        view.tick(TICK);
        let regenerated = moon(&view);
        run_set(&mut view, "Y129");
        view.tick(TICK);
        [before, rebuilt, regenerated, moon(&view)]
    }

    #[test]
    fn a_stellar_with_no_dead_type_draws_type_zero_from_the_next_rebuild_point_until_seen_whole() {
        assert_eq!(
            moon_through_a_rebuild(destructible()),
            [Moon::Type, Moon::TypeZero, Moon::Type, Moon::Type],
            "by default, the engine's"
        );
        assert_eq!(
            moon_through_a_rebuild(reading(RuleSource::Engine)),
            [Moon::Type, Moon::TypeZero, Moon::Type, Moon::Type]
        );
    }

    #[test]
    fn by_the_bible_a_stellar_with_no_dead_type_keeps_its_type_throughout() {
        assert_eq!(
            moon_through_a_rebuild(reading(RuleSource::Bible)),
            [Moon::Type; 4]
        );
    }

    #[test]
    fn a_take_off_that_only_rebuilds_draws_type_zero() {
        let mut view = FlightView::new(FakeCatalog {
            sites: vec![
                site(128, (6.0, -8.0), StellarFlags::CAN_LAND),
                site(129, (300.0, -200.0), StellarFlags::CAN_LAND),
            ],
            lives: vec![life(129, 0)],
            ..catalog()
        });
        land_now(&mut view);
        assert_eq!(
            view.session().expect("flying").landed(),
            Some(StellarId(128))
        );
        run_set(&mut view, "Y129");
        view.tick(TICK);
        assert_eq!(
            moon(&view),
            Moon::Type,
            "destroyed after the landing's rebuild"
        );
        view.catalog().systems_read.borrow_mut().clear();
        view.take_off();
        assert_eq!(moon(&view), Moon::TypeZero, "the take-off's rebuild");
        assert_eq!(systems_read(&view), [], "not laid out again");
    }

    /// A pilot loaded with the Moon destroyed, flown by `rules`.
    fn loaded(rules: Rulebook) -> View {
        let catalog = FakeCatalog {
            lives: vec![life(129, 0)],
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        let destroy = nova_sim::SetExpr::parse("Y129").expect("parses");
        session.run_set(&destroy, &mut NeverFires);
        FlightView::with_pilot(catalog, session.pilot().clone()).with_rules(rules)
    }

    #[test]
    fn a_pilot_loaded_with_a_stellar_destroyed_draws_it_as_its_rules_say_from_the_first_frame() {
        let view = loaded(Rulebook::default());
        assert!(
            view.session()
                .expect("flying")
                .dead_since_rebuild(StellarId(129))
        );
        assert_eq!(moon(&view), Moon::TypeZero, "a load is a rebuild point");
        let bible = Rulebook::default().with_override(RuleKey::DeadTypeFallback, RuleSource::Bible);
        let view = loaded(bible);
        assert_eq!(moon(&view), Moon::Type, "before any event");
        let view = loaded(bible).with_rules(Rulebook::default());
        assert_eq!(moon(&view), Moon::TypeZero, "and back");
    }

    #[test]
    fn a_system_entered_by_a_jump_with_a_stellar_destroyed_draws_type_zero_from_its_first_frame() {
        let mut view = destructible();
        run_set(&mut view, "Y129");
        jump_to_alpha(&mut view);
        plot(&mut view, 130);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        arrive_now(&mut view);
        assert_eq!(view.session().expect("flying").system(), SystemId(130));
        assert_eq!(moon(&view), Moon::TypeZero);
    }

    #[test]
    fn a_frame_with_the_map_open_still_redraws_from_the_events() {
        let mut view = flight();
        tap(&mut view, MAP);
        assert!(view.map_open());
        view.catalog().systems_read.borrow_mut().clear();
        run_set(&mut view, "M131");
        settle_session(&mut view);
        view.tick(TICK);
        assert!(view.map_open(), "still open");
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)]);

        let mut view = FlightView::new(shipbuying());
        tap(&mut view, MAP);
        run_set(&mut view, "H129");
        view.tick(TICK);
        assert!(view.map_open(), "still open");
        assert_eq!(sheets(&view), [ShipId(128), ShipId(129)]);
    }

    #[test]
    fn a_frame_during_the_streak_still_redraws_the_hull() {
        let mut view = FlightView::new(shipbuying());
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        run_set(&mut view, "H129");
        view.tick(TICK);
        assert!(view.jump_effect().is_some(), "still streaking");
        assert_eq!(view.session().map(Session::system), Ok(SystemId(130)));
        assert_eq!(sheets(&view), [ShipId(128), ShipId(129)]);
    }

    #[test]
    fn fighters_abandoned_are_told_only_with_the_layout_of_the_move_settled() {
        let rules = Rulebook::default().with_override(RuleKey::MoveArrival, RuleSource::Bible);
        let mut view = fighters_out().with_rules(rules);
        run_set(&mut view, "M131");
        settle_session(&mut view);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        view.settle_script();
        assert_eq!(view.message(), None, "no layout from this settling");
        let session = view.session.as_mut().expect("flying");
        assert_eq!(
            session.take_fighter_notes(),
            [FighterNote::Abandoned(2)],
            "left for the next arrival"
        );
    }

    #[test]
    fn a_desk_move_redraws_with_no_extra_call() {
        let mut view = landed_view();
        move_to(&mut view, 131, 140);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)]);
        assert_eq!(view.course_map().current(), Some(SystemId(131)));
        assert_eq!(view.shown_position(), Point::new(0.0, 0.0), "at Proxima");

        let mut view = landed_view();
        plot(&mut view, 131);
        move_to(&mut view, 130, 129);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
        assert_eq!(systems_read(&view), [], "the scene kept");
        assert_eq!(view.course_map().route(), [], "the course cleared");
        assert_eq!(
            view.shown_position(),
            Point::new(300.0, -200.0),
            "at the Moon"
        );
    }

    #[test]
    fn the_disable_rule_reaches_the_session_too() {
        let rule = Rc::new(Counting::default());
        let view = flight().with_disable_rule(rule.clone());
        let session = view.session().expect("flying");
        assert!(
            !session
                .disable_rule()
                .disabled(nova_sim::Gauge::full(10.0), &session.hull())
        );
        assert_eq!(rule.asked.get(), 1, "the session asks the rule given");
    }

    #[test]
    fn the_string_lists_reach_the_session() {
        let strings = FakeCatalog {
            strings: vec![(25040, vec!["Kestrel".to_owned()])],
            ..catalog()
        };
        let view = flight().with_strings(Rc::new(strings));
        assert_eq!(
            view.session().expect("flying").strings().string_list(25040),
            ["Kestrel"]
        );
    }

    // Boarding grants.

    /// Grants a person's `GrantCount` of the first outfit it may.
    #[derive(Debug)]
    struct Grants;

    impl BoardingRule for Grants {
        fn repels(&self, _booty: u16) -> bool {
            false
        }

        fn capture_odds(
            &self,
            _crew: &nova_sim::board::CaptureCrew,
            _chance: &mut dyn Chance,
        ) -> u8 {
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
            grant: &nova_sim::grant::PersonGrant,
            stock: &[nova_sim::grant::GrantStock],
            _free_mass: i64,
            _grant_max: nova_sim::RuleSource,
            _chance: &mut dyn Chance,
        ) -> Option<nova_sim::grant::Granted> {
            let first = nova_sim::grant::candidates(grant.class, stock)
                .first()
                .copied()?;
            Some(nova_sim::grant::Granted {
                outfit: first.outfit,
                count: grant.count,
            })
        }
    }

    /// [`peopled`] with Ace a derelict (government 140), so disabled,
    /// granting `count` spare parts (`oütf` 200, class 7), of no hail
    /// quote; its ship 129 has a crew of 3 and a board reach.
    fn granting(count: i16) -> FakeCatalog {
        let mut catalog = peopled();
        let ace = &mut catalog.persons[0];
        ace.govt = Some(GovtId(140));
        ace.hail_quote = -1;
        ace.grant_class = 7;
        ace.grant_prob = 100;
        ace.grant_count = count;
        catalog.govts = vec![nova_sim::GovtRecord {
            id: GovtId(140),
            flags: 0x0800,
            flags2: 0,
            crime_tol: 0,
            initial_rec: 0,
            penalties: nova_sim::Penalties::default(),
            max_odds: 100,
            classes: [-1; 4],
            allies: [-1; 4],
            enemies: [-1; 4],
            comm_name: String::new(),
            scan_mask: 0,
            scan_fine: 0,
            news_pic: -1,
            require: 0,
            inh_jam: [0; 4],
        }];
        for record in &mut catalog.ships {
            record.crew = 3;
        }
        catalog.hulls = vec![hull_of(129, &[])];
        catalog.outfits = vec![OutfitRecord {
            id: OutfitId(200),
            name: "Spare Part".to_owned(),
            short_name: "Spare Part".to_owned(),
            disp_weight: 0,
            mass: 0,
            tech_level: 1,
            max: 10,
            flags: 0,
            cost: 100,
            mods: [(0, 0); 4],
            contribute: 0,
            require: 0,
            require_govt: -1,
            buy_random: 100,
            availability: nova_sim::Test::default(),
            item_class: 7,
            lc_name: "spare part".to_owned(),
            lc_plural: "spare parts".to_owned(),
            scan_mask: 0,
            on_purchase: nova_sim::Script::default(),
            on_sell: nova_sim::Script::default(),
        }];
        catalog
    }

    /// [`granting`]'s flight, by `rule`, with Ace listed and placed on
    /// the player, targeted and boarded.
    fn boarded_ace(count: i16, rule: Rc<dyn BoardingRule>) -> View {
        let (_, chance) = scripted(&[99, 750, 750, 0]);
        let mut view = FlightView::new(granting(count))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_boarding_rule(rule);
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        tap(&mut view, BOARD_KEY);
        assert!(view.take_boarding().is_some(), "boarded");
        view
    }

    #[test]
    fn boarding_a_granting_person_says_what_it_retrieved_for_240_ticks() {
        let mut view = boarded_ace(2, Rc::new(Grants));
        let said = "You retrieved two spare parts from this ship.";
        assert_eq!(view.message(), Some(said));
        assert_eq!(view.pilot().expect("a pilot").owned(OutfitId(200)), 2);
        assert_lasts(&mut view, said, GRANT_TICKS);
        let view = boarded_ace(1, Rc::new(Grants));
        assert_eq!(
            view.message(),
            Some("You retrieved a spare part from this ship.")
        );
    }

    #[test]
    fn boarding_a_person_that_grants_nothing_says_nothing() {
        let view = boarded_ace(2, Rc::new(Sure));
        assert_eq!(view.message(), None);
    }

    // Persons' mission offers.

    /// [`peopled`] with Ace of `flags` linked to mission 130, offered
    /// from a ship, its offer text `offer`, refusable unless
    /// `unrefusable`.
    fn offering(flags: u16, offer: Option<&str>, unrefusable: bool) -> FakeCatalog {
        let mut catalog = peopled();
        catalog.persons[0].flags = flags;
        catalog.persons[0].link_mission = Some(130);
        catalog.missions = vec![nova_sim::catalog::MissionRecord {
            id: MissionId(130),
            avail_stel: -1,
            avail_loc: 2,
            avail_record: 0,
            avail_rating: 0,
            avail_random: 100,
            avail_ship_type: 0,
            avail_bits: nova_sim::Test::default(),
            require: 0,
            flags: if unrefusable { 0x0004 } else { 0 },
            flags2: 0,
            travel_stel: -1,
            return_stel: -1,
            cargo_type: -1,
            cargo_qty: 0,
            pickup_mode: -1,
            drop_off_mode: -1,
            scan_mask: 0,
            pay_val: 0,
            time_limit: 0,
            disp_weight: 0,
            on_accept: nova_sim::Script::default(),
            on_refuse: nova_sim::Script::default(),
            on_success: nova_sim::Script::default(),
            on_failure: nova_sim::Script::default(),
            on_abort: nova_sim::Script::default(),
            on_ship_done: nova_sim::Script::default(),
            can_abort: 1,
            comp_govt: None,
            comp_reward: 0,
            date_post_inc: 0,
            ships: nova_sim::catalog::MissionShip::NONE,
            aux: nova_sim::catalog::AuxShip::NONE,
        }];
        catalog.mission_texts = vec![nova_sim::catalog::MissionTexts {
            offer: offer.map(str::to_owned),
            ..nova_sim::catalog::MissionTexts::named(MissionId(130), "Help Ace")
        }];
        catalog
    }

    /// `catalog`'s flight with Ace listed and placed 100 above the
    /// player, targeted.
    fn targeting_ace(catalog: FakeCatalog) -> View {
        let (_, chance) = scripted(&ACE[..4]);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        view
    }

    #[test]
    fn y_on_a_person_offering_its_mission_reports_the_offer_once() {
        let mut view = targeting_ace(offering(0, Some("<OSN> needs help."), false));
        assert_eq!(view.take_offer(), None);
        view.input(&key(Key::Up, true));
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_hail(), None, "no comm dialog");
        let prompt = view.take_offer().expect("offered");
        assert_eq!(prompt.mission, MissionId(130));
        assert_eq!(prompt.text, "Ace needs help.");
        assert_eq!(view.take_offer(), None, "once");
        view.tick(TICK);
        assert_eq!(
            view.session().expect("flying").player().velocity,
            Vec2::ZERO,
            "the flight keys let go"
        );
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        assert_eq!(
            view.pilot().expect("a pilot").missions().missions(),
            [MissionId(130)]
        );
    }

    #[test]
    fn i_with_no_mission_says_you_have_no_active_missions() {
        let mut view = flight();
        let _ = view.take_sounds();
        tap(&mut view, MISSION_INFO_KEY);
        assert_eq!(view.take_mission_info_request(), None);
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::Refused)],
            "_beepSnd[3], snd 153"
        );
        assert_lasts(&mut view, NO_ACTIVE_MISSIONS, 240);
    }

    #[test]
    fn i_in_a_flight_that_never_started_says_no_active_missions_without_a_beep() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert!(view.session().is_err());
        let _ = view.take_sounds();
        tap(&mut view, MISSION_INFO_KEY);
        assert_eq!(view.take_mission_info_request(), None);
        assert_eq!(view.message(), Some(NO_ACTIVE_MISSIONS));
        assert_eq!(view.take_sounds(), [], "no session to beep");
    }

    #[test]
    fn i_with_an_active_mission_asks_for_the_info_screen_once() {
        let mut view = targeting_ace(offering(0, Some("Help?"), false));
        tap(&mut view, HAIL_KEY);
        view.take_offer().expect("offered");
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        view.input(&key(Key::Up, true));
        let _ = view.take_sounds();
        tap(&mut view, MISSION_INFO_KEY);
        assert_eq!(view.take_sounds(), [], "no beep from flight");
        let info = view.take_mission_info_request().expect("asked");
        assert_eq!(info, view.mission_info());
        assert_eq!(view.take_mission_info_request(), None, "once");
        assert_eq!(view.message(), None);
        view.tick(TICK);
        assert_eq!(
            view.session().expect("flying").player().velocity,
            Vec2::ZERO,
            "the flight keys let go"
        );
    }

    #[test]
    fn p_asks_for_the_player_info_once_letting_go_of_the_flight_keys() {
        let mut view = flight();
        assert!(!view.take_player_info_request());
        view.input(&key(Key::Up, true));
        tap(&mut view, PLAYER_INFO_KEY);
        assert!(view.take_player_info_request());
        assert!(!view.take_player_info_request(), "once");
        view.input(&Input::Key {
            key: PLAYER_INFO_KEY,
            pressed: true,
            repeat: true,
        });
        assert!(!view.take_player_info_request(), "not a repeat");
        view.tick(TICK);
        assert_eq!(
            view.session().expect("flying").player().velocity,
            Vec2::ZERO,
            "the flight keys let go"
        );
        assert_eq!(
            view.player_info().map(|info| info.credits),
            Some(view.pilot().expect("flying").cash())
        );
        assert!(view.take_sounds().is_empty(), "silent");
    }

    #[test]
    fn p_during_a_jump_beeps_and_asks_nothing() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        assert!(view.session().expect("flying").preparing_jump().is_some());
        let _ = view.take_sounds();
        tap(&mut view, PLAYER_INFO_KEY);
        assert!(!view.take_player_info_request());
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::Refused)],
            "turning to jump"
        );
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let _ = view.take_sounds();
        view.input(&Input::Key {
            key: PLAYER_INFO_KEY,
            pressed: true,
            repeat: true,
        });
        assert!(view.take_sounds().is_empty(), "not a repeat");
        tap(&mut view, PLAYER_INFO_KEY);
        assert!(!view.take_player_info_request());
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::Refused)],
            "streaking"
        );
    }

    #[test]
    fn p_while_the_player_breaks_up_beeps_and_asks_nothing() {
        let mut view = breaking_up();
        let _ = view.take_sounds();
        tap(&mut view, PLAYER_INFO_KEY);
        assert!(!view.take_player_info_request());
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::Refused)]);
    }

    #[test]
    fn p_once_the_player_has_exploded_or_ejected_does_nothing_and_says_nothing() {
        let mut holding = exploded();
        assert!(matches!(fate_of(&holding), PlayerFate::Holding { .. }));
        let mut pod = ejecting_view(ejecting_in(100, &[188, 187]), &[]);
        assert!(until_ejected(&mut pod).0);
        for (why, view) in [("holding", &mut holding), ("in the pod", &mut pod)] {
            let _ = view.take_sounds();
            tap(view, PLAYER_INFO_KEY);
            assert!(!view.take_player_info_request(), "{why}");
            assert_eq!(view.take_sounds(), [], "{why}");
        }
    }

    #[test]
    fn p_while_disabled_asks_for_the_player_info_in_silence() {
        let mut view = disabled_player();
        let _ = view.take_sounds();
        tap(&mut view, PLAYER_INFO_KEY);
        assert!(view.take_player_info_request());
        assert_eq!(view.take_sounds(), []);
    }

    #[test]
    fn l_while_disabled_selects_a_stellar_and_then_says_nothing_and_does_not_land() {
        let mut view = disabled_player();
        assert_eq!(nav_target(&view), None);
        tap(&mut view, LAND_KEY);
        let selected = nav_target(&view);
        assert!(selected.is_some(), "the nearest selected (@0x6a901)");
        assert_eq!(view.message(), None, "no clearance reply");
        tap(&mut view, LAND_KEY);
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.message(), None, "refused silently (@0x66af7)");
        assert_eq!(nav_target(&view), selected, "kept");
    }

    /// A flight under fire from an NPC whose every hit disables, stepped
    /// until the player's ship is disabled.
    fn disabled_player() -> View {
        let mut catalog = armed(1, &[]);
        catalog.hulls[1] = hull_of(129, &[BLASTER]);
        let mut view = fighting(catalog, &placed(750, 650, 180))
            .with_behaviour(Rc::new(Firing))
            .with_disable_rule(Rc::new(Disabling));
        for _ in 0..40 {
            if fate_of(&view) == PlayerFate::Disabled {
                return view;
            }
            view.tick(TICK);
        }
        panic!("the player's ship was never disabled");
    }

    #[test]
    fn opening_the_player_info_gives_the_screen_unless_in_a_jump() {
        let mut view = flight();
        assert_eq!(view.open_player_info(), view.player_info(), "flying");
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        assert!(view.session().expect("flying").preparing_jump().is_some());
        assert!(view.player_info().is_some(), "the getter reads it");
        assert_eq!(view.open_player_info(), None, "no opening (@0x5a02a)");
    }

    #[test]
    fn p_with_the_map_open_does_nothing() {
        let mut view = flight();
        tap(&mut view, MAP_KEY);
        tap(&mut view, PLAYER_INFO_KEY);
        assert!(!view.take_player_info_request());
        assert!(view.map_open());
        assert!(HELP.contains("P: player info"), "{HELP}");
    }

    #[test]
    fn the_help_line_names_mission_info() {
        assert!(HELP.contains("P: player info   I: mission info"), "{HELP}");
    }

    #[test]
    fn i_with_the_map_open_does_nothing() {
        let mut view = flight();
        tap(&mut view, MAP_KEY);
        assert!(view.map_open());
        tap(&mut view, MISSION_INFO_KEY);
        assert_eq!(view.message(), None);
        assert_eq!(view.take_mission_info_request(), None);
    }

    #[test]
    fn an_offer_keeps_the_message() {
        let mut view = targeting_ace(offering(0, Some("Help?"), false));
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, HAIL_KEY);
        view.take_offer().expect("offered");
        assert_eq!(view.message(), Some(MISSION_FAILED), "under the dialog");
    }

    #[test]
    fn a_refusal_of_the_offer_goes_through_the_session() {
        let mut view = targeting_ace(offering(0, Some("Help?"), false));
        tap(&mut view, HAIL_KEY);
        view.take_offer().expect("offered");
        assert_eq!(view.refuse_mission(MissionId(130)).map(|_| ()), Ok(()));
        assert_eq!(
            view.refuse_mission(MissionId(130)),
            Err(NotOffered),
            "answered"
        );
    }

    /// How a person's offer is answered over flight: accepted or refused
    /// in the dialog with a refuse button, or accepted in the one-button
    /// dialog.
    const FLIGHT_ANSWERS: [(&str, bool, bool); 3] = [
        ("an accept", false, false),
        ("a refusal", false, true),
        ("a one-button accept", true, false),
    ];

    /// [`offering`] Ace's refusable (or `unrefusable`) mission 130, with
    /// a `BriefText` and a `RefuseText`, whose `OnAccept` (or, `refuse`,
    /// `OnRefuse`) is `G200`, a tank: hailed and its offer taken, in
    /// flight with the nearest stellar, the Moon, selected, the HUD's
    /// cadence read as `cadence` says.
    fn offered_in_flight(cadence: RuleSource, unrefusable: bool, refuse: bool) -> View {
        let mut catalog = offering(0, Some("Help?"), unrefusable);
        catalog.outfits = vec![shipbuying().outfits[0].clone()];
        let script = nova_sim::Script::parse("G200");
        let mission = &mut catalog.missions[0];
        if refuse {
            mission.on_refuse = script;
        } else {
            mission.on_accept = script;
        }
        let texts = &mut catalog.mission_texts[0];
        texts.brief = Some("Go.".to_owned());
        texts.refuse = Some("No.".to_owned());
        let (_, chance) = scripted(&ACE[..4]);
        let rules = Rulebook::default().with_override(RuleKey::HudCadence, cadence);
        let mut view = FlightView::new(catalog)
            .with_rules(rules)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        tap(&mut view, NEAREST_STELLAR_KEY);
        tap(&mut view, TARGET_KEY);
        tap(&mut view, HAIL_KEY);
        view.take_offer().expect("offered");
        view
    }

    #[test]
    fn an_offer_answered_over_flight_shows_its_text_over_the_pre_answer_bar() {
        for (what, unrefusable, refuse) in FLIGHT_ANSWERS {
            let mut view = offered_in_flight(RuleSource::Engine, unrefusable, refuse);
            let before = view.reserves();
            let nav = view.nav_shown();
            if refuse {
                assert_eq!(
                    view.refuse_mission(MissionId(130)).map(|_| ()),
                    Ok(()),
                    "{what}"
                );
            } else {
                assert_eq!(view.accept_mission(MissionId(130)), Ok(0), "{what}");
            }
            assert_eq!(
                view.reserves().fuel.max,
                before.fuel.max + 100.0,
                "{what}: G200 ran"
            );
            // A selection the bar hasn't drawn: an `M` or `N` in the
            // answer can't stand for it, as its move lays the system out
            // afresh as it settles, which redraws the whole bar.
            view.select_stellar(StellarPick::Slot(0));
            assert_ne!(view.nav_now(), nav, "{what}: Earth selected");
            assert_eq!(view.take_mission_texts().len(), 1, "{what}: its text");
            assert_eq!(view.reserves_shown(), before, "{what}: under the text");
            assert_eq!(view.nav_shown(), nav, "{what}: the nav held too");
            view.close_mission_text();
            assert_eq!(
                view.reserves_shown(),
                view.reserves(),
                "{what}: live once it closed"
            );
            assert_eq!(view.nav_shown(), view.nav_now(), "{what}: nav live too");
            assert!(view.bar_steps.is_empty(), "{what}: let go");
        }
    }

    #[test]
    fn under_the_bible_cadence_flights_gauges_stay_live_under_an_offers_text() {
        for (what, unrefusable, refuse) in FLIGHT_ANSWERS {
            let mut view = offered_in_flight(RuleSource::Bible, unrefusable, refuse);
            if refuse {
                assert_eq!(
                    view.refuse_mission(MissionId(130)).map(|_| ()),
                    Ok(()),
                    "{what}"
                );
            } else {
                assert_eq!(view.accept_mission(MissionId(130)), Ok(0), "{what}");
            }
            assert_eq!(view.take_mission_texts().len(), 1, "{what}: its text");
            assert_eq!(view.reserves_shown(), view.reserves(), "{what}: live");
        }
    }

    #[test]
    fn b_on_a_person_offering_on_boarding_reports_the_offer_and_no_plunder() {
        let mut catalog = granting(2);
        let ace = &mut catalog.persons[0];
        ace.flags = 0x0200;
        ace.link_mission = Some(130);
        catalog.missions = offering(0, None, false).missions;
        catalog.mission_texts = offering(0, Some("Aboard."), false).mission_texts;
        let (_, chance) = scripted(&[99, 750, 750, 0]);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_boarding_rule(Rc::new(Grants));
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        tap(&mut view, BOARD_KEY);
        assert_eq!(view.take_boarding(), None, "no plunder dialog");
        assert_eq!(
            view.take_offer().map(|prompt| prompt.text),
            Some("Aboard.".to_owned())
        );
        assert_eq!(view.message(), None, "nothing granted");
    }

    #[test]
    fn an_offer_accepted_at_once_queues_its_before_movie_first() {
        let mut catalog = offering(0, None, true);
        catalog.mission_texts[0].movies.offer = nova_sim::DescMovie::from_desc("Ace.mov", 0x0004);
        let mut view = targeting_ace(catalog);
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_offer(), None);
        assert_eq!(
            view.take_mission_texts().first(),
            nova_sim::DescMovie::from_desc("Ace.mov", 0x0004)
                .map(nova_sim::MissionShown::Movie)
                .as_ref()
        );
    }

    #[test]
    fn an_offer_accepted_at_once_shows_nothing() {
        let mut view = targeting_ace(offering(0, None, true));
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_offer(), None);
        assert_eq!(view.take_hail(), None);
        assert_eq!(
            view.pilot().expect("a pilot").missions().missions(),
            [MissionId(130)]
        );
    }

    // The navigation target.

    /// Two stellars the ship starts over, at rest: A (128), small and
    /// nearest, then B (129), wide and a little further off, in that nav
    /// order; and C (130), drawn where the status panel covers it.
    fn a_and_b() -> View {
        let wide = LandingSite {
            frame_size: Some((200, 200)),
            ..site(129, (40.0, 0.0), StellarFlags::CAN_LAND)
        };
        flight_among(vec![
            site(128, (10.0, 0.0), StellarFlags::CAN_LAND),
            wide,
            site(130, (330.0, 0.0), StellarFlags::CAN_LAND),
        ])
    }

    /// Where `view` draws the system point `(x, y)`.
    fn on_screen(view: &View, (x, y): (f32, f32)) -> Point {
        view.camera().world_to_screen(at(x, y))
    }

    #[test]
    fn a_slot_key_retargets_and_l_lands_on_the_new_target() {
        for slot_key in STELLAR_SLOT_KEYS[1] {
            let mut view = a_and_b();
            tap(&mut view, LAND);
            assert_eq!(nav_target(&view), Some(StellarId(128)), "{slot_key:?}");
            tap(&mut view, slot_key);
            assert_eq!(nav_target(&view), Some(StellarId(129)), "{slot_key:?}");
            land_now(&mut view);
            assert_eq!(view.take_landing(), Some(StellarId(129)), "{slot_key:?}");
        }
    }

    #[test]
    fn the_slot_keys_are_1_to_4_and_f1_to_f4_in_nav_order() {
        assert_eq!(
            STELLAR_SLOT_KEYS,
            [
                [Key::Char('1'), Key::Function(1)],
                [Key::Char('2'), Key::Function(2)],
                [Key::Char('3'), Key::Function(3)],
                [Key::Char('4'), Key::Function(4)],
            ]
        );
        for (slot, keys) in STELLAR_SLOT_KEYS.iter().enumerate().take(3) {
            for &slot_key in keys {
                let mut view = a_and_b();
                tap(&mut view, slot_key);
                let expected = [128, 129, 130][slot];
                assert_eq!(nav_target(&view), Some(StellarId(expected)), "{slot_key:?}");
            }
        }
        let mut view = a_and_b();
        tap(&mut view, STELLAR_SLOT_KEYS[1][0]);
        for &empty in &STELLAR_SLOT_KEYS[3] {
            tap(&mut view, empty);
            assert_eq!(nav_target(&view), Some(StellarId(129)), "empty slot keeps");
        }
    }

    #[test]
    fn a_slot_key_repeat_or_release_does_nothing() {
        let mut view = a_and_b();
        view.input(&held(Key::Char('2')));
        view.input(&key(Key::Char('2'), false));
        assert_eq!(nav_target(&view), None);
    }

    #[test]
    fn a_click_on_a_stellar_retargets_and_l_lands_there() {
        let mut view = a_and_b();
        tap(&mut view, LAND);
        let b = on_screen(&view, (40.0, 0.0));
        click(&mut view, b);
        assert_eq!(nav_target(&view), Some(StellarId(129)));
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(129)));
    }

    #[test]
    fn a_click_picks_the_stellar_above_or_below_the_ship_where_it_is_drawn() {
        let mut view = flight_among(vec![
            site(128, (0.0, 150.0), StellarFlags::CAN_LAND),
            site(129, (0.0, -150.0), StellarFlags::CAN_LAND),
        ]);
        for (id, y) in [(129, -150.0), (128, 150.0), (129, -150.0)] {
            let drawn = on_screen(&view, (0.0, y));
            click(&mut view, drawn);
            assert_eq!(nav_target(&view), Some(StellarId(id)), "{y}");
        }
    }

    #[test]
    fn a_click_on_empty_space_keeps_the_target() {
        let mut view = a_and_b();
        tap(&mut view, LAND);
        let empty = on_screen(&view, (-300.0, -300.0));
        click(&mut view, empty);
        assert_eq!(nav_target(&view), Some(StellarId(128)));
    }

    #[test]
    fn a_right_click_or_a_release_selects_nothing() {
        let mut view = a_and_b();
        let b = on_screen(&view, (40.0, 0.0));
        for (button, pressed) in [
            (crate::MouseButton::Right, true),
            (crate::MouseButton::Left, false),
        ] {
            view.input(&Input::PointerButton {
                button,
                pressed,
                at: b,
            });
            assert_eq!(nav_target(&view), None, "{button:?} {pressed}");
        }
    }

    #[test]
    fn a_click_on_the_status_panel_keeps_the_target_even_over_a_stellar() {
        let mut view = a_and_b();
        tap(&mut view, LAND);
        let c = on_screen(&view, (330.0, 0.0));
        let edge = VIEW_SIZE.0 - STATUS_BAR_WIDTH;
        assert!(c.x > edge, "{c:?} is under the panel");
        click(&mut view, c);
        assert_eq!(nav_target(&view), Some(StellarId(128)), "over C");
        // C's box reaches 26 pixels either side of it, across the panel's
        // edge, which decides.
        let left_of_c = on_screen(&view, (330.0 - 26.0, 0.0)).x;
        assert!(left_of_c < edge && edge < c.x);
        view.take_sounds();
        click(&mut view, at(edge, c.y));
        assert_eq!(nav_target(&view), Some(StellarId(128)), "the edge is panel");
        assert_eq!(view.take_sounds(), [], "the panel is silent");
        click(&mut view, at(edge - 0.5, c.y));
        assert_eq!(
            nav_target(&view),
            Some(StellarId(130)),
            "left of it is space"
        );
    }

    #[test]
    fn f5_selects_the_nearest_stellar() {
        let mut view = a_and_b();
        tap(&mut view, STELLAR_SLOT_KEYS[1][0]);
        tap(&mut view, NEAREST_STELLAR_KEY);
        assert_eq!(nav_target(&view), Some(StellarId(128)));
        assert_eq!(NEAREST_STELLAR_KEY, Key::Function(5));
    }

    #[test]
    fn nav_off_clears_the_target_and_the_next_l_selects_the_nearest() {
        let mut view = a_and_b();
        tap(&mut view, STELLAR_SLOT_KEYS[1][0]);
        tap(&mut view, NAV_OFF_KEY);
        assert_eq!(nav_target(&view), None);
        tap(&mut view, LAND);
        assert_eq!(view.take_landing(), None, "selects, not lands");
        assert_eq!(nav_target(&view), Some(StellarId(128)));
        assert_eq!(NAV_OFF_KEY, Key::Char('`'));
    }

    fn nav_mode(view: &View) -> NavMode {
        view.session().expect("flying").nav_mode()
    }

    #[test]
    fn nav_off_with_a_course_draws_nav_system_off_keeps_the_course_and_refuses_j() {
        let mut view = flight();
        plot(&mut view, 131);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
        view.take_sounds();
        tap(&mut view, NAV_OFF_KEY);
        assert_eq!(nav(&view), [hud::NAV_OFF]);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::NavSelected)]);
        assert!(!view.radar_now().hyperspace_mode);
        assert_eq!(course(&view), [SystemId(131)], "kept");
        fly_out(&mut view);
        tap(&mut view, JUMP);
        assert!(!view.preparing_jump(), "refused");
        assert_eq!(view.message(), None, "a seasoned pilot is not told");
    }

    #[test]
    fn far_from_the_centre_nav_off_with_a_course_lets_the_arrow_show() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_far(&mut view);
        let most_lines = |view: &mut View| {
            (0..32)
                .map(|_| {
                    ticks(view, 1);
                    arrow_lines(view)
                })
                .max()
        };
        assert_eq!(most_lines(&mut view), Some(0), "mode 3 hides it");
        tap(&mut view, NAV_OFF_KEY);
        assert_eq!(most_lines(&mut view), Some(3), "mode -1 shows it");
    }

    #[test]
    fn hyperspace_mode_with_no_course_draws_hyperspace_over_no_destination() {
        let mut view = flight();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_NO_DESTINATION]);
        assert!(view.radar_now().hyperspace_mode, "the arrow hidden");
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::NavSelected)]);
        assert_eq!(HYPERSPACE_MODE_KEY, Key::Char('h'));
    }

    #[test]
    fn hyper_select_with_no_links_in_mode_3_draws_hyperspace_over_no_destination() {
        let mut view = FlightView::new(FakeCatalog {
            onward: true,
            ..catalog()
        });
        run_set(&mut view, "M132");
        view.settle_script();
        assert_eq!(system_of(&view), SystemId(132), "Barnard lists no links");
        tap(&mut view, HYPERSPACE_MODE_KEY);
        view.take_sounds();
        tap(&mut view, HYPER_SELECT_KEY);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
        assert_eq!(view.take_sounds(), [], "in silence");
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_NO_DESTINATION]);
    }

    #[test]
    fn closing_the_course_map_cues_the_courses_first_hop() {
        let mut view = flight();
        plot(&mut view, 131);
        tap(&mut view, NAV_OFF_KEY);
        assert_eq!(nav_mode(&view), NavMode::Off);
        tap(&mut view, MAP);
        tap(&mut view, MAP);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
    }

    #[test]
    fn the_hypergate_map_opens_with_the_current_system_selected() {
        let mut view = FlightView::new(gated());
        plot(&mut view, 131);
        land_now(&mut view);
        let map = view.course_map();
        assert_eq!(map.mode(), MapMode::Hypergate);
        assert_eq!(map.selected(), Some(SystemId(130)), "Sol");
        assert_eq!(map.hyper_selection(), None, "nothing in mode 3");
        tap(&mut view, MAP);
        assert_eq!(view.message(), Some(HYPERGATE_CANCELLED), "no pick");
    }

    #[test]
    fn closing_the_hypergate_map_turns_the_nav_off_and_cues_nothing() {
        let mut view = FlightView::new(gated());
        plot(&mut view, 131);
        land_now(&mut view);
        tap(&mut view, MAP);
        assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
        assert_eq!(course(&view), [SystemId(131)], "the course kept");
        assert_eq!(nav_mode(&view), NavMode::Off);
    }

    #[test]
    fn alt_backquote_lets_go_of_the_ship_target_and_leaves_the_nav_mode() {
        let mut view = fighting(armed(2, &[]), &two_npcs());
        tap(&mut view, LAND);
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), Some(NpcId(0)));
        view.take_sounds();
        alt_tap(&mut view, NAV_OFF_KEY);
        assert_eq!(target_of(&view), None);
        assert!(nav_target(&view).is_some(), "the nav system stays on");
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TargetCleared)]);
        alt_tap(&mut view, NAV_OFF_KEY);
        assert_eq!(view.take_sounds(), [], "no target: silent");
    }

    // The nav target's brackets.

    /// The nav bracket `cïcn`s drawn (10000-10007), and where.
    fn nav_icons(view: &View) -> Vec<(i16, Point)> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Picture { image, top_left } if icon_in(10000..=10007)(command) => {
                    Some((image.id, *top_left))
                }
                _ => None,
            })
            .collect()
    }

    /// The four corners from `cïcn` `first` round `centre`, half-size
    /// `half`: top-left, top-right, bottom-right, bottom-left.
    fn corners_at(first: i16, centre: Point, half: f32) -> Vec<(i16, Point)> {
        let (near, far) = (-half - 16.0, half);
        vec![
            (first, at(centre.x + near, centre.y + near)),
            (first + 1, at(centre.x + far, centre.y + near)),
            (first + 2, at(centre.x + far, centre.y + far)),
            (first + 3, at(centre.x + near, centre.y + far)),
        ]
    }

    #[test]
    fn selecting_a_stellar_draws_cicn_10000_to_10003_flying_in_then_settled() {
        let mut view = flight();
        assert_eq!(nav_icons(&view), []);
        tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
        // Earth (128), 20 x 20: half 10.
        let earth = on_screen(&view, (0.0, -600.0));
        assert_eq!(
            nav_icons(&view),
            corners_at(10000, earth, 10.0 + 196.0),
            "a frame drawn before the next tick"
        );
        for expand in [196.0, 136.0, 76.0, 16.0, 0.0, 0.0] {
            view.tick(TICK);
            let earth = on_screen(&view, (0.0, -600.0));
            assert_eq!(
                nav_icons(&view),
                corners_at(10000, earth, 10.0 + expand),
                "{expand}"
            );
        }
    }

    #[test]
    fn no_nav_brackets_without_a_nav_target_or_after_hyper_select() {
        let mut view = flight();
        tap(&mut view, STELLAR_SLOT_KEYS[1][0]);
        assert_eq!(nav_icons(&view).len(), 4);
        tap(&mut view, NAV_OFF_KEY);
        assert_eq!(nav_icons(&view), [], "Nav Off");
        view.tick(TICK);
        assert_eq!(nav_icons(&view), []);
        tap(&mut view, STELLAR_SLOT_KEYS[1][0]);
        let moon = on_screen(&view, (300.0, -200.0));
        assert_eq!(
            nav_icons(&view),
            corners_at(10000, moon, 10.0 + 196.0),
            "flies in again"
        );
        tap(&mut view, HYPER_SELECT_KEY);
        assert_eq!(view.session().expect("flying").nav_target(), None);
        assert_eq!(nav_icons(&view), [], "Hyper Select");
    }

    #[test]
    fn a_stellar_whose_sprite_cannot_be_read_is_bracketed_at_half_64() {
        let unreadable = StellarContents {
            sprite: Err("no rlëD".to_owned()),
            ..stellar(131, "Broken", (100, 0), 1)
        };
        let mut sites = catalog().sites;
        sites.push(site(131, (100.0, 0.0), StellarFlags::CAN_LAND));
        let mut view = FlightView::new(FakeCatalog {
            sites,
            sol_more: vec![unreadable],
            ..catalog()
        });
        tap(&mut view, STELLAR_SLOT_KEYS[2][0]);
        ticks(&mut view, 5);
        let broken = on_screen(&view, (100.0, 0.0));
        assert_eq!(nav_icons(&view), corners_at(10000, broken, 64.0));
    }

    #[test]
    fn an_always_dominated_stellar_is_bracketed_green() {
        let mut sites = catalog().sites;
        sites[0].flags2 = nova_sim::market::DOMINATED;
        let mut view = flight_among(sites);
        tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
        ticks(&mut view, 5);
        let earth = on_screen(&view, (0.0, -600.0));
        assert_eq!(nav_icons(&view), corners_at(10004, earth, 10.0));
    }

    #[test]
    fn with_the_map_open_a_click_goes_to_the_map_not_the_stellars() {
        let mut view = a_and_b();
        let b = on_screen(&view, (40.0, 0.0));
        tap(&mut view, MAP);
        click(&mut view, b);
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert_eq!(nav_target(&view), None);
    }

    fn nav_target(view: &View) -> Option<StellarId> {
        view.session().expect("flying").nav_target()
    }

    /// The texts drawn in the HUD's nav area, the stock bar's at (830, 0).
    fn nav(view: &View) -> Vec<String> {
        let area = layout().nav.offset(at(830.0, 0.0));
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } if area.contains(*origin) => {
                    Some(text.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// The texts drawn in the HUD's nav area, as [`nav`] gives them, each
    /// with its colour.
    fn nav_colored(view: &View) -> Vec<(String, Color)> {
        let area = layout().nav.offset(at(830.0, 0.0));
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    ..
                } if area.contains(*origin) => Some((text.clone(), *color)),
                _ => None,
            })
            .collect()
    }

    /// The colour the nav area's value, its second line, is drawn in.
    fn nav_value_color(view: &View) -> Color {
        nav_colored(view)[1].1
    }

    /// The texts drawn in the cargo area, at the stock bar's (830, 0),
    /// with where each starts.
    fn cargo(view: &View) -> Vec<(String, Point)> {
        let area = layout().cargo.offset(at(830.0, 0.0));
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } if area.contains(*origin) => {
                    Some((text.clone(), *origin))
                }
                _ => None,
            })
            .collect()
    }

    /// `text` from `x` right of the stock cargo area's left, (8, 458) at
    /// (830, 0), on `baseline` below its top, in Geneva 12.
    fn cargo_line(text: &str, x: f32, baseline: f32) -> (String, Point) {
        (text.to_owned(), at(838.0 + x, 458.0 + baseline - 12.0))
    }

    #[test]
    fn the_hud_lists_the_goods_bought_the_free_space_and_the_cash() {
        let labels = FakeCatalog {
            strings: vec![(4003, vec!["Food:".to_owned(), "Ind:".to_owned()])],
            ..catalog()
        };
        let mut view = FlightView::new(trading()).with_strings(Rc::new(labels));
        assert_eq!(
            cargo(&view),
            [
                cargo_line(hud::CARGO_FREE, 77.0, 12.0),
                cargo_line("10", 110.0, 12.0),
                cargo_line("Credits:", 77.0, 66.0),
                cargo_line("1,000", 87.0, 82.0),
            ],
            "an empty 10-ton hold"
        );
        land_now(&mut view);
        assert_eq!(view.trade(BUY_FOOD), Ok(10));
        assert_eq!(
            cargo(&view),
            [
                cargo_line("Food:", 3.0, 12.0),
                cargo_line("10", 41.0, 12.0),
                cargo_line(hud::CARGO_FREE, 77.0, 12.0),
                cargo_line("0", 110.0, 12.0),
                cargo_line("Credits:", 77.0, 66.0),
                cargo_line("250", 87.0, 82.0),
            ]
        );
    }

    #[test]
    fn a_cargo_area_with_room_below_the_credits_draws_no_date() {
        assert_eq!(
            layout().cargo,
            crate::geometry::Bounds {
                min: at(8.0, 458.0),
                max: at(184.0, 552.0),
            }
        );
        let taller = FakeCatalog {
            bar: Ok(StatusBarLayout {
                cargo: crate::geometry::Bounds {
                    min: at(8.0, 440.0),
                    max: at(184.0, 552.0),
                },
                ..layout()
            }),
            ..catalog()
        };
        let mut view = FlightView::new(taller);
        let no_date = |view: &View| {
            let date = view.session().expect("flying").date_text();
            let dates: Vec<_> = drawn(view)
                .iter()
                .filter(
                    |command| matches!(command, DrawCommand::Text { text, .. } if *text == date),
                )
                .cloned()
                .collect();
            assert_eq!(dates, [], "no date drawn for {date}");
            assert!(
                cargo(view).iter().any(|(text, _)| text == "Credits:"),
                "the cargo lines are still drawn"
            );
        };
        no_date(&view);
        jump_to_alpha(&mut view);
        no_date(&view);
    }

    #[test]
    fn with_nothing_selected_and_no_course_the_hud_says_nav_system_off() {
        assert_eq!(nav(&flight()), [hud::NAV_OFF]);
        assert_eq!(nav(&every_frame(flight())), [hud::NAV_OFF]);
    }

    #[test]
    fn the_nav_area_is_centred_by_the_metrics_given() {
        use crate::text::TextMetrics as _;
        use crate::text::fixture::MonoMetrics;
        let view = flight().with_metrics(Rc::new(MonoMetrics));
        let origin = drawn(&view)
            .iter()
            .find_map(|c| match c {
                DrawCommand::Text { text, origin, .. } if text == hud::NAV_OFF => Some(*origin),
                _ => None,
            })
            .expect("Nav System Off");
        let layout = layout();
        let area = layout.nav.offset(at(830.0, 0.0));
        let width = MonoMetrics.width(layout.font, layout.font_size, hud::NAV_OFF);
        // The stock area's middle is 830 + (8 + 184) / 2 = 926.
        assert_eq!(f32::midpoint(area.min.x, area.max.x), 926.0);
        assert_eq!(
            origin,
            at(
                926.0 - (width / 2.0).trunc(),
                area.min.y + hud::NAV_OFF_BASELINE - layout.font_size
            )
        );
    }

    #[test]
    fn a_stellar_target_not_in_the_scene_is_stellar_navigation_over_no_destination() {
        let mut view = flight();
        tap(&mut view, LAND);
        assert!(nav_target(&view).is_some());
        view.scene = None;
        assert_eq!(view.nav_now(), NavDisplay::Stellar(None));
    }

    #[test]
    fn the_hud_shows_the_selected_stellar_by_name() {
        let mut view = flight();
        tap(&mut view, LAND);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"], "the nearest");
    }

    #[test]
    fn a_plotted_jump_shows_hyperspace_and_the_next_system_if_explored() {
        let mut view = flight();
        plot(&mut view, 131);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
        jump_to_alpha(&mut view);
        assert_eq!(nav(&view), [hud::NAV_OFF], "arrived");
        plot(&mut view, 130);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Sol"]);
    }

    #[test]
    fn the_destination_is_dim_until_the_ship_is_out_far_enough_to_jump() {
        // Read the Bible's way, the nav area follows every frame.
        let mut view = every_frame(flight());
        plot(&mut view, 131);
        let hyperspace = |value: Color| {
            vec![
                (hud::NAV_HYPERSPACE.to_owned(), Color::DIM),
                (hud::NAV_UNEXPLORED.to_owned(), value),
            ]
        };
        assert_eq!(nav_colored(&view), hyperspace(Color::DIM));
        let clear_iff_out = |view: &View| {
            let out = player(view).position.length() >= MIN_JUMP_DISTANCE;
            let expected = if out { Color::WHITE } else { Color::DIM };
            assert_eq!(nav_value_color(view), expected, "{:?}", player(view));
            out
        };
        view.input(&key(Key::Up, true));
        let mut dim_ticks = 0;
        while !clear_iff_out(&view) {
            dim_ticks += 1;
            assert!(dim_ticks < 2000, "never got out");
            view.tick(TICK);
        }
        assert!(dim_ticks > 0, "it started inside");
        assert_eq!(nav_colored(&view), hyperspace(Color::WHITE));
        // Back inside: Down turns against the motion, then Up thrusts.
        view.input(&key(Key::Up, false));
        view.input(&key(Key::Down, true));
        for _ in 0..90 {
            view.tick(TICK);
            clear_iff_out(&view);
        }
        view.input(&key(Key::Down, false));
        view.input(&key(Key::Up, true));
        let mut out_ticks = 0;
        while clear_iff_out(&view) {
            out_ticks += 1;
            assert!(out_ticks < 2000, "never got back in");
            view.tick(TICK);
        }
        view.release_keys();
        assert_eq!(nav_colored(&view), hyperspace(Color::DIM));
    }

    #[test]
    fn in_a_system_with_no_stellars_the_destination_is_bright_from_the_centre() {
        let mut view = FlightView::new(FakeCatalog {
            sites: Vec::new(),
            ..catalog()
        });
        plot(&mut view, 131);
        assert_eq!(player(&view).position, Vec2::ZERO);
        assert_eq!(nav_value_color(&view), Color::WHITE);
    }

    #[test]
    fn once_j_is_accepted_the_hyperspace_label_turns_bright_too() {
        let mut view = every_frame(flight());
        plot(&mut view, 131);
        fly_out(&mut view);
        tap(&mut view, JUMP);
        assert_eq!(
            nav_colored(&view),
            [
                (hud::NAV_HYPERSPACE.to_owned(), Color::WHITE),
                (hud::NAV_UNEXPLORED.to_owned(), Color::WHITE),
            ]
        );
    }

    /// The engine tick `view` has reached.
    fn now_tick(view: &View) -> u64 {
        u64::try_from(crate::time::ticks(view.elapsed)).expect("in range")
    }

    #[test]
    fn the_nav_area_holds_its_colours_until_its_next_refresh() {
        let mut view = flight();
        plot(&mut view, 131);
        assert_eq!(nav_value_color(&view), Color::DIM, "inside, at once");
        view.input(&key(Key::Up, true));
        let mut expected = Color::DIM;
        let mut held = 0;
        for _ in 0..2000 {
            let before = now_tick(&view);
            view.tick(TICK);
            let now = now_tick(&view);
            let out = player(&view).position.length() >= MIN_JUMP_DISTANCE;
            if now / NAV_PERIOD_TICKS > before / NAV_PERIOD_TICKS {
                expected = if out { Color::WHITE } else { Color::DIM };
            }
            assert_eq!(nav_value_color(&view), expected, "tick {now}");
            if out && expected == Color::DIM {
                held += 1;
            }
            if expected == Color::WHITE {
                break;
            }
        }
        view.release_keys();
        assert_eq!(expected, Color::WHITE, "it got out");
        assert!(
            (1..u32::try_from(NAV_PERIOD_TICKS).expect("small")).contains(&held),
            "held dim {held} ticks after the ship got clear"
        );
    }

    #[test]
    fn the_nav_area_holds_its_text_until_its_next_refresh() {
        let mut view = flight();
        plot(&mut view, 131);
        // Move to just past a refresh.
        while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        run_set(&mut view, "X131");
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
        for _ in 1..NAV_PERIOD_TICKS - 1 {
            view.tick(TICK);
            assert_eq!(
                nav(&view),
                [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED],
                "tick {}",
                now_tick(&view)
            );
        }
        view.tick(TICK);
        assert_eq!(now_tick(&view) % NAV_PERIOD_TICKS, 0);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Alpha Centauri"]);
    }

    #[test]
    fn read_the_bibles_way_the_nav_area_follows_every_frame() {
        let mut view = every_frame(flight());
        plot(&mut view, 131);
        while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        run_set(&mut view, "X131");
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Alpha Centauri"]);
    }

    #[test]
    fn a_selection_change_shows_at_once() {
        let mut view = flight();
        while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        tap(&mut view, LAND);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"], "drawn at once");
        view.tick(TICK);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"], "and kept");
        tap(&mut view, HYPER_SELECT);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
        view.tick(TICK);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
        assert_ne!(now_tick(&view) % NAV_PERIOD_TICKS, 0, "between refreshes");
    }

    #[test]
    fn redrawing_the_status_retakes_the_nav_area() {
        let mut view = flight();
        plot(&mut view, 131);
        while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        run_set(&mut view, "X131");
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
        view.redraw_status();
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Alpha Centauri"]);
    }

    #[test]
    fn once_j_is_accepted_the_label_turns_bright_at_the_next_refresh() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        let label = |view: &View| nav_colored(view)[0].1;
        assert_eq!(label(&view), Color::DIM);
        tap(&mut view, JUMP);
        assert_eq!(label(&view), Color::DIM, "held");
        for _ in 1..NAV_PERIOD_TICKS - 1 {
            view.tick(TICK);
            assert_eq!(label(&view), Color::DIM, "tick {}", now_tick(&view));
        }
        view.tick(TICK);
        assert_eq!(label(&view), Color::WHITE, "tick {}", now_tick(&view));
    }

    #[test]
    fn with_a_target_and_a_course_the_hud_shows_the_target() {
        let mut view = flight();
        plot(&mut view, 131);
        tap(&mut view, LAND);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"]);
        assert_eq!(
            view.session().expect("flying").course(),
            [SystemId(131)],
            "the course is kept"
        );
    }

    #[test]
    fn arriving_clears_the_target_and_l_selects_in_the_new_system() {
        let mut view = flight();
        tap(&mut view, LAND);
        jump_to_alpha(&mut view);
        assert_eq!(nav_target(&view), None);
        assert_eq!(nav(&view), [hud::NAV_OFF]);
        tap(&mut view, LAND);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Proxima"]);
    }

    #[test]
    fn tab_in_a_flight_that_never_started_does_nothing() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert_eq!(view.input(&key(Key::Tab, true)), ScreenAction::None);
        assert!(view.session().is_err());
    }

    // Hyper Select.

    const HYPER_SELECT: Key = Key::Char('\\');

    /// A flight from Sol, which lists Alpha Centauri then Barnard.
    fn fanned() -> View {
        FlightView::new(FakeCatalog {
            fan: true,
            ..catalog()
        })
    }

    fn course(view: &View) -> Vec<SystemId> {
        view.session().expect("flying").course().to_vec()
    }

    /// The system selected in nav mode 3.
    fn hyper_selected(view: &View) -> Option<SystemId> {
        nav_mode(view).hyperspace()
    }

    #[test]
    fn the_hyper_select_key_selects_the_next_listed_system() {
        let mut view = fanned();
        assert_eq!(HYPER_SELECT_KEY, HYPER_SELECT);
        for expected in [131, 132, 131] {
            assert_eq!(view.input(&key(HYPER_SELECT, true)), ScreenAction::None);
            view.input(&key(HYPER_SELECT, false));
            assert_eq!(hyper_selected(&view), Some(SystemId(expected)));
            assert_eq!(course(&view), [], "no course plotted");
        }
    }

    #[test]
    fn hyper_select_in_flight_keeps_the_course() {
        let mut view = fanned();
        plot(&mut view, 131);
        tap(&mut view, HYPER_SELECT);
        assert_eq!(course(&view), [SystemId(131)]);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(132))));
    }

    #[test]
    fn the_hud_nav_area_shows_the_selected_system() {
        let mut view = flight();
        tap(&mut view, LAND);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"]);
        tap(&mut view, HYPER_SELECT);
        assert_eq!(nav_target(&view), None);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
    }

    #[test]
    fn a_repeat_or_release_of_the_hyper_select_key_does_not_cycle() {
        let mut view = fanned();
        view.input(&held(HYPER_SELECT));
        view.input(&key(HYPER_SELECT, false));
        assert_eq!(hyper_selected(&view), None);
        tap(&mut view, HYPER_SELECT);
        view.input(&held(HYPER_SELECT));
        view.input(&key(HYPER_SELECT, false));
        assert_eq!(hyper_selected(&view), Some(SystemId(131)));
    }

    #[test]
    fn hyper_select_is_ignored_while_a_jump_prepares_or_plays() {
        let mut view = fanned();
        tap(&mut view, HYPER_SELECT);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        assert!(view.preparing_jump(), "braking and turning");
        tap(&mut view, HYPER_SELECT);
        assert_eq!(hyper_selected(&view), Some(SystemId(131)));
        jump_now(&mut view);
        tap(&mut view, HYPER_SELECT);
        assert_eq!(hyper_selected(&view), Some(SystemId(131)));
    }

    #[test]
    fn hyper_select_with_the_map_open_changes_nothing() {
        let mut view = fanned();
        tap(&mut view, MAP);
        tap(&mut view, HYPER_SELECT);
        assert!(view.map_open());
        assert_eq!(course(&view), []);
    }

    // Hypergates and wormholes.

    use crate::flight::jump::{ARRIVAL_FLASH_FOR, FADE_IN_FOR};
    use nova_sim::GateKind;

    /// Stellar `id` at (`x`, `y`), with `flags2`, `links` and
    /// `exit_angle`.
    fn gate_site(
        id: i16,
        (x, y): (f32, f32),
        flags2: u16,
        links: &[i16],
        exit_angle: i16,
    ) -> GateSite {
        let mut slots = [None; 8];
        for (slot, &link) in slots.iter_mut().zip(links) {
            *slot = Some(StellarId(link));
        }
        GateSite {
            id: StellarId(id),
            position: Vec2::new(x, y),
            flags2,
            links: slots,
            exit_angle,
        }
    }

    /// Sol holds hypergate 300 (a station) at its centre, over the ship,
    /// linked to 310 in Alpha Centauri at (100, 200), heading ships out on
    /// 90°.
    fn gated() -> FakeCatalog {
        FakeCatalog {
            sites: vec![LandingSite {
                flags2: 0x1200,
                ..site(
                    300,
                    (0.0, 0.0),
                    StellarFlags::CAN_LAND | StellarFlags::STATION,
                )
            }],
            gates: vec![
                gate_site(300, (0.0, 0.0), 0x1200, &[310], 120),
                gate_site(310, (100.0, 200.0), 0x1000, &[300], 90),
            ],
            gate_listers: vec![(SystemId(131), StellarId(310))],
            ..catalog()
        }
    }

    /// Sol holds unlinked wormhole 400 at its centre, over the ship, and
    /// Alpha Centauri unlinked wormhole 410 at (-300, 40).
    fn holed() -> FakeCatalog {
        FakeCatalog {
            sites: vec![LandingSite {
                flags2: 0x2200,
                ..site(400, (0.0, 0.0), StellarFlags::CAN_LAND)
            }],
            gates: vec![
                gate_site(400, (0.0, 0.0), 0x2200, &[], 120),
                gate_site(410, (-300.0, 40.0), 0x2000, &[], 0),
            ],
            gate_listers: vec![(SystemId(131), StellarId(410))],
            ..catalog()
        }
    }

    fn system_of(view: &View) -> SystemId {
        view.session().expect("flying").system()
    }

    #[test]
    fn the_gate_messages_read_as_the_original() {
        assert_eq!(
            arrival_message_with(EXITING_HYPERGATE, "Alpha Centauri", DATE, true),
            "Exiting hypergate in the Alpha Centauri system on June 23, 1177 NC."
        );
        assert_eq!(
            arrival_message_with(PASSING_WORMHOLE, "Barnard", DATE, false),
            "Passing through a wormhole into the Barnard system on June 23, 1177 NC. \
             No stellar objects present."
        );
        assert_eq!(
            arrival_message_with(JUMPING_INTO, "Sol", DATE, true),
            arrival_message("Sol", DATE, true)
        );
        assert_eq!(
            gate_refusal_message(GateKind::Hypergate),
            "Your ship is unable to enter this hypergate - it is offline."
        );
        assert_eq!(
            gate_refusal_message(GateKind::Wormhole),
            "Your ship is unable to enter this wormhole - the radiation levels are too extreme."
        );
        assert_eq!(HYPERGATE_CANCELLED, "Hypergate jump cancelled.");
        assert_eq!(HYPERGATE_DENIED, "Hypergate usage denied.");
    }

    /// The course map's sounds `view` holds, taken with the rest.
    fn map_sounds(view: &mut View) -> Vec<UiSound> {
        ui_sounds(view)
            .into_iter()
            .filter(|sound| matches!(sound, UiSound::MapOpened | UiSound::MapClosed))
            .collect()
    }

    #[test]
    fn m_beeps_as_it_opens_and_closes_the_course_map() {
        let mut view = flight();
        view.take_sounds();
        tap(&mut view, MAP);
        assert!(view.map_open());
        assert_eq!(ui_sounds(&mut view), [UiSound::MapOpened], "opened");
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert_eq!(ui_sounds(&mut view), [UiSound::MapClosed], "closed");
    }

    #[test]
    fn escape_closing_the_course_map_beeps_its_close() {
        let mut view = flight();
        tap(&mut view, MAP);
        view.take_sounds();
        view.close_map();
        assert_eq!(ui_sounds(&mut view), [UiSound::MapClosed]);
    }

    #[test]
    fn the_course_map_makes_no_map_sound_on_its_own_input_or_ms_repeats() {
        let mut view = flight();
        tap(&mut view, MAP);
        view.take_sounds();
        view.input(&held(MAP));
        view.input(&key(MAP, false));
        for k in [Key::Up, Key::Down, Key::Left, Key::Right] {
            tap(&mut view, k);
        }
        let somewhere = on_map(&view, 131);
        click(&mut view, somewhere);
        assert!(view.map_open());
        assert_eq!(map_sounds(&mut view), []);
    }

    #[test]
    fn a_refused_m_makes_no_map_sound() {
        let mut view = breaking_up();
        view.take_sounds();
        tap(&mut view, MAP_KEY);
        assert!(!view.map_open());
        assert_eq!(map_sounds(&mut view), [], "breaking up");
        for mut view in through_a_jump(flight) {
            let stage = stage(&view);
            view.take_sounds();
            tap(&mut view, MAP);
            assert!(!view.map_open(), "{stage}");
            assert_eq!(map_sounds(&mut view), [], "{stage}");
        }
        let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(140)));
        assert_eq!(view.take_off(), Some(StellarId(140)));
        view.take_sounds();
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert_eq!(map_sounds(&mut view), [], "the take-off grace");
    }

    #[test]
    fn the_hypergate_map_makes_no_map_sound() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        assert_eq!(view.course_map().mode(), MapMode::Hypergate);
        assert_eq!(map_sounds(&mut view), [], "opened");
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        tap(&mut view, MAP);
        assert_eq!(system_of(&view), SystemId(131));
        assert_eq!(map_sounds(&mut view), [], "closed on a pick");
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        view.take_sounds();
        view.close_map();
        assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
        assert_eq!(map_sounds(&mut view), [], "cancelled");
    }

    #[test]
    fn a_hypergates_greeting_is_one_of_its_own_by_a_roll() {
        let mut view = FlightView::new(gated());
        tap(&mut view, LAND);
        assert_eq!(view.message(), None, "requested over it: silent");
        ticks(&mut view, 1);
        let ship = ship_name(&view);
        let first = format!("Cleared for hypergate entry, {ship}. ");
        assert_eq!(view.message(), Some(first.as_str()), "the first outcome");
        let last: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always::default()));
        let mut view = FlightView::new(gated()).with_chance(SharedChance::new(last));
        tap(&mut view, LAND);
        ticks(&mut view, 1);
        assert_lasts(&mut view, "You are cleared for hypergate entry. ", 250);
    }

    #[test]
    fn a_hypergate_the_pilot_may_not_use_denies_it_in_its_own_words() {
        let mut catalog = gated();
        catalog.sites[0].min_status = 100;
        let mut view = FlightView::new(catalog);
        tap(&mut view, LAND);
        assert_lasts(&mut view, HYPERGATE_DENIED, 250);
    }

    #[test]
    fn an_offline_gate_is_refused_in_its_own_words() {
        for (flags2, kind) in [(0x1000, GateKind::Hypergate), (0x2000, GateKind::Wormhole)] {
            let mut view = flight_among(vec![LandingSite {
                flags2,
                ..site(300, (0.0, 0.0), StellarFlags::STATION)
            }]);
            tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
            tap(&mut view, LAND);
            let refusal = gate_refusal_message(kind);
            assert_lasts(&mut view, &refusal, 250);
        }
        let mut planet = flight_among(vec![site(128, (0.0, 0.0), 0)]);
        tap(&mut planet, STELLAR_SLOT_KEYS[0][0]);
        tap(&mut planet, LAND);
        let line =
            "Your ship is unable to land on  Earth. The planet's environment is too hostile.";
        assert_lasts(&mut planet, line, 250);
    }

    #[test]
    fn l_twice_at_a_hypergate_opens_the_map_offering_its_links() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        assert!(view.map_open());
        assert_eq!(view.course_map().mode(), MapMode::Hypergate);
        assert_eq!(view.take_landing(), None, "not landed");
        assert_eq!(system_of(&view), SystemId(130));
    }

    /// Taps I in `view`, its sounds taken first: nothing is asked for,
    /// nothing said and nothing sounded.
    fn i_does_nothing_and_says_nothing(view: &mut View, why: &str) {
        let _ = view.take_sounds();
        let message = view.message().map(str::to_owned);
        tap(view, MISSION_INFO_KEY);
        assert_eq!(view.take_mission_info_request(), None, "{why}");
        assert_eq!(view.message(), message.as_deref(), "{why}");
        assert_eq!(view.take_sounds(), [], "{why}");
    }

    #[test]
    fn i_during_a_jump_does_nothing_and_says_nothing() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        assert!(view.session().expect("flying").preparing_jump().is_some());
        i_does_nothing_and_says_nothing(&mut view, "turning to jump (@0x70424)");
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        i_does_nothing_and_says_nothing(&mut view, "streaking (@0x70424)");
    }

    #[test]
    fn i_in_the_escape_pod_does_nothing_and_says_nothing() {
        let mut pod = ejecting_view(ejecting_in(100, &[188, 187]), &[]);
        assert!(until_ejected(&mut pod).0);
        i_does_nothing_and_says_nothing(&mut pod, "in the pod (@0x7042e)");
    }

    #[test]
    fn i_while_disabled_is_taken() {
        let mut view = disabled_player();
        let _ = view.take_sounds();
        tap(&mut view, MISSION_INFO_KEY);
        assert_eq!(view.take_mission_info_request(), None);
        assert_eq!(view.message(), Some(NO_ACTIVE_MISSIONS));
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::Refused)]);
    }

    #[test]
    fn closing_the_hypergate_map_on_a_pick_comes_out_of_its_gate() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert_eq!(view.course_map().mode(), MapMode::Course);
        assert_eq!(system_of(&view), SystemId(131));
        assert_eq!(player(&view).position, Vec2::new(100.0, 200.0));
        assert_eq!(view.scene().map(SystemScene::name), Some("Alpha Centauri"));
        assert_eq!(
            view.message(),
            Some("Exiting hypergate in the Alpha Centauri system on June 23, 1177 NC.")
        );
        assert_eq!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::FadeIn(0.0))
        );
        assert_eq!(view.shown_position(), Point::new(100.0, 200.0));
        view.tick(FADE_IN_FOR);
        assert_eq!(view.jump_effect(), None, "the fade is over");
        assert_eq!(system_of(&view), SystemId(131));
        assert_eq!(
            view.course_map().current(),
            Some(SystemId(131)),
            "the map follows"
        );
    }

    #[test]
    fn the_ship_flies_under_a_hypergates_fade_in() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        tap(&mut view, MAP);
        assert_eq!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::FadeIn(0.0))
        );
        let arrived = player(&view);
        assert_eq!(arrived.position, Vec2::new(100.0, 200.0));
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(player(&view), stepped(arrived, THRUST, 3));
        assert_ne!(player(&view).position, arrived.position, "it flies");
        assert!(translucent_fade(&drawn(&view)), "still fading in");
    }

    #[test]
    fn a_hypergate_whose_gates_system_is_hidden_is_picked_by_the_one_at_its_place() {
        let mut view = FlightView::new(FakeCatalog {
            reborn: true,
            ..gated()
        });
        change_galaxy(&mut view, hiding(131));
        view.tick(TICK);
        land_now(&mut view);
        assert_eq!(view.course_map().mode(), MapMode::Hypergate);
        let reborn = on_map(&view, 133);
        click(&mut view, reborn);
        tap(&mut view, MAP);
        assert_eq!(system_of(&view), SystemId(133));
        assert_eq!(player(&view).position, Vec2::new(100.0, 200.0), "at 310");
        assert_eq!(view.scene().map(SystemScene::name), Some("Alpha Reborn"));
    }

    #[test]
    fn escape_on_the_hypergate_map_enters_the_gate_too() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        view.close_map();
        assert_eq!(system_of(&view), SystemId(131));
    }

    #[test]
    fn closing_the_hypergate_map_without_a_pick_cancels() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert_eq!(view.course_map().mode(), MapMode::Course);
        assert_eq!(system_of(&view), SystemId(130));
        assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(nav_target(&view), None);
        // The map is a course map again.
        tap(&mut view, MAP);
        assert_eq!(view.course_map().mode(), MapMode::Course);
        tap(&mut view, MAP);
        assert_eq!(view.message(), Some(HYPERGATE_CANCELLED), "nothing new");
        assert_lasts(&mut view, HYPERGATE_CANCELLED, GATE_REFUSAL_TICKS);
    }

    // Tab, `\`, Return and the map key on the in-flight maps (`_MapFilter`).

    #[test]
    fn tab_on_the_course_map_draws_the_next_link_and_selects_it() {
        let mut view = FlightView::new(FakeCatalog {
            fan: true,
            ..catalog()
        });
        tap(&mut view, MAP);
        for (k, expected) in [(Key::Tab, 131), (Key::Char('\\'), 132), (Key::Tab, 131)] {
            view.input(&key(k, true));
            assert!(view.map_open(), "{k:?}");
            assert_eq!(
                nav_mode(&view),
                NavMode::Hyperspace(Some(SystemId(expected))),
                "{k:?}"
            );
            let map = view.course_map();
            assert_eq!(map.selected(), Some(SystemId(expected)));
            assert_eq!(map.hyper_selection(), Some(SystemId(expected)));
            assert_eq!(
                hyper_selected_links(&view),
                [(on_map(&view, 130), on_map(&view, expected))]
            );
            view.input(&key(k, false));
        }
        view.input(&held(Key::Tab));
        assert_eq!(
            nav_mode(&view),
            NavMode::Hyperspace(Some(SystemId(132))),
            "a repeat cycles too"
        );
    }

    #[test]
    fn tab_on_the_course_map_with_no_links_selects_none_and_the_map_selects_the_current_system() {
        let mut view = FlightView::new(FakeCatalog {
            onward: true,
            ..catalog()
        });
        run_set(&mut view, "M132");
        view.settle_script();
        assert_eq!(system_of(&view), SystemId(132), "Barnard lists no links");
        tap(&mut view, MAP);
        tap(&mut view, Key::Tab);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
        assert_eq!(view.course_map().selected(), Some(SystemId(132)));
        assert_eq!(view.course_map().hyper_selection(), None);
        assert_eq!(hyper_selected_links(&view), []);
    }

    #[test]
    fn tab_on_the_course_map_plays_no_sound() {
        let mut view = flight();
        tap(&mut view, MAP);
        view.take_sounds();
        tap(&mut view, Key::Tab);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
        assert_eq!(view.take_sounds(), []);
    }

    #[test]
    fn tab_on_the_course_map_redraws_the_nav_area_at_once() {
        let mut view = flight();
        tap(&mut view, MAP);
        let stale = NavDisplay::Stellar(Some("Stale".to_owned()));
        let alpha = NavMode::Hyperspace(Some(SystemId(131)));
        view.nav_scene = Some((alpha, stale.clone()));
        tap(&mut view, Key::Tab);
        assert_eq!(nav_mode(&view), alpha);
        assert_eq!(view.nav_shown(), view.nav_now(), "redrawn");
        assert_ne!(view.nav_shown(), stale);
        tap(&mut view, MAP);
        assert_eq!(nav_mode(&view), alpha, "no course to cue");
        assert_eq!(nav(&view), cueing_alpha(), "shown once the map closes");
    }

    #[test]
    fn closing_the_map_cues_the_courses_hop_over_a_tab_selection() {
        let mut view = FlightView::new(FakeCatalog {
            fan: true,
            ..catalog()
        });
        plot(&mut view, 131);
        tap(&mut view, MAP);
        tap(&mut view, Key::Tab);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(132))));
        tap(&mut view, MAP);
        assert_eq!(
            nav_mode(&view),
            NavMode::Hyperspace(Some(SystemId(131))),
            "the close's cue (@0x13c16)"
        );
    }

    /// [`gated`] with Alpha Reborn standing for a hidden Alpha Centauri,
    /// and hypergate 300 linked to 310, in Alpha Centauri, and 320, in
    /// Barnard: the hypergate map is open, offering Alpha Reborn and
    /// Barnard.
    fn at_a_shadowed_gate() -> View {
        let mut view = FlightView::new(FakeCatalog {
            reborn: true,
            onward: true,
            gates: vec![
                gate_site(300, (0.0, 0.0), 0x1200, &[310, 320], 120),
                gate_site(310, (100.0, 200.0), 0x1000, &[300], 90),
                gate_site(320, (-50.0, 0.0), 0x1000, &[], 0),
            ],
            gate_listers: vec![
                (SystemId(131), StellarId(310)),
                (SystemId(132), StellarId(320)),
            ],
            ..gated()
        });
        change_galaxy(&mut view, hiding(131));
        view.tick(TICK);
        land_now(&mut view);
        assert_eq!(view.course_map().mode(), MapMode::Hypergate);
        assert_eq!(view.course_map().selected(), Some(SystemId(130)));
        view
    }

    #[test]
    fn tab_on_the_hypergate_map_cycles_the_gates_destinations_shift_reversing_through_a_shadowed_system()
     {
        let mut view = at_a_shadowed_gate();
        let nav = nav_mode(&view);
        view.take_sounds();
        // `_FindSystemFromStellar` gives hidden 131 for 310, its only
        // lister, unresolved by place.
        for expected in [131, 132, 131] {
            tap(&mut view, Key::Tab);
            assert_eq!(view.course_map().selected(), Some(SystemId(expected)));
        }
        view.input(&key(Key::Shift, true));
        for expected in [132, 131] {
            tap(&mut view, Key::Char('\\'));
            assert_eq!(view.course_map().selected(), Some(SystemId(expected)));
        }
        view.input(&key(Key::Shift, false));
        assert_eq!(nav_mode(&view), nav, "no nav mode written");
        assert_eq!(view.course_map().hyper_selection(), None);
        assert_eq!(view.take_sounds(), []);
        tap(&mut view, MAP);
        assert_eq!(
            view.message(),
            Some(HYPERGATE_CANCELLED),
            "no link resolves to hidden 131 (@0x638cd-0x638ff)"
        );
        assert_eq!(system_of(&view), SystemId(130));
    }

    #[test]
    fn closing_the_hypergate_map_on_a_tab_pick_enters_the_gate() {
        let mut view = at_a_shadowed_gate();
        tap(&mut view, Key::Tab);
        tap(&mut view, Key::Tab);
        assert_eq!(view.course_map().selected(), Some(SystemId(132)));
        view.close_map();
        assert_eq!(system_of(&view), SystemId(132));
        assert_eq!(player(&view).position, Vec2::new(-50.0, 0.0), "at 320");
    }

    #[test]
    fn tab_on_the_bars_map_changes_nothing() {
        let mut view = in_the_bar(None);
        view.open_bar_map();
        let selected = view.course_map().selected();
        for k in [Key::Tab, Key::Char('\\')] {
            view.port_map_input(&key(k, true));
            view.port_map_input(&key(k, false));
            assert!(view.map_open(), "{k:?}");
            assert_eq!(
                view.session().map(Session::nav_mode),
                Ok(NavMode::Stellar(Some(StellarId(128)))),
                "{k:?}"
            );
            assert_eq!(view.course_map().selected(), selected, "{k:?}");
            assert_eq!(view.course_map().hyper_selection(), None, "{k:?}");
        }
    }

    #[test]
    fn return_closes_the_course_map_with_the_close_beep_and_the_cue() {
        let mut view = flight();
        plot(&mut view, 131);
        tap(&mut view, NAV_OFF_KEY);
        tap(&mut view, MAP);
        view.take_sounds();
        view.input(&held(Key::Enter));
        assert!(view.map_open(), "a repeat doesn't close it");
        tap(&mut view, Key::Enter);
        assert!(!view.map_open());
        assert_eq!(map_sounds(&mut view), [UiSound::MapClosed]);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
    }

    #[test]
    fn return_on_the_hypergate_map_enters_or_cancels_in_silence() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        view.take_sounds();
        tap(&mut view, Key::Enter);
        assert!(!view.map_open());
        assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
        assert_eq!(map_sounds(&mut view), []);
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        view.take_sounds();
        tap(&mut view, Key::Enter);
        assert_eq!(system_of(&view), SystemId(131));
        assert_eq!(map_sounds(&mut view), []);
    }

    #[test]
    fn return_closes_the_bars_map_and_puts_back_the_bars_nav() {
        let mut view = in_the_bar(None);
        view.open_bar_map();
        port_click(&mut view, 131);
        view.take_sounds();
        view.port_map_input(&key(Key::Enter, true));
        assert!(!view.map_open());
        assert_eq!(map_sounds(&mut view), []);
        assert_eq!(port_nav(&view), cueing_alpha(), "the map's selection drawn");
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Stellar(Some(StellarId(128)))),
            "the bar's nav put back"
        );
    }

    #[test]
    fn a_key_down_the_map_doesnt_take_closes_it_while_the_map_key_is_held() {
        let mut view = flight();
        view.input(&key(MAP, true));
        assert!(view.map_open());
        // Every key `_MapFilter` tests before the map key's close.
        for k in [
            Key::Tab,
            Key::Char('\\'),
            Key::Space,
            Key::Char('b'),
            Key::Char('c'),
            Key::Char('-'),
            Key::Char('='),
            Key::Char('+'),
            Key::Left,
            Key::Right,
            Key::Up,
            Key::Down,
            Key::Char('f'),
            Key::Char('s'),
        ] {
            tap(&mut view, k);
            assert!(view.map_open(), "{k:?}");
        }
        view.take_sounds();
        view.input(&held(LAND));
        assert!(view.map_open(), "a repeat never closes it");
        view.input(&key(LAND, true));
        assert!(!view.map_open(), "L with M held");
        assert_eq!(map_sounds(&mut view), [UiSound::MapClosed]);
        view.input(&key(LAND, false));
        view.input(&key(MAP, false));
        tap(&mut view, MAP);
        assert!(view.map_open());
        tap(&mut view, LAND);
        assert!(view.map_open(), "M let go");
    }

    #[test]
    fn letting_go_of_the_keys_lets_go_of_the_map_key() {
        // The window loses focus with M still down, so its release never
        // arrives: the next key the map doesn't take leaves it open.
        let mut view = flight();
        view.input(&key(MAP, true));
        assert!(view.map_open());
        view.release_keys();
        tap(&mut view, LAND);
        assert!(view.map_open(), "in flight");
        let mut view = in_the_bar(None);
        view.open_bar_map();
        view.port_map_input(&held(MAP));
        view.release_keys();
        view.port_map_input(&key(Key::Tab, true));
        assert!(view.map_open(), "in port");
    }

    #[test]
    fn a_modifier_is_no_key_down_and_never_closes_the_map() {
        // A Mac sends no key-down for a modifier alone, so `_MapFilter`
        // never sees one.
        let mut view = flight();
        view.input(&key(MAP, true));
        for k in [
            Key::Shift,
            Key::Control,
            Key::Alt,
            Key::Command,
            Key::CapsLock,
        ] {
            view.input(&key(k, true));
            assert!(view.map_open(), "{k:?}");
            view.input(&key(k, false));
        }
    }

    #[test]
    fn in_port_tab_or_backslash_with_the_map_key_held_closes_the_map() {
        // Both fall through on a planet (@0x13d20).
        for k in [Key::Tab, Key::Char('\\')] {
            let mut view = in_the_bar(None);
            view.open_bar_map();
            view.port_map_input(&held(MAP));
            view.port_map_input(&key(k, true));
            assert!(!view.map_open(), "{k:?}");
        }
    }

    #[test]
    fn closing_the_map_redraws_the_radar_at_once_leaving_the_timer_and_blink() {
        // The course map, closed by its key or as Escape does; then the
        // hypergate map, cancelled and entered (`_PlayerEnterHypergate`).
        // The gate's Sol lists Earth and the Moon too, so its radar holds
        // them.
        let gated_sol = || {
            let mut gated = gated();
            gated.sites.extend(catalog().sites);
            gated
        };
        for (catalog, pick, by_key) in [
            (catalog(), false, true),
            (catalog(), false, false),
            (gated_sol(), false, true),
            (gated_sol(), true, true),
        ] {
            let gate = !catalog.gates.is_empty();
            let mut view = FlightView::new(catalog);
            ticks(&mut view, 3);
            if gate {
                land_now(&mut view);
                assert_eq!(view.course_map().mode(), MapMode::Hypergate);
            } else {
                tap(&mut view, MAP);
            }
            assert!(view.map_open());
            if pick {
                let alpha = on_map(&view, 131);
                click(&mut view, alpha);
            }
            let stale = RadarScene::default();
            assert_ne!(view.radar_now(), stale);
            view.radar_scene = Some(stale);
            let cadence = view.hud_cadence;
            if by_key {
                tap(&mut view, MAP);
            } else {
                view.close_map();
            }
            let case = format!("gate {gate}, pick {pick}, by key {by_key}");
            assert!(!view.map_open(), "{case}");
            assert_eq!(
                view.hud_cadence, cadence,
                "neither timed nor flipped: {case}"
            );
            assert_eq!(*view.radar_shown(), view.radar_now(), "{case}");
        }
    }

    #[test]
    fn without_hyperspace_effects_a_hypergate_flashes_white() {
        let mut view = FlightView::new(gated()).with_hyperspace_effects(false);
        land_now(&mut view);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        tap(&mut view, MAP);
        assert_eq!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::Flash(0.0))
        );
        view.tick(ARRIVAL_FLASH_FOR);
        assert_eq!(view.jump_effect(), None);
    }

    #[test]
    fn a_hypergate_without_links_does_nothing() {
        let mut catalog = gated();
        catalog.gates[0].links = [None; 8];
        let mut view = FlightView::new(catalog);
        tap(&mut view, LAND);
        ticks(&mut view, 1);
        let cleared = view.message().map(str::to_owned);
        tap(&mut view, LAND);
        assert!(!view.map_open());
        assert_eq!(system_of(&view), SystemId(130));
        assert_eq!(view.message(), cleared.as_deref(), "nothing said");
        assert_eq!(nav_target(&view), None);
    }

    #[test]
    fn a_hypergate_whose_links_lead_nowhere_opens_an_empty_map_that_cancels() {
        let mut catalog = gated();
        catalog.gates[0].links[0] = Some(StellarId(999));
        let mut view = FlightView::new(catalog);
        land_now(&mut view);
        assert!(view.map_open());
        assert_eq!(view.course_map().mode(), MapMode::Hypergate);
        assert_ne!(view.message(), Some(HYPERGATE_CANCELLED), "not yet");
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert_eq!(system_of(&view), SystemId(130));
        assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
    }

    #[test]
    fn l_twice_at_a_wormhole_passes_through_it_with_a_flash() {
        let mut view = FlightView::new(holed());
        land_now(&mut view);
        assert!(!view.map_open());
        assert_eq!(system_of(&view), SystemId(131));
        assert_eq!(player(&view).position, Vec2::new(-300.0, 40.0));
        assert_eq!(
            view.message(),
            Some("Passing through a wormhole into the Alpha Centauri system on June 23, 1177 NC.")
        );
        assert_eq!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::Flash(0.0))
        );
        let mut plain = FlightView::new(holed()).with_hyperspace_effects(false);
        land_now(&mut plain);
        assert_eq!(
            plain.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::Flash(0.0)),
            "whatever the preference"
        );
        let passed =
            "Passing through a wormhole into the Alpha Centauri system on June 23, 1177 NC.";
        assert_lasts(&mut plain, passed, 240);
    }

    #[test]
    fn coming_out_of_a_hypergate_says_so_for_360_ticks() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        tap(&mut view, MAP);
        let exited = "Exiting hypergate in the Alpha Centauri system on June 23, 1177 NC.";
        assert_lasts(&mut view, exited, 360);
    }

    #[test]
    fn a_wormhole_with_no_exit_says_why() {
        let mut catalog = holed();
        catalog.gates.truncate(1);
        let mut view = FlightView::new(catalog);
        land_now(&mut view);
        assert_eq!(system_of(&view), SystemId(130));
        assert_eq!(view.jump_effect(), None);
        let why =
            "Your ship is unable to enter this wormhole - the radiation levels are too extreme.";
        assert_lasts(&mut view, why, GATE_REFUSAL_TICKS);
    }

    /// The sprites of ship 129's sheet (`rlëD` 2001) drawn.
    fn ship_129_sprites(list: &DrawList) -> usize {
        sprites(list)
            .iter()
            .filter(|(image, _)| image.id == 2001)
            .count()
    }

    #[test]
    fn an_attacker_following_through_a_wormhole_is_not_drawn_while_it_emerges_but_is_on_radar() {
        let mut catalog = FakeCatalog {
            sites: holed().sites,
            gates: holed().gates,
            gate_listers: holed().gate_listers,
            ..trafficked(&[130, 131], 0, 129, 3)
        };
        catalog.dudes[0].1.govt = Some(GovtId(140));
        catalog.govts = vec![nova_sim::GovtRecord {
            id: GovtId(140),
            flags: 0,
            flags2: nova_sim::govt::ALWAYS_FOLLOWS_THROUGH_WORMHOLE,
            crime_tol: 0,
            initial_rec: 0,
            penalties: nova_sim::Penalties::default(),
            max_odds: 100,
            classes: [-1; 4],
            allies: [-1; 4],
            enemies: [-1; 4],
            comm_name: String::new(),
            scan_mask: 0,
            scan_fine: 0,
            news_pic: -1,
            require: 0,
            inh_jam: [0; 4],
        }];
        // One ship of düde 128 (ship 129) attacking the player and
        // following it (`ShipSyst` -6).
        catalog.missions = vec![MissionRecord {
            ships: MissionShip {
                count: 1,
                syst: -6,
                dude: Some(nova_sim::DudeId(128)),
                behav: 0,
                start: 0,
                ..MissionShip::NONE
            },
            ..errand(140)
        }];
        let mut view = FlightView::new(catalog).with_behaviour(Rc::new(Still));
        run_set(&mut view, "S140");
        settle_session(&mut view);
        land_now(&mut view);
        assert_eq!(system_of(&view), SystemId(131));
        view.tick(TICK);
        let session = view.session().expect("flying");
        assert!(
            session.npcs().iter().any(Npc::emerging),
            "the attacker emerges after the player"
        );
        let list = drawn(&view);
        assert_eq!(
            sprites(&list),
            [
                (ImageKey::sprite(1140, 0), at(812.0, 344.0)),
                (ImageKey::sprite(2000, 0), VIEW_CENTER),
            ],
            "Proxima and the player, not the ship emerging"
        );
        // Once the radar has refreshed, the ship is on it, still emerging.
        ticks(&mut view, 8);
        let session = view.session().expect("flying");
        assert!(session.npcs().iter().any(Npc::emerging));
        assert_eq!(blips(&view).len(), 1, "on radar while it emerges");
        while view
            .session()
            .expect("flying")
            .npcs()
            .iter()
            .any(Npc::emerging)
        {
            view.tick(TICK);
        }
        assert_eq!(ship_129_sprites(&drawn(&view)), 1, "drawn once out");
    }

    #[test]
    fn the_escort_menu_closes_once_its_only_mission_escort_is_disabled() {
        let mut catalog = trafficked(&[130], 0, 129, 3);
        // One ship of düde 128 escorting the player (`ShipBehav` 1).
        catalog.missions = vec![MissionRecord {
            ships: MissionShip {
                count: 1,
                syst: -1,
                dude: Some(nova_sim::DudeId(128)),
                behav: 1,
                start: 0,
                ..MissionShip::NONE
            },
            ..errand(140)
        }];
        let mut view = FlightView::new(catalog).with_behaviour(Rc::new(Still));
        run_set(&mut view, "S140");
        settle_session(&mut view);
        view.tick(TICK);
        let escort = view
            .session()
            .expect("flying")
            .npcs()
            .iter()
            .find(|npc| npc.mission_escort())
            .map(|npc| npc.id)
            .expect("a mission escort");
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open(), "a mission escort opens it");
        nova_sim::fixture::disable(view.session.as_mut().expect("flying"), escort);
        view.tick(TICK);
        let session = view.session().expect("flying");
        assert!(
            session.escort_menu().iter().any(|row| row.present),
            "its row stays"
        );
        assert!(!view.escort_menu().is_open(), "no escort is flying");
    }

    /// The NPCs of [`escorted_on_a_mission`].
    #[derive(Clone, Copy)]
    struct Ships {
        /// The traffic ship, ship 129.
        traffic: NpcId,
        /// The hired escort, ship 130, drawn as a crossed box.
        hired: NpcId,
        /// The mission escort, ship 129.
        mission: NpcId,
    }

    /// [`escorted`]'s flight with mission 140's one ship of düde 128
    /// (ship 129, whose sheet is 2001) escorting the player too, as
    /// `ShipBehav` 1, set up with the system, and room for a traffic ship
    /// beside it: in NPC order the mission escort, the traffic ship and
    /// the hired escort.
    fn escorted_on_a_mission() -> (View, Ships) {
        let mut catalog = boardable();
        for record in &mut catalog.ships {
            if record.id == ShipId(130) {
                record.escort_type = 2;
                record.inherent_ai = 3;
            }
        }
        // Room for a traffic ship beside the mission's.
        for (_, traffic) in &mut catalog.traffic {
            traffic.avg_ships = 2;
        }
        catalog.missions = vec![MissionRecord {
            ships: MissionShip {
                count: 1,
                syst: -1,
                dude: Some(nova_sim::DudeId(128)),
                behav: 1,
                start: 0,
                ..MissionShip::NONE
            },
            ..errand(140)
        }];
        let pilot = with_escorts(&Pilot::new(&catalog, "Escorted").expect("starts"), &[130]);
        let (_, chance) = scripted(&placed(750, 650, 0));
        let mut view = FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        run_set(&mut view, "S140");
        settle_session(&mut view);
        view.tick(TICK);
        let session = view.session().expect("flying");
        let find = |is: fn(&Npc) -> bool| {
            let found: Vec<NpcId> = session
                .npcs()
                .iter()
                .filter(|npc| is(npc))
                .map(|npc| npc.id)
                .collect();
            assert_eq!(found.len(), 1, "{:?}", session.npcs());
            found[0]
        };
        let ships = Ships {
            traffic: find(|npc| !npc.escorts_player()),
            hired: find(|npc| npc.escort.is_some()),
            mission: find(Npc::mission_escort),
        };
        assert_eq!(session.npcs().len(), 3);
        let order: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(
            order,
            [ships.mission, ships.traffic, ships.hired],
            "the traffic ship between the escorts in NPC order"
        );
        (view, ships)
    }

    /// Where on screen `view` draws NPC `id`.
    fn npc_on_screen(view: &View, id: NpcId) -> Point {
        let session = view.session().expect("flying");
        let npc = session
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("in the system");
        view.camera().world_to_screen(view.shown_npc(npc).0)
    }

    /// The index in `list` of the first sprite centred on `at`, or of a
    /// crossed box's diagonal centred on it.
    fn drawn_at(list: &DrawList, at: Point) -> usize {
        let near = |p: Point| (p.x - at.x).abs() < 0.01 && (p.y - at.y).abs() < 0.01;
        list.iter()
            .position(|command| match *command {
                DrawCommand::Sprite { center, .. } => near(center),
                DrawCommand::Line { from, to, .. } => near(Point::new(
                    f32::midpoint(from.x, to.x),
                    f32::midpoint(from.y, to.y),
                )),
                _ => false,
            })
            .unwrap_or_else(|| panic!("nothing drawn at {at:?}"))
    }

    /// The index in `list` of the player's sprite (sheet 2000).
    fn player_drawn(list: &DrawList) -> usize {
        list.iter()
            .position(
                |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 2000),
            )
            .expect("the player drawn")
    }

    /// Where `view` draws its traffic ship, its hired escort, its mission
    /// escort and the player, as indices into what it draws.
    fn ship_indices(view: &View, ships: Ships) -> [usize; 4] {
        let list = drawn(view);
        let at = |id| drawn_at(&list, npc_on_screen(view, id));
        let ats = [ships.traffic, ships.hired, ships.mission].map(|id| npc_on_screen(view, id));
        assert!(
            ats.iter().enumerate().all(|(i, a)| ats[i + 1..]
                .iter()
                .all(|b| (a.x - b.x).abs() > 1.0 || (a.y - b.y).abs() > 1.0)),
            "each ship apart: {ats:?}"
        );
        [
            at(ships.traffic),
            at(ships.hired),
            at(ships.mission),
            player_drawn(&list),
        ]
    }

    #[test]
    fn a_hired_and_a_mission_escort_draw_in_the_escorts_layer() {
        let (view, ships) = escorted_on_a_mission();
        let session = view.session().expect("flying");
        let berth = |id| {
            let npc = session.npcs().iter().find(|npc| npc.id == id);
            npc.expect("here").berth
        };
        assert!(berth(ships.hired) < berth(ships.mission));
        let [traffic, hired, led, player] = ship_indices(&view, ships);
        assert!(
            hired < led,
            "the escorts in the order they entered their layer, by berth: {hired} {led}"
        );
        assert!(led < traffic, "under the other ships: {led} {traffic}");
        assert!(traffic < player, "the player on top: {traffic} {player}");
    }

    #[test]
    fn a_disabled_ship_draws_under_the_escorts() {
        let (mut view, ships) = escorted_on_a_mission();
        let session = view.session.as_mut().expect("flying");
        nova_sim::fixture::disable(session, ships.traffic);
        let [traffic, hired, led, player] = ship_indices(&view, ships);
        assert!(traffic < hired, "{traffic} {hired}");
        assert!(hired < led && led < player, "{hired} {led} {player}");
    }

    #[test]
    fn cloak_alpha_maps_each_shown_state_to_a_tint_alpha() {
        assert_eq!(cloak_alpha(CloakShown::Opaque), Some(255));
        assert_eq!(cloak_alpha(CloakShown::Translucent(0x1e)), Some(135));
        assert_eq!(cloak_alpha(CloakShown::Translucent(0)), Some(255));
        assert_eq!(
            cloak_alpha(CloakShown::Translucent(0x20)),
            Some(128),
            "127.5, rounded to nearest"
        );
        assert_eq!(cloak_alpha(CloakShown::Hidden), None);
    }

    #[test]
    fn a_fully_cloaked_escort_is_drawn_at_its_translucent_alpha() {
        let (mut view, ships) = escorted_on_a_mission();
        let session = view.session.as_mut().expect("flying");
        nova_sim::fixture::cloak(session, ships.mission, CLOAKED);
        let list = drawn(&view);
        let at = npc_on_screen(&view, ships.mission);
        let tint = list.iter().find_map(|command| match *command {
            DrawCommand::Sprite {
                image,
                center,
                tint,
                ..
            } if image.id == 2001 && center == at => Some(tint),
            _ => None,
        });
        assert_eq!(tint.map(|tint| tint.a), Some(135));
    }

    // The radar's IFF, density scanner, centre arrow and static.

    /// IFF, a density scanner and anti-interference 30 as outfits 300-302.
    fn radar_outfits() -> Vec<OutfitRecord> {
        let outfit = |id: i16, mod_type: i16, mod_val: i16| {
            let mut mods = [(0, 0); 4];
            mods[1] = (mod_type, mod_val);
            OutfitRecord {
                id: OutfitId(id),
                name: format!("Outfit {id}"),
                short_name: String::new(),
                disp_weight: 0,
                mass: 0,
                tech_level: 1,
                max: 10,
                flags: 0,
                cost: 1,
                mods,
                contribute: 0,
                require: 0,
                require_govt: -1,
                buy_random: 100,
                availability: nova_sim::Test::default(),
                on_purchase: nova_sim::Script::default(),
                on_sell: nova_sim::Script::default(),
                item_class: 0,
                lc_name: String::new(),
                lc_plural: String::new(),
                scan_mask: 0,
            }
        };
        vec![
            outfit(300, nova_sim::iff::MOD_IFF, 0),
            outfit(301, nova_sim::iff::MOD_DENSITY_SCANNER, 0),
            outfit(302, nova_sim::iff::MOD_ANTI_INTERFERENCE, 30),
        ]
    }

    /// `catalog`, its ship carrying `defaults` of [`radar_outfits`].
    fn outfitted(catalog: FakeCatalog, defaults: &[(i16, u16)]) -> FakeCatalog {
        FakeCatalog {
            outfits: radar_outfits(),
            defaults: defaults.iter().map(|&(id, n)| (OutfitId(id), n)).collect(),
            ..catalog
        }
    }

    #[test]
    fn an_iff_outfit_turns_on_the_black_backdrop_and_the_cyan_player() {
        let mut view = FlightView::new(outfitted(catalog(), &[(300, 1)]));
        ticks(&mut view, 1);
        assert!(view.radar_shown().iff);
        let marks = radar_marks(&view);
        assert_eq!(
            marks[0],
            RadarMark::Fill {
                rect: STOCK_RADAR,
                color: RadarColor::Black
            }
        );
        assert!(marks.contains(&RadarMark::Pixel {
            x: 926,
            y: 96,
            color: RadarColor::Cyan
        }));
        // Earth and the Moon, landable to a clean record: yellow.
        let stellars: Vec<StellarIff> = view.radar_shown().stellars.iter().map(|s| s.iff).collect();
        assert_eq!(stellars, [StellarIff::Cleared, StellarIff::Cleared]);
        let list = drawn(&view);
        let black = list.iter().any(|command| {
            matches!(command, DrawCommand::Line { color, .. } if *color == Color::rgba(0, 0, 0, 255))
        });
        assert!(black, "the black fill is drawn");
        for without in [outfitted(catalog(), &[]), outfitted(catalog(), &[(301, 1)])] {
            let mut view = FlightView::new(without);
            ticks(&mut view, 1);
            assert!(!view.radar_shown().iff);
            assert_eq!(radar_marks(&view).last(), Some(&PLAYER_PIXEL));
            assert!(
                !radar_marks(&view)
                    .iter()
                    .any(|m| matches!(m, RadarMark::Fill { .. }))
            );
        }
    }

    #[test]
    fn a_stellar_of_a_landing_blessed_government_is_yellow_on_the_radar() {
        // `_ColorCodeStellar` @0xaab9: the government's landing blessing
        // gives yellow whatever `MinStatus` says.
        let earth = |on_start: &str| {
            let base = outfitted(starting_with(on_start), &[(300, 1)]);
            let mut sites = base.sites.clone();
            sites[0].govt = Some(GovtId(140));
            sites[0].min_status = 10;
            let catalog = FakeCatalog {
                ranks: vec![nova_sim::catalog::RankRecord {
                    id: nova_sim::RankId(200),
                    name: String::new(),
                    weight: 0,
                    affil_govt: Some(GovtId(140)),
                    salary: 0,
                    salary_cap: 0,
                    contribute: 0,
                    flags: nova_sim::rank::BLESSES_LANDING,
                    conv_name: String::new(),
                    short_name: String::new(),
                    price_mod: 100,
                }],
                sites,
                ..base
            };
            let pilot = Pilot::new(&catalog, "Ada").expect("starts");
            let mut view = FlightView::with_pilot(catalog, pilot);
            // Begun, the ship sits at (50, 50), so Earth is drawn up and
            // to the left of the radar's centre.
            view.begin();
            ticks(&mut view, 1);
            radar_marks(&view)
                .into_iter()
                .find(|mark| matches!(mark, RadarMark::Oval { rect, .. } if *rect == IRect::around(924, 75, 1)))
        };
        let oval = |color| RadarMark::Oval {
            rect: IRect::around(924, 75, 1),
            color,
        };
        assert_eq!(earth("K200"), Some(oval(RadarColor::Yellow)));
        assert_eq!(earth(""), Some(oval(RadarColor::Orange)), "unblessed");
    }

    /// [`outfitting`] selling [`radar_outfits`] too, landed.
    fn landed_radar_outfitter() -> View {
        let mut catalog = outfitting();
        catalog.outfits.extend(radar_outfits());
        let mut view = FlightView::new(catalog);
        land_now(&mut view);
        view
    }

    /// Whether the radar shown has IFF's black fill.
    fn radar_filled(view: &View) -> bool {
        radar_marks(view).first()
            == Some(&RadarMark::Fill {
                rect: STOCK_RADAR,
                color: RadarColor::Black,
            })
    }

    #[test]
    fn an_outfitter_order_redraws_the_radar_at_once_while_docked() {
        type Place = fn(&mut View, OutfitOrder);
        let order = |direction| OutfitOrder {
            outfit: OutfitId(300),
            direction,
        };
        let entries: [(&str, Place); 3] = [
            ("outfit", |view, order| {
                assert_eq!(view.outfit(order), Ok(()));
            }),
            ("counted", |view, order| {
                assert_eq!(view.outfit_counted(order, 1), Ok(1));
            }),
            ("order", |view, order| {
                assert_eq!(view.outfit_order(order, 1).run.done, 1);
            }),
        ];
        for (name, place) in entries {
            let mut view = landed_radar_outfitter();
            assert!(view.hud_waits(), "the engine's reading");
            assert!(!view.radar_shown().iff, "{name}: landing's");
            place(&mut view, order(Direction::Buy));
            assert!(view.radar_shown().iff, "{name}: bought");
            assert!(radar_filled(&view), "{name}: bought");
            assert!(radar_marks(&view).contains(&RadarMark::Pixel {
                x: 926,
                y: 96,
                color: RadarColor::Cyan
            }));
            place(&mut view, order(Direction::Sell));
            assert!(!view.radar_shown().iff, "{name}: sold");
            assert!(!radar_filled(&view), "{name}: sold");
            assert_eq!(radar_marks(&view).last(), Some(&PLAYER_PIXEL), "{name}");
        }
    }

    /// A trafficked flight with one idle NPC of ship 129, whose class
    /// weighs `mass`, at (100, -100), radar pixel (929, 92); the player
    /// carries `defaults`.
    fn with_heavy_npc(mass: i16, defaults: &[(i16, u16)]) -> View {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let catalog = FakeCatalog {
            hulls: vec![nova_sim::HullRecord {
                mass,
                ..hull_of(129, &[])
            }],
            ..outfitted(trafficked(&[130], 1, 129, 3), defaults)
        };
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        view
    }

    #[test]
    fn a_heavy_npc_shows_as_a_square_only_with_a_density_scanner() {
        let square = RadarMark::Square {
            rect: IRect::around(929, 92, 1),
            color: RadarColor::Dim,
        };
        let dot = RadarMark::Pixel {
            x: 929,
            y: 92,
            color: RadarColor::Dim,
        };
        // The ships' marks: squares and pixels but the player's, last.
        let ships = |view: &View| {
            let mut marks: Vec<RadarMark> = radar_marks(view)
                .into_iter()
                .filter(|mark| matches!(mark, RadarMark::Pixel { .. } | RadarMark::Square { .. }))
                .collect();
            assert_eq!(marks.pop(), Some(PLAYER_PIXEL));
            marks
        };
        assert_eq!(ships(&with_heavy_npc(100, &[(301, 1)])), [square]);
        assert_eq!(ships(&with_heavy_npc(100, &[])), [dot]);
        assert_eq!(ships(&with_heavy_npc(99, &[(301, 1)])), [dot]);
    }

    #[test]
    fn a_ships_iff_code_reaches_the_radar() {
        let view = with_heavy_npc(0, &[(300, 1)]);
        let ships = &view.radar_shown().ships;
        assert_eq!(ships.len(), 1);
        assert_eq!(ships[0].iff, ShipIff::Other);
        assert!(!ships[0].heavy);
        assert!(radar_marks(&view).contains(&RadarMark::Pixel {
            x: 929,
            y: 92,
            color: RadarColor::Blue
        }));
    }

    #[test]
    fn a_plotted_course_hides_the_arrow_until_a_stellar_is_selected() {
        let mut view = flight();
        assert!(!view.radar_now().hyperspace_mode, "no course");
        plot(&mut view, 131);
        assert!(view.radar_now().hyperspace_mode);
        assert!(
            view.radar_shown().hyperspace_mode,
            "redrawn as the map closes"
        );
        tap(&mut view, NEAREST_STELLAR_KEY);
        assert!(view.session().expect("flying").nav_target().is_some());
        assert!(!view.radar_now().hyperspace_mode, "a stellar selected");
    }

    /// Thrusts straight up until the ship is more than 2700 from the
    /// centre, then lets go of Up.
    fn fly_far(view: &mut View) {
        view.input(&key(Key::Up, true));
        for _ in 0..5000 {
            if player(view).position.length() > 2700.0 {
                break;
            }
            view.tick(TICK);
        }
        view.input(&key(Key::Up, false));
        assert!(player(view).position.length() > 2700.0);
    }

    fn arrow_lines(view: &View) -> usize {
        radar_marks(view)
            .iter()
            .filter(|mark| matches!(mark, RadarMark::Line { .. }))
            .count()
    }

    #[test]
    fn far_from_the_centre_the_arrow_blinks_with_the_target() {
        let mut view = flight();
        assert_eq!(arrow_lines(&view), 0, "near the centre");
        fly_far(&mut view);
        let mut shown = Vec::new();
        for _ in 0..32 {
            ticks(&mut view, 1);
            shown.push((view.hud_cadence.blink_on(), arrow_lines(&view)));
        }
        for &(blink_on, lines) in &shown {
            assert_eq!(lines, if blink_on { 3 } else { 0 }, "{shown:?}");
        }
        let flips = shown
            .windows(2)
            .filter(|pair| pair[0].0 != pair[1].0)
            .count();
        assert_eq!(flips, 4, "every 8 ticks: {shown:?}");
    }

    /// `catalog` with Sol's `Interference` at `level`, its effects rolled
    /// on `draws`.
    fn interfered(level: i16, draws: &[u32]) -> (Rc<RefCell<Script>>, View) {
        let (script, chance) = scripted(draws);
        let catalog = FakeCatalog {
            sol_interference: level,
            ..catalog()
        };
        (script, FlightView::new(catalog).with_effects_chance(chance))
    }

    #[test]
    fn interference_shows_static_at_the_next_refresh() {
        // The threshold roll 0 (1 ≤ 100), then pattern 3.
        let (script, mut view) = interfered(100, &[0, 3]);
        assert_eq!(view.radar_shown().static_pattern, None, "before a tick");
        ticks(&mut view, 1);
        assert_eq!(script.borrow().asked, [100, 10]);
        assert_eq!(view.radar_shown().static_pattern, Some(131));
        assert_eq!(
            radar_marks(&view)[0],
            RadarMark::Pattern {
                rect: STOCK_RADAR,
                id: 131
            }
        );
        let list = drawn(&view);
        assert!(list.iter().any(|command| *command
            == DrawCommand::TiledPicture {
                image: ImageKey::pattern(131),
                area: STOCK_RADAR.to_bounds(),
            }));
    }

    #[test]
    fn without_interference_no_static_shows_and_nothing_is_rolled() {
        let (script, mut view) = interfered(0, &[]);
        for _ in 0..20 {
            ticks(&mut view, 1);
            assert_eq!(view.radar_shown().static_pattern, None);
        }
        assert!(script.borrow().asked.is_empty());
    }

    #[test]
    fn anti_interference_lowers_the_level_static_is_rolled_against() {
        // 100 less 30: a roll of 70 (71 > 70) shows none.
        let (script, chance) = scripted(&[70]);
        let catalog = FakeCatalog {
            sol_interference: 100,
            ..outfitted(catalog(), &[(302, 1)])
        };
        let mut view = FlightView::new(catalog).with_effects_chance(chance);
        ticks(&mut view, 1);
        assert_eq!(script.borrow().asked, [100]);
        assert_eq!(view.radar_shown().static_pattern, None);
    }

    #[test]
    fn the_hud_cadence_and_blink_advance_through_a_jumps_streak() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let streaking = |view: &View| {
            matches!(
                view.jump_effect().map(JumpEffect::phase),
                Some(JumpPhase::Streak(_))
            )
        };
        assert!(streaking(&view));
        // Mark the snapshot, then tick through the streak: the radar is
        // taken afresh, and the blink flips, every 8 ticks.
        let marker = RadarScene {
            player: at(-1.0, -1.0),
            ..RadarScene::default()
        };
        let mut refreshes = Vec::new();
        let mut tick = 0;
        while streaking(&view) {
            view.radar_scene = Some(marker.clone());
            let blink = view.hud_cadence.blink_on();
            view.tick(TICK);
            tick += 1;
            let retaken = view.radar_scene.as_ref() != Some(&marker);
            assert_eq!(retaken, view.hud_cadence.blink_on() != blink, "tick {tick}");
            if retaken {
                refreshes.push(tick);
            }
        }
        assert!(tick >= 24, "the streak lasts a second: {tick}");
        assert!(refreshes.len() >= 3, "{refreshes:?}");
        let gaps: Vec<u32> = refreshes.windows(2).map(|pair| pair[1] - pair[0]).collect();
        assert!(gaps.iter().all(|&gap| gap == 8), "{refreshes:?}");
    }

    // The Red Alert.

    /// Quiet sound preferences: effects at level 1.
    const QUIET: SoundPrefs = SoundPrefs {
        sound: true,
        music: true,
        effects_level: 1,
        music_level: 7,
    };

    /// Two NPCs, NPC 0 turning on the player at the first tick after the
    /// flight's first (frame 0), sound `prefs`: the Red Alert's check at
    /// frame 60 is the 60th tick from here.
    fn threatened(prefs: SoundPrefs) -> View {
        fighting(armed(2, &[]), &two_npcs())
            .with_behaviour(Rc::new(Threatening))
            .with_sound_prefs(prefs)
    }

    /// The Red Alert `cïcn`s drawn, and where.
    fn icons_drawn(view: &View) -> Vec<(i16, Point)> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Picture { image, top_left } if icon_in(18000..=18001)(command) => {
                    Some((image.id, *top_left))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_red_alert_draws_cicn_18000_then_18001_at_the_playfields_top_right() {
        let mut view = threatened(QUIET);
        for tick in 1..60 {
            view.tick(TICK);
            assert_eq!(icons_drawn(&view), [], "tick {tick}: before the check");
        }
        let corner = Point::new(1024.0 - 194.0 - 24.0, 0.0);
        for tick in 0..160_usize {
            view.tick(TICK);
            let expected = match tick {
                146.. => vec![],
                1.. if ((tick - 1) / 5).is_multiple_of(2) => vec![(18001, corner)],
                _ => vec![(18000, corner)],
            };
            assert_eq!(icons_drawn(&view), expected, "tick {tick} of the blink");
        }
    }

    #[test]
    fn no_alert_icon_is_drawn_for_a_loud_player() {
        let mut view = threatened(SoundPrefs::default());
        for tick in 0..300 {
            view.tick(TICK);
            assert_eq!(icons_drawn(&view), [], "tick {tick}");
        }
        assert!(!view.alert_quiet());
    }

    #[test]
    fn the_sound_prefs_reach_the_session_when_set() {
        let mut view = threatened(SoundPrefs::default());
        view.set_sound_prefs(QUIET);
        assert!(view.alert_quiet());
        ticks(&mut view, 60);
        assert_eq!(icons_drawn(&view).len(), 1);
        view.set_sound_prefs(SoundPrefs {
            sound: false,
            ..SoundPrefs::default()
        });
        assert!(view.alert_quiet(), "sound off is quiet");
        view.set_sound_prefs(SoundPrefs::default());
        assert!(!view.alert_quiet());
    }

    #[test]
    fn the_red_alert_sounds_once_through_the_view() {
        let mut view = threatened(QUIET);
        view.take_sounds();
        let mut alerts = 0;
        for _ in 0..300 {
            view.tick(TICK);
            alerts += view
                .take_sounds()
                .iter()
                .filter(|&&sound| sound == Sound::Sim(SimSound::RedAlert))
                .count();
        }
        assert_eq!(alerts, 1);
    }

    #[test]
    fn the_alert_icon_is_drawn_over_the_brackets() {
        let mut view = threatened(QUIET);
        ticks(&mut view, 1);
        tap(&mut view, NEAREST_KEY);
        assert_eq!(target_of(&view), Some(NpcId(0)));
        ticks(&mut view, 59);
        let list = drawn(&view);
        let commands: Vec<_> = list.iter().collect();
        let icon = commands
            .iter()
            .position(|command| icon_in(18000..=18001)(command))
            .expect("the icon is drawn");
        assert!(
            icon_in(10011..=10011)(commands[icon - 1]),
            "the brackets' bottom-left corner just before it: {:?}",
            commands[icon - 1]
        );
    }

    #[test]
    fn a_missing_alert_icon_draws_nothing_and_says_so_once() {
        let catalog = FakeCatalog {
            alert_icon: None,
            ..armed(2, &[])
        };
        let (_, chance) = scripted(&two_npcs());
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Threatening))
            .with_sound_prefs(QUIET);
        let missing =
            Diagnostic::Unreadable("no cïcn 18000: the Red Alert icon is not drawn".to_owned());
        let told = |view: &mut View| {
            let diagnostics = view.take_diagnostics();
            diagnostics.iter().filter(|&d| *d == missing).count()
        };
        assert_eq!(told(&mut view), 1);
        let mut blinked = false;
        for _ in 0..200 {
            view.tick(TICK);
            blinked |= view.session().expect("flying").red_alert_icon().is_some();
            assert_eq!(icons_drawn(&view), []);
        }
        assert!(blinked, "the session blinked it");
        assert_eq!(told(&mut view), 0, "once");
        assert_eq!(told(&mut flight()), 0, "the default catalog has it");
    }

    // Double time.

    /// Whether `view` draws the double-time icon, `cïcn` 20000 at the top
    /// left.
    fn double_time_drawn(view: &View) -> bool {
        drawn(view).iter().any(|command| {
            *command
                == DrawCommand::Picture {
                    image: ImageKey::icon(20000),
                    top_left: Point::new(0.0, 0.0),
                }
        })
    }

    #[test]
    fn double_time_shows_its_icon_at_the_top_left_over_the_red_alert() {
        let mut view = threatened(QUIET).with_double_time(true);
        assert!(view.double_time());
        ticks(&mut view, 60);
        let list = drawn(&view);
        let commands: Vec<_> = list.iter().collect();
        let icon = commands
            .iter()
            .position(|command| icon_in(20000..=20000)(command))
            .expect("the icon is drawn");
        assert_eq!(
            *commands[icon],
            DrawCommand::Picture {
                image: ImageKey::icon(20000),
                top_left: Point::new(0.0, 0.0),
            }
        );
        assert!(
            icon_in(18000..=18001)(commands[icon - 1]),
            "just after the Red Alert's: {:?}",
            commands[icon - 1]
        );
        view.set_double_time(false);
        assert!(!view.double_time());
        assert!(!double_time_drawn(&view), "off at once");
        view.tick(TICK);
        assert!(!double_time_drawn(&view));
        assert!(!double_time_drawn(&flight()), "off to begin with");
    }

    #[test]
    fn double_time_runs_a_second_pass_each_tick() {
        for (on, steps) in [(false, 3), (true, 6)] {
            let mut view = flight().with_double_time(on);
            view.input(&key(Key::Up, true));
            ticks(&mut view, 3);
            assert_eq!(player(&view), stepped(start(), THRUST, steps), "{on}");
        }
    }

    #[test]
    fn double_time_reaches_the_session_on_the_next_tick_and_when_set() {
        let mut view = flight();
        view.set_double_time(true);
        assert!(view.session().expect("flying").double_time(), "when set");
        view.session
            .as_mut()
            .expect("flying")
            .set_double_time_key(false);
        view.tick(TICK);
        assert!(view.session().expect("flying").double_time(), "each tick");
    }

    #[test]
    fn double_time_survives_the_events_that_let_go_of_the_flight_keys() {
        let mut view = flight().with_double_time(true);
        tap(&mut view, MAP);
        tap(&mut view, MAP);
        view.tick(TICK);
        assert!(double_time_drawn(&view), "the map");
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        arrive_now(&mut view);
        view.tick(TICK);
        assert!(double_time_drawn(&view), "a jump");
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
        let mut hailed = hailable(true).with_double_time(true);
        tap(&mut hailed, HAIL_KEY);
        assert!(hailed.take_hail().is_some());
        hailed.tick(TICK);
        assert!(double_time_drawn(&hailed), "a hail");
    }

    /// [`beside`] (intact) with the player holding 95 of fuel and NPC 0
    /// refuelling it, hailed and hung up.
    fn refuelling() -> View {
        let catalog = boardable();
        let pilot = Pilot::new(&catalog, "Low").expect("starts");
        let mut save: serde_json::Value =
            serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
        save["reserves"]["fuel"]["now"] = serde_json::json!(95.0);
        let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
        let (_, chance) = scripted(&placed(750, 750, 0));
        let mut view = FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_hail_options(HailOptions::empty().with(Rc::new(Refuel)));
        view.catalog.strings = vec![(3000, comm_strings())];
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        tap(&mut view, HAIL_KEY);
        view.take_hail().expect("hailed");
        view.answer(0).expect("open");
        view.hang_up();
        view
    }

    #[test]
    fn double_times_second_pass_gives_no_help_and_tells_no_comm() {
        for on in [false, true] {
            let mut view = refuelling().with_double_time(on);
            let mut ticks = 0;
            while view.message().is_none() {
                assert!(ticks < 20, "never refuelled: {on}");
                view.tick(TICK);
                ticks += 1;
            }
            // A tick of help on the advance pass alone: 95 to past 100,
            // one at a time.
            assert_eq!(ticks, 6, "{on}");
            assert_eq!(reserves(&view).fuel.now, 101.0, "{on}");
            let note = view.message().expect("refuelled").to_owned();
            assert!(note.ends_with(":  Energy transfer complete."), "{note}");
        }
    }

    #[test]
    fn double_time_ages_the_effects_on_both_passes() {
        let detonated = || {
            let mut view = fighting(detonating(), &[]);
            detonate(&mut view);
            view
        };
        let mut twice = detonated();
        ticks(&mut twice, 2);
        let mut doubled = detonated().with_double_time(true);
        doubled.tick(TICK);
        assert_eq!(doubled.effects().explosions()[0].frame, 2.0);
        assert_eq!(doubled.effects(), twice.effects());
    }

    #[test]
    fn a_shot_detonating_on_the_second_pass_is_drawn_that_tick() {
        // Its one move, on the advance pass, spends it; its end of life
        // comes in the next pass's hit pass.
        let lasting_one = || fighting(detonating(), &[]);
        let mut single = lasting_one();
        single.input(&key(FIRE_KEY, true));
        single.tick(TICK);
        assert!(single.effects().explosions().is_empty(), "not yet");
        let mut view = lasting_one().with_double_time(true);
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        assert_eq!(view.effects().explosions().len(), 1);
        first(&drawn(&view), sprite_of(400));
    }

    #[test]
    fn caps_lock_is_no_flight_key() {
        let mut view = flight();
        view.input(&key(Key::CapsLock, true));
        assert!(view.held.is_empty());
        ticks(&mut view, 3);
        assert_eq!(player(&view), start());
        assert!(!view.double_time(), "the app toggles it, not flight");
    }

    // The hover cursor (`_HandleMouse`).

    /// The cursor sprites in `list`: frame and centre.
    fn cursor_sprites(list: &DrawList) -> Vec<(u16, Point)> {
        sprites(list)
            .into_iter()
            .filter(|(image, _)| image.id == 651)
            .map(|(image, at)| (image.frame, at))
            .collect()
    }

    /// A flight with one still NPC, drawn after a tick, and its id.
    fn flight_with_npc(catalog: FakeCatalog) -> (View, NpcId) {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let id = npc_ids(&view)[0];
        (view, id)
    }

    /// A point on screen over nothing at all.
    const NOWHERE: Point = Point::new(40.0, 400.0);

    #[test]
    fn a_left_click_on_a_drawn_ship_targets_it_with_the_target_beep() {
        let (mut view, id) = flight_with_npc(trafficked(&[130], 1, 129, 3));
        let over = npc_on_screen(&view, id);
        assert!(over.x < VIEW_SIZE.0 - STATUS_BAR_WIDTH, "{over:?} in space");
        view.take_sounds();
        click(&mut view, over);
        assert_eq!(
            view.session().expect("flying").target().map(|npc| npc.id),
            Some(id)
        );
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TargetSelected)]);
    }

    #[test]
    fn a_click_on_the_players_own_ship_lets_go_of_the_target_with_the_clear_beep() {
        let (mut view, id) = flight_with_npc(trafficked(&[130], 1, 129, 3));
        let over = npc_on_screen(&view, id);
        assert!(
            (over.x - VIEW_CENTER.x).abs() > 64.0 || (over.y - VIEW_CENTER.y).abs() > 64.0,
            "the NPC at {over:?} sits clear of the player's ship"
        );
        tap(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), Some(id));
        view.take_sounds();
        // The camera centres on the player, so its ship is drawn at the centre.
        click(&mut view, VIEW_CENTER);
        assert_eq!(target_of(&view), None);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TargetCleared)]);
    }

    #[test]
    fn a_click_on_the_players_own_ship_with_no_target_is_silent() {
        let (mut view, _) = flight_with_npc(trafficked(&[130], 1, 129, 3));
        view.take_sounds();
        click(&mut view, VIEW_CENTER);
        assert_eq!(target_of(&view), None);
        assert_eq!(view.take_sounds(), []);
    }

    #[test]
    fn a_click_on_a_ship_targets_it_during_a_jump() {
        // A still NPC at (0, -750), where the ship flies out past.
        let mut catalog = trafficked(&[130], 1, 129, 3);
        for record in &mut catalog.ships {
            if record.id == ShipId(129) {
                record.fields.speed = 0;
            }
        }
        let (_, chance) = scripted(&placed(750, 0, 0));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let id = npc_ids(&view)[0];
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        assert!(view.session().expect("flying").preparing_jump().is_some());
        let over = npc_on_screen(&view, id);
        assert!(
            (0.0..VIEW_SIZE.0 - STATUS_BAR_WIDTH).contains(&over.x)
                && (0.0..VIEW_SIZE.1).contains(&over.y),
            "{over:?} in space"
        );
        view.take_sounds();
        click(&mut view, over);
        assert_eq!(
            view.session().expect("flying").target().map(|npc| npc.id),
            Some(id)
        );
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TargetSelected)]);
    }

    #[test]
    fn the_cursor_animates_over_an_npc_and_rests_off_it() {
        let (mut view, id) = flight_with_npc(trafficked(&[130], 1, 129, 3));
        let over = npc_on_screen(&view, id);
        view.input(&Input::PointerMoved(over));
        assert_eq!(cursor_sprites(&drawn(&view)), [(0, over)]);
        ticks(&mut view, 2);
        assert_eq!(cursor_sprites(&drawn(&view)), [(1, over)]);
        view.input(&Input::PointerMoved(NOWHERE));
        ticks(&mut view, 1);
        assert_eq!(cursor_sprites(&drawn(&view)), [(0, NOWHERE)]);
        ticks(&mut view, 4);
        assert_eq!(cursor_sprites(&drawn(&view)), [(0, NOWHERE)], "rests");
    }

    #[test]
    fn a_button_press_shows_the_cursor_where_it_is() {
        let mut view = flight();
        assert_eq!(cursor_sprites(&drawn(&view)), [], "no pointer yet");
        view.input(&Input::PointerButton {
            button: MouseButton::Right,
            pressed: true,
            at: NOWHERE,
        });
        assert_eq!(cursor_sprites(&drawn(&view)), [(0, NOWHERE)]);
    }

    #[test]
    fn the_cursor_draws_over_the_hud_and_under_the_jump_fade() {
        let cursor_at = |list: &DrawList| {
            list.iter()
                .position(
                    |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 651),
                )
                .expect("drawn")
        };
        let mut view = flight();
        view.input(&Input::PointerMoved(NOWHERE));
        let list = drawn(&view);
        let cursor = cursor_at(&list);
        assert!(cursor > hud_at(&list), "over the HUD");
        assert_eq!(cursor, list.len() - 1, "last, with no fade");

        let mut view = fading_in_on_course_to_barnard();
        view.input(&Input::PointerMoved(NOWHERE));
        let list = drawn(&view);
        let (at_fade, _) = fade(&list).expect("fading in");
        assert!(cursor_at(&list) < at_fade, "under the fade");
    }

    #[test]
    fn a_cloaked_npc_is_hovered_only_with_a_scanner_that_targets_cloaked_ships() {
        let cloaking = |scanned: bool| {
            let mut scanner = radar_outfits();
            let mut mods = [(0, 0); 4];
            mods[1] = (nova_sim::cloak::MOD_SCANNER, nova_sim::cloak::SCAN_TARGET);
            scanner[0].mods = mods;
            let catalog = FakeCatalog {
                outfits: scanner,
                defaults: if scanned {
                    vec![(OutfitId(300), 1)]
                } else {
                    Vec::new()
                },
                ..trafficked(&[130], 1, 129, 3)
            };
            let (mut view, id) = flight_with_npc(catalog);
            let session = view.session.as_mut().expect("flying");
            nova_sim::fixture::cloak(session, id, CLOAKED);
            let session = view.session().expect("flying");
            assert!(!session.scanner().on_screen, "cannot see it");
            assert_eq!(session.scanner().targets, scanned);
            let over = npc_on_screen(&view, id);
            view.input(&Input::PointerMoved(over));
            ticks(&mut view, 2);
            let frames: Vec<u16> = cursor_sprites(&drawn(&view))
                .into_iter()
                .map(|(frame, _)| frame)
                .collect();
            frames
        };
        assert_eq!(cloaking(false), [0], "rests");
        assert_eq!(
            cloaking(true),
            [1],
            "animates over the ship it does not draw"
        );
    }

    #[test]
    fn the_players_own_ship_is_hovered_only_while_it_holds_a_target() {
        let (mut view, _) = flight_with_npc(trafficked(&[130], 1, 129, 3));
        let frame = |view: &mut View| {
            view.input(&Input::PointerMoved(VIEW_CENTER));
            ticks(view, 2);
            cursor_sprites(&drawn(view))
        };
        assert_eq!(frame(&mut view), [(0, VIEW_CENTER)], "no target");
        view.input(&key(TARGET_KEY, true));
        view.input(&key(TARGET_KEY, false));
        assert!(view.session().expect("flying").target().is_some());
        assert_eq!(frame(&mut view), [(1, VIEW_CENTER)], "a target");
    }

    #[test]
    fn an_npc_of_a_cost_minus_one_type_is_not_drawn_but_is_still_hovered() {
        let (mut view, id) = flight_with_npc(trafficked(&[130], 1, 129, 3));
        let over = npc_on_screen(&view, id);
        let near = |list: &DrawList| {
            list.iter().any(|command| match *command {
                DrawCommand::Sprite { center, image, .. } => {
                    image.id != 651
                        && (center.x - over.x).abs() < 0.01
                        && (center.y - over.y).abs() < 0.01
                }
                _ => false,
            })
        };
        assert!(near(&drawn(&view)), "drawn while shown");
        nova_sim::fixture::hide(view.session.as_mut().expect("flying"), id);
        assert!(view.session().expect("flying").is_hidden(ShipRef::Npc(id)));
        assert!(!near(&drawn(&view)), "not drawn hidden");
        view.input(&Input::PointerMoved(over));
        ticks(&mut view, 2);
        assert_eq!(
            cursor_sprites(&drawn(&view)),
            [(1, over)],
            "animates over where it would be drawn"
        );
    }

    /// [`SlotSprites`] whose `slot` last cloned `shïp` 131's 100-pixel
    /// frames, as a flight that showed one there leaves it.
    fn cloned_131(slot: nova_sim::ship_layer::Slot) -> SlotSprites {
        let mut sprites = SlotSprites::default();
        sprites.frames.clone_frames(slot, ShipId(131));
        sprites
    }

    /// Whether the pointer at `over` hovers a ship in `view`, after the
    /// two ticks the cursor takes to animate.
    fn hovers(view: &mut View, over: Point) -> bool {
        view.input(&Input::PointerMoved(over));
        ticks(view, 2);
        cursor_sprites(&drawn(view))
            .first()
            .is_some_and(|&(frame, _)| frame == 1)
    }

    #[test]
    fn a_hidden_ship_is_hovered_by_the_box_its_berth_last_drew() {
        let engine = Rulebook::default;
        let hover = |rules: Rulebook, off: f32| {
            let (mut view, id) = flight_with_npc(trafficked(&[130], 1, 129, 3));
            let session = view.session.as_mut().expect("flying");
            nova_sim::fixture::hide(session, id);
            let berth = session
                .npcs()
                .iter()
                .find(|npc| npc.id == id)
                .expect("there")
                .berth;
            let mut view = view
                .with_rules(rules)
                .with_slot_sprites(cloned_131(nova_sim::ship_layer::Slot::Berth(berth)));
            let carried = sheets(&view).contains(&ShipId(131));
            assert_eq!(
                carried,
                rules == engine(),
                "read as the history is carried in"
            );
            let over = npc_on_screen(&view, id);
            hovers(&mut view, Point::new(over.x, over.y + off))
        };
        assert!(hover(engine(), 40.0), "inside shïp 131's 100-pixel frame");
        assert!(!hover(engine(), 60.0), "outside it");
        let own = || {
            Rulebook::default().with_override(
                nova_sim::RuleKey::HiddenShipHoverBox,
                nova_sim::RuleSource::Bible,
            )
        };
        assert!(hover(own(), 30.0), "inside its own 40-pixel frame's margin");
        assert!(!hover(own(), 40.0), "boxed by its own 40-pixel frame");
    }

    #[test]
    fn a_hidden_player_is_hovered_by_the_box_its_slot_last_drew() {
        let hover = |rules: Rulebook, off: f32| {
            let catalog = FakeCatalog {
                hulls: vec![nova_sim::HullRecord {
                    cost: -1,
                    ..hull_of(128, &[])
                }],
                ..trafficked(&[130], 1, 129, 3)
            };
            let (view, _) = flight_with_npc(catalog);
            let mut view = view
                .with_rules(rules)
                .with_slot_sprites(cloned_131(nova_sim::ship_layer::Slot::Player));
            assert!(view.session().expect("flying").is_hidden(ShipRef::Player));
            view.input(&key(TARGET_KEY, true));
            view.input(&key(TARGET_KEY, false));
            assert!(view.session().expect("flying").target().is_some());
            hovers(&mut view, Point::new(VIEW_CENTER.x, VIEW_CENTER.y + off))
        };
        let engine = Rulebook::default;
        assert!(hover(engine(), 40.0), "inside shïp 131's 100-pixel frame");
        assert!(!hover(engine(), 60.0));
        let own = || {
            Rulebook::default().with_override(
                nova_sim::RuleKey::HiddenShipHoverBox,
                nova_sim::RuleSource::Bible,
            )
        };
        assert!(hover(own(), 30.0));
        assert!(!hover(own(), 40.0), "boxed by its own sheet");
    }

    #[test]
    fn a_flight_hands_its_slot_sprites_to_its_session_and_back() {
        let mut sprites = SlotSprites::default();
        sprites.layers.put(
            nova_sim::ship_layer::Slot::Berth(2),
            nova_sim::ship_layer::ShipLayer::Led,
        );
        sprites
            .frames
            .clone_frames(nova_sim::ship_layer::Slot::Berth(2), ShipId(140));
        let view = flight().with_slot_sprites(sprites.clone());
        assert_eq!(view.session().expect("flying").slot_sprites(), &sprites);
        assert_eq!(view.slot_sprites(), Some(&sprites));
        let failed = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        })
        .with_slot_sprites(sprites);
        assert_eq!(failed.slot_sprites(), None);
    }

    #[test]
    fn cancelling_the_pointer_forgets_the_cursor_until_the_next_move() {
        let mut view = flight();
        view.input(&Input::PointerMoved(NOWHERE));
        assert_eq!(cursor_sprites(&drawn(&view)), [(0, NOWHERE)]);
        assert_eq!(view.cursor_at(), Some(NOWHERE));
        view.cancel_pointer();
        assert_eq!(cursor_sprites(&drawn(&view)), [], "not at the old point");
        assert_eq!(view.cursor_at(), None);
        let other = Point::new(60.0, 420.0);
        view.input(&Input::PointerMoved(other));
        assert_eq!(cursor_sprites(&drawn(&view)), [(0, other)]);
        assert_eq!(view.cursor_at(), Some(other));
    }

    #[test]
    fn the_cursor_is_hidden_once_the_pointer_leaves_the_window() {
        let mut view = flight();
        view.input(&Input::PointerMoved(NOWHERE));
        view.input(&Input::PointerLeft);
        assert_eq!(cursor_sprites(&drawn(&view)), []);
    }

    #[test]
    fn a_key_pressed_in_flight_fades_the_cursor_at_the_next_step() {
        let mut view = flight();
        view.input(&Input::PointerMoved(NOWHERE));
        view.input(&key(Key::Char('z'), true));
        ticks(&mut view, 1);
        let tint = |view: &View| {
            drawn(view).iter().find_map(|command| match *command {
                DrawCommand::Sprite { image, tint, .. } if image.id == 651 => Some(tint.a),
                _ => None,
            })
        };
        assert_eq!(tint(&view), Some(247), "31 of 32");
        view.input(&key(Key::Up, true));
        ticks(&mut view, 2);
        assert_eq!(tint(&view), Some(231), "held: 29 of 32");
    }

    #[test]
    fn the_cursor_is_not_drawn_while_the_map_is_open() {
        let mut view = flight();
        view.input(&Input::PointerMoved(NOWHERE));
        view.input(&key(MAP_KEY, true));
        assert!(view.map_open());
        assert_eq!(cursor_sprites(&drawn(&view)), []);
    }

    #[test]
    fn flight_hides_the_system_pointer_unless_the_map_is_open_or_it_has_no_cursor() {
        let mut view = flight();
        assert!(view.hides_pointer());
        view.input(&key(MAP_KEY, true));
        assert!(!view.hides_pointer(), "the map");
        let no_cursor = FakeCatalog {
            cursor_sheet: Err("no spïn 650".to_owned()),
            ..catalog()
        };
        assert!(!FlightView::new(no_cursor).hides_pointer());
        let failed = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        assert!(!FlightView::new(failed).hides_pointer());
    }

    // The floating map.

    use crate::flight::floating;
    use crate::galaxy::map::{
        BRIGHT_GREEN, CLEARED, CYAN, DARK_GREY, DENIED, DIM_GREEN, HOSTILE, LESS_DARK_GREY,
        LIGHT_GREY,
    };
    use crate::geometry::Bounds;
    use nova_sim::mission::screens::mac_ticks;

    /// The floating map's frame's centre, where Sol is drawn.
    const FRAME_CENTRE: Point = Point::new(686.0, 278.0);
    /// A point in the frame that is in no box, where [`with_sites`] puts
    /// stellar 128.
    const FRAME_MISS: Point = Point::new(700.0, 300.0);
    /// Where [`with_sites`] puts stellar 129, outside the frame.
    const OUTSIDE_FRAME: Point = Point::new(512.0, 284.0);

    /// [`catalog`] with Proxima Near ([`FakeCatalog::near`]).
    fn near() -> View {
        FlightView::new(FakeCatalog {
            near: true,
            ..catalog()
        })
    }

    /// [`near`]'s flight with Sol's stellars 128 under [`FRAME_MISS`] and
    /// 129 under [`OUTSIDE_FRAME`], as the camera starts.
    fn with_sites() -> View {
        let view = FlightView::new(FakeCatalog {
            near: true,
            sites: vec![
                site(128, (188.0, -84.0), StellarFlags::CAN_LAND),
                site(129, (0.0, -100.0), StellarFlags::CAN_LAND),
            ],
            ..catalog()
        });
        assert_eq!(on_screen(&view, (188.0, -84.0)), FRAME_MISS);
        assert_eq!(on_screen(&view, (0.0, -100.0)), OUTSIDE_FRAME);
        view
    }

    /// The floating map's border lines in `list`: those in the
    /// `FloatingMap` colour on the frame's edges, whatever their alpha.
    fn floating_border(list: &DrawList) -> Vec<DrawCommand> {
        let border = MENU_COLORS.border;
        list.iter()
            .filter(|command| {
                matches!(command, DrawCommand::Line { from, color, .. }
                    if (color.r, color.g, color.b) == (border.r, border.g, border.b)
                        && from.x >= 558.0)
            })
            .cloned()
            .collect()
    }

    /// The alpha the floating map's border is drawn at, if it is drawn.
    fn floating_alpha(view: &View) -> Option<u8> {
        floating_border(&drawn(view))
            .first()
            .map(|command| match command {
                DrawCommand::Line { color, .. } => color.a,
                _ => unreachable!(),
            })
    }

    /// Where the floating map draws systems' ovals, in draw order: the
    /// centre of each run of 1-pixel rows in a colour code's colour, each
    /// row the one after the last, inside the frame. Its black fills cover
    /// but draw nothing.
    fn floating_dots(view: &View) -> Vec<Point> {
        let codes = [
            DARK_GREY,
            LIGHT_GREY,
            BRIGHT_GREEN,
            CLEARED,
            DENIED,
            HOSTILE,
        ];
        let frame = floating::frame(VIEW_SIZE);
        let mut ovals: Vec<(Color, f32, Bounds)> = Vec::new();
        for command in &drawn(view) {
            let DrawCommand::Line {
                from,
                to,
                width,
                color,
            } = *command
            else {
                continue;
            };
            let opaque = Color { a: 255, ..color };
            if width != 1.0 || from.y != to.y || !codes.contains(&opaque) {
                continue;
            }
            if !floating::contains(frame, from) {
                continue;
            }
            let row = Bounds {
                min: Point::new(from.x.min(to.x), from.y - 0.5),
                max: Point::new(from.x.max(to.x), from.y + 0.5),
            };
            match ovals.last_mut() {
                Some((last, top, area))
                    if *last == opaque && (row.min.y == *top || row.min.y == *top + 1.0) =>
                {
                    *top = row.min.y;
                    area.min.x = area.min.x.min(row.min.x);
                    area.max.x = area.max.x.max(row.max.x);
                    area.max.y = row.max.y;
                }
                _ => ovals.push((opaque, row.min.y, row)),
            }
        }
        ovals
            .into_iter()
            .map(|(_, _, area)| area.center())
            .collect()
    }

    /// Where the floating map draws Proxima Near (134) at the zoom whose
    /// scale is `scale`: the pixel its point truncates to.
    fn near_at(scale: f32) -> Point {
        Point::new(
            (686.0 + 40.0 * scale).trunc(),
            (278.0 + 30.0 * scale).trunc(),
        )
    }

    fn sims(view: &mut View) -> Vec<SimSound> {
        view.take_sounds()
            .into_iter()
            .filter_map(|sound| match sound {
                Sound::Sim(sound) => Some(sound),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn holding_h_shows_the_floating_map_bordered_and_centred_on_sol() {
        let mut view = flight();
        assert_eq!(floating_border(&drawn(&view)), [], "down at first");
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        view.tick(TICK);
        let list = drawn(&view);
        let edge = |from: (f32, f32), to: (f32, f32)| DrawCommand::Line {
            from: at(from.0, from.1),
            to: at(to.0, to.1),
            width: 1.0,
            color: MENU_COLORS.border,
        };
        assert_eq!(
            floating_border(&list),
            [
                edge((558.0, 150.0), (814.0, 150.0)),
                edge((814.0, 150.0), (814.0, 406.0)),
                edge((814.0, 406.0), (558.0, 406.0)),
                edge((558.0, 406.0), (558.0, 150.0)),
            ]
        );
        assert_eq!(floating_dots(&view), [FRAME_CENTRE], "Sol, at the centre");
        assert!(
            list.iter().any(|command| matches!(command,
                DrawCommand::Line { color, .. } if *color == CYAN)),
            "the current system's dot in it"
        );
    }

    #[test]
    fn holding_h_keeps_it_full_and_letting_go_starts_the_fade() {
        let mut view = flight();
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        view.input(&held(HYPERSPACE_MODE_KEY));
        for _ in 0..100 {
            view.tick(TICK);
        }
        view.input(&key(HYPERSPACE_MODE_KEY, false));
        view.tick(mac_ticks(118));
        assert_eq!(floating_alpha(&view), Some(255), "shown again each frame");
        view.tick(mac_ticks(1));
        assert_eq!(floating_alpha(&view), Some(247), "31 of 32 at 119");
        view.tick(mac_ticks(31));
        assert_eq!(floating_alpha(&view), None, "faded out at 150");
    }

    #[test]
    fn a_tap_of_h_shows_it_before_the_next_frame() {
        let mut view = flight();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        assert_eq!(floating_alpha(&view), Some(255));
        view.tick(mac_ticks(118));
        assert_eq!(floating_alpha(&view), Some(255));
        view.tick(mac_ticks(1));
        assert_eq!(floating_alpha(&view), Some(247), "not shown again");
    }

    #[test]
    fn in_double_time_the_fade_keeps_its_ticks() {
        let mut view = flight().with_double_time(true);
        tap(&mut view, HYPERSPACE_MODE_KEY);
        view.tick(mac_ticks(118));
        assert_eq!(floating_alpha(&view), Some(255));
        view.tick(mac_ticks(1));
        assert_eq!(floating_alpha(&view), Some(247));
        view.tick(mac_ticks(31));
        assert_eq!(floating_alpha(&view), None);
    }

    #[test]
    fn h_held_through_a_jump_doesnt_show_it() {
        for mut view in through_a_jump(flight) {
            let stage = stage(&view);
            view.input(&key(HYPERSPACE_MODE_KEY, true));
            view.tick(TICK);
            assert_eq!(floating_alpha(&view), None, "{stage}");
        }
    }

    #[test]
    fn backslash_held_in_mode_3_shows_it_through_a_jump() {
        for mut view in through_a_jump(flight) {
            let stage = stage(&view);
            assert!(matches!(nav_mode(&view), NavMode::Hyperspace(_)), "{stage}");
            view.input(&key(HYPER_SELECT_KEY, true));
            assert_eq!(floating_alpha(&view), None, "{stage}: no press");
            view.tick(mac_ticks(1));
            assert_eq!(floating_alpha(&view), Some(255), "{stage}");
        }
    }

    #[test]
    fn backslash_held_out_of_mode_3_doesnt_show_it() {
        let mut view = FlightView::new(FakeCatalog {
            onward: true,
            ..catalog()
        });
        run_set(&mut view, "M132");
        view.settle_script();
        assert_eq!(system_of(&view), SystemId(132), "Barnard lists no links");
        view.input(&key(HYPER_SELECT_KEY, true));
        assert_eq!(nav_mode(&view), NavMode::Off);
        view.tick(TICK);
        assert_eq!(floating_alpha(&view), None);
    }

    #[test]
    fn h_held_in_the_take_off_grace_shows_it() {
        let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        land_now(&mut view);
        view.take_off().expect("takes off");
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        ticks(&mut view, 14);
        assert!(!view.takes(PlayerCommand::Map), "still in the grace");
        view.input(&key(HYPERSPACE_MODE_KEY, false));
        view.tick(mac_ticks(118));
        assert_eq!(floating_alpha(&view), Some(255), "held through the grace");
    }

    #[test]
    fn h_held_once_exploded_doesnt_show_it() {
        let mut view = exploded();
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        view.tick(TICK);
        assert_eq!(floating_alpha(&view), None);
    }

    #[test]
    fn backslash_held_in_mode_3_once_exploded_doesnt_show_it() {
        let mut view =
            fighting(killed_in(1), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        tap(&mut view, HYPERSPACE_MODE_KEY);
        tap(&mut view, MENU_CLOSE_KEY);
        until_the_player_explodes(&mut view, |_| {});
        view.tick(TICK);
        assert!(matches!(nav_mode(&view), NavMode::Hyperspace(_)), "kept");
        view.input(&key(HYPER_SELECT_KEY, true));
        view.tick(TICK);
        assert_eq!(floating_alpha(&view), None);
    }

    #[test]
    fn it_hides_once_more_than_500_ticks_have_passed() {
        let mut view = with_sites();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        view.tick(mac_ticks(500));
        click(&mut view, FRAME_MISS);
        assert_eq!(nav_target(&view), None, "still up at 500: swallowed");
        view.tick(mac_ticks(1));
        click(&mut view, FRAME_MISS);
        assert_eq!(nav_target(&view), Some(StellarId(128)), "hidden at 501");
    }

    #[test]
    fn return_hides_it_through_a_jump_too() {
        let mut view = flight();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        tap(&mut view, MENU_CLOSE_KEY);
        assert_eq!(floating_alpha(&view), None);
        for mut view in through_a_jump(flight) {
            let stage = stage(&view);
            tap(&mut view, HYPER_SELECT_KEY);
            view.input(&key(HYPER_SELECT_KEY, true));
            view.tick(TICK);
            view.input(&key(HYPER_SELECT_KEY, false));
            assert_eq!(floating_alpha(&view), Some(255), "{stage}");
            tap(&mut view, MENU_CLOSE_KEY);
            assert_eq!(floating_alpha(&view), None, "{stage}");
        }
    }

    #[test]
    fn a_click_on_the_centre_selects_mode_3_with_nothing_and_beeps() {
        let mut view = near();
        plot(&mut view, 131);
        tap(&mut view, HYPERSPACE_MODE_KEY);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
        view.take_sounds();
        click(&mut view, FRAME_CENTRE);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
        assert_eq!(sims(&mut view), [SimSound::HyperSelected]);
        view.tick(TICK);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_NO_DESTINATION]);
        assert_eq!(course(&view), [SystemId(131)], "the course kept");
    }

    #[test]
    fn a_click_in_a_links_box_selects_it_in_mode_3_and_beeps() {
        let mut view = near();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
        view.take_sounds();
        // Proxima Near, 40 right and 30 down at 16/9: (757.1, 331.3).
        click(&mut view, at(757.0, 331.0));
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(134))));
        assert_eq!(sims(&mut view), [SimSound::HyperSelected]);
    }

    #[test]
    fn away_from_the_origin_the_map_centres_on_the_current_system_and_clicks_follow() {
        let mut view = near();
        run_set(&mut view, "M134");
        view.settle_script();
        assert_eq!(system_of(&view), SystemId(134), "Proxima Near, at (40, 30)");
        tap(&mut view, HYPERSPACE_MODE_KEY);
        // Sol, 40 left and 30 up at 16/9: (614.9, 224.7).
        let sol = Point::new(
            (686.0 - 40.0 * (16.0_f32 / 9.0)).trunc(),
            (278.0 - 30.0 * (16.0_f32 / 9.0)).trunc(),
        );
        assert_eq!(
            floating_dots(&view),
            [sol, FRAME_CENTRE],
            "Sol, then Proxima Near at the centre, by ascending ID"
        );
        view.take_sounds();
        click(&mut view, at(614.0, 224.0));
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(130))));
        assert_eq!(sims(&mut view), [SimSound::HyperSelected]);
    }

    #[test]
    fn a_miss_in_the_frame_picks_nothing_and_plays_nothing() {
        let mut view = with_sites();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        view.take_sounds();
        click(&mut view, FRAME_MISS);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
        assert_eq!(view.take_sounds(), []);
        click(&mut view, OUTSIDE_FRAME);
        assert_eq!(
            nav_target(&view),
            Some(StellarId(129)),
            "outside: a stellar"
        );
        let mut view = with_sites();
        click(&mut view, FRAME_MISS);
        assert_eq!(nav_target(&view), Some(StellarId(128)), "the map down");
    }

    #[test]
    fn a_click_is_taken_while_the_map_is_faded_out() {
        let mut view = with_sites();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        view.tick(mac_ticks(200));
        assert_eq!(floating_alpha(&view), None, "faded out");
        click(&mut view, FRAME_MISS);
        assert_eq!(nav_target(&view), None, "swallowed");
        view.take_sounds();
        click(&mut view, FRAME_CENTRE);
        assert_eq!(sims(&mut view), [SimSound::HyperSelected]);
    }

    #[test]
    fn a_hit_shows_the_map_again() {
        let mut view = near();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        view.tick(mac_ticks(100));
        click(&mut view, FRAME_CENTRE);
        view.tick(mac_ticks(118));
        assert_eq!(floating_alpha(&view), Some(255));
        view.tick(mac_ticks(1));
        assert_eq!(floating_alpha(&view), Some(247));
    }

    #[test]
    fn a_click_in_the_frame_during_a_jump_selects_nothing() {
        for mut view in through_a_jump(flight) {
            let stage = stage(&view);
            view.input(&key(HYPER_SELECT_KEY, true));
            view.tick(TICK);
            assert_eq!(floating_alpha(&view), Some(255), "{stage}");
            let mode = nav_mode(&view);
            view.take_sounds();
            click(&mut view, FRAME_CENTRE);
            assert_eq!(nav_mode(&view), mode, "{stage}");
            assert!(
                !sims(&mut view).contains(&SimSound::HyperSelected),
                "{stage}"
            );
        }
    }

    #[test]
    fn minus_zooms_out_a_step_with_snd_150() {
        let mut view = near();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        view.take_sounds();
        assert_eq!(floating_dots(&view), [FRAME_CENTRE, near_at(16.0 / 9.0)]);
        tap(&mut view, ZOOM_OUT_KEY);
        assert_eq!(floating_dots(&view), [FRAME_CENTRE, near_at(4.0 / 3.0)]);
        assert_eq!(view.take_sounds(), [Sound::Ui(UiSound::FloatingMapZoomed)]);
        assert_eq!(ZOOM_OUT_KEY, Key::Char('-'));
    }

    #[test]
    fn equals_and_keypad_plus_zoom_in_a_step() {
        for k in [ZOOM_IN_KEY, ZOOM_IN_KEYPAD_KEY] {
            let mut view = near();
            tap(&mut view, HYPERSPACE_MODE_KEY);
            view.take_sounds();
            tap(&mut view, k);
            assert_eq!(floating_dots(&view)[1], near_at(64.0 / 27.0), "{k:?}");
            assert_eq!(
                view.take_sounds(),
                [Sound::Ui(UiSound::FloatingMapZoomed)],
                "{k:?}"
            );
        }
        assert_eq!(ZOOM_IN_KEY, Key::Char('='));
        assert_eq!(ZOOM_IN_KEYPAD_KEY, Key::Char('+'));
    }

    #[test]
    fn a_zoom_at_its_limit_changes_nothing_and_says_nothing() {
        let mut view = near();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        tap(&mut view, ZOOM_IN_KEY);
        view.take_sounds();
        tap(&mut view, ZOOM_IN_KEY);
        assert_eq!(floating_dots(&view)[1], near_at(64.0 / 27.0));
        assert_eq!(view.take_sounds(), []);
        for _ in 0..6 {
            tap(&mut view, ZOOM_OUT_KEY);
        }
        assert_eq!(floating_dots(&view)[1], near_at(27.0 / 64.0));
        view.take_sounds();
        tap(&mut view, ZOOM_OUT_KEY);
        assert_eq!(floating_dots(&view)[1], near_at(27.0 / 64.0));
        assert_eq!(view.take_sounds(), []);
    }

    #[test]
    fn a_zoom_key_does_nothing_while_another_key_is_held() {
        for k in [
            Key::Shift,
            Key::Alt,
            Key::Control,
            Key::Command,
            HYPERSPACE_MODE_KEY,
            HYPER_SELECT_KEY,
            NAV_OFF_KEY,
            JUMP_KEY,
            Key::Up,
        ] {
            let mut view = near();
            tap(&mut view, HYPERSPACE_MODE_KEY);
            view.input(&key(k, true));
            view.take_sounds();
            tap(&mut view, ZOOM_OUT_KEY);
            assert_eq!(floating_dots(&view)[1], near_at(16.0 / 9.0), "{k:?}");
            assert_eq!(view.take_sounds(), [], "{k:?}");
            view.input(&key(k, false));
            tap(&mut view, ZOOM_OUT_KEY);
            assert_eq!(floating_dots(&view)[1], near_at(4.0 / 3.0), "{k:?} let go");
        }
    }

    #[test]
    fn a_zoom_key_does_nothing_with_the_map_down() {
        let mut view = near();
        tap(&mut view, ZOOM_OUT_KEY);
        tap(&mut view, ZOOM_IN_KEY);
        assert_eq!(view.take_sounds(), []);
        tap(&mut view, HYPERSPACE_MODE_KEY);
        assert_eq!(floating_dots(&view)[1], near_at(16.0 / 9.0));
    }

    #[test]
    fn a_zoom_shows_the_map_again() {
        let mut view = near();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        view.tick(mac_ticks(140));
        tap(&mut view, ZOOM_OUT_KEY);
        view.tick(mac_ticks(118));
        assert_eq!(floating_alpha(&view), Some(255));
    }

    #[test]
    fn zooms_are_taken_through_a_jump_and_refused_once_exploded() {
        for mut view in through_a_jump(near) {
            let stage = stage(&view);
            view.input(&key(HYPER_SELECT_KEY, true));
            view.tick(TICK);
            view.input(&key(HYPER_SELECT_KEY, false));
            view.take_sounds();
            tap(&mut view, ZOOM_OUT_KEY);
            assert!(
                view.take_sounds()
                    .contains(&Sound::Ui(UiSound::FloatingMapZoomed)),
                "{stage}"
            );
            assert_eq!(view.floating.zoom(), 4, "{stage}");
        }
        let mut view = exploded();
        view.floating.show(sixtieths(view.elapsed));
        view.take_sounds();
        tap(&mut view, ZOOM_OUT_KEY);
        assert_eq!(view.floating.zoom(), floating::START_ZOOM);
        assert!(
            !view
                .take_sounds()
                .contains(&Sound::Ui(UiSound::FloatingMapZoomed))
        );
    }

    #[test]
    fn the_map_is_drawn_after_the_double_time_icon_and_before_the_escort_menu() {
        let mut view = escorted().with_double_time(true);
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        tap(&mut view, HYPERSPACE_MODE_KEY);
        let list = drawn(&view);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        let icon = commands
            .iter()
            .position(|command| icon_in(20000..=20000)(command))
            .expect("the double-time icon");
        let border = floating_border(&list);
        let map = commands
            .iter()
            .position(|command| **command == border[0])
            .expect("the floating map");
        let menu = commands
            .iter()
            .position(|command| {
                matches!(command, DrawCommand::Line { color, .. } if *color == Color::BLACK)
            })
            .expect("the escort menu's black");
        assert!(icon < map && map < menu, "{icon} < {map} < {menu}");
    }

    /// The rects of `color`, whatever their alpha, in `list`, as (left,
    /// top, right, bottom).
    fn rects_of(list: &DrawList, of: Color) -> Vec<(f32, f32, f32, f32)> {
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Line {
                    from,
                    to,
                    width,
                    color,
                } if Color { a: 255, ..color } == of && from.y == to.y => Some((
                    from.x.min(to.x),
                    from.y - width / 2.0,
                    from.x.max(to.x),
                    from.y + width / 2.0,
                )),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_floating_map_draws_the_sessions_selection_clipped_to_the_frame() {
        let mut view = flight();
        tap(&mut view, HYPER_SELECT_KEY);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
        assert_eq!(view.course_map().hyper_selection(), None, "never opened");
        let list = drawn(&view);
        assert_eq!(
            rects_of(&list, DIM_GREEN),
            [(690.0, 278.0, 814.0, 279.0)],
            "from the edge of Sol's oval to the frame's, drawn once"
        );
        assert_eq!(
            rects_of(&list, LESS_DARK_GREY),
            [],
            "the link under it is cut away"
        );
    }

    #[test]
    fn the_floating_map_draws_the_course_as_its_show_took_it() {
        let mut view = flight();
        view.session
            .as_mut()
            .expect("flying")
            .plot_course(SystemId(131))
            .expect("a route");
        assert_eq!(
            view.course_map().route(),
            [],
            "the course map never shown it"
        );
        tap(&mut view, HYPERSPACE_MODE_KEY);
        let course = rects_of(&drawn(&view), BRIGHT_GREEN);
        assert_eq!(
            course,
            [(690.0, 279.0, 814.0, 280.0)],
            "from the edge of Sol's oval to the frame's, at pen 2, its top \
             row under Sol's link"
        );
        assert_eq!(
            rects_of(&drawn(&view), DIM_GREEN),
            [(690.0, 278.0, 814.0, 279.0)],
            "the link to 131, the hop the plot cued, drawn after the course \
             over it, and over its own grey"
        );
        assert_eq!(rects_of(&drawn(&view), LESS_DARK_GREY), []);
        let session = view.session.as_mut().expect("flying");
        let _ = session.plot_course(SystemId(130));
        assert_ne!(session.course(), [SystemId(131)], "the course changed");
        assert_eq!(
            rects_of(&drawn(&view), BRIGHT_GREEN),
            course,
            "until the next show"
        );
    }

    /// The names the floating map draws, clipped to its frame.
    fn floating_names(view: &View) -> Vec<String> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Clipped { command, .. } => match command.as_ref() {
                    DrawCommand::Text { text, .. } => Some(text.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    #[test]
    fn above_a_zoom_of_1_1_the_floating_map_names_only_the_system_its_show_selected() {
        let mut view = near();
        run_set(&mut view, "X134");
        tap(&mut view, HYPER_SELECT_KEY);
        tap(&mut view, HYPER_SELECT_KEY);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(134))));
        assert_eq!(
            floating_names(&view),
            ["Sol", "Proxima Near"],
            "every explored system in the frame at 16/9"
        );
        for _ in 0..3 {
            tap(&mut view, ZOOM_OUT_KEY);
        }
        assert_eq!(floating_names(&view), ["Proxima Near"], "at 3/4");
        tap(&mut view, NAV_OFF_KEY);
        tap(&mut view, HYPERSPACE_MODE_KEY);
        assert_eq!(floating_names(&view), [] as [&str; 0], "shown with none");
    }

    #[test]
    fn the_floating_map_takes_its_mission_arrows_as_it_shows() {
        let mut catalog = catalog();
        catalog.missions = vec![MissionRecord {
            travel_stel: 140,
            ..errand(130)
        }];
        let mut view = FlightView::new(catalog);
        tap(&mut view, HYPERSPACE_MODE_KEY);
        let red = |view: &View| {
            view.floating_picture
                .as_ref()
                .expect("shown")
                .systems
                .iter()
                .any(|system| system.id == SystemId(131) && system.red_arrow)
        };
        assert!(!red(&view), "no mission");
        run_set(&mut view, "S130");
        view.settle_script();
        assert!(!red(&view), "not until the next show");
        tap(&mut view, HYPERSPACE_MODE_KEY);
        assert!(red(&view), "Alpha Centauri, 140's system, arrowed");
    }

    #[test]
    fn the_hypergate_maps_offers_get_their_ovals_from_its_second_draw() {
        // The gate leads to Barnard (132), which nothing has made known.
        let mut view = FlightView::new(FakeCatalog {
            gate_listers: vec![(SystemId(132), StellarId(310))],
            ..gated()
        });
        land_now(&mut view);
        let drawn_systems = |view: &View| -> Vec<SystemId> {
            view.course_map()
                .picture()
                .expect("drawn")
                .systems
                .iter()
                .map(|system| system.id)
                .collect()
        };
        assert_eq!(view.course_map().mode(), MapMode::Hypergate);
        assert_eq!(
            view.course_map().picture().expect("drawn").gate_arrows,
            [SystemId(132)]
        );
        assert!(!drawn_systems(&view).contains(&SystemId(132)), "first draw");
        assert!(
            view.session().expect("flying").is_known(SystemId(132)),
            "marked known as its arrow was drawn"
        );
        view.tick(TICK);
        assert!(!drawn_systems(&view).contains(&SystemId(132)), "a frame on");
        tap(&mut view, Key::Tab);
        assert!(drawn_systems(&view).contains(&SystemId(132)), "after Tab");
    }

    #[test]
    fn a_click_on_the_hypergate_map_draws_it_again() {
        let mut view = FlightView::new(FakeCatalog {
            gate_listers: vec![(SystemId(132), StellarId(310))],
            ..gated()
        });
        land_now(&mut view);
        click(&mut view, at(5.0, 5.0));
        let drawn: Vec<SystemId> = view
            .course_map()
            .picture()
            .expect("drawn")
            .systems
            .iter()
            .map(|system| system.id)
            .collect();
        assert!(drawn.contains(&SystemId(132)));
    }

    #[test]
    fn the_course_map_draws_the_sessions_known_and_explored_systems() {
        let mut view = flight();
        tap(&mut view, MAP);
        let picture = view.course_map().picture().expect("drawn").clone();
        let ids: Vec<i16> = picture.systems.iter().map(|system| system.id.0).collect();
        assert_eq!(ids, [130, 131], "Sol and its known link, not Barnard");
        assert_eq!(picture.named, [SystemId(130)]);
        assert_eq!(picture.current, Some(SystemId(130)));
        run_set(&mut view, "X132");
        view.tick(TICK);
        let ids: Vec<i16> = view
            .course_map()
            .picture()
            .expect("drawn")
            .systems
            .iter()
            .map(|system| system.id.0)
            .collect();
        assert_eq!(ids, [130, 131, 132], "explored while the map is open");
    }

    // Keys held across a jump's end, an Option release or a repair
    // (`_HandlePlayer`'s latches 0xdee04, 0xdee48 and 0xdee44).

    /// The times `view` raised `sound` since its sounds were last taken.
    fn count(view: &mut View, sound: SimSound) -> usize {
        sims(view).into_iter().filter(|&s| s == sound).count()
    }

    /// A flight on to Barnard's Star ([`FakeCatalog::onward`]) with a
    /// course plotted to `to` and flown out, 300 ms into the jump's
    /// streak, its sounds let go.
    fn streaking_to(to: i16) -> View {
        let mut view = FlightView::new(FakeCatalog {
            onward: true,
            ..catalog()
        });
        plot(&mut view, to);
        fly_out(&mut view);
        jump_now(&mut view);
        view.tick(ms(300));
        assert!(view.jump_effect().is_some());
        view.take_sounds();
        view
    }

    /// Ticks `view` through its arrival tick, asserting that no `sound`
    /// was raised before or on it, then once more, giving how many times
    /// that tick raised `sound`; then once more again, asserting it is not
    /// raised a second time.
    fn after_arrival(view: &mut View, sound: SimSound) -> (NavMode, usize, NavMode) {
        tick_to_arrival(view);
        assert_eq!(count(view, sound), 0, "not through the jump nor on arrival");
        let arrived = nav_mode(view);
        view.tick(TICK);
        let acted = count(view, sound);
        let after = nav_mode(view);
        view.tick(TICK);
        assert_eq!(count(view, sound), 0, "once only");
        assert_eq!(nav_mode(view), after, "nothing more");
        (arrived, acted, after)
    }

    #[test]
    fn h_pressed_before_j_and_held_through_the_jump_acts_the_tick_after_arrival() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        jump_now(&mut view);
        view.take_sounds();
        let (arrived, acted, after) = after_arrival(&mut view, SimSound::NavSelected);
        assert_eq!(arrived, NavMode::Off, "the course flown");
        assert_eq!(acted, 1);
        assert_eq!(after, NavMode::Hyperspace(None));
        assert_eq!(floating_alpha(&view), Some(255), "the map up");
    }

    #[test]
    fn h_held_through_a_jump_that_cues_the_next_hop_only_beeps() {
        let mut view = streaking_to(132);
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        let (arrived, acted, after) = after_arrival(&mut view, SimSound::NavSelected);
        assert_eq!(arrived, NavMode::Hyperspace(Some(SystemId(132))));
        assert_eq!((acted, after), (1, arrived), "the selection kept");
    }

    #[test]
    fn h_first_pressed_during_the_jump_and_held_acts_after_arrival() {
        for mut view in through_a_jump(flight) {
            let stage = stage(&view);
            view.input(&key(HYPERSPACE_MODE_KEY, true));
            view.take_sounds();
            let (_, acted, after) = after_arrival(&mut view, SimSound::NavSelected);
            assert_eq!((acted, after), (1, NavMode::Hyperspace(None)), "{stage}");
        }
    }

    #[test]
    fn h_pressed_during_the_jump_and_let_go_before_arrival_does_nothing_after() {
        let mut view = streaking_to(131);
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        view.tick(TICK);
        view.input(&key(HYPERSPACE_MODE_KEY, false));
        let (_, acted, after) = after_arrival(&mut view, SimSound::NavSelected);
        assert_eq!((acted, after), (0, NavMode::Off));
    }

    #[test]
    fn nav_off_held_through_the_jump_turns_the_nav_off_after_arrival() {
        let mut view = streaking_to(132);
        view.input(&key(NAV_OFF_KEY, true));
        let (arrived, acted, after) = after_arrival(&mut view, SimSound::NavSelected);
        assert_eq!(arrived, NavMode::Hyperspace(Some(SystemId(132))));
        assert_eq!((acted, after), (1, NavMode::Off));
    }

    #[test]
    fn option_backquote_turns_into_nav_off_when_option_is_let_go() {
        let mut view = flight();
        plot(&mut view, 131);
        view.input(&key(Key::Alt, true));
        view.input(&key(NAV_OFF_KEY, true));
        ticks(&mut view, 3);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
        assert_eq!(count(&mut view, SimSound::NavSelected), 0, "Option held");
        view.input(&key(Key::Alt, false));
        assert_eq!(
            count(&mut view, SimSound::NavSelected),
            0,
            "at the next frame"
        );
        view.tick(TICK);
        assert_eq!(count(&mut view, SimSound::NavSelected), 1);
        assert_eq!(nav_mode(&view), NavMode::Off);
        ticks(&mut view, 2);
        assert_eq!(count(&mut view, SimSound::NavSelected), 0, "once");
    }

    #[test]
    fn h_held_blocks_a_press_of_nav_off() {
        let mut view = flight();
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        view.tick(TICK);
        view.take_sounds();
        view.input(&key(NAV_OFF_KEY, true));
        view.tick(TICK);
        assert_eq!(sims(&mut view), []);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
    }

    #[test]
    fn nav_off_held_blocks_a_press_of_h_which_still_shows_the_map() {
        let mut view = flight();
        view.input(&key(NAV_OFF_KEY, true));
        view.tick(TICK);
        view.take_sounds();
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        assert_eq!(floating_alpha(&view), Some(255), "on the press's own frame");
        view.tick(TICK);
        assert_eq!(sims(&mut view), []);
        assert_eq!(nav_mode(&view), NavMode::Off);
    }

    #[test]
    fn a_key_of_the_block_let_go_frees_the_other_held_to_press_again() {
        let mut view = flight();
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        view.input(&key(NAV_OFF_KEY, true));
        view.input(&key(HYPERSPACE_MODE_KEY, false));
        view.tick(TICK);
        view.take_sounds();
        view.input(&key(NAV_OFF_KEY, false));
        view.tick(TICK);
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        assert_eq!(
            count(&mut view, SimSound::NavSelected),
            1,
            "both up a frame"
        );
    }

    #[test]
    fn each_fresh_press_of_h_or_nav_off_acts_with_no_tick_between() {
        for k in [HYPERSPACE_MODE_KEY, NAV_OFF_KEY] {
            let mut view = flight();
            tap(&mut view, k);
            tap(&mut view, k);
            assert_eq!(count(&mut view, SimSound::NavSelected), 2, "{k:?}");
        }
    }

    #[test]
    fn h_and_nav_off_held_through_the_jump_act_once_together_after_arrival() {
        let mut view = streaking_to(131);
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        view.input(&key(NAV_OFF_KEY, true));
        let (arrived, acted, after) = after_arrival(&mut view, SimSound::NavSelected);
        assert_eq!(arrived, NavMode::Off);
        assert_eq!((acted, after), (1, NavMode::Hyperspace(None)));
    }

    #[test]
    fn option_backquote_held_lets_go_of_a_target_picked_the_next_tick() {
        let mut view = fighting(armed(2, &[]), &two_npcs());
        view.input(&key(Key::Alt, true));
        view.input(&key(NAV_OFF_KEY, true));
        view.tick(TICK);
        view.take_sounds();
        tap(&mut view, NEAREST_KEY);
        assert!(target_of(&view).is_some(), "Option-R picks");
        view.take_sounds();
        view.tick(TICK);
        assert_eq!(target_of(&view), None);
        assert_eq!(count(&mut view, SimSound::TargetCleared), 1);
    }

    #[test]
    fn another_key_pressed_while_option_backquote_is_held_leaves_the_target_to_the_tick() {
        let mut view = fighting(armed(2, &[]), &two_npcs());
        view.input(&key(Key::Alt, true));
        view.input(&key(NAV_OFF_KEY, true));
        view.tick(TICK);
        alt_tap(&mut view, NEAREST_KEY);
        view.input(&key(Key::Alt, true));
        let picked = target_of(&view);
        assert!(picked.is_some());
        tap(&mut view, Key::Char('z'));
        assert_eq!(target_of(&view), picked, "no press of the block");
    }

    #[test]
    fn option_backquote_held_blocks_h_but_not_its_map_nor_with_h_held_its_target() {
        let mut view = fighting(armed(2, &[]), &two_npcs());
        view.input(&key(Key::Alt, true));
        view.input(&key(NAV_OFF_KEY, true));
        view.tick(TICK);
        view.take_sounds();
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        assert_eq!(floating_alpha(&view), Some(255), "the map shown");
        view.tick(TICK);
        assert_eq!(count(&mut view, SimSound::NavSelected), 0, "H blocked");
        assert_eq!(nav_mode(&view), NavMode::Off);
        tap(&mut view, NEAREST_KEY);
        assert!(target_of(&view).is_some());
        view.tick(TICK);
        assert!(target_of(&view).is_some(), "the latch held by H");
    }

    /// The system Hyper Select picks first from Alpha Centauri (131), the
    /// first of its links.
    fn first_link_of_131() -> NavMode {
        let mut view = flight();
        run_set(&mut view, "M131");
        view.settle_script();
        tap(&mut view, HYPER_SELECT_KEY);
        nav_mode(&view)
    }

    #[test]
    fn backslash_first_pressed_during_the_jump_and_held_acts_after_arrival() {
        let first = first_link_of_131();
        assert!(matches!(first, NavMode::Hyperspace(Some(_))));
        for mut view in through_a_jump(flight) {
            let stage = stage(&view);
            view.input(&key(HYPER_SELECT_KEY, true));
            view.take_sounds();
            let (_, acted, after) = after_arrival(&mut view, SimSound::HyperSelected);
            assert_eq!((acted, after), (1, first), "{stage}");
            assert_eq!(floating_alpha(&view), Some(255), "{stage}: the map up");
        }
    }

    #[test]
    fn backslash_pressed_before_j_and_held_through_does_nothing_after() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(HYPER_SELECT_KEY, true));
        assert_eq!(count(&mut view, SimSound::HyperSelected), 1);
        jump_now(&mut view);
        view.take_sounds();
        let (arrived, acted, after) = after_arrival(&mut view, SimSound::HyperSelected);
        assert_eq!((acted, after), (0, arrived));
    }

    #[test]
    fn backslash_let_go_and_pressed_again_during_the_jump_acts_after_arrival() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(HYPER_SELECT_KEY, true));
        jump_now(&mut view);
        view.input(&key(HYPER_SELECT_KEY, false));
        view.tick(TICK);
        view.input(&key(HYPER_SELECT_KEY, true));
        view.take_sounds();
        let (_, acted, after) = after_arrival(&mut view, SimSound::HyperSelected);
        assert_eq!(acted, 1);
        assert!(matches!(after, NavMode::Hyperspace(Some(_))));
    }

    /// Sets the player's condition in `view` to `to`.
    fn condition(view: &mut View, to: Condition) {
        let Ok(session) = &mut view.session else {
            panic!("flying");
        };
        nova_sim::fixture::set_player_condition(session, to);
    }

    /// Ticks `view` once with the player held disabled.
    fn tick_disabled(view: &mut View) {
        condition(view, Condition::Disabled);
        view.tick(TICK);
    }

    #[test]
    fn j_held_into_a_disable_acts_again_on_the_first_intact_tick() {
        let mut view = new_pilot(flight());
        view.input(&key(JUMP, true));
        assert_eq!(view.take_sounds(), [REFUSED]);
        assert_eq!(view.message(), Some(NO_DESTINATION));
        view.tick(TICK);
        assert!(!view.take_sounds().contains(&REFUSED), "held: no repeat");
        tick_disabled(&mut view);
        view.message = None;
        condition(&mut view, Condition::Intact);
        view.tick(TICK);
        assert_eq!(
            view.take_sounds()
                .iter()
                .filter(|&&sound| sound == REFUSED)
                .count(),
            1
        );
        assert_eq!(view.message(), Some(NO_DESTINATION));
        ticks(&mut view, 2);
        assert!(!view.take_sounds().contains(&REFUSED), "once");
    }

    #[test]
    fn j_first_pressed_while_disabled_and_held_jumps_on_the_first_intact_tick() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        condition(&mut view, Condition::Disabled);
        view.input(&key(JUMP, true));
        tick_disabled(&mut view);
        tick_disabled(&mut view);
        assert!(
            !view.preparing_jump() && view.jump_effect().is_none(),
            "silent"
        );
        condition(&mut view, Condition::Intact);
        view.tick(TICK);
        assert!(view.preparing_jump() || view.jump_effect().is_some());
    }

    #[test]
    fn j_let_go_while_disabled_does_nothing_on_the_repair() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        condition(&mut view, Condition::Disabled);
        view.input(&key(JUMP, false));
        tick_disabled(&mut view);
        condition(&mut view, Condition::Intact);
        ticks(&mut view, 2);
        assert!(!view.preparing_jump() && view.jump_effect().is_none());
    }

    #[test]
    fn j_held_while_intact_is_refused_once() {
        let mut view = new_pilot(flight());
        view.input(&key(JUMP, true));
        ticks(&mut view, 5);
        let refused = view
            .take_sounds()
            .iter()
            .filter(|&&sound| sound == REFUSED)
            .count();
        assert_eq!(refused, 1);
    }

    #[test]
    fn j_held_through_a_jump_with_a_hop_to_go_does_not_begin_the_next() {
        let mut view = FlightView::new(FakeCatalog {
            onward: true,
            ..catalog()
        });
        plot(&mut view, 132);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        tick_to_arrival(&mut view);
        assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(132))));
        let arrival = view.message().map(str::to_owned);
        ticks(&mut view, 3);
        assert!(!view.preparing_jump());
        assert_eq!(view.session().expect("flying").jumping(), None);
        assert_eq!(view.message().map(str::to_owned), arrival, "nor refused");
    }

    #[test]
    fn j_held_through_a_jump_does_nothing_after_arrival() {
        let mut view = new_pilot(flight());
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        assert!(view.preparing_jump());
        view.take_sounds();
        tick_to_arrival(&mut view);
        view.take_sounds();
        ticks(&mut view, 3);
        assert!(!view.take_sounds().contains(&REFUSED));
        assert!(!view.preparing_jump());
        assert_eq!(view.session().expect("flying").jumping(), None);
    }
}
