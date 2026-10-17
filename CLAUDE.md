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

- If the engine's behaviour has an obvious alternative (the Nova Bible
  disagrees, the engine has a quirk, or our plan said otherwise), add a
  `Rulebook` key (`crates/nova-sim/src/rulebook.rs`). The engine's reading is
  the default, and tests cover both readings. Ask the user only when the
  alternative isn't obvious, the scope grows, or the change alters a control
  everyone uses.
- If the engine has a bug that breaks stock missions, crons or storylines,
  make the intended behaviour the default and the bug the opt-in reading.
  Check the stock data before calling something broken.
- Don't replicate integer widths, wraps or overflows (16-bit cargo, 32-bit
  cash). Use wider numbers. Mention the width in the finding, but don't file
  a phase or key for it.
- If a difference is practically unobservable (sub-second timing, say),
  offer "record only" beside "build it". Don't assume either answer: the
  user has chosen each.

### Where new findings go

File every divergence the binary shows, however niche.

- If it serves the roadmap's goal, add it as a new phase of that roadmap.
- On a feature roadmap, a finding that doesn't block playing the stock
  storyline goes to `<slug>-fidelity` instead. Create that roadmap, depending
  on the feature roadmap, the first time it's needed.
- If the roadmap is finished, file a task, not a phase.
- If a nearly finished roadmap blocks others, ask whether new findings
  should go to a follow-up roadmap so it can finish.

## Testing with hand-written mocks

- Test core logic through its ports. In each test module, write small mocks or
  fakes that implement the port traits. Mocks return canned data and, when the
  interaction matters, record the calls they receive so the test can assert on them.
- Test each component on its own, with mocks standing in for its collaborators.
  When a component is hard to test this way, add a port at that boundary.
- Test each real adapter on its own against the behaviour that the port's mocks
  assume.
- Cover wiring with a small number of integration tests under `tests/`.

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

## Reading large files

`crates/nova-view/src/flight/view.rs`, `crates/nova-sim/src/session.rs`,
`crates/nova/src/app/screen.rs` and `crates/nova-sim/src/rulebook.rs` run to
thousands of lines. Agents read them thousands of times a week.

- Find the code first with `grep -n` or LSP symbols, then read only that range
  with the `Read` tool (`offset`/`limit`).
- Read with `Read`, not `sed`, `cat` or `head`: `Edit` refuses a file that
  `Read` hasn't seen, so a `sed` read costs a second read before any edit.

## Verification runs through cargo

Write all checks and verifiers as Rust tests (unit tests, integration tests
under `tests/`, or doctests) so `cargo` runs them.

- Run tests with `mise run test` (nextest plus doctests).
- In the TDD loop, run `cargo nextest run -p <crate>`, the same form the
  pre-commit hook uses, so its build stays warm. Don't switch between `-p` and
  `--workspace` in the loop: each switch rebuilds.
- Run the full gate with `mise run ci` (fmt, clippy with `-D warnings`, tests,
  doctests and the dev-tools legs). Run it once before moving an item to
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

### Stock-data tests

Tests over the original game's data (`stock_*.rs` under each crate's `tests/`)
skip, passing, unless `NOVA_DATA` points at a `Nova Files` folder. CI and
`mise run ci` don't set it, so these tests can break without anyone noticing.
When a change touches anything those tests cover, run them with the data:

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

## Mutation testing

After finishing a piece of work, run `cargo-mutants` once over the branch
(`mise run mutants -- --in-diff <(git diff main)`) to check that the tests
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
