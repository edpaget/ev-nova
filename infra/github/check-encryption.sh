#!/bin/sh
# Guard for infra/github's state and plan encryption, run by `mise run
# infra-check` (and so by CI's `infra` job). `tofu test` can't see the
# encryption block, so this probes the behaviour directly, against copies of
# the *.tf files in a temporary directory. It needs no credentials, no
# network and no `tofu init`, and it never touches the real state.
#
# 1. The unmodified config refuses a planted plaintext state and a plaintext
#    plan file ("unencrypted payload"). This fails if the encryption block,
#    or its `state` or `plan` block, is removed.
# 2. The same config, with an override file that adds an `unencrypted`
#    fallback to one block, refuses to load ("enforced flag is set"); once
#    for `state` and once for `plan`. This fails if either block's
#    `enforced = true` is dropped or set to false, since the fallback would
#    then read the plaintext file.
#
# Each probe matches the expected refusal, not just a non-zero exit, so a
# broken config fails the guard too. Expects TF_VAR_state_passphrase to hold
# a dummy value (the infra-check task sets one).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
unset TF_ENCRYPTION

# A plaintext v4 state and a plaintext plan file, from a config with no
# encryption and only the built-in terraform_data (no provider download).
mkdir "$work/plain"
cat >"$work/plain/main.tf" <<'EOF'
resource "terraform_data" "probe" {
  input = "plaintext-probe"
}
EOF
tofu -chdir="$work/plain" init -input=false -no-color >/dev/null
tofu -chdir="$work/plain" plan -input=false -no-color -out=plain.tfplan >/dev/null
cat >"$work/plain/terraform.tfstate" <<'EOF'
{"version":4,"terraform_version":"1.13.2","serial":1,"lineage":"00000000-0000-0000-0000-000000000000","outputs":{},"resources":[],"check_results":null}
EOF

# expect_refusal <label> <expected text> <tofu args...>
expect_refusal() {
  label=$1
  expected=$2
  shift 2
  if out=$(tofu "$@" -no-color 2>&1); then
    printf 'FAIL: %s: tofu accepted plaintext\n%s\n' "$label" "$out" >&2
    exit 1
  fi
  if ! printf '%s' "$out" | tr -s ' \n' '  ' | grep -q "$expected"; then
    printf 'FAIL: %s: expected "%s"\n%s\n' "$label" "$expected" "$out" >&2
    exit 1
  fi
  printf 'ok: %s: %s\n' "$label" "$expected"
}

probe() {
  dir="$work/$1"
  mkdir "$dir"
  cp "$here"/*.tf "$dir/"
  cp "$work/plain/terraform.tfstate" "$dir/"
}

probe as-is
expect_refusal "plaintext state" "unencrypted payload" \
  -chdir="$work/as-is" state list
expect_refusal "plaintext plan" "unencrypted payload" \
  -chdir="$work/as-is" show "$work/plain/plain.tfplan"

# fallback_probe <block>: a copy whose override adds an unencrypted
# fallback to that one block, so each block's `enforced` is checked alone.
fallback_probe() {
  probe "$1-fallback"
  cat >"$work/$1-fallback/probe_override.tf" <<EOF
terraform {
  encryption {
    method "unencrypted" "probe" {}

    $1 {
      method = method.aes_gcm.main
      fallback {
        method = method.unencrypted.probe
      }
    }
  }
}
EOF
}

fallback_probe state
expect_refusal "unencrypted state fallback" "enforced flag is set" \
  -chdir="$work/state-fallback" state list

fallback_probe plan
expect_refusal "unencrypted plan fallback" "enforced flag is set" \
  -chdir="$work/plan-fallback" show "$work/plain/plain.tfplan"
