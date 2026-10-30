# The update-signing environment, its seed and Flatpak key secrets, and the
# Actions variable that tells ci.yml the environment exists, against mocked
# providers. The passwords below are obvious placeholders: no real key is
# ever in the repository. Run by `mise run infra-check`.

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

override_data {
  target = data.onepassword_item.update_signing
  values = {
    password = "bW9jay11cGRhdGUta2V5"
  }
}

override_data {
  target = data.onepassword_item.flatpak_signing
  values = {
    password = "bW9jay1mbGF0cGFrLWtleQ=="
  }
}

run "environment_is_main_only" {
  command = plan

  assert {
    condition     = github_repository_environment.update_signing.environment == "update-signing"
    error_message = "The environment must be named update-signing, the only name ci.yml selects."
  }

  assert {
    condition     = github_repository_environment.update_signing.repository == github_repository.ev_nova.name
    error_message = "The environment must belong to the managed repository."
  }

  assert {
    condition = (
      github_repository_environment.update_signing.deployment_branch_policy[0].custom_branch_policies == true &&
      github_repository_environment.update_signing.deployment_branch_policy[0].protected_branches == false
    )
    error_message = "The environment must use a custom branch policy, not protected branches."
  }

  assert {
    condition     = length(github_repository_environment.update_signing.reviewers) == 0
    error_message = "main runs unattended: the environment must have no required reviewers."
  }

  assert {
    condition     = coalesce(github_repository_environment.update_signing.wait_timer, 0) == 0
    error_message = "The environment must have no wait timer."
  }

  assert {
    condition = (
      github_repository_environment_deployment_policy.update_signing_main.branch_pattern == "main" &&
      github_repository_environment_deployment_policy.update_signing_main.tag_pattern == null
    )
    error_message = "The only deployment policy must allow the main branch, and no tag."
  }

  assert {
    condition = (
      github_repository_environment_deployment_policy.update_signing_main.environment == github_repository_environment.update_signing.environment &&
      github_repository_environment_deployment_policy.update_signing_main.repository == github_repository.ev_nova.name
    )
    error_message = "The deployment policy must apply to the update-signing environment."
  }
}

run "the_secret_is_the_items_password" {
  command = plan

  assert {
    condition     = data.onepassword_item.update_signing.title == "update-signing-key"
    error_message = "The seed must come from the update-signing-key item."
  }

  assert {
    condition = (
      github_actions_environment_secret.update_signing_key.secret_name == "UPDATE_SIGNING_KEY" &&
      github_actions_environment_secret.update_signing_key.environment == github_repository_environment.update_signing.environment &&
      github_actions_environment_secret.update_signing_key.repository == github_repository.ev_nova.name
    )
    error_message = "UPDATE_SIGNING_KEY, the name nova-appcast reads, must live in the update-signing environment."
  }

  assert {
    condition     = nonsensitive(github_actions_environment_secret.update_signing_key.value == "bW9jay11cGRhdGUta2V5")
    error_message = "UPDATE_SIGNING_KEY must be the update-signing-key item's password."
  }
}

run "the_flatpak_secret_is_the_items_password" {
  command = plan

  assert {
    condition     = data.onepassword_item.flatpak_signing.title == "flatpak-signing-key"
    error_message = "The Flatpak repo's key must come from the flatpak-signing-key item."
  }

  assert {
    condition = (
      github_actions_environment_secret.flatpak_signing_key.secret_name == "FLATPAK_SIGNING_KEY" &&
      github_actions_environment_secret.flatpak_signing_key.environment == github_repository_environment.update_signing.environment &&
      github_actions_environment_secret.flatpak_signing_key.repository == github_repository.ev_nova.name
    )
    error_message = "FLATPAK_SIGNING_KEY, the name nova-flatpak-repo reads, must live in the update-signing environment."
  }

  assert {
    condition     = nonsensitive(github_actions_environment_secret.flatpak_signing_key.value == "bW9jay1mbGF0cGFrLWtleQ==")
    error_message = "FLATPAK_SIGNING_KEY must be the flatpak-signing-key item's password."
  }
}

run "the_variable_names_the_environment_for_ci" {
  command = plan

  assert {
    condition = (
      github_actions_variable.update_signing_environment.variable_name == "UPDATE_SIGNING_ENVIRONMENT" &&
      github_actions_variable.update_signing_environment.repository == github_repository.ev_nova.name
    )
    error_message = "ci.yml reads the repository variable UPDATE_SIGNING_ENVIRONMENT."
  }

  assert {
    condition     = github_actions_variable.update_signing_environment.value == github_repository_environment.update_signing.environment
    error_message = "The variable must hold the managed environment's name, the only value ci.yml's publish job accepts."
  }
}
