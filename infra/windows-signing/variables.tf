variable "state_passphrase" {
  description = "The pbkdf2 passphrase that encrypts state and plan files. Set by `op run --env-file=op.env` as TF_VAR_state_passphrase; never stored in a file."
  type        = string
  sensitive   = true
}

# Not a secret: it names the operator's completed Public Trust identity
# validation, and grants nothing by itself. The default is the individual
# validation that completed in November 2026 (README.md, "The two-apply
# flow"); replace it when the validation is renewed ("Validation renewal").
# Set to null, the certificate profile and its signer role assignment
# aren't created, as on the first apply.
variable "identity_validation_id" {
  description = "The ID of the completed Public Trust identity validation on the Artifact Signing account, or null before validation completes."
  type        = string
  default     = "ba5a4aef-b157-4944-93a6-cd11888acdd6"

  validation {
    condition     = var.identity_validation_id == null || try(length(trimspace(var.identity_validation_id)) > 0, false)
    error_message = "identity_validation_id must be null (no validation yet) or a non-empty validation ID."
  }
}

variable "location" {
  description = "The Azure region of the resource group and the Artifact Signing account. It must offer Microsoft.CodeSigning/codeSigningAccounts."
  type        = string
  default     = "eastus"
}

variable "resource_group_name" {
  description = "The resource group that holds the Artifact Signing account."
  type        = string
  default     = "ev-nova-signing"
}

# Globally unique across Azure. A taken name fails only at apply: change the
# default and plan again (README.md).
variable "account_name" {
  description = "The Artifact Signing account's name: 3-24 letters, digits and hyphens, starting with a letter, globally unique."
  type        = string
  default     = "ev-nova-signing"

  validation {
    condition     = can(regex("^[A-Za-z][A-Za-z0-9-]{2,23}$", var.account_name))
    error_message = "account_name must be 3-24 letters, digits and hyphens, starting with a letter."
  }
}

variable "profile_name" {
  description = "The Public Trust certificate profile's name, which CI signs with (ARTIFACT_SIGNING_PROFILE)."
  type        = string
  default     = "ev-nova-public"
}
