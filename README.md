# duansheli

A directory declutter tool. Moves old files/directories to an archive, and deletes entries that exceed the deletion threshold.

## Configuration

Create a TOML config file (e.g. `~/.duansheli/config.toml`):

```toml
[[dirs]]
path = "/path/to/watch"
time_to_archive_hours = 24
time_to_deletion_hours = 168
```

Each entry in `dirs` defines a directory to manage, when to archive entries, and when to delete them.

## Running

**Dry run** — print planned actions without making changes:

```sh
RUST_LOG=info cargo run -- -n ~/.duansheli/config.toml
```

**Live run** — execute the plan:

```sh
RUST_LOG=info cargo run -- ~/.duansheli/config.toml
```

**With a release build:**

```sh
cargo build --release
RUST_LOG=info ./target/release/duansheli ~/.duansheli/config.toml
```

> `RUST_LOG=info` is required to see output. Use `RUST_LOG=debug` for more verbose logging.

## Tests

```sh
cargo test
```
