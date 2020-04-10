SHELL := /bin/sh

include ../config.mk

dotnet-bindings:
	cd $(DBIND_DIR) && make $@

java-bindings:
	cd $(JBIND_DIR) && make $@

clean:
	cd $(JBIND_DIR) && make $@
	cd $(DBIND_DIR) && make $@
	cd $(CBIND_DIR) && make $@