SHELL := /bin/sh

include config.mk

.PHONY:=c-mash dotnet-mash java-mash c dotnet java c-bindings java-bindings c-examples dotnet-examples java-examples clean-bin clean

c-mash: clean c

dotnet-mash: clean dotnet

java-mash: clean java

c: clean-bin c-examples

dotnet: clean-bin dotnet-examples

java: clean-bin java-examples

c-examples: c-bindings
	@echo ""
	@echo Building examples
	cd $(EXAMPLES_DIR) && make $@

dotnet-examples: c-bindings
	@echo ""
	@echo Building examples
	cd $(EXAMPLES_DIR) && make $@

java-examples: java-bindings
	@echo ""
	@echo Building examples
	cd $(EXAMPLES_DIR) && make $@

c-bindings:
	@echo ""
	@echo Building C bindings
	cd $(BIND_DIR) && make $@
	@echo ""
	@echo Building Rust bindings
	cd $(CORE_DIR) && make $@

java-bindings: c-bindings
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