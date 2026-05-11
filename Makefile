BIN ?= $(CURDIR)/target/release/vibe-commit-msg

.PHONY: test build

test: build
	VIBE_BIN=$(BIN) bats tests/

build:
	cargo build --release
