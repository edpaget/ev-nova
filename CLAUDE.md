# CLAUDE.md

## Test-driven development

Write all code test-first:

1. **Red**: write a failing test for the next small piece of behavior. Run it
   and confirm it fails for the reason you expect.
2. **Green**: write the minimum code that makes the test pass.
3. **Refactor**: clean up the test and the implementation while the suite stays green.

- Don't write production code unless a failing test calls for it.
- When fixing a bug, first write a test that reproduces it.
- Keep tests fast and deterministic. Prefer small fixture data checked into the
  repo over reading files from `EV Nova.app` at test time.
- Run tests with `mise run test` (nextest plus doctests). Run the full gate with
  `mise run ci` (fmt, clippy with `-D warnings`, tests). The pre-commit hook
  runs the same gate.

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
