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
	cd $(GATEWAY) && cargo build

.PHONY: run
run: ## Run the gateway locally (expects mocks on :8001 and :8000)
	cd $(GATEWAY) && cargo run

.PHONY: test
test: ## Run gateway unit + integration tests
	cd $(GATEWAY) && cargo test

.PHONY: lint
lint: ## fmt check + clippy (deny warnings) + python lint
	cd $(GATEWAY) && cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings
	-ruff check clients/python tests benchmarks
	-mypy clients/python/fusionserve

.PHONY: audit
audit: ## Security audit of Rust deps (needs cargo-audit)
	cd $(GATEWAY) && cargo audit

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
