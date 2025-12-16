.PHONY: help dev dev-fast dev-first setup install check-deps build build-release build-release-fast build-universal build-arch build-all-macos build-frontend build-backend test test-backend test-coverage coverage-report clean clean-frontend clean-backend clean-all check fix logs kill rebuild migrate status bindings env-check version run-release update-deps outdated install-rust-tools fix-permissions validate-actions show-targets

# 默认目标
help:
	@echo "NeuraDock Build Commands"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "⚠️  首次使用请运行: make setup"
	@echo ""
	@echo "Targets:"
	@echo "  setup            - 🔧 首次安装所有依赖 (必须先运行)"
	@echo "  dev              - 🚀 启动开发模式 (RUST_LOG=info)"
	@echo "  dev-debug        - 🐛 启动开发模式 (RUST_LOG=debug - 详细日志)"
	@echo "  dev-trace        - 🔍 启动开发模式 (RUST_LOG=trace - 性能追踪)"
	@echo "  dev-warn         - ⚠️  启动开发模式 (RUST_LOG=warn - 仅警告)"
	@echo "  dev-first        - 🆕 首次运行 (自动安装依赖并启动)"
	@echo "  check-deps       - 🔍 检查依赖是否已安装"
	@echo "  build            - 📦 构建 Release 版本（不打包）"
	@echo "  build-release    - 🎁 构建并打包 Release 版本（生成安装包）"
	@echo "  build-release-fast - ⚡ 快速构建 Release（不打包）"
	@echo "  build-frontend   - 📦 仅构建前端"
	@echo "  build-backend    - 📦 仅构建后端"
	@echo "  run-release      - 🚀 运行 Release 版本"
	@echo "  test             - 🧪 运行所有测试"
	@echo "  test-backend     - 🧪 运行后端测试"
	@echo "  test-coverage    - 📊 运行测试并生成覆盖率报告"
	@echo "  coverage-report  - 📈 打开覆盖率报告 (HTML)"
	@echo "  clean            - 🧹 清理所有构建产物"
	@echo "  clean-all        - 🧹 深度清理（包括依赖）"
	@echo "  kill             - ⚠️  杀掉所有运行中的进程和端口"
	@echo "  check            - ✅ 检查代码格式"
	@echo "  fix              - 🔧 自动修复代码格式"
	@echo "  logs             - 📝 查看今天的日志"
	@echo "  install          - 📥 安装所有依赖 (同 setup)"
	@echo "  rebuild          - 🔄 清理后重新构建"
	@echo "  migrate          - 🗄️  运行数据库迁移"
	@echo "  status           - 📊 查看项目状态"
	@echo "  bindings         - 🔗 生成 TypeScript 绑定"
	@echo "  env-check        - 🔍 检查开发环境"
	@echo "  version          - 📋 显示版本信息"
	@echo "  update-deps      - 📦 更新所有依赖"
	@echo "  outdated         - 🔍 检查过时的依赖"
	@echo "  install-rust-tools - 🔧 安装 Rust 开发工具"
	@echo "  fix-permissions  - 🔧 修复文件权限"
	@echo "  dev-fast         - ⚡ 快速启动（跳过检查）"
	@echo "  build-universal  - 🍎 构建 macOS Universal Binary"
	@echo "  build-all-macos  - 🍎 构建所有 macOS 架构（aarch64 + x86_64 + universal）"
	@echo "  build-arch       - 📦 构建指定架构（需要 ARCH=xxx）"
	@echo "  validate-actions - ✅ 验证 GitHub Actions 配置"
	@echo "  show-targets     - 📋 显示所有可用的构建目标"
	@echo ""
	@echo "Examples:"
	@echo "  make setup              - 首次安装依赖"
	@echo "  make dev                - 启动开发服务器"
	@echo "  make build-release      - 构建当前架构版本"
	@echo "  make build-universal    - 构建 Universal Binary"
	@echo "  make build-all-macos    - 构建所有 macOS 架构"
	@echo "  make build-arch ARCH=x86_64-apple-darwin  - 构建 Intel 版本"
	@echo "  make kill dev           - 杀掉旧进程后启动开发"
	@echo "  make clean build        - 清理后重新构建"

