SHELL := /bin/sh

include config.mk

.PHONY: all mash core c dotnet java dotnet-bindings java-bindings rust-examples c-examples dotnet-examples java-examples lint clean

.DEFAULT: all

all: core c dotnet java

mash: clean core c dotnet java

core:
	@echo ""
	@echo Building Mothra core
	cargo build $(BUILD_MODE) --target-dir=$(OUT_DIR)
	ln -sf $(OUT_DIR)/$(TARGET_NAME)/libmothra.$(EXT) $(OUT_DIR)/
	ln -sf $(OUT_DIR)/$(TARGET_NAME)/rust-example $(OUT_DIR)/

c: c-examples

dotnet: dotnet-examples

java: java-examples

dotnet-bindings: core
	@echo ""
	@echo Building .Net bindings
	cd $(DBIND_DIR) && make $@

java-bindings: core
	@echo ""
	@echo Building Java bindings
	cd $(JBIND_DIR) && make $@

rust-examples:
	@echo ""
	@echo Building Rust examples
	cd $(REXAMPLES_DIR) && cargo build $(BUILD_MODE) --target-dir=$(OUT_DIR)

c-examples: core
	@echo ""
	@echo Building C examples
	cd $(CEXAMPLES_DIR) && make $@

dotnet-examples: dotnet-bindings
	@echo ""
	@echo Building .Net examples
	cd $(DEXAMPLES_DIR) && make $@

java-examples: java-bindings
	@echo ""
	@echo Building Java examples
	cd $(JEXAMPLES_DIR) && make $@

lint: 
	cargo fmt
	cargo clippy

clean:
	rm -rf $(COUT_DIR)
	rm -rf $(DOUT_DIR)
	rm -rf $(JOUT_DIR)
	rm -rf $(OUT_DIR)/$(TARGET_NAME)