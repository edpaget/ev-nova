variable "state_passphrase" {
  description = "The pbkdf2 passphrase that encrypts state and plan files. Set by `op run --env-file=op.env` as TF_VAR_state_passphrase; never stored in a file."
  type        = string
  sensitive   = true
}

variable "old_state_passphrase" {
  description = "The previous state passphrase, read only by the temporary encryption block of a passphrase rotation (README.md, \"Rotating the passphrase\"). Unset otherwise."
  type        = string
  sensitive   = true
  default     = null
}

variable "import_existing" {
  description = "Whether the import blocks run. Left true everywhere except tests/*.tftest.hcl, because `tofu test` can't import."
  type        = bool
  default     = true
  nullable    = false
}