# 杀掉所有进程
kill:
	@echo "🧹 清理所有进程和端口..."
	@pkill -f "tauri dev" 2>/dev/null || true
	@pkill -f "neuradock" 2>/dev/null || true
	@pkill -f "vite" 2>/dev/null || true
	@pkill -f "npm run dev" 2>/dev/null || true
	@pkill -f "npm run tauri" 2>/dev/null || true
	@sleep 1
	@lsof -ti:1420 | xargs kill -9 2>/dev/null || true
	@lsof -ti:5173 | xargs kill -9 2>/dev/null || true
	@echo "✅ 进程清理完成"

# 检查依赖是否已安装
check-deps:
	@echo "🔍 检查依赖..."
	@if [ ! -d "apps/desktop/node_modules" ]; then \
		echo "❌ 依赖未安装！"; \
		echo ""; \
		echo "请先运行: make setup"; \
		echo ""; \
		exit 1; \
	fi
	@echo "✅ 依赖已安装"

# 首次安装 - 安装所有依赖
setup:
	@echo "🔧 首次安装 - 设置开发环境..."
	@echo ""
	@echo "📦 安装 apps/desktop 依赖..."
	@cd apps/desktop && NODE_ENV=development npm install --legacy-peer-deps
	@echo ""
	@echo "✅ 安装完成！"
	@echo ""
	@echo "现在可以运行: make dev"

# 快捷方式：安装依赖
install: setup

# 首次运行 - 安装依赖并启动
dev-first:
	@echo "🆕 首次运行 - 安装依赖并启动开发模式..."
	@$(MAKE) setup
	@echo ""
	@$(MAKE) dev

# 开发模式 - 需要先安装依赖
dev: kill check-deps
	@echo "🚀 启动开发模式 (RUST_LOG=info)..."
	@cd apps/desktop && RUST_LOG=info npm run tauri:dev

# 开发模式 - 详细日志 (debug 级别)
dev-debug: kill check-deps
	@echo "🚀 启动开发模式 (RUST_LOG=debug)..."
	@cd apps/desktop && RUST_LOG=debug npm run tauri:dev

# 开发模式 - 性能追踪 (trace 级别 + spans)
dev-trace: kill check-deps
	@echo "🚀 启动开发模式 (RUST_LOG=trace - 性能追踪)..."
	@cd apps/desktop && RUST_LOG=trace npm run tauri:dev

# 开发模式 - 仅警告和错误
dev-warn: kill check-deps
	@echo "🚀 启动开发模式 (RUST_LOG=warn)..."
	@cd apps/desktop && RUST_LOG=warn npm run tauri:dev

# 构建开发版本（不打包）
build: build-frontend build-backend
	@echo "✅ 构建完成"
	@echo "二进制文件位置: apps/desktop/src-tauri/target/release/neuradock"

