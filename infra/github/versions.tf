terraform {
  required_version = "~> 1.13.0"

  required_providers {
    github = {
      source  = "integrations/github"
      version = "= 6.13.0"
    }
    # 2.x, not 3.x: 3.x reaches the desktop app only through the SDK's
    # "Integrate with other apps" channel, which isn't enabled here, and
    # 3.0.0 dropped the `op` CLI path that 2.x uses (README.md).
    onepassword = {
      source  = "1Password/onepassword"
      version = "= 2.2.1"
    }
  }

  # State and plan files are encrypted with a passphrase from 1Password
  # (`op.env`), which reaches OpenTofu only as TF_VAR_state_passphrase in the
  # infra-plan/infra-apply tasks' `op run` child process. There is no
  # `fallback` block, so a plaintext state is never read or migrated, and
  # `enforced` refuses to write one. check-encryption.sh guards both, and
  # README.md covers rotating the passphrase.
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
