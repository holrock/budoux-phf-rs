#!/bin/bash
set -euo pipefail

# Prepare a release in a single, reviewable commit:
#   - bump version in lib/Cargo.toml and wasm/Cargo.toml
#   - refresh Cargo.lock
#   - promote the CHANGELOG "[Unreleased]" section to "[VERSION] - <date>"
#   - commit everything as "release: vVERSION" and create tag vVERSION
#
# Nothing is pushed. Review, then:
#   git push && git push origin vVERSION
# The tag push triggers .github/workflows/release.yml (build/test/publish only).

usage() {
  echo "Usage: $0 <version>"
  echo "  Example: $0 0.1.8"
  exit 1
}

[ $# -eq 1 ] || usage
VERSION="$1"

if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Error: version must be in X.Y.Z format (got: $VERSION)"
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

TAG="v$VERSION"
TODAY="$(date +%Y-%m-%d)"
CHANGELOG="CHANGELOG.md"

# Preconditions.
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: working tree is not clean. Commit or stash changes first."
  exit 1
fi
if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  echo "Error: tag $TAG already exists."
  exit 1
fi
if ! grep -qE '^## \[Unreleased\]' "$CHANGELOG"; then
  echo "Error: no '## [Unreleased]' section in $CHANGELOG. Add your changes there first."
  exit 1
fi

# Bump the [package] version of a Cargo.toml in place.
# awk rather than `sed -i`: the latter needs a backup suffix on BSD/macOS sed,
# and "^version = " on its own would also match a dependency's version line.
bump_version() {
  local file="$1" tmp
  tmp="$(mktemp)"
  awk -v ver="$VERSION" '
    /^\[/ { in_package = ($0 == "[package]") }
    in_package && /^version = / && !done { print "version = \"" ver "\""; done = 1; next }
    { print }
  ' "$file" > "$tmp"
  # Write back through the original file so its mode survives; `mv` from a
  # mktemp file would leave it at 0600.
  cat "$tmp" > "$file"
  rm -f "$tmp"
}

bump_version lib/Cargo.toml
bump_version wasm/Cargo.toml
echo "Updated lib/Cargo.toml and wasm/Cargo.toml to $VERSION"

# Refresh Cargo.lock for both workspace crates.
cargo update -p budoux-phf-rs
cargo update -p budoux-phf-rs-wasm

# Promote [Unreleased] -> [VERSION] - date, and open a fresh [Unreleased] above it.
CHANGELOG_TMP="$(mktemp)"
awk -v ver="$VERSION" -v today="$TODAY" '
  /^## \[Unreleased\]/ && !done {
    print
    print ""
    print "## [" ver "] - " today
    done = 1
    next
  }
  { print }
' "$CHANGELOG" > "$CHANGELOG_TMP"
cat "$CHANGELOG_TMP" > "$CHANGELOG"
rm -f "$CHANGELOG_TMP"
echo "Promoted CHANGELOG [Unreleased] -> [$VERSION] - $TODAY"

# One commit, one tag.
git add lib/Cargo.toml wasm/Cargo.toml Cargo.lock "$CHANGELOG"
git commit -m "release: $TAG"
git tag "$TAG"

echo
echo "Prepared $TAG. Review the commit, then push to trigger the release workflow:"
echo "  git show"
echo "  git push && git push origin $TAG"
