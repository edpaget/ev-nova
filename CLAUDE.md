# CLAUDE.md

## Test-driven development

Write all code test-first:

1. **Red**: write a failing test for the next small piece of behavior. Run it
   and confirm it fails for the reason you expect.
2. **Green**: write the minimum code that makes the test pass.
3. **Refactor**: clean up the test and the implementation while the suite stays green.

- Write production code only in response to a failing test.
- Start every bug fix with a test that reproduces the bug.
- Keep tests fast and deterministic. Build test inputs from small fixture data
  checked into the repo, or construct them in code.

## Architecture: ports and adapters

Structure the code as ports and adapters (hexagonal architecture):

- **Core logic** (parsing, game rules, data models) is plain Rust that talks to
  the outside world only through ports.
- **Ports** are traits that the core defines. Each one describes a capability
  the core needs, in the core's own terms (e.g. "fetch resource bytes by type
  and ID"), not in terms of a specific technology.
- **Adapters** are concrete implementations of ports that touch real things:
  the filesystem, resource forks, git, processes, the clock, the network.
  Keep adapters thin; put decisions in the core.

When building a component:

1. Define the port traits for the collaborators it needs.
2. Take those collaborators as generics (`impl Trait`, `T: Trait`) or
   `&dyn Trait`, supplied by the caller.
3. Wire real adapters together at the edge of the program (`main`, or a
   top-level constructor).

When a UI drives core logic, it reaches it through a port the UI defines,
small enough to fake by hand:

- Grow the port with data, not methods: a new readout is a field of the frame
  the port fills, a new action a command variant, a new one-shot output an
  event variant, and a question a screen asks a variant of the screen's ask.
  Don't add a method per question.
- Keep each adapter forward to one call into the core. The decisions live in
  the core and are pinned by the core's tests.
- Test the UI's components on the hand-written fake. Run the few integration
  tests over the real core only where the port's module docs say.
- The port's module docs say where each kind of addition goes and where it is
  pinned.

## Porting the original's behaviour

Work that reads `EV Nova.app` to match the original follows these rules, so a
single investigation doesn't turn into a trail of follow-ups.

### Port the whole function

When a phase reads one of the original's functions, port all of it, not just
the branch the phase's feature needs.

- Build every branch the engine can model now.
- Record each branch you leave out in the phase body, with its address and
  the reason it was left out.
- Put all the branches left out of one function into a single follow-up,
  never one follow-up per branch.

### Sounds come with the function

A `PlaySound` or `_beepSnd` call is part of the function that makes it. Raise
it as a `SimSound` event in the same phase, and record which `snd ` it plays.
Add the `SimSound` variant even if the audio side can't play it yet. Don't
leave the sound for a later phase.

### Don't match the order of random draws

The engine's randomness (`SplitMix64` in `crates/nova/src/chance.rs`) isn't the
original's generator. Making the same draws in the same order can't reproduce
the original's sequence, so it changes nothing the player can see. Don't plan,
file or test for draw order. Match what a draw decides, such as its range,
its condition and what it sets.

### Ordering against the original

The engine may settle work later in a tick than the original does: in
reconciliation passes, in story subscriptions, or in after-frame hooks. It must
settle before any boundary where the player can see or keep the state: a frame
is drawn, a dialog opens, a landing or jump completes, or the pilot is saved.

- An ordering difference that crosses one of those boundaries is a
  divergence. Fix it or file it.
- An ordering difference that doesn't cross one, such as a one-tick lag the
  player never sees, isn't filed. Note it in the doc comment of the code that
  defers.

### Unknowns become investigations, not questions

When a phase depends on what the original does (a rate, a formula, a format,
a roll's timing), don't ask the user and don't leave it open. Make it step 1
of the phase: find it in the binary (`EV Nova.app`, i386 with symbols), record
the finding and its addresses in the phase body, and branch the later steps on
the answer. Before disassembling a function, run `rdm search <symbol>
--project ev-nova`: earlier phases record what they found, with addresses.

### When the original and another reading disagree

The original's behaviour is the default. A deviation from it needs a reason,
and a deviation with a reason becomes a setting, not a new default. The
original's bugs and limits are the only exceptions.

- When there's a reason to read a rule another way (the Nova Bible
  disagrees, our engine already behaves differently, or a plan said
  otherwise), add a `Rulebook` key (`crates/nova-sim-base/src/rulebook.rs`).
  The original's reading (`RuleSource::Engine`, named for the original
  executable) is the default, the other reading is opt-in, and tests cover
  both. Ask the user only when the alternative isn't obvious, the scope
  grows, or the change alters a control everyone uses.
