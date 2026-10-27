# Mocked-provider tests: no credentials, no network beyond the provider
# download that `tofu init` already did. Run by `mise run infra-check`.

variables {
  state_passphrase = "test-only-not-a-secret"
  # `tofu test` panics planning an import, even against a mock provider.
  import_existing = false
}

mock_provider "github" {}

mock_provider "onepassword" {
  mock_data "onepassword_vault" {
    defaults = {
      uuid = "mockvaultuuid"
    }
  }
}

# signing.tf looks fields up by label and attachments by name, which a mock
# provider can't satisfy, so the lookup modules' outputs are overridden with
# every label and name. These runs don't check the values.
override_module {
  target = module.developer_id_item
  outputs = {
    fields = { password = "mock", identity = "mock" }
    files  = { "nova-certs.p12" = "bW9jaw==" }
  }
}

override_module {
  target = module.api_key_item
  outputs = {
    fields = { "key id" = "MOCK", "issuer id" = "mock" }
    files  = { "AuthKey_MOCK.p8" = "bW9jaw==" }
  }
}

run "repository_keeps_its_identity_and_visibility" {
  command = plan

  assert {
    condition     = github_repository.ev_nova.name == "ev-nova"
    error_message = "The managed repository must be ev-nova."
  }

  assert {
    condition     = github_repository.ev_nova.visibility == "public"
    error_message = "The repository must stay public."
  }
}

run "repository_declares_the_live_merge_settings" {
  command = plan

  assert {
    condition = (
      github_repository.ev_nova.allow_merge_commit &&
      github_repository.ev_nova.allow_squash_merge &&
      github_repository.ev_nova.allow_rebase_merge
    )
    error_message = "Merge, squash and rebase merges must all stay enabled."
  }

  assert {
    condition = (
      !github_repository.ev_nova.allow_auto_merge &&
      !github_repository.ev_nova.allow_update_branch &&
      !github_repository.ev_nova.delete_branch_on_merge
    )
    error_message = "Auto-merge, update-branch and delete-on-merge must stay off."
  }
}

run "ci_vault_is_looked_up_by_name" {
  command = plan

  assert {
    condition     = data.onepassword_vault.ci.name == "ev-nova-ci"
    error_message = "The config must read the ev-nova-ci vault."
  }

  assert {
    condition     = local.ci_vault_uuid == "mockvaultuuid"
    error_message = "local.ci_vault_uuid must come from the vault data source."
  }
}
