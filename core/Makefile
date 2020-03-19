SHELL := /bin/sh

include ../config.mk

JAVA_INCLUDES = -I$(JAVA_HOME)/include/$(OS) -I$(JAVA_HOME)/include
JAVA_LIBS = -L$(JAVA_HOME)/lib/server/ -ljvm

rust-bindings:
	rm -f $(OUT_DIR)/target/release/*.*
	cargo build --release --target-dir=$(OUT_DIR)
	cp $(OUT_DIR)/release/libmothra.$(EXT) $(OUT_DIR)/

clean:
	cargo clean