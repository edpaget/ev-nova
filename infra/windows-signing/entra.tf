# The identity CI signs as. It has no client secret: GitHub Actions gets a
# token by OIDC, through the federated credential, and only for jobs that
# deploy to the windows-signing environment.

resource "azuread_application" "signing" {
  display_name = "ev-nova windows-signing (GitHub Actions)"
  owners       = [data.azuread_client_config.current.object_id]
}

resource "azuread_service_principal" "signing" {
  client_id = azuread_application.signing.client_id
  owners    = [data.azuread_client_config.current.object_id]
}

resource "azuread_application_federated_identity_credential" "github_windows_signing" {
  application_id = azuread_application.signing.id
  display_name   = "github-ev-nova-windows-signing"
  description    = "GitHub Actions jobs in edpaget/ev-nova that deploy to the windows-signing environment."
  issuer         = "https://token.actions.githubusercontent.com"
  audiences      = ["api://AzureADTokenExchange"]
  # The repository uses GitHub's immutable OIDC subjects, which carry the
  # owner's and the repository's numeric IDs (edpaget is 108177, ev-nova is
  # 1403578017), so a repository that later takes the name can't match. Check
  # with `gh api repos/edpaget/ev-nova/actions/oidc/customization/sub`.
  subject = "repo:edpaget@108177/ev-nova@1403578017:environment:windows-signing"
}
