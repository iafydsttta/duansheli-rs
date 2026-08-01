# Run the application with default config

# Note to self: Just syntax
# @             Do not print command that is executed
# -             Ignore errors on that line (continue even if it exits non-zero).
# @- / -@       Combine both (silent and error-tolerant).

# Print list of just commands
list:
    @just --list

run:
    cargo run -- run -v
    
run-verbose:
    cargo run -- run -vv
    
dry-run:
    cargo run -- run -v -n
    
print-config:
    cargo run -- print

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

# List gitignored files
list-ignored:
    git ls-files --others --ignored --exclude-standard | grep -v target

