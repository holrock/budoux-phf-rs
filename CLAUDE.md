# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# The CI matrix is .github/workflows/ci.yml — all of it must pass before pushing.
# Regenerating models and building the WASM packages: see the README.

# One test
cargo test -p budoux-phf-rs parser::tests_parse_with::test_parse_with_sentence_end

# Prove the no_std path really is no_std (compiling for the host does not, and
# CI does not cover this)
rustup target add thumbv7m-none-eabi
cargo check -p budoux-phf-rs --no-default-features --features ja --target thumbv7m-none-eabi
```

## The parser must stay bit-compatible with upstream BudouX

`Parser::parse_with` is a transcription of `parse()` in BudouX's `budoux/parser.py`. Each
of the thirteen features has a guard deciding whether its window exists, and those guards
are the whole correctness surface — a wrong one changes where sentences split.

Two traps, both of which shipped as real bugs:

- **Write every length guard as an addition** (`i + 2 < len`), never a subtraction
  (`i < len - 2`). `len` is a `usize` and the loop runs for `len >= 2`, so `len - 3`
  underflows on a two-character input and panics in debug builds.
- **A wrong-length slice fails silently.** The maps are keyed by 1-, 2-, or 3-character
  strings, so a 4-character lookup just misses and contributes 0. A feature can be
  effectively disabled with nothing failing. The `sentence[ci(x)..]` branches exist only
  where the window genuinely runs to the end of the input; do not add one anywhere else.

Changing the scoring loop means diffing against upstream, not just running the tests —
the existing tests are long sentences and stayed green through both bugs above, which
only affect the last few boundaries. The way to check is a differential test: implement
upstream's loop over a `Vec<char>` using `parser.model`'s public maps and compare
`parse` against it over every substring of a corpus, for each bundled model.

The ring buffer in `parse_with` holds byte offsets for char indices `i-3..=i+3` in 8
slots; the prefetch loop must keep filling through `i+3` for that to hold.

## Generated files

`lib/src/model_*.rs` are generated — never hand-edit them. `total_score` must stay the
sum of every score in the thirteen maps (the parser derives its base score from it), and
`codegen` computes it. Regenerating against a different BudouX release pulls in new model
data and changes segmentation output, so it is a behavioural change, not a refactor.

## Tests must be feature-gated

Anything touching a bundled model needs `#[cfg(feature = "ja")]` (etc.), and `parse`
needs `alloc`. Without the gates the reduced-feature CI runs fail to compile.

## Releasing

Follow [RELEASING.md](RELEASING.md). Two things it is worth knowing before you get there:

- **Never create the tag from the Releases page.** That publishes an empty release, and
  because this repository has immutable releases enabled, it can never be given the WASM
  assets and its tag name can never be reused.
- Pre-1.0, any breaking change means the next release is a minor bump (0.1.x → 0.2.0),
  not a patch. `scripts/release.sh` takes whatever version you hand it.
