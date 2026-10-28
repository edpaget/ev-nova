# Indexes one onepassword_item's fields by label and its attachments by name.
#
# Provider 2.2.1 puts a Secure Note's custom fields and attachments in an
# unlabelled section (section[].field, section[].file), so the lookup spans
# every section, and the top-level `file` list too. A label or name that
# appears twice fails the plan here instead of mapping a wrong value, and the
# caller's index of a missing key fails it too.
#
# It is a module only so that it can be tested: a mock provider can't fill
# the data source's computed blocks (tests/onepassword_item_index.tftest.hcl).

variable "item" {
  description = "A onepassword_item data source (only the attributes below are read)."
  type = object({
    file = list(object({
      name           = string
      content_base64 = string
    }))
    section = list(object({
      field = list(object({
        label = string
        value = string
      }))
      file = list(object({
        name           = string
        content_base64 = string
      }))
    }))
  })

  validation {
    condition = (
      length(flatten([for s in var.item.section : [for f in s.field : f.label]])) ==
      length(distinct(flatten([for s in var.item.section : [for f in s.field : f.label]])))
    )
    error_message = "The item has two fields with the same label."
  }

  validation {
    condition = (
      length(concat(var.item.file[*].name, flatten([for s in var.item.section : s.file[*].name]))) ==
      length(distinct(concat(var.item.file[*].name, flatten([for s in var.item.section : s.file[*].name]))))
    )
    error_message = "The item has two attachments with the same name."
  }
}

locals {
  all_files = concat(var.item.file, flatten([for s in var.item.section : s.file]))
}

output "fields" {
  description = "Field label => value."
  sensitive   = true
  value       = { for f in flatten([for s in var.item.section : s.field]) : f.label => f.value }
}

output "files" {
  description = "Attachment name => content, base64 (standard alphabet, unwrapped)."
  sensitive   = true
  value       = { for f in local.all_files : f.name => f.content_base64 }
}
