SHELL := /bin/sh

include config.mk

.PHONY: all mash lib rust c dotnet java ffi dotnet-bindings java-bindings rust-examples c-examples dotnet-examples java-examples rust-lint clean

.DEFAULT: all

all: lib rust c dotnet java

mash: clean rust c dotnet java

lib:
	@echo ""
	@echo Building Mothra library
	cargo build $(BUILD_MODE) --target-dir=$(OUT_DIR)

rust: rust-examples

c: c-examples

dotnet: dotnet-examples

java: java-examples

rust-examples:
	@echo ""
	@echo Building Rust examples
	cd $(EXAMPLES_DIR) && make $@

c-examples: ffi
	@echo ""
	@echo Building C examples
	cd $(EXAMPLES_DIR) && make $@

dotnet-examples: dotnet-bindings
	@echo ""
	@echo Building .Net examples
	cd $(EXAMPLES_DIR) && make $@

java-examples: java-bindings
	@echo ""
	@echo Building Java examples
	cd $(EXAMPLES_DIR) && make $@

ffi:
	@echo ""
	@echo Building FFI
	cd $(FFI_DIR) && make $@

dotnet-bindings: ffi
	@echo ""
	@echo Building .Net bindings
	cd $(BIND_DIR) && make $@

java-bindings: ffi
	@echo ""
	@echo Building Java bindings
	cd $(BIND_DIR) && make $@

rust-lint: 
	cargo fmt
	cargo clippy
	cd $(FFI_DIR) && make $@

clean:
	cargo clean --target-dir=$(OUT_DIR)
	cd $(FFI_DIR) && make $@
	cd $(BIND_DIR) && make $@
	cd $(EXAMPLES_DIR) && make $@