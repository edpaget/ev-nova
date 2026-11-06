// nova-ink's own fixture story: what the dialog port's contract story
// can't show without running Ink. Regenerate recall.ink.json with
// inklecate (see nova-ink's tests module).
VAR met = false
VAR name = "Tally"
LIST mood = calm, angry

-> END

=== ask ===
The clerk looks up.
* (work) [Ask about work] There's a run to Marrow.
    -> END
+ [Leave]
    # outcome: decline
    -> END

=== recall ===
{met: Welcome back, {name}.|Who are you?}
{ask.work: You asked about work.|You never asked.}
-> END

=== broken ===
This knot is a tunnel, entered as a knot.
->->
