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
//! follow it, the section names the [`Rulebook`](crate::rulebook::Rulebook) key the
//! building phase adds and its readings. Phase 2 adds `stay_days`, phase 3
//! `story_reentry`, phase 5 `destroy_invincible`, phase 8 `y_on_destroy`,
//! phase 9 `dead_type_fallback`; crons-news-ranks phase 5 adds
//! `colocated_resolution` and `visibility_refresh`.
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
//!    a later cron in the pass sees the bits an earlier one set. Built by
//!    crons-news-ranks phase 1: see the [`cron`](crate::cron) module.
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
//!   off from a gate. A pilot docked at one, by the engine's own
//!   relocation, passes the day on its take-off as any other.
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
//!   dialog (@0x4f060) on its next call, once the texts and dialogs shown
//!   over the port have closed, and `_DoPortDialog` leaves by the same
//!   close (@0x5fcc6-0x5fcd0, @0x60196, @0x601a0-0x6020e). Here the
//!   session holds the leave (`nova_sim::Session::leave_due`)
//!   and the take-off, through the port's Leave, carries it out.
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
//!    (@0x6bfce) and draws the table again (@0x6c040-0x6c066), as any
//!    mission's abort, failure or success forgets it (@0x99bcf, @0x99db5,
//!    @0xa03ef, @0xa0691), and a load (@0x75ef5). A take-off does not, so
//!    landing again on the same stellar keeps its offers. Built by
//!    missions phase 1 (see [`mission`](crate::mission)).
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
//!    and, if any mission succeeded, re-offers (@0xa1b04): availability
//!    and resolution again, the table kept. Here each of those four steps
//!    is the slot's own `Landed` subscription, `{Missions, slot, 0..=3}`,
//!    and the landing walks them live (see "One dispatch"), so a mission
//!    an `S` puts in a later slot is checked in full at this landing, one
//!    put in the slot just freed gets only the trailing check, and one put
//!    in a lower slot waits for the take-off, as the original's loop does.
//! 4. The news is chosen as the bar opens: `_DoBarDialog` (0x48ef5)
//!    calls `_RandomizeNewsDialog` (0x48238) as it opens (@0x48f15). It
//!    has no trigger point of its own (see [`news`](crate::news)).
//!
//! ## One take-off
//!
//! As the spaceport closes: the escorts (@0x601a0), then the outfitting
//! and ship days (@0x601fa, @0x6020e). Then `_PlayerLandOnStellar` passes
//! the take-off's day (@0x63440), runs the global price flux (@0x63445),
//! explores the system (`_AutoSetExploration` @0x634c8) and its nebulae
//! (@0x634da), restarts each mission's aux ships (@0x634df-0x6353c:
//! with `Flags` 0x0010 those left to come refilled, +0x6e; their
//! countdown, +0x6c, drawn again; their arrived count, +0x6a, zeroed) and
//! saves the pilot (@0x6354b). Here that restart is each slot's
//! [`Consumer::MissionShips`] subscription on `TookOff`, before the
//! system's setup and its draws (see the session's `aux_ships` module). Back in
//! `_HandlePlayerDockRequest`, the system's ships are set up
//! (`_SetupShipsInSystem` @0x674c2) and its stellars redrawn (@0x674c7).
//! The enter-system hook does not run.
//!
//! ## One arrival
//!
//! `_HandlePlayer`'s jump arrival, in order: the nebulae of every system
//! the jump passes and arrives at (`_ExploreNebulaeFromSystem` @0x6bddb,
//! @0x6be12); the galaxy refresh (@0x6bf3f); the offers' stellar
//! forgotten (@0x6bfce) and their table drawn again (@0x6c040-0x6c066,
//! read only by an offer in space until the next landing, missions phase
//! 6); the days (@0x6c144); the global and local price
//! flux (@0x6c153, @0x6c158); the enter-system hook
//! (`_MissionHandlePlayerEnteredNewSystem` @0x6c15d); the system explored
//! (@0x6c1db); its ships set up (@0x6c1f9) and a bounty hunter
//! (@0x6c1fe); its stellars redrawn (@0x6c203); the escorts' pay
//! (@0x6c620).
//!
//! The enter-system hook (@0x99f11) runs no script. For each active
//! mission, in slot order, it restarts the timers its ships appear by:
//! with `ShipStart` 1, ships left (+0x6) and the player in their system
//! (or `ShipSyst` -6), the special ships' appear timer (+0x4e), their
//! arrived count (+0x2e) zeroed (@0x99f5b-0x99fae); and its aux ships,
//! as the take-off restarts them (@0x99fb5-0x99ff4). On a hypergate's or
//! wormhole's exit, the aux ships of `AuxShipSyst` -1 are then lost until
//! the next restart (@0x6729b-0x672ac, as
//! [`RuleKey::AuxGate`](crate::RuleKey::AuxGate) reads). Here it is the
//! [`Consumer::MissionShips`] subscription each slot registers on
//! `EnteredSystem` (see the
//! [`traffic::spawn`](crate::traffic::spawn) module's "Mission ships"). It
//! runs on a jump's arrival (@0x6c15d), a gate's or wormhole's exit
//! (@0x67159) and an `M` or `N` in flight (@0x159a8); not on a landed `M`
//! or `N` (@0x15947, which only empties the docked offers), nor on a
//! take-off, which restarts only the aux ships' countdowns.
//!
//! ## Ship goals
//!
//! Goals are counted where they happen and checked by polling.
//! `_DoMissionAccept` zeroes the goals met, the failure and the counts
//! (@0xa1c0e-0xa1c59).
//!
//! - **Destroyed** (+0x28) is counted at a mission ship's final explosion,
//!   whoever destroyed it (`_HandleShipDisplay` @0x2dbd4), with its ships
//!   left (+0x6) one fewer. The first destroyed fails its mission there
//!   and then (`_QuickMissionFailure` @0x2dbba) for a goal to disable or
//!   escort them, or to board or rescue them with this one not boarded,
//!   unless the mission is failed already or hidden (`Flags` 0x0400).
//! - **Disabled** (+0x2c) is counted as a mission ship is newly disabled
//!   (`_DamageShip` @0x3af8b); the first fails an escort goal at once
//!   (@0x3af71), on the same terms.
//! - **Boarded** (+0x2a) is counted only on two paths of
//!   `_HandlePlayerBoardAttempt`: the `PickupMode` 2 cargo pickup
//!   (@0x653a7, mission-ships phase 14) and the assist path, for a board
//!   or rescue goal of `Flags` 0x0001 with one ship left (@0x65730), which
//!   says `STR#` 2002 #126 or, with `Flags` 0x0008, the refuelled ship's
//!   thanks. Any board of a mission ship whose mission has not failed
//!   then skips the `BoardPenalty`, the persons and the plunder
//!   (@0x657bf-0x657e3).
//! - **Chased off** (+0x30) is counted as a mission ship leaves the
//!   system, at the end of its warp out (`_LowLevelAIHandler` @0x8598a),
//!   for a chase-off goal (`ShipGoal` 6) whose ships do not follow the
//!   player (`ShipSyst` -6), with its ships left one fewer so it is not
//!   brought in again; a ship of any other goal that leaves counts
//!   nothing and comes back at the next setup
//!   ([`left_respawn`](crate::RuleKey::LeftRespawn) has the other
//!   reading). An AI ship that lands stays in the system, stopped at the
//!   stellar for its wait (`_HighLevelAIHandler` @0x8dadc-0x8db5a), then
//!   takes a new goal, so a landing counts nothing and only its later jump
//!   out may count ([`landed_leaves`](crate::RuleKey::LandedLeaves) has
//!   the other reading, which takes it out as it lands). One that leaves
//!   through a hypergate or wormhole is gone, but counts nothing either
//!   ([`gate_exit_counts`](crate::RuleKey::GateExitCounts)). Here the
//!   traffic's tick raises [`Fate::Departed`] for such a ship and counts
//!   it. Chase off is met while those destroyed and those chased off
//!   together reach `ShipCount`, and unmet otherwise (@0x9eb48-0x9eb78).
//!
//! Every frame `_DoPlayGameWork` (@0x44214) runs `_MissionHandler`
//! (@0x9eda8), which checks each slot's goals (`_MissionObjectivesCheck`
//! @0x9edb5): the counts against `ShipCount`, unmet while ships are still
//! to come; whether the escorted ships are still with the player
//! (@0x9e931-0x9e980), which fails an escort goal once one is destroyed
//! or disabled, as one destroyed fails a disable goal; and whether an
//! observed ship is visible to the player (`_ShipVisibleToShip`
//! @0x9ea5c: not emerging from a gate, and uncloaked or within a cloak
//! scanner's sight, see [`cloak`](crate::cloak)). Only a ship that can
//! cloak must also be on screen (@0x9ea74-0x9eb09,
//! [`cloak::on_screen`](crate::cloak::on_screen)). The goals met now and
//! not before, unfailed, show `ShipDone` (@0x9ed0f), run `OnShipDone`
//! for a mission with a `ShipGoal` (@0x9ed70), and auto-abort a mission
//! of `Flags` 0x0001 (@0x9ed87). A mission of no goal (`ShipGoal` -1)
//! is met at once, except one with ships, `Flags` 0x0001 and `ShipStart`
//! 1 none of whose ships has arrived: the check first clears its goals
//! met while ships are left (@0x9e7ef-0x9e80a) and then leaves them
//! (@0x9ed8e-0x9ed98), so it waits for them. A mission of `Flags`
//! 0x0001 also auto-aborts, whatever its goal, once per slot as its
//! special ships spawn: after the system's setup brings them
//! (`_SetupShipsInSystem` @0x4317e) and after its appear timer brings
//! them in by hyperspace (`_EnterMoreShips` @0x43ad4); its ships stay as
//! ordinary traffic. The same poll fails a mission whose
//! days left reached 0 (@0x9eba5-0x9ec61), so a deadline fails after its
//! transition has finished. The landing check runs it too (above).
//!
//! Here the poll is the [`Consumer::MissionShips`] subscriptions each slot
//! registers (see the session's `mission_ships` module): a check on every
//! `ShipFate`, on
//! `TookOff` (the first flight frame after a landing or a docked accept,
//! held until the take-off's setup has run, as the original sets the
//! system up first, @0x674c2), and in flight on `EnteredSystem`; an accept in flight is checked once
//! its settling is done. The ships' arrivals raise
//! [`Fate::Arrived`], their first sighting [`Fate::Observed`] and their
//! leaving [`Fate::Departed`], so an escort's presence, an observation
//! and a chase-off are checked as they change. A landing's two checks
//! are the slot's own [`Consumer::Missions`] hooks, before its cargo and
//! after its return (see "One landing").
//!
//! Mission cargo jettisoned (`_JettisonCargo` @0x3e18c), found by a scan
//! (`_ScanPlayer` @0x7e7ac) or plundered by an NPC
//! (`_AIPlunderShipContents` @0x84a0f) fails its mission at once. The
//! jettison and the scan are built (see the [`mission`](crate::mission)
//! module's "Cargo"); the plunder, and `mïsn` `Flags` 0x8000 ("fails if
//! boarded by pirates"), wait for NPC boarding.
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
//! reach the galaxy. It clears every stellar's owner (@0x99684-0x9969f);
//! evaluates every system's `Visibility` (@0x996a1-0x996eb), an empty test
//! true (@0x995ad-0x995b4); gives each stellar its owner by the
//! chain-shadow rule (@0x996ed-0x997bd): visiting the systems by
//! ascending ID, a visible system not yet done claims each of its
//! stellars still unowned, then marks every higher system at its place
//! done, so the owner is the lowest-ID visible system that lists the
//! stellar and is not shadowed by a lower visible system at its place,
//! and a stellar listed only by hidden or shadowed systems has none;
//! evaluates the `shïp` `AppearOn` (read by `_SelectShipFieldFromDude`
//! and `_HyperSpawnFleet`), the `përs` and `flët` `ActiveOn`, the `mïsn`
//! `AvailBits` and the `nëbu` `ActiveOn` (@0x997c3-0x998ef); and, if the
//! player's system is now hidden, moves the player and every ship in the
//! system to the visible system at the same place or, with none, makes
//! the first system at that place (the chain head) visible and moves them
//! there (@0x998f1-0x99a36; see "Checked for phase 4"). The move comes
//! after the ownership, so a chain head it shows owns nothing, and stays
//! visible only until the next refresh hides it again. It plays no sound.
//! It runs at a landing (with the offers @0xa175f, only when they roll,
//! and after the land check @0xa1af2), on a jump's arrival before its
//! days (@0x6bf3f), for a new pilot (`_DoNewPilot` @0x1925e, before the
//! chär `OnStart` @0x19345, so `OnStart`'s bits show at `_PlayGame`'s call
//! @0x457e9), at a load (`_LoadPilotData` @0x751ca, @0x7531f), from the
//! main screen (`_EnterGameFromMainScreen` @0x19f81, `_MainScreenChoice`
//! @0x1a8aa, `_DoMainScreen` @0x1afcf) and at start-up (`_InitEVNova`
//! @0x472ba, @0x4740c, before any pilot exists); never right after a
//! script. So a bit a cron sets on a jump's days shows at the next landing
//! or arrival.
//!
//! Crons-news-ranks phase 5 builds it: `GalaxyOverlay::refresh_visibility`
//! writes each system's visibility from its `Visibility`, and
//! [`OverlaidGalaxy::owner`] derives each stellar's owner from the overlay,
//! never stored, as [`RuleKey::ColocatedResolution`](crate::RuleKey::ColocatedResolution)
//! reads: by the engine the chain-shadow rule above; by the other reading,
//! with every visible system standing for itself, the lowest-ID visible
//! system listing it. The session refreshes at a landing, a jump's arrival
//! (evaluated before its days, written once the hops and the systems
//! explored are resolved by the flags as they stood), after a new pilot's
//! `OnStart` and, quietly, at a load, which covers Open Pilot's call
//! (`_MainScreenChoice` @0x1a8ca); a return from play to the main screen
//! keeps the session as play left it, as `_DoMainScreen`'s own setup runs
//! once a launch. By the other
//! reading of [`RuleKey::VisibilityRefresh`](crate::RuleKey::VisibilityRefresh)
//! it also refreshes after every settled script that changed the pilot.
//! The availability tests stay live reads at each spawn, where the
//! original caches them until the next refresh (left to rdm
//! roadmap/crons-news-ranks-fidelity).
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
//! `_ExploreNebulaeFromSystem` (@0x508f) marks explored (@0x515a), then
//! runs the `OnExplore` (@0x519e) of, every active nebula not yet
//! explored whose rectangle, inset by 8, holds the system (`PtInRect`,
//! half-open), so each runs once until the flag is forgotten. It runs
//! for every system an exploration reaches (`_RecursiveAutoExplore`
//! @0xd98a: an arrival after its days, a take-off, a map granted), for
//! the systems a jump passes and arrives at before its refresh and its
//! days (@0x6bddb, @0x6be12), and at the take-off (@0x634da). `X`
//! explores no nebula: it writes the explored level itself (@0x15c71),
//! and `_EvalSetExp` (0x150fc up to `_EvalTestExp`) calls none of these.
//! A `G` of a map does, through `_GrantOutfitItem` (@0x15444). A gate's
//! or wormhole's exit explores none (`_PlayerEnterHypergate` 0x637bf,
//! `_PlayerEnterWormhole` 0x64005 and `_HandlePlayerDockRequest` call
//! neither `_ExploreNebulaeFromSystem` nor `_AutoSetExploration`).
//!
//! A new pilot explores none: `_ResetGameData` clears every nebula's
//! active and explored bytes (@0x18de5, @0x1df44-0x1df4b), and
//! `_DoNewPilot` explores its start system (@0x19118) before its first
//! refresh (@0x1925e), while every nebula is inactive; `_MainScreenChoice`
//! skips its sweep after a new pilot (@0x1a7d2). The explored flag is
//! never saved: `_ResetPlayer` clears it (@0x1da86) before every load,
//! and after the load's refresh Open Pilot's `_MainScreenChoice`
//! (@0x1a8d4-0x1a906), and `_OpenLastPilotFile` at launch
//! (@0x765a2-0x765ce), explore every visible explored system by
//! ascending index, so `OnExplore` runs again after every load.
//! `_DoMainScreen` (@0x1afcf-0x1affe) is no sweep: it explores only the
//! player's system, clears no flag and runs once a launch (its one
//! caller is `_InitEVNova` @0x47428), so a return from play explores
//! nothing. Activity
//! (`ActiveOn`, both sizes above 0) is written at each refresh
//! (@0x998b4-0x998ef) and read only by the exploration and the maps,
//! which draw a nebula active and explored. See the
//! [`nebula`](crate::nebula) module.
//!
//! Built by crons-news-ranks phase 16: each nebula is a
//! [`Consumer::Nebulae`] subscription on every `Explored`, its rectangle
//! and explored flag asked as its consumer's predicate (see "Dispatch");
//! its activity is written at every refresh point beside visibility; a
//! load forgets the explored flags and sweeps the visible explored
//! systems, the launch's load of the last pilot too (main-screen phase 4,
//! `nova_sim::PilotKeeper::open_last`); the launch then explores the
//! player's system once, as `_DoMainScreen` does
//! (`nova_sim::Session::open_at_launch`); and the Rulebook keys
//! [`nebula_explore_once`](crate::RuleKey::NebulaExploreOnce) and
//! [`nebula_new_pilot`](crate::RuleKey::NebulaNewPilot) offer the other
//! readings (see the session's `nebulae` module).
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
//! `nova_sim::SessionEvent` outbox, which carries no data and
//! is emptied by `nova_sim::Session::take_events`.
//! Each event has one source, at a transition point that also emits a
//! `SessionEvent` or runs a step:
//!
//! | Event | Its one source | The original's |
//! |---|---|---|
//! | [`EnteredSystem`](StoryEvent::EnteredSystem) | `enter_system`'s `announce` step, for every [`Entrance`] | the enter-system hook |
//! | [`Landed`](StoryEvent::Landed) | `Session::dock`, as `nova_sim::Session::land` docks | the spaceport opening |
//! | [`TookOff`](StoryEvent::TookOff) | `nova_sim::Session::take_off`, after its day | `_PlayerLandOnStellar` after the spaceport |
//! | [`DayPassed`](StoryEvent::DayPassed) | the daily step, as its last participant | `_IncrementGameTime` |
//! | [`Explored`](StoryEvent::Explored) | exploring a system: a jump's arrival, for each system it passes and the one it arrives at; every other way in whose row explores but a gate's or wormhole's exit; a take-off; a map granted, for each system it reaches; `nova_sim::Session::fly`, for each visible explored system (a new pilot's withdrawn by `nova_sim::Session::begin` under the engine's [`nebula_new_pilot`](crate::RuleKey::NebulaNewPilot)). `X` raises none | `_ExploreNebulaeFromSystem` |
//! | [`ShipFate`](StoryEvent::ShipFate) | combat's destruction and disabling (a ship broken up while intact is disabled too); the session's boarding, repelled or not, a mission ship's included (a `PickupMode` 2 pickup's too; a refused pickup raises none); the system's setup and the appear timer's arrivals, for each mission ship they bring in ([`Fate::Arrived`]); the traffic's tick, for each mission ship that leaves the system ([`Fate::Departed`]); and the traffic's flight tick for a watched ship first seen (`nova_sim::Session::watch_ship`) | the goal counts and the per-frame poll |
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
//! explores, but a gate's or wormhole's exit, which explores no nebula.
//! Each exploration raises `Explored`, the system explored
//! already or not. Flying a pilot, a new one or a load, raises the
//! `Explored` of every visible explored system, by ascending ID, as the
//! original's Open Pilot sweeps their nebulae (`_MainScreenChoice`
//! @0x1a8d4-0x1a906; a new pilot has explored only its own,
//! `_DoNewPilot` @0x19118), and no `Landed`: a load resumes in flight and
//! lands nothing, as `_EnterGameFromMainScreen` and `_PlayGame` call no
//! landing, but forgets the stellar the offers were rolled at (@0x75ef5),
//! so the first landing rolls them afresh.
//!
//! The events wait on the session, in the order raised, for its next
//! `nova_sim::Session::settle_script`, which
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
//!   `nova_sim::Session::run_set` runs one, its operators
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
//! `nova_sim::Session::register_story` and
//! unregisters through
//! `nova_sim::Session::unregister_story`, which
//! clears the once-fired entry too. The stock consumers register at
//! `nova_sim::Session::fly` from their catalog data: so far
//! the missions' offer roll (missions phase 1) and re-offer, and each
//! active slot's hooks (missions phase 2). A mission's subscriptions, and
//! its ships', are registered when it becomes active, keyed by its slot. When it ends (success,
//! failure, `A` or an auto-abort), they are unregistered and their
//! once-fired entries cleared, so a mission accepted into the reused slot
//! starts fresh.
//!
//! **One dispatch.** A dispatch takes a snapshot of the matching ids, in
//! [`dispatch_order`]'s order; an event no subscription matches costs
//! nothing. For each id in turn, just before it runs, the dispatcher
//! skips it when it is no longer registered (a subscription removed by an
//! earlier one in the same dispatch), when it is once-only and has fired,
//! when its guard does not hold now (an earlier script can open or close
//! it), or when its consumer's own predicate refuses it (`Stage::admits`:
//! a nebula's rectangle and explored flag, which no control-bit test can
//! express; crons-news-ranks phase 16). Otherwise a once-only subscription is marked fired first,
//! so an event raised again inside its own run cannot run it twice; then
//! its consumer's effect runs, then its script. One registered during the
//! dispatch waits for the next event, except on a `Landed` event, which
//! walks live: after each id it asks for the matching ids again and takes
//! the first that sorts after the last one considered, so one registered
//! mid-landing runs in this landing when it sorts after the cursor. This
//! mirrors `_MissionLandCheck` reading each slot's active flag as it
//! reaches the slot (@0xa19f7-0xa1aec): a mission an `S` puts in a later
//! slot is checked in full at this landing.
//!
//! **Order.** [`dispatch_order`] runs an event's subscriptions by id:
//! consumer, then record, then hook. The consumers' order is
//! [`Consumer`]'s, the same for every event, because only one event has
//! several stock consumers in the original: a landing, which rolls the
//! offers before the spaceport opens (@0x66d62), then checks each
//! mission in turn (@0xa1a0c-0xa1ada). So [`Consumer::Offers`], whose one
//! subscription is the roll, `{Offers, 0, 0}`, comes first, then
//! [`Consumer::Missions`]. Missions go by slot, as the original's land
//! check (@0xa19f7), and a slot's landing hooks go goals, cargo, return
//! (failure, else the escort goal, then success) and the goals again;
//! the re-offer, `{Missions, 16, 0}`, comes after every slot.
//! Plug-ins run after every stock consumer: an engine decision with no
//! Rulebook key, since the original has no plug-ins.
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
//! **Where it nests.** `nova_sim::Session::run_set`
//! stays catalog-free: `apply_op` only queues a move or a `Q`, since a
//! script has no catalog to read the new system from (see
//! `script_effects`). The one place that dispatches is
//! `nova_sim::Session::settle_script`, which holds
//! the catalog and which the program calls after every input and tick.
//! The landing, the take-off, the daily step, combat and boarding have no
//! catalog, so the events they raise wait there. It settles in a loop:
//! the moves queued (and the `EnteredSystem` an `enter_system` they cause
//! raises), then the next event raised, first in, first out, dispatched,
//! and once none is left a `Q`'s leave in flight, whose events dispatch
//! in the same settling. A `Q` run while landed is held instead, as the
//! original holds `_absquatulateStr`
//! (`nova_sim::Session::leave_due`), and
//! `nova_sim::Session::leave_if_due` carries it
//! out as the port's Leave
//! (`nova_sim::Session::leave_port`) does: it takes
//! off and settles again at once itself, so the take-off's events
//! dispatch before its save and its first frame. The
//! subscriptions' scripts run through the dispatcher's
//! runner, which, after each operator handed to `apply_op`, settles the
//! moves it queued, dispatches the events raised, depth first, and only
//! then runs the next operator. So for the scripts the dispatcher runs,
//! the original's nesting in the middle of a script is matched. A `Q`
//! only sets the message (@0x15bee), as the original's does, so it is
//! never settled in the middle of a script.
//!
//! A script run through `run_set` in the middle of something else defers
//! instead. Those are the record hooks
//! `nova_sim::Session::run_script` runs (see the
//! `hooks` module):
//!
//! - `chär` `OnStart`, in `nova_sim::Session::begin`;
//! - `oütf` `OnPurchase` and `OnSell`, in
//!   `nova_sim::Session::outfit`;
//! - `shïp` `OnRetire` and `OnPurchase`, in
//!   `nova_sim::Session::buy_ship`;
//! - `shïp` `OnCapture`, and `OnRetire` on Use As My Ship, in
//!   `nova_sim::Session::plunder` and
//!   `nova_sim::Session::assign`.
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
//! each other for ever. Only an event that runs a subscription counts: it
//! spends the budget as its first subscription runs, so an event whose
//! subscriptions all hold back costs nothing, and a load's sweep that
//! raises `Explored` for every explored system cannot drop events
//! (crons-news-ranks phase 16). The rest are dropped, and the settling tells how many as one
//! [`ScriptNote::EventsDropped`](crate::control::ScriptNote::EventsDropped). The
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
//! destroyed stellars' regeneration, phase 6 the day rolls, phase 8
//! the damaged stellars' repair, crons-news-ranks phase 1 the crons and
//! crons-news-ranks phase 3 the ranks' salaries; the others are empty
//! slots until their phase:
//!
//! 1. the date moves on a day;
//! 2. the crons, each starting or ending cron's script run as it is
//!    reached (crons-news-ranks phase 1; see [`cron`](crate::cron)): run
//!    as a participant's effect through `Session::run_set`, as `OnRegen`
//!    is, each named by `cron::cron_hook` and never registered, its `M`,
//!    `N`, `Q` and `S` deferred to the next settling as any hook's are;
//! 3. the active missions' days left (missions phase 2);
//! 4. the dominated stellars' tribute (rdm task planet-domination);
//! 5. the day's events, `öops`: today's `market::step_day`;
//! 6. the stellars: every whole one repaired to its `Strength`
//!    (story-runtime phase 8; see "Built by phase 8"), a destroyed one's
//!    regeneration and `OnRegen`, each `OnRegen` run as its stellar is
//!    reached (story-runtime phase 5; see "Built by phase 5"), and a
//!    dominated one's defence regrowing (planet-domination);
//! 7. each visible system's reinforcement countdown, down by one
//!    (combat-ai-fidelity phase 5; see `nova_sim::ai::reinforcements`);
//! 8. the ranks' salaries, each active rank's in ID order, the cash
//!    floored at 0 after each (crons-news-ranks phase 3; see
//!    [`rank`](crate::rank));
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
//!   [`GateArrivalRule::LikeJump`](crate::gate::GateArrivalRule::LikeJump);
//! - leaving the spaceport (`nova_sim::Session::take_off`),
//!   as `day::Stay`, the stay's tally, counts: half as many days as
//!   escorts sold or upgraded, rounded down; then a day if an outfit was
//!   bought or sold, four if a ship was bought, and every take-off's own
//!   day. A `Q` that ends the stay is held to that take-off and leaves
//!   the same way;
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
//!   destroyed or regenerated, and the system it belongs to: its owner as
//!   derived from visibility ([`OverlaidGalaxy::owner`]), never stored;
//! - per nebula: whether it is active (`nëbu` `ActiveOn`), and whether it
//!   is explored, never saved; the map draws a nebula only while it is
//!   active and explored ([`OverlaidGalaxy::nebula_shown`]), as
//!   `_MapDialogUpdate` does.
//!
//! A system shows the stellars it holds, not every stellar its `NavDefs`
//! list: those it owns, and, for the player's system, any it lists that
//! no system owns, the rescue `_HandleStellarSystemVisibility` writes
//! (@0x32c58-0x32cb3) and that lasts until the next refresh
//! ([`OverlaidGalaxy::holder`], [`OverlaidGalaxy::shown`]). The session
//! derives the stellars shown at the original's rebuild points, and the
//! flight view draws only those. A reader that turns a stellar into a
//! system goes owner-first, as the original's do: the holder, else
//! `_FindSystemFromStellar`, the lowest visible lister, shadowed or not,
//! else the lowest lister ([`OverlaidGalaxy::system_of`],
//! [`OverlaidGalaxy::find_system_from_stellar`]); the hypergate's links
//! read so, and a wormhole reads the holder alone (see the
//! [`gate`](crate::gate) module). The galaxy map's stellar list stays the
//! selected system's `NavDefs`, as `_MapDialogUpdate` lists them
//! (@0x10373-0x104fd).
//!
//! Readers see these facts only through the overlay: hyperspace links and
//! navigation, traffic, landing, legal records, AI targeting, exploration,
//! and every `StarMap::govt` call. The galaxy map keeps its static
//! geometry (positions, nebula rectangles) from nova-view's
//! `GalaxyCatalog`, and reads visibility, government, links and which
//! nebulae are shown (active and explored) through a session accessor
//! over the overlaid galaxy.
//!
//! An override written directly (`Y`, `U`, domination) applies at once.
//! Visibility and nebula activity are evaluated from the bits at the
//! original's refresh points: at a landing, as the offers are rolled and
//! after the land check; on a jump's arrival, before its days; and for a
//! new pilot and a load. Crons-news-ranks phase 5 adds the Rulebook key
//! `visibility_refresh` for the other reading, after every settled
//! script, and `colocated_resolution` for which visible system stands for
//! a place two share. A system's visibility is `Some(false)` hidden by the
//! last refresh, `Some(true)` a chain head the player's move showed (the
//! refresh never writes it, so it marks the move's reveal exactly, and
//! such a system owns no stellar), and `None` visible. Each change to the
//! overlay raises one `SessionEvent`, on which
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
//! `nova_sim::Session::change_galaxy` is the one
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
//! the take-off, as after a landed `M`; a pilot loaded last landed at a
//! stellar of a hidden system at its place resumes in flight over it,
//! among the visible system's stellars, and a relocation to a stellar of a
//! system resolved to another docks there as a landed `M` does. One
//! owner decides this for every move of the player's system, the session's
//! `set_player_system` (see the session's `entry` module): docked, the
//! stellars shown are always those of the system whose stellar the ship is
//! docked at. Between
//! rebuild points the course keeps its system IDs, each hop resolved by
//! place when it is used, as `_CueNextHyperRouteDest` resolves it; each
//! rebuild point rewrites it as
//! [`RuleKey::RouteRewrite`](crate::RuleKey::RouteRewrite) says (see the
//! `nova_sim::course` module).
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
//! Two of the original's readings were kept the engine's way here: that
//! a visible system stands for itself, where `_FindActiveCoLocatedSystem`
//! gives the lowest-ID visible system at a place even for a visible system
//! (@0x4c02-0x4c40), and that a link reaches every visible system at its
//! target's place. This phase took them to differ only where more than
//! one system at a place is visible at once, which it took stock data
//! never to have. Stock data does reach that: Aldebaran 141 (`!(b147|b305)
//! | !b148`) and 532 (`(b147|b305)|b148`) are both visible from Polaris
//! mission 183, which sets b305, until mission 887, which sets b148.
//! Crons-news-ranks phase 5's `colocated_resolution` now follows the
//! original by default: the lowest-ID visible system stands for the place
//! and links reach only it, so the shadowed 532 owns no stellar of its own
//! (430 and 433 have no owner), with phase 4's reading kept as the other
//! reading.
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
//! `nova_sim::Session::change_galaxy`, which takes
//! a catalog for its follow-ups (a hidden system, a government, the
//! spawn table), and `Chart::set_stellar_state`, catalog-free, which
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
//!   [`StellarLife`](crate::catalog::StellarLife), which the session reads from
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
//! - A new pilot (`nova_sim::Session::begin`) starts with
//!   every stellar whose `Flags2` has 0x0040 destroyed, as `Y` destroys
//!   it, before its `OnStart`. A load keeps the stellars as saved. No
//!   stock stellar starts destroyed, so a save from before version 18
//!   needs no seeding.
//!
//! **Its readers.** [`OverlaidGalaxy::site`] stamps a stellar's
//! [`LandingSite::destroyed`](crate::catalog::LandingSite::destroyed) from the
//! overlay, as it stamps the government, and
//! [`is_landable`](crate::landing::is_landable) reads it as the original's
//! `_StellarIsLandable` does: a stellar can be landed on when it has the
//! can-land flag and is destroyed exactly when it can be landed on only
//! once destroyed. Docking (gates and the developer tools' relocation),
//! F5, the land key, and the NPCs' choice of stellar and whether they keep
//! heading for it follow. A landed `M` docks at the first stellar,
//! destroyed or not, as the original does. A player docked at a stellar
//! `Y` destroys stays docked: the spaceport stays open, the stellar stays
//! among those shown, flagged destroyed, the take-off works and passes its
//! days, and only landing there again is refused until it regenerates. A
//! pilot saved docked there resumes in flight over it.
//!
//! The galaxy map's panel reads `landable` for its ports, goods and
//! services (`nova_sim::map_panel`), and its colour code `is_landable`;
//! it has no service or trade mode yet. Defence fleets,
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
//! The one writer of the destroyed state, `Chart::set_stellar_state`,
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
//! animates only while destroyed, any other only while not. Since
//! stellar-fidelity phase 1 each stellar steps its own frame tick by
//! tick, keeps the frame it shows across a swap while it is in range, and
//! animates over the frame count of the sheet its slot held at the last
//! rebuild point (see "Built by stellar-fidelity phase 1"); before, every
//! stellar's frame came from the view's one clock.
//!
//! No Rulebook key is added. The original's other readings are filed as
//! their own phases, each to add a key with the original's reading as
//! the default: story-runtime phase 9 (a `DeadType` with no sheet draws
//! `spïn` 1000 after a rebuild; built, see "Built by phase 9"), phase 10
//! (the original's landing distance and its 150 fallback; built, see
//! "Built by phase 10"), phase 11 (clicking a destroyed
//! stellar by its `DeadType` sheet) and stellar-fidelity phase 1 (the
//! frame count kept from the last rebuild; built, see "Built by
//! stellar-fidelity phase 1").
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
//!   over (+0x474) is copied from the slot only at a rebuild (@0x331c9),
//!   so the animation follows the live state but not the live sheet's
//!   frame count (see "Checked for stellar-fidelity phase 1").
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
//!   (combat's step 7b), so a shot that would meet a planet-type ship and
//!   a stellar the same tick hits the ship (it passes through an ordinary
//!   one, see "Built by stellar-fidelity phase 2"), and one at the end of
//!   its life hits nothing. Only past its `ProxSafety`: its age above it.
//!   Any ship's shot, the player's or an NPC's. Beams never hit a stellar
//!   by the engine's reading of
//!   [`beam_stellars`](crate::RuleKey::BeamStellars).
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
//!   through `Chart::set_stellar_state` with the countdown `Y` gives it
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
//! [`ScriptNote::OnDestroyReentered`](crate::control::ScriptNote::OnDestroyReentered). No key for
//! the ten killings, the daily repair, `Y`'s explosion on an invincible
//! stellar or shots passing through one, which the Bible's `Strength`
//! ("0 or -1 for an invincible stellar") agrees with, nor for the
//! countdown. The Bible's planet-type weapon "can only hit planet-type
//! ships or destroyable stellars"; its ship side, the blasts, the AI
//! firing at stellars and whether beams hit stellars are stellar-fidelity
//! phase 2's (see "Built by stellar-fidelity phase 2"), and `Flags2`
//! 0x0400 is no longer reported as unimplemented.
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
//! - **Planet-type shots and ships.** This bullet once said
//!   `_ShotCanHitShip` never tests `Flags2` 0x0400. That was wrong:
//!   its rule 8 (@0x44a8e-0x44acc) matches the bit against the ship
//!   class's `Flags` 0x0400, so a planet-type shot hits only planet-type
//!   ships (corrected by stellar-fidelity phase 2; see "Checked for
//!   stellar-fidelity phase 2"). `_HandleShipHit` (@0x36a45) and
//!   `_ShipCollisionHandler` (@0x36f87) do not test it. The bit's other
//!   readers are a detonation blasting no ships (`_HandleShot`
//!   @0x35fb9), an asteroid hit's blast sparing ships (`_HandleAsteroidHit`
//!   @0x36734) and the AI (@0x804a9, @0x8c1f2, @0x8f94d): stellar-fidelity
//!   phase 2's.
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
//! outside 0 to 255, no `spïn`, an `rlëD` the engine cannot read, or a
//! `PICT` pair the engine cannot read) draws
//! graphic type 0, `spïn` 1000, from the next rebuild point until a
//! flight frame finds it whole, as the original's rebuild means to fill
//! the sprite slot it left empty (its own code faults first; see
//! "Checked for phase 9", as corrected by stellar-fidelity phase 3);
//! before that rebuild point it draws its `Type`. A whole stellar whose
//! `Type` cannot be read does the same (see "Built by stellar-fidelity
//! phase 3").
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
//! - **The sheet rule** is the session's since stellar-fidelity phase 1
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
//!   (`_SWSetCurrentFrameIndex` @0x332d0; the frame index is
//!   stellar-fidelity phase 1's).
//!   Corrected by stellar-fidelity phase 3: the pass reaches only the
//!   slots no stellar uses this time (the rebuild's clear set their +0xd0
//!   to -1, @0x32e8f, and hid them, @0x32ea6). A slot in use whose
//!   stellar's graphic has no master faults first, at the unconditional
//!   `_SWSetCurrentFrameIndex` (@0x33096): a divide by zero in a newly
//!   made slot (@0xa90e7), a read through a null frame table in a reused
//!   one (@0xa90f7). So graphic type 0 for an empty slot of a stellar,
//!   whether its `Type` or its `DeadType` is the missing one, is the
//!   original's intent (the "had no data" log), not its behaviour; the
//!   engine follows the intent (see "Checked for stellar-fidelity phase
//!   3"). When `spïn` 1000 cannot be loaded either, the slot stays empty,
//!   drawing nothing; the engine draws its placeholder box and the reason.
//!   Corrected by stellar-fidelity phase 8: a visible slot with no frame
//!   faults at `_SWSetSpriteLocation`'s hot-spot read (@0xab665, from
//!   @0x331aa and @0x2ef1a), so the engine's empty slot is the code's
//!   continuation, not a state the original survives. The engine's
//!   behaviour there is unchanged.
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
//!   @0x1ece3), which the engine reads as the original cuts it since
//!   stellar-fidelity phase 4 (see "Checked for stellar-fidelity phase
//!   4"). `_SWCreateSpriteFromRLEResource` (@0xa9990) clears the master
//!   first (@0xa99e3) and writes it only on success (@0xa9be7), so a
//!   failure leaves the slot empty, and graphic type 0 after a rebuild;
//!   each failure is reported in a stop alert (`_ReportSpriteLoadError`
//!   @0x1eb62) and play goes on. It reads only the `rlëD`'s header (the
//!   frame size, the depth at +4, the frame count at +8) and makes each
//!   frame at the header's depth with no pixel buffer (`_SWCreateFrame`
//!   @0xad62c → @0xad6f2), decoding no token at load: an `rlëD` whose
//!   tokens are corrupt loads, and one whose header says 8 bits loads and
//!   draws garbled (see "Checked for stellar-fidelity phase 4"). Only a
//!   corrupt `rlëD` reads as no sheet in the engine, which reports it.
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
//!   @0xacb9-0xacc1; for a stellar another system owns, read by NPC gate
//!   entry, as the dock request refuses such a stellar before the reach:
//!   see "Checked for stellar-fidelity phase 7"); 150 when `Type` (+0x0e)
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
//! - **Such a `spïn` shown in a slot**: a clone copies the count into
//!   the slot (@0xac4ec) and sets its frame through
//!   `_SWSetCurrentFrameIndex`, which divides by it (`idivl` @0xa90e7),
//!   so a product of 0 divides by zero, and with a negative one it
//!   returns before setting the frame (@0xa90ee), leaving the slot's
//!   frame as it was: none after a rebuild, which empties it (@0x32e7a)
//!   without filling it with graphic type 0, as its frames are not null
//!   (@0x33241-0x33250). Built by stellar-fidelity phase 8, which found
//!   that the rebuild itself faults on such a slot (see "Checked for
//!   stellar-fidelity phase 8"); the engine follows the state its code
//!   leaves.
//! - **The dock request** (`_HandlePlayerDockRequest`): after the
//!   clearance state 0x3b544c above 0x2ed (@0x66c18), the ship's +0x4c at
//!   most 0 (@0x66c32) and each velocity axis at most 0.75 (the float at
//!   0xdd658, @0x66c3c-0x66c66; built by stellar-fidelity phase 6), it
//!   takes |trunc(dx)| and |trunc(dy)|
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
//!   the 64-pixel placeholder. Since stellar-fidelity phase 9 the caller
//!   gives the slot's rect (`navigation::SpriteRect`, `Space::click_rect`)
//!   and `None` is no rect.
//! - **The sheet drawn** is decided once, by `Session::drawn_sheet` (the
//!   session's `stellars` module; one owner since stellar-fidelity
//!   phase 1): the
//!   `DeadType` sheet while destroyed, when there is one
//!   (`LandingSite::dead_frame_size`); graphic type 0's while destroyed
//!   and dead since the last rebuild point, by
//!   [`dead_type_fallback`](crate::RuleKey::DeadTypeFallback)'s engine
//!   reading; the `Type` sheet otherwise. `Space::drawn_frame_size`
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
//!   the order among overlapping stellars (below) are stellar-fidelity
//!   phase 9, as is the stellar's +0x44 skip flag. Built there: the margin
//!   reads the width.
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
//!   engine compared the height until stellar-fidelity phase 9 built the
//!   width test (`SMALL_SPRITE_WIDTH`).
//! - **A rectangle, not a mask**: `_PtInRect` (@0x6a686), half-open on
//!   the right and bottom, where the engine includes every edge; and the
//!   first slot holding the point wins, the slots filled by ascending
//!   `spöb` index (@0x32eda-0x32f06), not `NavDef` order. Both are built
//!   by stellar-fidelity phase 9.
//!
//! ## Built by stellar-fidelity phase 1
//!
//! The sheet a stellar draws is decided once, in the session, and a
//! stellar animates over the frame count of the sheet its sprite slot
//! held at the last rebuild point, as the original's does.
//!
//! - **One owner for the sheet drawn.** `Session::drawn_sheet` gives a
//!   [`DrawnSheet`] (`Type`, `Dead` or `TypeZero`) for each stellar shown,
//!   by the rule of "Built by phase 9". The click box
//!   (`Space::drawn_frame_size`) is built on it, and the flight view's
//!   `SystemScene::set_destroyed` takes it, so the view no longer reads
//!   the rulebook or works out graphic type 0 itself.
//! - **The counted sheet** is the sim's, kept in `Rebuilt` beside the dead
//!   set (the `rebuild` submodule), in the original's terms: the sheet
//!   the slot held at the last rebuild point. A whole stellar counts its
//!   `Type`; a destroyed one its `DeadType` when that can be read; a
//!   destroyed one with none, graphic type 0's when it was dead since the
//!   previous rebuild point (its slot filled with graphic type 0 then) and
//!   graphic type 0's sheet reads, its `Type` otherwise; and every stellar
//!   its `Type` before any rebuild point. `Session::counted_sheet` reads
//!   it. A rebuild point that changes a counted sheet tells
//!   `SessionEvent::StellarsRebuilt` even with the dead set unchanged: a
//!   stellar with no `DeadType` still destroyed at a second rebuild point
//!   goes from `Type` to `TypeZero`.
//! - **The frame state** is the view's: each `SceneStellar` holds its
//!   current frame, next frame, timer and the frame shown
//!   (`system::animation::Frames` in nova-view), stepped once a tick of
//!   1/30 s, as the original's sprite tick steps the stellar every play
//!   frame. The timer gains a tick; at `AnimDelay`, or `AnimDelay` ×
//!   `Frame0Bias` while the current frame is 0 and the bias is above 1,
//!   the current frame takes the next, the next becomes the one after it
//!   modulo the counted sheet's frame count, and the sheet drawn shows the
//!   current frame modulo its own count, so an index past the end of a
//!   shorter sheet wraps. A stellar that does not animate shows frame 0
//!   with its timer and frames frozen. A swap of the sheet drawn keeps the
//!   frame shown while it is in range, and shows frame 0 otherwise. A
//!   rebuild point that tells `StellarsRebuilt` shows each stellar whose
//!   look changed at its current frame, or frame 0 on graphic type 0's
//!   sheet. A rebuild point that changes nothing tells nothing, so its
//!   reshow is skipped: at most one `AnimDelay` of difference.
//! - **Kept across systems** (since stellar-fidelity phase 12): the
//!   original keeps each `spöb`'s animation state across systems, and so
//!   does the flight, which resumes a stellar shown again where it was
//!   left; see "Built by stellar-fidelity phase 12". The original's stale
//!   slot count after a jump (another stellar's) and the 0 count of a
//!   fresh slot are memory-layout accidents the engine does not copy: a
//!   stellar newly shown counts its `Type` until a rebuild point says
//!   otherwise.
//! - **Since built**: `Flags2` 0x0001 and 0x0002 and an `AnimDelay` of 0
//!   or less, which the original advances every frame: see "Built by
//!   stellar-fidelity phase 10". A hypergate draws the frame its session
//!   steps it to instead (`Session::gate_frame`, system-traffic-fidelity
//!   phase 7).
//!
//! **Readings.** The Rulebook key
//! [`stellar_frame_count`](crate::RuleKey::StellarFrameCount) defaults to
//! the engine: the frame count of the sheet the slot held at the last
//! rebuild point, so between a `Y` (or `U`) and the next rebuild point a
//! 0x0080 stellar whose `Type` sheet has 1 frame stays still, 4 counted
//! frames over a 3-frame sheet show 0, 1, 2, 0, 0, 1, 2, 0, and fewer
//! counted frames than drawn leave the rest unshown. By the Bible's
//! 0x0080, "the stellar's graphic is animated after it's been destroyed",
//! the frame count of the sheet drawn, from the moment it is swapped.
//! Graphic type 0's count is read as `Type`'s under
//! [`dead_type_fallback`](crate::RuleKey::DeadTypeFallback)'s Bible
//! reading, which never draws graphic type 0. How an index past the end
//! shows needs no key: the original wraps it modulo the drawn sheet's
//! count, and no other source gives a reading.
//!
//! ## Checked for stellar-fidelity phase 1
//!
//! - **The state is the stellar's.** The `spöb` record (0x498 bytes,
//!   behind 0x3b5010) holds the frame count (+0x474), the current frame
//!   (+0x476), the next frame (+0x478), `AnimDelay` (+0x470),
//!   `Frame0Bias` (+0x472) and a float timer (+0x490). `_InitObjects`
//!   sets the count to 1, the current frame and the timer to 0 and the
//!   bias to 1 (@0x1c8b9-0x1c8f6); the `spöb` loader then copies
//!   `AnimDelay` and `Frame0Bias` raw (@0x7761b, @0x77631), clamping
//!   neither.
//! - **The count is written only at a rebuild.** The one write besides
//!   `_InitObjects` is `_HandleStellarSystemVisibility`'s @0x331c9, which
//!   copies the slot's frame count (+0x54) after cloning the master of
//!   the graphic the stellar shows (@0x32ff6). Every other access reads
//!   it (@0x2e802-0x2ede3, @0x67616). A slot the rebuild had to make
//!   (`_SWCreateEmptySprite` @0x32f80, flag -0x121) resets the timer, the
//!   current and the next frame to 0, 0 and 1 (@0x331da-0x331fb), and
//!   `_SWCreateEmptySprite` clears +0x54 (@0xa8eb6), so such a slot left
//!   empty counts 0 and its stellar stays still. The rebuild's clear
//!   (@0x32e68-0x32e83) leaves +0x54, so an empty slot keeps its count,
//!   and graphic type 0's fill (@0x33229-0x332d0) comes after @0x331c9,
//!   so it does not refresh it. Slots are filled in `spöb` index order
//!   (@0x32eda-0x32f06), so within a system a slot held the same
//!   stellar's frames before.
//! - **The sprite tick** (`_HandleStellarSprites`) clones the shown
//!   master into the slot every play frame (@0x2e7e5), then animates only
//!   when `Flags2` 0x0080 matches the live destroyed state
//!   (@0x2e784-0x2e7bb) and the count is above 1 (`cmpw $1, 0x474`
//!   @0x2e802); otherwise the slot shows frame 0 (@0x2ee20) and the
//!   stellar's state is untouched. An animating stellar's timer gains
//!   `_gSpeedMult` (the double behind 0x3b511c, at 0x188b48,
//!   @0x2e810-0x2e825), which `_HandleTimeAdjustment` keeps at the
//!   milliseconds since the last frame × 0.03, smoothed as (3 × old +
//!   new) / 4 (@0x333b7-0x33424; 1.0 at the start, @0x513ec): about one
//!   1/30 s tick a frame, so the engine's one step a tick is the
//!   original's. A stellar that is not a hypergate (`Flags2` 0x1000
//!   clear, @0x2e832) compares the timer with `AnimDelay` × `Frame0Bias`
//!   while the current frame is 0 and the bias is above 1, `AnimDelay`
//!   otherwise (@0x2ec4b-0x2ec83). Below it, the tick skips
//!   `_SWSetCurrentFrameIndex` (@0x2ee35) and the slot keeps the index the
//!   clone left. At or above it, the timer is reset, the current frame
//!   takes the next (@0x2ed7f) and the next becomes (next + 1) mod +0x474
//!   (@0x2eddc-0x2edf4), then `_SWSetCurrentFrameIndex(slot, current)`
//!   (@0x2ee05-0x2ee30). A delay of 0 or less advances every frame: see
//!   "Checked for stellar-fidelity phase 10", which reads the whole
//!   sequential branch.
//! - **The index wraps.** `_SWSetCurrentFrameIndex` reads a negative index
//!   as 0 and takes it modulo the slot's own count (@0xa90d4-0xa90e9).
//!   The clone keeps the slot's index while it is below the new count and
//!   sets 0 otherwise (@0xac504-0xac532). The rebuild sets each slot to
//!   the current frame (@0x33096), and a slot it fills with graphic type
//!   0 to frame 0 (@0x332d0).
//!
//! ## Built by stellar-fidelity phase 2
//!
//! The ship side of planet-type weapons (`nova_sim::combat::planet_type`),
//! beams on stellars by the Bible, and the stellar-attacking AI
//! (`nova_sim::ai::death_star`), by the findings of "Checked for
//! stellar-fidelity phase 2".
//!
//! - **Shots** (rule 8). A shot hits a ship only when its weapon's
//!   `Flags2` 0x0400 matches the ship class's `Flags` 0x0400: in the
//!   contact passes, the proximity fuse, a seeking sub-munition's pick and
//!   point defence's shots at ships, a shot no ship fired included
//!   (`nova_sim::combat::sides`). So a planet-type shot passes through
//!   ordinary ships to the stellars behind them.
//! - **Blasts.** By the engine's reading of
//!   [`planet_blasts`](crate::RuleKey::PlanetBlasts), an expiry's and an
//!   asteroid hit's blast from a planet-type weapon reach no ship, neither
//!   reaches a planet-type ship, and a shot's on-hit blast reaches every
//!   ship. By the Bible's every blast reaches only the ships of its
//!   weapon's planet type.
//! - **Beams.** By the engine's reading of
//!   [`beam_planet_flag`](crate::RuleKey::BeamPlanetFlag) a beam is
//!   planet-type by its weapon's `Flags` 0x0400, by the Bible's by its
//!   `Flags2` 0x0400, and hits only ships of its planet type. By the
//!   engine's reading of [`beam_stellars`](crate::RuleKey::BeamStellars) a
//!   beam never hits a stellar; by the Bible's a planet-type beam is
//!   stopped by a stellar nearer along it than the ship it hits, takes its
//!   `MassDmg` + `EnergyDmg` times the pass's beam factor off it each
//!   tick, explodes there, and shoots it down below 0 as a shot does, the
//!   session running its `OnDestroy` after the beams' hits settle.
//! - **Sub-munitions** on their release tick are not tested against the
//!   stellars: one a fuse releases strikes on the next tick, one an end of
//!   life seats in a slot `_HandleShot` has passed a tick later still.
//! - **The stellar attacker.** `nova_sim::ai::Goal::AttackStellar` is
//!   goal 0x12: spared as a threat, busy to a hail, fighting nothing.
//!   `_DeathStarAI` (`DeathStar`) takes the first enemy stellar shots can
//!   shoot down and, with a planet gun of its class's stock, attacks it;
//!   otherwise the warship's think runs, a kept landing or gate entry
//!   going to the enemy stellar. It flies by its band from the stellar
//!   (`nova_sim::traffic::stellar_attack`: sub 2 by
//!   `nova_sim::traffic::approach_flight`, the run out from the centre,
//!   the brake, or the aim, the in-band 0.98 drag on its velocity and its
//!   speed along its heading) and, aiming within its turn and 15 degrees,
//!   fires `_AIFirePlanetGun`'s pick, the highest ready weapon, planet-type
//!   or not.
//! - **Routing.** A ship of a `ShipBehav` 2 mission, by its mark alone,
//!   thinks as `DeathStar` before the miners and every type, the escorts'
//!   included, so a captured one kept as a mission escort attacks too; its
//!   gate wait and reactions stay its own behaviour's.
//! - **Sounds.** None of these plays one, and no `SimSound` is added: a
//!   planet gun's shot sounds as any NPC's fire does.
//! - **Recorded, not built**: the readers of +0x70 as a ship slot in goal
//!   0x12 and a sub-munition of negative `ProxSafety` (see "Checked for
//!   stellar-fidelity phase 2").
//!
//! ## Checked for stellar-fidelity phase 2
//!
//! Planet-type weapons (`wëap` `Flags2` 0x0400) and planet-type ships
//! (`shïp` `Flags` 0x0400, the class's +0x9e8), and the AI that shoots
//! stellars.
//!
//! - **Who a shot may hit.** `_ShotCanHitShip`'s rule 8
//!   (@0x44a8e-0x44acc) returns false when `(class Flags & 0x0400) !=
//!   (wëap Flags2 (+0x22) & 0x0400)`: a planet-type shot hits only
//!   planet-type ships, and an ordinary shot never hits a planet-type
//!   ship, the player's included. A shot no ship fired (+0x2c above 63,
//!   @0x44a71-0x44a88) jumps to rule 8 too, and rules 9 (@0x44ad2-0x44b2f)
//!   and 10 (@0x44b35-0x44b60) then pass for it (firer -1 is above 0x3e
//!   and 0x3f). Rule 8 covers every caller: the contact passes, the
//!   proximity fuse, `_FindNearestValidShotTarget`'s seeking sub-munitions
//!   and point defence's shots at ships.
//! - **Who a beam may hit.** `_HandleBeams`' candidate filter
//!   (@0x30d34-0x30d82) reads the beam's weapon at +0x20, which
//!   `_LoadObjectData` fills from the resource's +0x1c (@0x793ca-0x793ce):
//!   `Flags`, not `Flags2` (+0x22 from +0x48, @0x793d9-0x793dd). So a beam
//!   matches its `Flags` 0x0400 (the "big smoke" bit) against the ship's
//!   `Flags` 0x0400 (@0x30d6c), and `Flags2` 0x0400 does nothing for it.
//!   No stock beam has `Flags` 0x0400 (the 16 stock beams' are
//!   0x10-0x6010), no stock `wëap` has `Flags2` 0x0400 and no stock `shïp`
//!   has `Flags` 0x0400, so the bug changes nothing in stock play.
//! - **Beams never touch stellars.** `_HandleBeams` (@0x30295-0x31e1f)
//!   reads neither the stellar table (0x3b5010) nor `_StellarIsDestroyed`,
//!   nor any sprite-slot array. Its calls are `_Accel`,
//!   `_XYFloatSquaredDist`, `_CalcInterceptAngle`, `_FloatAbs`,
//!   `_SpeedLimit`, `_AlliedWithPlayer`, `_AdjustedAccel`,
//!   `_SpawnParticles`, `_ShipMaxSpeed`, `_GetSpriteYSize`,
//!   `_GetMasterShipSpriteXSize`, `_CreateExplosion`, `_SWSetSpriteVisible`,
//!   `_SpawnExplod`, `_Rand`, `_IonizeShip`, `_DestroyAsteroid`,
//!   `_DamageShip`, `_AIShipIsAttackingAsteroid` and `_TickCount`, none of
//!   which plays a sound.
//! - **The expiry blast** (`_HandleShot` @0x35f97-0x36106).
//!   `_CreateExplosion` (@0x35f97) comes first, for every weapon. The
//!   blast pass is skipped when `BlastRadius` (+0x18) is 0 or less
//!   (@0x35fae) or the weapon is planet-type (@0x35fb9), and inside it each
//!   ship whose class is planet-type is skipped (@0x3602e). So a
//!   planet-type detonation blasts no ship, and no detonation blasts a
//!   planet-type ship. The ionization is inside the per-ship pass, so a
//!   skipped ship is not ionized.
//! - **The on-hit blast** (`_HandleShipHit` @0x36d4b-0x36efb) tests
//!   neither bit: a planet-type shot hitting a planet-type ship blasts the
//!   ordinary ships around it, and an ordinary shot's on-hit blast reaches
//!   a planet-type ship near its victim.
//! - **The asteroid-hit blast** (`_HandleAsteroidHit` @0x36734, @0x367a8)
//!   makes the expiry blast's two skips.
//! - **Every reader of the class's `Flags` 0x0400** (every `0x9e8(` read
//!   and every `testb $0x4, 0x9e9`): `_ShipAccelRate` @0x3601 and
//!   `_ShipMaxSpeed` @0x37b5 (still hull), `_AIHasAfterburner` @0x654c,
//!   `_HandleShipDisplay` @0x2d90d (no shockwave), `_HandleBeams` @0x31210,
//!   @0x31385, @0x31b12, @0x31bc7 (tractor), `_DamageShip` @0x3aacb
//!   (push), the expiry blast @0x3602e, the asteroid blast @0x367a8, the
//!   beam match @0x30d6c, the shot match @0x44aa8, and `_AIFireGun`
//!   @0x7ffc6-0x7ffdf, `_AIFireTurret` @0x80c04-0x80c23 and
//!   `_AIFireMissile` @0x812df-0x812fb.
//! - **Every reader of `Flags2` 0x0400**: `_HandleShot` @0x35fb9,
//!   `_HandleAsteroidHit` @0x36734, `_CheckShotProximities` @0x37224,
//!   `_AIFireGunUntargeted` @0x804a9 (the miner's untargeted gun skips
//!   planet-type weapons; asteroids phase 5's), `_DeathStarAI` @0x8c1f2 and
//!   `_HighLevelAIHandler` @0x8f94d.
//! - **`_DeathStarAI`** (@0x8c125-0x8c2d1).
//!   1. If `_IsDisabled`, `_IsDying` or goal 0x16 (count wait), it returns
//!      at once, writing nothing and calling no `_WarshipAI`
//!      (@0x8c12e-0x8c159).
//!   2. It walks the 16 stellar slots of the ship's system (sÿst +0x2a +
//!      2i, stride 0x1fc, from ship +0x74) in order, skipping -1, the
//!      destroyed (`_StellarIsDestroyed` @0x8c19e) and `Strength` (+0x40)
//!      0 or less (@0x8c1b1-0x8c1b6), and takes the first for which
//!      `_GovtEnemies(ship +0x98, stellar Govt +0x0a)` holds
//!      (@0x8c15f-0x8c1de). It writes +0x6c = that stellar at once
//!      (@0x8c286), before any weapon test.
//!   3. Whether or not a stellar was found, it walks weapon indices 0-255
//!      and takes the lowest with `Flags2` 0x0400 (@0x8c1f2), the
//!      **class's** stock count above 0 (class `+0x5e8[i]`, @0x8c20e), ammo
//!      (`_WeaponHasAmmo` @0x8c21f) and guidance not 0, not 3 and at most
//!      7 signed (-1, 1, 2, 4, 5, 6, 7; @0x8c230-0x8c244).
//!   4. With both: goal (+0xc8c8) 0x12 and +0x6c = +0x70 = the stellar
//!      (@0x8c25a-0x8c280), and it returns without `_WarshipAI`.
//!   5. Otherwise, when the goal is 0x12, it clears +0x70 and +0x6c to -1
//!      and the goal and sub (+0xc8ca) to 0; in every case it then
//!      tail-calls `_WarshipAI` (@0x8c293-0x8c2c5), never the ship's own
//!      type. `_WarshipAI` writes +0x6c but never reads it (@0x8b8ed,
//!      @0x8b90b, @0x8ba31, @0x8ba4f, @0x8c00c, @0x8c02a, @0x8c117), and
//!      the high level reads it for goal 1, landing (@0x8d8c9, @0x8daea;
//!      the gate hand-off @0x8d8b8-0x8d8fe), and goal 0x14, gate entry
//!      (@0x8db70). So a landing or gate entry the warship keeps is
//!      redirected to the enemy stellar.
//!   6. No sound and no `_Rand`.
//! - **Routing.** `_AIDispatch` sets a local flag (edi) on every think
//!   from the mark (+0xc8d2) when its mission slot is active and its
//!   `ShipBehav` (+0xc) is 2 (@0x8fbcd-0x8fc38), writing nothing on the
//!   ship. With it, `_DeathStarAI` runs in place of the whole type
//!   dispatch (@0x8ff3a-0x8ff43): `_DefenseShipAI`, `_MinerAI` and every AI
//!   type, `_EscortAI` included (@0x90016), after the disabled reset
//!   (@0x8fece-0x8ff13) and the goal-0x15 skip (@0x8ff18). No AI type
//!   reaches `_DeathStarAI`. Goal 0x12 is in the always-think set
//!   (@0x8fe30), which changes nothing at a think interval of 1.
//! - **The high level's goal 0x12** (`_HighLevelAIHandler`
//!   @0x8f906-0x8faeb) aims at +0x6c's spöb x and y (+0x4, +0x6). The
//!   weapon is `_DeathStarAI`'s scan (lowest index, class count, ammo,
//!   guidance, no reload test), and r its range (+0x5c); with none, r² is
//!   1.0 (0xdd068). With d² the squared distance: above 0.5625 r² (0xddaa0)
//!   sub 2, approach; else below 0.0625 r² (0xddaa8) sub 3, run out; else
//!   sub 1, brake, when |vx| or |vy| is at least 0.35 (0xdda68), and sub
//!   0x16, aim, otherwise. In those last two, vx, vy and the speed along
//!   the heading (+0x48) are each multiplied by 0.98 (0xdd988,
//!   @0x8faaf-0x8faeb).
//! - **The low level** zeroes +0x30 and the fire flag +0xba in its
//!   preamble (@0x851e6-0x851f6). Sub 1 (@0x8536d-0x85633) is the brake.
//!   Sub 2 (@0x8563b-0x857bd), gated on +0x6c != -1 and not disabled,
//!   turns to `_CalcInterceptAngle(ship, stellar)` and, within turn + 5.0
//!   (0xdd67c) strictly, thrusts at its full `_ShipAccelRate`: capped at
//!   +0x34 = `_ShipMaxSpeed` × 0.25 (0xdd684) within 500 on both axes
//!   (0xdd080, strictly), uncapped (+0x34 = 0) beyond; it writes +0x34
//!   with +0x30. Sub 3 (@0x857bd-0x8585b) runs out from the system centre
//!   (0,0), not the stellar, within turn + 3.0. Sub 0x16
//!   (@0x88645-0x886f4) turns to the intercept angle and, within turn +
//!   15.0 (0xdda30) strictly, calls `_AIFirePlanetGun`; it does not
//!   thrust and is not gated on disabled.
//! - **`_AIFirePlanetGun`** (@0x80536-0x805c9) walks 0-255 and keeps the
//!   **highest** index with the ship's **own** count (+0xc8 + i·0xc8)
//!   above 0, guidance -1, 1, 2, 4, 5, 6 or 7, ammo and reload (+0xf8 +
//!   i·0xc8) 0 or less, then sets +0x72 to it and +0xba to 1. It does not
//!   test `Flags2` 0x0400, so a ready ordinary gun above the planet gun
//!   fires instead, and its shots pass through ordinary ships. No range
//!   or damage ranking.
//! - **Goal 0x12 to other readers.** It is spared as a threat with goals
//!   7, 9, 0xf, 0xa, 0xb, 5 and 0xc (`_IsThreatToPlayer` @0x7f567,
//!   `_ExtendedIsThreatToShip` @0x81aff, @0x81b49,
//!   `_ExtendedIsThreatToPlayer` @0x81c24, @0x81c86, @0x81cce, @0x81d69,
//!   @0x81da8, `_IsThreatToShip` @0x82055), and `_AIIsShipBusy`
//!   (@0x82fac-0x82fe4) finds it busy, as every goal but 0, 1, 2, 7 and
//!   0x14.
//! - **Sub-munitions on their release tick.** `_CheckShotProximities`
//!   tests a shot against stellars only when `Count - ProxSafety > life`
//!   strictly (@0x371fc-0x3721e), and a fresh shot's life is `Count`
//!   (`_SpawnShot` @0x3e801-0x3e808). So a sub-munition of `ProxSafety` 0
//!   or more is never tested on the frame it is released, whether a fuse
//!   in `_CheckShotProximities`' loop or an end of life in the later
//!   `_HandleShot` released it.
//! - **Sounds.** None of `_DeathStarAI`, `_AIFirePlanetGun`, the goal-0x12
//!   block, subs 2 and 0x16, `_ShotCanHitShip`, the expiry blast's skip or
//!   `_HandleBeams`' filter plays one. The planet gun's shot sounds as any
//!   NPC fire does, through `_FireAIShipWeapon`.
//! - **Recorded, not built** (memory-layout accidents). In goal 0x12,
//!   +0x70 holds a stellar index (@0x8c278), and readers that take it for
//!   a ship slot are not copied: `_HighLevelAIHandler`'s target check reads
//!   the active byte of the ship slot numbered by the stellar index, past
//!   the 64-slot table for an index of 64 or more, and may clear it
//!   (@0x8d4e4-0x8d50d); `_FireAIShipWeapon` aims a turret (4) or homing
//!   (1) pick at the ship slot numbered by the stellar index; and stellar
//!   index 0 reads as "targeting the player"
//!   wherever +0x70 == 0 is tested without the goal. A sub-munition of
//!   negative `ProxSafety` released by a fuse into a later slot would be
//!   tested against stellars in the same loop by a stale sprite rectangle
//!   (`_SpawnShot` places no sprite; `_SWRLECollision` reads the slot's
//!   rectangle, +0x1a, +0x1c, @0xabfbd-0xac049); no stock weapon has one.
//!
//! ## Built by stellar-fidelity phase 3
//!
//! A whole stellar whose `Type` has no sheet that can be read (no `spïn`,
//! a sheet the engine cannot read, or a `Type` outside 0 to 255; the
//! catalog's `LandingSite::frame_size` is `None`) draws, is clicked by
//! and is shot by what its sprite slot holds, from the first rebuild point
//! that shows it: graphic type 0, as the rebuild means an empty slot to
//! get, or a `DeadType` sheet a flight frame cloned in since.
//!
//! - **The state** is `Rebuilt::held` (the `rebuild` submodule): the sheet
//!   whose frames each shown stellar's slot holds. A rebuild point sets it
//!   to the clone's sheet when the stellar shows a graphic that can be
//!   read, to graphic type 0 when it shows none and `spïn` 1000 reads, and
//!   leaves it when neither reads (the slot shows no frame, its count
//!   +0x54 kept). A flight frame sets it only for a stellar showing a
//!   graphic that can be read. The counted sheet at a rebuild point is the
//!   clone's, or what the slot held before (stellar-fidelity phase 1's
//!   rule, generalised).
//! - **The sheet rule** (`Session::drawn_sheet`, `Space::site_sheet`): a
//!   destroyed stellar with a `DeadType` sheet draws it, a whole one with a
//!   `Type` sheet draws that; any other draws, by the engine's reading of
//!   [`dead_type_fallback`](crate::RuleKey::DeadTypeFallback) while
//!   destroyed or [`type_sheet_fallback`](crate::RuleKey::TypeSheetFallback)
//!   while whole, graphic type 0 when its slot shows no frame (the view's
//!   placeholder with `spïn` 1000's reason) and what its slot holds
//!   otherwise; its `Type` by the other reading. For a destroyed stellar
//!   with a readable `Type`, "holds graphic type 0, or shows no frame" is
//!   exactly "dead since the last rebuild point", so phase 9's rule is
//!   unchanged.
//! - **The click box, the landing reach and the shots** read the same
//!   answer: the click box and the shots' disc are the drawn sheet's frame
//!   (`Space::drawn_frame_size`; `Session::stellar_targets`), and the
//!   landing reach reads 150 for the `Type` master that is never loaded
//!   (F3, already phase 10's model).
//! - **The counted sheet** (`Session::counted_sheet`) reads a counted
//!   graphic type 0 as `Type` only where the rules never let the stellar
//!   draw graphic type 0.
//! - **Problems.** The scene lists a `Type` sheet that cannot be read
//!   (`spöb <id> Type: <reason>`), so the reason is reported though the
//!   engine's reading draws graphic type 0 over the placeholder, and
//!   `spïn` 1000 when it cannot be read and some stellar has no `DeadType`
//!   sheet or no `Type` sheet, whatever the readings.
//!
//! **Readings.** The Rulebook key
//! [`type_sheet_fallback`](crate::RuleKey::TypeSheetFallback) defaults to
//! the engine: what the slot holds. The other reading, the intended
//! alternative since the Bible says nothing of an unreadable sheet, keeps
//! the placeholder box and the reason, which shows the broken data. No
//! sound: none of these routines calls `PlaySound` or `_beepSnd`.
//!
//! ## Checked for stellar-fidelity phase 3
//!
//! - **F1. The rebuild faults before its fill for a slot in use.**
//!   `_HandleStellarSystemVisibility` (@0x32aa5), for each stellar of the
//!   player's system, picks `DeadType` (+0x10) when it is destroyed and
//!   `Type` (+0x0e) otherwise (@0x32f0f-0x32f2f), loads that master if it
//!   is missing (@0x32f37-0x32f46), makes the stellar's slot if there is
//!   none (`_SWCreateEmptySprite` @0x32f80, which nulls the frames +0x4c
//!   and the count +0x54, @0xa8e0f, @0xa8eb6), skips the clone when the
//!   master is null (@0x32fe0-0x32fea), then calls
//!   `_SWSetCurrentFrameIndex(slot, current frame)` with no condition
//!   (@0x33096). That divides the index by the slot's count (`idivl`
//!   @0xa90e7) and reads the frame table through +0x4c (@0xa90f4-0xa90f7).
//!   Each clone sets +0xda (@0xac4c7), and the rebuild's clear nulls +0x4c,
//!   +0x50 and +0xd4 of every slot with +0xda set while keeping +0x54
//!   (@0x32e68-0x32e83). So a stellar whose shown graphic has no master
//!   divides by zero in a newly made slot (#DE @0xa90e7), and in a reused
//!   one reads through a null +0x4c below 4 × count, inside the 4 KB
//!   `__PAGEZERO` (@0xa90f7). The fill pass (@0x33231-0x332d0) tests +0x4c
//!   and +0xd4 (@0x33241-0x33250) but fills only the 16 slots no stellar
//!   uses this time: the clear set their +0xd0 to -1 (@0x32e8f), so the
//!   `ja` at @0x33260 skips the "had no data" log, and hid them
//!   (@0x32ea6). The log for a stellar's index (+0xd0 ≤ 0x7ff,
//!   @0x33256-0x3329c) shows the authors meant an empty slot of a stellar
//!   to get `spïn` 1000. The fault is the original's bug (the game ends at
//!   the rebuild point that shows such a stellar); the engine defaults to
//!   the intent, as phase 9 does for `DeadType`. Stock never reaches it:
//!   every stock `Type` sheet reads, no stock script uses `Y`, no stock
//!   `spöb` has `Flags2` 0x0040, and no stock weapon is planet-type.
//! - **F2. The per-frame sprite tick** (`_HandleStellarSprites`, as it
//!   would run if the rebuild survived; it also runs mid-system) loads the
//!   null `Type` master again each frame (@0x2e7c7-0x2e7d1). With no
//!   `spïn`, `_GetSpriteInfo` fails and `_LoadMasterStellarSprite` returns
//!   silently (@0x1ebb9-0x1ebc0); with an `rlëD` that
//!   `_SWCreateSpriteFromRLEResource` fails on, it raises
//!   `_ReportSpriteLoadError` (@0x1ec71), a stop alert through the import
//!   at 0x3b6257 (@0x1eb62) whenever the error code is non-zero
//!   (@0x1ea8a). The clone then gets a null master and returns -1 without
//!   touching the slot (@0xac448-0xac462). So the slot keeps its frames:
//!   graphic type 0's from the rebuild's intent, or a `DeadType`'s that a
//!   frame finding the stellar destroyed cloned in (@0x2e79b, @0x2e7e5),
//!   kept after a `U` until the next rebuild point. Animation is
//!   unchanged (stellar-fidelity phase 1's +0x474 rule).
//! - **F3. Landing radius.** `_GetStellarSpriteXSize` (@0xac91) gives 150
//!   for a null `Type` master (@0xacdf, returned @0xad2d). The dock
//!   request lands within trunc(150 × 1.75) = 262 when the slot shows a
//!   frame, and within 75 when it does not (@0x66cc7-0x66cfd). `Rebuilt`
//!   already models this (a `Type` with no sheet is never loaded, so its
//!   width is `Unloaded`), and this phase pins it with tests.
//! - **F4. Clicking.** `_HandlePlayer` (@0x6a5f9-0x6a686) uses the slot's
//!   rect (+0x1a), so such a stellar is clicked by the frame its slot
//!   holds.
//! - **F5. Shots.** `_CheckShotProximities` collides a planet-type shot
//!   with the stellar's slot sprite (stellar +0x00, `_SWRLECollision`
//!   @0x37288-0x372a8) after skipping the destroyed (@0x3727b), so a whole
//!   stellar with no `Type` sheet is hit by the frame its slot holds.
//! - **Sounds.** None of these routines calls `PlaySound` or `_beepSnd`;
//!   the stop alert's sound belongs to the alert, left out below.
//! - **Left out.**
//!   - The fault at the rebuild (@0x33096 → `_SWSetCurrentFrameIndex`
//!     @0xa90e7 in a newly made slot, @0xa90f7 in a reused one): the
//!     original's bug, which ends the game. Not copied, and no key, as a
//!     crash is not a reading; the engine follows the fill pass's intent
//!     (@0x33241-0x332d0), as stellar-fidelity phase 8 does for its divide
//!     by zero.
//!   - The stop alert for an unreadable `rlëD` (`_ReportSpriteLoadError`
//!     @0x1ea6c, alert through 0x3b6257 @0x1eb62), raised at the rebuild's
//!     load (@0x32f46 → @0x1ec71) and, if the slot survived, every frame
//!     (@0x2e7d1). For a `Type` sheet the original faults straight after
//!     the rebuild's alert, so the per-frame alert is unreachable. The
//!     engine reports the broken sheet once in the scene's problems.
//!   - The crash log lines ("had no data" @0x33256-0x3329c; @0x332de and
//!     @0x33304 in the check after the fill): diagnostics with no effect on
//!     play.
//!   - NPC landing's width for such a stellar (150, read as a quarter by
//!     `_HighLevelAIHandler` @0x8dbdf-0x8dbfa): stellar-fidelity phase 5.
//!   - The click rectangle's shape (width margin, half-open edges, first
//!     slot wins; `_HandlePlayer` @0x6a649-0x6a686): stellar-fidelity
//!     phase 9.
//!   - A `Type` outside 0 to 255 indexing neighbouring statics
//!     (`_masterStellarSprite` @0x2e7c7): a memory-layout accident; the
//!     engine reads it as no sheet, so it draws graphic type 0.
//! - **Left out, depending on unlanded work.**
//!   - `Type` sheets the original loads that the engine could not read: a
//!     `spïn` with no `rlëD` (the `PICT` pair,
//!     `_SWCreateSpriteFromSinglePictXY` @0x1ece3) and an 8-bit `rlëD`.
//!     Read since stellar-fidelity phase 4: such a `Type` draws its own
//!     sheet.
//!   - A `spïn` whose tiles multiply to 0 or less: built by
//!     stellar-fidelity phase 8. Such a `Type` reads, so it is outside
//!     this phase's rule.
//!
//! ## Built by stellar-fidelity phase 4
//!
//! A `spïn` with no `rlëD` reads from its `PICT` pair, and an `rlëD` whose
//! header says 8 bits reads, both as the original loads and draws them.
//! Stellars, shots and explosions share the lookup
//! (`nova_data::GameData::spin_sheet`), so all of them draw such sheets;
//! a stellar whose `Type` or `DeadType` is one now draws it, not graphic
//! type 0 or its `Type`.
//!
//! - **The `PICT` pair** (`nova_data::graphics::cut_pict_sheet`): the
//!   sprites `PICT` is cut into `xSize` x `ySize` frames, as many columns
//!   as its width holds, row by row, each colour kept to its top five bits;
//!   the mask shows the sprite only where it is white at five bits, or,
//!   when `MasksID` is `SpritesID`, the sprite hides its own white. The
//!   sheet's kind (`nova_data::SheetKind::PictPair`) has the views key its
//!   frames by the `spïn` (`nova_view::ImageKind::SpinSheet`), since there
//!   is no `rlëD` ID to key them by.
//! - **The 8-bit `rlëD`** (`nova_data::graphics::decode_rled_as`): by the
//!   engine's reading, its bytes as they fall into the 16-bit world; by the
//!   other, each byte a colour of the Mac's standard 256-colour table.
//! - **16-bit runs** now fill as `_BlitPixieMemSet` does: the run word's
//!   low half on the run's even pixels, its high half on the odd ones,
//!   where the engine took the high half everywhere. Stock runs are all
//!   `0x7FFF7FFF`, so stock draws the same.
//! - **Problems.** A `spïn` with neither an `rlëD` nor a `PICT`, a `PICT`
//!   pair with no mask, and a pair that does not decode or cut are each
//!   reported with their reason, and the fallbacks of phases 9 and 3 apply
//!   as to any sheet that cannot be read.
//!
//! **Readings.** The Rulebook key
//! [`eight_bit_sheet`](crate::RuleKey::EightBitSheet) defaults to the
//! engine: an 8-bit sheet drawn garbled, as the original draws it. The
//! other reading, the intended alternative since the Bible says only
//! "rleD/rle8 resources", draws each byte through the standard palette.
//! The `PICT` cut gets no key: it is a well-defined cut, not garbled, and
//! the Bible gives no rule for it. The 16-bit runs get no key either: the
//! high-half reading was a stand-in taken from an editor, not a reading of
//! the Bible or a choice the engine made for a reason, and stock cannot
//! tell the two apart. No sound: none of these routines calls `PlaySound`
//! or `_beepSnd`; the stop alerts and `_SWAssertFail`'s `SysBeep` belong to
//! the quit paths left out.
//!
//! ## Checked for stellar-fidelity phase 4
//!
//! - **The world is 16-bit.** `_main` clears `_useLegacyRenderingPipeline`
//!   (0x30faa0, through 0x3b5030) at @0xccab8-0xccabe, setting it only when
//!   `_meetsMinimumRequirements` fails (@0xccac1-0xccaca). With it clear,
//!   `_SWCreateSpriteWorld` makes the world 16-bit (@0xb6be2), as
//!   `_SWCreateFrame` and `_SWCreateGWorldFromPict` make their `GWorld`s
//!   (@0xad68d-0xad6a9, @0xa7e81-0xa7e9d). So `_LoadMasterStellarSprite`'s
//!   depth test (@0x1ebd7-0x1ebe8) always picks `rlëD`, never `rlë8`, as
//!   `_LoadSprites` does for every other `spïn` (@0x205bf-0x205d5).
//! - **F1. Loading a `spïn`** (`_LoadMasterStellarSprite` @0x1eb72;
//!   `_LoadSprites` repeats it, e.g. @0x20659-0x20752). `_GetSpriteInfo`
//!   (@0x1bf80) reads `SpritesID`, `MasksID`, `xSize`, `ySize`, `xTiles`
//!   and `yTiles` (+0 to +0xa). If `ResourceExists('rlëD', SpritesID)`
//!   (@0x1ebee), the sheet comes from the `rlëD`
//!   (`_SWCreateSpriteFromRLEResource` @0x1ec30); an `rlëD` that exists
//!   but fails is not followed by the `PICT`. Otherwise the game calls
//!   `_SWCreateSpriteFromSinglePictXY(world, &master, NULL, SpritesID,
//!   MasksID, xSize, ySize, border 0, 0, maxFrames 0, maskType 1)`
//!   (@0x1ece3), then `_SWLockSprite` (@0x1ed2f) and `_ConvertSpriteToRLE`
//!   (@0x1ed3a). On either path the master's count (+0x54) is then
//!   overwritten with `xTiles` × `yTiles` (@0x1ed8c-0x1ed97), phases 1 and
//!   8's count, unchanged here. `xTiles` and `yTiles` play no part in
//!   cutting a `PICT`.
//! - **F2. Cutting the `PICT` pair** (`_SWCreateSpriteFromSinglePictXY`
//!   @0xa9bee).
//!   - Each `PICT` is drawn into its own `GWorld` (`_SWCreateGWorldFromPict`
//!     @0xa7dc7) of its `picFrame` moved to (0, 0) (@0xa7e16-0xa7e34),
//!     erased to white first (`EraseRect` @0xa7ee5), then drawn
//!     (`_DrawPicHandle` @0xa7ef4, through `AmbrosiaTools`' `DT_ImageFileLoad`
//!     and `DT_DrawImage`, which ends in `CopyBits`, `AmbrosiaTools`
//!     @0x4780c, or `DrawPicture` @0x8637 if that load fails). Pixels the
//!     picture does not cover stay white, and colours land as RGB555, each
//!     channel's top five bits.
//!   - The mask `PICT` (`maskType` 1) gets a `GWorld` the same way
//!     (@0xa9e00-0xa9e15), which is inverted (`_SWInvertGWorld` @0xa9c9c →
//!     `InvertRect` @0xa6ff8).
//!   - With maxFrames 0 the frame count is (width / `xSize`) × (height /
//!     `ySize`), integer divisions over the port rect (@0xa9ce7-0xa9d25).
//!     Frames are cut row by row, left to right (@0xa9f3c-0xaa045), each an
//!     `xSize` x `ySize` rect (`_SWCreateFrameFromGWorldAndRectPartial`
//!     @0xa9fb5) whose pixel mask is the inverted mask `GWorld` at the same
//!     rect (+0x40/+0x48, @0xad8c2-0xad908); the cut moves down a row when
//!     the next rect passes the picture's width, so the columns are width /
//!     `xSize`, not `xTiles`.
//!   - When `MasksID == SpritesID` the mask comes from the sprite itself
//!     (@0xa9cd0 → @0xa9e30 `_SWCreatePixelMaskFromGWorld`): with the
//!     transparent colour white (`_gSWTransparentColor` @0xdf020 =
//!     ffff ffff ffff), `_SWCreateMaskAndFixImage` (@0xa8848) makes a pixel
//!     whose `& 0x7FFF` is `0x7FFF` transparent and zeroes it, every other
//!     pixel opaque (@0xa8671-0xa86b4).
//!   - The extra mask inversions for a world of 8 bits or fewer
//!     (@0xa9e54-0xa9e7b, @0xa9ece-0xa9edf) are not reached at 16 bits.
//! - **F3. Turning the cut into RLE** (`_ConvertSpriteToRLE` @0x1e489;
//!   `_SWCreateRLEData` @0xaec89; `_BlitPixieRLEParsePixels` @0xc2875).
//!   - The picture's area must equal frame width × height × frames
//!     (@0x1e4a0-0x1e4e4), true only for a whole grid. Otherwise it logs
//!     (@0x1e4e6), returns -1, and the loader raises
//!     `_ReportSpriteLoadError` kind 4 (@0x1ed7b); the master keeps frames
//!     with no RLE data (+0xd4 null, each frame's +0x68 null).
//!   - `_SWCreateRLEData(sprite, 0)` takes frame 0's depth (16), needs each
//!     frame's mask (@0xaee0d → error 0x6a), and calls
//!     `_BlitPixieRLEParsePixels`, which keeps a pixel exactly where its
//!     mask pixel is 0 (@0xc2958-0xc297e). After the inversion that is
//!     where the mask `PICT` was white at RGB555: white in the mask shows
//!     the sprite, black, grey and every other colour hide it, and the
//!     mask's uncovered pixels, erased white, show. Shown pixels keep their
//!     RGB555 value, black included. The mask is stepped by the sprites'
//!     rowBytes (@0xc2b88-0xc2b8e). No fill token (op 4) is emitted: state
//!     3 is never entered. The `GWorld`s are disposed (@0x1e518-0x1e5ba).
//! - **F4. An `rlëD` of depth 8** (`_SWCreateSpriteFromRLEResource`
//!   @0xa9990; `_SWCreateFrame` @0xad556; `_BlitPixieRLETranslucentDrawProc`
//!   @0xbb3fc; `_BlitPixieRLE` @0xbddd0).
//!   - The loader reads the header (@0xa99e9-0xa99fb) and makes min(count,
//!     `xTiles` × `yTiles` when that is above 0) frames at the header's
//!     depth (@0xa9a28-0xa9a41, @0xa9ac7). The frame-making flag is
//!     `maskType & 7` = 0, so `_SWCreateFrame` makes no `GWorld` (@0xad62c →
//!     @0xad6f2): each frame holds only its depth (+0x20 = 8) and shift
//!     (+0x5e = 0). `GetPixDepth` (@0xad64b) is in the `GWorld` branch,
//!     reached only by the legacy pipeline. No CLUT is read on this path.
//!   - The resource flipper `_FlipRle8` (@0xa5cd6, installed for `rlë8`
//!     and `rlëD`, @0xa58a7, @0xa58cf) swaps the header words;
//!     `_BlitPixieRLESwap` (@0xbe3a8) byte-swaps each token and run word
//!     (@0xbe440), 16-bit pixels (@0xbe44d) and 32-bit pixels (@0xbe46e),
//!     and leaves 8-bit pixel bytes as they are (@0xbe3f0-0xbe3f7).
//!   - Every stellar slot's draw proc is `_BlitPixieRLETranslucentDrawProc`
//!     (`_HandleStellarSystemVisibility` @0x33052 sets it from 0x3b532c).
//!     For a depth-8 frame with no translucency (+0xa8 ≤ 0) and no bias
//!     (+0xb0 ≤ 0) it calls plain `_BlitPixieRLE` (@0xbb792 → @0xbb8b6 →
//!     @0xbb9fb → @0xbb9c3; clipped, `_BlitPixieRLEClipped` @0xbb89f →
//!     @0xbb751), with the destination addressed by the 16-bit world's
//!     shift (@0xbb53c-0xbb555).
//!   - `_BlitPixieRLE` works in bytes: op 1 moves to the next row, op 2
//!     copies *count* bytes (`_BlitPixieMemCopy`) and aligns the stream to
//!     4, op 3 skips *count* bytes, and op 4 calls
//!     `_BlitPixieMemSet(dst, run word, count)` (@0xc0452), which stores
//!     the swapped word in 4-byte chunks, then a 2-byte tail of its low
//!     half and a 1-byte tail of its low byte (@0xc0452-0xc049a): the file
//!     word `b0 b1 b2 b3` lands as `b3 b2 b1 b0` repeating.
//!   - So row *y*'s bytes land at the frame's left edge in the 16-bit
//!     little-endian world: pixel *k* is `byte[2k] | byte[2k + 1] << 8`,
//!     RGB555 with its top bit ignored. The picture fills the left half of
//!     its frame box, each pair of colour indexes one meaningless colour,
//!     and a pixel only one of whose bytes is written keeps the
//!     background's other byte.
//!   - The same store makes a 16-bit run's pixel *k*, counted from the
//!     run's start, the word's low half when *k* is even and its high half
//!     when *k* is odd. The game's own encoder never emits a run (its dead
//!     run code would repeat the pixel into both halves, @0xc29d7-0xc29f4),
//!     so only a sheet from another encoder can tell the readings apart.
//!   - The frame rect (+0x22), used for clicks, collision and the slot
//!     rect, is the header's width × height whatever the depth, so the
//!     catalog's frame sizes are unchanged by the reading.
//! - **F5. What happens when loading fails.** Every `SpriteWorld` assert on
//!   these paths quits the game: `_SWAssertFail` (@0xa7a50) shows a stop
//!   alert (@0xa7b04), beeps (`SysBeep` @0xa7b15) and calls `ExitToShell`
//!   (@0xa7b2d).
//!   - A missing `PICT` or mask fails `_SWCreateSpriteFromSinglePictXY`
//!     (@0xa9c5a; mask @0xa9c83-0xa9c90), leaving the master null; the
//!     loader raises alert kind 3, then `_SWLockSprite(NULL)` asserts
//!     (@0xaa96e-0xaa973 → @0xaaaad-0xaaabc). (`_LoadSprites` would also
//!     write +0x54 through null, @0x2074d.)
//!   - An `xSize` or `ySize` of 0 divides by zero (@0xa9cff, @0xa9d1d); a
//!     negative one fails the rect assert in `…Partial` (@0xad818-0xad844).
//!   - A picture smaller than one frame cuts no frames, and
//!     `_SWSetCurrentFrameIndex(0)` (@0xa9f12) divides by a count of 0
//!     (@0xa90e7).
//!   - A picture that is not a whole grid raises alert kind 4 (F3), and the
//!     first draw asserts on the frame's null RLE pointer (@0xbb43c-0xbb441
//!     → @0xbb66d).
//!   - A mask of another width is read sheared, and a shorter one past its
//!     end, as it is stepped by the sprites' rowBytes.
//!   - `_SWLockSprite`'s frame walk (@0xaaa10-0xaaa5b) asserts when an
//!     `rlëD`'s line-start counts do not chain to another line start or
//!     the end.
//! - **Sounds.** None of these routines calls `PlaySound` or `_beepSnd`.
//! - **Left out.**
//!   - The quit and crash paths of a `PICT` pair (F5): the original's bugs,
//!     each ending the game, and a crash is not a reading. The engine reads
//!     each as no sheet with a reason (`NoSheet`, `NoMask`, `Picture` with
//!     `BadDimensions`, `NoFrames` or `PartialGrid`), so phase 9's and
//!     phase 3's fallbacks apply. No key.
//!   - The stop alerts of `_ReportSpriteLoadError` (kinds 3 and 4, @0x1ed24,
//!     @0x1ed7b; alert through 0x3b6257 @0x1eb62): each precedes a quit,
//!     and the engine lists the reason among the scene's problems.
//!   - A mask of another width than its sprites, or shorter
//!     (@0xc2b88-0xc2b8e): a memory-layout accident, read by the engine as
//!     `MaskSizeMismatch`.
//!   - An `rlëD` whose line-start counts do not chain (@0xaaa1f-0xaaa2e,
//!     @0xaaa4c-0xaaa5b): the original's bug; the engine ignores the
//!     counts, as it always has.
//!   - A 16-bit pixel of an 8-bit sheet only one of whose bytes is written
//!     keeps the background's other byte (F4): a straight-alpha texture
//!     cannot replicate it, so it is drawn clear.
//!   - The legacy pipeline: a world at the device's depth, `rlë8`
//!     (`0x726c9138`, @0x1ebd7-0x1ebe8, @0xa99b4-0xa99bd), `_SWCreateFrame`'s
//!     `GWorld` branch with `GetPixDepth` @0xad64b, and the 8-bit-world mask
//!     inversions (@0xa9e54-0xa9e7b, @0xa9ece-0xa9edf). It runs only when
//!     `_meetsMinimumRequirements` fails (@0xccac1-0xccaca); the engine
//!     models the 16-bit world every supported machine runs.
//!   - An `rlëD` of depth 32, or of any depth other than 8 or 16, loads
//!     (@0xa9a28-0xa9ac7). At 32 bits `_BlitPixieRLESwap` swaps each pixel
//!     (@0xbe46e) and `_BlitPixieRLE`'s byte copy draws it at twice its
//!     width, past its frame box; other depths misparse in the swap
//!     (@0xbe3f0-0xbe401), and the draw proc asserts on a depth of 7 or
//!     less (@0xbb612). The engine keeps `UnsupportedDepth`, so such a
//!     sheet falls back to graphic type 0.
//!   - Ship sprites from a `PICT` pair: `_LoadExtendedShipSprites`
//!     (@0x2298b-0x23917) calls the same cutter for `shän` images, and the
//!     engine's ship sprites still read `rlëD`s only. (An 8-bit ship `rlëD`
//!     now decodes as drawn through the shared decoder, matching the plain
//!     blit.)
//!   - Main-screen `spïn`s 600 to 610 from a `PICT` pair: `_LoadSprites`
//!     reads them the same way (@0x206b0); the main screen reads their
//!     `rlëD` only.
//!   - An 8-bit sheet drawn translucent: `_BlitPixieRLETranslucentDrawProc`'s
//!     depth-8 branches, `_BlitPixieRLETranslucent8`,
//!     `_BlitPixieRLETranslucentClipped8`, `_BlitPixieRLEAddOver8` and
//!     `_BlitPixieRLEAddOverClipped8` (@0xbb7ed-0xbb890, @0xbb8b6-0xbb9ec),
//!     where levels 0x20 and up that are not the plain add-over case draw
//!     nothing (@0xbb80a-0xbb83a, @0xbb8d7-0xbb907). A stellar reaches them
//!     only when murk gives it translucency (+0xa8 > 0), which the engine
//!     does not model yet; a ship reaches them under cloak or fade.
//!
//! ## Built by stellar-fidelity phase 5
//!
//! NPCs land as `_HighLevelAIHandler` state 1 (@0x8d8b8-0x8db5a) lands
//! them (`nova_sim::traffic::land_flight`, `nova_sim::landing::npc_over`):
//!
//! - **When it runs.** Only once the AI wait is out: the handler's wait
//!   gate returns while +0x4c > 0, for every goal but 0xa and 0xe
//!   (@0x8d4b9-0x8d4d2). So a landing goal on a hypergate or wormhole
//!   (`Flags2` & 0x3000, @0x8d8d7-0x8d8fe) becomes a gate entry, state 20
//!   in the same pass (@0x8db5f-0x8db62), only after the wait. With a
//!   travel stellar of -1 the block does nothing (@0x8d8c9-0x8d8d1), which
//!   the engine's landing goal, always naming its stellar, can't reach.
//!   The jump stage +0x50 is zeroed each pass (@0x8d903), as the engine's
//!   `warp_out::hold_stage` already zeroes it for a ship not jumping.
//! - **The box.** The offset from the stellar's centre, truncated to whole
//!   pixels (@0x8d90d-0x8d950), at most 40 on each axis (@0x8d90f), or
//!   trunc((9 − turn) × 8 + 32) in doubles below a `_ShipTurnRate` of 8
//!   (@0xdd044, @0xdda90, @0xdda98, @0xdd578, @0x8d90f-0x8d9a8),
//!   inclusive (@0x8d9d7, @0x8d9f9). The 16-bit `cwde` and `movsx`
//!   (@0x8d929, @0x8d94b, @0x8d9b7) are not replicated.
//! - **Outside**, sub 2, the stellar attacker's approach (@0x8d9b2-0x8da07;
//!   `_LowLevelAIHandler` @0x8563b-0x857bd).
//! - **Inside**, a folding `shän` (0x02) above fold set 0 unfolds
//!   (@0x8da0c-0x8da38), then the per-axis speed gate: 0.35 or more on
//!   either axis (the double @0xdda68, @0x8da41-0x8da8b) brakes by sub 1,
//!   its velocity and speed (+0x48) × 0.98 in doubles each pass (@0xdd988,
//!   @0x8da8d-0x8dad7); below it on both it lands (@0x8dadc).
//! - **Sounds.** None: state 1 calls no `PlaySound` or `_beepSnd`.
//!
//! - **The landing's end** (@0x8dadc-0x8db5a,
//!   `nova_sim::traffic::land_flight::stay`): its velocity and speed none,
//!   goal and sub-goal 0, +0x92 = +0x6c (@0x8db00, the stellar visited);
//!   +0x6c, its target and its provocation are not written; and its wait
//!   +0x4c is `Rand(75)` + 100 for a class of `shïp` `Flags3` 0x0002
//!   (class +0x9ec, copied from the record's +0x726 @0x7a204-0x7a20b;
//!   @0x8db19-0x8db3a), else `Rand(200)` + 300 (@0x8db41-0x8db5a). Its
//!   slot stays active (+0xb8 is not cleared), so it still counts toward
//!   the system's ships and `AvgShips` and keeps its berth.
//! - **The wait.** `_HandleShip` counts +0x4c down by `_gSpeedMult` (1.0
//!   a frame) for any class but the escape pod (@0x35025-0x35048), and
//!   skips the ship's turn, thrust and fire meanwhile (@0x34a54-0x34a72):
//!   the engine's AI wait, counted down at the end of the fly half,
//!   holding its flight and its guns. `_AIDispatch` runs its type's think
//!   every frame whatever the wait (@0x8ff26-0x90016), in goal 0, so the
//!   idle block finds the stellar visited and sends a trader or warship
//!   away by hyperspace, or stops it when it can't jump; the high level,
//!   skipped while the wait runs (@0x8d4b9-0x8d4d2), flies that once it is
//!   out. A miner's shuttle stays idle through its wait, then picks the
//!   nearest other stellar.
//! - **Mission ships.** By the engine's reading of
//!   [`landed_leaves`](crate::RuleKey::LandedLeaves) a mission ship stays
//!   too, and its landing counts nothing; by the other it leaves the system
//!   as it lands, counted as one that jumped out.
//!
//! **Readings.** The Rulebook key
//! [`npc_landing`](crate::RuleKey::NpcLanding) defaults to the engine's
//! box. The other reading, the port's earlier landing, lands within the
//! stellar's landing radius at or below 1.0 a tick, flying its own
//! approach; the Bible says nothing of how an NPC lands. The landing's end
//! and its wait are the original's under either reading.
//!
//! ## Built by stellar-fidelity phase 6
//!
//! - **The player's landing speed.** `nova_sim_state::landing::check_landing`
//!   reads the speed by [`landing_speed`](crate::RuleKey::LandingSpeed):
//!   by the engine a ship is too fast unless |vx| and |vy| are each at
//!   most `PER_AXIS_LANDING_SPEED`, 0.75, compared in single precision,
//!   inclusive, a NaN on either axis refused. So a ship at 0.7 on each
//!   axis (about 0.99 overall) lands, and one at 0.8 on one axis is too
//!   fast. `Session::land_press` passes the session's reading to
//!   `land_or_select`. The refusal, its wording and its beep are as
//!   before.
//!
//! **Readings.** The Rulebook key
//! [`landing_speed`](crate::RuleKey::LandingSpeed) defaults to the
//! engine's per-axis gate. The other reading is the port's earlier check,
//! the overall speed at most 1.0 a tick. The Bible says nothing of a
//! landing speed (the manual says only "moving too fast").
//!
//! ## Checked for stellar-fidelity phase 6
//!
//! `_HandlePlayerDockRequest` (@0x66691-0x67ef9), after the request's
//! earlier steps (landability @0x6695c-0x669ae, the fee @0x669b4-0x66a80,
//! the cloak @0x66a85-0x66ae9, disabled and dying silently to the
//! epilogue @0x66aee-0x66b14, the station byte @0x66b1a-0x66b3f, and
//! uninhabited "No response." @0x66b51-0x66bcd), gates the landing in
//! this order:
//!
//! 1. The target is the request's (@0x66bd2-0x66be5), else the request
//!    branch at 0x6788f.
//! 2. `_FloatAbs` of vx (+0x20) and vy (+0x24), each stored as a float
//!    (@0x66beb-0x66c1d).
//! 3. The counter `_dockPermission` (0x3b544c) above 0x2ed
//!    (@0x66c13-0x66c23; a 16-bit `cmpw`, not replicated), else 0x676db.
//! 4. The AI wait: the ship's +0x4c above 0 refuses (@0x66c29-0x66c36).
//!    The player's +0x4c is never above 0 (zeroed @0x66b6d; see
//!    ship-handling-fidelity phase 2), so this always passes.
//! 5. |vx| against the **single-precision float** 0.75 at 0xdd658 (bytes
//!    `0000403f`; the phase plan said a double, which it is not):
//!    `ucomiss`, `ja` refusing above and `jp` on NaN (@0x66c3c-0x66c51).
//! 6. |vy| against it, `jb` refusing above and, the compare unordered, on
//!    NaN (@0x66c57-0x66c66). So the gate is per axis and inclusive.
//! 7. The reach box (@0x66c6c-0x66cfd, built by phase 10), else "too far"
//!    at 0x67687.
//!
//! The refusal (@0x676db-0x67838) beeps `_beepSnd[3]`, `snd ` 153
//! (@0x676fe; silent for a `_dockTarget` above 0x7ff, @0x67711),
//! recomputes the reach (@0x67720-0x677b2) and says "too fast" (`STR#`
//! 2002 #69-#72) only when the ship is inside it on both axes and the
//! counter is above 0x2ed (@0x677b2-0x677c0), else "too far" (#65-#68),
//! for 360 ticks (@0x6787f). The engine's refusal order already matched;
//! only the speed compare changed. The fee's branches (@0x669b4-0x66a80,
//! @0x66db9-0x66df5) are built by stellar-fidelity phase 15; the cloak refusal
//! is cloaking-fidelity phase 8's; every other branch was already built.
//!
//! ## Built by stellar-fidelity phase 7
//!
//! - **The owner test** is `Chart::held_here` (`nova_sim`'s
//!   `session::chart_ownership`): the stellar's holder
//!   ([`OverlaidGalaxy::holder`], its owner, else the rescue to the
//!   player's system) is the player's system.
//! - **The width.** The player's landing reach and the NPCs' gate entry
//!   box read `TypeWidth::Unloaded`, 150, for a stellar not held, ahead of
//!   phase 3's master rule, as `_GetStellarSpriteXSize`'s owner test runs
//!   ahead of its master tests. So such a stellar reaches trunc(150 ×
//!   1.75) = 262 and a gate a quarter of 150, 37.
//! - **The press.** `land_or_select` refuses a stellar not held as not
//!   landable, after `is_landable` and before the uninhabited, request,
//!   speed and reach tests, with the not-landable line and its beep.
//!
//! Every stellar shown is held ([`OverlaidGalaxy::shows`]), so these
//! bite only on a stellar kept among the sites after the player's system
//! changed without a rebuild point. By the other reading of
//! [`landing_distance`](crate::RuleKey::LandingDistance), the circle, the
//! owner counts for nothing, as the master doesn't. The Bible says nothing
//! of ownership and the landing width, so there is no new key.
//!
//! ## Checked for stellar-fidelity phase 7
//!
//! - **The width's owner test.** `_GetStellarSpriteXSize`
//!   (@0xac91-0xad37) gives 150 when the stellar's owner word (+0x14) is
//!   not the player's system, the first ship's +0x74 through 0x3b5138
//!   (@0xacb2-0xacc1). It comes after the index test (@0xac9c) and before
//!   the `Type` range and master tests (@0xacc3-0xace6). The master table
//!   `_masterStellarSprite` (0x3b5040) is indexed by `Type`
//!   (@0xacd2-0xacda), not by system: a shared stellar's 150 elsewhere is
//!   the owner test, not a master loaded for one system only. The
//!   function plays no sound.
//! - **Who owns, and when.** Unchanged: see "Visibility and governments"
//!   and the ownership checks of crons-news-ranks phase 5, missions-fidelity
//!   phase 11 and crons-news-ranks-fidelity phases 18 and 38. The owner is
//!   the lowest-ID visible lister no lower visible system at its place
//!   shadows, recomputed only at a galaxy refresh; a rebuild only rescues
//!   an unowned stellar to the player's system.
//! - **The slot rule.** `_HandleStellarSystemVisibility` keeps a master
//!   and gives a sprite slot only to a stellar active and owned by the
//!   player's system (@0x32cf9-0x32d4c, @0x32ee4-0x33229). In another
//!   system that lists it, the stellar is not drawn, not clicked (the
//!   click walks the slots at 0x3b53d0, `_HandlePlayer`
//!   @0x6a5f9-0x6a686) and not hit (`_SWRLECollision` @0x372a8).
//!   [`OverlaidGalaxy::shows`] already gives this.
//! - **The dock request refuses first.** `_HandlePlayerDockRequest`
//!   tests the held stellar (+0x6c): `_StellarIsLandable` (@0x66977), its
//!   owner the player's +0x74 (@0x6699a-0x669a4), and active (@0x669aa).
//!   Any failure goes to 0x67cda, the not-landable line (`STR#` 2002
//!   #84-#90) and `_beepSnd[3]`, `snd ` 153 (@0x67e8f). Both its width
//!   calls come later (@0x66cd9, @0x6773a), so the player is never landed
//!   on a stellar another system owns at 150 × 1.75.
//! - **NPC gate entry reads the 150.** `_HighLevelAIHandler` state 20
//!   (@0x8dbdf) takes `_GetStellarSpriteXSize(gate) / 4`, inclusive: 37
//!   for a gate the player's system doesn't own. An NPC holds one as its
//!   destination: `_SelectRandomStellarDest` picks from its system's
//!   `NavDefs` (@0x806bd-0x8087c), testing only `_StellarIsLandable`
//!   (@0x8074e) and `Flags`/`Flags2`, never the owner or active. A ship an
//!   in-flight `M` or `N` leaves behind can hold one on its last tick, but
//!   is freed before it is drawn (`_HandleShipDisplay` @0x2ded6-0x2dee7).
//! - **Readers with no owner test.** `_FindNearestStellar`
//!   (@0x276c-0x2888) walks the ship's system's 16 `NavDefs`, requiring
//!   active (@0x27c3) and `Flags` 0x01 (@0x27cd), a gate only within the
//!   squared distance at 0xdd050 (@0x2841-0x2853). The land key with
//!   nothing held (@0x6a8ed-0x6a932) requests its pick, which the dock
//!   request then refuses when unowned; the tutorial's land hints name it
//!   (@0x709e8, @0x70bdc). The slot keys and F5 (`_HandlePlayer`
//!   @0x69e28-0x69e94) take `NavDefs[slot]` (@0x69e2e-0x69e46) or the
//!   nearest pick (@0x69e59) and act only on an owned one (@0x69e88), with
//!   no sound: slot N is `NavDefs` index N, gaps included. The status
//!   radar (`_DrawStatusRadar` @0x498ad-0x49a8a) draws every non-empty
//!   `NavDefs` entry, with no active or owner test, sizing the blip by
//!   whether the `Type` master is loaded (@0x499ef-0x49a27). The land
//!   key's and F5's nearest pick, with the gate limit and the owner drop,
//!   are built by stellar-fidelity phase 14 (below). The slot keys, the
//!   tutorial hints' and `_DoNewPilot`'s candidates, NPC destinations and
//!   the radar still read only the stellars shown:
//!   rdm:phase/stellar-fidelity-followups/phase-5-nearest-and-slot-picks-over-navdefs,
//!   rdm:phase/stellar-fidelity-followups/phase-6-npc-destinations-over-navdefs
//!   and rdm:task/radar-blips-every-listed-stellar.
//!
//! ## Built by stellar-fidelity phase 8
//!
//! A sprite slot cloning a graphic whose `spïn`'s `xTiles` × `yTiles` is
//! 0 or less keeps the frame it showed, as the original's code leaves it
//! with its faults stepped over (see "Checked for stellar-fidelity phase
//! 8"):
//!
//! - **from a rebuild point**, which empties every slot, it shows no
//!   frame: nothing is drawn, the dock request's reach is 75, the click
//!   falls back to the 64-pixel placeholder, and shots and deadly discs
//!   read nothing (`DrawnSheet::NoFrame`);
//! - **between rebuild points**, a flight frame's clone of such a master
//!   into a slot showing a frame keeps that frame, frozen at its index and
//!   placed as a 32-pixel sprite, its top left at the stellar's centre
//!   less 16 on each axis (`nova_sim::Session::frame_frozen`). The dock
//!   request then reads the loaded `Type`'s 32, a reach of 56;
//! - **a stellar whose last rebuild point copied such a count** never
//!   animates until the next (`nova_sim::Session::counts_no_frames`), even
//!   when a later clone brings counting frames into its slot.
//!
//! A product of exactly 0, which divides by zero in the original, is read
//! as the negative case. The state is `Rebuilt`'s (see the `rebuild`
//! submodule): whether each slot's frames count none, the sheet whose
//! frame a slot kept through such a clone, and the stellars that count
//! none, shown through `Rebuilt::showing`, `Rebuilt::frozen` and
//! `Rebuilt::still`. The catalog says whether each `DeadType` counts
//! (`LandingSite::dead_counts_frames`) and whether graphic type 0 does
//! (`PilotCatalog::fallback_counts_frames`), and graphic type 0's fill is
//! the three-way `rebuild::Fallback`. The slot's size as
//! `_GetSpriteXSize`/`_GetSpriteYSize` read it
//! (`nova_sim`'s `Space::slot_sprite_size`) places the slot and sizes a
//! hypergate's approach. The view draws a frozen frame shifted and keeps
//! its frame shown while its count steps on, a hypergate's included
//! whatever its session steps it to, reading the frozen state after each
//! tick's steps, since a flight frame changes it with no event.
//!
//! **Readings.** The Rulebook key
//! [`uncounted_sheet`](crate::RuleKey::UncountedSheet) defaults to the
//! engine's reading above. Its other reading is the behaviour before this
//! phase: the sheet's own decoded frames are drawn and animated and the
//! slot shows a frame, so a loaded `Type` reaches 56. That is also what the
//! rebuild's repair (@0x332f7) means a frameless slot to get. The Bible
//! says nothing of `xTiles` × `yTiles`.
//!
//! ## Checked for stellar-fidelity phase 8
//!
//! - **F1. `_SWSetCurrentFrameIndex`** (@0xa90b6) reads a negative index
//!   as 0 (@0xa90d4-0xa90df) and divides it by the slot's count +0x54
//!   (`idivl` @0xa90e7). It returns at @0xa919c when the count is not
//!   above the remainder (`jle` @0xa90ec-0xa90ee), before it writes the
//!   current frame +0x50 (@0xa9136 or @0xa91d2), the rect +0x1a..+0x20
//!   (@0xa913f-0xa9191), the index +0x68 (@0xa9195) or the moved byte
//!   +0x15 (@0xa9198). A count of 0 divides by zero (#DE @0xa90e7); a
//!   negative count gives a remainder in [0, |count|), never below the
//!   count, so it returns with nothing written.
//! - **F2. `_SWCloneFramesFromMaster`** (@0xac436) returns -1 with the
//!   slot untouched for a null master or slot (@0xac448-0xac462), a master
//!   with no frames (@0xac463-0xac468) or a slot owning its frames (+0xd9,
//!   @0xac46a). Otherwise it copies +0x4c, +0x98, +0x9c, the count +0x54
//!   (@0xac4ec-0xac4ef), +0x58 and +0xd4 (@0xac4ce-0xac4fe); keeps the
//!   slot's index +0x68 while below the new count, else 0
//!   (@0xac504-0xac50a, @0xac532); calls `_SWSetCurrentFrameIndex`
//!   (@0xac514); then reads the slot's current frame's lock byte (+0x50 →
//!   +0x1c) with no null test (@0xac519-0xac51c), locking the slot when it
//!   is clear (@0xac53f), and sets +0xb0 to 0 (@0xac524). A negative-count
//!   master replaces the slot's frames and count but leaves its frame,
//!   index and rect; with no current frame, the read at @0xac51c is
//!   through null, a fault.
//! - **F3. The rebuild point** (`_HandleStellarSystemVisibility`
//!   @0x32aa5). The clear nulls +0x4c (@0x32e71), +0x50 (@0x32e7a) and
//!   +0xd4 (@0x32e83) of every cloned slot (+0xda), keeping +0x54, sets
//!   +0xd0 to -1 (@0x32e8f) and hides it (@0x32ea6). For each stellar
//!   shown the rebuild loads the master if missing (@0x32f37-0x32f46),
//!   makes the slot if none (`_SWCreateEmptySprite` @0x32f80, which nulls
//!   +0x50 and +0x54, @0xa8eaf-0xa8eb6), clones only a master that is not
//!   null (@0x32fe0-0x32ff6), locks (@0x3300d), sets the draw proc
//!   (@0x33052) and shows the slot (@0x3306b), calls
//!   `_SWSetCurrentFrameIndex(slot, +0x476)` (@0x33096), places it
//!   (`_GetSpriteYSize` @0x330f3, `_GetSpriteXSize` @0x33143,
//!   `_SWSetSpriteLocation` @0x331aa), copies the count, +0x474 = slot
//!   +0x54 (@0x331c6-0x331c9), and resets a new slot's animation
//!   (@0x331da-0x331fb). The clone always meets a slot with no current
//!   frame, so a rebuild point showing a stellar whose shown graphic's
//!   tiles multiply to 0 or less faults in the original: #DE @0xa90e7 from
//!   @0xac514 for 0; the null read @0xac51c for a negative product, and
//!   past it `_SWSetSpriteLocation` (@0xab640) reads the null frame's hot
//!   spot (+0x2a/+0x2c, @0xab65e-0xab669, fault @0xab665, from @0x331aa).
//!   The phase body's "the slot shows no frame, so the reach is 75" is
//!   the state the code leaves with those faults stepped over: +0x50 stays
//!   null, as every `_SWSetCurrentFrameIndex` refuses the count.
//!   - **The check @0x33231-0x33321.** A slot in use whose +0x4c and
//!     +0xd4 are not null skips graphic type 0's fill (@0x33241-0x33250)
//!     and goes to @0x332d7: with +0x50 null it logs "warning: stellar
//!     sprite had invalid frame pointer" (0xd8f04, @0x332de-0x332e5) and
//!     calls `_SWSetCurrentFrameIndex(slot, 0)` (@0x332f7), which refuses
//!     (or faults on) the same count; with a frame whose lock byte +0x1c
//!     is clear it logs "warning: stellar sprite frame was not locked"
//!     (0xd8f38, @0x33304-0x3330b); then it locks the slot (@0x33315). Its
//!     repair cannot fill such a slot.
//!   - **Graphic type 0's fill** (@0x332a1-0x332d0): when `spïn` 1000
//!     itself counts no frames, the fill clones it (master 0 loaded) and
//!     its `_SWSetCurrentFrameIndex(slot, 0)` (@0x332d0) refuses or faults:
//!     the slot holds type 0's frames and shows none.
//!   - `_SWLockSprite` (@0xaa965) locks no frame for a count of 0 or less
//!     (@0xaa97c-0xaa981, @0xaa9c7-0xaa9cc), so it does not fault.
//! - **F4. The sprite tick** (`_HandleStellarSprites` @0x2e6f1), the whole
//!   function as it touches this. For each visible, owned slot
//!   (@0x2e713-0x2e73e) it clones the shown master (@0x2e7e5). It animates
//!   only when the stellar's own count +0x474 is above 1 (@0x2e802); its
//!   `idivl`s by +0x474 (@0x2ed44, @0x2edf2) and `_Rand(+0x474)` calls
//!   (@0x2ecf7, @0x2edb3) are behind that test, so never divide by 0. It
//!   calls `_SWSetCurrentFrameIndex(slot, frame 0 or current)`
//!   (@0x2ee20-0x2ee30), sizes the slot (`_GetSpriteXSize` @0x2ee3d,
//!   `_GetSpriteYSize` @0x2ee4e) and places it (`_SWSetSpriteLocation`
//!   @0x2ef1a). A hypergate's approach reads the slot's `_GetSpriteXSize`
//!   doubled (@0x2e906-0x2e90e). A clone of a negative-count master into a
//!   slot showing a frame replaces its frames and count; each
//!   `_SWSetCurrentFrameIndex` refuses, so the slot keeps showing its
//!   frame, frozen; `_GetSpriteXSize`/`_GetSpriteYSize` (@0x26d4-0x2701,
//!   @0x2703-0x2731) give 32 when +0x50 is null or +0x68 is not in [0,
//!   +0x54) (@0x26e5-0x26ef, @0x2714-0x271e), so the slot is placed as a
//!   32-pixel sprite, and the stale frame keeps its own size
//!   (`_SWSetSpriteLocation` moves the rect without resizing it,
//!   @0xab66d-0xab6a0). A product of 0 faults there (#DE @0xa90e7 via
//!   @0xac514); a clone into a slot with no frame faults at
//!   @0xac519-0xac51c; a clone of a counting master into a frameless slot
//!   sets its frame normally. No routine here calls `PlaySound` or
//!   `_beepSnd`, so there is no `SimSound`.
//! - **F5. Drawing.** The draw proc draws the slot's +0x50. The sprite
//!   library's own frame advance (`_SWProcessSpriteLayer`
//!   @0xb8110-0xb8118) skips a sprite whose frame time +0x5c is negative; `_SWCreateEmptySprite`
//!   sets it to -1 (@0xa8ec4), the clone leaves it, and nothing calls
//!   `_SWSetSpriteFrameTime` (@0xab5c3). So a frozen slot stays frozen.
//! - **F6. Who else reads the count.** The stellar's +0x474:
//!   `_InitObjects` (@0x1c8b9); the tick (@0x2e802-0x2ede3); the rebuild's
//!   copy (@0x331c6-0x331c9), so a count of 0 or less keeps the stellar
//!   still until the next rebuild point; and the dock request's take-off
//!   (@0x67616-0x67646), which sets the current frame to count − 1, the
//!   next to the current and the timer to −`AnimDelay`, with no effect for
//!   a count of 1 or less. The slot's +0x54, through
//!   `_GetSpriteXSize`/`_GetSpriteYSize`: the rebuild (@0x330f3,
//!   @0x33143), the tick (@0x2e906, @0x2ee3d, @0x2ee4e), the nav target
//!   brackets (`_HandleNavTargetSprites` @0x2fd6a, @0x2fe00, @0x2fe10) and
//!   the hail dialog's picture (`_PlanetCommDialogUpdate`
//!   @0x90f97-0x90faa). The `Type` master's count:
//!   `_GetStellarSpriteXSize` (@0xaced-0xacff, phase 10's 32), read by the
//!   dock request (@0x66cd9, @0x6773a) and NPC landing and gate entry
//!   (@0x8dbdf), and the status radar (`_DrawStatusRadar`
//!   @0x499e8-0x49a3f). The dock request tests the slot's +0x50
//!   (@0x66ccf) and uses 75 when it is null (@0x66cc7).
//! - **F7. Decisions.** The faults are the original's bug: not copied and
//!   no key, as a crash is not a reading (as in stellar-fidelity phases 3
//!   and 4). The stale frame is not a memory-layout accident, so it is
//!   ported: between rebuild points a slot belongs to one stellar (+0xd0 is
//!   set at the rebuild, slots handed out in `spöb` order), the rebuild
//!   nulls +0x50 before any slot changes hands (@0x32e7a), and masters are
//!   freed only at a rebuild point (@0x32d18-0x32e4e). So the stale frame
//!   is always a live frame of a sheet the same stellar's slot held since
//!   the last rebuild point: its `Type`, its `DeadType` or graphic type 0.
//! - **Left out.**
//!   - The faults: #DE @0xa90e7 for a product of 0 (from the clone
//!     @0xac514, the rebuild @0x33096, @0x332d0 and @0x332f7, and the tick
//!     @0x2ee30); the null read @0xac519-0xac51c; and
//!     `_SWSetSpriteLocation`'s null hot-spot read @0xab665 (from @0x331aa
//!     and @0x2ef1a).
//!   - The crash-log lines "invalid frame pointer" (@0x332de) and "frame
//!     was not locked" (@0x33304): diagnostics with no effect on play.
//!   - Shots and deadly stellars against a frozen slot, the nav target
//!     brackets and the hail dialog's picture sized by the slot, and the
//!     scene's problems for an uncounted `spïn` (the original reports
//!     nothing; none is added):
//!     rdm:phase/stellar-fidelity-followups/phase-7-uncounted-slot-readers.
//!   - A positive `xTiles` × `yTiles` below the `rlëD`'s frame count: the
//!     loader makes min(count, tiles) frames (@0xa9a28-0xa9a41) and the
//!     count is the tiles (@0x1ed8c-0x1ed97), so the original animates
//!     over the tiles; the view counts the decoded frames:
//!     rdm:phase/stellar-fidelity-followups/phase-8-tiles-below-frame-count.
//!   - Memory-layout accidents, not copied: a slot reused across a system
//!     change that keeps another stellar's +0x54 through a failed clone
//!     (@0x32fe0-0x32fea, @0x331c6), the engine keeping the state per
//!     stellar as phase 1 does; and tiles above the frame count, which
//!     read past the frame table (@0xa90f7).
//!   - The take-off's +0x474 write (@0x67616-0x67646) for a count of 1 or
//!     less: it has no effect.
//! - **Left out, depending on unlanded work.**
//!   - Clicking a frozen or frameless slot (`_HandlePlayer`
//!     @0x6a5f9-0x6a686 reads the slot's rect +0x1a): a frozen slot's rect
//!     is the stale frame's, its top left at the centre less 16, and a
//!     frameless one has none. Built by stellar-fidelity phase 9 (below).
//!   - The status radar's blip for a `Type` master that counts none (32,
//!     the smallest): it depends on rdm:task/radar-blips-every-listed-stellar;
//!     rdm:phase/stellar-fidelity-cross-roadmap/phase-2-radar-blip-uncounted-type.
//!
//! ## Built by stellar-fidelity phase 9
//!
//! A click in space and the hover pick a stellar by its sprite slot's
//! rect, as `_HandlePlayer` (@0x6a5e9-0x6a717) and `_HandleMouse`
//! (@0x3823f-0x38276) test it.
//!
//! - **The rule** is one type, `navigation::SpriteRect` (the slot's +0x1a,
//!   in system coordinates): `SpriteRect::holds` grows the rect by 16 on
//!   every side when its width is 47 or less (`SMALL_SPRITE_WIDTH`,
//!   whatever its height), then tests it half-open, left and top in,
//!   right and bottom out. `SpriteRect::centred` places a frame around a
//!   centre with its top left at half the size, halved as integers, so an
//!   odd size's extra pixel is on the right or the bottom. The ship click
//!   and hover box (`hover::in_box`) is the same rule around the view's
//!   drawn centre.
//! - **The order**: `navigation::stellar_under` and `stellar_under_past`
//!   give the lowest `StellarId` among the stellars whose rect holds the
//!   click, the one selected in mode 2 passed over.
//! - **The slot's rect** is `Space::click_rect` (the session's `stellars`
//!   module): none for a slot showing no frame (`DrawnSheet::NoFrame`);
//!   else the frame drawn (the 64-pixel placeholder when no sheet is),
//!   its top left at `sprite_pool::offset(position, player, slot size)`
//!   from the player, the slot size `Space::slot_sprite_size` gives: 32
//!   square for a frame frozen, so a frozen slot's stale frame sits with
//!   its top left at the centre less 16, and the frame's own size
//!   otherwise. `nova_sim::Session::click_in_space` and
//!   `nova_sim::Session::hover` read it.
//! - **Readings.** No key: the Nova Bible says nothing of a click's box,
//!   margin, edges or order (as story-runtime phase 11 found).
//! - **Left out**: whole-pixel placement of the drawn sprites and of the
//!   ship box (`_HandleStellarSprites` @0x2eeab-0x2ef1a,
//!   `_HandleShipDisplay` @0x2cd1c-0x2ce31): the view draws at float
//!   positions, centred, at most a pixel off:
//!   rdm:phase/stellar-fidelity-followups/phase-9-whole-pixel-sprite-placement.
//!   One `PtInRect` rule shared with the view
//!   (`Bounds::contains_half_open`, roadmap play-feel, not yet on main)
//!   cannot be: the sim crates sit below the view.
//!
//! ## Checked for stellar-fidelity phase 9
//!
//! Read with `otool -tV -arch i386` on `EV Nova.app/Contents/MacOS/EV Nova`.
//!
//! - **The jump gate** (@0x6a5e9-0x6a5f3): the click goes no further
//!   while the player's +0x50 > 0; built (`click_in_space` returns during
//!   a jump).
//! - **The walk** (@0x6a5f9-0x6a611, @0x6a70c-0x6a711) over the 16 slot
//!   pointers at `*0x3b53d0`, in slot order, skipping a null slot
//!   (@0x6a603) and one whose visible byte +0x14 is clear (@0x6a60b).
//!   The visible byte is set for each owned slot at a rebuild point
//!   (`_SWSetSpriteVisible(1)` @0x3306b); `_HandleStellarSprites` re-shows
//!   only visible slots (@0x2e720-0x2e74f); `_SWHideAllSprites` runs at
//!   @0x6c592, followed at once by HSSV (@0x6c597), and at @0x68e04 in
//!   the pod's fade-out. So no click falls where an owned slot is hidden.
//! - **The owner and +0x44** (@0x6a615-0x6a637): the owner is the slot's
//!   +0xd0, and a stellar whose record byte +0x44 is clear is skipped.
//!   +0x44 is "active this visit": `_HandleStellarSystemVisibility`
//!   clears it for every spöb (@0x32c42) and sets it (@0x32cdf) for one
//!   with +0x45 set in a system whose sÿst +0x1eb is set, and hands slots
//!   out only to active stellars (@0x32ee4). The other writes are
//!   load-time (`_InitObjects` @0x1c880, `_LoadObjectData` @0x77244,
//!   @0x77584, @0x779bf, @0x789c0) or to shot records
//!   (`_HandleStellarWeapons` @0x2f1f1, `_SpawnShot`). So every owned
//!   slot's stellar is active, which the engine's `sites` (the stellars
//!   shown) already match: no code.
//! - **The rect** (@0x6a63d-0x6a646): +0x1a, a QuickDraw `Rect` (`top,
//!   left, bottom, right`), copied to -0x38(%ebp); -0x36 is left, -0x32
//!   right.
//! - **The margin reads the width** (@0x6a649-0x6a671): right less left,
//!   `cmpl $0x2f` / `jg` past `_InsetRect(r, -16, -16)`. The same test
//!   guards the player's own sprite (@0x6a3b3-0x6a3db), each NPC
//!   (@0x6a4b7-0x6a4df) and the hover (`_HandleMouse` ships
//!   @0x38183-0x381a8, stellars @0x3823f-0x38264).
//! - **`_PtInRect`** (@0x6a676-0x6a68d, and @0x6a3f0, @0x6a4f4, @0x381ba,
//!   @0x38276): left ≤ h < right and top ≤ v < bottom.
//! - **The selected stellar is passed over** (@0x6a68f-0x6a69d) when the
//!   owner is +0x6c and +0x6a is 2; built (`stellar_under_past`).
//! - **A pick** (@0x6a69f-0x6a701): `PlaySound(_beepSnd[0], 1,
//!   _gSoundVolume, _gSoundVolume)` @0x6a6c1 (`snd ` 150,
//!   `SimSound::NavSelected`); +0x6c := owner (@0x6a6d5), +0x6a := 2
//!   (@0x6a6ec); `_navTargetExpand` := 256.0 (@0x6a6d9, the brackets'
//!   fly-in); `_statusRedraw` := 1 (@0x6a6e4, no model: the status panel
//!   is drawn every frame); `_dockTarget` and `_dockPermission` := −1
//!   (@0x6a6f2, @0x6a6fc, `port.dock = None`). All built before this
//!   phase. A miss walks on (@0x6a703-0x6a717); the walk's end is silent.
//! - **The order is lowest spöb ID first**: HSSV walks the spöbs in index
//!   order (@0x33216-0x3321d), and each active one the player's system
//!   owns (@0x32ee4-0x32f06) takes the next slot (@0x32f4b-0x32f6b), up
//!   to 16 (@0x33205-0x33214), its owner set (@0x32fda). The 16 cap never
//!   bites: a system's 16 `NavDefs` bound what it shows.
//! - **Where the rect sits** (`_HandleStellarSprites` @0x2ee3d-0x2ef1a,
//!   `_SWSetSpriteLocation` @0xab640-0xab6a0): the top left is
//!   `trunc(spöb − player) + centre − (size / 2)` on each axis
//!   (`cvttss2si` @0x2eed3, @0x2ef01; the halving `shrw`/`sarw` pair
//!   @0x2eec3-0x2eecb, @0x2eef0-0x2ef08), the size from
//!   `_GetSpriteXSize`/`_GetSpriteYSize` (@0x2ee3d, @0x2ee4e), less the
//!   frame's hot spot (+0x2a/+0x2c, @0xab665-0xab669), which is always 0
//!   (`_SWCreateFrame` allocates with `_NewPtrClear` @0xad5d9 and
//!   `_SWSetFrameHotSpot` @0xadf1a has no caller); the rect is moved, not
//!   resized (@0xab66d-0xab693). A frozen slot is sized 32 by
//!   `_GetSpriteXSize`/`_GetSpriteYSize` (@0x26e5-0x26ef, @0x2714-0x271e),
//!   so its stale frame's rect sits at the centre less 16. A frameless
//!   slot's rebuild faults first (`_SWSetSpriteLocation` @0xab665 from
//!   @0x331aa), so it has no rect. Ships are placed the same way
//!   (`_HandleShipDisplay` @0x2cd1c-0x2cd9d, @0x2cdb5-0x2ce31).
//! - **Integer widths**: the rect is shorts; the engine keeps `f32` and
//!   `u32`.
//!
//! ## Built by stellar-fidelity phase 10
//!
//! A stellar's frames advance by the whole of `_HandleStellarSprites`'
//! sequential branch (@0x2ec4b-0x2ee35): `Flags2` 0x0001 and 0x0002, and
//! an `AnimDelay` of 0 or less. The view builds it
//! (`nova_view::system::animation`'s `Frames`), as the frame a non-gate
//! stellar shows is drawn state the session never reads.
//!
//! - **The gate.** A stellar animates when its counted frames are above 1,
//!   its destroyed state is the one 0x0080 names, and it is not a
//!   hypergate (0x1000). No delay gate: a threshold of 0 or less is met
//!   every tick. A hypergate takes its own branch, ported by
//!   `nova_sim::gate_sprite`; where no session steps it (the galaxy
//!   viewer) it stays on frame 0, as a released, closed gate does
//!   (@0x2ec33-0x2ec36).
//! - **0x0001** shows frame 0 between every other frame: 0, 1, 0, 2, 0,
//!   3, 0, 1, ...; with 2 frames, 0, 1, 0, 1. Every visit to frame 0 lasts
//!   `Frame0Bias` × `AnimDelay` when the bias is above 1.
//! - **0x0002** picks each next frame uniformly among the counted frames
//!   but the one just made current; under 0x0001 as well, among 1 to
//!   count − 1 but the current. The pick is one draw over the frames
//!   allowed, on the view's own chance: flight's effects chance (never the
//!   session's), and in the galaxy viewer the one
//!   `AppScreen::with_effects_chance` gives its navigator.
//! - **Frozen** (stellar-fidelity phase 8): a frozen slot's timer, current
//!   and next frames step, its 0x0002 draws included, while the frame
//!   shown stays. An uncounted slot (counted 1) does not animate and draws
//!   nothing.
//! - **Stock**: spöb 514 "Link" (sÿst 472, graphic 59, 32 frames,
//!   `AnimDelay` 0, `Flags2` 0x2000) now plays a frame a tick, as in the
//!   original; 24 stock stellars animate, all on the Wormhole graphic. No
//!   stock `spöb` sets 0x0001 or 0x0002.
//! - **Readings.** No key. The Bible's `AnimDelay` ("frame delay in 30ths
//!   of a second") and `Frame0Bias` ("display-time multiplier for the
//!   first frame") give no reading for a delay of 0 or less, and its
//!   0x0001 ("shows frame 0 after each frame") and 0x0002 ("picks a random
//!   next frame") agree with the original. The engine's old `AnimDelay > 0`
//!   gate was a gap ("Not switched" under phase 1), not a reading.
//! - **Left out**:
//!   - the hypergate branch (@0x2e83c-0x2ec46), already built whole by
//!     system-traffic-fidelity phase 7 (`nova_sim::gate_sprite`);
//!   - the sprite-slot plumbing (@0x2e713-0x2e7e5, @0x2ee35-0x2ef1a),
//!     the view's drawing from phases 1 and 8: `_HandleSpriteMurkiness`
//!     (@0x2eea4) is rdm:task/murk-other-sprites, and whole-pixel
//!     placement (@0x2eeab-0x2ef1a) is
//!     rdm:phase/stellar-fidelity-followups/phase-9-whole-pixel-sprite-placement;
//!   - the hang under 0x0001 | 0x0002 over 2 counted frames
//!     (@0x2ece5-0x2ed1e), an original bug: the next frame stays, as
//!     `nova_sim::gate_sprite` keeps a gate's;
//!   - the integer widths: the 16-bit `Frame0Bias` × `AnimDelay` (`imul ax`
//!     @0x2ec62, so 200 × 200 wraps to −25536), `_Rand`'s 16-bit count
//!     sign-extended from +0x474 (@0x2eced), and the 16-bit wraps of
//!     next + 1 (@0x2ed2e, @0x2eddc);
//!   - the order of `_Rand`'s draws and its generator;
//!   - the dock request's take-off reset, outside this function
//!     (`_HandlePlayerDockRequest` @0x675ce-0x67653):
//!     rdm:phase/stellar-fidelity-followups/phase-10-takeoff-stellar-frame-reset.
//!
//! ## Checked for stellar-fidelity phase 10
//!
//! Read with `objdump --x86-asm-syntax=intel -d` on
//! `EV Nova.app/Contents/MacOS/EV Nova`. The per-stellar fields are as "Checked for stellar-fidelity phase 1"
//! records: `Flags2` +0x34, `AnimDelay` +0x470, `Frame0Bias` +0x472, the
//! counted count +0x474, the current frame +0x476, the next +0x478 and
//! the float timer +0x490.
//!
//! - **Before the branch** nothing tests `AnimDelay`: the 0x0080 test
//!   (@0x2e77b-0x2e7bb), the count above 1 (@0x2e802), the timer gaining
//!   `_gSpeedMult` (@0x2e810-0x2e825) and the hypergate test, the byte
//!   +0x35 & 0x10 (@0x2e832), whose branch runs @0x2e83c-0x2ec46. Every
//!   other stellar, a wormhole included, takes @0x2ec4b.
//! - **The threshold** (@0x2ec4b-0x2ec83): `Frame0Bias` × `AnimDelay`
//!   when the current frame is 0 and the bias above 1 (`imul ax`
//!   @0x2ec62), else `AnimDelay`; then `timer >= threshold` as floats.
//!   Below it the tick jumps to @0x2ee35, skipping
//!   `_SWSetCurrentFrameIndex`.
//! - **`AnimDelay` ≤ 0.** The loader copies it raw (@0x7761b) and
//!   `Frame0Bias` too (@0x77631). The timer is at least 0 before the add
//!   and `_gSpeedMult` is positive, so a threshold of 0 or less is met
//!   every frame, the product of a bias with such a delay included.
//! - **At the threshold** the timer is set to 0 (@0x2ec89). With 0x0001
//!   (@0x2ec9b-0x2ec9f): a current frame other than 0 becomes 0, the next
//!   untouched (@0x2eca5-0x2ecad, @0x2ed71); frame 0 takes the next
//!   (@0x2ecb3-0x2ecba), and the next is drawn under 0x0002, `_Rand(count)`
//!   again while it is 0 or the new current (@0x2ece5-0x2ed1e), or else
//!   is (next + 1) mod count, 1 when that gives 0 (@0x2ed2e-0x2ed63).
//!   Without 0x0001 the current takes the next (@0x2ed7f-0x2ed86), and
//!   the next is drawn under 0x0002, again while it is the new current
//!   (@0x2ed95-0x2edda), or else is (next + 1) mod count
//!   (@0x2eddc-0x2edf4). Then `_SWSetCurrentFrameIndex(slot, current)`
//!   (@0x2ee05-0x2ee30). `count` is always +0x474.
//! - **The draw**: `_Rand` (@0xa4c76), uniform over 0 to n − 1 (as
//!   `nova_sim::glow` records), with the counted count. Under both flags
//!   with 2 counted frames no frame can be picked, and the loop never
//!   ends: the game hangs. With 3 or more there is always one.
//! - **Frozen**: `_SWSetCurrentFrameIndex` refuses a slot whose frames
//!   count none (@0xa90e7-0xa90ee), but the branch runs anyway.
//! - **Sound**: none. The function's calls (0x2e6f1-0x2ef40) are
//!   `_StellarIsDestroyed`, `_LoadMasterStellarSprite`,
//!   `_SWCloneFramesFromMaster`, `_SWSetSpriteVisible`, `_IsDisabled`,
//!   `_AIShipIsEmergingFromHypergate`, `_AIShipIsShipGoingToStellar`,
//!   `_GetSpriteXSize`/`YSize`, `_FloatAbs`, `_Rand` ×4,
//!   `_SWSetCurrentFrameIndex`, `_HandleSpriteMurkiness` and
//!   `_SWSetSpriteLocation`: no `PlaySound` or `_beepSnd`.
//! - **The take-off reset** (`_HandlePlayerDockRequest`): @0x675ce-0x675fa
//!   zeroes every `spöb`'s current and next frame (all 2048 records, 0x498
//!   apart, to 0x24c000); when the byte at ebp−0x2a3 is set, likely the
//!   gate exit whose escort loop runs @0x674ef-0x675c8, @0x675fc-0x67646
//!   sets the stellar at ebp−0x298's current and next frames to count − 1
//!   and its timer to −`AnimDelay`; then `_HandleStellarSystemVisibility`
//!   rebuilds (@0x67653). Filed as
//!   rdm:phase/stellar-fidelity-followups/phase-10-takeoff-stellar-frame-reset.
//!
//! ## Built by stellar-fidelity phase 12
//!
//! A stellar's animation state (its current frame, next frame and timer)
//! outlives the system it is shown in, as the original keeps it on the
//! `spöb` record (+0x476, +0x478, +0x490), not on the sprite slot.
//!
//! - **Ordinary stellars, in the view.** The flight keeps each stellar's
//!   frames as its scene is let go, on a jump, a move or a rebuild that
//!   changes the stellars shown (`nova_view::system::KeptFrames`), and the
//!   next scene to show it resumes them on the sheet it now draws: the
//!   current frame modulo that sheet's count, or frame 0 on graphic type
//!   0's (`SystemScene::resume`, as the rebuild sets the slot @0x33096,
//!   @0x332d0). A stellar not shown stands still. One the flight has never
//!   shown starts at current 0, next 1, timer 0. The frame a non-gate
//!   stellar shows is drawn state the session never reads, and the state
//!   is not saved, so the view owns it: it settles as the scene is laid
//!   out, before the first frame drawn there.
//! - **Hypergates, in the session.** A gate keeps its sprite (the same
//!   three fields) in `Space`'s gate sprites while it is not shown, and
//!   stands still there; it starts closed only the first time a flight
//!   shows it (see the `nova_sim` `gate_sprites` module).
//! - **Load and new pilot.** Each is a new flight with a new `Space`, so
//!   every stellar and gate starts afresh, as the original does for a load
//!   right after launch.
//! - **Readings.** No key. The Bible says nothing of the animation's
//!   lifetime, and no other reading exists.
//! - **Left out**:
//!   - the new-slot reset (`_HandleStellarSystemVisibility`
//!     @0x331da-0x331fb, gated by ebp−0x121 from `_SWCreateEmptySprite`
//!     @0x32f80): whether it fires depends on how many sprite slots
//!     earlier systems in the process made, a memory-layout accident. The
//!     visible case, a stellar first shown in a flight, starts at
//!     current 0, next 1, timer 0, as the new slot gives the run's first
//!     system;
//!   - carrying the state over a load or a new pilot in the same run
//!     (`_DoNewPilot` @0x18de5, @0x190f0, @0x1937f; `_LoadPilotData`
//!     @0x751cf, @0x75324): process state leaking from another pilot's
//!     game through the static `spöb` array, not saved and not
//!     reproducible from the save;
//!   - the next frame's first value at launch, a read of uninitialised
//!     memory (+0x478 is never set by `_InitObjects` @0x1c8b9-0x1c902, and
//!     the array comes from an uncleared `_NewPtr` @0x1b7d4): the engine
//!     uses 1, as the new-slot reset gives;
//!   - the dock request's take-off reset (`_HandlePlayerDockRequest`
//!     @0x675ce-0x67646), already
//!     rdm:phase/stellar-fidelity-followups/phase-10-takeoff-stellar-frame-reset;
//!   - the integer widths: +0x476/+0x478 are 16-bit and the timer a 32-bit
//!     float;
//!   - the galaxy viewer (`nova_view::system::SystemView`), which still
//!     lays each system out afresh: it has no counterpart in the original.
//!
//! ## Checked for stellar-fidelity phase 12
//!
//! Read with `objdump --x86-asm-syntax=att -d` on
//! `EV Nova.app/Contents/MacOS/EV Nova`. Every instruction writing +0x476
//! (current frame), +0x478 (next frame) or +0x490 (float timer):
//!
//! - **`_InitObjects`** (@0x1c2e5): +0x476 = 0 (@0x1c8c5) and +0x490 = 0
//!   (@0x1c8d1), beside +0x474 = 1 (@0x1c8b9) and +0x472 = 1 (@0x1c8f6);
//!   +0x478 is not written. Its only caller is `_InitEVNova` (@0x47252),
//!   once per launch, before `_LoadObjectData` (@0x47280). The `spöb`
//!   array comes from `_NewPtr(0x24c000)` in `_InitArrays`
//!   (@0x1b7cd-0x1b7e1), which does not clear it.
//! - **`_HandleStellarSprites`** (@0x2e6f1): the timer gains
//!   `_gSpeedMult` (@0x2e825); the hypergate branch writes all three
//!   (@0x2ea3d-0x2ec3f); the sequential branch too (@0x2ec89-0x2edf4).
//! - **`_HandleStellarSystemVisibility`** (@0x32aa5), the new-slot reset:
//!   +0x490 = 0, +0x476 = 0, +0x478 = 1 (@0x331e2, @0x331ef, @0x331fb),
//!   only when the byte at ebp−0x121 is set, which happens only when slot
//!   k's pointer in the array behind 0x3b53d0 was null and
//!   `_SWCreateEmptySprite` ran (@0x32f6b-0x32f85, and @0x32faa when the
//!   create failed). Slot pointers are written only by `_InitSprites`
//!   (once, from `_InitEVNova` @0x47382), by this create, and at quit
//!   (`_DestroySpriteWorld`, `_Bye`). The rebuild's clear keeps the slot
//!   (it nulls +0x4c/+0x50/+0xd4 and sets +0xd0 to −1, @0x32e68-0x32e8f),
//!   so each slot index is made once per run: the reset hits every
//!   stellar of the first system after launch, and later only the
//!   stellars past the largest count seen so far, in `spöb` index order.
//! - **`_HandlePlayerDockRequest`** (@0x66691): the take-off reset, every
//!   record's +0x476/+0x478 = 0 (@0x675d8, @0x675e4, looping to 0x24c000
//!   @0x675f4), and the exit gate's three fields (@0x6761e, @0x67630,
//!   @0x67646).
//! - **Nothing else** writes them. The one other match, `-0x490(%ebp)`
//!   @0x9c324, is a local in `_MungeBriefing`. Reads come only from the
//!   four functions above (54 in `_HandleStellarSprites`, 6 in the dock
//!   request, 4 in the rebuild, 2 in `_InitObjects`).
//! - **New pilot: no reset.** `_DoNewPilot` (@0x18b0a) calls
//!   `_ResetGameData` (@0x18de5; its `spöb` loop @0x1de4e-0x1dea8 writes
//!   only +0x46, +0x3c and +0x47c), `_LoadObjectData` (@0x190f0; none of
//!   the three, and no memset or block copy) and the rebuild (@0x1937f).
//! - **Load: no reset.** `_LoadPilotData` (@0x750b5) rebuilds (@0x751cf,
//!   @0x75324). Its one `memcpy` (@0x757c8, 0x8ec bytes) fills the array
//!   behind 0x3b50f4, not `spöb`.
//! - **Not saved.** No save or load routine reads or writes the three
//!   fields (the reader list above is complete): the state lives for the
//!   process, not the pilot.
//! - **Still while elsewhere.** `_HandleStellarSprites` walks only the 16
//!   sprite slots (@0x2e713-0x2e73e), skipping a null slot, one not in
//!   use (+0x14) and one whose stellar index +0xd0 is outside 0-0x7ff. The
//!   rebuild sets every cleared slot's +0xd0 to −1 (@0x32e8f) and refills
//!   only the stellars shown (+0xd0 = index @0x32fda).
//!   `_DoStellarTimePassage` (@0x40867, the daily walk) touches none of
//!   the three. Shown again, the rebuild sets its slot to +0x476 modulo
//!   the slot's count (@0x33096), and its timer carries on.
//! - **Hypergates**: the gate branch's state is these same three fields,
//!   so a gate keeps its sprite across systems and does not step while
//!   not shown.
//!
//! ## Built by stellar-fidelity phase 13
//!
//! A stellar with a `Weapon` defends itself: each pass, after the ship
//! loop and before the beams, it fires at the nearest ship threatening it,
//! leading it (`nova_sim_combat::combat::stellar_guns`, the session's
//! `stellar_guns` module).
//!
//! - **Which stellars.** Each the player's system lists, in `NavDefs`
//!   order, that is active (its holder visible), not destroyed and armed
//!   with a `wëap` the session holds. A stellar another visible system
//!   holds still fires, unshown, as the original tests no owner.
//! - **Who is a threat.** Nobody, to a dominated stellar. An NPC jumping
//!   in is none. A `Flags2` 0x0200 stellar waits for a defence ship of its
//!   own, which the engine never spawns, so the four stock ones never
//!   fire. Any other NPC is a threat when it is an enemy of the stellar's
//!   own government. The player and the ships it leads directly are
//!   judged by the player's record in the system (by `LegalRecords`),
//!   against the `CrimeTol` and xenophobia of the stellar's government or
//!   else its system's, and by whether the player's class government is
//!   an enemy of the stellar's own; the stellar's own government's peace
//!   blessing, the effective government's IFF mark and the player's dock
//!   request at it spare them.
//! - **The shot.** The nearest threat in range, not cloaked, and not
//!   disabled for a disable-only or massless weapon, is fired at from the
//!   stellar's centre at the lead angle, plus the inaccuracy; the shot is
//!   no ship's (source -1), so it hits only its target and its hits are
//!   no one's crime. Its reload is the weapon's, or its burst reload at a
//!   burst's end. Each shot is heard from the stellar at priority 5,
//!   looped or not.
//! - **The timers** outlive the system, are zeroed for every active
//!   stellar each day and are never saved.
//! - **Readings.** No key. The routine's oddities (escorts weighed by the
//!   player's standing, the blessing read from the stellar's own
//!   government alone, the class test against the stellar's own
//!   government) are the original's, and no Bible text gives another
//!   reading. The record goes through `LegalRecords` and the full shot
//!   table through `CombatCaps`, as everywhere else. `PdHostility`'s other
//!   reading now calls a shot no ship launched hostile unasked, as its
//!   firer is only a placeholder.
//!
//! ## Checked for stellar-fidelity phase 13
//!
//! Read in `EV Nova.app/Contents/MacOS/EV Nova` (i386). Layouts: spöb
//! stride 0x498 (`*0x3b5010`), ship 0xc940 (`*0x3b5138`, slot 0 the
//! player), wëap 0xc8, gövt 0x88, sÿst 0x1fc, shot 0x48 × 128
//! (`*0x3b5474`).
//!
//! - **Where it runs.** `_DoPlayGameWork` calls `_HandleStellarWeapons`
//!   once, @0x444d7, after `_HandleStellarSprites` @0x444d2 and before
//!   `_HandleAsteroids` @0x444f5, `_HandleGravity` @0x44509 and
//!   `_HandleBeams` @0x4453d, on both passes. Landed, nothing calls it.
//! - **`_HandleStellarWeapons`** (@0x2ef3a-0x2f5d1):
//!   - returns at once when `*0x3b523c` is set (@0x2ef46), which only
//!     `_PrepareAllSprites` sets, never around this call;
//!   - walks the player's system's 16 `NavDefs` (@0x2ef7e-0x2efdb),
//!     skipping -1, a stellar not active (+0x44, set @0x32c42/@0x32cdf),
//!     one with no weapon (+0x2c = -1; `_LoadObjectData`
//!     @0x773f3-0x777b2 makes a `Weapon` of 127 or less -1, and any other
//!     the index ID − 128 with no upper bound) and a destroyed one
//!     (`_StellarIsDestroyed` @0x2efd4);
//!   - counts the float reload +0x494 down by `_gSpeedMult` while it is
//!     above 0, doing nothing else (@0x2efee-0x2eff9, @0x2f5a1-0x2f5b2);
//!   - picks the nearest ship of slots 0-63 that is active, not cloaked
//!     (@0x2f074), a threat (@0x2f096), within the weapon's range +0x5c
//!     squared in f32 (@0x2f0b3-0x2f0c4) and, for a disable-only
//!     (`Flags2` 0x1000) or massless (`MassDmg` 0) weapon
//!     (@0x2f01e-0x2f033), not disabled (@0x2f0d6); a new best needs a
//!     strictly smaller distance (@0x2f003-0x2f11a);
//!   - takes the first shot slot of life below -1.0 (@0x2f120-0x2f151),
//!     or skips the stellar with none;
//!   - builds the shot (@0x2f157-0x2f2ed): at the stellar's whole-pixel
//!     place, at rest, source -1 (+0x2c), aimed at the target (+0x2a),
//!     disable-only by the weapon (+0x44, @0x2f1f1), life `Count`, the
//!     jam +0x30 and +0x24/+0x32/+0x36 zeroed, the spin timer +0x1c zeroed
//!     (@0x2f26d), the spin start by `Flags` 0x0004, the decay seed, the
//!     system and durability max(`Durability`, 0); no miss roll (+0x42)
//!     and no sensor-interference roll;
//!   - aims it at `_CalcSpobLeadAngle` plus `Rand(2i) − i` for an
//!     `Inaccuracy` above 0, wrapped once (@0x2f2f1-0x2f3c9), and sends it
//!     at `_Accel(trunc(heading), Speed)` from rest (@0x2f3ce-0x2f3fe);
//!   - rolls `Rand(JamVuln + 1)` per type above 0 (@0x2f40c-0x2f454);
//!   - puts it in the shot layer whatever its guidance (@0x2f46f);
//!   - sets the reload to `trunc(Reload)`, or, once a bursting weapon's
//!     count +0x2e reaches `_WeaponBurstInterval(weapon, 1)` (@0x831c), to
//!     `BurstReload`, the count back to 0 (@0x2f4be-0x2f545);
//!   - plays `_PlaySoundDistance(-1, snd, 5, stellar, player)`
//!     (@0x2f553-0x2f59a), with no looped test.
//!   - +0x494 and +0x2e are zeroed for every active stellar by the daily
//!     step (`_IncrementGameTime` @0xb5df, @0xb5ed) and `_InitObjects`
//!     (@0x1c902, @0x1c90f), and never saved.
//! - **`_IsThreatToStellar(ship, stellar)`** (@0x9ef7-0xa0ae), in order:
//!   a dominated stellar (+0x46) sees no threat (@0x9f0c); an NPC jumping
//!   in (`_AIIsShipJumpingIn` @0x82f12, goal 8) is none (@0x9f16-0x9f2a);
//!   for `Flags2` 0x0200 (@0x9f30) only the player and the ships it leads
//!   directly are threats, and only while +0x47 is set (@0x9f36-0x9f5c),
//!   which `_DoPlayGameWork` sets each frame for the stellar an active NPC
//!   defends (ship +0x8c, @0x443d4-0x4444f); any other NPC is a threat by
//!   `_GovtEnemies(stellar Govt, ship govt)` (@0x9f61-0x9f73 → @0xa080);
//!   the player's side by `G` = the stellar's government or else the
//!   system's (none: no threat), `_playerRecord[system]` (@0x9fb8) below
//!   −`CrimeTol(G)` (@0x9fe3-0x9feb), else for a xenophobic `G` below 0
//!   (@0x9fed-0x9ff8), else `_GovtEnemies(class +0x9f4, stellar Govt)`
//!   (@0x9ffa-0xa028); spared by the inlined `_PlayerBlessedByGovt(own,
//!   0)` (@0xa02e-0xa04a, 0 for -1, so a system government's blessing
//!   never spares), `G`'s +0x83 (@0xa05f-0xa06a) or `_dockTarget` being
//!   the stellar (@0xa06e-0xa07a). No sound, no `_Rand`.
//! - **`_CalcSpobLeadAngle(stellar, ship)`** (@0x2b2f0-0x2b42e): the
//!   bearing for a stellar with no weapon, else the table angle to where
//!   the ship will be after d / speed in f32, a rocket's timing by
//!   `_CalcLeadAngle`'s doubles (0xdd540-0xdd558), with no firer velocity
//!   and no guidance gate. No sound, no `_Rand`.
//! - **Stock.** 95 `spöb` carry a `Weapon` of 128 or more (196 ×15, 197
//!   ×27, 159 ×22, 199 ×21, 198 ×9, 131 ×1); 7 more carry 0, none. Earth
//!   128, Port Kane 137, Pyrogenesis I 161 and New Ireland 508 have
//!   `Flags2` 0x0200. None is disable-only, planet-type or bursting; the
//!   Enormous Ion Turret (198) is massless.
//! - **Left out**, each with its owner:
//!   - +0x47, the defence ship in the system (@0x9f30-0x9f5c; set
//!     @0x44411-0x4444f, @0x3d4a8): the engine spawns no stellar defence
//!     ships, so it is always clear and the four stock 0x0200 stellars
//!     never fire; the gate is built and fed false. Task
//!     `planet-domination`.
//!   - +0x46, domination by a hail (@0x9f0c): the engine has only
//!     `Flags2` 0x0020, which is fed. Task `planet-domination`.
//!   - a `Weapon` past the last `wëap` (@0x777a9): a read past the table,
//!     a memory-layout accident; the engine gives that stellar no gun.
//!   - the frozen-flag test `*0x3b523c` (@0x2ef46): never set when the
//!     routine runs.
//!   - the player's own cloak: none until task `cloaking-device`, then it
//!     applies with no change here.
//!   - shot fields with no engine state: the decay seed +0x18, the system
//!     +0x2e, the zeroed +0x24/+0x32/+0x36, the collide proc and frame
//!     clone (@0x2f487, @0x2f4b9); the spin start +0x34 is the view's.
//!
//! ## Built by stellar-fidelity phase 14
//!
//! The nearest-stellar pick is `_FindNearestStellar` whole
//! (`nova_sim_state::landing::nearest_stellar`; the session's
//! `stellar_picks` module).
//!
//! - **The gate limit.** By the engine's reading of
//!   [`gate_pick_range`](crate::RuleKey::GatePickRange), a hypergate or
//!   wormhole counts only when its squared distance from the ship is
//!   strictly below 250000.0, 500 pixels; by the other reading, the
//!   port's earlier pick, at any distance. Ordinary stellars count at any
//!   distance. Distances are compared squared. Every caller reads the key:
//!   the land key, F5, the tutorial's land hint and a new pilot's saved
//!   stellar.
//! - **The candidates.** The land key and F5 pick among every stellar
//!   the system lists that is active (`nova_sim::Session`'s
//!   `Chart::pick_candidates`), shown there or not: a stellar another
//!   visible system owns is one. The land key selects it, starting its
//!   fly-in, and its dock request refuses it as not landable with its
//!   line and `snd ` 153; held, a second L is refused again. F5 drops it:
//!   nothing written, no sound, no fall back to the next nearest.
//! - **The selection.** A stellar selected is one of those candidates,
//!   not only one the system shows. The gate kind
//!   (`nova_sim::Session::gate_kind`) and the nav brackets read it from
//!   the stellars listed; the brackets carry its spöb position.
//! - **The view.** The brackets of a selection the scene doesn't hold are
//!   drawn at its spöb position at half 64, and the not-landable line
//!   names it by its spöb name, as the nav area already does.
//!
//! ## Checked for stellar-fidelity phase 14
//!
//! Read in `EV Nova.app/Contents/MacOS/EV Nova` (i386). Floats in
//! `__TEXT,__const`: 0xdd04c = -1.0, 0xdd050 = 250000.0, 0xdd054 = 0.0.
//!
//! - **`_FindNearestStellar(ship)`** (@0x276c-0x2888), whole. It plays no
//!   sound and writes only its locals.
//!   - F1: the best distance starts at -1.0 and the best ID at -1
//!     (@0x2777-0x2785).
//!   - F2: it walks i = 0..15 over the ship's system's `NavDefs`
//!     (`sÿst` +0x2a + 2i, @0x278a-0x27a2, loop @0x2873-0x2877), passing
//!     over an empty slot (@0x27a6-0x27aa).
//!   - F3: a stellar whose +0x44 is clear is passed over
//!     (@0x27c3-0x27c7).
//!   - F4: so is one without `Flags` 0x01 (@0x27cd-0x27d1). Destroyed and
//!     `Flags` 0x80 are not read.
//!   - F5: nothing reads the owner +0x14.
//!   - F6: dx and dy are `_FloatAbs` of the ship's float position less
//!     the spöb's 16-bit one (@0x27d7-0x281a), and d² = dx·dx + dy·dy in
//!     single precision (@0x2825-0x2844).
//!   - F7: `Flags2` & 0x3000 (`movzwl 0x34`, `testb $0x30,%ah`,
//!     @0x2838-0x2841) counts only when `ucomiss 0xdd050` finds d²
//!     strictly below 250000.0 (`jae` and `jp` pass it over,
//!     @0x284a-0x2853); any other stellar counts (`je 0x2855` @0x2848).
//!   - F8: it becomes the best when the best is strictly above d² (`ja`
//!     @0x285a-0x285d), or still below 0.0 (@0x285f-0x2868): equally
//!     near ones go to the earlier slot.
//!   - F9: it returns the best ID, or -1 (@0x287d).
//!   - A first counted stellar at a NaN distance would stick as the best
//!     (@0x285a-0x2868); positions are never NaN, so it is not modelled.
//!   - The spöb coordinates are 16-bit and the ID is returned as a short;
//!     the engine keeps `f32` positions and `i16` IDs.
//! - **Its callers**: `_DoNewPilot` @0x1926e, F5 @0x69e59, the land key
//!   @0x6a901, the tutorial hints @0x709e8 and @0x70bdc.
//! - **The land key** (@0x6a8ed-0x6a932): with mode +0x6a ≠ 2 or no
//!   selection it picks for the player; a pick writes mode 2, +0x6c,
//!   `_statusRedraw` and `_navTargetExpand` := 256.0, with no sound and no
//!   owner test. Then `_HandlePlayerDockRequest` (@0x6a938-0x6a956)
//!   refuses an unowned pick (@0x6699a-0x669a4) with the not-landable
//!   line (0x67cda) and `snd ` 153 (@0x67e8f).
//! - **F5 and the slot keys' shared tail** (@0x69e1f-0x69f12): F5 is key
//!   index 0x10 and picks for the player (@0x69e4f-0x69e5e); -1 does
//!   nothing (@0x69e65-0x69e6d); a pick whose +0x14 isn't the player's
//!   +0x74 leaves with nothing written and no sound (`jne 0x69dea`,
//!   @0x69e73-0x69e94); otherwise `snd ` 150, then, unless already
//!   selected in mode 2, the fly-in, +0x6c, `_statusRedraw`, mode 2 and
//!   the dock request dropped (@0x69e9a-0x69f12).
//! - **What a selection outside the stellars shown meets.**
//!   `_HandleNavTargetSprites` (@0x2fcaf-0x2fe41) brackets it at the spöb
//!   position (@0x2fd9f-0x2fdda) at half 64 when no visible slot holds it
//!   (@0x2fde7-0x2fe41). `_DrawStatusNav` names any selected stellar by
//!   its spöb name (@0x4a0fd-0x4a149). The not-landable line is `STR#`
//!   2002 #84 then #86 (wormhole) or #85 (hypergate) with no name, else
//!   #87/#88 and the name (@0x67cda-0x67dc4). Hailing
//!   (`_HandlePlayerCommunication` @0x61fac-0x62009) has no owner test;
//!   ours gives "No response.", unreachable in stock (wormhole 466 never
//!   answers): rdm:phase/stellar-fidelity-followups/phase-5-nearest-and-slot-picks-over-navdefs.
//! - **The other readers of the selection**, audited: the hail looks it
//!   up among the stellars shown (the divergence above); the person
//!   offers and the map cycle read its ID only; the face key falls back
//!   to the stellar's spöb place (`StellarMarks`); the dock request's
//!   wording and the landing read it only once it is owned, which the
//!   refusal stops first; the nav area names it by its spöb name. None
//!   panics or misbehaves.
//! - **Wormholes stay shown**: `_DrawStatusRadar` @0x498a0-0x49a8e,
//!   `_HandleStellarSprites` @0x2e713-0x2e74f and `_DrawStatusNav`
//!   @0x4a09c-0x4a18c have no gate test.
//! - **Stock**: only wormhole 466 (`Flags` 0x21, `Flags2` 0x2200) is
//!   listed by a system that doesn't own it: Spica (144) owns it, and
//!   K-005 (436, visible while `!b330`) lists it at (9500, 2600), its only
//!   stellar.
//! - **Left out**, in
//!   rdm:phase/stellar-fidelity-followups/phase-5-nearest-and-slot-picks-over-navdefs:
//!   the slot keys by `NavDefs` index (@0x69e28-0x69e46); the tutorial
//!   hints' candidates (@0x709e8, @0x70bdc) and `_DoNewPilot`'s
//!   (@0x1926e), which get the gate limit but still read the stellars
//!   shown; hailing an unowned selection.
//!
//! ## Built by stellar-fidelity phase 15
//!
//! A stellar's landing `Fee` (`nova_sim::LandingSite::fee`, raw from its
//! `spöb`):
//!
//! - **When it is due.** `nova_sim_state::landing::fee_due`: a `Fee`
//!   above 0 at a stellar not dominated. A `Fee` of 0 or below is no fee.
//! - **The refusal.** The land key refuses a pilot whose cash is below
//!   the fee due with `nova_sim::LandingRefusal::CannotPayFee`, right
//!   after the not-landable check and before the disabled ship's press,
//!   the uninhabited stellar's "No response." and the clearance, so on
//!   the first press too. It is silent and drops the dock request. The
//!   view says "You don't have enough credits to pay the landing fee."
//!   (or the docking or hypergate fee) for 250 ticks.
//! - **The charge.** A press that lands, or enters a hypergate or
//!   wormhole, pays the fee (`nova_sim_state::landing::pay_fee`, never
//!   below 0) before the gate branch; a cancelled hypergate map does not
//!   refund it (the session's `landing_fee` module).
//! - **The greeting.** `nova_sim::SimMessage::DockCleared` carries the fee
//!   due, and the dock approach's greeting ends with "  [Landing fee is
//!   1,000 credits.]" (or "[Docking fee is"), its millions by
//!   [`pay_millions`](crate::RuleKey::PayMillions); a hypergate's greeting
//!   has none.
//!
//! No new Rulebook key: the Bible's `Fee` ("The fee that is deducted
//! from the player's credits when landing on this stellar") says nothing
//! of a refusal, of domination or of gates, so it gives no other reading.
//!
//! ## Checked for stellar-fidelity phase 15
//!
//! Read in `EV Nova.app/Contents/MacOS/EV Nova` (i386),
//! `_HandlePlayerDockRequest` (@0x66691-0x67ef9) and
//! `_HandlePlayerDockApproach`. The runtime `spöb` record: `Flags` +0x30,
//! `Flags2` +0x34 (hypergate 0x1000), `Fee` +0x38 (i32), the domination
//! byte +0x46; the pilot's cash +0xa0 of `*0x3b5138`; `_creditsCString`
//! at 0x184560 ("credits").
//!
//! - **The refusal** (@0x669b4-0x66a80), after the landability tests
//!   (@0x66977-0x669aa) and before the cloak (`_ShipIsCloaked` @0x66a8e),
//!   the disabled epilogue (@0x66aee-0x66b14), the uninhabited reply
//!   (@0x66b51), the request match (@0x66bd2), the speed and reach gates
//!   and the clearance (@0x67903). `Fee > 0` (@0x669b7-0x669b9), not
//!   dominated (@0x669bf-0x669c3) and `Fee > cash`, signed
//!   (@0x669d0-0x669d6): cash equal to the fee pays. Text: #61, " ",
//!   `_creditsCString` (always plural, @0x66a0f-0x66a1b), " ", then #62
//!   at a hypergate (@0x66a40-0x66a46), else #63 at a station
//!   (@0x66a50-0x66a56), else #64 (@0x66a60). `_DisplayComm(text, 0xfa,
//!   …)` @0x67c88-0x67cbc, 250 ticks, no sound. `_dockTarget` and
//!   `_dockPermission` := -1 (@0x67cc1-0x67cd0), then the epilogue
//!   (0x67eef); the selection is untouched.
//! - **The charge** (@0x66db9-0x66df5), on the success path only, after
//!   the speed and reach gates and before the branch to
//!   `_PlayerEnterHypergate` (@0x66ed6-0x66edb), `_PlayerEnterWormhole`
//!   (@0x66f0d-0x66f12) or the landing: one charge serves all three
//!   (@0x66dce is its `Fee` read). `Fee > 0` (@0x66dd1), not dominated
//!   (@0x66dd5), cash − `Fee` (@0x66de3), a negative result set to 0
//!   (@0x66deb-0x66df5), unreachable from the press but ported. A
//!   hypergate map cancelled after (@0x66ee0-0x66f08) refunds nothing.
//! - **The greeting** (@0x64d70-0x64e58): after the second sentence,
//!   when `Fee > 0` (@0x64d87-0x64d8c) and not dominated
//!   (@0x64d92-0x64d96): "  " (@0x64da6), #103 at a station
//!   (@0x64dca-0x64dd0) or #104 (@0x64dda), " " (@0x64e00),
//!   `_StrcatCommaNumber(Fee)` (@0x64e21), " " (@0x64e2e),
//!   `_creditsCString` (@0x64e34-0x64e40), #105 (@0x64e45-0x64e58), still
//!   shown 250 ticks (@0x64e8e). A hypergate's greeting jumps past it
//!   (@0x64a59 → 0x64e5d), so #102 "[Hypergate fee is" (@0x64dba-0x64dc0)
//!   is unreachable and not built; a wormhole gets #103/#104.
//! - **Left out**: domination by Demand Tribute (+0x46 written by
//!   `_DoPlanetCommDialog` @0x978fa, cleared @0x9740f), which waives the
//!   refusal, the charge and the greeting line, is
//!   rdm:task/planet-domination's (`fee_due` is the one reader to switch);
//!   the 32-bit cash width and signed compare are not replicated; the
//!   `crashlog_print` calls after the charge (@0x66dff-0x66e56) are debug
//!   logging.
//! - **Merge note** (cloaking-fidelity phase 8, on `main` but not on this
//!   branch's base): `main`'s `land_press` returns
//!   `LandingRefusal::Cloaked` for any outcome but `NotLandable`,
//!   `NoStellars` and `NothingLandable` while cloaked. The fee refusal
//!   (@0x669b4) comes before the cloak (@0x66a8e), so when `main` is
//!   merged in, `CannotPayFee` must join that pass-through arm, with the
//!   dock request dropped for it, and a test that a cloaked ship which
//!   can't pay hears the fee refusal, silently
//!   (rdm:phase/stellar-fidelity-followups/phase-12-cannot-pay-fee-before-cloak).

