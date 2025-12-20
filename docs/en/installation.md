# Installation Guide

## Pre-built Binaries

The easiest way to install NeuraDock is to download pre-built binaries from the [Releases](https://github.com/i-rtfsc/NeuraDock/releases) page.

### macOS

1. Download the `.dmg` file for your architecture:
   - `NeuraDock_x.x.x_x64.dmg` for Intel Macs
   - `NeuraDock_x.x.x_aarch64.dmg` for Apple Silicon (M1/M2/M3)

2. Open the `.dmg` file

3. Drag NeuraDock to your Applications folder

4. First launch: Right-click → Open (required for unsigned apps)

### Windows

1. Download `NeuraDock_x.x.x_x64_en-US.msi`

2. Run the installer and follow the prompts

3. Launch NeuraDock from the Start Menu

### Linux

1. Download `NeuraDock_x.x.x_amd64.AppImage`

2. Make it executable:
   ```bash
   chmod +x NeuraDock_x.x.x_amd64.AppImage
   ```

3. Run the AppImage:
   ```bash
   ./NeuraDock_x.x.x_amd64.AppImage
   ```

## Building from Source

For development or to build the latest version from source:

### Prerequisites

- **Node.js**: >= 20.0.0
- **Rust**: >= 1.70.0 (install via [rustup](https://rustup.rs/))
- **npm**: Latest version
- **Chromium-based browser**: Chrome, Edge, or Brave (for WAF bypass)

### Build Steps

1. **Clone the repository**:
   ```bash
   git clone https://github.com/i-rtfsc/NeuraDock.git
   cd NeuraDock
   ```

2. **Install dependencies**:
   ```bash
   make install
   ```

3. **Start development server** (optional, for testing):
   ```bash
   make dev
   ```

4. **Build production binary**:
   ```bash
   make build
   ```

5. **Find the built application**:
   - macOS: `apps/desktop/src-tauri/target/release/bundle/dmg/`
   - Windows: `apps/desktop/src-tauri/target/release/bundle/msi/`
   - Linux: `apps/desktop/src-tauri/target/release/bundle/appimage/`

## Verifying Installation

After installation, verify NeuraDock is working:

1. Launch the application
2. Check the Dashboard loads correctly
3. Navigate to Settings → About to verify version

## Troubleshooting Installation

| Issue | Solution |
|-------|----------|
| macOS: "App is damaged" | Run `xattr -cr /Applications/NeuraDock.app` |
| Windows: SmartScreen warning | Click "More info" → "Run anyway" |
| Linux: AppImage won't run | Ensure FUSE is installed: `sudo apt install fuse` |
| Build fails on dependencies | Re-run `make install` |

See [Troubleshooting](./user_guide/troubleshooting.md) for more solutions.
