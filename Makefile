.PHONY: help dev dev-first setup install check-deps build build-frontend build-backend test test-backend clean clean-frontend clean-backend clean-all check fix logs kill rebuild

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
	@echo "  build            - 📦 构建生产版本 (前端 + 后端)"
	@echo "  build-frontend   - 📦 仅构建前端"
	@echo "  build-backend    - 📦 仅构建后端"
	@echo "  test             - 🧪 运行所有测试"
	@echo "  test-backend     - 🧪 运行后端测试"
	@echo "  clean            - 🧹 清理所有构建产物"
	@echo "  clean-all        - 🧹 深度清理（包括依赖）"
	@echo "  kill             - ⚠️  杀掉所有运行中的进程和端口"
	@echo "  check            - ✅ 检查代码格式"
	@echo "  fix              - 🔧 自动修复代码格式"
	@echo "  logs             - 📝 查看今天的日志"
	@echo "  install          - 📥 安装所有依赖 (同 setup)"
	@echo "  rebuild          - 🔄 清理后重新构建"
	@echo ""
	@echo "Examples:"
	@echo "  make setup       - 首次安装依赖"
	@echo "  make dev         - 启动开发服务器"
	@echo "  make kill dev    - 杀掉旧进程后启动开发"
	@echo "  make clean build - 清理后重新构建"

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

# 构建生产版本
build: build-frontend build-backend
	@echo "✅ 构建完成"
	@echo "二进制文件位置："
	@echo "  - macOS:   apps/desktop/src-tauri/target/release/bundle/dmg/"
	@echo "  - Windows: apps/desktop/src-tauri/target/release/bundle/msi/"
	@echo "  - Linux:   apps/desktop/src-tauri/target/release/bundle/appimage/"

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

# 清理构建产物
clean: clean-frontend clean-backend
	@echo "✅ 清理完成"

# 清理前端
clean-frontend:
	@echo "🧹 清理前端..."
	@rm -rf apps/desktop/dist
	@rm -rf apps/desktop/node_modules/.vite

# 清理后端
clean-backend:
	@echo "🧹 清理后端..."
	@cd apps/desktop/src-tauri && cargo clean

# 深度清理
clean-all:
	@echo "🧹 深度清理（包括依赖）..."
	@rm -rf apps/desktop/node_modules
	@rm -rf apps/desktop/dist
	@rm -rf apps/desktop/.vite
	@cd apps/desktop/src-tauri && cargo clean && rm -rf target
	@rm -rf ~/Library/Logs/neuradock
	@rm -f *.db *.db-shm *.db-wal
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
	@echo "Rust:"
	@rustc --version 2>/dev/null || echo "  ❌ 未安装"
	@echo ""
	@echo "Cargo:"
	@cargo --version 2>/dev/null || echo "  ❌ 未安装"
	@echo ""
	@echo "SQLx CLI:"
	@sqlx --version 2>/dev/null || echo "  ⚠️  未安装 (可选)"
