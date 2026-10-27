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
   attachments by name. Keep each label and attachment name unique within
   its item: a duplicate fails the plan.

   Provider 2.2.1 has no `section_map`/`field_map` (those are 3.x). It puts
   a Secure Note's custom fields and attachments in an unlabelled section,
   as `section[].field[]` (`label`, `value`) and `section[].file[]` (`name`,
   `content_base64`), and leaves the top-level `file` list empty.
   `modules/onepassword-item-index` indexes both across every section, and
   the top-level `file` list too, so a field moved into a named section still
   resolves. The attachments reach GitHub as the provider's `content_base64`
   (standard alphabet, unwrapped), which `nova-bundle` decodes as is.

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

## The macos-signing environment

`signing.tf` manages the `macos-signing` environment and the six secrets
that `nova-bundle` reads to sign with Developer ID and notarize:

| Secret | From |
|---|---|
| `MACOS_CERT_P12_BASE64` | `macos-developer-id-application`, attachment `nova-certs.p12` |
| `MACOS_CERT_PASSWORD` | `macos-developer-id-application`, field `password` |
| `MACOS_SIGNING_IDENTITY` | `macos-developer-id-application`, field `identity` |
| `APPLE_API_KEY_P8_BASE64` | `app-store-connect-api-key`, attachment `AuthKey_<key id>.p8` |
| `APPLE_API_KEY_ID` | `app-store-connect-api-key`, field `key id` |
| `APPLE_API_ISSUER_ID` | `app-store-connect-api-key`, field `issuer id` |

The `.p8` attachment is picked by the `key id` field, so the two must
change together.

The environment's deployment-branch policy is custom and allows `main`
only, with no required reviewers and no wait timer, since `main`'s nightly
runs unattended. `ci.yml`'s `build` job names the environment only on
`main`'s macOS leg:

```yaml
environment: ${{ github.ref == 'refs/heads/main' && matrix.os == 'macos-latest' && 'macos-signing' || '' }}
```

Every other leg and every other branch names no environment, so it still
runs. A job that names an environment its branch may not deploy to never
starts, and naming an environment that doesn't exist creates it with no
rules, so the expression must never name anything else. Environment secrets
override repository secrets of the same name, so on `main` the environment's
copies win even while the old repository-level secrets exist.

Plan and apply print every value as `(sensitive value)`, and no output
exposes one. The values do land in the encrypted state. `tofu show -json`
of a plan prints them in clear, so never run it except straight into a
filter that drops values.

### Rotating a signing secret

1. In the `ev-nova-ci` item, replace the attachment or the field value,
   keeping the attachment name and field label. For a new App Store Connect
   API key, the attachment becomes `AuthKey_<new id>.p8` and the `key id`
   field changes with it (delete the old `.p8` attachment).
2. `mise run infra-plan`: expect in-place updates to the affected
   `github_actions_environment_secret.signing[...]` entries only.
3. `mise run infra-apply`, check the summary matches, and type `yes`.

The next `main` nightly picks the new value up. No GitHub UI is involved.

### Cut-over: retiring the repository-level secrets

Before this environment existed, the six secrets were set by hand as
repository-level secrets, readable from any branch. OpenTofu never owned
those copies, so deleting them is a one-time manual step after landing,
owned by the rdm task `macos-signing-environment-cutover`:

1. Wait for the first `main` nightly that deploys to `macos-signing`, logs
   `nova-bundle: signing with Developer ID`, and whose `Nova.app` passes
   `xcrun stapler validate`.
2. Delete each repository-level copy:

   ```sh
   for name in MACOS_CERT_P12_BASE64 MACOS_CERT_PASSWORD MACOS_SIGNING_IDENTITY \
       APPLE_API_KEY_P8_BASE64 APPLE_API_KEY_ID APPLE_API_ISSUER_ID; do
     gh secret delete "$name" --repo edpaget/ev-nova
   done
   ```

3. Check that `gh secret list --repo edpaget/ev-nova` lists none of them.
   From then on a dispatch from any other branch signs ad-hoc.

## Actions and security settings

`actions.tf`, `security.tf` and `repository.tf`'s `security_and_analysis`
block declare the settings that decide what untrusted code can do in this
public repository. Provider 6.13.0 can't manage every one of them; the rest
are manual by operator choice and must never be loosened by hand.

| Setting | Managed by | Value |
|---|---|---|
| Actions enabled | `github_actions_repository_permissions.enabled` | `true` |
| Allowed actions | `github_actions_repository_permissions.allowed_actions` and `allowed_actions_config` | `selected`: GitHub-owned, no verified creators, `Swatinem/rust-cache@*`, `cargo-bins/cargo-binstall@*`, `jdx/mise-action@*` |
| SHA pinning | `github_actions_repository_permissions.sha_pinning_required` | `false` until `ci.yml` pins by SHA (rdm task `pin-ci-actions-by-sha`) |
| Default `GITHUB_TOKEN` permissions | `github_workflow_repository_permissions.default_workflow_permissions` | `read` |
| Actions may approve PRs | `github_workflow_repository_permissions.can_approve_pull_request_reviews` | `false` |
| Vulnerability (Dependabot) alerts | `github_repository_vulnerability_alerts.enabled` | `false` |
| Dependabot security updates | `github_repository_dependabot_security_updates.enabled` | `false`: its bot PRs don't fit the rebase-and-fast-forward landing |
| Secret scanning | `github_repository.security_and_analysis.secret_scanning.status` | `enabled` |
| Push protection | `github_repository.security_and_analysis.secret_scanning_push_protection.status` | `enabled` |
| Fork-PR contributor approval | manual by operator choice: no resource in 6.13.0 | `all_external_contributors` |
| Secret scanning, non-provider patterns | manual by operator choice: the provider declares the attribute but never sends or reads it | `disabled` |
| Secret scanning, AI detection | manual by operator choice: as above | not reported for this repository |
| Secret scanning, validity checks | manual by operator choice: no repository-level attribute | `disabled` |

