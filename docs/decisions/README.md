# Decision records

Short records of the decisions that shaped sember, and why.
Each one is numbered and is not rewritten later; a changed mind gets a new record that supersedes the old one.

These first ten were written after the fact, from the code, the README and the commit history.
Most of the original commits have no message body, so the reasoning could not always be recovered.
To keep that honest, every reason in these records is labelled:

- **Documented** means the reason is stated in a code comment, the README or a commit message, and is quoted or cited.
- **Inferred** means the reason is a reading of the code and is not stated anywhere.
  Treat it as a hypothesis, and correct it if you remember otherwise.

"Alternatives" lists only what the sources mention.
Where they mention none, the record says so rather than inventing some.

| # | Decision |
| --- | --- |
| [1](0001-edit-whitespace-only.md) | Edit only the whitespace between words; never re-render the document |
| [2](0002-verify-by-reparsing.md) | Re-parse the result and leave back any paragraph whose rendering would change |
| [3](0003-rule-based-sentence-detection.md) | Detect sentences with conservative rules, and keep ambiguous boundaries joined |
| [4](0004-styles-and-sembr-rules.md) | Three styles, and SemBr rules with a Never, Width or Always level |
| [5](0005-config-file-layout.md) | `.sember.toml` laid out like `.vale.ini`, strict about mistakes |
| [6](0006-check-mode-and-exit-codes.md) | `--check` and the exit-code contract |
| [7](0007-changed-shells-out-to-git.md) | `--changed` shells out to `git` and formats only changed paragraphs |
| [8](0008-cargo-dist-releases.md) | Build and publish releases with cargo-dist |
| [9](0009-npm-trusted-publishing.md) | Publish to npm with Trusted Publishing instead of a token |
| [10](0010-action-prebuilt-binaries.md) | The Action downloads prebuilt binaries, and pre-commit gets hooks that need no Rust |
