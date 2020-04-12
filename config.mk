OS:=$(shell uname -s | tr '[:upper:]' '[:lower:]')
CC:=gcc
AR:=ar
ifeq ($(OS), linux)
	EXT:=so
	OS_LFLAGS:=
	OS_CFLAGS:=-Wl,-rpath='$${ORIGIN}'
	JAVA_HOME:=$(shell java -XshowSettings:properties -version 2>&1 > /dev/null | grep 'java.home' | sed 's/\s*java.home = //' | sed 's/\/jre//')
else ifeq ($(OS), darwin)
	EXT:=dylib
	OS_LFLAGS:=-mmacosx-version-min=$(shell defaults read loginwindow SystemVersionStampAsString) -framework CoreFoundation -framework Security
	OS_CFLAGS:=
	JAVA_HOME:= $(shell java -XshowSettings:properties -version 2>&1 > /dev/null | grep 'java.home' | sed 's/\s*java.home = //' | sed 's/\/jre//')
endif

ifdef rls
	TARGET_NAME:=release
	BUILD_MODE:=--$(TARGET_NAME)
else
	TARGET_NAME:=debug
	BUILD_MODE:=
endif

ROOT_DIR:=$(shell dirname $(realpath $(lastword $(MAKEFILE_LIST))))
OUT_DIR:=$(ROOT_DIR)/bin
COUT_DIR = $(OUT_DIR)/c
DOUT_DIR = $(OUT_DIR)/dotnet
JOUT_DIR = $(OUT_DIR)/java
CORE_DIR:=$(ROOT_DIR)/core
FFI_DIR:=$(CORE_DIR)/ffi
BIND_DIR:=$(ROOT_DIR)/bindings
CBIND_DIR:=$(BIND_DIR)/c
DBIND_DIR:=$(BIND_DIR)/dotnet
JBIND_DIR:=$(BIND_DIR)/java
EXAMPLES_DIR:=$(ROOT_DIR)/examples
REXAMPLES_DIR:=$(EXAMPLES_DIR)/rust
CEXAMPLES_DIR:=$(EXAMPLES_DIR)/c
DEXAMPLES_DIR:=$(EXAMPLES_DIR)/dotnet
JEXAMPLES_DIR:=$(EXAMPLES_DIR)/java

$(shell mkdir -p $(OUT_DIR))

debug-%: ; @echo $*=$($*)