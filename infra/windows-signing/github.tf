# The windows-signing environment, which only `main` may deploy to, and the
# six variables CI's signing step reads. They are variables, not secrets:
# none of them grants anything without the federated credential. This root
# owns the environment because its values come from the Azure resources;
# infra/github never names it.

locals {
  # The repository, managed by infra/github; named here, not looked up.
  repository = "ev-nova"

  # Literal keys, so the variable names are fixed at plan time.
  signing_variables = {
    AZURE_TENANT_ID           = data.azurerm_client_config.current.tenant_id
    AZURE_CLIENT_ID           = azuread_application.signing.client_id
    AZURE_SUBSCRIPTION_ID     = data.azurerm_client_config.current.subscription_id
    ARTIFACT_SIGNING_ENDPOINT = azurerm_trusted_signing_account.signing.account_uri
    ARTIFACT_SIGNING_ACCOUNT  = azurerm_trusted_signing_account.signing.name
    # Set before the profile exists, so the first apply creates all six. CI's
    # signing step fails clearly until the second apply creates the profile.
    ARTIFACT_SIGNING_PROFILE = var.profile_name
  }
}

resource "github_repository_environment" "windows_signing" {
  environment = "windows-signing"
  repository  = local.repository

  # The API's defaults, set so a re-plan shows no change, as for
  # macos-signing in infra/github. No reviewers and no wait timer: main's
  # nightly runs unattended.
  can_admins_bypass   = true
  prevent_self_review = false

  deployment_branch_policy {
    protected_branches     = false
    custom_branch_policies = true
  }
}

# The whole custom policy: main, and nothing else.
resource "github_repository_environment_deployment_policy" "windows_signing_main" {
  repository     = github_repository_environment.windows_signing.repository
  environment    = github_repository_environment.windows_signing.environment
  branch_pattern = "main"
}

resource "github_actions_environment_variable" "signing" {
  for_each = local.signing_variables

  repository    = github_repository_environment.windows_signing.repository
  environment   = github_repository_environment.windows_signing.environment
  variable_name = each.key
  value         = each.value
}
