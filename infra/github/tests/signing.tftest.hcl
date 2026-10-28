# The macos-signing environment and its six secrets, against mocked providers.
# Every value below is an obvious placeholder: no real secret is ever in the
# repository. Run by `mise run infra-check`.

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

# A mock provider can't fill onepassword_item's section/field/file blocks,
# so these override the lookup module's outputs (its own lookup is tested in
# onepassword_item_index.tftest.hcl). Each item carries a decoy field and a
# decoy attachment, so a wrong key picks a wrong value.
override_module {
  target = module.developer_id_item
  outputs = {
    fields = {
      "notes-ish" = "decoy"
      password    = "mock-cert-password"
      identity    = "Developer ID Application: Mock (MOCKTEAM)"
    }
    files = {
      "readme.txt"     = "ZGVjb3k="
      "nova-certs.p12" = "bW9jay1wMTI="
    }
  }
}

override_module {
  target = module.api_key_item
  outputs = {
    fields = {
      decoy       = "decoy"
      "key id"    = "MOCKKEYID"
      "issuer id" = "mock-issuer"
    }
    files = {
      "AuthKey_OTHERKEY.p8"  = "ZGVjb3ktcDg="
      "AuthKey_MOCKKEYID.p8" = "bW9jay1wOA=="
    }
  }
}

run "environment_is_main_only" {
  command = plan

  assert {
    condition     = github_repository_environment.macos_signing.environment == "macos-signing"
    error_message = "The environment must be named macos-signing, the only name ci.yml selects."
  }

  assert {
    condition     = github_repository_environment.macos_signing.repository == github_repository.ev_nova.name
    error_message = "The environment must belong to the managed repository."
  }

  assert {
    condition = (
      github_repository_environment.macos_signing.deployment_branch_policy[0].custom_branch_policies == true &&
      github_repository_environment.macos_signing.deployment_branch_policy[0].protected_branches == false
    )
    error_message = "The environment must use a custom branch policy, not protected branches."
  }

  assert {
    condition     = length(github_repository_environment.macos_signing.reviewers) == 0
    error_message = "main runs unattended: the environment must have no required reviewers."
  }

  assert {
    condition     = coalesce(github_repository_environment.macos_signing.wait_timer, 0) == 0
    error_message = "The environment must have no wait timer."
  }

  assert {
    condition = (
      github_repository_environment_deployment_policy.macos_signing_main.branch_pattern == "main" &&
      github_repository_environment_deployment_policy.macos_signing_main.tag_pattern == null
    )
    error_message = "The only deployment policy must allow the main branch, and no tag."
  }

  assert {
    condition = (
      github_repository_environment_deployment_policy.macos_signing_main.environment == github_repository_environment.macos_signing.environment &&
      github_repository_environment_deployment_policy.macos_signing_main.repository == github_repository.ev_nova.name
    )
    error_message = "The deployment policy must apply to the macos-signing environment."
  }
}

run "each_secret_maps_to_its_field" {
  command = plan

  assert {
    condition     = length(github_actions_environment_secret.signing) == 6
    error_message = "There must be exactly six environment secrets."
  }

  assert {
    condition = alltrue([
      for name, s in github_actions_environment_secret.signing :
      s.secret_name == name &&
      s.environment == github_repository_environment.macos_signing.environment &&
      s.repository == github_repository.ev_nova.name
    ])
    error_message = "Every secret must be named by its key and live in the macos-signing environment."
  }

  assert {
    condition     = nonsensitive(github_actions_environment_secret.signing["MACOS_CERT_P12_BASE64"].value == "bW9jay1wMTI=")
    error_message = "MACOS_CERT_P12_BASE64 must be the nova-certs.p12 attachment's base64."
  }

  assert {
    condition     = nonsensitive(github_actions_environment_secret.signing["MACOS_CERT_PASSWORD"].value == "mock-cert-password")
    error_message = "MACOS_CERT_PASSWORD must be the developer ID item's password field."
  }

  assert {
    condition     = nonsensitive(github_actions_environment_secret.signing["MACOS_SIGNING_IDENTITY"].value == "Developer ID Application: Mock (MOCKTEAM)")
    error_message = "MACOS_SIGNING_IDENTITY must be the developer ID item's identity field."
  }

  assert {
    condition     = nonsensitive(github_actions_environment_secret.signing["APPLE_API_KEY_P8_BASE64"].value == "bW9jay1wOA==")
    error_message = "APPLE_API_KEY_P8_BASE64 must be the AuthKey_<key id>.p8 attachment's base64."
  }

  assert {
    condition     = nonsensitive(github_actions_environment_secret.signing["APPLE_API_KEY_ID"].value == "MOCKKEYID")
    error_message = "APPLE_API_KEY_ID must be the API key item's key id field."
  }

  assert {
    condition     = nonsensitive(github_actions_environment_secret.signing["APPLE_API_ISSUER_ID"].value == "mock-issuer")
    error_message = "APPLE_API_ISSUER_ID must be the API key item's issuer id field."
  }
}