//! # Mission lifecycle
//!
//! Built by missions phase 2 (see the [`mission`](crate::mission)
//! module's "Active missions" for the original's routines).
//!
//! A mission is offered (per visit), then active, in one of 16 slots
//! saved on `Pilot` (save version 20), until it succeeds, fails or is
//! aborted. What moves it:
//!
//! - the player, through `nova_sim::Session`:
//!   `nova_sim::Session::accept_mission` runs the accept
//!   checks, then fills the lowest free slot from the offer, registers
//!   its hooks and runs `OnAccept`, and ends an auto-aborted mission at
//!   once, its `DatePostInc` days passed through the daily step;
//!   `nova_sim::Session::refuse_mission` runs `OnRefuse`
//!   and drops the offer; `nova_sim::Session::abort_mission`
//!   runs the abort checks (`CanAbort`), then `OnAbort`;
//! - `Landed`: each slot's four landing subscriptions,
//!   `{Missions, slot, 0..=3}`, after the offer roll, in
//!   `_MissionLandCheck`'s order: by slot, the goal check, the arrival
//!   (`_MissionLandCargoCheck`: the cargo picked up and dropped off, and
//!   the destination reached), the return (failure, else an escort goal
//!   with none of its ships destroyed met, then success), and the goal
//!   check again. A failure runs `OnFailure` and applies the failure
//!   records; a success frees the slot, runs `OnSuccess`, passes its
//!   `DatePostInc` days through the daily step, applies the legal records
//!   and pays. The re-offer, `{Missions, 16, 0}`, runs after every slot
//!   and offers the missions again, the table kept, when one succeeded;
//! - the daily step: the days left count down (the third participant);
//!   a timed slot's `DayPassed` subscription, `{Missions, slot, 4}`, fails
//!   a mission whose days left reached 0, running `OnFailure`, once the
//!   ship is not docked, as the original's per-frame check skips the open
//!   spaceport. Its `TookOff` subscription, `{Missions, slot, 5}`,
//!   catches a deadline that ran out docked when the take-off passes no
//!   day. An unabortable mission stays active and failed until its
//!   return, where [`repeat_mission_failure`](crate::RuleKey::RepeatMissionFailure)
//!   says whether `OnFailure` runs again;
//! - `ShipFate`: the mission ships' goal checks (mission-ships phase 2;
//!   see "Ship goals"), each slot's `{MissionShips, slot, 2}`. The goals
//!   met show `ShipDone`, run `OnShipDone`, and auto-abort a mission of
//!   `Flags` 0x0001, with its `OnAbort`, its `DatePostInc` days and its
//!   pay or fuel as an auto-abort at accept does (as do the system's
//!   setup and the appear timer's arrivals, once they bring its ships); a check that fails a
//!   disable or escort goal marks the mission failed, nothing run, to
//!   fail at its return. A mission ship's loss that fails its mission
//!   fails it at once, at the loss, through the quick failure; cargo
//!   jettisoned (`nova_sim::Session::jettison_cargo`)
//!   or found by an interceptor's scan does the same, through the one
//!   quick failure the deadline uses; cargo plundered waits for NPC
//!   boarding;
//! - `S`, `A` and `F`, through `apply_op`: `S` accepts, running `OnAccept`
//!   there as the original does, a mission active already taking another
//!   slot as [`start_active_mission`](crate::RuleKey::StartActiveMission)
//!   says; `A` aborts there, running `OnAbort`; `F` marks the mission
//!   failed, and it fails at the next landing at its return.
//!
//! The scripts a transition runs (`OnAccept`, `OnAbort`, `OnSuccess`,
//! `OnFailure`) run inline, as `nova_sim::Session::run_set`
//! runs one, and the events they raise (an auto-abort's or a success's
//! `DayPassed`) wait for the nearest nesting point: the dispatcher's right
//! after the hook's effect, or the one after an `S` or `A` operator. So
//! under `story_reentry`'s nested default they dispatch before the
//! script's next operator, not between the transition's own operators.
//!
//! Every end (a success, a failure, `A`, an abort or an auto-abort) frees
//! the slot and unregisters its hooks, clearing their once-fired entries,
//! so a mission accepted into the reused slot starts fresh, and forgets
//! the stellar the offers were rolled at (`MissionRun::forget_offers`).
//! `EnteredSystem` moves no mission: the original's enter-system hook only
//! restarts its ships' timers (mission-ships phase 1). Offers are rolled
//! as the player lands, before the spaceport opens, and kept until a
//! landing at another stellar, a jump or a mission's end (missions
//! phase 1, [`mission`](crate::mission)). A landed `M` or `N` empties
//! them, the stellar kept. None of the offers is saved.
//!
//! # Text substitution
//!
//! Built by missions phase 3.
//!
//! [`text_tags`](crate::text_tags), which missions phase 3 extracted
//! from [`expand_tags`](crate::person::expand_tags), is the one
//! substitution path, as `_MungeBriefing` is the original's: a text is
//! filled for an offer, an active slot or no mission
//! (`nova_sim::Session::offer_text`,
//! `nova_sim::Session::mission_text`,
//! `nova_sim::Session::plain_text`). Its users:
//!
//! - mission text;
//! - a `Q` message run from a mission's script;
//! - person hail quotes, with no mission
//!   ([`expand_tags`](crate::person::expand_tags) wraps it);
//! - mission ships' `ShipDone` (mission-ships phase 2);
//! - news (crons-news-ranks phase 2), which the original fills with no
//!   tags: a text with none comes out unchanged;
//! - dialogue nodes (plugin-scripting-extensions phase 3).
//!
//! **Text sources** (missions-fidelity phase 2). The crossed rank tags
//! read the governments the session remembers
//! ([`RankTagGovts`](crate::text_tags::RankTagGovts), the original's
//! `_descRankGovt`), which a text changes only as it is shown, by where
//! it comes from: a `dësc` the original loads through `_LoadDescription`
//! (an offer, the lifecycle texts, the cargo and ship-done texts, the
//! mission computer's or mission info's selected row) resets them and
//! parses its own; a hail quote parses its own onto them; anything else
//! (the screens' lists of names, a `Q` message) reads them as they stand.
//! The `&self` texts and screens read them and change nothing; the
//! `&mut` paths that show a text take it in first, so the governments
//! settle as each text is shown, as the original's do.
//!
//! **Conditional text** (missions-fidelity phase 26). A text taken in
//! has its conditional text resolved first
//! ([`text_parse`](crate::text_parse), `_ParseTextDescription`): `{G
//! …}`, `{bnnn …}` and `{Pnnn …}` read the pilot as it is at that moment,
//! and the rank scan reads only the strings chosen. The `&self` screens
//! resolve it alike, as if loaded.
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
//!   nor `_ExploreNebulaeFromSystem`: the original's gate explores no
//!   nebula, and writes the explored level itself (@0x63a04-0x63a49). The
//!   engine's gate row explores the system, and since crons-news-ranks
//!   phase 16 raises no `Explored`.
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
//!   an escort of it that has a cloak itself (its parent +0x9a the
//!   target, `_HasCloak` @0x979b; never the player) or to an observer
//!   with a cloak scanner (+0xc91c) within 200 on both axes
//!   (@0x97a4-0x97f8); every other ship is visible. No stock outfit is a
//!   cloak scanner, so the stock player sees a cloaked ship only once it
//!   uncloaks (see [`cloak`](crate::cloak)). `_MissionObjectivesCheck`
//!   (@0x9ea5c) asks it, so a watched ship waiting at the hypergate or
//!   wormhole it comes out of stays watched until it has emerged (see
//!   [`emerge`](crate::traffic::emerge)). A watched ship not yet visible
//!   stays watched.
//! - `S` (`_DoMissionAccept` @0x15407), `A` (`_AbortMission` @0x1532e)
//!   and `M` and `N` in flight (@0x159a3, @0x159a8) are handled inside
//!   `_EvalSetExp`, before its next operator. `Q` only sets
//!   `_absquatulateStr` (@0x15bee); nothing between it and @0x15c6c leaves
//!   the stellar. Landed, the port's filter leaves on it at its next call
//!   (@0x4f047-0x4f063), after the landing's texts and dialogs; the
//!   session holds it to the take-off.
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
//!   of 0x28 at 0x3b52c0, which `_SpawnBox` fills, @0x99a0f-0x99a36:
//!   here [`cargo_box`](crate::cargo_box)'s boxes, which keep no system
//!   and so stay with the player as they are). It
//!   sets up no ships, runs no hook, explores nothing and redraws no
//!   stellars. So every NPC stays with the player, not the escorts alone,
//!   and no `galaxy_escorts` key is needed. It leaves the stellar the
//!   player is docked at alone: run at a landing (@0xa175f, @0xa1af2), the
//!   spaceport goes on at that stellar, and the new system's ships and
//!   stellars are set up only as the take-off sets the system up
//!   (`_SetupShipsInSystem` @0x674c2, the stellars @0x674c7).
//! - **The course** keeps system IDs between rebuild points, and each
//!   rebuild point rewrites it as
//!   [`RuleKey::RouteRewrite`](crate::RuleKey::RouteRewrite) says (see
//!   the `nova_sim::course` module). `_CueNextHyperRouteDest` resolves
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

