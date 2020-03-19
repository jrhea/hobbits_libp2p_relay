SHELL := /bin/sh

include ../config.mk

java-bindings:
	cd $(JBIND_DIR) && make $@

c-bindings:
	cd $(CBIND_DIR) && make $@

clean:
	cd $(JBIND_DIR) && make $@
	cd $(CBIND_DIR) && make $@