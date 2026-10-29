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
   | `update-signing-key` | Password | `password`: concealed, standard base64 of 32 random bytes (`openssl rand -base64 32`) | none | `UPDATE_SIGNING_KEY` |
   | `flatpak-signing-key` | Password | `password`: concealed, standard base64 on one line of the Flatpak repo's exported GPG secret key (`gpg --export-secret-keys --export-options export-minimal <fpr> \| base64 \| tr -d '\n'`) | none | `FLATPAK_SIGNING_KEY` |

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

Done on 2026-10-27, gated on `main` run
[40558074930](https://github.com/edpaget/ev-nova/actions/runs/40558074930)
(63b17659), which deployed to `macos-signing`, signed with Developer ID, and
notarized and stapled `Nova.app`. The six repository-level secrets are
deleted, so the environment's copies are the only ones and a branch dispatch
signs ad-hoc. The steps above stay as the record; to recover a deleted copy,
re-add it from the 1Password item `ev-nova-ci`, not by following them.

## The update-signing environment

`update_signing.tf` manages the `update-signing` environment, its two
secrets, and a repository variable:

| Name | Kind | From |
|---|---|---|
| `UPDATE_SIGNING_KEY` | environment secret | `update-signing-key`, field `password` |
| `FLATPAK_SIGNING_KEY` | environment secret | `flatpak-signing-key`, field `password` (see "The Flatpak repository key" below) |
| `UPDATE_SIGNING_ENVIRONMENT` | repository Actions variable | the environment's name, `update-signing` |

`UPDATE_SIGNING_KEY` is the Ed25519 seed `nova-appcast` signs the Sparkle
update feeds with (`crates/nova-bundle/src/appcast`): 32 random bytes as
standard base64. Its public key is committed as `update_public_key` in
`crates/nova/Cargo.toml`'s `[package.metadata.bundle]`, where the app
builds read it to embed it.

The environment's deployment-branch policy is custom and allows `main`
only, with no required reviewers and no wait timer, as for
`macos-signing`. `release.yml`'s `publish` and `flatpak-repo` jobs, which
`ci.yml` calls on pushes to `main` and `promote-beta.yml` on a promotion
dispatched on `main`, are the only jobs that enter it:

```yaml
environment: ${{ vars.UPDATE_SIGNING_ENVIRONMENT == 'update-signing' && 'update-signing' || '' }}
```

The variable is created after the environment, its policy and its secrets
(`depends_on`), so the jobs name the environment only once it exists with
its main-only policy. Before that apply the jobs name no environment, which
never creates one, and run with empty secrets.

`publish` signs only when both halves are in place, so the steps below can
happen in any order:

| `update_public_key` | `UPDATE_SIGNING_KEY` | `publish` |
|---|---|---|
| not committed | any | skips the feeds with a warning; the nightly publishes as before |
| committed | empty | skips the feeds with a warning |
| committed | malformed, or another key's seed | fails before the tag moves |
| committed | its seed | signs and uploads both feeds and the build-ID update files |

`nova-appcast` also refuses a build that isn't newer than the one the
current feeds offer. An older build fails the job and leaves the release
as it was. The same build (a re-run of a `publish` whose feeds already went
up) exits 3, which the job reports as a notice and carries on from, leaving
the feeds as they are.

`mise run infra-plan` and `infra-apply` fail until the
`update-signing-key` and `flatpak-signing-key` items both exist, because
their data sources can't find them, even for an operator who only wants
the Sparkle side. Create both items first. `mise run infra-check` mocks
1Password and passes without them.

### Setting up the key

No agent or CI job makes, prints or stores the seed. The operator does
this once, on their own machine:

1. Create the item in the `ev-nova-ci` vault:

   ```sh
   op item create --vault ev-nova-ci --category password --title update-signing-key \
     "password=$(openssl rand -base64 32)"
   ```

   Or create a Password item titled `update-signing-key` in the app and
   paste the output of `openssl rand -base64 32` (44 characters).
2. Print the public key, with the seed passed to that process alone:

   ```sh
   UPDATE_SIGNING_KEY=op://ev-nova-ci/update-signing-key/password \
     op run -- mise run appcast -- public-key
   ```

   Optionally record it in the item as a text field `public key`.
3. Set `update_public_key = "<printed value>"` in `crates/nova/Cargo.toml`'s
   `[package.metadata.bundle]`, in a `chore(nova): ...` commit, and land it.
4. Make the Flatpak repo's key too (steps 1 to 4 of "The Flatpak
   repository key" below), since the plan needs its item.
5. `mise run infra-plan`: expect 5 to add,
   `github_repository_environment.update_signing`,
   `github_repository_environment_deployment_policy.update_signing_main`,
   `github_actions_environment_secret.update_signing_key`,
   `github_actions_environment_secret.flatpak_signing_key` and
   `github_actions_variable.update_signing_environment`. Then
   `mise run infra-apply`.
6. The first `main` nightly after steps 1 to 5 deploys `publish` to
   `update-signing`, prints both feed paths and no "Update feeds not
   published" warning, and serves
   `releases/download/nightly/appcast-macos-nightly.xml` and
   `appcast-windows-nightly.xml`, each naming that run's build ID.

### Rotating the update key

Only after a compromise. Every installed copy embeds the old public key,
so it rejects feeds signed with a new one and stops updating until the
player reinstalls from the download page. To rotate, replace the item's
password, then repeat steps 2, 3 and 5 above: commit the new public key and
apply, which updates `github_actions_environment_secret.update_signing_key`
in place. Until both land, `publish` fails on the mismatch rather than sign
feeds no install accepts.

### The Flatpak repository key

`FLATPAK_SIGNING_KEY` is the GPG secret key `nova-flatpak-repo`
(`crates/nova-bundle/src/linux/flatpak/repo`) signs the Flatpak repo on
Pages with: the commits on each channel's branch and the summary. Its
public key is committed as `packaging/linux/flatpak/repo-key.asc`, which
the `.flatpakref` and the repo-following bundle carry, so every install
trusts it. The key has no passphrase, because pinentry can't run in CI:
the environment's main-only policy is its protection.

`release.yml`'s `flatpak-repo` job runs `stage` with no secret (it only learns
whether the secret is set), then `sign` in a step that holds the key alone,
with no token and no network. It publishes only when the channel, the
committed key and the secret are all in place:

