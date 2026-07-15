# Security Policy

## Reporting

Please report vulnerabilities privately via a GitHub Security Advisory on this
repository rather than a public issue. Include reproduction steps and impact.

## Scope & posture

- The gateway reads a non-secret YAML config and holds no credentials.
- **Admin endpoints** (`/admin/*`) mutate routing state and must be placed behind
  authentication / network policy before any non-local exposure. They are labeled
  development/admin endpoints.
- Metrics deliberately exclude request ids and prompt content to avoid data leaks.
- The container runs as a non-root user.
- Third-party engine images (Triton, Dynamo) are pulled from NVIDIA registries;
  pin exact tags and track their advisories.

This project has **not** undergone an independent security audit. Do not treat it
as production-ready without one (see `docs/limitations.md`).
