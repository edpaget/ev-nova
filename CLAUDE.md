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

## Verification runs through cargo

Write all checks and verifiers as Rust tests (unit tests, integration tests
under `tests/`, or doctests) so `cargo` runs them.

- Run tests with `mise run test` (nextest plus doctests).
- Run the full gate with `mise run ci` (fmt, clippy with `-D warnings`, tests).
  The pre-commit hook runs the same gate.
- Keep `mise` tasks as one-line `cargo` invocations. When a check needs logic,
  write it in Rust.

### Stock-data tests

Tests over the original game's data (`stock_*.rs` under each crate's `tests/`)
skip, passing, unless `NOVA_DATA` points at a `Nova Files` folder. CI and
`mise run ci` don't set it, so these tests can break without anyone noticing.
When a change touches anything those tests cover, run them with the data:

```sh
NOVA_DATA="/path/to/Nova Files" cargo nextest run --workspace
```

When a change to behaviour breaks a stock test on purpose, update the test in
the same commit. Don't leave the test failing for later.

### Slow or timing-out tests: check the target directory first

When the `nova-render::wgpu_offscreen` tests (or any other wgpu test) run for
seconds each or hit nextest's timeout, check how many files are in
`target/debug/deps` before you blame machine load or the code:

```sh
ls target/debug/deps | wc -l
```

A healthy count is a few thousand, and then the 23 offscreen tests take about
a second in parallel. Repeated `cargo-mutants` and stock runs can grow it to
hundreds of thousands. wgpu's Metal adapter setup reads the test binary's
directory, and nextest starts a new process for each test, so every test pays
that cost again. Run `cargo clean` in that worktree to fix it. Don't run the
tests serially or raise the timeouts to work around it. Long autopilot or
mutants sessions should clean the target directory now and then.

Give a `git bisect` (or any second checkout) its own `CARGO_TARGET_DIR`.
Sharing the main worktree's target directory can leave stale build artifacts
behind, which cause false compile errors and test failures until you run
`cargo clean -p` on the affected crates.

## Mutation testing

Use `cargo-mutants` to check that tests actually catch broken code. Let the
tool generate and run the mutants rather than editing source by hand.

- After finishing a piece of work, run mutants on the files you changed:
  `mise run mutants -- --file <path>` (repeat `--file` for each file), or
  `mise run mutants -- --in-diff <(git diff main)` for everything on the branch.
- For each `MISSED` mutant, write a test that fails against that mutant, then
  rerun to confirm it is `caught`.
- For a mutant that genuinely can't change observable behaviour, add a pattern
  for it to `exclude_re` in `.cargo/mutants.toml` with a comment explaining why.
- Results are written to `mutants.out/` (gitignored). `missed.txt` lists the
  survivors.

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
- Put rdm footers such as `Done: <roadmap-slug>/<phase-stem>` or
  `Done: task/<slug>` in the footer section.
