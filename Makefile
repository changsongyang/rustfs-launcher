.PHONY: help install-act test-ci test-ci-full clean

# Default target
help:
	@echo "RustFS Launcher - GitHub Actions Local Testing"
	@echo ""
	@echo "Available targets:"
	@echo "  make help          - Show this help message"
	@echo "  make install-act   - Install act tool for local GitHub Actions testing"
	@echo "  make test-ci       - Run CI workflow locally (quick, Ubuntu only)"
	@echo "  make test-ci-full  - Run CI workflow with full checks"
	@echo "  make test-build    - Test build workflow locally (single platform)"
	@echo "  make list-jobs     - List all available jobs in workflows"
	@echo "  make clean         - Clean act cache and temporary files"
	@echo ""

# Install act tool
install-act:
	@echo "Installing act..."
	@if command -v brew >/dev/null 2>&1; then \
		brew install act; \
	else \
		echo "Error: Homebrew not found. Please install Homebrew first."; \
		echo "Visit: https://brew.sh"; \
		exit 1; \
	fi
	@echo "act installed successfully!"
	@act --version

# Check if act is installed
check-act:
	@command -v act >/dev/null 2>&1 || { \
		echo "Error: act is not installed. Run 'make install-act' first."; \
		exit 1; \
	}

# Run CI workflow locally (quick test)
test-ci: check-act
	@echo "Running CI workflow locally..."
	@echo "Note: This uses Ubuntu container and may take a few minutes on first run."
	act push -W .github/workflows/ci.yml \
		--container-architecture linux/amd64 \
		--platform ubuntu-latest=catthehacker/ubuntu:act-latest

# Run CI workflow with full checks
test-ci-full: check-act
	@echo "Running CI workflow with all checks..."
	act push -W .github/workflows/ci.yml \
		--container-architecture linux/amd64 \
		--platform ubuntu-latest=catthehacker/ubuntu:full-latest

# Test build workflow (manual trigger mode)
test-build: check-act
	@echo "Testing build workflow (workflow_dispatch mode)..."
	@echo "Warning: This will attempt to download RustFS binaries and build the app."
	@echo "Press Ctrl+C to cancel, or wait 5 seconds to continue..."
	@sleep 5
	act workflow_dispatch -W .github/workflows/build.yml \
		--container-architecture linux/amd64

# List all jobs in workflows
list-jobs: check-act
	@echo "=== CI Workflow Jobs ==="
	@act -W .github/workflows/ci.yml -l
	@echo ""
	@echo "=== Build Workflow Jobs ==="
	@act -W .github/workflows/build.yml -l

# Dry run - show what would be executed
dry-run-ci: check-act
	@echo "Dry run of CI workflow..."
	act push -W .github/workflows/ci.yml -n

dry-run-build: check-act
	@echo "Dry run of build workflow..."
	act workflow_dispatch -W .github/workflows/build.yml -n

# Clean act cache and temporary files
clean:
	@echo "Cleaning act cache..."
	@rm -rf ~/.cache/act
	@rm -rf /tmp/act-*
	@echo "Cache cleaned!"

# Run specific job from CI workflow
test-ci-job: check-act
	@echo "Available jobs in CI workflow:"
	@act -W .github/workflows/ci.yml -l
	@echo ""
	@read -p "Enter job name to run: " job; \
	act push -W .github/workflows/ci.yml -j $$job

# Test with verbose output
test-ci-verbose: check-act
	@echo "Running CI workflow with verbose output..."
	act push -W .github/workflows/ci.yml \
		--container-architecture linux/amd64 \
		--platform ubuntu-latest=catthehacker/ubuntu:act-latest \
		--verbose

# Quick format check only
test-fmt: check-act
	@echo "Testing Rust format check..."
	@cd src-tauri && cargo fmt --all --check

# Quick clippy check only
test-clippy: check-act
	@echo "Testing clippy..."
	@cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings

# Run tests locally
test-local:
	@echo "Running tests locally (without act)..."
	@cd src-tauri && cargo test --all-features
