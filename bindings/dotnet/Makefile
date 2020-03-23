SHELL := /bin/sh

include ../../config.mk

.PHONY : dotnet-bindings clean

dotnet-bindings:
	dotnet build Mothra.csproj -o $(OUT_DIR)/dotnet

clean:
	rm -rf $(OUT_DIR)/dotnet