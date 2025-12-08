# Developer Documentation

This documentation is for NeuraDock developers, including build, release, and contribution guidelines.

## 📋 Table of Contents

- [Quick Start](#quick-start)
- [Local Build](#local-build)
- [CI/CD Configuration](#cicd-configuration)
- [Release Process](#release-process)
- [Contributing](#contributing)

---

## 🚀 Quick Start

### Requirements

- Node.js 20+
- Rust (via rustup)
- macOS / Windows / Linux

### Install Dependencies

\`\`\`bash
# First-time setup
make setup

# Or
cd apps/desktop && npm install --legacy-peer-deps
\`\`\`

### Development Mode

\`\`\`bash
# Start development server (with hot reload)
make dev

# Different log levels
make dev-debug    # DEBUG level
make dev-trace    # TRACE level (performance tracing)
make dev-warn     # Warning only
\`\`\`

---

## 📦 Local Build

### macOS Build

#### Universal Binary (Recommended) ⭐

\`\`\`bash
make build-universal
\`\`\`

Generates: \`NeuraDock_0.1.0_universal.dmg\` - **Supports both Intel + Apple Silicon**

**Why Universal Binary?**
- ✅ One installer for all Macs
- ✅ Best user experience
- ✅ Recommended for releases

#### Current Architecture

\`\`\`bash
make build-release
\`\`\`

Generates architecture-specific DMG for your current Mac.

#### All Architectures

\`\`\`bash
make build-all-macos
\`\`\`

Generates all three DMG files.

### Windows/Linux Build

**Cross-compilation not supported locally**. Use GitHub Actions instead.

---

## 🔄 CI/CD Configuration

### GitHub Actions can download built apps

Yes! When you trigger the **Test Build** workflow on GitHub Actions:

1. Build completes in ~10-15 minutes
2. Go to the workflow run page
3. Scroll to **Artifacts** section at the bottom
4. Download the installer for your platform:
   - \`NeuraDock-macOS-Universal\` - Contains .dmg file
   - \`NeuraDock-Windows-x64\` - Contains .msi file
   - \`NeuraDock-Linux-x64\` - Contains .deb and .AppImage files

The artifacts are retained for 7 days for testing.

### Workflows

#### 1. Test Build

**File**: \`.github/workflows/test-build.yml\`

**Purpose**: Test build process without creating a Release

**Trigger**: Manual only

**Output**: Build artifacts (7-day retention)

#### 2. Release

**File**: \`.github/workflows/release.yml\`

**Purpose**: Build all platforms and create GitHub Release

**Trigger**:
- Auto: Push \`v*\` tag
- Manual: From Actions page

**Output**:
- ✅ macOS Universal Binary (.dmg) - **Intel + Apple Silicon**
- ✅ Windows x64 (.msi)
- ✅ Linux (.deb, .AppImage)
- ✅ GitHub Release (draft)

### Pre-commit Validation

\`\`\`bash
# Install validation tools
brew install actionlint yamllint

# Run validation
make validate-actions
\`\`\`

---

## 🚀 Release Process

### Quick Release (5 minutes)

\`\`\`bash
# 1. Update version numbers in:
#    - apps/desktop/src-tauri/tauri.conf.json
#    - apps/desktop/package.json

# 2. Create and push tag
git add .
git commit -m "chore: bump version to v0.1.0"
git tag v0.1.0
git push origin main
git push origin v0.1.0

# 3. Wait ~20 minutes for GitHub Actions

# 4. Publish Release on GitHub
\`\`\`

---

## 🛠️ Common Commands

\`\`\`bash
make help               # Show all commands
make dev                # Start dev server
make build-universal    # Build Universal Binary (recommended)
make validate-actions   # Validate GitHub Actions
make show-targets       # Show all build options
\`\`\`

---

## 📚 References

- [Chinese Version](../zh/development.md)
- [User Manual](../../README.md)
- [Architecture](../../CLAUDE.md)

---

**Last Updated**: 2025-12-08
