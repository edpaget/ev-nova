//! Rules where the Nova Bible and the original `EV Nova` engine disagree:
//! each such rule can follow either, by the [`Rulebook`].
//!
//! The project's strategy is that wherever the Bible says one thing and
//! the engine does another, the rule is written both ways and the player
//! chooses. A [`Rulebook`] holds a [`RuleSource`] for every such rule
//! (each key's own default, [`RuleKey::default_source`]: the engine's but
//! for the two cron holdoff keys and the breakup stall, unless chosen
//! otherwise) and an
//! override for any single rule, named by its [`RuleKey`]. A rule asks the rulebook
//! [`Rulebook::source_for`] its key, and the rule's implementation makes
//! the decision. The settings file names the default `"rules"` and the
//! overrides `"rule_overrides"`, by each key's [`RuleKey::key`].
//!
//! The rules here so far:
//!
//! | [`RuleKey`] | settings key | engine | Bible |
//! |---|---|---|---|
//! | [`CrimeGains`](RuleKey::CrimeGains) | `crime_gains` | a crime pleases every government not allied with the victim's ([`NovaLaw`](crate::NovaLaw)) | only the victim's enemies |
//! | [`EmptyBooty`](RuleKey::EmptyBooty) | `empty_booty` | boarding a ship of `Booty` 0 opens the plunder dialog anyway ([`NovaBoarding`](crate::NovaBoarding)) | the player is repelled |
//! | [`CrewlessCapture`](RuleKey::CrewlessCapture) | `crewless_capture` | a player of no crew still has odds of 1 or more ([`NovaBoarding`](crate::NovaBoarding)) | no odds: it cannot capture |
//! | [`PiracyPolice`](RuleKey::PiracyPolice) | `piracy_police` | warships and interceptors answer the player's attack or boarding ([`NovaAi`](crate::NovaAi)) | interceptors only |
//! | [`QuietHails`](RuleKey::QuietHails) | `quiet_hails` | a quiet government's ships (`Flags2` 0x0008) answer Greetings, and Request Assistance and Beg For Mercy do nothing ([`hail::nova`](crate::hail::nova)) | they answer Greetings "No response.", and the middle button works |
//! | [`LongAdvice`](RuleKey::LongAdvice) | `long_advice` | an advice line exactly 42 characters long reads "Nice to meet you." ([`hail::reply`](crate::hail::reply)) | it is shown as written\* |
//! | [`EscortAi`](RuleKey::EscortAi) | `escort_ai` | an escort with no standing command keeps formation and fires its turrets at a threat to the player ([`EscortAi`](crate::EscortAi)) | it flies as its `InherentAI`: a warship or interceptor attacks the threat, a trader keeps formation |
//! | [`EscortOrders`](RuleKey::EscortOrders) | `escort_orders` | entering a system resets every escort's standing order to formation ([`Session`](crate::Session)) | the orders are kept\* |
//! | [`FighterLaunch`](RuleKey::FighterLaunch) | `fighter_launch` | a fighter launched takes its class's standing order and no target, and attacks the player's target on command ([`Session`](crate::Session)) | it attacks the player's target at once\* |
//! | [`FighterRecall`](RuleKey::FighterRecall) | `fighter_recall` | fighters out follow a jump when they hold a jump's fuel, the rest are abandoned, and they stay out while the player is landed ([`Session`](crate::Session)) | every fighter out goes back into its bay on arrival and on landing\* |
//! | [`HireRequire`](RuleKey::HireRequire) | `hire_require` | hiring ignores a ship's `Require`, which hides it only with `Flags3` 0x0200 ([`Session`](crate::Session)) | an unmet `Require` makes a listed ship greyed and refused\* |
//! | [`TakeOffPay`](RuleKey::TakeOffPay) | `take_off_pay` | each take-off pays every hired escort a day's wage ([`Session`](crate::Session)) | wages are paid only for a jump's days; the days a stay passes pay none\* |
//! | [`StayDays`](RuleKey::StayDays) | `stay_days` | leaving the spaceport passes days: half as many as escorts sold or upgraded, a day if an outfit was bought or sold, four if a ship was bought, and the take-off's own day (`_DoEscortLand` @0x40849, `_DoPortDialog` @0x601fa, @0x6020e, `_PlayerLandOnStellar` @0x63440; [`Session`](crate::Session)) | a stay passes no days: only a jump's and a gate's like a jump pass\*\*\*\*\*\* |
//! | [`StoryReentry`](RuleKey::StoryReentry) | `story_reentry` | an event a script raises is dispatched depth first, before the script's next operator (`_EvalSetExp` @0x150fc; `_DoMissionAccept` @0x15407; @0x159a3, @0x159a8; see [`story`](crate::story)) | it waits until the script has settled, then dispatches first in, first out†††††† |
//! | [`DestroyInvincible`](RuleKey::DestroyInvincible) | `destroy_invincible` | `Y` leaves an invincible stellar (`Strength` 0 or -1) as it was, as `_StellarIsDestroyed` holds none destroyed (@0x4e1a; see [`story`](crate::story)) | `Y` destroys any stellar: "Yxxx - destroy stellar ID xxx" |
//! | [`DayRolls`](RuleKey::DayRolls) | `day_rolls` | the shop and hire rolls are drawn once a day in the daily step, and a landing keeps them (`_IncrementGameTime` @0xb7c4-0xb833; see [`story`](crate::story)) | a landing forgets them, and each is drawn the first time a list asks for it‡‡‡‡‡‡ |
//! | [`YOnDestroy`](RuleKey::YOnDestroy) | `y_on_destroy` | `Y` runs no `OnDestroy`: only a shot's destruction does (`_EvalSetExp` @0x15a38-0x15ae6, `_CheckShotProximities` @0x374a9; see [`story`](crate::story)) | `Y` runs `OnDestroy` as it destroys a whole stellar: "evaluated when the stellar is destroyed" |
//! | [`DeadTypeFallback`](RuleKey::DeadTypeFallback) | `dead_type_fallback` | from the next rebuild point until a flight frame sees it whole, a destroyed stellar whose `DeadType` has no readable sheet draws graphic type 0, `spïn` 1000 (`_HandleStellarSystemVisibility` @0x32f0f-0x32f2f, @0x33244-0x332d0; `_HandleStellarSprites` @0x2e77b-0x2e7e5; see [`story`](crate::story)) | it keeps its `Type` sheet throughout: "-1: Don't display different graphic type when destroyed" |
//! | [`LandingDistance`](RuleKey::LandingDistance) | `landing_distance` | the ship lands when it is strictly within trunc(1.75 × the `Type` frame's width) of the stellar's centre on each axis; the width is 150 while the `Type` master is not loaded (a destroyed stellar from a rebuild point until a flight frame loads it again, or a `Type` sheet that cannot be read) and 32 when its `spïn` gives no frame count, and the reach is 75 when the stellar's slot shows no frame (`_HandlePlayerDockRequest` @0x66cc7-0x66cfd, @0x6773a-0x677b2; `_GetStellarSpriteXSize` @0xac91-0xad2d; see [`landing`](crate::landing)) | within half the larger side of the `Type` frame, as a circle, or 32 with no sprite\* |
//! | [`HireFee`](RuleKey::HireFee) | `hire_fee` | a hire takes trunc(cash − 0.1 × price) off the cash, a credit more than the fee shown when the tenth has a fraction ([`NovaHire`](crate::NovaHire)) | exactly the fee shown\* |
//! | [`EscortWage`](RuleKey::EscortWage) | `escort_wage` | a hired escort is paid the wage its ship type's `Cost` gives now ([`Session`](crate::Session)) | the wage kept on its record when hired\* |
//! | [`PersonOdds`](RuleKey::PersonOdds) | `person_odds` | a person comes on `Rand(7)` 0, then `Rand(1022)` landing on one who may appear ([`NovaPersons`](crate::NovaPersons)) | 5 %, one of those who may appear picked evenly |
//! | [`SystemPersons`](RuleKey::SystemPersons) | `system_persons` | a `sÿst` Person slot brings its person on `Rand(100) + 1` at most its chance ([`NovaPersons`](crate::NovaPersons)) | always |
//! | [`LinkSystSlip`](RuleKey::LinkSystSlip) | `link_syst_slip` | a person's system `LinkSyst` also allows the system of ID `LinkSyst` + 128 ([`NovaPersons`](crate::NovaPersons)) | that system only |
//! | [`ShieldMod`](RuleKey::ShieldMod) | `shield_mod` | a person's `ShieldMod` scales its shield and armour ([`NovaPersons`](crate::NovaPersons)) | its shield only |
//! | [`DerelictSetupSpeed`](RuleKey::DerelictSetupSpeed) | `derelict_setup_speed` | a person of a derelict government (`Flags` 0x0800) brought by a setup pass starts at its top speed, as its zeroed velocity is accelerated again (`_SetupShipsInSystem` @0x43245, @0x432eb), and one brought by its Person slot at rest (@0x433fe, then @0x43438) ([`NovaPersons`](crate::NovaPersons)) | both start at rest\* |
//! | [`PersonCoward`](RuleKey::PersonCoward) | `person_coward` | only a warship person of a government with `Flags` 0x0010 runs at `Coward` % ([`NovaAi`](crate::NovaAi)) | any warship or interceptor person |
//! | [`PersonCredits`](RuleKey::PersonCredits) | `person_credits` | a person carries half its `Credits` in thousands, kept in 16 bits, and a draw of as many more ([`NovaBoarding`](crate::NovaBoarding)) | its `Credits`, ±25 % |
//! | [`CommQuote`](RuleKey::CommQuote) | `comm_quote` | a person's comm quote is a friendly person's answer to Greetings ([`Session`](crate::Session)) | it opens the hail, in place of the opening line\* |
//! | [`GrantCount`](RuleKey::GrantCount) | `grant_count` | boarding a person grants trunc((50 + `Rand(51)`) x `GrantCount` / 100) outfits, at least 1 ([`NovaBoarding`](crate::NovaBoarding)) | 1 + `Rand(GrantCount)`\* |
//! | [`GrantMax`](RuleKey::GrantMax) | `grant_max` | a boarding grant may take the player past the outfit's `Max`, and `G` past its `Max` and the free mass ([`OutfitRules`](crate::OutfitRules), which the boarding grant reads too) | both are held to `Max` less the owned and to the free mass, weighed as the outfitter weighs it ([`held_to_max`](crate::grant::held_to_max))\* |
//! | [`PersonJoin`](RuleKey::PersonJoin) | `person_join` | no person offers to join: one whose `LinkMission` is available offers that mission in place of the comm dialog (`_HandlePlayerCommunication` @0x6220d; [`JoinFleet`](crate::hail::nova::JoinFleet)), and an accept with `Flags` 0x0040 and one ship left swaps it for its mission's ship | a person whose record allows it, its `LinkMission` available, lists Use As Escort and joins the fleet as itself, for good\* |
//! | [`MapExplore`](RuleKey::MapExplore) | `map_explore` | a map of `ModVal` N explores depth first, so it may miss systems within N jumps, and "inhabited" reads a system's first four stellars ([`OutfitRules`](crate::OutfitRules)) | every system within N jumps, and any stellar |
//! | [`InvalidMap`](RuleKey::InvalidMap) | `invalid_map` | a map whose `ModVal` explores nothing (0, -2 to -999) is used up ([`OutfitRules`](crate::OutfitRules)) | it is added to the outfits as a plain item\* |
//! | [`RemoveRefund`](RuleKey::RemoveRefund) | `remove_refund` | `D` removes an outfit and pays nothing ([`OutfitRules`](crate::OutfitRules)) | it pays what selling the outfit would\* |
//! | [`RefitReserves`](RuleKey::RefitReserves) | `refit_reserves` | a refit by `G`, `D` or boarding keeps a shield or armour above the new most until damage, the outfitter or a reload takes it ([`OutfitRules`](crate::OutfitRules)) | each is held to its most\* |
//! | [`PurchasePaintOrder`](RuleKey::PurchasePaintOrder) | `purchase_paint_order` | buying a ship clears the paint after the new ship's `OnPurchase`, so a paint it grants is lost ([`HookRules`](crate::HookRules)) | the paint is cleared before the hook, so a paint it grants stays\* |
//! | [`CaptureHookOrder`](RuleKey::CaptureHookOrder) | `capture_hook_order` | on Use As My Ship, `OnRetire` and `OnCapture` run before the outfit swap, which strips a non-persistent outfit either grants ([`HookRules`](crate::HookRules)) | they run after the swap, so it stays\* |
//! | [`StartShipPurchase`](RuleKey::StartShipPurchase) | `start_ship_purchase` | a new pilot's starting ship runs no `OnPurchase` ([`HookRules`](crate::HookRules)) | it runs once, right before the `chär`'s `OnStart`\* |
//! | [`ShipChangePersistence`](RuleKey::ShipChangePersistence) | `ship_change_persistence` | `H` keeps an outfit flagged 0x0004 or 0x0020 ([`ShipChangeRules`](crate::ShipChangeRules)) | only one flagged 0x0020 |
//! | [`ShipChangeMax`](RuleKey::ShipChangeMax) | `ship_change_max` | `E` and `H` hold every outfit owned, old and new, to its limit (ammunition cap, raised `Max`, gun and turret slots) after adding the new class's default items ([`ShipChangeRules`](crate::ShipChangeRules)) | nothing is held: every outfit is kept |
//! | [`ShipChangeCargo`](RuleKey::ShipChangeCargo) | `ship_change_cargo` | the cargo `C`, `E` and `H` leave past the new hold is trimmed as the set expression ends, as every set expression's reset trims: each commodity cut to trunc(held x holds/total) in single floats, against the fleet's holds with the trader escorts' `Holds`, the mission cargo counted and never cut, the `jünk` never cut (`_SystemInfoToShipStats` @0x1566a, `_EvalCurrentMissionBitSetString` @0x99a85, `_EvalMissionBitSetString` @0x99dea, `_ResetPlayerPrecalcedValues` @0xc7d8-0xc887; [`ShipChangeRules`](crate::ShipChangeRules)) | every ton is kept past the hold: no set expression's reset trims, and the cargo waits for a reset outside one (a take-off, a jump's arrival, a landing, a jettison, ...)\* |
//! | [`ShipChangeReserves`](RuleKey::ShipChangeReserves) | `ship_change_reserves` | `C`, `E` and `H` keep a shield or armour above the new class's most until damage takes it ([`ShipChangeRules`](crate::ShipChangeRules)) | each is held to its new most\* |
//! | [`ShipChangeBayAmmo`](RuleKey::ShipChangeBayAmmo) | `ship_change_bay_ammo` | `E` and `H` load a fighter bay's `AmmoLoad` onto the weapon its raw `AmmoType` names, so a bay of fighter `shïp` 128 to 255 loses its fighters ([`ShipChangeRules`](crate::ShipChangeRules)) | the bay's own fighters are loaded, as a purchase loads them\* |
//! | [`ShipChangeDefaultWeapons`](RuleKey::ShipChangeDefaultWeapons) | `ship_change_default_weapons` | `E` and `H` add a default item that is a weapon or ammunition outfit to the outfits alone, so the fold pulls it back to the weapon and rounds arrays' count and it is lost unless the array has a surplus (`_EvalSetExp` @0x15609, `_ShipStatsToSystemInfo` @0xcaee called @0x15622; [`ShipChangeRules`](crate::ShipChangeRules)) | its first-mod weapons and rounds are added to the arrays too, so it is kept\* |
//! | [`ShipChangeNegativeStock`](RuleKey::ShipChangeNegativeStock) | `ship_change_negative_stock` | `E` and `H` add a negative `WeapCount` or `AmmoLoad` signed, taking that many launchers or rounds away, floored at none (`_EvalSetExp` @0x15555, @0x155c3; [`ShipChangeRules`](crate::ShipChangeRules)) | it adds nothing, as a purchase reads it\* |
//! | [`MoveStarless`](RuleKey::MoveStarless) | `move_starless` | `M` in flight into a system with no stellar keeps the ship's position and velocity ([`ScriptEffectRules`](crate::ScriptEffectRules)) | the ship is put at rest at the system's centre |
//! | [`MoveArrival`](RuleKey::MoveArrival) | `move_arrival` | `M` and `N` only change where the ship is: the course is kept, the system is explored by the next take-off, and in flight only every ship the player leads (hired and mission escorts, fighters out) follows, the new system filling with arrivals over time ([`ScriptEffectRules`](crate::ScriptEffectRules)) | the move is an arrival: explored, the course cleared, and in flight populated at once\* |
//! | [`MoveKeepFlag`](RuleKey::MoveKeepFlag) | `move_keep_flag` | an `N` in flight leaves the next take-off keeping the ship where it landed ([`ScriptEffectRules`](crate::ScriptEffectRules)) | only a landed `N` keeps the landing position\* |
//! | [`BlankLeave`](RuleKey::BlankLeave) | `blank_leave` | a `Q` with no message to show does nothing, and cancels an earlier one ([`ScriptEffectRules`](crate::ScriptEffectRules)) | the player leaves all the same, with no message |
//! | [`ScriptSound`](RuleKey::ScriptSound) | `script_sound` | one mission sound at a time: `P` sounds on the next flight tick, the last one before it counting, and not while the last is still playing ([`ScriptEffectRules`](crate::ScriptEffectRules)) | every `P` sounds at once, landed or not\* |
//! | [`BuyRandom`](RuleKey::BuyRandom) | `buy_random` | an outfit's `BuyRandom` below 1 is never for sale and a ship's below 0 always (`_LoadObjectData` @0x78b8d, @0x7a340; [`Session`](crate::Session)) | an outfit's below 1 or above 100 always, and a ship's 0 or below never† |
//! | [`JunkFlags`](RuleKey::JunkFlags) | `junk_flags` | the free space, the fleet's holds less the goods and the mission cargo aboard (`_TotalCargo` @0x4a0b), measured once, lets tribbles goods overfill the hold; perishable goods decay only while there is space; a good with both flags decays only beside a perishable-only good (`_HandlePlayer` @0x70827-0x7093b, @0x70863-0x70888; [`Session`](crate::Session)) | tribbles goods grow only into that free space, and perishable goods always decay‡ |
//! | [`LauncherSale`](RuleKey::LauncherSale) | `launcher_sale` | a launcher cannot be sold while the rounds held overfill the remaining launchers' `MaxAmmo`, and only when its weapon's `MaxAmmo` is above 0 (`_DoOutfitDialog` @0x5ca75-0x5cbe0; [`Session`](crate::Session)); in a counted sale, (n - 1) is as `counted_sale` says | a launcher cannot be sold while any of its ammunition is held§ |
//! | [`RaisedMax`](RuleKey::RaisedMax) | `raised_max` | a `ModType` 27 outfit multiplies its target's `Max` by the outfits owned times their mods naming it, at least 1 (`_HasMaxOfItem` @0x45e9-0x46c3), and one of n cannot be sold while the target owned exceeds its raw `Max` x (n - 1) (`_DoOutfitDialog` @0x5c7be-0x5ca6f; [`Session`](crate::Session)); in a counted sale, (n - 1) is as `counted_sale` says | the multiplier is the items owned with such a mod, at least 1, and a sale is refused only while the target owned exceeds that maximum after it¶ |
//! | [`EventPrice`](RuleKey::EventPrice) | `event_price` | an active event prices its commodity at `BasePrice` + `PriceDelta`, whatever the level, the highest ID winning, and lists a commodity not traded there (`_DoTradeDialog` @0x5dcf8-0x5dd58; [`Session`](crate::Session)) | the level price plus every event's `PriceDelta`\*\* |
//! | [`PurchaseCargo`](RuleKey::PurchaseCargo) | `purchase_cargo` | a ship purchase trims the commodities to the new fleet's holds, counting the mission cargo aboard but never cutting it, then keeps each commodity's share A/B (the new ship's cargo space over that plus the trader escorts' `Holds`, uncapped) and loses that share of each `jünk`, then trims again (`_SystemInfoToShipStats` @0x5ee99, `_DestroyPartialFleetCargo` @0xcd32-0xcff6, each ending in `_ResetPlayerPrecalcedValues` @0xc7d8-0xc82c; `_TotalCargo` @0x4a0b called @0xc7ea; [`Session`](crate::Session)) | the mission cargo aboard takes the new ship's own cargo space first and always stays, and the goods that fit in the rest are kept, in order†† |
//! | [`OutfitSaleCargo`](RuleKey::OutfitSaleCargo) | `outfit_sale_cargo` | an outfit sale that leaves the fleet's holds below the cargo trims the commodities, as the sale's resets do (`_DoOutfitDialog` @0x5cff6, @0x5cffb; `_ResetPlayerPrecalcedValues` @0xc7d8-0xc887; [`Session`](crate::Session)) | no reset through the sale trims: the cargo stays past the hold until the next reset after it\* |
//! | [`ReleaseCargoShare`](RuleKey::ReleaseCargoShare) | `release_cargo_share` | an escort released by hail takes its share of the fleet's cargo whatever its ship type, as the release tests no `InherentAI` (`_DoCommDialog` @0x96864, `_DoEscortDialog` @0x9549d, @0x9561d; [`Session`](crate::Session)) | only a trader takes a share, as for every other escort lost\* |
//! | [`CaptureCargo`](RuleKey::CaptureCargo) | `capture_cargo` | Use As My Ship leaves the commodities with the old ship, the player taking the captured ship's, which carries none (`_DoShipCapture` @0x412e6-0x412fe, @0x41762-0x41779; [`Session`](crate::Session)) | the player keeps every commodity; either way the reset (@0x41a3e) trims them to the new fleet's holds\* |
//! | [`LeaderlessSwapHold`](RuleKey::LeaderlessSwapHold) | `leaderless_swap_hold` | a ship swapped in for a person by a `ShipGoal` 3 mission, neither hunting nor escorting the player, holds goal 10 with no leader: it coasts with no thrust and no fire, picks no destination, target or ship to inspect, and leaves the hold only when provoked, at its government's word, or as a plundering warship (`_AIMakeEscortFlyInForm` @0x89b8e-0x89c3d, `_HighLevelAIHandler` @0x8ed18-0x8ed31; see [`Goal::Hold`](crate::ai::Goal::Hold)) | it decides as its `düde`'s AI from the start, landing or jumping out\* |
//! | [`JunkPrice`](RuleKey::JunkPrice) | `junk_price` | a `jünk` price is signed, with no floor: a negative one is listed, bought at 0 tons and sold at a loss that can leave the cash below 0, and a row priced 0 is not listed (`_DoTradeDialog` @0x5dddb, @0x5de2f, @0x5e27d, @0x5e543-0x5e546; `_TradeDialogUpdate` @0x4d2a2-0x4d2ac; [`Session`](crate::Session)) | a `jünk` price is never below 0, and a row priced 0 is listed and bought free, limited by space‡‡ |
//! | [`JunkTrade`](RuleKey::JunkTrade) | `junk_trade` | every listed `jünk` row is bought and sold at its own price, the order naming the row (`_CanBuyGoods` @0xccec, `_CanSellGoods` @0x4a94, `_DrawTradeButtons` @0x2938f/0x2939d, `_TrackTradeButtons` @0x2960d/0x2963e; [`Session`](crate::Session)) | the `SoldAt` row is bought only and the `BoughtAt` row sold only, as the Bible's "`SoldAt` … where the commodity is sold" and "`BoughtAt` … where the commodity is purchased" say |
//! | [`OutfitLimit`](RuleKey::OutfitLimit) | `outfit_limit` | a map (`ModType` 16) or clean-record (`ModType` 21) outfit is refused while the last outfit bought since the outfitter opened was of its kind; any other buy, or opening the outfitter again, lifts it (`_CanBuyOutfitItem` @0x4e8bc-0x4e9c5, `_GrantOutfitItem` @0x44d67-0x44f8b, `_DoOutfitDialog` @0x5bb37-0x5bb44; [`Session`](crate::Session)) | no limit§§ |
//! | [`OutfitRefund`](RuleKey::OutfitRefund) | `outfit_refund` | an outfit sold while its count owned is above the count owned when the outfitter opened (one bought since) refunds its full price, any other half, item by item in a counted sale (`_DoOutfitDialog` @0x5badd-0x5bafa, @0x5ceee-0x5cf08; [`Outfitter::refund`](crate::Outfitter::refund)) | every sale refunds half§§§ |
//! | [`OutfitCount`](RuleKey::OutfitCount) | `outfit_count` | Option (Alt here) with Buy or Sell asks for a count in the quantity dialog at the most the order allows, and buys or sells the count confirmed (`_DoOutfitDialog` @0x5c0c0-0x5c1aa, @0x5c5e3-0x5c628; `_DoQuantityDialog` @0x56d5b; [`Outfitter::count_asked`](crate::Outfitter::count_asked)) | Alt changes nothing: each click moves one¶¶¶ |
//! | [`SaleMass`](RuleKey::SaleMass) | `sale_mass` | only an outfit of negative mass is refused, when the free mass clamped at 0 plus its mass is below 0; one of mass 0 or more sells while the free mass is below 0 (`_DoOutfitDialog` @0x5c735-0x5c747, `_ShipFreeMass` @0xb506-0xb50a; [`Session`](crate::Session)) | a sale is refused whenever it would leave the free mass below 0\*\*\*\* |
//! | [`SaleRefusal`](RuleKey::SaleRefusal) | `sale_refusal` | Sell is enabled by the Sell flag alone (some owned, not `CANNOT_SELL`). The free mass, a raised `Max` and a launcher's ammunition refuse only inside the sell loop. The first such refusal of an order shows its `STR#` 2002 #207-#212 words once in `DLOG` 3003, and nothing is sold. A counted sale runs on past it, each later pass refused silently; the sales before it stand (@0x5d19c-0x5d1b1) (`_DoOutfitDialog` @0x5c563-0x5c595, @0x5c735-0x5ce6c, flag -0x63e @0x5c6ca; `_OutfitFilter` @0x5808c-0x580b1; `_ShowTextDialog` @0x5a8cf; [`Outfitter::can`](crate::Outfitter::can) / [`Outfitter::sale_alert`](crate::Outfitter::sale_alert)) | Sell is greyed by any sale check too, and the info box words the first one; nothing is shown in a dialog‡‡‡‡ |
//! | [`CountedSale`](RuleKey::CountedSale) | `counted_sale` | each pass of a counted sale checks the order as a whole: a raised `Max` refuses while the target owned exceeds its raw `Max` x (owned now - the whole count) (@0x5c82d-0x5c85e), and a launcher while the rounds held exceed `MaxAmmo` x (owned now - the pass index - 1) (@0x5cb9c-0x5cbc7), signed, so an Alt sale is refused sooner than as many single clicks; only the engine readings of `raised_max` and `launcher_sale` read it (`_DoOutfitDialog`; [`Outfitter::check_at`](crate::Outfitter::check_at)) | each pass is checked as one sale, (owned now - 1)¶¶¶¶ |
//! | [`RelistedSale`](RuleKey::RelistedSale) | `relisted_sale` | a sale that changes the listing (`_SetupPortAvailableItems` true, @0x5d00a) resets the selection to `oütf` 128 and the loop index to 0, or 512 with nothing listed, so the passes left check and sell `oütf` 128, listed or not, and can outrun the count (`_DoOutfitDialog` @0x5d017-0x5d150, @0x5d164-0x5d1b1; [`Session`](crate::Session)) | the ordered outfit and the pass index are kept††††† |
//! | [`RelistedSelection`](RuleKey::RelistedSelection) | `relisted_selection` | an order that changes the listing (`_SetupPortAvailableItems` true after the buy loop @0x5c314 or a sale @0x5d00a) selects `oütf` 128 with no cell highlighted (`*0xded58` = 0, `*0xded5c` = -1, @0x5c32c-0x5c33b, @0x5d01c-0x5d02b). Buy and Sell then answer for it, listed or not (`_OutfitFilter` @0x5806f-0x580ca), and the picture, description and info box are blank. With nothing listed the outfitter closes (-0x63d @0x5c40c, @0x5d128, @0x5da33; [`Session`](crate::Session)) | the selected cell is kept and the outfitter stays open§§§§§ |
//! | [`GridSelection`](RuleKey::GridSelection) | `grid_selection` | the outfitter's selection is the selected outfit (`*0xded58`), which can be none, and its description is read only when it changes: a press on an empty cell position in the grid selects nothing (`*0xded58` = `*0xded5c` = -1, @0x58632-0x58648), greying Buy and Sell and blanking the picture, info box and description (@0x588e4-0x588fe; `_OutfitDialogUpdate` @0x5789b-0x578a5), and an arrow that leaves the selected outfit as it was reads no description (@0x58442-0x5844f) while the picture and info box follow the highlighted cell (@0x57949-0x579d6) (nova-view's `OutfitterScreen`) | the selection is the highlighted cell: a press on an empty cell is ignored, and the description follows the cell¶¶¶¶¶ |
//! | [`MassPrice`](RuleKey::MassPrice) | `mass_price` | an outfit flagged 0x0200 costs max(`Cost`, `Cost` × the flown class's `Mass`), so `Cost` on a class of `Mass` 0 or less, and a `Cost` of 0 or less costs 0, flag or not (`_AdjustedItemCost` @0x5869-0x5895; `_PlayerShipTradeInPrice` @0xb0e3-0xb106; [`unit_price`](crate::outfitter::unit_price)) | `Cost` × `Mass`, never below 0§§§§ |
//! | [`ScaledMass`](RuleKey::ScaledMass) | `scaled_mass` | an outfit flagged 0x0400 of `Mass` above 0 weighs max(`Mass`, `Mass` × the class's `Mass` / 100, truncated), so its own `Mass` on a class of `Mass` below 100, in the outfitter, the free mass, the grants and the Shipyard Info Space (`_AdjustedItemMass` @0x2889-0x28c2; `_ShipFreeMass` @0xb4c1-0xb4ed; [`unit_mass`](crate::outfitter::unit_mass)) | `Mass` × the class's `Mass` / 100, truncated, for `Mass` above 0‡‡‡‡‡ |
//! | [`TradeInOutfits`](RuleKey::TradeInOutfits) | `trade_in_outfits` | a ship's trade-in counts every outfit owned but a persistent one, an unsellable one included, at half its line (`_PlayerShipTradeInPrice` @0xb0d5-0xb0e1, @0xb109-0xb11f; [`Session`](crate::Session)) | an unsellable outfit is left out too†††† |
//! | [`TradeLot`](RuleKey::TradeLot) | `trade_lot` | a plain Buy at the exchange moves min(trunc(cash / price), 10, free) tons and a plain Sell min(held, 10); Option (Alt here) asks for a count (see [`TradeCount`](RuleKey::TradeCount)) (`_DoTradeDialog` @0x5e268-0x5e278, @0x5e48d-0x5e4fa; [`Session`](crate::Session)) | a plain trade moves 1 ton¶¶ |
//! | [`TradeQuotient`](RuleKey::TradeQuotient) | `trade_quotient` | the most a buy can move is min(trunc(f32(cash) / f32(price)), free), in single floats, made 32000 from 32001 up, so above 2^24 cash it can buy a ton more than the cash covers and leave the cash below 0 (`_DoTradeDialog` @0x5e21d-0x5e234, @0x5e23a-0x5e24c; [`Market::row_max`](crate::Market::row_max)) | exact integer division\*\*\* |
//! | [`TradeCount`](RuleKey::TradeCount) | `trade_count` | Option (Alt here) with Buy or Sell asks for a count in the quantity dialog, opening at the most the row offers, and trades the count confirmed (`_DoQuantityDialog` @0x56d5b; `_DoTradeDialog` @0x5e211, @0x5e45c; [`Market::option_asks_count`](crate::Market::option_asks_count)) | Alt trades the most at once††† |
//! | [`TradeDebt`](RuleKey::TradeDebt) | `trade_debt` | a buy divides the cash, signed, by the price, so with cash below 0 a `jünk` priced exactly at the cash is bought, 1 ton, leaving the cash at 0; Buy is refused while cash < price under both readings (`_DoTradeDialog` @0x5e21d-0x5e234; `_CanBuyGoods` @0xccec-0xcd31; [`Market::tons`](crate::Market::tons)) | cash below 0 counts as none‡‡‡ |
//! | [`TechLevel`](RuleKey::TechLevel) | `tech_level` | an outfit of `TechLevel` below 0 or exactly 32767, and a ship of one below 0, is never sold by tech (`_SetupPortAvailableItems` @0xbf88-0xbf9d, `_SetupPortAvailableShipTypes` @0xbc4f-0xbc52; [`wares::outfit_tech_allows`](crate::wares::outfit_tech_allows), [`wares::ship_tech_allows`](crate::wares::ship_tech_allows)); and `_CanBuyOutfitItem` tests an outfit's Buy apart: one of `TechLevel` -1 always passes, any other passes at or below the stellar's tech or by a slot, with no sign or 0x7fff test (@0x4e7ff-0x4e83d; [`wares::outfit_buy_tech_allows`](crate::wares::outfit_buy_tech_allows)) | any `TechLevel` at or below the stellar's, or equal to a slot, is sold, and can be bought, by that one test ([`wares::by_tech_level`](crate::wares::by_tech_level)) |
//! | [`PurchaseCarry`](RuleKey::PurchaseCarry) | `purchase_carry` | a ship purchase keeps every persistent outfit (`Flags` 0x0004) whatever its `Max` and mass, sells none back, and adds the new class's default items on top (`_DoShipyardDialog` @0x5ee1b-0x5ee97; [`Session`](crate::Session)) | each is kept up to its `Max` and while the new ship's free mass holds it, by ascending ID, and the rest sold back at the outfitter's rate on the old ship's mass\*\*\*\*\* |
//! | [`ShipInfoRounding`](RuleKey::ShipInfoRounding) | `ship_info_rounding` | the Shipyard Info panel reads `Speed` / 100 and `Accel` / 10000 back from the loader's single floats, so `Speed` 190 draws 189 and `Accel` 130 and 250 rate a step higher (`_LoadObjectData` @0x7a0b3-0x7a0ef, `_ShipDescDialogUpdate` @0x4fbf0-0x4fbfe, @0x4fc74-0x4fd05; [`ShipSpecs`](crate::ShipSpecs)) | `Speed` as recorded and `Accel` rated on whole numbers\* |
//! | [`StaleSecondary`](RuleKey::StaleSecondary) | `stale_secondary` | a refit outside the shops (`G`, `D`, a script's change of ship, boarding) leaves the secondary selected as it is, one the ship no longer carries too: it is still named and fires nothing, as `_EvalSetExp` and `_SystemInfoToShipStats` never write it and `_DrawStatusWeap` draws it (@0x4c16c-0x4c1b9; [`Session`](crate::Session)) | a refit lets go of a secondary the ship no longer carries\* |
//! | [`SecondarySale`](RuleKey::SecondarySale) | `secondary_sale` | the outfitter's sale lets go of the secondary selected when the sold outfit's own count falls to 0 and one of its `ModType` 1 slots names it, even with another outfit still mounting it (`_DoOutfitDialog` @0x5cf73-0x5cf8c, @0x5ce82; [`Session`](crate::Session)) | the sale lets it go when it leaves the ship not carrying it\* |
//! | [`RepeatMissionFailure`](RuleKey::RepeatMissionFailure) | `repeat_mission_failure` | an unabortable mission failed by its deadline stays active and failed, and at its return landing runs `OnFailure` a second time and applies the failure records (`_QuickMissionFailure` @0x99d1b-0x99dab, `_MissionLandCheck` @0xa1a92, `_DoMissionFailure` @0xa0285; see [`mission`](crate::mission)) | its return landing frees it without running `OnFailure` again, the failure records still applied once; a mission failed by `F` runs `OnFailure` once at its return either way\* |
//! | [`StartActiveMission`](RuleKey::StartActiveMission) | `start_active_mission` | `S` accepts a mission active already into another slot (`_EvalSetExp` @0x153b9-0x15418; see [`mission`](crate::mission)) | `S` leaves an active mission and the slots as they are\* |
//! | [`PayMillions`](RuleKey::PayMillions) | `pay_millions` | a mission text's `<PAY>` of a million or more reads in millions to two places, truncated, "1.23M" (`_StrcatCommaNumber` @0x8990-0x8aa3; see [`text_tags`](crate::text_tags)) | it reads in full with its commas, "1,234,567"\* |
//! | [`MissionOrder`](RuleKey::MissionOrder) | `mission_order` | offers of `DispWeight` 0 or more keep index order, the rest after by descending weight (`_RandomizeMissionData` @0xa1842-0xa197d; see [`mission`](crate::mission)) | all by descending `DispWeight`, ties in index order |
//! | [`PersonMissionStellar`](RuleKey::PersonMissionStellar) | `person_mission_stellar` | a person's mission is resolved at its spawn from the player's navigation target, or from no stellar with none, and an accept in flight reaches a destination that is the navigation target (`_RandomizeOneMission` @0xa151f-0xa1526, `_DoMissionAccept` @0xa1fc5-0xa1ff8; see [`mission`](crate::mission)) | it is resolved from the first stellar the player's system lists, and an accept in flight reaches no destination\* |
//! | [`MissionStellarDraw`](RuleKey::MissionStellarDraw) | `mission_stellar_draw` | a random destination -2 is drawn as the original draws it: any inhabited, normal-landable non-station stellar of a visible system but the origin and the one excluded, at any distance, once its scan finds a distant inhabited stellar, station or not; the draw tests the distance of the scan's first hit, not of the stellar drawn (`_SelectMissionStellar` @0x9a26a, @0x9a2c7-0x9a330, the distance test @0x9a3ce-0x9a3db; see [`mission`](crate::mission))\*\*\*\*\* | only a candidate distant from the origin is drawn, as every other random code draws\* |
//! | [`MissionStellarDistance`](RuleKey::MissionStellarDistance) | `mission_stellar_distance` | a random stellar is distant from an origin in a visible system when its system is not at the origin's place, neither system's own Con links reach a system at the other's place, and every system at its system's place lists it; with no origin, as availability asks, only by that last test; from an origin of no visible system nothing is distant (`_StellarsAreDistant` @0x9cb9-0x9ef6, the co-located chain walk @0x9e60-0x9ee4; see [`mission`](crate::mission)) | it is in another system that the visible map does not link with the origin's either way, and nothing is tested with no origin or an origin of no visible system\* |
//! | [`MissionStellarIdentity`](RuleKey::MissionStellarIdentity) | `mission_stellar_identity` | a mission's destination or return is the stellar docked at when their IDs match, or when both have the same `xPos` and `yPos` and the same name, in any system (`_StellarsAreIdentical` @0xab35-0xac90, called by `_MissionLandCargoCheck` @0x9edfd, @0x9f01d and `_MissionLandCheck` @0xa1a37; see [`mission`](crate::mission)) | only the stellar of that ID, as `TravelStel` and `ReturnStel` each name one stellar by ID |
//! | [`LegalRecords`](RuleKey::LegalRecords) | `legal_records` | one record a system (`_playerRecord`), read in the player's system (`_HandlePlayerDockRequest` @0x67903, `_IsMissionAvailable` @0x9b59d, `_DoTradeDialog` @0x5dc17; see [`legal`](crate::legal)) | one record a government, read through the system's (landing: the stellar's) government |
//! | [`LeftRespawn`](RuleKey::LeftRespawn) | `left_respawn` | only a chase-off goal's ship (`ShipGoal` 6) that leaves the system lowers its mission's ships left, so a ship of another goal comes back at the next setup (`_LowLevelAIHandler` @0x85983-0x859aa; see [`mission::goals`](crate::mission::goals)) | any departed mission ship not of `ShipSyst` -6 lowers its ships left, and is not brought back\* |
//! | [`LandedLeaves`](RuleKey::LandedLeaves) | `landed_leaves` | a mission ship that lands does not leave the system, as the original's waits at the stellar (`_HighLevelAIHandler` @0x8dadc-0x8db5a), so the engine taking it out counts nothing and it comes back at the next setup (see [`story`](crate::story)) | it leaves the system, counted as one that jumped out\* |
//! | [`AuxGate`](RuleKey::AuxGate) | `aux_gate` | at a hypergate or wormhole exit, the auxiliary ships of a mission of `AuxShipSyst` -1 come no more until their countdown next restarts (`_HandlePlayerDockRequest` @0x6729b-0x672ac; see [`mission::aux_ships`](crate::mission::aux_ships)) | the exit restarts them as a jump does: "-1: any system" |
//! | [`AuxAbortFallThrough`](RuleKey::AuxAbortFallThrough) | `aux_abort_fall_through` | after a `Flags` 0x0001 mission auto-aborts as its auxiliary ships all come, the special-ship part runs on the freed slot's stale record: when its appear timer runs out that frame the special ships come, marked with the freed slot, and the mission auto-aborts again, its script, days, fuel and pay over again (`_EnterMoreShips` @0x4373a-0x43752, @0x43757-0x43ad4; see [`mission::aux_ships`](crate::mission::aux_ships)) | the freed slot is skipped\* |
//! | [`PlainBoardCounts`](RuleKey::PlainBoardCounts) | `plain_board_counts` | a board or rescue goal's ship counts as boarded only on the assist path (`ShipGoal` 2 or 5, `Flags` 0x0001, one ship left) or on a `PickupMode` 2 pickup; a plain board only marks it boarded, so a goal with neither path is never met (`_HandlePlayerBoardAttempt` @0x653a7, @0x65575-0x65762; see [`mission::goals`](crate::mission::goals)) | any board of a board or rescue goal's ship counts, as the Bible's "board them" and "rescue them" read |
//! | [`HiddenLossFails`](RuleKey::HiddenLossFails) | `hidden_loss_fails` | a hidden mission (`Flags` 0x0400) is not failed at once when one of its ships is destroyed or disabled, so a hidden board or rescue mission whose ship is destroyed never ends (`_HandleShipDisplay` @0x2db27, `_DamageShip` @0x3aede; see [`mission::goals`](crate::mission::goals)) | the first loss fails it as any other, but with no message\* |
//! | [`ShipDoneOnce`](RuleKey::ShipDoneOnce) | `ship_done_once` | every time a mission's goals go from unmet to met, its `ShipDoneText` is shown and its `OnShipDone` runs, so an escort goal's run again each time its ships come back to the player (`_MissionObjectivesCheck` @0x9ec70-0x9ed87; see [`mission::goals`](crate::mission::goals)) | the moment's text, script and auto-abort happen only the first time\* |
//! | [`SwapArrives`](RuleKey::SwapArrives) | `swap_arrives` | the ship swapped in for a person raises no arrived count and leaves the appear timer running, so for a `ShipStart` of 1 or more the timer still brings another ship in beside it (`_HandlePlayerCommunication` @0x6244b-0x6252f, `_EnterMoreShips` @0x437ec-0x43aa5; see [`mission::ships`](crate::mission::ships)) | the swapped ship counts as the timer's arrival: the timer stops, and for `ShipStart` 1 the arrived count goes up by one\* |
//! | [`SmugglingShipName`](RuleKey::SmugglingShipName) | `smuggling_ship_name` | the smuggling notice names the scanner by the found mission's drawn ship name whenever that name is not empty, whoever the scanner is (`_ScanPlayer` @0x7e7ce; see [`scan`](crate::scan)) | by the mission's ship name only when the scanner is that mission's special ship, else by its class\* |
//! | [`HudCadence`](RuleKey::HudCadence) | `hud_cadence` | the HUD's content refreshes on the original's timing: the radar every 15 Mac ticks, 8 engine ticks (`_DrawStatus` @0x4c85d, @0x4c911-0x4c94f), the nav area every 15 engine ticks (`TickCount() % 30`, @0x4c899-0x4c8b6) or at once on a selection change, and the target panel every 45 engine ticks (`TickCount() % 45`, @0x4c8b9-0x4c8eb), at the next radar refresh once its shield or ship type changes (@0x4c963-0x4c994), or at once on a change of target; docked, the bar holds between the port's redraws: the radar after an outfitter order, radar, nav and target on a ship purchase (everything on a new `ïntf`, @0x739bd), fuel on a recharge (@0x5fe05), everything as a shop closes (`_RefreshSAT`); see `nova_view::flight::cadence` | it follows every frame\* |
//! | [`GateFollow`](RuleKey::GateFollow) | `gate_follow` | at a hypergate or wormhole exit, the ships of a mission following the player (`ShipSyst` -6) escort it at once, follow it out of the exit gate on their government's `Flags2` or odds when they attack it, and are otherwise lost until a `ShipStart` 1 timer restarts (`_HandlePlayerDockRequest` @0x6716a-0x67487; see [`mission::ships`](crate::mission::ships)) | the exit leaves them as a jump does, and its setup sets them up again\* |
//! | [`AlertBlink`](RuleKey::AlertBlink) | `alert_blink` | a Red Alert blinks its icon only while the player's sound is quiet, off or at effects level 1 or less (the original's `Volume` preference ≤ 1); louder, the alert is the sound alone (`_HandlePlayer` @0x68d04-0x68d15; see [`red_alert`](crate::red_alert)) | the icon blinks with every alert\* |
//! | [`CronRandomScale`](RuleKey::CronRandomScale) | `cron_random_scale` | `Rand(101) <= Random`, rolled for every inactive crön before its other checks (`_CronEventHandler` @0x38804-0x38818; see [`cron`](crate::cron)) | `Random` % (0 never, 100 or more always)§§§§§§ |
//! | [`CronDateCompare`](RuleKey::CronDateCompare) | `cron_date_compare` | year on its own, month and day on their own; a range across a year end with month/day set never matches (`_CronDateInRange` @0x74d3-0x75bf; see [`cron`](crate::cron)) | compared as dates; a range may cross a year end¶¶¶¶¶¶ |
//! | [`NewsGovtMatch`](RuleKey::NewsGovtMatch) | `news_govt_match` | a cron's news shows at a stellar whose government is allied with its `NewsGovt` (`_GovtAllies` @0x4e3d, called @0x48640; see [`news`](crate::news)) | only at a stellar of that government\*\*\*\*\*\*\* |
//! | [`RankUnaffiliatedGroup`](RuleKey::RankUnaffiliatedGroup) | `rank_unaffiliated_group` | the sweeps take every rank of `AffilGovt` -1 (or 127 and below) as one government, by raw equality (`_ActivateRank` @0x4535f-0x45367, `_DeactivateRank` @0x45487-0x4548f; see [`rank`](crate::rank)) | an unaffiliated rank shares no government: it is never swept, and its own sweeps clear nothing††††††† |
//! | [`SalaryCap`](RuleKey::SalaryCap) | `salary_cap` | the cap is tested before paying, so one payment can overshoot it; a cap of 0 or less is none (`_IncrementGameTime` @0xb779-0xb794; see [`rank`](crate::rank)) | the cash never goes past the cap‡‡‡‡‡‡‡ |
//! | [`RankGovtReach`](RuleKey::RankGovtReach) | `rank_govt_reach` | a rank's flags reach ships of its government and of every government `_GovtAllies` finds allied with it (`_DoCommDialog` @0x95dbf; see [`rank`](crate::rank)) | only ships of the rank's own government§§§§§§§ |
//! | [`RankPriceScope`](RuleKey::RankPriceScope) | `rank_price_scope` | a rank's `PriceMod` reaches only the shipyard: a ship's price, the hire fee and the trade-in (`_ApplyPriceAndTechnologyFlux` @0x4e6cd, read only by `_CalcShipCanBuy`, `_ShipyardDialogUpdate` and `_DoShipyardDialog`; see [`rank`](crate::rank)) | the outfitter's buy price too‡‡‡‡‡‡‡‡ |
//! | [`RankTradeIn`](RuleKey::RankTradeIn) | `rank_trade_in` | the trade-in goes through the flux twice, `_priceFluxMult[0]` then `[1]` (@0x4f769-0x4f7d3, @0x59433-0x5949a, @0x5ec13-0x5ec96), so the rank's factor is squared and the low-tech discount and rounding apply twice (see [`shipyard`](crate::shipyard)) | once, with `[1]`§§§§§§§§ |
//! | [`RankPriceTiming`](RuleKey::RankPriceTiming) | `rank_price_timing` | the multipliers are taken as the spaceport opens (`_DoPortDialog` @0x5f970-0x5fa04, before the landing's mission check @0x5fbeb), so a rank gained while landed changes no price until the next landing (see [`rank`](crate::rank)) | live: the ranks and the stellar's government as they are when a price is asked¶¶¶¶¶¶¶¶ |
//! | [`RankCrimeReach`](RuleKey::RankCrimeReach) | `rank_crime_reach` | a crime takes a rank of `Flags` 0x0004 or 0x0040 when `_GovtAllies` finds the victim's government allied with the rank's, an equal one included, so a crime against an independent ship takes an unaffiliated rank (`_SlapWithPenalty` @0x9c70; see [`rank`](crate::rank)) | 0x0040 only for a crime against the rank's own government, 0x0004 for its own or its allies, an unaffiliated rank never\*\*\*\*\*\*\*\*\* |
//! | [`ColocatedResolution`](RuleKey::ColocatedResolution) | `colocated_resolution` | the lowest-ID visible system at a place stands for every system there, a visible one too; links reach only it; a visible system it shadows claims no stellar (`_FindActiveCoLocatedSystem` @0x4c02-0x4c40, `_PropagateMissionBitEffects` @0x996ed-0x997bd; see [`story`](crate::story)) | a visible system stands for itself, links reach every visible system at the place, and each stellar goes to the lowest-ID visible system listing it¶¶¶¶¶¶¶ |
//! | [`VisibilityRefresh`](RuleKey::VisibilityRefresh) | `visibility_refresh` | `Visibility` is evaluated at a landing (@0xa175f, @0xa1af2), a jump's arrival before its days (@0x6bf3f), a new pilot after its `OnStart`, and a load (see [`story`](crate::story)) | after every settled script as well\*\*\*\*\*\*\*\* |
//! | [`CronZeroDurationHoldoff`](RuleKey::CronZeroDurationHoldoff) | `cron_zero_duration_holdoff` | a `Duration` 0 crön's pre-holdoff end runs `OnStart` and `OnEnd` and writes nothing more (`_CronEventHandler` @0x387a8-0x387c1), so the next day the running branch runs `OnEnd` again (@0x387d3-0x387e8) and ends as `cron_post_holdoff` says (see [`cron`](crate::cron)) | `OnEnd` once, then the `PostHoldoff`: **the default — a deliberate departure from the original**†††††††† |
//! | [`CronPostHoldoff`](RuleKey::CronPostHoldoff) | `cron_post_holdoff` | a running crön's end, with `PostHoldoff` above 0, loads its `PreHoldoff` into the holdoff and leaves its days (@0x387f1-0x387f6 → 0x388c5): it re-runs, or with `PreHoldoff` 0 its `OnEnd` runs every day for good and its news stays (see [`cron`](crate::cron)) | `PostHoldoff` days with the days at -1, then inactive: **the default — a deliberate departure from the original**†††††††† |
//! | [`BlessingRefresh`](RuleKey::BlessingRefresh) | `blessing_refresh` | the blessing table is taken at each of the original's resets (`_ResetPlayerPrecalcedValues` @0xc8be-0xc981), the end of every set expression that is not empty among them, so a `K` or `L` blesses or stops blessing as its expression ends, not between its operators, and a crime's rank loss (`_SlapWithPenalty`, which reaches no reset) only at the next reset (see [`rank`](crate::rank)) | live, from the active ranks: within an expression, and from the crime on\* |
//! | [`BlessingLanding`](RuleKey::BlessingLanding) | `blessing_landing` | `Flags` 0x0200 clears a governed stellar past both its `MinStatus` and its government's `Require` (`_HandlePlayerDockRequest` @0x67940-0x67a05; see [`landing`](crate::landing)) | past `MinStatus` only, so `Require` still refuses |
//! | [`BlessingAlwaysAttacks`](RuleKey::BlessingAlwaysAttacks) | `blessing_always_attacks` | a `Flags` 0x0004 government's word turns its warships and interceptors on a player that a rank's `Flags` 0x0100 blesses with that government's peace, as `_WarshipAI` @0x8b7b0 and `_InterceptorAI` @0x8cb5c read no blessing (see [`ai::warship`](crate::ai::warship)) | the blessing holds the word back, as the Bible's ränk 0x0100 "will not automatically attack" reads |
//! | [`ExploredByPlace`](RuleKey::ExploredByPlace) | `explored_by_place` | exploring a system explores its place's head (`_RecursiveAutoExplore` @0xd9b6-0xda10, the arrivals and the take-off), and each rebuild point explores every system at an explored head's place, hidden or not (`_HandleStellarSystemVisibility` @0x32b13-0x32b8f; see [`exploration`](crate::exploration)) | each system is explored on its own\* |
//! | [`RouteRewrite`](RuleKey::RouteRewrite) | `route_rewrite` | each rebuild point rewrites every hidden hop of the course to the system at its place and cuts the course at the first hop with none (`_HandleStellarSystemVisibility` @0x32ab3-0x32b0d), and an arrival that made no hop clears the course when its next hop stands elsewhere, or drops that hop when it stands at the arrival (@0x6c472-0x6c4c6; see [`course`](crate::course)) | hops resolved only when used, and an arrival drops only the hops made\* |
//! | [`NebulaExploreOnce`](RuleKey::NebulaExploreOnce) | `nebula_explore_once` | whether a nebula is explored is never saved, so a load's sweep over the visible explored systems runs its `OnExplore` again (`_ResetPlayer` @0x1da86, `_OpenLastPilotFile` @0x765a2-0x765ce; see [`nebula`](crate::nebula)) | `OnExplore` runs once a pilot, through the once-fired record\* |
//! | [`NebulaNewPilot`](RuleKey::NebulaNewPilot) | `nebula_new_pilot` | a new pilot explores no nebula until its first exploration, a take-off, a jump or a map (`_DoNewPilot` @0x18de5, @0x1df4b, @0x19118, before its first refresh @0x1925e; see [`nebula`](crate::nebula)) | the start system's nebulae are explored as the game begins††††††††† |
//! | [`ScanOutfitsPastJunk`](RuleKey::ScanOutfitsPastJunk) | `scan_outfits_past_junk` | while the player carries any `jünk` with a `ScanMask`, a scan that finds no illegal `jünk` ends without scanning the outfits (`_ScanPlayer` @0x7eabd-0x7eefa; see [`scan`](crate::scan)) | the outfits are scanned when no illegal `jünk` is found\* |
//! | [`OutfitScanFine`](RuleKey::OutfitScanFine) | `outfit_scan_fine` | only a `ScanFine` above 0 fines an illegal outfit, as it is; a negative one fines nothing (`_ScanPlayer` `jle` @0x7f222; see [`scan`](crate::scan)) | a negative `ScanFine` takes that percent of the cash in hundreds, at least 1, as for `jünk` and mission cargo (the Bible's `ScanFine`) |
//! | [`GateEscorts`](RuleKey::GateEscorts) | `gate_escorts` | at a hypergate or wormhole exit, every ship the player leads comes out of the exit gate after it, with its heading, waiting `Rand(20) + 15` ticks and bursting out at 15 (`_HandlePlayerDockRequest` @0x674ef-0x675c8, `_AIMakeShipEmergeFromHyperGate` @0x895aa-0x895bd) | each is snapped onto its slot beside the player\* |
//! | [`LostCargoShare`](RuleKey::LostCargoShare) | `lost_cargo_share` | a lost trader escort's own `Holds` are never in the fleet's space it takes its share of, so it takes min(1, its `Holds` over the rest) of every good (`_DamageShip` @0x3afdd, @0x3b013; `_DestroyPartialFleetCargo` @0xce98-0xcf37; [`Session`](crate::Session)) | they are, so it takes the share of the space it added, mission escorts too\* |
//! | [`NonStrictSpeed`](RuleKey::NonStrictSpeed) | `non_strict_speed` | the player and every ship it leads fly at 1.5 × their top speed when the pilot does not play Strict Play (`_ShipMaxSpeed` @0x382e-0x384f, @0xdd078; see [`top_speed`](crate::top_speed)) | every ship flies at its own top speed, Strict Play or not\* |
//! | [`HunterBribe`](RuleKey::HunterBribe) | `hunter_bribe` | a `ShipBehav` 0 mission ship the player paid off attacks the player again on its next think, as no site reads the bribe (`_HighLevelAIHandler` @0x8d628-0x8d6e9, `_AIMakeShipLeave` @0x7e2b0; see [`mission_hunt`](crate::ai::target::mission_hunt)) | the bribe holds: it decides as its type, a wimpy trader, does\* |
//! | [`ShipOverlap`](RuleKey::ShipOverlap) | `ship_overlap` | ships overlap within each draw layer in the order they last entered it, the player starting on top and a ship entering a layer going over every ship in it (`_PutSpriteInLayer` @0x766c, `_InitSprites` @0x1f0eb-0x1f88b, `_DoPlayGameWork` @0x43e29, @0x444ad; see [`Session::draw_order`](crate::Session::draw_order)) | each layer in the session's NPC order, the player on top of its own\* |
//! | [`QuadrantTurretArc`](RuleKey::QuadrantTurretArc) | `quadrant_turret_arc` | an NPC's quadrant turret fires only while the angle to its target less the heading (front, 7) or `(heading + 180) mod 360` (rear, 8), absolute and mod 360 with no fold, is at most 45, so one whose reference sits near 0 holds fire across the seam (`_AIFireTurret` @0x80c3d-0x80cf3; [`NovaAi`](crate::NovaAi)) | the angle folds and the arc wraps\* |
//! | [`HiddenShips`](RuleKey::HiddenShips) | `hidden_ships` | a ship whose type's `Cost` is -1 is hidden, the player too, and its slot keeps its draw-layer place, as `_HandleShipDisplay` skips its showing and layer block (@0x2b7f3 → @0x2cc6b; see [`Session::draw_order`](crate::Session::draw_order)) | drawn and laid out as any other ship\* |
//! | [`HiddenShipHoverBox`](RuleKey::HiddenShipHoverBox) | `hidden_ship_hover_box` | while `hidden_ships` reads the engine, the pointer hovers a hidden ship by the box of the frames its slot last cloned, its last shown type's or `shïp` 128's in a slot nothing shown has used, as `_HandleShipDisplay` skips `_SWCloneFramesFromMaster` (@0x2b824) and `_HandleMouse` boxes the stale sprite (@0x38171; `_InitSprites` @0x1f193; see [`Session::framed_by`](crate::Session::framed_by)) | the box of its own type's sheet\* |
//! | [`BreakupStall`](RuleKey::BreakupStall) | `breakup_stall` | a breakup whose death timer never rises above 0 by the display check (`DeathDelay` 0 or less, or 1 for an NPC) is set again every frame and never explodes (`_HandleShipDisplay` @0x2d80d-0x2d83d; the decrements @0x6d7e6, @0x35083; see [`Breakup`](crate::combat::hull::Breakup)): for the player, a soft-lock | it is destroyed on its breakup tick (0 or less) or the tick after (an NPC's 1): **the default — a deliberate departure from the original**†††††††††† |
//! | [`EjectVelocity`](RuleKey::EjectVelocity) | `eject_velocity` | as the player ejects, its new ship's x velocity takes the y velocity's value, the hulk keeping the true velocity, before the push of its top speed along its heading (`_HandlePlayer` @0x6f782-0x6f788; see [`escape`](crate::escape)) | the new ship keeps the velocity it had\* |
//! | [`RescueArmor`](RuleKey::RescueArmor) | `rescue_armor` | the escape pod's rescue sets the armour to the shield capacity, `_ShipShieldCapacity` a second time (`_HandlePlayer` @0x690db-0x69100; see the session's `rescue` module), so it may stand above the armour's most | the armour is filled to its own capacity\* |
//! | [`UnlistedStellar`](RuleKey::UnlistedStellar) | `unlisted_stellar` | a loaded pilot whose saved stellar no system lists resumes in the lowest system that exists, which needn't hold the stellar (`_LoadPilotData` @0x752d6-0x75318; see [`Session::load`](crate::Session::load)) | it keeps its saved system: **the default — a deliberate departure from the original**††††††††††† |
//! | [`NewbieFuelRefusal`](RuleKey::NewbieFuelRefusal) | `newbie_fuel_refusal` | a newbie whose J has a hyperspace selection but too little fuel is told nothing, with no beep, as the jump key's newbie branch swallows it (`_HandlePlayer` @0x6b995-0x6b9a3; see [`tutorial`](crate::tutorial)) | "Insufficient energy for hyperspace jump." with `snd ` 153, as for any pilot\* |
//! | [`MissingBuoyString`](RuleKey::MissingBuoyString) | `missing_buoy_string` | a hyperspace arrival whose system's buoy names an index `STR#` 1000 does not hold, with no `STR ` of it + 999, shows an empty line for 480 ticks in place of the arrival line (`_DisplayMessage` @0x52b3-0x52c7; see [`buoy`](crate::buoy)) | the arrival line, as for a system with no buoy\* |
//! | [`PatrolUninhabited`](RuleKey::PatrolUninhabited) | `patrol_uninhabited` | an interceptor's patrol, picking plain stellars only, can pick an uninhabited one: the count that lets it pick needs an inhabited one, but the pick does not test for one (`_SelectRandomStellarDest` @0x808b6, @0x80911-0x80940; see [`destination`](crate::traffic::destination)) | it leaves uninhabited stellars out too\* |
//! | [`GateExitCounts`](RuleKey::GateExitCounts) | `gate_exit_counts` | a mission ship that leaves through a hypergate or wormhole counts nothing, and comes back at the next setup: `_HandleShipDisplay` frees its slot (@0x2c078) without the end of a warp out's count (`_LowLevelAIHandler` @0x85960-0x859aa; see [`gate_entry`](crate::traffic::gate_entry)) | it is counted as one that jumped out\* |
//! | [`MoveLeftBehind`](RuleKey::MoveLeftBehind) | `move_left_behind` | the ships an in-flight `M` or `N` leaves behind stay in the ship table until that frame's display pass frees them (`_EvalSetExp` @0x15807, @0x1590b; `_HandleShipDisplay` @0x2ded6-0x2dee7): through the next traffic tick they hold their berths and persons, a move back brings them back as they were, and no aux ship of theirs is given back ([`Traffic::retarget`](crate::Traffic::retarget)) | they are dropped at the move, their aux ships given back as a setup gives them back (`_DeleteAllComputerShips` @0x3bfce-0x3c028)\* |
//! | [`SpawnReserve`](RuleKey::SpawnReserve) | `spawn_reserve` | a spawn searches only the slots below its reserve: the system's traffic, the fighters, the mission ships and the hired escorts stop at 55 NPCs, the old ship of a capture at 62 (`_GenericRandomShipSpawn` @0x3c8b1-0x3c8f0, @0x41ce9, @0x3d648, @0x3cd50, @0x3fe73, @0x41136; [`Traffic::free_berth`](crate::Traffic::free_berth)) | no slot is held back: each may take any of slots 1-63, as "Max Ships In System" is 64 |
//! | [`ArrivalHits`](RuleKey::ArrivalHits) | `arrival_hits` | a ship gliding in, waiting in a gate or bursting out of one answers a hit as a flying ship does: it takes the attacker as its target and the damage as provocation, a wait in a gate is cut to 20, and the player's hit on a ship waiting in a gate holds it there, seen, attacking the player (`_DamageShip` @0x3b4b0-0x3b4c2, @0x3be66-0x3be7c; `_AIMakeShipAttackPlayer` @0x89d3e-0x89d4d; see [`Traffic::tick_in`](crate::Traffic::tick_in)) | it ignores every hit until it has arrived\* |
//! | [`HireNoSlot`](RuleKey::HireNoSlot) | `hire_no_slot` | a hire that finds no ship-table slot below the spawn reserve still takes the fee and draws the class's roll again, and no escort joins, as `_DoShipyardDialog` ignores `_SpawnEscort`'s -1 (@0x5f1f1-0x5f221; [`Session::hire`](crate::Session::hire)) | the hire is refused and nothing changes\* |
//! | [`AsteroidEntryField`](RuleKey::AsteroidEntryField) | `asteroid_entry_field` | every way into a system kills the asteroid field, a jump's arrival as well, as each caller of `_CreateAsteroids` sets `_killAsteroids` first (@0x6c4fc, @0x3fa05): the field fills one asteroid a tick from the edges (see [`asteroid`](crate::asteroid)) | a jump's arrival, and a session starting or loading in flight, keeps the field `_CreateAsteroids` places\* |
//! | [`AsteroidRespawnRing`](RuleKey::AsteroidRespawnRing) | `asteroid_respawn_ring` | an asteroid spawned at the edge starts at least 0.7 × (half the play area + 128) out on both axes (`_SpawnAsteroid` @0x3a351-0x3a538; [`Ring::Original`](crate::asteroid::Ring::Original)), which can be on screen on a wide play area | just off the screen, inside the cull ([`Ring::Outside`](crate::asteroid::Ring::Outside))\* |
//! | [`FuseIgnoresAsteroids`](RuleKey::FuseIgnoresAsteroids) | `fuse_ignores_asteroids` | every armed proximity fuse is set off by the first asteroid in reach, as no code reads `Flags2` 0x0004 (`_CheckShotProximities` @0x376ab-0x37758; [`AsteroidFuse::Original`](crate::combat::asteroid_hit::AsteroidFuse::Original)) | the fuse of a weapon with `Flags2` 0x0004 ignores asteroids: "proximity detonator ignores asteroids" ([`AsteroidFuse::Bible`](crate::combat::asteroid_hit::AsteroidFuse::Bible)) |
//!
//! \* The Bible says nothing of `long_advice`, `escort_orders`,
//! `fighter_launch`, `fighter_recall`, `hire_require`, `take_off_pay`,
//! `hire_fee`, `escort_wage`, `grant_max`, `person_join`, `invalid_map`,
//! `remove_refund`, `refit_reserves`, `purchase_paint_order`,
//! `capture_hook_order`, `start_ship_purchase`, `ship_change_cargo`,
//! `ship_change_reserves`, `ship_change_bay_ammo`,
//! `ship_change_default_weapons`, `ship_change_negative_stock`, `move_arrival`,
//! `move_keep_flag`, `script_sound`, `ship_info_rounding`,
//! `stale_secondary`,
//! `secondary_sale`, `landing_distance`, `repeat_mission_failure`,
//! `start_active_mission`, `pay_millions`, `person_mission_stellar`,
//! `mission_stellar_draw`, `mission_stellar_distance`,
//! `derelict_setup_speed`, `left_respawn`, `landed_leaves`, `gate_follow`,
//! `hud_cadence`, `alert_blink`, `blessing_refresh`, `explored_by_place`,
//! `route_rewrite`, `scan_outfits_past_junk`, `gate_escorts`,
//! `lost_cargo_share`, `outfit_sale_cargo`, `release_cargo_share`, `capture_cargo`,
//! `leaderless_swap_hold`,
//! `hidden_loss_fails`,
//! `ship_done_once`,
//! `swap_arrives`,
//! `smuggling_ship_name`,
//! `non_strict_speed`, `hunter_bribe`, `ship_overlap`,
//! `quadrant_turret_arc`, `hidden_ships` (it gives `Cost` only as the
//! price), `hidden_ship_hover_box`, `eject_velocity`, `rescue_armor`,
//! `newbie_fuel_refusal`, `missing_buoy_string`, `patrol_uninhabited`,
//! `gate_exit_counts`, `move_left_behind`, `arrival_hits`, `hire_no_slot`,
//! `asteroid_entry_field`, `asteroid_respawn_ring`
//! or `comm_quote`
//! (only that the quote is "displayed in the communications dialog"), and
//! agrees with the engine on `grant_count` and `nebula_explore_once`
//! ("the explored state of the nebulae isn't saved from game to game ...
//! this expression may get evaluated multiple times"): for them, the reading other
//! than the engine's (`"bible"` in the settings) is the intended
//! behaviour, not the engine's bug or quirk, and not anything the Bible
//! says.
//!
//! † The Bible gives an outfit's `BuyRandom` below 1 or above 100 as 100,
//! and a ship's 0 as never, but is silent on a ship's negative value:
//! its reading here takes that as never too.
//!
//! ‡ The Bible says only that a tribbles good "multiplies like tribbles"
//! and a perishable one "gradually decays away", and gives no rates: both
//! readings take the engine's ton every due frame in flight, and only
//! the conditions differ. Both count the mission cargo aboard against the
//! free space, as the engine does; it only matters to the other reading's
//! growth, since its decay ignores the space.
//!
//! § The Bible says nothing of selling a launcher before its ammunition.
//! The other reading is the phase's first wording of the rule (refuse
//! while any of its ammunition is owned), not anything the Bible says.
//!
//! ¶ The Bible gives the multiplier as the number of items, and says the
//! `Max` is unchanged with none. It says nothing of selling. Its reading
//! here refuses a sale only past the maximum it gives after the sale.
//!
//! \*\* The Bible gives `PriceDelta` only as "how much to raise or lower
//! the price", read here as raising the stellar's own price. Under both
//! readings a commodity price of 4 or less is 5, as in the engine
//! (@0x5dcc8-0x5dcce), and the Bible's reading leaves an event on a
//! commodity not traded there unlisted: the Bible is silent on both, so
//! these are the engine's floor and this port's choice.
//!
//! †† The Bible says nothing of cargo on a ship purchase. The other
//! reading is the phase's first wording of the rule (keep the cargo that
//! fits the new ship), not anything the Bible says. Mission cargo is
//! never left behind under either reading, as the engine never cuts it.
//!
//! ‡‡ The Bible says nothing of a `jünk` of negative or zero price. The
//! other reading is the port's own floor of 0, which it had before the
//! engine's reading was found, not anything the Bible says.
//!
//! §§ The Bible says nothing of a limit on buying a map or clean-record
//! outfit. The other reading is the port's earlier behaviour, with no
//! limit, not anything the Bible says.
//!
//! ¶¶ The Bible says nothing of how many tons a click at the exchange
//! trades. The other reading is the port's earlier behaviour, one ton a
//! click, not anything the Bible says.
//!
//! \*\*\* The Bible says nothing of how the exchange divides the cash by
//! the price. The other reading is the port's earlier behaviour, exact
//! integer division, not anything the Bible says.
//!
//! ††† The Bible says nothing of what Option does at the exchange. The
//! other reading is the port's earlier behaviour, Alt trading the most at
//! once with no dialog, not anything the Bible says.
//!
//! ‡‡‡ The Bible says nothing of buying with cash below 0. The other
//! reading is the port's earlier behaviour, cash below 0 counting as
//! none, not anything the Bible says.
//!
//! §§§ The Bible says nothing of what an outfit sells back for. The other
//! reading is the port's earlier behaviour, half the price whenever it
//! was bought, not anything the Bible says.
//!
//! ¶¶¶ The Bible says nothing of what Option does at the outfitter. The
//! other reading is the port's earlier behaviour, Alt changing nothing,
//! not anything the Bible says.
//!
//! \*\*\*\* The Bible says nothing of when a sale is refused for the free
//! mass. The other reading is the port's earlier behaviour, any sale
//! refused that would leave the free mass below 0, not anything the Bible
//! says.
//!
//! †††† The Bible says only that a persistent outfit "stays with you when
//! you trade ships", and nothing of an unsellable one in a trade-in. The
//! other reading is the port's earlier behaviour, leaving unsellable
//! outfits out of the trade-in too, not anything the Bible says.
//!
//! ‡‡‡‡ The Bible says nothing of when Sell is enabled or how a refused
//! sale is worded. The other reading is the port's earlier behaviour,
//! Sell greyed and the refusal worded in the info box, not anything the
//! Bible says.
//!
//! §§§§ The Bible gives the price as the ship class's `Mass` times the
//! item's `Cost`, and says nothing of a `Cost` or `Mass` of 0 or less.
//! Its reading here is that product never below 0, the port's earlier
//! price, so a negative `Cost` on a class of negative `Mass` is priced
//! at the positive product.
//!
//! ‡‡‡‡‡ The Bible gives the mass as the ship class's `Mass` times the
//! item's `Mass`, divided by 100, for positive-mass items ("Only works
//! for positive-mass items"), and gives no floor. Its reading here is
//! that product, the port's earlier mass, so a flagged outfit can weigh
//! less than its own `Mass`, and 0 or a negative mass on a class of
//! `Mass` 0 or less.
//!
//! ¶¶¶¶ The Bible says nothing of a counted sale. The other reading is
//! the port's earlier behaviour, each pass of a counted sale checked as a
//! single sale, not anything the Bible says.
//!
//! ††††† The Bible says nothing of a counted sale. The other reading is
//! the port's earlier behaviour, the ordered outfit sold for each pass
//! whatever the listing does, not anything the Bible says. By the engine
//! reading the port also ends an order once it has restarted 512 times,
//! where the original has no guard and would sell forever: a deliberate
//! divergence that only a plug-in `OnSell` granting the outfit back can
//! reach. The ceiling is the only guard, with no repeat check, as
//! `OnSell`'s random draws and state outside any cheap key can end an
//! order that revisits an earlier restart's state.
//!
//! §§§§§ The Bible says nothing of the outfitter's selection. The other
//! reading is the port's earlier behaviour, the selected cell kept and
//! the outfitter left open after any order, not anything the Bible says.
//!
//! ¶¶¶¶¶ The Bible says nothing of the outfitter's grid. The other
//! reading is the port's earlier behaviour, a press on an empty cell
//! ignored and the description read whenever the highlighted cell shows
//! an outfit not already described, not anything the Bible says.
//!
//! \*\*\*\*\* The Bible says only that a persistent outfit "stays with
//! you when you trade ships", as the engine has it. The other reading is
//! the port's earlier behaviour, its first shipyard's carry, not anything
//! the Bible says.
//!
//! \*\*\*\*\*\* The Bible says nothing of days passing in port. The
//! other reading is the port's earlier behaviour, no days passing, not
//! anything the Bible says.
//!
//! †††††† The Bible says nothing of when the events a script raises are
//! handled. The other reading is the port's earlier after-settle order,
//! a script's moves applied once it has settled, not anything the Bible
//! says.
//!
//! ‡‡‡‡‡‡ The Bible says only that `BuyRandom` and `HireRandom` are a
//! percent chance, nothing of when they are drawn. The other reading is
//! the port's earlier behaviour, the rolls drawn afresh after each
//! landing, not anything the Bible says.
//!
//! \*\*\*\*\* Where the original's scan finds a stellar but no stellar
//! passes its draw (every stellar the scan passes is a station, and no
//! other is drawable), the original redraws `Rand(2048)` forever; the
//! engine's reading stops there and gives the origin, drawing nothing,
//! as the other reading does.
//!
//! §§§§§§ The Bible gives `Random` as a percent chance and says nothing
//! of values below 0 or above 100. Its reading here never starts a crön
//! of `Random` 0 or less and always starts one of 100 or more. Both
//! readings draw one roll per inactive crön, so switching readings never
//! shifts later draws.
//!
//! ¶¶¶¶¶¶ The Bible says only that a date field of 0 or -1 matches any,
//! and nothing of how the range is compared. The other reading is the
//! port's choice, not anything the Bible says: each end compared at the
//! precision it gives, a yearless range allowed to cross the year end
//! (see [`cron`](crate::cron)).
//!
//! \*\*\*\*\*\*\* The Bible names the government whose stellars show a
//! cron's news and says nothing of its allies. The other reading takes
//! that government only; it is the port's choice, not anything the Bible
//! says (see [`news`](crate::news)).
//!
//! ††††††† The Bible says a rank's sweep flags clear the ranks of "this
//! same govt" and says nothing of a rank of `AffilGovt` -1. The other
//! reading gives such a rank no government; it is the port's choice, not
//! anything the Bible says (see [`rank`](crate::rank)).
//!
//! ‡‡‡‡‡‡‡ The Bible calls `SalaryCap` the "maximum amount of money the
//! player can have"; the other reading pays only up to it. A fee (a
//! negative `Salary`) stops at the cap too under both readings, since
//! nothing is paid at or above it (see [`rank`](crate::rank)).
//!
//! §§§§§§§ The Bible gives a rank's flags to its affiliated government
//! and says nothing of that government's allies. The other reading
//! reaches only ships of the rank's own government, so an unaffiliated
//! rank reaches no ship; it is the port's choice, not anything the Bible
//! says (see [`rank`](crate::rank)).
//!
//! ¶¶¶¶¶¶¶ The Bible says a replacement system shares its original's
//! coordinates and does not say which of two visible systems at one place
//! wins. The other reading is story-runtime phase 4's, the port's choice,
//! not anything the Bible says (see [`story`](crate::story)).
//!
//! \*\*\*\*\*\*\*\* The Bible gives `Visibility` as a test and does not
//! say when it is evaluated. The other reading is the port's choice, not
//! anything the Bible says. Crons-news-ranks phase 16's nebula `ActiveOn`
//! follows the same key (see [`story`](crate::story)).
//!
//! ‡‡‡‡‡‡‡‡ The Bible gives `PriceMod` as the percentage the rank's
//! government charges for "items and ships". The other reading takes that
//! wording, so the outfitter's buy price takes the factor too; an outfit's
//! resale is unchanged (see [`outfitter`](crate::outfitter)).
//!
//! §§§§§§§§ The Bible says nothing of the trade-in under a rank. The other
//! reading, the factor once, is the port's choice, not anything the Bible
//! says (see [`shipyard`](crate::shipyard)).
//!
//! ¶¶¶¶¶¶¶¶ The Bible says nothing of when a rank's `PriceMod` takes
//! effect. The other reading, live prices, is the port's choice, not
//! anything the Bible says (see [`rank`](crate::rank)).
//!
//! \*\*\*\*\*\*\*\*\* The Bible says 0x0040 takes the rank on "any
//! crime against the affiliated government", and 0x0004 on disabling or
//! destroying "a ship of the affiliated government or its allies". The
//! other reading takes that wording; that an unaffiliated rank is never
//! taken is the port's choice, not anything the Bible says (see
//! [`rank`](crate::rank)).
//!
//! †††††††† `cron_zero_duration_holdoff` and `cron_post_holdoff`, with
//! `breakup_stall` (††††††††††) and `unlisted_stellar` (†††††††††††), are the only keys whose default is the
//! other reading, not the engine's ([`RuleKey::default_source`]): for
//! the two cron keys the user decided so on 2026-10-11, so
//! that the rare border-change storylines of stock crons 184, 185 and
//! 192 re-arm after their `PostHoldoff` and their news comes down. The
//! Bible gives `PostHoldoff` as the days after the end before the crön
//! can fire again, and says nothing of the rest; the other reading is
//! the intended end the cron pass builds (see [`cron`](crate::cron)).
//!
//! ††††††††† The Bible says a nebula's explored state is "recalculated
//! every time based on the systems the player has explored", which covers
//! a new pilot's start system: the other reading of `nebula_new_pilot`
//! takes that wording (see [`nebula`](crate::nebula)).
//! †††††††††† `breakup_stall` follows the other reading by default too:
//! the original's stall leaves a player whose ship has a `DeathDelay` of
//! 0 or less breaking up for good, a soft-lock with no way on, and no
//! stock ship is touched by either reading (the lowest stock `DeathDelay`
//! is 3). The Bible says nothing of it; the other reading is the
//! countdown without the original's per-frame reset.
//!
//! ††††††††††† `unlisted_stellar` follows the other reading by default too:
//! the original moves the pilot into the first system that exists, one
//! that doesn't hold the stellar the pilot was saved at, while the saved
//! system is where the pilot last was. `STR#` 140 #52 warns of the
//! missing data under both readings. The Bible says nothing of it.
//!
//! # Adding a rule
//!
//! 1. Add the variant, its doc comment and its settings key to the
//!    `rule_keys!` table below, a row to the table above, and the variant
//!    to this module's key-list test. That is the only list:
//!    [`RuleKey::ALL`], [`RuleKey::key`] and [`RuleKey::from_key`] come
//!    from it, and the settings file reads the key with no further change.
//!    A new key defaults to the engine's reading. A recorded exception
//!    goes in [`RuleKey::default_source`] and in the default test's list
//!    of exceptions.
//! 2. Where the rule decides, read `rules.source_for(RuleKey::X)`
//!    ([`Rulebook::source_for`]) from the rule set the code already holds:
//!    the one the session or a shop builder holds, or the rulebook a
//!    strategy port is built `from_rulebook` with (as
//!    [`NovaLaw::from_rulebook`](crate::NovaLaw::from_rulebook) does).
//!    Every layer already carries the whole rule set, so nothing is
//!    threaded.
//! 3. Test both readings, [`RuleSource::Engine`] and [`RuleSource::Bible`],
//!    building the rule set with
//!    `Rulebook::default().with_override(RuleKey::X, RuleSource::Bible)`.
//!
//! The per-layer reaches tests and the settings test loop over
//! [`RuleKey::ALL`], so they cover the new key unchanged. At the edge of
//! the program, the one rulebook the settings give (see
//! `nova::rulebook`) is the rule set every layer reads, and every
//! strategy port is built from it.

