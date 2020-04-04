SHELL := /bin/sh

include ../config.mk

rust-bindings:
	cargo build --release
	ln -sf $(CORE_DIR)/target/release $(OUT_DIR)/release

ffi-bindings:
	cargo build --release --features "ffi" --target-dir=$(OUT_DIR)
	cp $(OUT_DIR)/release/libmothra.$(EXT) $(OUT_DIR)/

clean:
	cargo clean