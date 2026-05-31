#!/usr/bin/env bash
set -euo pipefail

# Bump ShellQL beta patch version:
#   0.1.x-beta -> 0.1.(x+1)-beta
#
# Run from repo root (or anywhere inside the repo):
#   ./scripts/bump-beta.sh

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_TOML="$ROOT_DIR/Cargo.toml"

if [[ ! -f "$CARGO_TOML" ]]; then
  echo "error: Cargo.toml not found at $CARGO_TOML" >&2
  exit 1
fi

current_version="$({
  awk '
    $0=="[package]" { in_pkg=1; next }
    /^\[/ && $0!="[package]" { in_pkg=0 }
    in_pkg && $1=="version" {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' "$CARGO_TOML"
} || true)"

if [[ -z "$current_version" ]]; then
  echo "error: could not read package version from Cargo.toml" >&2
  exit 1
fi

if [[ ! "$current_version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)-beta$ ]]; then
  echo "error: expected beta version format MAJOR.MINOR.PATCH-beta, got: $current_version" >&2
  exit 1
fi

major="${BASH_REMATCH[1]}"
minor="${BASH_REMATCH[2]}"
patch="${BASH_REMATCH[3]}"

next_patch=$((patch + 1))
new_version="${major}.${minor}.${next_patch}-beta"
new_tag="v${new_version}"

awk -v new_version="$new_version" '
  BEGIN { in_pkg=0; replaced=0 }
  $0=="[package]" { in_pkg=1 }
  in_pkg && /^\[/ && $0!="[package]" { in_pkg=0 }
  {
    if (in_pkg && $1=="version" && replaced==0) {
      print "version = \"" new_version "\""
      replaced=1
    } else {
      print
    }
  }
  END {
    if (replaced==0) exit 1
  }
' "$CARGO_TOML" > "$CARGO_TOML.tmp"
mv "$CARGO_TOML.tmp" "$CARGO_TOML"

# Keep Cargo.lock in sync with package version.
(
  cd "$ROOT_DIR"
  cargo check -q >/dev/null
)

echo "Bumped version: $current_version -> $new_version"
echo
echo "Next steps:"
echo "  git add Cargo.toml Cargo.lock"
echo "  git commit -m \"chore: bump version to $new_version\""
echo "  git tag -a $new_tag -m \"Release $new_tag\""
echo "  git push origin main"
echo "  git push origin $new_tag"
