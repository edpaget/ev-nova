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
- **Rotating the passphrase:** see below.

### Rotating the passphrase

`pbkdf2` stores its salt in the state under the key provider's label
(`jq '.meta | keys' terraform.tfstate` shows `key_provider.pbkdf2.main`; the
salt isn't secret). A `fallback` can read the old state only through a key
provider with that same label, and OpenTofu accepts only static references
in the encryption block, so the committed config can't rotate by itself. A
rotation takes two applies under temporary edits: the first moves the state
from `main` (old key) to a `next` provider (new key), and the second moves it
back to `main` (new key). Each apply plans no changes and rewrites the state
anyway. Follow the steps in this order:

1. **Keep the old passphrase.** In 1Password, duplicate the
   `opentofu-state-encryption` item and title the copy
   `opentofu-state-encryption-previous`. Do this before changing anything:
   once the item's password changes, the old passphrase is gone.
2. **Change the passphrase.** Generate a new password (at least 16
   characters) on `opentofu-state-encryption`.
3. **Run 1: old key to `next`.** Add this line to `op.env`:

   ```sh
   TF_VAR_old_state_passphrase=op://ev-nova-ci/opentofu-state-encryption-previous/password
   ```

   and replace the `encryption` block in `versions.tf` with:

   ```hcl
   encryption {
     key_provider "pbkdf2" "main" {
       passphrase = var.old_state_passphrase
     }
     key_provider "pbkdf2" "next" {
       passphrase = var.state_passphrase
     }

     method "aes_gcm" "main" {
       keys = key_provider.pbkdf2.main
     }
     method "aes_gcm" "next" {
       keys = key_provider.pbkdf2.next
     }

     state {
       method   = method.aes_gcm.next
       enforced = true
       fallback {
         method = method.aes_gcm.main
       }
     }

     plan {
       method   = method.aes_gcm.next
       enforced = true
       fallback {
         method = method.aes_gcm.main
       }
     }
   }
   ```

   Run `mise run infra-apply`. It must report no changes (if it plans any,
   answer anything but `yes`). The state's meta now names
   `key_provider.pbkdf2.next`.
4. **Run 2: `next` back to `main`.** Remove the `op.env` line. In the block
   above, set the `main` provider's passphrase to `var.state_passphrase`,
   point `state` and `plan` at `method.aes_gcm.main`, and point both
   fallbacks at `method.aes_gcm.next`. Run `mise run infra-apply` again (no
   changes). The meta names `key_provider.pbkdf2.main` again.
5. **Restore and check.** `git checkout infra/github/versions.tf
   infra/github/op.env`, then `mise run infra-plan`: it must read the state
   and report `No changes.` Then delete the `opentofu-state-encryption-previous`
   item. `terraform.tfstate.backup` is encrypted under a key from the
   rotation, so keep it or delete it.

If a run fails, the state is still whole under whichever label its meta
names: `main` with the old key before run 1 completes, and `next` with the
new key after it. Resume from the matching step. `mise run infra-check`
fails while the temporary block is in place (its guard expects
`method.aes_gcm.main`), so don't commit the edits.

## Checks

`mise run infra-check` runs `tofu fmt -check`, `tofu init -backend=false`,
`tofu validate`, `tofu test` with mocked providers, and
`check-encryption.sh`. It needs no credentials and is what CI's `infra` job
runs.

`tofu test` can't see the encryption block, so `check-encryption.sh` guards
it by behaviour, in a temporary directory with copies of the `*.tf` files and
the infra-check task's dummy passphrase. It checks that the config refuses a
planted plaintext state and a plaintext plan file ("unencrypted payload"),
and that an override adding an `unencrypted` fallback to the `state` block,
and then to the `plan` block, is refused ("enforced flag is set"). It fails
if the encryption block, its `state` or `plan` block, or either block's
`enforced = true` is removed or turned off. It never reads the real state.

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
