SHELL := /bin/sh

include ../config.mk

ffi:
	cargo build $(BUILD_MODE) --target-dir=$(OUT_DIR)
	ln -sf $(OUT_DIR)/$(TARGET_NAME)/libmothra.$(EXT) $(OUT_DIR)/

rust-lint: 
	cargo fmt
	cargo clippy

clean:
	rm -f $(OUT_DIR)/libmothra.$(EXT)
	cargo clean --target-dir=$(OUT_DIR)