# The token comes from GITHUB_TOKEN, which the infra-plan/infra-apply tasks
# set from `gh auth token`. No PAT is stored anywhere.
provider "github" {
  owner = "edpaget"
}

# Provider 2.x goes through the `op` CLI and its desktop-app integration
# (biometric unlock). The account (sign-in address or account ID) comes from
# OP_ACCOUNT, and `op` is found on PATH (or OP_CLI_PATH), so no personal value
# or machine path is committed.
provider "onepassword" {}
