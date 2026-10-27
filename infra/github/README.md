# infra/github

OpenTofu configuration for the `edpaget/ev-nova` GitHub repository's
security-relevant settings. Cosmetic settings (description, homepage, topics)
are out of scope and ignored. Apply runs only on the operator's machine, never
in CI.

## One-time setup

1. `mise install` (pins OpenTofu in `mise.toml`).
2. `gh auth login` as a repository admin. The `infra-*` tasks pass
   `gh auth token` to the GitHub provider as `GITHUB_TOKEN`; no PAT is stored.
3. The 1Password desktop app, with the **CLI integration** enabled
   (Settings > Developer > "Integrate with 1Password CLI"), and the `op` CLI on
   `PATH` (or `OP_CLI_PATH` set).

   The config pins `1Password/onepassword` **2.2.1**, which reads through the
   `op` CLI. Provider 3.x was tried first: it talks to the desktop app only
   through the SDK's "Integrate with other apps" channel, which failed here
   with "desktop app connection channel is closed", and 3.0.0 removed the CLI
   path. To move to 3.x, enable "Integrate with other apps", bump the pin,
   relock (below) and set `OP_ACCOUNT` to the account name shown at the top
   of the app's sidebar.
4. Set `OP_ACCOUNT` to the 1Password account's sign-in address or account ID
   (`op account list --format json` shows `account_uuid`), in your shell or in
   a gitignored `mise.local.toml`:

   ```toml
   [env]
   OP_ACCOUNT = "<sign-in address or account ID>"
   ```

   It is a plain value, so setting it never runs `op`.
5. The `ev-nova-ci` vault, holding only what this config reads:

   | Item (title) | Category | Fields (label: type) | File attachment | Feeds |
   |---|---|---|---|---|
   | `macos-developer-id-application` | Secure Note | `password`: concealed, `identity`: text (`Developer ID Application: <name> (<TEAMID>)`) | `nova-certs.p12` | `MACOS_CERT_P12_BASE64`, `MACOS_CERT_PASSWORD`, `MACOS_SIGNING_IDENTITY` |
   | `app-store-connect-api-key` | Secure Note | `key id`: text, `issuer id`: text | `AuthKey_<KEYID>.p8` | `APPLE_API_KEY_P8_BASE64`, `APPLE_API_KEY_ID`, `APPLE_API_ISSUER_ID` |
   | `opentofu-state-encryption` | Password | `password`: concealed, random, at least 16 characters (`pbkdf2`'s minimum) | none | the state passphrase, through `op.env` |

   Create the vault in the app (New Vault, `ev-nova-ci`), then each item with
   exactly the title, field labels and attachment name above: the config
   looks the vault up by name, items by title, fields by label and
   attachments by name. This phase reads only the vault (by name) and
   `opentofu-state-encryption`; the signing items are read by later phases.

## The plan/apply loop

```sh
mise run infra-plan    # read the diff
mise run infra-apply   # interactive: check the summary, then type `yes`
```

Each run runs `tofu init` and then `plan`/`apply` inside one
`op run --env-file=infra/github/op.env`, which sets `TF_VAR_state_passphrase`
from `op://ev-nova-ci/opentofu-state-encryption/password` for that child
process only. Expect 1Password approvals for `op run` and for the provider's
own `op` session. Nothing else in the repository calls `op`: `mise run ci`,
`mise run test` and the hk hooks never touch 1Password or this directory.

Never run `tofu destroy`. `prevent_destroy` is set on the repository, and
apply never runs with `-auto-approve` against a plan that changes anything.

## State

The state is local (`terraform.tfstate`, gitignored) and encrypted with
OpenTofu's native encryption: a `pbkdf2` key from the passphrase and the
`aes_gcm` method, for both state and plan files, with `enforced = true`.
There is no `fallback` block, so an unencrypted state is refused, not read.

- **Lost state:** run `mise run infra-apply`. The `import` blocks adopt the
  existing resources again, with no change to GitHub.
- **Lost passphrase:** delete `terraform.tfstate` and re-apply, as above.
- **Rotating the passphrase:** add a temporary `fallback` block holding the
  old key provider, change the item's password, apply once (which rewrites the
  state under the new key), then remove the `fallback` block.

## Checks

`mise run infra-check` runs `tofu fmt -check`, `tofu init -backend=false`,
`tofu validate` and `tofu test` with mocked providers. It needs no
credentials and is what CI's `infra` job runs.

It is the repository's one check outside cargo, and its scope is narrow on
purpose: only the `infra-check` task and the `infra` CI job, never part of
`mise run ci` or the hk hooks. cargo can't test HCL, and a Rust test wrapping
`tofu` would need the OpenTofu binary and the network for provider
downloads, which would break the hermetic-test rule.

`tofu test` can't plan an `import` block, even against a mock provider
(OpenTofu 1.13 panics with "Importing is not supported in testing context"),
so the tests set `import_existing = false`, which empties the import block's
`for_each`. Nothing else sets that variable.

## Adding a resource

Add an `import` block (with the same `for_each = var.import_existing ? ...`
guard) and declare the resource with today's live values, so the first plan
reads `N to import, 0 to add, 0 to change, 0 to destroy`. Only then change
values on purpose.

After changing a provider pin, regenerate and commit the lock file:

```sh
tofu -chdir=infra/github providers lock -platform=darwin_arm64 -platform=linux_amd64
```
