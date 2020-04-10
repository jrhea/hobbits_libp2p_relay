SHELL := /bin/sh

include ../../config.mk

CFLAGS:=-O2 -fPIC
LFLAGS:=
INCLUDES:=-I$(CBIND_DIR)/

.PHONY: c-bindings clean	

c-bindings: 
	mkdir -p $(OUT_DIR)
	$(CC) $(INCLUDES) -c $(CBIND_DIR)/mothra-c.c $(CFLAGS) $(LFLAGS) -o $(OUT_DIR)/libmothra-c.$(EXT)

clean:
	rm -rf $(OUT_DIR)/libmothra-c.$(EXT)
