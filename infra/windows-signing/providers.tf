# azurerm, azuread and azapi authenticate from the operator's `az login`
# (their default, use_cli = true). azurerm takes the subscription from the
# CLI's default subscription (`az account set --subscription <id>`), so no
# tenant or subscription ID is committed.
provider "azurerm" {
  features {}

  # azurerm 5.x's default, written out: this config registers the one
  # resource provider it needs (Microsoft.CodeSigning, azure.tf) itself.
  resource_provider_registrations = "none"
}

provider "azuread" {}

provider "azapi" {}

# The token comes from GITHUB_TOKEN, which the infra-windows-signing-plan and
# -apply tasks set from `gh auth token`. No PAT is stored anywhere.
provider "github" {
  owner = "edpaget"
}
