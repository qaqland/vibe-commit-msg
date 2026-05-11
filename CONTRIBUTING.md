# Contributing

## Manual testing with `-t`

Build and invoke any tool directly:

```sh
cargo build --release

# read a file from the staged snapshot
./target/release/vibe-commit-msg -t read '{"path":"/src/main.rs"}'

# paginated read
./target/release/vibe-commit-msg -t read '{"path":"/src/main.rs","offset":5,"limit":20}'

# list a directory
./target/release/vibe-commit-msg -t list '{"path":"/src"}'

# regex search
./target/release/vibe-commit-msg -t grep '{"pattern":"fn main"}'

# file glob
./target/release/vibe-commit-msg -t glob '{"pattern":"src/**/*.rs"}'

# staged diff
./target/release/vibe-commit-msg -t diff '{}'

# staged stat
./target/release/vibe-commit-msg -t stat '{}'

# commit log
./target/release/vibe-commit-msg -t log '{"limit":5}'

# cache operations
./target/release/vibe-commit-msg -t cache '{}'
./target/release/vibe-commit-msg -t cache '{"path":"style.md"}'
./target/release/vibe-commit-msg -t cache '{"path":"test.md","content":"---\nkey: value\n---\nbody"}'

# enable error chain trace
VIBE_DEBUG=1 ./target/release/vibe-commit-msg -t read '{"path":"/nonexistent"}'
```

## Integration tests

Uses [bats](https://github.com/bats-core/bats-core).

```sh
# build release binary (if missing) and run all tests
make test

# run a single test file (binary must exist)
VIBE_BIN=./target/release/vibe-commit-msg bats tests/read.bats

# run with debug output
VIBE_BIN=./target/release/vibe-commit-msg bats -t tests/cache.bats
```