/// Which of the two a disputed rule follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RuleSource {
    /// What the original `EV Nova` executable does: the default.
    #[default]
    Engine,
    /// What the Nova Bible says.
    Bible,
}

impl RuleSource {
    /// Both, the engine's first.
    pub const ALL: [Self; 2] = [Self::Engine, Self::Bible];

    /// The source's name in the settings: `engine` or `bible`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Engine => "engine",
            Self::Bible => "bible",
        }
    }

    /// The source named `name` (see [`RuleSource::name`]), if any.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|source| source.name() == name)
    }
}

/// Declares [`RuleKey`]: each variant with its doc comment and its
/// settings key, the one list of the disputed rules.
macro_rules! rule_keys {
    ($($(#[$doc:meta])* $variant:ident => $key:literal,)+) => {
        /// A rule where the Bible and the engine disagree (see the module
        /// docs).
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum RuleKey {
            $($(#[$doc])* $variant,)+
        }

        impl RuleKey {
            /// Every disputed rule, in the order declared.
            pub const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            /// The rule's key in the settings' `rule_overrides`.
            #[must_use]
            pub const fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => $key,)+
                }
            }
        }
    };
}

rule_keys! {
    /// Which governments a crime against a ship improves the player's
    /// record with: by the engine, every government not allied with the
    /// victim's; by the Bible, only its enemies (see
    /// [`legal`](crate::legal)).
    CrimeGains => "crime_gains",
    /// Boarding a ship whose `düde` `Booty` is 0: by the engine, the
    /// plunder dialog opens anyway; by the Bible, the player is repelled
    /// (see [`board`](crate::board)).
    EmptyBooty => "empty_booty",
    /// Capturing with a ship of no crew: by the engine, the odds are held
    /// at 1 or more; by the Bible, it cannot capture (see
    /// [`board`](crate::board)).
    CrewlessCapture => "crewless_capture",
    /// Which NPCs come to the help of a ship the player fires on or
    /// boards: by the engine, warships and interceptors; by the Bible,
    /// interceptors only, the "piracy police" (see
    /// [`react`](crate::ai::react)).
    PiracyPolice => "piracy_police",
    /// What a quiet government's ships (`gövt` `Flags2` 0x0008, or
    /// their ship type's inherent government's) answer when hailed: by
    /// the engine, Greetings as usual, while Request Assistance and Beg
    /// For Mercy do nothing; by the Bible, "No response." to Greetings,
    /// and the middle button as usual (see [`hail::nova`](crate::hail::nova)).
    QuietHails => "quiet_hails",
    /// An advice line exactly 42 characters long: by the engine, it reads
    /// "Nice to meet you.", as the original mistakes its length for a
    /// `*`; otherwise it is shown as written. The Bible is silent here, so
    /// the other reading is the intended behaviour, not the engine's bug
    /// (see [`hail::reply`](crate::hail::reply)).
    LongAdvice => "long_advice",
    /// What the player's escort with no standing command does: by the
    /// engine, it keeps formation and fires its turrets at a ship
    /// threatening the player; by the Bible (`shïp` `InherentAI`), it
    /// flies as its AI type would, a warship or interceptor attacking
    /// such a ship with every weapon (see [`ai::escort`](crate::ai::escort)).
    EscortAi => "escort_ai",
    /// What becomes of the escorts' standing orders when the player
    /// enters a system (a jump's arrival, a take-off, a pilot loaded): by
    /// the engine, every one is reset to formation; otherwise they are
    /// kept. The Bible is silent here, so the other reading is the
    /// intended behaviour, not anything the Bible says (see
    /// [`Session::with_rules`](crate::Session::with_rules)).
    EscortOrders => "escort_orders",
    /// What a fighter the player launches from a bay does first: by the
    /// engine, it takes the standing order of the first ship of its class
    /// already in the fleet (Return to Hangar excepted) and no target, so
    /// it attacks the player's target once commanded; otherwise it
    /// attacks the player's target at once, as the help page has it
    /// ("choose a target and then launch your fighters"). The Bible is
    /// silent here, so the other reading is the intended behaviour, not
    /// anything the Bible says (see
    /// [`Session::with_rules`](crate::Session::with_rules)).
    FighterLaunch => "fighter_launch",
    /// What becomes of the player's fighters out when it leaves the
    /// system: by the engine, those whose ship type holds a jump's fuel
    /// follow it through a jump and the rest are abandoned, and landed
    /// they stay out; otherwise every one goes back into its bay at once
    /// on each arrival and landing, as the original's uncalled
    /// `_InstantFighterRecall` does. The Bible is silent here, so the
    /// other reading is the intended behaviour, not anything the Bible
    /// says (see
    /// [`Session::with_rules`](crate::Session::with_rules)).
    FighterRecall => "fighter_recall",
    /// Whether a ship's `Require` gates hiring it in the bar: by the
    /// engine, it does not (`_CalcShipCanBuy`'s hire branch never checks
    /// it), and an unmet `Require` only hides the ship when its `Flags3`
    /// has 0x0200; otherwise an unmet `Require` makes a listed ship
    /// greyed and refused, as it does in the shipyard. The Bible speaks
    /// of `Require` only for purchase, so the other reading is the
    /// intended behaviour, not anything the Bible says (see
    /// [`Session::with_rules`](crate::Session::with_rules)).
    HireRequire => "hire_require",
    /// Whether leaving the spaceport costs the hired escorts' wages for a
    /// day: by the engine, each take-off pays every hired escort a day's
    /// wage, an escort the player cannot pay defecting
    /// (`_DoEscortLand` @0x40853); otherwise wages are paid only for a
    /// jump's days; the days a stay passes ([`RuleKey::StayDays`]) pay
    /// none. The Bible is silent here, so the other reading is the
    /// intended behaviour, not anything the Bible says (see
    /// [`Session::with_rules`](crate::Session::with_rules)).
    TakeOffPay => "take_off_pay",
    /// Whether days pass in the spaceport: by the engine, leaving it
    /// passes, through the daily step, half as many days as escorts sold
    /// or upgraded there, rounded down (`_DoEscortLand` @0x40849), then,
    /// after their day's pay, a day if an outfit was bought or sold
    /// (`_DoPortDialog` @0x601fa), four if a ship was bought (@0x6020e),
    /// and the take-off's own day (`_PlayerLandOnStellar` @0x63440);
    /// otherwise a stay passes no days, and only a jump's and a gate's
    /// like a jump pass. The Bible says nothing of it, so the other
    /// reading is the port's earlier behaviour, not anything the Bible
    /// says (see [`Session::take_off`](crate::Session::take_off)).
    StayDays => "stay_days",
    /// When an event a script raises is dispatched (see the
    /// [`story`](crate::story) module's "Dispatch"): by the engine, depth
    /// first, before the script's next operator, as `_EvalSetExp`
    /// (@0x150fc) handles `S` (`_DoMissionAccept` @0x15407) and an `M` or
    /// `N` in flight (@0x159a3, @0x159a8) before going on; the moves a
    /// script queues are applied there too. Otherwise the events wait
    /// until the script has settled
    /// ([`Session::settle_script`](crate::Session::settle_script)), then
    /// dispatch first in, first out. Either way at most 64 events are
    /// dispatched from one settling. The Bible says nothing of it, so the
    /// other reading is the port's earlier after-settle order, not
    /// anything the Bible says.
    StoryReentry => "story_reentry",
    /// Whether `Y` destroys an invincible stellar, one whose `Strength` is
    /// 0 or -1: by the engine, it does not, as the original's
    /// `_StellarIsDestroyed` holds a stellar destroyed only when its
    /// `Strength` is above 0 (@0x4e1a), so the countdown `Y` sets
    /// (@0x15ac8-0x15ade) leaves it landable and drawn whole, and its next
    /// day restores it (@0xb700); by the Bible, whose `Y` is "destroy
    /// stellar ID xxx" whatever its `Strength`, which it calls invincible
    /// only to planetary weapons, it is destroyed (see the
    /// [`story`](crate::story) module's "Galaxy overlay").
    DestroyInvincible => "destroy_invincible",
    /// When the shop and hire day rolls are drawn: by the engine, once a
    /// day in the daily step, every ship class's hire and buy roll then
    /// every outfit's (`_IncrementGameTime` @0xb7c4-0xb833), and a landing
    /// keeps them, since no landing path writes one; otherwise a landing,
    /// and a relocation, forget them and each is drawn the first time a
    /// list asks for it. The Bible says nothing of it, so the other
    /// reading is the port's earlier landing-time draw, not anything the
    /// Bible says (see the [`story`](crate::story) module's "Built by
    /// phase 6").
    DayRolls => "day_rolls",
    /// Whether `Y` runs the stellar's `OnDestroy`: by the engine, it does
    /// not, as the original's `Y` (`_EvalSetExp` @0x15a38-0x15ae6) never
    /// reads it, and only a shot's destruction runs it
    /// (`_CheckShotProximities` @0x374a9); by the Bible, whose `OnDestroy`
    /// is "evaluated when the stellar is destroyed", `Y` runs it as it
    /// destroys a whole stellar, after the destruction, and not when the
    /// stellar was destroyed already or stays whole (an invincible one
    /// under [`RuleKey::DestroyInvincible`]'s engine reading; see the
    /// [`story`](crate::story) module's "Built by phase 8").
    YOnDestroy => "y_on_destroy",
    /// What a destroyed stellar whose `DeadType` has no readable sheet
    /// (-1, outside 0 to 255, no `spïn`, or an `rlëD` that cannot be
    /// read) draws: by the engine, from the next rebuild point until a
    /// flight frame sees it whole, graphic type 0 (`spïn` 1000), the
    /// sheet the original's rebuild gives a sprite slot it left empty
    /// (`_HandleStellarSystemVisibility` @0x32f0f-0x32f2f,
    /// @0x33244-0x332d0), which its per-frame sprite tick replaces only
    /// once it finds the stellar whole (`_HandleStellarSprites`
    /// @0x2e77b-0x2e7e5); by the Bible, whose `DeadType` -1 is "Don't
    /// display different graphic type when destroyed", its `Type` sheet
    /// throughout (see the [`story`](crate::story) module's "Built by
    /// phase 9").
    DeadTypeFallback => "dead_type_fallback",
    /// How close the player's ship must be to a stellar to land on it,
    /// dock at it or enter it: by the engine, strictly within trunc(1.75 ×
    /// the width) of its centre on each axis, as the original's dock
    /// request (`_HandlePlayerDockRequest` @0x66cc7-0x66cfd) checks both
    /// for landing and for which refusal it gives (@0x6773a-0x677b2), the
    /// width being `_GetStellarSpriteXSize`'s (@0xac91-0xad2d): the `Type`
    /// frame's, 150 while the `Type` master is not loaded, 32 when its
    /// `spïn` gives no frame count, and the reach 75 when the stellar's
    /// slot shows no frame; otherwise half the larger side of the `Type`
    /// frame, as a circle, or 32 with no sprite, the port's earlier
    /// radius. The Bible says nothing of a landing distance, so the other
    /// reading is the intended behaviour, not anything the Bible says
    /// (see the [`story`](crate::story) module's "Built by phase 10").
    LandingDistance => "landing_distance",
    /// What hiring an escort takes from the cash: by the engine, a tenth
    /// of the hire price worked out in floating point and the cash cut
    /// down to whole credits, which can take one credit more than the
    /// fee shown (`_DoShipyardDialog` @0x5f168-0x5f1d1); otherwise
    /// exactly the fee shown. The Bible gives no fee, so the other
    /// reading is the intended behaviour, not anything the Bible says
    /// (see [`NovaHire`](crate::NovaHire)).
    HireFee => "hire_fee",
    /// Which wage a hired escort is paid each day, and shows when hailed:
    /// by the engine, the wage its ship type's record gives now, worked
    /// out from its `Cost` on every pay day as `_DoEscortPayment` does;
    /// otherwise the wage kept on its record when it was hired, as the
    /// phase has it. The Bible is silent here, so the other reading is
    /// the intended behaviour, not anything the Bible says (see
    /// [`Session::with_rules`](crate::Session::with_rules)).
    EscortWage => "escort_wage",
    /// How often a person (`përs`) takes an ordinary ship's place, and
    /// which: by the engine, a draw of `Rand(7)` of 0, then `Rand(1022)`
    /// landing on a person who may appear (`_SpawnPerson` @0x40c95), so
    /// each person has a fixed 1-in-1022 share; by the Bible ("a 5% chance
    /// that a specific AI-person will also be created"), `Rand(100)`
    /// below 5, then one of the persons who may appear, picked evenly,
    /// taking the ship's place (see [`person`](crate::person)).
    PersonOdds => "person_odds",
    /// Whether a `sÿst` Person slot brings its person: by the engine,
    /// when `Rand(100) + 1` is at most the slot's chance
    /// (`_SetupShipsInSystem` @0x43363-0x43386); by the Bible ("Want to
    /// make a ‘pers’ type ship always appear? Put its ID into one of the
    /// Person fields"), always (see [`person`](crate::person)).
    SystemPersons => "system_persons",
    /// Which systems a system-linked person's `LinkSyst` allows: by the
    /// engine, that system and, by its slip comparing the raw value with
    /// the system's index (@0x409db), the one whose ID is `LinkSyst` +
    /// 128 too; by the Bible, that system only (see
    /// [`person`](crate::person)).
    LinkSystSlip => "link_syst_slip",
    /// What a person's `ShieldMod` scales: by the engine, its shield and
    /// its armour (`_SpawnPerson` @0x40f68-0x40f9e); by the Bible ("How
    /// much to increase/decrease the person's shield capacity"), its
    /// shield only (see [`person`](crate::person)).
    ShieldMod => "shield_mod",
    /// How fast a person of a derelict government (`gövt` `Flags`
    /// 0x0800) starts at system setup: by the engine, at its top speed
    /// along its heading when a setup pass brings it, as the zeroing of
    /// its velocity (`_SetupShipsInSystem` @0x43245-0x4325b) jumps on to
    /// the pass's shared `_AdjustedAccel` (@0x432b0, @0x432eb), and at
    /// rest when its Person slot brings it, the zeroing (@0x43418-0x43443)
    /// coming after that slot's `_AdjustedAccel` (@0x433fe); by the other
    /// reading, at rest either way (see [`spawn`](crate::traffic::spawn)).
    DerelictSetupSpeed => "derelict_setup_speed",
    /// Which persons run below `Coward`% of their shields: by the engine,
    /// only a warship person of a government with `Flags` 0x0010 and no
    /// fleet lead (`_WarshipAI` @0x8be36); by the Bible ("At what percent
    /// of total shield capacity will the person run away from a fight?"),
    /// any warship or interceptor person with no fleet lead (see
    /// [`ai::warship`](crate::ai::warship)).
    PersonCoward => "person_coward",
    /// The credits a boarded person carries: by the engine, half its
    /// `Credits` in thousands (kept in 16 bits), with a draw of as many
    /// more thousands above 2 (`_SetPlunderValues` @0x922f2-0x9238f); by
    /// the Bible ("This many credits, +/- 25%"), its `Credits` times 75 to
    /// 125 % (see [`board`](crate::board)).
    PersonCredits => "person_credits",
    /// When a person's comm quote (`STR#` 7100) is said: by the engine,
    /// as a friendly person's answer to Greetings, in place of its advice
    /// (`_LoadAdvice` @0x91ae7-0x91b55); otherwise, as the phase has it,
    /// in place of the hail's opening line, whatever the person's
    /// attitude. The Bible says only that it is "displayed in the
    /// communications dialog", so the other reading is the intended
    /// behaviour, not anything the Bible says (see
    /// [`Session::with_rules`](crate::Session::with_rules)).
    CommQuote => "comm_quote",
    /// How many outfits boarding a person grants: by the engine,
    /// trunc((50 + `Rand(51)`) x `GrantCount` / 100), at least 1
    /// (`_DoPlunderDialog` @0x9319b-0x931d6), between `GrantCount` / 2 and
    /// `GrantCount`, as the Bible says too; otherwise 1 + `Rand(GrantCount)`,
    /// between 1 and `GrantCount` evenly, as the phase has it. The Bible
    /// agrees with the engine, so the other reading is the intended
    /// behaviour, not anything the Bible says (see
    /// [`grant`](crate::grant)).
    GrantCount => "grant_count",
    /// Whether a grant may take the player past the outfit's `Max`: by the
    /// engine, a boarding grant may, as only an outfit's owned count is
    /// tested before the pick (`_DoPlunderDialog` @0x93119), though its
    /// count is held to the free mass; and `G` may pass both the `Max`
    /// and the free mass, as `_GrantOutfitItem` checks neither (@0x44d4f).
    /// Otherwise both are held to `Max` less the owned and to the free
    /// mass. The Bible is silent here, so the other reading is the
    /// obvious fix, not anything the Bible says (see
    /// [`grant`](crate::grant) and [`OutfitRules`](crate::OutfitRules)).
    GrantMax => "grant_max",
    /// Whether a person offers to join the player when hailed: by the
    /// engine, none does, as the original brings a person to fly with the
    /// player only through its `LinkMission`: a person whose mission is
    /// available offers it in place of the comm dialog
    /// (`_HandlePlayerCommunication` @0x6220d), and an accept of a person
    /// of `Flags` 0x0040 whose mission has one ship left, whatever its
    /// goal, swaps it for that ship (@0x62421-0x6252f;
    /// [`swaps_for_mission_ship`](crate::person::swaps_for_mission_ship)),
    /// which flies in formation with the player as a mission escort when
    /// the mission's `ShipBehav` is 1 (@0x62509, @0x8fc03-0x8fc22).
    /// Otherwise, a person whose record allows
    /// it ([`offers_to_join`](crate::person::offers_to_join)), its
    /// `LinkMission` available, opens the comm dialog listing Use As
    /// Escort while the fleet has room, and joins the fleet as itself,
    /// for good. The Bible is silent here, so the other reading is the
    /// intended behaviour, not anything the Bible says (see
    /// [`JoinFleet`](crate::hail::nova::JoinFleet)).
    PersonJoin => "person_join",
    /// Which systems a map outfit (`ModType` 16) of `ModVal` N above 0
    /// explores, and which systems count as inhabited for a `ModVal` of
    /// -1: by the engine, a depth-first walk from the player's system
    /// that follows each system's Con links in order and never enters a
    /// system twice (`_RecursiveAutoExplore` @0xd942), so a system first
    /// reached by a long way round, too deep to go on, is never entered
    /// again by a shorter one and the systems beyond it within N jumps
    /// are missed; and a system is inhabited when one of its *first four*
    /// stellars is neither uninhabited (`Flags` 0x0020) nor a hypergate
    /// or wormhole (`Flags2` 0x1000, 0x2000) (`_SystemIsInhabited`
    /// @0x4c4a). By the Bible ("how many jumps away from the current
    /// system to explore"), every system within N jumps, and any stellar
    /// of the system counts (see
    /// [`exploration`](crate::exploration)).
    MapExplore => "map_explore",
    /// What becomes of a map outfit whose `ModVal` explores nothing (0,
    /// or -2 to -999): by the engine, it is used up all the same, as any
    /// map slot keeps the outfit from being added (`_GrantOutfitItem`
    /// @0x44ea9); otherwise it is no map, and is added to the outfits as
    /// a plain item. The Bible is silent on those values, so the other
    /// reading is the obvious fix, not anything the Bible says (see
    /// [`outfit_effects`](crate::outfit_effects)).
    InvalidMap => "invalid_map",
    /// What the `D` set operator pays for the outfit it removes: by the
    /// engine, nothing (`_EvalSetExp` @0x1544b only lowers the count);
    /// otherwise what selling the outfit at the outfitter would. The
    /// Bible is silent here, so the other reading is the obvious
    /// alternative, not anything the Bible says (see
    /// [`OutfitRules`](crate::OutfitRules)).
    RemoveRefund => "remove_refund",
    /// What becomes of a shield or armour above its most when `G`, `D`
    /// or a boarding grant or ammunition take refits the ship: by the
    /// engine, it is kept, as each works the stats out through
    /// `_SystemInfoToShipStats` (`_EvalSetExp` @0x15489,
    /// `_DoPlunderDialog` @0x93226), which with
    /// `_ResetPlayerPrecalcedValues` (@0xc357-0xca32) clamps no reserve,
    /// so the surplus lasts until damage, the outfitter closing
    /// (`_DoOutfitDialog` @0x5da4b-0x5dad4) or a reload
    /// (`_LoadPilotData` @0x754dc) takes it; otherwise each is held to
    /// its most. The fuel is held to its most either way, as flight
    /// clamps it every frame (`_HandlePlayer` @0x6d9e1). The Bible is
    /// silent here, so the other reading is the obvious alternative, not
    /// anything the Bible says (see [`OutfitRules`](crate::OutfitRules)).
    RefitReserves => "refit_reserves",
    /// When buying a ship clears its paint: by the engine, after the new
    /// ship's `OnPurchase` runs (`_DoShipyardDialog` @0x5f00d, then
    /// @0x5f022), so a paint that hook grants is lost; otherwise before
    /// it, so the paint stays. The Bible is silent here, so the other
    /// reading is the obvious alternative, not anything the Bible says
    /// (see [`HookRules`](crate::HookRules)).
    PurchasePaintOrder => "purchase_paint_order",
    /// When "Use As My Ship" runs the old ship's `OnRetire` and the
    /// captured ship's `OnCapture`: by the engine, before the outfit swap
    /// (`_DoShipCapture` @0x41662 and @0x416c9, then @0x419dd), so a
    /// non-persistent outfit either grants is stripped; otherwise after
    /// it, so the outfit stays. The Bible is silent here, so the other
    /// reading is the obvious alternative, not anything the Bible says
    /// (see [`HookRules`](crate::HookRules)).
    CaptureHookOrder => "capture_hook_order",
    /// Whether a new pilot's starting ship runs its `OnPurchase`: by the
    /// engine, it does not (`_DoNewPilot` runs only the `chär`'s
    /// `OnStart`, @0x19345); otherwise it runs once, right before
    /// `OnStart`, as though the ship were bought. The Bible is silent
    /// here, so the other reading is the obvious alternative, not
    /// anything the Bible says (see [`HookRules`](crate::HookRules)).
    StartShipPurchase => "start_ship_purchase",
    /// Which outfits survive the `H` set operator's change of ship: by
    /// the engine, any flagged persistent (0x0004) or persistent through a
    /// mission's change of ship (0x0020), as it tests both bits
    /// (`_EvalSetExp` @0x154d5, `testb $0x24`); by the Bible's `oütf`
    /// flag 0x0020 ("persistent in the case where the player's ship is
    /// changed by a mission set operator", 0x0004 governing buying and
    /// capture), only those flagged 0x0020 (see
    /// [`ShipChangeRules`](crate::ShipChangeRules)).
    ShipChangePersistence => "ship_change_persistence",
    /// Whether the `E` and `H` set operators hold the outfits owned to
    /// their limits: by the engine, every outfit, old and new, is clamped
    /// to its limit once the new class's default items and stock weapons
    /// are added (`_HasMaxOfItem` @0x4512, called @0x15648): its
    /// ammunition cap, its raised `Max`, or the new class's gun or turret
    /// slots (see [`ship_change`](crate::ship_change)); by the Bible's
    /// "the player will keep all of his previous outfit items", nothing is
    /// clamped (see [`ShipChangeRules`](crate::ShipChangeRules)).
    ShipChangeMax => "ship_change_max",
    /// What becomes of the cargo a `C`, `E` or `H` change of ship leaves
    /// past the new hold. By the engine, it is trimmed as the set
    /// expression ends: the change never calls `_DestroyPartialFleetCargo`
    /// (`_EvalSetExp` @0x15493-0x156da), but its `_SystemInfoToShipStats`
    /// (@0x1566a) and the set-string wrapper's tail (@0x99a85, @0x99dea)
    /// end in `_ResetPlayerPrecalcedValues`, whose trim (@0xc7d8-0xc887)
    /// cuts each commodity to trunc(held x holds/total) in single floats,
    /// against the fleet's holds with the trader escorts' `Holds`, the
    /// mission cargo aboard counted and never cut, the `jünk` never cut.
    /// By the other reading every ton is kept past the hold: no set
    /// expression's reset trims, whatever the expression holds, and the
    /// cargo waits for a reset outside one, a take-off, a jump's arrival,
    /// a landing or a jettison among them. The Bible is silent here, so
    /// the other reading is the obvious alternative, not anything the
    /// Bible says (see [`ShipChangeRules`](crate::ShipChangeRules) and
    /// the session's `precalced` module).
    ShipChangeCargo => "ship_change_cargo",
    /// What becomes of a shield or armour above the new class's most on a
    /// `C`, `E` or `H` change of ship: by the engine, it is kept, as the
    /// change clamps nothing (`_EvalSetExp` @0x15493-0x156da) and flight
    /// regenerates the shield and armour only below their most, clamping
    /// the fuel alone (`_HandlePlayer` @0x6cf99, @0x6d01d, @0x6d9e1), so
    /// the surplus lasts until damage or landing takes it; otherwise each
    /// is held to its new most. The Bible is silent here, so the other
    /// reading is the obvious alternative, not anything the Bible says
    /// (see [`ShipChangeRules`](crate::ShipChangeRules)).
    ShipChangeReserves => "ship_change_reserves",
    /// Where the `E` and `H` set operators put a fighter bay's
    /// `AmmoLoad` when they add the new class's stock weapons: by the
    /// engine, on the weapon its raw `AmmoType` names when that is 0 to
    /// 255, whatever its guidance, as every weapon's load goes
    /// (`_EvalSetExp` @0x1551c-0x155de), so a bay whose fighter's `shïp`
    /// ID is 128 to 255 loads `wëap` 128 + that ID, which no outfit
    /// holds, and its fighters are lost; otherwise on the bay's own
    /// fighters, as a purchase loads them (`_DoShipyardDialog`
    /// @0x5ef3e-0x5ef56). The Bible is silent here, so the other reading
    /// is the obvious alternative, not anything the Bible says (see
    /// [`ShipChangeRules`](crate::ShipChangeRules)).
    ShipChangeBayAmmo => "ship_change_bay_ammo",
    /// What becomes of a new class's default item that is a weapon or
    /// ammunition outfit when the `E` and `H` set operators add it: by
    /// the engine, the default items are added to the outfits alone and
    /// not to the weapon and rounds arrays, so the fold that follows
    /// (`_ShipStatsToSystemInfo` @0xcaee, called @0x15622) pulls the
    /// outfits whose first mod names a weapon back to the arrays' count,
    /// and the default is lost unless the array has a surplus left;
    /// otherwise the default items' first-mod weapons and rounds are added
    /// to the arrays too, so they are kept. The Bible is silent here, so
    /// the other reading is the obvious alternative, not anything the
    /// Bible says (see [`ship_change`](crate::ship_change) and
    /// [`ShipChangeRules`](crate::ShipChangeRules)).
    ShipChangeDefaultWeapons => "ship_change_default_weapons",
    /// What a new class's negative `WeapCount` or `AmmoLoad` does when the
    /// `E` and `H` set operators add its stock weapons: by the engine, it
    /// is added signed (`_EvalSetExp` @0x15555, @0x155c3), so it takes
    /// that many launchers or rounds away from the outfits holding them,
    /// floored at none, through the fold (`_ShipStatsToSystemInfo`
    /// @0xcaee); otherwise it adds nothing, as a purchase reads it. The
    /// Bible is silent here, so the other reading is the obvious
    /// alternative, not anything the Bible says (see
    /// [`ShipChangeRules`](crate::ShipChangeRules)).
    ShipChangeNegativeStock => "ship_change_negative_stock",
    /// Where the `M` set operator puts a ship in flight when the system
    /// moved to has no stellar: by the engine, nowhere new, its position
    /// and velocity kept, as `_EvalSetExp` places the ship only on a
    /// stellar it finds (@0x1570e-0x158d4); by the Bible ("in the center
    /// of the system if no stellars exist there"), at rest at the
    /// system's centre (see [`ScriptEffectRules`](crate::ScriptEffectRules)).
    MoveStarless => "move_starless",
    /// What else the `M` and `N` set operators do beyond changing where
    /// the ship is: by the engine, nothing, as `_EvalSetExp` calls neither
    /// `_SetupShipsInSystem` nor `_AutoSetExploration` and leaves
    /// `_hyperRoute` alone, so the course is kept (and J refuses while its
    /// next system is not linked from here), the system is explored only
    /// by the next take-off, and in flight the last system's ships stay
    /// behind but for every ship the player leads (hired and mission
    /// escorts, fighters out), the new
    /// system filling only with arrivals over time; otherwise the move is
    /// an arrival, exploring the system and clearing the course at once,
    /// and in flight populating it at once, the fighters out kept or
    /// abandoned as the fighter rules say. What becomes of the ships
    /// left behind by the engine's reading is
    /// [`RuleKey::MoveLeftBehind`]'s; by the other, populating drops
    /// them. The Bible is silent here, so
    /// the other reading is the intended behaviour, not anything the
    /// Bible says (see [`ScriptEffectRules`](crate::ScriptEffectRules)).
    MoveArrival => "move_arrival",
    /// What the `N` set operator's flag does when it runs in flight: by
    /// the engine, `_dontMovePlayerAfterLanding` (@0x158d9) stays set
    /// until the next take-off, which then keeps the ship where it landed
    /// instead of at the stellar's centre, and clears it
    /// (`_PlayerLandOnStellar` @0x633ad-0x63400); otherwise only a landed
    /// `N` keeps the landing position, and a take-off after an `N` in
    /// flight is from the stellar's centre as any other. The Bible is
    /// silent here, so the other reading is the intended behaviour, not
    /// anything the Bible says (see
    /// [`ScriptEffectRules`](crate::ScriptEffectRules)).
    MoveKeepFlag => "move_keep_flag",
    /// What a `Q` set operator whose `STR#` is missing or empty, or whose
    /// pick is empty, does: by the engine, nothing, as the player leaves
    /// only on a message that is not empty (`_PlayGame` @0x4654c, the port
    /// dialogs' filters such as `_PortFilter` @0x4f041), and it cancels an
    /// earlier `Q`, whose message it overwrites; by the Bible ("make the
    /// player immediately leave"), the player leaves all the same, with no
    /// message (see [`ScriptEffectRules`](crate::ScriptEffectRules)).
    BlankLeave => "blank_leave",
    /// When the `P` set operator's sound plays: by the engine, one
    /// mission sound at a time, as `_missionSoundID` (@0x20fa14) is read
    /// only once a flight frame (`_PlayGame` @0x462fa): `P` is held to the
    /// next flight tick, so a landed `P` sounds after the take-off, a
    /// later `P` replaces it, a jump's arrival drops it, and it is dropped
    /// while the last mission sound is still playing; otherwise every `P`
    /// sounds at once, landed or not, over whatever is playing. The Bible
    /// is silent here, so the other reading is the intended behaviour, not
    /// anything the Bible says (see
    /// [`ScriptEffectRules`](crate::ScriptEffectRules)).
    ScriptSound => "script_sound",
    /// How a `BuyRandom` outside 1-100 reads: by the engine, an outfit's
    /// below 1 is never for sale and a ship's below 0 always, as the
    /// original's loader clamps them (`_LoadObjectData` @0x78b8d,
    /// @0x7a340); by the Bible, an outfit's below 1 or above 100 is
    /// always for sale and a ship's 0 or below never. The Bible is silent
    /// on a ship's negative value, so for it "0 means never" is taken to
    /// cover it (see [`outfitter`](crate::outfitter) and
    /// [`shipyard`](crate::shipyard)).
    BuyRandom => "buy_random",
    /// When held `jünk` with the Tribbles (0x0001) and Perishable
    /// (0x0002) `Flags` grow and decay, a ton each every due frame in
    /// flight: by the engine, on the free space measured once, so
    /// tribbles goods can fill the hold past full, perishable goods decay
    /// only while there is space, and a good with both flags decays only
    /// beside a perishable-only good (`_HandlePlayer` @0x70827-0x7093b);
    /// the free space is the fleet's holds less the goods and the mission
    /// cargo aboard, as `_TotalCargo` (@0x4a0b) counts them; by the Bible, which says only that tribbles goods multiply and
    /// perishable goods "gradually decay away", tribbles goods grow only
    /// into free space and perishable goods always decay. The Bible gives
    /// no rates, so both readings take the engine's ton every due frame
    /// (see [`market`](crate::market)).
    JunkFlags => "junk_flags",
    /// When a launcher outfit (one whose first `ModType` 1 names a
    /// `wëap`) cannot be sold for the ammunition held: by the engine,
    /// only when its weapon's `MaxAmmo` is above 0 and selling it would
    /// leave more rounds than the remaining launchers' `MaxAmmo` holds
    /// (`_DoOutfitDialog` @0x5ca75-0x5cbe0), so in stock data only fighter
    /// bays are ever blocked; by the other reading, whenever any of its
    /// ammunition is held, whatever its `MaxAmmo`. The Bible says nothing
    /// of it, so the other reading is the phase's first wording of the
    /// rule, not anything the Bible says (see
    /// [`outfitter`](crate::outfitter)). In a counted sale, (n - 1) is as
    /// [`CountedSale`](RuleKey::CountedSale) says.
    LauncherSale => "launcher_sale",
    /// How an `oütf` of `ModType` 27 raises the `Max` of the outfit its
    /// `ModVal` names, and when one can be sold. By the engine, the
    /// target's `Max` is multiplied by the sum, over the outfits owned, of
    /// the count owned times that outfit's `ModType` 27 mods naming it, at
    /// least 1 (`_HasMaxOfItem` @0x45e9-0x46c3), so an outfit with two
    /// such mods counts twice; selling one of n raisers is refused while
    /// the target owned exceeds its raw `Max` x (n - 1), other raisers
    /// ignored and with no floor (`_DoOutfitDialog` @0x5c7be-0x5ca6f). By
    /// the Bible, the multiplier is the number of owned items with any
    /// such mod, at least 1, and a sale is refused while the target owned
    /// exceeds that maximum after the sale (see
    /// [`outfitter`](crate::outfitter)). In a counted sale, (n - 1) is as
    /// [`CountedSale`](RuleKey::CountedSale) says.
    RaisedMax => "raised_max",
    /// How an active `öops` event prices its commodity at its stellar. By
    /// the engine, the price is the commodity's `BasePrice` plus the
    /// event's `PriceDelta`, whatever the stellar's level; of several
    /// events on one commodity the highest ID wins, and an event lists a
    /// commodity the stellar does not otherwise trade (`_DoTradeDialog`
    /// @0x5dcf8-0x5dd58). By the Bible, every active event's `PriceDelta`
    /// is added to the stellar's own (level) price, several add up, and an
    /// event on a commodity not traded there moves nothing. Under both, a
    /// standard commodity's price of 4 or less is 5 (see
    /// [`market`](crate::market)).
    EventPrice => "event_price",
    /// What cargo a ship purchase keeps. By the engine, as
    /// `_DoShipyardDialog` writes the new class's stats
    /// (`_SystemInfoToShipStats` @0x5ee99) and then calls
    /// `_DestroyPartialFleetCargo(0)` (@0x5ef9f, @0xcd32-0xcff6), each
    /// ending in `_ResetPlayerPrecalcedValues`: first, when the new
    /// fleet's holds are below everything held, the mission cargo aboard
    /// counted (`_TotalCargo` @0x4a0b, called @0xc7ea), each commodity is
    /// cut by their ratio in single floats, and the mission cargo is not
    /// cut (@0xc7d8-0xc82c); then, with A the new ship's cargo space and B
    /// that plus the `Holds` of the trader escorts, uncapped, each
    /// commodity becomes trunc(held x A/B) and each `jünk` loses
    /// trunc(held x A/B), so without such escorts every `jünk` goes; then
    /// the trim again, which cuts once more whatever `jünk` or mission
    /// cargo still holds the fleet past its holds. By the other
    /// reading, the mission cargo aboard takes the new ship's own cargo
    /// space first and always stays, and the goods that fit in the rest
    /// are kept, in order. The Bible says nothing of it, so the other
    /// reading is the phase's first wording of the rule, not anything the
    /// Bible says (see [`shipyard`](crate::shipyard)).
    PurchaseCargo => "purchase_cargo",
    /// What an outfit sale that leaves the fleet's holds below the cargo
    /// does to it. By the engine, the sale's resets trim the commodities
    /// to the holds left: `_DoOutfitDialog` checks no cargo before a sale
    /// (@0x5ce97-0x5cf37) and ends each in `_SystemInfoToShipStats`
    /// (@0x5cffb, tail @0xcae9), after `OnSell`'s own reset (@0x5cff6,
    /// tail @0x99dea), and `_ResetPlayerPrecalcedValues` cuts each
    /// commodity to trunc(held x holds/total) in single floats, the
    /// mission cargo counted and never cut, the `jünk` never cut
    /// (@0xc7d8-0xc887). By the other reading no reset through the sale
    /// trims, `OnSell`'s included, and the cargo stays past the hold until
    /// the next reset after it (a take-off at the latest), the port's
    /// earlier behaviour. The Bible says nothing of it, so the other
    /// reading is the obvious alternative, not anything the Bible says
    /// (see [`outfitter`](crate::outfitter) and the session's `precalced`
    /// module).
    OutfitSaleCargo => "outfit_sale_cargo",
    /// Which escorts released by hail take their share of the fleet's
    /// cargo. By the engine, any: the release's
    /// `_DestroyPartialFleetCargo` (`_DoCommDialog` @0x96864,
    /// `_DoEscortDialog` @0x9549d, @0x9561d) is called with no
    /// `InherentAI` test, unlike every other loss of an escort, so a
    /// warship or interceptor, whose `Holds` are never in the fleet's,
    /// takes min(1, its `Holds` over the fleet's holds) of every good; a
    /// trader, still in the fleet as it goes, takes its `Holds` over the
    /// fleet's holds with its own among them (@0xcd32-0xcff6). By the
    /// other reading only a trader takes a share. The Bible says nothing
    /// of it, so the other reading is the obvious alternative, not
    /// anything the Bible says (see the session's `escorts` module).
    ReleaseCargoShare => "release_cargo_share",
    /// What becomes of the commodities on Use As My Ship. By the engine,
    /// they stay with the old ship as it joins the fleet:
    /// `_DoShipCapture` copies them into the old ship's slot
    /// (@0x412e6-0x412fe) and sets the player's from the captured ship's
    /// (@0x41762-0x41779), and the engine's ships carry no cargo, so the
    /// player keeps none. The `jünk` and the mission cargo, which are the
    /// player's alone, stay. By the other reading the player keeps every
    /// commodity. Either way the reset of its `_SystemInfoToShipStats`
    /// (@0x41a3e) then trims the commodities to the new fleet's holds,
    /// the old ship's among them when it is a trader. The capture's
    /// `_DestroyPartialFleetCargo` (@0x41438) is in a branch only a second
    /// argument other than 0 reaches, and its one caller
    /// (`_DoPlunderDialog` @0x94515) passes 0, so no share is cut. The
    /// Bible says nothing of it, so the other reading is the obvious
    /// alternative, not anything the Bible says (see
    /// [`Session::assign`](crate::Session::assign)).
    CaptureCargo => "capture_cargo",
    /// What a ship swapped in for a person by a `ShipGoal` 3 mission
    /// does when it neither hunts nor escorts the player. By the engine,
    /// it holds, as the original's goal 10 with no leader: the swap calls
    /// `_AIMakeEscortFlyInForm` (@0x624f6-0x62509), which gives a ship
    /// with leader -1 goal 10 (@0x89bbd-0x89bcd), and no goal block of
    /// `_HighLevelAIHandler` flies goal 10 without a leader
    /// (@0x8ed18-0x8ed31). It coasts with no thrust and no fire, and keeps
    /// the hold until provoked, until its government's word turns it on
    /// the player, or, for a warship of a plundering government, at its
    /// first think (see [`Goal::Hold`](crate::ai::Goal::Hold)). By the
    /// other reading it decides as its `düde`'s AI from the start. The
    /// Bible says nothing of it, so the other reading is the intended
    /// behaviour, not anything the Bible says (see the session's
    /// `mission_ships` module).
    LeaderlessSwapHold => "leaderless_swap_hold",
    /// How a `jünk` of negative or zero price is traded. By the engine,
    /// `_DoTradeDialog` stores a `jünk`'s high price (@0x5dddb) and low
    /// price (@0x5de2f) in 16 bits with no floor, and every reader takes
    /// it signed: a negative price is listed at that price
    /// (`_TradeDialogUpdate` @0x4d545-0x4d54b), a buy of it gets 0 tons
    /// (the `jle` @0x5e27d), a sale of it takes tons x price from the
    /// cash, which can go below 0 (@0x5e543-0x5e546), and a row priced 0
    /// is not listed (`_TradeDialogUpdate` @0x4d2a2-0x4d2ac, `_TradeFilter`
    /// @0x4e262), the row each way on its own. By the other reading, the
    /// price is never below 0, and a row priced 0 is listed and bought
    /// free, limited by space. The Bible says nothing of it, so the other
    /// reading is the port's own floor, not anything the Bible says (see
    /// [`market`](crate::market)).
    JunkPrice => "junk_price",
    /// Which ways a listed `jünk` row trades. By the engine, every row
    /// trades both ways at its own price: the trade buttons ask only
    /// `_CanBuyGoods` (@0xccec, cash for a ton and free space) and
    /// `_CanSellGoods` (@0x4a94, tons held), never which list the row came
    /// from (`_DrawTradeButtons` @0x2938f/0x2939d, `_TrackTradeButtons`
    /// @0x2960d/0x2963e), and a buy or sale on the selected row is priced
    /// at that row's price (@0x5e2c4-0x5e2e2, @0x5e528-0x5e546). By the
    /// Bible, whose `SoldAt` is where "the commodity is sold" and
    /// `BoughtAt` where it "is purchased", the `SoldAt` row is bought only
    /// and the `BoughtAt` row sold only (see [`market`](crate::market)).
    JunkTrade => "junk_trade",
    /// Whether the outfitter sells a map or clean-record outfit only once
    /// an opening. By the engine, an outfit with a `ModType` 16 mod, or
    /// failing that a `ModType` 21 mod, is refused while the last outfit
    /// bought since the outfitter opened was of the same kind
    /// (`_CanBuyOutfitItem` @0x4e8bc-0x4e9c5 reads `_boughtMap` and
    /// `_boughtID`, which `_GrantOutfitItem` sets afresh on every buy,
    /// @0x44d67-0x44f8b, and `_DoOutfitDialog` clears on opening,
    /// @0x5bb37-0x5bb44), unless its first mod is `ModType` 3 or it has a
    /// `ModType` 2 mod. By the other reading, there is no limit. The Bible
    /// says nothing of it, so the other reading is the port's behaviour
    /// before the engine's was found, not anything the Bible says (see
    /// [`outfitter`](crate::outfitter)).
    OutfitLimit => "outfit_limit",
    /// What a sold outfit refunds. By the engine, `_DoOutfitDialog`
    /// copies the counts owned when the outfitter opens (@0x5badd-0x5bafa)
    /// and refunds the full price of an outfit whose count owned before
    /// the sale is above its copy, one bought since then, and any other
    /// at half truncated toward zero (@0x5ceee-0x5cf08, the 0.5 at
    /// @0xdd128); a counted sale does so item by item. By the other
    /// reading, every sale refunds half. The Bible says nothing of it, so
    /// the other reading is the port's earlier behaviour, not anything the
    /// Bible says (see [`Outfitter::refund`](crate::Outfitter::refund)).
    OutfitRefund => "outfit_refund",
    /// What Option (Alt here) does with Buy or Sell at the outfitter. By
    /// the engine, it asks for a count in the quantity dialog
    /// (`_DoQuantityDialog` @0x56d5b, called from `_DoOutfitDialog`
    /// @0x5c0c0-0x5c1aa and @0x5c5e3-0x5c628), which opens at the most
    /// the order allows ([`Outfitter::count_max`](crate::Outfitter::count_max))
    /// and buys or sells the count confirmed
    /// ([`Session::outfit_counted`](crate::Session::outfit_counted)). By
    /// the other reading, Alt changes nothing: each click moves one. The
    /// Bible says nothing of Option at the outfitter, so the other reading
    /// is the port's earlier behaviour, not anything the Bible says (see
    /// [`Outfitter::count_asked`](crate::Outfitter::count_asked)).
    OutfitCount => "outfit_count",
    /// When a sale at the outfitter is refused for the free mass. By the
    /// engine, `_DoOutfitDialog` checks only an outfit of negative mass
    /// (@0x5c735-0x5c73e) and refuses it when `_ShipFreeMass`, which
    /// clamps the free mass at 0 (@0xb506-0xb50a), plus its mass is below
    /// 0 (@0x5c740-0x5c747); an outfit of mass 0 or more sells even while
    /// the free mass is below 0. By the other reading, any sale is
    /// refused that would leave the unclamped free mass below 0. The
    /// Bible says nothing of it, so the other reading is the port's
    /// earlier behaviour, not anything the Bible says (see
    /// [`outfitter`](crate::outfitter)).
    SaleMass => "sale_mass",
    /// When Sell is enabled at the outfitter, and where a sale refused for
    /// the free mass, a raised `Max` or a launcher's ammunition is worded.
    /// By the engine, the Sell flag alone enables Sell (some owned and not
    /// `CANNOT_SELL`, `_DoOutfitDialog` @0x5c563-0x5c595, `_OutfitFilter`
    /// @0x5808c-0x580b1); those three refuse only inside the sell loop
    /// (@0x5c735-0x5ce6c), whose first refusal of an order shows its
    /// `STR#` 2002 #207-#212 words once (flag -0x63e, @0x5c6ca) in
    /// `DLOG` 3003 (`_ShowTextDialog` @0x5a8cf), and the info box never
    /// words them. By the other reading, any of them greys Sell and the
    /// info box words the first, with no dialog. The Bible says nothing
    /// of it, so the other reading is the port's earlier behaviour, not
    /// anything the Bible says (see [`outfitter`](crate::outfitter)).
    SaleRefusal => "sale_refusal",
    /// How each pass of a counted sale checks a raised `Max` and a
    /// launcher's ammunition. By the engine, each pass checks the order as
    /// a whole: `_DoOutfitDialog` refuses a raiser while the target owned
    /// exceeds its raw `Max` x (owned now - the whole count, -0x68c)
    /// (@0x5c82d-0x5c85e), and a launcher while the rounds held exceed
    /// `MaxAmmo` x (owned now - the pass index, -0x63a, - 1)
    /// (@0x5cb9c-0x5cbc7), owned now falling by one with each sale
    /// (@0x5cf22), signed and with no floor; so an Alt sale is refused
    /// sooner than as many single clicks. By the other reading, each pass
    /// is checked as one sale, (owned now - 1). A count of 1 at pass 0 is
    /// the same by both. Only the engine readings of
    /// [`RaisedMax`](RuleKey::RaisedMax) and
    /// [`LauncherSale`](RuleKey::LauncherSale) read it. The Bible says
    /// nothing of it, so the other reading is the port's earlier
    /// behaviour, not anything the Bible says (see
    /// [`outfitter`](crate::outfitter)).
    CountedSale => "counted_sale",
    /// Which outfit, and from which index, the passes of a counted sale
    /// go on after a sale that changes the listing. By the engine, after
    /// each sale `_DoOutfitDialog` rebuilds the list
    /// (`_SetupPortAvailableItems` @0x5d00a), and when it changed
    /// (@0xc216-0xc23d) sets the selected outfit to index 0, `oütf` 128,
    /// listed or not (@0x5d01c), and the loop index -0x63a to 0, or to
    /// 512 when nothing is listed (@0x5d0fe-0x5d120), before its
    /// increment (@0x5d19c-0x5d1b1). So the passes left, from index 1 (or
    /// 513), check and sell `oütf` 128 as any sale is checked and sold,
    /// the Sell flag asking no list (@0x5d164-0x5d199), and an order can
    /// sell more than its count; with 128 not owned they do nothing. By
    /// the other reading, the ordered outfit and the pass index are kept.
    /// A sale that leaves the listing unchanged is the same by both. By
    /// the engine reading the port ends an order once it has restarted
    /// 512 times, where the original, with no such guard, would sell
    /// forever (an `OnSell` that grants the outfit back and relists,
    /// reachable only by plug-ins). The ceiling is the only guard, with
    /// no repeat check, as `OnSell`'s random draws and state outside any
    /// cheap key can end an order that revisits an earlier state. The
    /// Bible says nothing of it, so the other reading is the port's
    /// earlier behaviour, not anything the Bible says (see
    /// [`Session::outfit_order`](crate::Session::outfit_order)).
    RelistedSale => "relisted_sale",
    /// What the outfitter selects after an order that changes the
    /// listing, and whether it stays open. By the engine, when
    /// `_SetupPortAvailableItems` says the list changed (@0xc216-0xc23d),
    /// once after the buy loop (@0x5c314) or after each sale (@0x5d00a),
    /// `_DoOutfitDialog` selects index 0, `oütf` 128, listed or not
    /// (`*0xded58` = 0, @0x5c336, @0x5d01c), highlights no cell and
    /// scrolls the grid to the top (`*0xded5c` = -1, `*0xdeda8` = 0,
    /// @0x5c32c-0x5c33b, @0x5d021-0x5d02b), and empties the description
    /// (@0x5c350-0x5c35b, @0x5d030-0x5d04b). `_OutfitFilter` then sets Buy
    /// and Sell from `oütf` 128 on every event, asking no list
    /// (@0x5806f-0x580ca), and `_OutfitDialogUpdate` draws no picture and
    /// no info box (@0x5789b-0x578a5), until a cell is chosen. With
    /// nothing listed it sets the leave flag -0x63d (@0x5c40c, @0x5d128),
    /// and the outfitter closes once the click is handled (@0x5da33). By
    /// the other reading, the selected cell is kept and the outfitter
    /// stays open. An order that leaves the listing unchanged is the same
    /// by both. The Bible says nothing of it, so the other reading is the
    /// port's earlier behaviour, not anything the Bible says (see
    /// [`Session::outfit_order`](crate::Session::outfit_order)).
    RelistedSelection => "relisted_selection",
    /// What the outfitter's selection is, as its grid's presses and
    /// arrow keys move it. By the engine, `_OutfitFilter` (@0x58039)
    /// keeps the selected outfit (`*0xded58`) apart from the highlighted
    /// cell (`*0xded5c`), and the outfit can be none (-1). A press on an
    /// empty cell position within the grid's 4 × 5 cells
    /// (@0x585ea-0x58607) sets both to -1 (@0x58632-0x58648): Buy and
    /// Sell grey, the picture, the info box and the description blank
    /// (@0x588e4-0x588fe; `_OutfitDialogUpdate` @0x5789b-0x578a5), and
    /// the press reaches no dialog item (@0x58902-0x58920). After a
    /// [`RelistedSelection`](RuleKey::RelistedSelection) reset, that lets
    /// `oütf` 128 go. An arrow key that leaves the selected outfit as it
    /// was skips `_LoadDescription` (@0x58442-0x5844f), while the
    /// picture and the info box follow the highlighted cell
    /// (@0x57949-0x579d6): an arrow onto 128's cell after a reset shows
    /// its picture and info box over the blank description. By the other
    /// reading, the selection is the highlighted cell: a press on an
    /// empty cell is ignored, falling through to the dialog with the
    /// selection or reset kept, and an arrow reads the description of an
    /// outfit not already described. The Bible says nothing of the grid,
    /// so the other reading is the port's earlier behaviour, not anything
    /// the Bible says (see nova-view's `OutfitterScreen`).
    GridSelection => "grid_selection",
    /// How an outfit flagged 0x0200 (price by ship mass) is priced. By
    /// the engine, `_AdjustedItemCost` (@0x5869-0x5895) gives 0 for a
    /// `Cost` of 0 or less (@0x5872-0x5878). Otherwise, with the flag, it
    /// gives max(`Cost`, `Cost` × the flown class's `Mass`)
    /// (@0x5883-0x588f), so `Cost` on a class of `Mass` 0 or less.
    /// `_PlayerShipTradeInPrice` inlines the same (@0xb0e3-0xb106). By the
    /// other reading, the price is `Cost` × `Mass`, never below 0, as the
    /// Bible's "ship class Mass field is multiplied by this item's Cost
    /// field" reads (see [`outfitter`](crate::outfitter)).
    MassPrice => "mass_price",
    /// How much an outfit flagged 0x0400 (mass by ship mass) weighs. By
    /// the engine, `_AdjustedItemMass` (@0x2889-0x28c2) gives the outfit's
    /// `Mass` when that is 0 or less (@0x2893-0x2895) or the flag is
    /// clear. Otherwise it gives max(`Mass`, `Mass` × the class's `Mass` /
    /// 100, truncated) (@0x28a0-0x28bc), so `Mass` on a class of `Mass`
    /// below 100. `_ShipFreeMass` inlines the same (@0xb4c1-0xb4ed). By
    /// the other reading, the mass is the bare product for a positive
    /// `Mass`, as the Bible's "ship class Mass field is multiplied by this
    /// item's Mass field and then divided by 100" reads, so a flagged
    /// outfit can weigh less than its `Mass`, and 0 or less on a class of
    /// `Mass` 0 or less (see [`outfitter`](crate::outfitter)).
    ScaledMass => "scaled_mass",
    /// Which outfits a ship's trade-in counts. By the engine,
    /// `_PlayerShipTradeInPrice` skips only an outfit with a count of 0
    /// or less (@0xb0d5-0xb0d8) and a persistent one (the record byte
    /// `_LoadObjectData` sets from `Flags` 0x0004, @0x78cad-0x78cb3; read
    /// @0xb0da-0xb0e1), so an unsellable (0x0008) outfit counts at half
    /// its line. By the other reading, an unsellable outfit is left out
    /// too. The Bible says only that a persistent outfit stays with the
    /// player, so the other reading is the port's earlier behaviour, not
    /// anything the Bible says (see [`shipyard`](crate::shipyard)).
    TradeInOutfits => "trade_in_outfits",
    /// How many tons a plain Buy or Sell at the exchange moves. By the
    /// engine, a plain buy moves min(trunc(cash / price), 10, free) tons
    /// (`_DoTradeDialog` @0x5e268-0x5e278) and a plain sale min(held, 10)
    /// (@0x5e48d-0x5e4fa); only Option opens the quantity dialog. By the
    /// other reading, a plain trade moves 1 ton. The Bible says nothing of
    /// it, so the other reading is the port's earlier behaviour, not
    /// anything the Bible says (see [`market`](crate::market)).
    TradeLot => "trade_lot",
    /// How the exchange divides the cash by a row's price for the most a
    /// buy can move (the default of the original's quantity dialog, and
    /// Alt's lot here). By the engine, trunc(f32(cash) / f32(price)), in
    /// single floats (`_DoTradeDialog` @0x5e21d-0x5e234), with the most
    /// then min(that, free) and anything from 32001 up made 32000
    /// (@0x5e23a-0x5e24c): above 2^24 cash it can round up to a ton more
    /// than the cash covers, which the buy then leaves below 0. By the
    /// other reading, exact integer division. The Bible says nothing of
    /// it, so the other reading is the port's earlier behaviour, not
    /// anything the Bible says (see [`market`](crate::market)).
    TradeQuotient => "trade_quotient",
    /// What Option (Alt here) does with Buy or Sell at the exchange. By
    /// the engine, it asks for a count in the quantity dialog
    /// (`_DoQuantityDialog` @0x56d5b, called from `_DoTradeDialog`
    /// @0x5e211 and @0x5e45c), which opens at the most the row offers
    /// ([`Market::row_max`](crate::Market::row_max)) and trades the count
    /// confirmed ([`Lot::Count`](crate::Lot::Count)). By the other
    /// reading, Alt trades the most at once ([`Lot::Max`](crate::Lot::Max)).
    /// The Bible says nothing of it, so the other reading is the port's
    /// earlier behaviour, not anything the Bible says (see
    /// [`market`](crate::market)).
    TradeCount => "trade_count",
    /// What cash a buy at the exchange divides by the row's price. By the
    /// engine, the cash as it stands, signed: the quotient
    /// trunc(f32(cash) / f32(price)) (`_DoTradeDialog` @0x5e21d-0x5e234)
    /// has no clamp before it or after it, and Buy is enabled only while
    /// cash >= price (`_CanBuyGoods` @0xccec-0xcd31). So with cash below 0
    /// a `jünk` priced exactly at the cash is bought, 1 ton, and the cash
    /// left at 0. By the other reading, cash below 0 counts as none, and
    /// such a buy is refused. Under both, a buy with cash below the price
    /// is refused, as the engine greys Buy. The Bible says nothing of it,
    /// so the other reading is the port's earlier behaviour, not anything
    /// the Bible says (see [`market`](crate::market)).
    TradeDebt => "trade_debt",
    /// Whether an outfit or a ship class of a negative `TechLevel`, or an
    /// outfit of 0x7fff, is sold by tech. By the engine, never: an outfit
    /// is tested by tech only when its `TechLevel` is 0 or more and not
    /// 0x7fff (`_SetupPortAvailableItems` @0xbf88-0xbf9d), and a ship
    /// class only when 0 or more, with no 0x7fff test
    /// (`_SetupPortAvailableShipTypes` @0xbc4b-0xbc52;
    /// [`wares::outfit_tech_allows`](crate::wares::outfit_tech_allows),
    /// [`wares::ship_tech_allows`](crate::wares::ship_tech_allows)). An
    /// outfit's Buy runs a tech test of its own (`_CanBuyOutfitItem`
    /// @0x4e7ff-0x4e83d; [`wares::outfit_buy_tech_allows`](crate::wares::outfit_buy_tech_allows)):
    /// one of `TechLevel` -1 always passes (`cmpw $-0x1` @0x4e802), and any
    /// other passes at or below the stellar's tech or by a slot, with no
    /// sign or 0x7fff test, so an owned `SELL_ANYWHERE` or `Flags2` 0x0400
    /// row listed off tech can still be bought. By the other reading, any
    /// `TechLevel` at or below the stellar's is sold, and the listing and
    /// Buy test that one rule: the Bible's `spöb` introduces the field with
    /// "The next fields tell Nova what items and ships are available for
    /// purchase at this stellar", and names no exception for -1.
    /// The Bible's `oütf` says "This item will be available at all
    /// spaceports with a tech level of this value or higher", its `shïp`
    /// "This ship will be available at all shipyards with a tech level of
    /// this value or higher", and its `spöb` `TechLevel` "Only items and
    /// ships with `TechLevels` at or below this value will be available"
    /// ([`wares::by_tech_level`](crate::wares::by_tech_level)). Under both
    /// readings a `SpecialTech` slot is compared raw, 0 and below
    /// included: the Bible's "only items and ships with exactly this
    /// `TechLevel` will appear here" gives no exception, and the engine
    /// makes none (@0xbfb2-0xbfbd, @0xbc67-0xbc72). The bar's hire list
    /// follows the engine alone (see [`hire`](crate::hire)).
    TechLevel => "tech_level",
    /// Which persistent outfits a ship purchase carries over. By the
    /// engine, `_DoShipyardDialog` zeroes only the outfits whose record
    /// byte +0x378 is clear (@0x5ee1b-0x5ee4f), which `_LoadObjectData`
    /// sets from `Flags` 0x0004 alone (@0x78cad-0x78cb3), and then adds
    /// the new class's default items on top (@0x5ee51-0x5ee97). It calls
    /// no `_HasMaxOfItem` and no `_ShipFreeMass`, and writes the cash only
    /// for the trade-in and the price, so every persistent outfit is kept
    /// whole, whatever its `Max` and mass, and none is sold back. By the
    /// other reading, each persistent outfit is capped at its `Max`, then
    /// kept one unit at a time by ascending ID while the new ship's free
    /// mass holds it with its default items and stock fitted, and the rest
    /// is sold back at the outfitter's rate on the old ship's mass. The
    /// Bible says only that a persistent outfit "stays with you when you
    /// trade ships" (see [`shipyard`](crate::shipyard)).
    PurchaseCarry => "purchase_carry",
    /// The Shipyard Info panel's Speed and Accel: by the engine, through
    /// the loader's single-precision `Speed` / 100 and `Accel` / 10000, so
    /// Speed can draw one less than recorded and Accel 130 and 250 rate a
    /// step higher; otherwise exact (see [`ShipSpecs`](crate::ShipSpecs)).
    /// The Bible is silent here, so the other reading is the intended
    /// behaviour, not anything the Bible says.
    ShipInfoRounding => "ship_info_rounding",
    /// What a refit does to the secondary selected when the ship no
    /// longer carries it: no outfit the pilot owns, at a count above 0,
    /// has a `ModType` 1 slot naming it. By the engine, nothing: it stays
    /// named and fires nothing, as `_EvalSetExp` and
    /// `_SystemInfoToShipStats` never write it (ship record +0x72) and
    /// `_DrawStatusWeap` still draws it (@0x4c16c-0x4c1b9). Otherwise, it
    /// is let go. The Bible is silent here, so the other reading is the
    /// intended behaviour, not anything the Bible says (see
    /// [`Session`](crate::Session)).
    StaleSecondary => "stale_secondary",
    /// When the outfitter's sale lets go of the secondary selected. By the
    /// engine, when the sold outfit's own count falls to 0 and one of its
    /// `ModType` 1 slots names it (`_DoOutfitDialog` @0x5cf73-0x5cf8c,
    /// @0x5ce82), even while another outfit still mounts the weapon.
    /// Otherwise, when the sale leaves the ship not carrying it. Under
    /// both, the test is made after the sale and before `OnSell`. The
    /// Bible is silent here, so the other reading is the intended
    /// behaviour, not anything the Bible says (see
    /// [`Session`](crate::Session)).
    SecondarySale => "secondary_sale",
    /// What an unabortable mission (`CanAbort` 0) failed by its deadline
    /// does at its return landing. By the engine, it fails there again:
    /// `_QuickMissionFailure` (@0x99d1b-0x99dab) runs its `OnFailure` and
    /// leaves it active and failed, applying no legal record, and the
    /// land check's `_DoMissionFailure` (@0xa1a92, @0xa0285) runs
    /// `OnFailure` a second time and applies the failure records. By the
    /// other reading, it is freed there without running `OnFailure` again,
    /// the failure records still applied once, as the original applies
    /// them only at that landing. A mission failed by `F` runs `OnFailure`
    /// once at its return either way. The Bible says nothing of it, so
    /// the other reading is the intended behaviour, not anything the
    /// Bible says (see the [`mission`](crate::mission) module's "Active
    /// missions").
    RepeatMissionFailure => "repeat_mission_failure",
    /// What `S` does with a mission active already. By the engine, it
    /// accepts the mission into another slot, as `_EvalSetExp`
    /// (@0x153b9-0x15418) checks neither its availability nor whether it
    /// is active. By the other reading, the active mission and the slots
    /// are left as they are. The Bible's "start mission xxx" says nothing
    /// of it, so the other reading is the intended behaviour, not anything
    /// the Bible says (see the [`mission`](crate::mission) module's
    /// "Active missions").
    StartActiveMission => "start_active_mission",
    /// How a mission text's `<PAY>` writes an amount of a million or
    /// more. By the engine, in millions to two places, truncated, as
    /// `_StrcatCommaNumber` (@0x8990-0x8aa3) writes it: "1.23M". By the
    /// other reading, in full with its commas: "1,234,567". The Bible
    /// says nothing of it, so the other reading is the intended
    /// behaviour, not anything the Bible says (see
    /// [`text_tags`](crate::text_tags)).
    PayMillions => "pay_millions",
    /// The order the mission computer and the other venues list their
    /// offers in. By the engine, as `_RandomizeMissionData`
    /// (@0xa1842-0xa197d) sorts them: every offer of `DispWeight` 0 or
    /// more in mission-index order first, then the rest by descending
    /// weight, ties in index order, as its first threshold reads the zero
    /// padding before the weights. By the Bible's "higher values appear
    /// nearer the top of the list", every offer by descending weight, ties
    /// in index order (see [`mission`](crate::mission)).
    MissionOrder => "mission_order",
    /// The stellar a person's mission is offered from, the player ship's
    /// stellar target (+0x6c), which the original reads both as a
    /// person's offer is resolved at its spawn (`_RandomizeOneMission`
    /// @0xa151f-0xa1526, from `_SpawnPerson` @0x4104b-0x41072) and as it
    /// is accepted in flight (`_DoMissionAccept` @0xa1fc5-0xa1ff8). By the
    /// engine it is the player's navigation target: the offer is resolved
    /// from it, or from no stellar with none, and an accept in flight
    /// reaches a destination that is the navigation target. By the other
    /// reading it is the first stellar the player's system lists, as `S`
    /// resolves in flight: the offer is resolved from it, and an accept
    /// in flight reaches no destination. The Bible says nothing of it, so
    /// the other reading is the intended behaviour, not anything the
    /// Bible says (see [`mission`](crate::mission)).
    PersonMissionStellar => "person_mission_stellar",
    /// How a random mission destination -2 is drawn
    /// (`_SelectMissionStellar` @0x9a26a). By the engine, as the original
    /// draws it: its scan (@0x9a2c7-0x9a330) only gates the draw, and the
    /// draw (@0x9a365-0x9a3e2) tests the distance of the scan's first hit
    /// rather than the stellar drawn (@0x9a3ce-0x9a3db), so it takes any
    /// inhabited, normal-landable non-station stellar of a visible system
    /// at any distance; where the original would redraw forever, it gives
    /// the origin. By the other reading, the port's earlier draw: only
    /// candidates distant from the origin. The Bible says nothing of it,
    /// so the other reading is the intended behaviour, not anything the
    /// Bible says (see [`mission`](crate::mission)).
    MissionStellarDraw => "mission_stellar_draw",
    /// Whether a mission's random stellar is distant from its origin
    /// (`_StellarsAreDistant` @0x9cb9-0x9ef6). By the engine, as the
    /// original tells it: with an origin in a visible system, a stellar is
    /// distant when its system is not at the origin's place
    /// (@0x9d35-0x9d9d), neither system's own Con links reach a system at
    /// the other's place (@0x9da3-0x9e1c), and every system at its
    /// system's place lists it (the co-located chain walk,
    /// @0x9e60-0x9ee4); with no origin, as availability asks, only by
    /// that last test (@0x9e20-0x9e5c); with an origin of no visible
    /// system, never (@0x9d1d-0x9d24). By the other reading, the port's
    /// earlier test: in another system that the visible map does not link
    /// with the origin's either way, and no test with no origin or an
    /// origin of no visible system. The Bible says nothing of it, so the
    /// other reading is the intended behaviour, not anything the Bible
    /// says (see [`mission`](crate::mission)).
    MissionStellarDistance => "mission_stellar_distance",
    /// Whether a mission's destination or return is reached at a stellar
    /// of another ID (`_StellarsAreIdentical` @0xab35-0xac90). By the
    /// engine, as the original tells it: the docked stellar is the stop
    /// when their IDs match (@0xab72-0xab75), or when both have the same
    /// `xPos` and `yPos` (@0xab98-0xaba0) and the same name, exactly
    /// (@0xaba6-0xac74), in any system, so a storyline's variant of a
    /// stellar stands in for it. By the Bible, only the stellar of that
    /// ID: its `TravelStel` and `ReturnStel` each name one stellar by ID
    /// (see [`mission`](crate::mission)).
    MissionStellarIdentity => "mission_stellar_identity",
    /// Which legal record the readers read (markup, hail and AI through
    /// the session's system record, landing, `AvailRecord`). By the
    /// engine, as the original keeps it: one record a system
    /// (`_playerRecord`, 2048 `i16` at 0x230760), read in the player's
    /// system, whatever the landed stellar's government
    /// (`_HandlePlayerDockRequest` @0x67903, `_IsMissionAvailable`
    /// @0x9b59d). By the Bible, one record a government, read through the
    /// system's government, and at a landing through the stellar's (see
    /// [`legal`](crate::legal)). Both stores are always written.
    LegalRecords => "legal_records",
    /// Which mission ships that leave the system lower their mission's
    /// ships left, so they are not brought in again. By the engine, as
    /// the original counts them, only a chase-off goal's (`ShipGoal` 6):
    /// a ship of any other goal that jumps out keeps its ships left and
    /// comes back at the next setup (`_LowLevelAIHandler`
    /// @0x85983-0x859aa). By the other reading, every departed mission
    /// ship lowers its ships left and is not brought back. Under both, a
    /// ship of a mission whose ships follow the player (`ShipSyst` -6)
    /// counts nothing (@0x8597c-0x85981), and only a chase-off goal's
    /// counts chased off. The Bible says nothing of it, so the other
    /// reading is the intended behaviour, not anything the Bible says
    /// (see [`mission::goals`](crate::mission::goals)).
    LeftRespawn => "left_respawn",
    /// Whether a mission ship that lands leaves the system, for the
    /// counts of [`RuleKey::LeftRespawn`]. By the engine, no: in the
    /// original an AI ship that lands stays in the system, waiting at the
    /// stellar (`_HighLevelAIHandler` @0x8dadc-0x8db5a), and only its
    /// later jump out counts, so the ship the engine takes out as it
    /// lands counts nothing. By the other reading, it leaves as one that
    /// jumped out does. The Bible says nothing of it, so the other
    /// reading is the intended behaviour, not anything the Bible says
    /// (see [`story`](crate::story)).
    LandedLeaves => "landed_leaves",
    /// What the player's exit from a hypergate or wormhole does to the
    /// special ships of a mission that follow the player (`ShipSyst` -6).
    /// By the engine, as the original's loop A decides
    /// (`_HandlePlayerDockRequest` @0x6716a-0x67294, see
    /// [`mission::ships::through_gate`](crate::mission::ships::through_gate)):
    /// escorts of the player are set up at once; attackers of the player
    /// follow on their `düde` government's `Flags2` or odds and come out
    /// of the exit gate after the player (@0x67346-0x67487), fresh ships;
    /// every other slot loses its ships until the enter-system hook
    /// restarts a `ShipStart` 1 slot's timer, and a `ShipStart` 0 or 2
    /// slot's for good. By the other reading, the exit leaves the slots as
    /// any jump does: the hook restarts a `ShipStart` 1 slot's timer, and
    /// the exit's setup sets the others up again in the new system. The
    /// Bible says nothing of it, so the other reading is the intended
    /// behaviour, not anything the Bible says (see the session's
    /// `mission_ships` module).
    GateFollow => "gate_follow",
    /// What the player's exit from a hypergate or wormhole does to the
    /// auxiliary ships of a mission whose `AuxShipSyst` is -1. By the
    /// engine, as the original's loop A does after the enter-system
    /// hook's restart (`_HandlePlayerDockRequest` @0x6729b-0x672ac): their
    /// countdown is set to 0x7fff, above the 32000 it counts down from,
    /// so they come no more until the next restart (a jump, an `M` or
    /// `N` in flight, or a take-off). By the other reading, the Bible's
    /// "-1: any system": the exit restarts them as a jump does (see
    /// [`mission::aux_ships`](crate::mission::aux_ships)).
    AuxGate => "aux_gate",
    /// What `_EnterMoreShips` does after a mission of `Flags` 0x0001
    /// auto-aborts once its auxiliary ships have all come. By the engine,
    /// as the original: the call to `_AutoAbortMission` (@0x4373a-0x43752)
    /// falls into the special-ship part (@0x43757) with no new test of
    /// the slot's active byte, and `_AbortMission` clears none of the
    /// slot's record, so the special part runs on the freed slot's stale
    /// state. When its appear timer runs out that frame, the special ships
    /// come, marked with the freed slot, and the mission auto-aborts a
    /// second time (@0x43abc-0x43ad4): its abort script runs again, the
    /// days pass again, the fuel is taken again and it pays again. By the
    /// other reading the freed slot is skipped. The Bible says nothing of
    /// it, so the other reading is the intended behaviour, not anything
    /// the Bible says (see the session's `aux_ships` module).
    AuxAbortFallThrough => "aux_abort_fall_through",
    /// Which boards of a board or rescue goal's special ship (`ShipGoal` 2
    /// or 5) count it boarded. By the engine, as the original: only the
    /// assist path (`_HandlePlayerBoardAttempt` @0x65575-0x65762: `Flags`
    /// 0x0001 and one ship left) or a `PickupMode` 2 pickup (@0x653a7)
    /// counts it; a plain board only marks the ship boarded, so a goal with
    /// neither path can never be met. By the other reading, the Bible's
    /// "board them" and "rescue them": any board of such a ship counts it
    /// (see [`mission::goals`](crate::mission::goals)).
    PlainBoardCounts => "plain_board_counts",
    /// Whether a hidden mission (`Flags` 0x0400) fails at once on the
    /// first loss of one of its ships. By the engine, as the original: the
    /// hidden test (`_HandleShipDisplay` @0x2db27, `_DamageShip` @0x3aede)
    /// skips both the message and the failure, so a hidden board or rescue
    /// mission whose ship is destroyed never ends. By the other reading the
    /// first loss fails it as it does any other mission, but quietly: no
    /// message, as a hidden mission shows nothing of itself. The Bible says
    /// nothing of it, so the other reading is the intended behaviour, not
    /// anything the Bible says (see [`mission::goals`](crate::mission::goals)).
    HiddenLossFails => "hidden_loss_fails",
    /// How often a mission's goals-met moment shows and runs. By the
    /// engine, as the original: every time its goals go from unmet to met
    /// (`_MissionObjectivesCheck` @0x9ec70-0x9ed87), its `ShipDoneText` is
    /// shown and its `OnShipDone` runs, so an escort goal's run again each
    /// time its ships come back to the player. By the other reading the
    /// moment's `ShipDoneText`, `OnShipDone` and auto-abort happen only the
    /// first time ([`SpecialShips::ship_done`](crate::mission::SpecialShips::ship_done));
    /// later moments still update the goals met but show and run nothing.
    /// The Bible says nothing of it, so the other reading is the intended
    /// behaviour, not anything the Bible says (see
    /// [`mission::goals`](crate::mission::goals)).
    ShipDoneOnce => "ship_done_once",
    /// What the ship swapped in for a person counts. By the engine, as the
    /// original's swap (`_HandlePlayerCommunication` @0x6244b-0x6252f): no
    /// count of the slot changes and the appear timer runs on, so for a
    /// `ShipStart` of 1 or more the timer still brings another ship in beside
    /// the swapped one (`_EnterMoreShips` @0x437ec-0x43aa5). By the other
    /// reading the swapped ship counts as the timer's arrival: the timer
    /// stops, as after the timer's own arrival, and for `ShipStart` 1 the
    /// arrived count goes up by one. The Bible says nothing of it, so the
    /// other reading is the intended behaviour, not anything the Bible says
    /// (see [`mission::ships`](crate::mission::ships)).
    SwapArrives => "swap_arrives",
    /// Which name the smuggling notice gives the scanner. By the engine, as
    /// the original (`_ScanPlayer` @0x7e7ce reads the found slot's drawn
    /// ship name, never the scanner's mark): the found mission's ship name
    /// whenever it is not empty, whoever the scanner is. By the other
    /// reading, that name only when the scanner is that mission's own
    /// special ship; any other scanner is named by its class. The Bible says
    /// nothing of it, so the other reading is the intended behaviour, not
    /// anything the Bible says (see [`scan`](crate::scan)).
    SmugglingShipName => "smuggling_ship_name",
    /// When the flight HUD's content refreshes. By the engine, on the
    /// original's timing, which `_DrawStatus` (@0x4c85d) keeps in 60 Hz
    /// `TickCount` units: the radar's blips every 15 Mac ticks
    /// (@0x4c911-0x4c94f), which is every 8 engine ticks (≈0.267 s), so
    /// they step a few times a second; the nav area's text and colours
    /// when `TickCount() % 30 == 0` (@0x4c899-0x4c8b6), every 15 engine
    /// ticks (0.5 s), or at once when the selection changes; and the
    /// target panel when `TickCount() % 45 == 0` (@0x4c8b9-0x4c8eb),
    /// every 45 engine ticks (90 Mac ticks, 1.5 s), at the next radar
    /// refresh once the target's whole shield or its ship type has
    /// changed (@0x4c963-0x4c994), or at once on a change of target.
    /// Docked, the status bar holds between the port's redraws: the radar
    /// after an outfitter order (@0x5c5a1, @0x5d328), the radar, nav and
    /// target on a ship purchase (@0x5f0b1; everything under a new
    /// `ïntf`, @0x739bd), the fuel gauge on a recharge (@0x5fe05), and
    /// everything as a shop closes (`_RefreshSAT`). By
    /// the other reading, every frame. The target's blink keeps the engine's timing under both,
    /// as it is a feature, not a redraw artefact. The Bible says nothing
    /// of it, so the other reading is the obvious alternative, not
    /// anything the Bible says (see `nova_view::flight::cadence`).
    HudCadence => "hud_cadence",
    /// When a Red Alert blinks its icon. By the engine, only while the
    /// player's sound is quiet: the original starts the blink only when
    /// its `Volume` preference is 1 or less (`_HandlePlayer`
    /// @0x68d04-0x68d15), which here is sound off or an effects level of
    /// 1 or less; louder, the alert is `snd ` 370 alone. By the other
    /// reading, with every alert. The Bible says nothing of it, so the
    /// other reading is the obvious alternative, not anything the Bible
    /// says (see [`red_alert`](crate::red_alert)).
    AlertBlink => "alert_blink",
    /// A crön's `Random`: by the engine, it starts when `_Rand(101)`
    /// (0-100) is at most `Random`, so 0 still fires 1 day in 101 and
    /// below 0 never; by the Bible, `Random` is a percent chance (see
    /// [`cron`](crate::cron)).
    CronRandomScale => "cron_random_scale",
    /// A crön's date range: by the engine, the year is compared on its
    /// own and the month and day on their own, so a range across a year
    /// end with month/day set never matches; by the other reading the
    /// range is compared as dates and may cross a year end (see
    /// [`cron`](crate::cron)).
    CronDateCompare => "cron_date_compare",
    /// Whose stellars show a cron's government news: by the engine, a
    /// stellar whose government `_GovtAllies` finds allied with the
    /// slot's `NewsGovt` (the same government, or a shared ally class);
    /// by the other reading, only a stellar of that government (see
    /// [`news`](crate::news)).
    NewsGovtMatch => "news_govt_match",
    /// Whether unaffiliated ranks are one government for the `K` and `L`
    /// sweeps: by the engine, every rank of `AffilGovt` -1 (or 127 and
    /// below) sweeps and is swept by the others, as the original compares
    /// the governments raw; by the other reading, an unaffiliated rank
    /// shares no government (see [`rank`](crate::rank)).
    RankUnaffiliatedGroup => "rank_unaffiliated_group",
    /// How a rank's `SalaryCap` limits its salary: by the engine, the cap
    /// is tested against the cash before paying, so one payment can
    /// overshoot it; by the other reading, the cash never goes past it
    /// (see [`rank`](crate::rank)).
    SalaryCap => "salary_cap",
    /// Which governments a rank's `Flags` reach: by the engine, ships of
    /// the rank's government and of every government `_GovtAllies` finds
    /// allied with it; by the other reading, only ships of the rank's
    /// own government, so an unaffiliated rank reaches none (see
    /// [`rank`](crate::rank)).
    RankGovtReach => "rank_govt_reach",
    /// What a rank's `PriceMod` reaches: by the engine, only the
    /// shipyard (a ship's price, the hire fee and the trade-in), as only
    /// the shipyard reads `_priceFluxMult`; by the other reading, the
    /// outfitter's buy price too, the Bible's "items and ships" (see
    /// [`rank`](crate::rank)).
    RankPriceScope => "rank_price_scope",
    /// How the ranks' factor reaches the trade-in: by the engine, through
    /// the flux twice, `_priceFluxMult[0]` then `[1]`, so the factor is
    /// squared and the low-tech discount and rounding apply twice; by the
    /// other reading, once, with `[1]` (see
    /// [`shipyard`](crate::shipyard)).
    RankTradeIn => "rank_trade_in",
    /// When the ranks' price factor is taken: by the engine, as the
    /// spaceport opens (`_DoPortDialog`), so a rank gained or lost while
    /// landed changes no price until the next landing; by the other
    /// reading, live, from the ranks and the stellar's government as they
    /// are when a price is asked (see [`rank`](crate::rank)).
    RankPriceTiming => "rank_price_timing",
    /// Which governments a crime reaches to take a rank of `Flags` 0x0004
    /// or 0x0040: by the engine, both reach every government
    /// `_GovtAllies` finds allied with the rank's, an equal one included,
    /// so a crime against an independent ship takes an unaffiliated rank;
    /// by the other reading, 0x0040 only the rank's own government,
    /// 0x0004 its own or its allies, and an unaffiliated rank none (see
    /// [`rank`](crate::rank)).
    RankCrimeReach => "rank_crime_reach",
    /// Which visible system stands for a place two visible systems share:
    /// by the engine, the lowest-ID one, for a visible system too, so links
    /// reach only it and a visible system it shadows claims no stellar; by
    /// the other reading, each visible system stands for itself, links
    /// reach every visible system at the place, and each stellar goes to
    /// the lowest-ID visible system listing it (see
    /// [`CoLocated`](crate::hyperspace::CoLocated) and the
    /// [`story`](crate::story) module's "Visibility and governments").
    ColocatedResolution => "colocated_resolution",
    /// When each `sÿst` `Visibility` is evaluated: by the engine, at the
    /// original's refresh points (a landing, a jump's arrival before its
    /// days, a new pilot after its `OnStart`, and a load); by the other
    /// reading, after every settled script as well (see the
    /// [`story`](crate::story) module's "Visibility and governments").
    VisibilityRefresh => "visibility_refresh",
    /// What a `Duration` 0 crön does on the day its `PreHoldoff` ends: by
    /// the engine, `OnStart` and `OnEnd` run but nothing more is written
    /// (`_CronEventHandler` @0x387a8-0x387c1), so the next day the
    /// running branch runs `OnEnd` again (@0x387d3-0x387e8) and takes
    /// the end [`CronPostHoldoff`](RuleKey::CronPostHoldoff) gives; by the
    /// other reading, the intended end: `OnEnd` once, then the
    /// `PostHoldoff`. The other reading is the default (see
    /// [`cron`](crate::cron)).
    CronZeroDurationHoldoff => "cron_zero_duration_holdoff",
    /// What a running crön's end loads: by the engine, its `PreHoldoff`
    /// into the holdoff when `PostHoldoff` is above 0, with its days left
    /// as they are (@0x387f1-0x387f6 → 0x388c5), so it re-runs, or with
    /// `PreHoldoff` 0 never ends; by the other reading, the intended
    /// end: `PostHoldoff` days with the days at -1. The other reading is
    /// the default (see [`cron`](crate::cron)).
    CronPostHoldoff => "cron_post_holdoff",
    /// When the ranks' blessing table is taken. By the engine, at each of
    /// the original's resets (`_ResetPlayerPrecalcedValues`
    /// @0xc8be-0xc981; the session's `precalced` module lists them), the
    /// end of every set expression that is not empty among them
    /// (@0x99a85, @0x99dea). So the two readings differ in two places: a
    /// `K` or `L` blesses, or stops blessing, as its expression ends, not
    /// between its operators; and a crime's rank loss
    /// (`_SlapWithPenalty`'s pass, which reaches no reset) stops blessing
    /// only at the next reset. By the other reading, live, from the active
    /// ranks (see [`rank`](crate::rank) and the session's `ranks`
    /// module).
    BlessingRefresh => "blessing_refresh",
    /// What a rank's `Flags` 0x0200 clears a governed stellar past: by the
    /// engine, both its `MinStatus` and its government's `Require`
    /// (`_HandlePlayerDockRequest` @0x67940-0x67a05); by the other
    /// reading, the Bible's, its `MinStatus` only, so an unmet `Require`
    /// still refuses (see [`landing`](crate::landing)).
    BlessingLanding => "blessing_landing",
    /// Whether a government's word to attack the player (`gövt` `Flags`
    /// 0x0004) reaches a player its ships are blessed to spare (rank
    /// `Flags` 0x0100): by the engine, it does, as `_WarshipAI` @0x8b7b0
    /// and `_InterceptorAI` @0x8cb5c read no blessing; by the other
    /// reading, the Bible's ("will not automatically attack"), the
    /// blessing holds the word back (see [`ai::warship`](crate::ai::warship)).
    BlessingAlwaysAttacks => "blessing_always_attacks",
    /// Whether exploration is shared across a place: by the engine,
    /// exploring a system explores its place's head too
    /// (`_RecursiveAutoExplore` @0xd9b6-0xda10), and each rebuild point
    /// explores every system at an explored head's place, hidden or not
    /// (`_HandleStellarSystemVisibility` @0x32b13-0x32b8f); by the other
    /// reading, each system is explored on its own (see
    /// [`exploration`](crate::exploration)).
    ExploredByPlace => "explored_by_place",
    /// Whether the player's course is rewritten at the rebuild points: by
    /// the engine, each rebuild point rewrites every hidden hop to the
    /// system at its place and cuts the course at the first hop with
    /// none, for good (`_HandleStellarSystemVisibility` @0x32ab3-0x32b0d),
    /// and an arrival that made no hop clears the course when its next
    /// hop stands elsewhere, or drops that hop when it stands at the
    /// arrival (@0x6c472-0x6c4c6); by the other reading, hops are resolved
    /// only when used, and an arrival drops only the hops made (see
    /// [`course`](crate::course)).
    RouteRewrite => "route_rewrite",
    /// How often a nebula's `OnExplore` runs: by the engine, whether a
    /// nebula is explored is never saved, so after every load the sweep
    /// over the visible explored systems explores it, and runs its
    /// `OnExplore`, again (`_ResetPlayer` @0x1da86, `_OpenLastPilotFile`
    /// @0x765a2-0x765ce); by the other reading, once a pilot, kept in the
    /// once-fired record. The Bible agrees with the engine ("this
    /// expression may get evaluated multiple times"), so the other reading
    /// is the port's choice (see [`nebula`](crate::nebula)).
    NebulaExploreOnce => "nebula_explore_once",
    /// Whether a new pilot's start system explores its nebulae: by the
    /// engine, not until the pilot's first exploration (a take-off, a jump
    /// or a map), as `_DoNewPilot` explores its start system while every
    /// nebula is inactive (@0x18de5, @0x1df4b, @0x19118, before its first
    /// refresh @0x1925e); by the other reading, as the game begins (see
    /// [`nebula`](crate::nebula)).
    NebulaNewPilot => "nebula_new_pilot",
    /// Whether a scan's set `jünk` latch shadows the outfits: by the
    /// engine, a scan that finds no illegal `jünk` while the player
    /// carries any `jünk` with a `ScanMask` ends there, outfits unscanned
    /// (`_ScanPlayer` @0x7eefa); by the other reading, the outfits are
    /// scanned then (see [`scan`](crate::scan)).
    ScanOutfitsPastJunk => "scan_outfits_past_junk",
    /// What a negative `ScanFine` fines an illegal outfit: by the engine,
    /// nothing, as only a `ScanFine` above 0 fines an outfit (`jle`
    /// @0x7f222); by the other reading, the Bible's, the percent of the
    /// cash in hundreds, as for `jünk` and mission cargo (see
    /// [`scan`](crate::scan)).
    OutfitScanFine => "outfit_scan_fine",
    /// What the ships the player leads do at a hypergate or wormhole
    /// exit. By the engine, as the original's loop C
    /// (`_HandlePlayerDockRequest` @0x674ef-0x675c8) and
    /// `_AIMakeShipEmergeFromHyperGate` (@0x895aa-0x895bd) do: every one
    /// comes out of the exit gate after the player, with its heading,
    /// waiting `Rand(20) + 15` ticks and bursting out at 15. By the other
    /// reading, each is snapped onto its slot beside the player.
    GateEscorts => "gate_escorts",
    /// Whether a lost trader escort's own `Holds` count in the fleet's
    /// cargo space it takes its share of (`_DestroyPartialFleetCargo`
    /// @0xcd32-0xcff6; see [`Session`](crate::Session)). By the engine,
    /// never: `_DamageShip` clears the leader of an escort a hit disables
    /// (@0x3afdd) before its call (@0x3b013), and a hit that leaves armour
    /// at 0 or less always disables, so the lost ship is out of its own
    /// sum (@0xce98-0xcf0d) and takes min(1, its `Holds` over the rest):
    /// a lone trader escort with more `Holds` than the player's ship takes
    /// all the cargo. A mission escort is never in the sum, by its mission
    /// mark (@0xced9). By the other reading, its `Holds` are in the sum,
    /// as `_HandleShip`'s branches count the ship (@0x339eb then @0x33a00;
    /// @0x350e6 then @0x35118), so it takes the share of the space it
    /// added; this applies to mission escorts too.
    LostCargoShare => "lost_cargo_share",
    /// Whether the player and every ship it leads fly half as fast again
    /// when the pilot does not play Strict Play (see
    /// [`top_speed`](crate::top_speed)). By the engine, as
    /// `_ShipMaxSpeed` does (@0x382e-0x384f): the top speed of the player
    /// and of every ship whose leader is the player is × 1.5 (@0xdd078)
    /// while the pilot's Strict Play flag is clear. By the other reading,
    /// no factor: every ship flies at its own top speed.
    NonStrictSpeed => "non_strict_speed",
    /// Whether a `ShipBehav` 0 mission ship the player has paid off
    /// attacks the player again (see
    /// [`mission_hunt`](crate::ai::target::mission_hunt)). By the engine,
    /// yes: neither `_SelectWarshipTarget` @0x89f25-0x89f84 nor
    /// `_HighLevelAIHandler` @0x8d628-0x8d6e9 reads the bribe, whose leave
    /// (`_AIMakeShipLeave` @0x7e2b0, called @0x961eb-0x961f8) sets state
    /// 2, which the handler does not skip, so the hunter takes the player
    /// again on its next think. By the other reading, the bribe holds: the
    /// paid-off ship decides as its type, a wimpy trader, does. The Bible
    /// says nothing of it, so the other reading is the intended behaviour,
    /// not anything the Bible says.
    HunterBribe => "hunter_bribe",
    /// The order ships overlap in within each of their three draw layers
    /// (see [`ship_layer`](crate::ship_layer)). By the engine, the order
    /// they last entered the layer, as `_PutSpriteInLayer` (@0x766c)
    /// leaves `SpriteWorld`'s layers from `_InitSprites`' start
    /// (@0x1f0eb-0x1f88b) and each tick's laying out of the player, then
    /// each slot in turn (`_DoPlayGameWork` @0x43e29, @0x444ad): a ship
    /// that enters a layer goes on top of it, so an escort released or a
    /// ship repaired is drawn over the player until history moves it. By
    /// the other reading, each layer in the session's NPC order with the
    /// player on top of its own. The engine's history outlives a pilot,
    /// as the original's sprites do, so a pilot loaded later in the same
    /// run starts from the order the last one left
    /// ([`Session::with_slot_sprites`](crate::Session::with_slot_sprites));
    /// the other reading has no history. The Bible says nothing of it, so
    /// the other reading is the intended behaviour, not anything the Bible
    /// says (see [`Session::draw_order`](crate::Session::draw_order)).
    ShipOverlap => "ship_overlap",
    /// How an NPC's quadrant turret (guidance 7 and 8) measures its arc
    /// (see [`fire`](crate::ai::fire)). By the engine, as `_AIFireTurret`
    /// does (@0x80c3d-0x80cf3): the angle to the target less the
    /// reference, the heading for a front turret and `(heading + 180) mod
    /// 360` for a rear one, made absolute and taken mod 360 with no fold
    /// past 180, must be at most 45, so a turret whose reference sits near
    /// 0 holds fire at a target across the seam. By the other reading, the
    /// angle folds and the arc wraps. The Bible says nothing of it, so the
    /// other reading is the intended behaviour, not anything the Bible
    /// says.
    QuadrantTurretArc => "quadrant_turret_arc",
    /// Whether a ship whose type's `Cost` is -1 is shown (see
    /// [`ship_layer`](crate::ship_layer)). By the engine, as
    /// `_HandleShipDisplay` does for a ship in the player's system
    /// (@0x2b7f3 → @0x2cc6b), the player's too: it is hidden, and its
    /// slot keeps its place in whatever layer it last stood in, as the
    /// branch skips the layer block. By the other reading, it is drawn and
    /// laid out as any other ship. The Bible gives `Cost` only as the
    /// price, so the other reading is the intended behaviour, not anything
    /// the Bible says.
    HiddenShips => "hidden_ships",
    /// The box the pointer hovers a hidden ship by (see
    /// [`Session::framed_by`](crate::Session::framed_by)); it matters
    /// only while [`RuleKey::HiddenShips`] reads the engine. By the
    /// engine, the box of the frames its slot's sprite last cloned, as
    /// `_HandleShipDisplay` skips `_SWCloneFramesFromMaster` for a hidden
    /// ship (@0x2b824) and `_HandleMouse` still tests the pointer against
    /// the sprite (@0x38171): the last shown type at that slot, or
    /// `shïp` 128 in a slot no shown ship has used this run, as
    /// `_InitSprites` clones it into every slot (@0x1f193). By the other
    /// reading, the box of its own type's sheet. The Bible says nothing
    /// of it, so the other reading is the intended behaviour, not
    /// anything the Bible says.
    HiddenShipHoverBox => "hidden_ship_hover_box",
    /// What becomes of a breakup whose death timer never rises above 0 by
    /// the time `_HandleShipDisplay` checks it: a `DeathDelay` of 0 or
    /// less for any ship, or of 1 for an NPC (see
    /// [`Breakup`](crate::combat::hull::Breakup)). By the engine, the
    /// timer is set again every frame (`_HandleShipDisplay`
    /// @0x2d80d-0x2d83d) and the decrements skip it (@0x6d7e6, @0x35083),
    /// so the ship breaks up for good and never explodes: for the player,
    /// a soft-lock. By the other reading, it is destroyed on its breakup
    /// tick (d <= 0) or the tick after (an NPC's d = 1). No stock ship has
    /// such a `DeathDelay`: the lowest is 3. The other reading is the
    /// default (see the module docs).
    BreakupStall => "breakup_stall",
    /// What becomes of the player's velocity as it ejects (see
    /// [`escape`](crate::escape)). By the engine, as `_HandlePlayer` does
    /// (@0x6f782-0x6f788), the new ship's x velocity takes the y
    /// velocity's value, so the hulk keeps the true velocity and the new
    /// ship's x is lost, before the push of its top speed along its
    /// heading. By the other reading, the new ship keeps the velocity it
    /// had. The Bible says nothing of it, so the other reading is the
    /// intended behaviour, not anything the Bible says.
    EjectVelocity => "eject_velocity",
    /// What the escape pod's rescue sets the armour to (see the session's
    /// `rescue` module). By the engine, as `_HandlePlayer` does
    /// (@0x690db-0x69100), the shield capacity, `_ShipShieldCapacity`
    /// read a second time where the armour's is meant, so the armour may
    /// stand above its most, which nothing takes away. By the other
    /// reading, the armour's own capacity. The Bible says nothing of it,
    /// so the other reading is the intended behaviour, not anything the
    /// Bible says.
    RescueArmor => "rescue_armor",
    /// Where a loaded pilot resumes when its saved stellar is one no
    /// system lists, so the loaded data doesn't place it (see
    /// [`Session::load`](crate::Session::load)). By the engine, as
    /// `_LoadPilotData` falls back when `_FindSystemFromStellar` finds no
    /// system (@0x752d6-0x75318), in the first system with +0x1ed set:
    /// the lowest that exists, which needn't hold the stellar. By the
    /// other reading, the default and a recorded departure, it keeps its
    /// saved system. Under both, the stellar is forgotten, the pilot
    /// starts undocked at the centre, and the load reports the repair
    /// ([`Session::load_repaired`](crate::Session::load_repaired)).
    UnlistedStellar => "unlisted_stellar",
    /// Whether a newbie (see [`tutorial`](crate::tutorial)) whose J has a
    /// hyperspace selection but too little fuel is told so. By the engine,
    /// no: the jump key's newbie branch, which says "No destination" with
    /// nothing selected, swallows the case with a selection
    /// (`_HandlePlayer` @0x6b995-0x6b9a3), so the newbie gets no message
    /// and no beep. By the other reading, "Insufficient energy for
    /// hyperspace jump." and `snd ` 153, as for any pilot (@0x6ba72-
    /// 0x6bab7). The Bible says nothing of it, so the other reading is the
    /// intended behaviour, not anything the Bible says (see
    /// [`Session::jump_refusal_told`](crate::Session::jump_refusal_told)).
    NewbieFuelRefusal => "newbie_fuel_refusal",
    /// What a hyperspace arrival in a system whose message buoy names an
    /// index `STR#` 1000 does not hold (0, below 0 but −1, or past its
    /// count), with no `STR ` of that index + 999, shows (see the
    /// [`buoy`](crate::buoy) module). By the engine, the original's empty
    /// line for 480 ticks in place of the arrival line, as
    /// `_DisplayMessage`'s `GetIndString` gives an empty string
    /// (@0x52b3-0x52c7): the two stock Nil'kol systems' 20003 shows it. By
    /// the other reading, the arrival line and its fighters suffix, as for
    /// a system with no buoy. The Bible says nothing of it, so the other
    /// reading is the intended behaviour, not anything the Bible says.
    MissingBuoyString => "missing_buoy_string",
    /// Whether an interceptor's patrol can pick an uninhabited stellar
    /// (see [`destination`](crate::traffic::destination)). `_InterceptorAI`
    /// asks `_SelectRandomStellarDest` for plain stellars only
    /// (@0x8d027-0x8d03a), which picks none unless one is inhabited
    /// (@0x808b6). By the engine, the pick among them does not test for
    /// an uninhabited one (@0x80911-0x80940), so it can be one. By the
    /// other reading, uninhabited stellars are left out of the pick too.
    /// The Bible says nothing of it, so the other reading is the intended
    /// behaviour, not anything the Bible says.
    PatrolUninhabited => "patrol_uninhabited",
    /// Whether a mission ship that leaves the system through a hypergate
    /// or wormhole counts, for the counts of [`RuleKey::LeftRespawn`]
    /// (see [`gate_entry`](crate::traffic::gate_entry)). By the engine,
    /// no: `_HandleShipDisplay` frees its slot as its fade ends (@0x2c078)
    /// with no call, and only the end of a warp out counts
    /// (`_LowLevelAIHandler` @0x85960-0x859aa), so it counts nothing and
    /// comes back at the next setup. By the other reading, it counts as
    /// one that jumped out. The Bible says nothing of it, so the other
    /// reading is the intended behaviour, not anything the Bible says.
    GateExitCounts => "gate_exit_counts",
    /// What becomes of the ships an in-flight `M` or `N` leaves behind in
    /// the system left: every ship the player does not lead (see
    /// [`Traffic::retarget`](crate::Traffic::retarget)). By the engine,
    /// they stay in the ship table until that frame's display pass frees
    /// them: `_EvalSetExp` moves only the ships the player leads
    /// (@0x15807-0x1583e, @0x1590b-0x1593d), and `_HandleShipDisplay`
    /// frees a ship outside the player's system (@0x2ded6-0x2dee7),
    /// counting nothing and giving nothing back. So through the next
    /// traffic tick they hold their berths and their persons, a move back
    /// before then brings them back as they were, and none of their aux
    /// ships is given back to its mission. By the other reading they are
    /// dropped at the move, each aux ship given back as a setup gives it
    /// back (`_DeleteAllComputerShips` @0x3bfce-0x3c028). Only a move
    /// that goes on in the new system reads it: by the other reading of
    /// [`RuleKey::MoveArrival`] an in-flight move populates the system,
    /// which drops every ship there. The Bible says nothing of it, so the
    /// other reading is the intended behaviour, not anything the Bible
    /// says.
    MoveLeftBehind => "move_left_behind",
    /// How many of the ship table's slots a spawn leaves free (see
    /// [`Traffic::free_berth`](crate::Traffic::free_berth)). By the engine, as
    /// `_GenericRandomShipSpawn(arg1, reserve)` searches only slots
    /// 1..(63 - reserve) of its 64 (@0x3c8b1-0x3c8f0): the system's own
    /// traffic (`düde` ships, persons, fleets and auxiliary ships), the
    /// fighters launched, the mission ships (`_MissionDudeSpawn`) and the
    /// hired escorts (`_SpawnEscort`) leave 8 free, so they stop at 55
    /// NPCs (@0x41ce9, @0x421a5, @0x43297, @0x42990, @0x435a8, @0x40cba,
    /// @0x3d648, @0x3cd50, @0x3fe73), and the old ship of a "Use As My Ship" capture leaves 1
    /// (@0x41136). By the Bible, which gives "Max Ships In System" as 64
    /// and holds none back, each may take any of slots 1-63.
    SpawnReserve => "spawn_reserve",
    /// Whether a ship still coming into the system answers a hit (see
    /// [`Traffic::tick_in`](crate::Traffic::tick_in)). By the engine, as
    /// `_DamageShip` drops a reaction only while the victim's hyperspace
    /// state +0x50 is above 0 (@0x3b4b0-0x3b4c2), a ship jumping out, and
    /// an arrival's is -1.0 or -999.0 (@0x89555, @0x82ece, @0x8f3c8): a
    /// ship gliding in, waiting in a gate or bursting out takes the
    /// attacker as its target and the damage as provocation, its wait in
    /// a gate is cut to 20 (@0x3be6e-0x3be7c), and the player's hit on a
    /// ship waiting in a gate holds it there attacking the player, seen
    /// and targetable (`_AIMakeShipAttackPlayer` @0x89d3e-0x89d4d), as a
    /// Good Samaritan's answer does. By the other reading it ignores every
    /// hit until it has arrived. The Bible says nothing of it, so the
    /// other reading is the intended behaviour, not anything the Bible
    /// says.
    ArrivalHits => "arrival_hits",
    /// What a hire does when it finds no ship-table slot below the spawn
    /// reserve (see [`Session::hire`](crate::Session::hire)). By the
    /// engine, as `_DoShipyardDialog` takes the fee and marks the pilot
    /// changed before `_SpawnEscort` (@0x5f168-0x5f1df), ignores its -1
    /// (@0x5f1f1) and draws the class's roll again (@0x5f20d-0x5f21a):
    /// the fee is gone, the roll is drawn again and no escort joins, as
    /// the original's escort is its slot. By the other reading the hire
    /// is refused and nothing changes. The Bible says nothing of it, so
    /// the other reading is the intended behaviour, not anything the Bible
    /// says.
    HireNoSlot => "hire_no_slot",
    /// What a hyperspace arrival does with the asteroid field `_CreateAsteroids`
    /// places (see the [`asteroid`](crate::asteroid) module). By the
    /// engine, every entry kills the field: each caller of
    /// `_CreateAsteroids` sets `_killAsteroids` first (@0x6c4fc), so
    /// `_HandleAsteroids` kills the placed field that same tick, and the
    /// player sees an empty field fill one asteroid a tick from the edges.
    /// By the other reading, `placed` in the settings, a jump's arrival
    /// keeps the placed field, as does a session starting or loading in
    /// flight; a gate, a move or a landing still kills it, as no caller of
    /// theirs places one. The Bible says nothing of it, so the other
    /// reading is the intended behaviour, not anything the Bible says.
    AsteroidEntryField => "asteroid_entry_field",
    /// Where an asteroid spawned at the field's edge starts (see
    /// [`Ring`](crate::asteroid::Ring)). By the engine, as
    /// `_SpawnAsteroid(true)` places it (@0x3a351-0x3a538): at least
    /// 0.7 × (half the play area + 128) out on both axes, which on a
    /// play area wider than the original's smallest can be on screen, so
    /// an asteroid pops in. By the other reading, `outside` in the
    /// settings, just off the screen and inside the cull, so it always
    /// drifts on. The Bible says nothing of it, so the other reading is
    /// the intended behaviour, not anything the Bible says.
    AsteroidRespawnRing => "asteroid_respawn_ring",
    /// Whether a weapon's `Flags2` 0x0004 keeps asteroids from setting off
    /// its proximity fuse (see
    /// [`asteroid_hit`](crate::combat::asteroid_hit)). By the engine, no
    /// code reads the bit (`_CheckShotProximities` @0x376ab-0x37758 tests
    /// only `Seeker` 0x0001, @0x37566-0x37575), so every armed fuse is
    /// set off by the first asteroid in reach. By the Bible, "proximity
    /// detonator ignores asteroids": the fuse of a weapon with the bit
    /// passes asteroids by, though its shots still hit one they touch.
    FuseIgnoresAsteroids => "fuse_ignores_asteroids",
}

impl RuleKey {
    /// The rule whose settings key is `key` (see [`RuleKey::key`]), if
    /// any.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rule| rule.key() == key)
    }

    /// The rule's place in [`RuleKey::ALL`], which lists the variants in
    /// the order declared.
    const fn index(self) -> usize {
        self as usize
    }

    /// The source this rule follows when the settings choose none: the
    /// engine's, but for the two cron holdoff keys, the breakup stall and
    /// the unlisted stellar, which follow the other reading as recorded departures from the
    /// original (see the module docs).
    #[must_use]
    pub const fn default_source(self) -> RuleSource {
        match self {
            Self::CronZeroDurationHoldoff
            | Self::CronPostHoldoff
            | Self::BreakupStall
            | Self::UnlistedStellar => RuleSource::Bible,
            _ => RuleSource::Engine,
        }
    }
}

