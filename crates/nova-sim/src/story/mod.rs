//! The story runtime: the one design that missions, crons, mission
//! ships, nebulae and plug-in triggers build on. Each kind of story event
//! has one source; the scripts events trigger run through one dispatcher,
//! in one order; days pass through one daily step; and changes to the
//! galaxy go through one overlay that every reader sees.
//!
//! This module holds the design and the pure types it names: the
//! [`StoryEvent`]s, the [`EventFilter`] and [`Subscription`] a consumer
//! registers, each named by its stable [`SubscriptionId`], and the
//! dispatch-order rule, [`dispatch_order`]. The sources raise the events
//! (see "Story events"), and the dispatcher (`dispatch`) runs the
//! subscriptions they match as the session settles (see "Dispatch"). Each
//! section names the phase that builds or fills it:
//!
//! - story events and dispatch: story-runtime phase 3;
//! - the daily step: story-runtime phase 2;
//! - the galaxy overlay: story-runtime phase 4, with `Y` and `U` in
//!   story-runtime phase 5;
//! - the mission lifecycle: missions phase 2;
//! - text substitution: missions phase 3.
//!
//! Where the original cannot settle a question, or the design does not
//! follow it, the section names the [`Rulebook`](crate::Rulebook) key the
//! building phase adds and its readings. Phase 2 adds `stay_days`, phase 3
//! `story_reentry`.
//!
//! # Findings
//!
//! Read from `EV Nova.app/Contents/MacOS/EV Nova` (`otool -tV -arch
//! i386`, `nm`). The active missions are 16 slots of 0x8ec bytes, each
//! with its flags in a parallel record of 0x14 bytes; the stellars are
//! records of 0x498 bytes and the systems of 0x1fc.
//!
//! ## A day (`_IncrementGameTime` @0xb516)
//!
//! One call passes one day, in this order:
//!
//! 1. The date moves on a day (@0xb51f-0xb585).
//! 2. The crons (`_CronEventHandler` @0xb58b): one pass over every `crön`
//!    by ID (@0x38762-0x38912). Each counts down its holdoffs and
//!    duration and ends (`_TerminateCron`, `OnEnd` @0x384dc), or, its
//!    `Random`, date range, `Require` and `EnableOn` met, starts
//!    (`_ActivateCron`, `OnStart` @0x38650). Each script runs at once, so
//!    a later cron in the pass sees the bits an earlier one set.
//! 3. Every active mission's days left (slot +0x48) go down by one while
//!    above 0 (@0xb590-0xb5b6). Nothing fails here: the per-frame goal
//!    check notices a deadline at 0 (see "Ship goals").
//! 4. Tribute (`_DoStellarTimePassage` @0xb5b8): every stellar the player
//!    dominates (+0x46) adds its tribute (+0x46a) to the cash
//!    (@0x40875-0x408ce).
//! 5. The day's events, `öops` (`_DisasterHandler` @0xb5bf).
//! 6. The stellars (@0xb5c4-0xb71e): a dominated one's defence fleet
//!    regrows a ship on `Rand(450)` 0, up to its count (@0xb5f6-0xb64c);
//!    a destroyed one counts down to its regeneration and, at 0, is
//!    restored and runs its `OnRegen` (`spöb` +0x365, @0xb6ee).
//! 7. Every visible system's reinforcement countdown (+0xc4, which
//!    `_HandleReinforcements` sets @0x3929b) goes down by one
//!    (@0xb724-0xb759).
//! 8. Every active rank pays its salary, up to its cap (@0xb75b-0xb7c2).
//! 9. The shipyard, hire and outfitter day rolls (@0xb7c4-0xb833).
//!
//! The commodity price flux (`_HandleGlobalPriceFlux` @0x32054,
//! `_HandleLocalPriceFlux` @0x320a6) and the escorts' pay
//! (`_DoEscortPayment`) are not part of a day: the caller runs them once,
//! after its days.
//!
//! ## Where days pass
//!
//! `_IncrementGameTime` has eight call sites:
//!
//! - a jump's arrival (`_HandlePlayer` @0x6c144): as many days as the
//!   longest hyperspace transit time in the fleet, then the global and
//!   local price flux (@0x6c153, @0x6c158);
//! - the take-off (`_PlayerLandOnStellar` @0x63440): one day, then the
//!   global price flux (@0x63445). A landing passes none: the day comes
//!   as the spaceport is left;
//! - leaving the spaceport having bought or sold an outfit
//!   (`_DoPortDialog` @0x601fa, the flag `_DoOutfitDialog` sets @0x5c555,
//!   @0x5d317): one more day;
//! - leaving it having bought a ship (@0x6020e, the flag
//!   `_DoShipyardDialog` sets @0x5ebce): four more days;
//! - selling or upgrading escorts there (`_DoEscortLand` @0x40849): half
//!   as many days as escorts sold and upgraded, rounded down, then their
//!   pay (@0x4085a);
//! - a mission's success (`_DoMissionSuccess` @0xa056f) and an
//!   auto-aborted mission (`_AutoAbortMission` @0x99ca3): its
//!   `DatePostInc` days;
//! - the escape pod (`_HandlePlayer` @0x6912a): `Rand(30) + 15` days.
//!
//! A stay ends with the escorts (@0x601a0), the outfitting day, the
//! ship's days, then the take-off's day.
//!
//! Checked for phase 2:
//!
//! - `_PlayerLandOnStellar` reaches `_IncrementGameTime` @0x63440 with no
//!   branch past it (@0x631c4-0x63440), whatever services the stellar
//!   has. Its one caller, `_HandlePlayerDockRequest` @0x66f29, sends a
//!   hypergate to `_PlayerEnterHypergate` (@0x66ed6) and a wormhole to
//!   `_PlayerEnterWormhole` (@0x66f0d) first, so the original never takes
//!   off from a gate. A pilot resumed docked at one is the engine's own,
//!   and its take-off passes the day as any other.
//! - `_DoPortDialog` clears both flags as the port opens
//!   (@0x5f93d-0x5f95a). The ship flag is set only once the name prompt
//!   is confirmed (@0x5ebc2-0x5ebce).
//! - The outfit flag is set once Buy or Sell passes its own check
//!   (`_CanBuyOutfitItem` @0x5c087-0x5c0a2; the sellable flag @0x5c5d5),
//!   whatever count follows: a quantity dialog's Cancel gives 0
//!   (`_DoQuantityDialog` @0x57138), the loop runs no time, and the flag
//!   is set all the same (@0x5c555 after buying, @0x5d317 after selling).
//!   A sale sets it as a purchase does.
//! - `_DoEscortLand` counts (sold + upgraded) / 2, truncated
//!   (@0x407ed-0x407fb), passes those days (@0x40849), then always pays
//!   one day's wage (`_DoEscortPayment(1)` @0x4085a), reached also with
//!   nothing sold (@0x40347, @0x405cf).
//! - A `Q` sets `_absquatulateStr` (@0x15bee); `_PortFilter` then ends the
//!   dialog (@0x4f060), and `_DoPortDialog` leaves by the same close
//!   (@0x5fcc6-0x5fcd0, @0x60196, @0x601a0-0x6020e).
//!
//! ## One landing
//!
//! `_HandlePlayerDockRequest`, at a stellar that is not a gate:
//!
//! 1. The offers are rolled first (@0x66d3f-0x66d62): 1000 `Rand(100) +
//!    1` rolls, then `_RandomizeMissionData`, which refreshes the galaxy
//!    first (@0xa175f; see "Visibility and governments"). They are rolled
//!    only at a stellar other than the one they were last rolled at
//!    (@0x66d2b-0x66d3d). A jump's arrival forgets that stellar
//!    (@0x6bfc9), as does any mission's abort, failure or success
//!    (@0x99bca, @0x99db0, @0xa03ea, @0xa068c). A take-off does not, so
//!    landing again on the same stellar keeps its offers.
//! 2. The spaceport opens (`_PlayerLandOnStellar` @0x66f29,
//!    `_DoPortDialog` @0x63381) and runs `_MissionLandCheck` (@0x5fbeb),
//!    then redraws the system's stellars (@0x5fbf0), then offers the
//!    spaceport's own missions (`_OfferMissionFromPort` @0x5fc37).
//! 3. `_MissionLandCheck` (@0xa19cc) takes the active missions in slot
//!    order, 0 to 15 (@0xa19f7-0xa1aec). For each, it checks the ship
//!    goals (`_MissionObjectivesCheck` @0xa1a0c) and the cargo
//!    (`_MissionLandCargoCheck` @0xa1a1b); then, landed at the mission's
//!    return stellar, failure (`_DoMissionFailure` @0xa1a92) before
//!    success (`_DoMissionSuccess` @0xa1ace); then the goals again
//!    (@0xa1ada). After the last slot it refreshes the galaxy (@0xa1af2)
//!    and, if any mission succeeded, rolls the offers again (@0xa1b04).
//! 4. The news is chosen as the bar opens (`_DoBarDialog` @0x48f15,
//!    `_RandomizeNewsDialog` @0x48238): it has no trigger point of its
//!    own.
//!
//! ## One take-off
//!
//! As the spaceport closes: the escorts (@0x601a0), then the outfitting
//! and ship days (@0x601fa, @0x6020e). Then `_PlayerLandOnStellar` passes
//! the take-off's day (@0x63440), runs the global price flux (@0x63445),
//! explores the system (`_AutoSetExploration` @0x634c8) and its nebulae
//! (@0x634da), restarts each mission's ship hail timers
//! (@0x634df-0x6353c) and saves the pilot (@0x6354b). Back in
//! `_HandlePlayerDockRequest`, the system's ships are set up
//! (`_SetupShipsInSystem` @0x674c2) and its stellars redrawn (@0x674c7).
//! The enter-system hook does not run.
//!
//! ## One arrival
//!
//! `_HandlePlayer`'s jump arrival, in order: the nebulae of every system
//! the jump passes and arrives at (`_ExploreNebulaeFromSystem` @0x6bddb,
//! @0x6be12); the galaxy refresh (@0x6bf3f); the offers' stellar
//! forgotten (@0x6bfc9); the days (@0x6c144); the global and local price
//! flux (@0x6c153, @0x6c158); the enter-system hook
//! (`_MissionHandlePlayerEnteredNewSystem` @0x6c15d); the system explored
//! (@0x6c1db); its ships set up (@0x6c1f9) and a bounty hunter
//! (@0x6c1fe); its stellars redrawn (@0x6c203); the escorts' pay
//! (@0x6c620).
//!
//! The enter-system hook (@0x99f11) runs no script. For each active
//! mission, in slot order, it restarts the timers its ships appear and
//! hail by (+0x4e, +0x6c, +0x6a). It runs on a jump's arrival
//! (@0x6c15d), a gate's or wormhole's exit (@0x67159) and an `M` or `N`
//! in flight (@0x159a8); not on a landed `M` or `N` (@0x15947, which only
//! empties the docked offers), nor on a take-off, which restarts only the
//! hail timers.
//!
//! ## Ship goals
//!
//! Goals are counted where they happen and checked by polling. A mission
//! ship destroyed is counted in `_HandleShipDisplay` (@0x2dbd4), disabled
//! in `_DamageShip` (@0x3af8b), boarded in `_HandlePlayerBoardAttempt`
//! (@0x653a7, @0x65730). A ship whose loss fails its mission fails it
//! there and then (`_QuickMissionFailure` @0x2dbba, @0x3af71). Every frame
//! `_DoPlayGameWork` (@0x44214) runs `_MissionHandler` (@0x9eda8), which
//! checks each slot's goals (`_MissionObjectivesCheck` @0x9edb5): the
//! counts, whether the escorted ships are still with the player
//! (@0x9e931-0x9e980), and whether an observed ship is on screen and
//! visible (@0x9ea5c, @0x9eb02). A goal met shows `ShipDone` (@0x9ed0f)
//! and runs `OnShipDone` (@0x9ed70). The same poll fails a mission whose
//! days left reached 0 (@0x9eba5-0x9ec61), so a deadline fails after its
//! transition has finished. The landing check runs it too (above).
//!
//! Mission cargo jettisoned (`_JettisonCargo` @0x3e18c), found by a scan
//! (`_ScanPlayer` @0x7e7ac) or plundered by an NPC
//! (`_AIPlunderShipContents` @0x84a0f) fails its mission at once.
//!
//! ## Scripts that raise events
//!
//! The original nests: an operator that causes an event handles it inside
//! `_EvalSetExp` (@0x150fc), before the next operator runs.
//!
//! - `S` rolls and accepts the mission there (`_RandomizeOneMission`
//!   @0x153ff, `_DoMissionAccept` @0x15407), running its `OnAccept`
//!   (@0xa1da2) and, for an auto-aborted mission, its `DatePostInc` days
//!   (@0xa2186), crons and all.
//! - `A` aborts the mission there (`_AbortMission` @0x1532e), running its
//!   `OnAbort` (@0x99ba1). `F` only marks it failed (@0x1539f): it fails
//!   at the next landing check.
//! - `M` and `N` in flight redraw the stellars and run the enter-system
//!   hook there (@0x159a3, @0x159a8).
//! - `K` and `L` activate and deactivate the rank there (@0x159e1,
//!   @0x15a17); that runs no script.
//!
//! Outside scripts it is the same: a mission's success passes its
//! `DatePostInc` days, crons and all, inside the landing check.
//!
//! ## Visibility and governments
//!
//! `_PropagateMissionBitEffects` (@0x99676) is the one place the bits
//! reach the galaxy. It evaluates every system's `Visibility`
//! (@0x996a1-0x996eb); gives each stellar to the lowest-ID visible system
//! that lists it (@0x9970d-0x997bd); evaluates `shïp` `Availability`,
//! `përs` and `flët` `ActiveOn`, `mïsn` `AvailBits` and `nëbu` `ActiveOn`
//! (@0x997c3-0x998ef); and, if the player's system is now hidden, moves
//! the player and every ship in the system to the visible system at the
//! same place or, with none, makes the first system at that place (the
//! chain head) visible and moves them there (@0x998f1-0x99a36; see
//! "Checked for phase 4"). It runs
//! at a landing (with the offers @0xa175f, in the land check @0xa1af2), on
//! a jump's arrival before its days (@0x6bf3f), and for a new pilot, the
//! main screen and a load; never right after a script. So a bit a cron
//! sets on a jump's days shows at the next landing or arrival.
//!
//! Between refreshes every reader reads the visible flags it left
//! (`_FindActiveCoLocatedSystem` @0x4be0; `_SystemVisible` @0x98207 has
//! no caller). A system's government changes only when another system at
//! the same place becomes visible; nothing re-reads governments on their
//! own. A stellar's destroyed state is read live (`_StellarIsDestroyed`
//! @0x4e14, by its sprite, weapons, shots and the AI).
//! `_HandleStellarSystemVisibility` (@0x32aa5) only rebuilds the current
//! system's stellar sprites by it: after a move in flight (@0x159a3), on
//! opening the spaceport (@0x5fbf0), on a gate's exit (@0x63e8c,
//! @0x6473e), after a take-off (@0x674c7) and on arrival (@0x6c203,
//! @0x6c597).
//!
//! ## Nebulae
//!
//! `_ExploreNebulaeFromSystem` (@0x508f) runs the `OnExplore` (@0x519e)
//! of every active nebula not yet explored whose rectangle, inset by 8,
//! holds the system, and marks it explored, so each runs once. It runs
//! for every system an exploration reaches (`_RecursiveAutoExplore`
//! @0xd98a: an arrival, a take-off, a map granted), for the systems a
//! jump passes and arrives at (@0x6bddb, @0x6be12), at the take-off
//! (@0x634da), and for a new pilot and a load (@0x19118, @0x765bc). `X`
//! explores no nebula: it writes the explored level itself (@0x15c71),
//! and `_EvalSetExp` (0x150fc up to `_EvalTestExp`) calls none of these.
//! A `G` of a map does, through `_GrantOutfitItem` (@0x15444).
//!
//! ## Text
//!
//! `_MungeBriefing` fills the text tags of every mission text (offers,
//! briefings, accept, success, failure, `ShipDone`, cargo), of a `Q`
//! message run from a mission's script (@0x15c4b), of a person's hail
//! quote (@0x44614) and of the comm dialog (@0x9080b). The news fills
//! none.
//!
//! # Story events
//!
//! Built by story-runtime phase 3.
//!
//! A [`StoryEvent`] says what happened, with its data. It is the sim's
//! own: story dispatch never drains or reuses the view's
//! [`SessionEvent`](crate::SessionEvent) outbox, which carries no data and
//! is emptied by [`Session::take_events`](crate::Session::take_events).
//! Each event has one source, at a transition point that also emits a
//! `SessionEvent` or runs a step:
//!
//! | Event | Its one source | The original's |
//! |---|---|---|
//! | [`EnteredSystem`](StoryEvent::EnteredSystem) | `enter_system`'s `announce` step, for every [`Entrance`] | the enter-system hook |
//! | [`Landed`](StoryEvent::Landed) | `Session::dock`, as [`Session::land`](crate::Session::land) docks | the spaceport opening |
//! | [`TookOff`](StoryEvent::TookOff) | [`Session::take_off`](crate::Session::take_off), after its day | `_PlayerLandOnStellar` after the spaceport |
//! | [`DayPassed`](StoryEvent::DayPassed) | the daily step, as its last participant | `_IncrementGameTime` |
//! | [`Explored`](StoryEvent::Explored) | exploring a system: a jump's arrival, for each system it passes and the one it arrives at; every other way in whose row explores; a take-off; a map granted, for each system it reaches; [`Session::fly`](crate::Session::fly), for its system. `X` raises none | `_ExploreNebulaeFromSystem` |
//! | [`ShipFate`](StoryEvent::ShipFate) | combat's destruction and disabling (a ship broken up while intact is disabled too), the session's boarding, repelled or not, and the traffic's flight tick for a watched ship first seen ([`Session::watch_ship`](crate::Session::watch_ship)) | the goal counts and the per-frame poll |
//!
//! A consumer that wants only some entrances reads the [`Entrance`] in
//! its subscription: mission ships restart their timers on a `Jump`, a
//! `Gate` and an in-flight `ScriptMove`, as the original's hook runs. A
//! take-off raises `DayPassed`, its `Explored`, then `TookOff`. A jump's
//! arrival raises the `Explored` of the systems it passes and of the one
//! it arrives at, its `DayPassed` events, then `EnteredSystem`: the
//! original explores those nebulae before the days (@0x6bddb, @0x6be12;
//! see "Checked for phase 3"). Every other way in raises its `DayPassed`
//! events, if any, `EnteredSystem`, then `Explored` when its row
//! explores. Each exploration raises `Explored`, the system explored
//! already or not. Flying a pilot, a new one or a load, raises its
//! system's `Explored`, as the original explores its nebulae there
//! (`_DoNewPilot` @0x19118, `_OpenLastPilotFile` @0x765bc).
//!
//! The events wait on the session, in the order raised, for its next
//! [`Session::settle_script`](crate::Session::settle_script), which
//! dispatches them (see "Dispatch").
//!
//! A new trigger kind adds an event variant and its one source, plus
//! subscriptions; never a loop of its own. The plug-in time-in-system and
//! proximity kinds (plugin-scripting-extensions phase 4) are new variants
//! raised from the flight tick.
//!
//! # Dispatch
//!
//! Built by story-runtime phase 3.
//!
//! A [`Subscription`] is a consumer's "on this event, if this test holds,
//! run this set expression, maybe once":
//!
//! - its [`EventFilter`]: a kind, or a kind at one place;
//! - its guard, a control-bit [`Test`] asked through
//!   [`Gate`](crate::control::Gate). A guard of more than one test is a
//!   [`Checks`](crate::rules::Checks) list that the subscription's
//!   consumer runs, never the dispatcher;
//! - its set expression, a [`Script`], run as
//!   [`Session::run_set`](crate::Session::run_set) runs one, its operators
//!   applied through `Session::apply_op`;
//! - its once-only flag;
//! - its stable [`SubscriptionId`]: consumer, record and hook. The record
//!   is the consumer's number for what it subscribes for: a resource ID,
//!   or for missions and their ships the active mission's slot, which is
//!   saved with it. The id keys the once-fired record saved on `Pilot`
//!   (save version 16), so it never depends on registration order. Ids
//!   are unique: the registry refuses a second
//!   ([`DuplicateSubscription`]).
//!
//! **Registering and unregistering.** A consumer registers through
//! [`Session::register_story`](crate::Session::register_story) and
//! unregisters through
//! [`Session::unregister_story`](crate::Session::unregister_story), which
//! clears the once-fired entry too. The stock consumers will register at
//! [`Session::fly`](crate::Session::fly) from their catalog data; none
//! does yet. A mission's subscriptions, and its ships', are registered
//! when it becomes active, keyed by its slot. When it ends (success,
//! failure, `A` or an auto-abort), they are unregistered and their
//! once-fired entries cleared, so a mission accepted into the reused slot
//! starts fresh.
//!
//! **One dispatch.** A dispatch takes a snapshot of the matching ids, in
//! [`dispatch_order`]'s order; an event no subscription matches costs
//! nothing. For each id in turn, just before it runs, the dispatcher
//! skips it when it is no longer registered (a subscription removed by an
//! earlier one in the same dispatch), when it is once-only and has fired,
//! or when its guard does not hold now (an earlier script can open or
//! close it). Otherwise a once-only subscription is marked fired first,
//! so an event raised again inside its own run cannot run it twice; then
//! its consumer's effect runs, then its script. One registered during the
//! dispatch waits for the next event.
//!
//! **Order.** [`dispatch_order`] runs an event's subscriptions by id:
//! consumer, then record, then hook. The consumers' order is
//! [`Consumer`]'s, the same for every event, because only one event has
//! two stock consumers in the original: a landing, which checks each
//! mission's ship goals before its cargo, failure and success
//! (@0xa1a0c-0xa1ace), so [`Consumer::MissionShips`] comes first. Missions
//! go by slot, as the original's land check (@0xa19f7), and a mission's
//! landing hooks go cargo, failure, success. Plug-ins run after every
//! stock consumer: an engine decision with no Rulebook key, since the
//! original has no plug-ins. One difference stays: the original checks
//! goals and then cargo, failure and success slot by slot, and here every
//! slot's goals come first. Goal checks read only their own mission's
//! counts, so the two differ only when a success script changes another
//! mission's ships.
//!
//! **Re-entrancy.** The Rulebook key
//! [`story_reentry`](crate::RuleKey::StoryReentry) has two readings. Its
//! default is the original's: **nested**. An event a script's operator
//! raises is dispatched depth first, inside the script run, before the
//! script's next operator, as `_EvalSetExp` (@0x150fc) does (see "Scripts
//! that raise events"):
//!
//! - `S` accepts the mission and runs its `OnAccept` before the next
//!   operator (`_DoMissionAccept` @0x15407, `OnAccept` @0xa1da2), with an
//!   auto-aborted mission's `DatePostInc` days, crons and all (@0xa2186);
//! - `A` aborts the mission and runs its `OnAbort` there (@0x1532e,
//!   @0x99ba1);
//! - `M` and `N` in flight enter the system, redraw its stellars and run
//!   the enter-system subscriptions there (@0x159a3, @0x159a8).
//!
//! **Where it nests.** [`Session::run_set`](crate::Session::run_set)
//! stays catalog-free: `apply_op` only queues a move or a `Q`, since a
//! script has no catalog to read the new system from (see
//! `script_effects`). The one place that dispatches is
//! [`Session::settle_script`](crate::Session::settle_script), which holds
//! the catalog and which the program calls after every input and tick.
//! The landing, the take-off, the daily step, combat and boarding have no
//! catalog, so the events they raise wait there. It settles in a loop:
//! the moves queued (and the `EnteredSystem` an `enter_system` they cause
//! raises), then the next event raised, first in, first out, dispatched,
//! and once none is left a `Q`'s leave, whose events dispatch in the same
//! settling. The subscriptions' scripts run through the dispatcher's
//! runner, which, after each operator handed to `apply_op`, settles the
//! moves it queued, dispatches the events raised, depth first, and only
//! then runs the next operator. So for the scripts the dispatcher runs,
//! the original's nesting in the middle of a script is matched. A `Q`
//! only sets the message (@0x15bee), as the original's does, so it is
//! never settled in the middle of a script.
//!
//! A script run through `run_set` in the middle of something else defers
//! instead. Those are the record hooks
//! [`Session::run_script`](crate::Session::run_script) runs (see the
//! `hooks` module):
//!
//! - `chär` `OnStart`, in [`Session::begin`](crate::Session::begin);
//! - `oütf` `OnPurchase` and `OnSell`, in
//!   [`Session::outfit`](crate::Session::outfit);
//! - `shïp` `OnRetire` and `OnPurchase`, in
//!   [`Session::buy_ship`](crate::Session::buy_ship);
//! - `shïp` `OnCapture`, and `OnRetire` on Use As My Ship, in
//!   [`Session::plunder`](crate::Session::plunder) and
//!   [`Session::assign`](crate::Session::assign).
//!
//! `C`, `E` and `H` change ships and run no hook. The deferred effects
//! queue. At the caller's next safe point, its `settle_script`, they
//! settle, and the events they raise dispatch, nested from there. So
//! under the default, nesting means depth first from the nearest safe
//! point.
//!
//! The other reading is **queued**, the port's earlier after-settle
//! order. An event raised by a script waits until the current dispatch
//! has finished and the script has settled, then dispatches in the order
//! raised, first in, first out; never in the middle of a script:
//!
//! 1. a dispatch runs its subscriptions' scripts, in order;
//! 2. `settle_script` applies the queued moves, and an `enter_system` they
//!    cause raises `EnteredSystem`;
//! 3. the dispatcher takes the next pending event, that one among them;
//! 4. and so on until none is pending.
//!
//! Under either reading at most 64 events are dispatched from one
//! settling, nested runs included, a guard against scripts that raise
//! each other for ever. Only an event some subscription matches counts.
//! The rest are dropped, and the settling tells how many as one
//! [`ScriptNote::EventsDropped`](crate::ScriptNote::EventsDropped). The
//! next settling starts a fresh count.
//!
//! A consumer's own effects, which are not set expressions (a mission's
//! pay, legal record and `DatePostInc` days, a ship goal's count), are
//! routed by the subscription's `id.consumer` through one match in
//! `session/story.rs`, as `apply_op` routes operators. The dispatcher, the
//! events and the order stay out of [`rules`](crate::rules): at most, a
//! guard is a rule.
//!
//! # Daily step
//!
//! Built by story-runtime phase 2 (`day`); each participant by the phase
//! named.
//!
//! One step passes one day. Its participants, `day::Participant`, run in
//! the original's order, `Participant::ORDER` (see "A day"), through the
//! `day::Day` port the session implements (`session/story.rs`). Phase 2
//! fills the date and the events, and phase 3 `DayPassed`; the others are
//! empty slots until their phase:
//!
//! 1. the date moves on a day;
//! 2. the crons, each starting or ending cron's script run as it is
//!    reached (crons-news-ranks phase 1);
//! 3. the active missions' days left (missions phase 2);
//! 4. the dominated stellars' tribute (rdm task planet-domination);
//! 5. the day's events, `öops`: today's `market::step_day`;
//! 6. the stellars: a dominated one's defence regrowing (planet-domination)
//!    and a destroyed one's regeneration and `OnRegen` (story-runtime
//!    phase 5);
//! 7. the reinforcement countdown, once reinforcements are built;
//! 8. the ranks' salaries (crons-news-ranks phase 3);
//! 9. the day rolls (`wares::DayRolls`): not filled yet, so the session
//!    still draws the shop and hire rolls afresh at each landing;
//! 10. `DayPassed`, raised with the new date.
//!
//! Plug-in daily triggers (plugin-scripting-extensions phase 4) and a
//! mission's missed deadline (missions phase 2) are `DayPassed`
//! subscriptions, so they run after every stock participant: the
//! deadline after the transition, as the original's per-frame check
//! notices it. News has no participant: it is chosen when the bar opens.
//!
//! Days pass through the step on every path the original passes them on:
//!
//! - a jump's arrival, its days (`pass_jump_days`), and a gate's under
//!   [`GateArrivalRule::LikeJump`](crate::GateArrivalRule::LikeJump);
//! - leaving the spaceport ([`Session::take_off`](crate::Session::take_off)),
//!   as `day::Stay`, the stay's tally, counts: half as many days as
//!   escorts sold or upgraded, rounded down; then a day if an outfit was
//!   bought or sold, four if a ship was bought, and every take-off's own
//!   day. A `Q` that ends the stay leaves the same way;
//! - a mission's success or auto-abort: its `DatePostInc` (missions
//!   phase 2).
//!
//! The stay's days follow the Rulebook key `stay_days`: by the engine,
//! as above; otherwise a stay passes no days, the port's earlier
//! behaviour. Nothing raises the escorts' count yet: the escort dialog's
//! Sell and Upgrade (rdm task escorts-follow-up) will. The escape pod's
//! days wait for the escape pod. `Session::set_date` and loading a save
//! write the date directly, with no participant run.
//!
//! Escort pay and the price flux are not participants: they run once
//! after a transition's days, as the original's do. `pass_jump_days`
//! pays the wages once after its days, and
//! [`RuleKey::TakeOffPay`](crate::RuleKey::TakeOffPay) pays one day's wage
//! at each take-off, after the escorts' days and before the others, as
//! `_DoEscortLand` pays (@0x4085a), whatever the days. The engine has no
//! price flux: `market::step_day` steps only the `öops` events, which move
//! prices, as the fifth participant.
//!
//! # Galaxy overlay
//!
//! Built by story-runtime phase 4; `Y` and `U` by story-runtime phase 5;
//! the visibility rules by crons-news-ranks phase 5; ownership by rdm
//! task planet-domination.
//!
//! The overlay holds what play changes over the static catalog:
//!
//! - per system: whether it is visible, and its government;
//! - per stellar: whether the player dominates it, whether it is
//!   destroyed or regenerated, and the system it belongs to (the
//!   lowest-ID visible system that lists it, which follows from
//!   visibility);
//! - per nebula: whether it is active (`nëbu` `ActiveOn`), so that the
//!   map reads nebulae as it reads systems.
//!
//! Readers see these facts only through the overlay: hyperspace links and
//! navigation, traffic, landing, legal records, AI targeting, exploration,
//! and every `StarMap::govt` call. The galaxy map keeps its static
//! geometry (positions, nebula rectangles) from nova-view's
//! `GalaxyCatalog`, and reads visibility, government, links and nebula
//! activity through a session accessor over the overlaid galaxy.
//!
//! An override written directly (`Y`, `U`, domination) applies at once.
//! Visibility and nebula activity are evaluated from the bits at the
//! original's refresh points: at a landing, as the offers are rolled and
//! after the land check; on a jump's arrival, before its days; and for a
//! new pilot and a load. Crons-news-ranks phase 5 adds the Rulebook key
//! `visibility_refresh` for the other reading, after every settled
//! script. Each change to the overlay raises one `SessionEvent`, on which
//! the view rebuilds the galaxy map. A player whose system becomes hidden
//! goes to the visible system at the same place, with every ship in the
//! system, or, with none, to the first system at that place, made
//! visible: story-runtime phase 4's rule (see "Built by phase 4").
//! The original changes a system's government only by showing another
//! system at the same place; a government override is the engine's own.
//! `shïp` `Availability` and `përs`, `flët` and `mïsn` availability stay
//! live reads, outside the overlay, where the original keeps them until
//! the next refresh.
//!
//! ## Built by phase 4
//!
//! The overlay is [`GalaxyOverlay`], saved on the
//! pilot (save version 17), and the session reads the galaxy only through
//! [`OverlaidGalaxy`]: the static catalog read
//! once, the overlay, and the star map built from both, rebuilt only when
//! the overlay changes. Its map has the visible systems alone, each link
//! to a hidden system resolved to the visible system at its place, and
//! each system's government as overridden. Every `StarMap::govt` the
//! session reads is that map's, and every stellar the session reads
//! passes through [`OverlaidGalaxy::site`],
//! so landing, the market and the outfitter see a stellar's government
//! override. With no overrides the galaxy is exactly the static catalog.
//!
//! [`Session::change_galaxy`](crate::Session::change_galaxy) is the one
//! writer. A change applies the moment it returns; when it is called is
//! the writer's choice, and the visibility and nebula writers
//! (crons-news-ranks phase 5) call it only at the refresh points above.
//! Each reader sees a change when it next reads:
//!
//! - hyperspace links, routes, Hyper Select and the next-jump check: at
//!   the next query, as every Con read resolves the visible flags afresh;
//! - the AI's system government: from the next traffic tick, as the
//!   original's warships read the system's `Govt` at each decision;
//! - traffic: arrivals at the next spawn, from the table the change
//!   replaced, as the original's per-frame spawner reads the current
//!   system's record afresh; the initial population at the next entry or
//!   take-off;
//! - the galaxy map: after the `GalaxyChanged` event.
//!
//! A player whose system becomes hidden moves as the original moves it
//! (@0x998f1-0x99a36): to the lowest-ID visible system at the same place,
//! or, with none, to the chain head (the lowest-ID system at that place),
//! made visible. It keeps its position, landed state, stellar and course,
//! and every ship in the system stays with it; it raises no story event,
//! explores nothing and passes no day. The course keeps its system IDs,
//! each hop resolved by place when it is used, as `_CueNextHyperRouteDest`
//! resolves it.
//!
//! The player is always in a visible system. Every arrival (a jump, a
//! gate's exit, `M` and `N`, and the developer tools' relocation) resolves
//! its system by the same rule before it enters. For a jump and a gate
//! this is the original's; for `M` and `N` it is the engine's own: the
//! original writes the system given (@0x158f2) and leaves the player in a
//! hidden system until the next refresh moves it. A government override
//! is the engine's own too, with no Rulebook key: the original changes a
//! government only by showing another system at the same place.
//!
//! Two of the original's readings are kept the engine's way, both only
//! where more than one system at a place is visible at once, which stock
//! data never has: `_FindActiveCoLocatedSystem` gives the lowest-ID
//! visible system at a place even for a visible system (@0x4c02-0x4c40),
//! where the engine keeps a visible system itself; and a link reaches
//! every visible system at its target's place, as the engine's
//! [`HyperlinkRule`](crate::HyperlinkRule) has always read it.
//!
//! # Mission lifecycle
//!
//! Built by missions phase 2.
//!
//! A mission is offered (per visit), then active, in one of 16 slots
//! saved on `Pilot`, until it succeeds, fails or is aborted. What moves
//! it:
//!
//! - the player: accepting runs `OnAccept` (and ends an auto-aborted
//!   mission at once), refusing `OnRefuse`, aborting `OnAbort`;
//! - `Landed`: the missions' landing subscriptions, after the mission
//!   ships' goal checks, in `_MissionLandCheck`'s order: by slot, cargo
//!   (missions phase 5), failure, success. A success pays, applies the
//!   legal record, passes its `DatePostInc` days through the daily step,
//!   runs `OnSuccess` and has the offers rolled again;
//! - the daily step: the days left count down; a `DayPassed`
//!   subscription fails a mission whose days left reached 0, running
//!   `OnFailure`;
//! - `ShipFate`: mission-ships phase 2 counts the goals, and a loss that
//!   fails the mission fails it at once, through this lifecycle; cargo
//!   jettisoned, found or plundered does the same (missions phase 5);
//! - `S`, `A` and `F`, through `apply_op`: `S` accepts, running `OnAccept`
//!   there as the original does, any event the accept raises being
//!   dispatched as `story_reentry` says, nested by default (see
//!   "Dispatch"); `A` aborts there; `F` marks the mission
//!   failed, and it fails at the next landing check.
//!
//! `EnteredSystem` moves no mission: the original's enter-system hook only
//! restarts its ships' timers (mission-ships phase 1). Offers are rolled
//! as the player lands, before the spaceport opens, and kept until a
//! landing at another stellar, a jump or a mission's end (missions
//! phase 1).
//!
//! # Text substitution
//!
//! Built by missions phase 3.
//!
//! The shared module missions phase 3 extracts from
//! [`expand_tags`](crate::person::expand_tags) is the one substitution
//! path, as `_MungeBriefing` is the original's. Its users:
//!
//! - mission text;
//! - a `Q` message run from a mission's script;
//! - person hail quotes, as today;
//! - mission ships' `ShipDone` (mission-ships phase 2);
//! - news (crons-news-ranks phase 2), which the original fills with no
//!   tags: a text with none comes out unchanged;
//! - dialogue nodes (plugin-scripting-extensions phase 3).
//!
//! ## Checked for phase 3
//!
//! - The firing points stand as "One landing", "One take-off" and "One
//!   arrival" give them: the enter-system hook after the days and the
//!   price flux and before the exploration (@0x6c144, @0x6c153, @0x6c158,
//!   @0x6c15d, @0x6c1db), at a gate's exit (@0x67159) and an `M` or `N`
//!   in flight (@0x159a8), not a landed one (@0x15947); the land check
//!   after the dock (@0x5fbeb); the take-off's day, then its exploration
//!   (@0x63440, `_AutoSetExploration` @0x634c8, which reaches
//!   `_ExploreNebulaeFromSystem` through `_RecursiveAutoExplore` @0xd98a,
//!   then @0x634da).
//! - A jump explores the nebulae of the system it arrives at before its
//!   days on every jump, a single one too: with no multi-jump, or a route
//!   that does not match, the branches at @0x6bd83 and @0x6bd9d go to
//!   @0x6be85, which joins @0x6be08 and calls @0x6be12 with the arrival
//!   system. The exploration after the hook (@0x6c1db) then finds its
//!   nebulae explored.
//! - The gate's exit (`_PlayerEnterHypergate` @0x637bf,
//!   `_PlayerEnterWormhole` @0x64005) calls neither `_AutoSetExploration`
//!   nor `_ExploreNebulaeFromSystem`: the original's gate explores
//!   nothing, where the engine's gate row in the session's `entry` table
//!   explores the system. That divergence is the engine's, kept for now
//!   (rdm task gate-exit-explores).
//! - A ship broken up while intact counts as disabled: `_DamageShip`
//!   notes whether the ship was disabled before the hit (`_IsDisabled`
//!   @0x3a9ad), and after it counts a mission ship disabled when
//!   `_IsDisabled` holds and it was not (@0x3ae5b-0x3ae90, @0x3af8b).
//!   `_IsDisabled` (@0x2ce6) compares the armour with a share of its
//!   most, so a ship whose armour is gone is disabled by it, and the kill
//!   path (@0x3ade4-0x3ae53) falls through to that count.
//! - Both boarding counts (@0x653a7 for a mission's cargo pick-up,
//!   @0x65730 for its boarding goal) come once the board has passed its
//!   checks (@0x6502b-0x65298) and before the boarding goes on
//!   (@0x6578f), so whatever follows, the plunder or the crew repelling
//!   the boarders, a board counts. A ship boarded already cannot be
//!   boarded again.
//! - `_ShipVisibleToShip` (@0x971c) holds a ship invisible while it is
//!   emerging from a hypergate (`_AIShipIsEmergingFromHypergate`
//!   @0x972d), and a cloaked one (its fade level, +0x64) visible only to
//!   a cloaking ship of its own side (`_HasCloak` @0x979b) or one close
//!   enough on both axes (@0x97ae-0x97f8); every other ship is visible.
//!   The engine has neither a cloak nor a ship emerging from a gate, so a
//!   watched ship in the system is visible.
//! - `S` (`_DoMissionAccept` @0x15407), `A` (`_AbortMission` @0x1532e)
//!   and `M` and `N` in flight (@0x159a3, @0x159a8) are handled inside
//!   `_EvalSetExp`, before its next operator. `Q` only sets
//!   `_absquatulateStr` (@0x15bee); nothing between it and @0x15c6c leaves
//!   the stellar.
//!
//! ## Checked for phase 4
//!
//! - **The co-located chain.** `_LoadObjectData` builds it once
//!   (@0x783d9-0x78517): every system's chain head (+0xba) and next link
//!   (+0xb8) start empty; then, by ascending ID, a system with no head
//!   becomes its own head and every higher-ID system at the same map
//!   position (+0x0, both coordinates) joins its chain, in ascending ID
//!   order. So the head is the lowest-ID system at a place.
//!   `_FindFirstCoLocatedSystem` (@0x4baa) gives the head, or the system
//!   itself without one; `_FindActiveCoLocatedSystem` (@0x4be0) walks the
//!   chain from the head and gives the first visible system (+0x1eb), the
//!   lowest-ID one at the place, or -1. It does so for a visible system
//!   too (@0x4c02-0x4c40), so of two visible systems at one place the
//!   higher is never reached through it.
//! - **A hidden player's system** (@0x998f1-0x99a36). When the player's
//!   system (+0x74) is hidden, the player moves to
//!   `_FindActiveCoLocatedSystem` (@0x99929) or, with none, to
//!   `_FindFirstCoLocatedSystem` (@0x9993f), which is made visible first
//!   (@0x999a4). The move rewrites system numbers only: the player
//!   (@0x999bf); every ship slot 1-63 in use (+0xb8, @0x999c3-0x999e0),
//!   which is every ship in the system: the spawns set it
//!   (`_RandomShipSpawn`, `_HyperShipSpawn`, `_SetupShipsInSystem`) and
//!   `_DeleteAllComputerShips` and a ship's leaving clear it; the shots
//!   in flight (the 128 records of 0x48 bytes at 0x3b5474, which
//!   `_SpawnShot` fills, @0x999e2-0x99a0d); and the cargo boxes (the 64
//!   of 0x28 at 0x3b52c0, which `_SpawnBox` fills, @0x99a0f-0x99a36). It
//!   sets up no ships, runs no hook, explores nothing and redraws no
//!   stellars. So every NPC stays with the player, not the escorts alone,
//!   and no `galaxy_escorts` key is needed.
//! - **The course** keeps system IDs. `_CueNextHyperRouteDest` resolves
//!   the next hop by place (`_FindActiveCoLocatedSystem` @0xe2fa) and
//!   matches it against the current system's Con slots, each resolved the
//!   same way (@0xe32f): a hidden hop with a visible system at its place
//!   is followed there, and one with none is not cued.
//! - **A stellar's system.** `_FindSystemFromStellar` (@0x7ec1-0x7f48)
//!   gives the lowest-ID visible system whose `NavDefs` list the stellar,
//!   else the lowest-ID system that lists it.
//! - **Arrivals.** A jump resolves its hops by place
//!   (`_FindActiveCoLocatedSystem` @0x6bd67, @0x6bd7b, @0x6bdc1). A
//!   hypergate's exit takes the stellar's system (`_FindSystemFromStellar`
//!   @0x638e4) and resolves it by place (@0x638ed), and so does a
//!   wormhole's (@0x64085, @0x64155, @0x64204). `M` and `N` write the
//!   system they are given (@0x158f2) and resolve nothing.
//! - **Traffic over time** reads the current system afresh at each spawn.
//!   Every frame `_DoPlayGameWork` calls `_EnterMoreShips` with the
//!   player's system (@0x44228-0x44237), which reads that system's record
//!   (@0x43b47-0x43b64), its fleet chance and `_SelectFleetFromSystem`
//!   (@0x43b7c-0x43bad), and its dude fields
//!   (`_SelectDudeFieldFromSystem` @0x43bed) at each roll. So arrivals
//!   after a change draw from the new system's table at once, and no
//!   `galaxy_traffic` key is needed.
//! - **The AI reads the system's government live.** `_SelectWarshipTarget`
//!   reads the player's system's `Govt` (+0x8) at each decision: for a
//!   xenophobe (@0x8a15f-0x8a189) and for the legal table
//!   (@0x8a757-0x8a768, @0x8a7f1-0x8a803). Ships already in the system
//!   see a change from their next decision; each keeps its own
//!   government.
//! - **The map.** `_DrawMap` draws links only from visible systems
//!   (+0x1eb @0xe9f0), each Con slot resolved through
//!   `_FindActiveCoLocatedSystem` and skipped when none is visible
//!   (@0xea31-0xea4d), and draws only visible systems (@0xed67).
//!   `_MapDialogUpdate` draws a nebula only while it is active (+0x8,
//!   which `_PropagateMissionBitEffects` writes @0x998da) and explored
//!   (+0x9) (@0xf947-0xf95f).

