# RustFS Launcher

A Tauri + Leptos application for launching RustFS.

## Prerequisites

- [Rust](https://rustup.rs/)
- [Node.js](https://nodejs.org/)
- [Trunk](https://trunkrs.dev/) - Install with `cargo install trunk`

## Building

Before building the application, you need to download the required RustFS binary for your platform:

### On macOS/Linux:
```bash
# Download binary for current platform
./build.sh

# Build for development
cargo tauri dev

# Build for production
cargo tauri build
```

### On Windows:
```cmd
# Download binary for current platform
build.bat

# Build for development
cargo tauri dev

# Build for production
cargo tauri build
```

The build script will automatically detect your platform and download the appropriate binary:
- **macOS Apple Silicon**: `rustfs-macos-aarch64`
- **macOS Intel**: `rustfs-macos-x86_64`
- **Windows x86_64**: `rustfs-windows-x86_64.exe`

This approach reduces download time and storage space by only downloading the binary needed for your current platform.

## Development Workflow

### Pre-commit Checks

Before committing your code, run all CI checks locally:

```bash
make pre-commit
```

This will run:
- Code formatting check (`cargo fmt`)
- Clippy linter (`cargo clippy`)
- Frontend build (`trunk build`)
- Unit tests (`cargo test`)

### Individual Checks

```bash
make check-fmt      # Check code formatting
make check-clippy   # Run Clippy linter
make check-frontend # Build frontend
make check-test     # Run tests
make fix-fmt        # Auto-fix formatting
```

### CI/CD

The project uses GitHub Actions for continuous integration and automated releases. See [.github/ACTIONS.md](.github/ACTIONS.md) for details.

For local testing of GitHub Actions workflows, see [.github/TESTING.md](.github/TESTING.md).

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