/// Which source each disputed rule follows: a global source for all, or
/// each rule's own default ([`RuleKey::default_source`]) when none is
/// chosen, and an override for any one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rulebook {
    global: Option<RuleSource>,
    overrides: [Option<RuleSource>; RuleKey::ALL.len()],
}

/// Every rule following its own default, overriding none: the engine's,
/// but for the two cron holdoff keys, the breakup stall and the unlisted
/// stellar. (Written out, as `Default` derives
/// only for arrays of up to 32 rules.)
impl Default for Rulebook {
    fn default() -> Self {
        Self {
            global: None,
            overrides: [None; RuleKey::ALL.len()],
        }
    }
}

impl Rulebook {
    /// Every rule following `global`, overriding none.
    #[must_use]
    pub const fn new(global: RuleSource) -> Self {
        Self {
            global: Some(global),
            overrides: [None; RuleKey::ALL.len()],
        }
    }

    /// The rulebook with `rule` following `source`, whatever the global
    /// source.
    #[must_use]
    pub fn with_override(mut self, rule: RuleKey, source: RuleSource) -> Self {
        self.overrides[rule.index()] = Some(source);
        self
    }

    /// The source chosen for every rule, if one was; `None` when each
    /// rule follows its own default.
    #[must_use]
    pub fn global_source(&self) -> Option<RuleSource> {
        self.global
    }

