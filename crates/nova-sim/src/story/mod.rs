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
//!   story-runtime phase 5, the stellars shots destroy in
//!   story-runtime phase 8 and the rebuild points in story-runtime
//!   phase 9;
//! - the mission lifecycle: missions phase 2;
//! - text substitution: missions phase 3.
//!
//! Where the original cannot settle a question, or the design does not
//! follow it, the section names the [`Rulebook`](crate::Rulebook) key the
//! building phase adds and its readings. Phase 2 adds `stay_days`, phase 3
//! `story_reentry`, phase 5 `destroy_invincible`, phase 8 `y_on_destroy`,
//! phase 9 `dead_type_fallback`.
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
//! 9. The shipyard, hire and outfitter day rolls (@0xb7c4-0xb833), each
//!    `Rand(100) + 1`, unconditionally for every record slot: for each
//!    `shïp` slot by index (0 to 767, stride 0xabc), its hire roll
//!    (+0xa2a, @0xb7d0-0xb7dd) and then its buy roll (+0xa2c,
//!    @0xb7ee-0xb801); then for each `oütf` slot (0 to 511, stride 0x37c)
//!    its buy roll (+0x1e, @0xb81c-0xb82f). See "Built by phase 6".
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
//! fills the date and the events, phase 3 `DayPassed`, phase 5 the
//! destroyed stellars' regeneration, phase 6 the day rolls, and phase 8
//! the damaged stellars' repair; the others
//! are empty slots until their phase:
//!
//! 1. the date moves on a day;
//! 2. the crons, each starting or ending cron's script run as it is
//!    reached (crons-news-ranks phase 1);
//! 3. the active missions' days left (missions phase 2);
//! 4. the dominated stellars' tribute (rdm task planet-domination);
//! 5. the day's events, `öops`: today's `market::step_day`;
//! 6. the stellars: every whole one repaired to its `Strength`
//!    (story-runtime phase 8; see "Built by phase 8"), a destroyed one's
//!    regeneration and `OnRegen`, each `OnRegen` run as its stellar is
//!    reached (story-runtime phase 5; see "Built by phase 5"), and a
//!    dominated one's defence regrowing (planet-domination);
//! 7. the reinforcement countdown, once reinforcements are built;
//! 8. the ranks' salaries (crons-news-ranks phase 3);
//! 9. the day rolls (`wares::DayRolls`): every ship class's hire and buy
//!    rolls, then every outfit's, drawn afresh (story-runtime phase 6;
//!    see "Built by phase 6");
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
//! the stellars shots destroy by story-runtime phase 8; the visibility
//! rules by crons-news-ranks phase 5; ownership by rdm
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
//! pilot (save version 17; a destroyed stellar's countdown, version 18),
//! and the session reads the galaxy only through
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
//! writer (with phase 5's catalog-free entry for a stellar's destroyed
//! state; see "Built by phase 5"). A change applies the moment it
//! returns; when it is called is
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
//! explores nothing and passes no day. A landed player stays docked: the
//! spaceport stays its stellar's, and the new system's stellars wait for
//! the take-off, as after a landed `M`; a pilot loaded landed at a stellar
//! of a hidden system at its place resumes docked there the same way, and
//! so does a relocation to a stellar of a system resolved to another. One
//! owner decides this for every move of the player's system, the session's
//! `set_player_system` (see the session's `entry` module): docked, the
//! stellars shown are always those of the system whose stellar the ship is
//! docked at. The
//! course keeps its system IDs, each hop resolved by place when it is
//! used, as `_CueNextHyperRouteDest` resolves it.
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
//! ## Built by phase 5
//!
//! A destroyed stellar holds its regeneration [`Countdown`] in the
//! overlay, the days left; one not destroyed holds none (save version
//! 18). The original's own pair, a current strength of -1 and a countdown
//! of -1 for "whole", is never stored.
//!
//! **One writer, two entries.** The writer has one core, which writes the
//! pilot's overlay and the galaxy's and makes a save due, and two entries:
//! [`Session::change_galaxy`](crate::Session::change_galaxy), which takes
//! a catalog for its follow-ups (a hidden system, a government, the
//! spawn table), and `Session::set_stellar_state`, catalog-free, which
//! can change a stellar's destroyed state alone, so it never needs them.
//! `apply_op` and the daily step have no catalog, and a cron's `Y` must
//! be counted by the same day's sixth participant, so neither waits for
//! a settling. The catalog-free entry stamps the stellars shown and those
//! waiting for the take-off afresh, and tells `GalaxyChanged` once, only
//! when a stellar is destroyed or regenerated; a countdown ticking down
//! only makes a save due.
//!
//! **Its writers** (the `stellars` module of the session):
//!
//! - `Y` destroys a stellar with its `DeadTime` as the countdown, or 1
//!   when that is negative, destroyed already or not; `U` regenerates it
//!   and runs no `OnRegen`. Each reads the stellar's
//!   [`StellarLife`](crate::StellarLife), which the session reads from
//!   the catalog as it starts. An ID outside 128 to 2175, or with no
//!   record, does nothing.
//! - The daily step's sixth participant counts each destroyed stellar
//!   down a day, by ascending ID, reading the overlay again after each.
//!   One of negative `DeadTime` stays destroyed with a day left; any
//!   other regenerates once 1 or 0 days were left, and runs its `OnRegen`
//!   there and then through `Session::run_set`. That is a participant's
//!   effect, not a story subscription: it runs inside the day, before the
//!   ranks and `DayPassed`. Its `M`, `N`, `Q` and `S` defer to the next
//!   settling, as any hook's do.
//! - A new pilot ([`Session::begin`](crate::Session::begin)) starts with
//!   every stellar whose `Flags2` has 0x0040 destroyed, as `Y` destroys
//!   it, before its `OnStart`. A load keeps the stellars as saved. No
//!   stock stellar starts destroyed, so a save from before version 18
//!   needs no seeding.
//!
//! **Its readers.** [`OverlaidGalaxy::site`] stamps a stellar's
//! [`LandingSite::destroyed`](crate::LandingSite::destroyed) from the
//! overlay, as it stamps the government, and
//! [`is_landable`](crate::is_landable) reads it as the original's
//! `_StellarIsLandable` does: a stellar can be landed on when it has the
//! can-land flag and is destroyed exactly when it can be landed on only
//! once destroyed. Docking (gates and the developer tools' relocation),
//! F5, the land key, and the NPCs' choice of stellar and whether they keep
//! heading for it follow. A landed `M` docks at the first stellar,
//! destroyed or not, as the original does. A player docked at a stellar
//! `Y` destroys stays docked: the spaceport stays open, the stellar stays
//! among those shown, flagged destroyed, the take-off works and passes its
//! days, and only landing there again is refused until it regenerates. A
//! pilot saved docked there resumes docked there.
//!
//! The galaxy map has no service, trade or colour mode and no stellar
//! panel yet; when built, they read `is_landable`. Defence fleets,
//! stellar weapons and the AI's stellar targets do not exist yet (rdm
//! task planet-domination, story-runtime phase 13); the original's
//! readers are listed below for them. Shots at stellars are story-runtime
//! phase 8's (see "Built by phase 8"). The stellar drawn in
//! flight is story-runtime phase 7's (see "Built by phase 7").
//!
//! **Readings.** The Rulebook key
//! [`destroy_invincible`](crate::RuleKey::DestroyInvincible) defaults to
//! the engine: `Y` leaves a stellar of `Strength` 0 or -1 as it was, and
//! such a stellar never starts the game destroyed. By the Bible's
//! "destroy stellar ID xxx", `Y` destroys it, and the day counts it down
//! as any other. No key is needed for `U` and `OnRegen`: the Bible's
//! `OnRegen` is "evaluated when the stellar automatically regenerates",
//! which `U` is not, so it agrees with the engine. No key either for a
//! second `Y` restarting the countdown, a cron's `Y` counted the same day,
//! a docked player staying docked, or a destroyed stellar with 0 days left
//! kept destroyed across a save (the original's reload restores it, with
//! no `OnRegen`, @0x76068-0x760b8): that last is a save-format artifact,
//! not a game rule.
//!
//! ## Checked for phase 5
//!
//! - **The runtime stellar** (0x498 bytes, `_LoadObjectData`
//!   @0x7733e-0x774ad): `Type` +0x0e (from `spöb` +0x04), `Flags` +0x30
//!   (+0x06), `Flags2` +0x34 (+0x20), `Strength` +0x40 and the current
//!   strength +0x3c, both from +0x23c (@0x77434-0x7743f), `DeadType` +0x10
//!   (+0x240), `DeadTime` +0x47a (+0x242), the countdown +0x47c, set to 0
//!   (@0x7746b), `ExplodType` +0x46e (+0x244). `OnDestroy` is copied to
//!   +0x266 and `OnRegen` to +0x365 (@0x774c7-0x77502). The stellar's
//!   system is +0x14. The Bible's `Flags2` 0x0040 is "Stellar starts the
//!   game destroyed", and `_DoNewPilot` reads it (@0x192d6).
//! - **Destroyed** (`_StellarIsDestroyed` @0x4e14-0x4e3c): `Strength` > 0
//!   and (current strength < 0 or countdown > 0). An invincible stellar is
//!   never destroyed.
//! - **`Y`** (`_EvalSetExp` @0x15a38-0x15ae6) takes IDs 128 to 2175
//!   (@0x15a3f-0x15a46) and ignores any other. When the stellar is in the
//!   player's system (+0x14 against the player's +0x74), not destroyed
//!   yet (@0x15a71) and has an `ExplodType`, it makes the explosion
//!   (`_CreateExplosion` @0x15ab4). Then, always, the countdown becomes
//!   `DeadTime`, or 1 when that is negative (@0x15ac8-0x15ad5), and the
//!   current strength -1 (@0x15ade). It runs no `OnDestroy`, redraws no
//!   stellar and touches no landing state.
//! - **`U`** (@0x15aeb-0x15b20): the same range; the countdown becomes -1
//!   and the current strength `Strength`. No `OnRegen`, no explosion, no
//!   redraw.
//! - **The day's stellars** (`_IncrementGameTime` @0xb5c4-0xb71e) visit
//!   every stellar slot in use (+0x44, @0xb5d5) by ascending index. A
//!   destroyed one with a negative `DeadTime` has its countdown set back
//!   to 1 (@0xb680, @0xb6f5). Any other destroyed one counts down
//!   (@0xb68a); at 0 or below the countdown becomes -1, the current
//!   strength `Strength`, and its `OnRegen` runs at once
//!   (`_EvalMissionBitSetString` @0xb6ee). A stellar not destroyed is
//!   repaired: current strength `Strength`, countdown -1 (@0xb700-0xb705).
//! - **Landing** (`_StellarIsLandable` @0xa989-0xa9f7): `Flags` 0x01, and
//!   destroyed exactly when `Flags` has 0x80. Its callers: the dock
//!   request (`_HandlePlayerDockRequest` @0x66977), the comm dialog
//!   (`_HandlePlayerCommunication` @0x61fda), NPC destinations
//!   (`_SelectRandomStellarDest` @0x8074e), mission stellars through
//!   `_StellarIsNormalLandable` (`_SelectMissionStellar`), the map's
//!   service, trade and colour modes and its panel (`_SystemTradesGoods`
//!   @0xd320, `_SystemHasService` @0xd3ce, `_DrawLegalStatusString`
//!   @0xd5c9, `_ColorCodeSystem` @0xd825, `_MapDialogUpdate` @0x10399,
//!   @0x10417), `_ColorCodeStellar` (@0xaa42) and the escape pod's search
//!   (`_RecursiveEscapeStellarSearch` @0xd1bb).
//! - **Other live readers of the destroyed state**: the sprite tick
//!   (`_HandleStellarSprites` @0x2e77b), stellar weapons
//!   (`_HandleStellarWeapons` @0x2efd4), shots (`_CheckShotProximities`
//!   @0x3727b, @0x37409), the AI (`_DeathStarAI` @0x8c19e,
//!   `_InterceptorAI` @0x8d118), the pilot log (`_DumpPilotLog`
//!   @0x74d07), and the drawn stellar's rebuild
//!   (`_HandleStellarSystemVisibility` @0x32d18, @0x32f0f). The sprite
//!   tick picks `DeadType` or `Type` for the drawn graphic every frame, so
//!   a `Y` or `U` changes it from the next frame (see "Checked for phase
//!   7").
//! - **Defence and traffic** read nothing more. `_EnterMoreShips` launches
//!   a stellar's defence by its own flags, +0x47 and the count left
//!   +0x466 (@0x43c8f-0x43c9d), and `_SpawnStellarDefenseShip`
//!   (@0x3d2c9) and the daily regrowing (+0x46, @0xb5f6) read no destroyed
//!   state; `_SelectRandomStellarDest` reads it only through
//!   `_StellarIsLandable`.
//! - **A landed `M`** (@0x1570e-0x158d4) makes the new system's first
//!   stellar the player's, checking neither landability nor the
//!   destroyed state: `_EvalSetExp` calls `_StellarIsDestroyed` only for
//!   `Y` (@0x15a71) and `_StellarIsLandable` never (see the
//!   `script_effects` module). The engine's `M` docks at the first
//!   stellar only when it has the can-land flag and is no gate, its own
//!   reading, and reads neither its destroyed state nor whether it is
//!   landable only once destroyed, so it docks there destroyed or not, as
//!   the original does.
//! - **Docked.** Neither `_DoPortDialog` nor the take-off
//!   (`_PlayerLandOnStellar`) reads the destroyed state or landability:
//!   the callers above are all there is. A `Y` on the docked stellar
//!   leaves the player docked.
//! - **Saved state**: the countdowns alone, 2048 `i16`s at +0x4d90
//!   (`_WritePilotData` @0x72ad0-0x72aff). On load (`_LoadPilotData`
//!   @0x76068-0x760b8) a countdown above 0 is destroyed (current strength
//!   -1); any other is whole (countdown -1, strength restored), so a
//!   destroyed stellar with 0 days left comes back whole, with no
//!   `OnRegen`.
//! - **New pilot** (`_DoNewPilot` @0x192b9-0x19332, and `_ResetGameData`
//!   @0x1de5e-0x1de92): `Flags2` 0x0020 starts a stellar dominated; 0x0040
//!   starts it destroyed, current strength -1, countdown `DeadTime` or 1;
//!   any other is whole with countdown 0. Then the `chär` is activated
//!   (`_ActivateCharResource` @0x19345).
//! - **Stock data** (`Nova Data 2.ndat`'s 411 `spöb`s and every
//!   expression field of the `mïsn`, `crön`, `oütf`, `shïp`, `chär`,
//!   `nëbu`, `përs` and `spöb` records): no stellar has `Flags2` 0x0040
//!   or `Flags` 0x80; every `DeadTime` is 0; no `OnRegen` is set; 17
//!   stellars have `Strength` 0 or below; and no script uses `Y` or `U`.
//!   So both readings and the seeding change nothing for stock play.
//! - **The Bible** says "Yxxx - destroy stellar ID xxx", "Uxxx -
//!   regenerate (Un-destroy) stellar ID xxx", `Strength` "0 or -1 for an
//!   invincible stellar" (of damage from planetary weapons), and `OnRegen`
//!   "evaluated when the stellar automatically regenerates".
//!
//! ## Built by phase 6
//!
//! The daily step's ninth participant draws the shop and hire rolls
//! (`wares::draw_day`): every roll is forgotten, a held owned outfit's
//! too, then each ship class by ascending ID draws its hire roll and then
//! its buy roll, and then each outfit by ascending ID its buy roll. The
//! engine does not replicate the original's `Rand` stream, so a roll is
//! drawn only when its reading is a chance of 1-99: a never or an always
//! draws nothing, but the order among the draws is the original's. The
//! readings are those the lists already use: `hire::hire_roll`,
//! `shipyard::buy_roll` and `outfitter::buy_roll`.
//!
//! **A landing keeps them.** No landing path writes a roll, so a landing
//! reads the rolls the last day drew, and one with no day passing (a
//! relocation, or a take-off under `stay_days` = bible) keeps them. An
//! owned outfit's hold lasts the rest of the day, past later landings. A
//! class bought or declined is drawn again at once by the original
//! (@0x5f0f8); here its pending redraw (`Session::shipyard`) is applied
//! by a landing that passes no day, and dropped by a day's draw, which
//! draws every roll afresh anyway. The single redraws after a hire, a
//! buy or a decline stay lazy, drawn when the list next asks.
//!
//! **Nothing is saved.** The original redraws every roll on load
//! (`_LoadPilotData` @0x7640b-0x76478) and for a new pilot (`_ResetPlayer`
//! @0x1d84a-0x1d8b9), so a session that starts or loads has no rolls, and
//! each is drawn the first time a list asks for it: the same outcome as
//! the original's eager draw.
//!
//! **Readings.** The Rulebook key `day_rolls` defaults to the engine's
//! daily draw. Its other reading is the port's earlier landing-time draw:
//! a landing (and a relocation) forgets every roll and each is drawn the
//! first time a list asks for it; the day draws none.
//!
//! In stock play every take-off passes a day under `stay_days` = engine,
//! so each landing still sees freshly drawn rolls: the visible changes
//! are only when the draws happen, a landing with no day passing, and an
//! owned outfit's hold lasting the day.
//!
//! ## Checked for phase 6
//!
//! - **The fields.** `_LoadObjectData` copies a `shïp`'s `BuyRandom`
//!   (+0x388) to +0xa28 and `HireRandom` (+0x38a) to +0xa26
//!   (@0x7a23d-0x7a259), and clamps each unsigned above 100 to 100
//!   (@0x7a340-0x7a367); an `oütf`'s `BuyRandom` (+0x3f0) goes to +0x1c
//!   (@0x78b7c-0x78b83), clamped to 0-100 (@0x78b8d-0x78ba6). The rolls
//!   are +0xa2a (hire), +0xa2c (buy) and the outfit's +0x1e.
//! - **Which roll is which** (`_SetupPortAvailableShipTypes`
//!   @0xbc7d-0xbcdd): the shipyard lists a class when `BuyRandom` is not
//!   0 and at least +0xa2c, unsigned (@0xbc88-0xbc9b); the bar when
//!   +0xa2a, times the registered test, is at most `HireRandom`
//!   (@0xbc9f-0xbcdd). `_SetupPortAvailableItems` (@0xc05d-0xc08f) lists
//!   an outfit not owned when its `BuyRandom` is above 0 and at least
//!   +0x1e, signed; one owned (the count at `0x3b51a8`, @0xc063) is
//!   listed, and its roll set to 0 (@0xc08f), so it holds until the next
//!   day's draw overwrites it (@0xb82f).
//! - **The order** (@0xb7c4-0xb833) is per ship slot hire then buy, then
//!   every outfit slot, with no test in either loop: no `BuyRandom`,
//!   `HireRandom`, tech or record-exists check. The step itself is
//!   reached unconditionally after the ranks' loop (@0xb7c2).
//! - **The other writers** of +0xa2a, +0xa2c and the outfit's +0x1e:
//!   `_InitObjects` sets 50 (@0x1c68a, @0x1c696, @0x1c83f), `_ResetPlayer`
//!   and `_LoadPilotData` draw with the same loops (@0x1d84a-0x1d8b9,
//!   @0x7640b-0x76478), `_DoShipyardDialog` redraws a bought or declined
//!   class's buy roll (@0x5f0f8) and a hired class's hire roll
//!   (@0x5f21a), and the owned-outfit hold (@0xc08f). Nothing else:
//!   neither `_PlayerLandOnStellar` nor `_DoPortDialog` writes one. The
//!   other `0x1e(` hits read other records: @0x4597 and @0x5c1f a `wëap`
//!   (stride 0xc8), @0x3620c a `0x3b540c` record, @0x3a681 a system
//!   (stride 0x1fc).
//! - **Not saved**: no roll is read outside the two listings (@0xbc94,
//!   @0xbcb6, @0xc07c), so `_WritePilotData` saves none, and
//!   `_LoadPilotData` draws them all afresh (@0x7640b-0x76478).
//! - **One divergence outside this phase**: the loader's unsigned clamp
//!   makes a negative `HireRandom` 100, always for hire (@0x7a35d-0x7a367,
//!   then @0xbca6 lists any but 0), where the port's bar reads it as
//!   never; the hire roll's reading is unchanged here (see the `hire`
//!   module).
//!
//! ## Built by phase 7
//!
//! The flight view draws a destroyed stellar with its `DeadType` sheet
//! from the first frame after the change, as the original's per-frame
//! sprite tick does. It samples the live state,
//! [`OverlaidGalaxy::destroyed`], through `Session::galaxy`, on the
//! events it already reacts to; there is no snapshot and no new event.
//! The one writer of the destroyed state, `Session::set_stellar_state`,
//! raises `GalaxyChanged` exactly when a stellar is destroyed or
//! regenerated (by `Y`, `U`, the daily step or a new pilot), and a
//! countdown that only ticks raises nothing, so reacting to it gives the
//! same picture as a per-frame poll. On `GalaxyChanged` the view redraws
//! the stellars of the scene shown without laying it out again; on
//! `SystemChanged`, and for its first scene, it lays the scene out and
//! then applies the state. Story-runtime phase 8's shots write through
//! `set_stellar_state` too, so they raise `GalaxyChanged`.
//!
//! The sheet rule lives in nova-view's system scene: a destroyed stellar
//! draws its `DeadType` sheet when that resolves to one, and its `Type`
//! sheet otherwise (a `DeadType` outside 0 to 255, or one with no
//! `spïn`), which is the Bible's "-1: Don't display different graphic
//! type when destroyed" and the original's reading between rebuilds
//! (after one it draws graphic type 0; see "Built by phase 9"). Its
//! animation follows the live state: one whose `Flags2` has 0x0080
//! animates only while destroyed, any other only while not. Every
//! stellar's frame comes from the view's one clock, so a swapped sheet
//! shows the frame that clock gives it, over its own frame count; the
//! original keeps the slot's frame index while it is in range, over the
//! frame count from the last rebuild (story-runtime phase 12).
//!
//! No Rulebook key is added. The original's other readings are filed as
//! their own phases, each to add a key with the original's reading as
//! the default: story-runtime phase 9 (a `DeadType` with no sheet draws
//! `spïn` 1000 after a rebuild; built, see "Built by phase 9"), phase 10
//! (the original's landing distance and its 150 fallback; built, see
//! "Built by phase 10"), phase 11 (clicking a destroyed
//! stellar by its `DeadType` sheet) and phase 12 (the frame count kept
//! from the last rebuild).
//!
//! ## Checked for phase 7
//!
//! - **The sprite tick swaps live.** `_DoPlayGameWork` calls
//!   `_HandleStellarSprites` unconditionally in every play frame
//!   (@0x444d2, between the ships' loop and `_HandleStellarWeapons`). It
//!   walks the 16 stellar sprite slots (the array behind 0x3b53d0; slot
//!   +0xd0 is the stellar's index), and for each in use calls
//!   `_StellarIsDestroyed` (@0x2e77b), takes `DeadType` (+0x10, @0x2e79b)
//!   when destroyed and `Type` (+0x0e, @0x2e7b7) when not, loads that
//!   master if it is missing (`_LoadMasterStellarSprite` @0x2e7d1), and
//!   clones its frames into the slot (`_SWCloneFramesFromMaster`
//!   @0x2e7e5). The clone replaces the frames whenever the master exists
//!   (@0xac4ce-0xac4f8) and keeps the slot's frame index when it is still
//!   in range (@0xac504-0xac532). Its one block, the slot's +0xd9
//!   (@0xac46a), is set only by sprites that own their frames
//!   (`_SWCreateSpriteFromRLEResource` @0xa9b1d, `_SWCreateRLEData`
//!   @0xaef6d); a stellar slot is made by `_SWCreateEmptySprite`, which
//!   clears it (@0xa9003). So a `Y` or `U` on a stellar of the player's
//!   system shows the other graphic from the next frame.
//! - **Animation follows the live state** (@0x2e784-0x2e7bb): a destroyed
//!   stellar animates only when `Flags2` (+0x34) has 0x0080, one not
//!   destroyed only when it lacks it, as the Bible says. One that does
//!   not animate shows frame 0 (@0x2ee20). The frame count it animates
//!   over (+0x474) is copied from the slot only at a rebuild (@0x331c9).
//! - **The rebuild** (`_HandleStellarSystemVisibility` @0x32aa5) makes
//!   the same choice for each stellar of the player's system
//!   (@0x32f0f-0x32f2f), frees each master no stellar there uses
//!   (@0x32d18-0x32e4e), clears the frames of every slot that was cloned
//!   (@0x32e68-0x32e83), and resets the animation only of a newly made
//!   slot (@0x331da-0x331fb). With no master to clone, a slot is left
//!   empty, and the last pass logs "had no data" and clones graphic type
//!   0's master (`spïn` 1000) into it (@0x33256-0x332d0). Its callers are
//!   the rebuild points: the in-flight `M` (@0x159a3), `_DoPortDialog`
//!   (@0x5fbf0), the gate and wormhole exits (@0x63e8c, @0x6473e), the
//!   dock request (@0x674c7, @0x67653), `_HandlePlayer` (@0x69135,
//!   @0x6c203, @0x6c597, @0x6dad1), a new pilot (@0x1937f), a load
//!   (@0x751cf, @0x75324) and the main screen (@0x1a354, @0x1a8ca,
//!   @0x1b003, @0x457ee).
//! - **A `DeadType` of -1** is not a special case: the master array
//!   `_masterStellarSprite` (0x20ee00, 256 pointers, behind 0x3b5040) is
//!   indexed with it (@0x2e7c7). Index -1 is 0x20edfc, a word of
//!   `_masterStarSprite` (0x20ede0) that nothing writes, since the star
//!   field uses only that array's first entry (`_LoadSprites` @0x210ca).
//!   `_LoadMasterStellarSprite(-1)` asks for `spïn` 999, which no stock
//!   file has, and `_GetSpriteInfo` fails without it (@0x1bfb2), so the
//!   clone fails (@0xac448) and, between rebuilds, the slot keeps its
//!   `Type` frames, as the Bible says. After a rebuild it draws `spïn`
//!   1000 (above; story-runtime phase 9).
//! - **Other out-of-range `DeadType`s** read the neighbouring statics:
//!   index 256 is `_masterWeapTargetSprite` (0x20f200), which
//!   `_LoadSprites` loads (@0x2170f), so a `DeadType` of 256 clones the
//!   weapon-target sprite; -8 is the star field's master. The engine
//!   reads any `DeadType` outside 0 to 255 as no different graphic.
//! - **The landing radius** (`_GetStellarSpriteXSize` @0xac91, called by
//!   the dock request @0x66cd9 and @0x6773a and by `_HighLevelAIHandler`
//!   @0x8dbdf) reads the `Type` master's current frame, never
//!   `DeadType`'s (@0xacc3), and its width, 32 for a frame index out of
//!   range (@0xacff), or 150 with no master loaded (@0xad2d), which is
//!   the case after a rebuild that found the stellar destroyed. The dock
//!   request multiplies it by 1.75 (the double at 0xdd6e8) and lands when
//!   both the x and the y distance are below that (@0x66cdf-0x66cfd), or
//!   below 75 when the stellar's slot shows no frame (@0x66cc7). The
//!   engine now does the same by default (see "Built by phase 10" and
//!   "Checked for phase 10", which corrects the 32 above).
//! - **Clicking** (`_HandlePlayer` @0x6a5f9-0x6a686) uses the drawn slot's
//!   rectangle, so a destroyed stellar is picked by its `DeadType` sheet;
//!   the engine does the same since phase 11 (see "Built by phase 11").
//!
//! ## Built by phase 8
//!
//! A shot of a planet-type weapon (`wëap` `Flags2` 0x0400) destroys a
//! stellar by taking its strength below 0 (the `stellar` module of the
//! combat, and the `stellars` module of the session).
//!
//! - **Which stellars.** Those shown, whole, of `Strength` above 0 and
//!   with a sprite. Each is a disc as big as the landing radius (half the
//!   larger side of its `Type` frame), which is what the player sees: the
//!   original tests the two sprites pixel by pixel (`_SWRLECollision`
//!   @0x372a8), and one with no sprite cannot be hit. The shot's path
//!   over the tick is swept, as for ships, and the nearest stellar along
//!   it is hit; the original takes the first of the 16 slots that
//!   collides, and explodes the shot at its own position, where the
//!   engine takes the point its path entered the disc.
//! - **When.** In a pass after the ship hits and the expiries
//!   (combat's step 7b), so a shot that would meet a ship and a stellar
//!   the same tick hits the ship, and one at the end of its life hits
//!   nothing. Only past its `ProxSafety`: its age above it. Any ship's
//!   shot, the player's or an NPC's. Beams never hit a stellar.
//! - **The hit.** It takes `MassDmg` + `EnergyDmg` off the current
//!   strength, explodes where it hit with its `ExplodType`, and is gone,
//!   with no blast and no sub-munitions.
//! - **The current strength** is the session's, kept only for a whole
//!   stellar that has taken damage, in a wide integer, and never saved, as
//!   the original saves none: a load gives every whole stellar its
//!   `Strength`. `Y` and `U` forget it, and the daily step's sixth
//!   participant forgets it all, which is the original's repair of every
//!   whole stellar (@0xb700): nothing damages one within a day.
//! - **Shot down** below 0, exactly 0 being whole. In order: its
//!   `ExplodType` explosion at its centre, the overlay's destroyed state
//!   through `Session::set_stellar_state` with the countdown `Y` gives it
//!   (so `GalaxyChanged` is raised and the view draws its `DeadType`),
//!   its `OnDestroy` at once through `Session::run_script`, with the
//!   stellar already destroyed, its `M`, `N`, `Q` and `S` deferred to the
//!   next settling as any hook's are, then the destroyed state again, so
//!   a `U` of its own is undone (@0x374be-0x374cf). A shot of the player's
//!   then convicts it ten times of killing a ship of the stellar's
//!   government as overlaid, each held to the record's limit; an NPC's,
//!   an escort's or a fighter's convicts no one. A stellar shot down is
//!   skipped by the tick's later shots, so `OnDestroy` runs once.
//! - **`Y`'s explosion.** `Y` on a stellar shown in flight that was not
//!   destroyed before makes its `ExplodType` explosion at its centre,
//!   whatever its `Strength`: an invincible one, which the engine's
//!   `destroy_invincible` leaves whole, explodes every time. None is made
//!   while landed, where the original's would be cleared on the take-off.
//!
//! **Readings.** The Rulebook key
//! [`y_on_destroy`](crate::RuleKey::YOnDestroy) defaults to the engine:
//! `Y` runs no `OnDestroy`. By the Bible's "evaluated when the stellar is
//! destroyed", `Y` runs it after destroying a whole stellar: not on one
//! destroyed already, nor on an invincible one it leaves whole.
//! That reading can loop, where the original cannot, since its `Y` runs
//! no `OnDestroy`: an `OnDestroy` of `U128 Y128`, or two that regenerate
//! and destroy each other, would run without end. So an `OnDestroy` never
//! runs inside itself, whether a shot or a `Y` started it: the `Y` still
//! destroys the stellar, and the skipped run is reported as
//! [`ScriptNote::OnDestroyReentered`](crate::ScriptNote::OnDestroyReentered). No key for
//! the ten killings, the daily repair, `Y`'s explosion on an invincible
//! stellar or shots passing through one, which the Bible's `Strength`
//! ("0 or -1 for an invincible stellar") agrees with, nor for the
//! countdown. The Bible's planet-type weapon "can only hit planet-type
//! ships or destroyable stellars", but its ship side, a planet-type blast
//! sparing ships, the AI firing at stellars and whether beams hit
//! stellars are story-runtime phase 13 "planet-type-weapons-and-ships";
//! `Flags2` 0x0400 is still reported as unimplemented for them.
//!
//! ## Checked for phase 8
//!
//! - **The weapon** (stride 0xc8, `_LoadObjectData` @0x792cf-0x7973b):
//!   `Count` +0x00, `MassDmg` +0x02, `EnergyDmg` +0x04, `ExplodType`
//!   +0x14, `BlastRadius` +0x18, `ProxSafety` +0x1c (@0x7973b), `Flags2`
//!   +0x22. The stellar's `Govt` is +0x0a (@0x77387-0x77416).
//! - **The stellar branch of `_CheckShotProximities`** (@0x371bc-0x37775),
//!   for each of the 128 shots (stride 0x48): alive (life +0x14 at least
//!   0, @0x371ea-0x371f6) and `Count - ProxSafety > life`
//!   (@0x371fc-0x3721e); `Flags2` 0x0400 (`testb $0x4, 0x23`, @0x37224),
//!   or straight on to the ship proximities; the 16 slots of the player's
//!   system (@0x37230-0x37254), skipping `Strength` 0 or below (@0x3726d)
//!   and the destroyed (@0x3727b); the sprite collision (@0x372a8); the
//!   current strength +0x3c less `MassDmg + EnergyDmg` (@0x372dc-0x372eb);
//!   `_CreateExplosion` at the shot with its `ExplodType` and
//!   `BlastRadius` (@0x3732a), visual only (@0x3f698-0x3f926); the shot
//!   dies, life -1 and its sprite hidden (@0x373c4-0x373fe). Then, when
//!   `_StellarIsDestroyed` (@0x37409): the stellar's `ExplodType` at its
//!   centre unless -1 (@0x37457), `OnDestroy` (+0x266,
//!   `_EvalMissionBitSetString` @0x374a9), and only then current strength
//!   -1 and countdown `DeadTime` (@0x374be-0x374cf); when the shot's +0x2c
//!   is 0, `_SlapWithPenalty(system, Govt, 3, -1)` ten times
//!   (@0x374e1-0x3751f), kind 3 being `KillPenalty` (`_DamageShip`
//!   @0x3ae34). One hit ends the stellar loop (`jmp 0x3752d`).
//! - **Proximity safety.** A shot's life starts at `Count` (`_SpawnShot`
//!   @0x3e801-0x3e808), and `_HandleShot` takes the frame's time off it
//!   each frame (@0x35691-0x356a3) and sets it to -1 at 0 or below
//!   (@0x356bb-0x356c1). After k moves it is `Count - k`, so the test is k
//!   above `ProxSafety`: the engine's age above `prox_safety`.
//! - **The firer.** Shot +0x2c is `_SpawnShot`'s first argument, the
//!   firing ship's slot (@0x3e57b, @0x3e7c8); slot 0 is the player's
//!   (`_ShotCanHitShip` tests a ship's +0x86 against 0, @0x448db). An
//!   escort's or a fighter's shot has its own slot, so only the player's
//!   own shots are punished.
//! - **Planet-type shots and ships.** `_ShotCanHitShip`
//!   (@0x4477e-0x44b6f), `_HandleShipHit` (@0x36a45) and
//!   `_ShipCollisionHandler` (@0x36f87) never test `Flags2` 0x0400, so a
//!   planet-type shot hits ordinary ships as any shot does. The bit's
//!   other readers are a detonation blasting no ships (`_HandleShot`
//!   @0x35fb9), an asteroid hit's blast sparing ships (`_HandleAsteroidHit`
//!   @0x36734) and the AI (@0x804a9, @0x8c1f2, @0x8f94d): story-runtime
//!   phase 13's.
//! - **No blast and no sub-munitions.** The `movb $0x1, 0x16` after the
//!   hit (@0x373fe) writes the shot's sprite (its first field), not the
//!   shot: the byte `_SWSetSpriteVisible` itself sets on hiding
//!   (@0xa95b0), as the ship hit does too (@0x36f7b). The next
//!   `_HandleShot` finds life -1, above -32000 (@0x36195), takes the
//!   end-of-life path (@0x35f08), finds the sprite hidden (@0x35f0a) and
//!   only sets life -2 (@0x3617c): no sub-munitions (@0x35f49) and no
//!   detonation (@0x35f97).
//! - **The order in a frame.** `_DoPlayGameWork` runs the ship collision
//!   passes (`_SWCollideSpriteLayer` @0x43e70-0x44010) and then
//!   `_CheckShotProximities` (@0x44015) before `_HandleShot` (@0x44385),
//!   which moves the shots and ends their lives. As frames repeat, each
//!   move and expiry is followed by the next frame's ship hits and then
//!   its stellar hits, which is the engine's tick: the shots fly, the
//!   ships are hit and the expired gone, then the stellar pass. A shot
//!   whose life ran out in its last move is dead before any collision at
//!   its new position, as `resolve` drops it before the stellar pass.
//! - **`Y`'s explosion** (@0x15a59-0x15ab4): the stellar's system +0x14
//!   is the player's +0x74, `_StellarIsDestroyed` is false and
//!   `ExplodType` is not -1: `_CreateExplosion(x, y, ExplodType, 0, 1)`,
//!   before the countdown is written.
//! - **Landed.** The dock request sets `_killExplods` (0x1b8c6d, @0x66f45,
//!   @0x67041), as the in-flight `M` does (@0x158ba, @0x1597e);
//!   `_HandleExplods` removes every explod while it is set (@0x2f604), and
//!   `_PlayGame` clears it only after a play frame (@0x46391). So an
//!   explosion a landed `Y` makes is gone on the take-off's first frame.
//! - **Gates.** `_PlayerEnterHypergate` (called from the dock request
//!   @0x66edb, after `_StellarIsLandable` @0x66977) never reads the
//!   destroyed state, and the hypergate map is modal: no frame runs
//!   between the press and the entry, as the engine's pending entry ends
//!   at any tick. The gate path is unchanged.
//! - **The day's repair** (`_IncrementGameTime` @0xb700-0xb705): a stellar
//!   not destroyed gets current strength `Strength` and countdown -1.
//! - **Saved**: the countdowns alone (`_WritePilotData` @0x72ad0-0x72aff);
//!   a load restores a whole stellar's strength (@0x76068-0x760b8).
//! - **Stock data**: no stock `wëap` has `Flags2` 0x0400, so stock play
//!   never shoots a stellar down; most stellars have an `OnDestroy`
//!   (`b6200`-`b6206`), and 393 an `ExplodType` of 0.
//!
//! ## Built by phase 9
//!
//! A destroyed stellar whose `DeadType` has no sheet that can be read (-1,
//! outside 0 to 255, no `spïn`, or an `rlëD` the engine cannot read) draws
//! graphic type 0, `spïn` 1000, from the next rebuild point until a
//! flight frame finds it whole, as the original's rebuild fills the sprite
//! slot it left empty; before that rebuild point it draws its `Type`.
//!
//! - **The state** is the sim's, kept for the stellars shown: `Rebuilt`
//!   (the `rebuild` submodule) holds those a rebuild point found destroyed
//!   and no flight frame has found whole since, read through
//!   `Session::dead_since_rebuild`. It is never saved: a load is itself a
//!   rebuild point, so `Session::fly` works it out again from the loaded
//!   overlay. Story-runtime phases 10 (the masters loaded and the
//!   empty slots the landing distance reads; see "Built by phase 10")
//!   and 12 (the frame count and frame index kept from the last rebuild)
//!   read the same state.
//! - **The rebuild points** are the engine's transitions the 17 callers of
//!   `_HandleStellarSystemVisibility` map to (the `stellars` module of the
//!   session lists them): every way into a system but a landed `M` or `N`,
//!   last; a landing, as it docks; the take-off, after the stay's days; a
//!   new pilot, after its `OnStart`; and a pilot flown, which tells
//!   nothing. A rebuild point that changes the set tells
//!   `SessionEvent::StellarsRebuilt`, and the flight view draws the
//!   stellars shown again.
//! - **The frame** that finds a stellar whole is the end of
//!   `Session::tick_combat` in flight, after the shots, as the original's
//!   sprite tick (@0x444d2) follows `_CheckShotProximities` (@0x44015): a
//!   stellar shot down in a frame is destroyed at that frame's look. So
//!   `Y`, `U`, `Y` with no frame between keeps graphic type 0, and with a
//!   frame while whole draws `Type` until the next rebuild point.
//! - **The sheet rule** is the session's since phase 12
//!   (`Session::drawn_sheet`, a [`DrawnSheet`]): a destroyed stellar
//!   draws its `DeadType` sheet when that can be read; otherwise graphic
//!   type 0's while dead since the last rebuild, by the engine's reading;
//!   otherwise its `Type` sheet. The view draws the sheet it names, and
//!   the click box reads the same answer (see "Built by phase 11").
//! - **Problems.** A `DeadType` sheet that exists but cannot be read is in
//!   the scene's problems (`spöb <id> DeadType: <reason>`), as a broken
//!   `Type` sheet is, and so is `spïn` 1000 when it cannot be read and a
//!   stellar of the system might fall back to it, whatever the reading.
//!
//! **Readings.** The Rulebook key
//! [`dead_type_fallback`](crate::RuleKey::DeadTypeFallback) defaults to
//! the engine: graphic type 0 from the next rebuild point. By the Bible's
//! "-1: Don't display different graphic type when destroyed", the stellar
//! keeps its `Type` sheet throughout.
//!
//! ## Checked for phase 9
//!
//! - **The fallback** (@0x33231-0x332d0): for each slot in use, one with
//!   no frames (+0x4c or +0xd4 null, @0x33241-0x33250) is logged "had no
//!   data" when its stellar index (+0xd0) is 0x7ff or less
//!   (@0x33256-0x3329c), then given graphic type 0's master, loaded first
//!   if missing (`_LoadMasterStellarSprite(0)` @0x332ae, `spïn` 1000),
//!   cloned in (`_SWCloneFramesFromMaster` @0x332be), and frame 0
//!   (`_SWSetCurrentFrameIndex` @0x332d0; the frame index is phase 12's).
//!   This is any empty slot, whether its `Type` or its `DeadType` is the
//!   missing one; a whole stellar with an unreadable `Type` is
//!   story-runtime phase 14. When `spïn` 1000 cannot be loaded either,
//!   the clone has no master and the slot stays empty, drawing nothing;
//!   the engine draws its placeholder box and the reason.
//! - **Index -1** of `_masterStellarSprite` is an unwritten word of
//!   `_masterStarSprite` (see "Checked for phase 7"), and other
//!   out-of-range indexes read neighbouring statics. That is a
//!   memory-layout accident, not copied: any `DeadType` outside 0 to 255
//!   is "no sheet".
//! - **The 17 callers** of `_HandleStellarSystemVisibility`, none
//!   indirect: the in-flight `M` and `N` (@0x159a3); `_DoNewPilot`
//!   (@0x1937f), after it destroys the stellars that start destroyed
//!   (@0x192d6-0x1931c), activates the `chär` (`_ActivateCharResource`
//!   @0x19345, its `OnStart`), saves (@0x1935f) and sets the ships up
//!   (@0x1937a); the main screen (`_EnterGameFromMainScreen` @0x1a354,
//!   `_MainScreenChoice` @0x1a8ca, `_DoMainScreen` @0x1b003); `_PlayGame`
//!   (@0x457ee), once before its `_DoPlayGameWork` loop; `_DoPortDialog`
//!   (@0x5fbf0); the hypergate and wormhole exits (@0x63e8c, @0x6473e);
//!   the dock request's take-off (@0x674c7, @0x67653); `_HandlePlayer`'s
//!   escape-pod rescue (@0x69135, after `_EscapeStellarSearch` @0x68f71),
//!   jump arrival (@0x6c203 and @0x6c597) and debug explore key
//!   (@0x6dad1, behind `QuickKeyCheck(0x51)` and the debug flag at
//!   0x3b5144); and `_LoadPilotData` (@0x751cf, @0x75324). The engine
//!   has no rescue (whoever builds it calls the rebuild) and no debug
//!   key. The hidden move (`leave_hidden`, @0x998f1-0x99a36) is not among
//!   them.
//! - **`_PlayGame`** (0x457ba) is called only from
//!   `_EnterGameFromMainScreen` (@0x1a46c), itself called only from the
//!   main screen (@0x1aa7f), and its address appears nowhere else as
//!   data. Play never re-enters it after a modal screen such as the map
//!   or a mission dialog, so it adds no rebuild point beyond a pilot
//!   flown.
//! - **`M` and `N`** (`_EvalSetExp`, `M` @0x1570e-0x158d4, `N`
//!   @0x158d9-0x159ad) each test the landed flag (0x3b5060: `M` @0x15846,
//!   `N` @0x15944) and reach the rebuild at @0x159a3 only in flight (`M`
//!   by @0x158d4, `N` falling through @0x1596e); landed, each only clears
//!   the arrays behind 0x3b5204 and goes on with the expression
//!   (@0x1586b, @0x15969).
//! - **The take-off's two rebuilds**: every jump into @0x674cc-0x67653 is
//!   from inside it (@0x674e9, @0x67530, @0x6753e, @0x675c8, @0x67603),
//!   so @0x67653 is reached only after @0x674c7, on one path: one rebuild
//!   point. The arrival's @0x6c203 and @0x6c597 are likewise one path,
//!   with nothing between them that runs a script or touches the destroyed
//!   state.
//! - **Loading a master** (`_LoadMasterStellarSprite` @0x1eb72) fails
//!   without the `spïn` (`_GetSpriteInfo` @0x1ebb9). At 16 bits it reads
//!   the `rlëD` (@0x1ebd7-0x1ebee); with no `rlëD` it builds the sprite
//!   from the `spïn`'s `PICT` pair instead (`_SWCreateSpriteFromSinglePictXY`
//!   @0x1ece3), which the engine does not read (story-runtime phase 15).
//!   `_SWCreateSpriteFromRLEResource` (@0xa9990) clears the master first
//!   (@0xa99e3) and writes it only on success (@0xa9be7), so a failure
//!   leaves the slot empty, and graphic type 0 after a rebuild; each
//!   failure is reported in a stop alert (`_ReportSpriteLoadError`
//!   @0x1eb62) and play goes on. It reads only the `rlëD`'s header (the
//!   frame size, the depth at +4, the frame count at +8) and makes each
//!   frame at the header's depth (`_SWCreateFrame`, `NewGWorld`
//!   @0xad6b7), decoding no token at load: an `rlëD` whose tokens are
//!   corrupt loads, and one whose header says 8 bits loads as 8-bit
//!   frames. The engine reads either as no sheet (it cannot draw them)
//!   and reports it; story-runtime phase 15 owns the 8-bit sheet.
//! - **Stock data**: no stock `spöb` has a `DeadType` in 0 to 255 whose
//!   sheet cannot be read, and `spïn` 1000 reads, so the stock scenes
//!   report no new problem (`stock_system_view`).
//!
//! ## Built by phase 10
//!
//! The player lands within the original's distance: a box around the
//! stellar, both the x and the y distance strictly below trunc(1.75 × the
//! width), the width `_GetStellarSpriteXSize`'s, read from the `Type`
//! master. It is 150 while that master is not loaded, so a destroyed
//! stellar lands within 262 from the next rebuild point until a flight
//! frame finds it whole, whatever its `Type`'s width, unless another
//! stellar shown keeps its `Type` loaded.
//!
//! - **The rule** is the `landing` module's: `LandingReach` (a box or a
//!   circle) and `landing_reach`, which reads the site, a `StellarShown`
//!   and the rule source. `check_landing` and `land_or_select` take the
//!   reach, so landing, docking at a station, entering a gate or a
//!   wormhole, and the choice between `TooFar` and `TooFast` all read it.
//! - **The state** is phase 9's `Rebuilt`, which now also keeps the
//!   graphics whose master is loaded and the stellars whose slot shows
//!   no frame, at the same rebuild points and flight frames (see the
//!   `rebuild` submodule). The session reads whether `spïn` 1000 reads
//!   as it starts (phase 11 turns that into `PilotCatalog::
//!   fallback_frame_size`, the sheet's frame size), and
//!   each `LandingSite` carries its `Type`, its `DeadType` when that
//!   sheet reads, and whether its `Type`'s `spïn` counts frames.
//! - **Not switched**: NPC landing keeps the `landing_radius` circle
//!   (story-runtime phase 16, npc-landing-reach), and so does the disc
//!   the shots hit (phase 8: the original tests shots pixel by pixel).
//!   Selecting the nearest stellar, clicking one, and the views are
//!   unchanged.
//!
//! **Readings.** The Rulebook key
//! [`landing_distance`](crate::RuleKey::LandingDistance) defaults to the
//! engine's box. The Bible says nothing of a landing distance; the other
//! reading is the port's earlier circle, half the larger side of the
//! `Type` frame, or 32 with no sprite, whatever was rebuilt.
//!
//! ## Checked for phase 10
//!
//! - **`_GetStellarSpriteXSize`** (@0xac91-0xad37) gives, in order: 150
//!   for a stellar index above 0x7ff (@0xac9c); 150 when the stellar's
//!   +0x14 is not the player's system (+0x74 of the first ship,
//!   @0xacb9-0xacc1; a stellar in two visible systems, story-runtime
//!   phase 18, stellar-in-a-second-system-reach); 150 when `Type` (+0x0e)
//!   is outside 0 to 255 (@0xacc3-0xacd0), its `_masterStellarSprite`
//!   entry is null (@0xacdf) or that master has no current frame (+0x50,
//!   @0xace6); 32 when the master's frame index (+0x68) is below 0 or
//!   not below its frame count (+0x54, @0xaced-0xacff); otherwise the
//!   master's rect width, +0x20 less +0x1c (@0xacf4-0xacf8). Each 150
//!   is logged and returned at @0xad2d.
//! - **The 32 is reachable**, unlike the scouting's guess. A master's
//!   frame index is 0 from `_SWSetCurrentFrameIndex(sprite, 0)` as
//!   `_SWCreateSpriteFromRLEResource` finishes (@0xa9bdc; the write at
//!   @0xa9add is to the frame, not the sprite), and nothing sets a
//!   master's frame index after: of the 16 reads of the
//!   `_masterStellarSprite` table (0x3b5040), only `_LoadMasterStellarSprite`
//!   writes into a live master (the rebuild's free @0x32e10 and
//!   `_DestroySpriteWorld`, `_Bye` and `_SetupSpriteWorld` only take it
//!   apart or clear the table), and the clones
//!   (`_SWCloneFramesFromMaster` @0xac436) write only the slot. But `_LoadMasterStellarSprite` then
//!   overwrites the master's frame count with the `spïn`'s `xTiles` ×
//!   `yTiles`, multiplied in 32 bits (@0x1ed8c-0x1ed97). So a loaded
//!   master whose `spïn` tiles multiply to 0 or less reads 32, and the
//!   dock request's reach is trunc(32 × 1.75) = 56. The engine carries it
//!   as `TypeWidth::OutOfRange` from `LandingSite::type_counts_frames`.
//!   Stock `xTiles` × `yTiles` always equals the frame count, so stock
//!   never reaches it.
//! - **Such a `spïn` shown in a slot** has more effects the engine does
//!   not copy, filed as story-runtime phase 19
//!   (no-frame-count-spin-in-a-slot): a clone copies the count
//!   into the slot (@0xac4ec) and sets its frame through
//!   `_SWSetCurrentFrameIndex`, which divides by it (`idivl` @0xa90e7),
//!   so a product of 0 divides by zero, and with a negative one it
//!   returns before setting the frame (@0xa90ee), leaving the slot's
//!   frame as it was: none after a rebuild, which empties it (@0x32e7a)
//!   without filling it with graphic type 0, as its frames are not null
//!   (@0x33241-0x33250).
//! - **The dock request** (`_HandlePlayerDockRequest`): after the
//!   clearance state 0x3b544c above 0x2ed (@0x66c18), the ship's +0x4c at
//!   most 0 (@0x66c32) and each velocity axis at most 0.75 (the float at
//!   0xdd658, @0x66c3c-0x66c66; story-runtime phase 17,
//!   per-axis-landing-speed), it takes |trunc(dx)| and |trunc(dy)|
//!   (@0x66c78-0x66cc3), sets the reach to 75 (@0x66cc7), and when the
//!   slot (the stellar's +0x00) has a current frame (+0x50, @0x66ccf)
//!   replaces it with cvttsd2si(width × 1.75), 0xdd6e8 being the double
//!   1.75 (@0x66cd9-0x66ceb). It lands only if the reach is above both
//!   (@0x66cef-0x66cfd), a strict box; with an integer reach,
//!   |trunc(d)| < reach is |d| < reach. The `cwtl` and `cmpw` narrow the
//!   distances and the reach to 16 bits, a word wrap not copied. A
//!   failed request computes the same reach the same way
//!   (@0x67720-0x677b2) and replies "too fast" (`STR#` 2002 #69-72,
//!   @0x677c2-0x677f8) inside it and "too far" (#65-68, @0x67802-0x67838)
//!   outside; a failed distance check also jumps to "too far" (@0x67687
//!   to @0x67806).
//! - **NPC landing** (`_HighLevelAIHandler` @0x8dbdf, AI state +0xc8c8
//!   0x14) reads the same width as a quarter, rounded towards 0
//!   (@0x8dbf1-0x8dbfa), inclusive on each axis (@0x8dc1b, @0x8dc40),
//!   with no 75 case and no speed gate: story-runtime phase 16. Nothing
//!   else calls `_GetStellarSpriteXSize`, and indirect.txt takes its
//!   address nowhere.
//! - **The masters freed** (@0x32d18-0x32e4e): a rebuild marks the
//!   graphic each stellar shown shows (`DeadType` when destroyed,
//!   @0x32d18-0x32d38) whose master is loaded, frees every master not
//!   marked, then loads the master of each stellar's shown graphic
//!   (@0x32f37-0x32f46) and clones it in (@0x32ff6); a slot left with no
//!   frames gets graphic type 0's (@0x33241-0x332d0). The sprite tick
//!   (`_HandleStellarSprites` @0x2e7be-0x2e7e5) loads and clones the
//!   shown graphic's master for every slot on every frame. So a `Type`
//!   master is unloaded exactly from a rebuild point at which no stellar
//!   shown shows it until a frame at which one does.
//! - **The slot shows no frame** when its +0x50 is null: a rebuild
//!   nulls it (@0x32e7a) and only a clone of a master with frames sets
//!   it, so a stellar showing no readable graphic, with `spïn` 1000
//!   unreadable too, shows none (75) until a frame finds a graphic it can
//!   clone.
//!
//! ## Built by phase 11
//!
//! A click in space picks a stellar by the frame of the sheet drawn for
//! it, so a destroyed stellar is clicked by its `DeadType` sheet, or by
//! graphic type 0's once a rebuild point has filled its empty slot.
//!
//! - **The rule** is `navigation::stellar_under`, which now takes the
//!   frame size drawn for each site from its caller; `None` still gives
//!   the 64-pixel placeholder.
//! - **The sheet drawn** is decided once, by `Session::drawn_sheet` (the
//!   session's `stellars` module; one owner since phase 12): the
//!   `DeadType` sheet while destroyed, when there is one
//!   (`LandingSite::dead_frame_size`); graphic type 0's while destroyed
//!   and dead since the last rebuild point, by
//!   [`dead_type_fallback`](crate::RuleKey::DeadTypeFallback)'s engine
//!   reading; the `Type` sheet otherwise. `Session::drawn_frame_size`
//!   gives that sheet's frame (graphic type 0's from
//!   `PilotCatalog::fallback_frame_size`, read as the session starts),
//!   and the flight view's `SceneStellar::drawn` draws the sheet the
//!   same answer names. With graphic type 0's sheet unreadable no sheet
//!   is drawn, and the placeholder is clicked.
//! - **Readings.** No new key: the Bible says nothing of the click's
//!   target, and the click follows whichever sheet is drawn, so
//!   `dead_type_fallback` decides it under both readings. By the Bible's
//!   no graphic type 0 is drawn, so none is clicked.
//! - **Not switched**: the margin's width test, the half-open edges and
//!   the order among overlapping stellars (below) are story-runtime phase
//!   20 (click-target-shape), as is the stellar's +0x44 skip flag.
//!
//! ## Checked for phase 11
//!
//! - **The rectangle** (`_HandlePlayer` @0x6a5f9-0x6a717): for each of
//!   the 16 stellar sprite slot pointers at 0x3b53d0, the loop skips a
//!   null slot (@0x6a603), one whose visible byte +0x14 is clear
//!   (@0x6a60b) and one whose stellar record's +0x44 is clear
//!   (@0x6a615-0x6a637), then copies the slot's +0x1a rect to
//!   -0x38(%ebp) (@0x6a63d-0x6a646). The pointer at -0x3ec(%ebp) that
//!   `_InsetRect` and `_PtInRect` read is `lea -0x38(%ebp)`, stored at
//!   @0x6a356-0x6a359.
//! - **The layout** of +0x1a is a QuickDraw `Rect`, `top, left, bottom,
//!   right`: `_SWSetCurrentFrameIndex` copies it from the frame's +0x22
//!   (@0xa9139-0xa9149) after moving +0x1a by the vertical hot spot
//!   (+0x2c, @0xa911f-0xa9132) and +0x1c by the horizontal one (+0x2a,
//!   @0xa9105-0xa911b). So the rect is the current frame of the sheet
//!   cloned into the slot, the one phase 7 draws: `DeadType`'s master
//!   while destroyed, `Type`'s otherwise (@0x2e77b-0x2e7e5), graphic
//!   type 0's in a slot a rebuild left empty (@0x332ae-0x332d0).
//!   `_HandleStellarSprites` centres it on the stellar every play frame
//!   (@0x2ee30-0x2ef1a).
//! - **The margin's test reads the width**: `movswl -0x36(%ebp)` (left)
//!   and `movswl -0x32(%ebp)` (right), `subl`, then `cmpl $0x2f` and
//!   `jg` past the `_InsetRect(r, -16, -16)` (@0x6a649-0x6a671). The
//!   engine's `SMALL_SPRITE_HEIGHT` compares the height: story-runtime
//!   phase 20.
//! - **A rectangle, not a mask**: `_PtInRect` (@0x6a686), half-open on
//!   the right and bottom, where the engine includes every edge; and the
//!   first slot holding the point wins, the slots filled by ascending
//!   `spöb` index (@0x32eda-0x32f06), not `NavDef` order. Both are phase
//!   20.

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
//!   and no `galaxy_escorts` key is needed. It leaves the stellar the
//!   player is docked at alone: run at a landing (@0xa175f, @0xa1af2), the
//!   spaceport goes on at that stellar, and the new system's ships and
//!   stellars are set up only as the take-off sets the system up
//!   (`_SetupShipsInSystem` @0x674c2, the stellars @0x674c7).
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
pub(crate) mod rebuild;
pub(crate) mod wreck;

pub use dispatch::DuplicateSubscription;
pub use galaxy::{
    Countdown, GalaxyOverlay, HiddenMove, OverlaidGalaxy, StellarOverride, SystemOverride,
};
pub use rebuild::DrawnSheet;

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
