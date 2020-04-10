SHELL := /bin/sh

include ../config.mk

ffi:
	cargo build --release --target-dir=$(OUT_DIR)
	ln -sf $(OUT_DIR)/release/libmothra.$(EXT) $(OUT_DIR)/

rust-lint: 
	cargo fmt
	cargo clippy

clean:
	unlink $(OUT_DIR)/release/libmothra.$(EXT)
	cargo clean --target-dir=$(OUT_DIR)