pub(crate) mod day;
pub(crate) mod dispatch;
pub mod galaxy;

pub use dispatch::DuplicateSubscription;
pub use galaxy::{GalaxyOverlay, HiddenMove, OverlaidGalaxy, StellarOverride, SystemOverride};

use crate::catalog::{StellarId, SystemId};
use crate::control::{Script, Test};
use crate::date::GameDate;
use crate::traffic::npc::NpcId;

/// How the player entered a system. The original's enter-system hook
/// runs on a `Jump`, a `Gate` and a `ScriptMove` in flight, never a
/// landed one (see the module docs' findings).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entrance {
    /// A hyperspace jump's arrival.
    Jump,
    /// Out of a hypergate or wormhole.
    Gate,
    /// `M` or `N`, landed or in flight.
    ScriptMove {
        /// Whether the player was landed.
        landed: bool,
    },
    /// The developer tools' move, landed.
    Relocate,
}

/// What became of a ship: what a mission ship's goal counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fate {
    /// Destroyed.
    Destroyed,
    /// Disabled.
    Disabled,
    /// Boarded by the player.
    Boarded,
    /// First seen by the player: on screen and visible to it, which the
    /// original polls each frame and the flight tick raises here.
    Observed,
}

/// Something that happened that story scripts react to, with its data,
/// raised from its one source (see the module docs' "Story events").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoryEvent {
    /// The player entered `system`.
    EnteredSystem {
        /// The system entered.
        system: SystemId,
        /// How it was entered.
        how: Entrance,
    },
    /// The player landed on `stellar`.
    Landed {
        /// The stellar landed on.
        stellar: StellarId,
    },
    /// The player took off from `stellar`.
    TookOff {
        /// The stellar taken off from.
        stellar: StellarId,
    },
    /// A day passed, and `date` is the new day.
    DayPassed {
        /// The new day.
        date: GameDate,
    },
    /// `system` was explored or reached.
    Explored {
        /// The system.
        system: SystemId,
    },
    /// Something became of the NPC `ship`.
    ShipFate {
        /// The ship.
        ship: NpcId,
        /// What became of it.
        fate: Fate,
    },
}

