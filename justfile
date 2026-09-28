# agntz - Agent utility toolkit
# https://github.com/byteowlz/agntz

set positional-arguments

# === Default ===

# List available commands
default:
    @just --list

# === Install ===

install:
    cargo install --path . --force

install-local:
    cargo build --release
    mkdir -p ~/.local/bin
    cp target/release/agntz ~/.local/bin/
    @echo "Installed agntz to ~/.local/bin/agntz"

uninstall:
    cargo uninstall agntz || true

uninstall-local:
    rm -f ~/.local/bin/agntz
    @echo "Removed agntz from ~/.local/bin"

# === Building ===

build:
    cargo build

build-release:
    cargo build --release

check:
    cargo check

clean:
    cargo clean

# === Testing ===

test:
    cargo test

# Run integration tests against the wrapped CLIs (mmry/trx/etc.)
integration:
    ./tests/integration/run_all.sh

# === Code Quality ===

fmt:
    cargo fmt

fmt-check:
    cargo fmt -- --check

clippy:
    cargo clippy --all-targets -- -D warnings

lint: clippy

fix:
    cargo clippy --fix --allow-dirty

# Run ast-grep guardrails if `sg` is available (optional)
lint-rust-ai-guardrails:
    @if command -v sg >/dev/null 2>&1; then \
        echo "Running ast-grep guardrails..." && \
        sg scan --config .ast-grep/sgconfig.yml; \
    else \
        echo "ast-grep (sg) not installed - skipping guardrail scan"; \
    fi

# Regenerate examples/config.toml + config.schema.json from the config structs
generate-config:
    cargo run --example generate_config

# Run all checks (fmt + clippy + ast-grep + test + drift)
check-all: fmt-check clippy lint-rust-ai-guardrails generate-config test
    ./scripts/drift-check.sh

# === Documentation ===

docs:
    cargo doc --no-deps --open

# === Dependencies ===

update:
    cargo update

# === Release ===

release version_type:
    cargo release {{version_type}}

release-check:
    cargo test --quiet
    cargo clippy --quiet --all-targets -- -D warnings
    cargo fmt -- --check
    echo "All checks passed!"

# === Shell Completions ===

completions shell="bash":
    @mkdir -p completions
    cargo run --quiet -- completions {{shell}} > completions/agntz.{{shell}}