# 构建并打包 release 版本（当前架构）
build-release: check-deps
	@echo "📦 构建 Release 版本（当前架构：$$(uname -m)）..."
	@cd apps/desktop && npm run tauri:build
	@echo "✅ Release 构建完成"
	@echo ""
	@echo "安装包位置："
	@echo "  - macOS:   apps/desktop/src-tauri/target/release/bundle/dmg/"
	@echo "  - Windows: apps/desktop/src-tauri/target/release/bundle/msi/"
	@echo "  - Linux:   apps/desktop/src-tauri/target/release/bundle/appimage/"
	@echo ""
	@echo "查看详细构建产物："
	@ls -lh apps/desktop/src-tauri/target/release/bundle/*/ 2>/dev/null || true

# 构建 macOS Universal Binary（支持 Intel + Apple Silicon）
build-universal: check-deps
	@echo "🍎 构建 macOS Universal Binary（Intel + Apple Silicon）..."
	@echo ""
	@echo "📋 检查并安装必要的 Rust targets..."
	@rustup target add x86_64-apple-darwin 2>/dev/null || true
	@rustup target add aarch64-apple-darwin 2>/dev/null || true
	@echo ""
	@echo "🔨 构建中..."
	@cd apps/desktop && npm run tauri:build -- --target universal-apple-darwin
	@echo ""
	@echo "✅ Universal Binary 构建完成"
	@echo ""
	@echo "📦 安装包位置："
	@ls -lh apps/desktop/src-tauri/target/universal-apple-darwin/release/bundle/dmg/ 2>/dev/null || \
	ls -lh apps/desktop/src-tauri/target/release/bundle/dmg/ 2>/dev/null || true

# 构建指定架构的版本
build-arch: check-deps
	@if [ -z "$(ARCH)" ]; then \
		echo "❌ 请指定架构"; \
		echo ""; \
		echo "用法: make build-arch ARCH=<架构>"; \
		echo ""; \
		echo "可用架构:"; \
		echo "  - aarch64-apple-darwin    (Apple Silicon)"; \
		echo "  - x86_64-apple-darwin     (Intel Mac)"; \
		echo "  - universal-apple-darwin  (Universal Binary)"; \
		echo ""; \
		exit 1; \
	fi
	@echo "📦 构建 $(ARCH) 版本..."
	@rustup target add $(ARCH) 2>/dev/null || true
	@cd apps/desktop && npm run tauri:build -- --target $(ARCH)
	@echo "✅ $(ARCH) 构建完成"

# 构建所有 macOS 架构
build-all-macos: check-deps
	@echo "🍎 构建所有 macOS 架构..."
	@echo ""
	@echo "1️⃣  构建 Apple Silicon (aarch64)..."
	@$(MAKE) build-arch ARCH=aarch64-apple-darwin
	@echo ""
	@echo "2️⃣  构建 Intel (x86_64)..."
	@$(MAKE) build-arch ARCH=x86_64-apple-darwin
	@echo ""
	@echo "3️⃣  构建 Universal Binary..."
	@$(MAKE) build-universal
	@echo ""
	@echo "✅ 所有 macOS 架构构建完成！"
	@echo ""
	@echo "📦 安装包列表："
	@find apps/desktop/src-tauri/target -name "*.dmg" -type f -exec ls -lh {} \;

# 快速构建 release（不打包，仅编译）
build-release-fast: build-frontend build-backend
	@echo "✅ 快速 Release 构建完成（未打包）"

# 构建前端
build-frontend: check-deps
	@echo "📦 构建前端..."
	@cd apps/desktop && npm run build

# 构建后端
build-backend:
	@echo "🦀 构建后端 (Release)..."
	@cd apps/desktop/src-tauri && cargo build --release --workspace

# 运行所有测试
test: test-backend
	@echo "✅ 所有测试完成"

# 运行后端测试
test-backend:
	@echo "🧪 运行后端测试..."
	@cd apps/desktop/src-tauri && cargo test --workspace

# 运行测试并生成覆盖率报告
test-coverage:
	@echo "📊 运行测试并生成覆盖率报告..."
	@if ! command -v cargo-tarpaulin &> /dev/null; then \
		echo "❌ cargo-tarpaulin 未安装"; \
		echo "安装: cargo install cargo-tarpaulin"; \
		exit 1; \
	fi
	@cd apps/desktop/src-tauri && cargo tarpaulin --workspace --lib --target-dir target/coverage --out Html --out Json --out Lcov --output-dir coverage
	@echo "✅ 覆盖率报告已生成"
	@cd apps/desktop/src-tauri && grep "coverage" coverage/tarpaulin-report.json | head -1 || true
	@echo ""
	@echo "报告位置:"
	@echo "  HTML: apps/desktop/src-tauri/coverage/tarpaulin-report.html"
	@echo "  JSON: apps/desktop/src-tauri/coverage/tarpaulin-report.json"
	@echo "  LCOV: apps/desktop/src-tauri/coverage/lcov.info"

# 打开覆盖率报告
coverage-report:
	@echo "📈 打开覆盖率报告..."
	@if [ -f "apps/desktop/src-tauri/coverage/index.html" ]; then \
		open apps/desktop/src-tauri/coverage/index.html; \
	else \
		echo "❌ 覆盖率报告不存在"; \
		echo "请先运行: make test-coverage"; \
	fi

# 清理构建产物
clean: clean-frontend clean-backend
	@echo "✅ 清理完成"

# 清理前端
clean-frontend:
	@echo "🧹 清理前端..."
	find . -name "node_modules" -type d -prune -exec rm -rf {} + 2>/dev/null || true
	find . -name "package-lock.json" -type f -delete 2>/dev/null || true

# 清理后端
clean-backend:
	@echo "🧹 清理后端..."
	@cd apps/desktop/src-tauri && cargo clean
	@rm -rf apps/desktop/src-tauri/target/coverage
	@rm -rf apps/desktop/src-tauri/coverage
	@rm -rf apps/desktop/dist 
	@rm -rf apps/desktop/src-tauri/target
	

# 深度清理
clean-all:
	@echo "🧹 深度清理（包括依赖）..."
	@echo "正在删除 node_modules..."
	@rm -rf apps/desktop/node_modules
	@rm -rf node_modules
	@echo "正在删除前端构建产物..."
	@rm -rf apps/desktop/dist
	@rm -rf apps/desktop/.vite
	@rm -rf apps/desktop/node_modules/.vite
	@echo "正在删除后端构建产物..."
	@cd apps/desktop/src-tauri && cargo clean && rm -rf target
	@rm -rf apps/desktop/src-tauri/coverage
	@echo "正在删除日志..."
	@rm -rf ~/Library/Logs/neuradock
	@rm -rf ~/Library/Logs/com.neuradock.app
	@echo "正在删除数据库文件..."
	@rm -f *.db *.db-shm *.db-wal
	@rm -f neuradock*.db neuradock*.db-shm neuradock*.db-wal
	@echo "✅ 深度清理完成"

# 代码检查
check:
	@echo "🔍 检查代码格式..."
	@cd apps/desktop/src-tauri && cargo fmt --all --check
	@cd apps/desktop/src-tauri && cargo clippy --workspace -- -D warnings
	@echo "✅ 代码检查完成"

# 自动修复
fix:
	@echo "🔧 自动修复代码格式..."
	@cd apps/desktop/src-tauri && cargo fmt --all
	@echo "✅ 代码格式修复完成"

# 查看日志
logs:
	@echo "📋 查看今天的日志..."
	@LOG_FILE="$$HOME/Library/Logs/neuradock/logs/neuradock.log.$$(date +%Y-%m-%d)"; \
	if [ -f "$$LOG_FILE" ]; then \
		if command -v jq &> /dev/null; then \
			cat "$$LOG_FILE" | jq .; \
		else \
			cat "$$LOG_FILE"; \
		fi \
	else \
		echo "未找到今天的日志文件"; \
		ls -lh ~/Library/Logs/neuradock/logs/ 2>/dev/null || echo "日志目录不存在"; \
	fi

# 快速重新构建（清理后构建）
rebuild: clean build
	@echo "✅ 重新构建完成"

# 数据库迁移
migrate:
	@echo "🗄️  运行数据库迁移..."
	@cd apps/desktop/src-tauri && sqlx migrate run --database-url sqlite:../../../neuradock_dev.db

# 查看项目状态
status:
	@echo "📊 项目状态"
	@echo ""
	@echo "前端依赖:"
	@cd apps/desktop && npm list --depth=0 2>/dev/null | head -20 || echo "  未安装"
	@echo ""
	@echo "后端依赖:"
	@cd apps/desktop/src-tauri && cargo tree --depth=1 2>/dev/null | head -20 || echo "  Cargo.lock 不存在"
	@echo ""
	@echo "数据库:"
	@ls -lh *.db 2>/dev/null || echo "  无数据库文件"
	@echo ""
	@echo "端口占用:"
	@lsof -ti:1420 &>/dev/null && echo "  Port 1420: 占用" || echo "  Port 1420: 空闲"
	@lsof -ti:5173 &>/dev/null && echo "  Port 5173: 占用" || echo "  Port 5173: 空闲"

# 生成 TypeScript 绑定
bindings:
	@echo "🔗 生成 TypeScript 绑定..."
	@cd apps/desktop/src-tauri && cargo build --workspace
	@echo "✅ 绑定已生成到 apps/desktop/src/lib/tauri.ts"

# 开发环境检查
env-check:
	@echo "🔍 检查开发环境..."
	@echo ""
	@echo "Node.js:"
	@node --version 2>/dev/null || echo "  ❌ 未安装"
	@echo ""
	@echo "npm:"
	@npm --version 2>/dev/null || echo "  ❌ 未安装"
	@echo ""
	@echo "pnpm:"
	@pnpm --version 2>/dev/null || echo "  ⚠️  未安装 (可选)"
	@echo ""
	@echo "Rust:"
	@rustc --version 2>/dev/null || echo "  ❌ 未安装"
	@echo ""
	@echo "Cargo:"
	@cargo --version 2>/dev/null || echo "  ❌ 未安装"
	@echo ""
	@echo "SQLx CLI:"
	@sqlx --version 2>/dev/null || echo "  ⚠️  未安装 (可选)"
	@echo ""
	@echo "cargo-tarpaulin:"
	@cargo tarpaulin --version 2>/dev/null || echo "  ⚠️  未安装 (可选，用于测试覆盖率)"

# 运行 Release 版本
run-release:
	@echo "🚀 运行 Release 版本..."
	@if [ -f "apps/desktop/src-tauri/target/release/neuradock" ]; then \
		./apps/desktop/src-tauri/target/release/neuradock; \
	else \
		echo "❌ Release 二进制文件不存在"; \
		echo "请先运行: make build"; \
		exit 1; \
	fi

# 显示版本信息
version:
	@echo "📋 NeuraDock 版本信息"
	@echo ""
	@echo "项目版本:"
	@grep "version" apps/desktop/src-tauri/Cargo.toml | head -1 || echo "  未找到"
	@echo ""
	@echo "Tauri 版本:"
	@grep "tauri" apps/desktop/src-tauri/Cargo.toml | grep "version" | head -1 || echo "  未找到"
	@echo ""
	@echo "React 版本:"
	@grep '"react"' apps/desktop/package.json | head -1 || echo "  未找到"

# 更新依赖
update-deps:
	@echo "📦 更新依赖..."
	@echo "更新前端依赖..."
	@cd apps/desktop && npm update --legacy-peer-deps
	@echo ""
	@echo "更新后端依赖..."
	@cd apps/desktop/src-tauri && cargo update
	@echo "✅ 依赖更新完成"

# 检查过时的依赖
outdated:
	@echo "🔍 检查过时的依赖..."
	@echo ""
	@echo "前端依赖:"
	@cd apps/desktop && npm outdated || true
	@echo ""
	@echo "后端依赖:"
	@cd apps/desktop/src-tauri && cargo outdated 2>/dev/null || echo "  需要安装 cargo-outdated: cargo install cargo-outdated"

# 安装 Rust 开发工具
install-rust-tools:
	@echo "🔧 安装 Rust 开发工具..."
	@echo "安装 sqlx-cli..."
	@cargo install sqlx-cli --no-default-features --features sqlite
	@echo ""
	@echo "安装 cargo-tarpaulin (测试覆盖率)..."
	@cargo install cargo-tarpaulin
	@echo ""
	@echo "安装 cargo-outdated..."
	@cargo install cargo-outdated
	@echo "✅ Rust 工具安装完成"

# 快速开发启动（跳过检查）
dev-fast:
	@echo "⚡ 快速启动开发模式（跳过依赖检查）..."
	@cd apps/desktop && RUST_LOG=info npm run tauri:dev

# 修复权限问题
fix-permissions:
	@echo "🔧 修复文件权限..."
	@chmod +x apps/desktop/src-tauri/target/release/neuradock 2>/dev/null || true
	@chmod -R u+w apps/desktop/node_modules 2>/dev/null || true
	@echo "✅ 权限修复完成"

# 验证 GitHub Actions workflow
validate-actions:
	@echo "🔍 验证 GitHub Actions..."
	@./.github/workflows/validate.sh

# 显示可用的构建目标
show-targets:
	@echo "📋 可用的构建目标："
	@echo ""
	@echo "🍎 macOS:"
	@echo "  make build-release          - 构建当前架构 ($$(uname -m))"
	@echo "  make build-universal        - 构建 Universal Binary (Intel + Apple Silicon)"
	@echo "  make build-all-macos        - 构建所有 macOS 架构"
	@echo "  make build-arch ARCH=x86_64-apple-darwin     - Intel Mac"
	@echo "  make build-arch ARCH=aarch64-apple-darwin    - Apple Silicon"
	@echo ""
	@echo "🌍 跨平台构建 (需要 GitHub Actions):"
	@echo "  使用 GitHub Actions 自动构建 Windows、Linux、macOS"
	@echo "  1. 推送带 tag 的提交: git tag v0.1.0 && git push origin v0.1.0"
	@echo "  2. 或手动触发: 在 GitHub Actions 页面选择 'Release' workflow"
	@echo ""
	@echo "📦 构建产物位置:"
	@echo "  - macOS DMG:  apps/desktop/src-tauri/target/*/release/bundle/dmg/"
	@echo "  - Windows:    apps/desktop/src-tauri/target/*/release/bundle/msi/"
	@echo "  - Linux DEB:  apps/desktop/src-tauri/target/*/release/bundle/deb/"
	@echo "  - Linux RPM:  apps/desktop/src-tauri/target/*/release/bundle/rpm/"
	@echo "  - AppImage:   apps/desktop/src-tauri/target/*/release/bundle/appimage/"
