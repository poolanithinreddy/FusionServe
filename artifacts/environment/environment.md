# Validation environment

- Audit date: 2026-07-15
- Classification: **CPU-only**
- OS: macOS 26.5.1, Darwin 25.5.0, arm64
- Hardware: Apple M4 MacBook Air, 10 CPU cores
- RAM: 16 GB
- Available workspace disk at audit: 232 GiB
- NVIDIA GPU: none
- GPU memory: not applicable
- NVIDIA driver: not installed
- CUDA: not installed
- NVIDIA Container Toolkit: unavailable
- Docker client: 28.1.1
- Docker Compose: 2.35.1
- Docker daemon: installed but not running during audit
- Rust compiler: 1.94.1
- Cargo: 1.94.1
- Python: 3.9.6
- Git commit at environment audit: `13e3690`

No usernames, hostnames, IP addresses, home-directory paths, credentials, or
hardware serial identifiers are included. Real Triton and Dynamo/vLLM GPU
validation was **NOT RUN** because the machine has no NVIDIA GPU and the Docker
daemon was unavailable.
