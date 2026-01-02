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