Merge methods and `delete_branch_on_merge` are on `github_repository` too
(see `repository.tf`).

**The allowlist rule.** A workflow may use only GitHub-owned actions and the
`patterns_allowed` entries in `actions.tf`. Before a workflow change adds a
new third-party action, add its pattern there and apply it, or the jobs that
use it fail at startup ("... is not allowed to be used in edpaget/ev-nova").
A composite action's own nested `uses:` count too.

## Main branch ruleset

`rulesets.tf` declares one ruleset, `protect-main`
(`github_repository_ruleset.main`), active on `refs/heads/main` (named
explicitly, not `~DEFAULT_BRANCH`, so a default-branch rename can't move it).
It has two rules: `non_fast_forward` (no force-push) and `deletion`.

Every rule that would refuse a direct push is left out on purpose, because
landing (`rdm:land`) pushes straight to `main`:

| Rule left out | Why |
|---|---|
| `required_linear_history` | A conflicting rebase is landed by merging `main` into the branch and `merge --no-ff`, which makes a merge commit |
| `pull_request` (require a PR) | The operator lands without PRs |
| `required_status_checks` | A landing push is the commit's first appearance on GitHub, so it has no green checks yet; the gate runs locally (`mise run ci`) |
| `update`, `creation`, `required_signatures` | `update` refuses every push, `main` already exists, and commits here aren't signed |

Landing pushes a fast-forward either way (`merge --ff-only`, or a `--no-ff`
merge on top of `main`'s tip), so the ruleset doesn't affect it.

**Bypass.** Only the repository admin role (`RepositoryRole`, id 5), in
`pull_request` mode. Direct pushes stay enforced for everyone, the admin
included; with `always` the operator's own accidental force-push would go
through. For an emergency, edit or disable the ruleset in the GitHub UI
(admins always can), then fix the config and re-apply.

**Re-running the probe.** Never test against `main`.

1. Add `refs/heads/ruleset-probe/force-push` to the include list and apply.
2. Create the probe from `main`'s tip:
   `git push origin "$(git rev-parse origin/main):refs/heads/ruleset-probe/force-push"`.
3. From a detached worktree at `origin/main~1` with an empty commit, run
   `git push --force origin HEAD:refs/heads/ruleset-probe/force-push`, and
   `git push origin --delete refs/heads/ruleset-probe/force-push`. Both must be
   refused with `GH013` ("Cannot force-push to this branch", "Cannot delete
   this branch").
4. Remove the probe from the include list and apply.
5. Only then delete the probe:
   `git push origin --delete refs/heads/ruleset-probe/force-push`.

## State

The state is local (`terraform.tfstate`, gitignored) and encrypted with
OpenTofu's native encryption: a `pbkdf2` key from the passphrase and the
`aes_gcm` method, for both state and plan files, with `enforced = true`.
There is no `fallback` block, so an unencrypted state is refused, not read.

- **Lost state:** run `mise run infra-apply`. The `import` blocks adopt the
  repository, its Actions permissions (`github_actions_repository_permissions`),
  its workflow token permissions (`github_workflow_repository_permissions`),
  its vulnerability alerts (`github_repository_vulnerability_alerts`) and its
  Dependabot security updates (`github_repository_dependabot_security_updates`)
  again, with no change to them. The `macos-signing` environment,
  its policy and its secrets have no `import` blocks (they were created by
  this config), so first do one of these:
  - Import the environment and its policy, and let the apply rewrite the
    secrets (GitHub never returns their values):

    ```sh
    gh api repos/edpaget/ev-nova/environments/macos-signing/deployment-branch-policies \
      --jq '.branch_policies[] | {id, name}'
    tofu -chdir=infra/github import github_repository_environment.macos_signing ev-nova:macos-signing
    tofu -chdir=infra/github import github_repository_environment_deployment_policy.macos_signing_main ev-nova:macos-signing:<id>
    ```

    Run both imports inside the same `op run --env-file=infra/github/op.env`
    as the `infra-*` tasks, with `GITHUB_TOKEN=$(gh auth token)`.
  - Or delete the environment
    (`gh api -X DELETE repos/edpaget/ev-nova/environments/macos-signing`)
    and apply, which recreates it. Nothing is lost: every value is in
    1Password.
- **The main ruleset** has no `import` block either. Before applying after a
  lost state, import it inside the same `op run` and `GITHUB_TOKEN` wrapping,
  or the apply creates a second `protect-main`:

  ```sh
  gh api repos/edpaget/ev-nova/rulesets --jq '.[] | {id, name}'
  tofu -chdir=infra/github import github_repository_ruleset.main ev-nova:<id>
  ```
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

For a setting that already exists on GitHub, add an `import` block (with the
same `for_each = var.import_existing ? ...` guard) and declare the resource
with today's live values, so the first plan reads
`N to import, 0 to add, 0 to change, 0 to destroy`. Only then change values
on purpose. A resource that doesn't exist yet (like the `macos-signing`
environment) is created instead, with no `import` block; add it to the
"Lost state" bullet above.

After changing a provider pin, regenerate and commit the lock file:

```sh
tofu -chdir=infra/github providers lock -platform=darwin_arm64 -platform=linux_amd64
```
