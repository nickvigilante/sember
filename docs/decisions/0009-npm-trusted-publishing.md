# 9. Publish to npm with Trusted Publishing instead of a token

Status: accepted.
Recorded after the fact; the decision landed in commit `6da6825` (#17), after the token-based version in `d09cae0` (#15).
It was tested with release 0.1.2 (`3fae940`) and tightened in `33318f1`.

## Context

The first npm publish used cargo-dist's built-in job, which needs an `NPM_TOKEN` secret.
A long-lived token stored in the repository is a standing risk.
The package is scoped, `@nickvigilante/sember`, because "the scoped name avoids npm's similar-name check, which could reject an unscoped 'sember' because of 'semver'" (documented, `d09cae0`).

## Decision

Publish through GitHub's OIDC identity with npm Trusted Publishing, and store no npm token.

Documented in `6da6825`: it "Replaces dist's built-in token-based npm publish job with a reusable workflow that publishes through GitHub OIDC, so no long-lived npm token is stored.
The npm installer is kept so dist still builds the package."

How it fits together:

- `dist-workspace.toml` lists the publish jobs as `["homebrew", "./publish-npm-trusted"]`.
- `.github/workflows/publish-npm-trusted.yml` is a reusable workflow with `id-token: write`.
  It installs `npm@^11.5.1` because Trusted Publishing needs a recent npm.
- `release.yml` calls it from a job that grants `id-token: write`.
- npm's trusted-publisher check sees the calling workflow, so the npm package settings must name `release.yml`.
- `33318f1` dropped the `contents` permission from the caller.
  The message is cut short in the log, but a called workflow cannot ask for more than its caller grants (inferred).

## Alternatives

The token-based job that cargo-dist generates was the first version and was replaced.
No others are documented.

## Consequences

- No secret to rotate or leak.
- Part of the setup is outside the repository: the trusted publisher on the package's npm settings page must name `nickvigilante/sember` and the workflow `release.yml`.
  This is documented in `6da6825`.
- The publish job is custom, so regenerating `release.yml` with cargo-dist could conflict with it (inferred).
- Release 0.1.2 carried no code changes and existed to prove this worked (documented in `3fae940`).
