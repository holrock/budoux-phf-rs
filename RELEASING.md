# Releasing

Version bumps and the changelog live in a **single human commit**; CI only
builds, tests, and publishes. CI never writes to the repository, so there is no
push race between a version bump and a changelog update.

## During development

Add every notable change under the `## [Unreleased]` section of
[`CHANGELOG.md`](CHANGELOG.md) as part of the PR that makes the change. Use the
standard subsections (`Added`, `Changed`, `Fixed`, `Security`, …).

## Cutting a release

1. Make sure `main` is up to date and the working tree is clean, then run:

   ```bash
   scripts/release.sh 0.1.9      # the new X.Y.Z version
   ```

   This bumps `lib/Cargo.toml` and `wasm/Cargo.toml`, refreshes `Cargo.lock`,
   promotes `## [Unreleased]` to `## [0.1.9] - <today>` (and opens a fresh
   `[Unreleased]`), then creates a single commit `release: v0.1.9` and the tag
   `v0.1.9`. **Nothing is pushed.**

2. Review the commit:

   ```bash
   git show
   ```

3. Push the branch and the tag:

   ```bash
   git push && git push origin v0.1.9
   ```

The tag push triggers [`.github/workflows/release.yml`](.github/workflows/release.yml),
which:

- verifies the tag matches the versions in both `Cargo.toml` files and that
  `CHANGELOG.md` has a section for it (fails fast otherwise),
- runs the tests and builds the WASM packages,
- creates the GitHub Release with notes extracted from `CHANGELOG.md`,
- publishes `budoux-phf-rs` to crates.io (via OIDC).

## Re-running a failed release

If a release job fails after the tag already exists, fix the cause and re-run
the workflow manually from the Actions tab against the **tag ref** (`v0.1.9`).
The `skip_publish` input skips the crates.io publish for testing. If the crate
was already published, that step will fail — re-run only the parts that are
still pending, or cut a new patch version.

## Notes

- The version in the tag is the single source of truth; the workflow refuses to
  run if `Cargo.toml` and the tag disagree.
- Release notes come only from `CHANGELOG.md` — there is no auto-generated
  notes fallback, so keep `[Unreleased]` current.
