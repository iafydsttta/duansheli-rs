# Run the application with default config
run:
    cargo run -- ~/.duansheli/config.toml

# Build the project
build:
    cargo build

# Run tests
test:
    cargo test -- --nocapture
    
# Run tests (warnings muted)
test-quiet $RUSTFLAGS="-A warnings":
     cargo test -- --nocapture

# Build release version
release:
    cargo build --release

# Clean build artifacts
clean:
    cargo clean
