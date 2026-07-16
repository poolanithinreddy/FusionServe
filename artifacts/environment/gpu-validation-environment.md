# GPU validation environment — 2026-07-15

Status: **BLOCKED — no NVIDIA GPU**.

| Component | Sanitized value |
|---|---|
| Commit qualified | `f28e241c70b73bd3f7b2c9b867d8a5d3efd4cba3` |
| OS | macOS 26.5.1, Darwin 25.5.0, arm64 |
| CPU | Apple M4, 10 physical / 10 logical cores |
| RAM | 16 GiB |
| Available disk | approximately 230 GiB |
| NVIDIA GPU | none detected |
| `nvidia-smi` | unavailable |
| CUDA compiler | unavailable |
| Docker client | 28.1.1 |
| Docker Compose | 2.35.1-desktop.1 |
| Docker daemon | unavailable |
| NVIDIA Container Toolkit | not verifiable |
| Rust | 1.94.1 |
| Python | 3.9.6 |
| CMake | 4.4.0 (installed after GPU qualification for C++ validation) |
| ShellCheck | 0.11.0 |

The qualification intentionally excludes hostname, username, addresses,
filesystem paths, serial numbers, credentials, and cloud identifiers. The
minimal CUDA-container check could not run because both an NVIDIA device and an
active Docker daemon are absent. Under the validation protocol, all real-GPU
phases stop here and remain NR.
