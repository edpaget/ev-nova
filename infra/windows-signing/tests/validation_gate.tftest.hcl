# The certificate profile waits for the identity validation, against mocked
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

run "no_profile_before_validation" {
  command = plan

  # The committed default is the completed validation, so the first apply's
  # state is set here explicitly.
  variables {
    identity_validation_id = null
  }

  assert {
    condition = (
      length(azapi_resource.certificate_profile) == 0 &&
      length(azurerm_role_assignment.profile_signer) == 0
    )
    error_message = "With no validation ID, neither the profile nor its signer role may be planned."
  }

  assert {
    condition = (
      azurerm_resource_group.signing.name == var.resource_group_name &&
      azurerm_trusted_signing_account.signing.name == var.account_name &&
      length(github_actions_environment_variable.signing) == 6
    )
    error_message = "The first apply must still create the resource group, the account and the six variables."
  }
}

run "profile_once_validated" {
  command = plan

  variables {
    identity_validation_id = "mock-validation-id"
  }

  assert {
    condition = (
      length(azapi_resource.certificate_profile) == 1 &&
      length(azurerm_role_assignment.profile_signer) == 1
    )
    error_message = "With a validation ID, the profile and its signer role must be planned."
  }

  assert {
    condition     = azapi_resource.certificate_profile[0].body.properties.profileType == "PublicTrust"
    error_message = "The profile must be Public Trust."
  }

  assert {
    condition     = azapi_resource.certificate_profile[0].body.properties.identityValidationId == "mock-validation-id"
    error_message = "The profile must use the committed validation ID."
  }

  assert {
    condition = (
      azapi_resource.certificate_profile[0].type == "Microsoft.CodeSigning/codeSigningAccounts/certificateProfiles@2025-10-13" &&
      azapi_resource.certificate_profile[0].name == var.profile_name
    )
    error_message = "The profile must be a certificateProfiles resource at the GA API version, named by profile_name."
  }
}

# This run pins the deployed configuration: with no override, the committed
# default must be the completed validation, so the next apply keeps the live
# profile and CI's signer role instead of destroying them.
run "committed_default_keeps_the_live_profile" {
  command = plan

  assert {
    condition = (
      length(azapi_resource.certificate_profile) == 1 &&
      length(azurerm_role_assignment.profile_signer) == 1
    )
    error_message = "The committed default must plan the live profile and its signer role."
  }

  assert {
    condition = (
      azapi_resource.certificate_profile[0].body.properties.identityValidationId != null &&
      azapi_resource.certificate_profile[0].body.properties.identityValidationId == var.identity_validation_id
    )
    error_message = "The live profile must use the committed default validation ID."
  }
}

# The variable validations refuse a blank validation ID and an account name
# Azure would reject, before any plan.
run "rejects_empty_validation_id" {
  command = plan

  variables {
    identity_validation_id = ""
  }

  expect_failures = [var.identity_validation_id]
}

run "rejects_blank_validation_id" {
  command = plan

  variables {
    identity_validation_id = "  "
  }

  expect_failures = [var.identity_validation_id]
}

run "rejects_account_name_starting_with_a_digit" {
  command = plan

  variables {
    account_name = "1bad"
  }

  expect_failures = [var.account_name]
}

run "rejects_account_name_over_24_characters" {
  command = plan

  variables {
    account_name = "abcdefghijklmnopqrstuvwxy"
  }

  expect_failures = [var.account_name]
}

run "rejects_account_name_under_3_characters" {
  command = plan

  variables {
    account_name = "ab"
  }

  expect_failures = [var.account_name]
}

run "accepts_account_name_of_3_characters" {
  command = plan

  variables {
    account_name = "abc"
  }

  assert {
    condition     = azurerm_trusted_signing_account.signing.name == "abc"
    error_message = "A 3-character account_name must pass validation and name the account."
  }
}

run "accepts_account_name_of_24_characters" {
  command = plan

  variables {
    account_name = "abcdefghijklmnopqrstuvwx"
  }

  assert {
    condition     = azurerm_trusted_signing_account.signing.name == "abcdefghijklmnopqrstuvwx"
    error_message = "A 24-character account_name must pass validation and name the account."
  }
}
