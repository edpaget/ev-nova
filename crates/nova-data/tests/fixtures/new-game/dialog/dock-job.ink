// The new-game fixture's offer conversation, which Hearth Dock names
// (stellars/hearth-dock.md: dialog: dialog/dock-job.ink#job_offer).
// It follows game-bible's Ink conventions: a walkthrough for Inky, every
// scene ending at scene_end, every path out of the offer tagged.
//
// No test runs inklecate. After editing, compile it again by hand, from
// the workspace root:
//   "/Applications/Inky.app/Contents/Resources/app.asar.unpacked/main-process/ink/inklecate_mac" \
//     -o crates/nova-data/tests/fixtures/new-game/dialog/dock-job.ink.json \
//     crates/nova-data/tests/fixtures/new-game/dialog/dock-job.ink

-> walkthrough

=== walkthrough ===
Walkthrough: choose a scene.
+ [The dock job: the offer] -> job_offer
+ [Stop] -> END

=== scene_end ===
{ walkthrough:
    -> walkthrough
}
-> END

=== job_offer ===
The dockmaster looks up from a slate.
"Ten tons of hull plate for Verge Camp. It pays on delivery."
+ [Take the job]
    "Bay two. Mind the dead gate past Verge." # outcome: accept
    -> scene_end
* [Ask about the route]
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
