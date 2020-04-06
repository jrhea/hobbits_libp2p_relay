SHELL := /bin/sh

include ../../config.mk

rust-examples:
	cargo build --release --target-dir=$(OUT_DIR)
	cp $(OUT_DIR)/release/rust-example $(OUT_DIR)/
	

clean:
	cargo clean