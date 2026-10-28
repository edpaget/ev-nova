# The update-signing environment: the Ed25519 seed nova-appcast signs the
# Sparkle update feeds with, readable only by jobs that deploy to it, and
# only `main` may deploy (README.md, "The update-signing environment").
# ci.yml's publish job, the one job that signs, selects it once the
# UPDATE_SIGNING_ENVIRONMENT variable below exists.
#
# The seed comes from the ev-nova-ci vault. Nothing here outputs it or calls
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

# ci.yml's publish job names the environment only when this variable holds
# its name. Naming an environment that doesn't exist creates it with no
# protection rules, so the variable is created last, once the environment,
# its main-only policy and its secret are all in place. Until the apply, the
# job names no environment and skips the feeds.
resource "github_actions_variable" "update_signing_environment" {
  repository    = github_repository.ev_nova.name
  variable_name = "UPDATE_SIGNING_ENVIRONMENT"
  value         = github_repository_environment.update_signing.environment

  depends_on = [
    github_repository_environment_deployment_policy.update_signing_main,
    github_actions_environment_secret.update_signing_key,
  ]
}
