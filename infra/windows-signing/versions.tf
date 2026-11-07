terraform {
  required_version = "~> 1.13.0"

  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "= 5.13.0"
    }
    azuread = {
      source  = "hashicorp/azuread"
      version = "= 3.10.0"
    }
    # Only for the certificate profile, which azurerm has no resource for.
    azapi = {
      source  = "Azure/azapi"
      version = "= 2.13.0"
    }
    # The same pin as infra/github.
    github = {
      source  = "integrations/github"
      version = "= 6.13.0"
    }
  }

  # Copied from infra/github/versions.tf; keep the two in step. State and
  # plan files are encrypted with the passphrase from 1Password (`op.env`),
  # which reaches OpenTofu only as TF_VAR_state_passphrase in the
  # infra-windows-signing-plan/-apply tasks' `op run` child process. There is
  # no `fallback` block, so a plaintext state is never read or migrated, and
  # `enforced` refuses to write one. infra/check-encryption.sh guards both.
  encryption {
    key_provider "pbkdf2" "main" {
      passphrase = var.state_passphrase
    }

    method "aes_gcm" "main" {
      keys = key_provider.pbkdf2.main
    }

    state {
      method   = method.aes_gcm.main
      enforced = true
    }

    plan {
      method   = method.aes_gcm.main
      enforced = true
    }
  }
}
