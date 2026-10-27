# modules/onepassword-item-index, run on its own. A mock provider can't fill
# onepassword_item's computed section/field/file blocks (OpenTofu 1.13 only
# overrides blocks written in configuration), so the lookup lives in this
# module and is tested here with full lists. Placeholder values only.

variables {
  state_passphrase = "test-only-not-a-secret"
}

# Shaped like provider 2.2.1's onepassword_item for a Secure Note: custom
# fields and attachments in an unlabelled section, an empty top-level `file`.
# The decoys come first, and in a second section, so a lookup by position or
# by section would pick them.
run "indexes_fields_by_label_and_files_by_name" {
  command = plan

  module {
    source = "./modules/onepassword-item-index"
  }

  variables {
    item = {
      file = [
        { name = "top-level.txt", content_base64 = "dG9w" },
      ]
      section = [
        {
          field = [
            { label = "notes-ish", value = "decoy" },
          ]
          file = [
            { name = "other.p12", content_base64 = "ZGVjb3k=" },
          ]
        },
        {
          field = [
            { label = "password", value = "mock-password" },
            { label = "identity", value = "mock-identity" },
          ]
          file = [
            { name = "nova-certs.p12", content_base64 = "bW9jay1wMTI=" },
          ]
        },
      ]
    }
  }

  assert {
    condition     = nonsensitive(output.fields == { "notes-ish" = "decoy", password = "mock-password", identity = "mock-identity" })
    error_message = "fields must map every field's label to its value, across all sections."
  }

  assert {
    condition     = nonsensitive(output.files == { "top-level.txt" = "dG9w", "other.p12" = "ZGVjb3k=", "nova-certs.p12" = "bW9jay1wMTI=" })
    error_message = "files must map every attachment's name to its base64, top-level and in sections."
  }
}

run "rejects_a_duplicate_field_label" {
  command = plan

  module {
    source = "./modules/onepassword-item-index"
  }

  variables {
    item = {
      file = []
      section = [
        { field = [{ label = "password", value = "a" }], file = [] },
        { field = [{ label = "password", value = "b" }], file = [] },
      ]
    }
  }

  expect_failures = [var.item]
}

run "rejects_a_duplicate_attachment_name" {
  command = plan

  module {
    source = "./modules/onepassword-item-index"
  }

  variables {
    item = {
      file = [{ name = "nova-certs.p12", content_base64 = "YQ==" }]
      section = [
        { field = [], file = [{ name = "nova-certs.p12", content_base64 = "Yg==" }] },
      ]
    }
  }

  expect_failures = [var.item]
}
