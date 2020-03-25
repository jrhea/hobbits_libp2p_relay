SHELL := /bin/sh

include ../config.mk

rust-bindings:
	cargo build --release
	ln -sf $(CORE_DIR)/target/release $(OUT_DIR)/release

clean:
	cargo clean