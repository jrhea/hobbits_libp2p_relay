SHELL := /bin/sh

include config.mk

.PHONY:= all mash rust c dotnet java rust-bindings ffi-bindings c-bindings dotnet-bindings java-bindings rust-examples c-examples dotnet-examples java-examples clean

.DEFAULT:= all

all: rust c dotnet java

mash: clean rust c dotnet java

style: rust-fmt rust-clippy

rust: rust-examples

c: c-examples

dotnet: dotnet-examples

java: java-examples

rust-examples:
	@echo ""
	@echo Building Rust examples
	cd $(EXAMPLES_DIR) && make $@

c-examples: c-bindings
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

rust-bindings:
	@echo ""
	@echo Building Rust bindings
	cd $(CORE_DIR) && make $@

ffi-bindings:
	@echo ""
	@echo Building FFI bindings
	cd $(CORE_DIR) && make $@

c-bindings: ffi-bindings
	@echo ""
	@echo Building C bindings
	cd $(BIND_DIR) && make $@

dotnet-bindings: ffi-bindings
	@echo ""
	@echo Building .Net bindings
	cd $(BIND_DIR) && make $@

java-bindings: ffi-bindings
	@echo ""
	@echo Building Java bindings
	cd $(BIND_DIR) && make $@

rust-fmt:
	@echo ""
	@echo Running cargo fmt
	cd $(CORE_DIR) && make $@

rust-clippy:
	@echo ""
	@echo Running cargo clippy
	cd $(CORE_DIR) && make $@

clean:
	rm -rf $(OUT_DIR)/*
	cd $(CORE_DIR) && make $@
	cd $(BIND_DIR) && make $@
	cd $(EXAMPLES_DIR) && make $@