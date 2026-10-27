# What GitHub Actions may run in this public repository, and what its
# GITHUB_TOKEN may do (README.md, "Actions and security settings").
#
# Fork-PR contributor approval (`all_external_contributors`) is manual by
# operator choice: integrations/github 6.13.0 has no resource for it. See the
# README section above; never loosen it by hand.

import {
  for_each = var.import_existing ? toset(["ev-nova"]) : toset([])
  to       = github_actions_repository_permissions.ev_nova
  id       = each.value
}

resource "github_actions_repository_permissions" "ev_nova" {
  repository = github_repository.ev_nova.name

  enabled         = true
  allowed_actions = "all"
  # Off until ci.yml pins its actions by SHA (rdm task pin-ci-actions-by-sha).
  sha_pinning_required = false
}

import {
  for_each = var.import_existing ? toset(["ev-nova"]) : toset([])
  to       = github_workflow_repository_permissions.ev_nova
  id       = each.value
}

# Already the strictest values: a read-only token that can't approve PRs.
resource "github_workflow_repository_permissions" "ev_nova" {
  repository = github_repository.ev_nova.name

  default_workflow_permissions     = "read"
  can_approve_pull_request_reviews = false
}
