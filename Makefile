SHELL := /bin/sh
.DEFAULT_GOAL := help

DESKTOP_DIR := apps/desktop
TAURI_DIR := $(DESKTOP_DIR)/src-tauri

NPM ?= npm
PNPM ?= pnpm
CARGO ?= cargo
SQLX ?= sqlx

RUST_LOG ?= info

NPM_DESKTOP := $(NPM) --prefix $(DESKTOP_DIR)
PNPM_DESKTOP := $(PNPM) -C $(DESKTOP_DIR)
CARGO_MANIFEST := --manifest-path $(TAURI_DIR)/Cargo.toml

FRONTEND_ARTIFACTS := \
	$(DESKTOP_DIR)/dist \
	$(DESKTOP_DIR)/.vite \
	$(DESKTOP_DIR)/src/lib/tauri.ts

BACKEND_ARTIFACTS := \
	$(TAURI_DIR)/target \
	$(TAURI_DIR)/coverage \
	$(TAURI_DIR)/gen \
	$(TAURI_DIR)/build_rs_cov.profraw

DEPS_DIRS := \
	$(DESKTOP_DIR)/node_modules \
	node_modules

FRONTEND_CACHE_DIRS := \
	$(DESKTOP_DIR)/node_modules/.vite \
	$(DESKTOP_DIR)/node_modules/.cache

.PHONY: help \
	install doctor \
	dev dev-debug dev-trace dev-warn kill \
	bindings \
	release \
	build build-frontend build-backend \
	package package-universal package-arch package-all-macos \
	test test-frontend test-backend \
	check check-frontend check-backend fmt \
	clean clean-db purge \
	migrate

help: ## Show this help
	@echo "Usage: make <target>"
	@echo ""
	@echo "Targets:"
	@awk 'BEGIN {FS=":.*##"} /^[a-zA-Z0-9_\\-]+:.*##/ { printf "  %-22s %s\n", $$1, $$2 }' $(MAKEFILE_LIST) | sort

install: ## Install frontend dependencies
	@echo "📦 Installing frontend dependencies (pnpm workspace)..."
	@command -v $(PNPM) >/dev/null 2>&1 || (echo "❌ pnpm not found. Install with: npm i -g pnpm (or enable Corepack on supported Node builds)" && exit 1)
	@NODE_ENV=development $(PNPM) install --frozen-lockfile
	@echo "✅ Done"

doctor: ## Check dev environment (node/rust/sqlx)
	@echo "Node:"; node --version 2>/dev/null || echo "  ❌ missing"
	@echo "npm:"; $(NPM) --version 2>/dev/null || echo "  ❌ missing"
	@echo "pnpm:"; $(PNPM) --version 2>/dev/null || echo "  ⚠️  missing (run: corepack enable)"
	@echo "Rust:"; rustc --version 2>/dev/null || echo "  ❌ missing"
	@echo "Cargo:"; $(CARGO) --version 2>/dev/null || echo "  ❌ missing"
	@echo "SQLx:"; $(SQLX) --version 2>/dev/null || echo "  ⚠️  missing (optional)"

dev: ## Start dev app (RUST_LOG=info by default)
	@echo "🚀 Starting dev (RUST_LOG=$(RUST_LOG))..."
	@RUST_LOG=$(RUST_LOG) $(PNPM_DESKTOP) run tauri:dev

dev-debug: ## Start dev app (RUST_LOG=debug)
	@$(MAKE) dev RUST_LOG=debug

dev-trace: ## Start dev app (RUST_LOG=trace)
	@$(MAKE) dev RUST_LOG=trace

dev-warn: ## Start dev app (RUST_LOG=warn)
	@$(MAKE) dev RUST_LOG=warn

kill: ## Kill tauri/vite processes and ports
	@echo "🧹 Killing processes/ports..."
	@pkill -f "tauri dev" 2>/dev/null || true
	@pkill -f "NeuraDock" 2>/dev/null || true
	@pkill -f "neuradock" 2>/dev/null || true
	@pkill -f "vite" 2>/dev/null || true
	@pkill -f "npm run dev" 2>/dev/null || true
	@pkill -f "npm run tauri" 2>/dev/null || true
	@sleep 1
	@lsof -ti:1420 | xargs kill -9 2>/dev/null || true
	@lsof -ti:5173 | xargs kill -9 2>/dev/null || true
	@echo "✅ Done"