- If the original has a bug that breaks stock missions, crons or storylines,
  make the intended behaviour the default and the bug the opt-in reading.
  Check the stock data before calling something broken.
- If the original has a resource limit (a cap on shots, beams or
  explosions kept in memory), default to no limit and make the cap the
  opt-in reading.
- Don't replicate integer widths, wraps or overflows (16-bit cargo, 32-bit
  cash). Use wider numbers. Mention the width in the finding, but don't file
  a phase or key for it.
- Build what the original does even when the difference is small: a pixel,
  sub-second timing, a click that acts on the press. Record without building
  only what can't be replicated, such as a read of uninitialised memory.

### Where new findings go

File every divergence the binary shows, however niche.

- If it serves the roadmap's goal, add it as a new phase of that roadmap.
- On a feature roadmap, a finding that doesn't block playing the stock
  storyline goes to `<slug>-fidelity` instead. Create that roadmap, depending
  on the feature roadmap, the first time it's needed.
- If the roadmap is finished, file a task, not a phase.

### Phase caps, follow-ups and dependencies

A roadmap that keeps growing never finishes, and it blocks the roadmaps
that depend on it.

- A roadmap holds at most 20 phases. Phases found during its run are added
  until it reaches 20.
- Past 20, a finding that needs only the roadmap's own branch goes to
  `<slug>-followups`. Create that roadmap the first time it's needed, with
  `rdm roadmap depend` on the capped roadmap.
- A finding that waits on another roadmap's not-started or unlanded work goes
  to `<slug>-cross-roadmap` at any phase count. That roadmap depends on the
  origin roadmap and on every roadmap the work waits on. A finding that waits
  on a task names the task in its body, since a task can't be a dependency.
- The existing phases stay, and amending them is fine. When a phase moves to
  one of these roadmaps, repoint the `rdm:phase/...` links that named it.

## Testing with hand-written mocks

- Test core logic through its ports. In each test module, write small mocks or
  fakes that implement the port traits. Mocks return canned data and, when the
  interaction matters, record the calls they receive so the test can assert on them.
- Test each component on its own, with mocks standing in for its collaborators.
  When a component is hard to test this way, add a port at that boundary.
- Test each real adapter on its own against the behaviour that the port's mocks
  assume. The crate that owns a port publishes its fake and a contract check,
  and every real adapter runs that check; don't write another crate's fake.
- Cover wiring with a small number of integration tests under `tests/`.

## Crate coupling

The pre-commit hook tests only the crates with staged changes (see
"Verification runs through cargo"), and it stays that way: don't widen it to
dependents. Instead, keep crates loose enough that a change to one breaks no
other crate's tests.

- A test asserts only its own crate's rules. Don't pin a rule, constant,
  default or timing an upstream crate owns; that crate's tests pin it.
- Reach upstream state through the builders and fakes the upstream crate
  publishes for tests, not by driving its real behaviour until it gets there.
- Don't match exhaustively on an upstream enum that grows. Have the owning
  crate supply what each variant needs, as data on the enum. Keep an
  exhaustive match only where the downstream crate must act on every
  variant, and say so in its doc comment.
- A test that runs a real upstream crate covers wiring only. It lives in the
  crate that wires the program together, or where a port's module docs place
  it (see "Architecture: ports and adapters"). The full gate runs it, not the
  hook of the crates it reaches.
- Each edge between workspace crates is a `bans` rule in `deny.toml` (see
  "Test behaviour, never text"). A new edge adds its rule.

## Test behaviour, never text

Don't write tests that scan source files or built binaries for particular
strings, such as a test that greps `src/` for a call, or reads the `nova`
binary to check that a crate's name is absent. They break on changes that
don't matter: two unrelated string literals once spelled `made` + `guidance`
side by side in a binary and failed an "egui is not linked" test. And what
they show is weak.

- Test what the code does, through its ports.
- Check dependency rules (which crates a build may pull in) with
  `cargo-deny`'s `bans` in `deny.toml` (`mise run deny`), which reads Cargo's
  resolved graph.

## Hermetic tests

Every test runs anywhere with only `cargo`, in any order, in parallel, and
leaves the developer's machine exactly as it found it.

- Reach git, the filesystem, environment variables, and other processes through
  ports, so unit tests use mocks instead of the real thing.
