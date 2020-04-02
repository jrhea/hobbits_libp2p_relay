SHELL := /bin/sh

include ../config.mk

rust-bindings:
	rm -f Cargo.toml
	ln -sf Cargo-rust.toml Cargo.toml
	cargo build --release
	ln -sf $(CORE_DIR)/target/release $(OUT_DIR)/release

ffi-bindings:
	rm -f Cargo.toml
	ln -sf Cargo-ffi.toml Cargo.toml
	cargo build --release --features "ffi" --target-dir=$(OUT_DIR)
	cp $(OUT_DIR)/release/libmothra.$(EXT) $(OUT_DIR)/

clean:
	ln -sf Cargo-rust.toml Cargo.toml
	cargo clean
	rm -f Cargo.toml
