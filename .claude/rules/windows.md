# Windows pitfalls

CI runs every test on `windows-latest` as well as macOS and Linux. These are
the mistakes that have broken the Windows leg before.

## File and directory names

Windows can't check out a path that breaks these rules, so `actions/checkout`
fails and no test runs at all.

- Don't name a file or directory `con`, `prn`, `aux`, `nul`, `com1`-`com9` or
  `lpt1`-`lpt9`, in any case, with or without an extension (`aux.rs` is as
  bad as `aux`).
- Don't use the characters `<`, `>`, `:`, `"`, `|`, `?`, `*` or `\` in a path,
  and don't end a name with a dot or a space.
- Don't add two paths that differ only in case.
- Name a Rust module so its file name is allowed: `aux_ships`, not `aux`.

## Paths in test expectations

On Windows, `Path::join` uses `\`, so `Path::new("/data").join("Broken")`
displays as `/data\Broken`.

- When a test compares *text* that holds a path (a warning, an error message,
  a log line), build the expected text with `Path`/`PathBuf`: `join`, then
  `display()`. Never write a literal `/` or `\` separator into the expected
  string.
- Comparing a `Path` with a `Path` (`assert_eq!(x.path, Path::new("/d/one"))`)
  is fine as it is: paths compare by component, so `/` matches `\`.
- Fix the test, not the production output, unless the output is wrong for a
  Windows player.
