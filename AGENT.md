# AGENT.md

This repository is `mito-reth`, a small overlay on top of upstream `reth` v1.11.3.

## Repository shape

- `bin/mito-reth`: CLI binary entry point
- `crates/primitives`: Mitosis-specific constants and bytecode
- `crates/evm`: EVM config, executor wrapper, and pre-execution hook logic

The core rule in this repo is to keep the Mitosis delta narrow. Prefer thin wrappers around upstream `reth` types and behavior. Avoid broad refactors unless they are clearly necessary.

## Build and test

- Rust effective MSRV is 1.91
- First build is slow because `reth` is pulled from git
- Common commands:

```bash
cargo build
cargo build --release
cargo test
cargo check
RUSTFLAGS="-D warnings" cargo check
cargo +nightly fmt --all
cargo +nightly fmt --all --check
```

## Release workflow

- GitHub Actions release workflow lives at `.github/workflows/release.yml`
- Supported release targets:
  - Linux `x86_64`
  - Linux `aarch64`
  - macOS `arm64`
  - Windows `x86_64`
- Manual `workflow_dispatch` runs are dry-run only:
  - they build and upload workflow artifacts
  - they do not create a GitHub Release
- Actual GitHub Release publication happens only on `v*.*.*` tag pushes

Before recommending a tag push, prefer validating the `Release` workflow through a manual dry-run.

## Editing guidance

- Keep upstream-compatible behavior unless the Mitosis-specific hook requires otherwise
- Put workspace-shared dependency versions in the root `Cargo.toml`
- Preserve zero-warning CI expectations
- Keep chain/block guards inside the hook implementation, not scattered at call sites
- If release automation changes, check both README and workflow behavior together

## Practical notes

- Release builds are heavy; expect multi-minute compile times
- If GitHub Actions failures appear only on one platform, inspect that job directly with `gh run view --job <job-id>`
- Prefer fixing workflow validation paths before asking for new tags or release recreation
