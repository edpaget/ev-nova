# Dependabot alerts and security updates (README.md, "Actions and security
# settings"). Both are off, and declared so a UI change shows in a plan.
# Secret scanning lives on github_repository.ev_nova (security_and_analysis).

import {
  for_each = var.import_existing ? toset(["ev-nova"]) : toset([])
  to       = github_repository_vulnerability_alerts.ev_nova
  id       = each.value
}

resource "github_repository_vulnerability_alerts" "ev_nova" {
  repository = github_repository.ev_nova.name
  enabled    = false
}

import {
  for_each = var.import_existing ? toset(["ev-nova"]) : toset([])
  to       = github_repository_dependabot_security_updates.ev_nova
  id       = each.value
}

# Off: its bot PRs don't fit this repository's rebase-and-fast-forward landing.
resource "github_repository_dependabot_security_updates" "ev_nova" {
  repository = github_repository.ev_nova.name
  enabled    = false
}
