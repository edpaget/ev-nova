# The macos-signing environment: the six Developer ID / notarization secrets,
# readable only by jobs that deploy to it, and only `main` may deploy
# (README.md, "The macos-signing environment"). ci.yml's build job selects it
# only on main's macOS leg.
#
# Every value comes from the ev-nova-ci vault. Nothing here outputs one or
# calls nonsensitive(), so plan and apply print "(sensitive value)". The
# values do land in state, which versions.tf's enforced encryption covers.

data "onepassword_item" "developer_id" {
  vault = local.ci_vault_uuid
  title = "macos-developer-id-application"
}

data "onepassword_item" "api_key" {
  vault = local.ci_vault_uuid
  title = "app-store-connect-api-key"
}

# Fields by label and attachments by name, across every section.
module "developer_id_item" {
  source = "./modules/onepassword-item-index"
  item   = data.onepassword_item.developer_id
}

module "api_key_item" {
  source = "./modules/onepassword-item-index"
  item   = data.onepassword_item.api_key
}

locals {
  dev_id_fields  = module.developer_id_item.fields
  dev_id_files   = module.developer_id_item.files
  api_key_fields = module.api_key_item.fields
  api_key_files  = module.api_key_item.files

  # The secret names nova-bundle reads (ci.yml's Package (macOS) step). The
  # .p8 is chosen by the key id, so the key and its id can't drift apart.
  signing_secrets = {
    MACOS_CERT_P12_BASE64   = local.dev_id_files["nova-certs.p12"]
    MACOS_CERT_PASSWORD     = local.dev_id_fields["password"]
    MACOS_SIGNING_IDENTITY  = local.dev_id_fields["identity"]
    APPLE_API_KEY_P8_BASE64 = local.api_key_files["AuthKey_${local.api_key_fields["key id"]}.p8"]
    APPLE_API_KEY_ID        = local.api_key_fields["key id"]
    APPLE_API_ISSUER_ID     = local.api_key_fields["issuer id"]
  }
}

# Created, not imported: it didn't exist before this config. README.md,
# "Lost state", covers rebuilding it.
resource "github_repository_environment" "macos_signing" {
  environment = "macos-signing"
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
resource "github_repository_environment_deployment_policy" "macos_signing_main" {
  repository     = github_repository.ev_nova.name
  environment    = github_repository_environment.macos_signing.environment
  branch_pattern = "main"
}

resource "github_actions_environment_secret" "signing" {
  # Literal names: keys() of a map with sensitive values can't drive for_each.
  for_each = toset([
    "MACOS_CERT_P12_BASE64",
    "MACOS_CERT_PASSWORD",
    "MACOS_SIGNING_IDENTITY",
    "APPLE_API_KEY_P8_BASE64",
    "APPLE_API_KEY_ID",
    "APPLE_API_ISSUER_ID",
  ])

  repository  = github_repository.ev_nova.name
  environment = github_repository_environment.macos_signing.environment
  secret_name = each.key
  value       = local.signing_secrets[each.key]
}
