SHELL := /bin/sh

include ../../config.mk

.PHONY : dotnet-examples clean

dotnet-examples:
	dotnet build Example.csproj -o $(DOUT_DIR)

clean:
	rm -rf $(DOUT_DIR)