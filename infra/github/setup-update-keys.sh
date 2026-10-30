#!/usr/bin/env bash
# Makes the two keys auto-updates sign with, stores their secret halves in
# 1Password and writes their public halves into this checkout. Run it once,
# on your own machine, signed in to 1Password (`op signin`):
#
#   infra/github/setup-update-keys.sh
#
# 1. The Sparkle update-feed key: a 32-byte Ed25519 seed, stored as the
#    Password item `update-signing-key`. Its public key is written as
#    `update_public_key` in crates/nova/Cargo.toml.
# 2. The Flatpak repository key: an RSA 4096 GPG signing key with no
#    passphrase, made in a throwaway keyring, stored (base64 of the exported
#    secret key) as `flatpak-signing-key`. Its public key is written to
#    packaging/linux/flatpak/repo-key.asc. The keyring is deleted on exit.
#
# Both items go in the `ev-nova-ci` vault. The script refuses to run if
# either item already exists: replacing a live key strands every install
# (see "Rotating the update key" in README.md). It commits and applies
# nothing; it prints what to do next. See README.md, "The update-signing
# environment" and "The Flatpak repository key".
set -euo pipefail

vault=ev-nova-ci
update_item=update-signing-key
flatpak_item=flatpak-signing-key

root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
cargo_toml="$root/crates/nova/Cargo.toml"
repo_key="$root/packaging/linux/flatpak/repo-key.asc"

for tool in op gpg gpgconf openssl mise; do
  command -v "$tool" >/dev/null || { echo "error: $tool is not installed" >&2; exit 1; }
done
op whoami >/dev/null 2>&1 || { echo "error: not signed in to 1Password; run \`op signin\` first" >&2; exit 1; }

for item in "$update_item" "$flatpak_item"; do
  if op item get "$item" --vault "$vault" >/dev/null 2>&1; then
    echo "error: $vault/$item already exists; this script only sets keys up the first time" >&2
    exit 1
  fi
done
grep -q '^# update_public_key = ' "$cargo_toml" || {
  echo "error: $cargo_toml has no commented update_public_key placeholder; is a key already committed?" >&2
  exit 1
}
[ ! -e "$repo_key" ] || { echo "error: $repo_key already exists" >&2; exit 1; }

# 1. The update-feed key.
echo "Creating $vault/$update_item..."
op item create --vault "$vault" --category password --title "$update_item" \
  "password=$(openssl rand -base64 32)" >/dev/null
public_key=$(cd "$root" && UPDATE_SIGNING_KEY="op://$vault/$update_item/password" \
  op run -- mise run -q appcast -- public-key)
[ -n "$public_key" ] || { echo "error: nova-appcast printed no public key" >&2; exit 1; }

# Replace the three-line placeholder comment with the key.
tmp=$(mktemp)
awk -v key="$public_key" '
  /^# update_public_key = / { print "update_public_key = \"" key "\""; skip = 1; next }
  skip && /^# / { next }
  { skip = 0; print }
' "$cargo_toml" > "$tmp"
mv "$tmp" "$cargo_toml"
echo "Wrote update_public_key to ${cargo_toml#"$root"/}"

# 2. The Flatpak repository key, in a keyring that lives only for this run.
GNUPGHOME=$(mktemp -d)
export GNUPGHOME
cleanup() {
  gpgconf --kill gpg-agent 2>/dev/null || true
  rm -rf "$GNUPGHOME"
}
trap cleanup EXIT

echo "Creating the Flatpak repository key..."
gpg --batch --quiet --pinentry-mode loopback --passphrase '' \
  --quick-gen-key 'ev-nova Flatpak repository' rsa4096 sign never
fpr=$(gpg --with-colons --list-secret-keys | awk -F: '/^fpr/ {print $10; exit}')

echo "Creating $vault/$flatpak_item..."
op item create --vault "$vault" --category password --title "$flatpak_item" \
  "password=$(gpg --export-secret-keys --export-options export-minimal "$fpr" | base64 | tr -d '\n')" >/dev/null

mkdir -p "$(dirname "$repo_key")"
gpg --armor --export-options export-minimal --export "$fpr" > "$repo_key"
echo "Wrote ${repo_key#"$root"/} (fingerprint $fpr)"

cat <<EOF

Done. Both secret keys are in 1Password; the keyring is deleted.

Next:
  1. Review the diff: git -C "$root" diff
  2. Commit it (e.g. "chore: commit the update and Flatpak public keys") and land it.
  3. mise run infra-plan    # expect 5 to add
     mise run infra-apply
  4. Check the first nightly on main: no "Update feeds not published" or
     "Flatpak repo not published" warning.
EOF
