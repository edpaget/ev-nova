# The dedicated vault this config reads. onepassword_item takes a vault UUID,
# so the vault is looked up by name. This phase reads no item through the
# provider; it only proves the provider's auth.
data "onepassword_vault" "ci" {
  name = "ev-nova-ci"
}

locals {
  ci_vault_uuid = data.onepassword_vault.ci.uuid
}
