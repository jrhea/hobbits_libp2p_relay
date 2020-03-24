SHELL := /bin/sh

include ../config.mk

rust-bindings:
	cd $(RBIND_DIR) && make $@

c-bindings:
	cd $(CBIND_DIR) && make $@

dotnet-bindings:
	cd $(DBIND_DIR) && make $@

java-bindings:
	cd $(JBIND_DIR) && make $@

clean:
	cd $(JBIND_DIR) && make $@
	cd $(DBIND_DIR) && make $@
	cd $(CBIND_DIR) && make $@