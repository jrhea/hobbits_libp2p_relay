SHELL := /bin/sh

include ../../config.mk

CFLAGS:=
INCLUDES:=-I$(CBIND_DIR)

.PHONY : c-examples clean

c-examples:
	mkdir -p $(COUT_DIR)
	$(CC) example.c $(INCLUDES) -L$(OUT_DIR)/ -lmothra $(OS_CFLAGS) $(CFLAGS) -o $(COUT_DIR)/c-example
ifeq ($(OS), darwin)
	install_name_tool -change $(OUT_DIR)/$(TARGET_NAME)/deps/libmothra.dylib @rpath/../libmothra.dylib $(COUT_DIR)/c-example
	install_name_tool -add_rpath @loader_path $(COUT_DIR)/c-example
endif

clean:
	rm -rf $(COUT_DIR)