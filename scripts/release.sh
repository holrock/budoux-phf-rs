#!/bin/bash
set -euo pipefail

usage() {
  echo "Usage: $0 <version>"
  echo "  Example: $0 0.1.5"
  exit 1
}

if [ $# -ne 1 ]; then
  usage
fi

VERSION="$1"

# Validate semver format
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Error: version must be in X.Y.Z format (got: $VERSION)"
  exit 1
fi

CARGO_TOML="lib/Cargo.toml"
TAG="v$VERSION"

# Check working tree is clean
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: working tree is not clean. Commit or stash changes first."
  exit 1
fi

# Update version in Cargo.toml
sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$CARGO_TOML"

echo "Updated $CARGO_TOML to version $VERSION"

# Update Cargo.lock
cargo update -p budoux-phf-rs

# Commit and tag
git add "$CARGO_TOML" Cargo.lock
git commit -m "bump version to $VERSION"
git tag "$TAG"

echo "Created commit and tag $TAG"
echo "Run 'git push && git push origin $TAG' to trigger the release workflow."
