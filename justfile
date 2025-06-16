# Build the project
build:
    cargo build

# Run all tests
test:
    cargo test --workspace

# Format the codebase
fmt:
    cargo fmt --all && cargo install taplo-cli --locked && taplo format

# Check codebase formatting for CI, assumes taplo installed.
check-fmt:
    cargo fmt --all --check && taplo format --check

# Check that Clippy is happy
lint:
    cargo clippy --workspace --tests -- -D warnings

# Scan for dependencies with vulnerabilities
audit:
    cargo install cargo-audit --locked && cargo audit

# Check for outdated dependencies
odeps:
    cargo install cargo-outdated --locked && cargo outdated --root-deps-only --verbose --exit-code 1

# Check for unused dependencies
udeps:
    cargo install cargo-machete --locked && cargo-machete --with-metadata

# Cargo clean
clean:
    cargo clean

# Generate documentation
doc:
    RUSTDOCFLAGS="--show-type-layout --generate-link-to-definition --enable-index-page -D warnings -Z unstable-options" \
    cargo +nightly doc --workspace --all-features --no-deps --document-private-items

# mdBook build
mdbook:
    cargo install mdbook --locked && mdbook test && cargo install mdbook-linkcheck --locked && mdbook-linkcheck --standalone

# Run all checks, unit tests and validate that the CI will pass
pre-release:
    just fmt lint test audit udeps
