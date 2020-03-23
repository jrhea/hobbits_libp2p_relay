SHELL := /bin/sh

include ../../config.mk

CFLAGS:=
INCLUDES:=-I$(CBIND_DIR)

.PHONY : examples clean

c-examples:
	$(CC) example.c $(CBIND_DIR)/mothra-c.c $(INCLUDES) -L$(OUT_DIR)/ -lmothra $(OS_CFLAGS) $(CFLAGS) -o $(OUT_DIR)/example
ifeq ($(OS), darwin)
	install_name_tool -change $(OUT_DIR)/release/deps/libmothra.dylib @rpath/libmothra.dylib $(OUT_DIR)/example
	install_name_tool -add_rpath @loader_path $(OUT_DIR)/example
endif

clean:
	rm -rf $(OUT_DIR)/example*