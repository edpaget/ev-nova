# Mocked-provider tests for rulesets.tf, the main branch ruleset. Run by
# `mise run infra-check`.

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

run "ruleset_targets_main" {
  command = plan

  assert {
    condition = (
      github_repository_ruleset.main.repository == github_repository.ev_nova.name &&
      github_repository_ruleset.main.target == "branch" &&
      github_repository_ruleset.main.enforcement == "active"
    )
    error_message = "The ruleset must be an active branch ruleset on the managed repository."
  }

  # The force-push probe is included for phase 4's proof only; the next
  # commit removes it.
  assert {
    condition     = github_repository_ruleset.main.conditions[0].ref_name[0].include == tolist(["refs/heads/main", "refs/heads/ruleset-probe/force-push"])
    error_message = "The ruleset must name refs/heads/main explicitly (not ~DEFAULT_BRANCH)."
  }

  assert {
    condition     = length(github_repository_ruleset.main.conditions[0].ref_name[0].exclude) == 0
    error_message = "The ruleset must exclude nothing."
  }
}

run "ruleset_refuses_force_push_and_deletion" {
  command = plan

  assert {
    condition     = github_repository_ruleset.main.rules[0].non_fast_forward == true
    error_message = "non_fast_forward must be on: no force-push to main."
  }

  assert {
    condition     = github_repository_ruleset.main.rules[0].deletion == true
    error_message = "deletion must be on: main can't be deleted."
  }
}

run "ruleset_leaves_landing_open" {
  command = plan

  assert {
    condition     = coalesce(github_repository_ruleset.main.rules[0].required_linear_history, false) == false
    error_message = "No required_linear_history: a conflicting land merges main in and lands a merge commit."
  }

  assert {
    condition = (
      coalesce(github_repository_ruleset.main.rules[0].update, false) == false &&
      coalesce(github_repository_ruleset.main.rules[0].creation, false) == false &&
      coalesce(github_repository_ruleset.main.rules[0].required_signatures, false) == false
    )
    error_message = "No update, creation or required_signatures rule: each would refuse the direct landing push."
  }

  assert {
    condition = (
      length(github_repository_ruleset.main.rules[0].pull_request) == 0 &&
      length(github_repository_ruleset.main.rules[0].required_status_checks) == 0 &&
      length(github_repository_ruleset.main.rules[0].required_deployments) == 0 &&
      length(github_repository_ruleset.main.rules[0].merge_queue) == 0
    )
    error_message = "No PR, status-check, deployment or merge-queue requirement: landing is a direct push to main."
  }
}

run "ruleset_bypass_is_admin_role_on_prs_only" {
  command = plan

  assert {
    condition = (
      length(github_repository_ruleset.main.bypass_actors) == 1 &&
      github_repository_ruleset.main.bypass_actors[0].actor_type == "RepositoryRole" &&
      github_repository_ruleset.main.bypass_actors[0].actor_id == 5 &&
      github_repository_ruleset.main.bypass_actors[0].bypass_mode == "pull_request"
    )
    error_message = "Only the repository admin role (id 5) may bypass, and only through a PR: direct pushes stay enforced for everyone."
  }
}