| `repo-key.asc` | `FLATPAK_SIGNING_KEY` | `flatpak-repo` |
|---|---|---|
| not committed | any | skips with one "Flatpak repo not published" warning; the nightly publishes as before, with the plain bundle and no `.flatpakref` link |
| holds a secret key, or no public key | any | fails, and with it `pages`; a committed secret key must be loud |
| committed | empty | skips with the warning |
| committed | another key's | fails, naming both fingerprints |
| committed | its secret key | signs, and `pages` serves `/flatpak/` with the `.flatpakref` linked |

No agent or CI job makes, prints or stores the secret key. The operator
does this once, on their own machine, with `gnupg` from Homebrew:

1. Make the key in a throwaway keyring:

   ```sh
   export GNUPGHOME="$(mktemp -d)"
   gpg --batch --pinentry-mode loopback --passphrase '' \
     --quick-gen-key 'ev-nova Flatpak repository' rsa4096 sign never
   fpr=$(gpg --with-colons --list-secret-keys | awk -F: '/^fpr/ {print $10; exit}')
   ```

2. Store the secret key in 1Password, before any `infra-plan`:

   ```sh
   op item create --vault ev-nova-ci --category password --title flatpak-signing-key \
     "password=$(gpg --export-secret-keys --export-options export-minimal "$fpr" | base64 | tr -d '\n')"
   ```

3. Commit the public key. In a worktree branch:

   ```sh
   mkdir -p packaging/linux/flatpak
   gpg --armor --export-options export-minimal --export "$fpr" > packaging/linux/flatpak/repo-key.asc
   gpg --show-keys packaging/linux/flatpak/repo-key.asc
   ```

   Commit it as `chore(packaging): commit the Flatpak repo's public key`,
   and land it.
4. Destroy the local keyring:
   `gpgconf --kill gpg-agent && rm -rf "$GNUPGHOME"; unset GNUPGHOME`. The
   secret key now lives only in 1Password.
5. `mise run infra-plan` adds `github_actions_environment_secret.flatpak_signing_key`
   (and the update-signing resources, if they aren't applied yet). Then
   `mise run infra-apply`.
6. The first `main` nightly after steps 1 to 5: `flatpak-repo` deploys to
   `update-signing` with no "Flatpak repo not published" warning and a
   "First Flatpak publish" notice;
   `https://edpaget.github.io/ev-nova/flatpak/ev-nova-nightly.flatpakref`
   downloads; the download page's Linux row leads with `flatpakref`; and
   the nightly's `.flatpak` asset installs with an origin remote at
   `https://edpaget.github.io/ev-nova/flatpak/repo/`.

Each `main` run rebuilds the whole `/flatpak/` tree from the live one,
because a Pages deploy replaces the site: `stage` mirrors the live repo,
GPG-verified against the committed key. A `404` for the live repo starts a
fresh, empty repo; any other failure fails the job, so an outage is never
taken for "no repo".

If the key or the secret is removed once the repo is live, a run skips and
its deploy drops `/flatpak/`: installs can't update until it returns, and
the next publish sees a `404` and starts fresh, which drops every other
channel's branch.

Every publish, nightly or beta, writes the `.flatpakref` of each channel
whose branch the repo holds, so a nightly's deploy keeps
`ev-nova-beta.flatpakref` and a promotion's keeps the nightly's.

## Promoting a nightly to beta

A beta is a nightly promoted by hand. `promote-beta.yml` rebuilds the
nightly's commit with the same build ID and `NOVA_CHANNEL=beta` (so the
app names the beta feeds), copies the nightly's Flatpak commit onto the
`beta` branch without a rebuild, and publishes the rolling `beta`
prerelease, its signed beta feeds and the Pages site:

