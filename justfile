# bmux — the tmux of Bluesky

default: lint test

# Build in debug mode
build:
    cargo build

# Build in release mode
release:
    cargo build --release

# Run with debug output
run *ARGS:
    cargo run -- {{ARGS}}

# Run in release mode
run-release *ARGS:
    cargo run --release -- {{ARGS}}

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run a specific test
test-one TEST:
    cargo test {{TEST}} -- --nocapture

# Check code compiles without building
check:
    cargo check

# Run clippy lints
lint:
    cargo clippy -- -W clippy::all

# Format code
fmt:
    cargo fmt

# Format check (CI)
fmt-check:
    cargo fmt -- --check

# Clean build artifacts
clean:
    cargo clean

# Watch for changes and rebuild
watch:
    cargo watch -x check

# Login with env vars and run
login:
    @echo "Set BMUX_IDENTIFIER and BMUX_PASSWORD env vars, then run:"
    @echo "  just run -u \$BMUX_IDENTIFIER -p \$BMUX_PASSWORD"

# Run with a specific theme
theme THEME:
    cargo run -- -t {{THEME}}

# Show dependency tree
deps:
    cargo tree

# Update dependencies
update:
    cargo update

# Full CI check: fmt, lint, test, build
ci: fmt-check lint test build
    @echo "All checks passed!"
