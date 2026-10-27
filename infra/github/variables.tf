variable "state_passphrase" {
  description = "The pbkdf2 passphrase that encrypts state and plan files. Set by `op run --env-file=op.env` as TF_VAR_state_passphrase; never stored in a file."
  type        = string
  sensitive   = true
}

variable "import_existing" {
  description = "Whether the import blocks run. Left true everywhere except tests/*.tftest.hcl, because `tofu test` can't import."
  type        = bool
  default     = true
  nullable    = false
}
