# The update-signing environment: the Ed25519 seed nova-appcast signs the
# Sparkle update feeds with, and the GPG secret key nova-flatpak-repo signs
# the Flatpak repo with, readable only by jobs that deploy to it, and only
# `main` may deploy (README.md, "The update-signing environment"). ci.yml's
# publish and flatpak-repo jobs, the two that sign, select it once the
# UPDATE_SIGNING_ENVIRONMENT variable below exists.
#
# Both secrets come from the ev-nova-ci vault. Nothing here outputs it or calls
# nonsensitive(), so plan and apply print "(sensitive value)". It does land
# in state, which versions.tf's enforced encryption covers.

# A Password item: provider 2.2.1 exposes its password at the top level, so
# the Secure Note index module isn't needed.
data "onepassword_item" "update_signing" {
  vault = local.ci_vault_uuid
  title = "update-signing-key"
}

# Created, not imported: it didn't exist before this config. README.md,
# "Lost state", covers rebuilding it.
resource "github_repository_environment" "update_signing" {
  environment = "update-signing"
  repository  = github_repository.ev_nova.name

  # The API's defaults, set so a re-plan shows no change. No reviewers and no
  # wait timer: main's nightly runs unattended.
  can_admins_bypass   = true
  prevent_self_review = false

  deployment_branch_policy {
    protected_branches     = false
    custom_branch_policies = true
  }
}

# The whole custom policy: main, and nothing else.
resource "github_repository_environment_deployment_policy" "update_signing_main" {
  repository     = github_repository.ev_nova.name
  environment    = github_repository_environment.update_signing.environment
  branch_pattern = "main"
}

resource "github_actions_environment_secret" "update_signing_key" {
  repository  = github_repository.ev_nova.name
  environment = github_repository_environment.update_signing.environment
  secret_name = "UPDATE_SIGNING_KEY"
  value       = data.onepassword_item.update_signing.password
}

# A Password item holding the base64, on one line, of the Flatpak repo's
# exported GPG secret key (README.md, "The Flatpak repository key"). Every
# plan fails until the item exists.
data "onepassword_item" "flatpak_signing" {
  vault = local.ci_vault_uuid
  title = "flatpak-signing-key"
}

resource "github_actions_environment_secret" "flatpak_signing_key" {
  repository  = github_repository.ev_nova.name
  environment = github_repository_environment.update_signing.environment
  secret_name = "FLATPAK_SIGNING_KEY"
  value       = data.onepassword_item.flatpak_signing.password
}

# ci.yml's publish and flatpak-repo jobs name the environment only when this
# variable holds its name. Naming an environment that doesn't exist creates
# it with no protection rules, so the variable is created last, once the
# environment, its main-only policy and its secrets are all in place. Until
# the apply, the jobs name no environment, and skip the feeds and the
# Flatpak repo.
resource "github_actions_variable" "update_signing_environment" {
  repository    = github_repository.ev_nova.name
  variable_name = "UPDATE_SIGNING_ENVIRONMENT"
  value         = github_repository_environment.update_signing.environment

  depends_on = [
    github_repository_environment_deployment_policy.update_signing_main,
    github_actions_environment_secret.update_signing_key,
    github_actions_environment_secret.flatpak_signing_key,
  ]
}
