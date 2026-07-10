#!/bin/bash
set -euo pipefail

# Build WASM packages with wasm-pack.
#
# Produces, for each target (web, bundler):
#   - a "full" package with all default languages   -> wasm/pkg/<target>
#   - one package per individual language           -> wasm/pkg/<target>-<lang>
#
# The full packages keep their original output dirs (wasm/pkg/web, wasm/pkg/bundler)
# so existing release asset names stay unchanged. Per-language packages are added
# alongside them.
#
# Usage:
#   scripts/build-wasm.sh [target ...]
#     target: web | bundler   (default: both)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# Every language feature exposed by the wasm crate.
LANGS=(ja ja_knbc th zh_hans zh_hant)

if [ "$#" -gt 0 ]; then
  TARGETS=("$@")
else
  TARGETS=(web bundler)
fi

build() {
  local target="$1" out="$2"; shift 2
  echo "==> wasm-pack build ($target) -> wasm/pkg/${out} $*"
  wasm-pack build wasm --target "$target" --out-dir "pkg/${out}" "$@"
}

for target in "${TARGETS[@]}"; do
  # Full package: all default languages.
  build "$target" "$target"
  # Per-language packages: one language each.
  for lang in "${LANGS[@]}"; do
    build "$target" "${target}-${lang}" -- --no-default-features --features "$lang"
  done
done

echo "Done. Packages are under wasm/pkg/"
