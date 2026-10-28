# Protects main's history: no force-push, no deletion. Landing (`rdm:land`)
# pushes straight to main as a fast-forward, so every rule that would refuse
# a direct push is left out on purpose (README.md, "Main branch ruleset"):
#
# - required_linear_history: a conflicting land merges main into the branch
#   and lands a `merge --no-ff` merge commit.
# - pull_request: the operator lands without PRs.
# - required_status_checks: a landing push is the commit's first appearance
#   on GitHub, so it never has green checks yet. The gate runs locally.
# - update, creation, required_signatures: update refuses every push,
#   main already exists, and commits here aren't signed.
#
# No import block: this ruleset was created by this config. After a lost
# state, import it as `ev-nova:<ruleset id>` before applying.
resource "github_repository_ruleset" "main" {
  name        = "protect-main"
  repository  = github_repository.ev_nova.name
  target      = "branch"
  enforcement = "active"

  conditions {
    ref_name {
      # Explicit ref, not ~DEFAULT_BRANCH, so a default-branch rename can't
      # move the protection.
      include = ["refs/heads/main"]
      exclude = []
    }
  }

  rules {
    deletion         = true
    non_fast_forward = true
  }

  # The repository admin role (id 5), only through a PR merge: direct pushes,
  # the admin's included, stay enforced. Emergency recovery is editing or
  # disabling the ruleset in the UI, which admins can always do.
  bypass_actors {
    actor_id    = 5
    actor_type  = "RepositoryRole"
    bypass_mode = "pull_request"
  }
}
