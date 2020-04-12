SHELL := /bin/sh

include ../../config.mk

.PHONY : dotnet-bindings clean

dotnet-bindings:
	dotnet build Mothra.csproj -o $(DOUT_DIR)

clean:
	rm -rf $(DOUT_DIR)