    /// The source `rule` follows: its override, else the global source,
    /// else its own default.
    #[must_use]
    pub fn source_for(&self, rule: RuleKey) -> RuleSource {
        self.overrides[rule.index()]
            .or(self.global)
            .unwrap_or(rule.default_source())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_follows_its_default_the_engines_but_the_cron_holdoffs_the_breakup_stall_and_the_unlisted_stellar()
     {
        assert_eq!(RuleSource::default(), RuleSource::Engine);
        let rulebook = Rulebook::default();
        assert_eq!(rulebook.global_source(), None);
        assert_ne!(rulebook, Rulebook::new(RuleSource::Engine));
        for rule in RuleKey::ALL {
            assert_eq!(rulebook.source_for(rule), rule.default_source(), "{rule:?}");
        }
        let bible: Vec<RuleKey> = RuleKey::ALL
            .into_iter()
            .filter(|rule| rule.default_source() == RuleSource::Bible)
            .collect();
        assert_eq!(
            bible,
            [
                RuleKey::CronZeroDurationHoldoff,
                RuleKey::CronPostHoldoff,
                RuleKey::BreakupStall,
                RuleKey::UnlistedStellar
            ]
        );
    }

    #[test]
    fn the_default_source_sets_every_rule() {
        for source in RuleSource::ALL {
            let rulebook = Rulebook::new(source);
            assert_eq!(rulebook.global_source(), Some(source));
            for rule in RuleKey::ALL {
                assert_eq!(rulebook.source_for(rule), source, "{source:?}: {rule:?}");
            }
        }
    }

    #[test]
    fn an_override_beats_the_default_both_ways() {
        for (default, other) in [
            (RuleSource::Engine, RuleSource::Bible),
            (RuleSource::Bible, RuleSource::Engine),
        ] {
            let rulebook = Rulebook::new(default).with_override(RuleKey::CrimeGains, other);
            assert_eq!(rulebook.source_for(RuleKey::CrimeGains), other);
            assert_eq!(rulebook.global_source(), Some(default), "unchanged");
            let again = rulebook.with_override(RuleKey::CrimeGains, default);
            assert_eq!(
                again.source_for(RuleKey::CrimeGains),
                default,
                "the last wins"
            );
        }
        let rulebook =
            Rulebook::default().with_override(RuleKey::CronPostHoldoff, RuleSource::Engine);
        assert_eq!(
            rulebook.source_for(RuleKey::CronPostHoldoff),
            RuleSource::Engine
        );
        assert_eq!(
            rulebook.source_for(RuleKey::CronZeroDurationHoldoff),
            RuleSource::Bible
        );
        assert_eq!(rulebook.source_for(RuleKey::CrimeGains), RuleSource::Engine);
        assert_eq!(rulebook.global_source(), None);
    }

    #[test]
    fn an_override_on_one_rule_leaves_the_others_on_the_default() {
        let opposite = |source| match source {
            RuleSource::Engine => RuleSource::Bible,
            RuleSource::Bible => RuleSource::Engine,
        };
        for rule in RuleKey::ALL {
            let overridden = opposite(rule.default_source());
            let rulebook = Rulebook::default().with_override(rule, overridden);
            for other in RuleKey::ALL {
                let expected = if other == rule {
                    overridden
                } else {
                    other.default_source()
                };
                assert_eq!(rulebook.source_for(other), expected, "{rule:?}: {other:?}");
            }
        }
    }

    #[test]
    fn the_trade_quotient_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::TradeQuotient.key(), "trade_quotient");
        assert_eq!(
            RuleKey::from_key("trade_quotient"),
            Some(RuleKey::TradeQuotient)
        );
    }

    #[test]
    fn the_trade_count_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::TradeCount.key(), "trade_count");
        assert_eq!(RuleKey::from_key("trade_count"), Some(RuleKey::TradeCount));
    }

    #[test]
    fn the_outfit_count_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::OutfitCount.key(), "outfit_count");
        assert_eq!(
            RuleKey::from_key("outfit_count"),
            Some(RuleKey::OutfitCount)
        );
    }

    #[test]
    fn the_trade_debt_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::TradeDebt.key(), "trade_debt");
        assert_eq!(RuleKey::from_key("trade_debt"), Some(RuleKey::TradeDebt));
    }

    #[test]
    fn the_mission_rules_are_found_by_their_keys() {
        assert_eq!(
            RuleKey::RepeatMissionFailure.key(),
            "repeat_mission_failure"
        );
        assert_eq!(
            RuleKey::from_key("repeat_mission_failure"),
            Some(RuleKey::RepeatMissionFailure)
        );
        assert_eq!(RuleKey::StartActiveMission.key(), "start_active_mission");
        assert_eq!(
            RuleKey::from_key("start_active_mission"),
            Some(RuleKey::StartActiveMission)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(65),
            Some(&RuleKey::StartActiveMission)
        );
    }

    #[test]
    fn the_tech_level_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::TechLevel.key(), "tech_level");
        assert_eq!(RuleKey::from_key("tech_level"), Some(RuleKey::TechLevel));
    }

    #[test]
    fn the_mass_price_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::MassPrice.key(), "mass_price");
        assert_eq!(RuleKey::from_key("mass_price"), Some(RuleKey::MassPrice));
    }

    #[test]
    fn the_scaled_mass_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::ScaledMass.key(), "scaled_mass");
        assert_eq!(RuleKey::from_key("scaled_mass"), Some(RuleKey::ScaledMass));
    }

    #[test]
    fn the_sale_refusal_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::SaleRefusal.key(), "sale_refusal");
        assert_eq!(
            RuleKey::from_key("sale_refusal"),
            Some(RuleKey::SaleRefusal)
        );
    }

    #[test]
    fn the_pay_millions_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::PayMillions.key(), "pay_millions");
        assert_eq!(
            RuleKey::from_key("pay_millions"),
            Some(RuleKey::PayMillions)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(64),
            Some(&RuleKey::PayMillions)
        );
    }

    #[test]
    fn the_counted_sale_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::CountedSale.key(), "counted_sale");
        assert_eq!(
            RuleKey::from_key("counted_sale"),
            Some(RuleKey::CountedSale)
        );
    }

    #[test]
    fn the_mission_order_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::MissionOrder.key(), "mission_order");
        assert_eq!(
            RuleKey::from_key("mission_order"),
            Some(RuleKey::MissionOrder)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(63),
            Some(&RuleKey::MissionOrder)
        );
    }

    #[test]
    fn the_relisted_sale_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::RelistedSale.key(), "relisted_sale");
        assert_eq!(
            RuleKey::from_key("relisted_sale"),
            Some(RuleKey::RelistedSale)
        );
    }

    #[test]
    fn the_person_mission_stellar_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(
            RuleKey::PersonMissionStellar.key(),
            "person_mission_stellar"
        );
        assert_eq!(
            RuleKey::from_key("person_mission_stellar"),
            Some(RuleKey::PersonMissionStellar)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(62),
            Some(&RuleKey::PersonMissionStellar)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::PersonMissionStellar),
            RuleSource::Engine
        );
    }

    #[test]
    fn the_relisted_selection_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::RelistedSelection.key(), "relisted_selection");
        assert_eq!(
            RuleKey::from_key("relisted_selection"),
            Some(RuleKey::RelistedSelection)
        );
    }

    #[test]
    fn the_mission_stellar_draw_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::MissionStellarDraw.key(), "mission_stellar_draw");
        assert_eq!(
            RuleKey::from_key("mission_stellar_draw"),
            Some(RuleKey::MissionStellarDraw)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(61),
            Some(&RuleKey::MissionStellarDraw)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::MissionStellarDraw),
            RuleSource::Engine
        );
    }

    #[test]
    fn the_mission_stellar_distance_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(
            RuleKey::MissionStellarDistance.key(),
            "mission_stellar_distance"
        );
        assert_eq!(
            RuleKey::from_key("mission_stellar_distance"),
            Some(RuleKey::MissionStellarDistance)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(60),
            Some(&RuleKey::MissionStellarDistance)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::MissionStellarDistance),
            RuleSource::Engine
        );
    }

    #[test]
    fn the_mission_stellar_identity_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(
            RuleKey::MissionStellarIdentity.key(),
            "mission_stellar_identity"
        );
        assert_eq!(
            RuleKey::from_key("mission_stellar_identity"),
            Some(RuleKey::MissionStellarIdentity)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(59),
            Some(&RuleKey::MissionStellarIdentity)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::MissionStellarIdentity),
            RuleSource::Engine
        );
    }

    #[test]
    fn the_mission_ship_departure_rules_are_found_by_their_keys_and_read_the_engine() {
        for (rule, key) in [
            (RuleKey::LeftRespawn, "left_respawn"),
            (RuleKey::LandedLeaves, "landed_leaves"),
        ] {
            assert_eq!(rule.key(), key);
            assert_eq!(RuleKey::from_key(key), Some(rule));
            assert_eq!(Rulebook::default().source_for(rule), RuleSource::Engine);
        }
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(56),
            Some(&RuleKey::LandedLeaves)
        );
    }

    #[test]
    fn the_leaderless_swap_hold_rule_reads_the_engine() {
        assert_eq!(
            Rulebook::default().source_for(RuleKey::LeaderlessSwapHold),
            RuleSource::Engine
        );
    }

    #[test]
    fn the_gate_follow_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::GateFollow.key(), "gate_follow");
        assert_eq!(RuleKey::from_key("gate_follow"), Some(RuleKey::GateFollow));
        assert_eq!(
            Rulebook::default().source_for(RuleKey::GateFollow),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(55),
            Some(&RuleKey::GateFollow)
        );
    }

    #[test]
    fn the_gate_escorts_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::GateEscorts.key(), "gate_escorts");
        assert_eq!(
            RuleKey::from_key("gate_escorts"),
            Some(RuleKey::GateEscorts)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::GateEscorts),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(22),
            Some(&RuleKey::GateEscorts)
        );
    }

    #[test]
    fn the_lost_cargo_share_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::LostCargoShare.key(), "lost_cargo_share");
        assert_eq!(
            RuleKey::from_key("lost_cargo_share"),
            Some(RuleKey::LostCargoShare)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::LostCargoShare),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(21),
            Some(&RuleKey::LostCargoShare)
        );
    }

    #[test]
    fn the_non_strict_speed_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::NonStrictSpeed.key(), "non_strict_speed");
        assert_eq!(
            RuleKey::from_key("non_strict_speed"),
            Some(RuleKey::NonStrictSpeed)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::NonStrictSpeed),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(20),
            Some(&RuleKey::NonStrictSpeed)
        );
    }

    #[test]
    fn the_hunter_bribe_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::HunterBribe.key(), "hunter_bribe");
        assert_eq!(
            RuleKey::from_key("hunter_bribe"),
            Some(RuleKey::HunterBribe)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::HunterBribe),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(19),
            Some(&RuleKey::HunterBribe)
        );
    }

    #[test]
    fn the_ship_overlap_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::ShipOverlap.key(), "ship_overlap");
        assert_eq!(
            RuleKey::from_key("ship_overlap"),
            Some(RuleKey::ShipOverlap)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::ShipOverlap),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(18),
            Some(&RuleKey::ShipOverlap)
        );
    }

    #[test]
    fn the_quadrant_turret_arc_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::QuadrantTurretArc.key(), "quadrant_turret_arc");
        assert_eq!(
            RuleKey::from_key("quadrant_turret_arc"),
            Some(RuleKey::QuadrantTurretArc)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::QuadrantTurretArc),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(17),
            Some(&RuleKey::QuadrantTurretArc)
        );
    }

    #[test]
    fn the_hidden_ships_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::HiddenShips.key(), "hidden_ships");
        assert_eq!(
            RuleKey::from_key("hidden_ships"),
            Some(RuleKey::HiddenShips)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::HiddenShips),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(16),
            Some(&RuleKey::HiddenShips)
        );
    }

    #[test]
    fn the_hidden_ship_hover_box_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::HiddenShipHoverBox.key(), "hidden_ship_hover_box");
        assert_eq!(
            RuleKey::from_key("hidden_ship_hover_box"),
            Some(RuleKey::HiddenShipHoverBox)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::HiddenShipHoverBox),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(15),
            Some(&RuleKey::HiddenShipHoverBox)
        );
    }

    #[test]
    fn the_breakup_stall_rule_is_found_by_its_key_and_defaults_to_the_other_reading() {
        assert_eq!(RuleKey::BreakupStall.key(), "breakup_stall");
        assert_eq!(
            RuleKey::from_key("breakup_stall"),
            Some(RuleKey::BreakupStall)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::BreakupStall),
            RuleSource::Bible
        );
        assert_eq!(
            Rulebook::new(RuleSource::Engine).source_for(RuleKey::BreakupStall),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(14),
            Some(&RuleKey::BreakupStall)
        );
    }

    #[test]
    fn the_eject_velocity_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::EjectVelocity.key(), "eject_velocity");
        assert_eq!(
            RuleKey::from_key("eject_velocity"),
            Some(RuleKey::EjectVelocity)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::EjectVelocity),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(13),
            Some(&RuleKey::EjectVelocity)
        );
    }

    #[test]
    fn the_rescue_armor_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::RescueArmor.key(), "rescue_armor");
        assert_eq!(
            RuleKey::from_key("rescue_armor"),
            Some(RuleKey::RescueArmor)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::RescueArmor),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(12),
            Some(&RuleKey::RescueArmor)
        );
    }

    #[test]
    fn the_unlisted_stellar_rule_is_found_by_its_key_and_defaults_to_the_other_reading() {
        assert_eq!(RuleKey::UnlistedStellar.key(), "unlisted_stellar");
        assert_eq!(
            RuleKey::from_key("unlisted_stellar"),
            Some(RuleKey::UnlistedStellar)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::UnlistedStellar),
            RuleSource::Bible
        );
        assert_eq!(
            Rulebook::new(RuleSource::Engine).source_for(RuleKey::UnlistedStellar),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(11),
            Some(&RuleKey::UnlistedStellar)
        );
    }

    #[test]
    fn the_newbie_fuel_refusal_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::NewbieFuelRefusal.key(), "newbie_fuel_refusal");
        assert_eq!(
            RuleKey::from_key("newbie_fuel_refusal"),
            Some(RuleKey::NewbieFuelRefusal)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::NewbieFuelRefusal),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(10),
            Some(&RuleKey::NewbieFuelRefusal)
        );
    }

    #[test]
    fn the_missing_buoy_string_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::MissingBuoyString.key(), "missing_buoy_string");
        assert_eq!(
            RuleKey::from_key("missing_buoy_string"),
            Some(RuleKey::MissingBuoyString)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::MissingBuoyString),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(9),
            Some(&RuleKey::MissingBuoyString)
        );
    }

    #[test]
    fn the_patrol_uninhabited_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::PatrolUninhabited.key(), "patrol_uninhabited");
        assert_eq!(
            RuleKey::from_key("patrol_uninhabited"),
            Some(RuleKey::PatrolUninhabited)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::PatrolUninhabited),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(8),
            Some(&RuleKey::PatrolUninhabited)
        );
    }

    #[test]
    fn the_gate_exit_counts_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::GateExitCounts.key(), "gate_exit_counts");
        assert_eq!(
            RuleKey::from_key("gate_exit_counts"),
            Some(RuleKey::GateExitCounts)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::GateExitCounts),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(7),
            Some(&RuleKey::GateExitCounts)
        );
    }

    #[test]
    fn the_spawn_reserve_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::SpawnReserve.key(), "spawn_reserve");
        assert_eq!(
            RuleKey::from_key("spawn_reserve"),
            Some(RuleKey::SpawnReserve)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::SpawnReserve),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(5),
            Some(&RuleKey::SpawnReserve)
        );
    }

    #[test]
    fn the_arrival_hits_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::ArrivalHits.key(), "arrival_hits");
        assert_eq!(
            RuleKey::from_key("arrival_hits"),
            Some(RuleKey::ArrivalHits)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::ArrivalHits),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(4),
            Some(&RuleKey::ArrivalHits)
        );
    }

    #[test]
    fn the_hire_no_slot_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::HireNoSlot.key(), "hire_no_slot");
        assert_eq!(RuleKey::from_key("hire_no_slot"), Some(RuleKey::HireNoSlot));
        assert_eq!(
            Rulebook::default().source_for(RuleKey::HireNoSlot),
            RuleSource::Engine
        );
        assert_eq!(RuleKey::ALL.iter().rev().nth(3), Some(&RuleKey::HireNoSlot));
    }

    #[test]
    fn the_asteroid_entry_field_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::AsteroidEntryField.key(), "asteroid_entry_field");
        assert_eq!(
            RuleKey::from_key("asteroid_entry_field"),
            Some(RuleKey::AsteroidEntryField)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::AsteroidEntryField),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(2),
            Some(&RuleKey::AsteroidEntryField)
        );
    }

    #[test]
    fn the_asteroid_respawn_ring_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::AsteroidRespawnRing.key(), "asteroid_respawn_ring");
        assert_eq!(
            RuleKey::from_key("asteroid_respawn_ring"),
            Some(RuleKey::AsteroidRespawnRing)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::AsteroidRespawnRing),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(1),
            Some(&RuleKey::AsteroidRespawnRing)
        );
    }

    #[test]
    fn the_fuse_ignores_asteroids_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(
            RuleKey::FuseIgnoresAsteroids.key(),
            "fuse_ignores_asteroids"
        );
        assert_eq!(
            RuleKey::from_key("fuse_ignores_asteroids"),
            Some(RuleKey::FuseIgnoresAsteroids)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::FuseIgnoresAsteroids),
            RuleSource::Engine
        );
        assert_eq!(RuleKey::ALL.last(), Some(&RuleKey::FuseIgnoresAsteroids));
    }

    #[test]
    fn the_move_left_behind_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::MoveLeftBehind.key(), "move_left_behind");
        assert_eq!(
            RuleKey::from_key("move_left_behind"),
            Some(RuleKey::MoveLeftBehind)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::MoveLeftBehind),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(6),
            Some(&RuleKey::MoveLeftBehind)
        );
    }

    #[test]
    fn the_aux_gate_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::AuxGate.key(), "aux_gate");
        assert_eq!(RuleKey::from_key("aux_gate"), Some(RuleKey::AuxGate));
        assert_eq!(
            Rulebook::default().source_for(RuleKey::AuxGate),
            RuleSource::Engine
        );
        assert_eq!(RuleKey::ALL.iter().rev().nth(54), Some(&RuleKey::AuxGate));
    }

    #[test]
    fn the_aux_abort_fall_through_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::AuxAbortFallThrough.key(), "aux_abort_fall_through");
        assert_eq!(
            RuleKey::from_key("aux_abort_fall_through"),
            Some(RuleKey::AuxAbortFallThrough)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::AuxAbortFallThrough),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(53),
            Some(&RuleKey::AuxAbortFallThrough)
        );
    }

    #[test]
    fn the_plain_board_counts_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::PlainBoardCounts.key(), "plain_board_counts");
        assert_eq!(
            RuleKey::from_key("plain_board_counts"),
            Some(RuleKey::PlainBoardCounts)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::PlainBoardCounts),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(52),
            Some(&RuleKey::PlainBoardCounts)
        );
    }

    #[test]
    fn the_hidden_loss_fails_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::HiddenLossFails.key(), "hidden_loss_fails");
        assert_eq!(
            RuleKey::from_key("hidden_loss_fails"),
            Some(RuleKey::HiddenLossFails)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::HiddenLossFails),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(51),
            Some(&RuleKey::HiddenLossFails)
        );
    }

    #[test]
    fn the_ship_done_once_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::ShipDoneOnce.key(), "ship_done_once");
        assert_eq!(
            RuleKey::from_key("ship_done_once"),
            Some(RuleKey::ShipDoneOnce)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::ShipDoneOnce),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(50),
            Some(&RuleKey::ShipDoneOnce)
        );
    }

    #[test]
    fn the_swap_arrives_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::SwapArrives.key(), "swap_arrives");
        assert_eq!(
            RuleKey::from_key("swap_arrives"),
            Some(RuleKey::SwapArrives)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::SwapArrives),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(49),
            Some(&RuleKey::SwapArrives)
        );
    }

    #[test]
    fn the_smuggling_ship_name_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::SmugglingShipName.key(), "smuggling_ship_name");
        assert_eq!(
            RuleKey::from_key("smuggling_ship_name"),
            Some(RuleKey::SmugglingShipName)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::SmugglingShipName),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(48),
            Some(&RuleKey::SmugglingShipName)
        );
    }

    #[test]
    fn the_hud_cadence_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::HudCadence.key(), "hud_cadence");
        assert_eq!(RuleKey::from_key("hud_cadence"), Some(RuleKey::HudCadence));
        assert_eq!(
            Rulebook::default().source_for(RuleKey::HudCadence),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(47),
            Some(&RuleKey::HudCadence)
        );
    }

    #[test]
    fn the_alert_blink_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::AlertBlink.key(), "alert_blink");
        assert_eq!(RuleKey::from_key("alert_blink"), Some(RuleKey::AlertBlink));
        assert_eq!(
            Rulebook::default().source_for(RuleKey::AlertBlink),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(46),
            Some(&RuleKey::AlertBlink)
        );
    }

    #[test]
    fn the_legal_records_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::LegalRecords.key(), "legal_records");
        assert_eq!(
            RuleKey::from_key("legal_records"),
            Some(RuleKey::LegalRecords)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(58),
            Some(&RuleKey::LegalRecords)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::LegalRecords),
            RuleSource::Engine
        );
    }

    #[test]
    fn the_cron_rules_are_found_by_their_keys_and_read_the_engine() {
        assert_eq!(RuleKey::CronRandomScale.key(), "cron_random_scale");
        assert_eq!(RuleKey::CronDateCompare.key(), "cron_date_compare");
        for rule in [RuleKey::CronRandomScale, RuleKey::CronDateCompare] {
            assert_eq!(RuleKey::from_key(rule.key()), Some(rule));
            assert_eq!(Rulebook::default().source_for(rule), RuleSource::Engine);
        }
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(44),
            Some(&RuleKey::CronDateCompare)
        );
    }

    #[test]
    fn the_news_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::NewsGovtMatch.key(), "news_govt_match");
        assert_eq!(
            RuleKey::from_key("news_govt_match"),
            Some(RuleKey::NewsGovtMatch)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::NewsGovtMatch),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(43),
            Some(&RuleKey::NewsGovtMatch)
        );
    }

    #[test]
    fn the_rank_rules_are_found_by_their_keys_and_read_the_engine() {
        assert_eq!(
            RuleKey::RankUnaffiliatedGroup.key(),
            "rank_unaffiliated_group"
        );
        assert_eq!(RuleKey::SalaryCap.key(), "salary_cap");
        for rule in [RuleKey::RankUnaffiliatedGroup, RuleKey::SalaryCap] {
            assert_eq!(RuleKey::from_key(rule.key()), Some(rule));
            assert_eq!(Rulebook::default().source_for(rule), RuleSource::Engine);
        }
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(42),
            Some(&RuleKey::RankUnaffiliatedGroup)
        );
        assert_eq!(RuleKey::ALL.iter().rev().nth(41), Some(&RuleKey::SalaryCap));
    }

    #[test]
    fn the_rank_govt_reach_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::RankGovtReach.key(), "rank_govt_reach");
        assert_eq!(
            RuleKey::from_key("rank_govt_reach"),
            Some(RuleKey::RankGovtReach)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::RankGovtReach),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(40),
            Some(&RuleKey::RankGovtReach)
        );
    }

    #[test]
    fn the_rank_price_rules_are_found_by_their_keys_and_read_the_engine() {
        assert_eq!(RuleKey::RankPriceScope.key(), "rank_price_scope");
        assert_eq!(RuleKey::RankTradeIn.key(), "rank_trade_in");
        assert_eq!(RuleKey::RankPriceTiming.key(), "rank_price_timing");
        for rule in [
            RuleKey::RankPriceScope,
            RuleKey::RankTradeIn,
            RuleKey::RankPriceTiming,
        ] {
            assert_eq!(RuleKey::from_key(rule.key()), Some(rule));
            assert_eq!(Rulebook::default().source_for(rule), RuleSource::Engine);
        }
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(39),
            Some(&RuleKey::RankPriceScope)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(38),
            Some(&RuleKey::RankTradeIn)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(37),
            Some(&RuleKey::RankPriceTiming)
        );
    }

    #[test]
    fn the_rank_crime_reach_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::RankCrimeReach.key(), "rank_crime_reach");
        assert_eq!(
            RuleKey::from_key("rank_crime_reach"),
            Some(RuleKey::RankCrimeReach)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::RankCrimeReach),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(36),
            Some(&RuleKey::RankCrimeReach)
        );
    }

    #[test]
    fn the_galaxy_visibility_rules_are_found_by_their_keys_and_read_the_engine() {
        assert_eq!(RuleKey::ColocatedResolution.key(), "colocated_resolution");
        assert_eq!(RuleKey::VisibilityRefresh.key(), "visibility_refresh");
        for rule in [RuleKey::ColocatedResolution, RuleKey::VisibilityRefresh] {
            assert_eq!(RuleKey::from_key(rule.key()), Some(rule));
            assert_eq!(Rulebook::default().source_for(rule), RuleSource::Engine);
        }
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(35),
            Some(&RuleKey::ColocatedResolution)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(34),
            Some(&RuleKey::VisibilityRefresh)
        );
    }

    #[test]
    fn the_cron_holdoff_rules_are_found_by_their_keys_and_read_the_bible_by_default() {
        assert_eq!(
            RuleKey::CronZeroDurationHoldoff.key(),
            "cron_zero_duration_holdoff"
        );
        assert_eq!(RuleKey::CronPostHoldoff.key(), "cron_post_holdoff");
        for rule in [RuleKey::CronZeroDurationHoldoff, RuleKey::CronPostHoldoff] {
            assert_eq!(RuleKey::from_key(rule.key()), Some(rule));
            assert_eq!(Rulebook::default().source_for(rule), RuleSource::Bible);
            assert_eq!(
                Rulebook::new(RuleSource::Engine).source_for(rule),
                RuleSource::Engine
            );
        }
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(33),
            Some(&RuleKey::CronZeroDurationHoldoff)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(32),
            Some(&RuleKey::CronPostHoldoff)
        );
    }

    #[test]
    fn the_blessing_rules_are_found_by_their_keys_and_read_the_engine() {
        assert_eq!(RuleKey::BlessingRefresh.key(), "blessing_refresh");
        assert_eq!(RuleKey::BlessingLanding.key(), "blessing_landing");
        for rule in [RuleKey::BlessingRefresh, RuleKey::BlessingLanding] {
            assert_eq!(RuleKey::from_key(rule.key()), Some(rule));
            assert_eq!(Rulebook::default().source_for(rule), RuleSource::Engine);
        }
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(31),
            Some(&RuleKey::BlessingRefresh)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(30),
            Some(&RuleKey::BlessingLanding)
        );
    }

    #[test]
    fn the_blessing_always_attacks_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(
            RuleKey::BlessingAlwaysAttacks.key(),
            "blessing_always_attacks"
        );
        assert_eq!(
            RuleKey::from_key("blessing_always_attacks"),
            Some(RuleKey::BlessingAlwaysAttacks)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::BlessingAlwaysAttacks),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(29),
            Some(&RuleKey::BlessingAlwaysAttacks)
        );
    }

    #[test]
    fn the_explored_by_place_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::ExploredByPlace.key(), "explored_by_place");
        assert_eq!(
            RuleKey::from_key("explored_by_place"),
            Some(RuleKey::ExploredByPlace)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::ExploredByPlace),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(28),
            Some(&RuleKey::ExploredByPlace)
        );
    }

    #[test]
    fn the_route_rewrite_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::RouteRewrite.key(), "route_rewrite");
        assert_eq!(
            RuleKey::from_key("route_rewrite"),
            Some(RuleKey::RouteRewrite)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::RouteRewrite),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(27),
            Some(&RuleKey::RouteRewrite)
        );
    }

    #[test]
    fn the_nebula_explore_once_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::NebulaExploreOnce.key(), "nebula_explore_once");
        assert_eq!(
            RuleKey::from_key("nebula_explore_once"),
            Some(RuleKey::NebulaExploreOnce)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::NebulaExploreOnce),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(26),
            Some(&RuleKey::NebulaExploreOnce)
        );
    }

    #[test]
    fn the_nebula_new_pilot_rule_is_found_by_its_key_and_reads_the_engine() {
        assert_eq!(RuleKey::NebulaNewPilot.key(), "nebula_new_pilot");
        assert_eq!(
            RuleKey::from_key("nebula_new_pilot"),
            Some(RuleKey::NebulaNewPilot)
        );
        assert_eq!(
            Rulebook::default().source_for(RuleKey::NebulaNewPilot),
            RuleSource::Engine
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(25),
            Some(&RuleKey::NebulaNewPilot)
        );
    }

    #[test]
    fn the_scan_rules_are_found_by_their_keys_and_read_the_engine() {
        assert_eq!(RuleKey::ScanOutfitsPastJunk.key(), "scan_outfits_past_junk");
        assert_eq!(RuleKey::OutfitScanFine.key(), "outfit_scan_fine");
        for rule in [RuleKey::ScanOutfitsPastJunk, RuleKey::OutfitScanFine] {
            assert_eq!(RuleKey::from_key(rule.key()), Some(rule));
            assert_eq!(Rulebook::default().source_for(rule), RuleSource::Engine);
        }
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(24),
            Some(&RuleKey::ScanOutfitsPastJunk)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(23),
            Some(&RuleKey::OutfitScanFine)
        );
    }

    #[test]
    fn the_grid_selection_rule_is_found_by_its_key() {
        assert_eq!(RuleKey::GridSelection.key(), "grid_selection");
        assert_eq!(
            RuleKey::from_key("grid_selection"),
            Some(RuleKey::GridSelection)
        );
    }

    #[test]
    fn the_outfitters_rules_have_settings_keys_found_by_name() {
        assert_eq!(RuleKey::OutfitRefund.key(), "outfit_refund");
        assert_eq!(
            RuleKey::from_key("outfit_refund"),
            Some(RuleKey::OutfitRefund)
        );
        assert_eq!(RuleKey::OutfitLimit.key(), "outfit_limit");
        assert_eq!(
            RuleKey::from_key("outfit_limit"),
            Some(RuleKey::OutfitLimit)
        );
        assert_eq!(RuleKey::SaleMass.key(), "sale_mass");
        assert_eq!(RuleKey::from_key("sale_mass"), Some(RuleKey::SaleMass));
        assert_eq!(RuleKey::TradeInOutfits.key(), "trade_in_outfits");
        assert_eq!(
            RuleKey::from_key("trade_in_outfits"),
            Some(RuleKey::TradeInOutfits)
        );
    }

    #[test]
    fn the_scripting_rules_have_settings_keys() {
        assert_eq!(RuleKey::MoveStarless.key(), "move_starless");
        assert_eq!(RuleKey::MoveArrival.key(), "move_arrival");
        assert_eq!(RuleKey::MoveKeepFlag.key(), "move_keep_flag");
        assert_eq!(RuleKey::BlankLeave.key(), "blank_leave");
        assert_eq!(RuleKey::ScriptSound.key(), "script_sound");
        assert_eq!(
            RuleKey::ShipChangePersistence.key(),
            "ship_change_persistence"
        );
        assert_eq!(RuleKey::ShipChangeMax.key(), "ship_change_max");
        assert_eq!(RuleKey::ShipChangeCargo.key(), "ship_change_cargo");
        assert_eq!(RuleKey::ShipChangeReserves.key(), "ship_change_reserves");
        assert_eq!(RuleKey::ShipChangeBayAmmo.key(), "ship_change_bay_ammo");
        assert_eq!(
            RuleKey::ShipChangeDefaultWeapons.key(),
            "ship_change_default_weapons"
        );
        assert_eq!(
            RuleKey::ShipChangeNegativeStock.key(),
            "ship_change_negative_stock"
        );
        assert_eq!(RuleKey::PurchasePaintOrder.key(), "purchase_paint_order");
        assert_eq!(RuleKey::CaptureHookOrder.key(), "capture_hook_order");
        assert_eq!(RuleKey::StartShipPurchase.key(), "start_ship_purchase");
        assert_eq!(RuleKey::MapExplore.key(), "map_explore");
        assert_eq!(RuleKey::InvalidMap.key(), "invalid_map");
        assert_eq!(RuleKey::RemoveRefund.key(), "remove_refund");
        assert_eq!(RuleKey::RefitReserves.key(), "refit_reserves");
    }

    #[test]
    fn the_shop_and_trade_rules_are_found_by_their_keys() {
        assert_eq!(RuleKey::TradeLot.key(), "trade_lot");
        assert_eq!(RuleKey::from_key("trade_lot"), Some(RuleKey::TradeLot));
        assert_eq!(RuleKey::JunkTrade.key(), "junk_trade");
        assert_eq!(RuleKey::from_key("junk_trade"), Some(RuleKey::JunkTrade));
        assert_eq!(RuleKey::JunkPrice.key(), "junk_price");
        assert_eq!(RuleKey::from_key("junk_price"), Some(RuleKey::JunkPrice));
        assert_eq!(RuleKey::PurchaseCargo.key(), "purchase_cargo");
        assert_eq!(
            RuleKey::from_key("purchase_cargo"),
            Some(RuleKey::PurchaseCargo)
        );
        assert_eq!(RuleKey::OutfitSaleCargo.key(), "outfit_sale_cargo");
        assert_eq!(
            RuleKey::from_key("outfit_sale_cargo"),
            Some(RuleKey::OutfitSaleCargo)
        );
        assert_eq!(RuleKey::ReleaseCargoShare.key(), "release_cargo_share");
        assert_eq!(
            RuleKey::from_key("release_cargo_share"),
            Some(RuleKey::ReleaseCargoShare)
        );
        assert_eq!(RuleKey::CaptureCargo.key(), "capture_cargo");
        assert_eq!(
            RuleKey::from_key("capture_cargo"),
            Some(RuleKey::CaptureCargo)
        );
        assert_eq!(RuleKey::LeaderlessSwapHold.key(), "leaderless_swap_hold");
        assert_eq!(
            RuleKey::from_key("leaderless_swap_hold"),
            Some(RuleKey::LeaderlessSwapHold)
        );
        assert_eq!(RuleKey::EventPrice.key(), "event_price");
        assert_eq!(RuleKey::from_key("event_price"), Some(RuleKey::EventPrice));
        assert_eq!(RuleKey::RaisedMax.key(), "raised_max");
        assert_eq!(RuleKey::from_key("raised_max"), Some(RuleKey::RaisedMax));
        assert_eq!(RuleKey::LauncherSale.key(), "launcher_sale");
        assert_eq!(
            RuleKey::from_key("launcher_sale"),
            Some(RuleKey::LauncherSale)
        );
        assert_eq!(RuleKey::JunkFlags.key(), "junk_flags");
        assert_eq!(RuleKey::from_key("junk_flags"), Some(RuleKey::JunkFlags));
        assert_eq!(RuleKey::BuyRandom.key(), "buy_random");
        assert_eq!(RuleKey::from_key("buy_random"), Some(RuleKey::BuyRandom));
        assert_eq!(RuleKey::PurchaseCarry.key(), "purchase_carry");
        assert_eq!(
            RuleKey::from_key("purchase_carry"),
            Some(RuleKey::PurchaseCarry)
        );
        assert_eq!(RuleKey::ShipInfoRounding.key(), "ship_info_rounding");
        assert_eq!(
            RuleKey::from_key("ship_info_rounding"),
            Some(RuleKey::ShipInfoRounding)
        );
        assert_eq!(
            RuleKey::ALL.iter().rev().nth(67),
            Some(&RuleKey::SecondarySale)
        );
    }

    #[test]
    fn the_secondary_selection_rules_are_found_by_their_keys() {
        assert_eq!(RuleKey::StaleSecondary.key(), "stale_secondary");
        assert_eq!(
            RuleKey::from_key("stale_secondary"),
            Some(RuleKey::StaleSecondary)
        );
        assert_eq!(RuleKey::SecondarySale.key(), "secondary_sale");
        assert_eq!(
            RuleKey::from_key("secondary_sale"),
            Some(RuleKey::SecondarySale)
        );
    }

    #[test]
    fn the_story_rules_are_found_by_their_keys() {
        assert_eq!(RuleKey::StayDays.key(), "stay_days");
        assert_eq!(RuleKey::from_key("stay_days"), Some(RuleKey::StayDays));
        assert_eq!(RuleKey::StoryReentry.key(), "story_reentry");
        assert_eq!(
            RuleKey::from_key("story_reentry"),
            Some(RuleKey::StoryReentry)
        );
        assert_eq!(RuleKey::DestroyInvincible.key(), "destroy_invincible");
        assert_eq!(
            RuleKey::from_key("destroy_invincible"),
            Some(RuleKey::DestroyInvincible)
        );
        assert_eq!(RuleKey::DayRolls.key(), "day_rolls");
        assert_eq!(RuleKey::from_key("day_rolls"), Some(RuleKey::DayRolls));
        assert_eq!(RuleKey::YOnDestroy.key(), "y_on_destroy");
        assert_eq!(RuleKey::from_key("y_on_destroy"), Some(RuleKey::YOnDestroy));
        assert_eq!(RuleKey::DeadTypeFallback.key(), "dead_type_fallback");
        assert_eq!(
            RuleKey::from_key("dead_type_fallback"),
            Some(RuleKey::DeadTypeFallback)
        );
        assert_eq!(RuleKey::LandingDistance.key(), "landing_distance");
        assert_eq!(
            RuleKey::from_key("landing_distance"),
            Some(RuleKey::LandingDistance)
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)] // one line per rule key
    fn each_rule_has_a_settings_key_found_by_name() {
        assert_eq!(
            RuleKey::ALL,
            [
                RuleKey::CrimeGains,
                RuleKey::EmptyBooty,
                RuleKey::CrewlessCapture,
                RuleKey::PiracyPolice,
                RuleKey::QuietHails,
                RuleKey::LongAdvice,
                RuleKey::EscortAi,
                RuleKey::EscortOrders,
                RuleKey::FighterLaunch,
                RuleKey::FighterRecall,
                RuleKey::HireRequire,
                RuleKey::TakeOffPay,
                RuleKey::StayDays,
                RuleKey::StoryReentry,
                RuleKey::DestroyInvincible,
                RuleKey::DayRolls,
                RuleKey::YOnDestroy,
                RuleKey::DeadTypeFallback,
                RuleKey::LandingDistance,
                RuleKey::HireFee,
                RuleKey::EscortWage,
                RuleKey::PersonOdds,
                RuleKey::SystemPersons,
                RuleKey::LinkSystSlip,
                RuleKey::ShieldMod,
                RuleKey::DerelictSetupSpeed,
                RuleKey::PersonCoward,
                RuleKey::PersonCredits,
                RuleKey::CommQuote,
                RuleKey::GrantCount,
                RuleKey::GrantMax,
                RuleKey::PersonJoin,
                RuleKey::MapExplore,
                RuleKey::InvalidMap,
                RuleKey::RemoveRefund,
                RuleKey::RefitReserves,
                RuleKey::PurchasePaintOrder,
                RuleKey::CaptureHookOrder,
                RuleKey::StartShipPurchase,
                RuleKey::ShipChangePersistence,
                RuleKey::ShipChangeMax,
                RuleKey::ShipChangeCargo,
                RuleKey::ShipChangeReserves,
                RuleKey::ShipChangeBayAmmo,
                RuleKey::ShipChangeDefaultWeapons,
                RuleKey::ShipChangeNegativeStock,
                RuleKey::MoveStarless,
                RuleKey::MoveArrival,
                RuleKey::MoveKeepFlag,
                RuleKey::BlankLeave,
                RuleKey::ScriptSound,
                RuleKey::BuyRandom,
                RuleKey::JunkFlags,
                RuleKey::LauncherSale,
                RuleKey::RaisedMax,
                RuleKey::EventPrice,
                RuleKey::PurchaseCargo,
                RuleKey::OutfitSaleCargo,
                RuleKey::ReleaseCargoShare,
                RuleKey::CaptureCargo,
                RuleKey::LeaderlessSwapHold,
                RuleKey::JunkPrice,
                RuleKey::JunkTrade,
                RuleKey::OutfitLimit,
                RuleKey::OutfitRefund,
                RuleKey::OutfitCount,
                RuleKey::SaleMass,
                RuleKey::SaleRefusal,
                RuleKey::CountedSale,
                RuleKey::RelistedSale,
                RuleKey::RelistedSelection,
                RuleKey::GridSelection,
                RuleKey::MassPrice,
                RuleKey::ScaledMass,
                RuleKey::TradeInOutfits,
                RuleKey::TradeLot,
                RuleKey::TradeQuotient,
                RuleKey::TradeCount,
                RuleKey::TradeDebt,
                RuleKey::TechLevel,
                RuleKey::PurchaseCarry,
                RuleKey::ShipInfoRounding,
                RuleKey::StaleSecondary,
                RuleKey::SecondarySale,
                RuleKey::RepeatMissionFailure,
                RuleKey::StartActiveMission,
                RuleKey::PayMillions,
                RuleKey::MissionOrder,
                RuleKey::PersonMissionStellar,
                RuleKey::MissionStellarDraw,
                RuleKey::MissionStellarDistance,
                RuleKey::MissionStellarIdentity,
                RuleKey::LegalRecords,
                RuleKey::LeftRespawn,
                RuleKey::LandedLeaves,
                RuleKey::GateFollow,
                RuleKey::AuxGate,
                RuleKey::AuxAbortFallThrough,
                RuleKey::PlainBoardCounts,
                RuleKey::HiddenLossFails,
                RuleKey::ShipDoneOnce,
                RuleKey::SwapArrives,
                RuleKey::SmugglingShipName,
                RuleKey::HudCadence,
                RuleKey::AlertBlink,
                RuleKey::CronRandomScale,
                RuleKey::CronDateCompare,
                RuleKey::NewsGovtMatch,
                RuleKey::RankUnaffiliatedGroup,
                RuleKey::SalaryCap,
                RuleKey::RankGovtReach,
                RuleKey::RankPriceScope,
                RuleKey::RankTradeIn,
                RuleKey::RankPriceTiming,
                RuleKey::RankCrimeReach,
                RuleKey::ColocatedResolution,
                RuleKey::VisibilityRefresh,
                RuleKey::CronZeroDurationHoldoff,
                RuleKey::CronPostHoldoff,
                RuleKey::BlessingRefresh,
                RuleKey::BlessingLanding,
                RuleKey::BlessingAlwaysAttacks,
                RuleKey::ExploredByPlace,
                RuleKey::RouteRewrite,
                RuleKey::NebulaExploreOnce,
                RuleKey::NebulaNewPilot,
                RuleKey::ScanOutfitsPastJunk,
                RuleKey::OutfitScanFine,
                RuleKey::GateEscorts,
                RuleKey::LostCargoShare,
                RuleKey::NonStrictSpeed,
                RuleKey::HunterBribe,
                RuleKey::ShipOverlap,
                RuleKey::QuadrantTurretArc,
                RuleKey::HiddenShips,
                RuleKey::HiddenShipHoverBox,
                RuleKey::BreakupStall,
                RuleKey::EjectVelocity,
                RuleKey::RescueArmor,
                RuleKey::UnlistedStellar,
                RuleKey::NewbieFuelRefusal,
                RuleKey::MissingBuoyString,
                RuleKey::PatrolUninhabited,
                RuleKey::GateExitCounts,
                RuleKey::MoveLeftBehind,
                RuleKey::SpawnReserve,
                RuleKey::ArrivalHits,
                RuleKey::HireNoSlot,
                RuleKey::AsteroidEntryField,
                RuleKey::AsteroidRespawnRing,
                RuleKey::FuseIgnoresAsteroids
            ]
        );
        assert_eq!(RuleKey::PersonJoin.key(), "person_join");
        assert_eq!(RuleKey::GrantCount.key(), "grant_count");
        assert_eq!(RuleKey::GrantMax.key(), "grant_max");
        assert_eq!(RuleKey::PersonOdds.key(), "person_odds");
        assert_eq!(RuleKey::SystemPersons.key(), "system_persons");
        assert_eq!(RuleKey::LinkSystSlip.key(), "link_syst_slip");
        assert_eq!(RuleKey::ShieldMod.key(), "shield_mod");
        assert_eq!(RuleKey::DerelictSetupSpeed.key(), "derelict_setup_speed");
        assert_eq!(RuleKey::PersonCoward.key(), "person_coward");
        assert_eq!(RuleKey::PersonCredits.key(), "person_credits");
        assert_eq!(RuleKey::CommQuote.key(), "comm_quote");
        assert_eq!(RuleKey::HireRequire.key(), "hire_require");
        assert_eq!(RuleKey::TakeOffPay.key(), "take_off_pay");
        assert_eq!(RuleKey::HireFee.key(), "hire_fee");
        assert_eq!(RuleKey::EscortWage.key(), "escort_wage");
        assert_eq!(RuleKey::FighterLaunch.key(), "fighter_launch");
        assert_eq!(RuleKey::FighterRecall.key(), "fighter_recall");
        assert_eq!(RuleKey::CrimeGains.key(), "crime_gains");
        assert_eq!(RuleKey::EmptyBooty.key(), "empty_booty");
        assert_eq!(RuleKey::CrewlessCapture.key(), "crewless_capture");
        assert_eq!(RuleKey::PiracyPolice.key(), "piracy_police");
        assert_eq!(RuleKey::QuietHails.key(), "quiet_hails");
        assert_eq!(RuleKey::LongAdvice.key(), "long_advice");
        assert_eq!(RuleKey::EscortAi.key(), "escort_ai");
        assert_eq!(RuleKey::EscortOrders.key(), "escort_orders");
        assert_eq!(RuleKey::ShipInfoRounding.key(), "ship_info_rounding");
        assert_eq!(RuleKey::UnlistedStellar.key(), "unlisted_stellar");
        assert_eq!(
            RuleKey::from_key("unlisted_stellar"),
            Some(RuleKey::UnlistedStellar)
        );
        for rule in RuleKey::ALL {
            assert_eq!(RuleKey::from_key(rule.key()), Some(rule));
        }
        assert_eq!(RuleKey::from_key("Crime_Gains"), None);
        assert_eq!(RuleKey::from_key(""), None);
    }

    #[test]
    fn each_source_has_a_name_found_by_name() {
        assert_eq!(RuleSource::ALL, [RuleSource::Engine, RuleSource::Bible]);
        assert_eq!(RuleSource::Engine.name(), "engine");
        assert_eq!(RuleSource::Bible.name(), "bible");
        for source in RuleSource::ALL {
            assert_eq!(RuleSource::from_name(source.name()), Some(source));
        }
        assert_eq!(RuleSource::from_name("Bible"), None);
        assert_eq!(RuleSource::from_name("both"), None);
    }
}
