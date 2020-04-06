SHELL := /bin/sh

include config.mk

.PHONY:=c-mash dotnet-mash java-mash c dotnet java c-bindings dotnet-bindings java-bindings c-examples dotnet-examples java-examples clean-bin clean

c-mash: clean c

dotnet-mash: clean dotnet

java-mash: clean java

c: clean-bin c-examples

rust: clean-bin rust-examples

dotnet: clean-bin dotnet-examples

java: clean-bin java-examples

rust-examples:
	@echo ""
	@echo Building examples
	cd $(EXAMPLES_DIR) && make $@

c-examples: c-bindings
	@echo ""
	@echo Building examples
	cd $(EXAMPLES_DIR) && make $@

dotnet-examples: dotnet-bindings
	@echo ""
	@echo Building examples
	cd $(EXAMPLES_DIR) && make $@

java-examples: java-bindings
	@echo ""
	@echo Building examples
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

clean-bin:
	rm -f $(OUT_DIR)/*.*
	rm -rf $(OUT_DIR)/net
	
clean:
	rm -rf $(OUT_DIR)/*
	cd $(CORE_DIR) && make $@
	cd $(BIND_DIR) && make $@
	cd $(EXAMPLES_DIR) && make $@