BIN ?= $(CURDIR)/target/release/vibe-commit-msg

.DEFAULT_GOAL := build
.PHONY: test build

test: build
	VIBE_BIN=$(BIN) bats tests/

build:
	cargo build --release
