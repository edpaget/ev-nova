// The sample fixture's conversations for the storyline holding-the-point,
// copied from game-bible's sample (data/dialog/holding-the-point.ink): the
// supply run's offer and completion (missions/supply-run.md:
// dialog/holding-the-point.ink#supply_run_offer and #supply_run_done), the
// crossroads offer (missions/the-crossroads-offer.md: #crossroads_offer),
// and take the point's briefing and completion (missions/take-the-point.md:
// #take_the_point_briefing and #take_the_point_done).
//
// No test runs inklecate. After editing, compile it again by hand, from
// the workspace root:
//   "/Applications/Inky.app/Contents/Resources/app.asar.unpacked/main-process/ink/inklecate_mac" \
//     -o crates/nova-data/tests/fixtures/sample/dialog/holding-the-point.ink.json \
//     crates/nova-data/tests/fixtures/sample/dialog/holding-the-point.ink
//
// Dialog for storyline 1, "Holding the point" (story/storylines.md).
// All names are placeholders.
//
// Conventions (see data/README.md):
// - A mission's offer knot ends every path with an outcome tag:
//   accept, decline or defer.
// - Every VAR is a world-state flag. The engine sets it before a knot runs;
//   the knot only reads it.
// - The engine keeps one story state per file, so read counts such as
//   {supply_run_done.monists} carry from one mission to the next.
// - The engine enters at a knot, so it never runs the walkthrough below.
//   Inky starts at the top of the file, so the walkthrough lets you play
//   every scene in one sitting, with read counts carrying between them.
//   It stands in for the engine by setting the flags.

VAR met_organizer = false

-> walkthrough

=== walkthrough ===
~ met_organizer = supply_run_done > 0
Walkthrough: choose a scene.
+ [Supply run: the offer] -> supply_run_offer
+ [Supply run: arrival at Marrow] -> supply_run_done
+ [The crossroads offer] -> crossroads_offer
+ [Take the point: the briefing] -> take_the_point_briefing
+ [Take the point: the gate is held] -> take_the_point_done
+ [Stop] -> END

// Where every scene ends: back to the list in Inky, out of the story in the game.
=== scene_end ===
{ walkthrough:
    -> walkthrough
}
-> END

=== supply_run_offer ===
The exchange clerk at Luyten Crossing doesn't look up from the manifest.
"Twelve tons for Marrow. Seed stock and pump parts. The commons pay on delivery, and they pay late."
+ [Take the job]
    "Dock four. If the Harrow boys hail you at the far gate, don't stop." # outcome: accept
    -> scene_end
* [Ask why nobody else will fly it]
    "Marrow's a long way to go for a short purse. And the Harrow band has been taxing the far gate all season."
    ++ [Take it anyway]
        "Dock four. Fly fast." # outcome: accept
        -> scene_end
    ++ [Pass]
        "Suit yourself. It'll be here tomorrow, and so will the price." # outcome: decline
        -> scene_end
+ [Pass]
    "Suit yourself." # outcome: decline
    -> scene_end

=== supply_run_done ===
The cargo bay opens on a station that is mostly patches. A person in a borrowed pressure suit signs for the crates without counting them.
"Tally Ashgrove. I organise things here, when they let me. You came in on the far gate?"
- (talk)
* [Ask who holds the far gate]
    "Nobody, properly. The commons can't afford to. The Harrow band taxes whoever it catches there, and the lord says it's his whenever there's a cut to be had."
    -> talk
* (monists) [Ask about the lights on the Lantern road]
    "The enclave. Monists. They keep to themselves and take in children nobody else will. Don't ask them for anything they haven't offered."
    -> talk
* [Ask what the commons need]
    "Ships that come back. Fuel at a fair price. Someone who'll fly the far gate when the band is out."
    -> talk
+ [Leave]
    "Come back when you're passing. We'll have work, if we're still here."
    -> scene_end

=== crossroads_offer ===
{ met_organizer:
    Tally is waiting at the dock, thinner than you remember.
    "You've heard about the Luyten road."
- else:
    A person in a borrowed pressure suit meets you at the dock.
    "Tally Ashgrove. I organise the commons. You came in on the far gate, so you've heard about the Luyten road."
}
"Half the Association run comes through Marrow now. That makes the far gate worth taking, and the Toll took it this morning."
- (options)
* [Ask what the warlord wants]
    "Tolls first. Then the Yard. Then us."
    -> options
* [Ask about the Crown's offer]
    "The agents from Kestrel Hold will protect the lords for an oath to the Crown. They don't say what happens to the commons, or to the enclave."
    -> options
* {supply_run_done.monists} [Ask whether the enclave would help]
    "They might. They'd want something nobody here can give them. Leave them out of it for now."
    -> options
+ [Offer to help the commons hold the point]
    "You know what that means. The Toll will want you dead, and when the Crown hears, so will they."
    ++ [Commit to the commons]
        "Then we start with the blockade." # outcome: accept
        -> scene_end
    ++ [Step back]
        "I thought so. Think on it, but not for long." # outcome: defer
        -> scene_end
+ [Tell them to take the Crown's protection]
    "Maybe we will. Maybe they'd even keep their word." # outcome: decline
    -> scene_end
+ [Say you need time]
    "Don't take long. The Toll won't." # outcome: defer
    -> scene_end

=== take_the_point_briefing ===
"The Toll has gunships on the far gate and a picket at the Hollow road. Break the blockade and they'll call for help from Harrow Drift. Be waiting at the Hollow road when it comes through."
+ [Fly the blockade]
    "We'll have the Yard's guns ready, such as they are." # outcome: accept
    -> scene_end
+ [Not yet]
    "Every day they hold it, they take a cut of everything we have." # outcome: defer
    -> scene_end

=== take_the_point_done ===
The far gate's beacon comes back on in the commons' colours. Someone on the Yard has painted over the band's mark by hand.
"It's ours," Tally says, as if trying out the words. "Now we find out what that costs."
-> scene_end
