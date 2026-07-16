# Changelog

All notable changes to FusionServe will be documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases
use [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Unified Rust gateway for Triton and Dynamo/vLLM workloads.
- Configuration-driven model registry and workload classification.
- Bounded admission control, backpressure, circuit breaking, and safe retries.
- Prometheus metrics, structured tracing, and health-aware routing.
- Python and C++ client examples, mock services, and benchmark tooling.
- Checksum-verified ResNet-50 download and pinned Triton 26.04 validation tools.
- Pinned Dynamo/vLLM 1.2.0 single-GPU validation workflow.
- Ten reproducible CPU/mock fault-injection experiments and evidence.
- Provisioned Grafana dashboards, DCGM GPU panels, coverage, and environment records.
- Schema-v2 benchmark result integrity validation and documentation link checks.
- Critical-path in-process Triton/Dynamo handler tests, raising Rust line
  coverage to 80.65% and measured branch coverage to 61.54%.
- Sanitized CPU-only GPU qualification record, validation matrix, benchmark
  methodology, reproducibility guide, and proposal-only upstream candidate.

### Changed

- Backend health metrics now use opaque bounded identifiers instead of endpoint URLs.
- Graceful shutdown closes admission before the server drains active requests.
- Streaming client disconnects now increment the cancellation metric.
- Independent benchmark percentiles are reported side by side; invalid
  percentile subtraction has been removed.
- Mock streaming output is labeled as SSE events rather than model tokens.

[Unreleased]: https://github.com/poolanithinreddy/FusionServe/commits/main
