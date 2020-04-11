SHELL := /bin/sh

include ../../config.mk

CFLAGS:=
INCLUDES:=-I$(CBIND_DIR)

.PHONY : examples clean

c-examples:
	$(CC) example.c $(INCLUDES) -L$(OUT_DIR)/ -lmothra $(OS_CFLAGS) $(CFLAGS) -o $(OUT_DIR)/c-example
ifeq ($(OS), darwin)
	install_name_tool -change $(OUT_DIR)/$(TARGET_NAME)/deps/libmothra.dylib @rpath/libmothra.dylib $(OUT_DIR)/c-example
	install_name_tool -add_rpath @loader_path $(OUT_DIR)/c-example
endif

clean:
	rm -rf $(OUT_DIR)/c-example