SHELL := /bin/sh

include ../../config.mk

rust-examples:
	cargo build --release --target-dir=$(OUT_DIR)

clean:
	cargo clean