# 8. Build and publish releases with cargo-dist

Status: accepted.
Recorded after the fact; the decision landed in commit `7002abb` (#12), with the release process tested through `16b71de` and `5ce78ed` and the npm installer added in `d09cae0`.

## Context

Users install sember from different places: Homebrew, npm, a shell or PowerShell installer, or a plain binary.
Building those by hand for each platform on each release is slow and error-prone.

## Decision

Tagging `vX.Y.Z` builds and publishes everything through cargo-dist.

Documented in `7002abb`: "Tagging vX.Y.Z now builds binaries for macOS, Linux (musl) and Windows, creates a GitHub release with shell and PowerShell installers, and publishes a Homebrew formula to nickvigilante/homebrew-tap."

The configuration is `dist-workspace.toml`: cargo-dist 0.32.0, GitHub CI, the shell, PowerShell, Homebrew and npm installers, and five targets (aarch64 and x86_64 macOS, aarch64 and x86_64 Linux with musl, and x86_64 Windows MSVC).
`install-updater` is off and installs go to `CARGO_HOME`.
The workflow is `.github/workflows/release.yml`.

A `[profile.dist]` inherits from `release` and uses `lto = "thin"`, while `[profile.release]` uses full `lto` and one codegen unit.
No reason is recorded for the thin setting; faster release builds in CI is a plausible one (inferred).

## Alternatives

None documented.

## Consequences

- cargo-dist requires the tag to match the crate version (documented in `16b71de`), so a test tag such as `v0.1.0-rc.1` needed a version bump, which `5ce78ed` then restored.
- Linux builds use musl, so the binaries are static (inferred).
- Windows has a release build but no prebuilt binary for the GitHub Action ([record 10](0010-action-prebuilt-binaries.md)), so Windows runners build from source (inferred).
- The release workflow is generated, and a custom publish job sits beside it ([record 9](0009-npm-trusted-publishing.md)), so regenerating the workflow needs care (inferred).
- The crate package leaves out `.github/` and `dist-workspace.toml` (`a058a98`; documented as what it does, with the reason inferred as these files being of no use to crate users).
