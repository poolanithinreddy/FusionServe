# FusionServe developer Makefile
# Most targets run against MOCK backends and need no GPU.

SHELL := /bin/bash
GATEWAY := gateway-rs
COMPOSE := docker compose

.PHONY: help
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

# ---- Rust gateway ---------------------------------------------------------
.PHONY: build
build: ## Build the Rust gateway
	cargo build -p fusionserve-gateway

.PHONY: run
run: ## Run the gateway locally (expects mocks on :8001 and :8000)
	cargo run -p fusionserve-gateway -- --config gateway-rs/config.yaml

.PHONY: test
test: ## Run gateway unit + integration tests
	cargo test -p fusionserve-gateway

.PHONY: lint
lint: ## fmt check + clippy (deny warnings) + python lint
	cargo fmt --all -- --check && cargo clippy -p fusionserve-gateway --all-targets --all-features -- -D warnings
	ruff check clients/python tests benchmarks scripts
	ruff format --check clients/python tests benchmarks scripts
	mypy clients/python/fusionserve
	python3 benchmarks/validate_results.py artifacts/benchmarks/mock
	python3 scripts/check_markdown_links.py .

.PHONY: audit
audit: ## Security audit of Rust deps (needs cargo-audit)
	cargo audit

# ---- Mock stack (no GPU) --------------------------------------------------
.PHONY: mocks
mocks: ## Start mock Triton (:8001) and mock Dynamo (:8000) in background
	@python3 tests/mocks/mock_triton.py --port 8001 & echo $$! > /tmp/fs_triton.pid
	@python3 tests/mocks/mock_dynamo.py --port 8000 & echo $$! > /tmp/fs_dynamo.pid
	@echo "mock backends started"

.PHONY: mocks-down
mocks-down: ## Stop mock backends
	-@kill `cat /tmp/fs_triton.pid` 2>/dev/null || true
	-@kill `cat /tmp/fs_dynamo.pid` 2>/dev/null || true
	@rm -f /tmp/fs_triton.pid /tmp/fs_dynamo.pid
	@echo "mock backends stopped"

.PHONY: dev-up
dev-up: ## Bring up the full mock stack via docker-compose
	$(COMPOSE) --profile mock up --build -d

.PHONY: dev-down
dev-down: ## Tear down the mock stack
	$(COMPOSE) --profile mock down -v

# ---- GPU stack ------------------------------------------------------------
.PHONY: gpu-up
gpu-up: ## Bring up the real Triton+Dynamo+vLLM stack (requires NVIDIA GPU)
	$(COMPOSE) --profile gpu up -d

.PHONY: gpu-down
gpu-down: ## Tear down the GPU stack
	$(COMPOSE) --profile gpu down

# ---- Benchmarks -----------------------------------------------------------
.PHONY: bench-mock
bench-mock: ## Run benchmark harness against mock backends (smoke, not for reporting)
	python3 benchmarks/gateway/run_gateway_overhead.py --target http://localhost:8080 --requests 200

.PHONY: report
report: ## Regenerate benchmark report from results/*.json
	python3 benchmarks/generate_report.py --input benchmarks/results --out docs/benchmark-results.md
