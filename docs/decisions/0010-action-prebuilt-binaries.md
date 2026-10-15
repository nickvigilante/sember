# 10. The Action downloads prebuilt binaries, and pre-commit gets hooks that need no Rust

Status: accepted.
Recorded after the fact; the decision landed in commit `50af542`, released as 0.1.3 in `4db2307`.
It replaces the approach from `6525353` (#7).

## Context

The GitHub Action first compiled sember with cargo, caching on a fingerprint of the source.
Documented in `50af542`: "The Action compiled sember with cargo on every cache miss."
The first pre-commit hooks had the same cost, since `language: rust` makes pre-commit build sember, so the machine needs Rust.

## Decision

**The Action** (`action.yml`) maps the runner's OS and architecture to a release target, downloads the release that matches the version it was pinned to with `gh release download`, and verifies the `.sha256` file.
Anything unexpected falls back to `cargo install --locked`.
The comment says what counts as unexpected: "a platform without a prebuilt, a version that isn't released yet, a runner without gh, or a checksum mismatch."

**Pre-commit** gets two more hooks, `sember-system` and `sember-system-changed`, which use `language: system`.
Documented in `.pre-commit-hooks.yaml`: the `language: rust` hooks "have pre-commit build sember from this repository with cargo, so the machine running them needs Rust", while the `sember-system` hooks "run a sember you already installed (Homebrew, npm, a release binary), so they need no Rust."

The README gained a "Use in CI" section covering pinned installs and pnpm.

## Alternatives

The source build is kept, as the Action's fallback and as the original pre-commit hooks.
No others are documented.

## Consequences

- Faster Action runs and no Rust toolchain needed on most runners.
- The Action depends on a release existing for the version in `Cargo.toml`, so a release tag has to match the crate version ([record 8](0008-cargo-dist-releases.md)); inferred.
- Windows runners build from source, since no Windows prebuilt is in the Action's target map (inferred).
- The system hooks make the user install sember themselves (documented in the hooks file).
- `--changed` needs the base ref fetched in CI ([record 7](0007-changed-shells-out-to-git.md)), and the Action takes care of that.
