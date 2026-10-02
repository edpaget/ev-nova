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

## Testing with hand-written mocks

- Test core logic through its ports. In each test module, write small mocks or
  fakes that implement the port traits. Mocks return canned data and, when the
  interaction matters, record the calls they receive so the test can assert on them.
- Test each component on its own, with mocks standing in for its collaborators.
  When a component is hard to test this way, add a port at that boundary.
- Test each real adapter on its own against the behaviour that the port's mocks
  assume.
- Cover wiring with a small number of integration tests under `tests/`.

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