- When an adapter test needs real files, create them in a fresh temporary
  directory owned by that test, and pass the path in explicitly. Keep the
  working directory, `$HOME`, environment variables, and the project's own git
  repository untouched.
- Set up any state a test needs inside the test itself, so it passes on a clean
  checkout.

## Large files

`crates/nova-sim/src/session.rs`, `crates/nova-sim-combat/src/combat/mod.rs`
and `crates/nova-sim-base/src/rulebook.rs` run to thousands of lines. Agents read them thousands of times a week.

- Find the code first with `grep -n` or LSP symbols, then read only that range
  with the `Read` tool (`offset`/`limit`).
- Read with `Read`, not `sed`, `cat` or `head`: `Edit` refuses a file that
  `Read` hasn't seen, so a `sed` read costs a second read before any edit.

Files grow large one sensible append at a time, so decide where new code goes
before writing it:

- When a change adds a new concern (a dialog, a routine of the original, a
  subsystem), put it in a new sibling module with its own tests. Don't append
  it to an existing large file because similar code lives there.
- If a file you're editing is over about 5,000 lines, don't split it in this
  change: a split mixed into other work is hard to review and conflicts with
  every branch that touches the file. Check for an rdm split task
  (`rdm search split --type task --project ev-nova`) and file one if there
  isn't one, naming the concerns that could come out. A project hook
  (`.claude/hooks/large-file-reminder.sh`) reminds you when you edit one.

## Session state

`Session` (`crates/nova-sim/src/session.rs`) coordinates owned sub-states,
each in its own module under `session/`: `Statics`, `Setup`, `Outbox`,
`PlayerShip`, `Space`, `Chart`, `Port`, `MissionRun` and `Mode`. The module
docs' "State" section says what each holds.

`Statics` and `Setup` are read-only bundles and `Outbox` is the sink for
events. The other six (`PlayerShip`, `Space`, `Chart`, `Port`, `MissionRun`
and `Mode`) are stateful.

- Put a new field on the sub-state it belongs to, not on `Session`.
- Put logic that touches one stateful sub-state in a method on it. Pass what
  else it reads as explicit parameters: `&Statics`, `&Setup`, `&mut Outbox`,
  `&Pilot` or `&mut Pilot`, a mode value such as
  `landed: Option<StellarId>`, or plain values the caller computes.
- A sub-state's method never takes another stateful sub-state (`PlayerShip`,
  `Space`, `Chart`, `Port`, `MissionRun`, `Mode`). If it needs two of those,
  it belongs on `Session`.
- Keep a method on `Session` only when it coordinates several stateful
  sub-states, writes the mode (a landing, a jump, the pod, the end), or is a
  `pub` entry point. Its body reaches the sub-states through their methods:
  `self.space.restock_fleet(&mut self.pilot, &self.statics, &self.setup)`
  borrows disjoint fields, where two `&mut self` calls wouldn't compile.
- A new field on `Session` itself needs a reason in its doc comment, saying
  why it belongs to no sub-state.

## Verification runs through cargo

Write all checks and verifiers as Rust tests (unit tests, integration tests
under `tests/`, or doctests) so `cargo` runs them.

- Run tests with `mise run test` (nextest plus doctests).
- In the TDD loop, run `cargo nextest run -p <crate>`, the same form the
  pre-commit hook uses, so its build stays warm. Don't switch between `-p` and
  `--workspace` in the loop: each switch rebuilds.
- Run the full gate with `mise run ci` (fmt, clippy with `-D warnings`, the
  `cargo-deny` bans, rustdoc with broken links denied, tests, doctests and the
  dev-tools legs). Run it once before moving an item to
  needs-review; `rdm:land` runs it again before landing. `hk check` runs it
  too.
- The pre-commit hook is lighter: `cargo fmt --check`, then clippy and nextest
  with `-p <crate>` for each crate with staged changes (and the workspace lint
  and tests when `Cargo.toml`, `Cargo.lock`, `mise.toml`, `clippy.toml`,
  `.config/nextest.toml` or `.cargo/` change). It skips dependent crates,
  doctests and the dev-tools legs, which the full gate covers.
- Don't run `mise run ci` just before a commit: the hook checks the commit
  anyway, so the gate would run twice.
- Keep `mise` tasks as one-line `cargo` invocations. When a check needs logic,
  write it in Rust.
