SHELL := /bin/sh

include ../config.mk

JAVA_INCLUDES = -I$(JAVA_HOME)/include/$(OS) -I$(JAVA_HOME)/include
JAVA_LIBS = -L$(JAVA_HOME)/lib/server/ -ljvm

c-bindings:
	rm -f $(OUT_DIR)/target/release/*.*
	cargo build --release

java-bindings-ingress-build:
	rm -f $(OUT_DIR)/target/release/*.*
	cargo build --release --target-dir=$(OUT_DIR)
	cp $(OUT_DIR)/release/libmothra.$(EXT) $(OUT_DIR)/

ifeq ($(OS), darwin)
java-bindings-ingress: java-bindings-ingress-build
	install_name_tool -change $(OUT_DIR)/libmothra-ingress.dylib @loader_path/libmothra-ingress.dylib $(OUT_DIR)/libmothra.dylib
else
java-bindings-ingress: java-bindings-ingress-build
endif

clean:
	cargo clean