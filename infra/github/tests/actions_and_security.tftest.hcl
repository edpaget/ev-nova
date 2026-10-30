# Mocked-provider tests for actions.tf, security.tf and the repository's
# security_and_analysis block. Run by `mise run infra-check`.

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

# As in repository.tftest.hcl: signing.tf's lookups need every label and
# name, which a mock provider can't supply.
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

run "actions_permissions_are_declared" {
  command = plan

  assert {
    condition     = github_actions_repository_permissions.ev_nova.repository == github_repository.ev_nova.name
    error_message = "The Actions permissions must be on the managed repository."
  }

  assert {
    condition = (
      github_actions_repository_permissions.ev_nova.enabled &&
      github_actions_repository_permissions.ev_nova.sha_pinning_required
    )
    error_message = "Actions must stay enabled, with SHA pinning required: every workflow pins its actions by commit SHA."
  }

  assert {
    condition     = github_actions_repository_permissions.ev_nova.allowed_actions == "selected"
    error_message = "allowed_actions must be selected: an explicit allowlist."
  }

  assert {
    condition = (
      github_actions_repository_permissions.ev_nova.allowed_actions_config[0].github_owned_allowed &&
      !github_actions_repository_permissions.ev_nova.allowed_actions_config[0].verified_allowed
    )
    error_message = "Allow GitHub-owned actions, but not every verified creator."
  }

  assert {
    condition = (
      toset(github_actions_repository_permissions.ev_nova.allowed_actions_config[0].patterns_allowed) ==
      toset(["Swatinem/rust-cache@*", "jdx/mise-action@*"])
    )
    error_message = "patterns_allowed must be exactly the two third-party actions the workflows use."
  }
}

run "workflow_token_is_read_only" {
  command = plan

  assert {
    condition     = github_workflow_repository_permissions.ev_nova.repository == github_repository.ev_nova.name
    error_message = "The workflow permissions must be on the managed repository."
  }

  assert {
    condition     = github_workflow_repository_permissions.ev_nova.default_workflow_permissions == "read"
    error_message = "GITHUB_TOKEN must default to read."
  }

  assert {
    condition     = !github_workflow_repository_permissions.ev_nova.can_approve_pull_request_reviews
    error_message = "Actions must not be able to approve pull requests."
  }
}

run "alerts_and_security_updates_declared" {
  command = plan

  assert {
    condition = (
      github_repository_vulnerability_alerts.ev_nova.repository == github_repository.ev_nova.name &&
      !github_repository_vulnerability_alerts.ev_nova.enabled
    )
    error_message = "Vulnerability alerts must be declared off on the managed repository."
  }

  assert {
    condition = (
      github_repository_dependabot_security_updates.ev_nova.repository == github_repository.ev_nova.name &&
      !github_repository_dependabot_security_updates.ev_nova.enabled
    )
    error_message = "Dependabot security updates must be declared off on the managed repository."
  }
}

run "secret_scanning_declared" {
  command = plan

  assert {
    condition     = github_repository.ev_nova.security_and_analysis[0].secret_scanning[0].status == "enabled"
    error_message = "Secret scanning must be enabled."
  }

  assert {
    condition     = github_repository.ev_nova.security_and_analysis[0].secret_scanning_push_protection[0].status == "enabled"
    error_message = "Push protection must be enabled: it refuses a pushed credential."
  }
}