/// A [`StoryEvent`]'s kind, without its data: what an
/// [`EventFilter::Any`] matches on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoryEventKind {
    /// [`StoryEvent::EnteredSystem`].
    EnteredSystem,
    /// [`StoryEvent::Landed`].
    Landed,
    /// [`StoryEvent::TookOff`].
    TookOff,
    /// [`StoryEvent::DayPassed`].
    DayPassed,
    /// [`StoryEvent::Explored`].
    Explored,
    /// [`StoryEvent::ShipFate`].
    ShipFate,
}

impl StoryEvent {
    /// The event's kind.
    #[must_use]
    pub const fn kind(&self) -> StoryEventKind {
        match self {
            Self::EnteredSystem { .. } => StoryEventKind::EnteredSystem,
            Self::Landed { .. } => StoryEventKind::Landed,
            Self::TookOff { .. } => StoryEventKind::TookOff,
            Self::DayPassed { .. } => StoryEventKind::DayPassed,
            Self::Explored { .. } => StoryEventKind::Explored,
            Self::ShipFate { .. } => StoryEventKind::ShipFate,
        }
    }
}

/// Which events a subscription is for: every event of a kind, or only
/// those at one place. Only the kinds that carry a place can be placed:
/// a system for [`StoryEvent::EnteredSystem`] and
/// [`StoryEvent::Explored`], a stellar for [`StoryEvent::Landed`] and
/// [`StoryEvent::TookOff`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventFilter {
    /// Every event of this kind.
    Any(StoryEventKind),
    /// Entering this system, however it is entered.
    EnteredSystem(SystemId),
    /// Landing on this stellar.
    Landed(StellarId),
    /// Taking off from this stellar.
    TookOff(StellarId),
    /// Exploring or reaching this system.
    Explored(SystemId),
}

