# Rust coverage

Measured locally on 2026-07-15 with:

```bash
cargo llvm-cov --workspace --all-features --summary-only
cargo +nightly llvm-cov --workspace --all-features --branch --summary-only
```

- Line coverage: **80.65%** (1,488 of 1,845 lines covered)
- Branch coverage: **61.54%** (80 of 130 branches covered)
- Region coverage: **79.38%**
- Function coverage: **78.64%**

The figure reflects Rust unit and in-process API tests. Python-launched live
gateway integration tests are separate processes and are not included. The
increase comes from in-process backend fixtures covering real Rust request
handlers, clients, policy, streaming, error classification, and propagation.
