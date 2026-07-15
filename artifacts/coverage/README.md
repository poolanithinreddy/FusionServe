# Rust coverage

Measured locally on 2026-07-15 with:

```bash
cargo llvm-cov --workspace --all-features --summary-only
```

- Line coverage: **57.24%** (1,056 of 1,845 lines covered)
- Region coverage: **56.78%**
- Function coverage: **61.36%**

The figure reflects Rust unit and in-process API tests. Python-launched live
gateway integration tests are separate processes and are not included in this
Rust instrumentation result. Coverage was recorded as evidence, not inflated
with low-value tests.
