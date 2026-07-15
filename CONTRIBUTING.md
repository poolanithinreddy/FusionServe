# Contributing to FusionServe

Thanks for your interest! FusionServe is a systems-engineering project; PRs that
improve correctness, tests, benchmarks, or docs are welcome.

## Development

```bash
make build      # cargo build
make test       # rust tests + python unit tests
make lint       # fmt + clippy -D warnings + ruff + mypy
bash tests/failure/run_failure_suite.sh   # fault injection (mock stack)
```

The mock stack (`tests/mocks/`) means you can develop the entire gateway without
a GPU. GPU-only work (real benchmarks, TensorRT engines) is run manually or on a
self-hosted runner.

## Ground rules

- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` must pass.
- Add/extend tests for behavior changes. New failure modes get a scenario.
- **Claim discipline** (see `docs/limitations.md`): don't describe configuration
  as implementation, or unmeasured behavior as benchmarked. Every claim in code
  or docs must be traceable to evidence.
- No model binaries or secrets in commits.

## Commit / PR

- Small, focused commits. Reference the milestone (M1–M8) where relevant.
- Fill in the PR template; note what you verified and how.
