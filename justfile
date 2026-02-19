# agntz - Agent utility toolkit
# https://github.com/byteowlz/agntz

set positional-arguments

# === Default ===

# List available commands
default:
    @just --list

# === Build ===

# Build debug binary
build:
    cargo build

# Build release binary
build-release:
    cargo build --release

# Fast compile check
check:
    cargo check

# === Test ===

# Run tests
test:
    cargo test

# === Lint & Format ===

# Run clippy linter
clippy:
    cargo clippy -- -D warnings

# Alias for clippy
lint: clippy

# Auto-fix lint warnings
fix:
    cargo clippy --fix --allow-dirty

# Format code
fmt:
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt -- --check

# === Install ===

# Install to ~/.cargo/bin
install:
    cargo install --path . --force

# Install to ~/.local/bin
install-local:
    cargo build --release
    mkdir -p ~/.local/bin
    cp target/release/agntz ~/.local/bin/
    @echo "Installed agntz to ~/.local/bin/agntz"

# Uninstall from ~/.cargo/bin
uninstall:
    cargo uninstall agntz || true

# Uninstall from ~/.local/bin
uninstall-local:
    rm -f ~/.local/bin/agntz
    @echo "Removed agntz from ~/.local/bin"

# === Docs ===

# Generate documentation
docs:
    cargo doc --no-deps --open

# === Clean ===

# Clean build artifacts
clean:
    cargo clean

# === Development ===

# Run in development mode
run *args:
    cargo run -- {{args}}

# Watch for changes and rebuild
watch:
    cargo watch -x check

# === Dependencies ===

# Update dependencies
update:
    cargo update

# === Release ===

# Release: bump version, commit, tag, and push
release-bump version:
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION="{{version}}"
    if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "Error: Version must be in format X.Y.Z"
        exit 1
    fi
    echo "Bumping version to $VERSION"
    sed -i "s/^version = .*/version = \"$VERSION\"/" Cargo.toml
    sed -i "s/version = \".*\"/version = \"$VERSION\"/" dist/homebrew/agntz.rb
    sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" dist/scoop/agntz.json
    sed -i "s/v0\.[0-9]\+\.[0-9]\+/v$VERSION/g" dist/scoop/agntz.json
    git add Cargo.toml dist/homebrew/agntz.rb dist/scoop/agntz.json
    git commit -m "chore: bump version to $VERSION"
    git tag "v$VERSION"
    git push origin main
    git push origin "v$VERSION"
    echo "Release v$VERSION pushed! Workflow will start automatically."

# Check release readiness
release-check:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Checking release readiness..."
    cargo test --quiet
    cargo clippy --quiet -- -D warnings
    cargo fmt -- --check
    echo "All checks passed!"

# Create release using cargo-release
release version_type:
    cargo release {{version_type}}
