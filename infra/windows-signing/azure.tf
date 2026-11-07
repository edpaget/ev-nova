# The Azure side of Windows signing: the Artifact Signing account, the
# operator's right to submit its identity validation, and, once that
# validation completes, the Public Trust certificate profile and the CI
# service principal's right to sign with it (README.md).

data "azurerm_client_config" "current" {}

# The operator, who runs `az login` as a user (README.md, "One-time
# prerequisites").
data "azuread_client_config" "current" {}

resource "azurerm_resource_provider_registration" "codesigning" {
  name = "Microsoft.CodeSigning"
}

resource "azurerm_resource_group" "signing" {
  name     = var.resource_group_name
  location = var.location
}

# azurerm still calls Artifact Signing by its old name, Trusted Signing.
resource "azurerm_trusted_signing_account" "signing" {
  name                = var.account_name
  resource_group_name = azurerm_resource_group.signing.name
  location            = var.location
  sku_name            = "Basic"

  depends_on = [azurerm_resource_provider_registration.codesigning]

  # It holds the completed identity validation, which can't be recreated
  # without validating again.
  lifecycle {
    prevent_destroy = true
  }
}

# Lets the operator submit the identity validation in the portal.
resource "azurerm_role_assignment" "identity_verifier" {
  scope                = azurerm_trusted_signing_account.signing.id
  role_definition_name = "Artifact Signing Identity Verifier"
  principal_id         = data.azuread_client_config.current.object_id
  principal_type       = "User"
}

# azurerm has no certificate-profile resource, so azapi manages it, at the
# GA API version. It needs the completed validation's ID, so it waits for
# var.identity_validation_id (the second apply).
resource "azapi_resource" "certificate_profile" {
  count = var.identity_validation_id == null ? 0 : 1

  type      = "Microsoft.CodeSigning/codeSigningAccounts/certificateProfiles@2025-10-13"
  name      = var.profile_name
  parent_id = azurerm_trusted_signing_account.signing.id

  body = {
    properties = {
      profileType          = "PublicTrust"
      identityValidationId = var.identity_validation_id
    }
  }
}

# CI's only right: sign with this one profile, not anything else on the
# account. principal_type skips the lookup that fails while a just-created
# service principal is still replicating through Entra.
resource "azurerm_role_assignment" "profile_signer" {
  count = var.identity_validation_id == null ? 0 : 1

  scope                = azapi_resource.certificate_profile[0].id
  role_definition_name = "Artifact Signing Certificate Profile Signer"
  principal_id         = azuread_service_principal.signing.object_id
  principal_type       = "ServicePrincipal"
}