impl EventFilter {
    /// Whether `event` is one this filter is for.
    #[must_use]
    pub fn matches(&self, event: &StoryEvent) -> bool {
        match (*self, *event) {
            (Self::Any(kind), event) => event.kind() == kind,
            (Self::EnteredSystem(at), StoryEvent::EnteredSystem { system, .. })
            | (Self::Explored(at), StoryEvent::Explored { system }) => system == at,
            (Self::Landed(at), StoryEvent::Landed { stellar })
            | (Self::TookOff(at), StoryEvent::TookOff { stellar }) => stellar == at,
            _ => false,
        }
    }
}

/// Who registered a subscription, in the order consumers run on one
/// event: mission ships before missions, as a landing checks the goals
/// before the missions, and every stock consumer before the plug-ins
/// (see the module docs' "Dispatch").
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Consumer {
    /// A mission's ships (`mïsn` `Ship*` fields).
    MissionShips,
    /// Missions (`mïsn`).
    Missions,
    /// Crons (`crön`).
    Crons,
    /// Nebulae (`nëbu`).
    Nebulae,
    /// Plug-in trigger records.
    Plugins,
    /// A test's own subscriptions.
    Fixture,
}

/// A subscription's name, stable from one session to the next: who
/// registered it, the record it is for, and which of that record's hooks.
/// It orders dispatch, field by field, and keys the once-fired record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionId {
    /// Who registered it.
    pub consumer: Consumer,
    /// The consumer's number for the record: a resource ID, or for
    /// missions and mission ships the active mission's slot.
    pub record: i16,
    /// The consumer's number for the record's hook.
    pub hook: u8,
}

