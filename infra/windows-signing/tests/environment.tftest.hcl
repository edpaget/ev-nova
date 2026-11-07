# The windows-signing environment and its six variables, against mocked
# providers. Every value below is a placeholder. Run by `mise run
# infra-check`.

variables {
  state_passphrase = "test-only-not-a-secret"
}

# The providers validate IDs and UUIDs even under a mock, so the mocked
# computed values are well-formed placeholders: subscription ...0b, tenant
# ...0a, operator ...0c, application ...0d (client ID ...0e), service
# principal ...0f.
#
# Every run is `command = plan`: the mocks fill computed values (IDs, client
# IDs) at plan already, and an apply would fail `tofu test`'s clean-up on the
# account's prevent_destroy.
mock_provider "azurerm" {
  mock_data "azurerm_client_config" {
    defaults = {
      tenant_id       = "00000000-0000-0000-0000-00000000000a"
      subscription_id = "00000000-0000-0000-0000-00000000000b"
    }
  }

  mock_resource "azurerm_resource_group" {
    defaults = {
      id = "/subscriptions/00000000-0000-0000-0000-00000000000b/resourceGroups/ev-nova-signing"
    }
  }

  mock_resource "azurerm_trusted_signing_account" {
    defaults = {
      id          = "/subscriptions/00000000-0000-0000-0000-00000000000b/resourceGroups/ev-nova-signing/providers/Microsoft.CodeSigning/codeSigningAccounts/ev-nova-signing"
      account_uri = "https://eus.codesigning.azure.net/"
    }
  }
}

mock_provider "azuread" {
  mock_data "azuread_client_config" {
    defaults = {
      object_id = "00000000-0000-0000-0000-00000000000c"
    }
  }

  mock_resource "azuread_application" {
    defaults = {
      id        = "/applications/00000000-0000-0000-0000-00000000000d"
      object_id = "00000000-0000-0000-0000-00000000000d"
      client_id = "00000000-0000-0000-0000-00000000000e"
    }
  }

  mock_resource "azuread_service_principal" {
    defaults = {
      id        = "/servicePrincipals/00000000-0000-0000-0000-00000000000f"
      object_id = "00000000-0000-0000-0000-00000000000f"
    }
  }
}

mock_provider "azapi" {
  mock_resource "azapi_resource" {
    defaults = {
      id = "/subscriptions/00000000-0000-0000-0000-00000000000b/resourceGroups/ev-nova-signing/providers/Microsoft.CodeSigning/codeSigningAccounts/ev-nova-signing/certificateProfiles/ev-nova-public"
    }
  }
}

mock_provider "github" {}

run "environment_is_main_only" {
  command = plan

  assert {
    condition     = github_repository_environment.windows_signing.environment == "windows-signing"
    error_message = "The environment must be named windows-signing, the name the federated subject trusts."
  }

  assert {
    condition = (
      github_repository_environment.windows_signing.deployment_branch_policy[0].custom_branch_policies == true &&
      github_repository_environment.windows_signing.deployment_branch_policy[0].protected_branches == false
    )
    error_message = "The environment must use a custom branch policy, not protected branches."
  }

  assert {
    condition     = length(github_repository_environment.windows_signing.reviewers) == 0
    error_message = "main runs unattended: the environment must have no required reviewers."
  }

  assert {
    condition = (
      github_repository_environment_deployment_policy.windows_signing_main.branch_pattern == "main" &&
      github_repository_environment_deployment_policy.windows_signing_main.tag_pattern == null
    )
    error_message = "The only deployment policy must allow the main branch, and no tag."
  }

  assert {
    condition = (
      github_repository_environment_deployment_policy.windows_signing_main.environment == github_repository_environment.windows_signing.environment &&
      github_repository_environment_deployment_policy.windows_signing_main.repository == github_repository_environment.windows_signing.repository
    )
    error_message = "The deployment policy must apply to the windows-signing environment."
  }
}

run "six_variables_in_the_environment" {
  command = plan

  assert {
    condition = toset(keys(github_actions_environment_variable.signing)) == toset([
      "AZURE_TENANT_ID",
      "AZURE_CLIENT_ID",
      "AZURE_SUBSCRIPTION_ID",
      "ARTIFACT_SIGNING_ENDPOINT",
      "ARTIFACT_SIGNING_ACCOUNT",
      "ARTIFACT_SIGNING_PROFILE",
    ])
    error_message = "There must be exactly the six variables CI's signing step reads."
  }

  assert {
    condition = alltrue([
      for name, v in github_actions_environment_variable.signing :
      v.variable_name == name &&
      v.environment == "windows-signing" &&
      v.repository == "ev-nova"
    ])
    error_message = "Every variable must be named by its key and live in the windows-signing environment."
  }

  assert {
    condition = (
      github_actions_environment_variable.signing["AZURE_TENANT_ID"].value == "00000000-0000-0000-0000-00000000000a" &&
      github_actions_environment_variable.signing["AZURE_SUBSCRIPTION_ID"].value == "00000000-0000-0000-0000-00000000000b"
    )
    error_message = "The tenant and subscription variables must come from the caller's az login."
  }

  assert {
    condition     = github_actions_environment_variable.signing["AZURE_CLIENT_ID"].value == azuread_application.signing.client_id
    error_message = "AZURE_CLIENT_ID must be the signing application's client ID."
  }

  assert {
    condition     = github_actions_environment_variable.signing["ARTIFACT_SIGNING_ENDPOINT"].value == azurerm_trusted_signing_account.signing.account_uri
    error_message = "ARTIFACT_SIGNING_ENDPOINT must be the account's regional endpoint."
  }

  assert {
    condition = (
      github_actions_environment_variable.signing["ARTIFACT_SIGNING_ACCOUNT"].value == azurerm_trusted_signing_account.signing.name &&
      github_actions_environment_variable.signing["ARTIFACT_SIGNING_PROFILE"].value == var.profile_name
    )
    error_message = "The account and profile variables must name the managed account and profile."
  }
}
