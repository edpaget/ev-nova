// The new-game fixture's offer conversation, which Hearth Dock names
// (stellars/hearth-dock.md: dialog: dialog/dock-job.ink#job_offer).
// It follows game-bible's Ink conventions: a walkthrough for Inky, every
// scene ending at scene_end, every path out of the offer tagged.
//
// dock_regular is a world flag: the engine sets it from the pilot's flags
// as the offer is entered, and the scene only reads it (the walkthrough
// sets it for Inky). The labelled route choice's read count is kept in the
// pilot, so a pilot who has asked about the route hears "You know the way
// by now." on a later visit, and isn't offered the question again.
//
// No test runs inklecate. After editing, compile it again by hand, from
// the workspace root:
//   "/Applications/Inky.app/Contents/Resources/app.asar.unpacked/main-process/ink/inklecate_mac" \
//     -o crates/nova-data/tests/fixtures/new-game/dialog/dock-job.ink.json \
//     crates/nova-data/tests/fixtures/new-game/dialog/dock-job.ink

VAR dock_regular = false

-> walkthrough

=== walkthrough ===
~ dock_regular = job_offer > 0
Walkthrough: choose a scene.
+ [The dock job: the offer] -> job_offer
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
+ [Not now]
    "It'll keep a day or two." # outcome: defer
    -> scene_end
