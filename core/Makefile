SHELL := /bin/sh

include ../config.mk

rust-bindings:
	cargo build --release --target-dir=$(OUT_DIR)

ffi-bindings:
	cargo build --release --features "ffi" --target-dir=$(OUT_DIR)
	cp $(OUT_DIR)/release/libmothra.$(EXT) $(OUT_DIR)/

clean:
	cargo clean