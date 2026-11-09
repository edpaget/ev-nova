# Who may do what on the Artifact Signing account, against mocked providers.
# Every value below is a placeholder. Run by `mise run infra-check`.

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

run "signer_role_is_scoped_to_the_profile" {
  command = plan

  variables {
    identity_validation_id = "mock-validation-id"
  }

  assert {
    condition     = azurerm_role_assignment.profile_signer[0].scope == azapi_resource.certificate_profile[0].id
    error_message = "The signer role must be scoped to the certificate profile."
  }

  assert {
    condition     = azurerm_role_assignment.profile_signer[0].scope != azurerm_trusted_signing_account.signing.id
    error_message = "The signer role must not be scoped to the whole account."
  }

  assert {
    condition     = azurerm_role_assignment.profile_signer[0].role_definition_name == "Artifact Signing Certificate Profile Signer"
    error_message = "The service principal must get the Certificate Profile Signer role, nothing broader."
  }

  assert {
    condition = (
      azurerm_role_assignment.profile_signer[0].principal_id == azuread_service_principal.signing.object_id &&
      azurerm_role_assignment.profile_signer[0].principal_type == "ServicePrincipal"
    )
    error_message = "The signer role must go to the signing service principal."
  }
}

run "operator_is_identity_verifier_on_the_account" {
  command = plan

  assert {
    condition     = azurerm_role_assignment.identity_verifier.scope == azurerm_trusted_signing_account.signing.id
    error_message = "The Identity Verifier role must be scoped to the account."
  }

  assert {
    condition     = azurerm_role_assignment.identity_verifier.role_definition_name == "Artifact Signing Identity Verifier"
    error_message = "The operator must get the Identity Verifier role."
  }

  assert {
    condition = (
      azurerm_role_assignment.identity_verifier.principal_id == "00000000-0000-0000-0000-00000000000c" &&
      azurerm_role_assignment.identity_verifier.principal_type == "User"
    )
    error_message = "The Identity Verifier role must go to the operator's own user."
  }
}

run "account_is_basic_and_its_provider_is_registered" {
  command = plan

  assert {
    condition     = azurerm_trusted_signing_account.signing.sku_name == "Basic"
    error_message = "The account must use the Basic SKU."
  }

  assert {
    condition     = azurerm_resource_provider_registration.codesigning.name == "Microsoft.CodeSigning"
    error_message = "The config must register Microsoft.CodeSigning, which the account needs."
  }

  assert {
    condition = (
      azurerm_trusted_signing_account.signing.resource_group_name == azurerm_resource_group.signing.name &&
      azurerm_trusted_signing_account.signing.location == var.location
    )
    error_message = "The account must live in the managed resource group and the chosen region."
  }
}
