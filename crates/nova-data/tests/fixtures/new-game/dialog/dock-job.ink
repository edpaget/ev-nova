// The new-game fixture's conversations for the storyline dock-work: the
// dock job's offer and completion (missions/dock-job.md:
// dialog/dock-job.ink#job_offer and #job_done), Verge Camp's own greeting
// (stellars/verge-camp.md: dialog: dialog/dock-job.ink#camp_greeting),
// and the wardens' toll and the road's offers (missions/warden-toll.md:
// #toll_offer, missions/clear-the-road.md: #road_offer).
// It follows game-bible's Ink conventions: a walkthrough for Inky, every
// scene ending at scene_end, every path out of an offer tagged with its
// outcome.
//
// dock_regular and hauling_plate are world flags: the engine sets them
// from the pilot's flags as a scene is entered, and the scenes only read
// them (the walkthrough sets them for Inky). The dock job's on_success
// sets dock_regular and clears hauling_plate before job_done is entered.
// The labelled route choice's read count is kept in the pilot, so a pilot
// who has asked about the route hears "You know the way by now." on a
// later offer and isn't offered the question again, and job_done, a scene
// of the same file, reads it too.
//
// No test runs inklecate. After editing, compile it again by hand, from
// the workspace root:
//   "/Applications/Inky.app/Contents/Resources/app.asar.unpacked/main-process/ink/inklecate_mac" \
//     -o crates/nova-data/tests/fixtures/new-game/dialog/dock-job.ink.json \
//     crates/nova-data/tests/fixtures/new-game/dialog/dock-job.ink

VAR dock_regular = false
VAR hauling_plate = false

-> walkthrough

=== walkthrough ===
~ dock_regular = job_done > 0
~ hauling_plate = job_offer > 0 && job_done == 0
Walkthrough: choose a scene.
+ [The dock job: the offer] -> job_offer
+ [The dock job: delivered] -> job_done
+ [Verge Camp: arriving] -> camp_greeting
+ [The wardens' toll: the offer] -> toll_offer
+ [Clear the road: the offer] -> road_offer
+ [Stop] -> END

=== scene_end ===
{ walkthrough:
    -> walkthrough
}
-> END

=== job_offer ===
{dock_regular:The dockmaster nods. "You again."|The dockmaster looks up from a slate.}
"Ten tons of hull plate for Verge Camp. It pays on delivery."
{route:"You know the way by now."}
+ [Take the job]
    "Bay two. Mind the dead gate past Verge." # outcome: accept
    -> scene_end
* (route) [Ask about the route]
    "Two gates lead to Verge. Either will do."
    ++ [Take it]
        "Bay two, then." # outcome: accept
        -> scene_end
    ++ [Pass]
        "Suit yourself." # outcome: decline
        -> scene_end
    ++ [Think it over]
        "Come back when you're sure." # outcome: defer
        -> scene_end
+ [Not now]
    "It'll keep a day or two." # outcome: defer
    -> scene_end

=== job_done ===
The camp boss signs for the plate.
{job_offer.route:"Found the road, then."|"Right on time."}
-> scene_end

=== camp_greeting ===
{hauling_plate:A warden waves you to bay one.|Wind snaps at the tents.}
-> scene_end

=== toll_offer ===
A warden leans on the camp's gate. "The side road's shut. Our road costs."
+ [Pay the toll]
    "Pleasure." # outcome: accept
    -> scene_end
+ [Refuse]
    "Then stay off the hearth road." # outcome: decline
    -> scene_end
+ [Later]
    "We'll be here." # outcome: defer
    -> scene_end

=== road_offer ===
The camp boss spreads a map. "Take the hearth road off the wardens."
+ [I'll do it]
    "Good hunting." # outcome: accept
    -> scene_end
+ [Not my fight]
    "Suit yourself." # outcome: decline
    -> scene_end
