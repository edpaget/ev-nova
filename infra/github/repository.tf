# Imported, not created: a lost state file is rebuilt by a plain apply.
# `tofu test` can't plan an import even against a mock provider (OpenTofu
# 1.13 panics), so the tests turn it off with var.import_existing.
import {
  for_each = var.import_existing ? toset(["ev-nova"]) : toset([])
  to       = github_repository.ev_nova
  id       = each.value
}

# Every value below is the live repository's value as of 2026-10-27
# (`gh api repos/edpaget/ev-nova`), so the import plans no change. Changing
# one here changes the live repository on the next apply.
resource "github_repository" "ev_nova" {
  name       = "ev-nova"
  visibility = "public"

  has_issues      = true
  has_projects    = true
  has_wiki        = true
  has_discussions = false
  is_template     = false
  archived        = false

  allow_merge_commit          = true
  allow_squash_merge          = true
  allow_rebase_merge          = true
  allow_auto_merge            = false
  allow_update_branch         = false
  delete_branch_on_merge      = false
  web_commit_signoff_required = false

  merge_commit_title          = "MERGE_MESSAGE"
  merge_commit_message        = "PR_TITLE"
  squash_merge_commit_title   = "COMMIT_OR_PR_TITLE"
  squash_merge_commit_message = "COMMIT_MESSAGES"

  lifecycle {
    prevent_destroy = true
    # Cosmetic settings are out of scope (see README.md).
    ignore_changes = [description, homepage_url, topics]
  }
}
