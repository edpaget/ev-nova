# Paths that become text in tests

A path compared as a `Path` matches across separators. Once it has been
turned into text (a command argument, an `OsString`, a message), it matches
only if both sides were spelled the same way. On Windows they often aren't,
and the test fails only on the Windows CI leg.

## How it breaks

The code under test builds a path from a base it was given as a string,
then joins onto it:

```rust
// cargo metadata's "target_directory": "/work/ev-nova/target"
let staging = spec.target_dir.join("bundle").join("staging");
Invocation::new("codesign").arg(&staging)   // args hold the text
```

On Windows that is `/work/ev-nova/target\bundle\staging`: the base keeps its
`/`, the joins add `\`. A test that rebuilds the expected path from scratch
gets something else:

```rust
let staging = Path::new("/work").join("ev-nova").join("target")
    .join("bundle").join("staging");       // /work\ev-nova\target\bundle\staging
assert_eq!(run.runs(), vec![cmd("codesign", ..., &[&staging])]);
```

The two paths are equal as `Path`s, but the strings in `args` differ, so the
`Vec<Invocation>` comparison fails. On macOS and Linux, `join` writes `/` and
the test passes, so only the Windows CI leg catches it.

This broke `nova-bundle` on Windows twice: once in
`bundle::tests::runs_the_whole_sequence_in_order`, then in 13 signing and
notary tests that shared a `paths()` helper.

## The rule

When a test's expected value holds a path as text, build it from the same
base the code under test uses, then make the same joins.

- If the base comes from a fixture (fake `cargo metadata` JSON, a config
  string, a fake environment variable), read it back through the same parser
  the code uses (`BundleSpec::from_metadata(&sample_metadata())?.target_dir`)
  instead of retyping it with `Path::new(..).join(..)`.
- If the test passes the base in itself (a `Host { out: Some(dir) }`, a temp
  dir), join onto that same value.
- Never write a `/` or `\` separator into an expected string literal.
- Comparing `Path` with `Path` needs none of this. Only paths that have
  become text do: `Invocation` args, `OsString`s, `display()` output, error
  messages, log lines.
- Fix the test, not the production code, unless the text is wrong for a
  Windows player.

A shared test helper that hands out expected paths (`paths()`, `layout()`)
should derive them from the fixture's base, so every test that uses it is
right on Windows.

## Checking it without a Windows machine

The macOS and Linux runs can't catch this. Before pushing a test that
compares path text, look at where each side's base comes from. If the two
are built differently, they'll differ on Windows.
