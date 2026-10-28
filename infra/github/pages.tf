# Enabled on purpose, not today's value: Pages is off until the first apply
# creates this. ci.yml's `pages` job deploys the site from an Actions
# artifact, so no branch holds built files and there is no `source` block.
# A resource of its own because provider 6.13.0 deprecates the `pages` block
# on github_repository in favour of it. No `cname`: the site stays on
# github.io.
resource "github_repository_pages" "ev_nova" {
  repository = github_repository.ev_nova.name
  build_type = "workflow"
}
