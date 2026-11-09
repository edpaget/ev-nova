# The GitHub Actions federated credential, against mocked providers. Every
# value below is a placeholder. Run by `mise run infra-check`.

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

run "federated_credential_trusts_only_the_windows_signing_environment" {
  command = plan

  assert {
    condition     = azuread_application_federated_identity_credential.github_windows_signing.subject == "repo:edpaget/ev-nova:environment:windows-signing"
    error_message = "The federated subject must name only the windows-signing environment, so no other job or branch can get a token."
  }

  assert {
    condition     = azuread_application_federated_identity_credential.github_windows_signing.issuer == "https://token.actions.githubusercontent.com"
    error_message = "The issuer must be GitHub Actions' OIDC issuer."
  }

  assert {
    condition     = azuread_application_federated_identity_credential.github_windows_signing.audiences == tolist(["api://AzureADTokenExchange"])
    error_message = "The only audience must be api://AzureADTokenExchange."
  }

  assert {
    condition     = azuread_application_federated_identity_credential.github_windows_signing.application_id == azuread_application.signing.id
    error_message = "The credential must belong to the signing application."
  }

  assert {
    condition     = azuread_service_principal.signing.client_id == azuread_application.signing.client_id
    error_message = "The service principal must be the signing application's."
  }

  assert {
    condition     = length(azuread_application.signing.password) == 0
    error_message = "The application must have no client secret: CI authenticates by OIDC only."
  }
}
