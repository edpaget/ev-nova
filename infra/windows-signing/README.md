# infra/windows-signing

OpenTofu configuration for signing the Windows builds with Azure Artifact
Signing (formerly Trusted Signing), and for the `windows-signing` GitHub
environment that CI signs from. Apply runs only on the operator's machine,
never in CI.

It manages:

- the `Microsoft.CodeSigning` resource-provider registration, the
  `ev-nova-signing` resource group and an Artifact Signing account (Basic
  SKU) in it (`azure.tf`);
- the **Artifact Signing Identity Verifier** role on the account for the
  operator's own user, so they can submit the identity validation;
- once the validation completes, a **Public Trust** certificate profile and
  the **Artifact Signing Certificate Profile Signer** role for CI's service
  principal, scoped to that profile, not the account;
- an Entra application and service principal with one federated identity
  credential (issuer `https://token.actions.githubusercontent.com`, audience
  `api://AzureADTokenExchange`, subject
  `repo:edpaget/ev-nova:environment:windows-signing`) and no client secret
  (`entra.tf`);
- the `windows-signing` environment, which only `main` may deploy to, and its
  six variables: `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`,
  `AZURE_SUBSCRIPTION_ID`, `ARTIFACT_SIGNING_ENDPOINT`,
  `ARTIFACT_SIGNING_ACCOUNT` and `ARTIFACT_SIGNING_PROFILE` (`github.tf`).

None of the six is a secret: CI gets a token only through the federated
credential, which trusts only jobs that deploy to `windows-signing`, and the
environment allows only `main`.

It is a separate root from `infra/github` so that planning the GitHub
settings never needs `az login`. The `windows-signing` environment lives
here, not there, because its values come from the Azure resources.
`infra/github` never names it, and no resource is managed by both roots.

## One-time prerequisites (manual)

1. **Public Trust eligibility.** Microsoft issues Public Trust certificates
   to individuals in the US and Canada, and to organisations in the countries
   it lists. Check before paying for anything.
