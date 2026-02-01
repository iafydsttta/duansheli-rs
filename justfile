# Run the application with default config
run:
    cargo run -- ~/.duansheli/config.toml info

# Build the project
build:
    cargo build

# Run tests
test:
    cargo test

# Build release version
release:
    cargo build --release

# Clean build artifacts
clean:
    cargo clean