- Integration tests live in `tests/it/`, as modules of one test binary per
  crate. Add a new `tests/it/<name>.rs` and a `mod <name>;` line in
  `tests/it/main.rs`, never a new `tests/*.rs` file: each of those is another
  binary to link on every build.

### Stock-data tests

Tests over the original game's data (the `stock_*` modules under each crate's
`tests/it/`, in `tests/it/stock_*.rs`) skip, passing, unless `NOVA_DATA`
points at a `Nova Files` folder. CI and `mise run ci` don't set it, so these
tests can break without anyone noticing. When a change touches anything those
tests cover, run them with the data:

```sh
NOVA_DATA="/Users/edward/Projects/ev-nova/EV Nova.app/Contents/Resources/Nova Files" cargo nextest run --workspace
```

That folder (inside the gitignored app bundle in the primary checkout) holds
the `*.ndat` files the loader reads. Other `Nova Files` folders on this
machine (the classic resource-fork copy, `windows-data/`) load nothing.

When a change to behaviour breaks a stock test on purpose, update the test in
the same commit. Don't leave the test failing for later.

### Slow or timing-out tests

When wgpu tests (or any tests) run for seconds each or time out, read
`.claude/rules/slow-tests.md` before blaming load or the code. The usual
cause is a bloated `target/debug/deps`, and the fix is `cargo clean`.

Give a `git bisect` (or any second checkout) its own `CARGO_TARGET_DIR`.
Sharing the main worktree's target directory can leave stale build artifacts
behind, which cause false compile errors and test failures until you run
`cargo clean -p` on the affected crates.

## Doc links between crates

`mise run doc` (part of `mise run ci`) runs rustdoc over every crate, private
items included, and fails on a broken intra-doc link. A doc may link only to
items its crate can name: its own and those of the crates below it. To refer
to an item in a crate above, such as a rulebook row naming the session code
that applies it, write its facade path as inline code (`` `nova_sim::Session` ``),
or move the link to the higher item's doc. Links within a crate, or pointing
down, stay links.

## Mutation testing

After finishing a piece of work, run `cargo-mutants` once over the branch
(`mise run mutants -- --in-diff <(git diff main...HEAD)`) to check that the tests
catch broken code. Read `.claude/rules/mutants.md` before starting a run: it
covers flags, running in the background, one session at a time, and what to
do with missed mutants.

## Conventional commits

Format every commit message as a [Conventional Commit](https://www.conventionalcommits.org/en/v1.0.0/):

```
<type>(<optional scope>): <imperative summary>

<optional body>

<optional footers>
```

- Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `build`, `ci`, `perf`, `style`.
- Scope is usually the crate name without the `nova-` prefix: `rsrc`, `data`, `nova`.
- Mark breaking changes with `!` after the type/scope, or with a
  `BREAKING CHANGE:` footer.
- Don't add `Done:` footers, and don't make one a requirement in a plan,
  acceptance criterion or subagent prompt. Landing marks items done. This
  overrides the global rdm instructions that say to add them.

## Working with rdm here

These hold for every session and every dispatched agent.

- **Landing.** Rebase the branch on `main`. If the rebase is clean, run the
  gate and `merge --ff-only`. If it conflicts, abort, merge `main` into the
  branch, resolve, run the gate and `merge --no-ff`. Landing runs the gate and
  the stock tests, not mutants. Mutants belong in the phase's own work.
- **Task bodies.** Keep checks that can only pass after landing (a `main` CI
  run, a release job) out of the acceptance criteria. Put them under
  `## After landing (not an acceptance criterion)`. The code review grades
  every criterion it finds.
- **Plan reviews.** The persisted review is the record, so don't append round
  notes to the item body. When a review suggests splitting a phase, propose it
  in chat with your own view and wait for the user. Don't file a plan-review
  task for it. The user usually keeps a phase whole when the only reason to
  split is size, but accepts a split by distinct source routine.
- **Moving phases.** Never run `rdm roadmap split` on a roadmap with in-flight
  phases: it renumbers stems and breaks plans, reviews and runs. Move a phase
  with `phase create` plus `phase remove`.
- **Lost review anchors.** If a code review's persist step asks to park over a
  lost anchor that is a false positive (the quote joined wrapped comment
  lines, say), triage the comment as whole-document feedback and carry on.
  Park only when the path doesn't exist at HEAD, the quote matches nothing even
  ignoring wraps and comment prefixes, or a blocking finding's anchor is
  really lost.