/// "On this event, if this test holds, run this set expression, maybe
/// once": one consumer's reaction to one kind of event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subscription {
    /// Its name, which orders it and keys its once-fired record.
    pub id: SubscriptionId,
    /// The events it is for.
    pub on: EventFilter,
    /// The control-bit test that must hold for it to run.
    pub guard: Test,
    /// The set expression it runs.
    pub script: Script,
    /// Whether it runs at most once a pilot.
    pub once: bool,
}

impl Subscription {
    /// `id` running `script` on the events `on` is for, every time, with
    /// a blank guard.
    #[must_use]
    pub fn new(id: SubscriptionId, on: EventFilter, script: Script) -> Self {
        Self {
            id,
            on,
            guard: Test::default(),
            script,
            once: false,
        }
    }
}

/// The subscriptions `event` runs, in the order they run: those whose
/// filter matches it, by [`SubscriptionId`] (consumer, then record, then
/// hook), whatever order they come in: the dispatch's snapshot. Which
/// have fired once already, which are still registered when their turn
/// comes, and whose guards hold, are the dispatcher's to ask (see the
/// module docs' "Dispatch").
#[must_use]
pub fn dispatch_order<'a>(
    subscriptions: impl IntoIterator<Item = &'a Subscription>,
    event: &StoryEvent,
) -> Vec<&'a Subscription> {
    let mut order: Vec<&Subscription> = subscriptions
        .into_iter()
        .filter(|sub| sub.on.matches(event))
        .collect();
    order.sort_by_key(|sub| sub.id);
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::Gate;

    fn on(consumer: Consumer, record: i16, hook: u8, filter: EventFilter) -> Subscription {
        Subscription::new(id(consumer, record, hook), filter, Script::default())
    }

    fn ids(order: &[&Subscription]) -> Vec<SubscriptionId> {
        order.iter().map(|sub| sub.id).collect()
    }

    const LANDED: StoryEvent = StoryEvent::Landed {
        stellar: StellarId(128),
    };

    #[test]
    fn dispatch_drops_the_subscriptions_the_event_does_not_match() {
        let subs = [
            on(Consumer::Fixture, 1, 0, EventFilter::Landed(StellarId(128))),
            on(Consumer::Fixture, 2, 0, EventFilter::Landed(StellarId(129))),
            on(
                Consumer::Fixture,
                3,
                0,
                EventFilter::Any(StoryEventKind::TookOff),
            ),
            on(
                Consumer::Fixture,
                4,
                0,
                EventFilter::Any(StoryEventKind::Landed),
            ),
        ];
        assert_eq!(
            ids(&dispatch_order(&subs, &LANDED)),
            [id(Consumer::Fixture, 1, 0), id(Consumer::Fixture, 4, 0)]
        );
    }

    /// Matching subscriptions in an order that is none of the sorted ones.
    fn jumbled() -> Vec<Subscription> {
        let any = EventFilter::Any(StoryEventKind::Landed);
        vec![
            on(Consumer::Plugins, 128, 0, any),
            on(Consumer::Missions, 3, 1, any),
            on(Consumer::Crons, 200, 0, any),
            on(Consumer::Missions, 3, 0, any),
            on(Consumer::MissionShips, 5, 0, any),
            on(Consumer::Missions, 1, 4, any),
            on(Consumer::Nebulae, 130, 0, any),
        ]
    }

    fn sorted_ids() -> Vec<SubscriptionId> {
        vec![
            id(Consumer::MissionShips, 5, 0),
            id(Consumer::Missions, 1, 4),
            id(Consumer::Missions, 3, 0),
            id(Consumer::Missions, 3, 1),
            id(Consumer::Crons, 200, 0),
            id(Consumer::Nebulae, 130, 0),
            id(Consumer::Plugins, 128, 0),
        ]
    }

    #[test]
    fn dispatch_runs_by_consumer_then_record_then_hook() {
        let subs = jumbled();
        assert_eq!(ids(&dispatch_order(&subs, &LANDED)), sorted_ids());
    }

    #[test]
    fn registration_order_never_decides_dispatch() {
        let subs = jumbled();
        for turn in 0..subs.len() {
            let mut shuffled: Vec<&Subscription> = subs.iter().collect();
            shuffled.rotate_left(turn);
            assert_eq!(
                ids(&dispatch_order(shuffled.iter().copied(), &LANDED)),
                sorted_ids(),
                "rotated {turn}"
            );
            shuffled.reverse();
            assert_eq!(
                ids(&dispatch_order(shuffled, &LANDED)),
                sorted_ids(),
                "rotated {turn}, reversed"
            );
        }
    }

    #[test]
    fn a_landing_checks_mission_ships_goals_before_the_missions() {
        let subs = [
            on(
                Consumer::Missions,
                0,
                0,
                EventFilter::Any(StoryEventKind::Landed),
            ),
            on(
                Consumer::MissionShips,
                15,
                0,
                EventFilter::Any(StoryEventKind::Landed),
            ),
        ];
        assert_eq!(
            ids(&dispatch_order(&subs, &LANDED)),
            [
                id(Consumer::MissionShips, 15, 0),
                id(Consumer::Missions, 0, 0)
            ]
        );
    }

    #[test]
    fn plugins_run_after_every_stock_consumer_on_every_event() {
        use Consumer::{Crons, MissionShips, Missions, Nebulae, Plugins};
        for event in one_of_each() {
            let any = EventFilter::Any(event.kind());
            let subs = [
                on(Plugins, i16::MIN, 0, any),
                on(Nebulae, i16::MAX, 9, any),
                on(Crons, i16::MAX, 9, any),
                on(Missions, i16::MAX, 9, any),
                on(MissionShips, i16::MAX, 9, any),
            ];
            let order = dispatch_order(&subs, &event);
            assert_eq!(order.len(), 5, "{event:?}");
            assert_eq!(order[4].id.consumer, Plugins, "{event:?}");
        }
    }

    #[test]
    fn a_new_subscription_runs_every_time_behind_a_blank_guard() {
        let sub = Subscription::new(
            id(Consumer::Fixture, 1, 2),
            EventFilter::Landed(StellarId(130)),
            Script::parse("b7"),
        );
        assert_eq!(sub.id, id(Consumer::Fixture, 1, 2));
        assert_eq!(sub.on, EventFilter::Landed(StellarId(130)));
        assert_eq!(sub.script, Script::parse("b7"));
        assert!(!sub.once);
        assert_eq!(sub.guard, Test::default());
        assert!(Gate::FRESH.allows(&sub.guard), "a blank test holds");
    }

    fn id(consumer: Consumer, record: i16, hook: u8) -> SubscriptionId {
        SubscriptionId {
            consumer,
            record,
            hook,
        }
    }

    #[test]
    fn ids_compare_by_consumer_then_record_then_hook() {
        use Consumer::{Crons, Missions};
        assert!(id(Missions, 900, 9) < id(Crons, 128, 0), "consumer first");
        assert!(id(Crons, 128, 9) < id(Crons, 129, 0), "then record");
        assert!(id(Crons, 128, 0) < id(Crons, 128, 1), "then hook");
        assert_eq!(
            id(Crons, 128, 1).cmp(&id(Crons, 128, 1)),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn the_stock_consumers_come_before_the_plugins() {
        use Consumer::{Crons, Fixture, MissionShips, Missions, Nebulae, Plugins};
        let mut consumers = [Fixture, Plugins, Nebulae, Crons, Missions, MissionShips];
        consumers.sort();
        assert_eq!(
            consumers,
            [MissionShips, Missions, Crons, Nebulae, Plugins, Fixture]
        );
    }

    fn day() -> GameDate {
        GameDate::from_start(crate::catalog::StartDate {
            day: 1,
            month: 1,
            year: 1177,
        })
    }

    /// One event of every kind, each at system 128 or stellar 128.
    fn one_of_each() -> [StoryEvent; 6] {
        [
            StoryEvent::EnteredSystem {
                system: SystemId(128),
                how: Entrance::Gate,
            },
            StoryEvent::Landed {
                stellar: StellarId(128),
            },
            StoryEvent::TookOff {
                stellar: StellarId(128),
            },
            StoryEvent::DayPassed { date: day() },
            StoryEvent::Explored {
                system: SystemId(128),
            },
            StoryEvent::ShipFate {
                ship: NpcId(1),
                fate: Fate::Boarded,
            },
        ]
    }

    #[test]
    fn a_kind_filter_matches_every_event_of_its_kind_and_no_other() {
        for wanted in one_of_each() {
            let filter = EventFilter::Any(wanted.kind());
            for event in one_of_each() {
                assert_eq!(
                    filter.matches(&event),
                    event.kind() == wanted.kind(),
                    "{filter:?} on {event:?}"
                );
            }
        }
        let elsewhere = StoryEvent::Landed {
            stellar: StellarId(300),
        };
        assert!(EventFilter::Any(StoryEventKind::Landed).matches(&elsewhere));
    }

    #[test]
    fn a_placed_filter_matches_its_kind_at_its_place_only() {
        let here = SystemId(128);
        let there = SystemId(129);
        let pad = StellarId(128);
        let other_pad = StellarId(129);
        let cases = [
            (
                EventFilter::EnteredSystem(here),
                StoryEvent::EnteredSystem {
                    system: here,
                    how: Entrance::ScriptMove { landed: true },
                },
                StoryEvent::EnteredSystem {
                    system: there,
                    how: Entrance::ScriptMove { landed: true },
                },
            ),
            (
                EventFilter::Explored(here),
                StoryEvent::Explored { system: here },
                StoryEvent::Explored { system: there },
            ),
            (
                EventFilter::Landed(pad),
                StoryEvent::Landed { stellar: pad },
                StoryEvent::Landed { stellar: other_pad },
            ),
            (
                EventFilter::TookOff(pad),
                StoryEvent::TookOff { stellar: pad },
                StoryEvent::TookOff { stellar: other_pad },
            ),
        ];
        for (filter, at, elsewhere) in cases {
            assert!(filter.matches(&at), "{filter:?} on {at:?}");
            assert!(!filter.matches(&elsewhere), "{filter:?} on {elsewhere:?}");
            for event in one_of_each() {
                assert_eq!(
                    filter.matches(&event),
                    event.kind() == at.kind(),
                    "{filter:?} on {event:?}: only its own kind at the same place"
                );
            }
        }
    }

    #[test]
    fn each_event_gives_its_kind() {
        let cases = [
            (
                StoryEvent::EnteredSystem {
                    system: SystemId(128),
                    how: Entrance::Jump,
                },
                StoryEventKind::EnteredSystem,
            ),
            (
                StoryEvent::Landed {
                    stellar: StellarId(128),
                },
                StoryEventKind::Landed,
            ),
            (
                StoryEvent::TookOff {
                    stellar: StellarId(128),
                },
                StoryEventKind::TookOff,
            ),
            (
                StoryEvent::DayPassed { date: day() },
                StoryEventKind::DayPassed,
            ),
            (
                StoryEvent::Explored {
                    system: SystemId(128),
                },
                StoryEventKind::Explored,
            ),
            (
                StoryEvent::ShipFate {
                    ship: NpcId(3),
                    fate: Fate::Destroyed,
                },
                StoryEventKind::ShipFate,
            ),
        ];
        for (event, kind) in cases {
            assert_eq!(event.kind(), kind, "{event:?}");
        }
    }
}