pub mod day;
pub mod dispatch;
pub mod galaxy;
pub mod rebuild;
pub mod sprite_pool;
pub mod wreck;

pub use dispatch::DuplicateSubscription;
pub use galaxy::{
    Countdown, GalaxyOverlay, HiddenMove, NebulaOverride, OverlaidGalaxy, StellarOverride,
    SystemOverride, Whereabouts, listers_of,
};
pub use rebuild::DrawnSheet;

use crate::catalog::{StellarId, SystemId};
use crate::control::{Script, Test};
pub use crate::cron::{Consumer, SubscriptionId};
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
    /// First seen by the player: visible to it, which the original polls
    /// each frame (`_ShipVisibleToShip` @0x9ea5c) and the flight tick
    /// raises here (`nova_sim::Session::sees`: not
    /// emerging from a gate, and uncloaked or within a cloak scanner's
    /// sight). Only a ship that can cloak must also be on screen
    /// (@0x9ea74-0x9eb09, `nova_sim::Session::on_screen`).
    Observed,
    /// Brought into the player's system, a mission's special ship set up
    /// with the system or come in on its timer: what the original's
    /// per-frame poll sees of an escort goal's ships (@0x9e931-0x9e980).
    Arrived,
    /// Left the player's system, a mission's special ship: jumped out, as
    /// the original frees its slot at the end of its warp out
    /// (`_LowLevelAIHandler` @0x85960-0x859bf), or landed under
    /// [`RuleKey::LandedLeaves`](crate::RuleKey::LandedLeaves)'s other
    /// reading. Raised as the traffic's tick takes it out, before its
    /// slot counts it.
    Departed,
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
            on(Consumer::Offers, 0, 0, any),
        ]
    }

    fn sorted_ids() -> Vec<SubscriptionId> {
        vec![
            id(Consumer::Offers, 0, 0),
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
    fn a_landing_rolls_the_offers_before_it_checks_the_missions() {
        let landed = EventFilter::Any(StoryEventKind::Landed);
        let subs = [
            on(Consumer::Missions, i16::MIN, 0, landed),
            on(Consumer::Offers, i16::MAX, 9, landed),
        ];
        assert_eq!(
            ids(&dispatch_order(&subs, &LANDED)),
            [
                id(Consumer::Offers, i16::MAX, 9),
                id(Consumer::Missions, i16::MIN, 0)
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
