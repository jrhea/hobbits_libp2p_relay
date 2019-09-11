SHELL := /bin/sh

include ../../config.mk

JAVA_INCLUDES = -I$(JAVA_HOME)/include/$(OS) -I$(JAVA_HOME)/include
JAVA_LIBS = -L$(JAVA_HOME)/lib/server -ljvm
INCLUDES = -I$(JBIND_DIR)/
CFLAGS = -O2 -fPIC
LFLAGS = -shared

.PHONY : java-bindings-ingress java-bindings-egress clean

java-bindings-ingress: 
	mkdir -p $(OUT_DIR)
	$(CC) $(JBIND_DIR)/mothra-jni-ingress.c $(INCLUDES) $(JAVA_INCLUDES) $(JAVA_LIBS) $(CFLAGS) $(LFLAGS) -o $(OUT_DIR)/libmothra-ingress.$(EXT)

java-bindings-egress-build: mothra.class
	$(CC) $(JBIND_DIR)/mothra-jni-egress.c $(INCLUDES) -L$(OUT_DIR) -lmothra $(JAVA_INCLUDES) $(JAVA_LIBS) $(CFLAGS) $(LFLAGS) -o $(OUT_DIR)/libmothra-egress.$(EXT)

ifeq ($(OS), darwin)
java-bindings-egress: java-bindings-egress-build
	install_name_tool -change $(OUT_DIR)/release/deps/libmothra.dylib @loader_path/libmothra.dylib $(OUT_DIR)/libmothra-egress.dylib
else
java-bindings-egress: java-bindings-egress-build
endif

%.class:
	javac $*.java -d $(OUT_DIR)/
	cd $(OUT_DIR) \
	&& printf "Class-Path: mothra.jar\n" > manifest.txt \
	&& jar cfm mothra.jar manifest.txt net/p2p/*.class

clean:
	rm -rf $(OUT_DIR)/libmothra-egress.$(EXT)
