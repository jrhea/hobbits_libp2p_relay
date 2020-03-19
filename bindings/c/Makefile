SHELL := /bin/sh

include ../../config.mk

CFLAGS:=-O2 -fPIC
LFLAGS:=-shared
INCLUDES:=-I$(CBIND_DIR)/

.PHONY: c-bindings clean	

c-bindings: 
	mkdir -p $(OUT_DIR)
	$(CC) $(INCLUDES) -c $(CBIND_DIR)/mothra-ingress.c $(CFLAGS) $(LFLAGS) -o $(OUT_DIR)/libmothra-ingress.$(EXT)

clean:
	rm -rf $(TARGET)
	rm -rf $(OBJ)