bindings: ## Generate TypeScript bindings (tauri-specta)
	@echo "🔗 Generating TS bindings..."
	@$(CARGO) run $(CARGO_MANIFEST) -p neuradock-app --bin export_ts_bindings
	@echo "✅ Generated $(DESKTOP_DIR)/src/lib/tauri.ts"

release: clean install ## Clean artifacts and produce release build (tauri:build)
	@echo "🧹 Cleaned artifacts. Starting release build..."
	@$(PNPM_DESKTOP) run tauri:build
	@echo "✅ Release build complete"

build: build-frontend build-backend ## Build frontend + backend (Release)
	@echo "✅ Build complete"

build-frontend: ## Build frontend (runs bindings via npm script)
	@$(PNPM_DESKTOP) run build

build-backend: ## Build backend (Release)
	@$(CARGO) build $(CARGO_MANIFEST) --release --workspace

package: install ## Build and package app (tauri build)
	@$(PNPM_DESKTOP) run tauri:build

package-universal: install ## macOS: package universal binary
	@rustup target add x86_64-apple-darwin 2>/dev/null || true
	@rustup target add aarch64-apple-darwin 2>/dev/null || true
	@$(PNPM_DESKTOP) run tauri:build -- --target universal-apple-darwin

package-arch: install ## macOS: package specific arch (ARCH=...)
	@if [ -z "$(ARCH)" ]; then \
		echo "❌ Missing ARCH. Example: make package-arch ARCH=x86_64-apple-darwin"; \
		exit 1; \
	fi
	@rustup target add $(ARCH) 2>/dev/null || true
	@$(PNPM_DESKTOP) run tauri:build -- --target $(ARCH)

package-all-macos: ## macOS: package aarch64 + x86_64 + universal
	@$(MAKE) package-arch ARCH=aarch64-apple-darwin
	@$(MAKE) package-arch ARCH=x86_64-apple-darwin
	@$(MAKE) package-universal

test: test-backend test-frontend ## Run all tests (backend + frontend)
	@echo "✅ Tests passed"

test-backend: ## Run backend tests (workspace)
	@$(CARGO) test $(CARGO_MANIFEST) --workspace

test-frontend: ## Run frontend tests (vitest run)
	@$(PNPM_DESKTOP) run test:run

check: check-backend check-frontend ## Run lint/type checks (backend + frontend)
	@echo "✅ Checks passed"

check-backend: ## cargo fmt --check + clippy -D warnings
	@cd $(TAURI_DIR) && cargo fmt --all --check
	@cd $(TAURI_DIR) && cargo clippy --workspace -- -D warnings

check-frontend: ## TypeScript typecheck (tsc --noEmit)
	@cd $(DESKTOP_DIR) && $(PNPM) exec tsc -p tsconfig.json --noEmit

fmt: ## Format Rust (cargo fmt)
	@cd $(TAURI_DIR) && cargo fmt --all

clean: ## Remove build artifacts (keeps dependencies)
	@echo "🧹 Cleaning artifacts..."
	@rm -rf $(FRONTEND_ARTIFACTS) $(BACKEND_ARTIFACTS) 2>/dev/null || true
	@rm -rf $(FRONTEND_CACHE_DIRS) 2>/dev/null || true
	@echo "✅ Done"

clean-db: ## Remove local sqlite db files (*.db)
	@rm -f *.db *.db-shm *.db-wal neuradock*.db neuradock*.db-shm neuradock*.db-wal 2>/dev/null || true

purge: clean clean-db ## Deep clean (artifacts + deps + db)
	@echo "🧹 Purging dependencies..."
	@find . -name "node_modules" -type d -prune -exec rm -rf {} + 2>/dev/null || true
	@rm -rf $(DEPS_DIRS) 2>/dev/null || true
	@rm -f "$(DESKTOP_DIR)/package-lock.json" "package-lock.json" 2>/dev/null || true
	@echo "✅ Done"

migrate: ## Run sqlx migrations against neuradock_dev.db
	@cd $(TAURI_DIR) && $(SQLX) migrate run --database-url sqlite:../../../neuradock_dev.db