2. **A Pay-As-You-Go (or other paid) subscription.** Artifact Signing does
   not work with free, trial or sponsored subscriptions
   (<https://learn.microsoft.com/azure/artifact-signing/faq>). If the
   subscription is a free trial, upgrade it **in place** (Azure portal >
   Subscriptions > the subscription > Upgrade) before the first apply, so it
   keeps its ID. Don't create the resources first: an identity validation
   can't move to another subscription, and resources left on a cancelled
   trial are suspended. This config doesn't check the subscription type; a
   trial fails at the account's creation.
3. `az login` as a **user** who is an Owner of the subscription and may
   register Entra applications. The Identity Verifier assignment uses
   `principal_type = "User"`, so logging in as a service principal won't do.
   Then pick the subscription; azurerm reads it from the CLI's default:

   ```sh
   az account set --subscription <subscription id>
   az account show --query '{subscription: id, tenant: tenantId}'
   ```
4. `gh auth login` as a repository admin. The tasks pass `gh auth token` to
   the GitHub provider as `GITHUB_TOKEN`; no PAT is stored.
5. 1Password with the `opentofu-state-encryption` item in the `ev-nova-ci`
   vault, the same one `infra/github` uses (its README, "One-time setup").
   Here `op` reads only that passphrase, through `op.env`.
6. The account name is globally unique across Azure. `account_name`
   (`variables.tf`) defaults to `ev-nova-signing`; if it is taken, the apply
   fails at the account. Change the default, commit, and plan again.

## The two-apply flow

The certificate profile needs a completed identity validation, and the
validation needs the account. So the first apply creates everything but the
profile, and the second creates the profile once the validation is done.

```sh
mise run infra-windows-signing-plan    # read the diff
mise run infra-windows-signing-apply   # interactive: check the summary, then type `yes`
```

1. **Apply 1.** With `identity_validation_id` unset (`null`), plan and apply.
   It creates the registration, the resource group, the account, the
   Identity Verifier assignment, the Entra application, service principal
   and federated credential, and the environment with its policy and six
   variables. `ARTIFACT_SIGNING_PROFILE` names the profile before it exists,
   so CI's signing step fails clearly until apply 2. Plan again: it must
   report `No changes.`
2. **Identity validation.** In the portal, open the account, then Identity
   validations > New > Public, and submit. Wait for the status to read
   **Completed** (it can take days), then copy the validation's ID.
3. **Apply 2.** Set the ID as `identity_validation_id`'s default in
   `variables.tf` (it isn't secret) and commit. Plan and apply: it creates
   the certificate profile and the Certificate Profile Signer assignment.
   Plan again: it must report `No changes.` If the follow-up plan shows drift
   on `includeStreetAddress` or `includePostalCode`, set both to `false`
   explicitly in the profile's `body`.

Each task runs `tofu init` and then `plan`/`apply` inside one
`op run --env-file=infra/windows-signing/op.env`, which sets
`TF_VAR_state_passphrase` for that child process only. Nothing in `mise run
ci`, `mise run test` or the hk hooks touches Azure, 1Password or this
directory.

Never run `tofu destroy`. The account carries `prevent_destroy`: it holds
the identity validation, which can't be recreated without validating again.

## Validation renewal

When the identity validation expires, start a new one on the account (as in
step 2 above), wait for it to complete, replace `identity_validation_id`'s
default with the new ID, commit and apply. Read the plan first: azapi may
update the profile in place or replace it. The profile keeps its name either
way, so the six GitHub variables don't change.

## State

The state is local (`terraform.tfstate`, gitignored) and encrypted with
OpenTofu's native encryption, with the same block and passphrase as
`infra/github` (`versions.tf` says so; keep the two in step). There is no
`fallback` block, so an unencrypted state is refused, not read.

- **Lost state:** nothing here has an `import` block (every resource was
  created by this config). Before applying, import each one inside the same
  `op run --env-file=infra/windows-signing/op.env`, with
  `GITHUB_TOKEN=$(gh auth token)`, as `tofu -chdir=infra/windows-signing
  import <address> <id>`:

  | Address | ID format |
  |---|---|
  | `azurerm_resource_provider_registration.codesigning` | `/subscriptions/<sub>/providers/Microsoft.CodeSigning` |
  | `azurerm_resource_group.signing` | `/subscriptions/<sub>/resourceGroups/ev-nova-signing` |
  | `azurerm_trusted_signing_account.signing` | `/subscriptions/<sub>/resourceGroups/ev-nova-signing/providers/Microsoft.CodeSigning/codeSigningAccounts/<account>` |
  | `azapi_resource.certificate_profile[0]` | `<account id>/certificateProfiles/<profile>?api-version=2025-10-13` |
  | `azurerm_role_assignment.identity_verifier`, `azurerm_role_assignment.profile_signer[0]` | `<scope>/providers/Microsoft.Authorization/roleAssignments/<assignment guid>` (`az role assignment list --scope <scope>`) |
  | `azuread_application.signing` | `/applications/<application object id>` |
  | `azuread_service_principal.signing` | `/servicePrincipals/<service principal object id>` |
  | `azuread_application_federated_identity_credential.github_windows_signing` | `/applications/<application object id>/federatedIdentityCredential/<credential id>` |
  | `github_repository_environment.windows_signing` | `ev-nova:windows-signing` |
  | `github_repository_environment_deployment_policy.windows_signing_main` | `ev-nova:windows-signing:<policy id>` (`gh api repos/edpaget/ev-nova/environments/windows-signing/deployment-branch-policies`) |
  | `github_actions_environment_variable.signing["<NAME>"]` | `ev-nova:windows-signing:<NAME>`, once per variable |

  The profile and its signer assignment exist only after apply 2. Then plan:
  it must report `No changes.` (The GitHub environment and its variables can
  instead be deleted and re-created by the apply; the Azure resources can't,
  since the account holds the validation.)
- **Lost passphrase:** import everything as above into a fresh state.
- **Rotating the passphrase:** follow `infra/github/README.md`, "Rotating
  the passphrase", with `infra/windows-signing` in place of `infra/github`
  and the `infra-windows-signing-*` tasks in place of `infra-*`. Both roots
  share the `opentofu-state-encryption` item, so changing its password means
  rotating both roots together, while the `-previous` copy still exists.

## Checks

`mise run infra-check` runs, for this root as for `infra/github`,
`tofu fmt -check`, `tofu init -backend=false`, `tofu validate`, `tofu test`
with mocked providers (`tests/`), and `infra/check-encryption.sh
infra/windows-signing`. It needs no Azure, GitHub or 1Password credentials,
and it is what CI's `infra` job runs. The tests check that the federated
subject names only the `windows-signing` environment, that the signer role
is scoped to the profile, that the profile and its role assignment are
absent while `identity_validation_id` is null, that the environment is
main-only, and that the six variables carry the right values.

After changing a provider pin, regenerate and commit the lock file:

```sh
tofu -chdir=infra/windows-signing providers lock -platform=darwin_arm64 -platform=linux_amd64
```
