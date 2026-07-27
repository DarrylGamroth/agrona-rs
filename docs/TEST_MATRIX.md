# Test and CI matrix

This matrix records the supported configurations exercised by automated tests.
It should be updated whenever the minimum Rust version, feature set, or
supported platform set changes.

| Area | Axis | Permutations | Evidence | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| Behavior | Test kind | Library unit, integration, and documentation tests | `cargo test --workspace --all-targets --all-features`; `cargo test --workspace --all-features --doc` | Covered | The all-targets invocation also builds and exercises benchmark targets. |
| Portability | Operating system | Linux, macOS, Windows | `test` job in `.github/workflows/ci.yml` | Covered in configuration | Execution evidence is available after the workflow runs on GitHub. |
| Toolchain | Rust channel | Stable | `quality` and three-OS `test` jobs in `.github/workflows/ci.yml` | Covered in configuration | Stable carries the formatting, linting, documentation, and behavioral checks. |
| Compatibility | Minimum Rust version | Rust 1.85 | `msrv` job in `.github/workflows/ci.yml` | Covered in configuration | This matches `package.rust-version` in `Cargo.toml`. |
| Coverage | Instrumented test suite | Linux stable | `coverage` job in `.github/workflows/ci.yml`; `codecov.yml` | Covered in configuration | `cargo-llvm-cov` emits LCOV and Codecov enforces project and patch status checks. |
| Features | Cargo feature set | All features | Every Cargo command in `.github/workflows/ci.yml` | Covered | The crate currently declares no optional features, so this is a single permutation. |

Dependabot checks both Cargo dependencies and GitHub Actions weekly via
`.github/dependabot.yml`.
