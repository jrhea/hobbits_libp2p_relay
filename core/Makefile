SHELL := /bin/sh

include ../config.mk

rust-bindings:
	rm -f $(OUT_DIR)/target/release/*.*
	cargo build --release --target-dir=$(OUT_DIR)
	cp $(OUT_DIR)/release/libmothra.$(EXT) $(OUT_DIR)/

clean:
	cargo clean