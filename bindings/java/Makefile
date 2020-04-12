SHELL := /bin/sh

include ../../config.mk

JAVA_INCLUDES = -I$(JAVA_HOME)/include/$(OS) -I$(JAVA_HOME)/include
JAVA_LIBS = -L$(JAVA_HOME)/lib/server -ljvm
INCLUDES = -I$(CBIND_DIR) -I$(JBIND_DIR)
CFLAGS = -O2 -fPIC
LFLAGS = -shared

.PHONY : java-bindings java-bindings-build clean

java-bindings-build: Mothra.class
	$(CC) $(JBIND_DIR)/mothra-jni.c $(INCLUDES) $(JAVA_INCLUDES) -L$(OUT_DIR)/$(TARGET_NAME)/deps -lmothra $(JAVA_LIBS) $(CFLAGS) $(LFLAGS) -o $(JOUT_DIR)/libmothra-jni.$(EXT)

ifeq ($(OS), darwin)
java-bindings: java-bindings-build
	install_name_tool -change $(OUT_DIR)/$(TARGET_NAME)/deps/libmothra.dylib @loader_path/../libmothra.dylib $(JOUT_DIR)/libmothra-jni.dylib
else
java-bindings: java-bindings-build
endif

%.class:
	javac $*.java -d $(JOUT_DIR)/
	cd $(JOUT_DIR) \
	&& printf "Class-Path: mothra.jar\n" > manifest.txt \
	&& jar cfm mothra.jar manifest.txt p2p/mothra/*.class

clean:
	rm -rf $(JOUT_DIR)