```sh
gh workflow run promote-beta.yml --ref main -f build-id=<YYYYMMDDHHMMSS>
```

The build ID is a nightly's (its release notes and update file names carry
it). The workflow's `check` job refuses, before anything is built or
published, a run off `main`, a malformed ID, a build the `nightly` release
no longer holds (both its `.dmg` and `.msi` update files), or one not newer
than the current beta, each naming the IDs.

Preconditions:

- The update-signing setup above is in place: without it no nightly holds
  update files, and every promotion is refused as "isn't on the nightly
  release". The Flatpak repository key is needed for a beta Flatpak; without
  it the beta publishes macOS and Windows only, as the nightly does.
- The nightly was built after promotion support landed, and is still among
  the newest three the nightly release keeps. An older commit's own code
  refuses `NOVA_CHANNEL=beta`, which fails its build legs before anything is
  published.

The `beta` tag moves only in `publish`, after every check and build leg, so
a refused or failed promotion leaves the beta as it was. The nightly's
release, feeds and Flatpak branch are never written.

Nightlies and promotions on `main` share one concurrency group,
`release-main`, which never cancels a running run: they run one at a time.
A push while a promotion is pending replaces it, and it shows as cancelled:
dispatch it again. To finish a promotion that failed after `check` (a flaky
leg, say), use "Re-run failed jobs", which keeps `check`'s result;
`publish` exits 3 for feeds that already went up and carries on.

**Rotating the Flatpak key.** Only after a compromise. Every
`.flatpakref` install trusts the old key and stops updating, and the next
`stage` refuses the live repo the old key signed, failing the job, until
the repo is deliberately started fresh. That procedure isn't written yet:
rdm task `flatpak-repo-key-rotation`.

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

## Pages

`pages.tf` enables GitHub Pages (`github_repository_pages.ev_nova`) with
`build_type = "workflow"` and no `source` branch: the site is deployed from
an Actions artifact, so no branch holds built files. It is a resource of its
own, not a `pages` block on `github_repository`, because provider 6.13.0
deprecates that block. `ci.yml`'s `pages` job is the only deployer; its
comment is the contract for jobs that add a subtree (an artifact named
`pages-<path>`).

Enabling Pages makes GitHub create the `github-pages` environment. This
config doesn't manage it: GitHub gives it a default branch policy, and the
`pages` job runs only on pushes to `main` anyway.

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
- **The `update-signing` environment**, its policy, its two secrets
  (`UPDATE_SIGNING_KEY` and `FLATPAK_SIGNING_KEY`) and the
  `UPDATE_SIGNING_ENVIRONMENT` variable have no `import` blocks either
  (they were created by this config). Import the environment, its policy
  and the variable, and let the apply rewrite the secrets:

  ```sh
  gh api repos/edpaget/ev-nova/environments/update-signing/deployment-branch-policies \
    --jq '.branch_policies[] | {id, name}'
  tofu -chdir=infra/github import github_repository_environment.update_signing ev-nova:update-signing
  tofu -chdir=infra/github import github_repository_environment_deployment_policy.update_signing_main ev-nova:update-signing:<id>
  tofu -chdir=infra/github import github_actions_variable.update_signing_environment ev-nova:UPDATE_SIGNING_ENVIRONMENT
  ```

  inside the same `op run` and `GITHUB_TOKEN` wrapping. Or delete the
  environment and the variable
  (`gh api -X DELETE repos/edpaget/ev-nova/environments/update-signing`,
  `gh variable delete UPDATE_SIGNING_ENVIRONMENT --repo edpaget/ev-nova`)
  and apply, which recreates them.
- **The main ruleset** has no `import` block either. Before applying after a
  lost state, import it inside the same `op run` and `GITHUB_TOKEN` wrapping,
  or the apply creates a second `protect-main`:

  ```sh
  gh api repos/edpaget/ev-nova/rulesets --jq '.[] | {id, name}'
  tofu -chdir=infra/github import github_repository_ruleset.main ev-nova:<id>
  ```
- **Pages** (`github_repository_pages.ev_nova`) has no `import` block
  either: it was created by this config. After a lost state, import it the
  same way before applying (the import ID format is in the provider's
  `github_repository_pages` docs), or the apply tries to enable Pages again.
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
on purpose. A resource that doesn't exist yet (like the `macos-signing` and
`update-signing` environments) is created instead, with no `import` block; add it to the
"Lost state" bullet above.

After changing a provider pin, regenerate and commit the lock file:

```sh
tofu -chdir=infra/github providers lock -platform=darwin_arm64 -platform=linux_amd64
```
