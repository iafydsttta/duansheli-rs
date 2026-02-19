# Run the application with default config
run:
    cargo run -- ~/.duansheli/config.toml

# Build the project
build:
    cargo build

# Run tests
test:
    cargo test
    
# Run tests (warnings muted)
test-quiet $RUSTFLAGS="-A warnings":
     cargo test                                                                                                                                             

# Build release version
release:
    cargo build --release

# Clean build artifacts
clean:
    cargo clean